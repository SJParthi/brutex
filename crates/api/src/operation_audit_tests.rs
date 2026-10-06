#![cfg(test)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "Private test fixtures use direct assertions; malformed/missing values must fail the test."
)]
use super::*;
use journal::ID_BASE;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "brutex-api-invocation-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Every route the server registers that the invocation journal does NOT
/// record, each with the reason. A new route must land in [`AUDITED`] or here,
/// by name, or the test below fails. D-1445.
pub(crate) const EXEMPT: &[(&str, &str)] = &[
    ("/dashboard", PAGE),
    ("/instruments", PAGE),
    ("/audit/page", PAGE),
    ("/store", PAGE),
    ("/bars", PAGE),
    ("/logs", PAGE),
    ("/pull", PAGE),
    ("/masters", PAGE),
    ("/masters.js", PAGE),
    ("/typeahead.js", PAGE),
    ("/health", PAGE),
    ("/instruments.json", DATA),
    ("/feeds.json", DATA),
    ("/universes.json", DATA),
    ("/calendar.json", DATA),
    ("/vocab.json", DATA),
    ("/indexmap.json", DATA),
    ("/folder.json", DATA),
    ("/bars.json", DATA),
    ("/gaps.json", DATA),
    ("/bars/window.json", DATA),
    ("/store.json", DATA),
    ("/verify.json", DATA),
    ("/audit.json", DATA),
    ("/logs.json", DATA),
    ("/pull/spot", DATA),
    ("/pull/fno", DATA),
    ("/pull/run", DATA),
    ("/pull/recovery", DATA),
    ("/pull/recovery.json", DATA),
    ("/pull/run.json", DATA),
    ("/pull/run/stop", DATA),
    ("/autopilot.json", DATA),
    ("/universe/resolve", DATA),
    ("/autopilot/pause", DATA),
    ("/autopilot/resume", DATA),
    ("/autopilot/control", DATA),
    ("/ingest/status.json", DATA),
    ("/ingest/queue", DATA),
    ("/masters/refresh", DATA),
    ("/masters/status.json", DATA),
    ("/index-stop.json", INDEX_STOP),
    ("/index-stop-candles.json", INDEX_STOP),
    ("/engine/index-stop-launch.json", INDEX_STOP),
    ("/index-stop-qualification.json", INDEX_STOP),
    ("/index-stop-ranking.json", INDEX_STOP),
    ("/index-stop-vix.json", INDEX_STOP),
    (
        "/backtest/audit.json",
        "the journal's own reader, excluded so inspection cannot recurse (D-0568)",
    ),
    (
        "/inspection.json",
        "the audited router's application-mode probe; not a sweep result or control",
    ),
];
const PAGE: &str = "a page, script or health check; not a sweep result or control";
const DATA: &str = "a store, pull, ingest, master or autopilot route; D-0568 selects sweep \
     result and control requests only";
const INDEX_STOP: &str = "an index-stop read or launch probe no decision has selected for \
     journaling; left open by D-1445, not decided";

/// The registered routes that are not [`EXEMPT`]: what the journal must
/// cover, read off the router rather than off [`AUDITED`].
pub(crate) fn journaled_by_registration() -> Vec<String> {
    registered_routes()
        .into_iter()
        .filter(|route| !EXEMPT.iter().any(|(path, _)| path == route))
        .collect()
}

/// Every path `.route(` registers in the production audited router: the route
/// table and the one extra route `audited_router_serving` adds. Comment lines
/// are skipped first, because the table's own comments quote a `.route(` that
/// is deliberately absent.
pub(crate) fn registered_routes() -> Vec<String> {
    let source = include_str!("server.rs");
    let from = source
        .find("pub fn audited_router_serving(")
        .expect("the audited router is defined");
    let to = from
        + source[from..]
            .find(".fallback(move |request: axum::extract::Request|")
            .expect("the route table ends at its fallback");
    let code: String = source[from..to]
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .flat_map(|line| [line, "\n"])
        .collect();
    let mut routes = Vec::new();
    let mut rest = code.as_str();
    while let Some(at) = rest.find(".route(") {
        rest = rest[at + ".route(".len()..].trim_start();
        let path = rest
            .strip_prefix('"')
            .and_then(|tail| tail.split_once('"'))
            .map(|(path, _)| path)
            .expect("every route is registered with a literal path");
        routes.push(path.to_owned());
    }
    routes
}

