//! `/settings.json` and `POST /settings`: the operator's settings, the
//! request ceilings and the closed lists, served from one place (R3-05).
//!
//! # What this replaces
//!
//! The operator's window, the instrument `/db` opens on, the rows every
//! picker always shows and the source folders were literals in a crate or a
//! page, or the start script's environment. The ceilings of one request
//! (24,000,000 rows read, 100,000 rows deep, 1,024 series, 240 months) were
//! constants in `crate::bars` that a page copied by hand, and the browser's
//! own sort and export caps lived only in the page.
//!
//! # Where each value comes from, said on every value
//!
//! 1. the newest saved generation (`store::settings`), named by number;
//! 2. else the start script's environment value the key names
//!    ([`store::settings::Key::env`]) — the named fallback;
//! 3. else this build's default, with the decision it came from.
//!
//! A generation that did not read is listed under `faults` with its file and
//! why; the newest that did stays in force and the answer says `degraded`.
//!
//! # A change
//!
//! `POST /settings` with a JSON body
//! `{"expected": <generation read>, "changed_by": "...", "reason": "...",
//!   "set": {"window.to": "2026-10-31"}, "unset": ["source.gdfl"]}`.
//! Every value is checked, then the whole result, then the server's own
//! check (an offered feed), and only then is a byte written — as the next
//! generation, never over an old one. The route is in
//! `crate::operation_audit::AUDITED`, so every attempt is on `/audit`, and
//! the generation itself records when, who, why and which keys.
//!
//! # Cost
//!
//! A GET is one `stat` of the next generation's name (when nothing was
//! saved) plus a fixed-size answer: O(1). `?history=N` reads N + 1 files,
//! N at most [`MAX_HISTORY`]. The first request after start lists the
//! settings directory once (named for `docs/06-limits.md`).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use axum::http::StatusCode;
use store::settings::{self, Change, Generation, Key, Reader, Refusal};

use crate::server::Loaded;

/// The most rows the browser reads to order a column the server cannot
/// (`web/src/lib/db/bars-plan.js` `LOCAL_SORT_BUDGET`).
pub const BROWSER_SORT_BUDGET: usize = 20_000;

/// The most rows the browser's whole-query export writes
/// (`web/src/lib/db/bars-plan.js` `EXPORT_CAP`).
pub const EXPORT_CAP: usize = 20_000;

/// The most generations one `?history=` answer reads.
pub const MAX_HISTORY: usize = 50;

/// One reader per store root, kept for the life of the process.
///
/// THE LOCK GUARDS THE MAP, NEVER A READ (R3-05 repair 3, the second
/// agent's blocking 2). It is held only to copy a store's reader out (a
/// path, a refcount and at most [`settings::MAX_FAULTS`] faults) and to put
/// one back; every `stat` and every read happens with it released. Before,
/// the refresh ran under it, so one generation name that made the read wait
/// (a FIFO) held every `/settings.json` and `/feeds.json` of every store
/// root, and concurrent `/feeds.json` requests then used up the runtime's
/// workers until unrelated routes stopped answering.
fn readers() -> &'static Mutex<std::collections::HashMap<PathBuf, Reader>> {
    static READERS: OnceLock<Mutex<std::collections::HashMap<PathBuf, Reader>>> = OnceLock::new();
    // PRE-SIZED (gate 11 rule 3): one reader per store root this process
    // serves — one in production, a handful in a test binary.
    READERS.get_or_init(|| Mutex::new(std::collections::HashMap::with_capacity(4)))
}

/// The map, its lock taken: a poisoned lock is taken over, since the map
/// holds only copies a refresh rebuilds.
fn map() -> std::sync::MutexGuard<'static, std::collections::HashMap<PathBuf, Reader>> {
    readers()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// A test seam called with the lock RELEASED, just before the refresh's
/// I/O: a test parks one store's read here and shows another store's read
/// still answers.
#[cfg(test)]
pub(crate) static BEFORE_REFRESH: std::sync::RwLock<Option<fn(&Path)>> =
    std::sync::RwLock::new(None);

