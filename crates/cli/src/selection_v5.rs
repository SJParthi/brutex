//! Receipt-last Selection V5 persistence for one combined signal rung.
//!
//! A V5 selection is one deterministic Top-25 over the complete combined
//! `NSE-NIFTY` plus `NSE-BANKNIFTY` Execution V3 population for one canonical
//! signal rung.  Eligibility is the exact conjunction `admitted &&
//! execution-authorized`.  Both families must nevertheless be complete and
//! every disposition participates in the population proof before any row can
//! be discarded.  Top-10 is always a slice of the persisted Top-25; it is
//! never calculated independently.
//!
//! The sole production constructor consumes and retains the concrete
//! `CommittedStoredExecutionV3` capability.  It reauthenticates the literal
//! Population V5 rows and durable Execution V3 dispositions, exact-joins them
//! in canonical order, derives ranking facts from the authenticated Candidate
//! bytes, and freshly reopens the Selection files.  A detached completion
//! digest, caller row vector, structural receipt, mask, metric, or disposition
//! is never accepted as authority.
//!
//! Selection uses the authoritative `runner::topn` two-pass fixed-point
//! ranking, including its complete deterministic tie order.  The selection
//! itself is O(C) over C combined dispositions and uses O(C) temporary
//! uniqueness/projection state in this persistence core.  Opening/reopening is
//! O(F) in explicitly bounded file bytes.  Exact lookup after open is one
//! expected/amortized-O(1) `HashMap` probe; Rust's `HashMap` does not promise
//! worst-case O(1).  Hashing, filesystem latency, selection, persistence,
//! recovery and a complete run are not O(1).
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;
#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
use std::os::unix::fs::OpenOptionsExt as _;

use brutex_core::blake3::Hasher;
use costs::fill::Direction;
use runner::admission::{AdmissionStatusV1, ObservedI64V1, ObservedU64V1};
use runner::portfolio::StrategyDigest;
use runner::topn::{
    Candidate, MAX_TOP, Metrics, PopulationProof, RankedCandidate, RankingPolicyV1, SCORE_SCALE,
    select,
};

use crate::execution_v3::{
    CommittedStoredExecutionV3, ExecutionV3AdmissionStatus, ExecutionV3Direction,
    ExecutionV3Family, ExecutionV3StructuralReceipt, ExecutionV3SuccessorDisposition,
    ExecutionV3Terminal,
};
use crate::population::{InstrumentFamilyV1, TopMetricsV1, TradeDirectionV1};
use crate::population_admission_v3::{AdmissionV3Family, AdmissionV3Status};
use crate::population_v5::{
    PopulationV5ExecutionDispositionSourceV1, PopulationV5ExecutionV3SourceV1,
};

/// Bytes in one sealed Selection V5 winner row.
pub(crate) const SELECTION_V5_ROW_BYTES: usize = 1_024;
/// Bytes in one receipt-last Selection V5 Completion.
pub(crate) const SELECTION_V5_COMPLETION_BYTES: usize = 1_024;

const VERSION: u32 = 5;
const ROW_MAGIC: [u8; 16] = *b"BTX-SELV5-ROW\0\0\0";
const COMPLETION_MAGIC: [u8; 16] = *b"BTX-SELV5-CMP\0\0\0";
const ROW_DOMAIN: u32 = 1;
const COMPLETION_DOMAIN: u32 = 2;
const SEAL_BYTES: usize = 32;
const ROW_PAYLOAD_BYTES: usize = SELECTION_V5_ROW_BYTES - SEAL_BYTES;
const COMPLETION_PAYLOAD_BYTES: usize = SELECTION_V5_COMPLETION_BYTES - SEAL_BYTES;
const REQUESTED_TOP: usize = 25;
const REQUESTED_TOP_U32: u32 = 25;
const REQUESTED_TOP_U64: u64 = 25;
const TOP_TEN: usize = 10;
const TOP_TEN_U64: u64 = 10;
const ROW_BYTES_U64: u64 = 1_024;
const COMPLETION_BYTES_U64: u64 = 1_024;
const TERMINAL_MATRIX_CELLS: usize = 16;
const READ_CHUNK_BYTES: usize = 16 * 1_024;
const CANONICAL_RUNGS: [u32; 8] = [60, 120, 180, 300, 600, 900, 1_800, 3_600];

const SELECTION_ID_DOMAIN: &[u8] = b"brutex-selection-v5-id\0";
const ROW_ID_DOMAIN: &[u8] = b"brutex-selection-v5-row-id\0";
const ORDERED_SELECTED_DOMAIN: &[u8] = b"brutex-selection-v5-ordered-selected\0";
const COMPLETION_ID_DOMAIN: &[u8] = b"brutex-selection-v5-completion-id\0";
const ROW_SEAL_DOMAIN: &[u8] = b"brutex-selection-v5-row-seal\0";
const COMPLETION_SEAL_DOMAIN: &[u8] = b"brutex-selection-v5-completion-seal\0";
const FILE_GENERATION_DOMAIN: &[u8] = b"brutex-selection-v5-file-generation\0";

const ROW_FILE: &str = "global-selection-rows-v5.bin";
const COMPLETION_FILE: &str = "global-selection-completions-v5.bin";
const LOCK_FILE: &str = "global-selection-v5.lock";
const LOCK_MAX_BYTES: u64 = 0;

#[cfg(any(target_os = "android", target_os = "linux"))]
const O_NOFOLLOW_FLAG: i32 = 0x20_000;
#[cfg(target_os = "macos")]
const O_NOFOLLOW_FLAG: i32 = 0x100;

const _: () = assert!(MAX_TOP == REQUESTED_TOP);
const _: () = assert!(ROW_PAYLOAD_BYTES + SEAL_BYTES == SELECTION_V5_ROW_BYTES);
const _: () = assert!(COMPLETION_PAYLOAD_BYTES + SEAL_BYTES == SELECTION_V5_COMPLETION_BYTES);

/// Operator-facing refusal at the Selection V5 boundary.
pub(crate) type SelectionV5Refusal = String;

/// Explicit physical ceilings.  There is deliberately no `Default`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SelectionV5Bounds {
    row_records: u64,
    row_bytes: u64,
    completion_records: u64,
    completion_bytes: u64,
}

