//! Admission-authoritative, globally exclusive replay and durable publication.
//!
//! This module is the first consumer that joins all eight canonical Selection
//! V3 rungs to their exact execution-capability authorities.  It reconstructs
//! every retained coordinate from caller-supplied training evidence, publishes
//! every pre-exclusivity OOS candidate, and offers candidates in timestamp
//! order to [`GlobalSinglePositionV1`].  The runner's strategy-local cell is
//! never consulted as a fallback.
//!
//! Four append-only files retain stream, decision, admitted-money and
//! receipt-last completion records.  Reopen replays the global scheduler from
//! the decision evidence and refuses a missing, reordered, copied or partially
//! committed block.  India VIX is stamped only after global admission through
//! [`VixReferenceMonth`].  It changes the publication identity, but never the
//! selection, P&L, execution run or replay identity.
//!
//! # Cost
//!
//! Coordinate reconstruction and OOS replay are linear in the supplied bars,
//! resolved grid and candidate paths.  The chronological merge examines at
//! most 200 stream heads per emitted minute.  Persistence, hashing and reopen
//! are linear in their records.  Only an already-indexed execution-capability
//! lookup is average O(1); this module makes no end-to-end O(1) claim.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use brutex_core::blake3::{self, Hasher};
use brutex_core::instrument::{Exchange, InstrumentKey};
use costs::fill::Direction;
use indicators::Candle;
use indicators::column::Column;
use pull::session::IstMoment;
use runner::exit_grid_policy::{
    ExecutionRunV1, ExecutionSeriesV1, OosExecutionSeriesV1, ReplayedCandidateUniverseV1,
};
use runner::grid::{ReplayCandidateV1, ReplayPathV1, ReplayPriceV1, TradeRow};
use runner::portfolio::{
    Constituent, Counters, Decision as PortfolioDecision, Disposition, Evidence,
    GlobalSinglePositionV1, Intent, MAX_INTENTS_PER_MINUTE, StrategyDigest,
};
use store::path::YearMonth;

use crate::execution_capability::{
    ExecutionCapabilityCompletionV1, ExecutionCapabilityLedger, exact_execution_law_digest_v1,
};
use crate::population::{InstrumentFamilyV1, PopulationRowV1, TradeDirectionV1};
use crate::selection::SelectedEntryV1;
use crate::selection_v3::{PopulationReferenceV3, SelectionReceiptV3};
use crate::vix_reference::{VixReferenceMonth, VixStamp};

/// Operator-facing refusal from replay construction, persistence or reopen.
pub type GlobalReplayRefusal = String;

const CANONICAL_RUNGS_SECONDS: [u32; 8] = [60, 120, 180, 300, 600, 900, 1_800, 3_600];
const MAX_STREAMS: usize = 8 * 25;
const HEADER_BYTES: u64 = 24;
const HEADER_BYTES_USIZE: usize = 24;
const FORMAT_VERSION: u32 = 1;
const SEAL_BYTES: usize = 32;

const STREAM_MAGIC: [u8; 8] = *b"BRUTXGS1";
const DECISION_MAGIC: [u8; 8] = *b"BRUTXGD1";
const TRADE_MAGIC: [u8; 8] = *b"BRUTXGT1";
const COMPLETION_MAGIC: [u8; 8] = *b"BRUTXGC1";

const STREAM_PAYLOAD_BYTES: usize = 368;
const DECISION_PAYLOAD_BYTES: usize = 208;
const TRADE_PAYLOAD_BYTES: usize = 320;
const COMPLETION_PAYLOAD_BYTES: usize = 1_216;

/// Fixed width of one reconstructed strategy-stream record including seal.
pub const GLOBAL_REPLAY_STREAM_STRIDE: usize = STREAM_PAYLOAD_BYTES + SEAL_BYTES;
/// Fixed width of one global scheduling-decision record including seal.
pub const GLOBAL_REPLAY_DECISION_STRIDE: usize = DECISION_PAYLOAD_BYTES + SEAL_BYTES;
/// Fixed width of one globally admitted, priceable trade including VIX stamps.
pub const GLOBAL_REPLAY_TRADE_STRIDE: usize = TRADE_PAYLOAD_BYTES + SEAL_BYTES;
/// Fixed width of one receipt-last global replay completion including seal.
pub const GLOBAL_REPLAY_COMPLETION_STRIDE: usize = COMPLETION_PAYLOAD_BYTES + SEAL_BYTES;

const MANIFEST_DOMAIN: &[u8] = b"brutex.cli.global-replay-manifest.v1\0";
const COMMON_COHORT_DOMAIN: &[u8] = b"brutex.cli.global-replay-common-cohort.v1\0";
const STREAM_ORDER_DOMAIN: &[u8] = b"brutex.cli.global-replay-stream-order.v1\0";
const DECISION_ORDER_DOMAIN: &[u8] = b"brutex.cli.global-replay-decision-order.v1\0";
const TRADE_ORDER_DOMAIN: &[u8] = b"brutex.cli.global-replay-trade-order.v1\0";
const CANDIDATE_DOMAIN: &[u8] = b"brutex.cli.global-replay-candidate.v1\0";
const REPLAY_ID_DOMAIN: &[u8] = b"brutex.cli.global-replay-id.v1\0";
const PUBLICATION_ID_DOMAIN: &[u8] = b"brutex.cli.global-replay-publication-id.v1\0";
const VIX_STAMP_POLICY_DOMAIN: &[u8] = b"brutex.cli.vix-exact-or-absent-no-interpolation.v1\0";

const _: () = assert!(MAX_STREAMS == MAX_INTENTS_PER_MINUTE);
const _: () = assert!(GLOBAL_REPLAY_STREAM_STRIDE == 400);
const _: () = assert!(GLOBAL_REPLAY_DECISION_STRIDE == 240);
const _: () = assert!(GLOBAL_REPLAY_TRADE_STRIDE == 352);
const _: () = assert!(GLOBAL_REPLAY_COMPLETION_STRIDE == 1_248);

/// Exact caller-held evidence needed to reconstruct one selected strategy.
///
/// The selection id and one-based rank locate the winner.  Every other field is
/// typed evidence rather than a digest guessed back into a value.
#[derive(Clone, Copy, Debug)]
pub struct SelectedReplayWitnessV1<'a> {
    /// V3 selection containing this selected row.
    pub selection_id: [u8; 32],
    /// One-based position within that selection's exact Top-25.
    pub rank: u16,
    /// Exact source population row.
    pub population_row: PopulationRowV1,
    /// Exact training execution series used to resolve the exit grid.
    pub training_series: ExecutionSeriesV1<'a>,
    /// Rebuilt training condition column with its evaluator capability.
    pub training_column: &'a Column,
    /// Canonical training run capability.
    pub training_run: ExecutionRunV1,
    /// Explicit OOS one-minute execution series and causal split.
    pub oos_series: OosExecutionSeriesV1<'a>,
    /// Complete OOS condition column.
    pub oos_column: &'a Column,
    /// Canonical OOS run capability.
    pub oos_run: ExecutionRunV1,
}

/// One fully prepared global replay, ready for receipt-last persistence.
///
/// Fields remain private so callers cannot detach a completion from the exact
/// stream, decision and trade vectors that produced it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedGlobalReplayV1 {
    manifest: ReplayManifestV1,
    streams: Vec<StreamRecordV1>,
    decisions: Vec<DecisionRecordV1>,
    trades: Vec<TradeRecordV1>,
    counters: Counters,
    pricing_refused_candidates: u64,
    admitted_pricing_refused: u64,
    replay_id: [u8; 32],
    publication_id: [u8; 32],
    ordered_stream_digest: [u8; 32],
    ordered_decision_digest: [u8; 32],
    ordered_trade_digest: [u8; 32],
}

impl PreparedGlobalReplayV1 {
    /// Global execution identity, deliberately independent of India VIX.
    #[must_use]
    pub const fn replay_id(&self) -> [u8; 32] {
        self.replay_id
    }

    /// Durable publication identity, including exact-or-absent VIX stamps.
    #[must_use]
    pub const fn publication_id(&self) -> [u8; 32] {
        self.publication_id
    }

    /// Number of selected strategy streams across all eight rungs.
    #[must_use]
    pub fn stream_count(&self) -> usize {
        self.streams.len()
    }

    /// Number of pre-exclusivity entries offered to the global scheduler.
    #[must_use]
    pub fn decision_count(&self) -> usize {
        self.decisions.len()
    }

    /// Number of globally admitted candidates carrying exact money rows.
    #[must_use]
    pub fn trade_count(&self) -> usize {
        self.trades.len()
    }

    /// Exhaustive scheduler counters.
    #[must_use]
    pub const fn counters(&self) -> Counters {
        self.counters
    }

    /// Reachable candidate paths whose money pricing was refused.
    #[must_use]
    pub const fn pricing_refused_candidates(&self) -> u64 {
        self.pricing_refused_candidates
    }

    /// Pricing-refused candidates that acquired occupancy but no money row.
    #[must_use]
    pub const fn admitted_pricing_refused(&self) -> u64 {
        self.admitted_pricing_refused
    }

    /// Identity of the exact eight selections and sixteen execution authorities.
    #[must_use]
    pub const fn manifest_digest(&self) -> [u8; 32] {
        self.manifest.manifest_digest
    }

    /// Exact canonical Selection V3 identities in rung order.
    #[must_use]
    pub const fn selection_ids(&self) -> [[u8; 32]; 8] {
        self.manifest.selection_ids
    }

    /// Exact execution-completion authorities in rung then family order.
    #[must_use]
    pub const fn execution_authority_ids(&self) -> [[u8; 32]; 16] {
        self.manifest.execution_authority_ids
    }
}

/// Reconstruct every selected strategy and run one global long/short position.
///
/// # Errors
///
/// Refuses a missing/duplicate rung, partial cross-rung cohort, absent or
/// mismatched execution authority, missing/duplicate witness, changed row,
/// training/OOS replay refusal, scheduler refusal, money row attached to a
/// pricing-refused path, quality-ceiling breach, missing VIX month authority or
/// malformed exact VIX stamp.
pub fn prepare_global_replay_v1(
    selections: &[SelectionReceiptV3],
    execution: &mut ExecutionCapabilityLedger,
    witnesses: &[SelectedReplayWitnessV1<'_>],
    vix_months: &[VixReferenceMonth],
) -> Result<PreparedGlobalReplayV1, GlobalReplayRefusal> {
    let manifest = ReplayManifestV1::new(selections, execution)?;
    for selection in selections {
        if selection.top_twenty_five().len() != 25 {
            return Err(format!(
                "global replay requires the complete Top-25 at {}s, found {} selected streams",
                selection.rung_seconds(),
                selection.top_twenty_five().len()
            ));
        }
    }
    let expected_streams = selections.iter().try_fold(0_usize, |sum, receipt| {
        sum.checked_add(receipt.top_twenty_five().len())
            .ok_or_else(|| "global replay selected-stream count overflow".to_owned())
    })?;
    if expected_streams != MAX_STREAMS {
        return Err(format!(
            "global replay requires exactly {MAX_STREAMS} selected streams, found {expected_streams}"
        ));
    }
    if witnesses.len() != expected_streams {
        return Err(format!(
            "global replay requires exactly {expected_streams} selected witnesses, found {}",
            witnesses.len()
        ));
    }
    let vix = VixCatalogV1::new(vix_months)?;
    let mut witness_index = HashMap::new();
    witness_index
        .try_reserve(witnesses.len())
        .map_err(|why| format!("global replay witness index allocation refused: {why}"))?;
    for (index, witness) in witnesses.iter().enumerate() {
        if witness.rank == 0 || witness.rank > 25 {
            return Err(format!(
                "selection {} witness rank {} is outside 1..=25",
                hex(&witness.selection_id),
                witness.rank
            ));
        }
        if witness_index
            .insert((witness.selection_id, witness.rank), index)
            .is_some()
        {
            return Err(format!(
                "selection {} rank {} has a duplicate replay witness",
                hex(&witness.selection_id),
                witness.rank
            ));
        }
    }
    let mut runtime = Vec::new();
    runtime
        .try_reserve_exact(expected_streams)
        .map_err(|why| format!("global replay stream allocation refused: {why}"))?;

    for (selection_index, selection) in selections.iter().enumerate() {
        for (rank_zero, entry) in selection.top_twenty_five().iter().copied().enumerate() {
            let rank = u16::try_from(
                rank_zero
                    .checked_add(1)
                    .ok_or_else(|| "global replay rank overflow".to_owned())?,
            )
            .map_err(|_| "global replay rank does not fit u16".to_owned())?;
            let witness_slot = witness_index
                .remove(&(selection.selection_id(), rank))
                .ok_or_else(|| {
                    format!(
                        "selection {} rank {rank} has no exact replay witness",
                        hex(&selection.selection_id())
                    )
                })?;
            let witness = witnesses
                .get(witness_slot)
                .ok_or_else(|| "selected witness index disappeared".to_owned())?;
            let stream_ordinal = u16::try_from(runtime.len())
                .map_err(|_| "global replay stream ordinal does not fit u16".to_owned())?;
            let built = reconstruct_stream(
                selection_index,
                selection,
                rank,
                entry,
                stream_ordinal,
                witness,
                execution,
                &manifest,
            )?;
            runtime.push(built);
        }
    }
    if !witness_index.is_empty() {
        return Err(
            "global replay contains a witness not named by the canonical eight selections"
                .to_owned(),
        );
    }

    schedule_runtime(manifest, &mut runtime, &vix)
}

#[derive(Clone, Debug)]
struct RuntimeStream {
    record: StreamRecordV1,
    universe: ReplayedCandidateUniverseV1,
    cursor: usize,
    max_ambiguous_bars: u64,
    max_gap_fills: u64,
    admitted_ambiguous_bars: u64,
    admitted_gap_fills: u64,
    feed: String,
}

fn require_exact_witness_row(
    selection: &SelectionReceiptV3,
    rank: u16,
    entry: SelectedEntryV1,
    witness: &SelectedReplayWitnessV1<'_>,
) -> Result<PopulationRowV1, GlobalReplayRefusal> {
    let row = witness.population_row;
    if witness.selection_id != selection.selection_id()
        || witness.rank != rank
        || row.population_id != entry.population_id()
        || row.sequence != entry.row_sequence()
        || row.strategy_digest != entry.strategy_digest()
        || row.instrument_family != entry.family()
        || row.rung_seconds != selection.rung_seconds()
    {
        return Err(format!(
            "selection {} rank {rank} witness differs from its exact selected population row",
            hex(&selection.selection_id())
        ));
    }
    Ok(row)
}

#[allow(
    clippy::too_many_arguments,
    reason = "each argument is a separately checked selection or execution authority term"
)]
fn reconstruct_stream(
    selection_index: usize,
    selection: &SelectionReceiptV3,
    rank: u16,
    entry: SelectedEntryV1,
    stream_ordinal: u16,
    witness: &SelectedReplayWitnessV1<'_>,
    execution: &mut ExecutionCapabilityLedger,
    manifest: &ReplayManifestV1,
) -> Result<RuntimeStream, GlobalReplayRefusal> {
    let row = require_exact_witness_row(selection, rank, entry, witness)?;
    let capability = execution
        .capability(&row.population_id, row.sequence)?
        .ok_or_else(|| {
            format!(
                "selection {} rank {rank} has no committed execution row capability",
                hex(&selection.selection_id())
            )
        })?;
    let parameters = execution
        .parameters(&row.population_id, row.direction)?
        .ok_or_else(|| {
            format!(
                "population {} has no exact {:?} execution parameters",
                hex(&row.population_id),
                row.direction
            )
        })?;
    let expected_policy_digest = match row.direction {
        TradeDirectionV1::Long => selection.cohort_identity().long_exit_policy_digest(),
        TradeDirectionV1::Short => selection.cohort_identity().short_exit_policy_digest(),
    };
    if parameters.policy().digest() != expected_policy_digest {
        return Err(format!(
            "selection {} rank {rank} execution parameters differ from the cohort's {:?} exit policy",
            hex(&selection.selection_id()),
            row.direction
        ));
    }
    let (resolved, selected) = capability.reconstruct_selected(
        parameters,
        row,
        witness.training_series,
        witness.training_column,
        witness.training_run,
    )?;
    let universe = resolved
        .replay_selected_universe(
            witness.oos_series,
            witness.oos_column,
            &selected,
            witness.oos_run,
        )
        .map_err(|why| {
            format!(
                "selection {} rank {rank} OOS candidate replay refused: {why:?}",
                hex(&selection.selection_id())
            )
        })?;
    universe.require_integrity().map_err(|why| {
        format!(
            "selection {} rank {rank} candidate universe failed integrity: {why:?}",
            hex(&selection.selection_id())
        )
    })?;
    let authority_id = manifest.authority_id(selection_index, entry.family())?;
    let candidate_count = usize_u64(universe.candidates().len(), "candidate universe")?;
    let record = StreamRecordV1 {
        replay_id: [0; 32],
        stream_ordinal,
        rank,
        rung_seconds: selection.rung_seconds(),
        family: entry.family(),
        direction: row.direction,
        selection_id: selection.selection_id(),
        population_id: row.population_id,
        row_sequence: row.sequence,
        strategy_digest: row.strategy_digest,
        execution_authority_id: authority_id,
        execution_capability_id: capability.capability_id(),
        training_run_id: witness.training_run.run_id().bytes(),
        selected_exit_digest: selected.digest(),
        oos_run_id: witness.oos_run.run_id().bytes(),
        universe_digest: universe.digest(),
        candidate_count,
        pricing_refused_paths: universe.pricing_refused_paths(),
    };
    record.validate_without_replay_id()?;
    Ok(RuntimeStream {
        record,
        universe,
        cursor: 0,
        max_ambiguous_bars: resolved.policy().max_ambiguous_bars(),
        max_gap_fills: resolved.policy().max_gap_fills(),
        admitted_ambiguous_bars: 0,
        admitted_gap_fills: 0,
        feed: witness.oos_series.series().feed().to_owned(),
    })
}

struct ScheduledRecordsV1 {
    decisions: Vec<DecisionRecordV1>,
    trades: Vec<TradeRecordV1>,
    counters: Counters,
    pricing_refused_candidates: u64,
    admitted_pricing_refused: u64,
}

struct ScheduleStateV1 {
    scheduler: GlobalSinglePositionV1,
    decisions: Vec<DecisionRecordV1>,
    trades: Vec<TradeRecordV1>,
    pricing_refused_candidates: u64,
    admitted_pricing_refused: u64,
}

impl ScheduleStateV1 {
    fn new(total_candidates: usize) -> Result<Self, GlobalReplayRefusal> {
        let mut decisions = Vec::new();
        decisions
            .try_reserve_exact(total_candidates)
            .map_err(|why| format!("global replay decision allocation refused: {why}"))?;
        Ok(Self {
            scheduler: GlobalSinglePositionV1::new(),
            decisions,
            trades: Vec::new(),
            pricing_refused_candidates: 0,
            admitted_pricing_refused: 0,
        })
    }

    fn schedule_minute(
        &mut self,
        entry_micros: i64,
        runtime: &mut [RuntimeStream],
        vix: &VixCatalogV1<'_>,
    ) -> Result<(), GlobalReplayRefusal> {
        let (offered, offered_streams) = offered_for_minute(runtime, entry_micros)?;
        let schedule = self
            .scheduler
            .schedule_minute(entry_micros, &offered)
            .map_err(|refusal| format!("global minute {entry_micros} refused: {refusal:?}"))?;
        for decision in schedule.decisions() {
            self.record_decision(decision, runtime, &offered_streams, vix)?;
        }
        Ok(())
    }

    fn record_decision(
        &mut self,
        decision: &PortfolioDecision,
        runtime: &mut [RuntimeStream],
        offered_streams: &[usize],
        vix: &VixCatalogV1<'_>,
    ) -> Result<(), GlobalReplayRefusal> {
        let stream_index = find_offered_stream(runtime, offered_streams, decision.constituent)?;
        let stream = runtime
            .get_mut(stream_index)
            .ok_or_else(|| "scheduled stream index disappeared".to_owned())?;
        let candidate = stream
            .universe
            .candidates()
            .get(stream.cursor)
            .copied()
            .ok_or_else(|| "scheduled candidate disappeared".to_owned())?;
        if path_tag(candidate.path()) != PathTagV1::Priceable {
            self.pricing_refused_candidates = self
                .pricing_refused_candidates
                .checked_add(1)
                .ok_or_else(|| "pricing-refused candidate count overflow".to_owned())?;
        }
        let decision_sequence = usize_u64(self.decisions.len(), "decision sequence")?;
        let record = DecisionRecordV1::from_scheduled(
            decision_sequence,
            &stream.record,
            stream.cursor,
            candidate,
            decision.disposition,
        )?;
        if matches!(decision.disposition, Disposition::Admitted { .. }) {
            self.record_admission(decision_sequence, stream, candidate, vix)?;
        }
        self.decisions.push(record);
        stream.cursor = stream
            .cursor
            .checked_add(1)
            .ok_or_else(|| "global replay candidate cursor overflow".to_owned())?;
        Ok(())
    }