/// Run `with` against the store's reader, refreshed first, with no lock
/// held across either. The reader goes back into the map unless a racing
/// request already put back one that has seen further.
fn with_reader<T>(root: &Path, with: impl FnOnce(&mut Reader, Option<String>) -> T) -> T {
    let held = map().get(root).cloned();
    let mut reader = held.unwrap_or_else(|| Reader::new(root));
    #[cfg(test)]
    if let Some(hook) = *BEFORE_REFRESH
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
    {
        hook(root);
    }
    let error = reader.refresh().err().map(|fault| {
        format!(
            "the settings directory {} cannot be read: {}",
            fault.path.display(),
            fault.why
        )
    });
    let out = with(&mut reader, error);
    let mut map = map();
    let keep = map
        .get(root)
        .is_none_or(|kept| kept.highest() <= reader.highest());
    if keep {
        map.insert(root.to_path_buf(), reader);
    }
    out
}

/// The settings in force for one store, as one consistent read.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// The highest generation on disk, `0` for none.
    pub highest: u64,
    /// The newest generation that read.
    pub newest: Option<Arc<Generation>>,
    /// The generations that did not read, newest first.
    pub faults: Vec<settings::Fault>,
    /// Why the reader stopped short, or could not list the directory.
    pub stopped: Option<String>,
    /// The generation numbers missing below the highest listed, in words
    /// (`store::settings::Reader::holes`), or `None` when they run unbroken.
    pub holes: Option<String>,
}

/// [`snapshot`] on the blocking pool, never on an async worker: the route
/// that calls it (`/feeds.json`) answers on a runtime worker, and a read
/// that is slow (a disk that stalls) must not hold one (R3-05 repair 3). A
/// read that did not finish is a degraded snapshot naming why, never an
/// absent one.
pub async fn snapshot_off_worker(root: PathBuf) -> Snapshot {
    match tokio::task::spawn_blocking(move || snapshot(&root)).await {
        Ok(snapshot) => snapshot,
        Err(why) => Snapshot {
            highest: 0,
            newest: None,
            faults: Vec::new(),
            stopped: Some(format!("the settings read did not finish: {why}")),
            holes: None,
        },
    }
}

/// The settings in force for the store at `root`. One `stat` when nothing
/// changed since the last call.
#[must_use]
pub fn snapshot(root: &Path) -> Snapshot {
    with_reader(root, |reader, error| Snapshot {
        highest: reader.highest(),
        newest: reader.newest().cloned(),
        faults: reader.faults().to_vec(),
        stopped: error.or_else(|| reader.stopped().map(str::to_owned)),
        holes: reader.holes().map(str::to_owned),
    })
}

/// One value in force and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effective {
    /// The value, or `None` when nothing sets it.
    pub value: Option<String>,
    /// Where it came from, in words.
    pub source: String,
    /// `generation`, `environment` or `default`.
    pub origin: &'static str,
}

/// This build's default for `key`, and the decision it came from.
fn default_of(key: Key) -> (Option<String>, String) {
    const OPEN: &str = "built-in default: the operator's answer 11 of 7 Oct 2026, /db opens on \
                        NIFTY · Spot · the feed's finest timeframe · newest first";
    match key {
        Key::WindowFrom | Key::WindowTo => {
            let window = pull::vendor::Feed::ALL
                .into_iter()
                .find_map(|feed| feed.descriptor().window);
            match window {
                Some(span) => (
                    Some(if key == Key::WindowFrom {
                        span.from.to_string()
                    } else {
                        span.to.to_string()
                    }),
                    format!(
                        "built-in default: the operator's window as this build records it \
                         ({})",
                        span.source
                    ),
                ),
                None => (None, "no window is recorded in this build".to_owned()),
            }
        }
        Key::OpenFeed => (
            None,
            "no default: the page opens on the first offered feed".to_owned(),
        ),
        Key::OpenInstrument => (Some("NIFTY".to_owned()), OPEN.to_owned()),
        Key::OpenSegment => (Some("INDEX".to_owned()), OPEN.to_owned()),
        Key::OpenTimeframe => (Some("finest".to_owned()), OPEN.to_owned()),
        Key::OpenOrder => (Some("newest".to_owned()), OPEN.to_owned()),
        Key::AlwaysShown => (
            Some("NIFTY,BANKNIFTY".to_owned()),
            "built-in default: CLAUDE.md §1, the two spot indices".to_owned(),
        ),
        Key::SourceGdfl => (
            None,
            "not set: no GDFL folder is named until one is saved".to_owned(),
        ),
        Key::SourceMasters => (
            None,
            "not set: the server derives the masters folder from HOME".to_owned(),
        ),
    }
}

