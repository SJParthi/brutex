#![cfg(test)]
#![allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes: a fixture step that fails must fail the test loudly"
)]
//! Generated fixture checks for program-aware resolved-coordinate authority.
use super::*;
use crate::exit_grid_policy::{
    ExecutionResolutionV1, ExecutionSeriesV1, ExitGridPolicyV1, ExitGridSelectorV1, ForcedStopV1,
    RangeResolutionV1, RatioLimitsV1, RationalPercentileV1, RungPlanV1,
    printed_ohlcv_cost_model_id_v2,
};
use crate::identity::{Direction, Params};
use brutex_core::instrument::{Exchange, InstrumentKey};
use indicators::column::Column;
use indicators::evaluator::{Calendar, Evaluator, Widths};

fn fixture() -> Result<(Vec<Candle>, Column), String> {
    let bars = crate::synthetic::sessions(8);
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

fn policy(side: crate::excursion::Side) -> Result<ExitGridPolicyV1, String> {
    let percentile = RationalPercentileV1::new(1, 2).map_err(display)?;
    ExitGridPolicyV1::new(
        ExecutionResolutionV1::OneMinuteOhlcv,
        RangeResolutionV1::PpmCeiling,
        side,
        RungPlanV1::new(vec![percentile], vec![percentile], vec![percentile], 1)
            .map_err(display)?,
        RatioLimitsV1::new(1, 1_000_000, 1).map_err(display)?,
        32,
        ExitGridSelectorV1::PessimisticTotal,
        printed_ohlcv_cost_model_id_v2(),
        ForcedStopV1::Disabled,
        u64::MAX,
        u64::MAX,
    )
    .map_err(display)
}

fn key() -> Result<InstrumentKey, String> {
    InstrumentKey::index(Exchange::Nse, "NIFTY").map_err(display)
}

fn expression(raw: &str) -> Result<Expression, String> {
    Expression::parse(raw).map_err(|why| format!("{why:?}"))
}

fn run<'a>(
    key: &'a InstrumentKey,
    bars: &[Candle],
    program: &Expression,
    side: Direction,
) -> Run<'a> {
    Run {
        mask: program.referenced(),
        direction: side,
        instrument: key,
        timeframe: "1min",
        params: Params::of(engine::Ladder::with_min_hits(1)),
        data_digest: crate::identity::data_digest_with_execution(bars, None),
        commit: "generated-expression-authority-fixture",
        feed: "zerodha",
    }
}

fn series<'a>(key: &'a InstrumentKey, bars: &'a [Candle]) -> Result<ExecutionSeriesV1<'a>, String> {
    ExecutionSeriesV1::new(
        key,
        "zerodha",
        "generated-expression-authority-fixture",
        [7; 32],
        bars,
    )
    .map_err(display)
}

#[test]
fn conjunction_uses_identical_exact_grid_and_every_coordinate_replays() -> Result<(), String> {
    let (bars, column) = fixture()?;
    let key = key()?;
    let program = expression("30 & 44")?;
    for (side, direction) in [
        (crate::excursion::Side::Long, Direction::Long),
        (crate::excursion::Side::Short, Direction::Short),
    ] {
        let resolved = policy(side)?
            .resolve_attested(series(&key, &bars)?)
            .map_err(display)?;
        let attested = resolved
            .attest_training(
                series(&key, &bars)?,
                &column,
                Horizon::bars(5).ok_or("horizon")?,
            )
            .map_err(display)?;
        let run = run(&key, &bars, &program, direction);
        let mask_run = ExecutionRunV1::new(&run, &bars, None).map_err(display)?;
        let expression_run = ExpressionExecutionRunV1::new(&run, &program, &bars, None)?;
        assert_ne!(expression_run.run_id(), mask_run.run_id());
        let legacy = resolved
            .evaluate_with_attested(&attested, mask_run)
            .map_err(display)?;
        let evaluated = resolved.evaluate_expression_with_attested(&attested, &expression_run)?;
        assert_eq!(evaluated.grid(), legacy.grid());
        let validated = resolved.validate_expression_evaluation(&evaluated)?;
        let mut admitted = 0;
        for (ordinal, cell) in evaluated.grid().cells.iter().enumerate() {
            let disposition =
                resolved.classify_expression_coordinate(&validated, Chosen::from_cell(cell))?;
            assert_eq!(disposition.ordinal(), ordinal);
            admitted += usize::from(disposition.selected().is_some());
            let rows =
                resolved.materialize_expression_coordinate(&attested, &validated, ordinal)?;
            assert_eq!(u64::try_from(rows.len()).map_err(display)?, cell.trades);
        }
        assert!(admitted > 0);
        assert_eq!(evaluated.grid().cells.len() as u64, resolved.cell_count());
    }
    Ok(())
}