impl SelectionV5Bounds {
    /// Builds nonzero record and byte ceilings.
    ///
    /// # Errors
    ///
    /// Refuses zero, arithmetic overflow, or a byte bound too small for its
    /// declared fixed-record count.
    pub(crate) fn new(
        max_row_records: u64,
        max_row_bytes: u64,
        max_completion_records: u64,
        max_completion_bytes: u64,
    ) -> Result<Self, SelectionV5Refusal> {
        for (name, value) in [
            ("row records", max_row_records),
            ("row bytes", max_row_bytes),
            ("Completion records", max_completion_records),
            ("Completion bytes", max_completion_bytes),
        ] {
            if value == 0 {
                return Err(format!("Selection V5 maximum {name} must be nonzero"));
            }
        }
        let required_rows = max_row_records
            .checked_mul(ROW_BYTES_U64)
            .ok_or_else(|| "Selection V5 row byte ceiling overflowed".to_owned())?;
        if max_row_bytes < required_rows {
            return Err(format!(
                "Selection V5 row byte maximum {max_row_bytes} cannot hold {max_row_records} records ({required_rows} bytes)"
            ));
        }
        let required_completions = max_completion_records
            .checked_mul(COMPLETION_BYTES_U64)
            .ok_or_else(|| "Selection V5 Completion byte ceiling overflowed".to_owned())?;
        if max_completion_bytes < required_completions {
            return Err(format!(
                "Selection V5 Completion byte maximum {max_completion_bytes} cannot hold {max_completion_records} records ({required_completions} bytes)"
            ));
        }
        Ok(Self {
            row_records: max_row_records,
            row_bytes: max_row_bytes,
            completion_records: max_completion_records,
            completion_bytes: max_completion_bytes,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
enum SelectionV5Family {
    Nifty = 1,
    BankNifty = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum SelectionV5Terminal {
    Authorized = 1,
    PolicyRefused = 2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SelectionV5SourceSnapshot {
    population_id: [u8; 32],
    population_ordered_digest: [u8; 32],
    execution_completion_id: [u8; 32],
    ordered_disposition_digest: [u8; 32],
    nifty_authority_id: [u8; 32],
    banknifty_authority_id: [u8; 32],
    rung_seconds: u32,
    row_count: u64,
    nifty_count: u64,
    banknifty_count: u64,
    admitted_count: u64,
    authorized_count: u64,
    eligible_count: u64,
    terminal_matrix: [u64; TERMINAL_MATRIX_CELLS],
}

impl SelectionV5SourceSnapshot {
    fn validate(&self) -> Result<(), SelectionV5Refusal> {
        for (name, value) in [
            ("Population V5 identity", self.population_id),
            (
                "Population V5 ordered digest",
                self.population_ordered_digest,
            ),
            ("Execution V3 Completion", self.execution_completion_id),
            (
                "Execution V3 ordered disposition digest",
                self.ordered_disposition_digest,
            ),
            ("Execution V3 NIFTY authority", self.nifty_authority_id),
            (
                "Execution V3 BANKNIFTY authority",
                self.banknifty_authority_id,
            ),
        ] {
            require_nonzero(name, value)?;
        }
        if !CANONICAL_RUNGS.contains(&self.rung_seconds) {
            return Err(format!(
                "Selection V5 rung {}s is not canonical",
                self.rung_seconds
            ));
        }
        if self.row_count == 0 || self.nifty_count == 0 || self.banknifty_count == 0 {
            return Err(
                "Selection V5 requires complete nonempty NIFTY and BANKNIFTY families".to_owned(),
            );
        }
        if checked_sum([self.nifty_count, self.banknifty_count], "family counts")? != self.row_count
            || self.admitted_count > self.row_count
            || self.authorized_count > self.row_count
            || self.eligible_count > self.admitted_count
            || self.eligible_count > self.authorized_count
            || checked_sum(self.terminal_matrix, "terminal matrix")? != self.row_count
        {
            return Err("Selection V5 source counts are contradictory".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SelectionV5SourceRow {
    population_id: [u8; 32],
    population_ordered_digest: [u8; 32],
    execution_completion_id: [u8; 32],
    disposition_id: [u8; 32],
    global_sequence: u64,
    family_sequence: u64,
    family: SelectionV5Family,
    strategy_digest: [u8; 32],
    mask_words: [u64; 6],
    direction: Direction,
    admission: AdmissionStatusV1,
    terminal: SelectionV5Terminal,
    metrics: Metrics,
}

impl SelectionV5SourceRow {
    fn candidate(self) -> Candidate {
        Candidate {
            strategy_digest: StrategyDigest::new(self.strategy_digest),
            mask_words: self.mask_words,
            direction: self.direction,
            admitted: self.admission == AdmissionStatusV1::Admitted
                && self.terminal == SelectionV5Terminal::Authorized,
            metrics: self.metrics,
        }
    }

    fn validate_against(
        self,
        source: &SelectionV5SourceSnapshot,
    ) -> Result<(), SelectionV5Refusal> {
        if self.population_id != source.population_id
            || self.population_ordered_digest != source.population_ordered_digest
            || self.execution_completion_id != source.execution_completion_id
        {
            return Err(
                "Selection V5 row names a different retained Execution V3 source".to_owned(),
            );
        }
        require_nonzero("Execution V3 disposition", self.disposition_id)?;
        require_nonzero("strategy digest", self.strategy_digest)?;
        if self.mask_words.iter().all(|word| *word == 0) {
            return Err("Selection V5 row contains an empty condition mask".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SelectionV5RowRecord {
    selection_id: [u8; 32],
    row_id: [u8; 32],
    population_id: [u8; 32],
    population_ordered_digest: [u8; 32],
    execution_completion_id: [u8; 32],
    ordered_disposition_digest: [u8; 32],
    nifty_authority_id: [u8; 32],
    banknifty_authority_id: [u8; 32],
    ranking_policy_digest: [u8; 32],
    disposition_id: [u8; 32],
    strategy_digest: [u8; 32],
    mask_words: [u64; 6],
    global_sequence: u64,
    family_sequence: u64,
    score: u64,
    metrics: Metrics,
    rung_seconds: u32,
    rank: u32,
    family: SelectionV5Family,
    direction: Direction,
    admission: AdmissionStatusV1,
    terminal: SelectionV5Terminal,
}

/// Reauthenticated winner projection for typed successors.
///
/// The persisted Selection row deliberately does not duplicate the selected
/// exit digest. This projection exact-joins it back to the retained durable
/// Execution V3 disposition before a successor can observe either value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SelectionV5SuccessorWinnerV1 {
    row: SelectionV5RowRecord,
    selected_exit_digest: Option<[u8; 32]>,
}

impl SelectionV5SuccessorWinnerV1 {
    #[must_use]
    pub(crate) const fn row(self) -> SelectionV5RowRecord {
        self.row
    }

    #[must_use]
    pub(crate) const fn selected_exit_digest(self) -> Option<[u8; 32]> {
        self.selected_exit_digest
    }
}

impl SelectionV5RowRecord {
    #[must_use]
    pub(crate) const fn selection_id(&self) -> [u8; 32] {
        self.selection_id
    }

    #[must_use]
    pub(crate) const fn row_id(&self) -> [u8; 32] {
        self.row_id
    }

    #[must_use]
    pub(crate) const fn execution_completion_id(&self) -> [u8; 32] {
        self.execution_completion_id
    }

    #[must_use]
    pub(crate) const fn disposition_id(&self) -> [u8; 32] {
        self.disposition_id
    }

    #[must_use]
    pub(crate) const fn strategy_digest(&self) -> [u8; 32] {
        self.strategy_digest
    }

    #[must_use]
    pub(crate) const fn mask_words(&self) -> [u64; 6] {
        self.mask_words
    }

    #[must_use]
    pub(crate) const fn direction(&self) -> Direction {
        self.direction
    }

    #[must_use]
    pub(crate) const fn instrument_family(&self) -> InstrumentFamilyV1 {
        match self.family {
            SelectionV5Family::Nifty => InstrumentFamilyV1::Nifty,
            SelectionV5Family::BankNifty => InstrumentFamilyV1::BankNifty,
        }
    }

    #[must_use]
    pub(crate) const fn metrics(&self) -> Metrics {
        self.metrics
    }

    #[must_use]
    pub(crate) const fn rung_seconds(&self) -> u32 {
        self.rung_seconds
    }

    #[must_use]
    pub(crate) const fn rank(&self) -> u32 {
        self.rank
    }

    /// This row's canonical bytes.
    ///
    /// # Why it has no caller yet
    ///
    /// `ledger-all` prints the Top-10 it read; it does not yet hash what it
    /// printed. A report that carried the digest of its own rows could be
    /// checked against the ledger without reopening it, and this is the
    /// function that would produce it. Recorded as an `expect` rather than an
    /// `allow` so that wiring it up makes THIS annotation the build failure.
    #[expect(
        dead_code,
        reason = "the digest of a printed report is Step 5; nothing hashes rendered rows today"
    )]
    pub(crate) fn canonical_record(
        &self,
    ) -> Result<[u8; SELECTION_V5_ROW_BYTES], SelectionV5Refusal> {
        self.encode()
    }

    fn validate(&self) -> Result<(), SelectionV5Refusal> {
        for (name, value) in [
            ("selection identity", self.selection_id),
            ("selection row identity", self.row_id),
            ("Population V5 identity", self.population_id),
            (
                "Population V5 ordered digest",
                self.population_ordered_digest,
            ),
            ("Execution V3 Completion", self.execution_completion_id),
            (
                "Execution V3 ordered disposition digest",
                self.ordered_disposition_digest,
            ),
            ("Execution V3 NIFTY authority", self.nifty_authority_id),
            (
                "Execution V3 BANKNIFTY authority",
                self.banknifty_authority_id,
            ),
            ("ranking policy", self.ranking_policy_digest),
            ("Execution V3 disposition", self.disposition_id),
            ("strategy digest", self.strategy_digest),
        ] {
            require_nonzero(name, value)?;
        }
        if !CANONICAL_RUNGS.contains(&self.rung_seconds)
            || usize::try_from(self.rank)
                .ok()
                .is_none_or(|rank| rank >= REQUESTED_TOP)
            || self.score > SCORE_SCALE
            || self.admission != AdmissionStatusV1::Admitted
            || self.terminal != SelectionV5Terminal::Authorized
            || self.mask_words.iter().all(|word| *word == 0)
        {
            return Err(
                "Selection V5 winner row violates rank/eligibility/metric policy".to_owned(),
            );
        }
        if self.row_id != self.derive_row_id() {
            return Err("Selection V5 winner row identity does not reproduce".to_owned());
        }
        Ok(())
    }

    fn derive_row_id(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(ROW_ID_DOMAIN);
        hasher.update(&self.selection_id);
        hasher.update(&self.rank.to_le_bytes());
        hasher.update(&self.disposition_id);
        hasher.update(&self.strategy_digest);
        hasher.finalize()
    }

    fn semantic_bytes(&self) -> Result<[u8; ROW_PAYLOAD_BYTES], SelectionV5Refusal> {
        let mut bytes = [0_u8; ROW_PAYLOAD_BYTES];
        self.encode_payload(&mut bytes)?;
        Ok(bytes)
    }

    fn encode(&self) -> Result<[u8; SELECTION_V5_ROW_BYTES], SelectionV5Refusal> {
        self.validate()?;
        let mut raw = [0_u8; SELECTION_V5_ROW_BYTES];
        let (payload, seal) = raw.split_at_mut(ROW_PAYLOAD_BYTES);
        self.encode_payload(payload)?;
        seal.copy_from_slice(&hash_parts(ROW_SEAL_DOMAIN, &[payload]));
        Ok(raw)
    }

    fn encode_payload(&self, payload: &mut [u8]) -> Result<(), SelectionV5Refusal> {
        let mut writer = FixedWriter::new(payload);
        write_header(&mut writer, ROW_MAGIC, ROW_DOMAIN)?;
        for value in [
            self.selection_id,
            self.row_id,
            self.population_id,
            self.population_ordered_digest,
            self.execution_completion_id,
            self.ordered_disposition_digest,
            self.nifty_authority_id,
            self.banknifty_authority_id,
            self.ranking_policy_digest,
            self.disposition_id,
            self.strategy_digest,
        ] {
            writer.bytes(&value)?;
        }
        for word in self.mask_words {
            writer.u64(word)?;
        }
        for value in [self.global_sequence, self.family_sequence, self.score] {
            writer.u64(value)?;
        }
        encode_metrics(&mut writer, self.metrics)?;
        writer.u32(self.rung_seconds)?;
        writer.u32(self.rank)?;
        writer.u8(family_byte(self.family))?;
        writer.u8(direction_byte(self.direction))?;
        writer.u8(admission_byte(self.admission))?;
        writer.u8(terminal_byte(self.terminal))?;
        writer.zeros(writer.remaining())?;
        writer.require_full("Selection V5 row payload")
    }

    fn decode(raw: &[u8; SELECTION_V5_ROW_BYTES]) -> Result<Self, SelectionV5Refusal> {
        let (payload, seal) = raw.split_at(ROW_PAYLOAD_BYTES);
        require_seal("winner row", ROW_SEAL_DOMAIN, payload, seal)?;
        let mut reader = FixedReader::new(payload);
        require_header("winner row", &mut reader, ROW_MAGIC, ROW_DOMAIN)?;
        let row = Self {
            selection_id: reader.array()?,
            row_id: reader.array()?,
            population_id: reader.array()?,
            population_ordered_digest: reader.array()?,
            execution_completion_id: reader.array()?,
            ordered_disposition_digest: reader.array()?,
            nifty_authority_id: reader.array()?,
            banknifty_authority_id: reader.array()?,
            ranking_policy_digest: reader.array()?,
            disposition_id: reader.array()?,
            strategy_digest: reader.array()?,
            mask_words: [
                reader.u64()?,
                reader.u64()?,
                reader.u64()?,
                reader.u64()?,
                reader.u64()?,
                reader.u64()?,
            ],
            global_sequence: reader.u64()?,
            family_sequence: reader.u64()?,
            score: reader.u64()?,
            metrics: decode_metrics(&mut reader)?,
            rung_seconds: reader.u32()?,
            rank: reader.u32()?,
            family: decode_family(reader.u8()?)?,
            direction: decode_direction(reader.u8()?)?,
            admission: decode_admission(reader.u8()?)?,
            terminal: decode_terminal(reader.u8()?)?,
        };
        reader.require_zeros(reader.remaining(), "winner row trailing reserve")?;
        row.validate()?;
        if row.encode()? != *raw {
            return Err("Selection V5 winner row is not byte-canonical".to_owned());
        }
        Ok(row)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SelectionV5CompletionRecord {
    block_sequence: u64,
    first_row_record: u64,
    selected_count: u64,
    completion_id: [u8; 32],
    selection_id: [u8; 32],
    population_id: [u8; 32],
    population_ordered_digest: [u8; 32],
    execution_completion_id: [u8; 32],
    ordered_disposition_digest: [u8; 32],
    nifty_authority_id: [u8; 32],
    banknifty_authority_id: [u8; 32],
    ranking_policy_digest: [u8; 32],
    ordered_selected_digest: [u8; 32],
    proof_ordered_digest: [u8; 32],
    rung_seconds: u32,
    requested_top: u32,
    top_ten_count: u32,
    row_count: u64,
    nifty_count: u64,
    banknifty_count: u64,
    admitted_count: u64,
    authorized_count: u64,
    eligible_count: u64,
    proof_considered: u64,
    proof_admitted: u64,
    proof_refused: u64,
    proof_unmeasured: u64,
    terminal_matrix: [u64; TERMINAL_MATRIX_CELLS],
}

impl SelectionV5CompletionRecord {
    fn validate(&self) -> Result<(), SelectionV5Refusal> {
        for (name, value) in [
            ("Completion identity", self.completion_id),
            ("selection identity", self.selection_id),
            ("Population V5 identity", self.population_id),
            (
                "Population V5 ordered digest",
                self.population_ordered_digest,
            ),
            ("Execution V3 Completion", self.execution_completion_id),
            (
                "Execution V3 ordered disposition digest",
                self.ordered_disposition_digest,
            ),
            ("Execution V3 NIFTY authority", self.nifty_authority_id),
            (
                "Execution V3 BANKNIFTY authority",
                self.banknifty_authority_id,
            ),
            ("ranking policy", self.ranking_policy_digest),
            ("ordered selected rows", self.ordered_selected_digest),
            ("complete population proof", self.proof_ordered_digest),
        ] {
            require_nonzero(name, value)?;
        }
        if !CANONICAL_RUNGS.contains(&self.rung_seconds)
            || self.requested_top != REQUESTED_TOP_U32
            || self.selected_count > REQUESTED_TOP_U64
            || u64::from(self.top_ten_count) != self.selected_count.min(TOP_TEN_U64)
            || self.selected_count != self.eligible_count.min(REQUESTED_TOP_U64)
        {
            return Err(
                "Selection V5 Completion violates fixed Top-25/Top-10 semantics".to_owned(),
            );
        }
        if self.row_count == 0
            || self.nifty_count == 0
            || self.banknifty_count == 0
            || checked_sum([self.nifty_count, self.banknifty_count], "family counts")?
                != self.row_count
            || self.admitted_count > self.row_count
            || self.authorized_count > self.row_count
            || self.eligible_count > self.admitted_count
            || self.eligible_count > self.authorized_count
            || checked_sum(self.terminal_matrix, "terminal matrix")? != self.row_count
            || self.proof_considered != self.row_count
            || self.proof_admitted != self.eligible_count
            || self.proof_unmeasured > self.proof_considered
            || checked_sum(
                [self.proof_admitted, self.proof_refused],
                "proof terminal counts",
            )? != self.proof_considered
        {
            return Err("Selection V5 Completion source/proof counts are contradictory".to_owned());
        }
        let (matrix_nifty, matrix_bank, matrix_admitted, matrix_authorized, matrix_eligible) =
            matrix_marginals(&self.terminal_matrix)?;
        if matrix_nifty != self.nifty_count
            || matrix_bank != self.banknifty_count
            || matrix_admitted != self.admitted_count
            || matrix_authorized != self.authorized_count
            || matrix_eligible != self.eligible_count
        {
            return Err("Selection V5 Completion matrix marginals do not reproduce".to_owned());
        }
        if self.completion_id != self.derive_completion_id() {
            return Err("Selection V5 Completion identity does not reproduce".to_owned());
        }
        Ok(())
    }

    fn derive_completion_id(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(COMPLETION_ID_DOMAIN);
        for value in [
            self.selection_id,
            self.population_id,
            self.population_ordered_digest,
            self.execution_completion_id,
            self.ordered_disposition_digest,
            self.nifty_authority_id,
            self.banknifty_authority_id,
            self.ranking_policy_digest,
            self.ordered_selected_digest,
            self.proof_ordered_digest,
        ] {
            hasher.update(&value);
        }
        for value in [
            u64::from(self.rung_seconds),
            u64::from(self.requested_top),
            u64::from(self.top_ten_count),
            self.selected_count,
            self.row_count,
            self.nifty_count,
            self.banknifty_count,
            self.admitted_count,
            self.authorized_count,
            self.eligible_count,
            self.proof_considered,
            self.proof_admitted,
            self.proof_refused,
            self.proof_unmeasured,
        ] {
            hasher.update(&value.to_le_bytes());
        }
        for value in self.terminal_matrix {
            hasher.update(&value.to_le_bytes());
        }
        hasher.finalize()
    }

    fn encode(&self) -> Result<[u8; SELECTION_V5_COMPLETION_BYTES], SelectionV5Refusal> {
        self.validate()?;
        let mut raw = [0_u8; SELECTION_V5_COMPLETION_BYTES];
        let (payload, seal) = raw.split_at_mut(COMPLETION_PAYLOAD_BYTES);
        let mut writer = FixedWriter::new(payload);
        write_header(&mut writer, COMPLETION_MAGIC, COMPLETION_DOMAIN)?;
        for value in [
            self.block_sequence,
            self.first_row_record,
            self.selected_count,
        ] {
            writer.u64(value)?;
        }
        for value in [
            self.completion_id,
            self.selection_id,
            self.population_id,
            self.population_ordered_digest,
            self.execution_completion_id,
            self.ordered_disposition_digest,
            self.nifty_authority_id,
            self.banknifty_authority_id,
            self.ranking_policy_digest,
            self.ordered_selected_digest,
            self.proof_ordered_digest,
        ] {
            writer.bytes(&value)?;
        }
        writer.u32(self.rung_seconds)?;
        writer.u32(self.requested_top)?;
        writer.u32(self.top_ten_count)?;
        writer.u32(0)?;
        for value in [
            self.row_count,
            self.nifty_count,
            self.banknifty_count,
            self.admitted_count,
            self.authorized_count,
            self.eligible_count,
            self.proof_considered,
            self.proof_admitted,
            self.proof_refused,
            self.proof_unmeasured,
        ] {
            writer.u64(value)?;
        }
        for value in self.terminal_matrix {
            writer.u64(value)?;
        }
        writer.zeros(writer.remaining())?;
        writer.require_full("Selection V5 Completion payload")?;
        seal.copy_from_slice(&hash_parts(COMPLETION_SEAL_DOMAIN, &[payload]));
        Ok(raw)
    }

    fn decode(raw: &[u8; SELECTION_V5_COMPLETION_BYTES]) -> Result<Self, SelectionV5Refusal> {
        let (payload, seal) = raw.split_at(COMPLETION_PAYLOAD_BYTES);
        require_seal("Completion", COMPLETION_SEAL_DOMAIN, payload, seal)?;
        let mut reader = FixedReader::new(payload);
        require_header(
            "Completion",
            &mut reader,
            COMPLETION_MAGIC,
            COMPLETION_DOMAIN,
        )?;
        let completion = Self {
            block_sequence: reader.u64()?,
            first_row_record: reader.u64()?,
            selected_count: reader.u64()?,
            completion_id: reader.array()?,
            selection_id: reader.array()?,
            population_id: reader.array()?,
            population_ordered_digest: reader.array()?,
            execution_completion_id: reader.array()?,
            ordered_disposition_digest: reader.array()?,
            nifty_authority_id: reader.array()?,
            banknifty_authority_id: reader.array()?,
            ranking_policy_digest: reader.array()?,
            ordered_selected_digest: reader.array()?,
            proof_ordered_digest: reader.array()?,
            rung_seconds: reader.u32()?,
            requested_top: reader.u32()?,
            top_ten_count: reader.u32()?,
            row_count: {
                if reader.u32()? != 0 {
                    return Err("Selection V5 Completion reserved word is nonzero".to_owned());
                }
                reader.u64()?
            },
            nifty_count: reader.u64()?,
            banknifty_count: reader.u64()?,
            admitted_count: reader.u64()?,
            authorized_count: reader.u64()?,
            eligible_count: reader.u64()?,
            proof_considered: reader.u64()?,
            proof_admitted: reader.u64()?,
            proof_refused: reader.u64()?,
            proof_unmeasured: reader.u64()?,
            terminal_matrix: [
                reader.u64()?,
                reader.u64()?,
                reader.u64()?,
                reader.u64()?,
                reader.u64()?,
                reader.u64()?,
                reader.u64()?,
                reader.u64()?,
                reader.u64()?,
                reader.u64()?,
                reader.u64()?,
                reader.u64()?,
                reader.u64()?,
                reader.u64()?,
                reader.u64()?,
                reader.u64()?,
            ],
        };
        reader.require_zeros(reader.remaining(), "Completion trailing reserve")?;
        completion.validate()?;
        if completion.encode()? != *raw {
            return Err("Selection V5 Completion is not byte-canonical".to_owned());
        }
        Ok(completion)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedSelectionV5 {
    source: SelectionV5SourceSnapshot,
    ranking_policy_digest: [u8; 32],
    proof: PopulationProof,
    rows: Vec<SelectionV5RowRecord>,
    selection_id: [u8; 32],
}

impl PreparedSelectionV5 {
    /// Builds one Selection V5 block from the only source-retaining Execution
    /// V3 production capability. The source is authenticated before, during,
    /// and after the durable disposition read; neither a row slice nor a
    /// detached receipt can enter this path.
    fn from_committed_execution(
        source: &mut CommittedStoredExecutionV3,
        policy: RankingPolicyV1,
    ) -> Result<Self, SelectionV5Refusal> {
        let execution_receipt_before = source.structural_receipt();
        let population_before = source
            .population_execution_source()
            .map_err(|why| format!("Selection V5 retained Population source refused: {why}"))?;
        let dispositions = source
            .ordered_authenticated_dispositions()
            .map_err(|why| format!("Selection V5 durable Execution source refused: {why}"))?;
        let population_after = source
            .population_execution_source()
            .map_err(|why| format!("Selection V5 post-read Population source refused: {why}"))?;
        let execution_receipt_after = source.structural_receipt();
        if execution_receipt_after != execution_receipt_before
            || population_after != population_before
        {
            return Err(
                "Selection V5 retained Population/Execution source changed during projection"
                    .to_owned(),
            );
        }
        let (snapshot, rows) =
            project_execution_source(&population_before, execution_receipt_before, &dispositions)?;
        Self::from_projected_source(snapshot, &rows, policy)
    }

    #[cfg(test)]
    fn from_authenticated_execution_fixture(
        source: SelectionV5SourceSnapshot,
        rows: &[SelectionV5SourceRow],
        policy: RankingPolicyV1,
    ) -> Result<Self, SelectionV5Refusal> {
        Self::from_projected_source(source, rows, policy)
    }

    fn from_projected_source(
        source: SelectionV5SourceSnapshot,
        rows: &[SelectionV5SourceRow],
        policy: RankingPolicyV1,
    ) -> Result<Self, SelectionV5Refusal> {
        source.validate()?;
        validate_complete_source(&source, rows)?;
        let (selection, proof) = Self::rank(&source, rows, policy)?;
        let ranking_policy_digest = policy.digest();
        require_nonzero("ranking policy", ranking_policy_digest)?;
        let mut selected_rows =
            Self::resolve_winners(&source, rows, ranking_policy_digest, &selection.rows)?;
        let selection_id =
            derive_selection_id(&source, ranking_policy_digest, proof, &selected_rows)?;
        for row in &mut selected_rows {
            row.selection_id = selection_id;
            row.row_id = row.derive_row_id();
            row.validate()?;
        }
        let prepared = Self {
            source,
            ranking_policy_digest,
            proof,
            rows: selected_rows,
            selection_id,
        };
        prepared.validate()?;
        Ok(prepared)
    }

    fn rank(
        source: &SelectionV5SourceSnapshot,
        rows: &[SelectionV5SourceRow],
        policy: RankingPolicyV1,
    ) -> Result<(runner::topn::Selection, PopulationProof), SelectionV5Refusal> {
        let candidates: Vec<Candidate> = rows
            .iter()
            .copied()
            .map(SelectionV5SourceRow::candidate)
            .collect();
        let selection = select(&candidates, policy, REQUESTED_TOP).map_err(|why| {
            format!("Selection V5 ranking refused the complete population: {why:?}")
        })?;
        let mut pass = runner::topn::PopulationPass::new();
        for candidate in &candidates {
            pass.observe(candidate);
        }
        let proof = pass.finish();
        if proof.considered != source.row_count
            || proof.admitted != source.eligible_count
            || proof.refused
                != source
                    .row_count
                    .checked_sub(source.eligible_count)
                    .ok_or_else(|| "Selection V5 proof refused-count underflowed".to_owned())?
            || selection.considered != proof.considered
            || selection.admitted != proof.admitted
            || selection.refused != proof.refused
            || selection.unmeasured != proof.unmeasured
        {
            return Err(
                "Selection V5 ranking did not reconcile its complete source proof".to_owned(),
            );
        }
        Ok((selection, proof))
    }

    fn resolve_winners(
        source: &SelectionV5SourceSnapshot,
        rows: &[SelectionV5SourceRow],
        ranking_policy_digest: [u8; 32],
        ranked_rows: &[runner::topn::RankedCandidate],
    ) -> Result<Vec<SelectionV5RowRecord>, SelectionV5Refusal> {
        let mut by_strategy = HashMap::new();
        by_strategy
            .try_reserve(rows.len())
            .map_err(|why| format!("Selection V5 source resolution index reserve failed: {why}"))?;
        for row in rows {
            if by_strategy.insert(row.strategy_digest, *row).is_some() {
                return Err(format!(
                    "Selection V5 strategy alias {} occurs more than once",
                    hex32(row.strategy_digest)
                ));
            }
        }
        let mut selected_rows = Vec::new();
        selected_rows
            .try_reserve_exact(ranked_rows.len())
            .map_err(|why| format!("Selection V5 selected-row reserve failed: {why}"))?;
        for (rank, ranked) in ranked_rows.iter().enumerate() {
            let strategy_digest = ranked.candidate.strategy_digest.bytes();
            let source_row = by_strategy.get(&strategy_digest).copied().ok_or_else(|| {
                format!(
                    "Selection V5 winner {} cannot be resolved to its Execution V3 disposition",
                    hex32(strategy_digest)
                )
            })?;
            if source_row.candidate() != ranked.candidate {
                return Err("Selection V5 winner differs from its resolved source row".to_owned());
            }
            selected_rows.push(SelectionV5RowRecord {
                selection_id: [0; 32],
                row_id: [0; 32],
                population_id: source.population_id,
                population_ordered_digest: source.population_ordered_digest,
                execution_completion_id: source.execution_completion_id,
                ordered_disposition_digest: source.ordered_disposition_digest,
                nifty_authority_id: source.nifty_authority_id,
                banknifty_authority_id: source.banknifty_authority_id,
                ranking_policy_digest,
                disposition_id: source_row.disposition_id,
                strategy_digest,
                mask_words: ranked.candidate.mask_words,
                global_sequence: source_row.global_sequence,
                family_sequence: source_row.family_sequence,
                score: ranked.score,
                metrics: ranked.candidate.metrics,
                rung_seconds: source.rung_seconds,
                rank: u32::try_from(rank)
                    .map_err(|_| "Selection V5 rank does not fit u32".to_owned())?,
                family: source_row.family,
                direction: source_row.direction,
                admission: source_row.admission,
                terminal: source_row.terminal,
            });
        }
        Ok(selected_rows)
    }

    fn validate(&self) -> Result<(), SelectionV5Refusal> {
        self.source.validate()?;
        require_nonzero("ranking policy", self.ranking_policy_digest)?;
        require_nonzero("selection identity", self.selection_id)?;
        if self.rows.len()
            != usize::try_from(self.source.eligible_count.min(REQUESTED_TOP_U64))
                .map_err(|_| "Selection V5 selected count does not fit usize".to_owned())?
            || self.proof.considered != self.source.row_count
            || self.proof.admitted != self.source.eligible_count
            || self.proof.unmeasured > self.proof.considered
        {
            return Err("Selection V5 prepared counts do not reconcile".to_owned());
        }
        let mut row_ids = HashSet::new();
        let mut strategy_ids = HashSet::new();
        row_ids
            .try_reserve(self.rows.len())
            .map_err(|why| format!("Selection V5 row-identity reserve failed: {why}"))?;
        strategy_ids
            .try_reserve(self.rows.len())
            .map_err(|why| format!("Selection V5 strategy-identity reserve failed: {why}"))?;
        for (index, row) in self.rows.iter().enumerate() {
            row.validate()?;
            if row.selection_id != self.selection_id
                || row.population_id != self.source.population_id
                || row.population_ordered_digest != self.source.population_ordered_digest
                || row.execution_completion_id != self.source.execution_completion_id
                || row.ordered_disposition_digest != self.source.ordered_disposition_digest
                || row.nifty_authority_id != self.source.nifty_authority_id
                || row.banknifty_authority_id != self.source.banknifty_authority_id
                || row.ranking_policy_digest != self.ranking_policy_digest
                || row.rung_seconds != self.source.rung_seconds
                || usize::try_from(row.rank).ok() != Some(index)
                || !row_ids.insert(row.row_id)
                || !strategy_ids.insert(row.strategy_digest)
            {
                return Err(
                    "Selection V5 prepared winner order/source/identity is invalid".to_owned(),
                );
            }
        }
        validate_strongest_first(&self.rows)?;
        if self.selection_id
            != derive_selection_id(
                &self.source,
                self.ranking_policy_digest,
                self.proof,
                &self.rows,
            )?
        {
            return Err("Selection V5 identity does not reproduce".to_owned());
        }
        Ok(())
    }

    fn expected_completion(
        &self,
        block_sequence: u64,
        first_row_record: u64,
    ) -> Result<SelectionV5CompletionRecord, SelectionV5Refusal> {
        self.validate()?;
        let selected_count = usize_to_u64(self.rows.len(), "selected row count")?;
        let ordered_selected_digest = ordered_selected_digest(&self.rows)?;
        let mut completion = SelectionV5CompletionRecord {
            block_sequence,
            first_row_record,
            selected_count,
            completion_id: [0; 32],
            selection_id: self.selection_id,
            population_id: self.source.population_id,
            population_ordered_digest: self.source.population_ordered_digest,
            execution_completion_id: self.source.execution_completion_id,
            ordered_disposition_digest: self.source.ordered_disposition_digest,
            nifty_authority_id: self.source.nifty_authority_id,
            banknifty_authority_id: self.source.banknifty_authority_id,
            ranking_policy_digest: self.ranking_policy_digest,
            ordered_selected_digest,
            proof_ordered_digest: self.proof.ordered_digest,
            rung_seconds: self.source.rung_seconds,
            requested_top: REQUESTED_TOP_U32,
            top_ten_count: u32::try_from(self.rows.len().min(TOP_TEN))
                .map_err(|_| "Selection V5 Top-10 count does not fit u32".to_owned())?,
            row_count: self.source.row_count,
            nifty_count: self.source.nifty_count,
            banknifty_count: self.source.banknifty_count,
            admitted_count: self.source.admitted_count,
            authorized_count: self.source.authorized_count,
            eligible_count: self.source.eligible_count,
            proof_considered: self.proof.considered,
            proof_admitted: self.proof.admitted,
            proof_refused: self.proof.refused,
            proof_unmeasured: self.proof.unmeasured,
            terminal_matrix: self.source.terminal_matrix,
        };
        completion.completion_id = completion.derive_completion_id();
        completion.validate()?;
        Ok(completion)
    }
}

fn derive_selection_id(
    source: &SelectionV5SourceSnapshot,
    ranking_policy_digest: [u8; 32],
    proof: PopulationProof,
    rows: &[SelectionV5RowRecord],
) -> Result<[u8; 32], SelectionV5Refusal> {
    let mut hasher = Hasher::new();
    hasher.update(SELECTION_ID_DOMAIN);
    for value in [
        source.population_id,
        source.population_ordered_digest,
        source.execution_completion_id,
        source.ordered_disposition_digest,
        source.nifty_authority_id,
        source.banknifty_authority_id,
        ranking_policy_digest,
        proof.ordered_digest,
    ] {
        hasher.update(&value);
    }
    for value in [
        u64::from(source.rung_seconds),
        source.row_count,
        source.nifty_count,
        source.banknifty_count,
        source.admitted_count,
        source.authorized_count,
        source.eligible_count,
        proof.considered,
        proof.admitted,
        proof.refused,
        proof.unmeasured,
    ] {
        hasher.update(&value.to_le_bytes());
    }
    for value in source.terminal_matrix {
        hasher.update(&value.to_le_bytes());
    }
    hasher.update(&ordered_selected_digest(rows)?);
    Ok(hasher.finalize())
}

fn ordered_selected_digest(rows: &[SelectionV5RowRecord]) -> Result<[u8; 32], SelectionV5Refusal> {
    let mut hasher = Hasher::new();
    hasher.update(ORDERED_SELECTED_DOMAIN);
    for row in rows {
        let mut projection = *row;
        projection.selection_id = [0; 32];
        projection.row_id = [0; 32];
        hasher.update(&projection.semantic_bytes()?);
    }
    Ok(hasher.finalize())
}

fn ranked_winner(row: &SelectionV5RowRecord) -> RankedCandidate {
    RankedCandidate {
        candidate: Candidate {
            strategy_digest: StrategyDigest::new(row.strategy_digest),
            mask_words: row.mask_words,
            direction: row.direction,
            admitted: row.admission == AdmissionStatusV1::Admitted
                && row.terminal == SelectionV5Terminal::Authorized,
            metrics: row.metrics,
        },
        score: row.score,
    }
}

fn validate_strongest_first(rows: &[SelectionV5RowRecord]) -> Result<(), SelectionV5Refusal> {
    if rows.windows(2).any(|pair| {
        pair.first()
            .zip(pair.get(1))
            .is_some_and(|(stronger, weaker)| ranked_winner(stronger) < ranked_winner(weaker))
    }) {
        Err("Selection V5 winner rows are not strongest-first".to_owned())
    } else {
        Ok(())
    }
}

fn project_execution_source(
    source: &PopulationV5ExecutionV3SourceV1,
    execution: ExecutionV3StructuralReceipt,
    durable: &[ExecutionV3SuccessorDisposition],
) -> Result<(SelectionV5SourceSnapshot, Vec<SelectionV5SourceRow>), SelectionV5Refusal> {
    let population = source.receipt();
    if execution.population_id() != population.population_id()
        || execution.population_ordered_digest() != population.ordered_row_digest()
        || execution.disposition_count() != population.row_count()
        || usize_to_u64(source.rows().len(), "Population V5 successor row count")?
            != population.row_count()
        || usize_to_u64(durable.len(), "durable Execution V3 disposition count")?
            != execution.disposition_count()
    {
        return Err(
            "Selection V5 Population and Execution receipts do not describe one exact source"
                .to_owned(),
        );
    }

    let mut rows = Vec::new();
    rows.try_reserve_exact(durable.len())
        .map_err(|why| format!("Selection V5 source-row reserve failed: {why}"))?;
    let mut rung = None;
    let mut status_counts = [0_u64; 4];
    for (index, (population_row, durable_row)) in source.rows().iter().zip(durable).enumerate() {
        let row = project_execution_row(population_row, durable_row, execution)?;
        let expected_sequence = usize_to_u64(index, "source-row ordinal")?;
        if row.global_sequence != expected_sequence {
            return Err(format!(
                "Selection V5 exact Population/Execution join is reordered at row {index}"
            ));
        }
        match rung {
            Some(expected) if expected != durable_row.rung() => {
                return Err(
                    "Selection V5 exact Population/Execution join contains multiple rungs"
                        .to_owned(),
                );
            }
            None => rung = Some(durable_row.rung()),
            Some(_) => {}
        }
        let status_slot = status_counts
            .get_mut(admission_index(row.admission))
            .ok_or_else(|| "Selection V5 admission count index escaped schema".to_owned())?;
        increment(status_slot, "Population V5 admission status count")?;
        rows.push(row);
    }
    let rung_seconds = rung.ok_or_else(|| {
        "Selection V5 cannot derive a rung from an empty Population V5 source".to_owned()
    })?;
    if status_counts
        != [
            population.admitted_count(),
            population.rejected_count(),
            population.unmeasured_count(),
            population.refused_count(),
        ]
    {
        return Err(
            "Selection V5 rows do not reproduce the Population V5 admission counts".to_owned(),
        );
    }

    let mut terminal_matrix = [0_u64; TERMINAL_MATRIX_CELLS];
    for row in &rows {
        let slot = terminal_matrix
            .get_mut(matrix_index(row.family, row.admission, row.terminal))
            .ok_or_else(|| "Selection V5 terminal matrix index escaped schema".to_owned())?;
        increment(slot, "Execution V3 terminal matrix cell")?;
    }
    let (nifty_count, banknifty_count, admitted_count, authorized_count, eligible_count) =
        matrix_marginals(&terminal_matrix)?;
    if nifty_count != population.nifty_count() || banknifty_count != population.banknifty_count() {
        return Err(
            "Selection V5 rows do not reproduce the Population V5 family counts".to_owned(),
        );
    }
    let snapshot = SelectionV5SourceSnapshot {
        population_id: population.population_id(),
        population_ordered_digest: population.ordered_row_digest(),
        execution_completion_id: execution.completion_id(),
        ordered_disposition_digest: execution.ordered_disposition_digest(),
        nifty_authority_id: execution.nifty_authority_id(),
        banknifty_authority_id: execution.banknifty_authority_id(),
        rung_seconds,
        row_count: population.row_count(),
        nifty_count,
        banknifty_count,
        admitted_count,
        authorized_count,
        eligible_count,
        terminal_matrix,
    };
    snapshot.validate()?;
    validate_complete_source(&snapshot, &rows)?;
    Ok((snapshot, rows))
}

#[expect(
    clippy::too_many_lines,
    reason = "one projection, and its length IS its exhaustiveness: fourteen \
              match arms cover every variant of the source once, so a new \
              variant is a compile error here rather than a silently unprojected \
              row. Splitting the match would let one half stay exhaustive while \
              the other quietly grew a catch-all"
)]
fn project_execution_row(
    source: &PopulationV5ExecutionDispositionSourceV1,
    durable: &ExecutionV3SuccessorDisposition,
    receipt: ExecutionV3StructuralReceipt,
) -> Result<SelectionV5SourceRow, SelectionV5Refusal> {
    let population = source.population();
    let candidate = population.candidate().row();
    let live = source.disposition();
    let family = selection_family(population.family());
    let admission = selection_admission(population.status());
    let direction = selection_direction(candidate.direction());
    let terminal = if live.is_authorized() {
        SelectionV5Terminal::Authorized
    } else {
        SelectionV5Terminal::PolicyRefused
    };
    let coordinate = live.coordinate();
    let (ttp_arm, ttp_trail) = match coordinate.ttp {
        Some(ttp) => (
            Some(selection_index(ttp.arm, "TTP arm")?),
            Some(selection_index(ttp.trail, "TTP trail")?),
        ),
        None => (None, None),
    };
    let selected_exit_digest = live
        .selected()
        .map(runner::exit_grid_policy::SelectedExitV1::digest);
    let expected_side = match candidate.direction() {
        TradeDirectionV1::Long => runner::excursion::Side::Long,
        TradeDirectionV1::Short => runner::excursion::Side::Short,
    };
    let expected_execution_family = match family {
        SelectionV5Family::Nifty => ExecutionV3Family::Nifty,
        SelectionV5Family::BankNifty => ExecutionV3Family::BankNifty,
    };
    let expected_execution_direction = match direction {
        Direction::Long => ExecutionV3Direction::Long,
        Direction::Short => ExecutionV3Direction::Short,
    };
    let expected_execution_admission = match admission {
        AdmissionStatusV1::Admitted => ExecutionV3AdmissionStatus::Admitted,
        AdmissionStatusV1::Rejected => ExecutionV3AdmissionStatus::Rejected,
        AdmissionStatusV1::Unmeasured => ExecutionV3AdmissionStatus::Unmeasured,
        AdmissionStatusV1::Refused => ExecutionV3AdmissionStatus::Refused,
    };
    let expected_execution_terminal = match terminal {
        SelectionV5Terminal::Authorized => ExecutionV3Terminal::Authorized,
        SelectionV5Terminal::PolicyRefused => ExecutionV3Terminal::PolicyRefused,
    };

    if population.population_id() != receipt.population_id()
        || durable.population_id() != population.population_id()
        || durable.population_row_id() != population.row_id()
        || durable.candidate_semantic_id() != candidate.candidate_semantic_digest()
        || durable.candidate_base_row_id() != population.candidate().base_candidate_row_digest()
        || durable.admission_decision_id() != population.admission().decision_id()
        || durable.finalization_row_id() != population.finalization_row_id()
        || durable.finalization_completion_id() != population.finalization_completion_id()
        || durable.global_sequence() != population.global_sequence()
        || durable.family_sequence() != population.family_sequence()
        || durable.cell_ordinal() != candidate.cell_ordinal()
        || durable.support_hits() != candidate.support_hits()
        || durable.family() != expected_execution_family
        || durable.direction() != expected_execution_direction
        || durable.admission_status() != expected_execution_admission
        || durable.terminal() != expected_execution_terminal
        || durable.rung() != candidate.rung_seconds()
        || durable.horizon_bars() != candidate.horizon_bars()
        || durable.execution_run_id() != candidate.execution_run_id()
        || durable.evaluated_grid_digest() != candidate.evaluated_grid_digest()
        || durable.resolution_digest() != live.resolution_digest()
        || durable.column_digest() != live.column_digest()
        || durable.context_digest() != live.context_digest()
        || durable.runner_disposition_digest() != live.disposition_digest()
        || durable.selected_exit_digest() != selected_exit_digest
        || durable.refusal_bits() != live.refusal_bits().bits()
        || durable.stop_index() != optional_selection_index(coordinate.stop, "stop")?
        || durable.target_index() != optional_selection_index(coordinate.target, "target")?
        || durable.tsl_index() != optional_selection_index(coordinate.tsl, "TSL")?
        || durable.ttp_arm_index() != ttp_arm
        || durable.ttp_trail_index() != ttp_trail
        || live.run_id().bytes() != candidate.execution_run_id()
        || live.mask().words() != candidate.mask_words()
        || live.horizon().as_bars() != candidate.horizon_bars()
        || live.side() != expected_side
        || selection_family_from_candidate(candidate.family()) != family
    {
        return Err(format!(
            "Selection V5 exact Population/Execution join differs at global row {}",
            population.global_sequence()
        ));
    }

    let cell = candidate
        .facts()
        .to_cell(candidate.exit())
        .map_err(|why| format!("Selection V5 Candidate metrics refused: {why}"))?;
    let assurance_ppm = if cell.trades == 0 {
        0
    } else {
        crate::institutional_evidence::wilson_lower_ppm(cell.wins, cell.trades, cell.assurance_bp())
            .map_err(|why| format!("Selection V5 Wilson projection refused: {why}"))?
    };
    let top_metrics = crate::population_admission_writer::metrics_from_cell(&cell, assurance_ppm)
        .map_err(|why| format!("Selection V5 ranking metrics refused: {why}"))?;
    validate_authenticated_metrics(
        population.global_sequence(),
        population.status(),
        candidate.support_hits(),
        top_metrics,
        &population.admission().comparison_values(),
    )?;

    Ok(SelectionV5SourceRow {
        population_id: receipt.population_id(),
        population_ordered_digest: receipt.population_ordered_digest(),
        execution_completion_id: receipt.completion_id(),
        disposition_id: durable.disposition_id(),
        global_sequence: durable.global_sequence(),
        family_sequence: durable.family_sequence(),
        family,
        strategy_digest: candidate.candidate_semantic_digest(),
        mask_words: candidate.mask_words(),
        direction,
        admission,
        terminal,
        metrics: ranking_metrics(top_metrics),
    })
}

fn validate_authenticated_metrics(
    sequence: u64,
    status: AdmissionV3Status,
    support_hits: u64,
    metrics: TopMetricsV1,
    values: &runner::admission::AdmissionEvidenceValuesV1,
) -> Result<(), SelectionV5Refusal> {
    let trades = metrics
        .winning_trades
        .checked_add(metrics.losing_trades)
        .ok_or_else(|| format!("Selection V5 row {sequence} trade count overflowed u64"))?;
    for (name, observed, exact) in [
        ("support_hits", values.support_hits, support_hits),
        ("trades", values.trades, trades),
        ("drawdown_paisa", values.drawdown_paisa, metrics.drawdown),
        (
            "worst_trade_loss_paisa",
            values.worst_trade_loss_paisa,
            metrics.worst_loss,
        ),
        (
            "losing_trade_rate_ppm",
            values.losing_trade_rate_ppm,
            metrics.losing_rate_ppm,
        ),
        ("losing_trades", values.losing_trades, metrics.losing_trades),
        (
            "winning_trades",
            values.winning_trades,
            metrics.winning_trades,
        ),
        ("win_rate_ppm", values.win_rate_ppm, metrics.win_rate_ppm),
        (
            "wilson_win_rate_ppm",
            values.wilson_win_rate_ppm,
            metrics.assurance_ppm,
        ),
        (
            "average_win_paisa",
            values.average_win_paisa,
            metrics.average_win,
        ),
        (
            "average_loss_paisa",
            values.average_loss_paisa,
            metrics.average_loss,
        ),
    ] {
        require_observed_u64(sequence, status, name, observed, exact)?;
    }
    require_observed_i64(
        sequence,
        status,
        "pessimistic_profit_paisa",
        values.pessimistic_profit_paisa,
        metrics.pessimistic_profit,
    )?;
    match metrics.reward_to_risk_ppm {
        Some(exact) => require_observed_u64(
            sequence,
            status,
            "worst_reward_risk_ppm",
            values.worst_reward_risk_ppm,
            exact,
        )?,
        None if matches!(values.worst_reward_risk_ppm, ObservedU64V1::Measured(_)) => {
            return Err(format!(
                "Selection V5 row {sequence} has measured reward/risk but an undefined direct denominator"
            ));
        }
        None => {}
    }
    Ok(())
}

fn require_observed_u64(
    sequence: u64,
    status: AdmissionV3Status,
    name: &str,
    observed: ObservedU64V1,
    exact: u64,
) -> Result<(), SelectionV5Refusal> {
    match observed {
        ObservedU64V1::Measured(value) if value == exact => Ok(()),
        ObservedU64V1::Measured(value) => Err(format!(
            "Selection V5 row {sequence} authenticated {name} {value} differs from direct Candidate metric {exact}"
        )),
        ObservedU64V1::Unmeasured | ObservedU64V1::Refused
            if status == AdmissionV3Status::Admitted =>
        {
            Err(format!(
                "Selection V5 admitted row {sequence} has no authenticated {name}"
            ))
        }
        ObservedU64V1::Unmeasured | ObservedU64V1::Refused => Ok(()),
    }
}

fn require_observed_i64(
    sequence: u64,
    status: AdmissionV3Status,
    name: &str,
    observed: ObservedI64V1,
    exact: i64,
) -> Result<(), SelectionV5Refusal> {
    match observed {
        ObservedI64V1::Measured(value) if value == exact => Ok(()),
        ObservedI64V1::Measured(value) => Err(format!(
            "Selection V5 row {sequence} authenticated {name} {value} differs from direct Candidate metric {exact}"
        )),
        ObservedI64V1::Unmeasured | ObservedI64V1::Refused
            if status == AdmissionV3Status::Admitted =>
        {
            Err(format!(
                "Selection V5 admitted row {sequence} has no authenticated {name}"
            ))
        }
        ObservedI64V1::Unmeasured | ObservedI64V1::Refused => Ok(()),
    }
}

const fn ranking_metrics(value: TopMetricsV1) -> Metrics {
    Metrics {
        drawdown: value.drawdown,
        worst_loss: value.worst_loss,
        losing_rate_ppm: value.losing_rate_ppm,
        losing_trades: value.losing_trades,
        loss_ratio_ppm: value.loss_ratio_ppm,
        pessimistic_profit: value.pessimistic_profit,
        winning_trades: value.winning_trades,
        win_rate_ppm: value.win_rate_ppm,
        reward_to_risk_ppm: value.reward_to_risk_ppm,
        average_win: value.average_win,
        average_loss: value.average_loss,
        assurance_ppm: value.assurance_ppm,
    }
}

const fn selection_family(value: AdmissionV3Family) -> SelectionV5Family {
    match value {
        AdmissionV3Family::Nifty => SelectionV5Family::Nifty,
        AdmissionV3Family::BankNifty => SelectionV5Family::BankNifty,
    }
}

const fn selection_family_from_candidate(value: InstrumentFamilyV1) -> SelectionV5Family {
    match value {
        InstrumentFamilyV1::Nifty => SelectionV5Family::Nifty,
        InstrumentFamilyV1::BankNifty => SelectionV5Family::BankNifty,
    }
}

const fn selection_admission(value: AdmissionV3Status) -> AdmissionStatusV1 {
    match value {
        AdmissionV3Status::Admitted => AdmissionStatusV1::Admitted,
        AdmissionV3Status::Rejected => AdmissionStatusV1::Rejected,
        AdmissionV3Status::Unmeasured => AdmissionStatusV1::Unmeasured,
        AdmissionV3Status::Refused => AdmissionStatusV1::Refused,
    }
}

const fn selection_direction(value: TradeDirectionV1) -> Direction {
    match value {
        TradeDirectionV1::Long => Direction::Long,
        TradeDirectionV1::Short => Direction::Short,
    }
}

fn optional_selection_index(
    value: Option<usize>,
    name: &str,
) -> Result<Option<u32>, SelectionV5Refusal> {
    value.map(|index| selection_index(index, name)).transpose()
}

fn selection_index(value: usize, name: &str) -> Result<u32, SelectionV5Refusal> {
    u32::try_from(value).map_err(|_| format!("Selection V5 {name} index does not fit u32"))
}

fn validate_complete_source(
    source: &SelectionV5SourceSnapshot,
    rows: &[SelectionV5SourceRow],
) -> Result<(), SelectionV5Refusal> {
    source.validate()?;
    if usize_to_u64(rows.len(), "source disposition count")? != source.row_count {
        return Err(format!(
            "Selection V5 received {} dispositions for a {}-row Execution V3 Completion",
            rows.len(),
            source.row_count
        ));
    }
    let capacity = rows.len();
    let mut disposition_ids = HashSet::new();
    let mut strategy_ids = HashSet::new();
    disposition_ids
        .try_reserve(capacity)
        .map_err(|why| format!("Selection V5 disposition-identity reserve failed: {why}"))?;
    strategy_ids
        .try_reserve(capacity)
        .map_err(|why| format!("Selection V5 strategy-identity reserve failed: {why}"))?;
    let mut family_sequences = [0_u64; 2];
    let mut matrix = [0_u64; TERMINAL_MATRIX_CELLS];
    let mut seen_banknifty = false;
    for (index, row) in rows.iter().copied().enumerate() {
        row.validate_against(source)?;
        let expected_global = usize_to_u64(index, "global disposition sequence")?;
        let family_index = family_index(row.family);
        let expected_family_sequence = family_sequences
            .get(family_index)
            .copied()
            .ok_or_else(|| "Selection V5 family sequence index is absent".to_owned())?;
        if row.global_sequence != expected_global || row.family_sequence != expected_family_sequence
        {
            return Err(format!(
                "Selection V5 source order is noncontiguous at global row {index}"
            ));
        }
        match row.family {
            SelectionV5Family::Nifty if seen_banknifty => {
                return Err(
                    "Selection V5 source order returned to NIFTY after BANKNIFTY".to_owned(),
                );
            }
            SelectionV5Family::Nifty => {}
            SelectionV5Family::BankNifty => seen_banknifty = true,
        }
        increment(
            family_sequences
                .get_mut(family_index)
                .ok_or_else(|| "Selection V5 mutable family sequence is absent".to_owned())?,
            "family disposition sequence",
        )?;
        increment(
            matrix
                .get_mut(matrix_index(row.family, row.admission, row.terminal))
                .ok_or_else(|| "Selection V5 mutable terminal matrix cell is absent".to_owned())?,
            "terminal matrix cell",
        )?;
        if !disposition_ids.insert(row.disposition_id) {
            return Err(format!(
                "Selection V5 disposition {} occurs more than once",
                hex32(row.disposition_id)
            ));
        }
        if !strategy_ids.insert(row.strategy_digest) {
            return Err(format!(
                "Selection V5 strategy alias {} occurs more than once",
                hex32(row.strategy_digest)
            ));
        }
    }
    if family_sequences != [source.nifty_count, source.banknifty_count]
        || matrix != source.terminal_matrix
    {
        return Err(
            "Selection V5 rows do not reproduce the complete family/matrix receipt".to_owned(),
        );
    }
    let (_, _, admitted, authorized, eligible) = matrix_marginals(&matrix)?;
    if admitted != source.admitted_count
        || authorized != source.authorized_count
        || eligible != source.eligible_count
    {
        return Err("Selection V5 source rows do not reproduce completion marginals".to_owned());
    }
    Ok(())
}

fn matrix_marginals(
    matrix: &[u64; TERMINAL_MATRIX_CELLS],
) -> Result<(u64, u64, u64, u64, u64), SelectionV5Refusal> {
    let nifty = checked_sum(
        matrix
            .get(..8)
            .ok_or_else(|| "Selection V5 NIFTY matrix range is absent".to_owned())?
            .iter()
            .copied(),
        "NIFTY matrix",
    )?;
    let banknifty = checked_sum(
        matrix
            .get(8..)
            .ok_or_else(|| "Selection V5 BANKNIFTY matrix range is absent".to_owned())?
            .iter()
            .copied(),
        "BANKNIFTY matrix",
    )?;
    let admitted = checked_sum(
        [
            matrix.first().copied().unwrap_or(0),
            *matrix.get(1).unwrap_or(&0),
            *matrix.get(8).unwrap_or(&0),
            *matrix.get(9).unwrap_or(&0),
        ],
        "admitted matrix marginal",
    )?;
    let authorized = checked_sum(
        [
            matrix.first().copied().unwrap_or(0),
            *matrix.get(2).unwrap_or(&0),
            *matrix.get(4).unwrap_or(&0),
            *matrix.get(6).unwrap_or(&0),
            *matrix.get(8).unwrap_or(&0),
            *matrix.get(10).unwrap_or(&0),
            *matrix.get(12).unwrap_or(&0),
            *matrix.get(14).unwrap_or(&0),
        ],
        "authorized matrix marginal",
    )?;
    let eligible = checked_sum(
        [
            matrix.first().copied().unwrap_or(0),
            *matrix.get(8).unwrap_or(&0),
        ],
        "eligible matrix marginal",
    )?;
    Ok((nifty, banknifty, admitted, authorized, eligible))
}

fn matrix_index(
    family: SelectionV5Family,
    admission: AdmissionStatusV1,
    terminal: SelectionV5Terminal,
) -> usize {
    family_index(family) * 8 + admission_index(admission) * 2 + terminal_index(terminal)
}

const fn family_index(family: SelectionV5Family) -> usize {
    match family {
        SelectionV5Family::Nifty => 0,
        SelectionV5Family::BankNifty => 1,
    }
}

const fn admission_index(admission: AdmissionStatusV1) -> usize {
    match admission {
        AdmissionStatusV1::Admitted => 0,
        AdmissionStatusV1::Rejected => 1,
        AdmissionStatusV1::Unmeasured => 2,
        AdmissionStatusV1::Refused => 3,
    }
}

const fn terminal_index(terminal: SelectionV5Terminal) -> usize {
    match terminal {
        SelectionV5Terminal::Authorized => 0,
        SelectionV5Terminal::PolicyRefused => 1,
    }
}

/// Structural result from a bounded fresh reopen.  It is not source authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SelectionV5StructuralReceipt {
    block_sequence: u64,
    first_row_record: u64,
    selected_count: u64,
    selection_id: [u8; 32],
    completion_id: [u8; 32],
    population_id: [u8; 32],
    execution_completion_id: [u8; 32],
    rung_seconds: u32,
    top_ten_count: u32,
}

impl SelectionV5StructuralReceipt {
    fn from_completion(value: &SelectionV5CompletionRecord) -> Self {
        Self {
            block_sequence: value.block_sequence,
            first_row_record: value.first_row_record,
            selected_count: value.selected_count,
            selection_id: value.selection_id,
            completion_id: value.completion_id,
            population_id: value.population_id,
            execution_completion_id: value.execution_completion_id,
            rung_seconds: value.rung_seconds,
            top_ten_count: value.top_ten_count,
        }
    }

    #[must_use]
    pub(crate) const fn selection_id(self) -> [u8; 32] {
        self.selection_id
    }

    #[must_use]
    pub(crate) const fn completion_id(self) -> [u8; 32] {
        self.completion_id
    }

    #[must_use]
    pub(crate) const fn selected_count(self) -> u64 {
        self.selected_count
    }

    #[must_use]
    pub(crate) const fn top_ten_count(self) -> u32 {
        self.top_ten_count
    }

    #[must_use]
    pub(crate) const fn population_id(self) -> [u8; 32] {
        self.population_id
    }

    #[must_use]
    pub(crate) const fn execution_completion_id(self) -> [u8; 32] {
        self.execution_completion_id
    }

    #[must_use]
    pub(crate) const fn rung_seconds(self) -> u32 {
        self.rung_seconds
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectionV5StructuralCommit {
    Written(SelectionV5StructuralReceipt),
    Reused(SelectionV5StructuralReceipt),
}

impl SelectionV5StructuralCommit {
    const fn was_written(self) -> bool {
        matches!(self, Self::Written(_))
    }

    const fn receipt(self) -> SelectionV5StructuralReceipt {
        match self {
            Self::Written(value) | Self::Reused(value) => value,
        }
    }
}

#[derive(Clone, Debug)]
struct TrailingSelectionV5 {
    first_row_record: u64,
    rows: Vec<SelectionV5RowRecord>,
}

struct SelectionV5Ledger {
    root: PathBuf,
    root_file: File,
    root_identity: PlatformIdentity,
    lock_file: File,
    lock_path: PathBuf,
    lock_identity: PlatformIdentity,
    rows: BoundedFixedFile,
    completions: BoundedFixedFile,
    bounds: SelectionV5Bounds,
    writable: bool,
    row_records: u64,
    completion_records: u64,
    receipts: HashMap<[u8; 32], SelectionV5StructuralReceipt>,
    latest: HashMap<u32, [u8; 32]>,
    trailing: Option<TrailingSelectionV5>,
}

impl SelectionV5Ledger {
    fn open_read(root: &Path, bounds: SelectionV5Bounds) -> Result<Self, SelectionV5Refusal> {
        Self::open(root, bounds, false)
    }

    fn open_write(root: &Path, bounds: SelectionV5Bounds) -> Result<Self, SelectionV5Refusal> {
        Self::open(root, bounds, true)
    }

    fn open(
        root: &Path,
        bounds: SelectionV5Bounds,
        writable: bool,
    ) -> Result<Self, SelectionV5Refusal> {
        let (root, root_file, root_identity) = open_root_directory(root)?;
        let lock_path = root.join(LOCK_FILE);
        let row_path = root.join(ROW_FILE);
        let completion_path = root.join(COMPLETION_FILE);
        let mut created = false;
        let lock_file = open_child(&lock_path, writable, &mut created)?;
        require_regular_unique_file(&lock_file, &lock_path)?;
        if lock_file
            .metadata()
            .map_err(|why| format!("cannot stat Selection V5 lock: {why}"))?
            .len()
            != LOCK_MAX_BYTES
        {
            return Err("Selection V5 lock file is not empty".to_owned());
        }
        let rows = BoundedFixedFile::open(
            row_path,
            writable,
            SELECTION_V5_ROW_BYTES,
            bounds.row_records,
            bounds.row_bytes,
            "winner rows",
            &mut created,
        )?;
        let completions = BoundedFixedFile::open(
            completion_path,
            writable,
            SELECTION_V5_COMPLETION_BYTES,
            bounds.completion_records,
            bounds.completion_bytes,
            "Completions",
            &mut created,
        )?;
        if created {
            sync_directory(&root_file, &root)?;
        }
        if named_identity(&root)? != root_identity {
            return Err("Selection V5 root changed while child files opened".to_owned());
        }
        let lock_identity = named_identity(&lock_path)?;
        let mut ledger = Self {
            root,
            root_file,
            root_identity,
            lock_file,
            lock_path,
            lock_identity,
            rows,
            completions,
            bounds,
            writable,
            row_records: 0,
            completion_records: 0,
            receipts: HashMap::new(),
            latest: HashMap::new(),
            trailing: None,
        };
        if writable {
            ledger
                .lock_file
                .lock()
                .map_err(|why| format!("cannot lock Selection V5 for open: {why}"))?;
        } else {
            ledger
                .lock_file
                .lock_shared()
                .map_err(|why| format!("cannot shared-lock Selection V5 for open: {why}"))?;
        }
        let scanned = ledger.scan();
        let released = ledger
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot unlock Selection V5 after open: {why}"));
        combine_lock_result(scanned, released)?;
        Ok(ledger)
    }

    fn scan(&mut self) -> Result<(), SelectionV5Refusal> {
        self.rows.refresh()?;
        self.completions.refresh()?;
        let row_records = self.rows.record_count()?;
        let completion_records = self.completions.record_count()?;
        let completion_capacity = usize::try_from(completion_records)
            .map_err(|_| "Selection V5 Completion count does not fit usize".to_owned())?;
        let mut receipts = HashMap::new();
        let mut latest = HashMap::new();
        receipts
            .try_reserve(completion_capacity)
            .map_err(|why| format!("Selection V5 receipt index reserve failed: {why}"))?;
        latest
            .try_reserve(completion_capacity)
            .map_err(|why| format!("Selection V5 latest index reserve failed: {why}"))?;
        let mut covered_rows = 0_u64;
        for block_sequence in 0..completion_records {
            let completion = SelectionV5CompletionRecord::decode(&read_fixed_at(
                &mut self.completions.file,
                block_sequence,
                SELECTION_V5_COMPLETION_BYTES,
                "Completion",
            )?)?;
            if completion.block_sequence != block_sequence
                || completion.first_row_record != covered_rows
            {
                return Err(format!(
                    "Selection V5 Completion {block_sequence} is not contiguous/canonical"
                ));
            }
            let end = checked_end(
                completion.first_row_record,
                completion.selected_count,
                "Completion row range",
            )?;
            if end > row_records {
                return Err(format!(
                    "Selection V5 Completion {block_sequence} is torn at row {end}; file has {row_records}"
                ));
            }
            let rows = self.read_rows(completion.first_row_record, completion.selected_count)?;
            validate_complete_block(&rows, &completion)?;
            let receipt = SelectionV5StructuralReceipt::from_completion(&completion);
            if receipts.insert(receipt.selection_id, receipt).is_some() {
                return Err(format!(
                    "Selection V5 identity {} appears more than once",
                    hex32(receipt.selection_id)
                ));
            }
            latest.insert(receipt.rung_seconds, receipt.selection_id);
            covered_rows = end;
        }
        let trailing_count = row_records
            .checked_sub(covered_rows)
            .ok_or_else(|| "Selection V5 covered-row count exceeds row file".to_owned())?;
        let trailing = if trailing_count == 0 {
            None
        } else {
            if trailing_count > REQUESTED_TOP_U64 {
                return Err(format!(
                    "Selection V5 orphan tail has {trailing_count} rows, above fixed Top-25"
                ));
            }
            let rows = self.read_rows(covered_rows, trailing_count)?;
            validate_trailing_prefix(&rows)?;
            if rows
                .first()
                .is_some_and(|row| receipts.contains_key(&row.selection_id))
            {
                return Err("Selection V5 orphan tail duplicates a committed selection".to_owned());
            }
            Some(TrailingSelectionV5 {
                first_row_record: covered_rows,
                rows,
            })
        };
        self.row_records = row_records;
        self.completion_records = completion_records;
        self.receipts = receipts;
        self.latest = latest;
        self.trailing = trailing;
        self.require_unchanged()
    }

    fn append(
        &mut self,
        prepared: &PreparedSelectionV5,
    ) -> Result<SelectionV5StructuralCommit, SelectionV5Refusal> {
        if !self.writable {
            return Err("Selection V5 ledger was opened read-only".to_owned());
        }
        self.lock_file
            .lock()
            .map_err(|why| format!("cannot take Selection V5 append lock: {why}"))?;
        let result = self.append_locked(prepared);
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Selection V5 append lock: {why}"));
        combine_lock_result(result, released)
    }

    fn append_locked(
        &mut self,
        prepared: &PreparedSelectionV5,
    ) -> Result<SelectionV5StructuralCommit, SelectionV5Refusal> {
        self.require_unchanged()?;
        prepared.validate()?;
        if let Some(existing) = self.receipts.get(&prepared.selection_id).copied() {
            return self.reuse_existing(prepared, existing);
        }
        let trailing = self.trailing.clone().unwrap_or(TrailingSelectionV5 {
            first_row_record: self.row_records,
            rows: Vec::new(),
        });
        Self::require_exact_prefix(prepared, &trailing)?;
        self.require_append_bound(prepared, &trailing)?;
        self.append_row_suffix(prepared, trailing.rows.len())?;
        self.rows
            .file
            .sync_data()
            .map_err(|why| format!("cannot sync Selection V5 winner rows: {why}"))?;
        self.rows.refresh()?;
        self.row_records = self.rows.record_count()?;
        self.require_unchanged()?;
        self.append_completion(prepared, trailing.first_row_record)?;
        self.scan()?;
        let receipt = self
            .receipts
            .get(&prepared.selection_id)
            .copied()
            .ok_or_else(|| "Selection V5 appended selection was not indexed".to_owned())?;
        Ok(SelectionV5StructuralCommit::Written(receipt))
    }

    fn reuse_existing(
        &mut self,
        prepared: &PreparedSelectionV5,
        existing: SelectionV5StructuralReceipt,
    ) -> Result<SelectionV5StructuralCommit, SelectionV5Refusal> {
        let rows = self.read_rows(existing.first_row_record, existing.selected_count)?;
        if rows != prepared.rows {
            return Err(format!(
                "Selection V5 identity {} exists with different exact winner rows",
                hex32(existing.selection_id)
            ));
        }
        let observed = SelectionV5CompletionRecord::decode(&read_fixed_at(
            &mut self.completions.file,
            existing.block_sequence,
            SELECTION_V5_COMPLETION_BYTES,
            "Completion",
        )?)?;
        let expected =
            prepared.expected_completion(existing.block_sequence, existing.first_row_record)?;
        if observed != expected {
            return Err(format!(
                "Selection V5 identity {} exists with a different Completion",
                hex32(existing.selection_id)
            ));
        }
        self.rows
            .file
            .sync_data()
            .and_then(|()| self.completions.file.sync_data())
            .map_err(|why| format!("cannot sync reused Selection V5 authority: {why}"))?;
        sync_directory(&self.root_file, &self.root)?;
        self.require_unchanged()?;
        Ok(SelectionV5StructuralCommit::Reused(existing))
    }

    fn require_exact_prefix(
        prepared: &PreparedSelectionV5,
        trailing: &TrailingSelectionV5,
    ) -> Result<(), SelectionV5Refusal> {
        if trailing.rows.len() > prepared.rows.len()
            || prepared.rows.get(..trailing.rows.len()) != Some(trailing.rows.as_slice())
        {
            return Err(
                "Selection V5 orphan tail is not the exact canonical retry prefix".to_owned(),
            );
        }
        Ok(())
    }

    fn require_append_bound(
        &self,
        prepared: &PreparedSelectionV5,
        trailing: &TrailingSelectionV5,
    ) -> Result<(), SelectionV5Refusal> {
        let target_rows = checked_end(
            trailing.first_row_record,
            usize_to_u64(prepared.rows.len(), "append selected count")?,
            "append row range",
        )?;
        let target_completions = self
            .completion_records
            .checked_add(1)
            .ok_or_else(|| "Selection V5 Completion count overflowed".to_owned())?;
        if target_rows > self.bounds.row_records {
            return Err(format!(
                "Selection V5 append reaches {target_rows} rows above bound {}",
                self.bounds.row_records
            ));
        }
        if target_completions > self.bounds.completion_records {
            return Err(format!(
                "Selection V5 append reaches {target_completions} Completions above bound {}",
                self.bounds.completion_records
            ));
        }
        Ok(())
    }

    fn append_row_suffix(
        &mut self,
        prepared: &PreparedSelectionV5,
        start: usize,
    ) -> Result<(), SelectionV5Refusal> {
        for row in prepared
            .rows
            .get(start..)
            .ok_or_else(|| format!("Selection V5 winner suffix start {start} is outside Top-25"))?
        {
            append_raw(&mut self.rows.file, &row.encode()?)?;
        }
        Ok(())
    }

    fn append_completion(
        &mut self,
        prepared: &PreparedSelectionV5,
        first_row_record: u64,
    ) -> Result<(), SelectionV5Refusal> {
        let completion = prepared.expected_completion(self.completion_records, first_row_record)?;
        append_raw(&mut self.completions.file, &completion.encode()?)?;
        self.completions
            .file
            .sync_data()
            .map_err(|why| format!("cannot sync Selection V5 Completion: {why}"))?;
        sync_directory(&self.root_file, &self.root)?;
        self.completions.refresh()?;
        self.completion_records = self.completions.record_count()?;
        self.require_unchanged()
    }

    fn read_rows(
        &mut self,
        first: u64,
        count: u64,
    ) -> Result<Vec<SelectionV5RowRecord>, SelectionV5Refusal> {
        let mut rows = bounded_vec(count, self.bounds.row_records, "winner rows")?;
        for offset in 0..count {
            rows.push(SelectionV5RowRecord::decode(&read_fixed_at(
                &mut self.rows.file,
                checked_end(first, offset, "winner row read offset")?,
                SELECTION_V5_ROW_BYTES,
                "winner row",
            )?)?);
        }
        Ok(rows)
    }

    fn structural_receipt(
        &self,
        selection_id: &[u8; 32],
    ) -> Result<Option<SelectionV5StructuralReceipt>, SelectionV5Refusal> {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take Selection V5 lookup lock: {why}"))?;
        let result = self
            .require_unchanged()
            .map(|()| self.receipts.get(selection_id).copied());
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Selection V5 lookup lock: {why}"));
        combine_lock_result(result, released)
    }

    fn selected_rows(
        &mut self,
        receipt: SelectionV5StructuralReceipt,
    ) -> Result<Vec<SelectionV5RowRecord>, SelectionV5Refusal> {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take Selection V5 row-read lock: {why}"))?;
        let result = (|| {
            self.require_unchanged()?;
            let rows = self.read_rows(receipt.first_row_record, receipt.selected_count)?;
            let completion = SelectionV5CompletionRecord::decode(&read_fixed_at(
                &mut self.completions.file,
                receipt.block_sequence,
                SELECTION_V5_COMPLETION_BYTES,
                "Completion",
            )?)?;
            if SelectionV5StructuralReceipt::from_completion(&completion) != receipt {
                return Err("Selection V5 lookup receipt changed after open".to_owned());
            }
            validate_complete_block(&rows, &completion)?;
            self.require_unchanged()?;
            Ok(rows)
        })();
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Selection V5 row-read lock: {why}"));
        combine_lock_result(result, released)
    }

    fn require_exact_prepared(
        &mut self,
        receipt: SelectionV5StructuralReceipt,
        prepared: &PreparedSelectionV5,
    ) -> Result<(), SelectionV5Refusal> {
        let rows = self.selected_rows(receipt)?;
        let completion = SelectionV5CompletionRecord::decode(&read_fixed_at(
            &mut self.completions.file,
            receipt.block_sequence,
            SELECTION_V5_COMPLETION_BYTES,
            "Completion",
        )?)?;
        if rows != prepared.rows
            || completion
                != prepared.expected_completion(receipt.block_sequence, receipt.first_row_record)?
        {
            return Err("Selection V5 fresh reopen differs from prepared bytes".to_owned());
        }
        self.require_unchanged()
    }

    fn require_unchanged(&self) -> Result<(), SelectionV5Refusal> {
        if named_identity(&self.root)? != self.root_identity
            || named_identity(&self.lock_path)? != self.lock_identity
            || self
                .lock_file
                .metadata()
                .map_err(|why| format!("cannot stat held Selection V5 lock: {why}"))?
                .len()
                != LOCK_MAX_BYTES
        {
            return Err("Selection V5 retained root or lock path changed".to_owned());
        }
        self.rows.require_unchanged()?;
        self.completions.require_unchanged()?;
        Ok(())
    }
}

/// Freshly reopened V5 persistence authority. The production wrapper below
/// retains `CommittedStoredExecutionV3` beside this ledger.
struct SelectionV5Authority {
    receipt: SelectionV5StructuralReceipt,
    ledger: SelectionV5Ledger,
}

impl SelectionV5Authority {
    const fn structural_receipt(&self) -> SelectionV5StructuralReceipt {
        self.receipt
    }

    fn top_twenty_five(&mut self) -> Result<Vec<SelectionV5RowRecord>, SelectionV5Refusal> {
        self.ledger.selected_rows(self.receipt)
    }

    /// The exact Top-10 prefix of this rung's Top-25.
    ///
    /// # Why it has no caller yet
    ///
    /// `ledger-all` renders the prefix by filtering rank off the all-rung
    /// successor visit, which yields all two hundred rows at once and never
    /// holds a single rung's ledger. This is the one-rung door to the same
    /// prefix, and a per-rung reader — a `/selection.json` route, a single-rung
    /// verb — is what would use it.
    /// `cfg_attr(not(test), ...)` and not a bare `expect`: the tests DO call
    /// this, so under `--all-targets` the lib compiles twice and a bare
    /// annotation is fulfilled in one pass and unfulfilled in the other. That
    /// is the pattern every other pending item in this crate already uses.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the all-rung visitor renders the prefix; no single-rung reader exists yet"
        )
    )]
    fn top_ten(&mut self) -> Result<Vec<SelectionV5RowRecord>, SelectionV5Refusal> {
        let rows = self.top_twenty_five()?;
        Ok(rows.into_iter().take(TOP_TEN).collect())
    }
}

enum SelectionV5Commit {
    Written(SelectionV5Authority),
    Reused(SelectionV5Authority),
}

impl SelectionV5Commit {
    const fn was_written(&self) -> bool {
        matches!(self, Self::Written(_))
    }

    const fn authority(&self) -> &SelectionV5Authority {
        match self {
            Self::Written(value) | Self::Reused(value) => value,
        }
    }

    fn authority_mut(&mut self) -> &mut SelectionV5Authority {
        match self {
            Self::Written(value) | Self::Reused(value) => value,
        }
    }
}

fn persist_prepared(
    root: &Path,
    bounds: SelectionV5Bounds,
    prepared: &PreparedSelectionV5,
) -> Result<SelectionV5Commit, SelectionV5Refusal> {
    let structural = {
        let mut writer = SelectionV5Ledger::open_write(root, bounds)?;
        writer.append(prepared)?
    };
    let writer_receipt = structural.receipt();
    let mut reader = SelectionV5Ledger::open_read(root, bounds)?;
    let reopened = reader
        .structural_receipt(&prepared.selection_id)?
        .ok_or_else(|| "Selection V5 fresh reopen did not find committed identity".to_owned())?;
    if reopened != writer_receipt {
        return Err("Selection V5 fresh reopen receipt differs from writer receipt".to_owned());
    }
    reader.require_exact_prepared(reopened, prepared)?;
    let authority = SelectionV5Authority {
        receipt: reopened,
        ledger: reader,
    };
    if structural.was_written() {
        Ok(SelectionV5Commit::Written(authority))
    } else {
        Ok(SelectionV5Commit::Reused(authority))
    }
}

/// Nonconstructible Selection V5 capability retaining the exact Execution V3
/// source from which its ranking facts and durable terminal dispositions were
/// derived.
pub(crate) struct CommittedStoredSelectionV5 {
    source: CommittedStoredExecutionV3,
    policy: RankingPolicyV1,
    selection: SelectionV5Commit,
}

impl CommittedStoredSelectionV5 {
    #[must_use]
    pub(crate) const fn was_written(&self) -> bool {
        self.selection.was_written()
    }

    #[must_use]
    pub(crate) const fn structural_receipt(&self) -> SelectionV5StructuralReceipt {
        self.selection.authority().structural_receipt()
    }

    #[must_use]
    pub(crate) const fn bounds(&self) -> SelectionV5Bounds {
        self.selection.authority().ledger.bounds
    }

    #[must_use]
    pub(crate) const fn rung_seconds(&self) -> u32 {
        self.selection
            .authority()
            .structural_receipt()
            .rung_seconds()
    }

    #[must_use]
    pub(crate) fn ranking_policy_digest(&self) -> [u8; 32] {
        self.policy.digest()
    }

    /// Reauthenticates the retained Population+Execution source and returns
    /// the freshly reopened authoritative Top-25 bytes.
    pub(crate) fn top_twenty_five(
        &mut self,
    ) -> Result<Vec<SelectionV5RowRecord>, SelectionV5Refusal> {
        let prepared_before =
            PreparedSelectionV5::from_committed_execution(&mut self.source, self.policy)?;
        let receipt = self.selection.authority().structural_receipt();
        if receipt.selection_id() != prepared_before.selection_id {
            return Err(
                "Selection V5 retained Execution source now derives a different selection"
                    .to_owned(),
            );
        }
        self.selection
            .authority_mut()
            .ledger
            .require_exact_prepared(receipt, &prepared_before)?;
        let rows = self.selection.authority_mut().top_twenty_five()?;
        let prepared_after =
            PreparedSelectionV5::from_committed_execution(&mut self.source, self.policy)?;
        if prepared_after != prepared_before {
            return Err(
                "Selection V5 retained Execution source changed during winner read".to_owned(),
            );
        }
        Ok(rows)
    }

    /// Returns the exact first ten rows of the reauthenticated Top-25. It is
    /// never independently selected.
    pub(crate) fn top_ten(&mut self) -> Result<Vec<SelectionV5RowRecord>, SelectionV5Refusal> {
        Ok(self.top_twenty_five()?.into_iter().take(TOP_TEN).collect())
    }

    /// Reauthenticates Selection and retained Execution, then exact-joins each
    /// winner to its durable selected-exit digest by nonduplicated disposition
    /// identity. No detached caller digest enters this projection.
    pub(crate) fn successor_winners(
        &mut self,
    ) -> Result<Vec<SelectionV5SuccessorWinnerV1>, SelectionV5Refusal> {
        let rows = self.top_twenty_five()?;
        let execution_receipt_before = self.source.structural_receipt();
        let dispositions = self
            .source
            .ordered_authenticated_dispositions()
            .map_err(|why| format!("Selection V5 successor Execution source refused: {why}"))?;
        if self.source.structural_receipt() != execution_receipt_before {
            return Err(
                "Selection V5 retained Execution receipt changed during successor projection"
                    .to_owned(),
            );
        }
        let mut selected_by_disposition = HashMap::new();
        selected_by_disposition
            .try_reserve(dispositions.len())
            .map_err(|why| {
                format!("Selection V5 successor disposition index reserve failed: {why}")
            })?;
        for disposition in dispositions {
            if selected_by_disposition
                .insert(
                    disposition.disposition_id(),
                    disposition.selected_exit_digest(),
                )
                .is_some()
            {
                return Err(
                    "Selection V5 retained Execution contains a duplicate disposition identity"
                        .to_owned(),
                );
            }
        }
        let mut projected = Vec::new();
        projected
            .try_reserve_exact(rows.len())
            .map_err(|why| format!("Selection V5 successor winner reserve failed: {why}"))?;
        for row in rows {
            let selected_exit_digest = selected_by_disposition
                .remove(&row.disposition_id())
                .ok_or_else(|| {
                    format!(
                        "Selection V5 winner {} has no retained Execution disposition",
                        hex32(row.row_id())
                    )
                })?;
            projected.push(SelectionV5SuccessorWinnerV1 {
                row,
                selected_exit_digest,
            });
        }
        let prepared_after =
            PreparedSelectionV5::from_committed_execution(&mut self.source, self.policy)?;
        if prepared_after.selection_id != self.structural_receipt().selection_id() {
            return Err(
                "Selection V5 retained source changed after successor projection".to_owned(),
            );
        }
        Ok(projected)
    }
}

/// Sole production Selection V5 commit door.
///
/// A caller supplies only the physical root/bounds and canonical ranking
/// policy. Every population, mask, metric and execution fact comes from the
/// consumed source-retaining Execution V3 capability. The prepared projection
/// is reproduced after write/reuse before this capability is returned.
pub(crate) fn commit_stored_selection_v5(
    root: &Path,
    bounds: SelectionV5Bounds,
    mut source: CommittedStoredExecutionV3,
    policy: RankingPolicyV1,
) -> Result<CommittedStoredSelectionV5, SelectionV5Refusal> {
    let prepared_before = PreparedSelectionV5::from_committed_execution(&mut source, policy)?;
    let mut selection = persist_prepared(root, bounds, &prepared_before)?;
    let prepared_after = PreparedSelectionV5::from_committed_execution(&mut source, policy)?;
    if prepared_after != prepared_before {
        return Err(
            "Selection V5 retained Population/Execution source changed during commit".to_owned(),
        );
    }
    let receipt = selection.authority().structural_receipt();
    selection
        .authority_mut()
        .ledger
        .require_exact_prepared(receipt, &prepared_after)?;
    Ok(CommittedStoredSelectionV5 {
        source,
        policy,
        selection,
    })
}

#[cfg(test)]
fn commit_prepared_for_test(
    root: &Path,
    bounds: SelectionV5Bounds,
    prepared: &PreparedSelectionV5,
) -> Result<SelectionV5Commit, SelectionV5Refusal> {
    persist_prepared(root, bounds, prepared)
}

fn validate_complete_block(
    rows: &[SelectionV5RowRecord],
    completion: &SelectionV5CompletionRecord,
) -> Result<(), SelectionV5Refusal> {
    completion.validate()?;
    if usize_to_u64(rows.len(), "decoded selected row count")? != completion.selected_count {
        return Err(format!(
            "Selection V5 Completion declares {} winners but decoded {}",
            completion.selected_count,
            rows.len()
        ));
    }
    let mut row_ids = HashSet::new();
    let mut strategy_ids = HashSet::new();
    row_ids
        .try_reserve(rows.len())
        .map_err(|why| format!("Selection V5 block row-identity reserve failed: {why}"))?;
    strategy_ids
        .try_reserve(rows.len())
        .map_err(|why| format!("Selection V5 block strategy reserve failed: {why}"))?;
    for (index, row) in rows.iter().enumerate() {
        row.validate()?;
        if row.selection_id != completion.selection_id
            || row.population_id != completion.population_id
            || row.population_ordered_digest != completion.population_ordered_digest
            || row.execution_completion_id != completion.execution_completion_id
            || row.ordered_disposition_digest != completion.ordered_disposition_digest
            || row.nifty_authority_id != completion.nifty_authority_id
            || row.banknifty_authority_id != completion.banknifty_authority_id
            || row.ranking_policy_digest != completion.ranking_policy_digest
            || row.rung_seconds != completion.rung_seconds
            || usize::try_from(row.rank).ok() != Some(index)
            || !row_ids.insert(row.row_id)
            || !strategy_ids.insert(row.strategy_digest)
        {
            return Err(
                "Selection V5 stored winner block violates source/order uniqueness".to_owned(),
            );
        }
    }
    validate_strongest_first(rows)?;
    if ordered_selected_digest(rows)? != completion.ordered_selected_digest {
        return Err("Selection V5 stored winner order digest does not reproduce".to_owned());
    }
    Ok(())
}

fn validate_trailing_prefix(rows: &[SelectionV5RowRecord]) -> Result<(), SelectionV5Refusal> {
    let Some(first) = rows.first() else {
        return Err("Selection V5 trailing-prefix validator received no rows".to_owned());
    };
    let mut row_ids = HashSet::new();
    let mut strategy_ids = HashSet::new();
    row_ids
        .try_reserve(rows.len())
        .map_err(|why| format!("Selection V5 orphan row-identity reserve failed: {why}"))?;
    strategy_ids
        .try_reserve(rows.len())
        .map_err(|why| format!("Selection V5 orphan strategy reserve failed: {why}"))?;
    for (index, row) in rows.iter().enumerate() {
        row.validate()?;
        if row.selection_id != first.selection_id
            || row.population_id != first.population_id
            || row.population_ordered_digest != first.population_ordered_digest
            || row.execution_completion_id != first.execution_completion_id
            || row.ordered_disposition_digest != first.ordered_disposition_digest
            || row.nifty_authority_id != first.nifty_authority_id
            || row.banknifty_authority_id != first.banknifty_authority_id
            || row.ranking_policy_digest != first.ranking_policy_digest
            || row.rung_seconds != first.rung_seconds
            || usize::try_from(row.rank).ok() != Some(index)
            || !row_ids.insert(row.row_id)
            || !strategy_ids.insert(row.strategy_digest)
        {
            return Err("Selection V5 orphan tail is not one canonical prefix".to_owned());
        }
    }
    validate_strongest_first(rows)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformIdentity {
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(not(unix))]
    canonical_marker: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGeneration {
    identity: PlatformIdentity,
    len: u64,
    digest: [u8; 32],
    #[cfg(unix)]
    modified_seconds: i64,
    #[cfg(unix)]
    modified_nanoseconds: i64,
    #[cfg(unix)]
    changed_seconds: i64,
    #[cfg(unix)]
    changed_nanoseconds: i64,
    #[cfg(unix)]
    links: u64,
}

struct BoundedFixedFile {
    file: File,
    path: PathBuf,
    stride: usize,
    max_records: u64,
    max_bytes: u64,
    label: &'static str,
    generation: FileGeneration,
}

impl BoundedFixedFile {
    #[allow(clippy::too_many_arguments)]
    fn open(
        path: PathBuf,
        writable: bool,
        stride: usize,
        max_records: u64,
        max_bytes: u64,
        label: &'static str,
        created: &mut bool,
    ) -> Result<Self, SelectionV5Refusal> {
        let existed = path.exists();
        let file = open_child(&path, writable, created)?;
        if !existed && !writable {
            return Err(format!("Selection V5 {label} {} is absent", path.display()));
        }
        require_regular_unique_file(&file, &path)?;
        let mut value = Self {
            file,
            path,
            stride,
            max_records,
            max_bytes,
            label,
            generation: empty_generation(),
        };
        value.refresh()?;
        Ok(value)
    }

    fn refresh(&mut self) -> Result<(), SelectionV5Refusal> {
        self.generation =
            measured_generation(&mut self.file, &self.path, self.max_bytes, self.label)?;
        self.record_count()?;
        Ok(())
    }

    fn record_count(&self) -> Result<u64, SelectionV5Refusal> {
        let stride = u64::try_from(self.stride)
            .map_err(|_| format!("Selection V5 {} stride does not fit u64", self.label))?;
        if self.generation.len > self.max_bytes {
            return Err(format!(
                "Selection V5 {} file has {} bytes above explicit maximum {}",
                self.label, self.generation.len, self.max_bytes
            ));
        }
        if !self.generation.len.is_multiple_of(stride) {
            return Err(format!(
                "Selection V5 {} file is ragged: {} bytes is not divisible by stride {}",
                self.label, self.generation.len, stride
            ));
        }
        let count = self.generation.len / stride;
        if count > self.max_records {
            return Err(format!(
                "Selection V5 {} file has {count} records above explicit maximum {}",
                self.label, self.max_records
            ));
        }
        Ok(count)
    }

    fn require_unchanged(&self) -> Result<(), SelectionV5Refusal> {
        let mut duplicate = self
            .file
            .try_clone()
            .map_err(|why| format!("cannot clone Selection V5 {} file: {why}", self.label))?;
        let observed = measured_generation(&mut duplicate, &self.path, self.max_bytes, self.label)?;
        if observed != self.generation {
            return Err(format!(
                "Selection V5 retained {} generation changed",
                self.label
            ));
        }
        Ok(())
    }
}

const fn empty_generation() -> FileGeneration {
    FileGeneration {
        identity: PlatformIdentity {
            #[cfg(unix)]
            device: 0,
            #[cfg(unix)]
            inode: 0,
            #[cfg(not(unix))]
            canonical_marker: 0,
        },
        len: 0,
        digest: [0; 32],
        #[cfg(unix)]
        modified_seconds: 0,
        #[cfg(unix)]
        modified_nanoseconds: 0,
        #[cfg(unix)]
        changed_seconds: 0,
        #[cfg(unix)]
        changed_nanoseconds: 0,
        #[cfg(unix)]
        links: 0,
    }
}

fn measured_generation(
    file: &mut File,
    path: &Path,
    max_bytes: u64,
    label: &str,
) -> Result<FileGeneration, SelectionV5Refusal> {
    require_regular_unique_file(file, path)?;
    let held = file
        .metadata()
        .map_err(|why| format!("cannot stat held Selection V5 {label}: {why}"))?;
    let len = held.len();
    if len > max_bytes {
        return Err(format!(
            "Selection V5 {label} file has {len} bytes above explicit maximum {max_bytes}"
        ));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("cannot seek Selection V5 {label} for generation: {why}"))?;
    let mut hasher = Hasher::new();
    hasher.update(FILE_GENERATION_DOMAIN);
    hasher.update(label.as_bytes());
    hasher.update(&len.to_le_bytes());
    let mut remaining = len;
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    while remaining != 0 {
        let take = usize::try_from(remaining.min(16 * 1_024_u64))
            .map_err(|_| "Selection V5 generation chunk does not fit usize".to_owned())?;
        let chunk = buffer
            .get_mut(..take)
            .ok_or_else(|| "Selection V5 generation chunk is outside buffer".to_owned())?;
        file.read_exact(chunk)
            .map_err(|why| format!("cannot hash Selection V5 {label}: {why}"))?;
        hasher.update(chunk);
        remaining = remaining
            .checked_sub(
                u64::try_from(take)
                    .map_err(|_| "Selection V5 generation chunk does not fit u64".to_owned())?,
            )
            .ok_or_else(|| "Selection V5 generation remaining underflowed".to_owned())?;
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("cannot rewind Selection V5 {label}: {why}"))?;
    let named = std::fs::metadata(path)
        .map_err(|why| format!("cannot stat named Selection V5 {label}: {why}"))?;
    if platform_identity(&held) != platform_identity(&named) {
        return Err(format!(
            "Selection V5 held {label} no longer matches {}",
            path.display()
        ));
    }
    Ok(FileGeneration {
        identity: platform_identity(&held),
        len,
        digest: hasher.finalize(),
        #[cfg(unix)]
        modified_seconds: held.mtime(),
        #[cfg(unix)]
        modified_nanoseconds: held.mtime_nsec(),
        #[cfg(unix)]
        changed_seconds: held.ctime(),
        #[cfg(unix)]
        changed_nanoseconds: held.ctime_nsec(),
        #[cfg(unix)]
        links: held.nlink(),
    })
}

fn open_root_directory(
    root: &Path,
) -> Result<(PathBuf, File, PlatformIdentity), SelectionV5Refusal> {
    require_not_symlink(root, false)?;
    let canonical = std::fs::canonicalize(root).map_err(|why| {
        format!(
            "Selection V5 root {} must already exist: {why}",
            root.display()
        )
    })?;
    let file = File::open(&canonical).map_err(|why| {
        format!(
            "cannot hold Selection V5 root {}: {why}",
            canonical.display()
        )
    })?;
    let metadata = file.metadata().map_err(|why| {
        format!(
            "cannot stat held Selection V5 root {}: {why}",
            canonical.display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "Selection V5 root {} is not a directory",
            canonical.display()
        ));
    }
    let identity = platform_identity(&metadata);
    if named_identity(&canonical)? != identity {
        return Err("Selection V5 root changed while opening".to_owned());
    }
    Ok((canonical, file, identity))
}

fn open_child(path: &Path, writable: bool, created: &mut bool) -> Result<File, SelectionV5Refusal> {
    require_not_symlink(path, true)?;
    let existed = path.exists();
    let mut options = OpenOptions::new();
    options.read(true);
    if writable {
        options.write(true).create(true).truncate(false);
    }
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    options.custom_flags(O_NOFOLLOW_FLAG);
    let file = options
        .open(path)
        .map_err(|why| format!("cannot open Selection V5 child {}: {why}", path.display()))?;
    if writable && !existed {
        *created = true;
    }
    Ok(file)
}

fn require_not_symlink(path: &Path, absent_ok: bool) -> Result<(), SelectionV5Refusal> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(format!("Selection V5 path {} is a symlink", path.display()))
        }
        Ok(_) => Ok(()),
        Err(why) if absent_ok && why.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(why) => Err(format!(
            "cannot inspect Selection V5 path {}: {why}",
            path.display()
        )),
    }
}

fn require_regular_unique_file(file: &File, path: &Path) -> Result<(), SelectionV5Refusal> {
    let held = file.metadata().map_err(|why| {
        format!(
            "cannot stat held Selection V5 file {}: {why}",
            path.display()
        )
    })?;
    let named = std::fs::metadata(path).map_err(|why| {
        format!(
            "cannot stat named Selection V5 file {}: {why}",
            path.display()
        )
    })?;
    if !held.is_file() || !named.is_file() || platform_identity(&held) != platform_identity(&named)
    {
        return Err(format!(
            "Selection V5 held file no longer matches regular path {}",
            path.display()
        ));
    }
    #[cfg(unix)]
    if held.nlink() != 1 || named.nlink() != 1 {
        return Err(format!(
            "Selection V5 file {} must have exactly one hard link",
            path.display()
        ));
    }
    Ok(())
}

fn named_identity(path: &Path) -> Result<PlatformIdentity, SelectionV5Refusal> {
    let metadata = std::fs::metadata(path)
        .map_err(|why| format!("cannot stat Selection V5 path {}: {why}", path.display()))?;
    Ok(platform_identity(&metadata))
}

#[cfg(unix)]
fn platform_identity(metadata: &std::fs::Metadata) -> PlatformIdentity {
    PlatformIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

#[cfg(not(unix))]
fn platform_identity(_metadata: &std::fs::Metadata) -> PlatformIdentity {
    PlatformIdentity {
        canonical_marker: 1,
    }
}

fn sync_directory(file: &File, root: &Path) -> Result<(), SelectionV5Refusal> {
    file.sync_all().map_err(|why| {
        format!(
            "cannot sync Selection V5 root directory {}: {why}",
            root.display()
        )
    })
}

fn append_raw(file: &mut File, raw: &[u8]) -> Result<(), SelectionV5Refusal> {
    file.seek(SeekFrom::End(0))
        .map_err(|why| format!("cannot seek Selection V5 append: {why}"))?;
    file.write_all(raw)
        .map_err(|why| format!("cannot append Selection V5 record: {why}"))
}

fn read_fixed_at<const N: usize>(
    file: &mut File,
    index: u64,
    stride: usize,
    label: &str,
) -> Result<[u8; N], SelectionV5Refusal> {
    if N != stride {
        return Err(format!(
            "Selection V5 {label} read buffer {N} differs from stride {stride}"
        ));
    }
    let offset = index
        .checked_mul(
            u64::try_from(stride)
                .map_err(|_| format!("Selection V5 {label} stride does not fit u64"))?,
        )
        .ok_or_else(|| format!("Selection V5 {label} read offset overflowed"))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|why| format!("cannot seek Selection V5 {label} {index}: {why}"))?;
    let mut raw = [0_u8; N];
    file.read_exact(&mut raw)
        .map_err(|why| format!("cannot read Selection V5 {label} {index}: {why}"))?;
    Ok(raw)
}

struct FixedWriter<'a> {
    bytes: &'a mut [u8],
    cursor: usize,
}

impl<'a> FixedWriter<'a> {
    const fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.cursor)
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), SelectionV5Refusal> {
        let end = self
            .cursor
            .checked_add(value.len())
            .ok_or_else(|| "Selection V5 writer cursor overflowed".to_owned())?;
        let slot = self
            .bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "Selection V5 writer exceeded fixed payload".to_owned())?;
        slot.copy_from_slice(value);
        self.cursor = end;
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), SelectionV5Refusal> {
        self.bytes(&[value])
    }

    fn u32(&mut self, value: u32) -> Result<(), SelectionV5Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), SelectionV5Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), SelectionV5Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), SelectionV5Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "Selection V5 zero-fill cursor overflowed".to_owned())?;
        let slot = self
            .bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "Selection V5 zero-fill exceeded fixed payload".to_owned())?;
        slot.fill(0);
        self.cursor = end;
        Ok(())
    }

    fn require_full(&self, label: &str) -> Result<(), SelectionV5Refusal> {
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err(format!(
                "{label} used {} of {} bytes",
                self.cursor,
                self.bytes.len()
            ))
        }
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

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.cursor)
    }

    fn bytes(&mut self, count: usize) -> Result<&'a [u8], SelectionV5Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "Selection V5 reader cursor overflowed".to_owned())?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| "Selection V5 reader exceeded fixed payload".to_owned())?;
        self.cursor = end;
        Ok(value)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], SelectionV5Refusal> {
        self.bytes(N)?
            .try_into()
            .map_err(|_| format!("Selection V5 could not decode a {N}-byte array"))
    }

    fn u8(&mut self) -> Result<u8, SelectionV5Refusal> {
        self.bytes(1)?
            .first()
            .copied()
            .ok_or_else(|| "Selection V5 u8 is absent".to_owned())
    }

    fn u32(&mut self) -> Result<u32, SelectionV5Refusal> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, SelectionV5Refusal> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn i64(&mut self) -> Result<i64, SelectionV5Refusal> {
        Ok(i64::from_le_bytes(self.array()?))
    }

    fn require_zeros(&mut self, count: usize, label: &str) -> Result<(), SelectionV5Refusal> {
        if self.bytes(count)?.iter().any(|byte| *byte != 0) {
            Err(format!("Selection V5 {label} is nonzero"))
        } else {
            Ok(())
        }
    }
}

