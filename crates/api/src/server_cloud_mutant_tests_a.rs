//! Cloud mutation pass A over `server.rs` lines 1-3099 and 14290-19458: the
//! serve start-up path (`run_in_over`, the serve lock, the banner, the log
//! directory and level), the connection wrappers (`poll_read`, `poll_write`,
//! `local_addr`), the `/bars` page arms, `locate_contract`, the write gate,
//! `local_host_authority`, `audit_row`, `Addressed::parse`, `Site::reparse`
//! and the cash-session audit refusal.
//!
//! Every server here binds `127.0.0.1:0` or a port the kernel just handed
//! out, and no test contacts a vendor, AWS, or a browser. The banner is
//! stdout-only, so the arms that only print are driven in a CHILD process of
//! this test binary whose environment the test chooses (`set_var` is `unsafe`
//! under edition 2024 and this crate forbids `unsafe`); the child always runs
//! with `BRUTEX_NO_OPEN=1` and `BRUTEX_AUTOPILOT=pause`, so no window opens and
//! no autopilot flies, and the parent reads the banner off the child's stdout.
#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a failed fixture or socket must fail the test by name"
)]

use super::tests::{DHAN_HEAD, GROWW_HEAD, agreeing, body_of, get, masters, served_over};
use super::*;
use std::io::Write as _;

/// A signal that has already fired: the serve stops as soon as it starts.
fn fired() -> Shutdown {
    Box::pin(std::future::ready(Ok(())))
}

/// One command line, owned.
fn argv(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| (*s).to_owned()).collect()
}

/// A fresh, empty directory under this process's scratch prefix.
fn fresh_dir(name: &str) -> PathBuf {
    let dir = crate::scratch::path(&format!("clouda-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// One full-session NSE day on or after the 2026-08-03 cash-auction change,
/// asked of the calendar rather than assumed.
fn auction_era_open_day() -> Day {
    let start = Day::new(2026, 8, 3).expect("a real date").days_from_epoch();
    (start..start + 30)
        .filter_map(|n| Day::from_days(n).ok())
        .find(|day| {
            pull::calendar::kind_of(i64::from(day.days_from_epoch()))
                == pull::calendar::DayKind::Open(pull::calendar::Session::full())
        })
        .expect("a full session within thirty days of the change")
}

/// **A member-failure row names its kind beside its scope**, so a row of
/// zeros reads as one member that did not land, not a run that did nothing.
/// Covers `audit_row`'s `Kind::MemberFailure` arm (server.rs:14336-14338).
#[test]
fn a_member_failure_row_names_its_kind_beside_its_scope() {
    let day = Day::new(2024, 3, 1).expect("a real date");
    let window = pull::session::Window::new(day, day).expect("a one-day window");
    let at = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_709_280_007);
    let record = audit::Record::member_failure(
        audit::Scope::Spot,
        at,
        "NIFTY",
        window,
        "the vendor said no",
    );
    let row = audit_row(audit::Entry {
        ordinal: 7,
        decoded: Ok(record),
        whole_note: None,
    });
    assert_eq!(row.ordinal, 7);
    assert_eq!(row.fault, None);
    assert_eq!(row.scope, "spot · member");
    assert_eq!(row.outcome, "FAILED");
    assert_eq!(row.source, "NIFTY");
    assert_eq!(row.note, "the vendor said no");
    assert_eq!(row.failures, 1);
    assert_eq!(row.window, format!("{day}..={day}"));
}

/// **A cut note is shown whole when the notes file kept it, and with the
/// reason when that file could not be read.** Covers `audit_row`'s
/// `Some(Ok(whole))` and `Some(Err(why))` note arms (server.rs:14362-14366).
#[test]
fn a_cut_note_is_shown_whole_or_with_the_reason_it_cannot_be() {
    let day = Day::new(2024, 3, 1).expect("a real date");
    let window = pull::session::Window::new(day, day).expect("a one-day window");
    let at = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_709_280_007);
    let long =
        "the vendor refused this member for a reason longer than a record holds; ".repeat(30);
    let record = audit::Record::member_failure(audit::Scope::Fno, at, "NIFTY", window, &long);
    assert!(record.note_was_cut(), "the fixture's note must not fit");
    let row = |whole_note| {
        audit_row(audit::Entry {
            ordinal: 3,
            decoded: Ok(record.clone()),
            whole_note,
        })
    };
    assert_eq!(row(Some(Ok(long.clone()))).note, long);
    assert_eq!(
        row(Some(Err("checksum mismatch".to_owned()))).note,
        format!(
            "{}… (cut from {} bytes; the whole note could not be read: checksum mismatch)",
            record.note, record.note_bytes
        )
    );
    assert_eq!(
        row(None).note,
        format!("{}… (cut from {} bytes)", record.note, record.note_bytes)
    );
    assert_eq!(row(None).scope, "expired F&O · member");
}

/// **A feed this build cannot read is refused by the address parse, by name.**
/// Covers `Addressed::parse`'s refusal arm (server.rs:1434-1437), which
/// `/bars.json` and `/gaps.json` answer with.
#[test]
fn an_unknown_feed_is_refused_by_the_address_parse() {
    let refused =
        Addressed::parse("feed=bogus&exchange=NSE&segment=INDEX&symbol=NIFTY&month=2024-01")
            .err()
            .expect("an unknown feed must refuse");
    assert_eq!(refused, "\"bogus\" is not a feed this build can read");
    // And the feed is read before anything else: a bad month behind it does
    // not change the sentence.
    let first = Addressed::parse("feed=nope&month=2024-13")
        .err()
        .expect("still the feed's refusal");
    assert_eq!(first, "\"nope\" is not a feed this build can read");
}

/// **A dated cash-session audit for a name with no exact ISIN is refused**,
/// never answered with an empty schedule. Covers `audit_cash_schedule_window`'s
/// ISIN refusal (server.rs:1534-1539), reached through `recovery_cash_schedule`.
#[tokio::test]
async fn a_cash_audit_window_for_a_name_with_no_isin_is_refused() {
    let dir = fresh_dir("cash-isin-masters");
    let root = fresh_dir("cash-isin-store");
    let site = Site::load(&dir, &root);
    let day = auction_era_open_day();
    let window = pull::session::Window::new(day, day).expect("a one-day window");
    let refused = recovery_cash_schedule(&site, "ZZNOISIN", window)
        .await
        .expect_err("no ISIN is a refusal, not an empty schedule");
    assert_eq!(
        refused,
        "ZZNOISIN: no exact current ISIN for dated cash-session audit"
    );
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&root);
}

/// **A reparse that reads an EMPTY universe keeps the one already held.**
/// Covers the refusal at server.rs:2441-2448 and kills `replace == with !=`,
/// `replace && with ||` (via the successful swap) and `delete !` at 2441.
#[test]
fn a_reparse_to_an_empty_universe_keeps_what_was_held() {
    let full = agreeing("clouda-reparse-full");
    let site = super::tests::site("clouda-reparse", &full);
    let held = site.universe().read.merged.by_key.len();
    let generation = site.universe().generation;
    assert!(held > 0, "the agreeing masters hold instruments");

    let empty = masters("clouda-reparse-empty", Some(GROWW_HEAD), Some(DHAN_HEAD));
    let refused = site
        .reparse(&empty)
        .expect_err("an empty parse must not replace a held universe");
    assert!(
        refused.starts_with(&format!(
            "the masters on disk parsed into an EMPTY universe, so the {held} \
             instrument(s) this process already holds were kept rather than \
             replaced with nothing: "
        )),
        "{refused}"
    );
    assert_eq!(site.universe().read.merged.by_key.len(), held);
    assert_eq!(
        site.universe().generation,
        generation,
        "nothing was swapped"
    );

    // The same parse over the full masters IS swapped: the refusal is about
    // emptiness, not about reparsing.
    assert!(site.reparse(&full).is_ok());
    assert_eq!(site.universe().generation, generation.wrapping_add(1));
    assert_eq!(site.universe().read.merged.by_key.len(), held);
}

/// **The write gate names each ground, and none for a server that writes.**
/// Covers `WriteGate::grounds`' `None` arm (server.rs:16637) and kills the
/// `grounds -> None/Some("")/Some("xyzzy")` and `refuses_writes` mutants.
#[test]
fn the_write_gate_names_each_ground_and_none_for_a_writer() {
    let writer = WriteGate::default();
    assert_eq!(writer.grounds(), None);
    assert!(!writer.refuses_writes());

    let read_only = WriteGate {
        read_only: true,
        unstamped: None,
    };
    assert_eq!(read_only.grounds(), Some("read-only (BRUTEX_READ_ONLY=1)"));
    assert!(read_only.refuses_writes());

    let unstamped = WriteGate {
        read_only: false,
        unstamped: Some("no stamp"),
    };
    assert_eq!(
        unstamped.grounds(),
        Some("unstamped (this build carries no commit stamp)")
    );
    assert!(unstamped.refuses_writes());

    // Both: the operator's word is said first.
    let both = WriteGate {
        read_only: true,
        unstamped: Some("no stamp"),
    };
    assert_eq!(both.grounds(), Some("read-only (BRUTEX_READ_ONLY=1)"));
    assert!(both.refuses_writes());
}

