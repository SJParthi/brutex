#![cfg(test)]
//! WP-81, ops safety, at the routes: a lost sidecar costs `/bars.json` a
//! constant number of bytes (DB-B75); every 429 says when to retry and every
//! answer names its build (DB-B51, DB-B71); a pre-version-4 writer is named on
//! `/store.json` (DB-B70) with the census format beside it (DB-B68); a
//! read-only server refuses every write (DB-B79); `/disk.json` answers (DB-B73).
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "a failed fixture must fail the test by name"
)]

use super::*;
use axum::http::{HeaderMap, Method, StatusCode};
use std::fs;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

/// A private store with masters that do not exist, and its site.
fn site_over(name: &str) -> (PathBuf, Loaded) {
    let root = crate::scratch::path(name);
    let _ = fs::remove_dir_all(&root);
    let masters = root.join("masters");
    fs::create_dir_all(&masters).expect("an owned store root");
    let site = Loaded::new(Site::load(&masters, &root));
    (root, site)
}

/// One NIFTY index one-minute month of `n` invented bars, its `.crc` written
/// by the store's own writer. Prices are paisa no market printed.
fn write_month(root: &Path, month: YearMonth, n: usize) -> PathBuf {
    let path = StorePath::new(PathParts {
        vendor: brutex_core::vendor::Vendor::Dhan,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month,
        file: FileKind::Bars,
    })
    .expect("a legal path");
    let hash = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
    let symbol = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
    let mut file = store::file::BarFile::open_or_create(root, path, symbol).expect("a writer");
    let start = i64::from(
        pull::session::Day::new(month.year(), month.month(), 1)
            .expect("a day")
            .days_from_epoch(),
    ) * 86_400_000_000;
    let rows: Vec<Bar> = (0..n)
        .map(|i| {
            let nth = i64::try_from(i).expect("small");
            Bar {
                ts_micros: start + nth * 60_000_000,
                open: 111_111 + nth,
                high: 111_211 + nth,
                low: 111_011 + nth,
                close: 111_111 + nth,
                volume: 0,
                open_interest: OI_NULL,
            }
        })
        .collect();
    file.append(&rows).expect("the batch appends");
    path.to_path_buf(root)
}

