//! Fail-closed institutional evidence for one Population V4 cell.
//!
//! [`crate::population_admission_writer::PopulationCellContextV1`] proves that
//! one cell belongs to one opaque, complete exit-grid evaluation.  It does not
//! by itself prove every statistic required by
//! [`runner::admission::AdmissionEvidenceV1`].  This module is the explicit
//! bridge: it derives only values whose source is present, reconciles every
//! relationship it can check, and keeps absent/refused sources distinct.
//!
//! # What is measured here
//!
//! * support comes from the uncapped population member;
//! * trade totals, risk, streaks and exit ambiguity come from the evaluated
//!   [`runner::grid::Cell`];
//! * exact maximum MAE in paisa, period weakness and concentration come from
//!   the exact chosen-cell [`runner::grid::TradeRow`] stream;
//! * walk-forward outcome aggregates come from one identity-bound
//!   [`runner::validate::Validated`]; a supplied [`runner::pbo::Pbo`] is checked
//!   only as a legacy anchored-fold diagnostic and never becomes genuine
//!   CSCV/PBO admission evidence; and
//! * White, SPA and legacy Romano--Wolf decisions come from one reconciled
//!   in-memory family-test result; and
//! * exact selected-candidate and full-family Romano--Wolf probabilities come
//!   only from a reopened receipt-last
//!   [`crate::institutional_statistics::InstitutionalStatisticsAuthorityV1`].
//!
//! # What is deliberately not manufactured
//!
//! The new durable family authority covers White/SPA source bits and exact
//! selected/full-family Romano--Wolf counts. It does **not** contain raw Wilson,
//! genuine CSCV/PBO, risk/ratio or every other statistical source required by
//! the global completeness flag. Those broader fields therefore remain
//! `Unmeasured` until a separate append-only all-source record exists. In
//! particular this module
//! still has no constructor capable of claiming
//! `full_precision_statistics_complete == Complete`.
//!
//! # Cost
//!
//! One complete exit grid is validated once in O(G), then each cell projection
//! and ordinal check is O(1) through the retained opaque capability. Trade
//! reconciliation, session concentration and stability are O(trades);
//! support-session measurement is O(signal bars), and validation reconciliation
//! is O(all fold candidate rows + F log F) for F folds. This is a once-per-result
//! boundary, not a per-bar or per-candidate inner-loop primitive.  Producing
//! exact TradeRows from the present grid is a replay over execution bars; doing
//! that for every Population V4 cell would repeat the pricing path and is not a
//! constant-cost population operation.  Until a bound detail receipt exists,
//! callers must leave those fields `Unmeasured` rather than hide that cost.
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

use indicators::column::{Column, Sourced};
use indicators::{Candle, IST_OFFSET_MICROS};
use runner::admission::{
    AdmissionEvidenceV1, AdmissionEvidenceValuesV1, CompletenessV1, HypothesisDecisionV1,
    ObservedI64V1, ObservedU64V1, PPM,
};
use runner::bootstrap::{RomanoWolfReceipt, Verdict};
use runner::grid::{Cell, TradeRow};
use runner::pbo::{Pbo, Placement, place, probability_of_overfitting};
use runner::validate::Validated;

use crate::institutional_statistics::InstitutionalStatisticsAuthorityV1;
use crate::population_admission_writer::{
    CompletePopulationAuthorityV1, PopulationCellContextV1, derive_population_id_v1,
    derive_strategy_digest_from_validated_v1,
};
use crate::stored_data_completeness::StoredDataCompletenessAuthorityV1;

const CANONICAL_FWER_ALPHA_PPM: u64 = 50_000;

/// A typed source yielded a value, did not measure one, or refused its input.
///
/// There is no default and no numeric fallback.  The two non-value states map
/// one-for-one onto the canonical admission observation tags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceSourceV1<T> {
    /// Source measured a value.
    Measured(T),
    /// Source ran without producing this measurement, or is not implemented.
    Unmeasured,
    /// Source explicitly refused its input/result.
    Refused,
}

/// Current stored-data completeness capability.
///
/// `Complete` is not a flag: it requires a durable, sealed authority that
/// recomputed the exact signal, full one-minute context and daily-reference
/// data identity and bound it to the same Population V4 authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataCompletenessSourceV1<'a> {
    /// Exact stored streams were durably reconciled with Population V4.
    Complete(&'a StoredDataCompletenessAuthorityV1),
    /// No durable stored-data reconciliation authority was supplied.
    Unmeasured,
    /// An upstream stored-data authority explicitly refused the input/result.
    Refused,
}

/// Typed completeness sources owned outside one evaluated cell.
///
/// Calendar and population completeness can become `Complete` only through a
/// [`CompletePopulationAuthorityV1`] whose private calendar receipts derive the
/// same population identity and whose exit-grid cardinalities reconcile with
/// the completed [`PopulationCellContextV1`].  No loose `Complete` flag enters
/// this builder.
#[derive(Clone, Copy, Debug)]
pub struct InstitutionalCompletenessV1<'a> {
    /// Pre-run authority containing both opaque complete calendar receipts.
    pub population_authority: EvidenceSourceV1<&'a CompletePopulationAuthorityV1<'a>>,
    /// Stored input-data reconciliation state.
    pub data: DataCompletenessSourceV1<'a>,
}

/// Current full-precision source capability.
///
/// No `Complete` variant exists because the durable family-statistics authority
/// covers only White/SPA/Romano--Wolf. It does not bind the raw Wilson, genuine
/// CSCV/PBO, risk/ratio and remaining statistical sources this global field
/// promises.
/// Accepting a boolean here would let a caller assert a broader claim than the
/// supplied bytes prove.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FullPrecisionStatisticsSourceV1 {
    /// Required raw values/denominators were not durably recorded.
    Unmeasured,
    /// An upstream raw-statistics authority explicitly refused the result.
    Refused,
}

/// How one canonical admission field is sourced today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceAvailabilityV1 {
    /// Opaque Population V4 or evaluated-cell state measures it directly.
    MeasuredDirect,
    /// Exact chosen-cell replay measures it; no cheaper cell aggregate exists.
    MeasuredByExactReplay,
    /// Exact signal bars plus the signal column measure it.
    MeasuredBySignalScan,
    /// The complete walk-forward arrays measure it in memory.
    MeasuredByValidation,
    /// White/SPA family verdicts measure it in memory.
    MeasuredByFamilyTest,
    /// Only one outcome is provable from the current source type.
    PartiallyMeasured,
    /// No current source or typed receipt can supply it honestly.
    Absent,
}

/// One row of the complete `AdmissionEvidenceValuesV1` source/blocker map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvidenceSourceMapRowV1 {
    /// Exact canonical field name.
    pub field: &'static str,
    /// Current measurement availability.
    pub availability: EvidenceAvailabilityV1,
    /// Exact Rust artifact or missing receipt.
    pub source: &'static str,
}

/// Exhaustive current source map for every `AdmissionEvidenceValuesV1` field.
///
/// `Absent` and `PartiallyMeasured` rows are admission blockers, not TODO values
/// to be replaced by zero.  The table is executable Rust data so a future
/// wiring layer can render the same matrix in CLI/API/dashboard surfaces.
pub const ADMISSION_EVIDENCE_SOURCE_MATRIX_V1: [EvidenceSourceMapRowV1; 44] = [
    EvidenceSourceMapRowV1 {
        field: "support_hits",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "PopulationCellContextV1.support_hits",
    },
    EvidenceSourceMapRowV1 {
        field: "independent_sessions",
        availability: EvidenceAvailabilityV1::MeasuredBySignalScan,
        source: "PopulationRun.column + exact signal-bar IST timestamps",
    },
    EvidenceSourceMapRowV1 {
        field: "trades",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "Cell.trades",
    },
    EvidenceSourceMapRowV1 {
        field: "max_mae_paisa",
        availability: EvidenceAvailabilityV1::MeasuredByExactReplay,
        source: "max TradeRow.adverse_paisa",
    },
    EvidenceSourceMapRowV1 {
        field: "worst_reward_risk_ppm",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "Cell.min_win / |Cell.worst_trade|",
    },
    EvidenceSourceMapRowV1 {
        field: "win_rate_ppm",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "Cell.wins / Cell.trades",
    },
    EvidenceSourceMapRowV1 {
        field: "wilson_win_rate_ppm",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "Cell wins/trades; reconciled with Cell.assurance_bp()",
    },
    EvidenceSourceMapRowV1 {
        field: "return_drawdown_ppm",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "Cell.pessimistic / Cell.max_drawdown",
    },
    EvidenceSourceMapRowV1 {
        field: "weakest_period_return_paisa",
        availability: EvidenceAvailabilityV1::MeasuredByExactReplay,
        source: "minimum stability::worst_period over all seven GRAINS",
    },
    EvidenceSourceMapRowV1 {
        field: "pbo_ppm",
        availability: EvidenceAvailabilityV1::Absent,
        source: "runner::pbo::Pbo is an anchored walk-forward bottom-half diagnostic, not a genuine CSCV/PBO authority",
    },
    EvidenceSourceMapRowV1 {
        field: "fwer_p_value_ppm",
        availability: EvidenceAvailabilityV1::Absent,
        source: "durable exact full-family evidence is post-selection; pre-selection admission cannot consume it without a selection/admission authority cycle",
    },
    EvidenceSourceMapRowV1 {
        field: "spa_p_value_ppm",
        availability: EvidenceAvailabilityV1::MeasuredByFamilyTest,
        source: "bootstrap::Verdict returned by spa",
    },
    EvidenceSourceMapRowV1 {
        field: "decided_folds",
        availability: EvidenceAvailabilityV1::MeasuredByValidation,
        source: "Validated folds with chosen candidate",
    },
    EvidenceSourceMapRowV1 {
        field: "ambiguous_fill_rate_ppm",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "Cell.ambiguous_bars / Cell.trades",
    },
    EvidenceSourceMapRowV1 {
        field: "gap_affected_rate_ppm",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "Cell.gapped / Cell.trades",
    },
    EvidenceSourceMapRowV1 {
        field: "session_concentration_ppm",
        availability: EvidenceAvailabilityV1::MeasuredByExactReplay,
        source: "largest IST entry-day TradeRow count / all TradeRows",
    },
    EvidenceSourceMapRowV1 {
        field: "largest_trade_profit_share_ppm",
        availability: EvidenceAvailabilityV1::MeasuredByExactReplay,
        source: "largest positive TradeRow.worst / reconciled gross win",
    },
    EvidenceSourceMapRowV1 {
        field: "execution_complete",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "complete population authority + ResolvedExitGridV1 evaluation capability",
    },
    EvidenceSourceMapRowV1 {
        field: "data_complete",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "StoredDataCompletenessAuthorityV1 sealed signal+minute-context+daily receipt",
    },
    EvidenceSourceMapRowV1 {
        field: "calendar_complete",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "CompletePopulationAuthorityV1 private complete calendar receipts",
    },
    EvidenceSourceMapRowV1 {
        field: "population_complete",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "PopulationRun + CompletionReconciliationV2 + complete authority",
    },
    EvidenceSourceMapRowV1 {
        field: "drawdown_paisa",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "Cell.max_drawdown",
    },
    EvidenceSourceMapRowV1 {
        field: "worst_trade_loss_paisa",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "negative magnitude of Cell.worst_trade",
    },
    EvidenceSourceMapRowV1 {
        field: "losing_trade_rate_ppm",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "(Cell.trades - Cell.wins) / Cell.trades",
    },
    EvidenceSourceMapRowV1 {
        field: "losing_trades",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "Cell.trades - Cell.wins",
    },
    EvidenceSourceMapRowV1 {
        field: "pessimistic_profit_paisa",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "Cell.pessimistic",
    },
    EvidenceSourceMapRowV1 {
        field: "winning_trades",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "Cell.wins",
    },
    EvidenceSourceMapRowV1 {
        field: "average_win_paisa",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "Cell.avg_win() with explicit no-winner Unmeasured state",
    },
    EvidenceSourceMapRowV1 {
        field: "average_loss_paisa",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "|Cell.avg_loss()| with explicit no-loser Unmeasured state",
    },
    EvidenceSourceMapRowV1 {
        field: "profit_factor_ppm",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "Cell.gross_win / |Cell.gross_loss|",
    },
    EvidenceSourceMapRowV1 {
        field: "consecutive_losing_streak",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "Cell.max_losing_streak",
    },
    EvidenceSourceMapRowV1 {
        field: "consecutive_winning_streak",
        availability: EvidenceAvailabilityV1::MeasuredDirect,
        source: "Cell.max_winning_streak",
    },
    EvidenceSourceMapRowV1 {
        field: "bootstrap_draws",
        availability: EvidenceAvailabilityV1::MeasuredByFamilyTest,
        source: "White/SPA verdicts reconciled with RomanoWolfReceipt.draws",
    },
    EvidenceSourceMapRowV1 {
        field: "bootstrap_strategies",
        availability: EvidenceAvailabilityV1::MeasuredByFamilyTest,
        source: "White/SPA verdicts reconciled with RomanoWolfReceipt.strategies",
    },
    EvidenceSourceMapRowV1 {
        field: "bootstrap_periods",
        availability: EvidenceAvailabilityV1::MeasuredByFamilyTest,
        source: "White/SPA verdicts reconciled with RomanoWolfReceipt.periods",
    },
    EvidenceSourceMapRowV1 {
        field: "pbo_contributing_folds",
        availability: EvidenceAvailabilityV1::Absent,
        source: "legacy anchored-fold placement counts cannot become genuine CSCV/PBO contributing-split evidence",
    },
    EvidenceSourceMapRowV1 {
        field: "pbo_unrankable_folds",
        availability: EvidenceAvailabilityV1::Absent,
        source: "legacy anchored-fold unrankable counts cannot become genuine CSCV/PBO unrankable-split evidence",
    },
    EvidenceSourceMapRowV1 {
        field: "profitable_oos_folds",
        availability: EvidenceAvailabilityV1::MeasuredByValidation,
        source: "positive chosen FoldResult.out_of_sample_exit values",
    },
    EvidenceSourceMapRowV1 {
        field: "oos_pessimistic_return_paisa",
        availability: EvidenceAvailabilityV1::MeasuredByValidation,
        source: "checked sum of every decided FoldResult.out_of_sample_exit",
    },
    EvidenceSourceMapRowV1 {
        field: "white_reality_p_value_ppm",
        availability: EvidenceAvailabilityV1::MeasuredByFamilyTest,
        source: "bootstrap::Verdict returned by reality_check",
    },
    EvidenceSourceMapRowV1 {
        field: "romano_wolf_p_value_ppm",
        availability: EvidenceAvailabilityV1::Absent,
        source: "durable exact candidate evidence is post-selection; pre-selection admission cannot consume it without a selection/admission authority cycle",
    },
    EvidenceSourceMapRowV1 {
        field: "white_reality_decision",
        availability: EvidenceAvailabilityV1::MeasuredByFamilyTest,
        source: "finite White p-value, nonzero draws and canonical 5% rule",
    },
    EvidenceSourceMapRowV1 {
        field: "romano_wolf_decision",
        availability: EvidenceAvailabilityV1::MeasuredByFamilyTest,
        source: "complete RomanoWolfReceipt membership at canonical 5% FWER",
    },
    EvidenceSourceMapRowV1 {
        field: "full_precision_statistics_complete",
        availability: EvidenceAvailabilityV1::Absent,
        source: "family authority exists, but no durable all-source Wilson+PBO+risk/ratio statistics record exists",
    },
];