/// **No `Host` names a listener that is not on loopback**, whatever it says.
/// Covers `local_host_authority`'s first refusal (server.rs:16847) and pins
/// every other arm, killing the `-> bool`, `delete !`, `|| with &&`,
/// `!= with ==` and `== with !=` mutants at 16846-16865.
#[test]
fn a_host_on_a_non_loopback_listener_is_never_local() {
    let wide: SocketAddr = "10.1.2.3:8080".parse().expect("an address");
    assert!(!local_host_authority("10.1.2.3:8080", wide));
    assert!(!local_host_authority("localhost:8080", wide));

    let local: SocketAddr = "127.0.0.1:8080".parse().expect("an address");
    for (host, expected) in [
        ("127.0.0.1:8080", true),
        ("localhost:8080", true),
        ("LocalHost:8080", true),
        ("127.0.0.1:8081", false),
        ("localhost:8081", false),
        ("127.0.0.1", false),
        ("user@127.0.0.1:8080", false),
        ("127.0.0.2:8080", false),
        ("[::1]:8080", false),
        ("example.com:8080", false),
        ("not a host", false),
    ] {
        assert_eq!(local_host_authority(host, local), expected, "{host}");
    }

    let six: SocketAddr = "[::1]:9000".parse().expect("an address");
    assert!(local_host_authority("[::1]:9000", six));
    assert!(local_host_authority("localhost:9000", six));
    assert!(!local_host_authority("127.0.0.1:9000", six));
    assert!(!local_host_authority("[::1]:9001", six));
}

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

/// One slot pool with one slot taken, for a hand-built `HeadDeadline`.
fn one_slot() -> (std::sync::Arc<Slots>, Slot) {
    let slots = std::sync::Arc::new(Slots {
        live: std::sync::atomic::AtomicUsize::new(1),
        cap: 1,
        freed: tokio::sync::Notify::new(),
    });
    let slot = Slot(std::sync::Arc::clone(&slots));
    (slots, slot)
}

/// **The limited listener reports its socket's own address.** Covers
/// `LimitedListener::local_addr` (server.rs:17146-17148), which `axum::serve`
/// never calls, and kills `-> Ok(Default::default())`.
#[tokio::test]
async fn the_limited_listener_reports_its_inner_address() {
    let inner = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let bound = inner.local_addr().expect("addr");
    let listener = LimitedListener {
        inner,
        slots: std::sync::Arc::new(Slots {
            live: std::sync::atomic::AtomicUsize::new(0),
            cap: 1,
            freed: tokio::sync::Notify::new(),
        }),
        head_read_timeout: std::time::Duration::from_secs(1),
    };
    let reported = axum::serve::Listener::local_addr(&listener).expect("an address");
    assert_eq!(reported, bound);
    assert_ne!(reported.port(), 0);
}

/// **A plain write through the head deadline reaches the peer, and re-arms
/// the next head's clock only when bytes moved.** hyper writes vectored, so
/// `poll_write` (server.rs:17340-17351) is reached only by a direct writer;
/// this kills the `poll_write -> Poll::Ready(Ok(0|1))` mutants and the
/// `n > 0` guard.
#[tokio::test]
async fn a_direct_write_through_the_head_deadline_rearms_only_when_bytes_moved() {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    let (server, mut client) = socket_pair().await;
    let (slots, slot) = one_slot();
    let mut wrapped = HeadDeadline::new(server, slot, std::time::Duration::from_secs(30));

    // AN EMPTY WRITE MOVES NOTHING AND RE-ARMS NOTHING.
    wrapped.state = HeadState::Delivered;
    let none = wrapped.write(b"").await.expect("an empty write");
    assert_eq!(none, 0);
    assert!(matches!(wrapped.state, HeadState::Delivered));

    // A REAL WRITE: every byte reported, every byte delivered, clock re-armed.
    let wrote = wrapped
        .write(b"HTTP/1.1 200 OK\r\n")
        .await
        .expect("a write");
    assert_eq!(wrote, 17);
    assert!(matches!(
        wrapped.state,
        HeadState::Awaiting {
            progress: 0,
            partial: false,
            ..
        }
    ));
    // THE NEXT HEAD'S CLOCK RUNS FROM THE WRITE, a whole timeout ahead (kills
    // `replace + with -` in `rearm`, server.rs:17265).
    let HeadState::Awaiting { deadline, .. } = wrapped.state else {
        panic!("re-armed above");
    };
    assert!(deadline > tokio::time::Instant::now() + std::time::Duration::from_secs(25));
    let mut got = [0_u8; 17];
    client
        .read_exact(&mut got)
        .await
        .expect("the peer reads it");
    assert_eq!(&got, b"HTTP/1.1 200 OK\r\n");

    drop(wrapped);
    assert_eq!(slots.live.load(std::sync::atomic::Ordering::Acquire), 0);
}

/// **A peer that resets the connection reaches the reader as its own error**,
/// not as a timeout and not as an end of stream. Covers `poll_read`'s
/// `Poll::Ready(Err(why))` arm (server.rs:17322).
#[tokio::test]
async fn a_reset_peer_reaches_the_reader_as_its_own_error() {
    use tokio::io::AsyncReadExt as _;
    let (server, client) = socket_pair().await;
    let (_slots, slot) = one_slot();
    let mut wrapped = HeadDeadline::new(server, slot, std::time::Duration::from_secs(30));
    // SO_LINGER of zero turns the close into a reset.
    #[allow(deprecated, reason = "the one portable way tokio offers to send RST")]
    client
        .set_linger(Some(std::time::Duration::ZERO))
        .expect("linger");
    drop(client);
    let mut buf = [0_u8; 16];
    let why = tokio::time::timeout(std::time::Duration::from_secs(5), wrapped.read(&mut buf))
        .await
        .expect("the reset arrives at once")
        .expect_err("a reset is an error");
    assert_eq!(why.kind(), std::io::ErrorKind::ConnectionReset, "{why}");
}

/// **A deadline that has passed while the socket waits ends the head**, even
/// when the clock check before the read had not yet seen it. The two are read
/// a few instructions apart in production; here the alarm is made due while
/// the recorded deadline still lies ahead, which is exactly the state that
/// race leaves. Covers `poll_read`'s pending-arm expiry (server.rs:17328-17331).
#[tokio::test]
async fn a_due_alarm_while_the_socket_waits_expires_the_head() {
    use tokio::io::AsyncReadExt as _;
    let (server, mut client) = socket_pair().await;
    let (_slots, slot) = one_slot();
    let mut wrapped = HeadDeadline::new(server, slot, std::time::Duration::from_hours(1));
    wrapped.state = HeadState::Awaiting {
        deadline: tokio::time::Instant::now() + std::time::Duration::from_hours(1),
        progress: 0,
        partial: true,
    };
    wrapped.alarm = Box::pin(tokio::time::sleep(std::time::Duration::ZERO));
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let mut buf = [0_u8; 16];
    let why = tokio::time::timeout(std::time::Duration::from_secs(5), wrapped.read(&mut buf))
        .await
        .expect("the due alarm answers at once")
        .expect_err("an expired head is an error");
    assert_eq!(why.kind(), std::io::ErrorKind::TimedOut);
    assert_eq!(
        why.to_string(),
        "the request head did not arrive before the head-read deadline"
    );
    // `partial` was set, so the client is told before the close.
    drop(wrapped);
    let mut said = String::new();
    client.read_to_string(&mut said).await.expect("the reply");
    assert!(said.starts_with("HTTP/1.1 408 Request Timeout"), "{said:?}");
}

/// **The served log directory and level are their parsers over this
/// process's environment.** Covers `served_log_dir` (server.rs:19055-19061)
/// and `served_log_level` (18789-18796), and kills their `-> Default` mutants.
#[test]
fn the_served_log_directory_and_level_are_their_parsers_over_the_environment() {
    let root = fresh_dir("served-log-dir");
    let served = served_log_dir(&root);
    assert_eq!(
        served,
        log_dir_from(
            std::env::var_os(LOG_DIR_ENV),
            std::env::current_dir().ok().as_deref(),
            &root
        )
    );
    assert_ne!(served, Ok(PathBuf::new()));
    if std::env::var_os(LOG_DIR_ENV).is_none() {
        // `cargo test` runs a unit test in `crates/api`, which is a crate and
        // not the workspace root, so the log goes beneath the store.
        assert_eq!(served, Ok(telemetry::dir_beneath_store(&root)));
    }

    let raw = std::env::var_os(LOG_LEVEL_ENV).map(|raw| raw.to_string_lossy().into_owned());
    let (config, note) = served_log_level();
    let (want_config, want_note) = log_level_from(raw.as_deref());
    assert_eq!(config, want_config);
    assert_eq!(note, want_note);
    assert!(!note.is_empty() && note != "xyzzy", "{note}");
    let _ = std::fs::remove_dir_all(&root);
}

