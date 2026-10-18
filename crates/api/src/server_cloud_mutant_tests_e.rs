//! Cloud mutation pass of 18 Oct (`cloud/mut-api-b-20261018`) over
//! `server.rs`: the arms of AH-02's [`HeadDeadline`] and [`exchange_watch`]
//! (D-4975) that a coverage run of the whole `api` lib suite on the merged
//! tree (`feat/db-fix` `67c4e58d`, merged at `19e2c2f9`) left unexecuted,
//! each driven where a test can reach it.
//!
//! The `HeadDeadline` here is built by hand around one end of a loopback
//! pair, so a test can put it in the state it needs (a request in flight, or
//! the head clock running) and read the one `api.serve` line it writes
//! through [`crate::emitted`], by a tag in the request it is said to be
//! answering.
//!
//! No test here opens a socket to anything but `127.0.0.1`, reads a
//! credential, or contacts a vendor.
#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a failed fixture must fail the test by name"
)]

use super::*;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

/// How long any one read or write below may take before the test fails.
const PATIENCE: std::time::Duration = std::time::Duration::from_secs(5);

/// A connected pair: the accepted server side, and the client.
async fn socket_pair() -> (tokio::net::TcpStream, tokio::net::TcpStream) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let client = tokio::net::TcpStream::connect(addr).await.expect("connect");
    let (server, _) = listener.accept().await.expect("accept");
    (server, client)
}

/// A `HeadDeadline` over `server` whose connection is answering
/// `GET /cloude/<tag>` with a `200` of 1,000 declared bytes, in the state
/// `state`. Its one slot is the pool's only one, handed back with it.
fn answering(
    server: tokio::net::TcpStream,
    tag: &str,
    state: HeadState,
) -> (std::sync::Arc<Slots>, HeadDeadline) {
    let slots = std::sync::Arc::new(Slots {
        live: std::sync::atomic::AtomicUsize::new(1),
        cap: 1,
        freed: tokio::sync::Notify::new(),
    });
    let slot = Slot(std::sync::Arc::clone(&slots));
    let mut wrapped = HeadDeadline::new(server, slot, std::time::Duration::from_secs(30));
    *wrapped
        .exchange
        .newest
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(ExchangeNote {
        method: axum::http::Method::GET,
        uri: format!("/cloude/{tag}").parse().expect("a path"),
        status: Some(200),
        length: Some(1_000),
        counted: None,
    });
    wrapped.state = state;
    (slots, wrapped)
}

/// The `api.serve` lines carrying `message` since `from` whose path names `tag`.
fn lines(from: u64, message: &str, tag: &str) -> Vec<telemetry::Record> {
    crate::emitted::landed(from, "api.serve", message)
        .into_iter()
        .filter(|record| crate::emitted::says(record, "path", tag))
        .collect()
}

/// **A client that ends its stream while a response is in flight is reported
/// once, as its close.** The end of the stream with a request in flight is
/// the client leaving before its answer was whole, and the line says which
/// request, its status, its declared length and that nothing of it went. A
/// read into a buffer with no room is not an end of stream and says nothing,
/// and a second end on the same connection writes no second line.
///
/// Covers `poll_read`'s end-of-stream arm (server.rs:17746-17753), which no
/// served test reaches (a client that leaves mid-body is met by a failed
/// write first), and the `reported` guard of `HeadDeadline::report`
/// (17682-17683).
#[tokio::test]
async fn a_client_that_ends_its_stream_mid_response_is_reported_once_as_its_close() {
    let _sink = crate::emitted::sink();
    let from = crate::emitted::mark();
    let (server, mut client) = socket_pair().await;
    let (_slots, mut wrapped) = answering(server, "cloudeends", HeadState::Delivered);
    client.shutdown().await.expect("the client ends its side");

    // NO ROOM, NO END: a zero-length read reads nothing and reports nothing.
    let mut no_room: [u8; 0] = [];
    let none = tokio::time::timeout(PATIENCE, wrapped.read(&mut no_room))
        .await
        .expect("a zero-length read answers at once")
        .expect("a zero-length read");
    assert_eq!(none, 0);
    assert!(!wrapped.reported);
    assert!(lines(from, "response closed by the client", "cloudeends").is_empty());

    // THE END OF THE STREAM, with room: reported, once.
    let mut buf = [0_u8; 16];
    let ended = tokio::time::timeout(PATIENCE, wrapped.read(&mut buf))
        .await
        .expect("the end answers at once")
        .expect("an end of stream is no error");
    assert_eq!(ended, 0);
    assert!(wrapped.reported);
    let said = lines(from, "response closed by the client", "cloudeends");
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(crate::emitted::says(
        &said[0],
        "why",
        "the client closed its side before the response was whole"
    ));
    assert!(crate::emitted::says(&said[0], "method", "GET"));
    assert!(crate::emitted::counts(&said[0], "status", 200));
    assert!(crate::emitted::counts(&said[0], "declared_length", 1_000));
    assert!(crate::emitted::counts(&said[0], "bytes_written", 0));
    assert_eq!(
        said[0]
            .field("delivered")
            .and_then(telemetry::OwnedValue::as_bool),
        Some(false)
    );
    assert_eq!(
        said[0]
            .field("access_counted")
            .and_then(telemetry::OwnedValue::as_bool),
        Some(false)
    );

    // A SECOND END writes no second line: one per connection.
    let again = tokio::time::timeout(PATIENCE, wrapped.read(&mut buf))
        .await
        .expect("the end answers again")
        .expect("an end of stream is no error");
    assert_eq!(again, 0);
    assert_eq!(
        lines(from, "response closed by the client", "cloudeends").len(),
        1
    );
    assert!(
        wrapped
            .report(telemetry::Level::Warn, "response cut", "a second word")
            .is_none(),
        "a connection already reported says nothing more"
    );
    assert!(lines(from, "response cut", "cloudeends").is_empty());
}

