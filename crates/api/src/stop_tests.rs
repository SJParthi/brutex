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

/// Whether another process could open `root`'s log right now: a fresh open
/// file description on `<store>/telemetry/events.lock`, the lock every sink
/// takes before it writes (R3-14 repair 3).
fn log_is_free(root: &Path) -> bool {
    let dir = telemetry::dir_beneath_store(root);
    std::fs::create_dir_all(&dir).expect("the log directory");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join("events.lock"))
        .expect("the log's lock file");
    file.try_lock().is_ok()
}

/// A sink on `root`'s own log directory, as the serving process installs one,
/// for the life of the test process (`Held` keeps a `'static` sink).
fn store_log(root: &Path) -> &'static telemetry::Sink {
    Box::leak(Box::new(
        telemetry::Sink::open(&telemetry::Config::new(telemetry::dir_beneath_store(root)))
            .expect("the store's log opens"),
    ))
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
            log: Some(store_log(&root)),
            lock: Some(lock),
            port: Some(port),
        };
        (held, addr, served)
    });
    assert!(held.holds());
    assert!(!store_is_free(&root), "the serving process holds the store");
    assert!(!log_is_free(&root), "and its log");

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
    assert!(!log_is_free(&root), "and so does the log");

    // THE END: the runtime drops the cut request, which counts it; then the
    // last minute is written; then the exit line, while everything is still
    // held; then the log, the store and the port are freed.
    let mut said_last = false;
    let _abandoned = ended_then_released(runtime, std::time::Duration::from_secs(5), held, || {
        assert!(
            !log_is_free(&root) && !store_is_free(&root) && !port_is_free(addr),
            "the exit line is said while the log, the store and the port are all held"
        );
        said_last = true;
    });
    assert!(said_last, "the exit line was said");
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
    assert!(log_is_free(&root), "and so is the log");
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
        log: None,
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

/// A [`telemetry::Target`] that, when the sink closing it syncs it, records
/// whether the store and the port could be taken at that instant.
#[derive(Debug)]
struct WhenClosed {
    root: PathBuf,
    addr: std::net::SocketAddr,
    seen: std::sync::Arc<std::sync::Mutex<Vec<(bool, bool)>>>,
}

impl telemetry::Target for WhenClosed {
    fn append(&mut self, _bytes: &[u8]) -> std::io::Result<()> {
        Ok(())
    }

    fn sync(&self) -> std::io::Result<()> {
        self.seen
            .lock()
            .expect("the record")
            .push((store_is_free(&self.root), port_is_free(self.addr)));
        Ok(())
    }

    fn reopen(&mut self, _path: &Path) -> std::io::Result<()> {
        Ok(())
    }
}

/// **WHENEVER THE PORT CAN BE BOUND, THE STORE AND THE LOG CAN BE TAKEN**
/// (R3-14 repair 3).
///
/// Fails before the repair: the port and the serve lock were freed while the
/// process's log lock lived on until its last instant, so a successor started
/// the moment the port freed ran its whole life with no log ("log: NOT
/// WRITABLE", 2 of 6 measured redeploys). Now [`Held::release`] closes the
/// log first, at which instant the store and the port must both still be
/// held; then frees the store, then the port. A dropped `Held` frees the
/// store and the port and leaves the log alone: a test process shares one.
#[tokio::test]
async fn whenever_the_port_is_free_the_store_and_the_log_are_free() {
    let root = crate::scratch::path("stop-log-first");
    std::fs::create_dir_all(&root).expect("mkdir");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a port");
    let addr = listener.local_addr().expect("its address");
    let lock = take_serve_lock(&root, addr).expect("a free store");
    let (listener, port) = keep_port(listener).expect("the port kept");
    drop(listener);
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let spy: &'static telemetry::Sink = Box::leak(Box::new(
        telemetry::Sink::with_target(
            &telemetry::Config::new(root.join("spy-log")),
            Box::new(WhenClosed {
                root: root.clone(),
                addr,
                seen: std::sync::Arc::clone(&seen),
            }),
        )
        .expect("the spy sink"),
    ));
    let held = Held {
        log: Some(spy),
        lock: Some(lock),
        port: Some(port),
    };
    held.release();
    assert_eq!(
        *seen.lock().expect("the record"),
        vec![(false, false)],
        "the log closed first, while the store and the port were both still held"
    );
    assert!(
        store_is_free(&root) && port_is_free(addr),
        "then both were freed"
    );
    assert_eq!(
        spy.emit(&telemetry::Event::error("t", "after the release")),
        telemetry::Emitted::Dropped,
        "nothing is written after the log is closed"
    );

    // THE REAL LOG LOCK, AND A DROPPED `Held` LEAVES IT ALONE.
    let lock = take_serve_lock(&root, addr).expect("the store again");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("the port again");
    let (listener, port) = keep_port(listener).expect("the port kept");
    drop(listener);
    let log = store_log(&root);
    drop(Held {
        log: Some(log),
        lock: Some(lock),
        port: Some(port),
    });
    assert!(store_is_free(&root) && port_is_free(addr));
    assert!(!log_is_free(&root), "a dropped Held does not close the log");
    log.close().expect("closed by hand");
    assert!(log_is_free(&root), "a closed log frees its lock");
}

