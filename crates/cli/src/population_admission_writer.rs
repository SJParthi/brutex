//! Production commit boundary for Population V4 and canonical admission.
//!
//! This module derives population and strategy identities from complete,
//! explicitly sourced authority.  It does not manufacture a calendar receipt,
//! statistical observation or policy threshold.  It validates every
//! representable relationship, recomputes every admission verdict, prepares
//! every byte before opening a writer, then commits the Population V4 receipt
//! before the admission receipt.
//!
//! The population and admission ledgers both serialize on
//! `results/population-write.lock`.  Their public APIs acquire that same lock
//! independently.  Consequently a crash between the two commits can expose a
//! complete Population V4 prefix, but it cannot expose joined admission
//! authority: [`crate::admission_join::AdmissionAuthoritativeLedger`] refuses
//! until the second receipt exists.  An exact rerun completes or byte-verifies
//! that prepared prefix.
//!
//! # Cost
//!
//! Preparation and commit are O(rows) time and O(rows) transient memory.
//! Opening either append-only ledger additionally scans its durable authority.
//! This path makes no O(1) population, persistence or hashing claim.

use std::path::Path;

use brutex_core::blake3::Hasher;
use indicators::column::Column;
use pull::session::Day;
use runner::admission::{
    AdmissionEvidenceV1, AdmissionPolicyV1, AdmissionStatusV1 as RunnerAdmissionStatusV1,
    CompletenessV1, ObservedI64V1, ObservedU64V1,
};
use runner::exit_grid_policy::{
    EvaluatedExitGridV1, ExecutionDispositionV1, ExecutionRunV1, ExecutionSeriesV1,
    InstrumentFamilyV1 as RunnerInstrumentFamilyV1, ResolvedExitGridV1, ValidatedExitGridV1,
};
use runner::grid::{Cell, Chosen};
use runner::outcome::Horizon;
use runner::{ClosureVerdict, PopulationMember, PopulationRun, Sweeper};

use crate::admission_store::{
    AdmissionAuthorityLedger, AdmissionCommitOutcomeV1, AdmissionDecisionRecordV1,
};
use crate::population::{
    AdmissionStatusV1, AdmissionV1, ClosureV1, CompletionReceiptV4, CompletionReconciliationV2,
    ExitCellsPerMaskV2, ExitCoordinateV1, InstrumentFamilyV1, PopulationCommit,
    PopulationIdentitiesV2, PopulationLedger, PopulationRowV1, RequestedSpanIdentityV1,
    TopMetricsV1, TradeDirectionV1,
};
use crate::stored::CompleteCalendarReceiptV2;

const POPULATION_ID_DOMAIN_V1: &[u8] = b"brutex-complete-population-id-v1\0";
const STRATEGY_ID_DOMAIN_V1: &[u8] = b"brutex-complete-strategy-id-v1\0";
const POPULATION_ID_VERSION_V1: u32 = 1;
const STRATEGY_ID_VERSION_V1: u32 = 1;
const PPM: u64 = 1_000_000;

/// Operator-facing refusal from the Population V4 plus admission writer.
pub type PopulationAdmissionWriterRefusal = String;

/// One already evaluated strategy/exit cell offered for durable authority.
///
/// Every field is mandatory.  The writer does not derive a strategy digest,
/// replace an absent measurement with zero or copy a long-side value into the
/// short side.  The upstream adapter must preserve the exact canonical order
/// of the complete expanded population.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvaluatedPopulationCandidateV1 {
    /// Digest created by the upstream canonical strategy-identity authority.
    pub strategy_digest: [u8; 32],
    /// Exact six-word append-only condition mask.
    pub mask_words: [u64; 6],
    /// Explicit long or short expansion.
    pub direction: TradeDirectionV1,
    /// Exact closure classification from the uncapped sweep.
    pub closure: ClosureV1,
    /// Exact support count measured by the sweep.
    pub support_hits: u64,
    /// Exact coordinate from the evaluated side-specific exit grid.
    pub exit: ExitCoordinateV1,
    /// Complete final-ranking metric projection for this exact cell.
    pub metrics: TopMetricsV1,
    /// Validated institutional evidence for this exact cell.
    pub evidence: AdmissionEvidenceV1,
}

/// Complete sourced request for one Population V4 plus admission commit.
///
/// There is intentionally no `Default` implementation.  Each identity,
/// reconciliation fact, calendar proof and policy must come from its owning
/// upstream authority.
#[derive(Clone, Copy, Debug)]
pub struct PopulationAdmissionWriteRequestV1<'a> {
    /// Identity shared by every row and both completion receipts.
    pub population_id: [u8; 32],
    /// NIFTY or BANKNIFTY population family.
    pub instrument_family: InstrumentFamilyV1,
    /// One of the eight exact signal-rung durations.
    pub rung_seconds: u32,
    /// Exact requested civil-month span.
    pub requested_span: RequestedSpanIdentityV1,
    /// Upstream sweep/grid population reconciliation.
    pub reconciliation: CompletionReconciliationV2,
    /// Every replay and policy identity required by Population V4.
    pub identities: PopulationIdentitiesV2,
    /// Complete signal-rung IST calendar receipt.
    pub signal_calendar: CompleteCalendarReceiptV2,
    /// Independent complete one-minute execution calendar receipt.
    pub execution_calendar: CompleteCalendarReceiptV2,
    /// Fully explicit canonical institutional policy.
    pub admission_policy: AdmissionPolicyV1,
    /// Complete canonical mask/direction/exit-cell sequence.
    pub candidates: &'a [EvaluatedPopulationCandidateV1],
}

/// Fully validated bytes prepared before any durable file is opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedPopulationAdmissionV1 {
    rows: Vec<PopulationRowV1>,
    decisions: Vec<AdmissionDecisionRecordV1>,
    receipt: CompletionReceiptV4,
    policy: AdmissionPolicyV1,
}

impl PreparedPopulationAdmissionV1 {
    /// Exact Population V4 rows in canonical sequence order.
    #[must_use]
    pub fn rows(&self) -> &[PopulationRowV1] {
        &self.rows
    }

    /// Exact recomputable admission decisions matching [`Self::rows`].
    #[must_use]
    pub fn decisions(&self) -> &[AdmissionDecisionRecordV1] {
        &self.decisions
    }

    /// Calendar-authoritative receipt appended after the population rows.
    #[must_use]
    pub const fn receipt(&self) -> CompletionReceiptV4 {
        self.receipt
    }

    /// Canonical policy used to recompute every prepared verdict.
    #[must_use]
    pub const fn policy(&self) -> AdmissionPolicyV1 {
        self.policy
    }
}

/// Every already-authoritative input shared by one exact instrument/rung
/// population.
///
/// This value has no default.  In particular, the two side-specific resolved
/// grids, the exact one-minute execution series, both complete calendar
/// receipts and every append-only policy identity must be supplied by their
/// owning upstream authority.
#[derive(Clone, Copy, Debug)]
pub struct CompletePopulationAuthorityV1<'a> {
    /// NIFTY or BANKNIFTY population family.
    pub instrument_family: InstrumentFamilyV1,
    /// One of the eight exact signal-rung durations.
    pub rung_seconds: u32,
    /// Exact execution horizon applied to every grid cell.
    pub horizon: Horizon,
    /// Exact requested civil-month span.
    pub requested_span: RequestedSpanIdentityV1,
    /// Every replay and policy identity required by Population V4.
    pub identities: PopulationIdentitiesV2,
    /// Complete signal-rung IST calendar receipt.
    pub signal_calendar: CompleteCalendarReceiptV2,
    /// Independent complete one-minute execution calendar receipt.
    pub execution_calendar: CompleteCalendarReceiptV2,
    /// Fully explicit canonical institutional policy.
    pub admission_policy: AdmissionPolicyV1,
    /// Exact long-side TRAINING resolution.
    pub long_exit_grid: &'a ResolvedExitGridV1,
    /// Exact short-side TRAINING resolution.
    pub short_exit_grid: &'a ResolvedExitGridV1,
    /// Exact attested one-minute slice priced by both sides.
    pub execution_series: ExecutionSeriesV1<'a>,
    /// Exact one-minute evaluation column paired with `execution_series`.
    pub execution_column: &'a Column,
}

/// Context handed to the mandatory per-cell statistical-evidence authority.
///
/// The adapter derives the cell's mask, side, coordinate, direct ranking
/// metrics and identities.  It cannot derive family-wise bootstrap,
/// walk-forward or full-precision hypothesis-test evidence from one cell, so
/// that evidence is supplied explicitly and validated by
/// [`prepare_population_admission_v1`].
#[derive(Clone, Copy, Debug)]
pub struct PopulationCellContextV1<'a> {
    /// Identity shared by this population's rows and receipts.
    pub population_id: [u8; 32],
    /// Complete semantic strategy identity derived by this adapter.
    pub strategy_digest: [u8; 32],
    /// Exact six-word condition combination.
    pub mask_words: [u64; 6],
    /// Exact itemset support measured by the uncapped sweep.
    pub support_hits: u64,
    /// Complete uncapped walk, exposed only after extinction and closure have
    /// been proved.
    pub population_run: &'a PopulationRun,
    /// Exact checked population/grid reconciliation for this walk.
    pub reconciliation: CompletionReconciliationV2,
    /// Explicit long or short side.
    pub direction: TradeDirectionV1,
    /// Exact canonical exit coordinate.
    pub exit: ExitCoordinateV1,
    /// Read-only complete measured cell.
    pub cell: &'a Cell,
    /// Canonical ordinal of `cell` in the complete evaluated grid.
    pub cell_ordinal: usize,
    /// Resolution that produced `evaluated`.
    pub resolved: &'a ResolvedExitGridV1,
    /// Opaque complete evaluation capability for this mask and side.
    pub evaluated: &'a EvaluatedExitGridV1,
    /// Single complete-grid integrity capability reused by every cell.
    pub validated: &'a ValidatedExitGridV1<'a>,
}

/// Statistical authority returned for one completely evaluated exit cell.
///
/// `assurance_ppm` is separate because [`Cell`] exposes the Wilson floor only
/// in basis points.  The adapter refuses a ppm value outside that exact
/// basis-point bucket; it never multiplies the coarser projection and calls it
/// a measurement.  Every other [`TopMetricsV1`] field is derived directly from
/// the complete cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationCellEvidenceV1 {
    /// Full ppm Wilson lower-bound projection owned by the statistics layer.
    pub assurance_ppm: u64,
    /// Complete institutional evidence, including family and OOS tests.
    pub admission: AdmissionEvidenceV1,
}

/// One finished uncapped population together with its prepared durable rows.
///
/// Nothing is committed by this value.  [`Self::prepared`] exposes the exact
/// rows/decisions/receipt for the existing receipt-last writer, while
/// [`Self::population_run`] retains the engine's extinction and closure proof.
#[derive(Debug)]
pub struct ProducedPopulationAdmissionV1 {
    population_id: [u8; 32],
    reconciliation: CompletionReconciliationV2,
    population_run: PopulationRun,
    prepared: PreparedPopulationAdmissionV1,
}

#[derive(Debug)]
struct EvaluatedPopulationSideV1 {
    mask_words: [u64; 6],
    support_hits: u64,
    direction: TradeDirectionV1,
    evaluated: EvaluatedExitGridV1,
}