/// Why the settings in force under `snapshot` are degraded — the directory
/// could not be listed or read on, a generation did not read, or a number
/// is missing below the highest listed — or `None` when every generation on
/// disk read and the numbers run unbroken.
#[must_use]
pub fn degraded(snapshot: &Snapshot) -> Option<String> {
    if let Some(stopped) = &snapshot.stopped {
        return Some(stopped.clone());
    }
    snapshot
        .faults
        .first()
        .map(|fault| {
            format!(
                "settings generation {} ({}) did not read: {}",
                fault.generation,
                fault.path.display(),
                fault.why
            )
        })
        // A HOLE IN THE NUMBERS IS DEGRADED TOO, never invisible (R3-05
        // repair 4): a generation past it was not seen by a live reader.
        .or_else(|| snapshot.holes.clone())
}

/// The value of `key` from `saved` (a whole settings result and the
/// generation it is, `0` for one not yet saved), else the start script's
/// environment, else this build's default.
///
/// An environment value is CHECKED as a save checks a value: one that
/// fails is not in force, and the answer says what it held and why, rather
/// than serving it or falling silently past it to the default.
fn effective_of(saved: Option<(&settings::Settings, u64)>, key: Key) -> Effective {
    effective_from(saved, key, key.env().and_then(std::env::var_os))
}

/// [`effective_of`] with the environment value given, so a test can show
/// each case (a test cannot set the environment: `set_var` is `unsafe` under
/// edition 2024 and this crate forbids it).
///
/// A key whose fallback has its own reader ([`fallback_reader_of`]: the
/// masters folder, read once at start by `server::masters_dir_from`) takes
/// its environment value AS THAT READER DID: a value a save would refuse is
/// still the one in force, and the answer says both. Before, it was served
/// as "nothing is in force" while the server was reading that very folder
/// (R3-05 repair 3, the second agent's follow-up).
fn effective_from(
    saved: Option<(&settings::Settings, u64)>,
    key: Key,
    env: Option<std::ffi::OsString>,
) -> Effective {
    if let Some((values, number)) = saved
        && let Some(value) = values.get(key)
    {
        return Effective {
            value: Some(value.to_owned()),
            source: if number == 0 {
                "this change".to_owned()
            } else {
                format!("settings generation {number}")
            },
            origin: "generation",
        };
    }
    if let Some(name) = key.env()
        && let Some(value) = env
            .map(|raw| raw.to_string_lossy().into_owned())
            .filter(|v| !v.trim().is_empty())
    {
        return match settings::check(key, &value) {
            Ok(checked) => Effective {
                value: Some(checked),
                source: format!(
                    "environment {name}: the start script's value, in force because no \
                     generation sets {}",
                    key.name()
                ),
                origin: "environment",
            },
            Err(refusal) => match fallback_reader_of(key) {
                Some(reader) => Effective {
                    source: format!(
                        "environment {name} holds {value:?}: {reader} reads it as given, so it \
                         is in force, although a save would refuse it ({refusal})"
                    ),
                    value: Some(value),
                    origin: "environment",
                },
                None => Effective {
                    value: None,
                    source: format!(
                        "environment {name} holds {value:?}, which is refused ({refusal}): \
                         nothing is in force for {}",
                        key.name()
                    ),
                    origin: "environment",
                },
            },
        };
    }
    let (value, source) = default_of(key);
    Effective {
        value,
        source,
        origin: "default",
    }
}