/// **BLOCKING WORK THAT OUTLIVES THE WAIT KEEPS THE STORE AND THE PORT UNTIL
/// THE PROCESS ENDS** (R3-14 repair 4).
///
/// Tokio's `shutdown_timeout` detaches a blocking thread still running at its
/// deadline, and it runs on until the process ends: measured 0.15 to 0.5 s
/// after the release on a shadow with a scrub in flight. The release freed
/// the store and the port into that gap. Fails before the repair: the store
/// and the port were both free the moment the end returned, with the work
/// still running. Now the log is closed, the store and the port are kept for
/// the process's end to free, and the keep is said at `Error`.
#[test]
fn blocking_work_past_the_wait_keeps_the_store_and_the_port_to_the_process_end() {
    let _shared = crate::emitted::sink();
    let mark = crate::emitted::mark();
    let root = crate::scratch::path("stop-outlived");
    std::fs::create_dir_all(&root).expect("mkdir");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime");
    let (held, addr) = runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a port");
        let addr = listener.local_addr().expect("its address");
        let lock = take_serve_lock(&root, addr).expect("a free store");
        let (_listener, port) = keep_port(listener).expect("the port kept");
        let held = Held {
            log: Some(store_log(&root)),
            lock: Some(lock),
            port: Some(port),
        };
        (held, addr)
    });
    let (started, has_started) = std::sync::mpsc::channel::<()>();
    let (finish, until_finished) = std::sync::mpsc::channel::<()>();
    let _work = runtime.spawn_blocking(move || {
        let _sent = started.send(());
        // Runs until this test lets it go, long past the wait.
        let _released = until_finished.recv();
    });
    has_started.recv().expect("the blocking work started");
    let grace = std::time::Duration::from_millis(100);
    let began = std::time::Instant::now();
    let mut said_last = false;
    let _abandoned = ended_then_released(runtime, grace, held, || said_last = true);
    assert!(began.elapsed() >= grace, "the end waited out its grace");
    assert!(said_last, "the exit line was said");
    assert!(
        !store_is_free(&root),
        "the store stays held while this process's blocking work still runs"
    );
    assert!(!port_is_free(addr), "and so does the port");
    assert!(
        log_is_free(&root),
        "the log is closed, as every end closes it"
    );
    let said = crate::emitted::landed(mark, "api.main", "blocking work outlived the shutdown wait");
    assert!(
        said.iter().any(|record| field_of(record, "grace_ms") == 100
            && field_of(record, "engine_abandoned") == 0),
        "the keep is said, with the wait it outlived: {said:?}"
    );
    drop(finish);
}

/// A CLEAN END IS NOT OUTLIVED, AND ONE THAT TOOK ALL ITS TIME IS: the
/// measure [`end_served`] decides the release by (R3-14 repair 4). Zero time
/// left counts as outlived, because an end with no time to wait cannot know.
#[test]
fn an_end_that_takes_all_its_time_is_outlived_and_a_quick_one_is_not() {
    let ms = std::time::Duration::from_millis;
    assert!(!took_all_of(ms(5_000), || {}), "a quick end");
    assert!(
        took_all_of(ms(30), || std::thread::sleep(ms(40))),
        "an end that ran past its time"
    );
    assert!(took_all_of(ms(0), || {}), "no time left");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime");
    let ended = wait_then_end_counting(runtime, ms(2_000), || 0);
    assert_eq!(
        ended,
        Ended {
            engine: 0,
            outlived: false
        },
        "a runtime with nothing running ends inside its wait and is released"
    );
}
