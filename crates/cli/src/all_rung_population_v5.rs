//! Canonical eight-rung stored coordinator through Population V5.
//!
//! The stored Candidate producer necessarily uses one root for both immutable
//! market bytes and its append-only Candidate/Pre-Admission ledgers.  This
//! coordinator therefore commits all sixteen Candidate family authorities
//! first, in canonical rung order and NIFTY-then-BANKNIFTY family order.  Only
//! after that shared source is frozen does it build the eight independent
//! successor chains.  A later Candidate append can consequently never stale a
//! retained Observation/Statistics reader from an earlier rung.
//!
//! Every successor chain is confined to one pre-existing canonical rung
//! directory below an independently admitted authority root.  The complete
//! root topology is admitted before the first write, rejects symlinks and
//! physical aliases, and is rechecked around every typed one-rung producer.
//! No caller supplies a rung list, family list, raw row, digest, identifier or
//! detached projection.

use std::fs::{self, File};
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use indicators::evaluator::Widths;
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use runner::Sweeper;
use runner::admission::AdmissionPolicyV1;
use runner::exit_grid_policy::ExitGridPolicyV1;
use runner::outcome::Horizon;

use crate::anchored_search_lineage_v4::AnchoredSearchLineageV4Bounds;
use crate::candidate_universe::CANDIDATE_SIGNAL_RUNGS_SECONDS_V1;
use crate::execution_v3::{
    CommittedStoredExecutionV3, ExecutionV3Bounds, ExecutionV3Family, ExecutionV3StructuralReceipt,
    commit_stored_execution_v3,
};
use crate::population::InstrumentFamilyV1;
use crate::population_admission_v3::{AdmissionV3Bounds, AdmissionV3Family};
use crate::population_finalization_v3::PopulationFinalizationV3Bounds;
use crate::population_observations_v1::ObservationAuthorityBoundsV1;
use crate::population_statistics_v2::{
    PopulationStatisticsProcedureV2, PopulationStatisticsV2Bounds,
};
use crate::population_v5::{
    CommittedStoredPopulationV5, PopulationV5Bounds, PopulationV5ExecutionV3SourceV1,
    PopulationV5StructuralReceipt, commit_population_v5,
};
use crate::step3_orchestrator::{
    CommittedStoredCandidatePreAdmissionV1, StoredCandidatePreAdmissionBoundsV1,
    StoredCandidatePreAdmissionRequestV1, commit_stored_candidate_pre_admission_authority_v1,
    commit_stored_observation_statistics_v2, commit_stored_population_admission_v3,
    commit_stored_population_finalization_v3, commit_stored_search_lineage_v4,
};

const RUNG_COUNT: usize = 8;
const CANONICAL_RUNG_NAMES_V1: [&str; RUNG_COUNT] = [
    "1min", "2min", "3min", "5min", "10min", "15min", "30min", "60min",
];
const CANONICAL_UNDERLYINGS_V1: [&str; 2] = ["NIFTY", "BANKNIFTY"];

const _: () = {
    assert!(CANDIDATE_SIGNAL_RUNGS_SECONDS_V1[0] == 60);
    assert!(CANDIDATE_SIGNAL_RUNGS_SECONDS_V1[1] == 120);
    assert!(CANDIDATE_SIGNAL_RUNGS_SECONDS_V1[2] == 180);
    assert!(CANDIDATE_SIGNAL_RUNGS_SECONDS_V1[3] == 300);
    assert!(CANDIDATE_SIGNAL_RUNGS_SECONDS_V1[4] == 600);
    assert!(CANDIDATE_SIGNAL_RUNGS_SECONDS_V1[5] == 900);
    assert!(CANDIDATE_SIGNAL_RUNGS_SECONDS_V1[6] == 1_800);
    assert!(CANDIDATE_SIGNAL_RUNGS_SECONDS_V1[7] == 3_600);
};

/// Complete caller decisions for one canonical stored eight-rung transaction.
///
/// The eight named Sweeper fields prevent positional array substitution.  The
/// coordinator itself owns the only rung/family topology and reuses each named
/// Sweeper for NIFTY then BANKNIFTY at that exact rung.
#[derive(Clone, Copy)]
pub(crate) struct AllRungStoredPopulationV5Request<'a> {
    /// Existing exact stored-market and Candidate ledger root.
    pub(crate) source_root: &'a Path,
    /// Existing parent of the eight exact canonical rung authority directories.
    pub(crate) authority_root: &'a Path,
    /// Exact stored feed.
    pub(crate) vendor: Vendor,
    /// Inclusive first requested `(year, month)`.
    pub(crate) from: (u16, u8),
    /// Inclusive last requested `(year, month)`.
    pub(crate) to: (u16, u8),
    /// One-minute Candidate sweeper.
    pub(crate) one_minute_sweeper: &'a Sweeper,
    /// Two-minute Candidate sweeper.
    pub(crate) two_minute_sweeper: &'a Sweeper,
    /// Three-minute Candidate sweeper.
    pub(crate) three_minute_sweeper: &'a Sweeper,
    /// Five-minute Candidate sweeper.
    pub(crate) five_minute_sweeper: &'a Sweeper,
    /// Ten-minute Candidate sweeper.
    pub(crate) ten_minute_sweeper: &'a Sweeper,
    /// Fifteen-minute Candidate sweeper.
    pub(crate) fifteen_minute_sweeper: &'a Sweeper,
    /// Thirty-minute Candidate sweeper.
    pub(crate) thirty_minute_sweeper: &'a Sweeper,
    /// Sixty-minute Candidate sweeper.
    pub(crate) sixty_minute_sweeper: &'a Sweeper,
    /// Explicit forward outcome horizon shared by the one requested cohort.
    pub(crate) horizon: Horizon,
    /// Explicit indicator tolerance widths.
    pub(crate) widths: Widths,
    /// Explicit volume/VWAP availability decision.
    pub(crate) availability: Availability,
    /// Explicit candlestick predicate thresholds.
    pub(crate) thresholds: Thresholds,
    /// Complete long-side dynamic exit-grid policy.
    pub(crate) long_exit_policy: &'a ExitGridPolicyV1,
    /// Complete short-side dynamic exit-grid policy.
    pub(crate) short_exit_policy: &'a ExitGridPolicyV1,
    /// Candidate, Base Evidence, Pre-Admission and stored-load ceilings.
    pub(crate) candidate_bounds: StoredCandidatePreAdmissionBoundsV1,
    /// Observation V1 ledger ceilings.
    pub(crate) observation_bounds: ObservationAuthorityBoundsV1,
    /// Statistics V2 ledger ceilings.
    pub(crate) statistics_bounds: PopulationStatisticsV2Bounds,
    /// Explicit deterministic Statistics V2 procedure.
    pub(crate) statistics_procedure: PopulationStatisticsProcedureV2,
    /// Search V4 lineage ledger ceilings.
    pub(crate) search_bounds: AnchoredSearchLineageV4Bounds,
    /// Admission V3 ledger ceilings.
    pub(crate) admission_bounds: AdmissionV3Bounds,
    /// Exact Runner admission policy.
    pub(crate) admission_policy: &'a AdmissionPolicyV1,
    /// Finalization V3 ledger ceilings.
    pub(crate) finalization_bounds: PopulationFinalizationV3Bounds,
    /// Population V5 ledger ceilings.
    pub(crate) population_bounds: PopulationV5Bounds,
}

impl AllRungStoredPopulationV5Request<'_> {
    fn sweeper(&self, index: usize) -> &Sweeper {
        match index {
            0 => self.one_minute_sweeper,
            1 => self.two_minute_sweeper,
            2 => self.three_minute_sweeper,
            3 => self.five_minute_sweeper,
            4 => self.ten_minute_sweeper,
            5 => self.fifteen_minute_sweeper,
            6 => self.thirty_minute_sweeper,
            7 => self.sixty_minute_sweeper,
            _ => unreachable!("canonical all-rung index is bounded by eight"),
        }
    }
}

/// Eight explicit Execution V3 output roots and ceilings.
///
/// Every field is named for one canonical signal rung. There is deliberately
/// no array constructor, iterator constructor, `Default`, shared root or
/// shared bound: a caller must name all eight destinations and all eight
/// resource ceilings. Root admission additionally checks each final path
/// component against its field name, so swapping two otherwise valid roots
/// refuses before the first Execution write.
#[derive(Clone, Copy)]
pub(crate) struct AllRungStoredExecutionV3Request<'a> {
    pub(crate) one_minute_root: &'a Path,
    pub(crate) one_minute_bounds: ExecutionV3Bounds,
    pub(crate) two_minute_root: &'a Path,
    pub(crate) two_minute_bounds: ExecutionV3Bounds,
    pub(crate) three_minute_root: &'a Path,
    pub(crate) three_minute_bounds: ExecutionV3Bounds,
    pub(crate) five_minute_root: &'a Path,
    pub(crate) five_minute_bounds: ExecutionV3Bounds,
    pub(crate) ten_minute_root: &'a Path,
    pub(crate) ten_minute_bounds: ExecutionV3Bounds,
    pub(crate) fifteen_minute_root: &'a Path,
    pub(crate) fifteen_minute_bounds: ExecutionV3Bounds,
    pub(crate) thirty_minute_root: &'a Path,
    pub(crate) thirty_minute_bounds: ExecutionV3Bounds,
    pub(crate) sixty_minute_root: &'a Path,
    pub(crate) sixty_minute_bounds: ExecutionV3Bounds,
}

/// Eight named Selection V5 root specifications admitted while every retained
/// Population and Execution root capability is still live.
///
/// These are caller-supplied destinations, not upstream-root disclosures. The
/// resulting authorization token exposes no path, identity or iterator.
#[derive(Clone, Copy)]
pub(crate) struct AllRungSelectionRootSpecs<'a> {
    /// Existing one-minute Selection root.
    pub(crate) one_minute: &'a Path,
    /// Existing two-minute Selection root.
    pub(crate) two_minute: &'a Path,
    /// Existing three-minute Selection root.
    pub(crate) three_minute: &'a Path,
    /// Existing five-minute Selection root.
    pub(crate) five_minute: &'a Path,
    /// Existing ten-minute Selection root.
    pub(crate) ten_minute: &'a Path,
    /// Existing fifteen-minute Selection root.
    pub(crate) fifteen_minute: &'a Path,
    /// Existing thirty-minute Selection root.
    pub(crate) thirty_minute: &'a Path,
    /// Existing sixty-minute Selection root.
    pub(crate) sixty_minute: &'a Path,
}