impl ProducedPopulationAdmissionV1 {
    /// Canonical identity shared by every produced row.
    #[must_use]
    pub const fn population_id(&self) -> [u8; 32] {
        self.population_id
    }

    /// Exact extinction/grid reconciliation supplied to Population V4.
    #[must_use]
    pub const fn reconciliation(&self) -> CompletionReconciliationV2 {
        self.reconciliation
    }

    /// Uncapped engine result that proves enumeration and closure completeness.
    #[must_use]
    pub fn population_run(&self) -> &PopulationRun {
        &self.population_run
    }

    /// Fully validated Population V4 rows and canonical admission decisions.
    #[must_use]
    pub fn prepared(&self) -> &PreparedPopulationAdmissionV1 {
        &self.prepared
    }

    /// Moves out the durable preparation after callers have recorded any run
    /// diagnostics they require.
    #[must_use]
    pub fn into_prepared(self) -> PreparedPopulationAdmissionV1 {
        self.prepared
    }
}

/// Derives the canonical identity for one exact complete population.
///
/// The preimage is versioned and domain-separated.  It binds the instrument,
/// signal rung, horizon, requested span, every Population V4 policy/source
/// identity, both side-specific resolved grids, and both complete calendar
/// receipts.  It deliberately excludes sweep results and row bytes so it can
/// be known before the uncapped stream starts and cannot form an identity
/// cycle with per-row strategy digests.
///
/// # Errors
///
/// Refuses absent identities, a copied/swapped/foreign side resolution, a
/// resolution not derived from the supplied one-minute series identity, or a
/// calendar/rung mismatch.
pub fn derive_population_id_v1(
    authority: &CompletePopulationAuthorityV1<'_>,
) -> Result<[u8; 32], PopulationAdmissionWriterRefusal> {
    validate_complete_population_authority(authority)?;
    let mut hasher = Hasher::new();
    hasher.update(POPULATION_ID_DOMAIN_V1);
    hasher.update(&POPULATION_ID_VERSION_V1.to_le_bytes());
    hasher.update(&[instrument_family_byte(authority.instrument_family)]);
    hasher.update(&authority.rung_seconds.to_le_bytes());
    hasher.update(&authority.horizon.as_bars().to_le_bytes());
    hasher.update(&authority.requested_span.canonical_bytes());
    hash_population_identities(&mut hasher, &authority.identities);
    hash_calendar_receipt(&mut hasher, authority.signal_calendar);
    hash_calendar_receipt(&mut hasher, authority.execution_calendar);
    let identity = hasher.finalize();
    require_nonzero_digest("derived population identity", &identity)?;
    Ok(identity)
}

/// Derives one complete semantic strategy identity.
///
/// The versioned preimage explicitly binds population, instrument, timeframe,
/// evaluation policy, directional run identity, mask, horizon, exact exit
/// resolution and coordinate.  Metrics and admission verdicts are results, not
/// strategy semantics, so neither enters the digest.
///
/// # Errors
///
/// Refuses an absent identity, a torn/foreign evaluation, a side mismatch, an
/// empty or non-live mask, or an exit index that cannot be represented by the
/// append-only Population V1 coordinate.
#[expect(
    clippy::too_many_arguments,
    reason = "each argument is an independently validated strategy-identity term and no term may be defaulted"
)]
pub fn derive_strategy_digest_v1(
    population_id: [u8; 32],
    instrument_family: InstrumentFamilyV1,
    rung_seconds: u32,
    evaluation_policy_digest: [u8; 32],
    direction: TradeDirectionV1,
    resolved: &ResolvedExitGridV1,
    evaluated: &EvaluatedExitGridV1,
    cell_ordinal: usize,
) -> Result<[u8; 32], PopulationAdmissionWriterRefusal> {
    let validated = resolved.validate_evaluation(evaluated).map_err(|why| {
        format!("strategy identity received a torn exit-grid evaluation: {why:?}")
    })?;
    derive_strategy_digest_from_validated_v1(
        population_id,
        instrument_family,
        rung_seconds,
        evaluation_policy_digest,
        direction,
        resolved,
        evaluated,
        &validated,
        cell_ordinal,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "the validated fast path preserves every independently sourced strategy-identity term"
)]
pub(crate) fn derive_strategy_digest_from_validated_v1(
    population_id: [u8; 32],
    instrument_family: InstrumentFamilyV1,
    rung_seconds: u32,
    evaluation_policy_digest: [u8; 32],
    direction: TradeDirectionV1,
    resolved: &ResolvedExitGridV1,
    evaluated: &EvaluatedExitGridV1,
    validated: &ValidatedExitGridV1<'_>,
    cell_ordinal: usize,
) -> Result<[u8; 32], PopulationAdmissionWriterRefusal> {
    require_nonzero_digest("population identity", &population_id)?;
    require_nonzero_digest("evaluation policy identity", &evaluation_policy_digest)?;
    validate_rung_seconds(rung_seconds)?;
    if validated.resolution_digest() != resolved.digest() || !validated.is_evaluation(evaluated) {
        return Err(
            "strategy identity validation capability belongs to another resolution or evaluation"
                .to_owned(),
        );
    }
    require_family_and_side(instrument_family, direction, resolved)?;
    if evaluated.resolution_digest() != resolved.digest() || evaluated.side() != resolved.side() {
        return Err(
            "strategy identity received an evaluation from another resolution or side".to_owned(),
        );
    }
    let mask = evaluated.mask();
    let mask_words = mask.words();
    if mask_words.iter().all(|word| *word == 0) {
        return Err("strategy identity cannot bind an empty condition mask".to_owned());
    }
    runner::replay_mask::from_stored_words(mask_words)
        .map_err(|why| format!("strategy identity mask is not canonical/live: {why}"))?;
    let cell = validated.cell(cell_ordinal).ok_or_else(|| {
        format!(
            "strategy identity exit-cell ordinal {cell_ordinal} is outside complete grid width {}",
            evaluated.grid().cells.len()
        )
    })?;
    let exit = exit_coordinate(cell)?;

    derive_strategy_digest_from_terms_v1(
        population_id,
        instrument_family,
        rung_seconds,
        evaluation_policy_digest,
        direction,
        evaluated.run_id().bytes(),
        mask_words,
        evaluated.horizon().as_bars(),
        resolved.policy_digest(),
        resolved.digest(),
        exit,
    )
}

/// Rebuilds the exact population strategy identity from an opaque execution
/// disposition.
///
/// This is the cross-module proof that the run sealed into an authorized or
/// policy-refused disposition is the same run that originally formed the
/// Population V4 row. Comparing only mask, side and coordinate would allow a
/// classification from another data/feed/commit run to borrow the row.
///
/// # Errors
///
/// Refuses an empty/retired mask, changed resolution or coordinate, malformed
/// identity input, or any value that cannot use the canonical V1 encoding.
#[expect(
    clippy::too_many_arguments,
    reason = "the durable strategy identity has independent population, policy, side and execution terms; none may be inferred"
)]
pub(crate) fn derive_strategy_digest_from_disposition_v1(
    population_id: [u8; 32],
    instrument_family: InstrumentFamilyV1,
    rung_seconds: u32,
    evaluation_policy_digest: [u8; 32],
    direction: TradeDirectionV1,
    exit_grid_policy_digest: [u8; 32],
    resolved_exit_grid_digest: [u8; 32],
    exit: ExitCoordinateV1,
    disposition: &ExecutionDispositionV1,
) -> Result<[u8; 32], PopulationAdmissionWriterRefusal> {
    require_nonzero_digest("population identity", &population_id)?;
    require_nonzero_digest("evaluation policy identity", &evaluation_policy_digest)?;
    require_nonzero_digest("exit-grid policy identity", &exit_grid_policy_digest)?;
    require_nonzero_digest("resolved exit-grid identity", &resolved_exit_grid_digest)?;
    validate_rung_seconds(rung_seconds)?;
    let disposition_exit = exit_coordinate_from_chosen(disposition.coordinate())?;
    if disposition.resolution_digest() != resolved_exit_grid_digest || disposition_exit != exit {
        return Err(
            "execution disposition resolution/coordinate differs from Population V4 strategy"
                .to_owned(),
        );
    }
    let mask_words = disposition.mask().words();
    if mask_words.iter().all(|word| *word == 0) {
        return Err("strategy identity cannot bind an empty condition mask".to_owned());
    }
    runner::replay_mask::from_stored_words(mask_words)
        .map_err(|why| format!("strategy identity mask is not canonical/live: {why}"))?;
    derive_strategy_digest_from_terms_v1(
        population_id,
        instrument_family,
        rung_seconds,
        evaluation_policy_digest,
        direction,
        disposition.run_id().bytes(),
        mask_words,
        disposition.horizon().as_bars(),
        exit_grid_policy_digest,
        resolved_exit_grid_digest,
        exit,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "this common hash boundary keeps every append-only V1 identity term explicit"
)]
fn derive_strategy_digest_from_terms_v1(
    population_id: [u8; 32],
    instrument_family: InstrumentFamilyV1,
    rung_seconds: u32,
    evaluation_policy_digest: [u8; 32],
    direction: TradeDirectionV1,
    run_id: [u8; 32],
    mask_words: [u64; 6],
    horizon_bars: u32,
    exit_grid_policy_digest: [u8; 32],
    resolved_exit_grid_digest: [u8; 32],
    exit: ExitCoordinateV1,
) -> Result<[u8; 32], PopulationAdmissionWriterRefusal> {
    for (name, digest) in [
        ("population identity", population_id),
        ("evaluation policy identity", evaluation_policy_digest),
        ("execution run identity", run_id),
        ("exit-grid policy identity", exit_grid_policy_digest),
        ("resolved exit-grid identity", resolved_exit_grid_digest),
    ] {
        require_nonzero_digest(name, &digest)?;
    }
    validate_rung_seconds(rung_seconds)?;
    if horizon_bars == 0 {
        return Err("strategy identity execution horizon is zero".to_owned());
    }
    let mut hasher = Hasher::new();
    hasher.update(STRATEGY_ID_DOMAIN_V1);
    hasher.update(&STRATEGY_ID_VERSION_V1.to_le_bytes());
    hasher.update(&population_id);
    hasher.update(&[instrument_family_byte(instrument_family)]);
    hasher.update(&rung_seconds.to_le_bytes());
    hasher.update(&evaluation_policy_digest);
    hasher.update(&run_id);
    for word in mask_words {
        hasher.update(&word.to_le_bytes());
    }
    hasher.update(&[direction_byte(direction)]);
    hasher.update(&horizon_bars.to_le_bytes());
    hasher.update(&exit_grid_policy_digest);
    hasher.update(&resolved_exit_grid_digest);
    hash_exit_coordinate(&mut hasher, exit);
    let identity = hasher.finalize();
    require_nonzero_digest("derived strategy identity", &identity)?;
    Ok(identity)
}

