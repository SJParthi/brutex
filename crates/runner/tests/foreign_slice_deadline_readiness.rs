//! Generated fixtures only: public cached facts cannot authorize a later exit.
use costs::fill::Direction;
use indicators::column::Column;
use indicators::evaluator::{Calendar, Evaluator, Widths};
use runner::excursion::Side;
use runner::expression::Expression;
use runner::grid::{self, Levels};
use runner::outcome::Horizon;
use runner::trade::{self, SliceFacts};
use vocab::ConditionMask;

const MINUTE: i64 = 60_000_000;

fn fixture() -> Result<(Vec<indicators::Candle>, Column), String> {
    let bars = runner::synthetic::sessions(8);
    let mut evaluator = Evaluator::with_calendar(
        Widths::pinned().map_err(|why| format!("{why:?}"))?,
        indicators::vwap::Availability::Absent,
        indicators::pattern::Thresholds::CLASSICAL,
        Calendar::all_regular(),
    );
    let column = Column::build(&bars, &mut evaluator);
    assert!(!column.is_empty());
    Ok((bars, column))
}

fn levels() -> Levels<'static> {
    Levels {
        rungs: 1,
        step_ppm: Some(100),
        forced: None,
        ratios: true,
        stops_ppm: &[100],
    }
}

fn projected(column: &Column, bars: &[indicators::Candle]) -> Result<Column, String> {
    let onto: Vec<_> = column
        .sources()
        .iter()
        .map(|source| Some(*source))
        .collect();
    let (projected, dropped) = column
        .reproject_checked(&onto, bars)
        .ok_or("checked projection")?;
    assert_eq!(dropped, 0);
    Ok(projected)
}

fn elapsed(bars: &[indicators::Candle], item: &trade::Trade) -> Result<i64, String> {
    let start = bars.get(item.entry_bar).ok_or("actual entry")?.ts_micros;
    let end = bars.get(item.exit_bar).ok_or("actual exit")?.ts_micros;
    end.checked_sub(start).ok_or("elapsed overflow".to_owned())
}

#[test]
fn every_public_cached_walk_refuses_foreign_facts_that_move_1509_to_1520() -> Result<(), String> {
    let (bars, column) = fixture()?;
    let projected = projected(&column, &bars)?;
    let foreign: Vec<_> = bars
        .iter()
        .copied()
        .map(|mut bar| {
            bar.ts_micros -= 11 * MINUTE;
            bar
        })
        .collect();
    let horizon = Horizon::bars(1000).ok_or("positive horizon")?;
    let expression = Expression::parse("30 | !30").map_err(|why| format!("{why:?}"))?;
    for column in [&column, &projected] {
        let original = SliceFacts::of(&bars, column);
        let stale = SliceFacts::of(&foreign, column);
        for (direction, side) in [
            (Direction::Long, Side::Long),
            (Direction::Short, Side::Short),
        ] {
            let control = trade::walk_over(
                &bars,
                column,
                &ConditionMask::ZERO,
                horizon,
                direction,
                &original,
            );
            assert!(control.count() > 0 && control.reconciles());
            for item in &control.trades {
                let actual = bars.get(item.exit_bar).ok_or("control exit")?;
                let minute = (i128::from(actual.ts_micros) + 19_800_000_000)
                    .rem_euclid(86_400_000_000)
                    / i128::from(MINUTE);
                assert_eq!(
                    minute, 909,
                    "actual 15:09 record prices the 15:10 forced exit"
                );
            }
            let mask = trade::walk_over(
                &bars,
                column,
                &ConditionMask::ZERO,
                horizon,
                direction,
                &stale,
            );
            let boolean = trade::walk_expression_over(
                &bars,
                column,
                &expression,
                horizon,
                direction,
                &stale,
            )?;
            assert_eq!(mask, boolean);
            assert!(mask.signals > 0 && mask.reconciles());
            assert!(
                mask.trades.is_empty(),
                "foreign cached15:09 is actual15:20 and cannot price any trade"
            );
            assert!(mask.eligible.is_empty() && mask.occupancy.is_empty());
            let mask_grid = grid::evaluate_over(
                &bars,
                column,
                &ConditionMask::ZERO,
                horizon,
                side,
                levels(),
                &stale,
            );
            let boolean_grid = grid::evaluate_expression_over(
                &bars,
                column,
                &expression,
                horizon,
                side,
                levels(),
                &stale,
            )?;
            assert_eq!(mask_grid, boolean_grid);
            assert!(
                mask_grid.cells.is_empty(),
                "neither grid may resurrect a refused timed path"
            );
        }
    }
    Ok(())
}

