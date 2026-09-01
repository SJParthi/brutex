//! Receipt-last mixed-terminal Population Finalization V4 authority.
//!
//! This is an independent append-only byte family. It consumes only the
//! retained, freshly reopened Admission V4 capability and never decodes or
//! reinterprets Finalization V3. One authority is a fixed Data record, exact
//! NIFTY then BANKNIFTY Family records, zero or more real evaluated-Candidate
//! Decision records, and an adjacent Completion synchronized last.
//!
//! `InsufficientForCscv` retains its one genuine Candidate lineage but has no
//! invented decision or PBO. `NaturallyExtinct` retains its Observation V2
//! extinction proof and has no Candidate, statistic or decision row. A
//! structural reopen is audit state only: the Population-facing capability is
//! minted only while retaining and reauthenticating the opaque Admission V4
//! source beside a freshly reopened, byte-equal Finalization block.
//!
//! Whole-source authentication, hashing, append, reopen and projection are
//! O(file bytes + Candidates), with filesystem-dependent latency and storage.
//! Only admitted fixed-record offset arithmetic is worst-case O(1) in record
//! count; the in-memory identity index has average O(1) lookup.

#![expect(
    dead_code,
    reason = "the version-separated Finalization V4 authority is the typed Population successor seam and awaits its non-test all-rung caller"
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
use runner::admission::{
    AdmissionEvidenceValuesV1, AdmissionStatusV1, AdmissionV3ArithmeticProjection,
    AdmissionVerdictV1,
};

use crate::population::RequestedSpanIdentityV1;
use crate::population_admission_v4::{
    AdmissionV4DecisionStatus, AdmissionV4Family, AdmissionV4FamilyTerminal,
    PopulationAdmissionV4Authority, PopulationAdmissionV4DecisionProjection,
    PopulationAdmissionV4FamilyProjection, PopulationAdmissionV4FinalizationProjection,
};

/// Bytes before the first Finalization V4 fixed record.
pub(crate) const POPULATION_FINALIZATION_V4_HEADER_BYTES: u64 = 64;
/// Bytes in every Finalization V4 Data, Family, Decision or Completion record.
pub(crate) const POPULATION_FINALIZATION_V4_RECORD_BYTES: u64 = 4_096;

const HEADER_BYTES: usize = 64;
const RECORD_BYTES: usize = 4_096;
const RECORD_BYTES_U32: u32 = 4_096;
const PAYLOAD_BYTES: usize = RECORD_BYTES - 32;
const VERSION: u32 = 4;
const DATA_KIND: u32 = 1;
const FAMILY_KIND: u32 = 2;
const DECISION_KIND: u32 = 3;
const COMPLETION_KIND: u32 = 4;
const HEADER_MAGIC: [u8; 16] = *b"BTX-POPFNL4-HDR\0";
const RECORD_MAGIC: [u8; 16] = *b"BTX-POPFNL4-REC\0";
const DATA_FILE: &str = "population-finalization-v4.bin";
const LOCK_FILE: &str = "population-finalization-v4.lock";
const HEADER_DOMAIN: &[u8] = b"brutex-population-finalization-v4-header\0";
const RECORD_DOMAIN: &[u8] = b"brutex-population-finalization-v4-record\0";
const FINALIZATION_DOMAIN: &[u8] = b"brutex-population-finalization-v4-id\0";
const FAMILY_ROW_DOMAIN: &[u8] = b"brutex-population-finalization-v4-family-row\0";
const DECISION_ROW_DOMAIN: &[u8] = b"brutex-population-finalization-v4-decision-row\0";
const ORDERED_FAMILIES_DOMAIN: &[u8] = b"brutex-population-finalization-v4-families\0";
const ORDERED_DECISIONS_DOMAIN: &[u8] = b"brutex-population-finalization-v4-decisions\0";
const COMPLETION_DOMAIN: &[u8] = b"brutex-population-finalization-v4-completion\0";
const POLICY_DOMAIN: &[u8] = b"brutex-population-admission-v4-policy\0";
const GENERATION_DOMAIN: &[u8] = b"brutex-population-finalization-v4-generation\0";
const RUNNER_POLICY_BYTES: usize = runner::admission::ADMISSION_POLICY_CANONICAL_LEN_V1;
const RUNNER_DECISION_BYTES: usize = runner::admission::ADMISSION_DECISION_CANONICAL_LEN_V3;
const RUNNER_HEADER_BYTES: usize = 12;
const RUNNER_POLICY_END: usize = RUNNER_HEADER_BYTES + RUNNER_POLICY_BYTES;
const READ_CHUNK_BYTES: usize = 16 * 1_024;

#[cfg(any(target_os = "android", target_os = "linux"))]
const O_NOFOLLOW_FLAG: i32 = 0x20_000;
#[cfg(target_os = "macos")]
const O_NOFOLLOW_FLAG: i32 = 0x100;

const _: () = assert!(PAYLOAD_BYTES + 32 == RECORD_BYTES);

/// Operator-facing fail-closed refusal.
pub(crate) type PopulationFinalizationV4Refusal = String;

/// Explicit nonzero authority, decision and file ceilings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationFinalizationV4Bounds {
    authorities: u64,
    decisions_per_authority: u64,
    file_bytes: u64,
}

