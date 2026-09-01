//! Atomic success boundary for Candidate Universe V1 and Pre-Admission Data V1.
//!
//! The public entry point begins with a store root, feed, sweepable underlying,
//! rung and inclusive month span. It admits all three market-data streams through
//! bounded stored readers, constructs complete calendars from the exact admitted
//! bars, derives the requested one-minute execution subspan, resolves both exit
//! grids from that attested series, and only then mints the crate-private
//! [`CandidateUniverseProductionSourceV1`]. No caller can supply bars, stream
//! digests, calendar receipts, a commit string or a pre-resolved grid.
//!
//! A successful call means Candidate, same-pass Base Evidence and
//! Pre-Admission receipt-last appends all reopened with the exact cross-ledger
//! identity. Candidate persistence may have completed before a later Base or
//! Pre-Admission refusal; that is a recoverable append-only prefix, not a
//! claimed success. Retrying the same typed source reuses exact completed bytes
//! and finishes the remaining authority chain.

use std::fs::{self, File};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use brutex_core::blake3::hash;
use brutex_core::instrument::{Exchange, InstrumentKey};
use brutex_core::vendor::Vendor;
use indicators::Candle;
use indicators::evaluator::{CHARTER_NON_REGULAR_IST_DAYS, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use pull::session::Day;
use runner::Sweeper;
use runner::admission::AdmissionPolicyV1;
use runner::exit_grid_policy::{
    ExecutionSeriesV1, ExitGridPolicyV1, InstrumentFamilyV1 as ExitInstrumentFamilyV1,
    ResolvedExitGridV1,
};
use runner::identity::{DailyReferenceBinding, ReferenceIntegrity};
use runner::outcome::Horizon;
use runner::validate::{AnchoredSearchAuthorityProjectionV4, AnchoredSearchValidationV4};

use crate::anchored_search_lineage_v4::{
    AnchoredSearchLineageMemberAuthorityV4, AnchoredSearchLineageV4Bounds,
    AuthenticatedAnchoredSearchLineageV4, persist_anchored_search_lineage_v4,
};
use crate::candidate_universe::{
    AuthenticatedCandidatePopulationRowV1, BaseEvidenceLedgerBoundsV2, BaseEvidenceLedgerReaderV2,
    BaseEvidenceReopenAuditV2, CandidateExecutionReplayAuthorityV1, CandidateUniverseBoundsV1,
    CandidateUniverseLedgerV1, CandidateUniverseProductionSourceV1, CandidateUniverseReceiptV1,
    CandidateUniverseReopenAuditV1, PairedBaseEvidenceAuthorityV2, PairedBaseEvidenceReaderV2,
    PairedBaseEvidenceRecordProjectionV2, pair_candidate_base_evidence_v2,
    produce_candidate_universe_v1,
};
use crate::population::{
    InstrumentFamilyV1, LongShortExitGridIdentitiesV2, RequestedSpanIdentityV1,
    SideExitGridIdentityV2,
};
use crate::population_admission_v3::{
    AdmissionV3Bounds, AdmissionV3Family, PopulationAdmissionV3Authority,
    PopulationAdmissionV3Commit, PopulationAdmissionV3DecisionProjection,
    PopulationAdmissionV3SuccessorProjection, commit_population_admission_v3_exact_grid,
};
use crate::population_finalization_v3::{
    PopulationFinalizationV3Authority, PopulationFinalizationV3Bounds,
    PopulationFinalizationV3RowProjection, PopulationFinalizationV3StructuralReceipt,
    commit_population_finalization_v3,
};
use crate::population_observations_v1::{
    CandidateFamilyObservationsV1, ObservationAuthorityBoundsV1, ObservationAuthorityCommitV1,
    PairedCandidateObservationsV1,
};
use crate::population_statistics_v2::{
    PopulationStatisticsAdmissionCandidateV3, PopulationStatisticsAdmissionProjectionV3,
    PopulationStatisticsObservationCommitV2, PopulationStatisticsProcedureV2,
    PopulationStatisticsV2Bounds, PopulationStatisticsV2Ledger,
    PopulationStatisticsV2ProjectionSource, prepare_population_statistics_v2_from_observations,
};
use crate::pre_admission_data::{
    PreAdmissionDataBoundsV1, PreAdmissionDataReopenAuditV1, PreAdmissionDataV1,
    produce_pre_admission_data_v1,
};
use crate::stored::{
    CalendarReceiptV2, CompleteCalendarReceiptV2, DAILY_ELIGIBILITY_POLICY, DAILY_REFERENCE_SCHEMA,
    DailyContext, EXACT_MINUTE_GAP_POLICY, ExactMinuteContext, Span, StoredSpanLoadBoundV1,
};
use crate::stored_post_training_oos::{
    StoredPostTrainingOosCohortV1, StoredPostTrainingOosRequestV1,
};

/// A named refusal from one stage of the Candidate-to-Pre-Admission transaction.
pub type Step3OrchestratorRefusal = String;

/// All explicit resource ceilings used by one stored Step-3 transaction.
///
/// There is no `Default`: callers must name load, Candidate-ledger and
/// Pre-Admission-ledger limits independently. Base Evidence is exactly one
/// record per Candidate row and one completion per Candidate universe, so its
/// ceilings are derived one-for-one from the Candidate limits rather than
/// admitting a contradictory fourth caller value. The three load ceilings are
/// enforced by stored file headers before record allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredCandidatePreAdmissionBoundsV1 {
    /// Maximum signal-rung records admitted across the requested month span.
    pub signal_records: StoredSpanLoadBoundV1,
    /// Maximum exact-minute records admitted across warm-up plus request span.
    pub minute_records: StoredSpanLoadBoundV1,
    /// Maximum one-day records admitted across warm-up plus request span.
    pub daily_records: StoredSpanLoadBoundV1,
    /// Candidate row/completion ledger ceilings.
    pub candidate: CandidateUniverseBoundsV1,
    /// Pre-Admission logical-row and byte ceilings.
    pub pre_admission: PreAdmissionDataBoundsV1,
}

/// Exact stored authority and evaluation policy for one Step-3 transaction.
///
/// The request contains no raw market bytes, digest, calendar receipt, source
/// commit, depth, fallback or pre-resolved result. `underlying` is validated by
/// the canonical stored loader against the two-instrument sweep surface, and
/// `rung_name` is resolved by the existing stored timeframe authority.
#[derive(Clone, Copy)]
pub struct StoredCandidatePreAdmissionRequestV1<'a> {
    /// Existing market-data and append-only Step-3 ledger root.
    pub root: &'a Path,
    /// Feed whose exact stored bytes are authoritative.
    pub vendor: Vendor,
    /// Canonical spot-index underlying (`NIFTY` or `BANKNIFTY`).
    pub underlying: &'a str,
    /// Canonical stored signal rung name.
    pub rung_name: &'a str,
    /// Inclusive first requested `(year, month)`.
    pub from: (u16, u8),
    /// Inclusive last requested `(year, month)`.
    pub to: (u16, u8),
    /// Naturally-extinct Apriori sweeper; there is no depth parameter.
    pub sweeper: &'a Sweeper,
    /// Explicit forward outcome horizon.
    pub horizon: Horizon,
    /// Explicit indicator tolerance widths.
    pub widths: Widths,
    /// Explicit volume/VWAP availability decision.
    pub availability: Availability,
    /// Explicit candlestick predicate thresholds.
    pub thresholds: Thresholds,
    /// Complete long-side dynamic exit-grid policy.
    pub long_exit_policy: &'a ExitGridPolicyV1,
    /// Complete short-side dynamic exit-grid policy.
    pub short_exit_policy: &'a ExitGridPolicyV1,
    /// Every pre-allocation and append-only ledger ceiling.
    pub bounds: StoredCandidatePreAdmissionBoundsV1,
}

/// Only the immutable identities recovered after both ledgers reopen.
///
/// No prepared row, writable ledger, caller-authored digest, source slice or
/// statistics claim escapes the transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidatePreAdmissionReopenIdentitiesV1 {
    candidate_universe_id: [u8; 32],
    candidate_completion_digest: [u8; 32],
    candidate_row_count: u64,
    pre_admission_authority_id: [u8; 32],
}

impl CandidatePreAdmissionReopenIdentitiesV1 {
    /// Candidate Universe V1 identity recovered from its reopened receipt.
    #[must_use]
    pub const fn candidate_universe_id(self) -> [u8; 32] {
        self.candidate_universe_id
    }

    /// Full Candidate completion/content digest recovered after reopen.
    #[must_use]
    pub const fn candidate_completion_digest(self) -> [u8; 32] {
        self.candidate_completion_digest
    }

    /// Exact completed Candidate row count bound by both reopened ledgers.
    #[must_use]
    pub const fn candidate_row_count(self) -> u64 {
        self.candidate_row_count
    }

    /// Pre-Admission V1 semantic authority recovered from its reopened pair.
    #[must_use]
    pub const fn pre_admission_authority_id(self) -> [u8; 32] {
        self.pre_admission_authority_id
    }
}

/// Exact reopened Candidate/Base/Pre-Admission chain plus sealed observations.
///
/// Public callers cannot construct this value: the only producer accepts the
/// crate-private Candidate source capability.  Keeping the observation family
/// beside the two reopen audits prevents the Statistics V2 successor from
/// re-reading or reinterpreting Candidate V1 bytes.
pub(crate) struct CommittedCandidatePreAdmissionV1 {
    candidate_audit: CandidateUniverseReopenAuditV1,
    candidate_bounds: CandidateUniverseBoundsV1,
    base_evidence_audit: BaseEvidenceReopenAuditV2,
    base_evidence_bounds: BaseEvidenceLedgerBoundsV2,
    pre_admission_audit: PreAdmissionDataReopenAuditV1,
    observations: CandidateFamilyObservationsV1,
    prepared_candidate_receipt: CandidateUniverseReceiptV1,
    prepared_pre_admission: PreAdmissionDataV1,
}

impl CommittedCandidatePreAdmissionV1 {
    /// Freshly reopened Candidate V1 audit.
    #[must_use]
    pub(crate) const fn candidate_audit(&self) -> CandidateUniverseReopenAuditV1 {
        self.candidate_audit
    }

    /// Exact Candidate ledger ceilings admitted by the production transaction.
    ///
    /// Retaining these bounds lets a later Population successor reopen the
    /// same Candidate bytes without inventing wider limits after the fact.
    #[must_use]
    pub(crate) const fn candidate_bounds(&self) -> CandidateUniverseBoundsV1 {
        self.candidate_bounds
    }

    /// Same-pass Base Evidence V2 authority appended, synced and freshly
    /// reopened against this exact Candidate completion.
    #[must_use]
    pub(crate) const fn base_evidence_audit(&self) -> &BaseEvidenceReopenAuditV2 {
        &self.base_evidence_audit
    }

    /// Explicit Base ledger ceilings inherited one-for-one from Candidate.
    #[must_use]
    pub(crate) const fn base_evidence_bounds(&self) -> BaseEvidenceLedgerBoundsV2 {
        self.base_evidence_bounds
    }

    /// Freshly reopened Pre-Admission V1 audit for that exact Candidate.
    #[must_use]
    pub(crate) const fn pre_admission_audit(&self) -> PreAdmissionDataReopenAuditV1 {
        self.pre_admission_audit
    }

    /// Sealed per-session observations produced in the same Candidate fold.
    #[must_use]
    pub(crate) const fn observations(&self) -> &CandidateFamilyObservationsV1 {
        &self.observations
    }

    const fn prepared_candidate_receipt(&self) -> &CandidateUniverseReceiptV1 {
        &self.prepared_candidate_receipt
    }

    const fn prepared_pre_admission(&self) -> &PreAdmissionDataV1 {
        &self.prepared_pre_admission
    }

    /// Existing public identity-only projection.
    #[must_use]
    pub(crate) const fn identities(&self) -> CandidatePreAdmissionReopenIdentitiesV1 {
        CandidatePreAdmissionReopenIdentitiesV1 {
            candidate_universe_id: self.candidate_audit.universe_id(),
            candidate_completion_digest: self.candidate_audit.content_digest(),
            candidate_row_count: self.candidate_audit.row_count(),
            pre_admission_authority_id: self.pre_admission_audit.authority_id(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ReopenedJoinFactsV1 {
    universe_id: [u8; 32],
    completion_digest: [u8; 32],
    row_count: u64,
}

/// Build-derived source authority unavailable to public callers.
///
/// Its field is private and the production constructor reads only the process-
/// free build stamp frozen into this binary. Tests in this module can exercise
/// the post-stamp transaction without widening the public API to a caller-
/// authored commit string.
#[derive(Clone, Copy)]
struct VerifiedBuildCommitV1<'a>(&'a str);

impl VerifiedBuildCommitV1<'static> {
    fn current() -> Result<Self, Step3OrchestratorRefusal> {
        crate::commit_stamp().map(Self).ok_or_else(|| {
            "Step 3 stored orchestration refused before opening market data: this binary has no clean canonical build commit stamp"
                .to_owned()
        })
    }
}

/// One canonical directory pathname plus an open identity capability.
///
/// Every downstream pathname is rooted at `canonical`, never at the caller's
/// spelling. The held directory descriptor and the current canonical pathname
/// must continue to name the admitted device/inode pair at every stage
/// boundary. This catches deterministic rename-and-replace substitution.
///
/// The existing store and ledger APIs are pathname based, not `openat`/dirfd
/// based. Consequently there is an irreducible same-process/OS race inside one
/// individual pathname call: an attacker able to rename directories between a
/// pre-check and that call can make the call fail or touch the replacement
/// before the post-check refuses the transaction. Success is never returned
/// after a detected substitution, but atomic confinement would require those
/// lower APIs to accept this directory capability directly.
pub(crate) struct AdmittedRootV1 {
    canonical: PathBuf,
    directory: File,
    generation: [u64; 2],
}

impl AdmittedRootV1 {
    pub(crate) fn admit(root: &Path) -> Result<Self, Step3OrchestratorRefusal> {
        let canonical = fs::canonicalize(root).map_err(|why| {
            format!(
                "Step 3 root admission could not canonicalize {}: {why}",
                root.display()
            )
        })?;
        let path_metadata = fs::metadata(&canonical).map_err(|why| {
            format!(
                "Step 3 root admission could not inspect {}: {why}",
                canonical.display()
            )
        })?;
        if !path_metadata.is_dir() {
            return Err(format!(
                "Step 3 root admission refused {} because it is not a directory",
                canonical.display()
            ));
        }
        let generation = directory_identity_v1(&path_metadata)?;
        let directory = File::open(&canonical).map_err(|why| {
            format!(
                "Step 3 root admission could not hold directory capability {}: {why}",
                canonical.display()
            )
        })?;
        let held_generation = directory
            .metadata()
            .map_err(|why| format!("Step 3 held root capability metadata refused: {why}"))
            .and_then(|metadata| directory_identity_v1(&metadata))?;
        if held_generation != generation {
            return Err(
                "Step 3 root changed between canonical pathname admission and capability open"
                    .to_owned(),
            );
        }
        Ok(Self {
            canonical,
            directory,
            generation,
        })
    }

    pub(crate) fn path(&self) -> &Path {
        self.canonical.as_path()
    }

    pub(crate) fn require_same(&self, stage: &str) -> Result<(), Step3OrchestratorRefusal> {
        let held = self
            .directory
            .metadata()
            .map_err(|why| format!("Step 3 held root capability failed at {stage}: {why}"))
            .and_then(|metadata| directory_identity_v1(&metadata))?;
        if held != self.generation {
            return Err(format!(
                "Step 3 held root directory generation changed at {stage}"
            ));
        }
        let recanonicalized = fs::canonicalize(&self.canonical).map_err(|why| {
            format!("Step 3 canonical root disappeared or became unreachable at {stage}: {why}")
        })?;
        if recanonicalized != self.canonical {
            return Err(format!(
                "Step 3 canonical root pathname resolved elsewhere at {stage}"
            ));
        }
        let current = fs::metadata(&self.canonical)
            .map_err(|why| format!("Step 3 canonical root metadata failed at {stage}: {why}"))
            .and_then(|metadata| directory_identity_v1(&metadata))?;
        if current != self.generation {
            return Err(format!(
                "Step 3 canonical root was substituted at {stage}; the held directory capability and pathname no longer agree"
            ));
        }
        Ok(())
    }

    /// Stable admitted device/inode bytes used only inside opaque successor
    /// seals; the pathname and raw generation fields do not escape.
    pub(crate) fn generation_bytes(&self) -> [u8; 16] {
        let mut bytes = [0_u8; 16];
        bytes[..8].copy_from_slice(&self.generation[0].to_le_bytes());
        bytes[8..].copy_from_slice(&self.generation[1].to_le_bytes());
        bytes
    }
}

#[cfg(unix)]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the non-Unix implementation must return an explicit unsupported-target refusal"
)]
fn directory_identity_v1(metadata: &fs::Metadata) -> Result<[u64; 2], Step3OrchestratorRefusal> {
    use std::os::unix::fs::MetadataExt;

    Ok([metadata.dev(), metadata.ino()])
}

#[cfg(not(unix))]
fn directory_identity_v1(_metadata: &fs::Metadata) -> Result<[u64; 2], Step3OrchestratorRefusal> {
    Err(
        "Step 3 root admission requires an OS directory-identity capability; this target exposes no supported device/inode API"
            .to_owned(),
    )
}

pub(crate) struct BoundedStoredContextV1 {
    pub(crate) requested_span: RequestedSpanIdentityV1,
    pub(crate) first_day: i64,
    pub(crate) last_day: i64,
    pub(crate) rung_seconds: u32,
    pub(crate) signal_calendar: CompleteCalendarReceiptV2,
    pub(crate) signal: Span,
    pub(crate) daily: DailyContext,
    pub(crate) minute: ExactMinuteContext,
}

struct ResolvedExecutionContextV1<'a> {
    series: ExecutionSeriesV1<'a>,
    execution_range: Range<usize>,
    calendar: CompleteCalendarReceiptV2,
    family: InstrumentFamilyV1,
    long: ResolvedExitGridV1,
    short: ResolvedExitGridV1,
}

/// Exact, owned stored inputs retained behind the Step-3 transaction boundary.
///
/// Nothing in this type is public or caller-authored.  In particular, the raw
/// bars and source digests have no accessor.  They remain together so a later
/// in-module adapter can rebuild the causal anchored column and exact-minute
/// execution path without reopening the store or accepting a second feed,
/// instrument, calendar, rung, span, commit or policy spelling.
struct RetainedStoredExecutionContextV1 {
    stored: BoundedStoredContextV1,
    execution_range: Range<usize>,
    source_commit: String,
    family: InstrumentFamilyV1,
    sweeper_ladder: engine::Ladder,
    horizon: Horizon,
    widths: Widths,
    availability: Availability,
    thresholds: Thresholds,
    search_splits: usize,
    load_ceilings: StoredLoadCeilingsV1,
    long: ResolvedExitGridV1,
    short: ResolvedExitGridV1,
}

/// Nonconstructible retained Search V4 authority for one stored Candidate.
///
/// The Runner validation is accepted only after all five signal-source terms
/// and both Long/Short grid identities equal the freshly reopened Candidate
/// receipt. No constructor accepts a slice, digest, split count or projection.
#[derive(Clone)]
pub(crate) struct StoredSearchMemberV4 {
    candidate_universe_id: [u8; 32],
    candidate_completion_digest: [u8; 32],
    validation: Arc<AnchoredSearchValidationV4>,
}

impl StoredSearchMemberV4 {
    fn bind(
        committed: &CommittedCandidatePreAdmissionV1,
        validation: AnchoredSearchValidationV4,
    ) -> Result<Self, Step3OrchestratorRefusal> {
        let audit = committed.candidate_audit();
        let receipt = audit.receipt();
        let authority = validation
            .search_authority_projection()
            .map_err(|why| format!("Step 3 retained Search V4 reconciliation refused: {why:?}"))?;
        let source = authority.source_identity();
        let signal = receipt.signal_stream();
        if source.signal_digest() != signal.digest()
            || source.signal_bars() != signal.count()
            || source.signal_first_ts_micros() != signal.first_ts_micros()
            || source.signal_last_ts_micros() != signal.last_ts_micros()
            || source.signal_column_digest() != receipt.signal_column_digest()
        {
            return Err(
                "Step 3 retained Search V4 source differs from the freshly reopened Candidate receipt"
                    .to_owned(),
            );
        }
        let grids = receipt.identities().exit_grids();
        let search_long = authority.long_grid_identity();
        let search_short = authority.short_grid_identity();
        if search_long.policy_digest() != grids.long.policy_digest
            || search_long.resolution_digest() != grids.long.resolved_digest
            || search_short.policy_digest() != grids.short.policy_digest
            || search_short.resolution_digest() != grids.short.resolved_digest
        {
            return Err(
                "Step 3 retained Search V4 grids differ from the freshly reopened Candidate receipt"
                    .to_owned(),
            );
        }
        Ok(Self {
            candidate_universe_id: audit.universe_id(),
            candidate_completion_digest: audit.content_digest(),
            validation: Arc::new(validation),
        })
    }

    /// Candidate universe to which this exact Search V4 authority was joined.
    #[must_use]
    pub(crate) const fn candidate_universe_id(&self) -> [u8; 32] {
        self.candidate_universe_id
    }

    /// Candidate completion to which this exact Search V4 authority was joined.
    #[must_use]
    pub(crate) const fn candidate_completion_digest(&self) -> [u8; 32] {
        self.candidate_completion_digest
    }

    /// Reconciles and projects the retained opaque Search V4 result.
    pub(crate) fn projection(
        &self,
    ) -> Result<AnchoredSearchAuthorityProjectionV4, Step3OrchestratorRefusal> {
        self.validation
            .search_authority_projection()
            .map_err(|why| format!("Step 3 retained Search V4 projection refused: {why:?}"))
    }

    /// Opaque Runner result, exposed only inside the crate after stored binding.
    #[must_use]
    pub(crate) fn validation(&self) -> &AnchoredSearchValidationV4 {
        self.validation.as_ref()
    }
}

/// Owned, shallow-cloned NIFTY/BANKNIFTY Search V4 pair.
///
/// Private fields and the absence of a constructor prevent a caller from
/// substituting either family. `Arc` keeps the large opaque fold evidence
/// single-owned in memory while allowing Admission to seek mutable ledger
/// readers without retaining an immutable borrow of the orchestrator.
pub(crate) struct StoredSearchPairV4 {
    nifty: StoredSearchMemberV4,
    banknifty: StoredSearchMemberV4,
}

impl StoredSearchPairV4 {
    /// Canonical first family.
    #[must_use]
    pub(crate) const fn nifty(&self) -> &StoredSearchMemberV4 {
        &self.nifty
    }

    /// Canonical second family.
    #[must_use]
    pub(crate) const fn banknifty(&self) -> &StoredSearchMemberV4 {
        &self.banknifty
    }
}

/// The exact stored transaction result retained for the Step-3 successor.
///
/// The ordinary public entry point projects this value back to the four
/// historical identities.  Crate code that continues the same transaction can
/// instead retain the opaque Candidate/Pre-Admission/Observation authority and
/// its privately owned execution sources; neither half can be constructed from
/// decoded audit bytes alone.
pub(crate) struct CommittedStoredCandidatePreAdmissionV1 {
    root: AdmittedRootV1,
    committed: CommittedCandidatePreAdmissionV1,
    search: StoredSearchMemberV4,
    execution: RetainedStoredExecutionContextV1,
}

impl CommittedStoredCandidatePreAdmissionV1 {
    #[must_use]
    pub(crate) const fn candidate_pre_admission(&self) -> &CommittedCandidatePreAdmissionV1 {
        &self.committed
    }

    /// Reopens the exact Candidate ledger under the originally admitted
    /// resource ceilings and proves that its indexed Completion is unchanged.
    ///
    /// The returned ledger is read-only structural authority retained beneath
    /// this opaque stored capability. A later successor must still join its
    /// authenticated rows to Admission and Finalization; callers cannot obtain
    /// the reader or substitute a detached audit.
    fn open_candidate_reader(&self) -> Result<CandidateUniverseLedgerV1, Step3OrchestratorRefusal> {
        self.root
            .require_same("before Candidate successor read-only reopen")?;
        let ledger = CandidateUniverseLedgerV1::open_read(
            self.root.path(),
            self.committed.candidate_bounds(),
        )
        .map_err(|why| format!("Step 3 Candidate successor reopen refused: {why}"))?;
        let expected = self.committed.candidate_audit();
        let reopened = ledger
            .reopen_audit(&expected.universe_id())
            .map_err(|why| format!("Step 3 Candidate successor audit refused: {why}"))?;
        if reopened != Some(expected) {
            return Err(
                "Step 3 Candidate successor reopen differs from its retained Candidate authority"
                    .to_owned(),
            );
        }
        self.root
            .require_same("after Candidate successor read-only reopen")?;
        Ok(ledger)
    }

    /// Reads the complete literal Candidate records behind this retained
    /// stored authority for the Population successor join.
    ///
    /// The vector is minted only by the freshly reopened read-only ledger; each
    /// entry contains its sealed 480-byte record and shared Base V2 digest.
    /// This method accepts no detached audit, row or digest from a caller.
    pub(crate) fn authenticated_candidate_population_rows(
        &self,
    ) -> Result<Vec<AuthenticatedCandidatePopulationRowV1>, Step3OrchestratorRefusal> {
        self.root
            .require_same("before complete Candidate successor projection")?;
        let ledger = self.open_candidate_reader()?;
        let audit = self.committed.candidate_audit();
        let rows = ledger
            .complete_population_rows(&audit)
            .map_err(|why| format!("Step 3 complete Candidate successor read refused: {why}"))?;
        if u64::try_from(rows.len()).ok() != Some(audit.row_count()) {
            return Err(
                "Step 3 complete Candidate successor cardinality differs from its retained Completion"
                    .to_owned(),
            );
        }
        self.root
            .require_same("after complete Candidate successor projection")?;
        Ok(rows)
    }

    /// Rebuilds exact Runner terminal dispositions from this retained stored
    /// source and its freshly authenticated Candidate bytes.
    pub(crate) fn execution_v3_replay_authority(
        &self,
    ) -> Result<CandidateExecutionReplayAuthorityV1, Step3OrchestratorRefusal> {
        self.root
            .require_same("before Candidate Execution V3 replay authentication")?;
        let rows = self.authenticated_candidate_population_rows()?;
        let authority = self.execution.execution_v3_replay(&self.committed, &rows)?;
        self.root
            .require_same("after Candidate Execution V3 replay authentication")?;
        Ok(authority)
    }