/// **The banner's universe verdict is the read's own.** Kills
/// `announce_universe -> true/false` and `delete !` (server.rs:18227-18240).
#[test]
fn the_banner_universe_verdict_is_the_reads_own() {
    let clean = Site::load(
        &agreeing("clouda-announce-clean"),
        &fresh_dir("announce-clean"),
    );
    assert!(announce_universe(&clean.universe().read));
    let missing = masters(
        "clouda-announce-missing",
        Some(&format!(
            "{GROWW_HEAD}NSE,CASH,,NIFTY,IDX,,NIFTY,,,NSE-NIFTY\n"
        )),
        None,
    );
    let degraded = Site::load(&missing, &fresh_dir("announce-missing"));
    assert!(!degraded.universe().read.is_clean());
    assert!(!announce_universe(&degraded.universe().read));
}

/// **`/bars` answers through the router with the page's own refusals**: a
/// month that is not named, and a rung that is named and unknown, each a
/// `400` on the bars page carrying its own sentence. Covers `bars_get`
/// (server.rs:14634-14643), `bars_month`'s absent arm (14774) and
/// `bars_html`'s unknown-rung arm (15279-15295).
#[tokio::test]
async fn the_bars_route_refuses_an_unnamed_month_and_an_unknown_rung() {
    let dir = agreeing("clouda-bars-route");
    let built = Loaded::new(super::tests::site("clouda-bars-route", &dir));
    served_over("clouda-bars-route", built, |addr| async move {
        let rung = get(
            addr,
            "/bars?feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&month=2024-03&timeframe=7x",
        )
        .await;
        assert!(rung.starts_with("HTTP/1.1 400"), "{rung}");
        let page = body_of(&rung);
        assert!(
            page.contains("&quot;7x&quot; is not a rung this store has a directory for"),
            "{page}"
        );
        assert!(page.contains("NIFTY"), "{page}");

        let month = get(
            addr,
            "/bars?feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1min",
        )
        .await;
        assert!(month.starts_with("HTTP/1.1 400"), "{month}");
        assert!(body_of(&month).contains("no month was named"), "{month}");
        assert!(!body_of(&month).contains("is not a rung"), "{month}");
    })
    .await;
}

/// **`/masters.js` is read from disk per request**: absent is a `404` that
/// names the file, present is the file. Covers the route closure in
/// `route_table` (server.rs:16088-16091).
#[tokio::test]
async fn the_masters_script_route_reads_the_file_on_each_request() {
    let dir = agreeing("clouda-masters-js");
    let built = Loaded::new(super::tests::site("clouda-masters-js", &dir));
    served_over("clouda-masters-js", built, |addr| async move {
        let absent = get(addr, "/masters.js").await;
        assert!(absent.starts_with("HTTP/1.1 404"), "{absent}");
        assert!(body_of(&absent).contains("could not be read"), "{absent}");
        assert!(body_of(&absent).contains("masters.js"), "{absent}");

        let script = crate::scratch::path("front-clouda-masters-js").join("masters.js");
        std::fs::write(&script, "export const masters = 41;").expect("the script");
        let present = get(addr, "/masters.js").await;
        assert!(present.starts_with("HTTP/1.1 200"), "{present}");
        assert_eq!(body_of(&present), "export const masters = 41;");
    })
    .await;
}

/// **A form past the distinct-field bound is refused before any handler**,
/// with its own sentence rather than the repeated-key one. Covers
/// `one_value_per_form_field`'s `FormKeys::TooMany` arm (server.rs:15875).
#[tokio::test]
async fn a_served_form_past_the_distinct_field_bound_is_refused() {
    let dir = agreeing("clouda-too-many-keys");
    let built = Loaded::new(super::tests::site("clouda-too-many-keys", &dir));
    served_over("clouda-too-many-keys", built, |addr| async move {
        let form = (0..=MAX_DISTINCT_FORM_KEYS)
            .map(|i| format!("k{i}=1"))
            .collect::<Vec<_>>()
            .join("&");
        let host = format!("localhost:{}", addr.port());
        let request = format!(
            "POST /pull/run HTTP/1.1\r\nHost: {host}\r\nOrigin: http://{host}\r\n\
             Sec-Fetch-Site: same-origin\r\n\
             Content-Type: application/x-www-form-urlencoded\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{form}",
            form.len()
        );
        let answer = tokio::task::spawn_blocking(move || {
            use std::io::Read as _;
            let mut s = std::net::TcpStream::connect(addr).expect("connect");
            s.write_all(request.as_bytes()).expect("write");
            let mut buf = String::new();
            s.read_to_string(&mut buf).expect("read");
            buf
        })
        .await
        .expect("the client thread");
        assert!(answer.starts_with("HTTP/1.1 400"), "{answer}");
        assert_eq!(
            body_of(&answer),
            format!(
                "REFUSED — the form names more than {MAX_DISTINCT_FORM_KEYS} distinct fields, \
                 and no form this server reads has that many. Nothing was read or run.\n"
            )
        );
    })
    .await;
}

/// **`/health` ends with exactly one build line after the report's own
/// newline.** Kills `delete !` at server.rs:1961, which would put a blank
/// line before `build`.
#[tokio::test]
async fn health_puts_one_build_line_after_the_report() {
    let dir = agreeing("clouda-health");
    let built = Loaded::new(super::tests::site("clouda-health", &dir));
    served_over("clouda-health", built, |addr| async move {
        let answer = get(addr, "/health").await;
        assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
        let body = body_of(&answer);
        assert!(body.starts_with("ok\n"), "{body}");
        assert!(
            body.ends_with(&format!(
                " eligibility conflicts\nbuild {}\n",
                build_label()
            )),
            "{body:?}"
        );
        assert!(!body.contains("\n\nbuild "), "{body:?}");
    })
    .await;
}

/// A store whose GDFL census holds, or fails to hold, one NIFTY option.
struct ContractStore {
    root: PathBuf,
    site: Site,
}

/// The NIFTY option every contract case asks for, and its query.
const OPTION: &str = "2024-03-07-2200000-CE";
const OPTION_QUERY: &str =
    "feed=gdfl&symbol=NIFTY&contract=2024-03-07-2200000-CE&timeframe=1s&month=2024-03";

impl ContractStore {
    /// `held` names the exchanges the census files the option under; `None`
    /// writes no GDFL counter at all, and `Some(&[])` with `corrupt` a
    /// counter that will not decode.
    fn new(name: &str, held: Option<&[brutex_core::instrument::Exchange]>, corrupt: bool) -> Self {
        use pull::manifest::{Entry, EntryKey, Manifest, manifest_path};
        let root = fresh_dir(name);
        let masters = root.join("masters");
        std::fs::create_dir_all(&masters).expect("empty offline masters");
        let path = manifest_path(&root, Vendor::Gdfl);
        std::fs::create_dir_all(path.parent().expect("manifest parent")).expect("manifest dir");
        if corrupt {
            std::fs::write(&path, [0xFF_u8; 16]).expect("a damaged counter");
        } else if let Some(exchanges) = held {
            let mut manifest = Manifest::open(Vendor::Gdfl, &[], &[]).expect("a manifest");
            let day = Day::new(2024, 3, 1).expect("a real date");
            let open = i64::from(day.days_from_epoch()) * 86_400_000_000 + 13_500_000_000;
            for exchange in exchanges {
                manifest
                    .record(Entry {
                        key: EntryKey {
                            contract: Some(
                                brutex_core::instrument::Contract::parse(OPTION).expect("c"),
                            ),
                            exchange: *exchange,
                            segment: brutex_core::instrument::Segment::Fno,
                            symbol: brutex_core::symbol::Symbol::new("NIFTY").expect("symbol"),
                            timeframe: store::path::Timeframe::SECOND_1,
                            month: store::path::YearMonth::new(2024, 3).expect("month"),
                        },
                        rows: 2,
                        first_ts_micros: open,
                        last_ts_micros: open + 1_000_000,
                    })
                    .expect("one held month");
            }
            let staged = path.with_extension("staged");
            std::fs::write(&staged, manifest.image()).expect("stage");
            std::fs::rename(&staged, &path).expect("install");
        }
        let site = Site::load(&masters, &root);
        Self { root, site }
    }
}

