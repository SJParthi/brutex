//! Receipt-last Execution V3 authority.
//!
//! This module is the version-separated persistence seam after Population V5.
//! It deliberately does not reinterpret a Population record. The sole
//! production door consumes and retains the live Population V5 capability,
//! reproduces its exact Candidate execution sources, and derives the four
//! family/direction parameter records, their percentile atoms, and one Runner
//! terminal disposition per source row. There is no caller-authored production
//! constructor or detached-digest overload in this file.
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
//! must not be inferred from an Execution V3 receipt alone.
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
    reason = "Execution V3 remains crate-private until Selection V5 consumes its source-retaining production capability"
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
use crate::population_admission_v3::{AdmissionV3Family, AdmissionV3Status};
use crate::population_v5::{
    CommittedStoredPopulationV5, PopulationV5ExecutionDispositionSourceV1,
    PopulationV5ExecutionV3SourceV1,
};

/// Bytes in one canonical Execution V3 parameter record.
pub(crate) const EXECUTION_V3_PARAMETER_BYTES: usize = 1_024;
/// Bytes in one canonical Execution V3 percentile atom.
pub(crate) const EXECUTION_V3_PERCENTILE_BYTES: usize = 128;
/// Bytes in one canonical Execution V3 terminal disposition.
pub(crate) const EXECUTION_V3_DISPOSITION_BYTES: usize = 1_024;
/// Bytes in one receipt-last Execution V3 Completion.
pub(crate) const EXECUTION_V3_COMPLETION_BYTES: usize = 1_024;

// The Execution V3 semantic seam remains V3, while this is its fifth
// fixed-record layout. Layout 4 bytes are deliberately refused
// rather than reinterpreted after replacing caller-authored bounds with exact
// Runner policy and resolved-grid facts.
const VERSION: u32 = 5;
const PARAMETER_MAGIC: [u8; 16] = *b"BTX-EXV3-PARAM\0\0";
const PERCENTILE_MAGIC: [u8; 16] = *b"BTX-EXV3-PCTL\0\0\0";
const DISPOSITION_MAGIC: [u8; 16] = *b"BTX-EXV3-DISP\0\0\0";
const COMPLETION_MAGIC: [u8; 16] = *b"BTX-EXV3-CMPL\0\0\0";
const PARAMETER_DOMAIN: u32 = 1;
const PERCENTILE_DOMAIN: u32 = 2;
const DISPOSITION_DOMAIN: u32 = 3;
const COMPLETION_DOMAIN: u32 = 4;
const SEAL_BYTES: usize = 32;
const PARAMETER_PAYLOAD_BYTES: usize = EXECUTION_V3_PARAMETER_BYTES - SEAL_BYTES;
const PERCENTILE_PAYLOAD_BYTES: usize = EXECUTION_V3_PERCENTILE_BYTES - SEAL_BYTES;
const DISPOSITION_PAYLOAD_BYTES: usize = EXECUTION_V3_DISPOSITION_BYTES - SEAL_BYTES;
const COMPLETION_PAYLOAD_BYTES: usize = EXECUTION_V3_COMPLETION_BYTES - SEAL_BYTES;
const EVALUATION_FINGERPRINT_BYTES: usize = 155;
const READ_CHUNK_BYTES: usize = 16 * 1_024;
const PARAMETER_COUNT_PER_BLOCK: usize = 4;
const MATRIX_CELLS: usize = 16;
const OPTIONAL_U32_NONE: u32 = u32::MAX;

const PARAMETER_CORE_ID_DOMAIN: &[u8] = b"brutex-execution-v3-layout-5-parameter-core-id\0";
const PARAMETER_ID_DOMAIN: &[u8] = b"brutex-execution-v3-layout-5-parameter-id\0";
const DISPOSITION_ID_DOMAIN: &[u8] = b"brutex-execution-v3-layout-5-disposition-id\0";
const COMPLETION_ID_DOMAIN: &[u8] = b"brutex-execution-v3-layout-5-completion-id\0";
const FAMILY_AUTHORITY_ID_DOMAIN: &[u8] = b"brutex-execution-v3-layout-5-family-authority-id\0";
const ORDERED_PARAMETERS_DOMAIN: &[u8] = b"brutex-execution-v3-layout-5-ordered-parameters\0";
const ORDERED_PERCENTILES_DOMAIN: &[u8] = b"brutex-execution-v3-layout-5-ordered-percentiles\0";
const ORDERED_DISPOSITIONS_DOMAIN: &[u8] = b"brutex-execution-v3-layout-5-ordered-dispositions\0";
const PARAMETER_SEAL_DOMAIN: &[u8] = b"brutex-execution-v3-layout-5-parameter-seal\0";
const PERCENTILE_SEAL_DOMAIN: &[u8] = b"brutex-execution-v3-layout-5-percentile-seal\0";
const DISPOSITION_SEAL_DOMAIN: &[u8] = b"brutex-execution-v3-layout-5-disposition-seal\0";
const COMPLETION_SEAL_DOMAIN: &[u8] = b"brutex-execution-v3-layout-5-completion-seal\0";
const FILE_GENERATION_DOMAIN: &[u8] = b"brutex-execution-v3-layout-5-file-generation\0";

const PARAMETER_FILE: &str = "execution-parameters-v3.bin";
const PERCENTILE_FILE: &str = "execution-percentiles-v3.bin";
const DISPOSITION_FILE: &str = "execution-dispositions-v3.bin";
const COMPLETION_FILE: &str = "execution-completions-v3.bin";
const LOCK_FILE: &str = "execution-v3.lock";
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

const _: () = assert!(PARAMETER_PAYLOAD_BYTES + SEAL_BYTES == EXECUTION_V3_PARAMETER_BYTES);
const _: () = assert!(PERCENTILE_PAYLOAD_BYTES + SEAL_BYTES == EXECUTION_V3_PERCENTILE_BYTES);
const _: () = assert!(DISPOSITION_PAYLOAD_BYTES + SEAL_BYTES == EXECUTION_V3_DISPOSITION_BYTES);
const _: () = assert!(COMPLETION_PAYLOAD_BYTES + SEAL_BYTES == EXECUTION_V3_COMPLETION_BYTES);

