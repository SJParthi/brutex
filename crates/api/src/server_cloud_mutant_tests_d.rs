//! Cloud mutation pass of 17 Oct (`cloud/mut-api-b-20261017`) over
//! `server.rs`: lines and branch sides that a coverage run of the whole `api`
//! lib suite on `feat/db-fix` `f9cdb584`, with the 16 Oct pass re-applied,
//! left unexecuted, each driven where a test can reach it.
//!
//! No test here opens a socket to anything but `127.0.0.1`, reads a
//! credential, or contacts a vendor. Every instrument and every day is
//! invented for the fixture.
#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a failed fixture must fail the test by name"
)]

use super::*;
use brutex_core::instrument::{Exchange, Expiry, InstrumentKey, Kind};

/// A fresh, empty directory under this process's scratch prefix.
fn fresh_dir(name: &str) -> PathBuf {
    let dir = crate::scratch::path(&format!("cloudd-{name}"));
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

/// A read over `kept` (one vendor's listings) with nothing else wrong.
fn read_of(vendor: Vendor, kept: Vec<brutex_core::vendor::Listing>) -> Read {
    let merged = crate::merge::merge(&[crate::merge::Source {
        vendor,
        kept,
        declined: Vec::new(),
    }]);
    Read::new(merged, Vec::new(), Vec::new(), 0, 0, 0)
}

/// **A read whose only fault is one decline nobody chose is not clean.**
/// No row is unreadable, no vendor is unavailable and no listing class is
/// unrecognised, so only `non_routine` is left to refuse it. Covers the false
/// side of `Read::is_clean`'s `non_routine == 0` (server.rs:564), which every
/// other unclean read short-circuited before.
#[test]
fn a_non_routine_decline_alone_makes_the_read_unclean() {
    let read = |non_routine| {
        Read::new(
            crate::merge::Merged::default(),
            Vec::new(),
            Vec::new(),
            0,
            0,
            non_routine,
        )
    };
    let clean = read(0);
    assert!(clean.is_clean(), "the premise: nothing else is wrong");
    assert_eq!(clean.status(), "ok");
    let declined = read(1);
    assert!(!declined.is_clean());
    assert_eq!(declined.status(), "DEGRADED");
}

/// **An NSE cash series that names a contract has no dated cash audit.** The
/// guard answers before any ISIN is looked up, so a name with no ISIN is not
/// refused for it. Covers the third disqualifier of
/// `audit_cash_schedule_window`'s guard (server.rs:1520); the other two are
/// `only_an_nse_cash_series_with_no_contract_has_a_dated_cash_audit`'s.
#[tokio::test]
async fn an_nse_cash_contract_has_no_dated_cash_audit() {
    let dir = fresh_dir("cash-contract-masters");
    let root = fresh_dir("cash-contract-store");
    let site = Site::load(&dir, &root);
    let day = auction_era_open_day();
    let window = pull::session::Window::new(day, day).expect("a one-day window");
    let asked = Addressed {
        vendor: Vendor::Zerodha,
        month: day.year_month().expect("a month"),
        timeframe: store::path::Timeframe::MINUTE_1,
        contract: Some(
            brutex_core::instrument::Contract::parse("2024-03-07-2200000-CE").expect("a contract"),
        ),
        exchange: "NSE".to_owned(),
        segment: "CASH".to_owned(),
        symbol: "ZZNOISIN".to_owned(),
    };
    let answered = audit_cash_schedule_window(&site, &asked, window).await;
    assert!(
        matches!(answered, Ok(None)),
        "{:?}",
        answered.map(|schedule| schedule.is_some())
    );
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&root);
}

/// **A run whose store refused a write, with no member failure, is classed
/// as the store's.** Covers the right side of `retry_fact`'s `&&`
/// (server.rs:4770): the store fact alone is enough to class a run.
#[test]
fn a_run_the_store_refused_with_no_member_failure_is_classed_store() {
    use crate::pullrun::RetryClass;
    let mut run = BrokerRun::default();
    assert_eq!(run.retry_fact(), None, "the premise: nothing to class");
    run.store_unavailable = Some("the store will not take a write".to_owned());
    let (label, said) = run.retry_fact().expect("a store fault is classed");
    assert_eq!(label, crate::pullrun::RETRY_CLASS_FACT);
    assert!(
        said.starts_with(&format!("{} — 0 failure(s)", RetryClass::Store.word())),
        "{said}"
    );
    let page = stored_html("Spot pull", "FAILED", "r", &[(label, said)]);
    assert_eq!(RetryClass::of_receipt(&page), Some(RetryClass::Store));
}

