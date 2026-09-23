#![cfg(test)]
//! The browser-facing hardening of D-0687, over real sockets and a private
//! store: every answer refuses framing and sniffing, every method must name
//! this listener, and a refused or cross-site request to a journaled route
//! never reaches the durable invocation journal.
#![expect(
    clippy::expect_used,
    reason = "a failed fixture or socket must fail the test by name"
)]

use super::*;
use cli::operation_audit as journal;

/// One HTTP/1.1 exchange on a fresh connection, blocking on its own thread —
/// the shape the route tests in `server.rs` use.
async fn ask(addr: SocketAddr, method: &str, path: &str, host: &str, extra: &str) -> String {
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\n{extra}Content-Length: 0\r\n\
         Connection: close\r\n\r\n"
    );
    tokio::task::spawn_blocking(move || {
        use std::io::{Read as _, Write as _};
        let mut socket = std::net::TcpStream::connect(addr).expect("connect");
        socket.write_all(request.as_bytes()).expect("write");
        let mut answer = String::new();
        socket.read_to_string(&mut answer).expect("read");
        answer
    })
    .await
    .expect("the client thread must not panic")
}

/// [`ask`], for a request that reaches the audit layer.
///
/// The audit layer's blocking work shares `detail::MAX_CONCURRENT` slots with
/// every other test in this binary running in parallel, and a full pool answers
/// a documented, retryable `429` BEFORE any record is begun. That answer — and
/// only that one — carries no `x-brutex-request-audit` id, so asking again
/// cannot add a journal row. Any other answer, a handler's own `429` included,
/// is returned as it came. Bounded: after the last attempt the busy answer is
/// returned and the caller's assertion names it.
async fn ask_journaled(
    addr: SocketAddr,
    method: &str,
    path: &str,
    host: &str,
    extra: &str,
) -> String {
    let mut answer = String::new();
    for _ in 0..500 {
        answer = ask(addr, method, path, host, extra).await;
        let busy = status_line(&answer).contains("429")
            && !head_of(&answer).contains("x-brutex-request-audit");
        if !busy {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    answer
}

/// A private store root, empty and owned by this test.
fn owned_root(name: &str) -> PathBuf {
    let root = crate::scratch::path(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("an owned store root");
    root
}

/// Serves `build(addr)` on a fresh loopback port while `body` runs, then stops.
async fn live<B, F, Fut>(build: B, body: F)
where
    B: FnOnce(SocketAddr) -> axum::Router,
    F: FnOnce(SocketAddr) -> Fut,
    Fut: Future<Output = ()>,
{
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let served = tokio::spawn(serve(
        listener,
        build(addr),
        Box::pin(async move {
            let _ = stopped.await;
            Ok::<(), std::io::Error>(())
        }),
    ));
    body(addr).await;
    let _ = stop.send(());
    served
        .await
        .expect("task")
        .expect("a graceful shutdown is not a failure");
}

/// The audited production router over `store`, with masters and front end
/// that do not exist — nothing here needs either.
fn audited(root: &Path, store: &Path, addr: SocketAddr) -> axum::Router {
    audited_router_serving(
        Loaded::new(Site::load(&root.join("masters"), store)),
        std::sync::Arc::new(assets::Assets::new(&root.join("web"))),
        addr,
    )
}

/// The response head, lower-cased, so header names and values compare exactly
/// whatever case the wire used.
fn head_of(answer: &str) -> String {
    answer
        .split_once("\r\n\r\n")
        .map_or(answer, |(head, _)| head)
        .to_ascii_lowercase()
}

/// The three headers, each present exactly once with its exact value.
fn assert_never_framed(what: &str, answer: &str) {
    let head = head_of(answer);
    for line in [
        "\r\nx-frame-options: deny",
        "\r\ncontent-security-policy: frame-ancestors 'none'",
        "\r\nx-content-type-options: nosniff",
    ] {
        assert_eq!(
            head.matches(line).count(),
            1,
            "{what} must carry {line:?} exactly once: {answer}"
        );
    }
}

fn status_line(answer: &str) -> &str {
    answer.lines().next().unwrap_or("")
}

/// A `403` from the admission layer: no audit id was stamped on it, because
/// the request never reached the audit layer.
fn refused(why: &str, answer: &str) {
    assert!(status_line(answer).contains("403"), "{why}: {answer}");
    assert!(
        !head_of(answer).contains("x-brutex-request-audit"),
        "{why}: {answer}"
    );
}

/// **EVERY ANSWER REFUSES TO BE FRAMED OR SNIFFED, whoever produced it.**
///
/// A registered route, the fallback, the audited router's own extra route, a
/// method the route does not take, both admission refusals, a journaled read
/// that crossed the audit layer, and the body limit.
#[tokio::test]
async fn every_answer_forbids_framing_and_sniffing() {
    let _apart = crate::detail::apart_from_slot_owners().await;
    let root = owned_root("af07-framing");
    live(
        |addr| audited(&root, &root, addr),
        |addr| async move {
            let local = addr.to_string();
            let local = local.as_str();
            let hostile = format!("evil.example:{}", addr.port());
            let same = "Sec-Fetch-Site: same-origin\r\n";
            let cross = "Sec-Fetch-Site: cross-site\r\n";
            let admitted_read = "an admitted journaled read";
            for (what, method, path, host, extra, status) in [
                ("a route", "GET", "/health", local, "", None),
                ("the fallback", "GET", "/no-such-page", local, "", None),
                (
                    "the audited router's own route",
                    "GET",
                    "/inspection.json",
                    local,
                    "",
                    Some("200"),
                ),
                (
                    "a method the route does not take",
                    "GET",
                    "/pull/spot",
                    local,
                    "",
                    Some("405"),
                ),
                (
                    "a rebound read",
                    "GET",
                    "/health",
                    hostile.as_str(),
                    "",
                    Some("403"),
                ),
                (
                    "a cross-site journaled read",
                    "GET",
                    "/backtest.json",
                    local,
                    cross,
                    Some("403"),
                ),
                (admitted_read, "GET", "/backtest.json", local, same, None),
                ("a HEAD", "HEAD", "/health", local, "", None),
            ] {
                let answer = ask_journaled(addr, method, path, host, extra).await;
                if let Some(status) = status {
                    assert!(status_line(&answer).contains(status), "{what}: {answer}");
                }
                assert_never_framed(what, &answer);
                assert_eq!(
                    head_of(&answer).contains("x-brutex-request-audit:"),
                    what == admitted_read,
                    "{what}: only the admitted journaled read crosses the audit layer: {answer}"
                );
            }

            // THE BODY LIMIT, on an otherwise admitted write.
            let origin = format!("Origin: http://{local}\r\n{same}");
            let huge = format!("pad={}", "x".repeat(MAX_FORM_BYTES + 1));
            let request = format!(
                "POST /ingest/queue HTTP/1.1\r\nHost: {local}\r\n{origin}\
                 Content-Type: application/x-www-form-urlencoded\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{huge}",
                huge.len()
            );
            let too_big = tokio::task::spawn_blocking(move || {
                use std::io::{Read as _, Write as _};
                let mut socket = std::net::TcpStream::connect(addr).expect("connect");
                socket.write_all(request.as_bytes()).expect("write");
                let mut answer = String::new();
                socket.read_to_string(&mut answer).expect("read");
                answer
            })
            .await
            .expect("join");
            assert!(status_line(&too_big).contains("413"), "{too_big}");
            assert_never_framed("the body limit", &too_big);
        },
    )
    .await;
    let _ = std::fs::remove_dir_all(&root);
}

/// **THE AUDIT LAYER'S OWN ANSWER IS NOT FRAMED EITHER, NOR THE PLAIN ROUTER'S.**
///
/// This is the ordering proof. A store root that does not exist makes
/// `journal::begin` refuse, so `note_request` builds a `503` itself, outside
/// every route. It carries the headers only because the header layer wraps the
/// audit layer too.
#[tokio::test]
async fn the_audit_layers_own_refusal_and_the_plain_router_are_never_framed() {
    let _apart = crate::detail::apart_from_slot_owners().await;
    let root = owned_root("af07-framing-outer");
    let absent = root.join("absent-store");
    live(
        |addr| audited(&root, &absent, addr),
        |addr| async move {
            let answer = ask_journaled(
                addr,
                "GET",
                "/backtest.json",
                &addr.to_string(),
                "Sec-Fetch-Site: same-origin\r\n",
            )
            .await;
            assert!(status_line(&answer).contains("503"), "{answer}");
            assert!(answer.contains("invocation_audit_unavailable"), "{answer}");
            assert_never_framed("the audit layer's own 503", &answer);
        },
    )
    .await;
    assert!(!absent.exists(), "a refused audit start creates no store");

    // AND THE PLAIN ROUTER, which read-only adapters serve.
    live(
        |addr| {
            router_serving(
                Loaded::new(Site::load(&root.join("masters"), &root)),
                std::sync::Arc::new(assets::Assets::new(&root.join("web"))),
                addr,
            )
        },
        |addr| async move {
            let answer = ask(addr, "GET", "/no-such-page", &addr.to_string(), "").await;
            assert_never_framed("the plain router's fallback", &answer);
        },
    )
    .await;
    let _ = std::fs::remove_dir_all(&root);
}

/// **A READ MUST NAME THIS LISTENER, AND THE OPERATOR'S BROWSER DOES.**
///
/// Both spellings a browser on this machine sends — `localhost:PORT` and the
/// bound IP — are answered, with no fetch metadata, with `none` for a typed
/// address, and with `cross-site` for a link from elsewhere to a page that is
/// not journaled. A rebound hostname, a wrong port and a repeated `Host` are
/// refused on `GET` and `HEAD` alike, by the admission layer and not a route.
#[tokio::test]
async fn a_read_must_name_this_listener_and_the_operators_browser_does() {
    let root = owned_root("af07-host");
    live(
        |addr| audited(&root, &root, addr),
        |addr| async move {
            let port = addr.port();
            for host in [format!("localhost:{port}"), addr.to_string()] {
                for extra in [
                    "",
                    "Sec-Fetch-Site: none\r\n",
                    "Sec-Fetch-Site: same-origin\r\n",
                    "Sec-Fetch-Site: cross-site\r\n",
                ] {
                    for path in ["/health", "/no-such-page", "/inspection.json"] {
                        let answer = ask(addr, "GET", path, &host, extra).await;
                        assert!(
                            !status_line(&answer).contains("403"),
                            "[{host}] [{extra}] {path} is the operator's own read: {answer}"
                        );
                    }
                }
            }

            for method in ["GET", "HEAD"] {
                for host in [
                    format!("rebind.evil.example:{port}"),
                    format!("localhost:{}", port.wrapping_add(1)),
                    "localhost".to_owned(),
                    format!("localhost:{port}\r\nHost: localhost:{port}"),
                ] {
                    let answer = ask(
                        addr,
                        method,
                        "/inspection.json",
                        &host,
                        "Sec-Fetch-Site: same-origin\r\n",
                    )
                    .await;
                    assert!(
                        status_line(&answer).contains("403"),
                        "{method} [{host}]: {answer}"
                    );
                    if method == "GET" {
                        assert!(
                            answer.contains("Host says this one came from somewhere else"),
                            "{method} [{host}]: {answer}"
                        );
                    }
                }
            }
        },
    )
    .await;
    let _ = std::fs::remove_dir_all(&root);
}

/// **A REFUSED OR CROSS-SITE REQUEST TO A JOURNALED ROUTE OPENS NO JOURNAL.**
///
/// Admission used to run inside the audit layer, so a rebound write left a
/// `Refused` invocation behind its 403, and a cross-site read of saved results
/// — an `<img>` on any page the operator had open — left a completed one.
/// Every refusal below is asserted against the journal DIRECTORY, not a count
/// of records: the first `journal::begin` creates it, so its absence proves no
/// begin ran. Then the three admitted shapes each add exactly one record.
#[tokio::test]
async fn a_refused_or_cross_site_journaled_request_never_opens_the_journal() {
    let _apart = crate::detail::apart_from_slot_owners().await;
    let root = owned_root("af07-journal");
    let journal_dir = root.join("audit");
    let (store, journal_dir) = (&root, &journal_dir);
    live(
        |addr| audited(store, store, addr),
        |addr| async move {
            let local = addr.to_string();
            let hostile = format!("rebind.evil.example:{}", addr.port());

            for method in ["GET", "HEAD"] {
                for path in ["/backtest.json", "/backtest/run.json", "/live.json"] {
                    for value in ["cross-site", "same-site", "nonsense"] {
                        let extra = format!("Sec-Fetch-Site: {value}\r\n");
                        let answer = ask(addr, method, path, &local, &extra).await;
                        refused("another site's read", &answer);
                        if method == "GET" {
                            assert!(
                                answer.contains(&format!(
                                    "Sec-Fetch-Site says this one came from somewhere else: {value}"
                                )),
                                "{answer}"
                            );
                            assert!(
                                answer.contains("no invocation audit record was written"),
                                "{answer}"
                            );
                        }
                    }
                }
            }
            let repeated = ask(
                addr,
                "GET",
                "/backtest.json",
                &local,
                "Sec-Fetch-Site: same-origin\r\nSec-Fetch-Site: same-origin\r\n",
            )
            .await;
            refused("a repeated fetch-site", &repeated);
            assert!(repeated.contains("<malformed-or-repeated>"), "{repeated}");

            // A REBOUND READ AND A REBOUND WRITE, both truthfully same-origin.
            let rebound_read = ask(
                addr,
                "GET",
                "/backtest.json",
                &hostile,
                "Sec-Fetch-Site: same-origin\r\n",
            )
            .await;
            refused("a rebound read", &rebound_read);
            let rebound_write = ask(
                addr,
                "POST",
                "/backtest/run",
                &hostile,
                &format!("Sec-Fetch-Site: same-origin\r\nOrigin: http://{hostile}\r\n"),
            )
            .await;
            refused("a rebound write", &rebound_write);
            let cross_write = ask(
                addr,
                "POST",
                "/backtest/run",
                &local,
                &format!("Sec-Fetch-Site: cross-site\r\nOrigin: http://{local}\r\n"),
            )
            .await;
            refused("a cross-site write", &cross_write);
            assert!(
                !journal_dir.exists(),
                "no refused request may open the invocation journal"
            );

            // THE OPERATOR'S OWN READS ARE STILL JOURNALED, one record each:
            // this page's fetch, a typed address, and a client with no fetch
            // metadata at all.
            for (count, extra) in [
                (1, "Sec-Fetch-Site: same-origin\r\n"),
                (2, "Sec-Fetch-Site: none\r\n"),
                (3, ""),
            ] {
                let answer = ask_journaled(addr, "GET", "/backtest.json", &local, extra).await;
                assert!(!status_line(&answer).contains("403"), "[{extra}] {answer}");
                assert!(
                    head_of(&answer).contains("x-brutex-request-audit:"),
                    "{answer}"
                );
                let rows = journal::page(store, None, 32).expect("a readable journal");
                assert_eq!(rows.len(), count, "[{extra}] one record per admitted read");
                assert!(rows.iter().all(|row| row.label == "GET /backtest.json"));
            }
        },
    )
    .await;
    let _ = std::fs::remove_dir_all(&root);
}

/// **THE JOURNALED-READ RULE, ARM BY ARM, WITHOUT A SOCKET.**
#[test]
fn a_journaled_read_is_refused_only_when_it_names_another_site() {
    let get = axum::http::Method::GET;
    let with = |value: &[u8]| {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::HeaderName::from_static("sec-fetch-site"),
            axum::http::HeaderValue::from_bytes(value).expect("a header value"),
        );
        headers
    };

    // ADMITTED: this page, a typed address, and no metadata at all.
    for headers in [
        with(b"same-origin"),
        with(b"none"),
        axum::http::HeaderMap::new(),
    ] {
        assert_eq!(
            journaled_read_refusal(&get, "/backtest.json", &headers),
            None
        );
    }
    // REFUSED: another site, a sibling site, a token nobody here knows, a
    // value that is not text, and a repeated value.
    for value in ["cross-site", "same-site", "nonsense"] {
        let why = journaled_read_refusal(&get, "/backtest.json", &with(value.as_bytes()))
            .expect("another site's journaled read is refused");
        assert!(
            why.contains(&format!(
                "Sec-Fetch-Site says this one came from somewhere else: {value}"
            )),
            "{why}"
        );
        assert!(why.starts_with("REFUSED — a GET on this server"), "{why}");
    }
    let mut repeated = with(b"same-origin");
    repeated.append(
        axum::http::HeaderName::from_static("sec-fetch-site"),
        axum::http::HeaderValue::from_static("same-origin"),
    );
    for headers in [with(&[0xff_u8]), repeated] {
        let why = journaled_read_refusal(&get, "/boolean-oos.json", &headers)
            .expect("an ambiguous fetch-site is not a pass");
        assert!(
            why.contains(
                "Sec-Fetch-Site says this one came from somewhere else: <malformed-or-repeated>"
            ),
            "{why}"
        );
    }
    // NOT JOURNALED, NOT GATED: a page, the audit reader itself (which the
    // journal excludes so inspection cannot recurse), and the front end.
    for path in [
        "/health",
        "/backtest/audit.json",
        "/",
        "/_app/immutable/x.js",
    ] {
        assert_eq!(
            journaled_read_refusal(&get, path, &with(b"cross-site")),
            None,
            "{path}"
        );
    }
}
