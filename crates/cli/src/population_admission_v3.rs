//! Receipt-last Population Admission V3 persistence.
//!
//! V3 is a new byte family. It neither decodes nor rewrites Admission V2.
//! One block joins exact Candidate, Pre-Admission, Observation, Statistics,
//! anchored Search, Base-Evidence and Runner Admission V3 authorities. NIFTY
//! decisions are ordered before BANKNIFTY decisions. Candidate cardinality is
//! deliberately independent of Search fold cardinality.
//!
//! Public reopen proves bounded structural consistency only. Production
//! authority is crate-private and is minted only after a freshly reopened block
//! is compared byte-for-byte with an opaque preparation. Consequently a
//! self-consistent reseal is never treated as authenticated source evidence.
//!
//! Opening, append, retry, reuse and authentication hash and validate bounded
//! files. They are O(file bytes + records + block decisions), not O(1). The
//! in-memory block index provides average O(1) lookup only after that scan and
//! retains O(Completion records + one trailing block) space. The opaque bulk
//! projection validates generations once before and once after one ordered
//! fixed-record pass; repeated per-ordinal projection is not used for a whole
//! downstream population.

use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;
#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
use std::os::unix::fs::OpenOptionsExt as _;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt as _;

use brutex_core::blake3::Hasher;
use runner::admission::{
    AdmissionEvidenceValuesV1, AdmissionPolicyV1, AdmissionStatusV1,
    AdmissionV3ArithmeticProjection, AdmissionVerdictV1,
};
use runner::validate::AnchoredSearchAuthorityProjectionV4;

use crate::population::{InstrumentFamilyV1, LongShortExitGridIdentitiesV2};
use crate::step3_orchestrator::{
    CommittedStoredCandidatePreAdmissionV1, CommittedStoredObservationStatisticsV2,
    StoredSearchMemberV4, StoredSearchPairV4,
};

/// Bytes in one canonical Population Admission V3 decision.
pub(crate) const POPULATION_ADMISSION_V3_DECISION_BYTES: usize = 2_048;
/// Bytes in one receipt-last Population Admission V3 Completion.
pub(crate) const POPULATION_ADMISSION_V3_COMPLETION_BYTES: usize = 4_096;
/// Bytes in Runner's immutable canonical Admission V3 decision.
pub(crate) const RUNNER_ADMISSION_V3_DECISION_BYTES: usize =
    runner::admission::ADMISSION_DECISION_CANONICAL_LEN_V3;
/// Bytes in Runner's canonical Admission V3 evidence.
const RUNNER_ADMISSION_V3_EVIDENCE_BYTES: usize =
    runner::admission::ADMISSION_EVIDENCE_CANONICAL_LEN_V3;
/// Bytes in the immutable Runner admission policy nested by each block.
pub(crate) const ADMISSION_V3_POLICY_BYTES: usize =
    runner::admission::ADMISSION_POLICY_CANONICAL_LEN_V1;
const RUNNER_VERDICT_BYTES: usize = runner::admission::ADMISSION_VERDICT_CANONICAL_LEN_V1;

/// Operator-facing refusal at the Population Admission V3 boundary.
pub(crate) type PopulationAdmissionV3Refusal = String;

const VERSION: u32 = 3;
const DECISION_MAGIC: [u8; 16] = *b"BTX-ADMV3-DEC\0\0\0";
const COMPLETION_MAGIC: [u8; 16] = *b"BTX-ADMV3-CMP\0\0\0";
const DECISION_DOMAIN: u32 = 1;
const COMPLETION_DOMAIN: u32 = 2;
const SEAL_BYTES: usize = 32;
const DECISION_PAYLOAD_BYTES: usize = POPULATION_ADMISSION_V3_DECISION_BYTES - SEAL_BYTES;
const COMPLETION_PAYLOAD_BYTES: usize = POPULATION_ADMISSION_V3_COMPLETION_BYTES - SEAL_BYTES;
const BLOCK_SOURCE_BYTES: usize = 3_006;
const BLOCK_IDENTITY_BYTES: usize = BLOCK_SOURCE_BYTES - 32;

const BLOCK_ID_DOMAIN: &[u8] = b"brutex-population-admission-v3-block-id\0";
const DECISION_ID_DOMAIN: &[u8] = b"brutex-population-admission-v3-decision-id\0";
const ORDERED_DECISIONS_DOMAIN: &[u8] = b"brutex-population-admission-v3-ordered-decisions\0";
const COMPLETION_ID_DOMAIN: &[u8] = b"brutex-population-admission-v3-completion-id\0";
const DECISION_SEAL_DOMAIN: &[u8] = b"brutex-population-admission-v3-decision-seal\0";
const COMPLETION_SEAL_DOMAIN: &[u8] = b"brutex-population-admission-v3-completion-seal\0";
const FILE_GENERATION_DOMAIN: &[u8] = b"brutex-population-admission-v3-generation\0";
const POLICY_DIGEST_DOMAIN: &[u8] = b"brutex-population-admission-v3-policy\0";
const RUNNER_DECISION_DIGEST_DOMAIN: &[u8] = b"brutex-population-admission-v3-runner-decision\0";
const RUNNER_EVIDENCE_DIGEST_DOMAIN: &[u8] = b"brutex-population-admission-v3-runner-evidence\0";
const RUNNER_VERDICT_DIGEST_DOMAIN: &[u8] = b"brutex-population-admission-v3-runner-verdict\0";
const SEARCH_MEMBER_ID_DOMAIN: &[u8] =
    b"brutex-population-admission-v3-exact-grid-search-member-v4\0";
const SEARCH_PAIR_ID_DOMAIN: &[u8] = b"brutex-population-admission-v3-exact-grid-search-pair-v4\0";

const DECISION_FILE: &str = "population-admission-v3.bin";
const COMPLETION_FILE: &str = "population-admission-completions-v3.bin";
const LOCK_FILE: &str = "population-admission-v3.lock";
const LOCK_MAX_BYTES: u64 = 0;
const READ_CHUNK_BYTES: usize = 16 * 1_024;

const RUNNER_HEADER_BYTES: usize = 12;
const RUNNER_POLICY_OFFSET: usize = RUNNER_HEADER_BYTES;
const RUNNER_EVIDENCE_OFFSET: usize = RUNNER_POLICY_OFFSET + ADMISSION_V3_POLICY_BYTES;
const RUNNER_VERDICT_OFFSET: usize = RUNNER_EVIDENCE_OFFSET + RUNNER_ADMISSION_V3_EVIDENCE_BYTES;
const RUNNER_DECISION_PAYLOAD_BYTES: u32 = 1_315;
const RUNNER_EVIDENCE_PAYLOAD_BYTES: u32 = 948;
const EVIDENCE_BASE_VALUES_BYTES: usize = 340;
const EVIDENCE_STATISTICS_OFFSET: usize = RUNNER_HEADER_BYTES + EVIDENCE_BASE_VALUES_BYTES;
const EVIDENCE_WALK_OFFSET: usize = 800;

#[cfg(any(target_os = "android", target_os = "linux"))]
const O_NOFOLLOW_FLAG: i32 = 0x20_000;
#[cfg(target_os = "macos")]
const O_NOFOLLOW_FLAG: i32 = 0x100;

const _: () =
    assert!(DECISION_PAYLOAD_BYTES + SEAL_BYTES == POPULATION_ADMISSION_V3_DECISION_BYTES);
const _: () =
    assert!(COMPLETION_PAYLOAD_BYTES + SEAL_BYTES == POPULATION_ADMISSION_V3_COMPLETION_BYTES);
const _: () =
    assert!(RUNNER_VERDICT_OFFSET + RUNNER_VERDICT_BYTES == RUNNER_ADMISSION_V3_DECISION_BYTES);
const _: () = assert!(EVIDENCE_WALK_OFFSET + 160 == RUNNER_ADMISSION_V3_EVIDENCE_BYTES);

/// Explicit nonzero physical ceilings for Population Admission V3.
///
/// There is no `Default`; every caller states every record, byte and per-block
/// ceiling. Inputs are refused rather than sampled or truncated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AdmissionV3Bounds {
    decision_records: u64,
    decision_bytes: u64,
    completion_records: u64,
    completion_bytes: u64,
    decisions_per_block: u64,
}

impl AdmissionV3Bounds {
    /// Constructs explicit nonzero ceilings.
    ///
    /// # Errors
    ///
    /// Refuses zero, overflow, a per-block ceiling above the total ceiling, or
    /// byte ceilings unable to contain their declared fixed records.
    pub(crate) fn new(
        max_decision_records: u64,
        max_decision_bytes: u64,
        max_completion_records: u64,
        max_completion_bytes: u64,
        max_decisions_per_block: u64,
    ) -> Result<Self, PopulationAdmissionV3Refusal> {
        for (name, value) in [
            ("decision records", max_decision_records),
            ("decision bytes", max_decision_bytes),
            ("Completion records", max_completion_records),
            ("Completion bytes", max_completion_bytes),
            ("decisions per block", max_decisions_per_block),
        ] {
            if value == 0 {
                return Err(format!("Admission V3 maximum {name} must be nonzero"));
            }
        }
        if max_decisions_per_block > max_decision_records {
            return Err(format!(
                "Admission V3 per-block maximum {max_decisions_per_block} exceeds total decision maximum {max_decision_records}"
            ));
        }
        let required_decision_bytes = max_decision_records
            .checked_mul(POPULATION_ADMISSION_V3_DECISION_BYTES as u64)
            .ok_or_else(|| "Admission V3 decision byte ceiling overflowed".to_owned())?;
        if max_decision_bytes < required_decision_bytes {
            return Err(format!(
                "Admission V3 decision byte maximum {max_decision_bytes} cannot hold {max_decision_records} records ({required_decision_bytes} bytes)"
            ));
        }
        let required_completion_bytes = max_completion_records
            .checked_mul(POPULATION_ADMISSION_V3_COMPLETION_BYTES as u64)
            .ok_or_else(|| "Admission V3 Completion byte ceiling overflowed".to_owned())?;
        if max_completion_bytes < required_completion_bytes {
            return Err(format!(
                "Admission V3 Completion byte maximum {max_completion_bytes} cannot hold {max_completion_records} records ({required_completion_bytes} bytes)"
            ));
        }
        Ok(Self {
            decision_records: max_decision_records,
            decision_bytes: max_decision_bytes,
            completion_records: max_completion_records,
            completion_bytes: max_completion_bytes,
            decisions_per_block: max_decisions_per_block,
        })
    }

    /// Maximum physical decision records.
    #[must_use]
    pub(crate) const fn max_decision_records(self) -> u64 {
        self.decision_records
    }

    /// Maximum decision-file bytes.
    #[must_use]
    pub(crate) const fn max_decision_bytes(self) -> u64 {
        self.decision_bytes
    }

    /// Maximum physical Completion records.
    #[must_use]
    pub(crate) const fn max_completion_records(self) -> u64 {
        self.completion_records
    }

    /// Maximum Completion-file bytes.
    #[must_use]
    pub(crate) const fn max_completion_bytes(self) -> u64 {
        self.completion_bytes
    }

    /// Maximum decisions in one semantic block.
    #[must_use]
    pub(crate) const fn max_decisions_per_block(self) -> u64 {
        self.decisions_per_block
    }
}

/// Canonical instrument-family order for Population Admission V3.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum AdmissionV3Family {
    /// `NSE-NIFTY` prefix.
    Nifty = 1,
    /// `NSE-BANKNIFTY` suffix.
    BankNifty = 2,
}

