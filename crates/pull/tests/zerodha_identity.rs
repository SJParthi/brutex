//! WP-13d — a broker's minute answer judged against the day the same series
//! was served, through the landing door every broker window takes
//! (`pull::ingest::from_window`, `repair_window`). Findings DB-B07, DB-B08,
//! DB-B10, DB-B60 and DB-B64.
//!
//! Every price is invented: a level of 10,000.00 that no instrument here
//! trades at, and impostors at a tenth, twice and thirty-seven times it.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test-only assertions"
)]

use std::fs;
use std::path::{Path, PathBuf};

use pull::fetch::{RawRow, RawWindow};
use pull::session::{Day, IST_OFFSET_SECS, Window};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("brutex-IDENTITY-{tag}-{}", std::process::id()));
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

/// The invented level every honest candle trades around, in paisa.
const LEVEL: i64 = 1_000_000;

/// One honest minute: inside `LEVEL - 40 ..= LEVEL + 1_300`.
fn candle(day: Day, minute: i64) -> RawRow {
    let base = LEVEL + (minute % 97) * 13 + i64::from(day.day());
    RawRow {
        timestamp: i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + minute * 60,
        open: base,
        high: base + 50,
        low: base - 40,
        close: base + 10,
        volume: 0,
        open_interest: None,
    }
}

/// The same minute as another series would print it: `times / over` the level.
fn impostor(day: Day, minute: i64, times: i64, over: i64) -> RawRow {
    let honest = candle(day, minute);
    let scale = |price: i64| price * times / over;
    RawRow {
        open: scale(honest.open),
        high: scale(honest.high),
        low: scale(honest.low),
        close: scale(honest.close),
        ..honest
    }
}

/// The served day bar of `day`, stamped at IST midnight, holding every honest
/// minute of the session with room to spare.
fn day_candle(day: Day) -> RawRow {
    RawRow {
        timestamp: i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS,
        open: LEVEL,
        high: LEVEL + 2_000,
        low: LEVEL - 1_000,
        close: LEVEL + 500,
        volume: 0,
        open_interest: None,
    }
}

fn request(from: Day, to: Day, granularity: pull::vendor::Granularity) -> pull::fetch::BarRequest {
    pull::fetch::BarRequest {
        instrument_id: String::new(),
        listing: pull::vendor::Listing::Index,
        window: Window::new(from, to).unwrap(),
        granularity,
    }
}

fn plan(request: &pull::fetch::BarRequest) -> pull::ingest::Plan<'_> {
    pull::ingest::Plan {
        calendar: pull::calendar::Runtime::default(),
        cash_schedule: None,
        columns: pull::csv::Columns::Gdfl,
        request,
        encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
        scale: pull::vendor::PriceScale::Paisa,
        vendor: brutex_core::vendor::Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        contract: None,
    }
}

/// Every session of the calendar of record in `from..=to`, with its minutes.
fn sessions(from: Day, to: Day) -> Vec<(Day, Vec<i64>)> {
    (from.days_from_epoch()..=to.days_from_epoch())
        .filter_map(|number| {
            let kind = pull::session::with_recorded_window(
                i64::from(number),
                pull::calendar::sourced_kind_of(i64::from(number)),
            );
            let pull::calendar::DayKind::Open(session) = kind else {
                return None;
            };
            let minutes = (0..1_440_u16)
                .filter(|m| session.expects(*m))
                .map(i64::from)
                .collect();
            Some((Day::from_days(number).unwrap(), minutes))
        })
        .collect()
}

fn answer(from: Day, to: Day) -> Vec<RawRow> {
    sessions(from, to)
        .into_iter()
        .flat_map(|(day, minutes)| minutes.into_iter().map(move |m| candle(day, m)))
        .collect()
}

fn symbol_id(symbol: &str) -> u32 {
    u32::try_from(brutex_core::universe::fnv1a(symbol) & u64::from(u32::MAX)).unwrap()
}