/// Runs one uncapped itemset population and expands every closed mask through
/// both complete dynamic exit grids in canonical order.
///
/// The emitted order is ladder level, mask, long side, every long coordinate,
/// short side, every short coordinate.  No bounded ranked frontier is called.
/// The caller supplies a typed directional [`ExecutionRunV1`] per closed mask
/// and side, plus the statistics that cannot be derived from one grid cell.
/// Both are checked by the existing opaque execution/admission capabilities.
///
/// Allocation and all candidate preparation remain local until the engine has
/// proved natural extinction, complete closure and exact count
/// reconciliation.  Any refusal drops the local prefix and exposes no
/// [`ProducedPopulationAdmissionV1`].
///
/// # Errors
///
/// Returns the first identity, run, grid, cell, evidence, allocation,
/// extinction, reconciliation or Population V4 preparation refusal.
pub fn produce_population_admission_v1<RunAuthority, EvidenceAuthority>(
    sweeper: &Sweeper,
    signal_column: Column,
    authority: CompletePopulationAuthorityV1<'_>,
    on_level: &dyn Fn(&engine::Frontier, usize, u64),
    mut execution_run: RunAuthority,
    mut evidence_for: EvidenceAuthority,
) -> Result<ProducedPopulationAdmissionV1, PopulationAdmissionWriterRefusal>
where
    RunAuthority: FnMut([u64; 6], TradeDirectionV1) -> Result<ExecutionRunV1, String>,
    EvidenceAuthority: for<'cell> FnMut(
        PopulationCellContextV1<'cell>,
    ) -> Result<PopulationCellEvidenceV1, String>,
{
    let population_id = derive_population_id_v1(&authority)?;
    let mut evaluated_sides = Vec::new();
    let population_run =
        sweeper.run_prepared_population_by_reporting(signal_column, on_level, |member| {
            evaluate_population_member(&authority, member, &mut evaluated_sides, &mut execution_run)
        })?;
    let expanded_cell_count = evaluated_cell_count(&evaluated_sides)?;
    let reconciliation =
        reconcile_population_run(&population_run, &authority, expanded_cell_count)?;
    let candidates = finalize_population_evidence(
        population_id,
        &authority,
        &population_run,
        reconciliation,
        &evaluated_sides,
        &mut evidence_for,
    )?;
    let prepared = prepare_population_admission_v1(&PopulationAdmissionWriteRequestV1 {
        population_id,
        instrument_family: authority.instrument_family,
        rung_seconds: authority.rung_seconds,
        requested_span: authority.requested_span,
        reconciliation,
        identities: authority.identities,
        signal_calendar: authority.signal_calendar,
        execution_calendar: authority.execution_calendar,
        admission_policy: authority.admission_policy,
        candidates: &candidates,
    })?;
    Ok(ProducedPopulationAdmissionV1 {
        population_id,
        reconciliation,
        population_run,
        prepared,
    })
}

/// Outcomes of the two receipt-last commits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopulationAdmissionCommitV1 {
    /// Whether Population V4 was newly written or byte-identically reused.
    pub population: PopulationCommit,
    /// Whether admission authority was newly written or byte-identically reused.
    pub admission: AdmissionCommitOutcomeV1,
    /// Digest binding the admission completion to this exact V4 receipt.
    pub population_v4_completion_digest: [u8; 32],
}

/// Prepares all Population V4 rows and admission decisions without I/O.
///
/// Explicit unmeasured or refused evidence is preserved and evaluated to the
/// corresponding fail-closed terminal status.  It is never promoted to a
/// numeric value or an admitted row.  When evidence directly duplicated by the
/// population row (support, trade counts/rates and monetary ranking fields) is
/// measured, it must be byte-semantically equal.  An explicit `Unmeasured` or
/// `Refused` state remains explicit so a zero-trade cell can be durably
/// classified without turning its undefined rate into numeric zero.
///
/// # Errors
///
/// Refuses a policy-identity mismatch, allocation/sequence overflow, any
/// row/evidence contradiction, a verdict projection mismatch, or every
/// Population V4 reconciliation/calendar refusal.
pub fn prepare_population_admission_v1(
    request: &PopulationAdmissionWriteRequestV1<'_>,
) -> Result<PreparedPopulationAdmissionV1, PopulationAdmissionWriterRefusal> {
    let policy_digest = request.admission_policy.digest();
    if request.identities.admission_policy_digest != policy_digest {
        return Err(
            "Population V4 admission-policy identity differs from the supplied canonical policy"
                .to_owned(),
        );
    }

    let mut rows = Vec::new();
    rows.try_reserve_exact(request.candidates.len())
        .map_err(|why| {
            format!(
                "Population V4 writer could not reserve {} rows: {why}",
                request.candidates.len()
            )
        })?;
    let mut decisions = Vec::new();
    decisions
        .try_reserve_exact(request.candidates.len())
        .map_err(|why| {
            format!(
                "admission writer could not reserve {} decisions: {why}",
                request.candidates.len()
            )
        })?;

    for (index, candidate) in request.candidates.iter().copied().enumerate() {
        let sequence = u64::try_from(index)
            .map_err(|_| "Population V4 candidate sequence does not fit u64".to_owned())?;
        validate_direct_evidence(sequence, &candidate)?;
        let sealed = request
            .admission_policy
            .evaluate_sealed(&candidate.evidence);
        let admission = admission_projection(sealed.verdict());
        admission
            .require_matches_verdict(sealed.verdict())
            .map_err(|why| format!("candidate {sequence} admission projection refused: {why}"))?;
        let row = PopulationRowV1 {
            population_id: request.population_id,
            sequence,
            strategy_digest: candidate.strategy_digest,
            mask_words: candidate.mask_words,
            direction: candidate.direction,
            instrument_family: request.instrument_family,
            closure: candidate.closure,
            rung_seconds: request.rung_seconds,
            support_hits: candidate.support_hits,
            exit: candidate.exit,
            metrics: candidate.metrics,
            admission,
        };
        let row_payload_digest = row
            .payload_digest()
            .map_err(|why| format!("candidate {sequence} population row refused: {why}"))?;
        let decision = AdmissionDecisionRecordV1::new(
            request.population_id,
            sequence,
            candidate.strategy_digest,
            row_payload_digest,
            &sealed,
        )
        .map_err(|why| format!("candidate {sequence} admission decision refused: {why}"))?;
        rows.push(row);
        decisions.push(decision);
    }

    let receipt = CompletionReceiptV4::for_rows(
        request.population_id,
        request.instrument_family,
        request.rung_seconds,
        &rows,
        request.requested_span,
        request.reconciliation,
        request.identities,
        request.signal_calendar,
        request.execution_calendar,
    )?;
    Ok(PreparedPopulationAdmissionV1 {
        rows,
        decisions,
        receipt,
        policy: request.admission_policy,
    })
}

/// Commits one fully prepared Population V4 and its admission authority.
///
/// All validation and allocation happen before writer creation.  Population
/// rows and their V4 receipt commit first.  Admission decisions then commit
/// under the same ledger lock path with their receipt last.  If the process
/// stops between those commits, joined readers refuse the incomplete pair and
/// an exact rerun resumes safely.
///
/// # Errors
///
/// Every preparation, append-only ledger, locking, integrity, reconciliation,
/// sync or same-identity/different-bytes refusal is returned unchanged.  A
/// Population V4 prefix can remain after a later admission refusal; it is not
/// joined authority and is recovered only by an exact rerun.
pub fn commit_population_admission_v1(
    root: &Path,
    request: &PopulationAdmissionWriteRequestV1<'_>,
) -> Result<PopulationAdmissionCommitV1, PopulationAdmissionWriterRefusal> {
    let prepared = prepare_population_admission_v1(request)?;
    let population_v4_completion_digest = prepared.receipt.content_digest()?;

    let population = {
        let mut populations = PopulationLedger::open(root)?;
        populations.append_complete_v4(&prepared.rows, &prepared.receipt)?
    };

    let mut admissions = AdmissionAuthorityLedger::open(root)?;
    let admission = admissions.commit(
        request.population_id,
        population_v4_completion_digest,
        &prepared.policy,
        &prepared.decisions,
    )?;
    Ok(PopulationAdmissionCommitV1 {
        population,
        admission,
        population_v4_completion_digest,
    })
}

fn evaluate_population_member<RunAuthority>(
    authority: &CompletePopulationAuthorityV1<'_>,
    member: PopulationMember,
    evaluated_sides: &mut Vec<EvaluatedPopulationSideV1>,
    execution_run: &mut RunAuthority,
) -> Result<(), PopulationAdmissionWriterRefusal>
where
    RunAuthority: FnMut([u64; 6], TradeDirectionV1) -> Result<ExecutionRunV1, String>,
{
    match member.closure {
        ClosureVerdict::Redundant => Ok(()),
        ClosureVerdict::Unknown => Err(format!(
            "uncapped population reached mask {:?} without a terminal closure verdict",
            member.item.mask.words()
        )),
        ClosureVerdict::Closed => {
            evaluate_population_side(
                authority,
                member,
                TradeDirectionV1::Long,
                authority.long_exit_grid,
                evaluated_sides,
                execution_run,
            )?;
            evaluate_population_side(
                authority,
                member,
                TradeDirectionV1::Short,
                authority.short_exit_grid,
                evaluated_sides,
                execution_run,
            )
        }
    }
}

fn evaluate_population_side<RunAuthority>(
    authority: &CompletePopulationAuthorityV1<'_>,
    member: PopulationMember,
    direction: TradeDirectionV1,
    resolved: &ResolvedExitGridV1,
    evaluated_sides: &mut Vec<EvaluatedPopulationSideV1>,
    execution_run: &mut RunAuthority,
) -> Result<(), PopulationAdmissionWriterRefusal>
where
    RunAuthority: FnMut([u64; 6], TradeDirectionV1) -> Result<ExecutionRunV1, String>,
{
    let mask_words = member.item.mask.words();
    let run = execution_run(mask_words, direction).map_err(|why| {
        format!(
            "{} execution-run authority refused mask {:?}: {why}",
            direction_name(direction),
            member.item.mask.words()
        )
    })?;
    if run.mask().words() != mask_words {
        return Err(format!(
            "{} execution-run authority returned another mask",
            direction_name(direction)
        ));
    }
    let evaluated = resolved
        .evaluate_training_grid_attested(
            authority.execution_series,
            authority.execution_column,
            authority.horizon,
            run,
        )
        .map_err(|why| {
            format!(
                "{} complete exit-grid evaluation refused mask {:?}: {why:?}",
                direction_name(direction),
                member.item.mask.words()
            )
        })?;
    let cell_count = u64::try_from(evaluated.grid().cells.len())
        .map_err(|_| "evaluated exit-cell count does not fit u64".to_owned())?;
    if cell_count != resolved.cell_count() {
        return Err(format!(
            "{} evaluated exit-cell count {cell_count} differs from resolved count {}",
            direction_name(direction),
            resolved.cell_count()
        ));
    }
    evaluated_sides
        .try_reserve_exact(1)
        .map_err(|why| format!("could not reserve one evaluated population side: {why}"))?;
    evaluated_sides.push(EvaluatedPopulationSideV1 {
        mask_words,
        support_hits: member.item.hits,
        direction,
        evaluated,
    });
    Ok(())
}

fn evaluated_cell_count(
    evaluated_sides: &[EvaluatedPopulationSideV1],
) -> Result<usize, PopulationAdmissionWriterRefusal> {
    evaluated_sides.iter().try_fold(0_usize, |total, side| {
        total
            .checked_add(side.evaluated.grid().cells.len())
            .ok_or_else(|| "expanded exit-cell population count overflowed usize".to_owned())
    })
}