    fn record_admission(
        &mut self,
        decision_sequence: u64,
        stream: &mut RuntimeStream,
        candidate: ReplayCandidateV1,
        vix: &VixCatalogV1<'_>,
    ) -> Result<(), GlobalReplayRefusal> {
        match candidate.path() {
            ReplayPathV1::Priceable(price) => {
                absorb_quality(stream, price)?;
                self.trades.push(TradeRecordV1::from_admitted(
                    decision_sequence,
                    &stream.record,
                    price,
                    vix.stamps(&stream.feed, price.row())?,
                )?);
            }
            ReplayPathV1::BlockOnly
            | ReplayPathV1::CrossingRefused
            | ReplayPathV1::BlockOnlyAndCrossingRefused => {
                self.admitted_pricing_refused = self
                    .admitted_pricing_refused
                    .checked_add(1)
                    .ok_or_else(|| "admitted pricing-refused count overflow".to_owned())?;
            }
        }
        Ok(())
    }

    fn finish(self, runtime: &[RuntimeStream]) -> Result<ScheduledRecordsV1, GlobalReplayRefusal> {
        if runtime
            .iter()
            .any(|stream| stream.cursor != stream.universe.candidates().len())
        {
            return Err(
                "global scheduler ended before every candidate reached a decision".to_owned(),
            );
        }
        let counters = self.scheduler.counters();
        if !counters.reconciles()
            || counters.offered != usize_u64(self.decisions.len(), "decision count")?
            || counters.unreachable != 0
            || counters.refused != 0
        {
            return Err(
                "global scheduler counters do not reconcile to all reachable candidates".to_owned(),
            );
        }
        let expected_trades = counters
            .admitted
            .checked_sub(self.admitted_pricing_refused)
            .ok_or_else(|| "admitted pricing-refused count exceeds admissions".to_owned())?;
        if expected_trades != usize_u64(self.trades.len(), "admitted money rows")? {
            return Err(
                "pricing-refused candidate acquired a money row or a priceable admission lost one"
                    .to_owned(),
            );
        }
        Ok(ScheduledRecordsV1 {
            decisions: self.decisions,
            trades: self.trades,
            counters,
            pricing_refused_candidates: self.pricing_refused_candidates,
            admitted_pricing_refused: self.admitted_pricing_refused,
        })
    }
}

fn offered_for_minute(
    runtime: &[RuntimeStream],
    entry_micros: i64,
) -> Result<(Vec<Intent>, Vec<usize>), GlobalReplayRefusal> {
    let mut offered = Vec::new();
    let mut offered_streams = Vec::new();
    offered
        .try_reserve_exact(MAX_STREAMS)
        .map_err(|why| format!("minute intent allocation refused: {why}"))?;
    offered_streams
        .try_reserve_exact(MAX_STREAMS)
        .map_err(|why| format!("minute stream-index allocation refused: {why}"))?;
    for (stream_index, stream) in runtime.iter().enumerate() {
        let Some(candidate) = stream.universe.candidates().get(stream.cursor) else {
            continue;
        };
        if candidate.entry_micros() == entry_micros {
            offered.push(Intent {
                constituent: constituent_of(stream)?,
                evidence: Evidence::Reachable {
                    occupied_through_micros: candidate.occupied_through_micros(),
                },
            });
            offered_streams.push(stream_index);
        }
    }
    Ok((offered, offered_streams))
}

fn schedule_runtime(
    manifest: ReplayManifestV1,
    runtime: &mut [RuntimeStream],
    vix: &VixCatalogV1<'_>,
) -> Result<PreparedGlobalReplayV1, GlobalReplayRefusal> {
    let total_candidates = runtime.iter().try_fold(0_usize, |sum, stream| {
        sum.checked_add(stream.universe.candidates().len())
            .ok_or_else(|| "global replay candidate count overflow".to_owned())
    })?;
    let mut state = ScheduleStateV1::new(total_candidates)?;
    while let Some(entry_micros) = next_entry_micros(runtime) {
        state.schedule_minute(entry_micros, runtime, vix)?;
    }
    finalize_prepared(manifest, runtime, state.finish(runtime)?)
}

fn finalize_prepared(
    manifest: ReplayManifestV1,
    runtime: &[RuntimeStream],
    scheduled: ScheduledRecordsV1,
) -> Result<PreparedGlobalReplayV1, GlobalReplayRefusal> {
    let ScheduledRecordsV1 {
        mut decisions,
        mut trades,
        counters,
        pricing_refused_candidates,
        admitted_pricing_refused,
    } = scheduled;
    let mut streams: Vec<_> = runtime.iter().map(|stream| stream.record).collect();
    let ordered_stream_digest = stream_order_digest(&streams)?;
    let ordered_decision_digest = decision_order_digest(&decisions)?;
    let replay_id = derive_replay_id(
        &manifest,
        ordered_stream_digest,
        ordered_decision_digest,
        counters,
        pricing_refused_candidates,
        admitted_pricing_refused,
    );
    for stream in &mut streams {
        stream.replay_id = replay_id;
    }
    for decision in &mut decisions {
        decision.replay_id = replay_id;
    }
    for trade in &mut trades {
        trade.replay_id = replay_id;
    }
    let ordered_stream_digest = stream_order_digest(&streams)?;
    let ordered_decision_digest = decision_order_digest(&decisions)?;
    let ordered_trade_digest = trade_order_digest(&trades)?;
    let publication_id = derive_publication_id(replay_id, ordered_trade_digest);
    let prepared = PreparedGlobalReplayV1 {
        manifest,
        streams,
        decisions,
        trades,
        counters,
        pricing_refused_candidates,
        admitted_pricing_refused,
        replay_id,
        publication_id,
        ordered_stream_digest,
        ordered_decision_digest,
        ordered_trade_digest,
    };
    prepared.validate()?;
    Ok(prepared)
}

fn next_entry_micros(runtime: &[RuntimeStream]) -> Option<i64> {
    runtime
        .iter()
        .filter_map(|stream| {
            stream
                .universe
                .candidates()
                .get(stream.cursor)
                .map(ReplayCandidateV1::entry_micros)
        })
        .min()
}

fn constituent_of(stream: &RuntimeStream) -> Result<Constituent, GlobalReplayRefusal> {
    Ok(Constituent {
        priority: stream.record.rank,
        strategy_digest: StrategyDigest::new(stream.record.strategy_digest),
        instrument: instrument_of(stream.record.family)?,
        direction: direction_of(stream.record.direction),
        rung_minutes: u16::try_from(stream.record.rung_seconds / 60)
            .map_err(|_| "signal rung minutes do not fit u16".to_owned())?,
    })
}

fn find_offered_stream(
    runtime: &[RuntimeStream],
    offered: &[usize],
    constituent: Constituent,
) -> Result<usize, GlobalReplayRefusal> {
    let mut found = None;
    for index in offered.iter().copied() {
        let Some(stream) = runtime.get(index) else {
            return Err("offered stream index is outside the runtime".to_owned());
        };
        if constituent_of(stream)? == constituent {
            if found.is_some() {
                return Err("scheduler constituent aliases more than one runtime stream".to_owned());
            }
            found = Some(index);
        }
    }
    found.ok_or_else(|| "scheduler returned a constituent not offered for this minute".to_owned())
}