fn write_header(
    writer: &mut FixedWriter<'_>,
    magic: [u8; 16],
    domain: u32,
) -> Result<(), SelectionV5Refusal> {
    writer.bytes(&magic)?;
    writer.u32(VERSION)?;
    writer.u32(domain)
}

fn require_header(
    label: &str,
    reader: &mut FixedReader<'_>,
    magic: [u8; 16],
    domain: u32,
) -> Result<(), SelectionV5Refusal> {
    if reader.array::<16>()? != magic {
        return Err(format!("Selection V5 {label} magic is unknown"));
    }
    let version = reader.u32()?;
    let observed_domain = reader.u32()?;
    if version != VERSION || observed_domain != domain {
        return Err(format!(
            "Selection V5 {label} version/domain {version}/{observed_domain} is unsupported"
        ));
    }
    Ok(())
}

fn encode_metrics(
    writer: &mut FixedWriter<'_>,
    metrics: Metrics,
) -> Result<(), SelectionV5Refusal> {
    for value in [
        metrics.drawdown,
        metrics.worst_loss,
        metrics.losing_rate_ppm,
        metrics.losing_trades,
    ] {
        writer.u64(value)?;
    }
    encode_optional_u64(writer, metrics.loss_ratio_ppm)?;
    writer.i64(metrics.pessimistic_profit)?;
    for value in [metrics.winning_trades, metrics.win_rate_ppm] {
        writer.u64(value)?;
    }
    encode_optional_u64(writer, metrics.reward_to_risk_ppm)?;
    for value in [
        metrics.average_win,
        metrics.average_loss,
        metrics.assurance_ppm,
    ] {
        writer.u64(value)?;
    }
    Ok(())
}