#[derive(Clone, Copy)]
struct CanonicalExecutionV3BoundsV1 {
    one_minute: ExecutionV3Bounds,
    two_minute: ExecutionV3Bounds,
    three_minute: ExecutionV3Bounds,
    five_minute: ExecutionV3Bounds,
    ten_minute: ExecutionV3Bounds,
    fifteen_minute: ExecutionV3Bounds,
    thirty_minute: ExecutionV3Bounds,
    sixty_minute: ExecutionV3Bounds,
}

impl From<&AllRungStoredExecutionV3Request<'_>> for CanonicalExecutionV3BoundsV1 {
    fn from(request: &AllRungStoredExecutionV3Request<'_>) -> Self {
        Self {
            one_minute: request.one_minute_bounds,
            two_minute: request.two_minute_bounds,
            three_minute: request.three_minute_bounds,
            five_minute: request.five_minute_bounds,
            ten_minute: request.ten_minute_bounds,
            fifteen_minute: request.fifteen_minute_bounds,
            thirty_minute: request.thirty_minute_bounds,
            sixty_minute: request.sixty_minute_bounds,
        }
    }
}

/// Opaque non-cloneable authority for exactly eight retained Population V5s.
///
/// There is no constructor from an array, vector, identifier, digest or
/// detached receipt.  Only [`commit_all_rung_stored_population_v5`] can mint
/// the wrapper after authenticating every rung and the complete live root
/// topology.
pub(crate) struct CommittedStoredAllRungPopulationV5 {
    populations: [CommittedStoredPopulationV5; RUNG_COUNT],
    roots: AdmittedAllRungRootsV1,
}

impl CommittedStoredAllRungPopulationV5 {
    /// Number of rung authorities written rather than exactly reused.
    #[must_use]
    pub(crate) fn written_rung_count(&self) -> usize {
        self.populations
            .iter()
            .filter(|population| population.was_written())
            .count()
    }

    /// Reauthenticates all eight retained Population V5s and every root.
    pub(crate) fn require_live_topology(&mut self) -> Result<(), String> {
        self.roots
            .require_same("while revalidating retained all-rung Population V5")?;
        require_all_population_topology_v1(&mut self.populations)?;
        self.roots
            .require_same("after revalidating retained all-rung Population V5")
    }
}

struct CanonicalStoredExecutionV3AuthoritiesV1 {
    one_minute: CommittedStoredExecutionV3,
    two_minute: CommittedStoredExecutionV3,
    three_minute: CommittedStoredExecutionV3,
    five_minute: CommittedStoredExecutionV3,
    ten_minute: CommittedStoredExecutionV3,
    fifteen_minute: CommittedStoredExecutionV3,
    thirty_minute: CommittedStoredExecutionV3,
    sixty_minute: CommittedStoredExecutionV3,
}

/// Named one-use Execution V3 sources for the canonical Selection successor.
///
/// This is deliberately not an array, tuple or iterator. Each live capability
/// remains bound to its canonical rung name while ownership crosses the typed
/// successor door.
pub(crate) struct AllRungExecutionV3SelectionSources {
    /// One-minute retained Execution source.
    pub(crate) one_minute: CommittedStoredExecutionV3,
    /// Two-minute retained Execution source.
    pub(crate) two_minute: CommittedStoredExecutionV3,
    /// Three-minute retained Execution source.
    pub(crate) three_minute: CommittedStoredExecutionV3,
    /// Five-minute retained Execution source.
    pub(crate) five_minute: CommittedStoredExecutionV3,
    /// Ten-minute retained Execution source.
    pub(crate) ten_minute: CommittedStoredExecutionV3,
    /// Fifteen-minute retained Execution source.
    pub(crate) fifteen_minute: CommittedStoredExecutionV3,
    /// Thirty-minute retained Execution source.
    pub(crate) thirty_minute: CommittedStoredExecutionV3,
    /// Sixty-minute retained Execution source.
    pub(crate) sixty_minute: CommittedStoredExecutionV3,
    /// Opaque retained proof covering upstream and Selection root topology.
    pub(crate) topology: AllRungSelectionTopologyToken,
}

/// Opaque retained proof that Selection roots remain physically disjoint from
/// every Population/Execution source root and from one another.
pub(crate) struct AllRungSelectionTopologyToken {
    population_roots: AdmittedAllRungRootsV1,
    execution_roots: AdmittedAllRungExecutionRootsV1,
    selection_roots: AdmittedAllRungSelectionRootsV1,
}

impl AllRungSelectionTopologyToken {
    /// Rechecks every held directory capability without revealing a path or
    /// physical identity to the successor module.
    pub(crate) fn require_same(&self, stage: &str) -> Result<(), String> {
        self.population_roots
            .require_same(&format!("{stage} (Population topology)"))?;
        self.execution_roots
            .require_same(&format!("{stage} (Execution topology)"))?;
        self.selection_roots
            .require_same(&format!("{stage} (Selection topology)"))
    }
}

#[derive(Clone, Copy)]
struct RungExecutionV3ReceiptsV1 {
    population: PopulationV5StructuralReceipt,
    execution: ExecutionV3StructuralReceipt,
}

#[derive(Clone, Copy)]
struct CanonicalExecutionV3ReceiptsV1 {
    one_minute: RungExecutionV3ReceiptsV1,
    two_minute: RungExecutionV3ReceiptsV1,
    three_minute: RungExecutionV3ReceiptsV1,
    five_minute: RungExecutionV3ReceiptsV1,
    ten_minute: RungExecutionV3ReceiptsV1,
    fifteen_minute: RungExecutionV3ReceiptsV1,
    thirty_minute: RungExecutionV3ReceiptsV1,
    sixty_minute: RungExecutionV3ReceiptsV1,
}

/// Opaque canonical eight-rung Execution V3 authority.
///
/// The eight live authorities remain named and private. No raw array, vector,
/// detached receipt or reorderable iterator escapes. A successor must consume
/// this complete wrapper through another typed production door.
pub(crate) struct CommittedStoredAllRungExecutionV3 {
    executions: CanonicalStoredExecutionV3AuthoritiesV1,
    population_roots: AdmittedAllRungRootsV1,
    execution_roots: AdmittedAllRungExecutionRootsV1,
    bounds: CanonicalExecutionV3BoundsV1,
    receipts: CanonicalExecutionV3ReceiptsV1,
}

impl CommittedStoredAllRungExecutionV3 {
    /// Number of rung blocks written rather than byte-exactly reused.
    #[must_use]
    pub(crate) fn written_rung_count(&self) -> usize {
        usize::from(self.executions.one_minute.was_written())
            + usize::from(self.executions.two_minute.was_written())
            + usize::from(self.executions.three_minute.was_written())
            + usize::from(self.executions.five_minute.was_written())
            + usize::from(self.executions.ten_minute.was_written())
            + usize::from(self.executions.fifteen_minute.was_written())
            + usize::from(self.executions.thirty_minute.was_written())
            + usize::from(self.executions.sixty_minute.was_written())
    }

    /// Reauthenticates all held roots, bounds, source receipts, rung identities
    /// and durable disposition blocks in literal canonical order.
    pub(crate) fn require_live_topology(&mut self) -> Result<(), String> {
        self.population_roots
            .require_same("before retained all-rung Execution V3 reauthentication")?;
        self.execution_roots
            .require_same("before retained all-rung Execution V3 reauthentication")?;
        require_committed_execution_rung_v1(
            &mut self.executions.one_minute,
            self.bounds.one_minute,
            self.receipts.one_minute,
            60,
            "1min",
        )?;
        require_committed_execution_rung_v1(
            &mut self.executions.two_minute,
            self.bounds.two_minute,
            self.receipts.two_minute,
            120,
            "2min",
        )?;
        require_committed_execution_rung_v1(
            &mut self.executions.three_minute,
            self.bounds.three_minute,
            self.receipts.three_minute,
            180,
            "3min",
        )?;
        require_committed_execution_rung_v1(
            &mut self.executions.five_minute,
            self.bounds.five_minute,
            self.receipts.five_minute,
            300,
            "5min",
        )?;
        require_committed_execution_rung_v1(
            &mut self.executions.ten_minute,
            self.bounds.ten_minute,
            self.receipts.ten_minute,
            600,
            "10min",
        )?;
        require_committed_execution_rung_v1(
            &mut self.executions.fifteen_minute,
            self.bounds.fifteen_minute,
            self.receipts.fifteen_minute,
            900,
            "15min",
        )?;
        require_committed_execution_rung_v1(
            &mut self.executions.thirty_minute,
            self.bounds.thirty_minute,
            self.receipts.thirty_minute,
            1_800,
            "30min",
        )?;
        require_committed_execution_rung_v1(
            &mut self.executions.sixty_minute,
            self.bounds.sixty_minute,
            self.receipts.sixty_minute,
            3_600,
            "60min",
        )?;
        self.population_roots
            .require_same("after retained all-rung Execution V3 reauthentication")?;
        self.execution_roots
            .require_same("after retained all-rung Execution V3 reauthentication")
    }

    /// Performs one final complete topology reauthentication, then moves the
    /// eight named sources into the only typed Selection successor shape.
    /// No detached receipt or reorderable collection escapes.
    pub(crate) fn into_selection_sources(
        mut self,
        roots: &AllRungSelectionRootSpecs<'_>,
    ) -> Result<AllRungExecutionV3SelectionSources, String> {
        self.require_live_topology()?;
        let selection_roots = AdmittedAllRungSelectionRootsV1::admit(
            roots,
            &self.population_roots,
            &self.execution_roots,
        )?;
        self.require_live_topology()?;
        let Self {
            executions,
            population_roots,
            execution_roots,
            bounds: _,
            receipts: _,
        } = self;
        let CanonicalStoredExecutionV3AuthoritiesV1 {
            one_minute,
            two_minute,
            three_minute,
            five_minute,
            ten_minute,
            fifteen_minute,
            thirty_minute,
            sixty_minute,
        } = executions;
        Ok(AllRungExecutionV3SelectionSources {
            one_minute,
            two_minute,
            three_minute,
            five_minute,
            ten_minute,
            fifteen_minute,
            thirty_minute,
            sixty_minute,
            topology: AllRungSelectionTopologyToken {
                population_roots,
                execution_roots,
                selection_roots,
            },
        })
    }
}

