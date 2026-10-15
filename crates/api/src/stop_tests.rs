//! R3-14 repair 1: a stop with a request still open.
//!
//! A `kill -TERM` became a graceful stop at R3-14, and a graceful stop closes
//! the listener at once while the process goes on for up to
//! [`crate::speed::STOP_BOUND`]: the open requests drain, then the runtime
//! waits for blocking work. The serve lock was held for all of that, so a
//! redeploy that waited for the port to close and then started the new build
//! was REFUSED by the lock, and nothing listened afterwards. And a request the
//! drain cut was counted only when the runtime dropped it, after the last
//! minute had been written, so its count was lost.
//!
//! The test below drives that exact stop in-process: one request that never
//! answers, a stop, the bound, the runtime's end.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "the same exception every test module in this workspace takes; the stop is \
              one sequence, read top to bottom"
)]

use super::*;

/// The `u64` field `key` of `record`, or zero.
fn field_of(record: &telemetry::Record, key: &str) -> u64 {
    record
        .field(key)
        .and_then(telemetry::OwnedValue::as_u64)
        .unwrap_or(0)
}

/// Whether another holder could take `root`'s serve lock right now: a fresh
/// open file description, as another process would make (advisory locks are
/// per description, so this conflicts inside one process too).
fn store_is_free(root: &Path) -> bool {
    let path = root.join(SERVE_LOCK);
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .expect("the lock file");
    store::flock::Flock::try_lock(file, path).is_ok()
}

/// Whether `addr` can be bound right now.
fn port_is_free(addr: std::net::SocketAddr) -> bool {
    std::net::TcpListener::bind(addr).is_ok()
}

/// **A STOP WITH AN OPEN REQUEST KEEPS THE PORT UNTIL THE STORE IS FREE, AND
/// COUNTS THE REQUEST IT CUT** (R3-14 repair 1).
///
/// Fails before the repair on three counts: the port was free the moment the
/// stop began while the store was still held (the redeploy's refusal); the
/// cut line named no route and no count; and the cut request never reached a
/// minute line.
#[test]
fn a_stop_with_an_open_request_keeps_the_port_until_the_store_is_free() {
    let _shared = crate::emitted::sink();
    let mark = crate::emitted::mark();
    let root = crate::scratch::path("stop-open-request");
    std::fs::create_dir_all(&root).expect("mkdir");
    let route = "/stop-test/a-request-that-never-answers";
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("a runtime");
    let (stop, stop_asked) = tokio::sync::oneshot::channel::<()>();
    let (held, addr, served) = runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a port");
        let addr = listener.local_addr().expect("its address");
        let lock = take_serve_lock(&root, addr).expect("a free store");
        let (listener, port) = keep_port(listener).expect("the port kept");
        let app = axum::Router::new()
            .route(
                route,
                axum::routing::get(std::future::pending::<&'static str>),
            )
            .layer(axum::middleware::from_fn(crate::logs::note_request));
        let shutdown: Shutdown = Box::pin(async move {
            let _ = stop_asked.await;
            Ok(())
        });
        let limits = ConnectionLimits {
            drain_grace: std::time::Duration::from_millis(200),
            ..ConnectionLimits::SERVED
        };
        let served = tokio::spawn(serve_limited(listener, app, shutdown, limits));
        let held = Held {
            lock: Some(lock),
            port: Some(port),
        };
        (held, addr, served)
    });
    assert!(held.holds());
    assert!(!store_is_free(&root), "the serving process holds the store");

    // ONE REQUEST, OPEN.
    let mut client = std::net::TcpStream::connect(addr).expect("the server answers the door");
    std::io::Write::write_all(
        &mut client,
        format!("GET {route} HTTP/1.1\r\nHost: test\r\n\r\n").as_bytes(),
    )
    .expect("the request is sent");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while !crate::speed::global()
        .open_now()
        .iter()
        .any(|(name, n)| name == route && *n == 1)
    {
        assert!(
            std::time::Instant::now() < deadline,
            "the request never opened"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // THE STOP, AND THE BOUND.
    stop.send(()).expect("the server waits for its stop");
    let ended = runtime
        .block_on(async { tokio::time::timeout(std::time::Duration::from_secs(20), served).await })
        .expect("the bound ended the serve, not this test's timeout")
        .expect("the serve task did not panic");
    assert!(ended.is_ok(), "{ended:?}");
    let started = crate::emitted::landed(mark, "api.serve", "stop started");
    assert!(
        started
            .iter()
            .any(|record| field_of(record, "drain_ms") == 200
                && field_of(record, "bound_ms") == 200 + millis(SHUTDOWN_GRACE)),
        "the stop says its bounds when it starts: {started:?}"
    );
    let cut = crate::emitted::landed(
        mark,
        "api.serve",
        "requests still open at the stop were cut short",
    );
    assert!(
        cut.iter().any(|record| {
            crate::emitted::says(record, "routes", &format!("{route}=1"))
                && field_of(record, "cut") >= 1
        }),
        "the cut names how many and where: {cut:?}"
    );

    // THE SERVE HAS RETURNED AND THE RUNTIME HAS NOT ENDED: blocking work may
    // still be writing, so the store is held, and so is the port.
    assert!(
        !store_is_free(&root),
        "the store is held until the runtime ends"
    );
    assert!(
        !port_is_free(addr),
        "the port stays bound while the store is held"
    );

    // THE END: the runtime drops the cut request, which counts it; then the
    // last minute is written; then the store and the port are freed.
    let _abandoned = ended_then_released(runtime, std::time::Duration::from_secs(5), held);
    let lines: Vec<_> = crate::emitted::landed(mark, "api.speed", "minute ended")
        .into_iter()
        .filter(|record| crate::emitted::says(record, "route", route))
        .collect();
    let n: u64 = lines.iter().map(|record| field_of(record, "n")).sum();
    let cancelled: u64 = lines
        .iter()
        .map(|record| field_of(record, "cancelled"))
        .sum();
    assert_eq!(
        (n, cancelled),
        (1, 1),
        "the cut request is counted once, as cancelled: {lines:?}"
    );
    assert!(store_is_free(&root), "the store is free after the end");
    assert!(port_is_free(addr), "and so is the port");
    drop(client);
}

/// **A KEPT PORT IS THE SAME SOCKET, AND IS RELEASED WITH THE STORE** (R3-14
/// repair 1): while a [`Held`] lives, the address cannot be bound even after
/// the server's own listener is gone; releasing it frees both.
#[tokio::test]
async fn a_kept_port_outlives_the_listener_until_released() {
    let root = crate::scratch::path("stop-kept-port");
    std::fs::create_dir_all(&root).expect("mkdir");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a port");
    let addr = listener.local_addr().expect("its address");
    let lock = take_serve_lock(&root, addr).expect("a free store");
    let (listener, port) = keep_port(listener).expect("the port kept");
    drop(listener);
    let held = Held {
        lock: Some(lock),
        port: Some(port),
    };
    assert!(!port_is_free(addr), "the kept duplicate holds the port");
    assert!(!store_is_free(&root));
    held.release();
    assert!(store_is_free(&root));
    assert!(port_is_free(addr));
    assert!(
        !Held::default().holds(),
        "a command that never served holds nothing"
    );
}
