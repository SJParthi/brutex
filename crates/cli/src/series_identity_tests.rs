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

use super::{USAGE, command_with, percentile, within, worker_count};
use crate::{FAILED, MISUSED, OK};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// The commit a stamped build names in its plan: invented, forty hex digits.
const STAMP: &str = "0123456789abcdef0123456789abcdef01234567";

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
        let code = command_with(arguments, &move || Ok(root.clone()), Some(STAMP), &mut out);
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
        // R4-17 repair 2: a flag first is never the instrument.
        &["--dry-run"],
        &["--segment", "CASH"],
        &["-X"],
    ] {
        let (code, out) = root.run(bad);
        assert_eq!(code, MISUSED, "{bad:?}: {out}");
        assert!(out.contains(USAGE), "{bad:?}: {out}");
    }
    let (_, out) = root.run(&["--dry-run"]);
    assert!(
        out.contains("`--dry-run` is a flag: name an instrument, or ALL, before any flag"),
        "{out}"
    );
    assert!(
        !root.0.join("audit").exists(),
        "a misuse opens and writes nothing"
    );
    let mut out = String::new();
    let code = command_with(&[], &|| Ok(PathBuf::new()), Some(STAMP), &mut out);
    assert_eq!(code, MISUSED);
    assert!(out.contains("name an instrument, or ALL"), "{out}");
}