impl AdmissionV3Family {
    fn decode(value: u8) -> Result<Self, PopulationAdmissionV3Refusal> {
        match value {
            1 => Ok(Self::Nifty),
            2 => Ok(Self::BankNifty),
            _ => Err(format!("Admission V3 family tag {value} is unknown")),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AdmissionV3Status {
    /// Every required institutional gate admitted the Candidate.
    Admitted = 1,
    /// Complete evidence was measured and at least one gate rejected it.
    Rejected = 2,
    /// The policy classified the complete evidence as not measurable.
    Unmeasured = 3,
    /// The Runner refused malformed, inconsistent or unsafe evidence.
    Refused = 4,
}

impl AdmissionV3Status {
    fn decode(value: u8) -> Result<Self, PopulationAdmissionV3Refusal> {
        match value {
            1 => Ok(Self::Admitted),
            2 => Ok(Self::Rejected),
            3 => Ok(Self::Unmeasured),
            4 => Ok(Self::Refused),
            _ => Err(format!("Admission V3 status tag {value} is unknown")),
        }
    }

    const fn from_runner(value: AdmissionStatusV1) -> Self {
        match value {
            AdmissionStatusV1::Admitted => Self::Admitted,
            AdmissionStatusV1::Rejected => Self::Rejected,
            AdmissionStatusV1::Unmeasured => Self::Unmeasured,
            AdmissionStatusV1::Refused => Self::Refused,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RequestedSpanV3 {
    from_year: u16,
    from_month: u8,
    to_year: u16,
    to_month: u8,
}

impl RequestedSpanV3 {
    fn validate(self) -> Result<(), PopulationAdmissionV3Refusal> {
        for (name, year) in [("from", self.from_year), ("to", self.to_year)] {
            if !(1970..=9999).contains(&year) {
                return Err(format!(
                    "Admission V3 {name} year {year} is outside 1970..=9999"
                ));
            }
        }
        for (name, month) in [("from", self.from_month), ("to", self.to_month)] {
            if !(1..=12).contains(&month) {
                return Err(format!(
                    "Admission V3 {name} month {month} is outside 1..=12"
                ));
            }
        }
        if (self.to_year, self.to_month) < (self.from_year, self.from_month) {
            return Err("Admission V3 requested span steps backwards".to_owned());
        }
        Ok(())
    }

    fn encode(self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationAdmissionV3Refusal> {
        writer.u16(self.from_year)?;
        writer.u8(self.from_month)?;
        writer.u8(0)?;
        writer.u16(self.to_year)?;
        writer.u8(self.to_month)?;
        writer.u8(0)
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationAdmissionV3Refusal> {
        let from_year = reader.u16()?;
        let from_month = reader.u8()?;
        reader.require_zeros(1, "Admission V3 span first reserve")?;
        let to_year = reader.u16()?;
        let to_month = reader.u8()?;
        reader.require_zeros(1, "Admission V3 span second reserve")?;
        let value = Self {
            from_year,
            from_month,
            to_year,
            to_month,
        };
        value.validate()?;
        Ok(value)
    }
}

/// Exact full-span signal stream and prepared signal-column identity.
///
/// Candidate V1 and Search V4 expose the same five equality components through
/// independently typed capabilities. Retaining both copies in their ordered
/// sources makes a decoded structural block capable of refusing a crosswire;
/// only the production constructor can populate them from those capabilities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SignalSourceV3 {
    digest: [u8; 32],
    bars: u64,
    first_ts_micros: i64,
    last_ts_micros: i64,
    column_digest: [u8; 32],
}

impl SignalSourceV3 {
    fn validate(self, owner: &str) -> Result<(), PopulationAdmissionV3Refusal> {
        require_nonzero(&format!("Admission V3 {owner} signal stream"), self.digest)?;
        require_nonzero(
            &format!("Admission V3 {owner} prepared signal column"),
            self.column_digest,
        )?;
        if self.bars == 0 {
            return Err(format!("Admission V3 {owner} signal count is zero"));
        }
        if self.first_ts_micros > self.last_ts_micros {
            return Err(format!(
                "Admission V3 {owner} signal timestamps run backwards"
            ));
        }
        Ok(())
    }

    fn encode(self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationAdmissionV3Refusal> {
        writer.array(&self.digest)?;
        writer.u64(self.bars)?;
        writer.i64(self.first_ts_micros)?;
        writer.i64(self.last_ts_micros)?;
        writer.array(&self.column_digest)
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationAdmissionV3Refusal> {
        let value = Self {
            digest: reader.array()?,
            bars: reader.u64()?,
            first_ts_micros: reader.i64()?,
            last_ts_micros: reader.i64()?,
            column_digest: reader.array()?,
        };
        value.validate("decoded")?;
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ExitGridSourceV3 {
    long_policy: [u8; 32],
    long_resolution: [u8; 32],
    short_policy: [u8; 32],
    short_resolution: [u8; 32],
    composite: [u8; 32],
}

impl ExitGridSourceV3 {
    fn from_candidate(
        value: LongShortExitGridIdentitiesV2,
    ) -> Result<Self, PopulationAdmissionV3Refusal> {
        let source = Self {
            long_policy: value.long.policy_digest,
            long_resolution: value.long.resolved_digest,
            short_policy: value.short.policy_digest,
            short_resolution: value.short.resolved_digest,
            composite: value.composite_digest()?,
        };
        source.validate("Candidate")?;
        Ok(source)
    }

    fn validate(self, owner: &str) -> Result<(), PopulationAdmissionV3Refusal> {
        for (name, value) in [
            ("Long policy", self.long_policy),
            ("Long resolution", self.long_resolution),
            ("Short policy", self.short_policy),
            ("Short resolution", self.short_resolution),
            ("Long-plus-Short composite", self.composite),
        ] {
            require_nonzero(&format!("Admission V3 {owner} {name}"), value)?;
        }
        if self.long_policy == self.short_policy || self.long_resolution == self.short_resolution {
            return Err(format!(
                "Admission V3 {owner} Long and Short grid components alias"
            ));
        }
        let candidate = LongShortExitGridIdentitiesV2 {
            long: crate::population::SideExitGridIdentityV2 {
                policy_digest: self.long_policy,
                resolved_digest: self.long_resolution,
            },
            short: crate::population::SideExitGridIdentityV2 {
                policy_digest: self.short_policy,
                resolved_digest: self.short_resolution,
            },
        };
        if candidate.composite_digest()? != self.composite {
            return Err(format!(
                "Admission V3 {owner} Long-plus-Short grid composite does not reproduce"
            ));
        }
        Ok(())
    }

    fn encode(self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationAdmissionV3Refusal> {
        for value in [
            self.long_policy,
            self.long_resolution,
            self.short_policy,
            self.short_resolution,
            self.composite,
        ] {
            writer.array(&value)?;
        }
        Ok(())
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationAdmissionV3Refusal> {
        let value = Self {
            long_policy: reader.array()?,
            long_resolution: reader.array()?,
            short_policy: reader.array()?,
            short_resolution: reader.array()?,
            composite: reader.array()?,
        };
        value.validate("decoded Candidate")?;
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FamilySourceV3 {
    candidate_universe_id: [u8; 32],
    candidate_completion_digest: [u8; 32],
    pre_admission_authority_id: [u8; 32],
    signal: SignalSourceV3,
    exit_grid: ExitGridSourceV3,
    candidate_count: u64,
}

impl FamilySourceV3 {
    fn validate(&self, family: &str) -> Result<(), PopulationAdmissionV3Refusal> {
        for (name, value) in [
            ("Candidate universe", self.candidate_universe_id),
            ("Candidate Completion", self.candidate_completion_digest),
            ("Pre-Admission authority", self.pre_admission_authority_id),
        ] {
            require_nonzero(&format!("{family} {name}"), value)?;
        }
        self.signal.validate(&format!("{family} Candidate"))?;
        self.exit_grid.validate(&format!("{family} Candidate"))?;
        Ok(())
    }

    fn encode(&self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationAdmissionV3Refusal> {
        writer.array(&self.candidate_universe_id)?;
        writer.array(&self.candidate_completion_digest)?;
        writer.array(&self.pre_admission_authority_id)?;
        self.signal.encode(writer)?;
        self.exit_grid.encode(writer)?;
        writer.u64(self.candidate_count)
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationAdmissionV3Refusal> {
        Ok(Self {
            candidate_universe_id: reader.array()?,
            candidate_completion_digest: reader.array()?,
            pre_admission_authority_id: reader.array()?,
            signal: SignalSourceV3::decode(reader)?,
            exit_grid: ExitGridSourceV3::decode(reader)?,
            candidate_count: reader.u64()?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ObservationSourceV3 {
    authority_id: [u8; 32],
    completion_digest: [u8; 32],
    pair_id: [u8; 32],
    source_id: [u8; 32],
    observation_policy_id: [u8; 32],
    layout_policy_id: [u8; 32],
    candidate_count: u64,
    period_count: u64,
}

impl ObservationSourceV3 {
    fn validate(self) -> Result<(), PopulationAdmissionV3Refusal> {
        for (name, value) in [
            ("Observation authority", self.authority_id),
            ("Observation Completion", self.completion_digest),
            ("Observation pair", self.pair_id),
            ("Observation source", self.source_id),
            ("Observation policy", self.observation_policy_id),
            ("Observation layout policy", self.layout_policy_id),
        ] {
            require_nonzero(name, value)?;
        }
        if self.period_count == 0 {
            return Err("Admission V3 Observation period count is zero".to_owned());
        }
        Ok(())
    }

    fn encode(self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationAdmissionV3Refusal> {
        for value in [
            self.authority_id,
            self.completion_digest,
            self.pair_id,
            self.source_id,
            self.observation_policy_id,
            self.layout_policy_id,
        ] {
            writer.array(&value)?;
        }
        writer.u64(self.candidate_count)?;
        writer.u64(self.period_count)
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationAdmissionV3Refusal> {
        Ok(Self {
            authority_id: reader.array()?,
            completion_digest: reader.array()?,
            pair_id: reader.array()?,
            source_id: reader.array()?,
            observation_policy_id: reader.array()?,
            layout_policy_id: reader.array()?,
            candidate_count: reader.u64()?,
            period_count: reader.u64()?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StatisticsSourceV3 {
    audit_id: [u8; 32],
    completion_digest: [u8; 32],
    observation_authority_id: [u8; 32],
    observation_completion_digest: [u8; 32],
    observation_pair_id: [u8; 32],
    observation_statistics_link_id: [u8; 32],
    observation_projection_policy_id: [u8; 32],
    wilson_policy_id: [u8; 32],
    cscv_policy_id: [u8; 32],
    ordered_candidate_id: [u8; 32],
    ordered_period_id: [u8; 32],
    ordered_split_id: [u8; 32],
    white_family_id: [u8; 32],
    spa_family_id: [u8; 32],
    romano_wolf_family_id: [u8; 32],
    candidate_count: u64,
    period_count: u64,
    split_count: u64,
    draw_count: u64,
    bootstrap_seed: u64,
    bootstrap_block_length: u64,
}

impl StatisticsSourceV3 {
    fn validate(&self) -> Result<(), PopulationAdmissionV3Refusal> {
        for (name, value) in [
            ("Statistics audit", self.audit_id),
            ("Statistics Completion", self.completion_digest),
            (
                "Statistics Observation authority",
                self.observation_authority_id,
            ),
            (
                "Statistics Observation Completion",
                self.observation_completion_digest,
            ),
            ("Statistics Observation pair", self.observation_pair_id),
            (
                "Observation-to-Statistics link",
                self.observation_statistics_link_id,
            ),
            (
                "Observation projection policy",
                self.observation_projection_policy_id,
            ),
            ("Wilson policy", self.wilson_policy_id),
            ("CSCV policy", self.cscv_policy_id),
            ("ordered Candidate family", self.ordered_candidate_id),
            ("ordered period family", self.ordered_period_id),
            ("ordered split family", self.ordered_split_id),
            ("White family", self.white_family_id),
            ("SPA family", self.spa_family_id),
            ("Romano-Wolf family", self.romano_wolf_family_id),
        ] {
            require_nonzero(name, value)?;
        }
        for (name, value) in [
            ("period", self.period_count),
            ("split", self.split_count),
            ("draw", self.draw_count),
            ("bootstrap block", self.bootstrap_block_length),
        ] {
            if value == 0 {
                return Err(format!("Admission V3 Statistics {name} count is zero"));
            }
        }
        Ok(())
    }

    fn encode(&self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationAdmissionV3Refusal> {
        for value in [
            self.audit_id,
            self.completion_digest,
            self.observation_authority_id,
            self.observation_completion_digest,
            self.observation_pair_id,
            self.observation_statistics_link_id,
            self.observation_projection_policy_id,
            self.wilson_policy_id,
            self.cscv_policy_id,
            self.ordered_candidate_id,
            self.ordered_period_id,
            self.ordered_split_id,
            self.white_family_id,
            self.spa_family_id,
            self.romano_wolf_family_id,
        ] {
            writer.array(&value)?;
        }
        for value in [
            self.candidate_count,
            self.period_count,
            self.split_count,
            self.draw_count,
            self.bootstrap_seed,
            self.bootstrap_block_length,
        ] {
            writer.u64(value)?;
        }
        Ok(())
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationAdmissionV3Refusal> {
        Ok(Self {
            audit_id: reader.array()?,
            completion_digest: reader.array()?,
            observation_authority_id: reader.array()?,
            observation_completion_digest: reader.array()?,
            observation_pair_id: reader.array()?,
            observation_statistics_link_id: reader.array()?,
            observation_projection_policy_id: reader.array()?,
            wilson_policy_id: reader.array()?,
            cscv_policy_id: reader.array()?,
            ordered_candidate_id: reader.array()?,
            ordered_period_id: reader.array()?,
            ordered_split_id: reader.array()?,
            white_family_id: reader.array()?,
            spa_family_id: reader.array()?,
            romano_wolf_family_id: reader.array()?,
            candidate_count: reader.u64()?,
            period_count: reader.u64()?,
            split_count: reader.u64()?,
            draw_count: reader.u64()?,
            bootstrap_seed: reader.u64()?,
            bootstrap_block_length: reader.u64()?,
        })
    }
}

/// Exact-grid Search V4 source for one instrument family.
///
/// This is not a legacy Search V3 authority. Every field is projected from an
/// opaque, freshly reconciled [`StoredSearchMemberV4`], and the member identity
/// is derived locally from the complete V4 equality surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SearchMemberV4 {
    member_id: [u8; 32],
    validation_policy_id: [u8; 32],
    signal: SignalSourceV3,
    full_grid_id: [u8; 32],
    long_policy_id: [u8; 32],
    long_resolution_id: [u8; 32],
    short_policy_id: [u8; 32],
    short_resolution_id: [u8; 32],
    validation_family_id: [u8; 32],
    walk_id: [u8; 32],
    fold_count: u64,
    decided_folds: u64,
    profitable_oos_folds: u64,
    aggregate_oos_paisa: i64,
    evaluated_population_count: u64,
}

impl SearchMemberV4 {
    fn from_retained(
        value: &StoredSearchMemberV4,
        candidate: &FamilySourceV3,
    ) -> Result<Self, PopulationAdmissionV3Refusal> {
        if value.candidate_universe_id() != candidate.candidate_universe_id
            || value.candidate_completion_digest() != candidate.candidate_completion_digest
        {
            return Err(
                "Admission V3 retained Search V4 Candidate authority differs from its family source"
                    .to_owned(),
            );
        }
        let projection = value
            .projection()
            .map_err(|why| format!("Admission V3 retained Search V4 projection refused: {why}"))?;
        Self::from_projection(&projection)
    }

    fn from_projection(
        value: &AnchoredSearchAuthorityProjectionV4,
    ) -> Result<Self, PopulationAdmissionV3Refusal> {
        let source = value.source_identity();
        let long = value.long_grid_identity();
        let short = value.short_grid_identity();
        let mut member = Self {
            member_id: [0; 32],
            validation_policy_id: value.policy_identity().digest(),
            signal: SignalSourceV3 {
                digest: source.signal_digest(),
                bars: source.signal_bars(),
                first_ts_micros: source.signal_first_ts_micros(),
                last_ts_micros: source.signal_last_ts_micros(),
                column_digest: source.signal_column_digest(),
            },
            full_grid_id: value.grid_identity().digest(),
            long_policy_id: long.policy_digest(),
            long_resolution_id: long.resolution_digest(),
            short_policy_id: short.policy_digest(),
            short_resolution_id: short.resolution_digest(),
            validation_family_id: value.family_identity().digest(),
            walk_id: value.walk_identity().digest(),
            fold_count: value.fold_count(),
            decided_folds: value.decided_folds(),
            profitable_oos_folds: value.profitable_oos_folds(),
            aggregate_oos_paisa: value.aggregate_oos_paisa(),
            evaluated_population_count: value.evaluated_population_cells(),
        };
        member.member_id = member.derive_member_id();
        member.validate("projected")?;
        Ok(member)
    }

    fn validate(&self, family: &str) -> Result<(), PopulationAdmissionV3Refusal> {
        for (name, value) in [
            ("member", self.member_id),
            ("validation policy", self.validation_policy_id),
            ("full grid", self.full_grid_id),
            ("Long policy", self.long_policy_id),
            ("Long resolution", self.long_resolution_id),
            ("Short policy", self.short_policy_id),
            ("Short resolution", self.short_resolution_id),
            ("validation family", self.validation_family_id),
            ("walk", self.walk_id),
        ] {
            require_nonzero(&format!("{family} exact-grid Search V4 {name}"), value)?;
        }
        self.signal
            .validate(&format!("{family} exact-grid Search V4"))?;
        if self.long_policy_id == self.short_policy_id
            || self.long_resolution_id == self.short_resolution_id
        {
            return Err(format!(
                "{family} Admission V3 exact-grid Search V4 Long and Short components alias"
            ));
        }
        if self.member_id != self.derive_member_id() {
            return Err(format!(
                "{family} Admission V3 exact-grid Search V4 member identity does not reproduce"
            ));
        }
        if self.fold_count == 0
            || self.decided_folds > self.fold_count
            || self.profitable_oos_folds > self.decided_folds
        {
            return Err(format!(
                "{family} Admission V3 Search fold hierarchy is invalid"
            ));
        }
        if (self.decided_folds == 0 && self.aggregate_oos_paisa != 0)
            || (self.profitable_oos_folds == 0 && self.aggregate_oos_paisa > 0)
            || (self.profitable_oos_folds == self.decided_folds
                && self.decided_folds > 0
                && self.aggregate_oos_paisa <= 0)
        {
            return Err(format!(
                "{family} Admission V3 Search outcome hierarchy is invalid"
            ));
        }
        Ok(())
    }

    fn derive_member_id(&self) -> [u8; 32] {
        hash_slices(
            SEARCH_MEMBER_ID_DOMAIN,
            &[
                &self.validation_policy_id,
                &self.signal.digest,
                &self.signal.bars.to_le_bytes(),
                &self.signal.first_ts_micros.to_le_bytes(),
                &self.signal.last_ts_micros.to_le_bytes(),
                &self.signal.column_digest,
                &self.full_grid_id,
                &self.long_policy_id,
                &self.long_resolution_id,
                &self.short_policy_id,
                &self.short_resolution_id,
                &self.validation_family_id,
                &self.walk_id,
                &self.fold_count.to_le_bytes(),
                &self.decided_folds.to_le_bytes(),
                &self.profitable_oos_folds.to_le_bytes(),
                &self.aggregate_oos_paisa.to_le_bytes(),
                &self.evaluated_population_count.to_le_bytes(),
            ],
        )
    }

    fn encode(&self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationAdmissionV3Refusal> {
        for value in [self.member_id, self.validation_policy_id] {
            writer.array(&value)?;
        }
        self.signal.encode(writer)?;
        for value in [
            self.full_grid_id,
            self.long_policy_id,
            self.long_resolution_id,
            self.short_policy_id,
            self.short_resolution_id,
            self.validation_family_id,
            self.walk_id,
        ] {
            writer.array(&value)?;
        }
        writer.u64(self.fold_count)?;
        writer.u64(self.decided_folds)?;
        writer.u64(self.profitable_oos_folds)?;
        writer.i64(self.aggregate_oos_paisa)?;
        writer.u64(self.evaluated_population_count)
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationAdmissionV3Refusal> {
        let value = Self {
            member_id: reader.array()?,
            validation_policy_id: reader.array()?,
            signal: SignalSourceV3::decode(reader)?,
            full_grid_id: reader.array()?,
            long_policy_id: reader.array()?,
            long_resolution_id: reader.array()?,
            short_policy_id: reader.array()?,
            short_resolution_id: reader.array()?,
            validation_family_id: reader.array()?,
            walk_id: reader.array()?,
            fold_count: reader.u64()?,
            decided_folds: reader.u64()?,
            profitable_oos_folds: reader.u64()?,
            aggregate_oos_paisa: reader.i64()?,
            evaluated_population_count: reader.u64()?,
        };
        value.validate("decoded")?;
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SearchSourceV4 {
    pair_id: [u8; 32],
    nifty: SearchMemberV4,
    banknifty: SearchMemberV4,
}

impl SearchSourceV4 {
    fn from_retained(
        pair: &StoredSearchPairV4,
        nifty: &FamilySourceV3,
        banknifty: &FamilySourceV3,
    ) -> Result<Self, PopulationAdmissionV3Refusal> {
        let mut value = Self {
            pair_id: [0; 32],
            nifty: SearchMemberV4::from_retained(pair.nifty(), nifty)?,
            banknifty: SearchMemberV4::from_retained(pair.banknifty(), banknifty)?,
        };
        value.pair_id = value.derive_pair_id();
        value.validate()?;
        Ok(value)
    }

    fn validate(&self) -> Result<(), PopulationAdmissionV3Refusal> {
        require_nonzero("Admission V3 exact-grid Search V4 pair", self.pair_id)?;
        self.nifty.validate("NIFTY")?;
        self.banknifty.validate("BANKNIFTY")?;
        if self.nifty.member_id == self.banknifty.member_id {
            return Err(
                "Admission V3 NIFTY and BANKNIFTY exact-grid Search V4 members alias".to_owned(),
            );
        }
        if self.pair_id != self.derive_pair_id() {
            return Err(
                "Admission V3 exact-grid Search V4 pair identity does not reproduce".to_owned(),
            );
        }
        Ok(())
    }

    fn derive_pair_id(&self) -> [u8; 32] {
        hash_slices(
            SEARCH_PAIR_ID_DOMAIN,
            &[&self.nifty.member_id, &self.banknifty.member_id],
        )
    }

    fn encode(&self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationAdmissionV3Refusal> {
        writer.array(&self.pair_id)?;
        self.nifty.encode(writer)?;
        self.banknifty.encode(writer)
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationAdmissionV3Refusal> {
        let value = Self {
            pair_id: reader.array()?,
            nifty: SearchMemberV4::decode(reader)?,
            banknifty: SearchMemberV4::decode(reader)?,
        };
        value.validate()?;
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BaseSourceV3 {
    paired_base: [u8; 32],
    nifty_completion: [u8; 32],
    banknifty_completion: [u8; 32],
}

impl BaseSourceV3 {
    fn validate(self) -> Result<(), PopulationAdmissionV3Refusal> {
        for (name, value) in [
            ("paired Base-Evidence", self.paired_base),
            ("NIFTY Base-Evidence Completion", self.nifty_completion),
            (
                "BANKNIFTY Base-Evidence Completion",
                self.banknifty_completion,
            ),
        ] {
            require_nonzero(name, value)?;
        }
        if self.nifty_completion == self.banknifty_completion {
            return Err("Admission V3 Base-Evidence Completion identities alias".to_owned());
        }
        Ok(())
    }

    fn encode(self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationAdmissionV3Refusal> {
        writer.array(&self.paired_base)?;
        writer.array(&self.nifty_completion)?;
        writer.array(&self.banknifty_completion)
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationAdmissionV3Refusal> {
        Ok(Self {
            paired_base: reader.array()?,
            nifty_completion: reader.array()?,
            banknifty_completion: reader.array()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BlockSourceV3 {
    signal_rung: u16,
    horizon_bars: u32,
    requested_span: RequestedSpanV3,
    feed_id: [u8; 32],
    source_commit_id: [u8; 32],
    calendar_policy_id: [u8; 32],
    daily_policy_id: [u8; 32],
    vocabulary_id: [u8; 32],
    evaluation_policy_id: [u8; 32],
    nifty: FamilySourceV3,
    banknifty: FamilySourceV3,
    observation: ObservationSourceV3,
    statistics: StatisticsSourceV3,
    search: SearchSourceV4,
    base: BaseSourceV3,
    policy: [u8; ADMISSION_V3_POLICY_BYTES],
    policy_digest: [u8; 32],
    nifty_decision_count: u64,
    banknifty_decision_count: u64,
    decision_count: u64,
    block_id: [u8; 32],
}

impl BlockSourceV3 {
    fn validate(&self) -> Result<(), PopulationAdmissionV3Refusal> {
        if self.signal_rung == 0 || self.horizon_bars == 0 {
            return Err("Admission V3 rung and horizon must be nonzero".to_owned());
        }
        self.requested_span.validate()?;
        for (name, value) in [
            ("feed", self.feed_id),
            ("source commit", self.source_commit_id),
            ("calendar policy", self.calendar_policy_id),
            ("daily-reference policy", self.daily_policy_id),
            ("vocabulary", self.vocabulary_id),
            ("evaluation policy", self.evaluation_policy_id),
        ] {
            require_nonzero(&format!("Admission V3 {name}"), value)?;
        }
        self.nifty.validate("NIFTY")?;
        self.banknifty.validate("BANKNIFTY")?;
        self.validate_distinct_families()?;
        self.observation.validate()?;
        self.statistics.validate()?;
        self.search.validate()?;
        self.base.validate()?;
        self.validate_source_joins()?;
        self.validate_cardinalities()?;
        AdmissionPolicyV1::from_canonical_bytes(&self.policy)
            .map_err(|why| format!("Admission V3 canonical Runner policy refused: {why:?}"))?;
        if self.policy_digest != hash_slices(POLICY_DIGEST_DOMAIN, &[&self.policy]) {
            return Err("Admission V3 policy digest differs from exact policy bytes".to_owned());
        }
        if self.block_id != self.derive_block_id()? {
            return Err("Admission V3 block identity differs from canonical sources".to_owned());
        }
        Ok(())
    }

    fn validate_distinct_families(&self) -> Result<(), PopulationAdmissionV3Refusal> {
        for (name, nifty, banknifty) in [
            (
                "Candidate universe",
                self.nifty.candidate_universe_id,
                self.banknifty.candidate_universe_id,
            ),
            (
                "Candidate Completion",
                self.nifty.candidate_completion_digest,
                self.banknifty.candidate_completion_digest,
            ),
            (
                "Pre-Admission authority",
                self.nifty.pre_admission_authority_id,
                self.banknifty.pre_admission_authority_id,
            ),
        ] {
            if nifty == banknifty {
                return Err(format!(
                    "Admission V3 NIFTY and BANKNIFTY {name} identities alias"
                ));
            }
        }
        Ok(())
    }

    fn validate_source_joins(&self) -> Result<(), PopulationAdmissionV3Refusal> {
        if self.observation.authority_id != self.statistics.observation_authority_id
            || self.observation.completion_digest != self.statistics.observation_completion_digest
            || self.observation.pair_id != self.statistics.observation_pair_id
        {
            return Err("Admission V3 Statistics crosswires Observation authority".to_owned());
        }
        if self.observation.period_count != self.statistics.period_count {
            return Err("Admission V3 Observation and Statistics period counts differ".to_owned());
        }
        for (family, candidate, search) in [
            ("NIFTY", &self.nifty, &self.search.nifty),
            ("BANKNIFTY", &self.banknifty, &self.search.banknifty),
        ] {
            if candidate.signal != search.signal {
                return Err(format!(
                    "Admission V3 {family} Candidate and exact-grid Search V4 signal/column sources differ"
                ));
            }
            if candidate.exit_grid.long_policy != search.long_policy_id
                || candidate.exit_grid.long_resolution != search.long_resolution_id
                || candidate.exit_grid.short_policy != search.short_policy_id
                || candidate.exit_grid.short_resolution != search.short_resolution_id
            {
                return Err(format!(
                    "Admission V3 {family} Candidate and exact-grid Search V4 Long/Short sources differ"
                ));
            }
        }
        Ok(())
    }

    fn validate_cardinalities(&self) -> Result<(), PopulationAdmissionV3Refusal> {
        let family_total = self
            .nifty_decision_count
            .checked_add(self.banknifty_decision_count)
            .ok_or_else(|| "Admission V3 family decision total overflowed".to_owned())?;
        if family_total != self.decision_count
            || self.nifty_decision_count != self.nifty.candidate_count
            || self.banknifty_decision_count != self.banknifty.candidate_count
        {
            return Err(
                "Admission V3 decisions do not exactly cover Candidate families".to_owned(),
            );
        }
        if self.decision_count != self.observation.candidate_count
            || self.decision_count != self.statistics.candidate_count
        {
            return Err(
                "Admission V3 Candidate, Observation and Statistics cardinalities differ"
                    .to_owned(),
            );
        }
        // Search counts are folds, not candidates. Comparing them here would
        // manufacture a false cross-domain cardinality invariant.
        Ok(())
    }

    fn derive_block_id(&self) -> Result<[u8; 32], PopulationAdmissionV3Refusal> {
        let mut bytes = [0_u8; BLOCK_IDENTITY_BYTES];
        let mut writer = FixedWriter::new(&mut bytes);
        self.encode_without_id(&mut writer)?;
        writer.require_full("Admission V3 block identity")?;
        Ok(hash_slices(BLOCK_ID_DOMAIN, &[&bytes]))
    }

    fn encode_without_id(
        &self,
        writer: &mut FixedWriter<'_>,
    ) -> Result<(), PopulationAdmissionV3Refusal> {
        writer.u16(self.signal_rung)?;
        writer.zeros(2)?;
        writer.u32(self.horizon_bars)?;
        self.requested_span.encode(writer)?;
        for value in [
            self.feed_id,
            self.source_commit_id,
            self.calendar_policy_id,
            self.daily_policy_id,
            self.vocabulary_id,
            self.evaluation_policy_id,
        ] {
            writer.array(&value)?;
        }
        self.nifty.encode(writer)?;
        self.banknifty.encode(writer)?;
        self.observation.encode(writer)?;
        self.statistics.encode(writer)?;
        self.search.encode(writer)?;
        self.base.encode(writer)?;
        writer.array(&self.policy)?;
        writer.array(&self.policy_digest)?;
        writer.u64(self.nifty_decision_count)?;
        writer.u64(self.banknifty_decision_count)?;
        writer.u64(self.decision_count)
    }

    fn encode(&self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationAdmissionV3Refusal> {
        self.encode_without_id(writer)?;
        writer.array(&self.block_id)
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationAdmissionV3Refusal> {
        let signal_rung = reader.u16()?;
        reader.require_zeros(2, "Admission V3 block rung reserve")?;
        let value = Self {
            signal_rung,
            horizon_bars: reader.u32()?,
            requested_span: RequestedSpanV3::decode(reader)?,
            feed_id: reader.array()?,
            source_commit_id: reader.array()?,
            calendar_policy_id: reader.array()?,
            daily_policy_id: reader.array()?,
            vocabulary_id: reader.array()?,
            evaluation_policy_id: reader.array()?,
            nifty: FamilySourceV3::decode(reader)?,
            banknifty: FamilySourceV3::decode(reader)?,
            observation: ObservationSourceV3::decode(reader)?,
            statistics: StatisticsSourceV3::decode(reader)?,
            search: SearchSourceV4::decode(reader)?,
            base: BaseSourceV3::decode(reader)?,
            policy: reader.array()?,
            policy_digest: reader.array()?,
            nifty_decision_count: reader.u64()?,
            banknifty_decision_count: reader.u64()?,
            decision_count: reader.u64()?,
            block_id: reader.array()?,
        };
        value.validate()?;
        Ok(value)
    }
}

#[derive(Clone, Copy)]
struct RunnerDecisionViewV3<'a> {
    policy: &'a [u8],
    evidence: &'a [u8],
    verdict: &'a [u8],
    status: AdmissionV3Status,
    candidate_semantic_id: [u8; 32],
    statistics_audit_id: [u8; 32],
    statistics_completion_digest: [u8; 32],
    observation_statistics_link_id: [u8; 32],
    cscv_policy_id: [u8; 32],
    ordered_split_id: [u8; 32],
    white_family_id: [u8; 32],
    spa_family_id: [u8; 32],
    romano_wolf_family_id: [u8; 32],
    split_count: u64,
    draw_count: u64,
    candidate_count: u64,
    period_count: u64,
    search_policy_id: [u8; 32],
    search_family_id: [u8; 32],
    search_walk_id: [u8; 32],
    search_source_authority_id: [u8; 32],
    fold_count: u64,
    decided_folds: u64,
    profitable_oos_folds: u64,
    aggregate_oos_paisa: i64,
}

impl RunnerDecisionViewV3<'_> {
    fn parse(
        bytes: &[u8; RUNNER_ADMISSION_V3_DECISION_BYTES],
    ) -> Result<RunnerDecisionViewV3<'_>, PopulationAdmissionV3Refusal> {
        require_runner_header(
            bytes,
            4,
            runner::admission::ADMISSION_VERSION_V3,
            RUNNER_DECISION_PAYLOAD_BYTES,
            "decision",
        )?;
        let policy = slice_at(
            bytes,
            RUNNER_POLICY_OFFSET,
            ADMISSION_V3_POLICY_BYTES,
            "policy",
        )?;
        let evidence = slice_at(
            bytes,
            RUNNER_EVIDENCE_OFFSET,
            RUNNER_ADMISSION_V3_EVIDENCE_BYTES,
            "evidence",
        )?;
        require_runner_header(
            evidence,
            2,
            runner::admission::ADMISSION_VERSION_V3,
            RUNNER_EVIDENCE_PAYLOAD_BYTES,
            "evidence",
        )?;
        let verdict = slice_at(
            bytes,
            RUNNER_VERDICT_OFFSET,
            RUNNER_VERDICT_BYTES,
            "verdict",
        )?;
        let decoded_verdict = AdmissionVerdictV1::from_canonical_bytes(verdict)
            .map_err(|why| format!("Admission V3 Runner verdict refused: {why:?}"))?;
        let statistics = EVIDENCE_STATISTICS_OFFSET;
        let walk = EVIDENCE_WALK_OFFSET;
        Ok(RunnerDecisionViewV3 {
            policy,
            evidence,
            verdict,
            status: AdmissionV3Status::from_runner(decoded_verdict.status()),
            candidate_semantic_id: array_at(evidence, statistics, "candidate semantic")?,
            statistics_audit_id: array_at(evidence, statistics + 32, "Statistics audit")?,
            statistics_completion_digest: array_at(
                evidence,
                statistics + 64,
                "Statistics Completion",
            )?,
            observation_statistics_link_id: array_at(
                evidence,
                statistics + 96,
                "Observation-to-Statistics link",
            )?,
            cscv_policy_id: array_at(evidence, statistics + 128, "CSCV policy")?,
            ordered_split_id: array_at(evidence, statistics + 160, "ordered split family")?,
            white_family_id: array_at(evidence, statistics + 192, "White family")?,
            spa_family_id: array_at(evidence, statistics + 224, "SPA family")?,
            romano_wolf_family_id: array_at(evidence, statistics + 256, "Romano-Wolf family")?,
            split_count: u64_at(evidence, statistics + 320, "CSCV split count")?,
            draw_count: u64_at(evidence, statistics + 424, "bootstrap draws")?,
            candidate_count: u64_at(evidence, statistics + 432, "bootstrap strategies")?,
            period_count: u64_at(evidence, statistics + 440, "bootstrap periods")?,
            search_policy_id: array_at(evidence, walk, "Search policy")?,
            search_family_id: array_at(evidence, walk + 32, "Search family")?,
            search_walk_id: array_at(evidence, walk + 64, "Search walk")?,
            search_source_authority_id: array_at(evidence, walk + 96, "Search authority")?,
            fold_count: u64_at(evidence, walk + 128, "Search fold count")?,
            decided_folds: u64_at(evidence, walk + 136, "Search decided folds")?,
            profitable_oos_folds: u64_at(evidence, walk + 144, "Search profitable folds")?,
            aggregate_oos_paisa: i64_at(evidence, walk + 152, "Search aggregate OOS")?,
        })
    }

    fn validate_sources(
        &self,
        source: &BlockSourceV3,
        candidate_semantic_id: [u8; 32],
        search: &SearchMemberV4,
    ) -> Result<(), PopulationAdmissionV3Refusal> {
        if self.policy != source.policy
            || self.candidate_semantic_id != candidate_semantic_id
            || self.statistics_audit_id != source.statistics.audit_id
            || self.statistics_completion_digest != source.statistics.completion_digest
            || self.observation_statistics_link_id
                != source.statistics.observation_statistics_link_id
            || self.cscv_policy_id != source.statistics.cscv_policy_id
            || self.ordered_split_id != source.statistics.ordered_split_id
            || self.white_family_id != source.statistics.white_family_id
            || self.spa_family_id != source.statistics.spa_family_id
            || self.romano_wolf_family_id != source.statistics.romano_wolf_family_id
            || self.split_count != source.statistics.split_count
            || self.draw_count != source.statistics.draw_count
            || self.candidate_count != source.statistics.candidate_count
            || self.period_count != source.statistics.period_count
        {
            return Err(
                "Admission V3 Runner decision crosswires Statistics/policy source".to_owned(),
            );
        }
        if self.search_policy_id != search.validation_policy_id
            || self.search_family_id != search.validation_family_id
            || self.search_walk_id != search.walk_id
            || self.search_source_authority_id == [0; 32]
            || self.fold_count != search.fold_count
            || self.decided_folds != search.decided_folds
            || self.profitable_oos_folds != search.profitable_oos_folds
            || self.aggregate_oos_paisa != search.aggregate_oos_paisa
        {
            return Err("Admission V3 Runner decision crosswires Search authority".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AdmissionDecisionRecordV3 {
    block_id: [u8; 32],
    global_sequence: u64,
    family: AdmissionV3Family,
    family_sequence: u64,
    status: AdmissionV3Status,
    candidate_semantic_id: [u8; 32],
    candidate_row_digest: [u8; 32],
    pre_admission_authority_id: [u8; 32],
    statistics_period_digest: [u8; 32],
    statistics_split_digest: [u8; 32],
    family_search_member_id: [u8; 32],
    base_evidence_id: [u8; 32],
    runner_decision: [u8; RUNNER_ADMISSION_V3_DECISION_BYTES],
    runner_decision_digest: [u8; 32],
    evidence_digest: [u8; 32],
    verdict_digest: [u8; 32],
    decision_id: [u8; 32],
}

impl AdmissionDecisionRecordV3 {
    fn validate(&self) -> Result<RunnerDecisionViewV3<'_>, PopulationAdmissionV3Refusal> {
        for (name, value) in [
            ("block", self.block_id),
            ("candidate semantic", self.candidate_semantic_id),
            ("candidate row", self.candidate_row_digest),
            ("Pre-Admission", self.pre_admission_authority_id),
            ("Statistics period", self.statistics_period_digest),
            ("Statistics split", self.statistics_split_digest),
            ("Search member", self.family_search_member_id),
            ("Base-Evidence", self.base_evidence_id),
            ("Runner decision", self.runner_decision_digest),
            ("Runner evidence", self.evidence_digest),
            ("Runner verdict", self.verdict_digest),
            ("decision", self.decision_id),
        ] {
            require_nonzero(&format!("Admission V3 {name}"), value)?;
        }
        let runner = RunnerDecisionViewV3::parse(&self.runner_decision)?;
        if self.runner_decision_digest
            != hash_slices(RUNNER_DECISION_DIGEST_DOMAIN, &[&self.runner_decision])
            || self.evidence_digest
                != hash_slices(RUNNER_EVIDENCE_DIGEST_DOMAIN, &[runner.evidence])
            || self.verdict_digest != hash_slices(RUNNER_VERDICT_DIGEST_DOMAIN, &[runner.verdict])
        {
            return Err("Admission V3 Runner decision/evidence/verdict digest mismatch".to_owned());
        }
        if self.status != runner.status
            || self.candidate_semantic_id != runner.candidate_semantic_id
        {
            return Err("Admission V3 outer status/candidate differs from Runner V3".to_owned());
        }
        if self.decision_id != self.derive_decision_id() {
            return Err("Admission V3 decision identity differs from canonical fields".to_owned());
        }
        Ok(runner)
    }

    fn validate_against(&self, source: &BlockSourceV3) -> Result<(), PopulationAdmissionV3Refusal> {
        let (pre_admission, search) = match self.family {
            AdmissionV3Family::Nifty => (
                source.nifty.pre_admission_authority_id,
                &source.search.nifty,
            ),
            AdmissionV3Family::BankNifty => (
                source.banknifty.pre_admission_authority_id,
                &source.search.banknifty,
            ),
        };
        if self.block_id != source.block_id
            || self.pre_admission_authority_id != pre_admission
            || self.family_search_member_id != search.member_id
        {
            return Err("Admission V3 decision crosswires block/family authority".to_owned());
        }
        self.validate()?
            .validate_sources(source, self.candidate_semantic_id, search)
    }

    fn derive_decision_id(&self) -> [u8; 32] {
        hash_slices(
            DECISION_ID_DOMAIN,
            &[
                &VERSION.to_le_bytes(),
                &self.block_id,
                &self.global_sequence.to_le_bytes(),
                &[self.family as u8],
                &[0; 7],
                &self.family_sequence.to_le_bytes(),
                &[self.status as u8],
                &[0; 7],
                &self.candidate_semantic_id,
                &self.candidate_row_digest,
                &self.pre_admission_authority_id,
                &self.statistics_period_digest,
                &self.statistics_split_digest,
                &self.family_search_member_id,
                &self.base_evidence_id,
                &self.runner_decision_digest,
                &self.evidence_digest,
                &self.verdict_digest,
            ],
        )
    }
}

/// Privately decoded Admission V3 row paired with its literal sealed record.
///
/// Its private constructor decodes one fixed record, then proves that canonical
/// re-encoding reproduces every retained byte.  Ledger callers additionally
/// hold the generation lock; the embedded-record verifier deliberately proves
/// byte integrity without claiming that separate provenance.  Callers receive
/// neither a constructor nor mutable access to either representation.
#[derive(Clone, Debug, PartialEq, Eq)]
struct AuthenticatedAdmissionDecisionRecordV3 {
    decision: AdmissionDecisionRecordV3,
    canonical_record: [u8; POPULATION_ADMISSION_V3_DECISION_BYTES],
}

impl AuthenticatedAdmissionDecisionRecordV3 {
    fn from_canonical_record(
        canonical_record: &[u8; POPULATION_ADMISSION_V3_DECISION_BYTES],
    ) -> Result<Self, PopulationAdmissionV3Refusal> {
        let decision = decode_decision(canonical_record)?;
        if encode_decision(&decision)? != *canonical_record {
            return Err(
                "Admission V3 decoded decision does not reproduce its literal sealed record"
                    .to_owned(),
            );
        }
        Ok(Self {
            decision,
            canonical_record: *canonical_record,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AdmissionCompletionRecordV3 {
    block_sequence: u64,
    first_decision_record: u64,
    source: BlockSourceV3,
    admitted_count: u64,
    rejected_count: u64,
    unmeasured_count: u64,
    refused_count: u64,
    ordered_decision_digest: [u8; 32],
    completion_id: [u8; 32],
}

impl AdmissionCompletionRecordV3 {
    fn validate(&self) -> Result<(), PopulationAdmissionV3Refusal> {
        self.source.validate()?;
        require_nonzero(
            "Admission V3 ordered decisions",
            self.ordered_decision_digest,
        )?;
        require_nonzero("Admission V3 Completion", self.completion_id)?;
        let classified = checked_sum(
            &[
                self.admitted_count,
                self.rejected_count,
                self.unmeasured_count,
                self.refused_count,
            ],
            "Admission V3 status counts",
        )?;
        if classified != self.source.decision_count {
            return Err("Admission V3 status counts differ from decision count".to_owned());
        }
        if self.completion_id != self.derive_completion_id() {
            return Err(
                "Admission V3 Completion identity differs from canonical fields".to_owned(),
            );
        }
        Ok(())
    }

    fn derive_completion_id(&self) -> [u8; 32] {
        hash_slices(
            COMPLETION_ID_DOMAIN,
            &[
                &VERSION.to_le_bytes(),
                &self.source.block_id,
                &self.source.decision_count.to_le_bytes(),
                &self.source.nifty_decision_count.to_le_bytes(),
                &self.source.banknifty_decision_count.to_le_bytes(),
                &self.admitted_count.to_le_bytes(),
                &self.rejected_count.to_le_bytes(),
                &self.unmeasured_count.to_le_bytes(),
                &self.refused_count.to_le_bytes(),
                &self.ordered_decision_digest,
            ],
        )
    }
}

fn family_source_from_committed_v3(
    source: &CommittedStoredCandidatePreAdmissionV1,
) -> Result<FamilySourceV3, PopulationAdmissionV3Refusal> {
    let committed = source.candidate_pre_admission();
    let reopened = committed.identities();
    let candidate = committed.candidate_audit().receipt();
    let signal = candidate.signal_stream();
    let value = FamilySourceV3 {
        candidate_universe_id: reopened.candidate_universe_id(),
        candidate_completion_digest: reopened.candidate_completion_digest(),
        pre_admission_authority_id: reopened.pre_admission_authority_id(),
        signal: SignalSourceV3 {
            digest: signal.digest(),
            bars: signal.count(),
            first_ts_micros: signal.first_ts_micros(),
            last_ts_micros: signal.last_ts_micros(),
            column_digest: candidate.signal_column_digest(),
        },
        exit_grid: ExitGridSourceV3::from_candidate(candidate.identities().exit_grids())?,
        candidate_count: reopened.candidate_row_count(),
    };
    value.validate("committed")?;
    Ok(value)
}

fn require_same_candidate_cohort_v3(
    committed: &CommittedStoredObservationStatisticsV2,
) -> Result<(), PopulationAdmissionV3Refusal> {
    let nifty = committed
        .nifty_source()
        .candidate_pre_admission()
        .candidate_audit()
        .receipt();
    let banknifty = committed
        .banknifty_source()
        .candidate_pre_admission()
        .candidate_audit()
        .receipt();
    let left = nifty.identities();
    let right = banknifty.identities();
    if nifty.rung_seconds() != banknifty.rung_seconds()
        || nifty.horizon_bars() != banknifty.horizon_bars()
        || nifty.requested_span() != banknifty.requested_span()
        || left.feed_digest() != right.feed_digest()
        || left.source_commit_digest() != right.source_commit_digest()
        || left.vocabulary_digest() != right.vocabulary_digest()
        || left.evaluation_policy_digest() != right.evaluation_policy_digest()
        || left.calendar_policy_digest() != right.calendar_policy_digest()
        || left.daily_reference_policy_digest() != right.daily_reference_policy_digest()
    {
        return Err(
            "Admission V3 NIFTY and BANKNIFTY Candidate authorities do not share one exact cohort"
                .to_owned(),
        );
    }
    let statistics = *committed.projection_source();
    if statistics.rung_seconds() != nifty.rung_seconds()
        || statistics.horizon_bars() != nifty.horizon_bars()
        || statistics.requested_span() != nifty.requested_span()
        || statistics.feed_digest() != left.feed_digest()
        || statistics.source_commit_digest() != left.source_commit_digest()
        || statistics.calendar_policy_digest() != left.calendar_policy_digest()
        || statistics.daily_reference_policy_digest() != left.daily_reference_policy_digest()
    {
        return Err(
            "Admission V3 Statistics authority does not share the exact Candidate cohort"
                .to_owned(),
        );
    }
    Ok(())
}

/// Prepares one complete Population Admission V3 block from authenticated
/// Candidate, Observation, Statistics, Base and exact-grid Search V4 sources.
///
/// Search V3 is neither accepted nor converted. Runner arithmetic is recomputed
/// per Candidate directly from the nonconstructible retained Search V4 pair,
/// the freshly joined Base record and the freshly joined Statistics record.
/// The returned preparation is authenticated input, not durable output:
/// durable authority exists only after receipt-last append, sync, fresh reopen
/// and exact authentication.
///
/// # Errors
///
/// Refuses any Candidate cohort, Long/Short grid component, Observation,
/// Statistics, Base ordinal, Search V4 seal, arithmetic, allocation, ordering
/// or count mismatch. No partial preparation is returned.
#[expect(
    clippy::too_many_lines,
    reason = "the preparation door spells out every independently authenticated source and every per-candidate equality join"
)]
pub(crate) fn prepare_population_admission_v3_exact_grid(
    committed: &mut CommittedStoredObservationStatisticsV2,
    policy: &AdmissionPolicyV1,
) -> Result<PreparedPopulationAdmissionV3, PopulationAdmissionV3Refusal> {
    require_same_candidate_cohort_v3(committed)?;
    let nifty_audit = committed
        .nifty_source()
        .candidate_pre_admission()
        .candidate_audit();
    let banknifty_audit = committed
        .banknifty_source()
        .candidate_pre_admission()
        .candidate_audit();
    let nifty_receipt = nifty_audit.receipt();
    let candidate_identities = nifty_receipt.identities();
    let requested = nifty_receipt.requested_span();
    let statistics_projection = *committed.projection_source();
    let observation_audit = committed.observation_commit().audit();
    let base = *committed.base_evidence();
    let base_candidate_count = base
        .nifty_audit()
        .record_count()
        .checked_add(base.banknifty_audit().record_count())
        .ok_or_else(|| "Admission V3 paired Base candidate count overflowed".to_owned())?;
    let nifty_family = family_source_from_committed_v3(committed.nifty_source())?;
    let banknifty_family = family_source_from_committed_v3(committed.banknifty_source())?;
    let retained_search = committed.search_pair_v4()?;
    let search = SearchSourceV4::from_retained(&retained_search, &nifty_family, &banknifty_family)?;
    let policy_bytes = policy.canonical_bytes();
    let mut source = BlockSourceV3 {
        signal_rung: u16::try_from(nifty_receipt.rung_seconds())
            .map_err(|why| format!("Admission V3 signal rung does not fit u16: {why}"))?,
        horizon_bars: nifty_receipt.horizon_bars(),
        requested_span: RequestedSpanV3 {
            from_year: requested.from_year(),
            from_month: requested.from_month(),
            to_year: requested.to_year(),
            to_month: requested.to_month(),
        },
        feed_id: candidate_identities.feed_digest(),
        source_commit_id: candidate_identities.source_commit_digest(),
        calendar_policy_id: candidate_identities.calendar_policy_digest(),
        daily_policy_id: candidate_identities.daily_reference_policy_digest(),
        vocabulary_id: candidate_identities.vocabulary_digest(),
        evaluation_policy_id: candidate_identities.evaluation_policy_digest(),
        nifty: nifty_family,
        banknifty: banknifty_family,
        observation: ObservationSourceV3 {
            authority_id: observation_audit.authority_id(),
            completion_digest: observation_audit.completion_digest(),
            pair_id: observation_audit.pair_identity(),
            source_id: observation_audit.source_identity(),
            observation_policy_id: observation_audit.observation_policy_digest(),
            layout_policy_id: observation_audit.layout_policy_digest(),
            candidate_count: observation_audit.candidate_count(),
            period_count: observation_audit.period_count(),
        },
        statistics: StatisticsSourceV3 {
            audit_id: statistics_projection.audit_id(),
            completion_digest: statistics_projection.completion_record_digest(),
            observation_authority_id: statistics_projection.observation_authority_id(),
            observation_completion_digest: observation_audit.completion_digest(),
            observation_pair_id: statistics_projection.observation_pair_identity(),
            observation_statistics_link_id: statistics_projection.observation_statistics_link_id(),
            observation_projection_policy_id: statistics_projection
                .observation_projection_policy_digest(),
            wilson_policy_id: statistics_projection.wilson_policy_digest(),
            cscv_policy_id: statistics_projection.cscv_policy_digest(),
            ordered_candidate_id: statistics_projection.ordered_candidate_digest(),
            ordered_period_id: statistics_projection.ordered_period_digest(),
            ordered_split_id: statistics_projection.cscv_split_family_digest(),
            white_family_id: statistics_projection.white().family_digest(),
            spa_family_id: statistics_projection.spa().family_digest(),
            romano_wolf_family_id: statistics_projection.romano_wolf_family_digest(),
            candidate_count: statistics_projection.candidate_count(),
            period_count: statistics_projection.period_count(),
            split_count: statistics_projection.split_count(),
            draw_count: statistics_projection.bootstrap_draws(),
            bootstrap_seed: statistics_projection.bootstrap_seed(),
            bootstrap_block_length: statistics_projection.bootstrap_block_length(),
        },
        search,
        base: BaseSourceV3 {
            paired_base: base.pair_id(),
            nifty_completion: base.nifty_audit().completion_id(),
            banknifty_completion: base.banknifty_audit().completion_id(),
        },
        policy: policy_bytes,
        policy_digest: hash_slices(POLICY_DIGEST_DOMAIN, &[&policy_bytes]),
        nifty_decision_count: nifty_audit.row_count(),
        banknifty_decision_count: banknifty_audit.row_count(),
        decision_count: statistics_projection.candidate_count(),
        block_id: [0; 32],
    };
    source.block_id = source.derive_block_id()?;
    source.validate()?;
    if base_candidate_count != source.decision_count {
        return Err(
            "Admission V3 Candidate/Statistics total differs from paired Base total".to_owned(),
        );
    }

    let decision_capacity = usize::try_from(source.decision_count)
        .map_err(|why| format!("Admission V3 decision count does not fit usize: {why}"))?;
    let mut decisions = Vec::new();
    decisions
        .try_reserve_exact(decision_capacity)
        .map_err(|why| format!("Admission V3 cannot reserve exact decision family: {why}"))?;
    let admission_inputs = committed.admission_candidate_inputs_family_v3()?;
    if admission_inputs.len() != decision_capacity {
        return Err("Admission V3 authenticated input cardinality differs".to_owned());
    }
    for (sequence, inputs) in admission_inputs.iter().enumerate() {
        let sequence = u64::try_from(sequence)
            .map_err(|why| format!("Admission V3 decision ordinal does not fit u64: {why}"))?;
        let statistics = inputs.statistics();
        let base = inputs.base();
        let (family, search_validation, search_member) = match statistics.family() {
            InstrumentFamilyV1::Nifty => (
                AdmissionV3Family::Nifty,
                retained_search.nifty().validation(),
                &source.search.nifty,
            ),
            InstrumentFamilyV1::BankNifty => (
                AdmissionV3Family::BankNifty,
                retained_search.banknifty().validation(),
                &source.search.banknifty,
            ),
        };
        let arithmetic = (*policy)
            .evaluate_v3_exact_grid_projection(
                base.record().admission_values(),
                statistics.draft(),
                search_validation,
            )
            .map_err(|why| format!("Admission V3 exact-grid Runner arithmetic refused: {why:?}"))?;
        let runner_decision = arithmetic.decision_bytes();
        let evidence = &runner_decision[RUNNER_EVIDENCE_OFFSET..RUNNER_VERDICT_OFFSET];
        let verdict = &runner_decision[RUNNER_VERDICT_OFFSET..];
        let mut decision = AdmissionDecisionRecordV3 {
            block_id: source.block_id,
            global_sequence: sequence,
            family,
            family_sequence: statistics.family_sequence(),
            status: AdmissionV3Status::from_runner(arithmetic.status()),
            candidate_semantic_id: statistics.candidate_semantic_digest(),
            candidate_row_digest: base.record().candidate_row_digest(),
            pre_admission_authority_id: statistics.pre_admission_authority_id(),
            statistics_period_digest: statistics.candidate_ordered_period_digest(),
            statistics_split_digest: statistics.candidate_ordered_split_digest(),
            family_search_member_id: search_member.member_id,
            base_evidence_id: base.record().evidence_id(),
            runner_decision,
            runner_decision_digest: hash_slices(RUNNER_DECISION_DIGEST_DOMAIN, &[&runner_decision]),
            evidence_digest: hash_slices(RUNNER_EVIDENCE_DIGEST_DOMAIN, &[evidence]),
            verdict_digest: hash_slices(RUNNER_VERDICT_DIGEST_DOMAIN, &[verdict]),
            decision_id: [0; 32],
        };
        decision.decision_id = decision.derive_decision_id();
        decision.validate_against(&source)?;
        decisions.push(decision);
    }
    let prepared = PreparedPopulationAdmissionV3 {
        source,
        decisions,
        base_candidate_count,
    };
    prepared.validate()?;
    Ok(prepared)
}

/// Opaque authenticated-input preparation for one Admission V3 block.
///
/// The production constructor accepts only opaque Search V4 and already joined
/// Step-3 capabilities. Current Search V3 omits required exact-grid identity
/// and cannot promote this ledger. This module accepts neither raw search/walk
/// values nor caller-authored Search identities.
pub(crate) struct PreparedPopulationAdmissionV3 {
    source: BlockSourceV3,
    decisions: Vec<AdmissionDecisionRecordV3>,
    base_candidate_count: u64,
}

impl PreparedPopulationAdmissionV3 {
    fn decision_count(&self) -> Result<u64, PopulationAdmissionV3Refusal> {
        u64::try_from(self.decisions.len())
            .map_err(|_| "Admission V3 prepared decision count does not fit u64".to_owned())
    }

    fn validate(&self) -> Result<(), PopulationAdmissionV3Refusal> {
        self.source.validate()?;
        let count = self.decision_count()?;
        if count != self.source.decision_count || count != self.base_candidate_count {
            return Err(
                "Admission V3 Candidate/decision total differs from authenticated Base count"
                    .to_owned(),
            );
        }
        let nifty_count = usize::try_from(self.source.nifty_decision_count)
            .map_err(|_| "Admission V3 NIFTY decision count does not fit usize".to_owned())?;
        let mut semantics = bounded_set(self.decisions.len(), "candidate semantic")?;
        let mut rows = bounded_set(self.decisions.len(), "candidate row")?;
        let mut periods = bounded_set(self.decisions.len(), "Statistics period")?;
        let mut splits = bounded_set(self.decisions.len(), "Statistics split")?;
        let mut base = bounded_set(self.decisions.len(), "Base-Evidence")?;
        let mut decisions = bounded_set(self.decisions.len(), "decision")?;
        for (index, decision) in self.decisions.iter().enumerate() {
            decision.validate_against(&self.source)?;
            let global = u64::try_from(index)
                .map_err(|_| "Admission V3 decision index does not fit u64".to_owned())?;
            let (family, family_sequence) = if index < nifty_count {
                (AdmissionV3Family::Nifty, global)
            } else {
                (
                    AdmissionV3Family::BankNifty,
                    u64::try_from(index - nifty_count).map_err(|_| {
                        "Admission V3 BANKNIFTY sequence does not fit u64".to_owned()
                    })?,
                )
            };
            if decision.global_sequence != global
                || decision.family != family
                || decision.family_sequence != family_sequence
            {
                return Err(format!(
                    "Admission V3 decision {index} violates NIFTY-first ordering"
                ));
            }
            for (name, inserted) in [
                (
                    "candidate semantic",
                    semantics.insert(decision.candidate_semantic_id),
                ),
                ("candidate row", rows.insert(decision.candidate_row_digest)),
                (
                    "Statistics period",
                    periods.insert(decision.statistics_period_digest),
                ),
                (
                    "Statistics split",
                    splits.insert(decision.statistics_split_digest),
                ),
                ("Base-Evidence", base.insert(decision.base_evidence_id)),
                ("decision", decisions.insert(decision.decision_id)),
            ] {
                if !inserted {
                    return Err(format!("Admission V3 duplicate {name} identity"));
                }
            }
        }
        Ok(())
    }

    fn expected_completion(
        &self,
        block_sequence: u64,
        first_decision_record: u64,
    ) -> Result<AdmissionCompletionRecordV3, PopulationAdmissionV3Refusal> {
        self.validate()?;
        let mut admitted_count = 0_u64;
        let mut rejected_count = 0_u64;
        let mut unmeasured_count = 0_u64;
        let mut refused_count = 0_u64;
        for decision in &self.decisions {
            let target = match decision.status {
                AdmissionV3Status::Admitted => &mut admitted_count,
                AdmissionV3Status::Rejected => &mut rejected_count,
                AdmissionV3Status::Unmeasured => &mut unmeasured_count,
                AdmissionV3Status::Refused => &mut refused_count,
            };
            *target = target
                .checked_add(1)
                .ok_or_else(|| "Admission V3 status count overflowed".to_owned())?;
        }
        let mut completion = AdmissionCompletionRecordV3 {
            block_sequence,
            first_decision_record,
            source: self.source.clone(),
            admitted_count,
            rejected_count,
            unmeasured_count,
            refused_count,
            ordered_decision_digest: ordered_decision_digest(&self.decisions)?,
            completion_id: [0; 32],
        };
        completion.completion_id = completion.derive_completion_id();
        completion.validate()?;
        Ok(completion)
    }
}

/// Structurally valid, non-authoritative receipt from a public bounded reopen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationAdmissionV3StructuralReceipt {
    block_id: [u8; 32],
    completion_id: [u8; 32],
    block_sequence: u64,
    first_decision_record: u64,
    decision_count: u64,
    nifty_decision_count: u64,
    banknifty_decision_count: u64,
    ordered_decision_digest: [u8; 32],
}

impl PopulationAdmissionV3StructuralReceipt {
    /// Semantic block identity.
    #[must_use]
    pub(crate) const fn block_id(self) -> [u8; 32] {
        self.block_id
    }

    /// Receipt-last Completion identity.
    #[must_use]
    pub(crate) const fn completion_id(self) -> [u8; 32] {
        self.completion_id
    }

    /// Physical Completion sequence.
    #[must_use]
    pub(crate) const fn block_sequence(self) -> u64 {
        self.block_sequence
    }

    /// First physical decision record.
    #[must_use]
    pub(crate) const fn first_decision_record(self) -> u64 {
        self.first_decision_record
    }

    /// Complete Candidate/decision count.
    #[must_use]
    pub(crate) const fn decision_count(self) -> u64 {
        self.decision_count
    }

    /// NIFTY decision prefix count.
    #[must_use]
    pub(crate) const fn nifty_decision_count(self) -> u64 {
        self.nifty_decision_count
    }

    /// BANKNIFTY decision suffix count.
    #[must_use]
    pub(crate) const fn banknifty_decision_count(self) -> u64 {
        self.banknifty_decision_count
    }

    /// Digest of canonical ordered decision identities.
    #[must_use]
    pub(crate) const fn ordered_decision_digest(self) -> [u8; 32] {
        self.ordered_decision_digest
    }
}

/// Authenticated fixed-offset Admission V3 projection for one Candidate.
///
/// The projection contains only identities, ordinals, status and bounded
/// arithmetic counts required by the append-only Finalization successor. It
/// exposes neither Runner bytes nor the privately decoded ledger record and it
/// has no caller-visible constructor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationAdmissionV3DecisionProjection {
    block_id: [u8; 32],
    completion_id: [u8; 32],
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
    runner_decision_digest: [u8; 32],
    runner_evidence_digest: [u8; 32],
    runner_verdict_digest: [u8; 32],
    decision_id: [u8; 32],
}

impl PopulationAdmissionV3DecisionProjection {
    fn from_authenticated(
        receipt: PopulationAdmissionV3StructuralReceipt,
        source: &BlockSourceV3,
        decision: &AdmissionDecisionRecordV3,
    ) -> Result<Self, PopulationAdmissionV3Refusal> {
        decision.validate_against(source)?;
        let (family_source, search, base_completion_id) = match decision.family {
            AdmissionV3Family::Nifty => (
                &source.nifty,
                &source.search.nifty,
                source.base.nifty_completion,
            ),
            AdmissionV3Family::BankNifty => (
                &source.banknifty,
                &source.search.banknifty,
                source.base.banknifty_completion,
            ),
        };
        Ok(Self {
            block_id: receipt.block_id(),
            completion_id: receipt.completion_id(),
            global_sequence: decision.global_sequence,
            family: decision.family,
            family_sequence: decision.family_sequence,
            status: decision.status,
            candidate_universe_id: family_source.candidate_universe_id,
            candidate_completion_digest: family_source.candidate_completion_digest,
            candidate_semantic_id: decision.candidate_semantic_id,
            candidate_row_digest: decision.candidate_row_digest,
            pre_admission_authority_id: decision.pre_admission_authority_id,
            statistics_audit_id: source.statistics.audit_id,
            statistics_completion_digest: source.statistics.completion_digest,
            statistics_period_digest: decision.statistics_period_digest,
            statistics_split_digest: decision.statistics_split_digest,
            search_pair_id: source.search.pair_id,
            search_member_id: search.member_id,
            search_signal_digest: search.signal.digest,
            search_signal_bars: search.signal.bars,
            search_signal_first_ts_micros: search.signal.first_ts_micros,
            search_signal_last_ts_micros: search.signal.last_ts_micros,
            search_signal_column_digest: search.signal.column_digest,
            search_policy_id: search.validation_policy_id,
            search_full_grid_id: search.full_grid_id,
            search_long_policy_id: search.long_policy_id,
            search_long_resolution_id: search.long_resolution_id,
            search_short_policy_id: search.short_policy_id,
            search_short_resolution_id: search.short_resolution_id,
            search_family_id: search.validation_family_id,
            search_walk_id: search.walk_id,
            search_fold_count: search.fold_count,
            search_decided_folds: search.decided_folds,
            search_profitable_oos_folds: search.profitable_oos_folds,
            search_aggregate_oos_paisa: search.aggregate_oos_paisa,
            search_evaluated_population_count: search.evaluated_population_count,
            paired_base_id: source.base.paired_base,
            base_completion_id,
            base_evidence_id: decision.base_evidence_id,
            runner_decision_digest: decision.runner_decision_digest,
            runner_evidence_digest: decision.evidence_digest,
            runner_verdict_digest: decision.verdict_digest,
            decision_id: decision.decision_id,
        })
    }

    /// Population Admission block identity.
    #[must_use]
    pub(crate) const fn block_id(&self) -> [u8; 32] {
        self.block_id
    }

    /// Receipt-last Population Admission Completion identity.
    #[must_use]
    pub(crate) const fn completion_id(&self) -> [u8; 32] {
        self.completion_id
    }

    /// Canonical NIFTY-then-BANKNIFTY ordinal.
    #[must_use]
    pub(crate) const fn global_sequence(&self) -> u64 {
        self.global_sequence
    }

    /// Instrument family at the canonical ordinal.
    #[must_use]
    pub(crate) const fn family(&self) -> AdmissionV3Family {
        self.family
    }

    /// Family-local Candidate ordinal.
    #[must_use]
    pub(crate) const fn family_sequence(&self) -> u64 {
        self.family_sequence
    }

    /// Runner Admission outcome.
    #[must_use]
    pub(crate) const fn status(&self) -> AdmissionV3Status {
        self.status
    }

    /// Candidate universe identity.
    #[must_use]
    pub(crate) const fn candidate_universe_id(&self) -> [u8; 32] {
        self.candidate_universe_id
    }

    /// Candidate Completion content digest.
    #[must_use]
    pub(crate) const fn candidate_completion_digest(&self) -> [u8; 32] {
        self.candidate_completion_digest
    }

    /// Candidate semantic identity.
    #[must_use]
    pub(crate) const fn candidate_semantic_id(&self) -> [u8; 32] {
        self.candidate_semantic_id
    }

    /// Exact Candidate-row digest.
    #[must_use]
    pub(crate) const fn candidate_row_digest(&self) -> [u8; 32] {
        self.candidate_row_digest
    }

    /// Pre-Admission authority identity.
    #[must_use]
    pub(crate) const fn pre_admission_authority_id(&self) -> [u8; 32] {
        self.pre_admission_authority_id
    }

    /// Population Statistics audit identity.
    #[must_use]
    pub(crate) const fn statistics_audit_id(&self) -> [u8; 32] {
        self.statistics_audit_id
    }

    /// Population Statistics Completion digest.
    #[must_use]
    pub(crate) const fn statistics_completion_digest(&self) -> [u8; 32] {
        self.statistics_completion_digest
    }

    /// Candidate-specific ordered-period digest.
    #[must_use]
    pub(crate) const fn statistics_period_digest(&self) -> [u8; 32] {
        self.statistics_period_digest
    }

    /// Candidate-specific ordered-split digest.
    #[must_use]
    pub(crate) const fn statistics_split_digest(&self) -> [u8; 32] {
        self.statistics_split_digest
    }

    /// Canonical NIFTY/BANKNIFTY Search V4 pair identity.
    #[must_use]
    pub(crate) const fn search_pair_id(&self) -> [u8; 32] {
        self.search_pair_id
    }

    /// Family Search V4 member identity.
    #[must_use]
    pub(crate) const fn search_member_id(&self) -> [u8; 32] {
        self.search_member_id
    }

    /// Full signal-stream digest bound by Search V4.
    #[must_use]
    pub(crate) const fn search_signal_digest(&self) -> [u8; 32] {
        self.search_signal_digest
    }

    /// Exact signal-bar count bound by Search V4.
    #[must_use]
    pub(crate) const fn search_signal_bars(&self) -> u64 {
        self.search_signal_bars
    }

    /// First signal timestamp bound by Search V4.
    #[must_use]
    pub(crate) const fn search_signal_first_ts_micros(&self) -> i64 {
        self.search_signal_first_ts_micros
    }

    /// Last signal timestamp bound by Search V4.
    #[must_use]
    pub(crate) const fn search_signal_last_ts_micros(&self) -> i64 {
        self.search_signal_last_ts_micros
    }

    /// Full prepared signal-column identity bound by Search V4.
    #[must_use]
    pub(crate) const fn search_signal_column_digest(&self) -> [u8; 32] {
        self.search_signal_column_digest
    }

    /// Exact Search V4 validation-policy identity.
    #[must_use]
    pub(crate) const fn search_policy_id(&self) -> [u8; 32] {
        self.search_policy_id
    }

    /// Full Long-plus-Short Search grid identity.
    #[must_use]
    pub(crate) const fn search_full_grid_id(&self) -> [u8; 32] {
        self.search_full_grid_id
    }

    /// Long exit-policy identity.
    #[must_use]
    pub(crate) const fn search_long_policy_id(&self) -> [u8; 32] {
        self.search_long_policy_id
    }

    /// Long resolved-grid identity.
    #[must_use]
    pub(crate) const fn search_long_resolution_id(&self) -> [u8; 32] {
        self.search_long_resolution_id
    }

    /// Short exit-policy identity.
    #[must_use]
    pub(crate) const fn search_short_policy_id(&self) -> [u8; 32] {
        self.search_short_policy_id
    }

    /// Short resolved-grid identity.
    #[must_use]
    pub(crate) const fn search_short_resolution_id(&self) -> [u8; 32] {
        self.search_short_resolution_id
    }

    /// Search V4 validation-family identity.
    #[must_use]
    pub(crate) const fn search_family_id(&self) -> [u8; 32] {
        self.search_family_id
    }

    /// Search V4 walk identity.
    #[must_use]
    pub(crate) const fn search_walk_id(&self) -> [u8; 32] {
        self.search_walk_id
    }

    /// Search V4 fold count.
    #[must_use]
    pub(crate) const fn search_fold_count(&self) -> u64 {
        self.search_fold_count
    }

    /// Search V4 decided-fold count.
    #[must_use]
    pub(crate) const fn search_decided_folds(&self) -> u64 {
        self.search_decided_folds
    }

    /// Search V4 profitable OOS-fold count.
    #[must_use]
    pub(crate) const fn search_profitable_oos_folds(&self) -> u64 {
        self.search_profitable_oos_folds
    }

    /// Search V4 aggregate fold OOS return in paisa.
    #[must_use]
    pub(crate) const fn search_aggregate_oos_paisa(&self) -> i64 {
        self.search_aggregate_oos_paisa
    }

    /// Number of population cells evaluated by exact-grid Search V4.
    #[must_use]
    pub(crate) const fn search_evaluated_population_count(&self) -> u64 {
        self.search_evaluated_population_count
    }

    /// Paired Base-Evidence identity.
    #[must_use]
    pub(crate) const fn paired_base_id(&self) -> [u8; 32] {
        self.paired_base_id
    }

    /// Family Base-Evidence Completion identity.
    #[must_use]
    pub(crate) const fn base_completion_id(&self) -> [u8; 32] {
        self.base_completion_id
    }

    /// Candidate-specific Base-Evidence identity.
    #[must_use]
    pub(crate) const fn base_evidence_id(&self) -> [u8; 32] {
        self.base_evidence_id
    }

    /// Digest of the immutable Runner Admission V3 decision bytes.
    #[must_use]
    pub(crate) const fn runner_decision_digest(&self) -> [u8; 32] {
        self.runner_decision_digest
    }

    /// Digest of the nested Runner Admission V3 evidence bytes.
    #[must_use]
    pub(crate) const fn runner_evidence_digest(&self) -> [u8; 32] {
        self.runner_evidence_digest
    }

    /// Digest of the nested Runner Admission V3 verdict bytes.
    #[must_use]
    pub(crate) const fn runner_verdict_digest(&self) -> [u8; 32] {
        self.runner_verdict_digest
    }

    /// Candidate-specific Population Admission V3 decision identity.
    #[must_use]
    pub(crate) const fn decision_id(&self) -> [u8; 32] {
        self.decision_id
    }
}

/// Source-independent integrity projection of one embedded Admission V3 row.
///
/// This proves only the literal outer seal and canonical codec, internal
/// decision identities/domain hashes, and Runner's detached V3 arithmetic with
/// exact decision/evidence/verdict bytes and status.  It does **not** prove that
/// the record came from an Admission ledger generation, nor that its Candidate,
/// Statistics or Search identities belong to any retained source.  A V5 reopen
/// must join those identities to its separately authenticated authorities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PopulationAdmissionV3EmbeddedProjection {
    canonical_record: [u8; POPULATION_ADMISSION_V3_DECISION_BYTES],
    decision: AdmissionDecisionRecordV3,
    comparison_values: AdmissionEvidenceValuesV1,
    verdict: AdmissionVerdictV1,
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the embedded integrity getters are consumed by the pending Population V5 reopen join"
    )
)]
impl PopulationAdmissionV3EmbeddedProjection {
    fn from_authenticated(
        authenticated: AuthenticatedAdmissionDecisionRecordV3,
    ) -> Result<Self, PopulationAdmissionV3Refusal> {
        let AuthenticatedAdmissionDecisionRecordV3 {
            decision,
            canonical_record,
        } = authenticated;
        if encode_decision(&decision)? != canonical_record {
            return Err(
                "embedded Admission V3 decision differs from literal sealed outer record"
                    .to_owned(),
            );
        }
        let arithmetic = AdmissionV3ArithmeticProjection::verify_decision_record_detached(
            &decision.runner_decision,
        )
        .map_err(|why| format!("embedded Admission V3 Runner arithmetic refused: {why:?}"))?;
        let reconstructed_decision = arithmetic.decision_bytes();
        let reconstructed_evidence = arithmetic.evidence_bytes();
        let reconstructed_verdict = arithmetic.verdict_bytes();
        let stored_evidence =
            &decision.runner_decision[RUNNER_EVIDENCE_OFFSET..RUNNER_VERDICT_OFFSET];
        let stored_verdict = &decision.runner_decision[RUNNER_VERDICT_OFFSET..];
        if reconstructed_decision != decision.runner_decision
            || reconstructed_evidence.as_slice() != stored_evidence
            || reconstructed_verdict.as_slice() != stored_verdict
        {
            return Err(
                "embedded Admission V3 Runner decision/evidence/verdict bytes differ after arithmetic verification"
                    .to_owned(),
            );
        }
        if hash_slices(RUNNER_DECISION_DIGEST_DOMAIN, &[&reconstructed_decision])
            != decision.runner_decision_digest
            || hash_slices(RUNNER_EVIDENCE_DIGEST_DOMAIN, &[&reconstructed_evidence])
                != decision.evidence_digest
            || hash_slices(RUNNER_VERDICT_DIGEST_DOMAIN, &[&reconstructed_verdict])
                != decision.verdict_digest
        {
            return Err(
                "embedded Admission V3 Runner decision/evidence/verdict digest mismatch".to_owned(),
            );
        }
        if AdmissionV3Status::from_runner(arithmetic.status()) != decision.status {
            return Err(
                "embedded Admission V3 recomputed status differs from outer decision".to_owned(),
            );
        }
        let verdict = arithmetic.verdict();
        if verdict.status() != arithmetic.status()
            || verdict.canonical_bytes() != reconstructed_verdict
        {
            return Err(
                "embedded Admission V3 typed verdict differs from Runner arithmetic".to_owned(),
            );
        }
        let comparison_values = arithmetic.comparison_values();
        Ok(Self {
            canonical_record,
            decision,
            comparison_values,
            verdict,
        })
    }

    /// Exact literal sealed outer record verified by this projection.
    #[must_use]
    pub(crate) const fn canonical_record(&self) -> &[u8; POPULATION_ADMISSION_V3_DECISION_BYTES] {
        &self.canonical_record
    }

    #[must_use]
    pub(crate) const fn block_id(&self) -> [u8; 32] {
        self.decision.block_id
    }

    #[must_use]
    pub(crate) const fn global_sequence(&self) -> u64 {
        self.decision.global_sequence
    }

    #[must_use]
    pub(crate) const fn family(&self) -> AdmissionV3Family {
        self.decision.family
    }

    #[must_use]
    pub(crate) const fn family_sequence(&self) -> u64 {
        self.decision.family_sequence
    }

    #[must_use]
    pub(crate) const fn status(&self) -> AdmissionV3Status {
        self.decision.status
    }

    #[must_use]
    pub(crate) const fn candidate_semantic_id(&self) -> [u8; 32] {
        self.decision.candidate_semantic_id
    }

    #[must_use]
    pub(crate) const fn candidate_row_digest(&self) -> [u8; 32] {
        self.decision.candidate_row_digest
    }

    #[must_use]
    pub(crate) const fn pre_admission_authority_id(&self) -> [u8; 32] {
        self.decision.pre_admission_authority_id
    }

    #[must_use]
    pub(crate) const fn statistics_period_digest(&self) -> [u8; 32] {
        self.decision.statistics_period_digest
    }

    #[must_use]
    pub(crate) const fn statistics_split_digest(&self) -> [u8; 32] {
        self.decision.statistics_split_digest
    }

    #[must_use]
    pub(crate) const fn family_search_member_id(&self) -> [u8; 32] {
        self.decision.family_search_member_id
    }

    #[must_use]
    pub(crate) const fn base_evidence_id(&self) -> [u8; 32] {
        self.decision.base_evidence_id
    }

    #[must_use]
    pub(crate) const fn runner_decision(&self) -> &[u8; RUNNER_ADMISSION_V3_DECISION_BYTES] {
        &self.decision.runner_decision
    }

    #[must_use]
    pub(crate) const fn runner_decision_digest(&self) -> [u8; 32] {
        self.decision.runner_decision_digest
    }

    #[must_use]
    pub(crate) const fn runner_evidence_digest(&self) -> [u8; 32] {
        self.decision.evidence_digest
    }

    #[must_use]
    pub(crate) const fn runner_verdict_digest(&self) -> [u8; 32] {
        self.decision.verdict_digest
    }

    #[must_use]
    pub(crate) const fn decision_id(&self) -> [u8; 32] {
        self.decision.decision_id
    }

    #[must_use]
    pub(crate) const fn comparison_values(&self) -> AdmissionEvidenceValuesV1 {
        self.comparison_values
    }

    #[must_use]
    pub(crate) const fn verdict(&self) -> AdmissionVerdictV1 {
        self.verdict
    }
}

/// Verifies one literal Admission V3 record embedded in a successor format.
///
/// This is a byte/internal-arithmetic verifier only.  It has no ledger handle,
/// performs no file-generation check, and cannot establish Candidate,
/// Statistics or Search provenance.  Its returned identities must be joined to
/// separately authenticated authorities by the caller.
///
/// # Errors
///
/// Refuses any outer seal/codec/identity mismatch, noncanonical re-encoding,
/// nested Runner byte/digest/status mismatch, or invalid detached arithmetic.
pub(crate) fn verify_population_v5_canonical_record(
    raw: &[u8; POPULATION_ADMISSION_V3_DECISION_BYTES],
) -> Result<PopulationAdmissionV3EmbeddedProjection, PopulationAdmissionV3Refusal> {
    let authenticated = AuthenticatedAdmissionDecisionRecordV3::from_canonical_record(raw)?;
    PopulationAdmissionV3EmbeddedProjection::from_authenticated(authenticated)
}

/// Returns one admitted canonical Admission V3 record for sibling unit tests.
///
/// This helper is not compiled into production.  It is generated by this
/// module's owning codec and is required to pass the same embedded verifier
/// used by Population V5 reopen before leaving the module.
#[cfg(test)]
pub(crate) fn population_v5_test_canonical_admission_record_for(
    candidate_semantic_id: [u8; 32],
    candidate_row_digest: [u8; 32],
    family: AdmissionV3Family,
    global_sequence: u64,
    family_sequence: u64,
) -> [u8; POPULATION_ADMISSION_V3_DECISION_BYTES] {
    let raw = tests::canonical_admission_record_for(
        candidate_semantic_id,
        candidate_row_digest,
        family,
        global_sequence,
        family_sequence,
    );
    let verified = verify_population_v5_canonical_record(&raw);
    assert!(
        verified.is_ok(),
        "Admission V3 test fixture must pass embedded verification: {verified:?}"
    );
    raw
}

/// Test-only source identity and decision coordinates for one Admission V3 fixture.
#[cfg(test)]
#[derive(Clone, Copy)]
pub(crate) struct PopulationV5AdmissionFixtureRequest {
    /// Candidate-universe identities in NIFTY, BANKNIFTY order.
    pub(crate) candidate_universe_ids: [[u8; 32]; 2],
    /// Candidate-completion identities in NIFTY, BANKNIFTY order.
    pub(crate) candidate_completion_digests: [[u8; 32]; 2],
    /// Candidate semantic identity bound into the decision.
    pub(crate) candidate_semantic_id: [u8; 32],
    /// Exact candidate-row digest bound into the decision.
    pub(crate) candidate_row_digest: [u8; 32],
    /// Instrument family that owns the decision.
    pub(crate) family: AdmissionV3Family,
    /// Global decision sequence within the block.
    pub(crate) global_sequence: u64,
    /// Family-local decision sequence within the block.
    pub(crate) family_sequence: u64,
    /// Terminal Admission V3 status to encode.
    pub(crate) status: AdmissionV3Status,
}

/// Returns exact outer bytes and source-authenticated identity for a
/// parameterized two-family Admission V3 fixture used by sibling successors.
///
/// This is absent from production. It exists so a successor unit test can
/// prove every repeated source field is load-bearing without inventing
/// getters on the detached embedded verifier for preimages that the literal
/// 2,048-byte decision does not contain.
#[cfg(test)]
pub(crate) fn population_v5_test_admission_fixture_for(
    request: PopulationV5AdmissionFixtureRequest,
) -> (
    [u8; POPULATION_ADMISSION_V3_DECISION_BYTES],
    PopulationAdmissionV3DecisionProjection,
) {
    tests::admission_fixture_for(request)
}

/// Arithmetic-reverified Admission V3 projection for a Population successor.
///
/// This is deliberately not [`runner::admission::AdmissionEvidenceV1`] and not
/// a legacy Admission decision. It retains the source-authenticated identity
/// projection beside a source-independent exact embedded-byte projection. Its
/// only constructor consumes a ledger-authenticated decoded-plus-raw bundle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PopulationAdmissionV3SuccessorProjection {
    identity: PopulationAdmissionV3DecisionProjection,
    embedded: PopulationAdmissionV3EmbeddedProjection,
}

impl PopulationAdmissionV3SuccessorProjection {
    fn from_authenticated(
        receipt: PopulationAdmissionV3StructuralReceipt,
        source: &BlockSourceV3,
        authenticated: AuthenticatedAdmissionDecisionRecordV3,
    ) -> Result<Self, PopulationAdmissionV3Refusal> {
        let identity = PopulationAdmissionV3DecisionProjection::from_authenticated(
            receipt,
            source,
            &authenticated.decision,
        )?;
        let embedded = PopulationAdmissionV3EmbeddedProjection::from_authenticated(authenticated)?;
        Ok(Self { identity, embedded })
    }

    /// Complete durable identity projection authenticated for this row.
    #[must_use]
    pub(crate) const fn identity(&self) -> PopulationAdmissionV3DecisionProjection {
        self.identity
    }

    /// Literal sealed Admission V3 outer record authenticated by the ledger.
    #[must_use]
    pub(crate) const fn canonical_record(&self) -> &[u8; POPULATION_ADMISSION_V3_DECISION_BYTES] {
        self.embedded.canonical_record()
    }

    /// Exact canonical Runner V3 decision reverified against the durable row.
    #[must_use]
    pub(crate) const fn runner_decision(&self) -> &[u8; RUNNER_ADMISSION_V3_DECISION_BYTES] {
        self.embedded.runner_decision()
    }

    /// Typed V3 comparison values reconstructed by Runner.
    #[must_use]
    pub(crate) const fn comparison_values(&self) -> AdmissionEvidenceValuesV1 {
        self.embedded.comparison_values()
    }

    /// Typed verdict recomputed by Runner from the stored canonical decision.
    #[must_use]
    pub(crate) const fn verdict(&self) -> AdmissionVerdictV1 {
        self.embedded.verdict()
    }
}

/// Nonconstructible durable Admission V3 authority.
///
/// The authority owns the exact source authenticated at commit time and keeps
/// the freshly reopened read ledger alive, so downstream consumers cannot
/// detach an identity receipt from the bytes that established it.
pub(crate) struct PopulationAdmissionV3Authority {
    receipt: PopulationAdmissionV3StructuralReceipt,
    source: BlockSourceV3,
    ledger: PopulationAdmissionV3Ledger,
}

impl PopulationAdmissionV3Authority {
    /// Authenticated receipt of the retained block.
    #[must_use]
    pub(crate) const fn structural_receipt(&self) -> PopulationAdmissionV3StructuralReceipt {
        self.receipt
    }

    /// Reads and authenticates one fixed-offset Candidate projection.
    ///
    /// The caller supplies only the canonical ordinal. The retained ledger
    /// rechecks path identity and every bounded file generation before and
    /// after reading exactly one fixed record.
    ///
    /// # Errors
    ///
    /// Refuses an out-of-range ordinal or any removed, replaced, stale,
    /// corrupt, reordered or crosswired retained source.
    pub(crate) fn decision_projection(
        &mut self,
        global_sequence: u64,
    ) -> Result<PopulationAdmissionV3DecisionProjection, PopulationAdmissionV3Refusal> {
        let decision = self
            .ledger
            .authenticated_decision_record(self.receipt, global_sequence)?;
        PopulationAdmissionV3DecisionProjection::from_authenticated(
            self.receipt,
            &self.source,
            &decision.decision,
        )
    }

    /// Reads the complete canonical NIFTY-then-BANKNIFTY projection block.
    ///
    /// Unlike repeated [`Self::decision_projection`] calls, this door holds
    /// one shared lock and validates all bounded file generations exactly once
    /// before and once after the ordered fixed-record read. It accepts neither
    /// ordinals nor caller-authored rows.
    ///
    /// # Errors
    ///
    /// Refuses any removed, replaced, stale, corrupt, reordered, incomplete
    /// or crosswired retained source, or any bounded allocation failure.
    pub(crate) fn ordered_decision_projections(
        &mut self,
    ) -> Result<Vec<PopulationAdmissionV3DecisionProjection>, PopulationAdmissionV3Refusal> {
        let decisions = self
            .ledger
            .authenticated_ordered_decision_records(self.receipt, &self.source)?;
        collect_authenticated_projections(
            self.receipt,
            &self.source,
            decisions,
            |receipt, source, authenticated| {
                PopulationAdmissionV3DecisionProjection::from_authenticated(
                    receipt,
                    source,
                    &authenticated.decision,
                )
            },
            "identity",
        )
    }

    /// Reads and arithmetic-reverifies one Population-successor projection.
    ///
    /// The retained ledger performs the same fixed-offset, pre/post-generation
    /// authenticated read as [`Self::decision_projection`].  Runner then
    /// decodes and exactly reconstructs its nested V3 decision, evidence and
    /// verdict before typed values leave this authority.
    ///
    /// # Errors
    ///
    /// Refuses an invalid ordinal, any stale/corrupt retained ledger, any
    /// canonical Runner arithmetic mismatch, or any outer identity mismatch.
    pub(crate) fn successor_projection(
        &mut self,
        global_sequence: u64,
    ) -> Result<PopulationAdmissionV3SuccessorProjection, PopulationAdmissionV3Refusal> {
        let decision = self
            .ledger
            .authenticated_decision_record(self.receipt, global_sequence)?;
        PopulationAdmissionV3SuccessorProjection::from_authenticated(
            self.receipt,
            &self.source,
            decision,
        )
    }

    /// Reads the bounded ordered block for a Population successor.
    ///
    /// One shared lock and one pre/post generation pair covers the whole
    /// NIFTY-then-BANKNIFTY record range.  Runner arithmetic is subsequently
    /// rerun on the owned bounded rows; no per-row whole-file validation or
    /// caller-authored row enters this path.
    ///
    /// # Errors
    ///
    /// Refuses any stale/corrupt/crosswired source, bounded allocation failure,
    /// invalid Runner arithmetic or byte/status mismatch.
    pub(crate) fn ordered_successor_projections(
        &mut self,
    ) -> Result<Vec<PopulationAdmissionV3SuccessorProjection>, PopulationAdmissionV3Refusal> {
        let decisions = self
            .ledger
            .authenticated_ordered_decision_records(self.receipt, &self.source)?;
        collect_authenticated_projections(
            self.receipt,
            &self.source,
            decisions,
            PopulationAdmissionV3SuccessorProjection::from_authenticated,
            "successor",
        )
    }
}

fn collect_authenticated_projections<T>(
    receipt: PopulationAdmissionV3StructuralReceipt,
    source: &BlockSourceV3,
    decisions: Vec<AuthenticatedAdmissionDecisionRecordV3>,
    mut project: impl FnMut(
        PopulationAdmissionV3StructuralReceipt,
        &BlockSourceV3,
        AuthenticatedAdmissionDecisionRecordV3,
    ) -> Result<T, PopulationAdmissionV3Refusal>,
    kind: &str,
) -> Result<Vec<T>, PopulationAdmissionV3Refusal> {
    let mut projections = Vec::new();
    projections
        .try_reserve_exact(decisions.len())
        .map_err(|why| format!("cannot reserve Admission V3 ordered {kind} projections: {why}"))?;
    for decision in decisions {
        projections.push(project(receipt, source, decision)?);
    }
    Ok(projections)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PopulationAdmissionV3StructuralCommit {
    Written(PopulationAdmissionV3StructuralReceipt),
    Reused(PopulationAdmissionV3StructuralReceipt),
}

impl PopulationAdmissionV3StructuralCommit {
    const fn receipt(self) -> PopulationAdmissionV3StructuralReceipt {
        match self {
            Self::Written(value) | Self::Reused(value) => value,
        }
    }
}

pub(crate) enum PopulationAdmissionV3Commit {
    Written(PopulationAdmissionV3Authority),
    Reused(PopulationAdmissionV3Authority),
}

fn ordered_decision_digest(
    decisions: &[AdmissionDecisionRecordV3],
) -> Result<[u8; 32], PopulationAdmissionV3Refusal> {
    let count = u64::try_from(decisions.len())
        .map_err(|_| "Admission V3 ordered decision count does not fit u64".to_owned())?;
    let mut hasher = Hasher::new();
    hasher.update(ORDERED_DECISIONS_DOMAIN);
    hasher.update(&count.to_le_bytes());
    for decision in decisions {
        hasher.update(&decision.decision_id);
    }
    Ok(hasher.finalize())
}

fn ordered_authenticated_decision_digest(
    decisions: &[AuthenticatedAdmissionDecisionRecordV3],
) -> Result<[u8; 32], PopulationAdmissionV3Refusal> {
    let count = u64::try_from(decisions.len())
        .map_err(|_| "Admission V3 ordered authenticated count does not fit u64".to_owned())?;
    let mut hasher = Hasher::new();
    hasher.update(ORDERED_DECISIONS_DOMAIN);
    hasher.update(&count.to_le_bytes());
    for authenticated in decisions {
        hasher.update(&authenticated.decision.decision_id);
    }
    Ok(hasher.finalize())
}

fn encode_decision(
    decision: &AdmissionDecisionRecordV3,
) -> Result<[u8; POPULATION_ADMISSION_V3_DECISION_BYTES], PopulationAdmissionV3Refusal> {
    decision.validate()?;
    let mut raw = [0_u8; POPULATION_ADMISSION_V3_DECISION_BYTES];
    {
        let payload = raw
            .get_mut(..DECISION_PAYLOAD_BYTES)
            .ok_or_else(|| "Admission V3 decision payload is absent".to_owned())?;
        let mut writer = FixedWriter::new(payload);
        writer.array(&DECISION_MAGIC)?;
        writer.u32(VERSION)?;
        writer.u32(DECISION_DOMAIN)?;
        writer.array(&decision.block_id)?;
        writer.u64(decision.global_sequence)?;
        writer.u8(decision.family as u8)?;
        writer.zeros(7)?;
        writer.u64(decision.family_sequence)?;
        writer.u8(decision.status as u8)?;
        writer.zeros(7)?;
        for value in [
            decision.candidate_semantic_id,
            decision.candidate_row_digest,
            decision.pre_admission_authority_id,
            decision.statistics_period_digest,
            decision.statistics_split_digest,
            decision.family_search_member_id,
            decision.base_evidence_id,
        ] {
            writer.array(&value)?;
        }
        writer.array(&decision.runner_decision)?;
        writer.array(&decision.runner_decision_digest)?;
        writer.array(&decision.evidence_digest)?;
        writer.array(&decision.verdict_digest)?;
        writer.array(&decision.decision_id)?;
        writer.zero_remaining();
    }
    let seal = hash_slices(
        DECISION_SEAL_DOMAIN,
        &[fixed_payload(&raw, DECISION_PAYLOAD_BYTES)?],
    );
    raw.get_mut(DECISION_PAYLOAD_BYTES..)
        .ok_or_else(|| "Admission V3 decision seal slot is absent".to_owned())?
        .copy_from_slice(&seal);
    Ok(raw)
}

fn decode_decision(
    raw: &[u8; POPULATION_ADMISSION_V3_DECISION_BYTES],
) -> Result<AdmissionDecisionRecordV3, PopulationAdmissionV3Refusal> {
    let payload = validate_fixed_payload(
        raw,
        DECISION_MAGIC,
        VERSION,
        DECISION_DOMAIN,
        DECISION_SEAL_DOMAIN,
        "decision",
    )?;
    let mut reader = FixedReader::new(payload);
    if reader.array::<16>()? != DECISION_MAGIC
        || reader.u32()? != VERSION
        || reader.u32()? != DECISION_DOMAIN
    {
        return Err("Admission V3 decision header/version/domain mismatch".to_owned());
    }
    let block_id = reader.array()?;
    let global_sequence = reader.u64()?;
    let family = AdmissionV3Family::decode(reader.u8()?)?;
    reader.require_zeros(7, "Admission V3 decision family reserve")?;
    let family_sequence = reader.u64()?;
    let status = AdmissionV3Status::decode(reader.u8()?)?;
    reader.require_zeros(7, "Admission V3 decision status reserve")?;
    let value = AdmissionDecisionRecordV3 {
        block_id,
        global_sequence,
        family,
        family_sequence,
        status,
        candidate_semantic_id: reader.array()?,
        candidate_row_digest: reader.array()?,
        pre_admission_authority_id: reader.array()?,
        statistics_period_digest: reader.array()?,
        statistics_split_digest: reader.array()?,
        family_search_member_id: reader.array()?,
        base_evidence_id: reader.array()?,
        runner_decision: reader.array()?,
        runner_decision_digest: reader.array()?,
        evidence_digest: reader.array()?,
        verdict_digest: reader.array()?,
        decision_id: reader.array()?,
    };
    reader.require_remaining_zero("Admission V3 decision reserve")?;
    value.validate()?;
    Ok(value)
}

fn encode_completion(
    completion: &AdmissionCompletionRecordV3,
) -> Result<[u8; POPULATION_ADMISSION_V3_COMPLETION_BYTES], PopulationAdmissionV3Refusal> {
    completion.validate()?;
    let mut raw = [0_u8; POPULATION_ADMISSION_V3_COMPLETION_BYTES];
    {
        let payload = raw
            .get_mut(..COMPLETION_PAYLOAD_BYTES)
            .ok_or_else(|| "Admission V3 Completion payload is absent".to_owned())?;
        let mut writer = FixedWriter::new(payload);
        writer.array(&COMPLETION_MAGIC)?;
        writer.u32(VERSION)?;
        writer.u32(COMPLETION_DOMAIN)?;
        writer.u64(completion.block_sequence)?;
        writer.u64(completion.first_decision_record)?;
        completion.source.encode(&mut writer)?;
        writer.u64(completion.admitted_count)?;
        writer.u64(completion.rejected_count)?;
        writer.u64(completion.unmeasured_count)?;
        writer.u64(completion.refused_count)?;
        writer.array(&completion.ordered_decision_digest)?;
        writer.array(&completion.completion_id)?;
        writer.zero_remaining();
    }
    let seal = hash_slices(
        COMPLETION_SEAL_DOMAIN,
        &[fixed_payload(&raw, COMPLETION_PAYLOAD_BYTES)?],
    );
    raw.get_mut(COMPLETION_PAYLOAD_BYTES..)
        .ok_or_else(|| "Admission V3 Completion seal slot is absent".to_owned())?
        .copy_from_slice(&seal);
    Ok(raw)
}

fn decode_completion(
    raw: &[u8; POPULATION_ADMISSION_V3_COMPLETION_BYTES],
) -> Result<AdmissionCompletionRecordV3, PopulationAdmissionV3Refusal> {
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
        return Err("Admission V3 Completion header/version/domain mismatch".to_owned());
    }
    let block_sequence = reader.u64()?;
    let first_decision_record = reader.u64()?;
    let value = AdmissionCompletionRecordV3 {
        block_sequence,
        first_decision_record,
        source: BlockSourceV3::decode(&mut reader)?,
        admitted_count: reader.u64()?,
        rejected_count: reader.u64()?,
        unmeasured_count: reader.u64()?,
        refused_count: reader.u64()?,
        ordered_decision_digest: reader.array()?,
        completion_id: reader.array()?,
    };
    reader.require_remaining_zero("Admission V3 Completion reserve")?;
    value.validate()?;
    Ok(value)
}

struct FixedWriter<'a> {
    bytes: &'a mut [u8],
    offset: usize,
}

impl<'a> FixedWriter<'a> {
    const fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn write(&mut self, value: &[u8]) -> Result<(), PopulationAdmissionV3Refusal> {
        let end = self
            .offset
            .checked_add(value.len())
            .ok_or_else(|| "Admission V3 fixed writer offset overflowed".to_owned())?;
        self.bytes
            .get_mut(self.offset..end)
            .ok_or_else(|| "Admission V3 fixed writer exceeded payload".to_owned())?
            .copy_from_slice(value);
        self.offset = end;
        Ok(())
    }

    fn array<const N: usize>(
        &mut self,
        value: &[u8; N],
    ) -> Result<(), PopulationAdmissionV3Refusal> {
        self.write(value)
    }

    fn u8(&mut self, value: u8) -> Result<(), PopulationAdmissionV3Refusal> {
        self.write(&[value])
    }

    fn u16(&mut self, value: u16) -> Result<(), PopulationAdmissionV3Refusal> {
        self.write(&value.to_le_bytes())
    }

    fn u32(&mut self, value: u32) -> Result<(), PopulationAdmissionV3Refusal> {
        self.write(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), PopulationAdmissionV3Refusal> {
        self.write(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), PopulationAdmissionV3Refusal> {
        self.write(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), PopulationAdmissionV3Refusal> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| "Admission V3 zero reserve offset overflowed".to_owned())?;
        self.bytes
            .get_mut(self.offset..end)
            .ok_or_else(|| "Admission V3 zero reserve exceeded payload".to_owned())?
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

    fn require_full(&self, name: &str) -> Result<(), PopulationAdmissionV3Refusal> {
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

    fn take<const N: usize>(&mut self) -> Result<[u8; N], PopulationAdmissionV3Refusal> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or_else(|| "Admission V3 fixed reader offset overflowed".to_owned())?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| "Admission V3 fixed reader exceeded payload".to_owned())?
            .try_into()
            .map_err(|_| "Admission V3 fixed reader width mismatch".to_owned())?;
        self.offset = end;
        Ok(value)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], PopulationAdmissionV3Refusal> {
        self.take()
    }

    fn u8(&mut self) -> Result<u8, PopulationAdmissionV3Refusal> {
        self.take::<1>()?
            .first()
            .copied()
            .ok_or_else(|| "Admission V3 one-byte field is absent".to_owned())
    }

    fn u16(&mut self) -> Result<u16, PopulationAdmissionV3Refusal> {
        Ok(u16::from_le_bytes(self.take()?))
    }

    fn u32(&mut self) -> Result<u32, PopulationAdmissionV3Refusal> {
        Ok(u32::from_le_bytes(self.take()?))
    }

    fn u64(&mut self) -> Result<u64, PopulationAdmissionV3Refusal> {
        Ok(u64::from_le_bytes(self.take()?))
    }

    fn i64(&mut self) -> Result<i64, PopulationAdmissionV3Refusal> {
        Ok(i64::from_le_bytes(self.take()?))
    }

    fn require_zeros(
        &mut self,
        count: usize,
        name: &str,
    ) -> Result<(), PopulationAdmissionV3Refusal> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| format!("{name} offset overflowed"))?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| format!("{name} is truncated"))?;
        if bytes.iter().any(|byte| *byte != 0) {
            return Err(format!("{name} is nonzero"));
        }
        self.offset = end;
        Ok(())
    }

    fn require_remaining_zero(&self, name: &str) -> Result<(), PopulationAdmissionV3Refusal> {
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

fn require_runner_header(
    bytes: &[u8],
    domain: u8,
    version: u16,
    payload_bytes: u32,
    name: &str,
) -> Result<(), PopulationAdmissionV3Refusal> {
    if array_at::<4>(bytes, 0, name)? != *b"BADM" {
        return Err(format!("Admission V3 {name} magic is invalid"));
    }
    if *slice_at(bytes, 4, 1, name)?
        .first()
        .ok_or_else(|| format!("Admission V3 {name} domain is absent"))?
        != domain
    {
        return Err(format!("Admission V3 {name} domain is invalid"));
    }
    if slice_at(bytes, 5, 1, name)?.iter().any(|byte| *byte != 0) {
        return Err(format!("Admission V3 {name} header reserve is nonzero"));
    }
    if u16_at(bytes, 6, name)? != version || u32_at(bytes, 8, name)? != payload_bytes {
        return Err(format!("Admission V3 {name} version/width is invalid"));
    }
    Ok(())
}

fn slice_at<'a>(
    bytes: &'a [u8],
    offset: usize,
    width: usize,
    name: &str,
) -> Result<&'a [u8], PopulationAdmissionV3Refusal> {
    let end = offset
        .checked_add(width)
        .ok_or_else(|| format!("Admission V3 {name} offset overflowed"))?;
    bytes
        .get(offset..end)
        .ok_or_else(|| format!("Admission V3 {name} is truncated"))
}

fn array_at<const N: usize>(
    bytes: &[u8],
    offset: usize,
    name: &str,
) -> Result<[u8; N], PopulationAdmissionV3Refusal> {
    slice_at(bytes, offset, N, name)?
        .try_into()
        .map_err(|_| format!("Admission V3 {name} width is not {N}"))
}

fn u16_at(bytes: &[u8], offset: usize, name: &str) -> Result<u16, PopulationAdmissionV3Refusal> {
    Ok(u16::from_le_bytes(array_at(bytes, offset, name)?))
}

fn u32_at(bytes: &[u8], offset: usize, name: &str) -> Result<u32, PopulationAdmissionV3Refusal> {
    Ok(u32::from_le_bytes(array_at(bytes, offset, name)?))
}

fn u64_at(bytes: &[u8], offset: usize, name: &str) -> Result<u64, PopulationAdmissionV3Refusal> {
    Ok(u64::from_le_bytes(array_at(bytes, offset, name)?))
}

fn i64_at(bytes: &[u8], offset: usize, name: &str) -> Result<i64, PopulationAdmissionV3Refusal> {
    Ok(i64::from_le_bytes(array_at(bytes, offset, name)?))
}

fn validate_fixed_payload<'a, const N: usize>(
    raw: &'a [u8; N],
    magic: [u8; 16],
    version: u32,
    domain: u32,
    seal_domain: &[u8],
    name: &str,
) -> Result<&'a [u8], PopulationAdmissionV3Refusal> {
    let payload_len = N
        .checked_sub(SEAL_BYTES)
        .ok_or_else(|| format!("Admission V3 {name} width is below the seal width"))?;
    let payload = raw
        .get(..payload_len)
        .ok_or_else(|| format!("Admission V3 {name} payload is absent"))?;
    let seal = raw
        .get(payload_len..)
        .ok_or_else(|| format!("Admission V3 {name} seal is absent"))?;
    if array_at::<16>(payload, 0, name)? != magic
        || u32_at(payload, 16, name)? != version
        || u32_at(payload, 20, name)? != domain
    {
        return Err(format!("Admission V3 {name} header is invalid"));
    }
    let expected = hash_slices(seal_domain, &[payload]);
    if seal != expected {
        return Err(format!("Admission V3 {name} seal is invalid"));
    }
    Ok(payload)
}

fn fixed_payload<const N: usize>(
    raw: &[u8; N],
    payload_len: usize,
) -> Result<&[u8], PopulationAdmissionV3Refusal> {
    raw.get(..payload_len)
        .ok_or_else(|| "Admission V3 fixed payload is absent".to_owned())
}

fn bounded_set<T>(capacity: usize, name: &str) -> Result<HashSet<T>, PopulationAdmissionV3Refusal>
where
    T: Eq + std::hash::Hash,
{
    let mut set = HashSet::new();
    set.try_reserve(capacity)
        .map_err(|why| format!("cannot reserve Admission V3 {name} index: {why}"))?;
    Ok(set)
}

fn hash_slices(domain: &[u8], values: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    for value in values {
        hasher.update(value);
    }
    hasher.finalize()
}

fn require_nonzero(name: &str, value: [u8; 32]) -> Result<(), PopulationAdmissionV3Refusal> {
    if value == [0_u8; 32] {
        return Err(format!("Admission V3 {name} is zero"));
    }
    Ok(())
}

fn checked_sum(values: &[u64], name: &str) -> Result<u64, PopulationAdmissionV3Refusal> {
    values.iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(*value)
            .ok_or_else(|| format!("Admission V3 {name} overflowed"))
    })
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
struct TrailingDecisionBlockV3 {
    first_decision_record: u64,
    block_id: [u8; 32],
    decisions: Vec<AdmissionDecisionRecordV3>,
}

/// Bounded structural view over Population Admission V3 files.
///
/// `open_read` never creates a path and never returns authenticated authority.
/// It verifies every fixed record and builds an average-O(1) receipt index only
/// after an O(file) bounded generation scan.
pub(crate) struct PopulationAdmissionV3Ledger {
    root: PathBuf,
    root_file: File,
    root_identity: PlatformIdentity,
    lock_path: PathBuf,
    lock_file: File,
    lock_generation: FileGeneration,
    decision_path: PathBuf,
    decision_file: File,
    decision_generation: FileGeneration,
    completion_path: PathBuf,
    completion_file: File,
    completion_generation: FileGeneration,
    bounds: AdmissionV3Bounds,
    writable: bool,
    receipts: HashMap<[u8; 32], PopulationAdmissionV3StructuralReceipt>,
    trailing: Option<TrailingDecisionBlockV3>,
    decision_records: u64,
    completion_records: u64,
    #[cfg(test)]
    reuse_barrier_count: u8,
    #[cfg(test)]
    trailing_barrier_order_code: u16,
}

impl PopulationAdmissionV3Ledger {
    /// Opens an existing V3 ledger without creating its root or children.
    ///
    /// # Errors
    ///
    /// Refuses missing, linked, non-regular, over-bound, ragged, corrupt,
    /// reordered, duplicated, crosswired, stale or path-replaced state.
    pub(crate) fn open_read(
        root: &Path,
        bounds: AdmissionV3Bounds,
    ) -> Result<Self, PopulationAdmissionV3Refusal> {
        Self::open(root, bounds, false)
    }

    fn open_write(
        root: &Path,
        bounds: AdmissionV3Bounds,
    ) -> Result<Self, PopulationAdmissionV3Refusal> {
        Self::open(root, bounds, true)
    }

    fn open(
        root: &Path,
        bounds: AdmissionV3Bounds,
        writable: bool,
    ) -> Result<Self, PopulationAdmissionV3Refusal> {
        let (root_path, root_file, root_identity) = open_root_directory(root)?;
        let lock_path = root_path.join(LOCK_FILE);
        let decision_path = root_path.join(DECISION_FILE);
        let completion_path = root_path.join(COMPLETION_FILE);
        let (lock_file, lock_created) = open_child(&lock_path, writable, writable)?;
        if writable {
            lock_file
                .lock()
                .map_err(|why| format!("cannot take exclusive Admission V3 open lock: {why}"))?;
        } else {
            lock_file
                .lock_shared()
                .map_err(|why| format!("cannot take shared Admission V3 open lock: {why}"))?;
        }
        let opened = (|| {
            let (decision_file, decision_created) = open_child(&decision_path, writable, writable)?;
            let (completion_file, completion_created) =
                open_child(&completion_path, writable, writable)?;
            if lock_created || decision_created || completion_created {
                sync_directory(&root_file, &root_path)?;
            }
            if named_root_identity(&root_path)? != root_identity {
                return Err("Admission V3 root changed while child files opened".to_owned());
            }
            let lock_generation = file_generation(&lock_file, &lock_path, LOCK_MAX_BYTES)?;
            let decision_generation =
                file_generation(&decision_file, &decision_path, bounds.max_decision_bytes())?;
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
                    .map_err(|why| format!("cannot clone Admission V3 held lock file: {why}"))?,
                lock_generation,
                decision_path,
                decision_file,
                decision_generation,
                completion_path,
                completion_file,
                completion_generation,
                bounds,
                writable,
                receipts: HashMap::new(),
                trailing: None,
                decision_records: 0,
                completion_records: 0,
                #[cfg(test)]
                reuse_barrier_count: 0,
                #[cfg(test)]
                trailing_barrier_order_code: 0,
            };
            ledger.scan()?;
            Ok(ledger)
        })();
        let released = lock_file
            .unlock()
            .map_err(|why| format!("cannot release Admission V3 open lock: {why}"));
        match (opened, released) {
            (Ok(ledger), Ok(())) => Ok(ledger),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn scan(&mut self) -> Result<(), PopulationAdmissionV3Refusal> {
        let decision_records = checked_record_count(
            self.decision_generation.len,
            POPULATION_ADMISSION_V3_DECISION_BYTES,
            self.bounds.max_decision_records(),
            "decision",
        )?;
        let completion_records = checked_record_count(
            self.completion_generation.len,
            POPULATION_ADMISSION_V3_COMPLETION_BYTES,
            self.bounds.max_completion_records(),
            "Completion",
        )?;
        self.receipts.clear();
        self.receipts
            .try_reserve(
                usize::try_from(completion_records)
                    .map_err(|_| "Admission V3 Completion count does not fit usize".to_owned())?,
            )
            .map_err(|why| format!("cannot reserve Admission V3 receipt index: {why}"))?;
        self.trailing = None;
        let mut covered_decisions = 0_u64;
        let mut completion_index = 0_u64;
        while completion_index < completion_records {
            let completion = decode_completion(&read_completion_at(
                &mut self.completion_file,
                completion_index,
            )?)?;
            if completion.block_sequence != completion_index {
                return Err(format!(
                    "Admission V3 Completion sequence {} is not canonical {completion_index}",
                    completion.block_sequence
                ));
            }
            if completion.first_decision_record != covered_decisions {
                return Err(format!(
                    "Admission V3 Completion {completion_index} starts at {}, not contiguous {covered_decisions}",
                    completion.first_decision_record
                ));
            }
            self.require_block_bound(completion.source.decision_count)?;
            let end = covered_decisions
                .checked_add(completion.source.decision_count)
                .ok_or_else(|| "Admission V3 completed decision range overflowed".to_owned())?;
            if end > decision_records {
                return Err(format!(
                    "Admission V3 Completion {completion_index} is torn: ends at {end}, file has {decision_records} decisions"
                ));
            }
            let decisions = read_decision_range(
                &mut self.decision_file,
                covered_decisions,
                completion.source.decision_count,
                self.bounds,
            )?;
            let receipt = validate_complete_block(&decisions, &completion)?;
            if self.receipts.insert(receipt.block_id, receipt).is_some() {
                return Err(format!(
                    "Admission V3 block {} appears more than once",
                    hex32(receipt.block_id)
                ));
            }
            covered_decisions = end;
            completion_index = completion_index
                .checked_add(1)
                .ok_or_else(|| "Admission V3 Completion scan overflowed".to_owned())?;
        }
        if covered_decisions < decision_records {
            let trailing_count = decision_records
                .checked_sub(covered_decisions)
                .ok_or_else(|| "Admission V3 trailing count underflowed".to_owned())?;
            self.require_block_bound(trailing_count)?;
            let decisions = read_decision_range(
                &mut self.decision_file,
                covered_decisions,
                trailing_count,
                self.bounds,
            )?;
            let block_id = validate_trailing_decisions(&decisions)?;
            if self.receipts.contains_key(&block_id) {
                return Err(format!(
                    "Admission V3 trailing block {} duplicates a completed block",
                    hex32(block_id)
                ));
            }
            self.trailing = Some(TrailingDecisionBlockV3 {
                first_decision_record: covered_decisions,
                block_id,
                decisions,
            });
        }
        self.decision_records = decision_records;
        self.completion_records = completion_records;
        self.require_unchanged()
    }

    /// Revalidates all bounded file bytes, then performs one average-O(1)
    /// structural receipt lookup. Absence is `Ok(None)`.
    ///
    /// # Errors
    ///
    /// Refuses lock, generation, content or path changes since open.
    pub(crate) fn reopen_structural_receipt(
        &self,
        block_id: &[u8; 32],
    ) -> Result<Option<PopulationAdmissionV3StructuralReceipt>, PopulationAdmissionV3Refusal> {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take shared Admission V3 lookup lock: {why}"))?;
        let result = self
            .require_unchanged()
            .map(|()| self.receipts.get(block_id).copied());
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release shared Admission V3 lookup lock: {why}"));
        match (result, released) {
            (Ok(receipt), Ok(())) => Ok(receipt),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn authenticated_decision_record(
        &mut self,
        receipt: PopulationAdmissionV3StructuralReceipt,
        global_sequence: u64,
    ) -> Result<AuthenticatedAdmissionDecisionRecordV3, PopulationAdmissionV3Refusal> {
        if global_sequence >= receipt.decision_count() {
            return Err(format!(
                "Admission V3 decision ordinal {global_sequence} is outside authenticated count {}",
                receipt.decision_count()
            ));
        }
        let (expected_family, expected_family_sequence) =
            if global_sequence < receipt.nifty_decision_count() {
                (AdmissionV3Family::Nifty, global_sequence)
            } else {
                (
                    AdmissionV3Family::BankNifty,
                    global_sequence
                        .checked_sub(receipt.nifty_decision_count())
                        .ok_or_else(|| {
                            "Admission V3 BANKNIFTY authenticated ordinal underflowed".to_owned()
                        })?,
                )
            };
        let physical = receipt
            .first_decision_record()
            .checked_add(global_sequence)
            .ok_or_else(|| "Admission V3 authenticated decision offset overflowed".to_owned())?;
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take shared Admission V3 decision lock: {why}"))?;
        let read_result = (|| {
            self.require_unchanged()?;
            if self.receipts.get(&receipt.block_id()) != Some(&receipt) {
                return Err(
                    "Admission V3 authenticated receipt is no longer indexed exactly".to_owned(),
                );
            }
            let raw = read_fixed_at::<POPULATION_ADMISSION_V3_DECISION_BYTES>(
                &mut self.decision_file,
                physical,
                POPULATION_ADMISSION_V3_DECISION_BYTES,
                "authenticated decision",
            )?;
            let authenticated =
                AuthenticatedAdmissionDecisionRecordV3::from_canonical_record(&raw)?;
            let decision = &authenticated.decision;
            if decision.block_id != receipt.block_id()
                || decision.global_sequence != global_sequence
                || decision.family != expected_family
                || decision.family_sequence != expected_family_sequence
            {
                return Err(
                    "Admission V3 fixed-offset decision differs from authenticated ordering"
                        .to_owned(),
                );
            }
            Ok(authenticated)
        })();
        let post_generation = self.require_unchanged();
        let result = match (read_result, post_generation) {
            (Ok(authenticated), Ok(())) => Ok(authenticated),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        };
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release shared Admission V3 decision lock: {why}"));
        match (result, released) {
            (Ok(decision), Ok(())) => Ok(decision),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the opaque bulk door keeps its single lock, two generation validations, canonical ordering and projection construction in one auditable refusal path"
    )]
    fn authenticated_ordered_decision_records(
        &mut self,
        receipt: PopulationAdmissionV3StructuralReceipt,
        source: &BlockSourceV3,
    ) -> Result<Vec<AuthenticatedAdmissionDecisionRecordV3>, PopulationAdmissionV3Refusal> {
        self.lock_file.lock_shared().map_err(|why| {
            format!("cannot take shared Admission V3 bulk projection lock: {why}")
        })?;
        let read_result = (|| {
            self.require_unchanged()?;
            if self.receipts.get(&receipt.block_id()) != Some(&receipt) {
                return Err(
                    "Admission V3 bulk projection receipt is no longer indexed exactly".to_owned(),
                );
            }
            self.require_block_bound(receipt.decision_count())?;
            let completion = decode_completion(&read_completion_at(
                &mut self.completion_file,
                receipt.block_sequence(),
            )?)?;
            if &completion.source != source
                || completion.block_sequence != receipt.block_sequence()
                || completion.first_decision_record != receipt.first_decision_record()
                || completion.completion_id != receipt.completion_id()
                || completion.source.block_id != receipt.block_id()
                || completion.source.decision_count != receipt.decision_count()
                || completion.source.nifty_decision_count != receipt.nifty_decision_count()
                || completion.source.banknifty_decision_count != receipt.banknifty_decision_count()
                || completion.ordered_decision_digest != receipt.ordered_decision_digest()
            {
                return Err(
                    "Admission V3 bulk projection Completion crosswires retained authority"
                        .to_owned(),
                );
            }
            let decisions = read_authenticated_decision_range(
                &mut self.decision_file,
                receipt.first_decision_record(),
                receipt.decision_count(),
                self.bounds,
            )?;
            if ordered_authenticated_decision_digest(&decisions)?
                != receipt.ordered_decision_digest()
            {
                return Err(
                    "Admission V3 bulk projection ordered decision digest changed".to_owned(),
                );
            }
            let mut admitted_count = 0_u64;
            let mut rejected_count = 0_u64;
            let mut unmeasured_count = 0_u64;
            let mut refused_count = 0_u64;
            for (index, authenticated) in decisions.iter().enumerate() {
                let decision = &authenticated.decision;
                let global_sequence = u64::try_from(index).map_err(|_| {
                    "Admission V3 bulk projection ordinal does not fit u64".to_owned()
                })?;
                let (expected_family, expected_family_sequence) =
                    if global_sequence < receipt.nifty_decision_count() {
                        (AdmissionV3Family::Nifty, global_sequence)
                    } else {
                        (
                            AdmissionV3Family::BankNifty,
                            global_sequence
                                .checked_sub(receipt.nifty_decision_count())
                                .ok_or_else(|| {
                                    "Admission V3 bulk BANKNIFTY ordinal underflowed".to_owned()
                                })?,
                        )
                    };
                if decision.block_id != receipt.block_id()
                    || decision.global_sequence != global_sequence
                    || decision.family != expected_family
                    || decision.family_sequence != expected_family_sequence
                {
                    return Err(format!(
                        "Admission V3 bulk projection decision {index} violates authenticated order"
                    ));
                }
                let target = match decision.status {
                    AdmissionV3Status::Admitted => &mut admitted_count,
                    AdmissionV3Status::Rejected => &mut rejected_count,
                    AdmissionV3Status::Unmeasured => &mut unmeasured_count,
                    AdmissionV3Status::Refused => &mut refused_count,
                };
                *target = target
                    .checked_add(1)
                    .ok_or_else(|| "Admission V3 bulk status count overflowed".to_owned())?;
                decision.validate_against(source)?;
            }
            if admitted_count != completion.admitted_count
                || rejected_count != completion.rejected_count
                || unmeasured_count != completion.unmeasured_count
                || refused_count != completion.refused_count
            {
                return Err(
                    "Admission V3 bulk projection status counts differ from Completion".to_owned(),
                );
            }
            Ok(decisions)
        })();
        let post_generation = self.require_unchanged();
        let result = match (read_result, post_generation) {
            (Ok(decisions), Ok(())) => Ok(decisions),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        };
        let released = self.lock_file.unlock().map_err(|why| {
            format!("cannot release shared Admission V3 bulk projection lock: {why}")
        });
        match (result, released) {
            (Ok(decisions), Ok(())) => Ok(decisions),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn append(
        &mut self,
        prepared: &PreparedPopulationAdmissionV3,
    ) -> Result<PopulationAdmissionV3StructuralCommit, PopulationAdmissionV3Refusal> {
        if !self.writable {
            return Err("Admission V3 ledger was opened read-only".to_owned());
        }
        self.lock_file
            .lock()
            .map_err(|why| format!("cannot take exclusive Admission V3 append lock: {why}"))?;
        let result = self.append_locked(prepared);
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Admission V3 append lock: {why}"));
        match (result, released) {
            (Ok(commit), Ok(())) => Ok(commit),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn append_locked(
        &mut self,
        prepared: &PreparedPopulationAdmissionV3,
    ) -> Result<PopulationAdmissionV3StructuralCommit, PopulationAdmissionV3Refusal> {
        self.require_unchanged()?;
        let count = self.require_prepared_bound(prepared)?;
        prepared.validate()?;
        if let Some(existing) = self.receipts.get(&prepared.source.block_id).copied() {
            return self.reuse_existing(prepared, existing);
        }
        if let Some(trailing) = self.trailing.clone() {
            return self.complete_trailing(prepared, &trailing);
        }
        self.require_append_bound(count, 1)?;
        let first = self.decision_records;
        for decision in &prepared.decisions {
            append_raw(&mut self.decision_file, &encode_decision(decision)?)?;
        }
        self.decision_file
            .sync_data()
            .map_err(|why| format!("cannot sync Admission V3 decisions: {why}"))?;
        self.decision_records = self
            .decision_records
            .checked_add(count)
            .ok_or_else(|| "Admission V3 decision count overflowed after append".to_owned())?;
        self.refresh_decision_generation()?;
        self.require_unchanged()?;
        let completion = prepared.expected_completion(self.completion_records, first)?;
        append_raw(&mut self.completion_file, &encode_completion(&completion)?)?;
        self.completion_file
            .sync_data()
            .map_err(|why| format!("cannot sync Admission V3 Completion: {why}"))?;
        sync_directory(&self.root_file, &self.root)?;
        self.completion_records = self
            .completion_records
            .checked_add(1)
            .ok_or_else(|| "Admission V3 Completion count overflowed after append".to_owned())?;
        self.refresh_completion_generation()?;
        self.require_unchanged()?;
        self.scan()?;
        let receipt = self
            .receipts
            .get(&prepared.source.block_id)
            .copied()
            .ok_or_else(|| "Admission V3 appended block was not indexed".to_owned())?;
        Ok(PopulationAdmissionV3StructuralCommit::Written(receipt))
    }

    fn reuse_existing(
        &mut self,
        prepared: &PreparedPopulationAdmissionV3,
        existing: PopulationAdmissionV3StructuralReceipt,
    ) -> Result<PopulationAdmissionV3StructuralCommit, PopulationAdmissionV3Refusal> {
        self.require_block_bound(existing.decision_count)?;
        let observed = read_decision_range(
            &mut self.decision_file,
            existing.first_decision_record,
            existing.decision_count,
            self.bounds,
        )?;
        if observed != prepared.decisions {
            return Err(format!(
                "Admission V3 block {} exists with different exact decisions",
                hex32(existing.block_id)
            ));
        }
        let observed_completion = decode_completion(&read_completion_at(
            &mut self.completion_file,
            existing.block_sequence,
        )?)?;
        let expected = prepared
            .expected_completion(existing.block_sequence, existing.first_decision_record)?;
        if observed_completion != expected {
            return Err(format!(
                "Admission V3 block {} exists with different exact Completion",
                hex32(existing.block_id)
            ));
        }
        self.require_unchanged()?;
        self.decision_file
            .sync_data()
            .map_err(|why| format!("cannot sync reused Admission V3 decisions: {why}"))?;
        self.completion_file
            .sync_data()
            .map_err(|why| format!("cannot sync reused Admission V3 Completion: {why}"))?;
        sync_directory(&self.root_file, &self.root)?;
        #[cfg(test)]
        {
            self.reuse_barrier_count = 3;
        }
        self.require_unchanged()?;
        Ok(PopulationAdmissionV3StructuralCommit::Reused(existing))
    }

    fn complete_trailing(
        &mut self,
        prepared: &PreparedPopulationAdmissionV3,
        trailing: &TrailingDecisionBlockV3,
    ) -> Result<PopulationAdmissionV3StructuralCommit, PopulationAdmissionV3Refusal> {
        if trailing.block_id != prepared.source.block_id || trailing.decisions != prepared.decisions
        {
            return Err(format!(
                "Admission V3 trailing block {} is not exact retry {}",
                hex32(trailing.block_id),
                hex32(prepared.source.block_id)
            ));
        }
        self.require_append_bound(0, 1)?;
        self.require_unchanged()?;
        self.decision_file
            .sync_data()
            .map_err(|why| format!("cannot sync Admission V3 retry decisions: {why}"))?;
        #[cfg(test)]
        {
            self.trailing_barrier_order_code = 1;
        }
        let completion = prepared
            .expected_completion(self.completion_records, trailing.first_decision_record)?;
        append_raw(&mut self.completion_file, &encode_completion(&completion)?)?;
        self.completion_file
            .sync_data()
            .map_err(|why| format!("cannot sync Admission V3 retry Completion: {why}"))?;
        #[cfg(test)]
        {
            self.trailing_barrier_order_code = 12;
        }
        sync_directory(&self.root_file, &self.root)?;
        #[cfg(test)]
        {
            self.trailing_barrier_order_code = 123;
        }
        self.completion_records = self
            .completion_records
            .checked_add(1)
            .ok_or_else(|| "Admission V3 retry Completion count overflowed".to_owned())?;
        self.refresh_completion_generation()?;
        self.require_unchanged()?;
        self.scan()?;
        let receipt = self
            .receipts
            .get(&prepared.source.block_id)
            .copied()
            .ok_or_else(|| "Admission V3 retried block was not indexed".to_owned())?;
        Ok(PopulationAdmissionV3StructuralCommit::Written(receipt))
    }

    fn require_prepared_bound(
        &self,
        prepared: &PreparedPopulationAdmissionV3,
    ) -> Result<u64, PopulationAdmissionV3Refusal> {
        let count = prepared.decision_count()?;
        self.require_block_bound(count)?;
        Ok(count)
    }

    fn require_block_bound(&self, count: u64) -> Result<(), PopulationAdmissionV3Refusal> {
        if count > self.bounds.max_decisions_per_block() {
            return Err(format!(
                "Admission V3 block has {count} decisions, above per-block maximum {}",
                self.bounds.max_decisions_per_block()
            ));
        }
        let bytes = count
            .checked_mul(POPULATION_ADMISSION_V3_DECISION_BYTES as u64)
            .ok_or_else(|| "Admission V3 block decision bytes overflowed".to_owned())?;
        if bytes > self.bounds.max_decision_bytes() {
            return Err(format!(
                "Admission V3 block requires {bytes} bytes, above maximum {}",
                self.bounds.max_decision_bytes()
            ));
        }
        Ok(())
    }

    fn require_append_bound(
        &self,
        decisions: u64,
        completions: u64,
    ) -> Result<(), PopulationAdmissionV3Refusal> {
        let next_decisions = self
            .decision_records
            .checked_add(decisions)
            .ok_or_else(|| "Admission V3 append decision count overflowed".to_owned())?;
        if next_decisions > self.bounds.max_decision_records() {
            return Err(format!(
                "Admission V3 append reaches {next_decisions} decisions, above maximum {}",
                self.bounds.max_decision_records()
            ));
        }
        let next_decision_bytes = next_decisions
            .checked_mul(POPULATION_ADMISSION_V3_DECISION_BYTES as u64)
            .ok_or_else(|| "Admission V3 append decision bytes overflowed".to_owned())?;
        if next_decision_bytes > self.bounds.max_decision_bytes() {
            return Err(format!(
                "Admission V3 append reaches {next_decision_bytes} bytes, above maximum {}",
                self.bounds.max_decision_bytes()
            ));
        }
        let next_completions = self
            .completion_records
            .checked_add(completions)
            .ok_or_else(|| "Admission V3 append Completion count overflowed".to_owned())?;
        if next_completions > self.bounds.max_completion_records() {
            return Err(format!(
                "Admission V3 append reaches {next_completions} Completions, above maximum {}",
                self.bounds.max_completion_records()
            ));
        }
        let next_completion_bytes = next_completions
            .checked_mul(POPULATION_ADMISSION_V3_COMPLETION_BYTES as u64)
            .ok_or_else(|| "Admission V3 append Completion bytes overflowed".to_owned())?;
        if next_completion_bytes > self.bounds.max_completion_bytes() {
            return Err(format!(
                "Admission V3 append reaches {next_completion_bytes} Completion bytes, above maximum {}",
                self.bounds.max_completion_bytes()
            ));
        }
        Ok(())
    }

    fn refresh_decision_generation(&mut self) -> Result<(), PopulationAdmissionV3Refusal> {
        self.decision_generation = file_generation(
            &self.decision_file,
            &self.decision_path,
            self.bounds.max_decision_bytes(),
        )?;
        Ok(())
    }

    fn refresh_completion_generation(&mut self) -> Result<(), PopulationAdmissionV3Refusal> {
        self.completion_generation = file_generation(
            &self.completion_file,
            &self.completion_path,
            self.bounds.max_completion_bytes(),
        )?;
        Ok(())
    }

    fn require_unchanged(&self) -> Result<(), PopulationAdmissionV3Refusal> {
        let held_root = self
            .root_file
            .metadata()
            .map_err(|why| format!("cannot stat held Admission V3 root: {why}"))?;
        if named_root_identity(&self.root)? != self.root_identity
            || PlatformIdentity::of(&held_root) != self.root_identity
        {
            return Err("Admission V3 root was replaced after open".to_owned());
        }
        let lock = file_generation(&self.lock_file, &self.lock_path, LOCK_MAX_BYTES)?;
        let decisions = file_generation(
            &self.decision_file,
            &self.decision_path,
            self.bounds.max_decision_bytes(),
        )?;
        let completions = file_generation(
            &self.completion_file,
            &self.completion_path,
            self.bounds.max_completion_bytes(),
        )?;
        if lock != self.lock_generation {
            return Err("Admission V3 lock generation changed after open".to_owned());
        }
        if decisions != self.decision_generation {
            return Err("Admission V3 decision generation changed after open".to_owned());
        }
        if completions != self.completion_generation {
            return Err("Admission V3 Completion generation changed after open".to_owned());
        }
        Ok(())
    }

    fn authenticate_structural_receipt(
        &mut self,
        receipt: PopulationAdmissionV3StructuralReceipt,
        prepared: &PreparedPopulationAdmissionV3,
    ) -> Result<(), PopulationAdmissionV3Refusal> {
        let count = self.require_prepared_bound(prepared)?;
        if count != receipt.decision_count()
            || prepared.source.block_id != receipt.block_id()
            || prepared.source.nifty_decision_count != receipt.nifty_decision_count()
            || prepared.source.banknifty_decision_count != receipt.banknifty_decision_count()
            || ordered_decision_digest(&prepared.decisions)? != receipt.ordered_decision_digest()
        {
            return Err(
                "Admission V3 structural receipt differs from opaque preparation".to_owned(),
            );
        }
        self.require_unchanged()?;
        if self.receipts.get(&receipt.block_id()) != Some(&receipt) {
            return Err("Admission V3 receipt is not the indexed reopened receipt".to_owned());
        }
        let decisions = read_decision_range(
            &mut self.decision_file,
            receipt.first_decision_record(),
            receipt.decision_count(),
            self.bounds,
        )?;
        if decisions != prepared.decisions {
            return Err("Admission V3 decisions differ from opaque preparation".to_owned());
        }
        let observed = decode_completion(&read_completion_at(
            &mut self.completion_file,
            receipt.block_sequence(),
        )?)?;
        let expected = prepared
            .expected_completion(receipt.block_sequence(), receipt.first_decision_record())?;
        if observed != expected {
            return Err("Admission V3 Completion differs from opaque preparation".to_owned());
        }
        if observed.completion_id != receipt.completion_id() {
            return Err("Admission V3 receipt Completion identity changed".to_owned());
        }
        self.require_unchanged()
    }
}

fn persist_population_admission_v3(
    root: &Path,
    bounds: AdmissionV3Bounds,
    prepared: &PreparedPopulationAdmissionV3,
) -> Result<PopulationAdmissionV3Commit, PopulationAdmissionV3Refusal> {
    let mut writer = PopulationAdmissionV3Ledger::open_write(root, bounds)?;
    let structural = writer.append(prepared)?;
    let was_written = matches!(
        structural,
        PopulationAdmissionV3StructuralCommit::Written(_)
    );
    let receipt = structural.receipt();
    drop(writer);
    let mut reopened = PopulationAdmissionV3Ledger::open_read(root, bounds)?;
    let observed = reopened
        .reopen_structural_receipt(&receipt.block_id())?
        .ok_or_else(|| "Admission V3 fresh reopen omitted persisted receipt".to_owned())?;
    if observed != receipt {
        return Err("Admission V3 fresh reopen changed structural receipt".to_owned());
    }
    reopened.authenticate_structural_receipt(observed, prepared)?;
    let authority = PopulationAdmissionV3Authority {
        receipt: observed,
        source: prepared.source.clone(),
        ledger: reopened,
    };
    Ok(if was_written {
        PopulationAdmissionV3Commit::Written(authority)
    } else {
        PopulationAdmissionV3Commit::Reused(authority)
    })
}

/// Prepares, persists, syncs, freshly reopens and authenticates one exact-grid
/// Population Admission V3 block.
///
/// The only Search input is the nonconstructible pair already retained inside
/// `committed`. The returned authority owns the freshly reopened ledger; this
/// door never returns a loose receipt, raw record or caller-authored identity.
///
/// # Errors
///
/// Refuses any source join, arithmetic, bound, allocation, append, sync,
/// retry/reuse, fresh-reopen, generation, corruption or authentication error.
pub(crate) fn commit_population_admission_v3_exact_grid(
    root: &Path,
    bounds: AdmissionV3Bounds,
    committed: &mut CommittedStoredObservationStatisticsV2,
    policy: &AdmissionPolicyV1,
) -> Result<PopulationAdmissionV3Commit, PopulationAdmissionV3Refusal> {
    let prepared = prepare_population_admission_v3_exact_grid(committed, policy)?;
    persist_population_admission_v3(root, bounds, &prepared)
}

fn validate_complete_block(
    decisions: &[AdmissionDecisionRecordV3],
    completion: &AdmissionCompletionRecordV3,
) -> Result<PopulationAdmissionV3StructuralReceipt, PopulationAdmissionV3Refusal> {
    let prepared = PreparedPopulationAdmissionV3 {
        source: completion.source.clone(),
        decisions: decisions.to_vec(),
        base_candidate_count: completion.source.decision_count,
    };
    prepared.validate()?;
    let expected = prepared
        .expected_completion(completion.block_sequence, completion.first_decision_record)?;
    if expected != *completion {
        return Err(
            "Admission V3 Completion does not exactly complete ordered decisions".to_owned(),
        );
    }
    Ok(PopulationAdmissionV3StructuralReceipt {
        block_id: completion.source.block_id,
        completion_id: completion.completion_id,
        block_sequence: completion.block_sequence,
        first_decision_record: completion.first_decision_record,
        decision_count: completion.source.decision_count,
        nifty_decision_count: completion.source.nifty_decision_count,
        banknifty_decision_count: completion.source.banknifty_decision_count,
        ordered_decision_digest: completion.ordered_decision_digest,
    })
}

fn validate_trailing_decisions(
    decisions: &[AdmissionDecisionRecordV3],
) -> Result<[u8; 32], PopulationAdmissionV3Refusal> {
    let first = decisions
        .first()
        .ok_or_else(|| "Admission V3 trailing block is empty".to_owned())?;
    let block_id = first.block_id;
    let mut bank_started = false;
    let mut bank_sequence = 0_u64;
    let mut semantics = bounded_set(decisions.len(), "trailing candidate semantic")?;
    let mut identities = bounded_set(decisions.len(), "trailing decision")?;
    for (index, decision) in decisions.iter().enumerate() {
        decision.validate()?;
        let global = u64::try_from(index)
            .map_err(|_| "Admission V3 trailing index does not fit u64".to_owned())?;
        if decision.block_id != block_id || decision.global_sequence != global {
            return Err(format!(
                "Admission V3 trailing decision {index} changes block or global order"
            ));
        }
        match decision.family {
            AdmissionV3Family::Nifty if bank_started => {
                return Err(
                    "Admission V3 trailing block returns to NIFTY after BANKNIFTY".to_owned(),
                );
            }
            AdmissionV3Family::Nifty => {
                if decision.family_sequence != global {
                    return Err(format!(
                        "Admission V3 trailing NIFTY sequence {} is not {global}",
                        decision.family_sequence
                    ));
                }
            }
            AdmissionV3Family::BankNifty => {
                bank_started = true;
                if decision.family_sequence != bank_sequence {
                    return Err(format!(
                        "Admission V3 trailing BANKNIFTY sequence {} is not {bank_sequence}",
                        decision.family_sequence
                    ));
                }
                bank_sequence = bank_sequence.checked_add(1).ok_or_else(|| {
                    "Admission V3 trailing BANKNIFTY sequence overflowed".to_owned()
                })?;
            }
        }
        if !semantics.insert(decision.candidate_semantic_id) {
            return Err("Admission V3 trailing candidate semantic is duplicated".to_owned());
        }
        if !identities.insert(decision.decision_id) {
            return Err("Admission V3 trailing decision identity is duplicated".to_owned());
        }
    }
    Ok(block_id)
}

fn checked_record_count(
    bytes: u64,
    stride: usize,
    max_records: u64,
    name: &str,
) -> Result<u64, PopulationAdmissionV3Refusal> {
    let stride_u64 = u64::try_from(stride)
        .map_err(|_| format!("Admission V3 {name} stride does not fit u64"))?;
    if !bytes.is_multiple_of(stride_u64) {
        return Err(format!(
            "Admission V3 {name} file has ragged length {bytes}, not a multiple of {stride}"
        ));
    }
    let records = bytes / stride_u64;
    if records > max_records {
        return Err(format!(
            "Admission V3 {name} file has {records} records, above maximum {max_records}"
        ));
    }
    Ok(records)
}

fn read_authenticated_decision_range(
    file: &mut File,
    first: u64,
    count: u64,
    bounds: AdmissionV3Bounds,
) -> Result<Vec<AuthenticatedAdmissionDecisionRecordV3>, PopulationAdmissionV3Refusal> {
    if count > bounds.max_decisions_per_block() {
        return Err(format!(
            "Admission V3 bounded authenticated read count {count} exceeds {}",
            bounds.max_decisions_per_block()
        ));
    }
    let end = first
        .checked_add(count)
        .ok_or_else(|| "Admission V3 authenticated decision read range overflowed".to_owned())?;
    if end > bounds.max_decision_records() {
        return Err(format!(
            "Admission V3 authenticated decision read ends at {end}, above maximum {}",
            bounds.max_decision_records()
        ));
    }
    let count_usize = usize::try_from(count)
        .map_err(|_| "Admission V3 authenticated read count does not fit usize".to_owned())?;
    let mut decisions = Vec::new();
    decisions
        .try_reserve_exact(count_usize)
        .map_err(|why| format!("cannot reserve Admission V3 authenticated read: {why}"))?;
    let mut physical = first;
    while physical < end {
        let raw = read_fixed_at::<POPULATION_ADMISSION_V3_DECISION_BYTES>(
            file,
            physical,
            POPULATION_ADMISSION_V3_DECISION_BYTES,
            "authenticated decision",
        )?;
        decisions.push(AuthenticatedAdmissionDecisionRecordV3::from_canonical_record(&raw)?);
        physical = physical
            .checked_add(1)
            .ok_or_else(|| "Admission V3 authenticated read cursor overflowed".to_owned())?;
    }
    Ok(decisions)
}

fn read_decision_range(
    file: &mut File,
    first: u64,
    count: u64,
    bounds: AdmissionV3Bounds,
) -> Result<Vec<AdmissionDecisionRecordV3>, PopulationAdmissionV3Refusal> {
    if count > bounds.max_decisions_per_block() {
        return Err(format!(
            "Admission V3 bounded read count {count} exceeds {}",
            bounds.max_decisions_per_block()
        ));
    }
    let end = first
        .checked_add(count)
        .ok_or_else(|| "Admission V3 decision read range overflowed".to_owned())?;
    if end > bounds.max_decision_records() {
        return Err(format!(
            "Admission V3 decision read ends at {end}, above maximum {}",
            bounds.max_decision_records()
        ));
    }
    let count_usize = usize::try_from(count)
        .map_err(|_| "Admission V3 decision read count does not fit usize".to_owned())?;
    let mut decisions = Vec::new();
    decisions
        .try_reserve_exact(count_usize)
        .map_err(|why| format!("cannot reserve Admission V3 decision read: {why}"))?;
    let mut physical = first;
    while physical < end {
        let raw = read_fixed_at::<POPULATION_ADMISSION_V3_DECISION_BYTES>(
            file,
            physical,
            POPULATION_ADMISSION_V3_DECISION_BYTES,
            "decision",
        )?;
        decisions.push(decode_decision(&raw)?);
        physical = physical
            .checked_add(1)
            .ok_or_else(|| "Admission V3 decision read cursor overflowed".to_owned())?;
    }
    Ok(decisions)
}

fn read_completion_at(
    file: &mut File,
    physical: u64,
) -> Result<[u8; POPULATION_ADMISSION_V3_COMPLETION_BYTES], PopulationAdmissionV3Refusal> {
    read_fixed_at::<POPULATION_ADMISSION_V3_COMPLETION_BYTES>(
        file,
        physical,
        POPULATION_ADMISSION_V3_COMPLETION_BYTES,
        "Completion",
    )
}

fn read_fixed_at<const N: usize>(
    file: &mut File,
    physical: u64,
    stride: usize,
    name: &str,
) -> Result<[u8; N], PopulationAdmissionV3Refusal> {
    let stride_u64 = u64::try_from(stride)
        .map_err(|_| format!("Admission V3 {name} stride does not fit u64"))?;
    let offset = physical
        .checked_mul(stride_u64)
        .ok_or_else(|| format!("Admission V3 {name} offset overflowed"))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|why| format!("cannot seek Admission V3 {name} {physical}: {why}"))?;
    let mut raw = [0_u8; N];
    file.read_exact(&mut raw)
        .map_err(|why| format!("cannot read Admission V3 {name} {physical}: {why}"))?;
    Ok(raw)
}

fn append_raw(file: &mut File, raw: &[u8]) -> Result<(), PopulationAdmissionV3Refusal> {
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(raw))
        .map_err(|why| format!("cannot append Admission V3 fixed record: {why}"))
}

fn open_root_directory(
    root: &Path,
) -> Result<(PathBuf, File, PlatformIdentity), PopulationAdmissionV3Refusal> {
    require_not_symlink(root, false)?;
    let canonical = std::fs::canonicalize(root).map_err(|why| {
        format!(
            "Admission V3 root {} must already exist: {why}",
            root.display()
        )
    })?;
    require_not_symlink(&canonical, false)?;
    let file = File::open(&canonical).map_err(|why| {
        format!(
            "cannot hold Admission V3 root {}: {why}",
            canonical.display()
        )
    })?;
    let metadata = file.metadata().map_err(|why| {
        format!(
            "cannot stat held Admission V3 root {}: {why}",
            canonical.display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "Admission V3 root {} is not a directory",
            canonical.display()
        ));
    }
    Ok((canonical, file, PlatformIdentity::of(&metadata)))
}

fn named_root_identity(root: &Path) -> Result<PlatformIdentity, PopulationAdmissionV3Refusal> {
    require_not_symlink(root, false)?;
    let file = File::open(root).map_err(|why| {
        format!(
            "cannot reopen named Admission V3 root {}: {why}",
            root.display()
        )
    })?;
    let metadata = file.metadata().map_err(|why| {
        format!(
            "cannot stat named Admission V3 root {}: {why}",
            root.display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "named Admission V3 root {} is not a directory",
            root.display()
        ));
    }
    Ok(PlatformIdentity::of(&metadata))
}

fn open_child(
    path: &Path,
    writable: bool,
    create: bool,
) -> Result<(File, bool), PopulationAdmissionV3Refusal> {
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
                    "cannot create Admission V3 file {}: {why}",
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
        .map_err(|why| format!("cannot open Admission V3 file {}: {why}", path.display()))?;
    require_not_symlink(path, false)?;
    require_regular_file(&file, path)?;
    Ok((file, false))
}

fn require_regular_file(file: &File, path: &Path) -> Result<(), PopulationAdmissionV3Refusal> {
    if !file
        .metadata()
        .map_err(|why| format!("cannot stat Admission V3 file {}: {why}", path.display()))?
        .is_file()
    {
        return Err(format!(
            "Admission V3 path {} is not a regular file",
            path.display()
        ));
    }
    Ok(())
}

fn require_not_symlink(
    path: &Path,
    absent_is_allowed: bool,
) -> Result<(), PopulationAdmissionV3Refusal> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(format!(
            "Admission V3 path {} is a symbolic link; no-follow refused it",
            path.display()
        )),
        Ok(_) => Ok(()),
        Err(why) if absent_is_allowed && why.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(why) => Err(format!(
            "cannot inspect Admission V3 path {} without following links: {why}",
            path.display()
        )),
    }
}

fn sync_directory(root_file: &File, root: &Path) -> Result<(), PopulationAdmissionV3Refusal> {
    root_file.sync_all().map_err(|why| {
        format!(
            "cannot durably sync Admission V3 directory entries in {}: {why}",
            root.display()
        )
    })
}

fn file_generation(
    file: &File,
    path: &Path,
    max_bytes: u64,
) -> Result<FileGeneration, PopulationAdmissionV3Refusal> {
    file_generation_with_between_hash_action(file, path, max_bytes, || Ok(()))
}

fn file_generation_with_between_hash_action(
    file: &File,
    path: &Path,
    max_bytes: u64,
    between_hashes: impl FnOnce() -> Result<(), PopulationAdmissionV3Refusal>,
) -> Result<FileGeneration, PopulationAdmissionV3Refusal> {
    let before_metadata = file.metadata().map_err(|why| {
        format!(
            "cannot stat held Admission V3 file {}: {why}",
            path.display()
        )
    })?;
    if !before_metadata.is_file() {
        return Err(format!(
            "held Admission V3 path {} is not a regular file",
            path.display()
        ));
    }
    if before_metadata.len() > max_bytes {
        return Err(format!(
            "Admission V3 file {} has {} bytes, above maximum {max_bytes}",
            path.display(),
            before_metadata.len()
        ));
    }
    let before = MetadataGeneration::of(&before_metadata);
    let identity = before.identity;
    let (named, _) = open_child(path, false, false)?;
    let named_metadata = named.metadata().map_err(|why| {
        format!(
            "cannot stat named Admission V3 file {}: {why}",
            path.display()
        )
    })?;
    if PlatformIdentity::of(&named_metadata) != identity {
        return Err(format!(
            "Admission V3 file {} was path-replaced after open",
            path.display()
        ));
    }
    let digest = hash_held_prefix(file, path, before.len)?;
    between_hashes()?;
    let middle_metadata = file.metadata().map_err(|why| {
        format!(
            "cannot restat held Admission V3 file {} after first hash: {why}",
            path.display()
        )
    })?;
    if MetadataGeneration::of(&middle_metadata) != before
        || named_root_or_file_identity(path)? != identity
    {
        return Err(format!(
            "Admission V3 file {} changed during bounded generation hashing",
            path.display()
        ));
    }
    let confirmation = hash_held_prefix(file, path, before.len)?;
    let after_metadata = file.metadata().map_err(|why| {
        format!(
            "cannot restat held Admission V3 file {} after second hash: {why}",
            path.display()
        )
    })?;
    if MetadataGeneration::of(&after_metadata) != before
        || named_root_or_file_identity(path)? != identity
    {
        return Err(format!(
            "Admission V3 file {} changed during bounded generation hashing",
            path.display()
        ));
    }
    if confirmation != digest {
        return Err(format!(
            "Admission V3 file {} changed between bounded generation hashes",
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

fn named_root_or_file_identity(
    path: &Path,
) -> Result<PlatformIdentity, PopulationAdmissionV3Refusal> {
    require_not_symlink(path, false)?;
    let metadata = std::fs::metadata(path).map_err(|why| {
        format!(
            "cannot stat named Admission V3 path {}: {why}",
            path.display()
        )
    })?;
    Ok(PlatformIdentity::of(&metadata))
}

fn hash_held_prefix(
    file: &File,
    path: &Path,
    length: u64,
) -> Result<[u8; 32], PopulationAdmissionV3Refusal> {
    let mut reader = file
        .try_clone()
        .map_err(|why| format!("cannot clone Admission V3 file {}: {why}", path.display()))?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|why| format!("cannot seek Admission V3 file {}: {why}", path.display()))?;
    hash_exact_prefix(&mut reader, path, length)
}

fn hash_exact_prefix(
    reader: &mut impl std::io::Read,
    path: &Path,
    length: u64,
) -> Result<[u8; 32], PopulationAdmissionV3Refusal> {
    let mut hasher = Hasher::new();
    hasher.update(FILE_GENERATION_DOMAIN);
    hasher.update(&length.to_le_bytes());
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    let mut remaining = length;
    while remaining != 0 {
        let requested = usize::try_from(remaining.min(READ_CHUNK_BYTES as u64))
            .map_err(|_| "Admission V3 bounded hash width does not fit usize".to_owned())?;
        let read = reader
            .read(
                buffer
                    .get_mut(..requested)
                    .ok_or_else(|| "Admission V3 bounded hash range is invalid".to_owned())?,
            )
            .map_err(|why| format!("cannot hash Admission V3 file {}: {why}", path.display()))?;
        if read == 0 {
            return Err(format!(
                "Admission V3 file {} shortened while hashing",
                path.display()
            ));
        }
        hasher.update(
            buffer
                .get(..read)
                .ok_or_else(|| "Admission V3 hash chunk range is invalid".to_owned())?,
        );
        remaining = remaining
            .checked_sub(
                u64::try_from(read)
                    .map_err(|_| "Admission V3 generation read does not fit u64".to_owned())?,
            )
            .ok_or_else(|| "Admission V3 bounded hash count underflowed".to_owned())?;
    }
    Ok(hasher.finalize())
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "focused adversarial tests intentionally fail fixture setup and mutate exact fixed offsets"
)]
mod tests {
    use super::*;
    use runner::admission::{AdmissionPolicyDraftV1, AdmissionReasonV1, ObservedU64V1, ReasonBits};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_ROOT: AtomicU64 = AtomicU64::new(0);
    const MAX_MAE_EVIDENCE_OFFSET: usize = RUNNER_HEADER_BYTES + 3 * 9;
    const OUTER_STATUS_OFFSET: usize = 80;
    const OUTER_RUNNER_OFFSET: usize = 312;
    const OUTER_RUNNER_DIGEST_OFFSET: usize =
        OUTER_RUNNER_OFFSET + RUNNER_ADMISSION_V3_DECISION_BYTES;
    const OUTER_EVIDENCE_DIGEST_OFFSET: usize = OUTER_RUNNER_DIGEST_OFFSET + 32;
    const OUTER_VERDICT_DIGEST_OFFSET: usize = OUTER_EVIDENCE_DIGEST_OFFSET + 32;

    struct TestRoot {
        path: PathBuf,
    }

    impl TestRoot {
        fn new(label: &str) -> Self {
            let sequence = NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "brutex-population-admission-v3-{}-{label}-{sequence}",
                std::process::id()
            ));
            std::fs::create_dir(&path).expect("create Admission V3 test root");
            Self { path }
        }

        fn absent(label: &str) -> Self {
            let sequence = NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed);
            Self {
                path: std::env::temp_dir().join(format!(
                    "brutex-population-admission-v3-absent-{}-{label}-{sequence}",
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
            if self.path.exists() {
                std::fs::remove_dir_all(&self.path).expect("remove Admission V3 test root");
            }
        }
    }

    fn digest(seed: u8) -> [u8; 32] {
        hash_slices(b"admission-v3-test-digest\0", &[&[seed]])
    }

    fn policy() -> AdmissionPolicyV1 {
        AdmissionPolicyV1::new(AdmissionPolicyDraftV1 {
            min_support_hits: Some(100),
            min_independent_sessions: Some(20),
            min_trades: Some(30),
            max_mae_paisa: Some(500),
            min_worst_reward_risk_ppm: Some(2_000_000),
            min_win_rate_ppm: Some(600_000),
            min_wilson_win_rate_ppm: Some(550_000),
            min_return_drawdown_ppm: Some(3_000_000),
            min_weakest_period_return_paisa: Some(0),
            max_pbo_ppm: Some(100_000),
            max_fwer_p_value_ppm: Some(50_000),
            max_spa_p_value_ppm: Some(50_000),
            min_decided_folds: Some(5),
            max_ambiguous_fill_rate_ppm: Some(100_000),
            max_gap_affected_rate_ppm: Some(100_000),
            max_session_concentration_ppm: Some(300_000),
            max_largest_trade_profit_share_ppm: Some(200_000),
            max_drawdown_paisa: Some(10_000),
            max_worst_trade_loss_paisa: Some(2_000),
            max_losing_trade_rate_ppm: Some(400_000),
            max_losing_trades: Some(20),
            min_pessimistic_profit_paisa: Some(1_000),
            min_winning_trades: Some(30),
            min_average_win_paisa: Some(300),
            max_average_loss_paisa: Some(150),
            min_profit_factor_ppm: Some(2_000_000),
            max_consecutive_losing_streak: Some(3),
            min_consecutive_winning_streak: Some(3),
            min_bootstrap_draws: Some(200),
            min_bootstrap_strategies: Some(4),
            min_bootstrap_periods: Some(40),
            min_pbo_contributing_folds: Some(5),
            max_pbo_unrankable_folds: Some(3),
            min_profitable_oos_folds: Some(3),
            min_oos_pessimistic_return_paisa: Some(1),
            max_white_reality_p_value_ppm: Some(50_000),
            require_white_reality_rejection: Some(true),
            max_romano_wolf_p_value_ppm: Some(50_000),
            require_romano_wolf_rejection: Some(true),
        })
        .expect("fixture policy is complete")
    }

    fn exit_grid(seed: u8) -> ExitGridSourceV3 {
        ExitGridSourceV3::from_candidate(LongShortExitGridIdentitiesV2 {
            long: crate::population::SideExitGridIdentityV2 {
                policy_digest: digest(seed),
                resolved_digest: digest(seed.wrapping_add(1)),
            },
            short: crate::population::SideExitGridIdentityV2 {
                policy_digest: digest(seed.wrapping_add(2)),
                resolved_digest: digest(seed.wrapping_add(3)),
            },
        })
        .expect("fixture exit grid validates")
    }

    fn signal(seed: u8) -> SignalSourceV3 {
        SignalSourceV3 {
            digest: digest(seed),
            bars: 10_000,
            first_ts_micros: 1_704_067_200_000_000,
            last_ts_micros: 1_735_603_140_000_000,
            column_digest: digest(seed.wrapping_add(1)),
        }
    }

    fn family(
        seed: u8,
        signal: SignalSourceV3,
        grid: ExitGridSourceV3,
        count: u64,
    ) -> FamilySourceV3 {
        FamilySourceV3 {
            candidate_universe_id: digest(seed),
            candidate_completion_digest: digest(seed.wrapping_add(1)),
            pre_admission_authority_id: digest(seed.wrapping_add(2)),
            signal,
            exit_grid: grid,
            candidate_count: count,
        }
    }

    fn observation(count: u64) -> ObservationSourceV3 {
        ObservationSourceV3 {
            authority_id: digest(20),
            completion_digest: digest(21),
            pair_id: digest(22),
            source_id: digest(23),
            observation_policy_id: digest(24),
            layout_policy_id: digest(25),
            candidate_count: count,
            period_count: 40,
        }
    }

    fn statistics(count: u64) -> StatisticsSourceV3 {
        StatisticsSourceV3 {
            audit_id: digest(30),
            completion_digest: digest(31),
            observation_authority_id: digest(20),
            observation_completion_digest: digest(21),
            observation_pair_id: digest(22),
            observation_statistics_link_id: digest(32),
            observation_projection_policy_id: digest(33),
            wilson_policy_id: digest(34),
            cscv_policy_id: digest(35),
            ordered_candidate_id: digest(36),
            ordered_period_id: digest(37),
            ordered_split_id: digest(38),
            white_family_id: digest(39),
            spa_family_id: digest(40),
            romano_wolf_family_id: digest(41),
            candidate_count: count,
            period_count: 40,
            split_count: 8,
            draw_count: 200,
            bootstrap_seed: 991,
            bootstrap_block_length: 5,
        }
    }

    fn search_member(
        seed: u8,
        signal: SignalSourceV3,
        grid: ExitGridSourceV3,
        fold_count: u64,
        profitable: u64,
        evaluated_population_count: u64,
    ) -> SearchMemberV4 {
        let mut member = SearchMemberV4 {
            member_id: [0; 32],
            validation_policy_id: digest(seed),
            signal,
            full_grid_id: digest(seed.wrapping_add(1)),
            long_policy_id: grid.long_policy,
            long_resolution_id: grid.long_resolution,
            short_policy_id: grid.short_policy,
            short_resolution_id: grid.short_resolution,
            validation_family_id: digest(seed.wrapping_add(2)),
            walk_id: digest(seed.wrapping_add(3)),
            fold_count,
            decided_folds: fold_count - 2,
            profitable_oos_folds: profitable,
            aggregate_oos_paisa: 125,
            evaluated_population_count,
        };
        member.member_id = member.derive_member_id();
        member
    }

    fn source() -> BlockSourceV3 {
        let policy = policy().canonical_bytes();
        let nifty_signal = signal(50);
        let banknifty_signal = signal(60);
        let nifty_grid = exit_grid(70);
        let banknifty_grid = exit_grid(80);
        let nifty_search = search_member(90, nifty_signal, nifty_grid, 8, 4, 2_048);
        let banknifty_search = search_member(100, banknifty_signal, banknifty_grid, 16, 3, 8_192);
        let mut search = SearchSourceV4 {
            pair_id: [0; 32],
            nifty: nifty_search,
            banknifty: banknifty_search,
        };
        search.pair_id = search.derive_pair_id();
        let mut value = BlockSourceV3 {
            signal_rung: 3,
            horizon_bars: 15,
            requested_span: RequestedSpanV3 {
                from_year: 2024,
                from_month: 1,
                to_year: 2025,
                to_month: 12,
            },
            feed_id: digest(1),
            source_commit_id: digest(2),
            calendar_policy_id: digest(3),
            daily_policy_id: digest(4),
            vocabulary_id: digest(5),
            evaluation_policy_id: digest(6),
            nifty: family(10, nifty_signal, nifty_grid, 2),
            banknifty: family(13, banknifty_signal, banknifty_grid, 2),
            observation: observation(4),
            statistics: statistics(4),
            search,
            base: BaseSourceV3 {
                paired_base: digest(64),
                nifty_completion: digest(65),
                banknifty_completion: digest(66),
            },
            policy,
            policy_digest: hash_slices(POLICY_DIGEST_DOMAIN, &[&policy]),
            nifty_decision_count: 2,
            banknifty_decision_count: 2,
            decision_count: 4,
            block_id: [0; 32],
        };
        value.block_id = value.derive_block_id().expect("derive fixture block");
        value
    }

    fn rekey_search_and_block(source: &mut BlockSourceV3) {
        source.search.nifty.member_id = source.search.nifty.derive_member_id();
        source.search.banknifty.member_id = source.search.banknifty.derive_member_id();
        source.search.pair_id = source.search.derive_pair_id();
        source.block_id = source.derive_block_id().expect("rekey fixture block");
    }

    fn put(bytes: &mut [u8], offset: usize, value: &[u8]) {
        let end = offset + value.len();
        bytes[offset..end].copy_from_slice(value);
    }

    fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
        put(bytes, offset, &value.to_le_bytes());
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        put(bytes, offset, &value.to_le_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        put(bytes, offset, &value.to_le_bytes());
    }

    fn put_i64(bytes: &mut [u8], offset: usize, value: i64) {
        put(bytes, offset, &value.to_le_bytes());
    }

    fn put_measured_u64(bytes: &mut [u8], offset: &mut usize, value: u64) {
        bytes[*offset] = 0;
        put_u64(bytes, *offset + 1, value);
        *offset += 9;
    }

    fn put_measured_i64(bytes: &mut [u8], offset: &mut usize, value: i64) {
        bytes[*offset] = 0;
        put_i64(bytes, *offset + 1, value);
        *offset += 9;
    }

    fn put_observed_u64_at(bytes: &mut [u8], offset: usize, value: ObservedU64V1) {
        match value {
            ObservedU64V1::Measured(measured) => {
                bytes[offset] = 0;
                put_u64(bytes, offset + 1, measured);
            }
            ObservedU64V1::Unmeasured => {
                bytes[offset] = 1;
                put_u64(bytes, offset + 1, 0);
            }
            ObservedU64V1::Refused => {
                bytes[offset] = 2;
                put_u64(bytes, offset + 1, 0);
            }
        }
    }

    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::float_arithmetic,
        reason = "the fixture must reproduce Runner's documented bit-exact Wilson calculation"
    )]
    fn fixture_wilson(wins: u64, trades: u64) -> (u64, u64) {
        let value = if trades == 0 {
            0.0
        } else {
            const Z: f64 = 1.959_964;
            let n = trades as f64;
            let p = wins.min(trades) as f64 / n;
            let z2 = Z * Z;
            let denominator = 1.0 + z2 / n;
            let centre = p + z2 / (2.0 * n);
            let margin = Z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt();
            ((centre - margin) / denominator).clamp(0.0, 1.0)
        };
        let ppm = (value * 1_000_000_f64).clamp(0.0, 1_000_000_f64).floor() as u64;
        (value.to_bits(), ppm)
    }

    fn put_admitted_comparison_values(
        evidence: &mut [u8],
        wilson_win_rate_ppm: u64,
        decided_folds: u64,
        profitable_oos_folds: u64,
        aggregate_oos_paisa: i64,
    ) {
        let mut offset = RUNNER_HEADER_BYTES;
        for value in [100, 20, 50, 500, 2_000_000, 800_000] {
            put_measured_u64(evidence, &mut offset, value);
        }
        put_measured_u64(evidence, &mut offset, wilson_win_rate_ppm);
        put_measured_u64(evidence, &mut offset, 3_000_000);
        put_measured_i64(evidence, &mut offset, 0);
        for value in [
            0,
            49_751,
            49_751,
            decided_folds,
            100_000,
            100_000,
            300_000,
            200_000,
        ] {
            put_measured_u64(evidence, &mut offset, value);
        }
        for _ in 0..4 {
            evidence[offset] = 0;
            offset += 1;
        }
        for value in [10_000, 2_000, 200_000, 10] {
            put_measured_u64(evidence, &mut offset, value);
        }
        put_measured_i64(evidence, &mut offset, 1_000);
        for value in [40, 300, 150, 2_000_000, 3, 3, 200, 4, 40, 6, 2] {
            put_measured_u64(evidence, &mut offset, value);
        }
        put_measured_u64(evidence, &mut offset, profitable_oos_folds);
        put_measured_i64(evidence, &mut offset, aggregate_oos_paisa);
        put_measured_u64(evidence, &mut offset, 49_751);
        put_measured_u64(evidence, &mut offset, 49_751);
        evidence[offset] = 0;
        evidence[offset + 1] = 0;
        evidence[offset + 2] = 0;
        offset += 3;
        assert_eq!(offset, EVIDENCE_STATISTICS_OFFSET);
    }

    fn canonical_verdict(status: AdmissionV3Status) -> [u8; RUNNER_VERDICT_BYTES] {
        let mut bytes = [0_u8; RUNNER_VERDICT_BYTES];
        put(&mut bytes, 0, b"BADM");
        bytes[4] = 3;
        put_u16(&mut bytes, 6, 1);
        put_u32(&mut bytes, 8, 33);
        let max_mae = ReasonBits::one(AdmissionReasonV1::MaxMae).bits();
        let (failed, unmeasured, refused, runner_status) = match status {
            AdmissionV3Status::Admitted => (0, 0, 0, 0),
            AdmissionV3Status::Rejected => (max_mae, 0, 0, 1),
            AdmissionV3Status::Unmeasured => (0, max_mae, 0, 2),
            AdmissionV3Status::Refused => (0, 0, max_mae, 3),
        };
        let reasons = failed | unmeasured | refused;
        put_u64(&mut bytes, 12, reasons);
        put_u64(&mut bytes, 20, failed);
        put_u64(&mut bytes, 28, unmeasured);
        put_u64(&mut bytes, 36, refused);
        bytes[44] = runner_status;
        bytes
    }

    fn runner_decision_for_status(
        source: &BlockSourceV3,
        candidate_semantic_id: [u8; 32],
        search: &SearchMemberV4,
        status: AdmissionV3Status,
    ) -> [u8; RUNNER_ADMISSION_V3_DECISION_BYTES] {
        let mut bytes = [0_u8; RUNNER_ADMISSION_V3_DECISION_BYTES];
        put(&mut bytes, 0, b"BADM");
        bytes[4] = 4;
        put_u16(&mut bytes, 6, runner::admission::ADMISSION_VERSION_V3);
        put_u32(&mut bytes, 8, RUNNER_DECISION_PAYLOAD_BYTES);
        put(&mut bytes, RUNNER_POLICY_OFFSET, &source.policy);
        let evidence = &mut bytes[RUNNER_EVIDENCE_OFFSET..RUNNER_VERDICT_OFFSET];
        put(evidence, 0, b"BADM");
        evidence[4] = 2;
        put_u16(evidence, 6, runner::admission::ADMISSION_VERSION_V3);
        put_u32(evidence, 8, RUNNER_EVIDENCE_PAYLOAD_BYTES);
        let (wilson_lower_bits, wilson_win_rate_ppm) = fixture_wilson(40, 50);
        put_admitted_comparison_values(
            evidence,
            wilson_win_rate_ppm,
            search.decided_folds,
            search.profitable_oos_folds,
            search.aggregate_oos_paisa,
        );
        let statistics = EVIDENCE_STATISTICS_OFFSET;
        for (offset, value) in [
            (0, candidate_semantic_id),
            (32, source.statistics.audit_id),
            (64, source.statistics.completion_digest),
            (96, source.statistics.observation_statistics_link_id),
            (128, source.statistics.cscv_policy_id),
            (160, source.statistics.ordered_split_id),
            (192, source.statistics.white_family_id),
            (224, source.statistics.spa_family_id),
            (256, source.statistics.romano_wolf_family_id),
        ] {
            put(evidence, statistics + offset, &value);
        }
        put_u64(evidence, statistics + 288, 50);
        put_u64(evidence, statistics + 296, 40);
        put_u64(evidence, statistics + 304, wilson_lower_bits);
        put_u64(evidence, statistics + 312, wilson_win_rate_ppm);
        put_u64(evidence, statistics + 320, source.statistics.split_count);
        put_u64(evidence, statistics + 328, 6);
        put_u64(evidence, statistics + 336, 2);
        put_u64(evidence, statistics + 344, 0);
        put_u64(evidence, statistics + 352, 6);
        for numerator_offset in [360, 376, 392, 408] {
            put_u64(evidence, statistics + numerator_offset, 10);
        }
        for denominator_offset in [368, 384, 400, 416] {
            put_u64(evidence, statistics + denominator_offset, 201);
        }
        put_u64(evidence, statistics + 424, source.statistics.draw_count);
        put_u64(
            evidence,
            statistics + 432,
            source.statistics.candidate_count,
        );
        put_u64(evidence, statistics + 440, source.statistics.period_count);
        let walk = EVIDENCE_WALK_OFFSET;
        for (offset, value) in [
            (0, search.validation_policy_id),
            (32, search.validation_family_id),
            (64, search.walk_id),
        ] {
            put(evidence, walk + offset, &value);
        }
        let search_authority = hash_slices(
            b"brutex.admission.anchored-search.authority.v3\0",
            &[
                &search.validation_policy_id,
                &search.validation_family_id,
                &search.walk_id,
                &search.fold_count.to_le_bytes(),
                &search.decided_folds.to_le_bytes(),
                &search.profitable_oos_folds.to_le_bytes(),
                &search.aggregate_oos_paisa.to_le_bytes(),
            ],
        );
        put(evidence, walk + 96, &search_authority);
        put_u64(evidence, walk + 128, search.fold_count);
        put_u64(evidence, walk + 136, search.decided_folds);
        put_u64(evidence, walk + 144, search.profitable_oos_folds);
        put_i64(evidence, walk + 152, search.aggregate_oos_paisa);
        let max_mae = match status {
            AdmissionV3Status::Admitted => ObservedU64V1::Measured(500),
            AdmissionV3Status::Rejected => ObservedU64V1::Measured(501),
            AdmissionV3Status::Unmeasured => ObservedU64V1::Unmeasured,
            AdmissionV3Status::Refused => ObservedU64V1::Refused,
        };
        put_observed_u64_at(evidence, MAX_MAE_EVIDENCE_OFFSET, max_mae);
        put(
            &mut bytes,
            RUNNER_VERDICT_OFFSET,
            &canonical_verdict(status),
        );
        let projection = AdmissionV3ArithmeticProjection::verify_decision_record_detached(&bytes)
            .expect("status-parameterized Runner V3 fixture revalidates");
        assert_eq!(AdmissionV3Status::from_runner(projection.status()), status);
        bytes
    }

    fn runner_decision(
        source: &BlockSourceV3,
        candidate_semantic_id: [u8; 32],
        search: &SearchMemberV4,
    ) -> [u8; RUNNER_ADMISSION_V3_DECISION_BYTES] {
        runner_decision_for_status(
            source,
            candidate_semantic_id,
            search,
            AdmissionV3Status::Admitted,
        )
    }

    fn decision(source: &BlockSourceV3, global_sequence: u64) -> AdmissionDecisionRecordV3 {
        let (family, family_sequence, pre_admission, search) = if global_sequence < 2 {
            (
                AdmissionV3Family::Nifty,
                global_sequence,
                source.nifty.pre_admission_authority_id,
                &source.search.nifty,
            )
        } else {
            (
                AdmissionV3Family::BankNifty,
                global_sequence - 2,
                source.banknifty.pre_admission_authority_id,
                &source.search.banknifty,
            )
        };
        let seed = u8::try_from(global_sequence + 80).expect("fixture seed fits u8");
        let candidate_semantic_id = digest(seed);
        let runner_decision = runner_decision(source, candidate_semantic_id, search);
        let evidence = &runner_decision[RUNNER_EVIDENCE_OFFSET..RUNNER_VERDICT_OFFSET];
        let verdict = &runner_decision[RUNNER_VERDICT_OFFSET..];
        let mut value = AdmissionDecisionRecordV3 {
            block_id: source.block_id,
            global_sequence,
            family,
            family_sequence,
            status: AdmissionV3Status::Admitted,
            candidate_semantic_id,
            candidate_row_digest: digest(seed.wrapping_add(1)),
            pre_admission_authority_id: pre_admission,
            statistics_period_digest: digest(seed.wrapping_add(2)),
            statistics_split_digest: digest(seed.wrapping_add(3)),
            family_search_member_id: search.member_id,
            base_evidence_id: digest(seed.wrapping_add(4)),
            runner_decision,
            runner_decision_digest: hash_slices(RUNNER_DECISION_DIGEST_DOMAIN, &[&runner_decision]),
            evidence_digest: hash_slices(RUNNER_EVIDENCE_DIGEST_DOMAIN, &[evidence]),
            verdict_digest: hash_slices(RUNNER_VERDICT_DIGEST_DOMAIN, &[verdict]),
            decision_id: [0; 32],
        };
        value.decision_id = value.derive_decision_id();
        value
    }

    pub(super) fn canonical_admission_record_for(
        candidate_semantic_id: [u8; 32],
        candidate_row_digest: [u8; 32],
        family: AdmissionV3Family,
        global_sequence: u64,
        family_sequence: u64,
    ) -> [u8; POPULATION_ADMISSION_V3_DECISION_BYTES] {
        let source = source();
        let (pre_admission_authority_id, search) = match family {
            AdmissionV3Family::Nifty => (
                source.nifty.pre_admission_authority_id,
                &source.search.nifty,
            ),
            AdmissionV3Family::BankNifty => (
                source.banknifty.pre_admission_authority_id,
                &source.search.banknifty,
            ),
        };
        let runner_decision = runner_decision(&source, candidate_semantic_id, search);
        let evidence = &runner_decision[RUNNER_EVIDENCE_OFFSET..RUNNER_VERDICT_OFFSET];
        let verdict = &runner_decision[RUNNER_VERDICT_OFFSET..];
        let mut decision = AdmissionDecisionRecordV3 {
            block_id: source.block_id,
            global_sequence,
            family,
            family_sequence,
            status: AdmissionV3Status::Admitted,
            candidate_semantic_id,
            candidate_row_digest,
            pre_admission_authority_id,
            statistics_period_digest: digest(246),
            statistics_split_digest: digest(247),
            family_search_member_id: search.member_id,
            base_evidence_id: digest(248),
            runner_decision,
            runner_decision_digest: hash_slices(RUNNER_DECISION_DIGEST_DOMAIN, &[&runner_decision]),
            evidence_digest: hash_slices(RUNNER_EVIDENCE_DIGEST_DOMAIN, &[evidence]),
            verdict_digest: hash_slices(RUNNER_VERDICT_DIGEST_DOMAIN, &[verdict]),
            decision_id: [0; 32],
        };
        decision.decision_id = decision.derive_decision_id();
        encode_decision(&decision).expect("encode parameterized Admission V3 test record")
    }

    pub(super) fn admission_fixture_for(
        request: PopulationV5AdmissionFixtureRequest,
    ) -> (
        [u8; POPULATION_ADMISSION_V3_DECISION_BYTES],
        PopulationAdmissionV3DecisionProjection,
    ) {
        let PopulationV5AdmissionFixtureRequest {
            candidate_universe_ids,
            candidate_completion_digests,
            candidate_semantic_id,
            candidate_row_digest,
            family,
            global_sequence,
            family_sequence,
            status,
        } = request;
        let mut source = source();
        source.nifty.candidate_universe_id = candidate_universe_ids[0];
        source.banknifty.candidate_universe_id = candidate_universe_ids[1];
        source.nifty.candidate_completion_digest = candidate_completion_digests[0];
        source.banknifty.candidate_completion_digest = candidate_completion_digests[1];
        source.block_id = source
            .derive_block_id()
            .expect("parameterized Admission V3 source identity derives");
        source
            .validate()
            .expect("parameterized Admission V3 source validates");
        let (pre_admission_authority_id, search) = match family {
            AdmissionV3Family::Nifty => (
                source.nifty.pre_admission_authority_id,
                &source.search.nifty,
            ),
            AdmissionV3Family::BankNifty => (
                source.banknifty.pre_admission_authority_id,
                &source.search.banknifty,
            ),
        };
        let runner_decision =
            runner_decision_for_status(&source, candidate_semantic_id, search, status);
        let evidence = &runner_decision[RUNNER_EVIDENCE_OFFSET..RUNNER_VERDICT_OFFSET];
        let verdict = &runner_decision[RUNNER_VERDICT_OFFSET..];
        let mut decision = AdmissionDecisionRecordV3 {
            block_id: source.block_id,
            global_sequence,
            family,
            family_sequence,
            status,
            candidate_semantic_id,
            candidate_row_digest,
            pre_admission_authority_id,
            statistics_period_digest: digest(246),
            statistics_split_digest: digest(247),
            family_search_member_id: search.member_id,
            base_evidence_id: digest(248),
            runner_decision,
            runner_decision_digest: hash_slices(RUNNER_DECISION_DIGEST_DOMAIN, &[&runner_decision]),
            evidence_digest: hash_slices(RUNNER_EVIDENCE_DIGEST_DOMAIN, &[evidence]),
            verdict_digest: hash_slices(RUNNER_VERDICT_DIGEST_DOMAIN, &[verdict]),
            decision_id: [0; 32],
        };
        decision.decision_id = decision.derive_decision_id();
        let raw = encode_decision(&decision)
            .expect("encode fully parameterized Admission V3 test record");
        let authenticated = AuthenticatedAdmissionDecisionRecordV3::from_canonical_record(&raw)
            .expect("parameterized Admission V3 record authenticates");
        let receipt = PopulationAdmissionV3StructuralReceipt {
            block_id: source.block_id,
            completion_id: digest(249),
            block_sequence: 0,
            first_decision_record: 0,
            decision_count: source.decision_count,
            nifty_decision_count: source.nifty_decision_count,
            banknifty_decision_count: source.banknifty_decision_count,
            ordered_decision_digest: digest(250),
        };
        let identity = PopulationAdmissionV3DecisionProjection::from_authenticated(
            receipt,
            &source,
            &authenticated.decision,
        )
        .expect("parameterized Admission V3 identity joins its owning source");
        (raw, identity)
    }

    pub(super) fn prepared() -> PreparedPopulationAdmissionV3 {
        let source = source();
        let decisions = (0_u64..4).map(|index| decision(&source, index)).collect();
        PreparedPopulationAdmissionV3 {
            source,
            decisions,
            base_candidate_count: 4,
        }
    }

    fn bounds() -> AdmissionV3Bounds {
        AdmissionV3Bounds::new(
            32,
            32 * POPULATION_ADMISSION_V3_DECISION_BYTES as u64,
            8,
            8 * POPULATION_ADMISSION_V3_COMPLETION_BYTES as u64,
            8,
        )
        .expect("fixture bounds are valid")
    }

    fn reseal<const N: usize>(raw: &mut [u8; N], payload: usize, domain: &[u8]) {
        let seal = hash_slices(domain, &[&raw[..payload]]);
        raw[payload..].copy_from_slice(&seal);
    }

    #[test]
    fn bounds_widths_offsets_reserves_and_cross_version_are_exact() {
        assert_eq!(POPULATION_ADMISSION_V3_DECISION_BYTES, 2_048);
        assert_eq!(POPULATION_ADMISSION_V3_COMPLETION_BYTES, 4_096);
        assert_eq!(RUNNER_ADMISSION_V3_DECISION_BYTES, 1_327);
        for invalid in [
            AdmissionV3Bounds::new(0, 1, 1, 4_096, 1),
            AdmissionV3Bounds::new(1, 2_048, 0, 1, 1),
            AdmissionV3Bounds::new(1, 2_048, 1, 4_096, 0),
            AdmissionV3Bounds::new(1, 2_047, 1, 4_096, 1),
            AdmissionV3Bounds::new(1, 2_048, 1, 4_095, 1),
            AdmissionV3Bounds::new(1, 2_048, 1, 4_096, 2),
        ] {
            assert!(invalid.is_err());
        }
        let prepared = prepared();
        prepared.validate().expect("fixture preparation validates");
        let decision = prepared.decisions.first().expect("fixture decision");
        let raw = encode_decision(decision).expect("encode decision");
        assert_eq!(&raw[..16], &DECISION_MAGIC);
        assert_eq!(
            u32::from_le_bytes(raw[16..20].try_into().expect("version")),
            3
        );
        assert_eq!(&raw[24..56], &prepared.source.block_id);
        assert_eq!(&raw[88..120], &decision.candidate_semantic_id);
        assert_eq!(decode_decision(&raw).expect("decode decision"), *decision);
        assert!(
            raw[1_767..DECISION_PAYLOAD_BYTES]
                .iter()
                .all(|byte| *byte == 0)
        );

        let completion = prepared
            .expected_completion(0, 0)
            .expect("complete fixture");
        let completion_raw = encode_completion(&completion).expect("encode Completion");
        assert_eq!(&completion_raw[..16], &COMPLETION_MAGIC);
        assert_eq!(
            decode_completion(&completion_raw).expect("decode Completion"),
            completion
        );
        assert!(
            completion_raw[3_142..COMPLETION_PAYLOAD_BYTES]
                .iter()
                .all(|byte| *byte == 0)
        );

        let mut v2_decision = raw;
        put_u32(&mut v2_decision, 16, 2);
        reseal(
            &mut v2_decision,
            DECISION_PAYLOAD_BYTES,
            DECISION_SEAL_DOMAIN,
        );
        assert!(decode_decision(&v2_decision).is_err());
        let mut v2_completion = completion_raw;
        put_u32(&mut v2_completion, 16, 2);
        reseal(
            &mut v2_completion,
            COMPLETION_PAYLOAD_BYTES,
            COMPLETION_SEAL_DOMAIN,
        );
        assert!(decode_completion(&v2_completion).is_err());
    }

    #[test]
    fn every_block_source_byte_rekeys_and_codecs_are_deterministic() {
        let source = source();
        source.validate().expect("source validates");
        let mut canonical = [0_u8; BLOCK_IDENTITY_BYTES];
        let mut writer = FixedWriter::new(&mut canonical);
        source
            .encode_without_id(&mut writer)
            .expect("encode identity fields");
        writer
            .require_full("test block source")
            .expect("fixed width");
        assert_eq!(source.block_id, hash_slices(BLOCK_ID_DOMAIN, &[&canonical]));
        for offset in 0..canonical.len() {
            let mut changed = canonical;
            changed[offset] ^= 1;
            assert_ne!(
                hash_slices(BLOCK_ID_DOMAIN, &[&changed]),
                source.block_id,
                "block byte {offset} did not rekey"
            );
        }
        let mut encoded = [0_u8; BLOCK_SOURCE_BYTES];
        let mut writer = FixedWriter::new(&mut encoded);
        source.encode(&mut writer).expect("encode source");
        writer.require_full("test source").expect("source width");
        let mut reader = FixedReader::new(&encoded);
        assert_eq!(
            BlockSourceV3::decode(&mut reader).expect("decode source"),
            source
        );
        assert_eq!(
            source.derive_block_id().expect("derive again"),
            source.block_id
        );
    }

    #[test]
    fn exact_grid_v4_components_counts_and_nested_identities_are_bound() {
        let baseline = source();
        baseline.validate().expect("baseline validates");

        let mut changed_search_signal = source();
        changed_search_signal.search.nifty.signal.digest[0] ^= 1;
        assert!(
            changed_search_signal.validate().is_err(),
            "a changed Search signal with a stale member identity must refuse"
        );
        rekey_search_and_block(&mut changed_search_signal);
        assert!(
            changed_search_signal.validate().is_err(),
            "a fully rekeyed Search signal must still refuse the Candidate signal join"
        );

        let mut changed_candidate_column = source();
        changed_candidate_column.banknifty.signal.column_digest[0] ^= 1;
        changed_candidate_column.block_id = changed_candidate_column
            .derive_block_id()
            .expect("derive Candidate signal-column mutation");
        assert!(
            changed_candidate_column.validate().is_err(),
            "a Candidate signal-column mutation must refuse the Search join"
        );

        let mut backwards_signal = source();
        backwards_signal.search.banknifty.signal.first_ts_micros =
            backwards_signal.search.banknifty.signal.last_ts_micros + 1;
        rekey_search_and_block(&mut backwards_signal);
        assert!(
            backwards_signal.validate().is_err(),
            "a rekeyed backwards Search signal stream must refuse"
        );

        let mut zero_signal_count = source();
        zero_signal_count.search.nifty.signal.bars = 0;
        rekey_search_and_block(&mut zero_signal_count);
        assert!(
            zero_signal_count.validate().is_err(),
            "a rekeyed zero-length Search signal stream must refuse"
        );

        let mut changed_grid = source();
        changed_grid.search.nifty.full_grid_id[0] ^= 1;
        assert!(changed_grid.validate().is_err(), "stale member must refuse");
        rekey_search_and_block(&mut changed_grid);
        changed_grid
            .validate()
            .expect("complete full-grid rekey remains structural");
        assert_ne!(
            changed_grid.search.nifty.member_id,
            baseline.search.nifty.member_id
        );
        assert_ne!(changed_grid.search.pair_id, baseline.search.pair_id);
        assert_ne!(changed_grid.block_id, baseline.block_id);

        let mut changed_count = source();
        changed_count.search.banknifty.evaluated_population_count += 1;
        assert!(changed_count.validate().is_err(), "stale count must refuse");
        rekey_search_and_block(&mut changed_count);
        changed_count
            .validate()
            .expect("complete evaluated-population rekey remains structural");
        assert_ne!(
            changed_count.search.banknifty.member_id,
            baseline.search.banknifty.member_id
        );
        assert_ne!(changed_count.search.pair_id, baseline.search.pair_id);
        assert_ne!(changed_count.block_id, baseline.block_id);

        let mut stale_pair = source();
        stale_pair.search.nifty.validation_family_id[0] ^= 1;
        stale_pair.search.nifty.member_id = stale_pair.search.nifty.derive_member_id();
        stale_pair.block_id = stale_pair
            .derive_block_id()
            .expect("derive source with stale pair");
        assert!(stale_pair.validate().is_err());

        let mut crosswired = source();
        crosswired.search.nifty.long_resolution_id = digest(220);
        rekey_search_and_block(&mut crosswired);
        assert!(crosswired.validate().is_err());

        let mut aliased = source();
        aliased.search.banknifty.short_policy_id = aliased.search.banknifty.long_policy_id;
        rekey_search_and_block(&mut aliased);
        assert!(aliased.validate().is_err());

        let mut false_candidate_composite = source();
        false_candidate_composite.nifty.exit_grid.composite[0] ^= 1;
        false_candidate_composite.block_id = false_candidate_composite
            .derive_block_id()
            .expect("derive false Candidate composite");
        assert!(false_candidate_composite.validate().is_err());
    }

    #[test]
    fn candidate_cardinality_is_independent_of_search_folds_and_all_joins_refuse() {
        let baseline = prepared();
        baseline.validate().expect("all joins validate");
        assert_eq!(baseline.source.decision_count, 4);
        assert_eq!(baseline.source.search.nifty.fold_count, 8);
        assert_eq!(baseline.source.search.banknifty.fold_count, 16);
        assert_eq!(
            baseline.source.search.nifty.evaluated_population_count,
            2_048
        );
        assert_eq!(
            baseline.source.search.banknifty.evaluated_population_count,
            8_192
        );

        let mut extinct_search = source();
        extinct_search.search.nifty.evaluated_population_count = 0;
        extinct_search.search.banknifty.evaluated_population_count = 0;
        rekey_search_and_block(&mut extinct_search);
        extinct_search
            .validate()
            .expect("zero evaluated populations do not erase Candidate receipts");
        assert_eq!(extinct_search.decision_count, 4);

        let mut wrong = prepared();
        wrong.source.observation.candidate_count = 8;
        wrong.source.block_id = wrong.source.derive_block_id().expect("derive wrong source");
        assert!(wrong.validate().is_err());

        let mut wrong = prepared();
        wrong.base_candidate_count = 3;
        assert!(wrong.validate().is_err());

        let mut wrong = prepared();
        wrong.source.base.banknifty_completion = wrong.source.base.nifty_completion;
        wrong.source.block_id = wrong
            .source
            .derive_block_id()
            .expect("derive wrong Base source");
        assert!(wrong.validate().is_err());

        let mut wrong = prepared();
        wrong.decisions.swap(0, 1);
        assert!(wrong.validate().is_err());

        let mut wrong = prepared();
        wrong.decisions[0].runner_decision_digest[0] ^= 1;
        assert!(wrong.validate().is_err());

        let mut wrong = prepared();
        wrong.decisions[0].status = AdmissionV3Status::Rejected;
        wrong.decisions[0].decision_id = wrong.decisions[0].derive_decision_id();
        assert!(wrong.validate().is_err());

        let mut wrong = prepared();
        let evidence_offset = RUNNER_EVIDENCE_OFFSET + EVIDENCE_WALK_OFFSET + 128;
        put_u64(
            &mut wrong.decisions[0].runner_decision,
            evidence_offset,
            999,
        );
        let runner = wrong.decisions[0].runner_decision;
        let evidence = &runner[RUNNER_EVIDENCE_OFFSET..RUNNER_VERDICT_OFFSET];
        wrong.decisions[0].runner_decision_digest =
            hash_slices(RUNNER_DECISION_DIGEST_DOMAIN, &[&runner]);
        wrong.decisions[0].evidence_digest =
            hash_slices(RUNNER_EVIDENCE_DIGEST_DOMAIN, &[evidence]);
        wrong.decisions[0].decision_id = wrong.decisions[0].derive_decision_id();
        assert!(wrong.validate().is_err());
    }

    #[test]
    fn zero_candidate_block_is_a_complete_receipt_not_a_missing_record() {
        let mut source = source();
        source.nifty.candidate_count = 0;
        source.banknifty.candidate_count = 0;
        source.observation.candidate_count = 0;
        source.statistics.candidate_count = 0;
        source.nifty_decision_count = 0;
        source.banknifty_decision_count = 0;
        source.decision_count = 0;
        source.block_id = source.derive_block_id().expect("derive empty source");
        let prepared = PreparedPopulationAdmissionV3 {
            source,
            decisions: Vec::new(),
            base_candidate_count: 0,
        };
        prepared.validate().expect("empty preparation validates");
        let root = TestRoot::new("empty");
        let commit = persist_population_admission_v3(root.path(), bounds(), &prepared)
            .expect("persist empty block");
        let mut authority = match commit {
            PopulationAdmissionV3Commit::Written(authority)
            | PopulationAdmissionV3Commit::Reused(authority) => authority,
        };
        let receipt = authority.structural_receipt();
        assert_eq!(receipt.decision_count(), 0);
        assert_eq!(receipt.first_decision_record(), 0);
        assert!(
            authority
                .ordered_decision_projections()
                .expect("authenticate empty ordered projection")
                .is_empty()
        );
        assert_eq!(
            std::fs::metadata(root.path().join(DECISION_FILE))
                .expect("decision file")
                .len(),
            0
        );
        assert_eq!(
            std::fs::metadata(root.path().join(COMPLETION_FILE))
                .expect("Completion file")
                .len(),
            POPULATION_ADMISSION_V3_COMPLETION_BYTES as u64
        );
    }

    #[test]
    fn successor_projection_reverifies_runner_v3_arithmetic() {
        let prepared = prepared();
        let root = TestRoot::new("successor-arithmetic");
        let commit = persist_population_admission_v3(root.path(), bounds(), &prepared)
            .expect("persist successor fixture");
        let mut authority = match commit {
            PopulationAdmissionV3Commit::Written(authority)
            | PopulationAdmissionV3Commit::Reused(authority) => authority,
        };
        let first = authority
            .successor_projection(0)
            .expect("reverify first Runner V3 decision");
        let ordered = authority
            .ordered_successor_projections()
            .expect("reverify ordered Runner V3 decisions");
        assert_eq!(ordered.len(), prepared.decisions.len());
        assert_eq!(ordered.first(), Some(&first));
        assert_eq!(first.identity().global_sequence(), 0);
        assert_eq!(
            first.canonical_record(),
            &encode_decision(&prepared.decisions[0]).expect("encode first durable outer record")
        );
        assert_eq!(
            first.runner_decision(),
            &prepared.decisions[0].runner_decision
        );
        for (projection, durable) in ordered.iter().zip(&prepared.decisions) {
            assert_eq!(
                projection.canonical_record(),
                &encode_decision(durable).expect("encode durable ordered outer record")
            );
            assert_eq!(projection.runner_decision(), &durable.runner_decision);
        }
        assert_eq!(first.verdict().status(), AdmissionStatusV1::Admitted);
        let values = first.comparison_values();
        assert_eq!(
            values.pbo_ppm,
            runner::admission::ObservedU64V1::Measured(0)
        );
        assert_eq!(
            values.fwer_p_value_ppm,
            runner::admission::ObservedU64V1::Measured(49_751)
        );
        assert_eq!(
            values.bootstrap_draws,
            runner::admission::ObservedU64V1::Measured(prepared.source.statistics.draw_count)
        );
        assert_eq!(
            values.bootstrap_strategies,
            runner::admission::ObservedU64V1::Measured(prepared.source.statistics.candidate_count)
        );
        assert_eq!(
            values.profitable_oos_folds,
            runner::admission::ObservedU64V1::Measured(
                prepared.source.search.nifty.profitable_oos_folds
            )
        );
        assert_eq!(
            values.full_precision_statistics_complete,
            runner::admission::CompletenessV1::Complete
        );
    }

    #[test]
    fn successor_projection_refuses_every_runner_byte_hash_and_status_mutation() {
        let prepared = prepared();
        let root = TestRoot::new("successor-mutations");
        let commit = persist_population_admission_v3(root.path(), bounds(), &prepared)
            .expect("persist successor mutation fixture");
        let authority = match commit {
            PopulationAdmissionV3Commit::Written(authority)
            | PopulationAdmissionV3Commit::Reused(authority) => authority,
        };
        let receipt = authority.structural_receipt();
        let baseline = prepared.decisions[0].clone();
        let baseline_raw = encode_decision(&baseline).expect("encode baseline outer decision");
        let baseline_authenticated =
            AuthenticatedAdmissionDecisionRecordV3::from_canonical_record(&baseline_raw)
                .expect("authenticate baseline outer decision");
        let baseline_projection = PopulationAdmissionV3SuccessorProjection::from_authenticated(
            receipt,
            &prepared.source,
            baseline_authenticated,
        )
        .expect("baseline successor decision revalidates");
        assert_eq!(baseline_projection.canonical_record(), &baseline_raw);
        assert_eq!(
            baseline_projection.runner_decision(),
            &baseline.runner_decision
        );

        for byte_index in 0..RUNNER_ADMISSION_V3_DECISION_BYTES {
            let mut mutated = baseline_raw;
            mutated[OUTER_RUNNER_OFFSET + byte_index] ^= 1;
            reseal(&mut mutated, DECISION_PAYLOAD_BYTES, DECISION_SEAL_DOMAIN);
            assert!(
                verify_population_v5_canonical_record(&mutated).is_err(),
                "Runner decision byte {byte_index} mutation must refuse"
            );
        }

        for (digest_kind, digest_offset) in [
            OUTER_RUNNER_DIGEST_OFFSET,
            OUTER_EVIDENCE_DIGEST_OFFSET,
            OUTER_VERDICT_DIGEST_OFFSET,
        ]
        .into_iter()
        .enumerate()
        {
            for byte_index in 0..32 {
                let mut mutated = baseline_raw;
                mutated[digest_offset + byte_index] ^= 1;
                reseal(&mut mutated, DECISION_PAYLOAD_BYTES, DECISION_SEAL_DOMAIN);
                assert!(
                    verify_population_v5_canonical_record(&mutated).is_err(),
                    "Runner digest family {digest_kind} byte {byte_index} mutation must refuse"
                );
            }
        }

        for status in [
            AdmissionV3Status::Rejected,
            AdmissionV3Status::Unmeasured,
            AdmissionV3Status::Refused,
        ] {
            let mut mutated = baseline_raw;
            mutated[OUTER_STATUS_OFFSET] = status as u8;
            reseal(&mut mutated, DECISION_PAYLOAD_BYTES, DECISION_SEAL_DOMAIN);
            assert!(
                verify_population_v5_canonical_record(&mutated).is_err(),
                "outer status {status:?} mutation must refuse"
            );
        }
    }

    #[test]
    fn embedded_verifier_is_exact_and_refuses_every_outer_byte() {
        let prepared = prepared();
        let decision = &prepared.decisions[0];
        let raw = population_v5_test_canonical_admission_record_for(
            decision.candidate_semantic_id,
            decision.candidate_row_digest,
            decision.family,
            decision.global_sequence,
            decision.family_sequence,
        );
        let projection =
            verify_population_v5_canonical_record(&raw).expect("verify exact embedded record");
        assert_eq!(projection.canonical_record(), &raw);
        assert_eq!(projection.block_id(), decision.block_id);
        assert_eq!(projection.global_sequence(), decision.global_sequence);
        assert_eq!(projection.family(), decision.family);
        assert_eq!(projection.family_sequence(), decision.family_sequence);
        assert_eq!(projection.status(), decision.status);
        assert_eq!(
            projection.candidate_semantic_id(),
            decision.candidate_semantic_id
        );
        assert_eq!(
            projection.candidate_row_digest(),
            decision.candidate_row_digest
        );
        assert_eq!(
            projection.pre_admission_authority_id(),
            decision.pre_admission_authority_id
        );
        assert_ne!(projection.statistics_period_digest(), [0; 32]);
        assert_ne!(projection.statistics_split_digest(), [0; 32]);
        assert_eq!(
            projection.family_search_member_id(),
            decision.family_search_member_id
        );
        assert_ne!(projection.base_evidence_id(), [0; 32]);
        assert_eq!(projection.runner_decision(), &decision.runner_decision);
        assert_eq!(
            projection.runner_decision_digest(),
            decision.runner_decision_digest
        );
        assert_eq!(
            projection.runner_evidence_digest(),
            decision.evidence_digest
        );
        assert_eq!(projection.runner_verdict_digest(), decision.verdict_digest);
        assert_ne!(projection.decision_id(), [0; 32]);
        assert_eq!(
            projection.comparison_values().bootstrap_draws,
            runner::admission::ObservedU64V1::Measured(prepared.source.statistics.draw_count)
        );
        assert_eq!(
            projection.verdict().status(),
            runner::admission::AdmissionStatusV1::Admitted
        );

        for byte_index in 0..POPULATION_ADMISSION_V3_DECISION_BYTES {
            let mut mutated = raw;
            mutated[byte_index] ^= 1;
            assert!(
                verify_population_v5_canonical_record(&mutated).is_err(),
                "outer byte {byte_index} mutation must refuse"
            );
        }

        let mut resealed_reserve = raw;
        resealed_reserve[DECISION_PAYLOAD_BYTES - 1] = 1;
        reseal(
            &mut resealed_reserve,
            DECISION_PAYLOAD_BYTES,
            DECISION_SEAL_DOMAIN,
        );
        assert!(verify_population_v5_canonical_record(&resealed_reserve).is_err());

        let mut resealed_runner = raw;
        resealed_runner[312 + RUNNER_HEADER_BYTES] ^= 1;
        reseal(
            &mut resealed_runner,
            DECISION_PAYLOAD_BYTES,
            DECISION_SEAL_DOMAIN,
        );
        assert!(verify_population_v5_canonical_record(&resealed_runner).is_err());
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one durable scenario checks every authenticated projection field before reuse and orphan recovery"
    )]
    fn fresh_reopen_exact_reuse_and_full_orphan_retry_are_durable() {
        let prepared = prepared();
        let root = TestRoot::new("persist-reuse");
        let first = persist_population_admission_v3(root.path(), bounds(), &prepared)
            .expect("persist fixture");
        assert!(
            matches!(&first, PopulationAdmissionV3Commit::Written(_)),
            "first commit must write"
        );
        let mut authority = match first {
            PopulationAdmissionV3Commit::Written(value)
            | PopulationAdmissionV3Commit::Reused(value) => value,
        };
        assert_eq!(
            authority.structural_receipt().block_id(),
            prepared.source.block_id
        );
        let projection = authority
            .decision_projection(0)
            .expect("read authenticated first decision");
        let ordered = authority
            .ordered_decision_projections()
            .expect("read authenticated ordered decision block");
        assert_eq!(ordered.len(), prepared.decisions.len());
        assert_eq!(ordered.first(), Some(&projection));
        let decision = prepared.decisions.first().expect("first decision");
        let family = &prepared.source.nifty;
        let search = &prepared.source.search.nifty;
        assert_eq!(projection.block_id(), prepared.source.block_id);
        assert_eq!(
            projection.completion_id(),
            authority.structural_receipt().completion_id()
        );
        assert_eq!(projection.global_sequence(), 0);
        assert_eq!(projection.family(), AdmissionV3Family::Nifty);
        assert_eq!(projection.family_sequence(), 0);
        assert_eq!(projection.status(), AdmissionV3Status::Admitted);
        assert_eq!(
            projection.candidate_universe_id(),
            family.candidate_universe_id
        );
        assert_eq!(
            projection.candidate_completion_digest(),
            family.candidate_completion_digest
        );
        assert_eq!(
            projection.candidate_semantic_id(),
            decision.candidate_semantic_id
        );
        assert_eq!(
            projection.candidate_row_digest(),
            decision.candidate_row_digest
        );
        assert_eq!(
            projection.pre_admission_authority_id(),
            decision.pre_admission_authority_id
        );
        assert_eq!(
            projection.statistics_audit_id(),
            prepared.source.statistics.audit_id
        );
        assert_eq!(
            projection.statistics_completion_digest(),
            prepared.source.statistics.completion_digest
        );
        assert_eq!(
            projection.statistics_period_digest(),
            decision.statistics_period_digest
        );
        assert_eq!(
            projection.statistics_split_digest(),
            decision.statistics_split_digest
        );
        assert_eq!(projection.search_pair_id(), prepared.source.search.pair_id);
        assert_eq!(projection.search_member_id(), search.member_id);
        assert_eq!(projection.search_signal_digest(), search.signal.digest);
        assert_eq!(projection.search_signal_bars(), search.signal.bars);
        assert_eq!(
            projection.search_signal_first_ts_micros(),
            search.signal.first_ts_micros
        );
        assert_eq!(
            projection.search_signal_last_ts_micros(),
            search.signal.last_ts_micros
        );
        assert_eq!(
            projection.search_signal_column_digest(),
            search.signal.column_digest
        );
        assert_eq!(projection.search_policy_id(), search.validation_policy_id);
        assert_eq!(projection.search_full_grid_id(), search.full_grid_id);
        assert_eq!(projection.search_long_policy_id(), search.long_policy_id);
        assert_eq!(
            projection.search_long_resolution_id(),
            search.long_resolution_id
        );
        assert_eq!(projection.search_short_policy_id(), search.short_policy_id);
        assert_eq!(
            projection.search_short_resolution_id(),
            search.short_resolution_id
        );
        assert_eq!(projection.search_family_id(), search.validation_family_id);
        assert_eq!(projection.search_walk_id(), search.walk_id);
        assert_eq!(projection.search_fold_count(), search.fold_count);
        assert_eq!(projection.search_decided_folds(), search.decided_folds);
        assert_eq!(
            projection.search_profitable_oos_folds(),
            search.profitable_oos_folds
        );
        assert_eq!(
            projection.search_aggregate_oos_paisa(),
            search.aggregate_oos_paisa
        );
        assert_eq!(
            projection.search_evaluated_population_count(),
            search.evaluated_population_count
        );
        assert_eq!(
            projection.paired_base_id(),
            prepared.source.base.paired_base
        );
        assert_eq!(
            projection.base_completion_id(),
            prepared.source.base.nifty_completion
        );
        assert_eq!(projection.base_evidence_id(), decision.base_evidence_id);
        assert_eq!(
            projection.runner_decision_digest(),
            decision.runner_decision_digest
        );
        assert_eq!(
            projection.runner_evidence_digest(),
            decision.evidence_digest
        );
        assert_eq!(projection.runner_verdict_digest(), decision.verdict_digest);
        assert_eq!(projection.decision_id(), decision.decision_id);
        let bank_projection = authority
            .decision_projection(3)
            .expect("read authenticated BANKNIFTY decision");
        assert_eq!(bank_projection.family(), AdmissionV3Family::BankNifty);
        assert_eq!(bank_projection.family_sequence(), 1);
        assert_eq!(
            bank_projection.candidate_universe_id(),
            prepared.source.banknifty.candidate_universe_id
        );
        assert_eq!(
            bank_projection.base_completion_id(),
            prepared.source.base.banknifty_completion
        );
        assert_eq!(
            bank_projection.search_member_id(),
            prepared.source.search.banknifty.member_id
        );
        assert!(authority.decision_projection(4).is_err());
        drop(authority);
        let second = persist_population_admission_v3(root.path(), bounds(), &prepared)
            .expect("reuse exact fixture");
        assert!(matches!(second, PopulationAdmissionV3Commit::Reused(_)));
        let writer = PopulationAdmissionV3Ledger::open_write(root.path(), bounds())
            .expect("open for barrier inspection");
        drop(writer);
        assert_eq!(
            std::fs::metadata(root.path().join(DECISION_FILE))
                .expect("decision file")
                .len(),
            4 * POPULATION_ADMISSION_V3_DECISION_BYTES as u64
        );

        let orphan = TestRoot::new("orphan");
        drop(
            PopulationAdmissionV3Ledger::open_write(orphan.path(), bounds())
                .expect("create orphan ledger"),
        );
        let mut decisions = OpenOptions::new()
            .append(true)
            .open(orphan.path().join(DECISION_FILE))
            .expect("open orphan decisions");
        for decision in &prepared.decisions {
            append_raw(
                &mut decisions,
                &encode_decision(decision).expect("encode orphan"),
            )
            .expect("append orphan");
        }
        decisions.sync_data().expect("sync orphan decisions");
        drop(decisions);
        let mut retry = PopulationAdmissionV3Ledger::open_write(orphan.path(), bounds())
            .expect("open complete orphan");
        assert!(matches!(
            retry.append(&prepared).expect("complete exact orphan"),
            PopulationAdmissionV3StructuralCommit::Written(_)
        ));
        assert_eq!(retry.trailing_barrier_order_code, 123);
    }

    #[test]
    fn partial_orphan_corruption_reserve_and_lock_bytes_refuse() {
        let prepared = prepared();
        let partial = TestRoot::new("partial");
        drop(
            PopulationAdmissionV3Ledger::open_write(partial.path(), bounds())
                .expect("create partial ledger"),
        );
        let mut decisions = OpenOptions::new()
            .append(true)
            .open(partial.path().join(DECISION_FILE))
            .expect("open partial decisions");
        append_raw(
            &mut decisions,
            &encode_decision(prepared.decisions.first().expect("first decision"))
                .expect("encode first"),
        )
        .expect("append first");
        decisions.sync_data().expect("sync partial");
        drop(decisions);
        let mut partial_ledger = PopulationAdmissionV3Ledger::open_write(partial.path(), bounds())
            .expect("open partial ledger");
        assert!(partial_ledger.append(&prepared).is_err());

        let corrupted = TestRoot::new("reserved");
        persist_population_admission_v3(corrupted.path(), bounds(), &prepared)
            .expect("persist corrupt fixture");
        let decision_path = corrupted.path().join(DECISION_FILE);
        let mut first = read_fixed_at::<POPULATION_ADMISSION_V3_DECISION_BYTES>(
            &mut File::open(&decision_path).expect("open first record"),
            0,
            POPULATION_ADMISSION_V3_DECISION_BYTES,
            "decision",
        )
        .expect("read first record");
        first[1_800] = 1;
        reseal(&mut first, DECISION_PAYLOAD_BYTES, DECISION_SEAL_DOMAIN);
        let mut file = OpenOptions::new()
            .write(true)
            .open(&decision_path)
            .expect("open corrupt decision");
        file.write_all(&first).expect("write corrupt first record");
        file.sync_data().expect("sync corrupt first record");
        drop(file);
        assert!(PopulationAdmissionV3Ledger::open_read(corrupted.path(), bounds()).is_err());

        let lock_root = TestRoot::new("lock-bytes");
        drop(
            PopulationAdmissionV3Ledger::open_write(lock_root.path(), bounds())
                .expect("create lock ledger"),
        );
        let mut lock = OpenOptions::new()
            .append(true)
            .open(lock_root.path().join(LOCK_FILE))
            .expect("open lock");
        lock.write_all(&[1]).expect("write forbidden lock byte");
        lock.sync_data().expect("sync lock byte");
        assert!(PopulationAdmissionV3Ledger::open_read(lock_root.path(), bounds()).is_err());
    }

    #[test]
    fn self_consistent_reseal_is_structural_only_and_stale_generation_refuses() {
        let prepared = prepared();
        let root = TestRoot::new("reseal");
        persist_population_admission_v3(root.path(), bounds(), &prepared)
            .expect("persist reseal fixture");
        let decision_path = root.path().join(DECISION_FILE);
        let completion_path = root.path().join(COMPLETION_FILE);
        let mut forged_decisions = prepared.decisions.clone();
        forged_decisions[0].base_evidence_id = digest(230);
        forged_decisions[0].decision_id = forged_decisions[0].derive_decision_id();
        let forged_prepared = PreparedPopulationAdmissionV3 {
            source: prepared.source.clone(),
            decisions: forged_decisions.clone(),
            base_candidate_count: 4,
        };
        forged_prepared
            .validate()
            .expect("forged block is self-consistent");
        let forged_completion = forged_prepared
            .expected_completion(0, 0)
            .expect("forge Completion");
        let mut decision_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&decision_path)
            .expect("open decision rewrite");
        for decision in &forged_decisions {
            decision_file
                .write_all(&encode_decision(decision).expect("encode forged decision"))
                .expect("write forged decision");
        }
        decision_file.sync_data().expect("sync forged decisions");
        let mut completion_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&completion_path)
            .expect("open Completion rewrite");
        completion_file
            .write_all(&encode_completion(&forged_completion).expect("encode forged Completion"))
            .expect("write forged Completion");
        completion_file.sync_data().expect("sync forged Completion");
        drop(decision_file);
        drop(completion_file);
        let mut reopened = PopulationAdmissionV3Ledger::open_read(root.path(), bounds())
            .expect("self-consistent reseal remains structural");
        let receipt = reopened
            .reopen_structural_receipt(&prepared.source.block_id)
            .expect("lookup reseal")
            .expect("resealed receipt exists");
        assert!(
            reopened
                .authenticate_structural_receipt(receipt, &prepared)
                .is_err()
        );

        let stale_root = TestRoot::new("stale");
        let stale_commit = persist_population_admission_v3(stale_root.path(), bounds(), &prepared)
            .expect("persist stale fixture");
        let mut stale_authority = match stale_commit {
            PopulationAdmissionV3Commit::Written(authority)
            | PopulationAdmissionV3Commit::Reused(authority) => authority,
        };
        let opened = PopulationAdmissionV3Ledger::open_read(stale_root.path(), bounds())
            .expect("open stale ledger");
        let path = stale_root.path().join(DECISION_FILE);
        let mut raw = std::fs::read(&path).expect("read stale file");
        raw[100] ^= 1;
        let mut changed = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&path)
            .expect("open stale file");
        changed.write_all(&raw).expect("write stale bytes");
        changed.sync_data().expect("sync stale bytes");
        assert!(
            opened
                .reopen_structural_receipt(&prepared.source.block_id)
                .is_err()
        );
        assert!(stale_authority.ordered_decision_projections().is_err());
        assert!(stale_authority.decision_projection(0).is_err());
    }

    #[test]
    fn late_outer_corruption_returns_no_ordered_successor_prefix() {
        let prepared = prepared();
        let root = TestRoot::new("late-outer-corruption");
        let commit = persist_population_admission_v3(root.path(), bounds(), &prepared)
            .expect("persist late-corruption fixture");
        let mut authority = match commit {
            PopulationAdmissionV3Commit::Written(authority)
            | PopulationAdmissionV3Commit::Reused(authority) => authority,
        };
        let receipt = authority.structural_receipt();
        let last_physical = receipt
            .first_decision_record()
            .checked_add(receipt.decision_count())
            .and_then(|end| end.checked_sub(1))
            .expect("nonempty final physical ordinal");
        let decision_path = root.path().join(DECISION_FILE);
        let mut last = read_fixed_at::<POPULATION_ADMISSION_V3_DECISION_BYTES>(
            &mut File::open(&decision_path).expect("open final record for read"),
            last_physical,
            POPULATION_ADMISSION_V3_DECISION_BYTES,
            "late corrupted decision",
        )
        .expect("read final record");
        last[DECISION_PAYLOAD_BYTES - 1] = 1;
        reseal(&mut last, DECISION_PAYLOAD_BYTES, DECISION_SEAL_DOMAIN);
        let mut changed = OpenOptions::new()
            .write(true)
            .open(&decision_path)
            .expect("open final record for corruption");
        changed
            .seek(SeekFrom::Start(
                last_physical
                    .checked_mul(POPULATION_ADMISSION_V3_DECISION_BYTES as u64)
                    .expect("final record offset"),
            ))
            .and_then(|_| changed.write_all(&last))
            .and_then(|()| changed.sync_data())
            .expect("write late corrupted record");
        drop(changed);

        let prefix_count = receipt
            .decision_count()
            .checked_sub(1)
            .expect("nonempty prefix count");
        let prefix = read_authenticated_decision_range(
            &mut File::open(&decision_path).expect("open intact prefix"),
            receipt.first_decision_record(),
            prefix_count,
            bounds(),
        )
        .expect("all rows before the late corruption remain canonical");
        assert_eq!(
            u64::try_from(prefix.len()).expect("prefix length fits u64"),
            prefix_count
        );
        assert!(
            read_authenticated_decision_range(
                &mut File::open(&decision_path).expect("open full corrupted range"),
                receipt.first_decision_record(),
                receipt.decision_count(),
                bounds(),
            )
            .is_err(),
            "a bad final row must return Err rather than a valid prefix"
        );
        assert!(authority.ordered_successor_projections().is_err());
        assert!(
            authority
                .successor_projection(
                    receipt
                        .decision_count()
                        .checked_sub(1)
                        .expect("last logical ordinal"),
                )
                .is_err()
        );
    }

    #[test]
    fn double_hash_detects_between_pass_mutation_and_absent_read_never_creates() {
        let absent = TestRoot::absent("read");
        assert!(PopulationAdmissionV3Ledger::open_read(absent.path(), bounds()).is_err());
        assert!(!absent.path().exists());

        let root = TestRoot::new("double-hash");
        let path = root.path().join("mutation.bin");
        std::fs::write(&path, [7_u8; 64]).expect("write generation fixture");
        let held = File::open(&path).expect("open held fixture");
        let result = file_generation_with_between_hash_action(&held, &path, 64, || {
            let mut changed = OpenOptions::new()
                .write(true)
                .open(&path)
                .map_err(|why| format!("open mutation fixture: {why}"))?;
            changed
                .seek(SeekFrom::Start(7))
                .and_then(|_| changed.write_all(&[9]))
                .and_then(|()| changed.sync_data())
                .map_err(|why| format!("mutate generation fixture: {why}"))
        });
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_and_path_replacement_are_refused() {
        use std::os::unix::fs::symlink;

        let prepared = prepared();
        let root = TestRoot::new("path-replace");
        let commit = persist_population_admission_v3(root.path(), bounds(), &prepared)
            .expect("persist path fixture");
        let mut authority = match commit {
            PopulationAdmissionV3Commit::Written(authority)
            | PopulationAdmissionV3Commit::Reused(authority) => authority,
        };
        let opened = PopulationAdmissionV3Ledger::open_read(root.path(), bounds())
            .expect("open before replacement");
        let decision = root.path().join(DECISION_FILE);
        let moved = root.path().join("moved-decisions.bin");
        std::fs::rename(&decision, &moved).expect("move decision file");
        std::fs::copy(&moved, &decision).expect("replace decision file");
        assert!(
            opened
                .reopen_structural_receipt(&prepared.source.block_id)
                .is_err()
        );
        assert!(authority.decision_projection(0).is_err());

        let linked = TestRoot::new("symlink");
        symlink(root.path(), linked.path().join("ledger-link")).expect("make root link");
        assert!(
            PopulationAdmissionV3Ledger::open_read(&linked.path().join("ledger-link"), bounds(),)
                .is_err()
        );
    }
}