impl Drop for ContractStore {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// **A contract filed under BOTH exchanges is refused as ambiguous, naming
/// both**, never opened at whichever sorted first. Covers `locate_contract`'s
/// `[first, second, ..]` arm (server.rs:14862-14866).
#[test]
fn a_contract_held_on_two_exchanges_is_ambiguous_and_names_both() {
    use brutex_core::instrument::Exchange;
    let store = ContractStore::new("contract-two", Some(&[Exchange::Nse, Exchange::Bse]), false);
    let (status, page) = bars_html(&store.site, OPTION_QUERY);
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST, "{page}");
    assert!(
        page.contains("holds NIFTY as NSE/FNO and as BSE/FNO, two different series"),
        "{page}"
    );

    // ONE exchange places it: the same census with only NSE answers past the
    // placement (to the absent bar file, a 404 naming the NSE path).
    let one = ContractStore::new("contract-one", Some(&[Exchange::Nse]), false);
    let (status, page) = bars_html(&one.site, OPTION_QUERY);
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND, "{page}");
    assert!(
        page.contains("/NSE/FNO/NIFTY/2024-03-07-2200000-CE/1s/2024-03"),
        "{page}"
    );
}

/// **The asked feed's own unreadable census refuses the contract placement
/// with 503**, naming the census, never guessing an exchange. Covers
/// `locate_contract`'s `Census::Unreadable` arm (server.rs:14835-14838).
#[test]
fn an_unreadable_asked_census_refuses_the_contract_placement() {
    let store = ContractStore::new("contract-unreadable", None, true);
    let (status, page) = bars_html(&store.site, OPTION_QUERY);
    assert_eq!(
        status,
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "{page}"
    );
    // The page escapes the apostrophe (`render::escape`).
    assert!(
        page.contains("the asked feed&#39;s own census could not be read: "),
        "{page}"
    );
    assert!(page.contains("No path was guessed."), "{page}");
    assert!(!page.contains("/FNO/NIFTY/2024-03-07"), "{page}");
}

/// **No counter at all for the asked feed is "not held", naming every
/// contract file asked about.** Covers `locate_contract`'s
/// `Census::Absent | None` arm (server.rs:14858).
#[test]
fn an_absent_asked_census_holds_no_contract_and_names_the_files_tried() {
    let store = ContractStore::new("contract-absent", None, false);
    let (status, page) = bars_html(&store.site, OPTION_QUERY);
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND, "{page}");
    assert!(
        page.contains("holds no NIFTY contract at this rung and month"),
        "{page}"
    );
    assert!(
        page.contains("/NSE/FNO/NIFTY/2024-03-07-2200000-CE/1s/2024-03.bin"),
        "{page}"
    );
    assert!(
        page.contains("/BSE/FNO/NIFTY/2024-03-07-2200000-CE/1s/2024-03.bin"),
        "{page}"
    );
}

/// The address every lock stamp in this file names.
fn lock_addr() -> SocketAddr {
    "127.0.0.1:0".parse().expect("an address")
}

/// **A serve over a real, empty store root runs the whole start and exits
/// on the universe's verdict.** Every line of `run_in_over` past the store
/// lock (server.rs:19226-19432) was unreached: the suite's only full serve
/// reads the environment's store root, which on a fresh host does not exist.
/// Asserts the exit code (kills `run_in_over -> 0/1` with the DEGRADED case
/// below), the first event's fields (kills `&& with ||` at 19310 through
/// `autopilot_flies`), the stamp the lock left, and that the lock and the
/// in-process key are released (kills `Drop for ServeLock -> ()`).
#[tokio::test]
async fn a_serve_over_a_real_store_root_runs_the_whole_start_and_exits_ok() {
    let _shared = crate::emitted::sink();
    let root = fresh_dir("serve-clean");
    let canonical = std::fs::canonicalize(&root).expect("the root resolves");
    let dir = agreeing("clouda-serve-clean");
    let from = crate::emitted::mark();
    let code = run_in_over(
        &dir,
        Ok(root.clone()),
        &argv(&["serve", "127.0.0.1:0"]),
        fired(),
    )
    .await;
    assert_eq!(code, OK, "a clean universe served and stopped is OK");

    let store = canonical.display().to_string();
    let listening = crate::emitted::landed(from, "api.serve", "listening");
    let mine = listening
        .iter()
        .find(|record| crate::emitted::says(record, "store", &store))
        .expect("the first event names this store");
    assert_eq!(mine.level, telemetry::Level::Info);
    let text = |key: &str| {
        mine.field(key)
            .and_then(telemetry::OwnedValue::as_str)
            .map(str::to_owned)
    };
    assert_eq!(text("addr").as_deref(), Some("127.0.0.1:0"));
    assert_eq!(text("build").as_deref(), Some(build_label()));
    assert_eq!(text("masters"), Some(dir.display().to_string()));
    assert_eq!(text("universe").as_deref(), Some("ok"));
    assert_eq!(
        mine.field("autopilot_flies")
            .and_then(telemetry::OwnedValue::as_bool),
        Some(false),
        "no BRUTEX_AUTOPILOT=run in this process, so it does not fly"
    );

    // THE STAMP the holder wrote, cut to its own length, is still on disk.
    assert_eq!(
        std::fs::read_to_string(canonical.join(SERVE_LOCK)).expect("the lock file"),
        format!("addr=127.0.0.1:0 pid={}\n", std::process::id())
    );
    // AND THE LOCK WENT WITH THE SERVE: the next take holds the file itself.
    let again = take_serve_lock(&root, lock_addr()).expect("the store is free again");
    assert!(again.held.is_some(), "a file lock, not an in-process pass");
    drop(again);
    let _ = std::fs::remove_dir_all(&root);
}