fn absorb_quality(
    stream: &mut RuntimeStream,
    price: ReplayPriceV1,
) -> Result<(), GlobalReplayRefusal> {
    stream.admitted_ambiguous_bars = stream
        .admitted_ambiguous_bars
        .checked_add(price.ambiguous_bars())
        .ok_or_else(|| "admitted ambiguous-bar count overflow".to_owned())?;
    stream.admitted_gap_fills = stream
        .admitted_gap_fills
        .checked_add(price.gap_fills())
        .ok_or_else(|| "admitted gap-fill count overflow".to_owned())?;
    if stream.admitted_ambiguous_bars > stream.max_ambiguous_bars
        || stream.admitted_gap_fills > stream.max_gap_fills
    {
        return Err(format!(
            "selection {} rank {} globally admitted quality {}/{} exceeds frozen ceilings {}/{}",
            hex(&stream.record.selection_id),
            stream.record.rank,
            stream.admitted_ambiguous_bars,
            stream.admitted_gap_fills,
            stream.max_ambiguous_bars,
            stream.max_gap_fills
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReplayManifestV1 {
    manifest_digest: [u8; 32],
    common_cohort_digest: [u8; 32],
    selection_ids: [[u8; 32]; 8],
    execution_authority_ids: [[u8; 32]; 16],
}

struct ReplayManifestBuilderV1 {
    first_common: [u8; 440],
    common_cohort_digest: [u8; 32],
    selection_ids: [[u8; 32]; 8],
    execution_authority_ids: [[u8; 32]; 16],
    populations: HashSet<[u8; 32]>,
    authorities: HashSet<[u8; 32]>,
    signal_coverages: HashSet<[u8; 32]>,
}

impl ReplayManifestBuilderV1 {
    fn new(first_selection: &SelectionReceiptV3) -> Self {
        let first_common = common_cohort_bytes(first_selection);
        Self {
            common_cohort_digest: digest_common_cohort(&first_common),
            first_common,
            selection_ids: [[0; 32]; 8],
            execution_authority_ids: [[0; 32]; 16],
            populations: HashSet::new(),
            authorities: HashSet::new(),
            signal_coverages: HashSet::new(),
        }
    }

    fn admit(
        &mut self,
        index: usize,
        selection: &SelectionReceiptV3,
        expected_rung: u32,
        execution: &mut ExecutionCapabilityLedger,
    ) -> Result<(), GlobalReplayRefusal> {
        if selection.rung_seconds() != expected_rung {
            return Err(format!(
                "global replay rung slot {index} requires {expected_rung}s, found {}s",
                selection.rung_seconds()
            ));
        }
        require_digest("selection V3 identity", &selection.selection_id())?;
        if common_cohort_bytes(selection) != self.first_common {
            return Err(format!(
                "selection {} at {}s is outside the common cross-rung cohort",
                hex(&selection.selection_id()),
                selection.rung_seconds()
            ));
        }
        let coverage = selection.cohort_identity().signal_coverage_digest();
        require_digest("selection signal-rung coverage authority", &coverage)?;
        if !self.signal_coverages.insert(coverage) {
            return Err(
                "two different signal rungs copied one calendar-coverage authority".to_owned(),
            );
        }
        if self
            .selection_ids
            .iter()
            .take(index)
            .any(|held| *held == selection.selection_id())
        {
            return Err("one Selection V3 receipt was copied into more than one rung".to_owned());
        }
        *self
            .selection_ids
            .get_mut(index)
            .ok_or_else(|| "global replay selection manifest slot is absent".to_owned())? =
            selection.selection_id();
        let references = *selection.populations();
        let [nifty_reference, bank_nifty_reference] = references;
        if nifty_reference.family() != InstrumentFamilyV1::Nifty
            || bank_nifty_reference.family() != InstrumentFamilyV1::BankNifty
        {
            return Err("Selection V3 populations are not NIFTY then BANKNIFTY".to_owned());
        }
        for (family_index, reference) in references.into_iter().enumerate() {
            self.admit_reference(index, family_index, selection, reference, execution)?;
        }
        Ok(())
    }

    fn admit_reference(
        &mut self,
        selection_index: usize,
        family_index: usize,
        selection: &SelectionReceiptV3,
        reference: PopulationReferenceV3,
        execution: &mut ExecutionCapabilityLedger,
    ) -> Result<(), GlobalReplayRefusal> {
        if !self.populations.insert(reference.population_id()) {
            return Err(format!(
                "population {} is reused across global replay rung/family slots",
                hex(&reference.population_id())
            ));
        }
        let completion = execution
            .completion(&reference.population_id())?
            .ok_or_else(|| {
                format!(
                    "selection {} {}s {:?} population {} has no execution completion authority",
                    hex(&selection.selection_id()),
                    selection.rung_seconds(),
                    reference.family(),
                    hex(&reference.population_id())
                )
            })?;
        require_completion_matches(reference, &completion)?;
        if !self.authorities.insert(completion.authority_id()) {
            return Err(
                "one execution completion authority was copied across populations".to_owned(),
            );
        }
        let authority_slot = selection_index
            .checked_mul(2)
            .and_then(|offset| offset.checked_add(family_index))
            .ok_or_else(|| "execution-authority manifest slot overflow".to_owned())?;
        *self
            .execution_authority_ids
            .get_mut(authority_slot)
            .ok_or_else(|| "execution-authority manifest slot is absent".to_owned())? =
            completion.authority_id();
        Ok(())
    }

    fn finish(self) -> Result<ReplayManifestV1, GlobalReplayRefusal> {
        let manifest = ReplayManifestV1 {
            manifest_digest: derive_manifest_digest(
                self.common_cohort_digest,
                &self.selection_ids,
                &self.execution_authority_ids,
            ),
            common_cohort_digest: self.common_cohort_digest,
            selection_ids: self.selection_ids,
            execution_authority_ids: self.execution_authority_ids,
        };
        manifest.validate()?;
        Ok(manifest)
    }
}

impl ReplayManifestV1 {
    fn new(
        selections: &[SelectionReceiptV3],
        execution: &mut ExecutionCapabilityLedger,
    ) -> Result<Self, GlobalReplayRefusal> {
        let selections: &[SelectionReceiptV3; 8] = selections.try_into().map_err(|_| {
            format!(
                "global replay requires exactly eight canonical rung selections, found {}",
                selections.len()
            )
        })?;
        let first_selection = selections
            .first()
            .ok_or_else(|| "global replay canonical selection array is empty".to_owned())?;
        let mut builder = ReplayManifestBuilderV1::new(first_selection);
        for (index, (selection, expected_rung)) in
            selections.iter().zip(CANONICAL_RUNGS_SECONDS).enumerate()
        {
            builder.admit(index, selection, expected_rung, execution)?;
        }
        builder.finish()
    }

    fn authority_id(
        &self,
        selection_index: usize,
        family: InstrumentFamilyV1,
    ) -> Result<[u8; 32], GlobalReplayRefusal> {
        let family_index = match family {
            InstrumentFamilyV1::Nifty => 0,
            InstrumentFamilyV1::BankNifty => 1,
        };
        self.execution_authority_ids
            .get(
                selection_index
                    .checked_mul(2)
                    .and_then(|offset| offset.checked_add(family_index))
                    .ok_or_else(|| "execution-authority manifest slot overflow".to_owned())?,
            )
            .copied()
            .ok_or_else(|| "execution-authority manifest slot is absent".to_owned())
    }

    fn validate(&self) -> Result<(), GlobalReplayRefusal> {
        require_digest("global replay manifest", &self.manifest_digest)?;
        require_digest("global replay common cohort", &self.common_cohort_digest)?;
        let mut selections = HashSet::new();
        for digest in self.selection_ids {
            require_digest("global replay selection", &digest)?;
            if !selections.insert(digest) {
                return Err("global replay manifest repeats a selection identity".to_owned());
            }
        }
        let mut authorities = HashSet::new();
        for digest in self.execution_authority_ids {
            require_digest("global replay execution authority", &digest)?;
            if !authorities.insert(digest) {
                return Err("global replay manifest repeats an execution authority".to_owned());
            }
        }
        if self.manifest_digest
            != derive_manifest_digest(
                self.common_cohort_digest,
                &self.selection_ids,
                &self.execution_authority_ids,
            )
        {
            return Err("global replay manifest digest differs from its fields".to_owned());
        }
        Ok(())
    }
}

fn require_completion_matches(
    reference: PopulationReferenceV3,
    completion: &ExecutionCapabilityCompletionV1,
) -> Result<(), GlobalReplayRefusal> {
    if completion.population_id() != reference.population_id()
        || completion.population_v4_digest() != reference.population_v4_completion_digest()
        || completion.row_count() != reference.row_count()
    {
        return Err(format!(
            "execution completion {} does not cover exact Selection V3 population {}",
            hex(&completion.authority_id()),
            hex(&reference.population_id())
        ));
    }
    Ok(())
}

fn common_cohort_bytes(selection: &SelectionReceiptV3) -> [u8; 440] {
    let mut bytes = selection.cohort_identity().canonical_bytes();
    // Header 24 + eleven preceding 32-byte digests. Signal coverage is the one
    // term that must differ by rung; every other cohort byte must be identical.
    if let Some(slot) = bytes.get_mut(376..408) {
        slot.fill(0);
    }
    bytes
}

fn digest_common_cohort(bytes: &[u8; 440]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(COMMON_COHORT_DOMAIN);
    hasher.update(bytes);
    hasher.finalize()
}

fn derive_manifest_digest(
    common_cohort_digest: [u8; 32],
    selection_ids: &[[u8; 32]; 8],
    execution_authority_ids: &[[u8; 32]; 16],
) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(MANIFEST_DOMAIN);
    hasher.update(&common_cohort_digest);
    for (rung, selection) in CANONICAL_RUNGS_SECONDS.into_iter().zip(selection_ids) {
        hasher.update(&rung.to_le_bytes());
        hasher.update(selection);
    }
    for authority in execution_authority_ids {
        hasher.update(authority);
    }
    hasher.update(&exact_execution_law_digest_v1());
    hasher.finalize()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StreamRecordV1 {
    replay_id: [u8; 32],
    stream_ordinal: u16,
    rank: u16,
    rung_seconds: u32,
    family: InstrumentFamilyV1,
    direction: TradeDirectionV1,
    selection_id: [u8; 32],
    population_id: [u8; 32],
    row_sequence: u64,
    strategy_digest: [u8; 32],
    execution_authority_id: [u8; 32],
    execution_capability_id: [u8; 32],
    training_run_id: [u8; 32],
    selected_exit_digest: [u8; 32],
    oos_run_id: [u8; 32],
    universe_digest: [u8; 32],
    candidate_count: u64,
    pricing_refused_paths: u64,
}

impl StreamRecordV1 {
    fn validate_without_replay_id(self) -> Result<(), GlobalReplayRefusal> {
        if usize::from(self.stream_ordinal) >= MAX_STREAMS {
            return Err("global replay stream ordinal exceeds the fixed surface".to_owned());
        }
        if self.rank == 0 || self.rank > 25 {
            return Err("global replay stream rank is outside 1..=25".to_owned());
        }
        validate_rung(self.rung_seconds)?;
        for (name, digest) in [
            ("stream selection", self.selection_id),
            ("stream population", self.population_id),
            ("stream strategy", self.strategy_digest),
            ("stream execution authority", self.execution_authority_id),
            ("stream execution capability", self.execution_capability_id),
            ("stream training run", self.training_run_id),
            ("stream selected exit", self.selected_exit_digest),
            ("stream OOS run", self.oos_run_id),
            ("stream candidate universe", self.universe_digest),
        ] {
            require_digest(name, &digest)?;
        }
        if self.pricing_refused_paths > self.candidate_count {
            return Err("stream pricing-refused count exceeds candidate count".to_owned());
        }
        Ok(())
    }

    fn validate(self) -> Result<(), GlobalReplayRefusal> {
        require_digest("stream replay identity", &self.replay_id)?;
        self.validate_without_replay_id()
    }

    fn payload(self) -> Result<[u8; STREAM_PAYLOAD_BYTES], GlobalReplayRefusal> {
        self.validate()?;
        let mut raw = [0; STREAM_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.bytes(&self.replay_id)?;
        encoder.u16(self.stream_ordinal)?;
        encoder.u16(self.rank)?;
        encoder.u32(self.rung_seconds)?;
        encoder.u8(family_byte(self.family))?;
        encoder.u8(trade_direction_byte(self.direction))?;
        encoder.zeros(6)?;
        encoder.bytes(&self.selection_id)?;
        encoder.bytes(&self.population_id)?;
        encoder.u64(self.row_sequence)?;
        for digest in [
            self.strategy_digest,
            self.execution_authority_id,
            self.execution_capability_id,
            self.training_run_id,
            self.selected_exit_digest,
            self.oos_run_id,
            self.universe_digest,
        ] {
            encoder.bytes(&digest)?;
        }
        encoder.u64(self.candidate_count)?;
        encoder.u64(self.pricing_refused_paths)?;
        encoder.zeros(8)?;
        encoder.finish()?;
        Ok(raw)
    }

    fn to_bytes(self) -> Result<[u8; GLOBAL_REPLAY_STREAM_STRIDE], GlobalReplayRefusal> {
        with_seal(self.payload()?)
    }

    fn from_bytes(raw: &[u8; GLOBAL_REPLAY_STREAM_STRIDE]) -> Result<Self, GlobalReplayRefusal> {
        let payload = checked_payload::<STREAM_PAYLOAD_BYTES, GLOBAL_REPLAY_STREAM_STRIDE>(
            raw,
            "global replay stream",
        )?;
        let mut decoder = Decoder::new(&payload);
        let value = Self {
            replay_id: decoder.array_32()?,
            stream_ordinal: decoder.u16()?,
            rank: decoder.u16()?,
            rung_seconds: decoder.u32()?,
            family: decode_family(decoder.u8()?)?,
            direction: decode_trade_direction(decoder.u8()?)?,
            selection_id: {
                decoder.zeros(6, "stream tag reserve")?;
                decoder.array_32()?
            },
            population_id: decoder.array_32()?,
            row_sequence: decoder.u64()?,
            strategy_digest: decoder.array_32()?,
            execution_authority_id: decoder.array_32()?,
            execution_capability_id: decoder.array_32()?,
            training_run_id: decoder.array_32()?,
            selected_exit_digest: decoder.array_32()?,
            oos_run_id: decoder.array_32()?,
            universe_digest: decoder.array_32()?,
            candidate_count: decoder.u64()?,
            pricing_refused_paths: decoder.u64()?,
        };
        decoder.zeros(8, "stream trailing reserve")?;
        decoder.finish()?;
        value.validate()?;
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PathTagV1 {
    Priceable,
    BlockOnly,
    CrossingRefused,
    BlockOnlyAndCrossingRefused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DispositionV1 {
    Admitted,
    BlockedOccupied,
    BlockedSimultaneous,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DecisionRecordV1 {
    replay_id: [u8; 32],
    sequence: u64,
    stream_ordinal: u16,
    rank: u16,
    rung_seconds: u32,
    family: InstrumentFamilyV1,
    direction: TradeDirectionV1,
    disposition: DispositionV1,
    path: PathTagV1,
    strategy_digest: [u8; 32],
    candidate_digest: [u8; 32],
    signal_micros: i64,
    entry_micros: i64,
    occupied_through_micros: i64,
    disposition_through_micros: i64,
    candidate_ordinal: u64,
    admitted_strategy_digest: [u8; 32],
}

impl DecisionRecordV1 {
    fn from_scheduled(
        sequence: u64,
        stream: &StreamRecordV1,
        candidate_ordinal: usize,
        candidate: ReplayCandidateV1,
        disposition: Disposition,
    ) -> Result<Self, GlobalReplayRefusal> {
        let (tag, disposition_through_micros, admitted_strategy_digest) = match disposition {
            Disposition::Admitted {
                occupied_through_micros,
            } => (DispositionV1::Admitted, occupied_through_micros, [0; 32]),
            Disposition::BlockedOccupied {
                occupied_through_micros,
            } => (
                DispositionV1::BlockedOccupied,
                occupied_through_micros,
                [0; 32],
            ),
            Disposition::BlockedSimultaneous { admitted } => (
                DispositionV1::BlockedSimultaneous,
                i64::MIN,
                admitted.bytes(),
            ),
            Disposition::Unreachable | Disposition::Refused(_) => {
                return Err(
                    "a reachable replay candidate became unreachable/refused in scheduling"
                        .to_owned(),
                );
            }
        };
        let value = Self {
            replay_id: [0; 32],
            sequence,
            stream_ordinal: stream.stream_ordinal,
            rank: stream.rank,
            rung_seconds: stream.rung_seconds,
            family: stream.family,
            direction: stream.direction,
            disposition: tag,
            path: path_tag(candidate.path()),
            strategy_digest: stream.strategy_digest,
            candidate_digest: digest_candidate(candidate)?,
            signal_micros: candidate.signal_micros(),
            entry_micros: candidate.entry_micros(),
            occupied_through_micros: candidate.occupied_through_micros(),
            disposition_through_micros,
            candidate_ordinal: usize_u64(candidate_ordinal, "candidate ordinal")?,
            admitted_strategy_digest,
        };
        value.validate_without_replay_id()?;
        Ok(value)
    }

    fn validate_without_replay_id(self) -> Result<(), GlobalReplayRefusal> {
        if usize::from(self.stream_ordinal) >= MAX_STREAMS || self.rank == 0 || self.rank > 25 {
            return Err("global replay decision has invalid stream/rank".to_owned());
        }
        validate_rung(self.rung_seconds)?;
        require_digest("decision strategy", &self.strategy_digest)?;
        require_digest("decision candidate", &self.candidate_digest)?;
        if self.occupied_through_micros < self.entry_micros
            || self.signal_micros >= self.entry_micros
        {
            return Err("decision has impossible signal/entry/occupancy timestamps".to_owned());
        }
        match self.disposition {
            DispositionV1::Admitted => {
                if self.disposition_through_micros != self.occupied_through_micros
                    || self.admitted_strategy_digest != [0; 32]
                {
                    return Err("admitted decision has contradictory fields".to_owned());
                }
            }
            DispositionV1::BlockedOccupied => {
                if self.disposition_through_micros < self.entry_micros
                    || self.admitted_strategy_digest != [0; 32]
                {
                    return Err("occupied-block decision has contradictory fields".to_owned());
                }
            }
            DispositionV1::BlockedSimultaneous => {
                if self.disposition_through_micros != i64::MIN {
                    return Err("simultaneous block carries an occupied-through value".to_owned());
                }
                require_digest(
                    "simultaneous admitted strategy",
                    &self.admitted_strategy_digest,
                )?;
            }
        }
        Ok(())
    }

    fn validate(self) -> Result<(), GlobalReplayRefusal> {
        require_digest("decision replay identity", &self.replay_id)?;
        self.validate_without_replay_id()
    }

    fn payload(self) -> Result<[u8; DECISION_PAYLOAD_BYTES], GlobalReplayRefusal> {
        self.validate()?;
        let mut raw = [0; DECISION_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.bytes(&self.replay_id)?;
        encoder.u64(self.sequence)?;
        encoder.u16(self.stream_ordinal)?;
        encoder.u16(self.rank)?;
        encoder.u32(self.rung_seconds)?;
        encoder.u8(family_byte(self.family))?;
        encoder.u8(trade_direction_byte(self.direction))?;
        encoder.u8(disposition_byte(self.disposition))?;
        encoder.u8(path_byte(self.path))?;
        encoder.zeros(4)?;
        encoder.bytes(&self.strategy_digest)?;
        encoder.bytes(&self.candidate_digest)?;
        encoder.i64(self.signal_micros)?;
        encoder.i64(self.entry_micros)?;
        encoder.i64(self.occupied_through_micros)?;
        encoder.i64(self.disposition_through_micros)?;
        encoder.u64(self.candidate_ordinal)?;
        encoder.bytes(&self.admitted_strategy_digest)?;
        encoder.zeros(16)?;
        encoder.finish()?;
        Ok(raw)
    }

    fn to_bytes(self) -> Result<[u8; GLOBAL_REPLAY_DECISION_STRIDE], GlobalReplayRefusal> {
        with_seal(self.payload()?)
    }

    fn from_bytes(raw: &[u8; GLOBAL_REPLAY_DECISION_STRIDE]) -> Result<Self, GlobalReplayRefusal> {
        let payload = checked_payload::<DECISION_PAYLOAD_BYTES, GLOBAL_REPLAY_DECISION_STRIDE>(
            raw,
            "global replay decision",
        )?;
        let mut decoder = Decoder::new(&payload);
        let value = Self {
            replay_id: decoder.array_32()?,
            sequence: decoder.u64()?,
            stream_ordinal: decoder.u16()?,
            rank: decoder.u16()?,
            rung_seconds: decoder.u32()?,
            family: decode_family(decoder.u8()?)?,
            direction: decode_trade_direction(decoder.u8()?)?,
            disposition: decode_disposition(decoder.u8()?)?,
            path: decode_path(decoder.u8()?)?,
            strategy_digest: {
                decoder.zeros(4, "decision tag reserve")?;
                decoder.array_32()?
            },
            candidate_digest: decoder.array_32()?,
            signal_micros: decoder.i64()?,
            entry_micros: decoder.i64()?,
            occupied_through_micros: decoder.i64()?,
            disposition_through_micros: decoder.i64()?,
            candidate_ordinal: decoder.u64()?,
            admitted_strategy_digest: decoder.array_32()?,
        };
        decoder.zeros(16, "decision trailing reserve")?;
        decoder.finish()?;
        value.validate()?;
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VixPairV1 {
    entry: VixStamp,
    exit: VixStamp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TradeRecordV1 {
    replay_id: [u8; 32],
    decision_sequence: u64,
    stream_ordinal: u16,
    rank: u16,
    rung_seconds: u32,
    family: InstrumentFamilyV1,
    direction: TradeDirectionV1,
    strategy_digest: [u8; 32],
    row: TradeRow,
    vix: VixPairV1,
    ambiguous_bars: u64,
    gap_fills: u64,
}

impl TradeRecordV1 {
    fn from_admitted(
        decision_sequence: u64,
        stream: &StreamRecordV1,
        price: ReplayPriceV1,
        vix: VixPairV1,
    ) -> Result<Self, GlobalReplayRefusal> {
        let value = Self {
            replay_id: [0; 32],
            decision_sequence,
            stream_ordinal: stream.stream_ordinal,
            rank: stream.rank,
            rung_seconds: stream.rung_seconds,
            family: stream.family,
            direction: stream.direction,
            strategy_digest: stream.strategy_digest,
            row: price.row(),
            vix,
            ambiguous_bars: price.ambiguous_bars(),
            gap_fills: price.gap_fills(),
        };
        value.validate_without_replay_id()?;
        Ok(value)
    }

    fn validate_without_replay_id(self) -> Result<(), GlobalReplayRefusal> {
        if usize::from(self.stream_ordinal) >= MAX_STREAMS || self.rank == 0 || self.rank > 25 {
            return Err("global replay trade has invalid stream/rank".to_owned());
        }
        validate_rung(self.rung_seconds)?;
        require_digest("trade strategy", &self.strategy_digest)?;
        validate_trade_row(self.row)?;
        if self.ambiguous_bars > 1 || self.gap_fills > 1 {
            return Err("global replay trade quality flag exceeds one candidate bar".to_owned());
        }
        validate_vix_stamp(self.vix.entry, self.row.entry_micros, "entry")?;
        validate_vix_stamp(self.vix.exit, self.row.exit_micros, "exit")
    }

    fn validate(self) -> Result<(), GlobalReplayRefusal> {
        require_digest("trade replay identity", &self.replay_id)?;
        self.validate_without_replay_id()
    }

    fn payload(self) -> Result<[u8; TRADE_PAYLOAD_BYTES], GlobalReplayRefusal> {
        self.validate()?;
        let mut raw = [0; TRADE_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.bytes(&self.replay_id)?;
        encoder.u64(self.decision_sequence)?;
        encoder.u16(self.stream_ordinal)?;
        encoder.u16(self.rank)?;
        encoder.u32(self.rung_seconds)?;
        encoder.u8(family_byte(self.family))?;
        encoder.u8(trade_direction_byte(self.direction))?;
        encoder.zeros(6)?;
        encoder.bytes(&self.strategy_digest)?;
        encode_trade_row(&mut encoder, self.row)?;
        encode_vix_stamp(&mut encoder, self.vix.entry)?;
        encode_vix_stamp(&mut encoder, self.vix.exit)?;
        encoder.u64(self.ambiguous_bars)?;
        encoder.u64(self.gap_fills)?;
        encoder.finish()?;
        Ok(raw)
    }

    fn to_bytes(self) -> Result<[u8; GLOBAL_REPLAY_TRADE_STRIDE], GlobalReplayRefusal> {
        with_seal(self.payload()?)
    }

    fn from_bytes(raw: &[u8; GLOBAL_REPLAY_TRADE_STRIDE]) -> Result<Self, GlobalReplayRefusal> {
        let payload = checked_payload::<TRADE_PAYLOAD_BYTES, GLOBAL_REPLAY_TRADE_STRIDE>(
            raw,
            "global replay trade",
        )?;
        let mut decoder = Decoder::new(&payload);
        let value = Self {
            replay_id: decoder.array_32()?,
            decision_sequence: decoder.u64()?,
            stream_ordinal: decoder.u16()?,
            rank: decoder.u16()?,
            rung_seconds: decoder.u32()?,
            family: decode_family(decoder.u8()?)?,
            direction: decode_trade_direction(decoder.u8()?)?,
            strategy_digest: {
                decoder.zeros(6, "trade tag reserve")?;
                decoder.array_32()?
            },
            row: decode_trade_row(&mut decoder)?,
            vix: VixPairV1 {
                entry: decode_vix_stamp(&mut decoder)?,
                exit: decode_vix_stamp(&mut decoder)?,
            },
            ambiguous_bars: decoder.u64()?,
            gap_fills: decoder.u64()?,
        };
        decoder.finish()?;
        value.validate()?;
        Ok(value)
    }
}

fn validate_trade_row(row: TradeRow) -> Result<(), GlobalReplayRefusal> {
    if row.signal_bar >= row.entry_bar
        || row.entry_bar > row.exit_bar
        || row.entry_micros > row.exit_micros
        || row.adverse < 0
        || row.adverse_paisa < 0
        || row.favourable < 0
        || row.favourable_paisa < 0
    {
        return Err("global replay money row has impossible coordinates or excursions".to_owned());
    }
    Ok(())
}

fn encode_trade_row(encoder: &mut Encoder<'_>, row: TradeRow) -> Result<(), GlobalReplayRefusal> {
    validate_trade_row(row)?;
    encoder.u64(usize_u64(row.signal_bar, "trade signal bar")?)?;
    encoder.u64(usize_u64(row.entry_bar, "trade entry bar")?)?;
    encoder.u64(usize_u64(row.exit_bar, "trade exit bar")?)?;
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
        encoder.i64(value)?;
    }
    Ok(())
}

fn decode_trade_row(decoder: &mut Decoder<'_>) -> Result<TradeRow, GlobalReplayRefusal> {
    let row = TradeRow {
        signal_bar: u64_usize(decoder.u64()?, "trade signal bar")?,
        entry_bar: u64_usize(decoder.u64()?, "trade entry bar")?,
        exit_bar: u64_usize(decoder.u64()?, "trade exit bar")?,
        best: decoder.i64()?,
        worst: decoder.i64()?,
        entry_micros: decoder.i64()?,
        exit_micros: decoder.i64()?,
        adverse: decoder.i64()?,
        adverse_paisa: decoder.i64()?,
        favourable: decoder.i64()?,
        favourable_paisa: decoder.i64()?,
    };
    validate_trade_row(row)?;
    Ok(row)
}

fn encode_vix_stamp(encoder: &mut Encoder<'_>, stamp: VixStamp) -> Result<(), GlobalReplayRefusal> {
    match stamp {
        VixStamp::Absent => {
            encoder.u8(0)?;
            encoder.zeros(63)
        }
        VixStamp::Exact(candle) => {
            encoder.u8(1)?;
            encoder.zeros(7)?;
            for value in [
                candle.ts_micros,
                candle.open,
                candle.high,
                candle.low,
                candle.close,
                candle.volume,
                candle.open_interest,
            ] {
                encoder.i64(value)?;
            }
            Ok(())
        }
    }
}

fn decode_vix_stamp(decoder: &mut Decoder<'_>) -> Result<VixStamp, GlobalReplayRefusal> {
    match decoder.u8()? {
        0 => {
            decoder.zeros(63, "absent VIX stamp payload")?;
            Ok(VixStamp::Absent)
        }
        1 => {
            decoder.zeros(7, "exact VIX stamp reserve")?;
            Ok(VixStamp::Exact(Candle {
                ts_micros: decoder.i64()?,
                open: decoder.i64()?,
                high: decoder.i64()?,
                low: decoder.i64()?,
                close: decoder.i64()?,
                volume: decoder.i64()?,
                open_interest: decoder.i64()?,
            }))
        }
        tag => Err(format!("VIX stamp tag {tag} is unknown")),
    }
}

fn validate_vix_stamp(
    stamp: VixStamp,
    expected_micros: i64,
    leg: &str,
) -> Result<(), GlobalReplayRefusal> {
    let VixStamp::Exact(candle) = stamp else {
        return Ok(());
    };
    if candle.ts_micros != expected_micros {
        return Err(format!(
            "{leg} VIX stamp timestamp {} differs from exact trade timestamp {expected_micros}",
            candle.ts_micros
        ));
    }
    if candle.open <= 0
        || candle.high < candle.open.max(candle.close)
        || candle.low > candle.open.min(candle.close)
        || candle.low <= 0
        || candle.volume < 0
        || (candle.open_interest < 0 && candle.open_interest != i64::MIN)
    {
        return Err(format!(
            "{leg} VIX stamp contains malformed stored OHLCV/OI"
        ));
    }
    Ok(())
}

struct VixCatalogV1<'a> {
    months: HashMap<YearMonth, &'a VixReferenceMonth>,
}

impl<'a> VixCatalogV1<'a> {
    fn new(months: &'a [VixReferenceMonth]) -> Result<Self, GlobalReplayRefusal> {
        let mut indexed = HashMap::new();
        indexed
            .try_reserve(months.len())
            .map_err(|why| format!("India VIX month index allocation refused: {why}"))?;
        let mut vendor = None;
        for month in months {
            if indexed.insert(month.month(), month).is_some() {
                return Err(format!(
                    "India VIX month {} was supplied more than once",
                    month.month()
                ));
            }
            if vendor.is_some_and(|held| held != month.vendor()) {
                return Err("India VIX catalog mixes vendor authorities".to_owned());
            }
            vendor = Some(month.vendor());
        }
        Ok(Self { months: indexed })
    }

    fn stamps(&self, feed: &str, row: TradeRow) -> Result<VixPairV1, GlobalReplayRefusal> {
        Ok(VixPairV1 {
            entry: self.stamp(feed, row.entry_micros)?,
            exit: self.stamp(feed, row.exit_micros)?,
        })
    }

    fn stamp(&self, feed: &str, ts_micros: i64) -> Result<VixStamp, GlobalReplayRefusal> {
        let wanted = year_month_of(ts_micros)?;
        let month = self.months.get(&wanted).copied().ok_or_else(|| {
            format!(
                "globally admitted trade at {ts_micros} has no loaded India VIX authority for {wanted}"
            )
        })?;
        if month.vendor().as_str() != feed {
            return Err(format!(
                "India VIX feed {} differs from execution feed {feed}",
                month.vendor().as_str()
            ));
        }
        month
            .stamp(ts_micros)
            .map_err(|why| format!("India VIX exact stamp refused: {why}"))
    }
}

fn year_month_of(ts_micros: i64) -> Result<YearMonth, GlobalReplayRefusal> {
    if ts_micros.rem_euclid(60_000_000) != 0 {
        return Err(format!(
            "trade timestamp {ts_micros} is off the one-minute grid"
        ));
    }
    let moment = IstMoment::from_epoch_secs(ts_micros.div_euclid(1_000_000))
        .map_err(|why| format!("trade timestamp {ts_micros} has no IST minute: {why}"))?;
    moment
        .day()
        .year_month()
        .map_err(|why| format!("trade timestamp {ts_micros} has no store month: {why}"))
}

/// Receipt-last authority for one completely reconciled global replay publication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobalReplayCompletionV1 {
    publication_id: [u8; 32],
    replay_id: [u8; 32],
    manifest: ReplayManifestV1,
    stream_first: u64,
    stream_count: u64,
    decision_first: u64,
    decision_count: u64,
    trade_first: u64,
    trade_count: u64,
    counters: Counters,
    pricing_refused_candidates: u64,
    admitted_pricing_refused: u64,
    ordered_stream_digest: [u8; 32],
    ordered_decision_digest: [u8; 32],
    ordered_trade_digest: [u8; 32],
}

impl GlobalReplayCompletionV1 {
    /// Publication identity including exact-or-absent VIX stamps.
    #[must_use]
    pub const fn publication_id(&self) -> [u8; 32] {
        self.publication_id
    }

    /// Execution-only replay identity; India VIX is deliberately absent.
    #[must_use]
    pub const fn replay_id(&self) -> [u8; 32] {
        self.replay_id
    }

    /// Number of selected strategy streams.
    #[must_use]
    pub const fn stream_count(&self) -> u64 {
        self.stream_count
    }

    /// Number of pre-exclusivity candidate decisions.
    #[must_use]
    pub const fn decision_count(&self) -> u64 {
        self.decision_count
    }

    /// Number of globally admitted exact money rows.
    #[must_use]
    pub const fn trade_count(&self) -> u64 {
        self.trade_count
    }

    /// Exhaustive global scheduler counters.
    #[must_use]
    pub const fn counters(&self) -> Counters {
        self.counters
    }

    /// Admitted reachable paths that retained occupancy without invented P&L.
    #[must_use]
    pub const fn admitted_pricing_refused(&self) -> u64 {
        self.admitted_pricing_refused
    }

    /// All reachable paths whose exact money pricing was refused.
    #[must_use]
    pub const fn pricing_refused_candidates(&self) -> u64 {
        self.pricing_refused_candidates
    }

    /// Identity of the exact eight selections and sixteen execution authorities.
    #[must_use]
    pub const fn manifest_digest(&self) -> [u8; 32] {
        self.manifest.manifest_digest
    }

    /// Exact canonical Selection V3 identities in rung order.
    #[must_use]
    pub const fn selection_ids(&self) -> [[u8; 32]; 8] {
        self.manifest.selection_ids
    }

    /// Exact execution-completion authorities in rung then family order.
    #[must_use]
    pub const fn execution_authority_ids(&self) -> [[u8; 32]; 16] {
        self.manifest.execution_authority_ids
    }

    fn for_prepared(
        prepared: &PreparedGlobalReplayV1,
        stream_first: u64,
        decision_first: u64,
        trade_first: u64,
    ) -> Result<Self, GlobalReplayRefusal> {
        let receipt = Self {
            publication_id: prepared.publication_id,
            replay_id: prepared.replay_id,
            manifest: prepared.manifest.clone(),
            stream_first,
            stream_count: usize_u64(prepared.streams.len(), "stream block")?,
            decision_first,
            decision_count: usize_u64(prepared.decisions.len(), "decision block")?,
            trade_first,
            trade_count: usize_u64(prepared.trades.len(), "trade block")?,
            counters: prepared.counters,
            pricing_refused_candidates: prepared.pricing_refused_candidates,
            admitted_pricing_refused: prepared.admitted_pricing_refused,
            ordered_stream_digest: prepared.ordered_stream_digest,
            ordered_decision_digest: prepared.ordered_decision_digest,
            ordered_trade_digest: prepared.ordered_trade_digest,
        };
        receipt.validate()?;
        Ok(receipt)
    }

    fn validate(&self) -> Result<(), GlobalReplayRefusal> {
        self.manifest.validate()?;
        for (name, digest) in [
            ("completion publication", self.publication_id),
            ("completion replay", self.replay_id),
            ("completion stream order", self.ordered_stream_digest),
            ("completion decision order", self.ordered_decision_digest),
            ("completion trade order", self.ordered_trade_digest),
        ] {
            require_digest(name, &digest)?;
        }
        for (first, count, name) in [
            (self.stream_first, self.stream_count, "stream"),
            (self.decision_first, self.decision_count, "decision"),
            (self.trade_first, self.trade_count, "trade"),
        ] {
            first
                .checked_add(count)
                .ok_or_else(|| format!("global replay {name} block range overflows u64"))?;
        }
        if self.stream_count != usize_u64(MAX_STREAMS, "maximum global replay streams")? {
            return Err(
                "global replay completion does not contain the exact 8 x 25 stream surface"
                    .to_owned(),
            );
        }
        let priceable_admissions = self
            .counters
            .admitted
            .checked_sub(self.admitted_pricing_refused)
            .ok_or_else(|| {
                "completion admitted pricing-refused count exceeds admissions".to_owned()
            })?;
        if !self.counters.reconciles()
            || self.counters.offered != self.decision_count
            || self.counters.unreachable != 0
            || self.counters.refused != 0
            || self.pricing_refused_candidates > self.decision_count
            || self.admitted_pricing_refused > self.pricing_refused_candidates
            || self.admitted_pricing_refused > self.counters.admitted
            || self.trade_count != priceable_admissions
        {
            return Err("global replay completion counts do not reconcile".to_owned());
        }
        if self.replay_id
            != derive_replay_id(
                &self.manifest,
                self.ordered_stream_digest,
                self.ordered_decision_digest,
                self.counters,
                self.pricing_refused_candidates,
                self.admitted_pricing_refused,
            )
        {
            return Err("global replay identity differs from execution-only fields".to_owned());
        }
        if self.publication_id != derive_publication_id(self.replay_id, self.ordered_trade_digest) {
            return Err(
                "global replay publication identity differs from VIX-bound trade rows".to_owned(),
            );
        }
        Ok(())
    }

    fn payload(&self) -> Result<[u8; COMPLETION_PAYLOAD_BYTES], GlobalReplayRefusal> {
        self.validate()?;
        let mut raw = [0; COMPLETION_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.bytes(&self.publication_id)?;
        encoder.bytes(&self.replay_id)?;
        encoder.bytes(&self.manifest.manifest_digest)?;
        encoder.bytes(&self.manifest.common_cohort_digest)?;
        for digest in self.manifest.selection_ids {
            encoder.bytes(&digest)?;
        }
        for digest in self.manifest.execution_authority_ids {
            encoder.bytes(&digest)?;
        }
        for value in [
            self.stream_first,
            self.stream_count,
            self.decision_first,
            self.decision_count,
            self.trade_first,
            self.trade_count,
            self.counters.offered,
            self.counters.admitted,
            self.counters.blocked_occupied,
            self.counters.blocked_simultaneous,
            self.counters.unreachable,
            self.counters.refused,
            self.pricing_refused_candidates,
            self.admitted_pricing_refused,
        ] {
            encoder.u64(value)?;
        }
        encoder.bytes(&self.ordered_stream_digest)?;
        encoder.bytes(&self.ordered_decision_digest)?;
        encoder.bytes(&self.ordered_trade_digest)?;
        encoder.bytes(&exact_execution_law_digest_v1())?;
        encoder.bytes(&vix_stamp_policy_digest())?;
        encoder.zeros(48)?;
        encoder.finish()?;
        Ok(raw)
    }

    fn to_bytes(&self) -> Result<[u8; GLOBAL_REPLAY_COMPLETION_STRIDE], GlobalReplayRefusal> {
        with_seal(self.payload()?)
    }

    fn from_bytes(
        raw: &[u8; GLOBAL_REPLAY_COMPLETION_STRIDE],
    ) -> Result<Self, GlobalReplayRefusal> {
        let payload = checked_payload::<COMPLETION_PAYLOAD_BYTES, GLOBAL_REPLAY_COMPLETION_STRIDE>(
            raw,
            "global replay completion",
        )?;
        let mut decoder = Decoder::new(&payload);
        let publication_id = decoder.array_32()?;
        let replay_id = decoder.array_32()?;
        let manifest_digest = decoder.array_32()?;
        let common_cohort_digest = decoder.array_32()?;
        let mut selection_ids = [[0; 32]; 8];
        for digest in &mut selection_ids {
            *digest = decoder.array_32()?;
        }
        let mut execution_authority_ids = [[0; 32]; 16];
        for digest in &mut execution_authority_ids {
            *digest = decoder.array_32()?;
        }
        let value = Self {
            publication_id,
            replay_id,
            manifest: ReplayManifestV1 {
                manifest_digest,
                common_cohort_digest,
                selection_ids,
                execution_authority_ids,
            },
            stream_first: decoder.u64()?,
            stream_count: decoder.u64()?,
            decision_first: decoder.u64()?,
            decision_count: decoder.u64()?,
            trade_first: decoder.u64()?,
            trade_count: decoder.u64()?,
            counters: Counters {
                offered: decoder.u64()?,
                admitted: decoder.u64()?,
                blocked_occupied: decoder.u64()?,
                blocked_simultaneous: decoder.u64()?,
                unreachable: decoder.u64()?,
                refused: decoder.u64()?,
            },
            pricing_refused_candidates: decoder.u64()?,
            admitted_pricing_refused: decoder.u64()?,
            ordered_stream_digest: decoder.array_32()?,
            ordered_decision_digest: decoder.array_32()?,
            ordered_trade_digest: decoder.array_32()?,
        };
        if decoder.array_32()? != exact_execution_law_digest_v1() {
            return Err("global replay completion execution law is unknown".to_owned());
        }
        if decoder.array_32()? != vix_stamp_policy_digest() {
            return Err("global replay completion VIX stamp policy is unknown".to_owned());
        }
        decoder.zeros(48, "completion trailing reserve")?;
        decoder.finish()?;
        value.validate()?;
        Ok(value)
    }
}

impl PreparedGlobalReplayV1 {
    fn validate(&self) -> Result<(), GlobalReplayRefusal> {
        self.manifest.validate()?;
        let priceable_admissions = self
            .counters
            .admitted
            .checked_sub(self.admitted_pricing_refused)
            .ok_or_else(|| {
                "prepared admitted pricing-refused count exceeds admissions".to_owned()
            })?;
        if self.streams.len() != MAX_STREAMS
            || !self.counters.reconciles()
            || self.counters.offered != usize_u64(self.decisions.len(), "decision count")?
            || self.counters.unreachable != 0
            || self.counters.refused != 0
            || usize_u64(self.trades.len(), "trade count")? != priceable_admissions
        {
            return Err("prepared global replay counts do not reconcile".to_owned());
        }
        validate_manifest_streams(&self.manifest, &self.streams)?;
        for (index, stream) in self.streams.iter().copied().enumerate() {
            stream.validate()?;
            if stream.replay_id != self.replay_id || usize::from(stream.stream_ordinal) != index {
                return Err("prepared global replay stream identity/order mismatch".to_owned());
            }
        }
        for (index, decision) in self.decisions.iter().copied().enumerate() {
            decision.validate()?;
            if decision.replay_id != self.replay_id
                || decision.sequence != usize_u64(index, "decision sequence")?
            {
                return Err("prepared global replay decision identity/order mismatch".to_owned());
            }
        }
        for trade in self.trades.iter().copied() {
            trade.validate()?;
            if trade.replay_id != self.replay_id {
                return Err("prepared global replay trade identity mismatch".to_owned());
            }
        }
        if self.ordered_stream_digest != stream_order_digest(&self.streams)?
            || self.ordered_decision_digest != decision_order_digest(&self.decisions)?
            || self.ordered_trade_digest != trade_order_digest(&self.trades)?
            || self.replay_id
                != derive_replay_id(
                    &self.manifest,
                    self.ordered_stream_digest,
                    self.ordered_decision_digest,
                    self.counters,
                    self.pricing_refused_candidates,
                    self.admitted_pricing_refused,
                )
            || self.publication_id
                != derive_publication_id(self.replay_id, self.ordered_trade_digest)
        {
            return Err("prepared global replay digests do not reproduce".to_owned());
        }
        validate_committed_blocks(&self.streams, &self.decisions, &self.trades, self.counters)
    }
}

fn derive_replay_id(
    manifest: &ReplayManifestV1,
    ordered_stream_digest: [u8; 32],
    ordered_decision_digest: [u8; 32],
    counters: Counters,
    pricing_refused_candidates: u64,
    admitted_pricing_refused: u64,
) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(REPLAY_ID_DOMAIN);
    hasher.update(&manifest.manifest_digest);
    hasher.update(&ordered_stream_digest);
    hasher.update(&ordered_decision_digest);
    for value in [
        counters.offered,
        counters.admitted,
        counters.blocked_occupied,
        counters.blocked_simultaneous,
        counters.unreachable,
        counters.refused,
        pricing_refused_candidates,
        admitted_pricing_refused,
    ] {
        hasher.update(&value.to_le_bytes());
    }
    hasher.update(&exact_execution_law_digest_v1());
    hasher.finalize()
}

fn derive_publication_id(replay_id: [u8; 32], ordered_trade_digest: [u8; 32]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(PUBLICATION_ID_DOMAIN);
    hasher.update(&replay_id);
    hasher.update(&ordered_trade_digest);
    hasher.update(&vix_stamp_policy_digest());
    hasher.finalize()
}

fn vix_stamp_policy_digest() -> [u8; 32] {
    blake3::hash(VIX_STAMP_POLICY_DOMAIN)
}

fn stream_order_digest(records: &[StreamRecordV1]) -> Result<[u8; 32], GlobalReplayRefusal> {
    let mut hasher = Hasher::new();
    hasher.update(STREAM_ORDER_DOMAIN);
    hasher.update(&usize_u64(records.len(), "stream order")?.to_le_bytes());
    for record in records.iter().copied() {
        record.validate_without_replay_id()?;
        let mut canonical = record;
        canonical.replay_id = [1; 32];
        let payload = canonical.payload()?;
        hasher.update(
            payload
                .get(32..)
                .ok_or_else(|| "stream semantic payload is absent".to_owned())?,
        );
    }
    Ok(hasher.finalize())
}

fn decision_order_digest(records: &[DecisionRecordV1]) -> Result<[u8; 32], GlobalReplayRefusal> {
    let mut hasher = Hasher::new();
    hasher.update(DECISION_ORDER_DOMAIN);
    hasher.update(&usize_u64(records.len(), "decision order")?.to_le_bytes());
    for record in records.iter().copied() {
        record.validate_without_replay_id()?;
        let mut canonical = record;
        canonical.replay_id = [1; 32];
        let payload = canonical.payload()?;
        hasher.update(
            payload
                .get(32..)
                .ok_or_else(|| "decision semantic payload is absent".to_owned())?,
        );
    }
    Ok(hasher.finalize())
}

fn trade_order_digest(records: &[TradeRecordV1]) -> Result<[u8; 32], GlobalReplayRefusal> {
    let mut hasher = Hasher::new();
    hasher.update(TRADE_ORDER_DOMAIN);
    hasher.update(&usize_u64(records.len(), "trade order")?.to_le_bytes());
    for record in records.iter().copied() {
        hasher.update(&record.payload()?);
    }
    Ok(hasher.finalize())
}

fn validate_manifest_streams(
    manifest: &ReplayManifestV1,
    streams: &[StreamRecordV1],
) -> Result<(), GlobalReplayRefusal> {
    if streams.len() != MAX_STREAMS {
        return Err(format!(
            "global replay manifest requires exactly {MAX_STREAMS} ordered streams, found {}",
            streams.len()
        ));
    }
    for (index, stream) in streams.iter().copied().enumerate() {
        let rung_slot = index / 25;
        let rank_slot = index % 25;
        let expected_rung = CANONICAL_RUNGS_SECONDS
            .get(rung_slot)
            .copied()
            .ok_or_else(|| "global replay stream rung slot is absent".to_owned())?;
        let expected_selection = manifest
            .selection_ids
            .get(rung_slot)
            .copied()
            .ok_or_else(|| "global replay stream selection slot is absent".to_owned())?;
        let family_slot = match stream.family {
            InstrumentFamilyV1::Nifty => 0_usize,
            InstrumentFamilyV1::BankNifty => 1_usize,
        };
        let authority_slot = rung_slot
            .checked_mul(2)
            .and_then(|offset| offset.checked_add(family_slot))
            .ok_or_else(|| "global replay stream authority slot overflow".to_owned())?;
        let expected_authority = manifest
            .execution_authority_ids
            .get(authority_slot)
            .copied()
            .ok_or_else(|| "global replay stream authority slot is absent".to_owned())?;
        let expected_rank = u16::try_from(
            rank_slot
                .checked_add(1)
                .ok_or_else(|| "global replay canonical rank overflow".to_owned())?,
        )
        .map_err(|_| "global replay canonical rank does not fit u16".to_owned())?;
        if usize::from(stream.stream_ordinal) != index
            || stream.rank != expected_rank
            || stream.rung_seconds != expected_rung
            || stream.selection_id != expected_selection
            || stream.execution_authority_id != expected_authority
        {
            return Err(format!(
                "global replay stream {index} differs from its exact rung/rank/family manifest slot"
            ));
        }
    }
    Ok(())
}

fn validate_committed_blocks(
    streams: &[StreamRecordV1],
    decisions: &[DecisionRecordV1],
    trades: &[TradeRecordV1],
    expected_counters: Counters,
) -> Result<(), GlobalReplayRefusal> {
    if streams.len() != MAX_STREAMS {
        return Err(
            "committed global replay does not contain the exact 8 x 25 stream surface".to_owned(),
        );
    }
    let replay_id = streams
        .first()
        .map(|record| record.replay_id)
        .or_else(|| decisions.first().map(|record| record.replay_id))
        .or_else(|| trades.first().map(|record| record.replay_id));
    let mut expected_candidates = 0_u64;
    let mut expected_pricing_refused = 0_u64;
    for (index, stream) in streams.iter().copied().enumerate() {
        stream.validate()?;
        if usize::from(stream.stream_ordinal) != index
            || replay_id.is_some_and(|held| held != stream.replay_id)
        {
            return Err("committed global replay stream block is reordered or foreign".to_owned());
        }
        expected_candidates = expected_candidates
            .checked_add(stream.candidate_count)
            .ok_or_else(|| "committed candidate count overflow".to_owned())?;
        expected_pricing_refused = expected_pricing_refused
            .checked_add(stream.pricing_refused_paths)
            .ok_or_else(|| "committed pricing-refused count overflow".to_owned())?;
    }
    if expected_candidates != usize_u64(decisions.len(), "decision block")? {
        return Err("stream candidate counts do not reconcile to decision rows".to_owned());
    }
    let mut per_stream = vec![0_u64; streams.len()];
    let mut per_stream_refused = vec![0_u64; streams.len()];
    for (index, decision) in decisions.iter().copied().enumerate() {
        decision.validate()?;
        if decision.sequence != usize_u64(index, "decision sequence")?
            || replay_id.is_some_and(|held| held != decision.replay_id)
        {
            return Err(
                "committed global replay decision block is reordered or foreign".to_owned(),
            );
        }
        let stream = streams
            .get(usize::from(decision.stream_ordinal))
            .ok_or_else(|| "decision names an absent replay stream".to_owned())?;
        if decision.rank != stream.rank
            || decision.rung_seconds != stream.rung_seconds
            || decision.family != stream.family
            || decision.direction != stream.direction
            || decision.strategy_digest != stream.strategy_digest
        {
            return Err("decision differs from its reconstructed stream authority".to_owned());
        }
        let count = per_stream
            .get_mut(usize::from(decision.stream_ordinal))
            .ok_or_else(|| "decision stream counter is absent".to_owned())?;
        if decision.candidate_ordinal != *count {
            return Err("decision candidate ordinals are missing or reordered".to_owned());
        }
        *count = count
            .checked_add(1)
            .ok_or_else(|| "decision stream count overflow".to_owned())?;
        if decision.path != PathTagV1::Priceable {
            let refused = per_stream_refused
                .get_mut(usize::from(decision.stream_ordinal))
                .ok_or_else(|| "decision refusal counter is absent".to_owned())?;
            *refused = refused
                .checked_add(1)
                .ok_or_else(|| "decision refusal count overflow".to_owned())?;
        }
    }
    for (index, stream) in streams.iter().enumerate() {
        if per_stream.get(index).copied() != Some(stream.candidate_count)
            || per_stream_refused.get(index).copied() != Some(stream.pricing_refused_paths)
        {
            return Err(
                "decision rows do not reconcile to per-stream candidate evidence".to_owned(),
            );
        }
    }
    if per_stream_refused.iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(*value)
            .ok_or_else(|| "pricing-refused decision sum overflow".to_owned())
    })? != expected_pricing_refused
    {
        return Err("pricing-refused stream and decision counts disagree".to_owned());
    }

    replay_decision_block(streams, decisions, expected_counters)?;
    validate_trade_block(streams, decisions, trades)
}

fn replay_decision_block(
    streams: &[StreamRecordV1],
    decisions: &[DecisionRecordV1],
    expected_counters: Counters,
) -> Result<(), GlobalReplayRefusal> {
    let mut scheduler = GlobalSinglePositionV1::new();
    let mut at = 0_usize;
    while at < decisions.len() {
        let entry = decisions
            .get(at)
            .ok_or_else(|| "decision group start is absent".to_owned())?
            .entry_micros;
        let mut end = at;
        while decisions
            .get(end)
            .is_some_and(|record| record.entry_micros == entry)
        {
            end = end
                .checked_add(1)
                .ok_or_else(|| "decision minute-group cursor overflow".to_owned())?;
        }
        let group = decisions
            .get(at..end)
            .ok_or_else(|| "decision minute group is absent".to_owned())?;
        let mut intents = Vec::new();
        intents
            .try_reserve_exact(group.len())
            .map_err(|why| format!("decision replay intent allocation refused: {why}"))?;
        for decision in group {
            let stream = streams
                .get(usize::from(decision.stream_ordinal))
                .ok_or_else(|| "decision replay stream is absent".to_owned())?;
            intents.push(Intent {
                constituent: constituent_of_record(stream)?,
                evidence: Evidence::Reachable {
                    occupied_through_micros: decision.occupied_through_micros,
                },
            });
        }
        let replayed = scheduler
            .schedule_minute(entry, &intents)
            .map_err(|why| format!("persisted decision minute {entry} refuses replay: {why:?}"))?;
        if replayed.decisions().count() != group.len() {
            return Err("persisted decision minute lost a scheduler outcome".to_owned());
        }
        for (actual, stored) in replayed.decisions().zip(group) {
            if actual.constituent
                != constituent_of_record(
                    streams
                        .get(usize::from(stored.stream_ordinal))
                        .ok_or_else(|| "stored decision stream is absent".to_owned())?,
                )?
                || !disposition_matches(stored, actual.disposition)
            {
                return Err(
                    "persisted decision differs from a fresh global scheduler replay".to_owned(),
                );
            }
        }
        at = end;
    }
    if scheduler.counters() != expected_counters {
        return Err("persisted scheduler counters differ from replayed decisions".to_owned());
    }
    Ok(())
}

fn constituent_of_record(stream: &StreamRecordV1) -> Result<Constituent, GlobalReplayRefusal> {
    Ok(Constituent {
        priority: stream.rank,
        strategy_digest: StrategyDigest::new(stream.strategy_digest),
        instrument: instrument_of(stream.family)?,
        direction: direction_of(stream.direction),
        rung_minutes: u16::try_from(stream.rung_seconds / 60)
            .map_err(|_| "persisted rung minutes do not fit u16".to_owned())?,
    })
}

fn disposition_matches(stored: &DecisionRecordV1, actual: Disposition) -> bool {
    match (stored.disposition, actual) {
        (
            DispositionV1::Admitted,
            Disposition::Admitted {
                occupied_through_micros,
            },
        )
        | (
            DispositionV1::BlockedOccupied,
            Disposition::BlockedOccupied {
                occupied_through_micros,
            },
        ) => stored.disposition_through_micros == occupied_through_micros,
        (DispositionV1::BlockedSimultaneous, Disposition::BlockedSimultaneous { admitted }) => {
            stored.admitted_strategy_digest == admitted.bytes()
        }
        _ => false,
    }
}

fn validate_trade_block(
    streams: &[StreamRecordV1],
    decisions: &[DecisionRecordV1],
    trades: &[TradeRecordV1],
) -> Result<(), GlobalReplayRefusal> {
    let mut trade_at = 0_usize;
    for decision in decisions {
        let needs_trade = decision.disposition == DispositionV1::Admitted
            && decision.path == PathTagV1::Priceable;
        if !needs_trade {
            continue;
        }
        let trade = trades
            .get(trade_at)
            .copied()
            .ok_or_else(|| "priceable global admission has no money row".to_owned())?;
        trade.validate()?;
        let stream = streams
            .get(usize::from(decision.stream_ordinal))
            .ok_or_else(|| "trade decision stream is absent".to_owned())?;
        if trade.decision_sequence != decision.sequence
            || trade.stream_ordinal != decision.stream_ordinal
            || trade.rank != stream.rank
            || trade.rung_seconds != stream.rung_seconds
            || trade.family != stream.family
            || trade.direction != stream.direction
            || trade.strategy_digest != stream.strategy_digest
            || trade.row.signal_bar >= trade.row.entry_bar
            || trade.row.entry_micros != decision.entry_micros
            || trade.row.exit_micros != decision.occupied_through_micros
            || digest_candidate_parts(
                trade.row.signal_bar,
                trade.row.entry_bar,
                trade.row.exit_bar,
                decision.signal_micros,
                decision.entry_micros,
                decision.occupied_through_micros,
                PathTagV1::Priceable,
                Some((trade.row, trade.ambiguous_bars, trade.gap_fills)),
            )? != decision.candidate_digest
        {
            return Err(
                "global replay money row differs from its admitted candidate decision".to_owned(),
            );
        }
        trade_at = trade_at
            .checked_add(1)
            .ok_or_else(|| "global replay trade cursor overflow".to_owned())?;
    }
    if trade_at != trades.len() {
        return Err("pricing-refused or blocked candidate acquired a money row".to_owned());
    }
    Ok(())
}

fn digest_candidate(candidate: ReplayCandidateV1) -> Result<[u8; 32], GlobalReplayRefusal> {
    let priced = match candidate.path() {
        ReplayPathV1::Priceable(price) => {
            Some((price.row(), price.ambiguous_bars(), price.gap_fills()))
        }
        ReplayPathV1::BlockOnly
        | ReplayPathV1::CrossingRefused
        | ReplayPathV1::BlockOnlyAndCrossingRefused => None,
    };
    digest_candidate_parts(
        candidate.signal_bar(),
        candidate.entry_bar(),
        candidate.occupied_through_bar(),
        candidate.signal_micros(),
        candidate.entry_micros(),
        candidate.occupied_through_micros(),
        path_tag(candidate.path()),
        priced,
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "the digest binds each independent candidate coordinate, timestamp, path and optional price fact"
)]
fn digest_candidate_parts(
    signal_bar: usize,
    entry_bar: usize,
    occupied_through_bar: usize,
    signal_micros: i64,
    entry_micros: i64,
    occupied_through_micros: i64,
    path: PathTagV1,
    priced: Option<(TradeRow, u64, u64)>,
) -> Result<[u8; 32], GlobalReplayRefusal> {
    let mut hasher = Hasher::new();
    hasher.update(CANDIDATE_DOMAIN);
    for value in [signal_bar, entry_bar, occupied_through_bar] {
        hasher.update(&usize_u64(value, "candidate bar")?.to_le_bytes());
    }
    for value in [signal_micros, entry_micros, occupied_through_micros] {
        hasher.update(&value.to_le_bytes());
    }
    hasher.update(&[path_byte(path)]);
    match (path, priced) {
        (PathTagV1::Priceable, Some((row, ambiguous, gaps))) => {
            validate_trade_row(row)?;
            hasher.update(&[1]);
            put_trade_row(&mut hasher, row)?;
            hasher.update(&ambiguous.to_le_bytes());
            hasher.update(&gaps.to_le_bytes());
        }
        (PathTagV1::Priceable, None) => {
            return Err("priceable candidate digest is missing its money evidence".to_owned());
        }
        (_, Some(_)) => {
            return Err("pricing-refused candidate digest was offered a money row".to_owned());
        }
        (_, None) => hasher.update(&[0]),
    }
    Ok(hasher.finalize())
}

fn put_trade_row(hasher: &mut Hasher, row: TradeRow) -> Result<(), GlobalReplayRefusal> {
    for value in [row.signal_bar, row.entry_bar, row.exit_bar] {
        hasher.update(&usize_u64(value, "candidate trade bar")?.to_le_bytes());
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

/// Outcome of an idempotent receipt-last global replay commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobalReplayCommitV1 {
    /// New stream/decision/trade blocks and completion were synced.
    Written,
    /// The exact publication was already committed and was fully revalidated.
    Reused,
}

/// Append-only global replay stream/decision/trade/completion authority.
#[derive(Debug)]
pub struct GlobalReplayLedger {
    stream_file: File,
    decision_file: File,
    trade_file: File,
    completion_file: File,
    writer_lock: File,
    paths: ReplayPaths,
    file_digests: [[u8; 32]; 4],
    completions: HashMap<[u8; 32], GlobalReplayCompletionV1>,
    latest_by_replay: HashMap<[u8; 32], [u8; 32]>,
    order: Vec<[u8; 32]>,
    max_completions: usize,
    writable: bool,
}

impl GlobalReplayLedger {
    /// Stream-record path.
    #[must_use]
    pub fn stream_path(root: &Path) -> PathBuf {
        root.join("results/global-replay-streams-v1.bin")
    }

    /// Decision-record path.
    #[must_use]
    pub fn decision_path(root: &Path) -> PathBuf {
        root.join("results/global-replay-decisions-v1.bin")
    }

    /// Admitted-money-record path.
    #[must_use]
    pub fn trade_path(root: &Path) -> PathBuf {
        root.join("results/global-replay-trades-v1.bin")
    }

    /// Receipt-last completion path.
    #[must_use]
    pub fn completion_path(root: &Path) -> PathBuf {
        root.join("results/global-replay-completions-v1.bin")
    }

    /// Shared writer-lock path for all four global replay files.
    #[must_use]
    pub fn lock_path(root: &Path) -> PathBuf {
        root.join("results/global-replay-write.lock")
    }

    /// Open a writable ledger under an explicit completion ceiling.
    ///
    /// # Errors
    ///
    /// Refuses zero/insufficient bounds, malformed files, invalid headers,
    /// ragged records, bad seals, overlapping completion blocks, any scheduler
    /// replay disagreement, duplicate identities, allocation, lock or I/O
    /// failure.
    pub fn open(root: &Path, max_completions: usize) -> Result<Self, GlobalReplayRefusal> {
        validate_completion_bound(max_completions)?;
        fs::create_dir_all(root.join("results")).map_err(|why| {
            format!(
                "{} result directory could not be created: {why}",
                root.join("results").display()
            )
        })?;
        let paths = ReplayPaths::of(root);
        let writer_lock = open_or_create(&paths.lock)?;
        writer_lock
            .lock()
            .map_err(|why| format!("{} could not be locked: {why}", paths.lock.display()))?;
        let opened = (|| {
            let mut files = ReplayFiles {
                stream: open_or_create(&paths.stream)?,
                decision: open_or_create(&paths.decision)?,
                trade: open_or_create(&paths.trade)?,
                completion: open_or_create(&paths.completion)?,
            };
            ensure_header(
                &mut files.stream,
                &paths.stream,
                STREAM_MAGIC,
                GLOBAL_REPLAY_STREAM_STRIDE,
            )?;
            ensure_header(
                &mut files.decision,
                &paths.decision,
                DECISION_MAGIC,
                GLOBAL_REPLAY_DECISION_STRIDE,
            )?;
            ensure_header(
                &mut files.trade,
                &paths.trade,
                TRADE_MAGIC,
                GLOBAL_REPLAY_TRADE_STRIDE,
            )?;
            ensure_header(
                &mut files.completion,
                &paths.completion,
                COMPLETION_MAGIC,
                GLOBAL_REPLAY_COMPLETION_STRIDE,
            )?;
            Self::from_files(
                files,
                paths.clone(),
                writer_lock.try_clone().map_err(|why| {
                    format!("global replay writer lock could not be cloned: {why}")
                })?,
                max_completions,
                true,
            )
        })();
        let released = writer_lock
            .unlock()
            .map_err(|why| format!("{} could not be unlocked: {why}", paths.lock.display()));
        match (opened, released) {
            (Ok(ledger), Ok(())) => Ok(ledger),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// Open and fully validate an existing ledger without write permission.
    ///
    /// # Errors
    ///
    /// Refuses an absent lock or data file, invalid bound/header/record/seal,
    /// partial or overlapping receipt block, scheduler replay disagreement,
    /// duplicate identity, allocation, locking or I/O failure.
    pub fn open_read(root: &Path, max_completions: usize) -> Result<Self, GlobalReplayRefusal> {
        validate_completion_bound(max_completions)?;
        let paths = ReplayPaths::of(root);
        let writer_lock = File::open(&paths.lock)
            .map_err(|why| format!("{} could not be opened: {why}", paths.lock.display()))?;
        writer_lock
            .lock_shared()
            .map_err(|why| format!("{} could not be shared-locked: {why}", paths.lock.display()))?;
        let opened = (|| {
            let files = ReplayFiles {
                stream: File::open(&paths.stream).map_err(|why| {
                    format!("{} could not be opened: {why}", paths.stream.display())
                })?,
                decision: File::open(&paths.decision).map_err(|why| {
                    format!("{} could not be opened: {why}", paths.decision.display())
                })?,
                trade: File::open(&paths.trade).map_err(|why| {
                    format!("{} could not be opened: {why}", paths.trade.display())
                })?,
                completion: File::open(&paths.completion).map_err(|why| {
                    format!("{} could not be opened: {why}", paths.completion.display())
                })?,
            };
            Self::from_files(
                files,
                paths.clone(),
                writer_lock.try_clone().map_err(|why| {
                    format!("global replay writer lock could not be cloned: {why}")
                })?,
                max_completions,
                false,
            )
        })();
        let released = writer_lock
            .unlock()
            .map_err(|why| format!("{} could not be unlocked: {why}", paths.lock.display()));
        match (opened, released) {
            (Ok(ledger), Ok(())) => Ok(ledger),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn from_files(
        mut files: ReplayFiles,
        paths: ReplayPaths,
        writer_lock: File,
        max_completions: usize,
        writable: bool,
    ) -> Result<Self, GlobalReplayRefusal> {
        let records = scan_replay_records(&mut files, &paths)?;
        let indexes = index_replay_completions(&records, max_completions)?;
        let file_digests = [
            digest_file(&mut files.stream, &paths.stream)?,
            digest_file(&mut files.decision, &paths.decision)?,
            digest_file(&mut files.trade, &paths.trade)?,
            digest_file(&mut files.completion, &paths.completion)?,
        ];
        Ok(Self {
            stream_file: files.stream,
            decision_file: files.decision,
            trade_file: files.trade,
            completion_file: files.completion,
            writer_lock,
            paths,
            file_digests,
            completions: indexes.completions,
            latest_by_replay: indexes.latest_by_replay,
            order: indexes.order,
            max_completions,
            writable,
        })
    }

    /// Number of fully committed publications.
    #[must_use]
    pub fn completions(&self) -> usize {
        self.order.len()
    }

    /// Publication identities in append order.
    #[must_use]
    pub fn publication_ids(&self) -> &[[u8; 32]] {
        &self.order
    }

    /// Exact publication completion after validated open.
    #[must_use]
    pub fn completion(&self, publication_id: &[u8; 32]) -> Option<&GlobalReplayCompletionV1> {
        self.completions.get(publication_id)
    }

    /// Latest append for one execution-only replay identity.
    #[must_use]
    pub fn latest_for_replay(&self, replay_id: &[u8; 32]) -> Option<&GlobalReplayCompletionV1> {
        self.latest_by_replay
            .get(replay_id)
            .and_then(|publication| self.completions.get(publication))
    }

    /// Append and sync streams, decisions, trades, then the completion last.
    ///
    /// # Errors
    ///
    /// Refuses a read-only or stale handle, invalid prepared replay, exhausted
    /// completion bound, changed file, encoding, locking, sync or I/O failure.
    pub fn append_complete(
        &mut self,
        prepared: &PreparedGlobalReplayV1,
    ) -> Result<GlobalReplayCommitV1, GlobalReplayRefusal> {
        if !self.writable {
            return Err("a read-only global replay ledger cannot append".to_owned());
        }
        prepared.validate()?;
        self.writer_lock
            .lock()
            .map_err(|why| format!("{} could not be locked: {why}", self.paths.lock.display()))?;
        let attempted = self.append_complete_locked(prepared);
        let released = self
            .writer_lock
            .unlock()
            .map_err(|why| format!("{} could not be unlocked: {why}", self.paths.lock.display()));
        match (attempted, released) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn append_complete_locked(
        &mut self,
        prepared: &PreparedGlobalReplayV1,
    ) -> Result<GlobalReplayCommitV1, GlobalReplayRefusal> {
        self.require_files_unchanged()?;
        if self.completions.contains_key(&prepared.publication_id) {
            self.sync_all()?;
            return Ok(GlobalReplayCommitV1::Reused);
        }
        if self.order.len() >= self.max_completions {
            return Err(format!(
                "global replay completion bound {} is exhausted; reopen with an explicit larger bound",
                self.max_completions
            ));
        }
        let receipt = self.completion_for(prepared)?;
        self.append_prepared_files(prepared, &receipt)?;
        self.refresh_file_digests()?;
        self.latest_by_replay
            .insert(receipt.replay_id, receipt.publication_id);
        self.order.push(receipt.publication_id);
        self.completions.insert(receipt.publication_id, receipt);
        Ok(GlobalReplayCommitV1::Written)
    }

    fn completion_for(
        &mut self,
        prepared: &PreparedGlobalReplayV1,
    ) -> Result<GlobalReplayCompletionV1, GlobalReplayRefusal> {
        let stream_first = record_count(
            &mut self.stream_file,
            &self.paths.stream,
            GLOBAL_REPLAY_STREAM_STRIDE,
        )?;
        let decision_first = record_count(
            &mut self.decision_file,
            &self.paths.decision,
            GLOBAL_REPLAY_DECISION_STRIDE,
        )?;
        let trade_first = record_count(
            &mut self.trade_file,
            &self.paths.trade,
            GLOBAL_REPLAY_TRADE_STRIDE,
        )?;
        GlobalReplayCompletionV1::for_prepared(prepared, stream_first, decision_first, trade_first)
    }

    fn append_prepared_files(
        &mut self,
        prepared: &PreparedGlobalReplayV1,
        receipt: &GlobalReplayCompletionV1,
    ) -> Result<(), GlobalReplayRefusal> {
        append_encoded(
            &mut self.stream_file,
            &self.paths.stream,
            prepared
                .streams
                .iter()
                .copied()
                .map(StreamRecordV1::to_bytes),
        )?;
        sync_file(&self.stream_file, &self.paths.stream)?;
        append_encoded(
            &mut self.decision_file,
            &self.paths.decision,
            prepared
                .decisions
                .iter()
                .copied()
                .map(DecisionRecordV1::to_bytes),
        )?;
        sync_file(&self.decision_file, &self.paths.decision)?;
        append_encoded(
            &mut self.trade_file,
            &self.paths.trade,
            prepared.trades.iter().copied().map(TradeRecordV1::to_bytes),
        )?;
        sync_file(&self.trade_file, &self.paths.trade)?;
        append_encoded(
            &mut self.completion_file,
            &self.paths.completion,
            core::iter::once(receipt.to_bytes()),
        )?;
        sync_file(&self.completion_file, &self.paths.completion)
    }

    fn require_files_unchanged(&mut self) -> Result<(), GlobalReplayRefusal> {
        let observed = [
            digest_file(&mut self.stream_file, &self.paths.stream)?,
            digest_file(&mut self.decision_file, &self.paths.decision)?,
            digest_file(&mut self.trade_file, &self.paths.trade)?,
            digest_file(&mut self.completion_file, &self.paths.completion)?,
        ];
        if observed != self.file_digests {
            return Err(
                "global replay files changed after open; reopen and fully revalidate".to_owned(),
            );
        }
        Ok(())
    }

    fn refresh_file_digests(&mut self) -> Result<(), GlobalReplayRefusal> {
        self.file_digests = [
            digest_file(&mut self.stream_file, &self.paths.stream)?,
            digest_file(&mut self.decision_file, &self.paths.decision)?,
            digest_file(&mut self.trade_file, &self.paths.trade)?,
            digest_file(&mut self.completion_file, &self.paths.completion)?,
        ];
        Ok(())
    }

    fn sync_all(&self) -> Result<(), GlobalReplayRefusal> {
        for (file, path) in [
            (&self.stream_file, &self.paths.stream),
            (&self.decision_file, &self.paths.decision),
            (&self.trade_file, &self.paths.trade),
            (&self.completion_file, &self.paths.completion),
        ] {
            sync_file(file, path)?;
        }
        Ok(())
    }
}

fn sync_file(file: &File, path: &Path) -> Result<(), GlobalReplayRefusal> {
    file.sync_all()
        .map_err(|why| format!("{} could not be synced: {why}", path.display()))
}

struct ReplayFiles {
    stream: File,
    decision: File,
    trade: File,
    completion: File,
}

#[derive(Clone, Debug)]
struct ReplayPaths {
    stream: PathBuf,
    decision: PathBuf,
    trade: PathBuf,
    completion: PathBuf,
    lock: PathBuf,
}

impl ReplayPaths {
    fn of(root: &Path) -> Self {
        Self {
            stream: GlobalReplayLedger::stream_path(root),
            decision: GlobalReplayLedger::decision_path(root),
            trade: GlobalReplayLedger::trade_path(root),
            completion: GlobalReplayLedger::completion_path(root),
            lock: GlobalReplayLedger::lock_path(root),
        }
    }
}

struct ScannedReplayRecordsV1 {
    streams: Vec<StreamRecordV1>,
    decisions: Vec<DecisionRecordV1>,
    trades: Vec<TradeRecordV1>,
    receipts: Vec<GlobalReplayCompletionV1>,
}

struct ReplayCompletionIndexesV1 {
    completions: HashMap<[u8; 32], GlobalReplayCompletionV1>,
    latest_by_replay: HashMap<[u8; 32], [u8; 32]>,
    order: Vec<[u8; 32]>,
}

fn scan_replay_records(
    files: &mut ReplayFiles,
    paths: &ReplayPaths,
) -> Result<ScannedReplayRecordsV1, GlobalReplayRefusal> {
    for (file, path, magic, stride) in [
        (
            &mut files.stream,
            &paths.stream,
            STREAM_MAGIC,
            GLOBAL_REPLAY_STREAM_STRIDE,
        ),
        (
            &mut files.decision,
            &paths.decision,
            DECISION_MAGIC,
            GLOBAL_REPLAY_DECISION_STRIDE,
        ),
        (
            &mut files.trade,
            &paths.trade,
            TRADE_MAGIC,
            GLOBAL_REPLAY_TRADE_STRIDE,
        ),
        (
            &mut files.completion,
            &paths.completion,
            COMPLETION_MAGIC,
            GLOBAL_REPLAY_COMPLETION_STRIDE,
        ),
    ] {
        check_record_file(file, path, magic, stride)?;
    }
    Ok(ScannedReplayRecordsV1 {
        streams: scan_records::<GLOBAL_REPLAY_STREAM_STRIDE, StreamRecordV1>(
            &mut files.stream,
            &paths.stream,
            StreamRecordV1::from_bytes,
        )?,
        decisions: scan_records::<GLOBAL_REPLAY_DECISION_STRIDE, DecisionRecordV1>(
            &mut files.decision,
            &paths.decision,
            DecisionRecordV1::from_bytes,
        )?,
        trades: scan_records::<GLOBAL_REPLAY_TRADE_STRIDE, TradeRecordV1>(
            &mut files.trade,
            &paths.trade,
            TradeRecordV1::from_bytes,
        )?,
        receipts: scan_records::<GLOBAL_REPLAY_COMPLETION_STRIDE, GlobalReplayCompletionV1>(
            &mut files.completion,
            &paths.completion,
            GlobalReplayCompletionV1::from_bytes,
        )?,
    })
}

fn index_replay_completions(
    records: &ScannedReplayRecordsV1,
    max_completions: usize,
) -> Result<ReplayCompletionIndexesV1, GlobalReplayRefusal> {
    if records.receipts.len() > max_completions {
        return Err(format!(
            "global replay ledger has {} completions above caller bound {max_completions}",
            records.receipts.len()
        ));
    }
    let mut completions = HashMap::new();
    completions
        .try_reserve(records.receipts.len())
        .map_err(|why| format!("global replay completion index allocation refused: {why}"))?;
    let mut latest_by_replay = HashMap::new();
    latest_by_replay
        .try_reserve(records.receipts.len())
        .map_err(|why| format!("global replay latest index allocation refused: {why}"))?;
    let mut order = Vec::new();
    order
        .try_reserve_exact(records.receipts.len())
        .map_err(|why| format!("global replay order allocation refused: {why}"))?;
    let mut stream_end = 0_u64;
    let mut decision_end = 0_u64;
    let mut trade_end = 0_u64;
    for receipt in &records.receipts {
        index_one_completion(
            records,
            receipt,
            &mut stream_end,
            &mut decision_end,
            &mut trade_end,
        )?;
        if completions
            .insert(receipt.publication_id, receipt.clone())
            .is_some()
        {
            return Err("duplicate global replay publication completion".to_owned());
        }
        latest_by_replay.insert(receipt.replay_id, receipt.publication_id);
        order.push(receipt.publication_id);
    }
    Ok(ReplayCompletionIndexesV1 {
        completions,
        latest_by_replay,
        order,
    })
}

fn index_one_completion(
    records: &ScannedReplayRecordsV1,
    receipt: &GlobalReplayCompletionV1,
    stream_end: &mut u64,
    decision_end: &mut u64,
    trade_end: &mut u64,
) -> Result<(), GlobalReplayRefusal> {
    require_monotonic_block(
        receipt.stream_first,
        receipt.stream_count,
        stream_end,
        "global replay stream",
    )?;
    require_monotonic_block(
        receipt.decision_first,
        receipt.decision_count,
        decision_end,
        "global replay decision",
    )?;
    require_monotonic_block(
        receipt.trade_first,
        receipt.trade_count,
        trade_end,
        "global replay trade",
    )?;
    let stream_block = block_slice(
        &records.streams,
        receipt.stream_first,
        receipt.stream_count,
        "global replay stream",
    )?;
    let decision_block = block_slice(
        &records.decisions,
        receipt.decision_first,
        receipt.decision_count,
        "global replay decision",
    )?;
    let trade_block = block_slice(
        &records.trades,
        receipt.trade_first,
        receipt.trade_count,
        "global replay trade",
    )?;
    validate_receipt_blocks(receipt, stream_block, decision_block, trade_block)
}

fn validate_receipt_blocks(
    receipt: &GlobalReplayCompletionV1,
    streams: &[StreamRecordV1],
    decisions: &[DecisionRecordV1],
    trades: &[TradeRecordV1],
) -> Result<(), GlobalReplayRefusal> {
    receipt.validate()?;
    validate_manifest_streams(&receipt.manifest, streams)?;
    validate_committed_blocks(streams, decisions, trades, receipt.counters)?;
    if stream_order_digest(streams)? != receipt.ordered_stream_digest
        || decision_order_digest(decisions)? != receipt.ordered_decision_digest
        || trade_order_digest(trades)? != receipt.ordered_trade_digest
        || streams
            .iter()
            .any(|record| record.replay_id != receipt.replay_id)
        || decisions
            .iter()
            .any(|record| record.replay_id != receipt.replay_id)
        || trades
            .iter()
            .any(|record| record.replay_id != receipt.replay_id)
    {
        return Err(
            "global replay completion block digest or identity differs from its records".to_owned(),
        );
    }
    let pricing_refused = decisions.iter().try_fold(0_u64, |count, decision| {
        if decision.path == PathTagV1::Priceable {
            Ok(count)
        } else {
            count
                .checked_add(1)
                .ok_or_else(|| "pricing-refused decision count overflow".to_owned())
        }
    })?;
    let admitted_pricing_refused = decisions.iter().try_fold(0_u64, |count, decision| {
        if decision.disposition == DispositionV1::Admitted && decision.path != PathTagV1::Priceable
        {
            count
                .checked_add(1)
                .ok_or_else(|| "admitted pricing-refused count overflow".to_owned())
        } else {
            Ok(count)
        }
    })?;
    if pricing_refused != receipt.pricing_refused_candidates
        || admitted_pricing_refused != receipt.admitted_pricing_refused
    {
        return Err(
            "global replay completion pricing-refusal counts differ from decisions".to_owned(),
        );
    }
    Ok(())
}

fn validate_completion_bound(max_completions: usize) -> Result<(), GlobalReplayRefusal> {
    if max_completions == 0 {
        Err("global replay completion bound must be nonzero".to_owned())
    } else {
        Ok(())
    }
}

fn open_or_create(path: &Path) -> Result<File, GlobalReplayRefusal> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|why| format!("{} could not be opened: {why}", path.display()))
}

fn ensure_header(
    file: &mut File,
    path: &Path,
    magic: [u8; 8],
    stride: usize,
) -> Result<(), GlobalReplayRefusal> {
    let len = measured_len(file, path)?;
    if len != 0 {
        return Ok(());
    }
    let raw = canonical_header(magic, stride)?;
    file.write_all(&raw)
        .and_then(|()| file.sync_all())
        .map_err(|why| {
            format!(
                "{} could not receive a durable header: {why}",
                path.display()
            )
        })
}

fn canonical_header(
    magic: [u8; 8],
    stride: usize,
) -> Result<[u8; HEADER_BYTES_USIZE], GlobalReplayRefusal> {
    let mut raw = [0; HEADER_BYTES_USIZE];
    let mut encoder = Encoder::new(&mut raw);
    encoder.bytes(&magic)?;
    encoder.u32(FORMAT_VERSION)?;
    encoder.u32(
        u32::try_from(HEADER_BYTES_USIZE)
            .map_err(|_| "global replay header width does not fit u32".to_owned())?,
    )?;
    encoder.u32(
        u32::try_from(stride)
            .map_err(|_| "global replay record stride does not fit u32".to_owned())?,
    )?;
    encoder.zeros(4)?;
    encoder.finish()?;
    Ok(raw)
}

fn check_record_file(
    file: &mut File,
    path: &Path,
    magic: [u8; 8],
    stride: usize,
) -> Result<(), GlobalReplayRefusal> {
    let len = measured_len(file, path)?;
    if len < HEADER_BYTES {
        return Err(format!(
            "{} is {len} bytes, shorter than the {HEADER_BYTES}-byte header",
            path.display()
        ));
    }
    let payload = len
        .checked_sub(HEADER_BYTES)
        .ok_or_else(|| "global replay file length underflow".to_owned())?;
    let stride_u64 = usize_u64(stride, "global replay stride")?;
    if payload % stride_u64 != 0 {
        return Err(format!(
            "{} has a ragged {payload}-byte record body for stride {stride}",
            path.display()
        ));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("{} could not be seeked: {why}", path.display()))?;
    let mut raw = [0; HEADER_BYTES_USIZE];
    file.read_exact(&mut raw)
        .map_err(|why| format!("{} header could not be read: {why}", path.display()))?;
    if raw != canonical_header(magic, stride)? {
        return Err(format!("{} header fields are noncanonical", path.display()));
    }
    Ok(())
}

fn measured_len(file: &File, path: &Path) -> Result<u64, GlobalReplayRefusal> {
    file.metadata()
        .map(|metadata| metadata.len())
        .map_err(|why| format!("{} metadata could not be read: {why}", path.display()))
}

fn record_count(file: &mut File, path: &Path, stride: usize) -> Result<u64, GlobalReplayRefusal> {
    let len = measured_len(file, path)?;
    let body = len
        .checked_sub(HEADER_BYTES)
        .ok_or_else(|| format!("{} is shorter than its header", path.display()))?;
    Ok(body / usize_u64(stride, "global replay record stride")?)
}

fn scan_records<const STRIDE: usize, T>(
    file: &mut File,
    path: &Path,
    decode: fn(&[u8; STRIDE]) -> Result<T, GlobalReplayRefusal>,
) -> Result<Vec<T>, GlobalReplayRefusal> {
    let count = record_count(file, path, STRIDE)?;
    let capacity = u64_usize(count, "global replay record count")?;
    let mut records = Vec::new();
    records
        .try_reserve_exact(capacity)
        .map_err(|why| format!("{} record allocation refused: {why}", path.display()))?;
    file.seek(SeekFrom::Start(HEADER_BYTES))
        .map_err(|why| format!("{} could not be seeked: {why}", path.display()))?;
    for sequence in 0..count {
        let mut raw = [0; STRIDE];
        file.read_exact(&mut raw).map_err(|why| {
            format!(
                "{} record {sequence} could not be read completely: {why}",
                path.display()
            )
        })?;
        records
            .push(decode(&raw).map_err(|why| {
                format!("{} record {sequence} is invalid: {why}", path.display())
            })?);
    }
    Ok(records)
}

fn append_encoded<const STRIDE: usize>(
    file: &mut File,
    path: &Path,
    records: impl IntoIterator<Item = Result<[u8; STRIDE], GlobalReplayRefusal>>,
) -> Result<(), GlobalReplayRefusal> {
    file.seek(SeekFrom::End(0))
        .map_err(|why| format!("{} could not be seeked for append: {why}", path.display()))?;
    for record in records {
        let raw = record?;
        file.write_all(&raw)
            .map_err(|why| format!("{} append failed: {why}", path.display()))?;
    }
    Ok(())
}

fn digest_file(file: &mut File, path: &Path) -> Result<[u8; 32], GlobalReplayRefusal> {
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("{} could not be seeked for hashing: {why}", path.display()))?;
    let mut hasher = Hasher::new();
    let mut buffer = [0_u8; 16_384];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|why| format!("{} could not be hashed: {why}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(
            buffer
                .get(..read)
                .ok_or_else(|| "file hash read width is outside its buffer".to_owned())?,
        );
    }
    Ok(hasher.finalize())
}

fn require_monotonic_block(
    first: u64,
    count: u64,
    prior_end: &mut u64,
    subject: &str,
) -> Result<(), GlobalReplayRefusal> {
    if first < *prior_end {
        return Err(format!(
            "{subject} completion blocks overlap or run backwards"
        ));
    }
    *prior_end = first
        .checked_add(count)
        .ok_or_else(|| format!("{subject} completion block overflows u64"))?;
    Ok(())
}

fn block_slice<'a, T>(
    records: &'a [T],
    first: u64,
    count: u64,
    subject: &str,
) -> Result<&'a [T], GlobalReplayRefusal> {
    let first = u64_usize(first, subject)?;
    let count = u64_usize(count, subject)?;
    let end = first
        .checked_add(count)
        .ok_or_else(|| format!("{subject} block address overflow"))?;
    records
        .get(first..end)
        .ok_or_else(|| format!("{subject} completion points outside its record file"))
}

fn with_seal<const PAYLOAD: usize, const STRIDE: usize>(
    payload: [u8; PAYLOAD],
) -> Result<[u8; STRIDE], GlobalReplayRefusal> {
    if PAYLOAD.checked_add(SEAL_BYTES) != Some(STRIDE) {
        return Err("global replay payload/stride compile-time contract differs".to_owned());
    }
    let mut raw = [0; STRIDE];
    raw.get_mut(..PAYLOAD)
        .ok_or_else(|| "global replay payload slot is absent".to_owned())?
        .copy_from_slice(&payload);
    raw.get_mut(PAYLOAD..)
        .ok_or_else(|| "global replay seal slot is absent".to_owned())?
        .copy_from_slice(&blake3::hash(&payload));
    Ok(raw)
}

fn checked_payload<const PAYLOAD: usize, const STRIDE: usize>(
    raw: &[u8; STRIDE],
    subject: &str,
) -> Result<[u8; PAYLOAD], GlobalReplayRefusal> {
    if PAYLOAD.checked_add(SEAL_BYTES) != Some(STRIDE) {
        return Err(format!("{subject} payload/stride contract differs"));
    }
    let payload: [u8; PAYLOAD] = raw
        .get(..PAYLOAD)
        .ok_or_else(|| format!("{subject} payload is absent"))?
        .try_into()
        .map_err(|_| format!("{subject} payload width differs"))?;
    let seal = raw
        .get(PAYLOAD..)
        .ok_or_else(|| format!("{subject} seal is absent"))?;
    if seal != blake3::hash(&payload) {
        return Err(format!("{subject} failed its complete BLAKE3 seal"));
    }
    Ok(payload)
}

fn path_tag(path: ReplayPathV1) -> PathTagV1 {
    match path {
        ReplayPathV1::Priceable(_) => PathTagV1::Priceable,
        ReplayPathV1::BlockOnly => PathTagV1::BlockOnly,
        ReplayPathV1::CrossingRefused => PathTagV1::CrossingRefused,
        ReplayPathV1::BlockOnlyAndCrossingRefused => PathTagV1::BlockOnlyAndCrossingRefused,
    }
}

const fn path_byte(path: PathTagV1) -> u8 {
    match path {
        PathTagV1::Priceable => 1,
        PathTagV1::BlockOnly => 2,
        PathTagV1::CrossingRefused => 3,
        PathTagV1::BlockOnlyAndCrossingRefused => 4,
    }
}

fn decode_path(byte: u8) -> Result<PathTagV1, GlobalReplayRefusal> {
    match byte {
        1 => Ok(PathTagV1::Priceable),
        2 => Ok(PathTagV1::BlockOnly),
        3 => Ok(PathTagV1::CrossingRefused),
        4 => Ok(PathTagV1::BlockOnlyAndCrossingRefused),
        _ => Err(format!(
            "global replay candidate-path tag {byte} is unknown"
        )),
    }
}

const fn disposition_byte(disposition: DispositionV1) -> u8 {
    match disposition {
        DispositionV1::Admitted => 1,
        DispositionV1::BlockedOccupied => 2,
        DispositionV1::BlockedSimultaneous => 3,
    }
}

fn decode_disposition(byte: u8) -> Result<DispositionV1, GlobalReplayRefusal> {
    match byte {
        1 => Ok(DispositionV1::Admitted),
        2 => Ok(DispositionV1::BlockedOccupied),
        3 => Ok(DispositionV1::BlockedSimultaneous),
        _ => Err(format!("global replay disposition tag {byte} is unknown")),
    }
}

const fn family_byte(family: InstrumentFamilyV1) -> u8 {
    match family {
        InstrumentFamilyV1::Nifty => 1,
        InstrumentFamilyV1::BankNifty => 2,
    }
}

fn decode_family(byte: u8) -> Result<InstrumentFamilyV1, GlobalReplayRefusal> {
    match byte {
        1 => Ok(InstrumentFamilyV1::Nifty),
        2 => Ok(InstrumentFamilyV1::BankNifty),
        _ => Err(format!(
            "global replay instrument-family tag {byte} is unknown"
        )),
    }
}

const fn trade_direction_byte(direction: TradeDirectionV1) -> u8 {
    match direction {
        TradeDirectionV1::Long => 1,
        TradeDirectionV1::Short => 2,
    }
}

fn decode_trade_direction(byte: u8) -> Result<TradeDirectionV1, GlobalReplayRefusal> {
    match byte {
        1 => Ok(TradeDirectionV1::Long),
        2 => Ok(TradeDirectionV1::Short),
        _ => Err(format!(
            "global replay trade-direction tag {byte} is unknown"
        )),
    }
}

const fn direction_of(direction: TradeDirectionV1) -> Direction {
    match direction {
        TradeDirectionV1::Long => Direction::Long,
        TradeDirectionV1::Short => Direction::Short,
    }
}

fn instrument_of(family: InstrumentFamilyV1) -> Result<InstrumentKey, GlobalReplayRefusal> {
    InstrumentKey::index(
        Exchange::Nse,
        match family {
            InstrumentFamilyV1::Nifty => "NIFTY",
            InstrumentFamilyV1::BankNifty => "BANKNIFTY",
        },
    )
    .map_err(|why| format!("canonical swept instrument could not be built: {why}"))
}

fn validate_rung(rung_seconds: u32) -> Result<(), GlobalReplayRefusal> {
    if matches!(
        rung_seconds,
        60 | 120 | 180 | 300 | 600 | 900 | 1_800 | 3_600
    ) {
        Ok(())
    } else {
        Err(format!(
            "global replay rung {rung_seconds}s is outside the eight canonical intraday rungs"
        ))
    }
}

fn require_digest(name: &str, digest: &[u8; 32]) -> Result<(), GlobalReplayRefusal> {
    if *digest == [0; 32] {
        Err(format!("{name} cannot be all zero"))
    } else {
        Ok(())
    }
}

fn usize_u64(value: usize, subject: &str) -> Result<u64, GlobalReplayRefusal> {
    u64::try_from(value).map_err(|_| format!("{subject} does not fit u64"))
}

fn u64_usize(value: u64, subject: &str) -> Result<usize, GlobalReplayRefusal> {
    usize::try_from(value).map_err(|_| format!("{subject} does not fit usize"))
}

fn hex(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(64);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

struct Encoder<'a> {
    bytes: &'a mut [u8],
    cursor: usize,
}

impl<'a> Encoder<'a> {
    const fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), GlobalReplayRefusal> {
        let end = self
            .cursor
            .checked_add(value.len())
            .ok_or_else(|| "global replay encoder cursor overflow".to_owned())?;
        self.bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "global replay encoder exceeded its fixed payload".to_owned())?
            .copy_from_slice(value);
        self.cursor = end;
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), GlobalReplayRefusal> {
        self.bytes(&[value])
    }

    fn u16(&mut self, value: u16) -> Result<(), GlobalReplayRefusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn u32(&mut self, value: u32) -> Result<(), GlobalReplayRefusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), GlobalReplayRefusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), GlobalReplayRefusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), GlobalReplayRefusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "global replay encoder reserve overflow".to_owned())?;
        let slot = self
            .bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "global replay encoder reserve exceeded payload".to_owned())?;
        slot.fill(0);
        self.cursor = end;
        Ok(())
    }

    fn finish(self) -> Result<(), GlobalReplayRefusal> {
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err(format!(
                "global replay encoder used {} of {} payload bytes",
                self.cursor,
                self.bytes.len()
            ))
        }
    }
}

