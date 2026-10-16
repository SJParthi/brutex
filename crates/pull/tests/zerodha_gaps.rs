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

// ---------------------------------------------------------------------------
// R3-18: the repair plan's short sessions and the stored-month gap ledger
// ---------------------------------------------------------------------------

use pull::ingest::gap_backfill;
use pull::ingest::repair_plan::{self, Missing, Owes};

/// Every minute `segment` owes from `from` to `to` by the calendar of record
/// ([`repair_plan::owed`]) that `keep` keeps, as invented candles.
fn owed_answer(segment: &str, from: Day, to: Day, keep: &dyn Fn(Day, i64) -> bool) -> Vec<RawRow> {
    let mut rows = Vec::new();
    for number in from.days_from_epoch()..=to.days_from_epoch() {
        let day = Day::from_days(number).unwrap();
        if let Owes::Windows((windows, count)) = repair_plan::owed(segment, number) {
            for &(open, close) in &windows[..count] {
                for minute in open..close {
                    if keep(day, i64::from(minute)) {
                        rows.push(candle(day, i64::from(minute)));
                    }
                }
            }
        }
    }
    rows
}

/// The first and the last session of record in `from..=to`.
fn sessions_of(segment: &str, from: Day, to: Day) -> (Day, Day) {
    let days: Vec<Day> = (from.days_from_epoch()..=to.days_from_epoch())
        .filter(|&n| matches!(repair_plan::owed(segment, n), Owes::Windows(_)))
        .map(|n| Day::from_days(n).unwrap())
        .collect();
    (days[0], days[days.len() - 1])
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

fn hm(h: i64, m: i64) -> i64 {
    h * 60 + m
}

fn index_parts() -> store::path::PathParts<'static> {
    store::path::PathParts {
        vendor: brutex_core::vendor::Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: store::path::YearMonth::new(2019, 8).unwrap(),
        file: store::path::FileKind::Bars,
    }
}

/// THE DB-A12 SHAPE, on invented candles landed through the real minute path
/// (`from_window`): NIFTY 1min, three held months, every session holding a
/// bar, and seven of them short:
///
/// - 2019-08 (the series' first month): its first session opens at 09:20
///   (`before_first`), and 2019-08-02 lacks 13:59 (`interior`);
/// - 2019-09: its first session lacks 09:15-09:16 (`front`), 2019-09-23 lacks
///   15:15-15:28 (`interior`), and its last session lacks 15:25-15:29
///   (`tail`, in a month before the newest);
/// - 2021-02 (the newest): the outage day 2021-02-24 ends at 10:08
///   (`interior`), and its last session lacks 15:29 (`tail`, the frontier).
///
/// Returns the seven short sessions with their held minutes and fills.
fn db_a12_fixture(root: &Path) -> Vec<(Day, u32, Missing)> {
    let (aug_first, _) = sessions_of("INDEX", day(2019, 8, 1), day(2019, 8, 31));
    let (sep_first, sep_last) = sessions_of("INDEX", day(2019, 9, 1), day(2019, 9, 30));
    let (_, feb_last) = sessions_of("INDEX", day(2021, 2, 1), day(2021, 2, 28));
    let outage = day(2021, 2, 24);
    let holes = move |d: Day, m: i64| {
        (d == aug_first && m < hm(9, 20))
            || (d == day(2019, 8, 2) && m == hm(13, 59))
            || (d == sep_first && m < hm(9, 17))
            || (d == day(2019, 9, 23) && (hm(15, 15)..=hm(15, 28)).contains(&m))
            || (d == sep_last && m >= hm(15, 25))
            || (d == outage && m > hm(10, 8))
            || (d == feb_last && m == hm(15, 29))
    };
    for (from, to) in [
        (day(2019, 8, 1), day(2019, 8, 31)),
        (day(2019, 9, 1), day(2019, 9, 30)),
        (day(2021, 2, 1), day(2021, 2, 28)),
    ] {
        let rows = owed_answer("INDEX", from, to, &|d, m| !holes(d, m));
        let req = request(pull::vendor::Listing::Index, from, to);
        let done = pull::ingest::from_window(
            &pull::fetch::RawWindow { rows },
            "NIFTY",
            "ZERODHA TEST",
            root,
            plan(&req, "INDEX"),
        );
        assert!(done.bars_committed > 0, "{:?}", done.failures);
    }
    let outage_owed = match repair_plan::owed("INDEX", outage.days_from_epoch()) {
        Owes::Windows((w, c)) => w[..c].iter().map(|(a, b)| b - a).sum::<u32>(),
        other => panic!("2021-02-24 is a session of record: {other:?}"),
    };
    assert!(
        outage_owed > 54,
        "the outage day owes more than 09:15-10:08"
    );
    vec![
        (aug_first, 370, Missing::BeforeFirst),
        (day(2019, 8, 2), 374, Missing::Interior),
        (sep_first, 373, Missing::Front),
        (day(2019, 9, 23), 361, Missing::Interior),
        (sep_last, 370, Missing::Tail),
        (outage, 54, Missing::Interior),
        (feb_last, 374, Missing::Tail),
    ]
}

