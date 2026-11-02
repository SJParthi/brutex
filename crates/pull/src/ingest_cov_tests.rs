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

/// Files `bars` under `symbol`'s month of `rung` as the store files them.
fn file_bars(
    root: &Path,
    (segment, symbol): (&str, &str),
    rung: Timeframe,
    at: YearMonth,
    bars: &[Bar],
) {
    let identity = identify(symbol, "NSE", segment).unwrap();
    let path = StorePath::new(parts_of(segment, identity.symbol.as_str(), rung, at)).unwrap();
    let mut file = BarFile::open_or_create(root, path, identity.symbol_id).unwrap();
    file.append(bars).unwrap();
}

fn parts_of<'a>(
    segment: &'a str,
    symbol: &'a str,
    timeframe: Timeframe,
    month: YearMonth,
) -> PathParts<'a> {
    PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment,
        symbol,
        contract: None,
        timeframe,
        month,
        file: FileKind::Bars,
    }
}

/// Files `bars` under `symbol`'s cash month as the store files them.
fn file_cash(root: &Path, symbol: &str, at: YearMonth, bars: &[Bar]) {
    file_bars(root, ("CASH", symbol), Timeframe::MINUTE_1, at, bars);
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
    chunk
        .failures
        .push(Failure::new("ZQB", "second".to_owned()));
    run.absorb(chunk);
    assert_eq!((run.members, run.rows_read, run.bars_stored), (11, 22, 33));
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
        provenance: 0,
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
    assert!(
        write_greeks(&[], target("../OUT")).is_ok(),
        "nothing to path"
    );
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
    let path = StorePath::new(parts_of("CASH", "ZQCASH", Timeframe::MINUTE_1, at)).unwrap();
    let file = path.to_path_buf(&root);
    std::fs::remove_file(&file).unwrap();
    std::fs::write(&file, b"cut").unwrap();
    let why = committed_cash_days(&root, Vendor::Gdfl, "ZQCASH", &[at]).unwrap_err();
    assert!(
        why.starts_with("ZQCASH 2026-08: committed cash source: "),
        "{why}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A bar of no price, in the stored shape.
fn unpriced(ts_micros: i64) -> Bar {
    Bar {
        open: 0,
        high: 0,
        low: 0,
        close: 0,
        ..bar(ts_micros)
    }
}

/// A scan judges what is stored as a landing would: a share's bar of no price
/// is flagged refused on both rungs, an index's is flagged held, and the
/// count of each rung's months says so. A series of nothing scans to nothing.
#[test]
fn a_scan_flags_what_a_landing_refuses_and_labels_what_an_index_holds() {
    use super::identity_screen::scan_series;
    let root = scratch("cov-scan");
    let at = month(2026, 8);
    let on = Day::new(2026, 8, 3).unwrap();
    let zero = [unpriced(opening(on))];
    for rung in [Timeframe::DAY_1, Timeframe::MINUTE_1] {
        file_bars(&root, ("CASH", "ZQCASH"), rung, at, &zero);
        file_bars(&root, ("INDEX", "ZQIDX"), rung, at, &zero);
    }
    let share = scan_series(&root, parts_of("CASH", "ZQCASH", Timeframe::MINUTE_1, at));
    let said: Vec<(Timeframe, bool)> = share
        .flagged
        .iter()
        .map(|flag| (flag.timeframe, flag.refused))
        .collect();
    assert_eq!(
        said,
        [(Timeframe::DAY_1, true), (Timeframe::MINUTE_1, true)],
        "{share:?}"
    );
    assert_eq!(share.unread, []);
    let index = scan_series(&root, parts_of("INDEX", "ZQIDX", Timeframe::MINUTE_1, at));
    let said: Vec<(Timeframe, bool)> = index
        .flagged
        .iter()
        .map(|flag| (flag.timeframe, flag.refused))
        .collect();
    assert_eq!(
        said,
        [(Timeframe::DAY_1, false), (Timeframe::MINUTE_1, false)],
        "{index:?}"
    );
    let contract = brutex_core::instrument::Contract::parse("2026-08-27-FUT");
    let none = scan_series(
        &root,
        PathParts {
            contract,
            ..parts_of("CASH", "ZQNONE", Timeframe::MINUTE_1, at)
        },
    );
    assert!(
        none.flagged.is_empty() && none.months.is_empty(),
        "{none:?}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A minute month that will not open costs that month, named by its rung; the
/// months beside it are still judged.
#[test]
fn a_scan_names_a_minute_month_it_could_not_read() {
    use super::identity_screen::scan_series;
    let root = scratch("cov-scan-unread");
    let at = month(2026, 8);
    let on = Day::new(2026, 8, 3).unwrap();
    file_cash(&root, "ZQCASH", at, &[bar(opening(on))]);
    let path = StorePath::new(parts_of("CASH", "ZQCASH", Timeframe::MINUTE_1, at)).unwrap();
    let file = path.to_path_buf(&root);
    std::fs::remove_file(&file).unwrap();
    std::fs::write(&file, b"cut").unwrap();
    let scan = scan_series(&root, parts_of("CASH", "ZQCASH", Timeframe::MINUTE_1, at));
    assert_eq!(scan.unread.len(), 1, "{scan:?}");
    assert!(scan.unread[0].1.starts_with("1min: "), "{scan:?}");
    std::fs::remove_dir_all(root).unwrap();
}
