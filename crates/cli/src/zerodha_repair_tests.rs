//! `cli zerodha-repair` on invented stores: the read-only plan, the table of
//! every series, the repair from another store's bars, its idempotence, and
//! misuse. D-3350, WP-13c; DB-N20, DB-N22. Every price is invented.

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

/// A store root that removes itself.
struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-cli-zrepair-{}-{}",
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

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// Every minute the calendar of record owes on `from..=to`, invented.
fn answer(from: Day, to: Day) -> Vec<RawRow> {
    let mut rows = Vec::new();
    for number in from.days_from_epoch()..=to.days_from_epoch() {
        let kind = pull::session::with_recorded_window(
            i64::from(number),
            pull::calendar::sourced_kind_of(i64::from(number)),
        );
        let pull::calendar::DayKind::Open(session) = kind else {
            continue;
        };
        for minute in (0..1_440_u16).filter(|m| session.expects(*m)) {
            let base = 2_000_000 + i64::from(minute % 89) * 7;
            rows.push(RawRow {
                timestamp: i64::from(number) * 86_400 - IST_OFFSET_SECS + i64::from(minute) * 60,
                open: base,
                high: base + 30,
                low: base - 30,
                close: base + 3,
                volume: 0,
                open_interest: None,
            });
        }
    }
    rows
}

/// Lands an ordinary Zerodha minute answer for `symbol` under `root`.
fn land(root: &Path, symbol: &str, from: Day, to: Day) {
    let request = pull::fetch::BarRequest {
        instrument_id: String::new(),
        listing: pull::vendor::Listing::Index,
        window: Window::new(from, to).unwrap(),
        granularity: pull::vendor::Granularity::Minute1,
    };
    let plan = pull::ingest::Plan {
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
    };
    let done = pull::ingest::from_window(
        &RawWindow {
            rows: answer(from, to),
        },
        symbol,
        "fixture",
        root,
        plan,
    );
    assert!(done.failures.is_empty(), "{:#?}", done.failures);
}