#[test]
fn a_store_root_that_cannot_be_named_fails_out_loud() {
    let mut out = String::new();
    let code = command_with(
        &["NIFTY"],
        &|| Err("no store root".to_owned()),
        Some(STAMP),
        &mut out,
    );
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
    assert!(
        !out.contains("day bar(s) a landing refuses"),
        "no day bar is refused: {out}"
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
    seed_in(root, (vendor, "CASH"), symbol, tf, bars);
}

/// [`seed`] under any segment.
fn seed_in(
    root: &Path,
    (vendor, segment): (brutex_core::vendor::Vendor, &str),
    symbol: &str,
    tf: store::path::Timeframe,
    bars: &[store::format::Bar],
) {
    let first = bars[0].ts_micros.div_euclid(1_000_000) + IST_OFFSET_SECS;
    let on = Day::from_days(u32::try_from(first.div_euclid(86_400)).unwrap()).unwrap();
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor,
        exchange: "NSE",
        segment,
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
    assert!(
        out.contains("measured causes: zero_price 0, before_listing 0, before_first_day_bar 0, basis_mismatch 0, foreign_level 0, no_gdfl_witness 10"),
        "{out}"
    );
    let (code, out) = clean.run(&["ALL", "--segment", "CASH"]);
    assert_eq!(code, OK, "{out}");
}

/// Z6, the PIIND quarantine plan: every flagged bar is given a measured cause
/// — another basis GDFL does not hold, a minute GDFL vouches for beside an
/// adjusted day bar, a zero day bar, bars before the listing — and the plan is
/// written before any bar is: none is, and a rerun writes nothing new.
/// PIIND's store as the scan meets it: Zerodha minutes with one block at
/// another basis on the third session, a day bar adjusted to half the level
/// on the fourth and a zero one on the fifth, a master listing it on the
/// second, and GDFL's one-second bars of the third and fourth sessions at
/// the series' own basis.
fn piind_store(symbol: &str) -> Root {
    let root = Root::new();
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
    root
}

#[test]
fn piind_minutes_are_named_by_cause_before_any_write() {
    let symbol = "PIIND";
    let root = piind_store(symbol);
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
    assert!(
        plan.starts_with(&format!(
            "brutex series-identity quarantine plan v2\ncommit\t{STAMP}\nfeed\tzerodha\nsegment\tCASH\nscope\tPIIND\n"
        )),
        "{plan}"
    );
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

/// What the scan meets at the edges: a suspect minute nothing vouches for
/// (labelled, not quarantined), a GDFL minute whose close is zero (no witness
/// to match: `foreign_level`), a share with one minute (nothing judged, so an
/// empty distribution), and a month that will not open (named, and the verb
/// fails).
#[test]
fn the_scan_names_every_edge_it_cannot_judge() {
    let root = Root::new();
    let vendor = brutex_core::vendor::Vendor::Zerodha;
    let (days, _) = week_minutes(|_, _| (1, 1));
    let day_bars: Vec<_> = days
        .iter()
        .map(|on| day_bar(*on, LEVEL - 1_000, LEVEL + 2_000))
        .collect();
    // MAZDOCK: the first session's 10:00 at 105% (suspect), the second's
    // 10:00 at twice the level, whose GDFL second closes at zero.
    let (_, minutes) = week_minutes(|on, m| match (on, m) {
        (on, 600) if on == days[0] => (105, 100),
        (on, 600) if on == days[1] => (2, 1),
        _ => (1, 1),
    });
    seed(
        &root.0,
        vendor,
        "MAZDOCK",
        store::path::Timeframe::DAY_1,
        &day_bars,
    );
    seed(
        &root.0,
        vendor,
        "MAZDOCK",
        store::path::Timeframe::MINUTE_1,
        &minutes,
    );
    let zero_close = store::format::Bar {
        low: 0,
        close: 0,
        ..minutes[375 + 600 - 555]
    };
    seed(
        &root.0,
        brutex_core::vendor::Vendor::Gdfl,
        "MAZDOCK",
        store::path::Timeframe::SECOND_1,
        &[zero_close],
    );
    let (code, out) = root.run(&["MAZDOCK", "--segment", "CASH"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("measured causes: zero_price 0, before_listing 0, before_first_day_bar 0, basis_mismatch 0, foreign_level 1, no_gdfl_witness 1"),
        "{out}"
    );
    let plan = fs::read_to_string(&plans(&root.0)[0].0).unwrap();
    assert!(plan.contains("\tno_gdfl_witness\tlabel\t1\t"), "{plan}");
    assert!(plan.contains("\tforeign_level\tquarantine\t1\t"), "{plan}");

    // IRFC: one stored minute, so no run and nothing judged.
    seed(
        &root.0,
        vendor,
        "IRFC",
        store::path::Timeframe::MINUTE_1,
        &minutes[..1],
    );
    let (_, out) = root.run(&["IRFC", "--segment", "CASH"]);
    assert!(
        out.contains("in-session jumps: 0 judged; p50 0 p99 0 p999 0 max 0 bps"),
        "{out}"
    );

    // KFINTECH: a minute month that is not a bar file.
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor,
        exchange: "NSE",
        segment: "CASH",
        symbol: "KFINTECH",
        contract: None,
        timeframe: store::path::Timeframe::MINUTE_1,
        month: store::path::YearMonth::new(2024, 1).unwrap(),
        file: store::path::FileKind::Bars,
    })
    .unwrap()
    .to_path_buf(&root.0);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"not a bar file").unwrap();
    let (code, out) = root.run(&["KFINTECH", "--segment", "CASH"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("2024-01: NOT READ: 1min:"), "{out}");
    let (code, out) = root.run(&["ALL", "--segment", "CASH"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("NOT READ: 1 month(s)"), "{out}");
}

/// A plan that cannot be written is said, and the verb fails: its directory
/// is a file, its staging name is taken by a directory, or its final name is.
#[test]
fn a_plan_that_cannot_be_written_fails_out_loud() {
    let (blocked, _) = store();
    fs::create_dir_all(blocked.0.join("audit")).unwrap();
    fs::write(blocked.0.join("audit").join("series-identity"), b"a file").unwrap();
    for arguments in [&["NIFTY"][..], &["ALL"]] {
        let (code, out) = blocked.run(arguments);
        assert_eq!(code, FAILED, "{out}");
        assert!(out.contains("QUARANTINE PLAN NOT WRITTEN: "), "{out}");
    }

    let (staging, _) = store();
    let dir = staging.0.join("audit").join("series-identity");
    fs::create_dir_all(dir.join(format!(".series-identity.{}.staging", std::process::id())))
        .unwrap();
    let (code, out) = staging.run(&["NIFTY"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("QUARANTINE PLAN NOT WRITTEN: staging_not_regular"),
        "{out}"
    );

    // The final name taken by a directory: the rename is refused by name.
    let (first, _) = store();
    let (_, out) = first.run(&["NIFTY"]);
    let name = plans(&first.0)[0].0.file_name().unwrap().to_owned();
    let (taken, _) = store();
    fs::create_dir_all(taken.0.join("audit").join("series-identity").join(&name)).unwrap();
    let (code, again) = taken.run(&["NIFTY"]);
    assert_eq!(code, FAILED, "{again}");
    assert!(
        again.contains("QUARANTINE PLAN NOT WRITTEN: "),
        "{out}\n{again}"
    );
}

/// R4-17 repair 2: a plan is never written through a symbolic link. A linked
/// `audit/series-identity`, or a linked `audit`, is refused by name, the verb
/// fails, and nothing lands where the link points.
#[test]
fn a_plan_is_never_written_through_a_linked_directory() {
    use std::os::unix::fs::symlink;
    let outside = Root::new();
    for linked in [&["audit", "series-identity"][..], &["audit"]] {
        let (store_root, _) = store();
        let link = linked
            .iter()
            .fold(store_root.0.clone(), |path, part| path.join(part));
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        // The fixture's landing already made `audit/` (its gap ledger): set
        // it aside so the link takes its name.
        if link.exists() {
            fs::rename(&link, link.with_extension("aside")).unwrap();
        }
        symlink(&outside.0, &link).unwrap();
        let (code, out) = store_root.run(&["NIFTY"]);
        assert_eq!(code, FAILED, "{linked:?}: {out}");
        assert!(
            out.contains("QUARANTINE PLAN NOT WRITTEN: linked_plan_dir: "),
            "{linked:?}: {out}"
        );
        assert!(
            fs::read_dir(&outside.0).unwrap().next().is_none(),
            "{linked:?}: nothing is written outside the store"
        );
        // The dry run names the plan it would write and touches nothing.
        let (_, dry) = store_root.run(&["NIFTY", "--dry-run"]);
        assert!(dry.contains("--dry-run; nothing written"), "{dry}");
        assert!(fs::read_dir(&outside.0).unwrap().next().is_none());
    }
}

/// A series whose only refusal is a day bar fails on it, and a series whose
/// only flag is a suspect minute passes, with its label planned; when that
/// plan cannot be written the verb fails and says so.
#[test]
fn a_day_bar_alone_fails_and_a_suspect_minute_alone_passes_unless_its_plan_fails() {
    let vendor = brutex_core::vendor::Vendor::Zerodha;
    let root = Root::new();
    let (days, minutes) = week_minutes(|_, _| (1, 1));
    let mut day_bars: Vec<_> = days
        .iter()
        .map(|on| day_bar(*on, LEVEL - 1_000, LEVEL + 2_000))
        .collect();
    day_bars[2] = day_bar(days[2], 0, 0);
    seed(
        &root.0,
        vendor,
        "MAZDOCK",
        store::path::Timeframe::DAY_1,
        &day_bars,
    );
    seed(
        &root.0,
        vendor,
        "MAZDOCK",
        store::path::Timeframe::MINUTE_1,
        &minutes,
    );
    let (code, out) = root.run(&["MAZDOCK", "--segment", "CASH"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("QUARANTINE NEEDED: 1 day bar(s)"), "{out}");
    assert!(!out.contains("month(s) hold"), "{out}");
    assert!(
        out.contains("2024-01-03 non_positive_price 1 day bar(s)"),
        "{out}"
    );
    // ALL over the segment fails on the day bar alone, and totals it.
    let (code, all) = root.run(&["ALL", "--segment", "CASH"]);
    assert_eq!(code, FAILED, "{all}");
    assert!(all.contains("day bars refused 1"), "{all}");
    assert!(all.contains("measured causes: zero_price 1,"), "{all}");

    let suspect = |on: Day, m: i64| {
        if on == days[1] && m == 600 {
            (105, 100)
        } else {
            (1, 1)
        }
    };
    let (_, minutes) = week_minutes(suspect);
    let clean_days: Vec<_> = days
        .iter()
        .map(|on| day_bar(*on, LEVEL - 1_000, LEVEL + 2_000))
        .collect();
    for blocked in [false, true] {
        let root = Root::new();
        seed(
            &root.0,
            vendor,
            "IRFC",
            store::path::Timeframe::DAY_1,
            &clean_days,
        );
        seed(
            &root.0,
            vendor,
            "IRFC",
            store::path::Timeframe::MINUTE_1,
            &minutes,
        );
        if blocked {
            fs::create_dir_all(root.0.join("audit")).unwrap();
            fs::write(root.0.join("audit").join("series-identity"), b"a file").unwrap();
        }
        let (code, out) = root.run(&["IRFC", "--segment", "CASH"]);
        assert!(out.contains("suspect_level 1"), "{out}");
        if blocked {
            assert_eq!(code, FAILED, "{out}");
            assert!(out.contains("QUARANTINE PLAN NOT WRITTEN"), "{out}");
        } else {
            assert_eq!(code, OK, "{out}");
            assert!(
                out.contains("no stored bar would be refused at landing"),
                "{out}"
            );
            let plan = fs::read_to_string(&plans(&root.0)[0].0).unwrap();
            assert!(plan.contains("\tno_gdfl_witness\tlabel\t1\t"), "{plan}");
        }
    }
}

/// The witness tolerance is 100 bps of GDFL's close, inclusive, either way;
/// a witness at zero vouches for nothing.
#[test]
fn a_witness_vouches_within_its_tolerance_and_no_further() {
    for (close, vouched) in [
        (10_000, true),
        (10_050, true),
        (10_100, true),
        (10_101, false),
        (9_950, true),
        (9_900, true),
        (9_899, false),
        (1_000, false),
    ] {
        assert_eq!(within(close, 10_000), vouched, "{close}");
    }
    assert!(!within(5, 0));
    assert!(!within(5, -5));
}

/// The nearest-rank percentile of an ascending distribution.
#[test]
fn a_percentile_is_the_nearest_rank() {
    let sorted: Vec<i64> = (1..=100).map(|n| n * 10).collect();
    assert_eq!(percentile(&sorted, 500), 500);
    assert_eq!(percentile(&sorted, 990), 990);
    assert_eq!(percentile(&sorted, 999), 1_000);
    assert_eq!(percentile(&sorted, 1_000), 1_000);
    assert_eq!(percentile(&sorted, 1), 10, "the first rank is the smallest");
    assert_eq!(percentile(&[7], 500), 7);
    assert_eq!(percentile(&[3, 9], 500), 3);
    assert_eq!(percentile(&[], 990), 0);
}

/// `ALL` on one worker and on many prints the same bytes, exits the same and
/// writes the same plan: each series is judged alone and joined in name order.
#[test]
fn the_all_table_is_the_same_bytes_on_one_worker_and_on_many() {
    let (root, _) = store();
    let (one_code, one) = root.run(&["ALL", "--workers", "1"]);
    assert_eq!(one_code, FAILED, "{one}");
    let one_plans = plans(&root.0);
    assert_eq!(one_plans.len(), 1, "{one}");
    for workers in ["2", "3", "64"] {
        let (code, many) = root.run(&["ALL", "--workers", workers]);
        assert_eq!((code, &many), (one_code, &one), "--workers {workers}");
        assert_eq!(
            plans(&root.0),
            one_plans,
            "--workers {workers} wrote another plan"
        );
    }
    let (code, default) = root.run(&["ALL"]);
    assert_eq!((code, default), (one_code, one));
}

/// The worker count is the one named, else the machine's, never more than
/// one per series and never none; a count that is not a whole number above
/// zero is a misuse, refused before anything is opened.
#[test]
fn a_worker_count_is_named_or_the_machines_and_never_above_one_per_series() {
    assert_eq!(worker_count(Some(8), 3), 3);
    assert_eq!(worker_count(Some(2), 5), 2);
    assert_eq!(worker_count(Some(4), 0), 1);
    assert_eq!(worker_count(None, 1), 1);
    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    assert_eq!(worker_count(None, 10_000), cores.min(10_000));
    for (arguments, says) in [
        (&["ALL", "--workers", "0"][..], "a whole number above zero"),
        (&["ALL", "--workers", "x"], "a whole number above zero"),
        (&["ALL", "--workers", "-1"], "a whole number above zero"),
        (&["ALL", "--workers"], "takes a value"),
    ] {
        let mut out = String::new();
        let code = command_with(arguments, &|| Ok(PathBuf::new()), Some(STAMP), &mut out);
        assert_eq!(code, MISUSED, "{arguments:?}: {out}");
        assert!(out.contains(says) && out.contains(USAGE), "{out}");
    }
}

/// A series with no minute rung whose day month will not open fails `ALL`
/// and is named with it: its day months are judged too, so one that could
/// not be read is a failure, not a silent omission.
#[test]
fn a_day_month_that_will_not_open_fails_the_run_with_no_minute_rung() {
    let root = Root::new();
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor: brutex_core::vendor::Vendor::Zerodha,
        exchange: "NSE",
        segment: "CASH",
        symbol: "KFINTECH",
        contract: None,
        timeframe: store::path::Timeframe::DAY_1,
        month: store::path::YearMonth::new(2024, 1).unwrap(),
        file: store::path::FileKind::Bars,
    })
    .unwrap()
    .to_path_buf(&root.0);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"not a bar file").unwrap();
    let (code, out) = root.run(&["ALL", "--segment", "CASH"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains(
            "DAY RUNG AND NO MINUTE RUNG (1): KFINTECH (1day 1 months 2024-01..2024-01, NOT READ: 1 month(s))"
        ),
        "{out}"
    );
}

/// An index's zero is held at landing (D-4992): the scan labels it, plans it
/// as a label and passes, where a share's zero fails (above). A zero minute
/// under a day that holds zero is labelled too; a zero minute under a day of
/// ordinary range is still refused by that range.
#[test]
fn an_index_zero_is_labelled_and_passes_where_a_share_zero_fails() {
    let vendor = (brutex_core::vendor::Vendor::Zerodha, "INDEX");
    let root = Root::new();
    let (days, mut minutes) = week_minutes(|_, _| (1, 1));
    let mut day_bars: Vec<_> = days
        .iter()
        .map(|on| day_bar(*on, LEVEL - 1_000, LEVEL + 2_000))
        .collect();
    day_bars[2] = day_bar(days[2], 0, 0);
    seed_in(
        &root.0,
        vendor,
        "NIFTY50DIVPOINT",
        store::path::Timeframe::DAY_1,
        &day_bars,
    );
    let (code, out) = root.run(&["NIFTY50DIVPOINT"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("measured causes: zero_price 1,"), "{out}");
    assert!(
        out.contains("no stored bar would be refused at landing"),
        "{out}"
    );
    let plan = fs::read_to_string(&plans(&root.0)[0].0).unwrap();
    assert!(
        plan.contains(
            "NSE-INDEX-NIFTY50DIVPOINT\t1day\t2024-01\ttimestamps\tzero_price\tlabel\t1\t"
        ),
        "{plan}"
    );

    // Minutes: day 3 holds zero (its minutes are unjudged, and its zero
    // minute is labelled); day 4 has an ordinary range, so its zero minute is
    // refused by range.
    let other = Root::new();
    day_bars[2] = day_bar(days[2], LEVEL - 1_000, LEVEL + 2_000);
    day_bars[3] = day_bar(days[3], 0, LEVEL + 2_000);
    let zero = |bar: &mut store::format::Bar| {
        bar.open = 0;
        bar.high = 0;
        bar.low = 0;
        bar.close = 0;
    };
    zero(&mut minutes[3 * 375 + 100]);
    zero(&mut minutes[4 * 375 + 100]);
    seed_in(
        &other.0,
        vendor,
        "NIFTY50DIVPOINT",
        store::path::Timeframe::DAY_1,
        &day_bars,
    );
    seed_in(
        &other.0,
        vendor,
        "NIFTY50DIVPOINT",
        store::path::Timeframe::MINUTE_1,
        &minutes,
    );
    let (code, out) = other.run(&["NIFTY50DIVPOINT"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("2024-01-05 foreign_level 1 minute(s)"),
        "{out}"
    );
    assert!(
        out.contains("measured causes: zero_price 2, before_listing 0, before_first_day_bar 0, basis_mismatch 0, foreign_level 0, no_gdfl_witness 1"),
        "{out}"
    );
    let plan = fs::read_to_string(&plans(&other.0)[0].0).unwrap();
    assert!(
        plan.contains("\t1min\t2024-01\ttimestamps\tzero_price\tlabel\t1\t"),
        "{plan}"
    );
}

// ---------------------------------------------------------------------------
// R4-17 repair 1: the plan writer is a store writer, and a GDFL witness that
// will not open is named, never measured as absence.
// ---------------------------------------------------------------------------

/// **THE PLAN WRITER IS GATED AS A STORE WRITER (DB-B71).** `series-identity`
/// writes its plan under `audit/`, so an unstamped build is refused before
/// anything is read and no plan appears; `--dry-run` is its read-only form and
/// is not refused; `BRUTEX_ALLOW_UNSTAMPED_WRITE=1` lets it write a plan that
/// says `unstamped`; a stamped build's plan names its commit.
#[test]
fn an_unstamped_build_writes_no_plan_unless_allowed_and_every_plan_names_its_commit() {
    use crate::store_write::{root_with, writes_the_store};
    let symbol = "PIIND";
    let root = piind_store(symbol);
    let before = bars(&root.0);
    let owned = root.0.clone();
    let store = move || -> Result<PathBuf, String> { Ok(owned.clone()) };
    let gated = |arguments: &[&str], allow: Option<&str>, commit: Option<&str>| {
        let mut out = String::new();
        let code = command_with(
            arguments,
            &|| {
                root_with(
                    "series-identity",
                    arguments,
                    commit,
                    allow.map(std::ffi::OsStr::new),
                    &store,
                )
            },
            commit,
            &mut out,
        );
        (code, out)
    };
    let writing = [symbol, "--segment", "CASH"];
    assert!(writes_the_store("series-identity", &writing));
    // Unstamped, not allowed: refused before a byte is read, no plan.
    let (code, out) = gated(&writing, None, None);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("SERIES IDENTITY REFUSED: this build carries no commit stamp"),
        "{out}"
    );
    assert!(out.contains("no plan was written"), "{out}");
    assert!(!out.contains("measured causes"), "nothing was read: {out}");
    assert!(plans(&root.0).is_empty(), "{out}");
    assert!(!root.0.join("audit").exists(), "nothing created: {out}");
    // `--dry-run`, unstamped: the read-only form is not refused, the scan runs
    // and its verdict stands, and the plan is named and not written.
    let dry = [symbol, "--segment", "CASH", "--dry-run"];
    assert!(!writes_the_store("series-identity", &dry));
    let (code, out) = gated(&dry, None, None);
    assert_eq!(code, FAILED, "the same verdict: {out}");
    assert!(out.contains("--dry-run: writes nothing"), "{out}");
    assert!(out.contains("measured causes: zero_price 1,"), "{out}");
    assert!(
        out.contains("QUARANTINE PLAN NOT WRITTEN (--dry-run; nothing written): would be "),
        "{out}"
    );
    assert!(!root.0.join("audit").exists(), "nothing created: {out}");
    // Unstamped and allowed on purpose: the plan says `unstamped`.
    let (code, out) = gated(&writing, Some("1"), None);
    assert_eq!(code, FAILED, "{out}");
    let written = plans(&root.0);
    assert_eq!(written.len(), 1, "{out}");
    let unstamped = fs::read_to_string(&written[0].0).unwrap();
    assert!(
        unstamped.starts_with("brutex series-identity quarantine plan v2\ncommit\tunstamped\n"),
        "{unstamped}"
    );
    // Stamped: a second plan, its own content, naming the commit.
    let (code, out) = gated(&writing, None, Some(STAMP));
    assert_eq!(code, FAILED, "{out}");
    let written = plans(&root.0);
    assert_eq!(written.len(), 2, "{out}");
    let stamped = written
        .iter()
        .map(|(path, _)| fs::read_to_string(path).unwrap())
        .find(|plan| plan.contains(&format!("\ncommit\t{STAMP}\n")))
        .expect("the stamped plan");
    assert_eq!(
        stamped.lines().skip(2).collect::<Vec<_>>(),
        unstamped.lines().skip(2).collect::<Vec<_>>(),
        "the same measurement under another commit"
    );
    assert_eq!(bars(&root.0), before, "no bar written");
    // The binary's own entry point hands the writer's gate and its stamp.
    let source = include_str!("series_identity.rs");
    let entry = source
        .split("pub(crate) fn command(")
        .nth(1)
        .and_then(|rest| rest.split("\n}\n").next())
        .expect("fn command");
    assert!(
        entry.contains("crate::store_write::root_for(\"series-identity\", arguments)"),
        "{entry}"
    );
    assert!(entry.contains("crate::commit_stamp()"), "{entry}");
    assert!(!entry.contains("crate::store_root"), "{entry}");
}

/// The GDFL 1s month of `symbol` in 2024-01 under `root`.
fn gdfl_month(root: &Path, symbol: &str) -> PathBuf {
    store::path::StorePath::new(store::path::PathParts {
        vendor: brutex_core::vendor::Vendor::Gdfl,
        exchange: "NSE",
        segment: "CASH",
        symbol,
        contract: None,
        timeframe: store::path::Timeframe::SECOND_1,
        month: store::path::YearMonth::new(2024, 1).unwrap(),
        file: store::path::FileKind::Bars,
    })
    .unwrap()
    .to_path_buf(root)
}

/// **A GDFL WITNESS MONTH THAT IS THERE AND WILL NOT OPEN IS NAMED, NEVER
/// MEASURED AS ABSENCE (Z6 repair 1).** PIIND's GDFL 1s month torn to half:
/// the minutes it would have vouched for are `gdfl_unread` with the action
/// `remeasure` — never `no_gdfl_witness`, whose `quarantine` would have been
/// read as a measurement — and the month is a `NOT READ` line that fails the
/// run, on one series and in `ALL`. A month GDFL does not hold at all stays
/// `no_gdfl_witness`.
#[test]
fn a_torn_gdfl_witness_month_is_named_and_fails_the_run_never_measured_as_absence() {
    let symbol = "PIIND";
    let root = piind_store(symbol);
    let path = gdfl_month(&root.0, symbol);
    let bytes = fs::read(&path).unwrap();
    fs::write(&path, &bytes[..bytes.len() / 2]).unwrap();
    let (code, out) = root.run(&[symbol, "--segment", "CASH"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("measured causes: zero_price 1, before_listing 376, before_first_day_bar 0, basis_mismatch 0, foreign_level 0, no_gdfl_witness 0, gdfl_unread 385"),
        "{out}"
    );
    assert!(
        out.contains("2024-01: NOT READ: gdfl 1s witness: "),
        "{out}"
    );
    let plan = fs::read_to_string(&plans(&root.0)[0].0).unwrap();
    assert!(plan.contains("\tgdfl_unread\tremeasure\t"), "{plan}");
    assert!(!plan.contains("\tno_gdfl_witness\t"), "{plan}");
    let (code, all) = root.run(&["ALL", "--segment", "CASH"]);
    assert_eq!(code, FAILED, "{all}");
    let line = all
        .lines()
        .find(|l| l.trim_start().starts_with(symbol))
        .unwrap_or_else(|| panic!("{all}"));
    assert!(line.ends_with("NOT READ: 1 month(s)"), "{line}");
    assert!(all.contains("gdfl_unread 385"), "{all}");
    // A month GDFL does not hold is absence, and only that.
    let other = piind_store(symbol);
    let held = gdfl_month(&other.0, symbol);
    let aside = held.with_extension("aside");
    fs::rename(&held, &aside).unwrap();
    let (code, out) = other.run(&[symbol, "--segment", "CASH"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("no_gdfl_witness 385, gdfl_unread 0"), "{out}");
    assert!(!out.contains("NOT READ"), "{out}");
}

/// **ONE EVENT PER RUN** (standing rule 4, gate 17): every run leaves exactly
/// one `cli.series_identity` record on `/logs` carrying what was scanned, its
/// exit, the commit, every measured cause's count, how many months would not
/// open and where its plan is — `warn` for a run that fails — and none per
/// bar or per series.
#[test]
fn every_run_leaves_one_event_with_its_counts_its_exit_and_its_plan() {
    use crate::ledger_all::tests::{counts, mark, says, sink};
    let from = mark();
    let symbol = "PIIND";
    let root = piind_store(symbol);
    let (code, out) = root.run(&[symbol, "--segment", "CASH"]);
    assert_eq!(code, FAILED, "{out}");
    let (code, all) = root.run(&["ALL", "--segment", "CASH", "--dry-run"]);
    assert_eq!(code, FAILED, "{all}");
    let sink = sink();
    let dir = sink
        .path()
        .parent()
        .expect("the sink writes its file inside a directory")
        .to_path_buf();
    let query = telemetry::Query::last(telemetry::MAX_LIMIT).from_target("cli.series_identity");
    let here = root.0.display().to_string();
    let mine: Vec<telemetry::Record> = telemetry::tail(&dir, sink.keep_files(), &query)
        .records
        .into_iter()
        .filter(|record| record.seq >= from && says(record, "plan", &here))
        .collect();
    assert_eq!(mine.len(), 2, "one per run: {mine:?}");
    let one = mine
        .iter()
        .find(|record| says(record, "scope", symbol))
        .expect("the one-series run");
    assert_eq!(one.level, telemetry::Level::Warn, "{one:?}");
    assert!(counts(one, "exit", u64::from(FAILED)), "{one:?}");
    assert!(counts(one, "unread_months", 0), "{one:?}");
    assert!(says(one, "commit", STAMP), "{one:?}");
    assert!(
        says(one, "feed", "zerodha") && says(one, "segment", "CASH"),
        "{one:?}"
    );
    assert!(
        says(
            one,
            "causes",
            "zero_price 1, before_listing 376, before_first_day_bar 0, basis_mismatch 375, foreign_level 10, no_gdfl_witness 0, gdfl_unread 0"
        ),
        "{one:?}"
    );
    assert!(says(one, "plan", ".plan"), "{one:?}");
    let every = mine
        .iter()
        .find(|record| says(record, "scope", "ALL"))
        .expect("the ALL run");
    assert!(says(every, "plan", "dry run, not written: "), "{every:?}");
    assert!(says(every, "causes", "zero_price 1,"), "{every:?}");
}