    /// Loads one later civil span through this exact retained stored source and
    /// returns an opaque post-training OOS capability.
    ///
    /// The request contains only the civil month range and explicit resource
    /// ceilings. Feed, instrument, rung, source commit, training boundary,
    /// evaluator and Long/Short exit grids are all inherited from this held
    /// transaction; none can be restated by a successor caller.
    #[allow(
        dead_code,
        reason = "the opaque OOS handoff is reserved for terminal-aware Selection V6 and Global Replay V4"
    )]
    pub(crate) fn stored_post_training_oos_cohort(
        &self,
        request: StoredPostTrainingOosRequestV1,
    ) -> Result<StoredPostTrainingOosCohortV1, Step3OrchestratorRefusal> {
        self.root
            .require_same("before stored post-training OOS cohort admission")?;
        let cohort = self
            .execution
            .post_training_oos_cohort(&self.root, request)?;
        self.root
            .require_same("after stored post-training OOS cohort admission")?;
        Ok(cohort)
    }

    fn root(&self) -> &AdmittedRootV1 {
        &self.root
    }

    #[must_use]
    fn identities(&self) -> CandidatePreAdmissionReopenIdentitiesV1 {
        // Public behavior intentionally projects away both sealed observations
        // and retained execution sources. Reading the opaque authorities here
        // makes that loss explicit rather than accidentally leaving them out of
        // the transaction result.
        let committed = self.candidate_pre_admission();
        let _sealed_observations = committed.observations();
        committed.identities()
    }
}

/// Opaque receipt-last Observation V1 plus Statistics V2 transaction.
///
/// The two stored source capabilities are consumed in canonical NIFTY then
/// BANKNIFTY order and retained beside every derived authority.  Consequently
/// no later Step-3 join can detach the paired observations or Statistics
/// projection from the exact Candidate/Pre-Admission transactions that caused
/// them.  There is no constructor from decoded audits, digests or rows.
pub(crate) struct CommittedStoredObservationStatisticsV2 {
    nifty_source: CommittedStoredCandidatePreAdmissionV1,
    banknifty_source: CommittedStoredCandidatePreAdmissionV1,
    roots: AdmittedObservationStatisticsRootsV2,
    base_evidence: PairedBaseEvidenceAuthorityV2,
    base_evidence_reader: PairedBaseEvidenceReaderV2,
    observations: PairedCandidateObservationsV1,
    observation: ObservationAuthorityCommitV1,
    statistics: PopulationStatisticsObservationCommitV2,
    projection: PopulationStatisticsV2ProjectionSource,
    statistics_reader: PopulationStatisticsV2Ledger,
    admission_projection: PopulationStatisticsAdmissionProjectionV3,
}

/// Receipt-last Search V4 lineage retained with its exact stored authority chain.
///
/// This capability cannot be built from decoded identities. It owns the
/// Candidate/Observation/Statistics transaction, the freshly authenticated
/// Search V4 pair, and a held directory identity for the lineage ledger. A
/// later Admission stage therefore cannot substitute a second Search result or
/// a replaced lineage root.
pub(crate) struct CommittedStoredSearchLineageV4 {
    source: CommittedStoredObservationStatisticsV2,
    lineage_root: AdmittedRootV1,
    lineage: AuthenticatedAnchoredSearchLineageV4,
}

impl CommittedStoredSearchLineageV4 {
    /// Exact retained Candidate/Observation/Statistics source.
    #[must_use]
    pub(crate) const fn source(&self) -> &CommittedStoredObservationStatisticsV2 {
        &self.source
    }

    /// Mutable source access reserved for the single Admission V3 producer.
    pub(crate) const fn source_mut(&mut self) -> &mut CommittedStoredObservationStatisticsV2 {
        &mut self.source
    }

    /// Fresh-reopen authenticated Search V4 lineage.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn lineage(&self) -> AuthenticatedAnchoredSearchLineageV4 {
        self.lineage
    }

    fn require_all_roots(&self, stage: &str) -> Result<(), Step3OrchestratorRefusal> {
        self.source.require_all_roots(stage)?;
        self.lineage_root
            .require_same(&format!("{stage} (Search V4 lineage root)"))
    }
}

/// Persists and freshly authenticates the exact retained NIFTY/BANKNIFTY
/// Search V4 pair without accepting a raw projection or signal source.
///
/// # Cost
///
/// This is a bounded whole-ledger operation, not O(1). The lineage ledger is
/// validated on open and the two fixed Search members are compared field by
/// field with the retained Runner projections.
///
/// # Errors
///
/// Refuses a removed or substituted upstream/root directory, a foreign
/// Candidate-to-Search binding, any V4 projection mismatch, an over-bound
/// ledger, corruption, a ragged/orphaned foreign tail, or failed durable sync.
pub(crate) fn commit_stored_search_lineage_v4(
    source: CommittedStoredObservationStatisticsV2,
    lineage_root: &Path,
    bounds: AnchoredSearchLineageV4Bounds,
) -> Result<CommittedStoredSearchLineageV4, Step3OrchestratorRefusal> {
    source.require_all_roots("before Search V4 lineage admission")?;
    let admitted_root = AdmittedRootV1::admit(lineage_root)
        .map_err(|why| format!("Step 3 Search V4 lineage root refused: {why}"))?;
    let search = source.search_pair_v4()?;
    let nifty = search.nifty().projection()?;
    let banknifty = search.banknifty().projection()?;
    source.require_all_roots("before Search V4 lineage receipt-last append/reopen")?;
    admitted_root.require_same("before Search V4 lineage receipt-last append/reopen")?;
    let lineage =
        persist_anchored_search_lineage_v4(admitted_root.path(), bounds, &nifty, &banknifty)
            .map_err(|why| format!("Step 3 Search V4 lineage commit refused: {why}"))?
            .authority();
    source.require_all_roots("after Search V4 lineage receipt-last append/reopen")?;
    admitted_root.require_same("after Search V4 lineage receipt-last append/reopen")?;
    require_exact_search_lineage_member_v4(&lineage.nifty(), &nifty, "NIFTY")?;
    require_exact_search_lineage_member_v4(&lineage.banknifty(), &banknifty, "BANKNIFTY")?;
    require_exact_search_lineage_receipt_v4(&lineage, &nifty, &banknifty)?;
    let committed = CommittedStoredSearchLineageV4 {
        source,
        lineage_root: admitted_root,
        lineage,
    };
    committed.require_all_roots("after final Search V4 lineage authentication")?;
    Ok(committed)
}

fn require_exact_search_lineage_member_v4(
    durable: &AnchoredSearchLineageMemberAuthorityV4,
    projection: &AnchoredSearchAuthorityProjectionV4,
    family: &str,
) -> Result<(), Step3OrchestratorRefusal> {
    let source = projection.source_identity();
    let long = projection.long_grid_identity();
    let short = projection.short_grid_identity();
    if durable.member_id() == [0_u8; 32]
        || durable.source_authority_id() == [0_u8; 32]
        || durable.validation_policy_id() != projection.policy_identity().digest()
        || durable.signal_digest() != source.signal_digest()
        || durable.signal_bars() != source.signal_bars()
        || durable.signal_first_ts_micros() != source.signal_first_ts_micros()
        || durable.signal_last_ts_micros() != source.signal_last_ts_micros()
        || durable.signal_column_digest() != source.signal_column_digest()
        || durable.aggregate_grid_id() != projection.grid_identity().digest()
        || durable.long_policy_id() != long.policy_digest()
        || durable.long_resolution_id() != long.resolution_digest()
        || durable.short_policy_id() != short.policy_digest()
        || durable.short_resolution_id() != short.resolution_digest()
        || durable.validation_family_id() != projection.family_identity().digest()
        || durable.walk_facts_id() != projection.walk_identity().digest()
        || durable.fold_count() != projection.fold_count()
        || durable.decided_folds() != projection.decided_folds()
        || durable.profitable_oos_folds() != projection.profitable_oos_folds()
        || durable.aggregate_oos_paisa() != projection.aggregate_oos_paisa()
        || durable.evaluated_population_cells() != projection.evaluated_population_cells()
    {
        return Err(format!(
            "Step 3 durable {family} Search V4 lineage differs from its retained projection"
        ));
    }
    Ok(())
}

fn require_exact_search_lineage_receipt_v4(
    durable: &AuthenticatedAnchoredSearchLineageV4,
    nifty: &AnchoredSearchAuthorityProjectionV4,
    banknifty: &AnchoredSearchAuthorityProjectionV4,
) -> Result<(), Step3OrchestratorRefusal> {
    let receipt = durable.structural_receipt();
    let block_sequence = receipt.block_sequence();
    let total_folds = nifty
        .fold_count()
        .checked_add(banknifty.fold_count())
        .ok_or_else(|| "Step 3 Search V4 total fold count overflowed".to_owned())?;
    let total_decided = nifty
        .decided_folds()
        .checked_add(banknifty.decided_folds())
        .ok_or_else(|| "Step 3 Search V4 total decided-fold count overflowed".to_owned())?;
    let total_profitable = nifty
        .profitable_oos_folds()
        .checked_add(banknifty.profitable_oos_folds())
        .ok_or_else(|| "Step 3 Search V4 total profitable-fold count overflowed".to_owned())?;
    let aggregate_oos_paisa = nifty
        .aggregate_oos_paisa()
        .checked_add(banknifty.aggregate_oos_paisa())
        .ok_or_else(|| "Step 3 Search V4 aggregate OOS paisa overflowed".to_owned())?;
    let total_evaluated = nifty
        .evaluated_population_cells()
        .checked_add(banknifty.evaluated_population_cells())
        .ok_or_else(|| "Step 3 Search V4 evaluated-population count overflowed".to_owned())?;
    if receipt.pair_id() == [0_u8; 32]
        || receipt.nifty_member_id() != durable.nifty().member_id()
        || receipt.banknifty_member_id() != durable.banknifty().member_id()
        || receipt.total_folds() != total_folds
        || receipt.total_decided() != total_decided
        || receipt.total_profitable() != total_profitable
        || receipt.aggregate_oos_paisa() != aggregate_oos_paisa
        || receipt.total_evaluated_population_cells() != total_evaluated
    {
        return Err(format!(
            "Step 3 durable Search V4 receipt block {block_sequence} differs from its retained NIFTY/BANKNIFTY projections"
        ));
    }
    Ok(())
}

/// Durable Population Admission V3 retained with every exact upstream source.
///
/// The type cannot be constructed from a receipt. It owns the authenticated
/// Candidate/Observation/Statistics/Search lineage chain, holds the admitted
/// Admission root identity, and retains the freshly reopened Admission ledger
/// needed by fixed-offset Finalization projections.
pub(crate) struct CommittedStoredPopulationAdmissionV3 {
    search: CommittedStoredSearchLineageV4,
    admission_root: AdmittedRootV1,
    admission: PopulationAdmissionV3Authority,
}

impl CommittedStoredPopulationAdmissionV3 {
    /// Authenticated Admission V3 ledger retained for Finalization.
    pub(crate) const fn admission_mut(&mut self) -> &mut PopulationAdmissionV3Authority {
        &mut self.admission
    }

    /// Authenticated Admission V3 receipt.
    #[must_use]
    pub(crate) const fn admission_receipt(
        &self,
    ) -> crate::population_admission_v3::PopulationAdmissionV3StructuralReceipt {
        self.admission.structural_receipt()
    }

    fn require_all_roots(&self, stage: &str) -> Result<(), Step3OrchestratorRefusal> {
        self.search.require_all_roots(stage)?;
        self.admission_root
            .require_same(&format!("{stage} (Population Admission V3 root)"))
    }
}

/// Commits Population Admission V3 from the retained durable Search V4 chain.
///
/// No caller can supply Candidate rows, Statistics rows, Search projections,
/// source identities or arithmetic. The only variable decision input is the
/// typed Runner admission policy; all evidence is reread from held authorities.
///
/// # Cost
///
/// Preparation and durable authentication are bounded whole-population/file
/// operations, not O(1). A later decision uses an O(1) fixed-record offset,
/// but its end-to-end authenticated read remains O(file bytes): stale-safety
/// deliberately rehashes every bounded ledger generation before and after it.
///
/// # Errors
///
/// Refuses any changed upstream/root identity, Search lineage mismatch,
/// Candidate/Base/Statistics/Runner arithmetic crosswire, over-bound ledger,
/// failed sync, corruption, stale handle, or inexact retry.
pub(crate) fn commit_stored_population_admission_v3(
    mut search: CommittedStoredSearchLineageV4,
    admission_root: &Path,
    bounds: AdmissionV3Bounds,
    policy: &AdmissionPolicyV1,
) -> Result<CommittedStoredPopulationAdmissionV3, Step3OrchestratorRefusal> {
    search.require_all_roots("before Population Admission V3 admission")?;
    require_exact_committed_search_lineage_v4(&search)?;
    let admitted_root = AdmittedRootV1::admit(admission_root)
        .map_err(|why| format!("Step 3 Population Admission V3 root refused: {why}"))?;
    admitted_root.require_same("before Population Admission V3 receipt-last append/reopen")?;
    let committed = commit_population_admission_v3_exact_grid(
        admitted_root.path(),
        bounds,
        search.source_mut(),
        policy,
    )
    .map_err(|why| format!("Step 3 Population Admission V3 commit refused: {why}"))?;
    let admission = match committed {
        PopulationAdmissionV3Commit::Written(value)
        | PopulationAdmissionV3Commit::Reused(value) => value,
    };
    admitted_root.require_same("after Population Admission V3 receipt-last append/reopen")?;
    search.require_all_roots("after Population Admission V3 receipt-last append/reopen")?;
    require_exact_committed_search_lineage_v4(&search)?;
    let committed = CommittedStoredPopulationAdmissionV3 {
        search,
        admission_root: admitted_root,
        admission,
    };
    committed.require_all_roots("after final Population Admission V3 authentication")?;
    Ok(committed)
}

/// Durable Population Finalization V3 retained with its exact Admission chain.
///
/// This capability cannot be constructed from receipts or raw row projections.
/// It owns the admitted Finalization root and the freshly reopened authenticated
/// Finalization ledger while retaining every upstream stored authority through
/// [`CommittedStoredPopulationAdmissionV3`].
pub(crate) struct CommittedStoredPopulationFinalizationV3 {
    admission: CommittedStoredPopulationAdmissionV3,
    finalization_root: AdmittedRootV1,
    finalization: PopulationFinalizationV3Authority,
}

impl CommittedStoredPopulationFinalizationV3 {
    /// Freshly reopened authenticated Finalization V3 ledger.
    #[cfg(test)]
    pub(crate) const fn finalization_mut(&mut self) -> &mut PopulationFinalizationV3Authority {
        &mut self.finalization
    }

    /// Structural receipt proven against the retained Admission V3 authority.
    #[must_use]
    pub(crate) const fn finalization_receipt(&self) -> PopulationFinalizationV3StructuralReceipt {
        self.finalization.structural_receipt()
    }

    /// Produces the complete authenticated input block for Population V5.
    ///
    /// Candidate bytes, arithmetic-reverified Admission V3 values and
    /// Finalization V3 rows are reread through their retained authorities. The
    /// three canonical NIFTY-then-BANKNIFTY sequences are then joined by every
    /// shared ordinal, family, source and decision identity. No caller can
    /// supply a detached row, digest or legacy Admission projection.
    ///
    /// # Cost
    ///
    /// This complete authority join is O(C + bounded ledger bytes) time and
    /// O(C) output space for C Candidates. It is intentionally not described
    /// as O(1); only each already-authenticated fixed-record address is O(1).
    ///
    /// # Errors
    ///
    /// Refuses changed roots, stale/corrupt ledgers, incomplete or reordered
    /// blocks, family crosswires, any Candidate/Base/Admission/Finalization
    /// identity mismatch, invalid Runner V3 arithmetic, or allocation failure.
    pub(crate) fn population_v5_inputs(
        &mut self,
    ) -> Result<Vec<PopulationV5Input>, Step3OrchestratorRefusal> {
        self.require_all_roots("before Population V5 authority join")?;

        let source = self.admission.search.source();
        let nifty_audit = source
            .nifty_source()
            .candidate_pre_admission()
            .candidate_audit();
        let banknifty_audit = source
            .banknifty_source()
            .candidate_pre_admission()
            .candidate_audit();
        let nifty_candidates = source
            .nifty_source()
            .authenticated_candidate_population_rows()?;
        let banknifty_candidates = source
            .banknifty_source()
            .authenticated_candidate_population_rows()?;

        let nifty_count = u64::try_from(nifty_candidates.len())
            .map_err(|_| "Step 3 NIFTY Candidate count does not fit u64".to_owned())?;
        let banknifty_count = u64::try_from(banknifty_candidates.len())
            .map_err(|_| "Step 3 BANKNIFTY Candidate count does not fit u64".to_owned())?;
        let total_count = nifty_count
            .checked_add(banknifty_count)
            .ok_or_else(|| "Step 3 Population V5 Candidate count overflowed".to_owned())?;
        let finalization_receipt = self.finalization.structural_receipt();
        let admission_receipt = self.admission.admission.structural_receipt();
        if nifty_count != nifty_audit.row_count()
            || banknifty_count != banknifty_audit.row_count()
            || nifty_count != admission_receipt.nifty_decision_count()
            || banknifty_count != admission_receipt.banknifty_decision_count()
            || total_count != admission_receipt.decision_count()
            || nifty_count != finalization_receipt.nifty_row_count()
            || banknifty_count != finalization_receipt.banknifty_row_count()
            || total_count != finalization_receipt.row_count()
        {
            return Err(
                "Step 3 Population V5 Candidate/Admission/Finalization cardinalities differ"
                    .to_owned(),
            );
        }

        let capacity = usize::try_from(total_count)
            .map_err(|_| "Step 3 Population V5 input count does not fit usize".to_owned())?;
        let mut candidates = Vec::new();
        candidates
            .try_reserve_exact(capacity)
            .map_err(|why| format!("cannot reserve Step 3 Population V5 Candidates: {why}"))?;
        candidates.extend(nifty_candidates);
        candidates.extend(banknifty_candidates);

        let admissions = self
            .admission
            .admission
            .ordered_successor_projections()
            .map_err(|why| format!("Step 3 Admission V3 successor read refused: {why}"))?;
        let finalizations = self
            .finalization
            .ordered_row_projections()
            .map_err(|why| format!("Step 3 Finalization V3 successor read refused: {why}"))?;
        if admissions.len() != capacity || finalizations.len() != capacity {
            return Err(
                "Step 3 Population V5 successor vectors differ from authenticated cardinality"
                    .to_owned(),
            );
        }

        let inputs = join_population_v5_inputs(
            candidates,
            admissions,
            finalizations,
            finalization_receipt.completion_id(),
            nifty_count,
            &nifty_audit,
            &banknifty_audit,
        )?;
        self.require_all_roots("after Population V5 authority join")?;
        Ok(inputs)
    }

    /// Canonical NIFTY/BANKNIFTY Candidate execution replay pair retained for
    /// the Population V5-to-Execution V3 production seam.
    pub(crate) fn execution_v3_replay_pair(
        &self,
    ) -> Result<StoredCandidateExecutionReplayPairV1, Step3OrchestratorRefusal> {
        self.require_all_roots("before Execution V3 Candidate replay pair")?;
        let source = self.admission.search.source();
        let nifty = source.nifty_source().execution_v3_replay_authority()?;
        let banknifty = source.banknifty_source().execution_v3_replay_authority()?;
        if nifty.receipt().family() != InstrumentFamilyV1::Nifty
            || banknifty.receipt().family() != InstrumentFamilyV1::BankNifty
        {
            return Err(
                "Step 3 Execution V3 replay pair is not canonical NIFTY then BANKNIFTY".to_owned(),
            );
        }
        self.require_all_roots("after Execution V3 Candidate replay pair")?;
        Ok(StoredCandidateExecutionReplayPairV1 { nifty, banknifty })
    }

    fn require_all_roots(&self, stage: &str) -> Result<(), Step3OrchestratorRefusal> {
        self.admission.require_all_roots(stage)?;
        self.finalization_root
            .require_same(&format!("{stage} (Population Finalization V3 root)"))
    }
}

/// Opaque canonical replay pair produced only from retained stored authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredCandidateExecutionReplayPairV1 {
    nifty: CandidateExecutionReplayAuthorityV1,
    banknifty: CandidateExecutionReplayAuthorityV1,
}

impl StoredCandidateExecutionReplayPairV1 {
    pub(crate) fn into_parts(
        self,
    ) -> (
        CandidateExecutionReplayAuthorityV1,
        CandidateExecutionReplayAuthorityV1,
    ) {
        (self.nifty, self.banknifty)
    }
}

fn join_population_v5_inputs(
    candidates: Vec<AuthenticatedCandidatePopulationRowV1>,
    admissions: Vec<PopulationAdmissionV3SuccessorProjection>,
    finalizations: Vec<PopulationFinalizationV3RowProjection>,
    finalization_completion_id: [u8; 32],
    nifty_count: u64,
    nifty_audit: &CandidateUniverseReopenAuditV1,
    banknifty_audit: &CandidateUniverseReopenAuditV1,
) -> Result<Vec<PopulationV5Input>, Step3OrchestratorRefusal> {
    let mut inputs = Vec::new();
    inputs
        .try_reserve_exact(candidates.len())
        .map_err(|why| format!("cannot reserve Step 3 Population V5 inputs: {why}"))?;
    for (index, ((candidate, admission), finalization)) in candidates
        .into_iter()
        .zip(admissions)
        .zip(finalizations)
        .enumerate()
    {
        let global_sequence = u64::try_from(index)
            .map_err(|_| "Step 3 Population V5 ordinal does not fit u64".to_owned())?;
        if admission.runner_decision().iter().all(|byte| *byte == 0) {
            return Err(format!(
                "Step 3 Population V5 canonical Runner decision is empty at ordinal {global_sequence}"
            ));
        }
        if admission.canonical_record().iter().all(|byte| *byte == 0) {
            return Err(format!(
                "Step 3 Population V5 canonical Admission record is empty at ordinal {global_sequence}"
            ));
        }
        let identity = admission.identity();
        require_candidate_admission_join(
            &candidate,
            &identity,
            global_sequence,
            nifty_count,
            nifty_audit,
            banknifty_audit,
        )?;
        require_admission_finalization_join(&identity, &finalization)?;
        inputs.push(PopulationV5Input {
            candidate,
            admission,
            finalization,
            finalization_completion_id,
        });
    }
    Ok(inputs)
}

/// Nonconstructible authenticated row for the append-only Population V5 seam.
///
/// This owns the literal sealed Candidate record, the typed arithmetic-verified
/// Admission V3 evidence/verdict and the exact Finalization V3 authority row.
/// Its fields stay private so a downstream producer must consume this joined
/// capability rather than reassemble three detached projections.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the exact joined capability is the input of the pending append-only Population V5 writer"
    )
)]
pub(crate) struct PopulationV5Input {
    candidate: AuthenticatedCandidatePopulationRowV1,
    admission: PopulationAdmissionV3SuccessorProjection,
    finalization: PopulationFinalizationV3RowProjection,
    finalization_completion_id: [u8; 32],
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the exact joined capability is the input of the pending append-only Population V5 writer"
    )
)]
impl PopulationV5Input {
    /// Literal authenticated Candidate input, including its sealed bytes.
    #[must_use]
    pub(crate) const fn candidate(&self) -> &AuthenticatedCandidatePopulationRowV1 {
        &self.candidate
    }

    /// Typed arithmetic-reverified Admission V3 input.
    #[must_use]
    pub(crate) const fn admission(&self) -> &PopulationAdmissionV3SuccessorProjection {
        &self.admission
    }

    /// Final receipt-last row joined to this Candidate and Admission decision.
    #[must_use]
    pub(crate) const fn finalization(&self) -> &PopulationFinalizationV3RowProjection {
        &self.finalization
    }

    /// Receipt-last Finalization V3 completion joined to this exact row.
    #[must_use]
    pub(crate) const fn finalization_completion_id(&self) -> [u8; 32] {
        self.finalization_completion_id
    }
}

fn require_candidate_admission_join(
    candidate: &AuthenticatedCandidatePopulationRowV1,
    admission: &PopulationAdmissionV3DecisionProjection,
    global_sequence: u64,
    nifty_count: u64,
    nifty_audit: &CandidateUniverseReopenAuditV1,
    banknifty_audit: &CandidateUniverseReopenAuditV1,
) -> Result<(), Step3OrchestratorRefusal> {
    let row = candidate.row();
    let (expected_family, expected_family_sequence, audit) = if global_sequence < nifty_count {
        (AdmissionV3Family::Nifty, global_sequence, nifty_audit)
    } else {
        (
            AdmissionV3Family::BankNifty,
            global_sequence
                .checked_sub(nifty_count)
                .ok_or_else(|| "Step 3 BANKNIFTY Population V5 ordinal underflowed".to_owned())?,
            banknifty_audit,
        )
    };
    let candidate_family = match row.family() {
        InstrumentFamilyV1::Nifty => AdmissionV3Family::Nifty,
        InstrumentFamilyV1::BankNifty => AdmissionV3Family::BankNifty,
    };
    if admission.global_sequence() != global_sequence
        || admission.family() != expected_family
        || admission.family_sequence() != expected_family_sequence
        || row.sequence() != expected_family_sequence
        || candidate_family != expected_family
        || row.universe_id() != audit.universe_id()
        || admission.candidate_universe_id() != audit.universe_id()
        || admission.candidate_completion_digest() != audit.content_digest()
        || admission.candidate_semantic_id() != row.candidate_semantic_digest()
        || admission.candidate_row_digest() != candidate.base_candidate_row_digest()
        || candidate.canonical_record().iter().all(|byte| *byte == 0)
    {
        return Err(format!(
            "Step 3 Population V5 Candidate/Admission join differs at canonical ordinal {global_sequence}"
        ));
    }
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "the authority seam deliberately compares every persisted Admission/Finalization identity instead of accepting a partial join"
)]
fn require_admission_finalization_join(
    admission: &PopulationAdmissionV3DecisionProjection,
    finalization: &PopulationFinalizationV3RowProjection,
) -> Result<(), Step3OrchestratorRefusal> {
    if admission.global_sequence() != finalization.global_sequence()
        || admission.family() != finalization.family()
        || admission.family_sequence() != finalization.family_sequence()
        || admission.status() != finalization.status()
    {
        return Err(format!(
            "Step 3 Population V5 Admission/Finalization ordering or status differs at ordinal {}",
            admission.global_sequence()
        ));
    }
    for (name, left, right) in [
        (
            "Candidate universe",
            admission.candidate_universe_id(),
            finalization.candidate_universe_id(),
        ),
        (
            "Candidate Completion",
            admission.candidate_completion_digest(),
            finalization.candidate_completion_digest(),
        ),
        (
            "Candidate semantic",
            admission.candidate_semantic_id(),
            finalization.candidate_semantic_id(),
        ),
        (
            "Candidate row",
            admission.candidate_row_digest(),
            finalization.candidate_row_digest(),
        ),
        (
            "Pre-Admission authority",
            admission.pre_admission_authority_id(),
            finalization.pre_admission_authority_id(),
        ),
        (
            "Statistics audit",
            admission.statistics_audit_id(),
            finalization.statistics_audit_id(),
        ),
        (
            "Statistics Completion",
            admission.statistics_completion_digest(),
            finalization.statistics_completion_digest(),
        ),
        (
            "Statistics period",
            admission.statistics_period_digest(),
            finalization.statistics_period_digest(),
        ),
        (
            "Statistics split",
            admission.statistics_split_digest(),
            finalization.statistics_split_digest(),
        ),
        (
            "Search pair",
            admission.search_pair_id(),
            finalization.search_pair_id(),
        ),
        (
            "Search member",
            admission.search_member_id(),
            finalization.search_member_id(),
        ),
        (
            "Search signal",
            admission.search_signal_digest(),
            finalization.search_signal_digest(),
        ),
        (
            "Search signal column",
            admission.search_signal_column_digest(),
            finalization.search_signal_column_digest(),
        ),
        (
            "Search policy",
            admission.search_policy_id(),
            finalization.search_policy_id(),
        ),
        (
            "Search full grid",
            admission.search_full_grid_id(),
            finalization.search_full_grid_id(),
        ),
        (
            "Search Long policy",
            admission.search_long_policy_id(),
            finalization.search_long_policy_id(),
        ),
        (
            "Search Long resolution",
            admission.search_long_resolution_id(),
            finalization.search_long_resolution_id(),
        ),
        (
            "Search Short policy",
            admission.search_short_policy_id(),
            finalization.search_short_policy_id(),
        ),
        (
            "Search Short resolution",
            admission.search_short_resolution_id(),
            finalization.search_short_resolution_id(),
        ),
        (
            "Search family",
            admission.search_family_id(),
            finalization.search_family_id(),
        ),
        (
            "Search walk",
            admission.search_walk_id(),
            finalization.search_walk_id(),
        ),
        (
            "paired Base Evidence",
            admission.paired_base_id(),
            finalization.paired_base_id(),
        ),
        (
            "Base Completion",
            admission.base_completion_id(),
            finalization.base_completion_id(),
        ),
        (
            "Base Evidence",
            admission.base_evidence_id(),
            finalization.base_evidence_id(),
        ),
        (
            "Admission block",
            admission.block_id(),
            finalization.admission_block_id(),
        ),
        (
            "Admission Completion",
            admission.completion_id(),
            finalization.admission_completion_id(),
        ),
        (
            "Runner decision",
            admission.runner_decision_digest(),
            finalization.admission_runner_decision_digest(),
        ),
        (
            "Runner evidence",
            admission.runner_evidence_digest(),
            finalization.admission_evidence_digest(),
        ),
        (
            "Runner verdict",
            admission.runner_verdict_digest(),
            finalization.admission_verdict_digest(),
        ),
        (
            "Admission decision",
            admission.decision_id(),
            finalization.admission_decision_id(),
        ),
    ] {
        if left != right {
            return Err(format!(
                "Step 3 Population V5 Admission/Finalization {name} differs at ordinal {}",
                admission.global_sequence()
            ));
        }
    }
    for (name, left, right) in [
        (
            "signal-bar count",
            admission.search_signal_bars(),
            finalization.search_signal_bars(),
        ),
        (
            "fold count",
            admission.search_fold_count(),
            finalization.search_fold_count(),
        ),
        (
            "decided-fold count",
            admission.search_decided_folds(),
            finalization.search_decided_folds(),
        ),
        (
            "profitable-fold count",
            admission.search_profitable_oos_folds(),
            finalization.search_profitable_oos_folds(),
        ),
        (
            "evaluated-population count",
            admission.search_evaluated_population_count(),
            finalization.search_evaluated_population_count(),
        ),
    ] {
        if left != right {
            return Err(format!(
                "Step 3 Population V5 Admission/Finalization {name} differs at ordinal {}",
                admission.global_sequence()
            ));
        }
    }
    for (name, left, right) in [
        (
            "first signal timestamp",
            admission.search_signal_first_ts_micros(),
            finalization.search_signal_first_ts_micros(),
        ),
        (
            "last signal timestamp",
            admission.search_signal_last_ts_micros(),
            finalization.search_signal_last_ts_micros(),
        ),
        (
            "aggregate OOS paisa",
            admission.search_aggregate_oos_paisa(),
            finalization.search_aggregate_oos_paisa(),
        ),
    ] {
        if left != right {
            return Err(format!(
                "Step 3 Population V5 Admission/Finalization {name} differs at ordinal {}",
                admission.global_sequence()
            ));
        }
    }
    Ok(())
}

