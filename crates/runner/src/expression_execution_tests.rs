#![cfg(test)]
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
        )?;
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
        )?;
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
    )?;
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
    )?;
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

/// **Every coordinate of a program replays over one program walk
/// (W3-runner2-1, D-1833).** The per-ordinal door walks the program and
/// measures every crossing for each coordinate; the replay walks once per
/// program. Counted: 30 coordinates over three programs cost 30 walks through
/// the per-ordinal door and 3 through the replay, and every coordinate's rows
/// are byte-identical. `30 & !30` is never true, so its walk is empty and the
/// empty-fold rule is exercised; an absent ordinal and a foreign attestation
/// refuse exactly as the per-ordinal door does.
#[test]
fn every_coordinate_of_a_program_replays_over_one_walk() -> Result<(), String> {
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
    let other = resolved
        .attest_training(
            series(&key, &bars)?,
            &column,
            Horizon::bars(4).ok_or("horizon")?,
        )
        .map_err(display)?;
    let (mut per_ordinal, mut replayed, mut coordinates, mut traded) = (0, 0, 0, 0);
    for text in ["30 & 44", "30 | 44", "30 & !30"] {
        let program = expression(text)?;
        let run = ExpressionExecutionRunV1::new(
            &run(&key, &bars, &program, Direction::Long),
            &program,
            &bars,
            None,
        )?;
        let evaluated = resolved.evaluate_expression_with_attested(&attested, &run)?;
        let validated = resolved.validate_expression_evaluation(&evaluated)?;
        let cells = evaluated.grid().cells.len();
        let before = crate::grid::expression_walks_on_this_thread();
        let one_off = (0..cells)
            .map(|ordinal| {
                resolved.materialize_expression_coordinate(&attested, &validated, ordinal)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let middle = crate::grid::expression_walks_on_this_thread();
        let replay = resolved.expression_coordinate_replay(&attested, &validated);
        let shared = (0..cells)
            .map(|ordinal| replay.materialize(ordinal))
            .collect::<Result<Vec<_>, _>>()?;
        let after = crate::grid::expression_walks_on_this_thread();
        assert_eq!(shared, one_off, "{text}: rows differ");
        assert_eq!(
            replay.materialize(cells),
            resolved.materialize_expression_coordinate(&attested, &validated, cells)
        );
        assert_eq!(
            replay.materialize(cells).err().as_deref(),
            Some("expression replay ordinal is absent")
        );
        let foreign = resolved.expression_coordinate_replay(&other, &validated);
        assert_eq!(
            foreign.materialize(0).err().as_deref(),
            Some("expression replay does not match authenticated training inputs")
        );
        assert_eq!(
            foreign.materialize(0),
            resolved.materialize_expression_coordinate(&other, &validated, 0)
        );
        assert_eq!(
            crate::grid::expression_walks_on_this_thread(),
            after,
            "a refusal before the walk walks nothing"
        );
        per_ordinal += middle - before;
        replayed += after - middle;
        coordinates += cells;
        traded += shared.iter().filter(|rows| !rows.is_empty()).count();
    }
    assert_eq!(coordinates, 30);
    assert!(traded > 0, "at least one coordinate must trade");
    assert_eq!((per_ordinal, replayed), (30, 3));
    Ok(())
}

/// The replay refuses a selected cell its walk does not reproduce, on both
/// the traded and the empty-walk path (D-1833).
#[test]
fn an_expression_replay_refuses_a_forged_cell() -> Result<(), String> {
    let (bars, column) = fixture()?;
    let facts = crate::trade::SliceFacts::of(&bars, &column);
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
    for (text, refusal) in [
        (
            "30 | 44",
            "selected expression cell differs from exact replay",
        ),
        (
            "30 & !30",
            "selected expression empty cell differs from exact replay",
        ),
    ] {
        let program = expression(text)?;
        let run = ExpressionExecutionRunV1::new(
            &run(&key, &bars, &program, Direction::Long),
            &program,
            &bars,
            None,
        )?;
        let evaluated = resolved.evaluate_expression_with_attested(&attested, &run)?;
        let grid = evaluated.grid();
        let replay = crate::grid::ExpressionCellReplay::prepare(
            &bars,
            &column,
            &program,
            Horizon::bars(5).ok_or("horizon")?,
            crate::excursion::Side::Long,
            grid,
            &facts,
        )?;
        for cell in &grid.cells {
            assert_eq!(
                replay.materialize(cell)?,
                crate::grid::materialize_expression_cell(
                    &bars,
                    &column,
                    &program,
                    Horizon::bars(5).ok_or("horizon")?,
                    crate::excursion::Side::Long,
                    grid,
                    cell,
                )?
            );
            let mut forged = *cell;
            forged.trades = forged.trades.wrapping_add(1);
            assert_eq!(replay.materialize(&forged).err().as_deref(), Some(refusal));
        }
    }
    Ok(())
}
