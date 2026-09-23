#![cfg(test)]
//! Generated fixtures, never market or admission evidence.
use super::super::super::ExpressionExecutionRunV1;
use super::*;
use crate::exit_grid_policy::{
    ExecutionResolutionV1, ExecutionSeriesV1, ExitGridPolicyV1, ExitGridSelectorV1, ForcedStopV1,
    RangeResolutionV1, RatioLimitsV1, RationalPercentileV1, RungPlanV1,
    printed_ohlcv_cost_model_id_v1,
};
use crate::expression::Expression;
use crate::identity::{Direction, Params, Run};
use crate::outcome::Horizon;
use brutex_core::instrument::{Exchange, InstrumentKey};
use indicators::{
    Candle,
    column::Column,
    evaluator::{Calendar, Evaluator, Widths},
};

const COMMIT: &str = "generated-fixed-training-fold-fixture";
const DAY: i64 = 86_400_000_000;
fn fixture(shift: i64, quiet_tail: bool) -> Result<(Vec<Candle>, Column), String> {
    let mut bars = crate::synthetic::sessions(8);
    let first = day(bars.first().ok_or("fixture empty")?.ts_micros);
    for bar in &mut bars {
        if quiet_tail && day(bar.ts_micros) >= first + 6 {
            bar.close = bar.open;
        }
        bar.ts_micros = bar
            .ts_micros
            .checked_add(shift)
            .ok_or("fixture timestamp overflow")?;
    }
    let mut evaluator = Evaluator::with_calendar(
        Widths::pinned().map_err(display)?,
        indicators::vwap::Availability::Absent,
        indicators::pattern::Thresholds::CLASSICAL,
        Calendar::all_regular(),
    );
    let column = Column::build(&bars, &mut evaluator);
    assert!(!column.is_empty());
    Ok((bars, column))
}
fn series<'a>(key: &'a InstrumentKey, bars: &'a [Candle]) -> Result<ExecutionSeriesV1<'a>, String> {
    ExecutionSeriesV1::new(key, "zerodha", COMMIT, [7; 32], bars).map_err(display)
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
    let p = RationalPercentileV1::new(1, 2).map_err(display)?;
    ExitGridPolicyV1::new(
        ExecutionResolutionV1::OneMinuteOhlcv,
        RangeResolutionV1::PpmCeiling,
        side,
        RungPlanV1::new(vec![p], vec![p], vec![p], 1).map_err(display)?,
        RatioLimitsV1::new(1, 1_000_000, 1).map_err(display)?,
        32,
        ExitGridSelectorV1::PessimisticTotal,
        printed_ohlcv_cost_model_id_v1(),
        ForcedStopV1::Disabled,
        u64::MAX,
        u64::MAX,
    )
    .map_err(display)
}
fn windows(bars: &[Candle]) -> Result<(LaterSessionWindowV1, [LaterSessionWindowV1; 2]), String> {
    let first = day(bars.first().ok_or("later empty")?.ts_micros);
    let last = day(bars.last().ok_or("later empty")?.ts_micros);
    Ok((
        LaterSessionWindowV1::new(first, last)?,
        [
            LaterSessionWindowV1::new(first, first + 5)?,
            LaterSessionWindowV1::new(first + 6, last)?,
        ],
    ))
}
fn with_projection(
    test: impl FnOnce(&FixedTrainingFoldProjectionV1, &[TradeRow]) -> Result<(), String>,
) -> Result<(), String> {
    let (bars, column) = fixture(0, false)?;
    let (later, later_column) = fixture(16 * DAY, true)?;
    let key = InstrumentKey::index(Exchange::Nse, "NIFTY").map_err(display)?;
    let resolved = policy(crate::excursion::Side::Long)?
        .resolve_research_attested(series(&key, &bars)?)
        .map_err(display)?;
    let attested = resolved
        .attest_training(
            series(&key, &bars)?,
            &column,
            Horizon::bars(5).ok_or("horizon")?,
        )
        .map_err(display)?;
    let program = Expression::parse("30").map_err(|e| format!("{e:?}"))?;
    let training = resolved.evaluate_expression_with_attested(
        &attested,
        &run(&key, &bars, &program, Direction::Long)?,
    )?;
    let validated = resolved.validate_expression_evaluation(&training)?;
    let anchor = validated.later_period_anchor();
    let observed = resolved.evaluate_expression_oos(
        &anchor,
        series(&key, &later)?,
        &later_column,
        &run(&key, &later, &program, Direction::Long)?,
    )?;
    let (requested, parts) = windows(&later)?;
    let plan = FixedTrainingFoldPlanV1::new(&resolved, &anchor, requested, &parts, 2)?;
    let bound = plan.bind(&observed, 100_000)?;
    let mut chosen = None;
    for (ordinal, cell) in training.grid().cells.iter().enumerate() {
        let disposition =
            resolved.classify_expression_coordinate(&validated, Chosen::from_cell(cell))?;
        if let Some(selected) = disposition.selected() {
            chosen = Some((ordinal, selected.clone()));
            break;
        }
    }
    let (ordinal, selected) = chosen.ok_or("fixture has no authorized original coordinate")?;
    let (trades, proof) = bound.materialize_coordinate(&selected, ordinal)?;
    assert!(!trades.is_empty());
    test(&proof, &trades)
}

