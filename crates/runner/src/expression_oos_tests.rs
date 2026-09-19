#![cfg(test)]
//! Generated, finite later-period authority and exact replay regressions.
use super::*;
use crate::exit_grid_policy::{
    ExecutionResolutionV1, ExecutionSeriesV1, ExitGridPolicyV1, ExitGridSelectorV1, ForcedStopV1,
    RangeResolutionV1, RatioLimitsV1, RationalPercentileV1, RungPlanV1,
    printed_ohlcv_cost_model_id_v1,
};
use crate::identity::{Direction, Params, Run};
use brutex_core::instrument::{Exchange, InstrumentKey};
use indicators::evaluator::{Calendar, Evaluator, Widths};
const COMMIT: &str = "generated-later-program-fixture";
fn fixture(shift: i64) -> Result<(Vec<Candle>, Column), String> {
    let mut bars = crate::synthetic::sessions(8);
    for bar in &mut bars {
        bar.ts_micros += shift;
    }
    let mut evaluator = Evaluator::with_calendar(
        Widths::pinned().map_err(super::super::display)?,
        indicators::vwap::Availability::Absent,
        indicators::pattern::Thresholds::CLASSICAL,
        Calendar::all_regular(),
    );
    let column = Column::build(&bars, &mut evaluator);
    assert!(!column.is_empty());
    Ok((bars, column))
}
fn series<'a>(key: &'a InstrumentKey, bars: &'a [Candle]) -> Result<ExecutionSeriesV1<'a>, String> {
    ExecutionSeriesV1::new(key, "zerodha", COMMIT, [7; 32], bars).map_err(super::super::display)
}
fn run(
    key: &InstrumentKey,
    bars: &[Candle],
    program: &Expression,
    side: Direction,
) -> Result<ExpressionExecutionRunV1, String> {
    ExpressionExecutionRunV1::new(
        &Run {
            mask: program.referenced(),
            direction: side,
            instrument: key,
            timeframe: "1min",
            params: Params::of(engine::Ladder::with_min_hits(1)),
            data_digest: crate::identity::data_digest_with_execution(bars, None),
            commit: COMMIT,
            feed: "zerodha",
        },
        program,
        bars,
        None,
    )
}
fn policy(side: crate::excursion::Side) -> Result<ExitGridPolicyV1, String> {
    let p = RationalPercentileV1::new(1, 2).map_err(super::super::display)?;
    ExitGridPolicyV1::new(
        ExecutionResolutionV1::OneMinuteOhlcv,
        RangeResolutionV1::PpmCeiling,
        side,
        RungPlanV1::new(vec![p], vec![p], vec![p], 1).map_err(super::super::display)?,
        RatioLimitsV1::new(1, 1_000_000, 1).map_err(super::super::display)?,
        32,
        ExitGridSelectorV1::PessimisticTotal,
        printed_ohlcv_cost_model_id_v1(),
        ForcedStopV1::Disabled,
        u64::MAX,
        u64::MAX,
    )
    .map_err(super::super::display)
}
#[test]
fn later_replays_every_frozen_coordinate_and_preserves_zero_unknown_programs() -> Result<(), String>
{
    let (bars, column) = fixture(0)?;
    let (later, later_column) = fixture(8 * 86_400_000_000)?;
    let key = InstrumentKey::index(Exchange::Nse, "NIFTY").map_err(super::super::display)?;
    for (side, direction) in [
        (crate::excursion::Side::Long, Direction::Long),
        (crate::excursion::Side::Short, Direction::Short),
    ] {
        let resolved = policy(side)?
            .resolve_research_attested(series(&key, &bars)?)
            .map_err(super::super::display)?;
        let attested = resolved
            .attest_training(
                series(&key, &bars)?,
                &column,
                Horizon::bars(5).ok_or("horizon")?,
            )
            .map_err(super::super::display)?;
        let mut anchors = std::collections::HashSet::new();
        let mut comparisons = std::collections::HashSet::new();
        for raw in ["30 | !30", "30 & !30", "146 | !146"] {
            let program = Expression::parse(raw).map_err(|e| format!("{e:?}"))?;
            let training = resolved.evaluate_expression_with_attested(
                &attested,
                &run(&key, &bars, &program, direction)?,
            )?;
            let anchor = resolved
                .validate_expression_evaluation(&training)?
                .later_period_anchor();
            let observed = resolved.evaluate_expression_oos(
                &anchor,
                series(&key, &later)?,
                &later_column,
                &run(&key, &later, &program, direction)?,
            )?;
            assert_eq!(anchor.run_id(), training.run_id());
            assert_eq!(anchor.horizon().as_bars(), 5);
            assert!(anchors.insert(anchor.digest()));
            assert!(comparisons.insert(observed.digest()));
            assert_eq!(observed.support_sessions(), training.support_sessions());
            assert_eq!(
                observed.run_id(),
                run(&key, &later, &program, direction)?.run_id()
            );
            assert_eq!(observed.grid(), training.grid());
            assert_ne!(observed.run_id(), anchor.run_id());
            assert_eq!(observed.grid().cells.len() as u64, resolved.cell_count());
            let mut total = 0;
            for (ordinal, cell) in observed.grid().cells.iter().enumerate() {
                let trades = observed.materialize(ordinal)?;
                assert_eq!(trades.len() as u64, cell.trades);
                assert_eq!(
                    trades.iter().map(|t| t.worst).sum::<i64>(),
                    cell.pessimistic
                );
                assert!(observed.refusal_bits(ordinal).is_some());
                if cell.trades == 0 {
                    assert!(
                        observed
                            .refusal_bits(ordinal)
                            .ok_or("refusal")?
                            .contains(crate::exit_grid_policy::ExecutionRefusalBitsV1::ZERO_TRADES)
                    );
                }
                for trade in &trades {
                    assert_eq!(
                        (i128::from(trade.entry_micros) + 19_800_000_000)
                            .div_euclid(86_400_000_000),
                        (i128::from(trade.exit_micros) + 19_800_000_000).div_euclid(86_400_000_000)
                    );
                    assert!(
                        (trade.exit_micros + 19_800_000_000).rem_euclid(86_400_000_000)
                            <= 54_540_000_000
                    );
                }
                total += trades.len();
            }
            if raw == "30 | !30" {
                assert!(total > 0);
            } else {
                assert_eq!(total, 0);
            }
            if raw == "146 | !146" {
                assert!(observed.summary().unknown > 0);
            }
            assert!(observed.materialize(observed.grid().cells.len()).is_err());
        }
    }
    Ok(())
}
#[test]
fn later_refuses_overlap_program_substitution_foreign_series_and_changed_anchor()
-> Result<(), String> {
    let (bars, column) = fixture(0)?;
    let (later, later_column) = fixture(30 * 86_400_000_000)?;
    let key = InstrumentKey::index(Exchange::Nse, "NIFTY").map_err(super::super::display)?;
    let program = Expression::parse("30 | 44").map_err(|e| format!("{e:?}"))?;
    let resolved = policy(crate::excursion::Side::Long)?
        .resolve_research_attested(series(&key, &bars)?)
        .map_err(super::super::display)?;
    let attested = resolved
        .attest_training(
            series(&key, &bars)?,
            &column,
            Horizon::bars(5).ok_or("horizon")?,
        )
        .map_err(super::super::display)?;
    let training = resolved.evaluate_expression_with_attested(
        &attested,
        &run(&key, &bars, &program, Direction::Long)?,
    )?;
    let anchor = resolved
        .validate_expression_evaluation(&training)?
        .later_period_anchor();
    let actual = run(&key, &later, &program, Direction::Long)?;
    assert!(
        resolved
            .evaluate_expression_oos(
                &anchor,
                series(&key, &bars)?,
                &column,
                &run(&key, &bars, &program, Direction::Long)?
            )
            .is_err()
    );
    let other = Expression::parse("30 & 44").map_err(|e| format!("{e:?}"))?;
    assert_eq!(other.referenced(), program.referenced());
    assert!(
        resolved
            .evaluate_expression_oos(
                &anchor,
                series(&key, &later)?,
                &later_column,
                &run(&key, &later, &other, Direction::Long)?
            )
            .is_err()
    );
    for (feed, commit, calendar) in [
        ("other", COMMIT, [7; 32]),
        ("zerodha", "changed", [7; 32]),
        ("zerodha", COMMIT, [8; 32]),
    ] {
        let foreign = ExecutionSeriesV1::new(&key, feed, commit, calendar, &later)
            .map_err(super::super::display)?;
        assert!(
            resolved
                .evaluate_expression_oos(&anchor, foreign, &later_column, &actual)
                .is_err()
        );
    }
    let mut broken = anchor.clone();
    broken.horizon = Horizon::bars(6).ok_or("horizon")?;
    assert!(
        resolved
            .evaluate_expression_oos(&broken, series(&key, &later)?, &later_column, &actual)
            .is_err()
    );
    assert_eq!(anchor.program(), &program);
    assert_eq!(anchor.resolution_digest(), resolved.digest());
    Ok(())
}

