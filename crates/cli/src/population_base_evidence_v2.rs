//! Same-pass pre-finalization evidence for Candidate Universe V1 rows.
//!
//! A Candidate row retains the evaluated [`Cell`] but not the exact
//! [`TradeRow`] sequence that produced it.  Replaying every selected candidate
//! later would be both expensive and a second computation whose source could
//! drift.  This module therefore folds the authoritative rows at the only
//! lossless seam: immediately after `materialize_cell` and before those rows
//! are dropped.
//!
//! The Phase-A builder is deliberately **not** a durable authority by itself.
//! It builds fixed-size, sealed records in memory and binds them to the
//! completed Candidate receipt after the naturally-extinct population closes.
//! The sibling receipt-last ledger accepts only that opaque prepared block
//! after Candidate itself has freshly reopened, syncs records before its
//! completion, and returns an authority only after a fresh read-only reopen.
//!
//! One support-session scan is performed per closed mask and shared by every
//! long/short exit cell for that mask.  The scan is bounded explicitly and is
//! O(signal bars); the `TradeRow` fold is O(trades); sealing is O(candidates).
//! None of those whole-operation costs, nor ledger opening/append/retry, is
//! described as O(1). Only lookup and fixed-offset record read are O(1) after
//! the bounded opening scan.
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

use std::collections::HashSet;

use brutex_core::blake3::Hasher;
use indicators::Candle;
use indicators::column::{Column, Sourced};
use runner::admission::{
    AdmissionEvidenceV1, AdmissionEvidenceValuesV1, CompletenessV1, HypothesisDecisionV1,
    ObservedI64V1, ObservedU64V1,
};
use runner::grid::{Cell, TradeRow};

use super::{CandidateUniverseReceiptV1, CandidateUniverseRowV1};

#[path = "population_base_evidence_ledger_v2.rs"]
mod ledger;

pub use ledger::{
    BaseEvidenceLedgerBoundsV2, BaseEvidenceLedgerReaderV2, BaseEvidenceLedgerRefusalV2,
    BaseEvidenceRecordProjectionV2, BaseEvidenceReopenAuditV2,
};
pub(crate) use ledger::{
    BaseEvidenceProductionCommitV2, PairedBaseEvidenceAuthorityV2, PairedBaseEvidenceReaderV2,
    PairedBaseEvidenceRecordProjectionV2, append_and_reopen_base_evidence_v2,
    pair_base_evidence_authority_v2,
};

/// Canonical bytes in one Base Evidence V2 record, including its 32-byte seal.
pub(crate) const BASE_EVIDENCE_RECORD_BYTES_V2: usize = 1_024;

const PAYLOAD_BYTES: usize = BASE_EVIDENCE_RECORD_BYTES_V2 - 32;
const VERSION: u32 = 2;
const PPM: u64 = 1_000_000;
const MAGIC: [u8; 16] = *b"BTX-BASE-EV-V2\0\0";
const RECORD_SEAL_DOMAIN: &[u8] = b"brutex-base-evidence-v2-record-seal\0";
const EVIDENCE_ID_DOMAIN: &[u8] = b"brutex-base-evidence-v2-id\0";
const POLICY_ID_DOMAIN: &[u8] = b"brutex-base-evidence-v2-policy\0";
const CANDIDATE_ROW_DOMAIN: &[u8] = b"brutex-base-evidence-v2-candidate-row\0";
const TRADE_ROWS_DOMAIN: &[u8] = b"brutex-base-evidence-v2-trade-rows\0";
const ORDERED_RECORDS_DOMAIN: &[u8] = b"brutex-base-evidence-v2-ordered-records\0";
const GRAIN_COUNT: usize = 7;

const _: () = assert!(PAYLOAD_BYTES + 32 == BASE_EVIDENCE_RECORD_BYTES_V2);
const _: () = assert!(GRAIN_COUNT == crate::stability::GRAINS.len());

/// Typed fail-closed reason at the Phase-A evidence boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BaseEvidenceRefusalV2 {
    /// One explicit bound was zero or internally contradictory.
    InvalidBound(&'static str),
    /// A count exceeded the exact caller-supplied bound.
    BoundExceeded {
        /// Bounded resource.
        resource: &'static str,
        /// Observed or requested count.
        actual: u64,
        /// Explicit maximum.
        maximum: u64,
    },
    /// A required digest was absent.
    ZeroIdentity(&'static str),
    /// A Candidate, Cell, `TradeRow`, support or receipt identity disagreed.
    Reconciliation(String),
    /// Fixed bytes were malformed or corrupt.
    Codec(String),
    /// Checked arithmetic could not represent the exact result.
    Arithmetic(&'static str),
    /// Memory for the bounded preparation could not be reserved.
    Allocation(String),
}

impl core::fmt::Display for BaseEvidenceRefusalV2 {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidBound(name) => {
                write!(formatter, "Base Evidence V2 {name} bound is invalid")
            }
            Self::BoundExceeded {
                resource,
                actual,
                maximum,
            } => write!(
                formatter,
                "Base Evidence V2 {resource} count {actual} exceeds explicit maximum {maximum}"
            ),
            Self::ZeroIdentity(name) => {
                write!(formatter, "Base Evidence V2 {name} identity is zero")
            }
            Self::Reconciliation(why) => write!(formatter, "Base Evidence V2 refused: {why}"),
            Self::Codec(why) => write!(formatter, "Base Evidence V2 codec refused: {why}"),
            Self::Arithmetic(name) => {
                write!(formatter, "Base Evidence V2 {name} arithmetic overflowed")
            }
            Self::Allocation(why) => {
                write!(formatter, "Base Evidence V2 allocation refused: {why}")
            }
        }
    }
}

/// Every allocation and scan ceiling used by the Phase-A builder.
///
/// There is intentionally no `Default`; a caller must bind the preparation to
/// the same explicit Candidate/store limits that admitted its source streams.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BaseEvidenceBoundsV2 {
    candidates: u64,
    trades_per_candidate: u64,
    signal_bars_per_mask: u64,
    sessions_per_mask: u64,
}

impl BaseEvidenceBoundsV2 {
    pub(crate) fn new(
        max_candidates: u64,
        max_trades_per_candidate: u64,
        max_signal_bars_per_mask: u64,
        max_sessions_per_mask: u64,
    ) -> Result<Self, BaseEvidenceRefusalV2> {
        for (name, value) in [
            ("candidate", max_candidates),
            ("trades-per-candidate", max_trades_per_candidate),
            ("signal-bars-per-mask", max_signal_bars_per_mask),
            ("sessions-per-mask", max_sessions_per_mask),
        ] {
            if value == 0 {
                return Err(BaseEvidenceRefusalV2::InvalidBound(name));
            }
        }
        if max_sessions_per_mask > max_signal_bars_per_mask {
            return Err(BaseEvidenceRefusalV2::InvalidBound(
                "sessions-per-mask exceeds signal-bars-per-mask",
            ));
        }
        Ok(Self {
            candidates: max_candidates,
            trades_per_candidate: max_trades_per_candidate,
            signal_bars_per_mask: max_signal_bars_per_mask,
            sessions_per_mask: max_sessions_per_mask,
        })
    }

    /// Retains the exact signal masks through one explicitly bounded,
    /// allocation-fallible copy for the retirement callback.
    pub(super) fn try_clone_signal_column(
        self,
        column: &Column,
    ) -> Result<Column, BaseEvidenceRefusalV2> {
        require_bound(
            "signal masks retained for Base Evidence",
            usize_u64("signal mask count", column.len())?,
            self.signal_bars_per_mask,
        )?;
        column
            .try_clone_exact()
            .map_err(|why| BaseEvidenceRefusalV2::Allocation(format!("signal column clone: {why}")))
    }
}

/// Exact support count and causally independent IST sessions for one mask.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MaskSupportEvidenceV2 {
    mask_words: [u64; 6],
    support_hits: u64,
    independent_sessions: u64,
}

