//! lookahead-oos-1: a holed day is CUT at its first hole, not withheld whole.
//!
//! `AnchoredColumn::build_required_withholding_from` takes `(IST day, first
//! withheld bar-open)` pairs. A bar of a cut day opening before its stamp keeps
//! its row; every bar from the stamp on is folded and given none (D-1781: the
//! fold stays whole). A whole day is `(day, i64::MIN)`, so the old door is the
//! special case and still answers exactly as it did.
//!
//! Invented bars only: a deterministic xorshift walk at paisa levels no index
//! printed.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a failing fixture must name its premise"
)]

use indicators::Candle;
use indicators::anchored::{AnchoredEvaluator, DailyEligibility, DailyReference};
use indicators::column::AnchoredColumn;
use indicators::evaluator::{Calendar, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;

const DAY: i64 = 86_400_000_000;
const MINUTE: i64 = 60_000_000;
/// 09:15 IST in UTC microseconds past midnight.
const OPEN: i64 = 225 * MINUTE;
const FIRST_DAY: i64 = 27_000;
const SESSIONS: i64 = 6;
const BARS_PER_SESSION: i64 = 375;
const HOLED_DAY: i64 = FIRST_DAY + 3;
/// The interior minute the vendor never served on the holed day.
const HOLE: i64 = 150;

fn walk() -> Vec<Candle> {
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        i64::try_from(state % 4_001).expect("small") - 2_000
    };
    let mut close: i64 = 3_141_500;
    let mut bars = Vec::new();
    for day in FIRST_DAY..FIRST_DAY + SESSIONS {
        for minute in 0..BARS_PER_SESSION {
            let open = close;
            close = (close + next()).max(1_000_000);
            let wick = next().abs() / 4;
            if day == HOLED_DAY && minute == HOLE {
                continue;
            }
            bars.push(Candle {
                ts_micros: day * DAY + OPEN + minute * MINUTE,
                open,
                high: open.max(close) + wick,
                low: open.min(close) - wick,
                close,
                volume: 0,
                open_interest: i64::MIN,
            });
        }
    }
    bars
}

fn references() -> Vec<DailyReference> {
    (FIRST_DAY - 5..FIRST_DAY + SESSIONS)
        .map(|day| {
            DailyReference::new(
                Candle {
                    ts_micros: day * DAY + OPEN,
                    open: 3_141_500,
                    high: 3_151_500,
                    low: 3_131_500,
                    close: 3_142_000,
                    volume: 0,
                    open_interest: i64::MIN,
                },
                DailyEligibility::Eligible,
            )
            .expect("an eligible generated daily record")
        })
        .collect()
}

fn anchored(references: &[DailyReference]) -> AnchoredEvaluator<'_> {
    AnchoredEvaluator::with_calendar(
        Widths::pinned().expect("pinned widths"),
        Availability::Absent,
        Thresholds::CLASSICAL,
        Calendar::all_regular(),
        references,
    )
    .expect("ordered references")
}