/// W3-runner2-0 (D-0741): programs priced and coordinates materialized over
/// one attestation derive the slice facts only in that attestation.
#[test]
fn programs_and_coordinates_over_one_attestation_derive_the_slice_facts_once() -> Result<(), String>
{
    let (bars, column) = fixture()?;
    let key = key()?;
    let resolved = policy(crate::excursion::Side::Long)?
        .resolve_attested(series(&key, &bars)?)
        .map_err(display)?;
    let before = crate::trade::slice_facts_derived_on_this_thread();
    let attested = resolved
        .attest_training(
            series(&key, &bars)?,
            &column,
            Horizon::bars(5).ok_or("horizon")?,
        )
        .map_err(display)?;
    let attested_at = crate::trade::slice_facts_derived_on_this_thread();
    let mut coordinates = 0_u64;
    for text in ["30 & 44", "30 | 44", "30 & !44"] {
        let program = expression(text)?;
        let run = ExpressionExecutionRunV1::new(
            &run(&key, &bars, &program, Direction::Long),
            &program,
            &bars,
            None,
        )
        .expect("a fixture step that must succeed");
        let evaluated = resolved.evaluate_expression_with_attested(&attested, &run)?;
        let validated = resolved.validate_expression_evaluation(&evaluated)?;
        for (ordinal, cell) in evaluated.grid().cells.iter().enumerate() {
            let rows =
                resolved.materialize_expression_coordinate(&attested, &validated, ordinal)?;
            assert_eq!(u64::try_from(rows.len()).map_err(display)?, cell.trades);
            coordinates += 1;
        }
    }
    assert_eq!(coordinates, 30);
    assert_eq!(
        crate::trade::slice_facts_derived_on_this_thread() - attested_at,
        0,
        "{coordinates} coordinates over three programs must not re-derive the slice facts"
    );
    assert_eq!(attested_at - before, 1, "attestation derives once");
    Ok(())
}