/// Finalizes one retained Population Admission V3 transaction.
///
/// The caller supplies only an admitted output root and explicit refusal
/// ceilings. Candidate, Statistics, Search, Base and Admission identities and
/// rows remain inside the retained authority chain. Success means the receipt
/// was written last, reopened read-only, authenticated byte-for-byte, and then
/// cross-checked against the exact Admission receipt held by this transaction.
///
/// # Cost
///
/// Preparation is a bounded whole-population operation and persistence hashes
/// bounded whole files. It is not O(1). A retained fixed-record row address is
/// O(1), while its end-to-end authenticated read remains O(file bytes) because
/// stale-safety validates the bounded ledger generation before and after it.
///
/// # Errors
///
/// Refuses a changed upstream/root identity, Admission/Finalization receipt
/// mismatch, over-bound ledger, failed sync, corruption, stale handle, foreign
/// replacement, symlink substitution, or inexact retry.
pub(crate) fn commit_stored_population_finalization_v3(
    mut admission: CommittedStoredPopulationAdmissionV3,
    finalization_root: &Path,
    bounds: PopulationFinalizationV3Bounds,
) -> Result<CommittedStoredPopulationFinalizationV3, Step3OrchestratorRefusal> {
    admission.require_all_roots("before Population Finalization V3 admission")?;
    let admission_receipt = admission.admission_receipt();
    let admitted_root = AdmittedRootV1::admit(finalization_root)
        .map_err(|why| format!("Step 3 Population Finalization V3 root refused: {why}"))?;
    admitted_root.require_same("before Population Finalization V3 receipt-last append/reopen")?;
    let finalization =
        commit_population_finalization_v3(admitted_root.path(), bounds, admission.admission_mut())
            .map_err(|why| format!("Step 3 Population Finalization V3 commit refused: {why}"))?
            .into_authority();
    admitted_root.require_same("after Population Finalization V3 receipt-last append/reopen")?;
    admission.require_all_roots("after Population Finalization V3 receipt-last append/reopen")?;
    let finalization_receipt = finalization.structural_receipt();
    if finalization_receipt.finalization_id() == [0_u8; 32]
        || finalization_receipt.completion_id() == [0_u8; 32]
        || finalization_receipt.admission_block_id() != admission_receipt.block_id()
        || finalization_receipt.admission_completion_id() != admission_receipt.completion_id()
        || finalization_receipt.row_count() != admission_receipt.decision_count()
        || finalization_receipt.nifty_row_count() != admission_receipt.nifty_decision_count()
        || finalization_receipt.banknifty_row_count()
            != admission_receipt.banknifty_decision_count()
    {
        return Err(
            "Step 3 Population Finalization V3 receipt differs from its retained Admission V3 authority"
                .to_owned(),
        );
    }
    let committed = CommittedStoredPopulationFinalizationV3 {
        admission,
        finalization_root: admitted_root,
        finalization,
    };
    committed.require_all_roots("after final Population Finalization V3 authentication")?;
    Ok(committed)
}

fn require_exact_committed_search_lineage_v4(
    committed: &CommittedStoredSearchLineageV4,
) -> Result<(), Step3OrchestratorRefusal> {
    let search = committed.source.search_pair_v4()?;
    let nifty = search.nifty().projection()?;
    let banknifty = search.banknifty().projection()?;
    let durable = committed.lineage;
    require_exact_search_lineage_member_v4(&durable.nifty(), &nifty, "NIFTY")?;
    require_exact_search_lineage_member_v4(&durable.banknifty(), &banknifty, "BANKNIFTY")?;
    require_exact_search_lineage_receipt_v4(&durable, &nifty, &banknifty)
}

/// Exact per-candidate Base and Statistics inputs retained under one Step-3
/// authority transaction.
///
/// The two projections are never accepted separately by the orchestrator. The
/// global ordinal is mapped through both complete NIFTY-then-BANKNIFTY sources
/// and every family, family-ordinal, Candidate-universe and Candidate-semantic
/// term is compared before this value can exist. It remains input evidence,
/// not an Admission verdict or Finalization capability.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the exact Base/Statistics input pair is consumed by the pending Population Admission V3 production constructor"
    )
)]
pub(crate) struct Step3AdmissionCandidateInputsV3 {
    base: PairedBaseEvidenceRecordProjectionV2,
    statistics: PopulationStatisticsAdmissionCandidateV3,
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the exact Base/Statistics input pair is consumed by the pending Population Admission V3 production constructor"
    )
)]
impl Step3AdmissionCandidateInputsV3 {
    /// Base evidence folded in the same pass as the exact Candidate row.
    #[must_use]
    pub(crate) const fn base(&self) -> &PairedBaseEvidenceRecordProjectionV2 {
        &self.base
    }

    /// Statistics evidence read from the complete paired-family ledger.
    #[must_use]
    pub(crate) const fn statistics(&self) -> &PopulationStatisticsAdmissionCandidateV3 {
        &self.statistics
    }
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "read-only projections are reserved for the next crate-private Step-3 authority join"
    )
)]
impl CommittedStoredObservationStatisticsV2 {
    #[must_use]
    pub(crate) const fn nifty_source(&self) -> &CommittedStoredCandidatePreAdmissionV1 {
        &self.nifty_source
    }

    #[must_use]
    pub(crate) const fn banknifty_source(&self) -> &CommittedStoredCandidatePreAdmissionV1 {
        &self.banknifty_source
    }

    /// Canonical retained Search V4 pair, with no raw-search constructor.
    pub(crate) fn search_pair_v4(&self) -> Result<StoredSearchPairV4, Step3OrchestratorRefusal> {
        let nifty_receipt = self
            .nifty_source
            .candidate_pre_admission()
            .candidate_audit()
            .receipt();
        let banknifty_receipt = self
            .banknifty_source
            .candidate_pre_admission()
            .candidate_audit()
            .receipt();
        if nifty_receipt.family() != InstrumentFamilyV1::Nifty
            || banknifty_receipt.family() != InstrumentFamilyV1::BankNifty
            || self.nifty_source.search.candidate_universe_id() != nifty_receipt.universe_id()
            || self.nifty_source.search.candidate_completion_digest()
                != nifty_receipt.content_digest()
            || self.banknifty_source.search.candidate_universe_id()
                != banknifty_receipt.universe_id()
            || self.banknifty_source.search.candidate_completion_digest()
                != banknifty_receipt.content_digest()
        {
            return Err(
                "Step 3 retained Search V4 pair is not the canonical NIFTY/BANKNIFTY Candidate pair"
                    .to_owned(),
            );
        }
        Ok(StoredSearchPairV4 {
            nifty: self.nifty_source.search.clone(),
            banknifty: self.banknifty_source.search.clone(),
        })
    }

    #[must_use]
    pub(crate) const fn base_evidence(&self) -> &PairedBaseEvidenceAuthorityV2 {
        &self.base_evidence
    }

    #[must_use]
    pub(crate) const fn observations(&self) -> &PairedCandidateObservationsV1 {
        &self.observations
    }

    #[must_use]
    pub(crate) const fn observation_commit(&self) -> &ObservationAuthorityCommitV1 {
        &self.observation
    }

    #[must_use]
    pub(crate) const fn statistics_commit(&self) -> &PopulationStatisticsObservationCommitV2 {
        &self.statistics
    }

    #[must_use]
    pub(crate) const fn projection_source(&self) -> &PopulationStatisticsV2ProjectionSource {
        &self.projection
    }

    /// Reads one exact Statistics candidate through the complete-family
    /// Admission V3 projection while the admitted roots remain held.
    ///
    /// # Errors
    ///
    /// Refuses a removed/substituted root, stale or foreign Statistics source,
    /// changed candidate, malformed exact probability, lock or I/O failure.
    pub(crate) fn admission_statistics_v3(
        &mut self,
        sequence: u64,
    ) -> Result<PopulationStatisticsAdmissionCandidateV3, Step3OrchestratorRefusal> {
        self.require_all_roots("before Admission V3 Statistics fixed-row projection")?;
        let projected = self
            .statistics_reader
            .admission_candidate_v3(&self.admission_projection, sequence)
            .map_err(|why| format!("Step 3 Admission V3 Statistics projection refused: {why}"))?;
        self.require_admission_statistics_candidate_v3(&projected, sequence)?;
        self.require_all_roots("after Admission V3 Statistics fixed-row projection")?;
        Ok(projected)
    }

    /// Reads and equality-joins the exact same Candidate ordinal from Base
    /// Evidence and Statistics while every admitted root remains held.
    ///
    /// # Errors
    ///
    /// Refuses a stale/replaced root, an out-of-range ordinal, a foreign Base
    /// pair, or any global ordinal, family, Candidate-universe or semantic
    /// identity difference between the two fixed-record projections.
    pub(crate) fn admission_candidate_inputs_v3(
        &mut self,
        sequence: u64,
    ) -> Result<Step3AdmissionCandidateInputsV3, Step3OrchestratorRefusal> {
        self.require_all_roots("before paired Base/Statistics candidate projection")?;
        let statistics = self.admission_statistics_v3(sequence)?;
        let base = self
            .base_evidence_reader
            .candidate(sequence)
            .map_err(|why| format!("Step 3 paired Base candidate projection refused: {why}"))?;
        self.require_admission_candidate_inputs_v3(&base, &statistics)?;
        self.require_all_roots("after paired Base/Statistics candidate projection")?;
        Ok(Step3AdmissionCandidateInputsV3 { base, statistics })
    }

    /// Reads and equality-joins the complete Candidate family after one
    /// authenticated Statistics scan.
    ///
    /// Base projections remain fixed-offset reads from the already opened and
    /// validated paired ledger. Statistics generation validation is performed
    /// once for the complete family instead of once per Candidate, making this
    /// join O(Statistics-file-bytes + Candidate-count) rather than
    /// O(Candidate-count × Statistics-file-bytes).
    ///
    /// # Errors
    ///
    /// Refuses a stale/replaced root, allocation failure, any foreign Base or
    /// Statistics row, cardinality difference, corrupt record, or identity
    /// mismatch at any canonical global ordinal.
    pub(crate) fn admission_candidate_inputs_family_v3(
        &mut self,
    ) -> Result<Vec<Step3AdmissionCandidateInputsV3>, Step3OrchestratorRefusal> {
        self.require_all_roots("before complete-family paired Base/Statistics projection")?;
        let statistics = self.admission_statistics_family_v3()?;
        let mut joined = Vec::new();
        joined
            .try_reserve_exact(statistics.len())
            .map_err(|why| format!("Step 3 cannot reserve paired Admission inputs: {why}"))?;
        for candidate in statistics {
            let base = self
                .base_evidence_reader
                .candidate(candidate.sequence())
                .map_err(|why| format!("Step 3 paired Base candidate projection refused: {why}"))?;
            self.require_admission_candidate_inputs_v3(&base, &candidate)?;
            joined.push(Step3AdmissionCandidateInputsV3 {
                base,
                statistics: candidate,
            });
        }
        if u64::try_from(joined.len()).ok() != Some(self.projection.candidate_count()) {
            return Err(
                "Step 3 complete-family paired Base/Statistics cardinality differs".to_owned(),
            );
        }
        self.require_all_roots("after complete-family paired Base/Statistics projection")?;
        Ok(joined)
    }

    fn require_admission_candidate_inputs_v3(
        &self,
        base: &PairedBaseEvidenceRecordProjectionV2,
        statistics: &PopulationStatisticsAdmissionCandidateV3,
    ) -> Result<(), Step3OrchestratorRefusal> {
        let family_source = self.projection.family_source(statistics.family());
        let record = base.record();
        if self.base_evidence_reader.authority() != &self.base_evidence
            || self.base_evidence_reader.candidate_count() != self.projection.candidate_count()
            || base.pair_id() != self.base_evidence.pair_id()
            || base.global_sequence() != statistics.sequence()
            || base.family() != statistics.family()
            || base.family_sequence() != statistics.family_sequence()
            || record.candidate_universe_id() != family_source.candidate_universe_id()
            || record.candidate_sequence() != statistics.family_sequence()
            || record.candidate_semantic_id() != statistics.candidate_semantic_digest()
            || record.evidence_id() == [0_u8; 32]
            || record.candidate_row_digest() == [0_u8; 32]
            || record.trade_rows_digest() == [0_u8; 32]
        {
            return Err(
                "Step 3 paired Base and Statistics candidate projections differ".to_owned(),
            );
        }
        Ok(())
    }

    /// Reads the complete source-bound Statistics candidate family under one
    /// generation-validation pair.
    ///
    /// # Errors
    ///
    /// Refuses any removed/substituted source or authority root, stale or
    /// foreign Statistics source, allocation failure, changed candidate,
    /// malformed exact probability, lock or I/O failure.
    pub(crate) fn admission_statistics_family_v3(
        &mut self,
    ) -> Result<Vec<PopulationStatisticsAdmissionCandidateV3>, Step3OrchestratorRefusal> {
        self.require_all_roots("before complete-family Admission V3 Statistics projection")?;
        let projected = self
            .statistics_reader
            .admission_candidates_v3(&self.admission_projection)
            .map_err(|why| {
                format!("Step 3 complete-family Admission V3 Statistics projection refused: {why}")
            })?;
        if u64::try_from(projected.len()).ok() != Some(self.projection.candidate_count()) {
            return Err(
                "Step 3 complete-family Admission V3 Statistics cardinality differs".to_owned(),
            );
        }
        for (sequence, candidate) in projected.iter().enumerate() {
            self.require_admission_statistics_candidate_v3(
                candidate,
                u64::try_from(sequence).map_err(|why| {
                    format!("Step 3 Admission V3 candidate ordinal does not fit u64: {why}")
                })?,
            )?;
        }
        self.require_all_roots("after complete-family Admission V3 Statistics projection")?;
        Ok(projected)
    }

    fn require_admission_statistics_candidate_v3(
        &self,
        candidate: &PopulationStatisticsAdmissionCandidateV3,
        expected_sequence: u64,
    ) -> Result<(), Step3OrchestratorRefusal> {
        let family_source = self.projection.family_source(candidate.family());
        let expected_global_sequence = match candidate.family() {
            InstrumentFamilyV1::Nifty => candidate.family_sequence(),
            InstrumentFamilyV1::BankNifty => self
                .projection
                .family_source(InstrumentFamilyV1::Nifty)
                .candidate_count()
                .checked_add(candidate.family_sequence())
                .ok_or_else(|| {
                    "Step 3 Admission V3 BANKNIFTY global ordinal overflowed".to_owned()
                })?,
        };
        let draft = candidate.draft();
        if candidate.source() != &self.projection
            || candidate.sequence() != expected_sequence
            || expected_global_sequence != expected_sequence
            || candidate.family_sequence() >= family_source.candidate_count()
            || candidate.pre_admission_authority_id() != family_source.pre_admission_authority_id()
            || candidate.candidate_semantic_digest() != draft.candidate_semantic_id
            || candidate.ordered_candidate_digest() != self.projection.ordered_candidate_digest()
            || candidate.ordered_period_family_digest() != self.projection.ordered_period_digest()
            || candidate.ordered_split_family_digest() != self.projection.cscv_split_family_digest()
            || candidate.ordered_split_family_digest() != draft.cscv_split_family_digest
            || candidate.candidate_ordered_period_digest() == [0_u8; 32]
            || candidate.candidate_ordered_split_digest() == [0_u8; 32]
            || candidate.candidate_count() != self.projection.candidate_count()
            || candidate.period_count() != self.projection.period_count()
            || candidate.split_count() != self.projection.split_count()
            || draft.statistics_audit_id != self.projection.audit_id()
            || draft.statistics_completion_digest != self.projection.completion_record_digest()
            || draft.observation_statistics_link_id
                != self.projection.observation_statistics_link_id()
        {
            return Err(
                "Step 3 Admission V3 candidate differs from retained family provenance".to_owned(),
            );
        }
        Ok(())
    }