/// **A serve over a degraded universe exits DEGRADED and says so, and a
/// recovery pointer that will not read is refused at start without stopping
/// the serve.** Covers the not-clean banner (server.rs:18230-18238), the
/// `RecoveryStart::Refused` arm (19415) and the DEGRADED stop.
#[tokio::test]
async fn a_degraded_serve_exits_degraded_and_refuses_an_unreadable_recovery() {
    let _shared = crate::emitted::sink();
    let root = fresh_dir("serve-degraded");
    let canonical = std::fs::canonicalize(&root).expect("the root resolves");
    let recovery = canonical.join("audit").join("recovery-v1");
    std::fs::create_dir_all(&recovery).expect("the recovery directory");
    std::fs::write(recovery.join("active.bin"), b"").expect("an empty pointer");
    let dir = masters(
        "clouda-serve-degraded",
        Some(&format!(
            "{GROWW_HEAD}NSE,CASH,,NIFTY,IDX,,NIFTY,,,NSE-NIFTY\n"
        )),
        None,
    );
    let from = crate::emitted::mark();
    let code = run_in_over(
        &dir,
        Ok(root.clone()),
        &argv(&["serve", "127.0.0.1:0"]),
        fired(),
    )
    .await;
    assert_eq!(code, DEGRADED);

    let store = canonical.display().to_string();
    let listening = crate::emitted::landed(from, "api.serve", "listening");
    let mine = listening
        .iter()
        .find(|record| crate::emitted::says(record, "store", &store))
        .expect("the first event names this store");
    assert_eq!(
        mine.field("universe")
            .and_then(telemetry::OwnedValue::as_str),
        Some("DEGRADED")
    );
    let stopped = crate::emitted::landed(
        from,
        "api.server",
        "the server stopped after serving a DEGRADED universe",
    );
    assert!(
        stopped.iter().any(|record| {
            record.level == telemetry::Level::Error
                && crate::emitted::counts(record, "exit", u64::from(DEGRADED))
        }),
        "{stopped:?}"
    );
    let refused = crate::emitted::landed(from, "api.recovery", "recovery not resumed");
    assert!(
        refused.iter().any(|record| {
            record.level == telemetry::Level::Error
                && crate::emitted::says(
                    record,
                    "why",
                    "active recovery pointer is empty; prior activation is unverified",
                )
        }),
        "{refused:?}"
    );
    // THE POINTER IS LEFT EXACTLY AS IT WAS: nothing claimed, nothing written.
    assert_eq!(
        std::fs::read(recovery.join("active.bin")).expect("still there"),
        b""
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **A second take inside one process shares the key and takes no second
/// file lock**, and the take after both are gone holds the file again.
/// Covers the in-process arm of `take_serve_lock` (server.rs:18373-18377).
#[test]
fn a_second_take_in_one_process_shares_the_key_without_a_file_lock() {
    let root = fresh_dir("serve-shared");
    let first = take_serve_lock(&root, lock_addr()).expect("a free store");
    assert!(first.held.is_some());
    let second = take_serve_lock(&root, lock_addr()).expect("the same process may serve twice");
    assert!(
        second.held.is_none(),
        "an in-process pass, not a second lock"
    );
    assert_eq!(second.root, first.root);
    assert_eq!(
        second.root,
        std::fs::canonicalize(&root).expect("the root resolves")
    );
    drop(second);
    drop(first);
    let third = take_serve_lock(&root, lock_addr()).expect("free once both are gone");
    assert!(third.held.is_some());
    drop(third);
    let _ = std::fs::remove_dir_all(&root);
}

/// **A store root that stats as a directory but will not resolve to a
/// physical path is refused, and nothing is served.** A directory removed
/// while a descriptor holds it is still a directory through
/// `/proc/self/fd/N`, and `realpath` cannot name it. Covers the canonicalize
/// refusal in `take_serve_lock` (server.rs:18348-18353).
#[cfg(target_os = "linux")]
#[tokio::test]
async fn a_store_root_that_will_not_resolve_is_refused_and_nothing_is_served() {
    use std::os::fd::AsRawFd as _;
    let _shared = crate::emitted::sink();
    let doomed = fresh_dir("serve-unresolvable");
    let handle = std::fs::File::open(&doomed).expect("open the directory");
    std::fs::remove_dir(&doomed).expect("remove it while it is held");
    let root = PathBuf::from(format!("/proc/self/fd/{}", handle.as_raw_fd()));
    assert!(std::fs::metadata(&root).expect("it still stats").is_dir());
    assert!(std::fs::canonicalize(&root).is_err());

    let why = take_serve_lock(&root, lock_addr()).expect_err("no physical path, no lock");
    let head = format!(
        "REFUSED: the configured store root {} could not be resolved to its existing \
         physical path, so no fallback identity will be used — ",
        root.display()
    );
    assert!(why.starts_with(&head), "{why}");

    let from = crate::emitted::mark();
    let code = run_in_over(
        &agreeing("clouda-serve-unresolvable"),
        Ok(root.clone()),
        &argv(&["serve", "127.0.0.1:0"]),
        fired(),
    )
    .await;
    assert_eq!(code, FAILED);
    let refused = crate::emitted::landed(from, "api.serve", "refused: this store cannot be served");
    assert!(
        refused.iter().any(|record| {
            record.level == telemetry::Level::Error
                && crate::emitted::says(record, "why", "could not be resolved to its existing")
        }),
        "{refused:?}"
    );
    drop(handle);
}

/// **A stamp the host refuses after the lock is held is cleared and SAID**,
/// and the lock is kept. `/proc/self/oom_score_adj` takes an advisory lock and
/// a truncate but refuses `addr=…` as a number, so the write fails and the
/// clear succeeds on one descriptor -- the split `stamp_serve_lock`'s doc
/// calls unreachable by fixture. Covers server.rs:18498-18499 (the
/// `note_unstamped_lock` call) and kills `stamp_serve_lock -> Ok(())`.
#[cfg(target_os = "linux")]
#[test]
fn a_stamp_the_host_refuses_after_the_lock_is_held_is_cleared_and_said() {
    let _shared = crate::emitted::sink();
    let root = fresh_dir("serve-unstampable");
    let canonical = std::fs::canonicalize(&root).expect("the root resolves");
    std::os::unix::fs::symlink("/proc/self/oom_score_adj", root.join(SERVE_LOCK))
        .expect("the lock name points at a file that refuses the stamp");
    let before = std::fs::read_to_string("/proc/self/oom_score_adj").expect("readable");

    let from = crate::emitted::mark();
    let taken = take_serve_lock(&root, lock_addr()).expect("held, though unstamped");
    assert!(taken.held.is_some(), "mutual exclusion is intact");
    let warned = crate::emitted::landed(
        from,
        "api.serve",
        "the serve lock is held but could not be stamped",
    );
    let head = format!(
        "the one-server lock {} is held, but this instance's address and pid could \
         not be written into it — ",
        canonical.join(SERVE_LOCK).display()
    );
    assert!(
        warned.iter().any(|record| {
            record.level == telemetry::Level::Warn && crate::emitted::says(record, "why", &head)
        }),
        "{warned:?}"
    );
    drop(taken);
    assert_eq!(
        std::fs::read_to_string("/proc/self/oom_score_adj").expect("readable"),
        before,
        "the refused write changed nothing about this process"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Set in a child of this binary: the test runs its child half.
const CHILD: &str = "BRUTEX_CLOUDA_CHILD";
/// The masters directory the parent built for the child.
const CHILD_MASTERS: &str = "BRUTEX_CLOUDA_MASTERS";
/// The store root the parent built for the child.
const CHILD_STORE: &str = "BRUTEX_CLOUDA_STORE";

/// Whether this process is a child that `rerun_child` started.
fn in_child() -> bool {
    std::env::var_os(CHILD).is_some()
}

/// Ends the harness's `test NAME ... ` line, which it leaves open while the
/// test runs, so the child's first banner line is a whole line of its own.
fn end_harness_line() {
    println!();
}

/// A path the parent handed the child through `name`.
fn handed(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).expect("the parent names it"))
}

/// Runs `test` alone in a child of this test binary with `env` added, and
/// returns its stdout and stderr.
///
/// `BRUTEX_NO_OPEN=1` is set LAST, so no case can lift it: no child opens a
/// browser. `BRUTEX_AUTOPILOT=pause` is set last too, unless the case is
/// read-only AND names the autopilot word itself: a read-only server grounds
/// its autopilot on its own (`Control::serving` pauses a server that refuses
/// writes, and `fly` dwells while paused), so that one shape may say `run` to
/// prove the banner does not announce a flight the gate forbids. Every child
/// serve stops at once besides, and its autopilot task is aborted long before
/// the twenty-second grace that precedes any vendor call.
///
/// The child must print the harness's own `1 passed` line, because `--exact`
/// with a name that matches nothing exits zero having tested nothing (the
/// reason `crate::isolated::rerun` gives).
fn rerun_child(test: &str, env: &[(&str, &std::ffi::OsStr)]) -> (String, String) {
    let mut command =
        std::process::Command::new(std::env::current_exe().expect("this test binary"));
    command
        .args([
            "--exact",
            &format!("server::cloud_mutant_tests_a::{test}"),
            "--nocapture",
            "--test-threads=1",
        ])
        // A FRONT END THE CASE OWNS unless it names one: never the shell's
        // `BRUTEX_WEB`, which could be empty and refuse a serve for a reason
        // the case is not about. A folder that does not exist serves as an
        // absent build, which every case but the banner's accepts.
        .env(assets::WEB_ENV, crate::scratch::path("clouda-no-front-end"))
        .envs(env.iter().copied())
        .env(CHILD, "1")
        .env(NO_OPEN_ENV, "1");
    let read_only = env
        .iter()
        .any(|&(key, value)| key == READ_ONLY_ENV && value == "1");
    let names_autopilot = env.iter().any(|&(key, _)| key == autopilot::AUTOPILOT_ENV);
    if !(read_only && names_autopilot) {
        command.env(autopilot::AUTOPILOT_ENV, "pause");
    }
    let output = command.output().expect("the child test process starts");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "child {test} failed:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("1 passed"),
        "child {test} did not run exactly one test:\n{stdout}\n{stderr}"
    );
    (stdout, stderr)
}

/// Whether `text` holds `line` as a whole line.
fn has_line(text: &str, line: &str) -> bool {
    text.lines().any(|l| l == line)
}

/// One serve of `dir` over `root` at `addr` on a runtime of its own, as a
/// child runs it.
fn serve_once(dir: &Path, root: &Path, addr: &str) -> u8 {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime")
        .block_on(run_in_over(
            dir,
            Ok(root.to_path_buf()),
            &argv(&["serve", addr]),
            fired(),
        ))
}

/// **The served banner names every fact of the start, in the state it is
/// actually in.** The banner is stdout and nothing else, so its arms are
/// driven in a child whose environment names the log directory, the level,
/// the front end and read-only mode, and read back off the child's stdout.
///
/// Covers `announce_log`'s written arm (server.rs:19132-19139) on the first
/// serve and its NOT WRITABLE arm (19141) on the second, `announce_front_end`
/// (18166-18172), the READ-ONLY line (19311-19317), the grounded recovery line
/// (19414), the port-zero skip (19388-19389) and, on a real port with
/// `BRUTEX_NO_OPEN=1`, the not-opened line (19391-19393) through
/// `open_in_browser`'s suppressed arm (18949-18951). Kills
/// `announce_log -> ()`, `announce_front_end -> ()`, `19388 == with !=`,
/// `19367 delete !`, `19248 delete field dir` (the log must land in the
/// named directory), `19310 delete !` (asked to fly while read-only, the
/// banner still says PAUSED), `read_only`/`read_only_from -> false`,
/// `WriteGate::of_process -> Default`, `open_in_browser -> Ok(())` and
/// `served_log_level -> (Default, "xyzzy")`.
#[test]
fn the_served_banner_names_every_fact_of_the_start() {
    if in_child() {
        end_harness_line();
        banner_child();
        return;
    }
    let dir = agreeing("clouda-banner");
    let root = fresh_dir("banner-store");
    let logs = fresh_dir("banner-logs");
    let _front = super::tests::front("clouda-banner");
    let web = crate::scratch::path("front-clouda-banner");
    let (stdout, _stderr) = rerun_child(
        "the_served_banner_names_every_fact_of_the_start",
        &[
            (CHILD_MASTERS, dir.as_os_str()),
            (CHILD_STORE, root.as_os_str()),
            (LOG_DIR_ENV, logs.as_os_str()),
            (LOG_LEVEL_ENV, std::ffi::OsStr::new(BANNER_LEVEL)),
            (READ_ONLY_ENV, std::ffi::OsStr::new("1")),
            // ASKED TO FLY, AND GROUNDED BY READ-ONLY: the banner must say
            // PAUSED, not the flight the operator asked for (kills `delete !`
            // at server.rs:19310). See `rerun_child` for why this is safe.
            (
                autopilot::AUTOPILOT_ENV,
                std::ffi::OsStr::new(autopilot::AUTOPILOT_RUN),
            ),
            (assets::WEB_ENV, web.as_os_str()),
        ],
    );
    let port: u16 = stdout
        .lines()
        .find_map(|line| line.strip_prefix("CLOUDA-PORT="))
        .expect("the child named its port")
        .parse()
        .expect("a port");
    let canonical = std::fs::canonicalize(&root).expect("the root resolves");
    for line in banner_lines(&dir, &canonical, &logs.join("events.ndjson"), port) {
        assert!(has_line(&stdout, &line), "missing {line:?} in:\n{stdout}");
    }
    let web_line = format!("  web:     {} (", web.join("build").display());
    assert_eq!(
        stdout
            .lines()
            .filter(|line| line.starts_with(&web_line) && line.ends_with(')'))
            .count(),
        2,
        "{stdout}"
    );
    assert_eq!(
        stdout
            .lines()
            .filter(|line| line.starts_with("  level:"))
            .count(),
        1,
        "the level is said with the log it applies to: {stdout}"
    );
    assert!(!stdout.contains("FIRST EVENT NOT WRITTEN"), "{stdout}");
    assert!(!stdout.contains("NO WRITES:"), "{stdout}");
    assert!(!stdout.contains("  autopilot: starting in"), "{stdout}");
    assert_listening_events_name_the_front_end(&logs, &web);
    for path in [&root, &logs, &web] {
        let _ = std::fs::remove_dir_all(path);
    }
}

/// The level the banner child is given: a floor, an override, and a word
/// that is neither.
const BANNER_LEVEL: &str = "debug,pull=trace,nonsense";

/// The banner test's child half: two serves (port zero, then a real port),
/// the suppressed browser, and the two environment readers.
fn banner_child() {
    let dir = handed(CHILD_MASTERS);
    let root = handed(CHILD_STORE);
    let logs = handed(LOG_DIR_ENV);
    assert_eq!(serve_once(&dir, &root, "127.0.0.1:0"), OK);
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|probe| probe.local_addr())
        .expect("a free port")
        .port();
    println!("CLOUDA-PORT={port}");
    assert_eq!(serve_once(&dir, &root, &format!("127.0.0.1:{port}")), OK);
    assert_eq!(
        open_in_browser("http://127.0.0.1:9/"),
        Err(format!("{NO_OPEN_ENV} is set"))
    );
    assert_eq!(served_log_dir(&root), Ok(logs));
    assert_eq!(served_log_level().1, log_level_from(Some(BANNER_LEVEL)).1);
}