#[test]
fn same_referenced_bits_do_not_collapse_or_not_or_unknown_programs() -> Result<(), String> {
    let (bars, column) = fixture()?;
    let key = key()?;
    let resolved = policy(crate::excursion::Side::Long)?
        .resolve_attested(series(&key, &bars)?)
        .map_err(display)?;
    let attested = resolved
        .attest_training(
            series(&key, &bars)?,
            &column,
            Horizon::bars(5).ok_or("horizon")?,
        )
        .map_err(display)?;
    let mut seen = Vec::new();
    for text in ["30 & 44", "30 | 44", "30 & !44"] {
        let program = expression(text)?;
        let run = ExpressionExecutionRunV1::new(
            &run(&key, &bars, &program, Direction::Long),
            &program,
            &bars,
            None,
        )
        .expect("a fixture step that must succeed");
        let evaluated = resolved.evaluate_expression_with_attested(&attested, &run)?;
        let validated = resolved.validate_expression_evaluation(&evaluated)?;
        assert_eq!(
            run.program().referenced(),
            expression("30 & 44")?.referenced()
        );
        seen.push((
            run.run_id(),
            validated.evaluation_digest(),
            evaluated.summary().hits,
        ));
    }
    for pair in seen.windows(2) {
        let [left, right] = pair else {
            return Err("pair".to_owned());
        };
        assert_ne!(left.0, right.0);
        assert_ne!(left.1, right.1);
        assert_ne!(left.2, right.2);
    }
    let unknown = expression("146 | !146")?;
    let run = ExpressionExecutionRunV1::new(
        &run(&key, &bars, &unknown, Direction::Long),
        &unknown,
        &bars,
        None,
    )
    .expect("a fixture step that must succeed");
    let evaluated = resolved.evaluate_expression_with_attested(&attested, &run)?;
    assert!(evaluated.summary().unknown > 0);
    assert_eq!(evaluated.summary().hits, 0);
    assert!(evaluated.grid().cells.iter().all(|cell| cell.trades == 0));
    let validated = resolved.validate_expression_evaluation(&evaluated)?;
    for (ordinal, cell) in evaluated.grid().cells.iter().enumerate() {
        let disposition =
            resolved.classify_expression_coordinate(&validated, Chosen::from_cell(cell))?;
        assert!(disposition.selected().is_none());
        assert!(
            resolved
                .materialize_expression_coordinate(&attested, &validated, ordinal)?
                .is_empty()
        );
        let mut corrupted = *cell;
        corrupted.pessimistic = 1;
        assert!(
            crate::grid::materialize_expression_cell(
                &bars,
                &column,
                &unknown,
                attested.horizon,
                crate::excursion::Side::Long,
                evaluated.grid(),
                &corrupted
            )
            .is_err()
        );
        assert!(
            disposition
                .refusal_bits()
                .contains(ExecutionRefusalBitsV1::ZERO_TRADES)
        );
    }
    Ok(())
}

#[test]
fn source_terms_foreign_resolutions_and_incomplete_populations_refuse() -> Result<(), String> {
    let (bars, column) = fixture()?;
    let key = key()?;
    let program = expression("30 | !44")?;
    let mut wrong = run(&key, &bars, &program, Direction::Long);
    wrong.mask = vocab::ConditionMask::ZERO;
    assert!(ExpressionExecutionRunV1::new(&wrong, &program, &bars, None).is_err());
    wrong.mask = program.referenced();
    wrong.data_digest = [9; 32];
    assert!(ExpressionExecutionRunV1::new(&wrong, &program, &bars, None).is_err());
    let long = policy(crate::excursion::Side::Long)?
        .resolve_attested(series(&key, &bars)?)
        .map_err(display)?;
    let short = policy(crate::excursion::Side::Short)?
        .resolve_attested(series(&key, &bars)?)
        .map_err(display)?;
    let attested = long
        .attest_training(
            series(&key, &bars)?,
            &column,
            Horizon::bars(5).ok_or("horizon")?,
        )
        .map_err(display)?;
    let run = ExpressionExecutionRunV1::new(
        &run(&key, &bars, &program, Direction::Long),
        &program,
        &bars,
        None,
    )
    .expect("a fixture step that must succeed");
    assert!(
        short
            .evaluate_expression_with_attested(&attested, &run)
            .is_err()
    );
    let evaluated = long.evaluate_expression_with_attested(&attested, &run)?;
    assert!(short.validate_expression_evaluation(&evaluated).is_err());
    let validated = long.validate_expression_evaluation(&evaluated)?;
    assert!(
        long.classify_expression_coordinate(
            &validated,
            Chosen {
                stop: Some(usize::MAX),
                target: None,
                tsl: None,
                ttp: None
            }
        )
        .is_err()
    );
    assert!(
        long.materialize_expression_coordinate(&attested, &validated, usize::MAX)
            .is_err()
    );
    let mut truncated = evaluated.clone();
    assert!(truncated.grid.cells.pop().is_some());
    assert!(long.validate_expression_evaluation(&truncated).is_err());
    let mut reordered = evaluated;
    let (first, rest) = reordered.grid.cells.split_first_mut().ok_or("first cell")?;
    std::mem::swap(first, rest.first_mut().ok_or("second cell")?);
    assert!(long.validate_expression_evaluation(&reordered).is_err());
    Ok(())
}