    fn require_all_roots(&self, stage: &str) -> Result<(), Step3OrchestratorRefusal> {
        require_exact_stored_source_root_pair_v2(
            &self.nifty_source,
            &self.banknifty_source,
            stage,
        )?;
        self.roots.require_same(stage)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ObservationStatisticsJoinFactsV2 {
    observation_authority_id: [u8; 32],
    observation_pair_identity: [u8; 32],
    candidate_count: u64,
    period_count: u64,
    split_count: u64,
}

impl ObservationStatisticsJoinFactsV2 {
    const fn from_observation(
        audit: &crate::population_observations_v1::ObservationAuthorityAuditV1,
    ) -> Self {
        Self {
            observation_authority_id: audit.authority_id(),
            observation_pair_identity: audit.pair_identity(),
            candidate_count: audit.candidate_count(),
            period_count: audit.period_count(),
            split_count: audit.split_count(),
        }
    }

    const fn from_statistics(source: &PopulationStatisticsV2ProjectionSource) -> Self {
        Self {
            observation_authority_id: source.observation_authority_id(),
            observation_pair_identity: source.observation_pair_identity(),
            candidate_count: source.candidate_count(),
            period_count: source.period_count(),
            split_count: source.split_count(),
        }
    }
}

struct AdmittedObservationStatisticsRootsV2 {
    observation: AdmittedRootV1,
    statistics: AdmittedRootV1,
}

impl AdmittedObservationStatisticsRootsV2 {
    fn admit(
        observation_root: &Path,
        statistics_root: &Path,
    ) -> Result<Self, Step3OrchestratorRefusal> {
        let observation = AdmittedRootV1::admit(observation_root)
            .map_err(|why| format!("Step 3 Observation authority root refused: {why}"))?;
        let statistics = AdmittedRootV1::admit(statistics_root)
            .map_err(|why| format!("Step 3 Statistics V2 root refused: {why}"))?;
        Ok(Self {
            observation,
            statistics,
        })
    }

    fn require_same(&self, stage: &str) -> Result<(), Step3OrchestratorRefusal> {
        self.observation
            .require_same(&format!("{stage} (Observation root)"))?;
        self.statistics
            .require_same(&format!("{stage} (Statistics root)"))
    }
}

/// Commits one exact stored NIFTY/BANKNIFTY Observation-to-Statistics chain.
///
/// Both output roots must already exist as directories.  The function accepts
/// no bars, candidate rows, identities, digests, split scores or fallback.  It
/// consumes the two opaque stored transactions, obtains observations and
/// Pre-Admission audits only through those capabilities, appends/reopens the
/// paired Observation authority, derives Statistics V2 from that same reopened
/// pair, appends/reopens Statistics, and revalidates the detached projection
/// before returning the still-coupled opaque result.
///
/// # Cost
///
/// Pairing and Statistics preparation are bounded whole-family operations, not
/// O(1): they retain and validate every admitted candidate-period/split row and
/// execute the explicit finite resampling procedure.  The two caller-provided
/// ledger bounds and procedure contain every applicable ceiling; there is no
/// implicit default.
///
/// # Errors
///
/// Refuses swapped or duplicate families, any foreign source/policy/session
/// term, absent/non-directory/substituted roots, bounded append failure,
/// foreign reopened Observation or Pre-Admission authority, Statistics
/// preparation failure, or any Observation-to-Statistics crosswire.  A
/// receipt-last Observation append may remain as a recoverable prefix if the
/// later Statistics stage refuses; no success capability escapes.
pub(crate) fn commit_stored_observation_statistics_v2(
    nifty_source: CommittedStoredCandidatePreAdmissionV1,
    banknifty_source: CommittedStoredCandidatePreAdmissionV1,
    observation_root: &Path,
    statistics_root: &Path,
    observation_bounds: ObservationAuthorityBoundsV1,
    statistics_bounds: PopulationStatisticsV2Bounds,
    procedure: PopulationStatisticsProcedureV2,
) -> Result<CommittedStoredObservationStatisticsV2, Step3OrchestratorRefusal> {
    require_exact_stored_source_root_pair_v2(
        &nifty_source,
        &banknifty_source,
        "before paired family validation",
    )?;
    let (base_evidence, base_evidence_reader) =
        reopen_paired_base_evidence_v2(&nifty_source, &banknifty_source)?;
    let nifty = nifty_source.candidate_pre_admission();
    let banknifty = banknifty_source.candidate_pre_admission();
    require_canonical_observation_family_order_v2(
        nifty.observations().family(),
        banknifty.observations().family(),
    )?;
    let roots = AdmittedObservationStatisticsRootsV2::admit(observation_root, statistics_root)?;
    let observations = PairedCandidateObservationsV1::from_families(
        nifty.observations().clone(),
        banknifty.observations().clone(),
    )
    .map_err(|why| format!("Step 3 paired observations refused: {why}"))?;
    let nifty_pre_admission = nifty.pre_admission_audit();
    let banknifty_pre_admission = banknifty.pre_admission_audit();

    require_exact_stored_source_root_pair_v2(
        &nifty_source,
        &banknifty_source,
        "before paired Observation receipt-last append/reopen",
    )?;
    roots.require_same("before paired Observation receipt-last append/reopen")?;
    let observation = observations
        .append_and_reopen(roots.observation.path(), observation_bounds)
        .map_err(|why| format!("Step 3 paired Observation commit refused: {why}"))?;
    roots.require_same("after paired Observation receipt-last append/reopen")?;
    require_exact_stored_source_root_pair_v2(
        &nifty_source,
        &banknifty_source,
        "after paired Observation receipt-last append/reopen",
    )?;

    let prepared_statistics = prepare_population_statistics_v2_from_observations(
        &observations,
        observation.audit(),
        nifty_pre_admission,
        banknifty_pre_admission,
        procedure,
    )
    .map_err(|why| format!("Step 3 Statistics V2 preparation refused: {why}"))?;
    require_exact_stored_source_root_pair_v2(
        &nifty_source,
        &banknifty_source,
        "before Statistics V2 receipt-last append/reopen",
    )?;
    roots.require_same("before Statistics V2 receipt-last append/reopen")?;
    let statistics = prepared_statistics
        .append_and_reopen(roots.statistics.path(), statistics_bounds)
        .map_err(|why| format!("Step 3 Statistics V2 commit refused: {why}"))?;
    roots.require_same("after Statistics V2 receipt-last append/reopen")?;
    require_exact_stored_source_root_pair_v2(
        &nifty_source,
        &banknifty_source,
        "after Statistics V2 receipt-last append/reopen",
    )?;
    let projection = require_exact_observation_statistics_join_v2(
        &observations,
        &observation,
        &statistics,
        &nifty_pre_admission,
        &banknifty_pre_admission,
    )?;
    roots.require_same("after final Observation-to-Statistics projection revalidation")?;
    require_exact_stored_source_root_pair_v2(
        &nifty_source,
        &banknifty_source,
        "after final Observation-to-Statistics projection revalidation",
    )?;
    let (statistics_reader, admission_projection) = prepare_admission_statistics_projection_v3(
        &roots,
        &projection,
        &nifty_source,
        &banknifty_source,
        statistics_bounds,
    )?;

    Ok(CommittedStoredObservationStatisticsV2 {
        nifty_source,
        banknifty_source,
        roots,
        base_evidence,
        base_evidence_reader,
        observations,
        observation,
        statistics,
        projection,
        statistics_reader,
        admission_projection,
    })
}

fn prepare_admission_statistics_projection_v3(
    roots: &AdmittedObservationStatisticsRootsV2,
    projection: &PopulationStatisticsV2ProjectionSource,
    nifty_source: &CommittedStoredCandidatePreAdmissionV1,
    banknifty_source: &CommittedStoredCandidatePreAdmissionV1,
    statistics_bounds: PopulationStatisticsV2Bounds,
) -> Result<
    (
        PopulationStatisticsV2Ledger,
        PopulationStatisticsAdmissionProjectionV3,
    ),
    Step3OrchestratorRefusal,
> {
    let mut reader =
        PopulationStatisticsV2Ledger::open_read(roots.statistics.path(), statistics_bounds)
            .map_err(|why| {
                format!("Step 3 Admission V3 Statistics read-only reopen refused: {why}")
            })?;
    let authority = reader
        .prepare_admission_projection_v3(projection)
        .map_err(|why| {
            format!("Step 3 complete-family Admission V3 Statistics scan refused: {why}")
        })?;
    roots.require_same("after complete-family Admission V3 Statistics scan")?;
    require_exact_stored_source_root_pair_v2(
        nifty_source,
        banknifty_source,
        "after complete-family Admission V3 Statistics scan",
    )?;
    Ok((reader, authority))
}

fn reopen_paired_base_evidence_v2(
    nifty_source: &CommittedStoredCandidatePreAdmissionV1,
    banknifty_source: &CommittedStoredCandidatePreAdmissionV1,
) -> Result<(PairedBaseEvidenceAuthorityV2, PairedBaseEvidenceReaderV2), Step3OrchestratorRefusal> {
    require_exact_stored_source_root_pair_v2(
        nifty_source,
        banknifty_source,
        "before one-scan paired Base Evidence reopen",
    )?;
    let nifty_expected = *nifty_source.candidate_pre_admission().base_evidence_audit();
    let banknifty_expected = *banknifty_source
        .candidate_pre_admission()
        .base_evidence_audit();
    let mut reader = BaseEvidenceLedgerReaderV2::open(
        nifty_source.root().path(),
        nifty_source
            .candidate_pre_admission()
            .base_evidence_bounds(),
    )
    .map_err(|why| format!("Step 3 paired Base Evidence reopen refused: {why}"))?;
    let nifty = reader
        .audit(nifty_expected.candidate_universe_id())
        .map_err(|why| format!("Step 3 NIFTY Base Evidence audit lookup refused: {why}"))?
        .ok_or_else(|| "Step 3 NIFTY Base Evidence completion disappeared".to_owned())?;
    let banknifty = reader
        .audit(banknifty_expected.candidate_universe_id())
        .map_err(|why| format!("Step 3 BANKNIFTY Base Evidence audit lookup refused: {why}"))?
        .ok_or_else(|| "Step 3 BANKNIFTY Base Evidence completion disappeared".to_owned())?;
    if nifty != nifty_expected || banknifty != banknifty_expected {
        return Err(
            "Step 3 freshly reopened Base Evidence differs from retained authority".to_owned(),
        );
    }
    let pair = pair_candidate_base_evidence_v2(&nifty, &banknifty)
        .map_err(|why| format!("Step 3 paired Base Evidence refused: {why}"))?;
    if pair.pair_id() == [0; 32]
        || pair.nifty_audit() != &nifty
        || pair.banknifty_audit() != &banknifty
    {
        return Err(
            "Step 3 paired Base Evidence capability differs from its freshly reopened inputs"
                .to_owned(),
        );
    }
    let paired_reader = reader
        .bind_pair(&pair)
        .map_err(|why| format!("Step 3 paired Base Evidence reader refused: {why}"))?;
    Ok((pair, paired_reader))
}

fn require_canonical_observation_family_order_v2(
    nifty: InstrumentFamilyV1,
    banknifty: InstrumentFamilyV1,
) -> Result<(), Step3OrchestratorRefusal> {
    if nifty != InstrumentFamilyV1::Nifty || banknifty != InstrumentFamilyV1::BankNifty {
        return Err(
            "Step 3 Observation/Statistics transaction requires exact NIFTY then BANKNIFTY source capabilities"
                .to_owned(),
        );
    }
    Ok(())
}

fn require_exact_stored_source_root_pair_v2(
    nifty: &CommittedStoredCandidatePreAdmissionV1,
    banknifty: &CommittedStoredCandidatePreAdmissionV1,
    stage: &str,
) -> Result<(), Step3OrchestratorRefusal> {
    require_exact_admitted_source_root_pair_v2(nifty.root(), banknifty.root(), stage)
}

fn require_exact_admitted_source_root_pair_v2(
    nifty_root: &AdmittedRootV1,
    banknifty_root: &AdmittedRootV1,
    stage: &str,
) -> Result<(), Step3OrchestratorRefusal> {
    nifty_root.require_same(&format!("{stage} (NIFTY source root)"))?;
    banknifty_root.require_same(&format!("{stage} (BANKNIFTY source root)"))?;
    if nifty_root.canonical != banknifty_root.canonical
        || nifty_root.generation != banknifty_root.generation
    {
        return Err(format!(
            "Step 3 Observation/Statistics transaction refused cross-root stored source capabilities at {stage}"
        ));
    }
    Ok(())
}

fn require_exact_observation_statistics_join_v2(
    observations: &PairedCandidateObservationsV1,
    observation: &ObservationAuthorityCommitV1,
    statistics: &PopulationStatisticsObservationCommitV2,
    nifty_pre_admission: &PreAdmissionDataReopenAuditV1,
    banknifty_pre_admission: &PreAdmissionDataReopenAuditV1,
) -> Result<PopulationStatisticsV2ProjectionSource, Step3OrchestratorRefusal> {
    let observation_audit = (*observation).audit();
    observations
        .require_reopened_authority(&observation_audit)
        .map_err(|why| format!("Step 3 reopened Observation authority refused: {why}"))?;
    (*statistics)
        .audit()
        .verify_pre_admission_pair(*nifty_pre_admission, *banknifty_pre_admission)
        .map_err(|why| format!("Step 3 reopened Statistics Pre-Admission join refused: {why}"))?;
    let projection = (*statistics)
        .projection_source()
        .map_err(|why| format!("Step 3 Statistics projection source refused: {why}"))?;
    require_exact_observation_statistics_facts_v2(
        ObservationStatisticsJoinFactsV2::from_observation(&observation_audit),
        ObservationStatisticsJoinFactsV2::from_statistics(&projection),
    )?;
    require_exact_statistics_family_source_v2(
        &projection,
        InstrumentFamilyV1::Nifty,
        nifty_pre_admission,
    )?;
    require_exact_statistics_family_source_v2(
        &projection,
        InstrumentFamilyV1::BankNifty,
        banknifty_pre_admission,
    )?;
    Ok(projection)
}

fn require_exact_observation_statistics_facts_v2(
    observation: ObservationStatisticsJoinFactsV2,
    statistics: ObservationStatisticsJoinFactsV2,
) -> Result<(), Step3OrchestratorRefusal> {
    if observation != statistics {
        return Err(
            "Step 3 Statistics projection is crosswired from another Observation authority, pair or cardinality"
                .to_owned(),
        );
    }
    Ok(())
}

fn require_exact_statistics_family_source_v2(
    projection: &PopulationStatisticsV2ProjectionSource,
    expected_family: InstrumentFamilyV1,
    pre_admission: &PreAdmissionDataReopenAuditV1,
) -> Result<(), Step3OrchestratorRefusal> {
    let source = (*projection).family_source(expected_family);
    let expected = (*pre_admission).value();
    if source.family() != expected_family
        || expected.family() != expected_family
        || source.pre_admission_authority_id() != pre_admission.authority_id()
        || source.candidate_universe_id() != expected.candidate_universe_id()
        || source.candidate_completion_digest() != expected.candidate_completion_digest()
        || source.candidate_count() != expected.candidate_row_count()
    {
        return Err(format!(
            "Step 3 Statistics projection carries a foreign {expected_family:?} Pre-Admission source"
        ));
    }
    Ok(())
}

impl RetainedStoredExecutionContextV1 {
    /// Loads a later OOS span without letting the caller restate any market or
    /// training fact retained by this stored transaction.
    #[allow(
        dead_code,
        reason = "the opaque OOS handoff is reserved for terminal-aware Selection V6 and Global Replay V4"
    )]
    fn post_training_oos_cohort(
        &self,
        root: &AdmittedRootV1,
        request: StoredPostTrainingOosRequestV1,
    ) -> Result<StoredPostTrainingOosCohortV1, Step3OrchestratorRefusal> {
        root.require_same("before derived stored OOS source load")?;
        let stored = load_bounded_stored_context_from_spec_v1(
            StoredContextLoadSpecV1 {
                vendor: self.stored.signal.vendor,
                underlying: self.stored.signal.key.underlying.as_str(),
                rung_name: self.stored.signal.timeframe,
                from: request.from(),
                to: request.to(),
                signal_bound: request.signal_bound(),
                minute_bound: request.minute_bound(),
                daily_bound: request.daily_bound(),
            },
            root,
        )?;
        if stored.signal.vendor != self.stored.signal.vendor
            || stored.signal.key != self.stored.signal.key
            || stored.signal.timeframe != self.stored.signal.timeframe
            || stored.rung_seconds != self.stored.rung_seconds
        {
            return Err(
                "Step 3 stored OOS loader returned a foreign feed, instrument or signal rung"
                    .to_owned(),
            );
        }
        let execution_range =
            requested_execution_range(&stored.minute.bars, stored.first_day, stored.last_day)?;
        root.require_same("after derived stored OOS source load")?;

        // Hold a second capability for the cohort's lifetime. The original
        // retained source remains borrowed by this method and cannot be moved
        // into a later Selection/Replay successor.
        let cohort_root = AdmittedRootV1::admit(root.path())?;
        root.require_same("after stored OOS cohort root admission")?;
        cohort_root.require_same("before stored OOS cohort construction")?;
        StoredPostTrainingOosCohortV1::from_retained(
            cohort_root,
            stored,
            execution_range,
            self.source_commit.clone(),
            self.family,
            self.sweeper_ladder,
            self.horizon,
            self.widths,
            self.availability,
            self.thresholds,
            StoredLoadCeilingsV1 {
                signal: request.signal_bound().max_records(),
                minute: request.minute_bound().max_records(),
                daily: request.daily_bound().max_records(),
            },
            self.long.clone(),
            self.short.clone(),
        )
        .map_err(|why| format!("Step 3 stored post-training OOS cohort refused: {why}"))
    }

    fn stored_facts(
        &self,
        committed: &CommittedCandidatePreAdmissionV1,
    ) -> Result<StoredExecutionJoinFactsV1, Step3OrchestratorRefusal> {
        let execution = self
            .stored
            .minute
            .bars
            .get(self.execution_range.clone())
            .ok_or_else(|| {
                "Step 3 retained one-minute execution range no longer fits its owned stored context"
                    .to_owned()
            })?;
        let calendar = crate::stored::calendar_receipt_v2_for_bars(
            execution,
            60,
            self.stored.first_day,
            self.stored.last_day,
        )
        .and_then(CalendarReceiptV2::require_complete)
        .map_err(|why| format!("Step 3 retained execution calendar refused: {why}"))?;
        let series = ExecutionSeriesV1::new(
            &self.stored.signal.key,
            self.stored.signal.vendor.as_str(),
            &self.source_commit,
            crate::stored::calendar_policy_digest_v2(),
            execution,
        )
        .map_err(|why| format!("Step 3 retained execution series refused: {why}"))?;
        let resolved = ResolvedExecutionContextV1 {
            series,
            execution_range: self.execution_range.clone(),
            calendar,
            family: self.family,
            long: self.long.clone(),
            short: self.short.clone(),
        };
        let daily_reference = DailyReferenceBinding {
            daily_bars: &self.stored.daily.bars,
            eligibility: &self.stored.daily.eligibility,
            schema: DAILY_REFERENCE_SCHEMA,
            eligibility_policy: DAILY_ELIGIBILITY_POLICY,
            gap_overlay_policy: EXACT_MINUTE_GAP_POLICY,
            excluded_ist_days: &CHARTER_NON_REGULAR_IST_DAYS,
            daily_integrity: ReferenceIntegrity::UnverifiedNoReceipt,
            minute_integrity: ReferenceIntegrity::UnverifiedNoReceipt,
        };
        let authorities = StoredFactAuthoritiesV1 {
            verified_commit: VerifiedBuildCommitV1(&self.source_commit),
            horizon: self.horizon,
            daily_reference,
            load_ceilings: self.load_ceilings,
            prepared_candidate: committed.prepared_candidate_receipt(),
            prepared_pre_admission: committed.prepared_pre_admission(),
        };
        let facts = stored_execution_facts_v1(&self.stored, &resolved, &authorities)?;

        // These immutable evaluator inputs were accepted by the original
        // Candidate source constructor. There is deliberately no second local
        // evaluator-policy codec: retaining the values is the future exact
        // replay input, while the reopened Candidate receipt remains the sole
        // current identity authority for their derived policy digest.
        let _accepted_evaluation_inputs = (
            runner::identity::Params::of(self.sweeper_ladder),
            self.widths,
            self.availability,
            self.thresholds,
        );
        Ok(facts)
    }

    /// Reconstructs the exact Candidate production source solely from the
    /// retained stored transaction and classifies its freshly reopened rows.
    fn execution_v3_replay(
        &self,
        committed: &CommittedCandidatePreAdmissionV1,
        rows: &[AuthenticatedCandidatePopulationRowV1],
    ) -> Result<CandidateExecutionReplayAuthorityV1, Step3OrchestratorRefusal> {
        let before = self.stored_facts(committed)?;
        let reopened_before = reopened_execution_facts_v1(committed)?;
        require_exact_stored_execution_join(&before, &reopened_before)?;
        let execution = self
            .stored
            .minute
            .bars
            .get(self.execution_range.clone())
            .ok_or_else(|| {
                "Step 3 Execution V3 retained one-minute subspan is outside its owned context"
                    .to_owned()
            })?;
        let execution_calendar = crate::stored::calendar_receipt_v2_for_bars(
            execution,
            60,
            self.stored.first_day,
            self.stored.last_day,
        )
        .and_then(CalendarReceiptV2::require_complete)
        .map_err(|why| format!("Step 3 Execution V3 calendar refused: {why}"))?;
        let execution_series = ExecutionSeriesV1::new(
            &self.stored.signal.key,
            self.stored.signal.vendor.as_str(),
            &self.source_commit,
            crate::stored::calendar_policy_digest_v2(),
            execution,
        )
        .map_err(|why| format!("Step 3 Execution V3 series refused: {why}"))?;
        let daily_reference = DailyReferenceBinding {
            daily_bars: &self.stored.daily.bars,
            eligibility: &self.stored.daily.eligibility,
            schema: DAILY_REFERENCE_SCHEMA,
            eligibility_policy: DAILY_ELIGIBILITY_POLICY,
            gap_overlay_policy: EXACT_MINUTE_GAP_POLICY,
            excluded_ist_days: &CHARTER_NON_REGULAR_IST_DAYS,
            daily_integrity: ReferenceIntegrity::UnverifiedNoReceipt,
            minute_integrity: ReferenceIntegrity::UnverifiedNoReceipt,
        };
        let signal_bound = StoredSpanLoadBoundV1::new(self.load_ceilings.signal)
            .map_err(|why| format!("Step 3 Execution V3 signal bound refused: {why}"))?;
        let minute_bound = StoredSpanLoadBoundV1::new(self.load_ceilings.minute)
            .map_err(|why| format!("Step 3 Execution V3 minute bound refused: {why}"))?;
        let daily_bound = StoredSpanLoadBoundV1::new(self.load_ceilings.daily)
            .map_err(|why| format!("Step 3 Execution V3 daily bound refused: {why}"))?;
        let source = CandidateUniverseProductionSourceV1::new(
            self.family,
            self.stored.rung_seconds,
            self.horizon,
            self.stored.requested_span,
            self.stored.signal_calendar,
            execution_calendar,
            &self.stored.signal.bars,
            &self.stored.daily.references,
            &self.stored.minute.bars,
            daily_reference,
            execution_series,
            self.widths,
            self.availability,
            self.thresholds,
            &self.long,
            &self.short,
            signal_bound,
            minute_bound,
            daily_bound,
        )
        .map_err(|why| format!("Step 3 Execution V3 Candidate source refused: {why}"))?;
        if source.search_splits() != self.search_splits {
            return Err("Step 3 Execution V3 rebuilt Candidate split policy changed".to_owned());
        }
        let authority = source
            .execution_v3_replay_authority(
                self.sweeper_ladder,
                committed.candidate_audit().receipt(),
                rows,
            )
            .map_err(|why| format!("Step 3 Execution V3 Candidate replay refused: {why}"))?;
        let after = self.stored_facts(committed)?;
        let reopened_after = reopened_execution_facts_v1(committed)?;
        if before != after || reopened_before != reopened_after {
            return Err(
                "Step 3 Execution V3 retained or reopened source changed during replay".to_owned(),
            );
        }
        require_exact_stored_execution_join(&after, &reopened_after)?;
        Ok(authority)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ExactStreamFactsV1 {
    count: u64,
    first_ts_micros: i64,
    last_ts_micros: i64,
    digest: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StoredEligibilityFactsV1 {
    count: u64,
    eligible_count: u64,
    excluded_day_count: u64,
    eligibility_digest: [u8; 32],
    excluded_days_digest: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StoredLoadCeilingsV1 {
    pub(crate) signal: u64,
    pub(crate) minute: u64,
    pub(crate) daily: u64,
}

struct StoredFactAuthoritiesV1<'a> {
    verified_commit: VerifiedBuildCommitV1<'a>,
    horizon: Horizon,
    daily_reference: DailyReferenceBinding<'a>,
    load_ceilings: StoredLoadCeilingsV1,
    prepared_candidate: &'a CandidateUniverseReceiptV1,
    prepared_pre_admission: &'a PreAdmissionDataV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StoredExecutionJoinFactsV1 {
    instrument: InstrumentKey,
    family: InstrumentFamilyV1,
    rung_seconds: u32,
    horizon_bars: u32,
    requested_span: RequestedSpanIdentityV1,
    feed_digest: [u8; 32],
    source_commit_digest: [u8; 32],
    calendar_policy_digest: [u8; 32],
    daily_reference_policy_digest: [u8; 32],
    signal_calendar_digest: [u8; 32],
    execution_calendar_digest: [u8; 32],
    composite_data_digest: [u8; 32],
    evaluation_policy_digest: [u8; 32],
    signal: ExactStreamFactsV1,
    minute_context: ExactStreamFactsV1,
    execution: ExactStreamFactsV1,
    daily: ExactStreamFactsV1,
    eligibility: StoredEligibilityFactsV1,
    execution_start_index: u64,
    load_ceilings: StoredLoadCeilingsV1,
    exit_grids: LongShortExitGridIdentitiesV2,
}

/// Load, derive, commit and reopen one stored Candidate-to-Pre-Admission chain.
///
/// The build commit is proved before any store path is opened. Signal, exact
/// minute and one-day streams then pass their independent pre-allocation
/// ceilings. Complete calendar receipts and dynamic exit-grid resolutions are
/// minted from those exact admitted bytes; only the reopened Candidate and
/// Pre-Admission identities escape.
///
/// # Cost
///
/// Loading is O(M + B) for M admitted months and B stored bars. Calendar
/// attestation is O(B), exit-grid resolution is O(E log E) for its bounded
/// execution samples, and Candidate production follows the naturally-extinct
/// Apriori frontier and complete grid cardinality. Returned auxiliary space is
/// bounded by the three explicit record ceilings and two ledger ceilings. This
/// is not claimed to be O(1) whole-run work or space. The public observer is
/// outside this cost contract: it is caller code, may block or perform arbitrary
/// work, and is invoked only for legacy progress compatibility. The retained
/// crate-private authority below installs its own no-op observer instead.
///
/// # Errors
///
/// Refuses a dirty/unstamped build before reading, an invalid span or rung,
/// missing/foreign/incomplete stored data, any load ceiling, an incomplete
/// calendar, malformed execution subspan, exit-grid policy failure, Candidate
/// production/commit failure, Pre-Admission derivation/commit failure, or a
/// cross-ledger identity mismatch. It never substitutes another feed, rung,
/// instrument, calendar, grid or commit.
pub fn commit_stored_candidate_pre_admission_v1(
    request: StoredCandidatePreAdmissionRequestV1<'_>,
    on_level: &dyn Fn(&engine::Frontier, usize, u64),
) -> Result<CandidatePreAdmissionReopenIdentitiesV1, Step3OrchestratorRefusal> {
    let verified_commit = VerifiedBuildCommitV1::current()?;
    commit_stored_with_verified_build_v1(request, verified_commit, on_level)
        .map(|committed| committed.identities())
}

/// Commit the same public stored request while retaining its opaque sources.
///
/// This door adds no input. It is deliberately crate-private, accepts the exact
/// public request unchanged, and owns a no-op progress observer, so a successor
/// cannot inject bars, digests, calendars, commits, observer behavior, a network
/// feed, a depth cap or a pre-resolved result.
pub(crate) fn commit_stored_candidate_pre_admission_authority_v1(
    request: StoredCandidatePreAdmissionRequestV1<'_>,
) -> Result<CommittedStoredCandidatePreAdmissionV1, Step3OrchestratorRefusal> {
    let verified_commit = VerifiedBuildCommitV1::current()?;
    let no_progress_observer = |_: &engine::Frontier, _: usize, _: u64| {};
    commit_stored_with_verified_build_v1(request, verified_commit, &no_progress_observer)
}

fn commit_stored_with_verified_build_v1(
    request: StoredCandidatePreAdmissionRequestV1<'_>,
    verified_commit: VerifiedBuildCommitV1<'_>,
    on_level: &dyn Fn(&engine::Frontier, usize, u64),
) -> Result<CommittedStoredCandidatePreAdmissionV1, Step3OrchestratorRefusal> {
    let root = AdmittedRootV1::admit(request.root)?;
    let context = load_bounded_stored_context_v1(&request, &root)?;
    let resolved = resolve_stored_execution_v1(
        &context,
        verified_commit,
        request.long_exit_policy,
        request.short_exit_policy,
    )?;
    let daily_reference = DailyReferenceBinding {
        daily_bars: &context.daily.bars,
        eligibility: &context.daily.eligibility,
        schema: DAILY_REFERENCE_SCHEMA,
        eligibility_policy: DAILY_ELIGIBILITY_POLICY,
        gap_overlay_policy: EXACT_MINUTE_GAP_POLICY,
        excluded_ist_days: &CHARTER_NON_REGULAR_IST_DAYS,
        daily_integrity: ReferenceIntegrity::UnverifiedNoReceipt,
        minute_integrity: ReferenceIntegrity::UnverifiedNoReceipt,
    };
    let source = CandidateUniverseProductionSourceV1::new(
        resolved.family,
        context.rung_seconds,
        request.horizon,
        context.requested_span,
        context.signal_calendar,
        resolved.calendar,
        &context.signal.bars,
        &context.daily.references,
        &context.minute.bars,
        daily_reference,
        resolved.series,
        request.widths,
        request.availability,
        request.thresholds,
        &resolved.long,
        &resolved.short,
        request.bounds.signal_records,
        request.bounds.minute_records,
        request.bounds.daily_records,
    )
    .map_err(|why| format!("Step 3 stored Candidate source refused: {why}"))?;

    let search_splits = source.search_splits();
    let search_validation = source
        .anchored_search_v4(request.sweeper)
        .map_err(|why| format!("Step 3 retained stored Search V4 refused: {why}"))?;

    let committed = commit_candidate_pre_admission_authority_guarded_v1(
        root.path(),
        Some(&root),
        request.sweeper,
        source,
        request.bounds.candidate,
        request.bounds.pre_admission,
        on_level,
    )?;
    let search = StoredSearchMemberV4::bind(&committed, search_validation)?;
    let execution_range = resolved.execution_range.clone();
    let family = resolved.family;
    let long = resolved.long.clone();
    let short = resolved.short.clone();
    drop(resolved);

    let execution = RetainedStoredExecutionContextV1 {
        stored: context,
        execution_range,
        source_commit: verified_commit.0.to_owned(),
        family,
        sweeper_ladder: request.sweeper.ladder(),
        horizon: request.horizon,
        widths: request.widths,
        availability: request.availability,
        thresholds: request.thresholds,
        search_splits,
        load_ceilings: StoredLoadCeilingsV1 {
            signal: request.bounds.signal_records.max_records(),
            minute: request.bounds.minute_records.max_records(),
            daily: request.bounds.daily_records.max_records(),
        },
        long,
        short,
    };
    root.require_same("before final retained/reopened authority join")?;
    let runtime_facts = execution.stored_facts(&committed)?;
    let reopened_facts = reopened_execution_facts_v1(&committed)?;
    require_exact_stored_execution_join(&runtime_facts, &reopened_facts)?;
    require_reopened_pre_admission_context_v1(&committed, &runtime_facts)?;
    let reopened_signal_bars = usize::try_from(reopened_facts.signal.count)
        .map_err(|_| "Step 3 reopened signal count does not fit usize".to_owned())?;
    if execution.search_splits != crate::walk_forward_splits(reopened_signal_bars)
        || execution.search_splits < 2
    {
        return Err(
            "Step 3 retained Search V4 split count differs from the canonical reopened signal policy"
                .to_owned(),
        );
    }
    Ok(CommittedStoredCandidatePreAdmissionV1 {
        root,
        committed,
        search,
        execution,
    })
}

fn load_bounded_stored_context_v1(
    request: &StoredCandidatePreAdmissionRequestV1<'_>,
    root: &AdmittedRootV1,
) -> Result<BoundedStoredContextV1, Step3OrchestratorRefusal> {
    load_bounded_stored_context_from_spec_v1(
        StoredContextLoadSpecV1 {
            vendor: request.vendor,
            underlying: request.underlying,
            rung_name: request.rung_name,
            from: request.from,
            to: request.to,
            signal_bound: request.bounds.signal_records,
            minute_bound: request.bounds.minute_records,
            daily_bound: request.bounds.daily_records,
        },
        root,
    )
}

/// One internal stored loader request. Market facts come either from the public
/// initial transaction or, for OOS, from its retained opaque source.
#[derive(Clone, Copy)]
struct StoredContextLoadSpecV1<'a> {
    vendor: Vendor,
    underlying: &'a str,
    rung_name: &'a str,
    from: (u16, u8),
    to: (u16, u8),
    signal_bound: StoredSpanLoadBoundV1,
    minute_bound: StoredSpanLoadBoundV1,
    daily_bound: StoredSpanLoadBoundV1,
}

fn load_bounded_stored_context_from_spec_v1(
    spec: StoredContextLoadSpecV1<'_>,
    root: &AdmittedRootV1,
) -> Result<BoundedStoredContextV1, Step3OrchestratorRefusal> {
    let requested_span =
        RequestedSpanIdentityV1::new(spec.from.0, spec.from.1, spec.to.0, spec.to.1)
            .map_err(|why| format!("Step 3 requested span refused: {why}"))?;
    let (first_day, last_day) = requested_span_days(requested_span)?;

    root.require_same("before bounded signal load")?;
    let signal = crate::stored::load_span_bounded(
        root.path(),
        spec.vendor,
        spec.underlying,
        spec.rung_name,
        spec.from,
        spec.to,
        spec.signal_bound,
    )
    .map_err(|why| format!("Step 3 bounded signal load refused: {why}"))?;
    root.require_same("after bounded signal load")?;
    require_complete_signal_span(&signal)?;

    let rung_micros = crate::stored::rung_length_micros(signal.timeframe)
        .map_err(|why| format!("Step 3 canonical signal rung refused: {why}"))?;
    let rung_seconds = rung_seconds(rung_micros)?;
    let signal_calendar = crate::stored::calendar_receipt_v2_for_bars(
        &signal.bars,
        rung_seconds,
        first_day,
        last_day,
    )
    .and_then(CalendarReceiptV2::require_complete)
    .map_err(|why| format!("Step 3 signal calendar refused: {why}"))?;

    root.require_same("before bounded daily-reference load")?;
    let daily = crate::stored::load_daily_context_bounded(
        root.path(),
        signal.vendor,
        signal.key.underlying.as_str(),
        (spec.from, spec.to),
        &signal.bars,
        spec.daily_bound,
    )
    .map_err(|why| format!("Step 3 bounded daily-reference load refused: {why}"))?;
    root.require_same("after bounded daily-reference load")?;
    root.require_same("before bounded exact-minute load")?;
    let minute = crate::stored::load_exact_minute_context_bounded(
        root.path(),
        signal.vendor,
        signal.key.underlying.as_str(),
        (spec.from, spec.to),
        &signal.bars,
        spec.minute_bound,
    )
    .map_err(|why| format!("Step 3 bounded exact-minute load refused: {why}"))?;
    root.require_same("after bounded exact-minute load")?;
    Ok(BoundedStoredContextV1 {
        requested_span,
        first_day,
        last_day,
        rung_seconds,
        signal_calendar,
        signal,
        daily,
        minute,
    })
}

fn resolve_stored_execution_v1<'a>(
    context: &'a BoundedStoredContextV1,
    verified_commit: VerifiedBuildCommitV1<'a>,
    long_policy: &ExitGridPolicyV1,
    short_policy: &ExitGridPolicyV1,
) -> Result<ResolvedExecutionContextV1<'a>, Step3OrchestratorRefusal> {
    let execution_range =
        requested_execution_range(&context.minute.bars, context.first_day, context.last_day)?;
    let execution = context
        .minute
        .bars
        .get(execution_range.clone())
        .ok_or_else(|| {
            "Step 3 exact-minute requested execution subspan disappeared after validation"
                .to_owned()
        })?;
    let calendar = crate::stored::calendar_receipt_v2_for_bars(
        execution,
        60,
        context.first_day,
        context.last_day,
    )
    .and_then(CalendarReceiptV2::require_complete)
    .map_err(|why| format!("Step 3 exact-minute execution calendar refused: {why}"))?;
    let series = ExecutionSeriesV1::new(
        &context.signal.key,
        context.signal.vendor.as_str(),
        verified_commit.0,
        crate::stored::calendar_policy_digest_v2(),
        execution,
    )
    .map_err(|why| format!("Step 3 execution-series authority refused: {why}"))?;
    let long = long_policy
        .resolve_attested(series)
        .map_err(|why| format!("Step 3 long exit-grid resolution refused: {why}"))?;
    let short = short_policy
        .resolve_attested(series)
        .map_err(|why| format!("Step 3 short exit-grid resolution refused: {why}"))?;
    let family = match long.family() {
        ExitInstrumentFamilyV1::Nifty => InstrumentFamilyV1::Nifty,
        ExitInstrumentFamilyV1::BankNifty => InstrumentFamilyV1::BankNifty,
    };
    Ok(ResolvedExecutionContextV1 {
        series,
        execution_range,
        calendar,
        family,
        long,
        short,
    })
}