#[test]
fn complete_fixed_original_fold_projection_matches_independent_trade_partition()
-> Result<(), String> {
    with_projection(|proof, trades| {
        assert_eq!(proof.decided_folds(), 2);
        assert_eq!(proof.folds().len(), 2);
        let mut total = 0_i128;
        let mut profitable = 0;
        for fold in proof.folds() {
            let selected = trades
                .iter()
                .filter(|trade| {
                    let d = (i128::from(trade.exit_micros) + 19_800_000_000)
                        .div_euclid(i128::from(DAY));
                    d >= i128::from(fold.window.first_day())
                        && d <= i128::from(fold.window.last_day())
                })
                .collect::<Vec<_>>();
            let sum = selected
                .iter()
                .map(|trade| i128::from(trade.worst))
                .sum::<i128>();
            assert_eq!(fold.trades, selected.len() as u64);
            assert_eq!(i128::from(fold.return_paisa), sum);
            assert_eq!(
                fold.wins,
                selected.iter().filter(|trade| trade.worst > 0).count() as u64
            );
            assert_eq!(
                i128::from(fold.sessions),
                i128::from(fold.window.last_day()) - i128::from(fold.window.first_day()) + 1
            );
            total += sum;
            profitable += u64::from(sum > 0);
        }
        let quiet = proof.folds().last().ok_or("quiet fold")?;
        assert_eq!((quiet.trades, quiet.wins, quiet.return_paisa), (0, 0, 0));
        assert_eq!(i128::from(proof.aggregate_oos_paisa()?), total);
        assert_eq!(proof.profitable_oos_folds(), profitable);
        assert_ne!(proof.training_run_id(), proof.later_run_id());
        assert_ne!(proof.anchor_digest(), proof.later_digest());
        Ok(())
    })
}

