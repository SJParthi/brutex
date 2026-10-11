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
    let before = listing(&root.0);
    let (code, out) = root.run(&["NIFTY"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("read-only; nothing is written"), "{out}");
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
        listing(&root.0),
        before,
        "a read-only verb changed the store"
    );
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
        out.contains("no stored minute would be refused at landing"),
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
    let before = listing(&root.0);
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
            "NIFTY", "2250", "375", "8", "0", "0", "2", "2024-01", "2021-03"
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
            "2024-01"
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
        listing(&root.0),
        before,
        "a read-only verb changed the store"
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
