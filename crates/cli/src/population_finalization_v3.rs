//! Receipt-last Population Finalization V3 authority.
//!
//! V3 is a new append-only byte family. It does not decode, rewrite or assign
//! Admission V3 meaning to Population Finalization V2 bytes. One fixed row is
//! derived from each authenticated, fixed-offset Population Admission V3
//! projection. Those rows preserve the complete Candidate, Statistics,
//! Search V4, Base-Evidence and Admission V3 lineage available at that seam.
//! A separate fixed Completion is appended and synced last.
//!
//! Publicly inspectable reopen state is structural only. Authenticated
//! authority is crate-private and is minted only after an opaque preparation
//! derived from [`PopulationAdmissionV3Authority`] is appended (or exactly
//! reused), the writer is dropped, and a fresh read-only reopen reproduces the
//! exact rows and Completion. No API in this module accepts a caller-authored
//! lineage digest, raw Runner decision, or detached structural receipt as
//! authority.
//!
//! This module deliberately does not invent a final Population V4 row or
//! receipt. Admission V3 currently exposes the exact Candidate-row digest and
//! final verdict but not authenticated Candidate row bytes or a Population V4
//! construction capability. That later promotion requires an additional
//! opaque upstream API.
//!
//! Opening, append, reuse and authenticated lookup hash bounded files and are
//! not O(1). Preparation uses Admission V3's opaque ordered bulk projection:
//! one shared lock, one generation validation before and after, and one
//! canonical fixed-record pass. It is O(Admission-file bytes + Candidates),
//! without accepting caller-authored rows.
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

#![expect(
    dead_code,
    reason = "the append-only successor exposes authenticated projection fields reserved for the downstream integration still blocked on Population V4"
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

use crate::population_admission_v3::{
    AdmissionV3Family, AdmissionV3Status, PopulationAdmissionV3Authority,
    PopulationAdmissionV3DecisionProjection,
};

/// Bytes in one canonical Population Finalization V3 row.
pub(crate) const POPULATION_FINALIZATION_V3_ROW_BYTES: usize = 2_048;
/// Bytes in one receipt-last Population Finalization V3 Completion.
pub(crate) const POPULATION_FINALIZATION_V3_COMPLETION_BYTES: usize = 4_096;

/// Operator-facing refusal at the Population Finalization V3 boundary.
pub(crate) type PopulationFinalizationV3Refusal = String;

const VERSION: u32 = 3;
const ROW_MAGIC: [u8; 16] = *b"BTX-PFNV3-ROW\0\0\0";
const COMPLETION_MAGIC: [u8; 16] = *b"BTX-PFNV3-CMP\0\0\0";
const ROW_DOMAIN: u32 = 1;
const COMPLETION_DOMAIN: u32 = 2;
const SEAL_BYTES: usize = 32;
const ROW_PAYLOAD_BYTES: usize = POPULATION_FINALIZATION_V3_ROW_BYTES - SEAL_BYTES;
const COMPLETION_PAYLOAD_BYTES: usize = POPULATION_FINALIZATION_V3_COMPLETION_BYTES - SEAL_BYTES;
const ROW_IDENTITY_BYTES: usize = 1_056;

const ROW_ID_DOMAIN: &[u8] = b"brutex-population-finalization-v3-row-id\0";
const FINALIZATION_ID_DOMAIN: &[u8] = b"brutex-population-finalization-v3-id\0";
const ORDERED_ROWS_DOMAIN: &[u8] = b"brutex-population-finalization-v3-ordered-rows\0";
const COMPLETION_ID_DOMAIN: &[u8] = b"brutex-population-finalization-v3-completion-id\0";
const ROW_SEAL_DOMAIN: &[u8] = b"brutex-population-finalization-v3-row-seal\0";
const COMPLETION_SEAL_DOMAIN: &[u8] = b"brutex-population-finalization-v3-completion-seal\0";
const FILE_GENERATION_DOMAIN: &[u8] = b"brutex-population-finalization-v3-generation\0";

const ROW_FILE: &str = "population-finalization-v3.bin";
const COMPLETION_FILE: &str = "population-finalization-completions-v3.bin";
const LOCK_FILE: &str = "population-finalization-v3.lock";
const LOCK_MAX_BYTES: u64 = 0;
const READ_CHUNK_BYTES: usize = 16 * 1_024;

#[cfg(any(target_os = "android", target_os = "linux"))]
const O_NOFOLLOW_FLAG: i32 = 0x20_000;
#[cfg(target_os = "macos")]
const O_NOFOLLOW_FLAG: i32 = 0x100;

const _: () = assert!(ROW_PAYLOAD_BYTES + SEAL_BYTES == POPULATION_FINALIZATION_V3_ROW_BYTES);
const _: () =
    assert!(COMPLETION_PAYLOAD_BYTES + SEAL_BYTES == POPULATION_FINALIZATION_V3_COMPLETION_BYTES);

/// Explicit physical ceilings for Population Finalization V3.
///
/// There is no default. Excess input is refused before row-family allocation;
/// it is never sampled, truncated or silently reduced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationFinalizationV3Bounds {
    row_records: u64,
    row_bytes: u64,
    completion_records: u64,
    completion_bytes: u64,
    rows_per_block: u64,
}

impl PopulationFinalizationV3Bounds {
    /// Constructs explicit nonzero ceilings.
    ///
    /// # Errors
    ///
    /// Refuses zero, arithmetic overflow, a per-block bound above the total
    /// row bound, or byte bounds unable to hold their declared fixed records.
    pub(crate) fn new(
        max_row_records: u64,
        max_row_bytes: u64,
        max_completion_records: u64,
        max_completion_bytes: u64,
        max_rows_per_block: u64,
    ) -> Result<Self, PopulationFinalizationV3Refusal> {
        for (name, value) in [
            ("row records", max_row_records),
            ("row bytes", max_row_bytes),
            ("Completion records", max_completion_records),
            ("Completion bytes", max_completion_bytes),
            ("rows per block", max_rows_per_block),
        ] {
            if value == 0 {
                return Err(format!("Finalization V3 maximum {name} must be nonzero"));
            }
        }
        if max_rows_per_block > max_row_records {
            return Err(format!(
                "Finalization V3 per-block maximum {max_rows_per_block} exceeds total row maximum {max_row_records}"
            ));
        }
        let required_row_bytes = max_row_records
            .checked_mul(POPULATION_FINALIZATION_V3_ROW_BYTES as u64)
            .ok_or_else(|| "Finalization V3 row byte ceiling overflowed".to_owned())?;
        if max_row_bytes < required_row_bytes {
            return Err(format!(
                "Finalization V3 row byte maximum {max_row_bytes} cannot hold {max_row_records} records ({required_row_bytes} bytes)"
            ));
        }
        let required_completion_bytes = max_completion_records
            .checked_mul(POPULATION_FINALIZATION_V3_COMPLETION_BYTES as u64)
            .ok_or_else(|| "Finalization V3 Completion byte ceiling overflowed".to_owned())?;
        if max_completion_bytes < required_completion_bytes {
            return Err(format!(
                "Finalization V3 Completion byte maximum {max_completion_bytes} cannot hold {max_completion_records} records ({required_completion_bytes} bytes)"
            ));
        }
        Ok(Self {
            row_records: max_row_records,
            row_bytes: max_row_bytes,
            completion_records: max_completion_records,
            completion_bytes: max_completion_bytes,
            rows_per_block: max_rows_per_block,
        })
    }

    #[must_use]
    pub(crate) const fn max_row_records(self) -> u64 {
        self.row_records
    }

    #[must_use]
    pub(crate) const fn max_row_bytes(self) -> u64 {
        self.row_bytes
    }

    #[must_use]
    pub(crate) const fn max_completion_records(self) -> u64 {
        self.completion_records
    }

    #[must_use]
    pub(crate) const fn max_completion_bytes(self) -> u64 {
        self.completion_bytes
    }

    #[must_use]
    pub(crate) const fn max_rows_per_block(self) -> u64 {
        self.rows_per_block
    }
}

/// Authenticated fixed-offset Finalization V3 projection for one Candidate.
///
/// Fields are private and there is no constructor. The type is also the exact
/// semantic row persisted by this module; fresh authority reads return it only
/// after revalidating retained file generations and canonical fixed offset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PopulationFinalizationV3RowProjection {
    finalization_id: [u8; 32],
    global_sequence: u64,
    family: AdmissionV3Family,
    family_sequence: u64,
    status: AdmissionV3Status,

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

    row_id: [u8; 32],
}

impl PopulationFinalizationV3RowProjection {
    fn from_admission(value: &PopulationAdmissionV3DecisionProjection) -> Self {
        Self {
            finalization_id: [0; 32],
            global_sequence: value.global_sequence(),
            family: value.family(),
            family_sequence: value.family_sequence(),
            status: value.status(),
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
            admission_block_id: value.block_id(),
            admission_completion_id: value.completion_id(),
            admission_runner_decision_digest: value.runner_decision_digest(),
            admission_evidence_digest: value.runner_evidence_digest(),
            admission_verdict_digest: value.runner_verdict_digest(),
            admission_decision_id: value.decision_id(),
            row_id: [0; 32],
        }
    }

    fn validate(&self) -> Result<(), PopulationFinalizationV3Refusal> {
        require_nonzero("Finalization identity", self.finalization_id)?;
        for (name, value) in [
            ("Candidate universe", self.candidate_universe_id),
            ("Candidate Completion", self.candidate_completion_digest),
            ("Candidate semantic", self.candidate_semantic_id),
            ("Candidate row", self.candidate_row_digest),
            ("Pre-Admission authority", self.pre_admission_authority_id),
            ("Statistics audit", self.statistics_audit_id),
            ("Statistics Completion", self.statistics_completion_digest),
            ("Statistics period", self.statistics_period_digest),
            ("Statistics split", self.statistics_split_digest),
            ("Search V4 pair", self.search_pair_id),
            ("Search V4 member", self.search_member_id),
            ("Search V4 signal", self.search_signal_digest),
            ("Search V4 signal column", self.search_signal_column_digest),
            ("Search V4 policy", self.search_policy_id),
            ("Search V4 full grid", self.search_full_grid_id),
            ("Search V4 Long policy", self.search_long_policy_id),
            ("Search V4 Long resolution", self.search_long_resolution_id),
            ("Search V4 Short policy", self.search_short_policy_id),
            (
                "Search V4 Short resolution",
                self.search_short_resolution_id,
            ),
            ("Search V4 family", self.search_family_id),
            ("Search V4 walk", self.search_walk_id),
            ("paired Base-Evidence", self.paired_base_id),
            ("Base-Evidence Completion", self.base_completion_id),
            ("Base-Evidence row", self.base_evidence_id),
            ("Admission V3 block", self.admission_block_id),
            ("Admission V3 Completion", self.admission_completion_id),
            (
                "Admission V3 Runner decision",
                self.admission_runner_decision_digest,
            ),
            ("Admission V3 evidence", self.admission_evidence_digest),
            ("Admission V3 verdict", self.admission_verdict_digest),
            ("Admission V3 decision", self.admission_decision_id),
            ("Finalization row", self.row_id),
        ] {
            require_nonzero(name, value)?;
        }
        if self.search_signal_bars == 0
            || self.search_signal_first_ts_micros > self.search_signal_last_ts_micros
        {
            return Err("Finalization V3 Search V4 signal shape is invalid".to_owned());
        }
        if self.search_long_policy_id == self.search_short_policy_id
            || self.search_long_resolution_id == self.search_short_resolution_id
        {
            return Err("Finalization V3 Search V4 Long/Short components alias".to_owned());
        }
        if self.search_fold_count == 0
            || self.search_decided_folds > self.search_fold_count
            || self.search_profitable_oos_folds > self.search_decided_folds
        {
            return Err("Finalization V3 Search V4 fold hierarchy is invalid".to_owned());
        }
        if (self.search_decided_folds == 0 && self.search_aggregate_oos_paisa != 0)
            || (self.search_profitable_oos_folds == 0 && self.search_aggregate_oos_paisa > 0)
            || (self.search_profitable_oos_folds == self.search_decided_folds
                && self.search_decided_folds > 0
                && self.search_aggregate_oos_paisa <= 0)
        {
            return Err("Finalization V3 Search V4 outcome hierarchy is invalid".to_owned());
        }
        if self.row_id != self.derive_row_id()? {
            return Err("Finalization V3 row identity does not reproduce".to_owned());
        }
        Ok(())
    }

    fn encode_identity_body(
        &self,
        writer: &mut FixedWriter<'_>,
    ) -> Result<(), PopulationFinalizationV3Refusal> {
        writer.u64(self.global_sequence)?;
        writer.u8(self.family as u8)?;
        writer.zeros(7)?;
        writer.u64(self.family_sequence)?;
        writer.u8(self.status as u8)?;
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
        Ok(())
    }

    fn derive_row_id(&self) -> Result<[u8; 32], PopulationFinalizationV3Refusal> {
        let mut identity = [0_u8; ROW_IDENTITY_BYTES];
        let mut writer = FixedWriter::new(&mut identity);
        self.encode_identity_body(&mut writer)?;
        writer.require_full("Finalization V3 row identity")?;
        Ok(hash_slices(
            ROW_ID_DOMAIN,
            &[&VERSION.to_le_bytes(), &identity],
        ))
    }

    fn decode_identity_body(
        finalization_id: [u8; 32],
        reader: &mut FixedReader<'_>,
    ) -> Result<Self, PopulationFinalizationV3Refusal> {
        let global_sequence = reader.u64()?;
        let family = decode_family(reader.u8()?)?;
        reader.require_zeros(7, "Finalization V3 row family reserve")?;
        let family_sequence = reader.u64()?;
        let status = decode_status(reader.u8()?)?;
        reader.require_zeros(7, "Finalization V3 row status reserve")?;
        Ok(Self {
            finalization_id,
            global_sequence,
            family,
            family_sequence,
            status,
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
            row_id: reader.array()?,
        })
    }

    #[must_use]
    pub(crate) const fn finalization_id(&self) -> [u8; 32] {
        self.finalization_id
    }

    #[must_use]
    pub(crate) const fn global_sequence(&self) -> u64 {
        self.global_sequence
    }

    #[must_use]
    pub(crate) const fn family(&self) -> AdmissionV3Family {
        self.family
    }

    #[must_use]
    pub(crate) const fn family_sequence(&self) -> u64 {
        self.family_sequence
    }