#[test]
fn numeric_observer_roundtrips_exact_bytes_but_never_creates_an_authoring_token()
-> Result<(), String> {
    with_projection(|proof, _| {
        let bytes = proof.canonical_bytes(448)?;
        assert_eq!(bytes.len(), 448);
        assert!(proof.canonical_bytes(447).is_err());
        let observer = ObservedFixedTrainingFoldsV1::decode(&bytes, 448, 2)?;
        assert_eq!(observer.canonical_bytes(448)?, bytes);
        assert_eq!(proof.digest(448)?, hash(&bytes));
        assert_eq!(observer.plan_digest(), proof.plan_digest());
        assert_eq!(observer.anchor_digest(), proof.anchor_digest());
        assert_eq!(observer.selected_digest(), proof.selected_digest());
        assert_eq!(observer.training_run_id(), proof.training_run_id());
        assert_eq!(observer.later_run_id(), proof.later_run_id());
        assert_eq!(observer.later_digest(), proof.later_digest());
        assert_eq!(observer.resolution_digest(), proof.resolution_digest());
        assert_eq!(observer.ordinal(), proof.ordinal());
        assert_eq!(observer.folds(), proof.folds());
        assert_eq!(observer.decided_folds(), proof.decided_folds());
        assert_eq!(
            observer.profitable_oos_folds(),
            proof.profitable_oos_folds()
        );
        assert_eq!(
            observer.aggregate_oos_paisa()?,
            proof.aggregate_oos_paisa()?
        );
        assert_eq!(
            observer.execution_refusal_bits(),
            proof.execution_refusal_bits()
        );
        assert!(observer.requested().first_day() > observer.training_last_day());
        assert!(ObservedFixedTrainingFoldsV1::decode(&bytes, 447, 2).is_err());
        assert!(ObservedFixedTrainingFoldsV1::decode(&bytes, 448, 1).is_err());
        for end in 0..bytes.len() {
            assert!(
                ObservedFixedTrainingFoldsV1::decode(bytes.get(..end).ok_or("prefix")?, 448, 2)
                    .is_err(),
                "prefix{end}"
            );
        }
        for offset in [0, 7, 240, 248, 256, 264, 272, 304, 320, 336, 368, 440] {
            let mut changed = bytes.clone();
            let value = changed.get_mut(offset).ok_or("byte")?;
            *value ^= 128;
            assert!(
                ObservedFixedTrainingFoldsV1::decode(&changed, 448, 2).is_err(),
                "offset{offset}"
            );
        }
        let mut appended = bytes.clone();
        appended.push(0);
        assert!(ObservedFixedTrainingFoldsV1::decode(&appended, 449, 2).is_err());
        Ok(())
    })
}

#[test]
fn fixed_plan_rejects_training_overlap_missing_dates_empty_actual_folds_and_mapping_ceiling()
-> Result<(), String> {
    let (bars, column) = fixture(0, false)?;
    let (later, later_column) = fixture(16 * DAY, false)?;
    let key = InstrumentKey::index(Exchange::Nse, "NIFTY").map_err(display)?;
    let resolved = policy(crate::excursion::Side::Long)?
        .resolve_research_attested(series(&key, &bars)?)
        .map_err(display)?;
    let attested = resolved
        .attest_training(
            series(&key, &bars)?,
            &column,
            Horizon::bars(5).ok_or("horizon")?,
        )
        .map_err(display)?;
    let program = Expression::parse("30 | !30").map_err(|e| format!("{e:?}"))?;
    let training = resolved.evaluate_expression_with_attested(
        &attested,
        &run(&key, &bars, &program, Direction::Long)?,
    )?;
    let anchor = resolved
        .validate_expression_evaluation(&training)?
        .later_period_anchor();
    let (requested, parts) = windows(&later)?;
    assert!(LaterSessionWindowV1::new(2, 1).is_err());
    assert!(FixedTrainingFoldPlanV1::new(&resolved, &anchor, requested, &parts, 1).is_err());
    assert!(FixedTrainingFoldPlanV1::new(&resolved, &anchor, requested, &[], 2).is_err());
    let gap = [
        parts[0],
        LaterSessionWindowV1::new(parts[1].first + 1, parts[1].last)?,
    ];
    assert!(FixedTrainingFoldPlanV1::new(&resolved, &anchor, requested, &gap, 2).is_err());
    let overlap = [
        parts[0],
        LaterSessionWindowV1::new(parts[1].first - 1, parts[1].last)?,
    ];
    assert!(FixedTrainingFoldPlanV1::new(&resolved, &anchor, requested, &overlap, 2).is_err());
    let training_overlap =
        LaterSessionWindowV1::new(day(resolved.training_last_ts_micros()), requested.last)?;
    assert!(
        FixedTrainingFoldPlanV1::new(&resolved, &anchor, training_overlap, &[training_overlap], 1)
            .is_err()
    );
    let observed = resolved.evaluate_expression_oos(
        &anchor,
        series(&key, &later)?,
        &later_column,
        &run(&key, &later, &program, Direction::Long)?,
    )?;
    let plan = FixedTrainingFoldPlanV1::new(&resolved, &anchor, requested, &parts, 2)?;
    assert_eq!(plan.requested(), requested);
    assert_eq!(plan.windows(), parts);
    assert_eq!(plan.anchor(), &anchor);
    let bytes = (later.len() * size_of::<usize>() + 2 * size_of::<u64>()) as u64;
    assert!(plan.bind(&observed, bytes - 1).is_err());
    assert!(plan.bind(&observed, bytes).is_ok());
    let first = LaterSessionWindowV1::new(requested.first - 1, requested.first - 1)?;
    let all = LaterSessionWindowV1::new(first.first, requested.last)?;
    let empty = FixedTrainingFoldPlanV1::new(&resolved, &anchor, all, &[first, requested], 2)?;
    assert!(empty.bind(&observed, bytes).is_err());
    assert_ne!(empty.digest(), plan.digest());
    Ok(())
}

