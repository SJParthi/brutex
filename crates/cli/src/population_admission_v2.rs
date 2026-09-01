//! Canonical pre-finalization Population Admission V2 records.
//!
//! This is a deliberately separate, versioned sidecar.  It does not reinterpret
//! Admission V1 bytes and it contains no final Population, strategy, Selection,
//! Admission V1 or Finalization identity.  One semantic block joins one rung of
//! NIFTY candidates followed by the same rung of BANKNIFTY candidates.  Decision
//! records are synced first; their fixed-width Completion is appended and synced
//! last.  A public reopen proves only structural consistency.  The crate-private
//! authenticated capability can be minted only by exact comparison with an
//! opaque preparation; no production preparation constructor exists yet.
//!
//! Opening, appending, retrying, reusing and promoting a receipt all perform
//! bounded full-ledger validation passes.  Their cost is O(total ledger bytes +
//! total ledger records + block decisions); repeated cumulative appends can
//! therefore be quadratic in the final ledger size.  The in-memory block index
//! provides average O(1) lookup only after O(file) generation validation.  Its
//! resident space is O(Completion records + the trailing decision block), under
//! explicit bounds, rather than O(1).
//!
//! A crash after all decisions have synced but before Completion is exactly
//! retryable.  A crash after only a proper subset of a block's fixed decision
//! records is detected as a trailing prefix but is deliberately not resumed:
//! the module has no authenticated source from which to prove the missing
//! suffix.  The next write refuses without truncating or rewriting history.

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

/// Bytes in one canonical Admission V2 decision record.
pub const POPULATION_ADMISSION_V2_DECISION_BYTES: usize = 2_048;
/// Bytes in one receipt-last Admission V2 Completion record.
pub const POPULATION_ADMISSION_V2_COMPLETION_BYTES: usize = 4_096;
/// Bytes in Runner's immutable canonical Admission V2 decision envelope.
pub const RUNNER_ADMISSION_V2_DECISION_BYTES: usize =
    runner::admission::ADMISSION_DECISION_CANONICAL_LEN_V2;
/// Bytes in the immutable Admission policy nested by the block source.
pub const ADMISSION_V2_POLICY_BYTES: usize = runner::admission::ADMISSION_POLICY_CANONICAL_LEN_V1;

/// Operator-facing refusal at the Admission V2 persistence boundary.
pub type PopulationAdmissionV2Refusal = String;

const VERSION: u32 = 2;
const DECISION_MAGIC: [u8; 16] = *b"BTX-ADMV2-DEC\0\0\0";
const COMPLETION_MAGIC: [u8; 16] = *b"BTX-ADMV2-CMP\0\0\0";
const DECISION_DOMAIN: u32 = 1;
const COMPLETION_DOMAIN: u32 = 2;
const DECISION_PAYLOAD_BYTES: usize = POPULATION_ADMISSION_V2_DECISION_BYTES - 32;
const COMPLETION_PAYLOAD_BYTES: usize = POPULATION_ADMISSION_V2_COMPLETION_BYTES - 32;
const SEAL_BYTES: usize = 32;
const BLOCK_ID_DOMAIN: &[u8] = b"brutex-population-admission-v2-block-id\0";
const DECISION_ID_DOMAIN: &[u8] = b"brutex-population-admission-v2-decision-id\0";
const ORDERED_DECISIONS_DOMAIN: &[u8] = b"brutex-population-admission-v2-ordered-decisions\0";
const COMPLETION_ID_DOMAIN: &[u8] = b"brutex-population-admission-v2-completion-id\0";
const DECISION_SEAL_DOMAIN: &[u8] = b"brutex-population-admission-v2-decision-seal\0";
const COMPLETION_SEAL_DOMAIN: &[u8] = b"brutex-population-admission-v2-completion-seal\0";
const FILE_GENERATION_DOMAIN: &[u8] = b"brutex-population-admission-v2-generation\0";
const POLICY_DIGEST_DOMAIN: &[u8] = b"brutex-population-admission-v2-policy\0";
const RUNNER_DECISION_DIGEST_DOMAIN: &[u8] = b"brutex-population-admission-v2-runner-decision\0";
const DECISION_FILE: &str = "population-admission-v2.bin";
const COMPLETION_FILE: &str = "population-admission-completions-v2.bin";
const LOCK_FILE: &str = "population-admission-v2.lock";
const READ_CHUNK_BYTES: usize = 16 * 1_024;

#[cfg(any(target_os = "android", target_os = "linux"))]
const O_NOFOLLOW_FLAG: i32 = 0x20_000;
#[cfg(target_os = "macos")]
const O_NOFOLLOW_FLAG: i32 = 0x100;

const _: () =
    assert!(DECISION_PAYLOAD_BYTES + SEAL_BYTES == POPULATION_ADMISSION_V2_DECISION_BYTES);
const _: () =
    assert!(COMPLETION_PAYLOAD_BYTES + SEAL_BYTES == POPULATION_ADMISSION_V2_COMPLETION_BYTES);

/// Explicit Admission V2 allocation and persistence ceilings.
///
/// There is intentionally no `Default`: every caller must state every physical
/// and per-block limit.  Limits refuse input before ledger-sized or block-sized
/// allocation; they never truncate or sample decisions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionV2Bounds {
    decision_records: u64,
    decision_bytes: u64,
    completion_records: u64,
    completion_bytes: u64,
    decisions_per_block: u64,
}