fn decode_metrics(reader: &mut FixedReader<'_>) -> Result<Metrics, SelectionV5Refusal> {
    Ok(Metrics {
        drawdown: reader.u64()?,
        worst_loss: reader.u64()?,
        losing_rate_ppm: reader.u64()?,
        losing_trades: reader.u64()?,
        loss_ratio_ppm: decode_optional_u64(reader)?,
        pessimistic_profit: reader.i64()?,
        winning_trades: reader.u64()?,
        win_rate_ppm: reader.u64()?,
        reward_to_risk_ppm: decode_optional_u64(reader)?,
        average_win: reader.u64()?,
        average_loss: reader.u64()?,
        assurance_ppm: reader.u64()?,
    })
}

fn encode_optional_u64(
    writer: &mut FixedWriter<'_>,
    value: Option<u64>,
) -> Result<(), SelectionV5Refusal> {
    if let Some(value) = value {
        writer.u8(1)?;
        writer.zeros(7)?;
        writer.u64(value)
    } else {
        writer.u8(0)?;
        writer.zeros(7)?;
        writer.u64(0)
    }
}

fn decode_optional_u64(reader: &mut FixedReader<'_>) -> Result<Option<u64>, SelectionV5Refusal> {
    let tag = reader.u8()?;
    reader.require_zeros(7, "optional-u64 reserve")?;
    let value = reader.u64()?;
    match (tag, value) {
        (0, 0) => Ok(None),
        (0, _) => Err("Selection V5 absent optional-u64 carries a value".to_owned()),
        (1, value) => Ok(Some(value)),
        _ => Err(format!("Selection V5 optional-u64 tag {tag} is unknown")),
    }
}

