//! THE ZERODHA MINUTE PATH, END TO END, ON INVENTED CANDLES (WP-13b).
//!
//! A Zerodha minute answer covers up to 60 days. These drive
//! `pull::ingest::from_window` the way `api::server::land_bodies_observed`
//! does, over a scratch store, and read back the bars, the derived rungs and
//! the ledger `<store>/audit/zerodha.gaps` (D-3335):
//!
//! - the 60-day answer around the 2020-11-14 Muhurat lands every regular
//!   session AND the Muhurat's recorded window, and a minute the vendor did
//!   not send is one `gap` line for its day (DB-012, DB-G33, DB-G37, D-3319);
//! - a disaster-recovery Saturday derives at every rung (DB-013, D-3321);
//! - a share's no-trade minutes fold into partial buckets, counted on the
//!   month's `derive` line (DB-G36, D-3321);
//! - the receipt counts repeated refusals instead of repeating them (DB-G88,
//!   D-3341).
//!
//! Prices are invented and cannot be mistaken for a real index level.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};

use pull::fetch::RawRow;
use pull::session::{Day, IST_OFFSET_SECS, Window};
use store::path::Timeframe;

struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("brutex-ZERODHA-{tag}-{}", std::process::id()));
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

/// An invented candle at `minute` past IST midnight on `day`.
fn candle(day: Day, minute: i64) -> RawRow {
    let base = 1_000_000 + (minute % 97) * 13;
    RawRow {
        timestamp: i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + minute * 60,
        open: base,
        high: base + 50,
        low: base - 40,
        close: base + 10,
        volume: 100 + minute,
        open_interest: None,
    }
}

fn request(listing: pull::vendor::Listing, from: Day, to: Day) -> pull::fetch::BarRequest {
    pull::fetch::BarRequest {
        instrument_id: String::new(),
        listing,
        window: Window::new(from, to).unwrap(),
        granularity: pull::vendor::Granularity::Minute1,
    }
}

fn plan<'a>(request: &'a pull::fetch::BarRequest, segment: &'a str) -> pull::ingest::Plan<'a> {
    pull::ingest::Plan {
        calendar: pull::calendar::Runtime::default(),
        cash_schedule: None,
        columns: pull::csv::Columns::Gdfl,
        request,
        encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
        scale: pull::vendor::PriceScale::Paisa,
        vendor: brutex_core::vendor::Vendor::Zerodha,
        exchange: "NSE",
        segment,
        contract: None,
    }
}

/// Every bar of one month file.
fn month_bars(
    root: &Path,
    segment: &str,
    symbol: &str,
    tf: Timeframe,
    month: (u16, u8),
) -> Vec<store::format::Bar> {
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor: brutex_core::vendor::Vendor::Zerodha,
        exchange: "NSE",
        segment,
        symbol,
        contract: None,
        timeframe: tf,
        month: store::path::YearMonth::new(month.0, month.1).unwrap(),
        file: store::path::FileKind::Bars,
    })
    .unwrap();
    let id = u32::try_from(brutex_core::universe::fnv1a(symbol) & u64::from(u32::MAX)).unwrap();
    let file = store::file::BarFile::open_existing(root, path, id).unwrap();
    (0..file.header().n_valid)
        .map(|i| file.read_record(i).unwrap())
        .collect()
}

/// The minutes past IST midnight of `bars` that fall on `day`.
fn on_day(day: Day, bars: &[store::format::Bar]) -> Vec<i64> {
    let midnight = i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS;
    bars.iter()
        .map(|b| b.ts_micros / 1_000_000 - midnight)
        .filter(|s| (0..86_400).contains(s))
        .map(|s| s / 60)
        .collect()
}

fn ledger(root: &Path) -> String {
    fs::read_to_string(root.join("audit").join("zerodha.gaps")).unwrap_or_default()
}