fn requested_span_days(
    requested_span: RequestedSpanIdentityV1,
) -> Result<(i64, i64), Step3OrchestratorRefusal> {
    let first = Day::new(requested_span.from_year(), requested_span.from_month(), 1)
        .map_err(|why| format!("Step 3 requested start day refused: {why}"))?;
    let last = Day::new(requested_span.to_year(), requested_span.to_month(), 1)
        .map_err(|why| format!("Step 3 requested end month refused: {why}"))?
        .end_of_month();
    Ok((
        i64::from(first.days_from_epoch()),
        i64::from(last.days_from_epoch()),
    ))
}

fn rung_seconds(rung_micros: i64) -> Result<u32, Step3OrchestratorRefusal> {
    if rung_micros <= 0 || rung_micros.rem_euclid(1_000_000) != 0 {
        return Err(format!(
            "Step 3 canonical rung length {rung_micros} microseconds is not a positive whole-second duration"
        ));
    }
    u32::try_from(rung_micros / 1_000_000).map_err(|_| {
        format!("Step 3 canonical rung length {rung_micros} microseconds does not fit u32 seconds")
    })
}

fn require_complete_signal_span(
    signal: &crate::stored::Span,
) -> Result<(), Step3OrchestratorRefusal> {
    if signal.complete() {
        return Ok(());
    }
    let missing = signal
        .missing
        .iter()
        .map(|(year, month)| format!("{year}-{month:02}"))
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "Step 3 signal stream is incomplete: missing stored month(s) {missing}; no smaller span or alternate feed was substituted"
    ))
}

#[cfg(test)]
fn requested_execution_subspan(
    minute_context: &[Candle],
    first_day: i64,
    last_day: i64,
) -> Result<&[Candle], Step3OrchestratorRefusal> {
    let range = requested_execution_range(minute_context, first_day, last_day)?;
    minute_context.get(range).ok_or_else(|| {
        "Step 3 exact-minute requested execution subspan disappeared after validation".to_owned()
    })
}

fn requested_execution_range(
    minute_context: &[Candle],
    first_day: i64,
    last_day: i64,
) -> Result<Range<usize>, Step3OrchestratorRefusal> {
    if first_day > last_day {
        return Err(format!(
            "Step 3 exact-minute requested execution subspan bounds are inconsistent: {first_day} is after {last_day}"
        ));
    }
    // One monotone pass replaces the former pair of binary partition searches.
    // Gate 11 deliberately permits no data-sized search in crate source, and
    // the bounded stored load already has to be validated once.  Keeping both
    // boundaries in the same pass also makes a future ordering regression loud
    // instead of relying on the unstated precondition of a partition search.
    let mut first = None;
    let mut end = 0_usize;
    let mut previous_day = None;
    for (index, bar) in minute_context.iter().enumerate() {
        let day = indicators::ist_day(bar.ts_micros);
        if previous_day.is_some_and(|previous| day < previous) {
            return Err(
                "Step 3 exact-minute context IST days are not monotonically ordered".to_owned(),
            );
        }
        previous_day = Some(day);
        if day < first_day {
            continue;
        }
        if day > last_day {
            break;
        }
        if first.is_none() {
            first = Some(index);
        }
        end = index.checked_add(1).ok_or_else(|| {
            "Step 3 exact-minute requested execution end index overflowed".to_owned()
        })?;
    }
    let first = first.ok_or_else(|| {
        format!(
            "Step 3 exact-minute context has no bars inside requested IST days {first_day}..={last_day}"
        )
    })?;
    let execution = minute_context.get(first..end).ok_or_else(|| {
        "Step 3 exact-minute requested execution subspan bounds are inconsistent".to_owned()
    })?;
    if execution.is_empty() {
        return Err(format!(
            "Step 3 exact-minute context has no bars inside requested IST days {first_day}..={last_day}"
        ));
    }
    Ok(first..end)
}

fn exact_stream_facts_v1(
    name: &str,
    bars: &[Candle],
) -> Result<ExactStreamFactsV1, Step3OrchestratorRefusal> {
    let first = bars
        .first()
        .ok_or_else(|| format!("Step 3 retained {name} stream is empty"))?;
    let last = bars
        .last()
        .ok_or_else(|| format!("Step 3 retained {name} stream lost its last bar"))?;
    Ok(ExactStreamFactsV1 {
        count: u64::try_from(bars.len())
            .map_err(|_| format!("Step 3 retained {name} stream length does not fit u64"))?,
        first_ts_micros: first.ts_micros,
        last_ts_micros: last.ts_micros,
        digest: runner::identity::data_digest(bars),
    })
}

fn canonical_instrument_v1(
    family: InstrumentFamilyV1,
) -> Result<InstrumentKey, Step3OrchestratorRefusal> {
    let underlying = match family {
        InstrumentFamilyV1::Nifty => "NIFTY",
        InstrumentFamilyV1::BankNifty => "BANKNIFTY",
    };
    InstrumentKey::index(Exchange::Nse, underlying)
        .map_err(|why| format!("Step 3 canonical {underlying} instrument refused: {why}"))
}

fn resolved_grid_identities_v1(
    long: &ResolvedExitGridV1,
    short: &ResolvedExitGridV1,
) -> LongShortExitGridIdentitiesV2 {
    LongShortExitGridIdentitiesV2 {
        long: SideExitGridIdentityV2 {
            policy_digest: long.policy_digest(),
            resolved_digest: long.digest(),
        },
        short: SideExitGridIdentityV2 {
            policy_digest: short.policy_digest(),
            resolved_digest: short.digest(),
        },
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the stored fact seal validates every source authority before constructing one atomic join value"
)]
fn stored_execution_facts_v1(
    context: &BoundedStoredContextV1,
    resolved: &ResolvedExecutionContextV1<'_>,
    authorities: &StoredFactAuthoritiesV1<'_>,
) -> Result<StoredExecutionJoinFactsV1, Step3OrchestratorRefusal> {
    let verified_commit = authorities.verified_commit;
    let horizon = authorities.horizon;
    let daily_reference = authorities.daily_reference;
    let load_ceilings = authorities.load_ceilings;
    let prepared_candidate = authorities.prepared_candidate;
    let prepared_pre_admission = authorities.prepared_pre_admission;
    let canonical = canonical_instrument_v1(resolved.family)?;
    if context.signal.key != canonical
        || resolved.series.instrument() != &canonical
        || resolved.long.instrument() != canonical
        || resolved.short.instrument() != canonical
    {
        return Err(
            "Step 3 retained stored execution instrument differs from its canonical NSE family"
                .to_owned(),
        );
    }
    if resolved.series.feed() != context.signal.vendor.as_str() {
        return Err(
            "Step 3 retained stored execution feed differs from the exact signal store prefix"
                .to_owned(),
        );
    }
    if resolved.series.commit() != verified_commit.0 {
        return Err(
            "Step 3 retained stored execution commit differs from the verified build authority"
                .to_owned(),
        );
    }
    if resolved.series.calendar_digest() != crate::stored::calendar_policy_digest_v2()
        || resolved.long.calendar_digest() != resolved.series.calendar_digest()
        || resolved.short.calendar_digest() != resolved.series.calendar_digest()
    {
        return Err(
            "Step 3 retained stored execution calendar policy differs across series and grids"
                .to_owned(),
        );
    }
    let minute_context = exact_stream_facts_v1("minute context", &context.minute.bars)?;
    let daily = exact_stream_facts_v1("daily-reference", &context.daily.bars)?;
    let composite_data_digest = runner::identity::data_digest_with_daily_reference(
        &context.signal.bars,
        &context.minute.bars,
        daily_reference,
    )
    .map_err(|why| format!("Step 3 retained composite stored-data identity refused: {why:?}"))?;
    let prepared_identities = prepared_candidate.identities();
    if composite_data_digest != prepared_identities.data_digest() {
        return Err(
            "Step 3 retained signal/minute/daily bytes differ from the exact prepared Candidate composite data identity"
                .to_owned(),
        );
    }
    let execution_start_index = u64::try_from(resolved.execution_range.start)
        .map_err(|_| "Step 3 retained execution start index does not fit u64".to_owned())?;
    let prepared_minute =
        exact_stream_facts_from_pre_admission_v1(prepared_pre_admission.minute_context());
    let prepared_daily = exact_stream_facts_from_pre_admission_v1(prepared_pre_admission.daily());
    if minute_context != prepared_minute
        || daily != prepared_daily
        || execution_start_index != prepared_pre_admission.execution_start_index()
    {
        return Err(
            "Step 3 retained minute/daily/execution-offset facts differ from exact in-process Pre-Admission preparation"
                .to_owned(),
        );
    }
    let eligibility = stored_eligibility_facts_v1(
        &context.daily.eligibility,
        prepared_pre_admission.eligibility(),
    )?;
    let prepared_load_ceilings = StoredLoadCeilingsV1 {
        signal: prepared_pre_admission.signal_load_ceiling(),
        minute: prepared_pre_admission.minute_load_ceiling(),
        daily: prepared_pre_admission.daily_load_ceiling(),
    };
    if load_ceilings != prepared_load_ceilings {
        return Err(
            "Step 3 retained stored-load ceilings differ from exact in-process Pre-Admission preparation"
                .to_owned(),
        );
    }
    Ok(StoredExecutionJoinFactsV1 {
        instrument: canonical,
        family: resolved.family,
        rung_seconds: context.rung_seconds,
        horizon_bars: horizon.as_bars(),
        requested_span: context.requested_span,
        feed_digest: hash(resolved.series.feed().as_bytes()),
        source_commit_digest: hash(verified_commit.0.as_bytes()),
        calendar_policy_digest: resolved.series.calendar_digest(),
        daily_reference_policy_digest:
            crate::stored_data_completeness::daily_reference_policy_digest_v1(daily_reference),
        signal_calendar_digest: context.signal_calendar.digest(),
        execution_calendar_digest: resolved.calendar.digest(),
        composite_data_digest,
        evaluation_policy_digest: prepared_identities.evaluation_policy_digest(),
        signal: exact_stream_facts_v1("signal", &context.signal.bars)?,
        minute_context,
        execution: exact_stream_facts_v1("execution", resolved.series.bars())?,
        daily,
        eligibility,
        execution_start_index,
        load_ceilings,
        exit_grids: resolved_grid_identities_v1(&resolved.long, &resolved.short),
    })
}

fn exact_stream_facts_from_pre_admission_v1(
    facts: crate::pre_admission_data::PreAdmissionStreamFactsV1,
) -> ExactStreamFactsV1 {
    ExactStreamFactsV1 {
        count: facts.count(),
        first_ts_micros: facts.first_ts_micros(),
        last_ts_micros: facts.last_ts_micros(),
        digest: facts.digest(),
    }
}

fn stored_eligibility_facts_v1(
    eligibility: &[u8],
    prepared: crate::pre_admission_data::PreAdmissionEligibilityFactsV1,
) -> Result<StoredEligibilityFactsV1, Step3OrchestratorRefusal> {
    let count = u64::try_from(eligibility.len())
        .map_err(|_| "Step 3 retained eligibility count does not fit u64".to_owned())?;
    let mut eligible_count = 0_u64;
    for (index, decision) in eligibility.iter().copied().enumerate() {
        match decision {
            0 => {}
            1 => {
                eligible_count = eligible_count.checked_add(1).ok_or_else(|| {
                    "Step 3 retained eligible-day count overflowed u64".to_owned()
                })?;
            }
            other => {
                return Err(format!(
                    "Step 3 retained eligibility byte {index} has unknown value {other}"
                ));
            }
        }
    }
    if count != prepared.count() || eligible_count != prepared.eligible_count() {
        return Err(
            "Step 3 retained eligibility count differs from exact in-process Pre-Admission preparation"
                .to_owned(),
        );
    }
    Ok(StoredEligibilityFactsV1 {
        count,
        eligible_count,
        excluded_day_count: prepared.excluded_day_count(),
        eligibility_digest: prepared.eligibility_digest(),
        excluded_days_digest: prepared.excluded_days_digest(),
    })
}

fn reopened_execution_facts_v1(
    committed: &CommittedCandidatePreAdmissionV1,
) -> Result<StoredExecutionJoinFactsV1, Step3OrchestratorRefusal> {
    let receipt = committed.candidate_audit().receipt();
    let signal = receipt.signal_stream();
    let execution = receipt.execution_stream();
    let calendars = receipt.calendar_coverage();
    let identities = receipt.identities();
    let pre_admission = committed.pre_admission_audit().value();
    Ok(StoredExecutionJoinFactsV1 {
        instrument: canonical_instrument_v1(receipt.family())?,
        family: receipt.family(),
        rung_seconds: receipt.rung_seconds(),
        horizon_bars: receipt.horizon_bars(),
        requested_span: receipt.requested_span(),
        feed_digest: identities.feed_digest(),
        source_commit_digest: identities.source_commit_digest(),
        calendar_policy_digest: identities.calendar_policy_digest(),
        daily_reference_policy_digest: identities.daily_reference_policy_digest(),
        signal_calendar_digest: calendars.signal_receipt_digest(),
        execution_calendar_digest: calendars.execution_receipt_digest(),
        composite_data_digest: identities.data_digest(),
        evaluation_policy_digest: identities.evaluation_policy_digest(),
        signal: ExactStreamFactsV1 {
            count: signal.count(),
            first_ts_micros: signal.first_ts_micros(),
            last_ts_micros: signal.last_ts_micros(),
            digest: signal.digest(),
        },
        minute_context: exact_stream_facts_from_pre_admission_v1(pre_admission.minute_context()),
        execution: ExactStreamFactsV1 {
            count: execution.count(),
            first_ts_micros: execution.first_ts_micros(),
            last_ts_micros: execution.last_ts_micros(),
            digest: execution.digest(),
        },
        daily: exact_stream_facts_from_pre_admission_v1(pre_admission.daily()),
        eligibility: StoredEligibilityFactsV1 {
            count: pre_admission.eligibility().count(),
            eligible_count: pre_admission.eligibility().eligible_count(),
            excluded_day_count: pre_admission.eligibility().excluded_day_count(),
            eligibility_digest: pre_admission.eligibility().eligibility_digest(),
            excluded_days_digest: pre_admission.eligibility().excluded_days_digest(),
        },
        execution_start_index: pre_admission.execution_start_index(),
        load_ceilings: StoredLoadCeilingsV1 {
            signal: pre_admission.signal_load_ceiling(),
            minute: pre_admission.minute_load_ceiling(),
            daily: pre_admission.daily_load_ceiling(),
        },
        exit_grids: identities.exit_grids(),
    })
}

fn require_exact_stored_execution_join(
    runtime: &StoredExecutionJoinFactsV1,
    reopened: &StoredExecutionJoinFactsV1,
) -> Result<(), Step3OrchestratorRefusal> {
    let mismatched = if runtime.instrument != reopened.instrument {
        "instrument"
    } else if runtime.family != reopened.family {
        "instrument family"
    } else if runtime.rung_seconds != reopened.rung_seconds {
        "signal rung"
    } else if runtime.horizon_bars != reopened.horizon_bars {
        "one-minute horizon"
    } else if runtime.requested_span != reopened.requested_span {
        "requested month span"
    } else if runtime.feed_digest != reopened.feed_digest {
        "feed"
    } else if runtime.source_commit_digest != reopened.source_commit_digest {
        "source commit"
    } else if runtime.calendar_policy_digest != reopened.calendar_policy_digest {
        "calendar policy"
    } else if runtime.daily_reference_policy_digest != reopened.daily_reference_policy_digest {
        "one-day reference policy"
    } else if runtime.signal_calendar_digest != reopened.signal_calendar_digest {
        "signal complete calendar"
    } else if runtime.execution_calendar_digest != reopened.execution_calendar_digest {
        "one-minute complete calendar"
    } else if runtime.composite_data_digest != reopened.composite_data_digest {
        "composite signal/minute/daily data"
    } else if runtime.evaluation_policy_digest != reopened.evaluation_policy_digest {
        "evaluator policy"
    } else if runtime.signal != reopened.signal {
        "signal stream"
    } else if runtime.minute_context != reopened.minute_context {
        "complete one-minute context"
    } else if runtime.execution != reopened.execution {
        "one-minute execution stream"
    } else if runtime.daily != reopened.daily {
        "one-day reference stream"
    } else if runtime.eligibility != reopened.eligibility {
        "one-day eligibility evidence"
    } else if runtime.execution_start_index != reopened.execution_start_index {
        "execution start index"
    } else if runtime.load_ceilings.signal != reopened.load_ceilings.signal {
        "signal load ceiling"
    } else if runtime.load_ceilings.minute != reopened.load_ceilings.minute {
        "minute-context load ceiling"
    } else if runtime.load_ceilings.daily != reopened.load_ceilings.daily {
        "daily-reference load ceiling"
    } else if runtime.exit_grids != reopened.exit_grids {
        "long/short exit grids"
    } else {
        return Ok(());
    };
    Err(format!(
        "Step 3 retained stored {mismatched} differs from the freshly reopened Candidate/Pre-Admission authority"
    ))
}

fn require_reopened_pre_admission_context_v1(
    committed: &CommittedCandidatePreAdmissionV1,
    runtime: &StoredExecutionJoinFactsV1,
) -> Result<(), Step3OrchestratorRefusal> {
    let value = committed.pre_admission_audit().value();
    let signal = value.signal();
    let execution = value.execution();
    let minute_context = exact_stream_facts_from_pre_admission_v1(value.minute_context());
    let daily = exact_stream_facts_from_pre_admission_v1(value.daily());
    let eligibility = value.eligibility();
    let eligibility = StoredEligibilityFactsV1 {
        count: eligibility.count(),
        eligible_count: eligibility.eligible_count(),
        excluded_day_count: eligibility.excluded_day_count(),
        eligibility_digest: eligibility.eligibility_digest(),
        excluded_days_digest: eligibility.excluded_days_digest(),
    };
    if value.family() != runtime.family
        || value.rung_seconds() != runtime.rung_seconds
        || value.horizon_bars() != runtime.horizon_bars
        || value.requested_span() != runtime.requested_span
        || value.feed_digest() != runtime.feed_digest
        || value.source_commit_digest() != runtime.source_commit_digest
        || value.calendar_policy_digest() != runtime.calendar_policy_digest
        || value.daily_reference_policy_digest() != runtime.daily_reference_policy_digest
        || value.signal_calendar().digest() != runtime.signal_calendar_digest
        || value.execution_calendar().digest() != runtime.execution_calendar_digest
        || signal.count() != runtime.signal.count
        || signal.digest() != runtime.signal.digest
        || minute_context != runtime.minute_context
        || execution.count() != runtime.execution.count
        || execution.digest() != runtime.execution.digest
        || daily != runtime.daily
        || eligibility != runtime.eligibility
        || value.execution_start_index() != runtime.execution_start_index
        || value.signal_load_ceiling() != runtime.load_ceilings.signal
        || value.minute_load_ceiling() != runtime.load_ceilings.minute
        || value.daily_load_ceiling() != runtime.load_ceilings.daily
    {
        return Err(
            "Step 3 retained stored context differs from the freshly reopened Pre-Admission authority"
                .to_owned(),
        );
    }
    Ok(())
}

/// Produces, commits and reopens one exact Candidate and its derived
/// Pre-Admission authority.
///
/// The source is an opaque crate-internal capability. The legacy callback
/// receives frontier progress and returns no control value, but it is arbitrary
/// caller code: its work, blocking and side effects are outside this contract.
/// The retained stored authority does not accept it and owns a no-op observer.
/// There is no sweep-depth parameter: the supplied [`Sweeper`] walks to natural
/// extinction.
///
/// # Errors
///
/// Refuses the first failed source validation, naturally-extinct production,
/// receipt-last Candidate append/reopen, Candidate-to-Pre-Admission derivation,
/// Pre-Admission append/reopen, or final cross-ledger identity comparison.  A
/// refusal returns no success identity and never falls back to another stream.
#[expect(
    dead_code,
    reason = "compatibility projection retained while Step-3 successors consume the opaque authority"
)]
pub(crate) fn commit_candidate_pre_admission_v1<'a>(
    root: &Path,
    sweeper: &Sweeper,
    source: CandidateUniverseProductionSourceV1<'a>,
    candidate_bounds: CandidateUniverseBoundsV1,
    pre_admission_bounds: PreAdmissionDataBoundsV1,
    on_level: &dyn Fn(&engine::Frontier, usize, u64),
) -> Result<CandidatePreAdmissionReopenIdentitiesV1, Step3OrchestratorRefusal> {
    commit_candidate_pre_admission_authority_v1(
        root,
        sweeper,
        source,
        candidate_bounds,
        pre_admission_bounds,
        on_level,
    )
    .map(|committed| committed.identities())
}

/// Produces the exact reopen audits while retaining the sealed observations.
///
/// This is the crate-private successor seam for paired Observation V1 and
/// Statistics V2 orchestration.  The existing identity-only function above is
/// a lossless projection of this value and keeps its public behavior unchanged.
pub(crate) fn commit_candidate_pre_admission_authority_v1<'a>(
    root: &Path,
    sweeper: &Sweeper,
    source: CandidateUniverseProductionSourceV1<'a>,
    candidate_bounds: CandidateUniverseBoundsV1,
    pre_admission_bounds: PreAdmissionDataBoundsV1,
    on_level: &dyn Fn(&engine::Frontier, usize, u64),
) -> Result<CommittedCandidatePreAdmissionV1, Step3OrchestratorRefusal> {
    commit_candidate_pre_admission_authority_guarded_v1(
        root,
        None,
        sweeper,
        source,
        candidate_bounds,
        pre_admission_bounds,
        on_level,
    )
}

fn commit_candidate_pre_admission_authority_guarded_v1<'a>(
    root: &Path,
    admitted_root: Option<&AdmittedRootV1>,
    sweeper: &Sweeper,
    source: CandidateUniverseProductionSourceV1<'a>,
    candidate_bounds: CandidateUniverseBoundsV1,
    pre_admission_bounds: PreAdmissionDataBoundsV1,
    on_level: &dyn Fn(&engine::Frontier, usize, u64),
) -> Result<CommittedCandidatePreAdmissionV1, Step3OrchestratorRefusal> {
    let candidate = produce_candidate_universe_v1(sweeper, source, candidate_bounds, on_level)
        .map_err(|why| format!("Step 3 Candidate production refused: {why}"))?;
    if !candidate.population_run().is_complete() {
        return Err(
            "Step 3 Candidate producer returned a non-complete naturally-extinct population"
                .to_owned(),
        );
    }
    let prepared_row_count = u64::try_from(candidate.row_count())
        .map_err(|_| "Step 3 prepared Candidate row count does not fit u64".to_owned())?;
    let prepared_receipt = candidate.receipt();
    let observations = candidate.observations().clone();
    require_admitted_root_v1(admitted_root, "before Candidate receipt-last append/reopen")?;
    let candidate_commit = candidate
        .append_and_reopen(root, candidate_bounds)
        .map_err(|why| format!("Step 3 Candidate receipt-last commit refused: {why}"))?;
    require_admitted_root_v1(admitted_root, "after Candidate receipt-last append/reopen")?;
    let candidate_audit = candidate_commit.audit();
    if candidate_audit.receipt() != prepared_receipt
        || candidate_audit.row_count() != prepared_row_count
        || observations.source() != candidate_audit.receipt()
    {
        return Err(
            "Step 3 reopened Candidate audit or sealed observations differ from the exact in-process preparation"
                .to_owned(),
        );
    }

    let base_evidence_bounds = BaseEvidenceLedgerBoundsV2::new(
        candidate_bounds.max_rows(),
        candidate_bounds.max_universes(),
    )
    .map_err(|why| format!("Step 3 Base Evidence bounds refused: {why}"))?;
    require_admitted_root_v1(
        admitted_root,
        "before Base Evidence receipt-last append/reopen",
    )?;
    let base_evidence = candidate
        .append_base_evidence_and_reopen(root, base_evidence_bounds, &candidate_audit)
        .map_err(|why| format!("Step 3 Base Evidence receipt-last commit refused: {why}"))?;
    require_admitted_root_v1(
        admitted_root,
        "after Base Evidence receipt-last append/reopen",
    )?;
    let base_evidence_audit = base_evidence.audit();
    if base_evidence_audit.candidate_universe_id() != candidate_audit.universe_id()
        || base_evidence_audit.record_count() != candidate_audit.row_count()
        || base_evidence_audit.family() != candidate_audit.receipt().family()
    {
        return Err(
            "Step 3 reopened Base Evidence differs from its exact Candidate authority".to_owned(),
        );
    }

    let pre_admission = produce_pre_admission_data_v1(&candidate, &candidate_commit)
        .map_err(|why| format!("Step 3 Pre-Admission derivation refused: {why}"))?;
    let prepared_pre_admission = pre_admission.value();
    require_admitted_root_v1(
        admitted_root,
        "before Pre-Admission receipt-last append/reopen",
    )?;
    let pre_admission_commit = pre_admission
        .append_and_reopen(root, pre_admission_bounds)
        .map_err(|why| format!("Step 3 Pre-Admission receipt-last commit refused: {why}"))?;
    require_admitted_root_v1(
        admitted_root,
        "after Pre-Admission receipt-last append/reopen",
    )?;
    let pre_admission_audit = pre_admission_commit.audit();
    let pre_admission_value = pre_admission_audit.value();
    if !same_pre_admission_semantics_v1(&pre_admission_value, &prepared_pre_admission) {
        return Err(
            "Step 3 reopened Pre-Admission audit differs from the exact in-process preparation"
                .to_owned(),
        );
    }

    let candidate_facts = ReopenedJoinFactsV1 {
        universe_id: candidate_audit.universe_id(),
        completion_digest: candidate_audit.content_digest(),
        row_count: candidate_audit.row_count(),
    };
    let pre_admission_facts = ReopenedJoinFactsV1 {
        universe_id: pre_admission_value.candidate_universe_id(),
        completion_digest: pre_admission_value.candidate_completion_digest(),
        row_count: pre_admission_value.candidate_row_count(),
    };
    require_exact_reopened_join(candidate_facts, pre_admission_facts)?;

    Ok(CommittedCandidatePreAdmissionV1 {
        candidate_audit,
        candidate_bounds,
        base_evidence_audit,
        base_evidence_bounds,
        pre_admission_audit,
        observations,
        prepared_candidate_receipt: prepared_receipt,
        prepared_pre_admission,
    })
}