fn require_seal(
    label: &str,
    domain: &[u8],
    payload: &[u8],
    seal: &[u8],
) -> Result<(), SelectionV5Refusal> {
    if seal == hash_parts(domain, &[payload]) {
        Ok(())
    } else {
        Err(format!("Selection V5 {label} seal mismatch"))
    }
}

fn hash_parts(domain: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize()
}

fn decode_family(value: u8) -> Result<SelectionV5Family, SelectionV5Refusal> {
    match value {
        1 => Ok(SelectionV5Family::Nifty),
        2 => Ok(SelectionV5Family::BankNifty),
        _ => Err(format!("Selection V5 family tag {value} is unknown")),
    }
}

const fn family_byte(value: SelectionV5Family) -> u8 {
    match value {
        SelectionV5Family::Nifty => 1,
        SelectionV5Family::BankNifty => 2,
    }
}

fn decode_direction(value: u8) -> Result<Direction, SelectionV5Refusal> {
    match value {
        1 => Ok(Direction::Long),
        2 => Ok(Direction::Short),
        _ => Err(format!("Selection V5 direction tag {value} is unknown")),
    }
}

const fn direction_byte(value: Direction) -> u8 {
    match value {
        Direction::Long => 1,
        Direction::Short => 2,
    }
}

fn decode_admission(value: u8) -> Result<AdmissionStatusV1, SelectionV5Refusal> {
    match value {
        1 => Ok(AdmissionStatusV1::Admitted),
        2 => Ok(AdmissionStatusV1::Rejected),
        3 => Ok(AdmissionStatusV1::Unmeasured),
        4 => Ok(AdmissionStatusV1::Refused),
        _ => Err(format!("Selection V5 admission tag {value} is unknown")),
    }
}

