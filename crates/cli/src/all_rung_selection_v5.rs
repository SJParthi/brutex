//! Canonical eight-rung Execution V3-to-Selection V5 coordinator.
//!
//! The coordinator consumes the opaque all-rung Execution capability once,
//! admits all eight pre-existing Selection roots before the first append, and
//! moves each named source through the sole one-rung Selection V5 door in the
//! literal 60, 120, 180, 300, 600, 900, 1,800 and 3,600-second order. There is
//! no caller rung list, authority array, detached receipt or caller-authored
//! winner fact.
//!
//! Each retained authority reproduces its complete Top-25 from the live
//! Execution source, proves Top-10 is the exact prefix, and exact-joins every
//! winner back to its durable selected-exit digest. The fixed eight-call
//! dispatch is O(1) in rung count. Ranking, source authentication, allocation,
//! persistence, synchronization and filesystem latency remain input- or
//! system-dependent and are not O(1).

use std::collections::HashSet;
use std::path::Path;

use runner::topn::RankingPolicyV1;

use crate::all_rung_population_v5::{
    AllRungExecutionV3SelectionSources, AllRungSelectionRootSpecs, AllRungSelectionTopologyToken,
    CommittedStoredAllRungExecutionV3,
};
use crate::execution_v3::{CommittedStoredExecutionV3, ExecutionV3StructuralReceipt};
use crate::selection_v5::{
    CommittedStoredSelectionV5, SelectionV5Bounds, SelectionV5RowRecord,
    SelectionV5StructuralReceipt, SelectionV5SuccessorWinnerV1, commit_stored_selection_v5,
};

const RUNG_COUNT: usize = 8;
const MAX_WINNERS: u64 = 25;
const MAX_WINNERS_USIZE: usize = 25;
const TOP_TEN: usize = 10;
const TOP_TEN_U64: u64 = 10;
const CANONICAL_RUNGS: [u32; RUNG_COUNT] = [60, 120, 180, 300, 600, 900, 1_800, 3_600];
const CANONICAL_RUNG_NAMES: [&str; RUNG_COUNT] = [
    "1min", "2min", "3min", "5min", "10min", "15min", "30min", "60min",
];

/// Explicit roots, bounds and ranking policy for all eight Selection V5
/// authorities. Every root must already exist with its exact canonical name.
#[derive(Clone, Copy)]
pub(crate) struct AllRungSelectionV5Request<'a> {
    /// Existing one-minute Selection root.
    pub(crate) one_minute_root: &'a Path,
    /// Explicit one-minute Selection bounds.
    pub(crate) one_minute_bounds: SelectionV5Bounds,
    /// Existing two-minute Selection root.
    pub(crate) two_minute_root: &'a Path,
    /// Explicit two-minute Selection bounds.
    pub(crate) two_minute_bounds: SelectionV5Bounds,
    /// Existing three-minute Selection root.
    pub(crate) three_minute_root: &'a Path,
    /// Explicit three-minute Selection bounds.
    pub(crate) three_minute_bounds: SelectionV5Bounds,
    /// Existing five-minute Selection root.
    pub(crate) five_minute_root: &'a Path,
    /// Explicit five-minute Selection bounds.
    pub(crate) five_minute_bounds: SelectionV5Bounds,
    /// Existing ten-minute Selection root.
    pub(crate) ten_minute_root: &'a Path,
    /// Explicit ten-minute Selection bounds.
    pub(crate) ten_minute_bounds: SelectionV5Bounds,
    /// Existing fifteen-minute Selection root.
    pub(crate) fifteen_minute_root: &'a Path,
    /// Explicit fifteen-minute Selection bounds.
    pub(crate) fifteen_minute_bounds: SelectionV5Bounds,
    /// Existing thirty-minute Selection root.
    pub(crate) thirty_minute_root: &'a Path,
    /// Explicit thirty-minute Selection bounds.
    pub(crate) thirty_minute_bounds: SelectionV5Bounds,
    /// Existing sixty-minute Selection root.
    pub(crate) sixty_minute_root: &'a Path,
    /// Explicit sixty-minute Selection bounds.
    pub(crate) sixty_minute_bounds: SelectionV5Bounds,
    /// Exact Runner ranking policy shared by this one all-rung cohort.
    pub(crate) ranking_policy: RankingPolicyV1,
}

