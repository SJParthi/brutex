//! Append-only Population V5 authority.
//!
//! V5 is the first Population format that consumes the exact Step-3 successor
//! join.  Every row retains the literal sealed Candidate record, the literal
//! sealed Population Admission V3 record (including Runner's full-precision
//! V3 evidence), and every semantic field of the receipt-last Finalization V3
//! projection.  It never decodes or assigns new meaning to Population V4.
//!
//! The only production preparation door consumes
//! [`CommittedStoredPopulationFinalizationV3`].  Caller-authored rows, detached
//! digests and structural receipts cannot mint an authority.  Rows are synced
//! before a separate Completion, and a commit becomes authority only after a
//! fresh read-only reopen reproduces every prepared byte.
//!
//! Sequential codec, hashing and persistence work is O(C) in Candidate count.
//! Duplicate/index operations use `HashSet`/`HashMap`, so their stated cost is
//! expected/amortized O(C), with a conservative O(C²) collision worst case;
//! retained row and uniqueness state is O(C).  Fixed-record offset arithmetic
//! alone is O(1). Generation validation hashes bounded files and is not O(1).

#![expect(
    dead_code,
    reason = "Population V5 and its Execution V3 successor remain crate-private until the all-rung coordinator consumes their authenticated authorities"
)]

use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;
#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
use std::os::unix::fs::OpenOptionsExt as _;

use brutex_core::blake3::Hasher;
use runner::exit_grid_policy::ExecutionDispositionV1;

use crate::candidate_universe::{
    AuthenticatedCandidatePopulationRowV1, CandidateExecutionParameterFactsV1,
    verify_population_v5_canonical_record as verify_candidate,
};
use crate::population::{InstrumentFamilyV1, TradeDirectionV1};
use crate::population_admission_v3::{
    AdmissionV3Family, AdmissionV3Status, POPULATION_ADMISSION_V3_DECISION_BYTES,
    PopulationAdmissionV3DecisionProjection, PopulationAdmissionV3EmbeddedProjection,
    verify_population_v5_canonical_record as verify_admission,
};
use crate::population_finalization_v3::{
    PopulationFinalizationV3RowProjection, PopulationFinalizationV3StructuralReceipt,
};
use crate::step3_orchestrator::{
    CommittedStoredPopulationFinalizationV3, PopulationV5Input,
    StoredCandidateExecutionReplayPairV1,
};

/// Bytes in one canonical Population V5 row.
pub(crate) const POPULATION_V5_ROW_BYTES: usize = 4_096;
/// Bytes in one receipt-last Population V5 Completion.
pub(crate) const POPULATION_V5_COMPLETION_BYTES: usize = 1_024;

const VERSION: u32 = 5;
const ROW_MAGIC: [u8; 16] = *b"BTX-POPV5-ROW\0\0\0";
const COMPLETION_MAGIC: [u8; 16] = *b"BTX-POPV5-CMP\0\0\0";
const ROW_DOMAIN: u32 = 1;
const COMPLETION_DOMAIN: u32 = 2;
const SEAL_BYTES: usize = 32;
const ROW_PAYLOAD_BYTES: usize = POPULATION_V5_ROW_BYTES - SEAL_BYTES;
const COMPLETION_PAYLOAD_BYTES: usize = POPULATION_V5_COMPLETION_BYTES - SEAL_BYTES;
const CANDIDATE_RECORD_BYTES: usize = 480;
const FINALIZATION_SNAPSHOT_BYTES: usize = 1_088;
const READ_CHUNK_BYTES: usize = 16 * 1_024;

const ROW_ID_DOMAIN: &[u8] = b"brutex-population-v5-row-id\0";
const POPULATION_ID_DOMAIN: &[u8] = b"brutex-population-v5-id\0";
const ORDERED_ROWS_DOMAIN: &[u8] = b"brutex-population-v5-ordered-rows\0";
const COMPLETION_ID_DOMAIN: &[u8] = b"brutex-population-v5-completion-id\0";
const ROW_SEAL_DOMAIN: &[u8] = b"brutex-population-v5-row-seal\0";
const COMPLETION_SEAL_DOMAIN: &[u8] = b"brutex-population-v5-completion-seal\0";
const FILE_GENERATION_DOMAIN: &[u8] = b"brutex-population-v5-file-generation\0";

const ROW_FILE: &str = "population-v5.bin";
const COMPLETION_FILE: &str = "population-completions-v5.bin";
const LOCK_FILE: &str = "population-v5.lock";
const LOCK_MAX_BYTES: u64 = 0;

#[cfg(any(target_os = "android", target_os = "linux"))]
const O_NOFOLLOW_FLAG: i32 = 0x20_000;
#[cfg(target_os = "macos")]
const O_NOFOLLOW_FLAG: i32 = 0x100;

const _: () = assert!(ROW_PAYLOAD_BYTES + SEAL_BYTES == POPULATION_V5_ROW_BYTES);
const _: () = assert!(COMPLETION_PAYLOAD_BYTES + SEAL_BYTES == POPULATION_V5_COMPLETION_BYTES);

/// Operator-facing refusal at the Population V5 boundary.
pub(crate) type PopulationV5Refusal = String;

/// Explicit physical ceilings for Population V5.
///
/// There is deliberately no `Default`.  Every caller states record, byte and
/// per-Population ceilings; excess input is refused rather than sampled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationV5Bounds {
    row_records: u64,
    row_bytes: u64,
    completion_records: u64,
    completion_bytes: u64,
    rows_per_population: u64,
}

impl PopulationV5Bounds {
    /// Constructs explicit nonzero ceilings.
    ///
    /// # Errors
    ///
    /// Refuses zero, arithmetic overflow, an impossible byte ceiling, or a
    /// per-Population ceiling above the total row ceiling.
    pub(crate) fn new(
        max_row_records: u64,
        max_row_bytes: u64,
        max_completion_records: u64,
        max_completion_bytes: u64,
        max_rows_per_population: u64,
    ) -> Result<Self, PopulationV5Refusal> {
        for (name, value) in [
            ("row records", max_row_records),
            ("row bytes", max_row_bytes),
            ("Completion records", max_completion_records),
            ("Completion bytes", max_completion_bytes),
            ("rows per Population", max_rows_per_population),
        ] {
            if value == 0 {
                return Err(format!("Population V5 maximum {name} must be nonzero"));
            }
        }
        if max_rows_per_population > max_row_records {
            return Err(format!(
                "Population V5 per-Population maximum {max_rows_per_population} exceeds total row maximum {max_row_records}"
            ));
        }
        let required_rows = max_row_records
            .checked_mul(POPULATION_V5_ROW_BYTES as u64)
            .ok_or_else(|| "Population V5 row byte ceiling overflowed".to_owned())?;
        if max_row_bytes < required_rows {
            return Err(format!(
                "Population V5 row byte maximum {max_row_bytes} cannot hold {max_row_records} records ({required_rows} bytes)"
            ));
        }
        let required_completions = max_completion_records
            .checked_mul(POPULATION_V5_COMPLETION_BYTES as u64)
            .ok_or_else(|| "Population V5 Completion byte ceiling overflowed".to_owned())?;
        if max_completion_bytes < required_completions {
            return Err(format!(
                "Population V5 Completion byte maximum {max_completion_bytes} cannot hold {max_completion_records} records ({required_completions} bytes)"
            ));
        }
        Ok(Self {
            row_records: max_row_records,
            row_bytes: max_row_bytes,
            completion_records: max_completion_records,
            completion_bytes: max_completion_bytes,
            rows_per_population: max_rows_per_population,
        })
    }

    const fn max_row_records(self) -> u64 {
        self.row_records
    }

    const fn max_row_bytes(self) -> u64 {
        self.row_bytes
    }

    const fn max_completion_records(self) -> u64 {
        self.completion_records
    }

    const fn max_completion_bytes(self) -> u64 {
        self.completion_bytes
    }

    const fn max_rows_per_population(self) -> u64 {
        self.rows_per_population
    }
}

/// Immutable typed Finalization V3 semantics authenticated inside one V5 row.
///
/// Fields remain private and no caller constructor exists.  A downstream
/// successor can inspect this projection only after the retained V5 authority
/// authenticates the enclosing fixed record and all nested Candidate,
/// Admission and Finalization joins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PopulationV5FinalizationProjection {
    finalization_id: [u8; 32],
    candidate_universe_id: [u8; 32],
    candidate_completion_digest: [u8; 32],
    candidate_semantic_id: [u8; 32],
    candidate_row_digest: [u8; 32],
    pre_admission_authority_id: [u8; 32],
    statistics_audit_id: [u8; 32],
    statistics_completion_digest: [u8; 32],
    statistics_period_digest: [u8; 32],
    statistics_split_digest: [u8; 32],
    search_pair_id: [u8; 32],
    search_member_id: [u8; 32],
    search_signal_digest: [u8; 32],
    search_signal_bars: u64,
    search_signal_first_ts_micros: i64,
    search_signal_last_ts_micros: i64,
    search_signal_column_digest: [u8; 32],
    search_policy_id: [u8; 32],
    search_full_grid_id: [u8; 32],
    search_long_policy_id: [u8; 32],
    search_long_resolution_id: [u8; 32],
    search_short_policy_id: [u8; 32],
    search_short_resolution_id: [u8; 32],
    search_family_id: [u8; 32],
    search_walk_id: [u8; 32],
    search_fold_count: u64,
    search_decided_folds: u64,
    search_profitable_oos_folds: u64,
    search_aggregate_oos_paisa: i64,
    search_evaluated_population_count: u64,
    paired_base_id: [u8; 32],
    base_completion_id: [u8; 32],
    base_evidence_id: [u8; 32],
    admission_block_id: [u8; 32],
    admission_completion_id: [u8; 32],
    admission_runner_decision_digest: [u8; 32],
    admission_evidence_digest: [u8; 32],
    admission_verdict_digest: [u8; 32],
    admission_decision_id: [u8; 32],
    finalization_row_id: [u8; 32],
}

impl PopulationV5FinalizationProjection {
    fn from_projection(value: &PopulationFinalizationV3RowProjection) -> Self {
        Self {
            finalization_id: value.finalization_id(),
            candidate_universe_id: value.candidate_universe_id(),
            candidate_completion_digest: value.candidate_completion_digest(),
            candidate_semantic_id: value.candidate_semantic_id(),
            candidate_row_digest: value.candidate_row_digest(),
            pre_admission_authority_id: value.pre_admission_authority_id(),
            statistics_audit_id: value.statistics_audit_id(),
            statistics_completion_digest: value.statistics_completion_digest(),
            statistics_period_digest: value.statistics_period_digest(),
            statistics_split_digest: value.statistics_split_digest(),
            search_pair_id: value.search_pair_id(),
            search_member_id: value.search_member_id(),
            search_signal_digest: value.search_signal_digest(),
            search_signal_bars: value.search_signal_bars(),
            search_signal_first_ts_micros: value.search_signal_first_ts_micros(),
            search_signal_last_ts_micros: value.search_signal_last_ts_micros(),
            search_signal_column_digest: value.search_signal_column_digest(),
            search_policy_id: value.search_policy_id(),
            search_full_grid_id: value.search_full_grid_id(),
            search_long_policy_id: value.search_long_policy_id(),
            search_long_resolution_id: value.search_long_resolution_id(),
            search_short_policy_id: value.search_short_policy_id(),
            search_short_resolution_id: value.search_short_resolution_id(),
            search_family_id: value.search_family_id(),
            search_walk_id: value.search_walk_id(),
            search_fold_count: value.search_fold_count(),
            search_decided_folds: value.search_decided_folds(),
            search_profitable_oos_folds: value.search_profitable_oos_folds(),
            search_aggregate_oos_paisa: value.search_aggregate_oos_paisa(),
            search_evaluated_population_count: value.search_evaluated_population_count(),
            paired_base_id: value.paired_base_id(),
            base_completion_id: value.base_completion_id(),
            base_evidence_id: value.base_evidence_id(),
            admission_block_id: value.admission_block_id(),
            admission_completion_id: value.admission_completion_id(),
            admission_runner_decision_digest: value.admission_runner_decision_digest(),
            admission_evidence_digest: value.admission_evidence_digest(),
            admission_verdict_digest: value.admission_verdict_digest(),
            admission_decision_id: value.admission_decision_id(),
            finalization_row_id: value.row_id(),
        }
    }

    #[must_use]
    pub(crate) const fn finalization_id(&self) -> [u8; 32] {
        self.finalization_id
    }

    #[must_use]
    pub(crate) const fn candidate_universe_id(&self) -> [u8; 32] {
        self.candidate_universe_id
    }

    #[must_use]
    pub(crate) const fn candidate_completion_digest(&self) -> [u8; 32] {
        self.candidate_completion_digest
    }

    #[must_use]
    pub(crate) const fn candidate_semantic_id(&self) -> [u8; 32] {
        self.candidate_semantic_id
    }

    #[must_use]
    pub(crate) const fn candidate_row_digest(&self) -> [u8; 32] {
        self.candidate_row_digest
    }

    #[must_use]
    pub(crate) const fn pre_admission_authority_id(&self) -> [u8; 32] {
        self.pre_admission_authority_id
    }

    #[must_use]
    pub(crate) const fn statistics_audit_id(&self) -> [u8; 32] {
        self.statistics_audit_id
    }

    #[must_use]
    pub(crate) const fn statistics_completion_digest(&self) -> [u8; 32] {
        self.statistics_completion_digest
    }

    #[must_use]
    pub(crate) const fn statistics_period_digest(&self) -> [u8; 32] {
        self.statistics_period_digest
    }

    #[must_use]
    pub(crate) const fn statistics_split_digest(&self) -> [u8; 32] {
        self.statistics_split_digest
    }

    #[must_use]
    pub(crate) const fn search_pair_id(&self) -> [u8; 32] {
        self.search_pair_id
    }

    #[must_use]
    pub(crate) const fn search_member_id(&self) -> [u8; 32] {
        self.search_member_id
    }

    #[must_use]
    pub(crate) const fn search_signal_digest(&self) -> [u8; 32] {
        self.search_signal_digest
    }

    #[must_use]
    pub(crate) const fn search_signal_bars(&self) -> u64 {
        self.search_signal_bars
    }

    #[must_use]
    pub(crate) const fn search_signal_first_ts_micros(&self) -> i64 {
        self.search_signal_first_ts_micros
    }

    #[must_use]
    pub(crate) const fn search_signal_last_ts_micros(&self) -> i64 {
        self.search_signal_last_ts_micros
    }

    #[must_use]
    pub(crate) const fn search_signal_column_digest(&self) -> [u8; 32] {
        self.search_signal_column_digest
    }

    #[must_use]
    pub(crate) const fn search_policy_id(&self) -> [u8; 32] {
        self.search_policy_id
    }

    #[must_use]
    pub(crate) const fn search_full_grid_id(&self) -> [u8; 32] {
        self.search_full_grid_id
    }

    #[must_use]
    pub(crate) const fn search_long_policy_id(&self) -> [u8; 32] {
        self.search_long_policy_id
    }

    #[must_use]
    pub(crate) const fn search_long_resolution_id(&self) -> [u8; 32] {
        self.search_long_resolution_id
    }

    #[must_use]
    pub(crate) const fn search_short_policy_id(&self) -> [u8; 32] {
        self.search_short_policy_id
    }

    #[must_use]
    pub(crate) const fn search_short_resolution_id(&self) -> [u8; 32] {
        self.search_short_resolution_id
    }

    #[must_use]
    pub(crate) const fn search_family_id(&self) -> [u8; 32] {
        self.search_family_id
    }

    #[must_use]
    pub(crate) const fn search_walk_id(&self) -> [u8; 32] {
        self.search_walk_id
    }

    #[must_use]
    pub(crate) const fn search_fold_count(&self) -> u64 {
        self.search_fold_count
    }

    #[must_use]
    pub(crate) const fn search_decided_folds(&self) -> u64 {
        self.search_decided_folds
    }

    #[must_use]
    pub(crate) const fn search_profitable_oos_folds(&self) -> u64 {
        self.search_profitable_oos_folds
    }

    #[must_use]
    pub(crate) const fn search_aggregate_oos_paisa(&self) -> i64 {
        self.search_aggregate_oos_paisa
    }

    #[must_use]
    pub(crate) const fn search_evaluated_population_count(&self) -> u64 {
        self.search_evaluated_population_count
    }

    #[must_use]
    pub(crate) const fn paired_base_id(&self) -> [u8; 32] {
        self.paired_base_id
    }

    #[must_use]
    pub(crate) const fn base_completion_id(&self) -> [u8; 32] {
        self.base_completion_id
    }

    #[must_use]
    pub(crate) const fn base_evidence_id(&self) -> [u8; 32] {
        self.base_evidence_id
    }

    #[must_use]
    pub(crate) const fn admission_block_id(&self) -> [u8; 32] {
        self.admission_block_id
    }

    #[must_use]
    pub(crate) const fn admission_completion_id(&self) -> [u8; 32] {
        self.admission_completion_id
    }

    #[must_use]
    pub(crate) const fn admission_runner_decision_digest(&self) -> [u8; 32] {
        self.admission_runner_decision_digest
    }

    #[must_use]
    pub(crate) const fn admission_evidence_digest(&self) -> [u8; 32] {
        self.admission_evidence_digest
    }

    #[must_use]
    pub(crate) const fn admission_verdict_digest(&self) -> [u8; 32] {
        self.admission_verdict_digest
    }

    #[must_use]
    pub(crate) const fn admission_decision_id(&self) -> [u8; 32] {
        self.admission_decision_id
    }

    #[must_use]
    pub(crate) const fn finalization_row_id(&self) -> [u8; 32] {
        self.finalization_row_id
    }

