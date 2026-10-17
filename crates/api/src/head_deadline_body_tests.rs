//! AH-02, g-ui-dom-vs-json-1, D-4975: A RESPONSE IS NEVER CUT MID-BODY WHILE
//! ITS READER KEEPS TAKING BYTES.
//!
//! Until D-4975 the first written byte of a response re-armed the head
//! deadline, so every body still streaming one deadline after it began was
//! cut — a `200` with a `Content-Length`, short. These tests drive
//! [`serve_limited`] over real loopback sockets with a deadline of
//! milliseconds. Each client shrinks its receive buffer to 64 KiB, so the
//! server must wait on the client's reads instead of parking a whole body in
//! the kernel, and every body carries a position pattern, so a skipped or a
//! repeated chunk is a mismatch and not only a short count.
//!
//! The stalled-reader tests read the `api.serve` lines through
//! [`crate::emitted`], the test binary's one shared sink, and find their own
//! by a `test=` tag in the request's query.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]

use super::{ConnectionLimits, HeadDeadline, HeadState, Slot, Slots, serve_limited};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;
use tokio::time::Instant;

/// The deadline most tests run with.
const T: Duration = Duration::from_millis(300);
/// The scheduling allowance on the far side of a deadline, stated once: a
/// cut or a close must come within the deadline plus this.
const SLACK: Duration = Duration::from_millis(700);
/// How early a measured close may be against the deadline it should wait
/// for: the server's clock starts a little before the client's.
const EARLY: Duration = Duration::from_millis(100);
/// The body every slow read below asks for: 16 MiB.
const BIG: usize = 16 << 20;
/// The slow reader: up to this many bytes a read, then [`PACE`] asleep —
/// at most 6.4 MB/s, so [`BIG`] takes at least 2.6 s, over eight of [`T`].
const CHUNK: usize = 64 << 10;
/// See [`CHUNK`].
const PACE: Duration = Duration::from_millis(10);

/// The byte at position `i` of every body this server sends.
fn pattern(i: usize) -> u8 {
    u8::try_from(i % 251).unwrap()
}

/// A finite body of `total` pattern bytes, sent in 64 KiB frames with no
/// declared length, so hyper frames it as chunked.
struct Chunks {
    sent: usize,
    total: usize,
}

impl http_body::Body for Chunks {
    type Data = axum::body::Bytes;
    type Error = std::convert::Infallible;

    fn poll_frame(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        if self.sent >= self.total {
            return std::task::Poll::Ready(None);
        }
        let n = (self.total - self.sent).min(CHUNK);
        let chunk: Vec<u8> = (self.sent..self.sent + n).map(pattern).collect();
        self.sent += n;
        std::task::Poll::Ready(Some(Ok(http_body::Frame::data(axum::body::Bytes::from(
            chunk,
        )))))
    }
}

fn app() -> axum::Router {
    use axum::extract::Path;
    use axum::routing::get;
    axum::Router::new()
        .route("/ok", get(|| async { "ok" }))
        .route(
            "/big/{n}",
            get(|Path(n): Path<usize>| async move { (0..n).map(pattern).collect::<Vec<u8>>() }),
        )
        .route(
            "/chunked/{n}",
            get(|Path(n): Path<usize>| async move {
                axum::body::Body::new(Chunks { sent: 0, total: n })
            }),
        )
        .route(
            "/slow",
            get(|| async {
                tokio::time::sleep(T * 3).await;
                "slow"
            }),
        )
        .route(
            "/empty",
            get(|| async { axum::http::StatusCode::NO_CONTENT }),
        )
        .route(
            "/same",
            get(|| async { axum::http::StatusCode::NOT_MODIFIED }),
        )
}

fn limits(timeout: Duration, cap: usize) -> ConnectionLimits {
    ConnectionLimits {
        head_read_timeout: timeout,
        body_read_timeout: timeout,
        max_connections: cap,
        drain_grace: timeout,
    }
}

/// A server of [`app`] on an ephemeral port, and the sender that stops it.
async fn start(limits: ConnectionLimits) -> (SocketAddr, tokio::sync::oneshot::Sender<()>) {
    start_app(limits, app()).await
}

/// A server of `router` on an ephemeral port, and the sender that stops it.
async fn start_app(
    limits: ConnectionLimits,
    router: axum::Router,
) -> (SocketAddr, tokio::sync::oneshot::Sender<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let _served = tokio::spawn(serve_limited(
        listener,
        router,
        Box::pin(async move {
            let _ = stopped.await;
            Ok(())
        }),
        limits,
    ));
    (addr, stop)
}

