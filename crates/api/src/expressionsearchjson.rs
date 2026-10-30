//! Bounded read-only observed expression-search progress and exact child links.

use crate::status_rule::Refused;
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
fn anchor(query: &str, number: &str, seal: &'static str) -> Result<Option<Anchor>, String> {
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
/// Serves observed search receipts using bounded background detail workers,
/// through the one status rule's path (D-7550).
pub async fn expression_search_json(uri: Uri) -> Response {
    let root = crate::server::store_dir();
    crate::status_rule::serve(
        Asked::parse(uri.query().unwrap_or_default())
            .map_err(|why| refused(StatusCode::BAD_REQUEST, &why)),
        refused,
        "search detail capacity full; nothing queued",
        move |asked| {
            crate::status_rule::page(
                answer(root, &asked),
                Some(asked.identity),
                "application/json",
                refused,
                "search JSON exceeds its response admission; no prefix exposed",
            )
        },
    )
    .await
}
struct Session {
    root: PathBuf,
    identity: [u8; 32],
    reader: Reader,
}
/// What the search names nothing of, as the not-found sentence says it.
const WHAT: &str = "Expression search";
/// The page, or the one status rule's fault (D-7550): a configured root, then
/// ONE probe of the search's checkpoint namespace before any snapshot or
/// reader is touched -- an identity that names nothing is `404`, pinned or
/// not -- then the read. A snapshot pin no session holds and a namespace the
/// reader finds gone are the same `404`, decided by their typed absence; any
/// other refusal of an entry the probe found is `503`, naming it.
fn answer(
    root: Result<PathBuf, String>,
    asked: &Asked,
) -> Result<String, crate::status_rule::Fault> {
    let root = crate::status_rule::configured(root)?;
    let held = crate::status_rule::probe_identity(
        &root,
        crate::status_rule::namespace::EXPRESSION_SEARCH,
        asked.identity,
        WHAT,
    )?;
    render(&root, asked).map_err(|refused| refused.fault(&root, held.shown, WHAT))
}
/// A snapshot pin no held session names: the pin is not held.
fn unpinned() -> Refused {
    Refused::NotHeld(
        "search snapshot is not admitted or expired; refresh its first page".to_owned(),
    )
}
fn render(root: &Path, asked: &Asked) -> Result<String, Refused> {
    static SESSIONS: OnceLock<Mutex<VecDeque<Session>>> = OnceLock::new();
    let mut sessions = SESSIONS
        .get_or_init(|| Mutex::new(VecDeque::new()))
        .lock()
        .map_err(|_| "search snapshot cache poisoned")?;
    let index = if let Some(snapshot) = asked.snapshot {
        let Some(index) = sessions.iter().position(|held| {
            held.root == root
                && held.identity == asked.identity
                && held.reader.progress().checkpoint == Some(snapshot)
        }) else {
            return Err(unpinned());
        };
        index
    } else {
        let Some(reader) = Reader::open(root, asked.identity, crate::detail::MAX_SCAN_BYTES)?
        else {
            return Err(Refused::Absent);
        };
        sessions.retain(|held| {
            held.root != root
                || held.identity != asked.identity
                || held.reader.progress().checkpoint != reader.progress().checkpoint
        });
        if sessions.len() == 8 {
            sessions.pop_front();
        }
        sessions.push_back(Session {
            root: root.to_path_buf(),
            identity: asked.identity,
            reader,
        });
        sessions.len() - 1
    };
    let held = sessions
        .get_mut(index)
        .ok_or("search snapshot cache entry missing")?;
    let page = held
        .reader
        .page(asked.cursor, asked.limit, crate::detail::MAX_SCAN_BYTES)?;
    Ok(body(held.reader.progress(), &page, asked.limit))
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
            matches!(
                render(
                    &crate::scratch::path("unadmitted-expression-snapshot"),
                    &asked
                ),
                Err(Refused::NotHeld(_))
            ),
            "unadmitted snapshot cannot splice a path"
        );
        // A PIN NO SESSION HOLDS NAMES NOTHING HELD (D-7550): one not-found,
        // never a 503 for the client's own stale pin.
        let root = crate::scratch::path("unadmitted-expression-pin");
        let held = root
            .join(crate::status_rule::namespace::EXPRESSION_SEARCH)
            .join(&id);
        std::fs::create_dir_all(&held).map_err(|why| why.to_string())?;
        let fault = answer(Ok(root.clone()), &asked).err().ok_or("refused")?;
        assert_eq!(fault.class, crate::status_rule::Class::NotFound);
        assert!(
            fault.why.contains("not admitted or expired"),
            "{}",
            fault.why
        );
        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }
    #[test]
    fn missing_search_read_creates_nothing_and_exact_progress_is_not_rounded() -> Result<(), String>
    {
        let root =
            std::env::temp_dir().join(format!("brutex-expression-api-{}", std::process::id()));
        let asked = Asked::parse(&format!("identity={}", "ab".repeat(32)))?;
        assert_eq!(render(&root, &asked), Err(Refused::Absent));
        // THE ROUTE'S ANSWER IS THE ONE NOT-FOUND (D-7550), never a 200.
        let fault = answer(Ok(root.clone()), &asked).err().ok_or("refused")?;
        assert_eq!(fault.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            fault.identity_body(Some(asked.identity))["status"],
            "not_found"
        );
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
