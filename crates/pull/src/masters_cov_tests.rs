#![cfg(test)]
//! BASE-COV-PULL repair 2 (D-7334): `PublicFetch::get`, the one door the
//! public master files come through, driven against a loopback socket this
//! test opens (no request leaves the machine): a body, a cookie kept and sent
//! back to its own host, a refusal by status, a declared length past the
//! ceiling, a body that is not UTF-8, a body cut short, and a port that does
//! not answer. Every byte is invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::io::{Read as _, Write as _};

use super::*;
use crate::chain::Discovery as _;

/// A loopback server answering one connection per entry of `answers`, in
/// order, each answer closing its connection; every request it read comes
/// back through the receiver, as text.
fn serve(answers: Vec<Vec<u8>>) -> (String, std::sync::mpsc::Receiver<String>) {
    let socket = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let addr = socket.local_addr().expect("an address");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for answer in answers {
            let (mut stream, _) = socket.accept().expect("a connection");
            // The whole head of the request: a GET carries no body.
            let mut request = Vec::new();
            let mut buf = [0_u8; 1024];
            while !request.windows(4).any(|end| end == b"\r\n\r\n") {
                let read = stream.read(&mut buf).expect("the request");
                assert!(read > 0, "the request ended before its head did");
                request.extend_from_slice(&buf[..read]);
            }
            tx.send(String::from_utf8_lossy(&request).into_owned())
                .expect("the test is listening");
            stream.write_all(&answer).expect("the answer is written");
            stream.flush().expect("flushed");
        }
    });
    (format!("http://{addr}"), rx)
}

/// One HTTP/1.1 answer of `status`, with `headers` and `body`, closing.
fn answer(status: &str, headers: &[&str], body: &[u8]) -> Vec<u8> {
    let mut out = format!("HTTP/1.1 {status}\r\nConnection: close\r\n").into_bytes();
    for header in headers {
        out.extend_from_slice(header.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(body);
    out
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime")
}

/// **A BODY, AND A COOKIE KEPT FOR ITS OWN HOST AND SENT BACK TO IT**: the
/// first answer sets a cookie, the second request carries it and the first
/// carried none, and both bodies come back whole.
#[test]
fn a_body_comes_back_and_a_cookie_goes_back_to_its_own_host() {
    let (base, asked) = serve(vec![
        answer(
            "200 OK",
            &["Content-Length: 5", "Set-Cookie: zqsession=one; Path=/"],
            b"first",
        ),
        answer("200 OK", &["Content-Length: 6"], b"second"),
    ]);
    let fetch = PublicFetch::new().unwrap();
    let runtime = runtime();
    let url = format!("{base}/master.csv");
    assert_eq!(runtime.block_on(fetch.get(&url)).unwrap(), "first");
    assert_eq!(runtime.block_on(fetch.get(&url)).unwrap(), "second");
    let first = asked.recv().unwrap().to_ascii_lowercase();
    let second = asked.recv().unwrap().to_ascii_lowercase();
    assert!(!first.contains("cookie:"), "{first}");
    assert!(second.contains("cookie: zqsession=one"), "{second}");
}

/// Every answer the door refuses is refused by name and status: a status
/// that is not success, a declared length past the ceiling (refused before
/// any body is read), a body that is not UTF-8, and a body cut short of its
/// declared length (extremes at once: one byte past the ceiling, and an
/// empty refusal body).
#[test]
fn every_answer_the_door_refuses_is_named_with_its_status() {
    let past = MAX_BODY_BYTES + 1;
    let (base, _asked) = serve(vec![
        answer("404 Not Found", &["Content-Length: 0"], b""),
        answer("200 OK", &[&format!("Content-Length: {past}")], b""),
        answer("200 OK", &["Content-Length: 2"], &[0xFF, 0xFE]),
        answer("200 OK", &["Content-Length: 100"], b"cut"),
    ]);
    let fetch = PublicFetch::new().unwrap();
    let runtime = runtime();
    let url = format!("{base}/master.csv");
    let mut said = Vec::new();
    for _ in 0..4 {
        let refused = runtime.block_on(fetch.get(&url)).unwrap_err();
        said.push((refused.status, refused.detail));
    }
    assert_eq!(said[0].0, Some(404));
    assert!(said[0].1.contains("answered 404"), "{said:?}");
    assert_eq!(said[1].0, Some(200));
    assert!(
        said[1].1.contains(&format!(
            "declared more than the {MAX_BODY_BYTES}-byte ceiling"
        )),
        "{said:?}"
    );
    assert!(said[2].1.contains("body is not UTF-8"), "{said:?}");
    assert_eq!(said[3].0, None, "a body cut short is the transport's");
    assert!(said[3].1.contains("body"), "{said:?}");
}

/// A port nothing listens on is a transport refusal naming the URL, with no
/// status: nothing answered.
#[test]
fn a_port_nothing_answers_is_a_transport_refusal() {
    let socket = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = socket.local_addr().unwrap();
    drop(socket);
    let fetch = PublicFetch::new().unwrap();
    let url = format!("http://{addr}/master.csv");
    let refused = runtime().block_on(fetch.get(&url)).unwrap_err();
    assert_eq!(refused.status, None);
    assert!(refused.detail.starts_with(&url), "{}", refused.detail);
}