/// A client with a 64 KiB receive buffer.
async fn connect(addr: SocketAddr) -> TcpStream {
    let socket = tokio::net::TcpSocket::new_v4().unwrap();
    socket.set_recv_buffer_size(64 << 10).unwrap();
    socket.connect(addr).await.unwrap()
}

/// One response head, and whatever body bytes arrived with it.
async fn head(stream: &mut TcpStream) -> (String, Vec<u8>) {
    let mut got = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        if let Some(end) = got.windows(4).position(|w| w == b"\r\n\r\n") {
            let rest = got.split_off(end + 4);
            return (String::from_utf8_lossy(&got).into_owned(), rest);
        }
        let n = tokio::time::timeout(Duration::from_secs(10), stream.read(&mut chunk))
            .await
            .expect("a head within 10 s")
            .expect("read");
        assert!(
            n > 0,
            "closed before a head: {:?}",
            String::from_utf8_lossy(&got)
        );
        got.extend_from_slice(&chunk[..n]);
    }
}

fn content_length(head: &str) -> usize {
    head.lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(|value| value.trim().parse().unwrap())
        })
        .unwrap_or_else(|| panic!("no Content-Length in {head:?}"))
}

/// Reads until `want` bytes are held or the server closes: up to `chunk`
/// bytes a read, then `pace` asleep. Returns the bytes and whether it closed.
async fn paced(
    stream: &mut TcpStream,
    mut have: Vec<u8>,
    want: usize,
    chunk: usize,
    pace: Duration,
) -> (Vec<u8>, bool) {
    let mut buf = vec![0_u8; chunk];
    while have.len() < want {
        let room = (want - have.len()).min(chunk);
        match tokio::time::timeout(Duration::from_secs(10), stream.read(&mut buf[..room])).await {
            Ok(Ok(0) | Err(_)) => return (have, true),
            Ok(Ok(n)) => have.extend_from_slice(&buf[..n]),
            Err(elapsed) => panic!("{elapsed}: no byte for 10 s after {} of {want}", have.len()),
        }
        if !pace.is_zero() {
            tokio::time::sleep(pace).await;
        }
    }
    (have, false)
}

/// Everything the server sends until it closes, or until `limit` passes, and
/// whether it closed.
async fn drain(stream: &mut TcpStream, limit: Duration) -> (String, bool) {
    let mut got = Vec::new();
    let mut chunk = [0_u8; 4096];
    let until = Instant::now() + limit;
    loop {
        match tokio::time::timeout_at(until, stream.read(&mut chunk)).await {
            Ok(Ok(0) | Err(_)) => return (String::from_utf8_lossy(&got).into_owned(), true),
            Ok(Ok(n)) => got.extend_from_slice(&chunk[..n]),
            Err(_elapsed) => return (String::from_utf8_lossy(&got).into_owned(), false),
        }
    }
}

/// Reads one response whose body ends with `tail`, leaving the socket open.
async fn one_response(stream: &mut TcpStream, tail: &str) -> String {
    let mut got = Vec::new();
    let mut chunk = [0_u8; 4096];
    while !String::from_utf8_lossy(&got).ends_with(tail) {
        let n = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut chunk))
            .await
            .expect("a response within 5 s")
            .expect("read");
        assert!(
            n > 0,
            "closed mid-response: {}",
            String::from_utf8_lossy(&got)
        );
        got.extend_from_slice(&chunk[..n]);
    }
    String::from_utf8_lossy(&got).into_owned()
}

fn assert_pattern(body: &[u8]) {
    if let Some(at) = body.iter().enumerate().position(|(i, b)| *b != pattern(i)) {
        panic!("byte {at} of {} is not the byte sent there", body.len());
    }
}

/// The `api.serve` lines carrying `message` since `from` whose path names `tag`.
fn lines(from: u64, message: &str, tag: &str) -> Vec<telemetry::Record> {
    crate::emitted::landed(from, "api.serve", message)
        .into_iter()
        .filter(|record| crate::emitted::says(record, "path", tag))
        .collect()
}