struct Decoder<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Decoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], GlobalReplayRefusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "global replay decoder cursor overflow".to_owned())?;
        let slot = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| "global replay decoder exceeded fixed payload".to_owned())?;
        self.cursor = end;
        Ok(slot)
    }

    fn u8(&mut self) -> Result<u8, GlobalReplayRefusal> {
        self.take(1)?
            .first()
            .copied()
            .ok_or_else(|| "global replay u8 is absent".to_owned())
    }

    fn u16(&mut self) -> Result<u16, GlobalReplayRefusal> {
        let raw: [u8; 2] = self
            .take(2)?
            .try_into()
            .map_err(|_| "global replay u16 width differs".to_owned())?;
        Ok(u16::from_le_bytes(raw))
    }

    fn u32(&mut self) -> Result<u32, GlobalReplayRefusal> {
        let raw: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| "global replay u32 width differs".to_owned())?;
        Ok(u32::from_le_bytes(raw))
    }

    fn u64(&mut self) -> Result<u64, GlobalReplayRefusal> {
        let raw: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| "global replay u64 width differs".to_owned())?;
        Ok(u64::from_le_bytes(raw))
    }

    fn i64(&mut self) -> Result<i64, GlobalReplayRefusal> {
        let raw: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| "global replay i64 width differs".to_owned())?;
        Ok(i64::from_le_bytes(raw))
    }

    fn array_32(&mut self) -> Result<[u8; 32], GlobalReplayRefusal> {
        self.take(32)?
            .try_into()
            .map_err(|_| "global replay digest width differs".to_owned())
    }

    fn zeros(&mut self, count: usize, subject: &str) -> Result<(), GlobalReplayRefusal> {
        if self.take(count)?.iter().any(|byte| *byte != 0) {
            Err(format!("{subject} is nonzero"))
        } else {
            Ok(())
        }
    }

    fn finish(self) -> Result<(), GlobalReplayRefusal> {
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err(format!(
                "global replay decoder consumed {} of {} payload bytes",
                self.cursor,
                self.bytes.len()
            ))
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "fixed-layout corruption fixtures use exact offsets and assertions that must be able to fail"
)]
mod tests {
    use std::fs::OpenOptions;
    use std::io::{Seek, SeekFrom, Write};
    use std::sync::atomic::{AtomicU64, Ordering};