#[test]
fn later_prices_cannot_reresolve_training_levels() -> Result<(), String> {
    let (bars, column) = fixture(0)?;
    let (mut later, _) = fixture(30 * 86_400_000_000)?;
    for bar in &mut later {
        bar.open += 2_000_000;
        bar.high += 2_000_000;
        bar.low += 2_000_000;
        bar.close += 2_000_000;
    }
    let mut evaluator = Evaluator::with_calendar(
        Widths::pinned().map_err(super::super::display)?,
        indicators::vwap::Availability::Absent,
        indicators::pattern::Thresholds::CLASSICAL,
        Calendar::all_regular(),
    );
    let later_column = Column::build(&later, &mut evaluator);
    let key = InstrumentKey::index(Exchange::Nse, "NIFTY").map_err(super::super::display)?;
    let policy = policy(crate::excursion::Side::Long)?;
    let resolved = policy
        .resolve_research_attested(series(&key, &bars)?)
        .map_err(super::super::display)?;
    let inappropriate = policy
        .resolve_research_attested(series(&key, &later)?)
        .map_err(super::super::display)?;
    assert_ne!(resolved.stop_levels_ppm(), inappropriate.stop_levels_ppm());
    let program = Expression::parse("30 | !30").map_err(|why| format!("{why:?}"))?;
    let attested = resolved
        .attest_training(
            series(&key, &bars)?,
            &column,
            Horizon::bars(5).ok_or("horizon")?,
        )
        .map_err(super::super::display)?;
    let training = resolved.evaluate_expression_with_attested(
        &attested,
        &run(&key, &bars, &program, Direction::Long)?,
    )?;
    let anchor = resolved
        .validate_expression_evaluation(&training)?
        .later_period_anchor();
    let observed = resolved.evaluate_expression_oos(
        &anchor,
        series(&key, &later)?,
        &later_column,
        &run(&key, &later, &program, Direction::Long)?,
    )?;
    assert_eq!(observed.grid().stops.rungs(), resolved.stop_levels_ppm());
    assert_eq!(
        observed.grid().targets.rungs(),
        resolved.target_levels_ppm()
    );
    assert_eq!(observed.grid().trails.rungs(), resolved.trail_levels_ppm());
    assert!(observed.grid().cells.iter().any(|cell| cell.trades > 0));
    Ok(())
}