/// THE FINDING'S SHAPE. A 16 MiB body to a reader taking at most 6.4 MB/s
/// takes over eight deadlines to arrive. Every byte arrives, in order, the
/// count equals `Content-Length`, and the kept-alive socket then serves a
/// second request. Before D-4975 the connection closed one deadline after the
/// first byte with most of the body unsent.
#[tokio::test]
async fn a_body_longer_than_the_head_deadline_reaches_a_slow_reader_whole() {
    let (addr, stop) = start(limits(T, 8)).await;
    let mut client = connect(addr).await;
    let began = Instant::now();
    client
        .write_all(format!("GET /big/{BIG} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes())
        .await
        .unwrap();
    let (head, rest) = head(&mut client).await;
    assert!(head.starts_with("HTTP/1.1 200 OK"), "{head:?}");
    let length = content_length(&head);
    assert_eq!(length, BIG);
    let (body, closed) = paced(&mut client, rest, length, CHUNK, PACE).await;
    let took = began.elapsed();
    assert!(
        !closed,
        "cut after {} of {length} bytes at {took:?}",
        body.len()
    );
    assert_eq!(body.len(), length);
    assert_pattern(&body);
    assert!(
        took >= T * 4,
        "the body drained in {took:?}: too fast to prove a slow reader is safe"
    );
    client
        .write_all(b"GET /ok HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let (said, closed) = drain(&mut client, T * 10).await;
    assert!(
        closed && said.starts_with("HTTP/1.1 200 OK") && said.ends_with("ok"),
        "{said:?}"
    );
    let _ = stop.send(());
}

/// A READER THAT STOPS IS CUT, ONCE, AND SAID TO BE CUT. With one slot, the
/// next client is answered only when the stopped reader's connection closes,
/// so its answer time is the cut's time: no earlier than one stall deadline
/// after the reader stopped and within the deadline plus [`SLACK`]. The
/// reader then finds its body short. One `api.serve` "response cut" line names
/// the request, its `200`, its declared length and the bytes that went, which
/// are fewer; no "closed by the client" line is written for it. Before D-4975
/// the close came at the same time but nothing was logged.
#[tokio::test]
async fn a_reader_that_stops_reading_is_closed_and_logged_as_cut() {
    let _sink = crate::emitted::sink();
    let from = crate::emitted::mark();
    let (addr, stop) = start(limits(T, 1)).await;
    let mut reader = connect(addr).await;
    reader
        .write_all(format!("GET /big/{BIG}?test=stops HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes())
        .await
        .unwrap();
    let (head, rest) = head(&mut reader).await;
    let length = content_length(&head);
    // Nothing more is read from here.
    let stopped = Instant::now();
    let mut next = TcpStream::connect(addr).await.unwrap();
    next.write_all(b"GET /ok HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let (said, closed) = drain(&mut next, T * 20).await;
    let freed = stopped.elapsed();
    assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
    assert!(
        freed + EARLY >= T,
        "the slot came back at {freed:?}, before a stall deadline"
    );
    assert!(
        freed < T + SLACK,
        "a stopped reader held its slot for {freed:?}"
    );
    let (body, closed) = paced(&mut reader, rest, length, 1 << 20, Duration::ZERO).await;
    assert!(
        closed && body.len() < length,
        "{} of {length} bytes",
        body.len()
    );
    let cut = lines(from, "response cut", "test=stops");
    assert_eq!(cut.len(), 1, "{cut:?}");
    let line = &cut[0];
    assert!(crate::emitted::counts(line, "status", 200), "{line:?}");
    assert!(
        crate::emitted::counts(line, "declared_length", u64::try_from(BIG).unwrap()),
        "{line:?}"
    );
    assert!(crate::emitted::says(line, "method", "GET"), "{line:?}");
    assert!(
        crate::emitted::says(line, "why", "no write progress for 300 ms"),
        "{line:?}"
    );
    let went = line
        .field("bytes_written")
        .and_then(telemetry::OwnedValue::as_u64)
        .expect("the bytes that went");
    assert!(
        went > 0 && went < u64::try_from(BIG).unwrap(),
        "{went} bytes written"
    );
    assert!(lines(from, "response closed by the client", "test=stops").is_empty());
    let _ = stop.send(());
}

/// A CUT `/db` READ IS NOT COUNTED SERVED. Behind the console's own
/// `logs::note_request`, a `/db` read route counts a read at handler return;
/// the response carries that count's mark, and the cut that follows hands it
/// back to `logs::note_cut`, so the window's `served` loses it and its `cut`
/// gains it (the arithmetic is `logs`'s
/// `a_cut_read_leaves_served_for_cut_in_the_window_that_counted_it`). The cut
/// line says `access_counted`; a route that is not a `/db` read says it was
/// not, and so does a read that arrived whole, which writes no line at all.
#[tokio::test]
async fn a_cut_db_read_is_taken_back_out_of_its_served_count() {
    let _sink = crate::emitted::sink();
    let from = crate::emitted::mark();
    let router = axum::Router::new()
        .route(
            "/store.json",
            axum::routing::get(|| async { (0..BIG).map(pattern).collect::<Vec<u8>>() }),
        )
        .route(
            "/other",
            axum::routing::get(|| async { (0..BIG).map(pattern).collect::<Vec<u8>>() }),
        )
        .layer(axum::middleware::from_fn(crate::logs::note_request));
    let (addr, stop) = start_app(limits(T, 8), router).await;
    for (target, counted) in [
        ("/store.json?feed=gdfl&test=db-cut", true),
        ("/other?test=not-db-cut", false),
    ] {
        let mut reader = connect(addr).await;
        reader
            .write_all(format!("GET {target} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes())
            .await
            .unwrap();
        let (head, rest) = head(&mut reader).await;
        assert!(head.starts_with("HTTP/1.1 200 OK"), "{head:?}");
        // Nothing is read until the cut is logged (the stopped-reader test
        // above bounds when that is).
        let tag = format!("test={}", target.split("test=").nth(1).unwrap());
        let waited = Instant::now();
        while lines(from, "response cut", &tag).is_empty() {
            assert!(
                waited.elapsed() < Duration::from_secs(10),
                "{target}: no cut"
            );
            tokio::time::sleep(T / 3).await;
        }
        let (body, closed) = paced(&mut reader, rest, BIG, 1 << 20, Duration::ZERO).await;
        assert!(closed && body.len() < BIG, "{target}: {} bytes", body.len());
        let cut = lines(from, "response cut", &tag);
        assert_eq!(cut.len(), 1, "{target}: {cut:?}");
        assert_eq!(
            cut[0]
                .field("access_counted")
                .and_then(telemetry::OwnedValue::as_bool),
            Some(counted),
            "{target}: {:?}",
            cut[0]
        );
    }
    let mut whole = connect(addr).await;
    whole
        .write_all(b"GET /store.json?test=db-whole HTTP/1.1\r\nHost: x\r\n\r\n")
        .await
        .unwrap();
    let (head, rest) = head(&mut whole).await;
    let (body, closed) = paced(
        &mut whole,
        rest,
        content_length(&head),
        1 << 20,
        Duration::ZERO,
    )
    .await;
    assert!(!closed && body.len() == BIG);
    assert!(lines(from, "response cut", "test=db-whole").is_empty());
    let _ = stop.send(());
}

/// THE KEEP-ALIVE BOUND STAYS. A response read whole at full speed is
/// complete, so the head clock runs again from its end: the idle socket is
/// closed, silently, one deadline later — not before it and not long after.
#[tokio::test]
async fn an_idle_kept_alive_connection_still_closes_after_the_deadline() {
    let (addr, stop) = start(limits(T, 8)).await;
    let mut client = connect(addr).await;
    client
        .write_all(b"GET /big/1048576 HTTP/1.1\r\nHost: x\r\n\r\n")
        .await
        .unwrap();
    let (head, rest) = head(&mut client).await;
    let length = content_length(&head);
    let (body, closed) = paced(&mut client, rest, length, 1 << 20, Duration::ZERO).await;
    assert!(!closed && body.len() == length);
    let idle = Instant::now();
    let (said, closed) = drain(&mut client, T * 10).await;
    let took = idle.elapsed();
    assert!(closed, "an idle kept-alive socket was held");
    assert_eq!(said, "", "no request, no reply");
    assert!(took + EARLY >= T, "closed early at {took:?}");
    assert!(took < T + SLACK, "closed late at {took:?}");
    let _ = stop.send(());
}

/// PAUSES. With a two-second deadline, a reader that stops for 0.9 of it four
/// times over is never cut and gets every byte; a reader that stops once for
/// 1.1 of it is cut and logged as cut. The stall clock starts when the
/// server's write first waits, which with a full socket is when the reader
/// stops, so the margin each way is a tenth of the deadline.
#[tokio::test]
async fn pauses_under_the_stall_deadline_are_never_cut_and_one_over_it_is() {
    let long = Duration::from_secs(2);
    let _sink = crate::emitted::sink();
    let from = crate::emitted::mark();
    let (addr, stop) = start(limits(long, 8)).await;
    let under = tokio::spawn(async move {
        let mut client = connect(addr).await;
        client
            .write_all(
                format!("GET /big/{BIG}?test=pauses-under HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes(),
            )
            .await
            .unwrap();
        let (head, mut body) = head(&mut client).await;
        let length = content_length(&head);
        for _ in 0..4 {
            let want = (body.len() + (512 << 10)).min(length);
            let (more, closed) = paced(&mut client, body, want, CHUNK, Duration::ZERO).await;
            assert!(!closed, "cut after {} bytes", more.len());
            body = more;
            tokio::time::sleep(long * 9 / 10).await;
        }
        let (body, closed) = paced(&mut client, body, length, 1 << 20, Duration::ZERO).await;
        assert!(!closed, "cut after {} of {length}", body.len());
        assert_eq!(body.len(), length);
        assert_pattern(&body);
    });
    let over = tokio::spawn(async move {
        let mut client = connect(addr).await;
        client
            .write_all(
                format!("GET /big/{BIG}?test=pause-over HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes(),
            )
            .await
            .unwrap();
        let (head, body) = head(&mut client).await;
        let length = content_length(&head);
        let (body, closed) = paced(&mut client, body, 512 << 10, CHUNK, Duration::ZERO).await;
        assert!(!closed);
        tokio::time::sleep(long * 11 / 10).await;
        let (body, closed) = paced(&mut client, body, length, 1 << 20, Duration::ZERO).await;
        assert!(
            closed && body.len() < length,
            "a 1.1x pause was not cut: {} of {length}",
            body.len()
        );
    });
    under.await.unwrap();
    over.await.unwrap();
    assert!(lines(from, "response cut", "test=pauses-under").is_empty());
    assert_eq!(lines(from, "response cut", "test=pause-over").len(), 1);
    let _ = stop.send(());
}

/// PIPELINING MID-BODY. A second request sent while the first body is a
/// quarter read waits its turn, and its handler takes three deadlines: the
/// first body still arrives whole at a slow reader's pace, and the second
/// response after it. The service counts the pipelined request in, so no
/// head clock runs while its handler works.
#[tokio::test]
async fn a_request_pipelined_mid_body_waits_its_turn_and_both_arrive_whole() {
    let (addr, stop) = start(limits(T, 8)).await;
    let mut client = connect(addr).await;
    let length = 8 << 20;
    client
        .write_all(format!("GET /big/{length} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes())
        .await
        .unwrap();
    let (head, rest) = head(&mut client).await;
    assert_eq!(content_length(&head), length);
    let (part, closed) = paced(&mut client, rest, length / 4, CHUNK, PACE).await;
    assert!(!closed);
    client
        .write_all(b"GET /slow HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let (body, closed) = paced(&mut client, part, length, CHUNK, PACE).await;
    assert!(!closed, "cut after {} of {length}", body.len());
    assert_pattern(&body);
    let (said, closed) = drain(&mut client, T * 20).await;
    assert!(
        closed && said.starts_with("HTTP/1.1 200 OK") && said.ends_with("slow"),
        "{said:?}"
    );
    let _ = stop.send(());
}

/// THE EMPTY-BODIED ANSWERS END THEIR RESPONSE. `HEAD`, `204`, `304` and a
/// zero-length `200` carry no body bytes; each completes, so a second request
/// sent half a deadline later is served on the same socket, and the socket
/// then closes, silently, one deadline after that.
#[tokio::test]
async fn head_204_304_and_empty_bodies_complete_and_restart_the_head_clock() {
    let (addr, stop) = start(limits(T, 8)).await;
    for (request, status) in [
        ("HEAD /big/1048576", "200"),
        ("GET /empty", "204"),
        ("GET /same", "304"),
        ("GET /big/0", "200"),
    ] {
        let mut client = connect(addr).await;
        client
            .write_all(format!("{request} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes())
            .await
            .unwrap();
        let (head, rest) = head(&mut client).await;
        assert!(
            head.starts_with(&format!("HTTP/1.1 {status}")),
            "{request}: {head:?}"
        );
        assert!(rest.is_empty(), "{request}: body bytes {rest:?}");
        tokio::time::sleep(T / 2).await;
        client
            .write_all(b"GET /ok HTTP/1.1\r\nHost: x\r\n\r\n")
            .await
            .unwrap();
        let second = one_response(&mut client, "ok").await;
        assert!(
            second.starts_with("HTTP/1.1 200 OK"),
            "{request}: {second:?}"
        );
        let idle = Instant::now();
        let (said, closed) = drain(&mut client, T * 10).await;
        let took = idle.elapsed();
        assert!(closed && said.is_empty(), "{request}: {said:?}");
        assert!(took + EARLY >= T, "{request}: closed early at {took:?}");
        assert!(took < T + SLACK, "{request}: closed late at {took:?}");
    }
    let _ = stop.send(());
}

/// Decodes a chunked body that ended with its last chunk.
fn unchunk(mut framed: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    loop {
        let line = framed
            .windows(2)
            .position(|w| w == b"\r\n")
            .expect("a chunk size line");
        let size = usize::from_str_radix(std::str::from_utf8(&framed[..line]).unwrap(), 16)
            .expect("a hex chunk size");
        framed = &framed[line + 2..];
        if size == 0 {
            assert_eq!(framed, b"\r\n", "the last chunk ends the body");
            return body;
        }
        body.extend_from_slice(&framed[..size]);
        assert_eq!(&framed[size..size + 2], b"\r\n");
        framed = &framed[size + 2..];
    }
}

/// A CHUNKED BODY WITH NO `Content-Length` reaches a slow reader whole: 8 MiB
/// at the slow reader's pace, over four deadlines, ending with its last chunk.
#[tokio::test]
async fn a_chunked_body_reaches_a_slow_reader_whole() {
    let (addr, stop) = start(limits(T, 8)).await;
    let mut client = connect(addr).await;
    let length = 8 << 20;
    let began = Instant::now();
    client
        .write_all(
            format!("GET /chunked/{length} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .await
        .unwrap();
    let (head, mut framed) = head(&mut client).await;
    assert!(
        head.to_ascii_lowercase()
            .contains("transfer-encoding: chunked"),
        "{head:?}"
    );
    let mut buf = vec![0_u8; CHUNK];
    loop {
        let n = tokio::time::timeout(Duration::from_secs(10), client.read(&mut buf))
            .await
            .expect("a byte within 10 s")
            .unwrap();
        if n == 0 {
            break;
        }
        framed.extend_from_slice(&buf[..n]);
        tokio::time::sleep(PACE).await;
    }
    assert!(
        framed.ends_with(b"\r\n0\r\n\r\n"),
        "cut after {} framed bytes at {:?}",
        framed.len(),
        began.elapsed()
    );
    let body = unchunk(&framed);
    assert_eq!(body.len(), length);
    assert_pattern(&body);
    assert!(began.elapsed() >= T * 4);
    let _ = stop.send(());
}

/// A BODY AT THE KERNEL'S SEND BUFFER SIZE, one byte under it and one over it.
/// The size is a fresh socket's `SO_SNDBUF` on this machine, read here rather
/// than assumed; with the client's 64 KiB receive buffer each body and its head
/// fit in the two kernel buffers, so no write waits. The client reads nothing
/// for two deadlines: each response is complete without it, so no stall is
/// logged, the head clock closes the idle socket, and every byte is still there
/// to read in order, followed by the close. A boundary off by one in deciding
/// "complete" would show here as a cut line or a short body.
#[tokio::test]
async fn bodies_at_the_kernel_send_buffer_size_complete_for_a_reader_that_comes_late() {
    let send_buffer = usize::try_from(
        tokio::net::TcpSocket::new_v4()
            .unwrap()
            .send_buffer_size()
            .unwrap(),
    )
    .unwrap();
    assert!(send_buffer > 1, "a send buffer of {send_buffer} bytes");
    let _sink = crate::emitted::sink();
    let from = crate::emitted::mark();
    let (addr, stop) = start(limits(T, 8)).await;
    for size in [send_buffer - 1, send_buffer, send_buffer + 1] {
        let mut client = connect(addr).await;
        client
            .write_all(
                format!("GET /big/{size}?test=kbuf-{size} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes(),
            )
            .await
            .unwrap();
        tokio::time::sleep(T * 2).await;
        let (head, rest) = head(&mut client).await;
        assert_eq!(content_length(&head), size, "{head:?}");
        let (body, _) = paced(&mut client, rest, size, 1 << 20, Duration::ZERO).await;
        assert_eq!(body.len(), size, "{size}: short");
        assert_pattern(&body);
        let (said, closed) = drain(&mut client, T * 10).await;
        assert!(closed && said.is_empty(), "{size}: {said:?}");
        let tag = format!("test=kbuf-{size}");
        assert!(lines(from, "response cut", &tag).is_empty(), "{size}: cut");
        assert!(
            lines(from, "response closed by the client", &tag).is_empty(),
            "{size}: a complete response was logged as the client's close"
        );
    }
    let _ = stop.send(());
}

/// A CLIENT THAT LEAVES MID-BODY IS ITS OWN CLOSE, NOT A CUT. It reads 256 KiB
/// and drops its socket. Its one slot comes back before any stall deadline
/// could have run out, and the log says the client closed it: one "response
/// closed by the client" line for the request, and no "response cut".
#[tokio::test]
async fn a_client_that_closes_mid_body_is_logged_as_its_close_not_a_cut() {
    let _sink = crate::emitted::sink();
    let from = crate::emitted::mark();
    let long = Duration::from_secs(5);
    let (addr, stop) = start(limits(long, 1)).await;
    let mut client = connect(addr).await;
    client
        .write_all(format!("GET /big/{BIG}?test=leaves HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes())
        .await
        .unwrap();
    let (head, rest) = head(&mut client).await;
    assert_eq!(content_length(&head), BIG);
    let (_part, closed) = paced(&mut client, rest, 256 << 10, CHUNK, Duration::ZERO).await;
    assert!(!closed);
    drop(client);
    let left = Instant::now();
    let mut next = TcpStream::connect(addr).await.unwrap();
    next.write_all(b"GET /ok HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let (said, closed) = drain(&mut next, long * 2).await;
    assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
    assert!(
        left.elapsed() < long,
        "the slot came back at {:?}, as late as a stall cut",
        left.elapsed()
    );
    let left_lines = lines(from, "response closed by the client", "test=leaves");
    assert_eq!(left_lines.len(), 1, "{left_lines:?}");
    assert!(crate::emitted::counts(&left_lines[0], "status", 200));
    assert!(lines(from, "response cut", "test=leaves").is_empty());
    let _ = stop.send(());
}

/// SLOW READERS AT THE CAP. Two slow readers hold both of two slots for over
/// four deadlines each; both get every byte, and a third client is answered
/// only after one of them is done — the cap still holds the next one back.
#[tokio::test]
async fn slow_readers_at_the_cap_are_served_whole_and_the_next_waits() {
    let (addr, stop) = start(limits(T, 2)).await;
    let length = 8 << 20;
    let began = Instant::now();
    let mut readers = Vec::new();
    for _ in 0..2 {
        readers.push(tokio::spawn(async move {
            let mut client = connect(addr).await;
            client
                .write_all(format!("GET /big/{length} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes())
                .await
                .unwrap();
            let (head, rest) = head(&mut client).await;
            let (body, closed) = paced(&mut client, rest, content_length(&head), CHUNK, PACE).await;
            assert!(!closed, "cut after {} bytes", body.len());
            assert_pattern(&body);
            let done = began.elapsed();
            drop(client);
            done
        }));
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    let mut next = TcpStream::connect(addr).await.unwrap();
    next.write_all(b"GET /ok HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let (said, closed) = drain(&mut next, Duration::from_secs(30)).await;
    let answered = began.elapsed();
    assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
    let mut first_done = Duration::MAX;
    for reader in readers {
        first_done = first_done.min(reader.await.unwrap());
    }
    assert!(
        answered + EARLY >= first_done,
        "the third client was answered at {answered:?}, before a slot came back at {first_done:?}"
    );
    assert!(
        first_done >= T * 4,
        "the readers were not slow: {first_done:?}"
    );
    let _ = stop.send(());
}

/// THE COST, COUNTED (D-4975). Over 400 writes into a socket a reader keeps
/// draining, a write that moves bytes leaves the timer exactly where it was
/// and starts no stall. Then, with the reader stopped, the first write that
/// waits starts one stall and resets the timer once; 200 more waiting polls
/// move neither; and once the deadline has passed the next poll is the cut.
/// Each poll is timed and the p50, p99 and max are printed for
/// `docs/06-limits.md` (`--nocapture`); the assertions are the counts.
#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one wrapped socket walked through moving writes, one stall and its cut, \
              each step's counts read from the state the one before left; split, a \
              step could pass on a socket the step before never filled"
)]
async fn a_progressing_write_touches_no_timer_and_a_stall_resets_it_once() {
    use std::future::poll_fn;
    use std::task::Poll;
    use tokio::io::AsyncWrite as _;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let client = connect(addr).await;
    let (io, _) = listener.accept().await.unwrap();
    let slots = std::sync::Arc::new(Slots {
        live: std::sync::atomic::AtomicUsize::new(1),
        cap: 1,
        freed: tokio::sync::Notify::new(),
    });
    let mut wrapped = HeadDeadline::new(io, Slot(slots), T);
    wrapped.deliver();
    assert!(matches!(wrapped.state, HeadState::Delivered));
    let armed = wrapped.alarm.deadline();
    let (keep_reading, mut reading) = tokio::sync::watch::channel(true);
    let reader = tokio::spawn(async move {
        let mut client = client;
        let mut buf = vec![0_u8; 1 << 20];
        let mut read = 0_usize;
        while *reading.borrow() {
            tokio::select! {
                n = client.read(&mut buf) => match n {
                    Ok(0) | Err(_) => break,
                    Ok(n) => read += n,
                },
                _ = reading.changed() => {}
            }
        }
        (client, read)
    });
    let chunk = [7_u8; 1024];
    let mut moved = Vec::with_capacity(400);
    let mut waited = 0_u32;
    let mut resets = 0_u32;
    while moved.len() < 400 {
        let (timer, stall) = (wrapped.alarm.deadline(), wrapped.stalled_since);
        let began = std::time::Instant::now();
        let polled =
            poll_fn(|cx| Poll::Ready(std::pin::Pin::new(&mut wrapped).poll_write(cx, &chunk)))
                .await;
        let took = began.elapsed();
        match polled {
            Poll::Ready(Ok(n)) => {
                assert_eq!(n, chunk.len());
                moved.push(took);
                assert!(
                    wrapped.stalled_since.is_none(),
                    "a moving write is no stall"
                );
                assert_eq!(
                    wrapped.alarm.deadline(),
                    timer,
                    "a moving write touched the timer"
                );
            }
            Poll::Ready(Err(why)) => panic!("{why}"),
            Poll::Pending => {
                waited += 1;
                if stall.is_some() {
                    assert_eq!(wrapped.alarm.deadline(), timer, "a stall reset twice");
                } else if wrapped.alarm.deadline() != timer {
                    resets += 1;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }
    }
    assert!(resets <= waited, "{resets} resets for {waited} waits");
    if waited == 0 {
        assert_eq!(wrapped.alarm.deadline(), armed, "no wait, no reset");
    }
    assert_eq!(wrapped.written, 400 * 1024);
    keep_reading.send(false).unwrap();
    let (_client, _read) = reader.await.unwrap();
    // Fill the socket until a write waits.
    let mut first_wait = None;
    for _ in 0..100_000 {
        let polled =
            poll_fn(|cx| Poll::Ready(std::pin::Pin::new(&mut wrapped).poll_write(cx, &chunk)))
                .await;
        if polled.is_pending() {
            first_wait = Some(
                wrapped
                    .stalled_since
                    .expect("a waiting write starts a stall"),
            );
            break;
        }
    }
    let since = first_wait.expect("the socket filled");
    let stall_deadline = wrapped.alarm.deadline();
    assert_eq!(
        stall_deadline,
        since + T,
        "one reset, to the stall's deadline"
    );
    let mut pending = Vec::with_capacity(200);
    for _ in 0..200 {
        let began = std::time::Instant::now();
        let polled =
            poll_fn(|cx| Poll::Ready(std::pin::Pin::new(&mut wrapped).poll_write(cx, &chunk)))
                .await;
        pending.push(began.elapsed());
        assert!(
            polled.is_pending(),
            "the stopped reader's socket stays full"
        );
        assert_eq!(wrapped.stalled_since, Some(since), "no second stall");
        assert_eq!(wrapped.alarm.deadline(), stall_deadline, "no second reset");
    }
    // The kernel may still grow a socket's send buffer while it is full, and a
    // write it then takes is progress, which ends that stall honestly. So the
    // stall is waited out until the deadline finds the socket still full, and
    // the poll that does is the cut; every progress in between restarts it.
    let mut deadline = stall_deadline;
    let mut grown = 0_u32;
    loop {
        tokio::time::sleep_until(deadline).await;
        let polled =
            poll_fn(|cx| Poll::Ready(std::pin::Pin::new(&mut wrapped).poll_write(cx, &chunk)))
                .await;
        match polled {
            Poll::Ready(Err(why)) => {
                assert_eq!(why.kind(), std::io::ErrorKind::TimedOut);
                assert!(why.to_string().starts_with("response cut"), "{why}");
                break;
            }
            Poll::Ready(Ok(_)) => {
                assert!(wrapped.stalled_since.is_none(), "progress ends a stall");
                grown += 1;
                assert!(grown < 1000, "the kernel took bytes for ever");
                let mut refilled = None;
                for _ in 0..100_000 {
                    let polled = poll_fn(|cx| {
                        Poll::Ready(std::pin::Pin::new(&mut wrapped).poll_write(cx, &chunk))
                    })
                    .await;
                    if polled.is_pending() {
                        refilled = wrapped.stalled_since;
                        break;
                    }
                }
                deadline = refilled.expect("the socket filled again") + T;
                assert_eq!(wrapped.alarm.deadline(), deadline, "one reset per stall");
            }
            Poll::Pending => panic!("the stall deadline passed and the write still waited"),
        }
    }
    let quantiles = |mut times: Vec<Duration>| {
        times.sort_unstable();
        let at = |q: usize| times[(times.len() * q / 100).min(times.len() - 1)];
        (times.len(), at(50), at(99), times[times.len() - 1])
    };
    let (n, p50, p99, max) = quantiles(moved);
    eprintln!(
        "AH-02 cost: a write that moves 1 KiB, n={n}: p50 {p50:?} p99 {p99:?} max {max:?} ({waited} waits, {resets} resets)"
    );
    let (n, p50, p99, max) = quantiles(pending);
    eprintln!("AH-02 cost: a write that waits, n={n}: p50 {p50:?} p99 {p99:?} max {max:?}");
}