#[test]
fn exact_ist_day_preserves_both_timestamp_extremes_and_negative_epoch_boundaries() {
    for stamp in [
        i64::MIN,
        i64::MIN + DAY,
        -DAY - 19_800_000_000,
        -19_800_000_001,
        -19_800_000_000,
        -1,
        0,
        DAY - 19_800_000_001,
        DAY - 19_800_000_000,
        i64::MAX - DAY,
        i64::MAX,
    ] {
        assert_eq!(
            i128::from(day(stamp)),
            (i128::from(stamp) + 19_800_000_000).div_euclid(i128::from(DAY))
        );
    }
}

#[test]
fn both_sides_reject_foreign_full_program_anchors_selections_and_ordinals() -> Result<(), String> {
    let (bars, column) = fixture(0, false)?;
    let (later, later_column) = fixture(16 * DAY, false)?;
    let key = InstrumentKey::index(Exchange::Nse, "NIFTY").map_err(display)?;
    let program = Expression::parse("30").map_err(|e| format!("{e:?}"))?;
    let foreign_program = Expression::parse("30 | !30").map_err(|e| format!("{e:?}"))?;
    assert_eq!(program.referenced(), foreign_program.referenced());
    for (side, direction) in [
        (crate::excursion::Side::Long, Direction::Long),
        (crate::excursion::Side::Short, Direction::Short),
    ] {
        let resolved = policy(side)?
            .resolve_research_attested(series(&key, &bars)?)
            .map_err(display)?;
        let attested = resolved
            .attest_training(
                series(&key, &bars)?,
                &column,
                Horizon::bars(5).ok_or("horizon")?,
            )
            .map_err(display)?;
        let training = resolved.evaluate_expression_with_attested(
            &attested,
            &run(&key, &bars, &program, direction)?,
        )?;
        let validated = resolved.validate_expression_evaluation(&training)?;
        let anchor = validated.later_period_anchor();
        let foreign_training = resolved.evaluate_expression_with_attested(
            &attested,
            &run(&key, &bars, &foreign_program, direction)?,
        )?;
        let foreign_validated = resolved.validate_expression_evaluation(&foreign_training)?;
        let foreign_anchor = foreign_validated.later_period_anchor();
        let observed = resolved.evaluate_expression_oos(
            &anchor,
            series(&key, &later)?,
            &later_column,
            &run(&key, &later, &program, direction)?,
        )?;
        let foreign_observed = resolved.evaluate_expression_oos(
            &foreign_anchor,
            series(&key, &later)?,
            &later_column,
            &run(&key, &later, &foreign_program, direction)?,
        )?;
        let (requested, parts) = windows(&later)?;
        let plan = FixedTrainingFoldPlanV1::new(&resolved, &anchor, requested, &parts, 2)?;
        assert!(plan.bind(&foreign_observed, 100_000).is_err());
        let mut corrupted = anchor.clone();
        corrupted.digest[0] ^= 1;
        assert!(FixedTrainingFoldPlanV1::new(&resolved, &corrupted, requested, &parts, 2).is_err());
        let bound = plan.bind(&observed, 100_000)?;
        let mut exercised = false;
        for (ordinal, cell) in training.grid().cells.iter().enumerate() {
            let disposition =
                resolved.classify_expression_coordinate(&validated, Chosen::from_cell(cell))?;
            let foreign = resolved
                .classify_expression_coordinate(&foreign_validated, Chosen::from_cell(cell))?;
            if let Some((selected, wrong)) = disposition.selected().zip(foreign.selected()) {
                let (trades, proof) = bound.materialize_coordinate(selected, ordinal)?;
                assert!(!trades.is_empty());
                assert_eq!(
                    proof.aggregate_oos_paisa()?,
                    observed
                        .grid()
                        .cells
                        .get(ordinal)
                        .ok_or("cell")?
                        .pessimistic
                );
                assert!(bound.materialize_coordinate(wrong, ordinal).is_err());
                assert!(bound.materialize_coordinate(selected, usize::MAX).is_err());
                assert!(
                    bound
                        .materialize_coordinate(selected, usize::from(ordinal == 0))
                        .is_err()
                );
                exercised = true;
                break;
            }
        }
        assert!(
            exercised,
            "generated side has an authorized comparison coordinate"
        );
    }
    Ok(())
}