/// One structural blocker outside the scalar evidence-field map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstitutionalEvidenceBlockerV1 {
    /// Affected evidence surface.
    pub surface: &'static str,
    /// Exact current limitation.
    pub current_gap: &'static str,
    /// Minimum artifact needed to close it without inference.
    pub required_artifact: &'static str,
}

/// Structural blockers that prevent a truthful fully-admitted Population V4.
pub const INSTITUTIONAL_EVIDENCE_BLOCKERS_V1: [InstitutionalEvidenceBlockerV1; 7] = [
    InstitutionalEvidenceBlockerV1 {
        surface: "full-precision statistics",
        current_gap: "admission stores ppm projections only; raw Wilson/bootstrap/PBO values and exact denominators are not durably recorded",
        required_artifact: "append-only source-statistics record with a canonical digest referenced by admission",
    },
    InstitutionalEvidenceBlockerV1 {
        surface: "pre-selection adjusted family statistics",
        current_gap: "D-0461 binds exact family statistics to a receipt-last selection, but admission is required before that selection exists",
        required_artifact: "an acyclic pre-selection complete-family authority that does not depend on an admitted selection receipt",
    },
    InstitutionalEvidenceBlockerV1 {
        surface: "genuine CSCV/PBO",
        current_gap: "runner::pbo::Pbo classifies anchored walk-forward winner placements; it has no complete even-segment CSCV split family with complementary in/out partitions",
        required_artifact: "a receipt-last CSCV/PBO authority binding the complete candidate family, segment policy, every complementary split, exact ranks and final probability",
    },
    InstitutionalEvidenceBlockerV1 {
        surface: "complete bootstrap family",
        current_gap: "current CLI family is a bounded mask subset, omits exit cells and discards Romano-Wolf membership",
        required_artifact: "uncapped population member+exit-cell family receipt with positional strategy digests",
    },
    InstitutionalEvidenceBlockerV1 {
        surface: "walk-forward provenance and shapes",
        current_gap: "Validated carries no population/ranking identity and admission has no separate anchored/rolling coverage receipt",
        required_artifact: "identity-bound complete validation receipt covering required shapes, folds, candidates and split policy",
    },
    InstitutionalEvidenceBlockerV1 {
        surface: "independent support sessions",
        current_gap: "PopulationRun retains the signal column but CompletePopulationAuthorityV1 does not retain the exact signal-bar slice/receipt",
        required_artifact: "signal-bar digest capability paired to PopulationRun.column and population_id",
    },
    InstitutionalEvidenceBlockerV1 {
        surface: "exact trade detail at population scale",
        current_gap: "Cell has no exact max-MAE-paisa/period/concentration rows; replaying every cell repeats an O(execution bars) path",
        required_artifact: "evaluated-grid detail receipt or incremental per-cell aggregates emitted during the authoritative evaluation",
    },
];

/// Identity-bound values derived by exact replay of one chosen exit cell.
///
/// Fields are private so a caller cannot attach aggregate-compatible rows from
/// another strategy.  [`Self::from_exact_replay`] is the only constructor and
/// replays the cell against the typed Population V4 authority before deriving
/// MAE, stability and concentration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TradeEvidenceV1 {
    population_id: [u8; 32],
    strategy_digest: [u8; 32],
    values: ReconciledTradeEvidenceV1,
}

/// Identity-bound count of distinct IST signal sessions supporting one mask.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndependentSessionsEvidenceV1 {
    population_id: [u8; 32],
    strategy_digest: [u8; 32],
    sessions: u64,
}

/// Identity-bound anchored walk-forward projections.
///
/// The three canonically named PBO admission fields stay `Unmeasured`.  The
/// legacy anchored-fold diagnostic may still be reconciled when supplied, but
/// it is not a genuine CSCV/PBO authority and therefore cannot authorize those
/// fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidationEvidenceV1 {
    population_id: [u8; 32],
    ranking_policy_digest: [u8; 32],
    decided_folds: ObservedU64V1,
    profitable_oos_folds: ObservedU64V1,
    oos_pessimistic_return_paisa: ObservedI64V1,
}

impl ValidationEvidenceV1 {
    /// Reconciles one completed walk-forward and an optional legacy diagnostic.
    ///
    /// # Errors
    ///
    /// Refuses absent identities, a refused/partial fold, candidate truncation,
    /// a legacy diagnostic that differs from the fold set, or integer overflow.
    /// An empty fold set returns [`EvidenceSourceV1::Unmeasured`] rather than a
    /// measured zero.  A walk carrying an upstream refusal returns
    /// [`EvidenceSourceV1::Refused`].
    #[expect(
        clippy::too_many_lines,
        reason = "one pass keeps fold completeness, exact candidate arrays, OOS totals and legacy diagnostic reconciliation inseparable"
    )]
    pub fn from_runner(
        population_id: [u8; 32],
        ranking_policy_digest: [u8; 32],
        validated: &Validated,
        supplied_legacy_diagnostic: Option<&Pbo>,
    ) -> Result<EvidenceSourceV1<Self>, String> {
        require_digest("validation population", &population_id)?;
        require_digest("validation ranking policy", &ranking_policy_digest)?;
        if validated.refused.is_some() {
            return Ok(EvidenceSourceV1::Refused);
        }
        if validated.folds.is_empty() {
            return Ok(EvidenceSourceV1::Unmeasured);
        }
        for (expected_index, fold) in validated.folds.iter().enumerate() {
            if fold.index != expected_index {
                return Err(format!(
                    "walk-forward fold index {} is not canonical sequence {expected_index}",
                    fold.index
                ));
            }
            if fold.halted.is_some() {
                return Ok(EvidenceSourceV1::Refused);
            }
            if fold.priced != fold.considered {
                return Err(format!(
                    "walk-forward fold {} priced {} candidates but considered {}; candidate truncation cannot become admission evidence",
                    fold.index, fold.priced, fold.considered
                ));
            }
            if fold.in_sample_all.len() != fold.out_of_sample_all.len() {
                return Err(format!(
                    "walk-forward fold {} carries misaligned in/out-of-sample candidate families",
                    fold.index
                ));
            }
            if usize_u64("walk-forward candidate family", fold.in_sample_all.len())? != fold.priced
            {
                return Err(format!(
                    "walk-forward fold {} retains {} candidate scores after pricing {} candidates",
                    fold.index,
                    fold.in_sample_all.len(),
                    fold.priced
                ));
            }
        }

        let decided = validated.decided();
        let mut profitable = 0_usize;
        let mut aggregate = 0_i64;
        let mut aggregate_complete = true;
        for fold in validated.folds.iter().filter(|fold| fold.chosen.is_some()) {
            match fold.out_of_sample_exit {
                Some(value) => {
                    aggregate = aggregate.checked_add(value).ok_or_else(|| {
                        "aggregate pessimistic out-of-sample return overflowed i64".to_owned()
                    })?;
                    let increment = usize::from(value > 0);
                    profitable = profitable.checked_add(increment).ok_or_else(|| {
                        "profitable out-of-sample fold count overflowed usize".to_owned()
                    })?;
                }
                None => aggregate_complete = false,
            }
        }

        let legacy_diagnostic = derive_anchored_fold_legacy_diagnostic(validated)?;
        if supplied_legacy_diagnostic.is_some_and(|supplied| *supplied != legacy_diagnostic) {
            return Err(
                "supplied anchored-fold legacy diagnostic does not equal the fold-derived diagnostic"
                    .to_owned(),
            );
        }
        let classified = legacy_diagnostic
            .folds
            .checked_add(legacy_diagnostic.unrankable)
            .ok_or_else(|| "anchored-fold legacy denominator overflowed usize".to_owned())?;
        if classified != validated.folds.len() {
            return Err(format!(
                "anchored-fold legacy diagnostic classified {classified} folds but walk-forward produced {}",
                validated.folds.len()
            ));
        }
        if legacy_diagnostic.overfit_folds > legacy_diagnostic.folds {
            return Err(
                "anchored-fold legacy overfit count exceeds its contributing folds".to_owned(),
            );
        }
        if legacy_diagnostic.folds > 0
            && !(0..=PPM.cast_signed()).contains(&legacy_diagnostic.median_placement)
        {
            return Err(format!(
                "anchored-fold legacy median placement {} is outside [0,{PPM}] ppm",
                legacy_diagnostic.median_placement
            ));
        }

        Ok(EvidenceSourceV1::Measured(Self {
            population_id,
            ranking_policy_digest,
            decided_folds: ObservedU64V1::Measured(usize_u64("decided folds", decided)?),
            profitable_oos_folds: if aggregate_complete && decided > 0 {
                ObservedU64V1::Measured(usize_u64("profitable out-of-sample folds", profitable)?)
            } else {
                ObservedU64V1::Unmeasured
            },
            oos_pessimistic_return_paisa: if aggregate_complete && decided > 0 {
                ObservedI64V1::Measured(aggregate)
            } else {
                ObservedI64V1::Unmeasured
            },
        }))
    }
}

fn derive_anchored_fold_legacy_diagnostic(validated: &Validated) -> Result<Pbo, String> {
    let mut placements = Vec::new();
    placements
        .try_reserve_exact(validated.folds.len())
        .map_err(|why| format!("could not reserve anchored-fold legacy placements: {why}"))?;
    for fold in &validated.folds {
        let placement = if fold.in_sample_all.is_empty() {
            Placement {
                candidates: 0,
                winner_rank: 0,
            }
        } else {
            place(&fold.in_sample_all, &fold.out_of_sample_all).ok_or_else(|| {
                format!(
                    "walk-forward fold {} could not produce an aligned legacy placement",
                    fold.index
                )
            })?
        };
        placements.push(placement);
    }
    Ok(probability_of_overfitting(&placements))
}

/// Identity-bound family-test projections available from current runner APIs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FamilyTestEvidenceV1 {
    population_id: [u8; 32],
    selection_receipt_digest: Option<[u8; 32]>,
    ranking_policy_digest: [u8; 32],
    strategy_digest: [u8; 32],
    durable_statistics_completion_digest: Option<[u8; 32]>,
    fwer_p_value_ppm: ObservedU64V1,
    spa_p_value_ppm: ObservedU64V1,
    bootstrap_draws: ObservedU64V1,
    bootstrap_strategies: ObservedU64V1,
    bootstrap_periods: ObservedU64V1,
    white_reality_p_value_ppm: ObservedU64V1,
    romano_wolf_p_value_ppm: ObservedU64V1,
    white_reality_decision: HypothesisDecisionV1,
    romano_wolf_decision: HypothesisDecisionV1,
}