/// Every whole line the banner child's two serves must print.
fn banner_lines(dir: &Path, canonical: &Path, file: &Path, port: u16) -> Vec<String> {
    vec![
        "brutex api listening on http://127.0.0.1:0/".to_owned(),
        format!("brutex api listening on http://127.0.0.1:{port}/"),
        format!("  build:   {}", build_label()),
        format!("  masters: {}", dir.display()),
        format!("  store:   {}", canonical.display()),
        format!(
            "  log:     {} (rolling, {} files x {} MiB ceiling)",
            file.display(),
            telemetry::DEFAULT_KEEP_FILES,
            telemetry::DEFAULT_MAX_FILE_BYTES / (1024 * 1024)
        ),
        format!("  level:   {}", log_level_from(Some(BANNER_LEVEL)).1),
        format!(
            "  log:     NOT WRITABLE — a telemetry sink is already installed, writing {}",
            file.display()
        ),
        "  universe: ok".to_owned(),
        format!(
            "  READ-ONLY: every write request is refused, the autopilot is grounded and no \
             recovery is resumed ({READ_ONLY_ENV}=1); only this server's own lock, log, \
             invocation journal and pull-journal read index are written"
        ),
        format!(
            "  autopilot: PAUSED — nothing will be asked of any vendor. Set {}={} to fly on \
             start, or press Resume at http://127.0.0.1:0/autopilot",
            autopilot::AUTOPILOT_ENV,
            autopilot::AUTOPILOT_RUN
        ),
        "  opening: skipped — port 0 addresses nothing".to_owned(),
        format!("  opening: NOT OPENED ({NO_OPEN_ENV} is set) — go to http://127.0.0.1:{port}/"),
        "  recovery not resumed: read-only (BRUTEX_READ_ONLY=1); no plan is in flight".to_owned(),
    ]
}

/// THE FIRST EVENT OF BOTH SERVES IS IN THE NAMED DIRECTORY, carrying the
/// front end's state and the grounded autopilot.
fn assert_listening_events_name_the_front_end(logs: &Path, web: &Path) {
    let tail = telemetry::tail(
        logs,
        telemetry::DEFAULT_KEEP_FILES,
        &telemetry::Query::last(telemetry::MAX_LIMIT).from_target("api.serve"),
    );
    let listening: Vec<&telemetry::Record> = tail
        .records
        .iter()
        .filter(|record| record.message == "listening")
        .collect();
    assert_eq!(listening.len(), 2, "{:?}", tail.records);
    let front = assets::Assets::new(web);
    for record in listening {
        assert_eq!(
            record
                .field("web_state")
                .and_then(telemetry::OwnedValue::as_str),
            Some(front.build().word())
        );
        assert_eq!(
            record
                .field("web_built")
                .and_then(telemetry::OwnedValue::as_bool),
            Some(front.built())
        );
        assert_eq!(
            record
                .field("autopilot_flies")
                .and_then(telemetry::OwnedValue::as_bool),
            Some(false)
        );
    }
}

/// **A log directory or a front end the environment names as an empty
/// string is a refusal, and nothing is served.** Both are only reachable
/// through the environment. Covers `run_in_over`'s log-directory refusal
/// (server.rs:19237-19239) and front-end refusal (19266-19269); each case's
/// child asserts `FAILED` and that the store lock went with the refusal.
#[test]
fn an_empty_log_or_web_folder_is_refused_before_anything_is_served() {
    if in_child() {
        end_harness_line();
        let dir = handed(CHILD_MASTERS);
        let root = handed(CHILD_STORE);
        assert_eq!(serve_once(&dir, &root, "127.0.0.1:0"), FAILED);
        let again = take_serve_lock(&root, lock_addr()).expect("the lock went with the refusal");
        assert!(again.held.is_some());
        return;
    }
    let dir = agreeing("clouda-refusals");
    let root = fresh_dir("refusals-store");
    let logs = fresh_dir("refusals-logs");
    let empty = std::ffi::OsStr::new("");

    let (stdout, stderr) = rerun_child(
        "an_empty_log_or_web_folder_is_refused_before_anything_is_served",
        &[
            (CHILD_MASTERS, dir.as_os_str()),
            (CHILD_STORE, root.as_os_str()),
            (LOG_DIR_ENV, empty),
        ],
    );
    assert!(
        has_line(
            &stderr,
            &format!(
                "REFUSED — {LOG_DIR_ENV} is set but empty. An empty folder is the working \
                 directory, which is never what an operator names; unset it to take the \
                 default or give a folder. No request was served."
            )
        ),
        "{stderr}"
    );
    assert!(!stdout.contains("brutex api listening"), "{stdout}");

    let (stdout, stderr) = rerun_child(
        "an_empty_log_or_web_folder_is_refused_before_anything_is_served",
        &[
            (CHILD_MASTERS, dir.as_os_str()),
            (CHILD_STORE, root.as_os_str()),
            (LOG_DIR_ENV, logs.as_os_str()),
            (assets::WEB_ENV, empty),
        ],
    );
    assert!(
        has_line(
            &stderr,
            &format!(
                "REFUSED — {} is set but empty. An empty folder is the working directory, \
                 which is never what an operator names; unset it to take the default or \
                 give a folder. No request was served.",
                assets::WEB_ENV
            )
        ),
        "{stderr}"
    );
    assert!(
        has_line(&stdout, "brutex api listening on http://127.0.0.1:0/"),
        "the front end is read after the banner opens: {stdout}"
    );
    assert!(
        !stdout.lines().any(|line| line.starts_with("  web:")),
        "{stdout}"
    );
    for path in [&root, &logs] {
        let _ = std::fs::remove_dir_all(path);
    }
}

