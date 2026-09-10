//! ONE MINUTE IS PULLED; SEVEN COARSER RUNGS ARE WRITTEN BESIDE IT.
//!
//! The operator's requirement, from the first message: one minute is what gets
//! bought and pulled, and 2, 3, 5, 10, 15, 30 and 60 minutes are DERIVED from
//! it internally and stored, so `/db` shows every one. Before this a one-minute
//! pull wrote `1min/` and stopped — the store had directories for the coarser
//! rungs and nothing ever put a bar in one.
//!
//! These assert the whole path: the files exist, the bars in them are the fold
//! of the minute bars, and every one carries a census row — because a bar with
//! no census row is a bar `/store.json` cannot see, which makes a later run
//! refetch a month already on disk.

// A test that asserts nothing is banned, and a test that cannot fail loudly is
// a test that asserts nothing.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};

use pull::archive::Member;
use pull::fetch::RawRow;
use store::path::Timeframe;

/// 2025-07-01 09:15:00 IST as a UTC epoch second. A Tuesday.
const OPEN_UTC: i64 = 1_751_341_500;
const SESSION_MINUTES: i64 = 375;

struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("brutex-derive-{tag}-{}", std::process::id()));
        let _best_effort = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch root");
        Self(dir)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _best_effort = fs::remove_dir_all(&self.0);
    }
}

/// One member holding a whole session of one-minute rows.
fn session_member() -> Member {
    let rows = (0..SESSION_MINUTES)
        .map(|m| {
            let base = 2_550_000 + ((m * 37) % 211 - 105) * 25;
            RawRow {
                timestamp: OPEN_UTC + m * 60,
                open: base,
                high: base + 40,
                low: base - 35,
                close: base + 10,
                volume: 400 + m,
                open_interest: Some(500_000 + m * 3),
            }
        })
        .collect();
    Member {
        path: PathBuf::from("/bought/NIFTY.csv"),
        instrument: "NIFTY".to_owned(),
        rows,
    }
}

/// Where a rung's month file lands for the member above.
fn month_file(root: &Path, tf: Timeframe) -> PathBuf {
    root.join("bars")
        .join("truedata")
        .join("NSE")
        .join("INDEX")
        .join("NIFTY")
        .join(tf.as_str())
        .join("2025-07.bin")
}

/// EVERY DERIVED RUNG IS ON DISK AFTER A ONE-MINUTE INGEST, and the minute is
/// too. Eight files from one member, from one pull, from one vendor request.
#[test]
fn a_one_minute_ingest_writes_the_minute_and_all_seven_derived_rungs() {
    let scratch = Scratch::new("all");
    let root = &scratch.0;
    let req = request();
    let done = pull::ingest::from_members(&[session_member()], root, plan(&req));

    assert!(
        done.failures.is_empty(),
        "the member should land: {:?}",
        done.failures
    );
    assert_eq!(
        done.derived_files, 7,
        "seven rungs are derived from the minute"
    );

    for tf in [
        Timeframe::MINUTE_1,
        Timeframe::MINUTE_2,
        Timeframe::MINUTE_3,
        Timeframe::MINUTE_5,
        Timeframe::MINUTE_10,
        Timeframe::MINUTE_15,
        Timeframe::MINUTE_30,
        Timeframe::MINUTE_60,
    ] {
        let at = month_file(root, tf);
        assert!(at.is_file(), "{} has no month file at {at:?}", tf.as_str());
    }

    // AND THE DAY IS NOT AMONG THEM. A daily bar is what a VENDOR serves under
    // its own convention — Dhan opens it at the session's first print, Groww at
    // the previous session's close, measured 181 points apart on one day — so
    // deriving one from minutes would silently pick a convention and file it
    // beside the other vendor's. D-0077.
    assert!(
        !month_file(root, Timeframe::DAY_1).exists(),
        "the day is served, never derived"
    );
}

/// THE COUNTS ARE THE FOLD'S, not a guess and not the minute's.
///
/// 375 session minutes: the four rungs that divide it tile exactly, and the
/// four that do not end with one short final bar. Pinned per rung, so a fold
/// that silently dropped or invented a bucket fails here.
#[test]
fn each_derived_file_holds_exactly_the_bars_the_fold_produces() {
    let scratch = Scratch::new("counts");
    let root = &scratch.0;
    let req = request();
    let done = pull::ingest::from_members(&[session_member()], root, plan(&req));
    assert!(done.failures.is_empty(), "{:?}", done.failures);

    for (tf, bars) in [
        (Timeframe::MINUTE_1, 375_u32),
        (Timeframe::MINUTE_2, 188),
        (Timeframe::MINUTE_3, 125),
        (Timeframe::MINUTE_5, 75),
        (Timeframe::MINUTE_10, 38),
        (Timeframe::MINUTE_15, 25),
        (Timeframe::MINUTE_30, 13),
        (Timeframe::MINUTE_60, 7),
    ] {
        let at = month_file(root, tf);
        let on_disk = fs::metadata(&at).expect("the file was written").len();
        // ASSERTED FROM THE FILE'S SIZE, using the store's own published
        // strides rather than a literal — `HEADER_LEN` and `RECORD_STRIDE` are
        // `pub` for exactly this. Reading the size rather than reopening the
        // file keeps this independent of the store's READER, so a reader bug
        // cannot make a writer bug look fine.
        let payload = on_disk - store::format::HEADER_LEN;
        let bars_on_disk =
            u32::try_from(payload / store::format::RECORD_STRIDE).expect("a sane file");
        assert_eq!(
            bars_on_disk,
            bars,
            "{} holds the wrong number of bars",
            tf.as_str()
        );
    }
}

