//! `GET /disk/copies.json` — the copies register: every extra copy the
//! operator names in `disk.extra_copy_roots`, with its bytes, reader, proof
//! and whether it duplicates a live source; every second name of a live
//! archive file as a red row; and the retirement list the operator acts on
//! by hand (CC-5, FM-607; D-7040..D-7048).
//!
//! # What it answers
//!
//! * `state` — `walked`; or `waiting` when the setting is unset (a named
//!   wait: the key, and that nothing was walked, never a default folder);
//!   or a `503` naming the first walk since start, which another request is
//!   running, with `Retry-After`.
//! * `walked_at`, `walked_at_micros`, `age_ms`, `walk_us`, `fresh` and `why`
//!   on every answer, and `roots_walked`: a root's own stat changes only when
//!   its DIRECT entries change, so a link made deeper in a tree is seen at the
//!   next walk, and the answer says how old its walk is (amendment (f)).
//!   `cli disk-copies` always walks afresh.
//! * `roots` (each copy root: bytes, files, reader, proof, twin,
//!   `duplicates_live`, faults), `live` (each live root and whether it was
//!   walked), `hazards` (each `"hazard": true`), `extra_copies_bytes`, and
//!   `retirement_list` with `retirement_text`, which holds no command:
//!   "the operator removes these by hand; the app never does".
//!
//! # Cost
//!
//! The work runs in the ops-read pool, off the async workers
//! ([`crate::detail::run_ops_read`]). A request whose roots and roots' own
//! stats are unchanged is one settings `stat`, one `lstat` per configured
//! root, and the held answer's body, rendered once per walk: O(roots), never
//! O(files). A request that finds a change walks, O(entries under the roots)
//! (`docs/06-limits.md` L-CC5-1), and a request arriving while another walks
//! is never made to wait for it. Proof: `crates/api/benches/ratio/copies.rs`,
//! n = 200 at 1x, 10x and 100x files.

use std::ffi::OsString;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use axum::http::{HeaderName, StatusCode, header};
use pull::copies_register::{
    self, Cache, Config, CopyRoot, Fs, Hazard, LiveReport, Receipt, Register, Served, Sources,
    State, Twin,
};
use serde_json::{Value, json};
use store::settings::Key;

use super::{Loaded, settings_route};