/// **THE REPAIR PLAN LISTS EVERY SESSION SHORT OF ITS OWED MINUTES (R3-18,
/// DB-A12, CMP-19).** Until R3-18 a session counted as held when it held one
/// bar: on this fixture every session holds one, so the plan named no hole in
/// any of the three months and planned no repair — the shape that left 12 of
/// the 14 months behind NIFTY's 213 missing minutes off the documented store
/// run. Now each short session is listed with its owed and held minutes and
/// what fills it, and every month holding an interior or front one is a
/// repair month.
#[test]
fn the_repair_plan_lists_every_session_short_of_its_owed_minutes() {
    let scratch = Scratch::new("PLAN-SHORT");
    let expected = db_a12_fixture(&scratch.0);
    let window = Window::new(day(2019, 8, 1), day(2021, 2, 28)).unwrap();
    let months = repair_plan::plan(&scratch.0, index_parts(), window);
    assert_eq!(months.len(), 19, "2019-08 through 2021-02");
    let held: Vec<&repair_plan::Month> = months.iter().filter(|m| m.bars > 0).collect();
    assert_eq!(held.len(), 3);
    for month in &held {
        assert!(month.refused.is_none(), "{:?}", month.refused);
        assert_eq!(month.held, month.sessions, "every session holds a bar");
        assert!(
            month.missing.is_empty(),
            "{}: {:?}",
            month.month,
            month.missing
        );
        assert!(month.length_unjudged.is_empty());
        assert!(
            month.needs_repair(),
            "{} holds a front or interior short",
            month.month
        );
    }
    let found: Vec<(Day, u32, Missing)> = held
        .iter()
        .flat_map(|m| m.short.iter().map(|s| (s.day, s.held, s.fill)))
        .collect();
    assert_eq!(found, expected);
    for short in held.iter().flat_map(|m| m.short.iter()) {
        assert!(short.owed > short.held && short.missing() == short.owed - short.held);
        if short.day != day(2021, 2, 24) {
            assert_eq!(short.owed, 375, "{} is a regular session", short.day);
        }
    }
    // The newest month's last session lacks 15:29: an ordinary pull's.
    assert!(!Missing::Tail.needs_repair() && !Missing::BeforeFirst.needs_repair());
    // A month the series does not hold is still an ordinary pull's, never a
    // repair's.
    let absent = &months[2];
    assert_eq!(absent.month.to_string(), "2019-10");
    assert!(absent.short.is_empty() && !absent.needs_repair());
    assert!(
        absent
            .missing
            .iter()
            .all(|(_, why)| *why == Missing::Absent)
    );

    // A DERIVED RUNG IS JUDGED BY HOLDING A BAR (its short buckets are the
    // derive ledger's `partial`): the 5min plan lists no short session.
    let five = repair_plan::plan(
        &scratch.0,
        store::path::PathParts {
            timeframe: Timeframe::MINUTE_5,
            ..index_parts()
        },
        window,
    );
    assert!(
        five.iter()
            .all(|m| m.short.is_empty() && m.length_unjudged.is_empty())
    );
}

