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

/// The window the session sits inside.
fn request() -> pull::fetch::BarRequest {
    pull::fetch::BarRequest {
        instrument_id: String::new(),
        listing: pull::vendor::Listing::Equity,
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
        columns: pull::csv::Columns::TrueDataIndex,
        request,
        encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
        scale: pull::vendor::PriceScale::Paisa,
        vendor: brutex_core::vendor::Vendor::TrueData,
        exchange: "NSE",
        segment: "INDEX",
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