fn require_admitted_root_v1(
    admitted_root: Option<&AdmittedRootV1>,
    stage: &str,
) -> Result<(), Step3OrchestratorRefusal> {
    admitted_root.map_or(Ok(()), |root| root.require_same(stage))
}

fn same_pre_admission_semantics_v1(left: &PreAdmissionDataV1, right: &PreAdmissionDataV1) -> bool {
    left.authority_id() == right.authority_id()
        && left.candidate_universe_id() == right.candidate_universe_id()
        && left.candidate_completion_digest() == right.candidate_completion_digest()
        && left.candidate_row_count() == right.candidate_row_count()
        && left.family() == right.family()
        && left.rung_seconds() == right.rung_seconds()
        && left.horizon_bars() == right.horizon_bars()
        && left.requested_span() == right.requested_span()
        && left.feed_digest() == right.feed_digest()
        && left.source_commit_digest() == right.source_commit_digest()
        && left.calendar_policy_digest() == right.calendar_policy_digest()
        && left.daily_reference_policy_digest() == right.daily_reference_policy_digest()
        && left.signal() == right.signal()
        && left.minute_context() == right.minute_context()
        && left.execution() == right.execution()
        && left.daily() == right.daily()
        && left.eligibility() == right.eligibility()
        && left.signal_calendar() == right.signal_calendar()
        && left.execution_calendar() == right.execution_calendar()
        && left.signal_load_ceiling() == right.signal_load_ceiling()
        && left.minute_load_ceiling() == right.minute_load_ceiling()
        && left.daily_load_ceiling() == right.daily_load_ceiling()
        && left.execution_start_index() == right.execution_start_index()
}

