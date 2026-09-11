//! Selection-V4/Execution-V2-authoritative global replay preparation.
//!
//! This is an append-only successor boundary.  It does not open, decode or
//! reinterpret the V1 global-replay files.  Exactly eight canonical Selection
//! V4 receipts name sixteen Population-V4/admission/Execution-V2 authorities.
//! Every selected row is reclassified from its exact training bytes through
//! the matching durable Execution V2 disposition before its frozen coordinate
//! is replayed on out-of-sample one-minute bars.
//!
//! The resulting candidate universes are offered to
//! [`GlobalSinglePositionV1`] in timestamp order.  Consequently a long blocks
//! a short, NIFTY blocks BANKNIFTY and one signal timeframe blocks every other
//! timeframe until the inclusive occupied-through minute has passed.
//!
//! # Exact persistence boundary
//!
//! [`GlobalReplayLedgerV2`] appends separately versioned and sealed manifest,
//! candidate, stream-header and decision records, synchronizes each authority,
//! then synchronizes [`GlobalReplayCompletionV2`] last. Reopen reconstructs all
//! 200 streams and re-runs the same scheduler before indexing a completion.
//! Valid unreferenced records remain append-only orphans; malformed or reordered
//! records, overlapping ranges and same-length stale-writer changes refuse.
//! This is intentionally an **execution-only** completion: current V2 prepared
//! types contain neither admitted-money rows nor VIX stamps, so the receipt
//! binds an explicit no-money/no-VIX scope and makes no publication claim for
//! either missing authority.
//!
//! # Cost
//!
//! Manifest construction is O(16).  Selected-row reconstruction is linear in
//! the exact training/OOS bars and resolved grid for each of 200 streams.  The
//! chronological merge examines at most 200 stream heads per emitted minute;
//! the delegated scheduler uses its documented fixed 200-intent bound.  Hashing
//! and canonical encoding are linear in their fixed records.  This module
//! makes no end-to-end O(1) claim.
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use brutex_core::blake3::{self, Hasher};
use brutex_core::instrument::{Exchange, InstrumentKey};
use costs::fill::Direction;
use indicators::column::Column;
use runner::admission::AdmissionStatusV1;
use runner::exit_grid_policy::{
    ExecutionRunV1, ExecutionSeriesV1, OosExecutionSeriesV1, ReplayedCandidateUniverseV1,
};
use runner::grid::{ReplayCandidateV1, ReplayPathV1};
use runner::portfolio::{
    Constituent, Counters, Disposition, Evidence, GlobalSinglePositionV1, Intent,
    MAX_INTENTS_PER_MINUTE, StrategyDigest,
};
use runner::topn::RankingPolicyV1;

use crate::admission_store::{AdmissionCompletionReceiptV1, AdmissionDecisionRecordV1};
use crate::execution_capability::exact_execution_law_digest_v1;
use crate::execution_disposition_v2::{
    ExecutionDispositionLedgerV2, ExecutionDispositionTagV2, ReclassifiedExecutionDispositionV2,
};
use crate::population::{
    CompletionReceiptV4, InstrumentFamilyV1, PopulationRowV1, TradeDirectionV1,
};
use crate::selection::SelectedEntryV1;
use crate::selection_v4::{
    PopulationReferenceV4, SelectionAuthorityViewV4, SelectionJoinedRowV4, SelectionReceiptV4,
};
use crate::selection_v4_authority::SelectionAuthorityLedgerV4;

/// Operator-facing refusal from V2 manifest construction, reconstruction or scheduling.
pub type GlobalReplayRefusalV2 = String;

/// Exact canonical signal rungs, in durable manifest order.
pub const GLOBAL_REPLAY_V2_RUNGS_SECONDS: [u32; 8] = [60, 120, 180, 300, 600, 900, 1_800, 3_600];

const MAX_STREAMS: usize = 8 * 25;
const MANIFEST_MAGIC_V2: [u8; 8] = *b"BRUTXGM2";
const MANIFEST_VERSION_V2: u32 = 2;
const MANIFEST_PAYLOAD_BYTES_V2: usize = 3_536;
const SEAL_BYTES: usize = 32;
const LEDGER_HEADER_BYTES_V2: u64 = 24;
const LEDGER_HEADER_BYTES_USIZE_V2: usize = 24;
const LEDGER_FORMAT_VERSION_V2: u32 = 2;
const SIGNAL_COVERAGE_START: usize = 376;
const SIGNAL_COVERAGE_END: usize = 408;
const COMMON_COHORT_BYTES: usize = 440;

/// Canonical bytes in one sealed V2 replay-authority manifest.
pub const GLOBAL_REPLAY_MANIFEST_STRIDE_V2: usize = MANIFEST_PAYLOAD_BYTES_V2 + SEAL_BYTES;

const MANIFEST_ID_DOMAIN_V2: &[u8] = b"brutex.cli.global-replay-manifest.v2\0";
const COMMON_COHORT_DOMAIN_V2: &[u8] = b"brutex.cli.global-replay-common-cohort.v2\0";
const STREAM_DOMAIN_V2: &[u8] = b"brutex.cli.global-replay-stream.v2\0";
const STREAM_ORDER_DOMAIN_V2: &[u8] = b"brutex.cli.global-replay-stream-order.v2\0";
const CANDIDATE_DOMAIN_V2: &[u8] = b"brutex.cli.global-replay-candidate.v2\0";
const DECISION_DOMAIN_V2: &[u8] = b"brutex.cli.global-replay-decision.v2\0";
const DECISION_ORDER_DOMAIN_V2: &[u8] = b"brutex.cli.global-replay-decision-order.v2\0";
const REPLAY_ID_DOMAIN_V2: &[u8] = b"brutex.cli.global-replay-id.v2\0";
const CANDIDATE_ORDER_DOMAIN_V2: &[u8] = b"brutex.cli.global-replay-candidate-order.v2\0";
const COMPLETION_ID_DOMAIN_V2: &[u8] = b"brutex.cli.global-replay-completion.v2\0";
const EXECUTION_ONLY_SCOPE_DOMAIN_V2: &[u8] =
    b"brutex.cli.global-replay-execution-only-no-money-no-vix.v2\0";

const LEDGER_MANIFEST_MAGIC_V2: [u8; 8] = *b"BRUX2GMF";
const LEDGER_STREAM_MAGIC_V2: [u8; 8] = *b"BRUX2STR";
const LEDGER_CANDIDATE_MAGIC_V2: [u8; 8] = *b"BRUX2CAN";
const LEDGER_DECISION_MAGIC_V2: [u8; 8] = *b"BRUX2DEC";
const LEDGER_COMPLETION_MAGIC_V2: [u8; 8] = *b"BRUX2COM";

const STREAM_PAYLOAD_BYTES_V2: usize = 400;
const CANDIDATE_PAYLOAD_BYTES_V2: usize = 104;
const DECISION_PAYLOAD_BYTES_V2: usize = 208;
const COMPLETION_PAYLOAD_BYTES_V2: usize = 384;

/// Fixed width of one durable V2 reconstructed-stream header including seal.
pub const GLOBAL_REPLAY_STREAM_STRIDE_V2: usize = STREAM_PAYLOAD_BYTES_V2 + SEAL_BYTES;
/// Fixed width of one durable V2 pre-exclusivity candidate including seal.
pub const GLOBAL_REPLAY_CANDIDATE_STRIDE_V2: usize = CANDIDATE_PAYLOAD_BYTES_V2 + SEAL_BYTES;
/// Fixed width of one durable V2 scheduler decision including seal.
pub const GLOBAL_REPLAY_DECISION_STRIDE_V2: usize = DECISION_PAYLOAD_BYTES_V2 + SEAL_BYTES;
/// Fixed width of one receipt-last V2 execution-only completion including seal.
pub const GLOBAL_REPLAY_COMPLETION_STRIDE_V2: usize = COMPLETION_PAYLOAD_BYTES_V2 + SEAL_BYTES;

const _: () = assert!(MAX_STREAMS == MAX_INTENTS_PER_MINUTE);
const _: () = assert!(COMMON_COHORT_BYTES == 440);
const _: () = assert!(SIGNAL_COVERAGE_END <= COMMON_COHORT_BYTES);
const _: () = assert!(
    MANIFEST_PAYLOAD_BYTES_V2 == 16 + 32 + 32 + (8 * 32) + (16 * 32 * 4) + (32 * 32) + (16 * 8)
);
const _: () = assert!(GLOBAL_REPLAY_MANIFEST_STRIDE_V2 == 3_568);
const _: () = assert!(GLOBAL_REPLAY_STREAM_STRIDE_V2 == 432);
const _: () = assert!(GLOBAL_REPLAY_CANDIDATE_STRIDE_V2 == 136);
const _: () = assert!(GLOBAL_REPLAY_DECISION_STRIDE_V2 == 240);
const _: () = assert!(GLOBAL_REPLAY_COMPLETION_STRIDE_V2 == 416);

/// Exact caller-held evidence needed to reclassify and replay one selected row.
#[derive(Clone, Copy, Debug)]
pub struct SelectedReplayWitnessV2<'a> {
    /// V4 selection containing this selected row.
    pub selection_id: [u8; 32],
    /// One-based position within that selection's exact Top-25.
    pub rank: u16,
    /// Exact Population V4 receipt named by the selection's family reference.
    pub population_v4: CompletionReceiptV4,
    /// Exact row returned by the same triple-authority view that rebuilt V4.
    pub joined_row: SelectionJoinedRowV4,
    /// Exact receipt-last admission authority for this population.
    pub admission_completion: &'a AdmissionCompletionReceiptV1,
    /// Exact admission decision for this population row.
    pub admission_decision: &'a AdmissionDecisionRecordV1,
    /// Exact training one-minute OHLCV used to resolve and evaluate the grid.
    pub training_series: ExecutionSeriesV1<'a>,
    /// Exact training condition column and evaluator capability.
    pub training_column: &'a Column,
    /// Canonical training run capability.
    pub training_run: ExecutionRunV1,
    /// Explicit causally later OOS one-minute execution series.
    pub oos_series: OosExecutionSeriesV1<'a>,
    /// Complete OOS condition column.
    pub oos_column: &'a Column,
    /// Canonical OOS run capability.
    pub oos_run: ExecutionRunV1,
}

/// One V4 receipt held beside the two concrete authorities that can reproduce it.
#[derive(Debug)]
pub struct ReplaySelectionAuthorityV2 {
    receipt: SelectionReceiptV4,
    nifty: SelectionAuthorityLedgerV4,
    bank_nifty: SelectionAuthorityLedgerV4,
    policy: RankingPolicyV1,
}

impl ReplaySelectionAuthorityV2 {
    /// Seals one replay input only after a complete three-pass V4 reproduction.
    ///
    /// # Errors
    ///
    /// Propagates every Selection V4 authority, ranking or byte-comparison
    /// refusal. A detached receipt can never become a replay authority.
    pub fn new(
        receipt: SelectionReceiptV4,
        mut nifty: SelectionAuthorityLedgerV4,
        mut bank_nifty: SelectionAuthorityLedgerV4,
        policy: RankingPolicyV1,
    ) -> Result<Self, GlobalReplayRefusalV2> {
        receipt.verify_against_authorities(&mut nifty, &mut bank_nifty, policy)?;
        Ok(Self {
            receipt,
            nifty,
            bank_nifty,
            policy,
        })
    }

    /// Exact receipt whose ranking is reproducible from the owned authorities.
    #[must_use]
    pub const fn receipt(&self) -> &SelectionReceiptV4 {
        &self.receipt
    }

    fn reverify(&mut self) -> Result<(), GlobalReplayRefusalV2> {
        self.receipt
            .verify_against_authorities(&mut self.nifty, &mut self.bank_nifty, self.policy)
    }

    fn joined_row(
        &mut self,
        family: InstrumentFamilyV1,
        sequence: u64,
    ) -> Result<SelectionJoinedRowV4, GlobalReplayRefusalV2> {
        let view: &mut dyn SelectionAuthorityViewV4 = match family {
            InstrumentFamilyV1::Nifty => &mut self.nifty,
            InstrumentFamilyV1::BankNifty => &mut self.bank_nifty,
        };
        let page = view.page(sequence, 1)?;
        if page.offset != sequence || page.rows.len() != 1 {
            return Err("Selection V4 authority did not return the exact selected row".to_owned());
        }
        page.rows
            .first()
            .copied()
            .ok_or_else(|| "Selection V4 selected authority row is absent".to_owned())
    }
}

/// Exact authority tuple for one rung/family slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlobalReplayAuthoritySlotV2 {
    population_id: [u8; 32],
    population_v4_completion_digest: [u8; 32],
    admission_completion_digest: [u8; 32],
    execution_v2_completion_id: [u8; 32],
    execution_parameter_ids: [[u8; 32]; 2],
    row_count: u64,
}

impl GlobalReplayAuthoritySlotV2 {
    /// Population identity in this canonical rung/family slot.
    #[must_use]
    pub const fn population_id(self) -> [u8; 32] {
        self.population_id
    }

    /// Exact Population V4 completion digest.
    #[must_use]
    pub const fn population_v4_completion_digest(self) -> [u8; 32] {
        self.population_v4_completion_digest
    }

    /// Exact receipt-last admission completion digest.
    #[must_use]
    pub const fn admission_completion_digest(self) -> [u8; 32] {
        self.admission_completion_digest
    }

    /// Exact receipt-last Execution V2 completion identity.
    #[must_use]
    pub const fn execution_v2_completion_id(self) -> [u8; 32] {
        self.execution_v2_completion_id
    }

    /// Canonical `[long, short]` dynamic execution-parameter identities.
    #[must_use]
    pub const fn execution_parameter_ids(self) -> [[u8; 32]; 2] {
        self.execution_parameter_ids
    }

    /// Exact complete population row count.
    #[must_use]
    pub const fn row_count(self) -> u64 {
        self.row_count
    }

    fn validate(self) -> Result<(), GlobalReplayRefusalV2> {
        for (name, digest) in [
            ("manifest population", self.population_id),
            (
                "manifest Population V4 completion",
                self.population_v4_completion_digest,
            ),
            (
                "manifest admission completion",
                self.admission_completion_digest,
            ),
            (
                "manifest Execution V2 completion",
                self.execution_v2_completion_id,
            ),
            (
                "manifest long execution parameter",
                self.execution_parameter_ids[0],
            ),
            (
                "manifest short execution parameter",
                self.execution_parameter_ids[1],
            ),
        ] {
            require_digest(name, &digest)?;
        }
        if self.execution_parameter_ids[0] == self.execution_parameter_ids[1] {
            return Err(
                "global replay V2 authority has identical long/short parameters".to_owned(),
            );
        }
        Ok(())
    }
}

/// New Selection-V4/Execution-V2 replay manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobalReplayManifestV2 {
    manifest_id: [u8; 32],
    common_cohort_digest: [u8; 32],
    selection_ids: [[u8; 32]; 8],
    authorities: [GlobalReplayAuthoritySlotV2; 16],
}

impl GlobalReplayManifestV2 {
    /// New append-only manifest path.  It never aliases a V1 replay file.
    #[must_use]
    pub fn path(root: &Path) -> PathBuf {
        root.join("results")
            .join("global-replay-authority-manifests-v2.bin")
    }