impl PopulationFinalizationV4Bounds {
    /// Creates explicit bounds; there is intentionally no `Default`.
    ///
    /// # Errors
    ///
    /// Refuses zero authority/decision ceilings and a byte ceiling unable to
    /// hold the smallest Data/two-Family/Completion block.
    pub(crate) fn new(
        max_authorities: u64,
        max_decisions_per_authority: u64,
        max_file_bytes: u64,
    ) -> Result<Self, PopulationFinalizationV4Refusal> {
        let minimum = POPULATION_FINALIZATION_V4_HEADER_BYTES
            .checked_add(4 * POPULATION_FINALIZATION_V4_RECORD_BYTES)
            .ok_or_else(|| "Finalization V4 minimum byte bound overflowed".to_owned())?;
        if max_authorities == 0 || max_decisions_per_authority == 0 || max_file_bytes < minimum {
            return Err(format!(
                "Finalization V4 bounds require nonzero authorities/decisions and at least {minimum} bytes"
            ));
        }
        Ok(Self {
            authorities: max_authorities,
            decisions_per_authority: max_decisions_per_authority,
            file_bytes: max_file_bytes,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FinalizationSourceV4 {
    admission_sequence: u64,
    admission_first_record: u64,
    admission_block_id: [u8; 32],
    admission_completion_id: [u8; 32],
    rung_seconds: u32,
    horizon_bars: u32,
    requested_span: RequestedSpanIdentityV1,
    feed_digest: [u8; 32],
    source_commit_digest: [u8; 32],
    calendar_policy_digest: [u8; 32],
    daily_reference_policy_digest: [u8; 32],
    vocabulary_digest: [u8; 32],
    evaluation_policy_digest: [u8; 32],
    statistics_authority_id: [u8; 32],
    statistics_completion_digest: [u8; 32],
    statistics_policy_digest: [u8; 32],
    base_pair_id: [u8; 32],
    search_pair_id: [u8; 32],
    policy: [u8; RUNNER_POLICY_BYTES],
    policy_digest: [u8; 32],
    nifty_admission_family_id: [u8; 32],
    banknifty_admission_family_id: [u8; 32],
    nifty_terminal: AdmissionV4FamilyTerminal,
    banknifty_terminal: AdmissionV4FamilyTerminal,
    nifty_candidate_count: u64,
    banknifty_candidate_count: u64,
    candidate_count: u64,
    decision_count: u64,
    finalization_id: [u8; 32],
}

impl FinalizationSourceV4 {
    fn from_projection(value: &PopulationAdmissionV4FinalizationProjection) -> Self {
        let receipt = value.receipt();
        let source = value.source();
        let [nifty, banknifty] = value.families();
        Self {
            admission_sequence: receipt.sequence(),
            admission_first_record: receipt.first_record(),
            admission_block_id: receipt.block_id(),
            admission_completion_id: receipt.completion_id(),
            rung_seconds: source.rung_seconds(),
            horizon_bars: source.horizon_bars(),
            requested_span: source.requested_span(),
            feed_digest: source.feed_digest(),
            source_commit_digest: source.source_commit_digest(),
            calendar_policy_digest: source.calendar_policy_digest(),
            daily_reference_policy_digest: source.daily_reference_policy_digest(),
            vocabulary_digest: source.vocabulary_digest(),
            evaluation_policy_digest: source.evaluation_policy_digest(),
            statistics_authority_id: source.statistics_authority_id(),
            statistics_completion_digest: source.statistics_completion_digest(),
            statistics_policy_digest: source.statistics_policy_digest(),
            base_pair_id: source.base_pair_id(),
            search_pair_id: source.search_pair_id(),
            policy: *source.policy(),
            policy_digest: source.policy_digest(),
            nifty_admission_family_id: nifty.family_id(),
            banknifty_admission_family_id: banknifty.family_id(),
            nifty_terminal: nifty.terminal(),
            banknifty_terminal: banknifty.terminal(),
            nifty_candidate_count: nifty.candidate_count(),
            banknifty_candidate_count: banknifty.candidate_count(),
            candidate_count: source.candidate_count(),
            decision_count: source.decision_count(),
            finalization_id: [0; 32],
        }
    }

    fn validate_common(&self) -> Result<(), PopulationFinalizationV4Refusal> {
        if self.rung_seconds == 0 || self.horizon_bars == 0 {
            return Err("Finalization V4 rung/horizon is zero".to_owned());
        }
        RequestedSpanIdentityV1::new(
            self.requested_span.from_year(),
            self.requested_span.from_month(),
            self.requested_span.to_year(),
            self.requested_span.to_month(),
        )?;
        for (name, value) in [
            ("Admission block", self.admission_block_id),
            ("Admission Completion", self.admission_completion_id),
            ("feed", self.feed_digest),
            ("source commit", self.source_commit_digest),
            ("calendar policy", self.calendar_policy_digest),
            ("daily-reference policy", self.daily_reference_policy_digest),
            ("vocabulary", self.vocabulary_digest),
            ("evaluation policy", self.evaluation_policy_digest),
            ("Statistics authority", self.statistics_authority_id),
            ("Statistics Completion", self.statistics_completion_digest),
            ("Statistics policy", self.statistics_policy_digest),
            ("Base pair", self.base_pair_id),
            ("Search pair", self.search_pair_id),
            ("Admission policy", self.policy_digest),
            ("NIFTY Admission family", self.nifty_admission_family_id),
            (
                "BANKNIFTY Admission family",
                self.banknifty_admission_family_id,
            ),
        ] {
            require_nonzero(name, value)?;
        }
        if self.policy_digest != hash_slices(POLICY_DOMAIN, &[&self.policy]) {
            return Err("Finalization V4 Admission policy digest does not reproduce".to_owned());
        }
        if self.nifty_admission_family_id == self.banknifty_admission_family_id {
            return Err("Finalization V4 Admission family identities alias".to_owned());
        }
        let candidate_count = self
            .nifty_candidate_count
            .checked_add(self.banknifty_candidate_count)
            .ok_or_else(|| "Finalization V4 Candidate count overflowed".to_owned())?;
        if candidate_count != self.candidate_count {
            return Err("Finalization V4 Candidate count does not reconcile".to_owned());
        }
        Ok(())
    }

    fn encode_body(
        &self,
        writer: &mut FixedWriter<'_>,
    ) -> Result<(), PopulationFinalizationV4Refusal> {
        writer.u64(self.admission_sequence)?;
        writer.u64(self.admission_first_record)?;
        writer.array(&self.admission_block_id)?;
        writer.array(&self.admission_completion_id)?;
        writer.u32(self.rung_seconds)?;
        writer.u32(self.horizon_bars)?;
        writer.u16(self.requested_span.from_year())?;
        writer.u8(self.requested_span.from_month())?;
        writer.u16(self.requested_span.to_year())?;
        writer.u8(self.requested_span.to_month())?;
        writer.zeros(6)?;
        for value in [
            self.feed_digest,
            self.source_commit_digest,
            self.calendar_policy_digest,
            self.daily_reference_policy_digest,
            self.vocabulary_digest,
            self.evaluation_policy_digest,
            self.statistics_authority_id,
            self.statistics_completion_digest,
            self.statistics_policy_digest,
            self.base_pair_id,
            self.search_pair_id,
        ] {
            writer.array(&value)?;
        }
        writer.array(&self.policy)?;
        writer.array(&self.policy_digest)?;
        writer.array(&self.nifty_admission_family_id)?;
        writer.array(&self.banknifty_admission_family_id)?;
        writer.u8(self.nifty_terminal as u8)?;
        writer.u8(self.banknifty_terminal as u8)?;
        writer.zeros(6)?;
        writer.u64(self.nifty_candidate_count)?;
        writer.u64(self.banknifty_candidate_count)?;
        writer.u64(self.candidate_count)?;
        writer.u64(self.decision_count)?;
        writer.array(&self.finalization_id)
    }

    fn decode_body(reader: &mut FixedReader<'_>) -> Result<Self, PopulationFinalizationV4Refusal> {
        let admission_sequence = reader.u64()?;
        let admission_first_record = reader.u64()?;
        let admission_block_id = reader.array()?;
        let admission_completion_id = reader.array()?;
        let rung_seconds = reader.u32()?;
        let horizon_bars = reader.u32()?;
        let from_year = reader.u16()?;
        let from_month = reader.u8()?;
        let to_year = reader.u16()?;
        let to_month = reader.u8()?;
        reader.require_zero(6, "source alignment")?;
        let requested_span =
            RequestedSpanIdentityV1::new(from_year, from_month, to_year, to_month)?;
        let value = Self {
            admission_sequence,
            admission_first_record,
            admission_block_id,
            admission_completion_id,
            rung_seconds,
            horizon_bars,
            requested_span,
            feed_digest: reader.array()?,
            source_commit_digest: reader.array()?,
            calendar_policy_digest: reader.array()?,
            daily_reference_policy_digest: reader.array()?,
            vocabulary_digest: reader.array()?,
            evaluation_policy_digest: reader.array()?,
            statistics_authority_id: reader.array()?,
            statistics_completion_digest: reader.array()?,
            statistics_policy_digest: reader.array()?,
            base_pair_id: reader.array()?,
            search_pair_id: reader.array()?,
            policy: reader.array()?,
            policy_digest: reader.array()?,
            nifty_admission_family_id: reader.array()?,
            banknifty_admission_family_id: reader.array()?,
            nifty_terminal: decode_terminal(reader.u8()?)?,
            banknifty_terminal: decode_terminal(reader.u8()?)?,
            nifty_candidate_count: {
                reader.require_zero(6, "terminal alignment")?;
                reader.u64()?
            },
            banknifty_candidate_count: reader.u64()?,
            candidate_count: reader.u64()?,
            decision_count: reader.u64()?,
            finalization_id: reader.array()?,
        };
        value.validate_common()?;
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FamilyRecordV4 {
    finalization_id: [u8; 32],
    family_sequence: u64,
    family: AdmissionV4Family,
    terminal: AdmissionV4FamilyTerminal,
    admission_family_id: [u8; 32],
    statistics_family_id: [u8; 32],
    candidate_universe_id: [u8; 32],
    candidate_completion_digest: [u8; 32],
    candidate_ordered_row_digest: [u8; 32],
    pre_admission_authority_id: [u8; 32],
    observation_authority_id: [u8; 32],
    observation_identity: [u8; 32],
    observation_policy_digest: [u8; 32],
    observation_data_digest: [u8; 32],
    observation_completion_digest: [u8; 32],
    base_completion_id: [u8; 32],
    signal_digest: [u8; 32],
    signal_bars: u64,
    signal_first_ts_micros: i64,
    signal_last_ts_micros: i64,
    signal_column_digest: [u8; 32],
    long_grid_policy_digest: [u8; 32],
    long_grid_resolution_digest: [u8; 32],
    short_grid_policy_digest: [u8; 32],
    short_grid_resolution_digest: [u8; 32],
    grid_composite_digest: [u8; 32],
    search_member_id: [u8; 32],
    search_source_authority_id: [u8; 32],
    search_validation_policy_id: [u8; 32],
    search_validation_family_id: [u8; 32],
    search_walk_id: [u8; 32],
    search_fold_count: u64,
    search_decided_folds: u64,
    search_profitable_oos_folds: u64,
    search_aggregate_oos_paisa: i64,
    search_evaluated_population_cells: u64,
    statistics_candidate_digest: [u8; 32],
    statistics_period_digest: [u8; 32],
    statistics_split_digest: [u8; 32],
    candidate_count: u64,
    decision_count: u64,
    row_id: [u8; 32],
}

impl FamilyRecordV4 {
    fn from_projection(
        finalization_id: [u8; 32],
        family_sequence: u64,
        value: &PopulationAdmissionV4FamilyProjection,
    ) -> Self {
        Self {
            finalization_id,
            family_sequence,
            family: value.family(),
            terminal: value.terminal(),
            admission_family_id: value.family_id(),
            statistics_family_id: value.statistics_family_id(),
            candidate_universe_id: value.candidate_universe_id(),
            candidate_completion_digest: value.candidate_completion_digest(),
            candidate_ordered_row_digest: value.candidate_ordered_row_digest(),
            pre_admission_authority_id: value.pre_admission_authority_id(),
            observation_authority_id: value.observation_authority_id(),
            observation_identity: value.observation_identity(),
            observation_policy_digest: value.observation_policy_digest(),
            observation_data_digest: value.observation_data_digest(),
            observation_completion_digest: value.observation_completion_digest(),
            base_completion_id: value.base_completion_id(),
            signal_digest: value.signal_digest(),
            signal_bars: value.signal_bars(),
            signal_first_ts_micros: value.signal_first_ts_micros(),
            signal_last_ts_micros: value.signal_last_ts_micros(),
            signal_column_digest: value.signal_column_digest(),
            long_grid_policy_digest: value.long_grid_policy_digest(),
            long_grid_resolution_digest: value.long_grid_resolution_digest(),
            short_grid_policy_digest: value.short_grid_policy_digest(),
            short_grid_resolution_digest: value.short_grid_resolution_digest(),
            grid_composite_digest: value.grid_composite_digest(),
            search_member_id: value.search_member_id(),
            search_source_authority_id: value.search_source_authority_id(),
            search_validation_policy_id: value.search_validation_policy_id(),
            search_validation_family_id: value.search_validation_family_id(),
            search_walk_id: value.search_walk_id(),
            search_fold_count: value.search_fold_count(),
            search_decided_folds: value.search_decided_folds(),
            search_profitable_oos_folds: value.search_profitable_oos_folds(),
            search_aggregate_oos_paisa: value.search_aggregate_oos_paisa(),
            search_evaluated_population_cells: value.search_evaluated_population_cells(),
            statistics_candidate_digest: value.statistics_candidate_digest(),
            statistics_period_digest: value.statistics_period_digest(),
            statistics_split_digest: value.statistics_split_digest(),
            candidate_count: value.candidate_count(),
            decision_count: value.decision_count(),
            row_id: [0; 32],
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one family validator keeps exact source lineage, terminal shape, Search hierarchy and row identity adjacent"
    )]
    fn validate(
        &self,
        source: &FinalizationSourceV4,
    ) -> Result<(), PopulationFinalizationV4Refusal> {
        let (
            expected_sequence,
            expected_family,
            expected_terminal,
            expected_admission_family,
            expected_candidates,
        ) = if self.family_sequence == 0 {
            (
                0,
                AdmissionV4Family::Nifty,
                source.nifty_terminal,
                source.nifty_admission_family_id,
                source.nifty_candidate_count,
            )
        } else {
            (
                1,
                AdmissionV4Family::BankNifty,
                source.banknifty_terminal,
                source.banknifty_admission_family_id,
                source.banknifty_candidate_count,
            )
        };
        if self.family_sequence != expected_sequence
            || self.family != expected_family
            || self.terminal != expected_terminal
            || self.admission_family_id != expected_admission_family
            || self.candidate_count != expected_candidates
            || self.finalization_id != source.finalization_id
        {
            return Err("Finalization V4 family crosswires its block/order".to_owned());
        }
        for (name, value) in [
            ("Admission family", self.admission_family_id),
            ("Statistics family", self.statistics_family_id),
            ("Candidate universe", self.candidate_universe_id),
            ("Candidate Completion", self.candidate_completion_digest),
            ("Candidate ordered rows", self.candidate_ordered_row_digest),
            ("Pre-Admission authority", self.pre_admission_authority_id),
            ("Observation identity", self.observation_identity),
            ("Observation policy", self.observation_policy_digest),
            ("Base Completion", self.base_completion_id),
            ("signal", self.signal_digest),
            ("signal column", self.signal_column_digest),
            ("Long grid policy", self.long_grid_policy_digest),
            ("Long grid resolution", self.long_grid_resolution_digest),
            ("Short grid policy", self.short_grid_policy_digest),
            ("Short grid resolution", self.short_grid_resolution_digest),
            ("grid composite", self.grid_composite_digest),
            ("Search member", self.search_member_id),
            ("Search source authority", self.search_source_authority_id),
            ("Search validation policy", self.search_validation_policy_id),
            ("Search validation family", self.search_validation_family_id),
            ("Search walk", self.search_walk_id),
        ] {
            require_nonzero(name, value)?;
        }
        if self.signal_bars == 0
            || self.signal_first_ts_micros > self.signal_last_ts_micros
            || self.long_grid_policy_digest == self.short_grid_policy_digest
            || self.long_grid_resolution_digest == self.short_grid_resolution_digest
            || self.search_fold_count == 0
            || self.search_decided_folds > self.search_fold_count
            || self.search_profitable_oos_folds > self.search_decided_folds
        {
            return Err("Finalization V4 family signal/grid/Search hierarchy differs".to_owned());
        }
        if (self.search_decided_folds == 0 && self.search_aggregate_oos_paisa != 0)
            || (self.search_profitable_oos_folds == 0 && self.search_aggregate_oos_paisa > 0)
            || (self.search_profitable_oos_folds == self.search_decided_folds
                && self.search_decided_folds > 0
                && self.search_aggregate_oos_paisa <= 0)
        {
            return Err("Finalization V4 family Search outcome hierarchy differs".to_owned());
        }
        match self.terminal {
            AdmissionV4FamilyTerminal::Evaluated => {
                if self.candidate_count < 2
                    || self.decision_count != self.candidate_count
                    || self.statistics_candidate_digest == [0; 32]
                    || self.statistics_period_digest == [0; 32]
                    || self.statistics_split_digest == [0; 32]
                    || self.observation_authority_id != [0; 32]
                    || self.observation_data_digest != [0; 32]
                    || self.observation_completion_digest != [0; 32]
                {
                    return Err("Finalization V4 evaluated family shape differs".to_owned());
                }
            }
            AdmissionV4FamilyTerminal::InsufficientForCscv => {
                if self.candidate_count != 1
                    || self.decision_count != 0
                    || self.statistics_candidate_digest == [0; 32]
                    || self.statistics_period_digest == [0; 32]
                    || self.statistics_split_digest == [0; 32]
                    || self.observation_authority_id != [0; 32]
                    || self.observation_data_digest != [0; 32]
                    || self.observation_completion_digest != [0; 32]
                {
                    return Err("Finalization V4 insufficient family shape differs".to_owned());
                }
            }
            AdmissionV4FamilyTerminal::NaturallyExtinct => {
                if self.candidate_count != 0
                    || self.decision_count != 0
                    || self.statistics_candidate_digest != [0; 32]
                    || self.statistics_period_digest != [0; 32]
                    || self.statistics_split_digest != [0; 32]
                    || self.observation_authority_id == [0; 32]
                    || self.observation_identity != self.observation_authority_id
                    || self.observation_data_digest == [0; 32]
                    || self.observation_completion_digest == [0; 32]
                {
                    return Err(
                        "Finalization V4 extinct family invents rows or loses its proof".to_owned(),
                    );
                }
            }
        }
        if self.row_id != derive_family_row_id(self)? {
            return Err("Finalization V4 family row identity does not reproduce".to_owned());
        }
        Ok(())
    }

    fn encode_body(
        &self,
        writer: &mut FixedWriter<'_>,
    ) -> Result<(), PopulationFinalizationV4Refusal> {
        writer.array(&self.finalization_id)?;
        writer.u64(self.family_sequence)?;
        writer.u8(self.family as u8)?;
        writer.u8(self.terminal as u8)?;
        writer.zeros(6)?;
        for value in [
            self.admission_family_id,
            self.statistics_family_id,
            self.candidate_universe_id,
            self.candidate_completion_digest,
            self.candidate_ordered_row_digest,
            self.pre_admission_authority_id,
            self.observation_authority_id,
            self.observation_identity,
            self.observation_policy_digest,
            self.observation_data_digest,
            self.observation_completion_digest,
            self.base_completion_id,
            self.signal_digest,
        ] {
            writer.array(&value)?;
        }
        writer.u64(self.signal_bars)?;
        writer.i64(self.signal_first_ts_micros)?;
        writer.i64(self.signal_last_ts_micros)?;
        for value in [
            self.signal_column_digest,
            self.long_grid_policy_digest,
            self.long_grid_resolution_digest,
            self.short_grid_policy_digest,
            self.short_grid_resolution_digest,
            self.grid_composite_digest,
            self.search_member_id,
            self.search_source_authority_id,
            self.search_validation_policy_id,
            self.search_validation_family_id,
            self.search_walk_id,
        ] {
            writer.array(&value)?;
        }
        writer.u64(self.search_fold_count)?;
        writer.u64(self.search_decided_folds)?;
        writer.u64(self.search_profitable_oos_folds)?;
        writer.i64(self.search_aggregate_oos_paisa)?;
        writer.u64(self.search_evaluated_population_cells)?;
        for value in [
            self.statistics_candidate_digest,
            self.statistics_period_digest,
            self.statistics_split_digest,
        ] {
            writer.array(&value)?;
        }
        writer.u64(self.candidate_count)?;
        writer.u64(self.decision_count)?;
        writer.array(&self.row_id)
    }

    fn decode_body(reader: &mut FixedReader<'_>) -> Result<Self, PopulationFinalizationV4Refusal> {
        let finalization_id = reader.array()?;
        let family_sequence = reader.u64()?;
        let family = decode_family(reader.u8()?)?;
        let terminal = decode_terminal(reader.u8()?)?;
        reader.require_zero(6, "family alignment")?;
        Ok(Self {
            finalization_id,
            family_sequence,
            family,
            terminal,
            admission_family_id: reader.array()?,
            statistics_family_id: reader.array()?,
            candidate_universe_id: reader.array()?,
            candidate_completion_digest: reader.array()?,
            candidate_ordered_row_digest: reader.array()?,
            pre_admission_authority_id: reader.array()?,
            observation_authority_id: reader.array()?,
            observation_identity: reader.array()?,
            observation_policy_digest: reader.array()?,
            observation_data_digest: reader.array()?,
            observation_completion_digest: reader.array()?,
            base_completion_id: reader.array()?,
            signal_digest: reader.array()?,
            signal_bars: reader.u64()?,
            signal_first_ts_micros: reader.i64()?,
            signal_last_ts_micros: reader.i64()?,
            signal_column_digest: reader.array()?,
            long_grid_policy_digest: reader.array()?,
            long_grid_resolution_digest: reader.array()?,
            short_grid_policy_digest: reader.array()?,
            short_grid_resolution_digest: reader.array()?,
            grid_composite_digest: reader.array()?,
            search_member_id: reader.array()?,
            search_source_authority_id: reader.array()?,
            search_validation_policy_id: reader.array()?,
            search_validation_family_id: reader.array()?,
            search_walk_id: reader.array()?,
            search_fold_count: reader.u64()?,
            search_decided_folds: reader.u64()?,
            search_profitable_oos_folds: reader.u64()?,
            search_aggregate_oos_paisa: reader.i64()?,
            search_evaluated_population_cells: reader.u64()?,
            statistics_candidate_digest: reader.array()?,
            statistics_period_digest: reader.array()?,
            statistics_split_digest: reader.array()?,
            candidate_count: reader.u64()?,
            decision_count: reader.u64()?,
            row_id: reader.array()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DecisionRecordV4 {
    finalization_id: [u8; 32],
    decision_sequence: u64,
    statistics_sequence: u64,
    family: AdmissionV4Family,
    family_sequence: u64,
    status: AdmissionV4DecisionStatus,
    admission_decision_id: [u8; 32],
    candidate_semantic_id: [u8; 32],
    candidate_row_digest: [u8; 32],
    pre_admission_authority_id: [u8; 32],
    statistics_period_digest: [u8; 32],
    statistics_split_digest: [u8; 32],
    search_member_id: [u8; 32],
    base_evidence_id: [u8; 32],
    runner_decision: [u8; RUNNER_DECISION_BYTES],
    row_id: [u8; 32],
}

impl DecisionRecordV4 {
    fn from_projection(
        finalization_id: [u8; 32],
        value: &PopulationAdmissionV4DecisionProjection,
    ) -> Self {
        Self {
            finalization_id,
            decision_sequence: value.decision_sequence(),
            statistics_sequence: value.statistics_sequence(),
            family: value.family(),
            family_sequence: value.family_sequence(),
            status: value.status(),
            admission_decision_id: value.decision_id(),
            candidate_semantic_id: value.candidate_semantic_id(),
            candidate_row_digest: value.candidate_row_digest(),
            pre_admission_authority_id: value.pre_admission_authority_id(),
            statistics_period_digest: value.statistics_period_digest(),
            statistics_split_digest: value.statistics_split_digest(),
            search_member_id: value.search_member_id(),
            base_evidence_id: value.base_evidence_id(),
            runner_decision: *value.runner_decision(),
            row_id: [0; 32],
        }
    }

    fn validate(
        &self,
        source: &FinalizationSourceV4,
        families: &[FamilyRecordV4; 2],
    ) -> Result<(AdmissionEvidenceValuesV1, AdmissionVerdictV1), PopulationFinalizationV4Refusal>
    {
        if self.finalization_id != source.finalization_id {
            return Err("Finalization V4 decision crosswires its block".to_owned());
        }
        for (name, value) in [
            ("Admission decision", self.admission_decision_id),
            ("Candidate semantic", self.candidate_semantic_id),
            ("Candidate row", self.candidate_row_digest),
            ("Pre-Admission authority", self.pre_admission_authority_id),
            ("Statistics period", self.statistics_period_digest),
            ("Statistics split", self.statistics_split_digest),
            ("Search member", self.search_member_id),
            ("Base Evidence", self.base_evidence_id),
            ("Finalization decision row", self.row_id),
        ] {
            require_nonzero(name, value)?;
        }
        let family_index = match self.family {
            AdmissionV4Family::Nifty => 0,
            AdmissionV4Family::BankNifty => 1,
        };
        let family = families
            .get(family_index)
            .ok_or_else(|| "Finalization V4 decision family is absent".to_owned())?;
        let statistics_offset = if self.family == AdmissionV4Family::Nifty {
            0
        } else {
            source.nifty_candidate_count
        };
        if family.terminal != AdmissionV4FamilyTerminal::Evaluated
            || self.family_sequence >= family.decision_count
            || self.statistics_sequence != statistics_offset + self.family_sequence
            || self.pre_admission_authority_id != family.pre_admission_authority_id
            || self.search_member_id != family.search_member_id
        {
            return Err("Finalization V4 decision crosswires family lineage/order".to_owned());
        }
        let policy = self
            .runner_decision
            .get(RUNNER_HEADER_BYTES..RUNNER_POLICY_END)
            .ok_or_else(|| "Finalization V4 Runner policy bytes are absent".to_owned())?;
        if policy != source.policy {
            return Err("Finalization V4 Runner policy differs from Admission block".to_owned());
        }
        let arithmetic =
            AdmissionV3ArithmeticProjection::verify_decision_record_detached(&self.runner_decision)
                .map_err(|why| format!("Finalization V4 Runner arithmetic refused: {why:?}"))?;
        if self.status != status_from_runner(arithmetic.status()) {
            return Err("Finalization V4 decision status differs from Runner bytes".to_owned());
        }
        if self.row_id != derive_decision_row_id(self)? {
            return Err("Finalization V4 decision row identity does not reproduce".to_owned());
        }
        Ok((arithmetic.comparison_values(), arithmetic.verdict()))
    }

    fn encode_body(
        &self,
        writer: &mut FixedWriter<'_>,
    ) -> Result<(), PopulationFinalizationV4Refusal> {
        writer.array(&self.finalization_id)?;
        writer.u64(self.decision_sequence)?;
        writer.u64(self.statistics_sequence)?;
        writer.u8(self.family as u8)?;
        writer.u8(self.status as u8)?;
        writer.zeros(6)?;
        writer.u64(self.family_sequence)?;
        for value in [
            self.admission_decision_id,
            self.candidate_semantic_id,
            self.candidate_row_digest,
            self.pre_admission_authority_id,
            self.statistics_period_digest,
            self.statistics_split_digest,
            self.search_member_id,
            self.base_evidence_id,
        ] {
            writer.array(&value)?;
        }
        writer.array(&self.runner_decision)?;
        writer.array(&self.row_id)
    }

    fn decode_body(reader: &mut FixedReader<'_>) -> Result<Self, PopulationFinalizationV4Refusal> {
        let finalization_id = reader.array()?;
        let decision_sequence = reader.u64()?;
        let statistics_sequence = reader.u64()?;
        let family = decode_family(reader.u8()?)?;
        let status = decode_status(reader.u8()?)?;
        reader.require_zero(6, "decision alignment")?;
        Ok(Self {
            finalization_id,
            decision_sequence,
            statistics_sequence,
            family,
            status,
            family_sequence: reader.u64()?,
            admission_decision_id: reader.array()?,
            candidate_semantic_id: reader.array()?,
            candidate_row_digest: reader.array()?,
            pre_admission_authority_id: reader.array()?,
            statistics_period_digest: reader.array()?,
            statistics_split_digest: reader.array()?,
            search_member_id: reader.array()?,
            base_evidence_id: reader.array()?,
            runner_decision: reader.array()?,
            row_id: reader.array()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedPopulationFinalizationV4 {
    source: FinalizationSourceV4,
    families: [FamilyRecordV4; 2],
    decisions: Vec<DecisionRecordV4>,
}

impl PreparedPopulationFinalizationV4 {
    fn from_projection(
        value: &PopulationAdmissionV4FinalizationProjection,
    ) -> Result<Self, PopulationFinalizationV4Refusal> {
        let mut source = FinalizationSourceV4::from_projection(value);
        source.finalization_id =
            derive_finalization_id(&source, value.families(), value.decisions())?;
        let [nifty, banknifty] = value.families();
        let mut families = [
            FamilyRecordV4::from_projection(source.finalization_id, 0, nifty),
            FamilyRecordV4::from_projection(source.finalization_id, 1, banknifty),
        ];
        for family in &mut families {
            family.row_id = derive_family_row_id(family)?;
        }
        let mut decisions = Vec::new();
        decisions
            .try_reserve_exact(value.decisions().len())
            .map_err(|why| format!("cannot reserve Finalization V4 decisions: {why}"))?;
        for projected in value.decisions() {
            let mut decision = DecisionRecordV4::from_projection(source.finalization_id, projected);
            decision.row_id = derive_decision_row_id(&decision)?;
            decisions.push(decision);
        }
        let prepared = Self {
            source,
            families,
            decisions,
        };
        prepared.validate()?;
        Ok(prepared)
    }

    fn validate(&self) -> Result<(), PopulationFinalizationV4Refusal> {
        self.source.validate_common()?;
        if self.source.finalization_id == [0; 32] {
            return Err("Finalization V4 identity is zero".to_owned());
        }
        for family in &self.families {
            family.validate(&self.source)?;
        }
        let decision_count = u64::try_from(self.decisions.len())
            .map_err(|_| "Finalization V4 decision count does not fit u64".to_owned())?;
        if decision_count != self.source.decision_count
            || self.families[0].decision_count + self.families[1].decision_count != decision_count
        {
            return Err("Finalization V4 decision counts do not reconcile".to_owned());
        }
        let mut admission_ids = bounded_set(self.decisions.len(), "Admission decisions")?;
        let mut candidate_ids = bounded_set(self.decisions.len(), "Candidate semantics")?;
        let mut candidate_rows = bounded_set(self.decisions.len(), "Candidate rows")?;
        let mut base_rows = bounded_set(self.decisions.len(), "Base Evidence rows")?;
        let mut row_ids = bounded_set(self.decisions.len(), "Finalization decision rows")?;
        let mut next_nifty = 0_u64;
        let mut next_banknifty = 0_u64;
        let mut banknifty_started = false;
        for (index, decision) in self.decisions.iter().enumerate() {
            let expected_sequence = u64::try_from(index)
                .map_err(|_| "Finalization V4 decision ordinal does not fit u64".to_owned())?;
            if decision.decision_sequence != expected_sequence {
                return Err("Finalization V4 decision sequence is not canonical".to_owned());
            }
            let expected_family_sequence = match decision.family {
                AdmissionV4Family::Nifty if !banknifty_started => &mut next_nifty,
                AdmissionV4Family::BankNifty => {
                    banknifty_started = true;
                    &mut next_banknifty
                }
                AdmissionV4Family::Nifty => {
                    return Err("Finalization V4 NIFTY decision follows BANKNIFTY".to_owned());
                }
            };
            if decision.family_sequence != *expected_family_sequence {
                return Err("Finalization V4 family decision sequence is not canonical".to_owned());
            }
            *expected_family_sequence = expected_family_sequence
                .checked_add(1)
                .ok_or_else(|| "Finalization V4 family sequence overflowed".to_owned())?;
            decision.validate(&self.source, &self.families)?;
            for (name, inserted) in [
                (
                    "Admission decision",
                    admission_ids.insert(decision.admission_decision_id),
                ),
                (
                    "Candidate semantic",
                    candidate_ids.insert(decision.candidate_semantic_id),
                ),
                (
                    "Candidate row",
                    candidate_rows.insert(decision.candidate_row_digest),
                ),
                (
                    "Base Evidence row",
                    base_rows.insert(decision.base_evidence_id),
                ),
                ("Finalization decision row", row_ids.insert(decision.row_id)),
            ] {
                if !inserted {
                    return Err(format!("Finalization V4 duplicates {name}"));
                }
            }
        }
        if next_nifty != self.families[0].decision_count
            || next_banknifty != self.families[1].decision_count
        {
            return Err("Finalization V4 per-family decision counts differ".to_owned());
        }
        if self.source.finalization_id
            != derive_finalization_id_from_records(&self.source, &self.families, &self.decisions)?
        {
            return Err("Finalization V4 semantic identity does not reproduce".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CompletionV4 {
    sequence: u64,
    source: FinalizationSourceV4,
    ordered_family_digest: [u8; 32],
    ordered_decision_digest: [u8; 32],
    admitted_count: u64,
    rejected_count: u64,
    unmeasured_count: u64,
    refused_count: u64,
    completion_id: [u8; 32],
}

impl CompletionV4 {
    fn from_prepared(
        sequence: u64,
        value: &PreparedPopulationFinalizationV4,
    ) -> Result<Self, PopulationFinalizationV4Refusal> {
        value.validate()?;
        let mut admitted_count = 0_u64;
        let mut rejected_count = 0_u64;
        let mut unmeasured_count = 0_u64;
        let mut refused_count = 0_u64;
        for decision in &value.decisions {
            let counter = match decision.status {
                AdmissionV4DecisionStatus::Admitted => &mut admitted_count,
                AdmissionV4DecisionStatus::Rejected => &mut rejected_count,
                AdmissionV4DecisionStatus::Unmeasured => &mut unmeasured_count,
                AdmissionV4DecisionStatus::Refused => &mut refused_count,
            };
            *counter = counter
                .checked_add(1)
                .ok_or_else(|| "Finalization V4 status count overflowed".to_owned())?;
        }
        let mut completion = Self {
            sequence,
            source: value.source.clone(),
            ordered_family_digest: ordered_family_digest(&value.families),
            ordered_decision_digest: ordered_decision_digest(&value.decisions)?,
            admitted_count,
            rejected_count,
            unmeasured_count,
            refused_count,
            completion_id: [0; 32],
        };
        completion.completion_id = derive_completion_id(&completion);
        completion.validate()?;
        Ok(completion)
    }

    fn validate(&self) -> Result<(), PopulationFinalizationV4Refusal> {
        self.source.validate_common()?;
        require_nonzero("ordered family digest", self.ordered_family_digest)?;
        require_nonzero("ordered decision digest", self.ordered_decision_digest)?;
        require_nonzero("Finalization Completion", self.completion_id)?;
        let status_count = checked_sum(
            &[
                self.admitted_count,
                self.rejected_count,
                self.unmeasured_count,
                self.refused_count,
            ],
            "Finalization V4 status counts",
        )?;
        if status_count != self.source.decision_count {
            return Err("Finalization V4 Completion status counts differ".to_owned());
        }
        if self.completion_id != derive_completion_id(self) {
            return Err("Finalization V4 Completion identity does not reproduce".to_owned());
        }
        Ok(())
    }

    fn encode_body(
        &self,
        writer: &mut FixedWriter<'_>,
    ) -> Result<(), PopulationFinalizationV4Refusal> {
        self.source.encode_body(writer)?;
        writer.array(&self.ordered_family_digest)?;
        writer.array(&self.ordered_decision_digest)?;
        writer.u64(self.admitted_count)?;
        writer.u64(self.rejected_count)?;
        writer.u64(self.unmeasured_count)?;
        writer.u64(self.refused_count)?;
        writer.array(&self.completion_id)
    }

    fn decode_body(
        sequence: u64,
        reader: &mut FixedReader<'_>,
    ) -> Result<Self, PopulationFinalizationV4Refusal> {
        let value = Self {
            sequence,
            source: FinalizationSourceV4::decode_body(reader)?,
            ordered_family_digest: reader.array()?,
            ordered_decision_digest: reader.array()?,
            admitted_count: reader.u64()?,
            rejected_count: reader.u64()?,
            unmeasured_count: reader.u64()?,
            refused_count: reader.u64()?,
            completion_id: reader.array()?,
        };
        value.validate()?;
        Ok(value)
    }
}

fn prepare_population_finalization_v4(
    admission: &mut PopulationAdmissionV4Authority,
) -> Result<PreparedPopulationFinalizationV4, PopulationFinalizationV4Refusal> {
    let projection = admission
        .finalization_projection()
        .map_err(|why| format!("Finalization V4 Admission source refused: {why}"))?;
    PreparedPopulationFinalizationV4::from_projection(&projection)
}

fn derive_finalization_id(
    source: &FinalizationSourceV4,
    families: &[PopulationAdmissionV4FamilyProjection; 2],
    decisions: &[PopulationAdmissionV4DecisionProjection],
) -> Result<[u8; 32], PopulationFinalizationV4Refusal> {
    let mut hasher = finalization_source_hasher(source);
    for family in families {
        hasher.update(&family.family_id());
    }
    let count = u64::try_from(decisions.len())
        .map_err(|_| "Finalization V4 identity decision count does not fit u64".to_owned())?;
    hasher.update(&count.to_le_bytes());
    for decision in decisions {
        hasher.update(&decision.decision_id());
        hasher.update(decision.runner_decision());
    }
    Ok(hasher.finalize())
}

fn derive_finalization_id_from_records(
    source: &FinalizationSourceV4,
    families: &[FamilyRecordV4; 2],
    decisions: &[DecisionRecordV4],
) -> Result<[u8; 32], PopulationFinalizationV4Refusal> {
    let mut hasher = finalization_source_hasher(source);
    for family in families {
        hasher.update(&family.admission_family_id);
    }
    let count = u64::try_from(decisions.len())
        .map_err(|_| "Finalization V4 identity decision count does not fit u64".to_owned())?;
    hasher.update(&count.to_le_bytes());
    for decision in decisions {
        hasher.update(&decision.admission_decision_id);
        hasher.update(&decision.runner_decision);
    }
    Ok(hasher.finalize())
}

fn finalization_source_hasher(source: &FinalizationSourceV4) -> Hasher {
    let mut hasher = Hasher::new();
    hasher.update(FINALIZATION_DOMAIN);
    hasher.update(&VERSION.to_le_bytes());
    hasher.update(&source.admission_sequence.to_le_bytes());
    hasher.update(&source.admission_first_record.to_le_bytes());
    hasher.update(&source.admission_block_id);
    hasher.update(&source.admission_completion_id);
    hasher.update(&source.rung_seconds.to_le_bytes());
    hasher.update(&source.horizon_bars.to_le_bytes());
    hasher.update(&source.requested_span.from_year().to_le_bytes());
    hasher.update(&[source.requested_span.from_month()]);
    hasher.update(&source.requested_span.to_year().to_le_bytes());
    hasher.update(&[source.requested_span.to_month()]);
    for value in [
        source.feed_digest,
        source.source_commit_digest,
        source.calendar_policy_digest,
        source.daily_reference_policy_digest,
        source.vocabulary_digest,
        source.evaluation_policy_digest,
        source.statistics_authority_id,
        source.statistics_completion_digest,
        source.statistics_policy_digest,
        source.base_pair_id,
        source.search_pair_id,
        source.policy_digest,
    ] {
        hasher.update(&value);
    }
    hasher.update(&source.policy);
    hasher.update(&[source.nifty_terminal as u8, source.banknifty_terminal as u8]);
    hasher.update(&source.nifty_candidate_count.to_le_bytes());
    hasher.update(&source.banknifty_candidate_count.to_le_bytes());
    hasher.update(&source.candidate_count.to_le_bytes());
    hasher.update(&source.decision_count.to_le_bytes());
    hasher
}

fn derive_family_row_id(
    value: &FamilyRecordV4,
) -> Result<[u8; 32], PopulationFinalizationV4Refusal> {
    let mut copy = *value;
    copy.row_id = [0; 32];
    let mut raw = [0_u8; RECORD_BYTES];
    let mut writer = FixedWriter::new(&mut raw[..PAYLOAD_BYTES], 0);
    copy.encode_body(&mut writer)?;
    Ok(hash_slices(FAMILY_ROW_DOMAIN, &[writer.written()?]))
}

fn derive_decision_row_id(
    value: &DecisionRecordV4,
) -> Result<[u8; 32], PopulationFinalizationV4Refusal> {
    let mut copy = value.clone();
    copy.row_id = [0; 32];
    let mut raw = [0_u8; RECORD_BYTES];
    let mut writer = FixedWriter::new(&mut raw[..PAYLOAD_BYTES], 0);
    copy.encode_body(&mut writer)?;
    Ok(hash_slices(DECISION_ROW_DOMAIN, &[writer.written()?]))
}

fn ordered_family_digest(values: &[FamilyRecordV4; 2]) -> [u8; 32] {
    hash_slices(
        ORDERED_FAMILIES_DOMAIN,
        &[&2_u64.to_le_bytes(), &values[0].row_id, &values[1].row_id],
    )
}

fn ordered_decision_digest(
    values: &[DecisionRecordV4],
) -> Result<[u8; 32], PopulationFinalizationV4Refusal> {
    let mut hasher = Hasher::new();
    hasher.update(ORDERED_DECISIONS_DOMAIN);
    let count = u64::try_from(values.len())
        .map_err(|_| "Finalization V4 ordered decision count does not fit u64".to_owned())?;
    hasher.update(&count.to_le_bytes());
    for value in values {
        hasher.update(&value.row_id);
    }
    Ok(hasher.finalize())
}

fn derive_completion_id(value: &CompletionV4) -> [u8; 32] {
    hash_slices(
        COMPLETION_DOMAIN,
        &[
            &VERSION.to_le_bytes(),
            &value.sequence.to_le_bytes(),
            &value.source.finalization_id,
            &value.source.admission_block_id,
            &value.source.admission_completion_id,
            &value.ordered_family_digest,
            &value.ordered_decision_digest,
            &value.admitted_count.to_le_bytes(),
            &value.rejected_count.to_le_bytes(),
            &value.unmeasured_count.to_le_bytes(),
            &value.refused_count.to_le_bytes(),
        ],
    )
}

fn encoded_block(
    prepared: &PreparedPopulationFinalizationV4,
    sequence: u64,
) -> Result<Vec<[u8; RECORD_BYTES]>, PopulationFinalizationV4Refusal> {
    let completion = CompletionV4::from_prepared(sequence, prepared)?;
    let capacity = prepared
        .decisions
        .len()
        .checked_add(4)
        .ok_or_else(|| "Finalization V4 encoded block size overflowed".to_owned())?;
    let mut records = Vec::new();
    records
        .try_reserve_exact(capacity)
        .map_err(|why| format!("cannot reserve Finalization V4 encoded block: {why}"))?;
    records.push(encode_manifest(&completion, DATA_KIND)?);
    records.push(encode_family(&prepared.families[0], sequence)?);
    records.push(encode_family(&prepared.families[1], sequence)?);
    for decision in &prepared.decisions {
        records.push(encode_decision(decision, sequence)?);
    }
    records.push(encode_manifest(&completion, COMPLETION_KIND)?);
    Ok(records)
}

fn encode_manifest(
    value: &CompletionV4,
    kind: u32,
) -> Result<[u8; RECORD_BYTES], PopulationFinalizationV4Refusal> {
    encode_record(kind, value.sequence, |writer| value.encode_body(writer))
}

fn decode_manifest(
    raw: &[u8; RECORD_BYTES],
    kind: u32,
) -> Result<CompletionV4, PopulationFinalizationV4Refusal> {
    let (sequence, mut reader) = decode_record(raw, kind)?;
    let value = CompletionV4::decode_body(sequence, &mut reader)?;
    reader.require_remaining_zero("manifest reserve")?;
    Ok(value)
}

fn encode_family(
    value: &FamilyRecordV4,
    sequence: u64,
) -> Result<[u8; RECORD_BYTES], PopulationFinalizationV4Refusal> {
    encode_record(FAMILY_KIND, sequence, |writer| value.encode_body(writer))
}

fn decode_family_record(
    raw: &[u8; RECORD_BYTES],
) -> Result<(u64, FamilyRecordV4), PopulationFinalizationV4Refusal> {
    let (sequence, mut reader) = decode_record(raw, FAMILY_KIND)?;
    let value = FamilyRecordV4::decode_body(&mut reader)?;
    reader.require_remaining_zero("family reserve")?;
    Ok((sequence, value))
}

fn encode_decision(
    value: &DecisionRecordV4,
    sequence: u64,
) -> Result<[u8; RECORD_BYTES], PopulationFinalizationV4Refusal> {
    encode_record(DECISION_KIND, sequence, |writer| value.encode_body(writer))
}

fn decode_decision(
    raw: &[u8; RECORD_BYTES],
) -> Result<(u64, DecisionRecordV4), PopulationFinalizationV4Refusal> {
    let (sequence, mut reader) = decode_record(raw, DECISION_KIND)?;
    let value = DecisionRecordV4::decode_body(&mut reader)?;
    reader.require_remaining_zero("decision reserve")?;
    Ok((sequence, value))
}

fn encode_record(
    kind: u32,
    sequence: u64,
    encode_body: impl FnOnce(&mut FixedWriter<'_>) -> Result<(), PopulationFinalizationV4Refusal>,
) -> Result<[u8; RECORD_BYTES], PopulationFinalizationV4Refusal> {
    let mut raw = [0_u8; RECORD_BYTES];
    raw[..16].copy_from_slice(&RECORD_MAGIC);
    raw[16..20].copy_from_slice(&VERSION.to_le_bytes());
    raw[20..24].copy_from_slice(&kind.to_le_bytes());
    raw[24..32].copy_from_slice(&sequence.to_le_bytes());
    let mut writer = FixedWriter::new(&mut raw[..PAYLOAD_BYTES], 32);
    encode_body(&mut writer)?;
    let seal = hash_slices(RECORD_DOMAIN, &[&raw[..PAYLOAD_BYTES]]);
    raw[PAYLOAD_BYTES..].copy_from_slice(&seal);
    Ok(raw)
}

fn decode_record(
    raw: &[u8; RECORD_BYTES],
    expected_kind: u32,
) -> Result<(u64, FixedReader<'_>), PopulationFinalizationV4Refusal> {
    let expected_seal = hash_slices(RECORD_DOMAIN, &[&raw[..PAYLOAD_BYTES]]);
    if raw[PAYLOAD_BYTES..] != expected_seal {
        return Err("Finalization V4 fixed-record seal is invalid".to_owned());
    }
    if raw[..16] != RECORD_MAGIC || u32_at(raw, 16)? != VERSION || u32_at(raw, 20)? != expected_kind
    {
        return Err("Finalization V4 record header/version/domain mismatch".to_owned());
    }
    let sequence = u64_at(raw, 24)?;
    Ok((sequence, FixedReader::new(&raw[..PAYLOAD_BYTES], 32)))
}

fn header() -> [u8; HEADER_BYTES] {
    let mut raw = [0_u8; HEADER_BYTES];
    raw[..16].copy_from_slice(&HEADER_MAGIC);
    raw[16..20].copy_from_slice(&VERSION.to_le_bytes());
    raw[20..24].copy_from_slice(&RECORD_BYTES_U32.to_le_bytes());
    let seal = hash_slices(HEADER_DOMAIN, &[&raw[..32]]);
    raw[32..].copy_from_slice(&seal);
    raw
}

fn verify_header(file: &mut File) -> Result<(), PopulationFinalizationV4Refusal> {
    let mut raw = [0_u8; HEADER_BYTES];
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| format!("cannot read Finalization V4 header: {why}"))?;
    if raw != header() {
        return Err("Finalization V4 header is invalid".to_owned());
    }
    Ok(())
}

fn status_from_runner(value: AdmissionStatusV1) -> AdmissionV4DecisionStatus {
    match value {
        AdmissionStatusV1::Admitted => AdmissionV4DecisionStatus::Admitted,
        AdmissionStatusV1::Rejected => AdmissionV4DecisionStatus::Rejected,
        AdmissionStatusV1::Unmeasured => AdmissionV4DecisionStatus::Unmeasured,
        AdmissionStatusV1::Refused => AdmissionV4DecisionStatus::Refused,
    }
}

fn decode_family(value: u8) -> Result<AdmissionV4Family, PopulationFinalizationV4Refusal> {
    match value {
        1 => Ok(AdmissionV4Family::Nifty),
        2 => Ok(AdmissionV4Family::BankNifty),
        _ => Err(format!("Finalization V4 family tag {value} is unknown")),
    }
}

fn decode_terminal(
    value: u8,
) -> Result<AdmissionV4FamilyTerminal, PopulationFinalizationV4Refusal> {
    match value {
        1 => Ok(AdmissionV4FamilyTerminal::Evaluated),
        2 => Ok(AdmissionV4FamilyTerminal::InsufficientForCscv),
        3 => Ok(AdmissionV4FamilyTerminal::NaturallyExtinct),
        _ => Err(format!("Finalization V4 terminal tag {value} is unknown")),
    }
}

fn decode_status(value: u8) -> Result<AdmissionV4DecisionStatus, PopulationFinalizationV4Refusal> {
    match value {
        1 => Ok(AdmissionV4DecisionStatus::Admitted),
        2 => Ok(AdmissionV4DecisionStatus::Rejected),
        3 => Ok(AdmissionV4DecisionStatus::Unmeasured),
        4 => Ok(AdmissionV4DecisionStatus::Refused),
        _ => Err(format!(
            "Finalization V4 decision status {value} is unknown"
        )),
    }
}

fn require_nonzero(name: &str, value: [u8; 32]) -> Result<(), PopulationFinalizationV4Refusal> {
    if value == [0; 32] {
        return Err(format!("Finalization V4 {name} is zero"));
    }
    Ok(())
}

fn checked_sum(values: &[u64], name: &str) -> Result<u64, PopulationFinalizationV4Refusal> {
    values.iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(*value)
            .ok_or_else(|| format!("{name} overflowed"))
    })
}

fn bounded_set(
    capacity: usize,
    name: &str,
) -> Result<HashSet<[u8; 32]>, PopulationFinalizationV4Refusal> {
    let mut values = HashSet::new();
    values
        .try_reserve(capacity)
        .map_err(|why| format!("cannot reserve Finalization V4 {name}: {why}"))?;
    Ok(values)
}

fn hash_slices(domain: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize()
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, PopulationFinalizationV4Refusal> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| "Finalization V4 u32 offset overflowed".to_owned())?;
    let raw: [u8; 4] = bytes
        .get(offset..end)
        .ok_or_else(|| "Finalization V4 u32 field is absent".to_owned())?
        .try_into()
        .map_err(|_| "Finalization V4 u32 width differs".to_owned())?;
    Ok(u32::from_le_bytes(raw))
}

fn u64_at(bytes: &[u8], offset: usize) -> Result<u64, PopulationFinalizationV4Refusal> {
    let end = offset
        .checked_add(8)
        .ok_or_else(|| "Finalization V4 u64 offset overflowed".to_owned())?;
    let raw: [u8; 8] = bytes
        .get(offset..end)
        .ok_or_else(|| "Finalization V4 u64 field is absent".to_owned())?
        .try_into()
        .map_err(|_| "Finalization V4 u64 width differs".to_owned())?;
    Ok(u64::from_le_bytes(raw))
}

struct FixedWriter<'a> {
    bytes: &'a mut [u8],
    offset: usize,
}

impl<'a> FixedWriter<'a> {
    fn new(bytes: &'a mut [u8], offset: usize) -> Self {
        Self { bytes, offset }
    }

    fn write(&mut self, value: &[u8]) -> Result<(), PopulationFinalizationV4Refusal> {
        let end = self
            .offset
            .checked_add(value.len())
            .ok_or_else(|| "Finalization V4 writer offset overflowed".to_owned())?;
        self.bytes
            .get_mut(self.offset..end)
            .ok_or_else(|| "Finalization V4 writer exceeded fixed payload".to_owned())?
            .copy_from_slice(value);
        self.offset = end;
        Ok(())
    }

    fn array<const N: usize>(
        &mut self,
        value: &[u8; N],
    ) -> Result<(), PopulationFinalizationV4Refusal> {
        self.write(value)
    }

    fn u8(&mut self, value: u8) -> Result<(), PopulationFinalizationV4Refusal> {
        self.write(&[value])
    }

    fn u32(&mut self, value: u32) -> Result<(), PopulationFinalizationV4Refusal> {
        self.write(&value.to_le_bytes())
    }

    fn u16(&mut self, value: u16) -> Result<(), PopulationFinalizationV4Refusal> {
        self.write(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), PopulationFinalizationV4Refusal> {
        self.write(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), PopulationFinalizationV4Refusal> {
        self.write(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), PopulationFinalizationV4Refusal> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| "Finalization V4 zero reserve overflowed".to_owned())?;
        let slot = self
            .bytes
            .get_mut(self.offset..end)
            .ok_or_else(|| "Finalization V4 zero reserve exceeded payload".to_owned())?;
        if slot.iter().any(|byte| *byte != 0) {
            return Err("Finalization V4 zero reserve was not initialized".to_owned());
        }
        self.offset = end;
        Ok(())
    }

    fn written(&self) -> Result<&[u8], PopulationFinalizationV4Refusal> {
        self.bytes
            .get(..self.offset)
            .ok_or_else(|| "Finalization V4 written prefix is outside its buffer".to_owned())
    }
}

struct FixedReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> FixedReader<'a> {
    const fn new(bytes: &'a [u8], offset: usize) -> Self {
        Self { bytes, offset }
    }

    fn take<const N: usize>(&mut self) -> Result<[u8; N], PopulationFinalizationV4Refusal> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or_else(|| "Finalization V4 reader offset overflowed".to_owned())?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| "Finalization V4 reader exceeded fixed payload".to_owned())?
            .try_into()
            .map_err(|_| "Finalization V4 reader width differs".to_owned())?;
        self.offset = end;
        Ok(value)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], PopulationFinalizationV4Refusal> {
        self.take()
    }

    fn u8(&mut self) -> Result<u8, PopulationFinalizationV4Refusal> {
        self.take::<1>()?
            .first()
            .copied()
            .ok_or_else(|| "Finalization V4 byte is absent".to_owned())
    }

    fn u32(&mut self) -> Result<u32, PopulationFinalizationV4Refusal> {
        Ok(u32::from_le_bytes(self.take()?))
    }

    fn u16(&mut self) -> Result<u16, PopulationFinalizationV4Refusal> {
        Ok(u16::from_le_bytes(self.take()?))
    }

    fn u64(&mut self) -> Result<u64, PopulationFinalizationV4Refusal> {
        Ok(u64::from_le_bytes(self.take()?))
    }

    fn i64(&mut self) -> Result<i64, PopulationFinalizationV4Refusal> {
        Ok(i64::from_le_bytes(self.take()?))
    }

    fn require_zero(
        &mut self,
        count: usize,
        name: &str,
    ) -> Result<(), PopulationFinalizationV4Refusal> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| format!("Finalization V4 {name} offset overflowed"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| format!("Finalization V4 {name} is truncated"))?;
        if value.iter().any(|byte| *byte != 0) {
            return Err(format!("Finalization V4 {name} is nonzero"));
        }
        self.offset = end;
        Ok(())
    }

    fn require_remaining_zero(&self, name: &str) -> Result<(), PopulationFinalizationV4Refusal> {
        let remaining = self
            .bytes
            .get(self.offset..)
            .ok_or_else(|| format!("Finalization V4 {name} offset is invalid"))?;
        if remaining.iter().any(|byte| *byte != 0) {
            return Err(format!("Finalization V4 {name} is nonzero"));
        }
        Ok(())
    }
}

/// Fresh structural receipt for one complete Finalization V4 block.
///
/// This is audit state, not a semantic mint. Population promotion additionally
/// requires the retained Admission authority and exact opaque preparation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationFinalizationV4StructuralReceipt {
    sequence: u64,
    first_record: u64,
    finalization_id: [u8; 32],
    completion_id: [u8; 32],
    admission_block_id: [u8; 32],
    admission_completion_id: [u8; 32],
    nifty_terminal: AdmissionV4FamilyTerminal,
    banknifty_terminal: AdmissionV4FamilyTerminal,
    nifty_candidate_count: u64,
    banknifty_candidate_count: u64,
    decision_count: u64,
}

impl PopulationFinalizationV4StructuralReceipt {
    pub(crate) const fn sequence(self) -> u64 {
        self.sequence
    }

    pub(crate) const fn first_record(self) -> u64 {
        self.first_record
    }

    pub(crate) const fn finalization_id(self) -> [u8; 32] {
        self.finalization_id
    }

    pub(crate) const fn completion_id(self) -> [u8; 32] {
        self.completion_id
    }

    pub(crate) const fn admission_block_id(self) -> [u8; 32] {
        self.admission_block_id
    }

    pub(crate) const fn admission_completion_id(self) -> [u8; 32] {
        self.admission_completion_id
    }

    pub(crate) const fn nifty_terminal(self) -> AdmissionV4FamilyTerminal {
        self.nifty_terminal
    }

    pub(crate) const fn banknifty_terminal(self) -> AdmissionV4FamilyTerminal {
        self.banknifty_terminal
    }

    pub(crate) const fn nifty_candidate_count(self) -> u64 {
        self.nifty_candidate_count
    }

    pub(crate) const fn banknifty_candidate_count(self) -> u64 {
        self.banknifty_candidate_count
    }

    pub(crate) const fn decision_count(self) -> u64 {
        self.decision_count
    }
}

/// Population-successor common source and exact Admission policy projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PopulationFinalizationV4SourceProjection {
    source: FinalizationSourceV4,
}

impl PopulationFinalizationV4SourceProjection {
    pub(crate) const fn finalization_id(&self) -> [u8; 32] {
        self.source.finalization_id
    }

    pub(crate) const fn admission_sequence(&self) -> u64 {
        self.source.admission_sequence
    }

    pub(crate) const fn admission_first_record(&self) -> u64 {
        self.source.admission_first_record
    }

    pub(crate) const fn admission_block_id(&self) -> [u8; 32] {
        self.source.admission_block_id
    }

    pub(crate) const fn admission_completion_id(&self) -> [u8; 32] {
        self.source.admission_completion_id
    }

    pub(crate) const fn rung_seconds(&self) -> u32 {
        self.source.rung_seconds
    }

    pub(crate) const fn horizon_bars(&self) -> u32 {
        self.source.horizon_bars
    }

    pub(crate) const fn requested_span(&self) -> RequestedSpanIdentityV1 {
        self.source.requested_span
    }

    pub(crate) const fn feed_digest(&self) -> [u8; 32] {
        self.source.feed_digest
    }

    pub(crate) const fn source_commit_digest(&self) -> [u8; 32] {
        self.source.source_commit_digest
    }

    pub(crate) const fn calendar_policy_digest(&self) -> [u8; 32] {
        self.source.calendar_policy_digest
    }

    pub(crate) const fn daily_reference_policy_digest(&self) -> [u8; 32] {
        self.source.daily_reference_policy_digest
    }

    pub(crate) const fn vocabulary_digest(&self) -> [u8; 32] {
        self.source.vocabulary_digest
    }

    pub(crate) const fn evaluation_policy_digest(&self) -> [u8; 32] {
        self.source.evaluation_policy_digest
    }

    pub(crate) const fn statistics_authority_id(&self) -> [u8; 32] {
        self.source.statistics_authority_id
    }

    pub(crate) const fn statistics_completion_digest(&self) -> [u8; 32] {
        self.source.statistics_completion_digest
    }

    pub(crate) const fn statistics_policy_digest(&self) -> [u8; 32] {
        self.source.statistics_policy_digest
    }

    pub(crate) const fn base_pair_id(&self) -> [u8; 32] {
        self.source.base_pair_id
    }

    pub(crate) const fn search_pair_id(&self) -> [u8; 32] {
        self.source.search_pair_id
    }

    pub(crate) const fn policy(&self) -> &[u8; RUNNER_POLICY_BYTES] {
        &self.source.policy
    }

    pub(crate) const fn policy_digest(&self) -> [u8; 32] {
        self.source.policy_digest
    }

    pub(crate) const fn candidate_count(&self) -> u64 {
        self.source.candidate_count
    }

    pub(crate) const fn decision_count(&self) -> u64 {
        self.source.decision_count
    }
}

/// Population-successor exact family projection, including terminal evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationFinalizationV4FamilyProjection {
    record: FamilyRecordV4,
}

impl PopulationFinalizationV4FamilyProjection {
    pub(crate) const fn row_id(&self) -> [u8; 32] {
        self.record.row_id
    }

    pub(crate) const fn family(&self) -> AdmissionV4Family {
        self.record.family
    }

    pub(crate) const fn terminal(&self) -> AdmissionV4FamilyTerminal {
        self.record.terminal
    }

    pub(crate) const fn admission_family_id(&self) -> [u8; 32] {
        self.record.admission_family_id
    }

    pub(crate) const fn statistics_family_id(&self) -> [u8; 32] {
        self.record.statistics_family_id
    }

    pub(crate) const fn candidate_universe_id(&self) -> [u8; 32] {
        self.record.candidate_universe_id
    }

    pub(crate) const fn candidate_completion_digest(&self) -> [u8; 32] {
        self.record.candidate_completion_digest
    }

    pub(crate) const fn candidate_ordered_row_digest(&self) -> [u8; 32] {
        self.record.candidate_ordered_row_digest
    }

    pub(crate) const fn pre_admission_authority_id(&self) -> [u8; 32] {
        self.record.pre_admission_authority_id
    }

    pub(crate) const fn observation_authority_id(&self) -> [u8; 32] {
        self.record.observation_authority_id
    }

    pub(crate) const fn observation_identity(&self) -> [u8; 32] {
        self.record.observation_identity
    }

    pub(crate) const fn observation_policy_digest(&self) -> [u8; 32] {
        self.record.observation_policy_digest
    }

    pub(crate) const fn observation_data_digest(&self) -> [u8; 32] {
        self.record.observation_data_digest
    }

    pub(crate) const fn observation_completion_digest(&self) -> [u8; 32] {
        self.record.observation_completion_digest
    }

    pub(crate) const fn base_completion_id(&self) -> [u8; 32] {
        self.record.base_completion_id
    }

    pub(crate) const fn signal_digest(&self) -> [u8; 32] {
        self.record.signal_digest
    }

    pub(crate) const fn signal_bars(&self) -> u64 {
        self.record.signal_bars
    }

    pub(crate) const fn signal_first_ts_micros(&self) -> i64 {
        self.record.signal_first_ts_micros
    }

    pub(crate) const fn signal_last_ts_micros(&self) -> i64 {
        self.record.signal_last_ts_micros
    }

    pub(crate) const fn signal_column_digest(&self) -> [u8; 32] {
        self.record.signal_column_digest
    }

    pub(crate) const fn long_grid_policy_digest(&self) -> [u8; 32] {
        self.record.long_grid_policy_digest
    }

    pub(crate) const fn long_grid_resolution_digest(&self) -> [u8; 32] {
        self.record.long_grid_resolution_digest
    }

    pub(crate) const fn short_grid_policy_digest(&self) -> [u8; 32] {
        self.record.short_grid_policy_digest
    }

    pub(crate) const fn short_grid_resolution_digest(&self) -> [u8; 32] {
        self.record.short_grid_resolution_digest
    }

    pub(crate) const fn grid_composite_digest(&self) -> [u8; 32] {
        self.record.grid_composite_digest
    }

    pub(crate) const fn search_member_id(&self) -> [u8; 32] {
        self.record.search_member_id
    }

    pub(crate) const fn search_source_authority_id(&self) -> [u8; 32] {
        self.record.search_source_authority_id
    }

    pub(crate) const fn search_validation_policy_id(&self) -> [u8; 32] {
        self.record.search_validation_policy_id
    }

    pub(crate) const fn search_validation_family_id(&self) -> [u8; 32] {
        self.record.search_validation_family_id
    }

    pub(crate) const fn search_walk_id(&self) -> [u8; 32] {
        self.record.search_walk_id
    }

    pub(crate) const fn search_fold_count(&self) -> u64 {
        self.record.search_fold_count
    }

    pub(crate) const fn search_decided_folds(&self) -> u64 {
        self.record.search_decided_folds
    }

    pub(crate) const fn search_profitable_oos_folds(&self) -> u64 {
        self.record.search_profitable_oos_folds
    }

    pub(crate) const fn search_aggregate_oos_paisa(&self) -> i64 {
        self.record.search_aggregate_oos_paisa
    }

    pub(crate) const fn search_evaluated_population_cells(&self) -> u64 {
        self.record.search_evaluated_population_cells
    }

    pub(crate) const fn statistics_candidate_digest(&self) -> [u8; 32] {
        self.record.statistics_candidate_digest
    }

    pub(crate) const fn statistics_period_digest(&self) -> [u8; 32] {
        self.record.statistics_period_digest
    }

    pub(crate) const fn statistics_split_digest(&self) -> [u8; 32] {
        self.record.statistics_split_digest
    }

    pub(crate) const fn candidate_count(&self) -> u64 {
        self.record.candidate_count
    }

    pub(crate) const fn decision_count(&self) -> u64 {
        self.record.decision_count
    }
}

/// Population-successor exact evaluated-Candidate decision projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PopulationFinalizationV4DecisionProjection {
    record: DecisionRecordV4,
    comparison_values: AdmissionEvidenceValuesV1,
    verdict: AdmissionVerdictV1,
}

impl PopulationFinalizationV4DecisionProjection {
    pub(crate) const fn row_id(&self) -> [u8; 32] {
        self.record.row_id
    }

    pub(crate) const fn decision_sequence(&self) -> u64 {
        self.record.decision_sequence
    }

    pub(crate) const fn statistics_sequence(&self) -> u64 {
        self.record.statistics_sequence
    }

    pub(crate) const fn family(&self) -> AdmissionV4Family {
        self.record.family
    }

    pub(crate) const fn family_sequence(&self) -> u64 {
        self.record.family_sequence
    }

    pub(crate) const fn status(&self) -> AdmissionV4DecisionStatus {
        self.record.status
    }

    pub(crate) const fn admission_decision_id(&self) -> [u8; 32] {
        self.record.admission_decision_id
    }

    pub(crate) const fn candidate_semantic_id(&self) -> [u8; 32] {
        self.record.candidate_semantic_id
    }

    pub(crate) const fn candidate_row_digest(&self) -> [u8; 32] {
        self.record.candidate_row_digest
    }

    pub(crate) const fn pre_admission_authority_id(&self) -> [u8; 32] {
        self.record.pre_admission_authority_id
    }

    pub(crate) const fn statistics_period_digest(&self) -> [u8; 32] {
        self.record.statistics_period_digest
    }

    pub(crate) const fn statistics_split_digest(&self) -> [u8; 32] {
        self.record.statistics_split_digest
    }

    pub(crate) const fn search_member_id(&self) -> [u8; 32] {
        self.record.search_member_id
    }

    pub(crate) const fn base_evidence_id(&self) -> [u8; 32] {
        self.record.base_evidence_id
    }

    pub(crate) const fn runner_decision(&self) -> &[u8; RUNNER_DECISION_BYTES] {
        &self.record.runner_decision
    }

    pub(crate) const fn comparison_values(&self) -> AdmissionEvidenceValuesV1 {
        self.comparison_values
    }

    pub(crate) const fn verdict(&self) -> AdmissionVerdictV1 {
        self.verdict
    }
}

/// Complete fresh Finalization V4 handoff for a version-separated Population
/// successor. Existing Population V5 bytes are V3-specific and are not widened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PopulationFinalizationV4PopulationSourceV1 {
    receipt: PopulationFinalizationV4StructuralReceipt,
    source: PopulationFinalizationV4SourceProjection,
    families: [PopulationFinalizationV4FamilyProjection; 2],
    decisions: Vec<PopulationFinalizationV4DecisionProjection>,
}

impl PopulationFinalizationV4PopulationSourceV1 {
    pub(crate) const fn receipt(&self) -> PopulationFinalizationV4StructuralReceipt {
        self.receipt
    }

    pub(crate) const fn source(&self) -> &PopulationFinalizationV4SourceProjection {
        &self.source
    }

    pub(crate) const fn families(&self) -> &[PopulationFinalizationV4FamilyProjection; 2] {
        &self.families
    }

    pub(crate) fn decisions(&self) -> &[PopulationFinalizationV4DecisionProjection] {
        &self.decisions
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TrailingV4 {
    first_record: u64,
    source: FinalizationSourceV4,
    record_count: u64,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGeneration {
    identity: PlatformIdentity,
    len: u64,
    digest: [u8; 32],
}

struct PopulationFinalizationV4Ledger {
    root: PathBuf,
    root_file: File,
    root_identity: PlatformIdentity,
    lock_path: PathBuf,
    lock_file: File,
    lock_generation: FileGeneration,
    data_path: PathBuf,
    data_file: File,
    data_generation: FileGeneration,
    bounds: PopulationFinalizationV4Bounds,
    writable: bool,
    receipts: HashMap<[u8; 32], PopulationFinalizationV4StructuralReceipt>,
    trailing: Option<TrailingV4>,
    record_count: u64,
}

impl PopulationFinalizationV4Ledger {
    fn open_read(
        root: &Path,
        bounds: PopulationFinalizationV4Bounds,
    ) -> Result<Self, PopulationFinalizationV4Refusal> {
        Self::open(root, bounds, false)
    }

    fn open_write(
        root: &Path,
        bounds: PopulationFinalizationV4Bounds,
    ) -> Result<Self, PopulationFinalizationV4Refusal> {
        Self::open(root, bounds, true)
    }

    fn open(
        root: &Path,
        bounds: PopulationFinalizationV4Bounds,
        writable: bool,
    ) -> Result<Self, PopulationFinalizationV4Refusal> {
        let (root, root_file, root_identity) = open_root(root)?;
        let lock_path = root.join(LOCK_FILE);
        let data_path = root.join(DATA_FILE);
        let (lock_file, lock_created) = open_child(&lock_path, writable)?;
        if writable {
            lock_file
                .lock()
                .map_err(|why| format!("cannot lock Finalization V4 writer: {why}"))?;
        } else {
            lock_file
                .lock_shared()
                .map_err(|why| format!("cannot shared-lock Finalization V4 reader: {why}"))?;
        }
        let opened = (|| {
            let (mut data_file, data_created) = open_child(&data_path, writable)?;
            if data_created {
                data_file
                    .write_all(&header())
                    .and_then(|()| data_file.sync_all())
                    .map_err(|why| format!("cannot initialize Finalization V4 data: {why}"))?;
            }
            if lock_created || data_created {
                root_file
                    .sync_all()
                    .map_err(|why| format!("cannot sync Finalization V4 root: {why}"))?;
            }
            verify_header(&mut data_file)?;
            let lock_generation = file_generation(&lock_file, &lock_path, 0)?;
            let data_generation = file_generation(&data_file, &data_path, bounds.file_bytes)?;
            let mut ledger = Self {
                root,
                root_file,
                root_identity,
                lock_path,
                lock_file: lock_file
                    .try_clone()
                    .map_err(|why| format!("cannot clone Finalization V4 lock: {why}"))?,
                lock_generation,
                data_path,
                data_file,
                data_generation,
                bounds,
                writable,
                receipts: HashMap::new(),
                trailing: None,
                record_count: 0,
            };
            ledger.scan()?;
            Ok(ledger)
        })();
        let unlocked = lock_file
            .unlock()
            .map_err(|why| format!("cannot unlock Finalization V4 after open: {why}"));
        combine(opened, unlocked)
    }

    fn scan(&mut self) -> Result<(), PopulationFinalizationV4Refusal> {
        self.receipts.clear();
        self.trailing = None;
        let len = self.data_generation.len;
        if len < HEADER_BYTES as u64
            || !(len - HEADER_BYTES as u64).is_multiple_of(RECORD_BYTES as u64)
        {
            return Err("Finalization V4 data file is ragged".to_owned());
        }
        let total = (len - HEADER_BYTES as u64) / RECORD_BYTES as u64;
        let mut first = 0_u64;
        while first < total {
            let data = decode_manifest(&read_record_at(&mut self.data_file, first)?, DATA_KIND)?;
            let expected_sequence = u64::try_from(self.receipts.len())
                .map_err(|_| "Finalization V4 authority sequence does not fit u64".to_owned())?;
            if data.sequence != expected_sequence {
                return Err("Finalization V4 logical sequence is not canonical".to_owned());
            }
            if data.source.decision_count > self.bounds.decisions_per_authority {
                return Err("Finalization V4 decision count exceeds explicit bound".to_owned());
            }
            let block_records = data
                .source
                .decision_count
                .checked_add(4)
                .ok_or_else(|| "Finalization V4 block record count overflowed".to_owned())?;
            let remaining = total - first;
            if remaining < block_records {
                validate_prefix(&mut self.data_file, first, remaining, &data)?;
                if self.receipts.contains_key(&data.source.finalization_id) {
                    return Err(
                        "Finalization V4 trailing prefix repeats a complete block".to_owned()
                    );
                }
                self.trailing = Some(TrailingV4 {
                    first_record: first,
                    source: data.source,
                    record_count: remaining,
                });
                first = total;
                continue;
            }
            let receipt = validate_complete_block(&mut self.data_file, first, &data)?;
            if self
                .receipts
                .insert(receipt.finalization_id, receipt)
                .is_some()
            {
                return Err("Finalization V4 identity appears more than once".to_owned());
            }
            let authority_count = u64::try_from(self.receipts.len())
                .map_err(|_| "Finalization V4 authority count does not fit u64".to_owned())?;
            if authority_count > self.bounds.authorities {
                return Err("Finalization V4 authority count exceeds explicit bound".to_owned());
            }
            first = first
                .checked_add(block_records)
                .ok_or_else(|| "Finalization V4 scan offset overflowed".to_owned())?;
        }
        self.record_count = total;
        self.require_unchanged()
    }

    fn append(
        &mut self,
        prepared: &PreparedPopulationFinalizationV4,
    ) -> Result<(bool, PopulationFinalizationV4StructuralReceipt), PopulationFinalizationV4Refusal>
    {
        if !self.writable {
            return Err("Finalization V4 ledger is read-only".to_owned());
        }
        self.lock_file
            .lock()
            .map_err(|why| format!("cannot lock Finalization V4 append: {why}"))?;
        let result = self.append_locked(prepared);
        let unlocked = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot unlock Finalization V4 append: {why}"));
        combine(result, unlocked)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one receipt-last append keeps exact-prefix retry, evidence sync, Completion sync and held-generation checks adjacent"
    )]
    fn append_locked(
        &mut self,
        prepared: &PreparedPopulationFinalizationV4,
    ) -> Result<(bool, PopulationFinalizationV4StructuralReceipt), PopulationFinalizationV4Refusal>
    {
        self.require_unchanged()?;
        prepared.validate()?;
        if prepared.source.decision_count > self.bounds.decisions_per_authority {
            return Err("Finalization V4 prepared decision count exceeds bound".to_owned());
        }
        if let Some(receipt) = self.receipts.get(&prepared.source.finalization_id).copied() {
            require_exact_records(
                &mut self.data_file,
                receipt.first_record,
                prepared,
                receipt.sequence,
            )?;
            self.data_file
                .sync_all()
                .map_err(|why| format!("cannot sync reused Finalization V4 data: {why}"))?;
            self.root_file
                .sync_all()
                .map_err(|why| format!("cannot sync reused Finalization V4 root: {why}"))?;
            self.require_unchanged()?;
            return Ok((false, receipt));
        }
        let sequence = u64::try_from(self.receipts.len())
            .map_err(|_| "Finalization V4 sequence does not fit u64".to_owned())?;
        let records = encoded_block(prepared, sequence)?;
        let prefix = if let Some(trailing) = &self.trailing {
            if trailing.source != prepared.source
                || trailing.first_record + trailing.record_count != self.record_count
            {
                return Err("Finalization V4 trailing prefix is not the exact retry".to_owned());
            }
            for index in 0..trailing.record_count {
                let stored = read_record_at(&mut self.data_file, trailing.first_record + index)?;
                let expected = records
                    .get(usize::try_from(index).map_err(|_| {
                        "Finalization V4 prefix index does not fit usize".to_owned()
                    })?)
                    .ok_or_else(|| {
                        "Finalization V4 trailing prefix is longer than preparation".to_owned()
                    })?;
                if &stored != expected {
                    return Err(
                        "Finalization V4 trailing prefix bytes differ from exact retry".to_owned(),
                    );
                }
            }
            trailing.record_count
        } else {
            0
        };
        let encoded_count = u64::try_from(records.len())
            .map_err(|_| "Finalization V4 record length does not fit u64".to_owned())?;
        let appended_count = encoded_count
            .checked_sub(prefix)
            .ok_or_else(|| "Finalization V4 trailing prefix exceeds its exact block".to_owned())?;
        let next_record_count = self
            .record_count
            .checked_add(appended_count)
            .ok_or_else(|| "Finalization V4 appended record count overflowed".to_owned())?;
        let next_bytes = (HEADER_BYTES as u64)
            .checked_add(
                next_record_count
                    .checked_mul(RECORD_BYTES as u64)
                    .ok_or_else(|| "Finalization V4 appended bytes overflowed".to_owned())?,
            )
            .ok_or_else(|| "Finalization V4 file bytes overflowed".to_owned())?;
        let authority_count = u64::try_from(self.receipts.len())
            .map_err(|_| "Finalization V4 authority count does not fit u64".to_owned())?;
        if next_bytes > self.bounds.file_bytes || authority_count >= self.bounds.authorities {
            return Err("Finalization V4 append exceeds explicit authority/file bound".to_owned());
        }
        let completion_index = u64::try_from(
            records
                .len()
                .checked_sub(1)
                .ok_or_else(|| "Finalization V4 encoded block omitted Completion".to_owned())?,
        )
        .map_err(|_| "Finalization V4 Completion index does not fit u64".to_owned())?;
        for index in prefix..completion_index {
            let record =
                records
                    .get(usize::try_from(index).map_err(|_| {
                        "Finalization V4 append index does not fit usize".to_owned()
                    })?)
                    .ok_or_else(|| "Finalization V4 append record is absent".to_owned())?;
            append_raw(&mut self.data_file, record)?;
        }
        self.data_file
            .sync_all()
            .map_err(|why| format!("cannot sync Finalization V4 evidence prefix: {why}"))?;
        append_raw(
            &mut self.data_file,
            records
                .last()
                .ok_or_else(|| "Finalization V4 encoded block is empty".to_owned())?,
        )?;
        self.data_file
            .sync_all()
            .map_err(|why| format!("cannot sync Finalization V4 Completion: {why}"))?;
        self.root_file
            .sync_all()
            .map_err(|why| format!("cannot sync Finalization V4 directory: {why}"))?;
        self.data_generation =
            file_generation(&self.data_file, &self.data_path, self.bounds.file_bytes)?;
        self.scan()?;
        let receipt = self
            .receipts
            .get(&prepared.source.finalization_id)
            .copied()
            .ok_or_else(|| "Finalization V4 appended block is absent after scan".to_owned())?;
        Ok((true, receipt))
    }

    fn read_complete(
        &mut self,
        receipt: PopulationFinalizationV4StructuralReceipt,
    ) -> Result<
        (CompletionV4, [FamilyRecordV4; 2], Vec<DecisionRecordV4>),
        PopulationFinalizationV4Refusal,
    > {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot shared-lock Finalization V4 projection: {why}"))?;
        let result = (|| {
            self.require_unchanged()?;
            let indexed = self
                .receipts
                .get(&receipt.finalization_id)
                .copied()
                .ok_or_else(|| "Finalization V4 receipt is absent".to_owned())?;
            if indexed != receipt {
                return Err("Finalization V4 receipt is stale or foreign".to_owned());
            }
            let data = decode_manifest(
                &read_record_at(&mut self.data_file, receipt.first_record)?,
                DATA_KIND,
            )?;
            let (nifty_sequence, nifty) = decode_family_record(&read_record_at(
                &mut self.data_file,
                receipt.first_record + 1,
            )?)?;
            let (bank_sequence, banknifty) = decode_family_record(&read_record_at(
                &mut self.data_file,
                receipt.first_record + 2,
            )?)?;
            if nifty_sequence != receipt.sequence || bank_sequence != receipt.sequence {
                return Err("Finalization V4 family record moved between blocks".to_owned());
            }
            let families = [nifty, banknifty];
            for family in &families {
                family.validate(&data.source)?;
            }
            let mut decisions = Vec::new();
            decisions
                .try_reserve_exact(
                    usize::try_from(data.source.decision_count).map_err(|_| {
                        "Finalization V4 decision count does not fit usize".to_owned()
                    })?,
                )
                .map_err(|why| format!("cannot reserve Finalization V4 read decisions: {why}"))?;
            for ordinal in 0..data.source.decision_count {
                let (sequence, decision) = decode_decision(&read_record_at(
                    &mut self.data_file,
                    receipt.first_record + 3 + ordinal,
                )?)?;
                if sequence != receipt.sequence || decision.decision_sequence != ordinal {
                    return Err("Finalization V4 decision moved or reordered".to_owned());
                }
                decision.validate(&data.source, &families)?;
                decisions.push(decision);
            }
            let completion = decode_manifest(
                &read_record_at(
                    &mut self.data_file,
                    receipt.first_record + 3 + data.source.decision_count,
                )?,
                COMPLETION_KIND,
            )?;
            if completion != data
                || completion.ordered_family_digest != ordered_family_digest(&families)
                || completion.ordered_decision_digest != ordered_decision_digest(&decisions)?
            {
                return Err("Finalization V4 complete block no longer reconciles".to_owned());
            }
            let prepared = PreparedPopulationFinalizationV4 {
                source: data.source.clone(),
                families,
                decisions: decisions.clone(),
            };
            prepared.validate()?;
            self.require_unchanged()?;
            Ok((completion, prepared.families, prepared.decisions))
        })();
        let unlocked = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot unlock Finalization V4 projection: {why}"));
        combine(result, unlocked)
    }

    fn require_unchanged(&self) -> Result<(), PopulationFinalizationV4Refusal> {
        if named_identity(&self.root)? != self.root_identity
            || PlatformIdentity::of(
                &self
                    .root_file
                    .metadata()
                    .map_err(|why| format!("cannot stat held Finalization V4 root: {why}"))?,
            ) != self.root_identity
            || file_generation(&self.lock_file, &self.lock_path, 0)? != self.lock_generation
            || file_generation(&self.data_file, &self.data_path, self.bounds.file_bytes)?
                != self.data_generation
        {
            return Err("Finalization V4 retained file/root generation changed".to_owned());
        }
        Ok(())
    }
}

fn validate_prefix(
    file: &mut File,
    first: u64,
    count: u64,
    data: &CompletionV4,
) -> Result<(), PopulationFinalizationV4Refusal> {
    if count == 0 {
        return Err("Finalization V4 trailing prefix omitted its Data record".to_owned());
    }
    let mut families = [None, None];
    if count >= 2 {
        let (sequence, family) = decode_family_record(&read_record_at(file, first + 1)?)?;
        if sequence != data.sequence {
            return Err("Finalization V4 trailing NIFTY family moved between blocks".to_owned());
        }
        family.validate(&data.source)?;
        families[0] = Some(family);
    }
    if count >= 3 {
        let (sequence, family) = decode_family_record(&read_record_at(file, first + 2)?)?;
        if sequence != data.sequence {
            return Err(
                "Finalization V4 trailing BANKNIFTY family moved between blocks".to_owned(),
            );
        }
        family.validate(&data.source)?;
        families[1] = Some(family);
    }
    let available_decisions = count.saturating_sub(3).min(data.source.decision_count);
    if available_decisions > 0 {
        let full_families = [
            families[0]
                .ok_or_else(|| "Finalization V4 trailing NIFTY family is absent".to_owned())?,
            families[1]
                .ok_or_else(|| "Finalization V4 trailing BANKNIFTY family is absent".to_owned())?,
        ];
        let mut next_nifty = 0_u64;
        let mut next_banknifty = 0_u64;
        let mut banknifty_started = false;
        for ordinal in 0..available_decisions {
            let (sequence, decision) =
                decode_decision(&read_record_at(file, first + 3 + ordinal)?)?;
            if sequence != data.sequence || decision.decision_sequence != ordinal {
                return Err("Finalization V4 trailing decision order differs".to_owned());
            }
            let expected = match decision.family {
                AdmissionV4Family::Nifty if !banknifty_started => &mut next_nifty,
                AdmissionV4Family::BankNifty => {
                    banknifty_started = true;
                    &mut next_banknifty
                }
                AdmissionV4Family::Nifty => {
                    return Err("Finalization V4 trailing NIFTY follows BANKNIFTY".to_owned());
                }
            };
            if decision.family_sequence != *expected {
                return Err("Finalization V4 trailing family order differs".to_owned());
            }
            *expected = expected
                .checked_add(1)
                .ok_or_else(|| "Finalization V4 trailing family count overflowed".to_owned())?;
            decision.validate(&data.source, &full_families)?;
        }
    }
    if count > 3 + data.source.decision_count {
        return Err(
            "Finalization V4 trailing prefix contains an unaccounted Completion".to_owned(),
        );
    }
    Ok(())
}

fn validate_complete_block(
    file: &mut File,
    first: u64,
    data: &CompletionV4,
) -> Result<PopulationFinalizationV4StructuralReceipt, PopulationFinalizationV4Refusal> {
    let block_records = data
        .source
        .decision_count
        .checked_add(4)
        .ok_or_else(|| "Finalization V4 block record count overflowed".to_owned())?;
    validate_prefix(file, first, block_records - 1, data)?;
    let completion = decode_manifest(
        &read_record_at(file, first + block_records - 1)?,
        COMPLETION_KIND,
    )?;
    if &completion != data {
        return Err("Finalization V4 Data and Completion records differ".to_owned());
    }
    let (_, nifty) = decode_family_record(&read_record_at(file, first + 1)?)?;
    let (_, banknifty) = decode_family_record(&read_record_at(file, first + 2)?)?;
    let families = [nifty, banknifty];
    let mut decisions = Vec::new();
    decisions
        .try_reserve_exact(
            usize::try_from(data.source.decision_count)
                .map_err(|_| "Finalization V4 decision count does not fit usize".to_owned())?,
        )
        .map_err(|why| format!("cannot reserve Finalization V4 validation decisions: {why}"))?;
    for ordinal in 0..data.source.decision_count {
        let (_, decision) = decode_decision(&read_record_at(file, first + 3 + ordinal)?)?;
        decisions.push(decision);
    }
    if completion.ordered_family_digest != ordered_family_digest(&families)
        || completion.ordered_decision_digest != ordered_decision_digest(&decisions)?
    {
        return Err("Finalization V4 ordered evidence digest differs".to_owned());
    }
    PreparedPopulationFinalizationV4 {
        source: data.source.clone(),
        families,
        decisions,
    }
    .validate()?;
    Ok(PopulationFinalizationV4StructuralReceipt {
        sequence: data.sequence,
        first_record: first,
        finalization_id: data.source.finalization_id,
        completion_id: data.completion_id,
        admission_block_id: data.source.admission_block_id,
        admission_completion_id: data.source.admission_completion_id,
        nifty_terminal: data.source.nifty_terminal,
        banknifty_terminal: data.source.banknifty_terminal,
        nifty_candidate_count: data.source.nifty_candidate_count,
        banknifty_candidate_count: data.source.banknifty_candidate_count,
        decision_count: data.source.decision_count,
    })
}

fn require_exact_records(
    file: &mut File,
    first: u64,
    prepared: &PreparedPopulationFinalizationV4,
    sequence: u64,
) -> Result<(), PopulationFinalizationV4Refusal> {
    let expected = encoded_block(prepared, sequence)?;
    for (offset, raw) in expected.iter().enumerate() {
        let offset = u64::try_from(offset)
            .map_err(|_| "Finalization V4 comparison offset does not fit u64".to_owned())?;
        if read_record_at(file, first + offset)? != *raw {
            return Err("Finalization V4 existing bytes differ from exact preparation".to_owned());
        }
    }
    Ok(())
}

fn record_offset(index: u64) -> Result<u64, PopulationFinalizationV4Refusal> {
    (HEADER_BYTES as u64)
        .checked_add(
            index
                .checked_mul(RECORD_BYTES as u64)
                .ok_or_else(|| "Finalization V4 record offset overflowed".to_owned())?,
        )
        .ok_or_else(|| "Finalization V4 record address overflowed".to_owned())
}

fn read_record_at(
    file: &mut File,
    index: u64,
) -> Result<[u8; RECORD_BYTES], PopulationFinalizationV4Refusal> {
    let mut raw = [0_u8; RECORD_BYTES];
    file.seek(SeekFrom::Start(record_offset(index)?))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| format!("cannot read Finalization V4 record {index}: {why}"))?;
    Ok(raw)
}

fn append_raw(
    file: &mut File,
    raw: &[u8; RECORD_BYTES],
) -> Result<(), PopulationFinalizationV4Refusal> {
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(raw))
        .map_err(|why| format!("cannot append Finalization V4 record: {why}"))
}

fn open_root(
    root: &Path,
) -> Result<(PathBuf, File, PlatformIdentity), PopulationFinalizationV4Refusal> {
    let metadata = std::fs::symlink_metadata(root)
        .map_err(|why| format!("cannot stat Finalization V4 root: {why}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("Finalization V4 root must be an existing nonsymlink directory".to_owned());
    }
    let canonical = std::fs::canonicalize(root)
        .map_err(|why| format!("cannot canonicalize Finalization V4 root: {why}"))?;
    let file =
        File::open(&canonical).map_err(|why| format!("cannot open Finalization V4 root: {why}"))?;
    let identity = PlatformIdentity::of(
        &file
            .metadata()
            .map_err(|why| format!("cannot stat held Finalization V4 root: {why}"))?,
    );
    if named_identity(&canonical)? != identity {
        return Err("Finalization V4 root changed while opening".to_owned());
    }
    Ok((canonical, file, identity))
}

fn open_child(
    path: &Path,
    writable: bool,
) -> Result<(File, bool), PopulationFinalizationV4Refusal> {
    if let Ok(metadata) = std::fs::symlink_metadata(path)
        && (metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err("Finalization V4 child is not a regular nonsymlink file".to_owned());
    }
    let mut options = OpenOptions::new();
    options.read(true).write(writable).create(writable);
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    options.custom_flags(O_NOFOLLOW_FLAG);
    let existed = path.exists();
    let file = options
        .open(path)
        .map_err(|why| format!("cannot open Finalization V4 child: {why}"))?;
    let metadata = file
        .metadata()
        .map_err(|why| format!("cannot stat Finalization V4 child: {why}"))?;
    if !metadata.is_file() || named_identity(path)? != PlatformIdentity::of(&metadata) {
        return Err("Finalization V4 named child differs from held file".to_owned());
    }
    Ok((file, !existed))
}

fn named_identity(path: &Path) -> Result<PlatformIdentity, PopulationFinalizationV4Refusal> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|why| format!("cannot stat named Finalization V4 path: {why}"))?;
    if metadata.file_type().is_symlink() {
        return Err("Finalization V4 named path became a symlink".to_owned());
    }
    Ok(PlatformIdentity::of(&metadata))
}

fn file_generation(
    file: &File,
    path: &Path,
    maximum: u64,
) -> Result<FileGeneration, PopulationFinalizationV4Refusal> {
    let metadata = file
        .metadata()
        .map_err(|why| format!("cannot stat Finalization V4 file: {why}"))?;
    let identity = PlatformIdentity::of(&metadata);
    if named_identity(path)? != identity {
        return Err("Finalization V4 named file was replaced".to_owned());
    }
    if (maximum == 0 && metadata.len() != 0) || (maximum != 0 && metadata.len() > maximum) {
        return Err(format!(
            "Finalization V4 file has {} bytes above bound {maximum}",
            metadata.len()
        ));
    }
    let mut clone = file
        .try_clone()
        .map_err(|why| format!("cannot clone Finalization V4 file for hashing: {why}"))?;
    clone
        .seek(SeekFrom::Start(0))
        .map_err(|why| format!("cannot rewind Finalization V4 file: {why}"))?;
    let mut hasher = Hasher::new();
    hasher.update(GENERATION_DOMAIN);
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    loop {
        let count = clone
            .read(&mut buffer)
            .map_err(|why| format!("cannot hash Finalization V4 file: {why}"))?;
        if count == 0 {
            break;
        }
        let bytes = buffer
            .get(..count)
            .ok_or_else(|| "Finalization V4 hash read exceeded its buffer".to_owned())?;
        hasher.update(bytes);
    }
    let after = file
        .metadata()
        .map_err(|why| format!("cannot restat Finalization V4 file: {why}"))?;
    if PlatformIdentity::of(&after) != identity
        || after.len() != metadata.len()
        || named_identity(path)? != identity
    {
        return Err("Finalization V4 file changed while hashing".to_owned());
    }
    Ok(FileGeneration {
        identity,
        len: metadata.len(),
        digest: hasher.finalize(),
    })
}

fn combine<T>(
    result: Result<T, PopulationFinalizationV4Refusal>,
    unlock: Result<(), PopulationFinalizationV4Refusal>,
) -> Result<T, PopulationFinalizationV4Refusal> {
    match (result, unlock) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(why), _) | (Ok(_), Err(why)) => Err(why),
    }
}

struct PopulationFinalizationV4Authority {
    receipt: PopulationFinalizationV4StructuralReceipt,
    prepared: PreparedPopulationFinalizationV4,
    ledger: PopulationFinalizationV4Ledger,
}

impl PopulationFinalizationV4Authority {
    fn population_source(
        &mut self,
    ) -> Result<PopulationFinalizationV4PopulationSourceV1, PopulationFinalizationV4Refusal> {
        let (completion, families, decisions) = self.ledger.read_complete(self.receipt)?;
        let expected = CompletionV4::from_prepared(self.receipt.sequence, &self.prepared)?;
        if completion != expected
            || families != self.prepared.families
            || decisions != self.prepared.decisions
        {
            return Err(
                "Finalization V4 retained block differs from opaque preparation".to_owned(),
            );
        }
        let mut projected = Vec::new();
        projected
            .try_reserve_exact(decisions.len())
            .map_err(|why| format!("cannot reserve Finalization V4 Population decisions: {why}"))?;
        for decision in decisions {
            let (comparison_values, verdict) =
                decision.validate(&self.prepared.source, &families)?;
            projected.push(PopulationFinalizationV4DecisionProjection {
                record: decision,
                comparison_values,
                verdict,
            });
        }
        let [nifty, banknifty] = families;
        Ok(PopulationFinalizationV4PopulationSourceV1 {
            receipt: self.receipt,
            source: PopulationFinalizationV4SourceProjection {
                source: self.prepared.source.clone(),
            },
            families: [
                PopulationFinalizationV4FamilyProjection { record: nifty },
                PopulationFinalizationV4FamilyProjection { record: banknifty },
            ],
            decisions: projected,
        })
    }
}

/// Source-retaining Finalization V4 capability. A Population successor can
/// obtain semantics only after both source and durable block are reauthenticated.
pub(crate) struct CommittedStoredPopulationFinalizationV4 {
    admission: PopulationAdmissionV4Authority,
    finalization: PopulationFinalizationV4Authority,
}

impl CommittedStoredPopulationFinalizationV4 {
    pub(crate) const fn structural_receipt(&self) -> PopulationFinalizationV4StructuralReceipt {
        self.finalization.receipt
    }

    /// Reauthenticates Admission before and after the freshly reopened
    /// Finalization block and returns the complete typed Population successor.
    ///
    /// # Errors
    ///
    /// Refuses any stale/replaced/corrupt source, a crosswired retained
    /// preparation, invalid Runner decision, or changed Finalization file.
    pub(crate) fn population_source(
        &mut self,
    ) -> Result<PopulationFinalizationV4PopulationSourceV1, PopulationFinalizationV4Refusal> {
        let before = prepare_population_finalization_v4(&mut self.admission)?;
        if before != self.finalization.prepared {
            return Err("Finalization V4 retained Admission source changed".to_owned());
        }
        let projected = self.finalization.population_source()?;
        let after = prepare_population_finalization_v4(&mut self.admission)?;
        if after != before {
            return Err("Finalization V4 Admission source changed during projection".to_owned());
        }
        Ok(projected)
    }
}

/// Exact new write or byte-identical reuse of Finalization V4.
pub(crate) enum PopulationFinalizationV4Commit {
    Written(CommittedStoredPopulationFinalizationV4),
    Reused(CommittedStoredPopulationFinalizationV4),
}

impl PopulationFinalizationV4Commit {
    pub(crate) fn authority_mut(&mut self) -> &mut CommittedStoredPopulationFinalizationV4 {
        match self {
            Self::Written(value) | Self::Reused(value) => value,
        }
    }

    pub(crate) fn into_authority(self) -> CommittedStoredPopulationFinalizationV4 {
        match self {
            Self::Written(value) | Self::Reused(value) => value,
        }
    }
}

/// Finalizes one retained fresh-reopen Admission V4 capability.
///
/// No row, family, terminal, digest, policy or status is caller-authored.
///
/// # Errors
///
/// Refuses invalid Admission semantics, bounds, foreign/torn/corrupt history,
/// a non-exact retry, persistence failure or fresh-reopen mismatch.
pub(crate) fn commit_population_finalization_v4(
    root: &Path,
    bounds: PopulationFinalizationV4Bounds,
    mut admission: PopulationAdmissionV4Authority,
) -> Result<PopulationFinalizationV4Commit, PopulationFinalizationV4Refusal> {
    let prepared = prepare_population_finalization_v4(&mut admission)?;
    let mut writer = PopulationFinalizationV4Ledger::open_write(root, bounds)?;
    let (written, receipt) = writer.append(&prepared)?;
    drop(writer);
    let mut ledger = PopulationFinalizationV4Ledger::open_read(root, bounds)?;
    let reopened = ledger
        .receipts
        .get(&prepared.source.finalization_id)
        .copied()
        .ok_or_else(|| "Finalization V4 fresh reopen omitted committed block".to_owned())?;
    if reopened != receipt {
        return Err("Finalization V4 fresh reopen changed structural receipt".to_owned());
    }
    let (completion, families, decisions) = ledger.read_complete(reopened)?;
    if completion != CompletionV4::from_prepared(reopened.sequence, &prepared)?
        || families != prepared.families
        || decisions != prepared.decisions
    {
        return Err("Finalization V4 fresh reopen differs from exact preparation".to_owned());
    }
    let authority = CommittedStoredPopulationFinalizationV4 {
        admission,
        finalization: PopulationFinalizationV4Authority {
            receipt: reopened,
            prepared,
            ledger,
        },
    };
    Ok(if written {
        PopulationFinalizationV4Commit::Written(authority)
    } else {
        PopulationFinalizationV4Commit::Reused(authority)
    })
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "fixed-record adversarial fixtures intentionally mutate exact bytes and fail loudly"
)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    use crate::population_admission_v4::{
        PopulationAdmissionV4Bounds, population_finalization_v4_test_admission_for,
    };

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new(label: &str) -> Self {
            let serial = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "brutex-population-finalization-v4-{}-{label}-{serial}",
                std::process::id()
            ));
            std::fs::create_dir(&path).expect("create Finalization V4 test root");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            if self.0.exists() {
                std::fs::remove_dir_all(&self.0).expect("remove Finalization V4 test root");
            }
        }
    }

    fn admission_bounds() -> PopulationAdmissionV4Bounds {
        PopulationAdmissionV4Bounds::new(32, 16, 4 * 1_024 * 1_024)
            .expect("Admission V4 fixture bounds")
    }

    fn bounds() -> PopulationFinalizationV4Bounds {
        PopulationFinalizationV4Bounds::new(32, 16, 4 * 1_024 * 1_024)
            .expect("Finalization V4 fixture bounds")
    }

    fn admission(
        root: &Path,
        nifty: AdmissionV4FamilyTerminal,
        banknifty: AdmissionV4FamilyTerminal,
        salt: u8,
    ) -> PopulationAdmissionV4Authority {
        population_finalization_v4_test_admission_for(
            root,
            admission_bounds(),
            nifty,
            banknifty,
            salt,
        )
        .expect("commit controlled Admission V4 source")
    }

    fn candidate_count(value: AdmissionV4FamilyTerminal) -> u64 {
        match value {
            AdmissionV4FamilyTerminal::Evaluated => 2,
            AdmissionV4FamilyTerminal::InsufficientForCscv => 1,
            AdmissionV4FamilyTerminal::NaturallyExtinct => 0,
        }
    }

    fn decision_count(value: AdmissionV4FamilyTerminal) -> u64 {
        if value == AdmissionV4FamilyTerminal::Evaluated {
            2
        } else {
            0
        }
    }

    fn write_prefix(root: &Path, records: &[[u8; RECORD_BYTES]], count: usize) {
        let mut file = File::create(root.join(DATA_FILE)).expect("create Finalization prefix");
        file.write_all(&header())
            .expect("write Finalization header");
        for raw in &records[..count] {
            file.write_all(raw)
                .expect("write Finalization prefix record");
        }
        file.sync_all().expect("sync Finalization prefix");
    }

    fn rewrite_record(root: &Path, index: u64, raw: &[u8; RECORD_BYTES]) {
        let mut file = OpenOptions::new()
            .write(true)
            .open(root.join(DATA_FILE))
            .expect("open Finalization record for rewrite");
        file.seek(SeekFrom::Start(
            record_offset(index).expect("record offset"),
        ))
        .expect("seek Finalization record");
        file.write_all(raw).expect("rewrite Finalization record");
        file.sync_all().expect("sync rewritten Finalization record");
    }

    #[test]
    fn all_nine_terminal_pairs_preserve_shape_without_invented_rows() {
        let terminals = [
            AdmissionV4FamilyTerminal::Evaluated,
            AdmissionV4FamilyTerminal::InsufficientForCscv,
            AdmissionV4FamilyTerminal::NaturallyExtinct,
        ];
        let mut salt = 1_u8;
        for nifty in terminals {
            for banknifty in terminals {
                let admission_root = TestRoot::new("all-terminals-admission");
                let finalization_root = TestRoot::new("all-terminals-finalization");
                let source = admission(admission_root.path(), nifty, banknifty, salt);
                salt = salt.wrapping_add(1);
                let mut committed =
                    commit_population_finalization_v4(finalization_root.path(), bounds(), source)
                        .expect("commit mixed-terminal Finalization V4");
                let population = committed
                    .authority_mut()
                    .population_source()
                    .expect("fresh Population successor projection");
                assert_eq!(population.families()[0].family(), AdmissionV4Family::Nifty);
                assert_eq!(
                    population.families()[1].family(),
                    AdmissionV4Family::BankNifty
                );
                assert_eq!(population.families()[0].terminal(), nifty);
                assert_eq!(population.families()[1].terminal(), banknifty);
                assert_eq!(
                    population.families()[0].candidate_count(),
                    candidate_count(nifty)
                );
                assert_eq!(
                    population.families()[1].candidate_count(),
                    candidate_count(banknifty)
                );
                let expected_decisions = decision_count(nifty) + decision_count(banknifty);
                assert_eq!(population.receipt().decision_count(), expected_decisions);
                assert_eq!(population.decisions().len() as u64, expected_decisions);
                assert_eq!(
                    population.source().candidate_count(),
                    candidate_count(nifty) + candidate_count(banknifty)
                );
                for (family, expected) in [
                    (&population.families()[0], nifty),
                    (&population.families()[1], banknifty),
                ] {
                    match expected {
                        AdmissionV4FamilyTerminal::Evaluated => {
                            assert_eq!(family.decision_count(), family.candidate_count());
                            assert_ne!(family.statistics_candidate_digest(), [0; 32]);
                            assert_eq!(family.observation_authority_id(), [0; 32]);
                        }
                        AdmissionV4FamilyTerminal::InsufficientForCscv => {
                            assert_eq!(family.candidate_count(), 1);
                            assert_eq!(family.decision_count(), 0);
                            assert_ne!(family.statistics_candidate_digest(), [0; 32]);
                        }
                        AdmissionV4FamilyTerminal::NaturallyExtinct => {
                            assert_eq!(family.candidate_count(), 0);
                            assert_eq!(family.decision_count(), 0);
                            assert_eq!(family.statistics_candidate_digest(), [0; 32]);
                            assert_ne!(family.observation_authority_id(), [0; 32]);
                            assert_eq!(
                                family.observation_identity(),
                                family.observation_authority_id()
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn write_fresh_reopen_exact_reuse_and_population_projection_are_exact() {
        let admission_root = TestRoot::new("reuse-admission");
        let finalization_root = TestRoot::new("reuse-finalization");
        let source = admission(
            admission_root.path(),
            AdmissionV4FamilyTerminal::InsufficientForCscv,
            AdmissionV4FamilyTerminal::Evaluated,
            31,
        );
        let mut written =
            commit_population_finalization_v4(finalization_root.path(), bounds(), source)
                .expect("write Finalization V4");
        assert!(matches!(
            &written,
            PopulationFinalizationV4Commit::Written(_)
        ));
        let receipt = written.authority_mut().structural_receipt();
        let population = written
            .authority_mut()
            .population_source()
            .expect("project Population successor");
        assert_eq!(population.receipt(), receipt);
        assert_eq!(
            population.source().admission_block_id(),
            receipt.admission_block_id()
        );
        assert_eq!(
            population.source().admission_completion_id(),
            receipt.admission_completion_id()
        );
        assert_eq!(population.source().decision_count(), 2);
        assert_eq!(population.decisions().len(), 2);
        assert_eq!(population.decisions()[0].decision_sequence(), 0);
        assert_eq!(population.decisions()[0].statistics_sequence(), 1);
        assert_eq!(population.decisions()[1].decision_sequence(), 1);
        assert_eq!(population.decisions()[1].statistics_sequence(), 2);
        for decision in population.decisions() {
            assert_eq!(decision.family(), AdmissionV4Family::BankNifty);
            assert_eq!(decision.status(), AdmissionV4DecisionStatus::Admitted);
            assert_eq!(decision.verdict().status(), AdmissionStatusV1::Admitted);
            assert_ne!(decision.row_id(), [0; 32]);
            assert_ne!(decision.admission_decision_id(), [0; 32]);
            assert_eq!(
                decision.pre_admission_authority_id(),
                population.families()[1].pre_admission_authority_id()
            );
            assert_eq!(
                decision.search_member_id(),
                population.families()[1].search_member_id()
            );
        }
        let reused_source = admission(
            admission_root.path(),
            AdmissionV4FamilyTerminal::InsufficientForCscv,
            AdmissionV4FamilyTerminal::Evaluated,
            31,
        );
        let reused =
            commit_population_finalization_v4(finalization_root.path(), bounds(), reused_source)
                .expect("reuse Finalization V4");
        assert!(matches!(&reused, PopulationFinalizationV4Commit::Reused(_)));
        assert_eq!(reused.into_authority().structural_receipt(), receipt);
    }

    #[test]
    fn every_exact_trailing_prefix_recovers_but_foreign_and_ragged_refuse() {
        for prefix in 1..6 {
            let admission_root = TestRoot::new("prefix-admission");
            let finalization_root = TestRoot::new("prefix-finalization");
            let mut source = admission(
                admission_root.path(),
                AdmissionV4FamilyTerminal::Evaluated,
                AdmissionV4FamilyTerminal::NaturallyExtinct,
                41,
            );
            let prepared = prepare_population_finalization_v4(&mut source)
                .expect("prepare Finalization prefix");
            let records = encoded_block(&prepared, 0).expect("encode Finalization prefix");
            assert_eq!(records.len(), 6);
            write_prefix(finalization_root.path(), &records, prefix);
            let committed =
                commit_population_finalization_v4(finalization_root.path(), bounds(), source)
                    .expect("recover exact Finalization prefix");
            assert!(matches!(
                committed,
                PopulationFinalizationV4Commit::Written(_)
            ));
            let metadata = std::fs::metadata(finalization_root.path().join(DATA_FILE))
                .expect("stat completed Finalization data");
            assert_eq!(
                metadata.len(),
                HEADER_BYTES as u64 + 6 * RECORD_BYTES as u64
            );
        }

        let foreign_admission_root = TestRoot::new("foreign-admission");
        let foreign_finalization_root = TestRoot::new("foreign-finalization");
        let mut prefix_source = admission(
            foreign_admission_root.path(),
            AdmissionV4FamilyTerminal::Evaluated,
            AdmissionV4FamilyTerminal::NaturallyExtinct,
            42,
        );
        let prefix_prepared =
            prepare_population_finalization_v4(&mut prefix_source).expect("prepare foreign prefix");
        let records = encoded_block(&prefix_prepared, 0).expect("encode foreign prefix");
        write_prefix(foreign_finalization_root.path(), &records, 3);
        let other_admission_root = TestRoot::new("other-admission");
        let other = admission(
            other_admission_root.path(),
            AdmissionV4FamilyTerminal::Evaluated,
            AdmissionV4FamilyTerminal::NaturallyExtinct,
            43,
        );
        assert!(
            commit_population_finalization_v4(foreign_finalization_root.path(), bounds(), other,)
                .is_err()
        );

        let ragged = TestRoot::new("ragged");
        let mut file = File::create(ragged.path().join(DATA_FILE)).expect("create ragged data");
        file.write_all(&header()).expect("write ragged header");
        file.write_all(&[1]).expect("write ragged byte");
        file.sync_all().expect("sync ragged file");
        let ragged_admission_root = TestRoot::new("ragged-admission");
        let ragged_source = admission(
            ragged_admission_root.path(),
            AdmissionV4FamilyTerminal::NaturallyExtinct,
            AdmissionV4FamilyTerminal::NaturallyExtinct,
            44,
        );
        assert!(commit_population_finalization_v4(ragged.path(), bounds(), ragged_source).is_err());
    }

    #[test]
    fn policy_family_duplicate_corruption_and_resealed_crosswires_refuse() {
        let admission_root = TestRoot::new("semantic-admission");
        let mut source = admission(
            admission_root.path(),
            AdmissionV4FamilyTerminal::Evaluated,
            AdmissionV4FamilyTerminal::Evaluated,
            51,
        );
        let prepared =
            prepare_population_finalization_v4(&mut source).expect("prepare semantic fixture");

        let mut reordered = prepared.clone();
        reordered.families.swap(0, 1);
        assert!(reordered.validate().is_err());

        let mut foreign_policy = prepared.clone();
        foreign_policy.source.policy_digest[0] ^= 1;
        assert!(foreign_policy.validate().is_err());

        let mut duplicate = prepared.clone();
        duplicate.decisions[1].candidate_semantic_id = duplicate.decisions[0].candidate_semantic_id;
        duplicate.decisions[1].row_id =
            derive_decision_row_id(&duplicate.decisions[1]).expect("rederive duplicate row");
        assert!(duplicate.validate().is_err());

        let corrupt_admission_root = TestRoot::new("corrupt-admission");
        let corrupt_root = TestRoot::new("corrupt-finalization");
        let corrupt_source = admission(
            corrupt_admission_root.path(),
            AdmissionV4FamilyTerminal::Evaluated,
            AdmissionV4FamilyTerminal::NaturallyExtinct,
            52,
        );
        commit_population_finalization_v4(corrupt_root.path(), bounds(), corrupt_source)
            .expect("commit corruption fixture");
        let mut raw = {
            let mut file = File::open(corrupt_root.path().join(DATA_FILE))
                .expect("open corrupt Finalization data");
            read_record_at(&mut file, 1).expect("read family record")
        };
        raw[100] ^= 1;
        rewrite_record(corrupt_root.path(), 1, &raw);
        assert!(PopulationFinalizationV4Ledger::open_read(corrupt_root.path(), bounds()).is_err());

        let resealed_admission_root = TestRoot::new("resealed-admission");
        let resealed_root = TestRoot::new("resealed-finalization");
        let resealed_source = admission(
            resealed_admission_root.path(),
            AdmissionV4FamilyTerminal::Evaluated,
            AdmissionV4FamilyTerminal::NaturallyExtinct,
            53,
        );
        commit_population_finalization_v4(resealed_root.path(), bounds(), resealed_source)
            .expect("commit resealed fixture");
        let mut family = {
            let mut file = File::open(resealed_root.path().join(DATA_FILE))
                .expect("open resealed Finalization data");
            let (_, family) = decode_family_record(
                &read_record_at(&mut file, 1).expect("read resealed family record"),
            )
            .expect("decode resealed family record");
            family
        };
        family.search_member_id[0] ^= 1;
        family.row_id = derive_family_row_id(&family).expect("rederive resealed family row");
        let raw = encode_family(&family, 0).expect("reseal family record");
        rewrite_record(resealed_root.path(), 1, &raw);
        assert!(PopulationFinalizationV4Ledger::open_read(resealed_root.path(), bounds()).is_err());
    }

    #[test]
    fn stale_same_length_and_path_replacement_refuse_retained_authority() {
        let admission_root = TestRoot::new("stale-source-admission");
        let finalization_root = TestRoot::new("stale-source-finalization");
        let source = admission(
            admission_root.path(),
            AdmissionV4FamilyTerminal::Evaluated,
            AdmissionV4FamilyTerminal::NaturallyExtinct,
            61,
        );
        let mut committed =
            commit_population_finalization_v4(finalization_root.path(), bounds(), source)
                .expect("commit stale-source fixture")
                .into_authority();
        let admission_path = admission_root.path().join("population-admission-v4.bin");
        let mut admission_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&admission_path)
            .expect("open Admission source for stale mutation");
        admission_file
            .seek(SeekFrom::Start(80))
            .expect("seek Admission source");
        let mut byte = [0_u8; 1];
        admission_file
            .read_exact(&mut byte)
            .expect("read Admission source byte");
        admission_file
            .seek(SeekFrom::Start(80))
            .expect("rewind Admission source");
        byte[0] ^= 1;
        admission_file
            .write_all(&byte)
            .expect("mutate Admission source byte");
        admission_file
            .sync_all()
            .expect("sync Admission source mutation");
        assert!(committed.population_source().is_err());

        let replacement_admission_root = TestRoot::new("replacement-admission");
        let replacement_root = TestRoot::new("replacement-finalization");
        let replacement_source = admission(
            replacement_admission_root.path(),
            AdmissionV4FamilyTerminal::NaturallyExtinct,
            AdmissionV4FamilyTerminal::NaturallyExtinct,
            62,
        );
        let mut replacement = commit_population_finalization_v4(
            replacement_root.path(),
            bounds(),
            replacement_source,
        )
        .expect("commit replacement fixture")
        .into_authority();
        let data_path = replacement_root.path().join(DATA_FILE);
        let backup_path = replacement_root
            .path()
            .join("population-finalization-v4.backup");
        let bytes = std::fs::read(&data_path).expect("read Finalization replacement bytes");
        std::fs::rename(&data_path, &backup_path).expect("move held Finalization path");
        let mut replacement_file = File::create(&data_path).expect("replace Finalization path");
        replacement_file
            .write_all(&bytes)
            .expect("write replacement Finalization bytes");
        replacement_file
            .sync_all()
            .expect("sync replacement Finalization bytes");
        assert!(replacement.population_source().is_err());
    }
}