#[test]
fn a_later_timestamp_in_the_same_ist_session_is_not_an_oos_period() -> Result<(), String> {
    let (bars, column) = fixture(0)?;
    let shift = bars.last().ok_or("training last")?.ts_micros
        - bars.first().ok_or("training first")?.ts_micros
        + 60_000_000;
    let (later, later_column) = fixture(shift)?;
    assert!(
        later.first().ok_or("later first")?.ts_micros
            > bars.last().ok_or("training last")?.ts_micros
    );
    let key = InstrumentKey::index(Exchange::Nse, "NIFTY").map_err(super::super::display)?;
    let resolved = policy(crate::excursion::Side::Long)?
        .resolve_research_attested(series(&key, &bars)?)
        .map_err(super::super::display)?;
    let program = Expression::parse("30 | !30").map_err(|why| format!("{why:?}"))?;
    let attested = resolved
        .attest_training(
            series(&key, &bars)?,
            &column,
            Horizon::bars(5).ok_or("horizon")?,
        )
        .map_err(super::super::display)?;
    let training = resolved.evaluate_expression_with_attested(
        &attested,
        &run(&key, &bars, &program, Direction::Long)?,
    )?;
    let anchor = resolved
        .validate_expression_evaluation(&training)?
        .later_period_anchor();
    let error = resolved
        .evaluate_expression_oos(
            &anchor,
            series(&key, &later)?,
            &later_column,
            &run(&key, &later, &program, Direction::Long)?,
        )
        .err()
        .ok_or("same session must refuse")?;
    assert!(error.contains("strictly later actual IST session"));
    Ok(())
}