    fn encode(&self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationV5Refusal> {
        for value in [
            self.finalization_id,
            self.candidate_universe_id,
            self.candidate_completion_digest,
            self.candidate_semantic_id,
            self.candidate_row_digest,
            self.pre_admission_authority_id,
            self.statistics_audit_id,
            self.statistics_completion_digest,
            self.statistics_period_digest,
            self.statistics_split_digest,
            self.search_pair_id,
            self.search_member_id,
            self.search_signal_digest,
        ] {
            writer.array(&value)?;
        }
        writer.u64(self.search_signal_bars)?;
        writer.i64(self.search_signal_first_ts_micros)?;
        writer.i64(self.search_signal_last_ts_micros)?;
        for value in [
            self.search_signal_column_digest,
            self.search_policy_id,
            self.search_full_grid_id,
            self.search_long_policy_id,
            self.search_long_resolution_id,
            self.search_short_policy_id,
            self.search_short_resolution_id,
            self.search_family_id,
            self.search_walk_id,
        ] {
            writer.array(&value)?;
        }
        writer.u64(self.search_fold_count)?;
        writer.u64(self.search_decided_folds)?;
        writer.u64(self.search_profitable_oos_folds)?;
        writer.i64(self.search_aggregate_oos_paisa)?;
        writer.u64(self.search_evaluated_population_count)?;
        for value in [
            self.paired_base_id,
            self.base_completion_id,
            self.base_evidence_id,
            self.admission_block_id,
            self.admission_completion_id,
            self.admission_runner_decision_digest,
            self.admission_evidence_digest,
            self.admission_verdict_digest,
            self.admission_decision_id,
            self.finalization_row_id,
        ] {
            writer.array(&value)?;
        }
        Ok(())
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationV5Refusal> {
        Ok(Self {
            finalization_id: reader.array()?,
            candidate_universe_id: reader.array()?,
            candidate_completion_digest: reader.array()?,
            candidate_semantic_id: reader.array()?,
            candidate_row_digest: reader.array()?,
            pre_admission_authority_id: reader.array()?,
            statistics_audit_id: reader.array()?,
            statistics_completion_digest: reader.array()?,
            statistics_period_digest: reader.array()?,
            statistics_split_digest: reader.array()?,
            search_pair_id: reader.array()?,
            search_member_id: reader.array()?,
            search_signal_digest: reader.array()?,
            search_signal_bars: reader.u64()?,
            search_signal_first_ts_micros: reader.i64()?,
            search_signal_last_ts_micros: reader.i64()?,
            search_signal_column_digest: reader.array()?,
            search_policy_id: reader.array()?,
            search_full_grid_id: reader.array()?,
            search_long_policy_id: reader.array()?,
            search_long_resolution_id: reader.array()?,
            search_short_policy_id: reader.array()?,
            search_short_resolution_id: reader.array()?,
            search_family_id: reader.array()?,
            search_walk_id: reader.array()?,
            search_fold_count: reader.u64()?,
            search_decided_folds: reader.u64()?,
            search_profitable_oos_folds: reader.u64()?,
            search_aggregate_oos_paisa: reader.i64()?,
            search_evaluated_population_count: reader.u64()?,
            paired_base_id: reader.array()?,
            base_completion_id: reader.array()?,
            base_evidence_id: reader.array()?,
            admission_block_id: reader.array()?,
            admission_completion_id: reader.array()?,
            admission_runner_decision_digest: reader.array()?,
            admission_evidence_digest: reader.array()?,
            admission_verdict_digest: reader.array()?,
            admission_decision_id: reader.array()?,
            finalization_row_id: reader.array()?,
        })
    }

    fn canonical_bytes(&self) -> Result<[u8; FINALIZATION_SNAPSHOT_BYTES], PopulationV5Refusal> {
        let mut raw = [0_u8; FINALIZATION_SNAPSHOT_BYTES];
        let mut writer = FixedWriter::new(&mut raw);
        self.encode(&mut writer)?;
        writer.require_full("Population V5 Finalization snapshot")?;
        Ok(raw)
    }

    fn validate(
        &self,
        global_sequence: u64,
        family: AdmissionV3Family,
        family_sequence: u64,
        status: AdmissionV3Status,
    ) -> Result<(), PopulationV5Refusal> {
        for (name, value) in [
            ("Finalization identity", self.finalization_id),
            ("Candidate universe", self.candidate_universe_id),
            ("Candidate Completion", self.candidate_completion_digest),
            ("Candidate semantic", self.candidate_semantic_id),
            ("Candidate row", self.candidate_row_digest),
            ("Pre-Admission authority", self.pre_admission_authority_id),
            ("Statistics audit", self.statistics_audit_id),
            ("Statistics Completion", self.statistics_completion_digest),
            ("Statistics period", self.statistics_period_digest),
            ("Statistics split", self.statistics_split_digest),
            ("Search pair", self.search_pair_id),
            ("Search member", self.search_member_id),
            ("Search signal", self.search_signal_digest),
            ("Search signal column", self.search_signal_column_digest),
            ("Search policy", self.search_policy_id),
            ("Search full grid", self.search_full_grid_id),
            ("Search Long policy", self.search_long_policy_id),
            ("Search Long resolution", self.search_long_resolution_id),
            ("Search Short policy", self.search_short_policy_id),
            ("Search Short resolution", self.search_short_resolution_id),
            ("Search family", self.search_family_id),
            ("Search walk", self.search_walk_id),
            ("paired Base Evidence", self.paired_base_id),
            ("Base Completion", self.base_completion_id),
            ("Base evidence", self.base_evidence_id),
            ("Admission block", self.admission_block_id),
            ("Admission Completion", self.admission_completion_id),
            (
                "Admission Runner decision",
                self.admission_runner_decision_digest,
            ),
            ("Admission evidence", self.admission_evidence_digest),
            ("Admission verdict", self.admission_verdict_digest),
            ("Admission decision", self.admission_decision_id),
            ("Finalization row", self.finalization_row_id),
        ] {
            require_nonzero(name, value)?;
        }
        if self.search_signal_bars == 0
            || self.search_signal_first_ts_micros > self.search_signal_last_ts_micros
        {
            return Err("Population V5 Finalization Search signal shape is invalid".to_owned());
        }
        if self.search_long_policy_id == self.search_short_policy_id
            || self.search_long_resolution_id == self.search_short_resolution_id
        {
            return Err("Population V5 Finalization Long/Short identities alias".to_owned());
        }
        if self.search_fold_count == 0
            || self.search_decided_folds > self.search_fold_count
            || self.search_profitable_oos_folds > self.search_decided_folds
        {
            return Err("Population V5 Finalization fold hierarchy is invalid".to_owned());
        }
        if (self.search_decided_folds == 0 && self.search_aggregate_oos_paisa != 0)
            || (self.search_profitable_oos_folds == 0 && self.search_aggregate_oos_paisa > 0)
            || (self.search_profitable_oos_folds == self.search_decided_folds
                && self.search_decided_folds > 0
                && self.search_aggregate_oos_paisa <= 0)
        {
            return Err("Population V5 Finalization outcome hierarchy is invalid".to_owned());
        }
        if self.finalization_row_id
            != self.derive_v3_row_id(global_sequence, family, family_sequence, status)?
        {
            return Err(
                "Population V5 embedded Finalization V3 row identity does not reproduce".to_owned(),
            );
        }
        Ok(())
    }

    fn derive_v3_row_id(
        &self,
        global_sequence: u64,
        family: AdmissionV3Family,
        family_sequence: u64,
        status: AdmissionV3Status,
    ) -> Result<[u8; 32], PopulationV5Refusal> {
        const V3_IDENTITY_BYTES: usize = 1_056;
        let mut identity = [0_u8; V3_IDENTITY_BYTES];
        let mut writer = FixedWriter::new(&mut identity);
        writer.u64(global_sequence)?;
        writer.u8(family as u8)?;
        writer.zeros(7)?;
        writer.u64(family_sequence)?;
        writer.u8(status as u8)?;
        writer.zeros(7)?;
        for value in [
            self.candidate_universe_id,
            self.candidate_completion_digest,
            self.candidate_semantic_id,
            self.candidate_row_digest,
            self.pre_admission_authority_id,
            self.statistics_audit_id,
            self.statistics_completion_digest,
            self.statistics_period_digest,
            self.statistics_split_digest,
            self.search_pair_id,
            self.search_member_id,
            self.search_signal_digest,
        ] {
            writer.array(&value)?;
        }
        writer.u64(self.search_signal_bars)?;
        writer.i64(self.search_signal_first_ts_micros)?;
        writer.i64(self.search_signal_last_ts_micros)?;
        for value in [
            self.search_signal_column_digest,
            self.search_policy_id,
            self.search_full_grid_id,
            self.search_long_policy_id,
            self.search_long_resolution_id,
            self.search_short_policy_id,
            self.search_short_resolution_id,
            self.search_family_id,
            self.search_walk_id,
        ] {
            writer.array(&value)?;
        }
        writer.u64(self.search_fold_count)?;
        writer.u64(self.search_decided_folds)?;
        writer.u64(self.search_profitable_oos_folds)?;
        writer.i64(self.search_aggregate_oos_paisa)?;
        writer.u64(self.search_evaluated_population_count)?;
        for value in [
            self.paired_base_id,
            self.base_completion_id,
            self.base_evidence_id,
            self.admission_block_id,
            self.admission_completion_id,
            self.admission_runner_decision_digest,
            self.admission_evidence_digest,
            self.admission_verdict_digest,
            self.admission_decision_id,
        ] {
            writer.array(&value)?;
        }
        writer.require_full("embedded Finalization V3 row identity")?;
        Ok(hash_parts(
            b"brutex-population-finalization-v3-row-id\0",
            &[&3_u32.to_le_bytes(), &identity],
        ))
    }
}

/// One canonical Population V5 row before authentication by a retained ledger.
#[derive(Clone, Debug, PartialEq, Eq)]
struct PopulationV5RowRecord {
    population_id: [u8; 32],
    global_sequence: u64,
    family: AdmissionV3Family,
    family_sequence: u64,
    status: AdmissionV3Status,
    candidate_record: [u8; CANDIDATE_RECORD_BYTES],
    admission_record: [u8; POPULATION_ADMISSION_V3_DECISION_BYTES],
    finalization_completion_id: [u8; 32],
    finalization: PopulationV5FinalizationProjection,
    row_id: [u8; 32],
}

impl PopulationV5RowRecord {
    fn from_input(
        population_id: [u8; 32],
        input: &PopulationV5Input,
    ) -> Result<Self, PopulationV5Refusal> {
        let finalization = input.finalization();
        let mut row = Self {
            population_id,
            global_sequence: finalization.global_sequence(),
            family: finalization.family(),
            family_sequence: finalization.family_sequence(),
            status: finalization.status(),
            candidate_record: *input.candidate().canonical_record(),
            admission_record: *input.admission().canonical_record(),
            finalization_completion_id: input.finalization_completion_id(),
            finalization: PopulationV5FinalizationProjection::from_projection(finalization),
            row_id: [0; 32],
        };
        row.row_id = row.derive_row_id()?;
        row.validate()?;
        row.validate_live_admission_join(&input.admission().identity())?;
        Ok(row)
    }

    fn validate(&self) -> Result<(), PopulationV5Refusal> {
        require_nonzero("Population V5 identity", self.population_id)?;
        require_nonzero(
            "Population V5 Finalization Completion",
            self.finalization_completion_id,
        )?;
        require_nonzero("Population V5 row", self.row_id)?;

        let candidate = verify_candidate(&self.candidate_record)
            .map_err(|why| format!("Population V5 embedded Candidate refused: {why}"))?;
        let admission = verify_admission(&self.admission_record)
            .map_err(|why| format!("Population V5 embedded Admission V3 refused: {why}"))?;
        self.validate_join(&candidate, &admission)?;
        if self.row_id != self.derive_row_id()? {
            return Err("Population V5 row identity does not reproduce".to_owned());
        }
        Ok(())
    }

    /// Joins every source-authenticated Admission V3 field repeated by the
    /// Finalization snapshot. The literal 2,048-byte Admission decision does
    /// not contain these source preimages; it contains the block identity that
    /// commits to them. Therefore this check deliberately accepts only the
    /// nonconstructible live successor projection, never the detached embedded
    /// verifier. Fresh V5 reads obtain the same proof by re-preparing through
    /// the retained Finalization capability and comparing every V5 byte.
    fn validate_live_admission_join(
        &self,
        admission: &PopulationAdmissionV3DecisionProjection,
    ) -> Result<(), PopulationV5Refusal> {
        if self.global_sequence != admission.global_sequence()
            || self.family != admission.family()
            || self.family_sequence != admission.family_sequence()
            || self.status != admission.status()
            || self.finalization.candidate_universe_id != admission.candidate_universe_id()
            || self.finalization.candidate_completion_digest
                != admission.candidate_completion_digest()
            || self.finalization.candidate_semantic_id != admission.candidate_semantic_id()
            || self.finalization.candidate_row_digest != admission.candidate_row_digest()
            || self.finalization.pre_admission_authority_id
                != admission.pre_admission_authority_id()
            || self.finalization.statistics_audit_id != admission.statistics_audit_id()
            || self.finalization.statistics_completion_digest
                != admission.statistics_completion_digest()
            || self.finalization.statistics_period_digest != admission.statistics_period_digest()
            || self.finalization.statistics_split_digest != admission.statistics_split_digest()
            || self.finalization.search_pair_id != admission.search_pair_id()
            || self.finalization.search_member_id != admission.search_member_id()
            || self.finalization.search_signal_digest != admission.search_signal_digest()
            || self.finalization.search_signal_bars != admission.search_signal_bars()
            || self.finalization.search_signal_first_ts_micros
                != admission.search_signal_first_ts_micros()
            || self.finalization.search_signal_last_ts_micros
                != admission.search_signal_last_ts_micros()
            || self.finalization.search_signal_column_digest
                != admission.search_signal_column_digest()
            || self.finalization.search_policy_id != admission.search_policy_id()
            || self.finalization.search_full_grid_id != admission.search_full_grid_id()
            || self.finalization.search_long_policy_id != admission.search_long_policy_id()
            || self.finalization.search_long_resolution_id != admission.search_long_resolution_id()
            || self.finalization.search_short_policy_id != admission.search_short_policy_id()
            || self.finalization.search_short_resolution_id
                != admission.search_short_resolution_id()
            || self.finalization.search_family_id != admission.search_family_id()
            || self.finalization.search_walk_id != admission.search_walk_id()
            || self.finalization.search_fold_count != admission.search_fold_count()
            || self.finalization.search_decided_folds != admission.search_decided_folds()
            || self.finalization.search_profitable_oos_folds
                != admission.search_profitable_oos_folds()
            || self.finalization.search_aggregate_oos_paisa
                != admission.search_aggregate_oos_paisa()
            || self.finalization.search_evaluated_population_count
                != admission.search_evaluated_population_count()
            || self.finalization.paired_base_id != admission.paired_base_id()
            || self.finalization.base_completion_id != admission.base_completion_id()
            || self.finalization.base_evidence_id != admission.base_evidence_id()
            || self.finalization.admission_block_id != admission.block_id()
            || self.finalization.admission_completion_id != admission.completion_id()
            || self.finalization.admission_runner_decision_digest
                != admission.runner_decision_digest()
            || self.finalization.admission_evidence_digest != admission.runner_evidence_digest()
            || self.finalization.admission_verdict_digest != admission.runner_verdict_digest()
            || self.finalization.admission_decision_id != admission.decision_id()
        {
            return Err(
                "Population V5 live Admission V3 source projection does not exactly join Finalization V3"
                    .to_owned(),
            );
        }
        Ok(())
    }

    fn validate_join(
        &self,
        candidate: &AuthenticatedCandidatePopulationRowV1,
        admission: &PopulationAdmissionV3EmbeddedProjection,
    ) -> Result<(), PopulationV5Refusal> {
        let candidate_row = candidate.row();
        let candidate_family = admission_family(candidate_row.family());
        if admission.canonical_record() != &self.admission_record
            || candidate.canonical_record() != &self.candidate_record
        {
            return Err(
                "Population V5 nested canonical bytes changed during verification".to_owned(),
            );
        }
        if self.global_sequence != admission.global_sequence()
            || self.family != admission.family()
            || self.family != candidate_family
            || self.family_sequence != admission.family_sequence()
            || self.family_sequence != candidate_row.sequence()
            || self.status != admission.status()
        {
            return Err(
                "Population V5 Candidate/Admission/Finalization ordinal, family or status differs"
                    .to_owned(),
            );
        }
        self.finalization.validate(
            self.global_sequence,
            self.family,
            self.family_sequence,
            self.status,
        )?;
        if self.finalization.candidate_universe_id != candidate_row.universe_id()
            || self.finalization.candidate_semantic_id != candidate_row.candidate_semantic_digest()
            || self.finalization.candidate_row_digest != candidate.base_candidate_row_digest()
            || self.finalization.candidate_semantic_id != admission.candidate_semantic_id()
            || self.finalization.candidate_row_digest != admission.candidate_row_digest()
            || self.finalization.pre_admission_authority_id
                != admission.pre_admission_authority_id()
            || self.finalization.statistics_period_digest != admission.statistics_period_digest()
            || self.finalization.statistics_split_digest != admission.statistics_split_digest()
            || self.finalization.search_member_id != admission.family_search_member_id()
            || self.finalization.base_evidence_id != admission.base_evidence_id()
            || self.finalization.admission_block_id != admission.block_id()
            || self.finalization.admission_runner_decision_digest
                != admission.runner_decision_digest()
            || self.finalization.admission_evidence_digest != admission.runner_evidence_digest()
            || self.finalization.admission_verdict_digest != admission.runner_verdict_digest()
            || self.finalization.admission_decision_id != admission.decision_id()
        {
            return Err(
                "Population V5 embedded Candidate/Admission identities do not join Finalization V3"
                    .to_owned(),
            );
        }
        Ok(())
    }

    fn derive_row_id(&self) -> Result<[u8; 32], PopulationV5Refusal> {
        let snapshot = self.finalization.canonical_bytes()?;
        Ok(hash_parts(
            ROW_ID_DOMAIN,
            &[
                &VERSION.to_le_bytes(),
                &self.population_id,
                &self.global_sequence.to_le_bytes(),
                &[self.family as u8],
                &self.family_sequence.to_le_bytes(),
                &[self.status as u8],
                &self.candidate_record,
                &self.admission_record,
                &self.finalization_completion_id,
                &snapshot,
            ],
        ))
    }

    fn encode(&self) -> Result<[u8; POPULATION_V5_ROW_BYTES], PopulationV5Refusal> {
        self.validate()?;
        let mut raw = [0_u8; POPULATION_V5_ROW_BYTES];
        let (payload, seal) = raw.split_at_mut(ROW_PAYLOAD_BYTES);
        let mut writer = FixedWriter::new(payload);
        writer.array(&ROW_MAGIC)?;
        writer.u32(VERSION)?;
        writer.u32(ROW_DOMAIN)?;
        writer.array(&self.population_id)?;
        writer.u64(self.global_sequence)?;
        writer.u8(self.family as u8)?;
        writer.zeros(7)?;
        writer.u64(self.family_sequence)?;
        writer.u8(self.status as u8)?;
        writer.zeros(7)?;
        writer.array(&self.candidate_record)?;
        writer.array(&self.admission_record)?;
        writer.array(&self.finalization_completion_id)?;
        writer.array(&self.finalization.canonical_bytes()?)?;
        writer.array(&self.row_id)?;
        writer.zeros(writer.remaining())?;
        writer.require_full("Population V5 row payload")?;
        seal.copy_from_slice(&hash_parts(ROW_SEAL_DOMAIN, &[payload]));
        Ok(raw)
    }

    fn decode(raw: &[u8; POPULATION_V5_ROW_BYTES]) -> Result<Self, PopulationV5Refusal> {
        let (payload, seal) = raw.split_at(ROW_PAYLOAD_BYTES);
        if seal != hash_parts(ROW_SEAL_DOMAIN, &[payload]) {
            return Err("Population V5 row seal mismatch".to_owned());
        }
        let mut reader = FixedReader::new(payload);
        if reader.array::<16>()? != ROW_MAGIC {
            return Err("Population V5 row magic mismatch".to_owned());
        }
        let version = reader.u32()?;
        let domain = reader.u32()?;
        if version != VERSION || domain != ROW_DOMAIN {
            return Err(format!(
                "Population V5 row version/domain {version}/{domain} is unsupported"
            ));
        }
        let row = Self {
            population_id: reader.array()?,
            global_sequence: reader.u64()?,
            family: decode_family(reader.u8()?)?,
            family_sequence: {
                reader.require_zeros(7, "Population V5 row family reserve")?;
                reader.u64()?
            },
            status: decode_status(reader.u8()?)?,
            candidate_record: {
                reader.require_zeros(7, "Population V5 row status reserve")?;
                reader.array()?
            },
            admission_record: reader.array()?,
            finalization_completion_id: reader.array()?,
            finalization: PopulationV5FinalizationProjection::decode(&mut reader)?,
            row_id: reader.array()?,
        };
        reader.require_zeros(reader.remaining(), "Population V5 row trailing reserve")?;
        row.validate()?;
        if row.encode()? != *raw {
            return Err("Population V5 row is not byte-canonical".to_owned());
        }
        Ok(row)
    }
}

/// Authenticated successor row returned only by a retained V5 authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PopulationV5SuccessorRow {
    canonical_record: [u8; POPULATION_V5_ROW_BYTES],
    record: PopulationV5RowRecord,
    candidate: AuthenticatedCandidatePopulationRowV1,
    admission: PopulationAdmissionV3EmbeddedProjection,
}

impl PopulationV5SuccessorRow {
    fn authenticate(
        canonical_record: &[u8; POPULATION_V5_ROW_BYTES],
    ) -> Result<Self, PopulationV5Refusal> {
        let record = PopulationV5RowRecord::decode(canonical_record)?;
        let candidate = verify_candidate(&record.candidate_record)
            .map_err(|why| format!("Population V5 embedded Candidate refused: {why}"))?;
        let admission = verify_admission(&record.admission_record)
            .map_err(|why| format!("Population V5 embedded Admission V3 refused: {why}"))?;
        record.validate_join(&candidate, &admission)?;
        Ok(Self {
            canonical_record: *canonical_record,
            record,
            candidate,
            admission,
        })
    }

    #[must_use]
    pub(crate) const fn canonical_record(&self) -> &[u8; POPULATION_V5_ROW_BYTES] {
        &self.canonical_record
    }

    #[must_use]
    pub(crate) const fn population_id(&self) -> [u8; 32] {
        self.record.population_id
    }

    #[must_use]
    pub(crate) const fn global_sequence(&self) -> u64 {
        self.record.global_sequence
    }

    #[must_use]
    pub(crate) const fn family(&self) -> AdmissionV3Family {
        self.record.family
    }

    #[must_use]
    pub(crate) const fn family_sequence(&self) -> u64 {
        self.record.family_sequence
    }

    #[must_use]
    pub(crate) const fn status(&self) -> AdmissionV3Status {
        self.record.status
    }

    #[must_use]
    pub(crate) const fn row_id(&self) -> [u8; 32] {
        self.record.row_id
    }

    #[must_use]
    pub(crate) const fn candidate(&self) -> &AuthenticatedCandidatePopulationRowV1 {
        &self.candidate
    }

    #[must_use]
    pub(crate) const fn admission(&self) -> &PopulationAdmissionV3EmbeddedProjection {
        &self.admission
    }

    /// Complete typed Finalization V3 semantics authenticated by this V5 row.
    #[must_use]
    pub(crate) const fn finalization(&self) -> &PopulationV5FinalizationProjection {
        &self.record.finalization
    }

    #[must_use]
    pub(crate) const fn finalization_row_id(&self) -> [u8; 32] {
        self.finalization().finalization_row_id()
    }

    #[must_use]
    pub(crate) const fn finalization_completion_id(&self) -> [u8; 32] {
        self.record.finalization_completion_id
    }
}

/// One exact Population V5 row joined to Runner's disposition derived from
/// that row's still-live Candidate execution source.
///
/// Construction stays private to [`PopulationV5ExecutionV3SourceV1`]. A
/// caller cannot replace either side of the join or present a detached Runner
/// result to Execution V3.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PopulationV5ExecutionDispositionSourceV1 {
    population: PopulationV5SuccessorRow,
    disposition: ExecutionDispositionV1,
}

impl PopulationV5ExecutionDispositionSourceV1 {
    #[must_use]
    pub(crate) const fn population(&self) -> &PopulationV5SuccessorRow {
        &self.population
    }

    #[must_use]
    pub(crate) const fn disposition(&self) -> &ExecutionDispositionV1 {
        &self.disposition
    }
}

/// Opaque Population V5-to-Execution V3 production source.
///
/// This type can be minted only by [`CommittedStoredPopulationV5`], after it
/// reauthenticates the live Candidate/Admission/Finalization/V5 roots and
/// joins every literal V5 row to Runner's freshly rebuilt terminal
/// disposition. It exposes scalar codec facts but never bars, columns, masks,
/// grids, row constructors, or a detached digest input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PopulationV5ExecutionV3SourceV1 {
    receipt: PopulationV5StructuralReceipt,
    parameters: [CandidateExecutionParameterFactsV1; 4],
    rows: Vec<PopulationV5ExecutionDispositionSourceV1>,
}