impl AdmissionV2Bounds {
    /// Builds nonzero explicit ceilings.
    ///
    /// # Errors
    ///
    /// Refuses zero ceilings, multiplication overflow, or byte ceilings too
    /// small to contain the stated record ceilings.
    pub fn new(
        max_decision_records: u64,
        max_decision_bytes: u64,
        max_completion_records: u64,
        max_completion_bytes: u64,
        max_decisions_per_block: u64,
    ) -> Result<Self, PopulationAdmissionV2Refusal> {
        for (name, value) in [
            ("decision records", max_decision_records),
            ("decision bytes", max_decision_bytes),
            ("completion records", max_completion_records),
            ("completion bytes", max_completion_bytes),
            ("decisions per block", max_decisions_per_block),
        ] {
            if value == 0 {
                return Err(format!("Admission V2 maximum {name} must be nonzero"));
            }
        }
        if max_decisions_per_block > max_decision_records {
            return Err(format!(
                "Admission V2 per-block maximum {max_decisions_per_block} exceeds total decision-record maximum {max_decision_records}"
            ));
        }
        let required_decision_bytes = max_decision_records
            .checked_mul(POPULATION_ADMISSION_V2_DECISION_BYTES as u64)
            .ok_or_else(|| "Admission V2 decision-byte ceiling overflowed".to_owned())?;
        if max_decision_bytes < required_decision_bytes {
            return Err(format!(
                "Admission V2 decision byte maximum {max_decision_bytes} cannot hold {max_decision_records} fixed records ({required_decision_bytes} bytes)"
            ));
        }
        let required_completion_bytes = max_completion_records
            .checked_mul(POPULATION_ADMISSION_V2_COMPLETION_BYTES as u64)
            .ok_or_else(|| "Admission V2 Completion-byte ceiling overflowed".to_owned())?;
        if max_completion_bytes < required_completion_bytes {
            return Err(format!(
                "Admission V2 Completion byte maximum {max_completion_bytes} cannot hold {max_completion_records} fixed records ({required_completion_bytes} bytes)"
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
    pub const fn max_decision_records(self) -> u64 {
        self.decision_records
    }

    /// Maximum decision-file bytes.
    #[must_use]
    pub const fn max_decision_bytes(self) -> u64 {
        self.decision_bytes
    }

    /// Maximum physical Completion records.
    #[must_use]
    pub const fn max_completion_records(self) -> u64 {
        self.completion_records
    }

    /// Maximum Completion-file bytes.
    #[must_use]
    pub const fn max_completion_bytes(self) -> u64 {
        self.completion_bytes
    }

    /// Maximum decisions admitted into one semantic block.
    #[must_use]
    pub const fn max_decisions_per_block(self) -> u64 {
        self.decisions_per_block
    }
}

/// The only canonical family tags accepted by Admission V2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AdmissionV2Family {
    /// `NSE-NIFTY`, which must precede every BANKNIFTY decision in a block.
    Nifty,
    /// `NSE-BANKNIFTY`, which follows all NIFTY decisions in a block.
    BankNifty,
}

impl AdmissionV2Family {
    const fn byte(self) -> u8 {
        match self {
            Self::Nifty => 1,
            Self::BankNifty => 2,
        }
    }

    fn from_byte(value: u8) -> Result<Self, PopulationAdmissionV2Refusal> {
        match value {
            1 => Ok(Self::Nifty),
            2 => Ok(Self::BankNifty),
            _ => Err(format!(
                "Admission V2 family byte {value} is unknown; only 1=NIFTY and 2=BANKNIFTY are canonical"
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AdmissionV2Status {
    Admitted,
    Rejected,
    Unmeasured,
    Refused,
}

impl AdmissionV2Status {
    const fn byte(self) -> u8 {
        match self {
            Self::Admitted => 1,
            Self::Rejected => 2,
            Self::Unmeasured => 3,
            Self::Refused => 4,
        }
    }

    fn from_byte(value: u8) -> Result<Self, PopulationAdmissionV2Refusal> {
        match value {
            1 => Ok(Self::Admitted),
            2 => Ok(Self::Rejected),
            3 => Ok(Self::Unmeasured),
            4 => Ok(Self::Refused),
            _ => Err(format!("Admission V2 status byte {value} is unknown")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RequestedSpanV2 {
    from_year: u16,
    from_month: u8,
    to_year: u16,
    to_month: u8,
}

impl RequestedSpanV2 {
    fn validate(self) -> Result<(), PopulationAdmissionV2Refusal> {
        for (name, year) in [("from", self.from_year), ("to", self.to_year)] {
            if !(1970..=9999).contains(&year) {
                return Err(format!(
                    "Admission V2 {name} year {year} is outside 1970..=9999"
                ));
            }
        }
        for (name, month) in [("from", self.from_month), ("to", self.to_month)] {
            if !(1..=12).contains(&month) {
                return Err(format!(
                    "Admission V2 {name} month {month} is outside 1..=12"
                ));
            }
        }
        if (self.to_year, self.to_month) < (self.from_year, self.from_month) {
            return Err("Admission V2 requested span steps backwards".to_owned());
        }
        Ok(())
    }

    fn encode(self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationAdmissionV2Refusal> {
        writer.u16(self.from_year)?;
        writer.u8(self.from_month)?;
        writer.u8(0)?;
        writer.u16(self.to_year)?;
        writer.u8(self.to_month)?;
        writer.u8(0)
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationAdmissionV2Refusal> {
        let value = Self {
            from_year: reader.u16()?,
            from_month: reader.u8()?,
            to_year: {
                reader.require_zeros(1, "Admission V2 span first reserve")?;
                reader.u16()?
            },
            to_month: reader.u8()?,
        };
        reader.require_zeros(1, "Admission V2 span second reserve")?;
        value.validate()?;
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FamilySourceV2 {
    candidate_universe_id: [u8; 32],
    candidate_completion_digest: [u8; 32],
    pre_admission_authority_id: [u8; 32],
    candidate_count: u64,
}

impl FamilySourceV2 {
    fn validate(self, name: &str) -> Result<(), PopulationAdmissionV2Refusal> {
        for (field, value) in [
            ("Candidate universe", self.candidate_universe_id),
            ("Candidate completion", self.candidate_completion_digest),
            ("PreAdmission authority", self.pre_admission_authority_id),
        ] {
            require_nonzero(&format!("{name} {field}"), value)?;
        }
        if self.candidate_count == 0 {
            return Err(format!("{name} Admission V2 candidate count is zero"));
        }
        Ok(())
    }

    fn encode(self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationAdmissionV2Refusal> {
        writer.array(&self.candidate_universe_id)?;
        writer.array(&self.candidate_completion_digest)?;
        writer.array(&self.pre_admission_authority_id)?;
        writer.u64(self.candidate_count)
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationAdmissionV2Refusal> {
        Ok(Self {
            candidate_universe_id: reader.array()?,
            candidate_completion_digest: reader.array()?,
            pre_admission_authority_id: reader.array()?,
            candidate_count: reader.u64()?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ObservationSourceV2 {
    audit_id: [u8; 32],
    completion_digest: [u8; 32],
    pair_digest: [u8; 32],
    source_digest: [u8; 32],
    policy_digest: [u8; 32],
    layout_digest: [u8; 32],
    candidate_count: u64,
    session_count: u64,
}

impl ObservationSourceV2 {
    fn validate(self) -> Result<(), PopulationAdmissionV2Refusal> {
        for (name, value) in [
            ("Observation audit", self.audit_id),
            ("Observation completion", self.completion_digest),
            ("Observation pair", self.pair_digest),
            ("Observation source", self.source_digest),
            ("Observation policy", self.policy_digest),
            ("Observation layout", self.layout_digest),
        ] {
            require_nonzero(name, value)?;
        }
        if self.candidate_count == 0 || self.session_count == 0 {
            return Err("Admission V2 Observation counts must be nonzero".to_owned());
        }
        Ok(())
    }

    fn encode(self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationAdmissionV2Refusal> {
        for value in [
            self.audit_id,
            self.completion_digest,
            self.pair_digest,
            self.source_digest,
            self.policy_digest,
            self.layout_digest,
        ] {
            writer.array(&value)?;
        }
        writer.u64(self.candidate_count)?;
        writer.u64(self.session_count)
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationAdmissionV2Refusal> {
        Ok(Self {
            audit_id: reader.array()?,
            completion_digest: reader.array()?,
            pair_digest: reader.array()?,
            source_digest: reader.array()?,
            policy_digest: reader.array()?,
            layout_digest: reader.array()?,
            candidate_count: reader.u64()?,
            session_count: reader.u64()?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StatisticsSourceV2 {
    audit_id: [u8; 32],
    completion_digest: [u8; 32],
    observation_audit_id: [u8; 32],
    observation_completion_digest: [u8; 32],
    observation_link_digest: [u8; 32],
    projection_digest: [u8; 32],
    wilson_policy_digest: [u8; 32],
    cscv_policy_digest: [u8; 32],
    split_family_digest: [u8; 32],
    ordered_candidate_digest: [u8; 32],
    ordered_period_digest: [u8; 32],
    ordered_split_digest: [u8; 32],
    white_policy_digest: [u8; 32],
    spa_policy_digest: [u8; 32],
    romano_wolf_policy_digest: [u8; 32],
    draw_policy_digest: [u8; 32],
    seed_digest: [u8; 32],
    block_policy_digest: [u8; 32],
    candidate_count: u64,
    period_count: u64,
    split_count: u64,
    draw_count: u64,
    block_length: u64,
}

impl StatisticsSourceV2 {
    fn validate(self) -> Result<(), PopulationAdmissionV2Refusal> {
        for (name, value) in [
            ("Statistics audit", self.audit_id),
            ("Statistics completion", self.completion_digest),
            ("Statistics Observation audit", self.observation_audit_id),
            (
                "Statistics Observation completion",
                self.observation_completion_digest,
            ),
            ("Statistics Observation link", self.observation_link_digest),
            ("Statistics projection", self.projection_digest),
            ("Statistics Wilson policy", self.wilson_policy_digest),
            ("Statistics CSCV policy", self.cscv_policy_digest),
            ("Statistics split family", self.split_family_digest),
            (
                "Statistics ordered candidates",
                self.ordered_candidate_digest,
            ),
            ("Statistics ordered periods", self.ordered_period_digest),
            ("Statistics ordered splits", self.ordered_split_digest),
            ("Statistics White policy", self.white_policy_digest),
            ("Statistics SPA policy", self.spa_policy_digest),
            (
                "Statistics Romano-Wolf policy",
                self.romano_wolf_policy_digest,
            ),
            ("Statistics draw policy", self.draw_policy_digest),
            ("Statistics seed", self.seed_digest),
            ("Statistics block policy", self.block_policy_digest),
        ] {
            require_nonzero(name, value)?;
        }
        for (name, value) in [
            ("candidate", self.candidate_count),
            ("period", self.period_count),
            ("split", self.split_count),
            ("draw", self.draw_count),
            ("block length", self.block_length),
        ] {
            if value == 0 {
                return Err(format!("Admission V2 Statistics {name} count is zero"));
            }
        }
        Ok(())
    }

    fn encode(self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationAdmissionV2Refusal> {
        for value in [
            self.audit_id,
            self.completion_digest,
            self.observation_audit_id,
            self.observation_completion_digest,
            self.observation_link_digest,
            self.projection_digest,
            self.wilson_policy_digest,
            self.cscv_policy_digest,
            self.split_family_digest,
            self.ordered_candidate_digest,
            self.ordered_period_digest,
            self.ordered_split_digest,
            self.white_policy_digest,
            self.spa_policy_digest,
            self.romano_wolf_policy_digest,
            self.draw_policy_digest,
            self.seed_digest,
            self.block_policy_digest,
        ] {
            writer.array(&value)?;
        }
        for value in [
            self.candidate_count,
            self.period_count,
            self.split_count,
            self.draw_count,
            self.block_length,
        ] {
            writer.u64(value)?;
        }
        Ok(())
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationAdmissionV2Refusal> {
        Ok(Self {
            audit_id: reader.array()?,
            completion_digest: reader.array()?,
            observation_audit_id: reader.array()?,
            observation_completion_digest: reader.array()?,
            observation_link_digest: reader.array()?,
            projection_digest: reader.array()?,
            wilson_policy_digest: reader.array()?,
            cscv_policy_digest: reader.array()?,
            split_family_digest: reader.array()?,
            ordered_candidate_digest: reader.array()?,
            ordered_period_digest: reader.array()?,
            ordered_split_digest: reader.array()?,
            white_policy_digest: reader.array()?,
            spa_policy_digest: reader.array()?,
            romano_wolf_policy_digest: reader.array()?,
            draw_policy_digest: reader.array()?,
            seed_digest: reader.array()?,
            block_policy_digest: reader.array()?,
            candidate_count: reader.u64()?,
            period_count: reader.u64()?,
            split_count: reader.u64()?,
            draw_count: reader.u64()?,
            block_length: reader.u64()?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SearchMemberV2 {
    validation_policy_id: [u8; 32],
    validation_family_id: [u8; 32],
    walk_facts_id: [u8; 32],
    anchored_authority_id: [u8; 32],
    candidate_count: u64,
    considered_count: u64,
    priced_count: u64,
    scored_count: u64,
    decided_count: u64,
}

impl SearchMemberV2 {
    fn validate(self, name: &str) -> Result<(), PopulationAdmissionV2Refusal> {
        for (field, value) in [
            ("validation policy", self.validation_policy_id),
            ("validation family", self.validation_family_id),
            ("walk facts", self.walk_facts_id),
            ("anchored authority", self.anchored_authority_id),
        ] {
            require_nonzero(&format!("{name} {field}"), value)?;
        }
        if self.candidate_count == 0 {
            return Err(format!("{name} search candidate count is zero"));
        }
        if self.considered_count > self.candidate_count
            || self.priced_count > self.considered_count
            || self.scored_count > self.priced_count
            || self.decided_count > self.scored_count
        {
            return Err(format!(
                "{name} search counts are not candidate>=considered>=priced>=scored>=decided"
            ));
        }
        Ok(())
    }

    fn encode(self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationAdmissionV2Refusal> {
        for value in [
            self.validation_policy_id,
            self.validation_family_id,
            self.walk_facts_id,
            self.anchored_authority_id,
        ] {
            writer.array(&value)?;
        }
        for value in [
            self.candidate_count,
            self.considered_count,
            self.priced_count,
            self.scored_count,
            self.decided_count,
        ] {
            writer.u64(value)?;
        }
        Ok(())
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationAdmissionV2Refusal> {
        Ok(Self {
            validation_policy_id: reader.array()?,
            validation_family_id: reader.array()?,
            walk_facts_id: reader.array()?,
            anchored_authority_id: reader.array()?,
            candidate_count: reader.u64()?,
            considered_count: reader.u64()?,
            priced_count: reader.u64()?,
            scored_count: reader.u64()?,
            decided_count: reader.u64()?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SearchSourceV2 {
    population_search_id: [u8; 32],
    ranking_policy_id: [u8; 32],
    source_policy_id: [u8; 32],
    finalization_family_id: [u8; 32],
    nifty: SearchMemberV2,
    banknifty: SearchMemberV2,
}

impl SearchSourceV2 {
    fn validate(self) -> Result<(), PopulationAdmissionV2Refusal> {
        for (name, value) in [
            ("population search", self.population_search_id),
            ("ranking policy", self.ranking_policy_id),
            ("search source policy", self.source_policy_id),
            ("search finalization family", self.finalization_family_id),
        ] {
            require_nonzero(name, value)?;
        }
        self.nifty.validate("NIFTY")?;
        self.banknifty.validate("BANKNIFTY")?;
        if self.nifty.anchored_authority_id == self.banknifty.anchored_authority_id {
            return Err("Admission V2 NIFTY and BANKNIFTY anchored authorities alias".to_owned());
        }
        Ok(())
    }

    fn encode(self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationAdmissionV2Refusal> {
        for value in [
            self.population_search_id,
            self.ranking_policy_id,
            self.source_policy_id,
            self.finalization_family_id,
        ] {
            writer.array(&value)?;
        }
        self.nifty.encode(writer)?;
        self.banknifty.encode(writer)
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationAdmissionV2Refusal> {
        Ok(Self {
            population_search_id: reader.array()?,
            ranking_policy_id: reader.array()?,
            source_policy_id: reader.array()?,
            finalization_family_id: reader.array()?,
            nifty: SearchMemberV2::decode(reader)?,
            banknifty: SearchMemberV2::decode(reader)?,
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
struct BlockSourceV2 {
    signal_rung: u16,
    horizon_bars: u32,
    requested_span: RequestedSpanV2,
    feed_id: [u8; 32],
    source_commit_id: [u8; 32],
    calendar_policy_id: [u8; 32],
    daily_policy_id: [u8; 32],
    vocabulary_id: [u8; 32],
    evaluation_policy_id: [u8; 32],
    exit_grid_policy_id: [u8; 32],
    ranking_policy_id: [u8; 32],
    admission_source_policy_id: [u8; 32],
    stored_source_policy_id: [u8; 32],
    nifty: FamilySourceV2,
    banknifty: FamilySourceV2,
    observation: ObservationSourceV2,
    statistics: StatisticsSourceV2,
    search: SearchSourceV2,
    policy: [u8; ADMISSION_V2_POLICY_BYTES],
    policy_digest: [u8; 32],
    nifty_decision_count: u64,
    banknifty_decision_count: u64,
    decision_count: u64,
    block_id: [u8; 32],
}

impl BlockSourceV2 {
    fn validate(&self) -> Result<(), PopulationAdmissionV2Refusal> {
        if self.signal_rung == 0 {
            return Err("Admission V2 signal rung must be nonzero".to_owned());
        }
        if self.horizon_bars == 0 {
            return Err("Admission V2 horizon bars must be nonzero".to_owned());
        }
        self.requested_span.validate()?;
        for (name, value) in [
            ("feed", self.feed_id),
            ("source commit", self.source_commit_id),
            ("calendar policy", self.calendar_policy_id),
            ("daily policy", self.daily_policy_id),
            ("vocabulary", self.vocabulary_id),
            ("evaluation policy", self.evaluation_policy_id),
            ("exit-grid policy", self.exit_grid_policy_id),
            ("ranking policy", self.ranking_policy_id),
            ("admission source policy", self.admission_source_policy_id),
            ("stored source policy", self.stored_source_policy_id),
        ] {
            require_nonzero(&format!("Admission V2 {name}"), value)?;
        }
        self.nifty.validate("NIFTY")?;
        self.banknifty.validate("BANKNIFTY")?;
        self.validate_family_identities()?;
        self.observation.validate()?;
        self.statistics.validate()?;
        self.search.validate()?;
        self.validate_source_joins()?;
        self.validate_cardinalities()?;
        let expected_policy_digest = hash_slices(POLICY_DIGEST_DOMAIN, &[&self.policy]);
        if self.policy_digest != expected_policy_digest {
            return Err("Admission V2 policy digest does not match exact policy bytes".to_owned());
        }
        let expected_block_id = self.derive_block_id()?;
        if self.block_id != expected_block_id {
            return Err("Admission V2 block identity does not match canonical sources".to_owned());
        }
        Ok(())
    }

    fn validate_family_identities(&self) -> Result<(), PopulationAdmissionV2Refusal> {
        for (name, nifty, banknifty) in [
            (
                "Candidate universe",
                self.nifty.candidate_universe_id,
                self.banknifty.candidate_universe_id,
            ),
            (
                "Candidate completion",
                self.nifty.candidate_completion_digest,
                self.banknifty.candidate_completion_digest,
            ),
            (
                "PreAdmission authority",
                self.nifty.pre_admission_authority_id,
                self.banknifty.pre_admission_authority_id,
            ),
        ] {
            if nifty == banknifty {
                return Err(format!(
                    "Admission V2 NIFTY and BANKNIFTY {name} identities alias"
                ));
            }
        }
        Ok(())
    }

    fn validate_source_joins(&self) -> Result<(), PopulationAdmissionV2Refusal> {
        if self.ranking_policy_id != self.search.ranking_policy_id {
            return Err("Admission V2 ranking policy crosswires its search lineage".to_owned());
        }
        if self.stored_source_policy_id != self.search.source_policy_id {
            return Err(
                "Admission V2 stored-source policy crosswires its search lineage".to_owned(),
            );
        }
        if self.observation.audit_id != self.statistics.observation_audit_id
            || self.observation.completion_digest != self.statistics.observation_completion_digest
        {
            return Err("Admission V2 Statistics crosswires its Observation authority".to_owned());
        }
        Ok(())
    }

    fn validate_cardinalities(&self) -> Result<(), PopulationAdmissionV2Refusal> {
        let family_total = self
            .nifty_decision_count
            .checked_add(self.banknifty_decision_count)
            .ok_or_else(|| "Admission V2 family decision total overflowed".to_owned())?;
        if family_total != self.decision_count {
            return Err(format!(
                "Admission V2 family decisions total {family_total}, not declared {}",
                self.decision_count
            ));
        }
        if self.nifty_decision_count == 0 || self.banknifty_decision_count == 0 {
            return Err("Admission V2 requires nonzero NIFTY and BANKNIFTY decisions".to_owned());
        }
        if self.nifty_decision_count != self.nifty.candidate_count
            || self.banknifty_decision_count != self.banknifty.candidate_count
        {
            return Err(
                "Admission V2 family decision counts do not cover exact Candidate families"
                    .to_owned(),
            );
        }
        if self.decision_count != self.observation.candidate_count
            || self.decision_count != self.statistics.candidate_count
        {
            return Err(
                "Admission V2 Candidate, Observation and Statistics cardinalities differ"
                    .to_owned(),
            );
        }
        if self.nifty.candidate_count != self.search.nifty.candidate_count
            || self.banknifty.candidate_count != self.search.banknifty.candidate_count
        {
            return Err(
                "Admission V2 Candidate and anchored-search family counts differ".to_owned(),
            );
        }
        Ok(())
    }

    fn derive_block_id(&self) -> Result<[u8; 32], PopulationAdmissionV2Refusal> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve(COMPLETION_PAYLOAD_BYTES)
            .map_err(|why| format!("cannot reserve Admission V2 block identity bytes: {why}"))?;
        {
            let mut writer = GrowingWriter::new(&mut bytes);
            writer.u32(VERSION);
            writer.u16(self.signal_rung);
            writer.u16(0);
            writer.u32(self.horizon_bars);
            writer.u32(0);
            writer.span(self.requested_span);
            for value in [
                self.feed_id,
                self.source_commit_id,
                self.calendar_policy_id,
                self.daily_policy_id,
                self.vocabulary_id,
                self.evaluation_policy_id,
                self.exit_grid_policy_id,
                self.ranking_policy_id,
                self.admission_source_policy_id,
                self.stored_source_policy_id,
            ] {
                writer.array(value);
            }
            writer.family(self.nifty);
            writer.family(self.banknifty);
            writer.observation(self.observation);
            writer.statistics(&self.statistics);
            writer.search(&self.search);
            writer.array(self.policy_digest);
            writer.u64(self.nifty_decision_count);
            writer.u64(self.banknifty_decision_count);
            writer.u64(self.decision_count);
        }
        Ok(hash_slices(BLOCK_ID_DOMAIN, &[&bytes]))
    }

    fn encode(&self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationAdmissionV2Refusal> {
        writer.u16(self.signal_rung)?;
        writer.u16(0)?;
        writer.u32(self.horizon_bars)?;
        self.requested_span.encode(writer)?;
        for value in [
            self.feed_id,
            self.source_commit_id,
            self.calendar_policy_id,
            self.daily_policy_id,
            self.vocabulary_id,
            self.evaluation_policy_id,
            self.exit_grid_policy_id,
            self.ranking_policy_id,
            self.admission_source_policy_id,
            self.stored_source_policy_id,
        ] {
            writer.array(&value)?;
        }
        self.nifty.encode(writer)?;
        self.banknifty.encode(writer)?;
        self.observation.encode(writer)?;
        self.statistics.encode(writer)?;
        self.search.encode(writer)?;
        writer.array(&self.policy)?;
        writer.array(&self.policy_digest)?;
        writer.u64(self.nifty_decision_count)?;
        writer.u64(self.banknifty_decision_count)?;
        writer.u64(self.decision_count)?;
        writer.array(&self.block_id)
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationAdmissionV2Refusal> {
        let signal_rung = reader.u16()?;
        reader.require_zeros(2, "Admission V2 block rung reserve")?;
        let value = Self {
            signal_rung,
            horizon_bars: reader.u32()?,
            requested_span: RequestedSpanV2::decode(reader)?,
            feed_id: reader.array()?,
            source_commit_id: reader.array()?,
            calendar_policy_id: reader.array()?,
            daily_policy_id: reader.array()?,
            vocabulary_id: reader.array()?,
            evaluation_policy_id: reader.array()?,
            exit_grid_policy_id: reader.array()?,
            ranking_policy_id: reader.array()?,
            admission_source_policy_id: reader.array()?,
            stored_source_policy_id: reader.array()?,
            nifty: FamilySourceV2::decode(reader)?,
            banknifty: FamilySourceV2::decode(reader)?,
            observation: ObservationSourceV2::decode(reader)?,
            statistics: StatisticsSourceV2::decode(reader)?,
            search: SearchSourceV2::decode(reader)?,
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

#[derive(Clone, PartialEq, Eq)]
struct AdmissionDecisionRecordV2 {
    block_id: [u8; 32],
    global_sequence: u64,
    family: AdmissionV2Family,
    family_sequence: u64,
    status: AdmissionV2Status,
    candidate_semantic_id: [u8; 32],
    candidate_row_digest: [u8; 32],
    pre_admission_authority_id: [u8; 32],
    statistics_period_digest: [u8; 32],
    statistics_split_digest: [u8; 32],
    family_search_member_id: [u8; 32],
    base_evidence_source_digest: [u8; 32],
    runner_decision: [u8; RUNNER_ADMISSION_V2_DECISION_BYTES],
    runner_decision_digest: [u8; 32],
    evidence_digest: [u8; 32],
    verdict_digest: [u8; 32],
    decision_id: [u8; 32],
}

impl AdmissionDecisionRecordV2 {
    fn validate(&self) -> Result<(), PopulationAdmissionV2Refusal> {
        for (name, value) in [
            ("block", self.block_id),
            ("candidate semantic", self.candidate_semantic_id),
            ("candidate row", self.candidate_row_digest),
            ("PreAdmission authority", self.pre_admission_authority_id),
            ("Statistics period", self.statistics_period_digest),
            ("Statistics split", self.statistics_split_digest),
            ("family search member", self.family_search_member_id),
            ("base-evidence source", self.base_evidence_source_digest),
            ("Runner decision", self.runner_decision_digest),
            ("evidence", self.evidence_digest),
            ("verdict", self.verdict_digest),
            ("decision", self.decision_id),
        ] {
            require_nonzero(&format!("Admission V2 {name}"), value)?;
        }
        let expected_runner = hash_slices(RUNNER_DECISION_DIGEST_DOMAIN, &[&self.runner_decision]);
        if self.runner_decision_digest != expected_runner {
            return Err(
                "Admission V2 Runner decision digest does not match exact bytes".to_owned(),
            );
        }
        if self.decision_id != self.derive_decision_id() {
            return Err(
                "Admission V2 decision identity does not match canonical fields".to_owned(),
            );
        }
        Ok(())
    }

    fn derive_decision_id(&self) -> [u8; 32] {
        let version = VERSION.to_le_bytes();
        let global = self.global_sequence.to_le_bytes();
        let family = [self.family.byte()];
        let family_reserve = [0_u8; 7];
        let family_sequence = self.family_sequence.to_le_bytes();
        let status = [self.status.byte()];
        let status_reserve = [0_u8; 7];
        hash_slices(
            DECISION_ID_DOMAIN,
            &[
                &version,
                &self.block_id,
                &global,
                &family,
                &family_reserve,
                &family_sequence,
                &status,
                &status_reserve,
                &self.candidate_semantic_id,
                &self.candidate_row_digest,
                &self.pre_admission_authority_id,
                &self.statistics_period_digest,
                &self.statistics_split_digest,
                &self.family_search_member_id,
                &self.base_evidence_source_digest,
                &self.runner_decision_digest,
                &self.evidence_digest,
                &self.verdict_digest,
            ],
        )
    }
}

#[derive(Clone, PartialEq, Eq)]
struct AdmissionCompletionRecordV2 {
    block_sequence: u64,
    first_decision_record: u64,
    source: BlockSourceV2,
    admitted_count: u64,
    rejected_count: u64,
    unmeasured_count: u64,
    refused_count: u64,
    ordered_decision_digest: [u8; 32],
    completion_id: [u8; 32],
}

impl AdmissionCompletionRecordV2 {
    fn validate(&self) -> Result<(), PopulationAdmissionV2Refusal> {
        self.source.validate()?;
        require_nonzero(
            "Admission V2 ordered decisions",
            self.ordered_decision_digest,
        )?;
        require_nonzero("Admission V2 Completion", self.completion_id)?;
        let classified = checked_sum(
            &[
                self.admitted_count,
                self.rejected_count,
                self.unmeasured_count,
                self.refused_count,
            ],
            "Admission V2 status counts",
        )?;
        if classified != self.source.decision_count {
            return Err(format!(
                "Admission V2 statuses total {classified}, not decision count {}",
                self.source.decision_count
            ));
        }
        if self.completion_id != self.derive_completion_id() {
            return Err(
                "Admission V2 Completion identity does not match canonical fields".to_owned(),
            );
        }
        Ok(())
    }

    fn derive_completion_id(&self) -> [u8; 32] {
        let version = VERSION.to_le_bytes();
        let decision_count = self.source.decision_count.to_le_bytes();
        let nifty = self.source.nifty_decision_count.to_le_bytes();
        let banknifty = self.source.banknifty_decision_count.to_le_bytes();
        let admitted = self.admitted_count.to_le_bytes();
        let rejected = self.rejected_count.to_le_bytes();
        let unmeasured = self.unmeasured_count.to_le_bytes();
        let refused = self.refused_count.to_le_bytes();
        hash_slices(
            COMPLETION_ID_DOMAIN,
            &[
                &version,
                &self.source.block_id,
                &decision_count,
                &nifty,
                &banknifty,
                &admitted,
                &rejected,
                &unmeasured,
                &refused,
                &self.ordered_decision_digest,
            ],
        )
    }
}

/// Opaque Admission V2 preparation.
///
/// Fields are private, this type has no `Debug`, and no production constructor
/// exists.  The future Step-3 orchestrator must build it only from freshly
/// reopened typed sources and an opaque Runner search-lineage authority.
pub(crate) struct PreparedPopulationAdmissionV2 {
    source: BlockSourceV2,
    decisions: Vec<AdmissionDecisionRecordV2>,
}

impl PreparedPopulationAdmissionV2 {
    fn checked_decision_count(&self) -> Result<u64, PopulationAdmissionV2Refusal> {
        u64::try_from(self.decisions.len())
            .map_err(|_| "Admission V2 prepared decision count does not fit u64".to_owned())
    }

    fn validate(&self) -> Result<(), PopulationAdmissionV2Refusal> {
        self.source.validate()?;
        let count = self.checked_decision_count()?;
        if count != self.source.decision_count {
            return Err(format!(
                "Admission V2 preparation has {count} decisions, not declared {}",
                self.source.decision_count
            ));
        }
        let nifty_count = usize::try_from(self.source.nifty_decision_count)
            .map_err(|_| "Admission V2 NIFTY count does not fit usize".to_owned())?;
        let mut semantics = HashSet::new();
        let mut identities = HashSet::new();
        semantics
            .try_reserve(self.decisions.len())
            .map_err(|why| format!("cannot reserve Admission V2 semantic index: {why}"))?;
        identities
            .try_reserve(self.decisions.len())
            .map_err(|why| format!("cannot reserve Admission V2 decision index: {why}"))?;
        for (index, decision) in self.decisions.iter().enumerate() {
            decision.validate()?;
            let global = u64::try_from(index)
                .map_err(|_| "Admission V2 decision index does not fit u64".to_owned())?;
            let (family, family_sequence, pre_admission, search_member) = if index < nifty_count {
                (
                    AdmissionV2Family::Nifty,
                    global,
                    self.source.nifty.pre_admission_authority_id,
                    self.source.search.nifty.anchored_authority_id,
                )
            } else {
                let bank_index = index
                    .checked_sub(nifty_count)
                    .ok_or_else(|| "Admission V2 BANKNIFTY index underflowed".to_owned())?;
                (
                    AdmissionV2Family::BankNifty,
                    u64::try_from(bank_index)
                        .map_err(|_| "Admission V2 BANKNIFTY index does not fit u64".to_owned())?,
                    self.source.banknifty.pre_admission_authority_id,
                    self.source.search.banknifty.anchored_authority_id,
                )
            };
            if decision.global_sequence != global
                || decision.family != family
                || decision.family_sequence != family_sequence
            {
                return Err(format!(
                    "Admission V2 decision {index} violates canonical NIFTY-first/BANKNIFTY ordering"
                ));
            }
            if decision.block_id != self.source.block_id
                || decision.pre_admission_authority_id != pre_admission
                || decision.family_search_member_id != search_member
            {
                return Err(format!(
                    "Admission V2 decision {index} crosswires its block/family authorities"
                ));
            }
            if !semantics.insert(decision.candidate_semantic_id) {
                return Err(format!(
                    "Admission V2 candidate semantic {} appears more than once",
                    hex32(decision.candidate_semantic_id)
                ));
            }
            if !identities.insert(decision.decision_id) {
                return Err(format!(
                    "Admission V2 decision identity {} appears more than once",
                    hex32(decision.decision_id)
                ));
            }
        }
        Ok(())
    }

    fn expected_completion(
        &self,
        block_sequence: u64,
        first_decision_record: u64,
    ) -> Result<AdmissionCompletionRecordV2, PopulationAdmissionV2Refusal> {
        self.validate()?;
        let ordered_decision_digest = ordered_decision_digest(&self.decisions)?;
        let mut admitted_count = 0_u64;
        let mut rejected_count = 0_u64;
        let mut unmeasured_count = 0_u64;
        let mut refused_count = 0_u64;
        for decision in &self.decisions {
            let target = match decision.status {
                AdmissionV2Status::Admitted => &mut admitted_count,
                AdmissionV2Status::Rejected => &mut rejected_count,
                AdmissionV2Status::Unmeasured => &mut unmeasured_count,
                AdmissionV2Status::Refused => &mut refused_count,
            };
            *target = target
                .checked_add(1)
                .ok_or_else(|| "Admission V2 status count overflowed".to_owned())?;
        }
        let mut completion = AdmissionCompletionRecordV2 {
            block_sequence,
            first_decision_record,
            source: self.source.clone(),
            admitted_count,
            rejected_count,
            unmeasured_count,
            refused_count,
            ordered_decision_digest,
            completion_id: [0; 32],
        };
        completion.completion_id = completion.derive_completion_id();
        completion.validate()?;
        Ok(completion)
    }
}

/// Structurally valid receipt returned by public bounded reopen.
///
/// This is not authenticated production authority: self-consistent disk bytes
/// can produce it, but cannot construct [`PopulationAdmissionV2Authority`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationAdmissionV2StructuralReceipt {
    block_id: [u8; 32],
    completion_id: [u8; 32],
    block_sequence: u64,
    first_decision_record: u64,
    decision_count: u64,
    nifty_decision_count: u64,
    banknifty_decision_count: u64,
    ordered_decision_digest: [u8; 32],
}

impl PopulationAdmissionV2StructuralReceipt {
    /// Semantic pre-finalization block identity.
    #[must_use]
    pub const fn block_id(self) -> [u8; 32] {
        self.block_id
    }

    /// Receipt-last Completion identity.
    #[must_use]
    pub const fn completion_id(self) -> [u8; 32] {
        self.completion_id
    }

    /// Canonical completed-block sequence in this ledger.
    #[must_use]
    pub const fn block_sequence(self) -> u64 {
        self.block_sequence
    }

    /// First physical decision record belonging to the block.
    #[must_use]
    pub const fn first_decision_record(self) -> u64 {
        self.first_decision_record
    }

    /// Complete candidate-decision count.
    #[must_use]
    pub const fn decision_count(self) -> u64 {
        self.decision_count
    }

    /// NIFTY decision count.
    #[must_use]
    pub const fn nifty_decision_count(self) -> u64 {
        self.nifty_decision_count
    }

    /// BANKNIFTY decision count.
    #[must_use]
    pub const fn banknifty_decision_count(self) -> u64 {
        self.banknifty_decision_count
    }

    /// Digest over ordered decision identities.
    #[must_use]
    pub const fn ordered_decision_digest(self) -> [u8; 32] {
        self.ordered_decision_digest
    }
}

/// Crate-private authority promoted only by exact opaque-preparation comparison.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct PopulationAdmissionV2Authority {
    receipt: PopulationAdmissionV2StructuralReceipt,
}

impl PopulationAdmissionV2Authority {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "authority consumption remains dormant until the production Step-3 constructor exists"
        )
    )]
    pub(crate) const fn structural_receipt(self) -> PopulationAdmissionV2StructuralReceipt {
        self.receipt
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PopulationAdmissionV2StructuralCommit {
    Written(PopulationAdmissionV2StructuralReceipt),
    Reused(PopulationAdmissionV2StructuralReceipt),
}

impl PopulationAdmissionV2StructuralCommit {
    const fn receipt(self) -> PopulationAdmissionV2StructuralReceipt {
        match self {
            Self::Written(receipt) | Self::Reused(receipt) => receipt,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PopulationAdmissionV2Commit {
    Written(PopulationAdmissionV2Authority),
    Reused(PopulationAdmissionV2Authority),
}

fn ordered_decision_digest(
    decisions: &[AdmissionDecisionRecordV2],
) -> Result<[u8; 32], PopulationAdmissionV2Refusal> {
    let count = u64::try_from(decisions.len())
        .map_err(|_| "Admission V2 ordered decision count does not fit u64".to_owned())?;
    let count_bytes = count.to_le_bytes();
    let mut hasher = Hasher::new();
    hasher.update(ORDERED_DECISIONS_DOMAIN);
    hasher.update(&count_bytes);
    for decision in decisions {
        hasher.update(&decision.decision_id);
    }
    Ok(hasher.finalize())
}

fn encode_decision(
    decision: &AdmissionDecisionRecordV2,
) -> Result<[u8; POPULATION_ADMISSION_V2_DECISION_BYTES], PopulationAdmissionV2Refusal> {
    decision.validate()?;
    let mut raw = [0_u8; POPULATION_ADMISSION_V2_DECISION_BYTES];
    {
        let payload = raw
            .get_mut(..DECISION_PAYLOAD_BYTES)
            .ok_or_else(|| "Admission V2 decision payload is absent".to_owned())?;
        let mut writer = FixedWriter::new(payload);
        writer.array(&DECISION_MAGIC)?;
        writer.u32(VERSION)?;
        writer.u32(DECISION_DOMAIN)?;
        writer.array(&decision.block_id)?;
        writer.u64(decision.global_sequence)?;
        writer.u8(decision.family.byte())?;
        writer.zeros(7)?;
        writer.u64(decision.family_sequence)?;
        writer.u8(decision.status.byte())?;
        writer.zeros(7)?;
        for value in [
            decision.candidate_semantic_id,
            decision.candidate_row_digest,
            decision.pre_admission_authority_id,
            decision.statistics_period_digest,
            decision.statistics_split_digest,
            decision.family_search_member_id,
            decision.base_evidence_source_digest,
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
        &[raw
            .get(..DECISION_PAYLOAD_BYTES)
            .ok_or_else(|| "Admission V2 decision seal payload is absent".to_owned())?],
    );
    raw.get_mut(DECISION_PAYLOAD_BYTES..)
        .ok_or_else(|| "Admission V2 decision seal slot is absent".to_owned())?
        .copy_from_slice(&seal);
    Ok(raw)
}

fn decode_decision(
    raw: &[u8; POPULATION_ADMISSION_V2_DECISION_BYTES],
) -> Result<AdmissionDecisionRecordV2, PopulationAdmissionV2Refusal> {
    let payload = raw
        .get(..DECISION_PAYLOAD_BYTES)
        .ok_or_else(|| "Admission V2 decision payload is absent".to_owned())?;
    let stored_seal: [u8; 32] = raw
        .get(DECISION_PAYLOAD_BYTES..)
        .ok_or_else(|| "Admission V2 decision seal is absent".to_owned())?
        .try_into()
        .map_err(|_| "Admission V2 decision seal width is invalid".to_owned())?;
    if stored_seal != hash_slices(DECISION_SEAL_DOMAIN, &[payload]) {
        return Err("Admission V2 decision seal mismatch".to_owned());
    }
    let mut reader = FixedReader::new(payload);
    if reader.array::<16>()? != DECISION_MAGIC {
        return Err("Admission V2 decision magic mismatch".to_owned());
    }
    if reader.u32()? != VERSION {
        return Err("Admission V2 decision version mismatch".to_owned());
    }
    if reader.u32()? != DECISION_DOMAIN {
        return Err("Admission V2 decision domain mismatch".to_owned());
    }
    let block_id = reader.array()?;
    let global_sequence = reader.u64()?;
    let family = AdmissionV2Family::from_byte(reader.u8()?)?;
    reader.require_zeros(7, "Admission V2 decision family reserve")?;
    let family_sequence = reader.u64()?;
    let status = AdmissionV2Status::from_byte(reader.u8()?)?;
    reader.require_zeros(7, "Admission V2 decision status reserve")?;
    let value = AdmissionDecisionRecordV2 {
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
        base_evidence_source_digest: reader.array()?,
        runner_decision: reader.array()?,
        runner_decision_digest: reader.array()?,
        evidence_digest: reader.array()?,
        verdict_digest: reader.array()?,
        decision_id: reader.array()?,
    };
    reader.require_remaining_zero("Admission V2 decision reserve")?;
    value.validate()?;
    Ok(value)
}

fn encode_completion(
    completion: &AdmissionCompletionRecordV2,
) -> Result<[u8; POPULATION_ADMISSION_V2_COMPLETION_BYTES], PopulationAdmissionV2Refusal> {
    completion.validate()?;
    let mut raw = [0_u8; POPULATION_ADMISSION_V2_COMPLETION_BYTES];
    {
        let payload = raw
            .get_mut(..COMPLETION_PAYLOAD_BYTES)
            .ok_or_else(|| "Admission V2 Completion payload is absent".to_owned())?;
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
        &[raw
            .get(..COMPLETION_PAYLOAD_BYTES)
            .ok_or_else(|| "Admission V2 Completion seal payload is absent".to_owned())?],
    );
    raw.get_mut(COMPLETION_PAYLOAD_BYTES..)
        .ok_or_else(|| "Admission V2 Completion seal slot is absent".to_owned())?
        .copy_from_slice(&seal);
    Ok(raw)
}

fn decode_completion(
    raw: &[u8; POPULATION_ADMISSION_V2_COMPLETION_BYTES],
) -> Result<AdmissionCompletionRecordV2, PopulationAdmissionV2Refusal> {
    let payload = raw
        .get(..COMPLETION_PAYLOAD_BYTES)
        .ok_or_else(|| "Admission V2 Completion payload is absent".to_owned())?;
    let stored_seal: [u8; 32] = raw
        .get(COMPLETION_PAYLOAD_BYTES..)
        .ok_or_else(|| "Admission V2 Completion seal is absent".to_owned())?
        .try_into()
        .map_err(|_| "Admission V2 Completion seal width is invalid".to_owned())?;
    if stored_seal != hash_slices(COMPLETION_SEAL_DOMAIN, &[payload]) {
        return Err("Admission V2 Completion seal mismatch".to_owned());
    }
    let mut reader = FixedReader::new(payload);
    if reader.array::<16>()? != COMPLETION_MAGIC {
        return Err("Admission V2 Completion magic mismatch".to_owned());
    }
    if reader.u32()? != VERSION {
        return Err("Admission V2 Completion version mismatch".to_owned());
    }
    if reader.u32()? != COMPLETION_DOMAIN {
        return Err("Admission V2 Completion domain mismatch".to_owned());
    }
    let value = AdmissionCompletionRecordV2 {
        block_sequence: reader.u64()?,
        first_decision_record: reader.u64()?,
        source: BlockSourceV2::decode(&mut reader)?,
        admitted_count: reader.u64()?,
        rejected_count: reader.u64()?,
        unmeasured_count: reader.u64()?,
        refused_count: reader.u64()?,
        ordered_decision_digest: reader.array()?,
        completion_id: reader.array()?,
    };
    reader.require_remaining_zero("Admission V2 Completion reserve")?;
    value.validate()?;
    Ok(value)
}

struct FixedWriter<'a> {
    bytes: &'a mut [u8],
    cursor: usize,
}

impl<'a> FixedWriter<'a> {
    const fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn write(&mut self, value: &[u8]) -> Result<(), PopulationAdmissionV2Refusal> {
        let end = self
            .cursor
            .checked_add(value.len())
            .ok_or_else(|| "Admission V2 encoder cursor overflowed".to_owned())?;
        self.bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "Admission V2 fixed record is too small".to_owned())?
            .copy_from_slice(value);
        self.cursor = end;
        Ok(())
    }

    fn array<const N: usize>(
        &mut self,
        value: &[u8; N],
    ) -> Result<(), PopulationAdmissionV2Refusal> {
        self.write(value)
    }

    fn u8(&mut self, value: u8) -> Result<(), PopulationAdmissionV2Refusal> {
        self.write(&[value])
    }

    fn u16(&mut self, value: u16) -> Result<(), PopulationAdmissionV2Refusal> {
        self.write(&value.to_le_bytes())
    }

    fn u32(&mut self, value: u32) -> Result<(), PopulationAdmissionV2Refusal> {
        self.write(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), PopulationAdmissionV2Refusal> {
        self.write(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), PopulationAdmissionV2Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "Admission V2 reserve cursor overflowed".to_owned())?;
        let reserve = self
            .bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "Admission V2 reserve escapes fixed record".to_owned())?;
        reserve.fill(0);
        self.cursor = end;
        Ok(())
    }

    fn zero_remaining(&mut self) {
        if let Some(remaining) = self.bytes.get_mut(self.cursor..) {
            remaining.fill(0);
            self.cursor = self.bytes.len();
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

    fn take<const N: usize>(&mut self) -> Result<[u8; N], PopulationAdmissionV2Refusal> {
        let end = self
            .cursor
            .checked_add(N)
            .ok_or_else(|| "Admission V2 decoder cursor overflowed".to_owned())?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| "Admission V2 fixed record ended early".to_owned())?;
        self.cursor = end;
        value
            .try_into()
            .map_err(|_| "Admission V2 fixed-width decode failed".to_owned())
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], PopulationAdmissionV2Refusal> {
        self.take()
    }

    fn u8(&mut self) -> Result<u8, PopulationAdmissionV2Refusal> {
        let value = self.take::<1>()?;
        value
            .first()
            .copied()
            .ok_or_else(|| "Admission V2 byte decode failed".to_owned())
    }

    fn u16(&mut self) -> Result<u16, PopulationAdmissionV2Refusal> {
        Ok(u16::from_le_bytes(self.take()?))
    }

    fn u32(&mut self) -> Result<u32, PopulationAdmissionV2Refusal> {
        Ok(u32::from_le_bytes(self.take()?))
    }

    fn u64(&mut self) -> Result<u64, PopulationAdmissionV2Refusal> {
        Ok(u64::from_le_bytes(self.take()?))
    }

    fn require_zeros(
        &mut self,
        count: usize,
        name: &str,
    ) -> Result<(), PopulationAdmissionV2Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| format!("{name} cursor overflowed"))?;
        let reserve = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| format!("{name} escapes fixed record"))?;
        if reserve.iter().any(|byte| *byte != 0) {
            return Err(format!("{name} is nonzero"));
        }
        self.cursor = end;
        Ok(())
    }

    fn require_remaining_zero(&mut self, name: &str) -> Result<(), PopulationAdmissionV2Refusal> {
        let remaining = self
            .bytes
            .get(self.cursor..)
            .ok_or_else(|| format!("{name} cursor is outside record"))?;
        if let Some(relative) = remaining.iter().position(|byte| *byte != 0) {
            return Err(format!(
                "{name} is nonzero at byte {}",
                self.cursor + relative
            ));
        }
        self.cursor = self.bytes.len();
        Ok(())
    }
}

struct GrowingWriter<'a> {
    bytes: &'a mut Vec<u8>,
}

impl<'a> GrowingWriter<'a> {
    const fn new(bytes: &'a mut Vec<u8>) -> Self {
        Self { bytes }
    }

    fn array<const N: usize>(&mut self, value: [u8; N]) {
        self.bytes.extend_from_slice(&value);
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn span(&mut self, value: RequestedSpanV2) {
        self.u16(value.from_year);
        self.bytes.push(value.from_month);
        self.bytes.push(0);
        self.u16(value.to_year);
        self.bytes.push(value.to_month);
        self.bytes.push(0);
    }

    fn family(&mut self, value: FamilySourceV2) {
        self.array(value.candidate_universe_id);
        self.array(value.candidate_completion_digest);
        self.array(value.pre_admission_authority_id);
        self.u64(value.candidate_count);
    }

    fn observation(&mut self, value: ObservationSourceV2) {
        for item in [
            value.audit_id,
            value.completion_digest,
            value.pair_digest,
            value.source_digest,
            value.policy_digest,
            value.layout_digest,
        ] {
            self.array(item);
        }
        self.u64(value.candidate_count);
        self.u64(value.session_count);
    }

    fn statistics(&mut self, value: &StatisticsSourceV2) {
        for item in [
            value.audit_id,
            value.completion_digest,
            value.observation_audit_id,
            value.observation_completion_digest,
            value.observation_link_digest,
            value.projection_digest,
            value.wilson_policy_digest,
            value.cscv_policy_digest,
            value.split_family_digest,
            value.ordered_candidate_digest,
            value.ordered_period_digest,
            value.ordered_split_digest,
            value.white_policy_digest,
            value.spa_policy_digest,
            value.romano_wolf_policy_digest,
            value.draw_policy_digest,
            value.seed_digest,
            value.block_policy_digest,
        ] {
            self.array(item);
        }
        for item in [
            value.candidate_count,
            value.period_count,
            value.split_count,
            value.draw_count,
            value.block_length,
        ] {
            self.u64(item);
        }
    }

    fn search_member(&mut self, value: SearchMemberV2) {
        for item in [
            value.validation_policy_id,
            value.validation_family_id,
            value.walk_facts_id,
            value.anchored_authority_id,
        ] {
            self.array(item);
        }
        for item in [
            value.candidate_count,
            value.considered_count,
            value.priced_count,
            value.scored_count,
            value.decided_count,
        ] {
            self.u64(item);
        }
    }

    fn search(&mut self, value: &SearchSourceV2) {
        for item in [
            value.population_search_id,
            value.ranking_policy_id,
            value.source_policy_id,
            value.finalization_family_id,
        ] {
            self.array(item);
        }
        self.search_member(value.nifty);
        self.search_member(value.banknifty);
    }
}

fn hash_slices(domain: &[u8], slices: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    for slice in slices {
        hasher.update(slice);
    }
    hasher.finalize()
}

fn require_nonzero(name: &str, value: [u8; 32]) -> Result<(), PopulationAdmissionV2Refusal> {
    if value == [0; 32] {
        return Err(format!("{name} is zero"));
    }
    Ok(())
}

fn checked_sum(values: &[u64], name: &str) -> Result<u64, PopulationAdmissionV2Refusal> {
    values.iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(*value)
            .ok_or_else(|| format!("{name} overflowed"))
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformIdentity {
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(windows)]
    volume_serial: Option<u32>,
    #[cfg(windows)]
    file_index: u64,
}

impl PlatformIdentity {
    fn of(metadata: &std::fs::Metadata) -> Self {
        Self {
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
            #[cfg(windows)]
            volume_serial: metadata.volume_serial_number(),
            #[cfg(windows)]
            file_index: metadata.file_index(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGeneration {
    identity: PlatformIdentity,
    len: u64,
    digest: [u8; 32],
}

#[derive(Clone, PartialEq, Eq)]
struct TrailingDecisionBlockV2 {
    first_decision_record: u64,
    block_id: [u8; 32],
    decisions: Vec<AdmissionDecisionRecordV2>,
}

/// Bounded structural view over Admission V2 decision and Completion files.
///
/// `open_read` never creates the root or any child and never returns production
/// authority.  It scans both explicitly bounded files, verifies every completed
/// block and builds an average-O(1) block-identity index.
pub struct PopulationAdmissionV2Ledger {
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
    bounds: AdmissionV2Bounds,
    writable: bool,
    receipts: HashMap<[u8; 32], PopulationAdmissionV2StructuralReceipt>,
    trailing: Option<TrailingDecisionBlockV2>,
    decision_records: u64,
    completion_records: u64,
    #[cfg(test)]
    reuse_barrier_count: u8,
    #[cfg(test)]
    trailing_barrier_order_code: u16,
}

impl PopulationAdmissionV2Ledger {
    /// Opens and validates an existing Admission V2 ledger without creating any
    /// root or child path.
    ///
    /// # Complexity
    ///
    /// O(decision records + Completion records), bounded by `bounds`.
    ///
    /// # Errors
    ///
    /// Refuses an absent/non-directory/symlink root, absent/symlink/non-regular
    /// files, a lock failure, exceeded bounds, ragged/torn/corrupt/reordered/
    /// duplicate/crosswired bytes, or a generation/path change during open.
    pub fn open_read(
        root: &Path,
        bounds: AdmissionV2Bounds,
    ) -> Result<Self, PopulationAdmissionV2Refusal> {
        Self::open(root, bounds, false)
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "write admission remains dormant until opaque Runner search lineage can construct PreparedPopulationAdmissionV2"
        )
    )]
    fn open_write(
        root: &Path,
        bounds: AdmissionV2Bounds,
    ) -> Result<Self, PopulationAdmissionV2Refusal> {
        Self::open(root, bounds, true)
    }

    fn open(
        root: &Path,
        bounds: AdmissionV2Bounds,
        writable: bool,
    ) -> Result<Self, PopulationAdmissionV2Refusal> {
        let (root_path, root_file, root_identity) = open_root_directory(root)?;
        let lock_path = root_path.join(LOCK_FILE);
        let decision_path = root_path.join(DECISION_FILE);
        let completion_path = root_path.join(COMPLETION_FILE);
        let (lock_file, lock_created) = open_child(&lock_path, writable, writable)?;
        if writable {
            lock_file
                .lock()
                .map_err(|why| format!("cannot take exclusive Admission V2 open lock: {why}"))?;
        } else {
            lock_file
                .lock_shared()
                .map_err(|why| format!("cannot take shared Admission V2 open lock: {why}"))?;
        }
        let opened = (|| {
            let (decision_file, decision_created) = open_child(&decision_path, writable, writable)?;
            let (completion_file, completion_created) =
                open_child(&completion_path, writable, writable)?;
            if lock_created || decision_created || completion_created {
                sync_directory(&root_file, &root_path)?;
            }
            let root_identity_after = named_root_identity(&root_path)?;
            if root_identity_after != root_identity {
                return Err("Admission V2 root changed while child files opened".to_owned());
            }
            let lock_generation = file_generation(&lock_file, &lock_path, bounds.completion_bytes)?;
            let decision_generation =
                file_generation(&decision_file, &decision_path, bounds.decision_bytes)?;
            let completion_generation =
                file_generation(&completion_file, &completion_path, bounds.completion_bytes)?;
            let mut ledger = Self {
                root: root_path,
                root_file,
                root_identity,
                lock_path,
                lock_file: lock_file
                    .try_clone()
                    .map_err(|why| format!("cannot clone Admission V2 held lock file: {why}"))?,
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
            .map_err(|why| format!("cannot release Admission V2 open lock: {why}"));
        match (opened, released) {
            (Ok(ledger), Ok(())) => Ok(ledger),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn scan(&mut self) -> Result<(), PopulationAdmissionV2Refusal> {
        let decision_records = checked_record_count(
            self.decision_generation.len,
            POPULATION_ADMISSION_V2_DECISION_BYTES,
            self.bounds.decision_records,
            "decision",
        )?;
        let completion_records = checked_record_count(
            self.completion_generation.len,
            POPULATION_ADMISSION_V2_COMPLETION_BYTES,
            self.bounds.completion_records,
            "Completion",
        )?;
        self.receipts = HashMap::new();
        self.receipts
            .try_reserve(
                usize::try_from(completion_records)
                    .map_err(|_| "Admission V2 Completion count does not fit usize".to_owned())?,
            )
            .map_err(|why| format!("cannot reserve Admission V2 receipt index: {why}"))?;
        self.trailing = None;
        let mut covered_decisions = 0_u64;
        let mut completion_index = 0_u64;
        while completion_index < completion_records {
            let raw = read_completion_at(&mut self.completion_file, completion_index)?;
            let completion = decode_completion(&raw)?;
            if completion.block_sequence != completion_index {
                return Err(format!(
                    "Admission V2 Completion sequence {} is not canonical {completion_index}",
                    completion.block_sequence
                ));
            }
            if completion.first_decision_record != covered_decisions {
                return Err(format!(
                    "Admission V2 Completion {completion_index} starts at {}, not contiguous {covered_decisions}",
                    completion.first_decision_record
                ));
            }
            self.require_block_bound(completion.source.decision_count)?;
            let end = covered_decisions
                .checked_add(completion.source.decision_count)
                .ok_or_else(|| "Admission V2 completed decision range overflowed".to_owned())?;
            if end > decision_records {
                return Err(format!(
                    "Admission V2 Completion {completion_index} is torn: ends at decision {end}, file has {decision_records}"
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
                    "Admission V2 block {} appears more than once",
                    hex32(receipt.block_id)
                ));
            }
            covered_decisions = end;
            completion_index = completion_index
                .checked_add(1)
                .ok_or_else(|| "Admission V2 Completion scan overflowed".to_owned())?;
        }
        if covered_decisions < decision_records {
            let trailing_count = decision_records
                .checked_sub(covered_decisions)
                .ok_or_else(|| "Admission V2 trailing decision count underflowed".to_owned())?;
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
                    "Admission V2 trailing block {} duplicates a completed block",
                    hex32(block_id)
                ));
            }
            self.trailing = Some(TrailingDecisionBlockV2 {
                first_decision_record: covered_decisions,
                block_id,
                decisions,
            });
        }
        self.decision_records = decision_records;
        self.completion_records = completion_records;
        self.require_unchanged()
    }

    /// Revalidates both complete bounded files and performs an average-O(1)
    /// block-identity lookup.  Absence is `Ok(None)`.
    ///
    /// # Complexity
    ///
    /// Generation validation is O(file bytes); only the hash-table lookup is
    /// average O(1).
    ///
    /// # Errors
    ///
    /// Refuses a lock, same-length mutation, append, symlink/path replacement,
    /// or any other root/lock/data generation change since open.
    pub fn reopen_structural_receipt(
        &self,
        block_id: &[u8; 32],
    ) -> Result<Option<PopulationAdmissionV2StructuralReceipt>, PopulationAdmissionV2Refusal> {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take shared Admission V2 lookup lock: {why}"))?;
        let result = self
            .require_unchanged()
            .map(|()| self.receipts.get(block_id).copied());
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release shared Admission V2 lookup lock: {why}"));
        match (result, released) {
            (Ok(receipt), Ok(())) => Ok(receipt),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "append remains dormant until opaque Runner search lineage can construct PreparedPopulationAdmissionV2"
        )
    )]
    /// Appends, exactly retries, or exactly reuses one prepared block.
    ///
    /// This performs repeated generation hashing and may rescan the bounded
    /// ledger, so its time is O(total ledger bytes + total ledger records +
    /// block decisions), not O(block) or O(1).
    fn append(
        &mut self,
        prepared: &PreparedPopulationAdmissionV2,
    ) -> Result<PopulationAdmissionV2StructuralCommit, PopulationAdmissionV2Refusal> {
        if !self.writable {
            return Err("Admission V2 ledger was opened read-only".to_owned());
        }
        self.lock_file
            .lock()
            .map_err(|why| format!("cannot take exclusive Admission V2 append lock: {why}"))?;
        let result = self.append_locked(prepared);
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Admission V2 append lock: {why}"));
        match (result, released) {
            (Ok(commit), Ok(())) => Ok(commit),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn append_locked(
        &mut self,
        prepared: &PreparedPopulationAdmissionV2,
    ) -> Result<PopulationAdmissionV2StructuralCommit, PopulationAdmissionV2Refusal> {
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
            .map_err(|why| format!("cannot sync Admission V2 decisions: {why}"))?;
        self.decision_records = self
            .decision_records
            .checked_add(count)
            .ok_or_else(|| "Admission V2 decision count overflowed after append".to_owned())?;
        self.refresh_decision_generation()?;
        self.require_unchanged()?;
        let completion = prepared.expected_completion(self.completion_records, first)?;
        append_raw(&mut self.completion_file, &encode_completion(&completion)?)?;
        self.completion_file
            .sync_data()
            .map_err(|why| format!("cannot sync Admission V2 Completion: {why}"))?;
        sync_directory(&self.root_file, &self.root)?;
        self.completion_records = self
            .completion_records
            .checked_add(1)
            .ok_or_else(|| "Admission V2 Completion count overflowed after append".to_owned())?;
        self.refresh_completion_generation()?;
        self.require_unchanged()?;
        self.scan()?;
        let receipt = self
            .receipts
            .get(&prepared.source.block_id)
            .copied()
            .ok_or_else(|| "Admission V2 appended block was not indexed".to_owned())?;
        Ok(PopulationAdmissionV2StructuralCommit::Written(receipt))
    }

    /// Reuses only exact bytes and reissues all three durability barriers.
    ///
    /// Full-file generation checks make this O(total ledger bytes + block
    /// decisions), not O(1), despite the initial average-O(1) index hit.
    fn reuse_existing(
        &mut self,
        prepared: &PreparedPopulationAdmissionV2,
        existing: PopulationAdmissionV2StructuralReceipt,
    ) -> Result<PopulationAdmissionV2StructuralCommit, PopulationAdmissionV2Refusal> {
        self.require_block_bound(existing.decision_count)?;
        let observed = read_decision_range(
            &mut self.decision_file,
            existing.first_decision_record,
            existing.decision_count,
            self.bounds,
        )?;
        if observed != prepared.decisions {
            return Err(format!(
                "Admission V2 block {} exists with different exact decision bytes",
                hex32(existing.block_id)
            ));
        }
        let completion_raw =
            read_completion_at(&mut self.completion_file, existing.block_sequence)?;
        let observed_completion = decode_completion(&completion_raw)?;
        let expected = prepared
            .expected_completion(existing.block_sequence, existing.first_decision_record)?;
        if observed_completion != expected {
            return Err(format!(
                "Admission V2 block {} exists with different exact Completion bytes",
                hex32(existing.block_id)
            ));
        }
        self.require_unchanged()?;
        self.decision_file
            .sync_data()
            .map_err(|why| format!("cannot sync reused Admission V2 decisions: {why}"))?;
        self.completion_file
            .sync_data()
            .map_err(|why| format!("cannot sync reused Admission V2 Completion: {why}"))?;
        sync_directory(&self.root_file, &self.root)?;
        #[cfg(test)]
        {
            self.reuse_barrier_count = 3;
        }
        self.require_unchanged()?;
        Ok(PopulationAdmissionV2StructuralCommit::Reused(existing))
    }

    /// Completes an exact full decision prefix after reissuing its durability
    /// barrier.
    ///
    /// Generation validation and the post-write rescan make this O(total
    /// ledger bytes + total ledger records + block decisions).
    fn complete_trailing(
        &mut self,
        prepared: &PreparedPopulationAdmissionV2,
        trailing: &TrailingDecisionBlockV2,
    ) -> Result<PopulationAdmissionV2StructuralCommit, PopulationAdmissionV2Refusal> {
        if trailing.block_id != prepared.source.block_id || trailing.decisions != prepared.decisions
        {
            return Err(format!(
                "Admission V2 trailing decision block {} is not the exact retry {}",
                hex32(trailing.block_id),
                hex32(prepared.source.block_id)
            ));
        }
        self.require_append_bound(0, 1)?;
        self.require_unchanged()?;
        self.decision_file
            .sync_data()
            .map_err(|why| format!("cannot sync Admission V2 retry decisions: {why}"))?;
        #[cfg(test)]
        {
            self.trailing_barrier_order_code = 1;
        }
        let completion = prepared
            .expected_completion(self.completion_records, trailing.first_decision_record)?;
        append_raw(&mut self.completion_file, &encode_completion(&completion)?)?;
        self.completion_file
            .sync_data()
            .map_err(|why| format!("cannot sync Admission V2 retry Completion: {why}"))?;
        #[cfg(test)]
        {
            self.trailing_barrier_order_code = self
                .trailing_barrier_order_code
                .saturating_mul(10)
                .saturating_add(2);
        }
        sync_directory(&self.root_file, &self.root)?;
        #[cfg(test)]
        {
            self.trailing_barrier_order_code = self
                .trailing_barrier_order_code
                .saturating_mul(10)
                .saturating_add(3);
        }
        self.completion_records = self
            .completion_records
            .checked_add(1)
            .ok_or_else(|| "Admission V2 retry Completion count overflowed".to_owned())?;
        self.refresh_completion_generation()?;
        self.require_unchanged()?;
        self.scan()?;
        let receipt = self
            .receipts
            .get(&prepared.source.block_id)
            .copied()
            .ok_or_else(|| "Admission V2 retried block was not indexed".to_owned())?;
        Ok(PopulationAdmissionV2StructuralCommit::Written(receipt))
    }

    fn require_prepared_bound(
        &self,
        prepared: &PreparedPopulationAdmissionV2,
    ) -> Result<u64, PopulationAdmissionV2Refusal> {
        let count = prepared.checked_decision_count()?;
        self.require_block_bound(count)?;
        Ok(count)
    }

    fn require_block_bound(&self, count: u64) -> Result<(), PopulationAdmissionV2Refusal> {
        if count == 0 {
            return Err("Admission V2 block has zero decisions".to_owned());
        }
        if count > self.bounds.decisions_per_block {
            return Err(format!(
                "Admission V2 block has {count} decisions, above explicit per-block maximum {}",
                self.bounds.decisions_per_block
            ));
        }
        let bytes = count
            .checked_mul(POPULATION_ADMISSION_V2_DECISION_BYTES as u64)
            .ok_or_else(|| "Admission V2 block decision bytes overflowed".to_owned())?;
        if bytes > self.bounds.decision_bytes {
            return Err(format!(
                "Admission V2 block requires {bytes} decision bytes, above explicit maximum {}",
                self.bounds.decision_bytes
            ));
        }
        Ok(())
    }

    fn require_append_bound(
        &self,
        decisions: u64,
        completions: u64,
    ) -> Result<(), PopulationAdmissionV2Refusal> {
        let next_decisions = self
            .decision_records
            .checked_add(decisions)
            .ok_or_else(|| "Admission V2 append decision count overflowed".to_owned())?;
        if next_decisions > self.bounds.decision_records {
            return Err(format!(
                "Admission V2 append reaches {next_decisions} decisions, above explicit maximum {}",
                self.bounds.decision_records
            ));
        }
        let next_decision_bytes = next_decisions
            .checked_mul(POPULATION_ADMISSION_V2_DECISION_BYTES as u64)
            .ok_or_else(|| "Admission V2 append decision bytes overflowed".to_owned())?;
        if next_decision_bytes > self.bounds.decision_bytes {
            return Err(format!(
                "Admission V2 append reaches {next_decision_bytes} decision bytes, above explicit maximum {}",
                self.bounds.decision_bytes
            ));
        }
        let next_completions = self
            .completion_records
            .checked_add(completions)
            .ok_or_else(|| "Admission V2 append Completion count overflowed".to_owned())?;
        if next_completions > self.bounds.completion_records {
            return Err(format!(
                "Admission V2 append reaches {next_completions} Completions, above explicit maximum {}",
                self.bounds.completion_records
            ));
        }
        let next_completion_bytes = next_completions
            .checked_mul(POPULATION_ADMISSION_V2_COMPLETION_BYTES as u64)
            .ok_or_else(|| "Admission V2 append Completion bytes overflowed".to_owned())?;
        if next_completion_bytes > self.bounds.completion_bytes {
            return Err(format!(
                "Admission V2 append reaches {next_completion_bytes} Completion bytes, above explicit maximum {}",
                self.bounds.completion_bytes
            ));
        }
        Ok(())
    }

    fn refresh_decision_generation(&mut self) -> Result<(), PopulationAdmissionV2Refusal> {
        self.decision_generation = file_generation(
            &self.decision_file,
            &self.decision_path,
            self.bounds.decision_bytes,
        )?;
        Ok(())
    }

    fn refresh_completion_generation(&mut self) -> Result<(), PopulationAdmissionV2Refusal> {
        self.completion_generation = file_generation(
            &self.completion_file,
            &self.completion_path,
            self.bounds.completion_bytes,
        )?;
        Ok(())
    }

    fn require_unchanged(&self) -> Result<(), PopulationAdmissionV2Refusal> {
        if named_root_identity(&self.root)? != self.root_identity
            || PlatformIdentity::of(
                &self
                    .root_file
                    .metadata()
                    .map_err(|why| format!("cannot stat held Admission V2 root: {why}"))?,
            ) != self.root_identity
        {
            return Err("Admission V2 root was replaced after open".to_owned());
        }
        let lock = file_generation(
            &self.lock_file,
            &self.lock_path,
            self.bounds.completion_bytes,
        )?;
        let decisions = file_generation(
            &self.decision_file,
            &self.decision_path,
            self.bounds.decision_bytes,
        )?;
        let completions = file_generation(
            &self.completion_file,
            &self.completion_path,
            self.bounds.completion_bytes,
        )?;
        if lock != self.lock_generation {
            return Err("Admission V2 lock generation changed after open".to_owned());
        }
        if decisions != self.decision_generation {
            return Err("Admission V2 decision generation changed after open".to_owned());
        }
        if completions != self.completion_generation {
            return Err("Admission V2 Completion generation changed after open".to_owned());
        }
        Ok(())
    }

    /// Promotes a structural receipt only after exact opaque-preparation join.
    ///
    /// Full-file generation validation plus block reads make this O(total
    /// ledger bytes + block decisions), not O(1).
    fn authenticate_structural_receipt(
        &mut self,
        receipt: PopulationAdmissionV2StructuralReceipt,
        prepared: &PreparedPopulationAdmissionV2,
    ) -> Result<PopulationAdmissionV2Authority, PopulationAdmissionV2Refusal> {
        let count = self.require_prepared_bound(prepared)?;
        if count != receipt.decision_count || prepared.source.block_id != receipt.block_id {
            return Err(
                "Admission V2 structural receipt does not match opaque preparation bounds/identity"
                    .to_owned(),
            );
        }
        self.require_unchanged()?;
        if self.receipts.get(&receipt.block_id) != Some(&receipt) {
            return Err(
                "Admission V2 structural receipt is not the indexed reopened receipt".to_owned(),
            );
        }
        let decisions = read_decision_range(
            &mut self.decision_file,
            receipt.first_decision_record,
            receipt.decision_count,
            self.bounds,
        )?;
        if decisions != prepared.decisions {
            return Err(
                "Admission V2 structural decisions differ from opaque preparation".to_owned(),
            );
        }
        let observed = decode_completion(&read_completion_at(
            &mut self.completion_file,
            receipt.block_sequence,
        )?)?;
        let expected =
            prepared.expected_completion(receipt.block_sequence, receipt.first_decision_record)?;
        if observed != expected {
            return Err(
                "Admission V2 structural Completion differs from opaque preparation".to_owned(),
            );
        }
        self.require_unchanged()?;
        Ok(PopulationAdmissionV2Authority { receipt })
    }
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "persistence remains dormant until opaque Runner search lineage can construct PreparedPopulationAdmissionV2"
    )
)]
/// Persists, drops the writer, freshly reopens, and authenticates the receipt.
///
/// Its full validation passes cost O(total ledger bytes + total ledger records
/// + block decisions); repeated cumulative calls can be quadratic overall.
fn persist_population_admission_v2(
    root: &Path,
    bounds: AdmissionV2Bounds,
    prepared: &PreparedPopulationAdmissionV2,
) -> Result<PopulationAdmissionV2Commit, PopulationAdmissionV2Refusal> {
    let mut writer = PopulationAdmissionV2Ledger::open_write(root, bounds)?;
    let structural = writer.append(prepared)?;
    let was_written = matches!(
        structural,
        PopulationAdmissionV2StructuralCommit::Written(_)
    );
    let receipt = structural.receipt();
    drop(writer);
    let mut reopened = PopulationAdmissionV2Ledger::open_read(root, bounds)?;
    let observed = reopened
        .reopen_structural_receipt(&receipt.block_id)?
        .ok_or_else(|| "Admission V2 fresh reopen omitted persisted receipt".to_owned())?;
    if observed != receipt {
        return Err("Admission V2 fresh reopen changed structural receipt".to_owned());
    }
    let authority = reopened.authenticate_structural_receipt(observed, prepared)?;
    Ok(if was_written {
        PopulationAdmissionV2Commit::Written(authority)
    } else {
        PopulationAdmissionV2Commit::Reused(authority)
    })
}

fn validate_complete_block(
    decisions: &[AdmissionDecisionRecordV2],
    completion: &AdmissionCompletionRecordV2,
) -> Result<PopulationAdmissionV2StructuralReceipt, PopulationAdmissionV2Refusal> {
    let prepared = PreparedPopulationAdmissionV2 {
        source: completion.source.clone(),
        decisions: decisions.to_vec(),
    };
    prepared.validate()?;
    let expected = prepared
        .expected_completion(completion.block_sequence, completion.first_decision_record)?;
    if expected != *completion {
        return Err(
            "Admission V2 Completion does not exactly complete its ordered decisions".to_owned(),
        );
    }
    Ok(PopulationAdmissionV2StructuralReceipt {
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
    decisions: &[AdmissionDecisionRecordV2],
) -> Result<[u8; 32], PopulationAdmissionV2Refusal> {
    let first = decisions
        .first()
        .ok_or_else(|| "Admission V2 trailing block is empty".to_owned())?;
    let block_id = first.block_id;
    let mut bank_started = false;
    let mut bank_sequence = 0_u64;
    let mut semantics = HashSet::new();
    let mut identities = HashSet::new();
    semantics
        .try_reserve(decisions.len())
        .map_err(|why| format!("cannot reserve Admission V2 trailing semantic index: {why}"))?;
    identities
        .try_reserve(decisions.len())
        .map_err(|why| format!("cannot reserve Admission V2 trailing identity index: {why}"))?;
    for (index, decision) in decisions.iter().enumerate() {
        decision.validate()?;
        let global = u64::try_from(index)
            .map_err(|_| "Admission V2 trailing index does not fit u64".to_owned())?;
        if decision.block_id != block_id || decision.global_sequence != global {
            return Err(format!(
                "Admission V2 trailing decision {index} changes block or global order"
            ));
        }
        match decision.family {
            AdmissionV2Family::Nifty if bank_started => {
                return Err(
                    "Admission V2 trailing block returns to NIFTY after BANKNIFTY".to_owned(),
                );
            }
            AdmissionV2Family::Nifty => {
                if decision.family_sequence != global {
                    return Err(format!(
                        "Admission V2 trailing NIFTY sequence {} is not {global}",
                        decision.family_sequence
                    ));
                }
            }
            AdmissionV2Family::BankNifty => {
                bank_started = true;
                if decision.family_sequence != bank_sequence {
                    return Err(format!(
                        "Admission V2 trailing BANKNIFTY sequence {} is not {bank_sequence}",
                        decision.family_sequence
                    ));
                }
                bank_sequence = bank_sequence.checked_add(1).ok_or_else(|| {
                    "Admission V2 trailing BANKNIFTY sequence overflowed".to_owned()
                })?;
            }
        }
        if !semantics.insert(decision.candidate_semantic_id) {
            return Err("Admission V2 trailing candidate semantic is duplicated".to_owned());
        }
        if !identities.insert(decision.decision_id) {
            return Err("Admission V2 trailing decision identity is duplicated".to_owned());
        }
    }
    Ok(block_id)
}

fn checked_record_count(
    bytes: u64,
    stride: usize,
    max_records: u64,
    name: &str,
) -> Result<u64, PopulationAdmissionV2Refusal> {
    let stride_u64 = u64::try_from(stride)
        .map_err(|_| format!("Admission V2 {name} stride does not fit u64"))?;
    if !bytes.is_multiple_of(stride_u64) {
        return Err(format!(
            "Admission V2 {name} file has ragged length {bytes}, not a multiple of {stride}"
        ));
    }
    let records = bytes / stride_u64;
    if records > max_records {
        return Err(format!(
            "Admission V2 {name} file has {records} records, above explicit maximum {max_records}"
        ));
    }
    Ok(records)
}

fn read_decision_range(
    file: &mut File,
    first: u64,
    count: u64,
    bounds: AdmissionV2Bounds,
) -> Result<Vec<AdmissionDecisionRecordV2>, PopulationAdmissionV2Refusal> {
    if count == 0 || count > bounds.decisions_per_block {
        return Err(format!(
            "Admission V2 bounded read count {count} is outside 1..={} ",
            bounds.decisions_per_block
        ));
    }
    let end = first
        .checked_add(count)
        .ok_or_else(|| "Admission V2 decision read range overflowed".to_owned())?;
    if end > bounds.decision_records {
        return Err(format!(
            "Admission V2 decision read ends at {end}, above explicit record maximum {}",
            bounds.decision_records
        ));
    }
    let count_usize = usize::try_from(count)
        .map_err(|_| "Admission V2 decision read count does not fit usize".to_owned())?;
    let mut decisions = Vec::new();
    decisions
        .try_reserve_exact(count_usize)
        .map_err(|why| format!("cannot reserve Admission V2 bounded decision read: {why}"))?;
    let mut physical = first;
    while physical < end {
        let raw = read_fixed_at::<POPULATION_ADMISSION_V2_DECISION_BYTES>(
            file,
            physical,
            POPULATION_ADMISSION_V2_DECISION_BYTES,
            "decision",
        )?;
        decisions.push(decode_decision(&raw)?);
        physical = physical
            .checked_add(1)
            .ok_or_else(|| "Admission V2 decision read cursor overflowed".to_owned())?;
    }
    Ok(decisions)
}

fn read_completion_at(
    file: &mut File,
    physical: u64,
) -> Result<[u8; POPULATION_ADMISSION_V2_COMPLETION_BYTES], PopulationAdmissionV2Refusal> {
    read_fixed_at::<POPULATION_ADMISSION_V2_COMPLETION_BYTES>(
        file,
        physical,
        POPULATION_ADMISSION_V2_COMPLETION_BYTES,
        "Completion",
    )
}

fn read_fixed_at<const N: usize>(
    file: &mut File,
    physical: u64,
    stride: usize,
    name: &str,
) -> Result<[u8; N], PopulationAdmissionV2Refusal> {
    let stride_u64 = u64::try_from(stride)
        .map_err(|_| format!("Admission V2 {name} stride does not fit u64"))?;
    let offset = physical
        .checked_mul(stride_u64)
        .ok_or_else(|| format!("Admission V2 {name} offset overflowed"))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|why| format!("cannot seek Admission V2 {name} record {physical}: {why}"))?;
    let mut raw = [0_u8; N];
    file.read_exact(&mut raw)
        .map_err(|why| format!("cannot read Admission V2 {name} record {physical}: {why}"))?;
    Ok(raw)
}

fn append_raw(file: &mut File, raw: &[u8]) -> Result<(), PopulationAdmissionV2Refusal> {
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(raw))
        .map_err(|why| format!("cannot append Admission V2 fixed record: {why}"))
}

fn open_root_directory(
    root: &Path,
) -> Result<(PathBuf, File, PlatformIdentity), PopulationAdmissionV2Refusal> {
    require_not_symlink(root, false)?;
    let canonical = std::fs::canonicalize(root).map_err(|why| {
        format!(
            "Admission V2 root {} must already exist: {why}",
            root.display()
        )
    })?;
    require_not_symlink(&canonical, false)?;
    let file = File::open(&canonical).map_err(|why| {
        format!(
            "cannot hold Admission V2 root {}: {why}",
            canonical.display()
        )
    })?;
    let metadata = file.metadata().map_err(|why| {
        format!(
            "cannot stat held Admission V2 root {}: {why}",
            canonical.display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "Admission V2 root {} is not a directory",
            canonical.display()
        ));
    }
    Ok((canonical, file, PlatformIdentity::of(&metadata)))
}

fn named_root_identity(root: &Path) -> Result<PlatformIdentity, PopulationAdmissionV2Refusal> {
    require_not_symlink(root, false)?;
    let file = File::open(root).map_err(|why| {
        format!(
            "cannot reopen named Admission V2 root {}: {why}",
            root.display()
        )
    })?;
    let metadata = file.metadata().map_err(|why| {
        format!(
            "cannot stat named Admission V2 root {}: {why}",
            root.display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "named Admission V2 root {} is not a directory",
            root.display()
        ));
    }
    Ok(PlatformIdentity::of(&metadata))
}

fn open_child(
    path: &Path,
    writable: bool,
    create: bool,
) -> Result<(File, bool), PopulationAdmissionV2Refusal> {
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
                    "cannot create Admission V2 file {}: {why}",
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
        .map_err(|why| format!("cannot open Admission V2 file {}: {why}", path.display()))?;
    require_not_symlink(path, false)?;
    require_regular_file(&file, path)?;
    Ok((file, false))
}

fn require_regular_file(file: &File, path: &Path) -> Result<(), PopulationAdmissionV2Refusal> {
    if !file
        .metadata()
        .map_err(|why| format!("cannot stat Admission V2 file {}: {why}", path.display()))?
        .is_file()
    {
        return Err(format!(
            "Admission V2 path {} is not a regular file",
            path.display()
        ));
    }
    Ok(())
}

fn require_not_symlink(
    path: &Path,
    absent_is_allowed: bool,
) -> Result<(), PopulationAdmissionV2Refusal> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(format!(
            "Admission V2 path {} is a symbolic link; no-follow admission refused it",
            path.display()
        )),
        Ok(_) => Ok(()),
        Err(why) if absent_is_allowed && why.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(why) => Err(format!(
            "cannot inspect Admission V2 path {} without following links: {why}",
            path.display()
        )),
    }
}

fn sync_directory(root_file: &File, root: &Path) -> Result<(), PopulationAdmissionV2Refusal> {
    root_file.sync_all().map_err(|why| {
        format!(
            "cannot durably sync Admission V2 directory entries in {}: {why}",
            root.display()
        )
    })
}

fn file_generation(
    file: &File,
    path: &Path,
    max_bytes: u64,
) -> Result<FileGeneration, PopulationAdmissionV2Refusal> {
    let before = file.metadata().map_err(|why| {
        format!(
            "cannot stat held Admission V2 file {}: {why}",
            path.display()
        )
    })?;
    if !before.is_file() {
        return Err(format!(
            "held Admission V2 path {} is not a regular file",
            path.display()
        ));
    }
    if before.len() > max_bytes {
        return Err(format!(
            "Admission V2 file {} has {} bytes, above explicit maximum {max_bytes}",
            path.display(),
            before.len()
        ));
    }
    let identity = PlatformIdentity::of(&before);
    let (named, _) = open_child(path, false, false)?;
    let named_metadata = named.metadata().map_err(|why| {
        format!(
            "cannot stat named Admission V2 file {}: {why}",
            path.display()
        )
    })?;
    if PlatformIdentity::of(&named_metadata) != identity {
        return Err(format!(
            "Admission V2 file {} was path-replaced after open",
            path.display()
        ));
    }
    let digest = hash_file(file, path, before.len(), max_bytes)?;
    let after = file.metadata().map_err(|why| {
        format!(
            "cannot restat held Admission V2 file {}: {why}",
            path.display()
        )
    })?;
    if PlatformIdentity::of(&after) != identity || after.len() != before.len() {
        return Err(format!(
            "Admission V2 file {} changed while its generation was hashed",
            path.display()
        ));
    }
    Ok(FileGeneration {
        identity,
        len: before.len(),
        digest,
    })
}

fn hash_file(
    file: &File,
    path: &Path,
    expected_len: u64,
    max_bytes: u64,
) -> Result<[u8; 32], PopulationAdmissionV2Refusal> {
    let mut reader = file
        .try_clone()
        .map_err(|why| format!("cannot clone Admission V2 file {}: {why}", path.display()))?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|why| format!("cannot seek Admission V2 file {}: {why}", path.display()))?;
    let mut hasher = Hasher::new();
    hasher.update(FILE_GENERATION_DOMAIN);
    hasher.update(&expected_len.to_le_bytes());
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    let mut total = 0_u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|why| format!("cannot hash Admission V2 file {}: {why}", path.display()))?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(
                u64::try_from(read)
                    .map_err(|_| "Admission V2 generation read does not fit u64".to_owned())?,
            )
            .ok_or_else(|| "Admission V2 generation byte count overflowed".to_owned())?;
        if total > max_bytes {
            return Err(format!(
                "Admission V2 file {} exceeded {max_bytes} bytes while hashing",
                path.display()
            ));
        }
        hasher.update(
            buffer
                .get(..read)
                .ok_or_else(|| "Admission V2 generation read escaped buffer".to_owned())?,
        );
    }
    if total != expected_len {
        return Err(format!(
            "Admission V2 file {} changed length from {expected_len} to {total} while hashing",
            path.display()
        ));
    }
    Ok(hasher.finalize())
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "focused adversarial tests intentionally stop on fixture setup failure and mutate exact fixed-format bytes"
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
                "brutex-population-admission-v2-{}-{label}-{sequence}",
                std::process::id()
            ));
            std::fs::create_dir(&path).expect("create isolated Admission V2 test root");
            Self { path }
        }

        fn absent(label: &str) -> Self {
            let sequence = NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed);
            Self {
                path: std::env::temp_dir().join(format!(
                    "brutex-population-admission-v2-absent-{}-{label}-{sequence}",
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
                std::fs::remove_dir_all(&self.path)
                    .expect("remove isolated Admission V2 test root");
            }
        }
    }

    fn digest(seed: u8) -> [u8; 32] {
        hash_slices(b"admission-v2-test-digest\0", &[&[seed]])
    }

    fn family(seed: u8, count: u64) -> FamilySourceV2 {
        FamilySourceV2 {
            candidate_universe_id: digest(seed),
            candidate_completion_digest: digest(seed.wrapping_add(1)),
            pre_admission_authority_id: digest(seed.wrapping_add(2)),
            candidate_count: count,
        }
    }

    fn observation(count: u64) -> ObservationSourceV2 {
        ObservationSourceV2 {
            audit_id: digest(20),
            completion_digest: digest(21),
            pair_digest: digest(22),
            source_digest: digest(23),
            policy_digest: digest(24),
            layout_digest: digest(25),
            candidate_count: count,
            session_count: 40,
        }
    }

    fn statistics(count: u64) -> StatisticsSourceV2 {
        StatisticsSourceV2 {
            audit_id: digest(30),
            completion_digest: digest(31),
            observation_audit_id: digest(20),
            observation_completion_digest: digest(21),
            observation_link_digest: digest(32),
            projection_digest: digest(33),
            wilson_policy_digest: digest(34),
            cscv_policy_digest: digest(35),
            split_family_digest: digest(36),
            ordered_candidate_digest: digest(37),
            ordered_period_digest: digest(38),
            ordered_split_digest: digest(39),
            white_policy_digest: digest(40),
            spa_policy_digest: digest(41),
            romano_wolf_policy_digest: digest(42),
            draw_policy_digest: digest(43),
            seed_digest: digest(44),
            block_policy_digest: digest(45),
            candidate_count: count,
            period_count: 40,
            split_count: 8,
            draw_count: 200,
            block_length: 5,
        }
    }

    fn search_member(seed: u8, count: u64) -> SearchMemberV2 {
        SearchMemberV2 {
            validation_policy_id: digest(seed),
            validation_family_id: digest(seed.wrapping_add(1)),
            walk_facts_id: digest(seed.wrapping_add(2)),
            anchored_authority_id: digest(seed.wrapping_add(3)),
            candidate_count: count,
            considered_count: count,
            priced_count: count,
            scored_count: count,
            decided_count: count,
        }
    }

    fn source() -> BlockSourceV2 {
        let policy = [0x5a; ADMISSION_V2_POLICY_BYTES];
        let mut value = BlockSourceV2 {
            signal_rung: 3,
            horizon_bars: 15,
            requested_span: RequestedSpanV2 {
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
            exit_grid_policy_id: digest(7),
            ranking_policy_id: digest(8),
            admission_source_policy_id: digest(9),
            stored_source_policy_id: digest(10),
            nifty: family(11, 2),
            banknifty: family(14, 2),
            observation: observation(4),
            statistics: statistics(4),
            search: SearchSourceV2 {
                population_search_id: digest(50),
                ranking_policy_id: digest(8),
                source_policy_id: digest(10),
                finalization_family_id: digest(51),
                nifty: search_member(52, 2),
                banknifty: search_member(60, 2),
            },
            policy,
            policy_digest: hash_slices(POLICY_DIGEST_DOMAIN, &[&policy]),
            nifty_decision_count: 2,
            banknifty_decision_count: 2,
            decision_count: 4,
            block_id: [0; 32],
        };
        value.block_id = value.derive_block_id().expect("derive fixture block ID");
        value
    }

    fn decision(source: &BlockSourceV2, global_sequence: u64) -> AdmissionDecisionRecordV2 {
        let (family, family_sequence, pre_admission, search_member) = if global_sequence < 2 {
            (
                AdmissionV2Family::Nifty,
                global_sequence,
                source.nifty.pre_admission_authority_id,
                source.search.nifty.anchored_authority_id,
            )
        } else {
            (
                AdmissionV2Family::BankNifty,
                global_sequence - 2,
                source.banknifty.pre_admission_authority_id,
                source.search.banknifty.anchored_authority_id,
            )
        };
        let seed = u8::try_from(global_sequence + 70).expect("fixture seed fits u8");
        let runner_decision = [seed; RUNNER_ADMISSION_V2_DECISION_BYTES];
        let mut value = AdmissionDecisionRecordV2 {
            block_id: source.block_id,
            global_sequence,
            family,
            family_sequence,
            status: match global_sequence {
                0 => AdmissionV2Status::Admitted,
                1 => AdmissionV2Status::Rejected,
                2 => AdmissionV2Status::Unmeasured,
                _ => AdmissionV2Status::Refused,
            },
            candidate_semantic_id: digest(seed),
            candidate_row_digest: digest(seed.wrapping_add(1)),
            pre_admission_authority_id: pre_admission,
            statistics_period_digest: digest(seed.wrapping_add(2)),
            statistics_split_digest: digest(seed.wrapping_add(3)),
            family_search_member_id: search_member,
            base_evidence_source_digest: digest(seed.wrapping_add(4)),
            runner_decision,
            runner_decision_digest: hash_slices(RUNNER_DECISION_DIGEST_DOMAIN, &[&runner_decision]),
            evidence_digest: digest(seed.wrapping_add(5)),
            verdict_digest: digest(seed.wrapping_add(6)),
            decision_id: [0; 32],
        };
        value.decision_id = value.derive_decision_id();
        value
    }

    fn prepared() -> PreparedPopulationAdmissionV2 {
        let source = source();
        let decisions = (0_u64..4)
            .map(|sequence| decision(&source, sequence))
            .collect();
        PreparedPopulationAdmissionV2 { source, decisions }
    }

    fn bounds() -> AdmissionV2Bounds {
        AdmissionV2Bounds::new(
            32,
            32 * POPULATION_ADMISSION_V2_DECISION_BYTES as u64,
            8,
            8 * POPULATION_ADMISSION_V2_COMPLETION_BYTES as u64,
            8,
        )
        .expect("valid fixture bounds")
    }

    fn initialize_empty(root: &Path) {
        drop(
            PopulationAdmissionV2Ledger::open_write(root, bounds())
                .expect("initialize empty Admission V2 ledger"),
        );
    }

    fn write_decisions(root: &Path, decisions: &[AdmissionDecisionRecordV2]) {
        let path = root.join(DECISION_FILE);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("open test decision file");
        for value in decisions {
            append_raw(
                &mut file,
                &encode_decision(value).expect("encode test decision"),
            )
            .expect("append test decision");
        }
        file.sync_all().expect("sync test decisions");
    }

    fn write_completion(root: &Path, completion: &AdmissionCompletionRecordV2) {
        let path = root.join(COMPLETION_FILE);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("open test Completion file");
        append_raw(
            &mut file,
            &encode_completion(completion).expect("encode test Completion"),
        )
        .expect("append test Completion");
        file.sync_all().expect("sync test Completion");
    }

    fn assert_refuses<T>(result: Result<T, PopulationAdmissionV2Refusal>, needle: &str) {
        let error = result.err().expect("operation must refuse");
        assert!(
            error.contains(needle),
            "expected refusal containing {needle:?}, got {error:?}"
        );
    }

    #[test]
    fn bounds_are_explicit_and_cover_all_five_axes() {
        assert_refuses(AdmissionV2Bounds::new(0, 1, 1, 4096, 1), "nonzero");
        assert_refuses(AdmissionV2Bounds::new(2, 2048, 1, 4096, 1), "cannot hold");
        assert_refuses(AdmissionV2Bounds::new(2, 4096, 1, 4096, 3), "exceeds total");
        assert_refuses(AdmissionV2Bounds::new(2, 4096, 2, 4096, 1), "cannot hold");
    }

    #[test]
    fn fixed_codecs_round_trip_and_reserves_are_zero() {
        let value = prepared();
        value.validate().expect("valid preparation");
        let decision_raw = encode_decision(&value.decisions[0]).expect("encode decision");
        assert_eq!(decision_raw.len(), POPULATION_ADMISSION_V2_DECISION_BYTES);
        assert!(decode_decision(&decision_raw).expect("decode decision") == value.decisions[0]);
        let completion = value.expected_completion(7, 11).expect("derive Completion");
        let completion_raw = encode_completion(&completion).expect("encode Completion");
        assert_eq!(
            completion_raw.len(),
            POPULATION_ADMISSION_V2_COMPLETION_BYTES
        );
        assert!(decode_completion(&completion_raw).expect("decode Completion") == completion);

        let mut bad_decision = decision_raw;
        bad_decision[65] = 1;
        let seal = hash_slices(
            DECISION_SEAL_DOMAIN,
            &[&bad_decision[..DECISION_PAYLOAD_BYTES]],
        );
        bad_decision[DECISION_PAYLOAD_BYTES..].copy_from_slice(&seal);
        assert_refuses(decode_decision(&bad_decision), "reserve");
    }

    #[test]
    fn canonical_order_crosswire_duplicate_and_every_identity_rekey_refuse() {
        let mut reordered = prepared();
        reordered.decisions.swap(1, 2);
        assert_refuses(reordered.validate(), "NIFTY-first");

        let mut duplicate = prepared();
        duplicate.decisions[1].candidate_semantic_id = duplicate.decisions[0].candidate_semantic_id;
        duplicate.decisions[1].decision_id = duplicate.decisions[1].derive_decision_id();
        assert_refuses(duplicate.validate(), "appears more than once");

        let original = source();
        for changed in [
            ("feed", digest(101)),
            ("commit", digest(102)),
            ("calendar", digest(103)),
            ("daily", digest(104)),
            ("vocabulary", digest(105)),
            ("evaluation", digest(106)),
            ("exit", digest(107)),
            ("ranking", digest(108)),
            ("admission", digest(109)),
            ("stored", digest(110)),
        ] {
            let mut altered = original.clone();
            match changed.0 {
                "feed" => altered.feed_id = changed.1,
                "commit" => altered.source_commit_id = changed.1,
                "calendar" => altered.calendar_policy_id = changed.1,
                "daily" => altered.daily_policy_id = changed.1,
                "vocabulary" => altered.vocabulary_id = changed.1,
                "evaluation" => altered.evaluation_policy_id = changed.1,
                "exit" => altered.exit_grid_policy_id = changed.1,
                "ranking" => {
                    altered.ranking_policy_id = changed.1;
                    altered.search.ranking_policy_id = changed.1;
                }
                "admission" => altered.admission_source_policy_id = changed.1,
                "stored" => {
                    altered.stored_source_policy_id = changed.1;
                    altered.search.source_policy_id = changed.1;
                }
                _ => panic!("unknown test field"),
            }
            let rekeyed = altered.derive_block_id().expect("derive altered block");
            assert_ne!(rekeyed, original.block_id, "{} must rekey", changed.0);
        }
    }

    #[test]
    fn instrument_bearing_family_aliases_refuse_but_content_hashes_may_match() {
        for field in [
            "candidate universe",
            "candidate completion",
            "PreAdmission authority",
            "anchored authority",
        ] {
            let mut aliased = source();
            match field {
                "candidate universe" => {
                    aliased.banknifty.candidate_universe_id = aliased.nifty.candidate_universe_id;
                }
                "candidate completion" => {
                    aliased.banknifty.candidate_completion_digest =
                        aliased.nifty.candidate_completion_digest;
                }
                "PreAdmission authority" => {
                    aliased.banknifty.pre_admission_authority_id =
                        aliased.nifty.pre_admission_authority_id;
                }
                "anchored authority" => {
                    aliased.search.banknifty.anchored_authority_id =
                        aliased.search.nifty.anchored_authority_id;
                }
                _ => panic!("unknown family alias field"),
            }
            assert_refuses(aliased.validate(), "alias");
        }

        let mut shareable = source();
        shareable.search.banknifty.validation_policy_id =
            shareable.search.nifty.validation_policy_id;
        shareable.search.banknifty.validation_family_id =
            shareable.search.nifty.validation_family_id;
        shareable.search.banknifty.walk_facts_id = shareable.search.nifty.walk_facts_id;
        shareable.block_id = shareable
            .derive_block_id()
            .expect("derive source with legitimate shared content hashes");
        shareable
            .validate()
            .expect("content-addressed policy/family/walk hashes may legitimately match");
    }

    #[test]
    fn persist_fresh_reopen_and_exact_reuse_promote_only_opaque_match() {
        let root = TestRoot::new("persist");
        let value = prepared();
        let first = persist_population_admission_v2(root.path(), bounds(), &value)
            .expect("persist Admission V2");
        let PopulationAdmissionV2Commit::Written(authority) = first else {
            panic!("first commit must be Written");
        };
        let receipt = authority.structural_receipt();
        assert_eq!(receipt.block_id(), value.source.block_id);
        assert_eq!(receipt.decision_count(), 4);

        let retry = persist_population_admission_v2(root.path(), bounds(), &value)
            .expect("reuse exact Admission V2");
        let PopulationAdmissionV2Commit::Reused(reused) = retry else {
            panic!("second commit must be Reused");
        };
        assert_eq!(reused.structural_receipt(), receipt);
        let reopened = PopulationAdmissionV2Ledger::open_read(root.path(), bounds())
            .expect("fresh structural reopen");
        assert_eq!(
            reopened
                .reopen_structural_receipt(&receipt.block_id())
                .expect("lookup structural receipt"),
            Some(receipt)
        );
    }

    #[test]
    fn exact_reuse_executes_both_file_and_directory_durability_barriers() {
        let root = TestRoot::new("reuse-barriers");
        let value = prepared();
        let mut ledger = PopulationAdmissionV2Ledger::open_write(root.path(), bounds())
            .expect("open reuse-barrier writer");
        assert!(matches!(
            ledger.append(&value).expect("write initial block"),
            PopulationAdmissionV2StructuralCommit::Written(_)
        ));
        assert_eq!(ledger.reuse_barrier_count, 0);
        assert!(matches!(
            ledger.append(&value).expect("reuse exact block"),
            PopulationAdmissionV2StructuralCommit::Reused(_)
        ));
        assert_eq!(ledger.reuse_barrier_count, 3);
    }

    #[test]
    fn exact_orphan_retry_reissues_ordered_durability_barriers() {
        let root = TestRoot::new("orphan-barriers");
        initialize_empty(root.path());
        let value = prepared();
        write_decisions(root.path(), &value.decisions);

        let mut ledger = PopulationAdmissionV2Ledger::open_write(root.path(), bounds())
            .expect("open exact-orphan retry writer");
        assert!(matches!(
            ledger.append(&value).expect("complete exact orphan"),
            PopulationAdmissionV2StructuralCommit::Written(_)
        ));
        assert_eq!(
            ledger.trailing_barrier_order_code, 123,
            "exact retry must sync decisions, then Completion, then directory"
        );
    }

    #[test]
    fn orphan_decisions_accept_only_exact_retry_then_completion() {
        let root = TestRoot::new("orphan");
        initialize_empty(root.path());
        let value = prepared();
        write_decisions(root.path(), &value.decisions);
        let open = PopulationAdmissionV2Ledger::open_read(root.path(), bounds())
            .expect("orphan prefix is structurally admitted");
        assert!(open.trailing.is_some());
        drop(open);

        let committed = persist_population_admission_v2(root.path(), bounds(), &value)
            .expect("complete exact orphan retry");
        assert!(matches!(committed, PopulationAdmissionV2Commit::Written(_)));

        let other_root = TestRoot::new("foreign-orphan");
        initialize_empty(other_root.path());
        write_decisions(other_root.path(), &value.decisions);
        let mut foreign = prepared();
        foreign.decisions[0].candidate_row_digest = digest(201);
        foreign.decisions[0].decision_id = foreign.decisions[0].derive_decision_id();
        assert_refuses(
            persist_population_admission_v2(other_root.path(), bounds(), &foreign),
            "exact retry",
        );

        let partial_root = TestRoot::new("partial-orphan");
        initialize_empty(partial_root.path());
        write_decisions(partial_root.path(), &value.decisions[..2]);
        assert_refuses(
            persist_population_admission_v2(partial_root.path(), bounds(), &value),
            "exact retry",
        );
    }

    #[test]
    fn ragged_torn_and_outer_seal_corruption_refuse() {
        let ragged = TestRoot::new("ragged");
        initialize_empty(ragged.path());
        let mut ragged_file = OpenOptions::new()
            .append(true)
            .open(ragged.path().join(DECISION_FILE))
            .expect("open ragged decision file");
        ragged_file.write_all(&[1]).expect("append ragged byte");
        ragged_file.sync_all().expect("sync ragged byte");
        assert_refuses(
            PopulationAdmissionV2Ledger::open_read(ragged.path(), bounds()),
            "ragged",
        );

        let corrupt = TestRoot::new("corrupt");
        let value = prepared();
        persist_population_admission_v2(corrupt.path(), bounds(), &value)
            .expect("persist corrupt fixture");
        let decision_path = corrupt.path().join(DECISION_FILE);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&decision_path)
            .expect("open corrupt decision");
        file.seek(SeekFrom::Start(100)).expect("seek corrupt byte");
        file.write_all(&[0xff]).expect("write corrupt byte");
        file.sync_all().expect("sync corrupt byte");
        assert_refuses(
            PopulationAdmissionV2Ledger::open_read(corrupt.path(), bounds()),
            "seal mismatch",
        );

        let torn = TestRoot::new("torn");
        persist_population_admission_v2(torn.path(), bounds(), &value)
            .expect("persist torn fixture");
        OpenOptions::new()
            .write(true)
            .open(torn.path().join(DECISION_FILE))
            .expect("open torn file")
            .set_len(3 * POPULATION_ADMISSION_V2_DECISION_BYTES as u64)
            .expect("truncate one decision");
        assert_refuses(
            PopulationAdmissionV2Ledger::open_read(torn.path(), bounds()),
            "torn",
        );
    }

    #[test]
    fn semantic_reseal_reordered_and_duplicate_disk_blocks_refuse() {
        let resealed = TestRoot::new("semantic-reseal");
        initialize_empty(resealed.path());
        let value = prepared();
        let mut raw = encode_decision(&value.decisions[0]).expect("encode semantic fixture");
        let target = value.decisions[0].statistics_period_digest;
        let offset = raw[..DECISION_PAYLOAD_BYTES]
            .windows(32)
            .position(|window| window == target)
            .expect("find unique Statistics period digest");
        raw[offset..offset + 32].copy_from_slice(&digest(220));
        let seal = hash_slices(DECISION_SEAL_DOMAIN, &[&raw[..DECISION_PAYLOAD_BYTES]]);
        raw[DECISION_PAYLOAD_BYTES..].copy_from_slice(&seal);
        let mut file = OpenOptions::new()
            .write(true)
            .open(resealed.path().join(DECISION_FILE))
            .expect("open resealed fixture");
        file.write_all(&raw).expect("write resealed decision");
        file.sync_all().expect("sync resealed decision");
        assert_refuses(
            PopulationAdmissionV2Ledger::open_read(resealed.path(), bounds()),
            "identity",
        );

        let reordered = TestRoot::new("disk-reorder");
        initialize_empty(reordered.path());
        let mut swapped = value.decisions.clone();
        swapped.swap(0, 1);
        write_decisions(reordered.path(), &swapped);
        let completion = value
            .expected_completion(0, 0)
            .expect("derive reorder Completion");
        write_completion(reordered.path(), &completion);
        assert_refuses(
            PopulationAdmissionV2Ledger::open_read(reordered.path(), bounds()),
            "NIFTY-first",
        );

        let duplicate = TestRoot::new("disk-duplicate");
        initialize_empty(duplicate.path());
        let mut duplicate_decisions = value.decisions.clone();
        duplicate_decisions[1].candidate_semantic_id = duplicate_decisions[0].candidate_semantic_id;
        duplicate_decisions[1].decision_id = duplicate_decisions[1].derive_decision_id();
        write_decisions(duplicate.path(), &duplicate_decisions);
        let ordered = ordered_decision_digest(&duplicate_decisions).expect("digest duplicates");
        let mut duplicate_completion = completion;
        duplicate_completion.ordered_decision_digest = ordered;
        duplicate_completion.completion_id = duplicate_completion.derive_completion_id();
        write_completion(duplicate.path(), &duplicate_completion);
        assert_refuses(
            PopulationAdmissionV2Ledger::open_read(duplicate.path(), bounds()),
            "appears more than once",
        );
    }

    #[test]
    fn status_swap_with_preserved_counts_rekeys_and_outer_reseal_refuses() {
        const STATUS_OFFSET: usize = 80;

        let original = prepared();
        let original_completion = original
            .expected_completion(0, 0)
            .expect("derive original status Completion");
        let mut rekeyed = prepared();
        let first_status = rekeyed.decisions[0].status;
        rekeyed.decisions[0].status = rekeyed.decisions[1].status;
        rekeyed.decisions[1].status = first_status;
        rekeyed.decisions[0].decision_id = rekeyed.decisions[0].derive_decision_id();
        rekeyed.decisions[1].decision_id = rekeyed.decisions[1].derive_decision_id();
        let rekeyed_completion = rekeyed
            .expected_completion(0, 0)
            .expect("derive rekeyed status Completion");
        assert_eq!(
            (
                original_completion.admitted_count,
                original_completion.rejected_count,
                original_completion.unmeasured_count,
                original_completion.refused_count,
            ),
            (
                rekeyed_completion.admitted_count,
                rekeyed_completion.rejected_count,
                rekeyed_completion.unmeasured_count,
                rekeyed_completion.refused_count,
            )
        );
        assert_ne!(
            original.decisions[0].decision_id,
            rekeyed.decisions[0].decision_id
        );
        assert_ne!(
            original_completion.ordered_decision_digest,
            rekeyed_completion.ordered_decision_digest
        );
        assert_ne!(
            original_completion.completion_id,
            rekeyed_completion.completion_id
        );

        let root = TestRoot::new("status-outer-reseal");
        initialize_empty(root.path());
        let mut raw_decisions: Vec<_> = original
            .decisions
            .iter()
            .map(|decision| encode_decision(decision).expect("encode status fixture"))
            .collect();
        let first = raw_decisions[0][STATUS_OFFSET];
        raw_decisions[0][STATUS_OFFSET] = raw_decisions[1][STATUS_OFFSET];
        raw_decisions[1][STATUS_OFFSET] = first;
        for raw in raw_decisions.iter_mut().take(2) {
            let seal = hash_slices(DECISION_SEAL_DOMAIN, &[&raw[..DECISION_PAYLOAD_BYTES]]);
            raw[DECISION_PAYLOAD_BYTES..].copy_from_slice(&seal);
        }
        let path = root.path().join(DECISION_FILE);
        let mut file = OpenOptions::new()
            .write(true)
            .open(path)
            .expect("open status-swap decision file");
        for raw in raw_decisions {
            file.write_all(&raw).expect("write status-swap decision");
        }
        file.sync_all().expect("sync status-swap decisions");
        write_completion(root.path(), &original_completion);
        assert_refuses(
            PopulationAdmissionV2Ledger::open_read(root.path(), bounds()),
            "decision identity",
        );
    }

    #[test]
    fn same_length_mutation_and_root_replacement_invalidate_open_view() {
        let stale = TestRoot::new("stale");
        let value = prepared();
        persist_population_admission_v2(stale.path(), bounds(), &value)
            .expect("persist stale fixture");
        let ledger = PopulationAdmissionV2Ledger::open_read(stale.path(), bounds())
            .expect("open stale fixture");
        let path = stale.path().join(DECISION_FILE);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("open stale decision");
        file.seek(SeekFrom::Start(80)).expect("seek stale byte");
        file.write_all(&[0xaa]).expect("write stale byte");
        file.sync_all().expect("sync stale byte");
        assert_refuses(
            ledger.reopen_structural_receipt(&value.source.block_id),
            "generation changed",
        );

        let replaced = TestRoot::new("root-replaced");
        persist_population_admission_v2(replaced.path(), bounds(), &value)
            .expect("persist root replacement fixture");
        let ledger = PopulationAdmissionV2Ledger::open_read(replaced.path(), bounds())
            .expect("open root replacement fixture");
        let moved = replaced.path().with_extension("moved");
        std::fs::rename(replaced.path(), &moved).expect("move admitted root");
        std::fs::create_dir(replaced.path()).expect("replace admitted root path");
        assert_refuses(
            ledger.reopen_structural_receipt(&value.source.block_id),
            "root",
        );
        std::fs::remove_dir(replaced.path()).expect("remove replacement root");
        std::fs::rename(&moved, replaced.path()).expect("restore admitted root for Drop");
    }

    #[test]
    fn existing_root_and_no_follow_are_fail_closed() {
        let absent = TestRoot::absent("root");
        assert_refuses(
            PopulationAdmissionV2Ledger::open_write(absent.path(), bounds()),
            "cannot inspect",
        );

        let missing = TestRoot::new("missing-children");
        assert_refuses(
            PopulationAdmissionV2Ledger::open_read(missing.path(), bounds()),
            "cannot inspect",
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let linked = TestRoot::new("linked-child");
            let external = linked.path().join("external.bin");
            File::create(&external).expect("create external symlink target");
            symlink(&external, linked.path().join(DECISION_FILE)).expect("create decision symlink");
            assert_refuses(
                PopulationAdmissionV2Ledger::open_write(linked.path(), bounds()),
                "symbolic link",
            );
        }
    }

    #[test]
    fn over_bound_preparation_refuses_before_duplicate_set_allocation() {
        let root = TestRoot::new("bound-order");
        let mut value = prepared();
        let extra = value.decisions[0].clone();
        value
            .decisions
            .extend([extra.clone(), extra, value.decisions[1].clone()]);
        value.source.decision_count = 7;
        let tight = AdmissionV2Bounds::new(
            8,
            8 * POPULATION_ADMISSION_V2_DECISION_BYTES as u64,
            2,
            2 * POPULATION_ADMISSION_V2_COMPLETION_BYTES as u64,
            4,
        )
        .expect("valid tight bounds");
        let mut ledger = PopulationAdmissionV2Ledger::open_write(root.path(), tight)
            .expect("open bounded writer");
        assert_refuses(ledger.append(&value), "per-block maximum");
    }
}