/// **A read that fails while a response is in flight is reported as the
/// client's close, naming the failure.** A reset peer is a client that left;
/// the line carries the read's own error kind. Covers `poll_read`'s
/// `Poll::Ready(Err(why))` report under `HeadState::Delivered`
/// (server.rs:17758-17763).
#[tokio::test]
async fn a_read_that_fails_mid_response_is_reported_as_the_clients_close() {
    let _sink = crate::emitted::sink();
    let from = crate::emitted::mark();
    let (server, client) = socket_pair().await;
    let (_slots, mut wrapped) = answering(server, "cloudereset", HeadState::Delivered);
    // SO_LINGER of zero turns the close into a reset.
    #[allow(deprecated, reason = "the one portable way tokio offers to send RST")]
    client
        .set_linger(Some(std::time::Duration::ZERO))
        .expect("linger");
    drop(client);
    let mut buf = [0_u8; 16];
    let why = tokio::time::timeout(PATIENCE, wrapped.read(&mut buf))
        .await
        .expect("the reset arrives at once")
        .expect_err("a reset is an error");
    assert_eq!(why.kind(), std::io::ErrorKind::ConnectionReset, "{why}");
    assert!(wrapped.reported);
    let said = lines(from, "response closed by the client", "cloudereset");
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(crate::emitted::says(
        &said[0],
        "why",
        "a read failed: connection reset"
    ));
    assert!(crate::emitted::counts(&said[0], "status", 200));
}

/// **A read that fails with no request in flight reports nothing.** The
/// connection was waiting for a head; nobody was being answered, so there is
/// no response to call closed. The error still reaches the reader as itself.
/// The `HeadState::Delivered` test of the same arm is what this one stands
/// beside.
#[tokio::test]
async fn a_read_that_fails_with_nothing_in_flight_reports_nothing() {
    let _sink = crate::emitted::sink();
    let from = crate::emitted::mark();
    let (server, client) = socket_pair().await;
    let waiting = HeadState::Awaiting {
        deadline: tokio::time::Instant::now() + std::time::Duration::from_secs(30),
        progress: 0,
        partial: false,
    };
    let (_slots, mut wrapped) = answering(server, "cloudeidle", waiting);
    #[allow(deprecated, reason = "the one portable way tokio offers to send RST")]
    client
        .set_linger(Some(std::time::Duration::ZERO))
        .expect("linger");
    drop(client);
    let mut buf = [0_u8; 16];
    let why = tokio::time::timeout(PATIENCE, wrapped.read(&mut buf))
        .await
        .expect("the reset arrives at once")
        .expect_err("a reset is an error");
    assert_eq!(why.kind(), std::io::ErrorKind::ConnectionReset, "{why}");
    assert!(!wrapped.reported);
    assert!(lines(from, "response closed by the client", "cloudeidle").is_empty());
}