struct CandidateFamilyPairV1 {
    nifty: CommittedStoredCandidatePreAdmissionV1,
    banknifty: CommittedStoredCandidatePreAdmissionV1,
}

/// Commits the sole canonical stored all-rung chain through Population V5.
///
/// Candidate/Pre-Admission production is completed for all eight rungs before
/// any successor reader is retained.  Successors then use exactly one isolated
/// canonical directory per rung.  Every one-rung producer consumes its opaque
/// predecessor directly; no decoded record, digest or identity crosses the
/// coordinator boundary.
///
/// # Errors
///
/// Refuses a noncanonical, missing, symlinked, aliased, replaced or removed
/// root before success; any one-rung stored/policy/ledger refusal; a rung or
/// family topology mismatch; or failure to retain exactly eight authorities.
/// Earlier receipt-last appends may remain as recoverable prefixes after a
/// later refusal, and an exact retry resumes/reuses them.
///
/// # Cost
///
/// This is a bounded whole-history, whole-population transaction and is not
/// O(1).  It performs exactly sixteen naturally-extinct Candidate sweeps plus
/// eight bounded successor chains.  Fixed rung/family dispatch and root lookup
/// are O(1); retained evidence is proportional to the admitted bounded inputs.
#[allow(
    clippy::too_many_lines,
    reason = "the fixed two-phase eight-rung transaction is kept visible as one indivisible authority door"
)]
pub(crate) fn commit_all_rung_stored_population_v5(
    request: &AllRungStoredPopulationV5Request<'_>,
) -> Result<CommittedStoredAllRungPopulationV5, String> {
    let roots = AdmittedAllRungRootsV1::admit(request.source_root, request.authority_root)?;
    roots.require_same("before the first all-rung write")?;

    // Phase one freezes every shared-root Candidate/Base/Pre-Admission append
    // before any successor ledger reader is retained.
    let mut candidate_pairs = Vec::with_capacity(RUNG_COUNT);
    for (index, rung_name) in CANONICAL_RUNG_NAMES_V1.iter().copied().enumerate() {
        roots.require_same(&format!(
            "before {rung_name} NIFTY Candidate/Pre-Admission commit"
        ))?;
        let nifty = commit_stored_candidate_pre_admission_authority_v1(
            StoredCandidatePreAdmissionRequestV1 {
                root: roots.source.path(),
                vendor: request.vendor,
                underlying: CANONICAL_UNDERLYINGS_V1[0],
                rung_name,
                from: request.from,
                to: request.to,
                sweeper: request.sweeper(index),
                horizon: request.horizon,
                widths: request.widths,
                availability: request.availability,
                thresholds: request.thresholds,
                long_exit_policy: request.long_exit_policy,
                short_exit_policy: request.short_exit_policy,
                bounds: request.candidate_bounds,
            },
        )
        .map_err(|why| format!("all-rung {rung_name} NIFTY refused: {why}"))?;

        roots.require_same(&format!(
            "between {rung_name} NIFTY and BANKNIFTY Candidate/Pre-Admission commits"
        ))?;
        let banknifty = commit_stored_candidate_pre_admission_authority_v1(
            StoredCandidatePreAdmissionRequestV1 {
                root: roots.source.path(),
                vendor: request.vendor,
                underlying: CANONICAL_UNDERLYINGS_V1[1],
                rung_name,
                from: request.from,
                to: request.to,
                sweeper: request.sweeper(index),
                horizon: request.horizon,
                widths: request.widths,
                availability: request.availability,
                thresholds: request.thresholds,
                long_exit_policy: request.long_exit_policy,
                short_exit_policy: request.short_exit_policy,
                bounds: request.candidate_bounds,
            },
        )
        .map_err(|why| format!("all-rung {rung_name} BANKNIFTY refused: {why}"))?;
        roots.require_same(&format!(
            "after {rung_name} BANKNIFTY Candidate/Pre-Admission commit"
        ))?;
        candidate_pairs.push(CandidateFamilyPairV1 { nifty, banknifty });
    }
    let candidate_count = candidate_pairs.len();
    let candidate_pairs: [CandidateFamilyPairV1; RUNG_COUNT] = candidate_pairs
        .try_into()
        .map_err(|_| format!("all-rung Candidate phase retained {candidate_count} pairs, not 8"))?;

    // Phase two can no longer mutate the shared Candidate source.  Each rung
    // owns a disjoint physical authority directory for all of its successor
    // files, so a later rung cannot stale an earlier retained V5 chain.
    let mut populations = Vec::with_capacity(RUNG_COUNT);
    for (((pair, &rung_name), &rung_seconds), rung_directory) in candidate_pairs
        .into_iter()
        .zip(CANONICAL_RUNG_NAMES_V1.iter())
        .zip(CANDIDATE_SIGNAL_RUNGS_SECONDS_V1.iter())
        .zip(roots.rungs.iter())
    {
        let rung_root = rung_directory.path();
        roots.require_same(&format!("before {rung_name} Observation/Statistics commit"))?;
        let observations = commit_stored_observation_statistics_v2(
            pair.nifty,
            pair.banknifty,
            rung_root,
            rung_root,
            request.observation_bounds,
            request.statistics_bounds,
            request.statistics_procedure,
        )
        .map_err(|why| format!("all-rung {rung_name} Observation/Statistics refused: {why}"))?;

        roots.require_same(&format!("before {rung_name} Search V4 commit"))?;
        let search =
            commit_stored_search_lineage_v4(observations, rung_root, request.search_bounds)
                .map_err(|why| format!("all-rung {rung_name} Search V4 refused: {why}"))?;

        roots.require_same(&format!("before {rung_name} Admission V3 commit"))?;
        let admission = commit_stored_population_admission_v3(
            search,
            rung_root,
            request.admission_bounds,
            request.admission_policy,
        )
        .map_err(|why| format!("all-rung {rung_name} Admission V3 refused: {why}"))?;

        roots.require_same(&format!("before {rung_name} Finalization V3 commit"))?;
        let finalization = commit_stored_population_finalization_v3(
            admission,
            rung_root,
            request.finalization_bounds,
        )
        .map_err(|why| format!("all-rung {rung_name} Finalization V3 refused: {why}"))?;

        roots.require_same(&format!("before {rung_name} Population V5 commit"))?;
        let mut population =
            commit_population_v5(rung_root, request.population_bounds, finalization)
                .map_err(|why| format!("all-rung {rung_name} Population V5 refused: {why}"))?;
        require_population_rung_topology_v1(&mut population, rung_seconds, rung_name)?;
        roots.require_same(&format!("after {rung_name} Population V5 authentication"))?;
        populations.push(population);
    }
    let population_count = populations.len();
    let mut populations: [CommittedStoredPopulationV5; RUNG_COUNT] = populations
        .try_into()
        .map_err(|_| format!("all-rung successor phase retained {population_count} V5s, not 8"))?;
    require_all_population_topology_v1(&mut populations)?;
    roots.require_same("after complete all-rung Population V5 authentication")?;

    Ok(CommittedStoredAllRungPopulationV5 { populations, roots })
}

/// Commits the canonical eight retained Population V5 authorities through the
/// sole one-rung Execution V3 production door.
///
/// The source is consumed by value, its private array is destructured exactly
/// once, and the calls below are deliberately written in canonical rung order.
/// All eight named execution roots are admitted before the first append. A
/// refusal after an earlier receipt-last append leaves that exact prefix for a
/// byte-identical retry after the caller reconstructs the Population V5 source
/// from its durable predecessors.
///
/// # Errors
///
/// Refuses missing, noncanonical, symlinked, aliased or replaced roots;
/// foreign/reordered Population or Execution receipts; a rung, family, bound,
/// disposition-count or durable-row mismatch; stale upstream generations; or
/// any one-rung Execution V3 persistence/reopen refusal.
///
/// # Cost
///
/// This performs eight complete retained-source replays and eight bounded
/// receipt-last file transactions. Its time and retained space grow with the
/// authenticated Population/file bytes. Only fixed eight-rung dispatch and
/// fixed-stride record addressing are O(1) in rung/candidate count.
#[allow(
    clippy::too_many_lines,
    reason = "the literal eight-rung move order is the authority and must remain reviewable"
)]
pub(crate) fn commit_all_rung_stored_execution_v3(
    mut source: CommittedStoredAllRungPopulationV5,
    request: &AllRungStoredExecutionV3Request<'_>,
) -> Result<CommittedStoredAllRungExecutionV3, String> {
    source.require_live_topology()?;
    let execution_roots = AdmittedAllRungExecutionRootsV1::admit(request, &source.roots)?;
    let bounds = CanonicalExecutionV3BoundsV1::from(request);
    source.require_live_topology()?;

    let CommittedStoredAllRungPopulationV5 {
        populations,
        roots: population_roots,
    } = source;
    let [
        one_minute_population,
        two_minute_population,
        three_minute_population,
        five_minute_population,
        ten_minute_population,
        fifteen_minute_population,
        thirty_minute_population,
        sixty_minute_population,
    ] = populations;

    let (one_minute, one_minute_receipts) = commit_execution_rung_v1(
        one_minute_population,
        &population_roots,
        &execution_roots,
        &execution_roots.one_minute,
        bounds.one_minute,
        60,
        "1min",
    )?;
    let (two_minute, two_minute_receipts) = commit_execution_rung_v1(
        two_minute_population,
        &population_roots,
        &execution_roots,
        &execution_roots.two_minute,
        bounds.two_minute,
        120,
        "2min",
    )?;
    let (three_minute, three_minute_receipts) = commit_execution_rung_v1(
        three_minute_population,
        &population_roots,
        &execution_roots,
        &execution_roots.three_minute,
        bounds.three_minute,
        180,
        "3min",
    )?;
    let (five_minute, five_minute_receipts) = commit_execution_rung_v1(
        five_minute_population,
        &population_roots,
        &execution_roots,
        &execution_roots.five_minute,
        bounds.five_minute,
        300,
        "5min",
    )?;
    let (ten_minute, ten_minute_receipts) = commit_execution_rung_v1(
        ten_minute_population,
        &population_roots,
        &execution_roots,
        &execution_roots.ten_minute,
        bounds.ten_minute,
        600,
        "10min",
    )?;
    let (fifteen_minute, fifteen_minute_receipts) = commit_execution_rung_v1(
        fifteen_minute_population,
        &population_roots,
        &execution_roots,
        &execution_roots.fifteen_minute,
        bounds.fifteen_minute,
        900,
        "15min",
    )?;
    let (thirty_minute, thirty_minute_receipts) = commit_execution_rung_v1(
        thirty_minute_population,
        &population_roots,
        &execution_roots,
        &execution_roots.thirty_minute,
        bounds.thirty_minute,
        1_800,
        "30min",
    )?;
    let (sixty_minute, sixty_minute_receipts) = commit_execution_rung_v1(
        sixty_minute_population,
        &population_roots,
        &execution_roots,
        &execution_roots.sixty_minute,
        bounds.sixty_minute,
        3_600,
        "60min",
    )?;

    let mut committed = CommittedStoredAllRungExecutionV3 {
        executions: CanonicalStoredExecutionV3AuthoritiesV1 {
            one_minute,
            two_minute,
            three_minute,
            five_minute,
            ten_minute,
            fifteen_minute,
            thirty_minute,
            sixty_minute,
        },
        population_roots,
        execution_roots,
        bounds,
        receipts: CanonicalExecutionV3ReceiptsV1 {
            one_minute: one_minute_receipts,
            two_minute: two_minute_receipts,
            three_minute: three_minute_receipts,
            five_minute: five_minute_receipts,
            ten_minute: ten_minute_receipts,
            fifteen_minute: fifteen_minute_receipts,
            thirty_minute: thirty_minute_receipts,
            sixty_minute: sixty_minute_receipts,
        },
    };
    committed.require_live_topology()?;
    Ok(committed)
}

