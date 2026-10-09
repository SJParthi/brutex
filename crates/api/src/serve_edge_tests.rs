#![cfg(test)]
//! D-1202: the edges of the serve path that refused nothing — a word after
//! the address, a repeated query key, a request of any size, a banner that
//! panicked on a closed pipe. Each test fails on the code before D-1202.
#![expect(
    clippy::expect_used,
    reason = "a failed fixture or socket must fail the test by name"
)]

use super::*;

fn parse(args: &[&str]) -> Result<Command, String> {
    let owned: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
    Command::parse(&owned)
}

/// probeapi-4: `serve ADDR --typo-flag 0.0.0.0:80` bound ADDR and served,
/// the rest unread. Every word after a complete command is now refused, by
/// name, and nothing is started.
#[test]
fn a_word_after_a_complete_command_is_refused_by_name() {
    let addr = "127.0.0.1:18794";
    for (args, extra) in [
        (
            vec!["serve", addr, "--typo-flag", "0.0.0.0:80"],
            "--typo-flag",
        ),
        (vec!["serve", addr, "0.0.0.0:80"], "0.0.0.0:80"),
        (vec!["serve", addr, ""], ""),
        (vec!["serve", "[::1]:9100", "serve"], "serve"),
        (vec!["report", "now"], "now"),
        (vec!["report", ""], ""),
        (vec!["report", "report", "report"], "report"),
    ] {
        let why = parse(&args).expect_err("a trailing word must refuse");
        assert!(why.contains("unknown argument"), "{args:?}: {why}");
        assert!(why.contains(&format!("{extra:?}")), "{args:?}: {why}");
        assert!(why.contains("nothing was started"), "{args:?}: {why}");
        assert!(why.contains("usage: api"), "{args:?}: {why}");
    }
    // The address is still judged first: a bad address is not reported as a
    // trailing-word problem, and a good one with nothing after it serves.
    let why = parse(&["serve", "0.0.0.0:80", "x"]).expect_err("non-loopback");
    assert!(why.contains("REFUSED"), "{why}");
    let why = parse(&["serve", "nope", "x"]).expect_err("not an address");
    assert!(why.contains("not a socket address"), "{why}");
    assert_eq!(
        parse(&["serve", addr]),
        Ok(Command::Serve(addr.parse().expect("loopback")))
    );
    assert_eq!(parse(&["serve"]), Ok(Command::Serve(DEFAULT_ADDR)));
    assert_eq!(parse(&[]), Ok(Command::Serve(DEFAULT_ADDR)));
    assert_eq!(parse(&["report"]), Ok(Command::Report));
}

/// probeapi-5: the first repeated key, compared as [`param`] compares it.
#[test]
fn the_first_repeated_query_key_is_found_and_nothing_else_is() {
    for (query, repeated) in [
        ("", None),
        ("a", None),
        ("a=1&b=2", None),
        ("a=1&&b=2&", None),
        ("&&&", None),
        // Undecoded, as `param` reads them: `fe%65d` is not `feed` to it.
        ("feed=1&fe%65d=2", None),
        ("feed=zerodha&feed=dhan", Some("feed")),
        ("feed=dhan&page=2&feed=bogus", Some("feed")),
        ("a&a", Some("a")),
        ("a=&a", Some("a")),
        ("a=1=2&a=3", Some("a")),
        ("=1&=2", Some("")),
        ("b=1&a=1&b=2&a=2", Some("b")),
    ] {
        assert_eq!(repeated_query_key(query), repeated, "{query:?}");
    }
}

/// o1api-4: the allocation-free reader answers exactly as the old one did.
#[test]
fn a_field_is_matched_by_its_whole_key() {
    assert_eq!(param("qq=1&q=2", "q"), "2");
    assert_eq!(param("q", "q"), "");
    assert_eq!(param("q=", "q"), "");
    assert_eq!(param("q=a+b%21", "q"), "a b!");
    assert_eq!(param("x=1", "q"), "");
    assert_eq!(param("", "q"), "");
    assert_eq!(params("m=a&mm=b&m=c&m", "m"), vec!["a", "c"]);
}

/// o1api-4, D-4436: the split-once reader answers every field exactly as
/// [`param`] does, first value winning, a bare key naming nothing, and an
/// escape decoded the same way.
#[test]
fn a_split_query_answers_every_field_as_param_does() {
    let corpus = [
        "qq=1&q=2",
        "q",
        "q=",
        "q=a+b%21",
        "x=1",
        "",
        "a=1&a=2",
        "a&a=2",
        "a=b=c",
        "=x&q=1",
        "&&a=1&&q=%zz",
        "%71=1&q=2",
        "feed=dhan&from=2026-01&to=2026-02&timeframe=1min&sort=c&dir=asc",
    ];
    let names = [
        "q", "qq", "x", "a", "", "%71", "b", "feed", "to", "dir", "absent",
    ];
    for raw in corpus {
        let split = Query::parse(raw);
        for name in names {
            assert_eq!(split.param(name), param(raw, name), "{raw:?} {name:?}");
        }
    }
}