/// **A SHARE'S CLOSE IS JUDGED UNDER EVERY ELIGIBILITY, AND A WHOLE DAY IS
/// NEVER SHORT.** From 2026-08-03 an eligible share's continuous session ends
/// at 15:15 and an ineligible one's at 15:30 (D-3320); the plan owes the
/// minutes both owe, so a share holding 09:15-15:14 is whole whichever its
/// flag, and one missing 11:00 is short. Written straight to a scratch month,
/// the same reader as the minute path's.
#[test]
fn a_shares_auction_close_is_not_a_short_session_and_a_missing_minute_is() {
    let scratch = Scratch::new("PLAN-CASH");
    let parts = store::path::PathParts {
        segment: "CASH",
        symbol: "INVENTEDSHR",
        month: store::path::YearMonth::new(2026, 8).unwrap(),
        ..index_parts()
    };
    let (from, to) = (day(2026, 8, 3), day(2026, 8, 7));
    let Owes::Windows((windows, 1)) = repair_plan::owed("CASH", from.days_from_epoch()) else {
        panic!("a regular cash session owes one window");
    };
    assert_eq!(windows[0], (555, 915), "09:15 to the eligible 15:15 close");
    assert!(matches!(
        repair_plan::owed("INDEX", from.days_from_epoch()),
        Owes::Windows(([(555, 930), _], 1))
    ));
    let mut bars = Vec::new();
    for n in from.days_from_epoch()..=to.days_from_epoch() {
        let d = Day::from_days(n).unwrap();
        // 09:15 to 15:29 on the 3rd and 4th, to 15:14 after: whole either way.
        let close = if n <= day(2026, 8, 4).days_from_epoch() {
            hm(15, 30)
        } else {
            hm(15, 15)
        };
        for m in hm(9, 15)..close {
            if d == day(2026, 8, 6) && m == hm(11, 0) {
                continue;
            }
            let row = candle(d, m);
            bars.push(store::format::Bar {
                ts_micros: row.timestamp * 1_000_000,
                open: row.open,
                high: row.high,
                low: row.low,
                close: row.close,
                volume: 1,
                open_interest: store::format::OI_NULL,
            });
        }
    }
    let id = repair_plan::symbol_id("INVENTEDSHR");
    let mut file = store::file::BarFile::open_or_create(
        &scratch.0,
        store::path::StorePath::new(parts).unwrap(),
        id,
    )
    .unwrap();
    file.append(&bars).unwrap();
    drop(file);
    let months = repair_plan::plan(&scratch.0, parts, Window::new(from, to).unwrap());
    assert_eq!(months[0].sessions, 5);
    assert_eq!(months[0].held, 5);
    let short: Vec<(Day, u32, u32, Missing)> = months[0]
        .short
        .iter()
        .map(|s| (s.day, s.owed, s.held, s.fill))
        .collect();
    assert_eq!(short, [(day(2026, 8, 6), 360, 359, Missing::Interior)]);

    // ITS LEDGER LINE SAYS THE MINUTE MAY BE ONE NOBODY TRADED (D-3321).
    let series = gap_backfill::series(
        &scratch.0,
        parts,
        Window::new(from, to).unwrap(),
        &gap_backfill::Have::default(),
    );
    assert_eq!(series.key, "NSE-CASH-INVENTEDSHR");
    assert_eq!(series.lines.len(), 1);
    assert!(
        series.lines[0].starts_with("gap NSE-CASH-INVENTEDSHR 2026-08-06 v=1 rung=1min owed=360 held=359 missing=1 first_missing=11:00 ")
            && series.lines[0].ends_with(" fill=interior source=store absence=may_be_no_trade"),
        "{}",
        series.lines[0]
    );
}

/// Every file under `dir` with its bytes, sorted: the evidence nothing but
/// the ledger was written.
fn tree(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in fs::read_dir(&at).unwrap().filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push((path.clone(), fs::read(&path).unwrap()));
            }
        }
    }
    out.sort();
    out
}

/// The `name=value` fields of a `gap` line that a vendor's line and a store's
/// line must agree on for one day.
fn counted(line: &str) -> Vec<&str> {
    line.split(' ')
        .filter(|w| {
            [
                "owed=",
                "held=",
                "missing=",
                "first_missing=",
                "runs=",
                "longest=",
                "longest_at=",
            ]
            .iter()
            .any(|p| w.starts_with(p))
        })
        .collect()
}