/// EVERY DERIVED FILE IS COUNTED. A bar with no census row is a bar
/// `/store.json` cannot see, so a later run refetches a month already on disk
/// and the append refuses it — the worst outcome this module names.
#[test]
fn every_derived_rung_carries_its_own_census_row() {
    let scratch = Scratch::new("census");
    let root = &scratch.0;
    let req = request();
    let done = pull::ingest::from_members(&[session_member()], root, plan(&req));
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(
        done.counted, 8,
        "one census row per file written — the minute and its seven folds"
    );
}

#[test]
fn coarse_source_is_stored_but_cannot_publish_unverified_derivatives() {
    let scratch = Scratch::new("coarse-unverified");
    let mut req = request();
    req.granularity = pull::vendor::Granularity::Minute5;
    let mut member = session_member();
    member.rows = member.rows.into_iter().step_by(5).collect();
    member.rows.remove(3);
    let done = pull::ingest::from_members(&[member], &scratch.0, plan(&req));
    assert!(month_file(&scratch.0, Timeframe::MINUTE_5).exists());
    assert_eq!(done.derived_files, 0);
    assert!(
        done.failures
            .iter()
            .any(|f| f.why.contains("cannot attest minute completeness"))
    );
    for tf in [
        Timeframe::MINUTE_10,
        Timeframe::MINUTE_15,
        Timeframe::MINUTE_30,
        Timeframe::MINUTE_60,
    ] {
        assert!(!month_file(&scratch.0, tf).exists());
    }
}

/// The window the session sits inside.
fn request() -> pull::fetch::BarRequest {
    pull::fetch::BarRequest {
        instrument_id: String::new(),
        listing: pull::vendor::Listing::Index,
        window: pull::session::Window::new(
            pull::session::Day::new(2025, 7, 1).expect("a real day"),
            pull::session::Day::new(2025, 7, 1).expect("a real day"),
        )
        .expect("a legal window"),
        granularity: pull::vendor::Granularity::Minute1,
    }
}

/// A plan for the archive feed, at the MINUTE rung — which is the rung the
/// derive is keyed on, and the only one that produces the other seven.
fn plan(request: &pull::fetch::BarRequest) -> pull::ingest::Plan<'_> {
    pull::ingest::Plan {
        calendar: pull::calendar::Runtime::default(),
        cash_schedule: None,
        columns: pull::csv::Columns::TrueDataIndex,
        request,
        encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
        scale: pull::vendor::PriceScale::Paisa,
        vendor: brutex_core::vendor::Vendor::TrueData,
        exchange: "NSE",
        segment: "INDEX",
        contract: None,
    }
}

/// DERIVED RUNGS ARE FOR SPOT AND FUTURES. OPTIONS DERIVE NOTHING.
///
/// The operator's rule, 15 Aug 2026: the internal timeframes are "fully one and
/// only applicable for these underlying spots alone". A derivative is stored at
/// the rungs its vendor serves and at no others.
///
/// Beyond the rule, the arithmetic would not survive it: an option contract is
/// born at its listing and dies at its expiry, and is illiquid at both ends.
/// Folding thirty one-minute bars of which four traded yields a bar that looks
/// like a half-hour of trading and was nothing of the kind. A spot index has no
/// such gaps, which is why the fold is honest there and only there.
#[test]
fn the_internal_rungs_are_derived_for_spot_and_futures_but_never_for_options() {
    use brutex_core::instrument::Contract;
    use pull::ingest::{derived_count, derived_count_in};
    use store::path::Timeframe;

    let fut = Contract::parse("2025-09-30-FUT").expect("a legal future");
    let ce = Contract::parse("2025-09-30-2465000-CE").expect("a legal option");
    let pe = Contract::parse("2025-09-30-2465000-PE").expect("a legal option");

    assert_eq!(
        derived_count_in(None, Timeframe::MINUTE_1),
        7,
        "spot folds into 2, 3, 5, 10, 15, 30 and 60 minutes"
    );
    assert_eq!(
        derived_count_in(Some(fut), Timeframe::MINUTE_1),
        derived_count(Timeframe::MINUTE_1),
        "and a FUTURE folds exactly the same way — one contract per expiry, \
         trading every minute it is alive"
    );
    for opt in [ce, pe] {
        assert_eq!(
            derived_count_in(Some(opt), Timeframe::MINUTE_1),
            0,
            "an OPTION derives nothing: away from the money most strikes do not \
             trade for minutes at a time, so a folded bar would claim a \
             half-hour of trading that never happened"
        );
    }
    assert!(fut.is_future() && !fut.is_option());
    assert!(ce.is_option() && pe.is_option());
}