#[test]
fn actual_clock_guard_preserves_earlier_horizons_on_both_public_predicate_paths()
-> Result<(), String> {
    let (bars, column) = fixture()?;
    let projected = projected(&column, &bars)?;
    let expression = Expression::parse("30 | !30").map_err(|why| format!("{why:?}"))?;
    for column in [&column, &projected] {
        let facts = SliceFacts::of(&bars, column);
        for h in [3, 15] {
            let horizon = Horizon::bars(h).ok_or("positive horizon")?;
            for (direction, side) in [
                (Direction::Long, Side::Long),
                (Direction::Short, Side::Short),
            ] {
                let mask = trade::walk_over(
                    &bars,
                    column,
                    &ConditionMask::ZERO,
                    horizon,
                    direction,
                    &facts,
                );
                let boolean = trade::walk_expression_over(
                    &bars,
                    column,
                    &expression,
                    horizon,
                    direction,
                    &facts,
                )?;
                assert_eq!(mask, boolean);
                assert!(mask.count() > 0 && mask.reconciles());
                let ordinary: Vec<_> = mask.trades.iter().filter(|item| !item.forced).collect();
                assert!(!ordinary.is_empty());
                for item in ordinary {
                    assert_eq!(elapsed(&bars, item)?, i64::from(h) * MINUTE);
                }
                let mask_grid = grid::evaluate_over(
                    &bars,
                    column,
                    &ConditionMask::ZERO,
                    horizon,
                    side,
                    levels(),
                    &facts,
                );
                let boolean_grid = grid::evaluate_expression_over(
                    &bars,
                    column,
                    &expression,
                    horizon,
                    side,
                    levels(),
                    &facts,
                )?;
                assert_eq!(mask_grid, boolean_grid);
                assert!(
                    !mask_grid.cells.is_empty(),
                    "valid earlier paths must still reach the grid"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn matching_forced_boundary_cannot_hide_a_foreign_earlier_horizon_lookup() -> Result<(), String> {
    let (bars, column) = fixture()?;
    let projected = projected(&column, &bars)?;
    // Preserve the exact forced row/index while translating all earlier rows.
    // The cache now points an ordinary deadline eleven actual minutes later.
    let foreign: Vec<_> = bars
        .iter()
        .copied()
        .map(|mut bar| {
            let minute = (i128::from(bar.ts_micros) + 19_800_000_000).rem_euclid(86_400_000_000)
                / i128::from(MINUTE);
            if minute < 909 {
                bar.ts_micros -= 11 * MINUTE;
            }
            bar
        })
        .collect();
    let horizon = Horizon::bars(3).ok_or("positive horizon")?;
    let expression = Expression::parse("30 | !30").map_err(|why| format!("{why:?}"))?;
    for column in [&column, &projected] {
        let stale = SliceFacts::of(&foreign, column);
        for direction in [Direction::Long, Direction::Short] {
            let mask = trade::walk_over(
                &bars,
                column,
                &ConditionMask::ZERO,
                horizon,
                direction,
                &stale,
            );
            let boolean = trade::walk_expression_over(
                &bars,
                column,
                &expression,
                horizon,
                direction,
                &stale,
            )?;
            assert_eq!(mask, boolean);
            assert!(mask.signals > 0 && mask.reconciles());
            assert!(
                !mask.occupancy.is_empty(),
                "missing exact horizons retain conservative occupancy"
            );
            for item in &mask.eligible {
                assert!(
                    item.forced || elapsed(&bars, item)? == 3 * MINUTE,
                    "a matching forced row cannot authorize a fourteen-minute ordinary hold"
                );
            }
            assert!(
                mask.eligible.iter().all(|item| item.forced),
                "every cached ordinary lookup was translated"
            );
        }
    }
    Ok(())
}

#[test]
fn checked_grids_refuse_an_actual_late_or_foreign_day_inside_cached_endpoints() -> Result<(), String>
{
    let (bars, column) = fixture()?;
    let projected = projected(&column, &bars)?;
    let first = column.sources().first().copied().ok_or("first signal")?;
    let corrupt_index = first.checked_add(2).ok_or("interior index")?;
    let horizon = Horizon::bars(15).ok_or("positive horizon")?;
    let expression = Expression::parse("30 | !30").map_err(|why| format!("{why:?}"))?;
    for column in [&column, &projected] {
        let facts = SliceFacts::of(&bars, column);
        for (direction, side) in [
            (Direction::Long, Side::Long),
            (Direction::Short, Side::Short),
        ] {
            let control = grid::evaluate_over(
                &bars,
                column,
                &ConditionMask::ZERO,
                horizon,
                side,
                levels(),
                &facts,
            );
            assert!(!control.cells.is_empty());
            assert_eq!(control.refused_paths, 0);
            for next_day in [false, true] {
                let mut corrupted = bars.clone();
                let bar = corrupted
                    .get_mut(corrupt_index)
                    .ok_or("interior actual row")?;
                let day = (i128::from(bar.ts_micros) + 19_800_000_000).div_euclid(86_400_000_000);
                let day = day + i128::from(next_day);
                bar.ts_micros =
                    i64::try_from(day * 86_400_000_000 + 920 * i128::from(MINUTE) - 19_800_000_000)
                        .map_err(|why| why.to_string())?;
                let timed = trade::walk_over(
                    &corrupted,
                    column,
                    &ConditionMask::ZERO,
                    horizon,
                    direction,
                    &facts,
                );
                assert!(timed.eligible.iter().any(|item| item.entry_bar < corrupt_index && corrupt_index < item.exit_bar),
                    "the attack must lie strictly inside a timed path whose endpoints still pass");
                let mask = grid::evaluate_over(
                    &corrupted,
                    column,
                    &ConditionMask::ZERO,
                    horizon,
                    side,
                    levels(),
                    &facts,
                );
                let boolean = grid::evaluate_expression_over(
                    &corrupted,
                    column,
                    &expression,
                    horizon,
                    side,
                    levels(),
                    &facts,
                )?;
                assert_eq!(mask, boolean);
                assert!(
                    mask.refused_paths > control.refused_paths,
                    "actual late/foreign interior timestamps cannot be admitted by clean cached acceptance"
                );
            }
        }
    }
    Ok(())
}