#[derive(Clone, Copy)]
struct CanonicalSelectionV5Bounds {
    one_minute: SelectionV5Bounds,
    two_minute: SelectionV5Bounds,
    three_minute: SelectionV5Bounds,
    five_minute: SelectionV5Bounds,
    ten_minute: SelectionV5Bounds,
    fifteen_minute: SelectionV5Bounds,
    thirty_minute: SelectionV5Bounds,
    sixty_minute: SelectionV5Bounds,
}

impl From<&AllRungSelectionV5Request<'_>> for CanonicalSelectionV5Bounds {
    fn from(value: &AllRungSelectionV5Request<'_>) -> Self {
        Self {
            one_minute: value.one_minute_bounds,
            two_minute: value.two_minute_bounds,
            three_minute: value.three_minute_bounds,
            five_minute: value.five_minute_bounds,
            ten_minute: value.ten_minute_bounds,
            fifteen_minute: value.fifteen_minute_bounds,
            thirty_minute: value.thirty_minute_bounds,
            sixty_minute: value.sixty_minute_bounds,
        }
    }
}

struct CanonicalStoredSelectionV5Authorities {
    one_minute: CommittedStoredSelectionV5,
    two_minute: CommittedStoredSelectionV5,
    three_minute: CommittedStoredSelectionV5,
    five_minute: CommittedStoredSelectionV5,
    ten_minute: CommittedStoredSelectionV5,
    fifteen_minute: CommittedStoredSelectionV5,
    thirty_minute: CommittedStoredSelectionV5,
    sixty_minute: CommittedStoredSelectionV5,
}

#[derive(Clone, Copy)]
struct RungSelectionV5Receipts {
    execution: ExecutionV3StructuralReceipt,
    selection: SelectionV5StructuralReceipt,
}

#[derive(Clone, Copy)]
struct CanonicalSelectionV5Receipts {
    one_minute: RungSelectionV5Receipts,
    two_minute: RungSelectionV5Receipts,
    three_minute: RungSelectionV5Receipts,
    five_minute: RungSelectionV5Receipts,
    ten_minute: RungSelectionV5Receipts,
    fifteen_minute: RungSelectionV5Receipts,
    thirty_minute: RungSelectionV5Receipts,
    sixty_minute: RungSelectionV5Receipts,
}

/// Opaque canonical eight-rung Selection V5 authority.
///
/// The eight live one-rung capabilities, their physical roots, exact bounds,
/// ranking policy and paired source receipts remain named and private.
pub(crate) struct CommittedStoredAllRungSelectionV5 {
    selections: CanonicalStoredSelectionV5Authorities,
    topology: AllRungSelectionTopologyToken,
    bounds: CanonicalSelectionV5Bounds,
    ranking_policy: RankingPolicyV1,
    receipts: CanonicalSelectionV5Receipts,
}

impl CommittedStoredAllRungSelectionV5 {
    /// Number of rung blocks written rather than byte-exactly reused.
    #[must_use]
    pub(crate) fn written_rung_count(&self) -> usize {
        usize::from(self.selections.one_minute.was_written())
            + usize::from(self.selections.two_minute.was_written())
            + usize::from(self.selections.three_minute.was_written())
            + usize::from(self.selections.five_minute.was_written())
            + usize::from(self.selections.ten_minute.was_written())
            + usize::from(self.selections.fifteen_minute.was_written())
            + usize::from(self.selections.thirty_minute.was_written())
            + usize::from(self.selections.sixty_minute.was_written())
    }

