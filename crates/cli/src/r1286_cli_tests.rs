#![cfg(test)]
//! Gate 18 run 1286, group cli: the surviving `lib.rs` mutants whose function
//! is private to the crate root, each pinned by the exact behaviour the
//! mutant changed (R1286-cli-NN, D-4100).

#![expect(
    clippy::indexing_slicing,
    reason = "fixed fixture indices: a wrong one must fail the test"
)]
use super::*;

/// Days since the Unix epoch of five consecutive IST sessions, Monday
/// 2025-01-06 onward.
const DAYS: [i64; 5] = [20_094, 20_095, 20_096, 20_097, 20_098];

/// The bar `minute` minutes after 09:15 IST on IST day `day`.
const fn bar(day: i64, minute: i64) -> indicators::Candle {
    const MICROS_PER_DAY: i64 = 86_400_000_000;
    const MICROS_PER_MINUTE: i64 = 60_000_000;
    // 09:15 IST is 03:45 UTC.
    let ts_micros = day * MICROS_PER_DAY + (3 * 60 + 45 + minute) * MICROS_PER_MINUTE;
    indicators::Candle {
        ts_micros,
        open: 2_500_000,
        high: 2_500_100,
        low: 2_499_900,
        close: 2_500_000,
        volume: 0,
        open_interest: indicators::OI_NULL,
    }
}

/// **A walk-forward window's folded series runs from just after the swept bar
/// before it, through its own last bar, and no further.** R1286-cli-09,
/// D-4100.
///
/// Five sessions of two bars each; the second and fourth are withheld, so the
/// swept series is sessions 1, 3 and 5. For each window of the swept series
/// the folded part must be exactly:
///
/// - the window at the start: its own two bars, nothing before them;
/// - the middle window: the withheld session 2 directly before it, then its
///   own bars -- and NOT the withheld session 4 or session 5 after its last
///   bar, nor session 1 before the swept bar that precedes it;
/// - the window at the end: the withheld session 4, then its own bars;
/// - one bar of the middle session alone, each half: the half after the first
///   bar starts after that first bar, not at session 2;
/// - the whole swept series: the whole folded series.
///
/// Both comparisons are exercised on both sides: a bar equal to the window's
/// first or last bar is inside it, one before the first is not, one after the
/// last ends the walk. The withheld days are passed through unchanged and the
/// swept slice is the window itself. An empty window folds nothing.
#[test]
fn a_windows_folded_series_is_exactly_the_withheld_bars_before_it_and_its_own() {
    let folded: Vec<indicators::Candle> = DAYS
        .iter()
        .flat_map(|&day| [bar(day, 0), bar(day, 1)])
        .collect();
    let withheld = [DAYS[1], DAYS[3]];
    let swept: Vec<indicators::Candle> = folded
        .iter()
        .copied()
        .filter(|bar| !withheld.contains(&indicators::ist_day(bar.ts_micros)))
        .collect();
    assert_eq!(swept.len(), 6, "premise: three swept sessions of two bars");
    for (window, folded_part) in [
        (0..2, 0..2),
        (2..4, 2..6),
        (4..6, 6..10),
        (2..3, 2..5),
        (3..4, 5..6),
        (0..6, 0..10),
    ] {
        let slice = &swept[window.clone()];
        let withholding = window_withholding(&folded, &withheld, slice);
        assert_eq!(
            withholding.folded,
            &folded[folded_part.clone()],
            "window {window:?} folds {folded_part:?}"
        );
        assert_eq!(withholding.swept, slice, "window {window:?}");
        assert_eq!(withholding.days, &withheld[..], "window {window:?}");
    }
    let empty = window_withholding(&folded, &withheld, &[]);
    assert!(empty.folded.is_empty() && empty.swept.is_empty() && empty.days.is_empty());
}
