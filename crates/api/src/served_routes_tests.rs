//! The production router over a real socket, asked what no handler's own test
//! asks it: every pooled door with its pool full and with its blocking task
//! lost, and the refusals a handler answers before it reaches a pool
//! (BASE-COV-API, D-7344).
//!
//! # Why a child process for the full pools
//!
//! A full pool refuses every request in the process for as long as it stands,
//! and other tests of this binary read through the same pools. The child
//! ([`crate::isolated`]) runs one test alone, so its crowds refuse nobody else,
//! and its environment can name the store root the research doors read through
//! `server::store_dir`. Its lines reach the coverage figures: the environment
//! is inherited, so a coverage run's `LLVM_PROFILE_FILE` follows it.
//!
//! No row here is market data: the store is an empty scratch root.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::future::Future;
use std::net::SocketAddr;
use std::path::Path;

use crate::detail::Named;
use crate::server::{Site, router_serving, serve};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

/// Names the child's store root.
const CHILD: &str = "BRUTEX_TEST_SERVED_ROUTES_CHILD";
/// An invented identity, canonical lowercase hex.
const ID: &str = "1111111111111111111111111111111111111111111111111111111111111111";

/// One answer: the status, the head lower-cased, the body.
struct Answer {
    status: u16,
    head: String,
    body: String,
}