const fn admission_byte(value: AdmissionStatusV1) -> u8 {
    match value {
        AdmissionStatusV1::Admitted => 1,
        AdmissionStatusV1::Rejected => 2,
        AdmissionStatusV1::Unmeasured => 3,
        AdmissionStatusV1::Refused => 4,
    }
}

fn decode_terminal(value: u8) -> Result<SelectionV5Terminal, SelectionV5Refusal> {
    match value {
        1 => Ok(SelectionV5Terminal::Authorized),
        2 => Ok(SelectionV5Terminal::PolicyRefused),
        _ => Err(format!(
            "Selection V5 execution-terminal tag {value} is unknown"
        )),
    }
}

const fn terminal_byte(value: SelectionV5Terminal) -> u8 {
    match value {
        SelectionV5Terminal::Authorized => 1,
        SelectionV5Terminal::PolicyRefused => 2,
    }
}

fn require_nonzero(name: &str, value: [u8; 32]) -> Result<(), SelectionV5Refusal> {
    if value == [0; 32] {
        Err(format!("Selection V5 {name} is zero"))
    } else {
        Ok(())
    }
}

fn checked_sum<I>(values: I, name: &str) -> Result<u64, SelectionV5Refusal>
where
    I: IntoIterator<Item = u64>,
{
    values.into_iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(value)
            .ok_or_else(|| format!("Selection V5 {name} overflowed u64"))
    })
}