/// The value of `key` in force under `snapshot`. While the settings are
/// [`degraded`], a value that does not come from a generation says so in
/// its source: a saved value may differ from it.
#[must_use]
pub fn effective(snapshot: &Snapshot, key: Key) -> Effective {
    let saved = snapshot
        .newest
        .as_deref()
        .map(|newest| (&newest.settings, newest.number));
    let mut got = effective_of(saved, key);
    if got.origin != "generation"
        && let Some(fault) = degraded(snapshot)
    {
        got.source = format!(
            "{}; the saved settings are degraded ({fault}), so a saved value may differ",
            got.source
        );
    }
    got
}

/// What reads `key`'s FALLBACK — its environment value or default — when no
/// generation sets it, where that differs from [`reader_of`]: the masters
/// folder is read once at start from `BRUTEX_MASTERS`, else `HOME`, so that
/// fallback is in force while a saved value is not.
#[must_use]
pub const fn fallback_reader_of(key: Key) -> Option<&'static str> {
    match key {
        Key::SourceMasters => Some("the server, once at start: BRUTEX_MASTERS, else HOME"),
        _ => None,
    }
}

/// What reads `key`'s value in force, or why nothing does yet and what
/// decides that behaviour instead.
///
/// One table, so a key's reader lands in one place: until a key has a
/// reader its value is served with `in_force: false` and the reason, never
/// as if it applied (the second agent's blocking 4).
///
/// # Errors
///
/// `Err` is not a failure: it is the sentence saying nothing reads `key`
/// yet, and what decides that behaviour instead.
pub const fn reader_of(key: Key) -> Result<&'static str, &'static str> {
    match key {
        Key::WindowFrom | Key::WindowTo => Ok(
            "/feeds.json window, on every request: the /ingest form and /db read the window there",
        ),
        Key::OpenFeed
        | Key::OpenInstrument
        | Key::OpenSegment
        | Key::OpenTimeframe
        | Key::OpenOrder => Err(
            "nothing reads this key yet: /db opens on its own constants \
             (web/src/lib/db/selection-model.js DEFAULT_INSTRUMENT, NIFTY · Spot · the feed's \
             finest timeframe · newest first); owner: the /db opening-view package (R3-33)",
        ),
        Key::AlwaysShown => Err(
            "nothing reads this key yet: the pickers show their own built-in rows; owner: the \
             /db opening-view package (R3-33)",
        ),
        Key::SourceGdfl => Err(
            "nothing reads this key yet: the GDFL folder is the one named on the /ingest form; \
             owner: the ingest chain (R4-07, R4-11)",
        ),
        Key::SourceMasters => Err(
            "nothing reads a saved value yet: the server reads BRUTEX_MASTERS, else HOME, once \
             at start; owner: the masters loader",
        ),
    }
}

/// `micros` as `YYYY-MM-DD HH:MM:SS IST`, or the bare number.
fn ist_text(micros: i64) -> String {
    let secs = micros.div_euclid(1_000_000).saturating_add(19_800);
    let day = secs.div_euclid(86_400);
    let of_day = secs.rem_euclid(86_400);
    u32::try_from(day)
        .ok()
        .and_then(|days| pull::session::Day::from_days(days).ok())
        .map_or_else(
            || micros.to_string(),
            |civil| {
                format!(
                    "{civil} {:02}:{:02}:{:02} IST",
                    of_day / 3_600,
                    of_day % 3_600 / 60,
                    of_day % 60
                )
            },
        )
}

/// One generation's record as JSON: when, who, why, which keys.
fn generation_json(generation: &Generation) -> serde_json::Value {
    serde_json::json!({
        "generation": generation.number,
        "changed_at_micros": generation.changed_at_micros,
        "changed_at": ist_text(generation.changed_at_micros),
        "changed_by": generation.changed_by,
        "reason": generation.reason,
        "changed": generation.changed.iter().map(|k| k.name()).collect::<Vec<_>>(),
        "path": generation.path.display().to_string(),
    })
}