#[test]
fn defensive_numeric_overflow_and_zero_outcomes_cannot_be_encoded_as_success() -> Result<(), String>
{
    with_projection(|proof, _| {
        let mut changed = proof.clone();
        for fold in &mut changed.data.folds {
            fold.trades = 1;
            fold.wins = 1;
            fold.return_paisa = i64::MAX;
        }
        assert!(changed.aggregate_oos_paisa().is_err());
        assert!(changed.canonical_bytes(448).is_err());
        let mut changed = proof.clone();
        let fold = changed.data.folds.first_mut().ok_or("fold")?;
        fold.trades = 0;
        fold.wins = 0;
        fold.return_paisa = 1;
        assert!(changed.canonical_bytes(448).is_err());
        let mut changed = proof.clone();
        for fold in &mut changed.data.folds {
            fold.trades = 0;
            fold.wins = 0;
            fold.return_paisa = 0;
        }
        changed.data.refusal = ExecutionRefusalBitsV1::ZERO_TRADES;
        let bytes = changed.canonical_bytes(448)?;
        let observed = ObservedFixedTrainingFoldsV1::decode(&bytes, 448, 2)?;
        assert_eq!(observed.decided_folds(), 2);
        assert_eq!(observed.profitable_oos_folds(), 0);
        assert_eq!(observed.aggregate_oos_paisa()?, 0);
        assert_eq!(
            observed.execution_refusal_bits(),
            ExecutionRefusalBitsV1::ZERO_TRADES
        );
        Ok(())
    })
}