/// **A GAP LEDGER IS WRITTEN FOR STORED MONTHS WITHOUT A VENDOR CALL (R3-18,
/// DB-A12, DB-G46).** The fixture's months are landed, then their ledger is
/// moved aside: the store as it stood before D-3335's ledger existed, every
/// stored hole reading "no ledger". From the store and the calendar alone:
///
/// - one `why=stored_short` line per front, interior or older-month tail
///   session, counted by the SAME counter the landing's vendor audit used: on
///   every such day its owed, held, missing, first missing minute, runs and
///   longest run equal the line the landing wrote;
/// - no line for the first month's `before_first` session or the newest
///   month's tail (the pull frontier), each counted;
/// - a day the ledger already names keeps its line (counted `already`);
/// - nothing but `audit/zerodha.gaps` changes; a second run writes nothing.
#[test]
#[allow(clippy::too_many_lines, reason = "one scenario, read top to bottom")]
fn a_gap_ledger_is_written_for_stored_months_without_a_vendor_call() {
    let scratch = Scratch::new("STORED-LEDGER");
    let expected = db_a12_fixture(&scratch.0);
    let audit = scratch.0.join("audit");
    let landed = ledger(&scratch.0);
    fs::rename(
        audit.join("zerodha.gaps"),
        audit.join("zerodha.gaps.landed"),
    )
    .unwrap();
    let vendor_line = landed
        .lines()
        .find(|l| l.starts_with("gap NSE-INDEX-NIFTY 2019-08-02 "))
        .expect("the landing named 2019-08-02's missing minute")
        .to_owned();
    fs::write(audit.join("zerodha.gaps"), format!("{vendor_line}\n")).unwrap();
    let bars_before = tree(&scratch.0.join("bars"));

    let window = Window::new(day(2019, 1, 1), day(2026, 9, 30)).unwrap();
    let vendor = brutex_core::vendor::Vendor::Zerodha;
    let mut have = gap_backfill::ledgered(&scratch.0, vendor).unwrap();
    assert_eq!(have.len(), 1);
    let mut series = gap_backfill::series(&scratch.0, index_parts(), window, &have);
    assert_eq!(series.key, "NSE-INDEX-NIFTY");
    assert!(
        series.tally.refused.is_empty(),
        "{:?}",
        series.tally.refused
    );
    assert_eq!(series.tally.months, 3);
    assert_eq!(series.tally.short, 7);
    assert_eq!(
        (
            series.tally.already,
            series.tally.before_first,
            series.tally.frontier
        ),
        (1, 1, 1)
    );
    let want: Vec<Day> = expected
        .iter()
        .filter(|(d, _, fill)| {
            *d != day(2019, 8, 2) && matches!(fill, Missing::Front | Missing::Interior)
                || (*fill == Missing::Tail && d.year() == 2019)
        })
        .map(|(d, _, _)| *d)
        .collect();
    assert_eq!(series.days, want);
    assert_eq!(series.lines.len(), 4);
    let outage_owed = match repair_plan::owed("INDEX", day(2021, 2, 24).days_from_epoch()) {
        Owes::Windows((w, c)) => u64::from(w[..c].iter().map(|(a, b)| b - a).sum::<u32>()),
        other => panic!("{other:?}"),
    };
    assert_eq!(series.tally.minutes, 2 + 14 + 5 + (outage_owed - 54));
    for (line, d) in series.lines.iter().zip(&series.days) {
        assert!(
            line.starts_with(&format!("gap NSE-INDEX-NIFTY {d} v=1 rung=1min ")),
            "{line}"
        );
        assert!(
            line.contains(" why=stored_short ") && line.ends_with(" source=store"),
            "{line}"
        );
        let vendor = landed
            .lines()
            .find(|l| l.starts_with(&format!("gap NSE-INDEX-NIFTY {d} ")))
            .unwrap_or_else(|| panic!("the landing's line for {d}"));
        assert_eq!(
            counted(line),
            counted(vendor),
            "{d}: store and vendor audit agree"
        );
    }
    let fills: Vec<&str> = series
        .lines
        .iter()
        .map(|l| l.split(' ').find(|w| w.starts_with("fill=")).unwrap())
        .collect();
    assert_eq!(
        fills,
        ["fill=front", "fill=interior", "fill=tail", "fill=interior"]
    );
    assert!(series.lines[1].starts_with(
        "gap NSE-INDEX-NIFTY 2019-09-23 v=1 rung=1min owed=375 held=361 missing=14 first_missing=15:15 runs=1 longest=14 longest_at=15:15 why=stored_short run="
    ), "{}", series.lines[1]);

    assert_eq!(
        gap_backfill::write(&scratch.0, vendor, &mut series, &mut have).unwrap(),
        4
    );
    let written = ledger(&scratch.0);
    assert_eq!(written.lines().count(), 5);
    assert_eq!(
        tree(&scratch.0.join("bars")),
        bars_before,
        "no bar, census or sidecar byte changed"
    );

    // IDEMPOTENT: a second run, from a fresh read of the ledger, writes nothing.
    let again = gap_backfill::ledgered(&scratch.0, vendor).unwrap();
    assert_eq!(again.len(), 5);
    let mut second = gap_backfill::series(&scratch.0, index_parts(), window, &again);
    assert!(
        second.lines.is_empty() && second.tally.already == 5,
        "{:?}",
        second.tally
    );
    assert_eq!(
        gap_backfill::write(&scratch.0, vendor, &mut second, &mut have.clone()).unwrap(),
        0
    );
    assert_eq!(ledger(&scratch.0), written);

    // A series the store does not hold owes nothing and reads nothing.
    let none = gap_backfill::series(
        &scratch.0,
        store::path::PathParts {
            symbol: "BANKNIFTY",
            ..index_parts()
        },
        window,
        &again,
    );
    assert_eq!((none.lines.len(), none.tally.months), (0, 0));
}

