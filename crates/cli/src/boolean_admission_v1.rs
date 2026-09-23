//! Every-candidate admission from opaque Boolean statistics and actual rows.
//! New fixed records retain all failed/unmeasured/refused reasons. Training
//! statistics alone never become anchored validation or post-training OOS proof.

use brutex_core::blake3::{Hasher, hash};
use runner::admission::research_projection::{
    ResearchAdmissionProjectionV1, hypothesis_decision, wilson_ppm,
};
use runner::admission::{
    AdmissionEvidenceValuesV1, AdmissionExactProbabilityV2, AdmissionPolicyV1, AdmissionVerdictV1,
    CompletenessV1, HypothesisDecisionV1, ObservedI64V1, ObservedU64V1,
};
use std::path::{Path, PathBuf};

use super::{
    BooleanCoordinateV1, CandidateStatisticsV1, CommittedBooleanStatisticsV1, MeasurementsV1,
    display, persistence,
};

#[path = "boolean_qualification_v1.rs"]
pub(crate) mod qualification;
#[path = "boolean_admission_reader.rs"]
pub(crate) mod reader;
use crate::candidate_universe::population_base_evidence_v2::{
    measured_rate, ratio_observed, return_drawdown,
};
use crate::sweep_evidence::{Completion, Operation};

const HEADER: u64 = 512;
const ROW: u64 = 1024;

pub(crate) struct CandidateAdmissionV1 {
    identity: [u8; 32],
    source_index: usize,
    projection: ResearchAdmissionProjectionV1,
}
impl CandidateAdmissionV1 {
    pub(crate) const fn identity(&self) -> [u8; 32] {
        self.identity
    }
    pub(crate) const fn source_index(&self) -> usize {
        self.source_index
    }
    pub(crate) const fn verdict(&self) -> AdmissionVerdictV1 {
        self.projection.verdict()
    }
    pub(crate) const fn values(&self) -> AdmissionEvidenceValuesV1 {
        self.projection.values()
    }
}

/// Only this source-bound producer can create a durable Boolean admission.
pub(crate) struct CommittedBooleanAdmissionV1 {
    statistics: CommittedBooleanStatisticsV1,
    policy: AdmissionPolicyV1,
    identity: [u8; 32],
    completion: [u8; 32],
    payload: [u8; 32],
    bytes: u64,
    directory: PathBuf,
    rows: Vec<CandidateAdmissionV1>,
}
impl CommittedBooleanAdmissionV1 {
    pub(crate) const fn policy(&self) -> AdmissionPolicyV1 {
        self.policy
    }
    pub(crate) fn require_current(&self) -> Result<(), String> {
        self.statistics.require_current()?;
        persistence::verify(
            &self.directory,
            self.identity,
            self.payload,
            self.bytes,
            self.completion,
        )?;
        self.statistics.require_current()
    }
    pub(crate) const fn identity(&self) -> [u8; 32] {
        self.identity
    }
    pub(crate) const fn completion_digest(&self) -> [u8; 32] {
        self.completion
    }
    pub(crate) const fn statistics(&self) -> &CommittedBooleanStatisticsV1 {
        &self.statistics
    }
    pub(crate) fn rows(&self) -> &[CandidateAdmissionV1] {
        &self.rows
    }
}