    /// Constructs the exact eight-rung/16-authority manifest from live V2 authorities.
    ///
    /// # Errors
    ///
    /// Refuses any missing, duplicated, reordered or cross-cohort selection;
    /// any missing/stale/mismatched Execution V2 completion; a matrix/count/law
    /// disagreement; or a reused population, completion or signal-coverage proof.
    #[allow(
        clippy::too_many_lines,
        reason = "the fixed 8-rung authority matrix is validated as one indivisible transaction"
    )]
    pub fn from_selections(
        selection_authorities: &mut [ReplaySelectionAuthorityV2],
        execution: &mut ExecutionDispositionLedgerV2,
    ) -> Result<Self, GlobalReplayRefusalV2> {
        let authority_count = selection_authorities.len();
        let selections: &mut [ReplaySelectionAuthorityV2; 8] =
            selection_authorities.try_into().map_err(|_| {
            format!(
                "global replay V2 requires exactly eight canonical Selection V4 receipts, found {authority_count}"
            )
        })?;
        for authority in selections.iter_mut() {
            authority.reverify()?;
        }
        let first = selections
            .first()
            .map(Self::selection_of)
            .ok_or_else(|| "global replay V2 canonical selection array is empty".to_owned())?;
        let first_common = common_cohort_bytes(first);
        let common_cohort_digest = digest_common_cohort(&first_common);
        let mut selection_ids = [[0_u8; 32]; 8];
        let mut authorities = [empty_authority_slot(); 16];
        let mut seen_selections = HashSet::new();
        let mut seen_populations = HashSet::new();
        let mut seen_execution = HashSet::new();
        let mut seen_signal_coverages = HashSet::new();

        for (rung_index, (selection_authority, expected_rung)) in selections
            .iter()
            .zip(GLOBAL_REPLAY_V2_RUNGS_SECONDS)
            .enumerate()
        {
            let selection = selection_authority.receipt();
            if selection.rung_seconds() != expected_rung {
                return Err(format!(
                    "global replay V2 rung slot {rung_index} requires {expected_rung}s, found {}s",
                    selection.rung_seconds()
                ));
            }
            if selection.top_twenty_five().len() != 25 {
                return Err(format!(
                    "global replay V2 requires the complete Top-25 at {expected_rung}s, found {}",
                    selection.top_twenty_five().len()
                ));
            }
            require_digest("Selection V4 identity", &selection.selection_id())?;
            if !seen_selections.insert(selection.selection_id()) {
                return Err("one Selection V4 receipt was copied across canonical rungs".to_owned());
            }
            if common_cohort_bytes(selection) != first_common {
                return Err(format!(
                    "Selection V4 {} at {expected_rung}s is outside the common cross-rung cohort",
                    hex(&selection.selection_id())
                ));
            }
            let signal_coverage = selection.cohort_identity().signal_coverage_digest();
            require_digest("selection signal-calendar coverage", &signal_coverage)?;
            if !seen_signal_coverages.insert(signal_coverage) {
                return Err(
                    "two canonical signal rungs copied one signal-calendar coverage authority"
                        .to_owned(),
                );
            }
            *selection_ids
                .get_mut(rung_index)
                .ok_or_else(|| "global replay V2 selection slot is absent".to_owned())? =
                selection.selection_id();
            let references = selection.populations();
            if references[0].family() != InstrumentFamilyV1::Nifty
                || references[1].family() != InstrumentFamilyV1::BankNifty
            {
                return Err("Selection V4 populations are not NIFTY then BANKNIFTY".to_owned());
            }
            for (family_index, reference) in references.iter().enumerate() {
                let slot_index = rung_index
                    .checked_mul(2)
                    .and_then(|offset| offset.checked_add(family_index))
                    .ok_or_else(|| "global replay V2 authority slot overflowed".to_owned())?;
                if !seen_populations.insert(reference.population_id()) {
                    return Err(
                        "one Population V4 identity was reused across authority slots".to_owned(),
                    );
                }
                let completion = execution
                    .completion(reference.population_id())?
                    .ok_or_else(|| {
                        format!(
                            "Selection V4 {} {:?} population {} has no Execution V2 completion",
                            hex(&selection.selection_id()),
                            reference.family(),
                            hex(&reference.population_id())
                        )
                    })?;
                require_execution_completion(reference, &completion)?;
                if !seen_execution.insert(completion.completion_id()) {
                    return Err(
                        "one Execution V2 completion was reused across populations".to_owned()
                    );
                }
                *authorities
                    .get_mut(slot_index)
                    .ok_or_else(|| "global replay V2 authority slot is absent".to_owned())? =
                    GlobalReplayAuthoritySlotV2 {
                        population_id: reference.population_id(),
                        population_v4_completion_digest: reference
                            .population_v4_completion_digest(),
                        admission_completion_digest: reference.admission_completion_digest(),
                        execution_v2_completion_id: completion.completion_id(),
                        execution_parameter_ids: completion.parameter_ids(),
                        row_count: reference.row_count(),
                    };
            }
        }

        let mut manifest = Self {
            manifest_id: [0; 32],
            common_cohort_digest,
            selection_ids,
            authorities,
        };
        manifest.manifest_id = manifest.derived_id();
        manifest.validate()?;
        Ok(manifest)
    }

    fn selection_of(authority: &ReplaySelectionAuthorityV2) -> &SelectionReceiptV4 {
        authority.receipt()
    }

    /// Content-derived V2 authority-manifest identity.
    #[must_use]
    pub const fn manifest_id(&self) -> [u8; 32] {
        self.manifest_id
    }

    /// Cross-rung cohort identity with only rung-specific signal coverage erased.
    #[must_use]
    pub const fn common_cohort_digest(&self) -> [u8; 32] {
        self.common_cohort_digest
    }

    /// Exact Selection V4 identities in canonical rung order.
    #[must_use]
    pub const fn selection_ids(&self) -> &[[u8; 32]; 8] {
        &self.selection_ids
    }

    /// Sixteen exact authority triples in rung then `[NIFTY, BANKNIFTY]` order.
    #[must_use]
    pub const fn authorities(&self) -> &[GlobalReplayAuthoritySlotV2; 16] {
        &self.authorities
    }

    /// One exact authority slot.
    ///
    /// # Errors
    ///
    /// Refuses an overflowing or out-of-range rung/family slot.
    pub fn authority(
        &self,
        rung_index: usize,
        family: InstrumentFamilyV1,
    ) -> Result<GlobalReplayAuthoritySlotV2, GlobalReplayRefusalV2> {
        let family_index = match family {
            InstrumentFamilyV1::Nifty => 0,
            InstrumentFamilyV1::BankNifty => 1,
        };
        self.authorities
            .get(
                rung_index
                    .checked_mul(2)
                    .and_then(|value| value.checked_add(family_index))
                    .ok_or_else(|| "global replay V2 authority lookup overflowed".to_owned())?,
            )
            .copied()
            .ok_or_else(|| "global replay V2 authority slot is absent".to_owned())
    }

    /// Canonical sealed V2 manifest record, ready for a receipt-last ledger.
    ///
    /// # Errors
    ///
    /// Refuses an invalid manifest or an internal fixed-width encoding mismatch.
    pub fn canonical_bytes(
        &self,
    ) -> Result<[u8; GLOBAL_REPLAY_MANIFEST_STRIDE_V2], GlobalReplayRefusalV2> {
        self.validate()?;
        let mut payload = [0_u8; MANIFEST_PAYLOAD_BYTES_V2];
        let mut encoder = Encoder::new(&mut payload);
        encoder.bytes(&MANIFEST_MAGIC_V2)?;
        encoder.u32(MANIFEST_VERSION_V2)?;
        encoder.u32(
            u32::try_from(MANIFEST_PAYLOAD_BYTES_V2)
                .map_err(|_| "global replay V2 manifest payload does not fit u32".to_owned())?,
        )?;
        encoder.bytes(&self.manifest_id)?;
        encoder.bytes(&self.common_cohort_digest)?;
        for digest in self.selection_ids {
            encoder.bytes(&digest)?;
        }
        for slot in self.authorities {
            encoder.bytes(&slot.population_id)?;
        }
        for slot in self.authorities {
            encoder.bytes(&slot.population_v4_completion_digest)?;
        }
        for slot in self.authorities {
            encoder.bytes(&slot.admission_completion_digest)?;
        }
        for slot in self.authorities {
            encoder.bytes(&slot.execution_v2_completion_id)?;
        }
        for slot in self.authorities {
            for parameter in slot.execution_parameter_ids {
                encoder.bytes(&parameter)?;
            }
        }
        for slot in self.authorities {
            encoder.u64(slot.row_count)?;
        }
        encoder.finish()?;
        let seal = blake3::hash(&payload);
        let mut record = [0_u8; GLOBAL_REPLAY_MANIFEST_STRIDE_V2];
        record[..MANIFEST_PAYLOAD_BYTES_V2].copy_from_slice(&payload);
        record[MANIFEST_PAYLOAD_BYTES_V2..].copy_from_slice(&seal);
        Ok(record)
    }

    /// Decodes one exact sealed V2 manifest record without consulting V1 files.
    ///
    /// # Errors
    ///
    /// Refuses a torn, foreign-version, malformed, or semantically invalid record.
    pub fn from_canonical_bytes(
        record: &[u8; GLOBAL_REPLAY_MANIFEST_STRIDE_V2],
    ) -> Result<Self, GlobalReplayRefusalV2> {
        let payload: [u8; MANIFEST_PAYLOAD_BYTES_V2] = record[..MANIFEST_PAYLOAD_BYTES_V2]
            .try_into()
            .map_err(|_| "global replay V2 manifest payload width differs".to_owned())?;
        if record[MANIFEST_PAYLOAD_BYTES_V2..] != blake3::hash(&payload) {
            return Err("global replay V2 manifest failed its complete BLAKE3 seal".to_owned());
        }
        let mut decoder = Decoder::new(&payload);
        if decoder.array_8()? != MANIFEST_MAGIC_V2 {
            return Err("global replay V2 manifest magic differs".to_owned());
        }
        if decoder.u32()? != MANIFEST_VERSION_V2 {
            return Err("global replay V2 manifest version differs".to_owned());
        }
        if usize::try_from(decoder.u32()?).ok() != Some(MANIFEST_PAYLOAD_BYTES_V2) {
            return Err("global replay V2 manifest payload length differs".to_owned());
        }
        let manifest_id = decoder.array_32()?;
        let common_cohort_digest = decoder.array_32()?;
        let mut selection_ids = [[0_u8; 32]; 8];
        for selection in &mut selection_ids {
            *selection = decoder.array_32()?;
        }
        let mut authorities = [empty_authority_slot(); 16];
        for slot in &mut authorities {
            slot.population_id = decoder.array_32()?;
        }
        for slot in &mut authorities {
            slot.population_v4_completion_digest = decoder.array_32()?;
        }
        for slot in &mut authorities {
            slot.admission_completion_digest = decoder.array_32()?;
        }
        for slot in &mut authorities {
            slot.execution_v2_completion_id = decoder.array_32()?;
        }
        for slot in &mut authorities {
            slot.execution_parameter_ids = [decoder.array_32()?, decoder.array_32()?];
        }
        for slot in &mut authorities {
            slot.row_count = decoder.u64()?;
        }
        decoder.finish()?;
        let manifest = Self {
            manifest_id,
            common_cohort_digest,
            selection_ids,
            authorities,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    fn validate(&self) -> Result<(), GlobalReplayRefusalV2> {
        require_digest("global replay V2 manifest", &self.manifest_id)?;
        require_digest("global replay V2 common cohort", &self.common_cohort_digest)?;
        let mut selections = HashSet::new();
        for selection in self.selection_ids {
            require_digest("global replay V2 selection", &selection)?;
            if !selections.insert(selection) {
                return Err("global replay V2 manifest repeats a selection".to_owned());
            }
        }
        let mut populations = HashSet::new();
        let mut executions = HashSet::new();
        for slot in self.authorities {
            slot.validate()?;
            if !populations.insert(slot.population_id) {
                return Err("global replay V2 manifest repeats a population".to_owned());
            }
            if !executions.insert(slot.execution_v2_completion_id) {
                return Err("global replay V2 manifest repeats an execution completion".to_owned());
            }
        }
        if self.derived_id() != self.manifest_id {
            return Err("global replay V2 manifest identity differs from its fields".to_owned());
        }
        Ok(())
    }

    fn derived_id(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(MANIFEST_ID_DOMAIN_V2);
        hasher.update(&MANIFEST_VERSION_V2.to_le_bytes());
        hasher.update(&self.common_cohort_digest);
        for selection in self.selection_ids {
            hasher.update(&selection);
        }
        for slot in self.authorities {
            hasher.update(&slot.population_id);
            hasher.update(&slot.population_v4_completion_digest);
            hasher.update(&slot.admission_completion_digest);
            hasher.update(&slot.execution_v2_completion_id);
            for parameter in slot.execution_parameter_ids {
                hasher.update(&parameter);
            }
            hasher.update(&slot.row_count.to_le_bytes());
        }
        hasher.update(&exact_execution_law_digest_v1());
        hasher.finalize()
    }
}

/// One verified reconstructed stream before durable V2 publication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedReplayStreamV2 {
    stream_id: [u8; 32],
    stream_ordinal: u16,
    rank: u16,
    rung_seconds: u32,
    family: InstrumentFamilyV1,
    direction: TradeDirectionV1,
    selection_id: [u8; 32],
    population_id: [u8; 32],
    row_sequence: u64,
    strategy_digest: [u8; 32],
    execution_completion_id: [u8; 32],
    execution_disposition_id: [u8; 32],
    execution_capability_id: [u8; 32],
    training_run_id: [u8; 32],
    selected_exit_digest: [u8; 32],
    oos_run_id: [u8; 32],
    universe_digest: [u8; 32],
    pricing_refused_paths: u64,
    candidates: Vec<CandidateScheduleEvidenceV2>,
    cursor: usize,
}

impl VerifiedReplayStreamV2 {
    /// Content identity of the exact reconstructed stream authority.
    #[must_use]
    pub const fn stream_id(&self) -> [u8; 32] {
        self.stream_id
    }

    /// Exact V4 selection identity.
    #[must_use]
    pub const fn selection_id(&self) -> [u8; 32] {
        self.selection_id
    }

    /// One-based rank within the V4 Top-25.
    #[must_use]
    pub const fn rank(&self) -> u16 {
        self.rank
    }

    /// Number of pre-exclusivity candidate entries in this stream.
    #[must_use]
    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    /// Exact durable Execution V2 row disposition used for reclassification.
    #[must_use]
    pub const fn execution_disposition_id(&self) -> [u8; 32] {
        self.execution_disposition_id
    }

    fn validate(&self) -> Result<(), GlobalReplayRefusalV2> {
        if usize::from(self.stream_ordinal) >= MAX_STREAMS || self.rank == 0 || self.rank > 25 {
            return Err("global replay V2 stream has invalid ordinal/rank".to_owned());
        }
        validate_rung(self.rung_seconds)?;
        for (name, digest) in [
            ("V2 replay stream", self.stream_id),
            ("V2 stream selection", self.selection_id),
            ("V2 stream population", self.population_id),
            ("V2 stream strategy", self.strategy_digest),
            (
                "V2 stream execution completion",
                self.execution_completion_id,
            ),
            (
                "V2 stream execution disposition",
                self.execution_disposition_id,
            ),
            (
                "V2 stream execution capability",
                self.execution_capability_id,
            ),
            ("V2 stream training run", self.training_run_id),
            ("V2 stream selected exit", self.selected_exit_digest),
            ("V2 stream OOS run", self.oos_run_id),
            ("V2 stream universe", self.universe_digest),
        ] {
            require_digest(name, &digest)?;
        }
        let pricing_refused_paths = u64::try_from(
            self.candidates
                .iter()
                .filter(|candidate| candidate.path.pricing_refused())
                .count(),
        )
        .map_err(|_| "V2 pricing-refused count does not fit u64".to_owned())?;
        if self.pricing_refused_paths != pricing_refused_paths {
            return Err("V2 stream pricing-refused count differs from its candidates".to_owned());
        }
        for (ordinal, candidate) in self.candidates.iter().enumerate() {
            candidate.validate(self.universe_digest, ordinal)?;
        }
        if self.derived_id()? != self.stream_id {
            return Err("global replay V2 stream identity differs from its fields".to_owned());
        }
        Ok(())
    }

    fn derived_id(&self) -> Result<[u8; 32], GlobalReplayRefusalV2> {
        let mut hasher = Hasher::new();
        hasher.update(STREAM_DOMAIN_V2);
        hasher.update(&self.stream_ordinal.to_le_bytes());
        hasher.update(&self.rank.to_le_bytes());
        hasher.update(&self.rung_seconds.to_le_bytes());
        hasher.update(&[family_byte(self.family), direction_byte(self.direction)]);
        hasher.update(&self.selection_id);
        hasher.update(&self.population_id);
        hasher.update(&self.row_sequence.to_le_bytes());
        for digest in [
            self.strategy_digest,
            self.execution_completion_id,
            self.execution_disposition_id,
            self.execution_capability_id,
            self.training_run_id,
            self.selected_exit_digest,
            self.oos_run_id,
            self.universe_digest,
        ] {
            hasher.update(&digest);
        }
        hasher
            .update(&usize_u64(self.candidates.len(), "V2 stream candidate count")?.to_le_bytes());
        hasher.update(&self.pricing_refused_paths.to_le_bytes());
        Ok(hasher.finalize())
    }
}

/// Stable candidate path class retained by the scheduler boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayCandidatePathV2 {
    /// Complete money/quality evidence exists.
    Priceable,
    /// Entry is known but only conservative occupancy is priceable.
    BlockOnly,
    /// Entry is known and exit crossing evidence was refused.
    CrossingRefused,
    /// Both conservative block-only and crossing-refused evidence apply.
    BlockOnlyAndCrossingRefused,
}

impl ReplayCandidatePathV2 {
    const fn byte(self) -> u8 {
        match self {
            Self::Priceable => 1,
            Self::BlockOnly => 2,
            Self::CrossingRefused => 3,
            Self::BlockOnlyAndCrossingRefused => 4,
        }
    }

    const fn pricing_refused(self) -> bool {
        !matches!(self, Self::Priceable)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CandidateScheduleEvidenceV2 {
    candidate_digest: [u8; 32],
    signal_micros: i64,
    entry_micros: i64,
    occupied_through_micros: i64,
    path: ReplayCandidatePathV2,
}

impl CandidateScheduleEvidenceV2 {
    fn from_candidate(
        universe_digest: [u8; 32],
        ordinal: usize,
        candidate: ReplayCandidateV1,
    ) -> Result<Self, GlobalReplayRefusalV2> {
        let path = match candidate.path() {
            ReplayPathV1::Priceable(_) => ReplayCandidatePathV2::Priceable,
            ReplayPathV1::BlockOnly => ReplayCandidatePathV2::BlockOnly,
            ReplayPathV1::CrossingRefused => ReplayCandidatePathV2::CrossingRefused,
            ReplayPathV1::BlockOnlyAndCrossingRefused => {
                ReplayCandidatePathV2::BlockOnlyAndCrossingRefused
            }
        };
        let mut value = Self {
            candidate_digest: [0; 32],
            signal_micros: candidate.signal_micros(),
            entry_micros: candidate.entry_micros(),
            occupied_through_micros: candidate.occupied_through_micros(),
            path,
        };
        value.candidate_digest = value.derived_digest(universe_digest, ordinal)?;
        value.validate(universe_digest, ordinal)?;
        Ok(value)
    }

    fn validate(
        self,
        universe_digest: [u8; 32],
        ordinal: usize,
    ) -> Result<(), GlobalReplayRefusalV2> {
        require_digest("V2 replay candidate", &self.candidate_digest)?;
        if self.signal_micros >= self.entry_micros
            || self.occupied_through_micros < self.entry_micros
        {
            return Err(
                "V2 replay candidate has impossible signal/entry/occupancy order".to_owned(),
            );
        }
        if self.derived_digest(universe_digest, ordinal)? != self.candidate_digest {
            return Err("V2 replay candidate identity differs from its evidence".to_owned());
        }
        Ok(())
    }

    fn derived_digest(
        self,
        universe_digest: [u8; 32],
        ordinal: usize,
    ) -> Result<[u8; 32], GlobalReplayRefusalV2> {
        let mut hasher = Hasher::new();
        hasher.update(CANDIDATE_DOMAIN_V2);
        hasher.update(&universe_digest);
        hasher.update(&usize_u64(ordinal, "V2 candidate ordinal")?.to_le_bytes());
        hasher.update(&self.signal_micros.to_le_bytes());
        hasher.update(&self.entry_micros.to_le_bytes());
        hasher.update(&self.occupied_through_micros.to_le_bytes());
        hasher.update(&[self.path.byte()]);
        Ok(hasher.finalize())
    }
}

/// Exhaustive scheduler terminal retained for each reachable candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobalReplayDispositionV2 {
    /// This candidate acquired global occupancy.
    Admitted {
        /// Inclusive minute through which the admitted position owns the portfolio.
        occupied_through_micros: i64,
    },
    /// A position from an earlier minute still occupied the portfolio.
    BlockedOccupied {
        /// Inclusive occupancy boundary of the already-open position.
        occupied_through_micros: i64,
    },
    /// Another candidate won this same minute by canonical priority/digest order.
    BlockedSimultaneous {
        /// Strategy identity of the canonical same-minute winner.
        admitted_strategy_digest: [u8; 32],
    },
}

/// One immutable V2 global scheduling decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlobalReplayDecisionV2 {
    decision_digest: [u8; 32],
    sequence: u64,
    stream_ordinal: u16,
    candidate_ordinal: u64,
    candidate_digest: [u8; 32],
    entry_micros: i64,
    strategy_digest: [u8; 32],
    disposition: GlobalReplayDispositionV2,
    path: ReplayCandidatePathV2,
}

impl GlobalReplayDecisionV2 {
    /// Content-derived decision identity.
    #[must_use]
    pub const fn decision_digest(self) -> [u8; 32] {
        self.decision_digest
    }

    /// Terminal global-position outcome.
    #[must_use]
    pub const fn disposition(self) -> GlobalReplayDispositionV2 {
        self.disposition
    }

    /// Whether money evidence was refused while occupancy remained real.
    #[must_use]
    pub const fn candidate_path(self) -> ReplayCandidatePathV2 {
        self.path
    }