fn commit_execution_rung_v1(
    mut population: CommittedStoredPopulationV5,
    population_roots: &AdmittedAllRungRootsV1,
    execution_roots: &AdmittedAllRungExecutionRootsV1,
    execution_root: &AdmittedDirectoryV1,
    bounds: ExecutionV3Bounds,
    expected_rung_seconds: u32,
    rung_name: &str,
) -> Result<(CommittedStoredExecutionV3, RungExecutionV3ReceiptsV1), String> {
    population_roots.require_same(&format!("before {rung_name} Execution V3 commit"))?;
    execution_roots.require_same(&format!("before {rung_name} Execution V3 commit"))?;
    let population_receipt = population.structural_receipt();
    require_population_rung_topology_v1(&mut population, expected_rung_seconds, rung_name)?;
    if population.structural_receipt() != population_receipt {
        return Err(format!(
            "all-rung {rung_name} Population V5 receipt changed before Execution V3 commit"
        ));
    }

    let mut execution = commit_stored_execution_v3(execution_root.path(), bounds, population)
        .map_err(|why| format!("all-rung {rung_name} Execution V3 refused: {why}"))?;
    let receipts = RungExecutionV3ReceiptsV1 {
        population: population_receipt,
        execution: execution.structural_receipt(),
    };
    require_committed_execution_rung_v1(
        &mut execution,
        bounds,
        receipts,
        expected_rung_seconds,
        rung_name,
    )?;
    population_roots.require_same(&format!("after {rung_name} Execution V3 commit"))?;
    execution_roots.require_same(&format!("after {rung_name} Execution V3 commit"))?;
    Ok((execution, receipts))
}

fn require_committed_execution_rung_v1(
    execution: &mut CommittedStoredExecutionV3,
    expected_bounds: ExecutionV3Bounds,
    expected_receipts: RungExecutionV3ReceiptsV1,
    expected_rung_seconds: u32,
    rung_name: &str,
) -> Result<(), String> {
    require_execution_bounds_v1(rung_name, expected_bounds, execution.bounds())?;
    let execution_receipt_before = execution.structural_receipt();
    require_execution_receipt_join_v1(
        rung_name,
        expected_receipts.population,
        expected_receipts.execution,
        execution_receipt_before,
    )?;

    let source = execution
        .population_execution_source()
        .map_err(|why| format!("all-rung {rung_name} retained Population refused: {why}"))?;
    require_execution_source_rung_v1(
        &source,
        expected_receipts.population,
        expected_rung_seconds,
        rung_name,
    )?;
    let dispositions = execution
        .ordered_authenticated_dispositions()
        .map_err(|why| format!("all-rung {rung_name} durable Execution refused: {why}"))?;
    let disposition_count = u64::try_from(dispositions.len()).map_err(|_| {
        format!("all-rung {rung_name} durable Execution disposition count does not fit u64")
    })?;
    if disposition_count != expected_receipts.population.row_count() {
        return Err(format!(
            "all-rung {rung_name} durable Execution disposition count {disposition_count} differs from Population count {}",
            expected_receipts.population.row_count()
        ));
    }
    require_execution_disposition_topology_v1(
        rung_name,
        expected_rung_seconds,
        expected_receipts.population.population_id(),
        expected_receipts.population.row_count(),
        dispositions
            .iter()
            .zip(source.rows())
            .map(|(execution, source)| {
                (
                    execution.global_sequence(),
                    execution.rung(),
                    execution.population_id(),
                    execution.population_row_id(),
                    source.population().row_id(),
                    execution.family(),
                    source.population().family(),
                )
            }),
    )?;
    let execution_receipt_after = execution.structural_receipt();
    if execution_receipt_after != execution_receipt_before
        || execution_receipt_after != expected_receipts.execution
    {
        return Err(format!(
            "all-rung {rung_name} Execution V3 structural receipt changed during reauthentication"
        ));
    }
    require_execution_bounds_v1(rung_name, expected_bounds, execution.bounds())
}

fn require_execution_source_rung_v1(
    source: &PopulationV5ExecutionV3SourceV1,
    expected_receipt: PopulationV5StructuralReceipt,
    expected_rung_seconds: u32,
    rung_name: &str,
) -> Result<(), String> {
    if source.receipt() != expected_receipt {
        return Err(format!(
            "all-rung {rung_name} Execution source carries a foreign Population receipt"
        ));
    }
    if source
        .parameters()
        .iter()
        .any(|parameter| parameter.rung_seconds != expected_rung_seconds)
    {
        return Err(format!(
            "all-rung {rung_name} Execution parameter block names a foreign rung"
        ));
    }
    require_rung_row_topology_v1(
        rung_name,
        expected_rung_seconds,
        expected_receipt.row_count(),
        expected_receipt.nifty_count(),
        expected_receipt.banknifty_count(),
        source.rows().iter().map(|row| {
            (
                row.population().candidate().row().rung_seconds(),
                row.population().family(),
                row.population().candidate().row().family(),
            )
        }),
    )
}

fn require_execution_bounds_v1(
    rung_name: &str,
    expected: ExecutionV3Bounds,
    actual: ExecutionV3Bounds,
) -> Result<(), String> {
    if actual != expected {
        return Err(format!(
            "all-rung {rung_name} Execution V3 reopened with foreign bounds"
        ));
    }
    Ok(())
}

fn require_execution_receipt_join_v1(
    rung_name: &str,
    expected_population: PopulationV5StructuralReceipt,
    expected_execution: ExecutionV3StructuralReceipt,
    actual_execution: ExecutionV3StructuralReceipt,
) -> Result<(), String> {
    if actual_execution != expected_execution {
        return Err(format!(
            "all-rung {rung_name} Execution V3 carries a foreign structural receipt"
        ));
    }
    require_execution_population_identity_v1(
        rung_name,
        expected_population.population_id(),
        actual_execution.population_id(),
    )?;
    if actual_execution.population_ordered_digest() != expected_population.ordered_row_digest() {
        return Err(format!(
            "all-rung {rung_name} Execution V3 receipt carries a foreign Population row digest"
        ));
    }
    if actual_execution.disposition_count() != expected_population.row_count() {
        return Err(format!(
            "all-rung {rung_name} Execution V3 receipt disposition count differs from Population"
        ));
    }
    Ok(())
}

fn require_execution_population_identity_v1(
    rung_name: &str,
    expected_population_id: [u8; 32],
    actual_population_id: [u8; 32],
) -> Result<(), String> {
    if actual_population_id != expected_population_id {
        return Err(format!(
            "all-rung {rung_name} Execution V3 receipt names a foreign Population"
        ));
    }
    Ok(())
}

fn require_execution_disposition_topology_v1(
    rung_name: &str,
    expected_rung_seconds: u32,
    expected_population_id: [u8; 32],
    expected_row_count: u64,
    rows: impl IntoIterator<
        Item = (
            u64,
            u32,
            [u8; 32],
            [u8; 32],
            [u8; 32],
            ExecutionV3Family,
            AdmissionV3Family,
        ),
    >,
) -> Result<(), String> {
    let mut observed = 0_u64;
    let mut banknifty_seen = false;
    for (
        global_sequence,
        rung_seconds,
        population_id,
        execution_population_row_id,
        source_population_row_id,
        execution_family,
        population_family,
    ) in rows
    {
        if global_sequence != observed {
            return Err(format!(
                "all-rung {rung_name} Execution disposition sequence {global_sequence} is not canonical {observed}"
            ));
        }
        if rung_seconds != expected_rung_seconds {
            return Err(format!(
                "all-rung {rung_name} Execution disposition {global_sequence} names {rung_seconds}s, expected {expected_rung_seconds}s"
            ));
        }
        if population_id != expected_population_id
            || execution_population_row_id != source_population_row_id
        {
            return Err(format!(
                "all-rung {rung_name} Execution disposition {global_sequence} names a foreign Population row"
            ));
        }
        match (execution_family, population_family, banknifty_seen) {
            (ExecutionV3Family::Nifty, AdmissionV3Family::Nifty, false) => {}
            (ExecutionV3Family::BankNifty, AdmissionV3Family::BankNifty, _) => {
                banknifty_seen = true;
            }
            _ => {
                return Err(format!(
                    "all-rung {rung_name} Execution disposition {global_sequence} breaks NIFTY-then-BANKNIFTY family topology"
                ));
            }
        }
        observed = observed
            .checked_add(1)
            .ok_or_else(|| format!("all-rung {rung_name} Execution row count overflowed"))?;
    }
    if observed != expected_row_count {
        return Err(format!(
            "all-rung {rung_name} Execution disposition count {observed} differs from Population count {expected_row_count}"
        ));
    }
    Ok(())
}