pub(crate) fn produce(
    root: &Path,
    statistics: CommittedBooleanStatisticsV1,
    policy: &AdmissionPolicyV1,
    max_bytes: u64,
) -> Result<CommittedBooleanAdmissionV1, String> {
    statistics.require_current()?;
    let count = statistics.measurements().candidates.len();
    let bytes = (count as u64)
        .checked_mul(ROW)
        .and_then(|n| n.checked_add(HEADER))
        .ok_or("Boolean admission record count overflow")?;
    if count == 0 || bytes > max_bytes {
        return Err("Boolean admission complete fixed body exceeds byte admission".to_owned());
    }
    let identity = identity(&statistics, policy, max_bytes);
    let attempt = crate::sweep_evidence::begin(root, identity, Operation::BooleanAdmission)?;
    let prepared = measure(&statistics, policy).and_then(|rows| {
        encode(&statistics, identity, policy, max_bytes, bytes, &rows).map(|body| (rows, body))
    });
    let (rows, body) = match prepared {
        Ok(value) => value,
        Err(why) => return Err(super::super::refuse(attempt, why)),
    };
    let pending =
        match persistence::prepare_in_namespace(root, "boolean-admission-v1", identity, &body) {
            Ok(value) => value,
            Err(why) => return Err(super::super::refuse(attempt, why)),
        };
    let payload = hash(&body);
    let completion = crate::finish_stored_month(
        attempt,
        Completion::Completed,
        || {
            statistics.require_current()?;
            pending.verify_body(payload, bytes)
        },
        || pending.finish(identity, payload, bytes),
    )?;
    let committed = CommittedBooleanAdmissionV1 {
        statistics,
        policy: *policy,
        identity,
        completion,
        payload,
        bytes,
        directory: pending.directory().to_path_buf(),
        rows,
    };
    committed.require_current()?;
    Ok(committed)
}

fn identity(
    statistics: &CommittedBooleanStatisticsV1,
    policy: &AdmissionPolicyV1,
    max_bytes: u64,
) -> [u8; 32] {
    let mut hash = Hasher::new();
    hash.update(b"brutex-boolean-admission-v1\0");
    hash.update(&statistics.identity());
    hash.update(&statistics.completion_digest());
    hash.update(&policy.digest());
    hash.update(&max_bytes.to_le_bytes());
    hash.finalize()
}

fn measure(
    statistics: &CommittedBooleanStatisticsV1,
    policy: &AdmissionPolicyV1,
) -> Result<Vec<CandidateAdmissionV1>, String> {
    let measured = statistics.measurements();
    let mut result = Vec::new();
    result
        .try_reserve_exact(measured.candidates.len())
        .map_err(display)?;
    for (index, candidate) in measured.candidates.iter().enumerate() {
        let source = statistics
            .sources()
            .get(candidate.family)
            .and_then(|family| family.rows().get(candidate.coordinate))
            .ok_or("Boolean admission source coordinate missing")?;
        if source.identity() != candidate.identity {
            return Err("Boolean admission statistics belong to another coordinate".to_owned());
        }
        let mut values = base_values(source)?;
        apply_statistics(&mut values, candidate, measured, index)?;
        let projection = policy
            .evaluate_research_projection(values)
            .map_err(|why| format!("Boolean admission comparison refused: {why:?}"))?;
        result.push(CandidateAdmissionV1 {
            identity: candidate.identity,
            source_index: index,
            projection,
        });
    }
    Ok(result)
}