/// Every minute the calendar of record owes from `from` to `to`, with the
/// Muhurat's window, less 10:00-10:02 on 2020-12-01.
fn sixty_day_answer(from: Day, to: Day, muhurat: Day) -> Vec<RawRow> {
    let short = Day::new(2020, 12, 1).unwrap();
    let mut rows = Vec::new();
    let mut sessions = 0;
    for number in from.days_from_epoch()..=to.days_from_epoch() {
        let day = Day::from_days(number).unwrap();
        let kind = pull::session::with_recorded_window(
            i64::from(number),
            pull::calendar::sourced_kind_of(i64::from(number)),
        );
        let pull::calendar::DayKind::Open(session) = kind else {
            continue;
        };
        for minute in 0..1_440_u16 {
            let skipped = day == short && (600..603).contains(&minute);
            if session.expects(minute) && !skipped {
                rows.push(candle(day, i64::from(minute)));
            }
        }
        sessions += usize::from(day != muhurat);
    }
    assert_eq!(
        sessions, 40,
        "the premise: 40 regular sessions and the Muhurat"
    );
    rows
}

/// **THE 60-DAY ANSWER AROUND A MUHURAT LANDS WHOLE, AND ITS ONE GAP IS ONE
/// LEDGER LINE (DB-012, DB-G33, DB-G37; D-3319, D-3335, D-3344).**
///
/// The request is the one `session::split_window` makes from 2018-01-01 at
/// Zerodha's 60-day cap: 2020-10-17..=2020-12-15, the window of the first of
/// the five holes. The invented answer holds every regular session (40) and
/// the Muhurat's sixty minutes, 18:15-19:14 on Saturday 2020-11-14, less three
/// minutes the "vendor" did not send on 2020-12-01. Before D-3344 the
/// Muhurat refused all of it; under D-3344 it refused that day. Now every
/// minute sent lands, every rung derives, and the one short day is one `gap`
/// line and one receipt failure.
#[test]
fn a_sixty_day_answer_around_a_muhurat_lands_whole_and_its_gap_is_one_ledger_line() {
    let scratch = Scratch::new("MUHURAT-WINDOW");
    let from = Day::new(2020, 10, 17).unwrap();
    let to = Day::new(2020, 12, 15).unwrap();
    let muhurat = Day::new(2020, 11, 14).unwrap();
    let rows = sixty_day_answer(from, to, muhurat);
    let req = request(pull::vendor::Listing::Index, from, to);
    let raw = pull::fetch::RawWindow { rows };
    let done = pull::ingest::from_window(
        &raw,
        "NIFTY",
        "ZERODHA TEST",
        &scratch.0,
        plan(&req, "INDEX"),
    );

    assert_eq!(
        done.bars_committed,
        40 * 375 - 3 + 60,
        "{:?}",
        done.failures
    );
    assert_eq!(
        done.derived_files,
        7 * 3,
        "seven rungs in each of three months"
    );
    let november = month_bars(
        &scratch.0,
        "INDEX",
        "NIFTY",
        Timeframe::MINUTE_1,
        (2020, 11),
    );
    let evening = on_day(muhurat, &november);
    assert_eq!(evening.len(), 60, "the Muhurat's hour is stored");
    assert_eq!((evening[0], evening[59]), (1_095, 1_154));
    let five = month_bars(
        &scratch.0,
        "INDEX",
        "NIFTY",
        Timeframe::MINUTE_5,
        (2020, 11),
    );
    assert_eq!(on_day(muhurat, &five).len(), 12, "and derived");

    // ONE GAP: one receipt failure for the month, one ledger line for the day.
    let gaps: Vec<_> = done
        .failures
        .iter()
        .filter(|f| f.why.starts_with("request minute coverage"))
        .collect();
    assert_eq!(gaps.len(), 1, "{:?}", done.failures);
    assert!(
        gaps[0].why.contains("2020-12-01: 10:00–10:03"),
        "{}",
        gaps[0].why
    );
    let text = ledger(&scratch.0);
    let lines: Vec<&str> = text.lines().filter(|l| l.starts_with("gap ")).collect();
    assert_eq!(lines.len(), 1, "{text}");
    assert!(
        lines[0].starts_with(
            "gap NSE-INDEX-NIFTY 2020-12-01 v=1 rung=1min owed=375 held=372 missing=3 first_missing=10:00 runs=1 longest=3 longest_at=10:00 why=vendor_empty run="
        ),
        "{}",
        lines[0]
    );
    derive_counts_and_reasons(&text, &done);

    // A re-run lands nothing and finds the same; the bars are byte-identical.
    let before = fs::read(
        scratch
            .0
            .join("bars/zerodha/NSE/INDEX/NIFTY/1min/2020-11.bin"),
    )
    .unwrap();
    let again = pull::ingest::from_window(
        &raw,
        "NIFTY",
        "ZERODHA TEST",
        &scratch.0,
        plan(&req, "INDEX"),
    );
    assert_eq!(again.bars_committed, 0);
    assert_eq!(again.failures, done.failures);
    assert_eq!(
        fs::read(
            scratch
                .0
                .join("bars/zerodha/NSE/INDEX/NIFTY/1min/2020-11.bin")
        )
        .unwrap(),
        before
    );
}