/// **A GAP MINUTE SERVED LATER IS NAMED AND REPAIRABLE (FM-166, R3-18).**
/// 2019-09-23 15:20 is missing when the month lands and its day is a ledger
/// line. The vendor later serves it: an ordinary re-pull is refused by the
/// append door (the minute lies inside the held range) and appends nothing in
/// place, its ledger line names the day again, and the repair plan — which
/// before R3-18 counted the day held — lists it as an interior short, so the
/// repair door fills it as a new generation with a `repaired` line.
#[test]
fn a_gap_minute_served_later_is_named_and_repairable() {
    let scratch = Scratch::new("SERVED-LATER");
    let (from, to) = (day(2019, 9, 1), day(2019, 9, 30));
    let x = (day(2019, 9, 23), hm(15, 20));
    let first = owed_answer("INDEX", from, to, &|d, m| (d, m) != x);
    let req = request(pull::vendor::Listing::Index, from, to);
    let land = |rows: Vec<RawRow>| {
        pull::ingest::from_window(
            &pull::fetch::RawWindow { rows },
            "NIFTY",
            "ZERODHA TEST",
            &scratch.0,
            plan(&req, "INDEX"),
        )
    };
    let done = land(first);
    assert!(done.bars_committed > 0);
    let last_line = |root: &Path| {
        ledger(root)
            .lines()
            .rfind(|l| l.starts_with("gap NSE-INDEX-NIFTY 2019-09-23 "))
            .map(str::to_owned)
    };
    assert!(
        last_line(&scratch.0)
            .unwrap()
            .contains(" missing=1 first_missing=15:20 ")
    );
    let parts = store::path::PathParts {
        month: store::path::YearMonth::new(2019, 9).unwrap(),
        ..index_parts()
    };
    let window = Window::new(from, to).unwrap();
    let short = |root: &Path| repair_plan::plan(root, parts, window)[0].short.clone();
    assert_eq!(short(&scratch.0).len(), 1);
    let held = month_bars(&scratch.0, "INDEX", "NIFTY", Timeframe::MINUTE_1, (2019, 9));

    // The vendor now serves the minute: an ordinary pull appends nothing.
    let whole = owed_answer("INDEX", from, to, &|_, _| true);
    let again = land(whole.clone());
    assert_eq!(again.bars_committed, 0, "{:?}", again.failures);
    assert_eq!(
        month_bars(&scratch.0, "INDEX", "NIFTY", Timeframe::MINUTE_1, (2019, 9)),
        held,
        "nothing appended in place"
    );
    // THE RUN NAMES IT: a new line for the day, last of its key, saying the
    // vendor's 15:20 was SERVED and refused at the append door (`refused_*`),
    // not that the vendor sent nothing (`vendor_empty`).
    let named = last_line(&scratch.0).unwrap();
    assert!(
        named.contains(" missing=1 first_missing=15:20 ") && named.contains(" why=refused_"),
        "the run names the day: {named}"
    );
    assert_eq!(
        ledger(&scratch.0)
            .lines()
            .filter(|l| l.starts_with("gap NSE-INDEX-NIFTY 2019-09-23 "))
            .count(),
        2,
        "the landing's line and the re-pull's"
    );
    let listed = short(&scratch.0);
    assert_eq!(
        listed
            .iter()
            .map(|s| (s.day, s.owed, s.held, s.fill))
            .collect::<Vec<_>>(),
        [(day(2019, 9, 23), 375, 374, Missing::Interior)]
    );
    assert!(repair_plan::plan(&scratch.0, parts, window)[0].needs_repair());

    // The repair door fills it: a new generation, the plan whole, the line
    // superseded by `repaired`.
    let repaired = pull::ingest::repair_window(
        &pull::fetch::RawWindow { rows: whole },
        "NIFTY",
        "ZERODHA TEST",
        &scratch.0,
        plan(&req, "INDEX"),
        "FM-166 test: the vendor now serves 2019-09-23 15:20",
    );
    assert!(repaired.failures.is_empty(), "{:?}", repaired.failures);
    let month = &repair_plan::plan(&scratch.0, parts, window)[0];
    assert!(month.generation > 0 && month.short.is_empty() && !month.needs_repair());
    assert!(last_line(&scratch.0).unwrap().contains(" why=repaired "));
}

/// One invented minute bar at `minute` past IST midnight on day `number`.
fn bar_at(number: u32, minute: u32) -> store::format::Bar {
    let secs = i64::from(number) * 86_400 - IST_OFFSET_SECS + i64::from(minute) * 60;
    store::format::Bar {
        ts_micros: secs * 1_000_000,
        open: 4_321_000,
        high: 4_321_500,
        low: 4_320_500,
        close: 4_321_100,
        volume: 0,
        open_interest: store::format::OI_NULL,
    }
}

/// A store holding `held` months of NIFTY 1min: 2019-09 whole but for
/// 2019-09-23 15:15-15:28 (DB-A12's shape), and the `held - 1` months before
/// it one bar each, so every one is a held month the rung listing names.
fn stored_months(root: &Path, held: u16) {
    let write = |month: store::path::YearMonth, bars: &[store::format::Bar]| {
        let parts = store::path::PathParts {
            month,
            ..index_parts()
        };
        let mut file = store::file::BarFile::open_or_create(
            root,
            store::path::StorePath::new(parts).unwrap(),
            repair_plan::symbol_id("NIFTY"),
        )
        .unwrap();
        file.append(bars).unwrap();
    };
    let mut bars = Vec::new();
    for number in day(2019, 9, 1).days_from_epoch()..=day(2019, 9, 30).days_from_epoch() {
        let Owes::Windows((windows, count)) = repair_plan::owed("INDEX", number) else {
            continue;
        };
        for &(open, close) in &windows[..count] {
            for minute in open..close {
                if number == day(2019, 9, 23).days_from_epoch() && (915..929).contains(&minute) {
                    continue;
                }
                bars.push(bar_at(number, minute));
            }
        }
    }
    write(store::path::YearMonth::new(2019, 9).unwrap(), &bars);
    // The months before 2019-09, newest first: 2019-08, 2019-07, ...
    let mut index: u16 = 2019 * 12 + 8 - 1;
    for _ in 1..held {
        let (year, month) = (index / 12, u8::try_from(index % 12 + 1).unwrap());
        let first = Day::new(year, month, 1).unwrap();
        // 10:00 on the 1st, whatever kind of day it was: a held month.
        write(
            store::path::YearMonth::new(year, month).unwrap(),
            &[bar_at(first.days_from_epoch(), 600)],
        );
        index -= 1;
    }
}