/// **A contract key is not a spot instrument, whatever id it carries.** The
/// key resolves to a vendor id, so the refusal is the kind's and nothing
/// else's. Covers `spot_vendor_identity_for`'s "not a spot instrument"
/// refusal (server.rs:5988-5989).
#[test]
fn a_contract_key_is_not_a_spot_instrument() {
    let mut key = InstrumentKey::cash(Exchange::Nse, "TCS").expect("a key");
    key.kind = Kind::Future {
        expiry: Expiry::new(2025, 5, 29).expect("an expiry"),
    };
    let read = read_of(
        Vendor::Dhan,
        vec![brutex_core::vendor::Listing {
            vendor_id: brutex_core::vendor::VendorId::new("4242").expect("an id"),
            key,
            isin: None,
            unsuffixed: None,
        }],
    );
    assert!(
        read.merged.by_key.get(&key).is_some(),
        "the premise: the key is in the universe"
    );
    let why = spot_vendor_identity_for(&read, &key, Vendor::Dhan, ingest::CashIdentity::Isin)
        .expect_err("a future is not spot");
    assert_eq!(why, "TCS: not a spot instrument");
}

/// One NSE cash minute window of `instrument`, as `broker_window` lands it.
fn landed_cash(instrument: &str) -> BrokerWindow {
    let spec = match pull::vendor::Feed::Dhan.descriptor().transport {
        pull::vendor::Transport::Http(spec) => Some(spec),
        pull::vendor::Transport::LocalArchive(_) => None,
    }
    .expect("Dhan is an HTTP feed");
    let day = auction_era_open_day();
    BrokerWindow {
        listing: pull::vendor::Listing::Equity,
        contract: None,
        unfetched: None,
        before_first_bar: None,
        instrument: instrument.to_owned(),
        origin: "test only".to_owned(),
        spec,
        exchange: "NSE",
        segment: "CASH",
        store_vendor: Vendor::Dhan,
        window: pull::session::Window::new(day, day).expect("a one-day window"),
        granularity: pull::vendor::Granularity::Minute1,
        bodies: Vec::new(),
    }
}

