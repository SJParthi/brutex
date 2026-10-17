//! `cli series-identity` driven through `command_with` on stores built by the
//! landing door. Every price is invented: a level of 10,000.00 and impostors
//! at twice and thirty-seven times it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use pull::fetch::{RawRow, RawWindow};
use pull::session::{Day, IST_OFFSET_SECS, Window};

use super::{USAGE, command_with};
use crate::{FAILED, MISUSED, OK};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-cli-identity-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn run(&self, arguments: &[&str]) -> (u8, String) {
        let mut out = String::new();
        let root = self.0.clone();
        let code = command_with(arguments, &move || Ok(root.clone()), &mut out);
        (code, out)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const LEVEL: i64 = 1_000_000;

fn candle(day: Day, minute: i64, times: i64) -> RawRow {
    let base = (LEVEL + (minute % 97) * 13 + i64::from(day.day())) * times;
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

fn land(root: &Path, symbol: &str, granularity: pull::vendor::Granularity, rows: Vec<RawRow>) {
    let day_of = |row: &RawRow| {
        Day::from_days(u32::try_from((row.timestamp + IST_OFFSET_SECS).div_euclid(86_400)).unwrap())
            .unwrap()
    };
    let request = pull::fetch::BarRequest {
        instrument_id: String::new(),
        listing: pull::vendor::Listing::Index,
        window: Window::new(day_of(&rows[0]), day_of(rows.last().unwrap())).unwrap(),
        granularity,
    };
    let sent = rows.len();
    let done = pull::ingest::from_window(
        &RawWindow { rows },
        symbol,
        "IDENTITY CLI TEST",
        root,
        pull::ingest::Plan {
            calendar: pull::calendar::Runtime::default(),
            cash_schedule: None,
            columns: pull::csv::Columns::Gdfl,
            request: &request,
            encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
            scale: pull::vendor::PriceScale::Paisa,
            vendor: brutex_core::vendor::Vendor::Zerodha,
            exchange: "NSE",
            segment: "INDEX",
            contract: None,
        },
    );
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(done.bars_committed, sent);
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// Every file under `root` with its length: what a read-only verb leaves be.
fn listing(root: &Path) -> Vec<(PathBuf, u64)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap().filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push((path.clone(), fs::metadata(&path).unwrap().len()));
            }
        }
    }
    out.sort();
    out
}

/// Every file under the store's `bars/` with its length: what this verb never
/// changes.
fn bars(root: &Path) -> Vec<(PathBuf, u64)> {
    if root.join("bars").is_dir() {
        listing(&root.join("bars"))
    } else {
        Vec::new()
    }
}

/// Every quarantine plan written under the store's audit directory.
fn plans(root: &Path) -> Vec<(PathBuf, u64)> {
    let dir = root.join("audit").join("series-identity");
    if dir.is_dir() {
        listing(&dir)
    } else {
        Vec::new()
    }
}

/// A store as the operator's is: minutes that landed before this screen
/// existed (here: before the day rung did), then the day rung.
///
/// `NIFTY`: five sessions of January 2024, eight minutes of the third at
/// twice the level, and one session of March 2021 at thirty-seven times it;
/// its day rung starts with the January week. `BANKNIFTY`: the same week,
/// clean. `NIFTYINFRA`: a day rung and no minutes at all.
fn store() -> (Root, Vec<Day>) {
    let root = Root::new();
    let week = sessions(day(2024, 1, 1), day(2024, 1, 5));
    let days: Vec<Day> = week.iter().map(|(d, _)| *d).collect();
    assert_eq!(days.len(), 5);
    let early = sessions(day(2021, 3, 1), day(2021, 3, 1));
    assert_eq!(early.len(), 1);
    let minute = pull::vendor::Granularity::Minute1;
    land(
        &root.0,
        "NIFTY",
        minute,
        early[0]
            .1
            .iter()
            .map(|m| candle(early[0].0, *m, 37))
            .collect(),
    );
    let mut rows = Vec::new();
    for (on, minutes) in &week {
        for m in minutes {
            let foreign = *on == days[2] && (765..=772).contains(m);
            rows.push(candle(*on, *m, if foreign { 2 } else { 1 }));
        }
    }
    land(&root.0, "NIFTY", minute, rows);
    land(
        &root.0,
        "BANKNIFTY",
        minute,
        week.iter()
            .flat_map(|(on, minutes)| minutes.iter().map(|m| candle(*on, *m, 1)))
            .collect(),
    );
    for symbol in ["NIFTY", "BANKNIFTY", "NIFTYINFRA"] {
        land(
            &root.0,
            symbol,
            pull::vendor::Granularity::Day1,
            days.iter().copied().map(day_candle).collect(),
        );
    }
    (root, days)
}

