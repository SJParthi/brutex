//! Source-retaining scaffold for the terminal-aware Population V6 successor.
//!
//! Population V5 is tied to Finalization V3 and remains byte-for-byte
//! unchanged.  This seam owns the opaque Finalization V4 authority together
//! with the exact zero, one or two stored Candidate/Pre-Admission authorities
//! needed to recover literal evaluated Candidate V1 records. Its independent fixed codec accepts
//! the four reachable Evaluated/NaturallyExtinct terminal pairs and explicitly
//! refuses every pair containing `InsufficientForCscv`: Candidate V1 expands
//! every closed mask into at least one Long and one Short execution-coordinate
//! row, so a truthful nonempty one-row Candidate receipt cannot be produced.
//!
//! Authentication is whole-source work, not O(1): every retained Candidate
//! ledger and Finalization are freshly revalidated before and after the joined
//! snapshot.

#![expect(
    dead_code,
    reason = "Population V6 source retention and fixed receipt-last codec await their all-rung production caller"
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
use runner::admission::{AdmissionStatusV1, AdmissionV3ArithmeticProjection};

use crate::candidate_universe::{
    AuthenticatedCandidatePopulationRowV1, CANDIDATE_ROW_STRIDE_V1,
    CandidateExecutionParameterFactsV1, CandidateUniverseReceiptV1,
    verify_population_candidate_canonical_record_v1,
};
use crate::population::{InstrumentFamilyV1, RequestedSpanIdentityV1};
use crate::population_admission_v4::{
    AdmissionV4DecisionStatus, AdmissionV4Family, AdmissionV4FamilyTerminal,
};
use crate::population_finalization_v4::{
    CommittedStoredPopulationFinalizationV4, PopulationFinalizationV4DecisionProjection,
    PopulationFinalizationV4FamilyProjection, PopulationFinalizationV4PopulationSourceV1,
};
use crate::step3_orchestrator::CommittedStoredCandidatePreAdmissionV1;
use runner::exit_grid_policy::ExecutionDispositionV1;

pub(crate) type PopulationV6Refusal = String;

pub(crate) const POPULATION_V6_HEADER_BYTES: u64 = 64;
pub(crate) const POPULATION_V6_RECORD_BYTES: u64 = 4_096;

const HEADER_BYTES: usize = 64;
const RECORD_BYTES: usize = 4_096;
const RECORD_BYTES_U32: u32 = 4_096;
const PAYLOAD_BYTES: usize = RECORD_BYTES - 32;
/// [`CANDIDATE_ROW_STRIDE_V1`] as the `usize` a record read needs.
///
/// # Why an `allow` is honest here and nowhere else
///
/// `usize::try_from` is not callable in a `const`, so the cast cannot be made
/// fallible at this site. What CAN be done is prove it, and the assert below
/// runs at compile time: a stride that no longer fits a `usize` is a build
/// failure, not a truncated constant that silently mis-strides every record
/// read after it. The lint's concern is discharged rather than silenced, and
/// the `allow` is scoped to this one constant, so the next unproved cast in
/// this file still fails the build.
#[allow(
    clippy::cast_possible_truncation,
    reason = "discharged by the const assert on the line below, which is a \
              compile-time check rather than a comment claiming one."
)]
const CANDIDATE_RECORD_BYTES: usize = {
    assert!(CANDIDATE_ROW_STRIDE_V1 <= usize::MAX as u64);
    CANDIDATE_ROW_STRIDE_V1 as usize
};
const RUNNER_POLICY_BYTES: usize = runner::admission::ADMISSION_POLICY_CANONICAL_LEN_V1;
const RUNNER_DECISION_BYTES: usize = runner::admission::ADMISSION_DECISION_CANONICAL_LEN_V3;
const RUNNER_HEADER_BYTES: usize = 12;
const RUNNER_POLICY_END: usize = RUNNER_HEADER_BYTES + RUNNER_POLICY_BYTES;
const VERSION: u32 = 6;
const DATA_KIND: u32 = 1;
const FAMILY_KIND: u32 = 2;
const CANDIDATE_KIND: u32 = 3;
const COMPLETION_KIND: u32 = 4;
const HEADER_MAGIC: [u8; 16] = *b"BTX-POPV6---HDR\0";
const RECORD_MAGIC: [u8; 16] = *b"BTX-POPV6---REC\0";
const DATA_FILE: &str = "population-v6.bin";
const LOCK_FILE: &str = "population-v6.lock";
const HEADER_DOMAIN: &[u8] = b"brutex-population-v6-header\0";
const RECORD_DOMAIN: &[u8] = b"brutex-population-v6-record\0";
const POPULATION_DOMAIN: &[u8] = b"brutex-population-v6-id\0";
const FAMILY_ROW_DOMAIN: &[u8] = b"brutex-population-v6-family-row\0";
const CANDIDATE_ROW_DOMAIN: &[u8] = b"brutex-population-v6-candidate-row\0";
const ORDERED_FAMILIES_DOMAIN: &[u8] = b"brutex-population-v6-families\0";
const ORDERED_CANDIDATES_DOMAIN: &[u8] = b"brutex-population-v6-candidates\0";
const COMPLETION_DOMAIN: &[u8] = b"brutex-population-v6-completion\0";
const POLICY_DOMAIN: &[u8] = b"brutex-population-admission-v4-policy\0";
const GENERATION_DOMAIN: &[u8] = b"brutex-population-v6-generation\0";
const READ_CHUNK_BYTES: usize = 16 * 1_024;

#[cfg(any(target_os = "android", target_os = "linux"))]
const O_NOFOLLOW_FLAG: i32 = 0x20_000;
#[cfg(target_os = "macos")]
const O_NOFOLLOW_FLAG: i32 = 0x100;

const _: () = assert!(PAYLOAD_BYTES + 32 == RECORD_BYTES);

/// Explicit nonzero authority, Candidate-row and file ceilings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationV6Bounds {
    authorities: u64,
    candidates_per_authority: u64,
    file_bytes: u64,
}