/// One plan of 2019-09 over `root`, in nanoseconds.
fn plan_time(root: &Path) -> u128 {
    let parts = store::path::PathParts {
        month: store::path::YearMonth::new(2019, 9).unwrap(),
        ..index_parts()
    };
    let window = Window::new(day(2019, 9, 1), day(2019, 9, 30)).unwrap();
    let at = std::time::Instant::now();
    let months = std::hint::black_box(repair_plan::plan(root, parts, window));
    let took = at.elapsed().as_nanos();
    assert_eq!(
        months[0].short.len(),
        1,
        "the one short session, every time"
    );
    took
}

/// **THE PLAN PER MONTH ASKED IS FLAT IN THE MONTHS STORED (R3-18).** One
/// month asked of a series holding 1, 10 and 100 months: the plan opens the
/// asked month alone, and its only per-series step is one listing of the rung
/// directory (O(months held), named for `docs/06-limits.md`). n = 200 plans
/// of each, warm (one plan of each first), INTERLEAVED — one of each per
/// round — so a load spike on a shared host lands on all three alike; p50,
/// p99 and max printed. The ratio of 10x and 100x to 1x is held under gate
/// 8's ceiling, 3.0x (`benches/ratio.rs` `CEILING_PERMILLE`), at the median,
/// the statistic one stall cannot inflate; p99's ratio is printed beside it,
/// measured, and not asserted on a shared machine.
#[test]
fn the_plan_per_month_asked_is_flat_in_months_stored() {
    const N: usize = 200;
    const CEILING_PERMILLE: u128 = 3_000;
    const HELD: [u16; 3] = [1, 10, 100];
    let stores: Vec<Scratch> = HELD
        .iter()
        .map(|&held| {
            let scratch = Scratch::new(&format!("PLAN-FLAT-{held}"));
            stored_months(&scratch.0, held);
            assert_eq!(
                repair_plan::held_months(&scratch.0, index_parts()).len(),
                usize::from(held)
            );
            let _warm = plan_time(&scratch.0);
            scratch
        })
        .collect();
    let mut times: Vec<Vec<u128>> = vec![Vec::with_capacity(N); HELD.len()];
    for _ in 0..N {
        for (store, out) in stores.iter().zip(times.iter_mut()) {
            out.push(plan_time(&store.0));
        }
    }
    let mut p50 = Vec::new();
    let mut p99s = Vec::new();
    for (held, mut sample) in HELD.iter().zip(times) {
        sample.sort_unstable();
        let (median, p99, max) = (sample[N / 2], sample[N * 99 / 100], sample[N - 1]);
        println!(
            "repair_plan::plan, one month asked, {held} month(s) stored: n={N} p50={median}ns p99={p99}ns max={max}ns"
        );
        p50.push(median);
        p99s.push(p99);
    }
    for (label, at, tail) in [("10x", p50[1], p99s[1]), ("100x", p50[2], p99s[2])] {
        let permille = at * 1_000 / p50[0].max(1);
        let tail_permille = tail * 1_000 / p99s[0].max(1);
        println!(
            "ratio {label} / 1x: p50 {permille} permille, p99 {tail_permille} permille (printed)"
        );
        assert!(
            permille <= CEILING_PERMILLE,
            "{label} months stored costs {permille} permille of 1x per month asked"
        );
    }
}