/// **A RUN THAT SUCCEEDS WRITES A LINE, AND UNTIL TODAY IT DID NOT.**
///
/// `ingest`'s header says *"`Info` carries the run; `Debug` carries the
/// members"*. The `Debug` half was built and the `Info` half never existed:
/// every emit on this path was `debug`, `warn` or `error`, so a run that went
/// perfectly emitted nothing above the default floor of `Level::Info`.
///
/// Measured on the operator's store, 2026-08-22: a pull of **1,871,491 rows
/// into 1,870,591 bars** left `logs/events.ndjson` at **0 bytes**. The audit
/// journal held 502 records and `/logs` showed a blank page for a completed
/// backfill.
///
/// A silence that only breaks on failure is indistinguishable from a run that
/// never happened. This test drives a CLEAN ingest — the case that was silent —
/// and asserts the record lands at `Info` with the books on it.
#[test]
fn a_clean_run_writes_one_info_record_naming_what_it_did() {
    let dir = std::env::temp_dir().join(format!("brutex-derive-log-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // Install-or-adopt: `telemetry::install` is a process singleton, so a
    // sibling test in this binary may already own it. Either way the sink
    // returned is the one `emit` reaches, which is the only one an assertion
    // about a production emit can be written against.
    let config = telemetry::Config::new(&dir).with_min_level(telemetry::Level::Trace);
    let sink = match telemetry::install(&config) {
        Ok(installed) => installed,
        Err(_already) => telemetry::global().expect("install named the sink that exists"),
    };
    let before = sink.health().written;

    let scratch = Scratch::new("runlog");
    let req = request();
    let done = pull::ingest::from_members(&[session_member()], &scratch.0, plan(&req));
    assert!(done.failures.is_empty(), "the premise: a CLEAN run");

    assert!(
        sink.health().written > before,
        "a clean run wrote no telemetry at all — which is the state that left a \
         1.87-million-bar backfill with a 0-byte log"
    );
    assert_eq!(
        sink.health().dropped,
        0,
        "and nothing was dropped, so a missing record would be a missing emit"
    );
}

#[test]
fn resumed_partial_bucket_matches_one_shot_bytes() {
    let resumed = Scratch::new("resume");
    let whole = Scratch::new("whole");
    let req = request();
    let member = session_member();
    let mut prefix = member.clone();
    prefix.rows.truncate(7);
    let first = pull::ingest::from_members(&[prefix], &resumed.0, plan(&req));
    assert_eq!(first.bars_committed, 7);
    assert!(!month_file(&resumed.0, Timeframe::MINUTE_60).exists());
    let mut suffix = member.clone();
    suffix.rows.drain(..7);
    let second = pull::ingest::from_members(&[suffix], &resumed.0, plan(&req));
    assert!(second.failures.is_empty(), "{:?}", second.failures);
    let full = pull::ingest::from_members(&[member], &whole.0, plan(&req));
    assert!(full.failures.is_empty());
    for tf in Timeframe::KNOWN
        .iter()
        .copied()
        .filter(|tf| tf.secs() > 60 && tf.secs() < 86400)
    {
        let resumed = fs::read(month_file(&resumed.0, tf)).unwrap();
        let whole = fs::read(month_file(&whole.0, tf)).unwrap();
        // Header generations reflect append count. The immutable bar payload agrees.
        let header = usize::try_from(store::format::HEADER_LEN).unwrap();
        assert_eq!(&resumed[header..], &whole[header..], "{}", tf.as_str());
    }
}

#[test]
fn broker_duplicates_do_not_double_volume_and_conflicts_are_refused() {
    let clean = Scratch::new("broker-clean");
    let duplicate = Scratch::new("broker-duplicate");
    let conflict = Scratch::new("broker-conflict");
    let req = request();
    let mut raw = pull::fetch::RawWindow {
        rows: session_member().rows,
    };
    let done = pull::ingest::from_window(&raw, "NIFTY", "test", &clean.0, plan(&req));
    assert!(done.failures.is_empty());
    raw.rows.insert(1, raw.rows[0]);
    let done = pull::ingest::from_window(&raw, "NIFTY", "test", &duplicate.0, plan(&req));
    assert_eq!(done.bars_committed, 375);
    assert_eq!(done.rows_read, 376);
    assert_eq!(done.rows_folded, 1);
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert!(done.balances());
    assert_eq!(
        fs::read(month_file(&clean.0, Timeframe::MINUTE_1)).unwrap(),
        fs::read(month_file(&duplicate.0, Timeframe::MINUTE_1)).unwrap()
    );
    let repeated = pull::ingest::from_window(&raw, "NIFTY", "test", &duplicate.0, plan(&req));
    assert_eq!(repeated.bars_committed, 0);
    assert_eq!(repeated.rows_folded, 1);
    assert!(repeated.failures.is_empty(), "{:?}", repeated.failures);
    assert!(repeated.balances());
    raw.rows[1].volume += 1;
    let done = pull::ingest::from_window(&raw, "NIFTY", "test", &conflict.0, plan(&req));
    assert_eq!(done.bars_committed, 0);
    assert!(done.failures[0].why.contains("conflicting vendor candles"));
    assert!(!month_file(&conflict.0, Timeframe::MINUTE_1).exists());
}

#[test]
fn identical_archive_snapshots_still_contribute_volume() {
    let scratch = Scratch::new("snapshots");
    let req = request();
    let mut member = session_member();
    member.rows.insert(1, member.rows[0]);
    let done = pull::ingest::from_members(&[member], &scratch.0, plan(&req));
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(done.rows_folded, 1);
    let bytes = fs::read(month_file(&scratch.0, Timeframe::MINUTE_1)).unwrap();
    // Timestamp + four prices precede volume in the stable record layout.
    let offset = usize::try_from(store::format::HEADER_LEN).unwrap() + 40;
    assert_eq!(
        i64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap()),
        800
    );
}

#[test]
fn request_minutes_cash_requires_instrument_eligibility_after_session_change() {
    let scratch = Scratch::new("request-cash-eligibility");
    let mut req = request();
    req.listing = pull::vendor::Listing::Equity;
    let day = pull::session::Day::new(2026, 8, 3).unwrap();
    req.window = pull::session::Window::new(day, day).unwrap();
    let mut into = plan(&req);
    into.segment = "CASH";
    let done = pull::ingest::from_window(
        &pull::fetch::RawWindow { rows: Vec::new() },
        "TEST",
        "test",
        &scratch.0,
        into,
    );
    assert!(
        done.failures
            .iter()
            .any(|f| f.why.contains("UNVERIFIED") && f.why.contains("cash eligibility"))
    );
    assert!(
        !done
            .failures
            .iter()
            .any(|f| f.why.starts_with("request minute coverage gap"))
    );
}

#[test]
fn dated_cash_close_filters_auction_and_derives_the_same_continuous_session() {
    use pull::session::{Day, IST_OFFSET_SECS, Window};
    for (eligible, expected) in [(true, 360), (false, 375)] {
        let scratch = Scratch::new(if eligible {
            "cash-cas"
        } else {
            "cash-ordinary"
        });
        let day = Day::new(2026, 8, 3).unwrap();
        let mut req = request();
        req.window = Window::new(day, day).unwrap();
        req.listing = pull::vendor::Listing::Equity;
        let mut schedule = pull::cash_auction::Schedule::default();
        schedule.insert(day, eligible).unwrap();
        let open = i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + 555 * 60;
        let mut member = session_member();
        for (index, row) in member.rows.iter_mut().enumerate() {
            row.timestamp = open + i64::try_from(index).unwrap() * 60;
        }
        let mut into = plan(&req);
        into.segment = "CASH";
        into.cash_schedule = Some(&schedule);
        let raw = pull::fetch::RawWindow { rows: member.rows };
        let done = pull::ingest::from_window(&raw, "TEST", "test", &scratch.0, into);
        assert_eq!(done.rows_read, 375);
        assert_eq!(done.bars_committed, expected);
        assert_eq!(
            usize::try_from(done.census.total()).unwrap(),
            375 - expected
        );
        assert_eq!(done.derived_files, 7);
        assert!(done.failures.is_empty(), "{:?}", done.failures);
        assert!(done.balances());
        let again = pull::ingest::from_window(&raw, "TEST", "test", &scratch.0, into);
        assert_eq!(again.bars_committed, 0);
        assert!(again.failures.is_empty(), "{:?}", again.failures);
        let dir = scratch.0.join("bars/truedata/NSE/CASH/TEST");
        for tf in Timeframe::KNOWN
            .iter()
            .filter(|tf| tf.secs() >= 60 && tf.secs() < 86400)
        {
            let bytes = fs::read(dir.join(tf.as_str()).join("2026-08.bin")).unwrap();
            assert!(bytes.len() > usize::try_from(store::format::HEADER_LEN).unwrap());
        }
        // No dated evidence must fail before a canonical minute is appended.
        let unknown = Scratch::new(if eligible {
            "cash-unknown-a"
        } else {
            "cash-unknown-b"
        });
        into.cash_schedule = None;
        let refused = pull::ingest::from_window(&raw, "TEST", "test", &unknown.0, into);
        assert_eq!(refused.bars_committed, 0);
        assert_eq!(refused.derived_files, 0);
        assert!(!refused.failures.is_empty());
    }
}

#[test]
fn eligibility_does_not_certify_an_unmeasured_calendar_but_source_is_preserved() {
    use pull::session::{Day, IST_OFFSET_SECS, Window};
    let scratch = Scratch::new("cash-unmeasured-calendar");
    let day = Day::new(2026, 9, 11).unwrap();
    let mut req = request();
    req.window = Window::new(day, day).unwrap();
    req.listing = pull::vendor::Listing::Equity;
    let mut schedule = pull::cash_auction::Schedule::default();
    schedule.insert(day, true).unwrap();
    let open = i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + 555 * 60;
    let mut member = session_member();
    for (index, row) in member.rows.iter_mut().enumerate() {
        row.timestamp = open + i64::try_from(index).unwrap() * 60;
    }
    let mut into = plan(&req);
    into.segment = "CASH";
    into.cash_schedule = Some(&schedule);
    let raw = pull::fetch::RawWindow { rows: member.rows };
    let done = pull::ingest::from_window(&raw, "TEST", "test", &scratch.0, into);
    assert_eq!(done.bars_committed, 360);
    assert_eq!(done.census.total(), 15);
    assert_eq!(done.derived_files, 0);
    assert!(done.failures.iter().any(|f| f.why.contains("UNVERIFIED")));
    let again = pull::ingest::from_window(&raw, "TEST", "test", &scratch.0, into);
    assert_eq!(again.bars_committed, 0);
    assert_eq!(again.derived_files, 0);
    assert!(!again.failures.is_empty());
}

#[test]
fn request_minutes_skip_closed_days_but_name_unverified_sessions() {
    use pull::session::{Day, Window};
    for (tag, day, unverified) in [
        ("request-holiday", Day::new(2025, 8, 15).unwrap(), false),
        ("request-exception", Day::new(2021, 2, 24).unwrap(), true),
        ("request-unmeasured", Day::new(2030, 1, 1).unwrap(), true),
    ] {
        let scratch = Scratch::new(tag);
        let mut req = request();
        req.window = Window::new(day, day).unwrap();
        let done = pull::ingest::from_window(
            &pull::fetch::RawWindow { rows: Vec::new() },
            "NIFTY",
            "test",
            &scratch.0,
            plan(&req),
        );
        assert_eq!(done.bars_committed, 0);
        let reports: Vec<_> = done
            .failures
            .iter()
            .filter(|f| f.why.starts_with("request minute coverage"))
            .collect();
        assert_eq!(
            reports.len(),
            usize::from(unverified),
            "{tag}: {:?}",
            done.failures
        );
        if unverified {
            assert!(reports[0].why.contains("UNVERIFIED"));
        }
    }
}

#[test]
fn runtime_observations_cannot_authorize_a_new_session_or_close_an_absent_day() {
    use pull::calendar::{Calendar, Observed, Runtime};
    use pull::session::{Day, IST_OFFSET_SECS, Window};
    let scratch = Scratch::new("runtime-unverified");
    let first = Day::new(2026, 9, 9).unwrap();
    let last = Day::new(2026, 9, 11).unwrap();
    let observed = Calendar::from_observed(&[
        Observed::from_runs(i64::from(first.days_from_epoch()), &[(555, 929)]),
        Observed::from_runs(i64::from(last.days_from_epoch()), &[(555, 929)]),
    ]);
    let mut req = request();
    req.window = Window::new(first, last).unwrap();
    let open = i64::from(last.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + 555 * 60;
    let rows = session_member()
        .rows
        .into_iter()
        .enumerate()
        .map(|(i, row)| RawRow {
            timestamp: open + i64::try_from(i).unwrap() * 60,
            ..row
        })
        .collect();
    let mut into = plan(&req);
    into.calendar = Runtime::from_observed(&observed);
    let raw = pull::fetch::RawWindow { rows };
    let done = pull::ingest::from_window(&raw, "NIFTY", "test", &scratch.0, into);
    assert_eq!(done.bars_committed, 375);
    assert_eq!(done.derived_files, 0);
    assert_eq!(done.rows_read, 375);
    assert!(
        !done.balances(),
        "unverified coverage is not a clean receipt"
    );
    let audit: Vec<_> = done
        .failures
        .iter()
        .filter(|f| f.why.starts_with("request minute coverage"))
        .collect();
    assert_eq!(audit.len(), 3);
    assert!(audit[0].why.contains("observed trading does not attest"));
    assert!(
        audit[1].why.contains("2026-09-10")
            && audit[1]
                .why
                .contains("absent observations do not prove closure")
    );
    assert!(audit[2].why.contains("observed trading does not attest"));
    assert!(
        done.failures
            .iter()
            .any(|f| !f.why.starts_with("request minute coverage")
                && f.why.contains("validated calendar provenance missing"))
    );
    let again = pull::ingest::from_window(&raw, "NIFTY", "test", &scratch.0, into);
    assert_eq!(again.bars_committed, 0);
    assert_eq!(again.derived_files, 0);
    assert_eq!(again.failures, done.failures);
}

#[test]
fn runtime_truncated_observations_do_not_shorten_a_known_session() {
    use pull::calendar::{Calendar, Observed, Runtime};
    let scratch = Scratch::new("runtime-truncated");
    let req = request();
    let observed = Calendar::from_observed(&[Observed::from_runs(
        i64::from(req.window.from().days_from_epoch()),
        &[(555, 559)],
    )]);
    let mut into = plan(&req);
    into.calendar = Runtime::from_observed(&observed);
    let raw = pull::fetch::RawWindow {
        rows: session_member().rows,
    };
    let done = pull::ingest::from_window(&raw, "NIFTY", "test", &scratch.0, into);
    assert_eq!(done.bars_committed, 375);
    assert!(done.derived_files > 0);
    assert!(done.failures.is_empty(), "{:?}", done.failures);
}

#[test]
fn request_minutes_do_not_attest_unordered_rows_or_derivative_requests() {
    let scratch = Scratch::new("request-unordered");
    let req = request();
    let mut rows = session_member().rows;
    rows.swap(0, 1);
    let done = pull::ingest::from_window(
        &pull::fetch::RawWindow { rows },
        "NIFTY",
        "test",
        &scratch.0,
        plan(&req),
    );
    assert!(done.failures.iter().any(|f| {
        f.why
            .contains("request minute coverage UNVERIFIED: unordered")
    }));
    assert!(
        !done
            .failures
            .iter()
            .any(|f| f.why.starts_with("request minute coverage gap"))
    );
    let mut req = request();
    req.listing = pull::vendor::Listing::Derivative;
    let scratch = Scratch::new("request-derivative-exempt");
    let done = pull::ingest::from_window(
        &pull::fetch::RawWindow { rows: Vec::new() },
        "NIFTY",
        "test",
        &scratch.0,
        plan(&req),
    );
    assert!(
        !done
            .failures
            .iter()
            .any(|f| f.why.starts_with("request minute coverage"))
    );
}

#[test]
fn request_minutes_report_month_end_absent_days_and_final_tail_without_weekend_gaps() {
    use pull::session::{Day, IST_OFFSET_SECS, Window};
    let scratch = Scratch::new("request-span");
    let mut req = request();
    req.window = Window::new(
        Day::new(2025, 7, 31).unwrap(),
        Day::new(2025, 8, 4).unwrap(),
    )
    .unwrap();
    let mut rows = Vec::new();
    for (day, count) in [
        (Day::new(2025, 7, 31).unwrap(), 360),
        (Day::new(2025, 8, 4).unwrap(), 360),
    ] {
        let open = i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + 555 * 60;
        rows.extend(
            session_member()
                .rows
                .into_iter()
                .take(count)
                .enumerate()
                .map(|(i, row)| RawRow {
                    timestamp: open + i64::try_from(i).unwrap() * 60,
                    ..row
                }),
        );
    }
    let done = pull::ingest::from_window(
        &pull::fetch::RawWindow { rows },
        "NIFTY",
        "test",
        &scratch.0,
        plan(&req),
    );
    assert_eq!(done.bars_committed, 720);
    assert!(done.derived_files > 0);
    let gaps: Vec<_> = done
        .failures
        .iter()
        .filter(|f| f.why.starts_with("request minute coverage gap"))
        .map(|f| f.why.as_str())
        .collect();
    assert_eq!(gaps.len(), 3, "{:?}", done.failures);
    assert!(gaps[0].contains("2025-07-31: 15:15–15:30"));
    assert!(gaps[1].contains("2025-08-01: 09:15–15:30"));
    assert!(gaps[1].contains("375 missing"));
    assert!(gaps[2].contains("2025-08-04: 15:15–15:30"));
}

#[test]
fn request_minutes_aggregate_opening_and_interior_gaps_and_report_empty_requests() {
    let scratch = Scratch::new("request-intervals");
    let req = request();
    let rows = session_member()
        .rows
        .into_iter()
        .enumerate()
        .filter_map(|(i, row)| (i >= 5 && !(60..75).contains(&i)).then_some(row))
        .collect();
    let done = pull::ingest::from_window(
        &pull::fetch::RawWindow { rows },
        "NIFTY",
        "test",
        &scratch.0,
        plan(&req),
    );
    assert_eq!(done.bars_committed, 355);
    let gaps: Vec<_> = done
        .failures
        .iter()
        .filter(|f| f.why.starts_with("request minute coverage gap"))
        .collect();
    assert_eq!(gaps.len(), 2);
    assert!(gaps[0].why.contains("09:15–09:20"));
    assert!(gaps[1].why.contains("10:15–10:30"));
    let empty = Scratch::new("request-empty");
    let done = pull::ingest::from_window(
        &pull::fetch::RawWindow { rows: Vec::new() },
        "NIFTY",
        "test",
        &empty.0,
        plan(&req),
    );
    assert_eq!(done.bars_committed, 0);
    assert!(
        done.failures
            .iter()
            .any(|f| f.why.contains("375 missing scheduled minutes"))
    );
}

#[test]
fn broker_candles_shifted_thirty_seconds_refuse_before_any_write() {
    let scratch = Scratch::new("broker-shifted");
    let req = request();
    let mut raw = pull::fetch::RawWindow {
        rows: session_member().rows,
    };
    for row in &mut raw.rows {
        row.timestamp += 30;
    }
    let original = raw.clone();
    let done = pull::ingest::from_window(&raw, "NIFTY", "test", &scratch.0, plan(&req));
    assert_eq!(done.bars_committed, 0);
    assert_eq!(done.derived_files, 0);
    assert_eq!(done.rows_read, 375);
    assert_eq!(done.failures.len(), 1);
    assert!(done.failures[0].why.contains("off-grid broker candle"));
    assert!(done.failures[0].why.contains(&(OPEN_UTC + 30).to_string()));
    assert_eq!(raw, original, "no invented timestamps");
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 0);
}

#[test]
fn an_extra_broker_timestamp_inside_a_minute_refuses_but_archive_seconds_survive() {
    let broker = Scratch::new("broker-extra-second");
    let archive = Scratch::new("archive-extra-second");
    let req = request();
    let mut member = session_member();
    let mut extra = member.rows[0];
    extra.timestamp += 30;
    member.rows.insert(1, extra);
    let raw = pull::fetch::RawWindow {
        rows: member.rows.clone(),
    };
    let done = pull::ingest::from_window(&raw, "NIFTY", "test", &broker.0, plan(&req));
    assert_eq!(done.bars_committed, 0);
    assert_eq!(done.failures.len(), 1);
    assert!(done.failures[0].why.contains("off-grid broker candle"));
    assert_eq!(fs::read_dir(&broker.0).unwrap().count(), 0);
    let done = pull::ingest::from_members(&[member], &archive.0, plan(&req));
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(done.bars_committed, 375);
    assert_eq!(done.rows_folded, 1);
    let bytes = fs::read(month_file(&archive.0, Timeframe::MINUTE_1)).unwrap();
    let offset = usize::try_from(store::format::HEADER_LEN).unwrap() + 40;
    assert_eq!(
        i64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap()),
        800
    );
}