    fn new(
        sequence: u64,
        stream: &VerifiedReplayStreamV2,
        candidate_ordinal: usize,
        candidate: CandidateScheduleEvidenceV2,
        disposition: Disposition,
    ) -> Result<Self, GlobalReplayRefusalV2> {
        let disposition = match disposition {
            Disposition::Admitted {
                occupied_through_micros,
            } => GlobalReplayDispositionV2::Admitted {
                occupied_through_micros,
            },
            Disposition::BlockedOccupied {
                occupied_through_micros,
            } => GlobalReplayDispositionV2::BlockedOccupied {
                occupied_through_micros,
            },
            Disposition::BlockedSimultaneous { admitted } => {
                GlobalReplayDispositionV2::BlockedSimultaneous {
                    admitted_strategy_digest: admitted.bytes(),
                }
            }
            Disposition::Unreachable | Disposition::Refused(_) => {
                return Err(
                    "reachable V2 replay candidate became unreachable/refused in scheduling"
                        .to_owned(),
                );
            }
        };
        let mut value = Self {
            decision_digest: [0; 32],
            sequence,
            stream_ordinal: stream.stream_ordinal,
            candidate_ordinal: usize_u64(candidate_ordinal, "V2 decision candidate ordinal")?,
            candidate_digest: candidate.candidate_digest,
            entry_micros: candidate.entry_micros,
            strategy_digest: stream.strategy_digest,
            disposition,
            path: candidate.path,
        };
        value.decision_digest = value.derived_digest();
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<(), GlobalReplayRefusalV2> {
        require_digest("V2 global decision", &self.decision_digest)?;
        require_digest("V2 decision candidate", &self.candidate_digest)?;
        require_digest("V2 decision strategy", &self.strategy_digest)?;
        if usize::from(self.stream_ordinal) >= MAX_STREAMS {
            return Err("V2 global decision stream ordinal exceeds 200".to_owned());
        }
        match self.disposition {
            GlobalReplayDispositionV2::Admitted {
                occupied_through_micros,
            }
            | GlobalReplayDispositionV2::BlockedOccupied {
                occupied_through_micros,
            } if occupied_through_micros < self.entry_micros => {
                return Err("V2 global decision occupancy ends before entry".to_owned());
            }
            GlobalReplayDispositionV2::BlockedSimultaneous {
                admitted_strategy_digest,
            } => require_digest("V2 simultaneous winner", &admitted_strategy_digest)?,
            GlobalReplayDispositionV2::Admitted { .. }
            | GlobalReplayDispositionV2::BlockedOccupied { .. } => {}
        }
        if self.derived_digest() != self.decision_digest {
            return Err("V2 global decision identity differs from its fields".to_owned());
        }
        Ok(())
    }

    fn derived_digest(self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(DECISION_DOMAIN_V2);
        hasher.update(&self.sequence.to_le_bytes());
        hasher.update(&self.stream_ordinal.to_le_bytes());
        hasher.update(&self.candidate_ordinal.to_le_bytes());
        hasher.update(&self.candidate_digest);
        hasher.update(&self.entry_micros.to_le_bytes());
        hasher.update(&self.strategy_digest);
        match self.disposition {
            GlobalReplayDispositionV2::Admitted {
                occupied_through_micros,
            } => {
                hasher.update(&[1]);
                hasher.update(&occupied_through_micros.to_le_bytes());
                hasher.update(&[0; 32]);
            }
            GlobalReplayDispositionV2::BlockedOccupied {
                occupied_through_micros,
            } => {
                hasher.update(&[2]);
                hasher.update(&occupied_through_micros.to_le_bytes());
                hasher.update(&[0; 32]);
            }
            GlobalReplayDispositionV2::BlockedSimultaneous {
                admitted_strategy_digest,
            } => {
                hasher.update(&[3]);
                hasher.update(&i64::MIN.to_le_bytes());
                hasher.update(&admitted_strategy_digest);
            }
        }
        hasher.update(&[self.path.byte()]);
        hasher.finalize()
    }
}

/// Fully reconstructed and globally scheduled V2 execution authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedGlobalReplayV2 {
    manifest: GlobalReplayManifestV2,
    streams: Vec<VerifiedReplayStreamV2>,
    decisions: Vec<GlobalReplayDecisionV2>,
    counters: Counters,
    pricing_refused_candidates: u64,
    admitted_pricing_refused: u64,
    replay_id: [u8; 32],
    ordered_stream_digest: [u8; 32],
    ordered_decision_digest: [u8; 32],
}

impl PreparedGlobalReplayV2 {
    /// V2 replay identity binding manifest, reconstructed streams and decisions.
    #[must_use]
    pub const fn replay_id(&self) -> [u8; 32] {
        self.replay_id
    }

    /// Exact Selection-V4/Execution-V2 manifest.
    #[must_use]
    pub const fn manifest(&self) -> &GlobalReplayManifestV2 {
        &self.manifest
    }

    /// Exactly 200 verified streams when preparation succeeds.
    #[must_use]
    pub fn streams(&self) -> &[VerifiedReplayStreamV2] {
        &self.streams
    }

    /// One exhaustive scheduler decision per reachable candidate.
    #[must_use]
    pub fn decisions(&self) -> &[GlobalReplayDecisionV2] {
        &self.decisions
    }

    /// Reconciled one-global-position counters.
    #[must_use]
    pub const fn counters(&self) -> Counters {
        self.counters
    }

    /// Known entries whose price path was refused but still held occupancy.
    #[must_use]
    pub const fn pricing_refused_candidates(&self) -> u64 {
        self.pricing_refused_candidates
    }

    /// Pricing-refused candidates that acquired the global lock.
    #[must_use]
    pub const fn admitted_pricing_refused(&self) -> u64 {
        self.admitted_pricing_refused
    }

    /// Ordered stream identity included in the replay id.
    #[must_use]
    pub const fn ordered_stream_digest(&self) -> [u8; 32] {
        self.ordered_stream_digest
    }

    /// Ordered decision identity included in the replay id.
    #[must_use]
    pub const fn ordered_decision_digest(&self) -> [u8; 32] {
        self.ordered_decision_digest
    }

