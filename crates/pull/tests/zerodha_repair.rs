//! ZERODHA MONTH HOLES: PER-ROW REFUSAL, THE LEDGER OF WHAT THE STORE
//! REFUSED, AND THE VERSIONED REPAIR (WP-13c; DB-N20, DB-N21, DB-N22,
//! DB-N23; D-3350, D-3351).
//!
//! Drives `pull::ingest::from_window` and `pull::ingest::repair_window` the way
//! `api::server::land_bodies_observed` does, over scratch stores, and reads
//! back the bars every reader opens, the derived rungs, the gap ledger
//! `<store>/audit/zerodha.gaps` and the repair journal
//! `<store>/audit/zerodha.repairs`.
//!
//! Every candle is invented; no price here can be mistaken for an index level.

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
        let dir = std::env::temp_dir().join(format!("brutex-REPAIR-{tag}-{}", std::process::id()));
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
    let base = 1_000_000 + (minute % 97) * 13 + i64::from(day.day());
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

fn request(from: Day, to: Day) -> pull::fetch::BarRequest {
    pull::fetch::BarRequest {
        instrument_id: String::new(),
        listing: pull::vendor::Listing::Index,
        window: Window::new(from, to).unwrap(),
        granularity: pull::vendor::Granularity::Minute1,
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

/// The days of `from..=to` the calendar of record traded, with the minutes
/// each owes (its recorded windows for a special session).
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

/// Every owed minute of `from..=to`, one invented candle each.
fn answer(from: Day, to: Day) -> Vec<RawRow> {
    sessions(from, to)
        .into_iter()
        .flat_map(|(day, minutes)| minutes.into_iter().map(move |m| candle(day, m)))
        .collect()
}

const SYMBOL: &str = "NIFTY";

fn symbol_id() -> u32 {
    u32::try_from(brutex_core::universe::fnv1a(SYMBOL) & u64::from(u32::MAX)).unwrap()
}

fn month_path(tf: Timeframe, month: (u16, u8)) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: brutex_core::vendor::Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        symbol: SYMBOL,
        contract: None,
        timeframe: tf,
        month: YearMonth::new(month.0, month.1).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// Every bar of one month, as every reader opens it.
fn month_bars(root: &Path, tf: Timeframe, month: (u16, u8)) -> Vec<store::format::Bar> {
    let file =
        store::file::BarFile::open_existing(root, month_path(tf, month), symbol_id()).unwrap();
    (0..file.header().n_valid)
        .map(|i| file.read_record(i).unwrap())
        .collect()
}

/// The distinct IST days the bars fall on, in order.
fn days_of(bars: &[store::format::Bar]) -> Vec<String> {
    let mut days: Vec<String> = Vec::new();
    for bar in bars {
        let number = (bar.ts_micros / 1_000_000 + IST_OFFSET_SECS).div_euclid(86_400);
        let day = Day::from_days(u32::try_from(number).unwrap())
            .unwrap()
            .to_string();
        if days.last() != Some(&day) {
            days.push(day);
        }
    }
    days
}

fn read(root: &Path, name: &str) -> String {
    fs::read_to_string(root.join("audit").join(name)).unwrap_or_default()
}

fn gap_lines(root: &Path) -> Vec<String> {
    read(root, "zerodha.gaps")
        .lines()
        .filter(|l| l.starts_with("gap "))
        .map(str::to_owned)
        .collect()
}

fn land(root: &Path, from: Day, to: Day, rows: Vec<RawRow>) -> pull::ingest::Ingested {
    let req = request(from, to);
    pull::ingest::from_window(
        &RawWindow { rows },
        SYMBOL,
        "ZERODHA TEST",
        root,
        plan(&req),
    )
}

fn repair(root: &Path, from: Day, to: Day, rows: Vec<RawRow>) -> pull::ingest::Ingested {
    let req = request(from, to);
    pull::ingest::repair_window(
        &RawWindow { rows },
        SYMBOL,
        "ZERODHA TEST",
        root,
        plan(&req),
        "WP-13c fixture: fill the leading hole",
    )
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// **ONE BAD ROW IS REFUSED BY NAME AND THE REST OF THE ANSWER LANDS (DB-N23,
/// D-3351).** A whole-month answer for November 2020 — the Muhurat month —
/// carrying four bad rows: a stamp before the epoch, a price that does not fit
/// the paisa grid, an off-grid candle and two candles at one stamp that
/// disagree. Each refused the whole answer before. Now each is one named
/// failure, every other minute lands, and the ledger says which days are short
/// because of a refusal.
#[test]
fn one_bad_row_is_refused_by_name_and_the_rest_of_the_answer_lands() {
    let scratch = Scratch::new("PER-ROW");
    let (from, to) = (day(2020, 11, 1), day(2020, 11, 30));
    let mut rows = answer(from, to);
    let whole = rows.len();
    // A STAMP THE CALENDAR CANNOT JUDGE, first so the answer stays ordered.
    rows.insert(
        0,
        RawRow {
            timestamp: -86_400 * 400,
            ..rows[0]
        },
    );
    // AN OFF-GRID CANDLE at 09:15:30 on 2020-11-03, beside the held 09:15:00.
    let nov3 = rows
        .iter()
        .position(|r| *r == candle(day(2020, 11, 3), 555))
        .unwrap();
    rows.insert(
        nov3 + 1,
        RawRow {
            timestamp: rows[nov3].timestamp + 30,
            ..rows[nov3]
        },
    );
    // TWO CANDLES AT 10:00 ON 2020-11-04 THAT DISAGREE: both refused.
    let nov4 = rows
        .iter()
        .position(|r| *r == candle(day(2020, 11, 4), 600))
        .unwrap();
    rows.insert(
        nov4 + 1,
        RawRow {
            close: rows[nov4].close + 1,
            ..rows[nov4]
        },
    );
    let done = land(&scratch.0, from, to, rows);

    // EVERY OTHER MINUTE LANDED: the whole answer less the one 10:00 minute.
    assert_eq!(done.bars_committed, whole - 1, "{:#?}", done.failures);
    let held = month_bars(&scratch.0, Timeframe::MINUTE_1, (2020, 11));
    assert_eq!(held.len(), whole - 1);
    assert_eq!(
        days_of(&held).len(),
        sessions(from, to).len(),
        "every session holds"
    );

    // EACH REFUSAL NAMED, ONCE: the before-epoch stamp, the off-grid candle,
    // the conflicting pair.
    let named = |code: &str| {
        done.failures
            .iter()
            .filter(|f| f.why.contains(code))
            .count()
    };
    assert_eq!(named("timestamp_refused"), 1, "{:#?}", done.failures);
    assert_eq!(named("off_grid_candle"), 1, "{:#?}", done.failures);
    assert_eq!(named("conflicting_candles"), 1, "{:#?}", done.failures);
    let conflict = done
        .failures
        .iter()
        .find(|f| f.why.contains("conflicting_candles"))
        .unwrap();
    assert!(
        conflict.why.contains("2 row(s) refused"),
        "{}",
        conflict.why
    );
    assert_eq!(
        conflict.day.map(|d| d.to_string()).as_deref(),
        Some("2020-11-04")
    );

    // THE LEDGER: 2020-11-04 is short one minute, refused; 2020-11-03 is not
    // short at all — its 09:15:00 candle holds the minute the off-grid row was
    // refused from.
    let lines = gap_lines(&scratch.0);
    assert_eq!(lines.len(), 1, "{lines:#?}");
    assert!(
        lines[0].starts_with(
            "gap NSE-INDEX-NIFTY 2020-11-04 v=1 rung=1min owed=375 held=374 missing=1 first_missing=10:00 runs=1 longest=1 longest_at=10:00 why=refused_day run="
        ),
        "{}",
        lines[0]
    );

    // A PRICE OFF THE PAISA GRID, in a rupee answer: that row only.
    let scratch = Scratch::new("PER-ROW-PRICE");
    let (from, to) = (day(2020, 10, 1), day(2020, 10, 31));
    let mut rows = answer(from, to);
    let whole = rows.len();
    rows[3].high = i64::MAX / 10;
    let req = request(from, to);
    let rupees = pull::ingest::Plan {
        scale: pull::vendor::PriceScale::Rupees,
        ..plan(&req)
    };
    let done = pull::ingest::from_window(
        &RawWindow { rows },
        SYMBOL,
        "ZERODHA TEST",
        &scratch.0,
        rupees,
    );
    assert_eq!(done.bars_committed, whole - 1, "{:#?}", done.failures);
    let price = done
        .failures
        .iter()
        .find(|f| f.why.contains("price_off_grid"))
        .expect("the price is named");
    assert!(price.why.contains("row 3: high"), "{}", price.why);
}

/// **A MONTH THE STORE REFUSES IS IN THE LEDGER, DAY BY DAY (DB-N22,
/// D-3351).** The DB-N20 shape: December 2020 already holds the 16th on, the
/// way the earlier answer's refusal and the next one's landing left it. The
/// whole month offered again is refused by the append-only store — and each of
/// the eleven sessions it lacks is now a `refused_front` line, owed with its
/// reason, where the ledger said nothing because the vendor's rows were whole.
#[test]
fn a_month_the_store_refuses_names_each_missing_session_in_the_ledger() {
    let scratch = Scratch::new("REFUSED-FRONT");
    let done = land(
        &scratch.0,
        day(2020, 12, 16),
        day(2020, 12, 31),
        answer(day(2020, 12, 16), day(2020, 12, 31)),
    );
    assert!(done.failures.is_empty(), "{:#?}", done.failures);
    let (from, to) = (day(2020, 12, 1), day(2020, 12, 31));
    let again = land(&scratch.0, from, to, answer(from, to));
    assert_eq!(again.bars_committed, 0);
    assert!(
        again
            .failures
            .iter()
            .any(|f| f.why.contains("2020-12:") && f.why.contains("this month only")),
        "{:#?}",
        again.failures
    );
    let lines = gap_lines(&scratch.0);
    let front: Vec<&String> = lines
        .iter()
        .filter(|l| l.contains("why=refused_front"))
        .collect();
    assert_eq!(front.len(), 11, "{lines:#?}");
    assert!(
        front[0].starts_with(
            "gap NSE-INDEX-NIFTY 2020-12-01 v=1 rung=1min owed=375 held=0 missing=375 first_missing=09:15 runs=1 longest=375 longest_at=09:15 why=refused_front run="
        ),
        "{}",
        front[0]
    );
    assert!(front[10].contains(" 2020-12-15 "), "{}", front[10]);
    assert_eq!(
        lines.len(),
        11,
        "no day the store holds is named: {lines:#?}"
    );
}

/// **THE VERSIONED REPAIR (DB-N20, DB-N22, D-3350).** The same December: the
/// repair door publishes a NEW generation holding the 16th on and the eleven
/// sessions before it, and every reader opens it — the 1min month, each
/// derived rung, the census row. The original month is byte for byte what it
/// was. The repair is journaled with its reason; a second repair finds nothing
/// new; and an ordinary pull of the month afterwards is not refused.
#[test]
fn a_repair_fills_the_leading_hole_as_a_new_generation_every_reader_opens() {
    let scratch = Scratch::new("REPAIR");
    let held_days = answer(day(2020, 12, 16), day(2020, 12, 31));
    let held = held_days.len();
    land(&scratch.0, day(2020, 12, 16), day(2020, 12, 31), held_days);
    let original = month_path(Timeframe::MINUTE_1, (2020, 12)).to_path_buf(&scratch.0);
    let before = fs::read(&original).unwrap();
    let five_before = month_bars(&scratch.0, Timeframe::MINUTE_5, (2020, 12));

    let (from, to) = (day(2020, 12, 1), day(2020, 12, 31));
    let whole = answer(from, to);
    // THE ORDINARY PULL IS REFUSED AND THE LEDGER SAYS SO, day by day.
    let refused = land(&scratch.0, from, to, whole.clone());
    assert_eq!(refused.bars_committed, 0);
    assert_eq!(gap_lines(&scratch.0).len(), 11);
    let done = repair(&scratch.0, from, to, whole.clone());
    assert_eq!(
        done.bars_committed,
        whole.len() - held,
        "{:#?}",
        done.failures
    );
    assert!(done.failures.is_empty(), "{:#?}", done.failures);
    // AND A `repaired` LINE SUPERSEDES EACH `refused_front` ONE (D-3350): the
    // last line of a key wins.
    let lines = gap_lines(&scratch.0);
    assert_eq!(lines.len(), 22, "{lines:#?}");
    assert!(lines[..11].iter().all(|l| l.contains("why=refused_front")));
    assert!(
        lines[11].starts_with(
            "gap NSE-INDEX-NIFTY 2020-12-01 v=1 rung=1min owed=375 held=375 missing=0 first_missing=- runs=0 longest=0 longest_at=- why=repaired run="
        ),
        "{}",
        lines[11]
    );
    assert!(lines[11..].iter().all(|l| l.contains("why=repaired")));

    // EVERY READER: the minute month, and every derived rung, now hold all
    // 22 sessions of December.
    let minutes = month_bars(&scratch.0, Timeframe::MINUTE_1, (2020, 12));
    assert_eq!(minutes.len(), whole.len());
    assert_eq!(
        days_of(&minutes).first().map(String::as_str),
        Some("2020-12-01")
    );
    assert_eq!(days_of(&minutes).len(), 22);
    let five = month_bars(&scratch.0, Timeframe::MINUTE_5, (2020, 12));
    assert_eq!(days_of(&five).len(), 22);
    assert_eq!(five.len(), 22 * 75);
    assert_eq!(
        &five[11 * 75..],
        &five_before[..],
        "the held derived bars are kept as they were"
    );
    let served =
        store::generation::served(&scratch.0, month_path(Timeframe::MINUTE_1, (2020, 12))).unwrap();
    assert_eq!(served.ordinal(), 1);

    // NEVER IN PLACE.
    assert_eq!(fs::read(&original).unwrap(), before);

    // THE CENSUS COUNTS THE NEW GENERATION: one row per rung was recorded.
    assert_eq!(done.counted, 8, "the minute month and seven derived rungs");

    // JOURNALED WITH ITS REASON.
    let journal = read(&scratch.0, "zerodha.repairs");
    let minute_line = journal
        .lines()
        .find(|l| l.starts_with("repair NSE-INDEX-NIFTY 1min 2020-12 "))
        .unwrap_or_else(|| panic!("{journal}"));
    assert!(
        minute_line.contains(&format!(
            "from_generation=0 to_generation=1 held={held} added={} rows={}",
            whole.len() - held,
            whole.len()
        )),
        "{minute_line}"
    );
    assert!(
        minute_line.contains("reason=WP-13c_fixture:_fill_the_leading_hole"),
        "{minute_line}"
    );
    assert_eq!(
        journal.lines().filter(|l| l.starts_with("repair ")).count(),
        8,
        "{journal}"
    );

    // IDEMPOTENT: nothing new, nothing written, no new journal line.
    let again = repair(&scratch.0, from, to, whole.clone());
    assert_eq!(again.bars_committed, 0, "{:#?}", again.failures);
    assert!(again.failures.is_empty(), "{:#?}", again.failures);
    assert_eq!(read(&scratch.0, "zerodha.repairs"), journal);

    // AN ORDINARY PULL OF THE REPAIRED MONTH is answered as held, not refused.
    let ordinary = land(&scratch.0, from, to, whole);
    assert_eq!(ordinary.bars_committed, 0);
    assert!(
        !ordinary.failures.iter().any(|f| f.why.contains("2020-12:")),
        "{:#?}",
        ordinary.failures
    );
}

/// **A REPAIR NEVER CHANGES A HELD BAR (D-3350).** An offered candle at a held
/// minute with another close refuses the month by name, writes nothing, and
/// journals the refusal.
#[test]
fn a_repair_that_would_change_a_held_bar_is_refused_and_writes_nothing() {
    let scratch = Scratch::new("REPAIR-REFUSED");
    land(
        &scratch.0,
        day(2020, 12, 16),
        day(2020, 12, 31),
        answer(day(2020, 12, 16), day(2020, 12, 31)),
    );
    let (from, to) = (day(2020, 12, 1), day(2020, 12, 31));
    let mut whole = answer(from, to);
    let at = whole
        .iter()
        .position(|r| *r == candle(day(2020, 12, 16), 555))
        .unwrap();
    whole[at].close += 3;
    let done = repair(&scratch.0, from, to, whole);
    assert_eq!(done.bars_committed, 0);
    let refused = done
        .failures
        .iter()
        .find(|f| f.why.contains("2020-12:"))
        .unwrap_or_else(|| panic!("{:#?}", done.failures));
    assert!(
        refused.why.contains("never changes a held bar"),
        "{}",
        refused.why
    );
    assert!(!scratch.0.join("bar-revisions-v1").exists());
    assert!(
        read(&scratch.0, "zerodha.repairs")
            .starts_with("refused NSE-INDEX-NIFTY 1min 2020-12 v=1 why=")
    );
}

/// **THE MUHURATS OF 2018 AND 2019 HAVE RECORDED WINDOWS, AND THEIR MINUTES
/// LAND INSIDE THEM (DB-N21, D-3319).** The build that ran judged 2018-11-07
/// and 2019-10-27 against regular hours and dropped any Muhurat minute as
/// after the close. The calendar of record now holds 17:30–18:29 and
/// 18:15–19:14: a whole-month answer lands all sixty minutes of each, and a
/// minute past the window is dropped as after the close, counted, never kept.
#[test]
fn the_2018_and_2019_muhurat_minutes_land_inside_their_recorded_windows() {
    for (muhurat, month, window) in [
        (day(2018, 11, 7), (2018, 11), (1_050_i64, 1_109_i64)),
        (day(2019, 10, 27), (2019, 10), (1_095, 1_154)),
    ] {
        let scratch = Scratch::new(&format!("MUHURAT-{muhurat}"));
        let owed = sessions(muhurat, muhurat);
        assert_eq!(owed.len(), 1, "{muhurat} is a session of record");
        assert_eq!(owed[0].1.len(), 60, "{muhurat} owes sixty minutes");
        assert_eq!(
            (owed[0].1[0], owed[0].1[59]),
            window,
            "{muhurat}'s recorded window"
        );
        let from = Day::new(month.0, month.1, 1).unwrap();
        let to = from.end_of_month();
        let mut rows = answer(from, to);
        let after = rows
            .iter()
            .position(|r| *r == candle(muhurat, window.1))
            .unwrap();
        rows.insert(after + 1, candle(muhurat, window.1 + 1));
        let done = land(&scratch.0, from, to, rows);
        assert!(done.failures.is_empty(), "{muhurat}: {:#?}", done.failures);
        let bars = month_bars(&scratch.0, Timeframe::MINUTE_1, month);
        let midnight = i64::from(muhurat.days_from_epoch()) * 86_400 - IST_OFFSET_SECS;
        let evening: Vec<i64> = bars
            .iter()
            .map(|b| b.ts_micros / 1_000_000 - midnight)
            .filter(|s| (0..86_400).contains(s))
            .map(|s| s / 60)
            .collect();
        assert_eq!(evening.len(), 60, "{muhurat}: the session's sixty minutes");
        assert_eq!((evening[0], evening[59]), window);
        assert_eq!(
            done.census
                .of(pull::session::DropReason::AtOrAfterSessionClose),
            1,
            "{muhurat}: the minute past the window is dropped and counted"
        );
    }
}

/// **A 60-DAY ANSWER WITH ONE MUHURAT MINUTE LANDS EVERY OTHER ROW (DB-N20,
/// D-3351).** The exact request the first pass of 6 Oct 2026 lost whole:
/// 2020-10-17..2020-12-15, Zerodha's 60-day minute cap, holding the
/// 2020-11-14 Muhurat. The answer carries every owed minute of the three
/// months it touches, the Muhurat's sixty inside its recorded window, and one
/// Muhurat minute past that window. Before D-3344 that one minute refused all
/// 60 days and the next request opened December at the 16th; now the minute
/// past the window is dropped and counted, and every other row lands in its
/// own month — October from the 17th, all of November, December to the 15th.
#[test]
fn a_sixty_day_answer_with_one_muhurat_minute_lands_every_other_row() {
    let scratch = Scratch::new("SIXTY-DAY-MUHURAT");
    let (from, to) = (day(2020, 10, 17), day(2020, 12, 15));
    assert_eq!(
        to.days_from_epoch() - from.days_from_epoch() + 1,
        60,
        "one request at the cap"
    );
    let muhurat = day(2020, 11, 14);
    let mut rows = answer(from, to);
    let owed = rows.len();
    let window = sessions(muhurat, muhurat);
    assert_eq!(window.len(), 1, "the Muhurat is a session of record");
    let last_minute = *window[0].1.last().unwrap();
    assert_eq!(
        window[0].1.len(),
        60,
        "its recorded window owes sixty minutes"
    );
    let at = rows
        .iter()
        .position(|r| *r == candle(muhurat, last_minute))
        .unwrap();
    rows.insert(at + 1, candle(muhurat, last_minute + 1));

    let done = land(&scratch.0, from, to, rows);
    assert!(done.failures.is_empty(), "{:#?}", done.failures);
    assert_eq!(done.bars_committed, owed, "every owed minute landed");
    assert_eq!(
        done.census
            .of(pull::session::DropReason::AtOrAfterSessionClose),
        1,
        "the one minute past the Muhurat's window is dropped and counted"
    );
    for (month, first, last, sessions_held) in [
        ((2020, 10), "2020-10-19", "2020-10-30", 10),
        ((2020, 11), "2020-11-02", "2020-11-27", 20),
        ((2020, 12), "2020-12-01", "2020-12-15", 11),
    ] {
        let bars = month_bars(&scratch.0, Timeframe::MINUTE_1, month);
        let days = days_of(&bars);
        assert_eq!(days.first().map(String::as_str), Some(first), "{month:?}");
        assert_eq!(days.last().map(String::as_str), Some(last), "{month:?}");
        assert_eq!(days.len(), sessions_held, "{month:?}");
    }
    let november = month_bars(&scratch.0, Timeframe::MINUTE_1, (2020, 11));
    assert!(
        days_of(&november).contains(&"2020-11-14".to_owned()),
        "the Muhurat's sixty minutes are held"
    );
    // AND THE NEXT REQUEST, December's own, appends the rest of December: the
    // month was never opened part-way past what an append can extend.
    let (rest_from, rest_to) = (day(2020, 12, 16), day(2020, 12, 31));
    let rest = land(&scratch.0, rest_from, rest_to, answer(rest_from, rest_to));
    assert!(rest.failures.is_empty(), "{:#?}", rest.failures);
    assert_eq!(
        days_of(&month_bars(&scratch.0, Timeframe::MINUTE_1, (2020, 12))).len(),
        22,
        "all of December 2020"
    );
}
