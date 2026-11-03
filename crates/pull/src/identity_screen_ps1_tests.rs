//! BASE-COV-PULL-PS1 (D-7335): a stored minute between the suspect band and
//! the foreign band is flagged by a series scan as suspect: landed, named,
//! never refused (Route B, recipe v4). The series is invented, and every
//! price.

#![allow(clippy::unwrap_used, reason = "a test that cannot panic cannot fail")]

use super::*;
use crate::gdfl_fixtures::scratch;
use crate::session::Day;
use brutex_core::vendor::Vendor;

/// 09:15 IST on `day`, in the store's epoch microseconds.
fn opening(day: Day) -> i64 {
    (i64::from(day.days_from_epoch()) * 86_400 + 13_500) * 1_000_000
}

/// A bar at `ts_micros` whose four prices are `paisa`.
fn bar(ts_micros: i64, paisa: i64) -> Bar {
    Bar {
        ts_micros,
        open: paisa,
        high: paisa,
        low: paisa,
        close: paisa,
        volume: 1,
        open_interest: i64::MIN,
    }
}

/// The invented index's parts at `timeframe` of `month`.
fn parts(timeframe: Timeframe, month: YearMonth) -> PathParts<'static> {
    PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "ZQIDX",
        contract: None,
        timeframe,
        month,
        file: FileKind::Bars,
    }
}

/// Files `bars` under the invented index's `timeframe` month as the store
/// files them.
fn file_bars(root: &Path, timeframe: Timeframe, month: YearMonth, bars: &[Bar]) {
    let identity = identify("ZQIDX", "NSE", "INDEX").unwrap();
    let path = StorePath::new(parts(timeframe, month)).unwrap();
    let mut file = BarFile::open_or_create(root, path, identity.symbol_id).unwrap();
    file.append(bars).unwrap();
}

/// **A SCAN NAMES A SUSPECT MINUTE AS LANDED, NOT REFUSED.** One day bar of
/// 100.00 and three minutes of that day: one at it, one 5% above it (past the
/// 2% suspect band, inside the 8% foreign band) and one 10% above it. The
/// scan flags the second as `suspect_level`, not refused, and the third as
/// `foreign_level`, refused; it flags nothing else, and the minute month's
/// tally counts one of each. (The scans already tested held no minute between
/// the two bands, so the suspect arm of the flagging never ran.)
#[test]
fn a_scan_names_a_suspect_minute_as_landed_not_refused() {
    let root = scratch("ps1.identity-suspect");
    let month = YearMonth::new(2026, 8).unwrap();
    let first_minute = opening(Day::new(2026, 8, 3).unwrap());
    file_bars(&root, Timeframe::DAY_1, month, &[bar(first_minute, 10_000)]);
    let inside = bar(first_minute, 10_000);
    let suspect = bar(first_minute + 60_000_000, 10_500);
    let foreign = bar(first_minute + 120_000_000, 11_000);
    file_bars(
        &root,
        Timeframe::MINUTE_1,
        month,
        &[inside, suspect, foreign],
    );
    let scan = scan_series(&root, parts(Timeframe::MINUTE_1, month));
    let said: Vec<(Timeframe, i64, &str, bool)> = scan
        .flagged
        .iter()
        .map(|flag| (flag.timeframe, flag.bar.ts_micros, flag.code, flag.refused))
        .collect();
    let foreign_code = Cause::ForeignLevel {
        day_low: 10_000,
        day_high: 10_000,
    }
    .code();
    assert_eq!(
        said,
        [
            (
                Timeframe::MINUTE_1,
                suspect.ts_micros,
                series_identity::SUSPECT_CODE,
                false
            ),
            (Timeframe::MINUTE_1, foreign.ts_micros, foreign_code, true),
        ],
        "{scan:?}"
    );
    assert_eq!(scan.months.len(), 1, "{scan:?}");
    let (at, tally) = scan.months.first().unwrap();
    assert_eq!(
        (*at, tally.held, tally.suspect_level, tally.foreign_level),
        (month, 1, 1, 1)
    );
    std::fs::remove_dir_all(root).unwrap();
}