impl FamilyTestEvidenceV1 {
    /// Reconciles White, SPA and one complete Romano--Wolf receipt.
    ///
    /// All three procedures must report identical draw/strategy/period
    /// denominators, and Romano--Wolf must use the canonical 5% family-wise
    /// threshold.  The opaque receipt distinguishes a complete non-rejection
    /// from malformed input, so both candidate decisions and the shared
    /// bootstrap denominators become measured.  The legacy fixed-alpha receipt
    /// accepted by this constructor carries decisions but no candidate-adjusted
    /// probabilities, so `romano_wolf_p_value_ppm` remains explicitly
    /// unmeasured on this path.  There is likewise no separate generic FWER
    /// p-value source, so `fwer_p_value_ppm` remains unmeasured rather than
    /// copied from White.
    ///
    /// # Errors
    ///
    /// Refuses absent identities, invalid/non-finite probabilities, mismatched
    /// family denominators, a noncanonical Romano--Wolf alpha, or an
    /// out-of-range candidate index.
    pub fn from_runner(
        population_id: [u8; 32],
        ranking_policy_digest: [u8; 32],
        strategy_digest: [u8; 32],
        candidate_index: usize,
        white: Option<&Verdict>,
        spa: Option<&Verdict>,
        romano_wolf: Option<&RomanoWolfReceipt>,
    ) -> Result<EvidenceSourceV1<Self>, String> {
        require_digest("family-test population", &population_id)?;
        require_digest("family-test ranking policy", &ranking_policy_digest)?;
        require_digest("family-test strategy", &strategy_digest)?;
        let (Some(white), Some(spa), Some(romano_wolf)) = (white, spa, romano_wolf) else {
            return Ok(EvidenceSourceV1::Unmeasured);
        };
        let family = (white.draws, white.strategies, white.periods);
        if family != (spa.draws, spa.strategies, spa.periods)
            || family
                != (
                    romano_wolf.draws(),
                    romano_wolf.strategies(),
                    romano_wolf.periods(),
                )
        {
            return Err(
                "White, SPA and Romano-Wolf describe different bootstrap families".to_owned(),
            );
        }
        if white.draws == 0 || white.strategies == 0 || white.periods == 0 {
            return Err(
                "family-test verdict has a zero draw, strategy or period denominator".to_owned(),
            );
        }
        if !white.statistic.is_finite() || !spa.statistic.is_finite() {
            return Err("family-test verdict has a non-finite source statistic".to_owned());
        }
        if romano_wolf.alpha_ppm() != CANONICAL_FWER_ALPHA_PPM {
            return Err(format!(
                "Romano-Wolf alpha {} ppm differs from canonical {CANONICAL_FWER_ALPHA_PPM} ppm",
                romano_wolf.alpha_ppm()
            ));
        }
        if candidate_index >= white.strategies {
            return Err(format!(
                "family candidate index {candidate_index} is outside {} compared strategies",
                white.strategies
            ));
        }
        let white_ppm = probability_ppm("White Reality Check", white.p_value)?;
        let spa_ppm = probability_ppm("SPA", spa.p_value)?;
        let candidate_rejected = romano_wolf.is_rejected(candidate_index).ok_or_else(|| {
            format!(
                "family candidate index {candidate_index} is outside Romano-Wolf family size {}",
                romano_wolf.strategies()
            )
        })?;
        let bootstrap_draws = usize_u64("bootstrap draws", white.draws)?;
        let bootstrap_strategies = usize_u64("bootstrap strategies", white.strategies)?;
        let bootstrap_periods = usize_u64("bootstrap periods", white.periods)?;

        Ok(EvidenceSourceV1::Measured(Self {
            population_id,
            selection_receipt_digest: None,
            ranking_policy_digest,
            strategy_digest,
            durable_statistics_completion_digest: None,
            fwer_p_value_ppm: ObservedU64V1::Unmeasured,
            spa_p_value_ppm: ObservedU64V1::Measured(spa_ppm),
            bootstrap_draws: ObservedU64V1::Measured(bootstrap_draws),
            bootstrap_strategies: ObservedU64V1::Measured(bootstrap_strategies),
            bootstrap_periods: ObservedU64V1::Measured(bootstrap_periods),
            white_reality_p_value_ppm: ObservedU64V1::Measured(white_ppm),
            romano_wolf_p_value_ppm: ObservedU64V1::Unmeasured,
            white_reality_decision: if white.clears() {
                HypothesisDecisionV1::RejectedNull
            } else {
                HypothesisDecisionV1::DidNotReject
            },
            romano_wolf_decision: if candidate_rejected {
                HypothesisDecisionV1::RejectedNull
            } else {
                HypothesisDecisionV1::DidNotReject
            },
        }))
    }

    /// Reconciles White/SPA with one reopened exact family-statistics authority.
    ///
    /// Unlike [`Self::from_runner`], this path can measure the exact-count
    /// selected-candidate and full-family Romano--Wolf probabilities. The
    /// durable authority must bind the same population, receipt-last selection,
    /// ranking policy, strategy, candidate and family dimensions, and its
    /// retained White/SPA `f64` bits must equal the supplied typed verdicts.
    ///
    /// This remains a *family* source. It deliberately does not claim the
    /// broader global full-precision flag, whose Wilson/PBO/risk sources are not
    /// present in this authority.
    ///
    /// # Errors
    ///
    /// Refuses absent/foreign identities, a candidate or family mismatch,
    /// changed White/SPA source bits, invalid probabilities or an unavailable
    /// exact Romano--Wolf decision at the canonical alpha.
    #[expect(
        clippy::too_many_arguments,
        reason = "the four institutional identities, candidate position, two independent verdicts and durable receipt are all required explicitly at the join"
    )]
    pub fn from_durable_statistics(
        population_id: [u8; 32],
        selection_receipt_digest: [u8; 32],
        ranking_policy_digest: [u8; 32],
        strategy_digest: [u8; 32],
        candidate_index: usize,
        white: &Verdict,
        spa: &Verdict,
        statistics: &InstitutionalStatisticsAuthorityV1,
    ) -> Result<EvidenceSourceV1<Self>, String> {
        require_digest("family-test population", &population_id)?;
        require_digest("family-test selection receipt", &selection_receipt_digest)?;
        require_digest("family-test ranking policy", &ranking_policy_digest)?;
        require_digest("family-test strategy", &strategy_digest)?;
        statistics.require_identity(
            &population_id,
            &selection_receipt_digest,
            &ranking_policy_digest,
            &strategy_digest,
        )?;
        let candidate_index_u64 = usize_u64("family candidate index", candidate_index)?;
        if statistics.candidate_index() != candidate_index_u64 {
            return Err(format!(
                "durable family candidate {} differs from requested {candidate_index}",
                statistics.candidate_index()
            ));
        }
        let family = (
            usize_u64("White draws", white.draws)?,
            usize_u64("White strategies", white.strategies)?,
            usize_u64("White periods", white.periods)?,
        );
        if family
            != (
                usize_u64("SPA draws", spa.draws)?,
                usize_u64("SPA strategies", spa.strategies)?,
                usize_u64("SPA periods", spa.periods)?,
            )
            || family
                != (
                    statistics.draws(),
                    statistics.strategies(),
                    statistics.periods(),
                )
        {
            return Err(
                "White, SPA and durable Romano-Wolf describe different bootstrap families"
                    .to_owned(),
            );
        }
        if white.draws == 0 || white.strategies == 0 || white.periods == 0 {
            return Err(
                "durable family-test verdict has a zero draw, strategy or period denominator"
                    .to_owned(),
            );
        }
        if (
            statistics.white_statistic().to_bits(),
            statistics.white_p_value().to_bits(),
            statistics.spa_statistic().to_bits(),
            statistics.spa_p_value().to_bits(),
        ) != (
            white.statistic.to_bits(),
            white.p_value.to_bits(),
            spa.statistic.to_bits(),
            spa.p_value.to_bits(),
        ) {
            return Err(
                "durable family statistics retain different White/SPA source bits".to_owned(),
            );
        }
        let white_ppm = probability_ppm("White Reality Check", white.p_value)?;
        let spa_ppm = probability_ppm("SPA", spa.p_value)?;
        let candidate_rejected = statistics
            .romano_wolf_rejects_at_ppm(CANONICAL_FWER_ALPHA_PPM)
            .ok_or_else(|| {
                "durable Romano-Wolf authority refused the canonical alpha".to_owned()
            })?;
        Ok(EvidenceSourceV1::Measured(Self {
            population_id,
            selection_receipt_digest: Some(selection_receipt_digest),
            ranking_policy_digest,
            strategy_digest,
            durable_statistics_completion_digest: Some(statistics.completion_digest()),
            fwer_p_value_ppm: ObservedU64V1::Measured(statistics.fwer_p_value_ppm()),
            spa_p_value_ppm: ObservedU64V1::Measured(spa_ppm),
            bootstrap_draws: ObservedU64V1::Measured(statistics.draws()),
            bootstrap_strategies: ObservedU64V1::Measured(statistics.strategies()),
            bootstrap_periods: ObservedU64V1::Measured(statistics.periods()),
            white_reality_p_value_ppm: ObservedU64V1::Measured(white_ppm),
            romano_wolf_p_value_ppm: ObservedU64V1::Measured(statistics.romano_wolf_p_value_ppm()),
            white_reality_decision: if white.clears() {
                HypothesisDecisionV1::RejectedNull
            } else {
                HypothesisDecisionV1::DidNotReject
            },
            romano_wolf_decision: if candidate_rejected {
                HypothesisDecisionV1::RejectedNull
            } else {
                HypothesisDecisionV1::DidNotReject
            },
        }))
    }

    /// Selection receipt bound by the durable post-selection family source.
    #[must_use]
    pub const fn selection_receipt_digest(&self) -> Option<[u8; 32]> {
        self.selection_receipt_digest
    }

    /// Receipt-last statistics completion bound by the durable family source.
    #[must_use]
    pub const fn durable_statistics_completion_digest(&self) -> Option<[u8; 32]> {
        self.durable_statistics_completion_digest
    }

    /// Exact-count-derived full-family probability projection.
    #[must_use]
    pub const fn fwer_p_value_ppm(&self) -> ObservedU64V1 {
        self.fwer_p_value_ppm
    }

    /// Exact-count-derived selected-candidate adjusted probability projection.
    #[must_use]
    pub const fn romano_wolf_p_value_ppm(&self) -> ObservedU64V1 {
        self.romano_wolf_p_value_ppm
    }

    /// Exact selected-candidate decision at the canonical family-wise alpha.
    #[must_use]
    pub const fn romano_wolf_decision(&self) -> HypothesisDecisionV1 {
        self.romano_wolf_decision
    }
}

/// Every source needed to build one canonical admission-evidence record.
///
/// The expected ranking digest binds procedure-level validation and family
/// tests to the population policy.  `trade_rows` and `independent_sessions`
/// retain their explicit non-measurement/refusal states rather than accepting
/// empty slices or zero as substitutes.
#[derive(Clone, Copy, Debug)]
pub struct InstitutionalEvidenceSourcesV1<'a> {
    /// Exact opaque cell/evaluation identity supplied by Population V4.
    pub context: PopulationCellContextV1<'a>,
    /// Population V4 final ranking-policy identity.
    pub ranking_policy_digest: [u8; 32],
    /// Exact selected-cell trade evidence.
    pub trade_rows: EvidenceSourceV1<TradeEvidenceV1>,
    /// Distinct signal sessions supporting this mask.
    pub independent_sessions: EvidenceSourceV1<IndependentSessionsEvidenceV1>,
    /// Procedure-level walk-forward evidence plus optional legacy diagnostic.
    pub validation: EvidenceSourceV1<ValidationEvidenceV1>,
    /// Family-level White/SPA/RW evidence.
    pub family_tests: EvidenceSourceV1<FamilyTestEvidenceV1>,
    /// Typed completeness sources owned outside one cell.
    pub completeness: InstitutionalCompletenessV1<'a>,
    /// Current raw-statistics source state.
    pub full_precision_statistics: FullPrecisionStatisticsSourceV1,
}

/// Builds one canonical fail-closed admission-evidence record.
///
/// # Errors
///
/// Refuses any torn/foreign evaluated cell, row/cell mismatch, invalid direct
/// metric, contradictory source identity, impossible subset/count relation or
/// canonical admission-domain refusal.  Missing optional authorities do not
/// cause an error: their fields retain `Unmeasured` or `Refused`, which makes
/// the resulting policy verdict fail closed.
pub fn build_institutional_evidence_v1(
    sources: InstitutionalEvidenceSourcesV1<'_>,
) -> Result<AdmissionEvidenceV1, String> {
    require_digest("evidence population", &sources.context.population_id)?;
    require_digest("evidence strategy", &sources.context.strategy_digest)?;
    require_digest("evidence ranking policy", &sources.ranking_policy_digest)?;
    require_context_evaluation(&sources.context)?;
    if sources.context.evaluated.mask().words() != sources.context.mask_words {
        return Err("institutional evidence context mask differs from evaluated mask".to_owned());
    }
    require_context_cell(&sources.context)?;

    let direct = direct_cell_values(sources.context.cell)?;
    let rows = trade_evidence_values(&sources.context, sources.trade_rows)?;
    let independent_sessions = independent_sessions_value(
        sources.independent_sessions,
        sources.context.population_id,
        sources.context.strategy_digest,
    )?;
    let completeness = completeness_values(
        &sources.context,
        sources.ranking_policy_digest,
        sources.completeness,
    )?;

    let validation = validation_values(
        sources.validation,
        sources.context.population_id,
        sources.ranking_policy_digest,
    )?;
    let family = family_values(
        &sources.family_tests,
        sources.context.population_id,
        sources.ranking_policy_digest,
        sources.context.strategy_digest,
    )?;
    let evidence = AdmissionEvidenceValuesV1 {
        support_hits: ObservedU64V1::Measured(sources.context.support_hits),
        independent_sessions,
        trades: ObservedU64V1::Measured(direct.trades),
        max_mae_paisa: rows.max_mae_paisa,
        worst_reward_risk_ppm: direct.worst_reward_risk_ppm,
        win_rate_ppm: direct.win_rate_ppm,
        wilson_win_rate_ppm: direct.wilson_win_rate_ppm,
        return_drawdown_ppm: direct.return_drawdown_ppm,
        weakest_period_return_paisa: rows.weakest_period_return_paisa,
        pbo_ppm: validation.pbo_ppm,
        fwer_p_value_ppm: family.fwer_p_value_ppm,
        spa_p_value_ppm: family.spa_p_value_ppm,
        decided_folds: validation.decided_folds,
        ambiguous_fill_rate_ppm: direct.ambiguous_fill_rate_ppm,
        gap_affected_rate_ppm: direct.gap_affected_rate_ppm,
        session_concentration_ppm: rows.session_concentration_ppm,
        largest_trade_profit_share_ppm: rows.largest_trade_profit_share_ppm,
        execution_complete: completeness.execution,
        data_complete: completeness.data,
        calendar_complete: completeness.calendar,
        population_complete: completeness.population,
        drawdown_paisa: ObservedU64V1::Measured(direct.drawdown_paisa),
        worst_trade_loss_paisa: ObservedU64V1::Measured(direct.worst_trade_loss_paisa),
        losing_trade_rate_ppm: direct.losing_trade_rate_ppm,
        losing_trades: ObservedU64V1::Measured(direct.losing_trades),
        pessimistic_profit_paisa: ObservedI64V1::Measured(direct.pessimistic_profit_paisa),
        winning_trades: ObservedU64V1::Measured(direct.winning_trades),
        average_win_paisa: direct.average_win_paisa,
        average_loss_paisa: direct.average_loss_paisa,
        profit_factor_ppm: direct.profit_factor_ppm,
        consecutive_losing_streak: direct.consecutive_losing_streak,
        consecutive_winning_streak: direct.consecutive_winning_streak,
        bootstrap_draws: family.bootstrap_draws,
        bootstrap_strategies: family.bootstrap_strategies,
        bootstrap_periods: family.bootstrap_periods,
        pbo_contributing_folds: validation.pbo_contributing_folds,
        pbo_unrankable_folds: validation.pbo_unrankable_folds,
        profitable_oos_folds: validation.profitable_oos_folds,
        oos_pessimistic_return_paisa: validation.oos_pessimistic_return_paisa,
        white_reality_p_value_ppm: family.white_reality_p_value_ppm,
        romano_wolf_p_value_ppm: family.romano_wolf_p_value_ppm,
        white_reality_decision: family.white_reality_decision,
        romano_wolf_decision: family.romano_wolf_decision,
        full_precision_statistics_complete: match sources.full_precision_statistics {
            FullPrecisionStatisticsSourceV1::Unmeasured => CompletenessV1::Unmeasured,
            FullPrecisionStatisticsSourceV1::Refused => CompletenessV1::Refused,
        },
    };
    AdmissionEvidenceV1::new(evidence)
        .map_err(|why| format!("canonical institutional evidence refused: {why:?}"))
}

