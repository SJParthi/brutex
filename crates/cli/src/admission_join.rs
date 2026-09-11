//! Atomic read join between V4 populations and sealed admission sidecars.
//!
//! Neither input is authoritative by itself.  Population V4 proves complete
//! rows and exact signal/execution calendar coverage, but its admission fields
//! are historical caller-authored summaries.  The admission sidecar proves a
//! canonical policy/evidence/verdict decision, but does not contain the
//! population row it names.  This module joins both under the one shared
//! `population-write.lock`, so no writer can change either half between their
//! generation checks and row-by-row comparison.
//!
//! # Cost
//!
//! Open has the sum of both underlying ledgers' linear scan/reconciliation
//! costs.  One page is O(requested rows), bounded by both ledgers' fixed
//! 256-row ceiling.  Receipt probes are average O(1).  The lock transaction,
//! hashing, page join, open and persistence are not claimed O(1).
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

use std::fs::File;
use std::path::{Path, PathBuf};

use runner::admission::AdmissionDecisionSealV1;

use crate::admission_store::{
    AdmissionAuthorityLedger, AdmissionCompletionReceiptV1, AdmissionDecisionRecordV1,
};
use crate::population::{CompletionReceiptV4, PopulationLedger, PopulationRowV1};

/// Operator-facing refusal from the admission-authoritative join.
pub type AdmissionJoinRefusal = String;

/// Digests proving which two receipt-last authorities were joined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionAuthorityIdentityV1 {
    /// Domain-separated exact V4 population completion digest.
    pub population_v4_completion_digest: [u8; 32],
    /// Domain-separated exact admission completion digest.
    pub admission_completion_digest: [u8; 32],
}

/// Immutable completion facts proved together when the joined ledger opened.
///
/// Selection keeps this snapshot beside any derived receipt so its authority
/// can name the complete V4 population and the exact admission policy/counts,
/// not only their digests.  Every page compares freshly read receipts with
/// this snapshot before returning rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmissionAuthoritySnapshotV1 {
    population_v4: CompletionReceiptV4,
    admission: AdmissionCompletionReceiptV1,
    identity: AdmissionAuthorityIdentityV1,
}

impl AdmissionAuthoritySnapshotV1 {
    /// Complete V4 population receipt, returned by value because it is fixed
    /// size and copy-safe.
    #[must_use]
    pub const fn population_v4(&self) -> CompletionReceiptV4 {
        self.population_v4
    }

    /// Complete admission receipt, including canonical policy, status counts
    /// and the ordered-decision digest.
    #[must_use]
    pub const fn admission(&self) -> &AdmissionCompletionReceiptV1 {
        &self.admission
    }

    /// Digests of the exact two completion receipts in this snapshot.
    #[must_use]
    pub const fn identity(&self) -> AdmissionAuthorityIdentityV1 {
        self.identity
    }
}

/// One population row paired with its recomputed admission decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmissionAuthoritativeRowV1 {
    /// Exact V4-authorized population row.
    pub population: PopulationRowV1,
    /// Exact sidecar evidence/verdict bound to `population`.
    pub admission: AdmissionDecisionRecordV1,
}

/// One bounded page whose two sources agreed under one shared lock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmissionAuthoritativePageV1 {
    /// Total decisions proved by both completion receipts.
    pub total: u64,
    /// Requested zero-based offset.
    pub offset: u64,
    /// Contiguous joined rows in canonical population sequence.
    pub rows: Vec<AdmissionAuthoritativeRowV1>,
    /// Exact authority digests applying to every returned row.
    pub authority: AdmissionAuthorityIdentityV1,
}

/// Read-only, single-population V4 plus admission authority.
#[derive(Debug)]
pub struct AdmissionAuthoritativeLedger {
    population_id: [u8; 32],
    populations: PopulationLedger,
    admissions: AdmissionAuthorityLedger,
    outer_lock: File,
    lock_path: PathBuf,
    snapshot: AdmissionAuthoritySnapshotV1,
}

impl AdmissionAuthoritativeLedger {
    /// Opens both ledgers and proves their receipts under one outer lock.
    ///
    /// # Errors
    ///
    /// Refuses an absent lock, either underlying open refusal, missing V4 or
    /// admission completion, policy/V4/count disagreement, or unlock failure.
    pub fn open_read(root: &Path, population_id: [u8; 32]) -> Result<Self, AdmissionJoinRefusal> {
        Self::open_impl(root, population_id, None)
    }

    /// Opens only when every underlying population/admission file fits the
    /// same caller-supplied byte ceiling.
    ///
    /// # Errors
    ///
    /// Every refusal from [`Self::open_read`], plus any underlying file over
    /// `max_bytes` before its index scan begins.
    pub fn open_read_bounded(
        root: &Path,
        population_id: [u8; 32],
        max_bytes: u64,
    ) -> Result<Self, AdmissionJoinRefusal> {
        Self::open_impl(root, population_id, Some(max_bytes))
    }

