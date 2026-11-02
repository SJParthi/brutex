//! BASE-COV-PULL: the public doors of `ingest` that only `api`'s tests walked
//! (Route B measures each crate by its own tests): the run counters, the
//! greeks door and the committed cash days. Every price is invented.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::gdfl_fixtures::scratch;
use crate::session::Day;
use store::format::{Bar, Greek};
use store::path::YearMonth;

fn month(year: u16, month: u8) -> YearMonth {
    YearMonth::new(year, month).unwrap()
}

/// 09:15 IST on `day`, in the store's epoch microseconds.
fn opening(day: Day) -> i64 {
    (i64::from(day.days_from_epoch()) * 86_400 + 13_500) * 1_000_000
}

fn bar(ts_micros: i64) -> Bar {
    Bar {
        ts_micros,
        open: 11_111,
        high: 11_111,
        low: 11_111,
        close: 11_111,
        volume: 1,
        open_interest: i64::MIN,
    }
}

/// Files `bars` under `symbol`'s cash month as the store files them.
fn file_cash(root: &Path, symbol: &str, at: YearMonth, bars: &[Bar]) {
    let identity = identify(symbol, "NSE", "CASH").unwrap();
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "CASH",
        symbol: identity.symbol.as_str(),
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: at,
        file: FileKind::Bars,
    })
    .unwrap();
    let mut file = BarFile::open_or_create(root, path, identity.symbol_id).unwrap();
    file.append(bars).unwrap();
}

/// A run's counters sum, its failures keep their order, and the first door
/// kind stands.
#[test]
fn a_chunks_counters_are_added_to_the_runs() {
    let mut run = Ingested {
        members: 1,
        rows_read: 2,
        bars_stored: 3,
        bars_committed: 4,
        rows_folded: 5,
        ..Ingested::default()
    };
    run.failures.push(Failure::new("ZQA", "first".to_owned()));
    let mut chunk = Ingested {
        members: 10,
        rows_read: 20,
        bars_stored: 30,
        bars_committed: 40,
        rows_folded: 50,
        ..Ingested::default()
    };
    chunk.failures.push(Failure::new("ZQB", "second".to_owned()));
    run.absorb(chunk);
    assert_eq!(
        (run.members, run.rows_read, run.bars_stored),
        (11, 22, 33)
    );
    assert_eq!((run.bars_committed, run.rows_folded), (44, 55));
    let order: Vec<&str> = run.failures.iter().map(|f| f.why.as_str()).collect();
    assert_eq!(order, ["first", "second"]);
    let same = run.clone();
    run.absorb(Ingested::default());
    assert_eq!(run, same, "an empty chunk changes nothing");
}

fn greek(ts_micros: i64) -> Greek {
    Greek {
        ts_micros,
        spot: 11_111,
        volatility: 0.25,
        delta: 0.5,
        gamma: 0.01,
        vega: 0.1,
        theta: -0.1,
        rho: 0.01,
        rate: 0.06,
    }
}

/// No rows open no file at all; a row lands under the bars' own identity; a
/// segment the store cannot path is refused by name.
#[test]
fn the_greeks_door_writes_what_it_is_given_and_nothing_when_given_nothing() {
    let root = scratch("cov-greeks");
    let at = month(2026, 8);
    let target = |segment: &'static str| GreekTarget {
        store_root: &root,
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment,
        symbol: "ZQOPT",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: at,
    };
    write_greeks(&[], target("FNO")).unwrap();
    assert!(!root.join("bars").exists(), "no rows, no file");
    let on = Day::new(2026, 8, 3).unwrap();
    write_greeks(&[greek(opening(on))], target("FNO")).unwrap();
    let held = root.join("bars/gdfl/NSE/FNO/ZQOPT");
    assert!(held.exists(), "the row is filed under its series");
    assert!(write_greeks(&[greek(opening(on))], target("../OUT")).is_err());
    assert!(write_greeks(&[], target("../OUT")).is_ok(), "nothing to path");
    std::fs::remove_dir_all(root).unwrap();
}

/// A month's cash days, kept only from the day the rule starts; a month with
/// no file passes over; months asked twice or out of order answer once, in
/// order. A store of nothing is no days, not a refusal.
#[test]
fn the_committed_cash_days_are_the_eligible_days_a_month_holds() {
    let root = scratch("cov-cash-days");
    let (july, aug) = (month(2026, 7), month(2026, 8));
    let before = Day::new(2026, 7, 31).unwrap();
    let (first, later) = (Day::new(2026, 8, 3).unwrap(), Day::new(2026, 8, 4).unwrap());
    file_cash(&root, "ZQCASH", july, &[bar(opening(before))]);
    file_cash(
        &root,
        "ZQCASH",
        aug,
        &[
            bar(opening(first)),
            bar(opening(first) + 60_000_000),
            bar(opening(later)),
        ],
    );
    let months = [month(2026, 9), aug, july, aug];
    let days = committed_cash_days(&root, Vendor::Gdfl, "ZQCASH", &months).unwrap();
    assert_eq!(days, [first, later]);
    assert_eq!(
        committed_cash_days(&root, Vendor::Gdfl, "ZQCASH", &[]).unwrap(),
        []
    );
    assert_eq!(
        committed_cash_days(&root, Vendor::Gdfl, "ZQNONE", &months).unwrap(),
        []
    );
    assert!(committed_cash_days(&root, Vendor::Gdfl, "NOT A SYMBOL", &months).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

/// A month's file that will not open is the month's refusal, by name: never
/// passed over as a month with no days.
#[test]
fn a_cash_month_that_will_not_open_is_refused_by_name() {
    let root = scratch("cov-cash-bad");
    let at = month(2026, 8);
    let first = Day::new(2026, 8, 3).unwrap();
    file_cash(&root, "ZQCASH", at, &[bar(opening(first))]);
    let file = root.join("bars/gdfl/NSE/CASH/ZQCASH/1min/2026-08.bars");
    std::fs::remove_file(&file).unwrap();
    std::fs::write(&file, b"cut").unwrap();
    let why = committed_cash_days(&root, Vendor::Gdfl, "ZQCASH", &[at]).unwrap_err();
    assert!(why.starts_with("ZQCASH 2026-08: committed cash source: "), "{why}");
    std::fs::remove_dir_all(root).unwrap();
}