/// Scans one unique closed mask against the original signal-sourced column.
///
/// This function is called before long/short expansion, never per exit cell.
pub(crate) fn measure_mask_support_v2(
    mask_words: [u64; 6],
    expected_support_hits: u64,
    signal_bars: &[Candle],
    signal_column: &Column,
    bounds: BaseEvidenceBoundsV2,
) -> Result<MaskSupportEvidenceV2, BaseEvidenceRefusalV2> {
    let signal_count = usize_u64("signal bars", signal_bars.len())?;
    require_bound(
        "signal bars per mask",
        signal_count,
        bounds.signal_bars_per_mask,
    )?;
    if expected_support_hits == 0 {
        return Err(BaseEvidenceRefusalV2::Reconciliation(
            "closed frequent mask has zero support".to_owned(),
        ));
    }
    if signal_column.sourced() != Sourced::Signal {
        return Err(BaseEvidenceRefusalV2::Reconciliation(
            "independent sessions require the original signal-sourced column".to_owned(),
        ));
    }
    let census = signal_column.census();
    if !census.reconciles()
        || census.offered != signal_count
        || census.swept != usize_u64("signal column width", signal_column.bits().len())?
        || signal_column.bits().len() != signal_column.sources().len()
    {
        return Err(BaseEvidenceRefusalV2::Reconciliation(
            "signal column census, width or source map does not reconcile".to_owned(),
        ));
    }
    let mask = runner::replay_mask::from_stored_words(mask_words).map_err(|why| {
        BaseEvidenceRefusalV2::Reconciliation(format!("support mask is not canonical/live: {why}"))
    })?;
    if mask_words == [0; 6] {
        return Err(BaseEvidenceRefusalV2::Reconciliation(
            "support mask is empty".to_owned(),
        ));
    }

    let mut hits = 0_u64;
    let mut sessions = 0_u64;
    let mut previous_source = None;
    let mut previous_hit_day = None;
    for (bar_mask, source) in signal_column
        .bits()
        .iter()
        .zip(signal_column.sources().iter().copied())
    {
        if previous_source.is_some_and(|previous| source <= previous) {
            return Err(BaseEvidenceRefusalV2::Reconciliation(
                "signal column source indices are not strictly increasing".to_owned(),
            ));
        }
        previous_source = Some(source);
        if !bar_mask.hits(&mask) {
            continue;
        }
        let bar = signal_bars.get(source).ok_or_else(|| {
            BaseEvidenceRefusalV2::Reconciliation(format!(
                "signal source index {source} is outside {signal_count} bars"
            ))
        })?;
        hits = hits
            .checked_add(1)
            .ok_or(BaseEvidenceRefusalV2::Arithmetic("support hit count"))?;
        let day = indicators::ist_day(bar.ts_micros);
        if previous_hit_day != Some(day) {
            sessions = sessions
                .checked_add(1)
                .ok_or(BaseEvidenceRefusalV2::Arithmetic(
                    "independent session count",
                ))?;
            require_bound(
                "independent sessions per mask",
                sessions,
                bounds.sessions_per_mask,
            )?;
            previous_hit_day = Some(day);
        }
    }
    if hits != expected_support_hits {
        return Err(BaseEvidenceRefusalV2::Reconciliation(format!(
            "support scan measured {hits} hits, not closed-mask support {expected_support_hits}"
        )));
    }
    Ok(MaskSupportEvidenceV2 {
        mask_words,
        support_hits: hits,
        independent_sessions: sessions,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CandidateSubjectV2 {
    candidate_universe_id: [u8; 32],
    candidate_sequence: u64,
    candidate_semantic_id: [u8; 32],
    candidate_row_digest: [u8; 32],
    execution_run_id: [u8; 32],
    evaluated_grid_digest: [u8; 32],
    cell_ordinal: u64,
    family: u8,
    direction: u8,
    rung_seconds: u32,
    horizon_bars: u32,
}

impl CandidateSubjectV2 {
    fn from_row(row: &CandidateUniverseRowV1) -> Result<Self, BaseEvidenceRefusalV2> {
        let canonical = row
            .record()
            .map_err(BaseEvidenceRefusalV2::Reconciliation)?;
        Ok(Self {
            candidate_universe_id: row.universe_id(),
            candidate_sequence: row.sequence(),
            candidate_semantic_id: row.candidate_semantic_digest(),
            candidate_row_digest: candidate_row_digest_v2(&canonical),
            execution_run_id: row.execution_run_id(),
            evaluated_grid_digest: row.evaluated_grid_digest(),
            cell_ordinal: row.cell_ordinal(),
            family: super::family_byte(row.family()),
            direction: super::direction_byte(row.direction()),
            rung_seconds: row.rung_seconds(),
            horizon_bars: row.horizon_bars(),
        })
    }

    fn validate(self) -> Result<(), BaseEvidenceRefusalV2> {
        for (name, value) in [
            ("candidate universe", self.candidate_universe_id),
            ("candidate semantic", self.candidate_semantic_id),
            ("candidate row", self.candidate_row_digest),
            ("execution run", self.execution_run_id),
            ("evaluated grid", self.evaluated_grid_digest),
        ] {
            require_identity(name, value)?;
        }
        if self.rung_seconds == 0 || self.horizon_bars == 0 {
            return Err(BaseEvidenceRefusalV2::Reconciliation(
                "candidate rung or horizon is zero".to_owned(),
            ));
        }
        if !matches!(self.family, 1 | 2) || !matches!(self.direction, 1 | 2) {
            return Err(BaseEvidenceRefusalV2::Reconciliation(
                "candidate family or direction tag is unknown".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Base Evidence V2 identity of one exact sealed Candidate row record.
///
/// Keeping this helper beside its private domain prevents a successor
/// Population projection from inventing a second spelling of the same
/// identity. The input is the literal fixed Candidate record, including its
/// seal, rather than decoded fields that could be reassembled differently.
pub(crate) fn candidate_row_digest_v2(
    canonical_record: &[u8; super::ROW_STRIDE_BYTES],
) -> [u8; 32] {
    digest_domain(CANDIDATE_ROW_DOMAIN, canonical_record)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TradeAggregatesV2 {
    trades: u64,
    wins: u64,
    losses: u64,
    pessimistic: i64,
    optimistic: i64,
    gross_win: i64,
    gross_loss: i64,
    best_trade: i64,
    /// The smallest win that EXCEEDED its own execution bracket — D-0595.
    ///
    /// # The counter that is deliberately NOT beside it
    ///
    /// Zero now carries two meanings — nothing won, or nothing won by more than
    /// its own pricing uncertainty — and a `scratch_wins` field would separate
    /// them. It is not added: this struct has `encode_aggregates` and
    /// `decode_aggregates` at a FIXED width, so a new field is a new file
    /// version at its own stride under `CLAUDE.md` §4, and §3 rule 8 forbids
    /// mutating a format in place. The ambiguity is recorded here rather than
    /// paid for with a format version nothing else needs yet.
    min_win: i64,
    worst_trade: i64,
    max_drawdown: i64,
    ambiguous_bars: u64,
    gapped: u64,
    max_losing_streak: u32,
    max_winning_streak: u32,
    max_mae_paisa: u64,
    max_session_trades: u64,
    weakest_period_return_paisa: [i64; GRAIN_COUNT],
}

impl TradeAggregatesV2 {
    fn validate(self) -> Result<(), BaseEvidenceRefusalV2> {
        if self.wins > self.trades
            || self.losses != self.trades.saturating_sub(self.wins)
            || self.ambiguous_bars > self.trades
            || self.gapped > self.trades
            || self.gross_win < 0
            || self.gross_loss > 0
            || self.best_trade < 0
            || self.min_win < 0
            || self.worst_trade > 0
            || self.max_drawdown < 0
            || self.max_session_trades > self.trades
        {
            return Err(BaseEvidenceRefusalV2::Reconciliation(
                "trade aggregates violate count, sign or subset domains".to_owned(),
            ));
        }
        if self.trades == 0
            && (self.max_mae_paisa != 0
                || self.max_session_trades != 0
                || self.weakest_period_return_paisa != [0; GRAIN_COUNT])
        {
            return Err(BaseEvidenceRefusalV2::Reconciliation(
                "empty trade aggregates carry measured row-only evidence".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BaseEvidenceDraftV2 {
    subject: CandidateSubjectV2,
    support_hits: u64,
    independent_sessions: u64,
    trade_rows_digest: [u8; 32],
    aggregates: TradeAggregatesV2,
}

impl BaseEvidenceDraftV2 {
    fn measure(
        subject: CandidateSubjectV2,
        support: MaskSupportEvidenceV2,
        expected: &Cell,
        rows: &[TradeRow],
        bounds: BaseEvidenceBoundsV2,
    ) -> Result<Self, BaseEvidenceRefusalV2> {
        subject.validate()?;
        if support.support_hits == 0 || support.independent_sessions == 0 {
            return Err(BaseEvidenceRefusalV2::Reconciliation(
                "frequent mask support/session evidence is empty".to_owned(),
            ));
        }
        let row_count = usize_u64("trade rows", rows.len())?;
        require_bound(
            "trades per candidate",
            row_count,
            bounds.trades_per_candidate,
        )?;
        let (aggregates, trade_rows_digest) = fold_trade_rows(expected, rows)?;
        Ok(Self {
            subject,
            support_hits: support.support_hits,
            independent_sessions: support.independent_sessions,
            trade_rows_digest,
            aggregates,
        })
    }
}

/// One sealed fixed-width Phase-A Base Evidence record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BaseEvidenceRecordV2 {
    evidence_id: [u8; 32],
    candidate_completion_digest: [u8; 32],
    candidate_ordered_row_digest: [u8; 32],
    base_policy_digest: [u8; 32],
    draft: BaseEvidenceDraftV2,
}

impl BaseEvidenceRecordV2 {
    fn seal(
        source: BaseEvidenceFamilySourceV2,
        draft: &BaseEvidenceDraftV2,
    ) -> Result<Self, BaseEvidenceRefusalV2> {
        source.validate()?;
        if draft.subject.candidate_universe_id != source.universe_id {
            return Err(BaseEvidenceRefusalV2::Reconciliation(
                "draft belongs to another Candidate universe".to_owned(),
            ));
        }
        let mut value = Self {
            evidence_id: [0; 32],
            candidate_completion_digest: source.completion_digest,
            candidate_ordered_row_digest: source.ordered_row_digest,
            base_policy_digest: base_policy_digest_v2(),
            draft: *draft,
        };
        value.evidence_id = derive_evidence_id(&value);
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<(), BaseEvidenceRefusalV2> {
        self.draft.subject.validate()?;
        self.draft.aggregates.validate()?;
        for (name, value) in [
            ("evidence", self.evidence_id),
            ("Candidate completion", self.candidate_completion_digest),
            ("Candidate ordered rows", self.candidate_ordered_row_digest),
            ("base policy", self.base_policy_digest),
            ("TradeRows", self.draft.trade_rows_digest),
        ] {
            require_identity(name, value)?;
        }
        if self.base_policy_digest != base_policy_digest_v2() {
            return Err(BaseEvidenceRefusalV2::Reconciliation(
                "base evidence policy digest is foreign".to_owned(),
            ));
        }
        if self.evidence_id != derive_evidence_id(&self) {
            return Err(BaseEvidenceRefusalV2::Reconciliation(
                "base evidence identity does not reproduce".to_owned(),
            ));
        }
        AdmissionEvidenceV1::new(self.admission_values()?).map_err(|why| {
            BaseEvidenceRefusalV2::Reconciliation(format!(
                "base admission projection is not canonical: {why:?}"
            ))
        })?;
        Ok(())
    }

    /// Exact Candidate semantic identity, never a final Population/Selection ID.
    #[cfg(test)]
    pub(crate) const fn candidate_semantic_id(self) -> [u8; 32] {
        self.draft.subject.candidate_semantic_id
    }

    /// Canonical Base Evidence identity.
    #[cfg(test)]
    pub(crate) const fn evidence_id(self) -> [u8; 32] {
        self.evidence_id
    }

    /// Exact 27-field base projection; all 17 Statistics/walk fields remain
    /// explicitly unmeasured.
    pub(crate) fn admission_values(
        self,
    ) -> Result<AdmissionEvidenceValuesV1, BaseEvidenceRefusalV2> {
        let a = self.draft.aggregates;
        let row_values_measured = a.trades > 0;
        let worst_loss = negative_magnitude(a.worst_trade);
        let gross_loss = negative_magnitude(a.gross_loss);
        let min_win = nonnegative_u64("minimum winning trade", a.min_win)?;
        let best_trade = nonnegative_u64("best trade", a.best_trade)?;
        let gross_win = nonnegative_u64("gross win", a.gross_win)?;
        let drawdown = nonnegative_u64("drawdown", a.max_drawdown)?;
        let weakest = if row_values_measured {
            ObservedI64V1::Measured(*a.weakest_period_return_paisa.iter().min().unwrap_or(&0))
        } else {
            ObservedI64V1::Unmeasured
        };
        Ok(AdmissionEvidenceValuesV1 {
            support_hits: ObservedU64V1::Measured(self.draft.support_hits),
            independent_sessions: ObservedU64V1::Measured(self.draft.independent_sessions),
            trades: ObservedU64V1::Measured(a.trades),
            max_mae_paisa: measured_when(row_values_measured, a.max_mae_paisa),
            worst_reward_risk_ppm: ratio_observed(min_win, worst_loss, "worst reward/risk")?,
            win_rate_ppm: measured_rate(a.wins, a.trades, "win rate")?,
            wilson_win_rate_ppm: ObservedU64V1::Unmeasured,
            return_drawdown_ppm: if row_values_measured {
                return_drawdown(a.pessimistic, a.max_drawdown)?
            } else {
                ObservedU64V1::Unmeasured
            },
            weakest_period_return_paisa: weakest,
            pbo_ppm: ObservedU64V1::Unmeasured,
            fwer_p_value_ppm: ObservedU64V1::Unmeasured,
            spa_p_value_ppm: ObservedU64V1::Unmeasured,
            decided_folds: ObservedU64V1::Unmeasured,
            ambiguous_fill_rate_ppm: measured_rate(
                a.ambiguous_bars,
                a.trades,
                "ambiguous fill rate",
            )?,
            gap_affected_rate_ppm: measured_rate(a.gapped, a.trades, "gap affected rate")?,
            session_concentration_ppm: if row_values_measured {
                ObservedU64V1::Measured(rate_ppm(
                    a.max_session_trades,
                    a.trades,
                    "session concentration",
                )?)
            } else {
                ObservedU64V1::Unmeasured
            },
            largest_trade_profit_share_ppm: if a.gross_win > 0 {
                ObservedU64V1::Measured(rate_ppm(
                    best_trade,
                    gross_win,
                    "largest trade profit share",
                )?)
            } else {
                ObservedU64V1::Unmeasured
            },
            execution_complete: CompletenessV1::Complete,
            // Candidate V1's typed slices do not yet attest concrete store
            // origin.  Phase A must not turn self-consistency into data origin.
            data_complete: CompletenessV1::Unmeasured,
            calendar_complete: CompletenessV1::Complete,
            population_complete: CompletenessV1::Complete,
            drawdown_paisa: ObservedU64V1::Measured(drawdown),
            worst_trade_loss_paisa: ObservedU64V1::Measured(worst_loss),
            losing_trade_rate_ppm: measured_rate(a.losses, a.trades, "losing trade rate")?,
            losing_trades: ObservedU64V1::Measured(a.losses),
            pessimistic_profit_paisa: ObservedI64V1::Measured(a.pessimistic),
            winning_trades: ObservedU64V1::Measured(a.wins),
            average_win_paisa: gross_win
                .checked_div(a.wins)
                .map_or(ObservedU64V1::Unmeasured, ObservedU64V1::Measured),
            average_loss_paisa: gross_loss
                .checked_div(a.losses)
                .map_or(ObservedU64V1::Unmeasured, ObservedU64V1::Measured),
            profit_factor_ppm: ratio_observed(gross_win, gross_loss, "profit factor")?,
            consecutive_losing_streak: measured_when(
                row_values_measured,
                u64::from(a.max_losing_streak),
            ),
            consecutive_winning_streak: measured_when(
                row_values_measured,
                u64::from(a.max_winning_streak),
            ),
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

    fn encode(self) -> Result<[u8; BASE_EVIDENCE_RECORD_BYTES_V2], BaseEvidenceRefusalV2> {
        self.validate()?;
        let mut payload = [0_u8; PAYLOAD_BYTES];
        let mut writer = FixedWriter::new(&mut payload);
        writer.bytes(&MAGIC)?;
        writer.u32(VERSION)?;
        writer.u32(0)?;
        writer.digest(self.evidence_id)?;
        let subject = self.draft.subject;
        for value in [
            subject.candidate_universe_id,
            self.candidate_completion_digest,
            self.candidate_ordered_row_digest,
            subject.candidate_semantic_id,
            subject.candidate_row_digest,
            subject.execution_run_id,
            subject.evaluated_grid_digest,
            self.draft.trade_rows_digest,
            self.base_policy_digest,
        ] {
            writer.digest(value)?;
        }
        writer.u64(subject.candidate_sequence)?;
        writer.u64(subject.cell_ordinal)?;
        writer.u64(self.draft.support_hits)?;
        writer.u64(self.draft.independent_sessions)?;
        writer.u8(subject.family)?;
        writer.u8(subject.direction)?;
        writer.u16(0)?;
        writer.u32(subject.rung_seconds)?;
        writer.u32(subject.horizon_bars)?;
        encode_aggregates(&mut writer, self.draft.aggregates)?;
        writer.require_remaining_zero();
        let mut record = [0_u8; BASE_EVIDENCE_RECORD_BYTES_V2];
        record[..PAYLOAD_BYTES].copy_from_slice(&payload);
        record[PAYLOAD_BYTES..].copy_from_slice(&digest_domain(RECORD_SEAL_DOMAIN, &payload));
        Ok(record)
    }

    fn decode(record: &[u8]) -> Result<Self, BaseEvidenceRefusalV2> {
        if record.len() != BASE_EVIDENCE_RECORD_BYTES_V2 {
            return Err(BaseEvidenceRefusalV2::Codec(format!(
                "record has {} bytes, not {BASE_EVIDENCE_RECORD_BYTES_V2}",
                record.len()
            )));
        }
        let (payload, seal) = record.split_at(PAYLOAD_BYTES);
        if seal != digest_domain(RECORD_SEAL_DOMAIN, payload) {
            return Err(BaseEvidenceRefusalV2::Codec(
                "record seal does not match payload".to_owned(),
            ));
        }
        let mut reader = FixedReader::new(payload);
        if reader.array::<16>()? != MAGIC || reader.u32()? != VERSION {
            return Err(BaseEvidenceRefusalV2::Codec(
                "record magic or version is unknown".to_owned(),
            ));
        }
        reader.require_zero_u32("header reserve")?;
        let evidence_id = reader.digest()?;
        let candidate_universe_id = reader.digest()?;
        let candidate_completion_digest = reader.digest()?;
        let candidate_ordered_row_digest = reader.digest()?;
        let candidate_semantic_id = reader.digest()?;
        let candidate_row_digest = reader.digest()?;
        let execution_run_id = reader.digest()?;
        let evaluated_grid_digest = reader.digest()?;
        let trade_rows_digest = reader.digest()?;
        let base_policy_digest = reader.digest()?;
        let candidate_sequence = reader.u64()?;
        let cell_ordinal = reader.u64()?;
        let support_hits = reader.u64()?;
        let independent_sessions = reader.u64()?;
        let family = reader.u8()?;
        let direction = reader.u8()?;
        reader.require_zero_u16("subject reserve")?;
        let rung_seconds = reader.u32()?;
        let horizon_bars = reader.u32()?;
        let aggregates = decode_aggregates(&mut reader)?;
        reader.require_zeros("record tail reserve")?;
        let value = Self {
            evidence_id,
            candidate_completion_digest,
            candidate_ordered_row_digest,
            base_policy_digest,
            draft: BaseEvidenceDraftV2 {
                subject: CandidateSubjectV2 {
                    candidate_universe_id,
                    candidate_sequence,
                    candidate_semantic_id,
                    candidate_row_digest,
                    execution_run_id,
                    evaluated_grid_digest,
                    cell_ordinal,
                    family,
                    direction,
                    rung_seconds,
                    horizon_bars,
                },
                support_hits,
                independent_sessions,
                trade_rows_digest,
                aggregates,
            },
        };
        value.validate()?;
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BaseEvidenceFamilySourceV2 {
    universe_id: [u8; 32],
    completion_digest: [u8; 32],
    ordered_row_digest: [u8; 32],
    row_count: u64,
}

impl BaseEvidenceFamilySourceV2 {
    fn from_receipt(receipt: &CandidateUniverseReceiptV1) -> Self {
        Self {
            universe_id: receipt.universe_id(),
            completion_digest: receipt.content_digest(),
            ordered_row_digest: receipt.ordered_row_digest(),
            row_count: receipt.row_count(),
        }
    }

    fn validate(self) -> Result<(), BaseEvidenceRefusalV2> {
        for (name, value) in [
            ("Candidate universe", self.universe_id),
            ("Candidate completion", self.completion_digest),
            ("Candidate ordered rows", self.ordered_row_digest),
        ] {
            require_identity(name, value)?;
        }
        Ok(())
    }
}

/// In-memory Phase-A family sealed against one Candidate completion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreparedBaseEvidenceV2 {
    source: BaseEvidenceFamilySourceV2,
    records: Vec<BaseEvidenceRecordV2>,
    ordered_record_digest: [u8; 32],
}

impl PreparedBaseEvidenceV2 {
    pub(super) fn validate_candidate_receipt(
        &self,
        receipt: &CandidateUniverseReceiptV1,
    ) -> Result<(), BaseEvidenceRefusalV2> {
        let source = BaseEvidenceFamilySourceV2::from_receipt(receipt);
        if source != self.source
            || source.row_count != usize_u64("prepared Base Evidence records", self.records.len())?
        {
            return Err(BaseEvidenceRefusalV2::Reconciliation(
                "prepared Base Evidence no longer matches its Candidate receipt".to_owned(),
            ));
        }
        if self.ordered_record_digest == [0; 32] {
            return Err(BaseEvidenceRefusalV2::ZeroIdentity(
                "ordered Base Evidence records",
            ));
        }
        Ok(())
    }

    /// Number of records, exactly one per Candidate row.
    #[cfg(test)]
    pub(crate) fn record_count(&self) -> usize {
        self.records.len()
    }

    /// Canonical ordered Phase-A record digest.  This is not a durable receipt.
    #[cfg(test)]
    pub(crate) const fn ordered_record_digest(&self) -> [u8; 32] {
        self.ordered_record_digest
    }

    /// Exact prepared records in Candidate sequence order.
    #[cfg(test)]
    pub(crate) fn records(&self) -> &[BaseEvidenceRecordV2] {
        &self.records
    }
}

/// Bounded same-pass builder retained beside Candidate row preparation.
#[derive(Debug)]
pub(crate) struct BaseEvidenceBuilderV2 {
    bounds: BaseEvidenceBoundsV2,
    semantics: HashSet<[u8; 32]>,
    drafts: Vec<BaseEvidenceDraftV2>,
}

impl BaseEvidenceBuilderV2 {
    pub(crate) fn new(bounds: BaseEvidenceBoundsV2) -> Self {
        Self {
            bounds,
            semantics: HashSet::new(),
            drafts: Vec::new(),
        }
    }

    pub(crate) fn observe_candidate(
        &mut self,
        row: &CandidateUniverseRowV1,
        support: MaskSupportEvidenceV2,
        expected: &Cell,
        rows: &[TradeRow],
    ) -> Result<(), BaseEvidenceRefusalV2> {
        let next = usize_u64("candidate records", self.drafts.len())?
            .checked_add(1)
            .ok_or(BaseEvidenceRefusalV2::Arithmetic("candidate record count"))?;
        require_bound("candidate records", next, self.bounds.candidates)?;
        if row.mask_words() != support.mask_words
            || row.support_hits() != support.support_hits
            || row.sequence() != usize_u64("candidate sequence", self.drafts.len())?
        {
            return Err(BaseEvidenceRefusalV2::Reconciliation(
                "Candidate row does not match its once-per-mask support or canonical sequence"
                    .to_owned(),
            ));
        }
        self.semantics
            .try_reserve(1)
            .map_err(|why| BaseEvidenceRefusalV2::Allocation(format!("semantic index: {why}")))?;
        if !self.semantics.insert(row.candidate_semantic_digest()) {
            return Err(BaseEvidenceRefusalV2::Reconciliation(
                "Candidate semantic identity was duplicated".to_owned(),
            ));
        }
        let draft = BaseEvidenceDraftV2::measure(
            CandidateSubjectV2::from_row(row)?,
            support,
            expected,
            rows,
            self.bounds,
        )?;
        self.drafts
            .try_reserve(1)
            .map_err(|why| BaseEvidenceRefusalV2::Allocation(format!("record vector: {why}")))?;
        self.drafts.push(draft);
        Ok(())
    }

    pub(crate) fn seal(
        self,
        receipt: &CandidateUniverseReceiptV1,
    ) -> Result<PreparedBaseEvidenceV2, BaseEvidenceRefusalV2> {
        let source = BaseEvidenceFamilySourceV2::from_receipt(receipt);
        source.validate()?;
        let count = usize_u64("prepared Base Evidence records", self.drafts.len())?;
        if count != source.row_count {
            return Err(BaseEvidenceRefusalV2::Reconciliation(format!(
                "Base Evidence has {count} records, not Candidate completion count {}",
                source.row_count
            )));
        }
        let mut records = Vec::new();
        records
            .try_reserve_exact(self.drafts.len())
            .map_err(|why| BaseEvidenceRefusalV2::Allocation(format!("sealed records: {why}")))?;
        let mut ordered = Hasher::new();
        ordered.update(ORDERED_RECORDS_DOMAIN);
        ordered.update(&source.universe_id);
        ordered.update(&source.completion_digest);
        ordered.update(&source.ordered_row_digest);
        ordered.update(&source.row_count.to_le_bytes());
        for (index, draft) in self.drafts.into_iter().enumerate() {
            if draft.subject.candidate_sequence != usize_u64("sealed sequence", index)? {
                return Err(BaseEvidenceRefusalV2::Reconciliation(
                    "Base Evidence drafts are not in canonical Candidate order".to_owned(),
                ));
            }
            let record = BaseEvidenceRecordV2::seal(source, &draft)?;
            let bytes = record.encode()?;
            ordered.update(&bytes);
            records.push(record);
        }
        let ordered_record_digest = ordered.finalize();
        require_identity("ordered Base Evidence records", ordered_record_digest)?;
        Ok(PreparedBaseEvidenceV2 {
            source,
            records,
            ordered_record_digest,
        })
    }
}

#[cfg(test)]
pub(super) fn fixture_prepared_base_evidence_v2(
    receipt: &CandidateUniverseReceiptV1,
    rows: &[CandidateUniverseRowV1],
) -> Result<PreparedBaseEvidenceV2, BaseEvidenceRefusalV2> {
    let source = BaseEvidenceFamilySourceV2::from_receipt(receipt);
    let mut records = Vec::with_capacity(rows.len());
    let mut ordered = Hasher::new();
    ordered.update(ORDERED_RECORDS_DOMAIN);
    ordered.update(&source.universe_id);
    ordered.update(&source.completion_digest);
    ordered.update(&source.ordered_row_digest);
    ordered.update(&source.row_count.to_le_bytes());
    for row in rows {
        let row_bytes = row.record().map_err(BaseEvidenceRefusalV2::Codec)?;
        let draft = BaseEvidenceDraftV2 {
            subject: CandidateSubjectV2::from_row(row)?,
            support_hits: row.support_hits(),
            independent_sessions: 1,
            trade_rows_digest: digest_domain(b"brutex-base-ledger-fixture-trades\0", &row_bytes),
            aggregates: TradeAggregatesV2 {
                trades: 0,
                wins: 0,
                losses: 0,
                pessimistic: 0,
                optimistic: 0,
                gross_win: 0,
                gross_loss: 0,
                best_trade: 0,
                min_win: 0,
                worst_trade: 0,
                max_drawdown: 0,
                ambiguous_bars: 0,
                gapped: 0,
                max_losing_streak: 0,
                max_winning_streak: 0,
                max_mae_paisa: 0,
                max_session_trades: 0,
                weakest_period_return_paisa: [0; GRAIN_COUNT],
            },
        };
        let record = BaseEvidenceRecordV2::seal(source, &draft)?;
        ordered.update(&record.encode()?);
        records.push(record);
    }
    let value = PreparedBaseEvidenceV2 {
        source,
        records,
        ordered_record_digest: ordered.finalize(),
    };
    value.validate_candidate_receipt(receipt)?;
    Ok(value)
}

#[expect(
    clippy::too_many_lines,
    reason = "one authoritative same-pass fold keeps digest, aggregate and Cell reconciliation visibly inseparable"
)]
fn fold_trade_rows(
    expected: &Cell,
    rows: &[TradeRow],
) -> Result<(TradeAggregatesV2, [u8; 32]), BaseEvidenceRefusalV2> {
    let mut digest = Hasher::new();
    digest.update(TRADE_ROWS_DOMAIN);
    digest.update(&usize_u64("trade row count", rows.len())?.to_le_bytes());
    let mut pessimistic = 0_i64;
    let mut optimistic = 0_i64;
    let mut gross_win = 0_i64;
    let mut gross_loss = 0_i64;
    let mut wins = 0_u64;
    let mut best_trade = 0_i64;
    let mut min_win = 0_i64;
    let mut worst_trade = 0_i64;
    let mut running = 0_i64;
    let mut peak = 0_i64;
    let mut max_drawdown = 0_i64;
    let mut winning_streak = 0_u32;
    let mut losing_streak = 0_u32;
    let mut max_winning_streak = 0_u32;
    let mut max_losing_streak = 0_u32;
    let mut max_mae_paisa = 0_u64;
    let mut max_session_trades = 0_u64;
    let mut session_trades = 0_u64;
    let mut session_day = None;
    let mut previous_entry = None;
    let mut occupied_through = None;
    let mut grains = [GrainAccumulatorV2::default(); GRAIN_COUNT];

    for row in rows {
        if row.signal_bar > row.entry_bar
            || row.entry_bar > row.exit_bar
            || row.entry_micros > row.exit_micros
            || previous_entry.is_some_and(|previous| row.entry_micros <= previous)
            || occupied_through.is_some_and(|previous| row.entry_micros <= previous)
        {
            return Err(BaseEvidenceRefusalV2::Reconciliation(
                "TradeRows are out of order, overlap, or carry reversed indices/timestamps"
                    .to_owned(),
            ));
        }
        if row.worst > row.best
            || row.adverse < 0
            || row.adverse_paisa < 0
            || row.favourable < 0
            || row.favourable_paisa < 0
        {
            return Err(BaseEvidenceRefusalV2::Reconciliation(
                "TradeRow price ordering or excursion magnitude is invalid".to_owned(),
            ));
        }
        hash_trade_row(&mut digest, row)?;
        previous_entry = Some(row.entry_micros);
        occupied_through = Some(row.exit_micros);
        pessimistic = checked_i64_add(pessimistic, row.worst, "pessimistic sum")?;
        optimistic = checked_i64_add(optimistic, row.best, "optimistic sum")?;
        max_mae_paisa = max_mae_paisa.max(row.adverse_paisa.unsigned_abs());
        if row.worst > 0 {
            wins = wins
                .checked_add(1)
                .ok_or(BaseEvidenceRefusalV2::Arithmetic("winning trade count"))?;
            gross_win = checked_i64_add(gross_win, row.worst, "gross win")?;
            best_trade = best_trade.max(row.worst);
            // A SCRATCH IS NOT THE SMALLEST WIN, AND THE THRESHOLD IS NOT A
            // NUMBER -- D-0595.
            //
            // The operator's rule is `min(win) >= 3x max(loss)`, and `min_win`
            // took the smallest STRICTLY POSITIVE trade. One trade that gained
            // a single paisa therefore set it to 1 and collapsed the ratio to
            // nearly zero, however large the real winners were. That was the
            // rule working exactly as written and it was still the wrong answer
            // to the question the rule asks.
            //
            // The repair is not a constant. `row.best` and `row.worst` are ONE
            // trade priced under the best and the worst reading of both legs,
            // so their difference is that trade's own execution uncertainty. A
            // gain no larger than that bracket is a win only under one of two
            // equally admissible readings -- flip the intra-bar ordering and it
            // is a loss. It cannot be the evidence a 3:1 rule rests on.
            //
            // So a win counts toward `min_win` when it EXCEEDS its own bracket,
            // and is a scratch otherwise. Self-scaling across instruments and
            // rungs, measured rather than declared, and with no parameter that
            // can be set wrongly -- which is §6's argument, honoured by having
            // nothing to set.
            //
            // NOT the direction `Rules::fills_hold` takes. That rule demands
            // `optimistic >= 2 * pessimistic` -- that the uncertainty be LARGE
            // beside the profit. This demands the profit be large beside the
            // uncertainty. They are opposite tests and only one of them is
            // about evidence.
            //
            // `wins`, `gross_win`, `best_trade` and every streak are untouched:
            // a scratch really did win, and `TradeAggregatesV2::validate`
            // requires `losses == trades - wins`. Only the question "what is
            // the smallest win this rule may rest on" changes.
            // `min(worst)` over the wins -- the bracket test this replaces was
            // backwards and inflated the 3:1 rule. See `grid::tally_trade`. D-0602.
            if row.worst > 0 && (min_win == 0 || row.worst < min_win) {
                min_win = row.worst;
            }
            losing_streak = 0;
            winning_streak = winning_streak
                .checked_add(1)
                .ok_or(BaseEvidenceRefusalV2::Arithmetic("winning streak"))?;
            max_winning_streak = max_winning_streak.max(winning_streak);
        } else {
            gross_loss = checked_i64_add(gross_loss, row.worst, "gross loss")?;
            winning_streak = 0;
            losing_streak = losing_streak
                .checked_add(1)
                .ok_or(BaseEvidenceRefusalV2::Arithmetic("losing streak"))?;
            max_losing_streak = max_losing_streak.max(losing_streak);
        }
        worst_trade = worst_trade.min(row.worst);
        running = checked_i64_add(running, row.worst, "running return")?;
        peak = peak.max(running);
        max_drawdown = max_drawdown.max(
            peak.checked_sub(running)
                .ok_or(BaseEvidenceRefusalV2::Arithmetic("drawdown"))?,
        );
        let day = indicators::ist_day(row.entry_micros);
        if session_day == Some(day) {
            session_trades = session_trades
                .checked_add(1)
                .ok_or(BaseEvidenceRefusalV2::Arithmetic("session trade count"))?;
        } else {
            if session_day.is_some_and(|previous| day <= previous) {
                return Err(BaseEvidenceRefusalV2::Reconciliation(
                    "TradeRow IST sessions are not strictly increasing".to_owned(),
                ));
            }
            max_session_trades = max_session_trades.max(session_trades);
            session_day = Some(day);
            session_trades = 1;
        }
        for (accumulator, grain) in grains.iter_mut().zip(crate::stability::GRAINS) {
            accumulator.observe(grain, row.entry_micros, row.worst)?;
        }
    }
    max_session_trades = max_session_trades.max(session_trades);
    let trades = usize_u64("trade row count", rows.len())?;
    let losses = trades
        .checked_sub(wins)
        .ok_or(BaseEvidenceRefusalV2::Arithmetic("losing trade count"))?;
    let weakest_period_return_paisa = if rows.is_empty() {
        [0; GRAIN_COUNT]
    } else {
        let mut values = [0_i64; GRAIN_COUNT];
        for (value, grain) in values.iter_mut().zip(grains) {
            *value = grain.finish()?;
        }
        values
    };
    let aggregates = TradeAggregatesV2 {
        trades,
        wins,
        losses,
        pessimistic,
        optimistic,
        gross_win,
        gross_loss,
        best_trade,
        min_win,
        worst_trade,
        max_drawdown,
        ambiguous_bars: expected.ambiguous_bars,
        gapped: expected.gapped,
        max_losing_streak,
        max_winning_streak,
        max_mae_paisa,
        max_session_trades,
        weakest_period_return_paisa,
    };
    aggregates.validate()?;
    if (
        trades,
        wins,
        pessimistic,
        optimistic,
        gross_win,
        gross_loss,
        best_trade,
        min_win,
        worst_trade,
        max_drawdown,
        max_losing_streak,
        max_winning_streak,
    ) != (
        expected.trades,
        expected.wins,
        expected.pessimistic,
        expected.optimistic,
        expected.gross_win,
        expected.gross_loss,
        expected.best_trade,
        expected.min_win,
        expected.worst_trade,
        expected.max_drawdown,
        expected.max_losing_streak,
        expected.max_winning_streak,
    ) {
        return Err(BaseEvidenceRefusalV2::Reconciliation(
            "same-pass TradeRows do not reproduce the evaluated Cell".to_owned(),
        ));
    }
    let trade_rows_digest = digest.finalize();
    require_identity("TradeRows", trade_rows_digest)?;
    Ok((aggregates, trade_rows_digest))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct GrainAccumulatorV2 {
    key: Option<i64>,
    current: i64,
    weakest: Option<i64>,
}

impl GrainAccumulatorV2 {
    fn observe(
        &mut self,
        grain: crate::stability::Grain,
        entry_micros: i64,
        value: i64,
    ) -> Result<(), BaseEvidenceRefusalV2> {
        let key = grain.bucket(entry_micros);
        match self.key {
            None => {
                self.key = Some(key);
                self.current = value;
            }
            Some(previous) if key == previous => {
                self.current = checked_i64_add(self.current, value, "period return")?;
            }
            Some(previous) if key > previous => {
                self.weakest = Some(
                    self.weakest
                        .map_or(self.current, |old| old.min(self.current)),
                );
                self.key = Some(key);
                self.current = value;
            }
            Some(_) => {
                return Err(BaseEvidenceRefusalV2::Reconciliation(format!(
                    "{} TradeRow buckets are not monotone",
                    grain.as_str()
                )));
            }
        }
        Ok(())
    }

    fn finish(self) -> Result<i64, BaseEvidenceRefusalV2> {
        if self.key.is_none() {
            return Err(BaseEvidenceRefusalV2::Reconciliation(
                "non-empty TradeRows produced an empty grain".to_owned(),
            ));
        }
        Ok(self
            .weakest
            .map_or(self.current, |old| old.min(self.current)))
    }
}

fn base_policy_digest_v2() -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(POLICY_ID_DOMAIN);
    hasher.update(&VERSION.to_le_bytes());
    hasher.update(&PPM.to_le_bytes());
    hasher.update(b"same-pass-authoritative-trade-rows\0");
    hasher.update(b"one-support-scan-per-closed-mask\0");
    hasher.update(b"entry-ist-session-concentration\0");
    hasher.update(b"year-half-quarter-month-week-day-hour\0");
    hasher.update(b"data-origin-unmeasured-until-durable-store-authority\0");
    hasher.finalize()
}

fn derive_evidence_id(value: &BaseEvidenceRecordV2) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(EVIDENCE_ID_DOMAIN);
    let subject = value.draft.subject;
    for digest in [
        subject.candidate_universe_id,
        value.candidate_completion_digest,
        value.candidate_ordered_row_digest,
        subject.candidate_semantic_id,
        subject.candidate_row_digest,
        subject.execution_run_id,
        subject.evaluated_grid_digest,
        value.base_policy_digest,
        value.draft.trade_rows_digest,
    ] {
        hasher.update(&digest);
    }
    hasher.update(&subject.candidate_sequence.to_le_bytes());
    hasher.update(&subject.cell_ordinal.to_le_bytes());
    hasher.update(&value.draft.support_hits.to_le_bytes());
    hasher.update(&value.draft.independent_sessions.to_le_bytes());
    hasher.update(&[subject.family, subject.direction]);
    hasher.update(&subject.rung_seconds.to_le_bytes());
    hasher.update(&subject.horizon_bars.to_le_bytes());
    hash_aggregates(&mut hasher, value.draft.aggregates);
    hasher.finalize()
}

fn hash_aggregates(hasher: &mut Hasher, value: TradeAggregatesV2) {
    for number in [
        value.trades,
        value.wins,
        value.losses,
        value.ambiguous_bars,
        value.gapped,
        value.max_mae_paisa,
        value.max_session_trades,
    ] {
        hasher.update(&number.to_le_bytes());
    }
    for number in [
        value.pessimistic,
        value.optimistic,
        value.gross_win,
        value.gross_loss,
        value.best_trade,
        value.min_win,
        value.worst_trade,
        value.max_drawdown,
    ] {
        hasher.update(&number.to_le_bytes());
    }
    hasher.update(&value.max_losing_streak.to_le_bytes());
    hasher.update(&value.max_winning_streak.to_le_bytes());
    for number in value.weakest_period_return_paisa {
        hasher.update(&number.to_le_bytes());
    }
}

fn hash_trade_row(hasher: &mut Hasher, row: &TradeRow) -> Result<(), BaseEvidenceRefusalV2> {
    for index in [row.signal_bar, row.entry_bar, row.exit_bar] {
        hasher.update(&usize_u64("TradeRow index", index)?.to_le_bytes());
    }
    for value in [
        row.best,
        row.worst,
        row.entry_micros,
        row.exit_micros,
        row.adverse,
        row.adverse_paisa,
        row.favourable,
        row.favourable_paisa,
    ] {
        hasher.update(&value.to_le_bytes());
    }
    Ok(())
}

fn encode_aggregates(
    writer: &mut FixedWriter<'_>,
    value: TradeAggregatesV2,
) -> Result<(), BaseEvidenceRefusalV2> {
    for number in [
        value.trades,
        value.wins,
        value.losses,
        value.ambiguous_bars,
        value.gapped,
        value.max_mae_paisa,
        value.max_session_trades,
    ] {
        writer.u64(number)?;
    }
    for number in [
        value.pessimistic,
        value.optimistic,
        value.gross_win,
        value.gross_loss,
        value.best_trade,
        value.min_win,
        value.worst_trade,
        value.max_drawdown,
    ] {
        writer.i64(number)?;
    }
    writer.u32(value.max_losing_streak)?;
    writer.u32(value.max_winning_streak)?;
    for number in value.weakest_period_return_paisa {
        writer.i64(number)?;
    }
    Ok(())
}

fn decode_aggregates(
    reader: &mut FixedReader<'_>,
) -> Result<TradeAggregatesV2, BaseEvidenceRefusalV2> {
    Ok(TradeAggregatesV2 {
        trades: reader.u64()?,
        wins: reader.u64()?,
        losses: reader.u64()?,
        ambiguous_bars: reader.u64()?,
        gapped: reader.u64()?,
        max_mae_paisa: reader.u64()?,
        max_session_trades: reader.u64()?,
        pessimistic: reader.i64()?,
        optimistic: reader.i64()?,
        gross_win: reader.i64()?,
        gross_loss: reader.i64()?,
        best_trade: reader.i64()?,
        min_win: reader.i64()?,
        worst_trade: reader.i64()?,
        max_drawdown: reader.i64()?,
        max_losing_streak: reader.u32()?,
        max_winning_streak: reader.u32()?,
        weakest_period_return_paisa: {
            let mut values = [0_i64; GRAIN_COUNT];
            for value in &mut values {
                *value = reader.i64()?;
            }
            values
        },
    })
}

struct FixedWriter<'a> {
    bytes: &'a mut [u8],
    cursor: usize,
}

impl<'a> FixedWriter<'a> {
    fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), BaseEvidenceRefusalV2> {
        let end = self
            .cursor
            .checked_add(value.len())
            .ok_or(BaseEvidenceRefusalV2::Arithmetic("codec cursor"))?;
        let target = self.bytes.get_mut(self.cursor..end).ok_or_else(|| {
            BaseEvidenceRefusalV2::Codec("payload capacity was exceeded".to_owned())
        })?;
        target.copy_from_slice(value);
        self.cursor = end;
        Ok(())
    }

    fn digest(&mut self, value: [u8; 32]) -> Result<(), BaseEvidenceRefusalV2> {
        self.bytes(&value)
    }

    fn u8(&mut self, value: u8) -> Result<(), BaseEvidenceRefusalV2> {
        self.bytes(&[value])
    }

    fn u16(&mut self, value: u16) -> Result<(), BaseEvidenceRefusalV2> {
        self.bytes(&value.to_le_bytes())
    }

    fn u32(&mut self, value: u32) -> Result<(), BaseEvidenceRefusalV2> {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), BaseEvidenceRefusalV2> {
        self.bytes(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), BaseEvidenceRefusalV2> {
        self.bytes(&value.to_le_bytes())
    }

    fn require_remaining_zero(&self) {
        debug_assert!(
            self.bytes
                .get(self.cursor..)
                .is_some_and(|remaining| remaining.iter().all(|byte| *byte == 0))
        );
    }
}

struct FixedReader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> FixedReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], BaseEvidenceRefusalV2> {
        let end = self
            .cursor
            .checked_add(N)
            .ok_or(BaseEvidenceRefusalV2::Arithmetic("decode cursor"))?;
        let value = self.bytes.get(self.cursor..end).ok_or_else(|| {
            BaseEvidenceRefusalV2::Codec("record ended before its fields".to_owned())
        })?;
        self.cursor = end;
        value
            .try_into()
            .map_err(|_| BaseEvidenceRefusalV2::Codec("fixed field width changed".to_owned()))
    }

    fn digest(&mut self) -> Result<[u8; 32], BaseEvidenceRefusalV2> {
        self.array()
    }

    fn u8(&mut self) -> Result<u8, BaseEvidenceRefusalV2> {
        Ok(self.array::<1>()?[0])
    }

    fn u16(&mut self) -> Result<u16, BaseEvidenceRefusalV2> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    fn u32(&mut self) -> Result<u32, BaseEvidenceRefusalV2> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, BaseEvidenceRefusalV2> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn i64(&mut self) -> Result<i64, BaseEvidenceRefusalV2> {
        Ok(i64::from_le_bytes(self.array()?))
    }

    fn require_zero_u16(&mut self, name: &str) -> Result<(), BaseEvidenceRefusalV2> {
        if self.u16()? == 0 {
            Ok(())
        } else {
            Err(BaseEvidenceRefusalV2::Codec(format!("{name} is nonzero")))
        }
    }

    fn require_zero_u32(&mut self, name: &str) -> Result<(), BaseEvidenceRefusalV2> {
        if self.u32()? == 0 {
            Ok(())
        } else {
            Err(BaseEvidenceRefusalV2::Codec(format!("{name} is nonzero")))
        }
    }

    fn require_zeros(&mut self, name: &str) -> Result<(), BaseEvidenceRefusalV2> {
        if self
            .bytes
            .get(self.cursor..)
            .is_some_and(|remaining| remaining.iter().all(|byte| *byte == 0))
        {
            self.cursor = self.bytes.len();
            Ok(())
        } else {
            Err(BaseEvidenceRefusalV2::Codec(format!("{name} is nonzero")))
        }
    }
}

fn require_bound(
    resource: &'static str,
    actual: u64,
    maximum: u64,
) -> Result<(), BaseEvidenceRefusalV2> {
    if actual <= maximum {
        Ok(())
    } else {
        Err(BaseEvidenceRefusalV2::BoundExceeded {
            resource,
            actual,
            maximum,
        })
    }
}

fn require_identity(name: &'static str, value: [u8; 32]) -> Result<(), BaseEvidenceRefusalV2> {
    if value == [0; 32] {
        Err(BaseEvidenceRefusalV2::ZeroIdentity(name))
    } else {
        Ok(())
    }
}

fn checked_i64_add(
    left: i64,
    right: i64,
    name: &'static str,
) -> Result<i64, BaseEvidenceRefusalV2> {
    left.checked_add(right)
        .ok_or(BaseEvidenceRefusalV2::Arithmetic(name))
}

fn nonnegative_u64(name: &'static str, value: i64) -> Result<u64, BaseEvidenceRefusalV2> {
    u64::try_from(value).map_err(|_| BaseEvidenceRefusalV2::Arithmetic(name))
}

fn usize_u64(name: &'static str, value: usize) -> Result<u64, BaseEvidenceRefusalV2> {
    u64::try_from(value).map_err(|_| BaseEvidenceRefusalV2::Arithmetic(name))
}

fn digest_domain(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    hasher.update(bytes);
    hasher.finalize()
}

const fn negative_magnitude(value: i64) -> u64 {
    if value < 0 { value.unsigned_abs() } else { 0 }
}

const fn measured_when(measured: bool, value: u64) -> ObservedU64V1 {
    if measured {
        ObservedU64V1::Measured(value)
    } else {
        ObservedU64V1::Unmeasured
    }
}

pub(crate) fn measured_rate(
    part: u64,
    total: u64,
    name: &'static str,
) -> Result<ObservedU64V1, BaseEvidenceRefusalV2> {
    if total == 0 {
        Ok(ObservedU64V1::Unmeasured)
    } else {
        Ok(ObservedU64V1::Measured(rate_ppm(part, total, name)?))
    }
}

fn rate_ppm(part: u64, total: u64, name: &'static str) -> Result<u64, BaseEvidenceRefusalV2> {
    let denominator = u128::from(total);
    let scaled = u128::from(part)
        .checked_mul(u128::from(PPM))
        .ok_or(BaseEvidenceRefusalV2::Arithmetic(name))?;
    let quotient = scaled
        .checked_div(denominator)
        .ok_or(BaseEvidenceRefusalV2::Arithmetic(name))?;
    u64::try_from(quotient).map_err(|_| BaseEvidenceRefusalV2::Arithmetic(name))
}

pub(crate) fn ratio_observed(
    numerator: u64,
    denominator: u64,
    name: &'static str,
) -> Result<ObservedU64V1, BaseEvidenceRefusalV2> {
    if denominator == 0 {
        Ok(ObservedU64V1::Unmeasured)
    } else {
        Ok(ObservedU64V1::Measured(rate_ppm(
            numerator,
            denominator,
            name,
        )?))
    }
}

pub(crate) fn return_drawdown(
    profit: i64,
    drawdown: i64,
) -> Result<ObservedU64V1, BaseEvidenceRefusalV2> {
    if profit <= 0 {
        Ok(ObservedU64V1::Measured(0))
    } else if drawdown == 0 {
        Ok(ObservedU64V1::Measured(u64::MAX))
    } else {
        ratio_observed(
            profit.unsigned_abs(),
            drawdown.unsigned_abs(),
            "return/drawdown",
        )
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "test fixtures use expect to keep the asserted invariant visible"
)]
mod tests {
    use super::*;

    fn subject() -> CandidateSubjectV2 {
        CandidateSubjectV2 {
            candidate_universe_id: [1; 32],
            candidate_sequence: 0,
            candidate_semantic_id: [2; 32],
            candidate_row_digest: [3; 32],
            execution_run_id: [4; 32],
            evaluated_grid_digest: [5; 32],
            cell_ordinal: 7,
            family: 1,
            direction: 1,
            rung_seconds: 300,
            horizon_bars: 30,
        }
    }

    fn rows() -> Vec<TradeRow> {
        vec![
            TradeRow {
                signal_bar: 0,
                entry_bar: 1,
                exit_bar: 2,
                best: 130,
                worst: 100,
                entry_micros: 1_704_080_700_000_000,
                exit_micros: 1_704_080_760_000_000,
                adverse: 10,
                adverse_paisa: 25,
                favourable: 50,
                favourable_paisa: 125,
            },
            TradeRow {
                signal_bar: 3,
                entry_bar: 4,
                exit_bar: 5,
                best: -30,
                worst: -50,
                entry_micros: 1_704_167_100_000_000,
                exit_micros: 1_704_167_160_000_000,
                adverse: 20,
                adverse_paisa: 50,
                favourable: 5,
                favourable_paisa: 12,
            },
        ]
    }

    fn cell() -> Cell {
        Cell {
            trades: 2,
            wins: 1,
            pessimistic: 50,
            optimistic: 100,
            gross_win: 100,
            gross_loss: -50,
            best_trade: 100,
            min_win: 100,
            worst_trade: -50,
            max_drawdown: 50,
            max_winning_streak: 1,
            max_losing_streak: 1,
            ambiguous_bars: 1,
            gapped: 0,
            stopped: 1,
            timed_out: 1,
            ..Cell::default()
        }
    }

    fn bounds() -> BaseEvidenceBoundsV2 {
        BaseEvidenceBoundsV2::new(10, 10, 100, 100).expect("explicit bounds")
    }

    fn family_source() -> BaseEvidenceFamilySourceV2 {
        BaseEvidenceFamilySourceV2 {
            universe_id: [1; 32],
            completion_digest: [6; 32],
            ordered_row_digest: [7; 32],
            row_count: 1,
        }
    }

    fn record() -> BaseEvidenceRecordV2 {
        let draft = BaseEvidenceDraftV2::measure(
            subject(),
            MaskSupportEvidenceV2 {
                mask_words: [1, 0, 0, 0, 0, 0],
                support_hits: 9,
                independent_sessions: 3,
            },
            &cell(),
            &rows(),
            bounds(),
        )
        .expect("same-pass evidence");
        BaseEvidenceRecordV2::seal(family_source(), &draft).expect("sealed record")
    }

    fn empty_record() -> BaseEvidenceRecordV2 {
        let draft = BaseEvidenceDraftV2::measure(
            subject(),
            MaskSupportEvidenceV2 {
                mask_words: [1, 0, 0, 0, 0, 0],
                support_hits: 9,
                independent_sessions: 3,
            },
            &Cell::default(),
            &[],
            bounds(),
        )
        .expect("empty same-pass evidence");
        BaseEvidenceRecordV2::seal(family_source(), &draft).expect("sealed empty record")
    }

    #[test]
    fn same_pass_rows_fill_exactly_the_27_base_fields() {
        let values = record().admission_values().expect("finite base projection");
        assert_eq!(values.support_hits, ObservedU64V1::Measured(9));
        assert_eq!(values.independent_sessions, ObservedU64V1::Measured(3));
        assert_eq!(values.trades, ObservedU64V1::Measured(2));
        assert_eq!(values.winning_trades, ObservedU64V1::Measured(1));
        assert_eq!(values.losing_trades, ObservedU64V1::Measured(1));
        assert_eq!(values.max_mae_paisa, ObservedU64V1::Measured(50));
        assert_eq!(
            values.worst_reward_risk_ppm,
            ObservedU64V1::Measured(2_000_000)
        );
        assert_eq!(values.win_rate_ppm, ObservedU64V1::Measured(500_000));
        assert_eq!(
            values.return_drawdown_ppm,
            ObservedU64V1::Measured(1_000_000)
        );
        assert_eq!(
            values.weakest_period_return_paisa,
            ObservedI64V1::Measured(-50)
        );
        assert_eq!(
            values.ambiguous_fill_rate_ppm,
            ObservedU64V1::Measured(500_000)
        );
        assert_eq!(values.gap_affected_rate_ppm, ObservedU64V1::Measured(0));
        assert_eq!(
            values.session_concentration_ppm,
            ObservedU64V1::Measured(500_000)
        );
        assert_eq!(
            values.largest_trade_profit_share_ppm,
            ObservedU64V1::Measured(1_000_000)
        );
        assert_eq!(values.profit_factor_ppm, ObservedU64V1::Measured(2_000_000));
        assert_eq!(values.execution_complete, CompletenessV1::Complete);
        assert_eq!(values.data_complete, CompletenessV1::Unmeasured);
        assert_eq!(values.calendar_complete, CompletenessV1::Complete);
        assert_eq!(values.population_complete, CompletenessV1::Complete);
        assert_eq!(values.drawdown_paisa, ObservedU64V1::Measured(50));
        assert_eq!(values.worst_trade_loss_paisa, ObservedU64V1::Measured(50));
        assert_eq!(
            values.losing_trade_rate_ppm,
            ObservedU64V1::Measured(500_000)
        );
        assert_eq!(values.pessimistic_profit_paisa, ObservedI64V1::Measured(50));
        assert_eq!(values.average_win_paisa, ObservedU64V1::Measured(100));
        assert_eq!(values.average_loss_paisa, ObservedU64V1::Measured(50));
        assert_eq!(values.consecutive_losing_streak, ObservedU64V1::Measured(1));
        assert_eq!(
            values.consecutive_winning_streak,
            ObservedU64V1::Measured(1)
        );

        assert_eq!(values.wilson_win_rate_ppm, ObservedU64V1::Unmeasured);
        assert_eq!(values.pbo_ppm, ObservedU64V1::Unmeasured);
        assert_eq!(values.fwer_p_value_ppm, ObservedU64V1::Unmeasured);
        assert_eq!(values.spa_p_value_ppm, ObservedU64V1::Unmeasured);
        assert_eq!(values.decided_folds, ObservedU64V1::Unmeasured);
        assert_eq!(values.bootstrap_draws, ObservedU64V1::Unmeasured);
        assert_eq!(values.bootstrap_strategies, ObservedU64V1::Unmeasured);
        assert_eq!(values.bootstrap_periods, ObservedU64V1::Unmeasured);
        assert_eq!(values.pbo_contributing_folds, ObservedU64V1::Unmeasured);
        assert_eq!(values.pbo_unrankable_folds, ObservedU64V1::Unmeasured);
        assert_eq!(values.profitable_oos_folds, ObservedU64V1::Unmeasured);
        assert_eq!(
            values.oos_pessimistic_return_paisa,
            ObservedI64V1::Unmeasured
        );
        assert_eq!(values.white_reality_p_value_ppm, ObservedU64V1::Unmeasured);
        assert_eq!(values.romano_wolf_p_value_ppm, ObservedU64V1::Unmeasured);
        assert_eq!(
            values.white_reality_decision,
            HypothesisDecisionV1::Unmeasured
        );
        assert_eq!(
            values.romano_wolf_decision,
            HypothesisDecisionV1::Unmeasured
        );
        assert_eq!(
            values.full_precision_statistics_complete,
            CompletenessV1::Unmeasured
        );
        AdmissionEvidenceV1::new(values).expect("canonical base projection");
    }

    #[test]
    fn finite_ratio_overflow_refuses_and_zero_drawdown_keeps_its_explicit_sentinel() {
        assert!(matches!(
            rate_ppm(u64::MAX, 1, "finite overflow"),
            Err(BaseEvidenceRefusalV2::Arithmetic("finite overflow"))
        ));
        assert_eq!(
            return_drawdown(1, 0).expect("zero drawdown has a versioned sentinel"),
            ObservedU64V1::Measured(u64::MAX)
        );

        let overflow_rows = [
            TradeRow {
                signal_bar: 0,
                entry_bar: 1,
                exit_bar: 2,
                best: i64::MAX,
                worst: i64::MAX,
                entry_micros: 1_704_080_700_000_000,
                exit_micros: 1_704_080_760_000_000,
                adverse: 0,
                adverse_paisa: 0,
                favourable: 0,
                favourable_paisa: 0,
            },
            TradeRow {
                signal_bar: 3,
                entry_bar: 4,
                exit_bar: 5,
                best: -1,
                worst: -1,
                entry_micros: 1_704_167_100_000_000,
                exit_micros: 1_704_167_160_000_000,
                adverse: 0,
                adverse_paisa: 0,
                favourable: 0,
                favourable_paisa: 0,
            },
        ];
        let overflow_cell = Cell {
            trades: 2,
            wins: 1,
            pessimistic: i64::MAX - 1,
            optimistic: i64::MAX - 1,
            gross_win: i64::MAX,
            gross_loss: -1,
            best_trade: i64::MAX,
            min_win: i64::MAX,
            worst_trade: -1,
            max_drawdown: 1,
            max_winning_streak: 1,
            max_losing_streak: 1,
            timed_out: 2,
            ..Cell::default()
        };
        let finite = BaseEvidenceDraftV2::measure(
            subject(),
            MaskSupportEvidenceV2 {
                mask_words: [1, 0, 0, 0, 0, 0],
                support_hits: 9,
                independent_sessions: 3,
            },
            &overflow_cell,
            &overflow_rows,
            bounds(),
        )
        .expect("two finite rows reconcile without sum overflow");
        assert!(matches!(
            BaseEvidenceRecordV2::seal(family_source(), &finite),
            Err(BaseEvidenceRefusalV2::Arithmetic("worst reward/risk"))
        ));
    }

    #[test]
    fn empty_trade_rows_preserve_the_complete_measured_unmeasured_matrix() {
        let values = empty_record()
            .admission_values()
            .expect("empty projection has no finite overflow");
        assert_eq!(values.support_hits, ObservedU64V1::Measured(9));
        assert_eq!(values.independent_sessions, ObservedU64V1::Measured(3));
        assert_eq!(values.trades, ObservedU64V1::Measured(0));
        for value in [
            values.max_mae_paisa,
            values.worst_reward_risk_ppm,
            values.win_rate_ppm,
            values.return_drawdown_ppm,
            values.ambiguous_fill_rate_ppm,
            values.gap_affected_rate_ppm,
            values.session_concentration_ppm,
            values.largest_trade_profit_share_ppm,
            values.losing_trade_rate_ppm,
            values.average_win_paisa,
            values.average_loss_paisa,
            values.profit_factor_ppm,
            values.consecutive_losing_streak,
            values.consecutive_winning_streak,
        ] {
            assert_eq!(value, ObservedU64V1::Unmeasured);
        }
        assert_eq!(
            values.weakest_period_return_paisa,
            ObservedI64V1::Unmeasured
        );
        assert_eq!(values.execution_complete, CompletenessV1::Complete);
        assert_eq!(values.data_complete, CompletenessV1::Unmeasured);
        assert_eq!(values.calendar_complete, CompletenessV1::Complete);
        assert_eq!(values.population_complete, CompletenessV1::Complete);
        assert_eq!(values.drawdown_paisa, ObservedU64V1::Measured(0));
        assert_eq!(values.worst_trade_loss_paisa, ObservedU64V1::Measured(0));
        assert_eq!(values.losing_trades, ObservedU64V1::Measured(0));
        assert_eq!(values.pessimistic_profit_paisa, ObservedI64V1::Measured(0));
        assert_eq!(values.winning_trades, ObservedU64V1::Measured(0));

        for value in [
            values.wilson_win_rate_ppm,
            values.pbo_ppm,
            values.fwer_p_value_ppm,
            values.spa_p_value_ppm,
            values.decided_folds,
            values.bootstrap_draws,
            values.bootstrap_strategies,
            values.bootstrap_periods,
            values.pbo_contributing_folds,
            values.pbo_unrankable_folds,
            values.profitable_oos_folds,
            values.white_reality_p_value_ppm,
            values.romano_wolf_p_value_ppm,
        ] {
            assert_eq!(value, ObservedU64V1::Unmeasured);
        }
        assert_eq!(
            values.oos_pessimistic_return_paisa,
            ObservedI64V1::Unmeasured
        );
        assert_eq!(
            values.white_reality_decision,
            HypothesisDecisionV1::Unmeasured
        );
        assert_eq!(
            values.romano_wolf_decision,
            HypothesisDecisionV1::Unmeasured
        );
        assert_eq!(
            values.full_precision_statistics_complete,
            CompletenessV1::Unmeasured
        );
    }

    #[test]
    fn fixed_codec_round_trips_and_rejects_corruption_and_stale_identity() {
        let value = record();
        let bytes = value.encode().expect("encode");
        assert_eq!(BaseEvidenceRecordV2::decode(&bytes).expect("decode"), value);

        let mut corrupt = bytes;
        corrupt[420] ^= 1;
        assert!(matches!(
            BaseEvidenceRecordV2::decode(&corrupt),
            Err(BaseEvidenceRefusalV2::Codec(_))
        ));

        let mut stale = value;
        stale.draft.aggregates.max_mae_paisa += 1;
        assert!(matches!(
            stale.encode(),
            Err(BaseEvidenceRefusalV2::Reconciliation(_))
        ));
    }

    #[test]
    fn codec_rejects_nonzero_reserved_bytes_even_under_a_matching_record_seal() {
        for (offset, name) in [
            (20_usize, "header reserve"),
            (378_usize, "subject reserve"),
            (700_usize, "record tail reserve"),
        ] {
            let mut bytes = record().encode().expect("encode");
            *bytes.get_mut(offset).expect("reserved offset exists") = 1;
            let payload = bytes
                .get(..PAYLOAD_BYTES)
                .expect("fixed payload boundary exists");
            let seal = digest_domain(RECORD_SEAL_DOMAIN, payload);
            bytes
                .get_mut(PAYLOAD_BYTES..)
                .expect("fixed seal boundary exists")
                .copy_from_slice(&seal);
            let why = BaseEvidenceRecordV2::decode(&bytes)
                .expect_err("reserved bytes are canonical zeroes, not extension space");
            assert_eq!(
                why,
                BaseEvidenceRefusalV2::Codec(format!("{name} is nonzero"))
            );
        }
    }

    #[test]
    fn explicit_boundaries_refuse_before_capture() {
        assert!(matches!(
            BaseEvidenceBoundsV2::new(0, 1, 1, 1),
            Err(BaseEvidenceRefusalV2::InvalidBound("candidate"))
        ));
        assert!(matches!(
            BaseEvidenceBoundsV2::new(1, 1, 1, 2),
            Err(BaseEvidenceRefusalV2::InvalidBound(_))
        ));
        let why = BaseEvidenceDraftV2::measure(
            subject(),
            MaskSupportEvidenceV2 {
                mask_words: [1, 0, 0, 0, 0, 0],
                support_hits: 9,
                independent_sessions: 3,
            },
            &cell(),
            &rows(),
            BaseEvidenceBoundsV2::new(1, 1, 10, 10).expect("tight bound"),
        )
        .expect_err("two rows exceed one");
        assert!(matches!(why, BaseEvidenceRefusalV2::BoundExceeded { .. }));
    }

    #[test]
    fn overlapping_or_non_reconciling_trade_rows_are_refused() {
        let mut overlapping = rows();
        let first_exit = overlapping.first().expect("first trade").exit_micros;
        overlapping.get_mut(1).expect("second trade").entry_micros = first_exit;
        assert!(matches!(
            BaseEvidenceDraftV2::measure(
                subject(),
                MaskSupportEvidenceV2 {
                    mask_words: [1, 0, 0, 0, 0, 0],
                    support_hits: 9,
                    independent_sessions: 3,
                },
                &cell(),
                &overlapping,
                bounds(),
            ),
            Err(BaseEvidenceRefusalV2::Reconciliation(_))
        ));

        let mut wrong_cell = cell();
        wrong_cell.pessimistic += 1;
        assert!(matches!(
            BaseEvidenceDraftV2::measure(
                subject(),
                MaskSupportEvidenceV2 {
                    mask_words: [1, 0, 0, 0, 0, 0],
                    support_hits: 9,
                    independent_sessions: 3,
                },
                &wrong_cell,
                &rows(),
                bounds(),
            ),
            Err(BaseEvidenceRefusalV2::Reconciliation(_))
        ));
    }
}