    /// Reauthenticates all held roots, bounds, source receipts, full Top-25
    /// sets, exact Top-10 prefixes and winner-to-disposition joins.
    pub(crate) fn require_live_topology(&mut self) -> Result<(), String> {
        self.topology
            .require_same("before retained all-rung Selection V5 reauthentication")?;
        let observations = CanonicalSelectionObservations {
            one_minute: require_committed_selection_rung(
                &mut self.selections.one_minute,
                &self.topology,
                self.bounds.one_minute,
                self.receipts.one_minute,
                self.ranking_policy,
                60,
                "1min",
            )?,
            two_minute: require_committed_selection_rung(
                &mut self.selections.two_minute,
                &self.topology,
                self.bounds.two_minute,
                self.receipts.two_minute,
                self.ranking_policy,
                120,
                "2min",
            )?,
            three_minute: require_committed_selection_rung(
                &mut self.selections.three_minute,
                &self.topology,
                self.bounds.three_minute,
                self.receipts.three_minute,
                self.ranking_policy,
                180,
                "3min",
            )?,
            five_minute: require_committed_selection_rung(
                &mut self.selections.five_minute,
                &self.topology,
                self.bounds.five_minute,
                self.receipts.five_minute,
                self.ranking_policy,
                300,
                "5min",
            )?,
            ten_minute: require_committed_selection_rung(
                &mut self.selections.ten_minute,
                &self.topology,
                self.bounds.ten_minute,
                self.receipts.ten_minute,
                self.ranking_policy,
                600,
                "10min",
            )?,
            fifteen_minute: require_committed_selection_rung(
                &mut self.selections.fifteen_minute,
                &self.topology,
                self.bounds.fifteen_minute,
                self.receipts.fifteen_minute,
                self.ranking_policy,
                900,
                "15min",
            )?,
            thirty_minute: require_committed_selection_rung(
                &mut self.selections.thirty_minute,
                &self.topology,
                self.bounds.thirty_minute,
                self.receipts.thirty_minute,
                self.ranking_policy,
                1_800,
                "30min",
            )?,
            sixty_minute: require_committed_selection_rung(
                &mut self.selections.sixty_minute,
                &self.topology,
                self.bounds.sixty_minute,
                self.receipts.sixty_minute,
                self.ranking_policy,
                3_600,
                "60min",
            )?,
        };
        require_cross_rung_observations(&observations)?;
        self.topology
            .require_same("after retained all-rung Selection V5 reauthentication")
    }

    /// Performs one final complete reauthentication and consumes this wrapper
    /// into the sole canonical Global Replay successor visit capability.
    pub(crate) fn into_successor_set(mut self) -> Result<AllRungSelectionV5SuccessorSetV1, String> {
        self.require_live_topology()?;
        let Self {
            selections,
            topology,
            bounds: _,
            ranking_policy: _,
            receipts: _,
        } = self;
        topology.require_same("while moving all-rung Selection V5 into its successor")?;
        let CanonicalStoredSelectionV5Authorities {
            one_minute,
            two_minute,
            three_minute,
            five_minute,
            ten_minute,
            fifteen_minute,
            thirty_minute,
            sixty_minute,
        } = selections;
        Ok(AllRungSelectionV5SuccessorSetV1 {
            one_minute,
            two_minute,
            three_minute,
            five_minute,
            ten_minute,
            fifteen_minute,
            thirty_minute,
            sixty_minute,
            topology,
        })
    }
}

/// One-use named successor capability for the canonical 8×25 replay cohort.
///
/// It exposes no root, receipt, collection or caller-writable winner fact.
/// The only operation authenticates every rung first, then visits exactly 200
/// winners in rung-major, rank-major canonical order.
pub(crate) struct AllRungSelectionV5SuccessorSetV1 {
    one_minute: CommittedStoredSelectionV5,
    two_minute: CommittedStoredSelectionV5,
    three_minute: CommittedStoredSelectionV5,
    five_minute: CommittedStoredSelectionV5,
    ten_minute: CommittedStoredSelectionV5,
    fifteen_minute: CommittedStoredSelectionV5,
    thirty_minute: CommittedStoredSelectionV5,
    sixty_minute: CommittedStoredSelectionV5,
    topology: AllRungSelectionTopologyToken,
}