fn require_all_population_topology_v1(
    populations: &mut [CommittedStoredPopulationV5; RUNG_COUNT],
) -> Result<(), String> {
    for ((population, &rung_seconds), &rung_name) in populations
        .iter_mut()
        .zip(CANDIDATE_SIGNAL_RUNGS_SECONDS_V1.iter())
        .zip(CANONICAL_RUNG_NAMES_V1.iter())
    {
        require_population_rung_topology_v1(population, rung_seconds, rung_name)?;
    }
    Ok(())
}

fn require_population_rung_topology_v1(
    population: &mut CommittedStoredPopulationV5,
    expected_rung_seconds: u32,
    rung_name: &str,
) -> Result<(), String> {
    let receipt_before = population.structural_receipt();
    let rows = population
        .ordered_authenticated_rows()
        .map_err(|why| format!("all-rung {rung_name} V5 reauthentication refused: {why}"))?;
    let receipt_after = population.structural_receipt();
    if receipt_after != receipt_before {
        return Err(format!(
            "all-rung {rung_name} V5 structural receipt changed during authenticated topology read"
        ));
    }
    require_rung_row_topology_v1(
        rung_name,
        expected_rung_seconds,
        receipt_before.row_count(),
        receipt_before.nifty_count(),
        receipt_before.banknifty_count(),
        rows.iter().map(|row| {
            (
                row.candidate().row().rung_seconds(),
                row.family(),
                row.candidate().row().family(),
            )
        }),
    )
}

fn require_rung_row_topology_v1(
    rung_name: &str,
    expected_rung_seconds: u32,
    expected_row_count: u64,
    expected_nifty_count: u64,
    expected_banknifty_count: u64,
    rows: impl IntoIterator<Item = (u32, AdmissionV3Family, InstrumentFamilyV1)>,
) -> Result<(), String> {
    let mut row_count = 0_u64;
    let mut nifty_count = 0_u64;
    let mut banknifty_count = 0_u64;
    let mut banknifty_seen = false;
    for (ordinal, (candidate_rung_seconds, population_family, candidate_family)) in
        rows.into_iter().enumerate()
    {
        row_count = row_count
            .checked_add(1)
            .ok_or_else(|| format!("all-rung {rung_name} V5 row count overflowed"))?;
        if candidate_rung_seconds != expected_rung_seconds {
            return Err(format!(
                "all-rung {rung_name} V5 row {ordinal} names {candidate_rung_seconds}s, expected {expected_rung_seconds}s"
            ));
        }
        let expected_family = match population_family {
            AdmissionV3Family::Nifty if !banknifty_seen => {
                nifty_count = nifty_count
                    .checked_add(1)
                    .ok_or_else(|| format!("all-rung {rung_name} V5 NIFTY row count overflowed"))?;
                InstrumentFamilyV1::Nifty
            }
            AdmissionV3Family::Nifty => {
                return Err(format!(
                    "all-rung {rung_name} V5 row {ordinal} returns to NIFTY after BANKNIFTY"
                ));
            }
            AdmissionV3Family::BankNifty => {
                banknifty_seen = true;
                banknifty_count = banknifty_count.checked_add(1).ok_or_else(|| {
                    format!("all-rung {rung_name} V5 BANKNIFTY row count overflowed")
                })?;
                InstrumentFamilyV1::BankNifty
            }
        };
        if candidate_family != expected_family {
            return Err(format!(
                "all-rung {rung_name} V5 row {ordinal} Candidate family differs from its authenticated V5 family"
            ));
        }
    }
    if row_count != expected_row_count
        || nifty_count != expected_nifty_count
        || banknifty_count != expected_banknifty_count
        || nifty_count.checked_add(banknifty_count) != Some(row_count)
    {
        return Err(format!(
            "all-rung {rung_name} V5 observed row/family counts {row_count}/{nifty_count}/{banknifty_count} differ from authenticated receipt {expected_row_count}/{expected_nifty_count}/{expected_banknifty_count}"
        ));
    }
    Ok(())
}

struct AdmittedDirectoryV1 {
    canonical: PathBuf,
    directory: File,
    identity: [u64; 2],
}

impl AdmittedDirectoryV1 {
    fn admit(label: &str, root: &Path) -> Result<Self, String> {
        let spelling = fs::symlink_metadata(root).map_err(|why| {
            format!(
                "all-rung root admission could not inspect {label} {}: {why}",
                root.display()
            )
        })?;
        if spelling.file_type().is_symlink() {
            return Err(format!(
                "all-rung root admission refused symlinked {label} {}",
                root.display()
            ));
        }
        if !spelling.is_dir() {
            return Err(format!(
                "all-rung root admission refused {label} {} because it is not a directory",
                root.display()
            ));
        }
        let canonical = fs::canonicalize(root).map_err(|why| {
            format!(
                "all-rung root admission could not canonicalize {label} {}: {why}",
                root.display()
            )
        })?;
        if canonical.as_path() != root {
            return Err(format!(
                "all-rung root admission requires the exact canonical spelling for {label}; {} resolves to {}",
                root.display(),
                canonical.display()
            ));
        }
        let identity = directory_identity_v1(&spelling)?;
        let directory = File::open(&canonical).map_err(|why| {
            format!(
                "all-rung root admission could not hold {label} {}: {why}",
                canonical.display()
            )
        })?;
        let held = directory
            .metadata()
            .map_err(|why| format!("all-rung held {label} metadata refused: {why}"))
            .and_then(|metadata| directory_identity_v1(&metadata))?;
        if held != identity {
            return Err(format!(
                "all-rung {label} changed between pathname admission and capability open"
            ));
        }
        Ok(Self {
            canonical,
            directory,
            identity,
        })
    }

    fn path(&self) -> &Path {
        self.canonical.as_path()
    }

    fn aliases(&self, other: &Self) -> bool {
        self.identity == other.identity
    }

    fn require_same(&self, stage: &str) -> Result<(), String> {
        let held = self
            .directory
            .metadata()
            .map_err(|why| format!("all-rung held root metadata failed at {stage}: {why}"))
            .and_then(|metadata| directory_identity_v1(&metadata))?;
        if held != self.identity {
            return Err(format!(
                "all-rung held root directory generation changed at {stage}"
            ));
        }
        let recanonicalized = fs::canonicalize(&self.canonical)
            .map_err(|why| format!("all-rung canonical root disappeared at {stage}: {why}"))?;
        if recanonicalized != self.canonical {
            return Err(format!(
                "all-rung canonical root pathname resolved elsewhere at {stage}"
            ));
        }
        let current = fs::symlink_metadata(&self.canonical)
            .map_err(|why| format!("all-rung canonical root metadata failed at {stage}: {why}"))?;
        if current.file_type().is_symlink() || directory_identity_v1(&current)? != self.identity {
            return Err(format!(
                "all-rung canonical root was substituted or symlinked at {stage}"
            ));
        }
        Ok(())
    }
}

struct AdmittedAllRungRootsV1 {
    source: AdmittedDirectoryV1,
    authority: AdmittedDirectoryV1,
    rungs: [AdmittedDirectoryV1; RUNG_COUNT],
}

impl AdmittedAllRungRootsV1 {
    fn admit(source_root: &Path, authority_root: &Path) -> Result<Self, String> {
        let source = AdmittedDirectoryV1::admit("stored source root", source_root)?;
        let authority = AdmittedDirectoryV1::admit("all-rung authority root", authority_root)?;
        if source.aliases(&authority) {
            return Err(
                "all-rung authority root physically aliases the stored source root".to_owned(),
            );
        }

        let mut rungs = Vec::with_capacity(RUNG_COUNT);
        for rung_name in CANONICAL_RUNG_NAMES_V1 {
            let path = authority.path().join(rung_name);
            let admitted =
                AdmittedDirectoryV1::admit(&format!("{rung_name} authority root"), path.as_path())?;
            if admitted.aliases(&source) || admitted.aliases(&authority) {
                return Err(format!(
                    "all-rung {rung_name} authority root physically aliases a parent/source root"
                ));
            }
            if rungs
                .iter()
                .any(|prior: &AdmittedDirectoryV1| admitted.aliases(prior))
            {
                return Err(format!(
                    "all-rung {rung_name} authority root physically aliases another rung"
                ));
            }
            rungs.push(admitted);
        }
        let rung_count = rungs.len();
        let rungs: [AdmittedDirectoryV1; RUNG_COUNT] = rungs
            .try_into()
            .map_err(|_| format!("all-rung root admission retained {rung_count} roots, not 8"))?;
        let admitted = Self {
            source,
            authority,
            rungs,
        };
        admitted.require_same("after complete root pre-admission")?;
        Ok(admitted)
    }

    fn require_same(&self, stage: &str) -> Result<(), String> {
        self.source
            .require_same(&format!("{stage} (stored source root)"))?;
        self.authority
            .require_same(&format!("{stage} (authority parent root)"))?;
        for (root, rung_name) in self.rungs.iter().zip(CANONICAL_RUNG_NAMES_V1) {
            root.require_same(&format!("{stage} ({rung_name} authority root)"))?;
        }
        Ok(())
    }

    fn overlaps(&self, other: &AdmittedDirectoryV1) -> bool {
        directory_trees_overlap_v1(&self.source, other)
            || directory_trees_overlap_v1(&self.authority, other)
            || self
                .rungs
                .iter()
                .any(|root| directory_trees_overlap_v1(root, other))
    }
}

struct AdmittedAllRungExecutionRootsV1 {
    one_minute: AdmittedDirectoryV1,
    two_minute: AdmittedDirectoryV1,
    three_minute: AdmittedDirectoryV1,
    five_minute: AdmittedDirectoryV1,
    ten_minute: AdmittedDirectoryV1,
    fifteen_minute: AdmittedDirectoryV1,
    thirty_minute: AdmittedDirectoryV1,
    sixty_minute: AdmittedDirectoryV1,
}

