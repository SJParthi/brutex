//! Bounded read-only observed expression-search progress and exact child links.

use axum::http::{StatusCode, Uri};
use cli::expression_search::reader::{Anchor, Page, Progress, Reader};
use serde_json::{Value, json};
use std::collections::{BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

type Response = (
    StatusCode,
    [(axum::http::header::HeaderName, &'static str); 1],
    String,
);
struct Asked {
    identity: [u8; 32],
    snapshot: Option<Anchor>,
    cursor: Option<Anchor>,
    limit: usize,
}
impl Asked {
    fn parse(query: &str) -> Result<Self, String> {
        crate::detail::query_is_bounded(query)?;
        let mut seen = BTreeSet::new();
        for pair in query.split('&') {
            let (name, value) = pair
                .split_once('=')
                .ok_or("search query requires key=value")?;
            if value.is_empty()
                || !matches!(
                    name,
                    "identity" | "snapshot" | "snapshot_seal" | "cursor" | "cursor_seal" | "limit"
                )
                || !seen.insert(name)
            {
                return Err("unknown, empty or repeated search query field".to_owned());
            }
        }
        let identity = crate::candidatejson::hex_param(query, "identity")?
            .ok_or("search identity required")?;
        let snapshot = anchor(query, "snapshot", "snapshot_seal")?;
        let cursor = anchor(query, "cursor", "cursor_seal")?;
        if cursor.is_some() && snapshot.is_none() {
            return Err("search continuation requires its exact snapshot".to_owned());
        }
        let limit = crate::candidatejson::integer(query, "limit")?.unwrap_or(8);
        if !(1..=256).contains(&limit) {
            return Err("search page limit must be 1..=256 checkpoint links".to_owned());
        }
        Ok(Self {
            identity,
            snapshot,
            cursor,
            limit: usize::try_from(limit).map_err(|why| why.to_string())?,
        })
    }
}
fn anchor(query: &str, number: &str, seal: &str) -> Result<Option<Anchor>, String> {
    match (
        crate::candidatejson::integer(query, number)?,
        crate::candidatejson::hex_param(query, seal)?,
    ) {
        (None, None) => Ok(None),
        (Some(sequence), Some(seal)) if sequence > 0 => Ok(Some(Anchor { sequence, seal })),
        _ => Err("search anchor requires both positive sequence and full seal".to_owned()),
    }
}
fn response(status: StatusCode, body: String) -> Response {
    (
        status,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        body,
    )
}
fn refused(status: StatusCode, why: &str) -> Response {
    response(
        status,
        json!({"schema_version":1,"status":"refused","refusal":why,"rows":[]}).to_string(),
    )
}
/// Serves observed search receipts using bounded background detail workers.
pub async fn expression_search_json(uri: Uri) -> Response {
    let asked = match Asked::parse(uri.query().unwrap_or_default()) {
        Ok(asked) => asked,
        Err(why) => return refused(StatusCode::BAD_REQUEST, &why),
    };
    let root = crate::server::store_dir();
    match crate::detail::run(move || match root.and_then(|root| render(&root, &asked)) {
        Ok(body) if body.len() <= crate::detail::MAX_RESPONSE_BYTES => {
            response(StatusCode::OK, body)
        }
        Ok(_) => refused(
            StatusCode::SERVICE_UNAVAILABLE,
            "search JSON exceeds its response admission; no prefix exposed",
        ),
        Err(why) => refused(StatusCode::SERVICE_UNAVAILABLE, &why),
    })
    .await
    {
        Ok(reply) => reply,
        Err(crate::detail::RunError::Saturated) => refused(
            StatusCode::TOO_MANY_REQUESTS,
            "search detail capacity full; nothing queued",
        ),
        Err(crate::detail::RunError::Join(why)) => refused(StatusCode::SERVICE_UNAVAILABLE, &why),
    }
}
struct Session {
    root: PathBuf,
    identity: [u8; 32],
    reader: Reader,
}
fn render(root: &Path, asked: &Asked) -> Result<String, String> {
    static SESSIONS: OnceLock<Mutex<VecDeque<Session>>> = OnceLock::new();
    let slot = SESSIONS.get_or_init(|| Mutex::new(VecDeque::new()));
    if let Some(snapshot) = asked.snapshot {
        // A WARM PAGE of an already-open reader, under the lock as before: it
        // is bounded by `MAX_SCAN_BYTES` and keeps a concurrent page of the
        // same snapshot from being refused as "expired" while it is out.
        // The session it pages is moved to the back of the LRU first
        // (conc:apicache-2, D-2777), found through `held_at` (D-2797).
        let mut sessions = slot.lock().map_err(|_| "search snapshot cache poisoned")?;
        let at = held_at(&sessions, |held| {
            held.root == root
                && held.identity == asked.identity
                && held.reader.progress().checkpoint == Some(snapshot)
        })
        .ok_or("search snapshot is not admitted or expired; refresh its first page")?;
        let index = most_recent(&mut sessions, at);
        let held = sessions
            .get_mut(index)
            .ok_or("search snapshot cache entry missing")?;
        let page = held
            .reader
            .page(asked.cursor, asked.limit, crate::detail::MAX_SCAN_BYTES)?;
        return Ok(body(held.reader.progress(), &page, asked.limit));
    }
    // THE COLD OPEN AND ITS FIRST PAGE RUN WITH THE LOCK RELEASED (expr-3,
    // D-2576). The mutex used to be held across `Reader::open` — a read of the
    // whole checkpoint namespace up to `MAX_SCAN_BYTES` — so every other search
    // request parked on it inside `detail::run`, each holding a detail permit,
    // and the other detail routes answered 429 behind one cold open. The lock
    // is now taken only to install the opened reader, and installing goes
    // through `first_page` (conc:apicache-2, D-2777): a held session that
    // observed exactly what the fresh reader observes is kept with every
    // cursor it learned and the fresh reader is dropped. An unpinned page is
    // always the head page (a cursor requires its snapshot), so the answer
    // paged from the fresh reader is the one the kept session would give.
    #[cfg(test)]
    crate::detail::note_slot_free(slot);
    let Some(mut reader) = Reader::open(root, asked.identity, crate::detail::MAX_SCAN_BYTES)?
    else {
        return Ok(json!({"schema_version":1,"status":"missing","identity":crate::server::hex32(asked.identity),"why":"No recorded expression-search checkpoint namespace exists for this exact search ID.","rows":[],"refusal":null}).to_string());
    };
    let answer = reader
        .page(asked.cursor, asked.limit, crate::detail::MAX_SCAN_BYTES)
        .map(|page| body(reader.progress(), &page, asked.limit));
    let fresh = reader.progress();
    let same_search = |held: &Session| held.root == root && held.identity == asked.identity;
    let (checkpoint, writer, interrupted) =
        (fresh.checkpoint, fresh.writer_observed, fresh.interrupted);
    let mut sessions = slot.lock().map_err(|_| "search snapshot cache poisoned")?;
    first_page(
        &mut sessions,
        Session {
            root: root.to_path_buf(),
            identity: asked.identity,
            reader,
        },
        |held| {
            let observed = held.reader.progress();
            same_search(held)
                && observed.checkpoint == checkpoint
                && observed.writer_observed == writer
                && observed.interrupted == interrupted
        },
        |held| same_search(held) && held.reader.progress().checkpoint == checkpoint,
    );
    answer
}
/// Where an unpinned first page is served from, as an index into `sessions`.
///
/// A held session that observed exactly what `fresh` observes (`unchanged`) is
/// kept and moved to the back: its reader has learned every cursor another
/// viewer paged to, and the fresh reader knows only the head. Replacing it
/// broke that viewer's next page with "search cursor is not linked to this
/// admitted snapshot" whenever anyone loaded the first page of a paused,
/// stopped or exhausted search. Otherwise held sessions of the same snapshot
/// (`superseded`) are dropped and `fresh` is held, the oldest evicted past
/// eight. conc:apicache-2, D-2777.
fn first_page<S>(
    sessions: &mut VecDeque<S>,
    fresh: S,
    unchanged: impl Fn(&S) -> bool,
    superseded: impl Fn(&S) -> bool,
) -> usize {
    if let Some(at) = held_at(sessions, &unchanged) {
        return most_recent(sessions, at);
    }
    sessions.retain(|held| !superseded(held));
    if sessions.len() == SESSIONS_HELD {
        sessions.pop_front();
    }
    sessions.push_back(fresh);
    sessions.len() - 1
}
/// How many search sessions this process holds; the oldest is evicted past it.
const SESSIONS_HELD: usize = 8;
/// The index of the first held session `hit` accepts.
///
/// THE ONE SCAN OF THE CACHE, BOUNDED BY [`SESSIONS_HELD`], a compile-time
/// constant, never by the data: [`first_page`] never lets the cache grow past
/// it. A pinned page and an unpinned first page both look up through here
/// (D-2777, D-2797).
fn held_at<S>(sessions: &VecDeque<S>, hit: impl Fn(&S) -> bool) -> Option<usize> {
    sessions.iter().position(hit)
}
/// Moves the session at `at` to the back of the LRU and returns its new index.
///
/// Used by a pinned page as well as a reused first page: eviction is
/// `pop_front`, so a session a viewer is actively paging through must not stay
/// at the front by insertion order, or eight first pages of OTHER searches
/// evict it mid-walk ("snapshot is not admitted or expired").
/// conc:apicache-2, D-2777.
fn most_recent<S>(sessions: &mut VecDeque<S>, at: usize) -> usize {
    if let Some(used) = sessions.remove(at) {
        sessions.push_back(used);
    }
    sessions.len().saturating_sub(1)
}
fn anchor_json(anchor: Option<Anchor>) -> Value {
    anchor.map_or(Value::Null,|anchor|json!({"sequence":anchor.sequence.to_string(),"seal":crate::server::hex32(anchor.seal)}))
}
fn body(progress: &Progress, page: &Page, limit: usize) -> String {
    let state = if progress.exhausted {
        "exhausted-observed"
    } else if progress.writer_observed {
        "writer-observed"
    } else if progress.checkpoint.is_some() {
        "paused-or-stopped"
    } else {
        "awaiting-first-checkpoint"
    };
    let rows=page.rows.iter().map(|row|json!({"ordinal":row.ordinal.to_string(),"identity":crate::server::hex32(row.identity),"attempt":row.attempt.to_string(),"expression":row.expression,"mask_words":row.mask_words.map(|word|word.to_string()),"signals":{"evaluated":row.summary.evaluated.to_string(),"hits":row.summary.hits.to_string(),"misses":row.summary.misses.to_string(),"unknown":row.summary.unknown.to_string()},"capture_digest":row.capture_digest.map(crate::server::hex32)})).collect::<Vec<_>>();
    json!({"schema_version":1,"status":"observed","identity":crate::server::hex32(progress.identity),"snapshot":anchor_json(progress.checkpoint),"state":state,"writer_observed":progress.writer_observed,"exhausted":progress.exhausted,"work":progress.work.to_string(),"candidates":progress.candidates.to_string(),"qualifying":progress.qualifying.to_string(),"signal_rows":progress.rows.to_string(),"interrupted_reservations":progress.interrupted.to_string(),"limit":limit,"links":page.links,"next":anchor_json(page.next),"page_complete":true,"authority":"saved-receipts-and-grammar-transitions","rows":rows,"refusal":null}).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **One viewer's first page does not discard another viewer's cursors.**
    /// conc:apicache-2, D-2777.
    ///
    /// A session is modelled as (snapshot, cursors learned). Viewer A has
    /// paged to two cursors on snapshot 7; viewer B's unpinned load of the
    /// same, unchanged snapshot must be served from A's session, which keeps
    /// what A learned. A snapshot that moved on still replaces the old one.
    #[test]
    fn an_unchanged_first_page_keeps_the_held_session_and_its_learned_cursors() {
        let mut sessions: VecDeque<(u32, Vec<u32>)> =
            VecDeque::from([(7, vec![7, 70, 700]), (8, vec![8])]);
        let at = first_page(
            &mut sessions,
            (7, vec![7]),
            |held| held.0 == 7,
            |held| held.0 == 7,
        );
        assert_eq!(sessions.len(), 2, "no second session for one snapshot");
        assert_eq!(
            sessions.get(at),
            Some(&(7, vec![7, 70, 700])),
            "viewer A's learned cursors survive viewer B's first page"
        );
        assert_eq!(at, 1, "and the kept session is the most recently used");

        let at = first_page(&mut sessions, (9, vec![9]), |_| false, |held| held.0 == 7);
        assert_eq!(sessions.get(at), Some(&(9, vec![9])));
        assert_eq!(
            sessions,
            VecDeque::from([(8, vec![8]), (9, vec![9])]),
            "a superseded snapshot is dropped"
        );
        for snapshot in 10..20 {
            first_page(&mut sessions, (snapshot, vec![]), |_| false, |_| false);
        }
        assert_eq!(sessions.len(), 8, "still at most eight held");
    }

    /// **A session a viewer is paging through is not evicted by insertion
    /// order.** conc:apicache-2, D-2777. A pinned page moves its session to
    /// the back, so eight first pages of other searches evict older, idle
    /// sessions first.
    #[test]
    fn a_pinned_page_keeps_its_session_from_being_evicted_first() {
        let mut sessions: VecDeque<u32> = (0..8).collect();
        let at = most_recent(&mut sessions, 0);
        assert_eq!((at, sessions.get(at)), (7, Some(&0)), "moved to the back");
        for other in 100..107 {
            first_page(&mut sessions, other, |_| false, |_| false);
        }
        assert!(
            sessions.contains(&0),
            "seven newer first pages evict the idle sessions, not the one in use"
        );
        assert_eq!(sessions.len(), 8);
    }
    /// expr-3, D-2576: a first page's cold `Reader::open` runs with the
    /// session cache's mutex FREE. The namespace is absent, so the open finds
    /// nothing and the answer is `missing` — what matters is the probe taken
    /// at the open point. On the old `render` the guard was held across the
    /// open and the probe recorded `false`.
    #[test]
    fn a_slow_cold_open_does_not_park_other_detail_permits() -> Result<(), String> {
        let id = "cd".repeat(32);
        let asked = Asked::parse(&format!("identity={id}"))?;
        crate::detail::SLOT_FREE_AT_OPEN.with(|cell| cell.set(None));
        // Absent namespace: `missing`, or a refusal naming the path; either
        // way the open was reached, which is what the probe needs.
        let answer = render(&crate::scratch::path("expr3-cold-unlocked"), &asked);
        assert_eq!(
            crate::detail::SLOT_FREE_AT_OPEN.with(std::cell::Cell::get),
            Some(true),
            "the reader was opened with the session cache locked"
        );
        if let Ok(answer) = answer {
            assert!(answer.contains(r#""status":"missing""#), "{answer}");
        }
        Ok(())
    }
    #[test]
    fn exact_snapshot_and_cursor_pairs_refuse_splicing_or_unbounded_queries() -> Result<(), String>
    {
        let id = "ab".repeat(32);
        let initial = format!("identity={id}");
        assert!(Asked::parse(&initial).is_ok());
        for suffix in [
            "&limit=257",
            "&limit=0",
            "&cursor=1",
            "&snapshot=0",
            "&unknown=1",
            "&limit=01",
            "&identity=bad",
        ] {
            assert!(Asked::parse(&format!("{initial}{suffix}")).is_err());
        }
        let full = format!("{initial}&snapshot=99&snapshot_seal={id}&cursor=88&cursor_seal={id}");
        let asked = Asked::parse(&full)?;
        assert_eq!(asked.cursor.map(|value| value.sequence), Some(88));
        assert!(
            render(
                &crate::scratch::path("unadmitted-expression-snapshot"),
                &asked
            )
            .is_err(),
            "unadmitted snapshot cannot splice a path"
        );
        Ok(())
    }
    #[test]
    fn missing_search_read_creates_nothing_and_exact_progress_is_not_rounded() -> Result<(), String>
    {
        let root =
            std::env::temp_dir().join(format!("brutex-expression-api-{}", std::process::id()));
        let asked = Asked::parse(&format!("identity={}", "ab".repeat(32)))?;
        let missing = render(&root, &asked)?;
        assert!(missing.contains("\"status\":\"missing\""));
        assert!(!root.exists());
        let progress = Progress {
            identity: [1; 32],
            checkpoint: Some(Anchor {
                sequence: 9_007_199_254_740_993,
                seal: [2; 32],
            }),
            work: u64::MAX,
            candidates: 0,
            qualifying: 0,
            rows: 0,
            exhausted: false,
            writer_observed: true,
            interrupted: 1,
        };
        let page = Page {
            rows: Vec::new(),
            links: 1,
            next: Some(Anchor {
                sequence: 1,
                seal: [3; 32],
            }),
        };
        let value: Value =
            serde_json::from_str(&body(&progress, &page, 256)).map_err(|why| why.to_string())?;
        assert_eq!(value.get("work"), Some(&json!(u64::MAX.to_string())));
        assert_eq!(value.get("state"), Some(&json!("writer-observed")));
        assert!(
            value.get("next").is_some_and(|value| !value.is_null()),
            "zero candidate page can continue"
        );
        Ok(())
    }
}
