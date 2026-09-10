//! Private exact projection from the opaque Execution V4 handoff.
use super::{Block, Encoder, MAGIC, SEAL_AT, SELECTION_V6_BLOCK_BYTES};
use crate::execution_v4::{
    CommittedStoredExecutionV4, ExecutionV4AdmissionStatus, ExecutionV4Direction,
    ExecutionV4Family, ExecutionV4FamilyTerminal, ExecutionV4SelectionV6RowSourceV1,
    ExecutionV4SelectionV6SourceV1, ExecutionV4Terminal,
};
use crate::population::TradeDirectionV1;
use crate::population_admission_v4::{
    AdmissionV4DecisionStatus, AdmissionV4Family, AdmissionV4FamilyTerminal,
};
use costs::fill::Direction;
use runner::admission::{AdmissionStatusV1, ObservedI64V1, ObservedU64V1};
use runner::portfolio::StrategyDigest;
use runner::topn::{
    Candidate, Metrics, PopulationPass, PopulationProof, RankedCandidate, RankingPolicyV1,
    Selection,
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Winner {
    pub(super) family: u64,
    pub(super) ranked: RankedCandidate,
    pub(super) disposition_id: [u8; 32],
    pub(super) selected_exit_digest: [u8; 32],
    global_sequence: u64,
    family_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Prepared {
    block: Block,
    pub(super) winners: Vec<Winner>,
}

impl Prepared {
    pub(super) fn from_execution(
        source: &mut CommittedStoredExecutionV4,
        policy: RankingPolicyV1,
    ) -> Result<Self, String> {
        Self::from_source(&source.selection_v6_source()?, policy)
    }

    fn from_source(
        source: &ExecutionV4SelectionV6SourceV1,
        policy: RankingPolicyV1,
    ) -> Result<Self, String> {
        validate_envelope(source)?;
        let (candidates, by_strategy) = project_rows(source)?;
        let (selection, proof) =
            rank_candidates(&candidates, source.execution().row_count(), policy)?;
        let mut winners = Vec::new();
        winners
            .try_reserve_exact(selection.rows.len())
            .map_err(|why| why.to_string())?;
        for ranked in selection.rows {
            let row = by_strategy
                .get(&ranked.candidate.strategy_digest.bytes())
                .ok_or("Selection V6 winner absent from exact source")?;
            if project(row)? != ranked.candidate || !ranked.candidate.admitted {
                return Err("Selection V6 winner differs from eligible source".to_owned());
            }
            let disposition = row.disposition();
            winners.push(Winner {
                family: match disposition.family() {
                    ExecutionV4Family::Nifty => 1,
                    ExecutionV4Family::BankNifty => 2,
                },
                ranked,
                disposition_id: disposition.disposition_id(),
                selected_exit_digest: disposition
                    .selected_exit_digest()
                    .ok_or("Selection V6 authorized winner has no selected exit")?,
                global_sequence: disposition.global_sequence(),
                family_sequence: disposition.family_sequence(),
            });
        }
        let block = encode_block(source, policy, proof, &winners)?;
        Ok(Self { block, winners })
    }
    pub(super) fn block(&self) -> Result<Block, String> {
        super::verify_block(&self.block)?;
        Ok(self.block)
    }
    pub(super) fn identity(&self) -> Result<[u8; 32], String> {
        super::block_identity(&self.block)
    }
}

type SourceRows<'a> = HashMap<[u8; 32], &'a ExecutionV4SelectionV6RowSourceV1>;

fn project_rows(
    source: &ExecutionV4SelectionV6SourceV1,
) -> Result<(Vec<Candidate>, SourceRows<'_>), String> {
    let mut by_strategy = HashMap::new();
    by_strategy
        .try_reserve(source.rows().len())
        .map_err(|why| why.to_string())?;
    let mut candidates = Vec::new();
    candidates
        .try_reserve_exact(source.rows().len())
        .map_err(|why| why.to_string())?;
    let mut dispositions = HashSet::new();
    dispositions
        .try_reserve(source.rows().len())
        .map_err(|why| why.to_string())?;
    let mut matrix = [0_u64; 16];
    for (index, row) in source.rows().iter().enumerate() {
        let candidate = project(row)?;
        if row.disposition().global_sequence()
            != u64::try_from(index).map_err(|why| why.to_string())?
            || !dispositions.insert(row.disposition().disposition_id())
            || by_strategy
                .insert(candidate.strategy_digest.bytes(), row)
                .is_some()
        {
            return Err(
                "Selection V6 source is reordered or repeats a strategy/disposition".to_owned(),
            );
        }
        let family = match row.disposition().family() {
            ExecutionV4Family::Nifty => 0,
            ExecutionV4Family::BankNifty => 1,
        };
        let status = match row.disposition().admission_status() {
            ExecutionV4AdmissionStatus::Admitted => 0,
            ExecutionV4AdmissionStatus::Rejected => 1,
            ExecutionV4AdmissionStatus::Unmeasured => 2,
            ExecutionV4AdmissionStatus::Refused => 3,
        };
        let terminal =
            usize::from(row.disposition().terminal() == ExecutionV4Terminal::PolicyRefused);
        let cell = matrix
            .get_mut(family * 8 + status * 2 + terminal)
            .ok_or("Selection V6 matrix bounds")?;
        *cell = cell.checked_add(1).ok_or("Selection V6 matrix overflow")?;
        candidates.push(candidate);
    }
    if matrix != source.execution().terminal_matrix() {
        return Err("Selection V6 terminal matrix disagrees with durable execution".to_owned());
    }
    Ok((candidates, by_strategy))
}

fn rank_candidates(
    candidates: &[Candidate],
    expected: u64,
    policy: RankingPolicyV1,
) -> Result<(Selection, PopulationProof), String> {
    let selection = runner::topn::select(candidates, policy, 25)
        .map_err(|why| format!("Selection V6 ranking refused: {why:?}"))?;
    let mut pass = PopulationPass::new();
    for candidate in candidates {
        pass.observe(candidate);
    }
    let proof = pass.finish();
    if proof.considered != expected
        || proof.admitted.checked_add(proof.refused) != Some(proof.considered)
        || selection.considered != proof.considered
        || selection.admitted != proof.admitted
        || selection.refused != proof.refused
        || selection.unmeasured != proof.unmeasured
        || u64::try_from(selection.rows.len()).map_err(|why| why.to_string())?
            != proof.admitted.min(25)
    {
        return Err("Selection V6 complete ranking proof does not reconcile".to_owned());
    }
    Ok((selection, proof))
}

fn encode_block(
    source: &ExecutionV4SelectionV6SourceV1,
    policy: RankingPolicyV1,
    proof: PopulationProof,
    winners: &[Winner],
) -> Result<Block, String> {
    let mut block = [0; SELECTION_V6_BLOCK_BYTES];
    let mut encoder = Encoder {
        bytes: block
            .get_mut(..SEAL_AT)
            .ok_or("Selection V6 fixed payload")?,
        at: 0,
    };
    encoder.bytes(MAGIC)?;
    encoder.bytes(&6_u32.to_le_bytes())?;
    encoder.bytes(&[0; 36])?;
    encode_source(&mut encoder, source, policy)?;
    for count in [
        proof.considered,
        proof.admitted,
        proof.refused,
        proof.unmeasured,
        u64::try_from(winners.len()).map_err(|why| why.to_string())?,
        u64::try_from(winners.len().min(10)).map_err(|why| why.to_string())?,
    ] {
        encoder.number(count)?;
    }
    encoder.bytes(&proof.ordered_digest)?;
    for winner in winners {
        encode_winner(&mut encoder, winner)?;
    }
    let identity = super::block_identity(&block)?;
    block
        .get_mut(24..56)
        .ok_or("Selection V6 identity slot")?
        .copy_from_slice(&identity);
    let seal = super::seal(&block)?;
    block
        .get_mut(SEAL_AT..)
        .ok_or("Selection V6 seal slot")?
        .copy_from_slice(&seal);
    super::verify_block(&block)?;
    Ok(block)
}

fn validate_envelope(source: &ExecutionV4SelectionV6SourceV1) -> Result<(), String> {
    let execution = source.execution();
    let population = source.population();
    if ![60, 120, 180, 300, 600, 900, 1800, 3600].contains(&execution.rung())
        || execution.horizon_bars() == 0
        || execution.population_id() != population.population_id()
        || execution.population_completion_id() != population.completion_id()
        || execution.population_ordered_digest() != population.ordered_candidate_digest()
        || execution.row_count() != population.candidate_count()
        || execution.disposition_count() != execution.row_count()
        || u64::try_from(source.rows().len()).map_err(|why| why.to_string())?
            != execution.row_count()
    {
        return Err("Selection V6 source receipts/rung/cardinality disagree".to_owned());
    }
    let expected = [
        (
            AdmissionV4Family::Nifty,
            execution.nifty_count(),
            execution.nifty_terminal(),
        ),
        (
            AdmissionV4Family::BankNifty,
            execution.banknifty_count(),
            execution.banknifty_terminal(),
        ),
    ];
    let mut active = 0;
    for (family, (expected_family, count, terminal)) in source.families().iter().zip(expected) {
        if family.family() != expected_family
            || family.candidate_count() != count
            || family.evaluated_count() != count
            || family.decision_count() != count
        {
            return Err("Selection V6 family order/counts disagree".to_owned());
        }
        active += u64::from(active_family(family.terminal(), terminal, count)?);
    }
    validate_parameter_shape(
        [execution.nifty_count(), execution.banknifty_count()],
        execution.row_count(),
        active,
        execution.parameter_count(),
    )
}

fn validate_parameter_shape(
    counts: [u64; 2],
    total: u64,
    active: u64,
    parameters: u64,
) -> Result<(), String> {
    let [nifty, banknifty] = counts;
    if active > 2 || parameters != active * 2 || nifty.checked_add(banknifty) != Some(total) {
        return Err(
            "Selection V6 requires exactly 0/2/4 parameters for its active families".to_owned(),
        );
    }
    Ok(())
}

fn active_family(
    population: AdmissionV4FamilyTerminal,
    execution: ExecutionV4FamilyTerminal,
    count: u64,
) -> Result<bool, String> {
    match (population, execution, count) {
        (AdmissionV4FamilyTerminal::Evaluated, ExecutionV4FamilyTerminal::Evaluated, 2..) => {
            Ok(true)
        }
        (
            AdmissionV4FamilyTerminal::NaturallyExtinct,
            ExecutionV4FamilyTerminal::NaturallyExtinct,
            0,
        ) => Ok(false),
        _ => Err("Selection V6 family terminal is unreachable or contradictory".to_owned()),
    }
}

fn project(source: &ExecutionV4SelectionV6RowSourceV1) -> Result<Candidate, String> {
    let population = source.population();
    let row = population.candidate().row();
    let disposition = source.disposition();
    let status = match population.status() {
        AdmissionV4DecisionStatus::Admitted => AdmissionStatusV1::Admitted,
        AdmissionV4DecisionStatus::Rejected => AdmissionStatusV1::Rejected,
        AdmissionV4DecisionStatus::Unmeasured => AdmissionStatusV1::Unmeasured,
        AdmissionV4DecisionStatus::Refused => AdmissionStatusV1::Refused,
    };
    let expected_status = match status {
        AdmissionStatusV1::Admitted => ExecutionV4AdmissionStatus::Admitted,
        AdmissionStatusV1::Rejected => ExecutionV4AdmissionStatus::Rejected,
        AdmissionStatusV1::Unmeasured => ExecutionV4AdmissionStatus::Unmeasured,
        AdmissionStatusV1::Refused => ExecutionV4AdmissionStatus::Refused,
    };
    let (direction, expected_direction) = match row.direction() {
        TradeDirectionV1::Long => (Direction::Long, ExecutionV4Direction::Long),
        TradeDirectionV1::Short => (Direction::Short, ExecutionV4Direction::Short),
    };
    if disposition.admission_status() != expected_status
        || disposition.direction() != expected_direction
        || disposition.candidate_semantic_id() != population.candidate_semantic_id()
        || row.candidate_semantic_digest() != population.candidate_semantic_id()
        || disposition.global_sequence() != population.global_sequence()
        || disposition.family_sequence() != population.family_sequence()
    {
        return Err("Selection V6 durable row and candidate/status disagree".to_owned());
    }
    let cell = row
        .facts()
        .to_cell(row.exit())
        .map_err(|why| format!("Selection V6 candidate metrics: {why}"))?;
    let assurance = if cell.trades == 0 {
        0
    } else {
        crate::institutional_evidence::wilson_lower_ppm(
            cell.wins,
            cell.trades,
            cell.assurance_bp(),
        )?
    };
    let metric = crate::population_admission_writer::metrics_from_cell(&cell, assurance)?;
    let comparison =
        runner::admission::AdmissionV3ArithmeticProjection::verify_decision_record_detached(
            population.runner_decision(),
        )
        .map_err(|why| format!("Selection V6 admission arithmetic: {why:?}"))?;
    if comparison.status() != status {
        return Err("Selection V6 authenticated admission status changed".to_owned());
    }
    compare_metrics(
        status,
        row.support_hits(),
        metric,
        &comparison.comparison_values(),
    )?;
    let metrics = Metrics {
        drawdown: metric.drawdown,
        worst_loss: metric.worst_loss,
        losing_rate_ppm: metric.losing_rate_ppm,
        losing_trades: metric.losing_trades,
        loss_ratio_ppm: metric.loss_ratio_ppm,
        pessimistic_profit: metric.pessimistic_profit,
        winning_trades: metric.winning_trades,
        win_rate_ppm: metric.win_rate_ppm,
        reward_to_risk_ppm: metric.reward_to_risk_ppm,
        average_win: metric.average_win,
        average_loss: metric.average_loss,
        assurance_ppm: metric.assurance_ppm,
    };
    Ok(Candidate {
        strategy_digest: StrategyDigest::new(row.candidate_semantic_digest()),
        mask_words: row.mask_words(),
        direction,
        admitted: status == AdmissionStatusV1::Admitted
            && disposition.terminal() == ExecutionV4Terminal::Authorized,
        metrics,
    })
}

fn compare_metrics(
    status: AdmissionStatusV1,
    support: u64,
    metrics: crate::population::TopMetricsV1,
    values: &runner::admission::AdmissionEvidenceValuesV1,
) -> Result<(), String> {
    let trades = metrics
        .winning_trades
        .checked_add(metrics.losing_trades)
        .ok_or("Selection V6 trade count overflow")?;
    for (observed, exact) in [
        (values.support_hits, support),
        (values.trades, trades),
        (values.drawdown_paisa, metrics.drawdown),
        (values.worst_trade_loss_paisa, metrics.worst_loss),
        (values.losing_trade_rate_ppm, metrics.losing_rate_ppm),
        (values.losing_trades, metrics.losing_trades),
        (values.winning_trades, metrics.winning_trades),
        (values.win_rate_ppm, metrics.win_rate_ppm),
        (values.wilson_win_rate_ppm, metrics.assurance_ppm),
        (values.average_win_paisa, metrics.average_win),
        (values.average_loss_paisa, metrics.average_loss),
    ] {
        compare_unsigned(status, observed, exact)?;
    }
    match values.pessimistic_profit_paisa {
        ObservedI64V1::Measured(value) if value == metrics.pessimistic_profit => {}
        ObservedI64V1::Measured(_) => {
            return Err("Selection V6 admission profit differs from candidate".to_owned());
        }
        _ if status == AdmissionStatusV1::Admitted => {
            return Err("Selection V6 admitted profit is not measured".to_owned());
        }
        _ => {}
    }
    match metrics.reward_to_risk_ppm {
        Some(exact) => compare_unsigned(status, values.worst_reward_risk_ppm, exact)?,
        None if matches!(values.worst_reward_risk_ppm, ObservedU64V1::Measured(_)) => {
            return Err("Selection V6 admission ratio has no direct denominator".to_owned());
        }
        None => {}
    }
    Ok(())
}

fn compare_unsigned(
    status: AdmissionStatusV1,
    observed: ObservedU64V1,
    exact: u64,
) -> Result<(), String> {
    match observed {
        ObservedU64V1::Measured(value) if value == exact => Ok(()),
        ObservedU64V1::Measured(_) => {
            Err("Selection V6 admission metric differs from direct candidate".to_owned())
        }
        _ if status == AdmissionStatusV1::Admitted => {
            Err("Selection V6 admitted metric is not measured".to_owned())
        }
        _ => Ok(()),
    }
}

fn encode_source(
    out: &mut Encoder<'_>,
    source: &ExecutionV4SelectionV6SourceV1,
    policy: RankingPolicyV1,
) -> Result<(), String> {
    let e = source.execution();
    for id in [
        e.population_id(),
        e.population_completion_id(),
        e.population_ordered_digest(),
        e.completion_id(),
        e.ordered_disposition_digest(),
        e.nifty_authority_id(),
        e.banknifty_authority_id(),
        policy.digest(),
    ] {
        if id == [0; 32] {
            return Err("Selection V6 source identity is zero".to_owned());
        }
        out.bytes(&id)?;
    }
    for value in [
        u64::from(e.rung()),
        u64::from(e.horizon_bars()),
        e.parameter_count(),
        e.percentile_count(),
        e.row_count(),
        e.nifty_count(),
        e.banknifty_count(),
        e.authorized_count(),
        e.policy_refused_count(),
    ] {
        out.number(value)?;
    }
    for count in e.admission_counts().into_iter().chain(e.terminal_matrix()) {
        out.number(count)?;
    }
    for family in source.families() {
        for id in [
            family.row_id(),
            family.candidate_universe_id(),
            family.candidate_completion_digest(),
            family.finalization_family_row_id(),
        ] {
            out.bytes(&id)?;
        }
        for count in [
            family.family() as u64,
            family.terminal() as u64,
            family.candidate_count(),
            family.evaluated_count(),
            family.decision_count(),
        ] {
            out.number(count)?;
        }
    }
    Ok(())
}

fn encode_winner(out: &mut Encoder<'_>, winner: &Winner) -> Result<(), String> {
    for id in [
        winner.ranked.candidate.strategy_digest.bytes(),
        winner.disposition_id,
        winner.selected_exit_digest,
    ] {
        out.bytes(&id)?;
    }
    for value in [
        winner.family,
        winner.global_sequence,
        winner.family_sequence,
        winner.ranked.score,
        match winner.ranked.candidate.direction {
            Direction::Long => 1,
            Direction::Short => 2,
        },
    ] {
        out.number(value)?;
    }
    for word in winner.ranked.candidate.mask_words {
        out.number(word)?;
    }
    let m = winner.ranked.candidate.metrics;
    for value in [
        m.drawdown,
        m.worst_loss,
        m.losing_rate_ppm,
        m.losing_trades,
        m.winning_trades,
        m.win_rate_ppm,
        m.average_win,
        m.average_loss,
        m.assurance_ppm,
    ] {
        out.number(value)?;
    }
    out.bytes(&m.pessimistic_profit.to_le_bytes())?;
    for ratio in [m.loss_ratio_ppm, m.reward_to_risk_ppm] {
        out.number(u64::from(ratio.is_some()))?;
        out.number(ratio.unwrap_or(0))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::indexing_slicing)]
    use super::*;

    #[test]
    fn exact_four_family_shapes_retain_zero_two_or_four_parameters() {
        for nifty in [false, true] {
            for banknifty in [false, true] {
                let mut active = 0;
                let counts = [if nifty { 2 } else { 0 }, if banknifty { 5 } else { 0 }];
                for (evaluated, count) in [nifty, banknifty].into_iter().zip(counts) {
                    let population = if evaluated {
                        AdmissionV4FamilyTerminal::Evaluated
                    } else {
                        AdmissionV4FamilyTerminal::NaturallyExtinct
                    };
                    let execution = if evaluated {
                        ExecutionV4FamilyTerminal::Evaluated
                    } else {
                        ExecutionV4FamilyTerminal::NaturallyExtinct
                    };
                    let actual =
                        active_family(population, execution, count).expect("reachable family");
                    assert_eq!(actual, evaluated);
                    active += u64::from(actual);
                }
                let total = counts.into_iter().sum();
                for parameters in 0..=5 {
                    assert_eq!(
                        validate_parameter_shape(counts, total, active, parameters).is_ok(),
                        parameters == active * 2
                    );
                }
                assert!(validate_parameter_shape(counts, total + 1, active, active * 2).is_err());
            }
        }
        assert!(validate_parameter_shape([u64::MAX, 1], 0, 2, 4).is_err());
        assert!(validate_parameter_shape([0, 0], 0, 3, 6).is_err());
    }

    #[test]
    fn family_terminal_cross_product_refuses_singleton_and_false_extinction() {
        for population in [
            AdmissionV4FamilyTerminal::Evaluated,
            AdmissionV4FamilyTerminal::NaturallyExtinct,
            AdmissionV4FamilyTerminal::InsufficientForCscv,
        ] {
            for execution in [
                ExecutionV4FamilyTerminal::Evaluated,
                ExecutionV4FamilyTerminal::NaturallyExtinct,
            ] {
                for count in [0, 1, 2, u64::MAX] {
                    let expected = (population == AdmissionV4FamilyTerminal::Evaluated
                        && execution == ExecutionV4FamilyTerminal::Evaluated
                        && count >= 2)
                        || (population == AdmissionV4FamilyTerminal::NaturallyExtinct
                            && execution == ExecutionV4FamilyTerminal::NaturallyExtinct
                            && count == 0);
                    assert_eq!(
                        active_family(population, execution, count).is_ok(),
                        expected
                    );
                }
            }
        }
    }

    fn candidate(ordinal: u8, admitted: bool) -> Candidate {
        Candidate {
            strategy_digest: StrategyDigest::new([ordinal; 32]),
            mask_words: [u64::from(ordinal), 0, 0, 0, 0, 0],
            direction: Direction::Long,
            admitted,
            metrics: Metrics {
                drawdown: 10,
                worst_loss: 10,
                losing_rate_ppm: 100_000,
                losing_trades: 1,
                loss_ratio_ppm: Some(0),
                pessimistic_profit: 100,
                winning_trades: 9,
                win_rate_ppm: 900_000,
                reward_to_risk_ppm: Some(2_000_000),
                average_win: 20,
                average_loss: 10,
                assurance_ppm: 600_000,
            },
        }
    }

    #[test]
    fn actual_top_counts_and_tie_order_are_reconciled_without_padding() {
        let policy = RankingPolicyV1::new(runner::topn::Weights::equal()).expect("ranking policy");
        for count in [0_u8, 1, 9, 10, 11, 24, 25, 26, 40] {
            let candidates: Vec<_> = (1..=count).rev().map(|id| candidate(id, true)).collect();
            let (selected, proof) =
                rank_candidates(&candidates, u64::from(count), policy).expect("actual prefix");
            assert_eq!(proof.considered, u64::from(count));
            assert_eq!(proof.admitted, u64::from(count));
            assert_eq!(proof.refused, 0);
            assert_eq!(selected.rows.len(), usize::from(count.min(25)));
            let expected: Vec<_> = (1..=count.min(25)).map(|id| candidate(id, true)).collect();
            assert_eq!(
                selected
                    .rows
                    .iter()
                    .map(|row| row.candidate)
                    .collect::<Vec<_>>(),
                expected
            );
            assert!(rank_candidates(&candidates, u64::from(count) + 1, policy).is_err());
        }
        let mut refused = candidate(1, false);
        refused.metrics.pessimistic_profit = i64::MAX;
        let admitted = candidate(2, true);
        let (selected, proof) =
            rank_candidates(&[refused, admitted], 2, policy).expect("mixed outcome");
        assert_eq!(proof.admitted, 1);
        assert_eq!(proof.refused, 1);
        assert_eq!(
            selected.rows.first().expect("actual winner").candidate,
            admitted
        );
        let (empty, proof) = rank_candidates(&[refused], 1, policy).expect("valid zero selected");
        assert!(empty.rows.is_empty());
        assert_eq!(proof.refused, 1);
    }

    #[test]
    fn undefined_ratios_and_measured_zero_have_distinct_stored_bytes() {
        let mut winner = Winner {
            family: 1,
            ranked: RankedCandidate {
                candidate: candidate(1, true),
                score: 1,
            },
            disposition_id: [2; 32],
            selected_exit_digest: [3; 32],
            global_sequence: 0,
            family_sequence: 0,
        };
        let mut measured = [0; 512];
        let mut unknown = [0; 512];
        encode_winner(
            &mut Encoder {
                bytes: &mut measured,
                at: 0,
            },
            &winner,
        )
        .expect("measured zero");
        winner.ranked.candidate.metrics.loss_ratio_ppm = None;
        encode_winner(
            &mut Encoder {
                bytes: &mut unknown,
                at: 0,
            },
            &winner,
        )
        .expect("undefined");
        assert_ne!(measured, unknown);
        assert!(
            compare_unsigned(AdmissionStatusV1::Admitted, ObservedU64V1::Unmeasured, 0).is_err()
        );
        assert!(
            compare_unsigned(AdmissionStatusV1::Rejected, ObservedU64V1::Unmeasured, 0).is_ok()
        );
        assert!(
            compare_unsigned(AdmissionStatusV1::Rejected, ObservedU64V1::Measured(1), 0).is_err()
        );
        assert!(
            compare_unsigned(AdmissionStatusV1::Admitted, ObservedU64V1::Measured(0), 0).is_ok()
        );
    }
}