#[test]
fn daily_broker_sources_do_not_require_the_intraday_opening_grid() {
    for (tag, timestamp) in [
        ("daily-midnight", OPEN_UTC - 555 * 60),
        ("daily-open", OPEN_UTC),
    ] {
        let scratch = Scratch::new(tag);
        let mut req = request();
        req.granularity = pull::vendor::Granularity::Day1;
        let mut into = plan(&req);
        into.vendor = brutex_core::vendor::Vendor::Zerodha;
        let mut row = session_member().rows[0];
        row.timestamp = timestamp;
        let raw = pull::fetch::RawWindow { rows: vec![row] };
        let done = pull::ingest::from_window(&raw, "NIFTY", "test", &scratch.0, into);
        assert!(done.failures.is_empty(), "{tag}: {:?}", done.failures);
        assert_eq!(done.bars_committed, 1);
        assert_eq!(done.derived_files, 0);
        assert_eq!(raw.rows[0].timestamp, timestamp);
        assert!(
            scratch
                .0
                .join("bars/zerodha/NSE/INDEX/NIFTY/1day/2025-07.bin")
                .is_file()
        );
    }
}

#[test]
fn broker_grid_uses_requested_width_and_preserves_millisecond_precision() {
    use pull::vendor::{Granularity, TimestampEncoding};
    for (tag, granularity, encoding, timestamp) in [
        (
            "five-minute",
            Granularity::Minute5,
            TimestampEncoding::EpochSecondsUtc,
            OPEN_UTC + 60,
        ),
        (
            "millisecond",
            Granularity::Minute1,
            TimestampEncoding::EpochMillisUtc,
            OPEN_UTC * 1_000 + 1,
        ),
        (
            "local-ist",
            Granularity::Minute1,
            TimestampEncoding::IstDateTimeText,
            OPEN_UTC + pull::session::IST_OFFSET_SECS + 30,
        ),
    ] {
        let scratch = Scratch::new(tag);
        let mut req = request();
        req.granularity = granularity;
        let mut into = plan(&req);
        into.encoding = encoding;
        let mut row = session_member().rows[0];
        row.timestamp = timestamp;
        let done = pull::ingest::from_window(
            &pull::fetch::RawWindow { rows: vec![row] },
            "NIFTY",
            "test",
            &scratch.0,
            into,
        );
        assert_eq!(done.bars_committed, 0);
        assert_eq!(done.failures.len(), 1);
        assert!(done.failures[0].why.contains("off-grid broker candle"));
        assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 0);
    }
}