pub(super) fn base_values(row: &BooleanCoordinateV1) -> Result<AdmissionEvidenceValuesV1, String> {
    let cell = row.cell();
    let details = crate::institutional_evidence::reconcile_trade_rows(cell, row.trades())?;
    let rate = |part, total, name| measured_rate(part, total, name).map_err(display);
    let ratio = |part, total, name| ratio_observed(part, total, name).map_err(display);
    let measured = |value| {
        if cell.trades > 0 {
            ObservedU64V1::Measured(value)
        } else {
            ObservedU64V1::Unmeasured
        }
    };
    let positive = |value: i64| u64::try_from(value).map_err(display);
    let losses = cell
        .trades
        .checked_sub(cell.wins)
        .ok_or("Boolean admission wins exceed trades")?;
    let worst_loss = cell.worst_trade.unsigned_abs();
    let gross_loss = cell.gross_loss.unsigned_abs();
    let gross_win = positive(cell.gross_win)?;
    Ok(AdmissionEvidenceValuesV1 {
        support_hits: ObservedU64V1::Measured(row.summary().hits),
        independent_sessions: ObservedU64V1::Measured(row.support_sessions()),
        trades: ObservedU64V1::Measured(cell.trades),
        max_mae_paisa: details.max_mae_paisa,
        worst_reward_risk_ppm: ratio(positive(cell.min_win)?, worst_loss, "worst reward/risk")?,
        win_rate_ppm: rate(cell.wins, cell.trades, "win rate")?,
        wilson_win_rate_ppm: ObservedU64V1::Unmeasured,
        return_drawdown_ppm: if cell.trades > 0 {
            return_drawdown(cell.pessimistic, cell.max_drawdown).map_err(display)?
        } else {
            ObservedU64V1::Unmeasured
        },
        weakest_period_return_paisa: details.weakest_period_return_paisa,
        pbo_ppm: ObservedU64V1::Unmeasured,
        fwer_p_value_ppm: ObservedU64V1::Unmeasured,
        spa_p_value_ppm: ObservedU64V1::Unmeasured,
        decided_folds: ObservedU64V1::Unmeasured,
        ambiguous_fill_rate_ppm: rate(cell.ambiguous_bars, cell.trades, "ambiguous rate")?,
        gap_affected_rate_ppm: rate(cell.gapped, cell.trades, "gap rate")?,
        session_concentration_ppm: details.session_concentration_ppm,
        largest_trade_profit_share_ppm: details.largest_trade_profit_share_ppm,
        execution_complete: if row.execution_refusal_bits().bits() == 0 {
            CompletenessV1::Complete
        } else {
            CompletenessV1::Refused
        },
        data_complete: CompletenessV1::Complete,
        calendar_complete: CompletenessV1::Complete,
        population_complete: CompletenessV1::Complete,
        drawdown_paisa: ObservedU64V1::Measured(positive(cell.max_drawdown)?),
        worst_trade_loss_paisa: ObservedU64V1::Measured(worst_loss),
        losing_trade_rate_ppm: rate(losses, cell.trades, "losing rate")?,
        losing_trades: ObservedU64V1::Measured(losses),
        pessimistic_profit_paisa: ObservedI64V1::Measured(cell.pessimistic),
        winning_trades: ObservedU64V1::Measured(cell.wins),
        average_win_paisa: gross_win
            .checked_div(cell.wins)
            .map_or(ObservedU64V1::Unmeasured, ObservedU64V1::Measured),
        average_loss_paisa: gross_loss
            .checked_div(losses)
            .map_or(ObservedU64V1::Unmeasured, ObservedU64V1::Measured),
        profit_factor_ppm: ratio(gross_win, gross_loss, "profit factor")?,
        consecutive_losing_streak: measured(u64::from(cell.max_losing_streak)),
        consecutive_winning_streak: measured(u64::from(cell.max_winning_streak)),
        bootstrap_draws: ObservedU64V1::Unmeasured,
        bootstrap_strategies: ObservedU64V1::Unmeasured,
        bootstrap_periods: ObservedU64V1::Unmeasured,
        pbo_contributing_folds: ObservedU64V1::Unmeasured,
        pbo_unrankable_folds: ObservedU64V1::Unmeasured,
        profitable_oos_folds: ObservedU64V1::Unmeasured,
        oos_pessimistic_return_paisa: ObservedI64V1::Unmeasured,
        white_reality_p_value_ppm: ObservedU64V1::Unmeasured,
        romano_wolf_p_value_ppm: ObservedU64V1::Unmeasured,
        white_reality_decision: HypothesisDecisionV1::Unmeasured,
        romano_wolf_decision: HypothesisDecisionV1::Unmeasured,
        full_precision_statistics_complete: CompletenessV1::Unmeasured,
    })
}