    use brutex_core::instrument::InstrumentKey;
    use brutex_core::vendor::Vendor;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use runner::excursion::Side;
    use runner::exit_grid_policy::{
        ExecutionResolutionV1, ExitGridPolicyV1, ExitGridSelectorV1, ForcedStopV1,
        RangeResolutionV1, RatioLimitsV1, RationalPercentileV1, ResolvedExitGridV1, RungPlanV1,
        SelectedExitV1, printed_ohlcv_cost_model_id_v1,
    };
    use runner::grid::Chosen;
    use runner::identity::{Direction as RunDirection, Params, Run, data_digest};
    use runner::outcome::Horizon;
    use runner::synthetic::{DAY_MICROS, sessions};
    use runner::topn::SCORE_SCALE;
    use store::file::BarFile;
    use store::path::{FileKind, StorePath, Timeframe};

    use crate::execution_capability::{
        canonical_execution_capabilities_fixture_v1, canonical_execution_parameters_fixture_v1,
    };
    use crate::population::{
        AdmissionStatusV1, AdmissionV1, ClosureV1, ExitCoordinateV1, TopMetricsV1,
    };
    use crate::selection::{SelectedEntryV1, SharedCohortIdentityV2};
    use crate::selection_v3::{
        CanonicalSelectionPopulationFixtureV3, canonical_selection_receipt_fixture_v3,
    };