/// **A cash schedule is refused for any source that is not the exact NSE
/// cash identity of the instrument asked**, before any body is read: another
/// exchange, another segment, a contract, or another name. A daily window
/// and an index listing owe none at all. Covers each disqualifier of
/// `prepare_cash_schedule`'s identity guard (server.rs:5644-5647).
#[tokio::test]
async fn a_cash_schedule_is_refused_for_any_source_but_the_exact_nse_cash_identity() {
    let root = fresh_dir("cash-schedule-identity");
    let site = Site::load(&root, &root);
    let key = InstrumentKey::cash(Exchange::Nse, "TCS").expect("a key");
    let refused = "cash schedule requires the exact NSE cash source identity";
    let mut bse = landed_cash("TCS");
    bse.exchange = "BSE";
    let mut index = landed_cash("TCS");
    index.segment = "INDEX";
    let mut contract = landed_cash("TCS");
    contract.contract = brutex_core::instrument::Contract::of(Kind::Future {
        expiry: Expiry::new(2026, 8, 27).expect("an expiry"),
    });
    assert!(contract.contract.is_some(), "the premise: a contract");
    for (why, landed) in [
        ("another exchange", bse),
        ("another segment", index),
        ("a contract", contract),
        ("another name", landed_cash("INFY")),
    ] {
        let mut dated = std::collections::HashMap::new();
        let answered = prepare_cash_schedule(&landed, &[], &key, &site, &mut dated).await;
        assert_eq!(
            answered.map(|schedule| schedule.is_some()),
            Err(refused.to_owned()),
            "{why}"
        );
    }
    let mut daily = landed_cash("TCS");
    daily.granularity = pull::vendor::Granularity::Day1;
    let mut listed_index = landed_cash("TCS");
    listed_index.listing = pull::vendor::Listing::Index;
    for (why, landed) in [
        ("a daily window", daily),
        ("an index listing", listed_index),
    ] {
        let mut dated = std::collections::HashMap::new();
        let answered = prepare_cash_schedule(&landed, &[], &key, &site, &mut dated).await;
        assert_eq!(
            answered.map(|schedule| schedule.is_some()),
            Ok(false),
            "{why}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// The address every lock stamp in this file names.
fn lock_addr() -> SocketAddr {
    "127.0.0.1:0".parse().expect("an address")
}

/// **A serve kept to the process's end keeps its store held, with no log to
/// close.** `keep_to_exit` forgets the lock instead of releasing it, so the
/// root stays in this process's serving set and the next take in this
/// process is handed the in-process pass, not the file lock. Covers the
/// no-log side of `keep_to_exit`'s `if let Some(log)` (server.rs:17618).
#[test]
fn a_serve_kept_to_exit_keeps_its_store_held_with_no_log_to_close() {
    let root = fresh_dir("kept-to-exit");
    let lock = take_serve_lock(&root, lock_addr()).expect("a free store");
    assert!(lock.held.is_some(), "the premise: the file lock is held");
    let port = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
    let held = Held {
        log: None,
        lock: Some(lock),
        port: Some(port),
    };
    assert!(held.holds());
    held.keep_to_exit();
    let again = take_serve_lock(&root, lock_addr()).expect("this process may serve twice");
    assert!(
        again.held.is_none(),
        "the store is still held by the kept lock: an in-process pass"
    );
}

/// **A lock whose stamp and clear both fail refuses the serve, naming both
/// errors.** `/dev/full` takes an advisory lock but refuses every write
/// (`ENOSPC`) and every truncation (`EINVAL`, it is a character device), so
/// the lock would be held while possibly naming a previous holder. Covers
/// the `?` of `stamp_serve_lock` (server.rs:18787).
#[cfg(target_os = "linux")]
#[test]
fn a_lock_that_can_be_neither_stamped_nor_emptied_refuses_the_serve() {
    let root = fresh_dir("lock-dev-full");
    let canonical = std::fs::canonicalize(&root).expect("the root resolves");
    std::os::unix::fs::symlink("/dev/full", root.join(SERVE_LOCK))
        .expect("the lock name points at a device that takes nothing");
    let why = take_serve_lock(&root, lock_addr()).expect_err("unstampable and unclearable");
    let head = format!(
        "REFUSED: the one-server lock {} was taken, but this instance's address and pid \
         could not be written into it — ",
        canonical.join(SERVE_LOCK).display()
    );
    assert!(why.starts_with(&head), "{why}");
    assert!(
        why.contains("and the file could not be emptied either — "),
        "{why}"
    );
    // AND THE KEY WAS GIVEN BACK: once the obstacle is gone the file lock is
    // taken, not an in-process pass.
    std::fs::remove_file(root.join(SERVE_LOCK)).expect("the obstacle removed");
    let taken = take_serve_lock(&root, lock_addr()).expect("free once the obstacle is gone");
    assert!(taken.held.is_some());
    drop(taken);
    let _ = std::fs::remove_dir_all(&root);
}

/// **An override with no subsystem name is ignored by name, and the rest of
/// the setting still applies.** `=debug` names a level and no target, so it
/// is neither the floor nor an override. Covers the empty-target side of
/// `log_level_from`'s `!target.is_empty()` (server.rs:19214).
#[test]
fn an_override_with_no_subsystem_is_ignored_by_name() {
    let (config, note) = log_level_from(Some("=debug,pull=trace"));
    assert_eq!(config.target_levels.len(), 1, "only pull=trace applied");
    assert!(note.contains("; per subsystem: pull=trace"), "{note}");
    assert!(
        note.contains("; IGNORED as unreadable: =debug — a level is one of "),
        "{note}"
    );
    assert!(
        note.starts_with("floor info (default)"),
        "no bare level was named: {note}"
    );
}

/// **An empty vectored write through the head deadline moves nothing and
/// re-arms nothing; a real one re-arms.** Covers the `n > 0` guard of
/// `poll_write_vectored` (server.rs:17416), the twin of
/// `a_direct_write_through_the_head_deadline_rearms_only_when_bytes_moved`.
#[tokio::test]
async fn a_vectored_write_through_the_head_deadline_rearms_only_when_bytes_moved() {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let mut client = tokio::net::TcpStream::connect(addr).await.expect("connect");
    let (server, _) = listener.accept().await.expect("accept");
    let slots = std::sync::Arc::new(Slots {
        live: std::sync::atomic::AtomicUsize::new(1),
        cap: 1,
        freed: tokio::sync::Notify::new(),
    });
    let slot = Slot(std::sync::Arc::clone(&slots));
    let mut wrapped = HeadDeadline::new(server, slot, std::time::Duration::from_secs(30));

    wrapped.state = HeadState::Delivered;
    let none = wrapped
        .write_vectored(&[std::io::IoSlice::new(b"")])
        .await
        .expect("an empty write");
    assert_eq!(none, 0);
    assert!(matches!(wrapped.state, HeadState::Delivered));

    let wrote = wrapped
        .write_vectored(&[
            std::io::IoSlice::new(b"HTTP/1.1 "),
            std::io::IoSlice::new(b"200"),
        ])
        .await
        .expect("a write");
    assert!(wrote > 0, "bytes moved");
    assert!(matches!(
        wrapped.state,
        HeadState::Awaiting {
            progress: 0,
            partial: false,
            ..
        }
    ));
    let mut got = vec![0_u8; wrote];
    client
        .read_exact(&mut got)
        .await
        .expect("the peer reads it");
    assert_eq!(&got, &b"HTTP/1.1 200"[..wrote]);
    drop(wrapped);
    assert_eq!(slots.live.load(std::sync::atomic::Ordering::Acquire), 0);
}

/// **A publishable pass says so with no reason, bounds its failures at eight
/// and lists every unlinked index apart, comma-separated.** Covers
/// `universe_snapshot_json`'s `Ok` verdict (server.rs:34893) and both lists'
/// separators (34913, 34927), which a one-element list never wrote.
#[test]
fn a_publishable_pass_writes_no_reason_and_separates_every_list() {
    let failure = |n: usize| pull::resolve::IndexFailure {
        at: format!("listing-{n}"),
        why: "503".to_owned(),
    };
    let snap = pull::resolve::Snapshot {
        day: Day::new(2026, 8, 4).expect("a day"),
        feed: Vendor::Groww.as_str().to_owned(),
        key: pull::universe::JoinKey::Isin,
        resolutions: Vec::new(),
        failures: (0..9).map(failure).collect(),
        unlinked: vec![failure(100), failure(101)],
    };
    let json = universe_snapshot_json(&snap, &Ok(()));
    let doc: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(doc["publishable"], true);
    assert!(doc.get("why").is_none(), "no reason for a publishable pass");
    assert_eq!(doc["failed"], 9, "the count is whole");
    let shown = doc["failures"].as_array().expect("a list");
    assert_eq!(shown.len(), 8, "the list is bounded at eight");
    assert_eq!(shown[7]["at"], "listing-7");
    assert_eq!(
        doc["unlinked_at"],
        serde_json::json!(["listing-100", "listing-101"])
    );
}

/// Set in a child of this binary: the test runs its child half.
const CHILD: &str = "BRUTEX_CLOUDD_CHILD";

/// **A released serve lock gives its key back even through a poisoned
/// set, so the next serve of that root holds the FILE lock again.**
/// `take_serve_lock` inserts through a poisoned set; `ServeLock::drop` and
/// `release_root` skipped the removal on poison, so after a panic poisoned
/// the set, a root once served stayed in it: the next take was handed the
/// in-process pass with no file lock held, and a second process could take
/// the store beside it. Driven in a child, because the poison is the whole
/// process's. Fails on the code before this pass's fix: both second takes
/// answered `held: None`.
#[test]
fn a_released_lock_gives_its_key_back_through_a_poisoned_set() {
    if std::env::var_os(CHILD).is_some() {
        let a = fresh_dir("poison-release-a");
        let b = fresh_dir("poison-release-b");
        let unwound = std::panic::catch_unwind(|| {
            let _held = serving_roots().lock();
            std::panic::resume_unwind(Box::new("poison the root set"));
        });
        assert!(unwound.is_err());
        assert!(serving_roots().is_poisoned(), "the premise");

        // A DROP UNDER POISON.
        let first = take_serve_lock(&a, lock_addr()).expect("a free store");
        assert!(first.held.is_some(), "a fresh root takes the file lock");
        drop(first);
        let again = take_serve_lock(&a, lock_addr()).expect("free again");
        assert!(
            again.held.is_some(),
            "the dropped lock gave its key back: the file lock, not an in-process pass"
        );
        drop(again);

        // A FAILED TAKE UNDER POISON (`release_root`).
        std::fs::create_dir(b.join(SERVE_LOCK)).expect("a directory in the lock's place");
        let _refused = take_serve_lock(&b, lock_addr()).expect_err("not a lock file");
        std::fs::remove_dir(b.join(SERVE_LOCK)).expect("the obstacle removed");
        let taken = take_serve_lock(&b, lock_addr()).expect("free once the obstacle is gone");
        assert!(
            taken.held.is_some(),
            "the refused take gave its key back through the poison"
        );
        drop(taken);
        serving_roots().clear_poison();
        println!();
        println!("CLOUDD-POISON-RELEASE=checked");
        for path in [&a, &b] {
            let _ = std::fs::remove_dir_all(path);
        }
        return;
    }
    let stdout = crate::isolated::rerun(
        "server::cloud_mutant_tests_d::a_released_lock_gives_its_key_back_through_a_poisoned_set",
        &[(CHILD, std::ffi::OsStr::new("1"))],
    );
    assert!(
        stdout
            .lines()
            .any(|line| line == "CLOUDD-POISON-RELEASE=checked"),
        "{stdout}"
    );
}

/// One raw request to `addr`, written as bytes, and the whole answer read
/// back as text.
async fn raw_exchange(addr: SocketAddr, request: Vec<u8>) -> String {
    tokio::task::spawn_blocking(move || {
        use std::io::{Read as _, Write as _};
        let mut stream = std::net::TcpStream::connect(addr).expect("connect");
        stream.write_all(&request).expect("write");
        let mut answer = Vec::new();
        stream.read_to_end(&mut answer).expect("read");
        String::from_utf8_lossy(&answer).into_owned()
    })
    .await
    .expect("the client thread")
}

/// **A form body that is not UTF-8 is not judged by its keys; the route
/// reading it answers for it.** The repeated-key refusal reads the body as
/// text, so a body that is not text passes the guard and reaches the
/// handler, which refuses it on its own terms. Covers the `Err` side of
/// `one_value_per_form_field`'s `from_utf8` (server.rs:15900).
#[tokio::test]
async fn a_form_body_that_is_not_utf8_is_left_to_its_route() {
    let dir = super::tests::agreeing("cloudd-form-not-utf8");
    let built = Loaded::new(super::tests::site("cloudd-form-not-utf8", &dir));
    super::tests::served_over("cloudd-form-not-utf8", built, |addr| async move {
        let host = format!("localhost:{}", addr.port());
        let form: &[u8] = b"k=1&k=1&\xff";
        let mut request = format!(
            "POST /pull/run HTTP/1.1\r\nHost: {host}\r\nOrigin: http://{host}\r\n\
             Sec-Fetch-Site: same-origin\r\n\
             Content-Type: application/x-www-form-urlencoded\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n",
            form.len()
        )
        .into_bytes();
        request.extend_from_slice(form);
        let answer = raw_exchange(addr, request).await;
        let body = super::tests::body_of(&answer);
        assert!(answer.starts_with("HTTP/1.1 400"), "{answer}");
        assert!(
            !body.contains("more than once"),
            "a body that is not text is not judged by its keys: {answer}"
        );
        assert!(
            body.contains("valid UTF-8"),
            "the route's own reader refused it: {answer}"
        );
    })
    .await;
}

/// A window over August and September 2026, which is no single month.
fn two_month_window() -> pull::session::Window {
    pull::session::Window::new(
        Day::new(2026, 8, 3).expect("a day"),
        Day::new(2026, 9, 4).expect("a day"),
    )
    .expect("a window")
}

/// **An instrument refused over a window of two months is logged with its
/// window and no month.** A month index keys only on ONE month, so a refusal
/// about a span carries no `month` field (D-3341). Covers the `None` side of
/// `note_instrument_refusal`'s `one_month` (server.rs:8775).
#[test]
fn an_instrument_refused_over_two_months_is_logged_with_no_month() {
    let _shared = crate::emitted::sink();
    let about = crate::autopilot::About::of(None, None, two_month_window());
    assert_eq!(about.one_month(), None, "the premise: a span");
    let from = crate::emitted::mark();
    note_instrument_refusal("ZZSPAN", &about, "dhan", 0, 1, "refused over a span");
    let logged: Vec<telemetry::Record> =
        crate::emitted::landed(from, "pull.spot", "instrument refused")
            .into_iter()
            .filter(|record| crate::emitted::says(record, "instrument", "ZZSPAN"))
            .collect();
    assert_eq!(logged.len(), 1, "{logged:?}");
    assert!(logged[0].field("month").is_none(), "{logged:?}");
    assert!(
        crate::emitted::says(&logged[0], "window", "2026-08-03..=2026-09-04"),
        "{logged:?}"
    );
}

/// **With no log installed, a member that did not land still reaches the
/// autopilot's page, and an instrument refusal is still made.** The emits
/// answer `NotInstalled` and each `debug_assert` allows exactly that: a
/// process with no sink. This test installs none; under nextest it is its
/// own process, so the permissive side of both asserts is the one taken
/// (server.rs:5133 and 8780). Run beside a test that installed the shared
/// sink, the emits are written instead, and every assertion still holds.
#[test]
fn with_no_log_installed_a_failure_still_reaches_the_page() {
    let root = fresh_dir("no-log-failure");
    let site = Site::load(&root, &root);
    let asked = ingest::parse_spot(
        "target=swept&feed=dhan&from=2026-08-03&to=2026-08-05&granularity=1min",
        Day::new(2026, 8, 10).expect("a day"),
    )
    .expect("a request in the past");
    let failure = pull::ingest::Failure {
        instrument: "ZZNOLOG".to_owned(),
        why: "a gap the vendor holds".to_owned(),
        month: None,
        day: Day::new(2026, 8, 4).ok(),
    };
    note_member_failure(&failure, &asked, &site);
    let about = crate::autopilot::About::of(None, None, two_month_window());
    note_instrument_refusal("ZZNOLOG", &about, "dhan", 0, 1, "refused");
    let json = site.autopilot.json();
    assert!(json.contains("ZZNOLOG"), "{json}");
    assert!(json.contains("a gap the vendor holds"), "{json}");
    assert!(json.contains("2026-08-04"), "the failure's own day: {json}");
    let _ = std::fs::remove_dir_all(&root);
}

/// **A name in the universe that the engine does not track is never a spot
/// target, whatever the target.** `ZZSMALL` is an NSE equity outside NIFTY
/// Total Market and not an index, so `catalog::tracked` refuses it before
/// the target is asked. Covers the false side of `spot_targets`'
/// `catalog::tracked(entry.universe)` (server.rs:5513).
#[test]
fn an_untracked_name_is_never_a_spot_target() {
    use super::tests::{DHAN_HEAD, GROWW_HEAD};
    // A well-formed ISIN for an invented company: the check digit is
    // computed by `Isin::new`'s own rule, so the row is a real equity row.
    let small = (0..=9)
        .find_map(|digit| brutex_core::isin::Isin::new(&format!("INE999Z0101{digit}")).ok())
        .expect("one check digit verifies");
    let groww = format!(
        "{GROWW_HEAD}NSE,CASH,,RELIANCE,EQ,EQ,INE002A01018,,,NSE-RELIANCE\n\
         NSE,CASH,,ZZSMALL,EQ,EQ,{},,,NSE-ZZSMALL\n",
        small.as_str()
    );
    let dir = super::tests::masters("cloudd-untracked", Some(&groww), Some(DHAN_HEAD));
    let site = super::tests::site("cloudd-untracked", &dir);
    let read = &site.universe().read;
    let untracked = read
        .merged
        .by_key
        .iter()
        .find(|(key, _)| key.underlying.as_str() == "ZZSMALL")
        .map(|(_, entry)| entry.universe)
        .expect("the premise: ZZSMALL is in the universe");
    assert!(
        !crate::catalog::tracked(untracked),
        "the premise: untracked"
    );
    for target in ["equities", "swept", "total"] {
        let asked = ingest::parse_spot(
            &format!("vendor=groww&target={target}&from=2026-08-03&to=2026-08-05"),
            Day::new(2026, 8, 10).expect("a day"),
        );
        let Ok(asked) = asked else {
            continue;
        };
        let names: Vec<String> = spot_targets(&asked, &site)
            .iter()
            .map(|key| key.underlying.as_str().to_owned())
            .collect();
        assert!(
            !names.iter().any(|name| name == "ZZSMALL"),
            "{target}: {names:?}"
        );
    }
    let equities = ingest::parse_spot(
        "vendor=groww&target=equities&from=2026-08-03&to=2026-08-05",
        Day::new(2026, 8, 10).expect("a day"),
    )
    .expect("the equities target parses");
    let names: Vec<String> = spot_targets(&equities, &site)
        .iter()
        .map(|key| key.underlying.as_str().to_owned())
        .collect();
    assert_eq!(names, ["RELIANCE"], "the tracked equity is still a target");
}