/// Every request ceiling, each once, from the constant that enforces it.
fn ceilings_json() -> serde_json::Value {
    serde_json::json!([
        {"name": "max_scan_bars", "value": crate::bars::MAX_SCAN_BARS,
         "enforced_by": "crate::bars::MAX_SCAN_BARS", "what": "rows one sort or scan may read"},
        {"name": "max_extremes_scan_bars", "value": crate::bars::MAX_EXTREMES_SCAN_BARS,
         "enforced_by": "crate::bars::MAX_EXTREMES_SCAN_BARS", "what": "rows one month-extremes fold may read"},
        {"name": "max_sort_depth", "value": crate::bars::MAX_SORT_DEPTH,
         "enforced_by": "crate::bars::MAX_SORT_DEPTH", "what": "the deepest row a non-time sort serves"},
        {"name": "max_series", "value": crate::bars::MAX_SERIES,
         "enforced_by": "crate::bars::MAX_SERIES", "what": "series one window request names"},
        {"name": "max_window_months", "value": crate::bars::MAX_WINDOW_MONTHS,
         "enforced_by": "crate::bars::MAX_WINDOW_MONTHS", "what": "months one window request spans"},
        {"name": "max_window_limit", "value": crate::bars::MAX_WINDOW_LIMIT,
         "enforced_by": "crate::bars::MAX_WINDOW_LIMIT", "what": "rows one window page serves"},
        {"name": "browser_sort_budget", "value": BROWSER_SORT_BUDGET,
         "enforced_by": "the browser (web/src/lib/db/bars-plan.js LOCAL_SORT_BUDGET)",
         "what": "rows the browser reads to order a column the server cannot"},
        {"name": "export_cap", "value": EXPORT_CAP,
         "enforced_by": "the browser (web/src/lib/db/bars-plan.js EXPORT_CAP)",
         "what": "rows the whole-query export writes"},
    ])
}

/// The closed lists a picker offers, read off the enums that define them.
fn lists_json() -> serde_json::Value {
    let feeds: Vec<serde_json::Value> = pull::vendor::Feed::ALL
        .into_iter()
        .map(|feed| {
            serde_json::json!({
                "wire": feed.wire(),
                "display": feed.display(),
                "offered": feed.descriptor().offer.is_offered(),
                "kind": match feed.source_kind() {
                    pull::vendor::SourceKind::Rest => "rest",
                    pull::vendor::SourceKind::Folder => "folder",
                },
            })
        })
        .collect();
    // EACH KIND ONCE, in the kinds' own order, kept when any feed has it:
    // two compile-time tables, no membership test on a growing list (gate
    // 11 rule 7).
    let kinds: Vec<&str> = [
        (pull::vendor::SourceKind::Rest, "rest"),
        (pull::vendor::SourceKind::Folder, "folder"),
    ]
    .into_iter()
    .filter(|(kind, _)| {
        pull::vendor::Feed::ALL
            .into_iter()
            .any(|feed| feed.source_kind() == *kind)
    })
    .map(|(_, word)| word)
    .collect();
    serde_json::json!({
        "feeds": feeds,
        "kinds": kinds,
        "segments": pull::manifest::SEGMENTS.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        "timeframes": store::path::Timeframe::KNOWN.iter().map(|tf| tf.as_str()).collect::<Vec<_>>(),
        "orders": ["newest", "oldest"],
        "keys": Key::ALL.iter().map(|k| serde_json::json!({"key": k.name(), "env": k.env()})).collect::<Vec<_>>(),
    })
}