fn month_path(symbol: &'static str, tf: Timeframe, month: (u16, u8)) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: brutex_core::vendor::Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        symbol,
        contract: None,
        timeframe: tf,
        month: YearMonth::new(month.0, month.1).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

fn month_bars(
    root: &Path,
    symbol: &'static str,
    tf: Timeframe,
    month: (u16, u8),
) -> Vec<store::format::Bar> {
    let file =
        store::file::BarFile::open_existing(root, month_path(symbol, tf, month), symbol_id(symbol))
            .unwrap();
    (0..file.header().n_valid)
        .map(|i| file.read_record(i).unwrap())
        .collect()
}

fn month_exists(root: &Path, symbol: &'static str, tf: Timeframe, month: (u16, u8)) -> bool {
    month_path(symbol, tf, month).to_path_buf(root).is_file()
}

fn land_days(root: &Path, symbol: &str, days: &[Day]) {
    let req = request(
        *days.first().unwrap(),
        *days.last().unwrap(),
        pull::vendor::Granularity::Day1,
    );
    let rows = days.iter().copied().map(day_candle).collect();
    let done = pull::ingest::from_window(
        &RawWindow { rows },
        symbol,
        "IDENTITY TEST",
        root,
        plan(&req),
    );
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(done.bars_committed, days.len());
}

fn land(
    root: &Path,
    symbol: &str,
    from: Day,
    to: Day,
    rows: Vec<RawRow>,
) -> pull::ingest::Ingested {
    let req = request(from, to, pull::vendor::Granularity::Minute1);
    pull::ingest::from_window(
        &RawWindow { rows },
        symbol,
        "IDENTITY TEST",
        root,
        plan(&req),
    )
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

fn whys(done: &pull::ingest::Ingested, code: &str) -> Vec<String> {
    done.failures
        .iter()
        .filter(|f| f.why.contains(code))
        .map(|f| f.why.clone())
        .collect()
}

fn gap_ledger(root: &Path) -> String {
    fs::read_to_string(root.join("audit").join("zerodha.gaps")).unwrap_or_default()
}

/// 2024-01-01 (Mon) .. 2024-01-05 (Fri): five ordinary sessions.
const FROM: (u16, u8, u8) = (2024, 1, 1);
const TO: (u16, u8, u8) = (2024, 1, 5);

fn week() -> (Day, Day, Vec<Day>) {
    let (from, to) = (day(FROM.0, FROM.1, FROM.2), day(TO.0, TO.1, TO.2));
    let days: Vec<Day> = sessions(from, to).into_iter().map(|(d, _)| d).collect();
    assert_eq!(days.len(), 5, "the fixture week holds five sessions");
    (from, to, days)
}

/// DB-B07, DB-B10, DB-B60: minutes at another series' level inside a session
/// are refused by name, that minute only, on every key the same way.
#[test]
fn a_minute_wholly_outside_its_own_served_day_is_refused_by_name_and_the_rest_lands() {
    for symbol in ["NIFTY", "BANKNIFTY", "NIFTYNEXT50"] {
        let scratch = Scratch::new(&format!("foreign-{symbol}"));
        let (from, to, days) = week();
        land_days(&scratch.0, symbol, &days);
        let mut rows = answer(from, to);
        let honest = rows.len();
        // Eight minutes at twice the level on the third session (12:40..12:47
        // is minute 765..=772), and six at a tenth on the fourth.
        let mut swapped = 0usize;
        for row in &mut rows {
            let local = row.timestamp + IST_OFFSET_SECS;
            let (number, minute) = (local.div_euclid(86_400), local.rem_euclid(86_400) / 60);
            let on = Day::from_days(u32::try_from(number).unwrap()).unwrap();
            if on == days[2] && (765..=772).contains(&minute) {
                *row = impostor(on, minute, 2, 1);
                swapped += 1;
            } else if on == days[3] && (600..=605).contains(&minute) {
                *row = impostor(on, minute, 1, 10);
                swapped += 1;
            }
        }
        assert_eq!(swapped, 14);
        let done = land(&scratch.0, symbol, from, to, rows);
        let named = whys(&done, "foreign_level");
        assert_eq!(
            named.len(),
            2,
            "{symbol}: one failure per (cause, day): {:?}",
            done.failures
        );
        assert!(named[0].contains("8 row(s) refused"), "{}", named[0]);
        assert!(named[1].contains("6 row(s) refused"), "{}", named[1]);
        assert!(named[0].contains("wholly outside its own served day's range"));
        assert_eq!(done.failures[0].instrument, symbol);
        assert_eq!(
            done.rows_read, honest,
            "{symbol}: every row the vendor sent is counted"
        );

        let held = month_bars(&scratch.0, symbol, Timeframe::MINUTE_1, (2024, 1));
        assert_eq!(
            held.len(),
            honest - 14,
            "{symbol}: the rest of the answer landed"
        );
        let top = held.iter().map(|b| b.high).max().unwrap();
        let bottom = held.iter().map(|b| b.low).min().unwrap();
        assert!(
            top <= LEVEL + 2_000 && bottom >= LEVEL - 1_000,
            "{symbol}: {bottom}..{top}"
        );
        // No derived rung was folded from a refused minute.
        for rung in [
            Timeframe::MINUTE_5,
            Timeframe::MINUTE_15,
            Timeframe::MINUTE_60,
        ] {
            let folded = month_bars(&scratch.0, symbol, rung, (2024, 1));
            assert!(!folded.is_empty(), "{symbol} {}", rung.as_str());
            assert!(
                folded
                    .iter()
                    .all(|b| b.high <= LEVEL + 2_000 && b.low >= LEVEL - 1_000),
                "{symbol} {}: a derived bar holds a refused minute",
                rung.as_str()
            );
        }
        // The gap ledger says the store lacks those minutes on those days.
        let ledger = gap_ledger(&scratch.0);
        for lacking in [days[2], days[3]] {
            assert!(
                ledger
                    .lines()
                    .any(|l| l.starts_with("gap ") && l.contains(&lacking.to_string())),
                "{symbol}: no gap line for {lacking}:\n{ledger}"
            );
        }
    }
}

/// DB-B08, DB-B64: minutes on days before the series' first served day bar
/// are refused, in a month the day rung does not hold at all and in the
/// listing month itself.
#[test]
fn minutes_before_the_first_served_day_bar_are_refused_and_the_listing_day_lands() {
    let scratch = Scratch::new("before-first");
    let symbol = "NIFTY";
    // The day rung starts on the fourth session of the week in January.
    let (from, to, days) = week();
    land_days(&scratch.0, symbol, &days[3..]);

    // A whole earlier month at thirty-seven times the level: no day month.
    let (early_from, early_to) = (day(2021, 3, 1), day(2021, 3, 5));
    let early: Vec<RawRow> = sessions(early_from, early_to)
        .into_iter()
        .flat_map(|(d, minutes)| minutes.into_iter().map(move |m| impostor(d, m, 37, 1)))
        .collect();
    let sent = early.len();
    let early_days = sessions(early_from, early_to).len();
    let done = land(&scratch.0, symbol, early_from, early_to, early);
    let named = whys(&done, "before_first_day_bar");
    assert_eq!(
        named.len(),
        early_days,
        "one failure a day: {:?}",
        done.failures
    );
    assert!(named[0].contains("375 row(s) refused"), "{}", named[0]);
    assert!(named[0].contains("before the first day bar this series was served"));
    assert_eq!(done.rows_read, sent);
    assert_eq!(done.bars_committed, 0);
    assert!(
        !month_exists(&scratch.0, symbol, Timeframe::MINUTE_1, (2021, 3)),
        "a month of refused minutes created a minute file"
    );
    assert!(!month_exists(
        &scratch.0,
        symbol,
        Timeframe::MINUTE_5,
        (2021, 3)
    ));

    // The listing month: honest minutes all week, the first three sessions of
    // it before the first day bar.
    let rows = answer(from, to);
    let sent = rows.len();
    let done = land(&scratch.0, symbol, from, to, rows);
    assert_eq!(
        whys(&done, "before_first_day_bar").len(),
        3,
        "{:?}",
        done.failures
    );
    assert!(whys(&done, "foreign_level").is_empty());
    assert_eq!(done.rows_read, sent);
    let held = month_bars(&scratch.0, symbol, Timeframe::MINUTE_1, (2024, 1));
    assert_eq!(held.len(), 2 * 375);
    let first = pull::series_identity::ist_day(held[0].ts_micros);
    assert_eq!(first, i64::from(days[3].days_from_epoch()));
}

/// A series the day rung does not hold, a day it lacks in the middle, and a
/// minute between the two bands all LAND: none is refused on no evidence.
#[test]
fn what_the_day_rung_cannot_judge_lands_and_a_suspect_minute_lands_too() {
    let (from, to, days) = week();

    let no_rung = Scratch::new("no-rung");
    let rows = answer(from, to);
    let sent = rows.len();
    let done = land(&no_rung.0, "NIFTY", from, to, rows);
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(done.bars_committed, sent);

    let hole = Scratch::new("day-hole");
    land_days(&hole.0, "NIFTY", &[days[0], days[1], days[3], days[4]]);
    let mut rows = answer(from, to);
    // The day the rung lacks carries minutes at twice the level: unjudged.
    for row in &mut rows {
        let local = row.timestamp + IST_OFFSET_SECS;
        let on = Day::from_days(u32::try_from(local.div_euclid(86_400)).unwrap()).unwrap();
        if on == days[2] {
            *row = impostor(on, local.rem_euclid(86_400) / 60, 2, 1);
        }
    }
    let done = land(&hole.0, "NIFTY", from, to, rows);
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(done.bars_committed, sent);

    let suspect = Scratch::new("suspect");
    land_days(&suspect.0, "NIFTY", &days);
    let mut rows = answer(from, to);
    // 5% above the level: past the suspect band, short of the refusing one.
    let at = rows.len() / 2;
    let on = Day::from_days(
        u32::try_from((rows[at].timestamp + IST_OFFSET_SECS).div_euclid(86_400)).unwrap(),
    )
    .unwrap();
    rows[at] = impostor(
        on,
        (rows[at].timestamp + IST_OFFSET_SECS).rem_euclid(86_400) / 60,
        105,
        100,
    );
    let done = land(&suspect.0, "NIFTY", from, to, rows);
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(done.bars_committed, sent);
}

/// The extremes: an answer that is all impostor, a single row, an empty
/// answer, the same answer twice, and the repair door.
#[test]
fn an_all_foreign_answer_a_single_row_an_empty_one_a_rerun_and_the_repair_door() {
    let (from, to, days) = week();

    let all = Scratch::new("all-foreign");
    land_days(&all.0, "NIFTY", &days);
    let rows: Vec<RawRow> = sessions(from, to)
        .into_iter()
        .flat_map(|(d, minutes)| minutes.into_iter().map(move |m| impostor(d, m, 1, 10)))
        .collect();
    let sent = rows.len();
    let done = land(&all.0, "NIFTY", from, to, rows.clone());
    assert_eq!(whys(&done, "foreign_level").len(), 5, "{:?}", done.failures);
    assert_eq!((done.rows_read, done.bars_committed), (sent, 0));
    assert!(!month_exists(
        &all.0,
        "NIFTY",
        Timeframe::MINUTE_1,
        (2024, 1)
    ));
    // The same answer again refuses the same rows and writes nothing.
    let again = land(&all.0, "NIFTY", from, to, rows);
    assert_eq!(whys(&again, "foreign_level").len(), 5);
    assert!(!month_exists(
        &all.0,
        "NIFTY",
        Timeframe::MINUTE_1,
        (2024, 1)
    ));

    let single = Scratch::new("single");
    land_days(&single.0, "NIFTY", &days);
    let done = land(
        &single.0,
        "NIFTY",
        from,
        to,
        vec![impostor(days[0], 555, 2, 1)],
    );
    assert_eq!(whys(&done, "foreign_level").len(), 1, "{:?}", done.failures);
    assert!(whys(&done, "foreign_level")[0].contains("1 row(s) refused"));
    assert_eq!(done.bars_committed, 0);

    let empty = Scratch::new("empty");
    land_days(&empty.0, "NIFTY", &days);
    let done = land(&empty.0, "NIFTY", from, to, Vec::new());
    assert!(whys(&done, "foreign_level").is_empty());
    assert!(whys(&done, "before_first_day_bar").is_empty());
    assert_eq!(done.bars_committed, 0);

    // The versioned repair door takes the same screen: a hole filled with
    // another series' minutes is refused, and the held month is untouched.
    let repair = Scratch::new("repair-door");
    land_days(&repair.0, "NIFTY", &days);
    let honest = answer(days[1], to);
    let held_before = honest.len();
    let done = land(&repair.0, "NIFTY", days[1], to, honest);
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    let mut whole = answer(from, to);
    for row in &mut whole {
        let local = row.timestamp + IST_OFFSET_SECS;
        let on = Day::from_days(u32::try_from(local.div_euclid(86_400)).unwrap()).unwrap();
        if on == days[0] {
            *row = impostor(on, local.rem_euclid(86_400) / 60, 37, 1);
        }
    }
    let req = request(from, to, pull::vendor::Granularity::Minute1);
    let done = pull::ingest::repair_window(
        &RawWindow { rows: whole },
        "NIFTY",
        "IDENTITY TEST",
        &repair.0,
        plan(&req),
        "WP-13d fixture: the leading session is another series'",
    );
    assert_eq!(whys(&done, "foreign_level").len(), 1, "{:?}", done.failures);
    let held = month_bars(&repair.0, "NIFTY", Timeframe::MINUTE_1, (2024, 1));
    assert_eq!(held.len(), held_before, "a refused session was repaired in");
    assert!(held.iter().all(|b| b.high <= LEVEL + 2_000));
}