#[test]
fn later_refuses_foreign_frozen_resolution_changed_evaluator_and_entirely_cold_data()
-> Result<(), String> {
    let (bars, column) = fixture(0)?;
    let (later, later_column) = fixture(8 * 86_400_000_000)?;
    let key = InstrumentKey::index(Exchange::Nse, "NIFTY").map_err(super::super::display)?;
    let resolved = policy(crate::excursion::Side::Long)?
        .resolve_research_attested(series(&key, &bars)?)
        .map_err(super::super::display)?;
    let program = Expression::parse("30 | !30").map_err(|why| format!("{why:?}"))?;
    let attested = resolved
        .attest_training(
            series(&key, &bars)?,
            &column,
            Horizon::bars(5).ok_or("horizon")?,
        )
        .map_err(super::super::display)?;
    let training = resolved.evaluate_expression_with_attested(
        &attested,
        &run(&key, &bars, &program, Direction::Long)?,
    )?;
    let anchor = resolved
        .validate_expression_evaluation(&training)?
        .later_period_anchor();
    let actual = run(&key, &later, &program, Direction::Long)?;
    let foreign = policy(crate::excursion::Side::Short)?
        .resolve_research_attested(series(&key, &bars)?)
        .map_err(super::super::display)?;
    let error = foreign
        .evaluate_expression_oos(&anchor, series(&key, &later)?, &later_column, &actual)
        .err()
        .ok_or("foreign resolution")?;
    assert!(error.contains("authority differs"));
    let mut changed = Evaluator::with_calendar(
        Widths::pinned().map_err(super::super::display)?,
        indicators::vwap::Availability::Present,
        indicators::pattern::Thresholds::CLASSICAL,
        Calendar::all_regular(),
    );
    let changed = Column::build(&later, &mut changed);
    let error = resolved
        .evaluate_expression_oos(&anchor, series(&key, &later)?, &changed, &actual)
        .err()
        .ok_or("changed evaluator")?;
    assert!(error.contains("authority differs"));
    let short = later.get(..100).ok_or("cold prefix")?;
    let mut cold = Evaluator::with_calendar(
        Widths::pinned().map_err(super::super::display)?,
        indicators::vwap::Availability::Absent,
        indicators::pattern::Thresholds::CLASSICAL,
        Calendar::all_regular(),
    );
    let cold = Column::build(short, &mut cold);
    assert!(cold.is_empty());
    assert_eq!(cold.acceptance_census().refused(), 0);
    let error = resolved
        .evaluate_expression_oos(
            &anchor,
            series(&key, short)?,
            &cold,
            &run(&key, short, &program, Direction::Long)?,
        )
        .err()
        .ok_or("cold data")?;
    assert!(error.contains("no warm reachable signal rows"));
    Ok(())
}