/// The whole `/settings.json` answer for `root`; `history` generations of
/// past changes when asked.
#[must_use]
pub fn answer(root: &Path, history: usize) -> serde_json::Value {
    let snap = snapshot(root);
    let values: Vec<serde_json::Value> = Key::ALL
        .into_iter()
        .map(|key| {
            let got = effective(&snap, key);
            let fallback = if got.origin == "generation" {
                None
            } else {
                fallback_reader_of(key)
            };
            let (read_by, not_applied) = match (reader_of(key), fallback) {
                (Ok(reader), _) | (Err(_), Some(reader)) => (Some(reader), None),
                (Err(why), None) => (
                    None,
                    Some(if got.origin == "generation" {
                        format!("saved; {why}")
                    } else {
                        why.to_owned()
                    }),
                ),
            };
            serde_json::json!({
                "key": key.name(),
                "value": got.value,
                "origin": got.origin,
                "source": got.source,
                "env": key.env(),
                "read_by": read_by,
                "in_force": read_by.is_some(),
                "not_applied": not_applied,
            })
        })
        .collect();
    let past: Vec<serde_json::Value> = if history == 0 {
        Vec::new()
    } else {
        let read = settings::history(root, snap.highest, history.saturating_add(1));
        let mut rows = Vec::new();
        for (at, one) in read.iter().enumerate().take(history) {
            let row = match one {
                Ok(generation) => {
                    // THE GENERATION IT CHANGED FROM: the nearest older one
                    // that read — a save compares against the newest that
                    // read, so a damaged one between is skipped here too.
                    let older = read
                        .get(at + 1..)
                        .unwrap_or_default()
                        .iter()
                        .find_map(|r| r.as_ref().ok());
                    let diff: Vec<serde_json::Value> = generation
                        .changed
                        .iter()
                        .map(|key| {
                            serde_json::json!({
                                "key": key.name(),
                                "before": older.and_then(|o| o.settings.get(*key)),
                                "after": generation.settings.get(*key),
                            })
                        })
                        .collect();
                    let mut row = generation_json(generation);
                    if let Some(object) = row.as_object_mut() {
                        object.insert("diff".to_owned(), serde_json::Value::Array(diff));
                    }
                    row
                }
                Err(fault) => serde_json::json!({
                    "generation": fault.generation,
                    "fault": fault.why,
                    "path": fault.path.display().to_string(),
                }),
            };
            rows.push(row);
        }
        rows
    };
    serde_json::json!({
        "schema_version": 1,
        "directory": settings::dir(root).display().to_string(),
        "generation": snap.newest.as_ref().map_or(0, |g| g.number),
        "highest": snap.highest,
        "degraded": degraded(&snap).is_some(),
        "degraded_why": degraded(&snap),
        "holes": snap.holes,
        "faults": snap.faults.iter().map(|f| serde_json::json!({
            "generation": f.generation, "path": f.path.display().to_string(), "why": f.why,
        })).collect::<Vec<_>>(),
        "stopped": snap.stopped,
        "last_change": snap.newest.as_deref().map(generation_json),
        "values": values,
        "history": past,
        "ceilings": ceilings_json(),
        "lists": lists_json(),
    })
}

fn json_headers() -> [(axum::http::HeaderName, &'static str); 1] {
    [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )]
}

/// `GET /settings.json[?history=N]`.
pub(super) async fn settings_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    axum::extract::RawQuery(query): axum::extract::RawQuery,
) -> (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let mut history = 0_usize;
    for pair in query
        .as_deref()
        .unwrap_or("")
        .split('&')
        .filter(|p| !p.is_empty())
    {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        let parsed = match name {
            // DIGITS ONLY: `usize::from_str` takes a leading `+`, so
            // `history=+5` was served while the same number spelt `%35` was
            // refused (R3-05 repair 3, the second agent's follow-up).
            "history" if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) => {
                value.parse::<usize>().ok().filter(|n| *n <= MAX_HISTORY)
            }
            _ => None,
        };
        let Some(n) = parsed else {
            return (
                StatusCode::BAD_REQUEST,
                json_headers(),
                serde_json::json!({
                    "refusal": format!(
                        "{pair:?}: /settings.json takes only history=0..={MAX_HISTORY}"
                    ),
                    "code": "bad_parameter",
                })
                .to_string(),
            );
        };
        history = n;
    }
    let root = site.store_root.clone();
    match tokio::task::spawn_blocking(move || answer(&root, history)).await {
        Ok(body) => (StatusCode::OK, json_headers(), body.to_string()),
        Err(why) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_headers(),
            serde_json::json!({"refusal": format!("the settings read did not finish: {why}")})
                .to_string(),
        ),
    }
}