/// An evaluation, the coordinate offsets its validation computed, and the
/// validation digest.
type EvaluatedFixture = (EvaluatedExpressionGridV1, Vec<Option<usize>>, [u8; 32]);

/// BASE-COV-LIBS-LS2: the fixture's long resolution, its attestation over the
/// fixture column, and an evaluation of `30 & 44` with its validation offsets.
fn evaluated_fixture(
    bars: &[Candle],
    column: &Column,
    key: &InstrumentKey,
) -> Result<EvaluatedFixture, String> {
    let program = expression("30 & 44")?;
    let resolved = policy(crate::excursion::Side::Long)?
        .resolve_attested(series(key, bars)?)
        .map_err(display)?;
    let attested = resolved
        .attest_training(
            series(key, bars)?,
            column,
            Horizon::bars(5).ok_or("horizon")?,
        )
        .map_err(display)?;
    let run = ExpressionExecutionRunV1::new(
        &run(key, bars, &program, Direction::Long),
        &program,
        bars,
        None,
    )?;
    let evaluated = resolved.evaluate_expression_with_attested(&attested, &run)?;
    let validated = resolved.validate_expression_evaluation(&evaluated)?;
    let (offsets, digest) = (validated.offsets.clone(), validated.digest);
    Ok((evaluated, offsets, digest))
}

/// THE DAILY-REFERENCE AND HOISTED-DIGEST RUN DOORS SEAL THE SAME PROGRAM
/// IDENTITY as the plain door when their data term is the plain one, refuse a
/// run whose data term is not theirs, and every read face of an evaluation,
/// its validation, a disposition and a selected exit answers what it bound.
#[test]
fn every_run_door_and_read_face_answers_what_it_bound() -> Result<(), String> {
    let (bars, column) = fixture()?;
    let key = key()?;
    let program = expression("30 & 44")?;
    let plain = run(&key, &bars, &program, Direction::Long);
    let sealed = ExpressionExecutionRunV1::new(&plain, &program, &bars, None)?;
    let digests = crate::exit_grid_policy::ExecutionDigestsV1::of(&bars, None);
    let hoisted = ExpressionExecutionRunV1::with_digests(&plain, &program, &digests)?;
    assert_eq!(hoisted.run_id(), sealed.run_id());
    assert_eq!(hoisted.execution_digest(), sealed.execution_digest());
    assert_eq!(
        sealed.execution_digest(),
        crate::identity::data_digest(&bars)
    );
    let reference = DailyReferenceBinding {
        daily_bars: &[],
        eligibility: &[],
        schema: 1,
        eligibility_policy: 1,
        gap_overlay_policy: 1,
        excluded_ist_days: &[],
        daily_integrity: crate::identity::ReferenceIntegrity::UnverifiedNoReceipt,
        minute_integrity: crate::identity::ReferenceIntegrity::UnverifiedNoReceipt,
        swept_series_calendar_policy: 1,
    };
    let referenced = Run {
        data_digest: crate::identity::data_digest_with_daily_reference(&bars, &bars, reference)
            .map_err(|why| format!("{why:?}"))?,
        ..run(&key, &bars, &program, Direction::Long)
    };
    let daily = ExpressionExecutionRunV1::new_with_daily_reference(
        &referenced,
        &program,
        &bars,
        &bars,
        &bars,
        reference,
    )?;
    assert_eq!(daily.program(), &program);
    assert_ne!(
        daily.run_id(),
        sealed.run_id(),
        "the reference is in the data term"
    );
    assert!(
        ExpressionExecutionRunV1::new_with_daily_reference(
            &plain, &program, &bars, &bars, &bars, reference
        )
        .is_err(),
        "the plain data term is not the referenced one"
    );

    let (evaluated, offsets, digest) = evaluated_fixture(&bars, &column, &key)?;
    let resolved = policy(crate::excursion::Side::Long)?
        .resolve_attested(series(&key, &bars)?)
        .map_err(display)?;
    assert_eq!(evaluated.resolution_digest(), resolved.digest());
    assert_eq!(evaluated.horizon(), Horizon::bars(5).ok_or("horizon")?);
    assert_eq!(evaluated.side(), crate::excursion::Side::Long);
    assert_eq!(
        evaluated.column_digest(),
        crate::exit_grid_policy::column_digest_v1(&column)
    );
    let validated = ValidatedExpressionGridV1 {
        evaluated: &evaluated,
        digest,
        offsets: offsets.clone(),
    };
    assert!(core::ptr::eq(
        validated.evaluation(),
        core::ptr::from_ref(&evaluated)
    ));
    let selected = evaluated
        .grid()
        .cells
        .iter()
        .find_map(|cell| {
            resolved
                .classify_expression_coordinate(&validated, Chosen::from_cell(cell))
                .ok()?
                .selected()
                .cloned()
        })
        .ok_or("the fixture admits a coordinate")?;
    let disposition = resolved.classify_expression_coordinate(&validated, selected.coordinate())?;
    assert_eq!(disposition.coordinate(), selected.coordinate());
    assert_eq!(disposition.digest(), selected.digest());
    assert_eq!(
        Chosen::from_cell(selected.training_cell()),
        selected.coordinate()
    );
    Ok(())
}