/// `GET target` on its own connection.
async fn get(addr: SocketAddr, target: &str) -> Answer {
    let mut socket = tokio::net::TcpStream::connect(addr).await.expect("connect");
    socket
        .write_all(
            format!("GET {target} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .await
        .expect("write");
    let mut bytes = Vec::new();
    socket.read_to_end(&mut bytes).await.expect("read");
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    Answer {
        status,
        head: head.to_ascii_lowercase(),
        body: body.to_owned(),
    }
}

/// `body` beside the production router over `root`.
async fn served<F, Fut>(root: &Path, body: F)
where
    F: FnOnce(SocketAddr) -> Fut,
    Fut: Future<Output = ()>,
{
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let router = router_serving(
        std::sync::Arc::new(Site::load(&root.join("masters"), root)),
        std::sync::Arc::new(crate::assets::Assets::new(&root.join("web"))),
        addr,
    );
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let serving = tokio::spawn(serve(
        listener,
        router,
        Box::pin(async move {
            let _ = stopped.await;
            Ok::<(), std::io::Error>(())
        }),
    ));
    body(addr).await;
    let _ = stop.send(());
    serving
        .await
        .expect("the server task")
        .expect("a graceful shutdown is not a failure");
}

/// Every pooled door with a query it reads, by pool.
fn pooled_doors() -> Vec<(Named, String)> {
    let id = format!("identity={ID}");
    let rung = cli::EVERY_RUNG[0];
    let month = "feed=zerodha&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1min&month=2024-01";
    let span = "feed=zerodha&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1min&from=2024-01&to=2024-01";
    let mut doors = Vec::new();
    for (pool, targets) in [
        (
            Named::StoreRead,
            vec![
                "/instruments.json?feed=zerodha".to_owned(),
                "/indexmap.json?feed=zerodha".to_owned(),
                format!("/bars.json?{month}"),
                format!("/bars/export.csv?{span}"),
                format!("/bars/window.json?{span}&limit=10"),
                "/census/facets.json?feed=zerodha".to_owned(),
                "/census/rollup.json?feed=gdfl&segment=FNO&underlying=NIFTY".to_owned(),
                "/imports.json?feed=gdfl".to_owned(),
                "/ingest/refusals.json?feed=zerodha&symbols=NIFTY".to_owned(),
                "/dayflags.json?feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2024-01-01&to=2024-01-31"
                    .to_owned(),
                "/tick-checks.json?year=2024&limit=1&offset=0".to_owned(),
                "/pull/repair.json?feed=zerodha&instrument=NIFTY&timeframe=1min&month=2024-01"
                    .to_owned(),
                "/pull/quarantine.json?symbol=NIFTY&from=2018-03-01&to=2018-03-31&cause=foreign_level"
                    .to_owned(),
                "/pull/recovery.json".to_owned(),
                "/audit.json?feed=zerodha".to_owned(),
            ],
        ),
        (
            Named::List,
            vec!["/feeds.json".to_owned(), "/settings.json".to_owned()],
        ),
        (
            Named::Calendar,
            vec![format!("/gaps.json?{month}")],
        ),
        (
            Named::OpsRead,
            vec![
                "/disk.json".to_owned(),
                "/basis.json?symbol=NIFTY".to_owned(),
                "/scrub.json?feed=gdfl&timeframe=1s".to_owned(),
            ],
        ),
        (Named::Census, vec!["/store.json?feed=zerodha".to_owned()]),
        (Named::Verify, vec!["/verify.json?feed=zerodha".to_owned()]),
        (
            Named::LogRead,
            vec!["/logs.json?limit=5".to_owned(), "/logs".to_owned()],
        ),
        (
            Named::Detail,
            vec![
                format!("/trades.json?{id}"),
                format!("/frontier.json?{id}"),
                format!("/sweep-evidence.json?{id}"),
                format!("/candidate-trades.json?{id}&attempt=1"),
                format!("/expression-search.json?{id}"),
                format!("/boolean-candidates.json?{id}"),
                format!("/boolean-statistics.json?{id}"),
                format!("/boolean-admission.json?{id}"),
                format!("/boolean-qualification.json?{id}"),
                format!("/boolean-campaign.json?{id}"),
                format!("/boolean-qualified-campaign.json?{id}"),
                format!("/boolean-qualified-search.json?{id}"),
                format!("/boolean-oos.json?{id}"),
                "/selection-v6.json?family=NIFTY".to_owned(),
                "/live.json".to_owned(),
                "/backtest.json".to_owned(),
                format!("/index-stop.json?{id}"),
                format!("/index-stop-candles.json?{id}&pin={ID}&setting=0&kind=source"),
                format!("/index-stop-qualification.json?{id}"),
                format!("/index-stop-ranking.json?{id}&timeframe={rung}"),
                format!("/index-stop-vix.json?{id}&pin={ID}&setting=0"),
                "/engine/top.json".to_owned(),
            ],
        ),
    ] {
        doors.extend(targets.into_iter().map(|target| (pool, target)));
    }
    doors
}

/// Asks every pooled door twice, its pool crowded and then its work lost.
async fn full_pools_and_lost_tasks(root: &Path) {
    served(root, |addr| async move {
        let mut wrong = Vec::new();
        for (pool, target) in pooled_doors() {
            let crowd = crate::detail::crowd(pool);
            let full = get(addr, &target).await;
            drop(crowd);
            if full.status != 429 || full.body.is_empty() {
                wrong.push(format!(
                    "{target} full: {} {}",
                    full.status,
                    full.body.chars().take(200).collect::<String>()
                ));
            }
        }
        crate::detail::lose_admitted_work(true);
        for (_, target) in pooled_doors() {
            let lost = get(addr, &target).await;
            if lost.status < 500 || lost.body.is_empty() {
                wrong.push(format!(
                    "{target} lost: {} {}",
                    lost.status,
                    lost.body.chars().take(200).collect::<String>()
                ));
            }
        }
        crate::detail::lose_admitted_work(false);
        // THE SERVER LIVES: a door that lost its task leaves its slot free.
        let after = get(addr, "/feeds.json").await;
        assert_eq!(after.status, 200, "{}{}", after.head, after.body);
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    })
    .await;
}

/// **EVERY POOLED DOOR NAMES A FULL POOL AND A LOST TASK.** A full pool is a
/// `429` with a body that says so, after the admission wait; a blocking task
/// that dies is a server failure with a body that says so, never a `200`, and
/// its slot comes back.
#[test]
fn every_pooled_door_names_a_full_pool_and_a_lost_task() {
    if let Some(root) = std::env::var_os(CHILD) {
        let root = Path::new(&root);
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("a runtime")
            .block_on(full_pools_and_lost_tasks(root));
        std::fs::write(root.join("child-ran"), b"asked").expect("the child's proof");
        return;
    }
    let root = crate::scratch::path("served-routes-pools");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("the child's root");
    let _ = crate::isolated::rerun(
        "served_routes_tests::every_pooled_door_names_a_full_pool_and_a_lost_task",
        &[
            (CHILD, root.as_os_str()),
            ("BRUTEX_STORE", root.as_os_str()),
        ],
    );
    assert_eq!(
        std::fs::read(root.join("child-ran")).ok().as_deref(),
        Some(&b"asked"[..]),
        "the child asked every door"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Names the no-root child's scratch folder.
const NO_ROOT_CHILD: &str = "BRUTEX_TEST_SERVED_ROUTES_NO_ROOT_CHILD";

/// **WITH NO STORE ROOT, THE DOORS THAT READ THE ENVIRONMENT SAY SO.** An
/// empty `BRUTEX_STORE` is refused, never read as the working directory, and
/// this process has no sink: so `/logs.json` has nothing to read and says so
/// with a `503`, the page says it in words, and a research door answers its
/// refusal instead of a page of nothing.
#[test]
fn with_no_store_root_the_doors_that_read_the_environment_say_so() {
    if let Some(root) = std::env::var_os(NO_ROOT_CHILD) {
        let root = Path::new(&root);
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("a runtime")
            .block_on(served(root, |addr| async move {
                let json = get(addr, "/logs.json?limit=5").await;
                assert_eq!(json.status, 503, "{}", json.body);
                let page = get(addr, "/logs").await;
                assert_eq!(page.status, 200, "{}", page.body);
                assert!(page.body.contains("No log"), "{}", page.body);
                let door = get(addr, &format!("/index-stop.json?identity={ID}")).await;
                assert_eq!(door.status, 503, "{}", door.body);
                assert!(door.body.contains("BRUTEX_STORE"), "{}", door.body);
            }));
        std::fs::write(root.join("child-ran"), b"said").expect("the child's proof");
        return;
    }
    let root = crate::scratch::path("served-routes-no-root");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("the child's root");
    let _ = crate::isolated::rerun(
        "served_routes_tests::with_no_store_root_the_doors_that_read_the_environment_say_so",
        &[
            (NO_ROOT_CHILD, root.as_os_str()),
            ("BRUTEX_STORE", std::ffi::OsStr::new("")),
        ],
    );
    assert_eq!(
        std::fs::read(root.join("child-ran")).ok().as_deref(),
        Some(&b"said"[..]),
        "the child asked"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **A DOOR REFUSES WHAT IT CANNOT READ BEFORE IT TAKES A SLOT, THROUGH THE
/// ROUTER.** Extremes: no field at all, a feed this build does not name, a feed
/// of the wrong kind for the door, a span no route answers, a symbol list past
/// its bound and a word with path separators in it, and both a bad feed and a
/// bad span together; every answer is said in words.
#[tokio::test]
async fn a_door_refuses_what_it_cannot_read_through_the_router() {
    let root = crate::scratch::path("served-routes-refusals");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    served(&root, |addr| async move {
        for (target, status, said) in [
            ("/folder.json", 400, "has no folder"),
            ("/folder.json?feed=nosuch", 400, "reads no feed called that"),
            ("/folder.json?feed=zerodha", 400, "has no folder"),
            (
                "/calendar.json?feed=zerodha&span=bogus",
                400,
                "is not a span",
            ),
            (
                "/calendar.json?feed=nosuch&span=bogus",
                400,
                "is not a span",
            ),
            ("/ingest/refusals.json", 400, "error"),
            (
                "/ingest/refusals.json?feed=zerodha&symbols=a%2Fb",
                400,
                "error",
            ),
        ] {
            let answer = get(addr, target).await;
            assert_eq!(answer.status, status, "{target}: {}", answer.body);
            assert!(answer.body.contains(said), "{target}: {}", answer.body);
        }
    })
    .await;
    let _ = std::fs::remove_dir_all(&root);
}