/// `/bars.json` through the handler, asked again on a full pool.
async fn month_get(site: &Loaded, query: &str) -> (StatusCode, String) {
    for _ in 0..400 {
        let uri = format!("/bars.json?{query}").parse().expect("month URI");
        let (status, _headers, body) =
            bars_json(axum::extract::State(Loaded::clone(site)), uri).await;
        if status != StatusCode::TOO_MANY_REQUESTS {
            return (status, body);
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("the store-read pool stayed full for two seconds");
}

/// **ONE LOST SIDECAR COSTS `/bars.json` A CONSTANT NUMBER OF BYTES
/// (DB-B75).** On a clone of the real store, GDFL NIFTY 1s 2020-07 without
/// its `.crc` answered 188,619,461 bytes, one sentence per record. A month of
/// 2,920 records (40 checksum blocks) here would have answered about 2,920
/// copies of the path; now it answers one run naming every record and every
/// block, and a count, in under 2 KiB.
#[tokio::test]
async fn a_month_whose_sidecar_is_lost_answers_one_run_and_a_count() {
    let (root, site) = site_over("ops-crc-lost");
    let bin = write_month(&root, YearMonth::new(2025, 5).expect("m"), 2_920);
    let crc = bin.with_extension("crc");
    assert!(crc.exists(), "the writer sealed the month");
    fs::remove_file(&crc).expect("the sidecar is lost");

    let (status, body) = month_get(
        &site,
        "feed=dhan&exchange=NSE&segment=INDEX&symbol=NIFTY&month=2025-05",
    )
    .await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT, "{body}");
    assert!(body.len() < 2_048, "{} bytes: {body}", body.len());
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("JSON");
    assert_eq!(parsed["bars"], serde_json::json!([]));
    assert_eq!(parsed["faulted_records"], 2_920);
    let faults = parsed["faults"].as_str().expect("named");
    assert!(faults.contains("records 0–2919 (2920)"), "{faults}");
    assert!(
        faults.contains("records 0–2919 (2920), blocks 0–39: "),
        "{faults}"
    );
    assert_eq!(
        faults.matches("2025-05.bin").count(),
        2,
        "the run, then the reason: {faults}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// **EVERY 429 SAYS WHEN TO RETRY, AND EVERY ANSWER NAMES ITS BUILD (DB-B51,
/// DB-B71).** The outermost layer stamps both, so no refusal escapes either:
/// a seek, scan, imports or folder 429 sent no `Retry-After` before.
#[tokio::test]
async fn every_refusal_says_when_and_every_answer_names_its_build() {
    use axum::response::IntoResponse as _;
    let busy = never_framed((StatusCode::TOO_MANY_REQUESTS, "busy").into_response()).await;
    assert_eq!(busy.headers()[axum::http::header::RETRY_AFTER], "1");
    assert_eq!(busy.headers()[&BUILD_HEADER], build_label());
    let own = never_framed(
        (
            StatusCode::TOO_MANY_REQUESTS,
            [(axum::http::header::RETRY_AFTER, "5")],
            "busy",
        )
            .into_response(),
    )
    .await;
    assert_eq!(own.headers()[axum::http::header::RETRY_AFTER], "5", "kept");
    let fine = never_framed((StatusCode::OK, "ok").into_response()).await;
    assert!(!fine.headers().contains_key(axum::http::header::RETRY_AFTER));
    assert_eq!(fine.headers()[&BUILD_HEADER], build_label());
    let label = build_label();
    assert!(
        label == "unstamped" || (label.len() == 40 && label.bytes().all(|b| b.is_ascii_hexdigit())),
        "{label}"
    );
    assert_eq!(cli::commit_stamp().unwrap_or("unstamped"), label);

    // `/health` keeps `ok`/`DEGRADED` on its first line and names the build last.
    let (_root, site) = site_over("ops-health");
    let (_code, body) = health(axum::extract::State(site)).await;
    assert!(body.ends_with(&format!("build {label}\n")), "{body}");
}

/// **A PRE-VERSION-4 WRITER IS NAMED ON `/store.json` (DB-B70).** The legacy
/// single file of a migrated vendor grew past its receipt's length: the state
/// word is `behind`, the drift header and the reason name both lengths. Back
/// at the receipt's length, the header is empty. The census format is named
/// beside it (DB-B68).
#[test]
fn a_legacy_file_written_after_migration_is_named_on_the_census() {
    let (root, site) = site_over("ops-legacy-drift");
    let dir = root.join("manifest/v4/gdfl");
    fs::create_dir_all(&dir).expect("migrated");
    fs::write(
        dir.join("MIGRATED"),
        "format=census-migration-v1\nlegacy_bytes=1000\n",
    )
    .expect("receipt");
    let legacy = root.join("manifest/gdfl.man");
    fs::write(&legacy, vec![0_u8; 1_128]).expect("an old writer appended");

    let uri: axum::http::Uri = "/store.json?feed=gdfl".parse().expect("uri");
    let (_status, headers, _) = store_response(&site, &HeaderMap::new(), &uri, &Method::HEAD);
    let drift = headers["x-brutex-census-legacy-drift"]
        .to_str()
        .expect("text")
        .to_owned();
    assert!(
        drift.contains("1128 bytes") && drift.contains("1000"),
        "{drift}"
    );
    let reason = headers["x-brutex-census-reason"].to_str().expect("text");
    assert!(reason.starts_with("legacy drift:"), "{reason}");
    let state = headers[CENSUS_STATE_HEADER].to_str().expect("text");
    assert!(
        state == "behind" || state == "unreadable",
        "a drifted census is never `held`: {state}"
    );
    assert!(headers.contains_key("x-brutex-census-format"));

    fs::write(&legacy, vec![0_u8; 1_000]).expect("as migrated");
    let (_status, headers, _) = store_response(&site, &HeaderMap::new(), &uri, &Method::HEAD);
    assert_eq!(headers["x-brutex-census-legacy-drift"], "");
    let _ = fs::remove_dir_all(&root);
}

/// **A STORE THAT REQUIRES WHAT THIS BUILD LACKS IS `unreadable`, BY NAME,
/// NEVER `held` (DB-B68).** The rollback the audit reproduced: a build over a
/// store whose census had moved to a format it did not know read the shape it
/// did know and answered `held` with 246,147 of 385,442 months. A line of
/// `manifest/REQUIRES` naming a feature this build does not have makes the
/// census unreadable for the vendor it binds, the reason naming the file, the
/// line and the feature; a vendor it does not bind is untouched.
#[test]
fn a_census_this_build_cannot_read_whole_is_unreadable_and_says_why() {
    let (root, site) = site_over("ops-requires");
    fs::create_dir_all(root.join("manifest")).expect("manifest");
    fs::write(
        root.join("manifest/REQUIRES"),
        "format=store-requires-v1\n\
         requires=census-v4 vendor=gdfl since=bd6b6f5a441aedc492ec19f75ad9d294de413fb4 at=1\n\
         requires=census-v5 vendor=gdfl since=ffff at=2\n",
    )
    .expect("a later migration's line");
    let uri: axum::http::Uri = "/store.json?feed=gdfl".parse().expect("uri");
    let (_status, headers, _) = store_response(&site, &HeaderMap::new(), &uri, &Method::HEAD);
    assert_eq!(headers[CENSUS_STATE_HEADER], "unreadable");
    let said: Vec<&str> = headers
        .values()
        .filter_map(|value| value.to_str().ok())
        .filter(|value| value.contains("census-v5"))
        .collect();
    assert!(
        said.iter().any(|value| value.contains("manifest/REQUIRES")
            && value.contains("line 3")
            && value.contains("this build does not have")),
        "the refusal is named on the answer: {headers:?}"
    );
    let other: axum::http::Uri = "/store.json?feed=zerodha".parse().expect("uri");
    let (_status, headers, _) = store_response(&site, &HeaderMap::new(), &other, &Method::HEAD);
    assert_ne!(
        headers[CENSUS_STATE_HEADER], "unreadable",
        "not bound by it"
    );

    // The census-v4 line alone is one this build honours.
    fs::write(
        root.join("manifest/REQUIRES"),
        "format=store-requires-v1\nrequires=census-v4 vendor=gdfl since=x at=1\n",
    )
    .expect("as the seal writes it");
    assert_eq!(
        pull::ingest::census_store::reader_refusal(&root, brutex_core::vendor::Vendor::Gdfl),
        None
    );
    let _ = fs::remove_dir_all(&root);
}

/// Serves `router` on a fresh loopback port while `body` runs, then stops.
async fn live<F, Fut>(router: axum::Router, body: F)
where
    F: FnOnce(SocketAddr) -> Fut,
    Fut: Future<Output = ()>,
{
    live_at(|_| router, body).await;
}

/// [`live`] for a router that must know the address it is served on.
async fn live_at<R, F, Fut>(router: R, body: F)
where
    R: FnOnce(SocketAddr) -> axum::Router,
    F: FnOnce(SocketAddr) -> Fut,
    Fut: Future<Output = ()>,
{
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let router = router(addr);
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let served = tokio::spawn(serve(
        listener,
        router,
        Box::pin(async move {
            let _ = stopped.await;
            Ok::<(), std::io::Error>(())
        }),
    ));
    body(addr).await;
    let _ = stop.send(());
    let _ = served.await;
}

/// One HTTP/1.1 exchange's status line.
async fn status_of(addr: SocketAddr, method: &str) -> String {
    let request = format!(
        "{method} /w HTTP/1.1\r\nHost: {addr}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
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
    .expect("the client thread")
}

/// **A READ-ONLY SERVER REFUSES EVERY WRITE (DB-B79).** The middleware the
/// production layers install, over a route that would answer both verbs: a
/// read passes, a write is refused by name before the handler; and off, both
/// pass. The variable is `1` and nothing else.
#[tokio::test]
async fn a_read_only_server_refuses_every_write_and_serves_every_read() {
    for on in [true, false] {
        let router = axum::Router::new()
            .route(
                "/w",
                axum::routing::get(|| async { "read" }).post(|| async { "wrote" }),
            )
            .layer(axum::middleware::from_fn(move |request, next| {
                refuse_writes(on, request, next)
            }));
        live(router, |addr| async move {
            assert!(status_of(addr, "GET").await.starts_with("HTTP/1.1 200"));
            let post = status_of(addr, "POST").await;
            if on {
                assert!(post.starts_with("HTTP/1.1 403"), "{post}");
                assert!(post.contains(READ_ONLY_ENV), "{post}");
            } else {
                assert!(post.starts_with("HTTP/1.1 200"), "{post}");
            }
        })
        .await;
    }
    assert!(read_only_from(Some(std::ffi::OsStr::new("1"))));
    for other in ["0", "true", "yes", " 1", ""] {
        assert!(
            !read_only_from(Some(std::ffi::OsStr::new(other))),
            "{other:?}"
        );
    }
    assert!(!read_only_from(None));
    assert!(read_only_refusal(&Method::HEAD, true).is_none());
    assert!(read_only_refusal(&Method::DELETE, true).is_some());
    assert!(read_only_refusal(&Method::DELETE, false).is_none());
}

/// **THE WRITE GATE IS AMONG THE PRODUCTION LAYERS (DB-B79, DB-B71).** The
/// test above mounts the middleware on a router of its own, so nothing
/// failed when the layer was dropped from `admitted`. This one sends a write
/// through `admitted`'s own layers over a real socket: a read-only server
/// answers 403 naming its variable, an unstamped build 403 naming its own,
/// each before the handler; a read passes both; and with neither gate the
/// request is not refused by either.
#[tokio::test]
async fn the_production_layers_refuse_a_write_to_a_read_only_or_unstamped_server() {
    const UNSTAMPED: &str = "this build carries no commit stamp, said the test";
    let (root, site) = site_over("ops-write-gate");
    for (gate, said) in [
        (
            WriteGate {
                read_only: true,
                unstamped: None,
            },
            Some(READ_ONLY_ENV),
        ),
        (
            WriteGate {
                read_only: false,
                unstamped: Some(UNSTAMPED),
            },
            Some(UNSTAMPED),
        ),
        (WriteGate::default(), None),
    ] {
        let reached = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = std::sync::Arc::clone(&reached);
        let table = axum::Router::<Loaded>::new().route(
            "/w",
            axum::routing::get(|| async { "read" }).post(move || {
                counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async { "wrote" }
            }),
        );
        let site = Loaded::clone(&site);
        live_at(
            |addr| admitted_gated(table, site, addr, gate),
            |addr| async move {
                let read = status_of(addr, "GET").await;
                assert!(read.starts_with("HTTP/1.1 200"), "{gate:?}: {read}");
                for method in ["POST", "PUT", "DELETE", "PATCH"] {
                    let wrote = status_of(addr, method).await;
                    match said {
                        Some(word) => {
                            assert!(
                                wrote.starts_with("HTTP/1.1 403"),
                                "{gate:?} {method}: {wrote}"
                            );
                            assert!(wrote.contains(word), "{gate:?} {method}: {wrote}");
                        }
                        None => assert!(
                            !wrote.contains(READ_ONLY_ENV) && !wrote.contains(UNSTAMPED),
                            "{method}: no gate, no gate's refusal: {wrote}"
                        ),
                    }
                }
            },
        )
        .await;
        if said.is_some() {
            assert_eq!(
                reached.load(std::sync::atomic::Ordering::SeqCst),
                0,
                "{gate:?}: no write reached the handler"
            );
        }
        assert_eq!(gate.refuses_writes(), said.is_some());
    }
    // The unstamped arm, by itself: a stamp or the allowance lifts it, and a
    // read is never refused.
    assert_eq!(unstamped_refusal(&Method::POST, None), None);
    assert_eq!(unstamped_refusal(&Method::GET, Some(UNSTAMPED)), None);
    assert_eq!(unstamped_refusal(&Method::HEAD, Some(UNSTAMPED)), None);
    let why = unstamped_refusal(&Method::POST, Some(UNSTAMPED)).expect("refused");
    assert!(why.contains(UNSTAMPED) && why.contains("POST"), "{why}");
    // What the binary arms it with: the one question `cli` asks.
    let stamp = Some("0123456789abcdef0123456789abcdef01234567");
    let allow = std::ffi::OsStr::new("1");
    assert_eq!(cli::unstamped_write_refusal(stamp, None), None);
    assert_eq!(cli::unstamped_write_refusal(None, Some(allow)), None);
    let refusal = cli::unstamped_write_refusal(None, None).expect("refused");
    assert!(refusal.contains(cli::ALLOW_UNSTAMPED_VAR), "{refusal}");
    let _ = fs::remove_dir_all(&root);
}

/// **`/disk.json` ANSWERS FREE SPACE, THE CENSUS'S RETENTION AND THE BACKUP
/// RECEIPT (DB-B73, DB-B59)** through the handler, on an empty store: no
/// receipt is `recorded: false` with the runbook named, never a silence.
#[tokio::test]
async fn disk_json_answers_through_its_handler() {
    let (root, site) = site_over("ops-disk");
    let (status, _headers, body) =
        disk_route::disk_json(axum::extract::State(Loaded::clone(&site))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("JSON");
    assert_eq!(parsed["schema"], 1);
    assert_eq!(parsed["backup"]["recorded"], false);
    assert!(
        parsed["backup"]["why"]
            .as_str()
            .expect("named")
            .contains("docs/36-ops-runbook.md")
    );
    assert!(
        parsed["retention"]
            .as_str()
            .expect("named")
            .contains("never deleted")
    );
    let _ = fs::remove_dir_all(&root);
}

/// **`/scrub.json` MAY REPEAT `key`, AS THE SELECTION IT DESCRIBES DOES
/// (DB-B74).** /db asks for every series on the page in one request. The
/// server-wide guard refused any repeated query field, so a two-series
/// selection — any option chain — was answered 400 `the query names "key"
/// more than once` and the page printed that instead of the scrub's findings
/// (reproduced on a shadow server before this). The route's whole list fits
/// the ordinary target cap; every other field of it still refuses a repeat,
/// and one key past the route's own bound is refused by the route, by name.
#[test]
fn scrub_json_reads_a_list_of_series_and_refuses_any_other_repeat() {
    use std::fmt::Write as _;
    let headers = HeaderMap::new();
    let refused = |target: &str| {
        let uri: axum::http::Uri = target.parse().expect("a target");
        request_bounds_refusal(&uri, &headers)
    };
    assert_eq!(
        refused("/scrub.json?feed=gdfl&timeframe=1s&key=NSE-INDEX-NIFTY&key=NSE-INDEX-BANKNIFTY"),
        None,
        "two series are one request"
    );
    let mut keys = String::new();
    for n in 0..scrub_route::MAX_KEYS {
        let _ = write!(
            keys,
            "&key=NSE-FNO-NIFTY-2019-11-28-{}-PE",
            1_000_000 + n * 5_000
        );
    }
    let most = format!("/scrub.json?feed=gdfl&timeframe=1s{keys}");
    assert!(
        most.len() <= MAX_REQUEST_TARGET_BYTES,
        "{} bytes",
        most.len()
    );
    assert_eq!(refused(&most), None, "the route's most keys fit one target");
    let (code, why) = refused("/scrub.json?feed=gdfl&key=A&key=B&feed=zerodha")
        .expect("a repeated feed is still refused");
    assert_eq!(code, StatusCode::BAD_REQUEST);
    assert!(why.contains("\"feed\""), "{why}");
    assert!(
        refused("/disk.json?key=A&key=B").is_some(),
        "another route's repeated key is still refused"
    );

    // AND THE ROUTE ITSELF ANSWERS THE LIST: each series named once, in the
    // order asked, and one key past its bound refused by its own word.
    let (root, _site) = site_over("ops-scrub-keys");
    let answered = scrub_route::answer(
        &root,
        "feed=gdfl&timeframe=1s&key=NSE-INDEX-NIFTY&key=NSE-INDEX-BANKNIFTY",
    )
    .expect("answered");
    let parsed: serde_json::Value = serde_json::from_str(&answered).expect("JSON");
    assert_eq!(parsed["recorded"], false, "no scrub ran on an empty store");
    let over = format!("feed=gdfl&timeframe=1s{keys}&key=NSE-INDEX-NIFTY");
    let why = scrub_route::answer(&root, &over).expect_err("one key too many");
    assert!(why.starts_with("too_many_series"), "{why}");
    let _ = fs::remove_dir_all(&root);
}