/// The derive's withheld buckets are counted on its lines, and each rung's
/// refusals are one failure per reason: the two-minute rung has a short bucket
/// (10:59) and an empty one (10:01), the three-minute rung only the empty
/// 10:00 bucket, every other rung one short bucket.
fn derive_counts_and_reasons(text: &str, done: &pull::ingest::Ingested) {
    assert!(
        text.contains(
            "derive NSE-INDEX-NIFTY 5min 2020-12 v=1 days=11 buckets=824 withheld=1 partial=0 run="
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "derive NSE-INDEX-NIFTY 2min 2020-12 v=1 days=11 buckets=2066 withheld=2 partial=0 run="
        ),
        "{text}"
    );
    for (rung, reasons) in [
        ("2min", 2),
        ("3min", 1),
        ("5min", 1),
        ("10min", 1),
        ("15min", 1),
        ("30min", 1),
        ("60min", 1),
    ] {
        let named = done
            .failures
            .iter()
            .filter(|f| f.why.starts_with(&format!("{rung}: ")))
            .count();
        assert_eq!(named, reasons, "{rung}: {:?}", done.failures);
    }
    assert_eq!(done.failures.len(), 1 + 8, "{:?}", done.failures);
}

/// **A DISASTER-RECOVERY SATURDAY DERIVES AT EVERY RUNG (DB-013, D-3321).**
///
/// BANKNIFTY 2024-03-02 held 105 one-minute bars, 09:15-09:59 and
/// 11:30-12:29, and 0 at 2, 5, 15 and 60 minutes. Here the Friday, the
/// Saturday and the Monday land and every rung holds the Saturday: 21
/// five-minute bars and the three hour bars that overlap its windows.
#[test]
fn a_disaster_recovery_saturday_derives_at_every_rung() {
    let scratch = Scratch::new("DR-SATURDAY");
    let friday = Day::new(2024, 3, 1).unwrap();
    let saturday = Day::new(2024, 3, 2).unwrap();
    let monday = Day::new(2024, 3, 4).unwrap();
    let mut rows: Vec<RawRow> = (555..930).map(|m| candle(friday, m)).collect();
    rows.extend((555..600).chain(690..750).map(|m| candle(saturday, m)));
    rows.extend((555..930).map(|m| candle(monday, m)));
    let req = request(pull::vendor::Listing::Index, friday, monday);
    let raw = pull::fetch::RawWindow { rows };
    let done = pull::ingest::from_window(
        &raw,
        "BANKNIFTY",
        "ZERODHA TEST",
        &scratch.0,
        plan(&req, "INDEX"),
    );
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(done.bars_committed, 375 * 2 + 105);
    assert_eq!(done.derived_files, 7);
    for (tf, expected) in [
        (Timeframe::MINUTE_2, 23 + 31),
        (Timeframe::MINUTE_5, 21),
        (Timeframe::MINUTE_15, 3 + 4),
        (Timeframe::MINUTE_60, 3),
    ] {
        let bars = month_bars(&scratch.0, "INDEX", "BANKNIFTY", tf, (2024, 3));
        assert_eq!(on_day(saturday, &bars).len(), expected, "{}", tf.as_str());
    }
    let text = ledger(&scratch.0);
    assert!(!text.contains("gap "), "nothing missing: {text}");
    assert!(
        text.contains("derive NSE-INDEX-BANKNIFTY 5min 2024-03 v=1 days=3 buckets=171 withheld=0 partial=0 run="),
        "{text}"
    );
}