    fn open_impl(
        root: &Path,
        population_id: [u8; 32],
        max_bytes: Option<u64>,
    ) -> Result<Self, AdmissionJoinRefusal> {
        let lock_path = root.join("results").join("population-write.lock");
        let outer_lock = File::open(&lock_path)
            .map_err(|why| format!("{} could not be opened: {why}", lock_path.display()))?;
        outer_lock.lock_shared().map_err(|why| {
            format!(
                "{} could not be outer shared-locked for admission join open: {why}",
                lock_path.display()
            )
        })?;
        let opened = (|| {
            let populations = match max_bytes {
                Some(maximum) => PopulationLedger::open_read_bounded(root, maximum),
                None => PopulationLedger::open_read(root),
            }?;
            let admissions = match max_bytes {
                Some(maximum) => AdmissionAuthorityLedger::open_read_bounded(root, maximum),
                None => AdmissionAuthorityLedger::open_read(root),
            }?;
            let v4 = populations.receipt_v4(&population_id).ok_or_else(|| {
                format!(
                    "population {} has no V4 calendar-authoritative completion",
                    hex(&population_id)
                )
            })?;
            let admission = admissions.completion(population_id)?.ok_or_else(|| {
                format!(
                    "population {} has no admission-authoritative completion",
                    hex(&population_id)
                )
            })?;
            let identity = validate_receipts(population_id, &v4, &admission)?;
            let snapshot = AdmissionAuthoritySnapshotV1 {
                population_v4: v4,
                admission,
                identity,
            };
            Ok(Self {
                population_id,
                populations,
                admissions,
                outer_lock: outer_lock.try_clone().map_err(|why| {
                    format!(
                        "{} outer lock could not be cloned: {why}",
                        lock_path.display()
                    )
                })?,
                lock_path: lock_path.clone(),
                snapshot,
            })
        })();
        release_outer(&outer_lock, &lock_path, opened)
    }

    /// Population identity fixed by this read-only handle.
    #[must_use]
    pub const fn population_id(&self) -> [u8; 32] {
        self.population_id
    }

    /// Exact receipt digests validated at open and revalidated per page.
    #[must_use]
    pub const fn authority(&self) -> AdmissionAuthorityIdentityV1 {
        self.snapshot.identity()
    }

    /// Full immutable receipts validated together at open and revalidated per
    /// page.  Callers receive a borrow so the admission policy/count authority
    /// cannot be detached from this handle accidentally.
    #[must_use]
    pub const fn snapshot(&self) -> &AdmissionAuthoritySnapshotV1 {
        &self.snapshot
    }

    /// Reads and re-authorizes one bounded joined page under one outer lock.
    ///
    /// Every row is checked for population, sequence, strategy, exact
    /// population-payload digest, canonical policy recomputation and exact
    /// equality with the legacy row summary.
    ///
    /// # Errors
    ///
    /// Refuses either stale ledger, missing authority, page shape mismatch,
    /// any row binding/recomputation mismatch, or unlock failure.
    pub fn page(
        &mut self,
        offset: u64,
        limit: u64,
    ) -> Result<AdmissionAuthoritativePageV1, AdmissionJoinRefusal> {
        self.outer_lock.lock_shared().map_err(|why| {
            format!(
                "{} could not be outer shared-locked for admission page: {why}",
                self.lock_path.display()
            )
        })?;
        let joined = (|| {
            let population_page = self
                .populations
                .page_v4(&self.population_id, offset, limit)?
                .ok_or_else(|| {
                    format!(
                        "population {} lost V4 page authority",
                        hex(&self.population_id)
                    )
                })?;
            let admission_rows = self
                .admissions
                .page(self.population_id, offset, limit)?
                .ok_or_else(|| {
                    format!(
                        "population {} lost admission page authority",
                        hex(&self.population_id)
                    )
                })?;
            let admission = self
                .admissions
                .completion(self.population_id)?
                .ok_or_else(|| {
                    format!(
                        "population {} lost admission completion authority",
                        hex(&self.population_id)
                    )
                })?;
            let v4 = self
                .populations
                .receipt_v4(&self.population_id)
                .ok_or_else(|| {
                    format!(
                        "population {} lost V4 completion authority",
                        hex(&self.population_id)
                    )
                })?;
            let identity = validate_receipts(self.population_id, &v4, &admission)?;
            let current_snapshot = AdmissionAuthoritySnapshotV1 {
                population_v4: v4,
                admission,
                identity,
            };
            if current_snapshot != self.snapshot {
                return Err(format!(
                    "population {} authority receipts differ from this handle's open-time snapshot",
                    hex(&self.population_id)
                ));
            }
            join_page(
                self.population_id,
                offset,
                limit,
                population_page,
                admission_rows,
                current_snapshot.admission(),
                current_snapshot.identity(),
            )
        })();
        release_outer(&self.outer_lock, &self.lock_path, joined)
    }
}