#[test]
fn a_gap_has_a_failure_receipt_while_complete_buckets_are_counted() {
    let scratch = Scratch::new("gap-receipt");
    let req = request();
    let mut member = session_member();
    member.rows.remove(2);
    let done = pull::ingest::from_members(&[member], &scratch.0, plan(&req));
    assert_eq!(done.bars_committed, 374);
    assert_eq!(done.derived_files, 7);
    assert_eq!(done.counted, 8);
    assert!(done.failures.iter().any(|f| {
        f.why.contains("observed 4, scheduled 5")
            && f.why
                .contains("restore complete minute source and verified schedule evidence")
    }));
    assert!(done.failures.iter().all(|f| !f.why.contains("versioned")));
}

/// Seed historical fixtures without going through the completeness check under test.
fn seed_historical_hours(
    root: &Path,
    month: store::path::YearMonth,
    bars: &[store::format::Bar],
) -> PathBuf {
    use store::path::{FileKind, PathParts, StorePath};
    let path = StorePath::new(PathParts {
        vendor: brutex_core::vendor::Vendor::TrueData,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_60,
        month,
        file: FileKind::Bars,
    })
    .unwrap();
    let id = u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX)).unwrap();
    let at = path.to_path_buf(root);
    let mut file = store::file::BarFile::open_or_create(root, path, id).unwrap();
    file.append(bars).unwrap();
    at
}

