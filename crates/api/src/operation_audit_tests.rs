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

#[test]
fn only_actual_fixed_result_and_control_routes_are_audited() {
    for path in [
        "/backtest/run",
        "/backtest.json",
        "/candidate-trades.json",
        "/boolean-candidates.json",
        "/boolean-statistics.json",
        "/boolean-admission.json",
        "/boolean-qualification.json",
        "/boolean-qualified-campaign.json",
        "/boolean-qualified-search.json",
        "/boolean-oos.json",
    ] {
        assert_eq!(audited_route(path), Some(path));
        assert!(
            include_str!("server.rs").contains(&format!("\"{path}\"")),
            "the route must actually be registered"
        );
    }
    for path in [
        "/backtest/audit.json",
        "/health",
        "/_app/immutable/x.js",
        "/private/secret",
        "/backtest.json?token=secret",
    ] {
        assert_eq!(audited_route(path), None);
    }
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
    assert!(matches!(
        parse("before=4&limit=2").unwrap(),
        Asked::Page {
            before: Some(4),
            limit: 2
        }
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
    assert_eq!(
        journal::read(&root.0, ID_BASE + 1).unwrap().unwrap().phase,
        Phase::Cancelled
    );
}

#[tokio::test]
async fn a_busy_journal_start_is_retryable_without_dispatching_the_handler() {
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