/// **A build with no commit stamp serves no write and says so on the
/// banner**, and resumes no recovery, naming why. The gate is armed only by
/// the binary's `main`, so the child arms it the way `main` does. The
/// expectation is computed from the same rule (`cli::unstamped_write_refusal`)
/// over this build's own stamp, and both arms assert. Covers the NO WRITES
/// line (server.rs:19319-19320) and the unstamped grounded recovery line.
#[test]
fn an_unstamped_build_serves_no_write_and_says_so() {
    if in_child() {
        end_harness_line();
        let armed = arm_unstamped_gate();
        println!("CLOUDA-ARMED={}", armed.is_some());
        assert_eq!(
            serve_once(&handed(CHILD_MASTERS), &handed(CHILD_STORE), "127.0.0.1:0"),
            OK
        );
        return;
    }
    let dir = agreeing("clouda-unstamped");
    let root = fresh_dir("unstamped-store");
    let logs = fresh_dir("unstamped-logs");
    let zero = std::ffi::OsStr::new("0");
    let (stdout, _stderr) = rerun_child(
        "an_unstamped_build_serves_no_write_and_says_so",
        &[
            (CHILD_MASTERS, dir.as_os_str()),
            (CHILD_STORE, root.as_os_str()),
            (LOG_DIR_ENV, logs.as_os_str()),
            (READ_ONLY_ENV, zero),
            (cli::ALLOW_UNSTAMPED_VAR, zero),
        ],
    );
    let grounded = "  recovery not resumed: unstamped (this build carries no commit stamp); no plan is in flight";
    if let Some(why) = cli::unstamped_write_refusal(cli::commit_stamp(), Some(zero)) {
        assert!(has_line(&stdout, "CLOUDA-ARMED=true"), "{stdout}");
        assert!(
            has_line(&stdout, &format!("  NO WRITES: {why}")),
            "{stdout}"
        );
        assert!(has_line(&stdout, grounded), "{stdout}");
    } else {
        assert!(has_line(&stdout, "CLOUDA-ARMED=false"), "{stdout}");
        assert!(!stdout.contains("NO WRITES:"), "{stdout}");
        assert!(!stdout.contains("recovery not resumed"), "{stdout}");
    }
    assert!(!stdout.contains("READ-ONLY:"), "{stdout}");
    for path in [&root, &logs] {
        let _ = std::fs::remove_dir_all(path);
    }
}

/// **A poisoned root set is still read, and still shares a held key.** A
/// panic that poisons the process-global set is driven in a child, because
/// while it is poisoned every other test's `ServeLock::drop` would skip its
/// key removal. The child asserts that a held root is shared without a
/// second lock and a fresh root is locked (kills `delete !` at
/// server.rs:18365), then clears the poison and proves both keys released.
/// Covers server.rs:18364-18370.
#[test]
fn a_poisoned_root_set_still_admits_and_still_shares_a_held_key() {
    if in_child() {
        end_harness_line();
        let a = fresh_dir("poison-a");
        let b = fresh_dir("poison-b");
        let first = take_serve_lock(&a, lock_addr()).expect("a free store");
        assert!(first.held.is_some());
        let unwound = std::panic::catch_unwind(|| {
            let _held = serving_roots().lock();
            std::panic::resume_unwind(Box::new("poison the root set"));
        });
        assert!(unwound.is_err());
        assert!(serving_roots().is_poisoned());

        let shared = take_serve_lock(&a, lock_addr()).expect("a poisoned set still admits");
        assert!(
            shared.held.is_none(),
            "the held key is shared, not relocked"
        );
        assert_eq!(shared.root, first.root);
        let fresh = take_serve_lock(&b, lock_addr()).expect("a fresh root under poison");
        assert!(fresh.held.is_some(), "a fresh root takes the file lock");

        serving_roots().clear_poison();
        let keys = [first.root.clone(), fresh.root.clone()];
        drop(shared);
        drop(fresh);
        drop(first);
        let set = serving_roots().lock().expect("cleared");
        assert!(keys.iter().all(|key| !set.contains(key)), "{set:?}");
        drop(set);
        println!("CLOUDA-POISON=checked");
        for path in [&a, &b] {
            let _ = std::fs::remove_dir_all(path);
        }
        return;
    }
    let (stdout, _stderr) = rerun_child(
        "a_poisoned_root_set_still_admits_and_still_shares_a_held_key",
        &[],
    );
    assert!(has_line(&stdout, "CLOUDA-POISON=checked"), "{stdout}");
}

/// **A lock name that cannot be opened refuses the serve, names the file,
/// and gives the in-process key back.** A directory where `serve.lock` should
/// be cannot be opened for writing. Covers the open refusal in
/// `take_serve_lock` (server.rs:18389-18394) and kills `release_root -> ()`:
/// with the key kept, the take after the directory is gone would be handed an
/// in-process pass instead of the file lock.
#[test]
fn a_lock_name_that_cannot_be_opened_refuses_and_gives_the_key_back() {
    let root = fresh_dir("serve-lock-is-a-dir");
    let canonical = std::fs::canonicalize(&root).expect("the root resolves");
    std::fs::create_dir(root.join(SERVE_LOCK)).expect("a directory in the lock's place");
    let why = take_serve_lock(&root, lock_addr()).expect_err("a directory is not a lock file");
    let head = format!(
        "REFUSED: the one-server lock {} could not be opened — ",
        canonical.join(SERVE_LOCK).display()
    );
    assert!(why.starts_with(&head), "{why}");
    std::fs::remove_dir(root.join(SERVE_LOCK)).expect("the obstacle removed");
    let taken = take_serve_lock(&root, lock_addr()).expect("free once the obstacle is gone");
    assert!(
        taken.held.is_some(),
        "the refused take gave its key back, so this one locks the file"
    );
    drop(taken);
    let _ = std::fs::remove_dir_all(&root);
}

/// **A log that cannot be opened does not stop the serve, and the banner says
/// both that and that the first event was not written.** The log directory is
/// named as an existing regular file, so no sink is installed in the child and
/// the first emit has nowhere to go. Covers `announce_log`'s NOT WRITABLE arm
/// from a failed open and the FIRST EVENT NOT WRITTEN line (server.rs:19368);
/// kills `delete !` at 19367 from the other side.
#[test]
fn a_log_that_cannot_open_is_said_and_the_serve_goes_on() {
    if in_child() {
        end_harness_line();
        assert_eq!(
            serve_once(&handed(CHILD_MASTERS), &handed(CHILD_STORE), "127.0.0.1:0"),
            OK
        );
        assert!(telemetry::global().is_none(), "no sink was installed");
        return;
    }
    let dir = agreeing("clouda-log-blocked");
    let root = fresh_dir("log-blocked-store");
    let blocker = fresh_dir("log-blocked").join("a-file");
    std::fs::write(&blocker, b"not a directory").expect("a file where the log dir goes");
    let (stdout, _stderr) = rerun_child(
        "a_log_that_cannot_open_is_said_and_the_serve_goes_on",
        &[
            (CHILD_MASTERS, dir.as_os_str()),
            (CHILD_STORE, root.as_os_str()),
            (LOG_DIR_ENV, blocker.as_os_str()),
        ],
    );
    assert!(
        stdout
            .lines()
            .any(|line| line.starts_with("  log:     NOT WRITABLE — ") && line.len() > 26),
        "{stdout}"
    );
    assert!(
        has_line(&stdout, "  log:     FIRST EVENT NOT WRITTEN — NotInstalled"),
        "{stdout}"
    );
    assert!(
        !stdout.contains("  level:"),
        "no level for a log that is not open"
    );
    assert!(has_line(&stdout, "  universe: ok"), "{stdout}");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(blocker.parent().expect("its directory"));
}