/// EVERY EXPRESSION DOOR REFUSES AN EVALUATION THAT IS NOT ITS OWN: one of the
/// other side, of another resolution, with its cells out of canonical order,
/// or replayed under another attestation's horizon, column or evaluator.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one refusal per door and per moved input, read as one list"
)]
fn every_expression_door_refuses_an_evaluation_that_is_not_its_own() -> Result<(), String> {
    let (bars, column) = fixture()?;
    let key = key()?;
    let (evaluated, offsets, digest) = evaluated_fixture(&bars, &column, &key)?;
    let resolved = policy(crate::excursion::Side::Long)?
        .resolve_attested(series(&key, &bars)?)
        .map_err(display)?;
    let attested = resolved
        .attest_training(
            series(&key, &bars)?,
            &column,
            Horizon::bars(5).ok_or("horizon")?,
        )
        .map_err(display)?;
    let mut other_side = evaluated.clone();
    other_side.side = crate::excursion::Side::Short;
    assert!(
        resolved
            .validate_expression_evaluation(&other_side)
            .is_err()
    );
    let around = |moved: &EvaluatedExpressionGridV1| -> Result<Vec<TradeRow>, String> {
        let validated = ValidatedExpressionGridV1 {
            evaluated: moved,
            digest,
            offsets: offsets.clone(),
        };
        resolved.materialize_expression_coordinate(&attested, &validated, 0)
    };
    assert!(around(&evaluated).is_ok());
    let first = Chosen::from_cell(evaluated.grid().cells.first().ok_or("cells")?);
    let mut foreign = evaluated.clone();
    foreign.resolution[0] ^= 1;
    let foreign_validated = ValidatedExpressionGridV1 {
        evaluated: &foreign,
        digest,
        offsets: offsets.clone(),
    };
    assert!(
        resolved
            .classify_expression_coordinate(&foreign_validated, first)
            .is_err()
    );
    let mut reordered = evaluated.clone();
    reordered.grid.cells.swap(0, 1);
    let reordered_validated = ValidatedExpressionGridV1 {
        evaluated: &reordered,
        digest,
        offsets: offsets.clone(),
    };
    assert_eq!(
        resolved
            .classify_expression_coordinate(&reordered_validated, first)
            .map(|_| ()),
        Err("expression coordinate ordinal changed".to_owned())
    );
    let mut horizon = evaluated.clone();
    horizon.horizon = Horizon::bars(6).ok_or("horizon")?;
    let mut column_digest = evaluated.clone();
    column_digest.column_digest[0] ^= 1;
    let mut evaluator = Evaluator::with_calendar(
        Widths::pinned().map_err(display)?,
        indicators::vwap::Availability::Present,
        indicators::pattern::Thresholds::CLASSICAL,
        Calendar::all_regular(),
    );
    let present = Column::build(&bars, &mut evaluator);
    let mut spec = evaluated.clone();
    spec.evaluation_spec = present
        .evaluation_spec_token()
        .ok_or("a built column has a spec")?;
    // An attestation of the other side's resolution, with this evaluation.
    let short = policy(crate::excursion::Side::Short)?
        .resolve_attested(series(&key, &bars)?)
        .map_err(display)?;
    let short_attested = short
        .attest_training(
            series(&key, &bars)?,
            &column,
            Horizon::bars(5).ok_or("horizon")?,
        )
        .map_err(display)?;
    let own = ValidatedExpressionGridV1 {
        evaluated: &evaluated,
        digest,
        offsets: offsets.clone(),
    };
    assert_eq!(
        resolved.materialize_expression_coordinate(&short_attested, &own, 0),
        Err("expression replay does not match authenticated training inputs".to_owned()),
        "another resolution's attestation"
    );
    for (what, moved) in [
        ("another resolution", &foreign),
        ("another horizon", &horizon),
        ("another column", &column_digest),
        ("another evaluator", &spec),
    ] {
        assert_eq!(
            around(moved),
            Err("expression replay does not match authenticated training inputs".to_owned()),
            "{what}"
        );
    }
    Ok(())
}