/// **A cut keeps every row before it, gives none from it on, and every kept
/// row is the whole fold's row for the same bar.**
#[test]
fn a_cut_day_keeps_its_rows_before_the_cut_and_folds_the_rest() {
    let bars = walk();
    let references = references();
    let whole = AnchoredColumn::build_required(&bars, &mut anchored(&references))
        .expect("every day has a prior record");
    let cut_at = HOLED_DAY * DAY + OPEN + HOLE * MINUTE;
    let cut = AnchoredColumn::build_required_withholding_from(
        &bars,
        &mut anchored(&references),
        &[(HOLED_DAY, cut_at)],
    )
    .expect("every day has a prior record");
    // 374 bars on the holed day; the 150 before the hole keep their rows.
    let after = bars
        .iter()
        .filter(|bar| indicators::ist_day(bar.ts_micros) == HOLED_DAY && bar.ts_micros >= cut_at)
        .count();
    assert_eq!(after, 224, "premise: minutes 151..374");
    assert_eq!(cut.withheld_bars(), 224);
    // THE COLUMN HOLDS WARM BARS ONLY: row `r` of the whole column is bar
    // `warm + r`. The warm-up is a prefix that ends before the holed day, and
    // the fold is whole, so the cut column warms at the same bar.
    let warm = whole.column().first_swept().expect("the walk warms up");
    assert_eq!(cut.column().first_swept(), Some(warm));
    assert_eq!(
        whole.column().len(),
        bars.len() - warm,
        "premise: warm-up is a prefix"
    );
    let holed_first = usize::try_from((HOLED_DAY - FIRST_DAY) * BARS_PER_SESSION).expect("small");
    assert!(warm < holed_first, "premise: warm before the holed day");
    let kept: Vec<usize> = (warm..bars.len())
        .filter(|&index| {
            let bar = &bars[index];
            !(indicators::ist_day(bar.ts_micros) == HOLED_DAY && bar.ts_micros >= cut_at)
        })
        .collect();
    assert_eq!(cut.column().bits().len(), kept.len());
    for (row, &index) in kept.iter().enumerate() {
        assert_eq!(
            cut.column().bits()[row],
            whole.column().bits()[index - warm],
            "bar {index}"
        );
        assert_eq!(
            cut.column().known()[row],
            whole.column().known()[index - warm],
            "bar {index}"
        );
    }
    // Rows before the cut on the holed day itself are kept: the cut, not the
    // day, decides (150 bars from 09:15 to the hole).
    let before_on_day = kept
        .iter()
        .filter(|&&index| indicators::ist_day(bars[index].ts_micros) == HOLED_DAY)
        .count();
    assert_eq!(before_on_day, 150);
    // The fold stays whole: the daily census is the whole fold's.
    assert_eq!(cut.reference_census(), whole.reference_census());
}

/// **`(day, i64::MIN)` is the whole-day door, a day cut twice keeps its
/// earlier cut, and a pair for a day the series does not hold withholds
/// nothing.**
#[test]
fn a_whole_day_is_the_cut_at_minus_infinity_and_the_earlier_cut_wins() {
    let bars = walk();
    let references = references();
    let whole_day =
        AnchoredColumn::build_required_withholding(&bars, &mut anchored(&references), &[HOLED_DAY])
            .expect("every day has a prior record");
    let as_pair = AnchoredColumn::build_required_withholding_from(
        &bars,
        &mut anchored(&references),
        &[(HOLED_DAY, i64::MIN)],
    )
    .expect("every day has a prior record");
    assert_eq!(whole_day.withheld_bars(), 374);
    assert_eq!(as_pair.column().bits(), whole_day.column().bits());
    assert_eq!(as_pair.column().known(), whole_day.column().known());
    assert_eq!(as_pair.withheld_bars(), whole_day.withheld_bars());

    let early = HOLED_DAY * DAY + OPEN + 10 * MINUTE;
    let late = HOLED_DAY * DAY + OPEN + 300 * MINUTE;
    let twice = AnchoredColumn::build_required_withholding_from(
        &bars,
        &mut anchored(&references),
        &[(HOLED_DAY, late), (HOLED_DAY, early)],
    )
    .expect("every day has a prior record");
    assert_eq!(twice.withheld_bars(), 364, "minutes 10..374 less the hole");

    let elsewhere = AnchoredColumn::build_required_withholding_from(
        &bars,
        &mut anchored(&references),
        &[(FIRST_DAY - 1, i64::MIN)],
    )
    .expect("every day has a prior record");
    assert_eq!(elsewhere.withheld_bars(), 0);
    let whole = AnchoredColumn::build_required(&bars, &mut anchored(&references))
        .expect("every day has a prior record");
    assert_eq!(elsewhere.column().bits(), whole.column().bits());
}