/// **A scrub that cannot be admitted answers the admission refusal, not a
/// report.** Every store-read slot is held by work that waits on this test,
/// so `/verify.json`'s scrub waits out its admission and is refused. Driven in
/// a child, because holding a process-global pool would refuse every other
/// test's store reads for the wait. Covers `verify_json`'s refusal arm
/// (server.rs:1904-1905).
#[test]
fn a_scrub_that_cannot_be_admitted_answers_the_admission_refusal() {
    if in_child() {
        end_harness_line();
        let site = Loaded::new(Site::load(&handed(CHILD_MASTERS), &handed(CHILD_STORE)));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime");
        runtime.block_on(async move {
            let (started, mut running) = tokio::sync::mpsc::unbounded_channel::<()>();
            let mut releases = Vec::new();
            let mut holders = Vec::new();
            for _ in 0..crate::detail::MAX_STORE_READ_CONCURRENT {
                let (release, wait) = std::sync::mpsc::channel::<()>();
                releases.push(release);
                let started = started.clone();
                holders.push(tokio::spawn(crate::detail::run_store_read(move || {
                    let _sent = started.send(());
                    let _released = wait.recv();
                })));
            }
            for _ in 0..crate::detail::MAX_STORE_READ_CONCURRENT {
                running.recv().await.expect("every holder starts");
            }
            let uri: axum::http::Uri = "/verify.json?feed=dhan".parse().expect("a uri");
            let (status, headers, body) =
                verify_json(axum::extract::State(Loaded::clone(&site)), uri).await;
            let (want_status, want_body) = crate::detail::admission_refused(
                "vendor scrub",
                crate::detail::MAX_STORE_READ_CONCURRENT,
                &crate::detail::RunError::Saturated,
            );
            assert_eq!(status, want_status);
            assert_eq!(status, axum::http::StatusCode::TOO_MANY_REQUESTS);
            assert_eq!(body, want_body);
            assert_eq!(
                headers.get(axum::http::header::CONTENT_TYPE),
                Some(&axum::http::HeaderValue::from_static(
                    "application/json; charset=utf-8"
                ))
            );
            for release in releases {
                release.send(()).expect("a holder waits");
            }
            for holder in holders {
                assert!(holder.await.expect("joined").is_ok());
            }
        });
        println!("CLOUDA-SCRUB=refused");
        return;
    }
    let dir = agreeing("clouda-scrub-refused");
    let root = fresh_dir("scrub-refused-store");
    let (stdout, _stderr) = rerun_child(
        "a_scrub_that_cannot_be_admitted_answers_the_admission_refusal",
        &[
            (CHILD_MASTERS, dir.as_os_str()),
            (CHILD_STORE, root.as_os_str()),
        ],
    );
    assert!(has_line(&stdout, "CLOUDA-SCRUB=refused"), "{stdout}");
    let _ = std::fs::remove_dir_all(&root);
}

/// **A browser `Origin` matches only the exact `Host` authority over plain
/// HTTP.** Pins every arm of `origin_matches_host` (server.rs:16869-16879),
/// killing `-> true/false`, `!= with ==` at 16873, and `delete !` and
/// `&& with ||` at 16878.
#[test]
fn an_origin_matches_only_the_exact_host_over_plain_http() {
    for (origin, host, expected) in [
        ("http://localhost:8080", "localhost:8080", true),
        ("http://LOCALHOST:8080", "localhost:8080", true),
        ("http://127.0.0.1:8080", "127.0.0.1:8080", true),
        ("https://localhost:8080", "localhost:8080", false),
        ("localhost:8080", "localhost:8080", false),
        ("http://localhost:8081", "localhost:8080", false),
        ("http://127.0.0.1:8080", "localhost:8080", false),
        ("http://user@localhost:8080", "user@localhost:8080", false),
        ("http://", "", false),
        ("http://not a host", "not a host", false),
    ] {
        assert_eq!(
            origin_matches_host(origin, host),
            expected,
            "{origin} against {host}"
        );
    }
}

/// **The served store root is the configured one, read once at the edge.**
/// Kills `served_store_root -> Ok(Default::default())` (server.rs:19119):
/// an empty root is refused by `take_serve_lock` exactly as an absent one
/// is, so no serve test can tell the two apart.
#[test]
fn the_served_store_root_is_the_configured_one() {
    let served = served_store_root();
    assert_eq!(served, store_dir());
    assert_ne!(served, Ok(PathBuf::new()));
}

/// **The entry point reads its two directories from the environment and
/// nowhere else, and serves through them.** A child names `BRUTEX_MASTERS`,
/// `BRUTEX_STORE` and `HOME` exactly, so each reader's answer is a fixed path
/// rather than this host's home directory, and then runs `run` itself: the
/// report over the named masters, and a serve over the named store. Kills
/// `masters_dir`, `store_dir`, `default_masters_dir` and `served_store_root
/// -> Ok(Default::default())` (server.rs:230, 257, 335, 19119) and drives the
/// real entry point through `run_from` and `run_in` to an OK serve.
#[test]
fn the_entry_point_reads_its_directories_from_the_environment() {
    if in_child() {
        end_harness_line();
        let masters = handed("BRUTEX_MASTERS");
        let store = handed("BRUTEX_STORE");
        let home = handed("HOME");
        assert_eq!(masters_dir(), Ok(masters));
        assert_eq!(store_dir(), Ok(store.clone()));
        assert_eq!(served_store_root(), Ok(store));
        assert_eq!(
            default_masters_dir(),
            Ok(home.join(".brutex").join("masters"))
        );
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime");
        assert_eq!(runtime.block_on(run(&argv(&["report"]), fired())), OK);
        assert_eq!(
            runtime.block_on(run(&argv(&["serve", "127.0.0.1:0"]), fired())),
            OK
        );
        return;
    }
    let dir = agreeing("clouda-entry");
    let root = fresh_dir("entry-store");
    let home = fresh_dir("entry-home");
    let logs = fresh_dir("entry-logs");
    let (stdout, _stderr) = rerun_child(
        "the_entry_point_reads_its_directories_from_the_environment",
        &[
            ("BRUTEX_MASTERS", dir.as_os_str()),
            ("BRUTEX_STORE", root.as_os_str()),
            ("HOME", home.as_os_str()),
            (LOG_DIR_ENV, logs.as_os_str()),
            (READ_ONLY_ENV, std::ffi::OsStr::new("0")),
        ],
    );
    let canonical = std::fs::canonicalize(&root).expect("the root resolves");
    assert!(has_line(&stdout, "ok"), "the report's verdict: {stdout}");
    assert!(
        has_line(&stdout, &format!("  masters: {}", dir.display())),
        "{stdout}"
    );
    assert!(
        has_line(&stdout, &format!("  store:   {}", canonical.display())),
        "{stdout}"
    );
    for path in [&root, &home, &logs] {
        let _ = std::fs::remove_dir_all(path);
    }
}

/// The universe counts every unreadable row and every unrecognised listing
/// class it met, ACROSS vendors (kills `unreadable += …` and `unrecognised
/// += …` turned to `*=`, which leave both at zero).
#[test]
fn the_universe_counts_unreadable_rows_and_unrecognised_classes_across_vendors() {
    let dir = crate::scratch::path("clouda-universe-counts");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(
        dir.join("groww_instruments.csv"),
        "exchange,segment,underlying_symbol,trading_symbol,instrument_type,series,isin,\
         expiry_date,strike_price,groww_symbol\n\
         NSE,CASH,,RELIANCE,EQ\n\
         NSE,CASH,,INFY,EQ\n\
         NSE,CASH,,TCS,EQ,EQX,INE467B01029,,,NSE-TCS\n\
         NSE,CASH,,CHOLAFIN,EQ,EQ,INE121A01024,,,NSE-CHOLAFIN\n",
    )
    .expect("the groww master");
    std::fs::write(
        dir.join("dhan_scrip.csv"),
        format!(
            "{}NSE,E,INE002A01018,EQUITY,RELIANCE,RELIANCE INDUSTRIES,ES,EQX,,,,1333\n\
             NSE,E,INE009A01021,EQUITY,INFY\n",
            super::tests::DHAN_HEAD
        ),
    )
    .expect("the dhan master");
    let read = universe(&dir);
    assert_eq!(
        read.unreadable, 3,
        "two short Groww rows and one short Dhan row"
    );
    assert_eq!(read.unrecognised, 2, "EQX at each vendor");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A dated cash-session audit is asked only of an NSE CASH series with no
/// contract: a BSE cash series and an NSE index are answered `None` at once,
/// before any ISIN is looked up, inside a window that holds audit-era days
/// (kills either `||` of the guard turned to `&&`, which send both on to
/// the ISIN refusal).
#[tokio::test]
async fn only_an_nse_cash_series_with_no_contract_has_a_dated_cash_audit() {
    let dir = fresh_dir("cash-guard-masters");
    let root = fresh_dir("cash-guard-store");
    let site = Site::load(&dir, &root);
    let day = auction_era_open_day();
    let window = pull::session::Window::new(day, day).expect("a one-day window");
    let asked = |exchange: &str, segment: &str| Addressed {
        vendor: brutex_core::vendor::Vendor::Zerodha,
        month: day.year_month().expect("a month"),
        timeframe: store::path::Timeframe::MINUTE_1,
        contract: None,
        exchange: exchange.to_owned(),
        segment: segment.to_owned(),
        symbol: "ZZNOISIN".to_owned(),
    };
    for (exchange, segment) in [("BSE", "CASH"), ("NSE", "INDEX")] {
        let answered = audit_cash_schedule_window(&site, &asked(exchange, segment), window).await;
        assert!(
            matches!(answered, Ok(None)),
            "{exchange} {segment}: {:?}",
            answered.map(|schedule| schedule.is_some())
        );
    }
    let refused = audit_cash_schedule_window(&site, &asked("NSE", "CASH"), window)
        .await
        .expect_err("NSE CASH is audited, and this name has no ISIN");
    assert_eq!(
        refused,
        "ZZNOISIN: no exact current ISIN for dated cash-session audit"
    );
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&root);
}