/// Operator-facing refusal at the Execution V3 boundary.
pub(crate) type ExecutionV3Refusal = String;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub(crate) enum ExecutionV3Family {
    Nifty = 1,
    BankNifty = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub(crate) enum ExecutionV3Direction {
    Long = 1,
    Short = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum ExecutionV3AdmissionStatus {
    Admitted = 1,
    Rejected = 2,
    Unmeasured = 3,
    Refused = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum ExecutionV3Terminal {
    Authorized = 1,
    PolicyRefused = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub(crate) enum ExecutionV3PercentileAxis {
    Stop = 1,
    Target = 2,
    Trail = 3,
}

/// Explicit fixed-file ceiling. There is deliberately no `Default`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExecutionV3FileBound {
    records: u64,
    bytes: u64,
}

impl ExecutionV3FileBound {
    pub(crate) fn new(
        max_records: u64,
        max_bytes: u64,
        stride: usize,
        name: &str,
    ) -> Result<Self, ExecutionV3Refusal> {
        if max_records == 0 || max_bytes == 0 {
            return Err(format!(
                "Execution V3 {name} record and byte ceilings must be nonzero"
            ));
        }
        let required = max_records
            .checked_mul(
                u64::try_from(stride)
                    .map_err(|_| format!("Execution V3 {name} stride does not fit u64"))?,
            )
            .ok_or_else(|| format!("Execution V3 {name} byte ceiling overflowed"))?;
        if max_bytes < required {
            return Err(format!(
                "Execution V3 {name} byte maximum {max_bytes} cannot hold {max_records} records ({required} bytes)"
            ));
        }
        Ok(Self {
            records: max_records,
            bytes: max_bytes,
        })
    }
}

/// Explicit ceilings for every V3 file and one semantic block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExecutionV3Bounds {
    parameters: ExecutionV3FileBound,
    percentiles: ExecutionV3FileBound,
    dispositions: ExecutionV3FileBound,
    completions: ExecutionV3FileBound,
    dispositions_per_block: u64,
    percentiles_per_block: u64,
}

impl ExecutionV3Bounds {
    pub(crate) fn new(
        parameters: ExecutionV3FileBound,
        percentiles: ExecutionV3FileBound,
        dispositions: ExecutionV3FileBound,
        completions: ExecutionV3FileBound,
        max_dispositions_per_block: u64,
        max_percentiles_per_block: u64,
    ) -> Result<Self, ExecutionV3Refusal> {
        if max_dispositions_per_block == 0 || max_percentiles_per_block == 0 {
            return Err(
                "Execution V3 per-block disposition and percentile ceilings must be nonzero"
                    .to_owned(),
            );
        }
        if max_dispositions_per_block > dispositions.records
            || max_percentiles_per_block > percentiles.records
            || parameters.records < PARAMETER_COUNT_PER_BLOCK as u64
        {
            return Err(
                "Execution V3 per-block ceilings exceed their fixed-file record ceilings"
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
struct ExecutionV3ParameterRecord {
    parameter_core_id: [u8; 32],
    parameter_id: [u8; 32],
    population_id: [u8; 32],
    population_ordered_digest: [u8; 32],
    source_finalization_id: [u8; 32],
    source_finalization_completion_id: [u8; 32],
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
    family: ExecutionV3Family,
    direction: ExecutionV3Direction,
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

impl ExecutionV3ParameterRecord {
    fn validate(&self) -> Result<(), ExecutionV3Refusal> {
        for (name, value) in [
            ("parameter core identity", self.parameter_core_id),
            ("parameter identity", self.parameter_id),
            ("Population V5 identity", self.population_id),
            (
                "Population V5 ordered digest",
                self.population_ordered_digest,
            ),
            ("Finalization V3 identity", self.source_finalization_id),
            (
                "Finalization V3 Completion",
                self.source_finalization_completion_id,
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
                "Execution V3 parameter does not bind one-minute OHLCV, next-minute entry and 15:10 IST close"
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
            return Err("Execution V3 parameter contains a zero required bound/policy".to_owned());
        }
        let possible_ratio_pairs = self
            .resolved_stop_count
            .checked_mul(self.resolved_target_count)
            .ok_or_else(|| "Execution V3 resolved ratio-pair envelope overflowed".to_owned())?;
        self.validate_forced_stop()?;
        let max_resolved_stops = self
            .max_levels
            .checked_add(u64::from(
                self.forced_stop_policy_tag != FORCED_STOP_DISABLED_TAG,
            ))
            .ok_or_else(|| "Execution V3 forced-stop-grown axis bound overflowed".to_owned())?;
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
            return Err("Execution V3 parameter bounds are contradictory or overflow".to_owned());
        }
        if self.evaluation_fingerprint.iter().all(|byte| *byte == 0) {
            return Err("Execution V3 evaluation fingerprint is zero".to_owned());
        }
        if self.parameter_core_id != self.derive_core_id() {
            return Err("Execution V3 parameter core identity does not reproduce".to_owned());
        }
        if self.parameter_id != self.derive_parameter_id() {
            return Err("Execution V3 parameter identity does not reproduce".to_owned());
        }
        Ok(())
    }

    fn validate_forced_stop(&self) -> Result<(), ExecutionV3Refusal> {
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
                "Execution V3 forced-stop tag, positive PPM and resolved index disagree".to_owned(),
            ),
            _ => Err("Execution V3 forced-stop policy tag is unknown".to_owned()),
        }
    }

    fn derive_core_id(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(PARAMETER_CORE_ID_DOMAIN);
        hasher.update(&VERSION.to_le_bytes());
        for value in [
            self.population_id,
            self.population_ordered_digest,
            self.source_finalization_id,
            self.source_finalization_completion_id,
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

    fn encode(&self) -> Result<[u8; EXECUTION_V3_PARAMETER_BYTES], ExecutionV3Refusal> {
        self.validate()?;
        let mut raw = [0_u8; EXECUTION_V3_PARAMETER_BYTES];
        let (payload, seal) = raw.split_at_mut(PARAMETER_PAYLOAD_BYTES);
        let mut writer = FixedWriter::new(payload);
        writer.array(&PARAMETER_MAGIC)?;
        writer.u32(VERSION)?;
        writer.u32(PARAMETER_DOMAIN)?;
        for value in [
            self.parameter_core_id,
            self.parameter_id,
            self.population_id,
            self.population_ordered_digest,
            self.source_finalization_id,
            self.source_finalization_completion_id,
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

    fn decode(raw: &[u8; EXECUTION_V3_PARAMETER_BYTES]) -> Result<Self, ExecutionV3Refusal> {
        let (payload, seal) = raw.split_at(PARAMETER_PAYLOAD_BYTES);
        require_seal("parameter", PARAMETER_SEAL_DOMAIN, payload, seal)?;
        let mut reader = FixedReader::new(payload);
        require_header("parameter", &mut reader, PARAMETER_MAGIC, PARAMETER_DOMAIN)?;
        let parameter = Self {
            parameter_core_id: reader.array()?,
            parameter_id: reader.array()?,
            population_id: reader.array()?,
            population_ordered_digest: reader.array()?,
            source_finalization_id: reader.array()?,
            source_finalization_completion_id: reader.array()?,
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
            return Err("Execution V3 parameter is not byte-canonical".to_owned());
        }
        Ok(parameter)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExecutionV3PercentileRecord {
    parameter_core_id: [u8; 32],
    axis: ExecutionV3PercentileAxis,
    ordinal: u32,
    numerator: u32,
    denominator: u32,
}

impl ExecutionV3PercentileRecord {
    fn validate(&self) -> Result<(), ExecutionV3Refusal> {
        require_nonzero("percentile parameter core identity", self.parameter_core_id)?;
        if self.denominator == 0 || self.numerator > self.denominator {
            return Err("Execution V3 percentile ratio is outside [0,1]".to_owned());
        }
        Ok(())
    }

    fn encode(&self) -> Result<[u8; EXECUTION_V3_PERCENTILE_BYTES], ExecutionV3Refusal> {
        self.validate()?;
        let mut raw = [0_u8; EXECUTION_V3_PERCENTILE_BYTES];
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

    fn decode(raw: &[u8; EXECUTION_V3_PERCENTILE_BYTES]) -> Result<Self, ExecutionV3Refusal> {
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
            return Err("Execution V3 percentile is not byte-canonical".to_owned());
        }
        Ok(percentile)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExecutionV3DispositionRecord {
    disposition_id: [u8; 32],
    population_id: [u8; 32],
    population_row_id: [u8; 32],
    candidate_semantic_id: [u8; 32],
    candidate_base_row_id: [u8; 32],
    admission_decision_id: [u8; 32],
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
    family: ExecutionV3Family,
    direction: ExecutionV3Direction,
    admission_status: ExecutionV3AdmissionStatus,
    terminal: ExecutionV3Terminal,
    rung: u32,
    horizon_bars: u32,
    stop_index: Option<u32>,
    target_index: Option<u32>,
    tsl_index: Option<u32>,
    ttp_arm_index: Option<u32>,
    ttp_trail_index: Option<u32>,
}

impl ExecutionV3DispositionRecord {
    fn validate(&self) -> Result<(), ExecutionV3Refusal> {
        for (name, value) in [
            ("disposition identity", self.disposition_id),
            ("Population V5 identity", self.population_id),
            ("Population V5 row", self.population_row_id),
            ("Candidate semantic identity", self.candidate_semantic_id),
            ("Candidate base row", self.candidate_base_row_id),
            ("Admission V3 decision", self.admission_decision_id),
            ("Finalization V3 row", self.finalization_row_id),
            (
                "Finalization V3 Completion",
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
            return Err("Execution V3 disposition has a zero required fact".to_owned());
        }
        if self.ttp_arm_index.is_some() != self.ttp_trail_index.is_some() {
            return Err("Execution V3 disposition contains a half TTP coordinate pair".to_owned());
        }
        match self.terminal {
            ExecutionV3Terminal::Authorized => {
                if self.selected_exit_digest == [0; 32]
                    || self.refusal_bits != 0
                    || self.stop_index.is_none()
                    || self.target_index.is_none()
                {
                    return Err(
                        "Execution V3 Authorized terminal lacks selected-exit evidence or carries refusal bits"
                            .to_owned(),
                    );
                }
            }
            ExecutionV3Terminal::PolicyRefused => {
                if self.selected_exit_digest != [0; 32]
                    || self.refusal_bits == 0
                    || runner::exit_grid_policy::ExecutionRefusalBitsV1::from_bits(
                        self.refusal_bits,
                    )
                    .is_none()
                {
                    return Err(
                        "Execution V3 PolicyRefused terminal contains invented selected-exit evidence or lacks exact known refusal bits"
                            .to_owned(),
                    );
                }
            }
        }
        if self.disposition_id != self.derive_id() {
            return Err("Execution V3 disposition identity does not reproduce".to_owned());
        }
        Ok(())
    }

    fn derive_id(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(DISPOSITION_ID_DOMAIN);
        hasher.update(&VERSION.to_le_bytes());
        for value in [
            self.population_id,
            self.population_row_id,
            self.candidate_semantic_id,
            self.candidate_base_row_id,
            self.admission_decision_id,
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

    fn encode(&self) -> Result<[u8; EXECUTION_V3_DISPOSITION_BYTES], ExecutionV3Refusal> {
        self.validate()?;
        let mut raw = [0_u8; EXECUTION_V3_DISPOSITION_BYTES];
        let (payload, seal) = raw.split_at_mut(DISPOSITION_PAYLOAD_BYTES);
        let mut writer = FixedWriter::new(payload);
        writer.array(&DISPOSITION_MAGIC)?;
        writer.u32(VERSION)?;
        writer.u32(DISPOSITION_DOMAIN)?;
        for value in [
            self.disposition_id,
            self.population_id,
            self.population_row_id,
            self.candidate_semantic_id,
            self.candidate_base_row_id,
            self.admission_decision_id,
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

    fn decode(raw: &[u8; EXECUTION_V3_DISPOSITION_BYTES]) -> Result<Self, ExecutionV3Refusal> {
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
            population_row_id: reader.array()?,
            candidate_semantic_id: reader.array()?,
            candidate_base_row_id: reader.array()?,
            admission_decision_id: reader.array()?,
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
            return Err("Execution V3 disposition is not byte-canonical".to_owned());
        }
        Ok(disposition)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExecutionV3CompletionRecord {
    block_sequence: u64,
    first_parameter_record: u64,
    parameter_count: u64,
    first_percentile_record: u64,
    percentile_count: u64,
    first_disposition_record: u64,
    disposition_count: u64,
    completion_id: [u8; 32],
    population_id: [u8; 32],
    population_ordered_digest: [u8; 32],
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
    parameter_ids: [[u8; 32]; PARAMETER_COUNT_PER_BLOCK],
    row_count: u64,
    nifty_count: u64,
    banknifty_count: u64,
    authorized_count: u64,
    policy_refused_count: u64,
    rung: u32,
    admission_counts: [u64; 4],
    terminal_matrix: [u64; MATRIX_CELLS],
}

impl ExecutionV3CompletionRecord {
    fn validate(&self) -> Result<(), ExecutionV3Refusal> {
        for (name, value) in [
            ("Completion identity", self.completion_id),
            ("Population V5 identity", self.population_id),
            (
                "Population V5 ordered digest",
                self.population_ordered_digest,
            ),
            ("Finalization V3 identity", self.source_finalization_id),
            (
                "Finalization V3 Completion",
                self.source_finalization_completion_id,
            ),
            ("Admission V3 block", self.source_admission_block_id),
            (
                "Admission V3 Completion",
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
        for parameter_id in self.parameter_ids {
            require_nonzero("Completion parameter identity", parameter_id)?;
        }
        if self.execution_law_digest != execution_law_digest()
            || self.parameter_count != PARAMETER_COUNT_PER_BLOCK as u64
            || self.percentile_count == 0
            || self.row_count == 0
            || self.rung == 0
            || self.disposition_count != self.row_count
            || checked_sum(&[self.nifty_count, self.banknifty_count], "family counts")?
                != self.row_count
            || checked_sum(
                &[self.authorized_count, self.policy_refused_count],
                "terminal counts",
            )? != self.row_count
            || checked_sum(&self.admission_counts, "admission counts")? != self.row_count
            || checked_sum(&self.terminal_matrix, "terminal matrix")? != self.row_count
        {
            return Err("Execution V3 Completion counts do not cover the block exactly".to_owned());
        }
        if self.completion_id != self.derive_completion_id() {
            return Err("Execution V3 Completion identity does not reproduce".to_owned());
        }
        Ok(())
    }

    fn derive_completion_id(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(COMPLETION_ID_DOMAIN);
        hasher.update(&VERSION.to_le_bytes());
        for value in [
            self.population_id,
            self.population_ordered_digest,
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
            self.nifty_count,
            self.banknifty_count,
            self.authorized_count,
            self.policy_refused_count,
        ] {
            hasher.update(&value.to_le_bytes());
        }
        hasher.update(&self.rung.to_le_bytes());
        for value in self.admission_counts {
            hasher.update(&value.to_le_bytes());
        }
        for value in self.terminal_matrix {
            hasher.update(&value.to_le_bytes());
        }
        hasher.finalize()
    }

    fn encode(&self) -> Result<[u8; EXECUTION_V3_COMPLETION_BYTES], ExecutionV3Refusal> {
        self.validate()?;
        let mut raw = [0_u8; EXECUTION_V3_COMPLETION_BYTES];
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
            self.population_ordered_digest,
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
            self.nifty_count,
            self.banknifty_count,
            self.authorized_count,
            self.policy_refused_count,
        ] {
            writer.u64(value)?;
        }
        writer.u32(self.rung)?;
        writer.zeros(4)?;
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

    fn decode(raw: &[u8; EXECUTION_V3_COMPLETION_BYTES]) -> Result<Self, ExecutionV3Refusal> {
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
            population_ordered_digest: reader.array()?,
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
            nifty_count: reader.u64()?,
            banknifty_count: reader.u64()?,
            authorized_count: reader.u64()?,
            policy_refused_count: reader.u64()?,
            rung: reader.u32()?,
            admission_counts: {
                reader.require_zeros(4, "Completion rung reserve")?;
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
            return Err("Execution V3 Completion is not byte-canonical".to_owned());
        }
        Ok(completion)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedExecutionV3 {
    population_id: [u8; 32],
    population_ordered_digest: [u8; 32],
    source_finalization_id: [u8; 32],
    source_finalization_completion_id: [u8; 32],
    source_admission_block_id: [u8; 32],
    source_admission_completion_id: [u8; 32],
    parameters: [ExecutionV3ParameterRecord; PARAMETER_COUNT_PER_BLOCK],
    percentiles: Vec<ExecutionV3PercentileRecord>,
    dispositions: Vec<ExecutionV3DispositionRecord>,
}

impl PreparedExecutionV3 {
    /// Private preparation path from the sole opaque Population V5 execution
    /// source. No row, digest, grid, mask, or disposition overload exists.
    fn from_population_source(
        source: &PopulationV5ExecutionV3SourceV1,
        bounds: ExecutionV3Bounds,
    ) -> Result<Self, ExecutionV3Refusal> {
        let receipt = source.receipt();
        let mut percentiles = Vec::new();
        let mut parameters = Vec::new();
        parameters
            .try_reserve_exact(PARAMETER_COUNT_PER_BLOCK)
            .map_err(|why| format!("cannot reserve Execution V3 parameters: {why}"))?;
        for facts in source.parameters() {
            let (parameter, mut segment) = parameter_from_population_source(
                receipt,
                facts,
                usize_to_u64(percentiles.len(), "percentile offset")?,
            )?;
            percentiles.append(&mut segment);
            parameters.push(parameter);
        }
        let parameters: [ExecutionV3ParameterRecord; PARAMETER_COUNT_PER_BLOCK] = parameters
            .try_into()
            .map_err(|values: Vec<ExecutionV3ParameterRecord>| {
                format!(
                    "Execution V3 Population source produced {} parameters, not four",
                    values.len()
                )
            })?;
        let mut dispositions = Vec::new();
        dispositions
            .try_reserve_exact(source.rows().len())
            .map_err(|why| format!("cannot reserve Execution V3 dispositions: {why}"))?;
        for row in source.rows() {
            dispositions.push(disposition_from_population_source(row, &parameters)?);
        }
        let prepared = Self {
            population_id: receipt.population_id(),
            population_ordered_digest: receipt.ordered_row_digest(),
            source_finalization_id: receipt.source_finalization_id(),
            source_finalization_completion_id: receipt.source_finalization_completion_id(),
            source_admission_block_id: receipt.source_admission_block_id(),
            source_admission_completion_id: receipt.source_admission_completion_id(),
            parameters,
            percentiles,
            dispositions,
        };
        prepared.validate(bounds)?;
        Ok(prepared)
    }

    fn validate(&self, bounds: ExecutionV3Bounds) -> Result<(), ExecutionV3Refusal> {
        for (name, value) in [
            ("Population V5 identity", self.population_id),
            (
                "Population V5 ordered digest",
                self.population_ordered_digest,
            ),
            ("Finalization V3 identity", self.source_finalization_id),
            (
                "Finalization V3 Completion",
                self.source_finalization_completion_id,
            ),
            ("Admission V3 block", self.source_admission_block_id),
            (
                "Admission V3 Completion",
                self.source_admission_completion_id,
            ),
        ] {
            require_nonzero(name, value)?;
        }
        require_block_counts(bounds, self.percentiles.len(), self.dispositions.len())?;
        let expected_order = [
            (ExecutionV3Family::Nifty, ExecutionV3Direction::Long),
            (ExecutionV3Family::Nifty, ExecutionV3Direction::Short),
            (ExecutionV3Family::BankNifty, ExecutionV3Direction::Long),
            (ExecutionV3Family::BankNifty, ExecutionV3Direction::Short),
        ];
        let mut parameter_ids = HashSet::new();
        parameter_ids
            .try_reserve(PARAMETER_COUNT_PER_BLOCK)
            .map_err(|why| format!("cannot reserve Execution V3 parameter identities: {why}"))?;
        let mut expected_percentile_offset = 0_u64;
        let [first_parameter, ..] = &self.parameters;
        let block_rung = first_parameter.rung;
        for (parameter, (family, direction)) in self.parameters.iter().zip(expected_order) {
            parameter.validate()?;
            if parameter.population_id != self.population_id
                || parameter.population_ordered_digest != self.population_ordered_digest
                || parameter.source_finalization_id != self.source_finalization_id
                || parameter.source_finalization_completion_id
                    != self.source_finalization_completion_id
                || parameter.execution_law_digest != execution_law_digest()
                || parameter.family != family
                || parameter.direction != direction
                || parameter.rung != block_rung
                || parameter.percentile_offset != expected_percentile_offset
                || !parameter_ids.insert(parameter.parameter_id)
            {
                return Err(
                    "Execution V3 parameters violate source/law/order/common-rung/unique identity"
                        .to_owned(),
                );
            }
            let end = parameter
                .percentile_offset
                .checked_add(parameter.percentile_count)
                .ok_or_else(|| "Execution V3 percentile range overflowed".to_owned())?;
            let start = usize::try_from(parameter.percentile_offset)
                .map_err(|_| "Execution V3 percentile start does not fit usize".to_owned())?;
            let end_usize = usize::try_from(end)
                .map_err(|_| "Execution V3 percentile end does not fit usize".to_owned())?;
            let segment = self.percentiles.get(start..end_usize).ok_or_else(|| {
                "Execution V3 parameter percentile segment exceeds prepared atoms".to_owned()
            })?;
            validate_percentile_segment(parameter, segment)?;
            expected_percentile_offset = end;
        }
        if expected_percentile_offset
            != u64::try_from(self.percentiles.len())
                .map_err(|_| "Execution V3 percentile count does not fit u64".to_owned())?
        {
            return Err("Execution V3 percentile segments do not cover every atom".to_owned());
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
    ) -> Result<ExecutionV3CompletionRecord, ExecutionV3Refusal> {
        let [first_parameter, ..] = &self.parameters;
        let parameter_count = PARAMETER_COUNT_PER_BLOCK as u64;
        let percentile_count = u64::try_from(self.percentiles.len())
            .map_err(|_| "Execution V3 percentile count does not fit u64".to_owned())?;
        let disposition_count = u64::try_from(self.dispositions.len())
            .map_err(|_| "Execution V3 disposition count does not fit u64".to_owned())?;
        let mut nifty_count = 0_u64;
        let mut banknifty_count = 0_u64;
        let mut authorized_count = 0_u64;
        let mut policy_refused_count = 0_u64;
        let mut admission_counts = [0_u64; 4];
        let mut terminal_matrix = [0_u64; MATRIX_CELLS];
        for row in &self.dispositions {
            match row.family {
                ExecutionV3Family::Nifty => increment(&mut nifty_count, "NIFTY count")?,
                ExecutionV3Family::BankNifty => {
                    increment(&mut banknifty_count, "BANKNIFTY count")?;
                }
            }
            match row.terminal {
                ExecutionV3Terminal::Authorized => {
                    increment(&mut authorized_count, "Authorized count")?;
                }
                ExecutionV3Terminal::PolicyRefused => {
                    increment(&mut policy_refused_count, "PolicyRefused count")?;
                }
            }
            let admission_slot = admission_counts
                .get_mut(admission_index(row.admission_status))
                .ok_or_else(|| "Execution V3 admission status index escaped schema".to_owned())?;
            increment(admission_slot, "admission count")?;
            let matrix_slot = terminal_matrix
                .get_mut(matrix_index(row.family, row.admission_status, row.terminal))
                .ok_or_else(|| "Execution V3 terminal matrix index escaped schema".to_owned())?;
            increment(matrix_slot, "terminal matrix cell")?;
        }
        let ordered_parameter_digest = ordered_parameter_digest(&self.parameters)?;
        let ordered_percentile_digest = ordered_percentile_digest(&self.percentiles)?;
        let ordered_disposition_digest = ordered_disposition_digest(&self.dispositions)?;
        let nifty_disposition_digest =
            family_disposition_digest(ExecutionV3Family::Nifty, &self.dispositions)?;
        let banknifty_disposition_digest =
            family_disposition_digest(ExecutionV3Family::BankNifty, &self.dispositions)?;
        let parameter_ids = self.parameters.each_ref().map(|value| value.parameter_id);
        let [nifty_long, nifty_short, banknifty_long, banknifty_short] = parameter_ids;
        let (nifty_matrix, banknifty_matrix) = terminal_matrix.split_at(8);
        let nifty_authority_id = derive_family_authority_id(
            ExecutionV3Family::Nifty,
            self,
            [nifty_long, nifty_short],
            nifty_disposition_digest,
            nifty_count,
            nifty_matrix,
        );
        let banknifty_authority_id = derive_family_authority_id(
            ExecutionV3Family::BankNifty,
            self,
            [banknifty_long, banknifty_short],
            banknifty_disposition_digest,
            banknifty_count,
            banknifty_matrix,
        );
        let mut completion = ExecutionV3CompletionRecord {
            block_sequence,
            first_parameter_record,
            parameter_count,
            first_percentile_record,
            percentile_count,
            first_disposition_record,
            disposition_count,
            completion_id: [0; 32],
            population_id: self.population_id,
            population_ordered_digest: self.population_ordered_digest,
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
            nifty_count,
            banknifty_count,
            authorized_count,
            policy_refused_count,
            rung: first_parameter.rung,
            admission_counts,
            terminal_matrix,
        };
        completion.completion_id = completion.derive_completion_id();
        completion.validate()?;
        Ok(completion)
    }
}

fn parameter_from_population_source(
    receipt: crate::population_v5::PopulationV5StructuralReceipt,
    facts: &CandidateExecutionParameterFactsV1,
    percentile_offset: u64,
) -> Result<(ExecutionV3ParameterRecord, Vec<ExecutionV3PercentileRecord>), ExecutionV3Refusal> {
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
        .ok_or_else(|| "Execution V3 percentile count overflowed usize".to_owned())?;
    let percentile_count = usize_to_u64(percentile_count, "parameter percentile count")?;
    let mut parameter = ExecutionV3ParameterRecord {
        parameter_core_id: [0; 32],
        parameter_id: [0; 32],
        population_id: receipt.population_id(),
        population_ordered_digest: receipt.ordered_row_digest(),
        source_finalization_id: receipt.source_finalization_id(),
        source_finalization_completion_id: receipt.source_finalization_completion_id(),
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
                .map_err(|_| "Execution V3 percentile count does not fit usize".to_owned())?,
        )
        .map_err(|why| format!("cannot reserve Execution V3 percentile atoms: {why}"))?;
    append_percentiles(
        &mut percentiles,
        parameter.parameter_core_id,
        ExecutionV3PercentileAxis::Stop,
        &facts.stop_percentiles,
    )?;
    append_percentiles(
        &mut percentiles,
        parameter.parameter_core_id,
        ExecutionV3PercentileAxis::Target,
        &facts.target_percentiles,
    )?;
    append_percentiles(
        &mut percentiles,
        parameter.parameter_core_id,
        ExecutionV3PercentileAxis::Trail,
        &facts.trail_percentiles,
    )?;
    parameter.percentile_digest = ordered_percentile_digest(&percentiles)?;
    parameter.parameter_id = parameter.derive_parameter_id();
    parameter.validate()?;
    Ok((parameter, percentiles))
}

fn append_percentiles(
    output: &mut Vec<ExecutionV3PercentileRecord>,
    parameter_core_id: [u8; 32],
    axis: ExecutionV3PercentileAxis,
    values: &[RationalPercentileV1],
) -> Result<(), ExecutionV3Refusal> {
    for (ordinal, value) in values.iter().enumerate() {
        let record = ExecutionV3PercentileRecord {
            parameter_core_id,
            axis,
            ordinal: u32::try_from(ordinal)
                .map_err(|_| "Execution V3 percentile ordinal does not fit u32".to_owned())?,
            numerator: value.numerator(),
            denominator: value.denominator(),
        };
        record.validate()?;
        output.push(record);
    }
    Ok(())
}

fn disposition_from_population_source(
    source: &PopulationV5ExecutionDispositionSourceV1,
    parameters: &[ExecutionV3ParameterRecord; PARAMETER_COUNT_PER_BLOCK],
) -> Result<ExecutionV3DispositionRecord, ExecutionV3Refusal> {
    let population = source.population();
    let candidate = population.candidate().row();
    let runner = source.disposition();
    let family = match population.family() {
        AdmissionV3Family::Nifty => ExecutionV3Family::Nifty,
        AdmissionV3Family::BankNifty => ExecutionV3Family::BankNifty,
    };
    let direction = execution_direction(candidate.direction());
    let parameter = parameters
        .get(parameter_index(family, direction))
        .ok_or_else(|| "Execution V3 parameter index escaped four-record block".to_owned())?;
    let coordinate = runner.coordinate();
    let (ttp_arm_index, ttp_trail_index) = match coordinate.ttp {
        Some(ttp) => (
            Some(usize_to_u32(ttp.arm, "TTP arm")?),
            Some(usize_to_u32(ttp.trail, "TTP trail")?),
        ),
        None => (None, None),
    };
    let terminal = if runner.is_authorized() {
        ExecutionV3Terminal::Authorized
    } else {
        ExecutionV3Terminal::PolicyRefused
    };
    let selected_exit_digest = runner.selected().map_or([0; 32], |value| value.digest());
    let mut row = ExecutionV3DispositionRecord {
        disposition_id: [0; 32],
        population_id: population.population_id(),
        population_row_id: population.row_id(),
        candidate_semantic_id: candidate.candidate_semantic_digest(),
        candidate_base_row_id: population.candidate().base_candidate_row_digest(),
        admission_decision_id: population.admission().decision_id(),
        finalization_row_id: population.finalization_row_id(),
        finalization_completion_id: population.finalization_completion_id(),
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

const fn execution_family(value: InstrumentFamilyV1) -> ExecutionV3Family {
    match value {
        InstrumentFamilyV1::Nifty => ExecutionV3Family::Nifty,
        InstrumentFamilyV1::BankNifty => ExecutionV3Family::BankNifty,
    }
}

const fn execution_direction(value: TradeDirectionV1) -> ExecutionV3Direction {
    match value {
        TradeDirectionV1::Long => ExecutionV3Direction::Long,
        TradeDirectionV1::Short => ExecutionV3Direction::Short,
    }
}

const fn execution_admission_status(value: AdmissionV3Status) -> ExecutionV3AdmissionStatus {
    match value {
        AdmissionV3Status::Admitted => ExecutionV3AdmissionStatus::Admitted,
        AdmissionV3Status::Rejected => ExecutionV3AdmissionStatus::Rejected,
        AdmissionV3Status::Unmeasured => ExecutionV3AdmissionStatus::Unmeasured,
        AdmissionV3Status::Refused => ExecutionV3AdmissionStatus::Refused,
    }
}

fn optional_usize_to_u32(
    value: Option<usize>,
    name: &str,
) -> Result<Option<u32>, ExecutionV3Refusal> {
    value.map(|index| usize_to_u32(index, name)).transpose()
}

fn usize_to_u32(value: usize, name: &str) -> Result<u32, ExecutionV3Refusal> {
    u32::try_from(value).map_err(|_| format!("Execution V3 {name} index does not fit u32"))
}

/// Structural receipt from a bounded fresh reopen. It is not authority by
/// itself and has no caller-controlled constructor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExecutionV3StructuralReceipt {
    block_sequence: u64,
    first_parameter_record: u64,
    first_percentile_record: u64,
    first_disposition_record: u64,
    population_id: [u8; 32],
    population_ordered_digest: [u8; 32],
    ordered_disposition_digest: [u8; 32],
    parameter_count: u64,
    percentile_count: u64,
    disposition_count: u64,
    completion_id: [u8; 32],
    nifty_authority_id: [u8; 32],
    banknifty_authority_id: [u8; 32],
}

impl ExecutionV3StructuralReceipt {
    fn from_completion(value: &ExecutionV3CompletionRecord) -> Self {
        Self {
            block_sequence: value.block_sequence,
            first_parameter_record: value.first_parameter_record,
            first_percentile_record: value.first_percentile_record,
            first_disposition_record: value.first_disposition_record,
            population_id: value.population_id,
            population_ordered_digest: value.population_ordered_digest,
            ordered_disposition_digest: value.ordered_disposition_digest,
            parameter_count: value.parameter_count,
            percentile_count: value.percentile_count,
            disposition_count: value.disposition_count,
            completion_id: value.completion_id,
            nifty_authority_id: value.nifty_authority_id,
            banknifty_authority_id: value.banknifty_authority_id,
        }
    }

    #[must_use]
    pub(crate) const fn population_id(self) -> [u8; 32] {
        self.population_id
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
    bound: ExecutionV3FileBound,
    stride: usize,
    name: &'static str,
}

impl HeldFile {
    fn record_count(&self) -> Result<u64, ExecutionV3Refusal> {
        checked_record_count(
            self.generation.len,
            self.stride,
            self.bound.records,
            self.name,
        )
    }

    fn refresh(&mut self) -> Result<(), ExecutionV3Refusal> {
        self.generation = file_generation(&self.file, &self.path, self.bound.bytes)?;
        Ok(())
    }

    fn require_unchanged(&self) -> Result<(), ExecutionV3Refusal> {
        if file_generation(&self.file, &self.path, self.bound.bytes)? != self.generation {
            return Err(format!(
                "Execution V3 retained {} generation changed",
                self.name
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TrailingExecutionV3 {
    first_parameter_record: u64,
    first_percentile_record: u64,
    first_disposition_record: u64,
    parameters: Vec<ExecutionV3ParameterRecord>,
    percentiles: Vec<ExecutionV3PercentileRecord>,
    dispositions: Vec<ExecutionV3DispositionRecord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExecutionV3StructuralCommit {
    Written(ExecutionV3StructuralReceipt),
    Reused(ExecutionV3StructuralReceipt),
}

impl ExecutionV3StructuralCommit {
    const fn receipt(self) -> ExecutionV3StructuralReceipt {
        match self {
            Self::Written(receipt) | Self::Reused(receipt) => receipt,
        }
    }

    const fn was_written(self) -> bool {
        matches!(self, Self::Written(_))
    }
}

/// Retained, generation-checked Execution V3 fixed-file ledger.
struct ExecutionV3Ledger {
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
    bounds: ExecutionV3Bounds,
    writable: bool,
    receipts: HashMap<[u8; 32], ExecutionV3StructuralReceipt>,
    trailing: Option<TrailingExecutionV3>,
    parameter_records: u64,
    percentile_records: u64,
    disposition_records: u64,
    completion_records: u64,
}

impl ExecutionV3Ledger {
    fn open_read(root: &Path, bounds: ExecutionV3Bounds) -> Result<Self, ExecutionV3Refusal> {
        Self::open(root, bounds, false)
    }

    fn open_write(root: &Path, bounds: ExecutionV3Bounds) -> Result<Self, ExecutionV3Refusal> {
        Self::open(root, bounds, true)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "opening one retained ledger validates and captures all five file generations atomically"
    )]
    fn open(
        root: &Path,
        bounds: ExecutionV3Bounds,
        writable: bool,
    ) -> Result<Self, ExecutionV3Refusal> {
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
                .map_err(|why| format!("cannot lock Execution V3 writer: {why}"))?;
        } else {
            lock_file
                .lock_shared()
                .map_err(|why| format!("cannot lock Execution V3 reader: {why}"))?;
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
                return Err("Execution V3 root changed while child files opened".to_owned());
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
                    .map_err(|why| format!("cannot clone Execution V3 lock file: {why}"))?,
                lock_generation,
                parameters: HeldFile {
                    path: parameter_path,
                    generation: parameter_generation,
                    file: parameter_file,
                    bound: bounds.parameters,
                    stride: EXECUTION_V3_PARAMETER_BYTES,
                    name: "parameter",
                },
                percentiles: HeldFile {
                    path: percentile_path,
                    generation: percentile_generation,
                    file: percentile_file,
                    bound: bounds.percentiles,
                    stride: EXECUTION_V3_PERCENTILE_BYTES,
                    name: "percentile",
                },
                dispositions: HeldFile {
                    path: disposition_path,
                    generation: disposition_generation,
                    file: disposition_file,
                    bound: bounds.dispositions,
                    stride: EXECUTION_V3_DISPOSITION_BYTES,
                    name: "disposition",
                },
                completions: HeldFile {
                    path: completion_path,
                    generation: completion_generation,
                    file: completion_file,
                    bound: bounds.completions,
                    stride: EXECUTION_V3_COMPLETION_BYTES,
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
            .map_err(|why| format!("cannot unlock Execution V3 open lock: {why}"));
        combine_lock_result(opened, released)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one receipt-last scan must reconcile every completed block and the sole trailing prefix before indexing"
    )]
    fn scan(&mut self) -> Result<(), ExecutionV3Refusal> {
        let parameter_records = self.parameters.record_count()?;
        let percentile_records = self.percentiles.record_count()?;
        let disposition_records = self.dispositions.record_count()?;
        let completion_records = self.completions.record_count()?;
        self.receipts.clear();
        self.receipts
            .try_reserve(
                usize::try_from(completion_records)
                    .map_err(|_| "Execution V3 Completion count does not fit usize".to_owned())?,
            )
            .map_err(|why| format!("cannot reserve Execution V3 receipt index: {why}"))?;
        self.trailing = None;
        let mut covered_parameters = 0_u64;
        let mut covered_percentiles = 0_u64;
        let mut covered_dispositions = 0_u64;
        for completion_index in 0..completion_records {
            let completion = ExecutionV3CompletionRecord::decode(&read_fixed_at(
                &mut self.completions.file,
                completion_index,
                EXECUTION_V3_COMPLETION_BYTES,
                "Completion",
            )?)?;
            if completion.block_sequence != completion_index
                || completion.first_parameter_record != covered_parameters
                || completion.first_percentile_record != covered_percentiles
                || completion.first_disposition_record != covered_dispositions
            {
                return Err(format!(
                    "Execution V3 Completion {completion_index} is not contiguous/canonical"
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
                    "Execution V3 Completion {completion_index} points beyond a fixed-record file"
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
                    "Execution V3 Population identity {} appears more than once",
                    hex32(receipt.population_id)
                ));
            }
            covered_parameters = parameter_end;
            covered_percentiles = percentile_end;
            covered_dispositions = disposition_end;
        }
        let parameter_tail = parameter_records
            .checked_sub(covered_parameters)
            .ok_or_else(|| "Execution V3 parameter tail underflowed".to_owned())?;
        let percentile_tail = percentile_records
            .checked_sub(covered_percentiles)
            .ok_or_else(|| "Execution V3 percentile tail underflowed".to_owned())?;
        let disposition_tail = disposition_records
            .checked_sub(covered_dispositions)
            .ok_or_else(|| "Execution V3 disposition tail underflowed".to_owned())?;
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
                    "Execution V3 trailing block duplicates a committed Population".to_owned(),
                );
            }
            self.trailing = Some(TrailingExecutionV3 {
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
        prepared: &PreparedExecutionV3,
    ) -> Result<ExecutionV3StructuralCommit, ExecutionV3Refusal> {
        if !self.writable {
            return Err("Execution V3 ledger was opened read-only".to_owned());
        }
        self.lock_file
            .lock()
            .map_err(|why| format!("cannot take Execution V3 append lock: {why}"))?;
        let result = self.append_locked(prepared);
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Execution V3 append lock: {why}"));
        combine_lock_result(result, released)
    }

    fn append_locked(
        &mut self,
        prepared: &PreparedExecutionV3,
    ) -> Result<ExecutionV3StructuralCommit, ExecutionV3Refusal> {
        self.require_unchanged()?;
        prepared.validate(self.bounds)?;
        if let Some(existing) = self.receipts.get(&prepared.population_id).copied() {
            return self.reuse_existing(prepared, existing);
        }
        let trailing = self.trailing.clone().unwrap_or(TrailingExecutionV3 {
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
            .map_err(|why| format!("cannot sync Execution V3 parameters: {why}"))?;
        self.parameters.refresh()?;
        self.parameter_records = self.parameters.record_count()?;
        self.require_unchanged()?;

        self.append_percentile_suffix(prepared, trailing.percentiles.len())?;
        self.percentiles
            .file
            .sync_data()
            .map_err(|why| format!("cannot sync Execution V3 percentiles: {why}"))?;
        self.percentiles.refresh()?;
        self.percentile_records = self.percentiles.record_count()?;
        self.require_unchanged()?;

        self.append_disposition_suffix(prepared, trailing.dispositions.len())?;
        self.dispositions
            .file
            .sync_data()
            .map_err(|why| format!("cannot sync Execution V3 dispositions: {why}"))?;
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
            .ok_or_else(|| "Execution V3 appended block was not indexed".to_owned())?;
        Ok(ExecutionV3StructuralCommit::Written(receipt))
    }

    fn reuse_existing(
        &mut self,
        prepared: &PreparedExecutionV3,
        existing: ExecutionV3StructuralReceipt,
    ) -> Result<ExecutionV3StructuralCommit, ExecutionV3Refusal> {
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
                "Execution V3 Population {} exists with different exact records",
                hex32(existing.population_id)
            ));
        }
        let observed = ExecutionV3CompletionRecord::decode(&read_fixed_at(
            &mut self.completions.file,
            existing.block_sequence,
            EXECUTION_V3_COMPLETION_BYTES,
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
                "Execution V3 Population {} exists with a different Completion",
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
                .map_err(|why| format!("cannot sync reused Execution V3 block: {why}"))?;
        }
        sync_directory(&self.root_file, &self.root)?;
        self.require_unchanged()?;
        Ok(ExecutionV3StructuralCommit::Reused(existing))
    }

    fn require_exact_prefix(
        prepared: &PreparedExecutionV3,
        trailing: &TrailingExecutionV3,
    ) -> Result<(), ExecutionV3Refusal> {
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
            return Err("Execution V3 orphan tail is not an exact retry prefix".to_owned());
        }
        if trailing.parameters.len() < prepared.parameters.len()
            && (!trailing.percentiles.is_empty() || !trailing.dispositions.is_empty())
        {
            return Err(
                "Execution V3 orphan persisted percentiles/dispositions before all parameters"
                    .to_owned(),
            );
        }
        if trailing.percentiles.len() < prepared.percentiles.len()
            && !trailing.dispositions.is_empty()
        {
            return Err(
                "Execution V3 orphan persisted dispositions before all percentiles".to_owned(),
            );
        }
        Ok(())
    }

    fn require_append_bound(
        &self,
        prepared: &PreparedExecutionV3,
        trailing: &TrailingExecutionV3,
    ) -> Result<(), ExecutionV3Refusal> {
        let target_parameters = checked_end(
            trailing.first_parameter_record,
            PARAMETER_COUNT_PER_BLOCK as u64,
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
            .ok_or_else(|| "Execution V3 Completion count overflowed".to_owned())?;
        for (target, bound, name) in [
            (target_parameters, self.bounds.parameters, "parameter"),
            (target_percentiles, self.bounds.percentiles, "percentile"),
            (target_dispositions, self.bounds.dispositions, "disposition"),
            (target_completions, self.bounds.completions, "Completion"),
        ] {
            if target > bound.records {
                return Err(format!(
                    "Execution V3 append reaches {target} {name} records above bound {}",
                    bound.records
                ));
            }
        }
        Ok(())
    }

    fn append_parameter_suffix(
        &mut self,
        prepared: &PreparedExecutionV3,
        start: usize,
    ) -> Result<(), ExecutionV3Refusal> {
        for record in prepared.parameters.get(start..).ok_or_else(|| {
            format!("Execution V3 parameter suffix start {start} is outside the block")
        })? {
            append_raw(&mut self.parameters.file, &record.encode()?)?;
        }
        Ok(())
    }

    fn append_percentile_suffix(
        &mut self,
        prepared: &PreparedExecutionV3,
        start: usize,
    ) -> Result<(), ExecutionV3Refusal> {
        for record in prepared.percentiles.get(start..).ok_or_else(|| {
            format!("Execution V3 percentile suffix start {start} is outside the block")
        })? {
            append_raw(&mut self.percentiles.file, &record.encode()?)?;
        }
        Ok(())
    }

    fn append_disposition_suffix(
        &mut self,
        prepared: &PreparedExecutionV3,
        start: usize,
    ) -> Result<(), ExecutionV3Refusal> {
        for record in prepared.dispositions.get(start..).ok_or_else(|| {
            format!("Execution V3 disposition suffix start {start} is outside the block")
        })? {
            append_raw(&mut self.dispositions.file, &record.encode()?)?;
        }
        Ok(())
    }

    fn append_completion(
        &mut self,
        prepared: &PreparedExecutionV3,
        first_parameter_record: u64,
        first_percentile_record: u64,
        first_disposition_record: u64,
    ) -> Result<(), ExecutionV3Refusal> {
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
            .map_err(|why| format!("cannot sync Execution V3 Completion: {why}"))?;
        sync_directory(&self.root_file, &self.root)?;
        self.completions.refresh()?;
        self.completion_records = self.completions.record_count()?;
        self.require_unchanged()
    }

    fn read_parameters(
        &mut self,
        first: u64,
        count: u64,
    ) -> Result<Vec<ExecutionV3ParameterRecord>, ExecutionV3Refusal> {
        let mut values = bounded_vec(count, self.bounds.parameters.records, "parameters")?;
        for offset in 0..count {
            values.push(ExecutionV3ParameterRecord::decode(&read_fixed_at(
                &mut self.parameters.file,
                checked_end(first, offset, "parameter read offset")?,
                EXECUTION_V3_PARAMETER_BYTES,
                "parameter",
            )?)?);
        }
        Ok(values)
    }

    fn read_percentiles(
        &mut self,
        first: u64,
        count: u64,
    ) -> Result<Vec<ExecutionV3PercentileRecord>, ExecutionV3Refusal> {
        let mut values = bounded_vec(count, self.bounds.percentiles.records, "percentiles")?;
        for offset in 0..count {
            values.push(ExecutionV3PercentileRecord::decode(&read_fixed_at(
                &mut self.percentiles.file,
                checked_end(first, offset, "percentile read offset")?,
                EXECUTION_V3_PERCENTILE_BYTES,
                "percentile",
            )?)?);
        }
        Ok(values)
    }

    fn read_dispositions(
        &mut self,
        first: u64,
        count: u64,
    ) -> Result<Vec<ExecutionV3DispositionRecord>, ExecutionV3Refusal> {
        let mut values = bounded_vec(count, self.bounds.dispositions.records, "dispositions")?;
        for offset in 0..count {
            values.push(ExecutionV3DispositionRecord::decode(&read_fixed_at(
                &mut self.dispositions.file,
                checked_end(first, offset, "disposition read offset")?,
                EXECUTION_V3_DISPOSITION_BYTES,
                "disposition",
            )?)?);
        }
        Ok(values)
    }

    fn structural_receipt(
        &self,
        population_id: &[u8; 32],
    ) -> Result<Option<ExecutionV3StructuralReceipt>, ExecutionV3Refusal> {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take Execution V3 lookup lock: {why}"))?;
        let result = self
            .require_unchanged()
            .map(|()| self.receipts.get(population_id).copied());
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Execution V3 lookup lock: {why}"));
        combine_lock_result(result, released)
    }

    fn require_exact_prepared(
        &mut self,
        receipt: ExecutionV3StructuralReceipt,
        prepared: &PreparedExecutionV3,
    ) -> Result<(), ExecutionV3Refusal> {
        let parameters =
            self.read_parameters(receipt.first_parameter_record, receipt.parameter_count)?;
        let percentiles =
            self.read_percentiles(receipt.first_percentile_record, receipt.percentile_count)?;
        let dispositions =
            self.read_dispositions(receipt.first_disposition_record, receipt.disposition_count)?;
        let completion = ExecutionV3CompletionRecord::decode(&read_fixed_at(
            &mut self.completions.file,
            receipt.block_sequence,
            EXECUTION_V3_COMPLETION_BYTES,
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
            return Err("Execution V3 fresh reopen differs from prepared bytes".to_owned());
        }
        self.require_unchanged()
    }

    fn authenticated_disposition(
        &mut self,
        receipt: ExecutionV3StructuralReceipt,
        global_sequence: u64,
    ) -> Result<ExecutionV3SuccessorDisposition, ExecutionV3Refusal> {
        if global_sequence >= receipt.disposition_count {
            return Err(format!(
                "Execution V3 disposition {global_sequence} is outside authenticated count {}",
                receipt.disposition_count
            ));
        }
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take Execution V3 disposition lock: {why}"))?;
        let result = (|| {
            self.require_unchanged()?;
            if self.receipts.get(&receipt.population_id) != Some(&receipt) {
                return Err("Execution V3 receipt is no longer indexed exactly".to_owned());
            }
            let physical = checked_end(
                receipt.first_disposition_record,
                global_sequence,
                "authenticated disposition offset",
            )?;
            let raw = read_fixed_at(
                &mut self.dispositions.file,
                physical,
                EXECUTION_V3_DISPOSITION_BYTES,
                "authenticated disposition",
            )?;
            let disposition = ExecutionV3SuccessorDisposition::authenticate(&raw)?;
            if disposition.population_id() != receipt.population_id
                || disposition.global_sequence() != global_sequence
            {
                return Err(
                    "Execution V3 fixed-offset disposition violates authenticated order".to_owned(),
                );
            }
            self.require_unchanged()?;
            Ok(disposition)
        })();
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Execution V3 disposition lock: {why}"));
        combine_lock_result(result, released)
    }

    fn authenticated_dispositions(
        &mut self,
        receipt: ExecutionV3StructuralReceipt,
    ) -> Result<Vec<ExecutionV3SuccessorDisposition>, ExecutionV3Refusal> {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take Execution V3 bulk lock: {why}"))?;
        let result = (|| {
            self.require_unchanged()?;
            if self.receipts.get(&receipt.population_id) != Some(&receipt) {
                return Err("Execution V3 bulk receipt is no longer indexed exactly".to_owned());
            }
            let completion = ExecutionV3CompletionRecord::decode(&read_fixed_at(
                &mut self.completions.file,
                receipt.block_sequence,
                EXECUTION_V3_COMPLETION_BYTES,
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
                return Err("Execution V3 bulk block differs from exact receipt".to_owned());
            }
            let mut authenticated = bounded_vec(
                receipt.disposition_count,
                self.bounds.dispositions_per_block,
                "authenticated dispositions",
            )?;
            for disposition in dispositions {
                let encoded = disposition.encode()?;
                authenticated.push(ExecutionV3SuccessorDisposition::authenticate(&encoded)?);
            }
            self.require_unchanged()?;
            Ok(authenticated)
        })();
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Execution V3 bulk lock: {why}"));
        combine_lock_result(result, released)
    }

    fn require_unchanged(&self) -> Result<(), ExecutionV3Refusal> {
        if named_identity(&self.root)? != self.root_identity
            || file_generation(&self.lock_file, &self.lock_path, LOCK_MAX_BYTES)?
                != self.lock_generation
        {
            return Err("Execution V3 retained root or lock generation changed".to_owned());
        }
        self.parameters.require_unchanged()?;
        self.percentiles.require_unchanged()?;
        self.dispositions.require_unchanged()?;
        self.completions.require_unchanged()?;
        Ok(())
    }
}

/// Freshly reopened V3 authority. A future production wrapper must retain the
/// live Population V5 source beside this ledger.
pub(crate) struct ExecutionV3Authority {
    receipt: ExecutionV3StructuralReceipt,
    ledger: ExecutionV3Ledger,
}

impl ExecutionV3Authority {
    #[must_use]
    pub(crate) const fn structural_receipt(&self) -> ExecutionV3StructuralReceipt {
        self.receipt
    }

    pub(crate) fn authenticated_disposition(
        &mut self,
        global_sequence: u64,
    ) -> Result<ExecutionV3SuccessorDisposition, ExecutionV3Refusal> {
        self.ledger
            .authenticated_disposition(self.receipt, global_sequence)
    }

    pub(crate) fn ordered_authenticated_dispositions(
        &mut self,
    ) -> Result<Vec<ExecutionV3SuccessorDisposition>, ExecutionV3Refusal> {
        self.ledger.authenticated_dispositions(self.receipt)
    }
}

/// Immutable terminal projection returned only by a retained Execution V3
/// authority. It has no public constructor and never decodes Population V5.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExecutionV3SuccessorDisposition {
    canonical_record: [u8; EXECUTION_V3_DISPOSITION_BYTES],
    record: ExecutionV3DispositionRecord,
}

impl ExecutionV3SuccessorDisposition {
    fn authenticate(
        canonical_record: &[u8; EXECUTION_V3_DISPOSITION_BYTES],
    ) -> Result<Self, ExecutionV3Refusal> {
        let record = ExecutionV3DispositionRecord::decode(canonical_record)?;
        Ok(Self {
            canonical_record: *canonical_record,
            record,
        })
    }

    #[must_use]
    pub(crate) const fn canonical_record(&self) -> &[u8; EXECUTION_V3_DISPOSITION_BYTES] {
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
    pub(crate) const fn admission_decision_id(&self) -> [u8; 32] {
        self.record.admission_decision_id
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
    pub(crate) const fn family(&self) -> ExecutionV3Family {
        self.record.family
    }

    #[must_use]
    pub(crate) const fn direction(&self) -> ExecutionV3Direction {
        self.record.direction
    }

    #[must_use]
    pub(crate) const fn admission_status(&self) -> ExecutionV3AdmissionStatus {
        self.record.admission_status
    }

    #[must_use]
    pub(crate) const fn terminal(&self) -> ExecutionV3Terminal {
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

enum ExecutionV3ProductionCommit {
    Written(ExecutionV3Authority),
    Reused(ExecutionV3Authority),
}

impl ExecutionV3ProductionCommit {
    const fn was_written(&self) -> bool {
        matches!(self, Self::Written(_))
    }

    const fn authority(&self) -> &ExecutionV3Authority {
        match self {
            Self::Written(authority) | Self::Reused(authority) => authority,
        }
    }

    fn authority_mut(&mut self) -> &mut ExecutionV3Authority {
        match self {
            Self::Written(authority) | Self::Reused(authority) => authority,
        }
    }
}

/// Nonconstructible stored Execution V3 capability retaining its complete live
/// Population V5 authority. Selection successors can reauthenticate Population
/// strategy/mask/ranking facts through the retained source; Execution V3 does
/// not silently widen layout 5 by copying those facts into its record.
pub(crate) struct CommittedStoredExecutionV3 {
    source: CommittedStoredPopulationV5,
    execution: ExecutionV3ProductionCommit,
}

impl CommittedStoredExecutionV3 {
    #[must_use]
    pub(crate) const fn was_written(&self) -> bool {
        self.execution.was_written()
    }

    #[must_use]
    pub(crate) const fn structural_receipt(&self) -> ExecutionV3StructuralReceipt {
        self.execution.authority().structural_receipt()
    }

    #[must_use]
    pub(crate) const fn bounds(&self) -> ExecutionV3Bounds {
        self.execution.authority().ledger.bounds
    }

    /// Reauthenticates the complete live Population V5 source for a successor
    /// without exposing raw bars, grids, masks, IDs, or detached metrics.
    pub(crate) fn population_execution_source(
        &mut self,
    ) -> Result<PopulationV5ExecutionV3SourceV1, ExecutionV3Refusal> {
        self.source
            .execution_v3_source()
            .map_err(|why| format!("Execution V3 retained Population source refused: {why}"))
    }

    /// Reauthenticates Population before and after a full durable disposition
    /// read and requires every reopened byte to equal the freshly derived
    /// source record.
    pub(crate) fn ordered_authenticated_dispositions(
        &mut self,
    ) -> Result<Vec<ExecutionV3SuccessorDisposition>, ExecutionV3Refusal> {
        let bounds = self.execution.authority_mut().ledger.bounds;
        let source_before = self.population_execution_source()?;
        let prepared = PreparedExecutionV3::from_population_source(&source_before, bounds)?;
        let rows = self
            .execution
            .authority_mut()
            .ordered_authenticated_dispositions()?;
        if rows.len() != prepared.dispositions.len() {
            return Err(
                "Execution V3 durable rows differ from the reauthenticated Population source"
                    .to_owned(),
            );
        }
        for (row, expected) in rows.iter().zip(&prepared.dispositions) {
            if row.canonical_record() != &expected.encode()? {
                return Err(
                    "Execution V3 durable rows differ from the reauthenticated Population source"
                        .to_owned(),
                );
            }
        }
        let source_after = self.population_execution_source()?;
        if source_after != source_before {
            return Err(
                "Execution V3 retained Population source changed during durable read".to_owned(),
            );
        }
        Ok(rows)
    }
}

/// The sole production Execution V3 commit door.
///
/// It consumes the nonconstructible stored Population V5 capability, derives
/// every parameter/percentile/disposition from its retained source, writes and
/// freshly reopens layout 5, then reauthenticates the complete Population
/// source after persistence. There is no detached preparation overload.
pub(crate) fn commit_stored_execution_v3(
    root: &Path,
    bounds: ExecutionV3Bounds,
    mut source: CommittedStoredPopulationV5,
) -> Result<CommittedStoredExecutionV3, ExecutionV3Refusal> {
    let source_before = source
        .execution_v3_source()
        .map_err(|why| format!("Execution V3 Population source refused: {why}"))?;
    let prepared = PreparedExecutionV3::from_population_source(&source_before, bounds)?;
    let execution = persist_prepared(root, bounds, &prepared)?;
    let source_after = source
        .execution_v3_source()
        .map_err(|why| format!("Execution V3 post-commit Population source refused: {why}"))?;
    if source_after != source_before
        || PreparedExecutionV3::from_population_source(&source_after, bounds)? != prepared
    {
        return Err(
            "Execution V3 Population source changed during persistence and fresh reopen".to_owned(),
        );
    }
    Ok(CommittedStoredExecutionV3 { source, execution })
}

fn persist_prepared(
    root: &Path,
    bounds: ExecutionV3Bounds,
    prepared: &PreparedExecutionV3,
) -> Result<ExecutionV3ProductionCommit, ExecutionV3Refusal> {
    let structural = {
        let mut writer = ExecutionV3Ledger::open_write(root, bounds)?;
        writer.append(prepared)?
    };
    let writer_receipt = structural.receipt();
    let mut reader = ExecutionV3Ledger::open_read(root, bounds)?;
    let reopened = reader
        .structural_receipt(&prepared.population_id)?
        .ok_or_else(|| "Execution V3 fresh reopen did not find committed identity".to_owned())?;
    if reopened != writer_receipt {
        return Err("Execution V3 fresh reopen receipt differs from writer receipt".to_owned());
    }
    reader.require_exact_prepared(reopened, prepared)?;
    let authority = ExecutionV3Authority {
        receipt: reopened,
        ledger: reader,
    };
    if structural.was_written() {
        Ok(ExecutionV3ProductionCommit::Written(authority))
    } else {
        Ok(ExecutionV3ProductionCommit::Reused(authority))
    }
}

#[cfg(test)]
fn commit_prepared_for_test(
    root: &Path,
    bounds: ExecutionV3Bounds,
    prepared: &PreparedExecutionV3,
) -> Result<ExecutionV3ProductionCommit, ExecutionV3Refusal> {
    persist_prepared(root, bounds, prepared)
}

fn validate_complete_block(
    parameters: &[ExecutionV3ParameterRecord],
    percentiles: &[ExecutionV3PercentileRecord],
    dispositions: &[ExecutionV3DispositionRecord],
    completion: &ExecutionV3CompletionRecord,
    bounds: ExecutionV3Bounds,
) -> Result<ExecutionV3StructuralReceipt, ExecutionV3Refusal> {
    completion.validate()?;
    let parameter_array: [ExecutionV3ParameterRecord; PARAMETER_COUNT_PER_BLOCK] = parameters
        .to_vec()
        .try_into()
        .map_err(|values: Vec<ExecutionV3ParameterRecord>| {
            format!(
                "Execution V3 Completion requires four parameters, decoded {}",
                values.len()
            )
        })?;
    let prepared = PreparedExecutionV3 {
        population_id: completion.population_id,
        population_ordered_digest: completion.population_ordered_digest,
        source_finalization_id: completion.source_finalization_id,
        source_finalization_completion_id: completion.source_finalization_completion_id,
        source_admission_block_id: completion.source_admission_block_id,
        source_admission_completion_id: completion.source_admission_completion_id,
        parameters: parameter_array,
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
            "Execution V3 Completion does not exactly bind its fixed-record block".to_owned(),
        );
    }
    Ok(ExecutionV3StructuralReceipt::from_completion(completion))
}

fn validate_trailing_prefix(
    parameters: &[ExecutionV3ParameterRecord],
    percentiles: &[ExecutionV3PercentileRecord],
    dispositions: &[ExecutionV3DispositionRecord],
    bounds: ExecutionV3Bounds,
) -> Result<(), ExecutionV3Refusal> {
    if parameters.is_empty() {
        if percentiles.is_empty() && dispositions.is_empty() {
            return Ok(());
        }
        return Err(
            "Execution V3 orphan has percentile/disposition records without parameters".to_owned(),
        );
    }
    if parameters.len() > PARAMETER_COUNT_PER_BLOCK {
        return Err("Execution V3 orphan has more than four parameter records".to_owned());
    }
    require_block_counts(bounds, percentiles.len().max(1), dispositions.len().max(1))?;
    let expected_order = [
        (ExecutionV3Family::Nifty, ExecutionV3Direction::Long),
        (ExecutionV3Family::Nifty, ExecutionV3Direction::Short),
        (ExecutionV3Family::BankNifty, ExecutionV3Direction::Long),
        (ExecutionV3Family::BankNifty, ExecutionV3Direction::Short),
    ];
    let first = parameters
        .first()
        .ok_or_else(|| "Execution V3 orphan parameter prefix is empty".to_owned())?;
    let mut expected_offset = 0_u64;
    for (parameter, (family, direction)) in parameters.iter().zip(expected_order) {
        parameter.validate()?;
        if parameter.family != family
            || parameter.direction != direction
            || parameter.population_id != first.population_id
            || parameter.population_ordered_digest != first.population_ordered_digest
            || parameter.source_finalization_id != first.source_finalization_id
            || parameter.source_finalization_completion_id
                != first.source_finalization_completion_id
            || parameter.execution_law_digest != first.execution_law_digest
            || parameter.rung != first.rung
            || parameter.percentile_offset != expected_offset
        {
            return Err("Execution V3 orphan parameter prefix is not canonical".to_owned());
        }
        expected_offset = checked_end(
            expected_offset,
            parameter.percentile_count,
            "orphan percentile range",
        )?;
    }
    if parameters.len() < PARAMETER_COUNT_PER_BLOCK {
        if !percentiles.is_empty() || !dispositions.is_empty() {
            return Err(
                "Execution V3 orphan wrote a later file before all four parameters".to_owned(),
            );
        }
        return Ok(());
    }
    let expected_percentiles = usize::try_from(expected_offset)
        .map_err(|_| "Execution V3 orphan percentile count does not fit usize".to_owned())?;
    if percentiles.len() > expected_percentiles {
        return Err("Execution V3 orphan has excess percentile records".to_owned());
    }
    validate_percentile_prefix(parameters, percentiles)?;
    if percentiles.len() < expected_percentiles {
        if !dispositions.is_empty() {
            return Err(
                "Execution V3 orphan wrote dispositions before all percentile atoms".to_owned(),
            );
        }
        return Ok(());
    }
    validate_disposition_prefix(parameters, percentiles, dispositions, first.population_id)?;
    Ok(())
}

fn validate_percentile_segment(
    parameter: &ExecutionV3ParameterRecord,
    segment: &[ExecutionV3PercentileRecord],
) -> Result<(), ExecutionV3Refusal> {
    if segment.is_empty()
        || u64::try_from(segment.len())
            .map_err(|_| "Execution V3 percentile segment length does not fit u64".to_owned())?
            != parameter.percentile_count
    {
        return Err("Execution V3 parameter has an empty/incomplete percentile segment".to_owned());
    }
    let axis_counts = validate_one_percentile_prefix(parameter, segment)?;
    if axis_counts.into_iter().any(|count| count == 0) {
        return Err(
            "Execution V3 parameter percentile schedule does not cover Stop, Target, and Trail"
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
        .ok_or_else(|| "Execution V3 forced-stop schedule ceiling overflowed".to_owned())?;
    if stop_schedule_count > parameter.max_levels
        || target_schedule_count > parameter.max_levels
        || trail_schedule_count > parameter.max_levels
        || parameter.resolved_stop_count > stop_schedule_ceiling
        || parameter.resolved_target_count > target_schedule_count
        || parameter.resolved_trail_count > trail_schedule_count
    {
        return Err(
            "Execution V3 percentile schedules/resolved axes exceed policy or forced-stop bounds"
                .to_owned(),
        );
    }
    if ordered_percentile_digest(segment)? != parameter.percentile_digest {
        return Err("Execution V3 parameter percentile digest does not reproduce".to_owned());
    }
    Ok(())
}

fn validate_percentile_prefix(
    parameters: &[ExecutionV3ParameterRecord],
    percentiles: &[ExecutionV3PercentileRecord],
) -> Result<(), ExecutionV3Refusal> {
    for parameter in parameters {
        let start = usize::try_from(parameter.percentile_offset)
            .map_err(|_| "Execution V3 percentile offset does not fit usize".to_owned())?;
        if start >= percentiles.len() {
            break;
        }
        let declared_end = checked_end(
            parameter.percentile_offset,
            parameter.percentile_count,
            "percentile prefix range",
        )?;
        let end = usize::try_from(declared_end)
            .map_err(|_| "Execution V3 percentile prefix end does not fit usize".to_owned())?
            .min(percentiles.len());
        let segment = percentiles.get(start..end).ok_or_else(|| {
            "Execution V3 percentile prefix segment escaped prepared atoms".to_owned()
        })?;
        validate_one_percentile_prefix(parameter, segment)?;
        if end
            == usize::try_from(declared_end)
                .map_err(|_| "Execution V3 percentile prefix end does not fit usize".to_owned())?
            && ordered_percentile_digest(segment)? != parameter.percentile_digest
        {
            return Err("Execution V3 completed orphan percentile digest differs".to_owned());
        }
    }
    Ok(())
}

fn validate_one_percentile_prefix(
    parameter: &ExecutionV3ParameterRecord,
    segment: &[ExecutionV3PercentileRecord],
) -> Result<[u64; 3], ExecutionV3Refusal> {
    let mut previous: Option<&ExecutionV3PercentileRecord> = None;
    let mut axis_counts = [0_u64; 3];
    for percentile in segment {
        percentile.validate()?;
        if percentile.parameter_core_id != parameter.parameter_core_id {
            return Err("Execution V3 percentile atom changes parameter identity".to_owned());
        }
        let axis_index = percentile_axis_index(percentile.axis);
        let axis_count = axis_counts
            .get(axis_index)
            .copied()
            .ok_or_else(|| "Execution V3 percentile axis escaped schema".to_owned())?;
        if percentile.ordinal
            != u32::try_from(axis_count)
                .map_err(|_| "Execution V3 percentile axis count does not fit u32".to_owned())?
        {
            return Err(
                "Execution V3 percentile ordinals are not contiguous from zero per axis".to_owned(),
            );
        }
        match previous {
            None if percentile.axis != ExecutionV3PercentileAxis::Stop => {
                return Err("Execution V3 percentile schedule does not begin with Stop".to_owned());
            }
            Some(prior) if percentile.axis == prior.axis => {
                let left = u64::from(percentile.numerator) * u64::from(prior.denominator);
                let right = u64::from(prior.numerator) * u64::from(percentile.denominator);
                if left <= right {
                    return Err(
                        "Execution V3 percentile values are not strictly increasing within an axis"
                            .to_owned(),
                    );
                }
            }
            Some(prior)
                if percentile_axis_index(percentile.axis)
                    != percentile_axis_index(prior.axis) + 1 =>
            {
                return Err(
                    "Execution V3 percentile axes are missing or not canonically ordered"
                        .to_owned(),
                );
            }
            None | Some(_) => {}
        }
        let axis_count = axis_counts
            .get_mut(axis_index)
            .ok_or_else(|| "Execution V3 percentile axis escaped schema".to_owned())?;
        increment(axis_count, "percentile axis count")?;
        previous = Some(percentile);
    }
    Ok(axis_counts)
}

const fn percentile_axis_index(axis: ExecutionV3PercentileAxis) -> usize {
    axis as usize - 1
}

fn validate_disposition_block(prepared: &PreparedExecutionV3) -> Result<(), ExecutionV3Refusal> {
    validate_disposition_prefix(
        &prepared.parameters,
        &prepared.percentiles,
        &prepared.dispositions,
        prepared.population_id,
    )?;
    if prepared.dispositions.is_empty()
        || !prepared
            .dispositions
            .iter()
            .any(|row| row.family == ExecutionV3Family::Nifty)
        || !prepared
            .dispositions
            .iter()
            .any(|row| row.family == ExecutionV3Family::BankNifty)
    {
        return Err(
            "Execution V3 block must cover both NIFTY and BANKNIFTY dispositions".to_owned(),
        );
    }
    Ok(())
}

fn validate_disposition_prefix(
    parameters: &[ExecutionV3ParameterRecord],
    percentiles: &[ExecutionV3PercentileRecord],
    dispositions: &[ExecutionV3DispositionRecord],
    population_id: [u8; 32],
) -> Result<(), ExecutionV3Refusal> {
    if parameters.len() != PARAMETER_COUNT_PER_BLOCK {
        if dispositions.is_empty() {
            return Ok(());
        }
        return Err("Execution V3 dispositions require four parameter records".to_owned());
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
    fn with_capacity(count: usize) -> Result<Self, ExecutionV3Refusal> {
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
            (&mut value.population_rows, "Population V5 rows"),
            (&mut value.candidates, "Candidates"),
            (&mut value.candidate_base_rows, "Candidate base rows"),
            (&mut value.admissions, "Admission V3 decisions"),
            (&mut value.finalizations, "Finalization V3 rows"),
        ] {
            set.try_reserve(count)
                .map_err(|why| format!("cannot reserve Execution V3 {name}: {why}"))?;
        }
        Ok(value)
    }

    fn insert(&mut self, row: &ExecutionV3DispositionRecord) -> bool {
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
    fn observe(&mut self, row: &ExecutionV3DispositionRecord) -> Result<(), ExecutionV3Refusal> {
        match row.family {
            ExecutionV3Family::Nifty if !self.banknifty_started => {
                if row.family_sequence != self.nifty {
                    return Err("Execution V3 NIFTY sequence is not contiguous".to_owned());
                }
                increment(&mut self.nifty, "NIFTY sequence")
            }
            ExecutionV3Family::BankNifty => {
                self.banknifty_started = true;
                if row.family_sequence != self.banknifty {
                    return Err("Execution V3 BANKNIFTY sequence is not contiguous".to_owned());
                }
                increment(&mut self.banknifty, "BANKNIFTY sequence")
            }
            ExecutionV3Family::Nifty => {
                Err("Execution V3 NIFTY disposition follows BANKNIFTY".to_owned())
            }
        }
    }
}

fn disposition_schedule_counts(
    parameters: &[ExecutionV3ParameterRecord],
    percentiles: &[ExecutionV3PercentileRecord],
) -> Result<[[u64; 3]; PARAMETER_COUNT_PER_BLOCK], ExecutionV3Refusal> {
    let common_rung = parameters
        .first()
        .ok_or_else(|| "Execution V3 parameter block is empty".to_owned())?
        .rung;
    let mut counts = [[0_u64; 3]; PARAMETER_COUNT_PER_BLOCK];
    for (index, parameter) in parameters.iter().enumerate() {
        if parameter.rung != common_rung {
            return Err("Execution V3 parameter block mixes signal rungs".to_owned());
        }
        let start = usize::try_from(parameter.percentile_offset)
            .map_err(|_| "Execution V3 percentile start does not fit usize".to_owned())?;
        let end = usize::try_from(checked_end(
            parameter.percentile_offset,
            parameter.percentile_count,
            "disposition percentile schedule",
        )?)
        .map_err(|_| "Execution V3 percentile end does not fit usize".to_owned())?;
        let segment = percentiles.get(start..end).ok_or_else(|| {
            "Execution V3 disposition parameter schedule exceeds percentile block".to_owned()
        })?;
        validate_percentile_segment(parameter, segment)?;
        let slot = counts
            .get_mut(index)
            .ok_or_else(|| "Execution V3 parameter schedule index escaped schema".to_owned())?;
        *slot = validate_one_percentile_prefix(parameter, segment)?;
    }
    Ok(counts)
}

fn validate_one_disposition_prefix(
    index: usize,
    row: &ExecutionV3DispositionRecord,
    population_id: [u8; 32],
    finalization_completion: Option<[u8; 32]>,
    parameters: &[ExecutionV3ParameterRecord],
    identities: &mut DispositionIdentitySets,
    sequences: &mut DispositionSequences,
) -> Result<(), ExecutionV3Refusal> {
    row.validate()?;
    if row.population_id != population_id
        || row.global_sequence != usize_to_u64(index, "disposition ordinal")?
        || Some(row.finalization_completion_id) != finalization_completion
        || !identities.insert(row)
    {
        return Err(
            "Execution V3 disposition source/order/unique identities are inconsistent".to_owned(),
        );
    }
    sequences.observe(row)?;
    let parameter_index = parameter_index(row.family, row.direction);
    let parameter = parameters
        .get(parameter_index)
        .ok_or_else(|| "Execution V3 family/direction parameter index escaped schema".to_owned())?;
    require_disposition_parameter_join(row, parameter)?;
    require_resolved_coordinate_bounds(row, parameter)
}

const fn parameter_index(family: ExecutionV3Family, direction: ExecutionV3Direction) -> usize {
    match (family, direction) {
        (ExecutionV3Family::Nifty, ExecutionV3Direction::Long) => 0,
        (ExecutionV3Family::Nifty, ExecutionV3Direction::Short) => 1,
        (ExecutionV3Family::BankNifty, ExecutionV3Direction::Long) => 2,
        (ExecutionV3Family::BankNifty, ExecutionV3Direction::Short) => 3,
    }
}

fn require_disposition_parameter_join(
    row: &ExecutionV3DispositionRecord,
    parameter: &ExecutionV3ParameterRecord,
) -> Result<(), ExecutionV3Refusal> {
    if row.parameter_id != parameter.parameter_id
        || row.resolution_digest != parameter.resolution_digest
        || row.rung != parameter.rung
        || row.horizon_bars != parameter.horizon_bars
        || row.cell_ordinal >= parameter.resolved_cell_count
    {
        return Err(
            "Execution V3 disposition does not join its exact family/direction parameter"
                .to_owned(),
        );
    }
    if parameter.forced_stop_policy_tag == FORCED_STOP_REQUIRE_TAG
        && row.terminal == ExecutionV3Terminal::Authorized
        && row.stop_index != parameter.forced_stop_index
    {
        return Err(
            "Execution V3 Authorized coordinate violates its required forced stop".to_owned(),
        );
    }
    Ok(())
}

fn require_resolved_coordinate_bounds(
    row: &ExecutionV3DispositionRecord,
    parameter: &ExecutionV3ParameterRecord,
) -> Result<(), ExecutionV3Refusal> {
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
                "Execution V3 {name} index {coordinate:?} is outside its {count}-level resolved axis"
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
            "Execution V3 TTP arm/trail is not strictly below its target/TSL cap".to_owned(),
        );
    }
    // Cardinalities cannot authenticate which stop-target pairs Runner admitted.
    // The production join must retain the full ResolvedExitGridV1 and consume
    // classify_coordinate; this fixed record only refuses impossible bounds.
    Ok(())
}

fn ordered_parameter_digest(
    records: &[ExecutionV3ParameterRecord; PARAMETER_COUNT_PER_BLOCK],
) -> Result<[u8; 32], ExecutionV3Refusal> {
    let mut hasher = begin_ordered_digest(ORDERED_PARAMETERS_DOMAIN, records.len())?;
    for record in records {
        hasher.update(&record.encode()?);
    }
    Ok(hasher.finalize())
}

fn ordered_percentile_digest(
    records: &[ExecutionV3PercentileRecord],
) -> Result<[u8; 32], ExecutionV3Refusal> {
    let mut hasher = begin_ordered_digest(ORDERED_PERCENTILES_DOMAIN, records.len())?;
    for record in records {
        hasher.update(&record.encode()?);
    }
    Ok(hasher.finalize())
}

fn ordered_disposition_digest(
    records: &[ExecutionV3DispositionRecord],
) -> Result<[u8; 32], ExecutionV3Refusal> {
    let mut hasher = begin_ordered_digest(ORDERED_DISPOSITIONS_DOMAIN, records.len())?;
    for record in records {
        hasher.update(&record.encode()?);
    }
    Ok(hasher.finalize())
}

fn family_disposition_digest(
    family: ExecutionV3Family,
    records: &[ExecutionV3DispositionRecord],
) -> Result<[u8; 32], ExecutionV3Refusal> {
    let count = records.iter().filter(|row| row.family == family).count();
    let mut hasher = begin_ordered_digest(ORDERED_DISPOSITIONS_DOMAIN, count)?;
    hasher.update(&[family as u8]);
    for record in records.iter().filter(|row| row.family == family) {
        hasher.update(&record.encode()?);
    }
    Ok(hasher.finalize())
}

fn begin_ordered_digest(domain: &[u8], count: usize) -> Result<Hasher, ExecutionV3Refusal> {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    hasher.update(&VERSION.to_le_bytes());
    hasher.update(&usize_to_u64(count, "ordered digest count")?.to_le_bytes());
    Ok(hasher)
}

fn derive_family_authority_id(
    family: ExecutionV3Family,
    prepared: &PreparedExecutionV3,
    parameter_ids: [[u8; 32]; 2],
    disposition_digest: [u8; 32],
    row_count: u64,
    matrix: &[u64],
) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(FAMILY_AUTHORITY_ID_DOMAIN);
    hasher.update(&VERSION.to_le_bytes());
    hasher.update(&[family as u8]);
    let [long_parameter_id, short_parameter_id] = parameter_ids;
    for value in [
        prepared.population_id,
        prepared.population_ordered_digest,
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
    hasher.update(&row_count.to_le_bytes());
    for value in matrix {
        hasher.update(&value.to_le_bytes());
    }
    hasher.finalize()
}

fn execution_law_digest() -> [u8; 32] {
    crate::execution_capability::exact_execution_law_digest_v1()
}

fn matrix_index(
    family: ExecutionV3Family,
    status: ExecutionV3AdmissionStatus,
    terminal: ExecutionV3Terminal,
) -> usize {
    let family_offset = match family {
        ExecutionV3Family::Nifty => 0,
        ExecutionV3Family::BankNifty => 8,
    };
    family_offset + admission_index(status) * 2 + terminal_index(terminal)
}

const fn admission_index(status: ExecutionV3AdmissionStatus) -> usize {
    status as usize - 1
}

const fn terminal_index(terminal: ExecutionV3Terminal) -> usize {
    terminal as usize - 1
}

fn require_block_counts(
    bounds: ExecutionV3Bounds,
    percentiles: usize,
    dispositions: usize,
) -> Result<(), ExecutionV3Refusal> {
    let percentiles = usize_to_u64(percentiles, "percentile block count")?;
    let dispositions = usize_to_u64(dispositions, "disposition block count")?;
    if percentiles == 0
        || dispositions == 0
        || percentiles > bounds.percentiles_per_block
        || dispositions > bounds.dispositions_per_block
    {
        return Err(format!(
            "Execution V3 block counts {percentiles} percentiles/{dispositions} dispositions exceed explicit nonzero bounds {}/{}",
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
) -> Result<(PathBuf, File, PlatformIdentity), ExecutionV3Refusal> {
    require_not_symlink(root, false)?;
    let canonical = std::fs::canonicalize(root).map_err(|why| {
        format!(
            "Execution V3 root {} must already exist: {why}",
            root.display()
        )
    })?;
    require_not_symlink(&canonical, false)?;
    let file = File::open(&canonical).map_err(|why| {
        format!(
            "cannot hold Execution V3 root {}: {why}",
            canonical.display()
        )
    })?;
    let metadata = file.metadata().map_err(|why| {
        format!(
            "cannot stat held Execution V3 root {}: {why}",
            canonical.display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "Execution V3 root {} is not a directory",
            canonical.display()
        ));
    }
    let identity = PlatformIdentity::of(&metadata);
    if named_identity(&canonical)? != identity {
        return Err("Execution V3 root changed while it was opened".to_owned());
    }
    Ok((canonical, file, identity))
}

fn named_identity(path: &Path) -> Result<PlatformIdentity, ExecutionV3Refusal> {
    require_not_symlink(path, false)?;
    let metadata = std::fs::metadata(path).map_err(|why| {
        format!(
            "cannot stat named Execution V3 path {}: {why}",
            path.display()
        )
    })?;
    Ok(PlatformIdentity::of(&metadata))
}

fn open_child(
    path: &Path,
    writable: bool,
    create: bool,
) -> Result<(File, bool), ExecutionV3Refusal> {
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
                    "cannot create Execution V3 file {}: {why}",
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
        .map_err(|why| format!("cannot open Execution V3 file {}: {why}", path.display()))?;
    require_not_symlink(path, false)?;
    require_regular_file(&file, path)?;
    Ok((file, false))
}

fn require_regular_file(file: &File, path: &Path) -> Result<(), ExecutionV3Refusal> {
    let metadata = file
        .metadata()
        .map_err(|why| format!("cannot stat Execution V3 file {}: {why}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!(
            "Execution V3 path {} is not a regular file",
            path.display()
        ));
    }
    require_single_link(&metadata, path)
}

fn require_single_link(
    metadata: &std::fs::Metadata,
    path: &Path,
) -> Result<(), ExecutionV3Refusal> {
    #[cfg(unix)]
    if metadata.nlink() != 1 {
        return Err(format!(
            "Execution V3 path {} has {} hard links; expected exactly one",
            path.display(),
            metadata.nlink()
        ));
    }
    #[cfg(not(unix))]
    let _ = (metadata, path);
    Ok(())
}

fn require_not_symlink(path: &Path, absent_allowed: bool) -> Result<(), ExecutionV3Refusal> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(format!(
            "Execution V3 path {} is a symbolic link; no-follow refused it",
            path.display()
        )),
        Ok(_) => Ok(()),
        Err(why) if absent_allowed && why.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(why) => Err(format!(
            "cannot inspect Execution V3 path {} without following links: {why}",
            path.display()
        )),
    }
}

fn sync_directory(root_file: &File, root: &Path) -> Result<(), ExecutionV3Refusal> {
    root_file.sync_all().map_err(|why| {
        format!(
            "cannot durably sync Execution V3 directory {}: {why}",
            root.display()
        )
    })
}

fn file_generation(
    file: &File,
    path: &Path,
    max_bytes: u64,
) -> Result<FileGeneration, ExecutionV3Refusal> {
    let before = file.metadata().map_err(|why| {
        format!(
            "cannot stat held Execution V3 file {}: {why}",
            path.display()
        )
    })?;
    if !before.is_file() || before.len() > max_bytes {
        return Err(format!(
            "Execution V3 file {} is not regular or its {} bytes exceed bound {max_bytes}",
            path.display(),
            before.len()
        ));
    }
    require_single_link(&before, path)?;
    let platform = PlatformIdentity::of(&before);
    if named_identity(path)? != platform {
        return Err(format!(
            "Execution V3 file {} was path-replaced",
            path.display()
        ));
    }
    let len = before.len();
    let modified = before.modified().ok();
    let first = hash_held_file(file, path, len)?;
    let middle = file.metadata().map_err(|why| {
        format!(
            "cannot restat Execution V3 file {} after first hash: {why}",
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
            "Execution V3 file {} changed during generation hash",
            path.display()
        ));
    }
    let second = hash_held_file(file, path, len)?;
    let after = file.metadata().map_err(|why| {
        format!(
            "cannot restat Execution V3 file {} after confirmation hash: {why}",
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
            "Execution V3 file {} changed between bounded generation hashes",
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

fn hash_held_file(file: &File, path: &Path, len: u64) -> Result<[u8; 32], ExecutionV3Refusal> {
    let mut reader = file
        .try_clone()
        .map_err(|why| format!("cannot clone Execution V3 file {}: {why}", path.display()))?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|why| format!("cannot seek Execution V3 file {}: {why}", path.display()))?;
    let mut hasher = Hasher::new();
    hasher.update(FILE_GENERATION_DOMAIN);
    hasher.update(&len.to_le_bytes());
    let mut remaining = len;
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    while remaining != 0 {
        let requested = usize::try_from(remaining.min(READ_CHUNK_BYTES as u64))
            .map_err(|_| "Execution V3 hash width does not fit usize".to_owned())?;
        let requested_buffer = buffer
            .get_mut(..requested)
            .ok_or_else(|| "Execution V3 hash request exceeds fixed buffer".to_owned())?;
        let read = reader
            .read(requested_buffer)
            .map_err(|why| format!("cannot hash Execution V3 file {}: {why}", path.display()))?;
        if read == 0 {
            return Err(format!(
                "Execution V3 file {} shortened while hashing",
                path.display()
            ));
        }
        let read_buffer = buffer
            .get(..read)
            .ok_or_else(|| "Execution V3 hash read exceeds fixed buffer".to_owned())?;
        hasher.update(read_buffer);
        remaining = remaining
            .checked_sub(usize_to_u64(read, "hash read")?)
            .ok_or_else(|| "Execution V3 hash remaining count underflowed".to_owned())?;
    }
    Ok(hasher.finalize())
}

fn checked_record_count(
    bytes: u64,
    stride: usize,
    max_records: u64,
    name: &str,
) -> Result<u64, ExecutionV3Refusal> {
    let stride = u64::try_from(stride)
        .map_err(|_| format!("Execution V3 {name} stride does not fit u64"))?;
    if !bytes.is_multiple_of(stride) {
        return Err(format!(
            "Execution V3 {name} file has ragged length {bytes}, not a multiple of {stride}"
        ));
    }
    let records = bytes / stride;
    if records > max_records {
        return Err(format!(
            "Execution V3 {name} file has {records} records above bound {max_records}"
        ));
    }
    Ok(records)
}

fn read_fixed_at<const N: usize>(
    file: &mut File,
    physical: u64,
    stride: usize,
    name: &str,
) -> Result<[u8; N], ExecutionV3Refusal> {
    if N != stride {
        return Err(format!(
            "Execution V3 {name} buffer {N} differs from stride {stride}"
        ));
    }
    let stride = u64::try_from(stride)
        .map_err(|_| format!("Execution V3 {name} stride does not fit u64"))?;
    let offset = physical
        .checked_mul(stride)
        .ok_or_else(|| format!("Execution V3 {name} offset overflowed"))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|why| format!("cannot seek Execution V3 {name} {physical}: {why}"))?;
    let mut raw = [0_u8; N];
    file.read_exact(&mut raw)
        .map_err(|why| format!("cannot read Execution V3 {name} {physical}: {why}"))?;
    Ok(raw)
}

fn append_raw(file: &mut File, raw: &[u8]) -> Result<(), ExecutionV3Refusal> {
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(raw))
        .map_err(|why| format!("cannot append Execution V3 fixed record: {why}"))
}

fn bounded_vec<T>(count: u64, max: u64, name: &str) -> Result<Vec<T>, ExecutionV3Refusal> {
    if count > max {
        return Err(format!(
            "Execution V3 {name} read count {count} exceeds bound {max}"
        ));
    }
    let capacity = usize::try_from(count)
        .map_err(|_| format!("Execution V3 {name} count does not fit usize"))?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(capacity)
        .map_err(|why| format!("cannot reserve Execution V3 {name}: {why}"))?;
    Ok(values)
}

fn checked_sum(values: &[u64], name: &str) -> Result<u64, ExecutionV3Refusal> {
    values.iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(*value)
            .ok_or_else(|| format!("Execution V3 {name} overflowed"))
    })
}

fn increment(value: &mut u64, name: &str) -> Result<(), ExecutionV3Refusal> {
    *value = value
        .checked_add(1)
        .ok_or_else(|| format!("Execution V3 {name} overflowed"))?;
    Ok(())
}

fn checked_end(first: u64, count: u64, name: &str) -> Result<u64, ExecutionV3Refusal> {
    first
        .checked_add(count)
        .ok_or_else(|| format!("Execution V3 {name} overflowed"))
}

fn usize_to_u64(value: usize, name: &str) -> Result<u64, ExecutionV3Refusal> {
    u64::try_from(value).map_err(|_| format!("Execution V3 {name} does not fit u64"))
}

fn require_nonzero(name: &str, value: [u8; 32]) -> Result<(), ExecutionV3Refusal> {
    if value == [0; 32] {
        Err(format!("Execution V3 {name} is zero"))
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

fn decode_family(value: u8) -> Result<ExecutionV3Family, ExecutionV3Refusal> {
    match value {
        1 => Ok(ExecutionV3Family::Nifty),
        2 => Ok(ExecutionV3Family::BankNifty),
        _ => Err(format!("Execution V3 family tag {value} is unknown")),
    }
}

fn decode_direction(value: u8) -> Result<ExecutionV3Direction, ExecutionV3Refusal> {
    match value {
        1 => Ok(ExecutionV3Direction::Long),
        2 => Ok(ExecutionV3Direction::Short),
        _ => Err(format!("Execution V3 direction tag {value} is unknown")),
    }
}

fn decode_admission_status(value: u8) -> Result<ExecutionV3AdmissionStatus, ExecutionV3Refusal> {
    match value {
        1 => Ok(ExecutionV3AdmissionStatus::Admitted),
        2 => Ok(ExecutionV3AdmissionStatus::Rejected),
        3 => Ok(ExecutionV3AdmissionStatus::Unmeasured),
        4 => Ok(ExecutionV3AdmissionStatus::Refused),
        _ => Err(format!(
            "Execution V3 admission status tag {value} is unknown"
        )),
    }
}

fn decode_terminal(value: u8) -> Result<ExecutionV3Terminal, ExecutionV3Refusal> {
    match value {
        1 => Ok(ExecutionV3Terminal::Authorized),
        2 => Ok(ExecutionV3Terminal::PolicyRefused),
        _ => Err(format!("Execution V3 terminal tag {value} is unknown")),
    }
}

fn decode_percentile_axis(value: u8) -> Result<ExecutionV3PercentileAxis, ExecutionV3Refusal> {
    match value {
        1 => Ok(ExecutionV3PercentileAxis::Stop),
        2 => Ok(ExecutionV3PercentileAxis::Target),
        3 => Ok(ExecutionV3PercentileAxis::Trail),
        _ => Err(format!(
            "Execution V3 percentile axis tag {value} is unknown"
        )),
    }
}

fn require_header(
    name: &str,
    reader: &mut FixedReader<'_>,
    magic: [u8; 16],
    domain: u32,
) -> Result<(), ExecutionV3Refusal> {
    if reader.array::<16>()? != magic {
        return Err(format!("Execution V3 {name} magic mismatch"));
    }
    let observed_version = reader.u32()?;
    let observed_domain = reader.u32()?;
    if observed_version != VERSION || observed_domain != domain {
        return Err(format!(
            "Execution V3 {name} version/domain {observed_version}/{observed_domain} is unsupported"
        ));
    }
    Ok(())
}

fn require_seal(
    name: &str,
    domain: &[u8],
    payload: &[u8],
    seal: &[u8],
) -> Result<(), ExecutionV3Refusal> {
    if seal != hash_parts(domain, &[payload]) {
        return Err(format!("Execution V3 {name} seal mismatch"));
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
    result: Result<T, ExecutionV3Refusal>,
    released: Result<(), ExecutionV3Refusal>,
) -> Result<T, ExecutionV3Refusal> {
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

    fn array<const N: usize>(&mut self, value: &[u8; N]) -> Result<(), ExecutionV3Refusal> {
        self.bytes(value)
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), ExecutionV3Refusal> {
        let end = self
            .cursor
            .checked_add(value.len())
            .ok_or_else(|| "Execution V3 writer offset overflowed".to_owned())?;
        let target = self
            .bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "Execution V3 fixed writer exceeded record".to_owned())?;
        target.copy_from_slice(value);
        self.cursor = end;
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), ExecutionV3Refusal> {
        self.bytes(&[value])
    }

    fn u32(&mut self, value: u32) -> Result<(), ExecutionV3Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), ExecutionV3Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), ExecutionV3Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), ExecutionV3Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "Execution V3 zero-fill offset overflowed".to_owned())?;
        let target = self
            .bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "Execution V3 zero-fill exceeded record".to_owned())?;
        target.fill(0);
        self.cursor = end;
        Ok(())
    }

    const fn remaining(&self) -> usize {
        self.bytes.len() - self.cursor
    }

    fn require_full(&self, name: &str) -> Result<(), ExecutionV3Refusal> {
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err(format!(
                "Execution V3 {name} wrote {} of {} bytes",
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

    fn array<const N: usize>(&mut self) -> Result<[u8; N], ExecutionV3Refusal> {
        let end = self
            .cursor
            .checked_add(N)
            .ok_or_else(|| "Execution V3 reader offset overflowed".to_owned())?;
        let source = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| "Execution V3 fixed reader exceeded record".to_owned())?;
        let mut value = [0_u8; N];
        value.copy_from_slice(source);
        self.cursor = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, ExecutionV3Refusal> {
        let [value] = self.array::<1>()?;
        Ok(value)
    }

    fn u32(&mut self) -> Result<u32, ExecutionV3Refusal> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, ExecutionV3Refusal> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn i64(&mut self) -> Result<i64, ExecutionV3Refusal> {
        Ok(i64::from_le_bytes(self.array()?))
    }

    fn require_zeros(&mut self, count: usize, name: &str) -> Result<(), ExecutionV3Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "Execution V3 reserve offset overflowed".to_owned())?;
        let source = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| "Execution V3 reserve exceeded record".to_owned())?;
        if source.iter().any(|byte| *byte != 0) {
            return Err(format!("Execution V3 {name} is nonzero"));
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
                "brutex-execution-v3-{label}-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir(&path).expect("create isolated Execution V3 root");
            Self { path }
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn digest(seed: u64) -> [u8; 32] {
        hash_parts(b"execution-v3-private-test\0", &[&seed.to_le_bytes()])
    }

    fn bounds() -> ExecutionV3Bounds {
        let parameters = ExecutionV3FileBound::new(
            40,
            40 * EXECUTION_V3_PARAMETER_BYTES as u64,
            EXECUTION_V3_PARAMETER_BYTES,
            "parameter",
        )
        .expect("parameter bounds");
        let percentiles = ExecutionV3FileBound::new(
            200,
            200 * EXECUTION_V3_PERCENTILE_BYTES as u64,
            EXECUTION_V3_PERCENTILE_BYTES,
            "percentile",
        )
        .expect("percentile bounds");
        let dispositions = ExecutionV3FileBound::new(
            200,
            200 * EXECUTION_V3_DISPOSITION_BYTES as u64,
            EXECUTION_V3_DISPOSITION_BYTES,
            "disposition",
        )
        .expect("disposition bounds");
        let completions = ExecutionV3FileBound::new(
            10,
            10 * EXECUTION_V3_COMPLETION_BYTES as u64,
            EXECUTION_V3_COMPLETION_BYTES,
            "Completion",
        )
        .expect("Completion bounds");
        ExecutionV3Bounds::new(parameters, percentiles, dispositions, completions, 20, 40)
            .expect("Execution V3 bounds")
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the fixed-record fixture constructs all four parameter identities and the complete admission/execution matrix explicitly"
    )]
    fn prepared(seed: u64) -> PreparedExecutionV3 {
        let population_id = digest(seed + 1);
        let population_ordered_digest = digest(seed + 2);
        let source_finalization_id = digest(seed + 3);
        let source_finalization_completion_id = digest(seed + 4);
        let source_admission_block_id = digest(seed + 5);
        let source_admission_completion_id = digest(seed + 6);
        let order = [
            (ExecutionV3Family::Nifty, ExecutionV3Direction::Long),
            (ExecutionV3Family::Nifty, ExecutionV3Direction::Short),
            (ExecutionV3Family::BankNifty, ExecutionV3Direction::Long),
            (ExecutionV3Family::BankNifty, ExecutionV3Direction::Short),
        ];
        let mut parameters = Vec::new();
        let mut percentiles = Vec::new();
        for (index, (family, direction)) in order.into_iter().enumerate() {
            let index_u64 = u64::try_from(index).expect("small parameter index");
            let mut parameter = ExecutionV3ParameterRecord {
                parameter_core_id: [0; 32],
                parameter_id: [0; 32],
                population_id,
                population_ordered_digest,
                source_finalization_id,
                source_finalization_completion_id,
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
                ExecutionV3PercentileRecord {
                    parameter_core_id: parameter.parameter_core_id,
                    axis: ExecutionV3PercentileAxis::Stop,
                    ordinal: 0,
                    numerator: 1,
                    denominator: 10,
                },
                ExecutionV3PercentileRecord {
                    parameter_core_id: parameter.parameter_core_id,
                    axis: ExecutionV3PercentileAxis::Stop,
                    ordinal: 1,
                    numerator: 2,
                    denominator: 10,
                },
                ExecutionV3PercentileRecord {
                    parameter_core_id: parameter.parameter_core_id,
                    axis: ExecutionV3PercentileAxis::Target,
                    ordinal: 0,
                    numerator: 3,
                    denominator: 10,
                },
                ExecutionV3PercentileRecord {
                    parameter_core_id: parameter.parameter_core_id,
                    axis: ExecutionV3PercentileAxis::Target,
                    ordinal: 1,
                    numerator: 4,
                    denominator: 10,
                },
                ExecutionV3PercentileRecord {
                    parameter_core_id: parameter.parameter_core_id,
                    axis: ExecutionV3PercentileAxis::Trail,
                    ordinal: 0,
                    numerator: 5,
                    denominator: 10,
                },
                ExecutionV3PercentileRecord {
                    parameter_core_id: parameter.parameter_core_id,
                    axis: ExecutionV3PercentileAxis::Trail,
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
        let parameters: [ExecutionV3ParameterRecord; PARAMETER_COUNT_PER_BLOCK] = parameters
            .try_into()
            .unwrap_or_else(|_| panic!("four synthetic parameters"));
        let statuses = [
            ExecutionV3AdmissionStatus::Admitted,
            ExecutionV3AdmissionStatus::Rejected,
            ExecutionV3AdmissionStatus::Unmeasured,
            ExecutionV3AdmissionStatus::Refused,
        ];
        let terminals = [
            ExecutionV3Terminal::Authorized,
            ExecutionV3Terminal::PolicyRefused,
        ];
        let mut dispositions = Vec::new();
        for family_index in 0..2_usize {
            let family = if family_index == 0 {
                ExecutionV3Family::Nifty
            } else {
                ExecutionV3Family::BankNifty
            };
            for (status_index, status) in statuses.into_iter().enumerate() {
                for (terminal_index, terminal) in terminals.into_iter().enumerate() {
                    let family_sequence = status_index * terminals.len() + terminal_index;
                    let global_sequence =
                        family_index * statuses.len() * terminals.len() + family_sequence;
                    let direction = if terminal == ExecutionV3Terminal::Authorized {
                        ExecutionV3Direction::Long
                    } else {
                        ExecutionV3Direction::Short
                    };
                    let parameter_index =
                        family_index * 2 + usize::from(direction == ExecutionV3Direction::Short);
                    let parameter = parameters
                        .get(parameter_index)
                        .expect("family/direction fixture parameter exists");
                    let mut disposition = ExecutionV3DispositionRecord {
                        disposition_id: [0; 32],
                        population_id,
                        population_row_id: digest(seed + 1_000 + global_sequence as u64),
                        candidate_semantic_id: digest(seed + 2_000 + global_sequence as u64),
                        candidate_base_row_id: digest(seed + 3_000 + global_sequence as u64),
                        admission_decision_id: digest(seed + 4_000 + global_sequence as u64),
                        finalization_row_id: digest(seed + 5_000 + global_sequence as u64),
                        finalization_completion_id: source_finalization_completion_id,
                        parameter_id: parameter.parameter_id,
                        execution_run_id: digest(seed + 6_000 + global_sequence as u64),
                        evaluated_grid_digest: digest(seed + 7_000 + global_sequence as u64),
                        resolution_digest: parameter.resolution_digest,
                        column_digest: digest(seed + 8_000 + global_sequence as u64),
                        context_digest: digest(seed + 9_000 + global_sequence as u64),
                        runner_disposition_digest: digest(seed + 10_000 + global_sequence as u64),
                        selected_exit_digest: if terminal == ExecutionV3Terminal::Authorized {
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
                        refusal_bits: if terminal == ExecutionV3Terminal::Authorized {
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
        let prepared = PreparedExecutionV3 {
            population_id,
            population_ordered_digest,
            source_finalization_id,
            source_finalization_completion_id,
            source_admission_block_id,
            source_admission_completion_id,
            parameters,
            percentiles,
            dispositions,
        };
        prepared.validate(bounds()).expect("valid synthetic block");
        prepared
    }

    fn rebind_parameter_and_percentiles(
        mut parameter: ExecutionV3ParameterRecord,
        percentiles: &[ExecutionV3PercentileRecord],
    ) -> (ExecutionV3ParameterRecord, Vec<ExecutionV3PercentileRecord>) {
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
        mut parameter: ExecutionV3ParameterRecord,
    ) -> ExecutionV3ParameterRecord {
        parameter.parameter_core_id = parameter.derive_core_id();
        parameter.parameter_id = parameter.derive_parameter_id();
        parameter
    }

    fn rebind_disposition(
        mut row: ExecutionV3DispositionRecord,
        parameter: &ExecutionV3ParameterRecord,
    ) -> ExecutionV3DispositionRecord {
        row.parameter_id = parameter.parameter_id;
        row.resolution_digest = parameter.resolution_digest;
        row.rung = parameter.rung;
        row.horizon_bars = parameter.horizon_bars;
        row.disposition_id = row.derive_id();
        row
    }

    fn append_exact_prefix(
        root: &Path,
        prepared: &PreparedExecutionV3,
        parameter_count: usize,
        percentile_count: usize,
        disposition_count: usize,
    ) {
        drop(ExecutionV3Ledger::open_write(root, bounds()).expect("create V3 files"));
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
            ExecutionV3ParameterRecord::decode(&parameter_raw).expect("parameter decode"),
            *parameter
        );
        let percentile = prepared.percentiles.first().expect("first percentile");
        let percentile_raw = percentile.encode().expect("percentile encode");
        assert_eq!(
            ExecutionV3PercentileRecord::decode(&percentile_raw).expect("percentile decode"),
            *percentile
        );
        let disposition = prepared.dispositions.first().expect("first disposition");
        let disposition_raw = disposition.encode().expect("disposition encode");
        assert_eq!(
            ExecutionV3DispositionRecord::decode(&disposition_raw).expect("disposition decode"),
            *disposition
        );
        let completion = prepared
            .expected_completion(0, 0, 0, 0)
            .expect("Completion");
        let completion_raw = completion.encode().expect("Completion encode");
        assert_eq!(
            ExecutionV3CompletionRecord::decode(&completion_raw).expect("Completion decode"),
            completion
        );

        let mut broken_seal = parameter_raw;
        *broken_seal.first_mut().expect("nonempty parameter record") ^= 1;
        assert!(ExecutionV3ParameterRecord::decode(&broken_seal).is_err());

        let mut unknown_family = disposition_raw;
        *unknown_family
            .get_mut(576)
            .expect("family tag is inside fixed record") = 9;
        reseal(
            &mut unknown_family,
            DISPOSITION_PAYLOAD_BYTES,
            DISPOSITION_SEAL_DOMAIN,
        );
        assert!(ExecutionV3DispositionRecord::decode(&unknown_family).is_err());

        let mut nonzero_reserve = completion_raw;
        *nonzero_reserve
            .get_mut(COMPLETION_PAYLOAD_BYTES - 1)
            .expect("Completion reserve is inside fixed record") = 1;
        reseal(
            &mut nonzero_reserve,
            COMPLETION_PAYLOAD_BYTES,
            COMPLETION_SEAL_DOMAIN,
        );
        assert!(ExecutionV3CompletionRecord::decode(&nonzero_reserve).is_err());

        let mut half_ttp = disposition.clone();
        half_ttp.ttp_trail_index = None;
        assert!(half_ttp.validate().is_err());

        let mut old_layout = parameter_raw;
        old_layout
            .get_mut(16..20)
            .expect("version slot is inside fixed record")
            .copy_from_slice(&4_u32.to_le_bytes());
        reseal(
            &mut old_layout,
            PARAMETER_PAYLOAD_BYTES,
            PARAMETER_SEAL_DOMAIN,
        );
        assert!(ExecutionV3ParameterRecord::decode(&old_layout).is_err());

        let mut wrong_close = parameter.clone();
        wrong_close.forced_exit_ist_minute = 909;
        wrong_close.parameter_core_id = wrong_close.derive_core_id();
        wrong_close.parameter_id = wrong_close.derive_parameter_id();
        assert!(wrong_close.validate().is_err());
    }

    #[test]
    fn admission_and_execution_are_independent_and_refusals_keep_run_provenance() {
        let prepared = prepared(11);
        for family in [ExecutionV3Family::Nifty, ExecutionV3Family::BankNifty] {
            for status in [
                ExecutionV3AdmissionStatus::Admitted,
                ExecutionV3AdmissionStatus::Rejected,
                ExecutionV3AdmissionStatus::Unmeasured,
                ExecutionV3AdmissionStatus::Refused,
            ] {
                for terminal in [
                    ExecutionV3Terminal::Authorized,
                    ExecutionV3Terminal::PolicyRefused,
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
                row.admission_status == ExecutionV3AdmissionStatus::Rejected
                    && row.terminal == ExecutionV3Terminal::Authorized
            })
            .expect("Rejected and Authorized fixture cell");
        assert_ne!(rejected_authorized.execution_run_id, [0; 32]);
        let admitted_refused = prepared
            .dispositions
            .iter()
            .find(|row| {
                row.admission_status == ExecutionV3AdmissionStatus::Admitted
                    && row.terminal == ExecutionV3Terminal::PolicyRefused
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
            ExecutionV3ProductionCommit::Written(authority)
            | ExecutionV3ProductionCommit::Reused(authority) => authority
                .ordered_authenticated_dispositions()
                .expect("authenticated dispositions"),
        };
        assert_eq!(rows.len(), prepared.dispositions.len());
        let first_row = rows.first().expect("first authenticated disposition");
        assert_eq!(first_row.global_sequence(), 0);
        assert_eq!(first_row.terminal(), ExecutionV3Terminal::Authorized);
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
            4 * EXECUTION_V3_PARAMETER_BYTES as u64
        );
        assert_eq!(
            std::fs::metadata(root.path.join(COMPLETION_FILE))
                .expect("Completion metadata")
                .len(),
            EXECUTION_V3_COMPLETION_BYTES as u64
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
            ExecutionV3Ledger::open_write(&out_of_order.path, bounds())
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
        assert!(ExecutionV3Ledger::open_read(&out_of_order.path, bounds()).is_err());
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
            ExecutionV3Ledger::open_read(&root.path, bounds()).expect("open retained reader");
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
        drop(ExecutionV3Ledger::open_write(&ragged.path, bounds()).expect("create ragged files"));
        let mut file = OpenOptions::new()
            .append(true)
            .open(ragged.path.join(DISPOSITION_FILE))
            .expect("open ragged file");
        file.write_all(&[1]).expect("append ragged byte");
        file.sync_data().expect("sync ragged byte");
        assert!(ExecutionV3Ledger::open_read(&ragged.path, bounds()).is_err());

        #[cfg(unix)]
        {
            let hardlink = TestRoot::new("hardlink");
            drop(
                ExecutionV3Ledger::open_write(&hardlink.path, bounds())
                    .expect("create hardlink files"),
            );
            std::fs::hard_link(
                hardlink.path.join(PARAMETER_FILE),
                hardlink.path.join("second-parameter-link"),
            )
            .expect("create hard link");
            assert!(ExecutionV3Ledger::open_read(&hardlink.path, bounds()).is_err());

            let target = TestRoot::new("symlink-target");
            let link_parent = TestRoot::new("symlink-parent");
            let link = link_parent.path.join("linked-root");
            symlink(&target.path, &link).expect("create root symlink");
            assert!(ExecutionV3Ledger::open_write(&link, bounds()).is_err());
        }
    }

    #[test]
    fn bounds_and_terminal_matrix_are_explicit_and_complete() {
        assert!(
            ExecutionV3FileBound::new(0, 1, EXECUTION_V3_PARAMETER_BYTES, "parameter").is_err()
        );
        assert!(
            ExecutionV3FileBound::new(
                2,
                EXECUTION_V3_PARAMETER_BYTES as u64,
                EXECUTION_V3_PARAMETER_BYTES,
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