impl AdmittedAllRungExecutionRootsV1 {
    fn admit(
        request: &AllRungStoredExecutionV3Request<'_>,
        population_roots: &AdmittedAllRungRootsV1,
    ) -> Result<Self, String> {
        let admitted = Self {
            one_minute: admit_execution_rung_root_v1("1min", request.one_minute_root)?,
            two_minute: admit_execution_rung_root_v1("2min", request.two_minute_root)?,
            three_minute: admit_execution_rung_root_v1("3min", request.three_minute_root)?,
            five_minute: admit_execution_rung_root_v1("5min", request.five_minute_root)?,
            ten_minute: admit_execution_rung_root_v1("10min", request.ten_minute_root)?,
            fifteen_minute: admit_execution_rung_root_v1("15min", request.fifteen_minute_root)?,
            thirty_minute: admit_execution_rung_root_v1("30min", request.thirty_minute_root)?,
            sixty_minute: admit_execution_rung_root_v1("60min", request.sixty_minute_root)?,
        };
        let entries = [
            ("1min", &admitted.one_minute),
            ("2min", &admitted.two_minute),
            ("3min", &admitted.three_minute),
            ("5min", &admitted.five_minute),
            ("10min", &admitted.ten_minute),
            ("15min", &admitted.fifteen_minute),
            ("30min", &admitted.thirty_minute),
            ("60min", &admitted.sixty_minute),
        ];
        for (index, (rung_name, root)) in entries.iter().enumerate() {
            if population_roots.overlaps(root) {
                return Err(format!(
                    "all-rung {rung_name} Execution root overlaps the stored Population authority topology"
                ));
            }
            for (prior_name, prior) in entries.iter().take(index) {
                if directory_trees_overlap_v1(root, prior) {
                    return Err(format!(
                        "all-rung {rung_name} Execution root overlaps {prior_name} Execution root"
                    ));
                }
            }
        }
        admitted.require_same("after complete all-rung Execution root pre-admission")?;
        Ok(admitted)
    }

    fn require_same(&self, stage: &str) -> Result<(), String> {
        self.one_minute
            .require_same(&format!("{stage} (1min Execution root)"))?;
        self.two_minute
            .require_same(&format!("{stage} (2min Execution root)"))?;
        self.three_minute
            .require_same(&format!("{stage} (3min Execution root)"))?;
        self.five_minute
            .require_same(&format!("{stage} (5min Execution root)"))?;
        self.ten_minute
            .require_same(&format!("{stage} (10min Execution root)"))?;
        self.fifteen_minute
            .require_same(&format!("{stage} (15min Execution root)"))?;
        self.thirty_minute
            .require_same(&format!("{stage} (30min Execution root)"))?;
        self.sixty_minute
            .require_same(&format!("{stage} (60min Execution root)"))
    }

    fn overlaps(&self, other: &AdmittedDirectoryV1) -> bool {
        directory_trees_overlap_v1(&self.one_minute, other)
            || directory_trees_overlap_v1(&self.two_minute, other)
            || directory_trees_overlap_v1(&self.three_minute, other)
            || directory_trees_overlap_v1(&self.five_minute, other)
            || directory_trees_overlap_v1(&self.ten_minute, other)
            || directory_trees_overlap_v1(&self.fifteen_minute, other)
            || directory_trees_overlap_v1(&self.thirty_minute, other)
            || directory_trees_overlap_v1(&self.sixty_minute, other)
    }
}

struct AdmittedAllRungSelectionRootsV1 {
    one_minute: AdmittedDirectoryV1,
    two_minute: AdmittedDirectoryV1,
    three_minute: AdmittedDirectoryV1,
    five_minute: AdmittedDirectoryV1,
    ten_minute: AdmittedDirectoryV1,
    fifteen_minute: AdmittedDirectoryV1,
    thirty_minute: AdmittedDirectoryV1,
    sixty_minute: AdmittedDirectoryV1,
}

impl AdmittedAllRungSelectionRootsV1 {
    fn admit(
        specs: &AllRungSelectionRootSpecs<'_>,
        population_roots: &AdmittedAllRungRootsV1,
        execution_roots: &AdmittedAllRungExecutionRootsV1,
    ) -> Result<Self, String> {
        let admitted = Self {
            one_minute: admit_selection_rung_root_v1("1min", specs.one_minute)?,
            two_minute: admit_selection_rung_root_v1("2min", specs.two_minute)?,
            three_minute: admit_selection_rung_root_v1("3min", specs.three_minute)?,
            five_minute: admit_selection_rung_root_v1("5min", specs.five_minute)?,
            ten_minute: admit_selection_rung_root_v1("10min", specs.ten_minute)?,
            fifteen_minute: admit_selection_rung_root_v1("15min", specs.fifteen_minute)?,
            thirty_minute: admit_selection_rung_root_v1("30min", specs.thirty_minute)?,
            sixty_minute: admit_selection_rung_root_v1("60min", specs.sixty_minute)?,
        };
        let entries = [
            ("1min", &admitted.one_minute),
            ("2min", &admitted.two_minute),
            ("3min", &admitted.three_minute),
            ("5min", &admitted.five_minute),
            ("10min", &admitted.ten_minute),
            ("15min", &admitted.fifteen_minute),
            ("30min", &admitted.thirty_minute),
            ("60min", &admitted.sixty_minute),
        ];
        for (index, (rung_name, root)) in entries.iter().enumerate() {
            if population_roots.overlaps(root) || execution_roots.overlaps(root) {
                return Err(format!(
                    "all-rung {rung_name} Selection root overlaps retained Population/Execution topology"
                ));
            }
            for (prior_name, prior) in entries.iter().take(index) {
                if directory_trees_overlap_v1(root, prior) {
                    return Err(format!(
                        "all-rung {rung_name} Selection root overlaps {prior_name} Selection root"
                    ));
                }
            }
        }
        admitted.require_same("after complete all-rung Selection root pre-admission")?;
        Ok(admitted)
    }

    fn require_same(&self, stage: &str) -> Result<(), String> {
        self.one_minute
            .require_same(&format!("{stage} (1min Selection root)"))?;
        self.two_minute
            .require_same(&format!("{stage} (2min Selection root)"))?;
        self.three_minute
            .require_same(&format!("{stage} (3min Selection root)"))?;
        self.five_minute
            .require_same(&format!("{stage} (5min Selection root)"))?;
        self.ten_minute
            .require_same(&format!("{stage} (10min Selection root)"))?;
        self.fifteen_minute
            .require_same(&format!("{stage} (15min Selection root)"))?;
        self.thirty_minute
            .require_same(&format!("{stage} (30min Selection root)"))?;
        self.sixty_minute
            .require_same(&format!("{stage} (60min Selection root)"))
    }
}

fn admit_execution_rung_root_v1(
    expected_rung_name: &str,
    root: &Path,
) -> Result<AdmittedDirectoryV1, String> {
    let admitted = AdmittedDirectoryV1::admit(
        &format!("{expected_rung_name} Execution authority root"),
        root,
    )?;
    if admitted.path().file_name().and_then(|name| name.to_str()) != Some(expected_rung_name) {
        return Err(format!(
            "all-rung {expected_rung_name} Execution root {} has a foreign final path component",
            admitted.path().display()
        ));
    }
    Ok(admitted)
}

fn admit_selection_rung_root_v1(
    expected_rung_name: &str,
    root: &Path,
) -> Result<AdmittedDirectoryV1, String> {
    let admitted = AdmittedDirectoryV1::admit(
        &format!("{expected_rung_name} Selection authority root"),
        root,
    )?;
    if admitted.path().file_name().and_then(|name| name.to_str()) != Some(expected_rung_name) {
        return Err(format!(
            "all-rung {expected_rung_name} Selection root {} has a foreign final path component",
            admitted.path().display()
        ));
    }
    Ok(admitted)
}

fn directory_trees_overlap_v1(left: &AdmittedDirectoryV1, right: &AdmittedDirectoryV1) -> bool {
    left.aliases(right)
        || left.path().starts_with(right.path())
        || right.path().starts_with(left.path())
}

#[cfg(unix)]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the non-Unix implementation must return an explicit unsupported-target refusal"
)]
fn directory_identity_v1(metadata: &fs::Metadata) -> Result<[u64; 2], String> {
    use std::os::unix::fs::MetadataExt as _;

    Ok([metadata.dev(), metadata.ino()])
}