impl PopulationV5ExecutionV3SourceV1 {
    #[must_use]
    pub(crate) const fn receipt(&self) -> PopulationV5StructuralReceipt {
        self.receipt
    }

    #[must_use]
    pub(crate) const fn parameters(&self) -> &[CandidateExecutionParameterFactsV1; 4] {
        &self.parameters
    }

    #[must_use]
    pub(crate) fn rows(&self) -> &[PopulationV5ExecutionDispositionSourceV1] {
        &self.rows
    }

    fn from_authenticated_sources(
        receipt: PopulationV5StructuralReceipt,
        rows: Vec<PopulationV5SuccessorRow>,
        pair: StoredCandidateExecutionReplayPairV1,
    ) -> Result<Self, PopulationV5Refusal> {
        if usize_to_u64(rows.len(), "Execution V3 source row count")? != receipt.row_count() {
            return Err(
                "Population V5 Execution V3 source row count differs from its Completion"
                    .to_owned(),
            );
        }
        let (nifty, banknifty) = pair.into_parts();
        let (nifty_receipt, [nifty_long, nifty_short], mut nifty_rows) = nifty.into_parts();
        let (banknifty_receipt, [banknifty_long, banknifty_short], mut banknifty_rows) =
            banknifty.into_parts();
        if nifty_receipt.family() != InstrumentFamilyV1::Nifty
            || banknifty_receipt.family() != InstrumentFamilyV1::BankNifty
            || nifty_receipt.row_count() != receipt.nifty_count()
            || banknifty_receipt.row_count() != receipt.banknifty_count()
            || usize_to_u64(nifty_rows.len(), "NIFTY Execution V3 replay rows")?
                != receipt.nifty_count()
            || usize_to_u64(banknifty_rows.len(), "BANKNIFTY Execution V3 replay rows")?
                != receipt.banknifty_count()
        {
            return Err(
                "Population V5 Execution V3 replay families/cardinalities differ from V5"
                    .to_owned(),
            );
        }
        if nifty_long.family != InstrumentFamilyV1::Nifty
            || nifty_long.direction != TradeDirectionV1::Long
            || nifty_short.family != InstrumentFamilyV1::Nifty
            || nifty_short.direction != TradeDirectionV1::Short
            || banknifty_long.family != InstrumentFamilyV1::BankNifty
            || banknifty_long.direction != TradeDirectionV1::Long
            || banknifty_short.family != InstrumentFamilyV1::BankNifty
            || banknifty_short.direction != TradeDirectionV1::Short
        {
            return Err(
                "Population V5 Execution V3 parameter order is not NIFTY Long/Short then BANKNIFTY Long/Short"
                    .to_owned(),
            );
        }
        let expected_total = nifty_rows
            .len()
            .checked_add(banknifty_rows.len())
            .ok_or_else(|| "Population V5 Execution V3 replay count overflowed".to_owned())?;
        if expected_total != rows.len() {
            return Err("Population V5 and Candidate Execution V3 replay counts differ".to_owned());
        }
        let mut projections = Vec::new();
        projections
            .try_reserve_exact(expected_total)
            .map_err(|why| {
                format!("cannot reserve Population V5 Execution V3 replay rows: {why}")
            })?;
        projections.append(&mut nifty_rows);
        projections.append(&mut banknifty_rows);
        let mut joined = Vec::new();
        joined.try_reserve_exact(expected_total).map_err(|why| {
            format!("cannot reserve Population V5 Execution V3 joined rows: {why}")
        })?;
        for (population, projection) in rows.into_iter().zip(projections) {
            let (candidate, disposition) = projection.into_parts();
            if population.candidate() != &candidate
                || population.population_id() != receipt.population_id()
                || population.global_sequence()
                    != usize_to_u64(joined.len(), "Execution V3 source ordinal")?
            {
                return Err(
                    "Population V5 row does not join its exact Candidate Execution V3 replay row"
                        .to_owned(),
                );
            }
            let candidate_row = candidate.row();
            let expected_family = match population.family() {
                AdmissionV3Family::Nifty => InstrumentFamilyV1::Nifty,
                AdmissionV3Family::BankNifty => InstrumentFamilyV1::BankNifty,
            };
            let expected_universe = match expected_family {
                InstrumentFamilyV1::Nifty => nifty_receipt.universe_id(),
                InstrumentFamilyV1::BankNifty => banknifty_receipt.universe_id(),
            };
            if candidate_row.family() != expected_family
                || candidate_row.universe_id() != expected_universe
                || population.family_sequence() != candidate_row.sequence()
            {
                return Err(
                    "Population V5 family/sequence does not join the authenticated Candidate block"
                        .to_owned(),
                );
            }
            joined.push(PopulationV5ExecutionDispositionSourceV1 {
                population,
                disposition,
            });
        }
        Ok(Self {
            receipt,
            parameters: [nifty_long, nifty_short, banknifty_long, banknifty_short],
            rows: joined,
        })
    }
}

