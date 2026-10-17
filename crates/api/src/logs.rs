//! The rolling log, over HTTP.
//!
//! # Why this exists
//!
//! `crates/telemetry` has held a complete, bounded reader — [`telemetry::tail`]
//! — with **no caller anywhere in the workspace**. So the only way to read a
//! run's log was to find the file on disk and open it in something else, and
//! the first question an operator asks after a failed pull ("what did it say?")
//! had no answer inside the application that wrote it.
//!
//! A live run refused 10 of 16 members in 18 seconds. The receipt named five of
//! them, the audit journal kept one, and the log — which now carries every one
//! — was not reachable from any page.
//!
//! # What it does not do
//!
//! **It does not follow.** No streaming, no polling, no JavaScript: `CLAUDE.md`
//! §2 permits none outside `web/`, and CI gate 1 enforces it. The page is a
//! snapshot with a form on it, and reloading is how you see more.
//!
//! **It does not search the whole history.** [`telemetry::tail`] walks newest
//! file first and stops at [`telemetry::Query::max_scan_bytes`], reporting
//! `hit_scan_cap` when it did. That flag is rendered rather than hidden,
//! because "what I found in the bytes I read" and "everything there is" are
//! different answers and only one of them is true.
//!
//! # Cost
//!
//! Bounded at both ends and by construction, which is what makes it safe to
//! expose. `limit` is capped at [`telemetry::MAX_LIMIT`]; the walk is capped at
//! `max_scan_bytes`; the number of files is `keep_files`. A request cannot make
//! the server read the disk without a bound, whatever the query string says.

use std::fmt::Write as _;

use crate::render;

/// The most events one page will render.
///
/// Smaller than [`telemetry::MAX_LIMIT`] on purpose: this is a page a person
/// reads, and the reader's own ceiling is the ceiling on the *buffer*, not on
/// what is useful to look at. `limit` above this is clamped, never refused —
/// a bookmarked query is not an error.
pub const PAGE_LIMIT: usize = 200;

/// How many bytes one request may read off the log before it stops.
///
/// 4 MiB against a 64 MiB window (8 files × 8 MiB), so a request reads at most
/// the newest sixteenth of the history. The walk says `hit_scan_cap` when it
/// stopped here and the page prints it.
pub const SCAN_BYTES: u64 = 4 * 1024 * 1024;

/// Where the log actually is, taken from the installed sink.
///
/// **Read off the sink rather than recomputed.** `server::served_log_dir`
/// consults the environment and the working directory, and calling it again
/// here would be a second answer to "where is the log" that could disagree with
/// the first — which is the shape of half the defects this repository has
/// recorded. The sink knows the directory it is appending to; nothing else is
/// entitled to an opinion.
///
/// [`None`] when logging was never installed, which is a real state (the sink
/// refuses loudly when its directory is unwritable) and is rendered as one.
fn log_dir() -> Option<std::path::PathBuf> {
    let sink = telemetry::global()?;
    sink.path().parent().map(std::path::Path::to_path_buf)
}

/// One field value as JSON, keeping the type the line carried.
///
/// **Not stringified.** A count that arrives as `Uint(806)` and leaves as
/// `"806"` makes every consumer parse it back, and the first one to forget
/// compares a string. The variants map onto JSON's own types, which is what
/// the log line already did.
///
/// A non-finite float becomes `null`: JSON has no `NaN` or `Infinity`, and
/// emitting the bare word would produce a document no parser accepts. `null`
/// is the honest shape for "a number that is not one".
fn value_json(value: &telemetry::OwnedValue) -> String {
    match *value {
        telemetry::OwnedValue::Str(ref s) => render::json_string(s),
        telemetry::OwnedValue::Int(v) => v.to_string(),
        telemetry::OwnedValue::Uint(v) => v.to_string(),
        telemetry::OwnedValue::Float(v) => {
            if v.is_finite() {
                format!("{v}")
            } else {
                "null".to_owned()
            }
        }
        telemetry::OwnedValue::Bool(v) => v.to_string(),
        telemetry::OwnedValue::Null => "null".to_owned(),
    }
}

/// One field value as text, for the page.
fn value_text(value: &telemetry::OwnedValue) -> String {
    match *value {
        telemetry::OwnedValue::Str(ref s) => s.clone(),
        telemetry::OwnedValue::Int(v) => v.to_string(),
        telemetry::OwnedValue::Uint(v) => v.to_string(),
        telemetry::OwnedValue::Float(v) => format!("{v}"),
        telemetry::OwnedValue::Bool(v) => v.to_string(),
        telemetry::OwnedValue::Null => "null".to_owned(),
    }
}

/// The query a request asked for, already clamped.
struct Asked {
    query: telemetry::Query,
    limit: usize,
    level: Option<telemetry::Level>,
    target: String,
    /// The run the reader narrowed to, or zero for every run.
    run: u64,
    /// Each filter the request carried that could not be read, by name, with
    /// what it said. Such a filter is no narrowing, and the answer says so in
    /// its JSON and with one Warn, rather than passing the unfiltered tail off
    /// as the filtered one (P1-01-02, D-1765).
    ignored: Vec<(&'static str, String)>,
    /// The feed, instrument, day and contract asked for, answered through the
    /// audit index (D-3341 item 3). Empty for a request that names none, which
    /// is answered by the ring alone, exactly as before.
    keys: Keys,
    /// `ring=1`: a keyed answer also reads the newest [`SCAN_BYTES`] of each
    /// ring, for lines no audit stream holds (WP-33c, DB-A44). Off by default:
    /// those rows cannot be paged and move as the ring moves, so they are
    /// asked for, never mixed in.
    ring: bool,
    /// `before=<cursor>`: the next page of a keyed answer, from the `next` a
    /// previous page handed back (WP-33c, DB-A44).
    before: Option<String>,
}

/// Every parameter `/logs.json` and `/logs` read. Any other is named under
/// `ignored` and logged once, never dropped in silence (WP-33c, DB-A45).
pub(crate) const PARAMS: [&str; 17] = [
    "limit",
    "level",
    "target",
    "run",
    "feed",
    "instrument",
    "day",
    "contract",
    "code",
    "ring",
    "before",
    "durable",
    "month",
    "reason",
    "kind",
    "from",
    "to",
];

/// IST's offset from UTC, in milliseconds: a `from` or `to` without a zone is
/// read as IST, the clock every page of this application shows
/// (`crate::server::ist_stamp`).
const IST_OFFSET_MS: i64 = 19_800_000;

/// The instant `text` names, in epoch milliseconds (R3-24): a whole number of
/// milliseconds; a day `YYYY-MM-DD` (its first millisecond, or its last when
/// `end`); or `YYYY-MM-DDTHH:MM` or `YYYY-MM-DDTHH:MM:SS`, whose first (or,
/// when `end`, last) millisecond is meant. A day or time is IST unless it
/// ends in `Z`, which is UTC. `None` for anything else, which the caller
/// names as ignored.
fn instant_of(text: &str, end: bool) -> Option<i64> {
    if !text.is_empty() && text.len() <= 16 && text.bytes().all(|byte| byte.is_ascii_digit()) {
        return text.parse::<i64>().ok();
    }
    let (body, utc) = text
        .strip_suffix('Z')
        .map_or((text, false), |body| (body, true));
    let (day, clock) = body.split_once('T').unwrap_or((body, ""));
    let day = crate::server::import_journal::parse_day(day)?;
    let number = |part: &str, most: i64| -> Option<i64> {
        (part.len() == 2 && part.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| part.parse::<i64>().ok())
            .flatten()
            .filter(|n| *n <= most)
    };
    let (from_ms, width_ms) = if clock.is_empty() {
        (0, 86_400_000)
    } else {
        let mut parts = clock.split(':');
        let hours = number(parts.next()?, 23)?;
        let minutes = number(parts.next()?, 59)?;
        let (seconds, width) = match parts.next() {
            Some(seconds) => (number(seconds, 59)?, 1_000),
            None => (0, 60_000),
        };
        if parts.next().is_some() {
            return None;
        }
        (((hours * 60 + minutes) * 60 + seconds) * 1_000, width)
    };
    let midnight = i64::from(day.days_from_epoch()) * 86_400_000;
    let start = midnight + from_ms - if utc { 0 } else { IST_OFFSET_MS };
    Some(if end { start + width_ms - 1 } else { start })
}

/// A month as a request names it: `YYYY-MM`, the month a real calendar has.
fn is_month(text: &str) -> bool {
    text.len() == 7 && crate::server::import_journal::parse_day(&format!("{text}-01")).is_some()
}

/// The names `query` carries that `known` does not hold, in the order sent,
/// each once. A name is the text before its `=`, as [`crate::server::param`]
/// matches it. O(query length).
pub(crate) fn unknown_params(query: &str, known: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let name = pair.split_once('=').map_or(pair, |(name, _)| name);
        if !known.contains(&name) && !out.iter().any(|held| held == name) {
            out.push(name.to_owned());
        }
    }
    out
}

/// Reads the query string into a bounded [`telemetry::Query`].
///
/// Every parameter is **clamped rather than refused**. A log viewer that
/// answers a mangled bookmark with a 400 is a log viewer an operator stops
/// reaching for, and the worst a bad `limit` can do is show a different count.
///
/// **A bad `level` or `run` is not a count.** It drops a narrowing, so the
/// unfiltered tail would arrive looking like the filtered one. Each is still
/// answered rather than refused, but it is NAMED: listed under `ignored` in the
/// JSON, beside the filters that were applied, and reported by one
/// `api.logs filter ignored` Warn — the bargain `audit_json`'s `page ignored`
/// strikes (P1-01-02, D-1765). `run` must be a canonical unsigned integer;
/// `+3`, `03` and `3.0` are not quietly read as some run.
fn asked(raw: &str) -> Asked {
    // NOT THROUGH `render::query_value`. That is a percent-ENCODER, and
    // `param` has already decoded; encoding a decoded value and then parsing it
    // as a number is backwards. It happened to work for a plain integer — every
    // digit is in the unreserved set — and would have turned any other input
    // into a `%XX` soup that parses to the default, which is the same answer for
    // the wrong reason.
    let limit = crate::server::param(raw, "limit")
        .parse::<usize>()
        .unwrap_or(50)
        .clamp(1, PAGE_LIMIT);
    let level_word = crate::server::param(raw, "level");
    let level = telemetry::Level::of_label(&level_word.to_ascii_lowercase());
    let target = crate::server::param(raw, "target");
    // ZERO IS "EVERY RUN", not run zero. An event outside a backfill omits the
    // key entirely, so there is no run zero to ask for and the value is free to
    // mean the absence of a filter.
    let run_word = crate::server::param(raw, "run");
    let run = run_word
        .parse::<u64>()
        .ok()
        .filter(|run| run.to_string() == run_word)
        .unwrap_or(0);
    let mut ignored = Vec::new();
    if level.is_none() && !level_word.is_empty() {
        ignored.push(("level", level_word));
    }
    if run == 0 && !run_word.is_empty() && run_word != "0" {
        ignored.push(("run", run_word));
    }
    let mut keys = keys_of(raw, &mut ignored);
    let flag = |name: &'static str, ignored: &mut Vec<(&'static str, String)>| {
        let word = crate::server::param(raw, name);
        match word.as_str() {
            "" | "0" => false,
            "1" => true,
            _ => {
                ignored.push((name, word));
                false
            }
        }
    };
    let mut ring = flag("ring", &mut ignored);
    // `durable=1` ASKS THE INDEX FOR A RUN OR A TARGET ALONE. `run=` without
    // it stays the ring's filter it always was, because a page polls it for a
    // live sweep and the durable stream does not hold progress lines.
    if flag("durable", &mut ignored) {
        if run != 0 {
            keys.run = Some(run);
        } else if !target.is_empty() {
            keys.target = Some(target.clone());
        } else {
            ignored.push((
                "durable",
                "1 with no run, target or key to probe the index by".to_owned(),
            ));
        }
    }
    // A TARGET ALONE IS A KEY (R3-24, DB-023): it is answered from the index
    // of every half, all of its history, and on a first page from the newest
    // bytes of each ring too, for its lines no stream audits. It used to read
    // the newest 4 MiB of each ring and stop. `run=` stays the ring's filter
    // for the reason `durable=1` gives above.
    if keys.is_empty() && run == 0 && !target.is_empty() {
        keys.target = Some(target.clone());
        ring = true;
    }
    let before_word = crate::server::param(raw, "before");
    let before = if before_word.is_empty() {
        None
    } else if keys.is_empty() {
        ignored.push(("before", before_word));
        None
    } else {
        Some(before_word)
    };
    for name in unknown_params(raw, &PARAMS) {
        ignored.push(("unknown parameter", name));
    }
    for (param, word) in &ignored {
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::warn("api.logs", "filter ignored")
                .with("param", telemetry::Value::Str(param))
                .with("asked", telemetry::Value::Str(word))
                .with(
                    "why",
                    telemetry::Value::Str("not a value this filter reads; answered without it"),
                ),
        );
    }

    let mut query = telemetry::Query::last(limit);
    query.max_scan_bytes = SCAN_BYTES;
    if let Some(level) = level {
        query = query.at_least(level);
    }
    if !target.is_empty() {
        query = query.from_target(target.clone());
    }
    if run != 0 {
        query = query.from_run(run);
    }
    Asked {
        query,
        limit,
        level,
        target,
        run,
        ignored,
        keys,
        ring,
        before,
    }
}

/// The keyed filters a query names, each unreadable one pushed to `ignored`
/// rather than read as no filter in silence (D-1765's rule, D-3341 item 3).
/// `feed` must be a wire this build names; `instrument` an instrument as
/// [`instrument_of`] reads one; `day` a `YYYY-MM-DD`; `contract` any text up
/// to a store segment's length.
fn keys_of(raw: &str, ignored: &mut Vec<(&'static str, String)>) -> Keys {
    let mut keys = Keys::default();
    let feed = crate::server::param(raw, "feed");
    if !feed.is_empty() {
        match brutex_core::vendor::Vendor::ALL
            .into_iter()
            .find(|vendor| vendor.as_str().eq_ignore_ascii_case(&feed))
        {
            Some(vendor) => keys.feed = Some(vendor.as_str().to_owned()),
            None => ignored.push(("feed", feed)),
        }
    }
    let instrument = crate::server::param(raw, "instrument");
    if !instrument.is_empty() {
        match instrument_of(&instrument) {
            Some(named) => keys.instrument = Some(named),
            None => ignored.push(("instrument", instrument)),
        }
    }
    let day = crate::server::param(raw, "day");
    if !day.is_empty() {
        match crate::server::import_journal::parse_day(&day) {
            Some(parsed) => keys.day = Some(parsed),
            None => ignored.push(("day", day)),
        }
    }
    let contract = crate::server::param(raw, "contract");
    if !contract.is_empty() {
        if contract.len() <= MAX_CONTRACT_ASKED && !contract.contains(['/', '\\']) {
            keys.contract = Some(contract);
        } else {
            ignored.push(("contract", contract));
        }
    }
    // A REFUSAL'S CODE (WP-33c, DB-A47): the `code` (or `reason`) a refusal
    // logged, one posting of the index.
    let code = crate::server::param(raw, "code");
    if !code.is_empty() {
        if code.len() <= MAX_CONTRACT_ASKED && !code.contains(['/', '\\']) {
            keys.code = Some(code);
        } else {
            ignored.push(("code", code));
        }
    }
    // A MONTH, A REASON AND AN IMPORT KIND (R3-24): each one posting of the
    // index, read exactly as a day or a code is.
    let month = crate::server::param(raw, "month");
    if !month.is_empty() {
        if is_month(&month) {
            keys.month = Some(month);
        } else {
            ignored.push(("month", month));
        }
    }
    let reason = crate::server::param(raw, "reason");
    if !reason.is_empty() {
        if reason.len() <= MAX_CONTRACT_ASKED && !reason.contains(['/', '\\']) {
            keys.reason = Some(reason);
        } else {
            ignored.push(("reason", reason));
        }
    }
    let kind = crate::server::param(raw, "kind");
    if !kind.is_empty() {
        match pull::gdfl_import::ImportKind::parse(&kind) {
            Ok(parsed) => keys.kind = Some(parsed.as_str().to_owned()),
            Err(_) => ignored.push(("kind", kind)),
        }
    }
    // A TIME SPAN (R3-24): either end may be left open; one that does not
    // read, or a span that ends before it starts, is named and not applied.
    let (from_word, to_word) = (
        crate::server::param(raw, "from"),
        crate::server::param(raw, "to"),
    );
    let from = (!from_word.is_empty()).then(|| instant_of(&from_word, false));
    let to = (!to_word.is_empty()).then(|| instant_of(&to_word, true));
    match (from, to) {
        (None, None) => {}
        (Some(None), _) => ignored.push(("from", from_word)),
        (_, Some(None)) => ignored.push(("to", to_word)),
        (from, to) => {
            let (from, to) = (
                from.flatten().unwrap_or(i64::MIN),
                to.flatten().unwrap_or(i64::MAX),
            );
            if from <= to {
                keys.span = Some((from, to));
            } else {
                ignored.push(("from", format!("{from_word} is after to={to_word}")));
            }
        }
    }
    keys
}

/// `GET /logs.json` — the tail, and every honesty flag the walk carries.
pub async fn logs_json(
    uri: axum::http::Uri,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let json = [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )];
    // THE SAME FALL-THROUGH AS `logs_page`, AND IT MATTERS MORE HERE.
    //
    // This is the endpoint a poller reads, so a `503` here is what a live view
    // would see forever while a sweep wrote thousands of records to the `cli`
    // half. The server's own sink being absent says nothing about whether
    // `cli` events exist -- and 2,090 of them existed when this was measured.
    // EVERY RUN DIRECTORY, NOT `logs/cli` ALONE (D-4862 round 6; IA-11r R5):
    // a GDFL run of a kind logs to its own directory, and a page that read two
    // of the eight would show none of its events.
    let halves = halves_of(log_dir(), crate::server::store_dir().ok().as_deref());
    if halves.is_empty() {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            json,
            NO_LOG_JSON.to_owned(),
        );
    }
    let asked = asked(uri.query().unwrap_or(""));
    let health = sink_health();
    // OFF THE ASYNC WORKERS, AND ADMITTED. `json_over` walks up to two
    // `SCAN_BYTES` halves and decodes every line it reads, and it ran inline
    // on a Tokio worker with nothing capping how many did so at once -- while
    // the backtest page polls this route every two seconds per running sweep
    // with a `run=` filter one half can never match, so that half is read to
    // its cap on every poll. It now runs in `detail`'s log-read pool, and a
    // full pool is a named 429, never a queue. log-3, P1-04-02, D-2327.
    // A KEYED REQUEST IS ANSWERED FROM THE AUDIT INDEX, then the ring; one
    // that names no key reads the ring alone, exactly as it always did.
    match crate::detail::run_log_read(move || {
        if asked.keys.is_empty() {
            json_across(&halves, &asked, health.as_ref())
        } else {
            keyed_json_across(&halves, &asked, health.as_ref())
        }
    })
    .await
    {
        Ok(body) => (axum::http::StatusCode::OK, json, body),
        Err(why) => {
            let (status, body) = crate::detail::admission_refused(
                "log read",
                crate::detail::MAX_LOG_READ_CONCURRENT,
                &why,
            );
            (status, json, body)
        }
    }
}

/// The JSON body, over a directory a caller names.
///
/// Split from the handler so a test can drive it against a REAL sink writing a
/// REAL file, rather than needing the process-global one installed. The
/// handler is then the one thing left untested here — resolving the directory
/// — and that is a single `?` on `telemetry::global()`.
#[cfg(test)]
fn json_over(
    dir: &std::path::Path,
    cli_dir: Option<&std::path::Path>,
    asked: &Asked,
    health: Option<&telemetry::Health>,
) -> String {
    json_across(&two_halves(dir, cli_dir), asked, health)
}

/// Where `crates/cli` ACTUALLY writes, resolved from the STORE root.
///
/// # The join this replaces was one level off, and it hid 485 events
///
/// [`both_halves`] used to build its second half as `dir.join(CLI_SUBDIR)`,
/// where `dir` is the directory the SERVER's own sink resolved. Under the run
/// configuration this repository ships — `.claude/launch.json` starts `api`
/// from the workspace root — `server::log_dir_from` answers `<workspace>/logs`,
/// so the reader looked in `<workspace>/logs/cli`. That directory does not
/// exist and never has.
///
/// `cli::log_dir_from` resolves `<store>/logs/cli`. Measured on this machine:
/// `~/.brutex/store/logs/cli/events.ndjson` held **485 `cli.*` records** —
/// 392 `ladder level`, 42 `stored span loaded`, 32 `rung sweeping`, 12
/// `threshold search over stored bars`, 7 `exit grid entered` — while the
/// directory `/logs` was reading held **zero**.
///
/// # Why it was silent, and why that is the worse half
///
/// `telemetry::tail` is documented never to fail and to render a missing
/// directory as no records. That is the right behaviour for a store where
/// nobody has run `cli`, and it is exactly what made this invisible: the page
/// answered "no `cli` events" in the same words for "none were written" and
/// "I looked in the wrong place". `CLAUDE.md` §4 bans a fallback that hides a
/// failure, and an absent-directory default that cannot tell those two apart is
/// one wearing a legitimate default's clothes.
///
/// The two halves are resolved from DIFFERENT roots now, deliberately: the
/// server's from its own sink, the `cli` half from the store. Deriving one from
/// the other is what made them able to disagree.
fn cli_half(store: &std::path::Path) -> std::path::PathBuf {
    store.join("logs").join(CLI_SUBDIR)
}

/// Where a CLI-run command writes its records, for a caller outside this module.
///
/// `cli_half` needs the store root and `log_dir` returns the SERVED half, so
/// every caller wanting the other half repeated the same two lines. Two of them
/// already existed here (`logs_json` and `logs_page`); `sweeprun::run_json` is
/// the third, and it is in another module.
///
/// Returns `None` for the same reason those two do -- the store may not be
/// configured -- and never creates: a reader that creates a directory reports a
/// sweep's log as present when nothing has ever written one.
#[must_use]
pub fn cli_log_dir() -> Option<std::path::PathBuf> {
    crate::server::store_dir().ok().map(|s| cli_half(&s))
}

/// One log field, JSON-escaped, for a caller building a response by hand.
///
/// A log MESSAGE is the one string in this crate that an operator's own data
/// reaches: a refusal quotes a vendor's text, a path, a store error. Pasting it
/// into a response with `format!` and no escape is how a stray quote turns a
/// JSON body into a parse error the page reports as "sweep unknown".
#[must_use]
pub fn quoted(raw: &str) -> String {
    crate::render::json_string(raw)
}

/// The subdirectory `cli` appends to, beside the server's own.
///
/// `crates/cli`'s `log_dir` resolves `<store>/logs/cli` and its banner has said
/// so on every run: *"The /logs page reads whatever directory `api` resolved,
/// which is NOT this one."* Its doc names the fix in the same breath — *"until
/// it walks `logs/` and `logs/cli/` the page shows the server's half"* — and
/// this is that walk.
const CLI_SUBDIR: &str = telemetry::run_dirs::CLI;

/// The closed set of directories a run process logs to, under `<store>/logs`
/// (D-4862): `cli` (every cli verb, and every run until the directory
/// switch) and every row of `telemetry::run_dirs::ALL`, each of which
/// belongs to the one lock that makes its writer alone there. With the
/// server's own directory these are the 8 `/logs` reads.
///
/// **The table is `telemetry`'s, not this module's (D-4897).** The writers
/// choose a run's directory through the same table, so a reader cannot look
/// where no writer logs and a ninth writer is a row there, a lock, and
/// `docs/06-limits.md` L-IA-13's bound recomputed.
/// `the_readers_read_the_table_of_run_directories` holds this function to
/// it, and `the_run_directories_name_every_import_kind` holds the table's
/// three kinds to `pull::gdfl_import::ImportKind::ALL`.
pub(crate) fn run_dirs() -> impl Iterator<Item = &'static str> {
    std::iter::once(CLI_SUBDIR).chain(telemetry::run_dirs::ALL.iter().map(|dir| dir.sub))
}

/// Of [`run_dirs`], the five a GDFL run can write, so the five the
/// importer's evidence is read from: for one kind `cli`, `gdfl/<kind>` and
/// `gdfl/store` (`telemetry::run_dirs::gdfl_evidence`).
pub(crate) fn import_dirs() -> impl Iterator<Item = &'static str> {
    telemetry::run_dirs::gdfl_evidence_all()
}

/// Whether a directory a reader was asked to read could answer. A reader
/// names every directory that is not [`Self::Read`], and says "none" only
/// when all of them were read: an absent directory and an empty one are
/// different facts, and so are an unreadable one and both (D-4862, the rule
/// [`cli_half`]'s doc states for one directory).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DirState {
    /// It is there and was read.
    Read,
    /// It does not exist: no run that logs there has ever started.
    Absent,
    /// It is there and could not be read, in the failure's own words.
    Unreadable(String),
}

impl DirState {
    /// The state's word on the wire.
    pub(crate) const fn word(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Absent => "absent",
            Self::Unreadable(_) => "unreadable",
        }
    }

    /// The state of `path`, asked of the disk and never of what this
    /// process read before: one `metadata` call for the directory and, when
    /// it is one, one for its current ring file BY NAME.
    ///
    /// # Why the second call
    ///
    /// `metadata` of a directory answers from its parent, so a directory
    /// nobody may search (mode `000`, or one without its `x` bit) is "there,
    /// and a directory" and used to read `read` while every file in it
    /// answered `EACCES`: `/logs.json` named it `read` beside its own
    /// `errors[]` saying `Permission denied`. A reader reaches a ring file by
    /// its name, so that is what is asked: a name that is not there is a
    /// directory that could answer and holds nothing, and any other failure
    /// is the directory's, in the failure's own words (D-4862: `"none"` only
    /// when every directory was read).
    pub(crate) fn of(path: &std::path::Path) -> Self {
        match std::fs::metadata(path) {
            Ok(meta) if meta.is_dir() => match std::fs::metadata(telemetry::current_path(path)) {
                Ok(_) => Self::Read,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::Read,
                Err(e) => Self::Unreadable(e.to_string()),
            },
            Ok(_) => Self::Unreadable("it is there and is not a directory".to_owned()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::Absent,
            Err(e) => Self::Unreadable(e.to_string()),
        }
    }
}

/// One directory a `/logs` request reads: its name on the wire (`served` for
/// the server's own, else one of [`run_dirs`]) and where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Half {
    /// The name rows and postings carry.
    pub(crate) name: &'static str,
    /// The directory.
    pub(crate) path: std::path::PathBuf,
}

/// Every directory a `/logs` request reads: the server's own, when its sink
/// resolved one, then each of [`run_dirs`] under the store, when a store root
/// resolved: 8 at most (D-4862). The two roots are resolved apart, for the
/// reason [`cli_half`] gives.
pub(crate) fn halves_of(
    served: Option<std::path::PathBuf>,
    store: Option<&std::path::Path>,
) -> Vec<Half> {
    served
        .map(|path| Half {
            name: "served",
            path,
        })
        .into_iter()
        .chain(run_dirs().filter_map(|name| {
            store.map(|root| Half {
                name,
                path: root.join(telemetry::run_dirs::LOGS).join(name),
            })
        }))
        .collect()
}

/// The first directory and the `cli` one, as [`json_of`] and [`page_of`]
/// name them: the pair every answer has always carried.
fn named_pair(halves: &[Half]) -> Option<(&std::path::Path, Option<&std::path::Path>)> {
    let first = halves.first()?;
    let cli = halves
        .iter()
        .skip(1)
        .find(|half| half.name == CLI_SUBDIR)
        .map(|half| half.path.as_path());
    Some((first.path.as_path(), cli))
}

/// `"dirs":[...]`: every directory the request read, by name, with its state
/// (`read`, `absent`, `unreadable` and why). At most two `metadata` calls
/// each ([`DirState::of`]).
fn dirs_json(halves: &[Half]) -> String {
    let mut out = String::from("\"dirs\":[");
    for (n, half) in halves.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        let state = DirState::of(&half.path);
        let _ = write!(
            out,
            r#"{{"half":{},"dir":{},"state":"{}","error":{}}}"#,
            render::json_string(half.name),
            render::json_string(&half.path.display().to_string()),
            state.word(),
            match &state {
                DirState::Unreadable(why) => render::json_string(why),
                DirState::Read | DirState::Absent => "null".to_owned(),
            },
        );
    }
    out.push(']');
    out
}

/// `body`, a JSON object, with [`dirs_json`] and [`reach_json`] as its last
/// members.
fn with_dirs(body: &str, halves: &[Half]) -> String {
    let mut out = body.strip_suffix('}').unwrap_or(body).to_owned();
    out.push(',');
    out.push_str(&dirs_json(halves));
    out.push(',');
    out.push_str(&reach_json(halves, telemetry::now_millis()));
    out.push(',');
    out.push_str(&picture_json(served_picture().as_ref()));
    let (audit, state) = served_streams();
    out.push(',');
    out.push_str(&streams_json(audit.as_ref(), state.as_ref()));
    out.push('}');
    out
}

/// An audit history younger than this many days raises the one alert every
/// log answer carries (R3-24, DB-G88): the operator's evidence for a month
/// ago is then simply not kept.
pub const YOUNG_AUDIT_DAYS: i64 = 30;

/// What every log answer says of how far back it reaches (R3-24): each
/// directory's three streams, the oldest audited line any of them keeps, and
/// one alert when that is younger than [`YOUNG_AUDIT_DAYS`] or there is none.
struct Reached {
    /// Per directory read, its reach.
    halves: Vec<(&'static str, telemetry::run_dirs::Reach)>,
    /// The oldest line either durable stream of any directory keeps.
    audited_from: Option<i64>,
    /// Whole days from it to `now`.
    days: Option<i64>,
    /// The one alert, in words, or `None`.
    alert: Option<String>,
}

/// How far back `halves` reach at `now` (epoch ms). Per directory, what
/// `telemetry::run_dirs::reach_of` reads: a few small reads, never a walk.
fn reached(halves: &[Half], now: i64) -> Reached {
    let halves: Vec<(&'static str, telemetry::run_dirs::Reach)> = halves
        .iter()
        .map(|half| {
            (
                half.name,
                telemetry::run_dirs::reach_of(&half.path, telemetry::DEFAULT_KEEP_FILES),
            )
        })
        .collect();
    let audited_from = halves
        .iter()
        .filter_map(|(_, reach)| reach.audited_from())
        .min();
    let days = audited_from.map(|from| now.saturating_sub(from).div_euclid(86_400_000));
    // NO RENDERED CLOCK IN THE WORDS: the JSON's times are epoch
    // milliseconds (`audited_from`), and the page adds its IST stamp itself.
    let alert = match days {
        Some(days) if days < YOUNG_AUDIT_DAYS => Some(format!(
            "the audit history reaches back {days} day(s): younger than {YOUNG_AUDIT_DAYS} days, \
             so nothing audited before then can be answered"
        )),
        Some(_) => None,
        _ => Some(
            "no directory read keeps an audit history: nothing audited can be answered for any \
             time"
                .to_owned(),
        ),
    };
    Reached {
        halves,
        audited_from,
        days,
        alert,
    }
}

/// `"reach":{...}`: [`reached`] as JSON, each time in epoch milliseconds or
/// `null` for a stream with no line.
fn reach_json(halves: &[Half], now: i64) -> String {
    format!("\"reach\":{}", reach_object(halves, now))
}

/// The log directories of a store an `/audit.json` answer reads (R3-24): the
/// store's own `logs` (where a server started on it logs) as `served`, then
/// every run directory.
pub(crate) fn store_halves(store_root: &std::path::Path) -> Vec<Half> {
    halves_of(
        Some(store_root.join(telemetry::run_dirs::LOGS)),
        Some(store_root),
    )
}

/// The most state-change lines `/audit.json` names (R3-24).
pub const STATE_CHANGES_SHOWN: usize = 20;

/// The bytes read from the end of one state stream for `/audit.json`'s
/// state changes: a fixed slice, whatever the stream holds.
const STATE_TAIL_BYTES: u64 = 64 * 1024;

/// The newest state-change lines of every directory in `halves`, newest
/// first, at most `limit`, each with its class (R3-24): what `/audit.json`
/// shows of who did what. Per directory one read of at most
/// [`STATE_TAIL_BYTES`] at the end of its state stream, which never rolls,
/// so the newest lines are always there; a stream that is not there is
/// passed over, one that cannot be read is named in `unread`.
pub(crate) fn state_changes_json(halves: &[Half], limit: usize) -> String {
    use std::io::{Read as _, Seek as _};
    let mut found: Vec<(&'static str, telemetry::Record)> = Vec::new();
    let mut unread: Vec<String> = Vec::new();
    for half in halves {
        let path = telemetry::audit_current_path(&telemetry::run_dirs::state_stream_of(&half.path));
        let len = match std::fs::metadata(&path) {
            Ok(meta) => meta.len(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                unread.push(format!("{}: {e}", path.display()));
                continue;
            }
        };
        let from = len.saturating_sub(STATE_TAIL_BYTES);
        let mut bytes = Vec::new();
        if let Err(e) = std::fs::File::open(&path).and_then(|mut file| {
            file.seek(std::io::SeekFrom::Start(from))?;
            file.take(STATE_TAIL_BYTES).read_to_end(&mut bytes)
        }) {
            unread.push(format!("{}: {e}", path.display()));
            continue;
        }
        // A READ THAT BEGAN MID-FILE begins mid-line: its first piece is not
        // a line.
        let lines = bytes
            .split(|&byte| byte == b'\n')
            .skip(usize::from(from > 0));
        found.extend(
            lines
                .filter_map(|line| telemetry::Record::decode(line).ok())
                .map(|record| (half.name, record)),
        );
    }
    found.sort_by(|left, right| {
        right
            .1
            .at_unix_millis
            .cmp(&left.1.at_unix_millis)
            .then_with(|| right.1.seq.cmp(&left.1.seq))
    });
    found.truncate(limit);
    let rows: Vec<String> = found
        .iter()
        .map(|(half, record)| {
            let class =
                telemetry::run_dirs::state_events::class_of(&record.target, &record.message)
                    .unwrap_or(telemetry::run_dirs::state_events::Class::Run);
            format!(
                r#"{{"half":{},"ts":{},"level":{},"target":{},"message":{},"class":{},"run":{}}}"#,
                render::json_string(half),
                record.at_unix_millis,
                render::json_string(record.level.label()),
                render::json_string(&record.target),
                render::json_string(&record.message),
                render::json_string(class.as_str()),
                run_of(record).map_or_else(|| "null".to_owned(), |run| format!("\"{run}\"")),
            )
        })
        .collect();
    let unread: Vec<String> = unread.iter().map(|why| render::json_string(why)).collect();
    format!(
        r#"{{"rows":[{}],"limit":{limit},"unread":[{}],"from":"the never-rolling state stream of each log directory"}}"#,
        rows.join(","),
        unread.join(","),
    )
}

/// [`reached`] as one JSON object.
pub(crate) fn reach_object(halves: &[Half], now: i64) -> String {
    let reach = reached(halves, now);
    let at = |ms: Option<i64>| ms.map_or_else(|| "null".to_owned(), |ms| ms.to_string());
    let mut out = String::from("{\"halves\":[");
    for (n, (half, one)) in reach.halves.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            r#"{{"half":{},"ring_from":{},"audit_from":{},"state_from":{}}}"#,
            render::json_string(half),
            at(one.ring_from),
            at(one.audit_from),
            at(one.state_from),
        );
    }
    let _ = write!(
        out,
        r#"],"audited_from":{},"days":{},"young_below_days":{YOUNG_AUDIT_DAYS},"alert":{}}}"#,
        at(reach.audited_from),
        at(reach.days),
        reach
            .alert
            .as_deref()
            .map_or_else(|| "null".to_owned(), render::json_string),
    );
    out
}

/// `"picture":{...}`: the served sink's counters (R3-24) as JSON, or `null`
/// for a process with no sink. Read from [`telemetry::Sink::picture`], a copy
/// of a fixed-size table: no file is opened.
fn picture_json(picture: Option<&telemetry::run_dirs::Picture>) -> String {
    let Some(p) = picture else {
        return "\"picture\":null".to_owned();
    };
    let mut out = String::from("\"picture\":{\"levels\":[");
    for (n, level) in telemetry::LEVELS.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        out.push_str(&render::json_string(level.label()));
    }
    out.push_str("],\"minutes\":[");
    for (n, (from, counts)) in p.minutes.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        let counts: Vec<String> = counts.iter().map(u64::to_string).collect();
        let _ = write!(out, r#"{{"from":{from},"counts":[{}]}}"#, counts.join(","));
    }
    out.push_str("],\"codes\":[");
    for (n, (code, count)) in p.codes.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            r#"{{"code":{},"count":{count}}}"#,
            render::json_string(code)
        );
    }
    let _ = write!(
        out,
        r#"],"codes_replaced":{},"oldest_ms":{},"counted":{},"since_ms":{},"from":"counters"}}"#,
        p.codes_replaced,
        p.oldest_ms
            .map_or_else(|| "null".to_owned(), |ms| ms.to_string()),
        p.counted,
        p.since_ms,
    );
    out
}

/// The served sink's picture, when this process writes a log.
fn served_picture() -> Option<telemetry::run_dirs::Picture> {
    telemetry::global().map(telemetry::Sink::picture)
}

/// `"audit_stream":{...},"state_stream":{...}`: both durable streams of the
/// served sink, their counters and the roll's waiting deletes (R3-24: the
/// waiting-delete counters are served on every answer, not only a keyed
/// one).
fn streams_json(
    audit: Option<&telemetry::AuditHealth>,
    state: Option<&telemetry::AuditHealth>,
) -> String {
    format!(
        r#""audit_stream":{},"state_stream":{}"#,
        audit_json_of(audit),
        audit_json_of(state)
    )
}

/// The page's line for both durable streams of the served sink: their size,
/// and the index generations a roll renamed aside that wait to be deleted.
fn streams_note(
    audit: Option<&telemetry::AuditHealth>,
    state: Option<&telemetry::AuditHealth>,
) -> String {
    let Some(audit) = audit else {
        return "<p class=\"lead\">This process keeps no audit stream.</p>".to_owned();
    };
    let mut out = format!(
        "<p class=\"{}\">Audit stream: file {} at {} byte(s), {} roll(s); {} index \
         generation(s) wait to be deleted, {} deleted, {} delete(s) failed{}.",
        if audit.is_loud() { "halt" } else { "lead" },
        audit.serial,
        audit.current_bytes,
        audit.rotations,
        audit.gone_waiting,
        audit.reaped,
        audit.reap_failures,
        if audit.reaper_behind() {
            " — the deletes are behind the rolls"
        } else {
            ""
        },
    );
    if let Some(state) = state {
        let _ = write!(
            out,
            " State stream (never rolls): {} byte(s), {} line(s) written by this process{}.",
            state.current_bytes,
            state.written,
            if state.grown_past_loud() {
                " — past its loud size: look at what writes it"
            } else {
                ""
            },
        );
    }
    if let Some(why) = audit.last_error.as_deref() {
        let _ = write!(out, " Most recent failure: {}.", render::escape(why));
    }
    out.push_str("</p>");
    out
}

/// The served sink's two durable streams' health.
fn served_streams() -> (
    Option<telemetry::AuditHealth>,
    Option<telemetry::AuditHealth>,
) {
    telemetry::global().map_or((None, None), |sink| {
        (sink.audit_health(), sink.state_health())
    })
}

/// The colour each level's bar is drawn in, by rank.
const LEVEL_FILL: [&str; 5] = ["#cbd5e1", "#94a3b8", "#4f46e5", "#d97706", "#dc2626"];

/// The `/logs` picture (R3-24), drawn here from the served sink's counters:
/// an hour of lines per minute, stacked by level, the codes written most and
/// the ring's oldest line. Plain HTML and one inline SVG, no script. `None`
/// draws the one line that says this process keeps no counters.
fn picture_note(picture: Option<&telemetry::run_dirs::Picture>) -> String {
    /// One minute's bar, wide, in the picture's own units.
    const BAR: usize = 8;
    /// The tallest bar.
    const HIGH: u64 = 64;
    let Some(p) = picture else {
        return "<p class=\"lead\">No picture: this process writes no log, so it counted \
                nothing.</p>"
            .to_owned();
    };
    let width = BAR * p.minutes.len();
    let most = p
        .minutes
        .iter()
        .map(|(_, counts)| counts.iter().sum::<u64>())
        .max()
        .unwrap_or(0)
        .max(1);
    let mut bars = String::new();
    for (n, (from, counts)) in p.minutes.iter().enumerate() {
        let mut top = HIGH;
        for (rank, count) in counts.iter().enumerate() {
            if *count == 0 {
                continue;
            }
            let tall = (count.saturating_mul(HIGH).div_ceil(most)).min(top);
            top = top.saturating_sub(tall);
            let _ = write!(
                bars,
                "<rect x=\"{}\" y=\"{top}\" width=\"{}\" height=\"{tall}\" fill=\"{}\">\
                 <title>{} · {count} {}</title></rect>",
                n.saturating_mul(BAR),
                BAR.saturating_sub(1),
                LEVEL_FILL.get(rank).copied().unwrap_or("#000"),
                render::escape(&crate::server::ist_stamp(from.div_euclid(1_000))),
                telemetry::LEVELS
                    .get(rank)
                    .map_or("?", |level| level.label()),
            );
        }
    }
    let mut legend = String::new();
    for (rank, level) in telemetry::LEVELS.iter().enumerate() {
        let total: u64 = p
            .minutes
            .iter()
            .map(|(_, counts)| counts.get(rank).copied().unwrap_or(0))
            .sum();
        let _ = write!(
            legend,
            " <span style=\"color:{}\">■</span> {} {total};",
            LEVEL_FILL.get(rank).copied().unwrap_or("#000"),
            level.label(),
        );
    }
    let mut codes = String::new();
    for (code, count) in &p.codes {
        let _ = write!(
            codes,
            "<tr><td class=\"mono\">{}</td><td>{count}</td></tr>",
            render::escape(code)
        );
    }
    format!(
        "<section class=\"lead\"><b>The last hour, per minute</b> (from the sink's own \
         counters, kept as it writes; no file read):{legend} peak {most} a minute.\
         <svg role=\"img\" viewBox=\"0 0 {width} {HIGH}\" width=\"100%\" height=\"{HIGH}\" \
         preserveAspectRatio=\"none\"><title>lines per minute by level</title>{bars}</svg>\
         <br>Oldest line the ring holds: {oldest}. {counted} line(s) counted since {since}.\
         {replaced}</section>{table}",
        oldest = render::escape(&p.oldest_ms.map_or_else(
            || "none".to_owned(),
            |ms| crate::server::ist_stamp(ms.div_euclid(1_000))
        )),
        counted = p.counted,
        since = render::escape(&crate::server::ist_stamp(p.since_ms.div_euclid(1_000))),
        replaced = if p.codes_replaced > 0 {
            format!(
                " The code table is full and a new code took the least-counted one's place \
                 {} time(s): a count may be over by the count it took over.",
                p.codes_replaced
            )
        } else {
            String::new()
        },
        table = if codes.is_empty() {
            "<p class=\"lead\">No line carried a code.</p>".to_owned()
        } else {
            format!(
                "<div class=\"hscroll\"><table class=\"codes\"><thead><tr><th>Code</th>\
                 <th>Lines</th></tr></thead><tbody>{codes}</tbody></table></div>"
            )
        },
    )
}

/// The page's reach line, and its one alert when there is one.
fn reach_note(halves: &[Half], now: i64) -> String {
    let reach = reached(halves, now);
    let mut out = String::new();
    if let Some(ref alert) = reach.alert {
        let _ = write!(
            out,
            "<p class=\"halt\"><b>Audit history is short</b>{}{}.</p>",
            render::escape(alert),
            reach.audited_from.map_or_else(String::new, |from| format!(
                " (its oldest line: {})",
                render::escape(&crate::server::ist_stamp(from.div_euclid(1_000)))
            )),
        );
    }
    out.push_str("<p class=\"lead\">Reaches back:");
    for (half, one) in &reach.halves {
        let when = |ms: Option<i64>| {
            ms.map_or_else(
                || "nothing".to_owned(),
                |ms| crate::server::ist_stamp(ms.div_euclid(1_000)),
            )
        };
        let _ = write!(
            out,
            " <code>{}</code> ring {}, audit {}, state {};",
            render::escape(half),
            render::escape(&when(one.ring_from)),
            render::escape(&when(one.audit_from)),
            render::escape(&when(one.state_from)),
        );
    }
    out.push_str("</p>");
    out
}

/// The page's line naming every directory read and the state of each, so a
/// run directory that does not exist yet reads `absent` and never as a
/// directory that held nothing.
fn dirs_note(halves: &[Half]) -> String {
    let mut out = String::from("<p class=\"lead\">Directories read:");
    for half in halves {
        let state = DirState::of(&half.path);
        let _ = write!(
            out,
            " <code>{}</code> {}{};",
            render::escape(half.name),
            state.word(),
            match &state {
                DirState::Unreadable(why) => format!(" ({})", render::escape(why)),
                DirState::Read | DirState::Absent => String::new(),
            },
        );
    }
    out.push_str("</p>");
    out
}

/// Every half of the log, merged newest-first: [`merged`] folded over each
/// directory's own walk, so the limit, the caps and `missing` combine exactly
/// as they did for two. At most 8 walks of at most [`SCAN_BYTES`] each.
fn all_halves(halves: &[Half], query: &telemetry::Query) -> telemetry::Tail {
    all_halves_live(halves, query, telemetry::global())
}

/// [`all_halves`], the directory `live` writes answered from its copy in
/// memory ([`live_tail`], R3-24) and every other directory from its files.
fn all_halves_live(
    halves: &[Half],
    query: &telemetry::Query,
    live: Option<&telemetry::Sink>,
) -> telemetry::Tail {
    let mut walks =
        halves.iter().map(
            |half| match live.filter(|sink| sink.dir() == half.path.as_path()) {
                Some(sink) => live_tail(sink, &half.path, query),
                None => telemetry::tail(&half.path, telemetry::DEFAULT_KEEP_FILES, query),
            },
        );
    let Some(first) = walks.next() else {
        return telemetry::Tail::default();
    };
    walks.fold(first, |so_far, next| merged(so_far, next, query.limit))
}

/// Whether the live tail keeps `record` for `query`: its floor, its run (the
/// line's own stamp, as the ring's filter reads it), its target or beneath it,
/// and its `since`. The rule `telemetry::Query` applies to a file's lines.
fn live_admits(query: &telemetry::Query, record: &telemetry::Record) -> bool {
    query
        .min_level
        .is_none_or(|floor| record.level.at_least(floor))
        && query.run.is_none_or(|run| record.run == run)
        && query
            .target
            .as_deref()
            .is_none_or(|target| target_admits(&record.target, target))
        && query
            .since_unix_millis
            .is_none_or(|since| record.at_unix_millis >= since)
}

/// The served directory's tail from the copy its sink keeps in memory
/// (R3-24): the newest [`telemetry::run_dirs::RECENT_LINES`] lines, filtered,
/// with no file opened and no byte read. A copy that cannot fill the page
/// (a filter that admits fewer lines than asked) is not passed off as the
/// whole answer: the files are read, exactly as before.
///
/// # Cost
///
/// One lock and a copy of at most [`telemetry::run_dirs::RECENT_LINES`]
/// lines, then a decode of each outside the lock: a constant of this build,
/// whatever the ring holds. Only the fallback reads the files.
fn live_tail(
    sink: &telemetry::Sink,
    dir: &std::path::Path,
    query: &telemetry::Query,
) -> telemetry::Tail {
    let (records, malformed) = sink.recent();
    let kept: Vec<telemetry::Record> = records
        .into_iter()
        .filter(|record| live_admits(query, record))
        .take(query.limit)
        .collect();
    if kept.len() < query.limit {
        return telemetry::tail(dir, telemetry::DEFAULT_KEEP_FILES, query);
    }
    // A GAP IN `seq` among the copies is a line the sink numbered and did not
    // keep; under a filter a gap means nothing, as the file tail says.
    let filtered = query.min_level.is_some()
        || query.target.is_some()
        || query.run.is_some()
        || query.since_unix_millis.is_some();
    let missing = (!filtered).then(|| {
        kept.windows(2)
            .map(|pair| {
                pair.first().zip(pair.get(1)).map_or(0, |(newer, older)| {
                    newer.seq.saturating_sub(older.seq).saturating_sub(1)
                })
            })
            .sum::<u64>()
    });
    telemetry::Tail {
        records: kept,
        bytes_read: 0,
        files_read: 0,
        malformed,
        hit_scan_cap: false,
        reached_oldest: false,
        partial_tail: false,
        errors: Vec::new(),
        missing,
    }
}

/// [`json_over`] across every directory in `halves`.
fn json_across(halves: &[Half], asked: &Asked, health: Option<&telemetry::Health>) -> String {
    let Some(pair) = named_pair(halves) else {
        return NO_LOG_JSON.to_owned();
    };
    let tail = all_halves(halves, &asked.query);
    let marks = contradicted(&tail.records, store_of(halves).as_deref());
    with_dirs(&json_of_marked(&tail, asked, health, pair, &marks), halves)
}

/// The message the GDFL importer closes a day `done` with (its
/// `summary_words`): the line the import journal can contradict.
pub(crate) const DAY_IMPORTED: &str = "day imported";

/// A "day imported" line the import journal contradicts (DB-A48, R3-24).
///
/// # Why a log line can be wrong about its own day
///
/// Before the importer's fix of 10 Oct (`3cd0623c`), a day the census refused
/// still closed in the log as "day imported" at `info`: 212 options days the
/// journal closed `incomplete`, and two days the journal holds no line for at
/// all. The journal is the record (`pull::gdfl_import`'s closing lines); the
/// log is what a run said as it went. So a reader that shows such a line
/// shows the journal's outcome beside it and never lets the line stand alone
/// as the answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Contradicted {
    /// The journal's outcome for the day's key, as `/imports.json` spells it,
    /// or `no journal line` when the journal holds none for it.
    pub(crate) journal: &'static str,
    /// The journal's deciding line, from 1; `None` with no line.
    pub(crate) line: Option<u64>,
}

impl Contradicted {
    /// As a record's `conflict` member.
    fn json(&self) -> String {
        format!(
            r#"{{"with":"import journal","journal_outcome":{},"journal_line":{},"note":{}}}"#,
            render::json_string(self.journal),
            self.line
                .map_or_else(|| "null".to_owned(), |line| line.to_string()),
            render::json_string(&self.words()),
        )
    }

    /// In the operator's words.
    fn words(&self) -> String {
        match self.line {
            Some(line) => format!(
                "the import journal closed this day {} (line {line}): it was not imported, \
                 whatever this line says",
                self.journal
            ),
            None => "the import journal holds no line for this day: it was not imported, \
                     whatever this line says"
                .to_owned(),
        }
    }
}

/// The store root a set of halves was resolved under: the directory two
/// levels (or three, for `gdfl/<kind>`) above any run directory. `None` when
/// only the server's own directory was read.
fn store_of(halves: &[Half]) -> Option<std::path::PathBuf> {
    halves
        .iter()
        .find(|half| half.name != "served")
        .and_then(|half| {
            let depth = std::path::Path::new(telemetry::run_dirs::LOGS)
                .join(half.name)
                .components()
                .count();
            half.path
                .ancestors()
                .nth(depth)
                .map(std::path::Path::to_path_buf)
        })
}

/// Per record of `records`, what the import journal under `store` says
/// against it: `Some` for a "day imported" line whose day's key the journal
/// closed other than `done` or holds no line for, `None` for every other
/// record (and for every record when there is no journal to compare with).
///
/// # Cost
///
/// One [`crate::server::import_journal::snapshot`] (a tail: only the bytes
/// appended since its last read), taken only when a record is a "day
/// imported" line, and one range probe of one day per such record:
/// O(log days) each, at most the page's limit of them.
fn contradicted(
    records: &[telemetry::Record],
    store: Option<&std::path::Path>,
) -> Vec<Option<Contradicted>> {
    use crate::server::import_journal::{parse_day, snapshot};
    let mut journal: Option<Option<std::sync::Arc<crate::server::import_journal::Parsed>>> = None;
    records
        .iter()
        .map(|record| {
            if record.target != pull::gdfl_import::TARGET || record.message != DAY_IMPORTED {
                return None;
            }
            let kind = pull::gdfl_import::ImportKind::parse(text_of(record, "kind")?).ok()?;
            let day = parse_day(text_of(record, "day")?)?;
            let parsed = journal
                .get_or_insert_with(|| {
                    store
                        .and_then(|root| snapshot(root).ok())
                        .filter(|read| read.present)
                        .map(|read| read.parsed)
                })
                .as_ref()?;
            let mut rows = parsed.range(day, day).filter(|row| row.kind == kind);
            // A MEMBER KEY'S CLOSE names its key; a day's does not, and its
            // key is the whole tree's (`*`) when the journal holds one.
            let row = if let Some(key) = text_of(record, "key") {
                rows.find(|row| key.ends_with(&format!(" {}", row.filter)))
            } else {
                let rows: Vec<_> = rows.collect();
                rows.iter()
                    .find(|row| row.filter == "*")
                    .or_else(|| rows.first())
                    .copied()
            };
            match row {
                Some(row) if row.outcome.is_done() => None,
                Some(row) => Some(Contradicted {
                    journal: row.outcome.as_str(),
                    line: Some(row.line),
                }),
                None => Some(Contradicted {
                    journal: "no journal line",
                    line: None,
                }),
            }
        })
        .collect()
}

/// What a request answers when no directory resolved at all.
const NO_LOG_JSON: &str = r#"{"error":"logging is not installed in this process and no store log directory could be resolved, so there is no log to read"}"#;

/// The server's directory and, when given, the `cli` one, as halves: the
/// two-directory form the tests of this module state their fixtures in.
#[cfg(test)]
fn two_halves(dir: &std::path::Path, cli_dir: Option<&std::path::Path>) -> Vec<Half> {
    std::iter::once(Half {
        name: "served",
        path: dir.to_path_buf(),
    })
    .chain(cli_dir.map(|cli| Half {
        name: CLI_SUBDIR,
        path: cli.to_path_buf(),
    }))
    .collect()
}

/// Both halves of the log, merged newest-first.
///
/// # The half of the audit trail the dashboard could not see
///
/// `telemetry::Config` takes a DIRECTORY and `sink::BASENAME` is a constant, so
/// two live processes appending to one directory would interleave lines in one
/// file — a corrupted record of the one thing that exists to say what happened.
/// `cli` therefore owns `<store>/logs/cli` and the server owns `<store>/logs`.
/// That split is correct and is not changed here.
///
/// What was missing is the READER. `/logs` walked the server's directory alone,
/// so every event a terminal-run sweep wrote — every span loaded, every derived
/// threshold, every month found or missing — was on disk and invisible on the
/// page that exists to show it. `CLAUDE.md`'s own requirement is that if it
/// runs, it must be auditable; a log written where nothing reads it meets the
/// letter and not the rule.
///
/// # Merged on TIME, never on sequence
///
/// Each sink numbers its own events from its own run, so `seq` is unique within
/// a directory and meaningless across two. Ordering the union by `seq` would
/// interleave a `cli` event from this morning with a server event from last
/// week wherever the counters happened to collide. `at_unix_millis` is the one
/// field both sets measure against the same clock.
///
/// The tie-break is `seq` descending, so two events stamped in the same
/// millisecond keep a stable order rather than one the sort chose — §3 rule 5
/// applies to a page's row order as much as to a total.
///
/// # `missing` is SUMMED, and that is the honest reading
///
/// Each half computes its own count of events its own sequence says are gone.
/// Those are two independent facts about two independent files, and their sum
/// is "events this page cannot show you that the sinks say existed". `None`
/// from a half means the question is unanswerable there — under a filter, or
/// with no directory — and a half that cannot answer contributes nothing rather
/// than a zero, because zero would read as "none lost".
#[cfg(test)]
fn both_halves(
    dir: &std::path::Path,
    cli_dir: Option<&std::path::Path>,
    query: &telemetry::Query,
) -> telemetry::Tail {
    let served = telemetry::tail(dir, telemetry::DEFAULT_KEEP_FILES, query);
    // AN ABSENT DIRECTORY IS AN EMPTY ANSWER, not an error: `telemetry::tail`
    // is documented never to fail and to render a missing directory as no
    // records. A store where nobody has run `cli` simply has no second half.
    let ran = cli_dir.map_or_else(telemetry::Tail::default, |d| {
        telemetry::tail(d, telemetry::DEFAULT_KEEP_FILES, query)
    });
    merged(served, ran, query.limit)
}

/// The union of two walked halves, and the completeness flags merged in the
/// direction that cannot over-promise (LG-05).
///
/// Split from [`both_halves`] so a test can hand it two walks it cannot cause
/// from real files, such as a half that hit its scan cap beside one that did
/// not (P12-02, D-1791). Behaviour is unchanged.
fn merged(served: telemetry::Tail, ran: telemetry::Tail, limit: usize) -> telemetry::Tail {
    let mut records = served.records;
    records.extend(ran.records);
    records.sort_by(|left, right| {
        right
            .at_unix_millis
            .cmp(&left.at_unix_millis)
            .then_with(|| right.seq.cmp(&left.seq))
    });
    // THE LIMIT APPLIES TO THE UNION. Each half already honoured it, so without
    // this the page would return up to twice what was asked for -- and the
    // caller's `limit` is what bounds the response, not a suggestion.
    records.truncate(limit);

    telemetry::Tail {
        records,
        bytes_read: served.bytes_read.saturating_add(ran.bytes_read),
        files_read: served.files_read.saturating_add(ran.files_read),
        malformed: served.malformed.saturating_add(ran.malformed),
        // EITHER HALF HITTING ITS CAP CAPS THE ANSWER. A page that said `false`
        // because the server's half fitted, while the `cli` half stopped short,
        // would promise completeness it does not have.
        hit_scan_cap: served.hit_scan_cap || ran.hit_scan_cap,
        // AND ONLY BOTH REACHING THE OLDEST MEANS THE OLDEST WAS REACHED.
        // `walked` initialises this `true` and clears it when it finds an older
        // file still holding bytes, so a directory that does not exist -- a
        // store where nobody has run `cli` -- answers `true` and cannot drag
        // the merged answer down. Checked rather than assumed: the same field
        // was once `false` on ordinary full pages, which is the defect its own
        // doc records.
        reached_oldest: served.reached_oldest && ran.reached_oldest,
        partial_tail: served.partial_tail || ran.partial_tail,
        errors: {
            let mut errors = served.errors;
            errors.extend(ran.errors);
            errors
        },
        missing: match (served.missing, ran.missing) {
            (Some(left), Some(right)) => Some(left.saturating_add(right)),
            (Some(only), None) | (None, Some(only)) => Some(only),
            (None, None) => None,
        },
    }
}

/// [`json_over`] over a walk a caller already has.
///
/// **Split so a test can state a walk it cannot cause.** `hit_scan_cap` needs
/// four megabytes of log, `partial_tail` needs a writer interrupted mid-line,
/// and `errors` needs an unreadable file — none of them reachable from a unit
/// test, so every branch that reports one would otherwise render for the first
/// time in production. `Tail`'s fields are `pub`; the state can be stated.
#[cfg(test)]
fn json_of(
    tail: &telemetry::Tail,
    asked: &Asked,
    health: Option<&telemetry::Health>,
    read: (&std::path::Path, Option<&std::path::Path>),
) -> String {
    json_of_marked(tail, asked, health, read, &[])
}

/// [`json_of`], with each record the import journal contradicts carrying its
/// `conflict` beside it (`marks[n]` for `tail.records[n]`, DB-A48). A record
/// with no mark carries no `conflict` member, so every other answer is
/// byte for byte what it was.
fn json_of_marked(
    tail: &telemetry::Tail,
    asked: &Asked,
    health: Option<&telemetry::Health>,
    read: (&std::path::Path, Option<&std::path::Path>),
    marks: &[Option<Contradicted>],
) -> String {
    let mut out = String::from("{\"records\":[");
    for (n, record) in tail.records.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            r#"{{"seq":{},"run":{},"run_key":"{}","ts":{},"level":{},"target":{},"message":{},"cut":{},"dropped_fields":{}"#,
            record.seq,
            record.run,
            record.run,
            record.at_unix_millis,
            render::json_string(record.level.label()),
            render::json_string(&record.target),
            render::json_string(&record.message),
            record.cut,
            record.dropped_fields,
        );
        if let Some(Some(mark)) = marks.get(n) {
            let _ = write!(out, r#","conflict":{}"#, mark.json());
        }
        out.push_str(",\"fields\":{");
        // The durable invocation namespace exceeds JavaScript's safe integer
        // range. Derive aliases from typed Rust values, before JSON can round
        // them. Preserve an explicitly supplied alias so a strict consumer can
        // reject contradictions; never emit a duplicate object key.
        let exact_attempt = record
            .field("attempt")
            .and_then(telemetry::OwnedValue::as_u64)
            .filter(|_| record.field("attempt_key").is_none());
        if let Some(attempt) = exact_attempt {
            let _ = write!(out, r#""attempt_key":"{attempt}""#);
        }
        for (n, (key, value)) in record.fields.iter().enumerate() {
            if n > 0 || exact_attempt.is_some() {
                out.push(',');
            }
            let _ = write!(out, "{}:{}", render::json_string(key), value_json(value));
        }
        out.push_str("}}");
    }
    // THE WALK'S OWN LIMITS, IN THE ANSWER. A caller that cannot tell a
    // complete answer from a truncated one will treat every answer as
    // complete, which is how "no errors in the log" comes to mean "no errors
    // in the four megabytes I happened to read".
    let _ = write!(
        out,
        r#"],"served_dir":{},"cli_dir":{},"bytes_read":{},"files_read":{},"malformed":{},"partial_tail":{},"hit_scan_cap":{},"reached_oldest":{},"scan_cap_bytes":{},"limit":{},"missing":{},"errors":["#,
        render::json_string(&read.0.display().to_string()),
        read.1.map_or_else(
            || String::from("null"),
            |p| render::json_string(&p.display().to_string()),
        ),
        tail.bytes_read,
        tail.files_read,
        tail.malformed,
        tail.partial_tail,
        tail.hit_scan_cap,
        tail.reached_oldest,
        SCAN_BYTES,
        asked.limit,
        // `null` under a filter, because a filter skips records on purpose and
        // a gap then means nothing. A NUMBER is an exact count of events the
        // sink numbered and this file does not hold.
        tail.missing
            .map_or_else(|| "null".to_owned(), |n| n.to_string()),
    );
    for (n, why) in tail.errors.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        out.push_str(&render::json_string(why));
    }
    // THE FILTERS THIS ANSWER APPLIED, AND THE ONES IT COULD NOT. Without them
    // `?level=eror` and no level at all answered byte-identically (D-1765).
    // The run twice, as each record carries it: a number, and the exact
    // decimal a JavaScript reader cannot round.
    let (run, run_key) = if asked.run == 0 {
        ("null".to_owned(), "null".to_owned())
    } else {
        (asked.run.to_string(), format!("\"{}\"", asked.run))
    };
    let _ = write!(
        out,
        r#"],"level":{},"target":{},"run":{run},"run_key":{run_key},"ignored":["#,
        asked.level.map_or_else(
            || "null".to_owned(),
            |level| render::json_string(level.label())
        ),
        if asked.target.is_empty() {
            "null".to_owned()
        } else {
            render::json_string(&asked.target)
        },
    );
    for (n, (param, word)) in asked.ignored.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            r#"{{"param":{},"asked":{}}}"#,
            render::json_string(param),
            render::json_string(word)
        );
    }
    out.push_str("],\"sink\":");
    out.push_str(&sink_json(health));
    out.push('}');
    out
}

/// Whether the banner and `/logs.json`'s `loud` must speak.
///
/// Wider than [`telemetry::Health::is_loud`], which stays "were events lost"
/// because the dropped-events note keys on it. A held clock and a standing
/// `last_error` lose no event, but each is something the operator must be told:
/// `clock_held` was counted and rendered nowhere, so after a clock jump every
/// event kept a frozen time while the page said "Sink healthy", and
/// `last_error` was shown only beside a loss (conc9-3, D-1774).
const fn banner_is_loud(h: &telemetry::Health) -> bool {
    h.is_loud() || h.clock_held > 0 || h.last_error.is_some()
}

/// The WRITE side of the log, as JSON. `null` when this process has no sink.
fn sink_json(health: Option<&telemetry::Health>) -> String {
    let Some(h) = health else {
        return "null".to_owned();
    };
    let mut out = String::new();
    let _ = write!(
        out,
        r#"{{"written":{},"dropped":{},"rotations":{},"rotation_failures":{},"current_bytes":{},"next_seq":{},"clock_held":{},"rationed":{},"loud":{},"last_error":{}}}"#,
        h.written,
        h.dropped,
        h.rotations,
        h.rotation_failures,
        h.current_bytes,
        h.next_seq,
        h.clock_held,
        h.rationed,
        banner_is_loud(h),
        h.last_error
            .as_deref()
            .map_or_else(|| "null".to_owned(), render::json_string),
    );
    out
}

/// `GET /logs` — the same tail, as a page.
pub async fn logs_page(
    uri: axum::http::Uri,
) -> (axum::http::StatusCode, axum::response::Html<String>) {
    let raw = uri.query().unwrap_or("");
    let asked = std::sync::Arc::new(asked(raw));
    // THE SECOND HALF IS RESOLVED FROM THE STORE, exactly as `logs_json` does
    // it. Bound to a local rather than written inline so the `Option<PathBuf>`
    // it borrows from plainly outlives the call -- and named `cli_dir` rather
    // than `cli`, which is a crate this one depends on.
    // EVERY RUN DIRECTORY, resolved from the store exactly as `logs_json`
    // does it, and the server's own when its sink resolved one. A server
    // started without telemetry still reads the store's directories: its own
    // sink being absent is not "nothing to read" (MEASURED, 2026-09-03:
    // `<store>/logs/cli/events.ndjson` held 2,090 records from a running
    // sweep while this page said there was nothing to read).
    let halves = halves_of(log_dir(), crate::server::store_dir().ok().as_deref());
    if halves.is_empty() {
        return (
            axum::http::StatusCode::OK,
            axum::response::Html(page_shell(
                &asked,
                "<p class=\"halt\"><b>No log</b>Logging is not installed in this \
                 process and no store log directory could be resolved, so there is \
                 nothing to read. The server names the reason on stdout at startup.</p>",
            )),
        );
    }
    let health = sink_health();
    let inside = std::sync::Arc::clone(&asked);
    // OFF THE ASYNC WORKERS, AND ADMITTED, for the reason `logs_json` gives:
    // the page runs the same two-half walk. A full pool is a 429 page that
    // says so in words, with the form still on it. log-3, P1-04-02, D-2327.
    match crate::detail::run_log_read(move || page_across(&halves, &inside, health.as_ref())).await
    {
        Ok(page) => (axum::http::StatusCode::OK, axum::response::Html(page)),
        Err(why) => {
            let (status, _json) = crate::detail::admission_refused(
                "log read",
                crate::detail::MAX_LOG_READ_CONCURRENT,
                &why,
            );
            let halt = format!(
                "<p class=\"halt\"><b>Not admitted</b>This log read was refused \
                 before it ran ({}): at most {} log reads run at once, off the \
                 async workers. Nothing below was read; reload to retry.</p>",
                render::escape(&format!("{why:?}")),
                crate::detail::MAX_LOG_READ_CONCURRENT,
            );
            (status, axum::response::Html(page_shell(&asked, &halt)))
        }
    }
}

/// The page body, over the directories a caller names — split for the reason
/// [`json_over`] is.
///
/// # It read ONE directory, and every `cli` event was invisible on the page
/// that exists to show them
///
/// [`both_halves`] was written for exactly this and was wired only to
/// [`json_over`]. So `/logs.json` merged the server's directory with
/// `<store>/logs/cli` and `/logs` — the surface an operator actually opens —
/// walked the server's alone. MEASURED on 2026-09-01: the page showed nothing
/// newer than 31 August while **66 sweep events sat unread** in
/// `<store>/logs/cli/events.ndjson`, and every `cli` run printed *"The /logs
/// page walks BOTH halves … so these events appear there"* the whole time.
///
/// **The test that guarded it asserted on the wrong function.**
/// `the_page_shows_the_cli_half_and_not_only_the_servers` is named for the page
/// and drove `json_over`, which never had the defect. That is how this survived
/// being written down twice; the test drives [`page_over`] now.
///
/// **A KEYED PAGE READS THE INDEX, AS `/logs.json` DOES** (WP-33c, DB-A45):
/// `/logs?feed=gdfl&day=2022-03-07` was byte-identical to `/logs`, because
/// this walked the ring with the keys dropped. It now answers through
/// [`keyed_walk`], the same rows `/logs.json` gives for the same query, with
/// a link to the next page when there is one.
#[cfg(test)]
fn page_over(
    dir: &std::path::Path,
    cli_dir: Option<&std::path::Path>,
    asked: &Asked,
    health: Option<&telemetry::Health>,
) -> String {
    page_across(&two_halves(dir, cli_dir), asked, health)
}

/// [`page_over`] across every directory in `halves`, each named with its
/// state under the lead.
fn page_across(halves: &[Half], asked: &Asked, health: Option<&telemetry::Health>) -> String {
    let Some(pair) = named_pair(halves) else {
        return page_shell(
            asked,
            "<p class=\"halt\"><b>No log</b>No log directory resolved, so there is nothing to \
             read.</p>",
        );
    };
    let now = telemetry::now_millis();
    if asked.keys.is_empty() {
        let tail = all_halves(halves, &asked.query);
        let marks = contradicted(&tail.records, store_of(halves).as_deref());
        let mut page = page_of_marked(&tail, asked, health, pair, &marks);
        page.push_str(&dirs_note(halves));
        page.push_str(&reach_note(halves, now));
        page.push_str(&picture_note(served_picture().as_ref()));
        let (audit, state) = served_streams();
        page.push_str(&streams_note(audit.as_ref(), state.as_ref()));
        return page;
    }
    let (tail, _keyed, next, said) = keyed_walk_said(halves, asked);
    let marks = contradicted(&tail.records, store_of(halves).as_deref());
    let mut page = page_of_marked(&tail, asked, health, pair, &marks);
    page.push_str(&dirs_note(halves));
    page.push_str(&reach_note(halves, now));
    page.push_str(&picture_note(served_picture().as_ref()));
    let (audit, state) = served_streams();
    page.push_str(&streams_note(audit.as_ref(), state.as_ref()));
    match next {
        Some(cursor) => {
            let _ = write!(
                page,
                "<p class=\"lead\">Read from the durable audit index, newest first: {}. \
                 <a href=\"/logs{}&amp;before={}\">Older matches</a></p>",
                render::escape(&said),
                json_query(asked),
                render::query_value(&cursor),
            );
        }
        None => {
            let _ = write!(
                page,
                "<p class=\"lead\">Read from the durable audit index: {}.</p>",
                render::escape(&said),
            );
        }
    }
    page
}

/// [`page_over`] over a walk a caller already has — split for [`json_of`]'s
/// reason, and the two must stay in step: a flag reported by one surface and
/// not the other is a reader who gets a different answer depending on which URL
/// they happened to open.
///
/// `read` is the pair of directories the walk covered, in [`json_of`]'s order
/// and for its reason: a page that names one of two directories invites the
/// reader to conclude the other was empty.
#[cfg(test)]
fn page_of(
    tail: &telemetry::Tail,
    asked: &Asked,
    health: Option<&telemetry::Health>,
    read: (&std::path::Path, Option<&std::path::Path>),
) -> String {
    page_of_marked(tail, asked, health, read, &[])
}

/// [`page_of`], each row the import journal contradicts marked beside it
/// (DB-A48), exactly as [`json_of_marked`] marks the JSON.
fn page_of_marked(
    tail: &telemetry::Tail,
    asked: &Asked,
    health: Option<&telemetry::Health>,
    read: (&std::path::Path, Option<&std::path::Path>),
    marks: &[Option<Contradicted>],
) -> String {
    let mut body = String::new();
    let _ = write!(
        body,
        "<p class=\"lead\">{} event(s) · {} byte(s) read from {} file(s) · \
         <code>{}</code>",
        tail.records.len(),
        tail.bytes_read,
        tail.files_read,
        render::escape(&read.0.display().to_string()),
    );
    // BOTH DIRECTORIES ARE NAMED, for the reason `json_of` serves both: the
    // walk covers two and a page that names one lets a reader conclude the
    // other was empty — which is the reading that made this defect invisible
    // for as long as it lasted.
    match read.1 {
        Some(cli) => {
            let _ = write!(
                body,
                " and <code>{}</code></p>",
                render::escape(&cli.display().to_string()),
            );
        }
        // NOT SILENCE. No store root resolved means the `cli` half was never
        // opened, and a page that said nothing would be reporting "no terminal
        // events" in the same words it uses for "there were none" — the
        // fallback that hides a failure CLAUDE.md §4 bans.
        None => body.push_str(
            " · <b>the cli half was NOT read</b> — no store root resolved, so \
             nothing a terminal-run sweep wrote is below.</p>",
        ),
    }

    body.push_str(&health_banner(health));

    let notes = walk_notes(tail, health);
    if !notes.is_empty() {
        body.push_str("<ul class=\"notes\">");
        for note in &notes {
            let _ = write!(body, "<li>{}</li>", render::escape(note));
        }
        body.push_str("</ul>");
    }

    if tail.records.is_empty() {
        // NAME THE FILTER THAT ACTUALLY EMPTIED THE PAGE.
        //
        // This used to point at `BRUTEX_LOG_LEVEL` unconditionally. Under
        // `?target=pull.http` matching nothing, that sends the reader to change
        // the WRITE floor and re-run a job, when the cause is a read filter they
        // can clear in one click. Worse, under `?level=warn` the suggested
        // remedy provably cannot change the answer: a `debug` line written to
        // the file is still excluded by the reader's own floor. `CLAUDE.md` §4
        // asks for the reason to be named, not for a remedy that cannot work.
        let filtered = asked.level.is_some() || !asked.target.is_empty() || asked.run != 0;
        if filtered {
            let mut said =
                String::from("<p class=\"lead\">Nothing matched <b>the filters in force</b>:");
            if let Some(level) = asked.level {
                let _ = write!(said, " level at or above <code>{}</code>;", level.label());
            }
            if !asked.target.is_empty() {
                let _ = write!(
                    said,
                    " target <code>{}</code>;",
                    render::escape(&asked.target)
                );
            }
            if asked.run != 0 {
                let _ = write!(said, " run <code>{}</code>;", asked.run);
            }
            said.push_str(" widening or clearing them is the first thing to try.");
            // ONLY WHEN IT COULD HELP. With a read floor above `debug`, raising
            // the WRITE floor changes nothing the reader would then admit.
            if asked
                .level
                .is_none_or(|l| telemetry::Level::Debug.at_least(l))
            {
                said.push_str(
                    " Members are also written at <code>debug</code>, below the \
                     default <code>info</code> write floor — set \
                     <code>BRUTEX_LOG_LEVEL=debug</code> and re-run to record them.",
                );
            }
            said.push_str("</p>");
            body.push_str(&said);
        } else {
            body.push_str(
                "<p class=\"lead\">Nothing matched. A quiet log is the ordinary \
                 state: members are written at <code>debug</code>, and the default \
                 floor is <code>info</code> — set <code>BRUTEX_LOG_LEVEL=debug</code> \
                 and re-run to see them.</p>",
            );
        }
        return page_shell(asked, &body);
    }

    body.push_str(&rows_table(tail, marks));
    page_shell(asked, &body)
}

/// Every flag the walk raised, in the operator's words.
///
/// **These go above the rows, not below them.** Someone who scrolls a table and
/// finds nothing alarming has been told something; they have only been told the
/// truth if they also know the walk stopped early. `hit_scan_cap` is the
/// difference between "no errors in the log" and "no errors in the four
/// megabytes I happened to read".
fn walk_notes(tail: &telemetry::Tail, health: Option<&telemetry::Health>) -> Vec<String> {
    let mut notes: Vec<String> = Vec::new();
    // FIRST, BECAUSE IT IS THE FIRST QUESTION. Every other note here reports a
    // limit of the READER — how far it walked, what it could not decode. This
    // one reports a loss by the WRITER, and it decides whether anything below
    // can be trusted. A log handed to somebody who did not run the job is
    // evidence, and evidence with silent holes is worse than none.
    if let Some(missing) = tail.missing.filter(|n| *n > 0) {
        // WHERE TO LOOK DEPENDS ON WHETHER THE BANNER CAN ANSWER.
        //
        // `tail.missing` is DURABLE: it is computed from the sequence hole in
        // the FILE, so it survives a restart. `Health::dropped` is PER-PROCESS —
        // `Sink::open` resumes `seq` from the file but starts `dropped` at zero.
        // After the restart that follows a failed run, which is the routine
        // action, the two disagree, and this note used to send the reader to a
        // banner reading "0 dropped" for a count it did not hold. The page
        // contradicted itself and pointed at the half that was wrong.
        let whither = if health.is_some_and(telemetry::Health::is_loud) {
            "see the sink banner above for the count and the reason."
        } else {
            "the banner above counts only THIS process's losses and counts none, \
             so these were lost by an earlier run and no counter on this page \
             holds the reason."
        };
        notes.push(format!(
            "{missing} event(s) are MISSING from this range. The sink numbered \
             them and the file does not hold them, so they were dropped rather \
             than filtered — {whither} Nothing below is a complete picture of \
             what happened."
        ));
    }
    if tail.hit_scan_cap {
        notes.push(format!(
            "The walk stopped after {SCAN_BYTES} byte(s). This is what was in \
             the newest part of the log, NOT everything there is."
        ));
    }
    if !tail.reached_oldest {
        notes.push(
            "Older events exist beyond what was read. Narrow by target or \
             level to reach them."
                .to_owned(),
        );
    }
    if tail.partial_tail {
        notes.push(
            "The newest file ended mid-line — a writer caught in the act, or a \
             torn write. The fragment was skipped, never guessed at."
                .to_owned(),
        );
    }
    if tail.malformed > 0 {
        notes.push(format!(
            "{} line(s) would not decode and were stepped over.",
            tail.malformed
        ));
    }
    for why in &tail.errors {
        notes.push(format!("A file could not be read: {why}"));
    }
    notes
}

/// This process's own sink, if it has one.
///
/// `telemetry::global()` is an `OnceLock` read and `Sink::health` is a handful
/// of relaxed atomic loads plus one uncontended mutex — no `metadata` call, no
/// directory scan, no walk of the file. O(1), and it stays O(1) as the log
/// grows, which is the only reason it is safe to do on every page render.
///
/// **Proved by `telemetry::sink::the_roll_decision_reads_the_running_count_and_never_the_files_size`**
/// (invariant T-01), which grows the log behind the sink's back and then asserts
/// `health().current_bytes < on_disk` — "the count is the sink's own, not the
/// file's". A `metadata` call anywhere in `health` would fail it. That is the
/// load-bearing half of the claim: the cost is independent of the file, which
/// is the size that grows all day.
fn sink_health() -> Option<telemetry::Health> {
    telemetry::global().map(telemetry::Sink::health)
}

/// **What the WRITER lost, which the reader cannot see.**
///
/// `telemetry::tail` reads what reached the file. It is structurally incapable
/// of reporting what never got there — a dropped event leaves no line to count.
/// So a page built only from the tail shows `0 events` for two opposite worlds:
/// nothing happened, and everything was thrown away. `Health::dropped` is the
/// only trace the second one leaves, and its own doc comment says "this is the
/// number a page must show". Until now no page showed it, and `is_loud` had no
/// caller at all.
///
/// `CLAUDE.md` §4 bans "a fallback that hides a failure — degrade loudly and
/// name the reason, or refuse. Never both silently." A sink that drops events
/// and renders a clean page is exactly that fallback.
///
/// The quiet case still prints a line. "Nothing was dropped" is a claim worth
/// making explicitly: an absent banner is indistinguishable from a banner this
/// page forgot to render, and the operator cannot tell those apart.
fn health_banner(health: Option<&telemetry::Health>) -> String {
    let Some(h) = health else {
        return "<p class=\"halt\"><b>No sink</b>This process is not writing a \
                log, so the events below — if any — are from an older run. The \
                server names the reason on stdout at startup.</p>"
            .to_owned();
    };
    if !banner_is_loud(h) {
        return format!(
            "<p class=\"lead\">Sink healthy · {} written · 0 dropped · {} \
             rotation(s) · {} byte(s) in the current file.</p>",
            h.written, h.rotations, h.current_bytes,
        );
    }
    let mut out = String::from(if h.is_loud() {
        "<p class=\"halt\"><b>The log is incomplete</b>"
    } else if h.clock_held > 0 {
        "<p class=\"halt\"><b>The log's clock is held</b>"
    } else {
        "<p class=\"halt\"><b>The sink reported a failure</b>"
    });
    if h.clock_held > 0 {
        let _ = write!(
            out,
            "{} event(s) read a clock BEHIND the last one written and were \
             stamped at that later instant instead, so their times on this page \
             are not when they happened, and a stale-run check reading them \
             may see a dead run as live. ",
            h.clock_held,
        );
    }
    if h.dropped > 0 {
        let _ = write!(
            out,
            "{} event(s) happened and were never written. They are gone — this \
             page cannot show them, and no filter will bring them back. ",
            h.dropped,
        );
    }
    if h.rotation_failures > 0 {
        let _ = write!(
            out,
            // WHAT THE SINK ACTUALLY DOES AFTER A FAILED ROLL. It stops
            // rotating for the life of the process (`Sink::rotation_broken`),
            // so nothing is overwritten after the failure: the file GROWS. The
            // banner said the opposite and never said a restart resumes
            // rotation (CE-41, D-1769).
            "{} roll(s) failed, so rotation has stopped for the life of this \
             process: the current file is growing past its bound and no later \
             event overwrites an older one. The failed roll itself may have \
             removed the oldest retained file. Fix the cause named below and \
             restart the server to resume rotation. ",
            h.rotation_failures,
        );
    }
    if let Some(why) = h.last_error.as_deref() {
        let _ = write!(out, "Most recent failure: {}.", render::escape(why));
    }
    out.push_str("</p>");
    out
}

/// The events themselves, newest first.
fn rows_table(tail: &telemetry::Tail, marks: &[Option<Contradicted>]) -> String {
    let mut out = String::from(
        "<div class=\"hscroll\"><table><thead><tr>\
         <th>When (IST)</th><th>Level</th><th>Target</th><th>Message</th>\
         <th>Fields</th></tr></thead><tbody>",
    );
    for (n, record) in tail.records.iter().enumerate() {
        let mark = marks.get(n).and_then(Option::as_ref);
        // A CONTRADICTED LINE IS SHOWN AS A FAULT, whatever its level: it
        // says a day was imported that was not (DB-A48).
        let loud = record.level.at_least(telemetry::Level::Warn) || mark.is_some();
        let mut fields = String::new();
        for (n, (key, value)) in record.fields.iter().enumerate() {
            if n > 0 {
                fields.push(' ');
            }
            let _ = write!(
                fields,
                "<b>{}</b>={}",
                render::escape(key),
                render::escape(&value_text(value))
            );
        }
        // BOTH TRUNCATION FLAGS ARE SHOWN. A field cut at its ceiling and a
        // field counted rather than kept are the two ways this log can be
        // shorter than the truth, and a reader who cannot see either will read
        // the shortened version as the whole one.
        if record.cut {
            fields.push_str(" <i>(a value was cut at its ceiling)</i>");
        }
        if record.dropped_fields > 0 {
            let _ = write!(
                fields,
                " <i>({} field(s) counted, not kept)</i>",
                record.dropped_fields
            );
        }
        if let Some(mark) = mark {
            let _ = write!(
                fields,
                " <b>CONFLICT</b> <i>({})</i>",
                render::escape(&mark.words())
            );
        }
        let _ = write!(
            out,
            "<tr{}><td class=\"when\">{}</td><td class=\"lvl\">{}</td>\
             <td class=\"mono\">{}</td><td>{}</td><td class=\"wide\">{fields}</td></tr>",
            if loud { " class=\"fault\"" } else { "" },
            // IST, THROUGH THE SAME FUNCTION EVERY OTHER PAGE USES.
            //
            // This column rendered `record.at_utc` — the line's own UTC text —
            // while `/audit` rendered the same instant through `ist_stamp`. An
            // operator reading a failed run on one page and its log on the
            // other was reading two clocks 5h30m apart, and nothing on either
            // page said so. That is the "two answers to one question" shape
            // this repository treats as a defect rather than a nicety.
            //
            // The STORED value is untouched and stays UTC — `at_unix_millis`
            // on the record, `ts` in the file, and `/logs.json` still answers
            // in epoch milliseconds. One representation on disk, one clock on
            // every human surface, and the conversion happens once at the
            // boundary. See `crate::server::ist_stamp`.
            render::escape(&crate::server::ist_stamp(
                record.at_unix_millis.div_euclid(1_000),
            )),
            render::escape(record.level.label()),
            render::escape(&record.target),
            render::escape(&record.message),
        );
    }
    out.push_str("</tbody></table></div>");
    out
}

/// The filter the page is showing, as a query string `/logs.json` will honour.
///
/// **Two defects lived in the one expression this replaces.**
///
/// It read `format!("?target={}", render::escape(&asked.target))`.
///
/// 1. It dropped the level and the limit. An operator who filtered the page to
///    `level=error&limit=200` and then followed "as JSON" got the unfiltered
///    default 50 — a different answer to the same question, from the link whose
///    entire purpose is to be the same answer in another format.
/// 2. It HTML-escaped where a URL needs percent-encoding. `render::escape`
///    rewrites five characters into entities; a target containing `&` became
///    `&amp;`, which a browser sends as a literal `&` and the server then reads
///    as a PARAMETER SEPARATOR. `render::query_value` is the encoder for this
///    position, and its own doc says the result must not then be passed through
///    `escape` — so the value is percent-encoded here and the surrounding
///    attribute is safe as it stands.
///
/// The level is written by label rather than by rank because that is what
/// `Level::of_label` reads back, and the limit is the CLAMPED number rather than
/// the one asked for, so the link reproduces the page rather than the request.
fn json_query(asked: &Asked) -> String {
    let mut q = format!("?limit={}", asked.limit);
    if let Some(level) = asked.level {
        let _ = write!(q, "&level={}", level.label());
    }
    if !asked.target.is_empty() {
        let _ = write!(q, "&target={}", render::query_value(&asked.target));
    }
    if asked.run != 0 {
        let _ = write!(q, "&run={}", asked.run);
    }
    // THE KEYS RIDE ALONG (WP-33c, DB-A45), so the JSON link and the next
    // page reproduce the page's own question.
    let keys = &asked.keys;
    for (name, value) in [
        ("feed", keys.feed.clone()),
        (
            "instrument",
            keys.instrument.as_ref().map(|i| i.asked.clone()),
        ),
        ("day", keys.day.map(|d| d.to_string())),
        ("contract", keys.contract.clone()),
        ("code", keys.code.clone()),
        ("month", keys.month.clone()),
        ("reason", keys.reason.clone()),
        ("kind", keys.kind.clone()),
        (
            "from",
            keys.span
                .map(|(from, _)| from)
                .filter(|from| *from != i64::MIN)
                .map(|from| from.to_string()),
        ),
        (
            "to",
            keys.span
                .map(|(_, to)| to)
                .filter(|to| *to != i64::MAX)
                .map(|to| to.to_string()),
        ),
    ] {
        if let Some(value) = value {
            let _ = write!(q, "&{name}={}", render::query_value(&value));
        }
    }
    if keys.run.is_some() || keys.target.is_some() {
        q.push_str("&durable=1");
    }
    if asked.ring {
        q.push_str("&ring=1");
    }
    q
}

/// The page around the table: the filter form, and nothing that needs script.
fn page_shell(asked: &Asked, body: &str) -> String {
    let mut levels = String::new();
    for level in telemetry::LEVELS {
        let on = asked.level == Some(level);
        let _ = write!(
            levels,
            "<option value=\"{}\"{}>{}</option>",
            level.label(),
            if on { " selected" } else { "" },
            level.label()
        );
    }
    format!(
        "<!doctype html><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>brutex · logs</title>\
         <style>\
         body{{background:#f5f7fb;color:#0a0f1e;font:15px/1.55 ui-sans-serif,-apple-system,system-ui,sans-serif;padding:0 0 4rem;margin:0}}\
         @media(prefers-color-scheme:dark){{body{{background:#060911;color:#e9efff}}\
         table,form{{background:#0e1524}}thead th{{background:#0e1524}}}}\
         h1{{max-width:1180px;margin:0 auto;padding:2rem 1.25rem .3rem;font-size:1.6rem;letter-spacing:-.5px}}\
         .lead{{max-width:1180px;margin:0 auto;padding:0 1.25rem;color:#5a6478;font-size:.9rem}}\
         form{{max-width:1180px;margin:1rem auto;padding:.9rem 1.25rem;display:flex;gap:.7rem;\
         align-items:end;flex-wrap:wrap;border:1px solid #e4e9f3;border-radius:12px}}\
         label{{display:flex;flex-direction:column;gap:.3rem;font-size:.7rem;letter-spacing:.09em;text-transform:uppercase;color:#5a6478}}\
         input,select{{font:inherit;padding:.45rem .6rem;border:1px solid #e4e9f3;border-radius:8px;background:transparent;color:inherit}}\
         button{{font:inherit;font-weight:700;padding:.5rem 1.1rem;border:0;border-radius:8px;background:#4f46e5;color:#fff;cursor:pointer}}\
         .hscroll{{overflow-x:auto;max-width:1180px;margin:0 auto;padding:0 1.25rem}}\
         table{{border-collapse:collapse;width:100%;font-size:13px;min-width:56rem}}\
         thead th{{position:sticky;top:0;text-align:left;padding:.6rem .7rem;border-bottom:1px solid #e4e9f3;\
         font-size:.65rem;letter-spacing:.09em;text-transform:uppercase;color:#5a6478;white-space:nowrap}}\
         td{{padding:.5rem .7rem;border-bottom:1px solid #e8eef6;vertical-align:top}}\
         td.when,td.mono,.lvl{{font-family:ui-monospace,Menlo,monospace;font-size:12px;white-space:nowrap}}\
         td.wide{{font-family:ui-monospace,Menlo,monospace;font-size:11.5px;color:#5a6478}}\
         tr.fault td{{background:rgba(220,38,38,.08);color:#dc2626}}\
         .notes{{max-width:1180px;margin:.8rem auto;padding:.7rem 1.25rem .7rem 2.6rem;color:#8a5a00;font-size:.85rem}}\
         .halt{{max-width:1180px;margin:1rem auto;padding:1rem 1.25rem;color:#dc2626}}\
         .halt b{{display:block;text-transform:uppercase;font-size:.7rem;letter-spacing:.1em}}\
         a{{color:#4f46e5}}\
         </style>\
         <h1>logs</h1>\
         <p class=\"lead\">The rolling log this process is writing. \
         Newest first. No script, no polling — reload to refresh.</p>\
         <form method=\"get\" action=\"/logs\">\
         <label>Level<select name=\"level\"><option value=\"\">every level</option>{levels}</select></label>\
         <label>Target<input type=\"text\" name=\"target\" value=\"{target}\" placeholder=\"pull, pull.member, api.serve\"></label>\
         <label>Run<input type=\"text\" name=\"run\" value=\"{run}\" placeholder=\"every run\"></label>\
         <label>Feed<input type=\"text\" name=\"feed\" value=\"{feed}\" placeholder=\"gdfl, zerodha\"></label>\
         <label>Instrument<input type=\"text\" name=\"instrument\" value=\"{instrument}\" placeholder=\"NSE-INDEX-NIFTY\"></label>\
         <label>Day<input type=\"text\" name=\"day\" value=\"{day}\" placeholder=\"YYYY-MM-DD\"></label>\
         <label>Contract<input type=\"text\" name=\"contract\" value=\"{contract}\"></label>\
         <label>Code<input type=\"text\" name=\"code\" value=\"{code}\" placeholder=\"too_many_entries\"></label>\
         <label>Reason<input type=\"text\" name=\"reason\" value=\"{reason}\"></label>\
         <label>Month<input type=\"text\" name=\"month\" value=\"{month}\" placeholder=\"YYYY-MM\"></label>\
         <label>Import kind<input type=\"text\" name=\"kind\" value=\"{kind}\" placeholder=\"indices, stocks, options\"></label>\
         <label>From<input type=\"text\" name=\"from\" value=\"{from}\" placeholder=\"YYYY-MM-DD[THH:MM] IST\"></label>\
         <label>To<input type=\"text\" name=\"to\" value=\"{to}\" placeholder=\"YYYY-MM-DD[THH:MM] IST\"></label>\
         <label>Limit<input type=\"number\" name=\"limit\" min=\"1\" max=\"{max}\" value=\"{limit}\"></label>\
         <button type=\"submit\">Show</button>\
         <a href=\"/logs.json{q}\">as JSON</a>\
         </form>{body}",
        levels = levels,
        target = render::escape(&asked.target),
        run = if asked.run == 0 {
            String::new()
        } else {
            asked.run.to_string()
        },
        limit = asked.limit,
        max = PAGE_LIMIT,
        q = json_query(asked),
        body = body,
        feed = render::escape(asked.keys.feed.as_deref().unwrap_or("")),
        instrument = render::escape(
            asked
                .keys
                .instrument
                .as_ref()
                .map_or("", |instrument| instrument.asked.as_str())
        ),
        day = asked
            .keys
            .day
            .map_or_else(String::new, |day| day.to_string()),
        contract = render::escape(asked.keys.contract.as_deref().unwrap_or("")),
        code = render::escape(asked.keys.code.as_deref().unwrap_or("")),
        reason = render::escape(asked.keys.reason.as_deref().unwrap_or("")),
        month = render::escape(asked.keys.month.as_deref().unwrap_or("")),
        kind = render::escape(asked.keys.kind.as_deref().unwrap_or("")),
        from = span_end(&asked.keys, false),
        to = span_end(&asked.keys, true),
    )
}

/// One end of a request's span as its form field holds it: the epoch
/// milliseconds, or nothing for an end left open.
fn span_end(keys: &Keys, to: bool) -> String {
    keys.span
        .map(|(from, until)| if to { until } else { from })
        .filter(|at| *at != i64::MIN && *at != i64::MAX)
        .map_or_else(String::new, |at| at.to_string())
}

/// Every HTTP request, once, with what it did and how long it took.
///
/// # Why middleware and not a line in each handler
///
/// There are two dozen routes and a fallback. A handler that forgets the line
/// is a route that is silently invisible, and the one that forgets is always
/// the one added last — which is the one being debugged. A layer cannot be
/// forgotten by a route that does not know it exists.
///
/// # What is NOT logged, deliberately
///
/// **The query string.** `/pull/spot` carries operator input and this
/// repository is public (`CLAUDE.md` §8 is the same instinct about paths).
/// The PATH is logged and the query is not, so a log shared with anybody
/// cannot leak what was typed into a form. The path alone answers "which
/// route", which is the question this event exists for.
///
/// **No body, no headers.** Both are unbounded and one of them carries the
/// credential header on its way out.
///
/// # Cost
///
/// One event per request, at `Debug` — so a normal run writes none of them
/// and the 64 MiB window is not spent on `GET /_app/immutable/...`. A
/// 5xx is re-emitted at `Error` regardless of floor, because a request that
/// failed is not detail.
pub async fn note_request(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let from_another_site = cross_site(request.headers());
    let method = request.method().as_str().to_owned();
    let path = request.uri().path().to_owned();
    // A `/db` READ KEEPS ITS SELECTION (WP-33c, DB-A42): the names it read
    // by, never the whole query, and only on a read route.
    let selection = is_db_read(&path).then(|| selection_of(request.uri().query().unwrap_or("")));
    // EVERY ROUTE IS TIMED INTO ITS BUCKETS (R3-14, RA-014): its matched
    // path, or the front end's one row. A request whose client leaves before
    // the answer is dropped here with its `Pending`, which counts it
    // cancelled.
    let pending = crate::speed::Pending::start(
        crate::speed::global().row(
            request
                .extensions()
                .get::<axum::extract::MatchedPath>()
                .map_or(crate::speed::FRONT_END, axum::extract::MatchedPath::as_str),
            crate::speed::Kind::Route,
            crate::speed::now_ms(),
        ),
    );
    let started = std::time::Instant::now();
    let response = next.run(request).await;
    let status = response.status();
    let micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    pending.finish(
        micros,
        crate::speed::Outcome::of(
            status.as_u16(),
            response
                .headers()
                .contains_key(axum::http::header::RETRY_AFTER),
        ),
    );
    let now = u64::try_from(telemetry::now_millis()).unwrap_or(0);

    let level = request_level(status);
    // THE FAILED-REQUEST LINES ARE RATIONED ON BOTH PATHS. See
    // [`FAILED_LINES_PER_WINDOW`] and [`LOCAL_FAILED_LINES_PER_WINDOW`]. A
    // refused `/db` read's access record is under the same ration.
    let mut admitted = true;
    if level != telemetry::Level::Debug {
        let admit = FAILED_LINES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .admit(from_another_site, now);
        if let Some(suppressed) = admit.summary {
            let _dropped_when_filtered = telemetry::emit(
                &telemetry::Event::new(
                    telemetry::Level::Warn,
                    "api.request",
                    suppression_message(from_another_site),
                )
                .with("suppressed", telemetry::Value::Uint(suppressed))
                .with(
                    "origin",
                    telemetry::Value::Str(if from_another_site {
                        "cross-site"
                    } else {
                        "same-origin"
                    }),
                )
                .with("window_ms", telemetry::Value::Uint(FAILED_LINE_WINDOW_MS)),
            );
        }
        admitted = admit.write;
    }
    if let Some(ref selection) = selection
        && !status.is_server_error()
    {
        note_access(&path, (status.as_u16(), micros), selection, admitted, now);
        // A REFUSED READ'S LINE IS ITS ACCESS RECORD, which the audit stream
        // keeps; a served one also leaves the `debug` request line below.
        if status.is_client_error() {
            return response;
        }
    }
    if !admitted {
        return response;
    }
    // A SERVER ERROR'S DURABLE COPY HAS ITS OWN RATION (WP-33c, DB-A52): the
    // first [`AUDITED_ERRORS_PER_WINDOW`] a window are written at `error`,
    // which the audit stream keeps; past it the line is still written, at
    // `warn`, to the ring alone, saying so, and the next window's first says
    // how many were kept out. A flood of 500s can no longer roll the stream.
    let mut kept_out = None;
    let mut durable = None;
    let level = if level == telemetry::Level::Error {
        let admit = AUDITED_ERRORS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .admit(now);
        kept_out = admit.summary;
        durable = Some(admit.write);
        if admit.write {
            telemetry::Level::Error
        } else {
            telemetry::Level::Warn
        }
    } else {
        level
    };
    let mut event = telemetry::Event::new(level, "api.request", "served")
        .with("method", telemetry::Value::Str(&method))
        .with("path", telemetry::Value::Str(&path))
        .with("status", telemetry::Value::Uint(u64::from(status.as_u16())))
        .with("micros", telemetry::Value::Uint(micros));
    if let Some(durable) = durable {
        event = event.with("durable", telemetry::Value::Bool(durable));
    }
    if let Some(kept_out) = kept_out {
        event = event.with("kept_out_of_audit_before", telemetry::Value::Uint(kept_out));
    }
    let _dropped_when_filtered = telemetry::emit(&event);
    response
}

/// The level a request's line is written at: `error` for a server error,
/// `warn` for a client error, `debug` otherwise.
///
/// 501 IS A PLACEHOLDER, NOT A FAILURE (WP-33c, DB-A52): a route this build
/// registers and has not built answers it by design, and one bench put 201 of
/// them into the durable audit stream at `error`. It is a `warn` in the ring,
/// rationed as every failed request is, and the audit stream never takes it.
fn request_level(status: axum::http::StatusCode) -> telemetry::Level {
    if status.is_server_error() && status != axum::http::StatusCode::NOT_IMPLEMENTED {
        telemetry::Level::Error
    } else if status.is_client_error() || status.is_server_error() {
        telemetry::Level::Warn
    } else {
        telemetry::Level::Debug
    }
}

/// How many server-error request lines one window writes at `error`, where
/// the durable audit stream keeps them (WP-33c, DB-A52). Past it a window's
/// lines go to the ring at `warn`, each marked `durable: false`, and the next
/// window's first line carries the count kept out.
pub const AUDITED_ERRORS_PER_WINDOW: u64 = 20;

/// The durable copy's ration, one window for every origin.
static AUDITED_ERRORS: std::sync::Mutex<Ration> =
    std::sync::Mutex::new(Ration::new(AUDITED_ERRORS_PER_WINDOW));

// ---------------------------------------------------------------------------
// /db READS, COUNTED AND SAMPLED (WP-33c, DB-A42)
// ---------------------------------------------------------------------------
//
// # What was missing
//
// A served read was a `debug` line, dropped at the `info` floor, and every
// `/db` data route is exempt from the invocation audit (D-0568, which this
// does not change: that journal records sweep results and controls). A
// refused read was a `warn` in the ring only. So "what did the operator read,
// and what was refused" had no durable answer at all.
//
// # The rule (stated, because it bounds the durable stream)
//
// * EVERY read of a [`DB_READS`] route is COUNTED, per route, per
//   [`ACCESS_WINDOW_MS`] window, served and refused apart, with its slowest
//   time.
// * The FIRST SERVED read of a route in a window is written whole, at `info`:
//   `api.access read served`, with its route, status, time and the selection
//   it named ([`ACCESS_KEYS`]: feed, instrument, symbol, day, month and the
//   rest of a `/db` selection, never the whole query).
// * EVERY REFUSED read is written, at `warn` (`api.access read refused`),
//   under the failed-request ration every 4xx already had: 200 a minute from
//   this console, 50 from another site, the rest counted and said.
// * When a route's window closes, its next read first writes
//   `api.access reads counted`: how many were served and refused in the
//   window that closed, and the slowest.
//
// A target ending in `access` is audited (`telemetry::is_audited`), so all
// three reach the durable stream and its index (feed, instrument, day, month):
// `/logs.json?instrument=NIFTY` finds the operator's own NIFTY reads. Bound:
// at most two lines per route a minute plus the rationed refusals, about
// 1.6 MB a day for every route read continuously; a route nobody reads writes
// nothing. A window's count rides on the route's next read, so the last
// window before silence is counted in memory and said when the route is next
// read: an honest limit, named.

/// The routes a `/db` page reads its data from: each read is counted and
/// sampled into the durable stream (WP-33c, DB-A42).
pub const DB_READS: [&str; 16] = [
    "/bars.json",
    "/bars/window.json",
    "/spot.json",
    "/greeks.json",
    "/chain.json",
    "/calendar.json",
    "/gaps.json",
    "/instruments.json",
    "/store.json",
    "/dayflags.json",
    "/imports.json",
    "/provenance.json",
    "/sources.json",
    "/sources/bars.json",
    "/sources/day.json",
    "/census/facets.json",
];

/// The query names an access record keeps: the selection, never anything
/// else a query may carry. At most [`telemetry::MAX_FIELDS`] less the four an
/// access record always carries are kept, in this order: the names `/db`
/// sends (`DbBarsGrid`'s window: feed, exchange, segment, symbol, contract,
/// timeframe, from, to) first, so `symbol` is kept and the audit index files
/// the read under its instrument.
pub const ACCESS_KEYS: [&str; 12] = [
    "feed",
    "symbol",
    "instrument",
    "segment",
    "exchange",
    "contract",
    "timeframe",
    "month",
    "day",
    "from",
    "to",
    "key",
];

/// An access record's window.
pub const ACCESS_WINDOW_MS: u64 = 60_000;

/// Whether `path` is a `/db` read route.
fn is_db_read(path: &str) -> bool {
    DB_READS.contains(&path)
}

/// The selection a read named, from its query: each of [`ACCESS_KEYS`] it
/// carries, decoded, in that order.
fn selection_of(query: &str) -> Vec<(&'static str, String)> {
    ACCESS_KEYS
        .iter()
        .filter_map(|name| {
            let value = crate::server::param(query, name);
            (!value.is_empty()).then_some((*name, value))
        })
        .take(telemetry::MAX_FIELDS.saturating_sub(4))
        .collect()
}

// THE PER-ROUTE MINUTE COUNTER LIVES IN `crate::speed` (R3-14): one module
// keeps every per-route count, and this one decides what a read writes.
use crate::speed::AccessWindows;

/// The access windows of this process.
static ACCESS: std::sync::Mutex<Option<AccessWindows>> = std::sync::Mutex::new(None);

/// Counts one `/db` read and writes what [`AccessWindows::account`] says to:
/// the closed window's count, this read whole (a sampled served read, or a
/// refused one the failed-request ration admits), or nothing.
pub(crate) fn note_access(
    path: &str,
    (status, micros): (u16, u64),
    selection: &[(&'static str, String)],
    refusal_admitted: bool,
    now_ms: u64,
) {
    let Some(route) = DB_READS.iter().copied().find(|route| *route == path) else {
        return;
    };
    let refused = status >= 400;
    let plan = ACCESS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get_or_insert_with(AccessWindows::default)
        .account(route, refused, micros, now_ms);
    let mut lines: Vec<telemetry::Event<'_>> = Vec::with_capacity(2);
    if let Some(closed) = plan.closed {
        lines.push(
            telemetry::Event::info("api.access", "reads counted")
                .with("route", telemetry::Value::Str(route))
                .with("served", telemetry::Value::Uint(closed.served))
                .with("refused", telemetry::Value::Uint(closed.refused))
                .with("slowest_micros", telemetry::Value::Uint(closed.slowest))
                .with("window_ms", telemetry::Value::Uint(ACCESS_WINDOW_MS)),
        );
    }
    // A REFUSAL IS WRITTEN WHEN THE RATION EVERY FAILED REQUEST HAS ADMITS IT
    // (`note_request` asked it, and said any count it held back).
    let write = if refused {
        refusal_admitted
    } else {
        plan.sample
    };
    if write {
        let mut line = if refused {
            telemetry::Event::warn("api.access", "read refused")
        } else {
            telemetry::Event::info("api.access", "read served")
        }
        .with("route", telemetry::Value::Str(route))
        .with("status", telemetry::Value::Uint(u64::from(status)))
        .with("micros", telemetry::Value::Uint(micros));
        for (name, value) in selection {
            line = line.with(name, telemetry::Value::Str(value));
        }
        lines.push(line);
    }
    for line in &lines {
        let _dropped_when_filtered = telemetry::emit(line);
    }
}

/// How many `api.request` lines at `Warn` or `Error` one window may write.
///
/// audit-20261003 hunt-api-3, D-1583. Every answer ≥ 400 was written, and a
/// cross-site GET with a local `Host` is admitted to every non-journaled route
/// by design — so any page the operator had open could loop `fetch(…,
/// {mode:'no-cors'})` and roll the whole 64 MiB window away (100,000 requests
/// in 29 s evicted a sentinel). Cross-site failures ([`cross_site`]) are now
/// rationed per window, and what
/// was held back is COUNTED and said in one line by the first failed request
/// of a later window, so a flood is still visible as a flood. Honest limit: a
/// count is said only once another cross-site request fails; a flood followed by silence
/// until shutdown leaves its last window's count unsaid.
///
/// Bound: at most this many lines plus one summary per window, ~620 bytes
/// each — about 1.8 MB an hour at the worst, against a 64 MiB retained log.
pub const FAILED_LINES_PER_WINDOW: u64 = 50;

/// How many failed-request lines the operator's own pages and local tools may
/// write in one window.
///
/// audit-20261003 hunt-api-3, D-1552. D-1583 rationed only requests another
/// site sent, so a same-origin client (a page of this console stuck in a
/// retry loop, a local script) still wrote one line per failed request and
/// could roll the whole 64 MiB window away exactly as the cross-site flood
/// did. This path now has its own ration, four times the cross-site one
/// because it is the operator's own traffic, with the same counted summary:
/// what is held back is said in one Warn line carrying `suppressed` = N, never
/// dropped silently. A separate count, so a local flood cannot spend the
/// cross-site lines and a cross-site flood cannot silence the operator's own
/// failures.
///
/// Bound: at most this many lines plus one summary per window, ~620 bytes
/// each, about 7.4 MB an hour at the worst; with the cross-site ration, about
/// 9.3 MB an hour, so a sustained flood of both still takes some seven hours
/// to roll the 64 MiB retained log (`docs/06-limits.md`).
pub const LOCAL_FAILED_LINES_PER_WINDOW: u64 = 200;

/// The summary line's message for one origin class.
pub(crate) const fn suppression_message(from_another_site: bool) -> &'static str {
    if from_another_site {
        "cross-site failed-request lines suppressed in the previous window"
    } else {
        "same-origin failed-request lines suppressed in the previous window"
    }
}

/// Whether a request came from another site, by the browser's own
/// `Sec-Fetch-Site`: anything but absent, `same-origin` or `none` (typed or
/// bookmarked). These failed requests draw on [`FAILED_LINES_PER_WINDOW`];
/// the operator's own pages and local tools draw on the separate
/// [`LOCAL_FAILED_LINES_PER_WINDOW`] (D-1552). Every browser that sends a
/// `no-cors` fetch sends this header with it.
pub(crate) fn cross_site(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get("sec-fetch-site")
        .is_some_and(|value| !matches!(value.as_bytes(), b"same-origin" | b"none"))
}

/// The length of one rationing window, in milliseconds.
pub const FAILED_LINE_WINDOW_MS: u64 = 60_000;

/// The process's failed-request rations, one per origin class.
static FAILED_LINES: std::sync::Mutex<Rations> = std::sync::Mutex::new(Rations::new());

/// Opens a fresh window on both rations, for a test that asserts on ONE
/// failed-request line.
///
/// The rations are the PROCESS's, and a test binary is one process: every test
/// that sends a request which fails draws on the same
/// [`LOCAL_FAILED_LINES_PER_WINDOW`] lines a minute. The api suite sent 267
/// such requests a run on 2026-10-11 (counted in its sink: 227 `Warn`, 40
/// `Error`), so a run quick enough to fit one window held back the line of
/// whichever test came late — correctly, and with no summary, because no
/// later window opened to carry one. A test about its own line opens its own
/// window first; a test about the ration itself builds its own [`Rations`].
#[cfg(test)]
pub(crate) fn fresh_failed_line_window() {
    *FAILED_LINES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Rations::new();
}

/// One ration per origin class: cross-site (D-1583) and same-origin (D-1552).
#[derive(Debug)]
pub(crate) struct Rations {
    /// Failed requests another site sent.
    cross: Ration,
    /// Failed requests from this console's own pages and local tools.
    local: Ration,
}

impl Rations {
    pub(crate) const fn new() -> Self {
        Self {
            cross: Ration::new(FAILED_LINES_PER_WINDOW),
            local: Ration::new(LOCAL_FAILED_LINES_PER_WINDOW),
        }
    }

    /// Accounts one failed request of its origin class at `now_ms`.
    pub(crate) fn admit(&mut self, from_another_site: bool, now_ms: u64) -> Admit {
        if from_another_site {
            self.cross.admit(now_ms)
        } else {
            self.local.admit(now_ms)
        }
    }
}

/// A fixed-window count of failed-request lines. O(1) per request: three
/// counters and no loop. The bound on lines written per window is proven by
/// `api::logs::a_flood_of_failed_requests_writes_a_bounded_number_of_lines`;
/// the per-request time is by construction and UNVERIFIED by any measurement
/// (`docs/06-limits.md`, D-1459).
#[derive(Debug)]
pub(crate) struct Ration {
    /// When the current window opened.
    opened_ms: u64,
    /// Lines written in it.
    written: u64,
    /// Lines held back in it.
    suppressed: u64,
    /// Lines one window may write.
    limit: u64,
}

/// What one failed request may write.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Admit {
    /// Whether its own line is written.
    pub(crate) write: bool,
    /// How many lines the window that just closed held back, when non-zero.
    pub(crate) summary: Option<u64>,
}

impl Ration {
    pub(crate) const fn new(limit: u64) -> Self {
        Self {
            opened_ms: 0,
            written: 0,
            suppressed: 0,
            limit,
        }
    }

    /// Accounts one failed request at `now_ms`.
    pub(crate) fn admit(&mut self, now_ms: u64) -> Admit {
        let mut summary = None;
        if now_ms.saturating_sub(self.opened_ms) >= FAILED_LINE_WINDOW_MS || now_ms < self.opened_ms
        {
            if self.suppressed > 0 {
                summary = Some(self.suppressed);
            }
            self.opened_ms = now_ms;
            self.written = 0;
            self.suppressed = 0;
        }
        let write = self.written < self.limit;
        if write {
            self.written = self.written.saturating_add(1);
        } else {
            self.suppressed = self.suppressed.saturating_add(1);
        }
        Admit { write, summary }
    }
}

// ---------------------------------------------------------------------------
// KEYED READS: feed, instrument, day and contract (D-3341 item 3; DB-023,
// DB-G88, DB-025)
// ---------------------------------------------------------------------------
//
// # What this adds, and why the ring alone could not
//
// `/logs.json` filtered by level, target and run, over the newest 4 MiB of a
// ring that held 4 to 12 minutes during a pull. "Why was NIFTY refused on
// 2022-03-07" was a question it could answer only within those minutes, and
// only by a caller who knew the target to ask for.
//
// A request naming a `feed`, an `instrument`, a `day` or a `contract` is
// answered from the durable audit stream each sink keeps
// (`telemetry::audit_postings`): ONE posting file per key, named by the key, in
// each half (the server's directory and `cli`'s), and one positional read per
// line it points at. Nothing is scanned to find them.
//
// Since WP-33c (DB-A44) that is the WHOLE answer by default, one page at a
// time: the postings are merged newest first from a cursor (`before=`, the
// `next` the previous page handed back), so every match can be reached, none
// twice, and a page does not move when unrelated lines arrive. The ring is
// read only when `ring=1` asks for it, on a first page, for lines no stream
// holds (`warn` and `info` detail, and lines older than the stream): those
// rows are labelled `source: ring`, are never paged, and move as the ring
// rolls, which is why they are asked for and never mixed in. Before WP-33c
// every keyed request read the newest 4 MiB of both rings (5,352,506 bytes on
// the operator's store, p50 22-28 ms), and its count moved with the ring.
//
// # What "bears on" means
//
// A key matches an event that names it in a field (`feed`/`vendor`,
// `instrument`/`symbol` with `segment` when both carry one, `day`, `contract`),
// and three things it cannot match by field:
//
// * an event whose target names the feed (`gdfl.import` for `feed=gdfl`): the
//   GDFL importer names its kind, day and file, never its feed;
// * a GDFL importer refusal about a FILE: the file name is resolved through
//   `pull::gdfl_cm::resolve` (`NIFTY 50.NSE_IDX.csv` is `NSE-INDEX-NIFTY`) and
//   an option ticker through `pull::gdfl_nfo::decode_ticker`, the importer's
//   own two readers, so the API cannot disagree with what was imported;
// * a refusal of a WHOLE DAY (the calendar, the census, a listing) of a kind:
//   it bears on every instrument of that kind's segment that day, and the
//   record says so with `"scope":"day"`. An event about the month a day falls
//   in carries `"scope":"month"`; an event naming the key itself, `"event"`.
//
// # Cost
//
// Per half: at most one `metadata` call per kept generation and one read per
// probed posting (two for a day: the day and its month), then one positional
// read per pointer read, at most [`MAX_KEYED_READS`] across the request; and
// the ring's [`SCAN_BYTES`] only when `ring=1` asked for it. None of it grows
// with the stream, the index or the store.

/// The longest `contract` a keyed request reads: a store contract segment
/// (`brutex_core::instrument::CONTRACT_CAPACITY`) or a vendor ticker, which
/// can be longer.
pub const MAX_CONTRACT_ASKED: usize = 64;

/// The most audit lines one keyed request reads, both halves together. The
/// answer says when pointers were left unread (`index.capped`).
pub const MAX_KEYED_READS: usize = 2_000;

/// What a keyed request narrows by.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Keys {
    /// The feed's wire, as `Vendor::as_str` spells it.
    pub(crate) feed: Option<String>,
    /// The instrument.
    pub(crate) instrument: Option<Instrument>,
    /// The day.
    pub(crate) day: Option<pull::session::Day>,
    /// The contract, as the store names its segment, or a vendor ticker.
    pub(crate) contract: Option<String>,
    /// A refusal's code (WP-33c, DB-A47).
    pub(crate) code: Option<String>,
    /// A run, probed in the index's `run` posting: set by `durable=1`.
    pub(crate) run: Option<u64>,
    /// A target, probed in the index's `target` posting: set by `durable=1`
    /// with no run, and by a `target=` that names no other key (R3-24).
    pub(crate) target: Option<String>,
    /// A month, `YYYY-MM`: a line's own `month`, or its `day`'s (R3-24).
    pub(crate) month: Option<String>,
    /// A line's own `reason` (R3-24).
    pub(crate) reason: Option<String>,
    /// An import kind, as `pull::gdfl_import::ImportKind` spells it (R3-24).
    pub(crate) kind: Option<String>,
    /// A time span, `from..=to` in epoch milliseconds, either end open as
    /// `i64::MIN` or `i64::MAX` (R3-24). Alone it is read through the audit
    /// files whose time span meets it ([`span_page`]); beside another key it
    /// narrows that key's answer and passes over every file outside it.
    pub(crate) span: Option<(i64, i64)>,
}

impl Keys {
    /// Whether no key was asked for.
    pub(crate) const fn is_empty(&self) -> bool {
        self.feed.is_none()
            && self.instrument.is_none()
            && self.day.is_none()
            && self.contract.is_none()
            && self.code.is_none()
            && self.run.is_none()
            && self.target.is_none()
            && self.month.is_none()
            && self.reason.is_none()
            && self.kind.is_none()
            && self.span.is_none()
    }
}

/// An instrument as a request names it: `NSE-INDEX-NIFTY`, `NSE-NIFTY` or
/// `NIFTY`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Instrument {
    /// The text asked.
    pub(crate) asked: String,
    /// The exchange, when named.
    pub(crate) exchange: Option<String>,
    /// The segment, when named.
    pub(crate) segment: Option<String>,
    /// The symbol.
    pub(crate) symbol: String,
}

/// The instrument `text` names, or `None` when its symbol is not one the store
/// files (`brutex_core::symbol::Symbol`).
pub(crate) fn instrument_of(text: &str) -> Option<Instrument> {
    use brutex_core::instrument::{Exchange, Segment};
    let exchange = text
        .split_once('-')
        .filter(|(exchange, _)| Exchange::parse(exchange).is_ok());
    let (exchange, segment, symbol) = match exchange {
        Some((exchange, rest)) => match rest.split_once('-') {
            Some((segment, symbol)) if Segment::parse(segment).is_ok() => {
                (Some(exchange), Some(segment), symbol)
            }
            _ => (Some(exchange), None, rest),
        },
        None => (None, None, text),
    };
    brutex_core::symbol::Symbol::new(symbol).ok()?;
    Some(Instrument {
        asked: text.to_owned(),
        exchange: exchange.map(str::to_owned),
        segment: segment.map(str::to_owned),
        symbol: symbol.to_owned(),
    })
}

/// How an event bears on a request's keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    /// It names the key itself.
    Event,
    /// It refuses a whole day of a kind the key belongs to.
    Day,
    /// It is about the month the asked day falls in.
    Month,
}

impl Scope {
    /// The wire's word.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Event => "event",
            Self::Day => "day",
            Self::Month => "month",
        }
    }

    /// The wider of two.
    const fn widest(self, other: Self) -> Self {
        match (self, other) {
            (Self::Month, _) | (_, Self::Month) => Self::Month,
            (Self::Day, _) | (_, Self::Day) => Self::Day,
            _ => Self::Event,
        }
    }
}

/// The text of one field.
fn text_of<'r>(record: &'r telemetry::Record, key: &str) -> Option<&'r str> {
    record.field(key).and_then(telemetry::OwnedValue::as_str)
}

/// What a GDFL importer event's `about` names, read with the importer's own
/// readers.
#[derive(Debug, Clone, PartialEq, Eq)]
enum About {
    /// A file of one instrument: its symbol, the segment its kind files it
    /// under, and the contract an option ticker names.
    File {
        symbol: String,
        segment: &'static str,
        contract: Option<String>,
    },
    /// A file the importer does not file (a reference index, a series
    /// variant): it bears on no swept instrument.
    Unfiled,
    /// The whole day of the kind: the calendar, the census, the listing.
    WholeDay { segment: &'static str },
}

/// What `record`, a GDFL importer event, is about; `None` when it is not one
/// or names no kind and day.
fn gdfl_about(record: &telemetry::Record) -> Option<About> {
    use pull::gdfl_cm::{CmKind, Resolution};
    use pull::gdfl_import::ImportKind;
    if record.target != pull::gdfl_import::TARGET {
        return None;
    }
    let kind = ImportKind::parse(text_of(record, "kind")?).ok()?;
    let segment = kind.segment();
    let about = text_of(record, "about").unwrap_or("");
    let cm = match kind {
        ImportKind::Indices => Some(CmKind::Indices),
        ImportKind::Stocks => Some(CmKind::Stocks),
        ImportKind::Options => None,
    };
    if let Some(cm) = cm {
        let Ok((stem, _)) = pull::gdfl_cm::split_name(about) else {
            return Some(About::WholeDay { segment });
        };
        return Some(match pull::gdfl_cm::resolve(cm, stem) {
            Resolution::Swept(key) => About::File {
                symbol: key.underlying.as_str().to_owned(),
                segment,
                contract: None,
            },
            _ => About::Unfiled,
        });
    }
    let day = text_of(record, "day").and_then(crate::server::import_journal::parse_day)?;
    Some(match pull::gdfl_nfo::decode_ticker(about, day) {
        Ok(ticker) => About::File {
            symbol: ticker.underlying.as_str().to_owned(),
            segment,
            contract: Some(ticker.contract.to_string()),
        },
        // NOT A TICKER: the calendar, the census, the listing. A ticker the
        // reader refused is still a file of one contract, and the event says
        // so in `about`; it is matched by its text below.
        Err(_) if !about.ends_with("CE") && !about.ends_with("PE") => About::WholeDay { segment },
        Err(_) => About::Unfiled,
    })
}

/// How `record` bears on `keys`, or `None` when it does not.
pub(crate) fn bears(record: &telemetry::Record, keys: &Keys) -> Option<Scope> {
    let mut scope = Scope::Event;
    let about = gdfl_about(record);
    if let Some(ref feed) = keys.feed {
        let named = text_of(record, "feed")
            .or_else(|| text_of(record, "vendor"))
            .is_some_and(|wire| wire == feed);
        let by_target = record.target.split('.').next() == Some(feed.as_str());
        if !named && !by_target {
            return None;
        }
    }
    if let Some(day) = keys.day {
        let asked = day.to_string();
        if let Some(text) = text_of(record, "day") {
            if text != asked {
                return None;
            }
        } else if text_of(record, "month") == asked.get(..7) {
            scope = scope.widest(Scope::Month);
        } else {
            return None;
        }
    }
    if let Some(ref instrument) = keys.instrument {
        scope = scope.widest(instrument_bears(record, about.as_ref(), instrument)?);
    }
    if let Some(ref contract) = keys.contract {
        let named = text_of(record, "contract") == Some(contract.as_str())
            || text_of(record, "about") == Some(contract.as_str());
        let decoded = matches!(
            about,
            Some(About::File { contract: Some(ref held), .. }) if held == contract
        );
        if !named && !decoded {
            return None;
        }
    }
    if let Some(ref code) = keys.code {
        let named = text_of(record, "code")
            .or_else(|| text_of(record, "reason"))
            .is_some_and(|held| held == code);
        if !named {
            return None;
        }
    }
    if let Some(run) = keys.run
        && run_of(record) != Some(run)
    {
        return None;
    }
    if let Some(ref target) = keys.target
        && !target_admits(&record.target, target)
    {
        return None;
    }
    // R3-24's KEYS: each a field the line itself carries, or its time.
    if let Some(ref month) = keys.month {
        let held = text_of(record, "month")
            .filter(|held| is_month(held))
            .or_else(|| text_of(record, "day").and_then(|day| day.get(..7)));
        if held != Some(month.as_str()) {
            return None;
        }
    }
    if let Some(ref reason) = keys.reason
        && text_of(record, "reason") != Some(reason.as_str())
    {
        return None;
    }
    if let Some(ref kind) = keys.kind
        && text_of(record, "kind") != Some(kind.as_str())
    {
        return None;
    }
    if let Some((from, to)) = keys.span
        && (record.at_unix_millis < from || record.at_unix_millis > to)
    {
        return None;
    }
    Some(scope)
}

/// The run a line belongs to: its top-level `run`, else a numeric `run`
/// field (the GDFL importer's `run started` and `run finished`), as the
/// index files it (`telemetry` `Keyed::run`).
fn run_of(record: &telemetry::Record) -> Option<u64> {
    if record.run != 0 {
        return Some(record.run);
    }
    record
        .field("run")
        .and_then(telemetry::OwnedValue::as_u64)
        .filter(|run| *run != 0)
}

/// Whether a line's target is `asked` or beneath it (`pull` covers
/// `pull.http`, never `pullover`).
fn target_admits(target: &str, asked: &str) -> bool {
    target == asked
        || target
            .strip_prefix(asked)
            .is_some_and(|rest| rest.starts_with('.'))
}

/// How `record` bears on one instrument.
fn instrument_bears(
    record: &telemetry::Record,
    about: Option<&About>,
    asked: &Instrument,
) -> Option<Scope> {
    let segment_fits = |segment: &str| {
        asked
            .segment
            .as_deref()
            .is_none_or(|wanted| wanted == segment)
    };
    if let Some(about) = about {
        return match *about {
            About::File {
                ref symbol,
                segment,
                ..
            } => (*symbol == asked.symbol && segment_fits(segment)).then_some(Scope::Event),
            About::WholeDay { segment } => segment_fits(segment).then_some(Scope::Day),
            About::Unfiled => None,
        };
    }
    let named = text_of(record, "instrument")
        .or_else(|| text_of(record, "symbol"))
        .is_some_and(|name| name == asked.symbol || name == asked.asked);
    let agrees = |field: &str, wanted: Option<&String>| {
        wanted
            .is_none_or(|wanted| text_of(record, field).is_none_or(|held| held == wanted.as_str()))
    };
    (named
        && agrees("segment", asked.segment.as_ref())
        && agrees("exchange", asked.exchange.as_ref()))
    .then_some(Scope::Event)
}

/// The postings a keyed request probes: the most selective key it names.
fn probes(keys: &Keys) -> Vec<(&'static str, String)> {
    let gdfl = keys
        .feed
        .as_deref()
        .is_none_or(|feed| feed == brutex_core::vendor::Vendor::Gdfl.as_str());
    let mut out = Vec::with_capacity(3);
    if let Some(day) = keys.day {
        let text = day.to_string();
        out.push(("month", text.get(..7).unwrap_or_default().to_owned()));
        out.push(("day", text));
    } else if let Some(ref contract) = keys.contract {
        out.push(("contract", contract.clone()));
        if gdfl {
            out.push(("target", pull::gdfl_import::TARGET.to_owned()));
        }
    } else if let Some(ref instrument) = keys.instrument {
        out.push(("instrument", instrument.symbol.clone()));
        if instrument.asked != instrument.symbol {
            out.push(("instrument", instrument.asked.clone()));
        }
        if gdfl {
            out.push(("target", pull::gdfl_import::TARGET.to_owned()));
        }
    } else if let Some(ref code) = keys.code {
        out.push(("code", code.clone()));
    } else if let Some(ref reason) = keys.reason {
        out.push(("reason", reason.clone()));
    } else if let Some(ref month) = keys.month {
        out.push(("month", month.clone()));
    } else if let Some(ref kind) = keys.kind {
        out.push(("kind", kind.clone()));
    } else if let Some(run) = keys.run {
        out.push(("run", run.to_string()));
    } else if let Some(ref feed) = keys.feed {
        out.push(("feed", feed.clone()));
        if gdfl {
            out.push(("target", pull::gdfl_import::TARGET.to_owned()));
        }
    } else if let Some(ref target) = keys.target {
        out.push(("target", target.clone()));
    } else if let Some((from, to)) = keys.span {
        // A SPAN ALONE names no posting: its stream reads the audit files
        // whose time span meets it ([`span_page`]).
        out.push((SPAN_PROBE, format!("{from}..{to}")));
    }
    out
}

/// The probe a span alone is read by, in place of a posting's dimension.
const SPAN_PROBE: &str = "span";

/// How many bytes a span stream reads from one audit file per request, at
/// most, beside its binary search: a fixed slice, so a span over a busy
/// hour costs what one over a quiet one does, and the cursor reaches the
/// rest.
pub const SPAN_SLICE_BYTES: u64 = 1024 * 1024;

/// One line of an audit file found by [`line_after`]: where it starts, its
/// length with its newline, and the record.
struct Placed {
    start: u64,
    len: u64,
    record: telemetry::Record,
}

/// The first whole line of `path` starting at or after `at` and before `end`,
/// or `None` when no line starts there. One read of at most two lines'
/// ceiling ([`telemetry::MAX_LINE_BYTES`]).
fn line_after(path: &std::path::Path, at: u64, end: u64) -> Option<Placed> {
    use std::io::{Read as _, Seek as _};
    let from = at.saturating_sub(1);
    let ceiling = u64::try_from(telemetry::MAX_LINE_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_mul(2);
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .and_then(|mut file| {
            file.seek(std::io::SeekFrom::Start(from))?;
            file.take(ceiling.min(end.saturating_sub(from)))
                .read_to_end(&mut bytes)
        })
        .ok()?;
    // AT 0 THE FIRST LINE STARTS THERE; else after the first newline from
    // the byte before `at`, so a line starting exactly at `at` is found.
    let skip = if at == 0 {
        0
    } else {
        bytes
            .iter()
            .position(|&byte| byte == b'\n')?
            .saturating_add(1)
    };
    let rest = bytes.get(skip..)?;
    let len = rest.iter().position(|&byte| byte == b'\n')?;
    let record = telemetry::Record::decode(rest.get(..len)?).ok()?;
    Some(Placed {
        start: from.saturating_add(u64::try_from(skip).ok()?),
        len: u64::try_from(len).ok()?.saturating_add(1),
        record,
    })
}

/// The offset in `path` where the first line stamped after `to` starts, below
/// `end`: every line before it is stamped at or before `to`. A binary search
/// over byte offsets, each step one [`line_after`] read, so O(log(file size))
/// reads whatever the file holds. Lines are in time order: the sink stamps
/// them under its lock and clamps its clock, so `ms` order is file order
/// (`telemetry` `Inner::stamp`).
fn end_of_span(path: &std::path::Path, end: u64, to: i64) -> u64 {
    // `low` is always a line's start: every line starting before it is
    // stamped at or before `to`. A probe that finds no line starting below
    // `high` narrows to `middle` and loses nothing: the line straddling
    // `middle` starts below it and is read whole, to `end`, when a later probe
    // reaches it, and if it is at or before `to` then `low` moves past it to
    // where `high` was.
    let (mut low, mut high) = (0u64, end);
    while low < high {
        let middle = low + (high - low) / 2;
        match line_after(path, middle, end) {
            Some(line) if line.start < high && line.record.at_unix_millis > to => {
                high = line.start;
            }
            Some(line) if line.start < high => low = line.start.saturating_add(line.len),
            _ => high = middle,
        }
    }
    low
}

/// One page of a span read alone (R3-24): the lines of the kept audit files
/// stamped `from..=to`, newest first, strictly older than `before`, at most
/// `take`, as places and pointers a [`Stream`] reads exactly as it reads a
/// posting's. A place is `(serial, the line's byte offset)`.
///
/// # Cost
///
/// Per kept generation, newest first: one [`telemetry::run_dirs::audit_span`]
/// probe (a file whose span misses `from..=to` is never opened), one binary
/// search ([`end_of_span`]) and backward reads of at most
/// [`SPAN_SLICE_BYTES`] in all. The walk stops at the first line older than
/// `from`: every older line is older still.
///
/// Answers the page and whether older lines in the span may remain: `true`
/// when it stopped at `take` lines or at its byte slice, never a guess.
///
/// # Errors
///
/// The serial, or a file that is there and cannot be read.
fn span_page(
    dir: &std::path::Path,
    (from, to): (i64, i64),
    take: usize,
    before: Option<telemetry::PostingAt>,
) -> Result<(telemetry::PostingsPage, bool), String> {
    let current = telemetry::audit_serial(dir)?;
    let oldest = current.saturating_sub(u64::from(telemetry::AUDIT_KEEP_FILES.saturating_sub(1)));
    let mut out = telemetry::PostingsPage::default();
    let mut budget = SPAN_SLICE_BYTES;
    for serial in (oldest..=current).rev() {
        if before.is_some_and(|at| serial > at.serial) {
            continue;
        }
        let (Some(path), Some(span)) = (
            telemetry::audit_path_for(dir, current, serial),
            telemetry::run_dirs::audit_span(dir, current, serial),
        ) else {
            continue;
        };
        if span.last_ms.is_some_and(|last| last < from) {
            // EVERY OLDER FILE IS OLDER STILL.
            return Ok((out, false));
        }
        if !span.overlaps(from, to) {
            continue;
        }
        let len = match std::fs::metadata(&path) {
            Ok(meta) => meta.len(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        let end = before
            .filter(|at| at.serial == serial)
            .map_or(len, |at| at.index.min(len));
        let cut = end_of_span(&path, end, to);
        match span_backwards(&path, serial, (cut, from), take, &mut budget, &mut out)? {
            Walked::Older => return Ok((out, false)),
            Walked::Stopped => return Ok((out, true)),
            Walked::FileStart => {}
        }
    }
    Ok((out, false))
}

/// How a backward walk of one audit file ended.
enum Walked {
    /// It reached a line older than the span: every older line is older.
    Older,
    /// It took `take` lines or spent its slice: older lines may remain.
    Stopped,
    /// It reached the file's first line.
    FileStart,
}

/// Reads `path` backwards from `cut`, one slice at a time and whole lines
/// only, pushing every line stamped at or after `from` until `take`, the
/// slice `budget` or a line older than `from`.
fn span_backwards(
    path: &std::path::Path,
    serial: u64,
    (mut cut, from): (u64, i64),
    take: usize,
    budget: &mut u64,
    out: &mut telemetry::PostingsPage,
) -> Result<Walked, String> {
    use std::io::{Read as _, Seek as _};
    while cut > 0 {
        if *budget == 0 || out.pointers.len() >= take {
            return Ok(Walked::Stopped);
        }
        let slice = (*budget).min(cut);
        let start = cut.saturating_sub(slice);
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .and_then(|mut file| {
                file.seek(std::io::SeekFrom::Start(start))?;
                file.take(slice).read_to_end(&mut bytes)
            })
            .map_err(|e| format!("{}: {e}", path.display()))?;
        *budget = budget.saturating_sub(slice);
        // A SLICE THAT DOES NOT START THE FILE starts mid-line: what is
        // before its first newline belongs to the next slice. A slice with
        // no newline at all is a slice spent on part of one line.
        let first = if start == 0 {
            0
        } else {
            match bytes.iter().position(|&byte| byte == b'\n') {
                Some(at) => at.saturating_add(1),
                None => return Ok(Walked::Stopped),
            }
        };
        let whole = bytes.get(first..).unwrap_or_default();
        let base = start.saturating_add(u64::try_from(first).unwrap_or(u64::MAX));
        let mut lines: Vec<(u64, &[u8])> = Vec::new();
        let mut offset = base;
        for line in whole.split_inclusive(|&byte| byte == b'\n') {
            lines.push((offset, line));
            offset = offset.saturating_add(u64::try_from(line.len()).unwrap_or(u64::MAX));
        }
        for (at, line) in lines.into_iter().rev() {
            if out.pointers.len() >= take {
                return Ok(Walked::Stopped);
            }
            let Ok(record) = telemetry::Record::decode(line.strip_suffix(b"\n").unwrap_or(line))
            else {
                out.malformed = out.malformed.saturating_add(1);
                continue;
            };
            if record.at_unix_millis < from {
                return Ok(Walked::Older);
            }
            out.total = out.total.saturating_add(1);
            out.pointers.push((
                telemetry::PostingAt { serial, index: at },
                telemetry::AuditPointer {
                    serial,
                    offset: at,
                    seq: record.seq,
                    len: u32::try_from(line.len()).unwrap_or(u32::MAX),
                    level: record.level,
                },
            ));
        }
        cut = base;
    }
    Ok(Walked::FileStart)
}

/// One posting of one half, read newest first from a place (WP-33c, DB-A44).
struct Stream<'d> {
    half: &'static str,
    /// Which of the half's two durable streams this reads: `audit`, the
    /// rolling one, or `state`, the never-rolling one (R3-24).
    source: &'static str,
    dir: &'d std::path::Path,
    /// A span the request narrows to, beside a key: a pointer into a file
    /// whose time span misses it is passed over unread (R3-24).
    prune: Option<(i64, i64)>,
    /// The spans looked up so far, by serial: one small read per file.
    spans: std::collections::HashMap<u64, Option<telemetry::run_dirs::AuditSpan>>,
    /// The half's current serial, as the pointers were read.
    serial: u64,
    dimension: &'static str,
    value: String,
    /// The pointers fetched, newest first, strictly older than the cursor.
    page: telemetry::PostingsPage,
    /// The fetched pointer the stream stands at.
    next: usize,
    /// The place of the last pointer this page consumed.
    last: Option<telemetry::PostingAt>,
    /// The place this page started from.
    from: Slot,
    /// Whether the posting may hold pointers older than those fetched.
    more: bool,
    /// The stream's next line, read and not yet consumed.
    head: Option<(telemetry::AuditPointer, telemetry::Record)>,
    /// Pointers this page consumed.
    read: usize,
}

/// Where one stream of a keyed answer stands, as a cursor names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    /// Not started: from the newest pointer.
    Newest,
    /// After this place: the next page reads strictly older pointers.
    At(telemetry::PostingAt),
    /// Every pointer read.
    Done,
}

impl Slot {
    /// The cursor's text for one stream: `*`, `<serial>.<index>` or `-`.
    fn text(self) -> String {
        match self {
            Self::Newest => "*".to_owned(),
            Self::At(at) => format!("{}.{}", at.serial, at.index),
            Self::Done => "-".to_owned(),
        }
    }

    /// One stream's cursor text, or `None` when it is not one.
    fn read(text: &str) -> Option<Self> {
        match text {
            "*" => Some(Self::Newest),
            "-" => Some(Self::Done),
            _ => {
                let (serial, index) = text.split_once('.')?;
                let number =
                    |part: &str| part.parse::<u64>().ok().filter(|n| n.to_string() == part);
                Some(Self::At(telemetry::PostingAt {
                    serial: number(serial)?,
                    index: number(index)?,
                }))
            }
        }
    }
}

/// The cursor a keyed answer hands back: one [`Slot`] per (half, posting),
/// joined by `~`, in the order the halves and the probes are named. `None`
/// when every stream is done: there is no next page.
fn cursor_text(slots: &[Slot]) -> Option<String> {
    slots.iter().any(|slot| *slot != Slot::Done).then(|| {
        slots
            .iter()
            .map(|slot| slot.text())
            .collect::<Vec<_>>()
            .join("~")
    })
}

impl Stream<'_> {
    /// Reads the stream's next line into its head, consuming pointers whose
    /// line has rolled off or will not read (each counted). One positional
    /// read per pointer, each against `budget`.
    fn fill(&mut self, budget: &mut usize, rolled_off: &mut u64, errors: &mut Vec<String>) {
        while self.head.is_none() && *budget > 0 {
            let Some(&(at, pointer)) = self.page.pointers.get(self.next) else {
                return;
            };
            // A FILE OUTSIDE THE SPAN ASKED FOR is passed over unread: its
            // pointers cost a lookup, never a line read (R3-24).
            if let Some((from, to)) = self.prune {
                let (dir, current) = (self.dir, self.serial);
                let span = *self.spans.entry(pointer.serial).or_insert_with(|| {
                    telemetry::run_dirs::audit_span(dir, current, pointer.serial)
                });
                if span.is_some_and(|span| !span.overlaps(from, to)) {
                    self.consume(at);
                    continue;
                }
            }
            *budget = budget.saturating_sub(1);
            match telemetry::audit_read(self.dir, self.serial, &pointer) {
                telemetry::AuditLine::Found(record) => self.head = Some((pointer, record)),
                telemetry::AuditLine::RolledOff => {
                    *rolled_off = rolled_off.saturating_add(1);
                    self.consume(at);
                }
                telemetry::AuditLine::Unreadable(why) => {
                    errors.push(why);
                    self.consume(at);
                }
            }
        }
    }

    /// Steps past the pointer at `at`.
    fn consume(&mut self, at: telemetry::PostingAt) {
        self.next = self.next.saturating_add(1);
        self.last = Some(at);
        self.read = self.read.saturating_add(1);
        self.head = None;
    }

    /// The place of the head, when there is one.
    fn head_at(&self) -> Option<telemetry::PostingAt> {
        self.head
            .as_ref()
            .and_then(|_| self.page.pointers.get(self.next).map(|(at, _)| *at))
    }

    /// Where the stream stands for the next page.
    fn slot(&self) -> Slot {
        if self.head.is_none() && self.next >= self.page.pointers.len() && !self.more {
            return Slot::Done;
        }
        self.last.map_or(self.from, Slot::At)
    }
}

/// What a keyed answer's index half found: its rows, in order, and how it
/// read them.
#[derive(Debug, Default)]
struct Indexed {
    /// `(half, source, scope, record)`, newest first: `source` is the stream
    /// the line was read from, `audit` or `state` (R3-24).
    rows: Vec<(&'static str, &'static str, Scope, telemetry::Record)>,
    /// The postings JSON, one object per (half, posting).
    postings: Vec<String>,
    /// The cursor for the next page, or `None` when every stream is done.
    next: Option<String>,
    /// Pointers left unread at [`MAX_KEYED_READS`].
    capped: bool,
    /// Pointers whose line has rolled out of the kept stream.
    rolled_off: u64,
    /// Lines read.
    reads: usize,
    /// A cursor that does not fit this request, named.
    cursor_refused: Option<String>,
    /// Failures, in their own words.
    errors: Vec<String>,
    /// Pointers the probed postings hold in all, every stream together: the
    /// `N` of "the first 2,000 of N" a capped answer says (R3-24). A line
    /// held in two postings, or in both of a half's streams, is counted in
    /// each, so it is an upper bound on the lines, never a count of matches.
    of: u64,
    /// Whether a span stream's count is open: it reads a span's lines a slice
    /// at a time, so it knows how many it found and not how many there are.
    of_open: bool,
}

/// One durable stream a keyed answer reads: the half it belongs to, which of
/// the half's two streams it is (`audit` or `state`, R3-24), and where.
type IndexDir<'d> = (&'static str, &'static str, &'d std::path::Path);

/// The index half of a keyed answer: every (half, posting) stream merged
/// newest first, read from the cursor, a line in two postings of one half
/// shown once, until `limit` lines bear on the keys or [`MAX_KEYED_READS`]
/// lines were read (WP-33c, DB-A44).
///
/// # Cost
///
/// Per stream: [`telemetry::audit_postings_before`] (O(generations) `metadata`
/// calls and one read of at most [`MAX_KEYED_READS`] pointers at a computed
/// offset), then one positional read per line, at most [`MAX_KEYED_READS`]
/// across the request. No ring is read and nothing grows with the stream:
/// the same request costs the same at any size, and a page past the first
/// starts where the last one stopped, so every match can be reached and none
/// is shown twice. Lines appended later are newer than every place already
/// handed out, so they never shift a later page.
fn index_rows(halves: &[IndexDir<'_>], asked: &Asked) -> Indexed {
    let mut out = Indexed::default();
    let probes = probes(&asked.keys);
    let wanted = halves.len().saturating_mul(probes.len());
    let Some(slots) = cursor_slots(asked.before.as_deref(), wanted, &mut out) else {
        return out;
    };
    let mut opened = open_streams(halves, &probes, &slots, &asked.keys, &mut out);
    out.of = opened
        .streams
        .iter()
        .fold(0u64, |sum, stream| sum.saturating_add(stream.page.total));
    out.of_open = opened
        .streams
        .iter()
        .any(|stream| stream.dimension == SPAN_PROBE && stream.more);
    let budget = merge_streams(&mut opened.streams, asked, &mut out);
    out.reads = MAX_KEYED_READS.saturating_sub(budget);
    let cursor: Vec<Slot> = opened
        .slot_of
        .iter()
        .enumerate()
        .map(|(index, held)| match *held {
            Some(at) => opened.streams.get(at).map_or(Slot::Done, Stream::slot),
            None => opened.fixed.get(index).copied().unwrap_or(Slot::Done),
        })
        .collect();
    out.next = cursor_text(&cursor);
    out.postings.extend(opened.streams.iter().map(posting_json));
    out
}

/// The cursor a request carried, as one [`Slot`] per (half, posting), or
/// every stream from its newest when it carried none. A cursor that is not
/// one of this request's is refused by name in `out` and `None` answered: no
/// page is read, never page one passed off as the next.
fn cursor_slots(before: Option<&str>, wanted: usize, out: &mut Indexed) -> Option<Vec<Slot>> {
    let Some(text) = before else {
        return Some(vec![Slot::Newest; wanted]);
    };
    let read: Option<Vec<Slot>> = text.split('~').map(Slot::read).collect();
    let fits = read.filter(|slots| slots.len() == wanted);
    if fits.is_none() {
        out.cursor_refused = Some(format!(
            "before={text:?} is not a cursor of this request: it names {} place(s) and this \
             request reads {wanted} posting(s); no page was read",
            text.split('~').count()
        ));
    }
    fits
}

/// The streams of one keyed request, and where each cursor slot stands.
struct Opened<'d> {
    streams: Vec<Stream<'d>>,
    /// Per cursor slot, the stream reading it, or `None` for a slot no stream
    /// reads (a half with no stream, a posting done, a read that failed).
    slot_of: Vec<Option<usize>>,
    /// Per cursor slot, what it stays when no stream reads it.
    fixed: Vec<Slot>,
}

/// Opens one stream per (half, posting) the cursor has not finished: one
/// [`telemetry::audit_postings_before`] each. A half with no audit stream is
/// named in `out` (DB-A46), never read as one holding nothing.
fn open_streams<'d>(
    halves: &[IndexDir<'d>],
    probes: &[(&'static str, String)],
    slots: &[Slot],
    keys: &Keys,
    out: &mut Indexed,
) -> Opened<'d> {
    let mut opened = Opened {
        streams: Vec::new(),
        slot_of: Vec::new(),
        fixed: Vec::new(),
    };
    for (h, &(half, source, dir)) in halves.iter().enumerate() {
        let state = telemetry::audit_state(dir);
        for (p, (dimension, value)) in probes.iter().enumerate() {
            let from = slots
                .get(h.saturating_mul(probes.len()).saturating_add(p))
                .copied()
                .unwrap_or(Slot::Done);
            let serial = match state {
                // NO STREAM IS NOT AN EMPTY STREAM (DB-A46): said by name,
                // never `total: 0`.
                Ok(None) => {
                    out.postings.push(format!(
                        r#"{{"half":{},"source":{},"dimension":{},"value":{},"state":"no {source} stream","total":null,"read":0}}"#,
                        render::json_string(half),
                        render::json_string(source),
                        render::json_string(dimension),
                        render::json_string(value),
                    ));
                    opened.slot_of.push(None);
                    opened.fixed.push(Slot::Done);
                    continue;
                }
                Err(ref why) => {
                    out.errors.push(why.clone());
                    opened.slot_of.push(None);
                    opened.fixed.push(from);
                    continue;
                }
                Ok(Some(serial)) => serial,
            };
            let before = match from {
                Slot::At(at) => Some(at),
                Slot::Newest => None,
                Slot::Done => {
                    opened.slot_of.push(None);
                    opened.fixed.push(Slot::Done);
                    continue;
                }
            };
            // A SPAN ALONE is read from the files its time meets; every other
            // probe is a posting (R3-24).
            let fetched = match keys.span {
                Some(span) if *dimension == SPAN_PROBE => {
                    span_page(dir, span, MAX_KEYED_READS, before)
                }
                _ => {
                    telemetry::audit_postings_before(dir, dimension, value, MAX_KEYED_READS, before)
                        .map(|page| {
                            let more = page.pointers.len() >= MAX_KEYED_READS;
                            (page, more)
                        })
                }
            };
            match fetched {
                Ok((page, more)) => {
                    opened.slot_of.push(Some(opened.streams.len()));
                    opened.fixed.push(from);
                    opened.streams.push(Stream {
                        half,
                        source,
                        dir,
                        prune: keys.span.filter(|_| *dimension != SPAN_PROBE),
                        spans: std::collections::HashMap::with_capacity(
                            usize::from(telemetry::AUDIT_KEEP_FILES).saturating_add(2),
                        ),
                        serial,
                        dimension,
                        value: value.clone(),
                        page,
                        next: 0,
                        last: None,
                        from,
                        more,
                        head: None,
                        read: 0,
                    });
                }
                Err(why) => {
                    out.errors.push(why);
                    opened.slot_of.push(None);
                    opened.fixed.push(from);
                }
            }
        }
    }
    opened
}

/// Merges the streams newest first into `out.rows` until `asked.limit` rows
/// bear on the keys or the read budget is spent; answers the budget left.
fn merge_streams(streams: &mut [Stream<'_>], asked: &Asked, out: &mut Indexed) -> usize {
    let mut budget = MAX_KEYED_READS;
    while out.rows.len() < asked.limit {
        for stream in streams.iter_mut() {
            stream.fill(&mut budget, &mut out.rolled_off, &mut out.errors);
        }
        // A STREAM WITH POINTERS LEFT AND NO HEAD READ could hold the newest
        // line: with the budget spent, the page stops rather than guess. So
        // does one whose fetched page is spent while its posting holds older
        // pointers (`more`): a line from another stream could be older than
        // one it has not fetched yet (R3-24).
        if streams
            .iter()
            .any(|s| s.head.is_none() && (s.next < s.page.pointers.len() || s.more))
        {
            out.capped = true;
            break;
        }
        // NEWEST FIRST; on a tie (the same line in a half's two streams) the
        // rolling audit stream's copy, which is where it was first written.
        let best = streams
            .iter()
            .enumerate()
            .filter_map(|(at, stream)| {
                stream.head.as_ref().map(|(_, record)| {
                    let half_rank = usize::from(stream.half != "served");
                    let source_rank = usize::from(stream.source == "audit");
                    (
                        at,
                        (record.at_unix_millis, half_rank, record.seq, source_rank),
                    )
                })
            })
            .max_by_key(|(_, key)| *key)
            .map(|(at, _)| at);
        let Some(chosen) = best.and_then(|best| streams.get_mut(best)) else {
            break;
        };
        let half = chosen.half;
        let (Some(at), Some((pointer, record))) = (chosen.head_at(), chosen.head.take()) else {
            break;
        };
        chosen.consume(at);
        // THE SAME LINE IN ANOTHER POSTING OF THIS HALF is the same line: it
        // stands at every such stream's head now, and is consumed there too.
        // So is its copy in the half's other stream: a state change is in
        // both the rolling audit stream and the state stream, at different
        // places, as the same line with the same `seq` and time (R3-24).
        let source = chosen.source;
        for other in streams.iter_mut() {
            if other.half == half
                && other.head.as_ref().is_some_and(|(held, line)| {
                    (other.source == source
                        && held.serial == pointer.serial
                        && held.offset == pointer.offset)
                        || (line.seq == record.seq && line.at_unix_millis == record.at_unix_millis)
                })
                && let Some(at) = other.head_at()
            {
                other.consume(at);
            }
        }
        if let Some(scope) = bears(&record, &asked.keys)
            && admitted(&record, asked)
        {
            out.rows.push((half, source, scope, record));
        }
    }
    budget
}

/// One stream's posting, as `index.postings` carries it.
fn posting_json(stream: &Stream<'_>) -> String {
    let uncovered: Vec<String> = stream
        .page
        .uncovered
        .iter()
        .map(|(serial, from)| {
            format!(
                r#"{{"serial":{serial},"indexed_from_offset":{}}}"#,
                from.map_or_else(|| "null".to_owned(), |from| from.to_string())
            )
        })
        .collect();
    format!(
        r#"{{"half":{},"source":{},"dimension":{},"value":{},"state":"held","total":{},"read":{},"malformed":{},"uncovered":[{}]}}"#,
        render::json_string(stream.half),
        render::json_string(stream.source),
        render::json_string(stream.dimension),
        render::json_string(&stream.value),
        stream.page.total,
        stream.read,
        stream.page.malformed,
        uncovered.join(","),
    )
}

/// Whether a record passes the request's level, target and run filters. A
/// keyed request's run is the line's own run as the index files it: its
/// top-level `run`, else its numeric `run` field (WP-33c, DB-A41).
fn admitted(record: &telemetry::Record, asked: &Asked) -> bool {
    asked.level.is_none_or(|level| record.level.at_least(level))
        && (asked.target.is_empty() || target_admits(&record.target, &asked.target))
        && (asked.run == 0 || run_of(record) == Some(asked.run))
}

/// What a bounded scan of one ring found.
#[derive(Debug, Default)]
struct Scanned {
    found: Vec<(Scope, telemetry::Record)>,
    bytes_read: u64,
    files_read: u64,
    hit_cap: bool,
    errors: Vec<String>,
}

/// The newest `budget` bytes of the ring in `dir`, newest file first, keeping
/// the records `keep` admits. A partial first line of a read is skipped, never
/// guessed at; so is an unterminated last one.
fn ring_scan(
    dir: &std::path::Path,
    budget: u64,
    keep: impl Fn(&telemetry::Record) -> Option<Scope>,
) -> Scanned {
    use std::io::{Read as _, Seek as _};
    let mut out = Scanned::default();
    let mut left = budget;
    for path in telemetry::paths_newest_first(dir, telemetry::DEFAULT_KEEP_FILES) {
        let len = match std::fs::metadata(&path) {
            Ok(meta) => meta.len(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                out.errors.push(format!("{}: {e}", path.display()));
                continue;
            }
        };
        if len == 0 {
            continue;
        }
        if left == 0 {
            out.hit_cap = true;
            break;
        }
        let take = len.min(left);
        let from = len.saturating_sub(take);
        let mut bytes = Vec::new();
        let read = std::fs::File::open(&path).and_then(|mut file| {
            file.seek(std::io::SeekFrom::Start(from))?;
            file.take(take).read_to_end(&mut bytes)
        });
        if let Err(e) = read {
            out.errors.push(format!("{}: {e}", path.display()));
            continue;
        }
        left = left.saturating_sub(take);
        out.bytes_read = out.bytes_read.saturating_add(take);
        out.files_read = out.files_read.saturating_add(1);
        let mut lines: Vec<&[u8]> = bytes.split(|&byte| byte == b'\n').collect();
        // The last piece is after the final newline: empty, or a line being
        // written. The first is a partial line when the read began mid-file.
        let _unterminated = lines.pop();
        let skip_first = usize::from(from > 0);
        for line in lines.iter().skip(skip_first).rev() {
            if let Ok(record) = telemetry::Record::decode(line)
                && let Some(scope) = keep(&record)
            {
                out.found.push((scope, record));
            }
        }
        if from > 0 {
            out.hit_cap = true;
            break;
        }
    }
    out
}

/// `/logs.json` for a request that names a key: the durable audit index of
/// both halves, newest first, one page from the cursor ([`index_rows`]); and,
/// only when `ring=1` asked for it on a first page, the newest [`SCAN_BYTES`]
/// of each ring for lines no stream holds (WP-33c, DB-A44). Each record
/// carries its half, its source and how it bears on the keys.
#[cfg(test)]
fn keyed_json(
    dir: &std::path::Path,
    cli_dir: Option<&std::path::Path>,
    asked: &Asked,
    health: Option<&telemetry::Health>,
) -> String {
    keyed_json_across(&two_halves(dir, cli_dir), asked, health)
}

/// [`keyed_json`] across every directory in `halves`: at most 8 directories
/// times at most 3 probes, 24 posting streams, under the one line budget
/// ([`MAX_KEYED_READS`]) the request has whatever the streams hold
/// (`docs/06-limits.md` L-IA-13).
fn keyed_json_across(halves: &[Half], asked: &Asked, health: Option<&telemetry::Health>) -> String {
    let Some(pair) = named_pair(halves) else {
        return NO_LOG_JSON.to_owned();
    };
    let (tail, keyed, _next) = keyed_walk(halves, asked);
    let marks = contradicted(&tail.records, store_of(halves).as_deref());
    let body = json_of_marked(&tail, asked, health, pair, &marks);
    let mut out = body.strip_suffix('}').unwrap_or(&body).to_owned();
    out.push_str(&keyed);
    out.push(',');
    out.push_str(&dirs_json(halves));
    out.push(',');
    out.push_str(&reach_json(halves, telemetry::now_millis()));
    out.push(',');
    out.push_str(&picture_json(served_picture().as_ref()));
    let (audit, state) = served_streams();
    out.push(',');
    out.push_str(&streams_json(audit.as_ref(), state.as_ref()));
    out.push('}');
    out
}

/// A keyed answer's records as a [`telemetry::Tail`] and its `keyed` object,
/// shared by `/logs.json` and the `/logs` page.
fn keyed_walk(named: &[Half], asked: &Asked) -> (telemetry::Tail, String, Option<String>) {
    let (tail, keyed, next, _said) = keyed_walk_said(named, asked);
    (tail, keyed, next)
}

/// [`keyed_walk`], with what it read in words ([`read_note`]).
fn keyed_walk_said(
    named: &[Half],
    asked: &Asked,
) -> (telemetry::Tail, String, Option<String>, String) {
    let halves: Vec<(&'static str, &std::path::Path)> = named
        .iter()
        .map(|half| (half.name, half.path.as_path()))
        .collect();
    // EACH HALF'S TWO DURABLE STREAMS (R3-24): the rolling audit stream and
    // the never-rolling state stream beside it, so a who-did-what line the
    // refusals rolled out of the first is still found in the second.
    let state_dirs: Vec<std::path::PathBuf> = named
        .iter()
        .map(|half| telemetry::run_dirs::state_stream_of(&half.path))
        .collect();
    let streams: Vec<IndexDir<'_>> = named
        .iter()
        .zip(&state_dirs)
        .flat_map(|(half, state)| {
            [
                (half.name, "audit", half.path.as_path()),
                (half.name, "state", state.as_path()),
            ]
        })
        .collect();
    let indexed = index_rows(&streams, asked);
    let mut rows: Vec<(&'static str, &'static str, Scope, telemetry::Record)> =
        indexed.rows.clone();
    // THE RING, ONLY WHEN ASKED, ONLY ON A FIRST PAGE: its rows move as the
    // ring moves and cannot be paged, so they are never mixed in unasked.
    let scan_ring = asked.ring && asked.before.is_none();
    let mut ring = (0u64, 0u64, false, Vec::new());
    if scan_ring {
        let mut seen: std::collections::HashSet<(&'static str, u64)> =
            rows.iter().map(|row| (row.0, row.3.seq)).collect();
        let mut found = Vec::new();
        for &(half, at) in &halves {
            let scanned = ring_scan(at, SCAN_BYTES, |record| {
                bears(record, &asked.keys).filter(|_| admitted(record, asked))
            });
            ring.0 = ring.0.saturating_add(scanned.bytes_read);
            ring.1 = ring.1.saturating_add(scanned.files_read);
            ring.2 |= scanned.hit_cap;
            ring.3.extend(scanned.errors);
            for (scope, record) in scanned.found {
                if seen.insert((half, record.seq)) {
                    found.push((half, "ring", scope, record));
                }
            }
        }
        // AT MOST `limit` RING ROWS, beside at most `limit` index rows: the
        // index rows of this page are never cut for a ring row, so the
        // cursor never skips one.
        found.sort_by(|left, right| {
            right
                .3
                .at_unix_millis
                .cmp(&left.3.at_unix_millis)
                .then_with(|| right.3.seq.cmp(&left.3.seq))
        });
        found.truncate(asked.limit);
        rows.extend(found);
        rows.sort_by(|left, right| {
            right
                .3
                .at_unix_millis
                .cmp(&left.3.at_unix_millis)
                .then_with(|| right.3.seq.cmp(&left.3.seq))
        });
    }
    let tail = telemetry::Tail {
        records: rows.iter().map(|row| row.3.clone()).collect(),
        bytes_read: ring.0,
        files_read: u32::try_from(ring.1).unwrap_or(u32::MAX),
        malformed: 0,
        hit_scan_cap: ring.2,
        reached_oldest: indexed.next.is_none(),
        partial_tail: false,
        errors: indexed.errors.iter().cloned().chain(ring.3).collect(),
        missing: None,
    };
    let mut keyed = String::new();
    keyed_object(&mut keyed, asked, &rows, &indexed, scan_ring);
    let said = read_note(&indexed);
    (tail, keyed, indexed.next, said)
}

/// The `keyed` object a keyed answer carries: the keys, the rows' half, source
/// and scope in the records' order, the index's postings and counts, the
/// cursor for the next page, and what the ring read when it was asked for.
fn keyed_object(
    out: &mut String,
    asked: &Asked,
    rows: &[(&'static str, &'static str, Scope, telemetry::Record)],
    indexed: &Indexed,
    scan_ring: bool,
) {
    let keys = &asked.keys;
    let text = |value: Option<&str>| value.map_or_else(|| "null".to_owned(), render::json_string);
    let _ = write!(
        out,
        r#","keyed":{{"feed":{},"instrument":{},"day":{},"contract":{},"code":{},"run":{},"target":{},"month":{},"reason":{},"kind":{},"span":{},"matched":{},"rows":["#,
        text(keys.feed.as_deref()),
        text(keys.instrument.as_ref().map(|i| i.asked.as_str())),
        keys.day.map_or_else(
            || "null".to_owned(),
            |d| render::json_string(&d.to_string())
        ),
        text(keys.contract.as_deref()),
        text(keys.code.as_deref()),
        keys.run
            .map_or_else(|| "null".to_owned(), |run| format!("\"{run}\"")),
        text(keys.target.as_deref()),
        text(keys.month.as_deref()),
        text(keys.reason.as_deref()),
        text(keys.kind.as_deref()),
        keys.span.map_or_else(
            || "null".to_owned(),
            |(from, to)| format!(
                r#"{{"from":{},"to":{}}}"#,
                if from == i64::MIN {
                    "null".to_owned()
                } else {
                    from.to_string()
                },
                if to == i64::MAX {
                    "null".to_owned()
                } else {
                    to.to_string()
                }
            )
        ),
        rows.len(),
    );
    for (n, (half, source, scope, record)) in rows.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            r#"{{"seq":{},"half":{},"source":{},"scope":{}}}"#,
            record.seq,
            render::json_string(half),
            render::json_string(source),
            render::json_string(scope.as_str()),
        );
    }
    let _ = write!(
        out,
        r#"],"index":{{"postings":[{}],"max_reads":{MAX_KEYED_READS},"reads":{},"capped":{},"rolled_off":{},"of":{},"of_open":{},"said":{}}},"next":{},"more":{},"cursor_refused":{},"ring":{{"asked":{},"scanned":{scan_ring},"paged":false,"note":{}}}}}"#,
        indexed.postings.join(","),
        indexed.reads,
        indexed.capped,
        indexed.rolled_off,
        indexed.of,
        indexed.of_open,
        render::json_string(&read_note(indexed)),
        indexed
            .next
            .as_deref()
            .map_or_else(|| "null".to_owned(), render::json_string),
        indexed.next.is_some(),
        indexed
            .cursor_refused
            .as_deref()
            .map_or_else(|| "null".to_owned(), render::json_string),
        asked.ring,
        render::json_string(if scan_ring {
            "the newest bytes of each ring were read too, for lines no audit stream holds: those \
             rows are this page's alone and move as the ring rolls"
        } else {
            "the audit index alone: every audited line, paged by `next`. Add ring=1 to a first \
             page for the newest unaudited lines of each ring (warn and info detail)"
        }),
    );
}

/// What a keyed answer read, in words (R3-24): a capped answer says it read
/// "the first 2,000 of N" and that `next` continues it, so a page that
/// stopped at its read budget is never taken for every match there is.
fn read_note(indexed: &Indexed) -> String {
    let of = if indexed.of_open {
        format!("at least {}", indexed.of)
    } else {
        indexed.of.to_string()
    };
    if indexed.capped {
        format!(
            "capped: the first {} of {of} indexed line(s) were read, newest first; ask again with \
             before=<next> to continue from where this page stopped",
            indexed.reads
        )
    } else if indexed.next.is_some() {
        format!(
            "{} of {of} indexed line(s) read for this page; ask again with before=<next> for \
             older matches",
            indexed.reads
        )
    } else {
        format!(
            "{} of {of} indexed line(s) read: every match the streams hold is on this page or an \
             earlier one",
            indexed.reads
        )
    }
}

/// The served sink's audit stream, as JSON; `null` when it keeps none.
fn audit_json_of(health: Option<&telemetry::AuditHealth>) -> String {
    let Some(h) = health else {
        return "null".to_owned();
    };
    format!(
        r#"{{"path":{},"serial":{},"current_bytes":{},"written":{},"dropped":{},"rotations":{},"rotation_failures":{},"index_records":{},"index_failures":{},"unindexed":{},"seeded":{},"gone_waiting":{},"reaped":{},"reap_failures":{},"reaper_behind":{},"reap_error":{},"loud":{},"last_error":{},"max_file_bytes":{},"keep_files":{},"never_rolls":{},"loud_bytes":{}}}"#,
        render::json_string(&h.path.display().to_string()),
        h.serial,
        h.current_bytes,
        h.written,
        h.dropped,
        h.rotations,
        h.rotation_failures,
        h.index_records,
        h.index_failures,
        h.unindexed,
        h.seeded,
        // THE REAPER'S SIDE (D-4838): index generations a roll renamed aside
        // and the reaper has not deleted yet, what it deleted, what it could
        // not, and whether more wait than the stream keeps files.
        h.gone_waiting,
        h.reaped,
        h.reap_failures,
        h.reaper_behind(),
        h.reap_error
            .as_deref()
            .map_or_else(|| "null".to_owned(), render::json_string),
        h.is_loud(),
        h.last_error
            .as_deref()
            .map_or_else(|| "null".to_owned(), render::json_string),
        // THE STATE STREAM HAS NO ROLL BOUND (R3-24): `null`, never the
        // audit stream's, and the size past which it is loud instead.
        if h.never_rolls {
            "null".to_owned()
        } else {
            telemetry::AUDIT_MAX_FILE_BYTES.to_string()
        },
        if h.never_rolls {
            "null".to_owned()
        } else {
            telemetry::AUDIT_KEEP_FILES.to_string()
        },
        h.never_rolls,
        if h.never_rolls {
            telemetry::run_dirs::STATE_LOUD_BYTES.to_string()
        } else {
            "null".to_owned()
        },
    )
}

// ---------------------------------------------------------------------------
// THE GDFL IMPORTER'S OWN REFUSALS, BY KIND AND DAY (DB-025, DB-G09, D-3317)
// ---------------------------------------------------------------------------

/// What the GDFL importer logged at `error`, by `(kind, day)`, read from the
/// `cli` half: the evidence `/imports.json` names beside a journal line that
/// carries no `stage=` (every line written before D-3317).
///
/// # Why from the log
///
/// The journal's own lines from before D-3317 say `failures=1` and nothing
/// else, so a census refusal and a vendor's malformed row closed alike
/// (DB-G09). The importer DID say which: its `refused` event names the day,
/// the file (`about`) and the reason (`why`), and since D-3317 its `stage` and
/// `code`. That event is the record; this reads it rather than re-deciding.
#[derive(Debug, Clone, Default)]
pub(crate) struct DirRefusals {
    /// The refusals, keyed `(day, kind)`: a day sorts as its text does, so a
    /// range of days is a range of the map.
    by_day: std::collections::BTreeMap<(String, String), Vec<telemetry::Record>>,
    /// Per kind, the newest [`RATE_WINDOW`] `day imported` events' stamps, in
    /// milliseconds, oldest first: what the import's pace is measured from.
    imported: std::collections::HashMap<String, std::collections::VecDeque<i64>>,
    /// Ring files read to build this.
    pub(crate) files: u64,
    /// Bytes read to build this, in all.
    pub(crate) bytes: u64,
    /// Lines that would not decode.
    pub(crate) malformed: u64,
    /// Files that could not be read, in their own words.
    pub(crate) errors: Vec<String>,
    /// The `cli` attempts each `(day, kind)`'s importer lines were logged
    /// under, in log order, each once (WP-33c; DB-A41). See [`Attempt`].
    attempts_of: std::collections::BTreeMap<(String, String), Vec<u64>>,
    /// Every `cli` attempt whose `command started` the ring holds.
    attempts: std::collections::BTreeMap<u64, Attempt>,
    /// The attempt the fold is inside: the run of the last `command started`
    /// whose `command finished` it has not read yet.
    current: Option<u64>,
}

/// One `cli` command, as its two lifecycle lines in the `cli` ring name it,
/// and the GDFL importer lines logged between them (WP-33c; DB-A41).
///
/// # Why an importer line can be filed under its attempt
///
/// `cli` stamps only its `command started` / `command finished` pair with the
/// attempt id (`cli::run_with_sink`: "Lifecycle IDs affect these two events
/// only"), so every importer line carries `run: 0`, and `/logs.json?run=4025`
/// found one line of a run that logged thousands. But the `cli` half has ONE
/// writer at a time: its sink holds `logs/cli/events.lock` for the life of the
/// process (`telemetry::Sink::open`, D-1458), and one process runs one
/// command. So every line between an attempt's `command started` and its
/// `command finished` (or the next `command started`, for an attempt that was
/// killed) is that attempt's, by position. That is read here; nothing is
/// guessed from time.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Attempt {
    /// The command word.
    pub(crate) command: String,
    /// `command started`'s stamp, ms.
    pub(crate) started_ms: i64,
    /// `command finished`'s stamp, ms: `None` for an attempt the ring holds no
    /// finish of (killed, or still running).
    pub(crate) finished_ms: Option<i64>,
    /// `command finished`'s `phase`.
    pub(crate) phase: Option<String>,
    /// The importer runs this attempt opened (`run started`'s `run` field).
    pub(crate) importer_runs: Vec<u64>,
    /// Importer lines logged under it.
    pub(crate) lines: u64,
}

impl DirRefusals {
    /// The refusals logged for one kind and day, oldest first.
    pub(crate) fn of(&self, kind: &str, day: &str) -> &[telemetry::Record] {
        self.by_day
            .get(&(day.to_owned(), kind.to_owned()))
            .map_or(&[], Vec::as_slice)
    }

    /// Every `(kind, day)` with a logged refusal in `from..=to`, both
    /// `YYYY-MM-DD`, oldest first. O(log n + days in the range).
    pub(crate) fn days(&self, from: &str, to: &str) -> impl Iterator<Item = (&str, &str)> {
        let low = (from.to_owned(), String::new());
        self.by_day
            .range(low..)
            .take_while(move |((day, _), _)| day.as_str() <= to)
            .map(|((day, kind), _)| (kind.as_str(), day.as_str()))
    }

    /// The import's pace for one kind, from this directory's events alone:
    /// see [`pace_of`].
    #[cfg(test)]
    pub(crate) fn pace(&self, kind: &str) -> Option<(usize, i64, i64)> {
        let stamps = self.imported.get(kind)?;
        let (front, back) = stamps.as_slices();
        if back.is_empty() {
            pace_of(front)
        } else {
            let whole: Vec<i64> = stamps.iter().copied().collect();
            pace_of(&whole)
        }
    }

    /// The stamp of a kind's newest `day imported` event.
    #[cfg(test)]
    pub(crate) fn newest_import(&self, kind: &str) -> Option<i64> {
        self.imported.get(kind)?.back().copied()
    }

    /// The `cli` attempts a kind's day was logged under, oldest first.
    pub(crate) fn attempts_of(&self, kind: &str, day: &str) -> &[u64] {
        self.attempts_of
            .get(&(day.to_owned(), kind.to_owned()))
            .map_or(&[], Vec::as_slice)
    }

    /// One `cli` attempt, by its id.
    pub(crate) fn attempt(&self, id: u64) -> Option<&Attempt> {
        self.attempts.get(&id)
    }

    /// The `cli` attempt that opened importer run `run`, when the ring names
    /// one. O(attempts).
    pub(crate) fn attempt_of_importer_run(&self, run: u64) -> Option<u64> {
        self.attempts
            .iter()
            .find(|(_, attempt)| attempt.importer_runs.contains(&run))
            .map(|(id, _)| *id)
    }

    /// A `cli.lifecycle` line: an attempt opens or closes.
    fn lifecycle(&mut self, record: &telemetry::Record) {
        match record.message.as_str() {
            "command started" if record.run != 0 => {
                self.current = Some(record.run);
                let _replaced = self.attempts.insert(
                    record.run,
                    Attempt {
                        command: text_of(record, "command").unwrap_or("").to_owned(),
                        started_ms: record.at_unix_millis,
                        ..Attempt::default()
                    },
                );
            }
            "command finished" if self.current == Some(record.run) => {
                if let Some(attempt) = self.attempts.get_mut(&record.run) {
                    attempt.finished_ms = Some(record.at_unix_millis);
                    attempt.phase = text_of(record, "phase").map(str::to_owned);
                }
                self.current = None;
            }
            _ => {}
        }
    }

    /// Files an importer line under the attempt the fold is inside.
    fn file_under_attempt(&mut self, record: &telemetry::Record) {
        let Some(id) = self.current else {
            return;
        };
        if let Some(attempt) = self.attempts.get_mut(&id) {
            attempt.lines = attempt.lines.saturating_add(1);
            if record.message == "run started"
                && let Some(run) = record.field("run").and_then(telemetry::OwnedValue::as_u64)
                && !attempt.importer_runs.contains(&run)
            {
                attempt.importer_runs.push(run);
            }
        }
        if let (Some(kind), Some(day)) = (text_of(record, "kind"), text_of(record, "day")) {
            let filed = self
                .attempts_of
                .entry((day.to_owned(), kind.to_owned()))
                .or_default();
            if !filed.contains(&id) {
                filed.push(id);
            }
        }
    }

    /// Folds one line in, when it is a GDFL importer event at `error`, one of
    /// its `day imported` events, or a `cli` lifecycle line (WP-33c), and
    /// files every importer line under its `cli` attempt.
    fn fold(&mut self, line: &[u8]) {
        // CHEAP FIRST: a line that cannot be the importer's or a lifecycle
        // line is not decoded.
        let wanted = br#""target":"gdfl.import""#;
        let lifecycle = br#""target":"cli.lifecycle""#;
        let importer = line.windows(wanted.len()).any(|window| window == wanted);
        if !importer
            && !line
                .windows(lifecycle.len())
                .any(|window| window == lifecycle)
        {
            return;
        }
        let Ok(record) = telemetry::Record::decode(line) else {
            self.malformed = self.malformed.saturating_add(1);
            return;
        };
        if !importer {
            self.lifecycle(&record);
            return;
        }
        self.file_under_attempt(&record);
        if record.level == telemetry::Level::Info && record.message == "day imported" {
            if let Some(kind) = text_of(&record, "kind") {
                let stamps = self.imported.entry(kind.to_owned()).or_default();
                stamps.push_back(record.at_unix_millis);
                if stamps.len() > RATE_WINDOW {
                    let _oldest = stamps.pop_front();
                }
            }
            return;
        }
        if record.level != telemetry::Level::Error {
            return;
        }
        let (Some(kind), Some(day)) = (text_of(&record, "kind"), text_of(&record, "day")) else {
            return;
        };
        self.by_day
            .entry((day.to_owned(), kind.to_owned()))
            .or_default()
            .push(record);
    }
}

/// How many of a kind's newest `day imported` events its pace is measured
/// over: 31 events, 30 gaps.
pub(crate) const RATE_WINDOW: usize = 31;

/// The import's pace, measured from the gaps between `day imported` stamps
/// (milliseconds, oldest first): `(gaps, p50 ms, p90 ms)`, or `None` with
/// fewer than two. A gap that spans a pause between runs is one of them,
/// which is why the median is the figure to read. At most [`RATE_WINDOW`]
/// stamps, so the one sort here is of at most 30 gaps.
fn pace_of(stamps: &[i64]) -> Option<(usize, i64, i64)> {
    let mut gaps: Vec<i64> = stamps
        .iter()
        .zip(stamps.iter().skip(1))
        .map(|(earlier, later)| later.saturating_sub(*earlier).max(0))
        .collect();
    if gaps.is_empty() {
        return None;
    }
    gaps.sort_unstable();
    let at = |per_cent: usize| {
        let rank = gaps
            .len()
            .saturating_mul(per_cent)
            .div_ceil(100)
            .saturating_sub(1);
        gaps.get(rank).copied().unwrap_or(0)
    };
    Some((gaps.len(), at(50), at(90)))
}

/// What one `metadata` call said of a ring file.
///
/// # Why three answers and not two
///
/// The tail used to keep `(0, None)` for every failure, so "no such file"
/// and "you may not look" were one remembered value. A directory first seen
/// absent or empty and then made unreadable compared equal to what was
/// remembered, was never read again, and kept answering "read, nothing
/// held" for as long as the api process lived, while the same directory in
/// the same state read `unreadable` after a restart. The failure is kept
/// apart, in its own words, so the change is seen and the error is the
/// directory's from that call on (D-4862).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Seen {
    /// No such file.
    Absent,
    /// It is there: its length, its modification time, and its device and
    /// inode.
    There {
        /// Bytes.
        len: u64,
        /// When it last changed, where the file system says.
        modified: Option<std::time::SystemTime>,
        /// `(dev, ino)`.
        identity: (u64, u64),
    },
    /// The question itself failed, in the failure's own words.
    Failed(String),
}

impl Seen {
    /// One `metadata` call.
    fn of(path: &std::path::Path) -> Self {
        use std::os::unix::fs::MetadataExt as _;
        match std::fs::metadata(path) {
            Ok(meta) => Self::There {
                len: meta.len(),
                modified: meta.modified().ok(),
                identity: (meta.dev(), meta.ino()),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::Absent,
            Err(e) => Self::Failed(e.to_string()),
        }
    }

    /// Its length; 0 for a file that is not there or could not be asked.
    const fn len(&self) -> u64 {
        match self {
            Self::There { len, .. } => *len,
            Self::Absent | Self::Failed(_) => 0,
        }
    }

    /// The same file in the same readable-or-not state, whatever its length:
    /// what the growing current file is compared by.
    fn same_file(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::There { identity: mine, .. },
                Self::There {
                    identity: theirs, ..
                },
            ) => mine == theirs,
            (mine, theirs) => mine == theirs,
        }
    }
}

/// What [`import_refusals`] keeps between calls for one directory.
#[derive(Debug)]
struct RefusalTail {
    /// Every rolled ring file as last seen, oldest first.
    rolled: Vec<Seen>,
    /// The current file as last seen.
    current: Seen,
    /// The bytes of the current file parsed, through its last newline.
    parsed: u64,
    /// Whether the current file is counted among `built.files`.
    counted: bool,
    /// The failure of the last read of the current file's new bytes, as it
    /// stands in `built.errors`: one slot, replaced and never appended to,
    /// so a file that stays unreadable does not grow the list by one line a
    /// request.
    read_error: Option<String>,
    built: std::sync::Arc<DirRefusals>,
    /// The rolled files not read to their end yet, oldest first (R3-24): a
    /// ring read afresh is read a slice at a time from here, by a request
    /// and by the follower, never whole inside one request.
    queue: std::collections::VecDeque<Queued>,
    /// The current file's length when a read reached its end and what was
    /// left unparsed was a line not yet whole (a writer mid-line, or one
    /// killed there): not behind, so never "not read yet" for ever. A file
    /// that grows past it is read on.
    fragment_end: u64,
}

/// One rolled ring file still to be read: where, how long it was when the
/// ring was read afresh, and how far it has been read.
#[derive(Debug)]
struct Queued {
    path: std::path::PathBuf,
    len: u64,
    at: u64,
}

/// The most ring bytes one page request reads of one directory's importer
/// evidence (R3-24): a fixed slice, so no request waits on a 64 MiB ring;
/// the follower reads the rest ([`follow_refusals`]). Twice the widest line
/// at least, so every slice makes progress.
pub const REQUEST_SLICE_BYTES: u64 = 256 * 1024;

/// A read of a ring this wide with no newline in it holds no line the writer
/// could have made (its widest is under [`telemetry::MAX_LINE_BYTES`]): it is
/// counted as malformed and stepped over, never waited on for ever.
const NO_LINE_BYTES: usize = 2 * telemetry::MAX_LINE_BYTES;

/// The most ring bytes the follower reads of one directory per turn, and how
/// long it waits between turns: one directory's lock is held for one slice at
/// a time, so a request waits at most that long for it.
pub const FOLLOW_SLICE_BYTES: u64 = 1024 * 1024;

/// How long the follower sleeps between turns.
const FOLLOW_PAUSE: std::time::Duration = std::time::Duration::from_millis(100);

/// How many directories the follower keeps following, newest asked last:
/// the five of a store, for a few stores.
const FOLLOWED_DIRS: usize = 32;

/// Every `cli` directory read so far.
fn refusal_tails()
-> &'static std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, RefusalTail>> {
    static TAILS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, RefusalTail>>,
    > = std::sync::OnceLock::new();
    TAILS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// One directory of the importer's evidence, as one read found it.
#[derive(Debug, Clone)]
pub(crate) struct DirEvidence {
    /// The directory's name under `logs/`: one of [`import_dirs`].
    pub(crate) half: &'static str,
    /// Where it is.
    pub(crate) path: std::path::PathBuf,
    /// Whether it could answer.
    pub(crate) state: DirState,
    /// What its ring holds.
    held: std::sync::Arc<DirRefusals>,
}

impl DirEvidence {
    /// Whether a run of `kind` can write here: `logs/cli` (every run before
    /// the directory switch, and every cli verb that is not a run),
    /// `logs/gdfl/store` (a v3 run, the census verbs) and the kind's own
    /// `logs/gdfl/<kind>` (D-4862).
    fn serves(&self, kind: &str) -> bool {
        telemetry::run_dirs::gdfl_evidence(kind).any(|sub| sub == self.half)
    }

    /// The directory as `/imports.json` names it: `logs/<half>`.
    pub(crate) fn name(&self) -> String {
        format!("{}/{}", telemetry::run_dirs::LOGS, self.half)
    }

    /// Ring files read.
    pub(crate) fn files(&self) -> u64 {
        self.held.files
    }

    /// Ring bytes read, in all.
    pub(crate) fn bytes(&self) -> u64 {
        self.held.bytes
    }

    /// Importer lines that would not decode.
    pub(crate) fn malformed(&self) -> u64 {
        self.held.malformed
    }

    /// Ring files that could not be read, in their own words.
    pub(crate) fn errors(&self) -> &[String] {
        &self.held.errors
    }
}

/// The importer's evidence over every directory a GDFL run can log to
/// ([`import_dirs`]), each read through its own tail and merged at the
/// question (D-4862 round 6; IA-11r R5).
///
/// # Why merged, and why by kind
///
/// Until the directory switch every run logs to `logs/cli`. After it a run of
/// a kind logs to `logs/gdfl/<kind>` and a v3 run to `logs/gdfl/store`, and a
/// reader that still looked in `logs/cli` alone would answer `"none"` for
/// every new run: the reader looking in the wrong place that [`cli_half`]'s
/// own doc bans (F78). So a question about a kind reads the three directories
/// a run of that kind can write, and names each one that could not answer
/// ([`Self::unread`]): `"none"` is said only when all three were read.
///
/// # Identity
///
/// [`import_refusals`] returns the SAME `Arc` while no directory's read
/// moved, so a caller keeps an answer against it by pointer, exactly as it
/// did against the one `logs/cli` read (`imports::summary`,
/// `store_route::behind`): the pointer stands for the tuple of the
/// per-directory reads.
#[derive(Debug, Clone, Default)]
pub(crate) struct ImportRefusals {
    /// One per directory of [`import_dirs`], in that order.
    dirs: Vec<DirEvidence>,
}

/// What a kind's evidence for one day is, and where it was read.
#[derive(Debug, Clone, Default)]
pub(crate) struct Logged<'e> {
    /// The refusals, oldest first.
    pub(crate) records: std::borrow::Cow<'e, [telemetry::Record]>,
    /// The directories (`logs/<half>`) that held any of them.
    pub(crate) from: Vec<String>,
}

/// A kind's pace and the directories its newest events were read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pace {
    /// Gaps measured.
    pub(crate) gaps: usize,
    /// The median gap, ms.
    pub(crate) p50: i64,
    /// The 90th-percentile gap, ms.
    pub(crate) p90: i64,
    /// The newest `day imported` stamp, ms.
    pub(crate) newest: Option<i64>,
    /// The directories (`logs/<half>`) whose events are among those measured.
    pub(crate) from: Vec<String>,
}

impl ImportRefusals {
    /// Every directory read, in [`import_dirs`] order.
    pub(crate) fn dirs(&self) -> &[DirEvidence] {
        &self.dirs
    }

    /// The directories a run of `kind` can write, in read order: at most 3.
    fn serving<'s>(&'s self, kind: &'s str) -> impl Iterator<Item = &'s DirEvidence> + 's {
        self.dirs.iter().filter(move |dir| dir.serves(kind))
    }

    /// The directories a run of `kind` can write that could NOT answer, each
    /// as `logs/<half> absent` or `logs/<half> unreadable: <why>`. Empty means
    /// every one of them was read, the only case in which a reader may say
    /// `"none"` (D-4862).
    pub(crate) fn unread(&self, kind: &str) -> Vec<String> {
        self.serving(kind)
            .filter_map(|dir| match &dir.state {
                DirState::Read => None,
                DirState::Absent => Some(format!("{} absent", dir.name())),
                DirState::Unreadable(why) => Some(format!("{} unreadable: {why}", dir.name())),
            })
            .collect()
    }

    /// The refusals logged for one kind and day, oldest first, from every
    /// directory a run of that kind can write. At most three map probes; the
    /// records are borrowed when one directory holds them all.
    pub(crate) fn of(&self, kind: &str, day: &str) -> std::borrow::Cow<'_, [telemetry::Record]> {
        self.logged(kind, day).records
    }

    /// [`Self::of`], with the directories the refusals were read from.
    pub(crate) fn logged(&self, kind: &str, day: &str) -> Logged<'_> {
        let held: Vec<(&DirEvidence, &[telemetry::Record])> = self
            .dirs
            .iter()
            .filter(|dir| dir.serves(kind))
            .map(|dir| (dir, dir.held.of(kind, day)))
            .filter(|(_, records)| !records.is_empty())
            .collect();
        let from = held.iter().map(|(dir, _)| dir.name()).collect();
        let records = match held.as_slice() {
            [] => std::borrow::Cow::Borrowed(&[][..]),
            [(_, only)] => std::borrow::Cow::Borrowed(*only),
            several => {
                let lists: Vec<&[telemetry::Record]> =
                    several.iter().map(|(_, records)| *records).collect();
                std::borrow::Cow::Owned(merged_oldest_first(&lists, |record| record.at_unix_millis))
            }
        };
        Logged { records, from }
    }

    /// Every `(kind, day)` with a logged refusal in `from..=to`, both
    /// `YYYY-MM-DD`, oldest first, each once whichever directories hold it.
    /// O(log n + days in the range) per directory, merged without a sort:
    /// each directory's range is already in `(day, kind)` order.
    pub(crate) fn days(&self, from: &str, to: &str) -> Vec<(&str, &str)> {
        let mut ranges: Vec<_> = self
            .dirs
            .iter()
            .map(|dir| {
                dir.held
                    .days(from, to)
                    .filter(move |(kind, _)| dir.serves(kind))
                    .peekable()
            })
            .collect();
        let mut out: Vec<(&str, &str)> = Vec::new();
        loop {
            // THE SMALLEST HEAD, by (day, kind), over at most five ranges.
            let next = ranges
                .iter_mut()
                .filter_map(|range| range.peek().copied())
                .min_by_key(|(kind, day)| (*day, *kind));
            let Some(next) = next else {
                break;
            };
            for range in &mut ranges {
                if range.peek() == Some(&next) {
                    let _taken = range.next();
                }
            }
            out.push(next);
        }
        out
    }

    /// The import's pace for one kind over its newest [`RATE_WINDOW`] `day
    /// imported` events wherever they were logged, and the directories those
    /// events were read from. `None` with fewer than two events.
    pub(crate) fn pace(&self, kind: &str) -> Option<Pace> {
        let held: Vec<(&DirEvidence, Vec<i64>)> = self
            .serving(kind)
            .filter_map(|dir| {
                let stamps = dir.held.imported.get(kind)?;
                Some((dir, stamps.iter().copied().collect::<Vec<i64>>()))
            })
            .collect();
        let lists: Vec<&[i64]> = held.iter().map(|(_, stamps)| stamps.as_slice()).collect();
        let all = merged_oldest_first(&lists, |stamp| *stamp);
        let window = all.get(all.len().saturating_sub(RATE_WINDOW)..)?;
        let (gaps, p50, p90) = pace_of(window)?;
        let oldest = window.first().copied()?;
        let from = held
            .iter()
            .filter(|(_, stamps)| stamps.last().is_some_and(|newest| *newest >= oldest))
            .map(|(dir, _)| dir.name())
            .collect();
        Some(Pace {
            gaps,
            p50,
            p90,
            newest: window.last().copied(),
            from,
        })
    }

    /// The `cli` attempts a kind's day was logged under, oldest directory
    /// first, each once.
    pub(crate) fn attempts_of(&self, kind: &str, day: &str) -> Vec<u64> {
        let mut out: Vec<u64> = Vec::new();
        for dir in self.serving(kind) {
            for id in dir.held.attempts_of(kind, day) {
                if !out.contains(id) {
                    out.push(*id);
                }
            }
        }
        out
    }

    /// One `cli` attempt, by its id, and the directory (`logs/<half>`) whose
    /// ring holds it.
    pub(crate) fn attempt(&self, id: u64) -> Option<(String, &Attempt)> {
        self.dirs
            .iter()
            .find_map(|dir| dir.held.attempt(id).map(|attempt| (dir.name(), attempt)))
    }

    /// The `cli` attempt that opened importer run `run`, when a ring names
    /// one. O(attempts) per directory.
    pub(crate) fn attempt_of_importer_run(&self, run: u64) -> Option<u64> {
        self.dirs
            .iter()
            .find_map(|dir| dir.held.attempt_of_importer_run(run))
    }

    /// Whether `other` is the same reads: the same directories in the same
    /// states holding the same tails, by pointer.
    fn same_reads(&self, other: &[DirEvidence]) -> bool {
        self.dirs.len() == other.len()
            && self.dirs.iter().zip(other).all(|(mine, theirs)| {
                mine.half == theirs.half
                    && mine.state == theirs.state
                    && std::sync::Arc::ptr_eq(&mine.held, &theirs.held)
            })
    }
}

/// Several lists, each already oldest first, as one list oldest first: at
/// each step the list whose head is oldest gives it up, and an earlier list
/// wins a tie, so the order is the same on every call. O(items × lists), with
/// at most five lists here, and no sort.
fn merged_oldest_first<T: Clone>(lists: &[&[T]], at: impl Fn(&T) -> i64) -> Vec<T> {
    let mut heads = vec![0usize; lists.len()];
    let mut out = Vec::with_capacity(lists.iter().map(|list| list.len()).sum());
    loop {
        let next = lists
            .iter()
            .zip(heads.iter())
            .enumerate()
            .filter_map(|(n, (list, head))| list.get(*head).map(|item| (at(item), n)))
            .min();
        let Some((_, n)) = next else {
            return out;
        };
        if let (Some(list), Some(head)) = (lists.get(n), heads.get_mut(n)) {
            if let Some(item) = list.get(*head) {
                out.push(item.clone());
            }
            *head = head.saturating_add(1);
        }
    }
}

/// Each store's merged evidence as last answered, so the same reads are the
/// same `Arc` again.
fn evidence_kept() -> &'static std::sync::Mutex<
    std::collections::HashMap<std::path::PathBuf, std::sync::Arc<ImportRefusals>>,
> {
    static KEPT: std::sync::OnceLock<
        std::sync::Mutex<
            std::collections::HashMap<std::path::PathBuf, std::sync::Arc<ImportRefusals>>,
        >,
    > = std::sync::OnceLock::new();
    KEPT.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::with_capacity(2)))
}

/// The GDFL importer's evidence under `store_root`: every directory a GDFL
/// run can log to ([`import_dirs`]: `logs/cli`, `logs/gdfl/<kind>` and
/// `logs/gdfl/store`), each named `read`, `absent` or `unreadable`
/// (D-4862 round 6; IA-11r R5; IA-LG-02).
///
/// # Cost
///
/// Per directory at most two `metadata` calls for its state
/// ([`DirState::of`], asked of the disk on every call) and
/// [`refusals_in`]'s tail: its whole ring once per api process, then only
/// the bytes appended since. Five directories, so at most five rings of at
/// most 64 MiB each read once (`docs/06-limits.md` L-IA-37), and a call that
/// finds nothing new is at most 10 `metadata` calls per directory (2 for the
/// state, one per ring file), five tail checks and one pointer comparison
/// each. The merge happens at the question, never here.
///
/// # The state is the disk's, whatever this process read before
///
/// A directory first seen absent or empty and then made unreadable answers
/// `unreadable` with the OS error on the next call, exactly as it does after
/// a restart: the order production meets, since the api starts before any
/// run creates its directory
/// (`a_directory_that_turns_unreadable_is_never_answered_as_read`).
pub(crate) fn import_refusals(store_root: &std::path::Path) -> std::sync::Arc<ImportRefusals> {
    let read: Vec<DirEvidence> = import_dirs()
        .map(|half| {
            let path = store_root.join(telemetry::run_dirs::LOGS).join(half);
            // FOLLOWED FROM HERE ON (R3-24): what this request leaves unread
            // the follower reads, a slice at a time, off every request.
            follow(&path);
            let (held, behind) = refusals_in(&path, REQUEST_SLICE_BYTES);
            // THE DISK'S ANSWER FIRST, then the tail's: a directory that can
            // be searched whose ring file could not be read is unread in
            // part, and cannot vouch for "none" either. Nor can one whose
            // ring is still being read: it is named with what is left.
            let state = match DirState::of(&path) {
                DirState::Read => match held.errors.first() {
                    Some(why) => DirState::Unreadable(why.clone()),
                    None if behind > 0 => DirState::Unreadable(behind_words(behind)),
                    None => DirState::Read,
                },
                other @ (DirState::Absent | DirState::Unreadable(_)) => other,
            };
            DirEvidence {
                half,
                path,
                state,
                held,
            }
        })
        .collect();
    let mut kept = evidence_kept()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(earlier) = kept.get(store_root)
        && earlier.same_reads(&read)
    {
        return std::sync::Arc::clone(earlier);
    }
    let merged = std::sync::Arc::new(ImportRefusals { dirs: read });
    let _replaced = kept.insert(store_root.to_path_buf(), std::sync::Arc::clone(&merged));
    merged
}

/// A directory whose ring is still being read, in words: it is not `read`,
/// and an answer of "none" may not rest on it (R3-24).
fn behind_words(behind: u64) -> String {
    behind_words_with(behind, FOLLOWER_FAILED.get().map(String::as_str))
}

/// [`behind_words`] with the follower's start failure, when it has one.
fn behind_words_with(behind: u64, failed: Option<&str>) -> String {
    match failed {
        None => format!(
            "{behind} byte(s) of its ring are not read yet: the follower reads them a slice at \
             a time, off every request; ask again"
        ),
        Some(why) => format!("{behind} byte(s) of its ring are not read yet, and {why}; ask again"),
    }
}

/// One directory's importer evidence, read through its tail, at most `slice`
/// bytes of it in this call; and how many bytes of its ring are still unread
/// after it (R3-24).
///
/// # Cost
///
/// At most `slice` bytes read, whatever the ring holds: a request passes
/// [`REQUEST_SLICE_BYTES`], the follower [`FOLLOW_SLICE_BYTES`]. A ring read
/// afresh (the first call, a roll, a file replaced, or a file whose
/// `metadata` answers differently: there, not there, or failing, in other
/// words than before) is queued, not read, and the slices work through it
/// oldest file first, then the current file's new bytes. Whole lines only: a
/// slice ends at its last newline and the next starts there. A line is
/// decoded only when it names the importer's target. One `metadata` call per
/// ring file per call (8). `docs/06-limits.md` L-IA-37.
///
/// # Errors are the read's, not the process's
///
/// `errors` names every ring file that could not be asked or read AS OF
/// THIS CALL: a file that turns unreadable is a change the tail sees
/// ([`Seen`]), a file that stays unreadable is named once however many
/// calls find it so, and one that becomes readable again is read and its
/// error dropped.
fn refusals_in(dir: &std::path::Path, slice: u64) -> (std::sync::Arc<DirRefusals>, u64) {
    let paths = telemetry::paths_newest_first(dir, telemetry::DEFAULT_KEEP_FILES);
    let Some((current, rolled_paths)) = paths.split_first() else {
        return (std::sync::Arc::default(), 0);
    };
    // OLDEST FIRST, as the ring was written.
    let rolled: Vec<(&std::path::PathBuf, Seen)> = rolled_paths
        .iter()
        .rev()
        .map(|path| (path, Seen::of(path)))
        .collect();
    let now = Seen::of(current);
    let len = now.len();
    let mut held = refusal_tails()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let tail = held
        .entry(dir.to_path_buf())
        .or_insert_with(RefusalTail::unread);
    let fresh = !tail.rolled.iter().eq(rolled.iter().map(|(_, seen)| seen))
        || !tail.current.same_file(&now)
        || len < tail.parsed;
    if fresh {
        *tail = RefusalTail::queue_ring(&rolled, current, now);
    }
    let left = tail.read_queued(slice);
    if len > tail.parsed && left > 0 && len != tail.fragment_end {
        tail.read_appended(current, len, left);
    }
    let behind = tail.behind(len);
    (std::sync::Arc::clone(&tail.built), behind)
}

/// One directory's evidence read to its end, a request's slice at a time:
/// what a test of the fold reads, where the slicing is not the question.
/// Stops after as many slices as a ring at its ceiling takes.
#[cfg(test)]
fn refusals_whole(dir: &std::path::Path) -> std::sync::Arc<DirRefusals> {
    let most = (64 * 1024 * 1024 / REQUEST_SLICE_BYTES).saturating_add(8);
    let mut held = std::sync::Arc::default();
    for _ in 0..most {
        let (read, behind) = refusals_in(dir, REQUEST_SLICE_BYTES);
        held = read;
        if behind == 0 {
            break;
        }
    }
    held
}

/// The directories the follower reads, oldest asked first, at most
/// [`FOLLOWED_DIRS`]; and whether its thread has been started.
fn followed() -> &'static std::sync::Mutex<(std::collections::VecDeque<std::path::PathBuf>, bool)> {
    static FOLLOWED: std::sync::OnceLock<
        std::sync::Mutex<(std::collections::VecDeque<std::path::PathBuf>, bool)>,
    > = std::sync::OnceLock::new();
    FOLLOWED.get_or_init(|| std::sync::Mutex::new((std::collections::VecDeque::new(), false)))
}

/// Follows `dir` from now on (R3-24): the follower reads what a request
/// left unread there. The first call starts the follower's thread; a thread
/// that cannot start is named once on the importer's evidence log line and
/// leaves every request reading its own slice, which still answers, only
/// more slowly. One lock and a scan of at most [`FOLLOWED_DIRS`] paths.
fn follow(dir: &std::path::Path) {
    let mut held = followed()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (dirs, started) = &mut *held;
    if !dirs.iter().any(|known| known == dir) {
        if dirs.len() >= FOLLOWED_DIRS {
            let _oldest = dirs.pop_front();
        }
        dirs.push_back(dir.to_path_buf());
    }
    if !*started {
        *started = true;
        let failed = start_follower(|| {
            std::thread::Builder::new()
                .name("api-refusal-follower".to_owned())
                .spawn(follow_refusals)
                .map(|_detached| ())
        });
        let _first_failure_kept = failed.map(|why| FOLLOWER_FAILED.set(why));
    }
}

/// Why the follower's thread could not start, once that was found: every
/// later answer of a directory still behind says so (R3-24). Never cleared:
/// a process whose follower did not start reads each ring one request's
/// slice at a time for its whole life, and says that, never "none".
static FOLLOWER_FAILED: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Starts the follower through `spawn`, answering the failure in words, or
/// `None` once it runs. The start is handed in so a test can state one that
/// fails, which no test can cause.
fn start_follower(spawn: impl FnOnce() -> std::io::Result<()>) -> Option<String> {
    spawn().err().map(|e| {
        format!(
            "the follower's thread could not start ({e}), so only each request's slice reads it"
        )
    })
}

/// The follower (R3-24): every [`FOLLOW_PAUSE`], one slice of at most
/// [`FOLLOW_SLICE_BYTES`] of each followed directory that is behind. It
/// holds the tails' lock for one slice of one directory at a time, so a
/// request waits at most that long, and it never ends: the process does.
fn follow_refusals() {
    loop {
        std::thread::sleep(FOLLOW_PAUSE);
        let dirs: Vec<std::path::PathBuf> = followed()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .0
            .iter()
            .cloned()
            .collect();
        for dir in dirs {
            let _read = refusals_in(&dir, FOLLOW_SLICE_BYTES);
        }
    }
}

impl RefusalTail {
    /// A directory nothing has been read from yet: its first question queues
    /// its ring.
    fn unread() -> Self {
        Self {
            rolled: Vec::new(),
            current: Seen::Absent,
            parsed: 0,
            counted: false,
            read_error: None,
            built: std::sync::Arc::default(),
            queue: std::collections::VecDeque::new(),
            fragment_end: 0,
        }
    }

    /// The ring read afresh: every rolled file that is there queued, oldest
    /// first, nothing read yet, and the current file left for
    /// [`Self::read_appended`] to read from its start. A file that could not
    /// be asked about, rolled or current, is named in `errors` and not
    /// queued.
    fn queue_ring(
        rolled: &[(&std::path::PathBuf, Seen)],
        current: &std::path::Path,
        now: Seen,
    ) -> Self {
        let mut built = DirRefusals::default();
        let mut queue = std::collections::VecDeque::new();
        for (path, seen) in rolled {
            match seen {
                Seen::Absent => {}
                Seen::Failed(why) => built.errors.push(format!("{}: {why}", path.display())),
                Seen::There { len, .. } => queue.push_back(Queued {
                    path: (*path).clone(),
                    len: *len,
                    at: 0,
                }),
            }
        }
        if let Seen::Failed(why) = &now {
            built.errors.push(format!("{}: {why}", current.display()));
        }
        Self {
            rolled: rolled.iter().map(|(_, seen)| seen.clone()).collect(),
            current: now,
            parsed: 0,
            counted: false,
            read_error: None,
            built: std::sync::Arc::new(built),
            queue,
            fragment_end: 0,
        }
    }

    /// Bytes of the ring not read yet, the current file being `len` long.
    fn behind(&self, len: u64) -> u64 {
        let current = if len == self.fragment_end {
            0
        } else {
            len.saturating_sub(self.parsed)
        };
        self.queue.iter().fold(current, |sum, file| {
            sum.saturating_add(file.len.saturating_sub(file.at))
        })
    }

    /// Reads the queued rolled files, oldest first, at most `slice` bytes in
    /// all, whole lines only (a file's last fragment is folded at its end, as
    /// a whole read folds it); answers the slice left. A file read to its end
    /// is counted and leaves the queue; one that cannot be read is named and
    /// leaves it too.
    fn read_queued(&mut self, slice: u64) -> u64 {
        use std::io::{Read as _, Seek as _};
        let mut left = slice;
        while left > 0 {
            let Some(file) = self.queue.front_mut() else {
                break;
            };
            let take = left.min(file.len.saturating_sub(file.at));
            let mut bytes = Vec::new();
            let read = std::fs::File::open(&file.path).and_then(|mut opened| {
                opened.seek(std::io::SeekFrom::Start(file.at))?;
                opened.take(take).read_to_end(&mut bytes)
            });
            let built = std::sync::Arc::make_mut(&mut self.built);
            match read {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    let _gone = self.queue.pop_front();
                    continue;
                }
                Err(e) => {
                    built.errors.push(format!("{}: {e}", file.path.display()));
                    let _named = self.queue.pop_front();
                    continue;
                }
                Ok(_) => {}
            }
            let at_end = file
                .at
                .saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX))
                >= file.len
                || bytes.is_empty();
            // WHOLE LINES: through the last newline, or the whole rest at the
            // file's end. A slice with no newline short of the end stops here
            // and the next, wider in all, goes on.
            let used = if at_end {
                bytes.len()
            } else {
                match bytes.iter().rposition(|&byte| byte == b'\n') {
                    Some(last) => last.saturating_add(1),
                    // WIDER THAN ANY LINE AND NO NEWLINE: not a line the
                    // writer could have made. Counted and stepped over,
                    // or the ring would never be read past it.
                    None if bytes.len() >= NO_LINE_BYTES => {
                        built.malformed = built.malformed.saturating_add(1);
                        file.at = file
                            .at
                            .saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
                        built.bytes = built
                            .bytes
                            .saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
                        left = left.saturating_sub(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
                        continue;
                    }
                    None => break,
                }
            };
            for line in bytes
                .get(..used)
                .unwrap_or_default()
                .split(|&byte| byte == b'\n')
            {
                built.fold(line);
            }
            let used = u64::try_from(used).unwrap_or(u64::MAX);
            built.bytes = built.bytes.saturating_add(used);
            file.at = file.at.saturating_add(used);
            left = left.saturating_sub(used.max(1));
            if at_end {
                built.files = built.files.saturating_add(1);
                let _done = self.queue.pop_front();
            }
        }
        left
    }

    /// Reads the current file's bytes from [`Self::parsed`] toward `len`, at
    /// most `slice` of them, and folds its whole lines in. A read that fails
    /// is named in `errors` in one slot, replaced and never appended to; one
    /// that succeeds drops it.
    fn read_appended(&mut self, current: &std::path::Path, len: u64, slice: u64) {
        use std::io::{Read as _, Seek as _};
        let mut bytes = Vec::new();
        let read = std::fs::File::open(current).and_then(|mut file| {
            file.seek(std::io::SeekFrom::Start(self.parsed))?;
            file.take(len.saturating_sub(self.parsed).min(slice))
                .read_to_end(&mut bytes)
        });
        let why = match read {
            Ok(_) => None,
            Err(e) => Some(format!("{}: {e}", current.display())),
        };
        if why.is_some() && why == self.read_error {
            // THE SAME FAILURE AS LAST TIME: the same answer, by pointer.
            return;
        }
        let built = std::sync::Arc::make_mut(&mut self.built);
        if let Some(was) = self.read_error.take() {
            built.errors.retain(|error| *error != was);
        }
        if let Some(why) = why {
            built.errors.push(why.clone());
            self.read_error = Some(why);
            return;
        }
        let mut used = 0usize;
        // WIDER THAN ANY LINE AND NO NEWLINE: stepped over and counted, as
        // [`Self::read_queued`] steps over one.
        if bytes.len() >= NO_LINE_BYTES && !bytes.contains(&b'\n') {
            built.malformed = built.malformed.saturating_add(1);
            used = bytes.len();
            self.parsed = self
                .parsed
                .saturating_add(u64::try_from(used).unwrap_or(u64::MAX));
        }
        if let Some(last) = bytes.iter().rposition(|&byte| byte == b'\n') {
            for line in bytes
                .get(..last)
                .unwrap_or_default()
                .split(|&byte| byte == b'\n')
            {
                built.fold(line);
            }
            used = last.saturating_add(1);
            self.parsed = self
                .parsed
                .saturating_add(u64::try_from(used).unwrap_or(u64::MAX));
        }
        // THE CURRENT FILE IS ONE FILE, counted the first time any of it is
        // read, whether or not the ring was read afresh in this call: a ring
        // first seen empty used to stay at 0 files while its bytes grew.
        built.files = built.files.saturating_add(u64::from(!self.counted));
        self.counted = true;
        // THE BYTES FOLDED, not the bytes read: a line not yet whole is read
        // again with the next slice and counted then, once.
        built.bytes = built
            .bytes
            .saturating_add(u64::try_from(used).unwrap_or(u64::MAX));
        // A READ THAT REACHED THE END and left bytes unparsed left a line not
        // yet whole: the file is read, not behind, until it grows.
        let reached_end = len.saturating_sub(self.parsed)
            <= u64::try_from(bytes.len().saturating_sub(used)).unwrap_or(u64::MAX);
        self.fragment_end = if reached_end && self.parsed < len {
            len
        } else {
            0
        };
    }
}

/// `/logs.json`'s body over directories a caller names: the server's own
/// (when it has one) and every run directory under `store_root`. The handler
/// is this with the process's own two roots, off the async workers.
///
/// Public so `crates/api/benches/ratio/logs_dirs.rs` (C-LG-01) times what
/// the route runs, the way `json_over` exists so a test can.
#[must_use]
pub fn logs_json_over(
    served: Option<&std::path::Path>,
    store_root: &std::path::Path,
    query: &str,
) -> String {
    let halves = halves_of(served.map(std::path::Path::to_path_buf), Some(store_root));
    let asked = asked(query);
    if asked.keys.is_empty() {
        json_across(&halves, &asked, None)
    } else {
        keyed_json_across(&halves, &asked, None)
    }
}

/// How many refusals the importer's evidence holds for one kind and day
/// under `store_root`: `(in the rings, in the audit streams)`. One read of
/// each tail and one keyed probe of each of the kind's directories.
///
/// Public for `crates/api/benches/ratio/logs_dirs.rs` (C-LG-01), which times
/// both readers behind `/imports.json`, `/dayflags.json` and `/store`.
#[must_use]
pub fn import_evidence_over(store_root: &std::path::Path, kind: &str, day: &str) -> (usize, usize) {
    let ring = import_refusals(store_root).of(kind, day).len();
    let audited = audited_import_refusals(store_root, kind, day).records.len();
    (ring, audited)
}

/// `/imports.json`'s body over a store a caller names.
///
/// Public for `crates/api/benches/ratio/logs_dirs.rs` (C-LG-02): the route's
/// module is private to the crate, and this one is where the importer's
/// evidence is read.
///
/// # Errors
///
/// A query the route refuses, or a journal that cannot be read, in the
/// route's own words.
pub fn imports_json_over(store_root: &std::path::Path, query: &str) -> Result<String, String> {
    let asked = crate::server::imports::asked(query)?;
    crate::server::imports::answer(store_root, &asked).map(|body| body.to_string())
}

/// `/dayflags.json`'s body over a store a caller names.
///
/// Public for `crates/api/benches/ratio/logs_dirs.rs`, which times the day
/// band's read of the import journal beside the runs page (C-LG-02): that
/// read still decodes every `run` line per request, which is
/// `docs/06-limits.md` L-IA-39 and is printed there, never gated.
///
/// # Errors
///
/// A query the route refuses, in the route's own words.
pub fn dayflags_json_over(store_root: &std::path::Path, query: &str) -> Result<String, String> {
    crate::server::imports::day_band_json(store_root, query)
}

/// What a keyed read of the importer's audit streams found, and what it
/// could not read.
#[derive(Debug, Clone, Default)]
pub(crate) struct Audited {
    /// The lines, oldest first.
    pub(crate) records: Vec<telemetry::Record>,
    /// The directories (`logs/<half>`) that held any of them.
    pub(crate) from: Vec<String>,
    /// A stream that is there and could not be read, or lines of it that
    /// could not, in the failure's own words. Never dropped: a reader that
    /// found nothing and has words here did not read everything.
    pub(crate) unread: Vec<String>,
    /// Whether a posting held more pointers than one read takes
    /// ([`MAX_KEYED_READS`]), so older lines exist that this did not read.
    pub(crate) capped: bool,
}

/// One probe of `dimension = value` in the audit stream of every directory in
/// `halves`, each line kept when `keep` says so: at most [`MAX_KEYED_READS`]
/// pointers per directory, one positional read each. A directory with no
/// audit stream holds nothing audited and is passed over; one that is there
/// and cannot be read is named in `unread`.
fn audited_in(
    store_root: &std::path::Path,
    halves: &[&'static str],
    dimension: &str,
    value: &str,
    keep: impl Fn(&telemetry::Record) -> bool,
) -> Audited {
    let mut out = Audited::default();
    let mut lists: Vec<Vec<telemetry::Record>> = Vec::with_capacity(halves.len().saturating_mul(2));
    for half in halves {
        let dir = store_root.join("logs").join(half);
        // BOTH DURABLE STREAMS OF THE DIRECTORY (R3-24): a run's lifecycle
        // the refusals rolled out of the audit stream is still in the state
        // stream. A line in both is one line: same `seq`, same time.
        let mut held_here = false;
        let mut seen: std::collections::HashSet<(u64, i64)> =
            std::collections::HashSet::with_capacity(MAX_KEYED_READS.saturating_mul(2));
        for (stream, at) in [
            ("audit stream", dir.clone()),
            ("state stream", telemetry::run_dirs::state_stream_of(&dir)),
        ] {
            let serial = match telemetry::audit_state(&at) {
                Ok(Some(serial)) => serial,
                Ok(None) => continue,
                Err(why) => {
                    out.unread.push(format!("logs/{half} {stream}: {why}"));
                    continue;
                }
            };
            let postings = match telemetry::audit_postings(&at, dimension, value, MAX_KEYED_READS) {
                Ok(postings) => postings,
                Err(why) => {
                    out.unread.push(format!("logs/{half} {stream}: {why}"));
                    continue;
                }
            };
            out.capped |= usize::try_from(postings.total).unwrap_or(usize::MAX) > MAX_KEYED_READS;
            let mut found: Vec<telemetry::Record> = Vec::with_capacity(postings.pointers.len());
            for pointer in &postings.pointers {
                match telemetry::audit_read(&at, serial, pointer) {
                    telemetry::AuditLine::Found(record)
                        if keep(&record) && seen.insert((record.seq, record.at_unix_millis)) =>
                    {
                        found.push(record);
                    }
                    telemetry::AuditLine::Found(_) | telemetry::AuditLine::RolledOff => {}
                    telemetry::AuditLine::Unreadable(why) => {
                        out.unread.push(format!("logs/{half} {stream}: {why}"));
                    }
                }
            }
            if !found.is_empty() {
                // NEWEST FIRST AS READ; oldest first as answered, each
                // stream's list merged with the rest below, never sorted.
                found.reverse();
                held_here = true;
                lists.push(found);
            }
        }
        if held_here {
            out.from.push(format!("logs/{half}"));
        }
    }
    let lists: Vec<&[telemetry::Record]> = lists.iter().map(Vec::as_slice).collect();
    out.records = merged_oldest_first(&lists, |record| record.at_unix_millis);
    out
}

/// The directories a run of `kind` can write, of [`import_dirs`]: the
/// table's own answer (`telemetry::run_dirs::gdfl_evidence`).
fn import_dirs_of(kind: &str) -> Vec<&'static str> {
    telemetry::run_dirs::gdfl_evidence(kind).collect()
}

/// The GDFL importer's refusals of one kind and day that the audit streams
/// hold: one probe of the `day` posting of each directory a run of that kind
/// can write (at most 3), for a day a ring has rolled away. O(pointers of
/// that day), at most [`MAX_KEYED_READS`] per directory (D-4862 round 6).
pub(crate) fn audited_import_refusals(
    store_root: &std::path::Path,
    kind: &str,
    day: &str,
) -> Audited {
    audited_in(store_root, &import_dirs_of(kind), "day", day, |record| {
        record.target == pull::gdfl_import::TARGET
            && record.level == telemetry::Level::Error
            && text_of(record, "kind") == Some(kind)
    })
}

/// The target a GDFL run's lifecycle events carry, from `run claimed` to
/// `finished` (the design's §7.1; D-4862). Every event of it at `info` or
/// above is audited (`telemetry::is_audited`: a target ending `lifecycle`).
pub(crate) const RUN_LIFECYCLE_TARGET: &str = "gdfl.import.lifecycle";

/// One run's lifecycle events, oldest first: the `run` posting of every
/// directory a GDFL run can log to (at most 5 probes, at most
/// [`MAX_KEYED_READS`] pointers each), keyed, never a ring walk. A run from
/// before those events existed has none, and its `cli` attempt answers
/// instead (`imports::run_json`).
pub(crate) fn run_lifecycle(store_root: &std::path::Path, run: u64) -> Audited {
    audited_in(
        store_root,
        &import_dirs().collect::<Vec<_>>(),
        "run",
        &run.to_string(),
        |record| record.target == RUN_LIFECYCLE_TARGET,
    )
}

#[cfg(test)]
mod tests {
    // The same exceptions every test module in this workspace takes.
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::indexing_slicing
    )]

    use super::*;
    use crate::speed::AccessPlan;

    /// audit-20261003 hunt-api-3, D-1583: A FLOOD OF FAILED REQUESTS CANNOT
    /// WIPE THE LOG. 100,000 failed requests inside one window write at most
    /// the ration, and the next window opens with one line counting the rest.
    #[test]
    fn a_flood_of_failed_requests_writes_a_bounded_number_of_lines() {
        let mut ration = Ration::new(FAILED_LINES_PER_WINDOW);
        let start = 1_000_000_u64;
        let mut written = 0_u64;
        for i in 0..100_000_u64 {
            let admit = ration.admit(start + i / 10);
            assert_eq!(admit.summary, None);
            if admit.write {
                written += 1;
            }
        }
        assert_eq!(written, FAILED_LINES_PER_WINDOW);
        let next = ration.admit(start + FAILED_LINE_WINDOW_MS);
        assert!(next.write, "a new window writes again");
        assert_eq!(
            next.summary,
            Some(100_000 - FAILED_LINES_PER_WINDOW),
            "what was held back is counted, not lost"
        );
        // A quiet window that held nothing back says nothing extra.
        let quiet = ration.admit(start + 3 * FAILED_LINE_WINDOW_MS);
        assert_eq!(
            quiet,
            Admit {
                write: true,
                summary: None
            }
        );
        // A clock that stepped backwards opens a window rather than wedging.
        let back = ration.admit(start);
        assert!(back.write);
        // Only another site's requests are rationed.
        let with = |value: &'static str| {
            let mut headers = axum::http::HeaderMap::new();
            headers.insert(
                "sec-fetch-site",
                axum::http::HeaderValue::from_static(value),
            );
            cross_site(&headers)
        };
        assert!(with("cross-site") && with("same-site"));
        assert!(!with("same-origin") && !with("none"));
        assert!(!cross_site(&axum::http::HeaderMap::new()));
    }

    /// audit-20261003 hunt-api-3, D-1552: A SAME-ORIGIN FLOOD IS BOUNDED TOO,
    /// AND COUNTED. 100,000 failed same-origin requests in one window write at
    /// most [`LOCAL_FAILED_LINES_PER_WINDOW`] lines and the next window names
    /// the rest; the two classes draw on separate counts, so a same-origin
    /// flood leaves the cross-site lines intact and the reverse.
    #[test]
    fn a_same_origin_flood_is_bounded_and_counted_separately_from_cross_site() {
        let mut rations = Rations::new();
        let start = 5_000_000_u64;
        let mut local = 0_u64;
        for i in 0..100_000_u64 {
            let admit = rations.admit(false, start + i / 10);
            assert_eq!(admit.summary, None);
            if admit.write {
                local += 1;
            }
        }
        assert_eq!(local, LOCAL_FAILED_LINES_PER_WINDOW);
        // The cross-site lines of the same window were not spent by it.
        let mut cross = 0_u64;
        for i in 0..1_000_u64 {
            if rations.admit(true, start + i).write {
                cross += 1;
            }
        }
        assert_eq!(cross, FAILED_LINES_PER_WINDOW);
        let next_local = rations.admit(false, start + FAILED_LINE_WINDOW_MS);
        assert!(next_local.write);
        assert_eq!(
            next_local.summary,
            Some(100_000 - LOCAL_FAILED_LINES_PER_WINDOW),
            "the same-origin lines held back are counted, not lost"
        );
        let next_cross = rations.admit(true, start + FAILED_LINE_WINDOW_MS);
        assert_eq!(next_cross.summary, Some(1_000 - FAILED_LINES_PER_WINDOW));
        // The summary line names its class and its count.
        assert_eq!(
            suppression_message(false),
            "same-origin failed-request lines suppressed in the previous window"
        );
        assert_eq!(
            suppression_message(true),
            "cross-site failed-request lines suppressed in the previous window"
        );
    }

    /// A sink of our own, in a scratch directory — never the process global.
    ///
    /// `telemetry::install` is a one-shot process singleton, so a test that
    /// used it would decide the answer for every other test in the binary and
    /// would pass or fail on ordering. `Sink::open` takes a config and owns its
    /// directory, which is why [`json_over`] and [`page_over`] take a path.
    ///
    /// # Why the path comes from `crate::scratch::path` and not from here
    ///
    /// It used to be `std::env::temp_dir().join(format!("brutex-logs-{name}"))`
    /// — one fixed directory per test name, in a directory every process on the
    /// machine shares — and the first thing this helper does to it is
    /// `remove_dir_all`. Two `api` test binaries alive at once therefore delete
    /// each other's sink mid-test: two `cargo test` runs, a `cargo llvm-cov`
    /// beside a `cargo test`, or two agents working one checkout are each
    /// enough. The failure it produces is not a clear one — the loser's events
    /// are gone by the time `json_over` reads the directory back, so it surfaces
    /// as a rendered page missing a row that was demonstrably emitted, which
    /// reads as a bug in the renderer and is not one.
    ///
    /// `crate::scratch::path` stamps the process id into the name and adds the
    /// `brutex-` prefix, so no two live processes name the same directory. That
    /// is the failure `crate::scratch` was written for; this call site was
    /// simply missed when the rest of the crate moved over.
    ///
    /// **It does not make `name` optional.** The stamp is the process, not the
    /// test, so two tests in this module passing the same `name` still share one
    /// sink and still delete each other's events — the collision just moves
    /// inside the binary, where it is at least deterministic. Every caller below
    /// passes a name no other one does, and a new caller must too.
    fn sink_in(name: &str) -> (std::path::PathBuf, telemetry::Sink) {
        let dir = crate::scratch::path(&format!("logs-{name}"));
        let _ignored = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let sink = telemetry::Sink::open(
            &telemetry::Config::new(&dir).with_min_level(telemetry::Level::Trace),
        )
        .expect("opens");
        (dir, sink)
    }

    /// THE FIXTURE DIRECTORY NAMES THE PROCESS THAT OPENED IT.
    ///
    /// This is the one property of [`sink_in`] no other test in this module can
    /// observe, because the collision it prevents is between two *processes* and
    /// every test here runs in one. Without the `crate::scratch::path` call the
    /// directory is `brutex-logs-pid-stamp` for every process on the machine, a
    /// name this helper opens by deleting — so the assertion below is the only
    /// thing standing between a future edit and the intermittent, misattributed
    /// failure described on `sink_in`.
    ///
    /// The rest of the naming contract — that two names differ, that the parent
    /// is the temporary directory, that one name asked for twice is one path —
    /// belongs to `crate::scratch` and is asserted there. Restating it here
    /// would be a second copy of a rule that already has one.
    #[test]
    fn a_sink_directory_names_the_process_that_opened_it() {
        let (dir, _sink) = sink_in("pid-stamp");
        let text = dir.to_string_lossy().into_owned();
        assert!(
            text.contains(&std::process::id().to_string()),
            "a fixture two processes can both name is one they can both delete: {text}"
        );
    }

    /// The whole path: emit, roll to disk, read back, render both surfaces.
    #[test]
    fn a_written_event_reaches_the_json_and_the_page() {
        let (dir, sink) = sink_in("roundtrip");
        assert!(
            sink.emit(
                &telemetry::Event::error("pull.member", "did not land")
                    .with("instrument", telemetry::Value::Str("BANKNIFTY"))
                    .with("bars", telemetry::Value::Uint(806))
                    .with("balanced", telemetry::Value::Bool(false)),
            )
            .is_written(),
            "the premise: the sink accepted it"
        );

        let asked = asked("limit=10");
        let json = json_over(&dir, Some(&dir.join(super::CLI_SUBDIR)), &asked, None);
        assert!(json.contains(r#""target":"pull.member""#), "{json}");
        assert!(json.contains(r#""instrument":"BANKNIFTY""#), "{json}");
        assert!(
            json.contains(r#""bars":806"#),
            "a count keeps its JSON type and is NOT stringified: {json}"
        );
        assert!(
            json.contains(r#""balanced":false"#),
            "a flag stays a flag: {json}"
        );
        assert!(
            json.contains(r#""hit_scan_cap":false"#) && json.contains(r#""malformed":0"#),
            "the walk's own limits travel with the answer: {json}"
        );

        let page = page_over(&dir, Some(&dir.join(super::CLI_SUBDIR)), &asked, None);
        assert!(page.contains("BANKNIFTY"), "{page}");
        assert!(
            page.contains("class=\"fault\""),
            "an error row is marked loud, not left to be read as ordinary"
        );
        assert!(
            !page.contains("<script"),
            "CLAUDE.md section 2 permits no script outside web/"
        );
    }

    /// EVERY HUMAN SURFACE READS THE SAME CLOCK, AND IT IS IST.
    ///
    /// This column rendered the line's own UTC text while `/audit` rendered
    /// the same instant through `server::ist_stamp` — two clocks 5h30m apart
    /// on two pages describing one run, with nothing on either saying so.
    ///
    /// The stored value is deliberately NOT converted: `at_unix_millis` stays
    /// UTC on the record, `ts` stays UTC in the file, and `/logs.json` still
    /// answers in epoch milliseconds. One representation on disk, one clock on
    /// every page, conversion exactly once at the boundary.
    #[test]
    fn the_page_renders_ist_and_the_json_stays_utc() {
        // 2026-08-10 04:58:45 UTC is 10:28:45 IST — an instant whose IST hour
        // differs from its UTC hour, so a page that forgot to convert cannot
        // pass by coincidence.
        const UTC_SECS: i64 = 1_786_337_925;
        let (dir, sink) = sink_in("ist");
        assert!(
            sink.emit(&telemetry::Event::error("pull.member", "did not land"))
                .is_written(),
            "the premise: the sink accepted it"
        );

        let asked = asked("limit=10");
        let page = page_over(&dir, Some(&dir.join(super::CLI_SUBDIR)), &asked, None);
        assert!(
            page.contains("When (IST)"),
            "the column says which clock it is: {page}"
        );
        assert!(
            !page.contains("When (UTC)"),
            "and never claims to be the other one"
        );

        // The renderer is the SHARED one, so /audit and /logs cannot drift.
        let expect = crate::server::ist_stamp(UTC_SECS);
        assert!(
            expect.ends_with(" IST"),
            "the shared renderer labels its own output: {expect}"
        );
        assert!(
            expect.starts_with("2026-08-10 10:28:45"),
            "10:28:45 IST is 04:58:45 UTC — the +5:30 is applied once: {expect}"
        );

        // AND THE MACHINE SURFACE IS UNTOUCHED. A consumer parsing this must
        // not have to guess which zone a number is in.
        let json = json_over(&dir, Some(&dir.join(super::CLI_SUBDIR)), &asked, None);
        assert!(
            json.contains(r#""ts":"#),
            "the JSON carries epoch millis, not a rendered string: {json}"
        );
        assert!(
            !json.contains("IST"),
            "and never a zone label, because it is UTC epoch: {json}"
        );
    }

    /// **The PAGE shows the `cli` half — and this test used to prove it about
    /// the JSON.**
    ///
    /// # The defect the test's own name describes, and did not cover
    ///
    /// `both_halves` was wired to `json_over` alone. `page_over` called
    /// `telemetry::tail` on the server's directory and nothing else, so `/logs`
    /// — the surface an operator opens — showed one half while `/logs.json`
    /// showed two. MEASURED on 2026-09-01: the page showed nothing newer than
    /// 31 August while **66 sweep events sat unread** in
    /// `<store>/logs/cli/events.ndjson`, and every `cli` run printed *"The
    /// /logs page walks BOTH halves … so these events appear there. D-0301."*
    ///
    /// This test was named `the_page_shows_the_cli_half…` and asserted on
    /// `json_over`. **A test that guards the wrong function is how a defect
    /// written down twice survives being written down twice**: the name says
    /// the property is covered, the assertion covers the surface that already
    /// had it, and nothing red ever appears. Both surfaces are driven below,
    /// and the page is asserted first because it is the one that was wrong.
    #[test]
    fn the_page_shows_the_cli_half_and_not_only_the_servers() {
        // THE REPRODUCED CASE, printed by `cli` on every single run:
        //   "events -> <store>/logs/cli
        //    The /logs page reads whatever directory `api` resolved, which is
        //    NOT this one."
        //
        // Every event a terminal-run sweep wrote -- every span loaded, every
        // derived threshold, every month found or missing -- was on disk and
        // invisible on the page that exists to show it.
        let (dir, served) = sink_in("both-halves");
        let cli_dir = dir.join(super::CLI_SUBDIR);
        std::fs::create_dir_all(&cli_dir).expect("the cli half");
        let ran = telemetry::Sink::open(
            &telemetry::Config::new(&cli_dir).with_min_level(telemetry::Level::Trace),
        )
        .expect("opens");

        let _ = served.emit(&telemetry::Event::info("api.serve", "the server said this"));
        let _ = ran.emit(&telemetry::Event::info(
            "cli.sweep",
            "the terminal said this",
        ));

        // THE PAGE FIRST, because the page is what was broken. `page_over` is
        // the function `logs_page` calls; asserting on `json_over` here proved
        // a property that surface already had.
        let page = page_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10"),
            None,
        );
        assert!(
            page.contains("the terminal said this"),
            "THE DEFECT: `/logs` walked the server's directory alone, so every \
             event a terminal-run sweep wrote was on disk and invisible on the \
             page that exists to show it: {page}"
        );
        assert!(
            page.contains("the server said this"),
            "and the server's half must not be lost to the merge: {page}"
        );
        assert!(
            page.contains(&dir.join(super::CLI_SUBDIR).display().to_string()),
            "the page names the second directory it read in full, so a reader \
             can tell an empty half from an unread one: {page}"
        );

        let json = json_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10"),
            None,
        );
        assert!(
            json.contains("the server said this"),
            "the server's half must not be lost to the merge: {json}"
        );
        assert!(
            json.contains("the terminal said this"),
            "this is the whole point of the walk: {json}"
        );
    }

    #[test]
    fn a_store_where_nobody_ran_cli_is_an_empty_half_and_not_an_error() {
        // `telemetry::tail` is documented never to fail and to render a missing
        // directory as no records. The merge must not turn that into a failure,
        // and must not report older events existing because a directory that
        // was never created could not reach its own oldest file.
        let (dir, served) = sink_in("no-cli-half");
        assert!(!dir.join(super::CLI_SUBDIR).exists(), "no cli half here");
        let _ = served.emit(&telemetry::Event::info("api.serve", "alone"));

        let json = json_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10"),
            None,
        );
        assert!(json.contains("alone"), "{json}");
        assert!(
            !json.contains(r#""reached_oldest":false"#),
            "an absent half must not claim older events exist: {json}"
        );
    }

    #[test]
    fn the_merged_answer_is_newest_first_across_both_halves() {
        // MERGED ON TIME, NEVER ON SEQUENCE. Each sink numbers its own events
        // from its own run, so `seq` is unique within a directory and
        // meaningless across two -- ordering the union by it would interleave a
        // terminal event from this morning with a server event from last week
        // wherever the two counters happened to collide.
        let (dir, served) = sink_in("merge-order");
        let cli_dir = dir.join(super::CLI_SUBDIR);
        std::fs::create_dir_all(&cli_dir).expect("the cli half");
        let ran = telemetry::Sink::open(
            &telemetry::Config::new(&cli_dir).with_min_level(telemetry::Level::Trace),
        )
        .expect("opens");

        // Interleaved in time, alternating sinks, so a merge that ordered by
        // anything but the clock would show them grouped by source.
        let _ = served.emit(&telemetry::Event::info("api.serve", "first"));
        let _ = ran.emit(&telemetry::Event::info("cli.sweep", "second"));
        let _ = served.emit(&telemetry::Event::info("api.serve", "third"));

        let tail = super::both_halves(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10").query,
        );
        let messages: Vec<&str> = tail
            .records
            .iter()
            .map(|record| record.message.as_str())
            .collect();
        assert_eq!(messages.len(), 3, "every event, once: {messages:?}");

        // NEWEST FIRST, which is the order every other reader in this module
        // promises. Timestamps are milliseconds and three emits can share one,
        // so the assertion is on the ORDERING RELATION rather than on an exact
        // sequence -- a tie broken by `seq` is still deterministic, and pinning
        // a permutation that depends on clock granularity is a flaky test.
        // `windows(2)` yields pairs and the lint denies indexing even where the
        // length is guaranteed, so the pair is destructured rather than indexed
        // -- which is also the spelling that cannot go wrong if the width ever
        // changes.
        for pair in tail.records.windows(2) {
            if let [newer, older] = pair {
                assert!(
                    newer.at_unix_millis >= older.at_unix_millis,
                    "out of order: {messages:?}"
                );
            }
        }
    }

    /// LG-05 (P12-02, D-1791): the completeness flags of two halves merge in
    /// the direction that cannot over-promise. `hit_scan_cap` and
    /// `partial_tail` OR, `reached_oldest` ANDs, and `missing` sums, with a
    /// half that answers `None` contributing nothing rather than a zero. Every
    /// one of the four input combinations of each flag is walked.
    #[test]
    fn completeness_flags_merge_in_the_direction_that_cannot_over_promise() {
        let half = |cap: bool, torn: bool, oldest: bool, missing: Option<u64>| telemetry::Tail {
            hit_scan_cap: cap,
            partial_tail: torn,
            reached_oldest: oldest,
            missing,
            ..telemetry::Tail::default()
        };
        for left in [false, true] {
            for right in [false, true] {
                let both = super::merged(
                    half(left, left, left, None),
                    half(right, right, right, None),
                    10,
                );
                assert_eq!(both.hit_scan_cap, left || right, "cap {left} {right}");
                assert_eq!(both.partial_tail, left || right, "torn {left} {right}");
                assert_eq!(both.reached_oldest, left && right, "oldest {left} {right}");
            }
        }
        for (served, ran, want) in [
            (Some(2), Some(3), Some(5)),
            (Some(2), None, Some(2)),
            (None, Some(3), Some(3)),
            (None, None, None),
            (Some(u64::MAX), Some(1), Some(u64::MAX)),
        ] {
            let both = super::merged(
                half(false, false, true, served),
                half(false, false, true, ran),
                10,
            );
            assert_eq!(both.missing, want, "{served:?} + {ran:?}");
        }
    }

    #[test]
    fn the_limit_bounds_the_union_and_not_each_half() {
        // Each half already honours the limit, so without a truncation on the
        // merged set the page returns up to TWICE what was asked for -- and a
        // caller's `limit` bounds the response, it is not a suggestion.
        let (dir, served) = sink_in("merge-limit");
        let cli_dir = dir.join(super::CLI_SUBDIR);
        std::fs::create_dir_all(&cli_dir).expect("the cli half");
        let ran = telemetry::Sink::open(
            &telemetry::Config::new(&cli_dir).with_min_level(telemetry::Level::Trace),
        )
        .expect("opens");
        for index in 0..6 {
            let _ = served.emit(&telemetry::Event::info("api.serve", "server line"));
            let _ = ran.emit(&telemetry::Event::info("cli.sweep", "cli line"));
            let _ = index;
        }

        let tail = super::both_halves(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=4").query,
        );
        assert_eq!(tail.records.len(), 4, "the union is what the limit bounds");
    }

    #[test]
    fn the_level_filter_is_applied_to_what_is_returned() {
        let (dir, sink) = sink_in("levels");
        let _ = sink.emit(&telemetry::Event::debug("pull.member", "landed"));
        let _ = sink.emit(&telemetry::Event::error("pull.member", "did not land"));

        let all = json_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10"),
            None,
        );
        assert!(
            all.contains("landed") && all.contains("did not land"),
            "{all}"
        );

        let loud = json_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10&level=error"),
            None,
        );
        assert!(loud.contains("did not land"), "{loud}");
        assert!(
            !loud.contains(r#""message":"landed""#),
            "a debug line must not survive an error floor: {loud}"
        );
    }

    /// `limit` is clamped at both ends rather than refused.
    #[test]
    fn a_mangled_limit_is_clamped_and_never_a_refusal() {
        assert_eq!(asked("limit=0").limit, 1, "zero clamps up");
        assert_eq!(
            asked("limit=99999").limit,
            PAGE_LIMIT,
            "past the ceiling clamps down"
        );
        assert_eq!(
            asked("limit=banana").limit,
            50,
            "unreadable takes the default"
        );
        assert_eq!(asked("").limit, 50, "absent takes the default");
        assert_eq!(
            asked("limit=10").query.max_scan_bytes,
            SCAN_BYTES,
            "the byte bound is never taken from the query string"
        );
    }

    /// An empty log says why it is empty rather than looking broken.
    #[test]
    fn an_empty_log_names_the_level_that_would_show_more() {
        let (dir, _sink) = sink_in("empty");
        let page = page_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10"),
            None,
        );
        assert!(
            page.contains("BRUTEX_LOG_LEVEL=debug"),
            "a quiet log is the ordinary state and the page must say what to \
             change to see more: {page}"
        );
    }

    /// **An empty page blames the filter that emptied it, not always the writer.**
    ///
    /// The message used to name `BRUTEX_LOG_LEVEL` whatever the query was. Under
    /// `?target=` that sends the reader to change the WRITE floor and re-run a
    /// job when the cause is a read filter they can clear in one click; under
    /// `?level=warn` it is worse than unhelpful, because a `debug` line written
    /// to the file would still be excluded by the reader's own floor — a remedy
    /// that provably cannot change the answer.
    #[test]
    fn an_empty_page_names_the_filter_in_force_rather_than_always_the_write_floor() {
        let (dir, _sink) = sink_in("empty-filtered");

        // A TARGET FILTER. The write floor is not the cause and is not the cure,
        // but debug lines could still be admitted by the reader, so the write
        // hint is allowed to stay beside the real reason.
        let by_target = page_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10&target=pull.http"),
            None,
        );
        assert!(
            by_target.contains("the filters in force"),
            "the page must say the filters are why it is empty: {by_target}"
        );
        assert!(
            by_target.contains("pull.http"),
            "and name the target it actually filtered on: {by_target}"
        );

        // A LEVEL FILTER ABOVE DEBUG. Raising the write floor cannot help, so the
        // suggestion must not appear at all.
        let by_level = page_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10&level=warn"),
            None,
        );
        assert!(
            by_level.contains("the filters in force"),
            "the level filter is named as the cause: {by_level}"
        );
        assert!(
            !by_level.contains("BRUTEX_LOG_LEVEL"),
            "a `debug` line would STILL be excluded by the reader's own floor, so \
             offering the write floor as the remedy is advice that cannot work: {by_level}"
        );

        // A RUN FILTER.
        let by_run = page_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10&run=42"),
            None,
        );
        assert!(
            by_run.contains("42"),
            "the run narrowed to is named: {by_run}"
        );

        // AND THE UNFILTERED CASE IS UNCHANGED — the original wording still
        // applies when nothing the reader chose is responsible.
        let plain = page_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10"),
            None,
        );
        assert!(
            plain.contains("BRUTEX_LOG_LEVEL=debug") && !plain.contains("the filters in force"),
            "with no filter the write floor IS the thing to change: {plain}"
        );
    }

    /// A `Health` this test owns, so it can describe a sink no test can cause.
    ///
    /// A dropped event needs a full disk or a failing write; a failed roll needs
    /// a rename to fail. Neither is reachable from a unit test, and a branch
    /// that is only reachable in production is a branch nobody has ever seen
    /// render. The fields are `pub`, so the state can be stated directly.
    /// The one note, without indexing — `clippy::indexing_slicing` is denied
    /// workspace-wide and a test is not exempt from it.
    fn note_of(notes: &[String]) -> &str {
        notes.first().map_or("<no note at all>", String::as_str)
    }

    /// **The MISSING note must not send a reader to a banner that cannot answer.**
    ///
    /// `Tail::missing` is computed from the sequence hole in the FILE and so
    /// survives a restart. `Health::dropped` is per-process and starts at zero.
    /// The routine action after a failed run is a restart, so the pair
    /// "5 missing / 0 dropped" is the NORMAL state, not an exotic one — and the
    /// note used to point at the banner for a count the banner did not hold.
    ///
    /// Both branches are pinned. A fix that only ever produced one of them would
    /// pass a test that checked the other.
    #[test]
    fn the_missing_note_points_at_the_banner_only_when_the_banner_knows() {
        let mut lost = walk(Vec::new());
        lost.missing = Some(5);

        // THE RESTART CASE. The file remembers the hole; this process does not.
        let after_restart = walk_notes(&lost, Some(&health(0, 0, None)));
        assert_eq!(
            after_restart.len(),
            1,
            "one flag, one note: {after_restart:?}"
        );
        assert!(
            note_of(&after_restart).contains("counts only THIS process's losses"),
            "a banner reading 0 dropped must not be offered as the source of a \
             count it does not have: {}",
            note_of(&after_restart)
        );
        assert!(
            !note_of(&after_restart).contains("see the sink banner above"),
            "and it must not ALSO say the opposite: {}",
            note_of(&after_restart)
        );

        // THE SAME-PROCESS CASE. The banner really does hold the count.
        let same_process = walk_notes(&lost, Some(&health(5, 0, None)));
        assert_eq!(same_process.len(), 1, "one flag, one note");
        assert!(
            note_of(&same_process).contains("see the sink banner above"),
            "when the banner is loud it IS the place to look: {}",
            note_of(&same_process)
        );

        // A ROTATION FAILURE ALSO MAKES THE BANNER LOUD, so it can answer too —
        // `is_loud` is an OR, and pinning only `dropped` would let a mutant
        // turn it into `dropped > 0` and survive.
        let rolled = walk_notes(&lost, Some(&health(0, 3, None)));
        assert!(
            note_of(&rolled).contains("see the sink banner above"),
            "rotation_failures makes the banner loud as well: {}",
            note_of(&rolled)
        );

        // NO SINK AT ALL. There is no banner to consult, so it must not be cited.
        let none = walk_notes(&lost, None);
        assert!(
            note_of(&none).contains("counts only THIS process's losses"),
            "with no sink there is nothing to point at: {}",
            note_of(&none)
        );

        // And zero missing is still not a note at all.
        assert!(
            walk_notes(&walk(Vec::new()), Some(&health(9, 9, None))).is_empty(),
            "a loud banner does not invent a MISSING note"
        );
    }

    /// CE-41, D-1769: after a failed roll the banner says rotation STOPPED
    /// and the file is growing, never that events were overwritten, and that a
    /// restart resumes it.
    #[test]
    fn a_failed_roll_is_described_as_stopped_rotation_not_overwrite() {
        let page = health_banner(Some(&health(0, 1, Some("rename refused"))));
        assert!(
            page.contains("rotation has stopped for the life of this process"),
            "{page}"
        );
        assert!(
            page.contains("restart the server to resume rotation"),
            "{page}"
        );
        assert!(
            !page.contains("may already have been overwritten"),
            "{page}"
        );
        assert!(page.contains("rename refused"), "{page}");
    }

    /// conc9-3: a held clock and a standing last error each make the banner
    /// and `/logs.json` speak, though no event was lost.
    #[test]
    fn a_held_clock_and_a_standing_error_are_shown() {
        let mut held = health(0, 0, None);
        held.clock_held = 3;
        let page = health_banner(Some(&held));
        assert!(
            page.contains("clock is held") && page.contains("3 event(s)"),
            "{page}"
        );
        assert!(!page.contains("Sink healthy"), "{page}");
        let json = sink_json(Some(&held));
        assert!(
            json.contains(r#""clock_held":3"#) && json.contains(r#""loud":true"#),
            "{json}"
        );

        let erred = health(0, 0, Some("floor resumed ahead of the clock"));
        let page = health_banner(Some(&erred));
        assert!(page.contains("reported a failure"), "{page}");
        assert!(page.contains("floor resumed ahead of the clock"), "{page}");

        let quiet = health(0, 0, None);
        assert!(health_banner(Some(&quiet)).contains("Sink healthy"));
        assert!(sink_json(Some(&quiet)).contains(r#""loud":false"#));
    }

    fn health(dropped: u64, rotation_failures: u64, last_error: Option<&str>) -> telemetry::Health {
        telemetry::Health {
            path: std::path::PathBuf::from("events.ndjson"),
            written: 1_200,
            dropped,
            rotations: 3,
            rotation_failures,
            last_error: last_error.map(str::to_owned),
            current_bytes: 4_096,
            next_seq: 1_201,
            clock_held: 0,
            rationed: 0,
        }
    }

    /// **THE WRITER'S LOSSES REACH BOTH SURFACES.**
    ///
    /// `telemetry::tail` reads what reached the file, so a page built from it
    /// alone renders `0 events` both when nothing happened and when everything
    /// was dropped. `Health::dropped` is the only thing that separates them —
    /// its own doc says "this is the number a page must show" — and before this
    /// test no page showed it and `Health::is_loud` had no caller anywhere.
    #[test]
    fn a_sink_that_lost_events_says_so_on_the_page_and_in_the_json() {
        // Healthy: the quiet case still makes its claim out loud, because an
        // absent banner and a forgotten banner look identical.
        let ok = health(0, 0, None);
        let page = health_banner(Some(&ok));
        assert!(page.contains("Sink healthy"), "{page}");
        assert!(
            page.contains("0 dropped"),
            "the zero is stated, not implied: {page}"
        );
        assert!(
            !page.contains("halt"),
            "a healthy sink is not an alarm: {page}"
        );

        // Lost events: loud, and honest that they are unrecoverable.
        let lost = health(17, 0, None);
        let page = health_banner(Some(&lost));
        assert!(
            page.contains("halt"),
            "a loss must be styled as a fault: {page}"
        );
        assert!(page.contains("17 event(s)"), "the count is named: {page}");
        assert!(
            page.contains("no filter will bring them back"),
            "and it must not read as something the operator can query around: {page}"
        );
        // The ABSENCE is the half that separates `> 0` from `>= 0`. Asserting
        // only that the banner says what it should leaves a guard that fires
        // unconditionally indistinguishable from one that fires correctly.
        assert!(
            !page.contains("roll(s) failed"),
            "and it must not claim a failed roll that did not happen: {page}"
        );

        // A failed roll is the OTHER way the log lies, and is reported apart
        // from a drop — the causes differ and so does the remedy.
        let torn = health(0, 2, Some("events.ndjson.1: Permission denied"));
        let page = health_banner(Some(&torn));
        assert!(
            page.contains("halt") && page.contains("2 roll(s) failed"),
            "{page}"
        );
        assert!(
            !page.contains("were never written"),
            "and it must not claim a drop that did not happen — `dropped > 0` is \
             a different question from `rotation_failures > 0`: {page}"
        );
        assert!(
            page.contains("Permission denied"),
            "the failure's own words: {page}"
        );

        // No sink at all is its own state, and NOT a healthy one: events on the
        // page then belong to an older run.
        let none = health_banner(None);
        assert!(
            none.contains("No sink") && none.contains("older run"),
            "{none}"
        );

        // The JSON carries the same facts, machine-readable, and `null` when
        // there is no sink — never a zeroed object, which would read as healthy.
        assert_eq!(sink_json(None), "null");
        let json = sink_json(Some(&lost));
        assert!(json.contains(r#""dropped":17"#), "{json}");
        assert!(json.contains(r#""loud":true"#), "{json}");
        assert!(json.contains(r#""last_error":null"#), "{json}");
        let json = sink_json(Some(&torn));
        assert!(json.contains(r#""rotation_failures":2"#), "{json}");
        assert!(json.contains("Permission denied"), "{json}");
        assert!(sink_json(Some(&ok)).contains(r#""loud":false"#));
    }

    /// A `Record` this test owns.
    fn rec(seq: u64, level: telemetry::Level, fields: &[(&str, i64)]) -> telemetry::Record {
        telemetry::Record {
            seq,
            run: 0,
            at_unix_millis: 1_767_205_800_000,
            at_utc: "2025-12-31T18:30:00Z".to_owned(),
            level,
            target: "pull.member".to_owned(),
            message: "did not land".to_owned(),
            fields: fields
                .iter()
                .map(|(k, v)| ((*k).to_owned(), telemetry::OwnedValue::Int(*v)))
                .collect(),
            cut: false,
            dropped_fields: 0,
        }
    }

    /// A `Tail` this test owns — see [`json_of`] for why it is stated, not caused.
    fn walk(records: Vec<telemetry::Record>) -> telemetry::Tail {
        telemetry::Tail {
            records,
            bytes_read: 512,
            files_read: 1,
            malformed: 0,
            partial_tail: false,
            hit_scan_cap: false,
            reached_oldest: true,
            errors: Vec::new(),
            // A hand-built walk has no holes by construction.
            missing: Some(0),
        }
    }

    #[test]
    fn durable_invocation_aliases_keep_every_u64_bit_before_json_parsing() {
        let id = cli::operation_audit::ID_BASE + 1;
        let mut record = rec(1, telemetry::Level::Info, &[]);
        record.run = id;
        record
            .fields
            .push(("attempt".to_owned(), telemetry::OwnedValue::Uint(id)));
        let json = json_of(
            &walk(vec![record.clone()]),
            &asked("limit=10"),
            None,
            (std::path::Path::new("/private-test"), None),
        );
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("complete JSON");
        let first = parsed
            .get("records")
            .and_then(|rows| rows.get(0))
            .expect("record");
        assert_eq!(
            first.get("run_key").and_then(serde_json::Value::as_str),
            Some(id.to_string().as_str())
        );
        assert_eq!(
            first.get("run").and_then(serde_json::Value::as_u64),
            Some(id)
        );
        assert_eq!(
            first
                .get("fields")
                .and_then(|fields| fields.get("attempt_key"))
                .and_then(serde_json::Value::as_str),
            Some(id.to_string().as_str())
        );

        record.fields.push((
            "attempt_key".to_owned(),
            telemetry::OwnedValue::Str("41".to_owned()),
        ));
        let json = json_of(
            &walk(vec![record]),
            &asked("limit=10"),
            None,
            (std::path::Path::new("/private-test"), None),
        );
        assert_eq!(
            json.matches("\"attempt_key\"").count(),
            1,
            "the supplied contradictory alias is not overwritten or duplicated"
        );
        assert!(json.contains("\"attempt_key\":\"41\""));
    }

    /// **THE JSON IS WELL-FORMED, NOT MERELY CONTAINING THE RIGHT WORDS.**
    ///
    /// Every `n > 0` in this file is a comma separator, and a `contains` assertion
    /// cannot see one in the wrong place: `{,"a":1}` still contains `"a":1`. The
    /// separators are therefore pinned by exact shape at all three levels — the
    /// record array, the field object, and the error array — which is the only
    /// form of the assertion that a misplaced comma fails.
    #[test]
    fn every_comma_in_the_json_is_pinned_by_shape_and_not_by_substring() {
        let two = walk(vec![
            rec(1, telemetry::Level::Error, &[("bars", 806), ("gaps", 2)]),
            rec(2, telemetry::Level::Info, &[("bars", 12)]),
        ]);
        let json = json_of(
            &two,
            &asked("limit=10"),
            None,
            (std::path::Path::new("/served"), None),
        );

        // Two records: exactly one comma between them, none before or after.
        assert!(json.starts_with(r#"{"records":[{"seq":1,"#), "{json}");
        assert!(
            json.contains(r#"}},{"seq":2,"#),
            "one comma between records: {json}"
        );
        // Two fields: exactly one comma between them, none after the brace.
        assert!(
            json.contains(r#""fields":{"bars":806,"gaps":2}}"#),
            "{json}"
        );
        // One field: no comma at all.
        assert!(json.contains(r#""fields":{"bars":12}}"#), "{json}");
        // Empty: neither a stray comma nor a missing brace.
        assert!(
            json_of(
                &walk(Vec::new()),
                &asked(""),
                None,
                (std::path::Path::new("/served"), None)
            )
            .starts_with(r#"{"records":[],"#),
            "an empty walk is an empty array"
        );

        // The error array is the third separator, and needs two unreadable files
        // — a state no unit test can cause, which is why the walk is stated.
        let mut torn = walk(Vec::new());
        torn.errors = vec!["one.ndjson: bad".to_owned(), "two.ndjson: bad".to_owned()];
        let json = json_of(
            &torn,
            &asked(""),
            None,
            (std::path::Path::new("/served"), None),
        );
        assert!(
            json.contains(r#""errors":["one.ndjson: bad","two.ndjson: bad"]"#),
            "{json}"
        );
        assert!(
            json_of(
                &walk(Vec::new()),
                &asked(""),
                None,
                (std::path::Path::new("/served"), None)
            )
            .contains(r#""errors":[]"#),
            "and no stray comma when there are none"
        );
    }

    /// A filter that could not be read is named in the answer, and the filters
    /// that were applied are echoed, so `?level=eror&run=12x` no longer answers
    /// byte-identically to no filter at all (P1-01-02, D-1765).
    #[test]
    fn an_unreadable_filter_is_named_and_the_applied_ones_are_echoed() {
        let bad = asked("level=eror&run=12x");
        assert_eq!(bad.level, None);
        assert_eq!(bad.run, 0);
        assert_eq!(
            bad.ignored,
            vec![("level", "eror".to_owned()), ("run", "12x".to_owned())]
        );
        // `+` is a space on the wire, so a literal plus arrives as `%2B`.
        for (wire, word) in [("%2B3", "+3"), ("03", "03"), ("3.0", "3.0"), ("-3", "-3")] {
            let one = asked(&format!("run={wire}"));
            assert_eq!(one.run, 0, "{word}");
            assert_eq!(one.ignored, vec![("run", word.to_owned())], "{word}");
        }
        let json = json_of(
            &walk(Vec::new()),
            &bad,
            None,
            (std::path::Path::new("/served"), None),
        );
        assert!(
            json.contains(
                r#""level":null,"target":null,"run":null,"run_key":null,"ignored":[{"param":"level","asked":"eror"},{"param":"run","asked":"12x"}],"sink":"#
            ),
            "{json}"
        );

        let good = asked("level=WARN&target=api.backtest&run=18446744073709551615");
        assert!(good.ignored.is_empty());
        let json = json_of(
            &walk(Vec::new()),
            &good,
            None,
            (std::path::Path::new("/served"), None),
        );
        assert!(
            json.contains(
                r#""level":"warn","target":"api.backtest","run":18446744073709551615,"run_key":"18446744073709551615","ignored":[],"sink":"#
            ),
            "{json}"
        );
        // Zero is "every run" and is not a mistake.
        assert!(asked("run=0").ignored.is_empty());
    }

    /// **EVERY WALK FLAG REACHES THE PAGE, AND EACH ONE ALONE.**
    ///
    /// `walk_notes` had six surviving mutants — including `-> vec![]`, which is
    /// "silently report nothing went wrong" and is precisely the failure
    /// `CLAUDE.md` §4 bans. Each flag is asserted on its own, so a note that
    /// fires for the wrong reason is not covered by a note that fires for the
    /// right one.
    #[test]
    fn each_walk_flag_produces_its_own_note_and_reaches_the_page() {
        assert!(
            walk_notes(&walk(Vec::new()), None).is_empty(),
            "a clean walk says nothing — otherwise every page cries wolf"
        );

        // `only` asserts the count as well as the text, so a flag that raises
        // two notes — or raises one belonging to a different flag — fails here
        // rather than passing on a `contains` that happened to match.
        let only = |tail: &telemetry::Tail| -> String {
            let notes = walk_notes(tail, None);
            assert_eq!(notes.len(), 1, "one flag, one note: {notes:?}");
            notes.into_iter().next().unwrap_or_default()
        };

        let mut capped = walk(Vec::new());
        capped.hit_scan_cap = true;
        assert!(only(&capped).contains("NOT everything there is"));

        let mut older = walk(Vec::new());
        older.reached_oldest = false;
        assert!(only(&older).contains("Older events exist"));

        let mut torn = walk(Vec::new());
        torn.partial_tail = true;
        assert!(only(&torn).contains("mid-line"));

        // `malformed > 0` — the zero case must produce NO note, which is what
        // separates `> 0` from `>= 0`.
        let mut bad = walk(Vec::new());
        bad.malformed = 3;
        assert!(only(&bad).contains("3 line(s)"));
        assert!(
            walk_notes(&walk(Vec::new()), None).is_empty(),
            "zero malformed lines is not a note"
        );

        let mut unreadable = walk(Vec::new());
        unreadable.errors = vec!["events.ndjson.2: Permission denied".to_owned()];
        assert!(only(&unreadable).contains("Permission denied"));

        // All of them at once, and all of them on the page — the `!is_empty()`
        // guard in `page_of` must not drop the block.
        let mut every = walk(vec![rec(1, telemetry::Level::Warn, &[("bars", 1)])]);
        every.hit_scan_cap = true;
        every.reached_oldest = false;
        every.partial_tail = true;
        every.malformed = 3;
        every.errors = vec!["events.ndjson.2: Permission denied".to_owned()];
        assert_eq!(walk_notes(&every, None).len(), 5, "five flags, five notes");
        let page = page_of(
            &every,
            &asked("limit=10"),
            None,
            (std::path::Path::new("/tmp/x"), None),
        );
        for want in [
            "NOT everything there is",
            "Older events exist",
            "mid-line",
            "3 line(s)",
            "Permission denied",
        ] {
            assert!(page.contains(want), "the page dropped {want}: {page}");
        }
        assert!(
            !page_of(
                &walk(Vec::new()),
                &asked(""),
                None,
                (std::path::Path::new("/tmp/x"), None)
            )
            .contains("<ul class=\"notes\">"),
            "and a clean walk renders no note list at all"
        );

        // THE PAIR IS RENDERED AS A PAIR. A page that names one of two
        // directories lets a reader conclude the other was empty, and `None`
        // has to SAY it was never opened rather than say nothing at all.
        let both = page_of(
            &walk(Vec::new()),
            &asked(""),
            None,
            (
                std::path::Path::new("/tmp/served"),
                Some(std::path::Path::new("/tmp/served/cli")),
            ),
        );
        assert!(
            both.contains("<code>/tmp/served</code>")
                && both.contains("<code>/tmp/served/cli</code>"),
            "both halves of the walk are named on the page, and each in its own \
             element — one path that happens to be a prefix of the other must \
             not be able to satisfy this by itself: {both}"
        );
        assert!(
            !both.contains("was NOT read"),
            "and a half that WAS read is not announced as missing: {both}"
        );
        assert!(
            page_of(
                &walk(Vec::new()),
                &asked(""),
                None,
                (std::path::Path::new("/tmp/served"), None)
            )
            .contains("the cli half was NOT read"),
            "an unresolved store root is named, not rendered as an empty half"
        );
    }

    /// The row rendering: field separators, and the two truncation marks.
    #[test]
    fn a_row_separates_its_fields_and_marks_what_was_not_kept() {
        let two = walk(vec![rec(
            1,
            telemetry::Level::Error,
            &[("bars", 806), ("gaps", 2)],
        )]);
        let html = rows_table(&two, &[]);
        assert!(
            html.contains("<b>bars</b>=806 <b>gaps</b>=2"),
            "exactly one space between fields, none leading: {html}"
        );
        assert!(
            rows_table(
                &walk(vec![rec(1, telemetry::Level::Info, &[("bars", 1)])]),
                &[]
            )
            .contains("<td class=\"wide\"><b>bars</b>=1<"),
            "and no leading space when there is only one"
        );

        // `dropped_fields > 0`: the zero case must add no mark, which is what
        // separates it from `>= 0`.
        let mut marked = rec(1, telemetry::Level::Warn, &[("bars", 1)]);
        marked.dropped_fields = 4;
        marked.cut = true;
        let html = rows_table(&walk(vec![marked]), &[]);
        assert!(html.contains("4 field(s) counted, not kept"), "{html}");
        assert!(html.contains("a value was cut at its ceiling"), "{html}");
        assert!(
            !rows_table(&two, &[]).contains("counted, not kept"),
            "no mark when nothing was dropped"
        );
        assert!(!rows_table(&two, &[]).contains("cut at its ceiling"));
    }

    /// The level `<select>` marks the level that was asked for, and only it.
    #[test]
    fn the_level_filter_remembers_what_was_asked_for() {
        let page = page_shell(&asked("level=warn"), "");
        assert!(
            page.contains("<option value=\"warn\" selected>warn</option>"),
            "the asked level is the selected one: {page}"
        );
        assert_eq!(
            page.matches(" selected>").count(),
            1,
            "and it is the only one"
        );
        assert_eq!(
            page_shell(&asked(""), "").matches(" selected>").count(),
            0,
            "no level asked for, nothing selected"
        );
    }

    /// The scan cap is a sixteenth of the window, and that is the claim its own
    /// doc comment makes. Pinned as a RELATION so it cannot be restated as a
    /// tautology: change either side and this fails.
    #[test]
    fn the_scan_cap_is_one_sixteenth_of_the_window_the_sink_keeps() {
        let window = u64::from(telemetry::DEFAULT_KEEP_FILES) * telemetry::DEFAULT_MAX_FILE_BYTES;
        assert_eq!(window, 64 * 1024 * 1024, "8 files x 8 MiB");
        assert_eq!(SCAN_BYTES * 16, window, "the doc says the newest sixteenth");
        assert_eq!(SCAN_BYTES, 4_194_304);
    }

    /// **THE HANDLERS FIND THE INSTALLED SINK.**
    ///
    /// `log_dir` and `sink_health` are the two functions that read the process
    /// global, and [`sink_in`] deliberately avoids installing one — so both had
    /// surviving mutants, including `log_dir -> None`, which would render "no
    /// log" on every request forever.
    ///
    /// # It no longer installs its own
    ///
    /// It used to open `brutex-logs-installed-<pid>` at the DEFAULT floor and
    /// call `telemetry::install` itself. That was correct while it was the only
    /// install in the binary and stopped being correct the moment a second test
    /// needed the global: `install` refuses a second call, so whichever test ran
    /// first decided the floor for the whole process — and at the default `Info`
    /// floor every `Debug` and `Trace` site writes nothing, so a test asserting
    /// against one of them would have been asserting the winner of a race.
    /// [`crate::emitted::sink`] is now the single owner, at `Trace`, and this
    /// test asserts against it. The claims below are unchanged.
    #[test]
    fn the_page_reads_the_directory_and_the_health_off_the_installed_sink() {
        let live = crate::emitted::sink();
        assert_eq!(
            log_dir().as_deref(),
            live.path().parent(),
            "the page must read the directory off the sink, never recompute it"
        );
        let health = sink_health().expect("an installed sink has health");
        assert_eq!(
            health.path,
            live.path(),
            "and the health is that same sink's"
        );
        assert!(
            !health.is_loud(),
            "and the sink every other test shares has lost nothing: {health:?}"
        );
    }

    /// **THE TWO HANDLERS THIS MODULE EXISTS FOR, ACTUALLY EXECUTED.**
    ///
    /// A coverage run on 2026-08-10 put `logs_json` and `logs_page` at **0%** —
    /// every line of both, under both compilations of this crate. Their only
    /// reference anywhere was the route registration in `server.rs`. Every test
    /// above drives `json_over` / `page_over`, which were split out *precisely*
    /// so a test could own the directory — and the split left the handlers, the
    /// part an operator's browser actually reaches, unexecuted.
    ///
    /// What only the handler holds, and what nothing else could have caught:
    /// the `log_dir()` resolution off the installed sink, the `503` when there
    /// is none, the `Content-Type`, and the fact that the query string reaches
    /// `asked` at all.
    ///
    /// Uses [`crate::emitted::sink`] rather than installing, because
    /// `telemetry::install` refuses a second call and this binary already has
    /// an owner for its process global.
    #[tokio::test]
    async fn the_handlers_answer_over_the_installed_sink_and_carry_their_content_type() {
        // Not while a test holds every log-read slot: that test expects 429s.
        let _apart = crate::detail::apart_from_slot_owners().await;
        let sink = crate::emitted::sink();
        assert!(
            sink.emit(
                &telemetry::Event::error("api.handler", "handler probe")
                    .with("probe", telemetry::Value::Str("handler-probe-marker")),
            )
            .is_written(),
            "the premise: this binary has a sink and it accepted a line"
        );

        // /logs.json — status, content type, and a body that PARSES.
        let uri: axum::http::Uri = "/logs.json?limit=50&target=api.handler"
            .parse()
            .expect("a legal uri");
        let (status, headers, body) = logs_json(uri).await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(
            headers[0].1, "application/json; charset=utf-8",
            "a JSON body must not be served as text/plain"
        );
        assert!(
            body.contains("handler-probe-marker"),
            "the query string reached `asked` and the walk found this test's own \
             record — which is the only thing that proves the handler is wired \
             to the installed sink rather than to nothing: {body}"
        );
        assert!(
            body.starts_with(r#"{"records":["#) && body.ends_with('}'),
            "and the document is whole"
        );

        // /logs — the same instant, as a page.
        let uri: axum::http::Uri = "/logs?limit=50&target=api.handler&level=error"
            .parse()
            .expect("a legal uri");
        let (status, page) = logs_page(uri).await;
        assert_eq!(status, axum::http::StatusCode::OK);
        let page = page.0;
        assert!(page.contains("handler-probe-marker"), "{page}");
        assert!(
            page.contains("<title>brutex · logs</title>"),
            "the shell is rendered, not just the rows"
        );
        assert!(
            page.contains("<option value=\"error\" selected>"),
            "and the level the URL asked for is the one the form remembers"
        );
    }

    /// A query string the handler cannot honour is still answered, not refused.
    ///
    /// Clamping rather than a `400` is this module's stated rule — "a log viewer
    /// that answers a mangled bookmark with a 400 is a log viewer an operator
    /// stops reaching for" — and until now that rule was only exercised through
    /// `asked`, never through the door an operator's browser knocks on.
    #[tokio::test]
    async fn a_mangled_query_string_still_gets_an_answer_from_the_handler() {
        let _apart = crate::detail::apart_from_slot_owners().await;
        let _sink = crate::emitted::sink();
        let uri: axum::http::Uri = "/logs.json?limit=-9999999&level=NOPE&target="
            .parse()
            .expect("a legal uri");
        let (status, _headers, body) = logs_json(uri).await;
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "a mangled bookmark is not an error"
        );
        assert!(
            body.contains(r#""limit":50"#),
            "the mangled limit fell back to the default rather than refusing: {body}"
        );

        let uri: axum::http::Uri = "/logs?limit=99999999".parse().expect("a legal uri");
        let page = logs_page(uri).await.1.0;
        assert!(
            page.contains(&format!("value=\"{PAGE_LIMIT}\"")),
            "a limit past the page ceiling is clamped to it, and the form shows \
             the number actually used: {page}"
        );
    }

    /// **THE "AS JSON" LINK REPRODUCES THE PAGE, NOT THE DEFAULT.**
    ///
    /// The link used to carry only the target, and to HTML-escape it. Two
    /// separate wrongs in one expression: an operator who filtered to
    /// `level=error&limit=200` and followed the link got the unfiltered default
    /// 50, and a target containing `&` was entity-escaped into something a
    /// browser sends as a literal `&` — which the server then reads as a
    /// parameter separator, silently truncating the filter.
    ///
    /// This walks the round trip the way a browser does: render the page,
    /// extract the `href`, entity-decode the attribute, parse it back through
    /// the same `asked` the handler uses, and demand the three fields match.
    #[test]
    fn the_json_link_carries_the_whole_filter_and_percent_encodes_it() {
        for raw in [
            "limit=200&level=error&target=pull.member",
            "limit=7&target=pull",
            "level=warn",
            "",
        ] {
            let page_asked = asked(raw);
            let page = page_shell(&page_asked, "");

            let at = page.find("/logs.json").expect("the link is on the page");
            let end = page[at..].find('"').expect("the attribute closes") + at;
            let href = &page[at..end];
            // What a browser actually sends: the attribute, entity-decoded.
            let sent = href
                .replace("&amp;", "&")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&#39;", "'");
            let link_asked = asked(sent.split_once('?').map_or("", |(_, q)| q));

            assert_eq!(link_asked.limit, page_asked.limit, "limit lost: {sent}");
            assert_eq!(link_asked.level, page_asked.level, "level lost: {sent}");
            assert_eq!(link_asked.target, page_asked.target, "target lost: {sent}");
        }
    }

    /// A target carrying the two characters that break a URL survives the link.
    ///
    /// `&` is the parameter separator and `=` the assignment; percent-encoding
    /// is what stops the value being read as structure. HTML escaping — which is
    /// what this did — leaves both intact after the browser decodes the
    /// attribute, so the filter was silently cut at the first `&`.
    #[test]
    fn a_target_holding_a_url_separator_is_percent_encoded_not_entity_escaped() {
        let page_asked = asked("target=a%26b%3Dc&limit=9");
        assert_eq!(page_asked.target, "a&b=c", "the premise: it decoded");

        let link = json_query(&page_asked);
        assert!(
            link.contains("target=a%26b%3Dc"),
            "the separator must be percent-encoded in the link: {link}"
        );
        assert!(
            !link.contains("&amp;"),
            "and never entity-escaped, which a browser would hand back as a \
             literal separator: {link}"
        );
        assert_eq!(
            asked(link.trim_start_matches('?')).target,
            "a&b=c",
            "so the value round-trips whole"
        );
    }

    /// **THE RUN FILTER IS REACHABLE FROM A URL AND FROM THE FORM.**
    ///
    /// `Query::from_run` existed for an hour with no caller on the HTTP
    /// surface — the same "built and unreachable" shape this module's own
    /// header records about `telemetry::tail`, and that D-0081 records about
    /// per-target levels. A filter nobody can ask for is not a feature.
    ///
    /// The round trip matters as much as the parse: an operator who narrows to
    /// one backfill and then follows "as JSON" must not silently get every run
    /// back, which is exactly the defect the level and limit had.
    #[test]
    fn a_run_can_be_asked_for_by_url_and_survives_the_json_link() {
        let narrowed = asked("run=1786197791427&limit=25");
        assert_eq!(narrowed.run, 1_786_197_791_427);
        assert_eq!(
            narrowed.query.run,
            Some(1_786_197_791_427),
            "and it reaches the walk, not just the struct"
        );

        // ZERO AND ABSENT BOTH MEAN EVERY RUN. There is no run zero to ask for
        // — an event outside a backfill omits the key — so the value is free to
        // mean the absence of a filter.
        assert_eq!(asked("").run, 0);
        assert_eq!(asked("").query.run, None, "absent is no filter");
        assert_eq!(asked("run=0").query.run, None, "and zero is no filter");
        assert_eq!(
            asked("run=notanumber").query.run,
            None,
            "a mangled run is no filter, not a refusal — this page clamps"
        );

        // THE ROUND TRIP, the way a browser makes it.
        let page = page_shell(&narrowed, "");
        let at = page.find("/logs.json").expect("the link is on the page");
        let end = page[at..].find('"').expect("the attribute closes") + at;
        let sent = page[at..end].replace("&amp;", "&");
        let back = asked(sent.split_once('?').map_or("", |(_, q)| q));
        assert_eq!(
            back.run, narrowed.run,
            "following the link must not widen the view back to every run: {sent}"
        );
        assert_eq!(back.limit, narrowed.limit);

        // And the form remembers it, so it is reachable without hand-editing a URL.
        assert!(
            page.contains(r#"name="run" value="1786197791427""#),
            "the form shows the run in force: {page}"
        );
        assert!(
            page_shell(&asked(""), "").contains(r#"name="run" value=""#),
            "and is empty when every run is shown"
        );
    }

    /// **THE LOG ROUTES ARE ADMITTED, AND A FULL POOL IS A NAMED REFUSAL.**
    /// log-3, P1-04-02, D-2327. Both handlers ran their two-half walk inline
    /// on a Tokio worker with no admission, so they answered 200 whatever else
    /// was walking. With every log-read slot held, each now answers 429 and
    /// says which pool refused it and its bound, before reading a byte; with
    /// the slots released, the same requests are answered again. A 429 can only
    /// come from `crate::detail::run_log_read`, so this also proves both
    /// handlers go through it.
    #[tokio::test]
    async fn the_log_routes_refuse_by_name_when_every_log_read_slot_is_held() {
        let _sink = crate::emitted::sink();
        let apart = crate::detail::apart_from_slot_owners().await;
        let held = crate::detail::take_every_log_read_slot(&apart);
        assert_eq!(held.len(), 4, "this test owns every log-read slot");

        let uri: axum::http::Uri = "/logs.json?limit=5".parse().expect("a legal uri");
        let (status, headers, body) = logs_json(uri).await;
        assert_eq!(status, axum::http::StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(headers[0].1, "application/json; charset=utf-8");
        assert!(
            body.contains("log read not admitted (Saturated): at most 4"),
            "{body}"
        );
        assert!(!body.contains("records"), "nothing was read: {body}");

        let uri: axum::http::Uri = "/logs?limit=5".parse().expect("a legal uri");
        let (status, page) = logs_page(uri).await;
        assert_eq!(status, axum::http::StatusCode::TOO_MANY_REQUESTS);
        assert!(page.0.contains("<b>Not admitted</b>"), "{}", page.0);
        assert!(page.0.contains("Saturated"), "{}", page.0);
        assert!(page.0.contains("at most 4 log reads"), "{}", page.0);
        assert!(
            page.0.contains("<title>brutex · logs</title>"),
            "the refusal is a whole page with its form"
        );

        drop(held);
        let uri: axum::http::Uri = "/logs.json?limit=5".parse().expect("a legal uri");
        let (status, _headers, body) = logs_json(uri).await;
        assert_eq!(status, axum::http::StatusCode::OK, "{body}");
        let uri: axum::http::Uri = "/logs?limit=5".parse().expect("a legal uri");
        assert_eq!(logs_page(uri).await.0, axum::http::StatusCode::OK);
    }

    // -----------------------------------------------------------------------
    // KEYED READS (D-3341 item 3; DB-023, DB-G88, DB-025)
    // -----------------------------------------------------------------------

    /// The importer's refusal of one index file, as `gdfl.import` writes it:
    /// kind, day, file and reason, and no feed or instrument field.
    fn index_refusal(file: &str, day: &str) -> telemetry::Event<'static> {
        let file: &'static str = Box::leak(file.to_owned().into_boxed_str());
        let day: &'static str = Box::leak(day.to_owned().into_boxed_str());
        telemetry::Event::error("gdfl.import", "refused")
            .with("kind", "indices")
            .with("day", day)
            .with("about", file)
            .with(
                "why",
                "line 9: LTP is not a decimal of at most two decimal places",
            )
    }

    /// The `/logs.json` body of a keyed request over `dir` alone.
    fn keyed(dir: &std::path::Path, query: &str) -> serde_json::Value {
        let asked = asked(query);
        assert!(!asked.keys.is_empty(), "{query} names a key");
        serde_json::from_str(&keyed_json(dir, None, &asked, None)).expect("JSON")
    }

    /// **THE OPERATOR'S CHECK: NIFTY'S REFUSAL ON 2022-03-07 IS FOUND BY
    /// FEED, INSTRUMENT AND DAY THROUGH THE AUDIT INDEX**, and BANKNIFTY's
    /// beside it is not; the GDFL importer names neither its feed nor its
    /// instrument, and the API reads both from its target and its file name
    /// with the importer's own resolver. The ring holds the line too, and it
    /// is shown once.
    #[test]
    fn a_refusal_is_found_by_feed_instrument_and_day_through_the_index() {
        let (dir, sink) = sink_in("keyed-nifty");
        let _ = sink.emit(&index_refusal("NIFTY 50.NSE_IDX.csv", "2022-03-07"));
        let _ = sink.emit(&index_refusal("NIFTY BANK.NSE_IDX.csv", "2022-03-07"));
        let _ = sink.emit(&index_refusal("NIFTY 50.NSE_IDX.csv", "2022-03-08"));
        let _ = sink.emit(
            &telemetry::Event::error("pull.spot", "member did not land")
                .with("instrument", "NIFTY")
                .with("feed", "zerodha")
                .with("month", "2022-03"),
        );

        let got = keyed(&dir, "feed=gdfl&instrument=NSE-INDEX-NIFTY&day=2022-03-07");
        let records = got["records"].as_array().expect("records");
        assert_eq!(records.len(), 1, "{got:#}");
        assert_eq!(records[0]["fields"]["about"], "NIFTY 50.NSE_IDX.csv");
        assert_eq!(got["keyed"]["rows"][0]["source"], "audit");
        assert_eq!(got["keyed"]["rows"][0]["scope"], "event");
        assert_eq!(got["keyed"]["matched"], 1);
        let postings = got["keyed"]["index"]["postings"].as_array().unwrap();
        assert!(
            postings
                .iter()
                .any(|p| p["dimension"] == "day" && p["total"] == 2)
        );

        // BANKNIFTY ON THE SAME DAY IS ITS OWN ANSWER.
        let bank = keyed(&dir, "instrument=NSE-INDEX-BANKNIFTY&day=2022-03-07");
        assert_eq!(bank["records"].as_array().unwrap().len(), 1);
        assert_eq!(
            bank["records"][0]["fields"]["about"],
            "NIFTY BANK.NSE_IDX.csv"
        );

        // ZERODHA'S MONTH EVENT BEARS ON THE DAY BY ITS MONTH, AND ONLY UNDER
        // ZERODHA: GDFL'S ANSWER ABOVE DID NOT CARRY IT.
        let zerodha = keyed(&dir, "feed=zerodha&instrument=NIFTY&day=2022-03-07");
        assert_eq!(
            zerodha["records"].as_array().unwrap().len(),
            1,
            "{zerodha:#}"
        );
        assert_eq!(zerodha["keyed"]["rows"][0]["scope"], "month");

        // AN INSTRUMENT ALONE PROBES ITS OWN POSTING AND THE IMPORTER'S TARGET:
        // both of NIFTY's import refusals and Zerodha's month event, which names
        // NIFTY and no segment that contradicts the index.
        let all_days = keyed(&dir, "instrument=NSE-INDEX-NIFTY");
        assert_eq!(
            all_days["records"].as_array().unwrap().len(),
            3,
            "{all_days:#}"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A LINE WRITTEN BEFORE THE STREAM EXISTED IS FOUND IN THE RING**, with
    /// `source: ring`: the `cli` log of 5 Oct 2026 holds such lines, and the
    /// stream begins only when a build that keeps one next opens that sink.
    #[test]
    fn a_line_older_than_the_stream_is_found_in_the_ring() {
        let dir = crate::scratch::path("logs-keyed-ring");
        let _ignored = std::fs::remove_dir_all(&dir);
        {
            let sink = telemetry::Sink::open(&telemetry::Config::new(&dir).with_audit(false))
                .expect("opens");
            let _ = sink.emit(&index_refusal("NIFTY 50.NSE_IDX.csv", "2022-03-07"));
        }
        // THE INDEX ALONE BY DEFAULT (WP-33c, DB-A44): the half keeps no
        // stream, said by name, and nothing is read from the ring unasked.
        let index_only = keyed(&dir, "feed=gdfl&instrument=NSE-INDEX-NIFTY&day=2022-03-07");
        assert_eq!(index_only["records"].as_array().unwrap().len(), 0);
        assert_eq!(
            index_only["keyed"]["index"]["postings"][0]["state"],
            "no audit stream"
        );
        assert_eq!(
            index_only["keyed"]["index"]["postings"][0]["total"],
            serde_json::Value::Null,
            "an absent stream is never `total: 0` (DB-A46)"
        );
        assert_eq!(index_only["keyed"]["ring"]["scanned"], false);
        assert_eq!(index_only["bytes_read"], 0, "no ring byte was read");
        let got = keyed(
            &dir,
            "feed=gdfl&instrument=NSE-INDEX-NIFTY&day=2022-03-07&ring=1",
        );
        assert_eq!(got["records"].as_array().unwrap().len(), 1, "{got:#}");
        assert_eq!(got["keyed"]["rows"][0]["source"], "ring");
        assert_eq!(got["keyed"]["ring"]["scanned"], true);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A REFUSAL OF A WHOLE DAY BEARS ON EVERY INSTRUMENT OF ITS SEGMENT**,
    /// as `scope: day`; a file of another instrument, an unswept reference
    /// file and another segment do not.
    #[test]
    fn a_whole_day_refusal_bears_on_its_segment_and_a_file_on_its_instrument() {
        let record = |about: &str, kind: &str| {
            telemetry::Record::decode(
                format!(
                    "{{\"seq\":1,\"ts\":\"2026-10-05T03:31:18.967Z\",\"ms\":1791171078967,\
                     \"level\":\"error\",\"target\":\"gdfl.import\",\"msg\":\"refused\",\
                     \"fields\":{{\"kind\":\"{kind}\",\"day\":\"2018-11-07\",\"about\":\"{about}\",\
                     \"why\":\"x\"}}}}"
                )
                .as_bytes(),
            )
            .expect("a line")
        };
        let keys = |instrument: &str| Keys {
            instrument: instrument_of(instrument),
            ..Keys::default()
        };
        let calendar = record("calendar", "indices");
        assert_eq!(bears(&calendar, &keys("NSE-INDEX-NIFTY")), Some(Scope::Day));
        assert_eq!(bears(&calendar, &keys("NIFTY")), Some(Scope::Day));
        assert_eq!(bears(&calendar, &keys("NSE-CASH-RELIANCE")), None);
        let vix = record("INDIA VIX.NSE_IDX.csv", "indices");
        assert_eq!(bears(&vix, &keys("NSE-INDEX-NIFTY")), None);
        let stock = record("RELIANCE.NSE.csv", "stocks");
        assert_eq!(
            bears(&stock, &keys("NSE-CASH-RELIANCE")),
            Some(Scope::Event)
        );
        assert_eq!(bears(&stock, &keys("NSE-INDEX-RELIANCE")), None);
        let day_only = Keys {
            day: crate::server::import_journal::parse_day("2018-11-07"),
            ..Keys::default()
        };
        assert_eq!(bears(&calendar, &day_only), Some(Scope::Event));
        let feed_gdfl = Keys {
            feed: Some("gdfl".to_owned()),
            ..Keys::default()
        };
        assert_eq!(bears(&calendar, &feed_gdfl), Some(Scope::Event));
        let feed_zerodha = Keys {
            feed: Some("zerodha".to_owned()),
            ..Keys::default()
        };
        assert_eq!(bears(&calendar, &feed_zerodha), None);
    }

    /// **AN OPTION REFUSAL IS FOUND BY ITS CONTRACT AND ITS UNDERLYING**: the
    /// ticker is read with the importer's own decoder, so the contract the
    /// store names and the vendor's ticker both find it; the spot index does
    /// not.
    #[test]
    fn an_option_refusal_is_found_by_its_contract_and_its_underlying() {
        let ticker = "BANKNIFTY15NOV1826700CE";
        let day = crate::server::import_journal::parse_day("2018-11-06").expect("a day");
        let decoded = pull::gdfl_nfo::decode_ticker(ticker, day).expect("the importer reads it");
        let record = telemetry::Record::decode(
            format!(
                "{{\"seq\":1,\"ts\":\"2026-10-05T03:31:18.967Z\",\"ms\":1791171078967,\
                 \"level\":\"error\",\"target\":\"gdfl.import\",\"msg\":\"refused\",\
                 \"fields\":{{\"kind\":\"options\",\"day\":\"2018-11-06\",\"about\":\"{ticker}\",\
                 \"why\":\"line 9\"}}}}"
            )
            .as_bytes(),
        )
        .expect("a line");
        let by = |contract: &str| Keys {
            contract: Some(contract.to_owned()),
            ..Keys::default()
        };
        assert_eq!(
            bears(&record, &by(&decoded.contract.to_string())),
            Some(Scope::Event)
        );
        assert_eq!(bears(&record, &by(ticker)), Some(Scope::Event));
        assert_eq!(bears(&record, &by("2018-11-15-2670000-PE")), None);
        let under = |instrument: &str| Keys {
            instrument: instrument_of(instrument),
            ..Keys::default()
        };
        assert_eq!(
            bears(&record, &under("NSE-FNO-BANKNIFTY")),
            Some(Scope::Event)
        );
        assert_eq!(bears(&record, &under("BANKNIFTY")), Some(Scope::Event));
        assert_eq!(bears(&record, &under("NSE-INDEX-BANKNIFTY")), None);
    }

    /// **AN INSTRUMENT IS READ IN EACH FORM A PAGE WRITES**, a symbol with a
    /// hyphen included, and a malformed key is NAMED under `ignored` rather
    /// than read as no filter.
    #[test]
    fn instruments_are_read_in_every_form_and_a_bad_key_is_named() {
        let full = instrument_of("NSE-INDEX-NIFTY").expect("full");
        assert_eq!(
            (
                full.exchange.as_deref(),
                full.segment.as_deref(),
                full.symbol.as_str()
            ),
            (Some("NSE"), Some("INDEX"), "NIFTY")
        );
        let short = instrument_of("NSE-NIFTY").expect("short");
        assert_eq!((short.segment, short.symbol.as_str()), (None, "NIFTY"));
        let hyphen = instrument_of("NSE-CASH-BAJAJ-AUTO").expect("hyphen");
        assert_eq!(hyphen.symbol, "BAJAJ-AUTO");
        assert_eq!(
            instrument_of("NSE-BAJAJ-AUTO").expect("hy").symbol,
            "BAJAJ-AUTO"
        );
        assert_eq!(instrument_of("NIFTY").expect("bare").exchange, None);
        assert!(instrument_of("nifty 50").is_none());

        let read =
            asked("feed=nasdaq&day=2022-3-7&instrument=x%20y&contract=2019-03-28-1150000-CE");
        let named: Vec<&str> = read.ignored.iter().map(|(param, _)| *param).collect();
        assert_eq!(named, ["feed", "instrument", "day"]);
        assert_eq!(read.keys.contract.as_deref(), Some("2019-03-28-1150000-CE"));
        assert!(asked("").keys.is_empty());
    }

    /// **A KEYED ANSWER PAGES THROUGH EVERY MATCH FROM ITS CURSOR, NONE TWICE
    /// AND NONE SKIPPED, AND A LINE WRITTEN BETWEEN PAGES DOES NOT MOVE THEM**
    /// (WP-33c, DB-A44): seven refusals of one day, three a page; a line in two
    /// postings of one half (a day's and its month's) is one row; the last
    /// page hands back no cursor; no ring byte is read.
    #[test]
    fn a_keyed_answer_pages_every_match_from_its_cursor() {
        let (dir, sink) = sink_in("keyed-pages");
        for n in 0..7u32 {
            let _ = sink.emit(&index_refusal("NIFTY 50.NSE_IDX.csv", "2022-03-07").with("n", n));
        }
        let page = |query: &str| keyed(&dir, query);
        let first = page("day=2022-03-07&limit=3");
        let mut seen: Vec<u64> = first["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["fields"]["n"].as_u64().unwrap())
            .collect();
        assert_eq!(seen, [6, 5, 4], "{first:#}");
        assert_eq!(first["keyed"]["more"], true);
        assert_eq!(first["bytes_read"], 0, "no ring byte is read unasked");
        // A LINE WRITTEN NOW IS NEWER THAN EVERY PLACE HANDED OUT.
        let _ = sink.emit(&index_refusal("NIFTY 50.NSE_IDX.csv", "2022-03-07").with("n", 99_u32));
        let mut cursor = first["keyed"]["next"].as_str().unwrap().to_owned();
        loop {
            let next = page(&format!("day=2022-03-07&limit=3&before={cursor}"));
            seen.extend(
                next["records"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| r["fields"]["n"].as_u64().unwrap()),
            );
            let Some(more) = next["keyed"]["next"].as_str() else {
                assert_eq!(next["keyed"]["more"], false);
                assert_eq!(next["reached_oldest"], true);
                break;
            };
            cursor = more.to_owned();
        }
        assert_eq!(
            seen,
            [6, 5, 4, 3, 2, 1, 0],
            "every match, once, newest first"
        );
        // A CURSOR OF ANOTHER QUESTION IS REFUSED BY NAME, NEVER READ AS PAGE 1.
        let wrong = page("day=2022-03-07&limit=3&before=1.2~3.4~5.6");
        assert_eq!(wrong["records"], serde_json::json!([]));
        assert!(
            wrong["keyed"]["cursor_refused"]
                .as_str()
                .unwrap()
                .contains("not a cursor of this request")
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A REFUSAL CODE AND A RUN ARE KEYS OF THE INDEX** (WP-33c; DB-A41,
    /// DB-A47): `code=` finds every refusal that logged it; `run=` with
    /// `durable=1` finds a run's lines in the index, its top-level run or the
    /// numeric `run` field the importer writes; `run=` alone stays the ring's
    /// filter a page polls.
    #[test]
    fn a_refusal_code_and_a_run_are_keys_of_the_index() {
        let (dir, sink) = sink_in("keyed-code-run");
        let _ = sink.emit(
            &index_refusal("NIFTY 50.NSE_IDX.csv", "2020-01-01").with("code", "too_many_entries"),
        );
        let _ = sink.emit(
            &index_refusal("NIFTY 50.NSE_IDX.csv", "2020-01-02").with("code", "price_refused"),
        );
        let _ = sink.emit(
            &telemetry::Event::info("gdfl.import", "run started").with("run", 1_791_270_000_u64),
        );
        let _ = sink.emit_for_run(
            4025,
            &telemetry::Event::info("cli.lifecycle", "command started"),
        );
        let coded = keyed(&dir, "code=too_many_entries");
        assert_eq!(coded["records"].as_array().unwrap().len(), 1, "{coded:#}");
        assert_eq!(coded["records"][0]["fields"]["day"], "2020-01-01");
        assert_eq!(coded["keyed"]["code"], "too_many_entries");
        let importer = keyed(&dir, "run=1791270000&durable=1");
        assert_eq!(
            importer["records"].as_array().unwrap().len(),
            1,
            "{importer:#}"
        );
        assert_eq!(importer["records"][0]["message"], "run started");
        let attempt = keyed(&dir, "run=4025&durable=1");
        assert_eq!(attempt["records"][0]["message"], "command started");
        assert!(
            asked("run=4025").keys.is_empty(),
            "run alone is the ring's filter"
        );
        let lone = asked("durable=1");
        assert!(lone.keys.is_empty());
        assert!(lone.ignored.iter().any(|(param, _)| *param == "durable"));
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A PARAMETER NO FILTER READS IS NAMED, NEVER DROPPED** (WP-33c,
    /// DB-A45): `cause=`, `why=` and any other name come back under
    /// `ignored`, and a cursor with no key to page is named too. (`reason=`
    /// was this test's unknown name until R3-24 made it a key.)
    #[test]
    fn a_parameter_no_filter_reads_is_named_never_dropped() {
        let read = asked("cause=census&why=x&bogus=1&before=1.2");
        let named: Vec<(&str, &str)> = read
            .ignored
            .iter()
            .map(|(param, word)| (*param, word.as_str()))
            .collect();
        assert_eq!(
            named,
            [
                ("before", "1.2"),
                ("unknown parameter", "cause"),
                ("unknown parameter", "why"),
                ("unknown parameter", "bogus"),
            ]
        );
        assert_eq!(unknown_params("limit=1&&level=x&x=1&x=2", &PARAMS), ["x"]);
        assert!(asked("feed=gdfl&ring=1&limit=5").ignored.is_empty());
        assert!(
            asked("ring=yes")
                .ignored
                .iter()
                .any(|(param, word)| *param == "ring" && word == "yes")
        );
    }

    /// **THE PAGE ANSWERS A KEYED QUESTION AS THE JSON DOES** (WP-33c,
    /// DB-A45): `/logs?feed=gdfl&day=...` is no longer `/logs` with its keys
    /// dropped: it shows the day's refusal, the form keeps the keys, and the
    /// JSON link carries them.
    #[test]
    fn the_page_answers_a_keyed_question_as_the_json_does() {
        let (dir, sink) = sink_in("keyed-page");
        let _ = sink.emit(&index_refusal("NIFTY 50.NSE_IDX.csv", "2022-03-07"));
        let _ = sink.emit(&telemetry::Event::error("pull.spot", "member did not land"));
        let plain = page_over(&dir, None, &asked(""), None);
        let keyed_page = page_over(&dir, None, &asked("feed=gdfl&day=2022-03-07"), None);
        assert_ne!(plain, keyed_page);
        assert!(keyed_page.contains("NIFTY 50.NSE_IDX.csv"), "{keyed_page}");
        assert!(!keyed_page.contains("member did not land"), "{keyed_page}");
        assert!(
            keyed_page.contains("value=\"2022-03-07\""),
            "the form keeps the day"
        );
        assert!(
            keyed_page.contains("/logs.json?limit=50&amp;feed=gdfl&amp;day=2022-03-07")
                || keyed_page.contains("/logs.json?limit=50&feed=gdfl&day=2022-03-07"),
            "the JSON link carries the keys: {keyed_page}"
        );
        assert!(keyed_page.contains("durable audit index"), "{keyed_page}");
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **EVERY /db READ IS COUNTED, THE FIRST OF A WINDOW SAMPLED, AND A
    /// WINDOW'S COUNT SAID WHEN IT CLOSES** (WP-33c, DB-A42); a lone sampled
    /// read is no news; the selection kept is the selection's names only.
    #[test]
    fn every_db_read_is_counted_and_the_first_of_a_window_sampled() {
        let mut windows = AccessWindows::default();
        let first = windows.account("/bars.json", false, 10, 1_000);
        assert_eq!(
            first,
            AccessPlan {
                closed: None,
                sample: true
            }
        );
        assert!(!windows.account("/bars.json", false, 30, 2_000).sample);
        assert!(!windows.account("/bars.json", true, 5, 3_000).sample);
        assert!(
            windows.account("/spot.json", false, 1, 3_000).sample,
            "per route"
        );
        let next = windows.account("/bars.json", false, 7, 1_000 + ACCESS_WINDOW_MS);
        assert!(next.sample, "a new window samples again");
        let closed = next.closed.expect("the closed window is said");
        assert_eq!((closed.served, closed.refused, closed.slowest), (2, 1, 30));
        let lone = windows.account("/spot.json", false, 1, 3_000 + ACCESS_WINDOW_MS);
        assert_eq!(lone.closed, None, "one sampled read is no news");
        assert_eq!(
            selection_of("feed=gdfl&instrument=NSE-INDEX-NIFTY&month=2019-03&token=secret&q=x%20y"),
            [
                ("feed", "gdfl".to_owned()),
                ("instrument", "NSE-INDEX-NIFTY".to_owned()),
                ("month", "2019-03".to_owned()),
            ]
        );
        // THE WINDOW /db ASKS: its symbol is kept, so the index files the read
        // under its instrument; the paging names are not a selection.
        assert_eq!(
            selection_of(
                "feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&contract=&timeframe=1s\
                 &from=2022-03&to=2022-03&sort=ts&dir=desc&offset=0&limit=50&extremes=1"
            ),
            [
                ("feed", "gdfl".to_owned()),
                ("symbol", "NIFTY".to_owned()),
                ("segment", "INDEX".to_owned()),
                ("exchange", "NSE".to_owned()),
                ("timeframe", "1s".to_owned()),
                ("from", "2022-03".to_owned()),
                ("to", "2022-03".to_owned()),
            ]
        );
        assert!(is_db_read("/bars.json") && !is_db_read("/pull/spot"));
    }

    /// **A 501 IS A PLACEHOLDER, NOT A FAILURE THE AUDIT STREAM KEEPS**
    /// (WP-33c, DB-A52), and a server error's durable copy has its own ration.
    #[test]
    fn a_501_is_a_warn_and_a_server_errors_durable_copy_is_rationed() {
        use axum::http::StatusCode;
        assert_eq!(
            request_level(StatusCode::NOT_IMPLEMENTED),
            telemetry::Level::Warn
        );
        assert_eq!(
            request_level(StatusCode::INTERNAL_SERVER_ERROR),
            telemetry::Level::Error
        );
        assert_eq!(
            request_level(StatusCode::SERVICE_UNAVAILABLE),
            telemetry::Level::Error
        );
        assert_eq!(request_level(StatusCode::NOT_FOUND), telemetry::Level::Warn);
        assert_eq!(request_level(StatusCode::OK), telemetry::Level::Debug);
        assert!(!telemetry::is_audited(
            request_level(StatusCode::NOT_IMPLEMENTED),
            "api.request",
            "served"
        ));
        let mut ration = Ration::new(AUDITED_ERRORS_PER_WINDOW);
        let admitted = (0..AUDITED_ERRORS_PER_WINDOW + 5)
            .filter(|_| ration.admit(1_000).write)
            .count();
        assert_eq!(admitted as u64, AUDITED_ERRORS_PER_WINDOW);
        assert_eq!(ration.admit(1_000 + FAILED_LINE_WINDOW_MS).summary, Some(5));
    }

    /// **THE IMPORT'S PACE IS THE MEDIAN GAP BETWEEN ITS NEWEST DAYS**, over
    /// at most [`RATE_WINDOW`] events per kind, and none with fewer than two.
    #[test]
    fn the_import_pace_is_the_median_gap_between_its_newest_days() {
        let mut refusals = DirRefusals::default();
        let day = |ms: i64| {
            format!(
                "{{\"seq\":1,\"ts\":\"2026-10-05T03:31:18.967Z\",\"ms\":{ms},\"level\":\"info\",\
                 \"target\":\"gdfl.import\",\"msg\":\"day imported\",\"fields\":{{\"kind\":\"options\",\
                 \"day\":\"2019-11-14\"}}}}"
            )
        };
        refusals.fold(day(1_000).as_bytes());
        assert_eq!(refusals.pace("options"), None);
        // Gaps of 60 s, 59 s, 61 s and one pause of 600 s.
        for at in [61_000, 120_000, 181_000, 781_000] {
            refusals.fold(day(at).as_bytes());
        }
        assert_eq!(refusals.pace("options"), Some((4, 60_000, 600_000)));
        assert_eq!(refusals.newest_import("options"), Some(781_000));
        for n in 0..100 {
            refusals.fold(day(1_000_000 + n * 1_000).as_bytes());
        }
        let (gaps, p50, _) = refusals.pace("options").expect("a pace");
        assert_eq!((gaps, p50), (RATE_WINDOW - 1, 1_000));
        assert_eq!(refusals.pace("stocks"), None);
    }

    /// **THE IMPORTER'S REFUSALS ARE READ ONCE, THEN BY THEIR TAIL**: a line
    /// appended later is found without re-reading the file, and a line that
    /// is not the importer's at `error` is never kept.
    #[test]
    fn the_importers_refusals_are_read_by_their_tail() {
        use std::io::Write as _;
        let dir = crate::scratch::path("logs-import-refusals");
        let _ignored = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        let line = |seq: u32, level: &str, day: &str| {
            format!(
                "{{\"seq\":{seq},\"ts\":\"2026-10-05T03:31:18.967Z\",\"ms\":1791171078967,\
                 \"level\":\"{level}\",\"target\":\"gdfl.import\",\"msg\":\"refused\",\
                 \"fields\":{{\"kind\":\"options\",\"day\":\"{day}\",\"about\":\"census\",\
                 \"why\":\"full\"}}}}\n"
            )
        };
        let current = telemetry::current_path(&dir);
        std::fs::write(
            &current,
            line(1, "error", "2019-11-14") + &line(2, "info", "2019-11-14"),
        )
        .expect("write");
        let first = refusals_whole(&dir);
        assert_eq!(first.of("options", "2019-11-14").len(), 1);
        let bytes = first.bytes;
        std::fs::OpenOptions::new()
            .append(true)
            .open(&current)
            .expect("open")
            .write_all(line(3, "error", "2019-11-15").as_bytes())
            .expect("append");
        let second = refusals_whole(&dir);
        assert_eq!(second.of("options", "2019-11-15").len(), 1);
        assert_eq!(
            second.bytes - bytes,
            line(3, "error", "2019-11-15").len() as u64,
            "only the appended line was read"
        );
        assert!(second.of("options", "2019-11-16").is_empty());
        // A ROLL READS THE RING AGAIN, oldest file first.
        std::fs::rename(&current, telemetry::rotated_path(&dir, 1)).expect("roll");
        std::fs::write(&current, line(4, "error", "2019-11-18")).expect("write");
        let third = refusals_whole(&dir);
        assert_eq!(third.of("options", "2019-11-14").len(), 1);
        assert_eq!(third.of("options", "2019-11-18").len(), 1);
        assert_eq!(third.files, 2);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    // -----------------------------------------------------------------------
    // IA-11r, R5 (D-4862 round 6): EVERY RUN DIRECTORY IS READ, AND ONE THAT
    // CANNOT ANSWER IS NAMED
    // -----------------------------------------------------------------------

    /// A sink on `dir`, made with its parents.
    fn run_sink(dir: &std::path::Path) -> telemetry::Sink {
        std::fs::create_dir_all(dir).expect("a run directory");
        telemetry::Sink::open(&telemetry::Config::new(dir).with_min_level(telemetry::Level::Trace))
            .expect("opens")
    }

    /// **THE CLOSED SET NAMES EVERY IMPORT KIND**: the table's three kind
    /// directories are `ImportKind::ALL`'s names in its order, so a fourth
    /// kind fails here until its directory is a row of
    /// `telemetry::run_dirs`; every importer directory is a run directory;
    /// and with the server's own there are 8 (D-4862, D-4897).
    #[test]
    fn the_run_directories_name_every_import_kind() {
        let kinds: Vec<&str> = pull::gdfl_import::ImportKind::ALL
            .iter()
            .map(|kind| kind.as_str())
            .collect();
        let own: Vec<&str> = telemetry::run_dirs::GDFL_KINDS
            .iter()
            .filter_map(|dir| dir.gdfl_kind)
            .collect();
        assert_eq!(own, kinds, "the table's kinds are the importer's, in order");
        let read: Vec<&str> = run_dirs().collect();
        assert!(import_dirs().all(|dir| read.contains(&dir)));
        assert_eq!(read.len() + 1, 8, "the server's own and these");
        for kind in pull::gdfl_import::ImportKind::ALL {
            let word = kind.as_str();
            assert_eq!(
                import_dirs_of(word),
                [
                    CLI_SUBDIR.to_owned(),
                    format!("gdfl/{word}"),
                    "gdfl/store".to_owned(),
                ],
                "a kind's evidence is read from three directories"
            );
        }
        let root = std::path::Path::new("/zz-no-such-store");
        assert_eq!(halves_of(Some(root.join("logs")), Some(root)).len(), 8);
        assert_eq!(halves_of(None, Some(root)).len(), 7, "no server sink");
        assert!(halves_of(None, None).is_empty());
    }

    /// **THE READERS READ THE WRITERS' TABLE** (D-4897, which continues
    /// D-4862): what `/logs` reads is `logs/`, `logs/cli/` and every row of
    /// `telemetry::run_dirs::ALL`, in the table's order and nothing else;
    /// the importer's evidence is read from a subset of it; and wherever the
    /// table sends a GDFL run of a kind, on either layout, is one of the
    /// directories that kind's evidence is read from. A directory added to
    /// the table is read here without an edit, and one a reader spelt itself
    /// fails the equality.
    #[test]
    fn the_readers_read_the_table_of_run_directories() {
        let root = std::path::Path::new("/zz-no-such-store");
        let read: Vec<std::path::PathBuf> = halves_of(Some(root.join("logs")), Some(root))
            .into_iter()
            .map(|half| half.path)
            .collect();
        let table: Vec<std::path::PathBuf> = [
            root.join(telemetry::run_dirs::LOGS),
            root.join(telemetry::run_dirs::LOGS)
                .join(telemetry::run_dirs::CLI),
        ]
        .into_iter()
        .chain(telemetry::run_dirs::ALL.iter().map(|dir| dir.beneath(root)))
        .collect();
        assert_eq!(read, table);

        // THE EVIDENCE READ names the same directories the walk does.
        let evidence = import_refusals(root);
        let named: Vec<String> = evidence.dirs().iter().map(DirEvidence::name).collect();
        let walked: Vec<String> = import_dirs().map(|dir| format!("logs/{dir}")).collect();
        assert_eq!(named, walked);
        assert_eq!(named.len(), 5);
        for dir in evidence.dirs() {
            assert!(
                read.contains(&dir.path),
                "{} is not a /logs read",
                dir.name()
            );
            assert_eq!(dir.state, DirState::Absent, "{}", dir.name());
        }

        // WHERE A RUN LOGS IS WHERE ITS KIND IS READ, on both layouts.
        for kind in pull::gdfl_import::ImportKind::ALL {
            for layout in [
                telemetry::run_dirs::GdflLayout::V3,
                telemetry::run_dirs::GdflLayout::V4,
            ] {
                let written = telemetry::run_dirs::for_gdfl(kind.as_str(), layout)
                    .expect("every import kind has a run directory");
                assert!(
                    import_dirs_of(kind.as_str()).contains(&written.sub),
                    "a {layout:?} run of {} logs to {}, which its readers do not read",
                    kind.as_str(),
                    written.sub
                );
                assert!(
                    evidence
                        .unread(kind.as_str())
                        .contains(&format!("{} absent", written.name())),
                    "{}",
                    written.name()
                );
            }
        }
    }

    /// Makes `dir` unsearchable for the life of the guard and gives it back
    /// its mode when dropped, so a failed assertion still cleans up.
    struct Locked(std::path::PathBuf);

    impl Locked {
        fn new(dir: &std::path::Path) -> Self {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o000))
                .expect("a directory with no permissions");
            // WHERE THE MODE BITS DO NOT BIND (a process that may read
            // anything) this test cannot make its case, and says so rather
            // than passing on nothing.
            assert!(
                std::fs::metadata(telemetry::current_path(dir))
                    .is_err_and(|e| e.kind() == std::io::ErrorKind::PermissionDenied),
                "mode 000 does not stop this process from searching {}: run the tests as a \
                 user the mode bits bind",
                dir.display()
            );
            Self(dir.to_path_buf())
        }
    }

    impl Drop for Locked {
        fn drop(&mut self) {
            use std::os::unix::fs::PermissionsExt as _;
            let _restored =
                std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
        }
    }

    /// **A DIRECTORY THAT TURNS UNREADABLE IS NEVER ANSWERED AS `read`**,
    /// whatever this process read before (D-4862; IA-LG-02). The api starts
    /// before any run creates its directory, so the order production meets
    /// is: first seen ABSENT, or first seen EMPTY, or first seen HOLDING A
    /// REFUSAL, and only then unreadable. Each of the three read `read` with
    /// no error, and the row said `"none"`, until the process restarted: the
    /// tail remembered "no such file" and "permission denied" as one value.
    /// Each must answer `unreadable` with the OS error on `/imports.json`'s
    /// evidence and on `/logs.json`'s and `/logs`'s `dirs`, name the
    /// directory in a kind's `unread`, and read again once it can be.
    #[test]
    fn a_directory_that_turns_unreadable_is_never_answered_as_read() {
        let root = crate::scratch::path("logs-dir-turns-unreadable");
        let _ignored = std::fs::remove_dir_all(&root);
        let logs = root.join("logs");
        std::fs::create_dir_all(&logs).expect("mkdir");
        let state_of = |half: &str| -> DirState {
            import_refusals(&root)
                .dirs()
                .iter()
                .find(|dir| dir.half == half)
                .unwrap_or_else(|| panic!("{half} is not read"))
                .state
                .clone()
        };
        let denied = |state: &DirState, half: &str| {
            let DirState::Unreadable(why) = state else {
                panic!("{half} reads {state:?}, not unreadable");
            };
            assert!(why.contains("Permission denied"), "{half}: {why}");
        };
        let refusal = "{\"seq\":1,\"ts\":\"2026-10-05T03:31:18.967Z\",\"ms\":1791171078967,\
             \"level\":\"error\",\"target\":\"gdfl.import\",\"msg\":\"refused\",\
             \"fields\":{\"kind\":\"stocks\",\"day\":\"2019-11-13\",\"about\":\"ZZ FIXTURE.csv\",\
             \"why\":\"a fixture\"}}\n";

        // FIRST SEEN: absent, empty, and holding one refusal.
        let absent_first = logs.join("gdfl").join("store");
        let empty_first = logs.join("gdfl").join("options");
        let held_first = logs.join("gdfl").join("stocks");
        std::fs::create_dir_all(&empty_first).expect("mkdir");
        std::fs::create_dir_all(&held_first).expect("mkdir");
        std::fs::write(telemetry::current_path(&held_first), refusal).expect("a ring");
        assert_eq!(state_of("gdfl/store"), DirState::Absent);
        assert_eq!(state_of("gdfl/options"), DirState::Read);
        assert_eq!(state_of("gdfl/stocks"), DirState::Read);
        let before = import_refusals(&root);
        assert_eq!(before.of("stocks", "2019-11-13").len(), 1);
        assert_eq!(
            before.unread("options"),
            ["logs/cli absent", "logs/gdfl/store absent"]
        );

        // THEN UNREADABLE, in the same process.
        std::fs::create_dir_all(&absent_first).expect("mkdir");
        {
            let _a = Locked::new(&absent_first);
            let _b = Locked::new(&empty_first);
            let _c = Locked::new(&held_first);
            for half in ["gdfl/store", "gdfl/options", "gdfl/stocks"] {
                denied(&state_of(half), half);
            }
            let after = import_refusals(&root);
            assert!(
                !std::sync::Arc::ptr_eq(&before, &after),
                "an answer kept against the earlier read is not this one"
            );
            let unread = after.unread("stocks");
            assert_eq!(unread.len(), 3, "{unread:?}");
            assert_eq!(unread[0], "logs/cli absent");
            for (said, dir) in unread[1..]
                .iter()
                .zip(["logs/gdfl/stocks", "logs/gdfl/store"])
            {
                assert!(
                    said.starts_with(&format!("{dir} unreadable: "))
                        && said.contains("Permission denied"),
                    "{said}"
                );
            }
            assert!(
                after.of("stocks", "2019-11-13").is_empty(),
                "what can no longer be read is not answered from memory"
            );
            for dir in after
                .dirs()
                .iter()
                .filter(|dir| dir.half != CLI_SUBDIR && dir.half != "gdfl/indices")
            {
                assert!(!dir.errors().is_empty(), "{} names no error", dir.name());
            }

            // ASKED AGAIN, it is the same answer and the same errors: a
            // directory that stays unreadable does not grow its list.
            let errors = |evidence: &ImportRefusals| -> usize {
                evidence.dirs().iter().map(|dir| dir.errors().len()).sum()
            };
            let again = import_refusals(&root);
            assert!(std::sync::Arc::ptr_eq(&after, &again));
            assert_eq!(errors(&after), errors(&again));
        }

        // READABLE AGAIN: read, the refusal back, the errors gone.
        assert_eq!(state_of("gdfl/store"), DirState::Read);
        assert_eq!(state_of("gdfl/options"), DirState::Read);
        assert_eq!(state_of("gdfl/stocks"), DirState::Read);
        let restored = import_refusals(&root);
        assert_eq!(restored.of("stocks", "2019-11-13").len(), 1);
        assert!(restored.dirs().iter().all(|dir| dir.errors().is_empty()));
        assert_eq!(restored.unread("stocks"), ["logs/cli absent"]);
        let _ignored = std::fs::remove_dir_all(&root);
    }

    /// **`/logs.json` AND `/logs` NAME AN UNREADABLE DIRECTORY TOO, FROM THE
    /// DISK** (D-4862; IA-LG-04): their `dirs` asked `metadata` of the
    /// directory alone, which a directory without search permission answers,
    /// so it read `read` beside the same answer's `errors[]` saying
    /// `Permission denied`. An absent one, an empty one and one holding a
    /// ring each read `unreadable` with the OS error while they are, and
    /// `read` once they are not.
    #[test]
    fn logs_names_a_directory_it_may_not_search() {
        let root = crate::scratch::path("logs-dirs-unsearchable");
        let _ignored = std::fs::remove_dir_all(&root);
        let logs = root.join("logs");
        let locked = ["gdfl/store", "gdfl/options", "gdfl/stocks"];
        for half in locked {
            std::fs::create_dir_all(logs.join(half)).expect("mkdir");
        }
        std::fs::write(telemetry::current_path(&logs.join("gdfl/stocks")), "{}\n").expect("a ring");
        let halves = halves_of(Some(logs.clone()), Some(&root));
        assert!(!dirs_json(&halves).contains("unreadable"));
        {
            let _guards: Vec<Locked> = locked
                .iter()
                .map(|half| Locked::new(&logs.join(half)))
                .collect();
            let body: serde_json::Value =
                serde_json::from_str(&format!("{{{}}}", dirs_json(&halves))).expect("json");
            for half in ["gdfl/store", "gdfl/options", "gdfl/stocks"] {
                let dir = body["dirs"]
                    .as_array()
                    .expect("dirs")
                    .iter()
                    .find(|dir| dir["half"] == half)
                    .unwrap_or_else(|| panic!("{half} is not named"));
                assert_eq!(dir["state"], "unreadable", "{dir:#}");
                assert!(
                    dir["error"]
                        .as_str()
                        .is_some_and(|why| why.contains("Permission denied")),
                    "{dir:#}"
                );
                assert_eq!(DirState::of(&logs.join(half)).word(), "unreadable");
            }
            let note = dirs_note(&halves);
            assert_eq!(
                note.matches("unreadable (Permission denied").count(),
                3,
                "{note}"
            );
            assert!(note.contains("<code>served</code> read;"), "{note}");
        }
        let after = dirs_json(&halves);
        assert!(!after.contains("unreadable"), "{after}");
        assert_eq!(after.matches("\"state\":\"read\"").count(), 4, "{after}");
        let _ignored = std::fs::remove_dir_all(&root);
    }

    /// **A RING FILE THAT CANNOT BE READ IS NAMED ONCE, AND COUNTED WHEN IT
    /// CAN BE.** The directory is searchable and its current file is not
    /// readable: every call used to append the same error again (one line a
    /// request, for ever, and a new answer identity each time). And a ring
    /// first seen empty stayed at 0 files while its bytes grew.
    #[test]
    fn a_ring_file_that_stays_unreadable_is_named_once() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = crate::scratch::path("logs-ring-file-unreadable");
        let _ignored = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        let empty = refusals_whole(&dir);
        assert_eq!((empty.files, empty.bytes), (0, 0));
        let line = "{\"seq\":1,\"ts\":\"2026-10-05T03:31:18.967Z\",\"ms\":1791171078967,\
             \"level\":\"error\",\"target\":\"gdfl.import\",\"msg\":\"refused\",\
             \"fields\":{\"kind\":\"options\",\"day\":\"2019-11-14\",\"about\":\"census\",\
             \"why\":\"a fixture\"}}\n";
        let current = telemetry::current_path(&dir);
        std::fs::write(&current, line).expect("a ring");
        std::fs::set_permissions(&current, std::fs::Permissions::from_mode(0o000)).expect("chmod");
        if std::fs::read(&current).is_ok() {
            let _restored =
                std::fs::set_permissions(&current, std::fs::Permissions::from_mode(0o644));
            panic!("mode 000 does not stop this process from reading: run as a user it binds");
        }
        let first = refusals_whole(&dir);
        let second = refusals_whole(&dir);
        let third = refusals_whole(&dir);
        let _restored = std::fs::set_permissions(&current, std::fs::Permissions::from_mode(0o644));
        assert_eq!(first.errors.len(), 1, "{:?}", first.errors);
        assert!(
            first.errors[0].contains("Permission denied"),
            "{:?}",
            first.errors
        );
        assert_eq!(
            third.errors, first.errors,
            "named once, however often asked"
        );
        assert!(std::sync::Arc::ptr_eq(&second, &third), "and the same read");
        assert_eq!(
            DirState::of(&dir),
            DirState::Read,
            "the directory can be searched"
        );

        // READABLE: read, counted as one file, and the error dropped.
        let readable = refusals_whole(&dir);
        assert!(readable.errors.is_empty(), "{:?}", readable.errors);
        assert_eq!(readable.of("options", "2019-11-14").len(), 1);
        assert_eq!(
            (readable.files, readable.bytes),
            (1, line.len() as u64),
            "a ring first seen empty counts its file when it reads it"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// A store whose server logged to `logs/`, an older run to `logs/cli`
    /// and an indices run (`run claimed`, then a refusal) to
    /// `logs/gdfl/indices`, each through a real sink; `logs/verify` is a
    /// FILE, so `logs/verify/zerodha` is neither there nor absent. Returns
    /// the store's root and the indices run's id.
    fn every_run_directory_store() -> (std::path::PathBuf, u64) {
        let root = crate::scratch::path("logs-every-run-directory");
        let _ignored = std::fs::remove_dir_all(&root);
        let served_dir = root.join("logs");
        let run = 1_791_700_000_u64;
        {
            let served = run_sink(&served_dir);
            let cli = run_sink(&served_dir.join("cli"));
            let indices = run_sink(&served_dir.join("gdfl").join("indices"));
            assert!(
                served
                    .emit(&telemetry::Event::info("api.serve", "the server said this"))
                    .is_written()
            );
            assert!(
                cli.emit(
                    &telemetry::Event::error("gdfl.import", "refused")
                        .with("kind", "options")
                        .with("day", "2019-11-14")
                        .with("run", 4_025_u64)
                )
                .is_written()
            );
            assert!(
                indices
                    .emit(
                        &telemetry::Event::info(RUN_LIFECYCLE_TARGET, "run claimed")
                            .with("run", run)
                            .with("kind", "indices")
                    )
                    .is_written()
            );
            assert!(
                indices
                    .emit(
                        &telemetry::Event::error("gdfl.import", "refused")
                            .with("run", run)
                            .with("kind", "indices")
                            .with("day", "2022-03-07")
                            .with("code", "malformed_row")
                    )
                    .is_written()
            );
        }
        // A DIRECTORY THAT CANNOT BE READ: `logs/verify` is a file, so
        // `logs/verify/zerodha` is neither there nor absent.
        std::fs::write(served_dir.join("verify"), b"not a directory").expect("a file");
        (root, run)
    }

    /// **`/logs` READS EVERY RUN DIRECTORY** (IA-LG-02, IA-GR-24's reader
    /// half): an indices run that logged to `logs/gdfl/indices` is on the
    /// unkeyed page and found by `run=` through that directory's own index; a
    /// directory no run has created reads `absent` and one that cannot be
    /// read `unreadable` with its error, by name, never as a directory that
    /// held nothing. The two-directory reader this replaces finds none of the
    /// run's lines, which is the defect (F78).
    #[test]
    fn logs_reads_every_run_directory() {
        let (root, run) = every_run_directory_store();
        let served_dir = root.join("logs");
        let halves = halves_of(Some(served_dir.clone()), Some(&root));
        assert_eq!(halves.len(), 8);
        let state_of = |body: &serde_json::Value, half: &str| -> (String, serde_json::Value) {
            let dir = body["dirs"]
                .as_array()
                .expect("dirs")
                .iter()
                .find(|dir| dir["half"] == half)
                .unwrap_or_else(|| panic!("{half} is not named: {body:#}"));
            (
                dir["state"].as_str().expect("a state").to_owned(),
                dir["error"].clone(),
            )
        };

        // UNKEYED: every directory's lines, and every directory named.
        let unkeyed: serde_json::Value =
            serde_json::from_str(&json_across(&halves, &asked(""), None)).expect("JSON");
        let messages: Vec<&str> = unkeyed["records"]
            .as_array()
            .expect("records")
            .iter()
            .map(|record| record["message"].as_str().expect("a message"))
            .collect();
        assert_eq!(messages.len(), 4, "{unkeyed:#}");
        assert!(messages.contains(&"run claimed"), "{messages:?}");
        assert!(messages.contains(&"the server said this"), "{messages:?}");
        assert_eq!(unkeyed["dirs"].as_array().expect("dirs").len(), 8);
        assert_eq!(state_of(&unkeyed, "served").0, "read");
        assert_eq!(state_of(&unkeyed, "cli").0, "read");
        assert_eq!(state_of(&unkeyed, "gdfl/indices").0, "read");
        for never_created in ["gdfl/stocks", "gdfl/options", "gdfl/store"] {
            assert_eq!(
                state_of(&unkeyed, never_created),
                ("absent".to_owned(), serde_json::Value::Null),
                "{never_created}"
            );
        }
        let (state, error) = state_of(&unkeyed, "verify/zerodha");
        assert_eq!(state, "unreadable");
        assert!(error.is_string(), "its error is said: {error}");

        // KEYED BY THE RUN: both of its audited lines, from its own directory.
        let by_run = asked(&format!("run={run}&durable=1"));
        assert!(!by_run.keys.is_empty(), "run= with durable=1 is a key");
        let keyed: serde_json::Value =
            serde_json::from_str(&keyed_json_across(&halves, &by_run, None)).expect("JSON");
        let rows = keyed["keyed"]["rows"].as_array().expect("rows");
        assert_eq!(rows.len(), 2, "{keyed:#}");
        assert!(
            rows.iter()
                .all(|row| row["half"] == "gdfl/indices" && row["source"] == "audit"),
            "{rows:?}"
        );
        assert_eq!(state_of(&keyed, "gdfl/stocks").0, "absent");
        assert_eq!(keyed["keyed"]["index"]["capped"], false);

        // THE READER THIS REPLACES read the server's directory and `logs/cli`,
        // and finds nothing of the run.
        let two: serde_json::Value = serde_json::from_str(&keyed_json(
            &served_dir,
            Some(&served_dir.join("cli")),
            &by_run,
            None,
        ))
        .expect("JSON");
        assert_eq!(two["keyed"]["matched"], 0, "{two:#}");

        // THE PAGE names each directory with its state too.
        let page = page_across(&halves, &asked(""), None);
        assert!(page.contains("<code>gdfl/stocks</code> absent"), "{page}");
        assert!(page.contains("<code>gdfl/indices</code> read"), "{page}");
        assert!(
            page.contains("<code>verify/zerodha</code> unreadable"),
            "{page}"
        );
        let keyed_page = page_across(&halves, &by_run, None);
        assert!(keyed_page.contains("run claimed"), "{keyed_page}");
        let _ignored = std::fs::remove_dir_all(&root);
    }

    /// **THE AUDIT STREAM'S HEALTH NAMES THE REAPER** (D-4838): what waits
    /// for deletion, what was deleted, what could not be, and `reaper_behind`
    /// with `loud` once more than the stream keeps files wait.
    #[test]
    fn the_audit_health_names_the_reaper() {
        let quiet = telemetry::AuditHealth {
            gone_waiting: 1,
            reaped: 7,
            ..telemetry::AuditHealth::default()
        };
        let body: serde_json::Value =
            serde_json::from_str(&audit_json_of(Some(&quiet))).expect("JSON");
        assert_eq!(body["gone_waiting"], 1);
        assert_eq!(body["reaped"], 7);
        assert_eq!(body["reap_failures"], 0);
        assert_eq!(body["reaper_behind"], false);
        assert_eq!(body["reap_error"], serde_json::Value::Null);
        assert_eq!(body["loud"], false);
        let behind = telemetry::AuditHealth {
            gone_waiting: u64::from(telemetry::AUDIT_KEEP_FILES) + 1,
            reap_failures: 1,
            reap_error: Some("ZZ: cannot delete".to_owned()),
            ..telemetry::AuditHealth::default()
        };
        let body: serde_json::Value =
            serde_json::from_str(&audit_json_of(Some(&behind))).expect("JSON");
        assert_eq!(body["reaper_behind"], true);
        assert_eq!(body["reap_failures"], 1);
        assert_eq!(body["reap_error"], "ZZ: cannot delete");
        assert_eq!(body["loud"], true);
        assert_eq!(audit_json_of(None), "null");
    }

    /// **SEVERAL LISTS MERGE OLDEST FIRST, AN EARLIER LIST WINNING A TIE**:
    /// empty lists, one list, equal stamps and interleaved ones.
    #[test]
    fn lists_merge_oldest_first_without_a_sort() {
        let at = |item: &(i64, &str)| item.0;
        let none: [&[(i64, &str)]; 0] = [];
        assert!(merged_oldest_first(&none, at).is_empty());
        assert!(merged_oldest_first(&[&[][..], &[][..]], at).is_empty());
        let one = [(1, "a"), (2, "b")];
        assert_eq!(merged_oldest_first(&[&one[..]], at), one);
        let left = [(1, "l1"), (5, "l5"), (5, "l5b")];
        let right = [(0, "r0"), (5, "r5"), (9, "r9")];
        assert_eq!(
            merged_oldest_first(&[&left[..], &[][..], &right[..]], at),
            [
                (0, "r0"),
                (1, "l1"),
                (5, "l5"),
                (5, "l5b"),
                (5, "r5"),
                (9, "r9")
            ]
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod r3_24_tests {
    //! R3-24: the never-rolling state stream read, the new keys, a capped
    //! answer continued, every answer's reach, and the import journal's
    //! contradiction of an old "day imported" line (DB-G88, DB-023, DB-A48).

    use super::{
        Half, IST_OFFSET_MS, MAX_KEYED_READS, SCAN_BYTES, YOUNG_AUDIT_DAYS, asked, instant_of,
        json_over, keyed_json, logs_json_over, page_over, reach_json, store_of,
    };

    /// A fresh directory and a sink writing every level into it.
    fn sink_in(name: &str) -> (std::path::PathBuf, telemetry::Sink) {
        let dir = crate::scratch::path(&format!("logs-r324-{name}"));
        let _ignored = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let sink = telemetry::Sink::open(
            &telemetry::Config::new(&dir).with_min_level(telemetry::Level::Trace),
        )
        .expect("opens");
        (dir, sink)
    }

    /// The keyed `/logs.json` body over `dir` alone.
    fn keyed(dir: &std::path::Path, query: &str) -> serde_json::Value {
        let asked = asked(query);
        assert!(!asked.keys.is_empty(), "{query} names a key");
        serde_json::from_str(&keyed_json(dir, None, &asked, None)).expect("JSON")
    }

    /// The records of an answer, as `(target, message)`.
    fn named(got: &serde_json::Value) -> Vec<(String, String)> {
        got["records"]
            .as_array()
            .expect("records")
            .iter()
            .map(|record| {
                (
                    record["target"].as_str().unwrap().to_owned(),
                    record["message"].as_str().unwrap().to_owned(),
                )
            })
            .collect()
    }

    /// The names an answer lists as ignored.
    fn ignored(got: &serde_json::Value) -> Vec<String> {
        got["ignored"]
            .as_array()
            .expect("ignored")
            .iter()
            .map(|entry| entry["param"].as_str().unwrap().to_owned())
            .collect()
    }

    /// **A CAPPED ANSWER CAN BE CONTINUED** (R3-24): 2,100 lines of one code
    /// stand newer than the one the request's target admits, so the first
    /// page reads its 2,000 lines, finds nothing, and says so: "the first
    /// 2000 of 2101", `capped`, and a cursor. The cursor's page reads on from
    /// there and finds the line. Before R3-24 the answer named no `of` and
    /// said nothing of how many it had not read.
    #[test]
    fn a_capped_answer_can_be_continued() {
        let (dir, sink) = sink_in("capped");
        let _ = sink.emit(
            &telemetry::Event::error("gdfl.import", "refused")
                .with("code", "too_many_entries")
                .with("kind", "options")
                .with("day", "2020-01-02"),
        );
        for n in 0..2_100u32 {
            let _ = sink.emit(
                &telemetry::Event::error("pull.spot", "member did not land")
                    .with("code", "too_many_entries")
                    .with("n", n),
            );
        }
        let query = "code=too_many_entries&target=gdfl.import&limit=5";
        let first = keyed(&dir, query);
        let index = &first["keyed"]["index"];
        assert_eq!(first["records"].as_array().unwrap().len(), 0, "{index:#}");
        assert_eq!(index["capped"], true);
        assert_eq!(index["reads"], MAX_KEYED_READS);
        assert_eq!(index["of"], 2_101);
        let said = index["said"].as_str().expect("words");
        assert!(said.contains("the first 2000 of 2101"), "{said}");
        assert!(said.contains("before=<next>"), "{said}");
        let next = first["keyed"]["next"].as_str().expect("a cursor");
        let second = keyed(&dir, &format!("{query}&before={next}"));
        assert_eq!(
            named(&second),
            [("gdfl.import".to_owned(), "refused".to_owned())]
        );
        assert_eq!(second["keyed"]["index"]["capped"], false);
        assert!(second["keyed"]["next"].is_null(), "every line read");
        assert!(
            second["keyed"]["index"]["said"]
                .as_str()
                .unwrap()
                .contains("every match")
        );
        drop(sink);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **THE LOG IS ASKED BY MONTH, BY REASON, BY IMPORT KIND AND BY A TIME
    /// SPAN** (R3-24): each through its own posting or the files its time
    /// meets, each narrowing a key beside it, each garbage value named.
    #[test]
    fn the_log_is_asked_by_month_reason_kind_and_span() {
        let (dir, sink) = sink_in("keys");
        let refusal = |kind: &'static str, day: &'static str, code: &'static str, reason| {
            telemetry::Event::error("gdfl.import", "refused")
                .with("kind", kind)
                .with("day", day)
                .with("code", code)
                .with("reason", reason)
        };
        let pause = || std::thread::sleep(std::time::Duration::from_millis(5));
        let _ = sink.emit(&refusal("options", "2020-01-02", "A", "census_full"));
        pause();
        let _ = sink.emit(&refusal("stocks", "2020-02-03", "B", "census_full"));
        pause();
        let _ = sink.emit(&refusal("options", "2020-02-04", "C", "bad_row"));
        let days = |got: &serde_json::Value| -> Vec<String> {
            got["records"]
                .as_array()
                .unwrap()
                .iter()
                .map(|record| record["fields"]["day"].as_str().unwrap().to_owned())
                .collect()
        };
        assert_eq!(
            days(&keyed(&dir, "month=2020-02")),
            ["2020-02-04", "2020-02-03"]
        );
        assert_eq!(
            days(&keyed(&dir, "reason=census_full")),
            ["2020-02-03", "2020-01-02"],
            "found by reason though each code differs"
        );
        assert_eq!(
            days(&keyed(&dir, "kind=options")),
            ["2020-02-04", "2020-01-02"]
        );
        assert_eq!(
            days(&keyed(&dir, "kind=stocks&month=2020-02")),
            ["2020-02-03"]
        );
        let by_kind = keyed(&dir, "kind=options");
        assert_eq!(
            by_kind["keyed"]["kind"], "options",
            "the importer's spelling"
        );
        assert_eq!(asked("kind=Options").ignored[0].0, "kind", "and only it");

        // A SPAN ALONE: exactly the line stamped inside it.
        let all = keyed(&dir, "month=2020-01");
        let at = all["records"][0]["ts"].as_i64().unwrap();
        let got = keyed(&dir, &format!("from={at}&to={at}"));
        assert_eq!(days(&got), ["2020-01-02"], "{got:#}");
        assert_eq!(got["keyed"]["span"]["from"], at);
        assert_eq!(got["keyed"]["index"]["postings"][0]["dimension"], "span");
        let open_ended = keyed(&dir, &format!("from={at}"));
        assert_eq!(open_ended["records"].as_array().unwrap().len(), 3);
        assert!(open_ended["keyed"]["span"]["to"].is_null());
        // A SPAN BESIDE A KEY narrows it, and one before every line is empty.
        assert_eq!(
            days(&keyed(&dir, &format!("reason=census_full&to={at}"))),
            ["2020-01-02"]
        );
        assert!(
            keyed(&dir, "to=2020-01-01")["records"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        // GARBAGE IS NAMED, never read as no filter.
        let bad = serde_json::from_str::<serde_json::Value>(&json_over(
            &dir,
            None,
            &asked("month=2020-13&kind=futures&reason=a/b&from=yesterday&to=x"),
            None,
        ))
        .expect("JSON");
        assert_eq!(ignored(&bad), ["month", "reason", "kind", "from"]);
        let reversed = asked("from=2026-10-17&to=2026-10-16");
        assert!(reversed.keys.span.is_none());
        assert_eq!(reversed.ignored[0].0, "from");
        let only_to = asked("to=junk");
        assert_eq!(only_to.ignored[0].0, "to");
        drop(sink);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// An instant is a number of milliseconds, an IST day or minute or second,
    /// or the same in UTC with `Z`; the end of a day or minute is its last
    /// millisecond; nothing else reads.
    #[test]
    fn an_instant_is_read_in_ist_unless_it_says_utc() {
        let midnight_utc = 20_378 * 86_400_000_i64; // 2025-10-17
        assert_eq!(instant_of("2025-10-17Z", false), Some(midnight_utc));
        assert_eq!(
            instant_of("2025-10-17", false),
            Some(midnight_utc - IST_OFFSET_MS)
        );
        assert_eq!(
            instant_of("2025-10-17", true),
            Some(midnight_utc - IST_OFFSET_MS + 86_400_000 - 1)
        );
        assert_eq!(
            instant_of("2025-10-17T09:15", false),
            Some(midnight_utc - IST_OFFSET_MS + (9 * 60 + 15) * 60_000)
        );
        assert_eq!(
            instant_of("2025-10-17T09:15", true),
            Some(midnight_utc - IST_OFFSET_MS + (9 * 60 + 16) * 60_000 - 1)
        );
        assert_eq!(
            instant_of("2025-10-17T09:15:30Z", true),
            Some(midnight_utc + ((9 * 60 + 15) * 60 + 31) * 1_000 - 1)
        );
        assert_eq!(instant_of("1760659200000", false), Some(1_760_659_200_000));
        for garbage in [
            "",
            "abc",
            "2025-10-17T24:00",
            "2025-10-17T09:60",
            "2025-10-17T09:15:60",
            "2025-10-17T09",
            "2025-10-17T9:15",
            "2025-10-17T09:15:30:00",
            "2025-13-01",
            "12345678901234567",
        ] {
            assert_eq!(instant_of(garbage, false), None, "{garbage:?}");
        }
    }

    /// **A TARGET ALONE READS EVERY AUDITED LINE, NOT THE NEWEST 4 MiB**
    /// (R3-24, DB-023): one refusal, then more than [`SCAN_BYTES`] of other
    /// lines over it. `target=` used to be the ring's filter and stopped at
    /// the scan cap without it; it is the index's key now, with the newest
    /// ring bytes beside it on a first page.
    #[test]
    fn a_target_alone_reads_every_audited_line_not_the_newest_4_mib() {
        let (dir, sink) = sink_in("target-alone");
        let _ = sink.emit(
            &telemetry::Event::error("pull.spot", "member did not land").with("instrument", "ACC"),
        );
        let wide = "w".repeat(telemetry::MAX_STR_VALUE_BYTES);
        let mut written = 0u64;
        while written < SCAN_BYTES + 512 * 1024 {
            let before = sink.health().current_bytes;
            let _ =
                sink.emit(&telemetry::Event::info("api.noise", "busy").with("w", wide.as_str()));
            written += sink.health().current_bytes - before;
        }
        let read = asked("target=pull.spot");
        assert_eq!(read.keys.target.as_deref(), Some("pull.spot"));
        assert!(read.ring, "a first page reads the ring's newest bytes too");
        let got = keyed(&dir, "target=pull.spot");
        assert_eq!(
            named(&got),
            [("pull.spot".to_owned(), "member did not land".to_owned())],
            "{:#}",
            got["keyed"]
        );
        assert_eq!(got["keyed"]["rows"][0]["source"], "audit");
        assert_eq!(
            got["hit_scan_cap"], true,
            "the ring alone would have missed it"
        );
        // A RUN BESIDE IT keeps `target=` a filter of the ring, as before.
        assert!(asked("target=pull.spot&run=7").keys.is_empty());
        drop(sink);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A STATE CHANGE THE AUDIT STREAM LOST IS ANSWERED FROM THE STATE
    /// STREAM** (R3-24, DB-G88), and one both streams hold is shown once.
    #[test]
    fn a_state_change_is_found_in_the_state_stream_once() {
        let (dir, sink) = sink_in("state");
        let _ = sink.emit(&telemetry::Event::warn("autopilot", "paused").with("epoch", 3u64));
        let _ = sink.emit(&telemetry::Event::error("x", "refused"));
        let both = keyed(&dir, "target=autopilot");
        assert_eq!(both["keyed"]["matched"], 1, "{:#}", both["keyed"]);
        assert_eq!(both["records"].as_array().unwrap().len(), 1);
        let sources: Vec<&str> = both["keyed"]["index"]["postings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|posting| posting["source"].as_str().unwrap())
            .collect();
        assert_eq!(sources, ["audit", "state"]);
        drop(sink);
        // AS IF THE REFUSALS HAD ROLLED IT AWAY: the audit stream is gone.
        std::fs::remove_file(telemetry::audit_current_path(&dir)).expect("the audit file");
        std::fs::remove_dir_all(dir.join(telemetry::AUDIT_INDEX_DIR)).expect("its index");
        let lost = keyed(&dir, "target=autopilot");
        assert_eq!(
            named(&lost),
            [("autopilot".to_owned(), "paused".to_owned())]
        );
        assert_eq!(lost["keyed"]["rows"][0]["source"], "state");
        assert_eq!(
            lost["keyed"]["index"]["postings"][0]["state"], "no audit stream",
            "the gone stream is named, never read as empty"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **EVERY LOG ANSWER SAYS HOW FAR BACK IT REACHES, WITH ONE ALERT WHEN
    /// THE AUDIT HISTORY IS YOUNGER THAN 30 DAYS** (R3-24, DB-G88).
    #[test]
    fn every_log_answer_says_how_far_back_it_reaches() {
        let (dir, sink) = sink_in("reach");
        let _ = sink.emit(&telemetry::Event::info("cli.lifecycle", "command started"));
        let _ = sink.emit(&telemetry::Event::error("x", "refused"));
        let first = serde_json::from_str::<serde_json::Value>(&json_over(
            &dir,
            None,
            &asked("limit=5"),
            None,
        ))
        .expect("JSON");
        let started = first["records"][1]["ts"].as_i64().unwrap();
        let reach = &first["reach"];
        assert_eq!(reach["halves"][0]["half"], "served");
        assert_eq!(reach["halves"][0]["ring_from"], started);
        assert_eq!(reach["halves"][0]["state_from"], started);
        assert_eq!(reach["audited_from"], started);
        assert_eq!(reach["young_below_days"], YOUNG_AUDIT_DAYS);
        assert!(
            reach["alert"]
                .as_str()
                .is_some_and(|alert| alert.contains("younger than 30 days")),
            "{reach:#}"
        );
        // THE KEYED ANSWER SAYS IT TOO.
        assert_eq!(keyed(&dir, "target=x")["reach"]["audited_from"], started);
        // THIRTY-ONE DAYS ON there is no alert.
        let halves = [Half {
            name: "served",
            path: dir.clone(),
        }];
        let later: serde_json::Value = serde_json::from_str(&format!(
            "{{{}}}",
            reach_json(&halves, started + 31 * 86_400_000)
        ))
        .expect("JSON");
        assert!(later["reach"]["alert"].is_null(), "{later:#}");
        assert_eq!(later["reach"]["days"], 31);
        // NOTHING KEPT is the alert, never a quiet answer.
        let empty = [Half {
            name: "served",
            path: dir.join("nothing-here"),
        }];
        let none: serde_json::Value =
            serde_json::from_str(&format!("{{{}}}", reach_json(&empty, started))).expect("JSON");
        assert!(
            none["reach"]["alert"]
                .as_str()
                .unwrap()
                .contains("no directory read keeps an audit history")
        );
        assert!(none["reach"]["halves"][0]["ring_from"].is_null());
        // THE PAGE says it, the alert first.
        let page = page_over(&dir, None, &asked("limit=5"), None);
        assert!(page.contains("Audit history is short"), "{page}");
        assert!(page.contains("Reaches back:"));
        drop(sink);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A CONTRADICTED DAY-IMPORTED LINE IS MARKED** (R3-24, DB-A48): the
    /// import journal closed 2020-01-02 `incomplete` and holds no line for
    /// 2022-03-07, so the log's "day imported" for each carries a conflict
    /// with the journal's outcome beside it; 2020-01-03, which the journal
    /// closed `done`, does not; nor does any other line.
    #[test]
    fn a_contradicted_day_imported_line_is_marked() {
        let root = crate::server::import_journal::tests::journal(
            "r324-contradicted",
            "begin options 2020-01-02 *\n\
             incomplete options 2020-01-02 * definition=2 files=12 seconds=333 failures=1 late=0 late_unresolved=0 max_back_s=0\n\
             begin options 2020-01-03 *\n\
             done options 2020-01-03 * definition=2 files=11 seconds=222 failures=0 late=0 late_unresolved=0 max_back_s=0\n",
        );
        let cli = root.join("logs").join("cli");
        {
            let sink = telemetry::Sink::open(&telemetry::Config::new(&cli)).expect("opens");
            for day in ["2020-01-02", "2020-01-03", "2022-03-07"] {
                let day: &'static str = Box::leak(day.to_owned().into_boxed_str());
                let _ = sink.emit(
                    &telemetry::Event::info("gdfl.import", "day imported")
                        .with("kind", "options")
                        .with("day", day),
                );
            }
            let _ = sink.emit(
                &telemetry::Event::info("gdfl.import", "day incomplete")
                    .with("kind", "options")
                    .with("day", "2020-01-02"),
            );
        }
        let got: serde_json::Value =
            serde_json::from_str(&logs_json_over(None, &root, "limit=10")).expect("JSON");
        let conflict_of = |day: &str, message: &str| -> serde_json::Value {
            got["records"]
                .as_array()
                .unwrap()
                .iter()
                .find(|record| record["fields"]["day"] == day && record["message"] == message)
                .unwrap_or_else(|| panic!("{day} {message}: {got:#}"))
                .get("conflict")
                .cloned()
                .unwrap_or(serde_json::Value::Null)
        };
        let incomplete = conflict_of("2020-01-02", "day imported");
        assert_eq!(
            incomplete["journal_outcome"], "incomplete",
            "{incomplete:#}"
        );
        assert_eq!(incomplete["journal_line"], 2);
        assert_eq!(incomplete["with"], "import journal");
        assert!(conflict_of("2020-01-03", "day imported").is_null());
        let unlined = conflict_of("2022-03-07", "day imported");
        assert_eq!(unlined["journal_outcome"], "no journal line");
        assert!(unlined["journal_line"].is_null());
        assert!(conflict_of("2020-01-02", "day incomplete").is_null());
        // THE PAGE marks the same rows.
        let halves = super::halves_of(None, Some(&root));
        assert_eq!(store_of(&halves).as_deref(), Some(root.as_path()));
        let page = super::page_across(&halves, &asked("limit=10"), None);
        assert_eq!(page.matches("<b>CONFLICT</b>").count(), 2, "{page}");
        assert!(page.contains("closed this day incomplete (line 2)"));
        // NO STORE, NO JOURNAL: nothing to compare with, nothing marked.
        assert!(
            store_of(&[Half {
                name: "served",
                path: cli.clone(),
            }])
            .is_none()
        );
        let _ignored = std::fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod r3_24_picture_tests {
    //! R3-24: the `/logs` picture from counters, the live tail from memory,
    //! and both durable streams' health on every answer.

    use super::{
        Half, all_halves_live, asked, live_admits, picture_json, picture_note, streams_json,
        streams_note,
    };

    fn sink_in(name: &str) -> (std::path::PathBuf, telemetry::Sink) {
        let dir = crate::scratch::path(&format!("logs-r324p-{name}"));
        let _ignored = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let sink = telemetry::Sink::open(
            &telemetry::Config::new(&dir).with_min_level(telemetry::Level::Trace),
        )
        .expect("opens");
        (dir, sink)
    }

    /// **THE LOGS PICTURE IS READ FROM COUNTERS, NOT FROM FILES** (R3-24):
    /// every ring file deleted under the sink, the picture still says how
    /// many lines of each level were written, which codes most, and the
    /// ring's oldest line, and the page draws it from the same counters.
    #[test]
    fn the_logs_picture_is_read_from_counters_not_from_files() {
        let (dir, sink) = sink_in("picture");
        let _ = sink.emit(&telemetry::Event::info("a", "one"));
        let _ = sink.emit(&telemetry::Event::info("a", "two"));
        let _ = sink.emit(&telemetry::Event::warn("a", "three").with("code", "late"));
        for _ in 0..3 {
            let _ = sink.emit(&telemetry::Event::error("a", "refused").with("code", "census_full"));
        }
        for path in telemetry::paths_newest_first(&dir, telemetry::DEFAULT_KEEP_FILES) {
            let _gone = std::fs::remove_file(path);
        }
        assert!(
            telemetry::tail(&dir, 8, &telemetry::Query::last(10))
                .records
                .is_empty(),
            "premise: no file holds a line"
        );
        let picture = sink.picture();
        let json: serde_json::Value =
            serde_json::from_str(&format!("{{{}}}", picture_json(Some(&picture)))).expect("JSON");
        let p = &json["picture"];
        assert_eq!(p["from"], "counters");
        assert_eq!(p["counted"], 6);
        assert_eq!(p["levels"][4], "error");
        let mut by_level = [0u64; 5];
        for minute in p["minutes"].as_array().expect("minutes") {
            for (sum, count) in by_level
                .iter_mut()
                .zip(minute["counts"].as_array().unwrap())
            {
                *sum += count.as_u64().unwrap();
            }
        }
        assert_eq!(by_level, [0, 0, 2, 1, 3]);
        assert_eq!(p["minutes"].as_array().unwrap().len(), 60);
        assert_eq!(p["codes"][0]["code"], "census_full");
        assert_eq!(p["codes"][0]["count"], 3);
        assert!(p["oldest_ms"].is_i64(), "{p:#}");
        let page = picture_note(Some(&picture));
        assert!(page.contains("<svg"), "{page}");
        assert!(page.contains("no file read"));
        assert!(page.contains("census_full"));
        assert!(page.matches("<rect").count() >= 2, "{page}");
        assert!(page.contains("error 3;"), "{page}");
        // A PROCESS WITH NO SINK says so, in both.
        assert_eq!(picture_json(None), "\"picture\":null");
        assert!(picture_note(None).contains("No picture"));
        drop(sink);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// A full code table says a count may be over, and a page with no code
    /// says that too.
    #[test]
    fn the_picture_says_when_its_code_table_replaced_one() {
        let picture = telemetry::run_dirs::Picture {
            codes_replaced: 2,
            ..telemetry::run_dirs::Picture::default()
        };
        let page = picture_note(Some(&picture));
        assert!(page.contains("took the least-counted one's place 2 time(s)"));
        assert!(page.contains("No line carried a code"));
        assert!(page.contains("Oldest line the ring holds: none"));
    }

    /// **THE LIVE TAIL IS A COPY FROM MEMORY, NOT A FILE SCAN** (R3-24): the
    /// ring's file deleted, the served directory's newest lines still answer,
    /// with no byte read; a filter the copy cannot fill reads the files, as
    /// it always did.
    #[test]
    fn the_live_tail_is_a_copy_from_memory() {
        let (dir, sink) = sink_in("live");
        for n in 0..60u32 {
            let _ = sink.emit(&telemetry::Event::info("a.b", "line").with("n", n));
        }
        std::fs::remove_file(telemetry::current_path(&dir)).expect("the ring file");
        let halves = [Half {
            name: "served",
            path: dir.clone(),
        }];
        let tail = all_halves_live(&halves, &asked("limit=50").query, Some(&sink));
        assert_eq!(tail.records.len(), 50);
        assert_eq!((tail.bytes_read, tail.files_read), (0, 0));
        assert_eq!(
            tail.records[0]
                .field("n")
                .and_then(telemetry::OwnedValue::as_u64),
            Some(59)
        );
        assert_eq!(tail.missing, Some(0));
        assert!(!tail.reached_oldest, "the copy is the newest lines only");
        let filtered = all_halves_live(&halves, &asked("limit=5&level=info").query, Some(&sink));
        assert_eq!(filtered.records.len(), 5);
        assert_eq!(filtered.missing, None, "a gap means nothing under a filter");
        // NOT ENOUGH IN THE COPY: the files answer, and the file is gone.
        let narrow = all_halves_live(&halves, &asked("limit=5&level=error").query, Some(&sink));
        assert!(narrow.records.is_empty());
        // ANOTHER DIRECTORY is never answered from this sink's copy.
        let other = [Half {
            name: "cli",
            path: dir.join("elsewhere"),
        }];
        assert!(
            all_halves_live(&other, &asked("limit=5").query, Some(&sink))
                .records
                .is_empty()
        );
        // THE LIVE FILTER IS THE FILE TAIL'S: floor, run, target, since.
        let record = &tail.records[0];
        let query = |q: &str| asked(q).query;
        assert!(live_admits(&query("level=info&target=a"), record));
        assert!(!live_admits(&query("level=warn"), record));
        assert!(!live_admits(&query("target=a.bc"), record));
        assert!(!live_admits(&query("run=4"), record));
        assert!(live_admits(
            &telemetry::Query::last(1).since(record.at_unix_millis),
            record
        ));
        assert!(!live_admits(
            &telemetry::Query::last(1).since(record.at_unix_millis + 1),
            record
        ));
        drop(sink);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **BOTH DURABLE STREAMS ARE SERVED, WITH THE ROLL'S WAITING DELETES**
    /// (R3-24): on the JSON and the page, loud when the reaper is behind or
    /// the state stream has grown past its loud size.
    #[test]
    fn both_streams_and_their_waiting_deletes_are_served() {
        let audit = telemetry::AuditHealth {
            serial: 7,
            current_bytes: 100,
            rotations: 3,
            gone_waiting: u64::from(telemetry::AUDIT_KEEP_FILES) + 1,
            reaped: 4,
            reap_failures: 1,
            last_error: Some("a <failure>".to_owned()),
            ..telemetry::AuditHealth::default()
        };
        let state = telemetry::AuditHealth {
            never_rolls: true,
            current_bytes: telemetry::run_dirs::STATE_LOUD_BYTES + 1,
            written: 9,
            ..telemetry::AuditHealth::default()
        };
        let json: serde_json::Value =
            serde_json::from_str(&format!("{{{}}}", streams_json(Some(&audit), Some(&state))))
                .expect("JSON");
        assert_eq!(json["audit_stream"]["gone_waiting"], 17);
        assert_eq!(json["audit_stream"]["reaper_behind"], true);
        assert_eq!(json["state_stream"]["loud"], true);
        assert_eq!(json["state_stream"]["written"], 9);
        assert!(
            json["state_stream"]["max_file_bytes"].is_null(),
            "it never rolls"
        );
        assert!(json["state_stream"]["keep_files"].is_null());
        assert_eq!(json["state_stream"]["never_rolls"], true);
        assert_eq!(
            json["state_stream"]["loud_bytes"],
            telemetry::run_dirs::STATE_LOUD_BYTES
        );
        assert_eq!(
            json["audit_stream"]["max_file_bytes"],
            telemetry::AUDIT_MAX_FILE_BYTES
        );
        assert!(json["audit_stream"]["loud_bytes"].is_null());
        let page = streams_note(Some(&audit), Some(&state));
        assert!(page.starts_with("<p class=\"halt\">"), "{page}");
        assert!(
            page.contains(
                "17 index generation(s) wait to be deleted, 4 deleted, 1 delete(s) failed"
            )
        );
        assert!(page.contains("the deletes are behind the rolls"));
        assert!(page.contains("past its loud size"));
        assert!(page.contains("a &lt;failure&gt;"));
        let quiet = streams_note(Some(&telemetry::AuditHealth::default()), None);
        assert!(quiet.starts_with("<p class=\"lead\">"), "{quiet}");
        assert!(!quiet.contains("behind"));
        assert!(streams_note(None, None).contains("keeps no audit stream"));
        let none: serde_json::Value =
            serde_json::from_str(&format!("{{{}}}", streams_json(None, None))).expect("JSON");
        assert!(none["audit_stream"].is_null() && none["state_stream"].is_null());
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod r3_24_follow_tests {
    //! R3-24: the importer's refusal lines are followed on a background
    //! thread, and no page request reads more than a fixed slice.

    use super::{REQUEST_SLICE_BYTES, import_refusals, refusals_in};

    /// A ring of rolled files holding `days` x `per_day` importer refusals of
    /// the options kind, written by a sink whose files roll at 256 KiB, under
    /// `<root>/logs/cli`. Answers the root.
    fn big_ring(name: &str, days: u32, per_day: u32) -> std::path::PathBuf {
        let root = crate::scratch::path(&format!("logs-r324f-{name}"));
        let _ignored = std::fs::remove_dir_all(&root);
        let dir = root.join("logs").join("cli");
        let sink = telemetry::Sink::open(
            &telemetry::Config::new(&dir)
                .with_max_file_bytes(256 * 1024)
                .with_audit(false),
        )
        .expect("opens");
        let why = "a reason long enough to make the ring several slices wide ".repeat(4);
        for day in 0..days {
            let text: &'static str =
                Box::leak(format!("2020-01-{:02}", day % 28 + 1).into_boxed_str());
            for n in 0..per_day {
                let _ = sink.emit(
                    &telemetry::Event::error("gdfl.import", "refused")
                        .with("kind", "options")
                        .with("day", text)
                        .with("n", n)
                        .with("why", why.as_str()),
                );
            }
        }
        assert!(
            sink.health().rotations >= 4,
            "premise: a ring of rolled files"
        );
        root
    }

    /// The refusals the evidence holds for every January 2020 day.
    fn held_in(refusals: &super::DirRefusals) -> usize {
        (1..=28)
            .map(|day| refusals.of("options", &format!("2020-01-{day:02}")).len())
            .sum()
    }

    /// **NO PAGE REQUEST READS MORE THAN A FIXED SLICE** (R3-24): a ring
    /// several slices wide is read across calls, each folding at most
    /// [`REQUEST_SLICE_BYTES`], each saying how much is left, and the last
    /// holds every refusal. It used to be read whole inside the first
    /// request.
    #[test]
    fn a_request_reads_at_most_its_slice() {
        let root = big_ring("slice", 28, 100);
        let dir = root.join("logs").join("cli");
        let mut calls = 0u32;
        let mut read = 0u64;
        let mut last_behind = u64::MAX;
        let held = loop {
            let (held, behind) = refusals_in(&dir, REQUEST_SLICE_BYTES);
            assert!(
                held.bytes - read <= REQUEST_SLICE_BYTES,
                "call {calls} read {} bytes",
                held.bytes - read
            );
            assert!(behind < last_behind, "every slice makes progress");
            read = held.bytes;
            last_behind = behind;
            calls += 1;
            if behind == 0 {
                break held;
            }
        };
        assert!(
            calls >= 4,
            "a ring several slices wide took {calls} call(s)"
        );
        assert_eq!(held_in(&held), 28 * 100);
        assert!(held.errors.is_empty(), "{:?}", held.errors);
        let _ignored = std::fs::remove_dir_all(&root);
    }

    /// **THE FOLLOWER READS THE REST, OFF EVERY REQUEST** (R3-24): the first
    /// question of a wide ring answers its slice and names the directory as
    /// not read yet, never "none"; the follower catches up in the background
    /// and a later question holds every refusal.
    #[test]
    fn the_follower_reads_what_a_request_left() {
        let root = big_ring("follow", 28, 100);
        let first = import_refusals(&root);
        let unread = first.unread("options");
        assert!(
            unread
                .iter()
                .any(|why| why.contains("logs/cli") && why.contains("not read yet")),
            "{unread:?}"
        );
        let started = std::time::Instant::now();
        let caught_up = loop {
            let now = import_refusals(&root);
            if now
                .unread("options")
                .iter()
                .all(|why| !why.contains("logs/cli"))
            {
                break now;
            }
            assert!(
                started.elapsed() < std::time::Duration::from_mins(1),
                "the follower did not catch up: {:?}",
                now.unread("options")
            );
            std::thread::sleep(std::time::Duration::from_millis(50));
        };
        let cli = caught_up
            .dirs()
            .iter()
            .find(|dir| dir.half == "cli")
            .expect("the cli directory");
        assert_eq!(cli.state, super::DirState::Read);
        let total: usize = (1..=28)
            .map(|day| caught_up.of("options", &format!("2020-01-{day:02}")).len())
            .sum();
        assert_eq!(total, 28 * 100);
        let _ignored = std::fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod r3_24_follower_start_tests {
    //! The follower's start, stated as failing, which no test can cause.

    /// **A FOLLOWER THAT CANNOT START IS SAID, NEVER SILENT** (R3-24): its
    /// words name the failure and what still reads the ring, and a directory
    /// still behind says them; one that starts says nothing.
    #[test]
    fn a_follower_that_cannot_start_is_said() {
        let failed = super::start_follower(|| Err(std::io::Error::other("no threads left")));
        let words = failed.expect("a failure in words");
        assert!(words.contains("no threads left"), "{words}");
        assert!(words.contains("only each request's slice reads it"));
        assert_eq!(super::start_follower(|| Ok(())), None);
        let behind = super::behind_words_with(7, Some(&words));
        assert!(behind.starts_with("7 byte(s) of its ring are not read yet, and the follower"));
        assert!(super::behind_words_with(7, None).contains("off every request"));
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod r3_24_ring_edge_tests {
    //! R3-24: a ring the follower reads a slice at a time never stalls on
    //! bytes that hold no line, and never reads a line not yet whole as one
    //! still to read.

    use super::{REQUEST_SLICE_BYTES, refusals_in};

    /// **A REGION WIDER THAN ANY LINE, WITH NO NEWLINE, IS STEPPED OVER AND
    /// COUNTED**, in a rolled file and in the current one, so the ring is read
    /// to its end and the refusal after it is found; and the current file's
    /// last bytes, a line not yet whole, leave the directory read, not
    /// behind.
    #[test]
    fn a_region_with_no_line_in_it_never_stalls_the_ring() {
        let root = crate::scratch::path("logs-r324e-noline");
        let _ignored = std::fs::remove_dir_all(&root);
        // ONE REAL REFUSAL LINE, as the sink writes it.
        let source = root.join("source");
        {
            let sink = telemetry::Sink::open(&telemetry::Config::new(&source).with_audit(false))
                .expect("opens");
            let _ = sink.emit(
                &telemetry::Event::error("gdfl.import", "refused")
                    .with("kind", "options")
                    .with("day", "2020-02-03"),
            );
        }
        let line = std::fs::read(telemetry::current_path(&source)).expect("the line");
        let dir = root.join("logs").join("cli");
        std::fs::create_dir_all(&dir).expect("the directory");
        let mut rolled = vec![b'x'; 600 * 1024];
        rolled.push(b'\n');
        rolled.extend_from_slice(&line);
        std::fs::write(telemetry::rotated_path(&dir, 1), &rolled).expect("a rolled file");
        let mut current = vec![b'y'; 300 * 1024];
        current.push(b'\n');
        current.extend_from_slice(&line);
        current.extend_from_slice(b"{\"seq\":9,\"a line not yet wh");
        std::fs::write(telemetry::current_path(&dir), &current).expect("the current file");
        let mut calls = 0;
        let held = loop {
            let (held, behind) = refusals_in(&dir, REQUEST_SLICE_BYTES);
            calls += 1;
            assert!(calls < 20, "the ring stalled: {behind} byte(s) behind");
            if behind == 0 {
                break held;
            }
        };
        assert_eq!(held.of("options", "2020-02-03").len(), 2, "{held:?}");
        assert!(
            held.malformed >= 2,
            "each wide region counted: {}",
            held.malformed
        );
        // ASKED AGAIN with nothing new: the fragment is not read again.
        let (again, behind) = refusals_in(&dir, REQUEST_SLICE_BYTES);
        assert_eq!(behind, 0);
        assert!(std::sync::Arc::ptr_eq(&held, &again));
        // THE LINE COMPLETED: read on.
        {
            use std::io::Write as _;
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(telemetry::current_path(&dir))
                .expect("the current file");
            file.write_all(b"ole\"}\n").expect("append");
        }
        let (grown, behind) = refusals_in(&dir, REQUEST_SLICE_BYTES);
        assert_eq!(behind, 0);
        assert!(
            !std::sync::Arc::ptr_eq(&held, &grown),
            "the grown file was read"
        );
        let _ignored = std::fs::remove_dir_all(&root);
    }
}