fn validate_receipts(
    population_id: [u8; 32],
    v4: &CompletionReceiptV4,
    admission: &AdmissionCompletionReceiptV1,
) -> Result<AdmissionAuthorityIdentityV1, AdmissionJoinRefusal> {
    if v4.population_id() != population_id || admission.population_id() != population_id {
        return Err(format!(
            "admission join population {} disagrees with one or both completion identities",
            hex(&population_id)
        ));
    }
    let v4_digest = v4.content_digest()?;
    if admission.population_v4_completion_digest() != v4_digest {
        return Err(format!(
            "population {} admission completion binds a different V4 digest",
            hex(&population_id)
        ));
    }
    let v2 = v4.v3().v2();
    let policy_digest = admission.policy().digest();
    if v2.identities.admission_policy_digest != policy_digest {
        return Err(format!(
            "population {} V4 admission-policy identity differs from the canonical sidecar policy",
            hex(&population_id)
        ));
    }
    let stored_counts = (
        admission.decision_count(),
        admission.admitted_count(),
        admission.rejected_count(),
        admission.unmeasured_count(),
        admission.refused_count(),
    );
    let population_counts = (
        v2.row_count,
        v2.admitted_rows,
        v2.rejected_rows,
        v2.unmeasured_rows,
        v2.refused_rows,
    );
    if stored_counts != population_counts {
        return Err(format!(
            "population {} V4 row/status counts differ from admission completion counts",
            hex(&population_id)
        ));
    }
    Ok(AdmissionAuthorityIdentityV1 {
        population_v4_completion_digest: v4_digest,
        admission_completion_digest: admission.digest()?,
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "the page join keeps both bounded source pages and their exact request/authority terms explicit"
)]
fn join_page(
    population_id: [u8; 32],
    offset: u64,
    limit: u64,
    population_page: crate::population::PopulationPageV1,
    admission_rows: Vec<AdmissionDecisionRecordV1>,
    receipt: &AdmissionCompletionReceiptV1,
    authority: AdmissionAuthorityIdentityV1,
) -> Result<AdmissionAuthoritativePageV1, AdmissionJoinRefusal> {
    if population_page.total != receipt.decision_count() {
        return Err("population/admission joined page totals differ".into());
    }
    if population_page.offset != offset {
        return Err(format!(
            "population page returned offset {}, requested {offset}",
            population_page.offset
        ));
    }
    let expected = population_page.total.saturating_sub(offset).min(limit);
    let population_len = u64::try_from(population_page.rows.len())
        .map_err(|_| "population page length does not fit u64".to_owned())?;
    let admission_len = u64::try_from(admission_rows.len())
        .map_err(|_| "admission page length does not fit u64".to_owned())?;
    if population_len != expected || admission_len != expected {
        return Err(format!(
            "joined page expected {expected} rows but population returned {population_len} and admission returned {admission_len}"
        ));
    }
    let capacity = usize::try_from(expected)
        .map_err(|_| "joined page length does not fit usize".to_owned())?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(capacity)
        .map_err(|why| format!("joined admission page could not reserve {capacity} rows: {why}"))?;
    let policy_bytes = receipt.policy().canonical_bytes();
    for (relative, (population, admission)) in population_page
        .rows
        .into_iter()
        .zip(admission_rows)
        .enumerate()
    {
        let relative = u64::try_from(relative)
            .map_err(|_| "joined row position does not fit u64".to_owned())?;
        let expected_sequence = offset
            .checked_add(relative)
            .ok_or_else(|| "joined row sequence overflowed u64".to_owned())?;
        validate_row(
            population_id,
            expected_sequence,
            &population,
            &admission,
            &policy_bytes,
        )?;
        rows.push(AdmissionAuthoritativeRowV1 {
            population,
            admission,
        });
    }
    Ok(AdmissionAuthoritativePageV1 {
        total: population_page.total,
        offset,
        rows,
        authority,
    })
}

fn validate_row(
    population_id: [u8; 32],
    expected_sequence: u64,
    population: &PopulationRowV1,
    admission: &AdmissionDecisionRecordV1,
    policy_bytes: &[u8],
) -> Result<(), AdmissionJoinRefusal> {
    if population.population_id != population_id || admission.population_id() != population_id {
        return Err(format!(
            "joined row {expected_sequence} crosses population identities"
        ));
    }
    if population.sequence != expected_sequence || admission.row_sequence() != expected_sequence {
        return Err(format!(
            "joined row sequence differs from expected {expected_sequence}"
        ));
    }
    if population.strategy_digest != admission.strategy_digest() {
        return Err(format!(
            "joined row {expected_sequence} strategy digest differs"
        ));
    }
    if population.payload_digest()? != admission.row_payload_digest() {
        return Err(format!(
            "joined row {expected_sequence} population payload digest differs"
        ));
    }
    let evidence_bytes = admission.evidence().canonical_bytes();
    let verdict_bytes = admission.verdict().canonical_bytes();
    let recomputed = AdmissionDecisionSealV1::from_canonical_parts(
        policy_bytes,
        &evidence_bytes,
        &verdict_bytes,
    )
    .map_err(|why| {
        format!("joined row {expected_sequence} verdict is not policy-authorized: {why:?}")
    })?;
    population
        .admission
        .require_matches_verdict(recomputed.verdict())
        .map_err(|why| format!("joined row {expected_sequence} legacy admission differs: {why}"))
}

fn release_outer<T>(
    lock: &File,
    path: &Path,
    result: Result<T, AdmissionJoinRefusal>,
) -> Result<T, AdmissionJoinRefusal> {
    let released = lock.unlock().map_err(|why| {
        format!(
            "{} outer shared lock could not be released: {why}",
            path.display()
        )
    });
    match (result, released) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(why), _) | (Ok(_), Err(why)) => Err(why),
    }
}