#[test]
fn every_registered_route_is_audited_or_exempt_by_name() {
    let routes = registered_routes();
    // A parser that stops matching would pass every check below on nothing.
    assert!(
        routes.len() >= AUDITED.len() + EXEMPT.len(),
        "parsed only {} routes",
        routes.len()
    );
    let mut seen = std::collections::HashSet::new();
    for route in &routes {
        let route = route.as_str();
        assert!(seen.insert(route), "{route} is registered twice");
        let audited = AUDITED.contains(&route);
        let exempt = EXEMPT.iter().any(|(path, _)| *path == route);
        assert!(
            audited != exempt,
            "{route} must be in exactly one of AUDITED and EXEMPT \
             (audited {audited}, exempt {exempt})"
        );
    }
    assert_eq!(routes.len(), AUDITED.len() + EXEMPT.len(), "a stale entry");
    for route in AUDITED {
        assert!(
            routes.iter().any(|r| r == route),
            "{route} is audited, not registered"
        );
        // The label is the list's own static, never the request's bytes. The
        // request is a fresh heap copy, so a label that pointed at it would be
        // caught; comparing against `route` itself compared two copies of one
        // literal, which the compiler may or may not merge, and the coverage
        // build did not (PR #74, run 1261). D-1461.
        let request = String::from(route);
        let found = audited_route(&request).expect("an audited route maps");
        assert_eq!(found, route);
        assert!(!std::ptr::eq(found, request.as_str()));
        assert_eq!(AUDITED.iter().filter(|other| **other == route).count(), 1);
    }
    for (route, why) in EXEMPT {
        assert!(
            routes.iter().any(|r| r == route),
            "{route} is exempt, not registered"
        );
        assert_eq!(audited_route(route), None, "{route}");
        assert!(!why.is_empty());
    }
}

#[test]
fn no_near_spelling_of_an_audited_route_is_audited() {
    for route in AUDITED {
        let upper = route.to_ascii_uppercase();
        let without = &route[1..];
        let near = [
            format!("{route}/"),
            format!("/{route}"),
            format!("{route}?token=secret"),
            format!("{route}#x"),
            format!("{route}\0"),
            format!(" {route}"),
            format!("{route} "),
            format!("/api{route}"),
            route.replacen('/', "//", 1),
            route.replacen('/', "/%2F", 1),
            upper,
            without.to_owned(),
            route[..route.len() - 1].to_owned(),
        ];
        for path in near {
            assert_eq!(audited_route(&path), None, "{path:?}");
        }
    }
    for path in [
        "",
        "/",
        "/backtest/audit.json",
        "/health",
        "/_app/immutable/x.js",
        "/private/secret",
        "/backtest.json?token=secret",
    ] {
        assert_eq!(audited_route(path), None);
    }
    assert_eq!(audited_route(&"/".repeat(1 << 16)), None);
    assert_eq!(
        public_method(&axum::http::Method::from_bytes(b"SECRET").unwrap()),
        "OTHER"
    );
}