    fn validate(&self) -> Result<(), GlobalReplayRefusalV2> {
        self.manifest.validate()?;
        require_digest("global replay V2 identity", &self.replay_id)?;
        require_digest("global replay V2 stream order", &self.ordered_stream_digest)?;
        require_digest(
            "global replay V2 decision order",
            &self.ordered_decision_digest,
        )?;
        if self.streams.len() != MAX_STREAMS {
            return Err(format!(
                "global replay V2 has {} reconstructed streams, not {MAX_STREAMS}",
                self.streams.len()
            ));
        }
        for (ordinal, stream) in self.streams.iter().enumerate() {
            stream.validate()?;
            if usize::from(stream.stream_ordinal) != ordinal {
                return Err(
                    "global replay V2 streams are not in canonical ordinal order".to_owned(),
                );
            }
        }
        for (sequence, decision) in self.decisions.iter().copied().enumerate() {
            decision.validate()?;
            if decision.sequence != usize_u64(sequence, "V2 decision sequence")? {
                return Err("global replay V2 decisions are not contiguous".to_owned());
            }
        }
        if !self.counters.reconciles()
            || self.counters.offered != usize_u64(self.decisions.len(), "V2 decision count")?
            || self.counters.unreachable != 0
            || self.counters.refused != 0
        {
            return Err("global replay V2 scheduler counters do not reconcile".to_owned());
        }
        if self.pricing_refused_candidates
            != self.streams.iter().try_fold(0_u64, |sum, stream| {
                sum.checked_add(stream.pricing_refused_paths)
                    .ok_or_else(|| "V2 pricing-refused count overflowed".to_owned())
            })?
        {
            return Err("global replay V2 pricing-refused total differs from streams".to_owned());
        }
        if self.admitted_pricing_refused > self.counters.admitted {
            return Err("V2 admitted pricing-refused count exceeds admissions".to_owned());
        }
        if stream_order_digest(&self.streams)? != self.ordered_stream_digest
            || decision_order_digest(&self.decisions)? != self.ordered_decision_digest
            || derive_replay_id(
                &self.manifest,
                self.ordered_stream_digest,
                self.ordered_decision_digest,
                self.counters,
                self.pricing_refused_candidates,
                self.admitted_pricing_refused,
            ) != self.replay_id
        {
            return Err("global replay V2 identity differs from reconstructed evidence".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DurableStreamHeaderV2 {
    stream_id: [u8; 32],
    stream_ordinal: u16,
    rank: u16,
    rung_seconds: u32,
    family: InstrumentFamilyV1,
    direction: TradeDirectionV1,
    selection_id: [u8; 32],
    population_id: [u8; 32],
    row_sequence: u64,
    strategy_digest: [u8; 32],
    execution_completion_id: [u8; 32],
    execution_disposition_id: [u8; 32],
    execution_capability_id: [u8; 32],
    training_run_id: [u8; 32],
    selected_exit_digest: [u8; 32],
    oos_run_id: [u8; 32],
    universe_digest: [u8; 32],
    pricing_refused_paths: u64,
    candidate_first: u64,
    candidate_count: u64,
}

impl DurableStreamHeaderV2 {
    fn from_stream(
        stream: &VerifiedReplayStreamV2,
        candidate_first: u64,
    ) -> Result<Self, GlobalReplayRefusalV2> {
        stream.validate()?;
        let value = Self {
            stream_id: stream.stream_id,
            stream_ordinal: stream.stream_ordinal,
            rank: stream.rank,
            rung_seconds: stream.rung_seconds,
            family: stream.family,
            direction: stream.direction,
            selection_id: stream.selection_id,
            population_id: stream.population_id,
            row_sequence: stream.row_sequence,
            strategy_digest: stream.strategy_digest,
            execution_completion_id: stream.execution_completion_id,
            execution_disposition_id: stream.execution_disposition_id,
            execution_capability_id: stream.execution_capability_id,
            training_run_id: stream.training_run_id,
            selected_exit_digest: stream.selected_exit_digest,
            oos_run_id: stream.oos_run_id,
            universe_digest: stream.universe_digest,
            pricing_refused_paths: stream.pricing_refused_paths,
            candidate_first,
            candidate_count: usize_u64(stream.candidates.len(), "V2 durable candidate count")?,
        };
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<(), GlobalReplayRefusalV2> {
        if usize::from(self.stream_ordinal) >= MAX_STREAMS || self.rank == 0 || self.rank > 25 {
            return Err("durable V2 stream has invalid ordinal/rank".to_owned());
        }
        validate_rung(self.rung_seconds)?;
        for (name, digest) in [
            ("durable V2 stream", self.stream_id),
            ("durable V2 stream selection", self.selection_id),
            ("durable V2 stream population", self.population_id),
            ("durable V2 stream strategy", self.strategy_digest),
            (
                "durable V2 stream execution completion",
                self.execution_completion_id,
            ),
            (
                "durable V2 stream execution disposition",
                self.execution_disposition_id,
            ),
            (
                "durable V2 stream execution capability",
                self.execution_capability_id,
            ),
            ("durable V2 stream training run", self.training_run_id),
            ("durable V2 stream selected exit", self.selected_exit_digest),
            ("durable V2 stream OOS run", self.oos_run_id),
            ("durable V2 stream universe", self.universe_digest),
        ] {
            require_digest(name, &digest)?;
        }
        self.candidate_first
            .checked_add(self.candidate_count)
            .ok_or_else(|| "durable V2 stream candidate range overflowed".to_owned())?;
        if self.pricing_refused_paths > self.candidate_count || self.derived_id() != self.stream_id
        {
            return Err("durable V2 stream header differs from its identity".to_owned());
        }
        Ok(())
    }

    fn derived_id(self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(STREAM_DOMAIN_V2);
        hasher.update(&self.stream_ordinal.to_le_bytes());
        hasher.update(&self.rank.to_le_bytes());
        hasher.update(&self.rung_seconds.to_le_bytes());
        hasher.update(&[family_byte(self.family), direction_byte(self.direction)]);
        hasher.update(&self.selection_id);
        hasher.update(&self.population_id);
        hasher.update(&self.row_sequence.to_le_bytes());
        for digest in [
            self.strategy_digest,
            self.execution_completion_id,
            self.execution_disposition_id,
            self.execution_capability_id,
            self.training_run_id,
            self.selected_exit_digest,
            self.oos_run_id,
            self.universe_digest,
        ] {
            hasher.update(&digest);
        }
        hasher.update(&self.candidate_count.to_le_bytes());
        hasher.update(&self.pricing_refused_paths.to_le_bytes());
        hasher.finalize()
    }

    fn into_stream(
        self,
        candidates: Vec<CandidateScheduleEvidenceV2>,
    ) -> Result<VerifiedReplayStreamV2, GlobalReplayRefusalV2> {
        self.validate()?;
        if usize_u64(candidates.len(), "durable V2 stream candidate count")? != self.candidate_count
        {
            return Err("durable V2 stream candidate block length differs".to_owned());
        }
        let stream = VerifiedReplayStreamV2 {
            stream_id: self.stream_id,
            stream_ordinal: self.stream_ordinal,
            rank: self.rank,
            rung_seconds: self.rung_seconds,
            family: self.family,
            direction: self.direction,
            selection_id: self.selection_id,
            population_id: self.population_id,
            row_sequence: self.row_sequence,
            strategy_digest: self.strategy_digest,
            execution_completion_id: self.execution_completion_id,
            execution_disposition_id: self.execution_disposition_id,
            execution_capability_id: self.execution_capability_id,
            training_run_id: self.training_run_id,
            selected_exit_digest: self.selected_exit_digest,
            oos_run_id: self.oos_run_id,
            universe_digest: self.universe_digest,
            pricing_refused_paths: self.pricing_refused_paths,
            candidates,
            cursor: 0,
        };
        stream.validate()?;
        Ok(stream)
    }

    fn to_bytes(self) -> Result<[u8; GLOBAL_REPLAY_STREAM_STRIDE_V2], GlobalReplayRefusalV2> {
        self.validate()?;
        let mut payload = [0; STREAM_PAYLOAD_BYTES_V2];
        let mut encoder = Encoder::new(&mut payload);
        encoder.bytes(&self.stream_id)?;
        encoder.u16(self.stream_ordinal)?;
        encoder.u16(self.rank)?;
        encoder.u32(self.rung_seconds)?;
        encoder.u8(family_byte(self.family))?;
        encoder.u8(direction_byte(self.direction))?;
        encoder.zeros(6)?;
        encoder.bytes(&self.selection_id)?;
        encoder.bytes(&self.population_id)?;
        encoder.u64(self.row_sequence)?;
        for digest in [
            self.strategy_digest,
            self.execution_completion_id,
            self.execution_disposition_id,
            self.execution_capability_id,
            self.training_run_id,
            self.selected_exit_digest,
            self.oos_run_id,
            self.universe_digest,
        ] {
            encoder.bytes(&digest)?;
        }
        encoder.u64(self.pricing_refused_paths)?;
        encoder.u64(self.candidate_first)?;
        encoder.u64(self.candidate_count)?;
        encoder.finish()?;
        with_seal_v2(payload)
    }

    fn from_bytes(
        raw: &[u8; GLOBAL_REPLAY_STREAM_STRIDE_V2],
    ) -> Result<Self, GlobalReplayRefusalV2> {
        let payload = checked_payload_v2::<STREAM_PAYLOAD_BYTES_V2, GLOBAL_REPLAY_STREAM_STRIDE_V2>(
            raw,
            "global replay V2 stream",
        )?;
        let mut decoder = Decoder::new(&payload);
        let stream_id = decoder.array_32()?;
        let stream_ordinal = decoder.u16()?;
        let rank = decoder.u16()?;
        let rung_seconds = decoder.u32()?;
        let family = decode_family_v2(decoder.u8()?)?;
        let direction = decode_direction_v2(decoder.u8()?)?;
        decoder.zeros(6)?;
        let selection_id = decoder.array_32()?;
        let population_id = decoder.array_32()?;
        let row_sequence = decoder.u64()?;
        let strategy_digest = decoder.array_32()?;
        let execution_completion_id = decoder.array_32()?;
        let execution_disposition_id = decoder.array_32()?;
        let execution_capability_id = decoder.array_32()?;
        let training_run_id = decoder.array_32()?;
        let selected_exit_digest = decoder.array_32()?;
        let oos_run_id = decoder.array_32()?;
        let universe_digest = decoder.array_32()?;
        let pricing_refused_paths = decoder.u64()?;
        let candidate_first = decoder.u64()?;
        let candidate_count = decoder.u64()?;
        decoder.finish()?;
        let value = Self {
            stream_id,
            stream_ordinal,
            rank,
            rung_seconds,
            family,
            direction,
            selection_id,
            population_id,
            row_sequence,
            strategy_digest,
            execution_completion_id,
            execution_disposition_id,
            execution_capability_id,
            training_run_id,
            selected_exit_digest,
            oos_run_id,
            universe_digest,
            pricing_refused_paths,
            candidate_first,
            candidate_count,
        };
        value.validate()?;
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DurableCandidateV2 {
    candidate_digest: [u8; 32],
    stream_id: [u8; 32],
    ordinal: u64,
    signal_micros: i64,
    entry_micros: i64,
    occupied_through_micros: i64,
    path: ReplayCandidatePathV2,
}

impl DurableCandidateV2 {
    fn new(
        stream: &VerifiedReplayStreamV2,
        ordinal: usize,
        candidate: CandidateScheduleEvidenceV2,
    ) -> Result<Self, GlobalReplayRefusalV2> {
        candidate.validate(stream.universe_digest, ordinal)?;
        let value = Self {
            candidate_digest: candidate.candidate_digest,
            stream_id: stream.stream_id,
            ordinal: usize_u64(ordinal, "durable V2 candidate ordinal")?,
            signal_micros: candidate.signal_micros,
            entry_micros: candidate.entry_micros,
            occupied_through_micros: candidate.occupied_through_micros,
            path: candidate.path,
        };
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<(), GlobalReplayRefusalV2> {
        require_digest("durable V2 candidate", &self.candidate_digest)?;
        require_digest("durable V2 candidate stream", &self.stream_id)?;
        if self.signal_micros >= self.entry_micros
            || self.occupied_through_micros < self.entry_micros
        {
            return Err("durable V2 candidate has impossible time order".to_owned());
        }
        Ok(())
    }

    fn evidence_for(
        self,
        stream: &DurableStreamHeaderV2,
        ordinal: usize,
    ) -> Result<CandidateScheduleEvidenceV2, GlobalReplayRefusalV2> {
        self.validate()?;
        if self.stream_id != stream.stream_id
            || self.ordinal != usize_u64(ordinal, "durable V2 candidate ordinal")?
        {
            return Err("durable V2 candidate is outside its stream/ordinal".to_owned());
        }
        let value = CandidateScheduleEvidenceV2 {
            candidate_digest: self.candidate_digest,
            signal_micros: self.signal_micros,
            entry_micros: self.entry_micros,
            occupied_through_micros: self.occupied_through_micros,
            path: self.path,
        };
        value.validate(stream.universe_digest, ordinal)?;
        Ok(value)
    }

    fn to_bytes(self) -> Result<[u8; GLOBAL_REPLAY_CANDIDATE_STRIDE_V2], GlobalReplayRefusalV2> {
        self.validate()?;
        let mut payload = [0; CANDIDATE_PAYLOAD_BYTES_V2];
        let mut encoder = Encoder::new(&mut payload);
        encoder.bytes(&self.candidate_digest)?;
        encoder.bytes(&self.stream_id)?;
        encoder.u64(self.ordinal)?;
        encoder.i64(self.signal_micros)?;
        encoder.i64(self.entry_micros)?;
        encoder.i64(self.occupied_through_micros)?;
        encoder.u8(self.path.byte())?;
        encoder.zeros(7)?;
        encoder.finish()?;
        with_seal_v2(payload)
    }

    fn from_bytes(
        raw: &[u8; GLOBAL_REPLAY_CANDIDATE_STRIDE_V2],
    ) -> Result<Self, GlobalReplayRefusalV2> {
        let payload = checked_payload_v2::<
            CANDIDATE_PAYLOAD_BYTES_V2,
            GLOBAL_REPLAY_CANDIDATE_STRIDE_V2,
        >(raw, "global replay V2 candidate")?;
        let mut decoder = Decoder::new(&payload);
        let value = Self {
            candidate_digest: decoder.array_32()?,
            stream_id: decoder.array_32()?,
            ordinal: decoder.u64()?,
            signal_micros: decoder.i64()?,
            entry_micros: decoder.i64()?,
            occupied_through_micros: decoder.i64()?,
            path: decode_candidate_path_v2(decoder.u8()?)?,
        };
        decoder.zeros(7)?;
        decoder.finish()?;
        value.validate()?;
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DurableDecisionV2 {
    replay_id: [u8; 32],
    decision: GlobalReplayDecisionV2,
}

impl DurableDecisionV2 {
    fn new(
        replay_id: [u8; 32],
        decision: GlobalReplayDecisionV2,
    ) -> Result<Self, GlobalReplayRefusalV2> {
        let value = Self {
            replay_id,
            decision,
        };
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<(), GlobalReplayRefusalV2> {
        require_digest("durable V2 decision replay", &self.replay_id)?;
        self.decision.validate()
    }

    fn to_bytes(self) -> Result<[u8; GLOBAL_REPLAY_DECISION_STRIDE_V2], GlobalReplayRefusalV2> {
        self.validate()?;
        let (tag, occupied, simultaneous) = encode_disposition_v2(self.decision.disposition);
        let mut payload = [0; DECISION_PAYLOAD_BYTES_V2];
        let mut encoder = Encoder::new(&mut payload);
        encoder.bytes(&self.decision.decision_digest)?;
        encoder.bytes(&self.replay_id)?;
        encoder.u64(self.decision.sequence)?;
        encoder.u16(self.decision.stream_ordinal)?;
        encoder.zeros(6)?;
        encoder.u64(self.decision.candidate_ordinal)?;
        encoder.bytes(&self.decision.candidate_digest)?;
        encoder.i64(self.decision.entry_micros)?;
        encoder.bytes(&self.decision.strategy_digest)?;
        encoder.u8(tag)?;
        encoder.u8(self.decision.path.byte())?;
        encoder.zeros(6)?;
        encoder.i64(occupied)?;
        encoder.bytes(&simultaneous)?;
        encoder.finish()?;
        with_seal_v2(payload)
    }

    fn from_bytes(
        raw: &[u8; GLOBAL_REPLAY_DECISION_STRIDE_V2],
    ) -> Result<Self, GlobalReplayRefusalV2> {
        let payload = checked_payload_v2::<
            DECISION_PAYLOAD_BYTES_V2,
            GLOBAL_REPLAY_DECISION_STRIDE_V2,
        >(raw, "global replay V2 decision")?;
        let mut decoder = Decoder::new(&payload);
        let decision_digest = decoder.array_32()?;
        let replay_id = decoder.array_32()?;
        let sequence = decoder.u64()?;
        let stream_ordinal = decoder.u16()?;
        decoder.zeros(6)?;
        let candidate_ordinal = decoder.u64()?;
        let candidate_digest = decoder.array_32()?;
        let entry_micros = decoder.i64()?;
        let strategy_digest = decoder.array_32()?;
        let disposition_tag = decoder.u8()?;
        let path = decode_candidate_path_v2(decoder.u8()?)?;
        decoder.zeros(6)?;
        let occupied = decoder.i64()?;
        let simultaneous = decoder.array_32()?;
        decoder.finish()?;
        let value = Self {
            replay_id,
            decision: GlobalReplayDecisionV2 {
                decision_digest,
                sequence,
                stream_ordinal,
                candidate_ordinal,
                candidate_digest,
                entry_micros,
                strategy_digest,
                disposition: decode_disposition_v2(disposition_tag, occupied, simultaneous)?,
                path,
            },
        };
        value.validate()?;
        Ok(value)
    }
}

/// Receipt-last execution-only publication of one V2 global replay.
///
/// This receipt intentionally asserts no admitted-money rows and no VIX stamps:
/// neither authority is modeled by the current V2 preparation types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlobalReplayCompletionV2 {
    completion_id: [u8; 32],
    replay_id: [u8; 32],
    manifest_id: [u8; 32],
    ordered_stream_digest: [u8; 32],
    ordered_candidate_digest: [u8; 32],
    ordered_decision_digest: [u8; 32],
    execution_law_digest: [u8; 32],
    authority_scope_digest: [u8; 32],
    manifest_first: u64,
    manifest_count: u64,
    stream_first: u64,
    stream_count: u64,
    candidate_first: u64,
    candidate_count: u64,
    decision_first: u64,
    decision_count: u64,
    counters: Counters,
    pricing_refused_candidates: u64,
    admitted_pricing_refused: u64,
}

impl GlobalReplayCompletionV2 {
    /// Content-derived receipt identity.
    #[must_use]
    pub const fn completion_id(self) -> [u8; 32] {
        self.completion_id
    }

    /// Exact scheduled replay identity.
    #[must_use]
    pub const fn replay_id(self) -> [u8; 32] {
        self.replay_id
    }

    /// Exact Selection-V4/Execution-V2 manifest identity.
    #[must_use]
    pub const fn manifest_id(self) -> [u8; 32] {
        self.manifest_id
    }

    /// Number of reconstructed streams covered by this receipt.
    #[must_use]
    pub const fn stream_count(self) -> u64 {
        self.stream_count
    }

    /// Number of exhaustive candidate/decision rows covered by this receipt.
    #[must_use]
    pub const fn decision_count(self) -> u64 {
        self.decision_count
    }

    /// Reconciled global-single-position counters.
    #[must_use]
    pub const fn counters(self) -> Counters {
        self.counters
    }

    fn for_prepared(
        prepared: &PreparedGlobalReplayV2,
        manifest_first: u64,
        stream_first: u64,
        candidate_first: u64,
        decision_first: u64,
    ) -> Result<Self, GlobalReplayRefusalV2> {
        prepared.validate()?;
        let mut value = Self {
            completion_id: [0; 32],
            replay_id: prepared.replay_id,
            manifest_id: prepared.manifest.manifest_id,
            ordered_stream_digest: prepared.ordered_stream_digest,
            ordered_candidate_digest: candidate_order_digest(&prepared.streams)?,
            ordered_decision_digest: prepared.ordered_decision_digest,
            execution_law_digest: exact_execution_law_digest_v1(),
            authority_scope_digest: execution_only_scope_digest_v2(),
            manifest_first,
            manifest_count: 1,
            stream_first,
            stream_count: usize_u64(prepared.streams.len(), "durable V2 stream count")?,
            candidate_first,
            candidate_count: prepared.counters.offered,
            decision_first,
            decision_count: usize_u64(prepared.decisions.len(), "durable V2 decision count")?,
            counters: prepared.counters,
            pricing_refused_candidates: prepared.pricing_refused_candidates,
            admitted_pricing_refused: prepared.admitted_pricing_refused,
        };
        value.completion_id = value.derived_id();
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<(), GlobalReplayRefusalV2> {
        for (name, digest) in [
            ("global replay V2 completion", self.completion_id),
            ("global replay V2 completion replay", self.replay_id),
            ("global replay V2 completion manifest", self.manifest_id),
            (
                "global replay V2 completion stream order",
                self.ordered_stream_digest,
            ),
            (
                "global replay V2 completion candidate order",
                self.ordered_candidate_digest,
            ),
            (
                "global replay V2 completion decision order",
                self.ordered_decision_digest,
            ),
            (
                "global replay V2 completion execution law",
                self.execution_law_digest,
            ),
            (
                "global replay V2 completion authority scope",
                self.authority_scope_digest,
            ),
        ] {
            require_digest(name, &digest)?;
        }
        if self.manifest_count != 1
            || self.stream_count != usize_u64(MAX_STREAMS, "global replay V2 stream count")?
            || self.candidate_count != self.decision_count
            || self.decision_count != self.counters.offered
            || !self.counters.reconciles()
            || self.counters.unreachable != 0
            || self.counters.refused != 0
            || self.pricing_refused_candidates > self.candidate_count
            || self.admitted_pricing_refused > self.counters.admitted
            || self.execution_law_digest != exact_execution_law_digest_v1()
            || self.authority_scope_digest != execution_only_scope_digest_v2()
        {
            return Err("global replay V2 completion counts or authority scope differ".to_owned());
        }
        for (first, count, subject) in [
            (self.manifest_first, self.manifest_count, "manifest"),
            (self.stream_first, self.stream_count, "stream"),
            (self.candidate_first, self.candidate_count, "candidate"),
            (self.decision_first, self.decision_count, "decision"),
        ] {
            first
                .checked_add(count)
                .ok_or_else(|| format!("global replay V2 completion {subject} range overflowed"))?;
        }
        if self.derived_id() != self.completion_id {
            return Err("global replay V2 completion identity differs from its fields".to_owned());
        }
        Ok(())
    }

    fn derived_id(self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(COMPLETION_ID_DOMAIN_V2);
        for digest in [
            self.replay_id,
            self.manifest_id,
            self.ordered_stream_digest,
            self.ordered_candidate_digest,
            self.ordered_decision_digest,
            self.execution_law_digest,
            self.authority_scope_digest,
        ] {
            hasher.update(&digest);
        }
        for value in [
            self.manifest_first,
            self.manifest_count,
            self.stream_first,
            self.stream_count,
            self.candidate_first,
            self.candidate_count,
            self.decision_first,
            self.decision_count,
            self.counters.offered,
            self.counters.admitted,
            self.counters.blocked_occupied,
            self.counters.blocked_simultaneous,
            self.counters.unreachable,
            self.counters.refused,
            self.pricing_refused_candidates,
            self.admitted_pricing_refused,
        ] {
            hasher.update(&value.to_le_bytes());
        }
        hasher.finalize()
    }

    fn to_bytes(self) -> Result<[u8; GLOBAL_REPLAY_COMPLETION_STRIDE_V2], GlobalReplayRefusalV2> {
        self.validate()?;
        let mut payload = [0; COMPLETION_PAYLOAD_BYTES_V2];
        let mut encoder = Encoder::new(&mut payload);
        for digest in [
            self.completion_id,
            self.replay_id,
            self.manifest_id,
            self.ordered_stream_digest,
            self.ordered_candidate_digest,
            self.ordered_decision_digest,
            self.execution_law_digest,
            self.authority_scope_digest,
        ] {
            encoder.bytes(&digest)?;
        }
        for value in [
            self.manifest_first,
            self.manifest_count,
            self.stream_first,
            self.stream_count,
            self.candidate_first,
            self.candidate_count,
            self.decision_first,
            self.decision_count,
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
        encoder.finish()?;
        with_seal_v2(payload)
    }

    fn from_bytes(
        raw: &[u8; GLOBAL_REPLAY_COMPLETION_STRIDE_V2],
    ) -> Result<Self, GlobalReplayRefusalV2> {
        let payload = checked_payload_v2::<
            COMPLETION_PAYLOAD_BYTES_V2,
            GLOBAL_REPLAY_COMPLETION_STRIDE_V2,
        >(raw, "global replay V2 completion")?;
        let mut decoder = Decoder::new(&payload);
        let completion_id = decoder.array_32()?;
        let replay_id = decoder.array_32()?;
        let manifest_id = decoder.array_32()?;
        let ordered_stream_digest = decoder.array_32()?;
        let ordered_candidate_digest = decoder.array_32()?;
        let ordered_decision_digest = decoder.array_32()?;
        let execution_law_digest = decoder.array_32()?;
        let authority_scope_digest = decoder.array_32()?;
        let manifest_first = decoder.u64()?;
        let manifest_count = decoder.u64()?;
        let stream_first = decoder.u64()?;
        let stream_count = decoder.u64()?;
        let candidate_first = decoder.u64()?;
        let candidate_count = decoder.u64()?;
        let decision_first = decoder.u64()?;
        let decision_count = decoder.u64()?;
        let counters = Counters {
            offered: decoder.u64()?,
            admitted: decoder.u64()?,
            blocked_occupied: decoder.u64()?,
            blocked_simultaneous: decoder.u64()?,
            unreachable: decoder.u64()?,
            refused: decoder.u64()?,
        };
        let pricing_refused_candidates = decoder.u64()?;
        let admitted_pricing_refused = decoder.u64()?;
        decoder.finish()?;
        let value = Self {
            completion_id,
            replay_id,
            manifest_id,
            ordered_stream_digest,
            ordered_candidate_digest,
            ordered_decision_digest,
            execution_law_digest,
            authority_scope_digest,
            manifest_first,
            manifest_count,
            stream_first,
            stream_count,
            candidate_first,
            candidate_count,
            decision_first,
            decision_count,
            counters,
            pricing_refused_candidates,
            admitted_pricing_refused,
        };
        value.validate()?;
        Ok(value)
    }
}

/// Outcome of an idempotent receipt-last V2 execution-authority commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobalReplayCommitV2 {
    /// New manifest/stream/candidate/decision blocks and completion were synced.
    Written,
    /// The exact already-committed replay was reconstructed and reused.
    Reused,
}

/// Append-only receipt-last V2 execution-authority ledger.
///
/// The ledger durably covers the exact V4 manifest, all 200 reconstructed
/// stream headers, every pre-exclusivity candidate, and every exhaustive
/// global-position decision. It deliberately does not claim money-row or VIX
/// authority because those fields do not exist in [`PreparedGlobalReplayV2`].
#[derive(Debug)]
pub struct GlobalReplayLedgerV2 {
    files: ReplayFilesV2,
    writer_lock: File,
    paths: ReplayPathsV2,
    file_digests: [[u8; 32]; 5],
    completions: HashMap<[u8; 32], GlobalReplayCompletionV2>,
    replays: HashMap<[u8; 32], PreparedGlobalReplayV2>,
    order: Vec<[u8; 32]>,
    max_completions: usize,
    writable: bool,
}

impl GlobalReplayLedgerV2 {
    /// Reconstructed-stream record path.
    #[must_use]
    pub fn stream_path(root: &Path) -> PathBuf {
        root.join("results/global-replay-streams-v2.bin")
    }

    /// Pre-exclusivity candidate record path.
    #[must_use]
    pub fn candidate_path(root: &Path) -> PathBuf {
        root.join("results/global-replay-candidates-v2.bin")
    }

    /// Exhaustive global scheduler decision path.
    #[must_use]
    pub fn decision_path(root: &Path) -> PathBuf {
        root.join("results/global-replay-decisions-v2.bin")
    }

    /// Receipt-last execution-only completion path.
    #[must_use]
    pub fn completion_path(root: &Path) -> PathBuf {
        root.join("results/global-replay-completions-v2.bin")
    }

    /// Shared writer lock for all five V2 files.
    #[must_use]
    pub fn lock_path(root: &Path) -> PathBuf {
        root.join("results/global-replay-write-v2.lock")
    }

    /// Opens or creates a writable V2 ledger and fully revalidates every record.
    ///
    /// # Errors
    ///
    /// Refuses a zero/insufficient bound; malformed header, record or seal;
    /// overlapping completion ranges; duplicate identity; scheduler replay
    /// disagreement; allocation, lock, sync, or I/O failure.
    pub fn open(root: &Path, max_completions: usize) -> Result<Self, GlobalReplayRefusalV2> {
        validate_completion_bound_v2(max_completions)?;
        fs::create_dir_all(root.join("results")).map_err(|why| {
            format!(
                "{} result directory could not be created: {why}",
                root.join("results").display()
            )
        })?;
        let paths = ReplayPathsV2::of(root);
        let writer_lock = open_or_create_v2(&paths.lock)?;
        writer_lock
            .lock()
            .map_err(|why| format!("{} could not be locked: {why}", paths.lock.display()))?;
        let opened = (|| {
            let mut files = ReplayFilesV2 {
                manifest: open_or_create_v2(&paths.manifest)?,
                stream: open_or_create_v2(&paths.stream)?,
                candidate: open_or_create_v2(&paths.candidate)?,
                decision: open_or_create_v2(&paths.decision)?,
                completion: open_or_create_v2(&paths.completion)?,
            };
            for (file, path, magic, stride) in [
                (
                    &mut files.manifest,
                    &paths.manifest,
                    LEDGER_MANIFEST_MAGIC_V2,
                    GLOBAL_REPLAY_MANIFEST_STRIDE_V2,
                ),
                (
                    &mut files.stream,
                    &paths.stream,
                    LEDGER_STREAM_MAGIC_V2,
                    GLOBAL_REPLAY_STREAM_STRIDE_V2,
                ),
                (
                    &mut files.candidate,
                    &paths.candidate,
                    LEDGER_CANDIDATE_MAGIC_V2,
                    GLOBAL_REPLAY_CANDIDATE_STRIDE_V2,
                ),
                (
                    &mut files.decision,
                    &paths.decision,
                    LEDGER_DECISION_MAGIC_V2,
                    GLOBAL_REPLAY_DECISION_STRIDE_V2,
                ),
                (
                    &mut files.completion,
                    &paths.completion,
                    LEDGER_COMPLETION_MAGIC_V2,
                    GLOBAL_REPLAY_COMPLETION_STRIDE_V2,
                ),
            ] {
                ensure_header_v2(file, path, magic, stride)?;
            }
            Self::from_files(
                files,
                paths.clone(),
                writer_lock.try_clone().map_err(|why| {
                    format!("global replay V2 writer lock could not be cloned: {why}")
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

    /// Opens and fully validates an existing V2 ledger without write permission.
    ///
    /// # Errors
    ///
    /// Refuses any absent file, invalid bound/header/record/seal/range,
    /// scheduler replay disagreement, duplicate identity, lock or I/O failure.
    pub fn open_read(root: &Path, max_completions: usize) -> Result<Self, GlobalReplayRefusalV2> {
        validate_completion_bound_v2(max_completions)?;
        let paths = ReplayPathsV2::of(root);
        let writer_lock = File::open(&paths.lock)
            .map_err(|why| format!("{} could not be opened: {why}", paths.lock.display()))?;
        writer_lock
            .lock_shared()
            .map_err(|why| format!("{} could not be shared-locked: {why}", paths.lock.display()))?;
        let opened = (|| {
            let files = ReplayFilesV2 {
                manifest: open_read_v2(&paths.manifest)?,
                stream: open_read_v2(&paths.stream)?,
                candidate: open_read_v2(&paths.candidate)?,
                decision: open_read_v2(&paths.decision)?,
                completion: open_read_v2(&paths.completion)?,
            };
            Self::from_files(
                files,
                paths.clone(),
                writer_lock.try_clone().map_err(|why| {
                    format!("global replay V2 writer lock could not be cloned: {why}")
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
        mut files: ReplayFilesV2,
        paths: ReplayPathsV2,
        writer_lock: File,
        max_completions: usize,
        writable: bool,
    ) -> Result<Self, GlobalReplayRefusalV2> {
        let records = scan_replay_records_v2(&mut files, &paths)?;
        let indexes = index_replay_completions_v2(&records, max_completions)?;
        let file_digests = digest_replay_files_v2(&mut files, &paths)?;
        Ok(Self {
            files,
            writer_lock,
            paths,
            file_digests,
            completions: indexes.completions,
            replays: indexes.replays,
            order: indexes.order,
            max_completions,
            writable,
        })
    }

    /// Number of receipt-last execution-only completions.
    #[must_use]
    pub fn completions(&self) -> usize {
        self.order.len()
    }

    /// Completion identities in append order.
    #[must_use]
    pub fn completion_ids(&self) -> &[[u8; 32]] {
        &self.order
    }

    /// Exact completion after full reopen validation.
    #[must_use]
    pub fn completion(&self, completion_id: &[u8; 32]) -> Option<&GlobalReplayCompletionV2> {
        self.completions.get(completion_id)
    }

    /// Fully reconstructed and re-scheduled replay after validated reopen.
    #[must_use]
    pub fn replay(&self, replay_id: &[u8; 32]) -> Option<&PreparedGlobalReplayV2> {
        self.replays.get(replay_id)
    }

    /// Appends/syncs manifest, candidates, stream headers and decisions before
    /// syncing the completion receipt last.
    ///
    /// # Errors
    ///
    /// Refuses a read-only/stale handle, invalid prepared replay, non-exact
    /// reuse, exhausted bound, encoding, lock, sync, or I/O failure.
    pub fn append_complete(
        &mut self,
        prepared: &PreparedGlobalReplayV2,
    ) -> Result<GlobalReplayCommitV2, GlobalReplayRefusalV2> {
        if !self.writable {
            return Err("a read-only global replay V2 ledger cannot append".to_owned());
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
        prepared: &PreparedGlobalReplayV2,
    ) -> Result<GlobalReplayCommitV2, GlobalReplayRefusalV2> {
        self.require_files_unchanged()?;
        if let Some(existing) = self.replays.get(&prepared.replay_id) {
            if existing != prepared {
                return Err(
                    "global replay V2 identity aliases non-identical prepared evidence".to_owned(),
                );
            }
            self.sync_all()?;
            return Ok(GlobalReplayCommitV2::Reused);
        }
        if self.order.len() >= self.max_completions {
            return Err(format!(
                "global replay V2 completion bound {} is exhausted; reopen with a larger explicit bound",
                self.max_completions
            ));
        }
        let receipt = self.completion_for(prepared)?;
        self.append_prepared_files(prepared, &receipt)?;
        self.refresh_file_digests()?;
        if self
            .completions
            .insert(receipt.completion_id, receipt)
            .is_some()
            || self
                .replays
                .insert(receipt.replay_id, prepared.clone())
                .is_some()
        {
            return Err("global replay V2 in-memory identity collision after append".to_owned());
        }
        self.order.push(receipt.completion_id);
        Ok(GlobalReplayCommitV2::Written)
    }

    fn completion_for(
        &mut self,
        prepared: &PreparedGlobalReplayV2,
    ) -> Result<GlobalReplayCompletionV2, GlobalReplayRefusalV2> {
        let manifest_first = record_count_v2(
            &mut self.files.manifest,
            &self.paths.manifest,
            GLOBAL_REPLAY_MANIFEST_STRIDE_V2,
        )?;
        let stream_first = record_count_v2(
            &mut self.files.stream,
            &self.paths.stream,
            GLOBAL_REPLAY_STREAM_STRIDE_V2,
        )?;
        let candidate_first = record_count_v2(
            &mut self.files.candidate,
            &self.paths.candidate,
            GLOBAL_REPLAY_CANDIDATE_STRIDE_V2,
        )?;
        let decision_first = record_count_v2(
            &mut self.files.decision,
            &self.paths.decision,
            GLOBAL_REPLAY_DECISION_STRIDE_V2,
        )?;
        GlobalReplayCompletionV2::for_prepared(
            prepared,
            manifest_first,
            stream_first,
            candidate_first,
            decision_first,
        )
    }

    fn append_prepared_files(
        &mut self,
        prepared: &PreparedGlobalReplayV2,
        receipt: &GlobalReplayCompletionV2,
    ) -> Result<(), GlobalReplayRefusalV2> {
        append_encoded_v2(
            &mut self.files.manifest,
            &self.paths.manifest,
            core::iter::once(prepared.manifest.canonical_bytes()),
        )?;
        sync_file_v2(&self.files.manifest, &self.paths.manifest)?;

        self.files.candidate.seek(SeekFrom::End(0)).map_err(|why| {
            format!(
                "{} append seek failed: {why}",
                self.paths.candidate.display()
            )
        })?;
        for stream in &prepared.streams {
            for (ordinal, candidate) in stream.candidates.iter().copied().enumerate() {
                let raw = DurableCandidateV2::new(stream, ordinal, candidate)?.to_bytes()?;
                self.files.candidate.write_all(&raw).map_err(|why| {
                    format!("{} append failed: {why}", self.paths.candidate.display())
                })?;
            }
        }
        sync_file_v2(&self.files.candidate, &self.paths.candidate)?;

        self.files
            .stream
            .seek(SeekFrom::End(0))
            .map_err(|why| format!("{} append seek failed: {why}", self.paths.stream.display()))?;
        let mut candidate_first = receipt.candidate_first;
        for stream in &prepared.streams {
            let header = DurableStreamHeaderV2::from_stream(stream, candidate_first)?;
            self.files
                .stream
                .write_all(&header.to_bytes()?)
                .map_err(|why| format!("{} append failed: {why}", self.paths.stream.display()))?;
            candidate_first = candidate_first
                .checked_add(header.candidate_count)
                .ok_or_else(|| "global replay V2 candidate append range overflowed".to_owned())?;
        }
        if candidate_first
            != receipt
                .candidate_first
                .checked_add(receipt.candidate_count)
                .ok_or_else(|| "global replay V2 receipt candidate range overflowed".to_owned())?
        {
            return Err("global replay V2 appended candidate range differs".to_owned());
        }
        sync_file_v2(&self.files.stream, &self.paths.stream)?;

        append_encoded_v2(
            &mut self.files.decision,
            &self.paths.decision,
            prepared
                .decisions
                .iter()
                .copied()
                .map(|decision| DurableDecisionV2::new(prepared.replay_id, decision)?.to_bytes()),
        )?;
        sync_file_v2(&self.files.decision, &self.paths.decision)?;

        append_encoded_v2(
            &mut self.files.completion,
            &self.paths.completion,
            core::iter::once((*receipt).to_bytes()),
        )?;
        sync_file_v2(&self.files.completion, &self.paths.completion)
    }

    fn require_files_unchanged(&mut self) -> Result<(), GlobalReplayRefusalV2> {
        if digest_replay_files_v2(&mut self.files, &self.paths)? != self.file_digests {
            return Err(
                "global replay V2 files changed after open; reopen and fully revalidate".to_owned(),
            );
        }
        Ok(())
    }

    fn refresh_file_digests(&mut self) -> Result<(), GlobalReplayRefusalV2> {
        self.file_digests = digest_replay_files_v2(&mut self.files, &self.paths)?;
        Ok(())
    }

    fn sync_all(&self) -> Result<(), GlobalReplayRefusalV2> {
        for (file, path) in [
            (&self.files.manifest, &self.paths.manifest),
            (&self.files.stream, &self.paths.stream),
            (&self.files.candidate, &self.paths.candidate),
            (&self.files.decision, &self.paths.decision),
            (&self.files.completion, &self.paths.completion),
        ] {
            sync_file_v2(file, path)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ReplayFilesV2 {
    manifest: File,
    stream: File,
    candidate: File,
    decision: File,
    completion: File,
}

#[derive(Clone, Debug)]
struct ReplayPathsV2 {
    manifest: PathBuf,
    stream: PathBuf,
    candidate: PathBuf,
    decision: PathBuf,
    completion: PathBuf,
    lock: PathBuf,
}

impl ReplayPathsV2 {
    fn of(root: &Path) -> Self {
        Self {
            manifest: GlobalReplayManifestV2::path(root),
            stream: GlobalReplayLedgerV2::stream_path(root),
            candidate: GlobalReplayLedgerV2::candidate_path(root),
            decision: GlobalReplayLedgerV2::decision_path(root),
            completion: GlobalReplayLedgerV2::completion_path(root),
            lock: GlobalReplayLedgerV2::lock_path(root),
        }
    }
}

struct ScannedReplayRecordsV2 {
    manifests: Vec<GlobalReplayManifestV2>,
    streams: Vec<DurableStreamHeaderV2>,
    candidates: Vec<DurableCandidateV2>,
    decisions: Vec<DurableDecisionV2>,
    completions: Vec<GlobalReplayCompletionV2>,
}

struct ReplayCompletionIndexesV2 {
    completions: HashMap<[u8; 32], GlobalReplayCompletionV2>,
    replays: HashMap<[u8; 32], PreparedGlobalReplayV2>,
    order: Vec<[u8; 32]>,
}

fn scan_replay_records_v2(
    files: &mut ReplayFilesV2,
    paths: &ReplayPathsV2,
) -> Result<ScannedReplayRecordsV2, GlobalReplayRefusalV2> {
    for (file, path, magic, stride) in [
        (
            &mut files.manifest,
            &paths.manifest,
            LEDGER_MANIFEST_MAGIC_V2,
            GLOBAL_REPLAY_MANIFEST_STRIDE_V2,
        ),
        (
            &mut files.stream,
            &paths.stream,
            LEDGER_STREAM_MAGIC_V2,
            GLOBAL_REPLAY_STREAM_STRIDE_V2,
        ),
        (
            &mut files.candidate,
            &paths.candidate,
            LEDGER_CANDIDATE_MAGIC_V2,
            GLOBAL_REPLAY_CANDIDATE_STRIDE_V2,
        ),
        (
            &mut files.decision,
            &paths.decision,
            LEDGER_DECISION_MAGIC_V2,
            GLOBAL_REPLAY_DECISION_STRIDE_V2,
        ),
        (
            &mut files.completion,
            &paths.completion,
            LEDGER_COMPLETION_MAGIC_V2,
            GLOBAL_REPLAY_COMPLETION_STRIDE_V2,
        ),
    ] {
        check_record_file_v2(file, path, magic, stride)?;
    }
    Ok(ScannedReplayRecordsV2 {
        manifests: scan_records_v2::<GLOBAL_REPLAY_MANIFEST_STRIDE_V2, GlobalReplayManifestV2>(
            &mut files.manifest,
            &paths.manifest,
            GlobalReplayManifestV2::from_canonical_bytes,
        )?,
        streams: scan_records_v2::<GLOBAL_REPLAY_STREAM_STRIDE_V2, DurableStreamHeaderV2>(
            &mut files.stream,
            &paths.stream,
            DurableStreamHeaderV2::from_bytes,
        )?,
        candidates: scan_records_v2::<GLOBAL_REPLAY_CANDIDATE_STRIDE_V2, DurableCandidateV2>(
            &mut files.candidate,
            &paths.candidate,
            DurableCandidateV2::from_bytes,
        )?,
        decisions: scan_records_v2::<GLOBAL_REPLAY_DECISION_STRIDE_V2, DurableDecisionV2>(
            &mut files.decision,
            &paths.decision,
            DurableDecisionV2::from_bytes,
        )?,
        completions: scan_records_v2::<GLOBAL_REPLAY_COMPLETION_STRIDE_V2, GlobalReplayCompletionV2>(
            &mut files.completion,
            &paths.completion,
            GlobalReplayCompletionV2::from_bytes,
        )?,
    })
}

fn index_replay_completions_v2(
    records: &ScannedReplayRecordsV2,
    max_completions: usize,
) -> Result<ReplayCompletionIndexesV2, GlobalReplayRefusalV2> {
    if records.completions.len() > max_completions {
        return Err(format!(
            "global replay V2 ledger has {} completions above caller bound {max_completions}",
            records.completions.len()
        ));
    }
    let mut completions = HashMap::new();
    completions
        .try_reserve(records.completions.len())
        .map_err(|why| format!("global replay V2 completion index allocation refused: {why}"))?;
    let mut replays = HashMap::new();
    replays
        .try_reserve(records.completions.len())
        .map_err(|why| format!("global replay V2 replay index allocation refused: {why}"))?;
    let mut order = Vec::new();
    order
        .try_reserve_exact(records.completions.len())
        .map_err(|why| format!("global replay V2 completion order allocation refused: {why}"))?;
    let mut manifest_end = 0;
    let mut stream_end = 0;
    let mut candidate_end = 0;
    let mut decision_end = 0;
    for receipt in records.completions.iter().copied() {
        require_monotonic_block_v2(
            receipt.manifest_first,
            receipt.manifest_count,
            &mut manifest_end,
            "global replay V2 manifest",
        )?;
        require_monotonic_block_v2(
            receipt.stream_first,
            receipt.stream_count,
            &mut stream_end,
            "global replay V2 stream",
        )?;
        require_monotonic_block_v2(
            receipt.candidate_first,
            receipt.candidate_count,
            &mut candidate_end,
            "global replay V2 candidate",
        )?;
        require_monotonic_block_v2(
            receipt.decision_first,
            receipt.decision_count,
            &mut decision_end,
            "global replay V2 decision",
        )?;
        let prepared = validate_completion_blocks_v2(records, &receipt)?;
        if completions.insert(receipt.completion_id, receipt).is_some() {
            return Err("duplicate global replay V2 completion identity".to_owned());
        }
        if replays.insert(receipt.replay_id, prepared).is_some() {
            return Err("duplicate global replay V2 replay identity".to_owned());
        }
        order.push(receipt.completion_id);
    }
    Ok(ReplayCompletionIndexesV2 {
        completions,
        replays,
        order,
    })
}

fn validate_completion_blocks_v2(
    records: &ScannedReplayRecordsV2,
    receipt: &GlobalReplayCompletionV2,
) -> Result<PreparedGlobalReplayV2, GlobalReplayRefusalV2> {
    receipt.validate()?;
    let manifest_block = block_slice_v2(
        &records.manifests,
        receipt.manifest_first,
        receipt.manifest_count,
        "global replay V2 manifest",
    )?;
    let manifest = manifest_block
        .first()
        .cloned()
        .ok_or_else(|| "global replay V2 completion has no manifest".to_owned())?;
    if manifest.manifest_id != receipt.manifest_id {
        return Err("global replay V2 completion names a foreign manifest".to_owned());
    }
    let headers = block_slice_v2(
        &records.streams,
        receipt.stream_first,
        receipt.stream_count,
        "global replay V2 stream",
    )?;
    let durable_decisions = block_slice_v2(
        &records.decisions,
        receipt.decision_first,
        receipt.decision_count,
        "global replay V2 decision",
    )?;
    let mut streams = Vec::new();
    streams
        .try_reserve_exact(headers.len())
        .map_err(|why| format!("global replay V2 reopen stream allocation refused: {why}"))?;
    let mut next_candidate = receipt.candidate_first;
    for (ordinal, header) in headers.iter().copied().enumerate() {
        header.validate()?;
        if header.candidate_first != next_candidate {
            return Err(
                "global replay V2 stream candidate blocks are reordered, overlapping, or gapped"
                    .to_owned(),
            );
        }
        validate_manifest_stream_v2(&manifest, &header, ordinal)?;
        let candidate_records = block_slice_v2(
            &records.candidates,
            header.candidate_first,
            header.candidate_count,
            "global replay V2 stream candidate",
        )?;
        let mut candidates = Vec::new();
        candidates
            .try_reserve_exact(candidate_records.len())
            .map_err(|why| format!("global replay V2 candidate allocation refused: {why}"))?;
        for (candidate_ordinal, record) in candidate_records.iter().copied().enumerate() {
            candidates.push(record.evidence_for(&header, candidate_ordinal)?);
        }
        streams.push(header.into_stream(candidates)?);
        next_candidate = next_candidate
            .checked_add(header.candidate_count)
            .ok_or_else(|| "global replay V2 candidate block overflowed".to_owned())?;
    }
    if next_candidate
        != receipt
            .candidate_first
            .checked_add(receipt.candidate_count)
            .ok_or_else(|| "global replay V2 receipt candidate range overflowed".to_owned())?
    {
        return Err("global replay V2 stream blocks do not exhaust receipt candidates".to_owned());
    }
    let persisted_decisions: Vec<GlobalReplayDecisionV2> = durable_decisions
        .iter()
        .map(|record| {
            if record.replay_id == receipt.replay_id {
                Ok(record.decision)
            } else {
                Err("global replay V2 decision belongs to a foreign replay".to_owned())
            }
        })
        .collect::<Result<_, _>>()?;
    if stream_order_digest(&streams)? != receipt.ordered_stream_digest
        || candidate_order_digest(&streams)? != receipt.ordered_candidate_digest
        || decision_order_digest(&persisted_decisions)? != receipt.ordered_decision_digest
    {
        return Err("global replay V2 completion block order digest differs".to_owned());
    }
    let prepared = schedule_streams(manifest, streams)?;
    if prepared.replay_id != receipt.replay_id
        || prepared.ordered_stream_digest != receipt.ordered_stream_digest
        || prepared.ordered_decision_digest != receipt.ordered_decision_digest
        || prepared.decisions != persisted_decisions
        || prepared.counters != receipt.counters
        || prepared.pricing_refused_candidates != receipt.pricing_refused_candidates
        || prepared.admitted_pricing_refused != receipt.admitted_pricing_refused
    {
        return Err(
            "global replay V2 receipt differs from authoritative scheduler reconstruction"
                .to_owned(),
        );
    }
    Ok(prepared)
}

fn validate_manifest_stream_v2(
    manifest: &GlobalReplayManifestV2,
    header: &DurableStreamHeaderV2,
    ordinal: usize,
) -> Result<(), GlobalReplayRefusalV2> {
    if usize::from(header.stream_ordinal) != ordinal {
        return Err("global replay V2 durable stream ordinal is noncanonical".to_owned());
    }
    let selection_index = ordinal / 25;
    let expected_rank = u16::try_from((ordinal % 25) + 1)
        .map_err(|_| "global replay V2 durable rank does not fit u16".to_owned())?;
    let expected_rung = GLOBAL_REPLAY_V2_RUNGS_SECONDS
        .get(selection_index)
        .copied()
        .ok_or_else(|| "global replay V2 durable stream selection slot is absent".to_owned())?;
    let selection_id = manifest
        .selection_ids
        .get(selection_index)
        .copied()
        .ok_or_else(|| "global replay V2 manifest selection slot is absent".to_owned())?;
    let authority = manifest.authority(selection_index, header.family)?;
    if header.rank != expected_rank
        || header.rung_seconds != expected_rung
        || header.selection_id != selection_id
        || header.population_id != authority.population_id
        || header.execution_completion_id != authority.execution_v2_completion_id
    {
        return Err("global replay V2 stream differs from its manifest slot".to_owned());
    }
    Ok(())
}

fn candidate_order_digest(
    streams: &[VerifiedReplayStreamV2],
) -> Result<[u8; 32], GlobalReplayRefusalV2> {
    let total = streams.iter().try_fold(0_usize, |sum, stream| {
        sum.checked_add(stream.candidates.len())
            .ok_or_else(|| "global replay V2 candidate order count overflowed".to_owned())
    })?;
    let mut hasher = Hasher::new();
    hasher.update(CANDIDATE_ORDER_DOMAIN_V2);
    hasher.update(&usize_u64(total, "global replay V2 candidate order count")?.to_le_bytes());
    for stream in streams {
        hasher.update(&stream.stream_id);
        hasher.update(
            &usize_u64(
                stream.candidates.len(),
                "global replay V2 stream candidate count",
            )?
            .to_le_bytes(),
        );
        for candidate in &stream.candidates {
            hasher.update(&candidate.candidate_digest);
        }
    }
    Ok(hasher.finalize())
}

fn execution_only_scope_digest_v2() -> [u8; 32] {
    blake3::hash(EXECUTION_ONLY_SCOPE_DOMAIN_V2)
}

fn digest_replay_files_v2(
    files: &mut ReplayFilesV2,
    paths: &ReplayPathsV2,
) -> Result<[[u8; 32]; 5], GlobalReplayRefusalV2> {
    Ok([
        digest_file_v2(&mut files.manifest, &paths.manifest)?,
        digest_file_v2(&mut files.stream, &paths.stream)?,
        digest_file_v2(&mut files.candidate, &paths.candidate)?,
        digest_file_v2(&mut files.decision, &paths.decision)?,
        digest_file_v2(&mut files.completion, &paths.completion)?,
    ])
}

fn validate_completion_bound_v2(max_completions: usize) -> Result<(), GlobalReplayRefusalV2> {
    if max_completions == 0 {
        Err("global replay V2 completion bound must be nonzero".to_owned())
    } else {
        Ok(())
    }
}

fn open_or_create_v2(path: &Path) -> Result<File, GlobalReplayRefusalV2> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|why| format!("{} could not be opened: {why}", path.display()))
}

fn open_read_v2(path: &Path) -> Result<File, GlobalReplayRefusalV2> {
    File::open(path).map_err(|why| format!("{} could not be opened: {why}", path.display()))
}

fn ensure_header_v2(
    file: &mut File,
    path: &Path,
    magic: [u8; 8],
    stride: usize,
) -> Result<(), GlobalReplayRefusalV2> {
    if measured_len_v2(file, path)? != 0 {
        return Ok(());
    }
    let header = canonical_header_v2(magic, stride)?;
    file.write_all(&header)
        .and_then(|()| file.sync_all())
        .map_err(|why| {
            format!(
                "{} could not receive a durable V2 header: {why}",
                path.display()
            )
        })
}

fn canonical_header_v2(
    magic: [u8; 8],
    stride: usize,
) -> Result<[u8; LEDGER_HEADER_BYTES_USIZE_V2], GlobalReplayRefusalV2> {
    let mut raw = [0; LEDGER_HEADER_BYTES_USIZE_V2];
    let mut encoder = Encoder::new(&mut raw);
    encoder.bytes(&magic)?;
    encoder.u32(LEDGER_FORMAT_VERSION_V2)?;
    encoder.u32(
        u32::try_from(LEDGER_HEADER_BYTES_USIZE_V2)
            .map_err(|_| "global replay V2 header width does not fit u32".to_owned())?,
    )?;
    encoder.u32(
        u32::try_from(stride).map_err(|_| "global replay V2 stride does not fit u32".to_owned())?,
    )?;
    encoder.zeros(4)?;
    encoder.finish()?;
    Ok(raw)
}

fn check_record_file_v2(
    file: &mut File,
    path: &Path,
    magic: [u8; 8],
    stride: usize,
) -> Result<(), GlobalReplayRefusalV2> {
    let len = measured_len_v2(file, path)?;
    if len < LEDGER_HEADER_BYTES_V2 {
        return Err(format!(
            "{} is {len} bytes, shorter than its V2 header",
            path.display()
        ));
    }
    let body = len
        .checked_sub(LEDGER_HEADER_BYTES_V2)
        .ok_or_else(|| "global replay V2 file length underflowed".to_owned())?;
    let stride_u64 = usize_u64(stride, "global replay V2 record stride")?;
    if body % stride_u64 != 0 {
        return Err(format!(
            "{} has a ragged {body}-byte record body for stride {stride}",
            path.display()
        ));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("{} could not be seeked: {why}", path.display()))?;
    let mut raw = [0; LEDGER_HEADER_BYTES_USIZE_V2];
    file.read_exact(&mut raw)
        .map_err(|why| format!("{} header could not be read: {why}", path.display()))?;
    if raw != canonical_header_v2(magic, stride)? {
        return Err(format!(
            "{} V2 header fields are noncanonical",
            path.display()
        ));
    }
    Ok(())
}

fn measured_len_v2(file: &File, path: &Path) -> Result<u64, GlobalReplayRefusalV2> {
    file.metadata()
        .map(|metadata| metadata.len())
        .map_err(|why| format!("{} metadata could not be read: {why}", path.display()))
}

fn record_count_v2(
    file: &mut File,
    path: &Path,
    stride: usize,
) -> Result<u64, GlobalReplayRefusalV2> {
    let body = measured_len_v2(file, path)?
        .checked_sub(LEDGER_HEADER_BYTES_V2)
        .ok_or_else(|| format!("{} is shorter than its V2 header", path.display()))?;
    Ok(body / usize_u64(stride, "global replay V2 record stride")?)
}

fn scan_records_v2<const STRIDE: usize, T>(
    file: &mut File,
    path: &Path,
    decode: fn(&[u8; STRIDE]) -> Result<T, GlobalReplayRefusalV2>,
) -> Result<Vec<T>, GlobalReplayRefusalV2> {
    let count = record_count_v2(file, path, STRIDE)?;
    let capacity = u64_usize_v2(count, "global replay V2 record count")?;
    let mut records = Vec::new();
    records
        .try_reserve_exact(capacity)
        .map_err(|why| format!("{} record allocation refused: {why}", path.display()))?;
    file.seek(SeekFrom::Start(LEDGER_HEADER_BYTES_V2))
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

fn append_encoded_v2<const STRIDE: usize>(
    file: &mut File,
    path: &Path,
    records: impl IntoIterator<Item = Result<[u8; STRIDE], GlobalReplayRefusalV2>>,
) -> Result<(), GlobalReplayRefusalV2> {
    file.seek(SeekFrom::End(0))
        .map_err(|why| format!("{} could not be seeked for append: {why}", path.display()))?;
    for record in records {
        file.write_all(&record?)
            .map_err(|why| format!("{} append failed: {why}", path.display()))?;
    }
    Ok(())
}

fn sync_file_v2(file: &File, path: &Path) -> Result<(), GlobalReplayRefusalV2> {
    file.sync_all()
        .map_err(|why| format!("{} could not be synced: {why}", path.display()))
}

fn digest_file_v2(file: &mut File, path: &Path) -> Result<[u8; 32], GlobalReplayRefusalV2> {
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("{} could not be seeked for hashing: {why}", path.display()))?;
    let mut hasher = Hasher::new();
    let mut buffer = [0; 16_384];
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
                .ok_or_else(|| "global replay V2 file hash width is invalid".to_owned())?,
        );
    }
    Ok(hasher.finalize())
}

fn require_monotonic_block_v2(
    first: u64,
    count: u64,
    prior_end: &mut u64,
    subject: &str,
) -> Result<(), GlobalReplayRefusalV2> {
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

fn block_slice_v2<'a, T>(
    records: &'a [T],
    first: u64,
    count: u64,
    subject: &str,
) -> Result<&'a [T], GlobalReplayRefusalV2> {
    let first = u64_usize_v2(first, subject)?;
    let count = u64_usize_v2(count, subject)?;
    let end = first
        .checked_add(count)
        .ok_or_else(|| format!("{subject} block address overflowed"))?;
    records
        .get(first..end)
        .ok_or_else(|| format!("{subject} completion points outside its record file"))
}

fn with_seal_v2<const PAYLOAD: usize, const STRIDE: usize>(
    payload: [u8; PAYLOAD],
) -> Result<[u8; STRIDE], GlobalReplayRefusalV2> {
    if PAYLOAD.checked_add(SEAL_BYTES) != Some(STRIDE) {
        return Err("global replay V2 payload/stride contract differs".to_owned());
    }
    let mut raw = [0; STRIDE];
    raw.get_mut(..PAYLOAD)
        .ok_or_else(|| "global replay V2 payload slot is absent".to_owned())?
        .copy_from_slice(&payload);
    raw.get_mut(PAYLOAD..)
        .ok_or_else(|| "global replay V2 seal slot is absent".to_owned())?
        .copy_from_slice(&blake3::hash(&payload));
    Ok(raw)
}

fn checked_payload_v2<const PAYLOAD: usize, const STRIDE: usize>(
    raw: &[u8; STRIDE],
    subject: &str,
) -> Result<[u8; PAYLOAD], GlobalReplayRefusalV2> {
    if PAYLOAD.checked_add(SEAL_BYTES) != Some(STRIDE) {
        return Err(format!("{subject} payload/stride contract differs"));
    }
    let payload: [u8; PAYLOAD] = raw
        .get(..PAYLOAD)
        .ok_or_else(|| format!("{subject} payload is absent"))?
        .try_into()
        .map_err(|_| format!("{subject} payload width differs"))?;
    if raw
        .get(PAYLOAD..)
        .ok_or_else(|| format!("{subject} seal is absent"))?
        != blake3::hash(&payload)
    {
        return Err(format!("{subject} failed its complete BLAKE3 seal"));
    }
    Ok(payload)
}

const fn encode_disposition_v2(disposition: GlobalReplayDispositionV2) -> (u8, i64, [u8; 32]) {
    match disposition {
        GlobalReplayDispositionV2::Admitted {
            occupied_through_micros,
        } => (1, occupied_through_micros, [0; 32]),
        GlobalReplayDispositionV2::BlockedOccupied {
            occupied_through_micros,
        } => (2, occupied_through_micros, [0; 32]),
        GlobalReplayDispositionV2::BlockedSimultaneous {
            admitted_strategy_digest,
        } => (3, i64::MIN, admitted_strategy_digest),
    }
}

fn decode_disposition_v2(
    tag: u8,
    occupied_through_micros: i64,
    admitted_strategy_digest: [u8; 32],
) -> Result<GlobalReplayDispositionV2, GlobalReplayRefusalV2> {
    match tag {
        1 if admitted_strategy_digest == [0; 32] => Ok(GlobalReplayDispositionV2::Admitted {
            occupied_through_micros,
        }),
        2 if admitted_strategy_digest == [0; 32] => {
            Ok(GlobalReplayDispositionV2::BlockedOccupied {
                occupied_through_micros,
            })
        }
        3 if occupied_through_micros == i64::MIN && admitted_strategy_digest != [0; 32] => {
            Ok(GlobalReplayDispositionV2::BlockedSimultaneous {
                admitted_strategy_digest,
            })
        }
        _ => Err("global replay V2 disposition fields are noncanonical".to_owned()),
    }
}

fn decode_candidate_path_v2(byte: u8) -> Result<ReplayCandidatePathV2, GlobalReplayRefusalV2> {
    match byte {
        1 => Ok(ReplayCandidatePathV2::Priceable),
        2 => Ok(ReplayCandidatePathV2::BlockOnly),
        3 => Ok(ReplayCandidatePathV2::CrossingRefused),
        4 => Ok(ReplayCandidatePathV2::BlockOnlyAndCrossingRefused),
        _ => Err(format!(
            "global replay V2 candidate path tag {byte} is unknown"
        )),
    }
}

fn decode_family_v2(byte: u8) -> Result<InstrumentFamilyV1, GlobalReplayRefusalV2> {
    match byte {
        1 => Ok(InstrumentFamilyV1::Nifty),
        2 => Ok(InstrumentFamilyV1::BankNifty),
        _ => Err(format!("global replay V2 family tag {byte} is unknown")),
    }
}

fn decode_direction_v2(byte: u8) -> Result<TradeDirectionV1, GlobalReplayRefusalV2> {
    match byte {
        1 => Ok(TradeDirectionV1::Long),
        2 => Ok(TradeDirectionV1::Short),
        _ => Err(format!("global replay V2 direction tag {byte} is unknown")),
    }
}

/// Reclassifies all 200 selected rows, replays their OOS universes and applies
/// one global long/short position across both indices and all eight timeframes.
///
/// # Errors
///
/// Refuses every manifest error; missing, duplicate or foreign witness; a
/// population/row/parameter/disposition mismatch; changed training or OOS
/// evidence; a formerly authorized row becoming policy-refused; replay
/// integrity failure; scheduler refusal; or accounting/identity disagreement.
pub fn prepare_global_replay_v2(
    selection_authorities: &mut [ReplaySelectionAuthorityV2],
    execution: &mut ExecutionDispositionLedgerV2,
    witnesses: &[SelectedReplayWitnessV2<'_>],
) -> Result<PreparedGlobalReplayV2, GlobalReplayRefusalV2> {
    let manifest = GlobalReplayManifestV2::from_selections(selection_authorities, execution)?;
    let expected_streams = selection_authorities
        .iter()
        .try_fold(0_usize, |sum, authority| {
            sum.checked_add(authority.receipt().top_twenty_five().len())
                .ok_or_else(|| "global replay V2 selected-stream count overflowed".to_owned())
        })?;
    if expected_streams != MAX_STREAMS || witnesses.len() != expected_streams {
        return Err(format!(
            "global replay V2 requires exactly {MAX_STREAMS} selected witnesses, found {}",
            witnesses.len()
        ));
    }

    let mut witness_index = HashMap::new();
    witness_index
        .try_reserve(witnesses.len())
        .map_err(|why| format!("global replay V2 witness index allocation refused: {why}"))?;
    for (index, witness) in witnesses.iter().enumerate() {
        if witness.rank == 0 || witness.rank > 25 {
            return Err(format!(
                "Selection V4 {} witness rank {} is outside 1..=25",
                hex(&witness.selection_id),
                witness.rank
            ));
        }
        if witness_index
            .insert((witness.selection_id, witness.rank), index)
            .is_some()
        {
            return Err("global replay V2 has a duplicate selection/rank witness".to_owned());
        }
    }

    let mut streams = Vec::new();
    streams
        .try_reserve_exact(MAX_STREAMS)
        .map_err(|why| format!("global replay V2 stream allocation refused: {why}"))?;
    for (selection_index, selection_authority) in selection_authorities.iter_mut().enumerate() {
        let selection = selection_authority.receipt().clone();
        for (rank_zero, entry) in selection.top_twenty_five().iter().copied().enumerate() {
            let rank = u16::try_from(
                rank_zero
                    .checked_add(1)
                    .ok_or_else(|| "global replay V2 rank overflowed".to_owned())?,
            )
            .map_err(|_| "global replay V2 rank does not fit u16".to_owned())?;
            let witness_index_value = witness_index
                .remove(&(selection.selection_id(), rank))
                .ok_or_else(|| {
                    format!(
                        "Selection V4 {} rank {rank} has no exact V2 replay witness",
                        hex(&selection.selection_id())
                    )
                })?;
            let witness = witnesses
                .get(witness_index_value)
                .ok_or_else(|| "global replay V2 witness index disappeared".to_owned())?;
            let stream_ordinal = u16::try_from(streams.len())
                .map_err(|_| "global replay V2 stream ordinal does not fit u16".to_owned())?;
            streams.push(reconstruct_stream_v2(
                selection_index,
                &selection,
                rank,
                entry,
                stream_ordinal,
                witness,
                execution,
                &manifest,
                selection_authority,
            )?);
        }
    }
    if !witness_index.is_empty() {
        return Err(
            "global replay V2 has a witness not named by the canonical selections".to_owned(),
        );
    }
    schedule_streams(manifest, streams)
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "every argument is a separately checked Selection V4 or Execution V2 authority term"
)]
fn reconstruct_stream_v2(
    selection_index: usize,
    selection: &SelectionReceiptV4,
    rank: u16,
    entry: SelectedEntryV1,
    stream_ordinal: u16,
    witness: &SelectedReplayWitnessV2<'_>,
    execution: &mut ExecutionDispositionLedgerV2,
    manifest: &GlobalReplayManifestV2,
    selection_authority: &mut ReplaySelectionAuthorityV2,
) -> Result<VerifiedReplayStreamV2, GlobalReplayRefusalV2> {
    let live_joined = selection_authority.joined_row(entry.family(), entry.row_sequence())?;
    if live_joined != witness.joined_row {
        return Err(
            "global replay V2 witness differs from the live triple-authority row".to_owned(),
        );
    }
    let row = witness.joined_row.population;
    if witness.selection_id != selection.selection_id()
        || witness.rank != rank
        || row.population_id != entry.population_id()
        || row.sequence != entry.row_sequence()
        || row.strategy_digest != entry.strategy_digest()
        || row.instrument_family != entry.family()
        || row.rung_seconds != selection.rung_seconds()
    {
        return Err(format!(
            "Selection V4 {} rank {rank} witness differs from its exact selected row",
            hex(&selection.selection_id())
        ));
    }
    let reference = selection_reference(selection, entry.family())?;
    let population_v4_digest = witness.population_v4.content_digest()?;
    if witness.population_v4.population_id() != reference.population_id()
        || population_v4_digest != reference.population_v4_completion_digest()
    {
        return Err("global replay V2 witness carries a foreign Population V4 receipt".to_owned());
    }
    let authority = manifest.authority(selection_index, entry.family())?;
    if authority.population_id != row.population_id
        || authority.population_v4_completion_digest != population_v4_digest
        || authority.admission_completion_digest != reference.admission_completion_digest()
        || authority.execution_v2_completion_id != reference.execution_v2_completion_digest()
    {
        return Err("global replay V2 selected row differs from its manifest authority".to_owned());
    }
    let durable = execution
        .row(row.population_id, row.sequence)?
        .ok_or_else(|| "global replay V2 selected row has no durable disposition".to_owned())?;
    if durable.execution_authority_id() == [0; 32]
        || durable.population_id() != row.population_id
        || durable.population_v4_digest() != authority.population_v4_completion_digest
        || durable.admission_completion_digest() != authority.admission_completion_digest
        || durable.admission_status() != AdmissionStatusV1::Admitted
        || durable.tag() != ExecutionDispositionTagV2::Authorized
    {
        return Err(
            "global replay V2 selected row is not exact admitted+authorized Execution V2 evidence"
                .to_owned(),
        );
    }
    let decision_bytes = witness.admission_decision.to_bytes()?;
    if witness.admission_completion.population_id() != row.population_id
        || witness.admission_completion.digest()? != authority.admission_completion_digest
        || witness.admission_decision.population_id() != row.population_id
        || witness.admission_decision.row_sequence() != row.sequence
        || witness.admission_decision.strategy_digest() != row.strategy_digest
        || witness.admission_decision.row_payload_digest() != row.payload_digest()?
        || brutex_core::blake3::hash(&decision_bytes)
            != witness.joined_row.admission.decision_digest
        || witness.admission_decision.verdict().status() != witness.joined_row.admission.status
        || durable.disposition_id() != witness.joined_row.execution.disposition_digest
    {
        return Err(
            "global replay V2 admission/disposition witness differs from the joined authority row"
                .to_owned(),
        );
    }
    let parameters = execution
        .parameters(row.population_id, row.direction)?
        .ok_or_else(|| {
            "global replay V2 selected row has no dynamic execution parameters".to_owned()
        })?;
    let parameter_slot = match row.direction {
        TradeDirectionV1::Long => 0,
        TradeDirectionV1::Short => 1,
    };
    if parameters.parameter_id()
        != *authority
            .execution_parameter_ids
            .get(parameter_slot)
            .ok_or_else(|| "global replay V2 parameter slot is absent".to_owned())?
    {
        return Err(
            "global replay V2 selected row parameters differ from its completion".to_owned(),
        );
    }
    parameters.require_column(witness.training_column)?;
    let resolved = parameters.reconstruct_grid(witness.training_series)?;
    let evaluated = resolved
        .evaluate_training_grid_attested(
            witness.training_series,
            witness.training_column,
            parameters.horizon(),
            witness.training_run,
        )
        .map_err(|why| format!("global replay V2 training grid evaluation refused: {why:?}"))?;
    let validated = resolved
        .validate_evaluation(&evaluated)
        .map_err(|why| format!("global replay V2 training grid validation refused: {why:?}"))?;
    let classification = resolved
        .classify_coordinate(&validated, durable.coordinate())
        .map_err(|why| format!("global replay V2 coordinate classification refused: {why:?}"))?;
    let selected = classification
        .selected()
        .cloned()
        .ok_or_else(|| "Selection V4 retained a freshly policy-refused coordinate".to_owned())?;
    let capability = match durable.require_reclassification(
        &witness.population_v4,
        witness.admission_completion,
        row,
        witness.admission_decision,
        &parameters,
        &classification,
    )? {
        ReclassifiedExecutionDispositionV2::Authorized(capability) => capability,
        ReclassifiedExecutionDispositionV2::PolicyRefused(_) => {
            return Err("Selection V4 retained a durable policy-refused coordinate".to_owned());
        }
    };
    let universe = resolved
        .replay_selected_universe(
            witness.oos_series,
            witness.oos_column,
            &selected,
            witness.oos_run,
        )
        .map_err(|why| format!("global replay V2 OOS replay refused: {why:?}"))?;
    universe
        .require_integrity()
        .map_err(|why| format!("global replay V2 OOS universe failed integrity: {why:?}"))?;
    stream_from_universe(
        stream_ordinal,
        rank,
        selection,
        &row,
        authority,
        durable.execution_authority_id(),
        capability.capability_id(),
        witness.training_run.run_id().bytes(),
        selected.digest(),
        witness.oos_run.run_id().bytes(),
        &universe,
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "one stream binds each exact reconstructed authority without a loose options bag"
)]
fn stream_from_universe(
    stream_ordinal: u16,
    rank: u16,
    selection: &SelectionReceiptV4,
    row: &PopulationRowV1,
    authority: GlobalReplayAuthoritySlotV2,
    execution_disposition_id: [u8; 32],
    execution_capability_id: [u8; 32],
    training_run_id: [u8; 32],
    selected_exit_digest: [u8; 32],
    oos_run_id: [u8; 32],
    universe: &ReplayedCandidateUniverseV1,
) -> Result<VerifiedReplayStreamV2, GlobalReplayRefusalV2> {
    let universe_digest = universe.digest();
    let mut candidates = Vec::new();
    candidates
        .try_reserve_exact(universe.candidates().len())
        .map_err(|why| format!("V2 candidate schedule allocation refused: {why}"))?;
    for (ordinal, candidate) in universe.candidates().iter().copied().enumerate() {
        candidates.push(CandidateScheduleEvidenceV2::from_candidate(
            universe_digest,
            ordinal,
            candidate,
        )?);
    }
    let mut stream = VerifiedReplayStreamV2 {
        stream_id: [0; 32],
        stream_ordinal,
        rank,
        rung_seconds: selection.rung_seconds(),
        family: row.instrument_family,
        direction: row.direction,
        selection_id: selection.selection_id(),
        population_id: row.population_id,
        row_sequence: row.sequence,
        strategy_digest: row.strategy_digest,
        execution_completion_id: authority.execution_v2_completion_id,
        execution_disposition_id,
        execution_capability_id,
        training_run_id,
        selected_exit_digest,
        oos_run_id,
        universe_digest,
        pricing_refused_paths: universe.pricing_refused_paths(),
        candidates,
        cursor: 0,
    };
    stream.stream_id = stream.derived_id()?;
    stream.validate()?;
    Ok(stream)
}

fn schedule_streams(
    manifest: GlobalReplayManifestV2,
    mut streams: Vec<VerifiedReplayStreamV2>,
) -> Result<PreparedGlobalReplayV2, GlobalReplayRefusalV2> {
    let total_candidates = streams.iter().try_fold(0_usize, |sum, stream| {
        sum.checked_add(stream.candidates.len())
            .ok_or_else(|| "global replay V2 candidate count overflowed".to_owned())
    })?;
    let mut decisions = Vec::new();
    decisions
        .try_reserve_exact(total_candidates)
        .map_err(|why| format!("global replay V2 decision allocation refused: {why}"))?;
    let mut scheduler = GlobalSinglePositionV1::new();
    let mut admitted_pricing_refused = 0_u64;
    while let Some(entry_micros) = next_entry_micros(&streams) {
        let (intents, offered_streams) = offered_for_minute(&streams, entry_micros)?;
        let schedule = scheduler
            .schedule_minute(entry_micros, &intents)
            .map_err(|why| format!("global replay V2 minute {entry_micros} refused: {why:?}"))?;
        for decision in schedule.decisions() {
            let stream_index =
                find_offered_stream(&streams, &offered_streams, decision.constituent)?;
            let stream = streams
                .get_mut(stream_index)
                .ok_or_else(|| "global replay V2 scheduled stream disappeared".to_owned())?;
            let candidate = stream
                .candidates
                .get(stream.cursor)
                .copied()
                .ok_or_else(|| "global replay V2 scheduled candidate disappeared".to_owned())?;
            if matches!(decision.disposition, Disposition::Admitted { .. })
                && candidate.path.pricing_refused()
            {
                admitted_pricing_refused = admitted_pricing_refused
                    .checked_add(1)
                    .ok_or_else(|| "V2 admitted pricing-refused count overflowed".to_owned())?;
            }
            let sequence = usize_u64(decisions.len(), "global replay V2 decision sequence")?;
            decisions.push(GlobalReplayDecisionV2::new(
                sequence,
                stream,
                stream.cursor,
                candidate,
                decision.disposition,
            )?);
            stream.cursor = stream
                .cursor
                .checked_add(1)
                .ok_or_else(|| "global replay V2 candidate cursor overflowed".to_owned())?;
        }
    }
    if streams
        .iter()
        .any(|stream| stream.cursor != stream.candidates.len())
    {
        return Err("global replay V2 scheduler ended before every candidate".to_owned());
    }
    let counters = scheduler.counters();
    if !counters.reconciles()
        || counters.offered != usize_u64(decisions.len(), "global replay V2 decisions")?
        || counters.unreachable != 0
        || counters.refused != 0
    {
        return Err("global replay V2 scheduler counters do not reconcile".to_owned());
    }
    let pricing_refused_candidates = streams.iter().try_fold(0_u64, |sum, stream| {
        sum.checked_add(stream.pricing_refused_paths)
            .ok_or_else(|| "global replay V2 pricing-refused total overflowed".to_owned())
    })?;
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
    let prepared = PreparedGlobalReplayV2 {
        manifest,
        streams,
        decisions,
        counters,
        pricing_refused_candidates,
        admitted_pricing_refused,
        replay_id,
        ordered_stream_digest,
        ordered_decision_digest,
    };
    prepared.validate()?;
    Ok(prepared)
}

fn next_entry_micros(streams: &[VerifiedReplayStreamV2]) -> Option<i64> {
    streams
        .iter()
        .filter_map(|stream| {
            stream
                .candidates
                .get(stream.cursor)
                .map(|candidate| candidate.entry_micros)
        })
        .min()
}

fn offered_for_minute(
    streams: &[VerifiedReplayStreamV2],
    entry_micros: i64,
) -> Result<(Vec<Intent>, Vec<usize>), GlobalReplayRefusalV2> {
    let mut intents = Vec::new();
    let mut indexes = Vec::new();
    intents
        .try_reserve_exact(MAX_STREAMS)
        .map_err(|why| format!("global replay V2 minute intent allocation refused: {why}"))?;
    indexes
        .try_reserve_exact(MAX_STREAMS)
        .map_err(|why| format!("global replay V2 stream-index allocation refused: {why}"))?;
    for (index, stream) in streams.iter().enumerate() {
        let Some(candidate) = stream.candidates.get(stream.cursor) else {
            continue;
        };
        if candidate.entry_micros == entry_micros {
            intents.push(Intent {
                constituent: constituent_of(stream)?,
                evidence: Evidence::Reachable {
                    occupied_through_micros: candidate.occupied_through_micros,
                },
            });
            indexes.push(index);
        }
    }
    Ok((intents, indexes))
}

fn find_offered_stream(
    streams: &[VerifiedReplayStreamV2],
    indexes: &[usize],
    constituent: Constituent,
) -> Result<usize, GlobalReplayRefusalV2> {
    let mut found = None;
    for index in indexes.iter().copied() {
        let stream = streams
            .get(index)
            .ok_or_else(|| "global replay V2 offered stream index is invalid".to_owned())?;
        if constituent_of(stream)? == constituent {
            if found.is_some() {
                return Err("global replay V2 scheduler constituent aliases streams".to_owned());
            }
            found = Some(index);
        }
    }
    found.ok_or_else(|| "global replay V2 scheduler returned an unoffered constituent".to_owned())
}

fn constituent_of(stream: &VerifiedReplayStreamV2) -> Result<Constituent, GlobalReplayRefusalV2> {
    Ok(Constituent {
        priority: stream.rank,
        strategy_digest: StrategyDigest::new(stream.strategy_digest),
        instrument: instrument_of(stream.family)?,
        direction: direction_of(stream.direction),
        rung_minutes: u16::try_from(stream.rung_seconds / 60)
            .map_err(|_| "global replay V2 rung minutes do not fit u16".to_owned())?,
    })
}

fn selection_reference(
    selection: &SelectionReceiptV4,
    family: InstrumentFamilyV1,
) -> Result<&PopulationReferenceV4, GlobalReplayRefusalV2> {
    let index = match family {
        InstrumentFamilyV1::Nifty => 0,
        InstrumentFamilyV1::BankNifty => 1,
    };
    selection
        .populations()
        .get(index)
        .filter(|reference| reference.family() == family)
        .ok_or_else(|| "Selection V4 population reference order differs".to_owned())
}

fn require_execution_completion(
    reference: &PopulationReferenceV4,
    completion: &crate::execution_disposition_v2::ExecutionCapabilityCompletionV2,
) -> Result<(), GlobalReplayRefusalV2> {
    if completion.population_id() != reference.population_id()
        || completion.population_v4_digest() != reference.population_v4_completion_digest()
        || completion.admission_completion_digest() != reference.admission_completion_digest()
        || completion.completion_id() != reference.execution_v2_completion_digest()
        || completion.row_count() != reference.row_count()
        || completion.execution_law_digest() != exact_execution_law_digest_v1()
    {
        return Err(format!(
            "Execution V2 completion {} does not cover exact Selection V4 population {}",
            hex(&completion.completion_id()),
            hex(&reference.population_id())
        ));
    }
    let matrix = completion.admission_execution_matrix();
    for admission in [
        AdmissionStatusV1::Admitted,
        AdmissionStatusV1::Rejected,
        AdmissionStatusV1::Unmeasured,
        AdmissionStatusV1::Refused,
    ] {
        for execution in [
            ExecutionDispositionTagV2::Authorized,
            ExecutionDispositionTagV2::PolicyRefused,
        ] {
            let selection_status = match execution {
                ExecutionDispositionTagV2::Authorized => {
                    crate::selection_v4::ExecutionSelectionStatusV2::Authorized
                }
                ExecutionDispositionTagV2::PolicyRefused => {
                    crate::selection_v4::ExecutionSelectionStatusV2::PolicyRefused
                }
            };
            if matrix.count(admission, execution)
                != reference.accounting().count(admission, selection_status)
            {
                return Err(
                    "Execution V2 completion matrix differs from Selection V4 accounting"
                        .to_owned(),
                );
            }
        }
    }
    Ok(())
}

fn common_cohort_bytes(selection: &SelectionReceiptV4) -> [u8; COMMON_COHORT_BYTES] {
    let mut bytes = selection.cohort_identity().canonical_bytes();
    if let Some(slot) = bytes.get_mut(SIGNAL_COVERAGE_START..SIGNAL_COVERAGE_END) {
        slot.fill(0);
    }
    bytes
}

fn digest_common_cohort(bytes: &[u8; COMMON_COHORT_BYTES]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(COMMON_COHORT_DOMAIN_V2);
    hasher.update(bytes);
    hasher.finalize()
}

fn stream_order_digest(
    streams: &[VerifiedReplayStreamV2],
) -> Result<[u8; 32], GlobalReplayRefusalV2> {
    let mut hasher = Hasher::new();
    hasher.update(STREAM_ORDER_DOMAIN_V2);
    hasher.update(&usize_u64(streams.len(), "V2 stream order count")?.to_le_bytes());
    for stream in streams {
        hasher.update(&stream.stream_id);
    }
    Ok(hasher.finalize())
}

fn decision_order_digest(
    decisions: &[GlobalReplayDecisionV2],
) -> Result<[u8; 32], GlobalReplayRefusalV2> {
    let mut hasher = Hasher::new();
    hasher.update(DECISION_ORDER_DOMAIN_V2);
    hasher.update(&usize_u64(decisions.len(), "V2 decision order count")?.to_le_bytes());
    for decision in decisions {
        hasher.update(&decision.decision_digest);
    }
    Ok(hasher.finalize())
}

fn derive_replay_id(
    manifest: &GlobalReplayManifestV2,
    ordered_stream_digest: [u8; 32],
    ordered_decision_digest: [u8; 32],
    counters: Counters,
    pricing_refused_candidates: u64,
    admitted_pricing_refused: u64,
) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(REPLAY_ID_DOMAIN_V2);
    hasher.update(&manifest.manifest_id);
    hasher.update(&ordered_stream_digest);
    hasher.update(&ordered_decision_digest);
    for count in [
        counters.offered,
        counters.admitted,
        counters.blocked_occupied,
        counters.blocked_simultaneous,
        counters.unreachable,
        counters.refused,
        pricing_refused_candidates,
        admitted_pricing_refused,
    ] {
        hasher.update(&count.to_le_bytes());
    }
    hasher.update(&exact_execution_law_digest_v1());
    hasher.finalize()
}

const fn empty_authority_slot() -> GlobalReplayAuthoritySlotV2 {
    GlobalReplayAuthoritySlotV2 {
        population_id: [0; 32],
        population_v4_completion_digest: [0; 32],
        admission_completion_digest: [0; 32],
        execution_v2_completion_id: [0; 32],
        execution_parameter_ids: [[0; 32]; 2],
        row_count: 0,
    }
}

const fn family_byte(family: InstrumentFamilyV1) -> u8 {
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

const fn direction_of(direction: TradeDirectionV1) -> Direction {
    match direction {
        TradeDirectionV1::Long => Direction::Long,
        TradeDirectionV1::Short => Direction::Short,
    }
}

fn instrument_of(family: InstrumentFamilyV1) -> Result<InstrumentKey, GlobalReplayRefusalV2> {
    InstrumentKey::index(
        Exchange::Nse,
        match family {
            InstrumentFamilyV1::Nifty => "NIFTY",
            InstrumentFamilyV1::BankNifty => "BANKNIFTY",
        },
    )
    .map_err(|why| format!("canonical swept instrument could not be built: {why}"))
}

fn validate_rung(rung_seconds: u32) -> Result<(), GlobalReplayRefusalV2> {
    if GLOBAL_REPLAY_V2_RUNGS_SECONDS.contains(&rung_seconds) {
        Ok(())
    } else {
        Err(format!(
            "global replay V2 rung {rung_seconds}s is outside the eight canonical intraday rungs"
        ))
    }
}

fn require_digest(name: &str, digest: &[u8; 32]) -> Result<(), GlobalReplayRefusalV2> {
    if digest == &[0; 32] {
        Err(format!("{name} digest is all zero"))
    } else {
        Ok(())
    }
}

fn usize_u64(value: usize, subject: &str) -> Result<u64, GlobalReplayRefusalV2> {
    u64::try_from(value).map_err(|_| format!("{subject} does not fit u64"))
}

fn u64_usize_v2(value: u64, subject: &str) -> Result<usize, GlobalReplayRefusalV2> {
    usize::try_from(value).map_err(|_| format!("{subject} does not fit usize"))
}

fn hex(digest: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in digest {
        use core::fmt::Write as _;
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

    fn bytes(&mut self, value: &[u8]) -> Result<(), GlobalReplayRefusalV2> {
        let end = self
            .cursor
            .checked_add(value.len())
            .ok_or_else(|| "global replay V2 encoder cursor overflowed".to_owned())?;
        self.bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "global replay V2 encoder exceeded fixed payload".to_owned())?
            .copy_from_slice(value);
        self.cursor = end;
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), GlobalReplayRefusalV2> {
        self.bytes(&[value])
    }

    fn u16(&mut self, value: u16) -> Result<(), GlobalReplayRefusalV2> {
        self.bytes(&value.to_le_bytes())
    }

    fn u32(&mut self, value: u32) -> Result<(), GlobalReplayRefusalV2> {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), GlobalReplayRefusalV2> {
        self.bytes(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), GlobalReplayRefusalV2> {
        self.bytes(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), GlobalReplayRefusalV2> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "global replay V2 encoder zero cursor overflowed".to_owned())?;
        self.bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "global replay V2 encoder zeros exceeded fixed payload".to_owned())?
            .fill(0);
        self.cursor = end;
        Ok(())
    }

    fn finish(self) -> Result<(), GlobalReplayRefusalV2> {
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err(format!(
                "global replay V2 encoder wrote {} of {} bytes",
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

    fn take(&mut self, count: usize) -> Result<&'a [u8], GlobalReplayRefusalV2> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "global replay V2 decoder cursor overflowed".to_owned())?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| "global replay V2 decoder exceeded fixed payload".to_owned())?;
        self.cursor = end;
        Ok(value)
    }

    fn array_8(&mut self) -> Result<[u8; 8], GlobalReplayRefusalV2> {
        self.take(8)?
            .try_into()
            .map_err(|_| "global replay V2 eight-byte field differs".to_owned())
    }

    fn array_32(&mut self) -> Result<[u8; 32], GlobalReplayRefusalV2> {
        self.take(32)?
            .try_into()
            .map_err(|_| "global replay V2 digest field differs".to_owned())
    }

    fn u8(&mut self) -> Result<u8, GlobalReplayRefusalV2> {
        self.take(1)?
            .first()
            .copied()
            .ok_or_else(|| "global replay V2 u8 field differs".to_owned())
    }

    fn u16(&mut self) -> Result<u16, GlobalReplayRefusalV2> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().map_err(
            |_| "global replay V2 u16 field differs".to_owned(),
        )?))
    }

    fn u32(&mut self) -> Result<u32, GlobalReplayRefusalV2> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().map_err(
            |_| "global replay V2 u32 field differs".to_owned(),
        )?))
    }

    fn u64(&mut self) -> Result<u64, GlobalReplayRefusalV2> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().map_err(
            |_| "global replay V2 u64 field differs".to_owned(),
        )?))
    }

    fn i64(&mut self) -> Result<i64, GlobalReplayRefusalV2> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().map_err(
            |_| "global replay V2 i64 field differs".to_owned(),
        )?))
    }

    fn zeros(&mut self, count: usize) -> Result<(), GlobalReplayRefusalV2> {
        if self.take(count)?.iter().any(|byte| *byte != 0) {
            Err("global replay V2 reserved bytes are nonzero".to_owned())
        } else {
            Ok(())
        }
    }

    fn finish(self) -> Result<(), GlobalReplayRefusalV2> {
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err("global replay V2 decoder left trailing bytes".to_owned())
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::too_many_lines,
    reason = "adversarial fixtures need terse setup failures and keep each end-to-end scenario visible"
)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use runner::topn::Weights;

    use super::*;
    use crate::execution_disposition_v2::v2_tests::{
        CanonicalExecutionDispositionV2Fixture, canonical_execution_disposition_v2_fixture,
    };

    static ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn digest(seed: u8) -> [u8; 32] {
        [seed.max(1); 32]
    }

    fn test_root(label: &str) -> PathBuf {
        let sequence = ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "brutex-global-replay-v2-{label}-{}-{sequence}",
            std::process::id()
        ))
    }

    fn build_population_authorities(
        root: &Path,
        execution: &mut ExecutionDispositionLedgerV2,
        ranking: RankingPolicyV1,
    ) -> Vec<CanonicalExecutionDispositionV2Fixture> {
        let mut fixtures = Vec::with_capacity(16);
        for (rung_index, rung_seconds) in GLOBAL_REPLAY_V2_RUNGS_SECONDS.into_iter().enumerate() {
            for (family_index, family) in [InstrumentFamilyV1::Nifty, InstrumentFamilyV1::BankNifty]
                .into_iter()
                .enumerate()
            {
                let ordinal = rung_index
                    .checked_mul(2)
                    .and_then(|offset| offset.checked_add(family_index))
                    .expect("fixture authority ordinal");
                let fixture = canonical_execution_disposition_v2_fixture(
                    root,
                    u8::try_from(ordinal + 1).expect("fixture population seed"),
                    family,
                    rung_seconds,
                    25,
                    ranking,
                );
                execution
                    .append_complete(&fixture.prepared)
                    .expect("append complete Execution V2 authority");
                fixtures.push(fixture);
            }
        }
        fixtures
    }

    fn open_blocked_selection_authorities(
        root: &Path,
        fixtures: &[CanonicalExecutionDispositionV2Fixture],
        ranking: RankingPolicyV1,
    ) -> Vec<ReplaySelectionAuthorityV2> {
        let mut authorities = Vec::with_capacity(8);
        for rung_index in 0..GLOBAL_REPLAY_V2_RUNGS_SECONDS.len() {
            let nifty = fixtures
                .get(rung_index * 2)
                .expect("NIFTY population fixture");
            let bank_nifty = fixtures
                .get(rung_index * 2 + 1)
                .expect("BANKNIFTY population fixture");
            let (mut nifty_authority, mut bank_authority) =
                SelectionAuthorityLedgerV4::open_pair_read(
                    root,
                    nifty.receipt.population_id(),
                    bank_nifty.receipt.population_id(),
                )
                .expect("open exact Selection V4 authority pair");
            let receipt = SelectionReceiptV4::from_authorities(
                &mut nifty_authority,
                &mut bank_authority,
                ranking,
            )
            .expect("build Selection V4 from all three authorities");
            assert_eq!(receipt.population_proof().admitted, 0);
            assert!(receipt.top_twenty_five().is_empty());
            authorities.push(
                ReplaySelectionAuthorityV2::new(receipt, nifty_authority, bank_authority, ranking)
                    .expect("seal replay selection authority"),
            );
        }
        authorities
    }

    fn manifest_fixture() -> GlobalReplayManifestV2 {
        let mut selection_ids = [[0_u8; 32]; 8];
        for (index, selection) in selection_ids.iter_mut().enumerate() {
            *selection = digest(u8::try_from(index + 1).expect("selection seed"));
        }
        let mut authorities = [empty_authority_slot(); 16];
        for (index, slot) in authorities.iter_mut().enumerate() {
            let seed = u8::try_from(index).expect("authority seed");
            *slot = GlobalReplayAuthoritySlotV2 {
                population_id: digest(seed.wrapping_add(21)),
                population_v4_completion_digest: digest(seed.wrapping_add(41)),
                admission_completion_digest: digest(seed.wrapping_add(61)),
                execution_v2_completion_id: digest(seed.wrapping_add(81)),
                execution_parameter_ids: [
                    digest(seed.wrapping_add(101)),
                    digest(seed.wrapping_add(121)),
                ],
                row_count: 25,
            };
        }
        let mut manifest = GlobalReplayManifestV2 {
            manifest_id: [0; 32],
            common_cohort_digest: digest(201),
            selection_ids,
            authorities,
        };
        manifest.manifest_id = manifest.derived_id();
        manifest.validate().expect("valid fixture manifest");
        manifest
    }

    fn candidate(
        universe: [u8; 32],
        ordinal: usize,
        signal: i64,
        entry: i64,
        through: i64,
        path: ReplayCandidatePathV2,
    ) -> CandidateScheduleEvidenceV2 {
        let mut value = CandidateScheduleEvidenceV2 {
            candidate_digest: [0; 32],
            signal_micros: signal,
            entry_micros: entry,
            occupied_through_micros: through,
            path,
        };
        value.candidate_digest = value
            .derived_digest(universe, ordinal)
            .expect("candidate identity");
        value
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "test fixture names every independently validated stream authority"
    )]
    fn stream(
        ordinal: u16,
        rank: u16,
        rung_seconds: u32,
        family: InstrumentFamilyV1,
        direction: TradeDirectionV1,
        strategy_seed: u8,
        candidates: Vec<CandidateScheduleEvidenceV2>,
    ) -> VerifiedReplayStreamV2 {
        let universe_digest = digest(strategy_seed.wrapping_add(1));
        let pricing_refused_paths = candidates
            .iter()
            .filter(|candidate| candidate.path.pricing_refused())
            .count()
            .try_into()
            .expect("pricing-refused count fits u64");
        let mut value = VerifiedReplayStreamV2 {
            stream_id: [0; 32],
            stream_ordinal: ordinal,
            rank,
            rung_seconds,
            family,
            direction,
            selection_id: digest(strategy_seed.wrapping_add(2)),
            population_id: digest(strategy_seed.wrapping_add(3)),
            row_sequence: u64::from(ordinal),
            strategy_digest: digest(strategy_seed),
            execution_completion_id: digest(strategy_seed.wrapping_add(4)),
            execution_disposition_id: digest(strategy_seed.wrapping_add(5)),
            execution_capability_id: digest(strategy_seed.wrapping_add(6)),
            training_run_id: digest(strategy_seed.wrapping_add(7)),
            selected_exit_digest: digest(strategy_seed.wrapping_add(8)),
            oos_run_id: digest(strategy_seed.wrapping_add(9)),
            universe_digest,
            pricing_refused_paths,
            candidates,
            cursor: 0,
        };
        value.stream_id = value.derived_id().expect("stream identity");
        value.validate().expect("valid fixture stream");
        value
    }

    fn controlled_manifest_stream(
        manifest: &GlobalReplayManifestV2,
        ordinal: usize,
    ) -> VerifiedReplayStreamV2 {
        let selection_index = ordinal / 25;
        let rank = u16::try_from((ordinal % 25) + 1).expect("one-based controlled rank");
        let family = if ordinal.is_multiple_of(2) {
            InstrumentFamilyV1::Nifty
        } else {
            InstrumentFamilyV1::BankNifty
        };
        let authority = manifest
            .authority(selection_index, family)
            .expect("controlled manifest authority slot");
        let rung_seconds = GLOBAL_REPLAY_V2_RUNGS_SECONDS
            .get(selection_index)
            .copied()
            .expect("controlled rung slot");
        let selection_id = manifest
            .selection_ids
            .get(selection_index)
            .copied()
            .expect("controlled selection slot");
        let seed = u8::try_from(ordinal + 1).expect("controlled stream seed");
        let universe_digest = digest(seed.wrapping_add(1));
        let entry_micros = i64::try_from(ordinal)
            .expect("controlled ordinal fits i64")
            .checked_mul(120)
            .and_then(|value| value.checked_add(60))
            .expect("controlled entry timestamp");
        let candidates = vec![candidate(
            universe_digest,
            0,
            entry_micros - 60,
            entry_micros,
            entry_micros,
            ReplayCandidatePathV2::Priceable,
        )];
        let mut value = VerifiedReplayStreamV2 {
            stream_id: [0; 32],
            stream_ordinal: u16::try_from(ordinal).expect("controlled ordinal fits u16"),
            rank,
            rung_seconds,
            family,
            direction: if ordinal.is_multiple_of(2) {
                TradeDirectionV1::Long
            } else {
                TradeDirectionV1::Short
            },
            selection_id,
            population_id: authority.population_id,
            row_sequence: u64::from((rank - 1) / 2),
            strategy_digest: digest(seed),
            execution_completion_id: authority.execution_v2_completion_id,
            execution_disposition_id: digest(seed.wrapping_add(2)),
            execution_capability_id: digest(seed.wrapping_add(3)),
            training_run_id: digest(seed.wrapping_add(4)),
            selected_exit_digest: digest(seed.wrapping_add(5)),
            oos_run_id: digest(seed.wrapping_add(6)),
            universe_digest,
            pricing_refused_paths: 0,
            candidates,
            cursor: 0,
        };
        value.stream_id = value.derived_id().expect("controlled stream identity");
        value.validate().expect("valid controlled stream");
        value
    }

    #[test]
    fn public_v1_authorities_block_global_top25_until_a_genuine_pbo_successor() {
        let root = test_root("public-v1-pbo-block");
        fs::create_dir(&root).expect("create explicit external authority root");
        let ranking = RankingPolicyV1::new(Weights::equal()).expect("equal ranking policy");
        let mut execution = ExecutionDispositionLedgerV2::open(&root)
            .expect("open writable Execution V2 authority");
        let fixtures = build_population_authorities(&root, &mut execution, ranking);
        assert_eq!(fixtures.len(), 16);
        assert!(fixtures.iter().all(|fixture| {
            fixture.admission_completion.admitted_count() == 0
                && fixture.admission_completion.rejected_count() == 0
                && fixture
                    .admission_completion
                    .unmeasured_count()
                    .checked_add(fixture.admission_completion.refused_count())
                    == Some(fixture.admission_completion.decision_count())
        }));

        let mut authorities = open_blocked_selection_authorities(&root, &fixtures, ranking);
        assert_eq!(authorities.len(), 8);
        assert!(authorities.iter().all(|authority| {
            authority.receipt().population_proof().admitted == 0
                && authority.receipt().top_twenty_five().is_empty()
        }));

        let blocked = GlobalReplayManifestV2::from_selections(&mut authorities, &mut execution)
            .expect_err("public V1 cannot fabricate a complete Top-25");
        assert!(blocked.contains("requires the complete Top-25"));
        assert!(blocked.contains("found 0"));

        drop(authorities);
        drop(execution);
        drop(fixtures);
        fs::remove_dir_all(root).expect("remove truthful public V1 fixture root");
    }

    #[test]
    fn private_controlled_scheduler_preserves_eight_by_twenty_five_algebra_in_memory() {
        let manifest = manifest_fixture();
        let streams = (0..MAX_STREAMS)
            .map(|ordinal| controlled_manifest_stream(&manifest, ordinal))
            .collect::<Vec<_>>();
        for (selection_index, selection) in streams.chunks_exact(25).enumerate() {
            let expected_rung = GLOBAL_REPLAY_V2_RUNGS_SECONDS
                .get(selection_index)
                .copied()
                .expect("controlled rung slot");
            let expected_selection = manifest
                .selection_ids
                .get(selection_index)
                .copied()
                .expect("controlled selection slot");
            assert_eq!(
                selection
                    .iter()
                    .map(VerifiedReplayStreamV2::rank)
                    .collect::<Vec<_>>(),
                (1_u16..=25).collect::<Vec<_>>()
            );
            assert!(selection.iter().all(|stream| {
                stream.rung_seconds == expected_rung && stream.selection_id == expected_selection
            }));
        }
        let prepared = schedule_streams(manifest, streams)
            .expect("private controlled 8x25 scheduler authority");
        assert_eq!(prepared.streams().len(), MAX_STREAMS);
        assert_eq!(prepared.decisions().len(), MAX_STREAMS);
        assert_eq!(prepared.counters().offered, 200);
        assert_eq!(prepared.counters().admitted, 200);
        assert!(prepared.counters().reconciles());
    }

    #[test]
    fn manifest_codec_is_new_sealed_and_round_trips_every_authority() {
        let manifest = manifest_fixture();
        let encoded = manifest.canonical_bytes().expect("manifest bytes");
        assert_eq!(encoded.get(..8).expect("magic bytes"), &MANIFEST_MAGIC_V2);
        assert_eq!(
            u32::from_le_bytes(
                encoded
                    .get(8..12)
                    .expect("version slice")
                    .try_into()
                    .expect("version bytes"),
            ),
            MANIFEST_VERSION_V2
        );
        assert_eq!(
            GlobalReplayManifestV2::from_canonical_bytes(&encoded).expect("manifest decode"),
            manifest
        );
        assert!(
            GlobalReplayManifestV2::path(Path::new("/tmp/brutex-v2"))
                .ends_with("results/global-replay-authority-manifests-v2.bin")
        );

        let mut torn = encoded;
        *torn.get_mut(100).expect("torn byte") ^= 1;
        assert!(
            GlobalReplayManifestV2::from_canonical_bytes(&torn)
                .expect_err("torn manifest must refuse")
                .contains("seal")
        );
    }

    #[test]
    fn manifest_identity_refuses_copied_selection_population_and_execution_authority() {
        let base = manifest_fixture();

        let mut copied_selection = base.clone();
        let first_selection = *copied_selection
            .selection_ids
            .first()
            .expect("first selection");
        *copied_selection
            .selection_ids
            .get_mut(1)
            .expect("second selection") = first_selection;
        copied_selection.manifest_id = copied_selection.derived_id();
        assert!(copied_selection.validate().is_err());

        let mut copied_population = base.clone();
        let first_population = copied_population
            .authorities
            .first()
            .expect("first authority")
            .population_id;
        copied_population
            .authorities
            .get_mut(1)
            .expect("second authority")
            .population_id = first_population;
        copied_population.manifest_id = copied_population.derived_id();
        assert!(copied_population.validate().is_err());

        let mut copied_execution = base;
        let first_execution = copied_execution
            .authorities
            .first()
            .expect("first authority")
            .execution_v2_completion_id;
        copied_execution
            .authorities
            .get_mut(1)
            .expect("second authority")
            .execution_v2_completion_id = first_execution;
        copied_execution.manifest_id = copied_execution.derived_id();
        assert!(copied_execution.validate().is_err());
    }

    #[test]
    fn candidate_identity_binds_universe_ordinal_time_and_refusal_path() {
        let universe = digest(9);
        let value = candidate(universe, 0, 1, 2, 3, ReplayCandidatePathV2::BlockOnly);
        value.validate(universe, 0).expect("exact candidate");
        assert!(value.validate(digest(10), 0).is_err());
        assert!(value.validate(universe, 1).is_err());
        let mut changed = value;
        changed.path = ReplayCandidatePathV2::Priceable;
        assert!(changed.validate(universe, 0).is_err());
    }

    #[test]
    fn global_scheduler_blocks_direction_instrument_and_timeframe_until_exit() {
        let universe_a = digest(32);
        let universe_b = digest(42);
        let first = stream(
            0,
            1,
            60,
            InstrumentFamilyV1::Nifty,
            TradeDirectionV1::Long,
            31,
            vec![candidate(
                universe_a,
                0,
                0,
                60,
                180,
                ReplayCandidatePathV2::Priceable,
            )],
        );
        let second = stream(
            1,
            2,
            3_600,
            InstrumentFamilyV1::BankNifty,
            TradeDirectionV1::Short,
            41,
            vec![
                candidate(
                    universe_b,
                    0,
                    60,
                    120,
                    120,
                    ReplayCandidatePathV2::Priceable,
                ),
                candidate(
                    universe_b,
                    1,
                    180,
                    240,
                    240,
                    ReplayCandidatePathV2::Priceable,
                ),
            ],
        );
        let manifest = manifest_fixture();
        let prepared = schedule_streams(manifest, vec![first, second])
            .expect_err("fixture has two streams rather than complete 200 and must fail final authority validation");
        assert!(prepared.contains("not 200"));

        // Test the exact delegated scheduler independently of final 200-stream
        // cardinality: NIFTY long owns through 180, so BANKNIFTY short at 120
        // is blocked; equality would also be blocked and only 240 may enter.
        let mut scheduler = GlobalSinglePositionV1::new();
        let first_constituent = constituent_of(&stream(
            0,
            1,
            60,
            InstrumentFamilyV1::Nifty,
            TradeDirectionV1::Long,
            51,
            Vec::new(),
        ))
        .expect("first constituent");
        let second_constituent = constituent_of(&stream(
            1,
            2,
            3_600,
            InstrumentFamilyV1::BankNifty,
            TradeDirectionV1::Short,
            61,
            Vec::new(),
        ))
        .expect("second constituent");
        let first_minute = scheduler
            .schedule_minute(
                60,
                &[Intent {
                    constituent: first_constituent,
                    evidence: Evidence::Reachable {
                        occupied_through_micros: 180,
                    },
                }],
            )
            .expect("first admission");
        assert!(matches!(
            first_minute
                .decisions()
                .next()
                .map(|decision| decision.disposition),
            Some(Disposition::Admitted { .. })
        ));
        let blocked = scheduler
            .schedule_minute(
                120,
                &[Intent {
                    constituent: second_constituent,
                    evidence: Evidence::Reachable {
                        occupied_through_micros: 120,
                    },
                }],
            )
            .expect("blocked minute");
        assert!(matches!(
            blocked
                .decisions()
                .next()
                .map(|decision| decision.disposition),
            Some(Disposition::BlockedOccupied {
                occupied_through_micros: 180
            })
        ));
        let admitted = scheduler
            .schedule_minute(
                240,
                &[Intent {
                    constituent: second_constituent,
                    evidence: Evidence::Reachable {
                        occupied_through_micros: 240,
                    },
                }],
            )
            .expect("post-exit minute");
        assert!(matches!(
            admitted
                .decisions()
                .next()
                .map(|decision| decision.disposition),
            Some(Disposition::Admitted { .. })
        ));
    }

    #[test]
    fn same_minute_order_is_priority_then_digest_not_caller_order() {
        let low = Constituent {
            priority: 1,
            strategy_digest: StrategyDigest::new(digest(200)),
            instrument: instrument_of(InstrumentFamilyV1::Nifty).expect("NIFTY"),
            direction: Direction::Long,
            rung_minutes: 1,
        };
        let high = Constituent {
            priority: 2,
            strategy_digest: StrategyDigest::new(digest(1)),
            instrument: instrument_of(InstrumentFamilyV1::BankNifty).expect("BANKNIFTY"),
            direction: Direction::Short,
            rung_minutes: 60,
        };
        let intents = [
            Intent {
                constituent: high,
                evidence: Evidence::Reachable {
                    occupied_through_micros: 300,
                },
            },
            Intent {
                constituent: low,
                evidence: Evidence::Reachable {
                    occupied_through_micros: 240,
                },
            },
        ];
        let mut scheduler = GlobalSinglePositionV1::new();
        let result = scheduler
            .schedule_minute(120, &intents)
            .expect("same-minute schedule");
        let decisions: Vec<_> = result.decisions().copied().collect();
        let first_decision = decisions.first().expect("first decision");
        let second_decision = decisions.get(1).expect("second decision");
        assert_eq!(first_decision.constituent, low);
        assert!(matches!(
            first_decision.disposition,
            Disposition::Admitted { .. }
        ));
        assert_eq!(second_decision.constituent, high);
        assert!(matches!(
            second_decision.disposition,
            Disposition::BlockedSimultaneous { admitted } if admitted == low.strategy_digest
        ));
    }
}