fn hex(bytes: &[u8; 32]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push(char::from(
            DIGITS.get(usize::from(byte >> 4)).copied().unwrap_or(b'?'),
        ));
        out.push(char::from(
            DIGITS
                .get(usize::from(byte & 0x0f))
                .copied()
                .unwrap_or(b'?'),
        ));
    }
    out
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::panic,
    reason = "joined-ledger fixtures fail loudly and adversarially mutate exact durable bytes"
)]
#[allow(
    clippy::too_many_lines,
    reason = "the integrated fixture keeps every V4 and admission authority term visible beside its mismatch tests"
)]
mod tests {
    use std::fs::{File, OpenOptions};
    use std::io::{Seek, SeekFrom, Write};
    use std::sync::atomic::{AtomicU64, Ordering};

    use pull::calendar::{DayKind, OPEN_MINUTE, kind_of};
    use pull::session::Day;
    use runner::admission::{
        AdmissionEvidenceV1, AdmissionEvidenceValuesV1, AdmissionPolicyDraftV1, AdmissionPolicyV1,
        CompletenessV1, HypothesisDecisionV1, ObservedI64V1, ObservedU64V1,
    };

    use crate::admission_store::{
        AdmissionAuthorityLedger, AdmissionCompletionReceiptV1, AdmissionDecisionRecordV1,
    };
    use crate::population::{
        AdmissionStatusV1, AdmissionV1, ClosureV1, CompletionReceiptV4, CompletionReconciliationV2,
        ExitCellsPerMaskV2, ExitCoordinateV1, InstrumentFamilyV1, LongShortExitGridIdentitiesV2,
        PopulationIdentitiesV2, PopulationLedger, PopulationRowV1, RequestedSpanIdentityV1,
        SideExitGridIdentityV2, TopMetricsV1, TradeDirectionV1,
    };
    use crate::stored::{CompleteCalendarReceiptV2, calendar_receipt_v2};