/// BASE-COV-LIBS-LS2 (5 Nov): AN EXPRESSION PRICED OVER A RESEALED RESOLUTION
/// THAT CANNOT ENUMERATE ITS OWN GRID IS REFUSED BY THE GRID'S POPULATION
/// CHECK, as the mask door refuses it: the digest is recomputed, so the
/// runtime integrity check passes, and the stored cell count is one past what
/// the resolution's ladders enumerate.
#[test]
fn an_expression_over_a_resolution_that_cannot_enumerate_its_grid_is_refused() -> Result<(), String>
{
    let (bars, column) = fixture()?;
    let key = key()?;
    let program = expression("30 & 44")?;
    let honest = policy(crate::excursion::Side::Long)?
        .resolve_attested(series(&key, &bars)?)
        .map_err(display)?;
    let resolved = crate::exit_grid_policy::resolution_tamper::with_one_more_cell(&honest);
    let horizon = Horizon::bars(5).ok_or("horizon")?;
    let attested = resolved
        .attest_training(series(&key, &bars)?, &column, horizon)
        .map_err(display)?;
    let run = run(&key, &bars, &program, Direction::Long);
    let expression_run = ExpressionExecutionRunV1::new(&run, &program, &bars, None)?;
    let refusal = crate::exit_grid_policy::ExitGridErrorV1::ReplayCellCountMismatch {
        expected: honest.cell_count() + 1,
        actual: honest.cell_count(),
    };
    assert_eq!(
        resolved
            .evaluate_expression_with_attested(&attested, &expression_run)
            .map(|_| ()),
        Err(refusal.to_string())
    );
    let mask_run = ExecutionRunV1::new(&run, &bars, None).map_err(display)?;
    assert_eq!(
        resolved
            .evaluate_with_attested(&attested, mask_run)
            .map(|_| ()),
        Err(refusal)
    );
    // The honest resolution prices the same program over the same bars.
    let attested = honest
        .attest_training(series(&key, &bars)?, &column, horizon)
        .map_err(display)?;
    assert!(
        honest
            .evaluate_expression_with_attested(&attested, &expression_run)
            .is_ok()
    );
    Ok(())
}