/// **WHAT IS AT THE LEDGER'S NAME DECIDES, NOT WHAT IT POINTS AT (R3-18,
/// R3-16's read door).** No ledger is no line yet; a symbolic link (even to
/// a real ledger) and a directory at `audit/zerodha.gaps` are refused by
/// name, never followed and never read as an empty ledger, so no stored-month
/// line is offered against a ledger nobody read; a malformed `gap` line is
/// counted by its line number, never skipped silently.
#[test]
fn a_ledger_name_holding_a_link_or_a_directory_is_refused_by_name() {
    let vendor = brutex_core::vendor::Vendor::Zerodha;
    let scratch = Scratch::new("LEDGER-NAME");
    let audit = scratch.0.join("audit");
    fs::create_dir_all(&audit).unwrap();
    let have = gap_backfill::ledgered(&scratch.0, vendor).unwrap();
    assert!(have.is_empty() && have.faults.is_empty(), "no ledger yet");

    let elsewhere = scratch.0.join("elsewhere.gaps");
    fs::write(
        &elsewhere,
        "gap NSE-INDEX-NIFTY 2019-09-23 v=1 rung=1min owed=375 held=361 missing=14 first_missing=15:15 runs=1 longest=14 longest_at=15:15 why=vendor_empty run=0\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(&elsewhere, audit.join("zerodha.gaps")).unwrap();
    let linked = gap_backfill::ledgered(&scratch.0, vendor).unwrap_err();
    assert!(linked.contains("zerodha.gaps"), "{linked}");

    fs::remove_file(audit.join("zerodha.gaps")).unwrap();
    fs::create_dir(audit.join("zerodha.gaps")).unwrap();
    let directory = gap_backfill::ledgered(&scratch.0, vendor).unwrap_err();
    assert!(directory.contains("zerodha.gaps"), "{directory}");

    fs::remove_dir(audit.join("zerodha.gaps")).unwrap();
    fs::write(
        audit.join("zerodha.gaps"),
        "gap NSE-INDEX-NIFTY 2019-09-23 v=1\ngap NSE-INDEX-NIFTY 23-09-2019 v=1\nderive NSE-INDEX-NIFTY 5min 2019-09 v=1\n",
    )
    .unwrap();
    let parsed = gap_backfill::ledgered(&scratch.0, vendor).unwrap();
    assert_eq!(parsed.len(), 1, "the well-keyed line");
    assert!(parsed.holds("NSE-INDEX-NIFTY", day(2019, 9, 23)));
    assert_eq!(
        parsed.faults,
        [2],
        "the day that is not YYYY-MM-DD, by line"
    );
}

/// The DB-A12 fixture with an EMPTY ledger, read: the store's four lines
/// (2019-09-23 among them) found and none written yet.
fn found_against_an_empty_ledger(
    root: &Path,
) -> (gap_backfill::Have, gap_backfill::Series, Window) {
    let _fixture = db_a12_fixture(root);
    let audit = root.join("audit");
    fs::write(audit.join("zerodha.gaps"), "").unwrap();
    let window = Window::new(day(2019, 1, 1), day(2026, 9, 30)).unwrap();
    let vendor = brutex_core::vendor::Vendor::Zerodha;
    let have = gap_backfill::ledgered(root, vendor).unwrap();
    let series = gap_backfill::series(root, index_parts(), window, &have);
    assert_eq!(series.lines.len(), 5, "{:?}", series.tally);
    (have, series, window)
}

/// The `(key, day)` of every `gap` line, in ledger order.
fn keyed(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter(|l| l.starts_with("gap "))
        .map(|l| {
            let mut w = l.split(' ').skip(1);
            (w.next().unwrap().to_owned(), w.next().unwrap().to_owned())
        })
        .collect()
}

/// **A LINE ANOTHER WRITER APPENDS AFTER THE READ IS NEVER OVERTAKEN**
/// (verify-R3-18-r1, blocking 1). The run reads the ledger and finds
/// 2019-09-23 short; before it writes, a pull appends its own vendor line for
/// that day. The write re-reads the ledger under its lock, drops the
/// store-found line for 2019-09-23 (counted `raced`), and writes the others:
/// the vendor's line stays the last line of its key and day. Before the
/// rework the store-found line was appended after it and hid the vendor's
/// reason.
#[test]
fn a_line_another_writer_appends_after_the_read_is_never_overtaken() {
    let scratch = Scratch::new("RACE-READ");
    let vendor = brutex_core::vendor::Vendor::Zerodha;
    let (mut have, mut series, _) = found_against_an_empty_ledger(&scratch.0);
    assert!(series.days.contains(&day(2019, 9, 23)));
    let pulled = "gap NSE-INDEX-NIFTY 2019-09-23 v=1 rung=1min owed=375 held=361 missing=14 first_missing=15:15 runs=1 longest=14 longest_at=15:15 why=vendor_empty run=99";
    {
        use std::io::Write as _;
        let mut other = fs::OpenOptions::new()
            .append(true)
            .open(scratch.0.join("audit/zerodha.gaps"))
            .unwrap();
        writeln!(other, "{pulled}").unwrap();
    }
    let written = gap_backfill::write(&scratch.0, vendor, &mut series, &mut have).unwrap();
    assert_eq!((written, series.tally.raced), (4, 1));
    let text = ledger(&scratch.0);
    let last_for_day = text
        .lines()
        .rev()
        .find(|l| l.starts_with("gap NSE-INDEX-NIFTY 2019-09-23 "))
        .unwrap();
    assert_eq!(last_for_day, pulled, "the vendor's reason stands:\n{text}");
    assert_eq!(text.lines().count(), 5);
    let mut pairs = keyed(&text);
    let all = pairs.len();
    pairs.sort();
    pairs.dedup();
    assert_eq!(pairs.len(), all, "each (key, day) once:\n{text}");
    // THE READ POSITION MOVED PAST THE APPEND: the same lines offered again
    // are all held, and nothing more is written.
    let written = gap_backfill::write(&scratch.0, vendor, &mut series, &mut have).unwrap();
    assert_eq!(written, 0);
    assert_eq!(ledger(&scratch.0), text);
}

/// **TWO RUNS AT ONCE WRITE EACH LINE ONCE** (verify-R3-18-r1, blocking 1).
/// Two threads each read the empty ledger and find the same five lines, then
/// write together: the lock serialises them and the second re-reads what the
/// first appended, so the ledger holds five lines, not ten.
#[test]
fn two_runs_at_once_write_each_line_once() {
    let scratch = Scratch::new("RACE-TWO");
    let vendor = brutex_core::vendor::Vendor::Zerodha;
    let (have, series, _) = found_against_an_empty_ledger(&scratch.0);
    let barrier = std::sync::Barrier::new(2);
    let written: Vec<u64> = std::thread::scope(|scope| {
        let runs: Vec<_> = (0..2)
            .map(|_| {
                let (mut have, mut series) = (have.clone(), series.clone());
                let (root, barrier) = (&scratch.0, &barrier);
                scope.spawn(move || {
                    barrier.wait();
                    gap_backfill::write(root, vendor, &mut series, &mut have).unwrap()
                })
            })
            .collect();
        runs.into_iter().map(|run| run.join().unwrap()).collect()
    });
    let mut sorted = written.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, [0, 5], "one run wrote, the other found it all");
    let text = ledger(&scratch.0);
    assert_eq!(text.lines().count(), 5, "{text}");
}