/// The server's own check of a whole result: an open feed must be offered,
/// and the EFFECTIVE window — each end saved, else the environment, else
/// the default — must not begin after it ends.
///
/// The store's own check compares only the ends a generation SETS; a
/// generation that sets `window.from` alone past the default `window.to`,
/// or unsets a later `window.to`, would be served inverted (the second
/// agent's blocking 2). The refusal names the key the change moved: an
/// unset key first, else `window.from` when the change sets it, else
/// `window.to`.
fn server_check(settings: &settings::Settings, change: &Change) -> Result<(), Refusal> {
    if let Some(wire) = settings.get(Key::OpenFeed) {
        let offered = pull::vendor::Feed::ALL
            .into_iter()
            .any(|feed| feed.wire() == wire && feed.descriptor().offer.is_offered());
        if !offered {
            return Err(Refusal::Invalid {
                key: Some(Key::OpenFeed),
                why: format!("{wire:?} is not an offered feed"),
            });
        }
    }
    let from = effective_of(Some((settings, 0)), Key::WindowFrom);
    let to = effective_of(Some((settings, 0)), Key::WindowTo);
    if let (Some(first), Some(last)) = (&from.value, &to.value)
        && first > last
    {
        // THE UNSET KEYS BY SLOT: one array probe per question, never a
        // membership test on the change's list (gate 11 rule 7).
        let mut unset = [false; Key::ALL.len()];
        for key in &change.unset {
            if let Some(slot) = unset.get_mut(key.slot()) {
                *slot = true;
            }
        }
        let moved = |key: Key| unset.get(key.slot()).copied().unwrap_or(false);
        let set = |key: Key| change.set.iter().any(|(k, _)| *k == key);
        let key = if moved(Key::WindowTo) {
            Key::WindowTo
        } else if moved(Key::WindowFrom) || set(Key::WindowFrom) {
            Key::WindowFrom
        } else {
            Key::WindowTo
        };
        return Err(Refusal::Invalid {
            key: Some(key),
            why: format!(
                "the window in force would begin {first} ({}) after it ends {last} ({})",
                from.source, to.source
            ),
        });
    }
    Ok(())
}

/// Parse a change body; `Err` is the refusal's words.
fn parse_change(body: &str) -> Result<Change, String> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|why| format!("the body is not JSON: {why}"))?;
    let object = value.as_object().ok_or("the body is not a JSON object")?;
    for name in object.keys() {
        if !matches!(
            name.as_str(),
            "expected" | "changed_by" | "reason" | "set" | "unset"
        ) {
            return Err(format!("unknown field {name:?}"));
        }
    }
    let expected = object
        .get("expected")
        .and_then(serde_json::Value::as_u64)
        .ok_or("`expected` must be the generation number read, 0 when none")?;
    let text = |name: &str| {
        object
            .get(name)
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| format!("`{name}` must be a string"))
    };
    let mut set = Vec::new();
    if let Some(map) = object.get("set") {
        let map = map
            .as_object()
            .ok_or("`set` must be an object of key: value")?;
        for (name, value) in map {
            let key = Key::of_name(name).ok_or_else(|| format!("unknown setting {name:?}"))?;
            let value = value
                .as_str()
                .ok_or_else(|| format!("{name}: the value must be a string"))?;
            set.push((key, value.to_owned()));
        }
    }
    let mut unset = Vec::new();
    if let Some(list) = object.get("unset") {
        let list = list.as_array().ok_or("`unset` must be an array of keys")?;
        for name in list {
            let name = name.as_str().ok_or("`unset` must hold key names")?;
            unset.push(Key::of_name(name).ok_or_else(|| format!("unknown setting {name:?}"))?);
        }
    }
    if set.is_empty() && unset.is_empty() {
        return Err("the change sets and unsets nothing".to_owned());
    }
    let at_micros = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_micros()).ok())
        .ok_or("the clock is before 1970 or past i64")?;
    Ok(Change {
        expected,
        set,
        unset,
        changed_by: text("changed_by")?,
        reason: text("reason")?,
        at_micros,
    })
}