fn require_exact_reopened_join(
    candidate: ReopenedJoinFactsV1,
    pre_admission: ReopenedJoinFactsV1,
) -> Result<(), Step3OrchestratorRefusal> {
    if candidate.universe_id != pre_admission.universe_id {
        return Err(
            "Step 3 reopened Pre-Admission authority names a foreign Candidate universe".to_owned(),
        );
    }
    if candidate.completion_digest != pre_admission.completion_digest {
        return Err(
            "Step 3 reopened Pre-Admission authority names a foreign Candidate completion"
                .to_owned(),
        );
    }
    if candidate.row_count != pre_admission.row_count {
        return Err(format!(
            "Step 3 reopened Candidate row count {} differs from Pre-Admission row count {}",
            candidate.row_count, pre_admission.row_count
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(crate) use tests::{
    StatisticsV3EvaluatedPairFixture, statistics_v3_evaluated_pair_fixture,
    statistics_v3_evaluated_pair_fixture_with_price_shift,
    with_population_v6_evaluated_pair_fixture,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::population_admission_v3::AdmissionV3Status;
    use indicators::anchored::{AnchoredEvaluator, overlay_exact_minute_gapfib};
    use indicators::column::AnchoredColumn;
    use indicators::evaluator::Calendar;
    use pull::calendar::{DayKind, kind_of};
    use runner::admission::AdmissionPolicyDraftV1;
    use runner::excursion::Side;
    use runner::exit_grid_policy::{
        ExecutionResolutionV1, ExitGridSelectorV1, ForcedStopV1, RangeResolutionV1, RatioLimitsV1,
        RationalPercentileV1, RungPlanV1, printed_ohlcv_cost_model_id_v1,
    };
    use store::file::BarFile;
    use store::format::Bar;
    use store::path::{FileKind, StorePath, Timeframe, YearMonth};

    const FIXTURE_FROM: (u16, u8) = (2025, 9);
    const FIXTURE_TO: (u16, u8) = (2025, 9);
    const FIXTURE_WARM_MONTH: (u16, u8) = (2025, 8);
    const FIXTURE_OOS_MONTH: (u16, u8) = (2025, 10);
    const DAY_MICROS: i64 = 86_400_000_000;
    const MINUTE_MICROS: i64 = 60_000_000;
    const FIXTURE_COMMIT: &str = "step3-stored-success-fixture-commit";

    struct StoredSuccessFixture {
        base: PathBuf,
        source: PathBuf,
        observation: PathBuf,
        statistics: PathBuf,
        search_lineage: PathBuf,
        admission: PathBuf,
        finalization: PathBuf,
        population_v5: PathBuf,
        population_v6: PathBuf,
        execution_v3: PathBuf,
    }

    impl StoredSuccessFixture {
        fn new() -> Result<Self, Step3OrchestratorRefusal> {
            Self::with_price_shift(0)
        }

        fn with_price_shift(price_shift: i64) -> Result<Self, Step3OrchestratorRefusal> {
            let base = scratch_root("stored-observation-statistics-success");
            let source = base.join("source");
            let observation = base.join("observation");
            let statistics = base.join("statistics");
            let search_lineage = base.join("search-lineage-v4");
            let admission = base.join("population-admission-v3");
            let finalization = base.join("population-finalization-v3");
            let population_v5 = base.join("population-v5");
            let population_v6 = base.join("population-v6");
            let execution_v3 = base.join("execution-v3");
            for directory in [
                &source,
                &observation,
                &statistics,
                &search_lineage,
                &admission,
                &finalization,
                &population_v5,
                &population_v6,
                &execution_v3,
            ] {
                fs::create_dir_all(directory).map_err(|why| {
                    format!(
                        "stored Step-3 success fixture could not create {}: {why}",
                        directory.display()
                    )
                })?;
            }
            for (symbol, base_price) in [("NIFTY", 2_000_000_i64), ("BANKNIFTY", 4_000_000_i64)] {
                let shifted_price = base_price.checked_add(price_shift).ok_or_else(|| {
                    "stored Step-3 success fixture price shift overflowed".to_owned()
                })?;
                seed_stored_family_month(
                    &source,
                    Vendor::Zerodha,
                    symbol,
                    FIXTURE_WARM_MONTH,
                    shifted_price,
                )?;
                seed_stored_family_month(
                    &source,
                    Vendor::Zerodha,
                    symbol,
                    FIXTURE_FROM,
                    shifted_price,
                )?;
            }
            Ok(Self {
                base,
                source,
                observation,
                statistics,
                search_lineage,
                admission,
                finalization,
                population_v5,
                population_v6,
                execution_v3,
            })
        }
    }

    impl Drop for StoredSuccessFixture {
        fn drop(&mut self) {
            let _ignored = fs::remove_dir_all(&self.base);
        }
    }

    pub(crate) struct StatisticsV3EvaluatedPairFixture {
        _stored: StoredSuccessFixture,
        nifty: CommittedStoredCandidatePreAdmissionV1,
        banknifty: CommittedStoredCandidatePreAdmissionV1,
    }

    impl StatisticsV3EvaluatedPairFixture {
        pub(crate) const fn nifty_observations(&self) -> &CandidateFamilyObservationsV1 {
            self.nifty.candidate_pre_admission().observations()
        }

        pub(crate) const fn banknifty_observations(&self) -> &CandidateFamilyObservationsV1 {
            self.banknifty.candidate_pre_admission().observations()
        }

        pub(crate) const fn nifty_pre_admission(&self) -> PreAdmissionDataReopenAuditV1 {
            self.nifty.candidate_pre_admission().pre_admission_audit()
        }

        pub(crate) const fn banknifty_pre_admission(&self) -> PreAdmissionDataReopenAuditV1 {
            self.banknifty
                .candidate_pre_admission()
                .pre_admission_audit()
        }
    }

    pub(crate) fn statistics_v3_evaluated_pair_fixture()
    -> Result<StatisticsV3EvaluatedPairFixture, Step3OrchestratorRefusal> {
        statistics_v3_evaluated_pair_fixture_with_price_shift(0)
    }

    pub(crate) fn statistics_v3_evaluated_pair_fixture_with_price_shift(
        price_shift: i64,
    ) -> Result<StatisticsV3EvaluatedPairFixture, Step3OrchestratorRefusal> {
        let stored = StoredSuccessFixture::with_price_shift(price_shift)?;
        let long = exit_policy(Side::Long)?;
        let short = exit_policy(Side::Short)?;
        let nifty = committed_fixture_family(&stored.source, "NIFTY", &long, &short)?;
        let banknifty = committed_fixture_family(&stored.source, "BANKNIFTY", &long, &short)?;
        Ok(StatisticsV3EvaluatedPairFixture {
            _stored: stored,
            nifty,
            banknifty,
        })
    }

    pub(crate) fn with_population_v6_evaluated_pair_fixture<T>(
        action: impl FnOnce(
            crate::population_finalization_v4::CommittedStoredPopulationFinalizationV4,
            CommittedStoredCandidatePreAdmissionV1,
            CommittedStoredCandidatePreAdmissionV1,
            &Path,
        ) -> Result<T, String>,
    ) -> Result<T, String> {
        let fixture = statistics_v3_evaluated_pair_fixture()?;
        let procedure = PopulationStatisticsProcedureV2::new(16, 79, 2)?;
        let produced = crate::population_statistics_v3::produce_evaluated_population_statistics_v3(
            fixture.nifty_observations(),
            fixture.nifty_pre_admission(),
            fixture.banknifty_observations(),
            fixture.banknifty_pre_admission(),
            procedure,
        )?;
        let statistics = produced.append_and_reopen(
            &fixture._stored.statistics,
            crate::population_statistics_v3::PopulationStatisticsV3Bounds::new(
                4,
                1_000_000,
                512 * 1_024 * 1_024,
            )?,
        )?;
        let statistics = produced.authenticated_admission_source(statistics)?;
        let search = StoredSearchPairV4 {
            nifty: fixture.nifty.search.clone(),
            banknifty: fixture.banknifty.search.clone(),
        };
        let nifty_search = search.nifty().projection()?;
        let banknifty_search = search.banknifty().projection()?;
        let search_lineage = persist_anchored_search_lineage_v4(
            &fixture._stored.search_lineage,
            AnchoredSearchLineageV4Bounds::new(
                4,
                8 * crate::anchored_search_lineage_v4::ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES
                    as u64,
                4 * crate::anchored_search_lineage_v4::ANCHORED_SEARCH_LINEAGE_V4_COMPLETION_BYTES
                    as u64,
            )?,
            &nifty_search,
            &banknifty_search,
        )?
        .authority();
        let (base, mut base_reader) =
            reopen_paired_base_evidence_v2(&fixture.nifty, &fixture.banknifty)?;
        if base != *base_reader.authority() {
            return Err("Population V6 fixture Base reader changed authority".to_owned());
        }
        let nifty_audit = fixture.nifty.candidate_pre_admission().candidate_audit();
        let banknifty_audit = fixture
            .banknifty
            .candidate_pre_admission()
            .candidate_audit();
        let admission = crate::population_admission_v4::prepare_population_admission_v4(
            &statistics,
            &nifty_audit,
            &banknifty_audit,
            &mut base_reader,
            &search,
            &search_lineage,
            &population_admission_policy()?,
        )?;
        let admission = crate::population_admission_v4::commit_population_admission_v4(
            &fixture._stored.admission,
            crate::population_admission_v4::PopulationAdmissionV4Bounds::new(
                4,
                1_000_000,
                512 * 1_024 * 1_024,
            )?,
            admission,
        )?
        .into_authority();
        let finalization = crate::population_finalization_v4::commit_population_finalization_v4(
            &fixture._stored.finalization,
            crate::population_finalization_v4::PopulationFinalizationV4Bounds::new(
                4,
                1_000_000,
                512 * 1_024 * 1_024,
            )?,
            admission,
        )?
        .into_authority();
        action(
            finalization,
            fixture.nifty,
            fixture.banknifty,
            &fixture._stored.population_v6,
        )
    }

    fn fixture_month_bars(
        year: u16,
        month: u8,
        base_price: i64,
    ) -> Result<(Vec<Bar>, Vec<Bar>), Step3OrchestratorRefusal> {
        let first =
            Day::new(year, month, 1).map_err(|why| format!("fixture first day refused: {why}"))?;
        let last = first.end_of_month();
        let mut minute_bars = Vec::new();
        let mut daily_bars = Vec::new();
        for day in i64::from(first.days_from_epoch())..=i64::from(last.days_from_epoch()) {
            match kind_of(day) {
                DayKind::Open(session) => {
                    let first_index = minute_bars.len();
                    for window in session.windows.iter().take(usize::from(session.count)) {
                        for minute in window.from..=window.to {
                            let ordinal =
                                day.saturating_mul(1_447).saturating_add(i64::from(minute));
                            let open = base_price
                                .saturating_add(ordinal.saturating_mul(37).rem_euclid(20_003));
                            let close = open
                                .saturating_add(ordinal.saturating_mul(43).rem_euclid(181) - 90);
                            let high = open.max(close).saturating_add(50 + ordinal.rem_euclid(71));
                            let low = open
                                .min(close)
                                .saturating_sub(50 + ordinal.saturating_mul(13).rem_euclid(67));
                            minute_bars.push(Bar {
                                ts_micros: day
                                    .saturating_mul(DAY_MICROS)
                                    .saturating_add(i64::from(minute).saturating_mul(MINUTE_MICROS))
                                    .saturating_sub(indicators::IST_OFFSET_MICROS),
                                open,
                                high,
                                low,
                                close,
                                volume: 1_000_i64
                                    .saturating_add(ordinal.saturating_mul(29).rem_euclid(9_973)),
                                open_interest: i64::MIN,
                            });
                        }
                    }
                    let session_bars = minute_bars.get(first_index..).ok_or_else(|| {
                        "stored fixture session range disappeared after construction".to_owned()
                    })?;
                    let first_bar = session_bars.first().copied().ok_or_else(|| {
                        format!("stored fixture open day {day} has no canonical minute")
                    })?;
                    let last_bar = session_bars.last().copied().ok_or_else(|| {
                        format!("stored fixture open day {day} has no closing minute")
                    })?;
                    let high = session_bars
                        .iter()
                        .map(|bar| bar.high)
                        .max()
                        .ok_or_else(|| "stored fixture daily high is absent".to_owned())?;
                    let low = session_bars
                        .iter()
                        .map(|bar| bar.low)
                        .min()
                        .ok_or_else(|| "stored fixture daily low is absent".to_owned())?;
                    let volume = session_bars.iter().try_fold(0_i64, |sum, bar| {
                        sum.checked_add(bar.volume)
                            .ok_or_else(|| "stored fixture daily volume overflowed".to_owned())
                    })?;
                    daily_bars.push(Bar {
                        ts_micros: first_bar.ts_micros,
                        open: first_bar.open,
                        high,
                        low,
                        close: last_bar.close,
                        volume,
                        open_interest: i64::MIN,
                    });
                }
                DayKind::Closed => {}
                DayKind::OpenLengthUnmeasured | DayKind::Unmeasured => {
                    return Err(format!(
                        "stored fixture month {year}-{month:02} includes unmeasured IST day {day}"
                    ));
                }
            }
        }
        Ok((minute_bars, daily_bars))
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "the test writes the exact low-32-bit symbol id used by the production store writer and reader"
    )]
    fn seed_stored_family_month(
        root: &Path,
        vendor: Vendor,
        symbol: &str,
        month: (u16, u8),
        base_price: i64,
    ) -> Result<(), Step3OrchestratorRefusal> {
        let key = InstrumentKey::index(Exchange::Nse, symbol)
            .map_err(|why| format!("fixture instrument refused: {why}"))?;
        let year_month = YearMonth::new(month.0, month.1)
            .map_err(|why| format!("fixture month refused: {why}"))?;
        let symbol_id = brutex_core::universe::fnv1a(symbol) as u32;
        let (minute_bars, daily_bars) = fixture_month_bars(month.0, month.1, base_price)?;
        for (timeframe, bars) in [
            (Timeframe::MINUTE_1, minute_bars.as_slice()),
            (Timeframe::DAY_1, daily_bars.as_slice()),
        ] {
            let path = StorePath::for_key(vendor, &key, timeframe, year_month, FileKind::Bars)
                .map_err(|why| format!("fixture store path refused: {why}"))?;
            let mut file = BarFile::open_or_create(root, path, symbol_id)
                .map_err(|why| format!("fixture store file refused: {why}"))?;
            file.append(bars)
                .map_err(|why| format!("fixture store append refused: {why}"))?;
        }
        Ok(())
    }

    fn exit_policy(side: Side) -> Result<ExitGridPolicyV1, Step3OrchestratorRefusal> {
        let half = RationalPercentileV1::new(1, 2)
            .map_err(|why| format!("fixture percentile refused: {why}"))?;
        ExitGridPolicyV1::new(
            ExecutionResolutionV1::OneMinuteOhlcv,
            RangeResolutionV1::PpmCeiling,
            side,
            RungPlanV1::new(vec![half], vec![half], vec![half], 1)
                .map_err(|why| format!("fixture rung plan refused: {why}"))?,
            RatioLimitsV1::new(1, 10_000, 1)
                .map_err(|why| format!("fixture ratio limits refused: {why}"))?,
            1_000,
            ExitGridSelectorV1::GuaranteedFloor,
            printed_ohlcv_cost_model_id_v1(),
            ForcedStopV1::Disabled,
            u64::MAX,
            u64::MAX,
        )
        .map_err(|why| format!("fixture exit policy refused: {why}"))
    }

    fn population_admission_policy() -> Result<AdmissionPolicyV1, Step3OrchestratorRefusal> {
        AdmissionPolicyV1::new(AdmissionPolicyDraftV1 {
            min_support_hits: Some(100),
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
            min_decided_folds: Some(5),
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
            min_bootstrap_draws: Some(200),
            min_bootstrap_strategies: Some(4),
            min_bootstrap_periods: Some(40),
            min_pbo_contributing_folds: Some(5),
            max_pbo_unrankable_folds: Some(3),
            min_profitable_oos_folds: Some(3),
            min_oos_pessimistic_return_paisa: Some(1),
            max_white_reality_p_value_ppm: Some(50_000),
            require_white_reality_rejection: Some(true),
            max_romano_wolf_p_value_ppm: Some(50_000),
            require_romano_wolf_rejection: Some(true),
        })
        .map_err(|why| format!("fixture Population Admission V3 policy refused: {why:?}"))
    }

    fn fixture_bounds() -> Result<StoredCandidatePreAdmissionBoundsV1, Step3OrchestratorRefusal> {
        Ok(StoredCandidatePreAdmissionBoundsV1 {
            signal_records: StoredSpanLoadBoundV1::new(20_000)?,
            minute_records: StoredSpanLoadBoundV1::new(40_000)?,
            daily_records: StoredSpanLoadBoundV1::new(128)?,
            candidate: CandidateUniverseBoundsV1::new(1_000_000, 8)?,
            pre_admission: PreAdmissionDataBoundsV1::new(8, 2 * 1_024 * 1_024)?,
        })
    }

    fn fixture_request<'a>(
        root: &'a Path,
        underlying: &'a str,
        sweeper: &'a Sweeper,
        long: &'a ExitGridPolicyV1,
        short: &'a ExitGridPolicyV1,
    ) -> Result<StoredCandidatePreAdmissionRequestV1<'a>, Step3OrchestratorRefusal> {
        Ok(StoredCandidatePreAdmissionRequestV1 {
            root,
            vendor: Vendor::Zerodha,
            underlying,
            rung_name: "1min",
            from: FIXTURE_FROM,
            to: FIXTURE_TO,
            sweeper,
            horizon: Horizon::DEFAULT,
            widths: Widths::pinned().map_err(|why| format!("fixture widths refused: {why:?}"))?,
            availability: Availability::Absent,
            thresholds: Thresholds::CLASSICAL,
            long_exit_policy: long,
            short_exit_policy: short,
            bounds: fixture_bounds()?,
        })
    }

    fn maximum_fixture_singleton_support(
        request: &StoredCandidatePreAdmissionRequestV1<'_>,
    ) -> Result<u64, Step3OrchestratorRefusal> {
        let root = AdmittedRootV1::admit(request.root)?;
        let context = load_bounded_stored_context_v1(request, &root)?;
        let mut evaluator = AnchoredEvaluator::new(
            request.widths,
            request.availability,
            request.thresholds,
            &context.daily.references,
        )
        .map_err(|why| format!("fixture anchored evaluator refused: {why:?}"))?;
        let anchored = AnchoredColumn::build_required(&context.signal.bars, &mut evaluator)
            .map_err(|why| format!("fixture anchored column refused: {why:?}"))?;
        let mut column = anchored.into_column();
        overlay_exact_minute_gapfib(
            &context.signal.bars,
            &context.minute.bars,
            i64::from(context.rung_seconds).saturating_mul(1_000_000),
            request.widths,
            Calendar::charter(),
            &mut column,
        )
        .map_err(|why| format!("fixture exact-minute overlay refused: {why:?}"))?;
        let swept = u64::try_from(column.len())
            .map_err(|_| "fixture signal column length does not fit u64".to_owned())?;
        let mut maximum = 0_u64;
        let mut ties = 0_usize;
        for bit in 0..vocab::ConditionMask::BITS {
            if !vocab::table::LIVE.get(bit) {
                continue;
            }
            let support = column.bits().iter().fold(0_u64, |count, mask| {
                count.saturating_add(u64::from(mask.get(bit)))
            });
            if support == swept {
                continue;
            }
            if support > maximum {
                maximum = support;
                ties = 1;
            } else if support == maximum && support != 0 {
                ties = ties.saturating_add(1);
            }
        }
        if maximum == 0 {
            return Err(
                "stored success fixture derived no non-constant supported live condition"
                    .to_owned(),
            );
        }
        if ties > 16 {
            return Err(format!(
                "stored success fixture has {ties} maximum-support singleton ties; the bounded proof permits at most 16"
            ));
        }
        Ok(maximum)
    }

    fn committed_stored_family(
        root: &Path,
        underlying: &str,
        sweeper: &Sweeper,
        long: &ExitGridPolicyV1,
        short: &ExitGridPolicyV1,
    ) -> Result<CommittedStoredCandidatePreAdmissionV1, Step3OrchestratorRefusal> {
        let request = fixture_request(root, underlying, sweeper, long, short)?;
        commit_stored_with_verified_build_v1(
            request,
            VerifiedBuildCommitV1(FIXTURE_COMMIT),
            &|_, _, _| {},
        )
    }

    fn committed_fixture_family(
        root: &Path,
        underlying: &str,
        long: &ExitGridPolicyV1,
        short: &ExitGridPolicyV1,
    ) -> Result<CommittedStoredCandidatePreAdmissionV1, Step3OrchestratorRefusal> {
        let diagnostic_sweeper = Sweeper::new(engine::Ladder::with_min_hits(1));
        let min_hits = maximum_fixture_singleton_support(&fixture_request(
            root,
            underlying,
            &diagnostic_sweeper,
            long,
            short,
        )?)?;
        let sweeper = Sweeper::new(engine::Ladder::with_min_hits(min_hits));
        committed_stored_family(root, underlying, &sweeper, long, short)
    }

    fn fixture_oos_request() -> Result<StoredPostTrainingOosRequestV1, Step3OrchestratorRefusal> {
        StoredPostTrainingOosRequestV1::new(
            FIXTURE_OOS_MONTH,
            FIXTURE_OOS_MONTH,
            StoredSpanLoadBoundV1::new(20_000)?,
            StoredSpanLoadBoundV1::new(40_000)?,
            StoredSpanLoadBoundV1::new(128)?,
        )
    }

    fn first_selected_disposition(
        committed: &CommittedStoredCandidatePreAdmissionV1,
    ) -> Result<runner::exit_grid_policy::ExecutionDispositionV1, Step3OrchestratorRefusal> {
        let (_, _, dispositions) = committed.execution_v3_replay_authority()?.into_parts();
        dispositions
            .into_iter()
            .map(|projection| projection.into_parts().1)
            .find(|disposition| disposition.selected().is_some())
            .ok_or_else(|| {
                "stored OOS fixture produced no authorized selected-exit disposition".to_owned()
            })
    }

    fn directory_bytes(root: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>, Step3OrchestratorRefusal> {
        fn visit(
            base: &Path,
            at: &Path,
            files: &mut Vec<(PathBuf, Vec<u8>)>,
        ) -> Result<(), Step3OrchestratorRefusal> {
            let entries = fs::read_dir(at).map_err(|why| {
                format!("fixture snapshot could not read {}: {why}", at.display())
            })?;
            for entry in entries {
                let entry = entry.map_err(|why| format!("fixture snapshot entry failed: {why}"))?;
                let path = entry.path();
                let file_type = entry
                    .file_type()
                    .map_err(|why| format!("fixture snapshot type failed: {why}"))?;
                if file_type.is_dir() {
                    visit(base, &path, files)?;
                } else if file_type.is_file() {
                    let relative = path
                        .strip_prefix(base)
                        .map_err(|why| format!("fixture snapshot prefix failed: {why}"))?
                        .to_path_buf();
                    let bytes = fs::read(&path).map_err(|why| {
                        format!("fixture snapshot could not read {}: {why}", path.display())
                    })?;
                    files.push((relative, bytes));
                }
            }
            Ok(())
        }

        let mut files = Vec::new();
        visit(root, root, &mut files)?;
        files.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        Ok(files)
    }

    fn candle(day: i64, minute: i64) -> Candle {
        let ts_micros = day
            .saturating_mul(86_400_000_000)
            .saturating_sub(indicators::IST_OFFSET_MICROS)
            .saturating_add(minute.saturating_mul(60_000_000));
        Candle {
            ts_micros,
            open: 2_500_000,
            high: 2_500_100,
            low: 2_499_900,
            close: 2_500_050,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    fn digest(tag: u8) -> [u8; 32] {
        [tag; 32]
    }

    fn facts() -> ReopenedJoinFactsV1 {
        ReopenedJoinFactsV1 {
            universe_id: digest(1),
            completion_digest: digest(2),
            row_count: 37,
        }
    }

    fn observation_statistics_facts() -> ObservationStatisticsJoinFactsV2 {
        ObservationStatisticsJoinFactsV2 {
            observation_authority_id: digest(51),
            observation_pair_identity: digest(52),
            candidate_count: 29,
            period_count: 24,
            split_count: 10,
        }
    }

    fn scratch_root(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};

        static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);
        let unique = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "brutex-step3-{tag}-{}-{unique}",
            std::process::id()
        ))
    }

    fn canonical_instrument(
        family: InstrumentFamilyV1,
    ) -> Result<InstrumentKey, Step3OrchestratorRefusal> {
        canonical_instrument_v1(family)
    }

    fn requested_span() -> Result<RequestedSpanIdentityV1, Step3OrchestratorRefusal> {
        RequestedSpanIdentityV1::new(2025, 1, 2025, 3)
    }

    fn stored_facts() -> Result<StoredExecutionJoinFactsV1, Step3OrchestratorRefusal> {
        Ok(StoredExecutionJoinFactsV1 {
            instrument: canonical_instrument(InstrumentFamilyV1::Nifty)?,
            family: InstrumentFamilyV1::Nifty,
            rung_seconds: 300,
            horizon_bars: Horizon::DEFAULT.as_bars(),
            requested_span: requested_span()?,
            feed_digest: digest(10),
            source_commit_digest: digest(11),
            calendar_policy_digest: digest(12),
            daily_reference_policy_digest: digest(13),
            signal_calendar_digest: digest(14),
            execution_calendar_digest: digest(15),
            composite_data_digest: digest(16),
            evaluation_policy_digest: digest(17),
            signal: ExactStreamFactsV1 {
                count: 123,
                first_ts_micros: 1_000,
                last_ts_micros: 9_000,
                digest: digest(18),
            },
            minute_context: ExactStreamFactsV1 {
                count: 700,
                first_ts_micros: 500,
                last_ts_micros: 9_500,
                digest: digest(19),
            },
            execution: ExactStreamFactsV1 {
                count: 615,
                first_ts_micros: 1_000,
                last_ts_micros: 9_000,
                digest: digest(20),
            },
            daily: ExactStreamFactsV1 {
                count: 5,
                first_ts_micros: -86_400_000_000,
                last_ts_micros: 8_640_000_000,
                digest: digest(21),
            },
            eligibility: StoredEligibilityFactsV1 {
                count: 5,
                eligible_count: 4,
                excluded_day_count: 2,
                eligibility_digest: digest(22),
                excluded_days_digest: digest(23),
            },
            execution_start_index: 85,
            load_ceilings: StoredLoadCeilingsV1 {
                signal: 1_000,
                minute: 5_000,
                daily: 100,
            },
            exit_grids: LongShortExitGridIdentitiesV2 {
                long: SideExitGridIdentityV2 {
                    policy_digest: digest(24),
                    resolved_digest: digest(25),
                },
                short: SideExitGridIdentityV2 {
                    policy_digest: digest(26),
                    resolved_digest: digest(27),
                },
            },
        })
    }

    fn assert_stored_join_refuses(
        runtime: &StoredExecutionJoinFactsV1,
        reopened: &StoredExecutionJoinFactsV1,
        expected_name: &str,
    ) {
        assert!(matches!(
            require_exact_stored_execution_join(runtime, reopened),
            Err(why) if why.contains(expected_name)
        ));
    }

    #[test]
    fn exact_reopened_join_is_the_only_success_shape() {
        let exact = facts();
        assert_eq!(require_exact_reopened_join(exact, exact), Ok(()));
    }

    #[test]
    fn observation_statistics_family_order_refuses_swapped_and_same_family() {
        assert_eq!(
            require_canonical_observation_family_order_v2(
                InstrumentFamilyV1::Nifty,
                InstrumentFamilyV1::BankNifty,
            ),
            Ok(())
        );
        for (first, second) in [
            (InstrumentFamilyV1::BankNifty, InstrumentFamilyV1::Nifty),
            (InstrumentFamilyV1::Nifty, InstrumentFamilyV1::Nifty),
            (InstrumentFamilyV1::BankNifty, InstrumentFamilyV1::BankNifty),
        ] {
            assert!(matches!(
                require_canonical_observation_family_order_v2(first, second),
                Err(why) if why.contains("NIFTY then BANKNIFTY")
            ));
        }
    }

    #[test]
    fn every_observation_statistics_crosswire_term_refuses() {
        let observation = observation_statistics_facts();
        assert_eq!(
            require_exact_observation_statistics_facts_v2(observation, observation),
            Ok(())
        );

        let mut foreign = observation;
        foreign.observation_authority_id = digest(53);
        assert_observation_statistics_crosswire_refuses(observation, foreign);

        foreign = observation;
        foreign.observation_pair_identity = digest(54);
        assert_observation_statistics_crosswire_refuses(observation, foreign);

        foreign = observation;
        foreign.candidate_count = foreign.candidate_count.saturating_add(1);
        assert_observation_statistics_crosswire_refuses(observation, foreign);

        foreign = observation;
        foreign.period_count = foreign.period_count.saturating_add(1);
        assert_observation_statistics_crosswire_refuses(observation, foreign);

        foreign = observation;
        foreign.split_count = foreign.split_count.saturating_add(1);
        assert_observation_statistics_crosswire_refuses(observation, foreign);
    }

    fn assert_observation_statistics_crosswire_refuses(
        observation: ObservationStatisticsJoinFactsV2,
        statistics: ObservationStatisticsJoinFactsV2,
    ) {
        assert!(matches!(
            require_exact_observation_statistics_facts_v2(observation, statistics),
            Err(why) if why.contains("crosswired")
        ));
    }

    #[test]
    fn paired_source_roots_require_the_same_held_directory_identity()
    -> Result<(), Step3OrchestratorRefusal> {
        let base = scratch_root("paired-source-root");
        let exact_path = base.join("exact");
        let foreign_path = base.join("foreign");
        fs::create_dir_all(&exact_path)
            .map_err(|why| format!("paired-root fixture could not create exact root: {why}"))?;
        fs::create_dir_all(&foreign_path)
            .map_err(|why| format!("paired-root fixture could not create foreign root: {why}"))?;
        let nifty = AdmittedRootV1::admit(&exact_path)?;
        let banknifty = AdmittedRootV1::admit(&exact_path)?;
        let foreign = AdmittedRootV1::admit(&foreign_path)?;

        assert_eq!(
            require_exact_admitted_source_root_pair_v2(&nifty, &banknifty, "exact test"),
            Ok(())
        );
        assert!(matches!(
            require_exact_admitted_source_root_pair_v2(&nifty, &foreign, "cross-root test"),
            Err(why) if why.contains("cross-root") && why.contains("cross-root test")
        ));
        drop((nifty, banknifty, foreign));
        fs::remove_dir_all(&base)
            .map_err(|why| format!("paired-root fixture cleanup failed: {why}"))?;
        Ok(())
    }

    #[test]
    fn observation_statistics_roots_must_both_preexist_as_directories()
    -> Result<(), Step3OrchestratorRefusal> {
        let base = scratch_root("observation-statistics-roots");
        let valid = base.join("valid");
        let missing = base.join("missing");
        let file = base.join("file");
        fs::create_dir_all(&valid)
            .map_err(|why| format!("root fixture could not create valid directory: {why}"))?;
        File::create(&file).map_err(|why| format!("root fixture could not create file: {why}"))?;

        assert!(matches!(
            AdmittedObservationStatisticsRootsV2::admit(&missing, &valid),
            Err(why) if why.contains("Observation authority root")
        ));
        assert!(matches!(
            AdmittedObservationStatisticsRootsV2::admit(&valid, &missing),
            Err(why) if why.contains("Statistics V2 root")
        ));
        assert!(matches!(
            AdmittedObservationStatisticsRootsV2::admit(&file, &valid),
            Err(why) if why.contains("Observation authority root") && why.contains("not a directory")
        ));
        assert!(matches!(
            AdmittedObservationStatisticsRootsV2::admit(&valid, &file),
            Err(why) if why.contains("Statistics V2 root") && why.contains("not a directory")
        ));

        fs::remove_dir_all(&base).map_err(|why| format!("root fixture cleanup failed: {why}"))?;
        assert!(!missing.exists());
        Ok(())
    }

    #[test]
    fn every_foreign_reopened_candidate_term_refuses_by_name() {
        let candidate = facts();

        let mut foreign_universe = candidate;
        foreign_universe.universe_id = digest(3);
        assert!(matches!(
            require_exact_reopened_join(candidate, foreign_universe),
            Err(why) if why.contains("foreign Candidate universe")
        ));

        let mut foreign_completion = candidate;
        foreign_completion.completion_digest = digest(4);
        assert!(matches!(
            require_exact_reopened_join(candidate, foreign_completion),
            Err(why) if why.contains("foreign Candidate completion")
        ));

        let mut foreign_count = candidate;
        foreign_count.row_count = candidate.row_count.saturating_add(1);
        assert!(matches!(
            require_exact_reopened_join(candidate, foreign_count),
            Err(why) if why.contains("row count")
        ));
    }

    #[test]
    fn stored_public_projection_and_private_authority_keep_distinct_return_types() {
        type Progress = dyn Fn(&engine::Frontier, usize, u64);
        type PublicEntry = for<'request, 'callback> fn(
            StoredCandidatePreAdmissionRequestV1<'request>,
            &'callback Progress,
        ) -> Result<
            CandidatePreAdmissionReopenIdentitiesV1,
            Step3OrchestratorRefusal,
        >;
        type AuthorityEntry = for<'request> fn(
            StoredCandidatePreAdmissionRequestV1<'request>,
        ) -> Result<
            CommittedStoredCandidatePreAdmissionV1,
            Step3OrchestratorRefusal,
        >;

        let public: PublicEntry = commit_stored_candidate_pre_admission_v1;
        let authority: AuthorityEntry = commit_stored_candidate_pre_admission_authority_v1;
        std::hint::black_box((public, authority));
    }

    #[test]
    fn paired_observation_statistics_seam_is_crate_private_and_source_retaining() {
        type Entry = for<'observation, 'statistics> fn(
            CommittedStoredCandidatePreAdmissionV1,
            CommittedStoredCandidatePreAdmissionV1,
            &'observation Path,
            &'statistics Path,
            ObservationAuthorityBoundsV1,
            PopulationStatisticsV2Bounds,
            PopulationStatisticsProcedureV2,
        ) -> Result<
            CommittedStoredObservationStatisticsV2,
            Step3OrchestratorRefusal,
        >;
        type SourceAccessor = for<'value> fn(
            &'value CommittedStoredObservationStatisticsV2,
        )
            -> &'value CommittedStoredCandidatePreAdmissionV1;
        type ObservationAccessor = for<'value> fn(
            &'value CommittedStoredObservationStatisticsV2,
        ) -> &'value PairedCandidateObservationsV1;
        type BaseEvidenceAccessor = for<'value> fn(
            &'value CommittedStoredObservationStatisticsV2,
        )
            -> &'value PairedBaseEvidenceAuthorityV2;
        type ObservationCommitAccessor = for<'value> fn(
            &'value CommittedStoredObservationStatisticsV2,
        )
            -> &'value ObservationAuthorityCommitV1;
        type StatisticsCommitAccessor =
            for<'value> fn(
                &'value CommittedStoredObservationStatisticsV2,
            ) -> &'value PopulationStatisticsObservationCommitV2;
        type ProjectionAccessor = for<'value> fn(
            &'value CommittedStoredObservationStatisticsV2,
        )
            -> &'value PopulationStatisticsV2ProjectionSource;

        let entry: Entry = commit_stored_observation_statistics_v2;
        let nifty: SourceAccessor = CommittedStoredObservationStatisticsV2::nifty_source;
        let banknifty: SourceAccessor = CommittedStoredObservationStatisticsV2::banknifty_source;
        let observations: ObservationAccessor =
            CommittedStoredObservationStatisticsV2::observations;
        let base_evidence: BaseEvidenceAccessor =
            CommittedStoredObservationStatisticsV2::base_evidence;
        let observation: ObservationCommitAccessor =
            CommittedStoredObservationStatisticsV2::observation_commit;
        let statistics: StatisticsCommitAccessor =
            CommittedStoredObservationStatisticsV2::statistics_commit;
        let projection: ProjectionAccessor =
            CommittedStoredObservationStatisticsV2::projection_source;
        std::hint::black_box((
            entry,
            nifty,
            banknifty,
            base_evidence,
            observations,
            observation,
            statistics,
            projection,
        ));
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one success-path proof keeps the full stored Candidate/Pre-Admission/Observation/Statistics identity and retry chain visible"
    )]
    fn stored_pair_commits_observation_and_statistics_then_reuses_exact_bytes()
    -> Result<(), Step3OrchestratorRefusal> {
        let fixture = StoredSuccessFixture::new()?;
        let long = exit_policy(Side::Long)?;
        let short = exit_policy(Side::Short)?;
        let diagnostic_sweeper = Sweeper::new(engine::Ladder::with_min_hits(1));
        let nifty_min_hits = maximum_fixture_singleton_support(&fixture_request(
            &fixture.source,
            "NIFTY",
            &diagnostic_sweeper,
            &long,
            &short,
        )?)?;
        let banknifty_min_hits = maximum_fixture_singleton_support(&fixture_request(
            &fixture.source,
            "BANKNIFTY",
            &diagnostic_sweeper,
            &long,
            &short,
        )?)?;
        let nifty_sweeper = Sweeper::new(engine::Ladder::with_min_hits(nifty_min_hits));
        let banknifty_sweeper = Sweeper::new(engine::Ladder::with_min_hits(banknifty_min_hits));
        let observation_bounds = ObservationAuthorityBoundsV1::new(4, 2 * 1_024 * 1_024)?;
        let statistics_bounds =
            PopulationStatisticsV2Bounds::new(4, 1_000_000, 64, 1_000_000, 512 * 1_024 * 1_024)?;
        let procedure = PopulationStatisticsProcedureV2::new(16, 7, 2)?;

        let first_nifty =
            committed_stored_family(&fixture.source, "NIFTY", &nifty_sweeper, &long, &short)?;
        let first_banknifty = committed_stored_family(
            &fixture.source,
            "BANKNIFTY",
            &banknifty_sweeper,
            &long,
            &short,
        )?;
        let mut first = commit_stored_observation_statistics_v2(
            first_nifty,
            first_banknifty,
            &fixture.observation,
            &fixture.statistics,
            observation_bounds,
            statistics_bounds,
            procedure,
        )?;

        assert!(matches!(
            first.observation_commit(),
            ObservationAuthorityCommitV1::Written(_)
        ));
        let canonical_source = fs::canonicalize(&fixture.source)
            .map_err(|why| format!("fixture source canonicalization failed: {why}"))?;
        assert_eq!(first.nifty_source().root().path(), canonical_source);
        assert_eq!(first.banknifty_source().root().path(), canonical_source);
        require_exact_stored_source_root_pair_v2(
            first.nifty_source(),
            first.banknifty_source(),
            "success fixture retained sources",
        )?;
        let observations = first.observations();
        assert_eq!(observations.nifty().family(), InstrumentFamilyV1::Nifty);
        assert_eq!(
            observations.banknifty().family(),
            InstrumentFamilyV1::BankNifty
        );
        assert!(observations.nifty().candidate_count() > 0);
        assert!(observations.banknifty().candidate_count() > 0);
        assert_eq!(
            observations.nifty().accepted_ist_sessions(),
            observations.banknifty().accepted_ist_sessions()
        );

        let observation_audit = (*first.observation_commit()).audit();
        let statistics_audit = (*first.statistics_commit()).audit();
        let projection = *first.projection_source();
        let candidate_count = u64::try_from(observations.candidate_count())
            .map_err(|_| "fixture candidate count does not fit u64".to_owned())?;
        assert_eq!(observation_audit.pair_identity(), observations.identity());
        assert_eq!(observation_audit.candidate_count(), candidate_count);
        assert_eq!(
            projection.observation_pair_identity(),
            observations.identity()
        );
        assert_eq!(
            projection.observation_authority_id(),
            observation_audit.authority_id()
        );
        assert_eq!(projection.audit_id(), statistics_audit.audit_id());
        assert_eq!(projection.candidate_count(), candidate_count);
        assert_eq!(projection.period_count(), observation_audit.period_count());
        assert_eq!(projection.split_count(), observation_audit.split_count());
        let first_search = first.search_pair_v4()?;
        for (source, search) in [
            (first.nifty_source(), first_search.nifty()),
            (first.banknifty_source(), first_search.banknifty()),
        ] {
            let candidate = source.candidate_pre_admission().candidate_audit().receipt();
            let signal = candidate.signal_stream();
            let search_projection = search.projection()?;
            let search_source = search_projection.source_identity();
            assert_eq!(search.candidate_universe_id(), candidate.universe_id());
            assert_eq!(
                search.candidate_completion_digest(),
                candidate.content_digest()
            );
            assert_eq!(search_source.signal_digest(), signal.digest());
            assert_eq!(search_source.signal_bars(), signal.count());
            assert_eq!(
                search_source.signal_first_ts_micros(),
                signal.first_ts_micros()
            );
            assert_eq!(
                search_source.signal_last_ts_micros(),
                signal.last_ts_micros()
            );
            assert_eq!(
                search_source.signal_column_digest(),
                candidate.signal_column_digest()
            );
        }
        let joined_first = first.admission_candidate_inputs_v3(0)?;
        assert_eq!(joined_first.base().global_sequence(), 0);
        assert_eq!(joined_first.statistics().sequence(), 0);
        assert_eq!(
            joined_first.base().record().candidate_semantic_id(),
            joined_first.statistics().candidate_semantic_digest()
        );
        for (family, source) in [
            (InstrumentFamilyV1::Nifty, first.nifty_source()),
            (InstrumentFamilyV1::BankNifty, first.banknifty_source()),
        ] {
            let committed = source.candidate_pre_admission();
            let statistics_source = projection.family_source(family);
            assert_eq!(statistics_source.family(), family);
            assert_eq!(
                statistics_source.pre_admission_authority_id(),
                committed.pre_admission_audit().authority_id()
            );
            assert_eq!(
                statistics_source.candidate_universe_id(),
                committed.candidate_audit().universe_id()
            );
            assert_eq!(
                statistics_source.candidate_completion_digest(),
                committed.candidate_audit().content_digest()
            );
            assert_eq!(
                statistics_source.candidate_count(),
                committed.candidate_audit().row_count()
            );
        }
        let first_admission_statistics = first.admission_statistics_v3(0)?;
        let first_admission_draft = first_admission_statistics.draft();
        assert_eq!(
            first_admission_statistics.family(),
            InstrumentFamilyV1::Nifty
        );
        assert_eq!(first_admission_statistics.sequence(), 0);
        assert_eq!(first_admission_statistics.family_sequence(), 0);
        assert_eq!(
            first_admission_statistics.pre_admission_authority_id(),
            first
                .nifty_source()
                .candidate_pre_admission()
                .pre_admission_audit()
                .authority_id()
        );
        assert_eq!(
            first_admission_draft.statistics_audit_id,
            statistics_audit.audit_id()
        );
        assert_eq!(
            first_admission_draft.statistics_completion_digest,
            projection.completion_record_digest()
        );
        assert_eq!(
            first_admission_draft.observation_statistics_link_id,
            projection.observation_statistics_link_id()
        );
        let first_admission_family = first.admission_statistics_family_v3()?;
        assert_eq!(
            u64::try_from(first_admission_family.len())
                .map_err(|_| "Admission V3 family length does not fit u64".to_owned())?,
            candidate_count
        );
        assert_eq!(
            first_admission_family
                .first()
                .ok_or_else(|| "Admission V3 family unexpectedly empty".to_owned())?,
            &first_admission_statistics
        );
        assert!(
            first_admission_family
                .iter()
                .zip(first_admission_family.iter().skip(1))
                .all(|(left, right)| left.sequence().checked_add(1) == Some(right.sequence()))
        );
        let first_nifty_identities = first.nifty_source().identities();
        let first_banknifty_identities = first.banknifty_source().identities();
        let observation_bytes = directory_bytes(&fixture.observation)?;
        let statistics_bytes = directory_bytes(&fixture.statistics)?;
        let search_lineage_bounds = AnchoredSearchLineageV4Bounds::new(
            2,
            4 * crate::anchored_search_lineage_v4::ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES as u64,
            2 * crate::anchored_search_lineage_v4::ANCHORED_SEARCH_LINEAGE_V4_COMPLETION_BYTES
                as u64,
        )?;
        let first_lineage =
            commit_stored_search_lineage_v4(first, &fixture.search_lineage, search_lineage_bounds)?;
        let first_lineage_authority = first_lineage.lineage();
        assert_ne!(
            first_lineage_authority.structural_receipt().pair_id(),
            [0_u8; 32]
        );
        assert_ne!(first_lineage_authority.nifty().member_id(), [0_u8; 32]);
        assert_ne!(first_lineage_authority.banknifty().member_id(), [0_u8; 32]);
        assert_eq!(first_lineage.source().projection_source(), &projection);
        let search_lineage_bytes = directory_bytes(&fixture.search_lineage)?;
        let admission_decision_bytes = candidate_count
            .checked_mul(
                crate::population_admission_v3::POPULATION_ADMISSION_V3_DECISION_BYTES as u64,
            )
            .ok_or_else(|| "fixture Admission V3 decision-byte bound overflowed".to_owned())?;
        let admission_bounds = AdmissionV3Bounds::new(
            candidate_count,
            admission_decision_bytes,
            1,
            crate::population_admission_v3::POPULATION_ADMISSION_V3_COMPLETION_BYTES as u64,
            candidate_count,
        )?;
        let admission_policy = population_admission_policy()?;
        let mut first_population_admission = commit_stored_population_admission_v3(
            first_lineage,
            &fixture.admission,
            admission_bounds,
            &admission_policy,
        )?;
        let first_admission_receipt = first_population_admission.admission_receipt();
        assert_eq!(first_admission_receipt.decision_count(), candidate_count);
        let first_admission_projection = first_population_admission
            .admission_mut()
            .decision_projection(0)?;
        assert_eq!(first_admission_projection.global_sequence(), 0);
        assert_eq!(
            first_admission_projection.block_id(),
            first_admission_receipt.block_id()
        );
        assert_eq!(
            first_admission_projection.completion_id(),
            first_admission_receipt.completion_id()
        );
        let admission_bytes = directory_bytes(&fixture.admission)?;
        let finalization_row_bytes = candidate_count
            .checked_mul(
                crate::population_finalization_v3::POPULATION_FINALIZATION_V3_ROW_BYTES as u64,
            )
            .ok_or_else(|| "fixture Finalization V3 row-byte bound overflowed".to_owned())?;
        let finalization_bounds = PopulationFinalizationV3Bounds::new(
            candidate_count,
            finalization_row_bytes,
            1,
            crate::population_finalization_v3::POPULATION_FINALIZATION_V3_COMPLETION_BYTES as u64,
            candidate_count,
        )?;
        let mut first_finalization = commit_stored_population_finalization_v3(
            first_population_admission,
            &fixture.finalization,
            finalization_bounds,
        )?;
        let first_finalization_receipt = first_finalization.finalization_receipt();
        assert_eq!(
            first_finalization_receipt.admission_block_id(),
            first_admission_receipt.block_id()
        );
        assert_eq!(
            first_finalization_receipt.admission_completion_id(),
            first_admission_receipt.completion_id()
        );
        assert_eq!(first_finalization_receipt.row_count(), candidate_count);
        let first_finalization_projection =
            first_finalization.finalization_mut().row_projection(0)?;
        let first_population_v5_inputs = first_finalization.population_v5_inputs()?;
        assert_eq!(
            u64::try_from(first_population_v5_inputs.len())
                .map_err(|_| "Population V5 fixture input count does not fit u64".to_owned())?,
            candidate_count
        );
        for input in &first_population_v5_inputs {
            let candidate = input.candidate();
            let admission = input.admission().identity();
            let finalization = input.finalization();
            assert!(candidate.canonical_record().iter().any(|byte| *byte != 0));
            assert!(
                input
                    .admission()
                    .canonical_record()
                    .iter()
                    .any(|byte| *byte != 0)
            );
            assert!(
                input
                    .admission()
                    .runner_decision()
                    .iter()
                    .any(|byte| *byte != 0)
            );
            assert_eq!(
                candidate.base_candidate_row_digest(),
                admission.candidate_row_digest()
            );
            assert_eq!(
                admission.decision_id(),
                finalization.admission_decision_id()
            );
            assert_eq!(admission.status(), finalization.status());
            assert_eq!(
                input.finalization_completion_id(),
                first_finalization_receipt.completion_id()
            );
        }
        let finalization_bytes = directory_bytes(&fixture.finalization)?;
        let population_v5_row_bytes = candidate_count
            .checked_mul(crate::population_v5::POPULATION_V5_ROW_BYTES as u64)
            .ok_or_else(|| "fixture Population V5 row-byte bound overflowed".to_owned())?;
        let population_v5_bounds = crate::population_v5::PopulationV5Bounds::new(
            candidate_count,
            population_v5_row_bytes,
            1,
            crate::population_v5::POPULATION_V5_COMPLETION_BYTES as u64,
            candidate_count,
        )?;
        let mut first_population_v5 = crate::population_v5::commit_population_v5(
            &fixture.population_v5,
            population_v5_bounds,
            first_finalization,
        )?;
        assert!(first_population_v5.was_written());
        let first_population_v5_receipt = first_population_v5.structural_receipt();
        assert_eq!(first_population_v5_receipt.row_count(), candidate_count);
        assert_eq!(
            first_population_v5_receipt
                .nifty_count()
                .checked_add(first_population_v5_receipt.banknifty_count()),
            Some(candidate_count)
        );
        assert_eq!(
            first_population_v5_receipt.source_finalization_id(),
            first_finalization_receipt.finalization_id()
        );
        assert_eq!(
            first_population_v5_receipt.source_finalization_completion_id(),
            first_finalization_receipt.completion_id()
        );
        assert_eq!(
            first_population_v5_receipt.source_finalization_ordered_row_digest(),
            first_finalization_receipt.ordered_row_digest()
        );
        assert_eq!(
            first_population_v5_receipt.source_admission_block_id(),
            first_finalization_receipt.admission_block_id()
        );
        assert_eq!(
            first_population_v5_receipt.source_admission_completion_id(),
            first_finalization_receipt.admission_completion_id()
        );
        assert_eq!(
            [
                first_population_v5_receipt.admitted_count(),
                first_population_v5_receipt.rejected_count(),
                first_population_v5_receipt.unmeasured_count(),
                first_population_v5_receipt.refused_count(),
            ],
            [
                first_finalization_receipt.admitted_count(),
                first_finalization_receipt.rejected_count(),
                first_finalization_receipt.unmeasured_count(),
                first_finalization_receipt.refused_count(),
            ]
        );
        let first_population_v5_rows = first_population_v5.ordered_authenticated_rows()?;
        assert_eq!(
            first_population_v5_rows.len(),
            first_population_v5_inputs.len()
        );
        let mut population_v5_status_counts = [0_u64; 4];
        for (index, (row, input)) in first_population_v5_rows
            .iter()
            .zip(&first_population_v5_inputs)
            .enumerate()
        {
            let expected_sequence = u64::try_from(index)
                .map_err(|_| "Population V5 fixture ordinal does not fit u64".to_owned())?;
            assert_eq!(
                row.population_id(),
                first_population_v5_receipt.population_id()
            );
            assert_eq!(row.global_sequence(), expected_sequence);
            assert_eq!(row.family(), input.finalization().family());
            assert_eq!(
                row.family_sequence(),
                input.finalization().family_sequence()
            );
            assert_eq!(row.status(), input.finalization().status());
            assert_eq!(row.candidate(), input.candidate());
            assert_eq!(
                row.admission().canonical_record(),
                input.admission().canonical_record()
            );
            assert_eq!(row.finalization_row_id(), input.finalization().row_id());
            assert_eq!(
                row.finalization_completion_id(),
                input.finalization_completion_id()
            );
            let status_index = match row.status() {
                AdmissionV3Status::Admitted => 0,
                AdmissionV3Status::Rejected => 1,
                AdmissionV3Status::Unmeasured => 2,
                AdmissionV3Status::Refused => 3,
            };
            let status_count = population_v5_status_counts
                .get_mut(status_index)
                .ok_or_else(|| "Population V5 fixture status lies outside matrix".to_owned())?;
            *status_count = (*status_count)
                .checked_add(1)
                .ok_or_else(|| "Population V5 fixture status count overflowed".to_owned())?;
        }
        assert_eq!(
            population_v5_status_counts,
            [
                first_population_v5_receipt.admitted_count(),
                first_population_v5_receipt.rejected_count(),
                first_population_v5_receipt.unmeasured_count(),
                first_population_v5_receipt.refused_count(),
            ]
        );
        let authenticated_first_population_v5_row = first_population_v5.authenticated_row(0)?;
        let expected_first_population_v5_row = first_population_v5_rows
            .first()
            .ok_or_else(|| "Population V5 fixture unexpectedly produced no rows".to_owned())?;
        assert_eq!(
            &authenticated_first_population_v5_row,
            expected_first_population_v5_row
        );
        let population_v5_bytes = directory_bytes(&fixture.population_v5)?;
        let execution_parameter_records = 4_u64;
        let execution_percentile_records = 256_u64;
        let execution_parameter_bytes = execution_parameter_records
            .checked_mul(crate::execution_v3::EXECUTION_V3_PARAMETER_BYTES as u64)
            .ok_or_else(|| "fixture Execution V3 parameter-byte bound overflowed".to_owned())?;
        let execution_percentile_bytes = execution_percentile_records
            .checked_mul(crate::execution_v3::EXECUTION_V3_PERCENTILE_BYTES as u64)
            .ok_or_else(|| "fixture Execution V3 percentile-byte bound overflowed".to_owned())?;
        let execution_disposition_bytes = candidate_count
            .checked_mul(crate::execution_v3::EXECUTION_V3_DISPOSITION_BYTES as u64)
            .ok_or_else(|| "fixture Execution V3 disposition-byte bound overflowed".to_owned())?;
        let execution_bounds = crate::execution_v3::ExecutionV3Bounds::new(
            crate::execution_v3::ExecutionV3FileBound::new(
                execution_parameter_records,
                execution_parameter_bytes,
                crate::execution_v3::EXECUTION_V3_PARAMETER_BYTES,
                "fixture parameter",
            )?,
            crate::execution_v3::ExecutionV3FileBound::new(
                execution_percentile_records,
                execution_percentile_bytes,
                crate::execution_v3::EXECUTION_V3_PERCENTILE_BYTES,
                "fixture percentile",
            )?,
            crate::execution_v3::ExecutionV3FileBound::new(
                candidate_count,
                execution_disposition_bytes,
                crate::execution_v3::EXECUTION_V3_DISPOSITION_BYTES,
                "fixture disposition",
            )?,
            crate::execution_v3::ExecutionV3FileBound::new(
                1,
                crate::execution_v3::EXECUTION_V3_COMPLETION_BYTES as u64,
                crate::execution_v3::EXECUTION_V3_COMPLETION_BYTES,
                "fixture Completion",
            )?,
            candidate_count,
            execution_percentile_records,
        )?;
        let mut first_execution_v3 = crate::execution_v3::commit_stored_execution_v3(
            &fixture.execution_v3,
            execution_bounds,
            first_population_v5,
        )?;
        assert!(first_execution_v3.was_written());
        let first_execution_v3_receipt = first_execution_v3.structural_receipt();
        assert_eq!(
            first_execution_v3_receipt.population_id(),
            first_population_v5_receipt.population_id()
        );
        let first_execution_v3_rows = first_execution_v3.ordered_authenticated_dispositions()?;
        assert_eq!(
            u64::try_from(first_execution_v3_rows.len())
                .map_err(|_| "fixture Execution V3 row count does not fit u64".to_owned())?,
            candidate_count
        );
        assert!(first_execution_v3_rows.iter().all(|row| {
            row.population_id() == first_population_v5_receipt.population_id()
                && row.canonical_record().iter().any(|byte| *byte != 0)
        }));
        let execution_v3_bytes = directory_bytes(&fixture.execution_v3)?;
        drop(first_execution_v3);

        let retry_nifty =
            committed_stored_family(&fixture.source, "NIFTY", &nifty_sweeper, &long, &short)?;
        let retry_banknifty = committed_stored_family(
            &fixture.source,
            "BANKNIFTY",
            &banknifty_sweeper,
            &long,
            &short,
        )?;
        let mut retry = commit_stored_observation_statistics_v2(
            retry_nifty,
            retry_banknifty,
            &fixture.observation,
            &fixture.statistics,
            observation_bounds,
            statistics_bounds,
            procedure,
        )?;
        assert!(matches!(
            retry.observation_commit(),
            ObservationAuthorityCommitV1::Reused(_)
        ));
        assert_eq!(retry.nifty_source().identities(), first_nifty_identities);
        assert_eq!(
            retry.banknifty_source().identities(),
            first_banknifty_identities
        );
        assert_eq!((*retry.observation_commit()).audit(), observation_audit);
        assert_eq!((*retry.statistics_commit()).audit(), statistics_audit);
        assert_eq!(*retry.projection_source(), projection);
        let retry_search = retry.search_pair_v4()?;
        assert!(retry_search.nifty().projection()? == first_search.nifty().projection()?);
        assert!(retry_search.banknifty().projection()? == first_search.banknifty().projection()?);
        assert_eq!(
            retry.admission_statistics_v3(0)?,
            first_admission_statistics
        );
        assert_eq!(
            retry.admission_statistics_family_v3()?,
            first_admission_family
        );
        assert_eq!(directory_bytes(&fixture.observation)?, observation_bytes);
        assert_eq!(directory_bytes(&fixture.statistics)?, statistics_bytes);
        let retry_lineage =
            commit_stored_search_lineage_v4(retry, &fixture.search_lineage, search_lineage_bounds)?;
        assert_eq!(
            retry_lineage.lineage().structural_receipt(),
            first_lineage_authority.structural_receipt()
        );
        assert_eq!(
            directory_bytes(&fixture.search_lineage)?,
            search_lineage_bytes
        );
        let mut retry_population_admission = commit_stored_population_admission_v3(
            retry_lineage,
            &fixture.admission,
            admission_bounds,
            &admission_policy,
        )?;
        assert_eq!(
            retry_population_admission.admission_receipt(),
            first_admission_receipt
        );
        assert_eq!(
            retry_population_admission
                .admission_mut()
                .decision_projection(0)?,
            first_admission_projection
        );
        assert_eq!(directory_bytes(&fixture.admission)?, admission_bytes);
        let mut retry_finalization = commit_stored_population_finalization_v3(
            retry_population_admission,
            &fixture.finalization,
            finalization_bounds,
        )?;
        assert_eq!(
            retry_finalization.finalization_receipt(),
            first_finalization_receipt
        );
        assert_eq!(
            retry_finalization.finalization_mut().row_projection(0)?,
            first_finalization_projection
        );
        assert_eq!(
            retry_finalization.population_v5_inputs()?,
            first_population_v5_inputs
        );
        assert_eq!(directory_bytes(&fixture.finalization)?, finalization_bytes);
        let mut retry_population_v5 = crate::population_v5::commit_population_v5(
            &fixture.population_v5,
            population_v5_bounds,
            retry_finalization,
        )?;
        assert!(!retry_population_v5.was_written());
        assert_eq!(
            retry_population_v5.structural_receipt(),
            first_population_v5_receipt
        );
        assert_eq!(
            retry_population_v5.ordered_authenticated_rows()?,
            first_population_v5_rows
        );
        assert_eq!(
            directory_bytes(&fixture.population_v5)?,
            population_v5_bytes
        );
        let mut retry_execution_v3 = crate::execution_v3::commit_stored_execution_v3(
            &fixture.execution_v3,
            execution_bounds,
            retry_population_v5,
        )?;
        assert!(!retry_execution_v3.was_written());
        assert_eq!(
            retry_execution_v3.structural_receipt(),
            first_execution_v3_receipt
        );
        assert_eq!(directory_bytes(&fixture.execution_v3)?, execution_v3_bytes);
        let finalization_row_path = fixture.finalization.join("population-finalization-v3.bin");
        let mut corrupted_finalization = fs::read(&finalization_row_path).map_err(|why| {
            format!(
                "fixture could not read Finalization V3 row file for stale-source attack: {why}"
            )
        })?;
        let final_byte = corrupted_finalization
            .last_mut()
            .ok_or_else(|| "fixture Finalization V3 row file is unexpectedly empty".to_owned())?;
        *final_byte ^= 1;
        fs::write(&finalization_row_path, corrupted_finalization).map_err(|why| {
            format!("fixture could not persist Finalization V3 stale-source attack: {why}")
        })?;
        let Err(stale_source_refusal) = retry_execution_v3.ordered_authenticated_dispositions()
        else {
            return Err(
                "Execution V3 authenticated dispositions after its retained Population/Finalization source changed"
                    .to_owned(),
            );
        };
        assert!(
            stale_source_refusal.contains("Finalization")
                || stale_source_refusal.contains("changed")
                || stale_source_refusal.contains("corrupt"),
            "unexpected retained-source refusal: {stale_source_refusal}"
        );
        Ok(())
    }

    #[test]
    fn exact_stored_retry_and_reopen_facts_are_identity_equal()
    -> Result<(), Step3OrchestratorRefusal> {
        let first = stored_facts()?;
        let retry = stored_facts()?;
        assert_eq!(first, retry);
        assert_eq!(require_exact_stored_execution_join(&first, &retry), Ok(()));
        Ok(())
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one exhaustive attack mutates every retained/reopened join field independently"
    )]
    fn every_foreign_stored_execution_term_refuses_by_name() -> Result<(), Step3OrchestratorRefusal>
    {
        let runtime = stored_facts()?;

        let mut foreign = runtime;
        foreign.instrument = canonical_instrument(InstrumentFamilyV1::BankNifty)?;
        assert_stored_join_refuses(&runtime, &foreign, "instrument");

        foreign = runtime;
        foreign.family = InstrumentFamilyV1::BankNifty;
        assert_stored_join_refuses(&runtime, &foreign, "instrument family");

        foreign = runtime;
        foreign.rung_seconds = 900;
        assert_stored_join_refuses(&runtime, &foreign, "signal rung");

        foreign = runtime;
        foreign.horizon_bars = runtime.horizon_bars.saturating_add(1);
        assert_stored_join_refuses(&runtime, &foreign, "one-minute horizon");

        foreign = runtime;
        foreign.requested_span = RequestedSpanIdentityV1::new(2025, 1, 2025, 4)?;
        assert_stored_join_refuses(&runtime, &foreign, "requested month span");

        foreign = runtime;
        foreign.feed_digest = digest(31);
        assert_stored_join_refuses(&runtime, &foreign, "feed");

        foreign = runtime;
        foreign.source_commit_digest = digest(32);
        assert_stored_join_refuses(&runtime, &foreign, "source commit");

        foreign = runtime;
        foreign.calendar_policy_digest = digest(33);
        assert_stored_join_refuses(&runtime, &foreign, "calendar policy");

        foreign = runtime;
        foreign.daily_reference_policy_digest = digest(34);
        assert_stored_join_refuses(&runtime, &foreign, "one-day reference policy");

        foreign = runtime;
        foreign.signal_calendar_digest = digest(35);
        assert_stored_join_refuses(&runtime, &foreign, "signal complete calendar");

        foreign = runtime;
        foreign.execution_calendar_digest = digest(36);
        assert_stored_join_refuses(&runtime, &foreign, "one-minute complete calendar");

        foreign = runtime;
        foreign.composite_data_digest = digest(37);
        assert_stored_join_refuses(&runtime, &foreign, "composite signal/minute/daily data");

        foreign = runtime;
        foreign.evaluation_policy_digest = digest(38);
        assert_stored_join_refuses(&runtime, &foreign, "evaluator policy");

        foreign = runtime;
        foreign.signal.count = foreign.signal.count.saturating_add(1);
        assert_stored_join_refuses(&runtime, &foreign, "signal stream");

        foreign = runtime;
        foreign.signal.first_ts_micros = foreign.signal.first_ts_micros.saturating_sub(1);
        assert_stored_join_refuses(&runtime, &foreign, "signal stream");

        foreign = runtime;
        foreign.signal.last_ts_micros = foreign.signal.last_ts_micros.saturating_add(1);
        assert_stored_join_refuses(&runtime, &foreign, "signal stream");

        foreign = runtime;
        foreign.signal.digest = digest(39);
        assert_stored_join_refuses(&runtime, &foreign, "signal stream");

        foreign = runtime;
        foreign.minute_context.count = foreign.minute_context.count.saturating_add(1);
        assert_stored_join_refuses(&runtime, &foreign, "complete one-minute context");

        foreign = runtime;
        foreign.minute_context.first_ts_micros =
            foreign.minute_context.first_ts_micros.saturating_sub(1);
        assert_stored_join_refuses(&runtime, &foreign, "complete one-minute context");

        foreign = runtime;
        foreign.minute_context.last_ts_micros =
            foreign.minute_context.last_ts_micros.saturating_add(1);
        assert_stored_join_refuses(&runtime, &foreign, "complete one-minute context");

        foreign = runtime;
        foreign.minute_context.digest = digest(40);
        assert_stored_join_refuses(&runtime, &foreign, "complete one-minute context");

        foreign = runtime;
        foreign.execution.count = foreign.execution.count.saturating_add(1);
        assert_stored_join_refuses(&runtime, &foreign, "one-minute execution stream");

        foreign = runtime;
        foreign.execution.first_ts_micros = foreign.execution.first_ts_micros.saturating_sub(1);
        assert_stored_join_refuses(&runtime, &foreign, "one-minute execution stream");

        foreign = runtime;
        foreign.execution.last_ts_micros = foreign.execution.last_ts_micros.saturating_add(1);
        assert_stored_join_refuses(&runtime, &foreign, "one-minute execution stream");

        foreign = runtime;
        foreign.execution.digest = digest(41);
        assert_stored_join_refuses(&runtime, &foreign, "one-minute execution stream");

        foreign = runtime;
        foreign.daily.count = foreign.daily.count.saturating_add(1);
        assert_stored_join_refuses(&runtime, &foreign, "one-day reference stream");

        foreign = runtime;
        foreign.daily.first_ts_micros = foreign.daily.first_ts_micros.saturating_sub(1);
        assert_stored_join_refuses(&runtime, &foreign, "one-day reference stream");

        foreign = runtime;
        foreign.daily.last_ts_micros = foreign.daily.last_ts_micros.saturating_add(1);
        assert_stored_join_refuses(&runtime, &foreign, "one-day reference stream");

        foreign = runtime;
        foreign.daily.digest = digest(42);
        assert_stored_join_refuses(&runtime, &foreign, "one-day reference stream");

        foreign = runtime;
        foreign.eligibility.count = foreign.eligibility.count.saturating_add(1);
        assert_stored_join_refuses(&runtime, &foreign, "one-day eligibility evidence");

        foreign = runtime;
        foreign.eligibility.eligible_count = foreign.eligibility.eligible_count.saturating_sub(1);
        assert_stored_join_refuses(&runtime, &foreign, "one-day eligibility evidence");

        foreign = runtime;
        foreign.eligibility.excluded_day_count =
            foreign.eligibility.excluded_day_count.saturating_add(1);
        assert_stored_join_refuses(&runtime, &foreign, "one-day eligibility evidence");

        foreign = runtime;
        foreign.eligibility.eligibility_digest = digest(43);
        assert_stored_join_refuses(&runtime, &foreign, "one-day eligibility evidence");

        foreign = runtime;
        foreign.eligibility.excluded_days_digest = digest(44);
        assert_stored_join_refuses(&runtime, &foreign, "one-day eligibility evidence");

        foreign = runtime;
        foreign.execution_start_index = foreign.execution_start_index.saturating_add(1);
        assert_stored_join_refuses(&runtime, &foreign, "execution start index");

        foreign = runtime;
        foreign.load_ceilings.signal = foreign.load_ceilings.signal.saturating_add(1);
        assert_stored_join_refuses(&runtime, &foreign, "signal load ceiling");

        foreign = runtime;
        foreign.load_ceilings.minute = foreign.load_ceilings.minute.saturating_add(1);
        assert_stored_join_refuses(&runtime, &foreign, "minute-context load ceiling");

        foreign = runtime;
        foreign.load_ceilings.daily = foreign.load_ceilings.daily.saturating_add(1);
        assert_stored_join_refuses(&runtime, &foreign, "daily-reference load ceiling");

        foreign = runtime;
        foreign.exit_grids.long.policy_digest = digest(45);
        assert_stored_join_refuses(&runtime, &foreign, "long/short exit grids");

        foreign = runtime;
        foreign.exit_grids.long.resolved_digest = digest(46);
        assert_stored_join_refuses(&runtime, &foreign, "long/short exit grids");

        foreign = runtime;
        foreign.exit_grids.short.policy_digest = digest(47);
        assert_stored_join_refuses(&runtime, &foreign, "long/short exit grids");

        foreign = runtime;
        foreign.exit_grids.short.resolved_digest = digest(48);
        assert_stored_join_refuses(&runtime, &foreign, "long/short exit grids");
        Ok(())
    }

    #[test]
    fn stored_post_training_oos_cohort_is_exact_and_mints_only_opaque_witnesses()
    -> Result<(), Step3OrchestratorRefusal> {
        let fixture = StoredSuccessFixture::new()?;
        seed_stored_family_month(
            &fixture.source,
            Vendor::Zerodha,
            "NIFTY",
            FIXTURE_OOS_MONTH,
            2_000_000,
        )?;
        let long = exit_policy(Side::Long)?;
        let short = exit_policy(Side::Short)?;
        let committed = committed_fixture_family(&fixture.source, "NIFTY", &long, &short)?;
        let disposition = first_selected_disposition(&committed)?;

        let first = committed.stored_post_training_oos_cohort(fixture_oos_request()?)?;
        let retry = committed.stored_post_training_oos_cohort(fixture_oos_request()?)?;
        assert_eq!(first.audit(), retry.audit());
        assert!(first.audit().execution_bars() > 0);
        assert!(first.audit().oos_first_ts_micros() > first.audit().training_last_ts_micros());
        assert!(first.audit().oos_last_ts_micros() >= first.audit().oos_first_ts_micros());

        let first_witness = first.mint_witness(&disposition)?;
        let retry_witness = retry.mint_witness(&disposition)?;
        assert_eq!(first_witness.cohort_id(), first.audit().cohort_id());
        assert_eq!(first_witness.cohort_id(), retry_witness.cohort_id());
        assert_eq!(first_witness.witness_id(), retry_witness.witness_id());
        let (cohort_id, witness_id, witness) = first_witness.into_parts();
        assert_eq!(cohort_id, first.audit().cohort_id());
        assert_eq!(witness_id, retry_witness.witness_id());
        witness
            .require_integrity()
            .map_err(|why| format!("stored OOS fixture witness integrity refused: {why:?}"))?;
        Ok(())
    }

    #[test]
    fn stored_post_training_oos_refuses_absent_overlap_and_preallocation_overflow()
    -> Result<(), Step3OrchestratorRefusal> {
        let fixture = StoredSuccessFixture::new()?;
        let long = exit_policy(Side::Long)?;
        let short = exit_policy(Side::Short)?;
        let committed = committed_fixture_family(&fixture.source, "NIFTY", &long, &short)?;

        let Err(absent) = committed.stored_post_training_oos_cohort(fixture_oos_request()?) else {
            return Err("an absent stored OOS month was accepted".to_owned());
        };
        assert!(
            absent.contains("missing")
                || absent.contains("incomplete")
                || absent.contains("absent"),
            "unexpected absent-month refusal: {absent}"
        );

        let overlap = StoredPostTrainingOosRequestV1::new(
            FIXTURE_FROM,
            FIXTURE_TO,
            StoredSpanLoadBoundV1::new(20_000)?,
            StoredSpanLoadBoundV1::new(40_000)?,
            StoredSpanLoadBoundV1::new(128)?,
        )?;
        assert!(matches!(
            committed.stored_post_training_oos_cohort(overlap),
            Err(why) if why.contains("not after training last")
        ));

        seed_stored_family_month(
            &fixture.source,
            Vendor::Zerodha,
            "NIFTY",
            FIXTURE_OOS_MONTH,
            2_000_000,
        )?;
        let bounded = StoredPostTrainingOosRequestV1::new(
            FIXTURE_OOS_MONTH,
            FIXTURE_OOS_MONTH,
            StoredSpanLoadBoundV1::new(1)?,
            StoredSpanLoadBoundV1::new(1)?,
            StoredSpanLoadBoundV1::new(1)?,
        )?;
        assert!(matches!(
            committed.stored_post_training_oos_cohort(bounded),
            Err(why) if why.contains("ceiling")
        ));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn stored_post_training_oos_refuses_crosswired_and_replaced_sources()
    -> Result<(), Step3OrchestratorRefusal> {
        let fixture = StoredSuccessFixture::new()?;
        for (symbol, base_price) in [("NIFTY", 2_000_000), ("BANKNIFTY", 4_000_000)] {
            seed_stored_family_month(
                &fixture.source,
                Vendor::Zerodha,
                symbol,
                FIXTURE_OOS_MONTH,
                base_price,
            )?;
        }
        let long = exit_policy(Side::Long)?;
        let short = exit_policy(Side::Short)?;
        let nifty = committed_fixture_family(&fixture.source, "NIFTY", &long, &short)?;
        let banknifty = committed_fixture_family(&fixture.source, "BANKNIFTY", &long, &short)?;
        let nifty_disposition = first_selected_disposition(&nifty)?;
        let banknifty_cohort = banknifty.stored_post_training_oos_cohort(fixture_oos_request()?)?;
        assert!(matches!(
            banknifty_cohort.mint_witness(&nifty_disposition),
            Err(why) if why.contains("foreign") || why.contains("crosswired")
        ));

        let nifty_cohort = nifty.stored_post_training_oos_cohort(fixture_oos_request()?)?;
        let displaced = fixture.base.join("source-displaced");
        fs::rename(&fixture.source, &displaced)
            .map_err(|why| format!("stored OOS fixture could not displace source root: {why}"))?;
        fs::create_dir_all(&fixture.source)
            .map_err(|why| format!("stored OOS fixture could not replace source root: {why}"))?;
        assert!(matches!(
            nifty_cohort.mint_witness(&nifty_disposition),
            Err(why) if why.contains("substituted")
        ));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn admitted_root_refuses_deterministic_rename_and_replace_substitution()
    -> Result<(), Step3OrchestratorRefusal> {
        use std::sync::atomic::{AtomicU64, Ordering};

        static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

        let unique = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!(
            "brutex-step3-root-swap-{}-{unique}",
            std::process::id()
        ));
        let admitted_path = base.join("admitted");
        let displaced_path = base.join("displaced");
        let replacement_path = base.join("replacement");
        fs::create_dir_all(&admitted_path)
            .map_err(|why| format!("root-swap fixture could not create admitted root: {why}"))?;
        fs::create_dir_all(&replacement_path)
            .map_err(|why| format!("root-swap fixture could not create replacement root: {why}"))?;
        let admitted = AdmittedRootV1::admit(&admitted_path)?;
        fs::rename(&admitted_path, &displaced_path)
            .map_err(|why| format!("root-swap fixture could not displace admitted root: {why}"))?;
        fs::rename(&replacement_path, &admitted_path).map_err(|why| {
            format!("root-swap fixture could not install replacement root: {why}")
        })?;

        let refusal = admitted.require_same("deterministic root-swap attack");
        fs::remove_dir_all(&base)
            .map_err(|why| format!("root-swap fixture cleanup failed: {why}"))?;
        assert!(matches!(
            refusal,
            Err(why) if why.contains("substituted") && why.contains("root-swap attack")
        ));
        Ok(())
    }

    #[test]
    fn requested_execution_is_the_exact_day_bounded_subspan() {
        let context = [
            candle(99, 929),
            candle(100, 555),
            candle(100, 556),
            candle(101, 555),
            candle(102, 555),
        ];
        let selected = requested_execution_subspan(&context, 100, 101);
        assert!(selected.is_ok(), "the exact requested subspan exists");
        if let Ok(selected) = selected {
            assert_eq!(selected.len(), 3);
            assert_eq!(
                selected
                    .first()
                    .map(|bar| indicators::ist_day(bar.ts_micros)),
                Some(100)
            );
            assert_eq!(
                selected
                    .last()
                    .map(|bar| indicators::ist_day(bar.ts_micros)),
                Some(101)
            );
        }

        assert!(matches!(
            requested_execution_subspan(&context, 103, 104),
            Err(why) if why.contains("has no bars")
        ));
        assert!(matches!(
            requested_execution_subspan(&context, 101, 100),
            Err(why) if why.contains("bounds are inconsistent")
        ));

        let reordered = [candle(100, 555), candle(102, 555), candle(101, 555)];
        assert!(matches!(
            requested_execution_subspan(&reordered, 100, 102),
            Err(why) if why.contains("not monotonically ordered")
        ));
    }

    #[test]
    fn canonical_rung_conversion_refuses_fractional_nonpositive_and_oversized_values() {
        assert_eq!(rung_seconds(60_000_000), Ok(60));
        for malformed in [0, -1, 60_000_001] {
            assert!(matches!(
                rung_seconds(malformed),
                Err(why) if why.contains("positive whole-second")
            ));
        }
        let oversized = i64::from(u32::MAX)
            .saturating_add(1)
            .saturating_mul(1_000_000);
        assert!(matches!(
            rung_seconds(oversized),
            Err(why) if why.contains("does not fit u32 seconds")
        ));
    }
}