/// What reading a route's fields costs at the request-target ceiling: one
/// [`param`] scan per field against one [`Query`] split and a probe per
/// field, for the eleven fields `/bars/window.json` reads with its fields at
/// the END of an 8 KiB query. A measurement, run on purpose; the numbers are
/// in `docs/06-limits.md`'s D-4436 row. o1api-4.
#[test]
#[ignore = "a latency measurement, run on purpose: see crate::latency"]
fn latency_query_fields_scan_per_field_against_split_once() -> Result<(), String> {
    let tail = "&feed=dhan&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1min\
                &from=2024-01&to=2024-02&sort=c&dir=asc&offset=0&limit=100";
    let mut raw = String::with_capacity(MAX_REQUEST_TARGET_BYTES);
    let mut n = 0;
    while raw.len() + tail.len() + 12 < MAX_REQUEST_TARGET_BYTES - 64 {
        std::fmt::Write::write_fmt(&mut raw, format_args!("pad{n}=x&"))
            .map_err(|e| e.to_string())?;
        n += 1;
    }
    raw.push_str(tail.trim_start_matches('&'));
    let names = [
        "feed",
        "exchange",
        "segment",
        "symbol",
        "timeframe",
        "from",
        "to",
        "sort",
        "dir",
        "offset",
        "limit",
    ];
    let scan = crate::latency::Timed::run(20_000, || {
        let total: usize = names.iter().map(|name| param(&raw, name).len()).sum();
        if total > 0 {
            Ok(())
        } else {
            Err("no field read".to_owned())
        }
    })?;
    let split = crate::latency::Timed::run(20_000, || {
        let fields = Query::parse(&raw);
        let total: usize = names.iter().map(|name| fields.param(name).len()).sum();
        if total > 0 {
            Ok(())
        } else {
            Err("no field read".to_owned())
        }
    })?;
    println!("query of {} bytes, {} fields", raw.len(), n + names.len());
    println!("{}", scan.line("11 fields by param, one scan each"));
    println!("{}", split.line("11 fields by Query::parse, one split"));
    Ok(())
}

/// probeapi-5 and o1api-3, over a real socket and the production layer stack:
/// a repeated key is 400, a target past the cap 414, headers past theirs 431,
/// and each boundary itself is admitted.
#[tokio::test]
async fn a_request_past_its_bounds_or_with_a_repeated_key_is_refused() {
    let root = crate::scratch::path("d1202-bounds");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("an owned root");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let app = router_serving(
        Loaded::new(Site::load(&root.join("masters"), &root)),
        std::sync::Arc::new(assets::Assets::new(&root.join("web"))),
        addr,
    );
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let served = tokio::spawn(serve(
        listener,
        app,
        Box::pin(async move {
            let _ = stopped.await;
            Ok::<(), std::io::Error>(())
        }),
    ));
    let ask = |path: String, extra: String| async move {
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\n{extra}Connection: close\r\n\r\n");
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
    };
    let status = |answer: &str| answer.lines().next().unwrap_or("").to_owned();

    for path in [
        "/audit.json?feed=zerodha&feed=dhan",
        "/audit.json?feed=dhan&feed=bogus",
        "/health?x=1&x",
        "/store.json?feed=dhan&encoding=&encoding=census-tuples-v1",
    ] {
        let answer = ask(path.to_owned(), String::new()).await;
        assert!(status(&answer).contains(" 400 "), "{path}: {answer}");
        assert!(answer.contains("more than once"), "{path}: {answer}");
        assert!(
            answer
                .to_ascii_lowercase()
                .contains("x-frame-options: deny"),
            "{path}: {answer}"
        );
    }
    let once = ask("/health?a=1&&b=2".to_owned(), String::new()).await;
    assert!(!status(&once).contains(" 400 "), "{once}");

    // The target: `/health?p=` plus padding, at the cap and one past it.
    let at_cap = format!("/health?p={}", "a".repeat(MAX_REQUEST_TARGET_BYTES - 10));
    assert_eq!(at_cap.len(), MAX_REQUEST_TARGET_BYTES);
    let answer = ask(at_cap.clone(), String::new()).await;
    assert!(!status(&answer).contains(" 414 "), "{answer}");
    let answer = ask(format!("{at_cap}a"), String::new()).await;
    assert!(status(&answer).contains(" 414 "), "{answer}");
    assert!(answer.contains("reads at most 8192"), "{answer}");

    // The headers: one long header well under, and one past the cap.
    let header = |n: usize| format!("X-Pad: {}\r\n", "b".repeat(n));
    let answer = ask("/health".to_owned(), header(MAX_HEADER_BYTES / 2)).await;
    assert!(!status(&answer).contains(" 431 "), "{answer}");
    let answer = ask("/health".to_owned(), header(MAX_HEADER_BYTES)).await;
    assert!(status(&answer).contains(" 431 "), "{answer}");
    assert!(answer.contains("reads at most 65536"), "{answer}");

    let _ = stop.send(());
    served
        .await
        .expect("task")
        .expect("a graceful shutdown is not a failure");
}