    use super::{
        AdmissionAuthoritativeLedger, AdmissionAuthorityIdentityV1, join_page, validate_receipts,
        validate_row,
    };

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    fn root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "brutex-admission-join-{name}-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    const fn digest(seed: u8) -> [u8; 32] {
        [seed; 32]
    }

    fn policy(min_support_hits: u64) -> AdmissionPolicyV1 {
        AdmissionPolicyV1::new(AdmissionPolicyDraftV1 {
            min_support_hits: Some(min_support_hits),
            min_independent_sessions: Some(20),
            min_trades: Some(30),
            max_mae_paisa: Some(500),
            min_worst_reward_risk_ppm: Some(2_000_000),
            min_win_rate_ppm: Some(600_000),
            min_wilson_win_rate_ppm: Some(550_000),
            min_return_drawdown_ppm: Some(3_000_000),
            min_weakest_period_return_paisa: Some(0),
            max_pbo_ppm: Some(100_000),
            max_fwer_p_value_ppm: Some(50_000),
            max_spa_p_value_ppm: Some(50_000),
            min_decided_folds: Some(10),
            max_ambiguous_fill_rate_ppm: Some(100_000),
            max_gap_affected_rate_ppm: Some(100_000),
            max_session_concentration_ppm: Some(300_000),
            max_largest_trade_profit_share_ppm: Some(200_000),
            max_drawdown_paisa: Some(10_000),
            max_worst_trade_loss_paisa: Some(2_000),
            max_losing_trade_rate_ppm: Some(400_000),
            max_losing_trades: Some(20),
            min_pessimistic_profit_paisa: Some(1_000),
            min_winning_trades: Some(30),
            min_average_win_paisa: Some(300),
            max_average_loss_paisa: Some(150),
            min_profit_factor_ppm: Some(2_000_000),
            max_consecutive_losing_streak: Some(3),
            min_consecutive_winning_streak: Some(3),
            min_bootstrap_draws: Some(1_000),
            min_bootstrap_strategies: Some(100),
            min_bootstrap_periods: Some(300),
            min_pbo_contributing_folds: Some(10),
            max_pbo_unrankable_folds: Some(2),
            min_profitable_oos_folds: Some(8),
            min_oos_pessimistic_return_paisa: Some(1_000),
            max_white_reality_p_value_ppm: Some(50_000),
            require_white_reality_rejection: Some(true),
            max_romano_wolf_p_value_ppm: Some(50_000),
            require_romano_wolf_rejection: Some(true),
        })
        .expect("complete policy")
    }

    fn evidence(support_hits: u64) -> AdmissionEvidenceV1 {
        AdmissionEvidenceV1::new(AdmissionEvidenceValuesV1 {
            support_hits: ObservedU64V1::Measured(support_hits),
            independent_sessions: ObservedU64V1::Measured(20),
            trades: ObservedU64V1::Measured(50),
            max_mae_paisa: ObservedU64V1::Measured(500),
            worst_reward_risk_ppm: ObservedU64V1::Measured(2_000_000),
            win_rate_ppm: ObservedU64V1::Measured(600_000),
            wilson_win_rate_ppm: ObservedU64V1::Measured(550_000),
            return_drawdown_ppm: ObservedU64V1::Measured(3_000_000),
            weakest_period_return_paisa: ObservedI64V1::Measured(0),
            pbo_ppm: ObservedU64V1::Unmeasured,
            fwer_p_value_ppm: ObservedU64V1::Measured(50_000),
            spa_p_value_ppm: ObservedU64V1::Measured(50_000),
            decided_folds: ObservedU64V1::Measured(10),
            ambiguous_fill_rate_ppm: ObservedU64V1::Measured(100_000),
            gap_affected_rate_ppm: ObservedU64V1::Measured(100_000),
            session_concentration_ppm: ObservedU64V1::Measured(300_000),
            largest_trade_profit_share_ppm: ObservedU64V1::Measured(200_000),
            execution_complete: CompletenessV1::Complete,
            data_complete: CompletenessV1::Complete,
            calendar_complete: CompletenessV1::Complete,
            population_complete: CompletenessV1::Complete,
            drawdown_paisa: ObservedU64V1::Measured(10_000),
            worst_trade_loss_paisa: ObservedU64V1::Measured(2_000),
            losing_trade_rate_ppm: ObservedU64V1::Measured(400_000),
            losing_trades: ObservedU64V1::Measured(20),
            pessimistic_profit_paisa: ObservedI64V1::Measured(1_000),
            winning_trades: ObservedU64V1::Measured(30),
            average_win_paisa: ObservedU64V1::Measured(300),
            average_loss_paisa: ObservedU64V1::Measured(150),
            profit_factor_ppm: ObservedU64V1::Measured(2_000_000),
            consecutive_losing_streak: ObservedU64V1::Measured(3),
            consecutive_winning_streak: ObservedU64V1::Measured(3),
            bootstrap_draws: ObservedU64V1::Measured(1_000),
            bootstrap_strategies: ObservedU64V1::Measured(100),
            bootstrap_periods: ObservedU64V1::Measured(300),
            pbo_contributing_folds: ObservedU64V1::Unmeasured,
            pbo_unrankable_folds: ObservedU64V1::Unmeasured,
            profitable_oos_folds: ObservedU64V1::Measured(8),
            oos_pessimistic_return_paisa: ObservedI64V1::Measured(1_000),
            white_reality_p_value_ppm: ObservedU64V1::Measured(50_000),
            romano_wolf_p_value_ppm: ObservedU64V1::Measured(50_000),
            white_reality_decision: HypothesisDecisionV1::RejectedNull,
            romano_wolf_decision: HypothesisDecisionV1::RejectedNull,
            full_precision_statistics_complete: CompletenessV1::Complete,
        })
        .expect("in-domain evidence")
    }

    fn admission_projection(verdict: runner::admission::AdmissionVerdictV1) -> AdmissionV1 {
        let status = match verdict.status() {
            runner::admission::AdmissionStatusV1::Admitted => AdmissionStatusV1::Admitted,
            runner::admission::AdmissionStatusV1::Rejected => AdmissionStatusV1::Rejected,
            runner::admission::AdmissionStatusV1::Unmeasured => AdmissionStatusV1::Unmeasured,
            runner::admission::AdmissionStatusV1::Refused => AdmissionStatusV1::Refused,
        };
        AdmissionV1 {
            status,
            reasons: verdict.reasons().bits(),
            failed: verdict.failed().bits(),
            unmeasured: verdict.unmeasured().bits(),
            refused: verdict.refused().bits(),
        }
    }

    fn admission_summary(policy: &AdmissionPolicyV1, support_hits: u64) -> AdmissionV1 {
        admission_projection(policy.evaluate(&evidence(support_hits)))
    }

    fn row(
        population_id: [u8; 32],
        sequence: u64,
        direction: TradeDirectionV1,
        legacy: AdmissionV1,
    ) -> PopulationRowV1 {
        PopulationRowV1 {
            population_id,
            sequence,
            strategy_digest: digest(u8::try_from(sequence + 30).expect("fixture sequence")),
            mask_words: [1, 0, 0, 0, 0, 0],
            direction,
            instrument_family: InstrumentFamilyV1::Nifty,
            closure: ClosureV1::Closed,
            rung_seconds: 60,
            support_hits: 100,
            exit: ExitCoordinateV1 {
                stop: Some(0),
                target: Some(2),
                tsl: None,
                ttp: Some((1, 1)),
            },
            metrics: TopMetricsV1 {
                drawdown: 10_000,
                worst_loss: 2_000,
                losing_rate_ppm: 400_000,
                losing_trades: 2,
                loss_ratio_ppm: Some(200_000),
                pessimistic_profit: 1_000,
                winning_trades: 3,
                win_rate_ppm: 600_000,
                reward_to_risk_ppm: Some(2_000_000),
                average_win: 300,
                average_loss: 150,
                assurance_ppm: 600_000,
            },
            admission: legacy,
        }
    }

    fn span() -> RequestedSpanIdentityV1 {
        RequestedSpanIdentityV1::new(2025, 1, 2025, 1).expect("fixture month")
    }

    fn complete_calendar(rung_seconds: u32) -> CompleteCalendarReceiptV2 {
        let span = span();
        let first = Day::new(span.from_year(), span.from_month(), 1).expect("first day");
        let last = Day::new(span.to_year(), span.to_month(), 1)
            .expect("last month")
            .end_of_month();
        let first_day = i64::from(first.days_from_epoch());
        let last_day = i64::from(last.days_from_epoch());
        let width = i64::from(rung_seconds / 60);
        let mut timestamps = Vec::new();
        for day in first_day..=last_day {
            match kind_of(day) {
                DayKind::Open(session) => {
                    for window in session.windows.iter().take(usize::from(session.count)) {
                        for minute in window.from..=window.to {
                            let bucket =
                                i64::from(minute).saturating_sub(i64::from(OPEN_MINUTE)) / width;
                            let bucket_minute =
                                i64::from(OPEN_MINUTE).saturating_add(bucket.saturating_mul(width));
                            let timestamp = day
                                .saturating_mul(86_400_000_000)
                                .saturating_add(bucket_minute.saturating_mul(60_000_000))
                                .saturating_sub(19_800_000_000);
                            if timestamps.last().copied() != Some(timestamp) {
                                timestamps.push(timestamp);
                            }
                        }
                    }
                }
                DayKind::Closed => {}
                DayKind::OpenLengthUnmeasured | DayKind::Unmeasured => {
                    panic!("fixture calendar day is unmeasured")
                }
            }
        }
        calendar_receipt_v2(&timestamps, rung_seconds, first_day, last_day)
            .expect("calendar receipt")
            .require_complete()
            .expect("complete calendar")
    }

    fn identities(policy_digest: [u8; 32]) -> PopulationIdentitiesV2 {
        PopulationIdentitiesV2 {
            run_identity: digest(60),
            data_digest: digest(61),
            feed_digest: digest(62),
            source_commit_digest: digest(63),
            vocabulary_digest: digest(64),
            evaluation_policy_digest: digest(65),
            exit_grids: LongShortExitGridIdentitiesV2 {
                long: SideExitGridIdentityV2 {
                    policy_digest: digest(66),
                    resolved_digest: digest(67),
                },
                short: SideExitGridIdentityV2 {
                    policy_digest: digest(68),
                    resolved_digest: digest(69),
                },
            },
            admission_policy_digest: policy_digest,
            ranking_policy_digest: digest(70),
            calendar_policy_digest: digest(71),
            daily_reference_policy_digest: digest(72),
        }
    }

    fn reconciliation() -> CompletionReconciliationV2 {
        CompletionReconciliationV2 {
            sweep_trials: 4,
            frequent_itemsets: 1,
            infrequent_itemsets: 3,
            closed_itemsets: 1,
            redundant_itemsets: 0,
            unknown_closure_itemsets: 0,
            exit_cells_per_mask: ExitCellsPerMaskV2::new(1, 1).expect("one cell per side"),
            extinction_depth: 2,
            extinction_complete: true,
            closure_complete: true,
        }
    }

    fn commit_population(
        root: &std::path::Path,
        policy: &AdmissionPolicyV1,
        rows: &[PopulationRowV1],
    ) -> CompletionReceiptV4 {
        let population_id = rows.first().expect("one row").population_id;
        let receipt = CompletionReceiptV4::for_rows(
            population_id,
            InstrumentFamilyV1::Nifty,
            60,
            rows,
            span(),
            reconciliation(),
            identities(policy.digest()),
            complete_calendar(60),
            complete_calendar(60),
        )
        .expect("V4 receipt");
        let mut ledger = PopulationLedger::open(root).expect("population ledger");
        ledger
            .append_complete_v4(rows, &receipt)
            .expect("V4 population commit");
        receipt
    }

    fn sidecar_decision(
        row: &PopulationRowV1,
        policy: &AdmissionPolicyV1,
        support_hits: u64,
        strategy_digest: [u8; 32],
        payload_digest: [u8; 32],
    ) -> AdmissionDecisionRecordV1 {
        let sealed = policy.evaluate_sealed(&evidence(support_hits));
        AdmissionDecisionRecordV1::new(
            row.population_id,
            row.sequence,
            strategy_digest,
            payload_digest,
            &sealed,
        )
        .expect("sidecar decision")
    }

    fn exact_decisions(
        rows: &[PopulationRowV1],
        policy: &AdmissionPolicyV1,
        supports: &[u64],
    ) -> Vec<AdmissionDecisionRecordV1> {
        rows.iter()
            .zip(supports)
            .map(|(row, support)| {
                sidecar_decision(
                    row,
                    policy,
                    *support,
                    row.strategy_digest,
                    row.payload_digest().expect("row payload digest"),
                )
            })
            .collect()
    }

    fn commit_sidecar(
        root: &std::path::Path,
        population_id: [u8; 32],
        v4_digest: [u8; 32],
        policy: &AdmissionPolicyV1,
        decisions: &[AdmissionDecisionRecordV1],
    ) {
        let mut ledger = AdmissionAuthorityLedger::open(root).expect("admission ledger");
        ledger
            .commit(population_id, v4_digest, policy, decisions)
            .expect("admission commit");
    }

    fn valid_fixture(
        name: &str,
    ) -> (
        std::path::PathBuf,
        [u8; 32],
        AdmissionPolicyV1,
        Vec<PopulationRowV1>,
        CompletionReceiptV4,
    ) {
        let root = root(name);
        let population_id = digest(50);
        let policy = policy(100);
        let rows = vec![
            row(
                population_id,
                0,
                TradeDirectionV1::Long,
                admission_summary(&policy, 100),
            ),
            row(
                population_id,
                1,
                TradeDirectionV1::Short,
                admission_summary(&policy, 100),
            ),
        ];
        let v4 = commit_population(&root, &policy, &rows);
        let decisions = exact_decisions(&rows, &policy, &[100, 100]);
        commit_sidecar(
            &root,
            population_id,
            v4.content_digest().expect("V4 digest"),
            &policy,
            &decisions,
        );
        (root, population_id, policy, rows, v4)
    }

    fn cleanup(root: &std::path::Path) {
        std::fs::remove_dir_all(root).expect("fixture cleanup");
    }

    #[test]
    fn valid_join_opens_pages_and_outer_lock_blocks_a_writer() {
        let (root, population_id, policy, rows, v4) = valid_fixture("valid");
        let mut joined =
            AdmissionAuthoritativeLedger::open_read(&root, population_id).expect("joined open");
        let expected_decisions = exact_decisions(&rows, &policy, &[100, 100]);
        let expected_admission = AdmissionCompletionReceiptV1::derive(
            population_id,
            v4.content_digest().expect("V4 digest"),
            &policy,
            &expected_decisions,
        )
        .expect("admission receipt");
        assert_eq!(joined.snapshot().population_v4(), v4);
        assert_eq!(joined.snapshot().admission(), &expected_admission);
        assert_eq!(joined.snapshot().identity(), joined.authority());
        let page = joined.page(0, 2).expect("joined page");
        assert_eq!(page.total, 2);
        assert_eq!(page.offset, 0);
        assert_eq!(page.rows.len(), 2);
        assert_eq!(page.rows[0].population, rows[0]);
        assert_ne!(page.authority.admission_completion_digest, [0; 32]);

        joined.outer_lock.lock_shared().expect("outer shared lock");
        let contender = File::open(&joined.lock_path).expect("writer contender");
        assert!(contender.try_lock().is_err());
        joined.outer_lock.unlock().expect("release outer lock");
        drop(joined);
        cleanup(&root);
    }

    #[test]
    fn open_refuses_missing_sidecar_and_receipt_policy_v4_or_count_mismatch() {
        for mutation in 0_u8..4 {
            let root = root("receipt-mismatch");
            let population_id = digest(mutation.saturating_add(80));
            let population_policy = policy(100);
            let rows = vec![
                row(
                    population_id,
                    0,
                    TradeDirectionV1::Long,
                    admission_summary(&population_policy, 100),
                ),
                row(
                    population_id,
                    1,
                    TradeDirectionV1::Short,
                    admission_summary(&population_policy, 100),
                ),
            ];
            let v4 = commit_population(&root, &population_policy, &rows);
            if mutation == 0 {
                assert!(
                    AdmissionAuthoritativeLedger::open_read(&root, population_id)
                        .expect_err("missing sidecar")
                        .contains("population-admission")
                );
                cleanup(&root);
                continue;
            }
            let sidecar_policy = if mutation == 1 {
                policy(101)
            } else {
                population_policy
            };
            let decisions = exact_decisions(&rows, &sidecar_policy, &[101, 101]);
            let offered = if mutation == 2 {
                digest(99)
            } else {
                v4.content_digest().expect("V4 digest")
            };
            let offered_decisions = if mutation == 3 {
                decisions.get(..1).expect("one decision")
            } else {
                &decisions
            };
            commit_sidecar(
                &root,
                population_id,
                offered,
                &sidecar_policy,
                offered_decisions,
            );
            let why = AdmissionAuthoritativeLedger::open_read(&root, population_id)
                .expect_err("receipt mismatch");
            let expected = match mutation {
                1 => "policy",
                2 => "different V4",
                _ => "counts",
            };
            assert!(why.contains(expected), "{why}");
            cleanup(&root);
        }
    }

    #[test]
    fn row_join_refuses_strategy_payload_sequence_and_legacy_summary_mismatch() {
        let population_id = digest(110);
        let policy = policy(100);
        let baseline_row = row(
            population_id,
            0,
            TradeDirectionV1::Long,
            admission_summary(&policy, 100),
        );
        let exact = sidecar_decision(
            &baseline_row,
            &policy,
            100,
            baseline_row.strategy_digest,
            baseline_row.payload_digest().expect("payload digest"),
        );
        let policy_bytes = policy.canonical_bytes();
        assert!(
            validate_row(population_id, 1, &baseline_row, &exact, &policy_bytes)
                .expect_err("sequence mismatch")
                .contains("sequence")
        );
        let wrong_strategy = sidecar_decision(
            &baseline_row,
            &policy,
            100,
            digest(111),
            baseline_row.payload_digest().expect("payload digest"),
        );
        assert!(
            validate_row(
                population_id,
                0,
                &baseline_row,
                &wrong_strategy,
                &policy_bytes
            )
            .expect_err("strategy mismatch")
            .contains("strategy")
        );
        let wrong_payload = sidecar_decision(
            &baseline_row,
            &policy,
            100,
            baseline_row.strategy_digest,
            digest(112),
        );
        assert!(
            validate_row(
                population_id,
                0,
                &baseline_row,
                &wrong_payload,
                &policy_bytes
            )
            .expect_err("payload mismatch")
            .contains("payload")
        );
        let different_decision = sidecar_decision(
            &baseline_row,
            &policy,
            99,
            baseline_row.strategy_digest,
            baseline_row.payload_digest().expect("payload digest"),
        );
        assert!(
            validate_row(
                population_id,
                0,
                &baseline_row,
                &different_decision,
                &policy_bytes
            )
            .expect_err("legacy mismatch")
            .contains("legacy admission")
        );
    }

    #[test]
    fn page_refuses_stale_sidecar_mutation_and_forged_legacy_summary() {
        let (root, population_id, _, _, _) = valid_fixture("stale");
        let mut joined =
            AdmissionAuthoritativeLedger::open_read(&root, population_id).expect("joined open");
        let path = AdmissionAuthorityLedger::completion_path(&root);
        let mut external = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("external mutator");
        external
            .seek(SeekFrom::Start(16))
            .and_then(|_| external.write_all(&[123]))
            .and_then(|()| external.sync_all())
            .expect("same-length mutation");
        assert!(
            joined
                .page(0, 2)
                .expect_err("stale page")
                .contains("generation")
        );
        drop(joined);
        cleanup(&root);

        let population_id = digest(120);
        let policy = policy(100);
        let rows = vec![
            row(
                population_id,
                0,
                TradeDirectionV1::Long,
                admission_summary(&policy, 100),
            ),
            row(
                population_id,
                1,
                TradeDirectionV1::Short,
                admission_summary(&policy, 99),
            ),
        ];
        let decisions = exact_decisions(&rows, &policy, &[99, 100]);
        let population_page = crate::population::PopulationPageV1 {
            total: 2,
            offset: 0,
            rows,
        };
        let receipt = crate::admission_store::AdmissionCompletionReceiptV1::derive(
            population_id,
            digest(121),
            &policy,
            &decisions,
        )
        .expect("balanced receipt");
        let why = join_page(
            population_id,
            0,
            2,
            population_page,
            decisions,
            &receipt,
            AdmissionAuthorityIdentityV1 {
                population_v4_completion_digest: digest(121),
                admission_completion_digest: receipt.digest().expect("receipt digest"),
            },
        )
        .expect_err("swapped legacy summaries");
        assert!(why.contains("legacy admission"), "{why}");
    }

    #[test]
    fn pure_receipt_validation_refuses_identity_mismatch() {
        let root = root("identity");
        let population_id = digest(125);
        let policy = policy(100);
        let rows = vec![
            row(
                population_id,
                0,
                TradeDirectionV1::Long,
                admission_summary(&policy, 100),
            ),
            row(
                population_id,
                1,
                TradeDirectionV1::Short,
                admission_summary(&policy, 100),
            ),
        ];
        let v4 = commit_population(&root, &policy, &rows);
        let decisions = exact_decisions(&rows, &policy, &[100, 100]);
        let admission = crate::admission_store::AdmissionCompletionReceiptV1::derive(
            population_id,
            v4.content_digest().expect("V4 digest"),
            &policy,
            &decisions,
        )
        .expect("admission receipt");
        assert!(
            validate_receipts(digest(126), &v4, &admission)
                .expect_err("identity mismatch")
                .contains("identities")
        );
        cleanup(&root);
    }
}