#[test]
fn queries_reject_aliases_duplicates_overflow_and_mixed_exact_pages() {
    for query in [
        "&",
        "before=2&",
        "before=2&&limit=1",
        "invocation=0",
        "invocation=01",
        "invocation=18446744073709551616",
        "invocation=1&invocation=2",
        "before=1&before=2",
        "limit=0",
        "limit=33",
        "invocation=1&limit=1",
        "query=secret",
        "before",
        "before=%31",
        "before=-1",
    ] {
        assert!(parse(query).is_err(), "{query}");
    }
    assert!(matches!(
        parse("invocation=18446744073709551615").unwrap(),
        Asked::Exact(u64::MAX)
    ));
    assert!(matches!(
        parse("").unwrap(),
        Asked::Page {
            before: None,
            limit: 32
        }
    ));
    let before = ID_BASE + 4;
    assert!(matches!(
        parse(&format!("before={before}&limit=2")).unwrap(),
        Asked::Page {
            before: Some(b),
            limit: 2
        } if b == before
    ));
    // An id at or below `ID_BASE` names no durable record: a caller error
    // (400), never the journal's 503 (Z1-slice13-F3, D-1762).
    for query in [
        "invocation=5".to_owned(),
        "before=5".to_owned(),
        format!("invocation={ID_BASE}"),
        format!("before={ID_BASE}"),
    ] {
        assert_eq!(
            parse(&query).err().as_deref(),
            Some("audit IDs must lie in the durable invocation namespace"),
            "{query}"
        );
    }
    assert!(matches!(
        parse(&format!("invocation={}", ID_BASE + 1)).unwrap(),
        Asked::Exact(id) if id == ID_BASE + 1
    ));
}

