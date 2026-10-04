#![cfg(test)]
//! Generated, source-bound tests; none of these thresholds is a research policy.
use super::*;
use runner::admission::{AdmissionPolicyDraftV1, AdmissionReasonV1};

pub(crate) fn policy(max_mae: u64) -> Result<AdmissionPolicyV1, String> {
    policy_with_folds(max_mae, 1)
}

pub(crate) fn policy_with_folds(max_mae: u64, folds: u64) -> Result<AdmissionPolicyV1, String> {
    AdmissionPolicyV1::new(AdmissionPolicyDraftV1 {
        min_support_hits: Some(1),
        min_independent_sessions: Some(1),
        min_trades: Some(1),
        max_mae_paisa: Some(max_mae),
        min_worst_reward_risk_ppm: Some(0),
        min_win_rate_ppm: Some(0),
        min_wilson_win_rate_ppm: Some(0),
        min_return_drawdown_ppm: Some(0),
        min_weakest_period_return_paisa: Some(i64::MIN),
        max_pbo_ppm: Some(1_000_000),
        max_fwer_p_value_ppm: Some(1_000_000),
        max_spa_p_value_ppm: Some(1_000_000),
        min_decided_folds: Some(folds),
        max_ambiguous_fill_rate_ppm: Some(1_000_000),
        max_gap_affected_rate_ppm: Some(1_000_000),
        max_session_concentration_ppm: Some(1_000_000),
        max_largest_trade_profit_share_ppm: Some(1_000_000),
        max_drawdown_paisa: Some(u64::MAX),
        max_worst_trade_loss_paisa: Some(u64::MAX),
        max_losing_trade_rate_ppm: Some(1_000_000),
        max_losing_trades: Some(u64::MAX),
        min_pessimistic_profit_paisa: Some(i64::MIN),
        min_winning_trades: Some(0),
        min_average_win_paisa: Some(0),
        max_average_loss_paisa: Some(u64::MAX),
        min_profit_factor_ppm: Some(0),
        max_consecutive_losing_streak: Some(u64::MAX),
        min_consecutive_winning_streak: Some(0),
        min_bootstrap_draws: Some(1),
        min_bootstrap_strategies: Some(1),
        min_bootstrap_periods: Some(1),
        min_pbo_contributing_folds: Some(1),
        max_pbo_unrankable_folds: Some(u64::MAX),
        min_profitable_oos_folds: Some(1),
        min_oos_pessimistic_return_paisa: Some(i64::MIN),
        max_white_reality_p_value_ppm: Some(1_000_000),
        require_white_reality_rejection: Some(true),
        max_romano_wolf_p_value_ppm: Some(1_000_000),
        require_romano_wolf_rejection: Some(true),
    })
    .map_err(|why| format!("generated admission policy: {why:?}"))
}

fn statistics(
    fixture: &super::super::super::tests::Fixture,
) -> Result<CommittedBooleanStatisticsV1, String> {
    let mut programs = super::super::super::tests::programs()?;
    // Cash VWAP is available, so its tautology is not an empty coordinate.
    // Retain a genuinely non-hitting expression in the requested catalog.
    programs
        .push(runner::expression::Expression::parse("30 & !30").map_err(|why| format!("{why:?}"))?);
    let source = fixture.produce_span("RELIANCE", &programs, 7, 8)?;
    super::super::produce(
        &fixture.output,
        vec![source],
        crate::population_statistics_v2::PopulationStatisticsProcedureV2::new(7, 49, 2)?,
        super::super::Bounds {
            families: 1,
            candidates: 1024,
            observations: 50_000,
            bootstrap_work: 10_000_000,
            split_work: 10_000_000,
            memory_bytes: 64 * 1024 * 1024,
            bytes: 4 * 1024 * 1024,
        },
    )
}

/// Three priced trades on two IST days: +10, -4 and a break-even 0.
fn three_trade_coordinate() -> Result<BooleanCoordinateV1, String> {
    let trades = [10_i64, -4, 0]
        .into_iter()
        .zip(0_i64..)
        .map(|(worst, slot)| {
            let entry =
                1_746_157_500_000_000 + (slot / 2) * 86_400_000_000 + (slot % 2) * 120_000_000;
            let index = usize::try_from(slot).unwrap_or_default();
            runner::grid::TradeRow {
                signal_bar: index * 2,
                entry_bar: index * 2 + 1,
                exit_bar: index * 2 + 2,
                entry_micros: entry,
                exit_micros: entry + 60_000_000,
                best: worst,
                worst,
                adverse: 0,
                adverse_paisa: 0,
                favourable: 0,
                favourable_paisa: 0,
            }
        })
        .collect();
    Ok(BooleanCoordinateV1 {
        identity: [8; 32],
        program_index: 0,
        run: [9; 32],
        side: runner::excursion::Side::Long,
        ordinal: 0,
        cell: runner::grid::Cell {
            trades: 3,
            wins: 1,
            pessimistic: 6,
            optimistic: 6,
            gross_win: 10,
            gross_loss: -4,
            best_trade: 10,
            min_win: 10,
            worst_trade: -4,
            max_drawdown: 4,
            max_winning_streak: 1,
            max_losing_streak: 2,
            timed_out: 3,
            ambiguous_bars: 1,
            gapped: 1,
            ..runner::grid::Cell::default()
        },
        refusal: runner::exit_grid_policy::ExecutionRefusalBitsV1::from_bits(0).ok_or("refusal")?,
        periods: vec![],
        trades,
        selected: None,
        summary: runner::expression::Summary {
            evaluated: 3,
            hits: 3,
            misses: 0,
            unknown: 0,
        },
        support_sessions: 2,
    })
}