/// THE PLAN NAMES THE LEADING HOLE AS A REPAIR'S AND WRITES NOTHING.
#[test]
fn the_plan_names_the_hole_and_the_request_that_fills_it_and_writes_nothing() {
    let store = Root::new();
    // NOVEMBER HELD, so December is not the series' first month.
    land(&store.0, "HOLEIDX", day(2020, 11, 27), day(2020, 11, 27));
    land(&store.0, "HOLEIDX", day(2020, 12, 16), day(2020, 12, 31));
    let before = fs::read_dir(store.0.join("audit"))
        .map(Iterator::count)
        .ok();
    let (code, out) = store.run(&["HOLEIDX", "2020-12-01", "2020-12-31"]);
    assert_eq!(code, FAILED, "a month needs a repair: {out}");
    assert!(
        out.contains("ZERODHA REPAIR PLAN (read-only; nothing is written)"),
        "{out}"
    );
    assert!(out.contains("2020-12  generation 0"), "{out}");
    assert!(out.contains("11 of 22 sessions held"), "{out}");
    assert!(
        out.contains("2020-12-01  front         only a versioned repair fills it"),
        "{out}"
    );
    assert!(
        out.contains("POST /pull/repair  target=indices&vendor=zerodha&granularity=1min&cash_identity=zerodha_nse_checked&member=HOLEIDX&from=2020-12-01&to=2020-12-31&repair_reason=<why>"),
        "{out}"
    );
    assert!(!store.0.join("bar-revisions-v1").exists());
    assert_eq!(
        fs::read_dir(store.0.join("audit"))
            .map(Iterator::count)
            .ok(),
        before
    );

    // A WHOLE MONTH NEEDS NOTHING.
    let (code, out) = store.run(&["HOLEIDX", "2020-12-16", "2020-12-31"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("0 month(s) need a versioned repair"), "{out}");
}

/// EVERY SERIES, ONE LINE EACH: the measurement WP-13c was asked for.
#[test]
fn every_series_is_counted_in_one_table() {
    let store = Root::new();
    land(&store.0, "AIDX", day(2020, 11, 27), day(2020, 11, 27));
    land(&store.0, "AIDX", day(2020, 12, 16), day(2020, 12, 31));
    land(&store.0, "BIDX", day(2020, 12, 1), day(2020, 12, 31));
    // CIDX's first held month is December: the sessions before its first bar
    // may be a listing, so they are counted apart and never planned.
    land(&store.0, "CIDX", day(2020, 12, 16), day(2020, 12, 31));
    let (code, out) = store.run(&["ALL", "2020-12-01", "2020-12-31"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains(": 3 series under"), "{out}");
    let row = |name: &str| {
        out.lines()
            .find(|line| line.trim_start().starts_with(name))
            .unwrap_or_else(|| panic!("{name}: {out}"))
            .split_whitespace()
            .skip(1)
            .map(|cell| cell.parse::<u64>().unwrap())
            .collect::<Vec<u64>>()
    };
    assert_eq!(row("AIDX"), [22, 11, 11, 0, 0, 0, 0, 1]);
    assert_eq!(row("BIDX"), [22, 22, 0, 0, 0, 0, 0, 0]);
    assert_eq!(row("CIDX"), [22, 11, 0, 0, 0, 0, 11, 0]);
    assert_eq!(row("TOTAL"), [66, 44, 11, 0, 0, 0, 11]);
}

/// THE REPAIR FROM ANOTHER STORE'S BARS: a new generation, journaled, and a
/// second run writes nothing.
#[test]
fn a_repair_from_another_stores_bars_fills_the_hole_once() {
    let store = Root::new();
    let source = Root::new();
    land(&store.0, "FIXIDX", day(2020, 12, 16), day(2020, 12, 31));
    land(&source.0, "FIXIDX", day(2020, 12, 1), day(2020, 12, 31));
    let source_root = source.0.display().to_string();
    let arguments = [
        "FIXIDX",
        "2020-12-01",
        "2020-12-31",
        "--from-store",
        source_root.as_str(),
        "--reason",
        "fixture repair",
    ];
    let (code, out) = store.run(&arguments);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("2020-12  offered   8250  written   4125  census rows  8  failures 0"),
        "{out}"
    );
    let journal = fs::read_to_string(store.0.join("audit").join("zerodha.repairs")).unwrap();
    assert!(
        journal
            .contains("repair NSE-INDEX-FIXIDX 1min 2020-12 v=1 from_generation=0 to_generation=1"),
        "{journal}"
    );
    assert!(journal.contains("reason=fixture_repair"), "{journal}");

    let (code, out) = store.run(&["FIXIDX", "2020-12-01", "2020-12-31"]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("2020-12  generation 1    8250 bars  22 of 22 sessions held"),
        "{out}"
    );

    let (code, out) = store.run(&arguments);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("written      0"), "{out}");
    assert_eq!(
        fs::read_to_string(store.0.join("audit").join("zerodha.repairs")).unwrap(),
        journal,
        "a second run journals nothing"
    );
}

/// MISUSE IS REFUSED BEFORE ANYTHING IS OPENED.
#[test]
fn misuse_is_refused_by_name() {
    let store = Root::new();
    for (arguments, why) in [
        (&["NIFTY", "2020-12-01"][..], "it was given 2"),
        (
            &["NIFTY", "2020-12", "2020-12-31"][..],
            "is not a YYYY-MM-DD day",
        ),
        (
            &["NIFTY", "2020-12-01", "2020-12-31", "--segment", "FNO"][..],
            "INDEX or CASH",
        ),
        (
            &["NIFTY", "2020-12-01", "2020-12-31", "--from-store", "/x"][..],
            "go together",
        ),
        (
            &[
                "ALL",
                "2020-12-01",
                "2020-12-31",
                "--from-store",
                "/x",
                "--reason",
                "r",
            ][..],
            "never ALL",
        ),
        (
            &["NIFTY", "2000-01-01", "2020-12-31"][..],
            "one run answers at most",
        ),
        (
            &[
                "NIFTY",
                "2020-12-01",
                "2020-12-31",
                "--rung",
                "5min",
                "--from-store",
                "/x",
                "--reason",
                "r",
            ][..],
            "is derived from the minutes",
        ),
    ] {
        let (code, out) = store.run(arguments);
        assert_eq!(code, MISUSED, "{arguments:?}: {out}");
        assert!(out.contains(why), "{arguments:?}: {out}");
        assert!(out.contains(USAGE), "{out}");
    }
}