#[test]
fn actual_window_bounds_and_cold_records_follow_sealed_acceptance_authority() -> Result<(), String>
{
    let (bars, column) = fixture(0, false)?;
    let (later, later_column) = fixture(16 * DAY, false)?;
    assert!(later_column.census().warming > 0);
    assert!(
        later_column
            .sources()
            .first()
            .is_some_and(|source| *source > 0)
    );
    // Cold records are still valid accepted execution observations; they have
    // no sweep signal. An unavailable/corrupt record is a different contract.
    assert!((0..later.len()).all(|index| later_column.accepts(index)));
    let key = InstrumentKey::index(Exchange::Nse, "NIFTY").map_err(display)?;
    let resolved = policy(crate::excursion::Side::Long)?
        .resolve_research_attested(series(&key, &bars)?)
        .map_err(display)?;
    let attested = resolved
        .attest_training(
            series(&key, &bars)?,
            &column,
            Horizon::bars(5).ok_or("horizon")?,
        )
        .map_err(display)?;
    let program = Expression::parse("30").map_err(|why| format!("{why:?}"))?;
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
    let (requested, parts) = windows(&later)?;
    let plan = FixedTrainingFoldPlanV1::new(&resolved, &anchor, requested, &parts, 2)?;
    let bound = plan.bind(&observed, 100_000)?;
    let days = later
        .iter()
        .map(|bar| (i128::from(bar.ts_micros) + 19_800_000_000).div_euclid(i128::from(DAY)))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(bound.sessions.iter().sum::<u64>(), days.len() as u64);
    assert_eq!(
        days.len(),
        8,
        "cold execution days remain measured zero-capable sessions"
    );
    let shortened = LaterSessionWindowV1::new(requested.first_day(), requested.last_day() - 1)?;
    assert!(FixedTrainingFoldPlanV1::new(&resolved, &anchor, requested, &[shortened], 1).is_err());
    for (window, expected) in [
        (
            LaterSessionWindowV1::new(requested.first_day() + 1, requested.last_day())?,
            "precedes declared fold",
        ),
        (shortened, "outside declared folds"),
    ] {
        let plan = FixedTrainingFoldPlanV1::new(&resolved, &anchor, window, &[window], 1)?;
        let refusal = plan
            .bind(&observed, 100_000)
            .err()
            .ok_or("out-of-window source must refuse")?;
        assert!(refusal.contains(expected), "{refusal}");
    }
    let mut corrupt = later.clone();
    corrupt.first_mut().ok_or("first actual record")?.volume = -1;
    let mut evaluator = Evaluator::with_calendar(
        Widths::pinned().map_err(display)?,
        indicators::vwap::Availability::Absent,
        indicators::pattern::Thresholds::CLASSICAL,
        Calendar::all_regular(),
    );
    let invalid = Column::build(&corrupt, &mut evaluator);
    assert!(!invalid.accepts(0));
    assert_eq!(invalid.census().negative_volume, 1);
    assert!(
        resolved
            .evaluate_expression_oos(
                &anchor,
                series(&key, &corrupt)?,
                &invalid,
                &run(&key, &corrupt, &program, Direction::Long)?
            )
            .is_err(),
        "rejected execution cannot mint the opaque later value required by bind"
    );
    Ok(())
}

#[test]
fn empty_numeric_observation_refuses_even_with_valid_version_and_complete_header()
-> Result<(), String> {
    with_projection(|proof, _| {
        let bytes = proof.canonical_bytes(448)?;
        let mut empty = bytes.get(..320).ok_or("full numeric header")?.to_vec();
        empty
            .get_mut(240..248)
            .ok_or("count")?
            .copy_from_slice(&0_u64.to_le_bytes());
        assert_eq!(empty.len(), 320);
        assert_eq!(empty.get(..8), Some(b"BRXFVL01".as_slice()));
        let refusal = ObservedFixedTrainingFoldsV1::decode(&empty, 320, usize::MAX)
            .err()
            .ok_or("empty proof must refuse")?;
        assert!(refusal.contains("count/length/padding"));
        let mut reversed = bytes;
        reversed
            .get_mut(280..288)
            .ok_or("requested first")?
            .copy_from_slice(&i64::MAX.to_le_bytes());
        assert!(ObservedFixedTrainingFoldsV1::decode(&reversed, 448, 2).is_err());
        Ok(())
    })
}