impl AllRungSelectionV5SuccessorSetV1 {
    /// Reauthenticates all eight complete Top-25 sets before the first visit,
    /// then yields exactly 200 opaque winner projections in literal
    /// 60→3,600-second and rank 1→25 order.
    ///
    /// # Errors
    ///
    /// Refuses before the first callback when any topology, rung, source,
    /// winner, selected-exit join or exact 25-row count differs. A callback
    /// error halts the already-authorized visit and is returned with its rung
    /// and rank context; the consumed capability cannot be retried.
    pub(crate) fn visit_canonical<F>(mut self, mut visit: F) -> Result<(), String>
    where
        F: FnMut(SelectionV5SuccessorWinnerV1) -> Result<(), String>,
    {
        self.topology
            .require_same("before all-rung Selection V5 successor preflight")?;
        let one_minute = preflight_successor_rung(&mut self.one_minute, 60, "1min")?;
        let two_minute = preflight_successor_rung(&mut self.two_minute, 120, "2min")?;
        let three_minute = preflight_successor_rung(&mut self.three_minute, 180, "3min")?;
        let five_minute = preflight_successor_rung(&mut self.five_minute, 300, "5min")?;
        let ten_minute = preflight_successor_rung(&mut self.ten_minute, 600, "10min")?;
        let fifteen_minute = preflight_successor_rung(&mut self.fifteen_minute, 900, "15min")?;
        let thirty_minute = preflight_successor_rung(&mut self.thirty_minute, 1_800, "30min")?;
        let sixty_minute = preflight_successor_rung(&mut self.sixty_minute, 3_600, "60min")?;
        self.topology
            .require_same("after all-rung Selection V5 successor preflight")?;

        visit_successor_rung("1min", one_minute, &mut visit)?;
        visit_successor_rung("2min", two_minute, &mut visit)?;
        visit_successor_rung("3min", three_minute, &mut visit)?;
        visit_successor_rung("5min", five_minute, &mut visit)?;
        visit_successor_rung("10min", ten_minute, &mut visit)?;
        visit_successor_rung("15min", fifteen_minute, &mut visit)?;
        visit_successor_rung("30min", thirty_minute, &mut visit)?;
        visit_successor_rung("60min", sixty_minute, &mut visit)
    }
}

fn preflight_successor_rung(
    selection: &mut CommittedStoredSelectionV5,
    expected_rung: u32,
    rung_name: &str,
) -> Result<Vec<SelectionV5SuccessorWinnerV1>, String> {
    let winners = selection.successor_winners()?;
    if winners.len() != MAX_WINNERS_USIZE {
        return Err(format!(
            "all-rung {rung_name} successor requires exactly 25 winners, observed {}",
            winners.len()
        ));
    }
    for (index, winner) in winners.iter().enumerate() {
        let row = winner.row();
        let expected_rank = u32::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| format!("all-rung {rung_name} successor rank overflowed"))?;
        if row.rung_seconds() != expected_rung
            || row.rank() != expected_rank
            || winner
                .selected_exit_digest()
                .is_none_or(|digest| digest.iter().all(|byte| *byte == 0))
        {
            return Err(format!(
                "all-rung {rung_name} successor winner {index} has a foreign rung, rank or selected-exit identity"
            ));
        }
    }
    Ok(winners)
}

fn visit_successor_rung<F>(
    rung_name: &str,
    winners: Vec<SelectionV5SuccessorWinnerV1>,
    visit: &mut F,
) -> Result<(), String>
where
    F: FnMut(SelectionV5SuccessorWinnerV1) -> Result<(), String>,
{
    for winner in winners {
        let rank = winner.row().rank();
        visit(winner).map_err(|why| {
            format!("all-rung {rung_name} rank {rank} successor visit refused: {why}")
        })?;
    }
    Ok(())
}