fn admission_family(family: InstrumentFamilyV1) -> AdmissionV3Family {
    match family {
        InstrumentFamilyV1::Nifty => AdmissionV3Family::Nifty,
        InstrumentFamilyV1::BankNifty => AdmissionV3Family::BankNifty,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceFinalizationV3Receipt {
    finalization_id: [u8; 32],
    admission_block_id: [u8; 32],
    admission_completion_id: [u8; 32],
    row_count: u64,
    nifty_row_count: u64,
    banknifty_row_count: u64,
    admitted_count: u64,
    rejected_count: u64,
    unmeasured_count: u64,
    refused_count: u64,
    ordered_row_digest: [u8; 32],
    completion_id: [u8; 32],
}

impl From<PopulationFinalizationV3StructuralReceipt> for SourceFinalizationV3Receipt {
    fn from(value: PopulationFinalizationV3StructuralReceipt) -> Self {
        Self {
            finalization_id: value.finalization_id(),
            admission_block_id: value.admission_block_id(),
            admission_completion_id: value.admission_completion_id(),
            row_count: value.row_count(),
            nifty_row_count: value.nifty_row_count(),
            banknifty_row_count: value.banknifty_row_count(),
            admitted_count: value.admitted_count(),
            rejected_count: value.rejected_count(),
            unmeasured_count: value.unmeasured_count(),
            refused_count: value.refused_count(),
            ordered_row_digest: value.ordered_row_digest(),
            completion_id: value.completion_id(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedPopulationV5 {
    source: SourceFinalizationV3Receipt,
    population_id: [u8; 32],
    rows: Vec<PopulationV5RowRecord>,
    nifty_count: u64,
    banknifty_count: u64,
    admitted_count: u64,
    rejected_count: u64,
    unmeasured_count: u64,
    refused_count: u64,
}

impl PreparedPopulationV5 {
    /// The only production preparation door.
    ///
    /// The exact three-ledger successor join is requested directly from the
    /// retained Finalization V3 capability. There is intentionally no
    /// constructor from caller-authored rows, vectors, digests or receipts.
    fn from_authority(
        source: &mut CommittedStoredPopulationFinalizationV3,
        bounds: PopulationV5Bounds,
    ) -> Result<Self, PopulationV5Refusal> {
        let receipt = source.finalization_receipt();
        if receipt.row_count() == 0 || receipt.row_count() > bounds.max_rows_per_population() {
            return Err(format!(
                "Population V5 source receipt count {} is outside explicit bound 1..={}",
                receipt.row_count(),
                bounds.max_rows_per_population()
            ));
        }
        let inputs = source
            .population_v5_inputs()
            .map_err(|why| format!("Population V5 source join refused: {why}"))?;
        let count = u64::try_from(inputs.len())
            .map_err(|_| "Population V5 input count does not fit u64".to_owned())?;
        if count == 0 || count > bounds.max_rows_per_population() {
            return Err(format!(
                "Population V5 source produced {count} rows outside explicit bound 1..={}",
                bounds.max_rows_per_population()
            ));
        }
        if count != receipt.row_count() {
            return Err(format!(
                "Population V5 source join returned {count} rows for Finalization receipt count {}",
                receipt.row_count()
            ));
        }
        let source_receipt = SourceFinalizationV3Receipt::from(receipt);
        let population_id = derive_population_id(&source_receipt);
        require_nonzero("Population V5 identity", population_id)?;
        let mut rows = Vec::new();
        rows.try_reserve_exact(inputs.len())
            .map_err(|why| format!("cannot reserve Population V5 rows: {why}"))?;
        for input in &inputs {
            rows.push(PopulationV5RowRecord::from_input(population_id, input)?);
        }
        let prepared = Self {
            source: source_receipt,
            population_id,
            rows,
            nifty_count: receipt.nifty_row_count(),
            banknifty_count: receipt.banknifty_row_count(),
            admitted_count: receipt.admitted_count(),
            rejected_count: receipt.rejected_count(),
            unmeasured_count: receipt.unmeasured_count(),
            refused_count: receipt.refused_count(),
        };
        prepared.validate(bounds)?;
        Ok(prepared)
    }

    fn validate(&self, bounds: PopulationV5Bounds) -> Result<(), PopulationV5Refusal> {
        self.validate_counts(bounds)?;
        self.validate_rows()
    }

    fn validate_counts(&self, bounds: PopulationV5Bounds) -> Result<(), PopulationV5Refusal> {
        require_nonzero("Population V5 identity", self.population_id)?;
        let count = u64::try_from(self.rows.len())
            .map_err(|_| "Population V5 row count does not fit u64".to_owned())?;
        if count == 0 || count > bounds.max_rows_per_population() {
            return Err(format!(
                "Population V5 row count {count} exceeds explicit per-Population bound {}",
                bounds.max_rows_per_population()
            ));
        }
        let paired = self
            .nifty_count
            .checked_add(self.banknifty_count)
            .ok_or_else(|| "Population V5 family count overflowed".to_owned())?;
        let classified = checked_sum(
            &[
                self.admitted_count,
                self.rejected_count,
                self.unmeasured_count,
                self.refused_count,
            ],
            "Population V5 status counts",
        )?;
        if paired != count
            || classified != count
            || self.source.row_count != count
            || self.source.nifty_row_count != self.nifty_count
            || self.source.banknifty_row_count != self.banknifty_count
            || self.source.admitted_count != self.admitted_count
            || self.source.rejected_count != self.rejected_count
            || self.source.unmeasured_count != self.unmeasured_count
            || self.source.refused_count != self.refused_count
            || derive_population_id(&self.source) != self.population_id
        {
            return Err(
                "Population V5 prepared counts or source identity differ from Finalization V3"
                    .to_owned(),
            );
        }
        Ok(())
    }

    fn validate_rows(&self) -> Result<(), PopulationV5Refusal> {
        let capacity = self.rows.len();
        let mut row_ids = bounded_set(capacity, "Population V5 row identities")?;
        let mut candidate_ids = bounded_set(capacity, "Population V5 Candidate identities")?;
        let mut candidate_rows = bounded_set(capacity, "Population V5 Candidate row digests")?;
        let mut admission_ids = bounded_set(capacity, "Population V5 Admission decisions")?;
        let mut finalization_rows = bounded_set(capacity, "Population V5 Finalization rows")?;
        let mut status_counts = [0_u64; 4];
        let mut nifty_universe = None;
        let mut banknifty_universe = None;
        for (index, row) in self.rows.iter().enumerate() {
            row.validate()?;
            let global_sequence = u64::try_from(index)
                .map_err(|_| "Population V5 ordinal does not fit u64".to_owned())?;
            let (expected_family, expected_family_sequence, family_universe) =
                if global_sequence < self.nifty_count {
                    (
                        AdmissionV3Family::Nifty,
                        global_sequence,
                        &mut nifty_universe,
                    )
                } else {
                    (
                        AdmissionV3Family::BankNifty,
                        global_sequence
                            .checked_sub(self.nifty_count)
                            .ok_or_else(|| "Population V5 family ordinal underflowed".to_owned())?,
                        &mut banknifty_universe,
                    )
                };
            if row.population_id != self.population_id
                || row.global_sequence != global_sequence
                || row.family != expected_family
                || row.family_sequence != expected_family_sequence
                || row.finalization.finalization_id != self.source.finalization_id
                || row.finalization.admission_block_id != self.source.admission_block_id
                || row.finalization.admission_completion_id != self.source.admission_completion_id
                || row.finalization_completion_id != self.source.completion_id
            {
                return Err(format!(
                    "Population V5 row {global_sequence} violates canonical source order or identity"
                ));
            }
            match family_universe {
                Some(value) if *value != row.finalization.candidate_universe_id => {
                    return Err(format!(
                        "Population V5 family {:?} changes Candidate universe at row {global_sequence}",
                        row.family
                    ));
                }
                None => *family_universe = Some(row.finalization.candidate_universe_id),
                Some(_) => {}
            }
            if !row_ids.insert(row.row_id)
                || !candidate_ids.insert(row.finalization.candidate_semantic_id)
                || !candidate_rows.insert(row.finalization.candidate_row_digest)
                || !admission_ids.insert(row.finalization.admission_decision_id)
                || !finalization_rows.insert(row.finalization.finalization_row_id)
            {
                return Err(format!(
                    "Population V5 row {global_sequence} duplicates a successor identity"
                ));
            }
            increment_status_count(&mut status_counts, row.status, "prepared")?;
        }
        if status_counts
            != [
                self.admitted_count,
                self.rejected_count,
                self.unmeasured_count,
                self.refused_count,
            ]
        {
            return Err("Population V5 row statuses differ from source counts".to_owned());
        }
        Ok(())
    }

    fn expected_completion(
        &self,
        block_sequence: u64,
        first_row_record: u64,
    ) -> Result<PopulationV5CompletionRecord, PopulationV5Refusal> {
        let mut completion = PopulationV5CompletionRecord {
            block_sequence,
            first_row_record,
            population_id: self.population_id,
            source_finalization_id: self.source.finalization_id,
            source_finalization_completion_id: self.source.completion_id,
            source_finalization_ordered_row_digest: self.source.ordered_row_digest,
            source_admission_block_id: self.source.admission_block_id,
            source_admission_completion_id: self.source.admission_completion_id,
            row_count: u64::try_from(self.rows.len())
                .map_err(|_| "Population V5 row count does not fit u64".to_owned())?,
            nifty_count: self.nifty_count,
            banknifty_count: self.banknifty_count,
            admitted_count: self.admitted_count,
            rejected_count: self.rejected_count,
            unmeasured_count: self.unmeasured_count,
            refused_count: self.refused_count,
            ordered_row_digest: ordered_row_digest(&self.rows)?,
            completion_id: [0; 32],
        };
        completion.completion_id = completion.derive_completion_id();
        completion.validate()?;
        Ok(completion)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PopulationV5CompletionRecord {
    block_sequence: u64,
    first_row_record: u64,
    population_id: [u8; 32],
    source_finalization_id: [u8; 32],
    source_finalization_completion_id: [u8; 32],
    source_finalization_ordered_row_digest: [u8; 32],
    source_admission_block_id: [u8; 32],
    source_admission_completion_id: [u8; 32],
    row_count: u64,
    nifty_count: u64,
    banknifty_count: u64,
    admitted_count: u64,
    rejected_count: u64,
    unmeasured_count: u64,
    refused_count: u64,
    ordered_row_digest: [u8; 32],
    completion_id: [u8; 32],
}

impl PopulationV5CompletionRecord {
    fn validate(&self) -> Result<(), PopulationV5Refusal> {
        for (name, value) in [
            ("Population V5 identity", self.population_id),
            (
                "source Finalization V3 identity",
                self.source_finalization_id,
            ),
            (
                "source Finalization V3 Completion",
                self.source_finalization_completion_id,
            ),
            (
                "source Finalization V3 ordered rows",
                self.source_finalization_ordered_row_digest,
            ),
            ("source Admission V3 block", self.source_admission_block_id),
            (
                "source Admission V3 Completion",
                self.source_admission_completion_id,
            ),
            ("ordered Population V5 rows", self.ordered_row_digest),
            ("Population V5 Completion", self.completion_id),
        ] {
            require_nonzero(name, value)?;
        }
        if self.row_count == 0
            || self
                .nifty_count
                .checked_add(self.banknifty_count)
                .ok_or_else(|| "Population V5 family count overflowed".to_owned())?
                != self.row_count
            || checked_sum(
                &[
                    self.admitted_count,
                    self.rejected_count,
                    self.unmeasured_count,
                    self.refused_count,
                ],
                "Population V5 Completion status counts",
            )? != self.row_count
        {
            return Err("Population V5 Completion counts do not cover every row".to_owned());
        }
        let source = SourceFinalizationV3Receipt {
            finalization_id: self.source_finalization_id,
            admission_block_id: self.source_admission_block_id,
            admission_completion_id: self.source_admission_completion_id,
            row_count: self.row_count,
            nifty_row_count: self.nifty_count,
            banknifty_row_count: self.banknifty_count,
            admitted_count: self.admitted_count,
            rejected_count: self.rejected_count,
            unmeasured_count: self.unmeasured_count,
            refused_count: self.refused_count,
            ordered_row_digest: self.source_finalization_ordered_row_digest,
            completion_id: self.source_finalization_completion_id,
        };
        if derive_population_id(&source) != self.population_id {
            return Err(
                "Population V5 identity does not reproduce from persisted Finalization V3 source"
                    .to_owned(),
            );
        }
        if self.completion_id != self.derive_completion_id() {
            return Err("Population V5 Completion identity does not reproduce".to_owned());
        }
        Ok(())
    }

    fn derive_completion_id(&self) -> [u8; 32] {
        hash_parts(
            COMPLETION_ID_DOMAIN,
            &[
                &VERSION.to_le_bytes(),
                &self.population_id,
                &self.source_finalization_id,
                &self.source_finalization_completion_id,
                &self.source_finalization_ordered_row_digest,
                &self.source_admission_block_id,
                &self.source_admission_completion_id,
                &self.row_count.to_le_bytes(),
                &self.nifty_count.to_le_bytes(),
                &self.banknifty_count.to_le_bytes(),
                &self.admitted_count.to_le_bytes(),
                &self.rejected_count.to_le_bytes(),
                &self.unmeasured_count.to_le_bytes(),
                &self.refused_count.to_le_bytes(),
                &self.ordered_row_digest,
            ],
        )
    }

    fn encode(&self) -> Result<[u8; POPULATION_V5_COMPLETION_BYTES], PopulationV5Refusal> {
        self.validate()?;
        let mut raw = [0_u8; POPULATION_V5_COMPLETION_BYTES];
        let (payload, seal) = raw.split_at_mut(COMPLETION_PAYLOAD_BYTES);
        let mut writer = FixedWriter::new(payload);
        writer.array(&COMPLETION_MAGIC)?;
        writer.u32(VERSION)?;
        writer.u32(COMPLETION_DOMAIN)?;
        writer.u64(self.block_sequence)?;
        writer.u64(self.first_row_record)?;
        for value in [
            self.population_id,
            self.source_finalization_id,
            self.source_finalization_completion_id,
            self.source_finalization_ordered_row_digest,
            self.source_admission_block_id,
            self.source_admission_completion_id,
        ] {
            writer.array(&value)?;
        }
        for value in [
            self.row_count,
            self.nifty_count,
            self.banknifty_count,
            self.admitted_count,
            self.rejected_count,
            self.unmeasured_count,
            self.refused_count,
        ] {
            writer.u64(value)?;
        }
        writer.array(&self.ordered_row_digest)?;
        writer.array(&self.completion_id)?;
        writer.zeros(writer.remaining())?;
        writer.require_full("Population V5 Completion payload")?;
        seal.copy_from_slice(&hash_parts(COMPLETION_SEAL_DOMAIN, &[payload]));
        Ok(raw)
    }

    fn decode(raw: &[u8; POPULATION_V5_COMPLETION_BYTES]) -> Result<Self, PopulationV5Refusal> {
        let (payload, seal) = raw.split_at(COMPLETION_PAYLOAD_BYTES);
        if seal != hash_parts(COMPLETION_SEAL_DOMAIN, &[payload]) {
            return Err("Population V5 Completion seal mismatch".to_owned());
        }
        let mut reader = FixedReader::new(payload);
        if reader.array::<16>()? != COMPLETION_MAGIC {
            return Err("Population V5 Completion magic mismatch".to_owned());
        }
        let version = reader.u32()?;
        let domain = reader.u32()?;
        if version != VERSION || domain != COMPLETION_DOMAIN {
            return Err(format!(
                "Population V5 Completion version/domain {version}/{domain} is unsupported"
            ));
        }
        let completion = Self {
            block_sequence: reader.u64()?,
            first_row_record: reader.u64()?,
            population_id: reader.array()?,
            source_finalization_id: reader.array()?,
            source_finalization_completion_id: reader.array()?,
            source_finalization_ordered_row_digest: reader.array()?,
            source_admission_block_id: reader.array()?,
            source_admission_completion_id: reader.array()?,
            row_count: reader.u64()?,
            nifty_count: reader.u64()?,
            banknifty_count: reader.u64()?,
            admitted_count: reader.u64()?,
            rejected_count: reader.u64()?,
            unmeasured_count: reader.u64()?,
            refused_count: reader.u64()?,
            ordered_row_digest: reader.array()?,
            completion_id: reader.array()?,
        };
        reader.require_zeros(
            reader.remaining(),
            "Population V5 Completion trailing reserve",
        )?;
        completion.validate()?;
        if completion.encode()? != *raw {
            return Err("Population V5 Completion is not byte-canonical".to_owned());
        }
        Ok(completion)
    }
}

fn derive_population_id(receipt: &SourceFinalizationV3Receipt) -> [u8; 32] {
    hash_parts(
        POPULATION_ID_DOMAIN,
        &[
            &VERSION.to_le_bytes(),
            &receipt.finalization_id,
            &receipt.completion_id,
            &receipt.admission_block_id,
            &receipt.admission_completion_id,
            &receipt.row_count.to_le_bytes(),
            &receipt.nifty_row_count.to_le_bytes(),
            &receipt.banknifty_row_count.to_le_bytes(),
            &receipt.admitted_count.to_le_bytes(),
            &receipt.rejected_count.to_le_bytes(),
            &receipt.unmeasured_count.to_le_bytes(),
            &receipt.refused_count.to_le_bytes(),
            &receipt.ordered_row_digest,
        ],
    )
}

fn ordered_row_digest(rows: &[PopulationV5RowRecord]) -> Result<[u8; 32], PopulationV5Refusal> {
    let mut hasher = Hasher::new();
    hasher.update(ORDERED_ROWS_DOMAIN);
    hasher.update(&VERSION.to_le_bytes());
    hasher.update(
        &u64::try_from(rows.len())
            .map_err(|_| "Population V5 row count does not fit u64".to_owned())?
            .to_le_bytes(),
    );
    for row in rows {
        hasher.update(&row.encode()?);
    }
    Ok(hasher.finalize())
}

/// Structural receipt from one bounded fresh reopen. It is not authority by
/// itself and has no public constructor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationV5StructuralReceipt {
    block_sequence: u64,
    first_row_record: u64,
    population_id: [u8; 32],
    source_finalization_id: [u8; 32],
    source_finalization_completion_id: [u8; 32],
    source_finalization_ordered_row_digest: [u8; 32],
    source_admission_block_id: [u8; 32],
    source_admission_completion_id: [u8; 32],
    row_count: u64,
    nifty_count: u64,
    banknifty_count: u64,
    admitted_count: u64,
    rejected_count: u64,
    unmeasured_count: u64,
    refused_count: u64,
    ordered_row_digest: [u8; 32],
    completion_id: [u8; 32],
}

impl PopulationV5StructuralReceipt {
    fn from_completion(value: &PopulationV5CompletionRecord) -> Self {
        Self {
            block_sequence: value.block_sequence,
            first_row_record: value.first_row_record,
            population_id: value.population_id,
            source_finalization_id: value.source_finalization_id,
            source_finalization_completion_id: value.source_finalization_completion_id,
            source_finalization_ordered_row_digest: value.source_finalization_ordered_row_digest,
            source_admission_block_id: value.source_admission_block_id,
            source_admission_completion_id: value.source_admission_completion_id,
            row_count: value.row_count,
            nifty_count: value.nifty_count,
            banknifty_count: value.banknifty_count,
            admitted_count: value.admitted_count,
            rejected_count: value.rejected_count,
            unmeasured_count: value.unmeasured_count,
            refused_count: value.refused_count,
            ordered_row_digest: value.ordered_row_digest,
            completion_id: value.completion_id,
        }
    }

    #[must_use]
    pub(crate) const fn population_id(self) -> [u8; 32] {
        self.population_id
    }

    #[must_use]
    pub(crate) const fn block_sequence(self) -> u64 {
        self.block_sequence
    }

    #[must_use]
    pub(crate) const fn first_row_record(self) -> u64 {
        self.first_row_record
    }

    #[must_use]
    pub(crate) const fn row_count(self) -> u64 {
        self.row_count
    }

    #[must_use]
    pub(crate) const fn nifty_count(self) -> u64 {
        self.nifty_count
    }

    #[must_use]
    pub(crate) const fn banknifty_count(self) -> u64 {
        self.banknifty_count
    }

    #[must_use]
    pub(crate) const fn completion_id(self) -> [u8; 32] {
        self.completion_id
    }

    #[must_use]
    pub(crate) const fn source_finalization_id(self) -> [u8; 32] {
        self.source_finalization_id
    }

    #[must_use]
    pub(crate) const fn source_finalization_completion_id(self) -> [u8; 32] {
        self.source_finalization_completion_id
    }

    #[must_use]
    pub(crate) const fn source_finalization_ordered_row_digest(self) -> [u8; 32] {
        self.source_finalization_ordered_row_digest
    }

    #[must_use]
    pub(crate) const fn source_admission_block_id(self) -> [u8; 32] {
        self.source_admission_block_id
    }

    #[must_use]
    pub(crate) const fn source_admission_completion_id(self) -> [u8; 32] {
        self.source_admission_completion_id
    }

    #[must_use]
    pub(crate) const fn admitted_count(self) -> u64 {
        self.admitted_count
    }

    #[must_use]
    pub(crate) const fn rejected_count(self) -> u64 {
        self.rejected_count
    }

    #[must_use]
    pub(crate) const fn unmeasured_count(self) -> u64 {
        self.unmeasured_count
    }

    #[must_use]
    pub(crate) const fn refused_count(self) -> u64 {
        self.refused_count
    }

    #[must_use]
    pub(crate) const fn ordered_row_digest(self) -> [u8; 32] {
        self.ordered_row_digest
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformIdentity {
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(not(unix))]
    created: Option<std::time::SystemTime>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGeneration {
    len: u64,
    platform: PlatformIdentity,
    modified: Option<std::time::SystemTime>,
    content_digest: [u8; 32],
    generation_digest: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TrailingRows {
    first_row_record: u64,
    population_id: [u8; 32],
    rows: Vec<PopulationV5RowRecord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PopulationV5StructuralCommit {
    Written(PopulationV5StructuralReceipt),
    Reused(PopulationV5StructuralReceipt),
}

impl PopulationV5StructuralCommit {
    const fn receipt(self) -> PopulationV5StructuralReceipt {
        match self {
            Self::Written(receipt) | Self::Reused(receipt) => receipt,
        }
    }

    const fn was_written(self) -> bool {
        matches!(self, Self::Written(_))
    }
}

/// Retained, generation-checked Population V5 ledger.
///
/// Opening and every authenticated read hash all explicitly bounded files;
/// they are O(file bytes), not O(1). Fixed row address arithmetic is O(1).
/// Receipt-index lookup is expected/amortized O(1), with O(B) collision worst
/// case for B bounded Completion records.
struct PopulationV5Ledger {
    root: PathBuf,
    root_file: File,
    root_identity: PlatformIdentity,
    lock_path: PathBuf,
    lock_file: File,
    lock_generation: FileGeneration,
    row_path: PathBuf,
    row_file: File,
    row_generation: FileGeneration,
    completion_path: PathBuf,
    completion_file: File,
    completion_generation: FileGeneration,
    bounds: PopulationV5Bounds,
    writable: bool,
    receipts: HashMap<[u8; 32], PopulationV5StructuralReceipt>,
    trailing: Option<TrailingRows>,
    row_records: u64,
    completion_records: u64,
}

impl PopulationV5Ledger {
    pub(crate) fn open_read(
        root: &Path,
        bounds: PopulationV5Bounds,
    ) -> Result<Self, PopulationV5Refusal> {
        Self::open(root, bounds, false)
    }

    fn open_write(root: &Path, bounds: PopulationV5Bounds) -> Result<Self, PopulationV5Refusal> {
        Self::open(root, bounds, true)
    }

    fn open(
        root: &Path,
        bounds: PopulationV5Bounds,
        writable: bool,
    ) -> Result<Self, PopulationV5Refusal> {
        let (root, root_file, root_identity) = open_root_directory(root)?;
        let lock_path = root.join(LOCK_FILE);
        let row_path = root.join(ROW_FILE);
        let completion_path = root.join(COMPLETION_FILE);
        let (lock_file, lock_created) = open_child(&lock_path, writable, writable)?;
        if writable {
            lock_file
                .lock()
                .map_err(|why| format!("cannot lock Population V5 writer: {why}"))?;
        } else {
            lock_file
                .lock_shared()
                .map_err(|why| format!("cannot lock Population V5 reader: {why}"))?;
        }
        let opened = (|| {
            let (row_file, row_created) = open_child(&row_path, writable, writable)?;
            let (completion_file, completion_created) =
                open_child(&completion_path, writable, writable)?;
            if lock_created || row_created || completion_created {
                sync_directory(&root_file, &root)?;
            }
            if named_identity(&root)? != root_identity {
                return Err("Population V5 root changed while child files opened".to_owned());
            }
            let lock_generation = file_generation(&lock_file, &lock_path, LOCK_MAX_BYTES)?;
            let row_generation = file_generation(&row_file, &row_path, bounds.max_row_bytes())?;
            let completion_generation = file_generation(
                &completion_file,
                &completion_path,
                bounds.max_completion_bytes(),
            )?;
            let mut ledger = Self {
                root,
                root_file,
                root_identity,
                lock_path,
                lock_file: lock_file
                    .try_clone()
                    .map_err(|why| format!("cannot clone Population V5 lock file: {why}"))?,
                lock_generation,
                row_path,
                row_file,
                row_generation,
                completion_path,
                completion_file,
                completion_generation,
                bounds,
                writable,
                receipts: HashMap::new(),
                trailing: None,
                row_records: 0,
                completion_records: 0,
            };
            ledger.scan()?;
            Ok(ledger)
        })();
        let released = lock_file
            .unlock()
            .map_err(|why| format!("cannot unlock Population V5 open lock: {why}"));
        combine_lock_result(opened, released)
    }

    fn scan(&mut self) -> Result<(), PopulationV5Refusal> {
        let row_records = checked_record_count(
            self.row_generation.len,
            POPULATION_V5_ROW_BYTES,
            self.bounds.max_row_records(),
            "row",
        )?;
        let completion_records = checked_record_count(
            self.completion_generation.len,
            POPULATION_V5_COMPLETION_BYTES,
            self.bounds.max_completion_records(),
            "Completion",
        )?;
        self.receipts.clear();
        self.receipts
            .try_reserve(
                usize::try_from(completion_records)
                    .map_err(|_| "Population V5 Completion count does not fit usize".to_owned())?,
            )
            .map_err(|why| format!("cannot reserve Population V5 receipt index: {why}"))?;
        self.trailing = None;
        let mut covered_rows = 0_u64;
        for completion_index in 0..completion_records {
            let completion = PopulationV5CompletionRecord::decode(&read_fixed_at(
                &mut self.completion_file,
                completion_index,
                POPULATION_V5_COMPLETION_BYTES,
                "Completion",
            )?)?;
            if completion.block_sequence != completion_index
                || completion.first_row_record != covered_rows
            {
                return Err(format!(
                    "Population V5 Completion {completion_index} is not contiguous/canonical"
                ));
            }
            require_block_bound(self.bounds, completion.row_count)?;
            let end = covered_rows
                .checked_add(completion.row_count)
                .ok_or_else(|| "Population V5 completed row range overflowed".to_owned())?;
            if end > row_records {
                return Err(format!(
                    "Population V5 Completion {completion_index} is torn at row {end}; file has {row_records}"
                ));
            }
            let rows = self.read_rows(covered_rows, completion.row_count)?;
            let receipt = validate_complete_block(&rows, &completion)?;
            if self
                .receipts
                .insert(receipt.population_id, receipt)
                .is_some()
            {
                return Err(format!(
                    "Population V5 identity {} appears more than once",
                    hex32(receipt.population_id)
                ));
            }
            covered_rows = end;
        }
        if covered_rows < row_records {
            let count = row_records
                .checked_sub(covered_rows)
                .ok_or_else(|| "Population V5 trailing row count underflowed".to_owned())?;
            require_block_bound(self.bounds, count)?;
            let rows = self.read_rows(covered_rows, count)?;
            let population_id = validate_trailing_rows(&rows)?;
            if self.receipts.contains_key(&population_id) {
                return Err(format!(
                    "Population V5 trailing identity {} duplicates a completed Population",
                    hex32(population_id)
                ));
            }
            self.trailing = Some(TrailingRows {
                first_row_record: covered_rows,
                population_id,
                rows,
            });
        }
        self.row_records = row_records;
        self.completion_records = completion_records;
        self.require_unchanged()
    }

    fn read_rows(
        &mut self,
        first: u64,
        count: u64,
    ) -> Result<Vec<PopulationV5RowRecord>, PopulationV5Refusal> {
        require_block_bound(self.bounds, count)?;
        let capacity = usize::try_from(count)
            .map_err(|_| "Population V5 read count does not fit usize".to_owned())?;
        let mut rows = Vec::new();
        rows.try_reserve_exact(capacity)
            .map_err(|why| format!("cannot reserve Population V5 read rows: {why}"))?;
        for offset in 0..count {
            let physical = first
                .checked_add(offset)
                .ok_or_else(|| "Population V5 row offset overflowed".to_owned())?;
            let raw = read_fixed_at(&mut self.row_file, physical, POPULATION_V5_ROW_BYTES, "row")?;
            rows.push(PopulationV5RowRecord::decode(&raw)?);
        }
        Ok(rows)
    }

    fn append(
        &mut self,
        prepared: &PreparedPopulationV5,
    ) -> Result<PopulationV5StructuralCommit, PopulationV5Refusal> {
        if !self.writable {
            return Err("Population V5 ledger was opened read-only".to_owned());
        }
        self.lock_file
            .lock()
            .map_err(|why| format!("cannot take Population V5 append lock: {why}"))?;
        let result = self.append_locked(prepared);
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Population V5 append lock: {why}"));
        combine_lock_result(result, released)
    }

    fn append_locked(
        &mut self,
        prepared: &PreparedPopulationV5,
    ) -> Result<PopulationV5StructuralCommit, PopulationV5Refusal> {
        self.require_unchanged()?;
        prepared.validate(self.bounds)?;
        if let Some(existing) = self.receipts.get(&prepared.population_id).copied() {
            return self.reuse_existing(prepared, &existing);
        }
        if let Some(trailing) = self.trailing.clone() {
            return self.complete_trailing(prepared, &trailing);
        }
        let count = u64::try_from(prepared.rows.len())
            .map_err(|_| "Population V5 prepared count does not fit u64".to_owned())?;
        self.require_append_bound(count, 1)?;
        let first = self.row_records;
        self.append_row_suffix(&prepared.rows, 0)?;
        self.row_file
            .sync_data()
            .map_err(|why| format!("cannot sync Population V5 rows: {why}"))?;
        self.row_records = self
            .row_records
            .checked_add(count)
            .ok_or_else(|| "Population V5 row count overflowed".to_owned())?;
        self.refresh_row_generation()?;
        self.require_unchanged()?;
        self.append_completion(prepared, first)?;
        self.finish_written(prepared.population_id)
    }

    fn complete_trailing(
        &mut self,
        prepared: &PreparedPopulationV5,
        trailing: &TrailingRows,
    ) -> Result<PopulationV5StructuralCommit, PopulationV5Refusal> {
        let prefix_len = trailing.rows.len();
        let expected_prefix = prepared.rows.get(..prefix_len).ok_or_else(|| {
            format!(
                "Population V5 trailing block has {prefix_len} rows above retry count {}",
                prepared.rows.len()
            )
        })?;
        if trailing.population_id != prepared.population_id
            || trailing.rows.as_slice() != expected_prefix
        {
            return Err(format!(
                "Population V5 trailing Population {} is not an exact canonical prefix of retry {}",
                hex32(trailing.population_id),
                hex32(prepared.population_id)
            ));
        }
        let missing = prepared
            .rows
            .len()
            .checked_sub(prefix_len)
            .ok_or_else(|| "Population V5 trailing prefix length underflowed".to_owned())?;
        let missing = u64::try_from(missing)
            .map_err(|_| "Population V5 missing suffix count does not fit u64".to_owned())?;
        self.require_append_bound(missing, 1)?;
        self.require_unchanged()?;
        self.append_row_suffix(&prepared.rows, prefix_len)?;
        self.row_file
            .sync_data()
            .map_err(|why| format!("cannot sync completed Population V5 row prefix: {why}"))?;
        self.row_records = self.row_records.checked_add(missing).ok_or_else(|| {
            "Population V5 row count overflowed while completing prefix".to_owned()
        })?;
        self.refresh_row_generation()?;
        self.require_unchanged()?;
        self.append_completion(prepared, trailing.first_row_record)?;
        self.finish_written(prepared.population_id)
    }

    fn append_row_suffix(
        &mut self,
        rows: &[PopulationV5RowRecord],
        start: usize,
    ) -> Result<(), PopulationV5Refusal> {
        for row in rows.get(start..).ok_or_else(|| {
            format!(
                "Population V5 suffix start {start} is outside {} rows",
                rows.len()
            )
        })? {
            append_raw(&mut self.row_file, &row.encode()?)?;
        }
        Ok(())
    }

    fn append_completion(
        &mut self,
        prepared: &PreparedPopulationV5,
        first_row_record: u64,
    ) -> Result<(), PopulationV5Refusal> {
        let completion = prepared.expected_completion(self.completion_records, first_row_record)?;
        append_raw(&mut self.completion_file, &completion.encode()?)?;
        self.completion_file
            .sync_data()
            .map_err(|why| format!("cannot sync Population V5 Completion: {why}"))?;
        sync_directory(&self.root_file, &self.root)?;
        self.completion_records = self
            .completion_records
            .checked_add(1)
            .ok_or_else(|| "Population V5 Completion count overflowed".to_owned())?;
        self.refresh_completion_generation()?;
        self.require_unchanged()
    }

    fn finish_written(
        &mut self,
        population_id: [u8; 32],
    ) -> Result<PopulationV5StructuralCommit, PopulationV5Refusal> {
        self.scan()?;
        let receipt = self.receipts.get(&population_id).copied().ok_or_else(|| {
            format!(
                "Population V5 appended identity {} was not indexed",
                hex32(population_id)
            )
        })?;
        Ok(PopulationV5StructuralCommit::Written(receipt))
    }

    fn reuse_existing(
        &mut self,
        prepared: &PreparedPopulationV5,
        existing: &PopulationV5StructuralReceipt,
    ) -> Result<PopulationV5StructuralCommit, PopulationV5Refusal> {
        let observed = self.read_rows(existing.first_row_record, existing.row_count)?;
        if observed != prepared.rows {
            return Err(format!(
                "Population V5 identity {} exists with different exact rows",
                hex32(existing.population_id)
            ));
        }
        let observed_completion = PopulationV5CompletionRecord::decode(&read_fixed_at(
            &mut self.completion_file,
            existing.block_sequence,
            POPULATION_V5_COMPLETION_BYTES,
            "Completion",
        )?)?;
        let expected =
            prepared.expected_completion(existing.block_sequence, existing.first_row_record)?;
        if observed_completion != expected {
            return Err(format!(
                "Population V5 identity {} exists with different exact Completion",
                hex32(existing.population_id)
            ));
        }
        self.row_file
            .sync_data()
            .map_err(|why| format!("cannot sync reused Population V5 rows: {why}"))?;
        self.completion_file
            .sync_data()
            .map_err(|why| format!("cannot sync reused Population V5 Completion: {why}"))?;
        sync_directory(&self.root_file, &self.root)?;
        self.require_unchanged()?;
        Ok(PopulationV5StructuralCommit::Reused(*existing))
    }

    fn require_append_bound(&self, rows: u64, completions: u64) -> Result<(), PopulationV5Refusal> {
        let next_rows = self
            .row_records
            .checked_add(rows)
            .ok_or_else(|| "Population V5 append row count overflowed".to_owned())?;
        if next_rows > self.bounds.max_row_records() {
            return Err(format!(
                "Population V5 append reaches {next_rows} rows above bound {}",
                self.bounds.max_row_records()
            ));
        }
        let next_row_bytes = next_rows
            .checked_mul(POPULATION_V5_ROW_BYTES as u64)
            .ok_or_else(|| "Population V5 append row bytes overflowed".to_owned())?;
        if next_row_bytes > self.bounds.max_row_bytes() {
            return Err(format!(
                "Population V5 append reaches {next_row_bytes} row bytes above bound {}",
                self.bounds.max_row_bytes()
            ));
        }
        let next_completions = self
            .completion_records
            .checked_add(completions)
            .ok_or_else(|| "Population V5 append Completion count overflowed".to_owned())?;
        if next_completions > self.bounds.max_completion_records() {
            return Err(format!(
                "Population V5 append reaches {next_completions} Completions above bound {}",
                self.bounds.max_completion_records()
            ));
        }
        let next_completion_bytes = next_completions
            .checked_mul(POPULATION_V5_COMPLETION_BYTES as u64)
            .ok_or_else(|| "Population V5 append Completion bytes overflowed".to_owned())?;
        if next_completion_bytes > self.bounds.max_completion_bytes() {
            return Err(format!(
                "Population V5 append reaches {next_completion_bytes} Completion bytes above bound {}",
                self.bounds.max_completion_bytes()
            ));
        }
        Ok(())
    }

    fn structural_receipt(
        &self,
        population_id: &[u8; 32],
    ) -> Result<Option<PopulationV5StructuralReceipt>, PopulationV5Refusal> {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take Population V5 lookup lock: {why}"))?;
        let result = self
            .require_unchanged()
            .map(|()| self.receipts.get(population_id).copied());
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Population V5 lookup lock: {why}"));
        combine_lock_result(result, released)
    }

    fn authenticated_row(
        &mut self,
        receipt: &PopulationV5StructuralReceipt,
        global_sequence: u64,
    ) -> Result<PopulationV5SuccessorRow, PopulationV5Refusal> {
        if global_sequence >= receipt.row_count {
            return Err(format!(
                "Population V5 row {global_sequence} is outside authenticated count {}",
                receipt.row_count
            ));
        }
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take Population V5 row lock: {why}"))?;
        let result = (|| {
            self.require_unchanged()?;
            if self.receipts.get(&receipt.population_id) != Some(receipt) {
                return Err("Population V5 receipt is no longer indexed exactly".to_owned());
            }
            let physical = receipt
                .first_row_record
                .checked_add(global_sequence)
                .ok_or_else(|| "Population V5 authenticated row offset overflowed".to_owned())?;
            let raw = read_fixed_at(
                &mut self.row_file,
                physical,
                POPULATION_V5_ROW_BYTES,
                "authenticated row",
            )?;
            let row = PopulationV5SuccessorRow::authenticate(&raw)?;
            let expected_family = if global_sequence < receipt.nifty_count {
                AdmissionV3Family::Nifty
            } else {
                AdmissionV3Family::BankNifty
            };
            let expected_family_sequence = if expected_family == AdmissionV3Family::Nifty {
                global_sequence
            } else {
                global_sequence
                    .checked_sub(receipt.nifty_count)
                    .ok_or_else(|| "Population V5 BANKNIFTY ordinal underflowed".to_owned())?
            };
            if row.population_id() != receipt.population_id
                || row.global_sequence() != global_sequence
                || row.family() != expected_family
                || row.family_sequence() != expected_family_sequence
            {
                return Err(
                    "Population V5 fixed-offset row violates authenticated order".to_owned(),
                );
            }
            self.require_unchanged()?;
            Ok(row)
        })();
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Population V5 row lock: {why}"));
        combine_lock_result(result, released)
    }

    fn authenticated_rows(
        &mut self,
        receipt: &PopulationV5StructuralReceipt,
    ) -> Result<Vec<PopulationV5SuccessorRow>, PopulationV5Refusal> {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take Population V5 bulk-row lock: {why}"))?;
        let result = (|| {
            self.require_unchanged()?;
            if self.receipts.get(&receipt.population_id) != Some(receipt) {
                return Err("Population V5 bulk receipt is no longer indexed exactly".to_owned());
            }
            let completion = PopulationV5CompletionRecord::decode(&read_fixed_at(
                &mut self.completion_file,
                receipt.block_sequence,
                POPULATION_V5_COMPLETION_BYTES,
                "Completion",
            )?)?;
            let rows = self.read_rows(receipt.first_row_record, receipt.row_count)?;
            if validate_complete_block(&rows, &completion)? != *receipt {
                return Err("Population V5 bulk block differs from exact receipt".to_owned());
            }
            let capacity = usize::try_from(receipt.row_count)
                .map_err(|_| "Population V5 bulk count does not fit usize".to_owned())?;
            let mut authenticated = Vec::new();
            authenticated
                .try_reserve_exact(capacity)
                .map_err(|why| format!("cannot reserve Population V5 bulk rows: {why}"))?;
            for row in rows {
                authenticated.push(PopulationV5SuccessorRow::authenticate(&row.encode()?)?);
            }
            self.require_unchanged()?;
            Ok(authenticated)
        })();
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Population V5 bulk-row lock: {why}"));
        combine_lock_result(result, released)
    }

    fn require_exact_prepared(
        &mut self,
        receipt: &PopulationV5StructuralReceipt,
        prepared: &PreparedPopulationV5,
    ) -> Result<(), PopulationV5Refusal> {
        if receipt.population_id != prepared.population_id {
            return Err("Population V5 fresh receipt has a different identity".to_owned());
        }
        let rows = self.authenticated_rows(receipt)?;
        if rows.len() != prepared.rows.len() {
            return Err("Population V5 fresh reopen row count differs".to_owned());
        }
        for (observed, expected) in rows.iter().zip(&prepared.rows) {
            if observed.canonical_record() != &expected.encode()? {
                return Err("Population V5 fresh reopen row bytes differ".to_owned());
            }
        }
        let completion = PopulationV5CompletionRecord::decode(&read_fixed_at(
            &mut self.completion_file,
            receipt.block_sequence,
            POPULATION_V5_COMPLETION_BYTES,
            "Completion",
        )?)?;
        if completion
            != prepared.expected_completion(receipt.block_sequence, receipt.first_row_record)?
        {
            return Err("Population V5 fresh reopen Completion bytes differ".to_owned());
        }
        self.require_unchanged()
    }

    fn refresh_row_generation(&mut self) -> Result<(), PopulationV5Refusal> {
        self.row_generation =
            file_generation(&self.row_file, &self.row_path, self.bounds.max_row_bytes())?;
        Ok(())
    }

    fn refresh_completion_generation(&mut self) -> Result<(), PopulationV5Refusal> {
        self.completion_generation = file_generation(
            &self.completion_file,
            &self.completion_path,
            self.bounds.max_completion_bytes(),
        )?;
        Ok(())
    }

    fn require_unchanged(&self) -> Result<(), PopulationV5Refusal> {
        if named_identity(&self.root)? != self.root_identity
            || file_generation(&self.lock_file, &self.lock_path, LOCK_MAX_BYTES)?
                != self.lock_generation
            || file_generation(&self.row_file, &self.row_path, self.bounds.max_row_bytes())?
                != self.row_generation
            || file_generation(
                &self.completion_file,
                &self.completion_path,
                self.bounds.max_completion_bytes(),
            )? != self.completion_generation
        {
            return Err("Population V5 retained root or file generation changed".to_owned());
        }
        Ok(())
    }
}

/// Durable Population V5 authority minted only after an exact fresh reopen.
pub(crate) struct PopulationV5Authority {
    receipt: PopulationV5StructuralReceipt,
    ledger: PopulationV5Ledger,
}

impl PopulationV5Authority {
    #[must_use]
    pub(crate) const fn structural_receipt(&self) -> PopulationV5StructuralReceipt {
        self.receipt
    }

    /// Reads one fixed-offset row after validating every retained generation.
    pub(crate) fn authenticated_row(
        &mut self,
        global_sequence: u64,
    ) -> Result<PopulationV5SuccessorRow, PopulationV5Refusal> {
        self.ledger
            .authenticated_row(&self.receipt, global_sequence)
    }

    /// Reads the complete ordered successor block atomically. No partial
    /// vector escapes when any row, Completion or generation fails.
    pub(crate) fn ordered_authenticated_rows(
        &mut self,
    ) -> Result<Vec<PopulationV5SuccessorRow>, PopulationV5Refusal> {
        self.ledger.authenticated_rows(&self.receipt)
    }
}

/// Byte-exact result of Population V5 append/reuse followed by a fresh reopen.
pub(crate) enum PopulationV5ProductionCommit {
    Written(PopulationV5Authority),
    Reused(PopulationV5Authority),
}

impl PopulationV5ProductionCommit {
    #[must_use]
    pub(crate) const fn was_written(&self) -> bool {
        matches!(self, Self::Written(_))
    }

    pub(crate) fn authority_mut(&mut self) -> &mut PopulationV5Authority {
        match self {
            Self::Written(authority) | Self::Reused(authority) => authority,
        }
    }
}

/// Nonconstructible stored Population V5 capability retaining every upstream
/// Candidate/Admission/Finalization root beside the freshly reopened V5
/// ledger. Embedded bytes never replace live provenance.
pub(crate) struct CommittedStoredPopulationV5 {
    source: CommittedStoredPopulationFinalizationV3,
    v5: PopulationV5ProductionCommit,
}

impl CommittedStoredPopulationV5 {
    #[must_use]
    pub(crate) fn was_written(&self) -> bool {
        self.v5.was_written()
    }

    #[must_use]
    pub(crate) fn structural_receipt(&mut self) -> PopulationV5StructuralReceipt {
        self.v5.authority_mut().structural_receipt()
    }

    /// Revalidates the complete live upstream source and V5 generations before
    /// returning one exact row. No embedded or detached identity can mint this
    /// capability.
    pub(crate) fn authenticated_row(
        &mut self,
        global_sequence: u64,
    ) -> Result<PopulationV5SuccessorRow, PopulationV5Refusal> {
        let bounds = self.v5.authority_mut().ledger.bounds;
        let prepared_before = PreparedPopulationV5::from_authority(&mut self.source, bounds)?;
        let expected_index = usize::try_from(global_sequence)
            .map_err(|_| "Population V5 row ordinal does not fit usize".to_owned())?;
        let expected = prepared_before.rows.get(expected_index).ok_or_else(|| {
            format!(
                "Population V5 row {global_sequence} is outside live source count {}",
                prepared_before.rows.len()
            )
        })?;
        let row = self.v5.authority_mut().authenticated_row(global_sequence)?;
        if row.canonical_record() != &expected.encode()? {
            return Err(
                "Population V5 row differs from revalidated live upstream source".to_owned(),
            );
        }
        let prepared_after = PreparedPopulationV5::from_authority(&mut self.source, bounds)?;
        if prepared_after != prepared_before {
            return Err(
                "Population V5 live upstream source changed during authenticated row read"
                    .to_owned(),
            );
        }
        Ok(row)
    }

    /// Revalidates all live upstream roots/generations, then returns the exact
    /// ordered V5 block atomically.
    pub(crate) fn ordered_authenticated_rows(
        &mut self,
    ) -> Result<Vec<PopulationV5SuccessorRow>, PopulationV5Refusal> {
        let bounds = self.v5.authority_mut().ledger.bounds;
        let prepared_before = PreparedPopulationV5::from_authority(&mut self.source, bounds)?;
        let rows = self.v5.authority_mut().ordered_authenticated_rows()?;
        if rows.len() != prepared_before.rows.len() {
            return Err(
                "Population V5 rows differ in count from revalidated live upstream source"
                    .to_owned(),
            );
        }
        for (row, expected) in rows.iter().zip(&prepared_before.rows) {
            if row.canonical_record() != &expected.encode()? {
                return Err(
                    "Population V5 rows differ from revalidated live upstream source".to_owned(),
                );
            }
        }
        let prepared_after = PreparedPopulationV5::from_authority(&mut self.source, bounds)?;
        if prepared_after != prepared_before {
            return Err(
                "Population V5 live upstream source changed during ordered authenticated read"
                    .to_owned(),
            );
        }
        Ok(rows)
    }

    /// Reauthenticates every retained root and joins the exact V5 rows to a
    /// freshly rebuilt Candidate execution replay.
    ///
    /// The returned capability is the only input accepted by Execution V3's
    /// private preparation path. It cannot be constructed from caller rows,
    /// detached digests, masks, grids, or prior Runner dispositions.
    pub(crate) fn execution_v3_source(
        &mut self,
    ) -> Result<PopulationV5ExecutionV3SourceV1, PopulationV5Refusal> {
        let rows_before = self.ordered_authenticated_rows()?;
        let receipt = self.structural_receipt();
        let pair = self
            .source
            .execution_v3_replay_pair()
            .map_err(|why| format!("Population V5 Execution V3 replay source refused: {why}"))?;
        let source = PopulationV5ExecutionV3SourceV1::from_authenticated_sources(
            receipt,
            rows_before,
            pair,
        )?;
        let rows_after = self.ordered_authenticated_rows()?;
        if rows_after.len() != source.rows.len()
            || rows_after
                .iter()
                .zip(&source.rows)
                .any(|(after, before)| after != before.population())
        {
            return Err(
                "Population V5 live source changed while deriving Execution V3 authority"
                    .to_owned(),
            );
        }
        Ok(source)
    }
}

/// Commits the exact retained three-ledger successor join as Population V5.
///
/// This is the sole production preparation door. It takes the nonconstructible
/// Finalization V3 capability directly and calls `population_v5_inputs()`;
/// there is no caller-vector, raw-record, digest or detached-receipt overload.
/// Rows are synced before Completion, the writer is dropped, and authority is
/// returned only after an independent read-only reopen reproduces every byte.
///
/// # Cost
///
/// Sequential work is O(C + bounded source/file bytes) and space is O(C) for
/// C Candidates. Hash-backed duplicate/index work is expected/amortized O(C),
/// with a conservative O(C²) collision worst case. The operation is
/// intentionally not described as O(1).
pub(crate) fn commit_population_v5(
    root: &Path,
    bounds: PopulationV5Bounds,
    mut source: CommittedStoredPopulationFinalizationV3,
) -> Result<CommittedStoredPopulationV5, PopulationV5Refusal> {
    let prepared = PreparedPopulationV5::from_authority(&mut source, bounds)?;
    let structural = {
        let mut writer = PopulationV5Ledger::open_write(root, bounds)?;
        writer.append(&prepared)?
    };
    let receipt = structural.receipt();
    let mut ledger = PopulationV5Ledger::open_read(root, bounds)?;
    let reopened = ledger
        .structural_receipt(&prepared.population_id)?
        .ok_or_else(|| "Population V5 fresh reopen did not find committed identity".to_owned())?;
    if reopened != receipt {
        return Err("Population V5 fresh reopen receipt differs from writer receipt".to_owned());
    }
    ledger.require_exact_prepared(&reopened, &prepared)?;
    let prepared_after = PreparedPopulationV5::from_authority(&mut source, bounds)?;
    if prepared_after != prepared {
        return Err(
            "Population V5 live upstream source changed during persistence and fresh reopen"
                .to_owned(),
        );
    }
    let authority = PopulationV5Authority {
        receipt: reopened,
        ledger,
    };
    let v5 = if structural.was_written() {
        PopulationV5ProductionCommit::Written(authority)
    } else {
        PopulationV5ProductionCommit::Reused(authority)
    };
    Ok(CommittedStoredPopulationV5 { source, v5 })
}

fn validate_complete_block(
    rows: &[PopulationV5RowRecord],
    completion: &PopulationV5CompletionRecord,
) -> Result<PopulationV5StructuralReceipt, PopulationV5Refusal> {
    completion.validate()?;
    let count = u64::try_from(rows.len())
        .map_err(|_| "Population V5 completed row count does not fit u64".to_owned())?;
    if count != completion.row_count || count == 0 {
        return Err(format!(
            "Population V5 Completion declares {} rows but decoded {count}",
            completion.row_count
        ));
    }
    let mut row_ids = bounded_set(rows.len(), "Population V5 completed row identities")?;
    let mut candidate_ids = bounded_set(rows.len(), "Population V5 completed Candidates")?;
    let mut candidate_rows = bounded_set(rows.len(), "Population V5 completed Candidate rows")?;
    let mut admission_ids = bounded_set(rows.len(), "Population V5 completed Admissions")?;
    let mut finalization_rows = bounded_set(rows.len(), "Population V5 completed Finalizations")?;
    let mut status_counts = [0_u64; 4];
    let mut nifty_universe = None;
    let mut banknifty_universe = None;
    for (index, row) in rows.iter().enumerate() {
        row.validate()?;
        let global_sequence = u64::try_from(index)
            .map_err(|_| "Population V5 completed ordinal does not fit u64".to_owned())?;
        let (family, family_sequence, universe) = if global_sequence < completion.nifty_count {
            (
                AdmissionV3Family::Nifty,
                global_sequence,
                &mut nifty_universe,
            )
        } else {
            (
                AdmissionV3Family::BankNifty,
                global_sequence
                    .checked_sub(completion.nifty_count)
                    .ok_or_else(|| {
                        "Population V5 completed family ordinal underflowed".to_owned()
                    })?,
                &mut banknifty_universe,
            )
        };
        if row.population_id != completion.population_id
            || row.global_sequence != global_sequence
            || row.family != family
            || row.family_sequence != family_sequence
            || row.finalization.finalization_id != completion.source_finalization_id
            || row.finalization_completion_id != completion.source_finalization_completion_id
            || row.finalization.admission_block_id != completion.source_admission_block_id
            || row.finalization.admission_completion_id != completion.source_admission_completion_id
        {
            return Err(format!(
                "Population V5 completed row {global_sequence} violates ordering/source identity"
            ));
        }
        match universe {
            Some(value) if *value != row.finalization.candidate_universe_id => {
                return Err(format!(
                    "Population V5 completed family {:?} changes Candidate universe",
                    row.family
                ));
            }
            None => *universe = Some(row.finalization.candidate_universe_id),
            Some(_) => {}
        }
        if !row_ids.insert(row.row_id)
            || !candidate_ids.insert(row.finalization.candidate_semantic_id)
            || !candidate_rows.insert(row.finalization.candidate_row_digest)
            || !admission_ids.insert(row.finalization.admission_decision_id)
            || !finalization_rows.insert(row.finalization.finalization_row_id)
        {
            return Err(format!(
                "Population V5 completed row {global_sequence} duplicates an identity"
            ));
        }
        increment_status_count(&mut status_counts, row.status, "completed")?;
    }
    if status_counts
        != [
            completion.admitted_count,
            completion.rejected_count,
            completion.unmeasured_count,
            completion.refused_count,
        ]
        || ordered_row_digest(rows)? != completion.ordered_row_digest
    {
        return Err("Population V5 Completion status or ordered-row digest mismatch".to_owned());
    }
    Ok(PopulationV5StructuralReceipt::from_completion(completion))
}

fn validate_trailing_rows(rows: &[PopulationV5RowRecord]) -> Result<[u8; 32], PopulationV5Refusal> {
    let first = rows
        .first()
        .ok_or_else(|| "Population V5 trailing block is empty".to_owned())?;
    let population_id = first.population_id;
    let finalization_id = first.finalization.finalization_id;
    let finalization_completion_id = first.finalization_completion_id;
    let admission_block_id = first.finalization.admission_block_id;
    let admission_completion_id = first.finalization.admission_completion_id;
    let mut banknifty_started = false;
    let mut nifty_sequence = 0_u64;
    let mut banknifty_sequence = 0_u64;
    let mut row_ids = bounded_set(rows.len(), "Population V5 trailing row identities")?;
    let mut candidate_ids = bounded_set(rows.len(), "Population V5 trailing Candidates")?;
    let mut admission_ids = bounded_set(rows.len(), "Population V5 trailing Admissions")?;
    let mut finalization_rows = bounded_set(rows.len(), "Population V5 trailing Finalizations")?;
    for (index, row) in rows.iter().enumerate() {
        row.validate()?;
        let global_sequence = u64::try_from(index)
            .map_err(|_| "Population V5 trailing ordinal does not fit u64".to_owned())?;
        if row.population_id != population_id
            || row.global_sequence != global_sequence
            || row.finalization.finalization_id != finalization_id
            || row.finalization_completion_id != finalization_completion_id
            || row.finalization.admission_block_id != admission_block_id
            || row.finalization.admission_completion_id != admission_completion_id
        {
            return Err(format!(
                "Population V5 trailing row {global_sequence} changes block/source identity"
            ));
        }
        match row.family {
            AdmissionV3Family::Nifty if !banknifty_started => {
                if row.family_sequence != nifty_sequence {
                    return Err(
                        "Population V5 trailing NIFTY sequence is not contiguous".to_owned()
                    );
                }
                nifty_sequence = nifty_sequence
                    .checked_add(1)
                    .ok_or_else(|| "Population V5 trailing NIFTY count overflowed".to_owned())?;
            }
            AdmissionV3Family::BankNifty => {
                banknifty_started = true;
                if row.family_sequence != banknifty_sequence {
                    return Err(
                        "Population V5 trailing BANKNIFTY sequence is not contiguous".to_owned(),
                    );
                }
                banknifty_sequence = banknifty_sequence.checked_add(1).ok_or_else(|| {
                    "Population V5 trailing BANKNIFTY count overflowed".to_owned()
                })?;
            }
            AdmissionV3Family::Nifty => {
                return Err("Population V5 trailing NIFTY row follows BANKNIFTY".to_owned());
            }
        }
        if !row_ids.insert(row.row_id)
            || !candidate_ids.insert(row.finalization.candidate_semantic_id)
            || !admission_ids.insert(row.finalization.admission_decision_id)
            || !finalization_rows.insert(row.finalization.finalization_row_id)
        {
            return Err("Population V5 trailing block duplicates an identity".to_owned());
        }
    }
    Ok(population_id)
}

impl PlatformIdentity {
    fn of(metadata: &std::fs::Metadata) -> Self {
        Self {
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
            #[cfg(not(unix))]
            created: metadata.created().ok(),
        }
    }

    fn update_hasher(self, hasher: &mut Hasher) {
        #[cfg(unix)]
        {
            hasher.update(&self.device.to_le_bytes());
            hasher.update(&self.inode.to_le_bytes());
        }
        #[cfg(not(unix))]
        {
            let nanos = self
                .created
                .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0_u128, |value| value.as_nanos());
            hasher.update(&nanos.to_le_bytes());
        }
    }
}

fn bounded_set(capacity: usize, name: &str) -> Result<HashSet<[u8; 32]>, PopulationV5Refusal> {
    let mut set = HashSet::new();
    set.try_reserve(capacity)
        .map_err(|why| format!("cannot reserve {name}: {why}"))?;
    Ok(set)
}

fn checked_sum(values: &[u64], name: &str) -> Result<u64, PopulationV5Refusal> {
    values.iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(*value)
            .ok_or_else(|| format!("{name} overflowed"))
    })
}

fn require_nonzero(name: &str, value: [u8; 32]) -> Result<(), PopulationV5Refusal> {
    if value == [0; 32] {
        Err(format!("Population V5 {name} is zero"))
    } else {
        Ok(())
    }
}

fn usize_to_u64(value: usize, name: &str) -> Result<u64, PopulationV5Refusal> {
    u64::try_from(value).map_err(|_| format!("Population V5 {name} does not fit u64"))
}

fn require_block_bound(bounds: PopulationV5Bounds, count: u64) -> Result<(), PopulationV5Refusal> {
    if count == 0 || count > bounds.max_rows_per_population() {
        return Err(format!(
            "Population V5 block count {count} is outside 1..={}",
            bounds.max_rows_per_population()
        ));
    }
    Ok(())
}

fn checked_record_count(
    bytes: u64,
    stride: usize,
    max_records: u64,
    name: &str,
) -> Result<u64, PopulationV5Refusal> {
    let stride = u64::try_from(stride)
        .map_err(|_| format!("Population V5 {name} stride does not fit u64"))?;
    if !bytes.is_multiple_of(stride) {
        return Err(format!(
            "Population V5 {name} file has ragged length {bytes}, not a multiple of {stride}"
        ));
    }
    let records = bytes / stride;
    if records > max_records {
        return Err(format!(
            "Population V5 {name} file has {records} records above bound {max_records}"
        ));
    }
    Ok(records)
}

fn decode_family(value: u8) -> Result<AdmissionV3Family, PopulationV5Refusal> {
    match value {
        1 => Ok(AdmissionV3Family::Nifty),
        2 => Ok(AdmissionV3Family::BankNifty),
        _ => Err(format!("Population V5 family tag {value} is unknown")),
    }
}

fn decode_status(value: u8) -> Result<AdmissionV3Status, PopulationV5Refusal> {
    match value {
        1 => Ok(AdmissionV3Status::Admitted),
        2 => Ok(AdmissionV3Status::Rejected),
        3 => Ok(AdmissionV3Status::Unmeasured),
        4 => Ok(AdmissionV3Status::Refused),
        _ => Err(format!("Population V5 status tag {value} is unknown")),
    }
}

fn increment_status_count(
    counts: &mut [u64; 4],
    status: AdmissionV3Status,
    context: &str,
) -> Result<(), PopulationV5Refusal> {
    let index = usize::from(status as u8)
        .checked_sub(1)
        .ok_or_else(|| format!("Population V5 {context} status index underflowed"))?;
    let count = counts
        .get_mut(index)
        .ok_or_else(|| format!("Population V5 {context} status index is outside fixed matrix"))?;
    *count = count
        .checked_add(1)
        .ok_or_else(|| format!("Population V5 {context} status count overflowed"))?;
    Ok(())
}

fn hash_parts(domain: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize()
}

fn read_fixed_at<const N: usize>(
    file: &mut File,
    physical: u64,
    stride: usize,
    name: &str,
) -> Result<[u8; N], PopulationV5Refusal> {
    if N != stride {
        return Err(format!(
            "Population V5 {name} buffer {N} differs from stride {stride}"
        ));
    }
    let stride = u64::try_from(stride)
        .map_err(|_| format!("Population V5 {name} stride does not fit u64"))?;
    let offset = physical
        .checked_mul(stride)
        .ok_or_else(|| format!("Population V5 {name} offset overflowed"))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|why| format!("cannot seek Population V5 {name} {physical}: {why}"))?;
    let mut raw = [0_u8; N];
    file.read_exact(&mut raw)
        .map_err(|why| format!("cannot read Population V5 {name} {physical}: {why}"))?;
    Ok(raw)
}

fn append_raw(file: &mut File, raw: &[u8]) -> Result<(), PopulationV5Refusal> {
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(raw))
        .map_err(|why| format!("cannot append Population V5 fixed record: {why}"))
}

fn open_root_directory(
    root: &Path,
) -> Result<(PathBuf, File, PlatformIdentity), PopulationV5Refusal> {
    require_not_symlink(root, false)?;
    let canonical = std::fs::canonicalize(root).map_err(|why| {
        format!(
            "Population V5 root {} must already exist: {why}",
            root.display()
        )
    })?;
    require_not_symlink(&canonical, false)?;
    let file = File::open(&canonical).map_err(|why| {
        format!(
            "cannot hold Population V5 root {}: {why}",
            canonical.display()
        )
    })?;
    let metadata = file.metadata().map_err(|why| {
        format!(
            "cannot stat held Population V5 root {}: {why}",
            canonical.display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "Population V5 root {} is not a directory",
            canonical.display()
        ));
    }
    Ok((canonical, file, PlatformIdentity::of(&metadata)))
}

fn named_identity(path: &Path) -> Result<PlatformIdentity, PopulationV5Refusal> {
    require_not_symlink(path, false)?;
    let metadata = std::fs::metadata(path).map_err(|why| {
        format!(
            "cannot stat named Population V5 path {}: {why}",
            path.display()
        )
    })?;
    Ok(PlatformIdentity::of(&metadata))
}

fn open_child(
    path: &Path,
    writable: bool,
    create: bool,
) -> Result<(File, bool), PopulationV5Refusal> {
    require_not_symlink(path, create)?;
    if create {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
        options.custom_flags(O_NOFOLLOW_FLAG);
        match options.open(path) {
            Ok(file) => {
                require_regular_file(&file, path)?;
                return Ok((file, true));
            }
            Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(why) => {
                return Err(format!(
                    "cannot create Population V5 file {}: {why}",
                    path.display()
                ));
            }
        }
    }
    require_not_symlink(path, false)?;
    let mut options = OpenOptions::new();
    options.read(true).write(writable).truncate(false);
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    options.custom_flags(O_NOFOLLOW_FLAG);
    let file = options
        .open(path)
        .map_err(|why| format!("cannot open Population V5 file {}: {why}", path.display()))?;
    require_not_symlink(path, false)?;
    require_regular_file(&file, path)?;
    Ok((file, false))
}

fn require_regular_file(file: &File, path: &Path) -> Result<(), PopulationV5Refusal> {
    let metadata = file
        .metadata()
        .map_err(|why| format!("cannot stat Population V5 file {}: {why}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!(
            "Population V5 path {} is not a regular file",
            path.display()
        ));
    }
    require_single_link(&metadata, path)?;
    Ok(())
}

fn require_single_link(
    metadata: &std::fs::Metadata,
    path: &Path,
) -> Result<(), PopulationV5Refusal> {
    #[cfg(unix)]
    if metadata.nlink() != 1 {
        return Err(format!(
            "Population V5 path {} has {} hard links; expected exactly one",
            path.display(),
            metadata.nlink()
        ));
    }
    #[cfg(not(unix))]
    let _ = (metadata, path);
    Ok(())
}

fn require_not_symlink(path: &Path, absent_allowed: bool) -> Result<(), PopulationV5Refusal> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(format!(
            "Population V5 path {} is a symbolic link; no-follow refused it",
            path.display()
        )),
        Ok(_) => Ok(()),
        Err(why) if absent_allowed && why.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(why) => Err(format!(
            "cannot inspect Population V5 path {} without following links: {why}",
            path.display()
        )),
    }
}

fn sync_directory(root_file: &File, root: &Path) -> Result<(), PopulationV5Refusal> {
    root_file.sync_all().map_err(|why| {
        format!(
            "cannot durably sync Population V5 directory {}: {why}",
            root.display()
        )
    })
}

fn file_generation(
    file: &File,
    path: &Path,
    max_bytes: u64,
) -> Result<FileGeneration, PopulationV5Refusal> {
    let before = file.metadata().map_err(|why| {
        format!(
            "cannot stat held Population V5 file {}: {why}",
            path.display()
        )
    })?;
    if !before.is_file() || before.len() > max_bytes {
        return Err(format!(
            "Population V5 file {} is not regular or its {} bytes exceed bound {max_bytes}",
            path.display(),
            before.len()
        ));
    }
    require_single_link(&before, path)?;
    let platform = PlatformIdentity::of(&before);
    if named_identity(path)? != platform {
        return Err(format!(
            "Population V5 file {} was path-replaced",
            path.display()
        ));
    }
    let len = before.len();
    let modified = before.modified().ok();
    let first = hash_held_file(file, path, len)?;
    let middle = file.metadata().map_err(|why| {
        format!(
            "cannot restat Population V5 file {} after first hash: {why}",
            path.display()
        )
    })?;
    require_single_link(&middle, path)?;
    if middle.len() != len
        || PlatformIdentity::of(&middle) != platform
        || middle.modified().ok() != modified
        || named_identity(path)? != platform
    {
        return Err(format!(
            "Population V5 file {} changed during generation hash",
            path.display()
        ));
    }
    let second = hash_held_file(file, path, len)?;
    let after = file.metadata().map_err(|why| {
        format!(
            "cannot restat Population V5 file {} after confirmation hash: {why}",
            path.display()
        )
    })?;
    require_single_link(&after, path)?;
    if first != second
        || after.len() != len
        || PlatformIdentity::of(&after) != platform
        || after.modified().ok() != modified
        || named_identity(path)? != platform
    {
        return Err(format!(
            "Population V5 file {} changed between bounded generation hashes",
            path.display()
        ));
    }
    let mut hasher = Hasher::new();
    hasher.update(FILE_GENERATION_DOMAIN);
    hasher.update(&VERSION.to_le_bytes());
    hasher.update(&len.to_le_bytes());
    platform.update_hasher(&mut hasher);
    hasher.update(&first);
    let generation_digest = hasher.finalize();
    Ok(FileGeneration {
        len,
        platform,
        modified,
        content_digest: first,
        generation_digest,
    })
}

fn hash_held_file(file: &File, path: &Path, len: u64) -> Result<[u8; 32], PopulationV5Refusal> {
    let mut reader = file
        .try_clone()
        .map_err(|why| format!("cannot clone Population V5 file {}: {why}", path.display()))?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|why| format!("cannot seek Population V5 file {}: {why}", path.display()))?;
    let mut hasher = Hasher::new();
    hasher.update(FILE_GENERATION_DOMAIN);
    hasher.update(&len.to_le_bytes());
    let mut remaining = len;
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    while remaining != 0 {
        let requested = usize::try_from(remaining.min(READ_CHUNK_BYTES as u64))
            .map_err(|_| "Population V5 hash width does not fit usize".to_owned())?;
        let target = buffer
            .get_mut(..requested)
            .ok_or_else(|| "Population V5 hash request exceeds fixed buffer".to_owned())?;
        let read = reader
            .read(target)
            .map_err(|why| format!("cannot hash Population V5 file {}: {why}", path.display()))?;
        if read == 0 {
            return Err(format!(
                "Population V5 file {} shortened while hashing",
                path.display()
            ));
        }
        let read_bytes = buffer
            .get(..read)
            .ok_or_else(|| "Population V5 hash read exceeds fixed buffer".to_owned())?;
        hasher.update(read_bytes);
        remaining = remaining
            .checked_sub(
                u64::try_from(read)
                    .map_err(|_| "Population V5 hash read does not fit u64".to_owned())?,
            )
            .ok_or_else(|| "Population V5 hash remaining count underflowed".to_owned())?;
    }
    Ok(hasher.finalize())
}

fn combine_lock_result<T>(
    result: Result<T, PopulationV5Refusal>,
    released: Result<(), PopulationV5Refusal>,
) -> Result<T, PopulationV5Refusal> {
    match (result, released) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(why), _) | (Ok(_), Err(why)) => Err(why),
    }
}