fn finalize_population_evidence<EvidenceAuthority>(
    population_id: [u8; 32],
    authority: &CompletePopulationAuthorityV1<'_>,
    population_run: &PopulationRun,
    reconciliation: CompletionReconciliationV2,
    evaluated_sides: &[EvaluatedPopulationSideV1],
    evidence_for: &mut EvidenceAuthority,
) -> Result<Vec<EvaluatedPopulationCandidateV1>, PopulationAdmissionWriterRefusal>
where
    EvidenceAuthority: for<'cell> FnMut(
        PopulationCellContextV1<'cell>,
    ) -> Result<PopulationCellEvidenceV1, String>,
{
    if !population_run.is_complete()
        || !reconciliation.extinction_complete
        || !reconciliation.closure_complete
        || reconciliation.unknown_closure_itemsets != 0
    {
        return Err(
            "statistical evidence cannot be requested before population completion".to_owned(),
        );
    }
    let cell_count = evaluated_cell_count(evaluated_sides)?;
    let mut candidates = Vec::new();
    candidates.try_reserve_exact(cell_count).map_err(|why| {
        format!("could not reserve {cell_count} finalized population rows: {why}")
    })?;
    for side in evaluated_sides {
        let resolved = match side.direction {
            TradeDirectionV1::Long => authority.long_exit_grid,
            TradeDirectionV1::Short => authority.short_exit_grid,
        };
        let validated = resolved
            .validate_evaluation(&side.evaluated)
            .map_err(|why| {
                format!(
                    "{} complete exit-grid validation refused mask {:?}: {why:?}",
                    direction_name(side.direction),
                    side.mask_words
                )
            })?;
        for (cell_ordinal, cell) in side.evaluated.grid().cells.iter().enumerate() {
            let exit = exit_coordinate(cell)?;
            let strategy_digest = derive_strategy_digest_from_validated_v1(
                population_id,
                authority.instrument_family,
                authority.rung_seconds,
                authority.identities.evaluation_policy_digest,
                side.direction,
                resolved,
                &side.evaluated,
                &validated,
                cell_ordinal,
            )?;
            let statistical = evidence_for(PopulationCellContextV1 {
                population_id,
                strategy_digest,
                mask_words: side.mask_words,
                support_hits: side.support_hits,
                population_run,
                reconciliation,
                direction: side.direction,
                exit,
                cell,
                cell_ordinal,
                resolved,
                evaluated: &side.evaluated,
                validated: &validated,
            })
            .map_err(|why| {
                format!(
                    "{} statistical evidence authority refused strategy {}: {why}",
                    direction_name(side.direction),
                    hex(&strategy_digest)
                )
            })?;
            let metrics = metrics_from_cell(cell, statistical.assurance_ppm)?;
            candidates.push(EvaluatedPopulationCandidateV1 {
                strategy_digest,
                mask_words: side.mask_words,
                direction: side.direction,
                closure: ClosureV1::Closed,
                support_hits: side.support_hits,
                exit,
                metrics,
                evidence: statistical.admission,
            });
        }
    }
    Ok(candidates)
}