#[test]
fn arguments_that_name_no_series_are_misused_before_anything_is_opened() {
    let root = Root::new();
    for bad in [
        &["nifty lower"][..],
        &["NIFTY", "--segment", "FNO"],
        &["NIFTY", "--feed", "nobody"],
        &["NIFTY", "--segment"],
        &["NIFTY", "--window", "x"],
    ] {
        let (code, out) = root.run(bad);
        assert_eq!(code, MISUSED, "{bad:?}: {out}");
        assert!(out.contains(USAGE), "{bad:?}: {out}");
    }
    let mut out = String::new();
    let code = command_with(&[], &|| Ok(PathBuf::new()), &mut out);
    assert_eq!(code, MISUSED);
    assert!(out.contains("name an instrument, or ALL"), "{out}");
}

#[test]
fn a_store_root_that_cannot_be_named_fails_out_loud() {
    let mut out = String::new();
    let code = command_with(&["NIFTY"], &|| Err("no store root".to_owned()), &mut out);
    assert_eq!(code, FAILED);
    assert!(
        out.contains("SERIES IDENTITY REFUSED: no store root"),
        "{out}"
    );
    let root = Root::new();
    let (code, out) = root.run(&["ALL"]);
    assert_eq!(code, FAILED);
    assert!(out.contains("cannot list"), "{out}");
}