fn hex32(value: [u8; 32]) -> String {
    let mut output = String::with_capacity(64);
    for byte in value {
        output.push(hex_nibble(byte >> 4));
        output.push(hex_nibble(byte & 0x0f));
    }
    output
}

fn hex_nibble(value: u8) -> char {
    let encoded = if value < 10 {
        b'0'.wrapping_add(value)
    } else {
        b'a'.wrapping_add(value.wrapping_sub(10))
    };
    char::from(encoded)
}

struct FixedWriter<'a> {
    bytes: &'a mut [u8],
    cursor: usize,
}

impl<'a> FixedWriter<'a> {
    const fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn array<const N: usize>(&mut self, value: &[u8; N]) -> Result<(), PopulationV5Refusal> {
        self.bytes(value)
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), PopulationV5Refusal> {
        let end = self
            .cursor
            .checked_add(value.len())
            .ok_or_else(|| "Population V5 writer offset overflowed".to_owned())?;
        let target = self
            .bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "Population V5 fixed writer exceeded record".to_owned())?;
        target.copy_from_slice(value);
        self.cursor = end;
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), PopulationV5Refusal> {
        self.bytes(&[value])
    }

    fn u32(&mut self, value: u32) -> Result<(), PopulationV5Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), PopulationV5Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), PopulationV5Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), PopulationV5Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "Population V5 zero-fill offset overflowed".to_owned())?;
        let target = self
            .bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "Population V5 zero-fill exceeded record".to_owned())?;
        target.fill(0);
        self.cursor = end;
        Ok(())
    }

    const fn remaining(&self) -> usize {
        self.bytes.len() - self.cursor
    }

    fn require_full(&self, name: &str) -> Result<(), PopulationV5Refusal> {
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err(format!(
                "Population V5 {name} wrote {} of {} bytes",
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

    fn array<const N: usize>(&mut self) -> Result<[u8; N], PopulationV5Refusal> {
        let end = self
            .cursor
            .checked_add(N)
            .ok_or_else(|| "Population V5 reader offset overflowed".to_owned())?;
        let source = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| "Population V5 fixed reader exceeded record".to_owned())?;
        let mut value = [0_u8; N];
        value.copy_from_slice(source);
        self.cursor = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, PopulationV5Refusal> {
        Ok(self.array::<1>()?[0])
    }

    fn u32(&mut self) -> Result<u32, PopulationV5Refusal> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, PopulationV5Refusal> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn i64(&mut self) -> Result<i64, PopulationV5Refusal> {
        Ok(i64::from_le_bytes(self.array()?))
    }

    fn require_zeros(&mut self, count: usize, name: &str) -> Result<(), PopulationV5Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "Population V5 reserve offset overflowed".to_owned())?;
        let source = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| "Population V5 reserve exceeded record".to_owned())?;
        if source.iter().any(|byte| *byte != 0) {
            return Err(format!("Population V5 {name} is nonzero"));
        }
        self.cursor = end;
        Ok(())
    }

    const fn remaining(&self) -> usize {
        self.bytes.len() - self.cursor
    }
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "fixed-record adversarial tests intentionally mutate exact bytes and fail fixture setup loudly"
)]
mod tests {
    use super::*;
    use crate::candidate_universe::{
        population_v5_test_canonical_candidate_record_for_identity,
        verify_population_v5_canonical_record as verify_candidate_fixture,
    };
    use crate::population_admission_v3::{
        PopulationV5AdmissionFixtureRequest, population_v5_test_admission_fixture_for,
        verify_population_v5_canonical_record as verify_admission_fixture,
    };
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    struct TestRoot {
        path: PathBuf,
    }