    use super::*;

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);
    const TEST_VENDOR: Vendor = Vendor::Dhan;

    fn root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "brutex-global-replay-{name}-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    const fn digest(seed: u8) -> [u8; 32] {
        [seed; 32]
    }

    fn indexed_digest(domain: u8, index: usize) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(&[domain]);
        hasher.update(
            &u64::try_from(index)
                .expect("small fixture index")
                .to_le_bytes(),
        );
        hasher.finalize()
    }

    fn test_column(bars: &[Candle]) -> Column {
        let widths = Widths::pinned().expect("pinned evaluator widths");
        let mut evaluator = Evaluator::new(widths, Availability::Absent, Thresholds::default());
        Column::build(bars, &mut evaluator)
    }

    fn busiest_shared_bit(training: &Column, oos: &Column) -> [u64; 6] {
        let mut best = None;
        for word in 0..6 {
            for bit in 0..64 {
                let value = 1_u64 << bit;
                let training_hits = training
                    .bits()
                    .iter()
                    .filter(|mask| runner::replay_mask::stored_words(mask)[word] & value != 0)
                    .count();
                let oos_hits = oos
                    .bits()
                    .iter()
                    .filter(|mask| runner::replay_mask::stored_words(mask)[word] & value != 0)
                    .count();
                let shared = training_hits.min(oos_hits);
                if shared > best.map_or(0, |(_, _, held)| held) {
                    best = Some((word, bit, shared));
                }
            }
        }
        let (word, bit, hits) = best.expect("the evaluator emits one shared live condition");
        assert!(hits > 5, "the replay condition must produce scheduler work");
        let mut words = [0_u64; 6];
        words[word] = 1_u64 << bit;
        words
    }

    fn policy(side: Side) -> ExitGridPolicyV1 {
        let middle = RationalPercentileV1::new(1, 2).expect("one reduced percentile");
        let rungs = RungPlanV1::new(vec![middle], vec![middle], vec![middle], 1)
            .expect("one exact rung per axis");
        ExitGridPolicyV1::new(
            ExecutionResolutionV1::OneMinuteOhlcv,
            RangeResolutionV1::PpmCeiling,
            side,
            rungs,
            RatioLimitsV1::new(1, 10_000, 4).expect("broad exact ratio interval"),
            16,
            ExitGridSelectorV1::GuaranteedFloor,
            printed_ohlcv_cost_model_id_v1(),
            ForcedStopV1::Disabled,
            u64::MAX,
            u64::MAX,
        )
        .expect("complete replay fixture policy")
    }

    fn execution_run(
        instrument: &InstrumentKey,
        mask_words: [u64; 6],
        side: Side,
        bars: &[Candle],
    ) -> ExecutionRunV1 {
        let direction = match side {
            Side::Long => RunDirection::Long,
            Side::Short => RunDirection::Short,
        };
        let mask = runner::replay_mask::from_stored_words(mask_words)
            .expect("fixture condition is a canonical live mask");
        ExecutionRunV1::new(
            &Run {
                mask,
                direction,
                instrument,
                timeframe: "1min",
                params: Params {
                    min_hits: 1,
                    ceiling: 1,
                    pair_budget: 1,
                    policy: 1,
                },
                data_digest: data_digest(bars),
                commit: "canonical-test-commit",
                feed: TEST_VENDOR.as_str(),
            },
            bars,
            None,
        )
        .expect("canonical execution run")
    }

    fn resolved_and_selected(
        instrument: &InstrumentKey,
        policy: &ExitGridPolicyV1,
        series: ExecutionSeriesV1<'_>,
        column: &Column,
        mask_words: [u64; 6],
        horizon: Horizon,
    ) -> (ResolvedExitGridV1, SelectedExitV1, ExecutionRunV1) {
        let resolved = policy
            .resolve_attested(series)
            .expect("exact synthetic training resolution");
        let run = execution_run(instrument, mask_words, resolved.side(), series.bars());
        let evaluated = resolved
            .evaluate_training_grid_attested(series, column, horizon, run)
            .expect("complete training grid");
        let selected = resolved
            .select(&evaluated)
            .expect("valid complete grid")
            .expect("the shared condition produces one admitted exit");
        (resolved, selected, run)
    }

    fn exit_coordinate(chosen: Chosen) -> ExitCoordinateV1 {
        ExitCoordinateV1 {
            stop: chosen
                .stop
                .map(|index| u32::try_from(index).expect("fixture stop index")),
            target: chosen
                .target
                .map(|index| u32::try_from(index).expect("fixture target index")),
            tsl: chosen
                .tsl
                .map(|index| u32::try_from(index).expect("fixture TSL index")),
            ttp: chosen.ttp.map(|ttp| {
                (
                    u32::try_from(ttp.arm).expect("fixture TTP arm index"),
                    u32::try_from(ttp.trail).expect("fixture TTP trail index"),
                )
            }),
        }
    }

    fn population_row(
        population_id: [u8; 32],
        sequence: usize,
        family: InstrumentFamilyV1,
        rung_seconds: u32,
        mask_words: [u64; 6],
        selected: &SelectedExitV1,
        strategy_index: usize,
    ) -> PopulationRowV1 {
        let direction = if sequence.is_multiple_of(2) {
            TradeDirectionV1::Long
        } else {
            TradeDirectionV1::Short
        };
        PopulationRowV1 {
            population_id,
            sequence: u64::try_from(sequence).expect("fixture row sequence"),
            strategy_digest: indexed_digest(210, strategy_index),
            mask_words,
            direction,
            instrument_family: family,
            closure: ClosureV1::Closed,
            rung_seconds,
            support_hits: 10,
            exit: exit_coordinate(selected.coordinate()),
            metrics: TopMetricsV1 {
                drawdown: 1,
                worst_loss: 1,
                losing_rate_ppm: 500_000,
                losing_trades: 1,
                loss_ratio_ppm: Some(1_000_000),
                pessimistic_profit: 1,
                winning_trades: 1,
                win_rate_ppm: 500_000,
                reward_to_risk_ppm: Some(1_000_000),
                average_win: 1,
                average_loss: 1,
                assurance_ppm: 500_000,
            },
            admission: AdmissionV1 {
                status: AdmissionStatusV1::Admitted,
                reasons: 0,
                failed: 0,
                unmeasured: 0,
                refused: 0,
            },
        }
    }

    fn ordered_population_digest(rows: &[PopulationRowV1]) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(b"brutex.cli.global-replay.canonical-test-population\0");
        for row in rows {
            hasher.update(&row.payload_digest().expect("canonical population row"));
        }
        hasher.finalize()
    }

    fn cohort(
        rung_index: usize,
        long_policy_digest: [u8; 32],
        short_policy_digest: [u8; 32],
    ) -> SharedCohortIdentityV2 {
        let mut bytes = [0_u8; 440];
        bytes[..16].copy_from_slice(b"brutex-cohort-v2");
        bytes[16..20].copy_from_slice(&2_u32.to_le_bytes());
        bytes[20..24].copy_from_slice(&416_u32.to_le_bytes());
        for index in 0..13 {
            let digest = match index {
                9 => long_policy_digest,
                10 => short_policy_digest,
                11 => indexed_digest(211, rung_index),
                _ => indexed_digest(212, index),
            };
            let start = 24 + index * 32;
            bytes[start..start + 32].copy_from_slice(&digest);
        }
        SharedCohortIdentityV2::from_canonical_bytes(&bytes)
            .expect("canonical cross-rung cohort fixture")
    }

    fn manifest() -> ReplayManifestV1 {
        let mut selection_ids = [[0; 32]; 8];
        for (index, slot) in selection_ids.iter_mut().enumerate() {
            *slot = digest(u8::try_from(index).expect("eight slots") + 1);
        }
        let mut execution_authority_ids = [[0; 32]; 16];
        for (index, slot) in execution_authority_ids.iter_mut().enumerate() {
            *slot = digest(u8::try_from(index).expect("sixteen slots") + 20);
        }
        let common_cohort_digest = digest(90);
        ReplayManifestV1 {
            manifest_digest: derive_manifest_digest(
                common_cohort_digest,
                &selection_ids,
                &execution_authority_ids,
            ),
            common_cohort_digest,
            selection_ids,
            execution_authority_ids,
        }
    }

    fn row(signal: usize, entry: usize, exit: usize, entry_micros: i64) -> TradeRow {
        TradeRow {
            signal_bar: signal,
            entry_bar: entry,
            exit_bar: exit,
            best: 400,
            worst: 100,
            entry_micros,
            exit_micros: entry_micros
                + i64::try_from(exit.saturating_sub(entry)).expect("small test bars") * 60_000_000,
            adverse: 1_000,
            adverse_paisa: 20,
            favourable: 2_000,
            favourable_paisa: 40,
        }
    }

    fn fixture_stream(manifest: &ReplayManifestV1, index: usize) -> StreamRecordV1 {
        let rung_slot = index / 25;
        let rank = u16::try_from(index % 25 + 1).expect("fixture rank");
        let family = if index.is_multiple_of(2) {
            InstrumentFamilyV1::Nifty
        } else {
            InstrumentFamilyV1::BankNifty
        };
        let family_slot = match family {
            InstrumentFamilyV1::Nifty => 0,
            InstrumentFamilyV1::BankNifty => 1,
        };
        StreamRecordV1 {
            replay_id: [0; 32],
            stream_ordinal: u16::try_from(index).expect("fixture stream ordinal"),
            rank,
            rung_seconds: CANONICAL_RUNGS_SECONDS[rung_slot],
            family,
            direction: if index.is_multiple_of(3) {
                TradeDirectionV1::Long
            } else {
                TradeDirectionV1::Short
            },
            selection_id: manifest.selection_ids[rung_slot],
            population_id: indexed_digest(100, rung_slot * 2 + family_slot),
            row_sequence: u64::from(rank),
            strategy_digest: indexed_digest(101, index),
            execution_authority_id: manifest.execution_authority_ids[rung_slot * 2 + family_slot],
            execution_capability_id: indexed_digest(102, index),
            training_run_id: indexed_digest(103, index),
            selected_exit_digest: indexed_digest(104, index),
            oos_run_id: indexed_digest(105, index),
            universe_digest: indexed_digest(106, index),
            candidate_count: match index {
                0 => 3,
                _ => 0,
            },
            pricing_refused_paths: match index {
                0 => 1,
                _ => 0,
            },
        }
    }

    fn first_decision(stream: &StreamRecordV1, row: TradeRow) -> DecisionRecordV1 {
        DecisionRecordV1 {
            replay_id: [0; 32],
            sequence: 0,
            stream_ordinal: 0,
            rank: 1,
            rung_seconds: 60,
            family: InstrumentFamilyV1::Nifty,
            direction: TradeDirectionV1::Long,
            disposition: DispositionV1::Admitted,
            path: PathTagV1::Priceable,
            strategy_digest: stream.strategy_digest,
            candidate_digest: digest_candidate_parts(
                0,
                1,
                2,
                0,
                60_000_000,
                120_000_000,
                PathTagV1::Priceable,
                Some((row, 0, 0)),
            )
            .expect("first candidate digest"),
            signal_micros: 0,
            entry_micros: 60_000_000,
            occupied_through_micros: 120_000_000,
            disposition_through_micros: 120_000_000,
            candidate_ordinal: 0,
            admitted_strategy_digest: [0; 32],
        }
    }

    fn occupied_decision(stream: &StreamRecordV1, row: TradeRow) -> DecisionRecordV1 {
        DecisionRecordV1 {
            replay_id: [0; 32],
            sequence: 1,
            stream_ordinal: 0,
            rank: 1,
            rung_seconds: 60,
            family: InstrumentFamilyV1::Nifty,
            direction: TradeDirectionV1::Long,
            disposition: DispositionV1::BlockedOccupied,
            path: PathTagV1::Priceable,
            strategy_digest: stream.strategy_digest,
            candidate_digest: digest_candidate_parts(
                1,
                2,
                3,
                60_000_000,
                120_000_000,
                180_000_000,
                PathTagV1::Priceable,
                Some((row, 0, 0)),
            )
            .expect("blocked candidate digest"),
            signal_micros: 60_000_000,
            entry_micros: 120_000_000,
            occupied_through_micros: 180_000_000,
            disposition_through_micros: 120_000_000,
            candidate_ordinal: 1,
            admitted_strategy_digest: [0; 32],
        }
    }

    fn block_only_decision(stream: &StreamRecordV1) -> DecisionRecordV1 {
        DecisionRecordV1 {
            replay_id: [0; 32],
            sequence: 2,
            stream_ordinal: 0,
            rank: 1,
            rung_seconds: 60,
            family: InstrumentFamilyV1::Nifty,
            direction: TradeDirectionV1::Long,
            disposition: DispositionV1::Admitted,
            path: PathTagV1::BlockOnly,
            strategy_digest: stream.strategy_digest,
            candidate_digest: digest_candidate_parts(
                2,
                3,
                4,
                120_000_000,
                180_000_000,
                240_000_000,
                PathTagV1::BlockOnly,
                None,
            )
            .expect("block-only candidate digest"),
            signal_micros: 120_000_000,
            entry_micros: 180_000_000,
            occupied_through_micros: 240_000_000,
            disposition_through_micros: 240_000_000,
            candidate_ordinal: 2,
            admitted_strategy_digest: [0; 32],
        }
    }

    fn fixture_trade(
        replay_id: [u8; 32],
        stream: &StreamRecordV1,
        first_row: TradeRow,
    ) -> TradeRecordV1 {
        TradeRecordV1 {
            replay_id,
            decision_sequence: 0,
            stream_ordinal: 0,
            rank: 1,
            rung_seconds: 60,
            family: InstrumentFamilyV1::Nifty,
            direction: TradeDirectionV1::Long,
            strategy_digest: stream.strategy_digest,
            row: first_row,
            vix: VixPairV1 {
                entry: VixStamp::Absent,
                exit: VixStamp::Absent,
            },
            ambiguous_bars: 0,
            gap_fills: 0,
        }
    }

    fn prepared() -> PreparedGlobalReplayV1 {
        let manifest = manifest();
        let mut streams: Vec<_> = (0..MAX_STREAMS)
            .map(|index| fixture_stream(&manifest, index))
            .collect();
        let stream = streams[0];
        let first_row = row(0, 1, 2, 60_000_000);
        let mut decisions = vec![
            first_decision(&stream, first_row),
            occupied_decision(&stream, row(1, 2, 3, 120_000_000)),
            block_only_decision(&stream),
        ];
        let counters = Counters {
            offered: 3,
            admitted: 2,
            blocked_occupied: 1,
            blocked_simultaneous: 0,
            unreachable: 0,
            refused: 0,
        };
        let stream_digest = stream_order_digest(&streams).expect("stream digest");
        let decision_digest = decision_order_digest(&decisions).expect("decision digest");
        let replay_id = derive_replay_id(&manifest, stream_digest, decision_digest, counters, 1, 1);
        for stream in &mut streams {
            stream.replay_id = replay_id;
        }
        for decision in &mut decisions {
            decision.replay_id = replay_id;
        }
        let trades = vec![fixture_trade(replay_id, &stream, first_row)];
        let ordered_stream_digest = stream_order_digest(&streams).expect("sealed stream digest");
        let ordered_decision_digest =
            decision_order_digest(&decisions).expect("sealed decision digest");
        assert_eq!(ordered_stream_digest, stream_digest);
        assert_eq!(ordered_decision_digest, decision_digest);
        let ordered_trade_digest = trade_order_digest(&trades).expect("trade digest");
        let publication_id = derive_publication_id(replay_id, ordered_trade_digest);
        let value = PreparedGlobalReplayV1 {
            manifest,
            streams,
            decisions,
            trades,
            counters,
            pricing_refused_candidates: 1,
            admitted_pricing_refused: 1,
            replay_id,
            publication_id,
            ordered_stream_digest,
            ordered_decision_digest,
            ordered_trade_digest,
        };
        value.validate().expect("valid global replay fixture");
        value
    }

    fn reseal<const PAYLOAD: usize, const STRIDE: usize>(raw: &mut [u8; STRIDE]) {
        let seal = blake3::hash(&raw[..PAYLOAD]);
        raw[PAYLOAD..].copy_from_slice(&seal);
    }

    fn cleanup(root: &Path) {
        fs::remove_dir_all(root).expect("unique global replay fixture cleanup");
    }

    struct SideReplayFixture {
        resolved: ResolvedExitGridV1,
        selected: SelectedExitV1,
        training_run: ExecutionRunV1,
        oos_run: ExecutionRunV1,
    }

    struct FamilyReplayFixture {
        family: InstrumentFamilyV1,
        long: SideReplayFixture,
        short: SideReplayFixture,
    }

    impl FamilyReplayFixture {
        fn side(&self, direction: TradeDirectionV1) -> &SideReplayFixture {
            match direction {
                TradeDirectionV1::Long => &self.long,
                TradeDirectionV1::Short => &self.short,
            }
        }
    }

    fn execution_series<'a>(
        instrument: &'a InstrumentKey,
        bars: &'a [Candle],
        calendar_digest: [u8; 32],
    ) -> ExecutionSeriesV1<'a> {
        ExecutionSeriesV1::new(
            instrument,
            TEST_VENDOR.as_str(),
            "canonical-test-commit",
            calendar_digest,
            bars,
        )
        .expect("canonical fixture execution series")
    }

    fn family_replay_fixture(
        family: InstrumentFamilyV1,
        instrument: &InstrumentKey,
        training: ExecutionSeriesV1<'_>,
        training_column: &Column,
        oos_bars: &[Candle],
        mask_words: [u64; 6],
        horizon: Horizon,
    ) -> FamilyReplayFixture {
        let (long_resolved, long_selected, long_training_run) = resolved_and_selected(
            instrument,
            &policy(Side::Long),
            training,
            training_column,
            mask_words,
            horizon,
        );
        let (short_resolved, short_selected, short_training_run) = resolved_and_selected(
            instrument,
            &policy(Side::Short),
            training,
            training_column,
            mask_words,
            horizon,
        );
        FamilyReplayFixture {
            family,
            long: SideReplayFixture {
                resolved: long_resolved,
                selected: long_selected,
                training_run: long_training_run,
                oos_run: execution_run(instrument, mask_words, Side::Long, oos_bars),
            },
            short: SideReplayFixture {
                resolved: short_resolved,
                selected: short_selected,
                training_run: short_training_run,
                oos_run: execution_run(instrument, mask_words, Side::Short, oos_bars),
            },
        }
    }

    struct AuthorityFixture<'a> {
        mask_words: [u64; 6],
        horizon: Horizon,
        run_params: Params,
        fingerprint: indicators::column::EvaluationSpecFingerprintV1,
        nifty: &'a FamilyReplayFixture,
        bank_nifty: &'a FamilyReplayFixture,
    }

    fn population_rows_fixture(
        source: &FamilyReplayFixture,
        population_id: [u8; 32],
        rung_index: usize,
        rung_seconds: u32,
        row_count: usize,
        mask_words: [u64; 6],
    ) -> Vec<PopulationRowV1> {
        let family_offset = match source.family {
            InstrumentFamilyV1::Nifty => 0,
            InstrumentFamilyV1::BankNifty => 1,
        };
        (0..row_count)
            .map(|sequence| {
                population_row(
                    population_id,
                    sequence,
                    source.family,
                    rung_seconds,
                    mask_words,
                    &source
                        .side(if sequence.is_multiple_of(2) {
                            TradeDirectionV1::Long
                        } else {
                            TradeDirectionV1::Short
                        })
                        .selected,
                    rung_index * 25 + sequence * 2 + family_offset,
                )
            })
            .collect()
    }

    fn append_population_fixture(
        execution: &mut ExecutionCapabilityLedger,
        inputs: &AuthorityFixture<'_>,
        source: &FamilyReplayFixture,
        population_v4_digest: [u8; 32],
        rung_seconds: u32,
        rows: &[PopulationRowV1],
    ) {
        let population_id = rows[0].population_id;
        let parameters = |direction, resolved: &ResolvedExitGridV1| {
            canonical_execution_parameters_fixture_v1(
                population_id,
                population_v4_digest,
                direction,
                source.family,
                rung_seconds,
                inputs.horizon,
                inputs.run_params,
                inputs.fingerprint,
                resolved,
            )
            .expect("canonical execution fixture parameters")
        };
        let selected: Vec<_> = rows
            .iter()
            .map(|row| source.side(row.direction).selected.clone())
            .collect();
        let prepared = canonical_execution_capabilities_fixture_v1(
            parameters(TradeDirectionV1::Long, &source.long.resolved),
            parameters(TradeDirectionV1::Short, &source.short.resolved),
            rows,
            &selected,
        )
        .expect("complete execution fixture authority");
        execution
            .append_complete(&prepared)
            .expect("execution fixture authority commit");
    }

    struct RungReplayFixture {
        selection: SelectionReceiptV3,
        ranked_rows: Vec<PopulationRowV1>,
    }

    fn append_rung_fixture(
        execution: &mut ExecutionCapabilityLedger,
        inputs: &AuthorityFixture<'_>,
        rung_index: usize,
        rung_seconds: u32,
    ) -> RungReplayFixture {
        let population_ids = [
            indexed_digest(214, rung_index * 2),
            indexed_digest(214, rung_index * 2 + 1),
        ];
        let v4_digests = [
            indexed_digest(215, rung_index * 2),
            indexed_digest(215, rung_index * 2 + 1),
        ];
        let nifty_rows = population_rows_fixture(
            inputs.nifty,
            population_ids[0],
            rung_index,
            rung_seconds,
            13,
            inputs.mask_words,
        );
        let bank_rows = population_rows_fixture(
            inputs.bank_nifty,
            population_ids[1],
            rung_index,
            rung_seconds,
            12,
            inputs.mask_words,
        );
        append_population_fixture(
            execution,
            inputs,
            inputs.nifty,
            v4_digests[0],
            rung_seconds,
            &nifty_rows,
        );
        append_population_fixture(
            execution,
            inputs,
            inputs.bank_nifty,
            v4_digests[1],
            rung_seconds,
            &bank_rows,
        );
        finish_rung_fixture(
            inputs,
            rung_index,
            rung_seconds,
            population_ids,
            v4_digests,
            &nifty_rows,
            &bank_rows,
        )
    }

    fn finish_rung_fixture(
        inputs: &AuthorityFixture<'_>,
        rung_index: usize,
        rung_seconds: u32,
        population_ids: [[u8; 32]; 2],
        v4_digests: [[u8; 32]; 2],
        nifty_rows: &[PopulationRowV1],
        bank_rows: &[PopulationRowV1],
    ) -> RungReplayFixture {
        let references = [
            CanonicalSelectionPopulationFixtureV3 {
                family: InstrumentFamilyV1::Nifty,
                population_id: population_ids[0],
                row_count: 13,
                ordered_row_digest: ordered_population_digest(nifty_rows),
                population_v4_completion_digest: v4_digests[0],
                admission_completion_digest: indexed_digest(216, rung_index * 2),
            },
            CanonicalSelectionPopulationFixtureV3 {
                family: InstrumentFamilyV1::BankNifty,
                population_id: population_ids[1],
                row_count: 12,
                ordered_row_digest: ordered_population_digest(bank_rows),
                population_v4_completion_digest: v4_digests[1],
                admission_completion_digest: indexed_digest(216, rung_index * 2 + 1),
            },
        ];
        let ranked_rows: Vec<_> = (0_usize..25_usize)
            .map(|rank| {
                if rank.is_multiple_of(2) {
                    nifty_rows[rank / 2]
                } else {
                    bank_rows[rank / 2]
                }
            })
            .collect();
        let selected = ranked_rows
            .iter()
            .enumerate()
            .map(|(rank, row)| {
                SelectedEntryV1::new(
                    row.instrument_family,
                    row.population_id,
                    row.sequence,
                    row.strategy_digest,
                    SCORE_SCALE - u64::try_from(rank).expect("fixture rank"),
                )
            })
            .collect();
        let rung_cohort = cohort(
            rung_index,
            inputs.nifty.long.resolved.policy_digest(),
            inputs.nifty.short.resolved.policy_digest(),
        );
        let selection = canonical_selection_receipt_fixture_v3(
            rung_seconds,
            &rung_cohort,
            &references,
            selected,
        )
        .expect("canonical Selection V3 fixture");
        RungReplayFixture {
            selection,
            ranked_rows,
        }
    }

    struct WitnessFixture<'a> {
        training_column: &'a Column,
        oos_column: &'a Column,
        nifty_training: ExecutionSeriesV1<'a>,
        bank_training: ExecutionSeriesV1<'a>,
        nifty_oos: OosExecutionSeriesV1<'a>,
        bank_oos: OosExecutionSeriesV1<'a>,
        nifty: &'a FamilyReplayFixture,
        bank_nifty: &'a FamilyReplayFixture,
    }

    fn replay_witnesses<'a>(
        rows: &'a [([u8; 32], u16, PopulationRowV1)],
        fixture: &WitnessFixture<'a>,
    ) -> Vec<SelectedReplayWitnessV1<'a>> {
        rows.iter()
            .map(|(selection_id, rank, row)| {
                let (training_series, oos_series, family) = match row.instrument_family {
                    InstrumentFamilyV1::Nifty => {
                        (fixture.nifty_training, fixture.nifty_oos, fixture.nifty)
                    }
                    InstrumentFamilyV1::BankNifty => {
                        (fixture.bank_training, fixture.bank_oos, fixture.bank_nifty)
                    }
                };
                let side = family.side(row.direction);
                SelectedReplayWitnessV1 {
                    selection_id: *selection_id,
                    rank: *rank,
                    population_row: *row,
                    training_series,
                    training_column: fixture.training_column,
                    training_run: side.training_run,
                    oos_series,
                    oos_column: fixture.oos_column,
                    oos_run: side.oos_run,
                }
            })
            .collect()
    }

    fn empty_vix_authority(root: &Path) -> VixReferenceMonth {
        let key = InstrumentKey::index(Exchange::Nse, "INDIAVIX").expect("India VIX reference key");
        let month = YearMonth::new(1970, 1).expect("synthetic OOS month");
        let path = StorePath::for_key(
            TEST_VENDOR,
            &key,
            Timeframe::MINUTE_1,
            month,
            FileKind::Bars,
        )
        .expect("India VIX fixture path");
        let symbol_hash = brutex_core::universe::fnv1a("INDIAVIX");
        let symbol_id =
            u32::try_from(symbol_hash & u64::from(u32::MAX)).expect("low 32-bit VIX symbol id");
        drop(
            BarFile::open_or_create(root, path, symbol_id).expect("empty authoritative VIX month"),
        );
        VixReferenceMonth::open(root, TEST_VENDOR, month).expect("indexed empty VIX authority")
    }

    fn shifted_oos_bars() -> Vec<Candle> {
        let mut bars = sessions(6);
        for bar in &mut bars {
            bar.ts_micros = bar
                .ts_micros
                .checked_add(14_i64 * DAY_MICROS)
                .expect("two-week OOS shift");
        }
        bars
    }

    fn assert_scheduler_coverage(prepared: &PreparedGlobalReplayV1) {
        assert_eq!(prepared.stream_count(), 200);
        assert_eq!(prepared.selection_ids().len(), 8);
        assert_eq!(prepared.execution_authority_ids().len(), 16);
        let scheduled_streams: HashSet<_> = prepared
            .decisions
            .iter()
            .map(|decision| decision.stream_ordinal)
            .collect();
        assert_eq!(scheduled_streams.len(), 200);
        assert!(
            prepared
                .streams
                .iter()
                .all(|stream| stream.candidate_count > 0)
        );
    }

    fn assert_authority_refusals(
        selections: &[SelectionReceiptV3],
        execution: &mut ExecutionCapabilityLedger,
        witnesses: &[SelectedReplayWitnessV1<'_>],
        missing_root: &Path,
    ) {
        let mut missing_execution =
            ExecutionCapabilityLedger::open(missing_root).expect("empty execution ledger");
        let missing = prepare_global_replay_v1(selections, &mut missing_execution, witnesses, &[])
            .expect_err("missing execution authorities must refuse");
        assert!(missing.contains("has no execution completion authority"));

        let mut foreign_selections = selections.to_vec();
        let original = &selections[0];
        let [nifty_reference, bank_reference] = *original.populations();
        let foreign_references = [
            CanonicalSelectionPopulationFixtureV3 {
                family: nifty_reference.family(),
                population_id: nifty_reference.population_id(),
                row_count: nifty_reference.row_count(),
                ordered_row_digest: nifty_reference.ordered_row_digest(),
                population_v4_completion_digest: indexed_digest(217, 0),
                admission_completion_digest: nifty_reference.admission_completion_digest(),
            },
            CanonicalSelectionPopulationFixtureV3 {
                family: bank_reference.family(),
                population_id: bank_reference.population_id(),
                row_count: bank_reference.row_count(),
                ordered_row_digest: bank_reference.ordered_row_digest(),
                population_v4_completion_digest: bank_reference.population_v4_completion_digest(),
                admission_completion_digest: bank_reference.admission_completion_digest(),
            },
        ];
        foreign_selections[0] = canonical_selection_receipt_fixture_v3(
            original.rung_seconds(),
            &original.cohort_identity(),
            &foreign_references,
            original.top_twenty_five().to_vec(),
        )
        .expect("semantically valid foreign Selection V3 fixture");
        let foreign = prepare_global_replay_v1(&foreign_selections, execution, witnesses, &[])
            .expect_err("foreign execution authority must refuse");
        assert!(foreign.contains("does not cover exact Selection V3 population"));
        drop(missing_execution);
        cleanup(missing_root);
    }

    #[test]
    fn public_prepare_reaches_scheduler_with_all_authorities_and_refuses_missing_or_foreign_ones() {
        let fixture_root = root("public-prepare");
        let missing_root = root("public-prepare-missing");

        let nifty_key = InstrumentKey::index(Exchange::Nse, "NIFTY").expect("NIFTY index key");
        let bank_key =
            InstrumentKey::index(Exchange::Nse, "BANKNIFTY").expect("BANKNIFTY index key");
        let training = sessions(6);
        let oos_bars = shifted_oos_bars();
        let training_column = test_column(&training);
        let oos_column = test_column(&oos_bars);
        let mask_words = busiest_shared_bit(&training_column, &oos_column);
        let horizon = Horizon::bars(5).expect("five-bar fixture horizon");
        let run_params = Params {
            min_hits: 1,
            ceiling: 1,
            pair_budget: 1,
            policy: 1,
        };
        let fingerprint = training_column
            .evaluation_spec_token()
            .expect("evaluator-built training column")
            .fingerprint_v1();
        let calendar_digest = indexed_digest(213, 0);
        let nifty_training = execution_series(&nifty_key, &training, calendar_digest);
        let bank_training = execution_series(&bank_key, &training, calendar_digest);
        let nifty_oos =
            OosExecutionSeriesV1::new(execution_series(&nifty_key, &oos_bars, calendar_digest), 0)
                .expect("NIFTY OOS boundary");
        let bank_oos =
            OosExecutionSeriesV1::new(execution_series(&bank_key, &oos_bars, calendar_digest), 0)
                .expect("BANKNIFTY OOS boundary");
        let nifty = family_replay_fixture(
            InstrumentFamilyV1::Nifty,
            &nifty_key,
            nifty_training,
            &training_column,
            &oos_bars,
            mask_words,
            horizon,
        );
        let bank_nifty = family_replay_fixture(
            InstrumentFamilyV1::BankNifty,
            &bank_key,
            bank_training,
            &training_column,
            &oos_bars,
            mask_words,
            horizon,
        );
        let mut execution =
            ExecutionCapabilityLedger::open(&fixture_root).expect("execution ledger");
        let authority = AuthorityFixture {
            mask_words,
            horizon,
            run_params,
            fingerprint,
            nifty: &nifty,
            bank_nifty: &bank_nifty,
        };
        let mut selections = Vec::new();
        let mut witnessed_rows = Vec::new();
        for (rung_index, rung_seconds) in CANONICAL_RUNGS_SECONDS.into_iter().enumerate() {
            let rung = append_rung_fixture(&mut execution, &authority, rung_index, rung_seconds);
            for (rank_zero, row) in rung.ranked_rows.into_iter().enumerate() {
                witnessed_rows.push((
                    rung.selection.selection_id(),
                    u16::try_from(rank_zero + 1).expect("one-based fixture rank"),
                    row,
                ));
            }
            selections.push(rung.selection);
        }
        let witnesses = replay_witnesses(
            &witnessed_rows,
            &WitnessFixture {
                training_column: &training_column,
                oos_column: &oos_column,
                nifty_training,
                bank_training,
                nifty_oos,
                bank_oos,
                nifty: &nifty,
                bank_nifty: &bank_nifty,
            },
        );

        let vix = empty_vix_authority(&fixture_root);
        let prepared = prepare_global_replay_v1(&selections, &mut execution, &witnesses, &[vix])
            .expect("public global replay preparation");
        assert_scheduler_coverage(&prepared);

        assert_authority_refusals(&selections, &mut execution, &witnesses, &missing_root);
        drop(execution);
        cleanup(&fixture_root);
    }

    #[test]
    fn fixed_record_codecs_roundtrip_and_seal_or_reserve_mutations_refuse() {
        let value = prepared();
        let stream = value.streams[0];
        let stream_raw = stream.to_bytes().expect("stream bytes");
        assert_eq!(StreamRecordV1::from_bytes(&stream_raw).unwrap(), stream);
        let decision = value.decisions[0];
        let decision_raw = decision.to_bytes().expect("decision bytes");
        assert_eq!(
            DecisionRecordV1::from_bytes(&decision_raw).unwrap(),
            decision
        );
        let trade = value.trades[0];
        let trade_raw = trade.to_bytes().expect("trade bytes");
        assert_eq!(TradeRecordV1::from_bytes(&trade_raw).unwrap(), trade);
        let completion =
            GlobalReplayCompletionV1::for_prepared(&value, 3, 8, 13).expect("completion fixture");
        let completion_raw = completion.to_bytes().expect("completion bytes");
        assert_eq!(
            GlobalReplayCompletionV1::from_bytes(&completion_raw).unwrap(),
            completion
        );

        let mut partial = value.streams.clone();
        partial.pop();
        assert!(
            validate_manifest_streams(&value.manifest, &partial)
                .unwrap_err()
                .contains("exactly 200")
        );
        let mut wrong_slot = value.streams.clone();
        wrong_slot[25].selection_id = value.manifest.selection_ids[0];
        assert!(
            validate_manifest_streams(&value.manifest, &wrong_slot)
                .unwrap_err()
                .contains("manifest slot")
        );

        let mut torn = decision_raw;
        torn[120] ^= 1;
        assert!(
            DecisionRecordV1::from_bytes(&torn)
                .unwrap_err()
                .contains("BLAKE3 seal")
        );

        let mut reserved = trade_raw;
        reserved[50] = 1;
        reseal::<TRADE_PAYLOAD_BYTES, GLOBAL_REPLAY_TRADE_STRIDE>(&mut reserved);
        assert!(
            TradeRecordV1::from_bytes(&reserved)
                .unwrap_err()
                .contains("trade tag reserve")
        );
    }

    #[test]
    fn inclusive_global_occupancy_replays_and_pricing_refusal_never_gets_money() {
        let value = prepared();
        assert_eq!(value.counters.offered, 3);
        assert_eq!(value.counters.admitted, 2);
        assert_eq!(value.counters.blocked_occupied, 1);
        assert_eq!(value.admitted_pricing_refused, 1);
        assert_eq!(value.trades.len(), 1);

        let mut fabricated = value.clone();
        let block_only = fabricated.decisions[2];
        let mut extra = fabricated.trades[0];
        extra.decision_sequence = block_only.sequence;
        extra.row.entry_micros = block_only.entry_micros;
        extra.row.exit_micros = block_only.occupied_through_micros;
        fabricated.trades.push(extra);
        assert!(
            validate_trade_block(
                &fabricated.streams,
                &fabricated.decisions,
                &fabricated.trades
            )
            .unwrap_err()
            .contains("pricing-refused or blocked")
        );
    }

    #[test]
    fn vix_changes_publication_but_not_selection_pnl_or_replay_identity() {
        let base = prepared();
        let mut changed = base.clone();
        let row = changed.trades[0].row;
        let exact = |ts_micros| {
            VixStamp::Exact(Candle {
                ts_micros,
                open: 1_500,
                high: 1_600,
                low: 1_400,
                close: 1_550,
                volume: 10,
                open_interest: i64::MIN,
            })
        };
        changed.trades[0].vix = VixPairV1 {
            entry: exact(row.entry_micros),
            exit: exact(row.exit_micros),
        };
        changed.ordered_trade_digest = trade_order_digest(&changed.trades).unwrap();
        changed.publication_id =
            derive_publication_id(changed.replay_id, changed.ordered_trade_digest);
        changed
            .validate()
            .expect("VIX-only publication remains valid");
        assert_eq!(changed.replay_id, base.replay_id);
        assert_eq!(changed.trades[0].row, base.trades[0].row);
        assert_ne!(changed.publication_id, base.publication_id);
    }

    #[test]
    fn receipt_last_ledger_reopens_replays_and_reuses_exact_publication() {
        let root = root("reopen");
        let value = prepared();
        {
            let mut ledger = GlobalReplayLedger::open(&root, 2).expect("create ledger");
            assert_eq!(
                ledger.append_complete(&value).unwrap(),
                GlobalReplayCommitV1::Written
            );
            assert_eq!(
                ledger.append_complete(&value).unwrap(),
                GlobalReplayCommitV1::Reused
            );
            assert_eq!(ledger.completions(), 1);
        }
        let ledger = GlobalReplayLedger::open_read(&root, 1).expect("reopen replay ledger");
        let receipt = ledger
            .completion(&value.publication_id)
            .expect("exact publication indexed");
        assert_eq!(receipt.replay_id(), value.replay_id);
        assert_eq!(receipt.decision_count(), 3);
        assert_eq!(receipt.trade_count(), 1);
        assert_eq!(ledger.latest_for_replay(&value.replay_id), Some(receipt));
        cleanup(&root);
    }

    #[test]
    fn valid_orphan_is_not_committed_and_reordered_decision_refuses_reopen() {
        let root = root("orphan-corruption");
        let value = prepared();
        {
            let mut ledger = GlobalReplayLedger::open(&root, 2).expect("create ledger");
            ledger
                .append_complete(&value)
                .expect("write receipt-last block");
        }
        let mut orphan = OpenOptions::new()
            .append(true)
            .open(GlobalReplayLedger::stream_path(&root))
            .expect("open orphan append");
        orphan
            .write_all(&value.streams[0].to_bytes().unwrap())
            .expect("append valid orphan");
        orphan.sync_all().expect("sync orphan");
        let reopened = GlobalReplayLedger::open_read(&root, 1).expect("orphan remains uncommitted");
        assert_eq!(reopened.completions(), 1);
        drop(reopened);

        let path = GlobalReplayLedger::decision_path(&root);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("open decision corruption");
        let second = HEADER_BYTES + u64::try_from(GLOBAL_REPLAY_DECISION_STRIDE).unwrap();
        file.seek(SeekFrom::Start(second))
            .expect("seek second decision");
        let mut raw = value.decisions[1].to_bytes().unwrap();
        raw[32..40].copy_from_slice(&0_u64.to_le_bytes());
        reseal::<DECISION_PAYLOAD_BYTES, GLOBAL_REPLAY_DECISION_STRIDE>(&mut raw);
        file.write_all(&raw)
            .expect("write reordered sealed decision");
        file.sync_all().expect("sync decision corruption");
        assert!(
            GlobalReplayLedger::open_read(&root, 1)
                .unwrap_err()
                .contains("reordered")
        );
        cleanup(&root);
    }

    #[test]
    fn same_length_external_mutation_makes_open_writer_stale() {
        let root = root("stale");
        let value = prepared();
        let mut ledger = GlobalReplayLedger::open(&root, 2).expect("create writer");
        ledger
            .append_complete(&value)
            .expect("write before same-length mutation");
        let path = GlobalReplayLedger::stream_path(&root);
        let mut external = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("open external mutation");
        external
            .seek(SeekFrom::Start(HEADER_BYTES + 10))
            .expect("seek header mutation target");
        external.write_all(&[1]).expect("same-length mutation");
        external.sync_all().expect("sync mutation");
        assert!(
            ledger
                .append_complete(&value)
                .unwrap_err()
                .contains("changed after open")
        );
        drop(ledger);
        cleanup(&root);
    }
}