#[derive(Clone, Copy)]
struct CompletenessValuesV1 {
    execution: CompletenessV1,
    data: CompletenessV1,
    calendar: CompletenessV1,
    population: CompletenessV1,
}

fn completeness_values(
    context: &PopulationCellContextV1<'_>,
    ranking_policy_digest: [u8; 32],
    sources: InstitutionalCompletenessV1<'_>,
) -> Result<CompletenessValuesV1, String> {
    match sources.population_authority {
        EvidenceSourceV1::Measured(authority) => {
            complete_population_values(context, ranking_policy_digest, authority, sources.data)
        }
        EvidenceSourceV1::Unmeasured => incomplete_population_values(
            CompletenessV1::Unmeasured,
            sources.data,
            "stored-data completeness cannot be bound without its complete population authority",
        ),
        EvidenceSourceV1::Refused => incomplete_population_values(
            CompletenessV1::Refused,
            sources.data,
            "stored-data completeness cannot override a refused population authority",
        ),
    }
}

fn complete_population_values(
    context: &PopulationCellContextV1<'_>,
    ranking_policy_digest: [u8; 32],
    authority: &CompletePopulationAuthorityV1<'_>,
    data_source: DataCompletenessSourceV1<'_>,
) -> Result<CompletenessValuesV1, String> {
    if authority.identities.ranking_policy_digest != ranking_policy_digest {
        return Err(
            "institutional evidence ranking policy differs from its complete population authority"
                .to_owned(),
        );
    }
    let derived = derive_population_id_v1(authority).map_err(|why| {
        format!("complete population authority refused during evidence binding: {why}")
    })?;
    if derived != context.population_id {
        return Err("complete population authority derives another population identity".to_owned());
    }
    reconcile_completed_population_context(context, authority)?;
    let strategy_digest = derive_strategy_digest_from_validated_v1(
        context.population_id,
        authority.instrument_family,
        authority.rung_seconds,
        authority.identities.evaluation_policy_digest,
        context.direction,
        context.resolved,
        context.evaluated,
        context.validated,
        context.cell_ordinal,
    )
    .map_err(|why| format!("complete strategy identity refused during evidence binding: {why}"))?;
    if strategy_digest != context.strategy_digest {
        return Err(
            "institutional evidence strategy differs from the canonical complete-population strategy"
                .to_owned(),
        );
    }
    let data = data_completeness_value(context.population_id, authority, data_source)?;
    Ok(CompletenessValuesV1 {
        execution: CompletenessV1::Complete,
        data,
        calendar: CompletenessV1::Complete,
        population: CompletenessV1::Complete,
    })
}

fn data_completeness_value(
    population_id: [u8; 32],
    authority: &CompletePopulationAuthorityV1<'_>,
    data_source: DataCompletenessSourceV1<'_>,
) -> Result<CompletenessV1, String> {
    match data_source {
        DataCompletenessSourceV1::Complete(data_authority) => {
            data_authority.require_population(authority).map_err(|why| {
                format!(
                    "stored-data completeness authority refused during institutional evidence binding: {why}"
                )
            })?;
            if data_authority.population_id() != population_id {
                return Err(
                    "stored-data completeness authority belongs to another evidence population"
                        .to_owned(),
                );
            }
            Ok(CompletenessV1::Complete)
        }
        DataCompletenessSourceV1::Unmeasured => Ok(CompletenessV1::Unmeasured),
        DataCompletenessSourceV1::Refused => Ok(CompletenessV1::Refused),
    }
}

#[cfg(test)]
pub(crate) fn test_data_completeness_value(
    population_id: [u8; 32],
    authority: &CompletePopulationAuthorityV1<'_>,
    data_source: DataCompletenessSourceV1<'_>,
) -> Result<CompletenessV1, String> {
    data_completeness_value(population_id, authority, data_source)
}

fn incomplete_population_values(
    population: CompletenessV1,
    data_source: DataCompletenessSourceV1<'_>,
    complete_refusal: &str,
) -> Result<CompletenessValuesV1, String> {
    let data = match data_source {
        DataCompletenessSourceV1::Complete(_) => return Err(complete_refusal.to_owned()),
        DataCompletenessSourceV1::Unmeasured => CompletenessV1::Unmeasured,
        DataCompletenessSourceV1::Refused => CompletenessV1::Refused,
    };
    Ok(CompletenessValuesV1 {
        execution: population,
        data,
        calendar: population,
        population,
    })
}

fn reconcile_completed_population_context(
    context: &PopulationCellContextV1<'_>,
    authority: &CompletePopulationAuthorityV1<'_>,
) -> Result<(), String> {
    let run = context.population_run;
    let reconciliation = context.reconciliation;
    if !run.is_complete() {
        return Err(
            "institutional evidence context carries an incomplete population run".to_owned(),
        );
    }
    let infrequent = run
        .outcome
        .trials
        .checked_sub(run.considered)
        .ok_or_else(|| "population frequent count exceeds its sweep trials".to_owned())?;
    let extinction_depth = u32::try_from(run.outcome.sweep.depth())
        .map_err(|_| "population extinction depth does not fit u32".to_owned())?;
    if (
        reconciliation.sweep_trials,
        reconciliation.frequent_itemsets,
        reconciliation.infrequent_itemsets,
        reconciliation.closed_itemsets,
        reconciliation.redundant_itemsets,
        reconciliation.unknown_closure_itemsets,
        reconciliation.extinction_depth,
        reconciliation.extinction_complete,
        reconciliation.closure_complete,
    ) != (
        run.outcome.trials,
        run.considered,
        infrequent,
        run.closed,
        run.redundant,
        0,
        extinction_depth,
        run.outcome.sweep.completed(),
        run.outcome.closure_complete,
    ) {
        return Err(
            "population reconciliation does not equal the completed uncapped run".to_owned(),
        );
    }
    if (
        reconciliation.exit_cells_per_mask.long(),
        reconciliation.exit_cells_per_mask.short(),
    ) != (
        authority.long_exit_grid.cell_count(),
        authority.short_exit_grid.cell_count(),
    ) {
        return Err(
            "population reconciliation exit-cell cardinalities differ from its typed authority"
                .to_owned(),
        );
    }
    let expected_resolution = match context.direction {
        crate::population::TradeDirectionV1::Long => authority.long_exit_grid,
        crate::population::TradeDirectionV1::Short => authority.short_exit_grid,
    };
    if !core::ptr::eq(context.resolved, expected_resolution) {
        return Err(
            "population cell context does not use its authority's side-specific resolution"
                .to_owned(),
        );
    }
    Ok(())
}

/// Measures independent support sessions from the exact signal column.
///
/// The column must still use signal-source semantics; a reprojected fill column
/// counts execution bars, not causally independent signal sessions.  The
/// measured hit count must equal the population member's support exactly.
///
/// # Errors
///
/// Refuses missing population/strategy identities or any signal-column scan
/// that cannot reconcile exactly with the population member's recorded support.
pub fn measure_independent_support_sessions_v1(
    context: &PopulationCellContextV1<'_>,
    signal_bars: &[Candle],
) -> Result<EvidenceSourceV1<IndependentSessionsEvidenceV1>, String> {
    require_digest("session population", &context.population_id)?;
    require_digest("session strategy", &context.strategy_digest)?;
    let measured = scan_independent_support_sessions_v1(
        signal_bars,
        &context.population_run.column,
        context.mask_words,
        context.support_hits,
    )?;
    Ok(match measured {
        EvidenceSourceV1::Measured(sessions) => {
            EvidenceSourceV1::Measured(IndependentSessionsEvidenceV1 {
                population_id: context.population_id,
                strategy_digest: context.strategy_digest,
                sessions,
            })
        }
        EvidenceSourceV1::Unmeasured => EvidenceSourceV1::Unmeasured,
        EvidenceSourceV1::Refused => EvidenceSourceV1::Refused,
    })
}

fn scan_independent_support_sessions_v1(
    signal_bars: &[Candle],
    signal_column: &Column,
    mask_words: [u64; 6],
    expected_support_hits: u64,
) -> Result<EvidenceSourceV1<u64>, String> {
    if signal_column.sourced() != Sourced::Signal {
        return Err("independent-session evidence requires a signal-sourced column".to_owned());
    }
    let census = signal_column.census();
    if !census.reconciles() {
        return Err("signal column census does not reconcile its offered bars".to_owned());
    }
    if census.offered != usize_u64("signal bar count", signal_bars.len())? {
        return Err(format!(
            "signal column was offered {} bars but session scan received {}",
            census.offered,
            signal_bars.len()
        ));
    }
    if census.swept != usize_u64("signal column width", signal_column.bits().len())? {
        return Err("signal column swept count differs from its mask width".to_owned());
    }
    if signal_column.bits().len() != signal_column.sources().len() {
        return Err("signal column bits and source indices are not parallel".to_owned());
    }
    let mask = runner::replay_mask::from_stored_words(mask_words)
        .map_err(|why| format!("independent-session mask is not canonical/live: {why}"))?;
    if mask.words().iter().all(|word| *word == 0) {
        return Err("independent-session evidence cannot measure an empty mask".to_owned());
    }
    let mut hits = 0_u64;
    let mut sessions = 0_u64;
    let mut previous_day = None;
    let mut previous_source = None;
    for (bar_mask, source) in signal_column
        .bits()
        .iter()
        .zip(signal_column.sources().iter().copied())
    {
        if previous_source.is_some_and(|previous| source <= previous) {
            return Err("signal column sources are not strictly increasing".to_owned());
        }
        previous_source = Some(source);
        if !bar_mask.hits(&mask) {
            continue;
        }
        let bar = signal_bars.get(source).ok_or_else(|| {
            format!(
                "signal column source {source} is outside {} supplied bars",
                signal_bars.len()
            )
        })?;
        hits = hits
            .checked_add(1)
            .ok_or_else(|| "independent-session support count overflowed u64".to_owned())?;
        let day = ist_day(bar.ts_micros)?;
        if previous_day != Some(day) {
            sessions = sessions
                .checked_add(1)
                .ok_or_else(|| "independent-session count overflowed u64".to_owned())?;
            previous_day = Some(day);
        }
    }
    if hits != expected_support_hits {
        return Err(format!(
            "independent-session scan measured {hits} support hits, not population support {expected_support_hits}"
        ));
    }
    if hits == 0 {
        Ok(EvidenceSourceV1::Unmeasured)
    } else {
        Ok(EvidenceSourceV1::Measured(sessions))
    }
}

#[derive(Clone, Copy, Debug)]
struct DirectCellEvidenceV1 {
    trades: u64,
    worst_reward_risk_ppm: ObservedU64V1,
    win_rate_ppm: ObservedU64V1,
    wilson_win_rate_ppm: ObservedU64V1,
    return_drawdown_ppm: ObservedU64V1,
    ambiguous_fill_rate_ppm: ObservedU64V1,
    gap_affected_rate_ppm: ObservedU64V1,
    drawdown_paisa: u64,
    worst_trade_loss_paisa: u64,
    losing_trade_rate_ppm: ObservedU64V1,
    losing_trades: u64,
    pessimistic_profit_paisa: i64,
    winning_trades: u64,
    average_win_paisa: ObservedU64V1,
    average_loss_paisa: ObservedU64V1,
    profit_factor_ppm: ObservedU64V1,
    consecutive_losing_streak: ObservedU64V1,
    consecutive_winning_streak: ObservedU64V1,
}

