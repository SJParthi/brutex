//! Receipt-last Execution V4 authority.
//!
//! This module is the version-separated persistence seam after Population V6.
//! It deliberately does not reinterpret a Population record. The sole
//! production door consumes and retains the live Population V6 capability,
//! reproduces its exact Candidate execution sources, and derives exactly two
//! Long/Short parameter records for each evaluated family, their percentile
//! atoms, and one Runner terminal disposition per evaluated source row. A
//! naturally extinct family contributes neither invented parameters nor
//! dispositions; its authenticated terminal envelope remains in Completion.
//! There is no caller-authored production constructor or detached-digest
//! overload in this file.
//!
//! Four append-only fixed-record files carry one committed block. Parameters,
//! percentile atoms, and dispositions are synced in that order before a
//! separate Completion is appended and synced. Physical offsets and block
//! sequence are structural and are excluded from semantic identities, so an
//! identical block has the same identities after relocation. Store versions
//! are never mutated in place.
//!
//! The execution-law digest binds stored one-minute OHLCV, a decision on the
//! current bar, entry on the next one-minute bar, one-minute exit-grid replay,
//! the 15:10 IST forced close, and the single-position policy. This module can
//! bind and authenticate that policy; the chronological proof that two
//! candidates never overlap belongs to the later global replay authority and
//! must not be inferred from an Execution V4 receipt alone.
//!
//! Preparing and validating a block is O(R + P), opening/reopening is O(F) in
//! explicitly bounded file bytes, and retained indexes consume O(B) space for
//! B committed blocks. Fixed-record offset arithmetic and isolated seek/decode
//! are O(1) in record count. An authenticated lookup deliberately rehashes all
//! retained bounded files and is O(F) in their total bytes; integrity is not
//! weakened to preserve an O(1) label. Filesystem latency, hashing, scanning,
//! duplicate-index construction, crash recovery, and append are not claimed to
//! be O(1).

#![expect(
    dead_code,
    reason = "Execution V4 remains crate-private until Selection V6 consumes its source-retaining production capability"
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
use runner::exit_grid_policy::{
    ExitGridSelectorV1, ForcedStopV1, RangeResolutionV1, RationalPercentileV1,
};

use crate::candidate_universe::CandidateExecutionParameterFactsV1;
use crate::population::{InstrumentFamilyV1, TradeDirectionV1};
use crate::population_admission_v4::{
    AdmissionV4DecisionStatus, AdmissionV4Family, AdmissionV4FamilyTerminal,
};
use crate::population_v6::{
    CommittedStoredPopulationV6, PopulationV6CandidateProjectionV1,
    PopulationV6ExecutionDispositionSourceV1, PopulationV6ExecutionV4SourceV1,
    PopulationV6FamilyProjectionV1, PopulationV6SourceProjectionV1, PopulationV6StructuralReceipt,
};

/// Bytes in one canonical Execution V4 parameter record.
pub(crate) const EXECUTION_V4_PARAMETER_BYTES: usize = 1_280;
/// Bytes in one canonical Execution V4 percentile atom.
pub(crate) const EXECUTION_V4_PERCENTILE_BYTES: usize = 128;
/// Bytes in one canonical Execution V4 terminal disposition.
pub(crate) const EXECUTION_V4_DISPOSITION_BYTES: usize = 1_024;
/// Bytes in one receipt-last Execution V4 Completion.
pub(crate) const EXECUTION_V4_COMPLETION_BYTES: usize = 1_280;

// Execution V4 is an independent append-only format. No Execution V3 byte is
// accepted or reinterpreted by this codec.
const VERSION: u32 = 4;
const PARAMETER_MAGIC: [u8; 16] = *b"BTX-EXV4-PARAM\0\0";
const PERCENTILE_MAGIC: [u8; 16] = *b"BTX-EXV4-PCTL\0\0\0";
const DISPOSITION_MAGIC: [u8; 16] = *b"BTX-EXV4-DISP\0\0\0";
const COMPLETION_MAGIC: [u8; 16] = *b"BTX-EXV4-CMPL\0\0\0";
const PARAMETER_DOMAIN: u32 = 1;
const PERCENTILE_DOMAIN: u32 = 2;
const DISPOSITION_DOMAIN: u32 = 3;
const COMPLETION_DOMAIN: u32 = 4;
const SEAL_BYTES: usize = 32;
const PARAMETER_PAYLOAD_BYTES: usize = EXECUTION_V4_PARAMETER_BYTES - SEAL_BYTES;
const PERCENTILE_PAYLOAD_BYTES: usize = EXECUTION_V4_PERCENTILE_BYTES - SEAL_BYTES;
const DISPOSITION_PAYLOAD_BYTES: usize = EXECUTION_V4_DISPOSITION_BYTES - SEAL_BYTES;
const COMPLETION_PAYLOAD_BYTES: usize = EXECUTION_V4_COMPLETION_BYTES - SEAL_BYTES;
const EVALUATION_FINGERPRINT_BYTES: usize = 155;
const READ_CHUNK_BYTES: usize = 16 * 1_024;
const MAX_PARAMETER_RECORDS_PER_BLOCK: usize = 4;
const MATRIX_CELLS: usize = 16;
const OPTIONAL_U32_NONE: u32 = u32::MAX;

const PARAMETER_CORE_ID_DOMAIN: &[u8] = b"brutex-execution-v4-parameter-core-id\0";
const PARAMETER_ID_DOMAIN: &[u8] = b"brutex-execution-v4-parameter-id\0";
const DISPOSITION_ID_DOMAIN: &[u8] = b"brutex-execution-v4-disposition-id\0";
const COMPLETION_ID_DOMAIN: &[u8] = b"brutex-execution-v4-completion-id\0";
const FAMILY_AUTHORITY_ID_DOMAIN: &[u8] = b"brutex-execution-v4-family-authority-id\0";
const ORDERED_PARAMETERS_DOMAIN: &[u8] = b"brutex-execution-v4-ordered-parameters\0";
const ORDERED_PERCENTILES_DOMAIN: &[u8] = b"brutex-execution-v4-ordered-percentiles\0";
const ORDERED_DISPOSITIONS_DOMAIN: &[u8] = b"brutex-execution-v4-ordered-dispositions\0";
const PARAMETER_SEAL_DOMAIN: &[u8] = b"brutex-execution-v4-parameter-seal\0";
const PERCENTILE_SEAL_DOMAIN: &[u8] = b"brutex-execution-v4-percentile-seal\0";
const DISPOSITION_SEAL_DOMAIN: &[u8] = b"brutex-execution-v4-disposition-seal\0";
const COMPLETION_SEAL_DOMAIN: &[u8] = b"brutex-execution-v4-completion-seal\0";
const FILE_GENERATION_DOMAIN: &[u8] = b"brutex-execution-v4-file-generation\0";

const PARAMETER_FILE: &str = "execution-parameters-v4.bin";
const PERCENTILE_FILE: &str = "execution-percentiles-v4.bin";
const DISPOSITION_FILE: &str = "execution-dispositions-v4.bin";
const COMPLETION_FILE: &str = "execution-completions-v4.bin";
const LOCK_FILE: &str = "execution-v4.lock";
const LOCK_MAX_BYTES: u64 = 0;

const EXECUTION_RESOLUTION_SECONDS: u32 = 60;
const ENTRY_DELAY_MINUTES: u32 = 1;
const FORCED_EXIT_IST_MINUTE: u32 = 15 * 60 + 10;
const FORCED_STOP_DISABLED_TAG: u8 = 0;
const FORCED_STOP_INCLUDE_TAG: u8 = 1;
const FORCED_STOP_REQUIRE_TAG: u8 = 2;

#[cfg(any(target_os = "android", target_os = "linux"))]
const O_NOFOLLOW_FLAG: i32 = 0x20_000;
#[cfg(target_os = "macos")]
const O_NOFOLLOW_FLAG: i32 = 0x100;

const _: () = assert!(PARAMETER_PAYLOAD_BYTES + SEAL_BYTES == EXECUTION_V4_PARAMETER_BYTES);
const _: () = assert!(PERCENTILE_PAYLOAD_BYTES + SEAL_BYTES == EXECUTION_V4_PERCENTILE_BYTES);
const _: () = assert!(DISPOSITION_PAYLOAD_BYTES + SEAL_BYTES == EXECUTION_V4_DISPOSITION_BYTES);
const _: () = assert!(COMPLETION_PAYLOAD_BYTES + SEAL_BYTES == EXECUTION_V4_COMPLETION_BYTES);

/// Operator-facing refusal at the Execution V4 boundary.
pub(crate) type ExecutionV4Refusal = String;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub(crate) enum ExecutionV4Family {
    Nifty = 1,
    BankNifty = 2,
}