fn checked_end(first: u64, count: u64, name: &str) -> Result<u64, SelectionV5Refusal> {
    first
        .checked_add(count)
        .ok_or_else(|| format!("Selection V5 {name} overflowed u64"))
}

fn increment(value: &mut u64, name: &str) -> Result<(), SelectionV5Refusal> {
    *value = value
        .checked_add(1)
        .ok_or_else(|| format!("Selection V5 {name} overflowed u64"))?;
    Ok(())
}

fn usize_to_u64(value: usize, name: &str) -> Result<u64, SelectionV5Refusal> {
    u64::try_from(value).map_err(|_| format!("Selection V5 {name} does not fit u64"))
}

fn bounded_vec<T>(count: u64, max: u64, name: &str) -> Result<Vec<T>, SelectionV5Refusal> {
    if count > max {
        return Err(format!(
            "Selection V5 {name} count {count} exceeds explicit maximum {max}"
        ));
    }
    let capacity = usize::try_from(count)
        .map_err(|_| format!("Selection V5 {name} count does not fit usize"))?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(capacity)
        .map_err(|why| format!("Selection V5 {name} reserve failed: {why}"))?;
    Ok(values)
}

fn combine_lock_result<T>(
    result: Result<T, SelectionV5Refusal>,
    released: Result<(), SelectionV5Refusal>,
) -> Result<T, SelectionV5Refusal> {
    match (result, released) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(why), _) | (Ok(_), Err(why)) => Err(why),
    }
}

fn hex32(value: [u8; 32]) -> String {
    let mut text = String::with_capacity(64);
    for byte in value {
        text.push(hex_nibble(byte >> 4));
        text.push(hex_nibble(byte & 0x0f));
    }
    text
}