#[test]
fn persistent_projection_keeps_zero_missing_and_foreign_origins_distinct() {
    let root = Scratch::new();
    let mut browser = journal::begin(&root.0, Origin::Browser, "sweep").unwrap();
    let start: serde_json::Value =
        serde_json::from_str(&persisted_status(&root.0, browser.id()).unwrap().unwrap()).unwrap();
    assert_eq!(start["running"]["status"], "unconfirmed");
    assert_eq!(start["running"]["in_flight"], false);
    assert_eq!(start["running"]["audit"]["completed_boundaries"], "0");
    assert!(start["running"]["audit"]["total_boundaries"].is_null());
    browser.finish(Phase::Refused, 0).unwrap();
    let finish: serde_json::Value =
        serde_json::from_str(&persisted_status(&root.0, browser.id()).unwrap().unwrap()).unwrap();
    assert_eq!(finish["running"]["status"], "refused");
    assert!(!finish["running"]["refusal"].is_null());
    assert_eq!(persisted_status(&root.0, ID_BASE + 999).unwrap(), None);
    let http = journal::begin(&root.0, Origin::Http, "GET /backtest.json").unwrap();
    assert!(persisted_status(&root.0, http.id()).is_err());
    let page: serde_json::Value = serde_json::from_str(
        &render(
            &root.0,
            &Asked::Page {
                before: None,
                limit: 1,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(page["records"][0]["invocation"], (ID_BASE + 2).to_string());
    assert_eq!(page["next_before"], (ID_BASE + 2).to_string());
}

#[tokio::test]
async fn successful_and_refused_handlers_leave_durable_separate_request_records() {
    // `request_audited` journals through a detail slot, so a test holding
    // every slot at the same moment answered this one 429. Observed once in a
    // full run; kept apart from the slot owners as the admission tests are.
    let _apart = crate::detail::apart_from_slot_owners().await;
    let root = Scratch::new();
    for (status, expected) in [
        (StatusCode::OK, Phase::Completed),
        (StatusCode::ACCEPTED, Phase::Completed),
        (StatusCode::BAD_REQUEST, Phase::Refused),
        (StatusCode::SERVICE_UNAVAILABLE, Phase::Failed),
    ] {
        let response = request_audited(
            root.0.clone(),
            "GET /backtest.json".to_owned(),
            async move { status.into_response() },
        )
        .await;
        assert_eq!(response.status(), status);
        let id = response
            .headers()
            .get("x-brutex-request-audit")
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        let record = journal::read(&root.0, id).unwrap().unwrap();
        assert_eq!(record.phase, expected);
        assert_eq!(record.response_status, status.as_u16());
        assert_eq!(record.origin, Origin::Http);
    }
}

#[tokio::test]
async fn failed_audit_start_never_polls_the_handler() {
    // `request_audited` journals through a detail slot, so this is kept apart
    // from the tests that hold every slot, as the first test that calls it is.
    // D-0695.
    let _apart = crate::detail::apart_from_slot_owners().await;
    let root = Scratch::new();
    let called = AtomicUsize::new(0);
    let response = request_audited(
        root.0.join("missing"),
        "POST /engine/command".to_owned(),
        async {
            called.fetch_add(1, Ordering::Relaxed);
            StatusCode::ACCEPTED.into_response()
        },
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(called.load(Ordering::Relaxed), 0);
    let body = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["handler_completed"], false);
}

#[tokio::test]
async fn cancelled_request_records_cancellation_and_never_completed() {
    // `request_audited` journals through a detail slot, so this is kept apart
    // from the tests that hold every slot, as the first test that calls it is.
    // D-0695.
    let _apart = crate::detail::apart_from_slot_owners().await;
    let root = Scratch::new();
    let entered = Arc::new(tokio::sync::Notify::new());
    let notify = Arc::clone(&entered);
    let path = root.0.clone();
    let task = tokio::spawn(async move {
        request_audited(path, "GET /trades.json".to_owned(), async move {
            notify.notify_one();
            std::future::pending::<axum::response::Response>().await
        })
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), entered.notified())
        .await
        .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    // The `Cancelled` write now runs on the blocking pool (log-2, D-2577), so
    // it is awaited rather than assumed to have happened inside the abort.
    let mut phase = None;
    for _ in 0..500 {
        phase = journal::read(&root.0, ID_BASE + 1)
            .unwrap()
            .map(|record| record.phase);
        if phase.is_some_and(|phase| phase.terminal()) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(phase, Some(Phase::Cancelled));
}

/// log-2, D-2577: an armed value dropped on an async worker is dropped on the
/// blocking pool instead — a different thread — and outside a runtime it is
/// dropped where it stands. On the old middleware the armed `Attempt` was
/// dropped directly by the future, on the worker thread itself: this probe's
/// `Drop` would have run on the test's own thread.
#[tokio::test(flavor = "current_thread")]
async fn a_dropped_audited_request_finishes_its_attempt_off_the_worker() {
    struct Probe(std::sync::mpsc::Sender<std::thread::ThreadId>);
    impl Drop for Probe {
        fn drop(&mut self) {
            let _sent = self.0.send(std::thread::current().id());
        }
    }
    let worker = std::thread::current().id();
    let (sent, seen) = std::sync::mpsc::channel();
    drop(super::DropOffWorker(Some(Probe(sent.clone()))));
    let dropped_on = tokio::task::spawn_blocking(move || {
        seen.recv_timeout(std::time::Duration::from_secs(10))
    })
    .await
    .unwrap()
    .unwrap();
    assert_ne!(dropped_on, worker, "the armed value was dropped on the worker");
    // An emptied guard drops nothing; outside a runtime the value drops here.
    drop(super::DropOffWorker::<Probe>(None));
    let (outside, seen_outside) = std::sync::mpsc::channel();
    let here = std::thread::spawn(move || {
        super::drop_off_worker(Probe(outside));
        std::thread::current().id()
    })
    .join()
    .unwrap();
    assert_eq!(
        seen_outside
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap(),
        here
    );
    drop(sent);
    // And the middleware itself: a dropped request still journals `Cancelled`.
    let _apart = crate::detail::apart_from_slot_owners().await;
    let root = Scratch::new();
    let path = root.0.clone();
    let cut = tokio::time::timeout(
        std::time::Duration::from_millis(200),
        request_audited(path, "GET /trades.json".to_owned(), async move {
            std::future::pending::<axum::response::Response>().await
        }),
    )
    .await;
    assert!(cut.is_err(), "the handler never answers, so the request is cut");
    let mut phase = None;
    for _ in 0..500 {
        phase = journal::read(&root.0, ID_BASE + 1)
            .unwrap()
            .map(|record| record.phase);
        if phase.is_some_and(|phase| phase.terminal()) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(phase, Some(Phase::Cancelled));
}

/// P3-01-03, D-1973. A write route whose client goes away keeps running to its
/// real terminal: a launch's admission cannot be cancelled, so the record must
/// not say `Cancelled` while the handler can still dispatch.
#[tokio::test]
async fn a_write_whose_client_goes_away_records_the_handlers_real_outcome() {
    let _apart = crate::detail::apart_from_slot_owners().await;
    let root = Scratch::new();
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let (notify, gate) = (Arc::clone(&entered), Arc::clone(&release));
    let path = root.0.clone();
    let connection = tokio::spawn(async move {
        super::request_audited_detached(path, "POST /backtest/run".to_owned(), async move {
            notify.notify_one();
            gate.notified().await;
            (StatusCode::ACCEPTED, "launched").into_response()
        })
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), entered.notified())
        .await
        .unwrap();
    // THE CLIENT GOES AWAY mid-admission: hyper drops the handler's future.
    connection.abort();
    assert!(connection.await.unwrap_err().is_cancelled());
    release.notify_one();
    let mut record = None;
    for _ in 0..500 {
        let seen = journal::read(&root.0, ID_BASE + 1).unwrap().unwrap();
        if seen.phase.terminal() {
            record = Some(seen);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let record = record.expect("the handler's terminal was recorded");
    assert_eq!(record.phase, Phase::Completed);
    assert_eq!(record.response_status, 202);
}

#[tokio::test]
async fn a_busy_journal_start_is_retryable_without_dispatching_the_handler() {
    // `request_audited` journals through a detail slot, so this is kept apart
    // from the tests that hold every slot, as the first test that calls it is.
    // D-0695.
    let _apart = crate::detail::apart_from_slot_owners().await;
    let root = Scratch::new();
    drop(journal::begin(&root.0, Origin::Http, "GET /backtest.json").unwrap());
    let index = std::fs::File::open(root.0.join("audit/invocations-v1/index.bin")).unwrap();
    index.try_lock().unwrap();
    let called = AtomicUsize::new(0);
    let response = request_audited(root.0.clone(), "GET /backtest.json".to_owned(), async {
        called.fetch_add(1, Ordering::Relaxed);
        StatusCode::OK.into_response()
    })
    .await;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(called.load(Ordering::Relaxed), 0);
    let site = Arc::new(crate::server::Site::load(&root.0.join("masters"), &root.0));
    let (status, _, body) = audit_json(
        axum::extract::State(site),
        format!("/backtest/audit.json?invocation={}", ID_BASE + 1)
            .parse()
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "invocation_audit_read_unavailable");
    assert!(body.get("handler_completed").is_none());
    assert_eq!(index.metadata().unwrap().len(), 256);
    index.unlock().unwrap();
}

#[tokio::test]
async fn failed_terminal_says_that_the_handler_already_ran() {
    use std::io::Write as _;
    // `request_audited` journals through a detail slot, so this is kept apart
    // from the tests that hold every slot, as the first test that calls it is.
    // D-0695.
    let _apart = crate::detail::apart_from_slot_owners().await;
    let root = Scratch::new();
    let path = root
        .0
        .join(format!("audit/invocations-v1/{:020}.bin", ID_BASE + 1));
    let response = request_audited(
        root.0.clone(),
        "POST /engine/command".to_owned(),
        async move {
            std::fs::OpenOptions::new()
                .append(true)
                .open(path)
                .unwrap()
                .write_all(&[1])
                .unwrap();
            StatusCode::ACCEPTED.into_response()
        },
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["handler_completed"], true);
    assert!(body["why"].as_str().unwrap().contains("already ran"));
    assert!(journal::read(&root.0, ID_BASE + 1).is_err());
}

/// Only a temporary loopback server and private store are used. This does not
/// enter the production startup/recovery path or read the operator's store.
#[tokio::test]
async fn production_router_wires_durable_request_audit_and_its_read_only_reader() {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    // THE ROUTER JOURNALS EACH AUDITED REQUEST THROUGH A DETAIL SLOT TOO, and
    // this test was not kept apart from the tests that hold every slot. Under a
    // full `api` lib run a review saw it answered 429, "bounded request audit
    // capacity is full", in 2 of 19 runs, and it passed alone. A probe that
    // held every slot got that 429 from this same request on each of five
    // runs, and 400 once the slots were released. D-0695.
    let _apart = crate::detail::apart_from_slot_owners().await;
    let root = Scratch::new();
    let site = Arc::new(crate::server::Site::load(&root.0.join("masters"), &root.0));
    let front = Arc::new(crate::assets::Assets::new(&root.0.join("web")));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = crate::server::audited_router_serving(site, front, address);
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });

    let body = r#"{"token":"private-secret"}"#;
    let request = format!(
        "POST /backtest/run?token=private-query HTTP/1.1\r\nHost: {address}\r\nOrigin: http://{address}\r\nConnection: close\r\nAuthorization: private-header\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
    socket.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    socket.read_to_string(&mut response).await.unwrap();
    assert!(response.contains("400 Bad Request"), "{response}");
    assert!(response.contains("x-brutex-request-audit:"), "{response}");
    let rows = journal::page(&root.0, None, 32).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].label, "POST /backtest/run");
    assert_eq!(rows[0].phase, Phase::Refused);
    assert_eq!(rows[0].response_status, 400);
    let bytes = std::fs::read(
        root.0
            .join(format!("audit/invocations-v1/{:020}.bin", rows[0].id)),
    )
    .unwrap();
    assert!(!bytes.windows(7).any(|bytes| bytes == b"private"));

    let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
    socket.write_all(format!("GET /backtest/audit.json?invocation={} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n", rows[0].id).as_bytes()).await.unwrap();
    let mut response = String::new();
    socket.read_to_string(&mut response).await.unwrap();
    assert!(response.contains("200 OK"), "{response}");
    assert!(response.contains("POST /backtest/run"), "{response}");
    assert_eq!(
        journal::page(&root.0, None, 32).unwrap().len(),
        1,
        "reading the audit creates no new invocation"
    );
    stop.send(()).unwrap();
    server.await.unwrap();
}

#[test]
fn oldest_invocation_ends_the_exact_newest_first_cursor() {
    let root = Scratch::new();
    let read_page = |before| -> serde_json::Value {
        serde_json::from_str(&render(&root.0, &Asked::Page { before, limit: 1 }).unwrap()).unwrap()
    };
    let empty = read_page(None);
    assert_eq!(empty["records"], serde_json::json!([]));
    assert_eq!(empty.get("next_before"), Some(&serde_json::Value::Null));

    let mut attempt = journal::begin(&root.0, Origin::Browser, "sweep").unwrap();
    assert_eq!(attempt.id(), ID_BASE + 1);
    attempt.finish(Phase::Completed, 0).unwrap();

    let last = read_page(None);
    assert_eq!(last["records"].as_array().unwrap().len(), 1);
    assert_eq!(last["records"][0]["invocation"], (ID_BASE + 1).to_string());
    assert_eq!(
        last.get("next_before"),
        Some(&serde_json::Value::Null),
        "the oldest invocation must end the page cursor"
    );
    let after = read_page(Some(ID_BASE + 1));
    assert_eq!(after["records"], serde_json::json!([]));
    assert_eq!(after.get("next_before"), Some(&serde_json::Value::Null));
}

/// **A TERMINAL OWED WHILE EVERY DETAIL SLOT IS TAKEN STILL RECORDS THE
/// HANDLER'S REAL OUTCOME, AND THE CLIENT GETS THE HANDLER'S REAL ANSWER.**
///
/// The handler saturates the shared pool itself, after the start was written
/// and before the terminal runs: the shape four concurrent detail reads give a
/// `POST /backtest/run` that already launched its engine task. Until D-1445 the
/// terminal's `detail::run` answered `Saturated`, the armed attempt was dropped,
/// its `Drop` wrote `Cancelled`/0 on the Tokio worker, and the client got a 503
/// saying the handler already ran. Every outcome class is tried, a handler's
/// own 429 included, and the slots are proven saturated before each terminal.
#[tokio::test]
async fn a_terminal_owed_while_every_slot_is_taken_records_the_handlers_real_outcome() {
    let apart = crate::detail::apart_from_slot_owners().await;
    let root = Scratch::new();
    for (status, expected) in [
        (StatusCode::OK, Phase::Completed),
        (StatusCode::ACCEPTED, Phase::Completed),
        (StatusCode::NO_CONTENT, Phase::Completed),
        (StatusCode::BAD_REQUEST, Phase::Refused),
        (StatusCode::TOO_MANY_REQUESTS, Phase::Refused),
        (StatusCode::INTERNAL_SERVER_ERROR, Phase::Failed),
        (StatusCode::SERVICE_UNAVAILABLE, Phase::Failed),
    ] {
        let held = std::sync::Mutex::new(Vec::new());
        let response = request_audited(root.0.clone(), "POST /backtest/run".to_owned(), async {
            let mut taken = crate::detail::take_every_free_slot(&apart);
            assert!(!taken.is_empty(), "the begin's own slot was returned");
            held.lock().unwrap().append(&mut taken);
            // Proof the pool is full at the moment the terminal is owed.
            assert_eq!(
                crate::detail::run(|| ()).await,
                Err(crate::detail::RunError::Saturated)
            );
            (status, "the handler's own body").into_response()
        })
        .await;
        assert_eq!(response.status(), status, "the handler's answer is kept");
        let id: u64 = response
            .headers()
            .get("x-brutex-request-audit")
            .expect("the terminal settled and stamped the id")
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        assert_eq!(&body[..], b"the handler's own body");
        let record = journal::read(&root.0, id).unwrap().unwrap();
        assert_eq!(record.phase, expected, "{status}");
        assert_eq!(record.response_status, status.as_u16());
        assert_eq!(record.label, "POST /backtest/run");
        // The owed slot was returned; only the test's own are still held.
        assert_eq!(
            crate::detail::run(|| ()).await,
            Err(crate::detail::RunError::Saturated)
        );
        held.lock().unwrap().clear();
        assert_eq!(crate::detail::run(|| 7).await, Ok(7));
    }
    let rows = journal::page(&root.0, None, 32).unwrap();
    assert_eq!(rows.len(), 7);
    assert!(rows.iter().all(|row| row.phase != Phase::Cancelled));
}

/// **AN OWED TASK IS NEVER REFUSED, AND IT COUNTS AGAINST NEW WORK WHILE IT
/// RUNS.** With every free slot taken, `run_owed` still runs; while it runs, a
/// slot freed by the test is not enough for `run`, because the owed task holds
/// one past the cap. Once it returns, everything it took is back. D-1445.
#[tokio::test]
async fn an_owed_task_runs_past_a_full_pool_and_counts_while_it_runs() {
    let apart = crate::detail::apart_from_slot_owners().await;
    let mut taken = crate::detail::take_every_free_slot(&apart);
    assert!(!taken.is_empty());
    assert_eq!(
        crate::detail::run(|| ()).await,
        Err(crate::detail::RunError::Saturated)
    );
    let (entered, inside) = std::sync::mpsc::channel();
    let (release, wait) = std::sync::mpsc::channel::<()>();
    let owed = tokio::spawn(crate::detail::run_owed(move || {
        entered.send(()).unwrap();
        wait.recv().unwrap();
        "owed"
    }));
    tokio::task::spawn_blocking(move || inside.recv().unwrap())
        .await
        .unwrap();
    // One slot back from the test: the owed task's past-the-cap slot still
    // fills the pool, so new work is refused.
    drop(taken.pop());
    assert_eq!(
        crate::detail::run(|| ()).await,
        Err(crate::detail::RunError::Saturated)
    );
    release.send(()).unwrap();
    assert_eq!(owed.await.unwrap(), Ok("owed"));
    // Now the slot the test returned is free again.
    assert_eq!(crate::detail::run(|| 1).await, Ok(1));
    drop(taken);
    for _ in 0..crate::detail::MAX_CONCURRENT * 3 {
        assert_eq!(crate::detail::run_owed(|| 2).await, Ok(2));
    }
    assert_eq!(crate::detail::run(|| 3).await, Ok(3));
}

/// **WHAT ONE AUDITED REQUEST LEAVES ON DISK, MEASURED.** Each finished
/// request adds one 256-byte start to `index.bin` and one new file of exactly
/// two 256-byte records (start and terminal) in the one flat directory, and
/// nothing is ever removed: the directory's entry count and the bytes on disk
/// grow with every request ever served, which `docs/06-limits.md` states under
/// D-1445. Reading a page stays at most 32 rows however many exist.
#[tokio::test]
async fn each_audited_request_adds_one_file_and_one_index_slot_and_nothing_is_removed() {
    let _apart = crate::detail::apart_from_slot_owners().await;
    let root = Scratch::new();
    let dir = root.0.join("audit/invocations-v1");
    for served in 1..=40_u64 {
        let response = request_audited(root.0.clone(), "GET /live.json".to_owned(), async {
            StatusCode::OK.into_response()
        })
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(
            u64::try_from(names.len()).unwrap(),
            served + 1,
            "one file per request, plus the index"
        );
        assert_eq!(
            std::fs::metadata(dir.join("index.bin")).unwrap().len(),
            256 * served
        );
        let own = dir.join(format!("{:020}.bin", ID_BASE + served));
        assert_eq!(std::fs::metadata(own).unwrap().len(), 512);
        assert!(
            (1..=served).all(|ordinal| dir.join(format!("{:020}.bin", ID_BASE + ordinal)).exists()),
            "no earlier record is rotated away"
        );
    }
    assert_eq!(journal::page(&root.0, None, 32).unwrap().len(), 32);
}

/// **THE GROWTH IS WRITTEN WHERE A LIMIT IS LOOKED FOR.** `docs/06-limits.md`
/// had no line for the journal's per-request growth; D-0568 said only that
/// storage grows with history. W1-api3-0, D-1445.
#[test]
fn the_journals_per_request_growth_is_stated_in_the_limits() {
    let limits = include_str!("../../../docs/06-limits.md");
    let section = limits
        .split("\n## ")
        .find(|section| section.contains("D-1445"))
        .expect("a D-1445 section in docs/06-limits.md");
    // Line breaks and sentence case are the document's to choose.
    let section = section
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    for phrase in [
        "audit/invocations-v1/",
        "one new file per audited request",
        "256 bytes",
        "no retention, rotation or sharding",
        "directory's entry count",
        "run_owed",
        "not bounded by `max_concurrent`",
    ] {
        assert!(section.contains(phrase), "missing {phrase:?}");
    }
    for route in AUDITED {
        assert!(section.contains(&format!("`{route}`")), "{route}");
        assert_eq!(route, route.to_lowercase(), "compared lower-cased");
    }
}

/// BT-12 (P12-02, D-1791): `/backtest` is NOT a registered route and
/// `/backtest.json` is. A registered route beats the router's fallback
/// unconditionally, so a Rust page at `/backtest` would make a click and a
/// reload render two different applications. Read off the same production
/// route table `every_registered_route_is_audited_or_exempt_by_name` reads.
#[test]
fn the_backtest_page_is_not_a_registered_route_and_its_json_is() {
    let routes = registered_routes();
    assert!(
        routes.iter().any(|route| route == "/backtest.json"),
        "the parser found the table: {routes:?}"
    );
    assert!(
        !routes.iter().any(|route| route == "/backtest"),
        "`/backtest` belongs to the front end's fallback: {routes:?}"
    );
}
