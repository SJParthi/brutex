//! BASE-COV-PULL-PS1 (D-7335): `get_parameter`'s socket half, driven as far
//! as a loopback proxy this test opens and no further. The child of the test
//! is handed that proxy as its `HTTPS_PROXY` (every other proxy variable that
//! could route around it removed), the proxy refuses the tunnel, and the read
//! is refused by the sentence that names the host. Nothing leaves the machine
//! and nothing reaches AWS: the request's only connection is the `CONNECT`
//! this test reads. The identity is invented and the parameter is the generic
//! shape `CLAUDE.md` §8 writes, not a path.
//!
//! And the signature's stamp at an instant before 1970, which no clock of a
//! test host reads on demand, handed to `stamp_at`.

#![allow(clippy::unwrap_used, reason = "a test that cannot panic cannot fail")]

use std::io::{BufRead as _, Write as _};

use super::*;

/// The variable the socket test's child runs its half by.
const CHILD: &str = "BRUTEX_SSM_PS1_SOCKET_CHILD";

/// The socket test's name, which its child runs `--exact`.
const SOCKET: &str =
    "ssm::ps1_tests::a_signed_read_reaches_only_its_proxy_and_its_refusal_names_the_host";

/// The host the one region §8 allows derives, spelled out rather than
/// derived, so a change to the derivation is seen here.
const HOST: &str = "ssm.ap-south-1.amazonaws.com";

/// The parameter the child asks for: §8's generic shape, never a real path.
const NAME: &str = "/org/env/vendor/field";

/// The invented secret half, which must reach neither the proxy nor the
/// refusal.
const SECRET: &str = "ZQ-INVENTED-SECRET";

/// The invented session token: it puts the one header `wire_headers` adds on
/// the request, and must reach neither the proxy nor the refusal.
const TOKEN: &str = "ZQ-INVENTED-SESSION-TOKEN";

/// What the proxy answers the one tunnel it is asked for.
const REFUSED: &[u8] = b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n";

/// **A SIGNED READ REACHES ONLY ITS PROXY, AND ITS REFUSAL NAMES THE HOST.**
/// In a child of this binary whose `HTTPS_PROXY` is a loopback socket of this
/// test's own, `get_parameter` builds the pooled client, signs the read for
/// the one region §8 allows, puts the fixed headers and the session token's
/// header on it and sends it; the proxy is asked to tunnel to the region's
/// host on 443 and nothing else, refuses, and the read comes back
/// `Unreachable`, naming the host and none of the secret, the token or the
/// parameter. The proxy saw neither the signature nor any header the read
/// carries: they travel only inside a tunnel that was never opened.
#[test]
fn a_signed_read_reaches_only_its_proxy_and_its_refusal_names_the_host() {
    if std::env::var_os(CHILD).is_some() {
        refused_through_the_proxy();
        return;
    }
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let proxy = format!("http://{address}");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let head = head_of(&mut stream);
        stream.write_all(REFUSED).unwrap();
        head
    });
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", SOCKET, "--test-threads=1"])
        .env(CHILD, "1")
        .env("HTTPS_PROXY", &proxy)
        .env("HTTPS_PROXY".to_ascii_lowercase(), proxy)
        .env_remove("NO_PROXY")
        .env_remove("NO_PROXY".to_ascii_lowercase())
        .env_remove("REQUEST_METHOD")
        .output()
        .unwrap();
    // A child that never reached the proxy leaves its accept waiting; one
    // connection of this test's own releases it, so that case fails below
    // rather than hanging the join.
    drop(std::net::TcpStream::connect(address));
    let said = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "{said}");
    assert!(said.contains("1 passed"), "{said}");
    let head = server.join().unwrap();
    let tunnel = format!("CONNECT {HOST}:443 HTTP/1.1\r\n");
    assert!(head.starts_with(&tunnel), "{head}");
    for kept in ["AWS4-HMAC-SHA256", "AmazonSSM", SECRET, TOKEN, NAME] {
        assert!(!head.contains(kept), "{kept} reached the proxy: {head}");
    }
}

/// The child's half: an invented identity with a session token reads in the
/// one region it may, and the proxy's refusal is the read's refusal.
fn refused_through_the_proxy() {
    let identity = AwsIdentity {
        key_id: "ZQINVENTEDKEYID".to_owned(),
        secret: SECRET.to_owned(),
        session_token: Some(TOKEN.to_owned()),
    };
    let read = get_parameter(&identity, crate::config::REGION, NAME, "20261103T000000Z");
    let SsmError { detail, kind } = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(read)
        .unwrap_err();
    assert_eq!(kind, SecretError::Unreachable);
    let refusal = format!("{HOST} was not reached: ");
    assert!(detail.starts_with(&refusal), "{detail}");
    for kept in [SECRET, TOKEN, NAME] {
        assert!(!detail.contains(kept), "{kept} is in the refusal: {detail}");
    }
}

/// The head of the one request a client sent: its lines up to the blank line
/// that ends it, or up to the end of the stream if that comes first.
fn head_of(stream: &mut std::net::TcpStream) -> String {
    let mut reader = std::io::BufReader::new(stream);
    let mut head = String::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        head.push_str(&line);
        if line.trim().is_empty() {
            return head;
        }
    }
}

/// **A CLOCK BEFORE 1970 SCOPES NO SIGNATURE.** `now_stamp` hands the system
/// clock to `stamp_at`. An instant one second before the epoch is refused as
/// a read that cannot reach AWS, naming the clock; one hour, one minute and
/// one second into 1970's second day is stamped by the closed form.
#[test]
fn a_clock_before_1970_scopes_no_signature() {
    let epoch = std::time::UNIX_EPOCH;
    let refused = stamp_at(epoch - std::time::Duration::from_secs(1)).unwrap_err();
    assert_eq!(refused.kind, SecretError::Unreachable);
    assert!(
        refused.detail.contains("clock reads before 1970"),
        "{refused}"
    );
    let later = epoch + std::time::Duration::from_secs(86_400 + 3_661);
    assert_eq!(stamp_at(later).unwrap(), "19700102T010101Z");
}