/// Commits all eight named Execution V3 sources through Selection V5 in the
/// one canonical rung order.
///
/// All Selection roots are admitted before the first write. A later refusal
/// does not roll back an already synchronized receipt-last prefix; an exact
/// reconstructed Execution source may retry and reuse those bytes.
///
/// # Errors
///
/// Refuses missing, noncanonical, symlinked, aliased, nested, replaced or
/// removed Selection roots; a foreign rung, bound, source receipt, winner,
/// rank, Top-10 prefix or durable selected-exit join; duplicate cross-rung
/// identities; stale retained Execution/Population state; or any one-rung
/// Selection persistence/reopen refusal.
///
/// # Cost
///
/// Dispatch is fixed at eight named calls. Source authentication, ranking,
/// persistence and retained winner proofs are linear in bounded source bytes
/// or rows and depend on allocation, hashing, locks and device latency.
#[allow(
    clippy::too_many_lines,
    reason = "the literal eight-rung move order is the authority and must remain reviewable"
)]
pub(crate) fn commit_all_rung_stored_selection_v5(
    mut source: CommittedStoredAllRungExecutionV3,
    request: &AllRungSelectionV5Request<'_>,
) -> Result<CommittedStoredAllRungSelectionV5, String> {
    source.require_live_topology()?;
    let bounds = CanonicalSelectionV5Bounds::from(request);
    source.require_live_topology()?;
    let root_specs = AllRungSelectionRootSpecs {
        one_minute: request.one_minute_root,
        two_minute: request.two_minute_root,
        three_minute: request.three_minute_root,
        five_minute: request.five_minute_root,
        ten_minute: request.ten_minute_root,
        fifteen_minute: request.fifteen_minute_root,
        thirty_minute: request.thirty_minute_root,
        sixty_minute: request.sixty_minute_root,
    };
    let AllRungExecutionV3SelectionSources {
        one_minute: one_minute_source,
        two_minute: two_minute_source,
        three_minute: three_minute_source,
        five_minute: five_minute_source,
        ten_minute: ten_minute_source,
        fifteen_minute: fifteen_minute_source,
        thirty_minute: thirty_minute_source,
        sixty_minute: sixty_minute_source,
        topology,
    } = source.into_selection_sources(&root_specs)?;

    let (one_minute, one_minute_receipts) = commit_selection_rung(
        one_minute_source,
        &topology,
        request.one_minute_root,
        bounds.one_minute,
        request.ranking_policy,
        60,
        "1min",
    )?;
    let (two_minute, two_minute_receipts) = commit_selection_rung(
        two_minute_source,
        &topology,
        request.two_minute_root,
        bounds.two_minute,
        request.ranking_policy,
        120,
        "2min",
    )?;
    let (three_minute, three_minute_receipts) = commit_selection_rung(
        three_minute_source,
        &topology,
        request.three_minute_root,
        bounds.three_minute,
        request.ranking_policy,
        180,
        "3min",
    )?;
    let (five_minute, five_minute_receipts) = commit_selection_rung(
        five_minute_source,
        &topology,
        request.five_minute_root,
        bounds.five_minute,
        request.ranking_policy,
        300,
        "5min",
    )?;
    let (ten_minute, ten_minute_receipts) = commit_selection_rung(
        ten_minute_source,
        &topology,
        request.ten_minute_root,
        bounds.ten_minute,
        request.ranking_policy,
        600,
        "10min",
    )?;
    let (fifteen_minute, fifteen_minute_receipts) = commit_selection_rung(
        fifteen_minute_source,
        &topology,
        request.fifteen_minute_root,
        bounds.fifteen_minute,
        request.ranking_policy,
        900,
        "15min",
    )?;
    let (thirty_minute, thirty_minute_receipts) = commit_selection_rung(
        thirty_minute_source,
        &topology,
        request.thirty_minute_root,
        bounds.thirty_minute,
        request.ranking_policy,
        1_800,
        "30min",
    )?;
    let (sixty_minute, sixty_minute_receipts) = commit_selection_rung(
        sixty_minute_source,
        &topology,
        request.sixty_minute_root,
        bounds.sixty_minute,
        request.ranking_policy,
        3_600,
        "60min",
    )?;

    let mut committed = CommittedStoredAllRungSelectionV5 {
        selections: CanonicalStoredSelectionV5Authorities {
            one_minute,
            two_minute,
            three_minute,
            five_minute,
            ten_minute,
            fifteen_minute,
            thirty_minute,
            sixty_minute,
        },
        topology,
        bounds,
        ranking_policy: request.ranking_policy,
        receipts: CanonicalSelectionV5Receipts {
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

fn commit_selection_rung(
    source: CommittedStoredExecutionV3,
    topology: &AllRungSelectionTopologyToken,
    root: &Path,
    bounds: SelectionV5Bounds,
    policy: RankingPolicyV1,
    expected_rung: u32,
    rung_name: &str,
) -> Result<(CommittedStoredSelectionV5, RungSelectionV5Receipts), String> {
    topology.require_same(&format!("before {rung_name} Selection V5 commit"))?;
    let execution = source.structural_receipt();
    let mut selection = commit_stored_selection_v5(root, bounds, source, policy)
        .map_err(|why| format!("all-rung {rung_name} Selection V5 refused: {why}"))?;
    let receipts = RungSelectionV5Receipts {
        execution,
        selection: selection.structural_receipt(),
    };
    let _ = require_committed_selection_rung(
        &mut selection,
        topology,
        bounds,
        receipts,
        policy,
        expected_rung,
        rung_name,
    )?;
    topology.require_same(&format!("after {rung_name} Selection V5 commit"))?;
    Ok((selection, receipts))
}

#[derive(Clone)]
struct RungSelectionObservation {
    proof: RungReceiptProof,
    winner_row_ids: Vec<[u8; 32]>,
}

#[derive(Clone, Copy)]
struct RungReceiptProof {
    selection_id: [u8; 32],
    completion_id: [u8; 32],
    population_id: [u8; 32],
    execution_completion_id: [u8; 32],
    rung_seconds: u32,
    selected_count: u64,
    top_ten_count: u32,
}

impl From<SelectionV5StructuralReceipt> for RungReceiptProof {
    fn from(value: SelectionV5StructuralReceipt) -> Self {
        Self {
            selection_id: value.selection_id(),
            completion_id: value.completion_id(),
            population_id: value.population_id(),
            execution_completion_id: value.execution_completion_id(),
            rung_seconds: value.rung_seconds(),
            selected_count: value.selected_count(),
            top_ten_count: value.top_ten_count(),
        }
    }
}

struct CanonicalSelectionObservations {
    one_minute: RungSelectionObservation,
    two_minute: RungSelectionObservation,
    three_minute: RungSelectionObservation,
    five_minute: RungSelectionObservation,
    ten_minute: RungSelectionObservation,
    fifteen_minute: RungSelectionObservation,
    thirty_minute: RungSelectionObservation,
    sixty_minute: RungSelectionObservation,
}

fn require_committed_selection_rung(
    selection: &mut CommittedStoredSelectionV5,
    topology: &AllRungSelectionTopologyToken,
    expected_bounds: SelectionV5Bounds,
    expected_receipts: RungSelectionV5Receipts,
    policy: RankingPolicyV1,
    expected_rung: u32,
    rung_name: &str,
) -> Result<RungSelectionObservation, String> {
    topology.require_same(&format!("before {rung_name} Selection V5 proof"))?;
    if selection.bounds() != expected_bounds {
        return Err(format!(
            "all-rung {rung_name} Selection V5 retained bounds differ from the explicit request"
        ));
    }
    if selection.ranking_policy_digest() != policy.digest() {
        return Err(format!(
            "all-rung {rung_name} Selection V5 ranking policy differs from the exact cohort policy"
        ));
    }
    let receipt_before = selection.structural_receipt();
    if receipt_before != expected_receipts.selection
        || receipt_before.execution_completion_id() != expected_receipts.execution.completion_id()
        || selection.rung_seconds() != expected_rung
    {
        return Err(format!(
            "all-rung {rung_name} Selection V5 receipt does not join its exact Execution V3 source/rung"
        ));
    }
    let proof = RungReceiptProof::from(receipt_before);
    require_rung_receipt_proof(rung_name, expected_rung, proof)?;

    let top_twenty_five = selection.top_twenty_five()?;
    let top_ten = selection.top_ten()?;
    require_exact_prefix(rung_name, &top_twenty_five, &top_ten)?;
    let successor = selection.successor_winners()?;
    require_successor_join(rung_name, receipt_before, &top_twenty_five, &successor)?;
    let winner_row_ids = require_winner_shapes(
        rung_name,
        receipt_before,
        &top_twenty_five,
        expected_receipts.execution.completion_id(),
    )?;
    if selection.structural_receipt() != receipt_before {
        return Err(format!(
            "all-rung {rung_name} Selection V5 receipt changed during retained proof"
        ));
    }
    topology.require_same(&format!("after {rung_name} Selection V5 proof"))?;
    Ok(RungSelectionObservation {
        proof,
        winner_row_ids,
    })
}

fn require_rung_receipt_proof(
    rung_name: &str,
    expected_rung: u32,
    proof: RungReceiptProof,
) -> Result<(), String> {
    for (name, value) in [
        ("selection", proof.selection_id),
        ("Completion", proof.completion_id),
        ("Population", proof.population_id),
        ("Execution Completion", proof.execution_completion_id),
    ] {
        if value.iter().all(|byte| *byte == 0) {
            return Err(format!(
                "all-rung {rung_name} Selection V5 {name} identity is zero"
            ));
        }
    }
    let expected_top_ten = u32::try_from(proof.selected_count.min(TOP_TEN_U64))
        .map_err(|_| format!("all-rung {rung_name} Top-10 count does not fit u32"))?;
    if proof.rung_seconds != expected_rung
        || proof.selected_count > MAX_WINNERS
        || proof.top_ten_count != expected_top_ten
    {
        return Err(format!(
            "all-rung {rung_name} Selection V5 receipt has foreign rung or Top-25/Top-10 counts"
        ));
    }
    Ok(())
}

fn require_exact_prefix<T: PartialEq>(
    rung_name: &str,
    top_twenty_five: &[T],
    top_ten: &[T],
) -> Result<(), String> {
    let prefix_count = top_twenty_five.len().min(TOP_TEN);
    // `.get(..)` rather than `[..]`: the range is clamped by the `min` above and
    // so cannot be out of bounds, but a slice index that is only PROVED safe by
    // the line above it is one edit away from not being. The `Option` costs
    // nothing and makes a future edit that breaks the clamp a compile error
    // here instead of a panic in an operator's run.
    if top_ten.len() != prefix_count || top_twenty_five.get(..prefix_count) != Some(top_ten) {
        return Err(format!(
            "all-rung {rung_name} Top-10 is not the exact Top-25 prefix"
        ));
    }
    Ok(())
}

fn require_successor_join(
    rung_name: &str,
    receipt: SelectionV5StructuralReceipt,
    rows: &[SelectionV5RowRecord],
    successors: &[SelectionV5SuccessorWinnerV1],
) -> Result<(), String> {
    if rows.len() != successors.len() {
        return Err(format!(
            "all-rung {rung_name} successor winner count differs from Top-25"
        ));
    }
    for (index, (row, successor)) in rows.iter().zip(successors).enumerate() {
        if successor.row() != *row
            || successor
                .selected_exit_digest()
                .is_none_or(|digest| digest.iter().all(|byte| *byte == 0))
            || row.selection_id() != receipt.selection_id()
        {
            return Err(format!(
                "all-rung {rung_name} successor winner {index} does not exact-join its durable selected exit"
            ));
        }
    }
    Ok(())
}

fn require_winner_shapes(
    rung_name: &str,
    receipt: SelectionV5StructuralReceipt,
    rows: &[SelectionV5RowRecord],
    expected_execution_completion: [u8; 32],
) -> Result<Vec<[u8; 32]>, String> {
    let declared = usize::try_from(receipt.selected_count())
        .map_err(|_| format!("all-rung {rung_name} selected count does not fit usize"))?;
    if rows.len() != declared {
        return Err(format!(
            "all-rung {rung_name} Top-25 row count differs from its Completion"
        ));
    }
    let mut row_ids = HashSet::new();
    row_ids
        .try_reserve(rows.len())
        .map_err(|why| format!("all-rung {rung_name} winner index reserve failed: {why}"))?;
    let mut ordered = Vec::new();
    ordered
        .try_reserve_exact(rows.len())
        .map_err(|why| format!("all-rung {rung_name} winner proof reserve failed: {why}"))?;
    for (index, row) in rows.iter().enumerate() {
        let expected_rank = u32::try_from(index)
            .ok()
            .and_then(|rank| rank.checked_add(1))
            .ok_or_else(|| format!("all-rung {rung_name} winner rank overflowed"))?;
        if row.selection_id() != receipt.selection_id()
            || row.execution_completion_id() != expected_execution_completion
            || row.rung_seconds() != receipt.rung_seconds()
            || row.rank() != expected_rank
            || row.row_id().iter().all(|byte| *byte == 0)
            || !row_ids.insert(row.row_id())
        {
            return Err(format!(
                "all-rung {rung_name} winner {index} has a foreign/duplicate identity, source, rung or rank"
            ));
        }
        ordered.push(row.row_id());
    }
    Ok(ordered)
}

fn require_cross_rung_observations(
    observations: &CanonicalSelectionObservations,
) -> Result<(), String> {
    let entries = [
        ("1min", 60, &observations.one_minute),
        ("2min", 120, &observations.two_minute),
        ("3min", 180, &observations.three_minute),
        ("5min", 300, &observations.five_minute),
        ("10min", 600, &observations.ten_minute),
        ("15min", 900, &observations.fifteen_minute),
        ("30min", 1_800, &observations.thirty_minute),
        ("60min", 3_600, &observations.sixty_minute),
    ];
    let mut selection_ids = HashSet::new();
    let mut completion_ids = HashSet::new();
    let mut population_ids = HashSet::new();
    let mut execution_ids = HashSet::new();
    let mut winner_row_ids = HashSet::new();
    for (rung_name, expected_rung, observation) in entries {
        require_rung_receipt_proof(rung_name, expected_rung, observation.proof)?;
        if !selection_ids.insert(observation.proof.selection_id)
            || !completion_ids.insert(observation.proof.completion_id)
            || !population_ids.insert(observation.proof.population_id)
            || !execution_ids.insert(observation.proof.execution_completion_id)
        {
            return Err(format!(
                "all-rung {rung_name} Selection V5 reuses a foreign cross-rung source/receipt identity"
            ));
        }
        for row_id in &observation.winner_row_ids {
            if !winner_row_ids.insert(*row_id) {
                return Err(format!(
                    "all-rung {rung_name} Selection V5 reuses a cross-rung winner row identity"
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "controlled adversarial fixtures mutate exact roots and proof fields"
)]
mod tests {
    use super::*;

    fn id(value: u8) -> [u8; 32] {
        [value; 32]
    }

    fn observation(rung: u32, identity: u8) -> RungSelectionObservation {
        RungSelectionObservation {
            proof: RungReceiptProof {
                selection_id: id(identity),
                completion_id: id(identity.wrapping_add(16)),
                population_id: id(identity.wrapping_add(32)),
                execution_completion_id: id(identity.wrapping_add(48)),
                rung_seconds: rung,
                selected_count: 2,
                top_ten_count: 2,
            },
            winner_row_ids: vec![id(identity.wrapping_add(64)), id(identity.wrapping_add(80))],
        }
    }

    fn canonical_observations() -> CanonicalSelectionObservations {
        CanonicalSelectionObservations {
            one_minute: observation(60, 1),
            two_minute: observation(120, 2),
            three_minute: observation(180, 3),
            five_minute: observation(300, 4),
            ten_minute: observation(600, 5),
            fifteen_minute: observation(900, 6),
            thirty_minute: observation(1_800, 7),
            sixty_minute: observation(3_600, 8),
        }
    }

    #[test]
    fn canonical_rungs_and_cross_rung_receipts_are_exact() {
        assert_eq!(CANONICAL_RUNGS, [60, 120, 180, 300, 600, 900, 1_800, 3_600]);
        assert_eq!(
            CANONICAL_RUNG_NAMES,
            [
                "1min", "2min", "3min", "5min", "10min", "15min", "30min", "60min"
            ]
        );
        assert!(require_cross_rung_observations(&canonical_observations()).is_ok());
    }

    #[test]
    fn foreign_counts_rungs_and_cross_rung_identities_refuse() {
        let mut observations = canonical_observations();
        observations.three_minute.proof.rung_seconds = 300;
        assert!(require_cross_rung_observations(&observations).is_err());

        observations = canonical_observations();
        observations.five_minute.proof.selected_count = 26;
        assert!(require_cross_rung_observations(&observations).is_err());

        observations = canonical_observations();
        observations.ten_minute.proof.top_ten_count = 1;
        assert!(require_cross_rung_observations(&observations).is_err());

        observations = canonical_observations();
        observations.sixty_minute.proof.execution_completion_id =
            observations.one_minute.proof.execution_completion_id;
        assert!(require_cross_rung_observations(&observations).is_err());

        observations = canonical_observations();
        observations.thirty_minute.winner_row_ids[0] = observations.two_minute.winner_row_ids[0];
        assert!(require_cross_rung_observations(&observations).is_err());
    }

    #[test]
    fn exact_top_ten_prefix_refuses_missing_reordered_or_foreign_rows() {
        let top = [1_u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        assert!(require_exact_prefix("1min", &top, &top[..10]).is_ok());
        assert!(require_exact_prefix("1min", &top, &top[..9]).is_err());
        let reordered = [1_u8, 2, 4, 3, 5, 6, 7, 8, 9, 10];
        assert!(require_exact_prefix("1min", &top, &reordered).is_err());
        let foreign = [1_u8, 2, 3, 4, 5, 6, 7, 8, 9, 99];
        assert!(require_exact_prefix("1min", &top, &foreign).is_err());
        assert!(require_exact_prefix::<u8>("1min", &[], &[]).is_ok());
    }
}