#[test]
fn boolean_base_values_round_max_gated_rates_up() -> Result<(), String> {
    // p2bool-1, D-1990: 1 of 3 and 2 of 3 are inexact in ppm. A floor put
    // each a fraction BELOW its true value, onto a cap the true value exceeds.
    let values = base_values(&three_trade_coordinate()?)?;
    assert_eq!(
        values.ambiguous_fill_rate_ppm,
        ObservedU64V1::Measured(333_334)
    );
    assert_eq!(
        values.gap_affected_rate_ppm,
        ObservedU64V1::Measured(333_334)
    );
    assert_eq!(
        values.session_concentration_ppm,
        ObservedU64V1::Measured(666_667)
    );
    // Minimum-gated, and the floor the runner reconciles for the losing rate.
    assert_eq!(values.win_rate_ppm, ObservedU64V1::Measured(333_333));
    assert_eq!(
        values.losing_trade_rate_ppm,
        ObservedU64V1::Measured(666_666)
    );
    Ok(())
}

#[test]
fn complete_cash_boolean_admission_retains_every_reason_and_missing_oos_cannot_pass()
-> Result<(), String> {
    let fixture = super::super::super::tests::Fixture::new()?;
    let stats = statistics(&fixture)?;
    let count = stats.measurements().candidates.len();
    let result = produce(&fixture.output, stats, &policy(u64::MAX)?, 4 * 1024 * 1024)?;
    result.require_current()?;
    assert_eq!(result.rows().len(), count);
    assert_ne!(result.identity(), result.statistics().identity());
    assert_ne!(result.completion_digest(), [0; 32]);
    let mut zero = 0;
    for (index, row) in result.rows().iter().enumerate() {
        let measured = result
            .statistics()
            .measurements()
            .candidates
            .get(index)
            .ok_or("statistic")?;
        assert_eq!(row.identity(), measured.identity);
        assert_eq!(row.source_index(), index);
        assert!(row.verdict().reconciles());
        assert!(!row.verdict().is_admitted());
        assert!(
            row.verdict()
                .unmeasured()
                .contains(AdmissionReasonV1::DecidedFolds)
        );
        assert_eq!(row.values().decided_folds, ObservedU64V1::Unmeasured);
        assert_eq!(
            row.values().oos_pessimistic_return_paisa,
            ObservedI64V1::Unmeasured
        );
        assert_eq!(
            row.values().trades,
            ObservedU64V1::Measured(measured.trades)
        );
        if measured.trades == 0 {
            zero += 1;
        }
    }
    assert!(
        zero > 0,
        "complete zero-coordinate population must participate"
    );
    assert!(
        result
            .rows()
            .iter()
            .all(|row| row.values().romano_wolf_decision == HypothesisDecisionV1::Unmeasured)
    );
    let bytes = std::fs::read(result.directory.join("body.bin")).map_err(display)?;
    assert_eq!(bytes.len() as u64, HEADER + count as u64 * ROW);
    assert_eq!(bytes.get(..8), Some(b"BTXBAM01".as_slice()));
    Ok(())
}

#[test]
fn admission_policy_identity_resource_refusal_and_corruption_are_explicit() -> Result<(), String> {
    let fixture = super::super::super::tests::Fixture::new()?;
    let stats = statistics(&fixture)?;
    let one = identity(&stats, &policy(1)?, 1_000_000);
    assert_ne!(one, identity(&stats, &policy(2)?, 1_000_000));
    assert_ne!(one, identity(&stats, &policy(1)?, 2_000_000));
    assert!(produce(&fixture.output, stats, &policy(1)?, 1).is_err());
    assert!(!fixture.output.join("boolean-admission-v1").exists());
    let result = produce(
        &fixture.output,
        statistics(&fixture)?,
        &policy(1)?,
        4 * 1024 * 1024,
    )?;
    let path = result.directory.join("body.bin");
    let mut body = std::fs::read(&path).map_err(display)?;
    *body.last_mut().ok_or("last byte")? = 1;
    std::fs::write(path, body).map_err(display)?;
    assert!(result.require_current().is_err());
    Ok(())
}