#[test]
fn a_series_holding_another_series_minutes_is_listed_by_month_day_and_cause() {
    let (root, days) = store();
    let before = bars(&root.0);
    let (code, out) = root.run(&["NIFTY"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("read-only of bars; writes only its quarantine plan"),
        "{out}"
    );
    assert!(
        out.contains("listing: no_listing; listed on unknown"),
        "{out}"
    );
    assert!(out.contains("zerodha NSE-INDEX-NIFTY"), "{out}");
    assert!(
        out.contains("1day: 1 months 2024-01..2024-01; first served day 2024-01-01"),
        "{out}"
    );
    assert!(out.contains("1min: 2 months 2021-03..2024-01"), "{out}");
    assert!(
        out.contains("minutes 2250: held 1867, before_first_day_bar 375, foreign_level 8, suspect_level 0, unjudged 0"),
        "{out}"
    );
    assert!(
        out.contains("2021-03: 375 of 375 minutes refused, 0 suspect"),
        "{out}"
    );
    assert!(
        out.contains("2021-03-01 before_first_day_bar 375 minute(s)"),
        "{out}"
    );
    assert!(
        out.contains("2024-01: 8 of 1875 minutes refused, 0 suspect"),
        "{out}"
    );
    assert!(
        out.contains(&format!("{} foreign_level 8 minute(s)", days[2])),
        "{out}"
    );
    assert!(
        out.contains("QUARANTINE NEEDED: 2 month(s) hold 383 minute(s) a landing refuses today: 2021-03 2024-01"),
        "{out}"
    );
    assert_eq!(
        bars(&root.0),
        before,
        "a read-only verb changed the store's bars"
    );
    assert_eq!(plans(&root.0).len(), 1, "one quarantine plan");
}

#[test]
fn a_clean_series_passes_and_a_series_the_store_lacks_holds_nothing() {
    let (root, _) = store();
    let (code, out) = root.run(&["BANKNIFTY", "--segment", "INDEX", "--feed", "zerodha"]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("minutes 1875: held 1875, before_first_day_bar 0, foreign_level 0"),
        "{out}"
    );
    assert!(
        out.contains("no stored bar would be refused at landing"),
        "{out}"
    );
    // A series with nothing on disk: nothing to refuse, and it says none.
    let (code, out) = root.run(&["NIFTYIT"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("1day: none; first served day none"), "{out}");
    assert!(out.contains("1min: none"), "{out}");
    // A feed that holds nothing for it.
    let (code, out) = root.run(&["NIFTY", "--feed", "gdfl"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("gdfl NSE-INDEX-NIFTY"), "{out}");
}

#[test]
fn every_series_is_one_line_and_a_day_rung_with_no_minutes_is_named() {
    let (root, _) = store();
    let before = bars(&root.0);
    let (code, out) = root.run(&["ALL"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("zerodha NSE/INDEX: 3 series"), "{out}");
    let line = |name: &str| {
        out.lines()
            .find(|l| l.trim_start().starts_with(name))
            .unwrap_or_else(|| panic!("no line for {name}:\n{out}"))
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        line("NIFTY "),
        [
            "NIFTY",
            "2250",
            "375",
            "8",
            "0",
            "0",
            "2",
            "2024-01",
            "2021-03",
            "0",
            "-",
            "-",
            "-",
            "0",
            "no_listing"
        ]
    );
    assert_eq!(
        line("BANKNIFTY"),
        [
            "BANKNIFTY",
            "1875",
            "0",
            "0",
            "0",
            "0",
            "0",
            "2024-01",
            "2024-01",
            "0",
            "-",
            "-",
            "-",
            "0",
            "no_listing"
        ]
    );
    assert!(
        out.contains("TOTAL: minutes 4125, before_first_day_bar 375, foreign_level 8, suspect_level 0, unjudged 0"),
        "{out}"
    );
    assert!(
        out.contains(
            "DAY RUNG AND NO MINUTE RUNG (1): NIFTYINFRA (1day 1 months 2024-01..2024-01)"
        ),
        "{out}"
    );
    assert_eq!(
        bars(&root.0),
        before,
        "a read-only verb changed the store's bars"
    );
    let written = plans(&root.0);
    assert_eq!(written.len(), 1, "one plan for the segment");
    assert!(
        written[0]
            .0
            .to_string_lossy()
            .contains("zerodha-INDEX-ALL-")
    );
    // A segment that holds only clean series passes.
    let clean = Root::new();
    let week = sessions(day(2024, 1, 1), day(2024, 1, 5));
    land(
        &clean.0,
        "BANKNIFTY",
        pull::vendor::Granularity::Minute1,
        week.iter()
            .flat_map(|(on, minutes)| minutes.iter().map(|m| candle(*on, *m, 1)))
            .collect(),
    );
    let (code, out) = clean.run(&["ALL"]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("DAY RUNG AND NO MINUTE RUNG (0): none"),
        "{out}"
    );
}

// ---------------------------------------------------------------------------
// R4-17: in-session jumps, measured causes and the quarantine plan. Every
// master, listing date and price is invented; PIIND is named only so
// `brutex_core::universe::nse_isin` gives it the ISIN its listing is keyed by.
// ---------------------------------------------------------------------------

/// A stored bar exactly as the vendor row would have been, written straight
/// to the store the way months landed before the R4-17 screen existed.
fn as_bar(row: &RawRow) -> store::format::Bar {
    store::format::Bar {
        ts_micros: row.timestamp * 1_000_000,
        open: row.open,
        high: row.high,
        low: row.low,
        close: row.close,
        volume: row.volume,
        open_interest: i64::MIN,
    }
}

/// Write `bars` (one month, ascending) under `vendor`'s CASH `symbol`, bypassing
/// every landing screen: the store as an older build left it.
fn seed(
    root: &Path,
    vendor: brutex_core::vendor::Vendor,
    symbol: &str,
    tf: store::path::Timeframe,
    bars: &[store::format::Bar],
) {
    let first = bars[0].ts_micros.div_euclid(1_000_000) + IST_OFFSET_SECS;
    let on = Day::from_days(u32::try_from(first.div_euclid(86_400)).unwrap()).unwrap();
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor,
        exchange: "NSE",
        segment: "CASH",
        symbol,
        contract: None,
        timeframe: tf,
        month: on.year_month().unwrap(),
        file: store::path::FileKind::Bars,
    })
    .unwrap();
    let id = pull::ingest::repair_plan::symbol_id(symbol);
    let mut file = store::file::BarFile::open_or_create(root, path, id).unwrap();
    file.append(bars).unwrap();
}

fn day_bar(on: Day, low: i64, high: i64) -> store::format::Bar {
    as_bar(&RawRow {
        low,
        high,
        open: low,
        close: high,
        ..day_candle(on)
    })
}

/// Seconds from 1980-01-01, as the NSE master states a lifecycle date.
fn nse_seconds(on: Day) -> u64 {
    u64::from(on.days_from_epoch() - day(1980, 1, 1).days_from_epoch()) * 86_400
}

fn install_master(root: &Path, symbol: &str, listed: Day) {
    use std::io::Write as _;
    let isin = brutex_core::universe::nse_isin(symbol).unwrap();
    let csv = format!(
        "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn,ListgDt,RmvlDt,RadmssnDt\n7000,{symbol},EQ,{},1,{},0,0\n",
        isin.as_str(),
        nse_seconds(listed)
    );
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(csv.as_bytes()).unwrap();
    let masters = root.join(pull::listing::MASTER_DIR);
    fs::create_dir_all(&masters).unwrap();
    pull::cash_session_cache::install(&masters, day(2026, 8, 3), &encoder.finish().unwrap())
        .unwrap();
}

/// The fixture week's sessions and every honest minute of them.
fn week_minutes(times: impl Fn(Day, i64) -> (i64, i64)) -> (Vec<Day>, Vec<store::format::Bar>) {
    let week = sessions(day(2024, 1, 1), day(2024, 1, 5));
    let days = week.iter().map(|(d, _)| *d).collect();
    let minutes = week
        .iter()
        .flat_map(|(on, minutes)| {
            let times = &times;
            minutes.iter().map(move |m| {
                let (num, den) = times(*on, *m);
                let honest = candle(*on, *m, 1);
                let scale = |p: i64| p * num / den;
                as_bar(&RawRow {
                    open: scale(honest.open),
                    high: scale(honest.high),
                    low: scale(honest.low),
                    close: scale(honest.close),
                    ..honest
                })
            })
        })
        .collect();
    (days, minutes)
}

/// RT-DATA-02 b: a clean CASH store passes, and a copy seeded with one block of
/// minutes at another price basis — under a day bar wide enough to hold both,
/// so only the run can see them — fails, counting each jump in its month.
#[test]
fn the_store_scan_counts_in_session_jumps_and_fails_on_a_seeded_copy() {
    let wide = |on: Day| day_bar(on, LEVEL * 8_990 / 82_165 - 1_000, LEVEL + 2_000);
    let clean = Root::new();
    let (days, minutes) = week_minutes(|_, _| (1, 1));
    seed(
        &clean.0,
        brutex_core::vendor::Vendor::Zerodha,
        "PIIND",
        store::path::Timeframe::DAY_1,
        &days.iter().copied().map(wide).collect::<Vec<_>>(),
    );
    seed(
        &clean.0,
        brutex_core::vendor::Vendor::Zerodha,
        "PIIND",
        store::path::Timeframe::MINUTE_1,
        &minutes,
    );
    let (code, out) = clean.run(&["PIIND", "--segment", "CASH"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("in_session_jump 0"), "{out}");
    // 1,875 minutes in five sessions: every one but each session's first is judged.
    assert!(out.contains("in-session jumps: 1870 judged"), "{out}");
    assert!(out.contains("listing: inferred_from_vendor_span"), "{out}");
    assert!(out.contains("no quarantine plan: nothing flagged"), "{out}");
    assert!(plans(&clean.0).is_empty());

    let seeded = Root::new();
    let (days, minutes) = week_minutes(|on, m| {
        if on == day(2024, 1, 3) && (600..=609).contains(&m) {
            (8_990, 82_165)
        } else {
            (1, 1)
        }
    });
    seed(
        &seeded.0,
        brutex_core::vendor::Vendor::Zerodha,
        "PIIND",
        store::path::Timeframe::DAY_1,
        &days.iter().copied().map(wide).collect::<Vec<_>>(),
    );
    seed(
        &seeded.0,
        brutex_core::vendor::Vendor::Zerodha,
        "PIIND",
        store::path::Timeframe::MINUTE_1,
        &minutes,
    );
    let before = bars(&seeded.0);
    let (code, out) = seeded.run(&["PIIND", "--segment", "CASH"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("in_session_jump 10"), "{out}");
    assert!(
        out.contains("2024-01: 10 of 1875 minutes refused, 0 suspect, 10 in-session jump(s)"),
        "{out}"
    );
    assert!(
        out.contains("2024-01-03 in_session_jump 10 minute(s)"),
        "{out}"
    );
    assert!(out.contains("10 beyond JUMP_BPS 3000"), "{out}");
    assert!(
        out.contains("QUARANTINE NEEDED: 1 month(s) hold 10 minute(s)"),
        "{out}"
    );
    assert_eq!(bars(&seeded.0), before, "no bar written");
    // ALL over the segment fails the same way and names the jumps in its row.
    let (code, out) = seeded.run(&["ALL", "--segment", "CASH"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("in_session_jump 10, day bars refused 0"),
        "{out}"
    );
    let (code, out) = clean.run(&["ALL", "--segment", "CASH"]);
    assert_eq!(code, OK, "{out}");
}

/// Z6, the PIIND quarantine plan: every flagged bar is given a measured cause
/// — another basis GDFL does not hold, a minute GDFL vouches for beside an
/// adjusted day bar, a zero day bar, bars before the listing — and the plan is
/// written before any bar is: none is, and a rerun writes nothing new.
#[test]
fn piind_minutes_are_named_by_cause_before_any_write() {
    let root = Root::new();
    let symbol = "PIIND";
    let (days, minutes) = week_minutes(|on, m| {
        if on == day(2024, 1, 3) && (600..=609).contains(&m) {
            (8_990, 82_165)
        } else {
            (1, 1)
        }
    });
    install_master(&root.0, symbol, days[1]);
    let mut day_bars: Vec<_> = days
        .iter()
        .map(|on| day_bar(*on, LEVEL - 1_000, LEVEL + 2_000))
        .collect();
    // The fourth session's day bar adjusted to half the level; the fifth's zero.
    day_bars[3] = day_bar(days[3], LEVEL / 2 - 1_000, LEVEL / 2 + 2_000);
    day_bars[4] = day_bar(days[4], 0, 0);
    seed(
        &root.0,
        brutex_core::vendor::Vendor::Zerodha,
        symbol,
        store::path::Timeframe::DAY_1,
        &day_bars,
    );
    seed(
        &root.0,
        brutex_core::vendor::Vendor::Zerodha,
        symbol,
        store::path::Timeframe::MINUTE_1,
        &minutes,
    );
    // GDFL's one-second bars of the third and fourth sessions, at the series'
    // own basis: one second per minute, so each folds to that minute.
    let honest: Vec<_> = week_minutes(|_, _| (1, 1))
        .1
        .into_iter()
        .filter(|bar| {
            let local = bar.ts_micros.div_euclid(1_000_000) + IST_OFFSET_SECS;
            let on = Day::from_days(u32::try_from(local.div_euclid(86_400)).unwrap()).unwrap();
            on == days[2] || on == days[3]
        })
        .collect();
    seed(
        &root.0,
        brutex_core::vendor::Vendor::Gdfl,
        symbol,
        store::path::Timeframe::SECOND_1,
        &honest,
    );
    let before = bars(&root.0);
    let (code, out) = root.run(&[symbol, "--segment", "CASH"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("listing: nse_security_master; listed on 2024-01-02"),
        "{out}"
    );
    assert!(
        out.contains("measured causes: zero_price 1, before_listing 376, before_first_day_bar 0, basis_mismatch 375, foreign_level 10, no_gdfl_witness 0"),
        "{out}"
    );
    assert!(
        out.contains("1day 2024-01: 2 of 5 day bars refused"),
        "{out}"
    );
    assert!(out.contains("QUARANTINE NEEDED: 2 day bar(s)"), "{out}");
    assert_eq!(bars(&root.0), before, "no bar written");
    let written = plans(&root.0);
    assert_eq!(written.len(), 1, "{out}");
    let plan = fs::read_to_string(&written[0].0).unwrap();
    assert!(plan.starts_with("brutex series-identity quarantine plan v1\nfeed\tzerodha\nsegment\tCASH\nscope\tPIIND\n"), "{plan}");
    assert!(plan.contains("bars_written\t0\n"), "{plan}");
    let row = |cause: &str| {
        plan.lines()
            .find(|l| l.split('\t').nth(4) == Some(cause))
            .unwrap_or_else(|| panic!("no {cause} row:\n{plan}"))
            .split('\t')
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        row("basis_mismatch")[..7],
        [
            "NSE-CASH-PIIND",
            "1min",
            "2024-01",
            "timestamps",
            "basis_mismatch",
            "label",
            "375"
        ]
    );
    assert_eq!(
        row("foreign_level")[..7],
        [
            "NSE-CASH-PIIND",
            "1min",
            "2024-01",
            "timestamps",
            "foreign_level",
            "quarantine",
            "10"
        ]
    );
    assert_eq!(row("foreign_level")[7].split(',').count(), 10);
    assert_eq!(
        row("zero_price")[..7],
        [
            "NSE-CASH-PIIND",
            "1day",
            "2024-01",
            "timestamps",
            "zero_price",
            "quarantine",
            "1"
        ]
    );
    // A rerun finds its plan already there: the same path, nothing new.
    let (_, again) = root.run(&[symbol, "--segment", "CASH"]);
    assert!(
        again.contains(&written[0].0.display().to_string()),
        "{again}"
    );
    assert_eq!(plans(&root.0), written);
}

/// A month every bar of which is refused for one cause is planned whole.
#[test]
fn a_month_refused_whole_is_planned_whole() {
    let (root, _) = store();
    let (_, out) = root.run(&["NIFTY"]);
    let written = plans(&root.0);
    assert_eq!(written.len(), 1, "{out}");
    let plan = fs::read_to_string(&written[0].0).unwrap();
    assert!(
        plan.contains(
            "NSE-INDEX-NIFTY\t1min\t2021-03\twhole_month\tbefore_first_day_bar\tquarantine\t375\t\n"
        ),
        "{plan}"
    );
    assert!(
        plan.contains(
            "NSE-INDEX-NIFTY\t1min\t2024-01\ttimestamps\tno_gdfl_witness\tquarantine\t8\t"
        ),
        "{plan}"
    );
}