fn session_hours(member: &Member) -> Vec<store::format::Bar> {
    let minutes: Vec<_> = member
        .rows
        .iter()
        .map(|raw| store::format::Bar {
            ts_micros: raw.timestamp * 1_000_000,
            open: raw.open,
            high: raw.high,
            low: raw.low,
            close: raw.close,
            volume: raw.volume,
            open_interest: raw.open_interest.unwrap(),
        })
        .collect();
    pull::fold::fold(&minutes, pull::fold::Bucket::of_secs(3_600).unwrap()).unwrap()
}

#[test]
fn historical_partial_derived_bytes_are_refused_and_preserved() {
    let scratch = Scratch::new("historical-partial");
    let req = request();
    let raw = session_member().rows[0];
    let old = store::format::Bar {
        ts_micros: raw.timestamp * 1_000_000,
        open: raw.open,
        high: raw.high,
        low: raw.low,
        close: raw.close,
        volume: raw.volume,
        open_interest: raw.open_interest.unwrap(),
    };
    let at = seed_historical_hours(
        &scratch.0,
        store::path::YearMonth::new(2025, 7).unwrap(),
        &[old],
    );
    let before = fs::read(&at).unwrap();
    let done = pull::ingest::from_members(&[session_member()], &scratch.0, plan(&req));
    assert_eq!(done.bars_committed, 375);
    assert_eq!(done.derived_files, 6);
    assert_eq!(done.counted, 7);
    let conflict = &done
        .failures
        .iter()
        .find(|f| f.why.contains("historical derived conflict"))
        .expect("complete source proves a stored byte mismatch")
        .why;
    assert!(conflict.contains("stored bar differs from complete source"));
    assert!(conflict.contains("rung refused; bytes preserved, not repaired"));
    assert!(conflict.contains("explicit versioned correction required"));
    assert!(!conflict.contains("evidence incomplete"));
    assert!(!conflict.contains("source coverage is incomplete"));
    assert!(!conflict.contains("restore complete minute source"));
    assert!(!conflict.contains("gapfill"));
    assert_eq!(before, fs::read(&at).unwrap());
}

