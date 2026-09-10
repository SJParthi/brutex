//! Generated OHLCV boundary fixtures; no market execution or profit assurance.

#![allow(
    clippy::expect_used,
    reason = "fixture failures name the broken premise"
)]

use costs::fill::Direction;
use indicators::Candle;
use indicators::column::Column;
use indicators::evaluator::{Calendar, Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use runner::outcome::{Horizon, SessionBounds};
use runner::trade::{SliceFacts, forced_exits, walk, walk_expression_over, walk_with};
use vocab::ConditionMask;
use vocab::expression::Expression;

const MINUTE: i64 = 60_000_000;
const DAY: i64 = 86_400_000_000;
const OFFSET: i64 = 330 * MINUTE;

fn candle(timestamp: i64) -> Candle {
    Candle::new(timestamp, 10_000, 10_010, 9_990, 10_005, 1, i64::MIN)
}

fn stamp(day: i64, minute: i64) -> i64 {
    day * DAY + minute * MINUTE - OFFSET
}

fn evaluator() -> Evaluator {
    Evaluator::with_calendar(
        Widths::pinned().expect("pinned widths"),
        Availability::Absent,
        Thresholds::CLASSICAL,
        Calendar::all_regular(),
    )
}

#[test]
fn microsecond_and_half_minute_offsets_never_certify_a_1509_record_or_a_fill() {
    for day in [-1, 0, 30_000] {
        for offset in [1, 30_000_000] {
            let bars: Vec<_> = (555..930)
                .map(|minute| candle(stamp(day, minute) + offset))
                .collect();
            let bounds = SessionBounds::of(&bars);
            for index in 0..bars.len() {
                assert!(!bounds.day_ended(index), "no exact15:09 at offset{offset}");
                assert!(
                    !bounds.fillable(index),
                    "off-grid row{index} is not an execution minute"
                );
                assert_eq!(bounds.last_fill_bar(index), None);
            }
        }
    }
}

#[test]
fn a_lone_off_grid_required_record_is_missing_while_earlier_exact_rows_stay_usable() {
    for offset in [1, 30_000_000] {
        let bars: Vec<_> = (555..930)
            .map(|minute| candle(stamp(30_000, minute) + if minute == 909 { offset } else { 0 }))
            .collect();
        let bounds = SessionBounds::of(&bars);
        assert!(!bounds.day_ended(0));
        assert!(
            bounds.fillable(0),
            "ordinary exact horizons before the absent forced price remain usable"
        );
        assert_eq!(
            bounds.last_fill_bar(0),
            Some(353),
            "15:08 is a bound, not a substitute forced fill"
        );
        assert!(!bounds.fillable(354));
        assert!(!bounds.fillable(355), "15:10 remains too late");
    }
}

#[test]
fn a_shifted_last_row_cannot_move_the_fixed_deadline_or_replace_the_exact_boundary() {
    for day in [-2, -1, 0, 30_000] {
        let mut bars: Vec<_> = (555..930)
            .map(|minute| candle(stamp(day, minute)))
            .collect();
        bars.last_mut().expect("regular fixture").ts_micros += 30_000_000;
        let bounds = SessionBounds::of(&bars);
        assert!(
            bounds.day_ended(0),
            "the actual exact15:09 is still present"
        );
        assert_eq!(bounds.last_fill_bar(0), Some(354));
        assert!(bounds.fillable(354));
        assert!(!bounds.fillable(355));
        assert!(!bounds.fillable(bars.len() - 1));
    }
}

#[test]
fn checked_civil_deadlines_handle_negative_days_and_timestamp_extremes_without_saturation() {
    let mut starts = vec![stamp(-2, 908), stamp(-1, 908), stamp(0, 908), stamp(1, 908)];
    let low_aligned =
        (i128::from(i64::MIN).div_euclid(i128::from(MINUTE)) + 1) * i128::from(MINUTE);
    let high_aligned =
        (i128::from(i64::MAX).div_euclid(i128::from(MINUTE)) - 3) * i128::from(MINUTE);
    starts.push(i64::try_from(low_aligned).expect("first representable aligned minute"));
    starts.push(i64::try_from(high_aligned).expect("last representable aligned minutes"));
    for start in starts {
        let bars: Vec<_> = (0..3).map(|index| candle(start + index * MINUTE)).collect();
        let bounds = SessionBounds::of(&bars);
        for (index, bar) in bars.iter().enumerate() {
            let local = i128::from(bar.ts_micros) + i128::from(OFFSET);
            let day = local.div_euclid(i128::from(DAY));
            let deadline = day * i128::from(DAY) + 910 * i128::from(MINUTE) - i128::from(OFFSET);
            let closes = i128::from(bar.ts_micros) + i128::from(MINUTE);
            let representable = i64::try_from(deadline).is_ok() && i64::try_from(closes).is_ok();
            assert_eq!(
                bounds.fillable(index),
                representable && closes <= deadline,
                "wide independent deadline at{}",
                bar.ts_micros
            );
        }
    }
    for bars in [
        [candle(i64::MIN), candle(i64::MIN + 1)],
        [candle(i64::MAX - 1), candle(i64::MAX)],
    ] {
        let bounds = SessionBounds::of(&bars);
        assert!(!bounds.fillable(0) && !bounds.fillable(1));
        assert!(!bounds.day_ended(0) && !bounds.day_ended(1));
    }
}

#[test]
fn native_and_projected_mask_and_expression_walks_refuse_off_grid_execution() {
    let aligned = runner::synthetic::sessions(8);
    let horizon = Horizon::bars(15).expect("positive horizon");
    let expression = Expression::parse("30 | !30").expect("available candle tautology");
    let control_column = Column::build(&aligned, &mut evaluator());
    let controls: Vec<_> = [Direction::Long, Direction::Short]
        .into_iter()
        .map(|direction| {
            let trades = walk(
                &aligned,
                &control_column,
                &ConditionMask::ZERO,
                horizon,
                direction,
            );
            assert!(
                trades.count() > 0 && trades.reconciles(),
                "valid exact-minute control trades"
            );
            trades
        })
        .collect();
    assert_eq!(controls.len(), 2);
    for offset in [1, 30_000_000] {
        let shifted: Vec<_> = aligned
            .iter()
            .copied()
            .map(|mut bar| {
                bar.ts_micros += offset;
                bar
            })
            .collect();
        let column = Column::build(&shifted, &mut evaluator());
        assert!(
            !column.is_empty() && column.census().refused() == 0,
            "indicator timestamp acceptance alone cannot prove minute execution"
        );
        let onto: Vec<_> = column
            .sources()
            .iter()
            .map(|source| Some(*source))
            .collect();
        let (projected, dropped) = column
            .reproject_checked(&onto, &shifted)
            .expect("same accepted rows");
        assert_eq!(dropped, 0);
        for column in [&column, &projected] {
            let facts = SliceFacts::of(&shifted, column);
            for index in 0..shifted.len() {
                assert!(!facts.accepts(index));
            }
            assert!(!facts.path_accepts(0, 1));
            assert_eq!(
                facts.at_timestamp(shifted.first().expect("fixture").ts_micros),
                None
            );
            for direction in [Direction::Long, Direction::Short] {
                let trades = walk(&shifted, column, &ConditionMask::ZERO, horizon, direction);
                let boolean =
                    walk_expression_over(&shifted, column, &expression, horizon, direction, &facts)
                        .expect("aligned truth/known vectors");
                assert_eq!(trades, boolean, "both execution doors share exact geometry");
                assert!(trades.signals > 0 && trades.reconciles());
                assert!(
                    trades.trades.is_empty()
                        && trades.eligible.is_empty()
                        && trades.occupancy.is_empty()
                );
            }
        }
    }
}

#[test]
fn an_off_grid_interior_row_cannot_price_a_path_but_unaffected_paths_survive() {
    let mut bars = runner::synthetic::sessions(8);
    let chosen = 7 * 375 + 100;
    bars.get_mut(chosen)
        .expect("last session interior")
        .ts_micros += 1;
    let column = Column::build(&bars, &mut evaluator());
    let facts = SliceFacts::of(&bars, &column);
    assert!(!facts.accepts(chosen));
    assert!(facts.accepts(chosen - 1) && facts.accepts(chosen + 1));
    assert!(!facts.path_accepts(chosen - 1, chosen + 1));
    assert!(facts.path_accepts(chosen + 1, chosen + 3));
    let trades = walk(
        &bars,
        &column,
        &ConditionMask::ZERO,
        Horizon::bars(5).expect("positive horizon"),
        Direction::Long,
    );
    assert!(trades.count() > 0 && trades.reconciles());
    for trade in trades.trades {
        assert!(!(trade.entry_bar..=trade.exit_bar).contains(&chosen));
    }
}

#[test]
fn public_compatibility_walk_refuses_a_later_boundary_from_a_foreign_slice() {
    let bars = runner::synthetic::sessions(8);
    let column = Column::build(&bars, &mut evaluator());
    let horizon = Horizon::bars(1000).expect("hold exceeding the intraday window");
    let exact = forced_exits(&bars);
    // The other slice's15:09 occurs at the index this slice stamps15:20.
    // SquareOff fields are private, but this public constructor still makes a
    // stale or foreign table reachable through the compatibility API.
    let foreign: Vec<_> = bars
        .iter()
        .copied()
        .map(|mut bar| {
            bar.ts_micros -= 11 * MINUTE;
            bar
        })
        .collect();
    let later = forced_exits(&foreign);
    for direction in [Direction::Long, Direction::Short] {
        let ordinary = walk(&bars, &column, &ConditionMask::ZERO, horizon, direction);
        assert!(ordinary.count() > 0 && ordinary.reconciles());
        assert_eq!(
            walk_with(
                &bars,
                &column,
                &ConditionMask::ZERO,
                horizon,
                direction,
                Some(&exact)
            ),
            ordinary
        );
        assert_eq!(
            walk_with(
                &bars,
                &column,
                &ConditionMask::ZERO,
                horizon,
                direction,
                None
            ),
            ordinary
        );
        for table in [&later[..], &[]] {
            let refused = walk_with(
                &bars,
                &column,
                &ConditionMask::ZERO,
                horizon,
                direction,
                Some(table),
            );
            assert!(refused.signals > 0 && refused.reconciles());
            assert!(
                refused.trades.is_empty()
                    && refused.eligible.is_empty()
                    && refused.occupancy.is_empty()
            );
            assert_eq!(
                refused.too_late, refused.signals,
                "mismatched or missing cache is refused, not replaced"
            );
        }
    }
}