/// **A SHARE'S NO-TRADE MINUTES FOLD INTO PARTIAL BUCKETS (DB-G36, D-3321).**
///
/// An illiquid share with no candle in six minutes of a session (360ONE
/// 2023-02-08 had six, and GDFL shows no trade in them either). Its 60min
/// rung used to store 3 of 7 buckets; now it stores all 7, four of them
/// partial, and the month's `derive` line says so. The six minutes are one
/// `gap` line for the day, `why=vendor_empty`: the ledger records what the
/// vendor sent, the derive what it means for a share.
#[test]
fn a_shares_no_trade_minutes_fold_into_partial_buckets() {
    let scratch = Scratch::new("SHARE-PARTIAL");
    let day = Day::new(2023, 2, 8).unwrap();
    let silent = [570_i64, 640, 641, 720, 800, 801];
    let rows: Vec<RawRow> = (555..930)
        .filter(|m| !silent.contains(m))
        .map(|m| candle(day, m))
        .collect();
    let req = request(pull::vendor::Listing::Equity, day, day);
    let raw = pull::fetch::RawWindow { rows };
    let done = pull::ingest::from_window(
        &raw,
        "SHAREX",
        "ZERODHA TEST",
        &scratch.0,
        plan(&req, "CASH"),
    );
    assert_eq!(done.bars_committed, 369);
    assert_eq!(done.derived_files, 7);
    let hour = month_bars(
        &scratch.0,
        "CASH",
        "SHAREX",
        Timeframe::MINUTE_60,
        (2023, 2),
    );
    assert_eq!(hour.len(), 7, "every hour bucket folds");
    let text = ledger(&scratch.0);
    assert!(
        text.contains(
            "derive NSE-CASH-SHAREX 60min 2023-02 v=1 days=1 buckets=7 withheld=0 partial=4 run="
        ),
        "{text}"
    );
    assert!(
        text.contains("gap NSE-CASH-SHAREX 2023-02-08 v=1 rung=1min owed=375 held=369 missing=6 first_missing=09:30 runs=4 longest=2 longest_at=10:40 why=vendor_empty run="),
        "{text}"
    );
    // No derive failure: a no-trade minute is not a withheld bucket.
    assert!(
        done.failures
            .iter()
            .all(|f| f.why.starts_with("request minute coverage")),
        "{:?}",
        done.failures
    );
}

/// **A LEDGER THAT CANNOT BE WRITTEN IS A NAMED FAILURE, NEVER SILENCE.**
/// `CLAUDE.md` §4. With a file where `<store>/audit` should be, the bars and
/// the derived rungs still land, and the receipt names both ledgers.
#[test]
fn a_ledger_that_cannot_be_written_is_named_and_the_bars_still_land() {
    let scratch = Scratch::new("LEDGER-BLOCKED");
    fs::write(scratch.0.join("audit"), b"not a directory").unwrap();
    let day = Day::new(2025, 7, 1).unwrap();
    let rows: Vec<RawRow> = (555..930)
        .filter(|m| *m != 600)
        .map(|m| candle(day, m))
        .collect();
    let req = request(pull::vendor::Listing::Index, day, day);
    let raw = pull::fetch::RawWindow { rows };
    let done = pull::ingest::from_window(
        &raw,
        "NIFTY",
        "ZERODHA TEST",
        &scratch.0,
        plan(&req, "INDEX"),
    );
    assert_eq!(done.bars_committed, 374);
    assert_eq!(done.derived_files, 7);
    assert!(
        done.failures.iter().any(|f| f
            .why
            .starts_with("the minute-gap ledger was not written (1 day line(s))")),
        "{:?}",
        done.failures
    );
    assert!(
        done.failures.iter().any(|f| f
            .why
            .starts_with("2025-07: the derive ledger was not written (7 line(s))")),
        "{:?}",
        done.failures
    );
}