/// Save a change for the store at `root`: the status and the JSON answer.
#[must_use]
pub fn change(root: &Path, body: &str) -> (StatusCode, serde_json::Value) {
    let parsed = match parse_change(body) {
        Ok(change) => change,
        Err(why) => {
            return (
                StatusCode::BAD_REQUEST,
                serde_json::json!({"refusal": why, "code": "bad_body", "saved": false}),
            );
        }
    };
    with_reader(root, |reader, _| {
        // WHAT THE CHANGE IS BUILT ON, SAID: when a generation newer than
        // the one in force did not read, the change is built on the older
        // one, and the answer names both rather than building on it silently
        // (R3-05 repair 3, the second agent's follow-up 1).
        let built_on = reader.newest().map_or(0, |generation| generation.number);
        let over_faults: Vec<serde_json::Value> = reader
            .faults()
            .iter()
            .map(|fault| {
                serde_json::json!({
                    "generation": fault.generation,
                    "path": fault.path.display().to_string(),
                    "why": fault.why,
                })
            })
            .collect();
        match settings::save(reader, &parsed, |result| server_check(result, &parsed)) {
            Ok(saved) => {
                // SAVED IS NOT APPLIED: every key this change moved that
                // nothing reads yet is named with its reason.
                let not_applied: Vec<serde_json::Value> = saved
                    .changed
                    .iter()
                    .filter_map(|key| {
                        reader_of(*key)
                            .err()
                            .map(|why| serde_json::json!({"key": key.name(), "why": why}))
                    })
                    .collect();
                (
                    StatusCode::OK,
                    serde_json::json!({
                        "saved": true,
                        "generation": generation_json(&saved),
                        "not_applied": not_applied,
                        "built_on": built_on,
                        "over_faults": over_faults,
                        "warning": (!over_faults.is_empty()).then(|| format!(
                            "built on generation {built_on}, the newest that reads: {} newer \
                             generation(s) did not read and are left as they are",
                            over_faults.len()
                        )),
                        "cleared_leftover": reader.last_cleared().map(|(path, bytes)| {
                            serde_json::json!({
                                "path": path.display().to_string(),
                                "bytes": bytes,
                            })
                        }),
                    }),
                )
            }
            Err(refusal) => {
                let (status, code, key) = match &refusal {
                    Refusal::Invalid { key, .. } => {
                        (StatusCode::BAD_REQUEST, "invalid", key.map(Key::name))
                    }
                    Refusal::Unchanged => (StatusCode::BAD_REQUEST, "unchanged", None),
                    Refusal::Stale { .. } => (StatusCode::CONFLICT, "stale", None),
                    Refusal::Io { kind, .. } if *kind == std::io::ErrorKind::StorageFull => {
                        (StatusCode::INSUFFICIENT_STORAGE, "disk_full", None)
                    }
                    Refusal::Io { .. } => (StatusCode::INTERNAL_SERVER_ERROR, "io", None),
                };
                (
                    status,
                    serde_json::json!({
                        "saved": false,
                        "refusal": refusal.to_string(),
                        "code": code,
                        "key": key,
                        "highest": reader.highest(),
                    }),
                )
            }
        }
    })
}

/// `POST /settings`.
pub(super) async fn settings_post(
    axum::extract::State(site): axum::extract::State<Loaded>,
    body: String,
) -> (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let root = site.store_root.clone();
    match tokio::task::spawn_blocking(move || change(&root, &body)).await {
        Ok((status, answer)) => (status, json_headers(), answer.to_string()),
        Err(why) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_headers(),
            serde_json::json!({"saved": false, "refusal": format!("the save did not finish: {why}")})
                .to_string(),
        ),
    }
}

#[cfg(test)]
#[path = "settings_route_tests.rs"]
mod tests;
