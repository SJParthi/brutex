//! Actual Boolean signals share the legacy fills, exit grid and exact replay.
#![allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes: a fixture or a pricing call that fails must fail the test loudly"
)]
use costs::fill::Direction;
use indicators::column::Column;
use indicators::evaluator::{Calendar, Evaluator, Widths};
use runner::expression::Expression;
use runner::grid::{self, Levels};
use runner::outcome::Horizon;
use runner::trade::{self, SliceFacts};

fn fixture() -> (Vec<indicators::Candle>, Column) {
    let bars = runner::synthetic::sessions(8);
    let mut evaluator = Evaluator::with_calendar(
        Widths::pinned().expect("pinned widths are valid"),
        indicators::vwap::Availability::Absent,
        indicators::pattern::Thresholds::CLASSICAL,
        Calendar::all_regular(),
    );
    let column = Column::build(&bars, &mut evaluator);
    assert!(!column.bits().is_empty());
    (bars, column)
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

#[test]
fn conjunctions_match_all_legacy_trade_and_grid_fields_for_both_sides() -> Result<(), String> {
    let (bars, column) = fixture();
    let facts = SliceFacts::of(&bars, &column);
    let horizon = Horizon::bars(3).ok_or("horizon")?;
    for source in ["0", "63", "369", "0 & 369"] {
        let expression = Expression::parse(source).expect("a valid fixture expression");
        let mask = expression.referenced();
        for (direction, side) in [
            (Direction::Long, runner::excursion::Side::Long),
            (Direction::Short, runner::excursion::Side::Short),
        ] {
            assert_eq!(
                trade::walk_expression_over(
                    &bars,
                    &column,
                    &expression,
                    horizon,
                    direction,
                    &facts
                )
                .expect("the expression walk prices"),
                trade::walk_over(&bars, &column, &mask, horizon, direction, &facts)
            );
            let priced = grid::evaluate_expression_over(
                &bars,
                &column,
                &expression,
                horizon,
                side,
                levels(),
                &facts,
            )
            .expect("the expression grid prices");
            let legacy =
                grid::evaluate_over(&bars, &column, &mask, horizon, side, levels(), &facts);
            assert_eq!(priced, legacy);
            for selected in &priced.cells {
                assert_eq!(
                    grid::materialize_expression_cell(
                        &bars,
                        &column,
                        &expression,
                        horizon,
                        side,
                        &priced,
                        selected
                    )
                    .expect("the expression cell replays"),
                    grid::materialize_cell(&bars, &column, &mask, horizon, side, &legacy, selected)
                        .expect("the legacy cell replays")
                );
            }
        }
    }
    Ok(())
}

#[test]
fn mixed_operators_price_only_definite_signals_and_replay_exact_selected_cells()
-> Result<(), String> {
    let (bars, column) = fixture();
    let facts = SliceFacts::of(&bars, &column);
    let horizon = Horizon::bars(3).ok_or("horizon")?;
    let mut priced_rows = 0;
    for source in ["0 | 369", "!0", "(0 | 369) & !63"] {
        let expression = Expression::parse(source).expect("a valid fixture expression");
        let expected = column
            .bits()
            .iter()
            .zip(column.known())
            .filter(|(truth, known)| {
                let either = (known.get(0) && truth.get(0)) || (known.get(369) && truth.get(369));
                match source {
                    "0 | 369" => either,
                    "!0" => known.get(0) && !truth.get(0),
                    _ => either && known.get(63) && !truth.get(63),
                }
            })
            .count();
        for side in [
            runner::excursion::Side::Long,
            runner::excursion::Side::Short,
        ] {
            let priced = grid::evaluate_expression_over(
                &bars,
                &column,
                &expression,
                horizon,
                side,
                levels(),
                &facts,
            )
            .expect("the expression grid prices");
            assert_eq!(
                priced.signals,
                u64::try_from(expected).expect("a small count")
            );
            if let Some(cell) = priced.best() {
                let rows = grid::materialize_expression_cell(
                    &bars,
                    &column,
                    &expression,
                    horizon,
                    side,
                    &priced,
                    cell,
                )
                .expect("the best cell replays");
                priced_rows += rows.len();
                assert_eq!(
                    u64::try_from(rows.len()).expect("a small count"),
                    cell.trades
                );
                let mut changed = *cell;
                changed.pessimistic = changed.pessimistic.checked_add(1).ok_or("fixture money")?;
                assert!(
                    grid::materialize_expression_cell(
                        &bars,
                        &column,
                        &expression,
                        horizon,
                        side,
                        &priced,
                        &changed
                    )
                    .is_err()
                );
            }
        }
    }
    assert!(
        priced_rows > 0,
        "fixture must actually price mixed expressions"
    );
    Ok(())
}

#[test]
fn unavailable_negation_and_contradiction_never_create_trades() -> Result<(), String> {
    let (bars, column) = fixture();
    let facts = SliceFacts::of(&bars, &column);
    let horizon = Horizon::bars(3).ok_or("horizon")?;
    assert!(
        column.known().iter().all(|known| !known.get(143)),
        "absent VWAP evidence"
    );
    for source in ["!143", "143 | !143", "0 & !0"] {
        let expression = Expression::parse(source).expect("a valid fixture expression");
        for side in [
            runner::excursion::Side::Long,
            runner::excursion::Side::Short,
        ] {
            let priced = grid::evaluate_expression_over(
                &bars,
                &column,
                &expression,
                horizon,
                side,
                levels(),
                &facts,
            )
            .expect("the expression grid prices");
            assert_eq!(priced.signals, 0);
            assert!(priced.cells.is_empty());
        }
    }
    Ok(())
}