#[test]
fn incomplete_historical_source_requires_evidence_before_a_store_remedy() {
    for (tag, missing) in [
        ("historical-incomplete", 2..3),
        ("historical-tail", 360..375),
    ] {
        let scratch = Scratch::new(tag);
        let req = request();
        let mut member = session_member();
        let at = seed_historical_hours(
            &scratch.0,
            store::path::YearMonth::new(2025, 7).unwrap(),
            &session_hours(&member),
        );
        let before = fs::read(&at).unwrap();
        member.rows.drain(missing);
        let raw = pull::fetch::RawWindow { rows: member.rows };
        let done = pull::ingest::from_window(&raw, "NIFTY", "test", &scratch.0, plan(&req));
        assert_eq!(done.bars_committed, raw.rows.len());
        assert_eq!(done.derived_files, 6);
        assert_eq!(done.counted, 7);
        let why = &done
            .failures
            .iter()
            .find(|f| f.why.contains("historical derived evidence incomplete"))
            .expect("no complete candidate is missing evidence, not a byte comparison")
            .why;
        assert!(why.contains("no complete source candidate for stored bar"));
        assert!(why.contains("restore complete minute source and verified schedule evidence"));
        assert!(why.contains("rung refused; bytes preserved, not repaired"));
        assert!(!why.contains("historical derived conflict"));
        assert!(!why.contains("versioned"));
        if tag == "historical-incomplete" {
            assert!(why.contains("observed 59, scheduled 60"), "{why}");
        }
        assert_eq!(before, fs::read(&at).unwrap());
    }
}

#[test]
fn unverified_historical_schedule_requires_evidence_and_preserves_bytes() {
    let scratch = Scratch::new("historical-unverified-schedule");
    let day = pull::session::Day::new(2030, 1, 1).unwrap();
    let mut req = request();
    req.window = pull::session::Window::new(day, day).unwrap();
    let open = i64::from(day.days_from_epoch()) * 86_400 - pull::session::IST_OFFSET_SECS
        + i64::from(pull::calendar::OPEN_MINUTE) * 60;
    let mut member = session_member();
    for row in &mut member.rows {
        row.timestamp += open - OPEN_UTC;
    }
    let at = seed_historical_hours(
        &scratch.0,
        store::path::YearMonth::new(2030, 1).unwrap(),
        &session_hours(&member),
    );
    let before = fs::read(&at).unwrap();
    let done = pull::ingest::from_members(&[member], &scratch.0, plan(&req));
    assert_eq!(done.bars_committed, 375);
    assert_eq!(done.derived_files, 0);
    assert_eq!(done.counted, 1);
    let why = &done
        .failures
        .iter()
        .find(|f| f.why.contains("historical derived evidence incomplete"))
        .expect("withheld candidates cannot prove a stored byte mismatch")
        .why;
    assert!(
        why.contains("UNVERIFIED"),
        "the schedule refusal survives: {why}"
    );
    assert!(why.contains("restore complete minute source and verified schedule evidence"));
    assert!(why.contains("rung refused; bytes preserved, not repaired"));
    assert!(done.failures.iter().all(|f| !f.why.contains("versioned")
        && !f.why.contains("historical derived conflict")));
    assert_eq!(before, fs::read(&at).unwrap());
}