    impl TestRoot {
        fn new(label: &str) -> Self {
            let sequence = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "brutex-population-v5-{}-{label}-{sequence}",
                std::process::id()
            ));
            std::fs::create_dir(&path).expect("create Population V5 test root");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let Ok(metadata) = std::fs::symlink_metadata(&self.path) else {
                return;
            };
            if metadata.is_dir() {
                std::fs::remove_dir_all(&self.path).expect("remove Population V5 test root");
            } else {
                std::fs::remove_file(&self.path).expect("remove Population V5 test path");
            }
        }
    }

    fn digest(seed: u64) -> [u8; 32] {
        hash_parts(b"brutex-population-v5-test-value\0", &[&seed.to_le_bytes()])
    }

    fn bounds() -> PopulationV5Bounds {
        PopulationV5Bounds::new(
            16,
            16 * POPULATION_V5_ROW_BYTES as u64,
            8,
            8 * POPULATION_V5_COMPLETION_BYTES as u64,
            4,
        )
        .expect("Population V5 test bounds are coherent")
    }

    fn finalization_snapshot(
        candidate: &AuthenticatedCandidatePopulationRowV1,
        admission: &PopulationAdmissionV3DecisionProjection,
        finalization_id: [u8; 32],
        admission_completion_id: [u8; 32],
    ) -> PopulationV5FinalizationProjection {
        assert_eq!(
            admission.candidate_universe_id(),
            candidate.row().universe_id()
        );
        assert_eq!(
            admission.candidate_semantic_id(),
            candidate.row().candidate_semantic_digest()
        );
        assert_eq!(
            admission.candidate_row_digest(),
            candidate.base_candidate_row_digest()
        );
        let mut snapshot = PopulationV5FinalizationProjection {
            finalization_id,
            candidate_universe_id: admission.candidate_universe_id(),
            candidate_completion_digest: admission.candidate_completion_digest(),
            candidate_semantic_id: admission.candidate_semantic_id(),
            candidate_row_digest: admission.candidate_row_digest(),
            pre_admission_authority_id: admission.pre_admission_authority_id(),
            statistics_audit_id: admission.statistics_audit_id(),
            statistics_completion_digest: admission.statistics_completion_digest(),
            statistics_period_digest: admission.statistics_period_digest(),
            statistics_split_digest: admission.statistics_split_digest(),
            search_pair_id: admission.search_pair_id(),
            search_member_id: admission.search_member_id(),
            search_signal_digest: admission.search_signal_digest(),
            search_signal_bars: admission.search_signal_bars(),
            search_signal_first_ts_micros: admission.search_signal_first_ts_micros(),
            search_signal_last_ts_micros: admission.search_signal_last_ts_micros(),
            search_signal_column_digest: admission.search_signal_column_digest(),
            search_policy_id: admission.search_policy_id(),
            search_full_grid_id: admission.search_full_grid_id(),
            search_long_policy_id: admission.search_long_policy_id(),
            search_long_resolution_id: admission.search_long_resolution_id(),
            search_short_policy_id: admission.search_short_policy_id(),
            search_short_resolution_id: admission.search_short_resolution_id(),
            search_family_id: admission.search_family_id(),
            search_walk_id: admission.search_walk_id(),
            search_fold_count: admission.search_fold_count(),
            search_decided_folds: admission.search_decided_folds(),
            search_profitable_oos_folds: admission.search_profitable_oos_folds(),
            search_aggregate_oos_paisa: admission.search_aggregate_oos_paisa(),
            search_evaluated_population_count: admission.search_evaluated_population_count(),
            paired_base_id: admission.paired_base_id(),
            base_completion_id: admission.base_completion_id(),
            base_evidence_id: admission.base_evidence_id(),
            admission_block_id: admission.block_id(),
            admission_completion_id,
            admission_runner_decision_digest: admission.runner_decision_digest(),
            admission_evidence_digest: admission.runner_evidence_digest(),
            admission_verdict_digest: admission.runner_verdict_digest(),
            admission_decision_id: admission.decision_id(),
            finalization_row_id: [0; 32],
        };
        snapshot.finalization_row_id = snapshot
            .derive_v3_row_id(
                admission.global_sequence(),
                admission.family(),
                admission.family_sequence(),
                admission.status(),
            )
            .expect("derive Finalization V3 fixture row identity");
        snapshot
    }

    struct FixtureCandidate {
        record: [u8; CANDIDATE_RECORD_BYTES],
        candidate: AuthenticatedCandidatePopulationRowV1,
        admission_family: AdmissionV3Family,
        family_sequence: u64,
        status: AdmissionV3Status,
    }

    struct FixtureInput {
        candidate_record: [u8; CANDIDATE_RECORD_BYTES],
        candidate: AuthenticatedCandidatePopulationRowV1,
        admission_record: [u8; POPULATION_ADMISSION_V3_DECISION_BYTES],
        admission: PopulationAdmissionV3EmbeddedProjection,
        admission_identity: PopulationAdmissionV3DecisionProjection,
    }

    fn fixture_candidates(
        nifty_statuses: &[AdmissionV3Status],
        banknifty_statuses: &[AdmissionV3Status],
    ) -> (u64, u64, Vec<FixtureCandidate>) {
        let nifty_count = u64::try_from(nifty_statuses.len()).expect("fixture count fits u64");
        let banknifty_count =
            u64::try_from(banknifty_statuses.len()).expect("fixture count fits u64");
        let total = nifty_count
            .checked_add(banknifty_count)
            .expect("fixture count does not overflow");
        assert!(total > 0 && total <= 4);
        assert!(nifty_count <= 2 && banknifty_count <= 2);
        let mut candidates = Vec::new();
        for global_sequence in 0..total {
            let (instrument_family, admission_family, family_sequence, status, universe_tag) =
                if global_sequence < nifty_count {
                    (
                        InstrumentFamilyV1::Nifty,
                        AdmissionV3Family::Nifty,
                        global_sequence,
                        nifty_statuses[usize::try_from(global_sequence)
                            .expect("fixture NIFTY ordinal fits usize")],
                        20,
                    )
                } else {
                    let family_sequence = global_sequence - nifty_count;
                    (
                        InstrumentFamilyV1::BankNifty,
                        AdmissionV3Family::BankNifty,
                        family_sequence,
                        banknifty_statuses[usize::try_from(family_sequence)
                            .expect("fixture BANKNIFTY ordinal fits usize")],
                        30,
                    )
                };
            let semantic_tag =
                u8::try_from(40 + global_sequence).expect("fixture semantic tag fits u8");
            let candidate_record = population_v5_test_canonical_candidate_record_for_identity(
                instrument_family,
                family_sequence,
                universe_tag,
                semantic_tag,
            );
            let candidate = verify_candidate_fixture(&candidate_record)
                .expect("parameterized Candidate fixture verifies");
            candidates.push(FixtureCandidate {
                record: candidate_record,
                candidate,
                admission_family,
                family_sequence,
                status,
            });
        }
        (nifty_count, banknifty_count, candidates)
    }

    fn fixture_universe_ids(candidates: &[FixtureCandidate]) -> [[u8; 32]; 2] {
        [
            candidates
                .iter()
                .find(|candidate| candidate.admission_family == AdmissionV3Family::Nifty)
                .map_or(digest(3_000), |candidate| {
                    candidate.candidate.row().universe_id()
                }),
            candidates
                .iter()
                .find(|candidate| candidate.admission_family == AdmissionV3Family::BankNifty)
                .map_or(digest(3_001), |candidate| {
                    candidate.candidate.row().universe_id()
                }),
        ]
    }

    fn fixture_inputs(candidates: Vec<FixtureCandidate>) -> Vec<FixtureInput> {
        let candidate_universe_ids = fixture_universe_ids(&candidates);
        let candidate_completion_digests = [digest(3_010), digest(3_011)];
        let mut inputs = Vec::new();
        for (global_sequence, candidate) in candidates.into_iter().enumerate() {
            let global_sequence = u64::try_from(global_sequence).expect("fixture ordinal fits u64");
            let (admission_record, admission_identity) =
                population_v5_test_admission_fixture_for(PopulationV5AdmissionFixtureRequest {
                    candidate_universe_ids,
                    candidate_completion_digests,
                    candidate_semantic_id: candidate.candidate.row().candidate_semantic_digest(),
                    candidate_row_digest: candidate.candidate.base_candidate_row_digest(),
                    family: candidate.admission_family,
                    global_sequence,
                    family_sequence: candidate.family_sequence,
                    status: candidate.status,
                });
            let admission = verify_admission_fixture(&admission_record)
                .expect("parameterized Admission fixture verifies");
            assert_eq!(admission.block_id(), admission_identity.block_id());
            inputs.push(FixtureInput {
                candidate_record: candidate.record,
                candidate: candidate.candidate,
                admission_record,
                admission,
                admission_identity,
            });
        }
        inputs
    }

    fn fixture_source(
        nifty_statuses: &[AdmissionV3Status],
        banknifty_statuses: &[AdmissionV3Status],
        nifty_count: u64,
        banknifty_count: u64,
        inputs: &[FixtureInput],
    ) -> SourceFinalizationV3Receipt {
        let admission_block_id = inputs[0].admission.block_id();
        let admission_completion_id = inputs[0].admission_identity.completion_id();
        assert!(
            inputs
                .iter()
                .all(|input| input.admission.block_id() == admission_block_id
                    && input.admission_identity.completion_id() == admission_completion_id)
        );
        let mut status_counts = [0_u64; 4];
        for status in nifty_statuses.iter().chain(banknifty_statuses).copied() {
            increment_status_count(&mut status_counts, status, "fixture")
                .expect("fixture status count remains in range");
        }
        let [
            admitted_count,
            rejected_count,
            unmeasured_count,
            refused_count,
        ] = status_counts;
        SourceFinalizationV3Receipt {
            finalization_id: digest(2_000),
            admission_block_id,
            admission_completion_id,
            row_count: nifty_count
                .checked_add(banknifty_count)
                .expect("fixture count does not overflow"),
            nifty_row_count: nifty_count,
            banknifty_row_count: banknifty_count,
            admitted_count,
            rejected_count,
            unmeasured_count,
            refused_count,
            ordered_row_digest: digest(2_002),
            completion_id: digest(2_003),
        }
    }

    fn fixture_rows(
        source: &SourceFinalizationV3Receipt,
        inputs: Vec<FixtureInput>,
    ) -> (
        Vec<PopulationV5RowRecord>,
        Vec<PopulationAdmissionV3DecisionProjection>,
    ) {
        let population_id = derive_population_id(source);
        let mut rows = Vec::new();
        let mut admission_identities = Vec::new();
        for (global_sequence, input) in inputs.into_iter().enumerate() {
            let global_sequence = u64::try_from(global_sequence).expect("fixture ordinal fits u64");
            let finalization = finalization_snapshot(
                &input.candidate,
                &input.admission_identity,
                source.finalization_id,
                source.admission_completion_id,
            );
            let mut row = PopulationV5RowRecord {
                population_id,
                global_sequence,
                family: input.admission.family(),
                family_sequence: input.admission.family_sequence(),
                status: input.admission.status(),
                candidate_record: input.candidate_record,
                admission_record: input.admission_record,
                finalization_completion_id: source.completion_id,
                finalization,
                row_id: [0; 32],
            };
            row.row_id = row
                .derive_row_id()
                .expect("derive Population V5 fixture row");
            row.validate().expect("Population V5 fixture row validates");
            row.validate_live_admission_join(&input.admission_identity)
                .expect("Population V5 fixture joins every live Admission field");
            rows.push(row);
            admission_identities.push(input.admission_identity);
        }
        (rows, admission_identities)
    }

    fn prepared_with_statuses(
        nifty_statuses: &[AdmissionV3Status],
        banknifty_statuses: &[AdmissionV3Status],
    ) -> (
        PreparedPopulationV5,
        Vec<PopulationAdmissionV3DecisionProjection>,
    ) {
        let (nifty_count, banknifty_count, candidates) =
            fixture_candidates(nifty_statuses, banknifty_statuses);
        let inputs = fixture_inputs(candidates);
        let source = fixture_source(
            nifty_statuses,
            banknifty_statuses,
            nifty_count,
            banknifty_count,
            &inputs,
        );
        let population_id = derive_population_id(&source);
        let (rows, admission_identities) = fixture_rows(&source, inputs);
        let value = PreparedPopulationV5 {
            population_id,
            rows,
            nifty_count,
            banknifty_count,
            admitted_count: source.admitted_count,
            rejected_count: source.rejected_count,
            unmeasured_count: source.unmeasured_count,
            refused_count: source.refused_count,
            source,
        };
        value
            .validate(bounds())
            .expect("Population V5 fixture preparation validates");
        (value, admission_identities)
    }

    fn prepared_with_admission_identities(
        nifty_count: u64,
        banknifty_count: u64,
    ) -> (
        PreparedPopulationV5,
        Vec<PopulationAdmissionV3DecisionProjection>,
    ) {
        let nifty = vec![
            AdmissionV3Status::Admitted;
            usize::try_from(nifty_count).expect("fixture NIFTY count fits usize")
        ];
        let banknifty = vec![
            AdmissionV3Status::Admitted;
            usize::try_from(banknifty_count)
                .expect("fixture BANKNIFTY count fits usize")
        ];
        prepared_with_statuses(&nifty, &banknifty)
    }

    fn prepared(nifty_count: u64, banknifty_count: u64) -> PreparedPopulationV5 {
        prepared_with_admission_identities(nifty_count, banknifty_count).0
    }

    fn create_empty_files(root: &Path) {
        drop(
            PopulationV5Ledger::open_write(root, bounds())
                .expect("create empty Population V5 files"),
        );
    }

    fn assert_row_codec_hardening(prepared: &PreparedPopulationV5) {
        let raw = prepared.rows[0]
            .encode()
            .expect("fixture row encodes canonically");
        assert_eq!(raw.len(), POPULATION_V5_ROW_BYTES);
        assert_eq!(
            PopulationV5RowRecord::decode(&raw)
                .expect("fixture row decodes")
                .encode()
                .expect("decoded row re-encodes"),
            raw
        );
        for index in 0..POPULATION_V5_ROW_BYTES {
            let mut changed = raw;
            changed[index] ^= 1;
            assert!(
                PopulationV5RowRecord::decode(&changed).is_err(),
                "unresealed Population V5 row byte {index} must fail"
            );
        }
        let mut wrong_magic = raw;
        wrong_magic[0] ^= 1;
        let wrong_magic_seal = hash_parts(ROW_SEAL_DOMAIN, &[&wrong_magic[..ROW_PAYLOAD_BYTES]]);
        wrong_magic[ROW_PAYLOAD_BYTES..].copy_from_slice(&wrong_magic_seal);
        assert_eq!(
            PopulationV5RowRecord::decode(&wrong_magic)
                .expect_err("resealed Population V5 row magic must fail"),
            "Population V5 row magic mismatch"
        );
        for unknown_family in [0_u8, 3, u8::MAX] {
            let mut changed = raw;
            changed[64] = unknown_family;
            let changed_seal = hash_parts(ROW_SEAL_DOMAIN, &[&changed[..ROW_PAYLOAD_BYTES]]);
            changed[ROW_PAYLOAD_BYTES..].copy_from_slice(&changed_seal);
            assert_eq!(
                PopulationV5RowRecord::decode(&changed)
                    .expect_err("resealed unknown Population V5 family must fail"),
                format!("Population V5 family tag {unknown_family} is unknown")
            );
        }
        for unknown_status in [0_u8, 5, u8::MAX] {
            let mut changed = raw;
            changed[80] = unknown_status;
            let changed_seal = hash_parts(ROW_SEAL_DOMAIN, &[&changed[..ROW_PAYLOAD_BYTES]]);
            changed[ROW_PAYLOAD_BYTES..].copy_from_slice(&changed_seal);
            assert_eq!(
                PopulationV5RowRecord::decode(&changed)
                    .expect_err("resealed unknown Population V5 status must fail"),
                format!("Population V5 status tag {unknown_status} is unknown")
            );
        }
        let mut reserved = raw;
        reserved[ROW_PAYLOAD_BYTES - 1] = 1;
        let reserve_seal = hash_parts(ROW_SEAL_DOMAIN, &[&reserved[..ROW_PAYLOAD_BYTES]]);
        reserved[ROW_PAYLOAD_BYTES..].copy_from_slice(&reserve_seal);
        assert!(
            PopulationV5RowRecord::decode(&reserved)
                .expect_err("resealed Population V5 reserve must fail")
                .contains("reserve")
        );
        for offset in [16_usize, 20] {
            let mut changed = raw;
            changed[offset] ^= 1;
            let changed_seal = hash_parts(ROW_SEAL_DOMAIN, &[&changed[..ROW_PAYLOAD_BYTES]]);
            changed[ROW_PAYLOAD_BYTES..].copy_from_slice(&changed_seal);
            assert!(
                PopulationV5RowRecord::decode(&changed)
                    .expect_err("resealed Population V5 version/domain must fail")
                    .contains("version/domain")
            );
        }
    }

    fn assert_completion_codec_hardening(prepared: &PreparedPopulationV5) {
        let completion = prepared
            .expected_completion(0, 0)
            .expect("fixture Completion derives");
        let completion_raw = completion.encode().expect("fixture Completion encodes");
        assert_eq!(completion_raw.len(), POPULATION_V5_COMPLETION_BYTES);
        assert_eq!(
            PopulationV5CompletionRecord::decode(&completion_raw)
                .expect("fixture Completion decodes"),
            completion
        );
        for index in 0..POPULATION_V5_COMPLETION_BYTES {
            let mut changed = completion_raw;
            changed[index] ^= 1;
            assert!(
                PopulationV5CompletionRecord::decode(&changed).is_err(),
                "unresealed Population V5 Completion byte {index} must fail"
            );
        }
        let mut wrong_magic = completion_raw;
        wrong_magic[0] ^= 1;
        let wrong_magic_seal = hash_parts(
            COMPLETION_SEAL_DOMAIN,
            &[&wrong_magic[..COMPLETION_PAYLOAD_BYTES]],
        );
        wrong_magic[COMPLETION_PAYLOAD_BYTES..].copy_from_slice(&wrong_magic_seal);
        assert_eq!(
            PopulationV5CompletionRecord::decode(&wrong_magic)
                .expect_err("resealed Population V5 Completion magic must fail"),
            "Population V5 Completion magic mismatch"
        );
        let mut reserved = completion_raw;
        reserved[COMPLETION_PAYLOAD_BYTES - 1] = 1;
        let reserve_seal = hash_parts(
            COMPLETION_SEAL_DOMAIN,
            &[&reserved[..COMPLETION_PAYLOAD_BYTES]],
        );
        reserved[COMPLETION_PAYLOAD_BYTES..].copy_from_slice(&reserve_seal);
        let reserve_refusal = PopulationV5CompletionRecord::decode(&reserved)
            .expect_err("resealed Population V5 Completion reserve must fail");
        assert!(
            reserve_refusal.ends_with("Completion trailing reserve is nonzero"),
            "unexpected Completion reserve refusal: {reserve_refusal}"
        );
        for offset in [16_usize, 20] {
            let mut changed = completion_raw;
            changed[offset] ^= 1;
            let changed_seal = hash_parts(
                COMPLETION_SEAL_DOMAIN,
                &[&changed[..COMPLETION_PAYLOAD_BYTES]],
            );
            changed[COMPLETION_PAYLOAD_BYTES..].copy_from_slice(&changed_seal);
            assert!(
                PopulationV5CompletionRecord::decode(&changed)
                    .expect_err("resealed Population V5 Completion version/domain must fail")
                    .contains("version/domain")
            );
        }
    }

    #[test]
    fn bounds_and_fixed_codecs_are_explicit_canonical_and_fully_sealed() {
        assert!(PopulationV5Bounds::new(0, 1, 1, 1_024, 1).is_err());
        assert!(PopulationV5Bounds::new(1, 4_095, 1, 1_024, 1).is_err());
        assert!(PopulationV5Bounds::new(1, 4_096, 1, 1_023, 1).is_err());
        assert!(PopulationV5Bounds::new(1, 4_096, 1, 1_024, 2).is_err());

        let prepared = prepared(1, 1);
        assert_row_codec_hardening(&prepared);
        assert_completion_codec_hardening(&prepared);
        let completion = prepared
            .expected_completion(0, 0)
            .expect("fixture Completion derives");

        let mut changed_source_order = completion.clone();
        changed_source_order.source_finalization_ordered_row_digest[0] ^= 1;
        assert!(
            changed_source_order
                .validate()
                .expect_err("source-order mutation must invalidate Population identity")
                .contains("does not reproduce")
        );

        assert_eq!(decode_status(1), Ok(AdmissionV3Status::Admitted));
        assert_eq!(decode_status(2), Ok(AdmissionV3Status::Rejected));
        assert_eq!(decode_status(3), Ok(AdmissionV3Status::Unmeasured));
        assert_eq!(decode_status(4), Ok(AdmissionV3Status::Refused));
        assert!(decode_status(0).is_err());
        assert!(decode_status(5).is_err());
    }

    #[test]
    fn semantic_completion_identity_is_relocation_stable_while_offsets_remain_structural() {
        let prepared = prepared(1, 1);
        let original = prepared
            .expected_completion(0, 0)
            .expect("fixture Completion derives");
        let mut relocated = original.clone();
        relocated.block_sequence = 17;
        relocated.first_row_record = 91;
        assert_eq!(relocated.derive_completion_id(), original.completion_id);
        assert_eq!(relocated.completion_id, original.completion_id);
        relocated
            .validate()
            .expect("physical relocation does not rekey semantic Completion");
        let relocated_raw = relocated.encode().expect("relocated Completion encodes");
        assert_ne!(
            relocated_raw,
            original.encode().expect("original Completion encodes"),
            "physical coordinates remain persisted structural facts"
        );
        assert_eq!(
            PopulationV5CompletionRecord::decode(&relocated_raw)
                .expect("relocated Completion decodes"),
            relocated
        );

        let root = TestRoot::new("illegal-physical-relocation");
        {
            let mut writer = PopulationV5Ledger::open_write(root.path(), bounds())
                .expect("physical-range fixture writer opens");
            writer
                .append(&prepared)
                .expect("physical-range fixture writes");
        }
        let completion_path = root.path().join(COMPLETION_FILE);
        let stored: [u8; POPULATION_V5_COMPLETION_BYTES] = std::fs::read(&completion_path)
            .expect("read physical-range Completion")
            .try_into()
            .expect("one exact Completion record");
        let mut physically_wrong =
            PopulationV5CompletionRecord::decode(&stored).expect("stored Completion decodes");
        physically_wrong.first_row_record = 91;
        assert_eq!(
            physically_wrong.completion_id, original.completion_id,
            "physical mutation does not rekey semantic identity"
        );
        std::fs::write(
            &completion_path,
            physically_wrong
                .encode()
                .expect("physically wrong but semantically valid Completion encodes"),
        )
        .expect("write physically relocated Completion");
        let Err(physical_refusal) = PopulationV5Ledger::open_read(root.path(), bounds()) else {
            panic!("ledger scan must reject a noncontiguous physical range");
        };
        assert_eq!(
            physical_refusal,
            "Population V5 Completion 0 is not contiguous/canonical"
        );
    }

    #[test]
    fn zero_family_blocks_remain_canonical_without_invented_rows() {
        let nifty_only = prepared(1, 0);
        nifty_only
            .validate(bounds())
            .expect("NIFTY-only source remains canonical");
        assert_eq!(nifty_only.rows.len(), 1);
        assert_eq!(nifty_only.rows[0].family, AdmissionV3Family::Nifty);

        let banknifty_only = prepared(0, 1);
        banknifty_only
            .validate(bounds())
            .expect("BANKNIFTY-only source remains canonical");
        assert_eq!(banknifty_only.rows.len(), 1);
        assert_eq!(banknifty_only.rows[0].family, AdmissionV3Family::BankNifty);
        assert_eq!(banknifty_only.rows[0].global_sequence, 0);
        assert_eq!(banknifty_only.rows[0].family_sequence, 0);
    }

    #[test]
    fn all_four_status_counts_and_nifty_first_matrix_survive_receipt_last_reopen() {
        let nifty = [AdmissionV3Status::Admitted, AdmissionV3Status::Rejected];
        let banknifty = [AdmissionV3Status::Unmeasured, AdmissionV3Status::Refused];
        let prepared = prepared_with_statuses(&nifty, &banknifty).0;
        assert_eq!(prepared.admitted_count, 1);
        assert_eq!(prepared.rejected_count, 1);
        assert_eq!(prepared.unmeasured_count, 1);
        assert_eq!(prepared.refused_count, 1);
        let expected_completion = prepared
            .expected_completion(0, 0)
            .expect("four-status Completion derives");
        assert_eq!(expected_completion.admitted_count, 1);
        assert_eq!(expected_completion.rejected_count, 1);
        assert_eq!(expected_completion.unmeasured_count, 1);
        assert_eq!(expected_completion.refused_count, 1);

        let root = TestRoot::new("four-status-matrix");
        let receipt = {
            let mut writer = PopulationV5Ledger::open_write(root.path(), bounds())
                .expect("four-status writer opens");
            writer
                .append(&prepared)
                .expect("four-status Population writes")
                .receipt()
        };
        assert_eq!(receipt.row_count(), 4);
        assert_eq!(receipt.nifty_count(), 2);
        assert_eq!(receipt.banknifty_count(), 2);
        assert_eq!(receipt.admitted_count(), 1);
        assert_eq!(receipt.rejected_count(), 1);
        assert_eq!(receipt.unmeasured_count(), 1);
        assert_eq!(receipt.refused_count(), 1);
        assert_eq!(
            receipt.ordered_row_digest(),
            ordered_row_digest(&prepared.rows).expect("four-status ordered row digest derives")
        );
        assert_eq!(
            receipt.source_admission_block_id(),
            prepared.source.admission_block_id
        );
        assert_eq!(
            receipt.source_admission_completion_id(),
            prepared.source.admission_completion_id
        );

        let mut reopened = PopulationV5Ledger::open_read(root.path(), bounds())
            .expect("four-status fresh reader opens");
        let rows = reopened
            .authenticated_rows(&receipt)
            .expect("four-status rows authenticate in order");
        let matrix: Vec<_> = rows
            .iter()
            .map(|row| {
                (
                    row.global_sequence(),
                    row.family(),
                    row.family_sequence(),
                    row.status(),
                )
            })
            .collect();
        assert_eq!(
            matrix,
            vec![
                (0, AdmissionV3Family::Nifty, 0, AdmissionV3Status::Admitted),
                (1, AdmissionV3Family::Nifty, 1, AdmissionV3Status::Rejected),
                (
                    2,
                    AdmissionV3Family::BankNifty,
                    0,
                    AdmissionV3Status::Unmeasured,
                ),
                (
                    3,
                    AdmissionV3Family::BankNifty,
                    1,
                    AdmissionV3Status::Refused,
                ),
            ]
        );
    }

    #[test]
    fn multiple_blocks_fill_exact_row_and_completion_bounds_without_partial_append() {
        let populations = [
            prepared_with_statuses(
                &[AdmissionV3Status::Admitted],
                &[AdmissionV3Status::Admitted],
            )
            .0,
            prepared_with_statuses(
                &[AdmissionV3Status::Rejected],
                &[AdmissionV3Status::Admitted],
            )
            .0,
            prepared_with_statuses(
                &[AdmissionV3Status::Unmeasured],
                &[AdmissionV3Status::Refused],
            )
            .0,
        ];
        assert_ne!(populations[0].population_id, populations[1].population_id);
        assert_ne!(populations[1].population_id, populations[2].population_id);
        assert_ne!(populations[0].population_id, populations[2].population_id);
        assert_row_record_bound(&populations);
        assert_completion_record_bound(&populations);
    }

    fn assert_row_record_bound(populations: &[PreparedPopulationV5; 3]) {
        let row_limited = PopulationV5Bounds::new(
            4,
            4 * POPULATION_V5_ROW_BYTES as u64,
            3,
            3 * POPULATION_V5_COMPLETION_BYTES as u64,
            2,
        )
        .expect("row-limited multi-block bounds are coherent");
        let row_root = TestRoot::new("row-bound-exhaustion");
        let row_receipts = {
            let mut writer = PopulationV5Ledger::open_write(row_root.path(), row_limited)
                .expect("row-limited writer opens");
            let first = writer
                .append(&populations[0])
                .expect("first bounded Population writes")
                .receipt();
            let second = writer
                .append(&populations[1])
                .expect("second bounded Population writes")
                .receipt();
            assert_eq!((first.block_sequence(), first.first_row_record()), (0, 0));
            assert_eq!((second.block_sequence(), second.first_row_record()), (1, 2));
            [first, second]
        };
        let row_path = row_root.path().join(ROW_FILE);
        let row_completion_path = row_root.path().join(COMPLETION_FILE);
        let rows_before = std::fs::read(&row_path).expect("snapshot row-bound rows");
        let completions_before =
            std::fs::read(&row_completion_path).expect("snapshot row-bound Completions");
        {
            let mut writer = PopulationV5Ledger::open_write(row_root.path(), row_limited)
                .expect("full row-limited writer reopens");
            assert_eq!(
                writer
                    .append(&populations[2])
                    .expect_err("third Population must exceed row-record bound"),
                "Population V5 append reaches 6 rows above bound 4"
            );
        }
        assert_eq!(
            std::fs::read(&row_path).expect("read rows after row-bound refusal"),
            rows_before
        );
        assert_eq!(
            std::fs::read(&row_completion_path).expect("read Completions after row-bound refusal"),
            completions_before
        );
        let mut row_reader = PopulationV5Ledger::open_read(row_root.path(), row_limited)
            .expect("exactly full multi-block ledger reopens");
        for (population, receipt) in populations.iter().zip(row_receipts) {
            assert_eq!(
                row_reader
                    .authenticated_rows(&receipt)
                    .expect("completed bounded block authenticates")
                    .len(),
                population.rows.len()
            );
        }
    }

    fn assert_completion_record_bound(populations: &[PreparedPopulationV5; 3]) {
        let completion_limited = PopulationV5Bounds::new(
            6,
            6 * POPULATION_V5_ROW_BYTES as u64,
            2,
            2 * POPULATION_V5_COMPLETION_BYTES as u64,
            2,
        )
        .expect("Completion-limited multi-block bounds are coherent");
        let completion_root = TestRoot::new("completion-bound-exhaustion");
        {
            let mut writer =
                PopulationV5Ledger::open_write(completion_root.path(), completion_limited)
                    .expect("Completion-limited writer opens");
            writer
                .append(&populations[0])
                .expect("first Completion-bounded Population writes");
            writer
                .append(&populations[1])
                .expect("second Completion-bounded Population writes");
        }
        let completion_row_path = completion_root.path().join(ROW_FILE);
        let completion_path = completion_root.path().join(COMPLETION_FILE);
        let rows_before =
            std::fs::read(&completion_row_path).expect("snapshot Completion-bounded rows");
        let completions_before =
            std::fs::read(&completion_path).expect("snapshot Completion-bound receipts");
        {
            let mut writer =
                PopulationV5Ledger::open_write(completion_root.path(), completion_limited)
                    .expect("full Completion-limited writer reopens");
            assert_eq!(
                writer
                    .append(&populations[2])
                    .expect_err("third Population must exceed Completion-record bound"),
                "Population V5 append reaches 3 Completions above bound 2"
            );
        }
        assert_eq!(
            std::fs::read(&completion_row_path).expect("read rows after Completion-bound refusal"),
            rows_before
        );
        assert_eq!(
            std::fs::read(&completion_path).expect("read receipts after Completion-bound refusal"),
            completions_before
        );
    }

    #[test]
    fn every_repeated_live_admission_field_is_load_bearing() {
        let (prepared, identities) = prepared_with_admission_identities(1, 1);
        let baseline = &prepared.rows[0];
        let identity = identities[0];
        baseline
            .validate_live_admission_join(&identity)
            .expect("exact live Admission projection joins");

        macro_rules! refuse_array_field {
            ($field:ident) => {{
                let mut changed = baseline.clone();
                changed.finalization.$field[0] ^= 1;
                assert!(
                    changed.validate_live_admission_join(&identity).is_err(),
                    concat!(stringify!($field), " must be load-bearing")
                );
            }};
        }
        macro_rules! refuse_scalar_field {
            ($field:ident) => {{
                let mut changed = baseline.clone();
                changed.finalization.$field ^= 1;
                assert!(
                    changed.validate_live_admission_join(&identity).is_err(),
                    concat!(stringify!($field), " must be load-bearing")
                );
            }};
        }

        refuse_array_field!(candidate_universe_id);
        refuse_array_field!(candidate_completion_digest);
        refuse_array_field!(candidate_semantic_id);
        refuse_array_field!(candidate_row_digest);
        refuse_array_field!(pre_admission_authority_id);
        refuse_array_field!(statistics_audit_id);
        refuse_array_field!(statistics_completion_digest);
        refuse_array_field!(statistics_period_digest);
        refuse_array_field!(statistics_split_digest);
        refuse_array_field!(search_pair_id);
        refuse_array_field!(search_member_id);
        refuse_array_field!(search_signal_digest);
        refuse_array_field!(search_signal_column_digest);
        refuse_array_field!(search_policy_id);
        refuse_array_field!(search_full_grid_id);
        refuse_array_field!(search_long_policy_id);
        refuse_array_field!(search_long_resolution_id);
        refuse_array_field!(search_short_policy_id);
        refuse_array_field!(search_short_resolution_id);
        refuse_array_field!(search_family_id);
        refuse_array_field!(search_walk_id);
        refuse_array_field!(paired_base_id);
        refuse_array_field!(base_completion_id);
        refuse_array_field!(base_evidence_id);
        refuse_array_field!(admission_block_id);
        refuse_array_field!(admission_completion_id);
        refuse_array_field!(admission_runner_decision_digest);
        refuse_array_field!(admission_evidence_digest);
        refuse_array_field!(admission_verdict_digest);
        refuse_array_field!(admission_decision_id);

        refuse_scalar_field!(search_signal_bars);
        refuse_scalar_field!(search_signal_first_ts_micros);
        refuse_scalar_field!(search_signal_last_ts_micros);
        refuse_scalar_field!(search_fold_count);
        refuse_scalar_field!(search_decided_folds);
        refuse_scalar_field!(search_profitable_oos_folds);
        refuse_scalar_field!(search_aggregate_oos_paisa);
        refuse_scalar_field!(search_evaluated_population_count);

        let mut ordinal = baseline.clone();
        ordinal.global_sequence ^= 1;
        assert!(ordinal.validate_live_admission_join(&identity).is_err());
        let mut family_sequence = baseline.clone();
        family_sequence.family_sequence ^= 1;
        assert!(
            family_sequence
                .validate_live_admission_join(&identity)
                .is_err()
        );
        let mut family = baseline.clone();
        family.family = AdmissionV3Family::BankNifty;
        assert!(family.validate_live_admission_join(&identity).is_err());
        let mut status = baseline.clone();
        status.status = AdmissionV3Status::Rejected;
        assert!(status.validate_live_admission_join(&identity).is_err());
    }

    #[test]
    fn authenticated_successor_exposes_every_typed_finalization_field_without_redecoding() {
        let prepared = prepared(1, 1);
        let expected_row = &prepared.rows[0];
        let successor = PopulationV5SuccessorRow::authenticate(
            &expected_row
                .encode()
                .expect("typed Finalization fixture row encodes"),
        )
        .expect("typed Finalization fixture authenticates");
        let actual = successor.finalization();
        let expected = &expected_row.finalization;

        macro_rules! assert_projection_field {
            ($field:ident) => {
                assert_eq!(actual.$field(), expected.$field)
            };
        }

        assert_projection_field!(finalization_id);
        assert_projection_field!(candidate_universe_id);
        assert_projection_field!(candidate_completion_digest);
        assert_projection_field!(candidate_semantic_id);
        assert_projection_field!(candidate_row_digest);
        assert_projection_field!(pre_admission_authority_id);
        assert_projection_field!(statistics_audit_id);
        assert_projection_field!(statistics_completion_digest);
        assert_projection_field!(statistics_period_digest);
        assert_projection_field!(statistics_split_digest);
        assert_projection_field!(search_pair_id);
        assert_projection_field!(search_member_id);
        assert_projection_field!(search_signal_digest);
        assert_projection_field!(search_signal_bars);
        assert_projection_field!(search_signal_first_ts_micros);
        assert_projection_field!(search_signal_last_ts_micros);
        assert_projection_field!(search_signal_column_digest);
        assert_projection_field!(search_policy_id);
        assert_projection_field!(search_full_grid_id);
        assert_projection_field!(search_long_policy_id);
        assert_projection_field!(search_long_resolution_id);
        assert_projection_field!(search_short_policy_id);
        assert_projection_field!(search_short_resolution_id);
        assert_projection_field!(search_family_id);
        assert_projection_field!(search_walk_id);
        assert_projection_field!(search_fold_count);
        assert_projection_field!(search_decided_folds);
        assert_projection_field!(search_profitable_oos_folds);
        assert_projection_field!(search_aggregate_oos_paisa);
        assert_projection_field!(search_evaluated_population_count);
        assert_projection_field!(paired_base_id);
        assert_projection_field!(base_completion_id);
        assert_projection_field!(base_evidence_id);
        assert_projection_field!(admission_block_id);
        assert_projection_field!(admission_completion_id);
        assert_projection_field!(admission_runner_decision_digest);
        assert_projection_field!(admission_evidence_digest);
        assert_projection_field!(admission_verdict_digest);
        assert_projection_field!(admission_decision_id);
        assert_projection_field!(finalization_row_id);
        assert_eq!(
            successor.finalization_completion_id(),
            expected_row.finalization_completion_id
        );
    }

    #[test]
    fn receipt_last_write_fresh_reopen_one_row_bulk_and_exact_reuse_are_green() {
        let root = TestRoot::new("write-reuse");
        let prepared = prepared(1, 1);
        let first = {
            let mut writer = PopulationV5Ledger::open_write(root.path(), bounds())
                .expect("Population V5 writer opens");
            writer.append(&prepared).expect("Population V5 writes")
        };
        assert!(first.was_written());
        let receipt = first.receipt();
        let mut reopened = PopulationV5Ledger::open_read(root.path(), bounds())
            .expect("Population V5 fresh reader opens");
        assert_eq!(
            reopened
                .structural_receipt(&prepared.population_id)
                .expect("receipt lookup succeeds"),
            Some(receipt)
        );
        reopened
            .require_exact_prepared(&receipt, &prepared)
            .expect("fresh reopen reproduces exact bytes");
        let one = reopened
            .authenticated_row(&receipt, 1)
            .expect("fixed-offset successor read succeeds");
        assert_eq!(one.global_sequence(), 1);
        assert_eq!(one.family(), AdmissionV3Family::BankNifty);
        let all = reopened
            .authenticated_rows(&receipt)
            .expect("ordered successor read succeeds");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].family(), AdmissionV3Family::Nifty);
        assert_eq!(all[1].family(), AdmissionV3Family::BankNifty);
        drop(reopened);

        let second = {
            let mut writer = PopulationV5Ledger::open_write(root.path(), bounds())
                .expect("Population V5 retry writer opens");
            writer
                .append(&prepared)
                .expect("Population V5 exact retry reuses")
        };
        assert!(!second.was_written());
        assert_eq!(second.receipt(), receipt);
        assert_eq!(
            std::fs::metadata(root.path().join(ROW_FILE))
                .expect("stat reused rows")
                .len(),
            2 * POPULATION_V5_ROW_BYTES as u64
        );
        assert_eq!(
            std::fs::metadata(root.path().join(COMPLETION_FILE))
                .expect("stat reused Completions")
                .len(),
            POPULATION_V5_COMPLETION_BYTES as u64
        );
    }

    #[test]
    fn every_exact_canonical_prefix_appends_only_its_suffix_then_reuses_byte_exactly() {
        for prefix in 0_usize..=2 {
            let root = TestRoot::new(&format!("prefix-{prefix}"));
            let prepared = prepared(1, 1);
            create_empty_files(root.path());
            let row_path = root.path().join(ROW_FILE);
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&row_path)
                .expect("open orphan row file");
            for row in &prepared.rows[..prefix] {
                append_raw(&mut file, &row.encode().expect("encode orphan row"))
                    .expect("append orphan row");
            }
            file.sync_data().expect("sync orphan rows");
            drop(file);
            let prefix_bytes = std::fs::read(&row_path).expect("read exact orphan prefix");
            let commit = {
                let mut writer = PopulationV5Ledger::open_write(root.path(), bounds())
                    .expect("orphan-aware writer opens");
                writer
                    .append(&prepared)
                    .expect("exact canonical prefix appends only missing suffix")
            };
            assert!(commit.was_written());
            let completed_rows = std::fs::read(&row_path).expect("read completed prefix rows");
            assert_eq!(
                &completed_rows[..prefix_bytes.len()],
                prefix_bytes.as_slice(),
                "existing canonical prefix bytes must never be overwritten or truncated"
            );
            let mut expected_rows = Vec::new();
            for row in &prepared.rows {
                expected_rows.extend_from_slice(&row.encode().expect("encode expected row"));
            }
            assert_eq!(completed_rows, expected_rows);
            let completion_path = root.path().join(COMPLETION_FILE);
            let completed_receipt =
                std::fs::read(&completion_path).expect("read completed prefix receipt");
            assert_eq!(completed_receipt.len(), POPULATION_V5_COMPLETION_BYTES);

            let retry = {
                let mut writer = PopulationV5Ledger::open_write(root.path(), bounds())
                    .expect("prefix retry writer opens");
                writer
                    .append(&prepared)
                    .expect("completed prefix retry reuses")
            };
            assert!(!retry.was_written());
            assert_eq!(
                std::fs::read(&row_path).expect("read rows after prefix retry"),
                expected_rows
            );
            assert_eq!(
                std::fs::read(&completion_path).expect("read receipt after prefix retry"),
                completed_receipt
            );
        }
    }

    #[test]
    fn foreign_orphan_and_orphan_completion_fail_closed_without_truncation() {
        let root = TestRoot::new("foreign-orphan");
        let expected = prepared(1, 1);
        let foreign = prepared(1, 0);
        create_empty_files(root.path());
        let row_path = root.path().join(ROW_FILE);
        let foreign_raw = foreign.rows[0].encode().expect("encode foreign orphan");
        std::fs::write(&row_path, foreign_raw).expect("write foreign orphan");
        let before = std::fs::read(&row_path).expect("read foreign orphan before retry");
        let mut writer = PopulationV5Ledger::open_write(root.path(), bounds())
            .expect("foreign-orphan writer opens structurally");
        assert!(
            writer
                .append(&expected)
                .expect_err("foreign orphan must refuse")
                .contains("not an exact canonical prefix")
        );
        drop(writer);
        assert_eq!(
            std::fs::read(&row_path).expect("read foreign orphan after refusal"),
            before,
            "valid foreign orphan bytes must never be truncated"
        );

        let torn_root = TestRoot::new("orphan-completion");
        create_empty_files(torn_root.path());
        let completion = expected
            .expected_completion(0, 0)
            .expect("derive orphan Completion")
            .encode()
            .expect("encode orphan Completion");
        std::fs::write(torn_root.path().join(COMPLETION_FILE), completion)
            .expect("write Completion without rows");
        let Err(torn_refusal) = PopulationV5Ledger::open_read(torn_root.path(), bounds()) else {
            panic!("Completion without rows must refuse");
        };
        assert!(torn_refusal.contains("torn"));
    }

    #[test]
    fn reordered_duplicate_cross_join_and_status_changes_refuse() {
        let mut reordered = prepared(1, 1);
        reordered.rows.swap(0, 1);
        assert!(reordered.validate(bounds()).is_err());

        let mut duplicate = prepared(1, 1);
        duplicate.rows[1].row_id = duplicate.rows[0].row_id;
        assert!(duplicate.validate(bounds()).is_err());

        let mut status = prepared(1, 1);
        status.admitted_count = 1;
        status.rejected_count = 1;
        assert!(status.validate(bounds()).is_err());

        let mut cross_join = prepared(1, 1);
        cross_join.rows[1].finalization.candidate_semantic_id =
            cross_join.rows[0].finalization.candidate_semantic_id;
        cross_join.rows[1].finalization.finalization_row_id = cross_join.rows[1]
            .finalization
            .derive_v3_row_id(
                cross_join.rows[1].global_sequence,
                cross_join.rows[1].family,
                cross_join.rows[1].family_sequence,
                cross_join.rows[1].status,
            )
            .expect("rederive deliberately crosswired Finalization row");
        cross_join.rows[1].row_id = cross_join.rows[1]
            .derive_row_id()
            .expect("rederive deliberately crosswired V5 row");
        assert!(cross_join.validate(bounds()).is_err());
    }

    #[test]
    fn same_length_corruption_ragged_files_and_cached_generation_changes_refuse() {
        let root = TestRoot::new("corrupt-stale");
        let prepared = prepared(1, 1);
        let receipt = {
            let mut writer = PopulationV5Ledger::open_write(root.path(), bounds())
                .expect("corruption fixture writer opens");
            writer
                .append(&prepared)
                .expect("corruption fixture writes")
                .receipt()
        };
        let mut cached = PopulationV5Ledger::open_read(root.path(), bounds())
            .expect("cached corruption fixture opens");
        let row_path = root.path().join(ROW_FILE);
        let mut changed = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&row_path)
            .expect("external row writer opens");
        let unchanged_length = changed
            .metadata()
            .expect("stat row file before same-length mutation")
            .len();
        let second_row_last_byte = (2 * POPULATION_V5_ROW_BYTES - 1) as u64;
        changed
            .seek(SeekFrom::Start(second_row_last_byte))
            .expect("seek later-row same-length mutation");
        let mut byte = [0_u8; 1];
        changed.read_exact(&mut byte).expect("read mutation byte");
        byte[0] ^= 1;
        changed
            .seek(SeekFrom::Start(second_row_last_byte))
            .and_then(|_| changed.write_all(&byte))
            .and_then(|()| changed.sync_data())
            .expect("persist later-row same-length mutation");
        assert_eq!(
            changed
                .metadata()
                .expect("stat row file after same-length mutation")
                .len(),
            unchanged_length,
            "later-row attack must preserve physical file length"
        );
        drop(changed);
        let Err(bulk_refusal) = cached.authenticated_rows(&receipt) else {
            panic!("bulk authentication returned rows after later-row corruption");
        };
        assert_eq!(
            bulk_refusal,
            "Population V5 retained root or file generation changed"
        );
        let Err(fresh_refusal) = PopulationV5Ledger::open_read(root.path(), bounds()) else {
            panic!("fresh reader must reject later-row seal corruption");
        };
        assert_eq!(fresh_refusal, "Population V5 row seal mismatch");

        let ragged = TestRoot::new("ragged");
        create_empty_files(ragged.path());
        std::fs::write(ragged.path().join(ROW_FILE), [1_u8]).expect("write ragged row file");
        let Err(ragged_refusal) = PopulationV5Ledger::open_read(ragged.path(), bounds()) else {
            panic!("ragged row file must refuse");
        };
        assert!(ragged_refusal.contains("ragged"));
    }

    #[cfg(unix)]
    #[test]
    fn replaced_child_paths_and_root_refuse_retained_authority() {
        for name in [LOCK_FILE, ROW_FILE, COMPLETION_FILE] {
            let root = TestRoot::new(name);
            let prepared = prepared(1, 1);
            let receipt = {
                let mut writer = PopulationV5Ledger::open_write(root.path(), bounds())
                    .expect("replacement fixture writer opens");
                writer
                    .append(&prepared)
                    .expect("replacement fixture writes")
                    .receipt()
            };
            let mut cached = PopulationV5Ledger::open_read(root.path(), bounds())
                .expect("replacement fixture reader opens");
            let named = root.path().join(name);
            let bytes = std::fs::read(&named).expect("read replacement bytes");
            let displaced = root.path().join(format!("{name}.displaced"));
            std::fs::rename(&named, &displaced).expect("displace held child inode");
            std::fs::write(&named, bytes).expect("replace child with same exact bytes");
            assert!(cached.authenticated_row(&receipt, 0).is_err());
        }

        for name in [LOCK_FILE, ROW_FILE, COMPLETION_FILE] {
            let root = TestRoot::new(&format!("hardlink-{name}"));
            let prepared = prepared(1, 1);
            let receipt = {
                let mut writer = PopulationV5Ledger::open_write(root.path(), bounds())
                    .expect("hard-link fixture writer opens");
                writer
                    .append(&prepared)
                    .expect("hard-link fixture writes")
                    .receipt()
            };
            let mut cached = PopulationV5Ledger::open_read(root.path(), bounds())
                .expect("hard-link fixture reader opens");
            let named = root.path().join(name);
            let alias = root.path().join(format!("{name}.alias"));
            std::fs::hard_link(&named, &alias).expect("add post-open hard link");
            assert!(
                cached
                    .authenticated_row(&receipt, 0)
                    .expect_err("post-open hard link must invalidate authority")
                    .contains("hard links")
            );
        }

        let root = TestRoot::new("root-replacement");
        let prepared = prepared(1, 0);
        let receipt = {
            let mut writer = PopulationV5Ledger::open_write(root.path(), bounds())
                .expect("root fixture writer opens");
            writer
                .append(&prepared)
                .expect("root fixture writes")
                .receipt()
        };
        let mut cached = PopulationV5Ledger::open_read(root.path(), bounds())
            .expect("root fixture reader opens");
        let displaced = root.path().with_extension("displaced");
        std::fs::rename(root.path(), &displaced).expect("displace held root");
        std::fs::create_dir(root.path()).expect("replace named root");
        assert!(cached.authenticated_row(&receipt, 0).is_err());
        drop(cached);
        std::fs::remove_dir(root.path()).expect("remove replacement root");
        std::fs::rename(&displaced, root.path()).expect("restore test root for cleanup");
    }
}