#[cfg(not(unix))]
fn directory_identity_v1(_metadata: &fs::Metadata) -> Result<[u64; 2], String> {
    Err("all-rung root admission requires a supported device/inode directory capability".to_owned())
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "controlled filesystem adversarial fixtures mutate exact topology and fail setup loudly"
)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let serial = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "brutex-all-rung-population-v5-{}-{serial}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("controlled all-rung temp root is creatable");
            Self(fs::canonicalize(path).expect("controlled all-rung temp root canonicalizes"))
        }

        fn path(&self) -> &Path {
            self.0.as_path()
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn controlled_roots() -> (TempDir, PathBuf, PathBuf) {
        let temp = TempDir::new();
        let source = temp.path().join("source");
        let authority = temp.path().join("authority");
        fs::create_dir(&source).expect("controlled source root is creatable");
        fs::create_dir(&authority).expect("controlled authority root is creatable");
        for rung_name in CANONICAL_RUNG_NAMES_V1 {
            fs::create_dir(authority.join(rung_name))
                .expect("controlled canonical rung root is creatable");
        }
        (temp, source, authority)
    }

    struct ControlledExecutionRoots {
        one_minute: PathBuf,
        two_minute: PathBuf,
        three_minute: PathBuf,
        five_minute: PathBuf,
        ten_minute: PathBuf,
        fifteen_minute: PathBuf,
        thirty_minute: PathBuf,
        sixty_minute: PathBuf,
    }

    fn controlled_execution_roots(temp: &TempDir) -> ControlledExecutionRoots {
        let parent = temp.path().join("execution");
        fs::create_dir(&parent).expect("controlled Execution parent is creatable");
        for rung_name in CANONICAL_RUNG_NAMES_V1 {
            fs::create_dir(parent.join(rung_name)).expect("controlled Execution rung is creatable");
        }
        ControlledExecutionRoots {
            one_minute: parent.join("1min"),
            two_minute: parent.join("2min"),
            three_minute: parent.join("3min"),
            five_minute: parent.join("5min"),
            ten_minute: parent.join("10min"),
            fifteen_minute: parent.join("15min"),
            thirty_minute: parent.join("30min"),
            sixty_minute: parent.join("60min"),
        }
    }

    struct ControlledSelectionRoots {
        one_minute: PathBuf,
        two_minute: PathBuf,
        three_minute: PathBuf,
        five_minute: PathBuf,
        ten_minute: PathBuf,
        fifteen_minute: PathBuf,
        thirty_minute: PathBuf,
        sixty_minute: PathBuf,
    }

    fn controlled_selection_roots(temp: &TempDir) -> ControlledSelectionRoots {
        let parent = temp.path().join("selection");
        fs::create_dir(&parent).expect("controlled Selection parent is creatable");
        for rung_name in CANONICAL_RUNG_NAMES_V1 {
            fs::create_dir(parent.join(rung_name)).expect("controlled Selection rung is creatable");
        }
        ControlledSelectionRoots {
            one_minute: parent.join("1min"),
            two_minute: parent.join("2min"),
            three_minute: parent.join("3min"),
            five_minute: parent.join("5min"),
            ten_minute: parent.join("10min"),
            fifteen_minute: parent.join("15min"),
            thirty_minute: parent.join("30min"),
            sixty_minute: parent.join("60min"),
        }
    }

    fn selection_specs(roots: &ControlledSelectionRoots) -> AllRungSelectionRootSpecs<'_> {
        AllRungSelectionRootSpecs {
            one_minute: &roots.one_minute,
            two_minute: &roots.two_minute,
            three_minute: &roots.three_minute,
            five_minute: &roots.five_minute,
            ten_minute: &roots.ten_minute,
            fifteen_minute: &roots.fifteen_minute,
            thirty_minute: &roots.thirty_minute,
            sixty_minute: &roots.sixty_minute,
        }
    }

    fn execution_bounds(scale: u64) -> ExecutionV3Bounds {
        let parameter_records = 4_u64.checked_mul(scale).expect("parameter test bound");
        let percentile_records = 24_u64.checked_mul(scale).expect("percentile test bound");
        let disposition_records = 16_u64.checked_mul(scale).expect("disposition test bound");
        let completion_records = scale;
        ExecutionV3Bounds::new(
            crate::execution_v3::ExecutionV3FileBound::new(
                parameter_records,
                parameter_records
                    .checked_mul(crate::execution_v3::EXECUTION_V3_PARAMETER_BYTES as u64)
                    .expect("parameter byte test bound"),
                crate::execution_v3::EXECUTION_V3_PARAMETER_BYTES,
                "all-rung test parameter",
            )
            .expect("parameter bound is valid"),
            crate::execution_v3::ExecutionV3FileBound::new(
                percentile_records,
                percentile_records
                    .checked_mul(crate::execution_v3::EXECUTION_V3_PERCENTILE_BYTES as u64)
                    .expect("percentile byte test bound"),
                crate::execution_v3::EXECUTION_V3_PERCENTILE_BYTES,
                "all-rung test percentile",
            )
            .expect("percentile bound is valid"),
            crate::execution_v3::ExecutionV3FileBound::new(
                disposition_records,
                disposition_records
                    .checked_mul(crate::execution_v3::EXECUTION_V3_DISPOSITION_BYTES as u64)
                    .expect("disposition byte test bound"),
                crate::execution_v3::EXECUTION_V3_DISPOSITION_BYTES,
                "all-rung test disposition",
            )
            .expect("disposition bound is valid"),
            crate::execution_v3::ExecutionV3FileBound::new(
                completion_records,
                completion_records
                    .checked_mul(crate::execution_v3::EXECUTION_V3_COMPLETION_BYTES as u64)
                    .expect("Completion byte test bound"),
                crate::execution_v3::EXECUTION_V3_COMPLETION_BYTES,
                "all-rung test Completion",
            )
            .expect("Completion bound is valid"),
            disposition_records,
            percentile_records,
        )
        .expect("all-rung Execution test bounds are valid")
    }

    fn execution_request(
        roots: &ControlledExecutionRoots,
        bounds: ExecutionV3Bounds,
    ) -> AllRungStoredExecutionV3Request<'_> {
        AllRungStoredExecutionV3Request {
            one_minute_root: &roots.one_minute,
            one_minute_bounds: bounds,
            two_minute_root: &roots.two_minute,
            two_minute_bounds: bounds,
            three_minute_root: &roots.three_minute,
            three_minute_bounds: bounds,
            five_minute_root: &roots.five_minute,
            five_minute_bounds: bounds,
            ten_minute_root: &roots.ten_minute,
            ten_minute_bounds: bounds,
            fifteen_minute_root: &roots.fifteen_minute,
            fifteen_minute_bounds: bounds,
            thirty_minute_root: &roots.thirty_minute,
            thirty_minute_bounds: bounds,
            sixty_minute_root: &roots.sixty_minute,
            sixty_minute_bounds: bounds,
        }
    }

    #[test]
    fn canonical_topology_is_exact_nifty_first_banknifty_second_and_eight_rungs() {
        assert_eq!(
            CANDIDATE_SIGNAL_RUNGS_SECONDS_V1,
            [60, 120, 180, 300, 600, 900, 1_800, 3_600]
        );
        assert_eq!(
            CANONICAL_RUNG_NAMES_V1,
            [
                "1min", "2min", "3min", "5min", "10min", "15min", "30min", "60min"
            ]
        );
        assert_eq!(CANONICAL_UNDERLYINGS_V1, ["NIFTY", "BANKNIFTY"]);
    }

    #[test]
    fn row_topology_authenticates_receipt_counts_and_canonical_order() {
        let valid = [
            (60, AdmissionV3Family::Nifty, InstrumentFamilyV1::Nifty),
            (
                60,
                AdmissionV3Family::BankNifty,
                InstrumentFamilyV1::BankNifty,
            ),
        ];
        assert!(require_rung_row_topology_v1("1min", 60, 2, 1, 1, valid).is_ok());
        assert!(require_rung_row_topology_v1("1min", 60, 0, 0, 0, []).is_ok());
        assert!(require_rung_row_topology_v1("1min", 60, 2, 2, 0, valid).is_err());
        assert!(
            require_rung_row_topology_v1("1min", 60, 1, 1, 0, valid[..1].iter().copied()).is_ok()
        );
        assert!(
            require_rung_row_topology_v1("1min", 60, 1, 0, 1, valid[1..].iter().copied()).is_ok()
        );
        assert!(
            require_rung_row_topology_v1("1min", 60, 2, 1, 1, valid.into_iter().rev()).is_err()
        );
        assert!(
            require_rung_row_topology_v1(
                "1min",
                60,
                2,
                1,
                1,
                [
                    (120, AdmissionV3Family::Nifty, InstrumentFamilyV1::Nifty,),
                    valid[1],
                ],
            )
            .is_err()
        );
        assert!(
            require_rung_row_topology_v1(
                "1min",
                60,
                2,
                1,
                1,
                [
                    (60, AdmissionV3Family::Nifty, InstrumentFamilyV1::BankNifty,),
                    valid[1],
                ],
            )
            .is_err()
        );
    }

    #[test]
    fn complete_root_preflight_is_exact_and_idempotent() {
        let (_temp, source, authority) = controlled_roots();
        let first = AdmittedAllRungRootsV1::admit(&source, &authority)
            .expect("first exact topology admits");
        let second = AdmittedAllRungRootsV1::admit(&source, &authority)
            .expect("same exact topology re-admits");
        first.require_same("first controlled recheck").unwrap();
        second.require_same("second controlled recheck").unwrap();
        for (index, rung_name) in CANONICAL_RUNG_NAMES_V1.iter().enumerate() {
            assert_eq!(first.rungs[index].path(), authority.join(rung_name));
            assert_eq!(second.rungs[index].path(), first.rungs[index].path());
        }
    }

    #[test]
    fn missing_rung_refuses_before_any_authority_file_is_written() {
        let (_temp, source, authority) = controlled_roots();
        fs::remove_dir(authority.join("30min")).expect("controlled rung removal succeeds");
        let refusal = AdmittedAllRungRootsV1::admit(&source, &authority)
            .err()
            .expect("missing canonical rung must refuse");
        assert!(refusal.contains("30min authority root"));
        assert!(fs::read_dir(&source).unwrap().next().is_none());
        for rung_name in CANONICAL_RUNG_NAMES_V1 {
            let path = authority.join(rung_name);
            if path.is_dir() {
                assert!(fs::read_dir(path).unwrap().next().is_none());
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_alias_rung_refuses_before_any_authority_file_is_written() {
        use std::os::unix::fs::symlink;

        let (_temp, source, authority) = controlled_roots();
        let alias = authority.join("2min");
        fs::remove_dir(&alias).expect("controlled alias slot removal succeeds");
        symlink(authority.join("1min"), &alias).expect("controlled rung symlink is creatable");
        let refusal = AdmittedAllRungRootsV1::admit(&source, &authority)
            .err()
            .expect("symlinked rung must refuse");
        assert!(refusal.contains("symlinked 2min authority root"));
        assert!(fs::read_dir(&source).unwrap().next().is_none());
        assert!(
            fs::read_dir(authority.join("1min"))
                .unwrap()
                .next()
                .is_none()
        );
    }

    #[test]
    fn execution_roots_and_bounds_are_explicit_canonical_and_rechecked() {
        let (temp, source, authority) = controlled_roots();
        let population_roots =
            AdmittedAllRungRootsV1::admit(&source, &authority).expect("Population roots admit");
        let roots = controlled_execution_roots(&temp);
        let mut request = execution_request(&roots, execution_bounds(1));
        request.two_minute_bounds = execution_bounds(2);
        request.three_minute_bounds = execution_bounds(3);
        request.five_minute_bounds = execution_bounds(4);
        request.ten_minute_bounds = execution_bounds(5);
        request.fifteen_minute_bounds = execution_bounds(6);
        request.thirty_minute_bounds = execution_bounds(7);
        request.sixty_minute_bounds = execution_bounds(8);
        let admitted = AdmittedAllRungExecutionRootsV1::admit(&request, &population_roots)
            .expect("canonical Execution roots admit");
        admitted
            .require_same("controlled canonical Execution root recheck")
            .expect("held Execution roots remain exact");
        assert_eq!(admitted.one_minute.path(), roots.one_minute);
        assert_eq!(admitted.sixty_minute.path(), roots.sixty_minute);
        let retained = CanonicalExecutionV3BoundsV1::from(&request);
        for (scale, actual) in [
            retained.one_minute,
            retained.two_minute,
            retained.three_minute,
            retained.five_minute,
            retained.ten_minute,
            retained.fifteen_minute,
            retained.thirty_minute,
            retained.sixty_minute,
        ]
        .into_iter()
        .enumerate()
        {
            require_execution_bounds_v1(
                "controlled",
                execution_bounds(u64::try_from(scale + 1).expect("test scale fits u64")),
                actual,
            )
            .expect("each named bound is retained exactly");
        }
    }

    #[test]
    fn reordered_missing_duplicate_nested_and_population_execution_roots_refuse() {
        let (temp, source, authority) = controlled_roots();
        let population_roots =
            AdmittedAllRungRootsV1::admit(&source, &authority).expect("Population roots admit");
        let roots = controlled_execution_roots(&temp);
        let bounds = execution_bounds(1);

        let mut reordered = execution_request(&roots, bounds);
        reordered.one_minute_root = &roots.two_minute;
        reordered.two_minute_root = &roots.one_minute;
        assert!(
            AdmittedAllRungExecutionRootsV1::admit(&reordered, &population_roots)
                .err()
                .expect("reordered roots refuse")
                .contains("foreign final path component")
        );

        let mut duplicate = execution_request(&roots, bounds);
        duplicate.two_minute_root = &roots.one_minute;
        assert!(AdmittedAllRungExecutionRootsV1::admit(&duplicate, &population_roots).is_err());

        fs::remove_dir(&roots.thirty_minute).expect("controlled missing root removal succeeds");
        assert!(
            AdmittedAllRungExecutionRootsV1::admit(
                &execution_request(&roots, bounds),
                &population_roots,
            )
            .err()
            .expect("missing root refuses")
            .contains("30min Execution authority root")
        );
        fs::create_dir(&roots.thirty_minute).expect("controlled missing root restores");

        let nested = roots.one_minute.join("2min");
        fs::create_dir(&nested).expect("controlled nested root is creatable");
        let mut overlapping = execution_request(&roots, bounds);
        overlapping.two_minute_root = &nested;
        assert!(
            AdmittedAllRungExecutionRootsV1::admit(&overlapping, &population_roots)
                .err()
                .expect("nested roots refuse")
                .contains("overlaps 1min")
        );

        let mut population_alias = execution_request(&roots, bounds);
        let population_one_minute = authority.join("1min");
        population_alias.one_minute_root = &population_one_minute;
        assert!(
            AdmittedAllRungExecutionRootsV1::admit(&population_alias, &population_roots)
                .err()
                .expect("Population-overlapping root refuses")
                .contains("Population authority topology")
        );
    }

    #[cfg(unix)]
    #[test]
    fn retained_execution_root_symlink_and_path_replacement_refuse() {
        use std::os::unix::fs::symlink;

        let (temp, source, authority) = controlled_roots();
        let population_roots =
            AdmittedAllRungRootsV1::admit(&source, &authority).expect("Population roots admit");
        let roots = controlled_execution_roots(&temp);
        let request = execution_request(&roots, execution_bounds(1));
        let admitted = AdmittedAllRungExecutionRootsV1::admit(&request, &population_roots)
            .expect("Execution roots admit");

        let moved = roots.ten_minute.with_extension("moved");
        fs::rename(&roots.ten_minute, &moved).expect("controlled root moves");
        fs::create_dir(&roots.ten_minute).expect("controlled replacement is creatable");
        assert!(admitted.require_same("after path replacement").is_err());
        fs::remove_dir(&roots.ten_minute).expect("replacement root removes");
        fs::rename(&moved, &roots.ten_minute).expect("original root restores");
        admitted
            .require_same("after exact root restoration")
            .expect("held root and original pathname reunite");

        fs::remove_dir(&roots.fifteen_minute).expect("controlled symlink slot removes");
        symlink(&roots.ten_minute, &roots.fifteen_minute)
            .expect("controlled replacement symlink is creatable");
        assert!(admitted.require_same("after symlink replacement").is_err());
    }

    #[test]
    fn foreign_bounds_population_and_disposition_topology_refuse() {
        let first_bounds = execution_bounds(1);
        let second_bounds = execution_bounds(2);
        assert!(require_execution_bounds_v1("1min", first_bounds, first_bounds).is_ok());
        assert!(require_execution_bounds_v1("1min", first_bounds, second_bounds).is_err());

        let population_id = [7_u8; 32];
        assert!(
            require_execution_population_identity_v1("1min", population_id, population_id).is_ok()
        );
        assert!(
            require_execution_population_identity_v1("1min", population_id, [8_u8; 32]).is_err()
        );

        let nifty_row = [9_u8; 32];
        let banknifty_row = [10_u8; 32];
        let valid = [
            (
                0,
                60,
                population_id,
                nifty_row,
                nifty_row,
                ExecutionV3Family::Nifty,
                AdmissionV3Family::Nifty,
            ),
            (
                1,
                60,
                population_id,
                banknifty_row,
                banknifty_row,
                ExecutionV3Family::BankNifty,
                AdmissionV3Family::BankNifty,
            ),
        ];
        assert!(
            require_execution_disposition_topology_v1("1min", 60, population_id, 2, valid,).is_ok()
        );
        assert!(
            require_execution_disposition_topology_v1(
                "1min",
                60,
                population_id,
                2,
                valid[..1].iter().copied(),
            )
            .is_err()
        );
        let mut reordered = valid;
        reordered.swap(0, 1);
        assert!(
            require_execution_disposition_topology_v1("1min", 60, population_id, 2, reordered,)
                .is_err()
        );
        let mut duplicate = valid;
        duplicate[1].0 = 0;
        assert!(
            require_execution_disposition_topology_v1("1min", 60, population_id, 2, duplicate,)
                .is_err()
        );
        let mut foreign = valid;
        foreign[0].2 = [11_u8; 32];
        assert!(
            require_execution_disposition_topology_v1("1min", 60, population_id, 2, foreign,)
                .is_err()
        );
        let mut foreign_row = valid;
        foreign_row[0].3 = [12_u8; 32];
        assert!(
            require_execution_disposition_topology_v1("1min", 60, population_id, 2, foreign_row,)
                .is_err()
        );
        let mut foreign_rung = valid;
        foreign_rung[0].1 = 120;
        assert!(
            require_execution_disposition_topology_v1("1min", 60, population_id, 2, foreign_rung,)
                .is_err()
        );
    }

    #[test]
    fn all_rung_selection_roots_are_named_disjoint_from_every_upstream_root_and_pre_admitted() {
        let (temp, source, authority) = controlled_roots();
        let population_roots =
            AdmittedAllRungRootsV1::admit(&source, &authority).expect("Population roots admit");
        let execution_paths = controlled_execution_roots(&temp);
        let execution_roots = AdmittedAllRungExecutionRootsV1::admit(
            &execution_request(&execution_paths, execution_bounds(1)),
            &population_roots,
        )
        .expect("Execution roots admit");
        let selection_paths = controlled_selection_roots(&temp);
        let admitted = AdmittedAllRungSelectionRootsV1::admit(
            &selection_specs(&selection_paths),
            &population_roots,
            &execution_roots,
        )
        .expect("canonical Selection roots admit");
        admitted
            .require_same("controlled Selection root recheck")
            .expect("held Selection roots remain exact");

        let mut swapped = selection_specs(&selection_paths);
        swapped.one_minute = &selection_paths.two_minute;
        assert!(
            AdmittedAllRungSelectionRootsV1::admit(&swapped, &population_roots, &execution_roots,)
                .is_err()
        );

        let mut duplicate = selection_specs(&selection_paths);
        duplicate.two_minute = &selection_paths.one_minute;
        assert!(
            AdmittedAllRungSelectionRootsV1::admit(
                &duplicate,
                &population_roots,
                &execution_roots,
            )
            .is_err()
        );

        let nested = selection_paths.five_minute.join("10min");
        fs::create_dir(&nested).expect("controlled nested Selection root is creatable");
        let mut nested_specs = selection_specs(&selection_paths);
        nested_specs.ten_minute = &nested;
        assert!(
            AdmittedAllRungSelectionRootsV1::admit(
                &nested_specs,
                &population_roots,
                &execution_roots,
            )
            .is_err()
        );

        let population_one_minute = authority.join("1min");
        let mut population_overlap = selection_specs(&selection_paths);
        population_overlap.one_minute = &population_one_minute;
        assert!(
            AdmittedAllRungSelectionRootsV1::admit(
                &population_overlap,
                &population_roots,
                &execution_roots,
            )
            .is_err()
        );

        let mut execution_overlap = selection_specs(&selection_paths);
        execution_overlap.one_minute = &execution_paths.one_minute;
        assert!(
            AdmittedAllRungSelectionRootsV1::admit(
                &execution_overlap,
                &population_roots,
                &execution_roots,
            )
            .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn all_rung_selection_retained_root_symlink_and_path_replacement_refuse() {
        use std::os::unix::fs::symlink;

        let (temp, source, authority) = controlled_roots();
        let population_roots =
            AdmittedAllRungRootsV1::admit(&source, &authority).expect("Population roots admit");
        let execution_paths = controlled_execution_roots(&temp);
        let execution_roots = AdmittedAllRungExecutionRootsV1::admit(
            &execution_request(&execution_paths, execution_bounds(1)),
            &population_roots,
        )
        .expect("Execution roots admit");
        let selection_paths = controlled_selection_roots(&temp);
        let admitted = AdmittedAllRungSelectionRootsV1::admit(
            &selection_specs(&selection_paths),
            &population_roots,
            &execution_roots,
        )
        .expect("Selection roots admit");

        let moved = selection_paths.ten_minute.with_extension("moved");
        fs::rename(&selection_paths.ten_minute, &moved).expect("controlled root moves");
        fs::create_dir(&selection_paths.ten_minute).expect("controlled replacement is creatable");
        assert!(admitted.require_same("after path replacement").is_err());
        fs::remove_dir(&selection_paths.ten_minute).expect("replacement root removes");
        fs::rename(&moved, &selection_paths.ten_minute).expect("original root restores");
        admitted
            .require_same("after exact root restoration")
            .expect("held root and original pathname reunite");

        fs::remove_dir(&selection_paths.fifteen_minute).expect("controlled symlink slot removes");
        symlink(&selection_paths.ten_minute, &selection_paths.fifteen_minute)
            .expect("controlled Selection symlink is creatable");
        assert!(admitted.require_same("after symlink replacement").is_err());
    }
}