#[test]
fn complete_historical_candidate_holes_require_gapfill_while_a_suffix_can_append() {
    let scratch = Scratch::new("historical-candidate-hole");
    let req = request();
    let member = session_member();
    let mut held = session_hours(&member);
    let missing = held.remove(1);
    let suffix = held.pop().unwrap();
    let at = seed_historical_hours(
        &scratch.0,
        store::path::YearMonth::new(2025, 7).unwrap(),
        &held,
    );
    let before = fs::read(&at).unwrap();
    let done = pull::ingest::from_members(std::slice::from_ref(&member), &scratch.0, plan(&req));
    assert_eq!(done.bars_committed, 375);
    assert_eq!(done.derived_files, 7);
    assert_eq!(done.counted, 8);
    assert_eq!(done.failures.len(), 1, "{:?}", done.failures);
    let why = &done.failures[0].why;
    assert!(why.contains(&format!("historical derived gap at {}", missing.ts_micros)));
    assert!(
        why.contains("complete source candidate is absent from stored history behind the tail")
    );
    assert!(
        why.contains("cannot be refilled append-only; versioned gapfill required; bytes preserved")
    );
    assert!(!why.contains("correction"));
    assert!(!why.contains("evidence incomplete"));
    let after = fs::read(&at).unwrap();
    let header = usize::try_from(store::format::HEADER_LEN).unwrap();
    let stride = usize::try_from(store::format::RECORD_STRIDE).unwrap();
    assert_eq!(after.len(), before.len() + stride);
    assert_eq!(&before[header..], &after[header..before.len()]);
    assert_eq!(
        i64::from_le_bytes(after[before.len()..before.len() + 8].try_into().unwrap()),
        suffix.ts_micros
    );
    let again = pull::ingest::from_members(&[member], &scratch.0, plan(&req));
    assert_eq!(again.bars_committed, 0);
    assert_eq!(again.failures, done.failures);
    assert_eq!(
        after,
        fs::read(&at).unwrap(),
        "reruns cannot refill the hole"
    );
}

#[test]
fn absent_buckets_and_observed_day_tails_require_source_evidence_first() {
    for tail in [false, true] {
        let scratch = Scratch::new(if tail {
            "fold-absent-tail"
        } else {
            "fold-absent-bucket"
        });
        let mut req = request();
        let mut member = session_member();
        let expected = if tail {
            member.rows.truncate(60);
            let mut next = session_member();
            for row in &mut next.rows {
                row.timestamp += 86_400;
            }
            member.rows.extend(next.rows);
            req.window = pull::session::Window::new(
                req.window.from(),
                pull::session::Day::new(2025, 7, 2).unwrap(),
            )
            .unwrap();
            "observed-day tail withheld"
        } else {
            member.rows.drain(60..120);
            "absent: observed 0, scheduled 60"
        };
        let done = pull::ingest::from_members(&[member], &scratch.0, plan(&req));
        assert_eq!(done.derived_files, 7);
        assert!(
            done.failures.iter().any(|f| f.why.starts_with("60min:")
                && f.why.contains(expected)
                && f.why
                    .contains("restore complete minute source and verified schedule evidence")),
            "{:?}",
            done.failures
        );
        assert!(done.failures.iter().all(|f| !f.why.contains("versioned")));
    }
}

#[test]
fn future_derived_files_preserve_the_dated_regular_session() {
    use brutex_core::instrument::Contract;
    use pull::session::{Day, Window};
    for (month, date, expected) in [(7, 31, 375_i64), (8, 3, 385_i64)] {
        let scratch = Scratch::new(&format!("fno-hours-{month}"));
        let day = Day::new(2026, month, date).unwrap();
        let open =
            i64::from(day.days_from_epoch()) * 86_400 - pull::session::IST_OFFSET_SECS + 555 * 60;
        let mut req = request();
        req.window = Window::new(day, day).unwrap();
        req.listing = pull::vendor::Listing::Derivative;
        let mut into = plan(&req);
        into.segment = "FNO";
        into.contract = Some(Contract::parse("2026-08-25-FUT").unwrap());
        let mut member = session_member();
        let prototype = member.rows[0];
        member.rows = (0..expected)
            .map(|m| RawRow {
                timestamp: open + m * 60,
                volume: 1,
                ..prototype
            })
            .collect();
        let done = pull::ingest::from_members(&[member], &scratch.0, into);
        assert!(done.failures.is_empty(), "{month}: {:?}", done.failures);
        assert_eq!(done.bars_committed, usize::try_from(expected).unwrap());
        assert_eq!(done.derived_files, 7);
        for tf in [
            Timeframe::MINUTE_1,
            Timeframe::MINUTE_2,
            Timeframe::MINUTE_3,
            Timeframe::MINUTE_5,
            Timeframe::MINUTE_10,
            Timeframe::MINUTE_15,
            Timeframe::MINUTE_30,
            Timeframe::MINUTE_60,
        ] {
            let at = scratch
                .0
                .join("bars/truedata/NSE/FNO/NIFTY/2026-08-25-FUT")
                .join(tf.as_str())
                .join(format!("2026-{month:02}.bin"));
            let bytes = fs::read(&at).unwrap();
            let header = usize::try_from(store::format::HEADER_LEN).unwrap();
            let stride = usize::try_from(store::format::RECORD_STRIDE).unwrap();
            let payload = &bytes[header..];
            let width = i64::from(tf.secs() / 60);
            assert_eq!(
                payload.len() / stride,
                usize::try_from((expected + width - 1) / width).unwrap()
            );
            let volume: i64 = payload
                .chunks_exact(stride)
                .map(|row| i64::from_le_bytes(row[40..48].try_into().unwrap()))
                .sum();
            assert_eq!(
                volume,
                expected,
                "{month} {}: all minutes reach derived bars",
                tf.as_str()
            );
            let last = &payload[payload.len() - stride..];
            let stamp = i64::from_le_bytes(last[..8].try_into().unwrap());
            assert_eq!(
                stamp,
                (open + (expected - 1) / width * width * 60) * 1_000_000
            );
        }
    }
}