/// The header count is exact at the boundary: names plus values, nothing else.
#[test]
fn the_header_cap_counts_names_and_values_exactly() {
    let uri: axum::http::Uri = "/health".parse().expect("uri");
    let mut headers = axum::http::HeaderMap::new();
    // "x-pad" is 5 bytes; the value fills the rest exactly.
    let fill = |n: usize| axum::http::HeaderValue::from_str(&"c".repeat(n)).expect("ascii");
    headers.insert("x-pad", fill(MAX_HEADER_BYTES - 5));
    assert_eq!(request_bounds_refusal(&uri, &headers), None);
    headers.insert("x-pad", fill(MAX_HEADER_BYTES - 4));
    let (code, _) = request_bounds_refusal(&uri, &headers).expect("one byte over");
    assert_eq!(
        code,
        axum::http::StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE
    );
    // A target with no path-and-query at all (authority form) is zero bytes.
    let bare: axum::http::Uri = "example.com:80".parse().expect("authority");
    assert_eq!(
        request_bounds_refusal(&bare, &axum::http::HeaderMap::new()),
        None
    );
}

/// A writer whose every write fails, as a closed pipe does.
struct Closed;

impl std::io::Write for Closed {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
    }
}

/// probeapi-7: a closed stdout drops the line and says so on stderr; with
/// both closed nothing panics. Before D-1202 this was `println!`, which
/// panicked, and the release profile aborts on a panic with `serve.lock` held.
#[test]
fn a_banner_line_on_a_closed_stream_is_dropped_loudly_and_never_panics() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert!(say_line(
        &mut out,
        &mut err,
        format_args!("listening on {}", 1)
    ));
    assert_eq!(out, b"listening on 1\n");
    assert!(err.is_empty());

    let mut err = Vec::new();
    assert!(!say_line(
        &mut Closed,
        &mut err,
        format_args!("  store: {}", "/s")
    ));
    let said = String::from_utf8(err).expect("utf-8");
    assert!(said.starts_with("stdout is not writable ("), "{said}");
    assert!(
        said.ends_with("a banner line was not shown:   store: /s\n"),
        "{said}"
    );

    assert!(!say_line(&mut Closed, &mut Closed, format_args!("x")));
    assert!(!shout_line(&mut Closed, format_args!("x")));
    let mut err = Vec::new();
    assert!(shout_line(&mut err, format_args!("why {}", 2)));
    assert_eq!(err, b"why 2\n");
}

/// `source` with every `#[cfg(test)]` item removed, by rustfmt's layout: the
/// attribute, any attributes after it, and the item through the closing brace
/// at the attribute's own indentation (or its one line when it ends in `;`).
fn without_test_items(source: &str) -> Vec<&str> {
    let mut kept = Vec::new();
    let mut lines = source.lines();
    while let Some(line) = lines.next() {
        if line.trim_start() != "#[cfg(test)]" {
            kept.push(line);
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let closing = format!("{}}}", " ".repeat(indent));
        let mut in_attribute = false;
        for item in lines.by_ref() {
            let trimmed = item.trim_start();
            if in_attribute || trimmed.starts_with("#[") {
                // An attribute may run over several lines (`#[allow(` ... `)]`).
                in_attribute = !trimmed.ends_with(']');
                continue;
            }
            if trimmed.ends_with(';') && item.len() - trimmed.len() == indent {
                break;
            }
            if item == closing {
                break;
            }
        }
    }
    kept
}

/// probeapi-7: no print macro that can panic is left in this file's
/// production code. The serve path prints through `say!` / `warn_line!`.
///
/// THE WHOLE FILE, LESS ITS TEST ITEMS (P1-12-04). The scan stopped at the
/// first `mod tests {`, and roughly 2,500 lines of production handlers sit
/// after it, between later `#[cfg(test)]` modules, so a print there was never
/// read. Every `#[cfg(test)]` item is removed instead, and a late production
/// handler must still be in what is scanned.
#[test]
fn production_code_in_server_rs_prints_through_the_panic_free_writers() {
    let source = include_str!("server.rs");
    let production = without_test_items(source);
    for late in ["async fn vocab_json(", "async fn calendar_json("] {
        assert!(
            production.iter().any(|line| line.starts_with(late)),
            "`{late}` is production code after the test modules and must be scanned"
        );
    }
    assert!(
        !production
            .iter()
            .any(|line| line.starts_with(concat!("mod ", "tests {"))),
        "the test module is not production code"
    );
    let offenders: Vec<&str> = production
        .into_iter()
        .filter(|line| !line.trim_start().starts_with("//"))
        .filter(|line| {
            ["println!(", "print!(", "eprintln!(", "eprint!("]
                .into_iter()
                .any(|banned| {
                    line.match_indices(banned).any(|(at, _)| {
                        // `eprintln!(` contains `println!(`; judge the whole word.
                        !line
                            .get(..at)
                            .and_then(|head| head.chars().last())
                            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
                    })
                })
        })
        .collect();
    assert!(offenders.is_empty(), "{offenders:#?}");
}