fn apply_statistics(
    values: &mut AdmissionEvidenceValuesV1,
    candidate: &CandidateStatisticsV1,
    measured: &MeasurementsV1,
    index: usize,
) -> Result<(), String> {
    if values.trades != ObservedU64V1::Measured(candidate.trades)
        || values.winning_trades != ObservedU64V1::Measured(candidate.wins)
    {
        return Err("Boolean admission statistics trade counts disagree".to_owned());
    }
    values.wilson_win_rate_ppm = ObservedU64V1::Measured(
        wilson_ppm(
            candidate.wins,
            candidate.trades,
            candidate.wilson_lower_bits,
        )
        .ok_or("Boolean admission Wilson source disagrees")?,
    );
    let white = measured.white.exact_p_value();
    let white = probability(white.numerator() as u64, white.denominator() as u64)?;
    let spa = measured.spa.exact_p_value();
    values.white_reality_p_value_ppm = ObservedU64V1::Measured(white.ppm());
    values.white_reality_decision = hypothesis_decision(white);
    values.spa_p_value_ppm = ObservedU64V1::Measured(
        probability(spa.numerator() as u64, spa.denominator() as u64)?.ppm(),
    );
    values.bootstrap_draws = ObservedU64V1::Measured(measured.white.draws() as u64);
    values.bootstrap_strategies = ObservedU64V1::Measured(measured.white.strategies() as u64);
    values.bootstrap_periods = ObservedU64V1::Measured(measured.white.periods() as u64);
    values.pbo_contributing_folds = ObservedU64V1::Measured(measured.contributing_splits);
    values.pbo_unrankable_folds = ObservedU64V1::Measured(
        (measured.splits.len() as u64)
            .checked_sub(measured.contributing_splits)
            .ok_or("Boolean PBO contributing count exceeds splits")?,
    );
    if measured.contributing_splits > 0 {
        values.pbo_ppm = ObservedU64V1::Measured(
            probability(measured.bottom_half_splits, measured.contributing_splits)?.ppm(),
        );
    }
    if let Some(romano) = &measured.romano {
        let family = romano
            .familywise_p_value()
            .ok_or("Boolean RW family missing")?;
        let adjusted = romano
            .candidate(index)
            .ok_or("Boolean RW candidate missing")?
            .adjusted_p_value();
        let adjusted = probability(adjusted.numerator() as u64, adjusted.denominator() as u64)?;
        values.fwer_p_value_ppm = ObservedU64V1::Measured(
            probability(family.numerator() as u64, family.denominator() as u64)?.ppm(),
        );
        values.romano_wolf_p_value_ppm = ObservedU64V1::Measured(adjusted.ppm());
        values.romano_wolf_decision = hypothesis_decision(adjusted);
        values.full_precision_statistics_complete = CompletenessV1::Complete;
    }
    Ok(())
}

fn probability(numerator: u64, denominator: u64) -> Result<AdmissionExactProbabilityV2, String> {
    AdmissionExactProbabilityV2::new(numerator, denominator)
        .map_err(|why| format!("Boolean exact probability refused: {why:?}"))
}

fn encode(
    statistics: &CommittedBooleanStatisticsV1,
    identity: [u8; 32],
    policy: &AdmissionPolicyV1,
    max_bytes: u64,
    bytes: u64,
    rows: &[CandidateAdmissionV1],
) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    body.try_reserve_exact(usize::try_from(bytes).map_err(display)?)
        .map_err(display)?;
    body.extend_from_slice(b"BTXBAM01");
    body.extend_from_slice(&identity);
    body.extend_from_slice(&statistics.identity());
    body.extend_from_slice(&statistics.completion_digest());
    body.extend_from_slice(&policy.canonical_bytes());
    body.extend_from_slice(&max_bytes.to_le_bytes());
    body.extend_from_slice(&(rows.len() as u64).to_le_bytes());
    if body.len() as u64 > HEADER {
        return Err("Boolean admission manifest exceeds fixed header".to_owned());
    }
    body.resize(usize::try_from(HEADER).map_err(display)?, 0);
    for row in rows {
        let end = body
            .len()
            .checked_add(usize::try_from(ROW).map_err(display)?)
            .ok_or("Boolean admission row offset overflow")?;
        body.extend_from_slice(&row.identity);
        body.extend_from_slice(&(row.source_index as u64).to_le_bytes());
        body.extend_from_slice(&row.projection.canonical_bytes());
        if body.len() > end {
            return Err("Boolean admission projection exceeds fixed row".to_owned());
        }
        body.resize(end, 0);
    }
    if body.len() as u64 != bytes {
        return Err("Boolean admission fixed body cardinality differs".to_owned());
    }
    Ok(body)
}

#[cfg(test)]
#[path = "boolean_admission_tests.rs"]
pub(super) mod tests;