fn reconcile_population_run(
    run: &PopulationRun,
    authority: &CompletePopulationAuthorityV1<'_>,
    candidate_count: usize,
) -> Result<CompletionReconciliationV2, PopulationAdmissionWriterRefusal> {
    if !run.is_complete() {
        return Err(format!(
            "uncapped population is incomplete: closure_complete={}, considered={}, redundant={}, closed={}, halted={:?}",
            run.outcome.closure_complete,
            run.considered,
            run.redundant,
            run.closed,
            run.outcome.sweep.halted
        ));
    }
    let classified = run
        .redundant
        .checked_add(run.closed)
        .ok_or_else(|| "closed plus redundant itemsets overflow u64".to_owned())?;
    let unknown_closure_itemsets = run
        .considered
        .checked_sub(classified)
        .ok_or_else(|| "closure counts exceed frequent itemsets".to_owned())?;
    if unknown_closure_itemsets != 0 {
        return Err(format!(
            "uncapped population retained {unknown_closure_itemsets} unknown closure verdict(s)"
        ));
    }
    let infrequent_itemsets = run
        .outcome
        .trials
        .checked_sub(run.considered)
        .ok_or_else(|| "frequent itemsets exceed sweep trials".to_owned())?;
    let exit_cells_per_mask = ExitCellsPerMaskV2::new(
        authority.long_exit_grid.cell_count(),
        authority.short_exit_grid.cell_count(),
    )?;
    let cells_per_mask = exit_cells_per_mask
        .long()
        .checked_add(exit_cells_per_mask.short())
        .ok_or_else(|| "long plus short exit cells per mask overflow u64".to_owned())?;
    let expected_candidates = run
        .closed
        .checked_mul(cells_per_mask)
        .ok_or_else(|| "closed masks times exit cells overflow u64".to_owned())?;
    let actual_candidates = u64::try_from(candidate_count)
        .map_err(|_| "produced candidate count does not fit u64".to_owned())?;
    if actual_candidates != expected_candidates {
        return Err(format!(
            "expanded population produced {actual_candidates} rows, not closed masks {} times long+short cells {cells_per_mask} = {expected_candidates}",
            run.closed
        ));
    }
    let extinction_depth = u32::try_from(run.outcome.sweep.depth())
        .map_err(|_| "extinction depth does not fit u32".to_owned())?;
    Ok(CompletionReconciliationV2 {
        sweep_trials: run.outcome.trials,
        frequent_itemsets: run.considered,
        infrequent_itemsets,
        closed_itemsets: run.closed,
        redundant_itemsets: run.redundant,
        unknown_closure_itemsets,
        exit_cells_per_mask,
        extinction_depth,
        extinction_complete: run.outcome.sweep.completed(),
        closure_complete: run.outcome.closure_complete,
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "one fail-closed authority door validates every identity, calendar, grid and execution-series term in fixed precedence"
)]
fn validate_complete_population_authority(
    authority: &CompletePopulationAuthorityV1<'_>,
) -> Result<(), PopulationAdmissionWriterRefusal> {
    validate_rung_seconds(authority.rung_seconds)?;
    if authority.signal_calendar.rung_seconds() != authority.rung_seconds {
        return Err(format!(
            "signal calendar rung {} differs from population rung {}",
            authority.signal_calendar.rung_seconds(),
            authority.rung_seconds
        ));
    }
    if authority.execution_calendar.rung_seconds() != 60 {
        return Err(format!(
            "execution calendar rung {} is not exact one-minute OHLCV",
            authority.execution_calendar.rung_seconds()
        ));
    }
    if (
        authority.signal_calendar.first_day(),
        authority.signal_calendar.last_day(),
    ) != (
        authority.execution_calendar.first_day(),
        authority.execution_calendar.last_day(),
    ) {
        return Err("signal and execution calendar coverage boundaries differ".to_owned());
    }
    let requested_bounds = requested_span_day_bounds(authority.requested_span)?;
    if (
        authority.signal_calendar.first_day(),
        authority.signal_calendar.last_day(),
    ) != requested_bounds
    {
        return Err(
            "complete calendar coverage boundaries differ from the requested civil-month span"
                .to_owned(),
        );
    }
    for (name, digest) in population_identity_digests(&authority.identities) {
        require_nonzero_digest(name, &digest)?;
    }
    authority.identities.exit_grids.composite_digest()?;
    if authority.identities.admission_policy_digest != authority.admission_policy.digest() {
        return Err(
            "Population V4 admission-policy identity differs from the canonical policy".to_owned(),
        );
    }
    require_family_and_side(
        authority.instrument_family,
        TradeDirectionV1::Long,
        authority.long_exit_grid,
    )?;
    require_family_and_side(
        authority.instrument_family,
        TradeDirectionV1::Short,
        authority.short_exit_grid,
    )?;
    require_resolved_identity(
        "long",
        authority.long_exit_grid,
        authority.identities.exit_grids.long.policy_digest,
        authority.identities.exit_grids.long.resolved_digest,
    )?;
    require_resolved_identity(
        "short",
        authority.short_exit_grid,
        authority.identities.exit_grids.short.policy_digest,
        authority.identities.exit_grids.short.resolved_digest,
    )?;
    let long = authority.long_exit_grid;
    let short = authority.short_exit_grid;
    if long.instrument() != short.instrument()
        || long.feed_digest() != short.feed_digest()
        || long.commit_digest() != short.commit_digest()
        || long.calendar_digest() != short.calendar_digest()
        || long.training_digest() != short.training_digest()
        || long.training_bars() != short.training_bars()
        || long.training_first_ts_micros() != short.training_first_ts_micros()
        || long.training_last_ts_micros() != short.training_last_ts_micros()
    {
        return Err(
            "long and short exit grids were not resolved from one exact instrument/feed/commit/calendar/training series"
                .to_owned(),
        );
    }
    if *authority.execution_series.instrument() != long.instrument() {
        return Err("execution series instrument differs from both resolved grids".to_owned());
    }
    let feed_digest = brutex_core::blake3::hash(authority.execution_series.feed().as_bytes());
    let commit_digest = brutex_core::blake3::hash(authority.execution_series.commit().as_bytes());
    if feed_digest != authority.identities.feed_digest || feed_digest != long.feed_digest() {
        return Err("execution feed identity differs from Population V4/resolved grids".to_owned());
    }
    if commit_digest != authority.identities.source_commit_digest
        || commit_digest != long.commit_digest()
    {
        return Err(
            "execution commit identity differs from Population V4/resolved grids".to_owned(),
        );
    }
    if authority.execution_series.calendar_digest() != authority.identities.calendar_policy_digest
        || authority.execution_series.calendar_digest() != long.calendar_digest()
    {
        return Err(
            "execution calendar-policy identity differs from Population V4/resolved grids"
                .to_owned(),
        );
    }
    let execution_digest = runner::identity::data_digest(authority.execution_series.bars());
    let execution_bars = u64::try_from(authority.execution_series.bars().len())
        .map_err(|_| "execution bar count does not fit u64".to_owned())?;
    let execution_first = authority
        .execution_series
        .bars()
        .first()
        .map(|bar| bar.ts_micros)
        .ok_or_else(|| "population execution series is empty".to_owned())?;
    let execution_last = authority
        .execution_series
        .bars()
        .last()
        .map(|bar| bar.ts_micros)
        .ok_or_else(|| "population execution series is empty".to_owned())?;
    if (
        execution_digest,
        execution_bars,
        execution_first,
        execution_last,
    ) != (
        long.training_digest(),
        long.training_bars(),
        long.training_first_ts_micros(),
        long.training_last_ts_micros(),
    ) {
        return Err(
            "execution bytes differ from the exact TRAINING series bound by both grids".to_owned(),
        );
    }
    Ok(())
}

fn require_resolved_identity(
    side: &str,
    resolved: &ResolvedExitGridV1,
    expected_policy: [u8; 32],
    expected_resolution: [u8; 32],
) -> Result<(), PopulationAdmissionWriterRefusal> {
    if !resolved.digest_is_valid() {
        return Err(format!(
            "{side} exit-grid resolution failed its internal digest"
        ));
    }
    if resolved.policy_digest() != expected_policy || resolved.digest() != expected_resolution {
        return Err(format!(
            "{side} exit-grid policy/resolution differs from Population V4 identity"
        ));
    }
    if resolved.cell_count() == 0 {
        return Err(format!("{side} resolved exit grid contains zero cells"));
    }
    Ok(())
}

fn validate_rung_seconds(rung_seconds: u32) -> Result<(), PopulationAdmissionWriterRefusal> {
    if matches!(
        rung_seconds,
        60 | 120 | 180 | 300 | 600 | 900 | 1_800 | 3_600
    ) {
        Ok(())
    } else {
        Err(format!(
            "population signal rung {rung_seconds} seconds is outside the exact eight-rung surface"
        ))
    }
}

fn requested_span_day_bounds(
    span: RequestedSpanIdentityV1,
) -> Result<(i64, i64), PopulationAdmissionWriterRefusal> {
    let first = Day::new(span.from_year(), span.from_month(), 1)
        .map_err(|_| "requested span first civil month is invalid".to_owned())?;
    let last = Day::new(span.to_year(), span.to_month(), 1)
        .map_err(|_| "requested span last civil month is invalid".to_owned())?
        .end_of_month();
    Ok((
        i64::from(first.days_from_epoch()),
        i64::from(last.days_from_epoch()),
    ))
}

fn require_family_and_side(
    family: InstrumentFamilyV1,
    direction: TradeDirectionV1,
    resolved: &ResolvedExitGridV1,
) -> Result<(), PopulationAdmissionWriterRefusal> {
    let expected_family = match family {
        InstrumentFamilyV1::Nifty => RunnerInstrumentFamilyV1::Nifty,
        InstrumentFamilyV1::BankNifty => RunnerInstrumentFamilyV1::BankNifty,
    };
    let expected_side = match direction {
        TradeDirectionV1::Long => runner::excursion::Side::Long,
        TradeDirectionV1::Short => runner::excursion::Side::Short,
    };
    if resolved.family() != expected_family {
        return Err(format!(
            "{} exit-grid instrument family differs from the population",
            direction_name(direction)
        ));
    }
    if resolved.side() != expected_side {
        return Err(format!(
            "{} exit-grid resolution carries the opposite side",
            direction_name(direction)
        ));
    }
    Ok(())
}

pub(crate) fn metrics_from_cell(
    cell: &Cell,
    assurance_ppm: u64,
) -> Result<TopMetricsV1, PopulationAdmissionWriterRefusal> {
    if assurance_ppm > PPM {
        return Err(format!(
            "statistical assurance {assurance_ppm} ppm exceeds one million"
        ));
    }
    let assurance_bp = u64::try_from(cell.assurance_bp())
        .map_err(|_| "cell Wilson assurance basis points are negative".to_owned())?;
    if assurance_ppm / 100 != assurance_bp {
        return Err(format!(
            "statistical assurance {assurance_ppm} ppm does not fall in cell Wilson floor bucket {assurance_bp} bp"
        ));
    }
    if cell.wins > cell.trades {
        return Err(format!(
            "cell winning trades {} exceed total trades {}",
            cell.wins, cell.trades
        ));
    }
    if cell.max_drawdown < 0 || cell.gross_win < 0 || cell.gross_loss > 0 || cell.min_win < 0 {
        return Err(
            "cell carries a negative drawdown/gross win/minimum win or a positive gross loss"
                .to_owned(),
        );
    }
    let losing_trades = cell.trades - cell.wins;
    let drawdown = u64::try_from(cell.max_drawdown)
        .map_err(|_| "cell drawdown does not fit u64".to_owned())?;
    let worst_loss = negative_magnitude(cell.worst_trade);
    let gross_win =
        u64::try_from(cell.gross_win).map_err(|_| "cell gross win does not fit u64".to_owned())?;
    let gross_loss = negative_magnitude(cell.gross_loss);
    let min_win =
        u64::try_from(cell.min_win).map_err(|_| "cell minimum win does not fit u64".to_owned())?;
    let average_win =
        u64::try_from(cell.avg_win()).map_err(|_| "cell average win is negative".to_owned())?;
    let average_loss = negative_magnitude(cell.avg_loss());
    Ok(TopMetricsV1 {
        drawdown,
        worst_loss,
        losing_rate_ppm: rate_ppm(losing_trades, cell.trades),
        losing_trades,
        loss_ratio_ppm: ratio_ppm(gross_loss, gross_win)?,
        pessimistic_profit: cell.pessimistic,
        winning_trades: cell.wins,
        win_rate_ppm: rate_ppm(cell.wins, cell.trades),
        reward_to_risk_ppm: ratio_ppm(min_win, worst_loss)?,
        average_win,
        average_loss,
        assurance_ppm,
    })
}

fn exit_coordinate(cell: &Cell) -> Result<ExitCoordinateV1, PopulationAdmissionWriterRefusal> {
    exit_coordinate_from_chosen(Chosen::from_cell(cell))
}

fn exit_coordinate_from_chosen(
    chosen: Chosen,
) -> Result<ExitCoordinateV1, PopulationAdmissionWriterRefusal> {
    Ok(ExitCoordinateV1 {
        stop: optional_index_u32("stop", chosen.stop)?,
        target: optional_index_u32("target", chosen.target)?,
        tsl: optional_index_u32("TSL", chosen.tsl)?,
        ttp: chosen
            .ttp
            .map(|ttp| {
                Ok::<(u32, u32), PopulationAdmissionWriterRefusal>((
                    index_u32("TTP arm", ttp.arm)?,
                    index_u32("TTP trail", ttp.trail)?,
                ))
            })
            .transpose()?,
    })
}

fn optional_index_u32(
    name: &str,
    index: Option<usize>,
) -> Result<Option<u32>, PopulationAdmissionWriterRefusal> {
    index.map(|value| index_u32(name, value)).transpose()
}

fn index_u32(name: &str, index: usize) -> Result<u32, PopulationAdmissionWriterRefusal> {
    let value = u32::try_from(index)
        .map_err(|_| format!("exit {name} index {index} does not fit Population V1 u32"))?;
    if value == u32::MAX {
        return Err(format!(
            "exit {name} index u32::MAX is reserved for an absent coordinate"
        ));
    }
    Ok(value)
}

fn rate_ppm(part: u64, total: u64) -> u64 {
    if total == 0 {
        return 0;
    }
    let projected = u128::from(part)
        .saturating_mul(u128::from(PPM))
        .checked_div(u128::from(total))
        .unwrap_or(0);
    u64::try_from(projected).unwrap_or(u64::MAX)
}

fn ratio_ppm(
    numerator: u64,
    denominator: u64,
) -> Result<Option<u64>, PopulationAdmissionWriterRefusal> {
    if denominator == 0 {
        return Ok(None);
    }
    let value = u128::from(numerator)
        .checked_mul(u128::from(PPM))
        .and_then(|scaled| scaled.checked_div(u128::from(denominator)))
        .ok_or_else(|| "cell ratio ppm arithmetic overflowed".to_owned())?;
    u64::try_from(value)
        .map(Some)
        .map_err(|_| "cell ratio ppm does not fit u64".to_owned())
}

const fn negative_magnitude(value: i64) -> u64 {
    if value < 0 { value.unsigned_abs() } else { 0 }
}

fn hash_population_identities(hasher: &mut Hasher, identities: &PopulationIdentitiesV2) {
    for digest in [
        identities.run_identity,
        identities.data_digest,
        identities.feed_digest,
        identities.source_commit_digest,
        identities.vocabulary_digest,
        identities.evaluation_policy_digest,
        identities.exit_grids.long.policy_digest,
        identities.exit_grids.long.resolved_digest,
        identities.exit_grids.short.policy_digest,
        identities.exit_grids.short.resolved_digest,
        identities.admission_policy_digest,
        identities.ranking_policy_digest,
        identities.calendar_policy_digest,
        identities.daily_reference_policy_digest,
    ] {
        hasher.update(&digest);
    }
}

fn population_identity_digests(
    identities: &PopulationIdentitiesV2,
) -> [(&'static str, [u8; 32]); 14] {
    [
        ("run identity", identities.run_identity),
        ("data identity", identities.data_digest),
        ("feed identity", identities.feed_digest),
        ("source commit identity", identities.source_commit_digest),
        ("vocabulary identity", identities.vocabulary_digest),
        (
            "evaluation policy identity",
            identities.evaluation_policy_digest,
        ),
        (
            "long exit-grid policy identity",
            identities.exit_grids.long.policy_digest,
        ),
        (
            "long exit-grid resolution identity",
            identities.exit_grids.long.resolved_digest,
        ),
        (
            "short exit-grid policy identity",
            identities.exit_grids.short.policy_digest,
        ),
        (
            "short exit-grid resolution identity",
            identities.exit_grids.short.resolved_digest,
        ),
        (
            "admission policy identity",
            identities.admission_policy_digest,
        ),
        ("ranking policy identity", identities.ranking_policy_digest),
        (
            "calendar policy identity",
            identities.calendar_policy_digest,
        ),
        (
            "daily reference policy identity",
            identities.daily_reference_policy_digest,
        ),
    ]
}

fn hash_calendar_receipt(hasher: &mut Hasher, receipt: CompleteCalendarReceiptV2) {
    hasher.update(&receipt.rung_seconds().to_le_bytes());
    hasher.update(&receipt.first_day().to_le_bytes());
    hasher.update(&receipt.last_day().to_le_bytes());
    hasher.update(&receipt.digest());
}

fn hash_exit_coordinate(hasher: &mut Hasher, exit: ExitCoordinateV1) {
    for index in [exit.stop, exit.target, exit.tsl] {
        hash_optional_index(hasher, index);
    }
    match exit.ttp {
        None => {
            hasher.update(&[0]);
        }
        Some((arm, trail)) => {
            hasher.update(&[1]);
            hasher.update(&arm.to_le_bytes());
            hasher.update(&trail.to_le_bytes());
        }
    }
}

fn hash_optional_index(hasher: &mut Hasher, index: Option<u32>) {
    match index {
        None => {
            hasher.update(&[0]);
        }
        Some(value) => {
            hasher.update(&[1]);
            hasher.update(&value.to_le_bytes());
        }
    }
}

fn require_nonzero_digest(
    name: &str,
    digest: &[u8; 32],
) -> Result<(), PopulationAdmissionWriterRefusal> {
    if digest.iter().all(|byte| *byte == 0) {
        Err(format!("{name} is absent (all zero)"))
    } else {
        Ok(())
    }
}

const fn instrument_family_byte(family: InstrumentFamilyV1) -> u8 {
    match family {
        InstrumentFamilyV1::Nifty => 1,
        InstrumentFamilyV1::BankNifty => 2,
    }
}

const fn direction_byte(direction: TradeDirectionV1) -> u8 {
    match direction {
        TradeDirectionV1::Long => 1,
        TradeDirectionV1::Short => 2,
    }
}

const fn direction_name(direction: TradeDirectionV1) -> &'static str {
    match direction {
        TradeDirectionV1::Long => "long",
        TradeDirectionV1::Short => "short",
    }
}

fn hex(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(64);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn admission_projection(verdict: runner::admission::AdmissionVerdictV1) -> AdmissionV1 {
    let status = match verdict.status() {
        RunnerAdmissionStatusV1::Admitted => AdmissionStatusV1::Admitted,
        RunnerAdmissionStatusV1::Rejected => AdmissionStatusV1::Rejected,
        RunnerAdmissionStatusV1::Unmeasured => AdmissionStatusV1::Unmeasured,
        RunnerAdmissionStatusV1::Refused => AdmissionStatusV1::Refused,
    };
    AdmissionV1 {
        status,
        reasons: verdict.reasons().bits(),
        failed: verdict.failed().bits(),
        unmeasured: verdict.unmeasured().bits(),
        refused: verdict.refused().bits(),
    }
}

fn validate_direct_evidence(
    sequence: u64,
    candidate: &EvaluatedPopulationCandidateV1,
) -> Result<(), PopulationAdmissionWriterRefusal> {
    let values = candidate.evidence.values();
    require_consistent_u64(
        sequence,
        "support_hits",
        values.support_hits,
        candidate.support_hits,
    )?;
    let trades = candidate
        .metrics
        .winning_trades
        .checked_add(candidate.metrics.losing_trades)
        .ok_or_else(|| format!("candidate {sequence} classified trade count overflowed u64"))?;
    for (name, observed, exact) in [
        ("trades", values.trades, trades),
        (
            "drawdown_paisa",
            values.drawdown_paisa,
            candidate.metrics.drawdown,
        ),
        (
            "worst_trade_loss_paisa",
            values.worst_trade_loss_paisa,
            candidate.metrics.worst_loss,
        ),
        (
            "losing_trade_rate_ppm",
            values.losing_trade_rate_ppm,
            candidate.metrics.losing_rate_ppm,
        ),
        (
            "losing_trades",
            values.losing_trades,
            candidate.metrics.losing_trades,
        ),
        (
            "winning_trades",
            values.winning_trades,
            candidate.metrics.winning_trades,
        ),
        (
            "win_rate_ppm",
            values.win_rate_ppm,
            candidate.metrics.win_rate_ppm,
        ),
        (
            "wilson_win_rate_ppm",
            values.wilson_win_rate_ppm,
            candidate.metrics.assurance_ppm,
        ),
        (
            "average_win_paisa",
            values.average_win_paisa,
            candidate.metrics.average_win,
        ),
        (
            "average_loss_paisa",
            values.average_loss_paisa,
            candidate.metrics.average_loss,
        ),
    ] {
        require_consistent_u64(sequence, name, observed, exact)?;
    }
    require_consistent_i64(
        sequence,
        "pessimistic_profit_paisa",
        values.pessimistic_profit_paisa,
        candidate.metrics.pessimistic_profit,
    )?;
    match candidate.metrics.reward_to_risk_ppm {
        Some(exact) => require_consistent_u64(
            sequence,
            "worst_reward_risk_ppm",
            values.worst_reward_risk_ppm,
            exact,
        )?,
        None if matches!(values.worst_reward_risk_ppm, ObservedU64V1::Measured(_)) => {
            return Err(format!(
                "candidate {sequence} has measured worst_reward_risk_ppm but its ranking metric is undefined"
            ));
        }
        None => {}
    }
    for (name, completeness) in [
        ("execution_complete", values.execution_complete),
        ("calendar_complete", values.calendar_complete),
        ("population_complete", values.population_complete),
    ] {
        if completeness != CompletenessV1::Complete {
            return Err(format!(
                "candidate {sequence} {name} is {completeness:?}; it cannot be bound to a complete Population V4 receipt"
            ));
        }
    }
    Ok(())
}

fn require_consistent_u64(
    sequence: u64,
    name: &str,
    observed: ObservedU64V1,
    exact: u64,
) -> Result<(), PopulationAdmissionWriterRefusal> {
    match observed {
        ObservedU64V1::Measured(value) if value == exact => Ok(()),
        ObservedU64V1::Measured(value) => Err(format!(
            "candidate {sequence} evidence {name}={value} differs from evaluated population value {exact}"
        )),
        ObservedU64V1::Unmeasured | ObservedU64V1::Refused => Ok(()),
    }
}

fn require_consistent_i64(
    sequence: u64,
    name: &str,
    observed: ObservedI64V1,
    exact: i64,
) -> Result<(), PopulationAdmissionWriterRefusal> {
    match observed {
        ObservedI64V1::Measured(value) if value == exact => Ok(()),
        ObservedI64V1::Measured(value) => Err(format!(
            "candidate {sequence} evidence {name}={value} differs from evaluated population value {exact}"
        )),
        ObservedI64V1::Unmeasured | ObservedI64V1::Refused => Ok(()),
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    reason = "focused writer fixtures fail loudly when their own canonical setup is invalid"
)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use brutex_core::instrument::{Exchange, InstrumentKey};
    use engine::Ladder;
    use indicators::column::Column;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use pull::calendar::{DayKind, OPEN_MINUTE, kind_of};
    use pull::session::Day;
    use runner::admission::{
        AdmissionEvidenceValuesV1, AdmissionPolicyDraftV1, HypothesisDecisionV1,
    };
    use runner::excursion::Side;
    use runner::exit_grid_policy::{
        ExecutionResolutionV1, ExitGridPolicyV1, ExitGridSelectorV1, ForcedStopV1,
        RangeResolutionV1, RatioLimitsV1, RationalPercentileV1, RungPlanV1,
        printed_ohlcv_cost_model_id_v1,
    };
    use runner::identity::{Direction, Params, Run};

    use crate::admission_join::AdmissionAuthoritativeLedger;
    use crate::population::{LongShortExitGridIdentitiesV2, SideExitGridIdentityV2};
    use crate::stored::calendar_receipt_v2;

    const TEST_FEED: &str = "population-producer-test-feed";
    const TEST_COMMIT: &str = "population-producer-test-commit";
    const TEST_CALENDAR_POLICY: [u8; 32] = [0xA5; 32];
    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    fn root(name: &str) -> std::path::PathBuf {
        let sequence = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "brutex-population-admission-writer-{}-{name}-{sequence}",
            std::process::id()
        ))
    }

    fn evidence() -> AdmissionEvidenceV1 {
        AdmissionEvidenceV1::new(AdmissionEvidenceValuesV1 {
            support_hits: ObservedU64V1::Measured(100),
            independent_sessions: ObservedU64V1::Measured(20),
            trades: ObservedU64V1::Measured(50),
            max_mae_paisa: ObservedU64V1::Measured(500),
            worst_reward_risk_ppm: ObservedU64V1::Measured(2_000_000),
            win_rate_ppm: ObservedU64V1::Measured(600_000),
            wilson_win_rate_ppm: ObservedU64V1::Measured(550_000),
            return_drawdown_ppm: ObservedU64V1::Measured(3_000_000),
            weakest_period_return_paisa: ObservedI64V1::Measured(0),
            // Admission V1 has no genuine CSCV authority.  Its PBO-named
            // slots stay explicitly unmeasured even in this otherwise
            // complete writer fixture, so the durable verdict fails closed.
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
        .expect("complete evidence fixture is valid")
    }

    fn admission_policy() -> AdmissionPolicyV1 {
        admission_policy_with_support(1)
    }

    fn admission_policy_with_support(min_support_hits: u64) -> AdmissionPolicyV1 {
        AdmissionPolicyV1::new(AdmissionPolicyDraftV1 {
            min_support_hits: Some(min_support_hits),
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
        .expect("fully explicit relaxed policy is valid")
    }

    fn evaluator() -> Evaluator {
        Evaluator::new(
            Widths::pinned().expect("pinned widths are valid"),
            Availability::Absent,
            Thresholds::CLASSICAL,
        )
    }

    fn instrument(symbol: &str) -> InstrumentKey {
        InstrumentKey::index(Exchange::Nse, symbol).expect("swept spot-index fixture")
    }

    fn percentile() -> RationalPercentileV1 {
        RationalPercentileV1::new(1, 2).expect("one-half is a valid percentile")
    }

    fn exit_policy(side: Side) -> ExitGridPolicyV1 {
        ExitGridPolicyV1::new(
            ExecutionResolutionV1::OneMinuteOhlcv,
            RangeResolutionV1::PpmCeiling,
            side,
            RungPlanV1::new(
                vec![percentile()],
                vec![percentile()],
                vec![percentile()],
                1,
            )
            .expect("one exact rung per axis is valid"),
            RatioLimitsV1::new(1, 10_000, 1).expect("wide exact ratio interval is valid"),
            1_000,
            ExitGridSelectorV1::GuaranteedFloor,
            printed_ohlcv_cost_model_id_v1(),
            ForcedStopV1::Disabled,
            u64::MAX,
            u64::MAX,
        )
        .expect("complete side-specific exit policy is valid")
    }

    fn span() -> RequestedSpanIdentityV1 {
        RequestedSpanIdentityV1::new(2026, 7, 2026, 7).expect("measured fixture month")
    }

    fn complete_calendar(rung_seconds: u32) -> CompleteCalendarReceiptV2 {
        let requested = span();
        let first =
            Day::new(requested.from_year(), requested.from_month(), 1).expect("first fixture day");
        let last = Day::new(requested.to_year(), requested.to_month(), 1)
            .expect("last fixture month")
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
                                .saturating_sub(indicators::IST_OFFSET_MICROS);
                            if timestamps.last().copied() != Some(timestamp) {
                                timestamps.push(timestamp);
                            }
                        }
                    }
                }
                DayKind::Closed => {}
                DayKind::OpenLengthUnmeasured | DayKind::Unmeasured => {
                    panic!("fixture calendar day {day} is unmeasured")
                }
            }
        }
        calendar_receipt_v2(&timestamps, rung_seconds, first_day, last_day)
            .expect("calendar receipt is constructible")
            .require_complete()
            .expect("fixture calendar is complete")
    }

    fn identities(
        bars: &[indicators::Candle],
        column: &Column,
        long: &ResolvedExitGridV1,
        short: &ResolvedExitGridV1,
        policy: &AdmissionPolicyV1,
    ) -> PopulationIdentitiesV2 {
        let evaluation_policy_digest = column
            .evaluation_spec_token()
            .map(|token| brutex_core::blake3::hash(token.fingerprint_v1().as_bytes()))
            .expect("ordinary column carries its exact evaluator identity");
        PopulationIdentitiesV2 {
            run_identity: [1; 32],
            data_digest: runner::identity::data_digest(bars),
            feed_digest: brutex_core::blake3::hash(TEST_FEED.as_bytes()),
            source_commit_digest: brutex_core::blake3::hash(TEST_COMMIT.as_bytes()),
            vocabulary_digest: [2; 32],
            evaluation_policy_digest,
            exit_grids: LongShortExitGridIdentitiesV2 {
                long: SideExitGridIdentityV2 {
                    policy_digest: long.policy_digest(),
                    resolved_digest: long.digest(),
                },
                short: SideExitGridIdentityV2 {
                    policy_digest: short.policy_digest(),
                    resolved_digest: short.digest(),
                },
            },
            admission_policy_digest: policy.digest(),
            ranking_policy_digest: [3; 32],
            calendar_policy_digest: TEST_CALENDAR_POLICY,
            daily_reference_policy_digest: [4; 32],
        }
    }

    fn execution_run(
        instrument: &InstrumentKey,
        bars: &[indicators::Candle],
        ladder: Ladder,
        mask_words: [u64; 6],
        direction: TradeDirectionV1,
    ) -> Result<ExecutionRunV1, String> {
        let mask = runner::replay_mask::from_stored_words(mask_words)
            .map_err(|why| format!("test execution mask refused: {why}"))?;
        let run = Run {
            mask,
            direction: match direction {
                TradeDirectionV1::Long => Direction::Long,
                TradeDirectionV1::Short => Direction::Short,
            },
            instrument,
            timeframe: "1min",
            params: Params::of(ladder),
            data_digest: runner::identity::data_digest(bars),
            commit: TEST_COMMIT,
            feed: TEST_FEED,
        };
        ExecutionRunV1::new(&run, bars, None)
            .map_err(|why| format!("test execution-run seal refused: {why:?}"))
    }

    fn cell_evidence(context: &PopulationCellContextV1<'_>) -> PopulationCellEvidenceV1 {
        assert!(context.population_run.is_complete());
        assert!(context.reconciliation.extinction_complete);
        assert!(context.reconciliation.closure_complete);
        assert_eq!(context.reconciliation.unknown_closure_itemsets, 0);
        let assurance_ppm = context
            .cell
            .assurance_bp()
            .unsigned_abs()
            .saturating_mul(100);
        let metrics = metrics_from_cell(context.cell, assurance_ppm)
            .expect("evaluated cell has canonical direct metrics");
        let mut values = evidence().values();
        values.support_hits = ObservedU64V1::Measured(context.support_hits);
        values.trades =
            ObservedU64V1::Measured(metrics.winning_trades.saturating_add(metrics.losing_trades));
        values.worst_reward_risk_ppm = metrics
            .reward_to_risk_ppm
            .map_or(ObservedU64V1::Unmeasured, ObservedU64V1::Measured);
        values.win_rate_ppm = ObservedU64V1::Measured(metrics.win_rate_ppm);
        values.wilson_win_rate_ppm = ObservedU64V1::Measured(assurance_ppm);
        values.drawdown_paisa = ObservedU64V1::Measured(metrics.drawdown);
        values.worst_trade_loss_paisa = ObservedU64V1::Measured(metrics.worst_loss);
        values.losing_trade_rate_ppm = ObservedU64V1::Measured(metrics.losing_rate_ppm);
        values.losing_trades = ObservedU64V1::Measured(metrics.losing_trades);
        values.pessimistic_profit_paisa = ObservedI64V1::Measured(metrics.pessimistic_profit);
        values.winning_trades = ObservedU64V1::Measured(metrics.winning_trades);
        values.average_win_paisa = ObservedU64V1::Measured(metrics.average_win);
        values.average_loss_paisa = ObservedU64V1::Measured(metrics.average_loss);
        values.consecutive_losing_streak = ObservedU64V1::Measured(metrics.losing_trades.min(3));
        values.consecutive_winning_streak = ObservedU64V1::Measured(metrics.winning_trades.min(3));
        PopulationCellEvidenceV1 {
            assurance_ppm,
            admission: AdmissionEvidenceV1::new(values)
                .expect("cell-specific evidence remains in-domain"),
        }
    }

    fn resolved_grids<'a>(
        instrument: &'a InstrumentKey,
        bars: &'a [indicators::Candle],
    ) -> (
        ExecutionSeriesV1<'a>,
        ResolvedExitGridV1,
        ResolvedExitGridV1,
    ) {
        let series = ExecutionSeriesV1::new(
            instrument,
            TEST_FEED,
            TEST_COMMIT,
            TEST_CALENDAR_POLICY,
            bars,
        )
        .expect("attested execution fixture");
        let long = exit_policy(Side::Long)
            .resolve_attested(series)
            .expect("long resolution fixture");
        let short = exit_policy(Side::Short)
            .resolve_attested(series)
            .expect("short resolution fixture");
        (series, long, short)
    }

    fn authority<'a>(
        bars: &[indicators::Candle],
        column: &'a Column,
        series: ExecutionSeriesV1<'a>,
        long: &'a ResolvedExitGridV1,
        short: &'a ResolvedExitGridV1,
        policy: &AdmissionPolicyV1,
    ) -> CompletePopulationAuthorityV1<'a> {
        CompletePopulationAuthorityV1 {
            instrument_family: InstrumentFamilyV1::Nifty,
            rung_seconds: 60,
            horizon: Horizon::bars(2).expect("two-bar horizon"),
            requested_span: span(),
            identities: identities(bars, column, long, short, policy),
            signal_calendar: complete_calendar(60),
            execution_calendar: complete_calendar(60),
            admission_policy: *policy,
            long_exit_grid: long,
            short_exit_grid: short,
            execution_series: series,
            execution_column: column,
        }
    }

    fn evaluated_grid(
        resolved: &ResolvedExitGridV1,
        series: ExecutionSeriesV1<'_>,
        column: &Column,
        bars: &[indicators::Candle],
        ladder: Ladder,
        words: [u64; 6],
        direction: TradeDirectionV1,
    ) -> EvaluatedExitGridV1 {
        let run = execution_run(series.instrument(), bars, ladder, words, direction)
            .expect("directional run fixture");
        resolved
            .evaluate_training_grid_attested(
                series,
                column,
                Horizon::bars(2).expect("two-bar horizon"),
                run,
            )
            .expect("complete evaluated grid fixture")
    }

    fn candidate() -> EvaluatedPopulationCandidateV1 {
        EvaluatedPopulationCandidateV1 {
            strategy_digest: [9; 32],
            mask_words: [1, 0, 0, 0, 0, 0],
            direction: TradeDirectionV1::Long,
            closure: ClosureV1::Closed,
            support_hits: 100,
            exit: ExitCoordinateV1 {
                stop: Some(0),
                target: Some(2),
                tsl: Some(1),
                ttp: None,
            },
            metrics: TopMetricsV1 {
                drawdown: 10_000,
                worst_loss: 2_000,
                losing_rate_ppm: 400_000,
                losing_trades: 20,
                loss_ratio_ppm: Some(500_000),
                pessimistic_profit: 1_000,
                winning_trades: 30,
                win_rate_ppm: 600_000,
                reward_to_risk_ppm: Some(2_000_000),
                average_win: 300,
                average_loss: 150,
                assurance_ppm: 550_000,
            },
            evidence: evidence(),
        }
    }

    #[test]
    fn direct_population_evidence_is_exact_or_refused() {
        let candidate = candidate();
        validate_direct_evidence(0, &candidate).expect("exact direct evidence agrees");

        let mut values = candidate.evidence.values();
        values.drawdown_paisa = ObservedU64V1::Measured(9_999);
        let mut mismatched = candidate;
        mismatched.evidence = AdmissionEvidenceV1::new(values).expect("value remains in-domain");
        let refusal = validate_direct_evidence(7, &mismatched).expect_err("mismatch must refuse");
        assert!(refusal.contains("candidate 7 evidence drawdown_paisa=9999"));
    }

    #[test]
    fn explicit_unmeasured_metric_is_not_replaced_by_numeric_row_value() {
        let mut values = evidence().values();
        values.worst_reward_risk_ppm = ObservedU64V1::Unmeasured;
        let mut candidate = candidate();
        candidate.evidence = AdmissionEvidenceV1::new(values).expect("state remains explicit");
        validate_direct_evidence(0, &candidate)
            .expect("an explicit absence is left for canonical policy evaluation");
    }

    #[test]
    fn complete_v4_claim_cannot_hide_incomplete_candidate_coverage() {
        let mut values = evidence().values();
        values.execution_complete = CompletenessV1::Unmeasured;
        let mut incomplete = candidate();
        incomplete.evidence = AdmissionEvidenceV1::new(values).expect("state remains explicit");
        let refusal = validate_direct_evidence(0, &incomplete).expect_err("must fail closed");
        assert!(refusal.contains("execution_complete is Unmeasured"));
    }

    #[test]
    fn undefined_ranking_ratio_cannot_hide_measured_admission_ratio() {
        let mut candidate = candidate();
        candidate.metrics.reward_to_risk_ppm = None;
        let refusal = validate_direct_evidence(0, &candidate).expect_err("must fail closed");
        assert!(refusal.contains("ranking metric is undefined"));
    }

    #[test]
    fn commit_coordinator_reopens_joined_authority_and_exact_rerun_reuses_both_receipts() {
        let store_root = root("commit-reopen-reuse");
        let bars = runner::synthetic::sessions(8);
        let instrument = instrument("NIFTY");
        let mut evaluator = evaluator();
        let column = Column::build(&bars, &mut evaluator);
        let (series, long_grid, short_grid) = resolved_grids(&instrument, &bars);
        let policy = admission_policy();
        let authority = authority(&bars, &column, series, &long_grid, &short_grid, &policy);
        let population_id = derive_population_id_v1(&authority).expect("canonical population id");
        let mut long = candidate();
        long.strategy_digest = [9; 32];
        let mut short = long;
        short.strategy_digest = [10; 32];
        short.direction = TradeDirectionV1::Short;
        let candidates = [long, short];
        let request = PopulationAdmissionWriteRequestV1 {
            population_id,
            instrument_family: authority.instrument_family,
            rung_seconds: authority.rung_seconds,
            requested_span: authority.requested_span,
            reconciliation: CompletionReconciliationV2 {
                sweep_trials: 3,
                frequent_itemsets: 1,
                infrequent_itemsets: 2,
                closed_itemsets: 1,
                redundant_itemsets: 0,
                unknown_closure_itemsets: 0,
                exit_cells_per_mask: ExitCellsPerMaskV2::new(1, 1)
                    .expect("one exact cell on each side"),
                extinction_depth: 2,
                extinction_complete: true,
                closure_complete: true,
            },
            identities: authority.identities,
            signal_calendar: authority.signal_calendar,
            execution_calendar: authority.execution_calendar,
            admission_policy: policy,
            candidates: &candidates,
        };

        let first =
            commit_population_admission_v1(&store_root, &request).expect("first two-ledger commit");
        assert_eq!(first.population, PopulationCommit::Written);
        assert!(matches!(
            first.admission,
            AdmissionCommitOutcomeV1::Appended(_)
        ));

        let mut joined = AdmissionAuthoritativeLedger::open_read(&store_root, population_id)
            .expect("both receipt-last ledgers join");
        let page = joined.page(0, 2).expect("exact joined page");
        assert_eq!(page.total, 2);
        assert_eq!(page.offset, 0);
        assert_eq!(page.rows.len(), 2);
        assert_eq!(
            page.rows
                .first()
                .expect("long joined row")
                .population
                .direction,
            TradeDirectionV1::Long
        );
        assert_eq!(
            page.rows
                .get(1)
                .expect("short joined row")
                .population
                .direction,
            TradeDirectionV1::Short
        );
        assert_eq!(
            page.authority.population_v4_completion_digest,
            first.population_v4_completion_digest
        );
        drop(joined);

        let second = commit_population_admission_v1(&store_root, &request)
            .expect("byte-identical rerun is safe");
        assert_eq!(second.population, PopulationCommit::Reused);
        assert!(matches!(
            second.admission,
            AdmissionCommitOutcomeV1::Reused(_)
        ));
        assert_eq!(
            second.population_v4_completion_digest,
            first.population_v4_completion_digest
        );
    }

    #[test]
    fn population_identity_binds_every_supplied_authority_term() {
        let bars = runner::synthetic::sessions(8);
        let instrument = instrument("NIFTY");
        let mut evaluator = evaluator();
        let column = Column::build(&bars, &mut evaluator);
        let (series, long, short) = resolved_grids(&instrument, &bars);
        let policy = admission_policy();
        let base_authority = authority(&bars, &column, series, &long, &short, &policy);
        let base = derive_population_id_v1(&base_authority).expect("base identity");

        let mut changed = base_authority;
        changed.horizon = Horizon::bars(3).expect("three-bar horizon");
        assert_ne!(
            derive_population_id_v1(&changed).expect("changed horizon remains valid"),
            base
        );
        changed = base_authority;
        changed.requested_span =
            RequestedSpanIdentityV1::new(2026, 8, 2026, 8).expect("next month");
        assert!(derive_population_id_v1(&changed).is_err());
        changed = base_authority;
        changed.rung_seconds = 120;
        changed.signal_calendar = complete_calendar(120);
        assert_ne!(
            derive_population_id_v1(&changed).expect("changed signal rung remains valid"),
            base
        );

        let identity_changes = [
            ("run", [11; 32]),
            ("data", [12; 32]),
            ("vocabulary", [13; 32]),
            ("evaluation", [14; 32]),
            ("ranking", [15; 32]),
            ("daily reference", [16; 32]),
        ];
        for (name, digest) in identity_changes {
            changed = base_authority;
            match name {
                "run" => changed.identities.run_identity = digest,
                "data" => changed.identities.data_digest = digest,
                "vocabulary" => changed.identities.vocabulary_digest = digest,
                "evaluation" => changed.identities.evaluation_policy_digest = digest,
                "ranking" => changed.identities.ranking_policy_digest = digest,
                "daily reference" => changed.identities.daily_reference_policy_digest = digest,
                _ => panic!("fixture named an unknown identity term"),
            }
            assert_ne!(
                derive_population_id_v1(&changed)
                    .expect("changed independent identity remains valid"),
                base,
                "{name} identity was not load-bearing"
            );
        }

        for (name, mutate) in [
            ("feed", 1_u8),
            ("commit", 2),
            ("calendar", 3),
            ("long policy", 4),
            ("long resolution", 5),
            ("short policy", 6),
            ("short resolution", 7),
        ] {
            changed = base_authority;
            let digest = [mutate; 32];
            match name {
                "feed" => changed.identities.feed_digest = digest,
                "commit" => changed.identities.source_commit_digest = digest,
                "calendar" => changed.identities.calendar_policy_digest = digest,
                "long policy" => changed.identities.exit_grids.long.policy_digest = digest,
                "long resolution" => changed.identities.exit_grids.long.resolved_digest = digest,
                "short policy" => changed.identities.exit_grids.short.policy_digest = digest,
                "short resolution" => changed.identities.exit_grids.short.resolved_digest = digest,
                _ => panic!("fixture named an unknown checked identity term"),
            }
            assert!(
                derive_population_id_v1(&changed).is_err(),
                "foreign {name} identity must refuse rather than collide"
            );
        }

        changed = base_authority;
        changed.admission_policy = admission_policy_with_support(2);
        assert!(derive_population_id_v1(&changed).is_err());
        changed = base_authority;
        changed.short_exit_grid = &long;
        assert!(derive_population_id_v1(&changed).is_err());
    }

    #[test]
    fn strategy_identity_binds_mask_side_and_exact_exit_coordinate() {
        let bars = runner::synthetic::sessions(8);
        let instrument = instrument("NIFTY");
        let mut evaluator = evaluator();
        let column = Column::build(&bars, &mut evaluator);
        let (series, long, short) = resolved_grids(&instrument, &bars);
        let ladder = Ladder::with_min_hits(600).with_ceiling(50_000);
        let first_mask = [1, 0, 0, 0, 0, 0];
        let second_mask = [2, 0, 0, 0, 0, 0];
        let long_first = evaluated_grid(
            &long,
            series,
            &column,
            &bars,
            ladder,
            first_mask,
            TradeDirectionV1::Long,
        );
        let long_second = evaluated_grid(
            &long,
            series,
            &column,
            &bars,
            ladder,
            second_mask,
            TradeDirectionV1::Long,
        );
        let short_first = evaluated_grid(
            &short,
            series,
            &column,
            &bars,
            ladder,
            first_mask,
            TradeDirectionV1::Short,
        );
        assert!(long_first.grid().cells.get(1).is_some());
        let population_id = [0xB1; 32];
        let evaluation_policy = [0xB2; 32];
        let base = derive_strategy_digest_v1(
            population_id,
            InstrumentFamilyV1::Nifty,
            60,
            evaluation_policy,
            TradeDirectionV1::Long,
            &long,
            &long_first,
            0,
        )
        .expect("base strategy identity");
        let coordinate = derive_strategy_digest_v1(
            population_id,
            InstrumentFamilyV1::Nifty,
            60,
            evaluation_policy,
            TradeDirectionV1::Long,
            &long,
            &long_first,
            1,
        )
        .expect("second coordinate identity");
        let mask = derive_strategy_digest_v1(
            population_id,
            InstrumentFamilyV1::Nifty,
            60,
            evaluation_policy,
            TradeDirectionV1::Long,
            &long,
            &long_second,
            0,
        )
        .expect("second mask identity");
        let side = derive_strategy_digest_v1(
            population_id,
            InstrumentFamilyV1::Nifty,
            60,
            evaluation_policy,
            TradeDirectionV1::Short,
            &short,
            &short_first,
            0,
        )
        .expect("short identity");
        assert_ne!(base, coordinate);
        assert_ne!(base, mask);
        assert_ne!(base, side);
        assert!(
            derive_strategy_digest_v1(
                population_id,
                InstrumentFamilyV1::Nifty,
                60,
                evaluation_policy,
                TradeDirectionV1::Short,
                &long,
                &long_first,
                0,
            )
            .is_err()
        );
    }

    #[test]
    fn uncapped_producer_expands_closed_masks_long_then_short_at_exact_count() {
        let bars = runner::synthetic::sessions(8);
        let instrument = instrument("NIFTY");
        let mut evaluator = evaluator();
        let column = Column::build(&bars, &mut evaluator);
        let (series, long, short) = resolved_grids(&instrument, &bars);
        let policy = admission_policy();
        let authority = authority(&bars, &column, series, &long, &short, &policy);
        let signal_column = column.clone();
        let ladder = Ladder::with_min_hits(600).with_ceiling(50_000);
        let mut evidence_order = Vec::new();
        let produced = produce_population_admission_v1(
            &Sweeper::new(ladder),
            signal_column,
            authority,
            &|_, _, _| {},
            |words, direction| execution_run(&instrument, &bars, ladder, words, direction),
            |context| {
                evidence_order.push((context.mask_words, context.direction, context.exit));
                Ok(cell_evidence(&context))
            },
        )
        .expect("complete uncapped producer fixture");
        let rows = produced.prepared().rows();
        assert!(produced.population_run().is_complete());
        assert!(produced.population_run().closed > 0);
        assert_eq!(rows.len(), evidence_order.len());
        let per_mask_u64 = long
            .cell_count()
            .checked_add(short.cell_count())
            .expect("per-mask count fits u64");
        let expected_u64 = produced
            .population_run()
            .closed
            .checked_mul(per_mask_u64)
            .expect("complete row count fits u64");
        assert_eq!(
            u64::try_from(rows.len()).expect("row count fits u64"),
            expected_u64
        );
        let per_mask = usize::try_from(per_mask_u64).expect("small fixture grid");
        let long_cells = usize::try_from(long.cell_count()).expect("small long grid");
        for chunk in rows.chunks_exact(per_mask) {
            let (long_rows, short_rows) = chunk.split_at(long_cells);
            assert!(
                long_rows
                    .iter()
                    .all(|row| row.direction == TradeDirectionV1::Long)
            );
            assert!(
                short_rows
                    .iter()
                    .all(|row| row.direction == TradeDirectionV1::Short)
            );
            assert!(chunk.iter().all(|row| row.closure == ClosureV1::Closed));
            let mask = chunk.first().expect("non-empty per-mask chunk").mask_words;
            assert!(chunk.iter().all(|row| row.mask_words == mask));
        }
        for (sequence, (row, observed)) in rows.iter().zip(evidence_order).enumerate() {
            assert_eq!(
                row.sequence,
                u64::try_from(sequence).expect("sequence fits")
            );
            assert_eq!((row.mask_words, row.direction, row.exit), observed);
        }
    }

    #[test]
    fn halted_unknown_population_refuses_before_evidence_finalization() {
        let bars = runner::synthetic::sessions(8);
        let instrument = instrument("NIFTY");
        let mut evaluator = evaluator();
        let column = Column::build(&bars, &mut evaluator);
        let (series, long, short) = resolved_grids(&instrument, &bars);
        let policy = admission_policy();
        let authority = authority(&bars, &column, series, &long, &short, &policy);
        let signal_column = column.clone();
        let ladder = Ladder::with_min_hits(600).with_ceiling(1);
        let mut evidence_calls = 0_u64;
        let refusal = produce_population_admission_v1(
            &Sweeper::new(ladder),
            signal_column,
            authority,
            &|_, _, _| {},
            |words, direction| execution_run(&instrument, &bars, ladder, words, direction),
            |context| {
                evidence_calls = evidence_calls.saturating_add(1);
                Ok(cell_evidence(&context))
            },
        )
        .expect_err("halted/unknown closure cannot become complete evidence");
        assert_eq!(evidence_calls, 0);
        assert!(
            refusal.contains("without a terminal closure verdict")
                || refusal.contains("uncapped population is incomplete")
        );
    }

    #[test]
    fn foreign_grid_and_wrong_side_execution_run_are_refused() {
        let bars = runner::synthetic::sessions(8);
        let instrument = instrument("NIFTY");
        let mut evaluator = evaluator();
        let column = Column::build(&bars, &mut evaluator);
        let (series, long, short) = resolved_grids(&instrument, &bars);
        let policy = admission_policy();
        let base = authority(&bars, &column, series, &long, &short, &policy);
        let mut copied_side = base;
        copied_side.short_exit_grid = &long;
        let refusal = derive_population_id_v1(&copied_side)
            .expect_err("one long grid cannot authorize the short side");
        assert!(refusal.contains("short exit-grid resolution carries the opposite side"));

        let ladder = Ladder::with_min_hits(600).with_ceiling(50_000);
        let words = [1, 0, 0, 0, 0, 0];
        let wrong_side = execution_run(&instrument, &bars, ladder, words, TradeDirectionV1::Long)
            .expect("well-formed but wrong-side run");
        assert!(
            short
                .evaluate_training_grid_attested(
                    series,
                    &column,
                    Horizon::bars(2).expect("two-bar horizon"),
                    wrong_side,
                )
                .is_err()
        );
    }

    #[test]
    fn mandatory_statistical_evidence_refusal_exposes_no_partial_population() {
        let bars = runner::synthetic::sessions(8);
        let instrument = instrument("NIFTY");
        let mut evaluator = evaluator();
        let column = Column::build(&bars, &mut evaluator);
        let (series, long, short) = resolved_grids(&instrument, &bars);
        let policy = admission_policy();
        let authority = authority(&bars, &column, series, &long, &short, &policy);
        let signal_column = column.clone();
        let ladder = Ladder::with_min_hits(600).with_ceiling(50_000);
        let mut evidence_calls = 0_u64;
        let refusal = produce_population_admission_v1(
            &Sweeper::new(ladder),
            signal_column,
            authority,
            &|_, _, _| {},
            |words, direction| execution_run(&instrument, &bars, ladder, words, direction),
            |context| {
                assert!(context.population_run.is_complete());
                assert!(context.reconciliation.extinction_complete);
                evidence_calls = evidence_calls.saturating_add(1);
                Err("full statistical evidence unavailable".to_owned())
            },
        )
        .expect_err("mandatory evidence refusal withholds the produced population");
        assert_eq!(evidence_calls, 1);
        assert!(refusal.contains("full statistical evidence unavailable"));
    }
}