fn hex_nibble(value: u8) -> char {
    let encoded = if value < 10 {
        b'0'.wrapping_add(value)
    } else {
        b'a'.wrapping_add(value.wrapping_sub(10))
    };
    char::from(encoded)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "tests must fail loudly and inspect exact fixed-record bytes"
)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use runner::topn::Weights;

    use super::*;

    static ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct TestRoot {
        path: PathBuf,
    }

    impl TestRoot {
        fn new(name: &str) -> Self {
            let sequence = ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "brutex-selection-v5-{}-{name}-{sequence}",
                std::process::id()
            ));
            std::fs::create_dir(&path).expect("create Selection V5 test root");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            if self.path.exists() {
                std::fs::remove_dir_all(&self.path).expect("remove Selection V5 test root");
            }
            let displaced = self.path.with_extension("displaced");
            if displaced.exists() {
                std::fs::remove_dir_all(displaced)
                    .expect("remove displaced Selection V5 test root");
            }
        }
    }

    fn digest(seed: u8) -> [u8; 32] {
        [seed.max(1); 32]
    }

    fn policy() -> RankingPolicyV1 {
        RankingPolicyV1::new(Weights::equal()).expect("equal policy is valid")
    }

    fn metrics(value: u64) -> Metrics {
        Metrics {
            drawdown: 10_000_u64.saturating_sub(value.min(9_000)),
            worst_loss: 9_000_u64.saturating_sub(value.min(8_000)),
            losing_rate_ppm: 800_000_u64.saturating_sub(value.min(700_000)),
            losing_trades: 1_000_u64.saturating_sub(value.min(900)),
            loss_ratio_ppm: Some(700_000_u64.saturating_sub(value.min(600_000))),
            pessimistic_profit: i64::try_from(value).unwrap_or(i64::MAX),
            winning_trades: value,
            win_rate_ppm: value.min(1_000_000),
            reward_to_risk_ppm: Some(1_000_000_u64.saturating_add(value)),
            average_win: 1_000_u64.saturating_add(value),
            average_loss: 8_000_u64.saturating_sub(value.min(7_000)),
            assurance_ppm: value.min(1_000_000),
        }
    }

    fn source_rows(per_family: u64) -> Vec<SelectionV5SourceRow> {
        let total = per_family.saturating_mul(2);
        (0..total)
            .map(|global_sequence| {
                let (family, family_sequence) = if global_sequence < per_family {
                    (SelectionV5Family::Nifty, global_sequence)
                } else {
                    (
                        SelectionV5Family::BankNifty,
                        global_sequence.saturating_sub(per_family),
                    )
                };
                let admission = match global_sequence % 11 {
                    8 => AdmissionStatusV1::Rejected,
                    9 => AdmissionStatusV1::Unmeasured,
                    10 => AdmissionStatusV1::Refused,
                    _ => AdmissionStatusV1::Admitted,
                };
                let terminal = if global_sequence % 7 == 6 {
                    SelectionV5Terminal::PolicyRefused
                } else {
                    SelectionV5Terminal::Authorized
                };
                let bit = u32::try_from(global_sequence % 370).unwrap_or(0);
                let mut mask_words = [0_u64; 6];
                mask_words[usize::try_from(bit / 64).unwrap_or(0)] =
                    1_u64.checked_shl(bit % 64).unwrap_or(0);
                SelectionV5SourceRow {
                    population_id: digest(1),
                    population_ordered_digest: digest(2),
                    execution_completion_id: digest(3),
                    disposition_id: digest(
                        u8::try_from(global_sequence.saturating_add(20)).unwrap_or(u8::MAX),
                    ),
                    global_sequence,
                    family_sequence,
                    family,
                    strategy_digest: digest(
                        u8::try_from(global_sequence.saturating_add(80)).unwrap_or(u8::MAX),
                    ),
                    mask_words,
                    direction: if global_sequence.is_multiple_of(2) {
                        Direction::Long
                    } else {
                        Direction::Short
                    },
                    admission,
                    terminal,
                    metrics: metrics(global_sequence.saturating_mul(10_000)),
                }
            })
            .collect()
    }

    fn snapshot(rows: &[SelectionV5SourceRow]) -> SelectionV5SourceSnapshot {
        let mut matrix = [0_u64; TERMINAL_MATRIX_CELLS];
        let mut nifty_count = 0_u64;
        let mut banknifty_count = 0_u64;
        for row in rows {
            match row.family {
                SelectionV5Family::Nifty => nifty_count = nifty_count.saturating_add(1),
                SelectionV5Family::BankNifty => banknifty_count = banknifty_count.saturating_add(1),
            }
            let index = matrix_index(row.family, row.admission, row.terminal);
            matrix[index] = matrix[index].saturating_add(1);
        }
        let (_, _, admitted_count, authorized_count, eligible_count) =
            matrix_marginals(&matrix).expect("fixture matrix sums");
        SelectionV5SourceSnapshot {
            population_id: digest(1),
            population_ordered_digest: digest(2),
            execution_completion_id: digest(3),
            ordered_disposition_digest: digest(4),
            nifty_authority_id: digest(5),
            banknifty_authority_id: digest(6),
            rung_seconds: 300,
            row_count: u64::try_from(rows.len()).expect("fixture row count fits"),
            nifty_count,
            banknifty_count,
            admitted_count,
            authorized_count,
            eligible_count,
            terminal_matrix: matrix,
        }
    }

    fn prepared(per_family: u64) -> PreparedSelectionV5 {
        let rows = source_rows(per_family);
        PreparedSelectionV5::from_authenticated_execution_fixture(snapshot(&rows), &rows, policy())
            .expect("fixture prepares")
    }

    fn bounds() -> SelectionV5Bounds {
        SelectionV5Bounds::new(
            250,
            250 * SELECTION_V5_ROW_BYTES as u64,
            10,
            10 * SELECTION_V5_COMPLETION_BYTES as u64,
        )
        .expect("fixture bounds are valid")
    }

    fn create_empty_files(root: &Path) {
        drop(SelectionV5Ledger::open_write(root, bounds()).expect("create V5 files"));
    }

    fn write_rows(root: &Path, rows: &[SelectionV5RowRecord]) {
        let raw: Vec<u8> = rows
            .iter()
            .flat_map(|row| row.encode().expect("winner row encodes"))
            .collect();
        std::fs::write(root.join(ROW_FILE), raw).expect("write Selection V5 rows");
    }

    fn assert_mutated_orphan_source_refuses(
        name: &str,
        fixture: &PreparedSelectionV5,
        mutate: impl FnOnce(&mut SelectionV5RowRecord),
    ) {
        let root = TestRoot::new(name);
        create_empty_files(root.path());
        let mut prefix = fixture.rows[..2].to_vec();
        mutate(&mut prefix[1]);
        write_rows(root.path(), &prefix);
        assert!(SelectionV5Ledger::open_write(root.path(), bounds()).is_err());
    }

    fn directory_bytes(root: &Path) -> Vec<(String, Vec<u8>)> {
        let mut values: Vec<_> = [ROW_FILE, COMPLETION_FILE, LOCK_FILE]
            .into_iter()
            .map(|name| {
                (
                    name.to_owned(),
                    std::fs::read(root.join(name)).expect("read Selection V5 file"),
                )
            })
            .collect();
        values.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        values
    }

    #[test]
    fn bounds_and_fixed_codecs_are_canonical_and_semantic_ids_ignore_offsets() {
        assert!(SelectionV5Bounds::new(0, 1, 1, 1_024).is_err());
        assert!(SelectionV5Bounds::new(1, 1_023, 1, 1_024).is_err());
        assert!(SelectionV5Bounds::new(1, 1_024, 1, 1_023).is_err());
        let fixture = prepared(24);
        let first = fixture.rows[0];
        let row_raw = first.encode().expect("winner encodes");
        assert_eq!(
            SelectionV5RowRecord::decode(&row_raw).expect("winner decodes"),
            first
        );
        let mut corrupted = row_raw;
        corrupted[100] ^= 1;
        assert!(SelectionV5RowRecord::decode(&corrupted).is_err());

        let original = fixture
            .expected_completion(0, 0)
            .expect("Completion derives");
        let relocated = fixture
            .expected_completion(9, 777)
            .expect("relocated Completion derives");
        assert_eq!(original.completion_id, relocated.completion_id);
        assert_ne!(
            original.encode().expect("Completion encodes"),
            relocated.encode().expect("relocated Completion encodes")
        );
        let raw = original.encode().expect("Completion encodes");
        assert_eq!(
            SelectionV5CompletionRecord::decode(&raw).expect("Completion decodes"),
            original
        );
        let mut corrupted = raw;
        corrupted[400] ^= 1;
        assert!(SelectionV5CompletionRecord::decode(&corrupted).is_err());
    }

    #[test]
    fn one_combined_top_twenty_five_has_exact_ties_and_one_top_ten_prefix() {
        let mut rows = source_rows(40);
        for row in &mut rows {
            row.admission = AdmissionStatusV1::Admitted;
            row.terminal = SelectionV5Terminal::Authorized;
            row.metrics = metrics(500_000);
        }
        let source = snapshot(&rows);
        let first = PreparedSelectionV5::from_authenticated_execution_fixture(
            source.clone(),
            &rows,
            policy(),
        )
        .expect("tied fixture selects");
        assert_eq!(first.rows.len(), 25);
        assert!(
            first
                .rows
                .iter()
                .any(|row| row.family == SelectionV5Family::Nifty)
        );
        assert!(
            first
                .rows
                .iter()
                .any(|row| row.family == SelectionV5Family::BankNifty)
        );
        let expected_top_ten: Vec<_> = first.rows.iter().take(10).copied().collect();

        for family in [SelectionV5Family::Nifty, SelectionV5Family::BankNifty] {
            let indices: Vec<_> = rows
                .iter()
                .enumerate()
                .filter_map(|(index, row)| (row.family == family).then_some(index))
                .collect();
            let values: Vec<_> = indices
                .iter()
                .map(|index| {
                    let row = rows[*index];
                    (
                        row.disposition_id,
                        row.strategy_digest,
                        row.mask_words,
                        row.direction,
                    )
                })
                .rev()
                .collect();
            for (index, value) in indices.into_iter().zip(values) {
                rows[index].disposition_id = value.0;
                rows[index].strategy_digest = value.1;
                rows[index].mask_words = value.2;
                rows[index].direction = value.3;
            }
        }
        let reordered =
            PreparedSelectionV5::from_authenticated_execution_fixture(source, &rows, policy())
                .expect("reordered tied fixture selects");
        assert_eq!(
            first
                .rows
                .iter()
                .map(|row| row.strategy_digest)
                .collect::<Vec<_>>(),
            reordered
                .rows
                .iter()
                .map(|row| row.strategy_digest)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            expected_top_ten
                .iter()
                .map(|row| row.strategy_digest)
                .collect::<Vec<_>>(),
            reordered
                .rows
                .iter()
                .take(10)
                .map(|row| row.strategy_digest)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn complete_two_family_source_may_legitimately_select_one_family_only() {
        let mut rows = source_rows(40);
        for row in &mut rows {
            row.admission = AdmissionStatusV1::Admitted;
            row.terminal = SelectionV5Terminal::Authorized;
            row.metrics = match row.family {
                SelectionV5Family::Nifty => metrics(900_000),
                SelectionV5Family::BankNifty => metrics(1),
            };
        }
        let prepared = PreparedSelectionV5::from_authenticated_execution_fixture(
            snapshot(&rows),
            &rows,
            policy(),
        )
        .expect("complete two-family source selects");
        assert_eq!(prepared.rows.len(), REQUESTED_TOP);
        assert!(
            prepared
                .rows
                .iter()
                .all(|winner| winner.family == SelectionV5Family::Nifty)
        );
        assert_eq!(prepared.source.banknifty_count, 40);
    }

    #[test]
    fn ranking_policy_identity_is_bound_and_exact_policy_rerun_reuses() {
        let root = TestRoot::new("policy-identity");
        let rows = source_rows(30);
        let equal = PreparedSelectionV5::from_authenticated_execution_fixture(
            snapshot(&rows),
            &rows,
            policy(),
        )
        .expect("equal policy prepares");
        let mut weighted = Weights::equal();
        weighted.drawdown = 2;
        let weighted_policy =
            RankingPolicyV1::new(weighted).expect("weighted policy is nonzero and valid");
        let weighted = PreparedSelectionV5::from_authenticated_execution_fixture(
            snapshot(&rows),
            &rows,
            weighted_policy,
        )
        .expect("weighted policy prepares");
        assert_ne!(equal.ranking_policy_digest, weighted.ranking_policy_digest);
        assert_ne!(equal.selection_id, weighted.selection_id);

        let equal_commit =
            commit_prepared_for_test(root.path(), bounds(), &equal).expect("equal policy commits");
        assert!(equal_commit.was_written());
        let weighted_commit = commit_prepared_for_test(root.path(), bounds(), &weighted)
            .expect("different policy writes a distinct block");
        assert!(weighted_commit.was_written());
        let weighted_reuse = commit_prepared_for_test(root.path(), bounds(), &weighted)
            .expect("same weighted policy reuses");
        assert!(!weighted_reuse.was_written());
        assert_eq!(
            weighted_reuse.authority().structural_receipt(),
            weighted_commit.authority().structural_receipt()
        );
    }

    #[test]
    fn eligibility_family_completeness_and_strategy_aliases_fail_closed() {
        let mut rows = source_rows(20);
        rows[0].metrics = metrics(1_000_000);
        rows[0].terminal = SelectionV5Terminal::PolicyRefused;
        let fixture = PreparedSelectionV5::from_authenticated_execution_fixture(
            snapshot(&rows),
            &rows,
            policy(),
        )
        .expect("complete source selects");
        assert!(
            fixture
                .rows
                .iter()
                .all(|row| row.disposition_id != rows[0].disposition_id)
        );

        let mut alias = rows.clone();
        alias[1].strategy_digest = alias[0].strategy_digest;
        assert!(
            PreparedSelectionV5::from_authenticated_execution_fixture(
                snapshot(&alias),
                &alias,
                policy()
            )
            .is_err()
        );

        let mut duplicate_disposition = rows.clone();
        duplicate_disposition[1].disposition_id = duplicate_disposition[0].disposition_id;
        assert!(
            PreparedSelectionV5::from_authenticated_execution_fixture(
                snapshot(&duplicate_disposition),
                &duplicate_disposition,
                policy()
            )
            .is_err()
        );

        let mut reordered = rows.clone();
        reordered.swap(1, 2);
        assert!(
            PreparedSelectionV5::from_authenticated_execution_fixture(
                snapshot(&reordered),
                &reordered,
                policy()
            )
            .is_err()
        );

        let mut foreign = rows.clone();
        foreign[3].execution_completion_id = digest(209);
        assert!(
            PreparedSelectionV5::from_authenticated_execution_fixture(
                snapshot(&foreign),
                &foreign,
                policy()
            )
            .is_err()
        );

        let nifty_only: Vec<_> = rows
            .iter()
            .copied()
            .filter(|row| row.family == SelectionV5Family::Nifty)
            .collect();
        assert!(
            PreparedSelectionV5::from_authenticated_execution_fixture(
                snapshot(&nifty_only),
                &nifty_only,
                policy()
            )
            .is_err()
        );

        let mut missing = rows.clone();
        missing.remove(3);
        assert!(
            PreparedSelectionV5::from_authenticated_execution_fixture(
                snapshot(&missing),
                &missing,
                policy()
            )
            .is_err()
        );
    }

    #[test]
    fn impossible_unmeasured_proof_refuses_after_identity_rederivation() {
        let mut prepared_attack = prepared(30);
        prepared_attack.proof.unmeasured = prepared_attack
            .proof
            .considered
            .checked_add(1)
            .expect("fixture considered count can increment");
        prepared_attack.selection_id = derive_selection_id(
            &prepared_attack.source,
            prepared_attack.ranking_policy_digest,
            prepared_attack.proof,
            &prepared_attack.rows,
        )
        .expect("attacker can rederive selection identity");
        for row in &mut prepared_attack.rows {
            row.selection_id = prepared_attack.selection_id;
            row.row_id = row.derive_row_id();
        }
        assert_eq!(
            prepared_attack
                .validate()
                .expect_err("impossible prepared proof must refuse"),
            "Selection V5 prepared counts do not reconcile"
        );

        let fixture = prepared(30);
        let mut completion_attack = fixture
            .expected_completion(0, 0)
            .expect("fixture Completion derives");
        completion_attack.proof_unmeasured = completion_attack
            .proof_considered
            .checked_add(1)
            .expect("fixture considered count can increment");
        completion_attack.completion_id = completion_attack.derive_completion_id();
        assert_eq!(
            completion_attack
                .validate()
                .expect_err("impossible Completion proof must refuse"),
            "Selection V5 Completion source/proof counts are contradictory"
        );
    }

    #[test]
    fn receipt_last_commit_fresh_reopen_and_exact_reuse_are_byte_identical() {
        let root = TestRoot::new("commit-reopen-reuse");
        let fixture = prepared(30);
        let mut first = commit_prepared_for_test(root.path(), bounds(), &fixture)
            .expect("first commit succeeds");
        assert!(first.was_written());
        let receipt = first.authority().structural_receipt();
        assert_eq!(receipt.selected_count(), 25);
        assert_eq!(receipt.top_ten_count(), 10);
        let top_twenty_five = first
            .authority_mut()
            .top_twenty_five()
            .expect("Top-25 reads");
        let top_ten = first.authority_mut().top_ten().expect("Top-10 reads");
        assert_eq!(top_ten, top_twenty_five[..10]);
        let before = directory_bytes(root.path());

        let mut reused =
            commit_prepared_for_test(root.path(), bounds(), &fixture).expect("exact rerun reuses");
        assert!(!reused.was_written());
        assert_eq!(reused.authority().structural_receipt(), receipt);
        assert_eq!(
            reused
                .authority_mut()
                .top_twenty_five()
                .expect("reused Top-25 reads"),
            top_twenty_five
        );
        assert_eq!(directory_bytes(root.path()), before);
    }

    #[test]
    fn every_exact_winner_prefix_recovers_but_foreign_or_reordered_prefix_refuses() {
        let fixture = prepared(30);
        for prefix in 0..=fixture.rows.len() {
            let root = TestRoot::new(&format!("prefix-{prefix}"));
            create_empty_files(root.path());
            let raw: Vec<u8> = fixture.rows[..prefix]
                .iter()
                .flat_map(|row| row.encode().expect("prefix row encodes"))
                .collect();
            std::fs::write(root.path().join(ROW_FILE), raw).expect("write exact orphan prefix");
            let committed = commit_prepared_for_test(root.path(), bounds(), &fixture)
                .expect("exact prefix recovers");
            assert!(committed.was_written());
        }

        let foreign = prepared(28);
        let root = TestRoot::new("foreign-prefix");
        create_empty_files(root.path());
        std::fs::write(
            root.path().join(ROW_FILE),
            foreign.rows[0].encode().expect("foreign row encodes"),
        )
        .expect("write foreign orphan");
        let mut writer = SelectionV5Ledger::open_write(root.path(), bounds())
            .expect("valid foreign prefix can be inspected");
        assert!(writer.append(&fixture).is_err());

        let root = TestRoot::new("reordered-prefix");
        create_empty_files(root.path());
        let mut reordered = fixture.rows[1];
        reordered.rank = 0;
        reordered.row_id = reordered.derive_row_id();
        std::fs::write(
            root.path().join(ROW_FILE),
            reordered.encode().expect("reordered row encodes"),
        )
        .expect("write reordered orphan");
        let mut writer = SelectionV5Ledger::open_write(root.path(), bounds())
            .expect("structurally canonical reordered prefix opens");
        assert!(writer.append(&fixture).is_err());
    }

    #[test]
    fn orphan_prefix_source_identity_mutations_refuse() {
        let fixture = prepared(30);
        assert_mutated_orphan_source_refuses("orphan-population-order", &fixture, |row| {
            row.population_ordered_digest = digest(210);
        });
        assert_mutated_orphan_source_refuses("orphan-disposition-order", &fixture, |row| {
            row.ordered_disposition_digest = digest(211);
        });
        assert_mutated_orphan_source_refuses("orphan-nifty-authority", &fixture, |row| {
            row.nifty_authority_id = digest(212);
        });
        assert_mutated_orphan_source_refuses("orphan-banknifty-authority", &fixture, |row| {
            row.banknifty_authority_id = digest(213);
        });
    }

    #[test]
    fn resealed_non_strongest_first_committed_block_refuses() {
        let fixture = prepared(30);
        let root = TestRoot::new("resealed-nonstrongest-first");
        create_empty_files(root.path());

        let mut reordered = fixture.rows.clone();
        reordered.swap(0, 1);
        for (rank, row) in reordered.iter_mut().enumerate() {
            row.rank = u32::try_from(rank).expect("fixture rank fits u32");
            row.row_id = row.derive_row_id();
            row.validate().expect("reranked row remains canonical");
        }
        assert!(ranked_winner(&reordered[0]) < ranked_winner(&reordered[1]));
        write_rows(root.path(), &reordered);

        let mut completion = fixture
            .expected_completion(0, 0)
            .expect("fixture Completion derives");
        completion.ordered_selected_digest =
            ordered_selected_digest(&reordered).expect("reordered digest derives");
        completion.completion_id = completion.derive_completion_id();
        completion
            .validate()
            .expect("resealed Completion is independently canonical");
        std::fs::write(
            root.path().join(COMPLETION_FILE),
            completion.encode().expect("resealed Completion encodes"),
        )
        .expect("write resealed Completion");

        assert!(SelectionV5Ledger::open_read(root.path(), bounds()).is_err());
    }

    #[test]
    fn corruption_ragged_files_replaced_children_and_replaced_root_fail_closed() {
        let root = TestRoot::new("corruption");
        let fixture = prepared(30);
        commit_prepared_for_test(root.path(), bounds(), &fixture).expect("fixture commits");
        let cached =
            SelectionV5Ledger::open_read(root.path(), bounds()).expect("cached reader opens");
        let row_path = root.path().join(ROW_FILE);
        let mut bytes = std::fs::read(&row_path).expect("read row bytes");
        bytes[200] ^= 1;
        std::fs::write(&row_path, bytes).expect("corrupt row file");
        assert!(cached.structural_receipt(&fixture.selection_id).is_err());
        assert!(SelectionV5Ledger::open_read(root.path(), bounds()).is_err());

        let root = TestRoot::new("ragged");
        create_empty_files(root.path());
        std::fs::write(root.path().join(ROW_FILE), [1_u8]).expect("write ragged row file");
        assert!(SelectionV5Ledger::open_read(root.path(), bounds()).is_err());

        let root = TestRoot::new("child-replacement");
        commit_prepared_for_test(root.path(), bounds(), &fixture).expect("fixture commits");
        let cached =
            SelectionV5Ledger::open_read(root.path(), bounds()).expect("cached reader opens");
        let row_path = root.path().join(ROW_FILE);
        let displaced = root.path().join("rows.displaced");
        let bytes = std::fs::read(&row_path).expect("read child before replacement");
        std::fs::rename(&row_path, &displaced).expect("displace child");
        std::fs::write(&row_path, bytes).expect("replace child at same path");
        assert!(cached.structural_receipt(&fixture.selection_id).is_err());

        let root = TestRoot::new("root-replacement");
        commit_prepared_for_test(root.path(), bounds(), &fixture).expect("fixture commits");
        let cached =
            SelectionV5Ledger::open_read(root.path(), bounds()).expect("cached reader opens");
        let displaced = root.path().with_extension("displaced");
        std::fs::rename(root.path(), &displaced).expect("displace root");
        std::fs::create_dir(root.path()).expect("replace root directory");
        assert!(cached.structural_receipt(&fixture.selection_id).is_err());
        std::fs::remove_dir(root.path()).expect("remove replacement root");
        std::fs::rename(&displaced, root.path()).expect("restore root for cleanup");
    }
}
