//! Finite adversarial evidence for the column's warm-up admission boundary.

#![allow(
    clippy::expect_used,
    reason = "a failing fixture must name its premise"
)]

use indicators::Candle;
use indicators::column::Column;
use indicators::daily::{DailyLevels, Unusable};
use indicators::evaluator::{Calendar, Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;

const DAY: i64 = 86_400_000_000;
const MINUTE: i64 = 60_000_000;
const OPEN: i64 = 225 * MINUTE;

fn candle(day: i64, minute: i64, high: i64, low: i64, close: i64) -> Candle {
    Candle {
        ts_micros: day * DAY + OPEN + minute * MINUTE,
        open: close,
        high,
        low,
        close,
        volume: 0,
        open_interest: i64::MIN,
    }
}

#[test]
fn a_session_rollover_cannot_admit_a_mask_after_its_daily_anchor_becomes_unusable() {
    let mut evaluator = Evaluator::with_calendar(
        Widths::pinned().expect("pinned widths"),
        Availability::Absent,
        Thresholds::CLASSICAL,
        Calendar::all_regular(),
    );
    for day in 27_000..27_006 {
        for minute in 0..50 {
            evaluator
                .step(&candle(day, minute, 2_501_000, 2_499_000, 2_500_000))
                .expect("ordinary positive-price warm-up bar");
        }
    }
    assert!(evaluator.warmed_up(), "the ordinary prefix must be warm");
    let ordinary = candle(27_005, 50, 2_501_000, 2_499_000, 2_500_000);
    let ordinary_column = Column::build(&[ordinary], &mut evaluator);
    assert_eq!(ordinary_column.census().swept, 1);
    assert_eq!(ordinary_column.census().warming, 0);
    assert_eq!(ordinary_column.bits().len(), 1);
    let extreme = candle(27_006, 0, i64::MAX, 1, 2_500_000);
    assert!(
        extreme.check_evaluable().is_ok(),
        "the bar is individually valid"
    );
    assert_eq!(
        DailyLevels::from_previous_session(extreme.high, extreme.low, extreme.close),
        Err(Unusable::LevelOverflows),
        "the completed session must make its derived ladder unrepresentable"
    );
    evaluator
        .step(&extreme)
        .expect("the individual bar is evaluable");
    assert!(
        evaluator.warmed_up(),
        "the previous ordinary anchor still applies"
    );

    let next = candle(27_007, 0, 2_501_000, 2_499_000, 2_500_000);
    let column = Column::build(&[next], &mut evaluator);
    assert!(
        !evaluator.warmed_up(),
        "rollover must invalidate the daily anchor"
    );
    assert!(
        !evaluator.has_yesterday(),
        "no stale ordinary ladder may remain"
    );
    assert_eq!(
        column.census().swept,
        0,
        "a stale pre-step warmth verdict must not admit the now-cold mask"
    );
    assert_eq!(column.census().warming, 1);
    assert!(column.bits().is_empty());
}