fn direct_cell_values(cell: &Cell) -> Result<DirectCellEvidenceV1, String> {
    if cell.wins > cell.trades {
        return Err(format!(
            "evaluated cell has {} wins above {} trades",
            cell.wins, cell.trades
        ));
    }
    if cell.max_drawdown < 0 || cell.gross_win < 0 || cell.gross_loss > 0 || cell.min_win < 0 {
        return Err(
            "evaluated cell has negative drawdown/gross-win/minimum-win or positive gross-loss"
                .to_owned(),
        );
    }
    let classified_exits = cell
        .stopped
        .checked_add(cell.trailed_stop)
        .and_then(|total| total.checked_add(cell.trailed_profit))
        .and_then(|total| total.checked_add(cell.targeted))
        .and_then(|total| total.checked_add(cell.timed_out))
        .ok_or_else(|| "evaluated cell exit classification overflowed u64".to_owned())?;
    if classified_exits != cell.trades {
        return Err(format!(
            "evaluated cell classifies {classified_exits} exits for {} trades",
            cell.trades
        ));
    }
    if cell.ambiguous_bars > cell.trades || cell.gapped > cell.trades {
        return Err("evaluated cell ambiguity/gap count exceeds its trades".to_owned());
    }
    let losing = cell.trades - cell.wins;
    let rates_measured = cell.trades > 0;
    let average_win = if cell.wins == 0 {
        ObservedU64V1::Unmeasured
    } else {
        ObservedU64V1::Measured(nonnegative_u64("average win", cell.avg_win())?)
    };
    let average_loss = if losing == 0 {
        ObservedU64V1::Unmeasured
    } else {
        ObservedU64V1::Measured(cell.avg_loss().unsigned_abs())
    };
    let worst_loss = negative_magnitude(cell.worst_trade);
    let min_win = nonnegative_u64("minimum win", cell.min_win)?;
    let gross_win = nonnegative_u64("gross win", cell.gross_win)?;
    let gross_loss = negative_magnitude(cell.gross_loss);

    Ok(DirectCellEvidenceV1 {
        trades: cell.trades,
        worst_reward_risk_ppm: ratio_observed(min_win, worst_loss)?,
        win_rate_ppm: measured_rate(cell.wins, cell.trades)?,
        wilson_win_rate_ppm: if rates_measured {
            ObservedU64V1::Measured(wilson_lower_ppm(
                cell.wins,
                cell.trades,
                cell.assurance_bp(),
            )?)
        } else {
            ObservedU64V1::Unmeasured
        },
        return_drawdown_ppm: if rates_measured {
            return_drawdown(cell.pessimistic, cell.max_drawdown)?
        } else {
            ObservedU64V1::Unmeasured
        },
        ambiguous_fill_rate_ppm: measured_rate(cell.ambiguous_bars, cell.trades)?,
        gap_affected_rate_ppm: measured_rate(cell.gapped, cell.trades)?,
        drawdown_paisa: nonnegative_u64("maximum drawdown", cell.max_drawdown)?,
        worst_trade_loss_paisa: worst_loss,
        losing_trade_rate_ppm: measured_rate(losing, cell.trades)?,
        losing_trades: losing,
        pessimistic_profit_paisa: cell.pessimistic,
        winning_trades: cell.wins,
        average_win_paisa: average_win,
        average_loss_paisa: average_loss,
        profit_factor_ppm: ratio_observed(gross_win, gross_loss)?,
        consecutive_losing_streak: if cell.trades == 0 {
            ObservedU64V1::Unmeasured
        } else {
            ObservedU64V1::Measured(u64::from(cell.max_losing_streak))
        },
        consecutive_winning_streak: if cell.trades == 0 {
            ObservedU64V1::Unmeasured
        } else {
            ObservedU64V1::Measured(u64::from(cell.max_winning_streak))
        },
    })
}