    #[must_use]
    pub(crate) const fn status(&self) -> AdmissionV3Status {
        self.status
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
    pub(crate) const fn row_id(&self) -> [u8; 32] {
        self.row_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PopulationFinalizationCompletionV3 {
    block_sequence: u64,
    first_row_record: u64,
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

impl PopulationFinalizationCompletionV3 {
    fn validate(&self) -> Result<(), PopulationFinalizationV3Refusal> {
        for (name, value) in [
            ("Finalization identity", self.finalization_id),
            ("Admission V3 block", self.admission_block_id),
            ("Admission V3 Completion", self.admission_completion_id),
            ("ordered Finalization rows", self.ordered_row_digest),
            ("Finalization Completion", self.completion_id),
        ] {
            require_nonzero(name, value)?;
        }
        let paired = self
            .nifty_row_count
            .checked_add(self.banknifty_row_count)
            .ok_or_else(|| "Finalization V3 paired row count overflowed".to_owned())?;
        let classified = checked_sum(
            &[
                self.admitted_count,
                self.rejected_count,
                self.unmeasured_count,
                self.refused_count,
            ],
            "Finalization V3 status counts",
        )?;
        if paired != self.row_count || classified != self.row_count {
            return Err("Finalization V3 Completion counts do not cover every row".to_owned());
        }
        if self.completion_id != self.derive_completion_id() {
            return Err("Finalization V3 Completion identity does not reproduce".to_owned());
        }
        Ok(())
    }

    fn derive_completion_id(&self) -> [u8; 32] {
        hash_slices(
            COMPLETION_ID_DOMAIN,
            &[
                &VERSION.to_le_bytes(),
                &self.finalization_id,
                &self.admission_block_id,
                &self.admission_completion_id,
                &self.row_count.to_le_bytes(),
                &self.nifty_row_count.to_le_bytes(),
                &self.banknifty_row_count.to_le_bytes(),
                &self.admitted_count.to_le_bytes(),
                &self.rejected_count.to_le_bytes(),
                &self.unmeasured_count.to_le_bytes(),
                &self.refused_count.to_le_bytes(),
                &self.ordered_row_digest,
            ],
        )
    }
}

/// Opaque authenticated-input preparation.
///
/// Fields are private. The only production constructor consumes the retained
/// nonconstructible Admission V3 authority one fixed ordinal at a time.
pub(crate) struct PreparedPopulationFinalizationV3 {
    finalization_id: [u8; 32],
    admission_block_id: [u8; 32],
    admission_completion_id: [u8; 32],
    nifty_row_count: u64,
    banknifty_row_count: u64,
    rows: Vec<PopulationFinalizationV3RowProjection>,
}

impl PreparedPopulationFinalizationV3 {
    fn row_count(&self) -> Result<u64, PopulationFinalizationV3Refusal> {
        u64::try_from(self.rows.len())
            .map_err(|_| "Finalization V3 prepared row count does not fit u64".to_owned())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the opaque block validator spells out every per-row uniqueness, family and global authority join"
    )]
    fn validate(&self) -> Result<(), PopulationFinalizationV3Refusal> {
        for (name, value) in [
            ("Finalization identity", self.finalization_id),
            ("Admission V3 block", self.admission_block_id),
            ("Admission V3 Completion", self.admission_completion_id),
        ] {
            require_nonzero(name, value)?;
        }
        let count = self.row_count()?;
        let expected = self
            .nifty_row_count
            .checked_add(self.banknifty_row_count)
            .ok_or_else(|| "Finalization V3 prepared family count overflowed".to_owned())?;
        if count != expected {
            return Err(format!(
                "Finalization V3 prepared block has {count} rows, expected {expected}"
            ));
        }
        let capacity = self.rows.len();
        let mut candidate_semantics = bounded_set(capacity, "Candidate semantic")?;
        let mut candidate_rows = bounded_set(capacity, "Candidate row")?;
        let mut statistics_periods = bounded_set(capacity, "Statistics period")?;
        let mut statistics_splits = bounded_set(capacity, "Statistics split")?;
        let mut base_evidence = bounded_set(capacity, "Base-Evidence row")?;
        let mut admission_decisions = bounded_set(capacity, "Admission decision")?;
        let mut finalization_rows = bounded_set(capacity, "Finalization row")?;
        let mut nifty_template = None;
        let mut banknifty_template = None;
        let mut global_template = None;
        for (index, row) in self.rows.iter().enumerate() {
            row.validate()?;
            let global = u64::try_from(index)
                .map_err(|_| "Finalization V3 row index does not fit u64".to_owned())?;
            let (expected_family, expected_family_sequence) = if global < self.nifty_row_count {
                (AdmissionV3Family::Nifty, global)
            } else {
                (
                    AdmissionV3Family::BankNifty,
                    global.checked_sub(self.nifty_row_count).ok_or_else(|| {
                        "Finalization V3 BANKNIFTY sequence underflowed".to_owned()
                    })?,
                )
            };
            if row.finalization_id != self.finalization_id
                || row.global_sequence != global
                || row.family != expected_family
                || row.family_sequence != expected_family_sequence
                || row.admission_block_id != self.admission_block_id
                || row.admission_completion_id != self.admission_completion_id
            {
                return Err(format!(
                    "Finalization V3 row {index} violates block or NIFTY-first ordering"
                ));
            }
            for (name, inserted) in [
                (
                    "Candidate semantic",
                    candidate_semantics.insert(row.candidate_semantic_id),
                ),
                (
                    "Candidate row",
                    candidate_rows.insert(row.candidate_row_digest),
                ),
                (
                    "Statistics period",
                    statistics_periods.insert(row.statistics_period_digest),
                ),
                (
                    "Statistics split",
                    statistics_splits.insert(row.statistics_split_digest),
                ),
                (
                    "Base-Evidence row",
                    base_evidence.insert(row.base_evidence_id),
                ),
                (
                    "Admission decision",
                    admission_decisions.insert(row.admission_decision_id),
                ),
                ("Finalization row", finalization_rows.insert(row.row_id)),
            ] {
                if !inserted {
                    return Err(format!("Finalization V3 duplicates {name} identity"));
                }
            }
            let global_facts = GlobalFactsV3::from(row);
            if let Some(template) = global_template {
                if template != global_facts {
                    return Err("Finalization V3 rows crosswire global authority".to_owned());
                }
            } else {
                global_template = Some(global_facts);
            }
            let family_facts = FamilyFactsV3::from(row);
            let template = match row.family {
                AdmissionV3Family::Nifty => &mut nifty_template,
                AdmissionV3Family::BankNifty => &mut banknifty_template,
            };
            if let Some(existing) = *template {
                if existing != family_facts {
                    return Err("Finalization V3 rows crosswire family authority".to_owned());
                }
            } else {
                *template = Some(family_facts);
            }
        }
        if derive_finalization_id(
            self.admission_block_id,
            self.admission_completion_id,
            self.nifty_row_count,
            self.banknifty_row_count,
            &self.rows,
        )? != self.finalization_id
        {
            return Err("Finalization V3 semantic identity does not reproduce".to_owned());
        }
        Ok(())
    }

    fn expected_completion(
        &self,
        block_sequence: u64,
        first_row_record: u64,
    ) -> Result<PopulationFinalizationCompletionV3, PopulationFinalizationV3Refusal> {
        self.validate()?;
        let mut admitted_count = 0_u64;
        let mut rejected_count = 0_u64;
        let mut unmeasured_count = 0_u64;
        let mut refused_count = 0_u64;
        for row in &self.rows {
            let target = match row.status {
                AdmissionV3Status::Admitted => &mut admitted_count,
                AdmissionV3Status::Rejected => &mut rejected_count,
                AdmissionV3Status::Unmeasured => &mut unmeasured_count,
                AdmissionV3Status::Refused => &mut refused_count,
            };
            *target = target
                .checked_add(1)
                .ok_or_else(|| "Finalization V3 status count overflowed".to_owned())?;
        }
        let mut value = PopulationFinalizationCompletionV3 {
            block_sequence,
            first_row_record,
            finalization_id: self.finalization_id,
            admission_block_id: self.admission_block_id,
            admission_completion_id: self.admission_completion_id,
            row_count: self.row_count()?,
            nifty_row_count: self.nifty_row_count,
            banknifty_row_count: self.banknifty_row_count,
            admitted_count,
            rejected_count,
            unmeasured_count,
            refused_count,
            ordered_row_digest: ordered_row_digest(&self.rows)?,
            completion_id: [0; 32],
        };
        value.completion_id = value.derive_completion_id();
        value.validate()?;
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GlobalFactsV3 {
    admission_block_id: [u8; 32],
    admission_completion_id: [u8; 32],
    statistics_audit_id: [u8; 32],
    statistics_completion_digest: [u8; 32],
    search_pair_id: [u8; 32],
    paired_base_id: [u8; 32],
}

impl From<&PopulationFinalizationV3RowProjection> for GlobalFactsV3 {
    fn from(value: &PopulationFinalizationV3RowProjection) -> Self {
        Self {
            admission_block_id: value.admission_block_id,
            admission_completion_id: value.admission_completion_id,
            statistics_audit_id: value.statistics_audit_id,
            statistics_completion_digest: value.statistics_completion_digest,
            search_pair_id: value.search_pair_id,
            paired_base_id: value.paired_base_id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FamilyFactsV3 {
    candidate_universe_id: [u8; 32],
    candidate_completion_digest: [u8; 32],
    pre_admission_authority_id: [u8; 32],
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
    base_completion_id: [u8; 32],
}

impl From<&PopulationFinalizationV3RowProjection> for FamilyFactsV3 {
    fn from(value: &PopulationFinalizationV3RowProjection) -> Self {
        Self {
            candidate_universe_id: value.candidate_universe_id,
            candidate_completion_digest: value.candidate_completion_digest,
            pre_admission_authority_id: value.pre_admission_authority_id,
            search_member_id: value.search_member_id,
            search_signal_digest: value.search_signal_digest,
            search_signal_bars: value.search_signal_bars,
            search_signal_first_ts_micros: value.search_signal_first_ts_micros,
            search_signal_last_ts_micros: value.search_signal_last_ts_micros,
            search_signal_column_digest: value.search_signal_column_digest,
            search_policy_id: value.search_policy_id,
            search_full_grid_id: value.search_full_grid_id,
            search_long_policy_id: value.search_long_policy_id,
            search_long_resolution_id: value.search_long_resolution_id,
            search_short_policy_id: value.search_short_policy_id,
            search_short_resolution_id: value.search_short_resolution_id,
            search_family_id: value.search_family_id,
            search_walk_id: value.search_walk_id,
            search_fold_count: value.search_fold_count,
            search_decided_folds: value.search_decided_folds,
            search_profitable_oos_folds: value.search_profitable_oos_folds,
            search_aggregate_oos_paisa: value.search_aggregate_oos_paisa,
            search_evaluated_population_count: value.search_evaluated_population_count,
            base_completion_id: value.base_completion_id,
        }
    }
}

/// Structural receipt from a bounded reopen. It is not source authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationFinalizationV3StructuralReceipt {
    finalization_id: [u8; 32],
    admission_block_id: [u8; 32],
    admission_completion_id: [u8; 32],
    block_sequence: u64,
    first_row_record: u64,
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

impl PopulationFinalizationV3StructuralReceipt {
    #[must_use]
    pub(crate) const fn finalization_id(self) -> [u8; 32] {
        self.finalization_id
    }

    #[must_use]
    pub(crate) const fn admission_block_id(self) -> [u8; 32] {
        self.admission_block_id
    }

    #[must_use]
    pub(crate) const fn admission_completion_id(self) -> [u8; 32] {
        self.admission_completion_id
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
    pub(crate) const fn nifty_row_count(self) -> u64 {
        self.nifty_row_count
    }

    #[must_use]
    pub(crate) const fn banknifty_row_count(self) -> u64 {
        self.banknifty_row_count
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

    #[must_use]
    pub(crate) const fn completion_id(self) -> [u8; 32] {
        self.completion_id
    }
}

/// Nonconstructible durable Finalization V3 authority.
///
/// It owns the freshly reopened ledger that authenticated the exact opaque
/// preparation. A detached structural receipt cannot construct this type.
pub(crate) struct PopulationFinalizationV3Authority {
    receipt: PopulationFinalizationV3StructuralReceipt,
    ledger: PopulationFinalizationV3Ledger,
}

impl PopulationFinalizationV3Authority {
    #[must_use]
    pub(crate) const fn structural_receipt(&self) -> PopulationFinalizationV3StructuralReceipt {
        self.receipt
    }

    /// Reads one canonical row from the retained authenticated ledger.
    ///
    /// # Errors
    ///
    /// Refuses an out-of-range ordinal or any stale, replaced, corrupt,
    /// reordered or crosswired retained source.
    pub(crate) fn row_projection(
        &mut self,
        global_sequence: u64,
    ) -> Result<PopulationFinalizationV3RowProjection, PopulationFinalizationV3Refusal> {
        self.ledger.authenticated_row(self.receipt, global_sequence)
    }

    /// Reads the complete canonical block authenticated by the retained receipt.
    ///
    /// The returned projections remain opaque: callers cannot construct or
    /// rewrite their identities. The ledger performs one bounded bulk read
    /// between one before/after generation-validation pair, rather than
    /// rehashing the complete files once per row.
    ///
    /// # Errors
    ///
    /// Refuses any stale, replaced, corrupt, reordered, crosswired,
    /// miscounted or receipt-divergent retained source. No prefix is returned
    /// when any row or the post-read generation validation fails.
    pub(crate) fn ordered_row_projections(
        &mut self,
    ) -> Result<Vec<PopulationFinalizationV3RowProjection>, PopulationFinalizationV3Refusal> {
        self.ledger.ordered_row_projections(self.receipt)
    }
}

/// Exact fresh-reopen persistence outcome.
pub(crate) enum PopulationFinalizationV3Commit {
    Written(PopulationFinalizationV3Authority),
    Reused(PopulationFinalizationV3Authority),
}

impl PopulationFinalizationV3Commit {
    #[must_use]
    pub(crate) fn into_authority(self) -> PopulationFinalizationV3Authority {
        match self {
            Self::Written(value) | Self::Reused(value) => value,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PopulationFinalizationV3StructuralCommit {
    Written(PopulationFinalizationV3StructuralReceipt),
    Reused(PopulationFinalizationV3StructuralReceipt),
}

impl PopulationFinalizationV3StructuralCommit {
    const fn receipt(self) -> PopulationFinalizationV3StructuralReceipt {
        match self {
            Self::Written(value) | Self::Reused(value) => value,
        }
    }
}

fn prepare_population_finalization_v3(
    admission: &mut PopulationAdmissionV3Authority,
    bounds: PopulationFinalizationV3Bounds,
) -> Result<PreparedPopulationFinalizationV3, PopulationFinalizationV3Refusal> {
    let receipt = admission.structural_receipt();
    let count = receipt.decision_count();
    require_block_bound(bounds, count)?;
    let capacity = usize::try_from(count)
        .map_err(|_| "Finalization V3 Admission count does not fit usize".to_owned())?;
    let projections = admission
        .ordered_decision_projections()
        .map_err(|why| format!("Finalization V3 Admission bulk projection refused: {why}"))?;
    if projections.len() != capacity {
        return Err(format!(
            "Finalization V3 Admission bulk projection has {} rows, expected {capacity}",
            projections.len()
        ));
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(capacity)
        .map_err(|why| format!("cannot reserve Finalization V3 row family: {why}"))?;
    for projected in &projections {
        let mut row = PopulationFinalizationV3RowProjection::from_admission(projected);
        if row.admission_block_id != receipt.block_id()
            || row.admission_completion_id != receipt.completion_id()
        {
            return Err(
                "Finalization V3 Admission projection crosswires its retained receipt".to_owned(),
            );
        }
        row.row_id = row.derive_row_id()?;
        rows.push(row);
    }
    let finalization_id = derive_finalization_id(
        receipt.block_id(),
        receipt.completion_id(),
        receipt.nifty_decision_count(),
        receipt.banknifty_decision_count(),
        &rows,
    )?;
    for row in &mut rows {
        row.finalization_id = finalization_id;
    }
    let prepared = PreparedPopulationFinalizationV3 {
        finalization_id,
        admission_block_id: receipt.block_id(),
        admission_completion_id: receipt.completion_id(),
        nifty_row_count: receipt.nifty_decision_count(),
        banknifty_row_count: receipt.banknifty_decision_count(),
        rows,
    };
    prepared.validate()?;
    Ok(prepared)
}

/// Finalizes one retained Population Admission V3 authority.
///
/// This is the only production preparation/persistence door. It accepts no raw
/// rows or lineage digests. Persistence is receipt-last; success follows writer
/// drop, fresh read-only reopen, exact structural receipt comparison and exact
/// row/Completion authentication against the opaque preparation.
pub(crate) fn commit_population_finalization_v3(
    root: &Path,
    bounds: PopulationFinalizationV3Bounds,
    admission: &mut PopulationAdmissionV3Authority,
) -> Result<PopulationFinalizationV3Commit, PopulationFinalizationV3Refusal> {
    let prepared = prepare_population_finalization_v3(admission, bounds)?;
    persist_population_finalization_v3(root, bounds, &prepared)
}

fn derive_finalization_id(
    admission_block_id: [u8; 32],
    admission_completion_id: [u8; 32],
    nifty_row_count: u64,
    banknifty_row_count: u64,
    rows: &[PopulationFinalizationV3RowProjection],
) -> Result<[u8; 32], PopulationFinalizationV3Refusal> {
    let count = u64::try_from(rows.len())
        .map_err(|_| "Finalization V3 identity row count does not fit u64".to_owned())?;
    let mut hasher = Hasher::new();
    hasher.update(FINALIZATION_ID_DOMAIN);
    hasher.update(&VERSION.to_le_bytes());
    hasher.update(&admission_block_id);
    hasher.update(&admission_completion_id);
    hasher.update(&count.to_le_bytes());
    hasher.update(&nifty_row_count.to_le_bytes());
    hasher.update(&banknifty_row_count.to_le_bytes());
    for row in rows {
        hasher.update(&row.row_id);
    }
    Ok(hasher.finalize())
}

fn ordered_row_digest(
    rows: &[PopulationFinalizationV3RowProjection],
) -> Result<[u8; 32], PopulationFinalizationV3Refusal> {
    let count = u64::try_from(rows.len())
        .map_err(|_| "Finalization V3 ordered row count does not fit u64".to_owned())?;
    let mut hasher = Hasher::new();
    hasher.update(ORDERED_ROWS_DOMAIN);
    hasher.update(&count.to_le_bytes());
    for row in rows {
        hasher.update(&row.row_id);
    }
    Ok(hasher.finalize())
}

fn require_block_bound(
    bounds: PopulationFinalizationV3Bounds,
    count: u64,
) -> Result<(), PopulationFinalizationV3Refusal> {
    if count > bounds.max_rows_per_block() {
        return Err(format!(
            "Finalization V3 block has {count} rows, above per-block maximum {}",
            bounds.max_rows_per_block()
        ));
    }
    let bytes = count
        .checked_mul(POPULATION_FINALIZATION_V3_ROW_BYTES as u64)
        .ok_or_else(|| "Finalization V3 block row bytes overflowed".to_owned())?;
    if bytes > bounds.max_row_bytes() {
        return Err(format!(
            "Finalization V3 block requires {bytes} bytes, above maximum {}",
            bounds.max_row_bytes()
        ));
    }
    Ok(())
}

fn encode_row(
    row: &PopulationFinalizationV3RowProjection,
    physical_sequence: u64,
) -> Result<[u8; POPULATION_FINALIZATION_V3_ROW_BYTES], PopulationFinalizationV3Refusal> {
    row.validate()?;
    let mut raw = [0_u8; POPULATION_FINALIZATION_V3_ROW_BYTES];
    {
        let payload = raw
            .get_mut(..ROW_PAYLOAD_BYTES)
            .ok_or_else(|| "Finalization V3 row payload is absent".to_owned())?;
        let mut writer = FixedWriter::new(payload);
        writer.array(&ROW_MAGIC)?;
        writer.u32(VERSION)?;
        writer.u32(ROW_DOMAIN)?;
        writer.u64(physical_sequence)?;
        writer.array(&row.finalization_id)?;
        row.encode_identity_body(&mut writer)?;
        writer.array(&row.row_id)?;
        writer.zero_remaining();
    }
    let seal = hash_slices(ROW_SEAL_DOMAIN, &[fixed_payload(&raw, ROW_PAYLOAD_BYTES)?]);
    raw.get_mut(ROW_PAYLOAD_BYTES..)
        .ok_or_else(|| "Finalization V3 row seal slot is absent".to_owned())?
        .copy_from_slice(&seal);
    Ok(raw)
}

fn decode_row(
    raw: &[u8; POPULATION_FINALIZATION_V3_ROW_BYTES],
    expected_physical_sequence: u64,
) -> Result<PopulationFinalizationV3RowProjection, PopulationFinalizationV3Refusal> {
    let payload =
        validate_fixed_payload(raw, ROW_MAGIC, VERSION, ROW_DOMAIN, ROW_SEAL_DOMAIN, "row")?;
    let mut reader = FixedReader::new(payload);
    if reader.array::<16>()? != ROW_MAGIC || reader.u32()? != VERSION || reader.u32()? != ROW_DOMAIN
    {
        return Err("Finalization V3 row header/version/domain mismatch".to_owned());
    }
    let physical_sequence = reader.u64()?;
    if physical_sequence != expected_physical_sequence {
        return Err(format!(
            "Finalization V3 row physical sequence {physical_sequence} is not expected {expected_physical_sequence}"
        ));
    }
    let finalization_id = reader.array()?;
    let value =
        PopulationFinalizationV3RowProjection::decode_identity_body(finalization_id, &mut reader)?;
    reader.require_remaining_zero("Finalization V3 row reserve")?;
    value.validate()?;
    Ok(value)
}

fn encode_completion(
    value: &PopulationFinalizationCompletionV3,
) -> Result<[u8; POPULATION_FINALIZATION_V3_COMPLETION_BYTES], PopulationFinalizationV3Refusal> {
    value.validate()?;
    let mut raw = [0_u8; POPULATION_FINALIZATION_V3_COMPLETION_BYTES];
    {
        let payload = raw
            .get_mut(..COMPLETION_PAYLOAD_BYTES)
            .ok_or_else(|| "Finalization V3 Completion payload is absent".to_owned())?;
        let mut writer = FixedWriter::new(payload);
        writer.array(&COMPLETION_MAGIC)?;
        writer.u32(VERSION)?;
        writer.u32(COMPLETION_DOMAIN)?;
        writer.u64(value.block_sequence)?;
        writer.u64(value.first_row_record)?;
        for digest in [
            value.finalization_id,
            value.admission_block_id,
            value.admission_completion_id,
        ] {
            writer.array(&digest)?;
        }
        for count in [
            value.row_count,
            value.nifty_row_count,
            value.banknifty_row_count,
            value.admitted_count,
            value.rejected_count,
            value.unmeasured_count,
            value.refused_count,
        ] {
            writer.u64(count)?;
        }
        writer.array(&value.ordered_row_digest)?;
        writer.array(&value.completion_id)?;
        writer.zero_remaining();
    }
    let seal = hash_slices(
        COMPLETION_SEAL_DOMAIN,
        &[fixed_payload(&raw, COMPLETION_PAYLOAD_BYTES)?],
    );
    raw.get_mut(COMPLETION_PAYLOAD_BYTES..)
        .ok_or_else(|| "Finalization V3 Completion seal slot is absent".to_owned())?
        .copy_from_slice(&seal);
    Ok(raw)
}

fn decode_completion(
    raw: &[u8; POPULATION_FINALIZATION_V3_COMPLETION_BYTES],
) -> Result<PopulationFinalizationCompletionV3, PopulationFinalizationV3Refusal> {
    let payload = validate_fixed_payload(
        raw,
        COMPLETION_MAGIC,
        VERSION,
        COMPLETION_DOMAIN,
        COMPLETION_SEAL_DOMAIN,
        "Completion",
    )?;
    let mut reader = FixedReader::new(payload);
    if reader.array::<16>()? != COMPLETION_MAGIC
        || reader.u32()? != VERSION
        || reader.u32()? != COMPLETION_DOMAIN
    {
        return Err("Finalization V3 Completion header/version/domain mismatch".to_owned());
    }
    let value = PopulationFinalizationCompletionV3 {
        block_sequence: reader.u64()?,
        first_row_record: reader.u64()?,
        finalization_id: reader.array()?,
        admission_block_id: reader.array()?,
        admission_completion_id: reader.array()?,
        row_count: reader.u64()?,
        nifty_row_count: reader.u64()?,
        banknifty_row_count: reader.u64()?,
        admitted_count: reader.u64()?,
        rejected_count: reader.u64()?,
        unmeasured_count: reader.u64()?,
        refused_count: reader.u64()?,
        ordered_row_digest: reader.array()?,
        completion_id: reader.array()?,
    };
    reader.require_remaining_zero("Finalization V3 Completion reserve")?;
    value.validate()?;
    Ok(value)
}

fn validate_fixed_payload<'a, const N: usize>(
    raw: &'a [u8; N],
    magic: [u8; 16],
    version: u32,
    domain: u32,
    seal_domain: &[u8],
    name: &str,
) -> Result<&'a [u8], PopulationFinalizationV3Refusal> {
    let payload_len = N
        .checked_sub(SEAL_BYTES)
        .ok_or_else(|| format!("Finalization V3 {name} width is below seal width"))?;
    let payload = raw
        .get(..payload_len)
        .ok_or_else(|| format!("Finalization V3 {name} payload is absent"))?;
    let seal = raw
        .get(payload_len..)
        .ok_or_else(|| format!("Finalization V3 {name} seal is absent"))?;
    if seal != hash_slices(seal_domain, &[payload]) {
        return Err(format!("Finalization V3 {name} seal is invalid"));
    }
    if array_at::<16>(payload, 0, name)? != magic
        || u32_at(payload, 16, name)? != version
        || u32_at(payload, 20, name)? != domain
    {
        return Err(format!("Finalization V3 {name} header is invalid"));
    }
    Ok(payload)
}

fn fixed_payload<const N: usize>(
    raw: &[u8; N],
    payload_len: usize,
) -> Result<&[u8], PopulationFinalizationV3Refusal> {
    raw.get(..payload_len)
        .ok_or_else(|| "Finalization V3 fixed payload is absent".to_owned())
}

fn array_at<const N: usize>(
    bytes: &[u8],
    offset: usize,
    name: &str,
) -> Result<[u8; N], PopulationFinalizationV3Refusal> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| format!("Finalization V3 {name} offset overflowed"))?;
    bytes
        .get(offset..end)
        .ok_or_else(|| format!("Finalization V3 {name} is truncated"))?
        .try_into()
        .map_err(|_| format!("Finalization V3 {name} width is not {N}"))
}

fn u32_at(bytes: &[u8], offset: usize, name: &str) -> Result<u32, PopulationFinalizationV3Refusal> {
    Ok(u32::from_le_bytes(array_at(bytes, offset, name)?))
}

struct FixedWriter<'a> {
    bytes: &'a mut [u8],
    offset: usize,
}

impl<'a> FixedWriter<'a> {
    const fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn write(&mut self, value: &[u8]) -> Result<(), PopulationFinalizationV3Refusal> {
        let end = self
            .offset
            .checked_add(value.len())
            .ok_or_else(|| "Finalization V3 fixed writer offset overflowed".to_owned())?;
        self.bytes
            .get_mut(self.offset..end)
            .ok_or_else(|| "Finalization V3 fixed writer exceeded payload".to_owned())?
            .copy_from_slice(value);
        self.offset = end;
        Ok(())
    }

    fn array<const N: usize>(
        &mut self,
        value: &[u8; N],
    ) -> Result<(), PopulationFinalizationV3Refusal> {
        self.write(value)
    }

    fn u8(&mut self, value: u8) -> Result<(), PopulationFinalizationV3Refusal> {
        self.write(&[value])
    }

    fn u32(&mut self, value: u32) -> Result<(), PopulationFinalizationV3Refusal> {
        self.write(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), PopulationFinalizationV3Refusal> {
        self.write(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), PopulationFinalizationV3Refusal> {
        self.write(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), PopulationFinalizationV3Refusal> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| "Finalization V3 zero reserve offset overflowed".to_owned())?;
        self.bytes
            .get_mut(self.offset..end)
            .ok_or_else(|| "Finalization V3 zero reserve exceeded payload".to_owned())?
            .fill(0);
        self.offset = end;
        Ok(())
    }

    fn zero_remaining(&mut self) {
        if let Some(tail) = self.bytes.get_mut(self.offset..) {
            tail.fill(0);
            self.offset = self.bytes.len();
        }
    }

    fn require_full(&self, name: &str) -> Result<(), PopulationFinalizationV3Refusal> {
        if self.offset != self.bytes.len() {
            return Err(format!(
                "{name} encoded {} bytes, not fixed {}",
                self.offset,
                self.bytes.len()
            ));
        }
        Ok(())
    }
}

struct FixedReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> FixedReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take<const N: usize>(&mut self) -> Result<[u8; N], PopulationFinalizationV3Refusal> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or_else(|| "Finalization V3 fixed reader offset overflowed".to_owned())?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| "Finalization V3 fixed reader exceeded payload".to_owned())?
            .try_into()
            .map_err(|_| "Finalization V3 fixed reader width mismatch".to_owned())?;
        self.offset = end;
        Ok(value)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], PopulationFinalizationV3Refusal> {
        self.take()
    }

    fn u8(&mut self) -> Result<u8, PopulationFinalizationV3Refusal> {
        self.take::<1>()?
            .first()
            .copied()
            .ok_or_else(|| "Finalization V3 one-byte field is absent".to_owned())
    }

    fn u32(&mut self) -> Result<u32, PopulationFinalizationV3Refusal> {
        Ok(u32::from_le_bytes(self.take()?))
    }

    fn u64(&mut self) -> Result<u64, PopulationFinalizationV3Refusal> {
        Ok(u64::from_le_bytes(self.take()?))
    }

    fn i64(&mut self) -> Result<i64, PopulationFinalizationV3Refusal> {
        Ok(i64::from_le_bytes(self.take()?))
    }

    fn require_zeros(
        &mut self,
        count: usize,
        name: &str,
    ) -> Result<(), PopulationFinalizationV3Refusal> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| format!("{name} offset overflowed"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| format!("{name} is truncated"))?;
        if value.iter().any(|byte| *byte != 0) {
            return Err(format!("{name} is nonzero"));
        }
        self.offset = end;
        Ok(())
    }

    fn require_remaining_zero(&self, name: &str) -> Result<(), PopulationFinalizationV3Refusal> {
        if self
            .bytes
            .get(self.offset..)
            .ok_or_else(|| format!("{name} offset escaped payload"))?
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(format!("{name} is nonzero"));
        }
        Ok(())
    }
}

fn decode_family(value: u8) -> Result<AdmissionV3Family, PopulationFinalizationV3Refusal> {
    match value {
        1 => Ok(AdmissionV3Family::Nifty),
        2 => Ok(AdmissionV3Family::BankNifty),
        _ => Err(format!("Finalization V3 family tag {value} is unknown")),
    }
}

fn decode_status(value: u8) -> Result<AdmissionV3Status, PopulationFinalizationV3Refusal> {
    match value {
        1 => Ok(AdmissionV3Status::Admitted),
        2 => Ok(AdmissionV3Status::Rejected),
        3 => Ok(AdmissionV3Status::Unmeasured),
        4 => Ok(AdmissionV3Status::Refused),
        _ => Err(format!("Finalization V3 status tag {value} is unknown")),
    }
}

fn require_nonzero(name: &str, value: [u8; 32]) -> Result<(), PopulationFinalizationV3Refusal> {
    if value == [0; 32] {
        return Err(format!("Finalization V3 {name} identity is zero"));
    }
    Ok(())
}

fn checked_sum(values: &[u64], name: &str) -> Result<u64, PopulationFinalizationV3Refusal> {
    values.iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(*value)
            .ok_or_else(|| format!("{name} overflowed"))
    })
}

fn bounded_set<T>(
    capacity: usize,
    name: &str,
) -> Result<HashSet<T>, PopulationFinalizationV3Refusal>
where
    T: Eq + std::hash::Hash,
{
    let mut value = HashSet::new();
    value
        .try_reserve(capacity)
        .map_err(|why| format!("cannot reserve Finalization V3 {name} index: {why}"))?;
    Ok(value)
}

fn hash_slices(domain: &[u8], values: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    for value in values {
        hasher.update(value);
    }
    hasher.finalize()
}

fn hex32(value: [u8; 32]) -> String {
    let mut output = String::with_capacity(64);
    for byte in value {
        for nibble in [byte >> 4, byte & 0x0f] {
            output.push(char::from(if nibble < 10 {
                b'0' + nibble
            } else {
                b'a' + (nibble - 10)
            }));
        }
    }
    output
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformIdentity {
    device: u64,
    inode: u64,
}

#[cfg(unix)]
impl PlatformIdentity {
    fn of(metadata: &std::fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
        }
    }
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MetadataGeneration {
    identity: PlatformIdentity,
    len: u64,
    mode: u32,
    links: u64,
    owner: u32,
    group: u32,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(unix)]
impl MetadataGeneration {
    fn of(metadata: &std::fs::Metadata) -> Self {
        Self {
            identity: PlatformIdentity::of(metadata),
            len: metadata.len(),
            mode: metadata.mode(),
            links: metadata.nlink(),
            owner: metadata.uid(),
            group: metadata.gid(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        }
    }
}

#[cfg(not(unix))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformIdentity {
    len: u64,
}

#[cfg(not(unix))]
impl PlatformIdentity {
    fn of(metadata: &std::fs::Metadata) -> Self {
        Self {
            len: metadata.len(),
        }
    }
}

#[cfg(not(unix))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MetadataGeneration {
    identity: PlatformIdentity,
    len: u64,
    readonly: bool,
    modified: Option<std::time::SystemTime>,
}

#[cfg(not(unix))]
impl MetadataGeneration {
    fn of(metadata: &std::fs::Metadata) -> Self {
        Self {
            identity: PlatformIdentity::of(metadata),
            len: metadata.len(),
            readonly: metadata.permissions().readonly(),
            modified: metadata.modified().ok(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGeneration {
    identity: PlatformIdentity,
    len: u64,
    metadata: MetadataGeneration,
    digest: [u8; 32],
}

#[derive(Clone, PartialEq, Eq)]
struct TrailingRowsV3 {
    first_row_record: u64,
    finalization_id: [u8; 32],
    rows: Vec<PopulationFinalizationV3RowProjection>,
}

/// Bounded structural view over Population Finalization V3 files.
///
/// Opening is O(file bytes + rows) time and retains O(Completions + one
/// trailing block) memory. The receipt index probe is average O(1) only after
/// bounded file-generation validation, which is itself O(file bytes).
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
pub(crate) struct PopulationFinalizationV3Ledger {
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
    bounds: PopulationFinalizationV3Bounds,
    writable: bool,
    receipts: HashMap<[u8; 32], PopulationFinalizationV3StructuralReceipt>,
    trailing: Option<TrailingRowsV3>,
    row_records: u64,
    completion_records: u64,
}

impl PopulationFinalizationV3Ledger {
    /// Opens existing Finalization V3 files read-only without creating paths.
    ///
    /// # Errors
    ///
    /// Refuses missing, linked, non-regular, over-bound, ragged, corrupt,
    /// reordered, duplicated, torn, stale or path-replaced state.
    pub(crate) fn open_read(
        root: &Path,
        bounds: PopulationFinalizationV3Bounds,
    ) -> Result<Self, PopulationFinalizationV3Refusal> {
        Self::open(root, bounds, false)
    }

    fn open_write(
        root: &Path,
        bounds: PopulationFinalizationV3Bounds,
    ) -> Result<Self, PopulationFinalizationV3Refusal> {
        Self::open(root, bounds, true)
    }

    fn open(
        root: &Path,
        bounds: PopulationFinalizationV3Bounds,
        writable: bool,
    ) -> Result<Self, PopulationFinalizationV3Refusal> {
        let (root_path, root_file, root_identity) = open_root_directory(root)?;
        let lock_path = root_path.join(LOCK_FILE);
        let row_path = root_path.join(ROW_FILE);
        let completion_path = root_path.join(COMPLETION_FILE);
        let (lock_file, lock_created) = open_child(&lock_path, writable, writable)?;
        if writable {
            lock_file
                .lock()
                .map_err(|why| format!("cannot take exclusive Finalization V3 open lock: {why}"))?;
        } else {
            lock_file
                .lock_shared()
                .map_err(|why| format!("cannot take shared Finalization V3 open lock: {why}"))?;
        }
        let opened = (|| {
            let (row_file, row_created) = open_child(&row_path, writable, writable)?;
            let (completion_file, completion_created) =
                open_child(&completion_path, writable, writable)?;
            if lock_created || row_created || completion_created {
                sync_directory(&root_file, &root_path)?;
            }
            if named_root_identity(&root_path)? != root_identity {
                return Err("Finalization V3 root changed while child files opened".to_owned());
            }
            let lock_generation = file_generation(&lock_file, &lock_path, LOCK_MAX_BYTES)?;
            let row_generation = file_generation(&row_file, &row_path, bounds.max_row_bytes())?;
            let completion_generation = file_generation(
                &completion_file,
                &completion_path,
                bounds.max_completion_bytes(),
            )?;
            let mut ledger = Self {
                root: root_path,
                root_file,
                root_identity,
                lock_path,
                lock_file: lock_file
                    .try_clone()
                    .map_err(|why| format!("cannot clone Finalization V3 held lock file: {why}"))?,
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
            .map_err(|why| format!("cannot release Finalization V3 open lock: {why}"));
        match (opened, released) {
            (Ok(ledger), Ok(())) => Ok(ledger),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn scan(&mut self) -> Result<(), PopulationFinalizationV3Refusal> {
        let row_records = checked_record_count(
            self.row_generation.len,
            POPULATION_FINALIZATION_V3_ROW_BYTES,
            self.bounds.max_row_records(),
            "row",
        )?;
        let completion_records = checked_record_count(
            self.completion_generation.len,
            POPULATION_FINALIZATION_V3_COMPLETION_BYTES,
            self.bounds.max_completion_records(),
            "Completion",
        )?;
        self.receipts.clear();
        self.receipts
            .try_reserve(
                usize::try_from(completion_records).map_err(|_| {
                    "Finalization V3 Completion count does not fit usize".to_owned()
                })?,
            )
            .map_err(|why| format!("cannot reserve Finalization V3 receipt index: {why}"))?;
        self.trailing = None;
        let mut covered_rows = 0_u64;
        let mut completion_index = 0_u64;
        while completion_index < completion_records {
            let completion = decode_completion(&read_completion_at(
                &mut self.completion_file,
                completion_index,
            )?)?;
            if completion.block_sequence != completion_index {
                return Err(format!(
                    "Finalization V3 Completion sequence {} is not canonical {completion_index}",
                    completion.block_sequence
                ));
            }
            if completion.first_row_record != covered_rows {
                return Err(format!(
                    "Finalization V3 Completion {completion_index} starts at {}, not contiguous {covered_rows}",
                    completion.first_row_record
                ));
            }
            require_block_bound(self.bounds, completion.row_count)?;
            let end = covered_rows
                .checked_add(completion.row_count)
                .ok_or_else(|| "Finalization V3 completed row range overflowed".to_owned())?;
            if end > row_records {
                return Err(format!(
                    "Finalization V3 Completion {completion_index} is torn: ends at {end}, file has {row_records} rows"
                ));
            }
            let rows = read_row_range(
                &mut self.row_file,
                covered_rows,
                completion.row_count,
                self.bounds,
            )?;
            let receipt = validate_complete_block(&rows, &completion)?;
            if self
                .receipts
                .insert(receipt.finalization_id, receipt)
                .is_some()
            {
                return Err(format!(
                    "Finalization V3 identity {} appears more than once",
                    hex32(receipt.finalization_id)
                ));
            }
            covered_rows = end;
            completion_index = completion_index
                .checked_add(1)
                .ok_or_else(|| "Finalization V3 Completion scan overflowed".to_owned())?;
        }
        if covered_rows < row_records {
            let trailing_count = row_records
                .checked_sub(covered_rows)
                .ok_or_else(|| "Finalization V3 trailing row count underflowed".to_owned())?;
            require_block_bound(self.bounds, trailing_count)?;
            let rows = read_row_range(
                &mut self.row_file,
                covered_rows,
                trailing_count,
                self.bounds,
            )?;
            let finalization_id = validate_trailing_rows(&rows)?;
            if self.receipts.contains_key(&finalization_id) {
                return Err(format!(
                    "Finalization V3 trailing block {} duplicates a completed block",
                    hex32(finalization_id)
                ));
            }
            self.trailing = Some(TrailingRowsV3 {
                first_row_record: covered_rows,
                finalization_id,
                rows,
            });
        }
        self.row_records = row_records;
        self.completion_records = completion_records;
        self.require_unchanged()
    }

    /// Revalidates all bounded generations before average-O(1) receipt lookup.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    pub(crate) fn reopen_structural_receipt(
        &self,
        finalization_id: &[u8; 32],
    ) -> Result<Option<PopulationFinalizationV3StructuralReceipt>, PopulationFinalizationV3Refusal>
    {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take shared Finalization V3 lookup lock: {why}"))?;
        let result = self
            .require_unchanged()
            .map(|()| self.receipts.get(finalization_id).copied());
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release shared Finalization V3 lookup lock: {why}"));
        match (result, released) {
            (Ok(receipt), Ok(())) => Ok(receipt),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn authenticated_row(
        &mut self,
        receipt: PopulationFinalizationV3StructuralReceipt,
        global_sequence: u64,
    ) -> Result<PopulationFinalizationV3RowProjection, PopulationFinalizationV3Refusal> {
        if global_sequence >= receipt.row_count() {
            return Err(format!(
                "Finalization V3 row ordinal {global_sequence} is outside authenticated count {}",
                receipt.row_count()
            ));
        }
        let (expected_family, expected_family_sequence) =
            if global_sequence < receipt.nifty_row_count() {
                (AdmissionV3Family::Nifty, global_sequence)
            } else {
                (
                    AdmissionV3Family::BankNifty,
                    global_sequence
                        .checked_sub(receipt.nifty_row_count())
                        .ok_or_else(|| {
                            "Finalization V3 authenticated BANKNIFTY ordinal underflowed".to_owned()
                        })?,
                )
            };
        let physical = receipt
            .first_row_record()
            .checked_add(global_sequence)
            .ok_or_else(|| "Finalization V3 authenticated row offset overflowed".to_owned())?;
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take shared Finalization V3 row lock: {why}"))?;
        let result = (|| {
            self.require_unchanged()?;
            if self.receipts.get(&receipt.finalization_id()) != Some(&receipt) {
                return Err(
                    "Finalization V3 authenticated receipt is no longer indexed exactly".to_owned(),
                );
            }
            let raw = read_fixed_at::<POPULATION_FINALIZATION_V3_ROW_BYTES>(
                &mut self.row_file,
                physical,
                POPULATION_FINALIZATION_V3_ROW_BYTES,
                "authenticated row",
            )?;
            let row = decode_row(&raw, physical)?;
            if row.finalization_id != receipt.finalization_id()
                || row.admission_block_id != receipt.admission_block_id()
                || row.admission_completion_id != receipt.admission_completion_id()
                || row.global_sequence != global_sequence
                || row.family != expected_family
                || row.family_sequence != expected_family_sequence
            {
                return Err(
                    "Finalization V3 fixed-offset row differs from authenticated ordering"
                        .to_owned(),
                );
            }
            self.require_unchanged()?;
            Ok(row)
        })();
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release shared Finalization V3 row lock: {why}"));
        match (result, released) {
            (Ok(row), Ok(())) => Ok(row),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn ordered_row_projections(
        &mut self,
        receipt: PopulationFinalizationV3StructuralReceipt,
    ) -> Result<Vec<PopulationFinalizationV3RowProjection>, PopulationFinalizationV3Refusal> {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take shared Finalization V3 bulk-row lock: {why}"))?;
        let result = (|| {
            self.require_unchanged()?;
            let read = (|| {
                if self.receipts.get(&receipt.finalization_id()) != Some(&receipt) {
                    return Err(
                        "Finalization V3 bulk-row receipt is no longer indexed exactly".to_owned(),
                    );
                }
                if receipt.block_sequence() >= self.completion_records {
                    return Err(format!(
                        "Finalization V3 bulk-row Completion {} is outside retained count {}",
                        receipt.block_sequence(),
                        self.completion_records
                    ));
                }
                let end = receipt
                    .first_row_record()
                    .checked_add(receipt.row_count())
                    .ok_or_else(|| {
                        "Finalization V3 bulk-row retained range overflowed".to_owned()
                    })?;
                if end > self.row_records {
                    return Err(format!(
                        "Finalization V3 bulk-row range ends at {end}, above retained count {}",
                        self.row_records
                    ));
                }

                let completion = decode_completion(&read_completion_at(
                    &mut self.completion_file,
                    receipt.block_sequence(),
                )?)?;
                let rows = read_row_range(
                    &mut self.row_file,
                    receipt.first_row_record(),
                    receipt.row_count(),
                    self.bounds,
                )?;
                let observed_count = u64::try_from(rows.len()).map_err(|_| {
                    "Finalization V3 bulk-row result count does not fit u64".to_owned()
                })?;
                if observed_count != receipt.row_count() {
                    return Err(format!(
                        "Finalization V3 bulk-row result has {observed_count} rows, expected {}",
                        receipt.row_count()
                    ));
                }

                let recovered = validate_complete_block(&rows, &completion)?;
                if recovered != receipt {
                    return Err(
                        "Finalization V3 bulk-row block differs from retained exact receipt"
                            .to_owned(),
                    );
                }
                Ok(rows)
            })();
            let after = self.require_unchanged();
            match (read, after) {
                (Ok(rows), Ok(())) => Ok(rows),
                (Err(why), _) | (Ok(_), Err(why)) => Err(why),
            }
        })();
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release shared Finalization V3 bulk-row lock: {why}"));
        match (result, released) {
            (Ok(rows), Ok(())) => Ok(rows),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn append(
        &mut self,
        prepared: &PreparedPopulationFinalizationV3,
    ) -> Result<PopulationFinalizationV3StructuralCommit, PopulationFinalizationV3Refusal> {
        if !self.writable {
            return Err("Finalization V3 ledger was opened read-only".to_owned());
        }
        self.lock_file
            .lock()
            .map_err(|why| format!("cannot take exclusive Finalization V3 append lock: {why}"))?;
        let result = self.append_locked(prepared);
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Finalization V3 append lock: {why}"));
        match (result, released) {
            (Ok(commit), Ok(())) => Ok(commit),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn append_locked(
        &mut self,
        prepared: &PreparedPopulationFinalizationV3,
    ) -> Result<PopulationFinalizationV3StructuralCommit, PopulationFinalizationV3Refusal> {
        self.require_unchanged()?;
        let count = self.require_prepared_bound(prepared)?;
        prepared.validate()?;
        if let Some(existing) = self.receipts.get(&prepared.finalization_id).copied() {
            return self.reuse_existing(prepared, existing);
        }
        if let Some(trailing) = self.trailing.clone() {
            return self.complete_trailing(prepared, &trailing);
        }
        self.require_append_bound(count, 1)?;
        let first = self.row_records;
        for (offset, row) in prepared.rows.iter().enumerate() {
            let physical = first
                .checked_add(
                    u64::try_from(offset)
                        .map_err(|_| "Finalization V3 append offset does not fit u64".to_owned())?,
                )
                .ok_or_else(|| "Finalization V3 append physical row overflowed".to_owned())?;
            append_raw(&mut self.row_file, &encode_row(row, physical)?)?;
        }
        self.row_file
            .sync_data()
            .map_err(|why| format!("cannot sync Finalization V3 rows: {why}"))?;
        self.row_records = self
            .row_records
            .checked_add(count)
            .ok_or_else(|| "Finalization V3 row count overflowed after append".to_owned())?;
        self.refresh_row_generation()?;
        self.require_unchanged()?;
        let completion = prepared.expected_completion(self.completion_records, first)?;
        append_raw(&mut self.completion_file, &encode_completion(&completion)?)?;
        self.completion_file
            .sync_data()
            .map_err(|why| format!("cannot sync Finalization V3 Completion: {why}"))?;
        sync_directory(&self.root_file, &self.root)?;
        self.completion_records = self
            .completion_records
            .checked_add(1)
            .ok_or_else(|| "Finalization V3 Completion count overflowed after append".to_owned())?;
        self.refresh_completion_generation()?;
        self.require_unchanged()?;
        self.scan()?;
        let receipt = self
            .receipts
            .get(&prepared.finalization_id)
            .copied()
            .ok_or_else(|| "Finalization V3 appended block was not indexed".to_owned())?;
        Ok(PopulationFinalizationV3StructuralCommit::Written(receipt))
    }

    fn reuse_existing(
        &mut self,
        prepared: &PreparedPopulationFinalizationV3,
        existing: PopulationFinalizationV3StructuralReceipt,
    ) -> Result<PopulationFinalizationV3StructuralCommit, PopulationFinalizationV3Refusal> {
        require_block_bound(self.bounds, existing.row_count())?;
        let observed = read_row_range(
            &mut self.row_file,
            existing.first_row_record(),
            existing.row_count(),
            self.bounds,
        )?;
        if observed != prepared.rows {
            return Err(format!(
                "Finalization V3 identity {} exists with different exact rows",
                hex32(existing.finalization_id())
            ));
        }
        let observed_completion = decode_completion(&read_completion_at(
            &mut self.completion_file,
            existing.block_sequence(),
        )?)?;
        let expected =
            prepared.expected_completion(existing.block_sequence(), existing.first_row_record())?;
        if observed_completion != expected {
            return Err(format!(
                "Finalization V3 identity {} exists with different exact Completion",
                hex32(existing.finalization_id())
            ));
        }
        self.require_unchanged()?;
        self.row_file
            .sync_data()
            .map_err(|why| format!("cannot sync reused Finalization V3 rows: {why}"))?;
        self.completion_file
            .sync_data()
            .map_err(|why| format!("cannot sync reused Finalization V3 Completion: {why}"))?;
        sync_directory(&self.root_file, &self.root)?;
        self.require_unchanged()?;
        Ok(PopulationFinalizationV3StructuralCommit::Reused(existing))
    }

    fn complete_trailing(
        &mut self,
        prepared: &PreparedPopulationFinalizationV3,
        trailing: &TrailingRowsV3,
    ) -> Result<PopulationFinalizationV3StructuralCommit, PopulationFinalizationV3Refusal> {
        if trailing.finalization_id != prepared.finalization_id || trailing.rows != prepared.rows {
            return Err(format!(
                "Finalization V3 trailing block {} is not exact retry {}",
                hex32(trailing.finalization_id),
                hex32(prepared.finalization_id)
            ));
        }
        self.require_append_bound(0, 1)?;
        self.require_unchanged()?;
        self.row_file
            .sync_data()
            .map_err(|why| format!("cannot sync Finalization V3 retry rows: {why}"))?;
        let completion =
            prepared.expected_completion(self.completion_records, trailing.first_row_record)?;
        append_raw(&mut self.completion_file, &encode_completion(&completion)?)?;
        self.completion_file
            .sync_data()
            .map_err(|why| format!("cannot sync Finalization V3 retry Completion: {why}"))?;
        sync_directory(&self.root_file, &self.root)?;
        self.completion_records = self
            .completion_records
            .checked_add(1)
            .ok_or_else(|| "Finalization V3 retry Completion count overflowed".to_owned())?;
        self.refresh_completion_generation()?;
        self.require_unchanged()?;
        self.scan()?;
        let receipt = self
            .receipts
            .get(&prepared.finalization_id)
            .copied()
            .ok_or_else(|| "Finalization V3 retried block was not indexed".to_owned())?;
        Ok(PopulationFinalizationV3StructuralCommit::Written(receipt))
    }

    fn require_prepared_bound(
        &self,
        prepared: &PreparedPopulationFinalizationV3,
    ) -> Result<u64, PopulationFinalizationV3Refusal> {
        let count = prepared.row_count()?;
        require_block_bound(self.bounds, count)?;
        Ok(count)
    }

    fn require_append_bound(
        &self,
        rows: u64,
        completions: u64,
    ) -> Result<(), PopulationFinalizationV3Refusal> {
        let next_rows = self
            .row_records
            .checked_add(rows)
            .ok_or_else(|| "Finalization V3 append row count overflowed".to_owned())?;
        if next_rows > self.bounds.max_row_records() {
            return Err(format!(
                "Finalization V3 append reaches {next_rows} rows, above maximum {}",
                self.bounds.max_row_records()
            ));
        }
        let next_row_bytes = next_rows
            .checked_mul(POPULATION_FINALIZATION_V3_ROW_BYTES as u64)
            .ok_or_else(|| "Finalization V3 append row bytes overflowed".to_owned())?;
        if next_row_bytes > self.bounds.max_row_bytes() {
            return Err(format!(
                "Finalization V3 append reaches {next_row_bytes} row bytes, above maximum {}",
                self.bounds.max_row_bytes()
            ));
        }
        let next_completions = self
            .completion_records
            .checked_add(completions)
            .ok_or_else(|| "Finalization V3 append Completion count overflowed".to_owned())?;
        if next_completions > self.bounds.max_completion_records() {
            return Err(format!(
                "Finalization V3 append reaches {next_completions} Completions, above maximum {}",
                self.bounds.max_completion_records()
            ));
        }
        let next_completion_bytes = next_completions
            .checked_mul(POPULATION_FINALIZATION_V3_COMPLETION_BYTES as u64)
            .ok_or_else(|| "Finalization V3 append Completion bytes overflowed".to_owned())?;
        if next_completion_bytes > self.bounds.max_completion_bytes() {
            return Err(format!(
                "Finalization V3 append reaches {next_completion_bytes} Completion bytes, above maximum {}",
                self.bounds.max_completion_bytes()
            ));
        }
        Ok(())
    }

    fn refresh_row_generation(&mut self) -> Result<(), PopulationFinalizationV3Refusal> {
        self.row_generation =
            file_generation(&self.row_file, &self.row_path, self.bounds.max_row_bytes())?;
        Ok(())
    }

    fn refresh_completion_generation(&mut self) -> Result<(), PopulationFinalizationV3Refusal> {
        self.completion_generation = file_generation(
            &self.completion_file,
            &self.completion_path,
            self.bounds.max_completion_bytes(),
        )?;
        Ok(())
    }

    fn require_unchanged(&self) -> Result<(), PopulationFinalizationV3Refusal> {
        let held_root = self
            .root_file
            .metadata()
            .map_err(|why| format!("cannot stat held Finalization V3 root: {why}"))?;
        if named_root_identity(&self.root)? != self.root_identity
            || PlatformIdentity::of(&held_root) != self.root_identity
        {
            return Err("Finalization V3 root was replaced after open".to_owned());
        }
        let lock = file_generation(&self.lock_file, &self.lock_path, LOCK_MAX_BYTES)?;
        let rows = file_generation(&self.row_file, &self.row_path, self.bounds.max_row_bytes())?;
        let completions = file_generation(
            &self.completion_file,
            &self.completion_path,
            self.bounds.max_completion_bytes(),
        )?;
        if lock != self.lock_generation {
            return Err("Finalization V3 lock generation changed after open".to_owned());
        }
        if rows != self.row_generation {
            return Err("Finalization V3 row generation changed after open".to_owned());
        }
        if completions != self.completion_generation {
            return Err("Finalization V3 Completion generation changed after open".to_owned());
        }
        Ok(())
    }

    fn authenticate_structural_receipt(
        &mut self,
        receipt: PopulationFinalizationV3StructuralReceipt,
        prepared: &PreparedPopulationFinalizationV3,
    ) -> Result<(), PopulationFinalizationV3Refusal> {
        let count = self.require_prepared_bound(prepared)?;
        if count != receipt.row_count()
            || prepared.finalization_id != receipt.finalization_id()
            || prepared.admission_block_id != receipt.admission_block_id()
            || prepared.admission_completion_id != receipt.admission_completion_id()
            || prepared.nifty_row_count != receipt.nifty_row_count()
            || prepared.banknifty_row_count != receipt.banknifty_row_count()
            || ordered_row_digest(&prepared.rows)? != receipt.ordered_row_digest()
        {
            return Err(
                "Finalization V3 structural receipt differs from opaque preparation".to_owned(),
            );
        }
        self.require_unchanged()?;
        if self.receipts.get(&receipt.finalization_id()) != Some(&receipt) {
            return Err(
                "Finalization V3 receipt is not the indexed freshly reopened receipt".to_owned(),
            );
        }
        let rows = read_row_range(
            &mut self.row_file,
            receipt.first_row_record(),
            receipt.row_count(),
            self.bounds,
        )?;
        if rows != prepared.rows {
            return Err("Finalization V3 rows differ from opaque preparation".to_owned());
        }
        let observed = decode_completion(&read_completion_at(
            &mut self.completion_file,
            receipt.block_sequence(),
        )?)?;
        let expected =
            prepared.expected_completion(receipt.block_sequence(), receipt.first_row_record())?;
        if observed != expected || observed.completion_id != receipt.completion_id() {
            return Err("Finalization V3 Completion differs from opaque preparation".to_owned());
        }
        self.require_unchanged()
    }
}

fn persist_population_finalization_v3(
    root: &Path,
    bounds: PopulationFinalizationV3Bounds,
    prepared: &PreparedPopulationFinalizationV3,
) -> Result<PopulationFinalizationV3Commit, PopulationFinalizationV3Refusal> {
    let mut writer = PopulationFinalizationV3Ledger::open_write(root, bounds)?;
    let structural = writer.append(prepared)?;
    let was_written = matches!(
        structural,
        PopulationFinalizationV3StructuralCommit::Written(_)
    );
    let receipt = structural.receipt();
    let written_root_identity = writer.root_identity;
    let written_lock_identity = writer.lock_generation.identity;
    let written_row_identity = writer.row_generation.identity;
    let written_completion_identity = writer.completion_generation.identity;
    drop(writer);

    let mut reopened = PopulationFinalizationV3Ledger::open_read(root, bounds)?;
    if reopened.root_identity != written_root_identity
        || reopened.lock_generation.identity != written_lock_identity
        || reopened.row_generation.identity != written_row_identity
        || reopened.completion_generation.identity != written_completion_identity
    {
        return Err(
            "Finalization V3 root or file identity changed across mandatory fresh reopen"
                .to_owned(),
        );
    }
    let observed = reopened
        .reopen_structural_receipt(&receipt.finalization_id())?
        .ok_or_else(|| "Finalization V3 fresh reopen omitted persisted receipt".to_owned())?;
    if observed != receipt {
        return Err("Finalization V3 fresh reopen changed structural receipt".to_owned());
    }
    reopened.authenticate_structural_receipt(observed, prepared)?;
    let authority = PopulationFinalizationV3Authority {
        receipt: observed,
        ledger: reopened,
    };
    Ok(if was_written {
        PopulationFinalizationV3Commit::Written(authority)
    } else {
        PopulationFinalizationV3Commit::Reused(authority)
    })
}

fn validate_complete_block(
    rows: &[PopulationFinalizationV3RowProjection],
    completion: &PopulationFinalizationCompletionV3,
) -> Result<PopulationFinalizationV3StructuralReceipt, PopulationFinalizationV3Refusal> {
    let prepared = PreparedPopulationFinalizationV3 {
        finalization_id: completion.finalization_id,
        admission_block_id: completion.admission_block_id,
        admission_completion_id: completion.admission_completion_id,
        nifty_row_count: completion.nifty_row_count,
        banknifty_row_count: completion.banknifty_row_count,
        rows: rows.to_vec(),
    };
    prepared.validate()?;
    let expected =
        prepared.expected_completion(completion.block_sequence, completion.first_row_record)?;
    if expected != *completion {
        return Err("Finalization V3 Completion does not exactly complete ordered rows".to_owned());
    }
    Ok(PopulationFinalizationV3StructuralReceipt {
        finalization_id: completion.finalization_id,
        admission_block_id: completion.admission_block_id,
        admission_completion_id: completion.admission_completion_id,
        block_sequence: completion.block_sequence,
        first_row_record: completion.first_row_record,
        row_count: completion.row_count,
        nifty_row_count: completion.nifty_row_count,
        banknifty_row_count: completion.banknifty_row_count,
        admitted_count: completion.admitted_count,
        rejected_count: completion.rejected_count,
        unmeasured_count: completion.unmeasured_count,
        refused_count: completion.refused_count,
        ordered_row_digest: completion.ordered_row_digest,
        completion_id: completion.completion_id,
    })
}

fn validate_trailing_rows(
    rows: &[PopulationFinalizationV3RowProjection],
) -> Result<[u8; 32], PopulationFinalizationV3Refusal> {
    let first = rows
        .first()
        .ok_or_else(|| "Finalization V3 trailing block is empty".to_owned())?;
    let finalization_id = first.finalization_id;
    let admission_block_id = first.admission_block_id;
    let admission_completion_id = first.admission_completion_id;
    let mut bank_started = false;
    let mut bank_sequence = 0_u64;
    let mut semantics = bounded_set(rows.len(), "trailing Candidate semantic")?;
    let mut row_ids = bounded_set(rows.len(), "trailing row")?;
    for (index, row) in rows.iter().enumerate() {
        row.validate()?;
        let global = u64::try_from(index)
            .map_err(|_| "Finalization V3 trailing index does not fit u64".to_owned())?;
        if row.finalization_id != finalization_id
            || row.admission_block_id != admission_block_id
            || row.admission_completion_id != admission_completion_id
            || row.global_sequence != global
        {
            return Err(format!(
                "Finalization V3 trailing row {index} changes block or global order"
            ));
        }
        match row.family {
            AdmissionV3Family::Nifty if bank_started => {
                return Err(
                    "Finalization V3 trailing block returns to NIFTY after BANKNIFTY".to_owned(),
                );
            }
            AdmissionV3Family::Nifty => {
                if row.family_sequence != global {
                    return Err(format!(
                        "Finalization V3 trailing NIFTY sequence {} is not {global}",
                        row.family_sequence
                    ));
                }
            }
            AdmissionV3Family::BankNifty => {
                bank_started = true;
                if row.family_sequence != bank_sequence {
                    return Err(format!(
                        "Finalization V3 trailing BANKNIFTY sequence {} is not {bank_sequence}",
                        row.family_sequence
                    ));
                }
                bank_sequence = bank_sequence.checked_add(1).ok_or_else(|| {
                    "Finalization V3 trailing BANKNIFTY sequence overflowed".to_owned()
                })?;
            }
        }
        if !semantics.insert(row.candidate_semantic_id) {
            return Err("Finalization V3 trailing Candidate semantic is duplicated".to_owned());
        }
        if !row_ids.insert(row.row_id) {
            return Err("Finalization V3 trailing row identity is duplicated".to_owned());
        }
    }
    Ok(finalization_id)
}

fn checked_record_count(
    bytes: u64,
    stride: usize,
    max_records: u64,
    name: &str,
) -> Result<u64, PopulationFinalizationV3Refusal> {
    let stride_u64 = u64::try_from(stride)
        .map_err(|_| format!("Finalization V3 {name} stride does not fit u64"))?;
    if !bytes.is_multiple_of(stride_u64) {
        return Err(format!(
            "Finalization V3 {name} file has ragged length {bytes}, not a multiple of {stride}"
        ));
    }
    let records = bytes / stride_u64;
    if records > max_records {
        return Err(format!(
            "Finalization V3 {name} file has {records} records, above maximum {max_records}"
        ));
    }
    Ok(records)
}

fn read_row_range(
    file: &mut File,
    first: u64,
    count: u64,
    bounds: PopulationFinalizationV3Bounds,
) -> Result<Vec<PopulationFinalizationV3RowProjection>, PopulationFinalizationV3Refusal> {
    require_block_bound(bounds, count)?;
    let end = first
        .checked_add(count)
        .ok_or_else(|| "Finalization V3 row read range overflowed".to_owned())?;
    if end > bounds.max_row_records() {
        return Err(format!(
            "Finalization V3 row read ends at {end}, above maximum {}",
            bounds.max_row_records()
        ));
    }
    let capacity = usize::try_from(count)
        .map_err(|_| "Finalization V3 row read count does not fit usize".to_owned())?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(capacity)
        .map_err(|why| format!("cannot reserve Finalization V3 row read: {why}"))?;
    let mut physical = first;
    while physical < end {
        let raw = read_fixed_at::<POPULATION_FINALIZATION_V3_ROW_BYTES>(
            file,
            physical,
            POPULATION_FINALIZATION_V3_ROW_BYTES,
            "row",
        )?;
        rows.push(decode_row(&raw, physical)?);
        physical = physical
            .checked_add(1)
            .ok_or_else(|| "Finalization V3 row read cursor overflowed".to_owned())?;
    }
    Ok(rows)
}

fn read_completion_at(
    file: &mut File,
    physical: u64,
) -> Result<[u8; POPULATION_FINALIZATION_V3_COMPLETION_BYTES], PopulationFinalizationV3Refusal> {
    read_fixed_at::<POPULATION_FINALIZATION_V3_COMPLETION_BYTES>(
        file,
        physical,
        POPULATION_FINALIZATION_V3_COMPLETION_BYTES,
        "Completion",
    )
}

fn read_fixed_at<const N: usize>(
    file: &mut File,
    physical: u64,
    stride: usize,
    name: &str,
) -> Result<[u8; N], PopulationFinalizationV3Refusal> {
    let stride_u64 = u64::try_from(stride)
        .map_err(|_| format!("Finalization V3 {name} stride does not fit u64"))?;
    let offset = physical
        .checked_mul(stride_u64)
        .ok_or_else(|| format!("Finalization V3 {name} offset overflowed"))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|why| format!("cannot seek Finalization V3 {name} {physical}: {why}"))?;
    let mut raw = [0_u8; N];
    file.read_exact(&mut raw)
        .map_err(|why| format!("cannot read Finalization V3 {name} {physical}: {why}"))?;
    Ok(raw)
}

fn append_raw(file: &mut File, raw: &[u8]) -> Result<(), PopulationFinalizationV3Refusal> {
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(raw))
        .map_err(|why| format!("cannot append Finalization V3 fixed record: {why}"))
}

fn open_root_directory(
    root: &Path,
) -> Result<(PathBuf, File, PlatformIdentity), PopulationFinalizationV3Refusal> {
    require_not_symlink(root, false)?;
    let canonical = std::fs::canonicalize(root).map_err(|why| {
        format!(
            "Finalization V3 root {} must already exist: {why}",
            root.display()
        )
    })?;
    require_not_symlink(&canonical, false)?;
    let file = File::open(&canonical).map_err(|why| {
        format!(
            "cannot hold Finalization V3 root {}: {why}",
            canonical.display()
        )
    })?;
    let metadata = file.metadata().map_err(|why| {
        format!(
            "cannot stat held Finalization V3 root {}: {why}",
            canonical.display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "Finalization V3 root {} is not a directory",
            canonical.display()
        ));
    }
    Ok((canonical, file, PlatformIdentity::of(&metadata)))
}

fn named_root_identity(root: &Path) -> Result<PlatformIdentity, PopulationFinalizationV3Refusal> {
    require_not_symlink(root, false)?;
    let file = File::open(root).map_err(|why| {
        format!(
            "cannot reopen named Finalization V3 root {}: {why}",
            root.display()
        )
    })?;
    let metadata = file.metadata().map_err(|why| {
        format!(
            "cannot stat named Finalization V3 root {}: {why}",
            root.display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "named Finalization V3 root {} is not a directory",
            root.display()
        ));
    }
    Ok(PlatformIdentity::of(&metadata))
}

fn open_child(
    path: &Path,
    writable: bool,
    create: bool,
) -> Result<(File, bool), PopulationFinalizationV3Refusal> {
    require_not_symlink(path, create)?;
    if create {
        let mut create_options = OpenOptions::new();
        create_options.read(true).write(true).create_new(true);
        #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
        create_options.custom_flags(O_NOFOLLOW_FLAG);
        match create_options.open(path) {
            Ok(file) => {
                require_regular_file(&file, path)?;
                return Ok((file, true));
            }
            Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(why) => {
                return Err(format!(
                    "cannot create Finalization V3 file {}: {why}",
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
        .map_err(|why| format!("cannot open Finalization V3 file {}: {why}", path.display()))?;
    require_not_symlink(path, false)?;
    require_regular_file(&file, path)?;
    Ok((file, false))
}

fn require_regular_file(file: &File, path: &Path) -> Result<(), PopulationFinalizationV3Refusal> {
    if !file
        .metadata()
        .map_err(|why| format!("cannot stat Finalization V3 file {}: {why}", path.display()))?
        .is_file()
    {
        return Err(format!(
            "Finalization V3 path {} is not a regular file",
            path.display()
        ));
    }
    Ok(())
}

fn require_not_symlink(
    path: &Path,
    absent_is_allowed: bool,
) -> Result<(), PopulationFinalizationV3Refusal> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(format!(
            "Finalization V3 path {} is a symbolic link; no-follow refused it",
            path.display()
        )),
        Ok(_) => Ok(()),
        Err(why) if absent_is_allowed && why.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(why) => Err(format!(
            "cannot inspect Finalization V3 path {} without following links: {why}",
            path.display()
        )),
    }
}

fn sync_directory(root_file: &File, root: &Path) -> Result<(), PopulationFinalizationV3Refusal> {
    root_file.sync_all().map_err(|why| {
        format!(
            "cannot durably sync Finalization V3 directory entries in {}: {why}",
            root.display()
        )
    })
}

fn file_generation(
    file: &File,
    path: &Path,
    max_bytes: u64,
) -> Result<FileGeneration, PopulationFinalizationV3Refusal> {
    file_generation_with_between_hash_action(file, path, max_bytes, || Ok(()))
}

fn file_generation_with_between_hash_action(
    file: &File,
    path: &Path,
    max_bytes: u64,
    between_hashes: impl FnOnce() -> Result<(), PopulationFinalizationV3Refusal>,
) -> Result<FileGeneration, PopulationFinalizationV3Refusal> {
    let before_metadata = file.metadata().map_err(|why| {
        format!(
            "cannot stat held Finalization V3 file {}: {why}",
            path.display()
        )
    })?;
    if !before_metadata.is_file() {
        return Err(format!(
            "held Finalization V3 path {} is not a regular file",
            path.display()
        ));
    }
    if before_metadata.len() > max_bytes {
        return Err(format!(
            "Finalization V3 file {} has {} bytes, above maximum {max_bytes}",
            path.display(),
            before_metadata.len()
        ));
    }
    let before = MetadataGeneration::of(&before_metadata);
    let identity = before.identity;
    let (named, _) = open_child(path, false, false)?;
    let named_metadata = named.metadata().map_err(|why| {
        format!(
            "cannot stat named Finalization V3 file {}: {why}",
            path.display()
        )
    })?;
    if PlatformIdentity::of(&named_metadata) != identity {
        return Err(format!(
            "Finalization V3 file {} was path-replaced after open",
            path.display()
        ));
    }
    let digest = hash_held_prefix(file, path, before.len)?;
    between_hashes()?;
    let middle_metadata = file.metadata().map_err(|why| {
        format!(
            "cannot restat held Finalization V3 file {} after first hash: {why}",
            path.display()
        )
    })?;
    if MetadataGeneration::of(&middle_metadata) != before || named_path_identity(path)? != identity
    {
        return Err(format!(
            "Finalization V3 file {} changed during bounded generation hashing",
            path.display()
        ));
    }
    let confirmation = hash_held_prefix(file, path, before.len)?;
    let after_metadata = file.metadata().map_err(|why| {
        format!(
            "cannot restat held Finalization V3 file {} after second hash: {why}",
            path.display()
        )
    })?;
    if MetadataGeneration::of(&after_metadata) != before || named_path_identity(path)? != identity {
        return Err(format!(
            "Finalization V3 file {} changed during bounded generation hashing",
            path.display()
        ));
    }
    if confirmation != digest {
        return Err(format!(
            "Finalization V3 file {} changed between bounded generation hashes",
            path.display()
        ));
    }
    Ok(FileGeneration {
        identity,
        len: before.len,
        metadata: before,
        digest,
    })
}

fn named_path_identity(path: &Path) -> Result<PlatformIdentity, PopulationFinalizationV3Refusal> {
    require_not_symlink(path, false)?;
    let metadata = std::fs::metadata(path).map_err(|why| {
        format!(
            "cannot stat named Finalization V3 path {}: {why}",
            path.display()
        )
    })?;
    Ok(PlatformIdentity::of(&metadata))
}

fn hash_held_prefix(
    file: &File,
    path: &Path,
    length: u64,
) -> Result<[u8; 32], PopulationFinalizationV3Refusal> {
    let mut reader = file.try_clone().map_err(|why| {
        format!(
            "cannot clone Finalization V3 file {}: {why}",
            path.display()
        )
    })?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|why| format!("cannot seek Finalization V3 file {}: {why}", path.display()))?;
    hash_exact_prefix(&mut reader, path, length)
}

fn hash_exact_prefix(
    reader: &mut impl std::io::Read,
    path: &Path,
    length: u64,
) -> Result<[u8; 32], PopulationFinalizationV3Refusal> {
    let mut hasher = Hasher::new();
    hasher.update(FILE_GENERATION_DOMAIN);
    hasher.update(&length.to_le_bytes());
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    let mut remaining = length;
    while remaining != 0 {
        let requested = usize::try_from(remaining.min(READ_CHUNK_BYTES as u64))
            .map_err(|_| "Finalization V3 bounded hash width does not fit usize".to_owned())?;
        let read = reader
            .read(
                buffer
                    .get_mut(..requested)
                    .ok_or_else(|| "Finalization V3 bounded hash range is invalid".to_owned())?,
            )
            .map_err(|why| format!("cannot hash Finalization V3 file {}: {why}", path.display()))?;
        if read == 0 {
            return Err(format!(
                "Finalization V3 file {} shortened while hashing",
                path.display()
            ));
        }
        hasher.update(
            buffer
                .get(..read)
                .ok_or_else(|| "Finalization V3 hash chunk range is invalid".to_owned())?,
        );
        remaining = remaining
            .checked_sub(
                u64::try_from(read)
                    .map_err(|_| "Finalization V3 generation read does not fit u64".to_owned())?,
            )
            .ok_or_else(|| "Finalization V3 bounded hash count underflowed".to_owned())?;
    }
    Ok(hasher.finalize())
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "focused adversarial tests intentionally fail fixture setup and mutate exact fixed offsets"
)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_ROOT: AtomicU64 = AtomicU64::new(0);

    struct TestRoot {
        path: PathBuf,
    }

    impl TestRoot {
        fn new(label: &str) -> Self {
            let sequence = NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "brutex-population-finalization-v3-{}-{label}-{sequence}",
                std::process::id()
            ));
            std::fs::create_dir(&path).expect("create Finalization V3 test root");
            Self { path }
        }

        fn absent(label: &str) -> Self {
            let sequence = NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed);
            Self {
                path: std::env::temp_dir().join(format!(
                    "brutex-population-finalization-v3-absent-{}-{label}-{sequence}",
                    std::process::id()
                )),
            }
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
                std::fs::remove_dir_all(&self.path).expect("remove Finalization V3 test root");
            } else {
                std::fs::remove_file(&self.path).expect("remove Finalization V3 test path");
            }
        }
    }

    fn digest(seed: u64) -> [u8; 32] {
        hash_slices(
            b"brutex-population-finalization-v3-test-value\0",
            &[&seed.to_le_bytes()],
        )
    }

    fn row(
        global_sequence: u64,
        family: AdmissionV3Family,
        family_sequence: u64,
        status: AdmissionV3Status,
    ) -> PopulationFinalizationV3RowProjection {
        let family_seed = match family {
            AdmissionV3Family::Nifty => 1_000,
            AdmissionV3Family::BankNifty => 2_000,
        };
        let candidate_seed = 10_000 + global_sequence * 20;
        let mut value = PopulationFinalizationV3RowProjection {
            finalization_id: [0; 32],
            global_sequence,
            family,
            family_sequence,
            status,
            candidate_universe_id: digest(family_seed),
            candidate_completion_digest: digest(family_seed + 1),
            candidate_semantic_id: digest(candidate_seed),
            candidate_row_digest: digest(candidate_seed + 1),
            pre_admission_authority_id: digest(family_seed + 2),
            statistics_audit_id: digest(10),
            statistics_completion_digest: digest(11),
            statistics_period_digest: digest(candidate_seed + 2),
            statistics_split_digest: digest(candidate_seed + 3),
            search_pair_id: digest(12),
            search_member_id: digest(family_seed + 3),
            search_signal_digest: digest(family_seed + 4),
            search_signal_bars: 10_000 + family_seed,
            search_signal_first_ts_micros: 1_704_067_200_000_000,
            search_signal_last_ts_micros: 1_735_603_140_000_000,
            search_signal_column_digest: digest(family_seed + 5),
            search_policy_id: digest(family_seed + 6),
            search_full_grid_id: digest(family_seed + 7),
            search_long_policy_id: digest(family_seed + 8),
            search_long_resolution_id: digest(family_seed + 9),
            search_short_policy_id: digest(family_seed + 10),
            search_short_resolution_id: digest(family_seed + 11),
            search_family_id: digest(family_seed + 12),
            search_walk_id: digest(family_seed + 13),
            search_fold_count: 8,
            search_decided_folds: 6,
            search_profitable_oos_folds: 3,
            search_aggregate_oos_paisa: 125,
            search_evaluated_population_count: 8_192,
            paired_base_id: digest(13),
            base_completion_id: digest(family_seed + 14),
            base_evidence_id: digest(candidate_seed + 4),
            admission_block_id: digest(14),
            admission_completion_id: digest(15),
            admission_runner_decision_digest: digest(candidate_seed + 5),
            admission_evidence_digest: digest(candidate_seed + 6),
            admission_verdict_digest: digest(candidate_seed + 7),
            admission_decision_id: digest(candidate_seed + 8),
            row_id: [0; 32],
        };
        value.row_id = value.derive_row_id().expect("derive fixture row");
        value
    }

    fn reseal_prepared(
        mut value: PreparedPopulationFinalizationV3,
    ) -> PreparedPopulationFinalizationV3 {
        for row in &mut value.rows {
            row.finalization_id = [0; 32];
            row.row_id = row.derive_row_id().expect("rederive fixture row");
        }
        value.finalization_id = derive_finalization_id(
            value.admission_block_id,
            value.admission_completion_id,
            value.nifty_row_count,
            value.banknifty_row_count,
            &value.rows,
        )
        .expect("derive fixture Finalization identity");
        for row in &mut value.rows {
            row.finalization_id = value.finalization_id;
        }
        value
    }

    fn prepared() -> PreparedPopulationFinalizationV3 {
        reseal_prepared(PreparedPopulationFinalizationV3 {
            finalization_id: [0; 32],
            admission_block_id: digest(14),
            admission_completion_id: digest(15),
            nifty_row_count: 2,
            banknifty_row_count: 2,
            rows: vec![
                row(0, AdmissionV3Family::Nifty, 0, AdmissionV3Status::Admitted),
                row(1, AdmissionV3Family::Nifty, 1, AdmissionV3Status::Rejected),
                row(
                    2,
                    AdmissionV3Family::BankNifty,
                    0,
                    AdmissionV3Status::Unmeasured,
                ),
                row(
                    3,
                    AdmissionV3Family::BankNifty,
                    1,
                    AdmissionV3Status::Refused,
                ),
            ],
        })
    }

    fn altered_prepared(seed: u64) -> PreparedPopulationFinalizationV3 {
        let mut value = prepared();
        value.rows[0].candidate_row_digest = digest(seed);
        reseal_prepared(value)
    }

    fn bounds() -> PopulationFinalizationV3Bounds {
        PopulationFinalizationV3Bounds::new(
            32,
            32 * POPULATION_FINALIZATION_V3_ROW_BYTES as u64,
            8,
            8 * POPULATION_FINALIZATION_V3_COMPLETION_BYTES as u64,
            8,
        )
        .expect("fixture Finalization V3 bounds")
    }

    fn create_empty_files(root: &Path) {
        File::create(root.join(LOCK_FILE)).expect("create fixture lock");
        File::create(root.join(ROW_FILE)).expect("create fixture row file");
        File::create(root.join(COMPLETION_FILE)).expect("create fixture Completion file");
    }

    fn write_block(root: &Path, value: &PreparedPopulationFinalizationV3, complete: bool) {
        create_empty_files(root);
        let mut rows = OpenOptions::new()
            .append(true)
            .open(root.join(ROW_FILE))
            .expect("open fixture row file");
        for (physical, row) in value.rows.iter().enumerate() {
            rows.write_all(
                &encode_row(
                    row,
                    u64::try_from(physical).expect("fixture physical sequence"),
                )
                .expect("encode fixture row"),
            )
            .expect("write fixture row");
        }
        rows.sync_data().expect("sync fixture rows");
        if complete {
            let completion = value.expected_completion(0, 0).expect("fixture Completion");
            let mut completions = OpenOptions::new()
                .append(true)
                .open(root.join(COMPLETION_FILE))
                .expect("open fixture Completion file");
            completions
                .write_all(&encode_completion(&completion).expect("encode fixture Completion"))
                .expect("write fixture Completion");
            completions.sync_data().expect("sync fixture Completion");
        }
    }

    fn reseal<const N: usize>(raw: &mut [u8; N], payload: usize, domain: &[u8]) {
        let seal = hash_slices(domain, &[&raw[..payload]]);
        raw[payload..].copy_from_slice(&seal);
    }

    fn assert_refuses<T>(result: Result<T, PopulationFinalizationV3Refusal>, expected: &str) {
        let Err(why) = result else {
            panic!("malformed Finalization V3 input must refuse");
        };
        assert!(
            why.contains(expected),
            "refusal `{why}` did not contain `{expected}`"
        );
    }

    #[test]
    fn fixed_codecs_are_v3_only_receipt_last_and_every_identity_byte_is_load_bearing() {
        let value = prepared();
        value.validate().expect("fixture validates");
        let first = &value.rows[0];
        let mut identity = [0_u8; ROW_IDENTITY_BYTES];
        let mut writer = FixedWriter::new(&mut identity);
        first
            .encode_identity_body(&mut writer)
            .expect("encode fixture identity");
        writer
            .require_full("fixture identity")
            .expect("exact width");
        for offset in 0..identity.len() {
            let mut changed = identity;
            changed[offset] ^= 1;
            assert_ne!(
                hash_slices(ROW_ID_DOMAIN, &[&VERSION.to_le_bytes(), &changed]),
                first.row_id,
                "row identity byte {offset} was not load-bearing"
            );
        }

        let raw = encode_row(first, 17).expect("encode row");
        assert_eq!(&raw[..16], &ROW_MAGIC);
        assert_eq!(
            u32::from_le_bytes(raw[16..20].try_into().expect("version")),
            3
        );
        assert_eq!(&decode_row(&raw, 17).expect("decode row"), first);
        assert!(raw[1_152..ROW_PAYLOAD_BYTES].iter().all(|byte| *byte == 0));

        let completion = value.expected_completion(9, 17).expect("Completion");
        let completion_raw = encode_completion(&completion).expect("encode Completion");
        assert_eq!(&completion_raw[..16], &COMPLETION_MAGIC);
        assert_eq!(
            decode_completion(&completion_raw).expect("decode Completion"),
            completion
        );
        assert!(
            completion_raw[256..COMPLETION_PAYLOAD_BYTES]
                .iter()
                .all(|byte| *byte == 0)
        );

        let mut wrong_version = raw;
        wrong_version[16..20].copy_from_slice(&2_u32.to_le_bytes());
        reseal(&mut wrong_version, ROW_PAYLOAD_BYTES, ROW_SEAL_DOMAIN);
        assert_refuses(decode_row(&wrong_version, 17), "header is invalid");

        let mut wrong_domain = raw;
        wrong_domain[20..24].copy_from_slice(&COMPLETION_DOMAIN.to_le_bytes());
        reseal(&mut wrong_domain, ROW_PAYLOAD_BYTES, ROW_SEAL_DOMAIN);
        assert_refuses(decode_row(&wrong_domain, 17), "header is invalid");

        let mut wrong_physical = raw;
        wrong_physical[24..32].copy_from_slice(&18_u64.to_le_bytes());
        reseal(&mut wrong_physical, ROW_PAYLOAD_BYTES, ROW_SEAL_DOMAIN);
        assert_refuses(decode_row(&wrong_physical, 17), "physical sequence");

        let mut wrong_reserve = raw;
        wrong_reserve[ROW_PAYLOAD_BYTES - 1] = 1;
        reseal(&mut wrong_reserve, ROW_PAYLOAD_BYTES, ROW_SEAL_DOMAIN);
        assert_refuses(decode_row(&wrong_reserve, 17), "row reserve");

        let mut corrupt = raw;
        corrupt[200] ^= 1;
        assert_refuses(decode_row(&corrupt, 17), "seal is invalid");
    }

    #[test]
    fn family_global_candidate_statistics_search_base_and_admission_joins_refuse() {
        let mut reordered = prepared();
        reordered.rows.swap(0, 1);
        assert_refuses(reordered.validate(), "NIFTY-first ordering");

        let mut duplicate = prepared();
        duplicate.rows[1].candidate_semantic_id = duplicate.rows[0].candidate_semantic_id;
        duplicate = reseal_prepared(duplicate);
        assert_refuses(duplicate.validate(), "duplicates Candidate semantic");

        let mut cross_family = prepared();
        cross_family.rows[1].search_signal_column_digest = digest(90_001);
        cross_family = reseal_prepared(cross_family);
        assert_refuses(cross_family.validate(), "crosswire family authority");

        let mut cross_global = prepared();
        cross_global.rows[3].paired_base_id = digest(90_002);
        cross_global = reseal_prepared(cross_global);
        assert_refuses(cross_global.validate(), "crosswire global authority");

        let mut foreign_admission = prepared();
        foreign_admission.rows[2].admission_completion_id = digest(90_003);
        foreign_admission = reseal_prepared(foreign_admission);
        assert_refuses(foreign_admission.validate(), "violates block");

        let mut aliased_sides = prepared();
        aliased_sides.rows[0].search_short_policy_id = aliased_sides.rows[0].search_long_policy_id;
        aliased_sides = reseal_prepared(aliased_sides);
        assert_refuses(aliased_sides.validate(), "Long/Short components alias");

        let mut zero_family = prepared();
        zero_family.nifty_row_count = 0;
        zero_family.banknifty_row_count = 4;
        zero_family = reseal_prepared(zero_family);
        assert_refuses(zero_family.validate(), "NIFTY-first ordering");
    }

    #[test]
    fn extinct_or_one_sided_candidate_families_remain_complete_receipts() {
        let empty = reseal_prepared(PreparedPopulationFinalizationV3 {
            finalization_id: [0; 32],
            admission_block_id: digest(14),
            admission_completion_id: digest(15),
            nifty_row_count: 0,
            banknifty_row_count: 0,
            rows: Vec::new(),
        });
        empty.validate().expect("empty preparation validates");
        let completion = empty.expected_completion(0, 0).expect("empty Completion");
        assert_eq!(completion.row_count, 0);
        assert_eq!(completion.admitted_count, 0);

        let mut nifty_only = prepared();
        nifty_only.rows.truncate(2);
        nifty_only.banknifty_row_count = 0;
        let nifty_only = reseal_prepared(nifty_only);
        nifty_only
            .validate()
            .expect("one-sided preparation validates");

        let root = TestRoot::new("empty-complete");
        let first = persist_population_finalization_v3(root.path(), bounds(), &empty)
            .expect("persist empty Finalization");
        assert!(matches!(first, PopulationFinalizationV3Commit::Written(_)));
        let second = persist_population_finalization_v3(root.path(), bounds(), &empty)
            .expect("reuse empty Finalization");
        let mut authority = second.into_authority();
        assert_eq!(authority.structural_receipt().row_count(), 0);
        assert_eq!(
            authority
                .ordered_row_projections()
                .expect("empty authenticated bulk projection"),
            Vec::new()
        );
        assert!(authority.row_projection(0).is_err());
    }

    #[test]
    fn explicit_bounds_existing_root_fresh_reopen_and_exact_reuse_are_required() {
        for invalid in [
            PopulationFinalizationV3Bounds::new(0, 1, 1, 4_096, 1),
            PopulationFinalizationV3Bounds::new(1, 2_048, 0, 1, 1),
            PopulationFinalizationV3Bounds::new(1, 2_048, 1, 4_096, 0),
            PopulationFinalizationV3Bounds::new(1, 2_047, 1, 4_096, 1),
            PopulationFinalizationV3Bounds::new(1, 2_048, 1, 4_095, 1),
            PopulationFinalizationV3Bounds::new(1, 2_048, 1, 4_096, 2),
        ] {
            assert!(invalid.is_err());
        }
        let limits = bounds();
        assert_eq!(limits.max_row_records(), 32);
        assert_eq!(limits.max_completion_records(), 8);

        let absent = TestRoot::absent("read-does-not-create");
        assert!(PopulationFinalizationV3Ledger::open_read(absent.path(), limits).is_err());
        assert!(!absent.path().exists());
        assert!(persist_population_finalization_v3(absent.path(), limits, &prepared()).is_err());
        assert!(!absent.path().exists());

        let root = TestRoot::new("write-reuse");
        let value = prepared();
        let first = persist_population_finalization_v3(root.path(), limits, &value)
            .expect("persist first block");
        let mut authority = match first {
            PopulationFinalizationV3Commit::Written(value) => value,
            PopulationFinalizationV3Commit::Reused(_) => panic!("first write must be Written"),
        };
        let receipt = authority.structural_receipt();
        assert_eq!(receipt.finalization_id(), value.finalization_id);
        assert_eq!(receipt.block_sequence(), 0);
        assert_eq!(receipt.first_row_record(), 0);
        assert_eq!(receipt.row_count(), 4);
        assert_eq!(receipt.nifty_row_count(), 2);
        assert_eq!(receipt.banknifty_row_count(), 2);
        assert_eq!(receipt.admitted_count(), 1);
        assert_eq!(receipt.rejected_count(), 1);
        assert_eq!(receipt.unmeasured_count(), 1);
        assert_eq!(receipt.refused_count(), 1);
        assert_ne!(receipt.completion_id(), [0; 32]);
        assert_eq!(
            authority
                .ordered_row_projections()
                .expect("complete authenticated bulk projection"),
            value.rows
        );
        assert_eq!(
            authority
                .row_projection(2)
                .expect("fixed-offset authenticated row"),
            value.rows[2]
        );
        assert!(authority.row_projection(4).is_err());

        let retry = persist_population_finalization_v3(root.path(), limits, &value)
            .expect("reuse exact block");
        assert!(matches!(retry, PopulationFinalizationV3Commit::Reused(_)));
        assert_eq!(
            std::fs::metadata(root.path().join(ROW_FILE))
                .expect("stat row file")
                .len(),
            4 * POPULATION_FINALIZATION_V3_ROW_BYTES as u64
        );
        assert_eq!(
            std::fs::metadata(root.path().join(COMPLETION_FILE))
                .expect("stat Completion file")
                .len(),
            POPULATION_FINALIZATION_V3_COMPLETION_BYTES as u64
        );

        let read_only = PopulationFinalizationV3Ledger::open_read(root.path(), limits)
            .expect("open read-only ledger");
        assert_eq!(
            read_only
                .reopen_structural_receipt(&value.finalization_id)
                .expect("lookup structural receipt"),
            Some(receipt)
        );
    }

    #[test]
    fn full_orphan_exact_retry_completes_once_and_foreign_retry_refuses() {
        let limits = bounds();
        let value = prepared();
        let root = TestRoot::new("orphan-retry");
        write_block(root.path(), &value, false);
        let before_rows = std::fs::read(root.path().join(ROW_FILE)).expect("read orphan rows");
        let commit = persist_population_finalization_v3(root.path(), limits, &value)
            .expect("complete exact orphan");
        assert!(matches!(commit, PopulationFinalizationV3Commit::Written(_)));
        assert_eq!(
            std::fs::read(root.path().join(ROW_FILE)).expect("reread orphan rows"),
            before_rows
        );
        assert_eq!(
            std::fs::metadata(root.path().join(COMPLETION_FILE))
                .expect("stat retried Completion")
                .len(),
            POPULATION_FINALIZATION_V3_COMPLETION_BYTES as u64
        );

        let foreign_root = TestRoot::new("foreign-orphan");
        write_block(foreign_root.path(), &value, false);
        let before = std::fs::read(foreign_root.path().join(ROW_FILE)).expect("read foreign rows");
        assert_refuses(
            persist_population_finalization_v3(
                foreign_root.path(),
                limits,
                &altered_prepared(91_000),
            ),
            "not exact retry",
        );
        assert_eq!(
            std::fs::read(foreign_root.path().join(ROW_FILE)).expect("reread foreign rows"),
            before
        );
    }

    #[test]
    fn ragged_torn_corrupt_resealed_reordered_and_structural_forgery_refuse() {
        let limits = bounds();

        let ragged = TestRoot::new("ragged");
        create_empty_files(ragged.path());
        std::fs::write(ragged.path().join(ROW_FILE), [1_u8]).expect("write ragged row file");
        assert_refuses(
            PopulationFinalizationV3Ledger::open_read(ragged.path(), limits),
            "ragged length",
        );

        let torn = TestRoot::new("torn");
        let value = prepared();
        write_block(torn.path(), &value, true);
        OpenOptions::new()
            .write(true)
            .open(torn.path().join(ROW_FILE))
            .expect("open torn rows")
            .set_len(3 * POPULATION_FINALIZATION_V3_ROW_BYTES as u64)
            .expect("truncate torn rows");
        assert_refuses(
            PopulationFinalizationV3Ledger::open_read(torn.path(), limits),
            "is torn",
        );

        let corrupt = TestRoot::new("corrupt");
        write_block(corrupt.path(), &value, true);
        let row_path = corrupt.path().join(ROW_FILE);
        let mut raw = std::fs::read(&row_path).expect("read corrupt rows");
        raw[200] ^= 1;
        std::fs::write(&row_path, raw).expect("write corrupt rows");
        assert_refuses(
            PopulationFinalizationV3Ledger::open_read(corrupt.path(), limits),
            "seal is invalid",
        );

        let resealed = TestRoot::new("resealed-semantic");
        write_block(resealed.path(), &value, true);
        let row_path = resealed.path().join(ROW_FILE);
        let mut first = read_fixed_at::<POPULATION_FINALIZATION_V3_ROW_BYTES>(
            &mut File::open(&row_path).expect("open resealed row"),
            0,
            POPULATION_FINALIZATION_V3_ROW_BYTES,
            "row",
        )
        .expect("read resealed row");
        first[200] ^= 1;
        reseal(&mut first, ROW_PAYLOAD_BYTES, ROW_SEAL_DOMAIN);
        let mut file = OpenOptions::new()
            .write(true)
            .open(&row_path)
            .expect("open resealed writer");
        file.write_all(&first).expect("write resealed row");
        file.sync_data().expect("sync resealed row");
        assert!(PopulationFinalizationV3Ledger::open_read(resealed.path(), limits).is_err());

        let reordered = TestRoot::new("reordered");
        create_empty_files(reordered.path());
        let mut file = OpenOptions::new()
            .append(true)
            .open(reordered.path().join(ROW_FILE))
            .expect("open reordered rows");
        file.write_all(&encode_row(&value.rows[1], 1).expect("encode second"))
            .expect("write second first");
        file.write_all(&encode_row(&value.rows[0], 0).expect("encode first"))
            .expect("write first second");
        file.sync_data().expect("sync reordered rows");
        assert_refuses(
            PopulationFinalizationV3Ledger::open_read(reordered.path(), limits),
            "physical sequence",
        );

        let forged_root = TestRoot::new("structural-forgery");
        let forged = altered_prepared(92_000);
        write_block(forged_root.path(), &forged, true);
        let mut reopened = PopulationFinalizationV3Ledger::open_read(forged_root.path(), limits)
            .expect("self-consistent forged block is structural");
        let structural = reopened
            .reopen_structural_receipt(&forged.finalization_id)
            .expect("forged lookup")
            .expect("forged structural receipt");
        assert_refuses(
            reopened.authenticate_structural_receipt(structural, &value),
            "differs from opaque preparation",
        );
    }

    #[test]
    fn stale_same_length_mutation_and_file_or_root_substitution_refuse() {
        let limits = bounds();
        let value = prepared();

        let stale = TestRoot::new("stale");
        persist_population_finalization_v3(stale.path(), limits, &value)
            .expect("seed stale ledger");
        let opened = PopulationFinalizationV3Ledger::open_read(stale.path(), limits)
            .expect("open before stale mutation");
        let path = stale.path().join(ROW_FILE);
        let mut raw = std::fs::read(&path).expect("read stale rows");
        raw[100] ^= 1;
        let mut changed = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&path)
            .expect("open stale writer");
        changed.write_all(&raw).expect("write stale rows");
        changed.sync_data().expect("sync stale rows");
        assert!(
            opened
                .reopen_structural_receipt(&value.finalization_id)
                .is_err()
        );

        let replaced = TestRoot::new("file-replacement");
        persist_population_finalization_v3(replaced.path(), limits, &value)
            .expect("seed replacement ledger");
        let opened = PopulationFinalizationV3Ledger::open_read(replaced.path(), limits)
            .expect("open before file replacement");
        let row_path = replaced.path().join(ROW_FILE);
        let moved = replaced.path().join("population-finalization-v3.moved");
        let exact = std::fs::read(&row_path).expect("read exact replacement bytes");
        std::fs::rename(&row_path, &moved).expect("move admitted row file");
        std::fs::write(&row_path, exact).expect("write same-byte replacement");
        assert!(
            opened
                .reopen_structural_receipt(&value.finalization_id)
                .is_err()
        );

        let root = TestRoot::new("root-substitution");
        persist_population_finalization_v3(root.path(), limits, &value)
            .expect("seed root substitution ledger");
        let opened = PopulationFinalizationV3Ledger::open_read(root.path(), limits)
            .expect("open before root substitution");
        let original = root.path().to_path_buf();
        let moved_root = root.path().with_extension("held-generation");
        std::fs::rename(&original, &moved_root).expect("move admitted root");
        std::fs::create_dir(&original).expect("create replacement root");
        assert!(
            opened
                .reopen_structural_receipt(&value.finalization_id)
                .is_err()
        );
        std::fs::remove_dir(&original).expect("remove replacement root");
        std::fs::rename(&moved_root, &original).expect("restore admitted root");
    }

    #[test]
    fn bulk_projection_refuses_stale_replaced_and_late_corruption_without_partial_rows() {
        let limits = bounds();
        let value = prepared();

        let stale = TestRoot::new("bulk-stale");
        let mut stale_authority = persist_population_finalization_v3(stale.path(), limits, &value)
            .expect("seed bulk stale ledger")
            .into_authority();
        let stale_path = stale.path().join(ROW_FILE);
        let mut stale_raw = std::fs::read(&stale_path).expect("read bulk stale rows");
        stale_raw[100] ^= 1;
        let mut stale_writer = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&stale_path)
            .expect("open bulk stale writer");
        stale_writer
            .write_all(&stale_raw)
            .expect("write bulk stale rows");
        stale_writer.sync_data().expect("sync bulk stale rows");
        assert_refuses(
            stale_authority.ordered_row_projections(),
            "row generation changed after open",
        );

        let replaced = TestRoot::new("bulk-replaced");
        let mut replaced_authority =
            persist_population_finalization_v3(replaced.path(), limits, &value)
                .expect("seed bulk replacement ledger")
                .into_authority();
        let replaced_path = replaced.path().join(ROW_FILE);
        let displaced_path = replaced
            .path()
            .join("population-finalization-v3.bulk-moved");
        let exact = std::fs::read(&replaced_path).expect("read bulk replacement rows");
        std::fs::rename(&replaced_path, &displaced_path).expect("move bulk retained row file");
        std::fs::write(&replaced_path, exact).expect("write same-byte bulk replacement");
        assert_refuses(
            replaced_authority.ordered_row_projections(),
            "path-replaced after open",
        );

        let corrupt = TestRoot::new("bulk-late-corrupt");
        let mut corrupt_authority =
            persist_population_finalization_v3(corrupt.path(), limits, &value)
                .expect("seed bulk corrupt ledger")
                .into_authority();
        let corrupt_path = corrupt.path().join(ROW_FILE);
        let mut corrupt_raw = std::fs::read(&corrupt_path).expect("read bulk corrupt rows");
        let late_offset = 3 * POPULATION_FINALIZATION_V3_ROW_BYTES + 200;
        corrupt_raw[late_offset] ^= 1;
        let mut corrupt_writer = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&corrupt_path)
            .expect("open bulk corrupt writer");
        corrupt_writer
            .write_all(&corrupt_raw)
            .expect("write bulk corrupt rows");
        corrupt_writer.sync_data().expect("sync bulk corrupt rows");
        let refreshed = file_generation(
            &corrupt_authority.ledger.row_file,
            &corrupt_authority.ledger.row_path,
            corrupt_authority.ledger.bounds.max_row_bytes(),
        )
        .expect("admit corrupt fixture generation for decoder attack");
        corrupt_authority.ledger.row_generation = refreshed;
        assert_refuses(
            corrupt_authority.ordered_row_projections(),
            "seal is invalid",
        );
    }

    #[test]
    fn bounded_open_append_and_between_hash_mutation_refuse_before_claim() {
        let value = prepared();
        let root = TestRoot::new("bounded-open");
        write_block(root.path(), &value, true);
        let narrow = PopulationFinalizationV3Bounds::new(
            3,
            4 * POPULATION_FINALIZATION_V3_ROW_BYTES as u64,
            1,
            POPULATION_FINALIZATION_V3_COMPLETION_BYTES as u64,
            3,
        )
        .expect("narrow bounds");
        assert_refuses(
            PopulationFinalizationV3Ledger::open_read(root.path(), narrow),
            "above maximum 3",
        );

        let exact_root = TestRoot::new("bounded-append");
        let exact = PopulationFinalizationV3Bounds::new(
            4,
            4 * POPULATION_FINALIZATION_V3_ROW_BYTES as u64,
            1,
            POPULATION_FINALIZATION_V3_COMPLETION_BYTES as u64,
            4,
        )
        .expect("exact one-block bounds");
        persist_population_finalization_v3(exact_root.path(), exact, &value)
            .expect("first block fits exact bounds");
        let before = std::fs::read(exact_root.path().join(ROW_FILE)).expect("read before refusal");
        assert_refuses(
            persist_population_finalization_v3(exact_root.path(), exact, &altered_prepared(93_000)),
            "above maximum 4",
        );
        assert_eq!(
            std::fs::read(exact_root.path().join(ROW_FILE)).expect("read after refusal"),
            before
        );

        let hash_root = TestRoot::new("double-hash");
        let path = hash_root.path().join("mutation.bin");
        std::fs::write(&path, [7_u8; 64]).expect("write generation fixture");
        let held = File::open(&path).expect("open held generation fixture");
        let result = file_generation_with_between_hash_action(&held, &path, 64, || {
            let mut mutator = OpenOptions::new()
                .write(true)
                .open(&path)
                .map_err(|why| format!("open mutation fixture: {why}"))?;
            mutator
                .seek(SeekFrom::Start(7))
                .and_then(|_| mutator.write_all(&[9]))
                .and_then(|()| mutator.sync_data())
                .map_err(|why| format!("mutate generation fixture: {why}"))
        });
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn preplaced_symlink_roots_and_children_refuse_without_touching_targets() {
        use std::os::unix::fs::symlink;

        let limits = bounds();
        let value = prepared();

        let real_root = TestRoot::new("real-root");
        let linked_root = TestRoot::absent("linked-root");
        symlink(real_root.path(), linked_root.path()).expect("create root symlink");
        assert!(persist_population_finalization_v3(linked_root.path(), limits, &value).is_err());
        assert!(
            std::fs::read_dir(real_root.path())
                .expect("read untouched real root")
                .next()
                .is_none()
        );

        let external = TestRoot::absent("external-target");
        std::fs::write(external.path(), b"untouched").expect("create external target");
        let before = std::fs::read(external.path()).expect("read external target");
        let child_root = TestRoot::new("linked-child");
        symlink(external.path(), child_root.path().join(ROW_FILE))
            .expect("create row-file symlink");
        assert!(persist_population_finalization_v3(child_root.path(), limits, &value).is_err());
        assert_eq!(
            std::fs::read(external.path()).expect("reread external target"),
            before
        );
    }
}
