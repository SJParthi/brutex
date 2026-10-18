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

// ---------------------------------------------------------------------------
// R4-17: a CASH share's own witnesses — its listing day in the receipted NSE
// master, its own previous minute — and the day-rung arm. RT-DATA-02, ING-01,
// Z5, COR-07. Every master, listing date and price below is invented; the
// symbols are real only so `brutex_core::universe::nse_isin` gives each the
// exact ISIN a landing keys its listing by.
// ---------------------------------------------------------------------------

fn cash_request(
    from: Day,
    to: Day,
    granularity: pull::vendor::Granularity,
) -> pull::fetch::BarRequest {
    pull::fetch::BarRequest {
        listing: pull::vendor::Listing::Equity,
        ..request(from, to, granularity)
    }
}

fn cash_plan(request: &pull::fetch::BarRequest) -> pull::ingest::Plan<'_> {
    pull::ingest::Plan {
        segment: "CASH",
        ..plan(request)
    }
}

fn cash_month(
    root: &Path,
    symbol: &str,
    tf: Timeframe,
    month: (u16, u8),
) -> Vec<store::format::Bar> {
    let path = StorePath::new(PathParts {
        vendor: brutex_core::vendor::Vendor::Zerodha,
        exchange: "NSE",
        segment: "CASH",
        symbol,
        contract: None,
        timeframe: tf,
        month: YearMonth::new(month.0, month.1).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    match store::file::BarFile::open_existing(root, path, symbol_id(symbol)) {
        Ok(file) => (0..file.header().n_valid)
            .map(|i| file.read_record(i).unwrap())
            .collect(),
        Err(store::file::StoreError::Missing { .. }) => Vec::new(),
        Err(why) => panic!("{symbol} {}: {why}", tf.as_str()),
    }
}

fn land_cash(
    root: &Path,
    symbol: &str,
    (from, to): (Day, Day),
    granularity: pull::vendor::Granularity,
    rows: Vec<RawRow>,
) -> pull::ingest::Ingested {
    let req = cash_request(from, to, granularity);
    pull::ingest::from_window(
        &RawWindow { rows },
        symbol,
        "IDENTITY TEST",
        root,
        cash_plan(&req),
    )
}

/// Seconds from 1980-01-01, as the NSE master states a lifecycle date.
fn nse_seconds(on: Day) -> u64 {
    u64::from(on.days_from_epoch() - day(1980, 1, 1).days_from_epoch()) * 86_400
}

/// Install an invented, receipted NSE master dated 2026-08-03 under `root`
/// stating, for each symbol, its listing and removal days (`None` is `0`).
fn install_master(root: &Path, rows: &[(&str, Option<Day>, Option<Day>)]) {
    use std::fmt::Write as _;
    use std::io::Write as _;
    let mut csv = String::from(
        "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn,ListgDt,RmvlDt,RadmssnDt\n",
    );
    for (id, (symbol, listed, removed)) in rows.iter().enumerate() {
        let isin = brutex_core::universe::nse_isin(symbol).expect("an NSE ISIN");
        let cell =
            |on: &Option<Day>| on.map_or_else(|| "0".to_owned(), |d| nse_seconds(d).to_string());
        writeln!(
            csv,
            "{},{symbol},EQ,{},1,{},{},0",
            7_000 + id,
            isin.as_str(),
            cell(listed),
            cell(removed)
        )
        .unwrap();
    }
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(csv.as_bytes()).unwrap();
    let bytes = encoder.finish().unwrap();
    let masters = root.join(pull::listing::MASTER_DIR);
    fs::create_dir_all(&masters).unwrap();
    pull::cash_session_cache::install(&masters, day(2026, 8, 3), &bytes).expect("fixture master");
}

/// The local minute of the day of a fixture row, and its day.
fn local_minute(row: &RawRow) -> (Day, i64) {
    let local = row.timestamp + IST_OFFSET_SECS;
    (
        Day::from_days(u32::try_from(local.div_euclid(86_400)).unwrap()).unwrap(),
        local.rem_euclid(86_400) / 60,
    )
}

/// PIIND's shape (RT-DATA-02 a): inside one session, a block of minutes at
/// another price basis — the 09:15 close 821.65 beside the 09:16 open 89.90 —
/// under a day bar that spans both, so the day rung cannot tell them apart.
/// The block is refused whole by name; the minutes after it, back on the
/// series' own basis, land; nothing refused is folded.
#[test]
fn a_session_interleaving_two_price_bases_is_refused_at_landing() {
    let scratch = Scratch::new("interleave");
    let symbol = "PIIND";
    let (from, to, days) = week();
    // A day bar mixed like the vendor's: low at the foreign basis, high at the
    // series' own.
    let mixed = RawRow {
        low: LEVEL * 8_990 / 82_165 - 1_000,
        ..day_candle(days[2])
    };
    let mut day_rows: Vec<RawRow> = days.iter().copied().map(day_candle).collect();
    day_rows[2] = mixed;
    let done = land_cash(
        &scratch.0,
        symbol,
        (from, to),
        pull::vendor::Granularity::Day1,
        day_rows,
    );
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    let mut rows = answer(from, to);
    let honest = rows.len();
    let mut swapped = 0usize;
    for row in &mut rows {
        let (on, minute) = local_minute(row);
        // 10:00..10:09: ten minutes at 8,990/82,165 of the level.
        if on == days[2] && (600..=609).contains(&minute) {
            *row = impostor(on, minute, 8_990, 82_165);
            swapped += 1;
        }
    }
    assert_eq!(swapped, 10);
    let done = land_cash(
        &scratch.0,
        symbol,
        (from, to),
        pull::vendor::Granularity::Minute1,
        rows,
    );
    let named = whys(&done, "in_session_jump");
    assert_eq!(
        named.len(),
        1,
        "one failure per (cause, day): {:?}",
        done.failures
    );
    assert!(named[0].contains("10 row(s) refused"), "{}", named[0]);
    assert!(named[0].contains("continuous run"), "{}", named[0]);
    assert!(
        whys(&done, "foreign_level").is_empty(),
        "the mixed day bar holds them"
    );
    let held = cash_month(&scratch.0, symbol, Timeframe::MINUTE_1, (2024, 1));
    assert_eq!(held.len(), honest - 10, "the rest of the answer landed");
    assert!(
        held.iter().all(|b| b.low >= LEVEL - 1_000),
        "no foreign minute stored"
    );
    for rung in [Timeframe::MINUTE_5, Timeframe::MINUTE_15] {
        let folded = cash_month(&scratch.0, symbol, rung, (2024, 1));
        assert!(!folded.is_empty());
        assert!(
            folded.iter().all(|b| b.low >= LEVEL - 1_000),
            "{}",
            rung.as_str()
        );
    }
}

/// A halt or a missing minute splits the run, and a session's first minute is
/// never judged: a level change across either is not a jump.
#[test]
fn a_halt_splits_the_run_and_a_sessions_first_minute_is_never_a_jump() {
    let scratch = Scratch::new("halt");
    let symbol = "PIIND";
    let (from, to, days) = week();
    let wide = |d: Day| RawRow {
        low: LEVEL / 2 - 1_000,
        high: LEVEL * 2,
        ..day_candle(d)
    };
    let done = land_cash(
        &scratch.0,
        symbol,
        (from, to),
        pull::vendor::Granularity::Day1,
        days.iter().copied().map(wide).collect(),
    );
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    let mut rows = answer(from, to);
    // Day 2: a halt from 11:00 to 11:44, after which the level is 1.6 times.
    // Day 4: the whole session at 1.6 times the level from its first minute.
    rows.retain(|row| {
        let (on, minute) = local_minute(row);
        !(on == days[1] && (660..705).contains(&minute))
    });
    let kept = rows.len();
    for row in &mut rows {
        let (on, minute) = local_minute(row);
        if (on == days[1] && minute >= 705) || on == days[3] {
            *row = impostor(on, minute, 8, 5);
        }
    }
    let done = land_cash(
        &scratch.0,
        symbol,
        (from, to),
        pull::vendor::Granularity::Minute1,
        rows,
    );
    assert!(
        whys(&done, "in_session_jump").is_empty(),
        "{:?}",
        done.failures
    );
    assert_eq!(
        cash_month(&scratch.0, symbol, Timeframe::MINUTE_1, (2024, 1)).len(),
        kept
    );
}

/// MAZDOCK's shape (ING-01 c): 1day bars with o=h=l=c=0 land today. The
/// day-rung arm refuses each by name and lands the rest. An index's zero day
/// bar is NOT refused (D-4992 point 3, the test below): only a price below
/// zero is, on an index.
#[test]
fn a_zero_price_day_bar_is_refused_by_name_at_landing() {
    zero_day_landing("MAZDOCK", "CASH", 2);
}

/// D-4992, measured: an index's zero is its own (`NIFTY50DIVPOINT` restarts
/// from zero after each March expiry), so on an INDEX the zero day bar lands
/// and only the row below zero is refused by name.
#[test]
fn an_index_zero_day_bar_lands_and_a_negative_one_is_refused_by_name() {
    zero_day_landing("NIFTY50DIVPOINT", "INDEX", 1);
}

/// A week of day bars, row 1 at zero and row 3 with a low of -5 paisa, landed
/// under `segment`: `refused` of them are refused by name, the rest land.
fn zero_day_landing(symbol: &str, segment: &str, refused: usize) {
    {
        let scratch = Scratch::new(&format!("zero-day-{symbol}"));
        let (from, to, days) = week();
        let mut rows: Vec<RawRow> = days.iter().copied().map(day_candle).collect();
        rows[1] = RawRow {
            open: 0,
            high: 0,
            low: 0,
            close: 0,
            ..rows[1]
        };
        rows[3] = RawRow { low: -5, ..rows[3] };
        let req = cash_request(from, to, pull::vendor::Granularity::Day1);
        let req = pull::fetch::BarRequest {
            listing: if segment == "CASH" {
                pull::vendor::Listing::Equity
            } else {
                pull::vendor::Listing::Index
            },
            ..req
        };
        let done = pull::ingest::from_window(
            &RawWindow { rows },
            symbol,
            "IDENTITY TEST",
            &scratch.0,
            pull::ingest::Plan {
                segment,
                ..plan(&req)
            },
        );
        let named = whys(&done, "non_positive_price");
        assert_eq!(named.len(), refused, "{symbol}: {:?}", done.failures);
        // Each refusal names its own row of the answer.
        let negative = named.last().unwrap();
        assert!(negative.starts_with("row 3:"), "{negative}");
        assert!(negative.contains("low -5"), "{negative}");
        if refused == 2 {
            assert!(named[0].starts_with("row 1:"), "{}", named[0]);
            assert!(
                named[0].contains("open 0, high 0, low 0, close 0"),
                "{}",
                named[0]
            );
        }
        assert_eq!(
            done.bars_committed,
            5 - refused,
            "{symbol}: every other day lands"
        );
    }
}

/// The day rung is the witness, never the accused: a day bar at any positive
/// level lands, however far from its neighbours (D-4992).
#[test]
fn the_day_rung_is_never_judged_by_range() {
    let scratch = Scratch::new("day-range");
    let symbol = "MAZDOCK";
    let (from, to, days) = week();
    let mut rows: Vec<RawRow> = days.iter().copied().map(day_candle).collect();
    rows[2] = RawRow {
        open: LEVEL * 37,
        high: LEVEL * 37 + 10,
        low: LEVEL * 37 - 10,
        close: LEVEL * 37,
        ..rows[2]
    };
    rows[4] = RawRow {
        open: 1,
        high: 1,
        low: 1,
        close: 1,
        ..rows[4]
    };
    let done = land_cash(
        &scratch.0,
        symbol,
        (from, to),
        pull::vendor::Granularity::Day1,
        rows,
    );
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(done.bars_committed, 5);
}

/// IRFC's shape (Z5, ING-01 d): day bars before the master's listing day are
/// another security's; the listing day itself lands.
#[test]
fn a_day_bar_before_listing_is_refused_by_name() {
    let scratch = Scratch::new("day-before-listing");
    let symbol = "IRFC";
    let (from, to, days) = week();
    install_master(&scratch.0, &[(symbol, Some(days[2]), None)]);
    let rows = days.iter().copied().map(day_candle).collect();
    let done = land_cash(
        &scratch.0,
        symbol,
        (from, to),
        pull::vendor::Granularity::Day1,
        rows,
    );
    let named = whys(&done, "before_listing");
    assert_eq!(named.len(), 2, "one per (cause, day): {:?}", done.failures);
    assert!(named[0].contains("nse_security_master"), "{}", named[0]);
    assert_eq!(done.bars_committed, 3, "the listing day and after land");
    let held = cash_month(&scratch.0, symbol, Timeframe::DAY_1, (2024, 1));
    let first = pull::series_identity::ist_day(held[0].ts_micros);
    assert_eq!(first, i64::from(days[2].days_from_epoch()));
}

/// IRFC's and KFINTECH's minutes years before their listing (Z5): refused as
/// `before_listing`, not `before_first_day_bar`. DALBHARAT's (D-5832): minutes
/// on or after the listing and before the vendor's first day bar are the
/// share's own and land.
#[test]
fn a_minute_before_listing_is_refused_by_name() {
    let scratch = Scratch::new("minute-before-listing");
    let (from, to, days) = week();
    install_master(
        &scratch.0,
        &[
            ("IRFC", Some(days[2]), None),
            ("DALBHARAT", Some(days[0]), None),
        ],
    );
    for symbol in ["IRFC", "DALBHARAT"] {
        // The vendor's day rung starts on the third session for both.
        let rows = days[2..].iter().copied().map(day_candle).collect();
        let done = land_cash(
            &scratch.0,
            symbol,
            (days[2], to),
            pull::vendor::Granularity::Day1,
            rows,
        );
        assert!(done.failures.is_empty(), "{symbol}: {:?}", done.failures);
    }
    let honest = answer(from, to).len();
    let before: usize = sessions(from, to)
        .iter()
        .filter(|(d, _)| *d < days[2])
        .map(|(_, minutes)| minutes.len())
        .sum();

    let done = land_cash(
        &scratch.0,
        "IRFC",
        (from, to),
        pull::vendor::Granularity::Minute1,
        answer(from, to),
    );
    let named = whys(&done, "before_listing");
    assert_eq!(named.len(), 2, "{:?}", done.failures);
    assert!(
        whys(&done, "before_first_day_bar").is_empty(),
        "the listing, never the first day bar"
    );
    assert_eq!(
        cash_month(&scratch.0, "IRFC", Timeframe::MINUTE_1, (2024, 1)).len(),
        honest - before
    );

    let done = land_cash(
        &scratch.0,
        "DALBHARAT",
        (from, to),
        pull::vendor::Granularity::Minute1,
        answer(from, to),
    );
    assert!(
        done.failures.iter().all(|f| !f.why.contains("refused for")),
        "{:?}",
        done.failures
    );
    assert_eq!(
        cash_month(&scratch.0, "DALBHARAT", Timeframe::MINUTE_1, (2024, 1)).len(),
        honest
    );
}

/// TATACONSUM's and TMPV's shape: a master whose `ListgDt` is `0` states no
/// listing, so nothing is refused on it; minutes before the first day bar fall
/// back to WP-13d's rule, and day bars on any day land.
#[test]
fn epoch_zero_listing_refuses_nothing() {
    let scratch = Scratch::new("epoch-zero-landing");
    let symbol = "TATACONSUM";
    let (from, to, days) = week();
    install_master(&scratch.0, &[(symbol, None, None)]);
    let rows = days[2..].iter().copied().map(day_candle).collect();
    let done = land_cash(
        &scratch.0,
        symbol,
        (days[2], to),
        pull::vendor::Granularity::Day1,
        rows,
    );
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    let done = land_cash(
        &scratch.0,
        symbol,
        (from, to),
        pull::vendor::Granularity::Minute1,
        answer(from, to),
    );
    assert!(
        whys(&done, "before_listing").is_empty(),
        "{:?}",
        done.failures
    );
    assert_eq!(
        whys(&done, "before_first_day_bar").len(),
        2,
        "{:?}",
        done.failures
    );
    // A day bar years earlier lands: the listing is unknown.
    let early = day(2019, 1, 1);
    let early = day_candle(Day::from_days(early.days_from_epoch() + 1).unwrap());
    let done = land_cash(
        &scratch.0,
        symbol,
        (day(2019, 1, 1), day(2019, 1, 3)),
        pull::vendor::Granularity::Day1,
        vec![early],
    );
    assert!(done.failures.is_empty(), "{:?}", done.failures);
}

/// A share in no master on disk (DHFL, JETAIRWAYS): its span is the vendor's
/// own, labelled `inferred_from_vendor_span`, and it refuses nothing a day bar
/// carries.
#[test]
fn a_name_absent_from_every_master_is_labelled_inferred() {
    let scratch = Scratch::new("inferred-landing");
    let (from, to, days) = week();
    install_master(&scratch.0, &[("IRFC", Some(days[4]), None)]);
    let rows = days.iter().copied().map(day_candle).collect();
    let done = land_cash(
        &scratch.0,
        "PIIND",
        (from, to),
        pull::vendor::Granularity::Day1,
        rows,
    );
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(done.bars_committed, 5);
    let scan = pull::ingest::identity_screen::scan_series(
        &scratch.0,
        PathParts {
            vendor: brutex_core::vendor::Vendor::Zerodha,
            exchange: "NSE",
            segment: "CASH",
            symbol: "PIIND",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2024, 1).unwrap(),
            file: FileKind::Bars,
        },
    );
    assert_eq!(scan.listing, "inferred_from_vendor_span");
    assert_eq!(scan.listed_on, None);
}

/// COR-07: a 404 or an empty answer for days after a master's removal day is
/// the end of the security's history, recorded and never a failure; a 404 for
/// a security still listed is the failure it was.
#[test]
fn a_404_after_delisting_is_history_ending_not_a_failure() {
    use pull::fetch::{FakeSource, FetchError, fetch_and_land_listed};
    let scratch = Scratch::new("history-ends");
    let removed = day(2023, 3, 31);
    install_master(
        &scratch.0,
        &[
            ("IRFC", Some(day(2021, 1, 4)), Some(removed)),
            ("PIIND", Some(day(2010, 1, 4)), None),
        ],
    );
    let table = pull::listing::Listing::load(&scratch.0.join(pull::listing::MASTER_DIR));
    let isin = |s: &str| brutex_core::universe::nse_isin(s).unwrap();
    let gone = table
        .span("NSE", "IRFC", Some(isin("IRFC").as_str()))
        .unwrap();
    let listed = table
        .span("NSE", "PIIND", Some(isin("PIIND").as_str()))
        .unwrap();
    let not_found = || FetchError::VendorRefused {
        status: 404,
        detail: "not found".to_owned(),
        named: None,
    };
    let after = cash_request(
        day(2024, 1, 1),
        day(2024, 1, 5),
        pull::vendor::Granularity::Day1,
    );
    let encoding = pull::vendor::TimestampEncoding::EpochSecondsUtc;
    let scale = pull::vendor::PriceScale::Paisa;
    for source in [FakeSource::refusing(not_found()), FakeSource::default()] {
        let landed = fetch_and_land_listed(&source, &after, encoding, scale, Some(&gone))
            .expect("the end of history is not a failure");
        assert_eq!(
            landed.history_ends,
            Some(pull::fetch::HistoryEnds {
                delisted_on: removed
            })
        );
        assert!(landed.bars.is_empty());
    }
    assert_eq!(pull::fetch::HistoryEnds::CODE, "history_ends_delisted");
    // Still listed: the 404 is the failure it was.
    let refused = fetch_and_land_listed(
        &FakeSource::refusing(not_found()),
        &after,
        encoding,
        scale,
        Some(&listed),
    );
    assert_eq!(refused, Err(not_found()));
    // A window that includes the removal day is judged as today.
    let straddle = cash_request(removed, day(2023, 4, 5), pull::vendor::Granularity::Day1);
    let refused = fetch_and_land_listed(
        &FakeSource::refusing(not_found()),
        &straddle,
        encoding,
        scale,
        Some(&gone),
    );
    assert_eq!(refused, Err(not_found()));
    // Another refusal after delisting is still that refusal.
    let other = FetchError::VendorRefused {
        status: 500,
        detail: "server".to_owned(),
        named: None,
    };
    let refused = fetch_and_land_listed(
        &FakeSource::refusing(other.clone()),
        &after,
        encoding,
        scale,
        Some(&gone),
    );
    assert_eq!(refused, Err(other));
    // An index is never judged by a share's span.
    let index = request(
        day(2024, 1, 1),
        day(2024, 1, 5),
        pull::vendor::Granularity::Day1,
    );
    let refused = fetch_and_land_listed(
        &FakeSource::refusing(not_found()),
        &index,
        encoding,
        scale,
        Some(&gone),
    );
    assert_eq!(refused, Err(not_found()));
    // Rows that do arrive after delisting are landed as rows, not ended.
    let rows = vec![day_candle(day(2024, 1, 2))];
    let landed = fetch_and_land_listed(
        &FakeSource::returning(rows),
        &after,
        encoding,
        scale,
        Some(&gone),
    )
    .unwrap();
    assert_eq!(landed.history_ends, None);
    assert_eq!(landed.bars.len(), 1);
}

/// An option's price of zero is not this rule's (D-4992): a zero day bar and a
/// zero minute of an option contract land, on either arm.
#[test]
fn an_options_zero_price_is_never_refused_as_non_positive() {
    let scratch = Scratch::new("option-zero");
    let (from, to, days) = week();
    let contract = brutex_core::instrument::Contract::parse("2024-01-25-2150000-CE").unwrap();
    let option = |granularity| pull::fetch::BarRequest {
        listing: pull::vendor::Listing::Derivative,
        ..request(from, to, granularity)
    };
    let zero = |row: RawRow| RawRow {
        open: 0,
        high: 0,
        low: 0,
        close: 0,
        ..row
    };
    for (granularity, rows) in [
        (
            pull::vendor::Granularity::Day1,
            days.iter()
                .copied()
                .map(day_candle)
                .map(zero)
                .collect::<Vec<_>>(),
        ),
        (
            pull::vendor::Granularity::Minute1,
            answer(days[0], days[0]).into_iter().map(zero).collect(),
        ),
    ] {
        let req = option(granularity);
        let done = pull::ingest::from_window(
            &RawWindow { rows },
            "NIFTY",
            "IDENTITY TEST",
            &scratch.0,
            pull::ingest::Plan {
                segment: "FNO",
                contract: Some(contract),
                ..plan(&req)
            },
        );
        assert!(
            whys(&done, "non_positive_price").is_empty(),
            "{:?}",
            done.failures
        );
    }
}

/// Write `bytes` where `symbol`'s CASH day month 2024-01 is, so it will not
/// open as a bar file.
fn unreadable_day_month(root: &Path, symbol: &str, bytes: &[u8]) {
    let path = StorePath::new(PathParts {
        vendor: brutex_core::vendor::Vendor::Zerodha,
        exchange: "NSE",
        segment: "CASH",
        symbol,
        contract: None,
        timeframe: Timeframe::DAY_1,
        month: YearMonth::new(2024, 1).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
    .to_path_buf(root);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, bytes).unwrap();
}

/// A day rung that will not open refuses nothing BY RANGE: a clean answer
/// lands whole, and the screen says so rather than refusing on a witness it
/// could not read.
#[test]
fn a_day_rung_that_will_not_open_refuses_nothing_by_range() {
    let scratch = Scratch::new("unread-day");
    let (from, to, _) = week();
    let path = StorePath::new(PathParts {
        vendor: brutex_core::vendor::Vendor::Zerodha,
        exchange: "NSE",
        segment: "CASH",
        symbol: "PIIND",
        contract: None,
        timeframe: Timeframe::DAY_1,
        month: YearMonth::new(2024, 1).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
    .to_path_buf(&scratch.0);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"not a bar file").unwrap();
    let rows = answer(from, to);
    let sent = rows.len();
    let done = land_cash(
        &scratch.0,
        "PIIND",
        (from, to),
        pull::vendor::Granularity::Minute1,
        rows,
    );
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(
        cash_month(&scratch.0, "PIIND", Timeframe::MINUTE_1, (2024, 1)).len(),
        sent
    );
    // The store scan names the month it could not open and judges nothing by it.
    let scan = pull::ingest::identity_screen::scan_series(
        &scratch.0,
        PathParts {
            vendor: brutex_core::vendor::Vendor::Zerodha,
            exchange: "NSE",
            segment: "CASH",
            symbol: "PIIND",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2024, 1).unwrap(),
            file: FileKind::Bars,
        },
    );
    assert_eq!(scan.unread.len(), 1, "{:?}", scan.unread);
    assert!(scan.unread[0].1.starts_with("1day: "), "{:?}", scan.unread);
    assert!(scan.day_verdicts.is_empty());
    // An instrument that is no symbol is not judged here: the landing refuses
    // it by name itself.
    let done = land_cash(
        &scratch.0,
        "not a symbol",
        (from, to),
        pull::vendor::Granularity::Minute1,
        answer(from, to),
    );
    assert!(!done.failures.is_empty());
    assert!(whys(&done, "refused for").is_empty(), "{:?}", done.failures);
}

/// R4-17 repair 1, ING-01 c: a zero-price minute is refused by name even
/// when the day rung will not open. That rule reads no day bar, so a torn
/// day month is no reason to store a price of zero (it was stored, with no
/// failure, before this repair).
#[test]
fn a_zero_minute_is_refused_by_name_when_the_day_rung_will_not_open() {
    let scratch = Scratch::new("zero-unread-day");
    let (from, to, _) = week();
    unreadable_day_month(&scratch.0, "MAZDOCK", b"not a bar file");
    let mut rows = answer(from, to);
    let sent = rows.len();
    rows[41] = RawRow {
        open: 0,
        high: 0,
        low: 0,
        close: 0,
        ..rows[41]
    };
    let done = land_cash(
        &scratch.0,
        "MAZDOCK",
        (from, to),
        pull::vendor::Granularity::Minute1,
        rows,
    );
    let named = whys(&done, "non_positive_price");
    assert_eq!(named.len(), 1, "{:?}", done.failures);
    assert!(named[0].contains("1 row(s) refused"), "{}", named[0]);
    assert_eq!(done.failures.len(), 1, "nothing else: {:?}", done.failures);
    let stored = cash_month(&scratch.0, "MAZDOCK", Timeframe::MINUTE_1, (2024, 1));
    assert_eq!(stored.len(), sent - 1);
    assert!(
        stored
            .iter()
            .all(|b| b.open > 0 && b.high > 0 && b.low > 0 && b.close > 0),
        "no price at or below zero stored"
    );
}

/// R4-17 repair 1, ING-01 d: a CASH minute before its master's listing day
/// is refused by name even when the day rung will not open: its witness is
/// the master, which was read. IRFC's shape, listed on the third session.
#[test]
fn a_minute_before_listing_is_refused_by_name_when_the_day_rung_will_not_open() {
    let scratch = Scratch::new("listing-unread-day");
    let (from, to, days) = week();
    install_master(&scratch.0, &[("IRFC", Some(days[2]), None)]);
    unreadable_day_month(&scratch.0, "IRFC", b"not a bar file");
    let rows = answer(from, to);
    let sent = rows.len();
    let done = land_cash(
        &scratch.0,
        "IRFC",
        (from, to),
        pull::vendor::Granularity::Minute1,
        rows,
    );
    let named = whys(&done, "before_listing");
    assert_eq!(named.len(), 2, "one per day: {:?}", done.failures);
    assert!(
        named.iter().all(|why| why.contains("375 row(s) refused")),
        "{named:?}"
    );
    assert_eq!(done.failures.len(), 2, "nothing else: {:?}", done.failures);
    let stored = cash_month(&scratch.0, "IRFC", Timeframe::MINUTE_1, (2024, 1));
    assert_eq!(stored.len(), sent - 750);
    let listed = i64::from(days[2].days_from_epoch());
    assert!(
        stored
            .iter()
            .all(|b| pull::series_identity::ist_day(b.ts_micros) >= listed),
        "no minute before the listing stored"
    );
}

/// R4-17 repair 1, RT-DATA-02 a: the jump rule reads no day bar either, so a
/// block of minutes at another basis is refused whole by name when the day
/// rung will not open, and the rest of the answer lands.
#[test]
fn an_in_session_jump_is_refused_by_name_when_the_day_rung_will_not_open() {
    let scratch = Scratch::new("jump-unread-day");
    let (from, to, days) = week();
    unreadable_day_month(&scratch.0, "PIIND", b"not a bar file");
    let mut rows = answer(from, to);
    let honest = rows.len();
    for row in &mut rows {
        let (on, minute) = local_minute(row);
        if on == days[2] && (600..=609).contains(&minute) {
            *row = impostor(on, minute, 8_990, 82_165);
        }
    }
    let done = land_cash(
        &scratch.0,
        "PIIND",
        (from, to),
        pull::vendor::Granularity::Minute1,
        rows,
    );
    let named = whys(&done, "in_session_jump");
    assert_eq!(named.len(), 1, "{:?}", done.failures);
    assert!(named[0].contains("10 row(s) refused"), "{}", named[0]);
    assert_eq!(
        done.failures.len(),
        1,
        "nothing by range: {:?}",
        done.failures
    );
    let held = cash_month(&scratch.0, "PIIND", Timeframe::MINUTE_1, (2024, 1));
    assert_eq!(held.len(), honest - 10);
    assert!(
        held.iter().all(|b| b.low >= LEVEL - 1_000),
        "no foreign minute stored"
    );
}

/// A zero-price minute of a share or an index is refused by name on the
/// minute arm, before any other rule (ING-01 c), and the rest lands.
#[test]
fn a_zero_price_minute_is_refused_by_name() {
    let scratch = Scratch::new("zero-minute");
    let (from, to, days) = week();
    let rows = days.iter().copied().map(day_candle).collect();
    let done = land_cash(
        &scratch.0,
        "MAZDOCK",
        (from, to),
        pull::vendor::Granularity::Day1,
        rows,
    );
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    let mut rows = answer(from, to);
    let sent = rows.len();
    rows[40].low = 0;
    rows[41] = RawRow {
        open: 0,
        high: 0,
        low: 0,
        close: 0,
        ..rows[41]
    };
    let done = land_cash(
        &scratch.0,
        "MAZDOCK",
        (from, to),
        pull::vendor::Granularity::Minute1,
        rows,
    );
    let named = whys(&done, "non_positive_price");
    assert_eq!(named.len(), 1, "{:?}", done.failures);
    assert!(named[0].contains("2 row(s) refused"), "{}", named[0]);
    assert_eq!(
        cash_month(&scratch.0, "MAZDOCK", Timeframe::MINUTE_1, (2024, 1)).len(),
        sent - 2
    );
}

/// A price the landing cannot hold (a rupee price that overflows paisa) is
/// not judged here, on either arm: the landing refuses it in its own words.
#[test]
fn a_price_the_landing_cannot_hold_is_left_to_the_landing() {
    let scratch = Scratch::new("overflow");
    let (from, to, days) = week();
    for (granularity, mut rows) in [
        (
            pull::vendor::Granularity::Day1,
            days.iter().copied().map(day_candle).collect::<Vec<_>>(),
        ),
        (pull::vendor::Granularity::Minute1, answer(days[0], days[0])),
    ] {
        for row in &mut rows {
            *row = RawRow {
                open: row.open / 100,
                high: row.high / 100,
                low: row.low / 100,
                close: row.close / 100,
                ..*row
            };
        }
        rows[1].high = i64::MAX / 10;
        let req = cash_request(from, to, granularity);
        let done = pull::ingest::from_window(
            &RawWindow { rows },
            "PIIND",
            "IDENTITY TEST",
            &scratch.0,
            pull::ingest::Plan {
                scale: pull::vendor::PriceScale::Rupees,
                ..cash_plan(&req)
            },
        );
        assert_eq!(
            whys(&done, "price_off_grid").len(),
            1,
            "{:?}",
            done.failures
        );
        assert!(whys(&done, "non_positive_price").is_empty());
    }
}

/// The jump rule is a CASH rule (D-4993 item 4): the same interleave under an
/// index key and a day bar that spans both levels is not judged by it. An
/// index is judged by its day rung alone (`foreign_level`).
#[test]
fn an_index_is_never_judged_by_the_jump_rule() {
    let scratch = Scratch::new("index-no-jump");
    let symbol = "NIFTY";
    let (from, to, days) = week();
    let mut day_rows: Vec<RawRow> = days.iter().copied().map(day_candle).collect();
    day_rows[2].low = LEVEL * 8_990 / 82_165 - 1_000;
    let req = request(from, to, pull::vendor::Granularity::Day1);
    let done = pull::ingest::from_window(
        &RawWindow { rows: day_rows },
        symbol,
        "IDENTITY TEST",
        &scratch.0,
        plan(&req),
    );
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    let mut rows = answer(from, to);
    let sent = rows.len();
    for row in &mut rows {
        let (on, minute) = local_minute(row);
        if on == days[2] && (600..=609).contains(&minute) {
            *row = impostor(on, minute, 8_990, 82_165);
        }
    }
    let done = land(&scratch.0, symbol, from, to, rows);
    assert!(
        whys(&done, "in_session_jump").is_empty(),
        "{:?}",
        done.failures
    );
    assert_eq!(
        month_bars(&scratch.0, symbol, Timeframe::MINUTE_1, (2024, 1)).len(),
        sent
    );
}

/// The one telemetry sink this test binary installs, or the one already there:
/// `telemetry::install` refuses a second, so every log test shares it.
fn log_sink() -> &'static telemetry::Sink {
    let dir = std::env::temp_dir().join(format!("brutex-identity-log-{}", std::process::id()));
    let config = telemetry::Config::new(&dir).with_min_level(telemetry::Level::Trace);
    match telemetry::install(&config) {
        Ok(installed) => installed,
        Err(_already) => telemetry::global().expect("install named the sink that exists"),
    }
}

/// Every `pull.identity` line the sink holds for `instrument`.
fn identity_lines(sink: &telemetry::Sink, instrument: &str) -> Vec<String> {
    let needle = format!("\"instrument\":\"{instrument}\"");
    sink.paths()
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .flat_map(|text| text.lines().map(str::to_owned).collect::<Vec<_>>())
        .filter(|line| line.contains("\"target\":\"pull.identity\"") && line.contains(&needle))
        .collect()
}

/// `/logs` says what the screen did (one `warn` per refused cause and day, an
/// `info` line for minutes left unjudged) and says nothing about an answer
/// that was wholly held. Each symbol here is used by no other test in this
/// binary, so the global sink's lines can be told apart.
#[test]
fn the_screen_names_each_refused_cause_and_day_on_the_log() {
    let sink = log_sink();
    let scratch = Scratch::new("identity-log");
    let (from, to, days) = week();

    // KFINTECH: a zero day bar (day arm), then a zero minute (minute arm).
    let mut day_rows: Vec<RawRow> = days.iter().copied().map(day_candle).collect();
    day_rows[1] = RawRow {
        open: 0,
        high: 0,
        low: 0,
        close: 0,
        ..day_rows[1]
    };
    let done = land_cash(
        &scratch.0,
        "KFINTECH",
        (from, to),
        pull::vendor::Granularity::Day1,
        day_rows,
    );
    assert_eq!(whys(&done, "non_positive_price").len(), 1);
    let lines = identity_lines(sink, "KFINTECH");
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("\"level\":\"warn\""), "{}", lines[0]);
    assert!(
        lines[0].contains("\"cause\":\"non_positive_price\""),
        "{}",
        lines[0]
    );
    assert!(
        lines[0].contains("\"listing\":\"inferred_from_vendor_span\""),
        "{}",
        lines[0]
    );
    let mut rows = answer(days[2], days[2]);
    rows[7].close = 0;
    rows[7].low = 0;
    let done = land_cash(
        &scratch.0,
        "KFINTECH",
        (days[2], days[2]),
        pull::vendor::Granularity::Minute1,
        rows,
    );
    assert_eq!(whys(&done, "non_positive_price").len(), 1);
    assert!(
        whys(&done, "non_positive_price")[0].contains("row 7:"),
        "{:?}",
        done.failures
    );
    assert_eq!(identity_lines(sink, "KFINTECH").len(), 2);

    // PATANJALI: a clean day rung and clean minutes. Nothing to say.
    let rows = days.iter().copied().map(day_candle).collect();
    let done = land_cash(
        &scratch.0,
        "PATANJALI",
        (from, to),
        pull::vendor::Granularity::Day1,
        rows,
    );
    assert!(done.failures.is_empty());
    let done = land_cash(
        &scratch.0,
        "PATANJALI",
        (from, to),
        pull::vendor::Granularity::Minute1,
        answer(from, to),
    );
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert!(
        identity_lines(sink, "PATANJALI").is_empty(),
        "{:?}",
        identity_lines(sink, "PATANJALI")
    );

    // UNITDSPR: minutes and no day rung. They land, unjudged, and it is said.
    let done = land_cash(
        &scratch.0,
        "UNITDSPR",
        (from, to),
        pull::vendor::Granularity::Minute1,
        answer(from, to),
    );
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    let lines = identity_lines(sink, "UNITDSPR");
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("\"level\":\"info\""), "{}", lines[0]);
    assert!(lines[0].contains("\"no_day_rung\":1875"), "{}", lines[0]);
}

/// RADICO: a day month that will not open and one zero minute (R4-17 repair
/// 1). One warn names why the rung would not open, and the zero is still
/// refused, with its own warn; no `info` line claims the rung was read. The
/// symbol is used by no other test in this binary.
#[test]
fn a_day_rung_that_will_not_open_is_named_on_the_log_beside_its_refusals() {
    let sink = log_sink();
    let scratch = Scratch::new("identity-log-unread");
    let (from, to, _) = week();
    unreadable_day_month(&scratch.0, "RADICO", b"not a bar file");
    let mut rows = answer(from, to);
    rows[9].open = 0;
    let done = land_cash(
        &scratch.0,
        "RADICO",
        (from, to),
        pull::vendor::Granularity::Minute1,
        rows,
    );
    assert_eq!(
        whys(&done, "non_positive_price").len(),
        1,
        "{:?}",
        done.failures
    );
    let lines = identity_lines(sink, "RADICO");
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(
        lines.iter().all(|l| l.contains("\"level\":\"warn\"")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("the day rung would not open")
                && l.contains("\"why\":")
                && l.contains("\"no_day_rung\":1874")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("\"cause\":\"non_positive_price\"") && l.contains("\"rows\":1")),
        "{lines:?}"
    );
}