impl PopulationV6Bounds {
    /// Creates explicit bounds; there is intentionally no `Default`.
    pub(crate) fn new(
        authorities: u64,
        candidates_per_authority: u64,
        file_bytes: u64,
    ) -> Result<Self, PopulationV6Refusal> {
        let minimum = POPULATION_V6_HEADER_BYTES
            .checked_add(4 * POPULATION_V6_RECORD_BYTES)
            .ok_or_else(|| "Population V6 minimum byte bound overflowed".to_owned())?;
        if authorities == 0 || candidates_per_authority == 0 || file_bytes < minimum {
            return Err(format!(
                "Population V6 bounds require nonzero authorities/Candidates and at least {minimum} bytes"
            ));
        }
        Ok(Self {
            authorities,
            candidates_per_authority,
            file_bytes,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceRecordV6 {
    finalization_sequence: u64,
    finalization_first_record: u64,
    finalization_id: [u8; 32],
    finalization_completion_id: [u8; 32],
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
    nifty_terminal: AdmissionV4FamilyTerminal,
    banknifty_terminal: AdmissionV4FamilyTerminal,
    nifty_candidate_count: u64,
    banknifty_candidate_count: u64,
    candidate_count: u64,
    evaluated_count: u64,
    decision_count: u64,
    population_id: [u8; 32],
}

impl SourceRecordV6 {
    fn from_upstream(
        value: &AuthenticatedPopulationV6UpstreamV1,
    ) -> Result<Self, PopulationV6Refusal> {
        let receipt = value.finalization.receipt();
        let source = value.finalization.source();
        let [nifty, banknifty] = value.finalization.families();
        let nifty_count = nifty.candidate_count();
        let banknifty_count = banknifty.candidate_count();
        let candidate_count = nifty_count
            .checked_add(banknifty_count)
            .ok_or_else(|| "Population V6 Candidate count overflowed".to_owned())?;
        let mut result = Self {
            finalization_sequence: receipt.sequence(),
            finalization_first_record: receipt.first_record(),
            finalization_id: receipt.finalization_id(),
            finalization_completion_id: receipt.completion_id(),
            admission_sequence: source.admission_sequence(),
            admission_first_record: source.admission_first_record(),
            admission_block_id: source.admission_block_id(),
            admission_completion_id: source.admission_completion_id(),
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
            nifty_terminal: nifty.terminal(),
            banknifty_terminal: banknifty.terminal(),
            nifty_candidate_count: nifty_count,
            banknifty_candidate_count: banknifty_count,
            candidate_count,
            evaluated_count: candidate_count,
            decision_count: source.decision_count(),
            population_id: [0; 32],
        };
        result.population_id = derive_population_id(&result, nifty, banknifty);
        result.validate()?;
        Ok(result)
    }

    fn validate(&self) -> Result<(), PopulationV6Refusal> {
        if self.rung_seconds == 0 || self.horizon_bars == 0 {
            return Err("Population V6 rung/horizon is zero".to_owned());
        }
        RequestedSpanIdentityV1::new(
            self.requested_span.from_year(),
            self.requested_span.from_month(),
            self.requested_span.to_year(),
            self.requested_span.to_month(),
        )?;
        for (name, value) in [
            ("Finalization", self.finalization_id),
            ("Finalization Completion", self.finalization_completion_id),
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
            ("Population", self.population_id),
        ] {
            require_nonzero(name, value)?;
        }
        if self.policy_digest != hash_slices(POLICY_DOMAIN, &[&self.policy]) {
            return Err("Population V6 policy digest does not reproduce".to_owned());
        }
        let total = self
            .nifty_candidate_count
            .checked_add(self.banknifty_candidate_count)
            .ok_or_else(|| "Population V6 family Candidate count overflowed".to_owned())?;
        if total != self.candidate_count
            || self.evaluated_count != self.candidate_count
            || self.decision_count != self.candidate_count
        {
            return Err("Population V6 Candidate/evaluated/decision counts differ".to_owned());
        }
        validate_reachable_terminal(self.nifty_terminal, self.nifty_candidate_count)?;
        validate_reachable_terminal(self.banknifty_terminal, self.banknifty_candidate_count)
    }

    fn encode_body(&self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationV6Refusal> {
        writer.u64(self.finalization_sequence)?;
        writer.u64(self.finalization_first_record)?;
        writer.array(&self.finalization_id)?;
        writer.array(&self.finalization_completion_id)?;
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
        writer.zeros(2)?;
        for digest in [
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
            writer.array(&digest)?;
        }
        writer.array(&self.policy)?;
        writer.array(&self.policy_digest)?;
        writer.u8(self.nifty_terminal as u8)?;
        writer.u8(self.banknifty_terminal as u8)?;
        writer.zeros(6)?;
        writer.u64(self.nifty_candidate_count)?;
        writer.u64(self.banknifty_candidate_count)?;
        writer.u64(self.candidate_count)?;
        writer.u64(self.evaluated_count)?;
        writer.u64(self.decision_count)?;
        writer.array(&self.population_id)
    }

    fn decode_body(reader: &mut FixedReader<'_>) -> Result<Self, PopulationV6Refusal> {
        let finalization_sequence = reader.u64()?;
        let finalization_first_record = reader.u64()?;
        let finalization_id = reader.array()?;
        let finalization_completion_id = reader.array()?;
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
        reader.require_zero(2, "source span alignment")?;
        let value = Self {
            finalization_sequence,
            finalization_first_record,
            finalization_id,
            finalization_completion_id,
            admission_sequence,
            admission_first_record,
            admission_block_id,
            admission_completion_id,
            rung_seconds,
            horizon_bars,
            requested_span: RequestedSpanIdentityV1::new(from_year, from_month, to_year, to_month)?,
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
            nifty_terminal: decode_terminal(reader.u8()?)?,
            banknifty_terminal: decode_terminal(reader.u8()?)?,
            nifty_candidate_count: {
                reader.require_zero(6, "source terminal alignment")?;
                reader.u64()?
            },
            banknifty_candidate_count: reader.u64()?,
            candidate_count: reader.u64()?,
            evaluated_count: reader.u64()?,
            decision_count: reader.u64()?,
            population_id: reader.array()?,
        };
        value.validate()?;
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FamilyRecordV6 {
    population_id: [u8; 32],
    family_sequence: u64,
    family: AdmissionV4Family,
    terminal: AdmissionV4FamilyTerminal,
    finalization_family_row_id: [u8; 32],
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
    evaluated_count: u64,
    decision_count: u64,
    row_id: [u8; 32],
}

impl FamilyRecordV6 {
    fn from_projection(
        source: &SourceRecordV6,
        family_sequence: u64,
        value: &PopulationFinalizationV4FamilyProjection,
    ) -> Result<Self, PopulationV6Refusal> {
        let mut result = Self {
            population_id: source.population_id,
            family_sequence,
            family: value.family(),
            terminal: value.terminal(),
            finalization_family_row_id: value.row_id(),
            admission_family_id: value.admission_family_id(),
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
            evaluated_count: value.candidate_count(),
            decision_count: value.decision_count(),
            row_id: [0; 32],
        };
        result.row_id = derive_family_row_id(&result);
        result.validate(source, family_sequence)?;
        Ok(result)
    }

    /// Validates against the slot the CALLER read this record from.
    ///
    /// **`ordinal` is a parameter because it used to be `self.family_sequence`,
    /// and that made every expectation below a restatement of the record's own
    /// claim.** A BANKNIFTY record declaring `family_sequence: 1` validated
    /// perfectly while sitting in slot 0: the branch that chose "expect
    /// BANKNIFTY" was chosen BY the field the branch then went on to check. A
    /// complete block still caught the swap, but only downstream, through
    /// `ordered_family_digest` against the Completion record -- and a trailing
    /// PREFIX has no Completion, so `validate_prefix` caught nothing at all and
    /// `open_write` accepted a file whose second record was the wrong record.
    fn validate(&self, source: &SourceRecordV6, ordinal: u64) -> Result<(), PopulationV6Refusal> {
        let (expected_family, expected_terminal, expected_count) = if ordinal == 0 {
            (
                AdmissionV4Family::Nifty,
                source.nifty_terminal,
                source.nifty_candidate_count,
            )
        } else if ordinal == 1 {
            (
                AdmissionV4Family::BankNifty,
                source.banknifty_terminal,
                source.banknifty_candidate_count,
            )
        } else {
            return Err("Population V6 Family slot is neither NIFTY nor BANKNIFTY".to_owned());
        };
        if self.population_id != source.population_id
            || self.family_sequence != ordinal
            || self.family != expected_family
            || self.terminal != expected_terminal
            || self.candidate_count != expected_count
            || self.evaluated_count != self.candidate_count
            || self.decision_count != self.candidate_count
        {
            return Err("Population V6 Family crosswires its block/order/counts".to_owned());
        }
        for (name, value) in [
            ("Finalization Family row", self.finalization_family_row_id),
            ("Admission Family", self.admission_family_id),
            ("Statistics Family", self.statistics_family_id),
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
            || self.search_fold_count == 0
            || self.search_decided_folds > self.search_fold_count
            || self.search_profitable_oos_folds > self.search_decided_folds
        {
            return Err("Population V6 Family signal/Search shape differs".to_owned());
        }
        match self.terminal {
            AdmissionV4FamilyTerminal::Evaluated => {
                if self.candidate_count < 2
                    || self.observation_authority_id != [0; 32]
                    || self.observation_data_digest != [0; 32]
                    || self.observation_completion_digest != [0; 32]
                    || self.statistics_candidate_digest == [0; 32]
                    || self.statistics_period_digest == [0; 32]
                    || self.statistics_split_digest == [0; 32]
                {
                    return Err("Population V6 evaluated Family shape differs".to_owned());
                }
            }
            AdmissionV4FamilyTerminal::NaturallyExtinct => {
                if self.candidate_count != 0
                    || self.observation_authority_id == [0; 32]
                    || self.observation_identity != self.observation_authority_id
                    || self.observation_data_digest == [0; 32]
                    || self.observation_completion_digest == [0; 32]
                    || self.statistics_candidate_digest != [0; 32]
                    || self.statistics_period_digest != [0; 32]
                    || self.statistics_split_digest != [0; 32]
                {
                    return Err("Population V6 extinct Family loses or invents evidence".to_owned());
                }
            }
            AdmissionV4FamilyTerminal::InsufficientForCscv => {
                return Err(
                    "Population V6 refuses production-unreachable Candidate V1 singleton"
                        .to_owned(),
                );
            }
        }
        if self.row_id != derive_family_row_id(self) {
            return Err("Population V6 Family row identity does not reproduce".to_owned());
        }
        Ok(())
    }

    fn encode_body(&self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationV6Refusal> {
        writer.array(&self.population_id)?;
        writer.u64(self.family_sequence)?;
        writer.u8(self.family as u8)?;
        writer.u8(self.terminal as u8)?;
        writer.zeros(6)?;
        for digest in [
            self.finalization_family_row_id,
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
            writer.array(&digest)?;
        }
        writer.u64(self.signal_bars)?;
        writer.i64(self.signal_first_ts_micros)?;
        writer.i64(self.signal_last_ts_micros)?;
        for digest in [
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
            writer.array(&digest)?;
        }
        writer.u64(self.search_fold_count)?;
        writer.u64(self.search_decided_folds)?;
        writer.u64(self.search_profitable_oos_folds)?;
        writer.i64(self.search_aggregate_oos_paisa)?;
        writer.u64(self.search_evaluated_population_cells)?;
        for digest in [
            self.statistics_candidate_digest,
            self.statistics_period_digest,
            self.statistics_split_digest,
        ] {
            writer.array(&digest)?;
        }
        writer.u64(self.candidate_count)?;
        writer.u64(self.evaluated_count)?;
        writer.u64(self.decision_count)?;
        writer.array(&self.row_id)
    }

    fn decode_body(reader: &mut FixedReader<'_>) -> Result<Self, PopulationV6Refusal> {
        let population_id = reader.array()?;
        let family_sequence = reader.u64()?;
        let family = decode_family(reader.u8()?)?;
        let terminal = decode_terminal(reader.u8()?)?;
        reader.require_zero(6, "Family alignment")?;
        Ok(Self {
            population_id,
            family_sequence,
            family,
            terminal,
            finalization_family_row_id: reader.array()?,
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
            evaluated_count: reader.u64()?,
            decision_count: reader.u64()?,
            row_id: reader.array()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CandidateRecordV6 {
    population_id: [u8; 32],
    global_sequence: u64,
    family: AdmissionV4Family,
    family_sequence: u64,
    finalization_family_row_id: [u8; 32],
    candidate_universe_id: [u8; 32],
    candidate_completion_digest: [u8; 32],
    candidate_semantic_id: [u8; 32],
    candidate_row_digest: [u8; 32],
    candidate_record: [u8; CANDIDATE_RECORD_BYTES],
    finalization_row_id: [u8; 32],
    finalization_decision_sequence: u64,
    statistics_sequence: u64,
    status: AdmissionV4DecisionStatus,
    admission_decision_id: [u8; 32],
    pre_admission_authority_id: [u8; 32],
    statistics_period_digest: [u8; 32],
    statistics_split_digest: [u8; 32],
    search_member_id: [u8; 32],
    base_evidence_id: [u8; 32],
    runner_decision: [u8; RUNNER_DECISION_BYTES],
    row_id: [u8; 32],
}

impl CandidateRecordV6 {
    fn from_join(
        source: &SourceRecordV6,
        families: &[FamilyRecordV6; 2],
        global_sequence: u64,
        candidate: &AuthenticatedCandidatePopulationRowV1,
        decision: &PopulationFinalizationV4DecisionProjection,
    ) -> Result<Self, PopulationV6Refusal> {
        let row = candidate.row();
        let family = match row.family() {
            InstrumentFamilyV1::Nifty => AdmissionV4Family::Nifty,
            InstrumentFamilyV1::BankNifty => AdmissionV4Family::BankNifty,
        };
        let family_index = match family {
            AdmissionV4Family::Nifty => 0,
            AdmissionV4Family::BankNifty => 1,
        };
        let family_record = families
            .get(family_index)
            .ok_or_else(|| "Population V6 Candidate family is absent".to_owned())?;
        let mut result = Self {
            population_id: source.population_id,
            global_sequence,
            family,
            family_sequence: row.sequence(),
            finalization_family_row_id: family_record.finalization_family_row_id,
            candidate_universe_id: row.universe_id(),
            candidate_completion_digest: family_record.candidate_completion_digest,
            candidate_semantic_id: row.candidate_semantic_digest(),
            candidate_row_digest: candidate.base_candidate_row_digest(),
            candidate_record: *candidate.canonical_record(),
            finalization_row_id: decision.row_id(),
            finalization_decision_sequence: decision.decision_sequence(),
            statistics_sequence: decision.statistics_sequence(),
            status: decision.status(),
            admission_decision_id: decision.admission_decision_id(),
            pre_admission_authority_id: decision.pre_admission_authority_id(),
            statistics_period_digest: decision.statistics_period_digest(),
            statistics_split_digest: decision.statistics_split_digest(),
            search_member_id: decision.search_member_id(),
            base_evidence_id: decision.base_evidence_id(),
            runner_decision: *decision.runner_decision(),
            row_id: [0; 32],
        };
        result.row_id = derive_candidate_row_id(&result);
        result.validate(source, families)?;
        Ok(result)
    }

    fn validate(
        &self,
        source: &SourceRecordV6,
        families: &[FamilyRecordV6; 2],
    ) -> Result<(), PopulationV6Refusal> {
        let (expected_family, expected_family_sequence, family_index) =
            if self.global_sequence < source.nifty_candidate_count {
                (AdmissionV4Family::Nifty, self.global_sequence, 0_usize)
            } else {
                (
                    AdmissionV4Family::BankNifty,
                    self.global_sequence
                        .checked_sub(source.nifty_candidate_count)
                        .ok_or_else(|| "Population V6 family sequence underflowed".to_owned())?,
                    1_usize,
                )
            };
        let family = families
            .get(family_index)
            .ok_or_else(|| "Population V6 Candidate family record is absent".to_owned())?;
        if self.population_id != source.population_id
            || self.global_sequence >= source.candidate_count
            || self.family != expected_family
            || self.family_sequence != expected_family_sequence
            || family.terminal != AdmissionV4FamilyTerminal::Evaluated
            || self.finalization_family_row_id != family.finalization_family_row_id
            || self.candidate_universe_id != family.candidate_universe_id
            || self.candidate_completion_digest != family.candidate_completion_digest
            || self.finalization_decision_sequence != self.global_sequence
            || self.statistics_sequence != self.global_sequence
            || self.pre_admission_authority_id != family.pre_admission_authority_id
            || self.search_member_id != family.search_member_id
        {
            return Err("Population V6 Candidate crosswires its order/family lineage".to_owned());
        }
        for (name, value) in [
            ("Candidate semantic", self.candidate_semantic_id),
            ("Candidate row", self.candidate_row_digest),
            ("Finalization row", self.finalization_row_id),
            ("Admission decision", self.admission_decision_id),
            ("Pre-Admission", self.pre_admission_authority_id),
            ("Statistics period", self.statistics_period_digest),
            ("Statistics split", self.statistics_split_digest),
            ("Search member", self.search_member_id),
            ("Base Evidence", self.base_evidence_id),
            ("Population Candidate row", self.row_id),
        ] {
            require_nonzero(name, value)?;
        }
        let authenticated = verify_population_candidate_canonical_record_v1(&self.candidate_record)
            .map_err(|why| format!("Population V6 nested Candidate refused: {why}"))?;
        let candidate = authenticated.row();
        let expected_instrument = match self.family {
            AdmissionV4Family::Nifty => InstrumentFamilyV1::Nifty,
            AdmissionV4Family::BankNifty => InstrumentFamilyV1::BankNifty,
        };
        if candidate.universe_id() != self.candidate_universe_id
            || candidate.sequence() != self.family_sequence
            || candidate.family() != expected_instrument
            || candidate.rung_seconds() != source.rung_seconds
            || candidate.horizon_bars() != source.horizon_bars
            || candidate.candidate_semantic_digest() != self.candidate_semantic_id
            || authenticated.base_candidate_row_digest() != self.candidate_row_digest
        {
            return Err("Population V6 nested Candidate identity differs".to_owned());
        }
        let policy = self
            .runner_decision
            .get(RUNNER_HEADER_BYTES..RUNNER_POLICY_END)
            .ok_or_else(|| "Population V6 Runner policy is absent".to_owned())?;
        if policy != source.policy {
            return Err("Population V6 Runner policy differs from source".to_owned());
        }
        let arithmetic =
            AdmissionV3ArithmeticProjection::verify_decision_record_detached(&self.runner_decision)
                .map_err(|why| format!("Population V6 Runner arithmetic refused: {why:?}"))?;
        if self.status != status_from_runner(arithmetic.status()) {
            return Err("Population V6 status differs from Runner bytes".to_owned());
        }
        if self.row_id != derive_candidate_row_id(self) {
            return Err("Population V6 Candidate row identity does not reproduce".to_owned());
        }
        Ok(())
    }

    fn encode_body(&self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationV6Refusal> {
        writer.array(&self.population_id)?;
        writer.u64(self.global_sequence)?;
        writer.u8(self.family as u8)?;
        writer.u8(self.status as u8)?;
        writer.zeros(6)?;
        writer.u64(self.family_sequence)?;
        for digest in [
            self.finalization_family_row_id,
            self.candidate_universe_id,
            self.candidate_completion_digest,
            self.candidate_semantic_id,
            self.candidate_row_digest,
        ] {
            writer.array(&digest)?;
        }
        writer.array(&self.candidate_record)?;
        writer.array(&self.finalization_row_id)?;
        writer.u64(self.finalization_decision_sequence)?;
        writer.u64(self.statistics_sequence)?;
        for digest in [
            self.admission_decision_id,
            self.pre_admission_authority_id,
            self.statistics_period_digest,
            self.statistics_split_digest,
            self.search_member_id,
            self.base_evidence_id,
        ] {
            writer.array(&digest)?;
        }
        writer.array(&self.runner_decision)?;
        writer.array(&self.row_id)
    }

    fn decode_body(reader: &mut FixedReader<'_>) -> Result<Self, PopulationV6Refusal> {
        let population_id = reader.array()?;
        let global_sequence = reader.u64()?;
        let family = decode_family(reader.u8()?)?;
        let status = decode_status(reader.u8()?)?;
        reader.require_zero(6, "Candidate alignment")?;
        Ok(Self {
            population_id,
            global_sequence,
            family,
            status,
            family_sequence: reader.u64()?,
            finalization_family_row_id: reader.array()?,
            candidate_universe_id: reader.array()?,
            candidate_completion_digest: reader.array()?,
            candidate_semantic_id: reader.array()?,
            candidate_row_digest: reader.array()?,
            candidate_record: reader.array()?,
            finalization_row_id: reader.array()?,
            finalization_decision_sequence: reader.u64()?,
            statistics_sequence: reader.u64()?,
            admission_decision_id: reader.array()?,
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
struct PreparedPopulationV6 {
    source: SourceRecordV6,
    families: [FamilyRecordV6; 2],
    candidates: Vec<CandidateRecordV6>,
}

impl PreparedPopulationV6 {
    fn from_upstream(
        value: AuthenticatedPopulationV6UpstreamV1,
        bounds: PopulationV6Bounds,
    ) -> Result<Self, PopulationV6Refusal> {
        validate_candidate_pair(
            &value.finalization,
            value.nifty.as_ref(),
            value.banknifty.as_ref(),
        )?;
        let source = SourceRecordV6::from_upstream(&value)?;
        if source.candidate_count > bounds.candidates_per_authority {
            return Err("Population V6 source exceeds Candidate bound".to_owned());
        }
        let [nifty_projection, banknifty_projection] = value.finalization.families();
        let families = [
            FamilyRecordV6::from_projection(&source, 0, nifty_projection)?,
            FamilyRecordV6::from_projection(&source, 1, banknifty_projection)?,
        ];
        let capacity = usize::try_from(source.candidate_count)
            .map_err(|_| "Population V6 Candidate count does not fit usize".to_owned())?;
        let mut authenticated = Vec::new();
        authenticated
            .try_reserve_exact(capacity)
            .map_err(|why| format!("cannot reserve Population V6 Candidate inputs: {why}"))?;
        if let Some(nifty) = value.nifty {
            authenticated.extend(nifty.rows);
        }
        if let Some(banknifty) = value.banknifty {
            authenticated.extend(banknifty.rows);
        }
        if authenticated.len() != value.finalization.decisions().len() {
            return Err(
                "Population V6 Candidate and Finalization decision cardinalities differ".to_owned(),
            );
        }
        let mut candidates = Vec::new();
        candidates
            .try_reserve_exact(capacity)
            .map_err(|why| format!("cannot reserve Population V6 records: {why}"))?;
        for (index, (candidate, decision)) in authenticated
            .iter()
            .zip(value.finalization.decisions())
            .enumerate()
        {
            let global_sequence = u64::try_from(index)
                .map_err(|_| "Population V6 global sequence does not fit u64".to_owned())?;
            candidates.push(CandidateRecordV6::from_join(
                &source,
                &families,
                global_sequence,
                candidate,
                decision,
            )?);
        }
        let result = Self {
            source,
            families,
            candidates,
        };
        result.validate(bounds)?;
        Ok(result)
    }

    fn validate(&self, bounds: PopulationV6Bounds) -> Result<(), PopulationV6Refusal> {
        self.source.validate()?;
        if self.source.population_id
            != derive_population_id_from_family_records(&self.source, &self.families)
        {
            return Err(
                "Population V6 identity does not reproduce from its exact Families".to_owned(),
            );
        }
        // Enumerated, not iterated: the slot is the authority for which family
        // belongs here, and the record's own `family_sequence` is not.
        for (ordinal, family) in self.families.iter().enumerate() {
            let ordinal = u64::try_from(ordinal)
                .map_err(|_| "Population V6 Family ordinal does not fit u64".to_owned())?;
            family.validate(&self.source, ordinal)?;
        }
        let count = u64::try_from(self.candidates.len())
            .map_err(|_| "Population V6 Candidate length does not fit u64".to_owned())?;
        if count != self.source.candidate_count || count > bounds.candidates_per_authority {
            return Err("Population V6 Candidate count differs or exceeds bound".to_owned());
        }
        let capacity = self.candidates.len();
        let mut population_rows = bounded_set(capacity, "Population rows")?;
        let mut semantics = bounded_set(capacity, "Candidate semantics")?;
        let mut candidate_rows = bounded_set(capacity, "Candidate rows")?;
        let mut finalization_rows = bounded_set(capacity, "Finalization rows")?;
        let mut admission_decisions = bounded_set(capacity, "Admission decisions")?;
        let mut base_rows = bounded_set(capacity, "Base Evidence rows")?;
        let mut statuses = [0_u64; 4];
        for (index, candidate) in self.candidates.iter().enumerate() {
            let expected = u64::try_from(index)
                .map_err(|_| "Population V6 validation ordinal does not fit u64".to_owned())?;
            if candidate.global_sequence != expected {
                return Err("Population V6 Candidate sequence has a duplicate or gap".to_owned());
            }
            candidate.validate(&self.source, &self.families)?;
            for (name, inserted) in [
                ("Population row", population_rows.insert(candidate.row_id)),
                (
                    "Candidate semantic",
                    semantics.insert(candidate.candidate_semantic_id),
                ),
                (
                    "Candidate row",
                    candidate_rows.insert(candidate.candidate_row_digest),
                ),
                (
                    "Finalization row",
                    finalization_rows.insert(candidate.finalization_row_id),
                ),
                (
                    "Admission decision",
                    admission_decisions.insert(candidate.admission_decision_id),
                ),
                (
                    "Base Evidence row",
                    base_rows.insert(candidate.base_evidence_id),
                ),
            ] {
                if !inserted {
                    return Err(format!("Population V6 duplicates {name}"));
                }
            }
            let status_index = usize::from(candidate.status as u8 - 1);
            let slot = statuses
                .get_mut(status_index)
                .ok_or_else(|| "Population V6 status index is outside its census".to_owned())?;
            *slot = slot
                .checked_add(1)
                .ok_or_else(|| "Population V6 status count overflowed".to_owned())?;
        }
        if checked_sum(&statuses, "Population V6 status counts")? != self.source.decision_count {
            return Err("Population V6 status counts differ from decisions".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CompletionV6 {
    sequence: u64,
    source: SourceRecordV6,
    ordered_family_digest: [u8; 32],
    ordered_candidate_digest: [u8; 32],
    admitted_count: u64,
    rejected_count: u64,
    unmeasured_count: u64,
    refused_count: u64,
    completion_id: [u8; 32],
}

impl CompletionV6 {
    fn from_prepared(
        sequence: u64,
        value: &PreparedPopulationV6,
        bounds: PopulationV6Bounds,
    ) -> Result<Self, PopulationV6Refusal> {
        value.validate(bounds)?;
        let mut statuses = [0_u64; 4];
        for candidate in &value.candidates {
            let index = usize::from(candidate.status as u8 - 1);
            let slot = statuses
                .get_mut(index)
                .ok_or_else(|| "Population V6 completion status is unknown".to_owned())?;
            *slot = slot
                .checked_add(1)
                .ok_or_else(|| "Population V6 completion status overflowed".to_owned())?;
        }
        let mut result = Self {
            sequence,
            source: value.source.clone(),
            ordered_family_digest: ordered_family_digest(&value.families),
            ordered_candidate_digest: ordered_candidate_digest(&value.candidates),
            admitted_count: statuses[0],
            rejected_count: statuses[1],
            unmeasured_count: statuses[2],
            refused_count: statuses[3],
            completion_id: [0; 32],
        };
        result.completion_id = derive_completion_id(&result);
        result.validate()?;
        Ok(result)
    }

    fn validate(&self) -> Result<(), PopulationV6Refusal> {
        self.source.validate()?;
        require_nonzero("ordered Families", self.ordered_family_digest)?;
        require_nonzero("ordered Candidates", self.ordered_candidate_digest)?;
        require_nonzero("Completion", self.completion_id)?;
        let statuses = checked_sum(
            &[
                self.admitted_count,
                self.rejected_count,
                self.unmeasured_count,
                self.refused_count,
            ],
            "Population V6 Completion statuses",
        )?;
        if statuses != self.source.decision_count
            || self.completion_id != derive_completion_id(self)
        {
            return Err("Population V6 Completion does not reconcile".to_owned());
        }
        Ok(())
    }

    fn encode_body(&self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationV6Refusal> {
        self.source.encode_body(writer)?;
        writer.array(&self.ordered_family_digest)?;
        writer.array(&self.ordered_candidate_digest)?;
        writer.u64(self.admitted_count)?;
        writer.u64(self.rejected_count)?;
        writer.u64(self.unmeasured_count)?;
        writer.u64(self.refused_count)?;
        writer.array(&self.completion_id)
    }

    fn decode_body(
        sequence: u64,
        reader: &mut FixedReader<'_>,
    ) -> Result<Self, PopulationV6Refusal> {
        let value = Self {
            sequence,
            source: SourceRecordV6::decode_body(reader)?,
            ordered_family_digest: reader.array()?,
            ordered_candidate_digest: reader.array()?,
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

fn derive_population_id(
    value: &SourceRecordV6,
    nifty: &PopulationFinalizationV4FamilyProjection,
    banknifty: &PopulationFinalizationV4FamilyProjection,
) -> [u8; 32] {
    derive_population_id_from_parts(
        value,
        [
            (
                nifty.row_id(),
                nifty.candidate_universe_id(),
                nifty.candidate_completion_digest(),
                nifty.candidate_ordered_row_digest(),
            ),
            (
                banknifty.row_id(),
                banknifty.candidate_universe_id(),
                banknifty.candidate_completion_digest(),
                banknifty.candidate_ordered_row_digest(),
            ),
        ],
    )
}

fn derive_population_id_from_family_records(
    value: &SourceRecordV6,
    families: &[FamilyRecordV6; 2],
) -> [u8; 32] {
    derive_population_id_from_parts(
        value,
        families.map(|family| {
            (
                family.finalization_family_row_id,
                family.candidate_universe_id,
                family.candidate_completion_digest,
                family.candidate_ordered_row_digest,
            )
        }),
    )
}

/// The four 32-byte digests one family contributes to a Population V6 identity:
/// its candidate row, its universe, its completion and its ordering.
///
/// Named rather than spelled inline because the inline form is four identical
/// `[u8; 32]`s in a fixed order, and a reader has no way to tell from the type
/// which position means which digest -- nor would swapping two of them be a
/// compile error. The name does not fix that, but it gives the doc comment
/// somewhere to live.
type FamilyDigestsV6 = ([u8; 32], [u8; 32], [u8; 32], [u8; 32]);

fn derive_population_id_from_parts(
    value: &SourceRecordV6,
    families: [FamilyDigestsV6; 2],
) -> [u8; 32] {
    let [
        (nifty_row, nifty_universe, nifty_completion, nifty_ordered),
        (bank_row, bank_universe, bank_completion, bank_ordered),
    ] = families;
    let mut hasher = Hasher::new();
    hasher.update(POPULATION_DOMAIN);
    for digest in [
        value.finalization_id,
        value.finalization_completion_id,
        value.admission_block_id,
        value.admission_completion_id,
        value.statistics_authority_id,
        value.statistics_completion_digest,
        value.base_pair_id,
        value.search_pair_id,
        value.policy_digest,
        nifty_row,
        bank_row,
        nifty_universe,
        nifty_completion,
        nifty_ordered,
        bank_universe,
        bank_completion,
        bank_ordered,
    ] {
        hasher.update(&digest);
    }
    hasher.update(&[value.nifty_terminal as u8, value.banknifty_terminal as u8]);
    for count in [
        value.nifty_candidate_count,
        value.banknifty_candidate_count,
        value.candidate_count,
        value.evaluated_count,
        value.decision_count,
    ] {
        hasher.update(&count.to_le_bytes());
    }
    hasher.finalize()
}

fn derive_family_row_id(value: &FamilyRecordV6) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(FAMILY_ROW_DOMAIN);
    hasher.update(&value.population_id);
    hasher.update(&value.family_sequence.to_le_bytes());
    hasher.update(&[value.family as u8, value.terminal as u8]);
    for digest in [
        value.finalization_family_row_id,
        value.admission_family_id,
        value.statistics_family_id,
        value.candidate_universe_id,
        value.candidate_completion_digest,
        value.candidate_ordered_row_digest,
        value.pre_admission_authority_id,
        value.observation_authority_id,
        value.observation_identity,
        value.observation_policy_digest,
        value.observation_data_digest,
        value.observation_completion_digest,
        value.base_completion_id,
        value.signal_digest,
        value.signal_column_digest,
        value.long_grid_policy_digest,
        value.long_grid_resolution_digest,
        value.short_grid_policy_digest,
        value.short_grid_resolution_digest,
        value.grid_composite_digest,
        value.search_member_id,
        value.search_source_authority_id,
        value.search_validation_policy_id,
        value.search_validation_family_id,
        value.search_walk_id,
        value.statistics_candidate_digest,
        value.statistics_period_digest,
        value.statistics_split_digest,
    ] {
        hasher.update(&digest);
    }
    hasher.update(&value.signal_first_ts_micros.to_le_bytes());
    hasher.update(&value.signal_last_ts_micros.to_le_bytes());
    hasher.update(&value.search_aggregate_oos_paisa.to_le_bytes());
    for number in [
        value.signal_bars,
        value.search_fold_count,
        value.search_decided_folds,
        value.search_profitable_oos_folds,
        value.search_evaluated_population_cells,
        value.candidate_count,
        value.evaluated_count,
        value.decision_count,
    ] {
        hasher.update(&number.to_le_bytes());
    }
    hasher.finalize()
}

fn derive_candidate_row_id(value: &CandidateRecordV6) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(CANDIDATE_ROW_DOMAIN);
    hasher.update(&value.population_id);
    hasher.update(&value.global_sequence.to_le_bytes());
    hasher.update(&[value.family as u8, value.status as u8]);
    hasher.update(&value.family_sequence.to_le_bytes());
    for digest in [
        value.finalization_family_row_id,
        value.candidate_universe_id,
        value.candidate_completion_digest,
        value.candidate_semantic_id,
        value.candidate_row_digest,
        value.finalization_row_id,
        value.admission_decision_id,
        value.pre_admission_authority_id,
        value.statistics_period_digest,
        value.statistics_split_digest,
        value.search_member_id,
        value.base_evidence_id,
    ] {
        hasher.update(&digest);
    }
    hasher.update(&value.candidate_record);
    hasher.update(&value.finalization_decision_sequence.to_le_bytes());
    hasher.update(&value.statistics_sequence.to_le_bytes());
    hasher.update(&value.runner_decision);
    hasher.finalize()
}

fn ordered_family_digest(value: &[FamilyRecordV6; 2]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(ORDERED_FAMILIES_DOMAIN);
    hasher.update(&2_u64.to_le_bytes());
    for family in value {
        hasher.update(&family.row_id);
    }
    hasher.finalize()
}

fn ordered_candidate_digest(value: &[CandidateRecordV6]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(ORDERED_CANDIDATES_DOMAIN);
    hasher.update(&u64::try_from(value.len()).unwrap_or(u64::MAX).to_le_bytes());
    for candidate in value {
        hasher.update(&candidate.row_id);
    }
    hasher.finalize()
}

fn derive_completion_id(value: &CompletionV6) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(COMPLETION_DOMAIN);
    hasher.update(&value.sequence.to_le_bytes());
    hasher.update(&value.source.population_id);
    hasher.update(&value.source.finalization_id);
    hasher.update(&value.source.finalization_completion_id);
    hasher.update(&value.ordered_family_digest);
    hasher.update(&value.ordered_candidate_digest);
    for count in [
        value.source.candidate_count,
        value.source.evaluated_count,
        value.source.decision_count,
        value.admitted_count,
        value.rejected_count,
        value.unmeasured_count,
        value.refused_count,
    ] {
        hasher.update(&count.to_le_bytes());
    }
    hasher.finalize()
}

fn hash_slices(domain: &[u8], values: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    for value in values {
        hasher.update(value);
    }
    hasher.finalize()
}

fn require_nonzero(name: &str, value: [u8; 32]) -> Result<(), PopulationV6Refusal> {
    if value == [0; 32] {
        Err(format!("Population V6 {name} identity is zero"))
    } else {
        Ok(())
    }
}

fn checked_sum(values: &[u64], name: &str) -> Result<u64, PopulationV6Refusal> {
    values.iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(*value)
            .ok_or_else(|| format!("{name} overflowed u64"))
    })
}

fn bounded_set(capacity: usize, name: &str) -> Result<HashSet<[u8; 32]>, PopulationV6Refusal> {
    let mut set = HashSet::new();
    set.try_reserve(capacity)
        .map_err(|why| format!("cannot reserve Population V6 {name}: {why}"))?;
    Ok(set)
}

fn validate_reachable_terminal(
    terminal: AdmissionV4FamilyTerminal,
    candidate_count: u64,
) -> Result<(), PopulationV6Refusal> {
    match terminal {
        AdmissionV4FamilyTerminal::Evaluated if candidate_count >= 2 => Ok(()),
        AdmissionV4FamilyTerminal::NaturallyExtinct if candidate_count == 0 => Ok(()),
        AdmissionV4FamilyTerminal::InsufficientForCscv => {
            Err("Population V6 refuses production-unreachable Candidate V1 singleton".to_owned())
        }
        AdmissionV4FamilyTerminal::Evaluated => {
            Err("Population V6 evaluated family has fewer than two Candidate V1 rows".to_owned())
        }
        AdmissionV4FamilyTerminal::NaturallyExtinct => {
            Err("Population V6 extinct family carries Candidate V1 rows".to_owned())
        }
    }
}

fn status_from_runner(value: AdmissionStatusV1) -> AdmissionV4DecisionStatus {
    match value {
        AdmissionStatusV1::Admitted => AdmissionV4DecisionStatus::Admitted,
        AdmissionStatusV1::Rejected => AdmissionV4DecisionStatus::Rejected,
        AdmissionStatusV1::Unmeasured => AdmissionV4DecisionStatus::Unmeasured,
        AdmissionStatusV1::Refused => AdmissionV4DecisionStatus::Refused,
    }
}

fn decode_family(value: u8) -> Result<AdmissionV4Family, PopulationV6Refusal> {
    match value {
        1 => Ok(AdmissionV4Family::Nifty),
        2 => Ok(AdmissionV4Family::BankNifty),
        _ => Err(format!("Population V6 Family tag {value} is unknown")),
    }
}

fn decode_terminal(value: u8) -> Result<AdmissionV4FamilyTerminal, PopulationV6Refusal> {
    match value {
        1 => Ok(AdmissionV4FamilyTerminal::Evaluated),
        2 => Ok(AdmissionV4FamilyTerminal::InsufficientForCscv),
        3 => Ok(AdmissionV4FamilyTerminal::NaturallyExtinct),
        _ => Err(format!("Population V6 terminal tag {value} is unknown")),
    }
}

fn decode_status(value: u8) -> Result<AdmissionV4DecisionStatus, PopulationV6Refusal> {
    match value {
        1 => Ok(AdmissionV4DecisionStatus::Admitted),
        2 => Ok(AdmissionV4DecisionStatus::Rejected),
        3 => Ok(AdmissionV4DecisionStatus::Unmeasured),
        4 => Ok(AdmissionV4DecisionStatus::Refused),
        _ => Err(format!("Population V6 status tag {value} is unknown")),
    }
}

fn header() -> [u8; HEADER_BYTES] {
    let mut raw = [0_u8; HEADER_BYTES];
    raw[..16].copy_from_slice(&HEADER_MAGIC);
    raw[16..20].copy_from_slice(&VERSION.to_le_bytes());
    raw[20..24].copy_from_slice(&RECORD_BYTES_U32.to_le_bytes());
    let digest = hash_slices(HEADER_DOMAIN, &[&raw[..32]]);
    raw[32..64].copy_from_slice(&digest);
    raw
}

fn encode_manifest(
    value: &CompletionV6,
    kind: u32,
) -> Result<[u8; RECORD_BYTES], PopulationV6Refusal> {
    let mut raw = record_prefix(kind, value.sequence)?;
    let body = raw
        .get_mut(32..PAYLOAD_BYTES)
        .ok_or_else(|| "Population V6 manifest body is absent".to_owned())?;
    let mut writer = FixedWriter::new(body);
    value.encode_body(&mut writer)?;
    seal_record(&mut raw);
    Ok(raw)
}

fn encode_family(
    value: &FamilyRecordV6,
    sequence: u64,
) -> Result<[u8; RECORD_BYTES], PopulationV6Refusal> {
    let mut raw = record_prefix(FAMILY_KIND, sequence)?;
    let body = raw
        .get_mut(32..PAYLOAD_BYTES)
        .ok_or_else(|| "Population V6 Family body is absent".to_owned())?;
    let mut writer = FixedWriter::new(body);
    value.encode_body(&mut writer)?;
    seal_record(&mut raw);
    Ok(raw)
}

fn encode_candidate(
    value: &CandidateRecordV6,
    sequence: u64,
) -> Result<[u8; RECORD_BYTES], PopulationV6Refusal> {
    let mut raw = record_prefix(CANDIDATE_KIND, sequence)?;
    let body = raw
        .get_mut(32..PAYLOAD_BYTES)
        .ok_or_else(|| "Population V6 Candidate body is absent".to_owned())?;
    let mut writer = FixedWriter::new(body);
    value.encode_body(&mut writer)?;
    seal_record(&mut raw);
    Ok(raw)
}

fn record_prefix(kind: u32, sequence: u64) -> Result<[u8; RECORD_BYTES], PopulationV6Refusal> {
    if !matches!(
        kind,
        DATA_KIND | FAMILY_KIND | CANDIDATE_KIND | COMPLETION_KIND
    ) {
        return Err("Population V6 record kind is invalid".to_owned());
    }
    let mut raw = [0_u8; RECORD_BYTES];
    raw[..16].copy_from_slice(&RECORD_MAGIC);
    raw[16..20].copy_from_slice(&VERSION.to_le_bytes());
    raw[20..24].copy_from_slice(&kind.to_le_bytes());
    raw[24..32].copy_from_slice(&sequence.to_le_bytes());
    Ok(raw)
}

fn seal_record(raw: &mut [u8; RECORD_BYTES]) {
    let seal = hash_slices(RECORD_DOMAIN, &[&raw[..PAYLOAD_BYTES]]);
    raw[PAYLOAD_BYTES..].copy_from_slice(&seal);
}

fn decode_prefix(raw: &[u8; RECORD_BYTES], kind: u32) -> Result<u64, PopulationV6Refusal> {
    if raw[..16] != RECORD_MAGIC
        || u32::from_le_bytes(raw[16..20].try_into().map_err(|_| "version".to_owned())?) != VERSION
        || u32::from_le_bytes(raw[20..24].try_into().map_err(|_| "kind".to_owned())?) != kind
        || raw[PAYLOAD_BYTES..] != hash_slices(RECORD_DOMAIN, &[&raw[..PAYLOAD_BYTES]])
    {
        return Err("Population V6 record prefix or seal is invalid".to_owned());
    }
    Ok(u64::from_le_bytes(raw[24..32].try_into().map_err(
        |_| "Population V6 sequence bytes are absent".to_owned(),
    )?))
}

fn decode_manifest(
    raw: &[u8; RECORD_BYTES],
    kind: u32,
) -> Result<CompletionV6, PopulationV6Refusal> {
    let sequence = decode_prefix(raw, kind)?;
    let body = raw
        .get(32..PAYLOAD_BYTES)
        .ok_or_else(|| "Population V6 manifest body is absent".to_owned())?;
    let mut reader = FixedReader::new(body);
    let value = CompletionV6::decode_body(sequence, &mut reader)?;
    reader.require_remaining_zero("manifest reserve")?;
    Ok(value)
}

fn decode_family_record(
    raw: &[u8; RECORD_BYTES],
) -> Result<(u64, FamilyRecordV6), PopulationV6Refusal> {
    let sequence = decode_prefix(raw, FAMILY_KIND)?;
    let body = raw
        .get(32..PAYLOAD_BYTES)
        .ok_or_else(|| "Population V6 Family body is absent".to_owned())?;
    let mut reader = FixedReader::new(body);
    let value = FamilyRecordV6::decode_body(&mut reader)?;
    reader.require_remaining_zero("Family reserve")?;
    Ok((sequence, value))
}

fn decode_candidate_record(
    raw: &[u8; RECORD_BYTES],
) -> Result<(u64, CandidateRecordV6), PopulationV6Refusal> {
    let sequence = decode_prefix(raw, CANDIDATE_KIND)?;
    let body = raw
        .get(32..PAYLOAD_BYTES)
        .ok_or_else(|| "Population V6 Candidate body is absent".to_owned())?;
    let mut reader = FixedReader::new(body);
    let value = CandidateRecordV6::decode_body(&mut reader)?;
    reader.require_remaining_zero("Candidate reserve")?;
    Ok((sequence, value))
}

fn encoded_block(
    value: &PreparedPopulationV6,
    sequence: u64,
    bounds: PopulationV6Bounds,
) -> Result<Vec<[u8; RECORD_BYTES]>, PopulationV6Refusal> {
    let completion = CompletionV6::from_prepared(sequence, value, bounds)?;
    let capacity = value
        .candidates
        .len()
        .checked_add(4)
        .ok_or_else(|| "Population V6 encoded block capacity overflowed".to_owned())?;
    let mut records = Vec::new();
    records
        .try_reserve_exact(capacity)
        .map_err(|why| format!("cannot reserve Population V6 encoded block: {why}"))?;
    records.push(encode_manifest(&completion, DATA_KIND)?);
    records.push(encode_family(&value.families[0], sequence)?);
    records.push(encode_family(&value.families[1], sequence)?);
    for candidate in &value.candidates {
        records.push(encode_candidate(candidate, sequence)?);
    }
    records.push(encode_manifest(&completion, COMPLETION_KIND)?);
    Ok(records)
}

struct FixedWriter<'a> {
    bytes: &'a mut [u8],
    offset: usize,
}

impl<'a> FixedWriter<'a> {
    fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn raw(&mut self, value: &[u8]) -> Result<(), PopulationV6Refusal> {
        let end = self
            .offset
            .checked_add(value.len())
            .ok_or_else(|| "Population V6 writer offset overflowed".to_owned())?;
        self.bytes
            .get_mut(self.offset..end)
            .ok_or_else(|| "Population V6 record body is too small".to_owned())?
            .copy_from_slice(value);
        self.offset = end;
        Ok(())
    }

    fn array<const N: usize>(&mut self, value: &[u8; N]) -> Result<(), PopulationV6Refusal> {
        self.raw(value)
    }

    fn u8(&mut self, value: u8) -> Result<(), PopulationV6Refusal> {
        self.raw(&[value])
    }

    fn u16(&mut self, value: u16) -> Result<(), PopulationV6Refusal> {
        self.raw(&value.to_le_bytes())
    }

    fn u32(&mut self, value: u32) -> Result<(), PopulationV6Refusal> {
        self.raw(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), PopulationV6Refusal> {
        self.raw(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), PopulationV6Refusal> {
        self.raw(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), PopulationV6Refusal> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| "Population V6 zero-fill offset overflowed".to_owned())?;
        let target = self
            .bytes
            .get_mut(self.offset..end)
            .ok_or_else(|| "Population V6 zero-fill exceeds record".to_owned())?;
        target.fill(0);
        self.offset = end;
        Ok(())
    }
}

struct FixedReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> FixedReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn raw(&mut self, count: usize) -> Result<&'a [u8], PopulationV6Refusal> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| "Population V6 reader offset overflowed".to_owned())?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| "Population V6 record body is truncated".to_owned())?;
        self.offset = end;
        Ok(value)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], PopulationV6Refusal> {
        self.raw(N)?
            .try_into()
            .map_err(|_| "Population V6 fixed array is absent".to_owned())
    }

    fn u8(&mut self) -> Result<u8, PopulationV6Refusal> {
        self.raw(1)?
            .first()
            .copied()
            .ok_or_else(|| "Population V6 byte is absent".to_owned())
    }

    fn u16(&mut self) -> Result<u16, PopulationV6Refusal> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    fn u32(&mut self) -> Result<u32, PopulationV6Refusal> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, PopulationV6Refusal> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn i64(&mut self) -> Result<i64, PopulationV6Refusal> {
        Ok(i64::from_le_bytes(self.array()?))
    }

    fn require_zero(&mut self, count: usize, name: &str) -> Result<(), PopulationV6Refusal> {
        if self.raw(count)?.iter().any(|byte| *byte != 0) {
            return Err(format!("Population V6 {name} is nonzero"));
        }
        Ok(())
    }

    fn require_remaining_zero(&self, name: &str) -> Result<(), PopulationV6Refusal> {
        if self
            .bytes
            .get(self.offset..)
            .ok_or_else(|| format!("Population V6 {name} offset is invalid"))?
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(format!("Population V6 {name} is nonzero"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationV6StructuralReceipt {
    sequence: u64,
    first_record: u64,
    population_id: [u8; 32],
    completion_id: [u8; 32],
    ordered_candidate_digest: [u8; 32],
    finalization_id: [u8; 32],
    finalization_completion_id: [u8; 32],
    nifty_terminal: AdmissionV4FamilyTerminal,
    banknifty_terminal: AdmissionV4FamilyTerminal,
    nifty_candidate_count: u64,
    banknifty_candidate_count: u64,
    candidate_count: u64,
    evaluated_count: u64,
    decision_count: u64,
}

impl PopulationV6StructuralReceipt {
    pub(crate) const fn population_id(self) -> [u8; 32] {
        self.population_id
    }

    pub(crate) const fn completion_id(self) -> [u8; 32] {
        self.completion_id
    }

    pub(crate) const fn ordered_candidate_digest(self) -> [u8; 32] {
        self.ordered_candidate_digest
    }

    pub(crate) const fn candidate_count(self) -> u64 {
        self.candidate_count
    }

    pub(crate) const fn evaluated_count(self) -> u64 {
        self.evaluated_count
    }

    pub(crate) const fn decision_count(self) -> u64 {
        self.decision_count
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TrailingV6 {
    first_record: u64,
    source: SourceRecordV6,
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
    modified: Option<std::time::SystemTime>,
}

#[cfg(not(unix))]
impl PlatformIdentity {
    fn of(metadata: &std::fs::Metadata) -> Self {
        Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGeneration {
    identity: PlatformIdentity,
    len: u64,
    digest: [u8; 32],
}

struct PopulationV6Ledger {
    root: PathBuf,
    root_file: File,
    root_identity: PlatformIdentity,
    lock_path: PathBuf,
    lock_file: File,
    lock_generation: FileGeneration,
    data_path: PathBuf,
    data_file: File,
    data_generation: FileGeneration,
    bounds: PopulationV6Bounds,
    receipts: HashMap<[u8; 32], PopulationV6StructuralReceipt>,
    trailing: Option<TrailingV6>,
    record_count: u64,
    writable: bool,
}

impl PopulationV6Ledger {
    fn open_write(root: &Path, bounds: PopulationV6Bounds) -> Result<Self, PopulationV6Refusal> {
        Self::open(root, bounds, true)
    }

    fn open_read(root: &Path, bounds: PopulationV6Bounds) -> Result<Self, PopulationV6Refusal> {
        Self::open(root, bounds, false)
    }

    fn open(
        root: &Path,
        bounds: PopulationV6Bounds,
        writable: bool,
    ) -> Result<Self, PopulationV6Refusal> {
        let (canonical, root_file, root_identity) = open_root(root)?;
        let lock_path = canonical.join(LOCK_FILE);
        let data_path = canonical.join(DATA_FILE);
        let (lock_file, lock_created) = open_child(&lock_path, writable)?;
        if writable {
            lock_file
                .lock()
                .map_err(|why| format!("cannot lock Population V6 writer: {why}"))?;
        } else {
            lock_file
                .lock_shared()
                .map_err(|why| format!("cannot shared-lock Population V6 reader: {why}"))?;
        }
        let opened = (|| {
            let (mut data_file, data_created) = open_child(&data_path, writable)?;
            if data_created {
                data_file
                    .write_all(&header())
                    .and_then(|_| data_file.sync_all())
                    .map_err(|why| format!("cannot initialize Population V6 header: {why}"))?;
                root_file
                    .sync_all()
                    .map_err(|why| format!("cannot sync Population V6 directory: {why}"))?;
            } else {
                verify_header(&mut data_file)?;
            }
            if lock_created {
                lock_file
                    .sync_all()
                    .map_err(|why| format!("cannot sync Population V6 lock file: {why}"))?;
                root_file
                    .sync_all()
                    .map_err(|why| format!("cannot sync Population V6 lock directory: {why}"))?;
            }
            let lock_generation = file_generation(&lock_file, &lock_path, 0)?;
            let data_generation = file_generation(&data_file, &data_path, bounds.file_bytes)?;
            let mut ledger = Self {
                root: canonical,
                root_file,
                root_identity,
                lock_path,
                lock_file: lock_file
                    .try_clone()
                    .map_err(|why| format!("cannot clone Population V6 lock: {why}"))?,
                lock_generation,
                data_path,
                data_file,
                data_generation,
                bounds,
                receipts: HashMap::new(),
                trailing: None,
                record_count: 0,
                writable,
            };
            ledger.scan()?;
            Ok(ledger)
        })();
        let unlocked = lock_file
            .unlock()
            .map_err(|why| format!("cannot unlock Population V6 open: {why}"));
        combine(opened, unlocked)
    }

    fn scan(&mut self) -> Result<(), PopulationV6Refusal> {
        verify_header(&mut self.data_file)?;
        let metadata = self
            .data_file
            .metadata()
            .map_err(|why| format!("cannot stat Population V6 data: {why}"))?;
        if metadata.len() > self.bounds.file_bytes || metadata.len() < HEADER_BYTES as u64 {
            return Err("Population V6 data length exceeds bounds or omits header".to_owned());
        }
        let body = metadata.len() - HEADER_BYTES as u64;
        if !body.is_multiple_of(RECORD_BYTES as u64) {
            return Err("Population V6 data file is ragged".to_owned());
        }
        self.record_count = body / RECORD_BYTES as u64;
        self.receipts = HashMap::new();
        self.receipts
            .try_reserve(
                usize::try_from(self.bounds.authorities)
                    .map_err(|_| "Population V6 authority bound does not fit usize".to_owned())?,
            )
            .map_err(|why| format!("cannot reserve Population V6 receipt index: {why}"))?;
        self.trailing = None;
        let mut cursor = 0_u64;
        let mut sequence = 0_u64;
        while cursor < self.record_count {
            if sequence >= self.bounds.authorities {
                return Err("Population V6 authority count exceeds bound".to_owned());
            }
            let data = decode_manifest(&read_record_at(&mut self.data_file, cursor)?, DATA_KIND)?;
            if data.sequence != sequence {
                return Err("Population V6 block sequence is not canonical".to_owned());
            }
            let block_records = data
                .source
                .candidate_count
                .checked_add(4)
                .ok_or_else(|| "Population V6 block record count overflowed".to_owned())?;
            if data.source.candidate_count > self.bounds.candidates_per_authority {
                return Err("Population V6 block exceeds Candidate bound".to_owned());
            }
            let remaining = self.record_count - cursor;
            if remaining < block_records {
                validate_prefix(&mut self.data_file, cursor, remaining, &data, self.bounds)?;
                self.trailing = Some(TrailingV6 {
                    first_record: cursor,
                    source: data.source,
                    record_count: remaining,
                });
                cursor = self.record_count;
            } else {
                let receipt =
                    validate_complete_block(&mut self.data_file, cursor, &data, self.bounds)?;
                if self
                    .receipts
                    .insert(receipt.population_id, receipt)
                    .is_some()
                {
                    return Err("Population V6 duplicates a Population identity".to_owned());
                }
                cursor = cursor
                    .checked_add(block_records)
                    .ok_or_else(|| "Population V6 scan cursor overflowed".to_owned())?;
                sequence = sequence
                    .checked_add(1)
                    .ok_or_else(|| "Population V6 scan sequence overflowed".to_owned())?;
            }
        }
        self.data_generation =
            file_generation(&self.data_file, &self.data_path, self.bounds.file_bytes)?;
        Ok(())
    }

    fn append(
        &mut self,
        prepared: &PreparedPopulationV6,
    ) -> Result<(bool, PopulationV6StructuralReceipt), PopulationV6Refusal> {
        if !self.writable {
            return Err("Population V6 read-only ledger cannot append".to_owned());
        }
        self.lock_file
            .lock()
            .map_err(|why| format!("cannot lock Population V6 append: {why}"))?;
        let result = self.append_locked(prepared);
        let unlocked = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot unlock Population V6 append: {why}"));
        combine(result, unlocked)
    }

    fn append_locked(
        &mut self,
        prepared: &PreparedPopulationV6,
    ) -> Result<(bool, PopulationV6StructuralReceipt), PopulationV6Refusal> {
        self.require_unchanged()?;
        prepared.validate(self.bounds)?;
        if let Some(existing) = self.receipts.get(&prepared.source.population_id).copied() {
            require_exact_records(
                &mut self.data_file,
                existing.first_record,
                prepared,
                existing.sequence,
                self.bounds,
            )?;
            return Ok((false, existing));
        }
        let sequence = u64::try_from(self.receipts.len())
            .map_err(|_| "Population V6 authority sequence does not fit u64".to_owned())?;
        let records = encoded_block(prepared, sequence, self.bounds)?;
        let prefix = if let Some(trailing) = &self.trailing {
            if trailing.source.population_id != prepared.source.population_id {
                return Err("Population V6 foreign retry cannot replace trailing prefix".to_owned());
            }
            for index in 0..trailing.record_count {
                let stored = read_record_at(&mut self.data_file, trailing.first_record + index)?;
                let expected =
                    records
                        .get(usize::try_from(index).map_err(|_| {
                            "Population V6 prefix index does not fit usize".to_owned()
                        })?)
                        .ok_or_else(|| {
                            "Population V6 trailing prefix exceeds exact preparation".to_owned()
                        })?;
                if &stored != expected {
                    return Err("Population V6 trailing prefix differs from exact retry".to_owned());
                }
            }
            trailing.record_count
        } else {
            0
        };
        let record_len = u64::try_from(records.len())
            .map_err(|_| "Population V6 record length does not fit u64".to_owned())?;
        let appended = record_len
            .checked_sub(prefix)
            .ok_or_else(|| "Population V6 prefix is longer than its block".to_owned())?;
        let next_records = self
            .record_count
            .checked_add(appended)
            .ok_or_else(|| "Population V6 record total overflowed".to_owned())?;
        let next_bytes = (HEADER_BYTES as u64)
            .checked_add(
                next_records
                    .checked_mul(RECORD_BYTES as u64)
                    .ok_or_else(|| "Population V6 byte total overflowed".to_owned())?,
            )
            .ok_or_else(|| "Population V6 byte total overflowed".to_owned())?;
        if next_bytes > self.bounds.file_bytes || sequence >= self.bounds.authorities {
            return Err("Population V6 append exceeds authority/file bound".to_owned());
        }
        let completion_index = record_len
            .checked_sub(1)
            .ok_or_else(|| "Population V6 block omits Completion".to_owned())?;
        for index in prefix..completion_index {
            let raw = records
                .get(
                    usize::try_from(index)
                        .map_err(|_| "Population V6 append index does not fit usize".to_owned())?,
                )
                .ok_or_else(|| "Population V6 append record is absent".to_owned())?;
            append_raw(&mut self.data_file, raw)?;
        }
        self.data_file
            .sync_all()
            .map_err(|why| format!("cannot sync Population V6 evidence: {why}"))?;
        append_raw(
            &mut self.data_file,
            records
                .last()
                .ok_or_else(|| "Population V6 encoded block is empty".to_owned())?,
        )?;
        self.data_file
            .sync_all()
            .map_err(|why| format!("cannot sync Population V6 Completion: {why}"))?;
        self.root_file
            .sync_all()
            .map_err(|why| format!("cannot sync Population V6 directory: {why}"))?;
        self.data_generation =
            file_generation(&self.data_file, &self.data_path, self.bounds.file_bytes)?;
        self.scan()?;
        let receipt = self
            .receipts
            .get(&prepared.source.population_id)
            .copied()
            .ok_or_else(|| "Population V6 appended block is absent after scan".to_owned())?;
        Ok((true, receipt))
    }

    fn read_complete(
        &mut self,
        receipt: PopulationV6StructuralReceipt,
    ) -> Result<(CompletionV6, [FamilyRecordV6; 2], Vec<CandidateRecordV6>), PopulationV6Refusal>
    {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot shared-lock Population V6 projection: {why}"))?;
        let result = (|| {
            self.require_unchanged()?;
            let indexed = self
                .receipts
                .get(&receipt.population_id)
                .copied()
                .ok_or_else(|| "Population V6 receipt is absent".to_owned())?;
            if indexed != receipt {
                return Err("Population V6 receipt is stale or foreign".to_owned());
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
                return Err("Population V6 Family moved between blocks".to_owned());
            }
            let families = [nifty, banknifty];
            let mut candidates = Vec::new();
            candidates
                .try_reserve_exact(
                    usize::try_from(data.source.candidate_count)
                        .map_err(|_| "Population V6 read count does not fit usize".to_owned())?,
                )
                .map_err(|why| format!("cannot reserve Population V6 read rows: {why}"))?;
            for ordinal in 0..data.source.candidate_count {
                let (sequence, candidate) = decode_candidate_record(&read_record_at(
                    &mut self.data_file,
                    receipt.first_record + 3 + ordinal,
                )?)?;
                if sequence != receipt.sequence || candidate.global_sequence != ordinal {
                    return Err("Population V6 Candidate moved or reordered".to_owned());
                }
                candidates.push(candidate);
            }
            let completion = decode_manifest(
                &read_record_at(
                    &mut self.data_file,
                    receipt.first_record + 3 + data.source.candidate_count,
                )?,
                COMPLETION_KIND,
            )?;
            let prepared = PreparedPopulationV6 {
                source: data.source.clone(),
                families,
                candidates,
            };
            prepared.validate(self.bounds)?;
            if completion != data
                || completion.ordered_family_digest != ordered_family_digest(&prepared.families)
                || completion.ordered_candidate_digest
                    != ordered_candidate_digest(&prepared.candidates)
            {
                return Err("Population V6 complete block no longer reconciles".to_owned());
            }
            self.require_unchanged()?;
            Ok((completion, prepared.families, prepared.candidates))
        })();
        let unlocked = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot unlock Population V6 projection: {why}"));
        combine(result, unlocked)
    }

    fn require_unchanged(&self) -> Result<(), PopulationV6Refusal> {
        if named_identity(&self.root)? != self.root_identity
            || PlatformIdentity::of(
                &self
                    .root_file
                    .metadata()
                    .map_err(|why| format!("cannot stat held Population V6 root: {why}"))?,
            ) != self.root_identity
            || file_generation(&self.lock_file, &self.lock_path, 0)? != self.lock_generation
            || file_generation(&self.data_file, &self.data_path, self.bounds.file_bytes)?
                != self.data_generation
        {
            return Err("Population V6 retained file/root generation changed".to_owned());
        }
        Ok(())
    }
}

fn validate_prefix(
    file: &mut File,
    first: u64,
    count: u64,
    data: &CompletionV6,
    bounds: PopulationV6Bounds,
) -> Result<(), PopulationV6Refusal> {
    if count == 0 {
        return Err("Population V6 trailing prefix omits Data".to_owned());
    }
    let mut families = [None, None];
    if count >= 2 {
        let (sequence, family) = decode_family_record(&read_record_at(file, first + 1)?)?;
        if sequence != data.sequence {
            return Err("Population V6 trailing NIFTY Family moved".to_owned());
        }
        family.validate(&data.source, 0)?;
        families[0] = Some(family);
    }
    if count >= 3 {
        let (sequence, family) = decode_family_record(&read_record_at(file, first + 2)?)?;
        if sequence != data.sequence {
            return Err("Population V6 trailing BANKNIFTY Family moved".to_owned());
        }
        family.validate(&data.source, 1)?;
        families[1] = Some(family);
    }
    let available = count.saturating_sub(3).min(data.source.candidate_count);
    if available > 0 {
        let full_families = [
            families[0].ok_or_else(|| "Population V6 trailing NIFTY is absent".to_owned())?,
            families[1].ok_or_else(|| "Population V6 trailing BANKNIFTY is absent".to_owned())?,
        ];
        for ordinal in 0..available {
            let (sequence, candidate) =
                decode_candidate_record(&read_record_at(file, first + 3 + ordinal)?)?;
            if sequence != data.sequence || candidate.global_sequence != ordinal {
                return Err("Population V6 trailing Candidate order differs".to_owned());
            }
            candidate.validate(&data.source, &full_families)?;
        }
    }
    if count > data.source.candidate_count + 3 {
        return Err("Population V6 trailing prefix contains an unaccounted Completion".to_owned());
    }
    if data.source.candidate_count > bounds.candidates_per_authority {
        return Err("Population V6 trailing prefix exceeds Candidate bound".to_owned());
    }
    Ok(())
}

fn validate_complete_block(
    file: &mut File,
    first: u64,
    data: &CompletionV6,
    bounds: PopulationV6Bounds,
) -> Result<PopulationV6StructuralReceipt, PopulationV6Refusal> {
    let block_records = data
        .source
        .candidate_count
        .checked_add(4)
        .ok_or_else(|| "Population V6 block record count overflowed".to_owned())?;
    validate_prefix(file, first, block_records - 1, data, bounds)?;
    let completion = decode_manifest(
        &read_record_at(file, first + block_records - 1)?,
        COMPLETION_KIND,
    )?;
    if &completion != data {
        return Err("Population V6 Data and Completion differ".to_owned());
    }
    let (_, nifty) = decode_family_record(&read_record_at(file, first + 1)?)?;
    let (_, banknifty) = decode_family_record(&read_record_at(file, first + 2)?)?;
    let families = [nifty, banknifty];
    let mut candidates = Vec::new();
    candidates
        .try_reserve_exact(
            usize::try_from(data.source.candidate_count)
                .map_err(|_| "Population V6 validation count does not fit usize".to_owned())?,
        )
        .map_err(|why| format!("cannot reserve Population V6 validation rows: {why}"))?;
    for ordinal in 0..data.source.candidate_count {
        let (_, candidate) = decode_candidate_record(&read_record_at(file, first + 3 + ordinal)?)?;
        candidates.push(candidate);
    }
    if completion.ordered_family_digest != ordered_family_digest(&families)
        || completion.ordered_candidate_digest != ordered_candidate_digest(&candidates)
    {
        return Err("Population V6 ordered evidence digest differs".to_owned());
    }
    PreparedPopulationV6 {
        source: data.source.clone(),
        families,
        candidates,
    }
    .validate(bounds)?;
    Ok(PopulationV6StructuralReceipt {
        sequence: data.sequence,
        first_record: first,
        population_id: data.source.population_id,
        completion_id: data.completion_id,
        ordered_candidate_digest: data.ordered_candidate_digest,
        finalization_id: data.source.finalization_id,
        finalization_completion_id: data.source.finalization_completion_id,
        nifty_terminal: data.source.nifty_terminal,
        banknifty_terminal: data.source.banknifty_terminal,
        nifty_candidate_count: data.source.nifty_candidate_count,
        banknifty_candidate_count: data.source.banknifty_candidate_count,
        candidate_count: data.source.candidate_count,
        evaluated_count: data.source.evaluated_count,
        decision_count: data.source.decision_count,
    })
}

fn require_exact_records(
    file: &mut File,
    first: u64,
    prepared: &PreparedPopulationV6,
    sequence: u64,
    bounds: PopulationV6Bounds,
) -> Result<(), PopulationV6Refusal> {
    let expected = encoded_block(prepared, sequence, bounds)?;
    for (offset, raw) in expected.iter().enumerate() {
        let offset = u64::try_from(offset)
            .map_err(|_| "Population V6 comparison offset does not fit u64".to_owned())?;
        if read_record_at(file, first + offset)? != *raw {
            return Err("Population V6 existing bytes differ from exact preparation".to_owned());
        }
    }
    Ok(())
}

fn record_offset(index: u64) -> Result<u64, PopulationV6Refusal> {
    (HEADER_BYTES as u64)
        .checked_add(
            index
                .checked_mul(RECORD_BYTES as u64)
                .ok_or_else(|| "Population V6 record offset overflowed".to_owned())?,
        )
        .ok_or_else(|| "Population V6 record address overflowed".to_owned())
}

fn read_record_at(file: &mut File, index: u64) -> Result<[u8; RECORD_BYTES], PopulationV6Refusal> {
    let mut raw = [0_u8; RECORD_BYTES];
    file.seek(SeekFrom::Start(record_offset(index)?))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| format!("cannot read Population V6 record {index}: {why}"))?;
    Ok(raw)
}

fn append_raw(file: &mut File, raw: &[u8; RECORD_BYTES]) -> Result<(), PopulationV6Refusal> {
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(raw))
        .map_err(|why| format!("cannot append Population V6 record: {why}"))
}

fn open_root(root: &Path) -> Result<(PathBuf, File, PlatformIdentity), PopulationV6Refusal> {
    let metadata = std::fs::symlink_metadata(root)
        .map_err(|why| format!("cannot stat Population V6 root: {why}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("Population V6 root must be an existing nonsymlink directory".to_owned());
    }
    let canonical = std::fs::canonicalize(root)
        .map_err(|why| format!("cannot canonicalize Population V6 root: {why}"))?;
    let file =
        File::open(&canonical).map_err(|why| format!("cannot open Population V6 root: {why}"))?;
    let identity = PlatformIdentity::of(
        &file
            .metadata()
            .map_err(|why| format!("cannot stat held Population V6 root: {why}"))?,
    );
    if named_identity(&canonical)? != identity {
        return Err("Population V6 root changed while opening".to_owned());
    }
    Ok((canonical, file, identity))
}

fn open_child(path: &Path, writable: bool) -> Result<(File, bool), PopulationV6Refusal> {
    if let Ok(metadata) = std::fs::symlink_metadata(path)
        && (metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err("Population V6 child is not a regular nonsymlink file".to_owned());
    }
    let mut options = OpenOptions::new();
    options.read(true).write(writable).create(writable);
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    options.custom_flags(O_NOFOLLOW_FLAG);
    let existed = path.exists();
    let file = options
        .open(path)
        .map_err(|why| format!("cannot open Population V6 child: {why}"))?;
    let metadata = file
        .metadata()
        .map_err(|why| format!("cannot stat Population V6 child: {why}"))?;
    if !metadata.is_file() || named_identity(path)? != PlatformIdentity::of(&metadata) {
        return Err("Population V6 named child differs from held file".to_owned());
    }
    Ok((file, !existed))
}

fn named_identity(path: &Path) -> Result<PlatformIdentity, PopulationV6Refusal> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|why| format!("cannot stat named Population V6 path: {why}"))?;
    if metadata.file_type().is_symlink() {
        return Err("Population V6 named path became a symlink".to_owned());
    }
    Ok(PlatformIdentity::of(&metadata))
}

fn file_generation(
    file: &File,
    path: &Path,
    maximum: u64,
) -> Result<FileGeneration, PopulationV6Refusal> {
    let metadata = file
        .metadata()
        .map_err(|why| format!("cannot stat Population V6 file: {why}"))?;
    let identity = PlatformIdentity::of(&metadata);
    if named_identity(path)? != identity {
        return Err("Population V6 named file was replaced".to_owned());
    }
    if (maximum == 0 && metadata.len() != 0) || (maximum != 0 && metadata.len() > maximum) {
        return Err(format!(
            "Population V6 file has {} bytes above bound {maximum}",
            metadata.len()
        ));
    }
    let mut clone = file
        .try_clone()
        .map_err(|why| format!("cannot clone Population V6 file for hashing: {why}"))?;
    clone
        .seek(SeekFrom::Start(0))
        .map_err(|why| format!("cannot rewind Population V6 file: {why}"))?;
    let mut hasher = Hasher::new();
    hasher.update(GENERATION_DOMAIN);
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    loop {
        let count = clone
            .read(&mut buffer)
            .map_err(|why| format!("cannot hash Population V6 file: {why}"))?;
        if count == 0 {
            break;
        }
        let bytes = buffer
            .get(..count)
            .ok_or_else(|| "Population V6 hash read exceeds buffer".to_owned())?;
        hasher.update(bytes);
    }
    let after = file
        .metadata()
        .map_err(|why| format!("cannot restat Population V6 file: {why}"))?;
    if PlatformIdentity::of(&after) != identity
        || after.len() != metadata.len()
        || named_identity(path)? != identity
    {
        return Err("Population V6 file changed while hashing".to_owned());
    }
    Ok(FileGeneration {
        identity,
        len: metadata.len(),
        digest: hasher.finalize(),
    })
}

fn verify_header(file: &mut File) -> Result<(), PopulationV6Refusal> {
    let mut raw = [0_u8; HEADER_BYTES];
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| format!("cannot read Population V6 header: {why}"))?;
    if raw != header() {
        return Err("Population V6 header is invalid".to_owned());
    }
    Ok(())
}

fn combine<T>(
    result: Result<T, PopulationV6Refusal>,
    unlock: Result<(), PopulationV6Refusal>,
) -> Result<T, PopulationV6Refusal> {
    match (result, unlock) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(why), _) | (Ok(_), Err(why)) => Err(why),
    }
}

/// One exact, source-retaining Population V6 production capability.
///
/// Its fields are private and its constructor accepts only opaque authorities;
/// no family, direction, mask, terminal, identity, count or status is supplied
/// by a caller. Candidate V1 authorities exist only for evaluated families;
/// naturally extinct families retain their exact zero-row lineage through
/// Finalization V4 and do not invent an impossible Pre-Admission V1 source.
pub(crate) struct PopulationV6ProductionSourceV1 {
    finalization: CommittedStoredPopulationFinalizationV4,
    candidates: PopulationV6CandidateAuthoritiesV1,
}

/// Which of the two charter families this Population V6 source actually holds.
///
/// # Why every payload is boxed
///
/// A `CommittedStoredCandidatePreAdmissionV1` is about 6.4 KiB, so an inline
/// `Both` made the enum 12,896 bytes -- and `None` still cost all of it,
/// because an enum is as large as its widest variant. Every move of this token
/// through the bind/authenticate chain copied that, and the `None` case copied
/// twelve kilobytes of nothing.
///
/// Boxing makes all four variants pointer-sized. The cost is one allocation per
/// bound family, and the bind happens sixteen times in an all-rung run -- twice
/// per rung -- never inside a loop over bars or candidates. That is well under
/// the granularity `CLAUDE.md` §3 rule 4 governs: it is not a per-operation
/// cost, so it does not touch the O(1) claim either way.
enum PopulationV6CandidateAuthoritiesV1 {
    Both {
        nifty: Box<CommittedStoredCandidatePreAdmissionV1>,
        banknifty: Box<CommittedStoredCandidatePreAdmissionV1>,
    },
    Nifty(Box<CommittedStoredCandidatePreAdmissionV1>),
    BankNifty(Box<CommittedStoredCandidatePreAdmissionV1>),
    None,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AuthenticatedPopulationV6CandidateFamilyV1 {
    receipt: CandidateUniverseReceiptV1,
    rows: Vec<AuthenticatedCandidatePopulationRowV1>,
}

/// Complete authenticated upstream snapshot consumed by the V6 codec.
///
/// This value remains private so it cannot become a detached production door.
/// It records the exact Candidate V1 execution-coordinate rows.  The abstract
/// upstream singleton terminal is refused because it is unreachable under the
/// current mandatory Long+Short Candidate production law.
#[derive(Clone, Debug, PartialEq, Eq)]
struct AuthenticatedPopulationV6UpstreamV1 {
    finalization: PopulationFinalizationV4PopulationSourceV1,
    nifty: Option<AuthenticatedPopulationV6CandidateFamilyV1>,
    banknifty: Option<AuthenticatedPopulationV6CandidateFamilyV1>,
}

impl PopulationV6ProductionSourceV1 {
    /// Reauthenticates Finalization and every reachable Candidate source as one snapshot.
    fn authenticate(&mut self) -> Result<AuthenticatedPopulationV6UpstreamV1, PopulationV6Refusal> {
        let first_finalization = self
            .finalization
            .population_source()
            .map_err(|why| format!("Population V6 Finalization source refused: {why}"))?;
        let (nifty, banknifty) = authenticate_candidate_authorities(&self.candidates)?;
        validate_candidate_pair(&first_finalization, nifty.as_ref(), banknifty.as_ref())?;

        // Re-read every retained authority. Equality is required, not
        // just matching counts, so a same-length replacement or semantic
        // crosswire cannot straddle the snapshot boundary.
        let (second_nifty, second_banknifty) =
            authenticate_candidate_authorities(&self.candidates)?;
        let second_finalization = self
            .finalization
            .population_source()
            .map_err(|why| format!("Population V6 trailing Finalization source refused: {why}"))?;
        if second_finalization != first_finalization
            || second_nifty != nifty
            || second_banknifty != banknifty
        {
            return Err(
                "Population V6 upstream authority changed during authentication".to_owned(),
            );
        }
        Ok(AuthenticatedPopulationV6UpstreamV1 {
            finalization: first_finalization,
            nifty,
            banknifty,
        })
    }
}

fn authenticate_candidate_authorities(
    candidates: &PopulationV6CandidateAuthoritiesV1,
) -> Result<
    (
        Option<AuthenticatedPopulationV6CandidateFamilyV1>,
        Option<AuthenticatedPopulationV6CandidateFamilyV1>,
    ),
    PopulationV6Refusal,
> {
    let authenticate = |source: &CommittedStoredCandidatePreAdmissionV1| {
        Ok::<AuthenticatedPopulationV6CandidateFamilyV1, PopulationV6Refusal>(
            AuthenticatedPopulationV6CandidateFamilyV1 {
                receipt: source.candidate_pre_admission().candidate_audit().receipt(),
                rows: source.authenticated_candidate_population_rows()?,
            },
        )
    };
    match candidates {
        PopulationV6CandidateAuthoritiesV1::Both { nifty, banknifty } => Ok((
            Some(authenticate(nifty.as_ref())?),
            Some(authenticate(banknifty.as_ref())?),
        )),
        PopulationV6CandidateAuthoritiesV1::Nifty(nifty) => {
            Ok((Some(authenticate(nifty.as_ref())?), None))
        }
        PopulationV6CandidateAuthoritiesV1::BankNifty(banknifty) => {
            Ok((None, Some(authenticate(banknifty.as_ref())?)))
        }
        PopulationV6CandidateAuthoritiesV1::None => Ok((None, None)),
    }
}

/// Atomically binds Finalization V4 to the exact retained Candidate pair.
///
/// # Errors
///
/// Refuses a stale, replaced, corrupt, same-family, reordered or crosswired
/// authority or any terminal containing the production-unreachable Candidate
/// V1 singleton.
pub(crate) fn bind_population_v6_source_v1(
    finalization: CommittedStoredPopulationFinalizationV4,
    nifty: CommittedStoredCandidatePreAdmissionV1,
    banknifty: CommittedStoredCandidatePreAdmissionV1,
) -> Result<PopulationV6ProductionSourceV1, PopulationV6Refusal> {
    bind_population_v6_source(
        finalization,
        PopulationV6CandidateAuthoritiesV1::Both {
            nifty: Box::new(nifty),
            banknifty: Box::new(banknifty),
        },
    )
}

pub(crate) fn bind_population_v6_nifty_evaluated_source_v1(
    finalization: CommittedStoredPopulationFinalizationV4,
    nifty: CommittedStoredCandidatePreAdmissionV1,
) -> Result<PopulationV6ProductionSourceV1, PopulationV6Refusal> {
    bind_population_v6_source(
        finalization,
        PopulationV6CandidateAuthoritiesV1::Nifty(Box::new(nifty)),
    )
}

pub(crate) fn bind_population_v6_banknifty_evaluated_source_v1(
    finalization: CommittedStoredPopulationFinalizationV4,
    banknifty: CommittedStoredCandidatePreAdmissionV1,
) -> Result<PopulationV6ProductionSourceV1, PopulationV6Refusal> {
    bind_population_v6_source(
        finalization,
        PopulationV6CandidateAuthoritiesV1::BankNifty(Box::new(banknifty)),
    )
}

pub(crate) fn bind_population_v6_all_extinct_source_v1(
    finalization: CommittedStoredPopulationFinalizationV4,
) -> Result<PopulationV6ProductionSourceV1, PopulationV6Refusal> {
    bind_population_v6_source(finalization, PopulationV6CandidateAuthoritiesV1::None)
}

fn bind_population_v6_source(
    finalization: CommittedStoredPopulationFinalizationV4,
    candidates: PopulationV6CandidateAuthoritiesV1,
) -> Result<PopulationV6ProductionSourceV1, PopulationV6Refusal> {
    let mut source = PopulationV6ProductionSourceV1 {
        finalization,
        candidates,
    };
    source.authenticate()?;
    Ok(source)
}

fn validate_candidate_pair(
    finalization: &PopulationFinalizationV4PopulationSourceV1,
    nifty_candidate: Option<&AuthenticatedPopulationV6CandidateFamilyV1>,
    banknifty_candidate: Option<&AuthenticatedPopulationV6CandidateFamilyV1>,
) -> Result<(), PopulationV6Refusal> {
    let families = finalization.families();
    let nifty = families
        .first()
        .ok_or_else(|| "Population V6 Finalization NIFTY family is absent".to_owned())?;
    let banknifty = families
        .get(1)
        .ok_or_else(|| "Population V6 Finalization BANKNIFTY family is absent".to_owned())?;
    if nifty.family() != AdmissionV4Family::Nifty
        || banknifty.family() != AdmissionV4Family::BankNifty
        || nifty.candidate_universe_id() == banknifty.candidate_universe_id()
    {
        return Err("Population V6 authorities are not distinct NIFTY then BANKNIFTY".to_owned());
    }
    validate_candidate_family(finalization, nifty, nifty_candidate)?;
    validate_candidate_family(finalization, banknifty, banknifty_candidate)
}

fn validate_candidate_family(
    finalization: &PopulationFinalizationV4PopulationSourceV1,
    family: &PopulationFinalizationV4FamilyProjection,
    candidate: Option<&AuthenticatedPopulationV6CandidateFamilyV1>,
) -> Result<(), PopulationV6Refusal> {
    validate_reachable_terminal(family.terminal(), family.candidate_count())?;
    match family.terminal() {
        AdmissionV4FamilyTerminal::Evaluated => {}
        AdmissionV4FamilyTerminal::NaturallyExtinct => {
            if candidate.is_some() || family.decision_count() != 0 {
                return Err(
                    "Population V6 extinct Family received an invented Candidate authority"
                        .to_owned(),
                );
            }
            return Ok(());
        }
        AdmissionV4FamilyTerminal::InsufficientForCscv => {
            return Err(
                "Population V6 refuses production-unreachable Candidate V1 singleton".to_owned(),
            );
        }
    }
    let candidate = candidate.ok_or_else(|| {
        "Population V6 evaluated Family lacks its retained Candidate V1 authority".to_owned()
    })?;
    let receipt = candidate.receipt;
    let rows = candidate.rows.as_slice();
    let expected_family = match family.family() {
        AdmissionV4Family::Nifty => InstrumentFamilyV1::Nifty,
        AdmissionV4Family::BankNifty => InstrumentFamilyV1::BankNifty,
    };
    let source = finalization.source();
    let identities = receipt.identities();
    let signal = receipt.signal_stream();
    let grids = identities.exit_grids();
    let grid_composite = grids
        .composite_digest()
        .map_err(|why| format!("Population V6 Candidate grid identity refused: {why}"))?;
    if receipt.family() != expected_family
        || receipt.universe_id() != family.candidate_universe_id()
        || receipt.content_digest() != family.candidate_completion_digest()
        || receipt.ordered_row_digest() != family.candidate_ordered_row_digest()
        || receipt.rung_seconds() != source.rung_seconds()
        || receipt.horizon_bars() != source.horizon_bars()
        || receipt.requested_span() != source.requested_span()
        || identities.feed_digest() != source.feed_digest()
        || identities.source_commit_digest() != source.source_commit_digest()
        || identities.calendar_policy_digest() != source.calendar_policy_digest()
        || identities.daily_reference_policy_digest() != source.daily_reference_policy_digest()
        || identities.vocabulary_digest() != source.vocabulary_digest()
        || identities.evaluation_policy_digest() != source.evaluation_policy_digest()
        || signal.digest() != family.signal_digest()
        || signal.count() != family.signal_bars()
        || signal.first_ts_micros() != family.signal_first_ts_micros()
        || signal.last_ts_micros() != family.signal_last_ts_micros()
        || receipt.signal_column_digest() != family.signal_column_digest()
        || grids.long.policy_digest != family.long_grid_policy_digest()
        || grids.long.resolved_digest != family.long_grid_resolution_digest()
        || grids.short.policy_digest != family.short_grid_policy_digest()
        || grids.short.resolved_digest != family.short_grid_resolution_digest()
        || grid_composite != family.grid_composite_digest()
        || u64::try_from(rows.len()).ok() != Some(receipt.row_count())
        || receipt.row_count() != family.candidate_count()
    {
        return Err("Population V6 Candidate/Finalization family authority differs".to_owned());
    }
    if family.decision_count() != receipt.row_count() {
        return Err(
            "Population V6 reachable family decision count differs from Candidate V1".to_owned(),
        );
    }
    for (index, authenticated) in rows.iter().enumerate() {
        let row = authenticated.row();
        let sequence = u64::try_from(index)
            .map_err(|_| "Population V6 Candidate ordinal does not fit u64".to_owned())?;
        if row.family() != expected_family
            || row.universe_id() != receipt.universe_id()
            || row.sequence() != sequence
            || row.rung_seconds() != source.rung_seconds()
            || row.horizon_bars() != source.horizon_bars()
        {
            return Err(format!(
                "Population V6 Candidate row {sequence} escapes its exact family authority"
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PopulationV6SourceProjectionV1 {
    source: SourceRecordV6,
}

impl PopulationV6SourceProjectionV1 {
    pub(crate) const fn population_id(&self) -> [u8; 32] {
        self.source.population_id
    }

    pub(crate) const fn finalization_id(&self) -> [u8; 32] {
        self.source.finalization_id
    }

    pub(crate) const fn finalization_completion_id(&self) -> [u8; 32] {
        self.source.finalization_completion_id
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

    pub(crate) const fn policy(&self) -> &[u8; RUNNER_POLICY_BYTES] {
        &self.source.policy
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationV6FamilyProjectionV1 {
    record: FamilyRecordV6,
}

impl PopulationV6FamilyProjectionV1 {
    pub(crate) const fn row_id(self) -> [u8; 32] {
        self.record.row_id
    }

    pub(crate) const fn family(self) -> AdmissionV4Family {
        self.record.family
    }

    pub(crate) const fn terminal(self) -> AdmissionV4FamilyTerminal {
        self.record.terminal
    }

    pub(crate) const fn candidate_count(self) -> u64 {
        self.record.candidate_count
    }

    pub(crate) const fn evaluated_count(self) -> u64 {
        self.record.evaluated_count
    }

    pub(crate) const fn decision_count(self) -> u64 {
        self.record.decision_count
    }

    pub(crate) const fn candidate_universe_id(self) -> [u8; 32] {
        self.record.candidate_universe_id
    }

    pub(crate) const fn candidate_completion_digest(self) -> [u8; 32] {
        self.record.candidate_completion_digest
    }

    pub(crate) const fn finalization_family_row_id(self) -> [u8; 32] {
        self.record.finalization_family_row_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PopulationV6CandidateProjectionV1 {
    completion_id: [u8; 32],
    record: CandidateRecordV6,
    candidate: AuthenticatedCandidatePopulationRowV1,
}

impl PopulationV6CandidateProjectionV1 {
    pub(crate) const fn population_id(&self) -> [u8; 32] {
        self.record.population_id
    }

    pub(crate) const fn completion_id(&self) -> [u8; 32] {
        self.completion_id
    }

    pub(crate) const fn global_sequence(&self) -> u64 {
        self.record.global_sequence
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

    pub(crate) const fn row_id(&self) -> [u8; 32] {
        self.record.row_id
    }

    pub(crate) const fn candidate(&self) -> &AuthenticatedCandidatePopulationRowV1 {
        &self.candidate
    }

    pub(crate) const fn candidate_semantic_id(&self) -> [u8; 32] {
        self.record.candidate_semantic_id
    }

    pub(crate) const fn candidate_row_digest(&self) -> [u8; 32] {
        self.record.candidate_row_digest
    }

    pub(crate) const fn admission_decision_id(&self) -> [u8; 32] {
        self.record.admission_decision_id
    }

    pub(crate) const fn finalization_row_id(&self) -> [u8; 32] {
        self.record.finalization_row_id
    }

    pub(crate) const fn finalization_family_row_id(&self) -> [u8; 32] {
        self.record.finalization_family_row_id
    }

    pub(crate) const fn base_evidence_id(&self) -> [u8; 32] {
        self.record.base_evidence_id
    }

    pub(crate) const fn runner_decision(&self) -> &[u8; RUNNER_DECISION_BYTES] {
        &self.record.runner_decision
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PopulationV6ExecutionDispositionSourceV1 {
    population: PopulationV6CandidateProjectionV1,
    disposition: ExecutionDispositionV1,
}

impl PopulationV6ExecutionDispositionSourceV1 {
    pub(crate) const fn population(&self) -> &PopulationV6CandidateProjectionV1 {
        &self.population
    }

    pub(crate) const fn disposition(&self) -> &ExecutionDispositionV1 {
        &self.disposition
    }

    pub(crate) fn into_parts(self) -> (PopulationV6CandidateProjectionV1, ExecutionDispositionV1) {
        (self.population, self.disposition)
    }
}

pub(crate) struct PopulationV6ExecutionV4SourceV1 {
    receipt: PopulationV6StructuralReceipt,
    source: PopulationV6SourceProjectionV1,
    families: [PopulationV6FamilyProjectionV1; 2],
    parameters: Vec<CandidateExecutionParameterFactsV1>,
    rows: Vec<PopulationV6ExecutionDispositionSourceV1>,
}

impl PopulationV6ExecutionV4SourceV1 {
    pub(crate) const fn receipt(&self) -> PopulationV6StructuralReceipt {
        self.receipt
    }

    pub(crate) const fn source(&self) -> &PopulationV6SourceProjectionV1 {
        &self.source
    }

    pub(crate) const fn families(&self) -> &[PopulationV6FamilyProjectionV1; 2] {
        &self.families
    }

    pub(crate) fn parameters(&self) -> &[CandidateExecutionParameterFactsV1] {
        &self.parameters
    }

    pub(crate) fn rows(&self) -> &[PopulationV6ExecutionDispositionSourceV1] {
        &self.rows
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        PopulationV6StructuralReceipt,
        PopulationV6SourceProjectionV1,
        [PopulationV6FamilyProjectionV1; 2],
        Vec<CandidateExecutionParameterFactsV1>,
        Vec<PopulationV6ExecutionDispositionSourceV1>,
    ) {
        (
            self.receipt,
            self.source,
            self.families,
            self.parameters,
            self.rows,
        )
    }
}

struct PopulationV6Authority {
    receipt: PopulationV6StructuralReceipt,
    prepared: PreparedPopulationV6,
    ledger: PopulationV6Ledger,
}

impl PopulationV6Authority {
    fn projection(
        &mut self,
    ) -> Result<
        (
            PopulationV6SourceProjectionV1,
            [PopulationV6FamilyProjectionV1; 2],
            Vec<PopulationV6CandidateProjectionV1>,
        ),
        PopulationV6Refusal,
    > {
        let (completion, families, candidates) = self.ledger.read_complete(self.receipt)?;
        if completion
            != CompletionV6::from_prepared(
                self.receipt.sequence,
                &self.prepared,
                self.ledger.bounds,
            )?
            || families != self.prepared.families
            || candidates != self.prepared.candidates
        {
            return Err("Population V6 retained block differs from preparation".to_owned());
        }
        let mut projected = Vec::new();
        projected
            .try_reserve_exact(candidates.len())
            .map_err(|why| format!("cannot reserve Population V6 projection: {why}"))?;
        for record in candidates {
            let candidate =
                verify_population_candidate_canonical_record_v1(&record.candidate_record)
                    .map_err(|why| format!("Population V6 projected Candidate refused: {why}"))?;
            projected.push(PopulationV6CandidateProjectionV1 {
                completion_id: self.receipt.completion_id,
                record,
                candidate,
            });
        }
        Ok((
            PopulationV6SourceProjectionV1 {
                source: self.prepared.source.clone(),
            },
            [
                PopulationV6FamilyProjectionV1 {
                    record: families[0],
                },
                PopulationV6FamilyProjectionV1 {
                    record: families[1],
                },
            ],
            projected,
        ))
    }
}

pub(crate) struct CommittedStoredPopulationV6 {
    upstream: PopulationV6ProductionSourceV1,
    population: PopulationV6Authority,
}

impl CommittedStoredPopulationV6 {
    pub(crate) const fn structural_receipt(&self) -> PopulationV6StructuralReceipt {
        self.population.receipt
    }

    pub(crate) fn execution_v4_source(
        &mut self,
    ) -> Result<PopulationV6ExecutionV4SourceV1, PopulationV6Refusal> {
        let before = self.upstream.authenticate()?;
        let prepared =
            PreparedPopulationV6::from_upstream(before.clone(), self.population.ledger.bounds)?;
        if prepared != self.population.prepared {
            return Err("Population V6 retained upstream changed".to_owned());
        }
        let (source, families, population_rows) = self.population.projection()?;
        let mut parameters = Vec::new();
        parameters
            .try_reserve_exact(4)
            .map_err(|why| format!("cannot reserve Population V6 replay parameters: {why}"))?;
        let mut replay_rows = Vec::new();
        replay_rows
            .try_reserve_exact(population_rows.len())
            .map_err(|why| format!("cannot reserve Population V6 replay rows: {why}"))?;
        let mut append_replay = |candidate_source: &CommittedStoredCandidatePreAdmissionV1,
                                 expected: &AuthenticatedPopulationV6CandidateFamilyV1|
         -> Result<(), PopulationV6Refusal> {
            let replay = candidate_source.execution_v3_replay_authority()?;
            let (receipt, [long, short], mut family_rows) = replay.into_parts();
            if receipt != expected.receipt {
                return Err(
                    "Population V6 replay receipt differs from retained Candidate".to_owned(),
                );
            }
            parameters.push(long);
            parameters.push(short);
            replay_rows.append(&mut family_rows);
            Ok(())
        };
        match (&self.upstream.candidates, &before.nifty, &before.banknifty) {
            (
                PopulationV6CandidateAuthoritiesV1::Both { nifty, banknifty },
                Some(expected_nifty),
                Some(expected_banknifty),
            ) => {
                append_replay(nifty, expected_nifty)?;
                append_replay(banknifty, expected_banknifty)?;
            }
            (PopulationV6CandidateAuthoritiesV1::Nifty(nifty), Some(expected_nifty), None) => {
                append_replay(nifty, expected_nifty)?
            }
            (
                PopulationV6CandidateAuthoritiesV1::BankNifty(banknifty),
                None,
                Some(expected_banknifty),
            ) => append_replay(banknifty, expected_banknifty)?,
            (PopulationV6CandidateAuthoritiesV1::None, None, None) => {}
            _ => {
                return Err(
                    "Population V6 retained Candidate topology changed before replay".to_owned(),
                );
            }
        }
        if replay_rows.len() != population_rows.len() {
            return Err("Population V6 replay disposition count differs".to_owned());
        }
        let mut rows = Vec::new();
        rows.try_reserve_exact(population_rows.len())
            .map_err(|why| format!("cannot reserve Population V6 replay joins: {why}"))?;
        for (population, replay) in population_rows.into_iter().zip(replay_rows) {
            let (candidate, disposition) = replay.into_parts();
            if population.candidate != candidate {
                return Err("Population V6 replay Candidate differs from durable row".to_owned());
            }
            rows.push(PopulationV6ExecutionDispositionSourceV1 {
                population,
                disposition,
            });
        }
        let after = self.upstream.authenticate()?;
        if after != before
            || PreparedPopulationV6::from_upstream(after, self.population.ledger.bounds)?
                != prepared
        {
            return Err("Population V6 upstream changed during Execution V4 projection".to_owned());
        }
        Ok(PopulationV6ExecutionV4SourceV1 {
            receipt: self.population.receipt,
            source,
            families,
            parameters,
            rows,
        })
    }
}

pub(crate) enum PopulationV6Commit {
    Written(CommittedStoredPopulationV6),
    Reused(CommittedStoredPopulationV6),
}

impl PopulationV6Commit {
    pub(crate) fn authority_mut(&mut self) -> &mut CommittedStoredPopulationV6 {
        match self {
            Self::Written(value) | Self::Reused(value) => value,
        }
    }

    pub(crate) fn into_authority(self) -> CommittedStoredPopulationV6 {
        match self {
            Self::Written(value) | Self::Reused(value) => value,
        }
    }
}

pub(crate) fn commit_population_v6(
    root: &Path,
    bounds: PopulationV6Bounds,
    mut upstream: PopulationV6ProductionSourceV1,
) -> Result<PopulationV6Commit, PopulationV6Refusal> {
    let first = upstream.authenticate()?;
    let prepared = PreparedPopulationV6::from_upstream(first, bounds)?;
    let mut writer = PopulationV6Ledger::open_write(root, bounds)?;
    let (written, receipt) = writer.append(&prepared)?;
    drop(writer);
    let mut ledger = PopulationV6Ledger::open_read(root, bounds)?;
    let reopened = ledger
        .receipts
        .get(&prepared.source.population_id)
        .copied()
        .ok_or_else(|| "Population V6 fresh reopen omitted committed block".to_owned())?;
    if reopened != receipt {
        return Err("Population V6 fresh reopen changed receipt".to_owned());
    }
    let (completion, families, candidates) = ledger.read_complete(reopened)?;
    if completion != CompletionV6::from_prepared(reopened.sequence, &prepared, bounds)?
        || families != prepared.families
        || candidates != prepared.candidates
    {
        return Err("Population V6 fresh reopen differs from exact preparation".to_owned());
    }
    let after = upstream.authenticate()?;
    if PreparedPopulationV6::from_upstream(after, bounds)? != prepared {
        return Err("Population V6 upstream changed during commit".to_owned());
    }
    let committed = CommittedStoredPopulationV6 {
        upstream,
        population: PopulationV6Authority {
            receipt: reopened,
            prepared,
            ledger,
        },
    };
    Ok(if written {
        PopulationV6Commit::Written(committed)
    } else {
        PopulationV6Commit::Reused(committed)
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

    fn bounds() -> PopulationV6Bounds {
        PopulationV6Bounds::new(8, 1_000_000, 512 * 1_024 * 1_024)
            .expect("Population V6 fixture bounds are valid")
    }

    fn with_evaluated_prepared<T>(
        action: impl FnOnce(
            PopulationV6ProductionSourceV1,
            PreparedPopulationV6,
            &Path,
        ) -> Result<T, String>,
    ) -> Result<T, String> {
        crate::step3_orchestrator::with_population_v6_evaluated_pair_fixture(
            |finalization, nifty, banknifty, root| {
                let mut source = bind_population_v6_source_v1(finalization, nifty, banknifty)?;
                let upstream = source.authenticate()?;
                let prepared = PreparedPopulationV6::from_upstream(upstream, bounds())?;
                action(source, prepared, root)
            },
        )
    }

    #[test]
    fn terminal_reachability_matrix_accepts_four_pairs_and_refuses_five_singleton_pairs() {
        let evaluated = AdmissionV4FamilyTerminal::Evaluated;
        let extinct = AdmissionV4FamilyTerminal::NaturallyExtinct;
        let insufficient = AdmissionV4FamilyTerminal::InsufficientForCscv;
        for (nifty, banknifty) in [
            (evaluated, evaluated),
            (evaluated, extinct),
            (extinct, evaluated),
            (extinct, extinct),
        ] {
            let nifty_count = if nifty == evaluated { 2 } else { 0 };
            let banknifty_count = if banknifty == evaluated { 2 } else { 0 };
            assert_eq!(validate_reachable_terminal(nifty, nifty_count), Ok(()));
            assert_eq!(
                validate_reachable_terminal(banknifty, banknifty_count),
                Ok(())
            );
        }
        for (nifty, banknifty) in [
            (insufficient, evaluated),
            (insufficient, extinct),
            (insufficient, insufficient),
            (evaluated, insufficient),
            (extinct, insufficient),
        ] {
            let rejected = [
                validate_reachable_terminal(nifty, 1),
                validate_reachable_terminal(banknifty, 1),
            ];
            assert!(
                rejected
                    .iter()
                    .any(|result| matches!(result, Err(why) if why.contains("singleton")))
            );
        }
    }

    #[test]
    fn candidate_duplicates_gaps_crosswires_and_nested_corruption_refuse() -> Result<(), String> {
        with_evaluated_prepared(|_source, prepared, _root| {
            assert!(prepared.candidates.len() >= 4);

            let mut duplicate = prepared.clone();
            duplicate.candidates[1] = duplicate.candidates[0].clone();
            assert!(
                duplicate
                    .validate(bounds())
                    .expect_err("duplicated Candidate must refuse")
                    .contains("sequence")
            );

            let mut gap = prepared.clone();
            gap.candidates[1].global_sequence = gap.candidates[1]
                .global_sequence
                .checked_add(1)
                .ok_or_else(|| "test Candidate sequence overflowed".to_owned())?;
            gap.candidates[1].row_id = derive_candidate_row_id(&gap.candidates[1]);
            assert!(
                gap.validate(bounds())
                    .expect_err("Candidate gap must refuse")
                    .contains("sequence")
            );

            let mut crosswired = prepared.clone();
            crosswired.candidates[0].statistics_sequence = 1;
            crosswired.candidates[0].row_id = derive_candidate_row_id(&crosswired.candidates[0]);
            assert!(
                crosswired
                    .validate(bounds())
                    .expect_err("Statistics crosswire must refuse")
                    .contains("crosswires")
            );

            let mut corrupt_candidate = prepared.clone();
            corrupt_candidate.candidates[0].candidate_record[0] ^= 1;
            corrupt_candidate.candidates[0].row_id =
                derive_candidate_row_id(&corrupt_candidate.candidates[0]);
            assert!(
                corrupt_candidate
                    .validate(bounds())
                    .expect_err("corrupt nested Candidate must refuse")
                    .contains("nested Candidate")
            );

            let mut corrupt_runner = prepared;
            corrupt_runner.candidates[0].runner_decision[0] ^= 1;
            corrupt_runner.candidates[0].row_id =
                derive_candidate_row_id(&corrupt_runner.candidates[0]);
            assert!(
                corrupt_runner
                    .validate(bounds())
                    .expect_err("corrupt Runner decision must refuse")
                    .contains("Runner arithmetic")
            );
            Ok(())
        })
    }

    #[test]
    fn every_exact_prefix_recovers_but_foreign_and_ragged_prefixes_refuse() -> Result<(), String> {
        with_evaluated_prepared(|_source, prepared, root| {
            let records = encoded_block(&prepared, 0, bounds())?;
            for cut in 1..records.len() {
                let prefix_root = root.join(format!("exact-prefix-{cut}"));
                std::fs::create_dir(&prefix_root)
                    .map_err(|why| format!("cannot create prefix root: {why}"))?;
                drop(PopulationV6Ledger::open_write(&prefix_root, bounds())?);
                let mut file = OpenOptions::new()
                    .append(true)
                    .open(prefix_root.join(DATA_FILE))
                    .map_err(|why| format!("cannot open prefix file: {why}"))?;
                for record in records.iter().take(cut) {
                    file.write_all(record)
                        .map_err(|why| format!("cannot write prefix record: {why}"))?;
                }
                file.sync_all()
                    .map_err(|why| format!("cannot sync prefix records: {why}"))?;
                drop(file);
                let mut writer = PopulationV6Ledger::open_write(&prefix_root, bounds())?;
                let (written, receipt) = writer.append(&prepared)?;
                assert!(written);
                assert_eq!(receipt.population_id(), prepared.source.population_id);
                drop(writer);
                let mut reopened = PopulationV6Ledger::open_read(&prefix_root, bounds())?;
                reopened.read_complete(receipt)?;
            }

            let foreign_root = root.join("foreign-prefix");
            std::fs::create_dir(&foreign_root)
                .map_err(|why| format!("cannot create foreign-prefix root: {why}"))?;
            drop(PopulationV6Ledger::open_write(&foreign_root, bounds())?);
            let mut foreign = OpenOptions::new()
                .append(true)
                .open(foreign_root.join(DATA_FILE))
                .map_err(|why| format!("cannot open foreign-prefix file: {why}"))?;
            foreign
                .write_all(&records[0])
                .and_then(|()| foreign.write_all(&records[2]))
                .and_then(|()| foreign.sync_all())
                .map_err(|why| format!("cannot persist foreign prefix: {why}"))?;
            drop(foreign);
            assert!(PopulationV6Ledger::open_write(&foreign_root, bounds()).is_err());

            let ragged_root = root.join("ragged-prefix");
            std::fs::create_dir(&ragged_root)
                .map_err(|why| format!("cannot create ragged-prefix root: {why}"))?;
            drop(PopulationV6Ledger::open_write(&ragged_root, bounds())?);
            let mut ragged = OpenOptions::new()
                .append(true)
                .open(ragged_root.join(DATA_FILE))
                .map_err(|why| format!("cannot open ragged-prefix file: {why}"))?;
            ragged
                .write_all(&[1])
                .and_then(|()| ragged.sync_all())
                .map_err(|why| format!("cannot persist ragged prefix: {why}"))?;
            drop(ragged);
            assert!(PopulationV6Ledger::open_read(&ragged_root, bounds()).is_err());
            Ok(())
        })
    }

    #[cfg(unix)]
    #[test]
    fn same_length_corruption_stale_handle_and_named_path_replacement_refuse() -> Result<(), String>
    {
        use std::os::unix::fs::symlink;

        with_evaluated_prepared(|_source, prepared, root| {
            let stale_root = root.join("stale-generation");
            std::fs::create_dir(&stale_root)
                .map_err(|why| format!("cannot create stale-generation root: {why}"))?;
            let mut writer = PopulationV6Ledger::open_write(&stale_root, bounds())?;
            let (_, stale_receipt) = writer.append(&prepared)?;
            drop(writer);
            let mut stale_reader = PopulationV6Ledger::open_read(&stale_root, bounds())?;
            let stale_path = stale_root.join(DATA_FILE);
            let mutation_offset = record_offset(0)?
                .checked_add(40)
                .ok_or_else(|| "test mutation offset overflowed".to_owned())?;
            let mut external = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&stale_path)
                .map_err(|why| format!("cannot open Population V6 external mutator: {why}"))?;
            external
                .seek(SeekFrom::Start(mutation_offset))
                .map_err(|why| format!("cannot seek Population V6 mutation byte: {why}"))?;
            let mut byte = [0_u8; 1];
            external
                .read_exact(&mut byte)
                .map_err(|why| format!("cannot read Population V6 mutation byte: {why}"))?;
            byte[0] ^= 1;
            external
                .seek(SeekFrom::Start(mutation_offset))
                .and_then(|_| external.write_all(&byte))
                .and_then(|_| external.sync_all())
                .map_err(|why| format!("cannot persist Population V6 mutation: {why}"))?;
            drop(external);
            assert!(
                stale_reader
                    .read_complete(stale_receipt)
                    .expect_err("same-length mutation must stale a held reader")
                    .contains("generation changed")
            );
            assert!(PopulationV6Ledger::open_read(&stale_root, bounds()).is_err());

            let replacement_root = root.join("path-replacement");
            std::fs::create_dir(&replacement_root)
                .map_err(|why| format!("cannot create path-replacement root: {why}"))?;
            let mut writer = PopulationV6Ledger::open_write(&replacement_root, bounds())?;
            let (_, replacement_receipt) = writer.append(&prepared)?;
            drop(writer);
            let mut replacement_reader =
                PopulationV6Ledger::open_read(&replacement_root, bounds())?;
            let named = replacement_root.join(DATA_FILE);
            let displaced = replacement_root.join("displaced-population-v6.bin");
            std::fs::rename(&named, &displaced)
                .map_err(|why| format!("cannot displace Population V6 named file: {why}"))?;
            std::fs::copy(&displaced, &named).map_err(|why| {
                format!("cannot install exact-byte Population V6 replacement: {why}")
            })?;
            assert!(
                replacement_reader
                    .read_complete(replacement_receipt)
                    .expect_err("exact-byte named-file replacement must refuse")
                    .contains("replaced")
            );

            let root_link = root.join("root-symlink");
            symlink(&replacement_root, &root_link)
                .map_err(|why| format!("cannot create Population V6 root symlink: {why}"))?;
            assert!(PopulationV6Ledger::open_read(&root_link, bounds()).is_err());
            Ok(())
        })
    }

    #[test]
    fn genuine_evaluated_pair_commits_reopens_reuses_and_mints_exact_execution_source()
    -> Result<(), String> {
        crate::step3_orchestrator::with_population_v6_evaluated_pair_fixture(
            |finalization, nifty, banknifty, root| {
                let source = bind_population_v6_source_v1(finalization, nifty, banknifty)?;
                let mut committed = commit_population_v6(root, bounds(), source)?;
                assert!(matches!(&committed, PopulationV6Commit::Written(_)));
                let authority = committed.authority_mut();
                let receipt = authority.structural_receipt();
                assert!(receipt.candidate_count() >= 4);
                assert_eq!(receipt.candidate_count(), receipt.evaluated_count());
                assert_eq!(receipt.candidate_count(), receipt.decision_count());
                assert_ne!(receipt.ordered_candidate_digest(), [0; 32]);

                let prepared = authority.population.prepared.clone();
                let mut retry = PopulationV6Ledger::open_write(root, bounds())?;
                let (written, retried_receipt) = retry.append(&prepared)?;
                assert!(!written);
                assert_eq!(retried_receipt, receipt);
                drop(retry);

                let source = authority.execution_v4_source()?;
                assert_eq!(source.parameters().len(), 4);
                assert_eq!(
                    u64::try_from(source.rows().len())
                        .map_err(|_| "Population V6 test row count does not fit u64".to_owned())?,
                    receipt.candidate_count()
                );
                let [nifty_family, banknifty_family] = source.families();
                assert_eq!(nifty_family.family(), AdmissionV4Family::Nifty);
                assert_eq!(banknifty_family.family(), AdmissionV4Family::BankNifty);
                assert_eq!(
                    nifty_family.terminal(),
                    AdmissionV4FamilyTerminal::Evaluated
                );
                assert_eq!(
                    banknifty_family.terminal(),
                    AdmissionV4FamilyTerminal::Evaluated
                );
                for (sequence, row) in source.rows().iter().enumerate() {
                    assert_eq!(
                        row.population().global_sequence(),
                        u64::try_from(sequence).map_err(|_| {
                            "Population V6 test sequence does not fit u64".to_owned()
                        })?
                    );
                    let candidate = row.population().candidate().row();
                    assert_eq!(candidate.sequence(), row.population().family_sequence());
                    assert_eq!(
                        candidate.candidate_semantic_digest(),
                        row.population().candidate_semantic_id()
                    );
                    assert_ne!(row.population().row_id(), [0; 32]);
                    assert_ne!(row.population().finalization_row_id(), [0; 32]);
                    assert_ne!(row.population().admission_decision_id(), [0; 32]);
                    assert_ne!(row.population().base_evidence_id(), [0; 32]);
                }
                Ok(())
            },
        )
    }
}