/// **A direct write while the head clock runs starts a response.** hyper
/// writes a response for a head it parsed from its own buffer, which this
/// wrapper never saw; the write stops the head clock, counts its bytes from
/// zero, and the flush after it, with every request finished, starts a fresh
/// head's clock with nothing of the old partial head kept. Covers
/// `poll_write`'s `HeadState::Awaiting` arm (server.rs:17790-17791); hyper's
/// vectored writes reach only its twin.
#[tokio::test]
async fn a_direct_write_while_the_head_clock_runs_starts_a_response() {
    let (server, mut client) = socket_pair().await;
    let partway = HeadState::Awaiting {
        deadline: tokio::time::Instant::now() + std::time::Duration::from_secs(30),
        progress: 1,
        partial: true,
    };
    let (slots, mut wrapped) = answering(server, "cloudewrite", partway);
    wrapped.written = 99;
    let wrote = tokio::time::timeout(PATIENCE, wrapped.write(b"HTTP/1.1 200 OK\r\n"))
        .await
        .expect("a write answers at once")
        .expect("a write");
    assert_eq!(wrote, 17);
    assert!(matches!(wrapped.state, HeadState::Delivered));
    assert_eq!(
        wrapped.written, 17,
        "counted from zero at the response's start"
    );

    tokio::time::timeout(PATIENCE, wrapped.flush())
        .await
        .expect("a flush answers at once")
        .expect("a flush");
    let HeadState::Awaiting {
        deadline,
        progress,
        partial,
    } = wrapped.state
    else {
        panic!("the flush re-arms the head clock");
    };
    assert_eq!((progress, partial), (0, false));
    assert!(deadline > tokio::time::Instant::now() + std::time::Duration::from_secs(25));

    let mut got = [0_u8; 17];
    tokio::time::timeout(PATIENCE, client.read_exact(&mut got))
        .await
        .expect("the peer reads it at once")
        .expect("the peer reads it");
    assert_eq!(&got, b"HTTP/1.1 200 OK\r\n");
    drop(wrapped);
    assert_eq!(slots.live.load(std::sync::atomic::Ordering::Acquire), 0);
}

/// **A router served without the limited listener passes through the watch
/// untouched.** `exchange_watch` reads its connection's `Exchange` from the
/// connect-info extension, which only `serve_limited` provides; anywhere else
/// a request is answered as the router answers it. Covers the `else` arm of
/// `exchange_watch`'s extension read (server.rs:17387-17392).
#[tokio::test]
async fn a_router_served_without_the_limited_listener_passes_through_the_watch() {
    let app = axum::Router::new()
        .route("/", axum::routing::get(|| async { "plain answer" }))
        .layer(axum::middleware::from_fn(exchange_watch));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let served = tokio::spawn(async move { axum::serve(listener, app).await });
    let mut client = tokio::net::TcpStream::connect(addr).await.expect("connect");
    client
        .write_all(b"GET / HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n")
        .await
        .expect("a request");
    let mut said = String::new();
    tokio::time::timeout(PATIENCE, client.read_to_string(&mut said))
        .await
        .expect("an answer")
        .expect("a readable answer");
    served.abort();
    assert!(said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
    assert!(said.ends_with("plain answer"), "{said:?}");
}

/// What `finished_day_only` must answer, at this instant, for a window that
/// ends on `to`: refused while `to` is today and today's session has not
/// settled, with the reason today is held back, and accepted otherwise.
/// Built from the same parts the guard reads (the calendar of record, the
/// venue's dated close, the minute of the day), but asked of them here.
fn ending_on(to: Day) -> Result<(), String> {
    let now = ingest::ist_moment(std::time::SystemTime::now()).expect("a usable clock");
    let today = now.day();
    if to != today {
        // The day turned between building the window and asking: it ended
        // yesterday, which is askable.
        return Ok(());
    }
    let close = pull::vendor::Venue::NseCash
        .hours_on(today)
        .ok()
        .map(pull::vendor::Session::close_minute);
    match settled_today(
        pull::calendar::exchange_kind_of(i64::from(today.days_from_epoch())),
        close,
        now.minute_of_day(),
    ) {
        Ok(()) => Ok(()),
        Err(why) => Err(format!(
            "{} Today: {why}.",
            ingest::Refusal::WindowReachesToday { to, today }
        )),
    }
}

/// **A window that ends today is refused, with the reason today is held
/// back, exactly while today's session has not settled.** The guard reads
/// the wall clock, and this build has no clock seam, so the test asks the
/// same question of the same parts at the moment of the call. Before and
/// after it, so a minute (or a day) that turns in between is tolerated, and
/// holds the answer to one of the two. On a day already settled (any closed
/// day, and every open day from five minutes past its close) the answer is
/// `Ok`. In the hours before that, it is the refusal ending in " Today: …",
/// which kills the match guard at `finished_day_only`'s refusal
/// (server.rs:6723) turned to `false`. Those hours are the only ones in
/// which any test can tell that mutant apart.
#[test]
fn a_window_ending_today_is_held_back_exactly_while_today_is_unsettled() {
    let today = ingest::ist_day(std::time::SystemTime::now()).expect("a usable clock");
    let tomorrow = today.succ().expect("a day after today");
    let asked = ingest::parse_spot(
        &format!("target=swept&vendor=dhan&granularity=1day&from={today}&to={today}"),
        tomorrow,
    )
    .expect("a one-day window ending today, parsed as of tomorrow");
    assert_eq!(asked.window.to(), today);
    let before = ending_on(today);
    let got = finished_day_only(&asked);
    let after = ending_on(today);
    assert!(
        got == before || got == after,
        "got {got:?}; the guard's parts said {before:?} then {after:?}"
    );
}