/// **THE WRITE WAITS FOR ANOTHER HOLDER OF THE LEDGER'S LOCK.** A writer that
/// holds the ledger's `flock` (as a pull's append does while it writes) keeps
/// the stored-month write out until it lets go; the write then reads what
/// that writer appended and writes nothing over it.
#[test]
fn the_write_waits_for_the_ledgers_lock_and_reads_what_its_holder_wrote() {
    use std::io::Write as _;
    let scratch = Scratch::new("RACE-LOCK");
    let vendor = brutex_core::vendor::Vendor::Zerodha;
    let (mut have, mut series, _) = found_against_an_empty_ledger(&scratch.0);
    let path = scratch.0.join("audit/zerodha.gaps");
    let mut holder = fs::OpenOptions::new().append(true).open(&path).unwrap();
    holder.lock().unwrap();
    let pulled = "gap NSE-INDEX-NIFTY 2019-09-23 v=1 rung=1min owed=375 held=361 missing=14 first_missing=15:15 runs=1 longest=14 longest_at=15:15 why=vendor_empty run=7";
    let (written, raced) = std::thread::scope(|scope| {
        let run = scope.spawn(|| {
            let written = gap_backfill::write(&scratch.0, vendor, &mut series, &mut have);
            (written, series.tally.raced)
        });
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(!run.is_finished(), "the write waited for the lock");
        assert_eq!(fs::read_to_string(&path).unwrap(), "", "nothing yet");
        writeln!(holder, "{pulled}").unwrap();
        holder.unlock().unwrap();
        run.join().unwrap()
    });
    assert_eq!((written.unwrap(), raced), (4, 1));
    let text = ledger(&scratch.0);
    assert_eq!(text.lines().next(), Some(pulled), "{text}");
    assert_eq!(
        text.lines()
            .filter(|l| l.starts_with("gap NSE-INDEX-NIFTY 2019-09-23 "))
            .count(),
        1,
        "{text}"
    );
}

/// **A TORN LAST LINE IS CLOSED, NEVER EXTENDED.** A writer killed mid-line
/// left a `gap` fragment with no line break; the read names its day (so no
/// line is written over it), the write closes it with a line break before
/// its own lines, and every written line stays whole.
#[test]
fn a_torn_last_line_is_closed_before_the_stored_lines() {
    let scratch = Scratch::new("TORN-TAIL");
    let vendor = brutex_core::vendor::Vendor::Zerodha;
    let _fixture = db_a12_fixture(&scratch.0);
    let path = scratch.0.join("audit/zerodha.gaps");
    let torn = "gap NSE-INDEX-NIFTY 2019-09-23 v=1 rung=1min owed=375 held=3";
    fs::write(&path, torn).unwrap();
    let window = Window::new(day(2019, 1, 1), day(2026, 9, 30)).unwrap();
    let mut have = gap_backfill::ledgered(&scratch.0, vendor).unwrap();
    assert!(have.holds("NSE-INDEX-NIFTY", day(2019, 9, 23)));
    let mut series = gap_backfill::series(&scratch.0, index_parts(), window, &have);
    assert_eq!(series.lines.len(), 4);
    assert_eq!(
        gap_backfill::write(&scratch.0, vendor, &mut series, &mut have).unwrap(),
        4
    );
    let text = ledger(&scratch.0);
    assert!(
        text.starts_with(&format!("{torn}\ngap NSE-INDEX-NIFTY ")),
        "{text}"
    );
    assert!(text.ends_with('\n'));
    assert_eq!(text.lines().count(), 5);
    assert!(
        text.lines()
            .skip(1)
            .all(|l| l.contains(" why=stored_short ") && l.ends_with(" source=store")),
        "{text}"
    );
}