impl TradeEvidenceV1 {
    /// Replays one exact cell and derives its row-only institutional metrics.
    ///
    /// # Errors
    ///
    /// Refuses a foreign authority/context, torn evaluation, non-canonical
    /// mask, replay disagreement or any TradeRow-to-Cell reconciliation gap.
    pub fn from_exact_replay(
        context: &PopulationCellContextV1<'_>,
        authority: &CompletePopulationAuthorityV1<'_>,
    ) -> Result<EvidenceSourceV1<Self>, String> {
        let derived = derive_population_id_v1(authority)
            .map_err(|why| format!("trade replay population authority refused: {why}"))?;
        if derived != context.population_id {
            return Err("trade replay authority derives another population identity".to_owned());
        }
        reconcile_completed_population_context(context, authority)?;
        require_context_evaluation(context)?;
        require_context_cell(context)?;
        let mask = runner::replay_mask::from_stored_words(context.mask_words)
            .map_err(|why| format!("trade replay mask is not canonical/live: {why}"))?;
        if mask.words() != context.evaluated.mask().words() {
            return Err("trade replay context and evaluated masks differ".to_owned());
        }
        let side = match context.direction {
            crate::population::TradeDirectionV1::Long => runner::excursion::Side::Long,
            crate::population::TradeDirectionV1::Short => runner::excursion::Side::Short,
        };
        let rows = runner::grid::materialize_cell(
            authority.execution_series.bars(),
            authority.execution_column,
            &mask,
            authority.horizon,
            side,
            context.evaluated.grid(),
            context.cell,
        )
        .map_err(|why| format!("exact chosen-cell TradeRow replay refused: {why}"))?;
        let values = reconcile_trade_rows(context.cell, &rows)?;
        Ok(EvidenceSourceV1::Measured(Self {
            population_id: context.population_id,
            strategy_digest: context.strategy_digest,
            values,
        }))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ReconciledTradeEvidenceV1 {
    max_mae_paisa: ObservedU64V1,
    weakest_period_return_paisa: ObservedI64V1,
    session_concentration_ppm: ObservedU64V1,
    largest_trade_profit_share_ppm: ObservedU64V1,
}

impl ReconciledTradeEvidenceV1 {
    const fn unmeasured() -> Self {
        Self {
            max_mae_paisa: ObservedU64V1::Unmeasured,
            weakest_period_return_paisa: ObservedI64V1::Unmeasured,
            session_concentration_ppm: ObservedU64V1::Unmeasured,
            largest_trade_profit_share_ppm: ObservedU64V1::Unmeasured,
        }
    }

    const fn refused() -> Self {
        Self {
            max_mae_paisa: ObservedU64V1::Refused,
            weakest_period_return_paisa: ObservedI64V1::Refused,
            session_concentration_ppm: ObservedU64V1::Refused,
            largest_trade_profit_share_ppm: ObservedU64V1::Refused,
        }
    }
}

fn trade_evidence_values(
    context: &PopulationCellContextV1<'_>,
    source: EvidenceSourceV1<TradeEvidenceV1>,
) -> Result<ReconciledTradeEvidenceV1, String> {
    match source {
        EvidenceSourceV1::Measured(measured) => {
            if measured.population_id != context.population_id
                || measured.strategy_digest != context.strategy_digest
            {
                return Err("trade evidence belongs to another population or strategy".to_owned());
            }
            Ok(measured.values)
        }
        EvidenceSourceV1::Unmeasured => Ok(ReconciledTradeEvidenceV1::unmeasured()),
        EvidenceSourceV1::Refused => Ok(ReconciledTradeEvidenceV1::refused()),
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one forward fold keeps every TradeRow-to-Cell reconciliation beside the accumulator it proves; splitting it would let row-derived evidence bypass part of the exact-cell check"
)]
fn reconcile_trade_rows(
    cell: &Cell,
    rows: &[TradeRow],
) -> Result<ReconciledTradeEvidenceV1, String> {
    let row_count = usize_u64("trade-row count", rows.len())?;
    if row_count != cell.trades {
        return Err(format!(
            "chosen-cell detail has {row_count} rows but evaluated cell has {} trades",
            cell.trades
        ));
    }
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
    let mut drawdown = 0_i64;
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

    for row in rows {
        if row.signal_bar > row.entry_bar || row.entry_bar > row.exit_bar {
            return Err("chosen-cell row has signal/entry/exit indices out of order".to_owned());
        }
        if row.entry_micros > row.exit_micros {
            return Err("chosen-cell row exits before it enters".to_owned());
        }
        if previous_entry.is_some_and(|previous| row.entry_micros <= previous) {
            return Err("chosen-cell rows are not strictly ordered by entry time".to_owned());
        }
        if occupied_through.is_some_and(|previous| row.entry_micros <= previous) {
            return Err("chosen-cell rows overlap under the single-position rule".to_owned());
        }
        if row.worst > row.best {
            return Err(
                "chosen-cell pessimistic trade is better than its optimistic trade".to_owned(),
            );
        }
        if row.adverse < 0
            || row.adverse_paisa < 0
            || row.favourable < 0
            || row.favourable_paisa < 0
        {
            return Err("chosen-cell trade carries a negative excursion magnitude".to_owned());
        }
        previous_entry = Some(row.entry_micros);
        occupied_through = Some(row.exit_micros);
        pessimistic = pessimistic
            .checked_add(row.worst)
            .ok_or_else(|| "chosen-cell pessimistic row sum overflowed i64".to_owned())?;
        optimistic = optimistic
            .checked_add(row.best)
            .ok_or_else(|| "chosen-cell optimistic row sum overflowed i64".to_owned())?;
        max_mae_paisa = max_mae_paisa.max(row.adverse_paisa.unsigned_abs());
        if row.worst > 0 {
            wins = wins
                .checked_add(1)
                .ok_or_else(|| "chosen-cell winning count overflowed u64".to_owned())?;
            gross_win = gross_win
                .checked_add(row.worst)
                .ok_or_else(|| "chosen-cell gross-win row sum overflowed i64".to_owned())?;
            best_trade = best_trade.max(row.worst);
            if min_win == 0 || row.worst < min_win {
                min_win = row.worst;
            }
            losing_streak = 0;
            winning_streak = winning_streak
                .checked_add(1)
                .ok_or_else(|| "chosen-cell winning streak overflowed u32".to_owned())?;
            max_winning_streak = max_winning_streak.max(winning_streak);
        } else {
            gross_loss = gross_loss
                .checked_add(row.worst)
                .ok_or_else(|| "chosen-cell gross-loss row sum overflowed i64".to_owned())?;
            winning_streak = 0;
            losing_streak = losing_streak
                .checked_add(1)
                .ok_or_else(|| "chosen-cell losing streak overflowed u32".to_owned())?;
            max_losing_streak = max_losing_streak.max(losing_streak);
        }
        worst_trade = worst_trade.min(row.worst);
        running = running
            .checked_add(row.worst)
            .ok_or_else(|| "chosen-cell running P&L overflowed i64".to_owned())?;
        peak = peak.max(running);
        drawdown = drawdown.max(
            peak.checked_sub(running)
                .ok_or_else(|| "chosen-cell drawdown overflowed i64".to_owned())?,
        );

        let day = ist_day(row.entry_micros)?;
        if session_day == Some(day) {
            session_trades = session_trades
                .checked_add(1)
                .ok_or_else(|| "chosen-cell session count overflowed u64".to_owned())?;
        } else {
            if session_day.is_some_and(|previous| day <= previous) {
                return Err("chosen-cell trade sessions are not strictly increasing".to_owned());
            }
            max_session_trades = max_session_trades.max(session_trades);
            session_day = Some(day);
            session_trades = 1;
        }
    }
    max_session_trades = max_session_trades.max(session_trades);

    if (
        pessimistic,
        optimistic,
        wins,
        gross_win,
        gross_loss,
        best_trade,
        min_win,
        worst_trade,
        drawdown,
        max_winning_streak,
        max_losing_streak,
    ) != (
        cell.pessimistic,
        cell.optimistic,
        cell.wins,
        cell.gross_win,
        cell.gross_loss,
        cell.best_trade,
        cell.min_win,
        cell.worst_trade,
        cell.max_drawdown,
        cell.max_winning_streak,
        cell.max_losing_streak,
    ) {
        return Err("chosen-cell trade detail does not reconcile every P&L, count, extreme, drawdown and streak field with its evaluated cell".to_owned());
    }

    if rows.is_empty() {
        return Ok(ReconciledTradeEvidenceV1::unmeasured());
    }

    let weakest = checked_weakest_period_return(rows)?;
    let gross_win_u64 = nonnegative_u64("row gross win", gross_win)?;
    let largest_share = if gross_win_u64 == 0 {
        ObservedU64V1::Unmeasured
    } else {
        ObservedU64V1::Measured(rate_ppm(
            nonnegative_u64("largest winning trade", best_trade)?,
            gross_win_u64,
        )?)
    };

    Ok(ReconciledTradeEvidenceV1 {
        max_mae_paisa: ObservedU64V1::Measured(max_mae_paisa),
        weakest_period_return_paisa: ObservedI64V1::Measured(weakest),
        session_concentration_ppm: ObservedU64V1::Measured(rate_ppm(
            max_session_trades,
            row_count,
        )?),
        largest_trade_profit_share_ppm: largest_share,
    })
}

fn checked_weakest_period_return(rows: &[TradeRow]) -> Result<i64, String> {
    let mut weakest = i64::MAX;
    for grain in crate::stability::GRAINS {
        let mut previous_key = None;
        let mut period_total = 0_i64;
        let mut grain_weakest = i64::MAX;
        for row in rows {
            let key = grain.bucket(row.entry_micros);
            match previous_key {
                None => {
                    previous_key = Some(key);
                    period_total = row.worst;
                }
                Some(previous) if key == previous => {
                    period_total = period_total.checked_add(row.worst).ok_or_else(|| {
                        format!("{} weakest-period return overflowed i64", grain.as_str())
                    })?;
                }
                Some(previous) if key > previous => {
                    grain_weakest = grain_weakest.min(period_total);
                    previous_key = Some(key);
                    period_total = row.worst;
                }
                Some(_) => {
                    return Err(format!(
                        "chosen-cell {} period keys are not monotone",
                        grain.as_str()
                    ));
                }
            }
        }
        if previous_key.is_none() {
            return Err("a non-empty trade stream produced an empty stability grain".to_owned());
        }
        grain_weakest = grain_weakest.min(period_total);
        weakest = weakest.min(grain_weakest);
    }
    Ok(weakest)
}

fn require_context_evaluation(context: &PopulationCellContextV1<'_>) -> Result<(), String> {
    if context.validated.resolution_digest() != context.resolved.digest()
        || !context.validated.is_evaluation(context.evaluated)
    {
        return Err(
            "institutional evidence received a torn evaluation capability from another resolution or evaluation"
                .to_owned(),
        );
    }
    Ok(())
}

fn require_context_cell(context: &PopulationCellContextV1<'_>) -> Result<(), String> {
    require_context_evaluation(context)?;
    let cell = context.cell;
    let stop = optional_index("context stop", cell.stop)?;
    let target = optional_index("context target", cell.target)?;
    let tsl = optional_index("context TSL", cell.tsl)?;
    let ttp = cell
        .ttp
        .map(|value| {
            Ok::<(u32, u32), String>((
                index_u32("context TTP arm", value.arm)?,
                index_u32("context TTP trail", value.trail)?,
            ))
        })
        .transpose()?;
    if (stop, target, tsl, ttp)
        != (
            context.exit.stop,
            context.exit.target,
            context.exit.tsl,
            context.exit.ttp,
        )
    {
        return Err("institutional evidence context coordinate differs from its cell".to_owned());
    }
    context_cell_ordinal(context)?;
    Ok(())
}

fn context_cell_ordinal(context: &PopulationCellContextV1<'_>) -> Result<usize, String> {
    match context.validated.cell(context.cell_ordinal) {
        Some(candidate) if core::ptr::eq(candidate, context.cell) => Ok(context.cell_ordinal),
        _ => Err(
            "institutional evidence cell is not owned by its evaluated grid at the stated ordinal"
                .to_owned(),
        ),
    }
}

#[derive(Clone, Copy)]
struct ValidationValuesV1 {
    pbo_ppm: ObservedU64V1,
    decided_folds: ObservedU64V1,
    pbo_contributing_folds: ObservedU64V1,
    pbo_unrankable_folds: ObservedU64V1,
    profitable_oos_folds: ObservedU64V1,
    oos_pessimistic_return_paisa: ObservedI64V1,
}

fn validation_values(
    source: EvidenceSourceV1<ValidationEvidenceV1>,
    population_id: [u8; 32],
    ranking_policy_digest: [u8; 32],
) -> Result<ValidationValuesV1, String> {
    match source {
        EvidenceSourceV1::Measured(value) => {
            if value.population_id != population_id
                || value.ranking_policy_digest != ranking_policy_digest
            {
                return Err(
                    "walk-forward/legacy-diagnostic evidence belongs to another population or ranking policy"
                        .to_owned(),
                );
            }
            Ok(ValidationValuesV1 {
                // D-0467: the retained runner value is an anchored-fold legacy
                // diagnostic.  Even a future internal constructor must not
                // promote it into genuine CSCV/PBO admission fields.
                pbo_ppm: ObservedU64V1::Unmeasured,
                decided_folds: value.decided_folds,
                pbo_contributing_folds: ObservedU64V1::Unmeasured,
                pbo_unrankable_folds: ObservedU64V1::Unmeasured,
                profitable_oos_folds: value.profitable_oos_folds,
                oos_pessimistic_return_paisa: value.oos_pessimistic_return_paisa,
            })
        }
        EvidenceSourceV1::Unmeasured => Ok(ValidationValuesV1 {
            pbo_ppm: ObservedU64V1::Unmeasured,
            decided_folds: ObservedU64V1::Unmeasured,
            pbo_contributing_folds: ObservedU64V1::Unmeasured,
            pbo_unrankable_folds: ObservedU64V1::Unmeasured,
            profitable_oos_folds: ObservedU64V1::Unmeasured,
            oos_pessimistic_return_paisa: ObservedI64V1::Unmeasured,
        }),
        EvidenceSourceV1::Refused => Ok(ValidationValuesV1 {
            pbo_ppm: ObservedU64V1::Refused,
            decided_folds: ObservedU64V1::Refused,
            pbo_contributing_folds: ObservedU64V1::Refused,
            pbo_unrankable_folds: ObservedU64V1::Refused,
            profitable_oos_folds: ObservedU64V1::Refused,
            oos_pessimistic_return_paisa: ObservedI64V1::Refused,
        }),
    }
}

#[derive(Clone, Copy)]
struct FamilyValuesV1 {
    fwer_p_value_ppm: ObservedU64V1,
    spa_p_value_ppm: ObservedU64V1,
    bootstrap_draws: ObservedU64V1,
    bootstrap_strategies: ObservedU64V1,
    bootstrap_periods: ObservedU64V1,
    white_reality_p_value_ppm: ObservedU64V1,
    romano_wolf_p_value_ppm: ObservedU64V1,
    white_reality_decision: HypothesisDecisionV1,
    romano_wolf_decision: HypothesisDecisionV1,
}

fn family_values(
    source: &EvidenceSourceV1<FamilyTestEvidenceV1>,
    population_id: [u8; 32],
    ranking_policy_digest: [u8; 32],
    strategy_digest: [u8; 32],
) -> Result<FamilyValuesV1, String> {
    match *source {
        EvidenceSourceV1::Measured(value) => {
            if value.population_id != population_id
                || value.ranking_policy_digest != ranking_policy_digest
                || value.strategy_digest != strategy_digest
            {
                return Err(
                    "family-test evidence belongs to another population, ranking policy or strategy"
                        .to_owned(),
                );
            }
            match (
                value.selection_receipt_digest,
                value.durable_statistics_completion_digest,
            ) {
                (Some(selection), Some(completion)) => {
                    require_digest("family-test selection receipt", &selection)?;
                    require_digest("family-test durable completion", &completion)?;
                    return Err(
                        "selection-bound durable family statistics are post-selection evidence and cannot authorize the pre-selection admission that selection requires"
                            .to_owned(),
                    );
                }
                (None, None) => {
                    if value.fwer_p_value_ppm != ObservedU64V1::Unmeasured
                        || value.romano_wolf_p_value_ppm != ObservedU64V1::Unmeasured
                    {
                        return Err(
                            "legacy in-memory family evidence cannot carry durable exact probabilities"
                                .to_owned(),
                        );
                    }
                }
                _ => {
                    return Err(
                        "family-test evidence has a partial durable statistics identity".to_owned(),
                    );
                }
            }
            Ok(FamilyValuesV1 {
                fwer_p_value_ppm: value.fwer_p_value_ppm,
                spa_p_value_ppm: value.spa_p_value_ppm,
                bootstrap_draws: value.bootstrap_draws,
                bootstrap_strategies: value.bootstrap_strategies,
                bootstrap_periods: value.bootstrap_periods,
                white_reality_p_value_ppm: value.white_reality_p_value_ppm,
                romano_wolf_p_value_ppm: value.romano_wolf_p_value_ppm,
                white_reality_decision: value.white_reality_decision,
                romano_wolf_decision: value.romano_wolf_decision,
            })
        }
        EvidenceSourceV1::Unmeasured => Ok(FamilyValuesV1 {
            fwer_p_value_ppm: ObservedU64V1::Unmeasured,
            spa_p_value_ppm: ObservedU64V1::Unmeasured,
            bootstrap_draws: ObservedU64V1::Unmeasured,
            bootstrap_strategies: ObservedU64V1::Unmeasured,
            bootstrap_periods: ObservedU64V1::Unmeasured,
            white_reality_p_value_ppm: ObservedU64V1::Unmeasured,
            romano_wolf_p_value_ppm: ObservedU64V1::Unmeasured,
            white_reality_decision: HypothesisDecisionV1::Unmeasured,
            romano_wolf_decision: HypothesisDecisionV1::Unmeasured,
        }),
        EvidenceSourceV1::Refused => Ok(FamilyValuesV1 {
            fwer_p_value_ppm: ObservedU64V1::Refused,
            spa_p_value_ppm: ObservedU64V1::Refused,
            bootstrap_draws: ObservedU64V1::Refused,
            bootstrap_strategies: ObservedU64V1::Refused,
            bootstrap_periods: ObservedU64V1::Refused,
            white_reality_p_value_ppm: ObservedU64V1::Refused,
            romano_wolf_p_value_ppm: ObservedU64V1::Refused,
            white_reality_decision: HypothesisDecisionV1::Refused,
            romano_wolf_decision: HypothesisDecisionV1::Refused,
        }),
    }
}

fn independent_sessions_value(
    source: EvidenceSourceV1<IndependentSessionsEvidenceV1>,
    population_id: [u8; 32],
    strategy_digest: [u8; 32],
) -> Result<ObservedU64V1, String> {
    match source {
        EvidenceSourceV1::Measured(value) => {
            if value.population_id != population_id || value.strategy_digest != strategy_digest {
                return Err(
                    "independent-session evidence belongs to another population or strategy"
                        .to_owned(),
                );
            }
            Ok(ObservedU64V1::Measured(value.sessions))
        }
        EvidenceSourceV1::Unmeasured => Ok(ObservedU64V1::Unmeasured),
        EvidenceSourceV1::Refused => Ok(ObservedU64V1::Refused),
    }
}

fn measured_rate(part: u64, total: u64) -> Result<ObservedU64V1, String> {
    if total == 0 {
        Ok(ObservedU64V1::Unmeasured)
    } else {
        Ok(ObservedU64V1::Measured(rate_ppm(part, total)?))
    }
}

fn rate_ppm(part: u64, total: u64) -> Result<u64, String> {
    if total == 0 {
        return Err("rate projection has a zero denominator".to_owned());
    }
    let projected = u128::from(part)
        .checked_mul(u128::from(PPM))
        .and_then(|scaled| scaled.checked_div(u128::from(total)))
        .ok_or_else(|| "rate projection overflowed".to_owned())?;
    u64::try_from(projected).map_err(|_| "rate projection does not fit u64".to_owned())
}

fn ratio_observed(numerator: u64, denominator: u64) -> Result<ObservedU64V1, String> {
    if denominator == 0 {
        return Ok(ObservedU64V1::Unmeasured);
    }
    let ratio = u128::from(numerator)
        .checked_mul(u128::from(PPM))
        .and_then(|scaled| scaled.checked_div(u128::from(denominator)))
        .ok_or_else(|| "ratio projection overflowed".to_owned())?;
    Ok(ObservedU64V1::Measured(u64::try_from(ratio).map_err(
        |_| "ratio projection does not fit u64".to_owned(),
    )?))
}

fn return_drawdown(profit: i64, drawdown: i64) -> Result<ObservedU64V1, String> {
    if profit <= 0 {
        return Ok(ObservedU64V1::Measured(0));
    }
    if drawdown < 0 {
        return Err("return/drawdown source has a negative drawdown".to_owned());
    }
    if drawdown == 0 {
        return Ok(ObservedU64V1::Measured(u64::MAX));
    }
    ratio_observed(profit.unsigned_abs(), drawdown.unsigned_abs())
}

#[expect(
    clippy::float_arithmetic,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Wilson is a full-precision statistic over counts, never a price; this projects it down only at the canonical admission comparison boundary"
)]
pub(crate) fn wilson_lower_ppm(wins: u64, trades: u64, expected_bp: i64) -> Result<u64, String> {
    const Z: f64 = 1.959_964;

    if trades == 0 || wins > trades {
        return Err("Wilson source has an invalid wins/trades denominator".to_owned());
    }
    let n = trades as f64;
    let p = wins as f64 / n;
    let z2 = Z * Z;
    let denominator = 1.0 + z2 / n;
    let centre = p + z2 / (2.0 * n);
    let margin = Z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt();
    let projected = (((centre - margin) / denominator) * PPM as f64)
        .clamp(0.0, PPM as f64)
        .floor() as u64;
    let bp = i64::try_from(projected / 100)
        .map_err(|_| "Wilson basis-point projection does not fit i64".to_owned())?;
    if bp != expected_bp {
        return Err(format!(
            "full-ppm Wilson projection {projected} does not reconcile with evaluated-cell bucket {expected_bp} bp"
        ));
    }
    Ok(projected)
}

#[expect(
    clippy::float_arithmetic,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "bootstrap probabilities are full-precision statistics, never prices; ppm exists only as the canonical admission comparison projection"
)]
fn probability_ppm(name: &str, value: f64) -> Result<u64, String> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(format!("{name} p-value is outside finite [0,1]"));
    }
    Ok((value * PPM as f64).floor() as u64)
}

fn require_digest(name: &str, digest: &[u8; 32]) -> Result<(), String> {
    if digest.iter().all(|byte| *byte == 0) {
        Err(format!("{name} identity is absent"))
    } else {
        Ok(())
    }
}

fn usize_u64(name: &str, value: usize) -> Result<u64, String> {
    u64::try_from(value).map_err(|_| format!("{name} does not fit u64"))
}

fn nonnegative_u64(name: &str, value: i64) -> Result<u64, String> {
    u64::try_from(value).map_err(|_| format!("{name} is negative"))
}

const fn negative_magnitude(value: i64) -> u64 {
    if value < 0 { value.unsigned_abs() } else { 0 }
}

fn optional_index(name: &str, index: Option<usize>) -> Result<Option<u32>, String> {
    index.map(|value| index_u32(name, value)).transpose()
}

fn index_u32(name: &str, index: usize) -> Result<u32, String> {
    let value = u32::try_from(index).map_err(|_| format!("{name} index does not fit u32"))?;
    if value == u32::MAX {
        Err(format!("{name} index u32::MAX is reserved"))
    } else {
        Ok(value)
    }
}

fn ist_day(micros: i64) -> Result<i64, String> {
    Ok(micros
        .checked_add(IST_OFFSET_MICROS)
        .ok_or_else(|| "IST timestamp conversion overflowed i64".to_owned())?
        .div_euclid(86_400_000_000))
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes"
)]
mod tests {
    use super::{
        ADMISSION_EVIDENCE_SOURCE_MATRIX_V1, DataCompletenessSourceV1, EvidenceAvailabilityV1,
        EvidenceSourceV1, FamilyTestEvidenceV1, FullPrecisionStatisticsSourceV1,
        INSTITUTIONAL_EVIDENCE_BLOCKERS_V1, IndependentSessionsEvidenceV1,
        InstitutionalCompletenessV1, InstitutionalEvidenceSourcesV1, ValidationEvidenceV1,
        build_institutional_evidence_v1, checked_weakest_period_return, probability_ppm,
        validation_values,
    };
    use brutex_core::instrument::{Exchange, InstrumentKey};
    use engine::Ladder;
    use indicators::column::Column;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use runner::admission::{
        AdmissionPolicyDraftV1, AdmissionPolicyV1, CompletenessV1, HypothesisDecisionV1,
        ObservedI64V1, ObservedU64V1, PPM,
    };
    use runner::bootstrap::{
        DEFAULT_BLOCK, Verdict, romano_wolf_adjusted_p_values_v1, romano_wolf_receipt,
    };
    use runner::excursion::Side;
    use runner::exit_grid_policy::{
        EvaluatedExitGridV1, ExecutionResolutionV1, ExecutionRunV1, ExecutionSeriesV1,
        ExitGridPolicyV1, ExitGridSelectorV1, ForcedStopV1, RangeResolutionV1, RatioLimitsV1,
        RationalPercentileV1, ResolvedExitGridV1, RungPlanV1, printed_ohlcv_cost_model_id_v1,
    };
    use runner::grid::{Cell, TradeRow};
    use runner::identity::{Direction, Params, Run};
    use runner::outcome::Horizon;
    use runner::pbo::{place, probability_of_overfitting};
    use runner::validate::{FoldResult, Validated};
    use runner::{PopulationRun, Sweeper};