/// The headers of every answer.
type Headers = [(HeaderName, &'static str); 3];

/// The JSON content type, no caching, and a retry hint for the `503`.
const HEADERS: Headers = [
    (header::CONTENT_TYPE, "application/json; charset=utf-8"),
    (header::CACHE_CONTROL, "no-store"),
    (header::RETRY_AFTER, "1"),
];

/// A path as JSON: its text when it is UTF-8, else its escaped form, which
/// opens with a quote, so it is never mistaken for a lossy guess.
fn path_json(path: &Path) -> Value {
    path.to_str().map_or_else(
        || Value::String(copies_register::quoted(path)),
        |text| Value::String(text.to_owned()),
    )
}

/// The register's body, everything but what changes per request.
fn register_json(register: &Register) -> serde_json::Map<String, Value> {
    let mut walked: Vec<Value> = register
        .roots
        .iter()
        .map(|root| path_json(&root.path))
        .collect();
    walked.extend(
        register
            .live
            .iter()
            .filter(|live| live.walked)
            .map(|live| path_json(&live.root.path)),
    );
    let retirement: Vec<Value> = copies_register::retirement(register)
        .iter()
        .map(|retire| {
            json!({
                "path": path_json(&retire.path),
                "bytes": retire.bytes,
                "files": retire.files,
                "why": retire.why,
                "twin": retire.twin,
                "caution": retire.caution,
                "hazard": retire.hazard,
            })
        })
        .collect();
    let mut map = serde_json::Map::with_capacity(16);
    map.insert("key".to_owned(), json!(copies_register::KEY));
    map.insert(
        "walked_at".to_owned(),
        json!(copies_register::ist_text(register.walked_at_micros)),
    );
    map.insert(
        "walked_at_micros".to_owned(),
        json!(register.walked_at_micros),
    );
    map.insert("walk_us".to_owned(), json!(register.walk_micros));
    map.insert(
        "note".to_owned(),
        json!(
            "a root's own stat changes only when its direct entries change: a link made deeper in \
             a tree is seen at the next walk; cli disk-copies always walks afresh"
        ),
    );
    map.insert("roots_walked".to_owned(), Value::Array(walked));
    map.insert("whole".to_owned(), json!(register.whole()));
    map.insert("extra_copies_bytes".to_owned(), json!(register.extra_bytes));
    map.insert(
        "roots".to_owned(),
        Value::Array(register.roots.iter().map(root_json).collect()),
    );
    map.insert(
        "live".to_owned(),
        Value::Array(register.live.iter().map(live_json).collect()),
    );
    map.insert("unset_live".to_owned(), json!(register.unset_live));
    map.insert(
        "hazards".to_owned(),
        Value::Array(register.hazards.iter().map(hazard_json).collect()),
    );
    map.insert("hazard_count".to_owned(), json!(register.hazards.len()));
    map.insert("retirement_list".to_owned(), Value::Array(retirement));
    map.insert(
        "retirement_text".to_owned(),
        json!(copies_register::retirement_text(register)),
    );
    map.insert(
        "retirement_sentence".to_owned(),
        json!(copies_register::RETIREMENT_SENTENCE),
    );
    map
}

/// One copy root.
fn root_json(root: &CopyRoot) -> Value {
    let (state, refused) = match &root.state {
        State::Walked { whole } => (if *whole { "walked" } else { "partial" }, None),
        State::Refused(why) => ("refused", Some(why.as_str())),
    };
    let measured = !matches!(root.state, State::Refused(_));
    json!({
        "path": path_json(&root.path),
        "state": state,
        "refused": refused,
        "files": measured.then_some(root.files),
        "dirs": measured.then_some(root.dirs),
        "links_not_followed": root.links,
        "others_not_opened": root.others,
        "bytes": measured.then_some(root.bytes),
        "extra_bytes": measured.then_some(root.extra_bytes),
        "newest_mtime": root.newest_mtime_s.map(|s| copies_register::ist_text(s.saturating_mul(1_000_000))),
        "reader": root.reader.as_ref().map_or_else(
            || Value::String("none found".to_owned()),
            |reader| json!({
                "kind": reader.kind.word(),
                "path": path_json(&reader.path),
                "named_by": reader.named_by,
                "how": reader.how,
            }),
        ),
        "inside": root.inside.as_ref().map(|live| json!({
            "kind": live.kind.word(),
            "path": path_json(&live.path),
        })),
        "proof": root.proof.as_deref().map_or_else(|| Value::String("none".to_owned()), path_json),
        "receipts": root.receipts.iter().map(receipt_json).collect::<Vec<_>>(),
        "receipt_count": root.receipt_count,
        "twin": match &root.twin {
            Twin::Proven { sources } => json!({"proven": true, "sources": sources, "words": root.twin.words()}),
            Twin::NotProven(_) => json!({"proven": false, "words": root.twin.words()}),
        },
        "duplicates_live": root.linked_live > 0,
        "linked_live": root.linked_live,
        "linked_elsewhere": root.linked_elsewhere.iter().map(|p| path_json(p)).collect::<Vec<_>>(),
        "faults": root.faults.iter().map(|(path, why)| json!({"path": path_json(path), "why": why})).collect::<Vec<_>>(),
        "skipped_roots": root.skipped.iter().map(|p| path_json(p)).collect::<Vec<_>>(),
        "revisited": root.revisited.iter().map(|p| path_json(p)).collect::<Vec<_>>(),
        "retirable": root.retirable(),
    })
}

/// One receipt.
fn receipt_json(receipt: &Receipt) -> Value {
    json!({
        "path": path_json(&receipt.path),
        "kind": receipt.kind.word(),
        "verdict": receipt.verdict,
        "why": receipt.why,
        "statement": receipt.statement,
        "sources": receipt.sources,
        "matched": receipt.matched,
        "first_unmatched": receipt.first_unmatched,
    })
}

/// One live root.
fn live_json(live: &LiveReport) -> Value {
    json!({
        "kind": live.root.kind.word(),
        "path": path_json(&live.root.path),
        "named_by": live.root.named_by,
        "walked": live.walked,
        "why": live.why,
        "files": live.files,
    })
}

/// One red row.
fn hazard_json(hazard: &Hazard) -> Value {
    json!({
        "hazard": true,
        "path": path_json(&hazard.path),
        "live": path_json(&hazard.live),
        "kind": hazard.kind.word(),
        "dev": hazard.dev,
        "ino": hazard.ino,
        "nlink": hazard.nlink,
        "bytes": hazard.bytes,
        "partial": hazard.partial,
    })
}

/// The last register rendered and its body: a request served from memory
/// renders nothing. The `Arc` is kept, so a register is never mistaken for a
/// later one at the same address.
static RENDERED: Mutex<Option<(Arc<Register>, Arc<str>)>> = Mutex::new(None);

/// The rendered body of `register`, once per walk: one JSON object's text
/// without its opening brace, so a request's own fields go in front of it.
fn rendered(register: &Arc<Register>) -> Arc<str> {
    let mut held = RENDERED.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((was, body)) = held.as_ref()
        && Arc::ptr_eq(was, register)
    {
        return Arc::clone(body);
    }
    let text = Value::Object(register_json(register)).to_string();
    let body: Arc<str> = Arc::from(text.strip_prefix('{').unwrap_or(&text));
    *held = Some((Arc::clone(register), Arc::clone(&body)));
    body
}

/// The configuration the settings of `store_root` and `env` name, or the
/// named wait.
fn configured(
    store_root: &Path,
    env: &dyn Fn(&str) -> Option<OsString>,
) -> Result<Config, (copies_register::Wait, Option<String>)> {
    let snapshot = settings_route::snapshot(store_root);
    let saved = |key: Key| {
        snapshot
            .newest
            .as_deref()
            .and_then(|newest| newest.settings.get(key))
    };
    copies_register::config(&Sources {
        copy_roots: saved(Key::ExtraCopyRoots),
        source_gdfl: saved(Key::SourceGdfl),
        source_masters: saved(Key::SourceMasters),
        store_root,
        env,
    })
    .map_err(|wait| (wait, settings_route::degraded(&snapshot)))
}

/// The answer for the store at `store_root`, from `cache`, through `fs`.
pub(crate) fn answer(
    store_root: &Path,
    cache: &Cache,
    fs: &dyn Fs,
    env: &dyn Fn(&str) -> Option<OsString>,
    now: &dyn Fn() -> i64,
) -> (StatusCode, String) {
    let config = match configured(store_root, env) {
        Ok(config) => config,
        Err((wait, degraded)) => {
            return (
                StatusCode::OK,
                json!({
                    "state": "waiting",
                    "key": copies_register::KEY,
                    "why": wait.to_string(),
                    "settings_degraded": degraded,
                    "walked": false,
                })
                .to_string(),
            );
        }
    };
    match copies_register::answer(cache, fs, &config, now) {
        Served::Walking => (
            StatusCode::SERVICE_UNAVAILABLE,
            json!({
                "state": "walking",
                "error": "the first walk of the copy roots since start is running in another \
                          request; ask again in a moment",
            })
            .to_string(),
        ),
        Served::Register {
            register,
            age_ms,
            fresh,
            why,
            rewalking,
        } => {
            // THE HEAD IS THIS REQUEST'S, THE REST THE WALK'S: two objects'
            // texts joined into one, so nothing of the register is rendered
            // or cloned per request.
            let rest = rendered(&register);
            (
                StatusCode::OK,
                format!(
                    "{{\"state\":\"walked\",\"age_ms\":{age_ms},\"fresh\":{fresh},\"why\":{},\
                     \"rewalking\":{rewalking},{rest}",
                    Value::String(why)
                ),
            )
        }
    }
}

/// `GET /disk/copies.json`. See the module header.
pub(super) async fn copies_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
) -> (StatusCode, Headers, String) {
    let root = site.store_root.clone();
    let work = move || {
        answer(
            &root,
            copies_register::process_cache(),
            &copies_register::Host,
            &|name| std::env::var_os(name),
            &copies_register::now_micros,
        )
    };
    match crate::detail::run_ops_read(work).await {
        Ok((status, body)) => (status, HEADERS, body),
        Err(why) => {
            let (status, body) = crate::detail::admission_refused(
                "a copies register read",
                crate::detail::MAX_OPS_READ_CONCURRENT,
                &why,
            );
            (status, HEADERS, body)
        }
    }
}

#[cfg(test)]
#[path = "copies_route_tests.rs"]
mod tests;