/// Exact Population V6 family terminal retained even when the family has no
/// Candidate and therefore no Execution disposition row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum ExecutionV4FamilyTerminal {
    Evaluated = 1,
    NaturallyExtinct = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub(crate) enum ExecutionV4Direction {
    Long = 1,
    Short = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum ExecutionV4AdmissionStatus {
    Admitted = 1,
    Rejected = 2,
    Unmeasured = 3,
    Refused = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum ExecutionV4Terminal {
    Authorized = 1,
    PolicyRefused = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub(crate) enum ExecutionV4PercentileAxis {
    Stop = 1,
    Target = 2,
    Trail = 3,
}

/// Explicit fixed-file ceiling. There is deliberately no `Default`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExecutionV4FileBound {
    records: u64,
    bytes: u64,
}

impl ExecutionV4FileBound {
    pub(crate) fn new(
        max_records: u64,
        max_bytes: u64,
        stride: usize,
        name: &str,
    ) -> Result<Self, ExecutionV4Refusal> {
        if max_records == 0 || max_bytes == 0 {
            return Err(format!(
                "Execution V4 {name} record and byte ceilings must be nonzero"
            ));
        }
        let required = max_records
            .checked_mul(
                u64::try_from(stride)
                    .map_err(|_| format!("Execution V4 {name} stride does not fit u64"))?,
            )
            .ok_or_else(|| format!("Execution V4 {name} byte ceiling overflowed"))?;
        if max_bytes < required {
            return Err(format!(
                "Execution V4 {name} byte maximum {max_bytes} cannot hold {max_records} records ({required} bytes)"
            ));
        }
        Ok(Self {
            records: max_records,
            bytes: max_bytes,
        })
    }
}

/// Explicit ceilings for every V4 file and one semantic block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExecutionV4Bounds {
    parameters: ExecutionV4FileBound,
    percentiles: ExecutionV4FileBound,
    dispositions: ExecutionV4FileBound,
    completions: ExecutionV4FileBound,
    dispositions_per_block: u64,
    percentiles_per_block: u64,
}

impl ExecutionV4Bounds {
    pub(crate) fn new(
        parameters: ExecutionV4FileBound,
        percentiles: ExecutionV4FileBound,
        dispositions: ExecutionV4FileBound,
        completions: ExecutionV4FileBound,
        max_dispositions_per_block: u64,
        max_percentiles_per_block: u64,
    ) -> Result<Self, ExecutionV4Refusal> {
        if max_dispositions_per_block == 0 || max_percentiles_per_block == 0 {
            return Err(
                "Execution V4 per-block disposition and percentile ceilings must be nonzero"
                    .to_owned(),
            );
        }
        if max_dispositions_per_block > dispositions.records
            || max_percentiles_per_block > percentiles.records
            || parameters.records < MAX_PARAMETER_RECORDS_PER_BLOCK as u64
        {
            return Err(
                "Execution V4 per-block ceilings exceed their fixed-file record ceilings"
                    .to_owned(),
            );
        }
        Ok(Self {
            parameters,
            percentiles,
            dispositions,
            completions,
            dispositions_per_block: max_dispositions_per_block,
            percentiles_per_block: max_percentiles_per_block,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExecutionV4ParameterRecord {
    parameter_core_id: [u8; 32],
    parameter_id: [u8; 32],
    population_id: [u8; 32],
    population_completion_id: [u8; 32],
    population_ordered_digest: [u8; 32],
    source_finalization_id: [u8; 32],
    source_finalization_completion_id: [u8; 32],
    source_admission_block_id: [u8; 32],
    source_admission_completion_id: [u8; 32],
    policy_digest: [u8; 32],
    resolution_digest: [u8; 32],
    training_digest: [u8; 32],
    instrument_digest: [u8; 32],
    feed_digest: [u8; 32],
    commit_digest: [u8; 32],
    calendar_digest: [u8; 32],
    percentile_digest: [u8; 32],
    cost_model_id: [u8; 32],
    execution_law_digest: [u8; 32],
    evaluation_fingerprint: [u8; EVALUATION_FINGERPRINT_BYTES],
    family: ExecutionV4Family,
    direction: ExecutionV4Direction,
    range_policy_tag: u8,
    selector_policy_tag: u8,
    forced_stop_policy_tag: u8,
    rung: u32,
    horizon_bars: u32,
    execution_resolution_seconds: u32,
    entry_delay_minutes: u32,
    forced_exit_ist_minute: u32,
    forced_stop_index: Option<u32>,
    run_params: [u64; 4],
    max_levels: u64,
    ratio_min_hundredths: i64,
    ratio_max_hundredths: i64,
    max_ratio_pairs: u64,
    policy_max_cells: u64,
    resolved_stop_count: u64,
    resolved_target_count: u64,
    resolved_trail_count: u64,
    resolved_ratio_pair_count: u64,
    resolved_cell_count: u64,
    forced_stop_ppm: i64,
    max_ambiguity_bars: u64,
    max_gap_bars: u64,
    training_bars: u64,
    training_first_ts_micros: i64,
    training_last_ts_micros: i64,
    percentile_offset: u64,
    percentile_count: u64,
}

impl ExecutionV4ParameterRecord {
    fn validate(&self) -> Result<(), ExecutionV4Refusal> {
        for (name, value) in [
            ("parameter core identity", self.parameter_core_id),
            ("parameter identity", self.parameter_id),
            ("Population V6 identity", self.population_id),
            ("Population V6 Completion", self.population_completion_id),
            (
                "Population V6 ordered Candidate digest",
                self.population_ordered_digest,
            ),
            ("Finalization V4 identity", self.source_finalization_id),
            (
                "Finalization V4 Completion",
                self.source_finalization_completion_id,
            ),
            ("Admission V4 block", self.source_admission_block_id),
            (
                "Admission V4 Completion",
                self.source_admission_completion_id,
            ),
            ("grid policy", self.policy_digest),
            ("resolution policy", self.resolution_digest),
            ("training digest", self.training_digest),
            ("instrument digest", self.instrument_digest),
            ("feed digest", self.feed_digest),
            ("commit digest", self.commit_digest),
            ("calendar digest", self.calendar_digest),
            ("percentile digest", self.percentile_digest),
            ("cost model identity", self.cost_model_id),
            ("execution law", self.execution_law_digest),
        ] {
            require_nonzero(name, value)?;
        }
        if self.execution_law_digest != execution_law_digest()
            || self.execution_resolution_seconds != EXECUTION_RESOLUTION_SECONDS
            || self.entry_delay_minutes != ENTRY_DELAY_MINUTES
            || self.forced_exit_ist_minute != FORCED_EXIT_IST_MINUTE
        {
            return Err(
                "Execution V4 parameter does not bind one-minute OHLCV, next-minute entry and 15:10 IST close"
                    .to_owned(),
            );
        }
        let [min_hits, ceiling, pair_budget, _policy] = self.run_params;
        if !matches!(self.range_policy_tag, 1 | 2)
            || !matches!(self.selector_policy_tag, 1..=3)
            || self.rung == 0
            || self.horizon_bars == 0
            || min_hits == 0
            || ceiling == 0
            || pair_budget == 0
            || self.max_levels == 0
            || self.max_ratio_pairs == 0
            || self.policy_max_cells == 0
            || self.resolved_stop_count == 0
            || self.resolved_target_count == 0
            || self.resolved_trail_count == 0
            || self.resolved_ratio_pair_count == 0
            || self.resolved_cell_count == 0
            || self.training_bars == 0
            || self.percentile_count == 0
        {
            return Err("Execution V4 parameter contains a zero required bound/policy".to_owned());
        }
        let possible_ratio_pairs = self
            .resolved_stop_count
            .checked_mul(self.resolved_target_count)
            .ok_or_else(|| "Execution V4 resolved ratio-pair envelope overflowed".to_owned())?;
        self.validate_forced_stop()?;
        let max_resolved_stops = self
            .max_levels
            .checked_add(u64::from(
                self.forced_stop_policy_tag != FORCED_STOP_DISABLED_TAG,
            ))
            .ok_or_else(|| "Execution V4 forced-stop-grown axis bound overflowed".to_owned())?;
        if self.ratio_min_hundredths <= 0
            || self.ratio_max_hundredths < self.ratio_min_hundredths
            || self.resolved_stop_count > max_resolved_stops
            || self.resolved_target_count > self.max_levels
            || self.resolved_trail_count > self.max_levels
            || self.resolved_ratio_pair_count > self.max_ratio_pairs
            || self.resolved_ratio_pair_count > possible_ratio_pairs
            || self.resolved_cell_count > self.policy_max_cells
            || self.training_first_ts_micros > self.training_last_ts_micros
        {
            return Err("Execution V4 parameter bounds are contradictory or overflow".to_owned());
        }
        if self.evaluation_fingerprint.iter().all(|byte| *byte == 0) {
            return Err("Execution V4 evaluation fingerprint is zero".to_owned());
        }
        if self.parameter_core_id != self.derive_core_id() {
            return Err("Execution V4 parameter core identity does not reproduce".to_owned());
        }
        if self.parameter_id != self.derive_parameter_id() {
            return Err("Execution V4 parameter identity does not reproduce".to_owned());
        }
        Ok(())
    }

    fn validate_forced_stop(&self) -> Result<(), ExecutionV4Refusal> {
        match (
            self.forced_stop_policy_tag,
            self.forced_stop_ppm,
            self.forced_stop_index,
        ) {
            (FORCED_STOP_DISABLED_TAG, 0, None) => Ok(()),
            (FORCED_STOP_INCLUDE_TAG | FORCED_STOP_REQUIRE_TAG, ppm, Some(index))
                if ppm > 0
                    && index != OPTIONAL_U32_NONE
                    && u64::from(index) < self.resolved_stop_count =>
            {
                Ok(())
            }
            (FORCED_STOP_DISABLED_TAG..=FORCED_STOP_REQUIRE_TAG, _, _) => Err(
                "Execution V4 forced-stop tag, positive PPM and resolved index disagree".to_owned(),
            ),
            _ => Err("Execution V4 forced-stop policy tag is unknown".to_owned()),
        }
    }

    fn derive_core_id(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(PARAMETER_CORE_ID_DOMAIN);
        hasher.update(&VERSION.to_le_bytes());
        for value in [
            self.population_id,
            self.population_completion_id,
            self.population_ordered_digest,
            self.source_finalization_id,
            self.source_finalization_completion_id,
            self.source_admission_block_id,
            self.source_admission_completion_id,
            self.policy_digest,
            self.resolution_digest,
            self.training_digest,
            self.instrument_digest,
            self.feed_digest,
            self.commit_digest,
            self.calendar_digest,
            self.cost_model_id,
            self.execution_law_digest,
        ] {
            hasher.update(&value);
        }
        hasher.update(&self.evaluation_fingerprint);
        hasher.update(&[
            self.family as u8,
            self.direction as u8,
            self.range_policy_tag,
            self.selector_policy_tag,
            self.forced_stop_policy_tag,
        ]);
        for value in [
            self.rung,
            self.horizon_bars,
            self.execution_resolution_seconds,
            self.entry_delay_minutes,
            self.forced_exit_ist_minute,
            encode_optional_u32(self.forced_stop_index),
        ] {
            hasher.update(&value.to_le_bytes());
        }
        for value in self.run_params {
            hasher.update(&value.to_le_bytes());
        }
        hasher.update(&self.ratio_min_hundredths.to_le_bytes());
        hasher.update(&self.ratio_max_hundredths.to_le_bytes());
        for value in [
            self.max_levels,
            self.max_ratio_pairs,
            self.policy_max_cells,
            self.resolved_stop_count,
            self.resolved_target_count,
            self.resolved_trail_count,
            self.resolved_ratio_pair_count,
            self.resolved_cell_count,
            self.max_ambiguity_bars,
            self.max_gap_bars,
            self.training_bars,
            u64::from_le_bytes(self.training_first_ts_micros.to_le_bytes()),
            u64::from_le_bytes(self.training_last_ts_micros.to_le_bytes()),
            self.percentile_offset,
            self.percentile_count,
        ] {
            hasher.update(&value.to_le_bytes());
        }
        hasher.update(&self.forced_stop_ppm.to_le_bytes());
        hasher.finalize()
    }

    fn derive_parameter_id(&self) -> [u8; 32] {
        hash_parts(
            PARAMETER_ID_DOMAIN,
            &[
                &VERSION.to_le_bytes(),
                &self.parameter_core_id,
                &self.percentile_digest,
            ],
        )
    }

    fn encode(&self) -> Result<[u8; EXECUTION_V4_PARAMETER_BYTES], ExecutionV4Refusal> {
        self.validate()?;
        let mut raw = [0_u8; EXECUTION_V4_PARAMETER_BYTES];
        let (payload, seal) = raw.split_at_mut(PARAMETER_PAYLOAD_BYTES);
        let mut writer = FixedWriter::new(payload);
        writer.array(&PARAMETER_MAGIC)?;
        writer.u32(VERSION)?;
        writer.u32(PARAMETER_DOMAIN)?;
        for value in [
            self.parameter_core_id,
            self.parameter_id,
            self.population_id,
            self.population_completion_id,
            self.population_ordered_digest,
            self.source_finalization_id,
            self.source_finalization_completion_id,
            self.source_admission_block_id,
            self.source_admission_completion_id,
            self.policy_digest,
            self.resolution_digest,
            self.training_digest,
            self.instrument_digest,
            self.feed_digest,
            self.commit_digest,
            self.calendar_digest,
            self.percentile_digest,
            self.cost_model_id,
            self.execution_law_digest,
        ] {
            writer.array(&value)?;
        }
        writer.array(&self.evaluation_fingerprint)?;
        writer.u8(self.family as u8)?;
        writer.u8(self.direction as u8)?;
        writer.u8(self.range_policy_tag)?;
        writer.u8(self.selector_policy_tag)?;
        writer.u8(self.forced_stop_policy_tag)?;
        writer.zeros(3)?;
        for value in [
            self.rung,
            self.horizon_bars,
            self.execution_resolution_seconds,
            self.entry_delay_minutes,
            self.forced_exit_ist_minute,
            encode_optional_u32(self.forced_stop_index),
        ] {
            writer.u32(value)?;
        }
        for value in self.run_params {
            writer.u64(value)?;
        }
        writer.i64(self.ratio_min_hundredths)?;
        writer.i64(self.ratio_max_hundredths)?;
        for value in [
            self.max_levels,
            self.max_ratio_pairs,
            self.policy_max_cells,
            self.resolved_stop_count,
            self.resolved_target_count,
            self.resolved_trail_count,
            self.resolved_ratio_pair_count,
            self.resolved_cell_count,
            self.max_ambiguity_bars,
            self.max_gap_bars,
            self.training_bars,
        ] {
            writer.u64(value)?;
        }
        writer.i64(self.forced_stop_ppm)?;
        writer.i64(self.training_first_ts_micros)?;
        writer.i64(self.training_last_ts_micros)?;
        writer.u64(self.percentile_offset)?;
        writer.u64(self.percentile_count)?;
        writer.zeros(writer.remaining())?;
        writer.require_full("parameter payload")?;
        seal.copy_from_slice(&hash_parts(PARAMETER_SEAL_DOMAIN, &[payload]));
        Ok(raw)
    }

    fn decode(raw: &[u8; EXECUTION_V4_PARAMETER_BYTES]) -> Result<Self, ExecutionV4Refusal> {
        let (payload, seal) = raw.split_at(PARAMETER_PAYLOAD_BYTES);
        require_seal("parameter", PARAMETER_SEAL_DOMAIN, payload, seal)?;
        let mut reader = FixedReader::new(payload);
        require_header("parameter", &mut reader, PARAMETER_MAGIC, PARAMETER_DOMAIN)?;
        let parameter = Self {
            parameter_core_id: reader.array()?,
            parameter_id: reader.array()?,
            population_id: reader.array()?,
            population_completion_id: reader.array()?,
            population_ordered_digest: reader.array()?,
            source_finalization_id: reader.array()?,
            source_finalization_completion_id: reader.array()?,
            source_admission_block_id: reader.array()?,
            source_admission_completion_id: reader.array()?,
            policy_digest: reader.array()?,
            resolution_digest: reader.array()?,
            training_digest: reader.array()?,
            instrument_digest: reader.array()?,
            feed_digest: reader.array()?,
            commit_digest: reader.array()?,
            calendar_digest: reader.array()?,
            percentile_digest: reader.array()?,
            cost_model_id: reader.array()?,
            execution_law_digest: reader.array()?,
            evaluation_fingerprint: reader.array()?,
            family: decode_family(reader.u8()?)?,
            direction: decode_direction(reader.u8()?)?,
            range_policy_tag: reader.u8()?,
            selector_policy_tag: reader.u8()?,
            forced_stop_policy_tag: reader.u8()?,
            rung: {
                reader.require_zeros(3, "parameter tag reserve")?;
                reader.u32()?
            },
            horizon_bars: reader.u32()?,
            execution_resolution_seconds: reader.u32()?,
            entry_delay_minutes: reader.u32()?,
            forced_exit_ist_minute: reader.u32()?,
            forced_stop_index: decode_optional_u32(reader.u32()?),
            run_params: [reader.u64()?, reader.u64()?, reader.u64()?, reader.u64()?],
            ratio_min_hundredths: reader.i64()?,
            ratio_max_hundredths: reader.i64()?,
            max_levels: reader.u64()?,
            max_ratio_pairs: reader.u64()?,
            policy_max_cells: reader.u64()?,
            resolved_stop_count: reader.u64()?,
            resolved_target_count: reader.u64()?,
            resolved_trail_count: reader.u64()?,
            resolved_ratio_pair_count: reader.u64()?,
            resolved_cell_count: reader.u64()?,
            max_ambiguity_bars: reader.u64()?,
            max_gap_bars: reader.u64()?,
            training_bars: reader.u64()?,
            forced_stop_ppm: reader.i64()?,
            training_first_ts_micros: reader.i64()?,
            training_last_ts_micros: reader.i64()?,
            percentile_offset: reader.u64()?,
            percentile_count: reader.u64()?,
        };
        reader.require_zeros(reader.remaining(), "parameter trailing reserve")?;
        parameter.validate()?;
        if parameter.encode()? != *raw {
            return Err("Execution V4 parameter is not byte-canonical".to_owned());
        }
        Ok(parameter)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExecutionV4PercentileRecord {
    parameter_core_id: [u8; 32],
    axis: ExecutionV4PercentileAxis,
    ordinal: u32,
    numerator: u32,
    denominator: u32,
}

impl ExecutionV4PercentileRecord {
    fn validate(&self) -> Result<(), ExecutionV4Refusal> {
        require_nonzero("percentile parameter core identity", self.parameter_core_id)?;
        if self.denominator == 0 || self.numerator > self.denominator {
            return Err("Execution V4 percentile ratio is outside [0,1]".to_owned());
        }
        Ok(())
    }

    fn encode(&self) -> Result<[u8; EXECUTION_V4_PERCENTILE_BYTES], ExecutionV4Refusal> {
        self.validate()?;
        let mut raw = [0_u8; EXECUTION_V4_PERCENTILE_BYTES];
        let (payload, seal) = raw.split_at_mut(PERCENTILE_PAYLOAD_BYTES);
        let mut writer = FixedWriter::new(payload);
        writer.array(&PERCENTILE_MAGIC)?;
        writer.u32(VERSION)?;
        writer.u32(PERCENTILE_DOMAIN)?;
        writer.array(&self.parameter_core_id)?;
        writer.u8(self.axis as u8)?;
        writer.zeros(3)?;
        writer.u32(self.ordinal)?;
        writer.u32(self.numerator)?;
        writer.u32(self.denominator)?;
        writer.zeros(writer.remaining())?;
        writer.require_full("percentile payload")?;
        seal.copy_from_slice(&hash_parts(PERCENTILE_SEAL_DOMAIN, &[payload]));
        Ok(raw)
    }

    fn decode(raw: &[u8; EXECUTION_V4_PERCENTILE_BYTES]) -> Result<Self, ExecutionV4Refusal> {
        let (payload, seal) = raw.split_at(PERCENTILE_PAYLOAD_BYTES);
        require_seal("percentile", PERCENTILE_SEAL_DOMAIN, payload, seal)?;
        let mut reader = FixedReader::new(payload);
        require_header(
            "percentile",
            &mut reader,
            PERCENTILE_MAGIC,
            PERCENTILE_DOMAIN,
        )?;
        let percentile = Self {
            parameter_core_id: reader.array()?,
            axis: decode_percentile_axis(reader.u8()?)?,
            ordinal: {
                reader.require_zeros(3, "percentile axis reserve")?;
                reader.u32()?
            },
            numerator: reader.u32()?,
            denominator: reader.u32()?,
        };
        reader.require_zeros(reader.remaining(), "percentile trailing reserve")?;
        percentile.validate()?;
        if percentile.encode()? != *raw {
            return Err("Execution V4 percentile is not byte-canonical".to_owned());
        }
        Ok(percentile)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExecutionV4DispositionRecord {
    disposition_id: [u8; 32],
    population_id: [u8; 32],
    population_completion_id: [u8; 32],
    population_family_row_id: [u8; 32],
    population_row_id: [u8; 32],
    candidate_semantic_id: [u8; 32],
    candidate_base_row_id: [u8; 32],
    base_evidence_id: [u8; 32],
    admission_decision_id: [u8; 32],
    finalization_family_row_id: [u8; 32],
    finalization_row_id: [u8; 32],
    finalization_completion_id: [u8; 32],
    parameter_id: [u8; 32],
    execution_run_id: [u8; 32],
    evaluated_grid_digest: [u8; 32],
    resolution_digest: [u8; 32],
    column_digest: [u8; 32],
    context_digest: [u8; 32],
    runner_disposition_digest: [u8; 32],
    selected_exit_digest: [u8; 32],
    global_sequence: u64,
    family_sequence: u64,
    cell_ordinal: u64,
    support_hits: u64,
    refusal_bits: u64,
    family: ExecutionV4Family,
    direction: ExecutionV4Direction,
    admission_status: ExecutionV4AdmissionStatus,
    terminal: ExecutionV4Terminal,
    rung: u32,
    horizon_bars: u32,
    stop_index: Option<u32>,
    target_index: Option<u32>,
    tsl_index: Option<u32>,
    ttp_arm_index: Option<u32>,
    ttp_trail_index: Option<u32>,
}

impl ExecutionV4DispositionRecord {
    fn validate(&self) -> Result<(), ExecutionV4Refusal> {
        for (name, value) in [
            ("disposition identity", self.disposition_id),
            ("Population V6 identity", self.population_id),
            ("Population V6 Completion", self.population_completion_id),
            ("Population V6 Family row", self.population_family_row_id),
            ("Population V6 Candidate row", self.population_row_id),
            ("Candidate semantic identity", self.candidate_semantic_id),
            ("Candidate base row", self.candidate_base_row_id),
            ("Base Evidence V2 row", self.base_evidence_id),
            ("Admission V4 decision", self.admission_decision_id),
            (
                "Finalization V4 Family row",
                self.finalization_family_row_id,
            ),
            ("Finalization V4 row", self.finalization_row_id),
            (
                "Finalization V4 Completion",
                self.finalization_completion_id,
            ),
            ("execution parameter", self.parameter_id),
            ("execution run", self.execution_run_id),
            ("evaluated grid", self.evaluated_grid_digest),
            ("resolution digest", self.resolution_digest),
            ("indicator column", self.column_digest),
            ("causal execution context", self.context_digest),
            (
                "Runner terminal disposition",
                self.runner_disposition_digest,
            ),
        ] {
            require_nonzero(name, value)?;
        }
        if self.rung == 0 || self.horizon_bars == 0 || self.support_hits == 0 {
            return Err("Execution V4 disposition has a zero required fact".to_owned());
        }
        if self.ttp_arm_index.is_some() != self.ttp_trail_index.is_some() {
            return Err("Execution V4 disposition contains a half TTP coordinate pair".to_owned());
        }
        match self.terminal {
            ExecutionV4Terminal::Authorized => {
                if self.selected_exit_digest == [0; 32]
                    || self.refusal_bits != 0
                    || self.stop_index.is_none()
                    || self.target_index.is_none()
                {
                    return Err(
                        "Execution V4 Authorized terminal lacks selected-exit evidence or carries refusal bits"
                            .to_owned(),
                    );
                }
            }
            ExecutionV4Terminal::PolicyRefused => {
                if self.selected_exit_digest != [0; 32]
                    || self.refusal_bits == 0
                    || runner::exit_grid_policy::ExecutionRefusalBitsV1::from_bits(
                        self.refusal_bits,
                    )
                    .is_none()
                {
                    return Err(
                        "Execution V4 PolicyRefused terminal contains invented selected-exit evidence or lacks exact known refusal bits"
                            .to_owned(),
                    );
                }
            }
        }
        if self.disposition_id != self.derive_id() {
            return Err("Execution V4 disposition identity does not reproduce".to_owned());
        }
        Ok(())
    }

    fn derive_id(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(DISPOSITION_ID_DOMAIN);
        hasher.update(&VERSION.to_le_bytes());
        for value in [
            self.population_id,
            self.population_completion_id,
            self.population_family_row_id,
            self.population_row_id,
            self.candidate_semantic_id,
            self.candidate_base_row_id,
            self.base_evidence_id,
            self.admission_decision_id,
            self.finalization_family_row_id,
            self.finalization_row_id,
            self.finalization_completion_id,
            self.parameter_id,
            self.execution_run_id,
            self.evaluated_grid_digest,
            self.resolution_digest,
            self.column_digest,
            self.context_digest,
            self.runner_disposition_digest,
            self.selected_exit_digest,
        ] {
            hasher.update(&value);
        }
        for value in [
            self.global_sequence,
            self.family_sequence,
            self.cell_ordinal,
            self.support_hits,
            self.refusal_bits,
        ] {
            hasher.update(&value.to_le_bytes());
        }
        hasher.update(&[
            self.family as u8,
            self.direction as u8,
            self.admission_status as u8,
            self.terminal as u8,
        ]);
        for value in [
            self.rung,
            self.horizon_bars,
            encode_optional_u32(self.stop_index),
            encode_optional_u32(self.target_index),
            encode_optional_u32(self.tsl_index),
            encode_optional_u32(self.ttp_arm_index),
            encode_optional_u32(self.ttp_trail_index),
        ] {
            hasher.update(&value.to_le_bytes());
        }
        hasher.finalize()
    }

    fn encode(&self) -> Result<[u8; EXECUTION_V4_DISPOSITION_BYTES], ExecutionV4Refusal> {
        self.validate()?;
        let mut raw = [0_u8; EXECUTION_V4_DISPOSITION_BYTES];
        let (payload, seal) = raw.split_at_mut(DISPOSITION_PAYLOAD_BYTES);
        let mut writer = FixedWriter::new(payload);
        writer.array(&DISPOSITION_MAGIC)?;
        writer.u32(VERSION)?;
        writer.u32(DISPOSITION_DOMAIN)?;
        for value in [
            self.disposition_id,
            self.population_id,
            self.population_completion_id,
            self.population_family_row_id,
            self.population_row_id,
            self.candidate_semantic_id,
            self.candidate_base_row_id,
            self.base_evidence_id,
            self.admission_decision_id,
            self.finalization_family_row_id,
            self.finalization_row_id,
            self.finalization_completion_id,
            self.parameter_id,
            self.execution_run_id,
            self.evaluated_grid_digest,
            self.resolution_digest,
            self.column_digest,
            self.context_digest,
            self.runner_disposition_digest,
            self.selected_exit_digest,
        ] {
            writer.array(&value)?;
        }
        for value in [
            self.global_sequence,
            self.family_sequence,
            self.cell_ordinal,
            self.support_hits,
            self.refusal_bits,
        ] {
            writer.u64(value)?;
        }
        writer.u8(self.family as u8)?;
        writer.u8(self.direction as u8)?;
        writer.u8(self.admission_status as u8)?;
        writer.u8(self.terminal as u8)?;
        writer.zeros(4)?;
        for value in [
            self.rung,
            self.horizon_bars,
            encode_optional_u32(self.stop_index),
            encode_optional_u32(self.target_index),
            encode_optional_u32(self.tsl_index),
            encode_optional_u32(self.ttp_arm_index),
            encode_optional_u32(self.ttp_trail_index),
        ] {
            writer.u32(value)?;
        }
        writer.zeros(writer.remaining())?;
        writer.require_full("disposition payload")?;
        seal.copy_from_slice(&hash_parts(DISPOSITION_SEAL_DOMAIN, &[payload]));
        Ok(raw)
    }

    fn decode(raw: &[u8; EXECUTION_V4_DISPOSITION_BYTES]) -> Result<Self, ExecutionV4Refusal> {
        let (payload, seal) = raw.split_at(DISPOSITION_PAYLOAD_BYTES);
        require_seal("disposition", DISPOSITION_SEAL_DOMAIN, payload, seal)?;
        let mut reader = FixedReader::new(payload);
        require_header(
            "disposition",
            &mut reader,
            DISPOSITION_MAGIC,
            DISPOSITION_DOMAIN,
        )?;
        let disposition = Self {
            disposition_id: reader.array()?,
            population_id: reader.array()?,
            population_completion_id: reader.array()?,
            population_family_row_id: reader.array()?,
            population_row_id: reader.array()?,
            candidate_semantic_id: reader.array()?,
            candidate_base_row_id: reader.array()?,
            base_evidence_id: reader.array()?,
            admission_decision_id: reader.array()?,
            finalization_family_row_id: reader.array()?,
            finalization_row_id: reader.array()?,
            finalization_completion_id: reader.array()?,
            parameter_id: reader.array()?,
            execution_run_id: reader.array()?,
            evaluated_grid_digest: reader.array()?,
            resolution_digest: reader.array()?,
            column_digest: reader.array()?,
            context_digest: reader.array()?,
            runner_disposition_digest: reader.array()?,
            selected_exit_digest: reader.array()?,
            global_sequence: reader.u64()?,
            family_sequence: reader.u64()?,
            cell_ordinal: reader.u64()?,
            support_hits: reader.u64()?,
            refusal_bits: reader.u64()?,
            family: decode_family(reader.u8()?)?,
            direction: decode_direction(reader.u8()?)?,
            admission_status: decode_admission_status(reader.u8()?)?,
            terminal: decode_terminal(reader.u8()?)?,
            rung: {
                reader.require_zeros(4, "disposition tag reserve")?;
                reader.u32()?
            },
            horizon_bars: reader.u32()?,
            stop_index: decode_optional_u32(reader.u32()?),
            target_index: decode_optional_u32(reader.u32()?),
            tsl_index: decode_optional_u32(reader.u32()?),
            ttp_arm_index: decode_optional_u32(reader.u32()?),
            ttp_trail_index: decode_optional_u32(reader.u32()?),
        };
        reader.require_zeros(reader.remaining(), "disposition trailing reserve")?;
        disposition.validate()?;
        if disposition.encode()? != *raw {
            return Err("Execution V4 disposition is not byte-canonical".to_owned());
        }
        Ok(disposition)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExecutionV4CompletionRecord {
    block_sequence: u64,
    first_parameter_record: u64,
    parameter_count: u64,
    first_percentile_record: u64,
    percentile_count: u64,
    first_disposition_record: u64,
    disposition_count: u64,
    completion_id: [u8; 32],
    population_id: [u8; 32],
    population_completion_id: [u8; 32],
    population_ordered_digest: [u8; 32],
    nifty_population_family_row_id: [u8; 32],
    banknifty_population_family_row_id: [u8; 32],
    nifty_finalization_family_row_id: [u8; 32],
    banknifty_finalization_family_row_id: [u8; 32],
    source_finalization_id: [u8; 32],
    source_finalization_completion_id: [u8; 32],
    source_admission_block_id: [u8; 32],
    source_admission_completion_id: [u8; 32],
    execution_law_digest: [u8; 32],
    ordered_parameter_digest: [u8; 32],
    ordered_percentile_digest: [u8; 32],
    ordered_disposition_digest: [u8; 32],
    nifty_disposition_digest: [u8; 32],
    banknifty_disposition_digest: [u8; 32],
    nifty_authority_id: [u8; 32],
    banknifty_authority_id: [u8; 32],
    parameter_ids: [[u8; 32]; MAX_PARAMETER_RECORDS_PER_BLOCK],
    row_count: u64,
    evaluated_count: u64,
    decision_count: u64,
    nifty_count: u64,
    nifty_evaluated_count: u64,
    nifty_decision_count: u64,
    banknifty_count: u64,
    banknifty_evaluated_count: u64,
    banknifty_decision_count: u64,
    authorized_count: u64,
    policy_refused_count: u64,
    rung: u32,
    horizon_bars: u32,
    nifty_terminal: ExecutionV4FamilyTerminal,
    banknifty_terminal: ExecutionV4FamilyTerminal,
    admission_counts: [u64; 4],
    terminal_matrix: [u64; MATRIX_CELLS],
}

impl ExecutionV4CompletionRecord {
    fn validate(&self) -> Result<(), ExecutionV4Refusal> {
        for (name, value) in [
            ("Completion identity", self.completion_id),
            ("Population V6 identity", self.population_id),
            ("Population V6 Completion", self.population_completion_id),
            (
                "Population V6 ordered Candidate digest",
                self.population_ordered_digest,
            ),
            (
                "Population V6 NIFTY Family row",
                self.nifty_population_family_row_id,
            ),
            (
                "Population V6 BANKNIFTY Family row",
                self.banknifty_population_family_row_id,
            ),
            (
                "Finalization V4 NIFTY Family row",
                self.nifty_finalization_family_row_id,
            ),
            (
                "Finalization V4 BANKNIFTY Family row",
                self.banknifty_finalization_family_row_id,
            ),
            ("Finalization V4 identity", self.source_finalization_id),
            (
                "Finalization V4 Completion",
                self.source_finalization_completion_id,
            ),
            ("Admission V4 block", self.source_admission_block_id),
            (
                "Admission V4 Completion",
                self.source_admission_completion_id,
            ),
            ("execution law", self.execution_law_digest),
            ("ordered parameters", self.ordered_parameter_digest),
            ("ordered percentiles", self.ordered_percentile_digest),
            ("ordered dispositions", self.ordered_disposition_digest),
            ("NIFTY dispositions", self.nifty_disposition_digest),
            ("BANKNIFTY dispositions", self.banknifty_disposition_digest),
            ("NIFTY execution authority", self.nifty_authority_id),
            ("BANKNIFTY execution authority", self.banknifty_authority_id),
        ] {
            require_nonzero(name, value)?;
        }
        validate_parameter_id_slots(
            self.parameter_ids,
            self.nifty_terminal,
            self.banknifty_terminal,
            self.parameter_count,
        )?;
        if self.execution_law_digest != execution_law_digest()
            || self.rung == 0
            || self.horizon_bars == 0
            || (self.parameter_count == 0) != (self.percentile_count == 0)
            || self.disposition_count != self.row_count
            || self.evaluated_count != self.row_count
            || self.decision_count != self.row_count
            || checked_sum(&[self.nifty_count, self.banknifty_count], "family counts")?
                != self.row_count
            || checked_sum(
                &[self.nifty_evaluated_count, self.banknifty_evaluated_count],
                "family evaluated counts",
            )? != self.evaluated_count
            || checked_sum(
                &[self.nifty_decision_count, self.banknifty_decision_count],
                "family decision counts",
            )? != self.decision_count
            || checked_sum(
                &[self.authorized_count, self.policy_refused_count],
                "terminal counts",
            )? != self.row_count
            || checked_sum(&self.admission_counts, "admission counts")? != self.row_count
            || checked_sum(&self.terminal_matrix, "terminal matrix")? != self.row_count
        {
            return Err("Execution V4 Completion counts do not cover the block exactly".to_owned());
        }
        let nifty_dispositions = checked_sum(&self.terminal_matrix[..8], "NIFTY matrix")?;
        let banknifty_dispositions = checked_sum(&self.terminal_matrix[8..], "BANKNIFTY matrix")?;
        validate_terminal_envelope(
            self.nifty_terminal,
            self.nifty_count,
            self.nifty_evaluated_count,
            self.nifty_decision_count,
            nifty_dispositions,
        )?;
        validate_terminal_envelope(
            self.banknifty_terminal,
            self.banknifty_count,
            self.banknifty_evaluated_count,
            self.banknifty_decision_count,
            banknifty_dispositions,
        )?;
        if self.completion_id != self.derive_completion_id() {
            return Err("Execution V4 Completion identity does not reproduce".to_owned());
        }
        Ok(())
    }

    fn derive_completion_id(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(COMPLETION_ID_DOMAIN);
        hasher.update(&VERSION.to_le_bytes());
        for value in [
            self.population_id,
            self.population_completion_id,
            self.population_ordered_digest,
            self.nifty_population_family_row_id,
            self.banknifty_population_family_row_id,
            self.nifty_finalization_family_row_id,
            self.banknifty_finalization_family_row_id,
            self.source_finalization_id,
            self.source_finalization_completion_id,
            self.source_admission_block_id,
            self.source_admission_completion_id,
            self.execution_law_digest,
            self.ordered_parameter_digest,
            self.ordered_percentile_digest,
            self.ordered_disposition_digest,
            self.nifty_disposition_digest,
            self.banknifty_disposition_digest,
            self.nifty_authority_id,
            self.banknifty_authority_id,
        ] {
            hasher.update(&value);
        }
        for parameter_id in self.parameter_ids {
            hasher.update(&parameter_id);
        }
        for value in [
            self.parameter_count,
            self.percentile_count,
            self.disposition_count,
            self.row_count,
            self.evaluated_count,
            self.decision_count,
            self.nifty_count,
            self.nifty_evaluated_count,
            self.nifty_decision_count,
            self.banknifty_count,
            self.banknifty_evaluated_count,
            self.banknifty_decision_count,
            self.authorized_count,
            self.policy_refused_count,
        ] {
            hasher.update(&value.to_le_bytes());
        }
        hasher.update(&self.rung.to_le_bytes());
        hasher.update(&self.horizon_bars.to_le_bytes());
        hasher.update(&[self.nifty_terminal as u8, self.banknifty_terminal as u8]);
        for value in self.admission_counts {
            hasher.update(&value.to_le_bytes());
        }
        for value in self.terminal_matrix {
            hasher.update(&value.to_le_bytes());
        }
        hasher.finalize()
    }

    fn encode(&self) -> Result<[u8; EXECUTION_V4_COMPLETION_BYTES], ExecutionV4Refusal> {
        self.validate()?;
        let mut raw = [0_u8; EXECUTION_V4_COMPLETION_BYTES];
        let (payload, seal) = raw.split_at_mut(COMPLETION_PAYLOAD_BYTES);
        let mut writer = FixedWriter::new(payload);
        writer.array(&COMPLETION_MAGIC)?;
        writer.u32(VERSION)?;
        writer.u32(COMPLETION_DOMAIN)?;
        for value in [
            self.block_sequence,
            self.first_parameter_record,
            self.parameter_count,
            self.first_percentile_record,
            self.percentile_count,
            self.first_disposition_record,
            self.disposition_count,
        ] {
            writer.u64(value)?;
        }
        for value in [
            self.completion_id,
            self.population_id,
            self.population_completion_id,
            self.population_ordered_digest,
            self.nifty_population_family_row_id,
            self.banknifty_population_family_row_id,
            self.nifty_finalization_family_row_id,
            self.banknifty_finalization_family_row_id,
            self.source_finalization_id,
            self.source_finalization_completion_id,
            self.source_admission_block_id,
            self.source_admission_completion_id,
            self.execution_law_digest,
            self.ordered_parameter_digest,
            self.ordered_percentile_digest,
            self.ordered_disposition_digest,
            self.nifty_disposition_digest,
            self.banknifty_disposition_digest,
            self.nifty_authority_id,
            self.banknifty_authority_id,
        ] {
            writer.array(&value)?;
        }
        for parameter_id in self.parameter_ids {
            writer.array(&parameter_id)?;
        }
        for value in [
            self.row_count,
            self.evaluated_count,
            self.decision_count,
            self.nifty_count,
            self.nifty_evaluated_count,
            self.nifty_decision_count,
            self.banknifty_count,
            self.banknifty_evaluated_count,
            self.banknifty_decision_count,
            self.authorized_count,
            self.policy_refused_count,
        ] {
            writer.u64(value)?;
        }
        writer.u32(self.rung)?;
        writer.u32(self.horizon_bars)?;
        writer.u8(self.nifty_terminal as u8)?;
        writer.u8(self.banknifty_terminal as u8)?;
        writer.zeros(2)?;
        for value in self.admission_counts {
            writer.u64(value)?;
        }
        for value in self.terminal_matrix {
            writer.u64(value)?;
        }
        writer.zeros(writer.remaining())?;
        writer.require_full("Completion payload")?;
        seal.copy_from_slice(&hash_parts(COMPLETION_SEAL_DOMAIN, &[payload]));
        Ok(raw)
    }

    fn decode(raw: &[u8; EXECUTION_V4_COMPLETION_BYTES]) -> Result<Self, ExecutionV4Refusal> {
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
            first_parameter_record: reader.u64()?,
            parameter_count: reader.u64()?,
            first_percentile_record: reader.u64()?,
            percentile_count: reader.u64()?,
            first_disposition_record: reader.u64()?,
            disposition_count: reader.u64()?,
            completion_id: reader.array()?,
            population_id: reader.array()?,
            population_completion_id: reader.array()?,
            population_ordered_digest: reader.array()?,
            nifty_population_family_row_id: reader.array()?,
            banknifty_population_family_row_id: reader.array()?,
            nifty_finalization_family_row_id: reader.array()?,
            banknifty_finalization_family_row_id: reader.array()?,
            source_finalization_id: reader.array()?,
            source_finalization_completion_id: reader.array()?,
            source_admission_block_id: reader.array()?,
            source_admission_completion_id: reader.array()?,
            execution_law_digest: reader.array()?,
            ordered_parameter_digest: reader.array()?,
            ordered_percentile_digest: reader.array()?,
            ordered_disposition_digest: reader.array()?,
            nifty_disposition_digest: reader.array()?,
            banknifty_disposition_digest: reader.array()?,
            nifty_authority_id: reader.array()?,
            banknifty_authority_id: reader.array()?,
            parameter_ids: [
                reader.array()?,
                reader.array()?,
                reader.array()?,
                reader.array()?,
            ],
            row_count: reader.u64()?,
            evaluated_count: reader.u64()?,
            decision_count: reader.u64()?,
            nifty_count: reader.u64()?,
            nifty_evaluated_count: reader.u64()?,
            nifty_decision_count: reader.u64()?,
            banknifty_count: reader.u64()?,
            banknifty_evaluated_count: reader.u64()?,
            banknifty_decision_count: reader.u64()?,
            authorized_count: reader.u64()?,
            policy_refused_count: reader.u64()?,
            rung: reader.u32()?,
            horizon_bars: reader.u32()?,
            nifty_terminal: decode_family_terminal(reader.u8()?)?,
            banknifty_terminal: decode_family_terminal(reader.u8()?)?,
            admission_counts: {
                reader.require_zeros(2, "Completion terminal reserve")?;
                [reader.u64()?, reader.u64()?, reader.u64()?, reader.u64()?]
            },
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
            return Err("Execution V4 Completion is not byte-canonical".to_owned());
        }
        Ok(completion)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ExecutionV4FamilyEnvelope {
    family: ExecutionV4Family,
    terminal: ExecutionV4FamilyTerminal,
    population_family_row_id: [u8; 32],
    finalization_family_row_id: [u8; 32],
    candidate_count: u64,
    evaluated_count: u64,
    decision_count: u64,
}

impl ExecutionV4FamilyEnvelope {
    fn from_population(
        value: PopulationV6FamilyProjectionV1,
        expected_family: ExecutionV4Family,
    ) -> Result<Self, ExecutionV4Refusal> {
        let family = execution_family_from_admission(value.family());
        let terminal = execution_family_terminal(value.terminal())?;
        let result = Self {
            family,
            terminal,
            population_family_row_id: value.row_id(),
            finalization_family_row_id: value.finalization_family_row_id(),
            candidate_count: value.candidate_count(),
            evaluated_count: value.evaluated_count(),
            decision_count: value.decision_count(),
        };
        if result.family != expected_family {
            return Err("Execution V4 Family envelopes are not NIFTY then BANKNIFTY".to_owned());
        }
        require_nonzero("Population V6 Family row", result.population_family_row_id)?;
        require_nonzero(
            "Finalization V4 Family row",
            result.finalization_family_row_id,
        )?;
        validate_terminal_envelope(
            result.terminal,
            result.candidate_count,
            result.evaluated_count,
            result.decision_count,
            result.candidate_count,
        )?;
        Ok(result)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedExecutionV4 {
    population_id: [u8; 32],
    population_completion_id: [u8; 32],
    population_ordered_digest: [u8; 32],
    source_finalization_id: [u8; 32],
    source_finalization_completion_id: [u8; 32],
    source_admission_block_id: [u8; 32],
    source_admission_completion_id: [u8; 32],
    families: [ExecutionV4FamilyEnvelope; 2],
    rung: u32,
    horizon_bars: u32,
    parameters: Vec<ExecutionV4ParameterRecord>,
    percentiles: Vec<ExecutionV4PercentileRecord>,
    dispositions: Vec<ExecutionV4DispositionRecord>,
}

impl PreparedExecutionV4 {
    /// Private preparation path consuming the sole opaque Population V6
    /// execution source by value. No row, digest, grid, mask, disposition or
    /// terminal-envelope overload exists.
    fn from_population_source(
        source: PopulationV6ExecutionV4SourceV1,
        bounds: ExecutionV4Bounds,
    ) -> Result<Self, ExecutionV4Refusal> {
        Self::project_population_source(source, bounds, false).map(|value| value.0)
    }

    fn from_population_source_with_rows(
        source: PopulationV6ExecutionV4SourceV1,
        bounds: ExecutionV4Bounds,
    ) -> Result<
        (
            Self,
            PopulationV6StructuralReceipt,
            [PopulationV6FamilyProjectionV1; 2],
            Vec<PopulationV6CandidateProjectionV1>,
        ),
        ExecutionV4Refusal,
    > {
        Self::project_population_source(source, bounds, true)
    }

    fn project_population_source(
        source: PopulationV6ExecutionV4SourceV1,
        bounds: ExecutionV4Bounds,
        retain_population_rows: bool,
    ) -> Result<
        (
            Self,
            PopulationV6StructuralReceipt,
            [PopulationV6FamilyProjectionV1; 2],
            Vec<PopulationV6CandidateProjectionV1>,
        ),
        ExecutionV4Refusal,
    > {
        let (receipt, source, upstream_families, upstream_parameters, upstream_rows) =
            source.into_parts();
        if receipt.population_id() != source.population_id() {
            return Err("Execution V4 Population V6 source and receipt differ".to_owned());
        }
        let selection_families = upstream_families;
        let [nifty_family, banknifty_family] = upstream_families;
        let families = [
            ExecutionV4FamilyEnvelope::from_population(nifty_family, ExecutionV4Family::Nifty)?,
            ExecutionV4FamilyEnvelope::from_population(
                banknifty_family,
                ExecutionV4Family::BankNifty,
            )?,
        ];
        let family_candidate_count = families[0]
            .candidate_count
            .checked_add(families[1].candidate_count)
            .ok_or_else(|| "Execution V4 family Candidate count overflowed".to_owned())?;
        let family_evaluated_count = families[0]
            .evaluated_count
            .checked_add(families[1].evaluated_count)
            .ok_or_else(|| "Execution V4 family evaluated count overflowed".to_owned())?;
        let family_decision_count = families[0]
            .decision_count
            .checked_add(families[1].decision_count)
            .ok_or_else(|| "Execution V4 family decision count overflowed".to_owned())?;
        if family_candidate_count != receipt.candidate_count()
            || family_evaluated_count != receipt.evaluated_count()
            || family_decision_count != receipt.decision_count()
        {
            return Err("Execution V4 Population V6 Family envelopes do not reconcile".to_owned());
        }
        let mut percentiles = Vec::new();
        let mut parameters = Vec::new();
        parameters
            .try_reserve_exact(upstream_parameters.len())
            .map_err(|why| format!("cannot reserve Execution V4 parameters: {why}"))?;
        for facts in &upstream_parameters {
            let (parameter, mut segment) = parameter_from_population_source(
                receipt,
                &source,
                facts,
                usize_to_u64(percentiles.len(), "percentile offset")?,
            )?;
            percentiles.append(&mut segment);
            parameters.push(parameter);
        }
        let mut dispositions = Vec::new();
        dispositions
            .try_reserve_exact(upstream_rows.len())
            .map_err(|why| format!("cannot reserve Execution V4 dispositions: {why}"))?;
        for row in &upstream_rows {
            dispositions.push(disposition_from_population_source(
                row,
                &parameters,
                &families,
                source.finalization_completion_id(),
            )?);
        }
        let prepared = Self {
            population_id: receipt.population_id(),
            population_completion_id: receipt.completion_id(),
            population_ordered_digest: receipt.ordered_candidate_digest(),
            source_finalization_id: source.finalization_id(),
            source_finalization_completion_id: source.finalization_completion_id(),
            source_admission_block_id: source.admission_block_id(),
            source_admission_completion_id: source.admission_completion_id(),
            families,
            rung: source.rung_seconds(),
            horizon_bars: source.horizon_bars(),
            parameters,
            percentiles,
            dispositions,
        };
        prepared.validate(bounds)?;
        let mut population_rows = Vec::new();
        if retain_population_rows {
            population_rows
                .try_reserve_exact(upstream_rows.len())
                .map_err(|why| format!("cannot reserve Execution V4 Selection rows: {why}"))?;
            for row in upstream_rows {
                population_rows.push(row.into_parts().0);
            }
        }
        Ok((prepared, receipt, selection_families, population_rows))
    }

    fn validate(&self, bounds: ExecutionV4Bounds) -> Result<(), ExecutionV4Refusal> {
        for (name, value) in [
            ("Population V6 identity", self.population_id),
            ("Population V6 Completion", self.population_completion_id),
            (
                "Population V6 ordered Candidate digest",
                self.population_ordered_digest,
            ),
            ("Finalization V4 identity", self.source_finalization_id),
            (
                "Finalization V4 Completion",
                self.source_finalization_completion_id,
            ),
            ("Admission V4 block", self.source_admission_block_id),
            (
                "Admission V4 Completion",
                self.source_admission_completion_id,
            ),
        ] {
            require_nonzero(name, value)?;
        }
        if self.families[0].family != ExecutionV4Family::Nifty
            || self.families[1].family != ExecutionV4Family::BankNifty
            || self.rung == 0
            || self.horizon_bars == 0
        {
            return Err(
                "Execution V4 Family envelopes are reordered or source timing is zero".to_owned(),
            );
        }
        let expected_parameter_count = expected_parameter_count(&self.families)?;
        if self.parameters.len() != expected_parameter_count
            || self.parameters.len() > MAX_PARAMETER_RECORDS_PER_BLOCK
            || self.parameters.is_empty() != self.percentiles.is_empty()
        {
            return Err(
                "Execution V4 parameter count is not two per evaluated Family with matching percentile evidence"
                    .to_owned(),
            );
        }
        require_block_counts(bounds, self.percentiles.len(), self.dispositions.len())?;
        let expected_order = [
            (ExecutionV4Family::Nifty, ExecutionV4Direction::Long),
            (ExecutionV4Family::Nifty, ExecutionV4Direction::Short),
            (ExecutionV4Family::BankNifty, ExecutionV4Direction::Long),
            (ExecutionV4Family::BankNifty, ExecutionV4Direction::Short),
        ];
        let mut parameter_ids = HashSet::new();
        parameter_ids
            .try_reserve(expected_parameter_count)
            .map_err(|why| format!("cannot reserve Execution V4 parameter identities: {why}"))?;
        let mut expected_percentile_offset = 0_u64;
        let mut actual_parameter_index = 0_usize;
        for (family, direction) in expected_order {
            if self.families[family_index(family)].terminal
                == ExecutionV4FamilyTerminal::NaturallyExtinct
            {
                continue;
            }
            let parameter = self
                .parameters
                .get(actual_parameter_index)
                .ok_or_else(|| "Execution V4 evaluated Family parameter is absent".to_owned())?;
            parameter.validate()?;
            if parameter.population_id != self.population_id
                || parameter.population_completion_id != self.population_completion_id
                || parameter.population_ordered_digest != self.population_ordered_digest
                || parameter.source_finalization_id != self.source_finalization_id
                || parameter.source_finalization_completion_id
                    != self.source_finalization_completion_id
                || parameter.source_admission_block_id != self.source_admission_block_id
                || parameter.source_admission_completion_id != self.source_admission_completion_id
                || parameter.execution_law_digest != execution_law_digest()
                || parameter.family != family
                || parameter.direction != direction
                || parameter.rung != self.rung
                || parameter.horizon_bars != self.horizon_bars
                || parameter.percentile_offset != expected_percentile_offset
                || !parameter_ids.insert(parameter.parameter_id)
            {
                return Err(
                    "Execution V4 parameters violate source/law/order/common-rung/unique identity"
                        .to_owned(),
                );
            }
            let end = parameter
                .percentile_offset
                .checked_add(parameter.percentile_count)
                .ok_or_else(|| "Execution V4 percentile range overflowed".to_owned())?;
            let start = usize::try_from(parameter.percentile_offset)
                .map_err(|_| "Execution V4 percentile start does not fit usize".to_owned())?;
            let end_usize = usize::try_from(end)
                .map_err(|_| "Execution V4 percentile end does not fit usize".to_owned())?;
            let segment = self.percentiles.get(start..end_usize).ok_or_else(|| {
                "Execution V4 parameter percentile segment exceeds prepared atoms".to_owned()
            })?;
            validate_percentile_segment(parameter, segment)?;
            expected_percentile_offset = end;
            actual_parameter_index = actual_parameter_index
                .checked_add(1)
                .ok_or_else(|| "Execution V4 parameter ordinal overflowed".to_owned())?;
        }
        if actual_parameter_index != self.parameters.len()
            || expected_percentile_offset
                != u64::try_from(self.percentiles.len())
                    .map_err(|_| "Execution V4 percentile count does not fit u64".to_owned())?
        {
            return Err("Execution V4 percentile segments do not cover every atom".to_owned());
        }
        validate_disposition_block(self)?;
        Ok(())
    }

    fn expected_completion(
        &self,
        block_sequence: u64,
        first_parameter_record: u64,
        first_percentile_record: u64,
        first_disposition_record: u64,
    ) -> Result<ExecutionV4CompletionRecord, ExecutionV4Refusal> {
        let parameter_count = usize_to_u64(self.parameters.len(), "parameter count")?;
        let percentile_count = u64::try_from(self.percentiles.len())
            .map_err(|_| "Execution V4 percentile count does not fit u64".to_owned())?;
        let disposition_count = u64::try_from(self.dispositions.len())
            .map_err(|_| "Execution V4 disposition count does not fit u64".to_owned())?;
        let mut nifty_disposition_count = 0_u64;
        let mut banknifty_disposition_count = 0_u64;
        let mut authorized_count = 0_u64;
        let mut policy_refused_count = 0_u64;
        let mut admission_counts = [0_u64; 4];
        let mut terminal_matrix = [0_u64; MATRIX_CELLS];
        for row in &self.dispositions {
            match row.family {
                ExecutionV4Family::Nifty => {
                    increment(&mut nifty_disposition_count, "NIFTY disposition count")?;
                }
                ExecutionV4Family::BankNifty => {
                    increment(
                        &mut banknifty_disposition_count,
                        "BANKNIFTY disposition count",
                    )?;
                }
            }
            match row.terminal {
                ExecutionV4Terminal::Authorized => {
                    increment(&mut authorized_count, "Authorized count")?;
                }
                ExecutionV4Terminal::PolicyRefused => {
                    increment(&mut policy_refused_count, "PolicyRefused count")?;
                }
            }
            let admission_slot = admission_counts
                .get_mut(admission_index(row.admission_status))
                .ok_or_else(|| "Execution V4 admission status index escaped schema".to_owned())?;
            increment(admission_slot, "admission count")?;
            let matrix_slot = terminal_matrix
                .get_mut(matrix_index(row.family, row.admission_status, row.terminal))
                .ok_or_else(|| "Execution V4 terminal matrix index escaped schema".to_owned())?;
            increment(matrix_slot, "terminal matrix cell")?;
        }
        let ordered_parameter_digest = ordered_parameter_digest(&self.parameters)?;
        let ordered_percentile_digest = ordered_percentile_digest(&self.percentiles)?;
        let ordered_disposition_digest = ordered_disposition_digest(&self.dispositions)?;
        let nifty_disposition_digest =
            family_disposition_digest(ExecutionV4Family::Nifty, &self.dispositions)?;
        let banknifty_disposition_digest =
            family_disposition_digest(ExecutionV4Family::BankNifty, &self.dispositions)?;
        let parameter_ids = parameter_id_slots(&self.parameters, &self.families)?;
        let [nifty_long, nifty_short, banknifty_long, banknifty_short] = parameter_ids;
        let (nifty_matrix, banknifty_matrix) = terminal_matrix.split_at(8);
        let [nifty_family, banknifty_family] = self.families;
        if nifty_disposition_count != nifty_family.candidate_count
            || banknifty_disposition_count != banknifty_family.candidate_count
        {
            return Err(
                "Execution V4 dispositions do not reproduce Population V6 Family counts".to_owned(),
            );
        }
        let nifty_authority_id = derive_family_authority_id(
            nifty_family,
            self,
            [nifty_long, nifty_short],
            nifty_disposition_digest,
            nifty_matrix,
        );
        let banknifty_authority_id = derive_family_authority_id(
            banknifty_family,
            self,
            [banknifty_long, banknifty_short],
            banknifty_disposition_digest,
            banknifty_matrix,
        );
        let mut completion = ExecutionV4CompletionRecord {
            block_sequence,
            first_parameter_record,
            parameter_count,
            first_percentile_record,
            percentile_count,
            first_disposition_record,
            disposition_count,
            completion_id: [0; 32],
            population_id: self.population_id,
            population_completion_id: self.population_completion_id,
            population_ordered_digest: self.population_ordered_digest,
            nifty_population_family_row_id: nifty_family.population_family_row_id,
            banknifty_population_family_row_id: banknifty_family.population_family_row_id,
            nifty_finalization_family_row_id: nifty_family.finalization_family_row_id,
            banknifty_finalization_family_row_id: banknifty_family.finalization_family_row_id,
            source_finalization_id: self.source_finalization_id,
            source_finalization_completion_id: self.source_finalization_completion_id,
            source_admission_block_id: self.source_admission_block_id,
            source_admission_completion_id: self.source_admission_completion_id,
            execution_law_digest: execution_law_digest(),
            ordered_parameter_digest,
            ordered_percentile_digest,
            ordered_disposition_digest,
            nifty_disposition_digest,
            banknifty_disposition_digest,
            nifty_authority_id,
            banknifty_authority_id,
            parameter_ids,
            row_count: disposition_count,
            evaluated_count: disposition_count,
            decision_count: disposition_count,
            nifty_count: nifty_family.candidate_count,
            nifty_evaluated_count: nifty_family.evaluated_count,
            nifty_decision_count: nifty_family.decision_count,
            banknifty_count: banknifty_family.candidate_count,
            banknifty_evaluated_count: banknifty_family.evaluated_count,
            banknifty_decision_count: banknifty_family.decision_count,
            authorized_count,
            policy_refused_count,
            rung: self.rung,
            horizon_bars: self.horizon_bars,
            nifty_terminal: nifty_family.terminal,
            banknifty_terminal: banknifty_family.terminal,
            admission_counts,
            terminal_matrix,
        };
        completion.completion_id = completion.derive_completion_id();
        completion.validate()?;
        Ok(completion)
    }
}

fn parameter_from_population_source(
    receipt: PopulationV6StructuralReceipt,
    source: &PopulationV6SourceProjectionV1,
    facts: &CandidateExecutionParameterFactsV1,
    percentile_offset: u64,
) -> Result<(ExecutionV4ParameterRecord, Vec<ExecutionV4PercentileRecord>), ExecutionV4Refusal> {
    let family = execution_family(facts.family);
    let direction = execution_direction(facts.direction);
    let range_policy_tag = match facts.range_resolution {
        RangeResolutionV1::PpmFloor => 1,
        RangeResolutionV1::PpmCeiling => 2,
    };
    let selector_policy_tag = match facts.selector {
        ExitGridSelectorV1::PessimisticTotal => 1,
        ExitGridSelectorV1::EdgeThenPessimistic => 2,
        ExitGridSelectorV1::GuaranteedFloor => 3,
    };
    let forced_stop_policy_tag = match facts.forced_stop {
        ForcedStopV1::Disabled => FORCED_STOP_DISABLED_TAG,
        ForcedStopV1::IncludeExactObserved(_) => FORCED_STOP_INCLUDE_TAG,
        ForcedStopV1::RequireExactObserved(_) => FORCED_STOP_REQUIRE_TAG,
    };
    let percentile_count = facts
        .stop_percentiles
        .len()
        .checked_add(facts.target_percentiles.len())
        .and_then(|count| count.checked_add(facts.trail_percentiles.len()))
        .ok_or_else(|| "Execution V4 percentile count overflowed usize".to_owned())?;
    let percentile_count = usize_to_u64(percentile_count, "parameter percentile count")?;
    let mut parameter = ExecutionV4ParameterRecord {
        parameter_core_id: [0; 32],
        parameter_id: [0; 32],
        population_id: receipt.population_id(),
        population_completion_id: receipt.completion_id(),
        population_ordered_digest: receipt.ordered_candidate_digest(),
        source_finalization_id: source.finalization_id(),
        source_finalization_completion_id: source.finalization_completion_id(),
        source_admission_block_id: source.admission_block_id(),
        source_admission_completion_id: source.admission_completion_id(),
        policy_digest: facts.policy_digest,
        resolution_digest: facts.resolution_digest,
        training_digest: facts.training_digest,
        instrument_digest: facts.instrument_digest,
        feed_digest: facts.feed_digest,
        commit_digest: facts.commit_digest,
        calendar_digest: facts.calendar_digest,
        percentile_digest: [0; 32],
        cost_model_id: facts.cost_model_id,
        execution_law_digest: execution_law_digest(),
        evaluation_fingerprint: facts.evaluation_fingerprint,
        family,
        direction,
        range_policy_tag,
        selector_policy_tag,
        forced_stop_policy_tag,
        rung: facts.rung_seconds,
        horizon_bars: facts.horizon_bars,
        execution_resolution_seconds: EXECUTION_RESOLUTION_SECONDS,
        entry_delay_minutes: ENTRY_DELAY_MINUTES,
        forced_exit_ist_minute: FORCED_EXIT_IST_MINUTE,
        forced_stop_index: facts.forced_stop_index,
        run_params: facts.run_params,
        max_levels: facts.max_levels,
        ratio_min_hundredths: facts.ratio_min_hundredths,
        ratio_max_hundredths: facts.ratio_max_hundredths,
        max_ratio_pairs: facts.max_ratio_pairs,
        policy_max_cells: facts.policy_max_cells,
        resolved_stop_count: facts.resolved_stop_count,
        resolved_target_count: facts.resolved_target_count,
        resolved_trail_count: facts.resolved_trail_count,
        resolved_ratio_pair_count: facts.resolved_ratio_pair_count,
        resolved_cell_count: facts.resolved_cell_count,
        forced_stop_ppm: facts.forced_stop_ppm,
        max_ambiguity_bars: facts.max_ambiguity_bars,
        max_gap_bars: facts.max_gap_bars,
        training_bars: facts.training_bars,
        training_first_ts_micros: facts.training_first_ts_micros,
        training_last_ts_micros: facts.training_last_ts_micros,
        percentile_offset,
        percentile_count,
    };
    parameter.parameter_core_id = parameter.derive_core_id();
    let mut percentiles = Vec::new();
    percentiles
        .try_reserve_exact(
            usize::try_from(percentile_count)
                .map_err(|_| "Execution V4 percentile count does not fit usize".to_owned())?,
        )
        .map_err(|why| format!("cannot reserve Execution V4 percentile atoms: {why}"))?;
    append_percentiles(
        &mut percentiles,
        parameter.parameter_core_id,
        ExecutionV4PercentileAxis::Stop,
        &facts.stop_percentiles,
    )?;
    append_percentiles(
        &mut percentiles,
        parameter.parameter_core_id,
        ExecutionV4PercentileAxis::Target,
        &facts.target_percentiles,
    )?;
    append_percentiles(
        &mut percentiles,
        parameter.parameter_core_id,
        ExecutionV4PercentileAxis::Trail,
        &facts.trail_percentiles,
    )?;
    parameter.percentile_digest = ordered_percentile_digest(&percentiles)?;
    parameter.parameter_id = parameter.derive_parameter_id();
    parameter.validate()?;
    Ok((parameter, percentiles))
}

fn append_percentiles(
    output: &mut Vec<ExecutionV4PercentileRecord>,
    parameter_core_id: [u8; 32],
    axis: ExecutionV4PercentileAxis,
    values: &[RationalPercentileV1],
) -> Result<(), ExecutionV4Refusal> {
    for (ordinal, value) in values.iter().enumerate() {
        let record = ExecutionV4PercentileRecord {
            parameter_core_id,
            axis,
            ordinal: u32::try_from(ordinal)
                .map_err(|_| "Execution V4 percentile ordinal does not fit u32".to_owned())?,
            numerator: value.numerator(),
            denominator: value.denominator(),
        };
        record.validate()?;
        output.push(record);
    }
    Ok(())
}

fn disposition_from_population_source(
    source: &PopulationV6ExecutionDispositionSourceV1,
    parameters: &[ExecutionV4ParameterRecord],
    families: &[ExecutionV4FamilyEnvelope; 2],
    finalization_completion_id: [u8; 32],
) -> Result<ExecutionV4DispositionRecord, ExecutionV4Refusal> {
    let population = source.population();
    let candidate = population.candidate().row();
    let runner = source.disposition();
    let family = execution_family_from_admission(population.family());
    let family_envelope = families
        .get(family_index(family))
        .ok_or_else(|| "Execution V4 Population Family envelope is absent".to_owned())?;
    if family_envelope.family != family
        || family_envelope.terminal != ExecutionV4FamilyTerminal::Evaluated
        || population.finalization_family_row_id() != family_envelope.finalization_family_row_id
        || population.candidate_semantic_id() != candidate.candidate_semantic_digest()
        || population.candidate_row_digest() != population.candidate().base_candidate_row_digest()
    {
        return Err(
            "Execution V4 disposition source differs from its evaluated Population V6 Family"
                .to_owned(),
        );
    }
    let direction = execution_direction(candidate.direction());
    let parameter = find_parameter(parameters, family, direction)?;
    let coordinate = runner.coordinate();
    let (ttp_arm_index, ttp_trail_index) = match coordinate.ttp {
        Some(ttp) => (
            Some(usize_to_u32(ttp.arm, "TTP arm")?),
            Some(usize_to_u32(ttp.trail, "TTP trail")?),
        ),
        None => (None, None),
    };
    let terminal = if runner.is_authorized() {
        ExecutionV4Terminal::Authorized
    } else {
        ExecutionV4Terminal::PolicyRefused
    };
    let selected_exit_digest = runner.selected().map_or([0; 32], |value| value.digest());
    let mut row = ExecutionV4DispositionRecord {
        disposition_id: [0; 32],
        population_id: population.population_id(),
        population_completion_id: population.completion_id(),
        population_family_row_id: family_envelope.population_family_row_id,
        population_row_id: population.row_id(),
        candidate_semantic_id: candidate.candidate_semantic_digest(),
        candidate_base_row_id: population.candidate_row_digest(),
        base_evidence_id: population.base_evidence_id(),
        admission_decision_id: population.admission_decision_id(),
        finalization_family_row_id: population.finalization_family_row_id(),
        finalization_row_id: population.finalization_row_id(),
        finalization_completion_id,
        parameter_id: parameter.parameter_id,
        execution_run_id: candidate.execution_run_id(),
        evaluated_grid_digest: candidate.evaluated_grid_digest(),
        resolution_digest: runner.resolution_digest(),
        column_digest: runner.column_digest(),
        context_digest: runner.context_digest(),
        runner_disposition_digest: runner.disposition_digest(),
        selected_exit_digest,
        global_sequence: population.global_sequence(),
        family_sequence: population.family_sequence(),
        cell_ordinal: candidate.cell_ordinal(),
        support_hits: candidate.support_hits(),
        refusal_bits: runner.refusal_bits().bits(),
        family,
        direction,
        admission_status: execution_admission_status(population.status()),
        terminal,
        rung: candidate.rung_seconds(),
        horizon_bars: candidate.horizon_bars(),
        stop_index: optional_usize_to_u32(coordinate.stop, "stop")?,
        target_index: optional_usize_to_u32(coordinate.target, "target")?,
        tsl_index: optional_usize_to_u32(coordinate.tsl, "TSL")?,
        ttp_arm_index,
        ttp_trail_index,
    };
    row.disposition_id = row.derive_id();
    row.validate()?;
    Ok(row)
}

const fn execution_family(value: InstrumentFamilyV1) -> ExecutionV4Family {
    match value {
        InstrumentFamilyV1::Nifty => ExecutionV4Family::Nifty,
        InstrumentFamilyV1::BankNifty => ExecutionV4Family::BankNifty,
    }
}

const fn execution_family_from_admission(value: AdmissionV4Family) -> ExecutionV4Family {
    match value {
        AdmissionV4Family::Nifty => ExecutionV4Family::Nifty,
        AdmissionV4Family::BankNifty => ExecutionV4Family::BankNifty,
    }
}

fn execution_family_terminal(
    value: AdmissionV4FamilyTerminal,
) -> Result<ExecutionV4FamilyTerminal, ExecutionV4Refusal> {
    match value {
        AdmissionV4FamilyTerminal::Evaluated => Ok(ExecutionV4FamilyTerminal::Evaluated),
        AdmissionV4FamilyTerminal::NaturallyExtinct => {
            Ok(ExecutionV4FamilyTerminal::NaturallyExtinct)
        }
        AdmissionV4FamilyTerminal::InsufficientForCscv => Err(
            "Execution V4 refuses production-unreachable Candidate V1 singleton terminal"
                .to_owned(),
        ),
    }
}

const fn execution_direction(value: TradeDirectionV1) -> ExecutionV4Direction {
    match value {
        TradeDirectionV1::Long => ExecutionV4Direction::Long,
        TradeDirectionV1::Short => ExecutionV4Direction::Short,
    }
}

const fn execution_admission_status(
    value: AdmissionV4DecisionStatus,
) -> ExecutionV4AdmissionStatus {
    match value {
        AdmissionV4DecisionStatus::Admitted => ExecutionV4AdmissionStatus::Admitted,
        AdmissionV4DecisionStatus::Rejected => ExecutionV4AdmissionStatus::Rejected,
        AdmissionV4DecisionStatus::Unmeasured => ExecutionV4AdmissionStatus::Unmeasured,
        AdmissionV4DecisionStatus::Refused => ExecutionV4AdmissionStatus::Refused,
    }
}

fn optional_usize_to_u32(
    value: Option<usize>,
    name: &str,
) -> Result<Option<u32>, ExecutionV4Refusal> {
    value.map(|index| usize_to_u32(index, name)).transpose()
}

fn usize_to_u32(value: usize, name: &str) -> Result<u32, ExecutionV4Refusal> {
    u32::try_from(value).map_err(|_| format!("Execution V4 {name} index does not fit u32"))
}

/// Structural receipt from a bounded fresh reopen. It is not authority by
/// itself and has no caller-controlled constructor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExecutionV4StructuralReceipt {
    block_sequence: u64,
    first_parameter_record: u64,
    first_percentile_record: u64,
    first_disposition_record: u64,
    population_id: [u8; 32],
    population_completion_id: [u8; 32],
    population_ordered_digest: [u8; 32],
    ordered_disposition_digest: [u8; 32],
    parameter_count: u64,
    percentile_count: u64,
    disposition_count: u64,
    completion_id: [u8; 32],
    nifty_authority_id: [u8; 32],
    banknifty_authority_id: [u8; 32],
    row_count: u64,
    nifty_count: u64,
    banknifty_count: u64,
    authorized_count: u64,
    policy_refused_count: u64,
    rung: u32,
    horizon_bars: u32,
    nifty_terminal: ExecutionV4FamilyTerminal,
    banknifty_terminal: ExecutionV4FamilyTerminal,
    admission_counts: [u64; 4],
    terminal_matrix: [u64; MATRIX_CELLS],
}

impl ExecutionV4StructuralReceipt {
    fn from_completion(value: &ExecutionV4CompletionRecord) -> Self {
        Self {
            block_sequence: value.block_sequence,
            first_parameter_record: value.first_parameter_record,
            first_percentile_record: value.first_percentile_record,
            first_disposition_record: value.first_disposition_record,
            population_id: value.population_id,
            population_completion_id: value.population_completion_id,
            population_ordered_digest: value.population_ordered_digest,
            ordered_disposition_digest: value.ordered_disposition_digest,
            parameter_count: value.parameter_count,
            percentile_count: value.percentile_count,
            disposition_count: value.disposition_count,
            completion_id: value.completion_id,
            nifty_authority_id: value.nifty_authority_id,
            banknifty_authority_id: value.banknifty_authority_id,
            row_count: value.row_count,
            nifty_count: value.nifty_count,
            banknifty_count: value.banknifty_count,
            authorized_count: value.authorized_count,
            policy_refused_count: value.policy_refused_count,
            rung: value.rung,
            horizon_bars: value.horizon_bars,
            nifty_terminal: value.nifty_terminal,
            banknifty_terminal: value.banknifty_terminal,
            admission_counts: value.admission_counts,
            terminal_matrix: value.terminal_matrix,
        }
    }

    #[must_use]
    pub(crate) const fn population_id(self) -> [u8; 32] {
        self.population_id
    }

    #[must_use]
    pub(crate) const fn population_completion_id(self) -> [u8; 32] {
        self.population_completion_id
    }

    #[must_use]
    pub(crate) const fn population_ordered_digest(self) -> [u8; 32] {
        self.population_ordered_digest
    }

    #[must_use]
    pub(crate) const fn ordered_disposition_digest(self) -> [u8; 32] {
        self.ordered_disposition_digest
    }

    #[must_use]
    pub(crate) const fn parameter_count(self) -> u64 {
        self.parameter_count
    }

    #[must_use]
    pub(crate) const fn percentile_count(self) -> u64 {
        self.percentile_count
    }

    #[must_use]
    pub(crate) const fn disposition_count(self) -> u64 {
        self.disposition_count
    }

    #[must_use]
    pub(crate) const fn completion_id(self) -> [u8; 32] {
        self.completion_id
    }

    #[must_use]
    pub(crate) const fn nifty_authority_id(self) -> [u8; 32] {
        self.nifty_authority_id
    }

    #[must_use]
    pub(crate) const fn banknifty_authority_id(self) -> [u8; 32] {
        self.banknifty_authority_id
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
    pub(crate) const fn authorized_count(self) -> u64 {
        self.authorized_count
    }

    #[must_use]
    pub(crate) const fn policy_refused_count(self) -> u64 {
        self.policy_refused_count
    }

    #[must_use]
    pub(crate) const fn rung(self) -> u32 {
        self.rung
    }

    #[must_use]
    pub(crate) const fn horizon_bars(self) -> u32 {
        self.horizon_bars
    }

    #[must_use]
    pub(crate) const fn nifty_terminal(self) -> ExecutionV4FamilyTerminal {
        self.nifty_terminal
    }

    #[must_use]
    pub(crate) const fn banknifty_terminal(self) -> ExecutionV4FamilyTerminal {
        self.banknifty_terminal
    }

    #[must_use]
    pub(crate) const fn admission_counts(self) -> [u64; 4] {
        self.admission_counts
    }

    #[must_use]
    pub(crate) const fn terminal_matrix(self) -> [u64; MATRIX_CELLS] {
        self.terminal_matrix
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

struct HeldFile {
    path: PathBuf,
    file: File,
    generation: FileGeneration,
    bound: ExecutionV4FileBound,
    stride: usize,
    name: &'static str,
}

impl HeldFile {
    fn record_count(&self) -> Result<u64, ExecutionV4Refusal> {
        checked_record_count(
            self.generation.len,
            self.stride,
            self.bound.records,
            self.name,
        )
    }

    fn refresh(&mut self) -> Result<(), ExecutionV4Refusal> {
        self.generation = file_generation(&self.file, &self.path, self.bound.bytes)?;
        Ok(())
    }

    fn require_unchanged(&self) -> Result<(), ExecutionV4Refusal> {
        if file_generation(&self.file, &self.path, self.bound.bytes)? != self.generation {
            return Err(format!(
                "Execution V4 retained {} generation changed",
                self.name
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TrailingExecutionV4 {
    first_parameter_record: u64,
    first_percentile_record: u64,
    first_disposition_record: u64,
    parameters: Vec<ExecutionV4ParameterRecord>,
    percentiles: Vec<ExecutionV4PercentileRecord>,
    dispositions: Vec<ExecutionV4DispositionRecord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExecutionV4StructuralCommit {
    Written(ExecutionV4StructuralReceipt),
    Reused(ExecutionV4StructuralReceipt),
}

impl ExecutionV4StructuralCommit {
    const fn receipt(self) -> ExecutionV4StructuralReceipt {
        match self {
            Self::Written(receipt) | Self::Reused(receipt) => receipt,
        }
    }

    const fn was_written(self) -> bool {
        matches!(self, Self::Written(_))
    }
}

/// Retained, generation-checked Execution V4 fixed-file ledger.
struct ExecutionV4Ledger {
    root: PathBuf,
    root_file: File,
    root_identity: PlatformIdentity,
    lock_path: PathBuf,
    lock_file: File,
    lock_generation: FileGeneration,
    parameters: HeldFile,
    percentiles: HeldFile,
    dispositions: HeldFile,
    completions: HeldFile,
    bounds: ExecutionV4Bounds,
    writable: bool,
    receipts: HashMap<[u8; 32], ExecutionV4StructuralReceipt>,
    trailing: Option<TrailingExecutionV4>,
    parameter_records: u64,
    percentile_records: u64,
    disposition_records: u64,
    completion_records: u64,
}

impl ExecutionV4Ledger {
    fn open_read(root: &Path, bounds: ExecutionV4Bounds) -> Result<Self, ExecutionV4Refusal> {
        Self::open(root, bounds, false)
    }

    fn open_write(root: &Path, bounds: ExecutionV4Bounds) -> Result<Self, ExecutionV4Refusal> {
        Self::open(root, bounds, true)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "opening one retained ledger validates and captures all five file generations atomically"
    )]
    fn open(
        root: &Path,
        bounds: ExecutionV4Bounds,
        writable: bool,
    ) -> Result<Self, ExecutionV4Refusal> {
        let (root, root_file, root_identity) = open_root_directory(root)?;
        let lock_path = root.join(LOCK_FILE);
        let parameter_path = root.join(PARAMETER_FILE);
        let percentile_path = root.join(PERCENTILE_FILE);
        let disposition_path = root.join(DISPOSITION_FILE);
        let completion_path = root.join(COMPLETION_FILE);
        let (lock_file, lock_created) = open_child(&lock_path, writable, writable)?;
        if writable {
            lock_file
                .lock()
                .map_err(|why| format!("cannot lock Execution V4 writer: {why}"))?;
        } else {
            lock_file
                .lock_shared()
                .map_err(|why| format!("cannot lock Execution V4 reader: {why}"))?;
        }
        let opened = (|| {
            let (parameter_file, parameter_created) =
                open_child(&parameter_path, writable, writable)?;
            let (percentile_file, percentile_created) =
                open_child(&percentile_path, writable, writable)?;
            let (disposition_file, disposition_created) =
                open_child(&disposition_path, writable, writable)?;
            let (completion_file, completion_created) =
                open_child(&completion_path, writable, writable)?;
            if lock_created
                || parameter_created
                || percentile_created
                || disposition_created
                || completion_created
            {
                sync_directory(&root_file, &root)?;
            }
            if named_identity(&root)? != root_identity {
                return Err("Execution V4 root changed while child files opened".to_owned());
            }
            let lock_generation = file_generation(&lock_file, &lock_path, LOCK_MAX_BYTES)?;
            let parameter_generation =
                file_generation(&parameter_file, &parameter_path, bounds.parameters.bytes)?;
            let percentile_generation =
                file_generation(&percentile_file, &percentile_path, bounds.percentiles.bytes)?;
            let disposition_generation = file_generation(
                &disposition_file,
                &disposition_path,
                bounds.dispositions.bytes,
            )?;
            let completion_generation =
                file_generation(&completion_file, &completion_path, bounds.completions.bytes)?;
            let mut ledger = Self {
                root,
                root_file,
                root_identity,
                lock_path,
                lock_file: lock_file
                    .try_clone()
                    .map_err(|why| format!("cannot clone Execution V4 lock file: {why}"))?,
                lock_generation,
                parameters: HeldFile {
                    path: parameter_path,
                    generation: parameter_generation,
                    file: parameter_file,
                    bound: bounds.parameters,
                    stride: EXECUTION_V4_PARAMETER_BYTES,
                    name: "parameter",
                },
                percentiles: HeldFile {
                    path: percentile_path,
                    generation: percentile_generation,
                    file: percentile_file,
                    bound: bounds.percentiles,
                    stride: EXECUTION_V4_PERCENTILE_BYTES,
                    name: "percentile",
                },
                dispositions: HeldFile {
                    path: disposition_path,
                    generation: disposition_generation,
                    file: disposition_file,
                    bound: bounds.dispositions,
                    stride: EXECUTION_V4_DISPOSITION_BYTES,
                    name: "disposition",
                },
                completions: HeldFile {
                    path: completion_path,
                    generation: completion_generation,
                    file: completion_file,
                    bound: bounds.completions,
                    stride: EXECUTION_V4_COMPLETION_BYTES,
                    name: "Completion",
                },
                bounds,
                writable,
                receipts: HashMap::new(),
                trailing: None,
                parameter_records: 0,
                percentile_records: 0,
                disposition_records: 0,
                completion_records: 0,
            };
            ledger.scan()?;
            Ok(ledger)
        })();
        let released = lock_file
            .unlock()
            .map_err(|why| format!("cannot unlock Execution V4 open lock: {why}"));
        combine_lock_result(opened, released)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one receipt-last scan must reconcile every completed block and the sole trailing prefix before indexing"
    )]
    fn scan(&mut self) -> Result<(), ExecutionV4Refusal> {
        let parameter_records = self.parameters.record_count()?;
        let percentile_records = self.percentiles.record_count()?;
        let disposition_records = self.dispositions.record_count()?;
        let completion_records = self.completions.record_count()?;
        self.receipts.clear();
        self.receipts
            .try_reserve(
                usize::try_from(completion_records)
                    .map_err(|_| "Execution V4 Completion count does not fit usize".to_owned())?,
            )
            .map_err(|why| format!("cannot reserve Execution V4 receipt index: {why}"))?;
        self.trailing = None;
        let mut covered_parameters = 0_u64;
        let mut covered_percentiles = 0_u64;
        let mut covered_dispositions = 0_u64;
        for completion_index in 0..completion_records {
            let completion = ExecutionV4CompletionRecord::decode(&read_fixed_at(
                &mut self.completions.file,
                completion_index,
                EXECUTION_V4_COMPLETION_BYTES,
                "Completion",
            )?)?;
            if completion.block_sequence != completion_index
                || completion.first_parameter_record != covered_parameters
                || completion.first_percentile_record != covered_percentiles
                || completion.first_disposition_record != covered_dispositions
            {
                return Err(format!(
                    "Execution V4 Completion {completion_index} is not contiguous/canonical"
                ));
            }
            let parameter_end = checked_end(
                covered_parameters,
                completion.parameter_count,
                "completed parameter range",
            )?;
            let percentile_end = checked_end(
                covered_percentiles,
                completion.percentile_count,
                "completed percentile range",
            )?;
            let disposition_end = checked_end(
                covered_dispositions,
                completion.disposition_count,
                "completed disposition range",
            )?;
            if parameter_end > parameter_records
                || percentile_end > percentile_records
                || disposition_end > disposition_records
            {
                return Err(format!(
                    "Execution V4 Completion {completion_index} points beyond a fixed-record file"
                ));
            }
            let parameters = self.read_parameters(
                completion.first_parameter_record,
                completion.parameter_count,
            )?;
            let percentiles = self.read_percentiles(
                completion.first_percentile_record,
                completion.percentile_count,
            )?;
            let dispositions = self.read_dispositions(
                completion.first_disposition_record,
                completion.disposition_count,
            )?;
            let receipt = validate_complete_block(
                &parameters,
                &percentiles,
                &dispositions,
                &completion,
                self.bounds,
            )?;
            if self
                .receipts
                .insert(receipt.population_id, receipt)
                .is_some()
            {
                return Err(format!(
                    "Execution V4 Population identity {} appears more than once",
                    hex32(receipt.population_id)
                ));
            }
            covered_parameters = parameter_end;
            covered_percentiles = percentile_end;
            covered_dispositions = disposition_end;
        }
        let parameter_tail = parameter_records
            .checked_sub(covered_parameters)
            .ok_or_else(|| "Execution V4 parameter tail underflowed".to_owned())?;
        let percentile_tail = percentile_records
            .checked_sub(covered_percentiles)
            .ok_or_else(|| "Execution V4 percentile tail underflowed".to_owned())?;
        let disposition_tail = disposition_records
            .checked_sub(covered_dispositions)
            .ok_or_else(|| "Execution V4 disposition tail underflowed".to_owned())?;
        if parameter_tail != 0 || percentile_tail != 0 || disposition_tail != 0 {
            let parameters = self.read_parameters(covered_parameters, parameter_tail)?;
            let percentiles = self.read_percentiles(covered_percentiles, percentile_tail)?;
            let dispositions = self.read_dispositions(covered_dispositions, disposition_tail)?;
            validate_trailing_prefix(&parameters, &percentiles, &dispositions, self.bounds)?;
            if parameters
                .first()
                .is_some_and(|parameter| self.receipts.contains_key(&parameter.population_id))
            {
                return Err(
                    "Execution V4 trailing block duplicates a committed Population".to_owned(),
                );
            }
            self.trailing = Some(TrailingExecutionV4 {
                first_parameter_record: covered_parameters,
                first_percentile_record: covered_percentiles,
                first_disposition_record: covered_dispositions,
                parameters,
                percentiles,
                dispositions,
            });
        }
        self.parameter_records = parameter_records;
        self.percentile_records = percentile_records;
        self.disposition_records = disposition_records;
        self.completion_records = completion_records;
        self.require_unchanged()
    }

    fn append(
        &mut self,
        prepared: &PreparedExecutionV4,
    ) -> Result<ExecutionV4StructuralCommit, ExecutionV4Refusal> {
        if !self.writable {
            return Err("Execution V4 ledger was opened read-only".to_owned());
        }
        self.lock_file
            .lock()
            .map_err(|why| format!("cannot take Execution V4 append lock: {why}"))?;
        let result = self.append_locked(prepared);
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Execution V4 append lock: {why}"));
        combine_lock_result(result, released)
    }

    fn append_locked(
        &mut self,
        prepared: &PreparedExecutionV4,
    ) -> Result<ExecutionV4StructuralCommit, ExecutionV4Refusal> {
        self.require_unchanged()?;
        prepared.validate(self.bounds)?;
        if let Some(existing) = self.receipts.get(&prepared.population_id).copied() {
            return self.reuse_existing(prepared, existing);
        }
        let trailing = self.trailing.clone().unwrap_or(TrailingExecutionV4 {
            first_parameter_record: self.parameter_records,
            first_percentile_record: self.percentile_records,
            first_disposition_record: self.disposition_records,
            parameters: Vec::new(),
            percentiles: Vec::new(),
            dispositions: Vec::new(),
        });
        Self::require_exact_prefix(prepared, &trailing)?;
        self.require_append_bound(prepared, &trailing)?;

        self.append_parameter_suffix(prepared, trailing.parameters.len())?;
        self.parameters
            .file
            .sync_data()
            .map_err(|why| format!("cannot sync Execution V4 parameters: {why}"))?;
        self.parameters.refresh()?;
        self.parameter_records = self.parameters.record_count()?;
        self.require_unchanged()?;

        self.append_percentile_suffix(prepared, trailing.percentiles.len())?;
        self.percentiles
            .file
            .sync_data()
            .map_err(|why| format!("cannot sync Execution V4 percentiles: {why}"))?;
        self.percentiles.refresh()?;
        self.percentile_records = self.percentiles.record_count()?;
        self.require_unchanged()?;

        self.append_disposition_suffix(prepared, trailing.dispositions.len())?;
        self.dispositions
            .file
            .sync_data()
            .map_err(|why| format!("cannot sync Execution V4 dispositions: {why}"))?;
        self.dispositions.refresh()?;
        self.disposition_records = self.dispositions.record_count()?;
        self.require_unchanged()?;

        self.append_completion(
            prepared,
            trailing.first_parameter_record,
            trailing.first_percentile_record,
            trailing.first_disposition_record,
        )?;
        self.scan()?;
        let receipt = self
            .receipts
            .get(&prepared.population_id)
            .copied()
            .ok_or_else(|| "Execution V4 appended block was not indexed".to_owned())?;
        Ok(ExecutionV4StructuralCommit::Written(receipt))
    }

    fn reuse_existing(
        &mut self,
        prepared: &PreparedExecutionV4,
        existing: ExecutionV4StructuralReceipt,
    ) -> Result<ExecutionV4StructuralCommit, ExecutionV4Refusal> {
        let parameters =
            self.read_parameters(existing.first_parameter_record, existing.parameter_count)?;
        let percentiles =
            self.read_percentiles(existing.first_percentile_record, existing.percentile_count)?;
        let dispositions = self.read_dispositions(
            existing.first_disposition_record,
            existing.disposition_count,
        )?;
        if parameters.as_slice() != prepared.parameters.as_slice()
            || percentiles != prepared.percentiles
            || dispositions != prepared.dispositions
        {
            return Err(format!(
                "Execution V4 Population {} exists with different exact records",
                hex32(existing.population_id)
            ));
        }
        let observed = ExecutionV4CompletionRecord::decode(&read_fixed_at(
            &mut self.completions.file,
            existing.block_sequence,
            EXECUTION_V4_COMPLETION_BYTES,
            "Completion",
        )?)?;
        let expected = prepared.expected_completion(
            existing.block_sequence,
            existing.first_parameter_record,
            existing.first_percentile_record,
            existing.first_disposition_record,
        )?;
        if observed != expected {
            return Err(format!(
                "Execution V4 Population {} exists with a different Completion",
                hex32(existing.population_id)
            ));
        }
        for file in [
            &self.parameters.file,
            &self.percentiles.file,
            &self.dispositions.file,
            &self.completions.file,
        ] {
            file.sync_data()
                .map_err(|why| format!("cannot sync reused Execution V4 block: {why}"))?;
        }
        sync_directory(&self.root_file, &self.root)?;
        self.require_unchanged()?;
        Ok(ExecutionV4StructuralCommit::Reused(existing))
    }

    fn require_exact_prefix(
        prepared: &PreparedExecutionV4,
        trailing: &TrailingExecutionV4,
    ) -> Result<(), ExecutionV4Refusal> {
        if trailing.parameters.len() > prepared.parameters.len()
            || trailing.percentiles.len() > prepared.percentiles.len()
            || trailing.dispositions.len() > prepared.dispositions.len()
            || prepared.parameters.get(..trailing.parameters.len())
                != Some(trailing.parameters.as_slice())
            || prepared.percentiles.get(..trailing.percentiles.len())
                != Some(trailing.percentiles.as_slice())
            || prepared.dispositions.get(..trailing.dispositions.len())
                != Some(trailing.dispositions.as_slice())
        {
            return Err("Execution V4 orphan tail is not an exact retry prefix".to_owned());
        }
        if trailing.parameters.len() < prepared.parameters.len()
            && (!trailing.percentiles.is_empty() || !trailing.dispositions.is_empty())
        {
            return Err(
                "Execution V4 orphan persisted percentiles/dispositions before all parameters"
                    .to_owned(),
            );
        }
        if trailing.percentiles.len() < prepared.percentiles.len()
            && !trailing.dispositions.is_empty()
        {
            return Err(
                "Execution V4 orphan persisted dispositions before all percentiles".to_owned(),
            );
        }
        Ok(())
    }

    fn require_append_bound(
        &self,
        prepared: &PreparedExecutionV4,
        trailing: &TrailingExecutionV4,
    ) -> Result<(), ExecutionV4Refusal> {
        let target_parameters = checked_end(
            trailing.first_parameter_record,
            usize_to_u64(prepared.parameters.len(), "parameter count")?,
            "append parameter range",
        )?;
        let target_percentiles = checked_end(
            trailing.first_percentile_record,
            usize_to_u64(prepared.percentiles.len(), "percentile count")?,
            "append percentile range",
        )?;
        let target_dispositions = checked_end(
            trailing.first_disposition_record,
            usize_to_u64(prepared.dispositions.len(), "disposition count")?,
            "append disposition range",
        )?;
        let target_completions = self
            .completion_records
            .checked_add(1)
            .ok_or_else(|| "Execution V4 Completion count overflowed".to_owned())?;
        for (target, bound, name) in [
            (target_parameters, self.bounds.parameters, "parameter"),
            (target_percentiles, self.bounds.percentiles, "percentile"),
            (target_dispositions, self.bounds.dispositions, "disposition"),
            (target_completions, self.bounds.completions, "Completion"),
        ] {
            if target > bound.records {
                return Err(format!(
                    "Execution V4 append reaches {target} {name} records above bound {}",
                    bound.records
                ));
            }
        }
        Ok(())
    }

    fn append_parameter_suffix(
        &mut self,
        prepared: &PreparedExecutionV4,
        start: usize,
    ) -> Result<(), ExecutionV4Refusal> {
        for record in prepared.parameters.get(start..).ok_or_else(|| {
            format!("Execution V4 parameter suffix start {start} is outside the block")
        })? {
            append_raw(&mut self.parameters.file, &record.encode()?)?;
        }
        Ok(())
    }

    fn append_percentile_suffix(
        &mut self,
        prepared: &PreparedExecutionV4,
        start: usize,
    ) -> Result<(), ExecutionV4Refusal> {
        for record in prepared.percentiles.get(start..).ok_or_else(|| {
            format!("Execution V4 percentile suffix start {start} is outside the block")
        })? {
            append_raw(&mut self.percentiles.file, &record.encode()?)?;
        }
        Ok(())
    }

    fn append_disposition_suffix(
        &mut self,
        prepared: &PreparedExecutionV4,
        start: usize,
    ) -> Result<(), ExecutionV4Refusal> {
        for record in prepared.dispositions.get(start..).ok_or_else(|| {
            format!("Execution V4 disposition suffix start {start} is outside the block")
        })? {
            append_raw(&mut self.dispositions.file, &record.encode()?)?;
        }
        Ok(())
    }

    fn append_completion(
        &mut self,
        prepared: &PreparedExecutionV4,
        first_parameter_record: u64,
        first_percentile_record: u64,
        first_disposition_record: u64,
    ) -> Result<(), ExecutionV4Refusal> {
        let completion = prepared.expected_completion(
            self.completion_records,
            first_parameter_record,
            first_percentile_record,
            first_disposition_record,
        )?;
        append_raw(&mut self.completions.file, &completion.encode()?)?;
        self.completions
            .file
            .sync_data()
            .map_err(|why| format!("cannot sync Execution V4 Completion: {why}"))?;
        sync_directory(&self.root_file, &self.root)?;
        self.completions.refresh()?;
        self.completion_records = self.completions.record_count()?;
        self.require_unchanged()
    }

    fn read_parameters(
        &mut self,
        first: u64,
        count: u64,
    ) -> Result<Vec<ExecutionV4ParameterRecord>, ExecutionV4Refusal> {
        let mut values = bounded_vec(count, self.bounds.parameters.records, "parameters")?;
        for offset in 0..count {
            values.push(ExecutionV4ParameterRecord::decode(&read_fixed_at(
                &mut self.parameters.file,
                checked_end(first, offset, "parameter read offset")?,
                EXECUTION_V4_PARAMETER_BYTES,
                "parameter",
            )?)?);
        }
        Ok(values)
    }

    fn read_percentiles(
        &mut self,
        first: u64,
        count: u64,
    ) -> Result<Vec<ExecutionV4PercentileRecord>, ExecutionV4Refusal> {
        let mut values = bounded_vec(count, self.bounds.percentiles.records, "percentiles")?;
        for offset in 0..count {
            values.push(ExecutionV4PercentileRecord::decode(&read_fixed_at(
                &mut self.percentiles.file,
                checked_end(first, offset, "percentile read offset")?,
                EXECUTION_V4_PERCENTILE_BYTES,
                "percentile",
            )?)?);
        }
        Ok(values)
    }

    fn read_dispositions(
        &mut self,
        first: u64,
        count: u64,
    ) -> Result<Vec<ExecutionV4DispositionRecord>, ExecutionV4Refusal> {
        let mut values = bounded_vec(count, self.bounds.dispositions.records, "dispositions")?;
        for offset in 0..count {
            values.push(ExecutionV4DispositionRecord::decode(&read_fixed_at(
                &mut self.dispositions.file,
                checked_end(first, offset, "disposition read offset")?,
                EXECUTION_V4_DISPOSITION_BYTES,
                "disposition",
            )?)?);
        }
        Ok(values)
    }

    fn structural_receipt(
        &self,
        population_id: &[u8; 32],
    ) -> Result<Option<ExecutionV4StructuralReceipt>, ExecutionV4Refusal> {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take Execution V4 lookup lock: {why}"))?;
        let result = self
            .require_unchanged()
            .map(|()| self.receipts.get(population_id).copied());
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Execution V4 lookup lock: {why}"));
        combine_lock_result(result, released)
    }

    fn require_exact_prepared(
        &mut self,
        receipt: ExecutionV4StructuralReceipt,
        prepared: &PreparedExecutionV4,
    ) -> Result<(), ExecutionV4Refusal> {
        let parameters =
            self.read_parameters(receipt.first_parameter_record, receipt.parameter_count)?;
        let percentiles =
            self.read_percentiles(receipt.first_percentile_record, receipt.percentile_count)?;
        let dispositions =
            self.read_dispositions(receipt.first_disposition_record, receipt.disposition_count)?;
        let completion = ExecutionV4CompletionRecord::decode(&read_fixed_at(
            &mut self.completions.file,
            receipt.block_sequence,
            EXECUTION_V4_COMPLETION_BYTES,
            "Completion",
        )?)?;
        if parameters.as_slice() != prepared.parameters.as_slice()
            || percentiles != prepared.percentiles
            || dispositions != prepared.dispositions
            || completion
                != prepared.expected_completion(
                    receipt.block_sequence,
                    receipt.first_parameter_record,
                    receipt.first_percentile_record,
                    receipt.first_disposition_record,
                )?
        {
            return Err("Execution V4 fresh reopen differs from prepared bytes".to_owned());
        }
        self.require_unchanged()
    }

    fn authenticated_disposition(
        &mut self,
        receipt: ExecutionV4StructuralReceipt,
        global_sequence: u64,
    ) -> Result<ExecutionV4SuccessorDisposition, ExecutionV4Refusal> {
        if global_sequence >= receipt.disposition_count {
            return Err(format!(
                "Execution V4 disposition {global_sequence} is outside authenticated count {}",
                receipt.disposition_count
            ));
        }
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take Execution V4 disposition lock: {why}"))?;
        let result = (|| {
            self.require_unchanged()?;
            if self.receipts.get(&receipt.population_id) != Some(&receipt) {
                return Err("Execution V4 receipt is no longer indexed exactly".to_owned());
            }
            let physical = checked_end(
                receipt.first_disposition_record,
                global_sequence,
                "authenticated disposition offset",
            )?;
            let raw = read_fixed_at(
                &mut self.dispositions.file,
                physical,
                EXECUTION_V4_DISPOSITION_BYTES,
                "authenticated disposition",
            )?;
            let disposition = ExecutionV4SuccessorDisposition::authenticate(&raw)?;
            if disposition.population_id() != receipt.population_id
                || disposition.global_sequence() != global_sequence
            {
                return Err(
                    "Execution V4 fixed-offset disposition violates authenticated order".to_owned(),
                );
            }
            self.require_unchanged()?;
            Ok(disposition)
        })();
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Execution V4 disposition lock: {why}"));
        combine_lock_result(result, released)
    }

    fn authenticated_dispositions(
        &mut self,
        receipt: ExecutionV4StructuralReceipt,
    ) -> Result<Vec<ExecutionV4SuccessorDisposition>, ExecutionV4Refusal> {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take Execution V4 bulk lock: {why}"))?;
        let result = (|| {
            self.require_unchanged()?;
            if self.receipts.get(&receipt.population_id) != Some(&receipt) {
                return Err("Execution V4 bulk receipt is no longer indexed exactly".to_owned());
            }
            let completion = ExecutionV4CompletionRecord::decode(&read_fixed_at(
                &mut self.completions.file,
                receipt.block_sequence,
                EXECUTION_V4_COMPLETION_BYTES,
                "Completion",
            )?)?;
            let parameters =
                self.read_parameters(receipt.first_parameter_record, receipt.parameter_count)?;
            let percentiles =
                self.read_percentiles(receipt.first_percentile_record, receipt.percentile_count)?;
            let dispositions = self
                .read_dispositions(receipt.first_disposition_record, receipt.disposition_count)?;
            if validate_complete_block(
                &parameters,
                &percentiles,
                &dispositions,
                &completion,
                self.bounds,
            )? != receipt
            {
                return Err("Execution V4 bulk block differs from exact receipt".to_owned());
            }
            let mut authenticated = bounded_vec(
                receipt.disposition_count,
                self.bounds.dispositions_per_block,
                "authenticated dispositions",
            )?;
            for disposition in dispositions {
                let encoded = disposition.encode()?;
                authenticated.push(ExecutionV4SuccessorDisposition::authenticate(&encoded)?);
            }
            self.require_unchanged()?;
            Ok(authenticated)
        })();
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Execution V4 bulk lock: {why}"));
        combine_lock_result(result, released)
    }

    fn require_unchanged(&self) -> Result<(), ExecutionV4Refusal> {
        if named_identity(&self.root)? != self.root_identity
            || file_generation(&self.lock_file, &self.lock_path, LOCK_MAX_BYTES)?
                != self.lock_generation
        {
            return Err("Execution V4 retained root or lock generation changed".to_owned());
        }
        self.parameters.require_unchanged()?;
        self.percentiles.require_unchanged()?;
        self.dispositions.require_unchanged()?;
        self.completions.require_unchanged()?;
        Ok(())
    }
}

/// Freshly reopened V4 authority. The production wrapper retains the live
/// Population V6 source beside this ledger.
pub(crate) struct ExecutionV4Authority {
    receipt: ExecutionV4StructuralReceipt,
    ledger: ExecutionV4Ledger,
}

impl ExecutionV4Authority {
    #[must_use]
    pub(crate) const fn structural_receipt(&self) -> ExecutionV4StructuralReceipt {
        self.receipt
    }

    pub(crate) fn authenticated_disposition(
        &mut self,
        global_sequence: u64,
    ) -> Result<ExecutionV4SuccessorDisposition, ExecutionV4Refusal> {
        self.ledger
            .authenticated_disposition(self.receipt, global_sequence)
    }

    pub(crate) fn ordered_authenticated_dispositions(
        &mut self,
    ) -> Result<Vec<ExecutionV4SuccessorDisposition>, ExecutionV4Refusal> {
        self.ledger.authenticated_dispositions(self.receipt)
    }
}

/// Immutable terminal projection returned only by a retained Execution V4
/// authority. It has no public constructor and never decodes Population V6.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExecutionV4SuccessorDisposition {
    canonical_record: [u8; EXECUTION_V4_DISPOSITION_BYTES],
    record: ExecutionV4DispositionRecord,
}

impl ExecutionV4SuccessorDisposition {
    fn authenticate(
        canonical_record: &[u8; EXECUTION_V4_DISPOSITION_BYTES],
    ) -> Result<Self, ExecutionV4Refusal> {
        let record = ExecutionV4DispositionRecord::decode(canonical_record)?;
        Ok(Self {
            canonical_record: *canonical_record,
            record,
        })
    }

    #[must_use]
    pub(crate) const fn canonical_record(&self) -> &[u8; EXECUTION_V4_DISPOSITION_BYTES] {
        &self.canonical_record
    }

    #[must_use]
    pub(crate) const fn disposition_id(&self) -> [u8; 32] {
        self.record.disposition_id
    }

    #[must_use]
    pub(crate) const fn population_id(&self) -> [u8; 32] {
        self.record.population_id
    }

    #[must_use]
    pub(crate) const fn population_completion_id(&self) -> [u8; 32] {
        self.record.population_completion_id
    }

    #[must_use]
    pub(crate) const fn population_family_row_id(&self) -> [u8; 32] {
        self.record.population_family_row_id
    }

    #[must_use]
    pub(crate) const fn population_row_id(&self) -> [u8; 32] {
        self.record.population_row_id
    }

    #[must_use]
    pub(crate) const fn candidate_semantic_id(&self) -> [u8; 32] {
        self.record.candidate_semantic_id
    }

    #[must_use]
    pub(crate) const fn candidate_base_row_id(&self) -> [u8; 32] {
        self.record.candidate_base_row_id
    }

    #[must_use]
    pub(crate) const fn base_evidence_id(&self) -> [u8; 32] {
        self.record.base_evidence_id
    }

    #[must_use]
    pub(crate) const fn admission_decision_id(&self) -> [u8; 32] {
        self.record.admission_decision_id
    }

    #[must_use]
    pub(crate) const fn finalization_family_row_id(&self) -> [u8; 32] {
        self.record.finalization_family_row_id
    }

    #[must_use]
    pub(crate) const fn finalization_row_id(&self) -> [u8; 32] {
        self.record.finalization_row_id
    }

    #[must_use]
    pub(crate) const fn finalization_completion_id(&self) -> [u8; 32] {
        self.record.finalization_completion_id
    }

    #[must_use]
    pub(crate) const fn parameter_id(&self) -> [u8; 32] {
        self.record.parameter_id
    }

    #[must_use]
    pub(crate) const fn execution_run_id(&self) -> [u8; 32] {
        self.record.execution_run_id
    }

    #[must_use]
    pub(crate) const fn evaluated_grid_digest(&self) -> [u8; 32] {
        self.record.evaluated_grid_digest
    }

    #[must_use]
    pub(crate) const fn resolution_digest(&self) -> [u8; 32] {
        self.record.resolution_digest
    }

    #[must_use]
    pub(crate) const fn column_digest(&self) -> [u8; 32] {
        self.record.column_digest
    }

    #[must_use]
    pub(crate) const fn context_digest(&self) -> [u8; 32] {
        self.record.context_digest
    }

    #[must_use]
    pub(crate) const fn runner_disposition_digest(&self) -> [u8; 32] {
        self.record.runner_disposition_digest
    }

    #[must_use]
    pub(crate) fn selected_exit_digest(&self) -> Option<[u8; 32]> {
        if self.record.selected_exit_digest == [0; 32] {
            None
        } else {
            Some(self.record.selected_exit_digest)
        }
    }

    #[must_use]
    pub(crate) const fn global_sequence(&self) -> u64 {
        self.record.global_sequence
    }

    #[must_use]
    pub(crate) const fn family_sequence(&self) -> u64 {
        self.record.family_sequence
    }

    #[must_use]
    pub(crate) const fn cell_ordinal(&self) -> u64 {
        self.record.cell_ordinal
    }

    #[must_use]
    pub(crate) const fn support_hits(&self) -> u64 {
        self.record.support_hits
    }

    #[must_use]
    pub(crate) const fn refusal_bits(&self) -> u64 {
        self.record.refusal_bits
    }

    #[must_use]
    pub(crate) const fn family(&self) -> ExecutionV4Family {
        self.record.family
    }

    #[must_use]
    pub(crate) const fn direction(&self) -> ExecutionV4Direction {
        self.record.direction
    }

    #[must_use]
    pub(crate) const fn admission_status(&self) -> ExecutionV4AdmissionStatus {
        self.record.admission_status
    }

    #[must_use]
    pub(crate) const fn terminal(&self) -> ExecutionV4Terminal {
        self.record.terminal
    }

    #[must_use]
    pub(crate) const fn rung(&self) -> u32 {
        self.record.rung
    }

    #[must_use]
    pub(crate) const fn horizon_bars(&self) -> u32 {
        self.record.horizon_bars
    }

    #[must_use]
    pub(crate) const fn stop_index(&self) -> Option<u32> {
        self.record.stop_index
    }

    #[must_use]
    pub(crate) const fn target_index(&self) -> Option<u32> {
        self.record.target_index
    }

    #[must_use]
    pub(crate) const fn tsl_index(&self) -> Option<u32> {
        self.record.tsl_index
    }

    #[must_use]
    pub(crate) const fn ttp_arm_index(&self) -> Option<u32> {
        self.record.ttp_arm_index
    }

    #[must_use]
    pub(crate) const fn ttp_trail_index(&self) -> Option<u32> {
        self.record.ttp_trail_index
    }
}

/// One exact Selection V6 handoff row. Its constructor is private: a caller
/// cannot pair a Population row with a detached durable disposition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExecutionV4SelectionV6RowSourceV1 {
    population: PopulationV6CandidateProjectionV1,
    disposition: ExecutionV4SuccessorDisposition,
}

impl ExecutionV4SelectionV6RowSourceV1 {
    #[must_use]
    pub(crate) const fn population(&self) -> &PopulationV6CandidateProjectionV1 {
        &self.population
    }

    #[must_use]
    pub(crate) const fn disposition(&self) -> &ExecutionV4SuccessorDisposition {
        &self.disposition
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        PopulationV6CandidateProjectionV1,
        ExecutionV4SuccessorDisposition,
    ) {
        (self.population, self.disposition)
    }
}

/// Nonconstructible, fresh-reopen source accepted by the Selection V6 private
/// preparation path. The two Family projections remain present when their row
/// counts are zero, so an all-extinct rung never loses its terminal envelope.
pub(crate) struct ExecutionV4SelectionV6SourceV1 {
    execution: ExecutionV4StructuralReceipt,
    population: PopulationV6StructuralReceipt,
    families: [PopulationV6FamilyProjectionV1; 2],
    rows: Vec<ExecutionV4SelectionV6RowSourceV1>,
}

impl ExecutionV4SelectionV6SourceV1 {
    #[must_use]
    pub(crate) const fn execution(&self) -> ExecutionV4StructuralReceipt {
        self.execution
    }

    #[must_use]
    pub(crate) const fn population(&self) -> PopulationV6StructuralReceipt {
        self.population
    }

    #[must_use]
    pub(crate) const fn families(&self) -> &[PopulationV6FamilyProjectionV1; 2] {
        &self.families
    }

    #[must_use]
    pub(crate) fn rows(&self) -> &[ExecutionV4SelectionV6RowSourceV1] {
        &self.rows
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        ExecutionV4StructuralReceipt,
        PopulationV6StructuralReceipt,
        [PopulationV6FamilyProjectionV1; 2],
        Vec<ExecutionV4SelectionV6RowSourceV1>,
    ) {
        (self.execution, self.population, self.families, self.rows)
    }
}

enum ExecutionV4ProductionCommit {
    Written(ExecutionV4Authority),
    Reused(ExecutionV4Authority),
}

impl ExecutionV4ProductionCommit {
    const fn was_written(&self) -> bool {
        matches!(self, Self::Written(_))
    }

    const fn authority(&self) -> &ExecutionV4Authority {
        match self {
            Self::Written(authority) | Self::Reused(authority) => authority,
        }
    }

    fn authority_mut(&mut self) -> &mut ExecutionV4Authority {
        match self {
            Self::Written(authority) | Self::Reused(authority) => authority,
        }
    }
}

/// Nonconstructible stored Execution V4 capability retaining its complete live
/// Population V6 authority. Selection successors can reauthenticate Population
/// strategy/mask/ranking facts through the retained source; Execution V4 does
/// not expose detached Population rows or Runner dispositions.
pub(crate) struct CommittedStoredExecutionV4 {
    source: CommittedStoredPopulationV6,
    execution: ExecutionV4ProductionCommit,
}

impl CommittedStoredExecutionV4 {
    #[must_use]
    pub(crate) const fn was_written(&self) -> bool {
        self.execution.was_written()
    }

    #[must_use]
    pub(crate) const fn structural_receipt(&self) -> ExecutionV4StructuralReceipt {
        self.execution.authority().structural_receipt()
    }

    #[must_use]
    pub(crate) const fn bounds(&self) -> ExecutionV4Bounds {
        self.execution.authority().ledger.bounds
    }

    /// Internal Population V6 reauthentication used to build the sole
    /// Selection V6 successor capability.
    fn population_execution_source(
        &mut self,
    ) -> Result<PopulationV6ExecutionV4SourceV1, ExecutionV4Refusal> {
        self.source
            .execution_v4_source()
            .map_err(|why| format!("Execution V4 retained Population source refused: {why}"))
    }

    /// Reauthenticates Population before and after a full durable disposition
    /// read and requires every reopened byte to equal the freshly derived
    /// source record.
    fn ordered_authenticated_dispositions(
        &mut self,
    ) -> Result<Vec<ExecutionV4SuccessorDisposition>, ExecutionV4Refusal> {
        let bounds = self.execution.authority_mut().ledger.bounds;
        let source_before = self.population_execution_source()?;
        let prepared = PreparedExecutionV4::from_population_source(source_before, bounds)?;
        let rows = self
            .execution
            .authority_mut()
            .ordered_authenticated_dispositions()?;
        if rows.len() != prepared.dispositions.len() {
            return Err(
                "Execution V4 durable rows differ from the reauthenticated Population source"
                    .to_owned(),
            );
        }
        for (row, expected) in rows.iter().zip(&prepared.dispositions) {
            if row.canonical_record() != &expected.encode()? {
                return Err(
                    "Execution V4 durable rows differ from the reauthenticated Population source"
                        .to_owned(),
                );
            }
        }
        let source_after = self.population_execution_source()?;
        if PreparedExecutionV4::from_population_source(source_after, bounds)? != prepared {
            return Err(
                "Execution V4 retained Population source changed during durable read".to_owned(),
            );
        }
        Ok(rows)
    }

    /// Reauthenticates Population V6, freshly reopens every durable Execution
    /// V4 disposition, exact-joins both sources, and reauthenticates Population
    /// once more before returning one opaque Selection V6 capability.
    pub(crate) fn selection_v6_source(
        &mut self,
    ) -> Result<ExecutionV4SelectionV6SourceV1, ExecutionV4Refusal> {
        let execution_before = self.structural_receipt();
        let bounds = self.bounds();
        let source_before = self.population_execution_source()?;
        let (prepared, population, families, population_rows) =
            PreparedExecutionV4::from_population_source_with_rows(source_before, bounds)?;
        if execution_before.population_id() != population.population_id()
            || execution_before.population_completion_id() != population.completion_id()
            || execution_before.population_ordered_digest() != population.ordered_candidate_digest()
            || execution_before.disposition_count() != population.candidate_count()
            || execution_before.row_count() != population.candidate_count()
            || usize_to_u64(population_rows.len(), "Selection V6 Population row count")?
                != population.candidate_count()
        {
            return Err(
                "Execution V4 and Population V6 receipts do not describe one exact source"
                    .to_owned(),
            );
        }
        let durable = self
            .execution
            .authority_mut()
            .ordered_authenticated_dispositions()?;
        if durable.len() != prepared.dispositions.len() || durable.len() != population_rows.len() {
            return Err(
                "Execution V4 Selection source cardinalities differ after fresh reopen".to_owned(),
            );
        }
        for (actual, expected) in durable.iter().zip(&prepared.dispositions) {
            if actual.canonical_record() != &expected.encode()? {
                return Err(
                    "Execution V4 durable disposition differs from fresh Population replay"
                        .to_owned(),
                );
            }
        }
        let mut rows = Vec::new();
        rows.try_reserve_exact(durable.len())
            .map_err(|why| format!("cannot reserve Execution V4 Selection source rows: {why}"))?;
        for (population_row, disposition) in population_rows.into_iter().zip(durable) {
            if disposition.population_id() != population_row.population_id()
                || disposition.population_completion_id() != population_row.completion_id()
                || disposition.population_row_id() != population_row.row_id()
                || disposition.candidate_semantic_id() != population_row.candidate_semantic_id()
                || disposition.candidate_base_row_id() != population_row.candidate_row_digest()
                || disposition.base_evidence_id() != population_row.base_evidence_id()
                || disposition.admission_decision_id() != population_row.admission_decision_id()
                || disposition.finalization_family_row_id()
                    != population_row.finalization_family_row_id()
                || disposition.finalization_row_id() != population_row.finalization_row_id()
                || disposition.global_sequence() != population_row.global_sequence()
                || disposition.family_sequence() != population_row.family_sequence()
            {
                return Err(
                    "Execution V4 Selection row crosswires Population and disposition lineage"
                        .to_owned(),
                );
            }
            rows.push(ExecutionV4SelectionV6RowSourceV1 {
                population: population_row,
                disposition,
            });
        }
        let source_after = self.population_execution_source()?;
        if PreparedExecutionV4::from_population_source(source_after, bounds)? != prepared
            || self.structural_receipt() != execution_before
        {
            return Err(
                "Execution V4 retained Population/Execution source changed during Selection projection"
                    .to_owned(),
            );
        }
        Ok(ExecutionV4SelectionV6SourceV1 {
            execution: execution_before,
            population,
            families,
            rows,
        })
    }
}

/// The sole production Execution V4 commit door.
///
/// It consumes the nonconstructible stored Population V6 capability, derives
/// every parameter/percentile/disposition from its retained source, writes and
/// freshly reopens V4, then reauthenticates the complete Population
/// source after persistence. There is no detached preparation overload.
pub(crate) fn commit_stored_execution_v4(
    root: &Path,
    bounds: ExecutionV4Bounds,
    mut source: CommittedStoredPopulationV6,
) -> Result<CommittedStoredExecutionV4, ExecutionV4Refusal> {
    let source_before = source
        .execution_v4_source()
        .map_err(|why| format!("Execution V4 Population source refused: {why}"))?;
    let prepared = PreparedExecutionV4::from_population_source(source_before, bounds)?;
    let execution = persist_prepared(root, bounds, &prepared)?;
    let source_after = source
        .execution_v4_source()
        .map_err(|why| format!("Execution V4 post-commit Population source refused: {why}"))?;
    if PreparedExecutionV4::from_population_source(source_after, bounds)? != prepared {
        return Err(
            "Execution V4 Population source changed during persistence and fresh reopen".to_owned(),
        );
    }
    Ok(CommittedStoredExecutionV4 { source, execution })
}

fn persist_prepared(
    root: &Path,
    bounds: ExecutionV4Bounds,
    prepared: &PreparedExecutionV4,
) -> Result<ExecutionV4ProductionCommit, ExecutionV4Refusal> {
    let structural = {
        let mut writer = ExecutionV4Ledger::open_write(root, bounds)?;
        writer.append(prepared)?
    };
    let writer_receipt = structural.receipt();
    let mut reader = ExecutionV4Ledger::open_read(root, bounds)?;
    let reopened = reader
        .structural_receipt(&prepared.population_id)?
        .ok_or_else(|| "Execution V4 fresh reopen did not find committed identity".to_owned())?;
    if reopened != writer_receipt {
        return Err("Execution V4 fresh reopen receipt differs from writer receipt".to_owned());
    }
    reader.require_exact_prepared(reopened, prepared)?;
    let authority = ExecutionV4Authority {
        receipt: reopened,
        ledger: reader,
    };
    if structural.was_written() {
        Ok(ExecutionV4ProductionCommit::Written(authority))
    } else {
        Ok(ExecutionV4ProductionCommit::Reused(authority))
    }
}

#[cfg(test)]
fn commit_prepared_for_test(
    root: &Path,
    bounds: ExecutionV4Bounds,
    prepared: &PreparedExecutionV4,
) -> Result<ExecutionV4ProductionCommit, ExecutionV4Refusal> {
    persist_prepared(root, bounds, prepared)
}

fn validate_complete_block(
    parameters: &[ExecutionV4ParameterRecord],
    percentiles: &[ExecutionV4PercentileRecord],
    dispositions: &[ExecutionV4DispositionRecord],
    completion: &ExecutionV4CompletionRecord,
    bounds: ExecutionV4Bounds,
) -> Result<ExecutionV4StructuralReceipt, ExecutionV4Refusal> {
    completion.validate()?;
    let prepared = PreparedExecutionV4 {
        population_id: completion.population_id,
        population_completion_id: completion.population_completion_id,
        population_ordered_digest: completion.population_ordered_digest,
        source_finalization_id: completion.source_finalization_id,
        source_finalization_completion_id: completion.source_finalization_completion_id,
        source_admission_block_id: completion.source_admission_block_id,
        source_admission_completion_id: completion.source_admission_completion_id,
        families: [
            ExecutionV4FamilyEnvelope {
                family: ExecutionV4Family::Nifty,
                terminal: completion.nifty_terminal,
                population_family_row_id: completion.nifty_population_family_row_id,
                finalization_family_row_id: completion.nifty_finalization_family_row_id,
                candidate_count: completion.nifty_count,
                evaluated_count: completion.nifty_evaluated_count,
                decision_count: completion.nifty_decision_count,
            },
            ExecutionV4FamilyEnvelope {
                family: ExecutionV4Family::BankNifty,
                terminal: completion.banknifty_terminal,
                population_family_row_id: completion.banknifty_population_family_row_id,
                finalization_family_row_id: completion.banknifty_finalization_family_row_id,
                candidate_count: completion.banknifty_count,
                evaluated_count: completion.banknifty_evaluated_count,
                decision_count: completion.banknifty_decision_count,
            },
        ],
        rung: completion.rung,
        horizon_bars: completion.horizon_bars,
        parameters: parameters.to_vec(),
        percentiles: percentiles.to_vec(),
        dispositions: dispositions.to_vec(),
    };
    prepared.validate(bounds)?;
    let expected = prepared.expected_completion(
        completion.block_sequence,
        completion.first_parameter_record,
        completion.first_percentile_record,
        completion.first_disposition_record,
    )?;
    if expected != *completion {
        return Err(
            "Execution V4 Completion does not exactly bind its fixed-record block".to_owned(),
        );
    }
    Ok(ExecutionV4StructuralReceipt::from_completion(completion))
}

fn validate_trailing_prefix(
    parameters: &[ExecutionV4ParameterRecord],
    percentiles: &[ExecutionV4PercentileRecord],
    dispositions: &[ExecutionV4DispositionRecord],
    bounds: ExecutionV4Bounds,
) -> Result<(), ExecutionV4Refusal> {
    if parameters.is_empty() {
        if percentiles.is_empty() && dispositions.is_empty() {
            return Ok(());
        }
        return Err(
            "Execution V4 orphan has percentile/disposition records without parameters".to_owned(),
        );
    }
    if parameters.len() > MAX_PARAMETER_RECORDS_PER_BLOCK {
        return Err("Execution V4 orphan has more than four parameter records".to_owned());
    }
    require_block_counts(bounds, percentiles.len(), dispositions.len())?;
    let first = parameters
        .first()
        .ok_or_else(|| "Execution V4 orphan parameter prefix is empty".to_owned())?;
    let expected_order = match first.family {
        ExecutionV4Family::Nifty => [
            Some((ExecutionV4Family::Nifty, ExecutionV4Direction::Long)),
            Some((ExecutionV4Family::Nifty, ExecutionV4Direction::Short)),
            Some((ExecutionV4Family::BankNifty, ExecutionV4Direction::Long)),
            Some((ExecutionV4Family::BankNifty, ExecutionV4Direction::Short)),
        ],
        ExecutionV4Family::BankNifty => [
            Some((ExecutionV4Family::BankNifty, ExecutionV4Direction::Long)),
            Some((ExecutionV4Family::BankNifty, ExecutionV4Direction::Short)),
            None,
            None,
        ],
    };
    if parameters.len() > expected_order.iter().flatten().count() {
        return Err("Execution V4 orphan parameter topology is impossible".to_owned());
    }
    let mut expected_offset = 0_u64;
    for (parameter, expected) in parameters.iter().zip(expected_order) {
        let (family, direction) = expected
            .ok_or_else(|| "Execution V4 orphan parameter topology escaped schema".to_owned())?;
        parameter.validate()?;
        if parameter.family != family
            || parameter.direction != direction
            || parameter.population_id != first.population_id
            || parameter.population_completion_id != first.population_completion_id
            || parameter.population_ordered_digest != first.population_ordered_digest
            || parameter.source_finalization_id != first.source_finalization_id
            || parameter.source_finalization_completion_id
                != first.source_finalization_completion_id
            || parameter.source_admission_block_id != first.source_admission_block_id
            || parameter.source_admission_completion_id != first.source_admission_completion_id
            || parameter.execution_law_digest != first.execution_law_digest
            || parameter.rung != first.rung
            || parameter.horizon_bars != first.horizon_bars
            || parameter.percentile_offset != expected_offset
        {
            return Err("Execution V4 orphan parameter prefix is not canonical".to_owned());
        }
        expected_offset = checked_end(
            expected_offset,
            parameter.percentile_count,
            "orphan percentile range",
        )?;
    }
    let parameter_block_is_complete = match (first.family, parameters.len()) {
        (ExecutionV4Family::BankNifty, 2) | (ExecutionV4Family::Nifty, 4) => true,
        (ExecutionV4Family::Nifty, 2) if !percentiles.is_empty() || !dispositions.is_empty() => {
            true
        }
        _ => false,
    };
    if !parameter_block_is_complete {
        if !percentiles.is_empty() || !dispositions.is_empty() {
            return Err(
                "Execution V4 orphan wrote a later file before its active-Family parameters"
                    .to_owned(),
            );
        }
        return Ok(());
    }
    let expected_percentiles = usize::try_from(expected_offset)
        .map_err(|_| "Execution V4 orphan percentile count does not fit usize".to_owned())?;
    if percentiles.len() > expected_percentiles {
        return Err("Execution V4 orphan has excess percentile records".to_owned());
    }
    validate_percentile_prefix(parameters, percentiles)?;
    if percentiles.len() < expected_percentiles {
        if !dispositions.is_empty() {
            return Err(
                "Execution V4 orphan wrote dispositions before all percentile atoms".to_owned(),
            );
        }
        return Ok(());
    }
    validate_disposition_prefix(parameters, percentiles, dispositions, first.population_id)?;
    Ok(())
}

fn validate_percentile_segment(
    parameter: &ExecutionV4ParameterRecord,
    segment: &[ExecutionV4PercentileRecord],
) -> Result<(), ExecutionV4Refusal> {
    if segment.is_empty()
        || u64::try_from(segment.len())
            .map_err(|_| "Execution V4 percentile segment length does not fit u64".to_owned())?
            != parameter.percentile_count
    {
        return Err("Execution V4 parameter has an empty/incomplete percentile segment".to_owned());
    }
    let axis_counts = validate_one_percentile_prefix(parameter, segment)?;
    if axis_counts.into_iter().any(|count| count == 0) {
        return Err(
            "Execution V4 parameter percentile schedule does not cover Stop, Target, and Trail"
                .to_owned(),
        );
    }
    let [
        stop_schedule_count,
        target_schedule_count,
        trail_schedule_count,
    ] = axis_counts;
    let stop_schedule_ceiling = stop_schedule_count
        .checked_add(u64::from(
            parameter.forced_stop_policy_tag != FORCED_STOP_DISABLED_TAG,
        ))
        .ok_or_else(|| "Execution V4 forced-stop schedule ceiling overflowed".to_owned())?;
    if stop_schedule_count > parameter.max_levels
        || target_schedule_count > parameter.max_levels
        || trail_schedule_count > parameter.max_levels
        || parameter.resolved_stop_count > stop_schedule_ceiling
        || parameter.resolved_target_count > target_schedule_count
        || parameter.resolved_trail_count > trail_schedule_count
    {
        return Err(
            "Execution V4 percentile schedules/resolved axes exceed policy or forced-stop bounds"
                .to_owned(),
        );
    }
    if ordered_percentile_digest(segment)? != parameter.percentile_digest {
        return Err("Execution V4 parameter percentile digest does not reproduce".to_owned());
    }
    Ok(())
}

fn validate_percentile_prefix(
    parameters: &[ExecutionV4ParameterRecord],
    percentiles: &[ExecutionV4PercentileRecord],
) -> Result<(), ExecutionV4Refusal> {
    for parameter in parameters {
        let start = usize::try_from(parameter.percentile_offset)
            .map_err(|_| "Execution V4 percentile offset does not fit usize".to_owned())?;
        if start >= percentiles.len() {
            break;
        }
        let declared_end = checked_end(
            parameter.percentile_offset,
            parameter.percentile_count,
            "percentile prefix range",
        )?;
        let end = usize::try_from(declared_end)
            .map_err(|_| "Execution V4 percentile prefix end does not fit usize".to_owned())?
            .min(percentiles.len());
        let segment = percentiles.get(start..end).ok_or_else(|| {
            "Execution V4 percentile prefix segment escaped prepared atoms".to_owned()
        })?;
        validate_one_percentile_prefix(parameter, segment)?;
        if end
            == usize::try_from(declared_end)
                .map_err(|_| "Execution V4 percentile prefix end does not fit usize".to_owned())?
            && ordered_percentile_digest(segment)? != parameter.percentile_digest
        {
            return Err("Execution V4 completed orphan percentile digest differs".to_owned());
        }
    }
    Ok(())
}

fn validate_one_percentile_prefix(
    parameter: &ExecutionV4ParameterRecord,
    segment: &[ExecutionV4PercentileRecord],
) -> Result<[u64; 3], ExecutionV4Refusal> {
    let mut previous: Option<&ExecutionV4PercentileRecord> = None;
    let mut axis_counts = [0_u64; 3];
    for percentile in segment {
        percentile.validate()?;
        if percentile.parameter_core_id != parameter.parameter_core_id {
            return Err("Execution V4 percentile atom changes parameter identity".to_owned());
        }
        let axis_index = percentile_axis_index(percentile.axis);
        let axis_count = axis_counts
            .get(axis_index)
            .copied()
            .ok_or_else(|| "Execution V4 percentile axis escaped schema".to_owned())?;
        if percentile.ordinal
            != u32::try_from(axis_count)
                .map_err(|_| "Execution V4 percentile axis count does not fit u32".to_owned())?
        {
            return Err(
                "Execution V4 percentile ordinals are not contiguous from zero per axis".to_owned(),
            );
        }
        match previous {
            None if percentile.axis != ExecutionV4PercentileAxis::Stop => {
                return Err("Execution V4 percentile schedule does not begin with Stop".to_owned());
            }
            Some(prior) if percentile.axis == prior.axis => {
                let left = u64::from(percentile.numerator) * u64::from(prior.denominator);
                let right = u64::from(prior.numerator) * u64::from(percentile.denominator);
                if left <= right {
                    return Err(
                        "Execution V4 percentile values are not strictly increasing within an axis"
                            .to_owned(),
                    );
                }
            }
            Some(prior)
                if percentile_axis_index(percentile.axis)
                    != percentile_axis_index(prior.axis) + 1 =>
            {
                return Err(
                    "Execution V4 percentile axes are missing or not canonically ordered"
                        .to_owned(),
                );
            }
            None | Some(_) => {}
        }
        let axis_count = axis_counts
            .get_mut(axis_index)
            .ok_or_else(|| "Execution V4 percentile axis escaped schema".to_owned())?;
        increment(axis_count, "percentile axis count")?;
        previous = Some(percentile);
    }
    Ok(axis_counts)
}

const fn percentile_axis_index(axis: ExecutionV4PercentileAxis) -> usize {
    axis as usize - 1
}

fn validate_disposition_block(prepared: &PreparedExecutionV4) -> Result<(), ExecutionV4Refusal> {
    validate_disposition_prefix(
        &prepared.parameters,
        &prepared.percentiles,
        &prepared.dispositions,
        prepared.population_id,
    )?;
    let mut family_dispositions = [0_u64; 2];
    for row in &prepared.dispositions {
        let envelope = prepared
            .families
            .get(family_index(row.family))
            .ok_or_else(|| "Execution V4 disposition Family envelope is absent".to_owned())?;
        if row.population_completion_id != prepared.population_completion_id
            || row.population_family_row_id != envelope.population_family_row_id
            || row.finalization_family_row_id != envelope.finalization_family_row_id
            || envelope.terminal != ExecutionV4FamilyTerminal::Evaluated
        {
            return Err(
                "Execution V4 disposition escapes its Population V6 evaluated Family envelope"
                    .to_owned(),
            );
        }
        let count = family_dispositions
            .get_mut(family_index(row.family))
            .ok_or_else(|| "Execution V4 disposition Family count is absent".to_owned())?;
        increment(count, "Family disposition count")?;
    }
    for (envelope, dispositions) in prepared.families.iter().zip(family_dispositions) {
        validate_terminal_envelope(
            envelope.terminal,
            envelope.candidate_count,
            envelope.evaluated_count,
            envelope.decision_count,
            dispositions,
        )?;
    }
    Ok(())
}

fn validate_disposition_prefix(
    parameters: &[ExecutionV4ParameterRecord],
    percentiles: &[ExecutionV4PercentileRecord],
    dispositions: &[ExecutionV4DispositionRecord],
    population_id: [u8; 32],
) -> Result<(), ExecutionV4Refusal> {
    if parameters.is_empty() {
        if dispositions.is_empty() && percentiles.is_empty() {
            return Ok(());
        }
        return Err(
            "Execution V4 parameterless extinct block contains percentile/disposition evidence"
                .to_owned(),
        );
    }
    if !matches!(parameters.len(), 2 | MAX_PARAMETER_RECORDS_PER_BLOCK) {
        return Err(
            "Execution V4 dispositions require two or four active-Family parameters".to_owned(),
        );
    }
    let mut identities = DispositionIdentitySets::with_capacity(dispositions.len())?;
    let finalization_completion = dispositions
        .first()
        .map(|row| row.finalization_completion_id);
    disposition_schedule_counts(parameters, percentiles)?;
    let mut sequences = DispositionSequences::default();
    for (index, disposition) in dispositions.iter().enumerate() {
        validate_one_disposition_prefix(
            index,
            disposition,
            population_id,
            finalization_completion,
            parameters,
            &mut identities,
            &mut sequences,
        )?;
    }
    Ok(())
}

struct DispositionIdentitySets {
    disposition_ids: HashSet<[u8; 32]>,
    population_rows: HashSet<[u8; 32]>,
    candidates: HashSet<[u8; 32]>,
    candidate_base_rows: HashSet<[u8; 32]>,
    admissions: HashSet<[u8; 32]>,
    finalizations: HashSet<[u8; 32]>,
}

impl DispositionIdentitySets {
    fn with_capacity(count: usize) -> Result<Self, ExecutionV4Refusal> {
        let mut value = Self {
            disposition_ids: HashSet::new(),
            population_rows: HashSet::new(),
            candidates: HashSet::new(),
            candidate_base_rows: HashSet::new(),
            admissions: HashSet::new(),
            finalizations: HashSet::new(),
        };
        for (set, name) in [
            (&mut value.disposition_ids, "disposition identities"),
            (&mut value.population_rows, "Population V6 rows"),
            (&mut value.candidates, "Candidates"),
            (&mut value.candidate_base_rows, "Candidate base rows"),
            (&mut value.admissions, "Admission V4 decisions"),
            (&mut value.finalizations, "Finalization V4 rows"),
        ] {
            set.try_reserve(count)
                .map_err(|why| format!("cannot reserve Execution V4 {name}: {why}"))?;
        }
        Ok(value)
    }

    fn insert(&mut self, row: &ExecutionV4DispositionRecord) -> bool {
        self.disposition_ids.insert(row.disposition_id)
            && self.population_rows.insert(row.population_row_id)
            && self.candidates.insert(row.candidate_semantic_id)
            && self.candidate_base_rows.insert(row.candidate_base_row_id)
            && self.admissions.insert(row.admission_decision_id)
            && self.finalizations.insert(row.finalization_row_id)
    }
}

#[derive(Default)]
struct DispositionSequences {
    banknifty_started: bool,
    nifty: u64,
    banknifty: u64,
}

impl DispositionSequences {
    fn observe(&mut self, row: &ExecutionV4DispositionRecord) -> Result<(), ExecutionV4Refusal> {
        match row.family {
            ExecutionV4Family::Nifty if !self.banknifty_started => {
                if row.family_sequence != self.nifty {
                    return Err("Execution V4 NIFTY sequence is not contiguous".to_owned());
                }
                increment(&mut self.nifty, "NIFTY sequence")
            }
            ExecutionV4Family::BankNifty => {
                self.banknifty_started = true;
                if row.family_sequence != self.banknifty {
                    return Err("Execution V4 BANKNIFTY sequence is not contiguous".to_owned());
                }
                increment(&mut self.banknifty, "BANKNIFTY sequence")
            }
            ExecutionV4Family::Nifty => {
                Err("Execution V4 NIFTY disposition follows BANKNIFTY".to_owned())
            }
        }
    }
}

fn disposition_schedule_counts(
    parameters: &[ExecutionV4ParameterRecord],
    percentiles: &[ExecutionV4PercentileRecord],
) -> Result<[[u64; 3]; MAX_PARAMETER_RECORDS_PER_BLOCK], ExecutionV4Refusal> {
    let common_rung = parameters
        .first()
        .ok_or_else(|| "Execution V4 parameter block is empty".to_owned())?
        .rung;
    let mut counts = [[0_u64; 3]; MAX_PARAMETER_RECORDS_PER_BLOCK];
    for (index, parameter) in parameters.iter().enumerate() {
        if parameter.rung != common_rung {
            return Err("Execution V4 parameter block mixes signal rungs".to_owned());
        }
        let start = usize::try_from(parameter.percentile_offset)
            .map_err(|_| "Execution V4 percentile start does not fit usize".to_owned())?;
        let end = usize::try_from(checked_end(
            parameter.percentile_offset,
            parameter.percentile_count,
            "disposition percentile schedule",
        )?)
        .map_err(|_| "Execution V4 percentile end does not fit usize".to_owned())?;
        let segment = percentiles.get(start..end).ok_or_else(|| {
            "Execution V4 disposition parameter schedule exceeds percentile block".to_owned()
        })?;
        validate_percentile_segment(parameter, segment)?;
        let slot = counts
            .get_mut(index)
            .ok_or_else(|| "Execution V4 parameter schedule index escaped schema".to_owned())?;
        *slot = validate_one_percentile_prefix(parameter, segment)?;
    }
    Ok(counts)
}

fn validate_one_disposition_prefix(
    index: usize,
    row: &ExecutionV4DispositionRecord,
    population_id: [u8; 32],
    finalization_completion: Option<[u8; 32]>,
    parameters: &[ExecutionV4ParameterRecord],
    identities: &mut DispositionIdentitySets,
    sequences: &mut DispositionSequences,
) -> Result<(), ExecutionV4Refusal> {
    row.validate()?;
    if row.population_id != population_id
        || row.global_sequence != usize_to_u64(index, "disposition ordinal")?
        || Some(row.finalization_completion_id) != finalization_completion
        || !identities.insert(row)
    {
        return Err(
            "Execution V4 disposition source/order/unique identities are inconsistent".to_owned(),
        );
    }
    sequences.observe(row)?;
    let parameter = find_parameter(parameters, row.family, row.direction)?;
    require_disposition_parameter_join(row, parameter)?;
    require_resolved_coordinate_bounds(row, parameter)
}

const fn family_index(family: ExecutionV4Family) -> usize {
    match family {
        ExecutionV4Family::Nifty => 0,
        ExecutionV4Family::BankNifty => 1,
    }
}

fn expected_parameter_count(
    families: &[ExecutionV4FamilyEnvelope; 2],
) -> Result<usize, ExecutionV4Refusal> {
    families
        .iter()
        .filter(|family| family.terminal == ExecutionV4FamilyTerminal::Evaluated)
        .count()
        .checked_mul(2)
        .ok_or_else(|| "Execution V4 evaluated-Family parameter count overflowed".to_owned())
}

fn find_parameter(
    parameters: &[ExecutionV4ParameterRecord],
    family: ExecutionV4Family,
    direction: ExecutionV4Direction,
) -> Result<&ExecutionV4ParameterRecord, ExecutionV4Refusal> {
    let mut matching = parameters
        .iter()
        .filter(|parameter| parameter.family == family && parameter.direction == direction);
    let parameter = matching.next().ok_or_else(|| {
        "Execution V4 disposition has no exact active-family/direction parameter".to_owned()
    })?;
    if matching.next().is_some() {
        return Err(
            "Execution V4 disposition has duplicate active-family/direction parameters".to_owned(),
        );
    }
    Ok(parameter)
}

fn parameter_id_slots(
    parameters: &[ExecutionV4ParameterRecord],
    families: &[ExecutionV4FamilyEnvelope; 2],
) -> Result<[[u8; 32]; MAX_PARAMETER_RECORDS_PER_BLOCK], ExecutionV4Refusal> {
    let order = [
        (ExecutionV4Family::Nifty, ExecutionV4Direction::Long),
        (ExecutionV4Family::Nifty, ExecutionV4Direction::Short),
        (ExecutionV4Family::BankNifty, ExecutionV4Direction::Long),
        (ExecutionV4Family::BankNifty, ExecutionV4Direction::Short),
    ];
    let mut slots = [[0_u8; 32]; MAX_PARAMETER_RECORDS_PER_BLOCK];
    for (slot, (family, direction)) in slots.iter_mut().zip(order) {
        if families[family_index(family)].terminal == ExecutionV4FamilyTerminal::Evaluated {
            *slot = find_parameter(parameters, family, direction)?.parameter_id;
        }
    }
    Ok(slots)
}

fn validate_parameter_id_slots(
    slots: [[u8; 32]; MAX_PARAMETER_RECORDS_PER_BLOCK],
    nifty_terminal: ExecutionV4FamilyTerminal,
    banknifty_terminal: ExecutionV4FamilyTerminal,
    parameter_count: u64,
) -> Result<(), ExecutionV4Refusal> {
    let active = [
        nifty_terminal == ExecutionV4FamilyTerminal::Evaluated,
        banknifty_terminal == ExecutionV4FamilyTerminal::Evaluated,
    ];
    let expected_count = active
        .iter()
        .filter(|value| **value)
        .count()
        .checked_mul(2)
        .ok_or_else(|| "Execution V4 Completion parameter count overflowed".to_owned())?;
    if parameter_count != usize_to_u64(expected_count, "Completion parameter count")? {
        return Err(
            "Execution V4 Completion does not retain two parameters per evaluated Family"
                .to_owned(),
        );
    }
    let mut seen = HashSet::new();
    seen.try_reserve(expected_count)
        .map_err(|why| format!("cannot reserve Completion parameter identities: {why}"))?;
    for (family_index, is_active) in active.into_iter().enumerate() {
        let start = family_index * 2;
        let pair = slots
            .get(start..start + 2)
            .ok_or_else(|| "Execution V4 Completion parameter slots escaped schema".to_owned())?;
        for parameter_id in pair {
            if is_active {
                require_nonzero("Completion active parameter identity", *parameter_id)?;
                if !seen.insert(*parameter_id) {
                    return Err(
                        "Execution V4 Completion repeats an active parameter identity".to_owned(),
                    );
                }
            } else if *parameter_id != [0; 32] {
                return Err(
                    "Execution V4 Completion invents a parameter for an extinct Family".to_owned(),
                );
            }
        }
    }
    Ok(())
}

fn require_disposition_parameter_join(
    row: &ExecutionV4DispositionRecord,
    parameter: &ExecutionV4ParameterRecord,
) -> Result<(), ExecutionV4Refusal> {
    if row.parameter_id != parameter.parameter_id
        || row.resolution_digest != parameter.resolution_digest
        || row.rung != parameter.rung
        || row.horizon_bars != parameter.horizon_bars
        || row.cell_ordinal >= parameter.resolved_cell_count
    {
        return Err(
            "Execution V4 disposition does not join its exact family/direction parameter"
                .to_owned(),
        );
    }
    if parameter.forced_stop_policy_tag == FORCED_STOP_REQUIRE_TAG
        && row.terminal == ExecutionV4Terminal::Authorized
        && row.stop_index != parameter.forced_stop_index
    {
        return Err(
            "Execution V4 Authorized coordinate violates its required forced stop".to_owned(),
        );
    }
    Ok(())
}

fn require_resolved_coordinate_bounds(
    row: &ExecutionV4DispositionRecord,
    parameter: &ExecutionV4ParameterRecord,
) -> Result<(), ExecutionV4Refusal> {
    for (name, coordinate, count) in [
        ("stop", row.stop_index, parameter.resolved_stop_count),
        ("target", row.target_index, parameter.resolved_target_count),
        ("TSL", row.tsl_index, parameter.resolved_trail_count),
        (
            "TTP arm",
            row.ttp_arm_index,
            parameter.resolved_target_count,
        ),
        (
            "TTP trail",
            row.ttp_trail_index,
            parameter.resolved_trail_count,
        ),
    ] {
        if coordinate.is_some_and(|index| index == OPTIONAL_U32_NONE || u64::from(index) >= count) {
            return Err(format!(
                "Execution V4 {name} index {coordinate:?} is outside its {count}-level resolved axis"
            ));
        }
    }
    if row
        .ttp_arm_index
        .zip(row.target_index)
        .is_some_and(|(arm, target)| arm >= target)
        || row
            .ttp_trail_index
            .zip(row.tsl_index)
            .is_some_and(|(trail, tsl)| trail >= tsl)
    {
        return Err(
            "Execution V4 TTP arm/trail is not strictly below its target/TSL cap".to_owned(),
        );
    }
    // Cardinalities cannot authenticate which stop-target pairs Runner admitted.
    // The production join must retain the full ResolvedExitGridV1 and consume
    // classify_coordinate; this fixed record only refuses impossible bounds.
    Ok(())
}

fn ordered_parameter_digest(
    records: &[ExecutionV4ParameterRecord],
) -> Result<[u8; 32], ExecutionV4Refusal> {
    let mut hasher = begin_ordered_digest(ORDERED_PARAMETERS_DOMAIN, records.len())?;
    for record in records {
        hasher.update(&record.encode()?);
    }
    Ok(hasher.finalize())
}

fn ordered_percentile_digest(
    records: &[ExecutionV4PercentileRecord],
) -> Result<[u8; 32], ExecutionV4Refusal> {
    let mut hasher = begin_ordered_digest(ORDERED_PERCENTILES_DOMAIN, records.len())?;
    for record in records {
        hasher.update(&record.encode()?);
    }
    Ok(hasher.finalize())
}

fn ordered_disposition_digest(
    records: &[ExecutionV4DispositionRecord],
) -> Result<[u8; 32], ExecutionV4Refusal> {
    let mut hasher = begin_ordered_digest(ORDERED_DISPOSITIONS_DOMAIN, records.len())?;
    for record in records {
        hasher.update(&record.encode()?);
    }
    Ok(hasher.finalize())
}

fn family_disposition_digest(
    family: ExecutionV4Family,
    records: &[ExecutionV4DispositionRecord],
) -> Result<[u8; 32], ExecutionV4Refusal> {
    let count = records.iter().filter(|row| row.family == family).count();
    let mut hasher = begin_ordered_digest(ORDERED_DISPOSITIONS_DOMAIN, count)?;
    hasher.update(&[family as u8]);
    for record in records.iter().filter(|row| row.family == family) {
        hasher.update(&record.encode()?);
    }
    Ok(hasher.finalize())
}

fn begin_ordered_digest(domain: &[u8], count: usize) -> Result<Hasher, ExecutionV4Refusal> {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    hasher.update(&VERSION.to_le_bytes());
    hasher.update(&usize_to_u64(count, "ordered digest count")?.to_le_bytes());
    Ok(hasher)
}

fn derive_family_authority_id(
    family: ExecutionV4FamilyEnvelope,
    prepared: &PreparedExecutionV4,
    parameter_ids: [[u8; 32]; 2],
    disposition_digest: [u8; 32],
    matrix: &[u64],
) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(FAMILY_AUTHORITY_ID_DOMAIN);
    hasher.update(&VERSION.to_le_bytes());
    hasher.update(&[family.family as u8, family.terminal as u8]);
    let [long_parameter_id, short_parameter_id] = parameter_ids;
    for value in [
        prepared.population_id,
        prepared.population_completion_id,
        prepared.population_ordered_digest,
        family.population_family_row_id,
        family.finalization_family_row_id,
        prepared.source_finalization_id,
        prepared.source_finalization_completion_id,
        prepared.source_admission_block_id,
        prepared.source_admission_completion_id,
        execution_law_digest(),
        long_parameter_id,
        short_parameter_id,
        disposition_digest,
    ] {
        hasher.update(&value);
    }
    for value in [
        family.candidate_count,
        family.evaluated_count,
        family.decision_count,
    ] {
        hasher.update(&value.to_le_bytes());
    }
    hasher.update(&prepared.rung.to_le_bytes());
    hasher.update(&prepared.horizon_bars.to_le_bytes());
    for value in matrix {
        hasher.update(&value.to_le_bytes());
    }
    hasher.finalize()
}

fn execution_law_digest() -> [u8; 32] {
    crate::execution_capability::exact_execution_law_digest_v1()
}

fn matrix_index(
    family: ExecutionV4Family,
    status: ExecutionV4AdmissionStatus,
    terminal: ExecutionV4Terminal,
) -> usize {
    let family_offset = match family {
        ExecutionV4Family::Nifty => 0,
        ExecutionV4Family::BankNifty => 8,
    };
    family_offset + admission_index(status) * 2 + terminal_index(terminal)
}

const fn admission_index(status: ExecutionV4AdmissionStatus) -> usize {
    status as usize - 1
}

const fn terminal_index(terminal: ExecutionV4Terminal) -> usize {
    terminal as usize - 1
}

fn require_block_counts(
    bounds: ExecutionV4Bounds,
    percentiles: usize,
    dispositions: usize,
) -> Result<(), ExecutionV4Refusal> {
    let percentiles = usize_to_u64(percentiles, "percentile block count")?;
    let dispositions = usize_to_u64(dispositions, "disposition block count")?;
    if percentiles > bounds.percentiles_per_block || dispositions > bounds.dispositions_per_block {
        return Err(format!(
            "Execution V4 block counts {percentiles} percentiles/{dispositions} dispositions exceed explicit bounds {}/{}",
            bounds.percentiles_per_block, bounds.dispositions_per_block
        ));
    }
    Ok(())
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

fn open_root_directory(
    root: &Path,
) -> Result<(PathBuf, File, PlatformIdentity), ExecutionV4Refusal> {
    require_not_symlink(root, false)?;
    let canonical = std::fs::canonicalize(root).map_err(|why| {
        format!(
            "Execution V4 root {} must already exist: {why}",
            root.display()
        )
    })?;
    require_not_symlink(&canonical, false)?;
    let file = File::open(&canonical).map_err(|why| {
        format!(
            "cannot hold Execution V4 root {}: {why}",
            canonical.display()
        )
    })?;
    let metadata = file.metadata().map_err(|why| {
        format!(
            "cannot stat held Execution V4 root {}: {why}",
            canonical.display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "Execution V4 root {} is not a directory",
            canonical.display()
        ));
    }
    let identity = PlatformIdentity::of(&metadata);
    if named_identity(&canonical)? != identity {
        return Err("Execution V4 root changed while it was opened".to_owned());
    }
    Ok((canonical, file, identity))
}

fn named_identity(path: &Path) -> Result<PlatformIdentity, ExecutionV4Refusal> {
    require_not_symlink(path, false)?;
    let metadata = std::fs::metadata(path).map_err(|why| {
        format!(
            "cannot stat named Execution V4 path {}: {why}",
            path.display()
        )
    })?;
    Ok(PlatformIdentity::of(&metadata))
}

fn open_child(
    path: &Path,
    writable: bool,
    create: bool,
) -> Result<(File, bool), ExecutionV4Refusal> {
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
                    "cannot create Execution V4 file {}: {why}",
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
        .map_err(|why| format!("cannot open Execution V4 file {}: {why}", path.display()))?;
    require_not_symlink(path, false)?;
    require_regular_file(&file, path)?;
    Ok((file, false))
}

fn require_regular_file(file: &File, path: &Path) -> Result<(), ExecutionV4Refusal> {
    let metadata = file
        .metadata()
        .map_err(|why| format!("cannot stat Execution V4 file {}: {why}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!(
            "Execution V4 path {} is not a regular file",
            path.display()
        ));
    }
    require_single_link(&metadata, path)
}

fn require_single_link(
    metadata: &std::fs::Metadata,
    path: &Path,
) -> Result<(), ExecutionV4Refusal> {
    #[cfg(unix)]
    if metadata.nlink() != 1 {
        return Err(format!(
            "Execution V4 path {} has {} hard links; expected exactly one",
            path.display(),
            metadata.nlink()
        ));
    }
    #[cfg(not(unix))]
    let _ = (metadata, path);
    Ok(())
}

fn require_not_symlink(path: &Path, absent_allowed: bool) -> Result<(), ExecutionV4Refusal> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(format!(
            "Execution V4 path {} is a symbolic link; no-follow refused it",
            path.display()
        )),
        Ok(_) => Ok(()),
        Err(why) if absent_allowed && why.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(why) => Err(format!(
            "cannot inspect Execution V4 path {} without following links: {why}",
            path.display()
        )),
    }
}

fn sync_directory(root_file: &File, root: &Path) -> Result<(), ExecutionV4Refusal> {
    root_file.sync_all().map_err(|why| {
        format!(
            "cannot durably sync Execution V4 directory {}: {why}",
            root.display()
        )
    })
}

fn file_generation(
    file: &File,
    path: &Path,
    max_bytes: u64,
) -> Result<FileGeneration, ExecutionV4Refusal> {
    let before = file.metadata().map_err(|why| {
        format!(
            "cannot stat held Execution V4 file {}: {why}",
            path.display()
        )
    })?;
    if !before.is_file() || before.len() > max_bytes {
        return Err(format!(
            "Execution V4 file {} is not regular or its {} bytes exceed bound {max_bytes}",
            path.display(),
            before.len()
        ));
    }
    require_single_link(&before, path)?;
    let platform = PlatformIdentity::of(&before);
    if named_identity(path)? != platform {
        return Err(format!(
            "Execution V4 file {} was path-replaced",
            path.display()
        ));
    }
    let len = before.len();
    let modified = before.modified().ok();
    let first = hash_held_file(file, path, len)?;
    let middle = file.metadata().map_err(|why| {
        format!(
            "cannot restat Execution V4 file {} after first hash: {why}",
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
            "Execution V4 file {} changed during generation hash",
            path.display()
        ));
    }
    let second = hash_held_file(file, path, len)?;
    let after = file.metadata().map_err(|why| {
        format!(
            "cannot restat Execution V4 file {} after confirmation hash: {why}",
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
            "Execution V4 file {} changed between bounded generation hashes",
            path.display()
        ));
    }
    let mut hasher = Hasher::new();
    hasher.update(FILE_GENERATION_DOMAIN);
    hasher.update(&VERSION.to_le_bytes());
    hasher.update(&len.to_le_bytes());
    platform.update_hasher(&mut hasher);
    hasher.update(&first);
    Ok(FileGeneration {
        len,
        platform,
        modified,
        content_digest: first,
        generation_digest: hasher.finalize(),
    })
}

fn hash_held_file(file: &File, path: &Path, len: u64) -> Result<[u8; 32], ExecutionV4Refusal> {
    let mut reader = file
        .try_clone()
        .map_err(|why| format!("cannot clone Execution V4 file {}: {why}", path.display()))?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|why| format!("cannot seek Execution V4 file {}: {why}", path.display()))?;
    let mut hasher = Hasher::new();
    hasher.update(FILE_GENERATION_DOMAIN);
    hasher.update(&len.to_le_bytes());
    let mut remaining = len;
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    while remaining != 0 {
        let requested = usize::try_from(remaining.min(READ_CHUNK_BYTES as u64))
            .map_err(|_| "Execution V4 hash width does not fit usize".to_owned())?;
        let requested_buffer = buffer
            .get_mut(..requested)
            .ok_or_else(|| "Execution V4 hash request exceeds fixed buffer".to_owned())?;
        let read = reader
            .read(requested_buffer)
            .map_err(|why| format!("cannot hash Execution V4 file {}: {why}", path.display()))?;
        if read == 0 {
            return Err(format!(
                "Execution V4 file {} shortened while hashing",
                path.display()
            ));
        }
        let read_buffer = buffer
            .get(..read)
            .ok_or_else(|| "Execution V4 hash read exceeds fixed buffer".to_owned())?;
        hasher.update(read_buffer);
        remaining = remaining
            .checked_sub(usize_to_u64(read, "hash read")?)
            .ok_or_else(|| "Execution V4 hash remaining count underflowed".to_owned())?;
    }
    Ok(hasher.finalize())
}

fn checked_record_count(
    bytes: u64,
    stride: usize,
    max_records: u64,
    name: &str,
) -> Result<u64, ExecutionV4Refusal> {
    let stride = u64::try_from(stride)
        .map_err(|_| format!("Execution V4 {name} stride does not fit u64"))?;
    if !bytes.is_multiple_of(stride) {
        return Err(format!(
            "Execution V4 {name} file has ragged length {bytes}, not a multiple of {stride}"
        ));
    }
    let records = bytes / stride;
    if records > max_records {
        return Err(format!(
            "Execution V4 {name} file has {records} records above bound {max_records}"
        ));
    }
    Ok(records)
}

fn read_fixed_at<const N: usize>(
    file: &mut File,
    physical: u64,
    stride: usize,
    name: &str,
) -> Result<[u8; N], ExecutionV4Refusal> {
    if N != stride {
        return Err(format!(
            "Execution V4 {name} buffer {N} differs from stride {stride}"
        ));
    }
    let stride = u64::try_from(stride)
        .map_err(|_| format!("Execution V4 {name} stride does not fit u64"))?;
    let offset = physical
        .checked_mul(stride)
        .ok_or_else(|| format!("Execution V4 {name} offset overflowed"))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|why| format!("cannot seek Execution V4 {name} {physical}: {why}"))?;
    let mut raw = [0_u8; N];
    file.read_exact(&mut raw)
        .map_err(|why| format!("cannot read Execution V4 {name} {physical}: {why}"))?;
    Ok(raw)
}

fn append_raw(file: &mut File, raw: &[u8]) -> Result<(), ExecutionV4Refusal> {
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(raw))
        .map_err(|why| format!("cannot append Execution V4 fixed record: {why}"))
}

fn bounded_vec<T>(count: u64, max: u64, name: &str) -> Result<Vec<T>, ExecutionV4Refusal> {
    if count > max {
        return Err(format!(
            "Execution V4 {name} read count {count} exceeds bound {max}"
        ));
    }
    let capacity = usize::try_from(count)
        .map_err(|_| format!("Execution V4 {name} count does not fit usize"))?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(capacity)
        .map_err(|why| format!("cannot reserve Execution V4 {name}: {why}"))?;
    Ok(values)
}

fn checked_sum(values: &[u64], name: &str) -> Result<u64, ExecutionV4Refusal> {
    values.iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(*value)
            .ok_or_else(|| format!("Execution V4 {name} overflowed"))
    })
}

fn validate_terminal_envelope(
    terminal: ExecutionV4FamilyTerminal,
    candidates: u64,
    evaluated: u64,
    decisions: u64,
    dispositions: u64,
) -> Result<(), ExecutionV4Refusal> {
    match terminal {
        ExecutionV4FamilyTerminal::Evaluated
            if candidates >= 2
                && evaluated == candidates
                && decisions == candidates
                && dispositions == candidates =>
        {
            Ok(())
        }
        ExecutionV4FamilyTerminal::NaturallyExtinct
            if candidates == 0 && evaluated == 0 && decisions == 0 && dispositions == 0 =>
        {
            Ok(())
        }
        ExecutionV4FamilyTerminal::Evaluated => Err(
            "Execution V4 evaluated Family does not retain at least two fully decided dispositions"
                .to_owned(),
        ),
        ExecutionV4FamilyTerminal::NaturallyExtinct => Err(
            "Execution V4 naturally extinct Family invents Candidate, decision or disposition evidence"
                .to_owned(),
        ),
    }
}

fn increment(value: &mut u64, name: &str) -> Result<(), ExecutionV4Refusal> {
    *value = value
        .checked_add(1)
        .ok_or_else(|| format!("Execution V4 {name} overflowed"))?;
    Ok(())
}

fn checked_end(first: u64, count: u64, name: &str) -> Result<u64, ExecutionV4Refusal> {
    first
        .checked_add(count)
        .ok_or_else(|| format!("Execution V4 {name} overflowed"))
}

fn usize_to_u64(value: usize, name: &str) -> Result<u64, ExecutionV4Refusal> {
    u64::try_from(value).map_err(|_| format!("Execution V4 {name} does not fit u64"))
}

fn require_nonzero(name: &str, value: [u8; 32]) -> Result<(), ExecutionV4Refusal> {
    if value == [0; 32] {
        Err(format!("Execution V4 {name} is zero"))
    } else {
        Ok(())
    }
}

fn encode_optional_u32(value: Option<u32>) -> u32 {
    value.unwrap_or(OPTIONAL_U32_NONE)
}

fn decode_optional_u32(value: u32) -> Option<u32> {
    (value != OPTIONAL_U32_NONE).then_some(value)
}

fn decode_family(value: u8) -> Result<ExecutionV4Family, ExecutionV4Refusal> {
    match value {
        1 => Ok(ExecutionV4Family::Nifty),
        2 => Ok(ExecutionV4Family::BankNifty),
        _ => Err(format!("Execution V4 family tag {value} is unknown")),
    }
}

fn decode_family_terminal(value: u8) -> Result<ExecutionV4FamilyTerminal, ExecutionV4Refusal> {
    match value {
        1 => Ok(ExecutionV4FamilyTerminal::Evaluated),
        2 => Ok(ExecutionV4FamilyTerminal::NaturallyExtinct),
        _ => Err(format!(
            "Execution V4 Family terminal tag {value} is unknown or production-unreachable"
        )),
    }
}

fn decode_direction(value: u8) -> Result<ExecutionV4Direction, ExecutionV4Refusal> {
    match value {
        1 => Ok(ExecutionV4Direction::Long),
        2 => Ok(ExecutionV4Direction::Short),
        _ => Err(format!("Execution V4 direction tag {value} is unknown")),
    }
}

fn decode_admission_status(value: u8) -> Result<ExecutionV4AdmissionStatus, ExecutionV4Refusal> {
    match value {
        1 => Ok(ExecutionV4AdmissionStatus::Admitted),
        2 => Ok(ExecutionV4AdmissionStatus::Rejected),
        3 => Ok(ExecutionV4AdmissionStatus::Unmeasured),
        4 => Ok(ExecutionV4AdmissionStatus::Refused),
        _ => Err(format!(
            "Execution V4 admission status tag {value} is unknown"
        )),
    }
}

fn decode_terminal(value: u8) -> Result<ExecutionV4Terminal, ExecutionV4Refusal> {
    match value {
        1 => Ok(ExecutionV4Terminal::Authorized),
        2 => Ok(ExecutionV4Terminal::PolicyRefused),
        _ => Err(format!("Execution V4 terminal tag {value} is unknown")),
    }
}

fn decode_percentile_axis(value: u8) -> Result<ExecutionV4PercentileAxis, ExecutionV4Refusal> {
    match value {
        1 => Ok(ExecutionV4PercentileAxis::Stop),
        2 => Ok(ExecutionV4PercentileAxis::Target),
        3 => Ok(ExecutionV4PercentileAxis::Trail),
        _ => Err(format!(
            "Execution V4 percentile axis tag {value} is unknown"
        )),
    }
}

fn require_header(
    name: &str,
    reader: &mut FixedReader<'_>,
    magic: [u8; 16],
    domain: u32,
) -> Result<(), ExecutionV4Refusal> {
    if reader.array::<16>()? != magic {
        return Err(format!("Execution V4 {name} magic mismatch"));
    }
    let observed_version = reader.u32()?;
    let observed_domain = reader.u32()?;
    if observed_version != VERSION || observed_domain != domain {
        return Err(format!(
            "Execution V4 {name} version/domain {observed_version}/{observed_domain} is unsupported"
        ));
    }
    Ok(())
}

fn require_seal(
    name: &str,
    domain: &[u8],
    payload: &[u8],
    seal: &[u8],
) -> Result<(), ExecutionV4Refusal> {
    if seal != hash_parts(domain, &[payload]) {
        return Err(format!("Execution V4 {name} seal mismatch"));
    }
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

fn combine_lock_result<T>(
    result: Result<T, ExecutionV4Refusal>,
    released: Result<(), ExecutionV4Refusal>,
) -> Result<T, ExecutionV4Refusal> {
    match (result, released) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(why), _) | (Ok(_), Err(why)) => Err(why),
    }
}

fn hex32(value: [u8; 32]) -> String {
    let mut output = String::with_capacity(64);
    for byte in value {
        output.push(hex_digit(byte >> 4));
        output.push(hex_digit(byte & 0x0f));
    }
    output
}

fn hex_digit(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0' + nibble),
        10..=15 => char::from(b'a' + nibble - 10),
        _ => '?',
    }
}

struct FixedWriter<'a> {
    bytes: &'a mut [u8],
    cursor: usize,
}

impl<'a> FixedWriter<'a> {
    const fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn array<const N: usize>(&mut self, value: &[u8; N]) -> Result<(), ExecutionV4Refusal> {
        self.bytes(value)
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), ExecutionV4Refusal> {
        let end = self
            .cursor
            .checked_add(value.len())
            .ok_or_else(|| "Execution V4 writer offset overflowed".to_owned())?;
        let target = self
            .bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "Execution V4 fixed writer exceeded record".to_owned())?;
        target.copy_from_slice(value);
        self.cursor = end;
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), ExecutionV4Refusal> {
        self.bytes(&[value])
    }

    fn u32(&mut self, value: u32) -> Result<(), ExecutionV4Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), ExecutionV4Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), ExecutionV4Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), ExecutionV4Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "Execution V4 zero-fill offset overflowed".to_owned())?;
        let target = self
            .bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "Execution V4 zero-fill exceeded record".to_owned())?;
        target.fill(0);
        self.cursor = end;
        Ok(())
    }

    const fn remaining(&self) -> usize {
        self.bytes.len() - self.cursor
    }

    fn require_full(&self, name: &str) -> Result<(), ExecutionV4Refusal> {
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err(format!(
                "Execution V4 {name} wrote {} of {} bytes",
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

    fn array<const N: usize>(&mut self) -> Result<[u8; N], ExecutionV4Refusal> {
        let end = self
            .cursor
            .checked_add(N)
            .ok_or_else(|| "Execution V4 reader offset overflowed".to_owned())?;
        let source = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| "Execution V4 fixed reader exceeded record".to_owned())?;
        let mut value = [0_u8; N];
        value.copy_from_slice(source);
        self.cursor = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, ExecutionV4Refusal> {
        let [value] = self.array::<1>()?;
        Ok(value)
    }

    fn u32(&mut self) -> Result<u32, ExecutionV4Refusal> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, ExecutionV4Refusal> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn i64(&mut self) -> Result<i64, ExecutionV4Refusal> {
        Ok(i64::from_le_bytes(self.array()?))
    }

    fn require_zeros(&mut self, count: usize, name: &str) -> Result<(), ExecutionV4Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "Execution V4 reserve offset overflowed".to_owned())?;
        let source = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| "Execution V4 reserve exceeded record".to_owned())?;
        if source.iter().any(|byte| *byte != 0) {
            return Err(format!("Execution V4 {name} is nonzero"));
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
    clippy::unwrap_used,
    clippy::panic,
    reason = "private fixed-record tests fail fixture setup loudly and intentionally inspect exact canonical slots"
)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    use super::*;

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(1);

    struct TestRoot {
        path: PathBuf,
    }

    impl TestRoot {
        fn new(label: &str) -> Self {
            let nonce = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "brutex-execution-v4-{label}-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir(&path).expect("create isolated Execution V4 root");
            Self { path }
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn digest(seed: u64) -> [u8; 32] {
        hash_parts(b"execution-v4-private-test\0", &[&seed.to_le_bytes()])
    }

    fn bounds() -> ExecutionV4Bounds {
        let parameters = ExecutionV4FileBound::new(
            40,
            40 * EXECUTION_V4_PARAMETER_BYTES as u64,
            EXECUTION_V4_PARAMETER_BYTES,
            "parameter",
        )
        .expect("parameter bounds");
        let percentiles = ExecutionV4FileBound::new(
            200,
            200 * EXECUTION_V4_PERCENTILE_BYTES as u64,
            EXECUTION_V4_PERCENTILE_BYTES,
            "percentile",
        )
        .expect("percentile bounds");
        let dispositions = ExecutionV4FileBound::new(
            200,
            200 * EXECUTION_V4_DISPOSITION_BYTES as u64,
            EXECUTION_V4_DISPOSITION_BYTES,
            "disposition",
        )
        .expect("disposition bounds");
        let completions = ExecutionV4FileBound::new(
            10,
            10 * EXECUTION_V4_COMPLETION_BYTES as u64,
            EXECUTION_V4_COMPLETION_BYTES,
            "Completion",
        )
        .expect("Completion bounds");
        ExecutionV4Bounds::new(parameters, percentiles, dispositions, completions, 20, 40)
            .expect("Execution V4 bounds")
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the fixed-record fixture constructs all four parameter identities and the complete admission/execution matrix explicitly"
    )]
    fn prepared(seed: u64) -> PreparedExecutionV4 {
        let population_id = digest(seed + 1);
        let population_completion_id = digest(seed + 2);
        let population_ordered_digest = digest(seed + 3);
        let source_finalization_id = digest(seed + 4);
        let source_finalization_completion_id = digest(seed + 5);
        let source_admission_block_id = digest(seed + 6);
        let source_admission_completion_id = digest(seed + 7);
        let nifty_population_family_row_id = digest(seed + 8);
        let banknifty_population_family_row_id = digest(seed + 9);
        let nifty_finalization_family_row_id = digest(seed + 10);
        let banknifty_finalization_family_row_id = digest(seed + 11);
        let order = [
            (ExecutionV4Family::Nifty, ExecutionV4Direction::Long),
            (ExecutionV4Family::Nifty, ExecutionV4Direction::Short),
            (ExecutionV4Family::BankNifty, ExecutionV4Direction::Long),
            (ExecutionV4Family::BankNifty, ExecutionV4Direction::Short),
        ];
        let mut parameters = Vec::new();
        let mut percentiles = Vec::new();
        for (index, (family, direction)) in order.into_iter().enumerate() {
            let index_u64 = u64::try_from(index).expect("small parameter index");
            let mut parameter = ExecutionV4ParameterRecord {
                parameter_core_id: [0; 32],
                parameter_id: [0; 32],
                population_id,
                population_completion_id,
                population_ordered_digest,
                source_finalization_id,
                source_finalization_completion_id,
                source_admission_block_id,
                source_admission_completion_id,
                policy_digest: digest(seed + 100 + index_u64),
                resolution_digest: digest(seed + 110 + index_u64),
                training_digest: digest(seed + 120 + index_u64),
                instrument_digest: digest(seed + 130 + index_u64),
                feed_digest: digest(seed + 140 + index_u64),
                commit_digest: digest(seed + 150),
                calendar_digest: digest(seed + 160),
                percentile_digest: [0; 32],
                cost_model_id: digest(seed + 170),
                execution_law_digest: execution_law_digest(),
                evaluation_fingerprint: [u8::try_from(index + 1).expect("fingerprint byte");
                    EVALUATION_FINGERPRINT_BYTES],
                family,
                direction,
                range_policy_tag: 1,
                selector_policy_tag: 1,
                forced_stop_policy_tag: FORCED_STOP_INCLUDE_TAG,
                rung: 8,
                horizon_bars: 16,
                execution_resolution_seconds: EXECUTION_RESOLUTION_SECONDS,
                entry_delay_minutes: ENTRY_DELAY_MINUTES,
                forced_exit_ist_minute: FORCED_EXIT_IST_MINUTE,
                forced_stop_index: Some(0),
                run_params: [1, 2, 3, 4],
                max_levels: 8,
                ratio_min_hundredths: 100,
                ratio_max_hundredths: 1_000,
                max_ratio_pairs: 64,
                policy_max_cells: 64,
                resolved_stop_count: 2,
                resolved_target_count: 2,
                resolved_trail_count: 2,
                resolved_ratio_pair_count: 4,
                resolved_cell_count: 54,
                forced_stop_ppm: 500,
                max_ambiguity_bars: 2,
                max_gap_bars: 2,
                training_bars: 1_000,
                training_first_ts_micros: 1_000,
                training_last_ts_micros: 2_000,
                percentile_offset: u64::try_from(percentiles.len())
                    .expect("small percentile offset"),
                percentile_count: 6,
            };
            parameter.parameter_core_id = parameter.derive_core_id();
            let segment = [
                ExecutionV4PercentileRecord {
                    parameter_core_id: parameter.parameter_core_id,
                    axis: ExecutionV4PercentileAxis::Stop,
                    ordinal: 0,
                    numerator: 1,
                    denominator: 10,
                },
                ExecutionV4PercentileRecord {
                    parameter_core_id: parameter.parameter_core_id,
                    axis: ExecutionV4PercentileAxis::Stop,
                    ordinal: 1,
                    numerator: 2,
                    denominator: 10,
                },
                ExecutionV4PercentileRecord {
                    parameter_core_id: parameter.parameter_core_id,
                    axis: ExecutionV4PercentileAxis::Target,
                    ordinal: 0,
                    numerator: 3,
                    denominator: 10,
                },
                ExecutionV4PercentileRecord {
                    parameter_core_id: parameter.parameter_core_id,
                    axis: ExecutionV4PercentileAxis::Target,
                    ordinal: 1,
                    numerator: 4,
                    denominator: 10,
                },
                ExecutionV4PercentileRecord {
                    parameter_core_id: parameter.parameter_core_id,
                    axis: ExecutionV4PercentileAxis::Trail,
                    ordinal: 0,
                    numerator: 5,
                    denominator: 10,
                },
                ExecutionV4PercentileRecord {
                    parameter_core_id: parameter.parameter_core_id,
                    axis: ExecutionV4PercentileAxis::Trail,
                    ordinal: 1,
                    numerator: 6,
                    denominator: 10,
                },
            ];
            parameter.percentile_digest =
                ordered_percentile_digest(&segment).expect("percentile digest");
            parameter.parameter_id = parameter.derive_parameter_id();
            parameters.push(parameter);
            percentiles.extend(segment);
        }
        let statuses = [
            ExecutionV4AdmissionStatus::Admitted,
            ExecutionV4AdmissionStatus::Rejected,
            ExecutionV4AdmissionStatus::Unmeasured,
            ExecutionV4AdmissionStatus::Refused,
        ];
        let terminals = [
            ExecutionV4Terminal::Authorized,
            ExecutionV4Terminal::PolicyRefused,
        ];
        let mut dispositions = Vec::new();
        for family_index in 0..2_usize {
            let family = if family_index == 0 {
                ExecutionV4Family::Nifty
            } else {
                ExecutionV4Family::BankNifty
            };
            for (status_index, status) in statuses.into_iter().enumerate() {
                for (terminal_index, terminal) in terminals.into_iter().enumerate() {
                    let family_sequence = status_index * terminals.len() + terminal_index;
                    let global_sequence =
                        family_index * statuses.len() * terminals.len() + family_sequence;
                    let direction = if terminal == ExecutionV4Terminal::Authorized {
                        ExecutionV4Direction::Long
                    } else {
                        ExecutionV4Direction::Short
                    };
                    let parameter_index =
                        family_index * 2 + usize::from(direction == ExecutionV4Direction::Short);
                    let parameter = parameters
                        .get(parameter_index)
                        .expect("family/direction fixture parameter exists");
                    let mut disposition = ExecutionV4DispositionRecord {
                        disposition_id: [0; 32],
                        population_id,
                        population_completion_id,
                        population_family_row_id: if family == ExecutionV4Family::Nifty {
                            nifty_population_family_row_id
                        } else {
                            banknifty_population_family_row_id
                        },
                        population_row_id: digest(seed + 1_000 + global_sequence as u64),
                        candidate_semantic_id: digest(seed + 2_000 + global_sequence as u64),
                        candidate_base_row_id: digest(seed + 3_000 + global_sequence as u64),
                        base_evidence_id: digest(seed + 3_500 + global_sequence as u64),
                        admission_decision_id: digest(seed + 4_000 + global_sequence as u64),
                        finalization_family_row_id: if family == ExecutionV4Family::Nifty {
                            nifty_finalization_family_row_id
                        } else {
                            banknifty_finalization_family_row_id
                        },
                        finalization_row_id: digest(seed + 5_000 + global_sequence as u64),
                        finalization_completion_id: source_finalization_completion_id,
                        parameter_id: parameter.parameter_id,
                        execution_run_id: digest(seed + 6_000 + global_sequence as u64),
                        evaluated_grid_digest: digest(seed + 7_000 + global_sequence as u64),
                        resolution_digest: parameter.resolution_digest,
                        column_digest: digest(seed + 8_000 + global_sequence as u64),
                        context_digest: digest(seed + 9_000 + global_sequence as u64),
                        runner_disposition_digest: digest(seed + 10_000 + global_sequence as u64),
                        selected_exit_digest: if terminal == ExecutionV4Terminal::Authorized {
                            digest(seed + 11_000 + global_sequence as u64)
                        } else {
                            [0; 32]
                        },
                        global_sequence: u64::try_from(global_sequence)
                            .expect("small global ordinal"),
                        family_sequence: u64::try_from(family_sequence)
                            .expect("small family ordinal"),
                        cell_ordinal: u64::try_from(global_sequence).expect("small cell ordinal"),
                        support_hits: 20,
                        refusal_bits: if terminal == ExecutionV4Terminal::Authorized {
                            0
                        } else {
                            1_u64 << status_index
                        },
                        family,
                        direction,
                        admission_status: status,
                        terminal,
                        rung: parameter.rung,
                        horizon_bars: parameter.horizon_bars,
                        stop_index: Some(0),
                        target_index: Some(1),
                        tsl_index: Some(1),
                        ttp_arm_index: Some(0),
                        ttp_trail_index: Some(0),
                    };
                    disposition.disposition_id = disposition.derive_id();
                    dispositions.push(disposition);
                }
            }
        }
        let prepared = PreparedExecutionV4 {
            population_id,
            population_completion_id,
            population_ordered_digest,
            source_finalization_id,
            source_finalization_completion_id,
            source_admission_block_id,
            source_admission_completion_id,
            families: [
                ExecutionV4FamilyEnvelope {
                    family: ExecutionV4Family::Nifty,
                    terminal: ExecutionV4FamilyTerminal::Evaluated,
                    population_family_row_id: nifty_population_family_row_id,
                    finalization_family_row_id: nifty_finalization_family_row_id,
                    candidate_count: 8,
                    evaluated_count: 8,
                    decision_count: 8,
                },
                ExecutionV4FamilyEnvelope {
                    family: ExecutionV4Family::BankNifty,
                    terminal: ExecutionV4FamilyTerminal::Evaluated,
                    population_family_row_id: banknifty_population_family_row_id,
                    finalization_family_row_id: banknifty_finalization_family_row_id,
                    candidate_count: 8,
                    evaluated_count: 8,
                    decision_count: 8,
                },
            ],
            rung: 8,
            horizon_bars: 16,
            parameters,
            percentiles,
            dispositions,
        };
        prepared.validate(bounds()).expect("valid synthetic block");
        prepared
    }

    fn rebind_parameter_and_percentiles(
        mut parameter: ExecutionV4ParameterRecord,
        percentiles: &[ExecutionV4PercentileRecord],
    ) -> (ExecutionV4ParameterRecord, Vec<ExecutionV4PercentileRecord>) {
        parameter.percentile_count =
            u64::try_from(percentiles.len()).expect("fixture percentile count fits u64");
        parameter.parameter_core_id = parameter.derive_core_id();
        let mut rebound = percentiles.to_vec();
        for percentile in &mut rebound {
            percentile.parameter_core_id = parameter.parameter_core_id;
        }
        parameter.percentile_digest =
            ordered_percentile_digest(&rebound).expect("rebound percentile digest");
        parameter.parameter_id = parameter.derive_parameter_id();
        (parameter, rebound)
    }

    fn reidentify_parameter(
        mut parameter: ExecutionV4ParameterRecord,
    ) -> ExecutionV4ParameterRecord {
        parameter.parameter_core_id = parameter.derive_core_id();
        parameter.parameter_id = parameter.derive_parameter_id();
        parameter
    }

    fn rebind_disposition(
        mut row: ExecutionV4DispositionRecord,
        parameter: &ExecutionV4ParameterRecord,
    ) -> ExecutionV4DispositionRecord {
        row.parameter_id = parameter.parameter_id;
        row.resolution_digest = parameter.resolution_digest;
        row.rung = parameter.rung;
        row.horizon_bars = parameter.horizon_bars;
        row.disposition_id = row.derive_id();
        row
    }

    fn with_family_topology(
        mut prepared: PreparedExecutionV4,
        evaluated: [bool; 2],
    ) -> PreparedExecutionV4 {
        let original_parameters = std::mem::take(&mut prepared.parameters);
        let original_percentiles = std::mem::take(&mut prepared.percentiles);
        for mut parameter in original_parameters {
            if !evaluated[family_index(parameter.family)] {
                continue;
            }
            let start = usize::try_from(parameter.percentile_offset)
                .expect("fixture percentile offset fits usize");
            let end = usize::try_from(
                parameter
                    .percentile_offset
                    .checked_add(parameter.percentile_count)
                    .expect("fixture percentile range"),
            )
            .expect("fixture percentile end fits usize");
            let segment = original_percentiles
                .get(start..end)
                .expect("fixture parameter percentile segment");
            parameter.percentile_offset =
                u64::try_from(prepared.percentiles.len()).expect("small rebound offset");
            let (parameter, mut segment) = rebind_parameter_and_percentiles(parameter, segment);
            prepared.parameters.push(parameter);
            prepared.percentiles.append(&mut segment);
        }
        prepared
            .dispositions
            .retain(|row| evaluated[family_index(row.family)]);
        for (index, row) in prepared.dispositions.iter_mut().enumerate() {
            let parameter = find_parameter(&prepared.parameters, row.family, row.direction)
                .expect("active topology parameter");
            row.parameter_id = parameter.parameter_id;
            row.resolution_digest = parameter.resolution_digest;
            row.global_sequence = u64::try_from(index).expect("small topology ordinal");
            row.disposition_id = row.derive_id();
        }
        let family_counts = [
            prepared
                .dispositions
                .iter()
                .filter(|row| row.family == ExecutionV4Family::Nifty)
                .count(),
            prepared
                .dispositions
                .iter()
                .filter(|row| row.family == ExecutionV4Family::BankNifty)
                .count(),
        ];
        for (index, family) in prepared.families.iter_mut().enumerate() {
            if evaluated[index] {
                let count =
                    u64::try_from(family_counts[index]).expect("small topology Family count");
                family.terminal = ExecutionV4FamilyTerminal::Evaluated;
                family.candidate_count = count;
                family.evaluated_count = count;
                family.decision_count = count;
            } else {
                family.terminal = ExecutionV4FamilyTerminal::NaturallyExtinct;
                family.candidate_count = 0;
                family.evaluated_count = 0;
                family.decision_count = 0;
            }
        }
        prepared
            .validate(bounds())
            .expect("terminal topology is internally consistent");
        prepared
    }

    fn append_exact_prefix(
        root: &Path,
        prepared: &PreparedExecutionV4,
        parameter_count: usize,
        percentile_count: usize,
        disposition_count: usize,
    ) {
        drop(ExecutionV4Ledger::open_write(root, bounds()).expect("create V4 files"));
        append_records(
            &root.join(PARAMETER_FILE),
            prepared
                .parameters
                .get(..parameter_count)
                .expect("parameter prefix is in fixture bounds")
                .iter()
                .map(|record| record.encode().expect("parameter encode")),
        );
        append_records(
            &root.join(PERCENTILE_FILE),
            prepared
                .percentiles
                .get(..percentile_count)
                .expect("percentile prefix is in fixture bounds")
                .iter()
                .map(|record| record.encode().expect("percentile encode")),
        );
        append_records(
            &root.join(DISPOSITION_FILE),
            prepared
                .dispositions
                .get(..disposition_count)
                .expect("disposition prefix is in fixture bounds")
                .iter()
                .map(|record| record.encode().expect("disposition encode")),
        );
    }

    fn append_records<const N: usize>(path: &Path, records: impl IntoIterator<Item = [u8; N]>) {
        let mut file = OpenOptions::new()
            .append(true)
            .open(path)
            .expect("open test append file");
        for raw in records {
            file.write_all(&raw).expect("append test record");
        }
        file.sync_data().expect("sync test records");
    }

    fn reseal<const N: usize>(raw: &mut [u8; N], payload_bytes: usize, domain: &[u8]) {
        let (payload, seal) = raw.split_at_mut(payload_bytes);
        let digest = hash_parts(domain, &[&*payload]);
        seal.copy_from_slice(&digest);
    }

    #[test]
    fn fixed_codecs_are_canonical_and_fail_closed() {
        let prepared = prepared(10);
        let parameter = prepared.parameters.first().expect("first parameter");
        let parameter_raw = parameter.encode().expect("parameter encode");
        assert_eq!(
            ExecutionV4ParameterRecord::decode(&parameter_raw).expect("parameter decode"),
            *parameter
        );
        let percentile = prepared.percentiles.first().expect("first percentile");
        let percentile_raw = percentile.encode().expect("percentile encode");
        assert_eq!(
            ExecutionV4PercentileRecord::decode(&percentile_raw).expect("percentile decode"),
            *percentile
        );
        let disposition = prepared.dispositions.first().expect("first disposition");
        let disposition_raw = disposition.encode().expect("disposition encode");
        assert_eq!(
            ExecutionV4DispositionRecord::decode(&disposition_raw).expect("disposition decode"),
            *disposition
        );
        let completion = prepared
            .expected_completion(0, 0, 0, 0)
            .expect("Completion");
        let completion_raw = completion.encode().expect("Completion encode");
        assert_eq!(
            ExecutionV4CompletionRecord::decode(&completion_raw).expect("Completion decode"),
            completion
        );

        let mut broken_seal = parameter_raw;
        *broken_seal.first_mut().expect("nonempty parameter record") ^= 1;
        assert!(ExecutionV4ParameterRecord::decode(&broken_seal).is_err());

        assert!(decode_family(9).is_err());
        assert!(decode_family_terminal(3).is_err());

        let mut nonzero_reserve = completion_raw;
        *nonzero_reserve
            .get_mut(COMPLETION_PAYLOAD_BYTES - 1)
            .expect("Completion reserve is inside fixed record") = 1;
        reseal(
            &mut nonzero_reserve,
            COMPLETION_PAYLOAD_BYTES,
            COMPLETION_SEAL_DOMAIN,
        );
        assert!(ExecutionV4CompletionRecord::decode(&nonzero_reserve).is_err());

        let mut half_ttp = disposition.clone();
        half_ttp.ttp_trail_index = None;
        assert!(half_ttp.validate().is_err());

        let mut old_layout = parameter_raw;
        old_layout
            .get_mut(16..20)
            .expect("version slot is inside fixed record")
            .copy_from_slice(&3_u32.to_le_bytes());
        reseal(
            &mut old_layout,
            PARAMETER_PAYLOAD_BYTES,
            PARAMETER_SEAL_DOMAIN,
        );
        assert!(ExecutionV4ParameterRecord::decode(&old_layout).is_err());

        let mut wrong_close = parameter.clone();
        wrong_close.forced_exit_ist_minute = 909;
        wrong_close.parameter_core_id = wrong_close.derive_core_id();
        wrong_close.parameter_id = wrong_close.derive_parameter_id();
        assert!(wrong_close.validate().is_err());
    }

    #[test]
    fn admission_and_execution_are_independent_and_refusals_keep_run_provenance() {
        let prepared = prepared(11);
        for family in [ExecutionV4Family::Nifty, ExecutionV4Family::BankNifty] {
            for status in [
                ExecutionV4AdmissionStatus::Admitted,
                ExecutionV4AdmissionStatus::Rejected,
                ExecutionV4AdmissionStatus::Unmeasured,
                ExecutionV4AdmissionStatus::Refused,
            ] {
                for terminal in [
                    ExecutionV4Terminal::Authorized,
                    ExecutionV4Terminal::PolicyRefused,
                ] {
                    assert!(prepared.dispositions.iter().any(|row| {
                        row.family == family
                            && row.admission_status == status
                            && row.terminal == terminal
                    }));
                }
            }
        }

        let rejected_authorized = prepared
            .dispositions
            .iter()
            .find(|row| {
                row.admission_status == ExecutionV4AdmissionStatus::Rejected
                    && row.terminal == ExecutionV4Terminal::Authorized
            })
            .expect("Rejected and Authorized fixture cell");
        assert_ne!(rejected_authorized.execution_run_id, [0; 32]);
        let admitted_refused = prepared
            .dispositions
            .iter()
            .find(|row| {
                row.admission_status == ExecutionV4AdmissionStatus::Admitted
                    && row.terminal == ExecutionV4Terminal::PolicyRefused
            })
            .expect("Admitted and PolicyRefused fixture cell");
        assert_ne!(admitted_refused.execution_run_id, [0; 32]);

        let mut missing_run = admitted_refused.clone();
        missing_run.execution_run_id = [0; 32];
        missing_run.disposition_id = missing_run.derive_id();
        assert!(missing_run.validate().is_err());

        let mut unknown_refusal = admitted_refused.clone();
        unknown_refusal.refusal_bits = 1_u64 << 63;
        unknown_refusal.disposition_id = unknown_refusal.derive_id();
        assert!(unknown_refusal.validate().is_err());

        let mut invented_refusal = rejected_authorized.clone();
        invented_refusal.refusal_bits =
            runner::exit_grid_policy::ExecutionRefusalBitsV1::MISSING_STOP.bits();
        invented_refusal.disposition_id = invented_refusal.derive_id();
        assert!(invented_refusal.validate().is_err());
    }

    #[test]
    fn three_schedules_and_resolved_axes_bound_all_five_coordinates_and_cell() {
        let prepared = prepared(12);
        let parameter = prepared.parameters.first().expect("first parameter");
        let segment = prepared
            .percentiles
            .get(..usize::try_from(parameter.percentile_count).expect("fixture count"))
            .expect("first parameter percentile schedule");
        assert_eq!(
            validate_one_percentile_prefix(parameter, segment).expect("three schedules"),
            [2, 2, 2]
        );

        let mut without_trail = parameter.clone();
        let stop_and_target = segment.get(..4).expect("Stop and Target fixture schedules");
        without_trail.percentile_count = 4;
        without_trail.percentile_digest =
            ordered_percentile_digest(stop_and_target).expect("short schedule digest");
        assert!(validate_percentile_segment(&without_trail, stop_and_target).is_err());

        let mut ordinal_gap = segment.to_vec();
        ordinal_gap
            .get_mut(1)
            .expect("second Stop percentile")
            .ordinal = 2;
        assert!(validate_one_percentile_prefix(parameter, &ordinal_gap).is_err());

        let mut nonincreasing = segment.to_vec();
        nonincreasing
            .get_mut(1)
            .expect("second Stop percentile")
            .numerator = 1;
        assert!(validate_one_percentile_prefix(parameter, &nonincreasing).is_err());

        for coordinate in 0..5 {
            let mut out_of_range = prepared.clone();
            let row = out_of_range
                .dispositions
                .first_mut()
                .expect("first disposition");
            match coordinate {
                0 => row.stop_index = Some(2),
                1 => row.target_index = Some(2),
                2 => row.tsl_index = Some(2),
                3 => row.ttp_arm_index = Some(2),
                4 => row.ttp_trail_index = Some(2),
                _ => unreachable!("five fixed coordinate cases"),
            }
            row.disposition_id = row.derive_id();
            assert!(out_of_range.validate(bounds()).is_err());
        }

        let mut cell_out_of_range = prepared.clone();
        let row = cell_out_of_range
            .dispositions
            .first_mut()
            .expect("first disposition");
        row.cell_ordinal = 54;
        row.disposition_id = row.derive_id();
        assert!(cell_out_of_range.validate(bounds()).is_err());
    }

    #[test]
    fn forced_stop_states_and_run_params_are_canonical_and_identity_bound() {
        let prepared = prepared(121);
        let base = prepared
            .parameters
            .first()
            .expect("first parameter")
            .clone();
        assert_eq!(
            execution_law_digest(),
            crate::execution_capability::exact_execution_law_digest_v1()
        );

        let mut disabled = base.clone();
        disabled.forced_stop_policy_tag = FORCED_STOP_DISABLED_TAG;
        disabled.forced_stop_ppm = 0;
        disabled.forced_stop_index = None;
        let disabled = reidentify_parameter(disabled);
        disabled.validate().expect("canonical Disabled forced stop");

        for (tag, ppm, index) in [
            (FORCED_STOP_DISABLED_TAG, 1, None),
            (FORCED_STOP_DISABLED_TAG, 0, Some(0)),
            (FORCED_STOP_INCLUDE_TAG, 0, Some(0)),
            (FORCED_STOP_INCLUDE_TAG, 1, None),
            (FORCED_STOP_REQUIRE_TAG, -1, Some(0)),
            (FORCED_STOP_REQUIRE_TAG, 1, Some(OPTIONAL_U32_NONE)),
            (FORCED_STOP_REQUIRE_TAG, 1, Some(2)),
            (3, 1, Some(0)),
        ] {
            let mut invalid = base.clone();
            invalid.forced_stop_policy_tag = tag;
            invalid.forced_stop_ppm = ppm;
            invalid.forced_stop_index = index;
            let invalid = reidentify_parameter(invalid);
            assert!(invalid.validate().is_err());
        }

        for tag in [FORCED_STOP_INCLUDE_TAG, FORCED_STOP_REQUIRE_TAG] {
            let mut valid = base.clone();
            valid.forced_stop_policy_tag = tag;
            valid.forced_stop_ppm = 1;
            valid.forced_stop_index = Some(1);
            let valid = reidentify_parameter(valid);
            valid.validate().expect("canonical enabled forced stop");
        }

        let mut zero_policy = base.clone();
        zero_policy.run_params = [1, 2, 3, 0];
        let zero_policy = reidentify_parameter(zero_policy);
        zero_policy
            .validate()
            .expect("Runner Params policy zero remains authoritative");
        for run_params in [[0, 2, 3, 4], [1, 0, 3, 4], [1, 2, 0, 4]] {
            let mut missing_runner_bound = base.clone();
            missing_runner_bound.run_params = run_params;
            let missing_runner_bound = reidentify_parameter(missing_runner_bound);
            assert!(missing_runner_bound.validate().is_err());
        }

        let original_id = base.parameter_id;
        let mut changed_ratio = base;
        changed_ratio.ratio_max_hundredths += 1;
        let changed_ratio = reidentify_parameter(changed_ratio);
        changed_ratio
            .validate()
            .expect("exact ratio bound is valid");
        assert_ne!(changed_ratio.parameter_id, original_id);
    }

    #[test]
    fn forced_stop_growth_uses_resolved_axis_and_require_matches_exact_index() {
        let prepared = prepared(122);
        let template = prepared
            .parameters
            .first()
            .expect("first parameter")
            .clone();
        let segment = prepared
            .percentiles
            .get(..usize::try_from(template.percentile_count).expect("fixture count"))
            .expect("first percentile segment");
        let mut grown = template;
        grown.max_levels = 2;
        grown.resolved_stop_count = 3;
        grown.forced_stop_policy_tag = FORCED_STOP_INCLUDE_TAG;
        grown.forced_stop_ppm = 250;
        grown.forced_stop_index = Some(2);
        let (grown, grown_segment) = rebind_parameter_and_percentiles(grown, segment);
        grown.validate().expect("one exact forced-stop insertion");
        validate_percentile_segment(&grown, &grown_segment)
            .expect("resolved Stop axis may grow beyond the two policy atoms");

        let mut selected = prepared
            .dispositions
            .first()
            .expect("first disposition")
            .clone();
        selected.stop_index = Some(2);
        selected = rebind_disposition(selected, &grown);
        selected
            .validate()
            .expect("grown Stop coordinate is canonical");
        require_disposition_parameter_join(&selected, &grown)
            .expect("Include accepts the resolved forced-stop coordinate");
        require_resolved_coordinate_bounds(&selected, &grown)
            .expect("coordinate is bounded by resolved rather than policy axis");

        let mut require = grown.clone();
        require.forced_stop_policy_tag = FORCED_STOP_REQUIRE_TAG;
        let (require, require_segment) = rebind_parameter_and_percentiles(require, &grown_segment);
        require.validate().expect("canonical Require policy");
        validate_percentile_segment(&require, &require_segment)
            .expect("Require retains the forced-stop-grown axis");

        let mut wrong_stop = selected.clone();
        wrong_stop.stop_index = Some(1);
        wrong_stop = rebind_disposition(wrong_stop, &require);
        assert!(require_disposition_parameter_join(&wrong_stop, &require).is_err());

        let exact_stop = rebind_disposition(selected, &require);
        require_disposition_parameter_join(&exact_stop, &require)
            .expect("Require accepts only the exact resolved forced-stop index");

        let mut excess_growth = require;
        excess_growth.resolved_stop_count = 4;
        let excess_growth = reidentify_parameter(excess_growth);
        assert!(excess_growth.validate().is_err());
    }

    #[test]
    fn resolved_grid_cardinalities_and_candidate_cell_boundary_fail_closed() {
        let prepared = prepared(123);
        let base = prepared
            .parameters
            .first()
            .expect("first parameter")
            .clone();

        let mut boundary = base.clone();
        boundary.policy_max_cells = boundary.resolved_cell_count;
        let boundary = reidentify_parameter(boundary);
        boundary
            .validate()
            .expect("resolved cell count may equal the policy ceiling");
        let mut last_cell = prepared
            .dispositions
            .first()
            .expect("first disposition")
            .clone();
        last_cell.cell_ordinal = boundary.resolved_cell_count - 1;
        last_cell = rebind_disposition(last_cell, &boundary);
        require_disposition_parameter_join(&last_cell, &boundary)
            .expect("last resolved Candidate cell is admitted");
        let mut past_end = last_cell;
        past_end.cell_ordinal = boundary.resolved_cell_count;
        past_end.disposition_id = past_end.derive_id();
        assert!(require_disposition_parameter_join(&past_end, &boundary).is_err());

        let mut zero_cells = base.clone();
        zero_cells.resolved_cell_count = 0;
        let zero_cells = reidentify_parameter(zero_cells);
        assert!(zero_cells.validate().is_err());
        let mut excess_cells = base.clone();
        excess_cells.resolved_cell_count = excess_cells.policy_max_cells + 1;
        let excess_cells = reidentify_parameter(excess_cells);
        assert!(excess_cells.validate().is_err());

        let mut impossible_pairs = base.clone();
        impossible_pairs.resolved_ratio_pair_count = 5;
        let impossible_pairs = reidentify_parameter(impossible_pairs);
        assert!(impossible_pairs.validate().is_err());
        let mut over_pair_budget = base.clone();
        over_pair_budget.max_ratio_pairs = 3;
        let over_pair_budget = reidentify_parameter(over_pair_budget);
        assert!(over_pair_budget.validate().is_err());

        let mut inverted_ratio = base.clone();
        inverted_ratio.ratio_min_hundredths = inverted_ratio.ratio_max_hundredths + 1;
        let inverted_ratio = reidentify_parameter(inverted_ratio);
        assert!(inverted_ratio.validate().is_err());

        let segment = prepared
            .percentiles
            .get(..usize::try_from(base.percentile_count).expect("fixture count"))
            .expect("first percentile segment");
        let mut narrower_target = base.clone();
        narrower_target.resolved_target_count = 1;
        narrower_target.resolved_ratio_pair_count = 2;
        let (narrower_target, narrower_segment) =
            rebind_parameter_and_percentiles(narrower_target, segment);
        narrower_target
            .validate()
            .expect("resolved Target axis may be smaller than policy schedule");
        validate_percentile_segment(&narrower_target, &narrower_segment)
            .expect("larger policy schedule remains separately authenticated");
        let mut target_outside_resolution = prepared
            .dispositions
            .first()
            .expect("first disposition")
            .clone();
        target_outside_resolution.target_index = Some(1);
        target_outside_resolution.ttp_arm_index = None;
        target_outside_resolution.ttp_trail_index = None;
        target_outside_resolution = rebind_disposition(target_outside_resolution, &narrower_target);
        assert!(
            require_resolved_coordinate_bounds(&target_outside_resolution, &narrower_target)
                .is_err()
        );

        let mut schedules_exceed_policy = base;
        schedules_exceed_policy.max_levels = 1;
        schedules_exceed_policy.resolved_stop_count = 1;
        schedules_exceed_policy.resolved_target_count = 1;
        schedules_exceed_policy.resolved_trail_count = 1;
        schedules_exceed_policy.resolved_ratio_pair_count = 1;
        schedules_exceed_policy.forced_stop_index = Some(0);
        let (schedules_exceed_policy, oversized_schedules) =
            rebind_parameter_and_percentiles(schedules_exceed_policy, segment);
        schedules_exceed_policy
            .validate()
            .expect("resolved axes themselves fit the policy");
        assert!(
            validate_percentile_segment(&schedules_exceed_policy, &oversized_schedules).is_err()
        );
    }

    #[test]
    fn common_rung_and_candidate_base_row_uniqueness_fail_closed() {
        let prepared = prepared(13);
        let mut mixed_parameters = prepared.parameters.clone();
        mixed_parameters.get_mut(1).expect("second parameter").rung += 1;
        assert!(
            validate_disposition_prefix(
                &mixed_parameters,
                &prepared.percentiles,
                &prepared.dispositions,
                prepared.population_id,
            )
            .is_err()
        );

        let mut duplicate_base = prepared.clone();
        let base_row_id = duplicate_base
            .dispositions
            .first()
            .expect("first disposition")
            .candidate_base_row_id;
        let duplicate = duplicate_base
            .dispositions
            .get_mut(1)
            .expect("second disposition");
        duplicate.candidate_base_row_id = base_row_id;
        duplicate.disposition_id = duplicate.derive_id();
        assert!(duplicate_base.validate(bounds()).is_err());
    }

    #[test]
    fn semantic_identities_are_relocation_stable() {
        let prepared = prepared(20);
        let first = prepared
            .expected_completion(0, 0, 0, 0)
            .expect("first Completion");
        let relocated = prepared
            .expected_completion(9, 400, 800, 1_000)
            .expect("relocated Completion");
        assert_eq!(first.completion_id, relocated.completion_id);
        assert_eq!(first.nifty_authority_id, relocated.nifty_authority_id);
        assert_eq!(
            first.banknifty_authority_id,
            relocated.banknifty_authority_id
        );
        assert_ne!(
            first.encode().expect("first bytes"),
            relocated.encode().expect("relocated bytes")
        );
    }

    #[test]
    fn receipt_last_append_fresh_reopen_and_reuse_are_exact() {
        let root = TestRoot::new("append-reuse");
        let prepared = prepared(30);
        let mut first =
            commit_prepared_for_test(&root.path, bounds(), &prepared).expect("first exact commit");
        assert!(first.was_written());
        let receipt = first.authority().structural_receipt();
        assert_eq!(receipt.population_id(), prepared.population_id);
        let rows = match &mut first {
            ExecutionV4ProductionCommit::Written(authority)
            | ExecutionV4ProductionCommit::Reused(authority) => authority
                .ordered_authenticated_dispositions()
                .expect("authenticated dispositions"),
        };
        assert_eq!(rows.len(), prepared.dispositions.len());
        let first_row = rows.first().expect("first authenticated disposition");
        assert_eq!(first_row.global_sequence(), 0);
        assert_eq!(first_row.terminal(), ExecutionV4Terminal::Authorized);
        assert_eq!(
            first_row.canonical_record(),
            &prepared
                .dispositions
                .first()
                .expect("first expected disposition")
                .encode()
                .expect("expected disposition bytes")
        );
        drop(first);

        let second =
            commit_prepared_for_test(&root.path, bounds(), &prepared).expect("exact reuse commit");
        assert!(!second.was_written());
        assert_eq!(second.authority().structural_receipt(), receipt);
        assert_eq!(
            std::fs::metadata(root.path.join(PARAMETER_FILE))
                .expect("parameter metadata")
                .len(),
            4 * EXECUTION_V4_PARAMETER_BYTES as u64
        );
        assert_eq!(
            std::fs::metadata(root.path.join(COMPLETION_FILE))
                .expect("Completion metadata")
                .len(),
            EXECUTION_V4_COMPLETION_BYTES as u64
        );
    }

    #[test]
    fn every_receipt_last_crash_prefix_resumes_only_exact_bytes() {
        let prepared = prepared(40);
        for (label, parameters, percentiles, dispositions) in [
            ("parameters", 2, 0, 0),
            ("percentiles", 4, 3, 0),
            ("dispositions", 4, 24, 4),
            ("full-orphan", 4, 24, 16),
        ] {
            let root = TestRoot::new(label);
            append_exact_prefix(&root.path, &prepared, parameters, percentiles, dispositions);
            let committed = commit_prepared_for_test(&root.path, bounds(), &prepared)
                .expect("exact crash prefix resumes");
            assert!(committed.was_written());
            assert_eq!(
                committed.authority().structural_receipt().population_id(),
                prepared.population_id
            );
        }
    }

    #[test]
    fn mixed_topology_crash_prefixes_resume_for_nifty_or_banknifty_first_parameter() {
        for (topology_label, evaluated) in [
            ("mixed-nifty", [true, false]),
            ("mixed-bank", [false, true]),
        ] {
            let prepared = with_family_topology(prepared(41), evaluated);
            for (prefix_label, parameters, percentiles, dispositions) in [
                ("parameter", 1, 0, 0),
                ("percentile", 2, 3, 0),
                ("disposition", 2, 12, 4),
                ("full-orphan", 2, 12, 8),
            ] {
                let root = TestRoot::new(&format!("{topology_label}-{prefix_label}"));
                append_exact_prefix(&root.path, &prepared, parameters, percentiles, dispositions);
                let committed = commit_prepared_for_test(&root.path, bounds(), &prepared)
                    .expect("mixed exact crash prefix resumes");
                assert!(committed.was_written());
                assert_eq!(
                    committed.authority().structural_receipt().parameter_count(),
                    2
                );
            }
        }
    }

    #[test]
    fn foreign_or_out_of_order_orphans_are_refused_without_overwrite() {
        let expected = prepared(50);
        let foreign = prepared(60);
        let foreign_root = TestRoot::new("foreign-prefix");
        append_exact_prefix(&foreign_root.path, &foreign, 1, 0, 0);
        let before = std::fs::read(foreign_root.path.join(PARAMETER_FILE))
            .expect("read foreign prefix before");
        assert!(
            commit_prepared_for_test(&foreign_root.path, bounds(), &expected).is_err(),
            "foreign valid prefix must not be overwritten"
        );
        assert_eq!(
            std::fs::read(foreign_root.path.join(PARAMETER_FILE))
                .expect("read foreign prefix after"),
            before
        );

        let out_of_order = TestRoot::new("out-of-order");
        drop(
            ExecutionV4Ledger::open_write(&out_of_order.path, bounds())
                .expect("create out-of-order files"),
        );
        append_records(
            &out_of_order.path.join(PERCENTILE_FILE),
            [expected
                .percentiles
                .first()
                .expect("first percentile")
                .encode()
                .expect("percentile encode")],
        );
        assert!(ExecutionV4Ledger::open_read(&out_of_order.path, bounds()).is_err());
    }

    #[test]
    fn retained_generation_symlink_hardlink_and_ragged_files_fail_closed() {
        let prepared = prepared(70);
        let root = TestRoot::new("generation");
        let committed = commit_prepared_for_test(&root.path, bounds(), &prepared)
            .expect("commit generation fixture");
        let receipt = committed.authority().structural_receipt();
        drop(committed);
        let retained =
            ExecutionV4Ledger::open_read(&root.path, bounds()).expect("open retained reader");
        let parameter_path = root.path.join(PARAMETER_FILE);
        let mut bytes = std::fs::read(&parameter_path).expect("read parameter file");
        *bytes.first_mut().expect("nonempty parameter file") ^= 1;
        std::fs::write(&parameter_path, bytes).expect("same-length external mutation");
        assert!(
            retained
                .structural_receipt(&receipt.population_id())
                .is_err()
        );
        drop(retained);

        let ragged = TestRoot::new("ragged");
        drop(ExecutionV4Ledger::open_write(&ragged.path, bounds()).expect("create ragged files"));
        let mut file = OpenOptions::new()
            .append(true)
            .open(ragged.path.join(DISPOSITION_FILE))
            .expect("open ragged file");
        file.write_all(&[1]).expect("append ragged byte");
        file.sync_data().expect("sync ragged byte");
        assert!(ExecutionV4Ledger::open_read(&ragged.path, bounds()).is_err());

        #[cfg(unix)]
        {
            let hardlink = TestRoot::new("hardlink");
            drop(
                ExecutionV4Ledger::open_write(&hardlink.path, bounds())
                    .expect("create hardlink files"),
            );
            std::fs::hard_link(
                hardlink.path.join(PARAMETER_FILE),
                hardlink.path.join("second-parameter-link"),
            )
            .expect("create hard link");
            assert!(ExecutionV4Ledger::open_read(&hardlink.path, bounds()).is_err());

            let target = TestRoot::new("symlink-target");
            let link_parent = TestRoot::new("symlink-parent");
            let link = link_parent.path.join("linked-root");
            symlink(&target.path, &link).expect("create root symlink");
            assert!(ExecutionV4Ledger::open_write(&link, bounds()).is_err());
        }
    }

    #[test]
    fn all_extinct_block_retains_terminals_and_nonzero_empty_authorities() {
        let root = TestRoot::new("all-extinct");
        let prepared = with_family_topology(prepared(75), [false, false]);
        let completion = prepared
            .expected_completion(0, 0, 0, 0)
            .expect("all-extinct Completion");
        assert_eq!(completion.parameter_count, 0);
        assert_eq!(completion.percentile_count, 0);
        assert_eq!(completion.disposition_count, 0);
        assert_eq!(completion.row_count, 0);
        assert_eq!(completion.evaluated_count, 0);
        assert_eq!(completion.decision_count, 0);
        assert_eq!(completion.nifty_count, 0);
        assert_eq!(completion.banknifty_count, 0);
        assert_eq!(
            completion.parameter_ids,
            [[0; 32]; MAX_PARAMETER_RECORDS_PER_BLOCK]
        );
        assert_eq!(
            completion.nifty_terminal,
            ExecutionV4FamilyTerminal::NaturallyExtinct
        );
        assert_eq!(
            completion.banknifty_terminal,
            ExecutionV4FamilyTerminal::NaturallyExtinct
        );
        assert_eq!(completion.admission_counts, [0; 4]);
        assert_eq!(completion.terminal_matrix, [0; MATRIX_CELLS]);
        assert_eq!(completion.rung, prepared.rung);
        assert_eq!(completion.horizon_bars, prepared.horizon_bars);
        let expected_parameter_empty = begin_ordered_digest(ORDERED_PARAMETERS_DOMAIN, 0)
            .expect("empty parameter digest")
            .finalize();
        assert_eq!(
            completion.ordered_parameter_digest,
            expected_parameter_empty
        );
        let expected_percentile_empty = begin_ordered_digest(ORDERED_PERCENTILES_DOMAIN, 0)
            .expect("empty percentile digest")
            .finalize();
        assert_eq!(
            completion.ordered_percentile_digest,
            expected_percentile_empty
        );
        let expected_ordered_empty = begin_ordered_digest(ORDERED_DISPOSITIONS_DOMAIN, 0)
            .expect("empty ordered digest")
            .finalize();
        assert_eq!(
            completion.ordered_disposition_digest,
            expected_ordered_empty
        );
        let mut expected_nifty_empty =
            begin_ordered_digest(ORDERED_DISPOSITIONS_DOMAIN, 0).expect("empty NIFTY digest");
        expected_nifty_empty.update(&[ExecutionV4Family::Nifty as u8]);
        assert_eq!(
            completion.nifty_disposition_digest,
            expected_nifty_empty.finalize()
        );
        let mut expected_bank_empty =
            begin_ordered_digest(ORDERED_DISPOSITIONS_DOMAIN, 0).expect("empty BANKNIFTY digest");
        expected_bank_empty.update(&[ExecutionV4Family::BankNifty as u8]);
        assert_eq!(
            completion.banknifty_disposition_digest,
            expected_bank_empty.finalize()
        );
        for digest in [
            completion.ordered_parameter_digest,
            completion.ordered_percentile_digest,
            completion.ordered_disposition_digest,
            completion.nifty_disposition_digest,
            completion.banknifty_disposition_digest,
            completion.nifty_authority_id,
            completion.banknifty_authority_id,
        ] {
            assert_ne!(digest, [0; 32]);
        }
        assert_ne!(
            completion.nifty_disposition_digest,
            completion.banknifty_disposition_digest
        );
        assert_ne!(
            completion.nifty_authority_id,
            completion.banknifty_authority_id
        );

        let mut committed = commit_prepared_for_test(&root.path, bounds(), &prepared)
            .expect("all-extinct exact commit");
        assert!(committed.was_written());
        assert_eq!(
            std::fs::metadata(root.path.join(PARAMETER_FILE))
                .expect("all-extinct parameter metadata")
                .len(),
            0
        );
        assert_eq!(
            std::fs::metadata(root.path.join(PERCENTILE_FILE))
                .expect("all-extinct percentile metadata")
                .len(),
            0
        );
        assert_eq!(
            std::fs::metadata(root.path.join(DISPOSITION_FILE))
                .expect("all-extinct disposition metadata")
                .len(),
            0
        );
        assert!(
            committed
                .authority_mut()
                .ordered_authenticated_dispositions()
                .expect("all-extinct durable disposition block")
                .is_empty()
        );
        let receipt = committed.authority().structural_receipt();
        assert_eq!(receipt.disposition_count(), 0);
        assert_eq!(receipt.row_count(), 0);
        assert_eq!(
            receipt.nifty_terminal(),
            ExecutionV4FamilyTerminal::NaturallyExtinct
        );
        assert_eq!(
            receipt.banknifty_terminal(),
            ExecutionV4FamilyTerminal::NaturallyExtinct
        );
    }

    #[test]
    fn mixed_evaluated_extinct_topologies_persist_in_both_family_orders() {
        for (label, evaluated, expected_nifty, expected_bank) in [
            ("nifty-evaluated", [true, false], 8, 0),
            ("bank-evaluated", [false, true], 0, 8),
        ] {
            let root = TestRoot::new(label);
            let prepared = with_family_topology(prepared(76), evaluated);
            let completion = prepared
                .expected_completion(0, 0, 0, 0)
                .expect("mixed Completion");
            let active_slots = if evaluated[0] {
                &completion.parameter_ids[..2]
            } else {
                &completion.parameter_ids[2..]
            };
            let extinct_slots = if evaluated[0] {
                &completion.parameter_ids[2..]
            } else {
                &completion.parameter_ids[..2]
            };
            assert!(
                active_slots
                    .iter()
                    .all(|parameter_id| *parameter_id != [0; 32])
            );
            assert!(
                extinct_slots
                    .iter()
                    .all(|parameter_id| *parameter_id == [0; 32])
            );
            let mut committed = commit_prepared_for_test(&root.path, bounds(), &prepared)
                .expect("mixed terminal topology commits");
            let receipt = committed.authority().structural_receipt();
            assert_eq!(receipt.nifty_count(), expected_nifty);
            assert_eq!(receipt.banknifty_count(), expected_bank);
            assert_eq!(receipt.parameter_count(), 2);
            assert_eq!(receipt.percentile_count(), 12);
            assert_eq!(receipt.disposition_count(), 8);
            assert_eq!(
                receipt.nifty_terminal(),
                if evaluated[0] {
                    ExecutionV4FamilyTerminal::Evaluated
                } else {
                    ExecutionV4FamilyTerminal::NaturallyExtinct
                }
            );
            assert_eq!(
                receipt.banknifty_terminal(),
                if evaluated[1] {
                    ExecutionV4FamilyTerminal::Evaluated
                } else {
                    ExecutionV4FamilyTerminal::NaturallyExtinct
                }
            );
            let rows = committed
                .authority_mut()
                .ordered_authenticated_dispositions()
                .expect("mixed durable dispositions");
            assert_eq!(rows.len(), 8);
            let expected_family_index = if evaluated[0] { 0 } else { 1 };
            assert!(
                rows.iter()
                    .all(|row| family_index(row.family()) == expected_family_index)
            );
        }
    }

    #[test]
    fn terminal_envelopes_reject_singletons_missing_rows_and_extinct_rows() {
        assert!(
            validate_terminal_envelope(ExecutionV4FamilyTerminal::Evaluated, 1, 1, 1, 1).is_err()
        );
        assert!(
            validate_terminal_envelope(ExecutionV4FamilyTerminal::NaturallyExtinct, 1, 1, 1, 1)
                .is_err()
        );
        assert!(execution_family_terminal(AdmissionV4FamilyTerminal::InsufficientForCscv).is_err());

        let mut missing_evaluated = prepared(77);
        missing_evaluated.dispositions.pop();
        assert!(missing_evaluated.validate(bounds()).is_err());

        let full = prepared(78);
        let mut extinct_with_row = with_family_topology(full.clone(), [true, false]);
        let mut invented = full
            .dispositions
            .get(8)
            .expect("first BANKNIFTY row")
            .clone();
        invented.global_sequence = 8;
        invented.disposition_id = invented.derive_id();
        extinct_with_row.dispositions.push(invented);
        assert!(extinct_with_row.validate(bounds()).is_err());

        let extinct = with_family_topology(prepared(79), [false, false]);
        let mut invented_parameter_slot = extinct
            .expected_completion(0, 0, 0, 0)
            .expect("all-extinct Completion");
        invented_parameter_slot.parameter_ids[0] = digest(79_000);
        invented_parameter_slot.completion_id = invented_parameter_slot.derive_completion_id();
        assert!(invented_parameter_slot.validate().is_err());

        let mixed = with_family_topology(prepared(80), [false, true]);
        let mut missing_active_slot = mixed
            .expected_completion(0, 0, 0, 0)
            .expect("mixed Completion");
        missing_active_slot.parameter_ids[2] = [0; 32];
        missing_active_slot.completion_id = missing_active_slot.derive_completion_id();
        assert!(missing_active_slot.validate().is_err());

        let mut impossible_count = mixed
            .expected_completion(0, 0, 0, 0)
            .expect("mixed Completion");
        impossible_count.parameter_count = 3;
        impossible_count.completion_id = impossible_count.derive_completion_id();
        assert!(impossible_count.validate().is_err());
    }

    #[test]
    fn bounds_and_terminal_matrix_are_explicit_and_complete() {
        assert!(
            ExecutionV4FileBound::new(0, 1, EXECUTION_V4_PARAMETER_BYTES, "parameter").is_err()
        );
        assert!(
            ExecutionV4FileBound::new(
                2,
                EXECUTION_V4_PARAMETER_BYTES as u64,
                EXECUTION_V4_PARAMETER_BYTES,
                "parameter"
            )
            .is_err()
        );
        let prepared = prepared(80);
        let completion = prepared
            .expected_completion(0, 0, 0, 0)
            .expect("matrix Completion");
        assert_eq!(
            checked_sum(&completion.terminal_matrix, "test matrix").unwrap(),
            16
        );
        assert_eq!(completion.nifty_count, 8);
        assert_eq!(completion.banknifty_count, 8);
        assert_eq!(completion.authorized_count, 8);
        assert_eq!(completion.policy_refused_count, 8);
        assert_eq!(completion.admission_counts, [4, 4, 4, 4]);
        assert!(
            completion
                .terminal_matrix
                .into_iter()
                .all(|count| count == 1)
        );
        assert_ne!(
            completion.nifty_authority_id,
            completion.banknifty_authority_id
        );
    }
}