    use crate::institutional_statistics::{
        InstitutionalStatisticsLedgerV1, prepare_institutional_statistics_v1,
    };
    use crate::population::{CompletionReconciliationV2, ExitCellsPerMaskV2, ExitCoordinateV1};
    use crate::population_admission_writer::PopulationCellContextV1;

    const TEST_FEED: &str = "institutional-evidence-test-feed";
    const TEST_COMMIT: &str = "institutional-evidence-test-commit";
    const TEST_CALENDAR_POLICY: [u8; 32] = [0xA5; 32];

    fn digest(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn evaluator() -> Evaluator {
        Evaluator::new(
            Widths::pinned().expect("pinned widths are valid"),
            Availability::Absent,
            Thresholds::CLASSICAL,
        )
    }

    fn exit_policy(side: Side) -> ExitGridPolicyV1 {
        let percentile = RationalPercentileV1::new(1, 2).expect("one-half percentile");
        ExitGridPolicyV1::new(
            ExecutionResolutionV1::OneMinuteOhlcv,
            RangeResolutionV1::PpmCeiling,
            side,
            RungPlanV1::new(vec![percentile], vec![percentile], vec![percentile], 1)
                .expect("one exact rung per axis"),
            RatioLimitsV1::new(1, 10_000, 1).expect("wide exact ratio interval"),
            1_000,
            ExitGridSelectorV1::GuaranteedFloor,
            printed_ohlcv_cost_model_id_v1(),
            ForcedStopV1::Disabled,
            u64::MAX,
            u64::MAX,
        )
        .expect("complete side-specific exit policy")
    }

    fn evaluated_grid(
        resolved: &ResolvedExitGridV1,
        series: ExecutionSeriesV1<'_>,
        column: &Column,
        bars: &[indicators::Candle],
        mask_words: [u64; 6],
        direction: Direction,
    ) -> EvaluatedExitGridV1 {
        let mask = runner::replay_mask::from_stored_words(mask_words)
            .expect("fixture mask is canonical and live");
        let ladder = Ladder::with_min_hits(600).with_ceiling(50_000);
        let run = Run {
            mask,
            direction,
            instrument: series.instrument(),
            timeframe: "1min",
            params: Params::of(ladder),
            data_digest: runner::identity::data_digest(bars),
            commit: TEST_COMMIT,
            feed: TEST_FEED,
        };
        let execution_run =
            ExecutionRunV1::new(&run, bars, None).expect("fixture execution identity");
        resolved
            .evaluate_training_grid_attested(
                series,
                column,
                Horizon::bars(2).expect("two-bar horizon"),
                execution_run,
            )
            .expect("complete evaluated grid fixture")
    }

    fn exit_coordinate(cell: &Cell) -> ExitCoordinateV1 {
        let index = |value: usize| u32::try_from(value).expect("fixture exit index fits u32");
        ExitCoordinateV1 {
            stop: cell.stop.map(index),
            target: cell.target.map(index),
            tsl: cell.tsl.map(index),
            ttp: cell.ttp.map(|value| (index(value.arm), index(value.trail))),
        }
    }

    fn reconciliation(
        population: &PopulationRun,
        long: &ResolvedExitGridV1,
        short: &ResolvedExitGridV1,
    ) -> CompletionReconciliationV2 {
        CompletionReconciliationV2 {
            sweep_trials: population.outcome.trials,
            frequent_itemsets: population.considered,
            infrequent_itemsets: population
                .outcome
                .trials
                .checked_sub(population.considered)
                .expect("fixture frequent count is bounded by trials"),
            closed_itemsets: population.closed,
            redundant_itemsets: population.redundant,
            unknown_closure_itemsets: 0,
            exit_cells_per_mask: ExitCellsPerMaskV2::new(long.cell_count(), short.cell_count())
                .expect("both fixture resolutions contain cells"),
            extinction_depth: u32::try_from(population.outcome.sweep.depth())
                .expect("fixture extinction depth fits u32"),
            extinction_complete: population.outcome.sweep.completed(),
            closure_complete: population.outcome.closure_complete,
        }
    }

    fn missing_sources<'a>(
        context: &PopulationCellContextV1<'a>,
    ) -> InstitutionalEvidenceSourcesV1<'a> {
        InstitutionalEvidenceSourcesV1 {
            context: *context,
            ranking_policy_digest: digest(3),
            trade_rows: EvidenceSourceV1::Unmeasured,
            independent_sessions: EvidenceSourceV1::Unmeasured,
            validation: EvidenceSourceV1::Unmeasured,
            family_tests: EvidenceSourceV1::Unmeasured,
            completeness: InstitutionalCompletenessV1 {
                population_authority: EvidenceSourceV1::Unmeasured,
                data: DataCompletenessSourceV1::Unmeasured,
            },
            full_precision_statistics: FullPrecisionStatisticsSourceV1::Unmeasured,
        }
    }

    fn relaxed_policy() -> AdmissionPolicyV1 {
        AdmissionPolicyV1::new(AdmissionPolicyDraftV1 {
            min_support_hits: Some(1),
            min_independent_sessions: Some(1),
            min_trades: Some(1),
            max_mae_paisa: Some(u64::MAX),
            min_worst_reward_risk_ppm: Some(0),
            min_win_rate_ppm: Some(0),
            min_wilson_win_rate_ppm: Some(0),
            min_return_drawdown_ppm: Some(0),
            min_weakest_period_return_paisa: Some(i64::MIN),
            max_pbo_ppm: Some(PPM),
            max_fwer_p_value_ppm: Some(PPM),
            max_spa_p_value_ppm: Some(PPM),
            min_decided_folds: Some(1),
            max_ambiguous_fill_rate_ppm: Some(PPM),
            max_gap_affected_rate_ppm: Some(PPM),
            max_session_concentration_ppm: Some(PPM),
            max_largest_trade_profit_share_ppm: Some(PPM),
            max_drawdown_paisa: Some(u64::MAX),
            max_worst_trade_loss_paisa: Some(u64::MAX),
            max_losing_trade_rate_ppm: Some(PPM),
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
            max_white_reality_p_value_ppm: Some(PPM),
            max_romano_wolf_p_value_ppm: Some(PPM),
            require_white_reality_rejection: Some(false),
            require_romano_wolf_rejection: Some(false),
        })
        .expect("fully explicit relaxed policy")
    }

    fn with_evidence_fixture(
        test: impl for<'a> FnOnce(
            PopulationCellContextV1<'a>,
            &'a ResolvedExitGridV1,
            &'a EvaluatedExitGridV1,
            &'a Cell,
        ),
    ) {
        let bars = runner::synthetic::sessions(8);
        let instrument =
            InstrumentKey::index(Exchange::Nse, "NIFTY").expect("swept spot-index fixture");
        let execution_column = Column::build(&bars, &mut evaluator());
        let population_column = Column::build(&bars, &mut evaluator());
        let population = Sweeper::new(Ladder::with_min_hits(600).with_ceiling(50_000))
            .run_prepared_population_by_reporting(population_column, &|_, _, _| {}, |_| {
                Ok::<(), &'static str>(())
            })
            .expect("in-memory population sink cannot refuse");
        let series = ExecutionSeriesV1::new(
            &instrument,
            TEST_FEED,
            TEST_COMMIT,
            TEST_CALENDAR_POLICY,
            &bars,
        )
        .expect("attested execution fixture");
        let long = exit_policy(Side::Long)
            .resolve_attested(series)
            .expect("long resolution fixture");
        let short = exit_policy(Side::Short)
            .resolve_attested(series)
            .expect("short resolution fixture");
        let evaluated = evaluated_grid(
            &long,
            series,
            &execution_column,
            &bars,
            [1, 0, 0, 0, 0, 0],
            Direction::Long,
        );
        let foreign_evaluated = evaluated_grid(
            &long,
            series,
            &execution_column,
            &bars,
            [2, 0, 0, 0, 0, 0],
            Direction::Long,
        );
        let validated = long
            .validate_evaluation(&evaluated)
            .expect("complete evaluation fixture");
        let cell = evaluated
            .grid()
            .cells
            .first()
            .expect("fixture grid has a cell");
        let foreign_cell = foreign_evaluated
            .grid()
            .cells
            .first()
            .expect("foreign fixture grid has a cell");
        let context = PopulationCellContextV1 {
            population_id: digest(1),
            strategy_digest: digest(2),
            mask_words: [1, 0, 0, 0, 0, 0],
            support_hits: 1,
            population_run: &population,
            reconciliation: reconciliation(&population, &long, &short),
            direction: crate::population::TradeDirectionV1::Long,
            exit: exit_coordinate(cell),
            cell,
            cell_ordinal: 0,
            resolved: &long,
            evaluated: &evaluated,
            validated: &validated,
        };
        test(context, &short, &foreign_evaluated, foreign_cell);
    }

    #[test]
    fn public_builder_keeps_absent_admission_authorities_fail_closed() {
        with_evidence_fixture(|context, _, _, _| {
            let evidence = build_institutional_evidence_v1(missing_sources(&context))
                .expect("missing evidence is explicit rather than malformed");
            let values = evidence.values();
            assert_eq!(values.data_complete, CompletenessV1::Unmeasured);
            assert_eq!(
                values.full_precision_statistics_complete,
                CompletenessV1::Unmeasured
            );
            assert_eq!(values.fwer_p_value_ppm, ObservedU64V1::Unmeasured);
            assert_eq!(values.romano_wolf_p_value_ppm, ObservedU64V1::Unmeasured);
            assert_eq!(
                values.romano_wolf_decision,
                HypothesisDecisionV1::Unmeasured
            );
            assert!(
                !relaxed_policy().evaluate(&evidence).is_admitted(),
                "even a numerically relaxed policy must not admit absent authorities"
            );
        });
    }

    #[test]
    fn public_builder_refuses_foreign_population_strategy_cell_and_evaluation_authorities() {
        with_evidence_fixture(|context, foreign_resolution, _, foreign_cell| {
            let mut foreign_population = missing_sources(&context);
            foreign_population.independent_sessions =
                EvidenceSourceV1::Measured(IndependentSessionsEvidenceV1 {
                    population_id: digest(9),
                    strategy_digest: context.strategy_digest,
                    sessions: 1,
                });
            let why = build_institutional_evidence_v1(foreign_population)
                .expect_err("foreign population evidence must refuse");
            assert!(why.contains("another population or strategy"));

            let mut foreign_strategy = missing_sources(&context);
            foreign_strategy.independent_sessions =
                EvidenceSourceV1::Measured(IndependentSessionsEvidenceV1 {
                    population_id: context.population_id,
                    strategy_digest: digest(9),
                    sessions: 1,
                });
            let why = build_institutional_evidence_v1(foreign_strategy)
                .expect_err("foreign strategy evidence must refuse");
            assert!(why.contains("another population or strategy"));

            let mut cell_context = context;
            cell_context.cell = foreign_cell;
            cell_context.exit = exit_coordinate(foreign_cell);
            let why = build_institutional_evidence_v1(missing_sources(&cell_context))
                .expect_err("a cell from another evaluation must refuse");
            assert!(why.contains("cell is not owned by its evaluated grid"));

            let mut evaluation_context = context;
            evaluation_context.resolved = foreign_resolution;
            let why = build_institutional_evidence_v1(missing_sources(&evaluation_context))
                .expect_err("a foreign evaluation authority must refuse");
            assert!(why.contains("torn evaluation"));
        });
    }

    #[test]
    fn missing_family_parts_are_unmeasured_and_never_zero() {
        let source =
            FamilyTestEvidenceV1::from_runner(digest(1), digest(2), digest(3), 0, None, None, None)
                .expect("missing evidence is a state, not a malformed input");
        assert_eq!(source, EvidenceSourceV1::Unmeasured);
    }

    fn family_returns() -> Vec<Vec<i64>> {
        vec![
            (0..300)
                .map(|period| if period % 2 == 0 { -1 } else { 1 })
                .collect::<Vec<_>>(),
            (0..300)
                .map(|period| if period % 2 == 0 { 99 } else { 101 })
                .collect::<Vec<_>>(),
            (0..300)
                .map(|period| if period % 2 == 0 { -2 } else { 2 })
                .collect::<Vec<_>>(),
        ]
    }

    fn family_verdicts() -> (Verdict, Verdict) {
        let white = Verdict {
            statistic: 4.0,
            p_value: 0.01,
            draws: 100,
            strategies: 3,
            periods: 300,
        };
        let spa = Verdict {
            p_value: 0.02,
            ..white
        };
        (white, spa)
    }

    #[test]
    fn family_receipt_reconciles_rejected_and_non_rejected_candidates() {
        let family = family_returns();
        let romano_wolf = romano_wolf_receipt(&family, 100, 7, DEFAULT_BLOCK, 50_000)
            .expect("aligned family has a complete Romano-Wolf receipt");
        let (white, spa) = family_verdicts();
        let measured = FamilyTestEvidenceV1::from_runner(
            digest(1),
            digest(2),
            digest(3),
            1,
            Some(&white),
            Some(&spa),
            Some(&romano_wolf),
        )
        .expect("one reconciled family");
        let EvidenceSourceV1::Measured(measured) = measured else {
            unreachable!("reconciled family must be measured");
        };
        assert_eq!(
            measured.romano_wolf_decision,
            HypothesisDecisionV1::RejectedNull
        );
        assert_eq!(measured.bootstrap_draws, ObservedU64V1::Measured(100));
        assert_eq!(measured.bootstrap_strategies, ObservedU64V1::Measured(3));
        assert_eq!(measured.bootstrap_periods, ObservedU64V1::Measured(300));

        let not_named = FamilyTestEvidenceV1::from_runner(
            digest(1),
            digest(2),
            digest(3),
            0,
            Some(&white),
            Some(&spa),
            Some(&romano_wolf),
        )
        .expect("same complete raw family");
        let EvidenceSourceV1::Measured(not_named) = not_named else {
            unreachable!("reconciled family must be measured");
        };
        assert_eq!(
            not_named.romano_wolf_decision,
            HypothesisDecisionV1::DidNotReject
        );
    }

    #[test]
    fn durable_family_authority_is_exact_post_selection_evidence_not_admission_input() {
        with_evidence_fixture(|context, _, _, _| {
            let family = family_returns();
            let adjusted = romano_wolf_adjusted_p_values_v1(&family, 100, 7, DEFAULT_BLOCK)
                .expect("aligned family has exact adjusted probabilities");
            let (white, spa) = family_verdicts();
            let prepared = prepare_institutional_statistics_v1(
                context.population_id,
                digest(4),
                digest(3),
                context.strategy_digest,
                1,
                &white,
                &spa,
                &adjusted,
            )
            .expect("prepare exact institutional family statistics");
            let root = std::env::temp_dir().join(format!(
                "brutex-institutional-evidence-family-{}",
                std::process::id()
            ));
            if root.exists() {
                std::fs::remove_dir_all(&root).expect("clear durable family fixture");
            }
            let mut ledger = InstitutionalStatisticsLedgerV1::open(&root, 2)
                .expect("open durable family ledger");
            let authority = ledger
                .append_complete(&prepared)
                .expect("commit durable family statistics")
                .authority();
            let family_source = FamilyTestEvidenceV1::from_durable_statistics(
                context.population_id,
                digest(4),
                digest(3),
                context.strategy_digest,
                1,
                &white,
                &spa,
                &authority,
            )
            .expect("join durable family statistics");
            let EvidenceSourceV1::Measured(family_evidence) = family_source else {
                unreachable!("durable family must be measured");
            };
            assert_eq!(
                family_evidence.fwer_p_value_ppm(),
                ObservedU64V1::Measured(authority.fwer_p_value_ppm())
            );
            assert_eq!(
                family_evidence.romano_wolf_p_value_ppm(),
                ObservedU64V1::Measured(authority.romano_wolf_p_value_ppm())
            );
            assert_eq!(family_evidence.selection_receipt_digest(), Some(digest(4)));
            assert_eq!(
                family_evidence.durable_statistics_completion_digest(),
                Some(authority.completion_digest())
            );

            let mut sources = missing_sources(&context);
            sources.family_tests = EvidenceSourceV1::Measured(family_evidence);
            let cycle = build_institutional_evidence_v1(sources)
                .expect_err("post-selection evidence must not authorize pre-selection admission");
            assert!(cycle.contains("post-selection evidence"));
            let why = FamilyTestEvidenceV1::from_durable_statistics(
                context.population_id,
                digest(9),
                digest(3),
                context.strategy_digest,
                1,
                &white,
                &spa,
                &authority,
            )
            .expect_err("foreign selection identity must refuse");
            assert!(why.contains("foreign"));
            drop(ledger);
            std::fs::remove_dir_all(&root).expect("remove durable family fixture");
        });
    }

    #[test]
    fn family_receipt_refuses_mismatched_denominators_and_alpha() {
        let family = family_returns();
        let romano_wolf = romano_wolf_receipt(&family, 100, 7, DEFAULT_BLOCK, 50_000)
            .expect("aligned family has a complete Romano-Wolf receipt");
        let (white, spa) = family_verdicts();
        let wrong = Verdict {
            periods: 299,
            ..spa
        };
        let why = FamilyTestEvidenceV1::from_runner(
            digest(1),
            digest(2),
            digest(3),
            1,
            Some(&white),
            Some(&wrong),
            Some(&romano_wolf),
        )
        .expect_err("two denominator sets are not one family");
        assert!(why.contains("different bootstrap families"));

        let wrong_denominator = romano_wolf_receipt(&family, 99, 7, DEFAULT_BLOCK, 50_000)
            .expect("the deliberately different family remains internally complete");
        let why = FamilyTestEvidenceV1::from_runner(
            digest(1),
            digest(2),
            digest(3),
            1,
            Some(&white),
            Some(&spa),
            Some(&wrong_denominator),
        )
        .expect_err("Romano-Wolf denominators must join the same family");
        assert!(why.contains("different bootstrap families"));

        let wrong_alpha = romano_wolf_receipt(&family, 100, 7, DEFAULT_BLOCK, 40_000)
            .expect("a noncanonical alpha still produces an explicit runner receipt");
        let why = FamilyTestEvidenceV1::from_runner(
            digest(1),
            digest(2),
            digest(3),
            1,
            Some(&white),
            Some(&spa),
            Some(&wrong_alpha),
        )
        .expect_err("admission binds the canonical Romano-Wolf alpha");
        assert!(why.contains("differs from canonical"));
    }

    #[test]
    fn supplied_anchored_fold_legacy_diagnostic_must_match_the_walk_forward_fold_set() {
        let fold = FoldResult {
            index: 0,
            considered: 2,
            priced: 2,
            chosen: Some(
                runner::replay_mask::from_stored_words([1, 0, 0, 0, 0, 0])
                    .expect("bit zero is a live canonical condition"),
            ),
            out_of_sample_exit: Some(10),
            in_sample_all: vec![2, 1],
            out_of_sample_all: vec![2, 1],
            ..FoldResult::default()
        };
        let validated = Validated {
            folds: vec![fold],
            refused: None,
        };
        let placements = [
            place(&[2, 1], &[2, 1]).expect("rankable"),
            place(&[2, 1], &[1, 2]).expect("rankable"),
        ];
        let wrong = probability_of_overfitting(&placements);
        let why = ValidationEvidenceV1::from_runner(digest(1), digest(2), &validated, Some(&wrong))
            .expect_err("two legacy diagnostic folds cannot describe one validation fold");
        assert!(why.contains("does not equal the fold-derived diagnostic"));
    }

    #[test]
    fn aligned_legacy_diagnostic_never_authorizes_genuine_pbo_fields() {
        let fold = FoldResult {
            index: 0,
            considered: 2,
            priced: 2,
            chosen: Some(
                runner::replay_mask::from_stored_words([1, 0, 0, 0, 0, 0])
                    .expect("bit zero is a live canonical condition"),
            ),
            out_of_sample_exit: Some(10),
            in_sample_all: vec![2, 1],
            out_of_sample_all: vec![2, 1],
            ..FoldResult::default()
        };
        let validated = Validated {
            folds: vec![fold],
            refused: None,
        };
        let placement = place(&[2, 1], &[2, 1]).expect("aligned legacy placement");
        let supplied = probability_of_overfitting(&[placement]);
        let measured =
            ValidationEvidenceV1::from_runner(digest(1), digest(2), &validated, Some(&supplied))
                .expect("aligned walk-forward evidence remains usable");
        let EvidenceSourceV1::Measured(measured) = measured else {
            unreachable!("one complete decided fold measures walk-forward outcomes");
        };
        let admission =
            validation_values(EvidenceSourceV1::Measured(measured), digest(1), digest(2))
                .expect("aligned validation identity projects into admission");
        assert_eq!(admission.pbo_ppm, ObservedU64V1::Unmeasured);
        assert_eq!(admission.pbo_contributing_folds, ObservedU64V1::Unmeasured);
        assert_eq!(admission.pbo_unrankable_folds, ObservedU64V1::Unmeasured);
        assert_eq!(admission.decided_folds, ObservedU64V1::Measured(1));
        assert_eq!(admission.profitable_oos_folds, ObservedU64V1::Measured(1));
        assert_eq!(
            admission.oos_pessimistic_return_paisa,
            ObservedI64V1::Measured(10)
        );
    }

    #[test]
    fn empty_walk_forward_does_not_invent_pbo_or_outcome_evidence() {
        let validated = Validated {
            folds: Vec::new(),
            refused: None,
        };
        let evidence = ValidationEvidenceV1::from_runner(digest(1), digest(2), &validated, None)
            .expect("empty validation is an explicit non-value state");
        assert!(matches!(evidence, EvidenceSourceV1::Unmeasured));
    }

    #[test]
    fn misaligned_legacy_candidate_arrays_still_refuse_validation_evidence() {
        let validated = Validated {
            folds: vec![FoldResult {
                index: 0,
                considered: 2,
                priced: 2,
                chosen: Some(
                    runner::replay_mask::from_stored_words([1, 0, 0, 0, 0, 0])
                        .expect("bit zero is a live canonical condition"),
                ),
                out_of_sample_exit: Some(10),
                in_sample_all: vec![2, 1],
                out_of_sample_all: vec![2],
                ..FoldResult::default()
            }],
            refused: None,
        };
        let why = ValidationEvidenceV1::from_runner(digest(1), digest(2), &validated, None)
            .expect_err("misaligned candidate arrays cannot become admission evidence");
        assert!(why.contains("misaligned in/out-of-sample candidate families"));
    }

    #[test]
    fn invalid_probabilities_are_refused_instead_of_clamped() {
        assert!(probability_ppm("test", f64::NAN).is_err());
        assert!(probability_ppm("test", 1.000_001).is_err());
        assert_eq!(probability_ppm("test", 0.05).expect("in domain"), 50_000);
    }

    #[test]
    fn singleton_legacy_fold_never_becomes_measured_pbo_admission_evidence() {
        let validated = Validated {
            folds: vec![FoldResult {
                index: 0,
                considered: 1,
                priced: 1,
                chosen: Some(
                    runner::replay_mask::from_stored_words([1, 0, 0, 0, 0, 0])
                        .expect("bit zero is a live canonical condition"),
                ),
                out_of_sample_exit: Some(10),
                in_sample_all: vec![2],
                out_of_sample_all: vec![1],
                ..FoldResult::default()
            }],
            refused: None,
        };
        let measured = ValidationEvidenceV1::from_runner(digest(1), digest(2), &validated, None)
            .expect("one singleton fold is explicit unrankable evidence");
        let EvidenceSourceV1::Measured(measured) = measured else {
            unreachable!("validation ran and classified its fold");
        };
        let admission =
            validation_values(EvidenceSourceV1::Measured(measured), digest(1), digest(2))
                .expect("singleton validation identity projects into admission");
        assert_eq!(admission.pbo_ppm, ObservedU64V1::Unmeasured);
        assert_eq!(admission.pbo_contributing_folds, ObservedU64V1::Unmeasured);
        assert_eq!(admission.pbo_unrankable_folds, ObservedU64V1::Unmeasured);
        assert_eq!(admission.decided_folds, ObservedU64V1::Measured(1));
        assert_eq!(admission.profitable_oos_folds, ObservedU64V1::Measured(1));
        assert_eq!(
            admission.oos_pessimistic_return_paisa,
            ObservedI64V1::Measured(10)
        );
    }

    #[test]
    fn weakest_period_refuses_bucket_overflow_instead_of_saturating() {
        let row = |worst| TradeRow {
            signal_bar: 0,
            entry_bar: 1,
            exit_bar: 2,
            best: worst,
            worst,
            entry_micros: 1_800_000_000,
            exit_micros: 1_860_000_000,
            adverse: 0,
            adverse_paisa: 0,
            favourable: 0,
            favourable_paisa: 0,
        };
        let why = checked_weakest_period_return(&[row(i64::MAX), row(1)])
            .expect_err("same-period P&L overflow cannot be a measured weakest period");
        assert!(why.contains("weakest-period return overflowed"));
    }

    #[test]
    fn source_and_blocker_matrices_are_complete_unique_and_renderable() {
        let source_fields = ADMISSION_EVIDENCE_SOURCE_MATRIX_V1
            .iter()
            .map(|row| row.field)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ADMISSION_EVIDENCE_SOURCE_MATRIX_V1.len(), 44);
        assert_eq!(source_fields.len(), 44);
        assert!(
            ADMISSION_EVIDENCE_SOURCE_MATRIX_V1
                .iter()
                .all(|row| { !row.field.is_empty() && !row.source.is_empty() })
        );
        for field in ["pbo_ppm", "pbo_contributing_folds", "pbo_unrankable_folds"] {
            let row = ADMISSION_EVIDENCE_SOURCE_MATRIX_V1
                .iter()
                .find(|row| row.field == field)
                .expect("every genuine PBO admission field has an explicit source-map row");
            assert_eq!(row.availability, EvidenceAvailabilityV1::Absent);
            assert!(row.source.contains("CSCV/PBO"));
        }

        let blocker_surfaces = INSTITUTIONAL_EVIDENCE_BLOCKERS_V1
            .iter()
            .map(|row| row.surface)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(INSTITUTIONAL_EVIDENCE_BLOCKERS_V1.len(), 7);
        assert_eq!(blocker_surfaces.len(), 7);
        assert!(blocker_surfaces.contains("genuine CSCV/PBO"));
        assert!(INSTITUTIONAL_EVIDENCE_BLOCKERS_V1.iter().all(|row| {
            !row.surface.is_empty()
                && !row.current_gap.is_empty()
                && !row.required_artifact.is_empty()
        }));
    }
}
