//! Canonical Population Finalization V2 records.
//!
//! This module owns the fixed byte contract that D-0464 and D-0479 require at
//! the final Candidate/Pre-Admission/Observation/Statistics/Population/
//! Admission join.  [`PreparedPopulationFinalizationV2`] has private fields,
//! no `Debug` implementation and no production constructor; it can currently
//! be created only by this module's adversarial tests.  A production preparation
//! door remains blocked until CLI can supply a freshly reopened, opaque
//! anchored-search lineage authority.  Runner's detached arithmetic projection
//! and caller-authored digests are not that authority.
//!
//! The byte order is one `Data` record, every canonical NIFTY-then-BANKNIFTY
//! `Rekey` record, and one `Completion` record.  The existing-root ledger is
//! bounded, append-only and receipt-last.  Public reopen yields only a plainly
//! structural receipt: self-consistent bytes do not authenticate their source.
//! Only crate-private persistence can exact-compare freshly reopened bytes with
//! an already-opaque preparation and promote that receipt.  No production
//! preparation or authenticated authority is currently constructible.

use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;
#[cfg(any(target_os = "android", target_os = "linux"))]
use std::os::unix::fs::OpenOptionsExt as _;
#[cfg(target_os = "macos")]
use std::os::unix::fs::OpenOptionsExt as _;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt as _;

use brutex_core::blake3::Hasher;

use crate::population::{InstrumentFamilyV1, RequestedSpanIdentityV1};

/// Bytes in every fixed Population Finalization V2 record.
pub const POPULATION_FINALIZATION_V2_RECORD_BYTES: usize = 2_048;

/// Operator-facing refusal at the Population Finalization V2 canonical seam.
pub type PopulationFinalizationV2Refusal = String;

/// Explicit physical-record and byte ceilings for Finalization V2.
///
/// There is no default.  Both ceilings are checked before any ledger-sized or
/// block-sized allocation and before every append.  They refuse excess input;
/// they never truncate, sample or silently reduce a population.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationFinalizationV2Bounds {
    max_records: u64,
    max_file_bytes: u64,
}

impl PopulationFinalizationV2Bounds {
    /// Constructs nonzero ceilings large enough for Data, both required family
    /// rows and Completion.
    ///
    /// # Errors
    ///
    /// Refuses fewer than four records, byte multiplication overflow or a byte
    /// ceiling smaller than four complete records.
    pub fn new(
        max_records: u64,
        max_file_bytes: u64,
    ) -> Result<Self, PopulationFinalizationV2Refusal> {
        const MINIMUM_RECORDS: u64 = 4;
        if max_records < MINIMUM_RECORDS {
            return Err(format!(
                "Finalization V2 max records {max_records} cannot hold Data, both family rows and Completion"
            ));
        }
        let minimum_bytes = MINIMUM_RECORDS
            .checked_mul(POPULATION_FINALIZATION_V2_RECORD_BYTES as u64)
            .ok_or_else(|| "Finalization V2 minimum byte bound overflowed".to_owned())?;
        if max_file_bytes < minimum_bytes {
            return Err(format!(
                "Finalization V2 max bytes {max_file_bytes} cannot hold the {minimum_bytes}-byte minimum block"
            ));
        }
        Ok(Self {
            max_records,
            max_file_bytes,
        })
    }

    /// Maximum fixed physical records, including a trailing prepared block.
    #[must_use]
    pub const fn max_records(self) -> u64 {
        self.max_records
    }

    /// Maximum ledger bytes.
    #[must_use]
    pub const fn max_file_bytes(self) -> u64 {
        self.max_file_bytes
    }
}

const PAYLOAD_BYTES: usize = POPULATION_FINALIZATION_V2_RECORD_BYTES - 32;
const SEAL_BYTES: usize = 32;
const VERSION: u32 = 2;
const MAGIC: [u8; 16] = *b"BTX-POPFNL-V2\0\0\0";
const DATA_DOMAIN: u32 = 1;
const REKEY_DOMAIN: u32 = 2;
const COMPLETION_DOMAIN: u32 = 3;
const RECORD_SEAL_DOMAIN: &[u8] = b"brutex-population-finalization-v2-record-seal\0";
const FINALIZATION_ID_DOMAIN: &[u8] = b"brutex-population-finalization-v2-id\0";
const ORDERED_REKEY_DOMAIN: &[u8] = b"brutex-population-finalization-v2-ordered-rekeys\0";
const COMPLETION_ID_DOMAIN: &[u8] = b"brutex-population-finalization-v2-completion\0";
const RAW_RECORD_DIGEST_DOMAIN: &[u8] = b"brutex-population-finalization-v2-raw-record\0";
const GENERATION_DOMAIN: &[u8] = b"brutex-population-finalization-v2-generation\0";
const LEDGER_FILE: &str = "population-finalization-v2.bin";
const LOCK_FILE: &str = "population-finalization-v2.lock";
const READ_CHUNK_BYTES: usize = 16 * 1_024;

#[cfg(any(target_os = "android", target_os = "linux"))]
const O_NOFOLLOW_FLAG: i32 = 0x20_000;
#[cfg(target_os = "macos")]
const O_NOFOLLOW_FLAG: i32 = 0x100;

const _: () = assert!(PAYLOAD_BYTES + SEAL_BYTES == POPULATION_FINALIZATION_V2_RECORD_BYTES);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RecordDomainV2 {
    Data,
    Rekey,
    Completion,
}

impl RecordDomainV2 {
    const fn value(self) -> u32 {
        match self {
            Self::Data => DATA_DOMAIN,
            Self::Rekey => REKEY_DOMAIN,
            Self::Completion => COMPLETION_DOMAIN,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FamilyAuthorityV2 {
    candidate_universe_id: [u8; 32],
    candidate_completion_digest: [u8; 32],
    pre_admission_authority_id: [u8; 32],
    population_id: [u8; 32],
    population_v4_completion_digest: [u8; 32],
    admission_completion_digest: [u8; 32],
    ordered_admission_decision_digest: [u8; 32],
    row_count: u64,
    admitted_count: u64,
    rejected_count: u64,
    unmeasured_count: u64,
    refused_count: u64,
}

impl FamilyAuthorityV2 {
    fn validate(self, name: &str) -> Result<(), PopulationFinalizationV2Refusal> {
        for (field, value) in [
            ("Candidate universe", self.candidate_universe_id),
            ("Candidate completion", self.candidate_completion_digest),
            ("Pre-Admission authority", self.pre_admission_authority_id),
            ("final Population V4", self.population_id),
            (
                "Population V4 completion",
                self.population_v4_completion_digest,
            ),
            ("Admission V1 completion", self.admission_completion_digest),
            (
                "Admission V1 ordered decisions",
                self.ordered_admission_decision_digest,
            ),
        ] {
            require_nonzero(&format!("{name} {field}"), value)?;
        }
        if self.row_count == 0 {
            return Err(format!(
                "{name} Finalization family has zero complete candidates"
            ));
        }
        let classified = checked_sum(
            &[
                self.admitted_count,
                self.rejected_count,
                self.unmeasured_count,
                self.refused_count,
            ],
            &format!("{name} Admission V1 status counts"),
        )?;
        if classified != self.row_count {
            return Err(format!(
                "{name} final rows total {}, but Admission V1 statuses total {classified}",
                self.row_count
            ));
        }
        Ok(())
    }

    fn encode(self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationFinalizationV2Refusal> {
        for value in [
            self.candidate_universe_id,
            self.candidate_completion_digest,
            self.pre_admission_authority_id,
            self.population_id,
            self.population_v4_completion_digest,
            self.admission_completion_digest,
            self.ordered_admission_decision_digest,
        ] {
            writer.bytes(&value)?;
        }
        for value in [
            self.row_count,
            self.admitted_count,
            self.rejected_count,
            self.unmeasured_count,
            self.refused_count,
        ] {
            writer.u64(value)?;
        }
        Ok(())
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationFinalizationV2Refusal> {
        Ok(Self {
            candidate_universe_id: reader.array()?,
            candidate_completion_digest: reader.array()?,
            pre_admission_authority_id: reader.array()?,
            population_id: reader.array()?,
            population_v4_completion_digest: reader.array()?,
            admission_completion_digest: reader.array()?,
            ordered_admission_decision_digest: reader.array()?,
            row_count: reader.u64()?,
            admitted_count: reader.u64()?,
            rejected_count: reader.u64()?,
            unmeasured_count: reader.u64()?,
            refused_count: reader.u64()?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EvidenceAuthorityV2 {
    observation_authority_id: [u8; 32],
    observation_completion_digest: [u8; 32],
    observation_pair_identity: [u8; 32],
    observation_source_identity: [u8; 32],
    observation_policy_digest: [u8; 32],
    observation_layout_policy_digest: [u8; 32],
    observation_statistics_link_id: [u8; 32],
    statistics_audit_id: [u8; 32],
    statistics_completion_digest: [u8; 32],
    statistics_projection_policy_digest: [u8; 32],
    wilson_policy_digest: [u8; 32],
    cscv_policy_digest: [u8; 32],
    ordered_candidate_digest: [u8; 32],
    ordered_period_digest: [u8; 32],
    ordered_split_digest: [u8; 32],
    white_family_digest: [u8; 32],
    spa_family_digest: [u8; 32],
    romano_wolf_family_digest: [u8; 32],
    candidate_count: u64,
    period_count: u64,
    split_count: u64,
    bootstrap_draws: u64,
    bootstrap_seed: u64,
    bootstrap_block_length: u64,
}

impl EvidenceAuthorityV2 {
    fn validate(self) -> Result<(), PopulationFinalizationV2Refusal> {
        for (name, value) in [
            ("Observation authority", self.observation_authority_id),
            ("Observation completion", self.observation_completion_digest),
            ("Observation pair", self.observation_pair_identity),
            ("Observation source", self.observation_source_identity),
            ("Observation policy", self.observation_policy_digest),
            (
                "Observation layout policy",
                self.observation_layout_policy_digest,
            ),
            (
                "Observation-to-Statistics link",
                self.observation_statistics_link_id,
            ),
            ("Statistics audit", self.statistics_audit_id),
            ("Statistics completion", self.statistics_completion_digest),
            (
                "Statistics projection policy",
                self.statistics_projection_policy_digest,
            ),
            ("Wilson policy", self.wilson_policy_digest),
            ("CSCV policy", self.cscv_policy_digest),
            (
                "Statistics ordered candidates",
                self.ordered_candidate_digest,
            ),
            ("Statistics ordered periods", self.ordered_period_digest),
            ("Statistics ordered splits", self.ordered_split_digest),
            ("White family", self.white_family_digest),
            ("SPA family", self.spa_family_digest),
            ("Romano-Wolf family", self.romano_wolf_family_digest),
        ] {
            require_nonzero(name, value)?;
        }
        if self.period_count < 2
            || self.split_count == 0
            || self.bootstrap_draws == 0
            || self.bootstrap_block_length == 0
        {
            return Err(
                "Finalization Statistics shape requires >=2 periods and nonzero splits/draws/block"
                    .to_owned(),
            );
        }
        Ok(())
    }

    fn encode(self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationFinalizationV2Refusal> {
        for value in [
            self.observation_authority_id,
            self.observation_completion_digest,
            self.observation_pair_identity,
            self.observation_source_identity,
            self.observation_policy_digest,
            self.observation_layout_policy_digest,
            self.observation_statistics_link_id,
            self.statistics_audit_id,
            self.statistics_completion_digest,
            self.statistics_projection_policy_digest,
            self.wilson_policy_digest,
            self.cscv_policy_digest,
            self.ordered_candidate_digest,
            self.ordered_period_digest,
            self.ordered_split_digest,
            self.white_family_digest,
            self.spa_family_digest,
            self.romano_wolf_family_digest,
        ] {
            writer.bytes(&value)?;
        }
        for value in [
            self.candidate_count,
            self.period_count,
            self.split_count,
            self.bootstrap_draws,
            self.bootstrap_seed,
            self.bootstrap_block_length,
        ] {
            writer.u64(value)?;
        }
        Ok(())
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationFinalizationV2Refusal> {
        Ok(Self {
            observation_authority_id: reader.array()?,
            observation_completion_digest: reader.array()?,
            observation_pair_identity: reader.array()?,
            observation_source_identity: reader.array()?,
            observation_policy_digest: reader.array()?,
            observation_layout_policy_digest: reader.array()?,
            observation_statistics_link_id: reader.array()?,
            statistics_audit_id: reader.array()?,
            statistics_completion_digest: reader.array()?,
            statistics_projection_policy_digest: reader.array()?,
            wilson_policy_digest: reader.array()?,
            cscv_policy_digest: reader.array()?,
            ordered_candidate_digest: reader.array()?,
            ordered_period_digest: reader.array()?,
            ordered_split_digest: reader.array()?,
            white_family_digest: reader.array()?,
            spa_family_digest: reader.array()?,
            romano_wolf_family_digest: reader.array()?,
            candidate_count: reader.u64()?,
            period_count: reader.u64()?,
            split_count: reader.u64()?,
            bootstrap_draws: reader.u64()?,
            bootstrap_seed: reader.u64()?,
            bootstrap_block_length: reader.u64()?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SearchAuthorityV2 {
    population_search_id: [u8; 32],
    ranking_validation_policy_digest: [u8; 32],
    walk_facts_digest: [u8; 32],
    source_policy_digest: [u8; 32],
    finalization_family_digest: [u8; 32],
    anchored_walk_authority_id: [u8; 32],
    opaque_cli_search_lineage_digest: [u8; 32],
    fold_count: u64,
    decided_folds: u64,
    profitable_oos_folds: u64,
    aggregate_oos_paisa: i64,
}

impl SearchAuthorityV2 {
    fn validate(self) -> Result<(), PopulationFinalizationV2Refusal> {
        for (name, value) in [
            ("population search", self.population_search_id),
            (
                "ranking/validation policy",
                self.ranking_validation_policy_digest,
            ),
            ("walk facts", self.walk_facts_digest),
            ("source policy", self.source_policy_digest),
            ("finalization family", self.finalization_family_digest),
            ("anchored walk authority", self.anchored_walk_authority_id),
            (
                "opaque freshly reopened CLI search lineage",
                self.opaque_cli_search_lineage_digest,
            ),
        ] {
            require_nonzero(name, value)?;
        }
        if self.fold_count == 0
            || self.decided_folds > self.fold_count
            || self.profitable_oos_folds > self.decided_folds
        {
            return Err("Finalization anchored-walk count hierarchy is invalid".to_owned());
        }
        if (self.decided_folds == 0 && self.aggregate_oos_paisa != 0)
            || (self.profitable_oos_folds == 0 && self.aggregate_oos_paisa > 0)
            || (self.profitable_oos_folds == self.decided_folds
                && self.decided_folds > 0
                && self.aggregate_oos_paisa <= 0)
        {
            return Err("Finalization anchored-walk outcome hierarchy is invalid".to_owned());
        }
        Ok(())
    }

    fn encode(self, writer: &mut FixedWriter<'_>) -> Result<(), PopulationFinalizationV2Refusal> {
        for value in [
            self.population_search_id,
            self.ranking_validation_policy_digest,
            self.walk_facts_digest,
            self.source_policy_digest,
            self.finalization_family_digest,
            self.anchored_walk_authority_id,
            self.opaque_cli_search_lineage_digest,
        ] {
            writer.bytes(&value)?;
        }
        writer.u64(self.fold_count)?;
        writer.u64(self.decided_folds)?;
        writer.u64(self.profitable_oos_folds)?;
        writer.i64(self.aggregate_oos_paisa)
    }

    fn decode(reader: &mut FixedReader<'_>) -> Result<Self, PopulationFinalizationV2Refusal> {
        Ok(Self {
            population_search_id: reader.array()?,
            ranking_validation_policy_digest: reader.array()?,
            walk_facts_digest: reader.array()?,
            source_policy_digest: reader.array()?,
            finalization_family_digest: reader.array()?,
            anchored_walk_authority_id: reader.array()?,
            opaque_cli_search_lineage_digest: reader.array()?,
            fold_count: reader.u64()?,
            decided_folds: reader.u64()?,
            profitable_oos_folds: reader.u64()?,
            aggregate_oos_paisa: reader.i64()?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PopulationFinalizationDataV2 {
    finalization_id: [u8; 32],
    rung_seconds: u32,
    horizon_bars: u32,
    requested_span: RequestedSpanIdentityV1,
    feed_digest: [u8; 32],
    source_commit_digest: [u8; 32],
    calendar_policy_digest: [u8; 32],
    daily_reference_policy_digest: [u8; 32],
    ranking_policy_digest: [u8; 32],
    admission_policy_digest: [u8; 32],
    source_policy_digest: [u8; 32],
    finalization_policy_digest: [u8; 32],
    nifty: FamilyAuthorityV2,
    banknifty: FamilyAuthorityV2,
    evidence: EvidenceAuthorityV2,
    search: SearchAuthorityV2,
    rekey_count: u64,
}

impl PopulationFinalizationDataV2 {
    fn validate(self) -> Result<(), PopulationFinalizationV2Refusal> {
        require_nonzero("Finalization identity", self.finalization_id)?;
        if self.rung_seconds == 0 || self.horizon_bars == 0 {
            return Err("Finalization rung and horizon must be nonzero".to_owned());
        }
        RequestedSpanIdentityV1::new(
            self.requested_span.from_year(),
            self.requested_span.from_month(),
            self.requested_span.to_year(),
            self.requested_span.to_month(),
        )
        .map_err(|why| format!("Finalization requested span is invalid: {why}"))?;
        for (name, value) in [
            ("feed", self.feed_digest),
            ("source commit", self.source_commit_digest),
            ("calendar policy", self.calendar_policy_digest),
            ("daily-reference policy", self.daily_reference_policy_digest),
            ("ranking policy", self.ranking_policy_digest),
            ("admission policy", self.admission_policy_digest),
            ("source policy", self.source_policy_digest),
            ("finalization policy", self.finalization_policy_digest),
        ] {
            require_nonzero(name, value)?;
        }
        self.nifty.validate("NIFTY")?;
        self.banknifty.validate("BANKNIFTY")?;
        self.evidence.validate()?;
        self.search.validate()?;
        let expected_rekeys = self
            .nifty
            .row_count
            .checked_add(self.banknifty.row_count)
            .ok_or_else(|| "Finalization paired row count overflowed".to_owned())?;
        if expected_rekeys != self.rekey_count || expected_rekeys != self.evidence.candidate_count {
            return Err(format!(
                "Finalization declares {} rekeys and {} Statistics candidates, but final families contain {expected_rekeys}",
                self.rekey_count, self.evidence.candidate_count
            ));
        }
        if self.ranking_policy_digest != self.search.ranking_validation_policy_digest
            || self.source_policy_digest != self.search.source_policy_digest
        {
            return Err(
                "Finalization top-level ranking/source policies crosswire anchored search"
                    .to_owned(),
            );
        }
        Ok(())
    }

    fn encode_body(
        self,
        writer: &mut FixedWriter<'_>,
    ) -> Result<(), PopulationFinalizationV2Refusal> {
        writer.u32(self.rung_seconds)?;
        writer.u32(self.horizon_bars)?;
        encode_span(writer, self.requested_span)?;
        for value in [
            self.feed_digest,
            self.source_commit_digest,
            self.calendar_policy_digest,
            self.daily_reference_policy_digest,
            self.ranking_policy_digest,
            self.admission_policy_digest,
            self.source_policy_digest,
            self.finalization_policy_digest,
        ] {
            writer.bytes(&value)?;
        }
        self.nifty.encode(writer)?;
        self.banknifty.encode(writer)?;
        self.evidence.encode(writer)?;
        self.search.encode(writer)?;
        writer.u64(self.rekey_count)
    }

    fn decode_body(
        finalization_id: [u8; 32],
        reader: &mut FixedReader<'_>,
    ) -> Result<Self, PopulationFinalizationV2Refusal> {
        Ok(Self {
            finalization_id,
            rung_seconds: reader.u32()?,
            horizon_bars: reader.u32()?,
            requested_span: decode_span(reader)?,
            feed_digest: reader.array()?,
            source_commit_digest: reader.array()?,
            calendar_policy_digest: reader.array()?,
            daily_reference_policy_digest: reader.array()?,
            ranking_policy_digest: reader.array()?,
            admission_policy_digest: reader.array()?,
            source_policy_digest: reader.array()?,
            finalization_policy_digest: reader.array()?,
            nifty: FamilyAuthorityV2::decode(reader)?,
            banknifty: FamilyAuthorityV2::decode(reader)?,
            evidence: EvidenceAuthorityV2::decode(reader)?,
            search: SearchAuthorityV2::decode(reader)?,
            rekey_count: reader.u64()?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PopulationFinalizationRekeyV2 {
    finalization_id: [u8; 32],
    sequence: u64,
    family: InstrumentFamilyV1,
    family_sequence: u64,
    candidate_semantic_digest: [u8; 32],
    statistics_period_digest: [u8; 32],
    statistics_split_digest: [u8; 32],
    population_id: [u8; 32],
    final_strategy_digest: [u8; 32],
    population_row_payload_digest: [u8; 32],
    admission_v1_decision_digest: [u8; 32],
    admission_v2_evidence_digest: [u8; 32],
    admission_v2_verdict_digest: [u8; 32],
    admission_v2_decision_digest: [u8; 32],
}

impl PopulationFinalizationRekeyV2 {
    fn validate(self) -> Result<(), PopulationFinalizationV2Refusal> {
        require_nonzero("rekey Finalization identity", self.finalization_id)?;
        for (name, value) in [
            ("Candidate semantic", self.candidate_semantic_digest),
            (
                "candidate Statistics periods",
                self.statistics_period_digest,
            ),
            ("candidate Statistics splits", self.statistics_split_digest),
            ("final Population V4", self.population_id),
            ("final strategy", self.final_strategy_digest),
            (
                "Population V4 row payload",
                self.population_row_payload_digest,
            ),
            ("Admission V1 decision", self.admission_v1_decision_digest),
            ("Admission V2 evidence", self.admission_v2_evidence_digest),
            ("Admission V2 verdict", self.admission_v2_verdict_digest),
            ("Admission V2 decision", self.admission_v2_decision_digest),
        ] {
            require_nonzero(name, value)?;
        }
        Ok(())
    }

    fn encode_body(
        self,
        writer: &mut FixedWriter<'_>,
    ) -> Result<(), PopulationFinalizationV2Refusal> {
        writer.u64(self.sequence)?;
        writer.u8(family_byte(self.family))?;
        writer.zeros(7)?;
        writer.u64(self.family_sequence)?;
        for value in [
            self.candidate_semantic_digest,
            self.statistics_period_digest,
            self.statistics_split_digest,
            self.population_id,
            self.final_strategy_digest,
            self.population_row_payload_digest,
            self.admission_v1_decision_digest,
            self.admission_v2_evidence_digest,
            self.admission_v2_verdict_digest,
            self.admission_v2_decision_digest,
        ] {
            writer.bytes(&value)?;
        }
        Ok(())
    }

    fn decode_body(
        finalization_id: [u8; 32],
        reader: &mut FixedReader<'_>,
    ) -> Result<Self, PopulationFinalizationV2Refusal> {
        let sequence = reader.u64()?;
        let family = decode_family(reader.u8()?)?;
        reader.require_zeros(7, "rekey family reserve")?;
        let family_sequence = reader.u64()?;
        Ok(Self {
            finalization_id,
            sequence,
            family,
            family_sequence,
            candidate_semantic_digest: reader.array()?,
            statistics_period_digest: reader.array()?,
            statistics_split_digest: reader.array()?,
            population_id: reader.array()?,
            final_strategy_digest: reader.array()?,
            population_row_payload_digest: reader.array()?,
            admission_v1_decision_digest: reader.array()?,
            admission_v2_evidence_digest: reader.array()?,
            admission_v2_verdict_digest: reader.array()?,
            admission_v2_decision_digest: reader.array()?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PopulationFinalizationCompletionV2 {
    finalization_id: [u8; 32],
    block_sequence: u64,
    data_record_digest: [u8; 32],
    rekey_count: u64,
    nifty_rekey_count: u64,
    banknifty_rekey_count: u64,
    ordered_rekey_digest: [u8; 32],
    completion_id: [u8; 32],
}

impl PopulationFinalizationCompletionV2 {
    fn validate(self) -> Result<(), PopulationFinalizationV2Refusal> {
        for (name, value) in [
            ("Completion Finalization", self.finalization_id),
            ("Completion Data record", self.data_record_digest),
            ("Completion ordered rekeys", self.ordered_rekey_digest),
            ("Completion identity", self.completion_id),
        ] {
            require_nonzero(name, value)?;
        }
        let paired = self
            .nifty_rekey_count
            .checked_add(self.banknifty_rekey_count)
            .ok_or_else(|| "Completion paired rekey count overflowed".to_owned())?;
        if paired != self.rekey_count {
            return Err(format!(
                "Completion declares {} rekeys, but family counts total {paired}",
                self.rekey_count
            ));
        }
        if derive_completion_id(self) != self.completion_id {
            return Err("Completion identity does not reproduce".to_owned());
        }
        Ok(())
    }

    fn encode_body(
        self,
        writer: &mut FixedWriter<'_>,
    ) -> Result<(), PopulationFinalizationV2Refusal> {
        writer.u64(self.block_sequence)?;
        writer.bytes(&self.data_record_digest)?;
        writer.u64(self.rekey_count)?;
        writer.u64(self.nifty_rekey_count)?;
        writer.u64(self.banknifty_rekey_count)?;
        writer.bytes(&self.ordered_rekey_digest)?;
        writer.bytes(&self.completion_id)
    }

    fn decode_body(
        finalization_id: [u8; 32],
        reader: &mut FixedReader<'_>,
    ) -> Result<Self, PopulationFinalizationV2Refusal> {
        Ok(Self {
            finalization_id,
            block_sequence: reader.u64()?,
            data_record_digest: reader.array()?,
            rekey_count: reader.u64()?,
            nifty_rekey_count: reader.u64()?,
            banknifty_rekey_count: reader.u64()?,
            ordered_rekey_digest: reader.array()?,
            completion_id: reader.array()?,
        })
    }
}

/// Opaque block prepared for receipt-last persistence.
///
/// All fields are private and this module exposes no production constructor.
/// In particular, there is no API that accepts a caller-written search-lineage
/// digest.  A later production constructor must consume a freshly reopened,
/// opaque CLI search authority before it may return this type.
#[derive(Clone, PartialEq, Eq)]
pub struct PreparedPopulationFinalizationV2 {
    data: PopulationFinalizationDataV2,
    rekeys: Vec<PopulationFinalizationRekeyV2>,
}

impl PreparedPopulationFinalizationV2 {
    /// Semantic identity binding every source, policy, final receipt and rekey family.
    #[must_use]
    pub const fn finalization_id(&self) -> [u8; 32] {
        self.data.finalization_id
    }

    /// Exact NIFTY-plus-BANKNIFTY rekey cardinality.
    #[must_use]
    pub const fn rekey_count(&self) -> u64 {
        self.data.rekey_count
    }

    /// Computes the physical Data + Rekey + Completion cardinality without
    /// validating semantics or allocating any duplicate-detection index.
    fn checked_physical_record_count(&self) -> Result<u64, PopulationFinalizationV2Refusal> {
        u64::try_from(self.rekeys.len())
            .map_err(|_| "Finalization rekey length does not fit u64".to_owned())?
            .checked_add(2)
            .ok_or_else(|| "Finalization physical record count overflowed u64".to_owned())
    }

    fn validate(&self) -> Result<(), PopulationFinalizationV2Refusal> {
        self.data.validate()?;
        let actual = self
            .checked_physical_record_count()?
            .checked_sub(2)
            .ok_or_else(|| "Finalization physical record count underflowed".to_owned())?;
        if actual != self.data.rekey_count {
            return Err(format!(
                "Finalization prepared block has {actual} rekeys, expected {}",
                self.data.rekey_count
            ));
        }
        let mut candidate_ids = HashSet::new();
        let mut strategy_ids = HashSet::new();
        candidate_ids
            .try_reserve(self.rekeys.len())
            .map_err(|why| format!("cannot reserve Finalization Candidate identity set: {why}"))?;
        strategy_ids
            .try_reserve(self.rekeys.len())
            .map_err(|why| format!("cannot reserve Finalization strategy identity set: {why}"))?;
        for (index, row) in self.rekeys.iter().copied().enumerate() {
            row.validate()?;
            let sequence = u64::try_from(index)
                .map_err(|_| "Finalization rekey index does not fit u64".to_owned())?;
            if row.finalization_id != self.data.finalization_id || row.sequence != sequence {
                return Err(format!(
                    "Finalization rekey {index} has foreign identity or sequence"
                ));
            }
            let (expected_family, expected_family_sequence, population_id) =
                if sequence < self.data.nifty.row_count {
                    (
                        InstrumentFamilyV1::Nifty,
                        sequence,
                        self.data.nifty.population_id,
                    )
                } else {
                    (
                        InstrumentFamilyV1::BankNifty,
                        sequence
                            .checked_sub(self.data.nifty.row_count)
                            .ok_or_else(|| "Finalization family sequence underflowed".to_owned())?,
                        self.data.banknifty.population_id,
                    )
                };
            if row.family != expected_family
                || row.family_sequence != expected_family_sequence
                || row.population_id != population_id
            {
                return Err(format!(
                    "Finalization rekey {index} violates canonical NIFTY-then-BANKNIFTY family mapping"
                ));
            }
            if !candidate_ids.insert(row.candidate_semantic_digest) {
                return Err(format!(
                    "Finalization repeats Candidate semantic identity at rekey {index}"
                ));
            }
            if !strategy_ids.insert(row.final_strategy_digest) {
                return Err(format!(
                    "Finalization repeats final strategy identity at rekey {index}"
                ));
            }
        }
        if derive_finalization_id(&self.data, &self.rekeys)? != self.data.finalization_id {
            return Err(
                "Finalization identity does not reproduce from every Data and rekey term"
                    .to_owned(),
            );
        }
        Ok(())
    }

    fn records(
        &self,
        block_sequence: u64,
        first_physical_record: u64,
    ) -> Result<Vec<[u8; POPULATION_FINALIZATION_V2_RECORD_BYTES]>, PopulationFinalizationV2Refusal>
    {
        self.validate()?;
        let additional = self
            .rekeys
            .len()
            .checked_add(2)
            .ok_or_else(|| "Finalization record count overflowed usize".to_owned())?;
        let mut records = Vec::new();
        records
            .try_reserve_exact(additional)
            .map_err(|why| format!("cannot reserve Finalization records: {why}"))?;
        let data_record = encode_data(&self.data, first_physical_record)?;
        let data_record_digest = raw_record_digest(&data_record);
        records.push(data_record);
        let mut ordered = Hasher::new();
        ordered.update(ORDERED_REKEY_DOMAIN);
        ordered.update(&self.data.finalization_id);
        ordered.update(&self.data.rekey_count.to_le_bytes());
        for row in &self.rekeys {
            let physical = first_physical_record
                .checked_add(
                    u64::try_from(records.len())
                        .map_err(|_| "Finalization physical index does not fit u64".to_owned())?,
                )
                .ok_or_else(|| "Finalization physical rekey index overflowed".to_owned())?;
            let raw = encode_rekey(row, physical)?;
            ordered.update(&raw);
            records.push(raw);
        }
        let mut completion = PopulationFinalizationCompletionV2 {
            finalization_id: self.data.finalization_id,
            block_sequence,
            data_record_digest,
            rekey_count: self.data.rekey_count,
            nifty_rekey_count: self.data.nifty.row_count,
            banknifty_rekey_count: self.data.banknifty.row_count,
            ordered_rekey_digest: ordered.finalize(),
            completion_id: [0; 32],
        };
        completion.completion_id = derive_completion_id(completion);
        completion.validate()?;
        let physical = first_physical_record
            .checked_add(
                u64::try_from(records.len())
                    .map_err(|_| "Finalization Completion index does not fit u64".to_owned())?,
            )
            .ok_or_else(|| "Finalization Completion index overflowed".to_owned())?;
        records.push(encode_completion(completion, physical)?);
        Ok(records)
    }
}

/// Freshly indexed receipt-last structural receipt.
///
/// This proves only that bounded bytes are internally self-consistent.  It is
/// intentionally public for audit and must never be treated as authenticated
/// source authority.  A caller able to write and reseal all records can create
/// a structurally valid ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationFinalizationV2StructuralReceipt {
    finalization_id: [u8; 32],
    block_sequence: u64,
    first_physical_record: u64,
    rekey_count: u64,
    data_record_digest: [u8; 32],
    completion_id: [u8; 32],
    completion_record_digest: [u8; 32],
}

impl PopulationFinalizationV2StructuralReceipt {
    /// Semantic identity binding the entire finalized population.
    #[must_use]
    pub const fn finalization_id(self) -> [u8; 32] {
        self.finalization_id
    }

    /// Canonical zero-based completed-block sequence.
    #[must_use]
    pub const fn block_sequence(self) -> u64 {
        self.block_sequence
    }

    /// Physical record at which this block's Data member begins.
    #[must_use]
    pub const fn first_physical_record(self) -> u64 {
        self.first_physical_record
    }

    /// Exact NIFTY-plus-BANKNIFTY Rekey cardinality.
    #[must_use]
    pub const fn rekey_count(self) -> u64 {
        self.rekey_count
    }

    /// Digest of the exact fixed Data record bytes.
    #[must_use]
    pub const fn data_record_digest(self) -> [u8; 32] {
        self.data_record_digest
    }

    /// Semantic receipt-last Completion identity.
    #[must_use]
    pub const fn completion_id(self) -> [u8; 32] {
        self.completion_id
    }

    /// Digest of the exact fixed Completion record bytes freshly scanned.
    #[must_use]
    pub const fn completion_record_digest(self) -> [u8; 32] {
        self.completion_record_digest
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the dormant persistence seam cannot be called until upstream lineage authority is constructible"
        )
    )]
    fn record_count(self) -> Result<u64, PopulationFinalizationV2Refusal> {
        self.rekey_count
            .checked_add(2)
            .ok_or_else(|| "Finalization V2 structural receipt record count overflowed".to_owned())
    }
}

/// Crate-private authenticated authority.
///
/// This wrapper is minted only after fresh read-only reopen and exact byte
/// comparison against an opaque [`PreparedPopulationFinalizationV2`].  It has
/// no public construction or formatting surface.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct PopulationFinalizationV2Authority {
    receipt: PopulationFinalizationV2StructuralReceipt,
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "authenticated accessors remain dormant until upstream lineage authority is constructible"
    )
)]
impl PopulationFinalizationV2Authority {
    #[must_use]
    pub(crate) const fn finalization_id(self) -> [u8; 32] {
        self.receipt.finalization_id
    }

    #[must_use]
    pub(crate) const fn block_sequence(self) -> u64 {
        self.receipt.block_sequence
    }

    #[must_use]
    pub(crate) const fn first_physical_record(self) -> u64 {
        self.receipt.first_physical_record
    }

    #[must_use]
    pub(crate) const fn rekey_count(self) -> u64 {
        self.receipt.rekey_count
    }

    #[must_use]
    pub(crate) const fn data_record_digest(self) -> [u8; 32] {
        self.receipt.data_record_digest
    }

    #[must_use]
    pub(crate) const fn completion_id(self) -> [u8; 32] {
        self.receipt.completion_id
    }

    #[must_use]
    pub(crate) const fn completion_record_digest(self) -> [u8; 32] {
        self.receipt.completion_record_digest
    }
}

/// Structural write result retained only until fresh reopen authenticates it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PopulationFinalizationV2StructuralCommit {
    Written(PopulationFinalizationV2StructuralReceipt),
    Reused(PopulationFinalizationV2StructuralReceipt),
}

impl PopulationFinalizationV2StructuralCommit {
    const fn receipt(self) -> PopulationFinalizationV2StructuralReceipt {
        match self {
            Self::Written(receipt) | Self::Reused(receipt) => receipt,
        }
    }
}

/// Exact authenticated persistence outcome returned only after the writer is
/// dropped, a fresh read-only open reproduces the structural receipt and every
/// byte exactly matches the opaque preparation.
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the persistence result remains crate-private until upstream lineage authority is constructible"
    )
)]
pub(crate) enum PopulationFinalizationV2Commit {
    /// New Data/Rekey bytes and Completion were durably appended.
    Written(PopulationFinalizationV2Authority),
    /// An existing byte-identical completed block was reused.
    Reused(PopulationFinalizationV2Authority),
}

impl PopulationFinalizationV2Commit {
    /// Freshly reopened authority for either exact outcome.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the persistence result remains dormant until upstream lineage authority is constructible"
        )
    )]
    pub(crate) const fn authority(self) -> PopulationFinalizationV2Authority {
        match self {
            Self::Written(authority) | Self::Reused(authority) => authority,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
struct TrailingPreparedBlockV2 {
    first_physical_record: u64,
    block_sequence: u64,
    prepared: PreparedPopulationFinalizationV2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGenerationV2 {
    len: u64,
    content_digest: [u8; 32],
    platform: PlatformGenerationV2,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGenerationV2 {
    device: u64,
    inode: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGenerationV2 {
    volume_serial: u32,
    file_index: u64,
    creation_time: u64,
    last_write_time: u64,
}

#[cfg(not(any(unix, windows)))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGenerationV2;

#[cfg(unix)]
impl PlatformGenerationV2 {
    const fn same_file_object(self, other: Self) -> bool {
        self.device == other.device && self.inode == other.inode
    }
}

#[cfg(windows)]
impl PlatformGenerationV2 {
    const fn same_file_object(self, other: Self) -> bool {
        self.volume_serial == other.volume_serial && self.file_index == other.file_index
    }
}

#[cfg(not(any(unix, windows)))]
impl PlatformGenerationV2 {
    const fn same_file_object(self, _other: Self) -> bool {
        false
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DirectoryGenerationV2 {
    canonical_path: PathBuf,
    platform: PlatformGenerationV2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LedgerGenerationWitnessV2 {
    root: DirectoryGenerationV2,
    lock: FileGenerationV2,
    data: FileGenerationV2,
}

/// Open bounded Finalization V2 ledger.
///
/// Opening scans and validates at most the configured file ceiling, O(file
/// bytes + Rekey rows) time and O(completed blocks + largest block) memory.
/// The in-memory identity probe is average O(1) only after that scan.  Public
/// lookup first re-hashes the bounded file to reject same-length mutation, so
/// the complete lookup operation is O(file bytes), not universally O(1).
pub struct PopulationFinalizationV2Ledger {
    root_file: File,
    root_path: PathBuf,
    root_generation: DirectoryGenerationV2,
    lock_path: PathBuf,
    data_path: PathBuf,
    lock_file: File,
    data_file: File,
    bounds: PopulationFinalizationV2Bounds,
    receipts: HashMap<[u8; 32], PopulationFinalizationV2StructuralReceipt>,
    trailing: Option<TrailingPreparedBlockV2>,
    completed_blocks: u64,
    physical_records: u64,
    lock_generation: FileGenerationV2,
    data_generation: FileGenerationV2,
    #[cfg(test)]
    before_completion_test_hook:
        Option<Box<dyn FnOnce() -> Result<(), PopulationFinalizationV2Refusal>>>,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "write mode remains dormant until upstream lineage authority is constructible"
        )
    )]
    writable: bool,
}

impl PopulationFinalizationV2Ledger {
    /// Opens existing lock and ledger files read-only without creating any path.
    ///
    /// # Errors
    ///
    /// Refuses an absent/non-directory root, absent files, a bound breach,
    /// lock/path-generation failure, ragged/torn/corrupt/foreign/reordered/
    /// duplicate records, a middle orphan or more than one trailing block.
    pub fn open_read(
        root: impl AsRef<Path>,
        bounds: PopulationFinalizationV2Bounds,
    ) -> Result<Self, PopulationFinalizationV2Refusal> {
        Self::open_inner(root.as_ref(), bounds, false)
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "write open remains dormant until upstream lineage authority is constructible"
        )
    )]
    fn open_write(
        root: &Path,
        bounds: PopulationFinalizationV2Bounds,
    ) -> Result<Self, PopulationFinalizationV2Refusal> {
        Self::open_inner(root, bounds, true)
    }

    fn open_inner(
        root: &Path,
        bounds: PopulationFinalizationV2Bounds,
        writable: bool,
    ) -> Result<Self, PopulationFinalizationV2Refusal> {
        let root_file = open_root_directory(root)?;
        let root_path = root.to_path_buf();
        let lock_path = root.join(LOCK_FILE);
        let data_path = root.join(LEDGER_FILE);
        let lock_file = open_ledger_file(&lock_path, writable, writable)?;
        if writable {
            lock_file.lock().map_err(|why| {
                format!(
                    "cannot take Finalization V2 writer lock {}: {why}",
                    lock_path.display()
                )
            })?;
        } else {
            lock_file.lock_shared().map_err(|why| {
                format!(
                    "cannot take shared Finalization V2 lock {}: {why}",
                    lock_path.display()
                )
            })?;
        }
        let held_lock = lock_file.try_clone().map_err(|why| {
            format!(
                "cannot clone Finalization V2 lock {}: {why}",
                lock_path.display()
            )
        })?;
        let opened = (|| {
            let data_file = open_ledger_file(&data_path, writable, writable)?;
            require_regular_file(&held_lock, &lock_path)?;
            require_regular_file(&data_file, &data_path)?;
            if writable {
                sync_directory_entries(&root_file, &root_path)?;
            }
            let root_generation = directory_generation(&root_file, &root_path)?;
            let lock_generation = file_generation(&held_lock, &lock_path, 0)?;
            let data_generation = file_generation(&data_file, &data_path, bounds.max_file_bytes)?;
            let mut ledger = Self {
                root_file,
                root_path,
                root_generation,
                lock_path: lock_path.clone(),
                data_path,
                lock_file: held_lock,
                data_file,
                bounds,
                receipts: HashMap::new(),
                trailing: None,
                completed_blocks: 0,
                physical_records: 0,
                lock_generation,
                data_generation,
                #[cfg(test)]
                before_completion_test_hook: None,
                writable,
            };
            ledger.scan()?;
            Ok(ledger)
        })();
        let released = lock_file.unlock().map_err(|why| {
            format!(
                "cannot release Finalization V2 open lock {}: {why}",
                lock_path.display()
            )
        });
        match (opened, released) {
            (Ok(ledger), Ok(())) => Ok(ledger),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn scan(&mut self) -> Result<(), PopulationFinalizationV2Refusal> {
        let file_len = checked_file_len(&self.data_file, &self.data_path, self.bounds)?;
        let physical_records = record_count_from_bytes(file_len)?;
        self.reset_scan_state(physical_records);

        let mut physical = 0_u64;
        while physical < physical_records {
            let data_raw = read_record_at(&mut self.data_file, physical)?;
            let data = decode_data(&data_raw, physical)?;
            let prefix_count = data
                .rekey_count
                .checked_add(1)
                .ok_or_else(|| "Finalization V2 prefix record count overflowed".to_owned())?;
            let block_count = prefix_count
                .checked_add(1)
                .ok_or_else(|| "Finalization V2 block record count overflowed".to_owned())?;
            self.require_block_bound(block_count)?;
            let available = physical_records
                .checked_sub(physical)
                .ok_or_else(|| "Finalization V2 physical availability underflowed".to_owned())?;
            if available < prefix_count {
                return Err(format!(
                    "Finalization V2 trailing block at record {physical} is torn: it declares {prefix_count} Data/Rekey records but only {available} remain"
                ));
            }
            let read_count = if available >= block_count {
                block_count
            } else {
                prefix_count
            };
            let mut raw_records =
                read_record_range(&mut self.data_file, physical, read_count, self.bounds)?;
            let first_raw = raw_records.first_mut().ok_or_else(|| {
                "Finalization V2 bounded range unexpectedly omitted its Data record".to_owned()
            })?;
            *first_raw = data_raw;

            if read_count == prefix_count {
                let prepared = decode_prepared_prefix(&raw_records, physical)?;
                let expected = prepared.records(self.completed_blocks, physical)?;
                if expected.get(..raw_records.len()) != Some(raw_records.as_slice()) {
                    return Err(
                        "Finalization V2 trailing prepared block does not reproduce byte-for-byte"
                            .to_owned(),
                    );
                }
                self.trailing = Some(TrailingPreparedBlockV2 {
                    first_physical_record: physical,
                    block_sequence: self.completed_blocks,
                    prepared,
                });
                physical = physical_records;
                continue;
            }

            let completion_physical = physical
                .checked_add(block_count - 1)
                .ok_or_else(|| "Finalization V2 Completion physical index overflowed".to_owned())?;
            let completion = decode_completion(
                raw_records
                    .last()
                    .ok_or_else(|| "Finalization V2 Completion record is absent".to_owned())?,
                completion_physical,
            )?;
            if completion.block_sequence != self.completed_blocks {
                return Err(format!(
                    "Finalization V2 block sequence {} is not canonical {}",
                    completion.block_sequence, self.completed_blocks
                ));
            }
            let prepared = decode_block(&raw_records, self.completed_blocks, physical)?;
            let authority = structural_receipt_from_block(
                &prepared,
                completion,
                physical,
                raw_records
                    .last()
                    .ok_or_else(|| "Finalization V2 Completion record is absent".to_owned())?,
            )?;
            self.receipts
                .try_reserve(1)
                .map_err(|why| format!("cannot reserve Finalization V2 receipt index: {why}"))?;
            if self
                .receipts
                .insert(authority.finalization_id, authority)
                .is_some()
            {
                return Err(format!(
                    "Finalization V2 identity {} appears more than once",
                    hex32(authority.finalization_id)
                ));
            }
            self.completed_blocks = self
                .completed_blocks
                .checked_add(1)
                .ok_or_else(|| "Finalization V2 completed-block count overflowed".to_owned())?;
            physical = physical
                .checked_add(block_count)
                .ok_or_else(|| "Finalization V2 scan cursor overflowed".to_owned())?;
        }
        self.require_unchanged()
    }

    fn reset_scan_state(&mut self, physical_records: u64) {
        self.receipts = HashMap::new();
        self.trailing = None;
        self.completed_blocks = 0;
        self.physical_records = physical_records;
    }

    /// Revalidates the bounded file generation, then performs one average-O(1)
    /// in-memory identity probe.  Absence is `Ok(None)`.
    ///
    /// # Complexity
    ///
    /// Content-generation validation is O(file bytes); only the subsequent
    /// hash-table probe is average O(1).
    ///
    /// # Errors
    ///
    /// Refuses lock failure, same-length mutation, append, path replacement or
    /// any generation change since open.
    pub fn reopen_structural_receipt(
        &self,
        finalization_id: &[u8; 32],
    ) -> Result<Option<PopulationFinalizationV2StructuralReceipt>, PopulationFinalizationV2Refusal>
    {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take shared Finalization V2 lookup lock: {why}"))?;
        let result = self
            .require_unchanged()
            .map(|()| self.receipts.get(finalization_id).copied());
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release shared Finalization V2 lookup lock: {why}"));
        match (result, released) {
            (Ok(authority), Ok(())) => Ok(authority),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "authentication remains dormant until upstream lineage authority can construct an opaque preparation"
        )
    )]
    fn authenticate_structural_receipt(
        &mut self,
        receipt: PopulationFinalizationV2StructuralReceipt,
        prepared: &PreparedPopulationFinalizationV2,
    ) -> Result<PopulationFinalizationV2Authority, PopulationFinalizationV2Refusal> {
        self.lock_file.lock_shared().map_err(|why| {
            format!("cannot take shared Finalization V2 authentication lock: {why}")
        })?;
        let result = (|| {
            let prepared_records = prepared.checked_physical_record_count()?;
            let prepared_rekeys = prepared_records
                .checked_sub(2)
                .ok_or_else(|| "Finalization V2 prepared record count underflowed".to_owned())?;
            let receipt_records = receipt.record_count()?;
            self.require_block_bound(receipt_records)?;
            self.require_block_bound(prepared_records)?;
            if receipt.rekey_count != prepared_rekeys || receipt_records != prepared_records {
                return Err(format!(
                    "Finalization V2 structural receipt has {receipt_records} physical records and {} rekeys, but the opaque preparation has {prepared_records} physical records and {prepared_rekeys} rekeys",
                    receipt.rekey_count
                ));
            }
            self.require_unchanged()?;
            if self.receipts.get(&receipt.finalization_id) != Some(&receipt)
                || prepared.finalization_id() != receipt.finalization_id
            {
                return Err(
                    "Finalization V2 structural receipt is not the indexed opaque preparation identity"
                        .to_owned(),
                );
            }
            let observed = read_record_range(
                &mut self.data_file,
                receipt.first_physical_record,
                receipt_records,
                self.bounds,
            )?;
            let expected =
                prepared.records(receipt.block_sequence, receipt.first_physical_record)?;
            if observed != expected {
                return Err(
                    "Finalization V2 structural bytes do not exactly authenticate against the opaque preparation"
                        .to_owned(),
                );
            }
            self.require_unchanged()?;
            Ok(PopulationFinalizationV2Authority { receipt })
        })();
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Finalization V2 authentication lock: {why}"));
        match (result, released) {
            (Ok(authority), Ok(())) => Ok(authority),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "append remains dormant until upstream lineage authority is constructible"
        )
    )]
    fn append(
        &mut self,
        prepared: &PreparedPopulationFinalizationV2,
    ) -> Result<PopulationFinalizationV2StructuralCommit, PopulationFinalizationV2Refusal> {
        if !self.writable {
            return Err("Finalization V2 ledger was opened read-only".to_owned());
        }
        self.lock_file
            .lock()
            .map_err(|why| format!("cannot take Finalization V2 append lock: {why}"))?;
        let result = self.append_locked(prepared).and_then(|commit| {
            self.require_unchanged()?;
            Ok(commit)
        });
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release Finalization V2 append lock: {why}"));
        match (result, released) {
            (Ok(commit), Ok(())) => Ok(commit),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "locked append remains dormant until upstream lineage authority is constructible"
        )
    )]
    fn append_locked(
        &mut self,
        prepared: &PreparedPopulationFinalizationV2,
    ) -> Result<PopulationFinalizationV2StructuralCommit, PopulationFinalizationV2Refusal> {
        self.require_unchanged()?;
        self.require_prepared_block_bound(prepared)?;
        prepared.validate()?;
        let finalization_id = prepared.finalization_id();
        if let Some(existing) = self.receipts.get(&finalization_id).copied() {
            return self.reuse_existing(prepared, existing);
        }

        self.receipts
            .try_reserve(1)
            .map_err(|why| format!("cannot reserve Finalization V2 append index: {why}"))?;

        if let Some(trailing) = self.trailing.clone() {
            return self.complete_trailing(prepared, finalization_id, &trailing);
        }

        self.append_new_block(prepared, finalization_id)
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "exact reuse remains dormant until upstream lineage authority is constructible"
        )
    )]
    fn reuse_existing(
        &mut self,
        prepared: &PreparedPopulationFinalizationV2,
        existing: PopulationFinalizationV2StructuralReceipt,
    ) -> Result<PopulationFinalizationV2StructuralCommit, PopulationFinalizationV2Refusal> {
        self.require_prepared_block_bound(prepared)?;
        let observed = read_record_range(
            &mut self.data_file,
            existing.first_physical_record,
            existing.record_count()?,
            self.bounds,
        )?;
        let expected = prepared.records(existing.block_sequence, existing.first_physical_record)?;
        if observed != expected {
            return Err(format!(
                "Finalization V2 identity {} is complete with different exact bytes",
                hex32(existing.finalization_id)
            ));
        }
        self.require_unchanged()?;
        Ok(PopulationFinalizationV2StructuralCommit::Reused(existing))
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "orphan completion remains dormant until upstream lineage authority is constructible"
        )
    )]
    fn complete_trailing(
        &mut self,
        prepared: &PreparedPopulationFinalizationV2,
        finalization_id: [u8; 32],
        trailing: &TrailingPreparedBlockV2,
    ) -> Result<PopulationFinalizationV2StructuralCommit, PopulationFinalizationV2Refusal> {
        self.require_prepared_block_bound(prepared)?;
        if trailing.prepared != *prepared {
            return Err(format!(
                "Finalization V2 trailing prepared block belongs to {}, not exact retry {}",
                hex32(trailing.prepared.finalization_id()),
                hex32(finalization_id)
            ));
        }
        self.require_append_bound(1)?;
        let expected = prepared.records(trailing.block_sequence, trailing.first_physical_record)?;
        let observed = read_record_range(
            &mut self.data_file,
            trailing.first_physical_record,
            prepared
                .rekey_count()
                .checked_add(1)
                .ok_or_else(|| "Finalization V2 orphan prefix count overflowed".to_owned())?,
            self.bounds,
        )?;
        if expected.get(..observed.len()) != Some(observed.as_slice()) {
            return Err(
                "Finalization V2 trailing prepared bytes changed before exact retry".to_owned(),
            );
        }
        let completion_raw = expected
            .last()
            .ok_or_else(|| "Finalization V2 retry Completion is absent".to_owned())?;
        #[cfg(test)]
        self.run_before_completion_test_hook()?;
        self.require_unchanged()?;
        append_raw_record(&mut self.data_file, completion_raw)?;
        self.sync_completion()?;
        let completion = decode_completion(completion_raw, self.physical_records - 1)?;
        let authority = structural_receipt_from_block(
            prepared,
            completion,
            trailing.first_physical_record,
            completion_raw,
        )?;
        self.finish_indexed_append(finalization_id, authority, "exact retry")?;
        self.require_unchanged()?;
        Ok(PopulationFinalizationV2StructuralCommit::Written(authority))
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "new block append remains dormant until upstream lineage authority is constructible"
        )
    )]
    fn append_new_block(
        &mut self,
        prepared: &PreparedPopulationFinalizationV2,
        finalization_id: [u8; 32],
    ) -> Result<PopulationFinalizationV2StructuralCommit, PopulationFinalizationV2Refusal> {
        let block_count = self.require_prepared_block_bound(prepared)?;
        self.require_append_bound(block_count)?;
        let first_physical_record = self.physical_records;
        let block_sequence = self.completed_blocks;
        let expected = prepared.records(block_sequence, first_physical_record)?;
        let prefix_len = expected
            .len()
            .checked_sub(1)
            .ok_or_else(|| "Finalization V2 append block lacks Completion".to_owned())?;
        for raw in expected
            .get(..prefix_len)
            .ok_or_else(|| "Finalization V2 Data/Rekey prefix is absent".to_owned())?
        {
            append_raw_record(&mut self.data_file, raw)?;
        }
        self.data_file
            .sync_data()
            .map_err(|why| format!("cannot sync Finalization V2 Data/Rekeys: {why}"))?;
        self.physical_records = self
            .physical_records
            .checked_add(
                u64::try_from(prefix_len)
                    .map_err(|_| "Finalization V2 prefix length does not fit u64".to_owned())?,
            )
            .ok_or_else(|| "Finalization V2 prefix record count overflowed".to_owned())?;
        self.trailing = Some(TrailingPreparedBlockV2 {
            first_physical_record,
            block_sequence,
            prepared: prepared.clone(),
        });
        self.refresh_data_generation_after_owned_write(self.physical_records)?;

        let completion_raw = expected
            .last()
            .ok_or_else(|| "Finalization V2 append Completion is absent".to_owned())?;
        #[cfg(test)]
        self.run_before_completion_test_hook()?;
        self.require_unchanged()?;
        append_raw_record(&mut self.data_file, completion_raw)?;
        self.sync_completion()?;
        let completion = decode_completion(completion_raw, self.physical_records - 1)?;
        let authority = structural_receipt_from_block(
            prepared,
            completion,
            first_physical_record,
            completion_raw,
        )?;
        self.finish_indexed_append(finalization_id, authority, "append")?;
        self.require_unchanged()?;
        Ok(PopulationFinalizationV2StructuralCommit::Written(authority))
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Completion sync remains dormant until upstream lineage authority is constructible"
        )
    )]
    fn sync_completion(&mut self) -> Result<(), PopulationFinalizationV2Refusal> {
        self.data_file
            .sync_data()
            .map_err(|why| format!("cannot sync Finalization V2 Completion: {why}"))?;
        self.physical_records = self
            .physical_records
            .checked_add(1)
            .ok_or_else(|| "Finalization V2 physical record count overflowed".to_owned())?;
        self.refresh_data_generation_after_owned_write(self.physical_records)
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "index finalization remains dormant until upstream lineage authority is constructible"
        )
    )]
    fn finish_indexed_append(
        &mut self,
        finalization_id: [u8; 32],
        authority: PopulationFinalizationV2StructuralReceipt,
        operation: &str,
    ) -> Result<(), PopulationFinalizationV2Refusal> {
        if self.receipts.insert(finalization_id, authority).is_some() {
            return Err(format!(
                "Finalization V2 {operation} collided with indexed identity"
            ));
        }
        self.completed_blocks = self
            .completed_blocks
            .checked_add(1)
            .ok_or_else(|| "Finalization V2 completed-block count overflowed".to_owned())?;
        self.trailing = None;
        Ok(())
    }

    fn require_block_bound(
        &self,
        block_records: u64,
    ) -> Result<(), PopulationFinalizationV2Refusal> {
        if block_records > self.bounds.max_records {
            return Err(format!(
                "Finalization V2 block has {block_records} records, above explicit maximum {}",
                self.bounds.max_records
            ));
        }
        let bytes = block_records
            .checked_mul(POPULATION_FINALIZATION_V2_RECORD_BYTES as u64)
            .ok_or_else(|| "Finalization V2 block bytes overflowed".to_owned())?;
        if bytes > self.bounds.max_file_bytes {
            return Err(format!(
                "Finalization V2 block has {bytes} bytes, above explicit maximum {}",
                self.bounds.max_file_bytes
            ));
        }
        Ok(())
    }

    /// Checks an opaque preparation's actual vector length before any
    /// semantic validation, duplicate set or encoded-record allocation.
    fn require_prepared_block_bound(
        &self,
        prepared: &PreparedPopulationFinalizationV2,
    ) -> Result<u64, PopulationFinalizationV2Refusal> {
        let block_records = prepared.checked_physical_record_count()?;
        self.require_block_bound(block_records)?;
        Ok(block_records)
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "append bounds remain dormant until upstream lineage authority is constructible"
        )
    )]
    fn require_append_bound(
        &self,
        added_records: u64,
    ) -> Result<(), PopulationFinalizationV2Refusal> {
        let desired_records = self
            .physical_records
            .checked_add(added_records)
            .ok_or_else(|| "Finalization V2 desired record count overflowed".to_owned())?;
        if desired_records > self.bounds.max_records {
            return Err(format!(
                "Finalization V2 append would produce {desired_records} records, above explicit maximum {}",
                self.bounds.max_records
            ));
        }
        let desired_bytes = desired_records
            .checked_mul(POPULATION_FINALIZATION_V2_RECORD_BYTES as u64)
            .ok_or_else(|| "Finalization V2 desired byte count overflowed".to_owned())?;
        if desired_bytes > self.bounds.max_file_bytes {
            return Err(format!(
                "Finalization V2 append would produce {desired_bytes} bytes, above explicit maximum {}",
                self.bounds.max_file_bytes
            ));
        }
        Ok(())
    }

    fn require_unchanged(&self) -> Result<(), PopulationFinalizationV2Refusal> {
        require_directory_generation(&self.root_generation, &self.root_file, &self.root_path)?;
        require_file_generation(self.lock_generation, &self.lock_file, &self.lock_path, 0)?;
        require_file_generation(
            self.data_generation,
            &self.data_file,
            &self.data_path,
            self.bounds.max_file_bytes,
        )
    }

    fn require_root_and_lock_unchanged(&self) -> Result<(), PopulationFinalizationV2Refusal> {
        require_directory_generation(&self.root_generation, &self.root_file, &self.root_path)?;
        require_file_generation(self.lock_generation, &self.lock_file, &self.lock_path, 0)
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "owned-write generation refresh remains dormant until upstream lineage authority is constructible"
        )
    )]
    fn refresh_data_generation_after_owned_write(
        &mut self,
        expected_records: u64,
    ) -> Result<(), PopulationFinalizationV2Refusal> {
        self.require_root_and_lock_unchanged()?;
        let observed =
            file_generation(&self.data_file, &self.data_path, self.bounds.max_file_bytes)?;
        if !self
            .data_generation
            .platform
            .same_file_object(observed.platform)
        {
            return Err(format!(
                "Finalization V2 data file {} was replaced during owned append",
                self.data_path.display()
            ));
        }
        let expected_bytes = expected_records
            .checked_mul(POPULATION_FINALIZATION_V2_RECORD_BYTES as u64)
            .ok_or_else(|| "Finalization V2 owned append byte count overflowed".to_owned())?;
        if observed.len != expected_bytes {
            return Err(format!(
                "Finalization V2 owned append produced {} bytes, expected {expected_bytes}",
                observed.len
            ));
        }
        self.data_generation = observed;
        self.require_unchanged()
    }

    fn generation_witness(&self) -> LedgerGenerationWitnessV2 {
        LedgerGenerationWitnessV2 {
            root: self.root_generation.clone(),
            lock: self.lock_generation,
            data: self.data_generation,
        }
    }

    fn require_generation_witness(
        &self,
        expected: &LedgerGenerationWitnessV2,
    ) -> Result<(), PopulationFinalizationV2Refusal> {
        self.require_unchanged()?;
        let observed = self.generation_witness();
        if observed != *expected {
            return Err(
                "Finalization V2 fresh reopen changed root/lock/data generation".to_owned(),
            );
        }
        Ok(())
    }

    #[cfg(test)]
    fn run_before_completion_test_hook(&mut self) -> Result<(), PopulationFinalizationV2Refusal> {
        if let Some(hook) = self.before_completion_test_hook.take() {
            hook()?;
        }
        Ok(())
    }
}

/// Persists one already-opaque preparation, drops the writer and independently
/// reopens the exact receipt read-only before returning Written or Reused.
///
/// This is crate-private and cannot mint a preparation.  Until an upstream
/// freshly reopened CLI search-lineage constructor exists, production callers
/// have no value with which to invoke it.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "production persistence remains intentionally unreachable until upstream lineage authority is constructible"
    )
)]
pub(crate) fn persist_population_finalization_v2(
    root: impl AsRef<Path>,
    bounds: PopulationFinalizationV2Bounds,
    prepared: &PreparedPopulationFinalizationV2,
) -> Result<PopulationFinalizationV2Commit, PopulationFinalizationV2Refusal> {
    let finalization_id = prepared.finalization_id();
    let mut writer = PopulationFinalizationV2Ledger::open_write(root.as_ref(), bounds)?;
    let first = writer.append(prepared)?;
    writer.require_unchanged()?;
    let expected = first.receipt();
    let generation_witness = writer.generation_witness();
    drop(writer);

    let mut reopened = PopulationFinalizationV2Ledger::open_read(root.as_ref(), bounds)?;
    reopened.require_generation_witness(&generation_witness)?;
    let actual = reopened
        .reopen_structural_receipt(&finalization_id)?
        .ok_or_else(|| {
            "Finalization V2 fresh reopen lost the just-committed identity".to_owned()
        })?;
    if actual != expected {
        return Err(
            "Finalization V2 fresh reopen structural receipt differs from the writer result"
                .to_owned(),
        );
    }
    let authority = reopened.authenticate_structural_receipt(actual, prepared)?;
    reopened.require_generation_witness(&generation_witness)?;
    Ok(match first {
        PopulationFinalizationV2StructuralCommit::Written(_) => {
            PopulationFinalizationV2Commit::Written(authority)
        }
        PopulationFinalizationV2StructuralCommit::Reused(_) => {
            PopulationFinalizationV2Commit::Reused(authority)
        }
    })
}

fn encode_data(
    value: &PopulationFinalizationDataV2,
    physical_sequence: u64,
) -> Result<[u8; POPULATION_FINALIZATION_V2_RECORD_BYTES], PopulationFinalizationV2Refusal> {
    value.validate()?;
    encode_record(
        RecordDomainV2::Data,
        physical_sequence,
        value.finalization_id,
        |writer| value.encode_body(writer),
    )
}

fn decode_data(
    raw: &[u8; POPULATION_FINALIZATION_V2_RECORD_BYTES],
    physical_sequence: u64,
) -> Result<PopulationFinalizationDataV2, PopulationFinalizationV2Refusal> {
    let (finalization_id, mut reader) =
        decode_record(raw, RecordDomainV2::Data, physical_sequence)?;
    let value = PopulationFinalizationDataV2::decode_body(finalization_id, &mut reader)?;
    reader.require_remaining_zero("Data trailing reserve")?;
    value.validate()?;
    Ok(value)
}

fn encode_rekey(
    value: &PopulationFinalizationRekeyV2,
    physical_sequence: u64,
) -> Result<[u8; POPULATION_FINALIZATION_V2_RECORD_BYTES], PopulationFinalizationV2Refusal> {
    value.validate()?;
    encode_record(
        RecordDomainV2::Rekey,
        physical_sequence,
        value.finalization_id,
        |writer| value.encode_body(writer),
    )
}

fn decode_rekey(
    raw: &[u8; POPULATION_FINALIZATION_V2_RECORD_BYTES],
    physical_sequence: u64,
) -> Result<PopulationFinalizationRekeyV2, PopulationFinalizationV2Refusal> {
    let (finalization_id, mut reader) =
        decode_record(raw, RecordDomainV2::Rekey, physical_sequence)?;
    let value = PopulationFinalizationRekeyV2::decode_body(finalization_id, &mut reader)?;
    reader.require_remaining_zero("Rekey trailing reserve")?;
    value.validate()?;
    Ok(value)
}

fn encode_completion(
    value: PopulationFinalizationCompletionV2,
    physical_sequence: u64,
) -> Result<[u8; POPULATION_FINALIZATION_V2_RECORD_BYTES], PopulationFinalizationV2Refusal> {
    value.validate()?;
    encode_record(
        RecordDomainV2::Completion,
        physical_sequence,
        value.finalization_id,
        |writer| value.encode_body(writer),
    )
}

fn decode_completion(
    raw: &[u8; POPULATION_FINALIZATION_V2_RECORD_BYTES],
    physical_sequence: u64,
) -> Result<PopulationFinalizationCompletionV2, PopulationFinalizationV2Refusal> {
    let (finalization_id, mut reader) =
        decode_record(raw, RecordDomainV2::Completion, physical_sequence)?;
    let value = PopulationFinalizationCompletionV2::decode_body(finalization_id, &mut reader)?;
    reader.require_remaining_zero("Completion trailing reserve")?;
    value.validate()?;
    Ok(value)
}

fn decode_block(
    records: &[[u8; POPULATION_FINALIZATION_V2_RECORD_BYTES]],
    block_sequence: u64,
    first_physical_record: u64,
) -> Result<PreparedPopulationFinalizationV2, PopulationFinalizationV2Refusal> {
    if records.len() < 2 {
        return Err("Finalization block lacks Data or Completion".to_owned());
    }
    let data = decode_data(
        records
            .first()
            .ok_or_else(|| "Finalization Data record is absent".to_owned())?,
        first_physical_record,
    )?;
    let rekey_len = records
        .len()
        .checked_sub(2)
        .ok_or_else(|| "Finalization record count underflowed".to_owned())?;
    if u64::try_from(rekey_len).map_err(|_| "rekey length does not fit u64".to_owned())?
        != data.rekey_count
    {
        return Err("Finalization record count differs from Data rekey count".to_owned());
    }
    let mut rekeys = Vec::new();
    rekeys
        .try_reserve_exact(rekey_len)
        .map_err(|why| format!("cannot reserve decoded Finalization rekeys: {why}"))?;
    for (offset, raw) in records
        .get(1..=rekey_len)
        .ok_or_else(|| "Finalization rekey record range is absent".to_owned())?
        .iter()
        .enumerate()
    {
        let physical = first_physical_record
            .checked_add(
                u64::try_from(offset)
                    .map_err(|_| "rekey offset does not fit u64".to_owned())?
                    .checked_add(1)
                    .ok_or_else(|| "rekey offset overflowed".to_owned())?,
            )
            .ok_or_else(|| "rekey physical sequence overflowed".to_owned())?;
        rekeys.push(decode_rekey(raw, physical)?);
    }
    let prepared = PreparedPopulationFinalizationV2 { data, rekeys };
    prepared.validate()?;
    let completion_physical = first_physical_record
        .checked_add(
            u64::try_from(records.len() - 1)
                .map_err(|_| "Completion offset does not fit u64".to_owned())?,
        )
        .ok_or_else(|| "Completion physical sequence overflowed".to_owned())?;
    let completion = decode_completion(
        records
            .last()
            .ok_or_else(|| "Finalization Completion record is absent".to_owned())?,
        completion_physical,
    )?;
    if completion.block_sequence != block_sequence {
        return Err("Finalization Completion block sequence is foreign".to_owned());
    }
    let expected = prepared.records(block_sequence, first_physical_record)?;
    if expected.as_slice() != records {
        return Err(
            "Finalization Data/rekeys/Completion do not reproduce byte-for-byte".to_owned(),
        );
    }
    Ok(prepared)
}

fn encode_record(
    domain: RecordDomainV2,
    physical_sequence: u64,
    finalization_id: [u8; 32],
    body: impl FnOnce(&mut FixedWriter<'_>) -> Result<(), PopulationFinalizationV2Refusal>,
) -> Result<[u8; POPULATION_FINALIZATION_V2_RECORD_BYTES], PopulationFinalizationV2Refusal> {
    require_nonzero("record Finalization identity", finalization_id)?;
    let mut raw = [0_u8; POPULATION_FINALIZATION_V2_RECORD_BYTES];
    {
        let payload = raw
            .get_mut(..PAYLOAD_BYTES)
            .ok_or_else(|| "Finalization payload slot is absent".to_owned())?;
        let mut writer = FixedWriter::new(payload);
        writer.bytes(&MAGIC)?;
        writer.u32(VERSION)?;
        writer.u32(domain.value())?;
        writer.u64(physical_sequence)?;
        writer.bytes(&finalization_id)?;
        body(&mut writer)?;
        writer.zero_remaining();
    }
    let seal = record_seal(
        raw.get(..PAYLOAD_BYTES)
            .ok_or_else(|| "Finalization payload is absent".to_owned())?,
    );
    raw.get_mut(PAYLOAD_BYTES..)
        .ok_or_else(|| "Finalization seal slot is absent".to_owned())?
        .copy_from_slice(&seal);
    Ok(raw)
}

fn decode_record(
    raw: &[u8; POPULATION_FINALIZATION_V2_RECORD_BYTES],
    expected_domain: RecordDomainV2,
    expected_physical_sequence: u64,
) -> Result<([u8; 32], FixedReader<'_>), PopulationFinalizationV2Refusal> {
    let payload = raw
        .get(..PAYLOAD_BYTES)
        .ok_or_else(|| "Finalization payload is absent".to_owned())?;
    let stored_seal = raw
        .get(PAYLOAD_BYTES..)
        .ok_or_else(|| "Finalization record seal is absent".to_owned())?;
    if stored_seal != record_seal(payload) {
        return Err("Finalization record failed its complete domain-separated seal".to_owned());
    }
    let mut reader = FixedReader::new(payload);
    if reader.array::<16>()? != MAGIC {
        return Err("Finalization record magic is unknown".to_owned());
    }
    let version = reader.u32()?;
    if version != VERSION {
        return Err(format!(
            "Finalization record version {version} is unknown; expected {VERSION}"
        ));
    }
    let domain = reader.u32()?;
    if domain != expected_domain.value() {
        return Err(format!(
            "Finalization record domain {domain} is not expected {}",
            expected_domain.value()
        ));
    }
    let physical_sequence = reader.u64()?;
    if physical_sequence != expected_physical_sequence {
        return Err(format!(
            "Finalization record physical sequence {physical_sequence} is not expected {expected_physical_sequence}"
        ));
    }
    let finalization_id = reader.array()?;
    require_nonzero("decoded record Finalization identity", finalization_id)?;
    Ok((finalization_id, reader))
}

fn derive_finalization_id(
    data: &PopulationFinalizationDataV2,
    rekeys: &[PopulationFinalizationRekeyV2],
) -> Result<[u8; 32], PopulationFinalizationV2Refusal> {
    let mut hasher = Hasher::new();
    hasher.update(FINALIZATION_ID_DOMAIN);
    let mut body = [0_u8; PAYLOAD_BYTES];
    let mut writer = FixedWriter::new(&mut body);
    data.encode_body(&mut writer)?;
    writer.zero_remaining();
    hasher.update(&body);
    hasher.update(
        &u64::try_from(rekeys.len())
            .map_err(|_| "Finalization identity rekey count does not fit u64".to_owned())?
            .to_le_bytes(),
    );
    for row in rekeys.iter().copied() {
        let mut canonical = row;
        canonical.finalization_id = [0; 32];
        let mut row_body = [0_u8; PAYLOAD_BYTES];
        let mut row_writer = FixedWriter::new(&mut row_body);
        canonical.encode_body(&mut row_writer)?;
        row_writer.zero_remaining();
        hasher.update(&row_body);
    }
    Ok(hasher.finalize())
}

fn derive_completion_id(mut value: PopulationFinalizationCompletionV2) -> [u8; 32] {
    value.completion_id = [0; 32];
    let mut hasher = Hasher::new();
    hasher.update(COMPLETION_ID_DOMAIN);
    hasher.update(&value.finalization_id);
    hasher.update(&value.block_sequence.to_le_bytes());
    hasher.update(&value.data_record_digest);
    hasher.update(&value.rekey_count.to_le_bytes());
    hasher.update(&value.nifty_rekey_count.to_le_bytes());
    hasher.update(&value.banknifty_rekey_count.to_le_bytes());
    hasher.update(&value.ordered_rekey_digest);
    hasher.finalize()
}

fn raw_record_digest(raw: &[u8; POPULATION_FINALIZATION_V2_RECORD_BYTES]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(RAW_RECORD_DIGEST_DOMAIN);
    hasher.update(raw);
    hasher.finalize()
}

fn record_seal(payload: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(RECORD_SEAL_DOMAIN);
    hasher.update(payload);
    hasher.finalize()
}

fn encode_span(
    writer: &mut FixedWriter<'_>,
    span: RequestedSpanIdentityV1,
) -> Result<(), PopulationFinalizationV2Refusal> {
    writer.u16(span.from_year())?;
    writer.u8(span.from_month())?;
    writer.u8(0)?;
    writer.u16(span.to_year())?;
    writer.u8(span.to_month())?;
    writer.u8(0)
}

fn decode_span(
    reader: &mut FixedReader<'_>,
) -> Result<RequestedSpanIdentityV1, PopulationFinalizationV2Refusal> {
    let from_year = reader.u16()?;
    let from_month = reader.u8()?;
    if reader.u8()? != 0 {
        return Err("Finalization requested-span first reserve is nonzero".to_owned());
    }
    let to_year = reader.u16()?;
    let to_month = reader.u8()?;
    if reader.u8()? != 0 {
        return Err("Finalization requested-span last reserve is nonzero".to_owned());
    }
    RequestedSpanIdentityV1::new(from_year, from_month, to_year, to_month)
        .map_err(|why| format!("Finalization requested span is invalid: {why}"))
}

const fn family_byte(family: InstrumentFamilyV1) -> u8 {
    match family {
        InstrumentFamilyV1::Nifty => 1,
        InstrumentFamilyV1::BankNifty => 2,
    }
}

fn decode_family(value: u8) -> Result<InstrumentFamilyV1, PopulationFinalizationV2Refusal> {
    match value {
        1 => Ok(InstrumentFamilyV1::Nifty),
        2 => Ok(InstrumentFamilyV1::BankNifty),
        _ => Err(format!(
            "Finalization instrument-family byte {value} is unknown"
        )),
    }
}

fn require_nonzero(name: &str, value: [u8; 32]) -> Result<(), PopulationFinalizationV2Refusal> {
    if value == [0; 32] {
        Err(format!("{name} identity is all zero"))
    } else {
        Ok(())
    }
}

fn checked_sum(values: &[u64], name: &str) -> Result<u64, PopulationFinalizationV2Refusal> {
    values.iter().try_fold(0_u64, |total, value| {
        total
            .checked_add(*value)
            .ok_or_else(|| format!("{name} overflow u64"))
    })
}

struct FixedWriter<'a> {
    bytes: &'a mut [u8],
    cursor: usize,
}

impl<'a> FixedWriter<'a> {
    const fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), PopulationFinalizationV2Refusal> {
        let end = self
            .cursor
            .checked_add(value.len())
            .ok_or_else(|| "Finalization encoder cursor overflowed".to_owned())?;
        self.bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "Finalization fixed record is too small".to_owned())?
            .copy_from_slice(value);
        self.cursor = end;
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), PopulationFinalizationV2Refusal> {
        self.bytes(&[value])
    }

    fn u16(&mut self, value: u16) -> Result<(), PopulationFinalizationV2Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn u32(&mut self, value: u32) -> Result<(), PopulationFinalizationV2Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), PopulationFinalizationV2Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), PopulationFinalizationV2Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), PopulationFinalizationV2Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "Finalization zero-reserve cursor overflowed".to_owned())?;
        let target = self
            .bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "Finalization fixed reserve is outside record".to_owned())?;
        target.fill(0);
        self.cursor = end;
        Ok(())
    }

    fn zero_remaining(&mut self) {
        if let Some(remaining) = self.bytes.get_mut(self.cursor..) {
            remaining.fill(0);
        }
        self.cursor = self.bytes.len();
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

    fn take<const N: usize>(&mut self) -> Result<[u8; N], PopulationFinalizationV2Refusal> {
        let end = self
            .cursor
            .checked_add(N)
            .ok_or_else(|| "Finalization decoder cursor overflowed".to_owned())?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| "Finalization fixed record ended early".to_owned())?;
        self.cursor = end;
        value
            .try_into()
            .map_err(|_| "Finalization fixed-width decode failed".to_owned())
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], PopulationFinalizationV2Refusal> {
        self.take()
    }

    fn u8(&mut self) -> Result<u8, PopulationFinalizationV2Refusal> {
        Ok(self.take::<1>()?[0])
    }

    fn u16(&mut self) -> Result<u16, PopulationFinalizationV2Refusal> {
        Ok(u16::from_le_bytes(self.take()?))
    }

    fn u32(&mut self) -> Result<u32, PopulationFinalizationV2Refusal> {
        Ok(u32::from_le_bytes(self.take()?))
    }

    fn u64(&mut self) -> Result<u64, PopulationFinalizationV2Refusal> {
        Ok(u64::from_le_bytes(self.take()?))
    }

    fn i64(&mut self) -> Result<i64, PopulationFinalizationV2Refusal> {
        Ok(i64::from_le_bytes(self.take()?))
    }

    fn require_zeros(
        &mut self,
        count: usize,
        name: &str,
    ) -> Result<(), PopulationFinalizationV2Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| format!("{name} cursor overflowed"))?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| format!("{name} is outside fixed record"))?;
        if value.iter().any(|byte| *byte != 0) {
            return Err(format!("{name} is nonzero"));
        }
        self.cursor = end;
        Ok(())
    }

    fn require_remaining_zero(
        &mut self,
        name: &str,
    ) -> Result<(), PopulationFinalizationV2Refusal> {
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

fn open_root_directory(root: &Path) -> Result<File, PopulationFinalizationV2Refusal> {
    if matches!(
        std::fs::symlink_metadata(root),
        Err(ref why) if why.kind() == std::io::ErrorKind::NotFound
    ) {
        return Err(format!(
            "Finalization V2 root {} must already exist",
            root.display()
        ));
    }
    require_not_symlink(root, false)?;
    let root_file = File::open(root).map_err(|why| {
        format!(
            "Finalization V2 root {} must already exist: {why}",
            root.display()
        )
    })?;
    let metadata = root_file.metadata().map_err(|why| {
        format!(
            "cannot stat held Finalization V2 root {}: {why}",
            root.display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "Finalization V2 root {} is not a directory",
            root.display()
        ));
    }
    require_not_symlink(root, false)?;
    Ok(root_file)
}

fn sync_directory_entries(
    root_file: &File,
    root: &Path,
) -> Result<(), PopulationFinalizationV2Refusal> {
    root_file.sync_all().map_err(|why| {
        format!(
            "cannot durably sync Finalization V2 directory entries in {}: {why}",
            root.display()
        )
    })
}

fn require_regular_file(file: &File, path: &Path) -> Result<(), PopulationFinalizationV2Refusal> {
    let metadata = file
        .metadata()
        .map_err(|why| format!("cannot stat Finalization V2 file {}: {why}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!(
            "Finalization V2 path {} is not a regular file",
            path.display()
        ));
    }
    Ok(())
}

fn open_ledger_file(
    path: &Path,
    writable: bool,
    create: bool,
) -> Result<File, PopulationFinalizationV2Refusal> {
    // macOS, Linux and Android provide a safe `OpenOptionsExt` surface for
    // refusing a final-component symbolic link atomically.  Safe `std` has no
    // portable open-at-relative-to-an-already-held-directory primitive.  On
    // other targets the pre/post `symlink_metadata` checks below reject a
    // preplaced link, but a concurrent pathname swap remains an explicit
    // residual TOCTOU limitation.  Production preparation stays unavailable;
    // this module must not upgrade that weaker structural admission into an
    // authenticated-authority claim.
    if !create
        && matches!(
            std::fs::symlink_metadata(path),
            Err(ref why) if why.kind() == std::io::ErrorKind::NotFound
        )
    {
        return Err(format!(
            "cannot open Finalization V2 file {}: path is absent",
            path.display()
        ));
    }
    require_not_symlink(path, create)?;
    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(writable)
        .create(create)
        .truncate(false);
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    options.custom_flags(O_NOFOLLOW_FLAG);
    let file = options
        .open(path)
        .map_err(|why| format!("cannot open Finalization V2 file {}: {why}", path.display()))?;
    require_not_symlink(path, false)?;
    Ok(file)
}

fn require_not_symlink(
    path: &Path,
    absent_is_allowed: bool,
) -> Result<(), PopulationFinalizationV2Refusal> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(format!(
            "Finalization V2 path {} is a symbolic link; no-follow admission refused it",
            path.display()
        )),
        Ok(_) => Ok(()),
        Err(why) if absent_is_allowed && why.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(why) => Err(format!(
            "cannot inspect Finalization V2 path {} without following links: {why}",
            path.display()
        )),
    }
}

fn checked_file_len(
    file: &File,
    path: &Path,
    bounds: PopulationFinalizationV2Bounds,
) -> Result<u64, PopulationFinalizationV2Refusal> {
    let len = file
        .metadata()
        .map_err(|why| format!("cannot stat Finalization V2 file {}: {why}", path.display()))?
        .len();
    if len > bounds.max_file_bytes {
        return Err(format!(
            "Finalization V2 file {} has {len} bytes, above explicit maximum {}",
            path.display(),
            bounds.max_file_bytes
        ));
    }
    let records = record_count_from_bytes(len)?;
    if records > bounds.max_records {
        return Err(format!(
            "Finalization V2 file {} has {records} records, above explicit maximum {}",
            path.display(),
            bounds.max_records
        ));
    }
    Ok(len)
}

fn record_count_from_bytes(bytes: u64) -> Result<u64, PopulationFinalizationV2Refusal> {
    let stride = POPULATION_FINALIZATION_V2_RECORD_BYTES as u64;
    if !bytes.is_multiple_of(stride) {
        return Err(format!(
            "Finalization V2 file has {bytes} bytes, ragged against {stride}-byte records"
        ));
    }
    Ok(bytes / stride)
}

fn record_offset(index: u64) -> Result<u64, PopulationFinalizationV2Refusal> {
    index
        .checked_mul(POPULATION_FINALIZATION_V2_RECORD_BYTES as u64)
        .ok_or_else(|| "Finalization V2 record offset overflowed".to_owned())
}

fn read_record_at(
    file: &mut File,
    index: u64,
) -> Result<[u8; POPULATION_FINALIZATION_V2_RECORD_BYTES], PopulationFinalizationV2Refusal> {
    let mut raw = [0_u8; POPULATION_FINALIZATION_V2_RECORD_BYTES];
    file.seek(SeekFrom::Start(record_offset(index)?))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| format!("cannot read Finalization V2 record {index}: {why}"))?;
    Ok(raw)
}

fn read_record_range(
    file: &mut File,
    first: u64,
    count: u64,
    bounds: PopulationFinalizationV2Bounds,
) -> Result<Vec<[u8; POPULATION_FINALIZATION_V2_RECORD_BYTES]>, PopulationFinalizationV2Refusal> {
    if count > bounds.max_records {
        return Err(format!(
            "Finalization V2 read asks for {count} records, above explicit maximum {}",
            bounds.max_records
        ));
    }
    let bytes = count
        .checked_mul(POPULATION_FINALIZATION_V2_RECORD_BYTES as u64)
        .ok_or_else(|| "Finalization V2 requested read bytes overflowed".to_owned())?;
    if bytes > bounds.max_file_bytes {
        return Err(format!(
            "Finalization V2 read asks for {bytes} bytes, above explicit maximum {}",
            bounds.max_file_bytes
        ));
    }
    first
        .checked_add(count)
        .ok_or_else(|| "Finalization V2 requested record range overflowed".to_owned())?;
    let capacity = usize::try_from(count)
        .map_err(|_| "Finalization V2 requested record count does not fit usize".to_owned())?;
    let mut records = Vec::new();
    records
        .try_reserve_exact(capacity)
        .map_err(|why| format!("cannot reserve Finalization V2 record read: {why}"))?;
    for offset in 0..count {
        let physical = first
            .checked_add(offset)
            .ok_or_else(|| "Finalization V2 physical read index overflowed".to_owned())?;
        records.push(read_record_at(file, physical)?);
    }
    Ok(records)
}

fn decode_prepared_prefix(
    records: &[[u8; POPULATION_FINALIZATION_V2_RECORD_BYTES]],
    first_physical_record: u64,
) -> Result<PreparedPopulationFinalizationV2, PopulationFinalizationV2Refusal> {
    let data_raw = records
        .first()
        .ok_or_else(|| "Finalization V2 prepared prefix lacks Data".to_owned())?;
    let data = decode_data(data_raw, first_physical_record)?;
    let expected_len = data
        .rekey_count
        .checked_add(1)
        .ok_or_else(|| "Finalization V2 prepared prefix length overflowed".to_owned())?;
    if u64::try_from(records.len())
        .map_err(|_| "Finalization V2 prepared prefix length does not fit u64".to_owned())?
        != expected_len
    {
        return Err(format!(
            "Finalization V2 prepared prefix has {} records, expected {expected_len}",
            records.len()
        ));
    }
    let capacity = usize::try_from(data.rekey_count)
        .map_err(|_| "Finalization V2 Rekey count does not fit usize".to_owned())?;
    let mut rekeys = Vec::new();
    rekeys
        .try_reserve_exact(capacity)
        .map_err(|why| format!("cannot reserve Finalization V2 prepared Rekeys: {why}"))?;
    for (offset, raw) in records.iter().skip(1).enumerate() {
        let physical = first_physical_record
            .checked_add(
                u64::try_from(offset)
                    .map_err(|_| "Finalization V2 Rekey offset does not fit u64".to_owned())?
                    .checked_add(1)
                    .ok_or_else(|| "Finalization V2 Rekey offset overflowed".to_owned())?,
            )
            .ok_or_else(|| "Finalization V2 Rekey physical index overflowed".to_owned())?;
        rekeys.push(decode_rekey(raw, physical)?);
    }
    let prepared = PreparedPopulationFinalizationV2 { data, rekeys };
    prepared.validate()?;
    Ok(prepared)
}

fn structural_receipt_from_block(
    prepared: &PreparedPopulationFinalizationV2,
    completion: PopulationFinalizationCompletionV2,
    first_physical_record: u64,
    completion_raw: &[u8; POPULATION_FINALIZATION_V2_RECORD_BYTES],
) -> Result<PopulationFinalizationV2StructuralReceipt, PopulationFinalizationV2Refusal> {
    if completion.finalization_id != prepared.finalization_id()
        || completion.rekey_count != prepared.rekey_count()
        || completion.nifty_rekey_count != prepared.data.nifty.row_count
        || completion.banknifty_rekey_count != prepared.data.banknifty.row_count
    {
        return Err("Finalization V2 Completion crosswires its prepared block".to_owned());
    }
    Ok(PopulationFinalizationV2StructuralReceipt {
        finalization_id: completion.finalization_id,
        block_sequence: completion.block_sequence,
        first_physical_record,
        rekey_count: completion.rekey_count,
        data_record_digest: completion.data_record_digest,
        completion_id: completion.completion_id,
        completion_record_digest: raw_record_digest(completion_raw),
    })
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "raw append remains dormant until upstream lineage authority is constructible"
    )
)]
fn append_raw_record(
    file: &mut File,
    raw: &[u8; POPULATION_FINALIZATION_V2_RECORD_BYTES],
) -> Result<(), PopulationFinalizationV2Refusal> {
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(raw))
        .map_err(|why| format!("cannot append Finalization V2 record: {why}"))
}

fn directory_generation(
    held: &File,
    path: &Path,
) -> Result<DirectoryGenerationV2, PopulationFinalizationV2Refusal> {
    require_not_symlink(path, false)?;
    let canonical_path = std::fs::canonicalize(path).map_err(|why| {
        format!(
            "cannot canonicalize Finalization V2 root {}: {why}",
            path.display()
        )
    })?;
    let held_before = held.metadata().map_err(|why| {
        format!(
            "cannot stat held Finalization V2 root {}: {why}",
            path.display()
        )
    })?;
    let named = File::open(path).map_err(|why| {
        format!(
            "cannot reopen named Finalization V2 root {}: {why}",
            path.display()
        )
    })?;
    let named_metadata = named.metadata().map_err(|why| {
        format!(
            "cannot stat named Finalization V2 root {}: {why}",
            path.display()
        )
    })?;
    if !held_before.is_dir() || !named_metadata.is_dir() {
        return Err(format!(
            "Finalization V2 root {} is not a directory",
            path.display()
        ));
    }
    let held_platform = platform_generation(&held_before, path)?;
    let named_platform = platform_generation(&named_metadata, path)?;
    if held_platform != named_platform {
        return Err(format!(
            "held Finalization V2 root no longer names {}; path replacement refused",
            path.display()
        ));
    }
    require_not_symlink(path, false)?;
    Ok(DirectoryGenerationV2 {
        canonical_path,
        platform: held_platform,
    })
}

fn require_directory_generation(
    expected: &DirectoryGenerationV2,
    held: &File,
    path: &Path,
) -> Result<(), PopulationFinalizationV2Refusal> {
    let observed = directory_generation(held, path)?;
    if observed != *expected {
        return Err(format!(
            "Finalization V2 root {} changed or was path-replaced after open",
            path.display()
        ));
    }
    Ok(())
}

fn file_generation(
    held: &File,
    path: &Path,
    max_bytes: u64,
) -> Result<FileGenerationV2, PopulationFinalizationV2Refusal> {
    let held_before = held.metadata().map_err(|why| {
        format!(
            "cannot stat held Finalization V2 file {}: {why}",
            path.display()
        )
    })?;
    if !held_before.is_file() {
        return Err(format!(
            "held Finalization V2 path {} is not a regular file",
            path.display()
        ));
    }
    if held_before.len() > max_bytes {
        return Err(format!(
            "Finalization V2 file {} has {} bytes, above generation bound {max_bytes}",
            path.display(),
            held_before.len()
        ));
    }
    let mut named = File::open(path).map_err(|why| {
        format!(
            "cannot reopen named Finalization V2 file {}: {why}",
            path.display()
        )
    })?;
    let named_before = named.metadata().map_err(|why| {
        format!(
            "cannot stat named Finalization V2 file {}: {why}",
            path.display()
        )
    })?;
    let held_platform = platform_generation(&held_before, path)?;
    let named_platform = platform_generation(&named_before, path)?;
    if held_before.len() != named_before.len() || held_platform != named_platform {
        return Err(format!(
            "held Finalization V2 file no longer names {}; path replacement or generation change refused",
            path.display()
        ));
    }
    let content_digest = hash_file_bounded(&mut named, path, max_bytes)?;
    let named_after = named.metadata().map_err(|why| {
        format!(
            "cannot restat named Finalization V2 file {}: {why}",
            path.display()
        )
    })?;
    let held_after = held.metadata().map_err(|why| {
        format!(
            "cannot restat held Finalization V2 file {}: {why}",
            path.display()
        )
    })?;
    let expected_platform = platform_generation(&held_before, path)?;
    if held_before.len() != named_after.len()
        || held_before.len() != held_after.len()
        || expected_platform != platform_generation(&named_after, path)?
        || expected_platform != platform_generation(&held_after, path)?
    {
        return Err(format!(
            "Finalization V2 file {} changed while its generation was hashed",
            path.display()
        ));
    }
    Ok(FileGenerationV2 {
        len: held_after.len(),
        content_digest,
        platform: expected_platform,
    })
}

fn require_file_generation(
    expected: FileGenerationV2,
    held: &File,
    path: &Path,
    max_bytes: u64,
) -> Result<(), PopulationFinalizationV2Refusal> {
    let observed = file_generation(held, path, max_bytes)?;
    if observed != expected {
        return Err(format!(
            "Finalization V2 file {} changed after bounded open",
            path.display()
        ));
    }
    Ok(())
}

fn hash_file_bounded(
    file: &mut File,
    path: &Path,
    max_bytes: u64,
) -> Result<[u8; 32], PopulationFinalizationV2Refusal> {
    file.seek(SeekFrom::Start(0)).map_err(|why| {
        format!(
            "cannot seek Finalization V2 file {} for generation hash: {why}",
            path.display()
        )
    })?;
    let mut hasher = Hasher::new();
    hasher.update(GENERATION_DOMAIN);
    let mut total = 0_u64;
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|why| format!("cannot hash Finalization V2 file {}: {why}", path.display()))?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(
                u64::try_from(read)
                    .map_err(|_| "Finalization V2 generation read does not fit u64".to_owned())?,
            )
            .ok_or_else(|| "Finalization V2 generation byte count overflowed".to_owned())?;
        if total > max_bytes {
            return Err(format!(
                "Finalization V2 file {} exceeded {max_bytes} bytes during generation hash",
                path.display()
            ));
        }
        hasher.update(
            buffer
                .get(..read)
                .ok_or_else(|| "Finalization V2 generation read escaped buffer".to_owned())?,
        );
    }
    Ok(hasher.finalize())
}

#[cfg(unix)]
fn platform_generation(
    metadata: &std::fs::Metadata,
    path: &Path,
) -> Result<PlatformGenerationV2, PopulationFinalizationV2Refusal> {
    let modified_nanoseconds = metadata.mtime_nsec();
    let changed_nanoseconds = metadata.ctime_nsec();
    if !(0..1_000_000_000).contains(&modified_nanoseconds)
        || !(0..1_000_000_000).contains(&changed_nanoseconds)
    {
        return Err(format!(
            "Finalization V2 path {} exposes an invalid Unix generation timestamp",
            path.display()
        ));
    }
    Ok(PlatformGenerationV2 {
        device: metadata.dev(),
        inode: metadata.ino(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds,
        changed_seconds: metadata.ctime(),
        changed_nanoseconds,
    })
}

#[cfg(windows)]
fn platform_generation(
    metadata: &std::fs::Metadata,
    path: &Path,
) -> Result<PlatformGenerationV2, PopulationFinalizationV2Refusal> {
    Ok(PlatformGenerationV2 {
        volume_serial: metadata.volume_serial_number().ok_or_else(|| {
            format!(
                "Finalization V2 file {} has no Windows volume serial",
                path.display()
            )
        })?,
        file_index: metadata.file_index().ok_or_else(|| {
            format!(
                "Finalization V2 file {} has no Windows file index",
                path.display()
            )
        })?,
        creation_time: metadata.creation_time(),
        last_write_time: metadata.last_write_time(),
    })
}

#[cfg(not(any(unix, windows)))]
fn platform_generation(
    _metadata: &std::fs::Metadata,
    path: &Path,
) -> Result<PlatformGenerationV2, PopulationFinalizationV2Refusal> {
    Err(format!(
        "Finalization V2 cannot prove stable path identity for {} on this target",
        path.display()
    ))
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

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "focused adversarial tests intentionally fail immediately on malformed fixtures and mutate exact fixed offsets"
)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    const BLOCK_SEQUENCE: u64 = 29;
    const FIRST_PHYSICAL_RECORD: u64 = 101;
    static NEXT_TEST_PATH: AtomicU64 = AtomicU64::new(0);

    struct TestPath {
        path: PathBuf,
    }

    impl TestPath {
        fn absent(label: &str) -> Self {
            let sequence = NEXT_TEST_PATH.fetch_add(1, Ordering::Relaxed);
            Self {
                path: std::env::temp_dir().join(format!(
                    "brutex-pop-finalization-v2-{}-{sequence}-{label}",
                    std::process::id()
                )),
            }
        }

        fn directory(label: &str) -> Self {
            let value = Self::absent(label);
            std::fs::create_dir(&value.path).expect("create admitted test root");
            value
        }

        fn file(label: &str) -> Self {
            let value = Self::absent(label);
            std::fs::write(&value.path, b"not a directory").expect("create non-directory root");
            value
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestPath {
        fn drop(&mut self) {
            let Ok(metadata) = std::fs::symlink_metadata(&self.path) else {
                return;
            };
            if metadata.is_dir() {
                let _ = std::fs::remove_dir_all(&self.path);
            } else {
                let _ = std::fs::remove_file(&self.path);
            }
        }
    }

    fn digest(seed: u64) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(b"brutex-population-finalization-v2-test-value\0");
        hasher.update(&seed.to_le_bytes());
        hasher.finalize()
    }

    fn family(seed: u64, row_count: u64) -> FamilyAuthorityV2 {
        FamilyAuthorityV2 {
            candidate_universe_id: digest(seed),
            candidate_completion_digest: digest(seed + 1),
            pre_admission_authority_id: digest(seed + 2),
            population_id: digest(seed + 3),
            population_v4_completion_digest: digest(seed + 4),
            admission_completion_digest: digest(seed + 5),
            ordered_admission_decision_digest: digest(seed + 6),
            row_count,
            admitted_count: row_count,
            rejected_count: 0,
            unmeasured_count: 0,
            refused_count: 0,
        }
    }

    fn evidence() -> EvidenceAuthorityV2 {
        EvidenceAuthorityV2 {
            observation_authority_id: digest(100),
            observation_completion_digest: digest(101),
            observation_pair_identity: digest(102),
            observation_source_identity: digest(103),
            observation_policy_digest: digest(104),
            observation_layout_policy_digest: digest(105),
            observation_statistics_link_id: digest(106),
            statistics_audit_id: digest(107),
            statistics_completion_digest: digest(108),
            statistics_projection_policy_digest: digest(109),
            wilson_policy_digest: digest(110),
            cscv_policy_digest: digest(111),
            ordered_candidate_digest: digest(112),
            ordered_period_digest: digest(113),
            ordered_split_digest: digest(114),
            white_family_digest: digest(115),
            spa_family_digest: digest(116),
            romano_wolf_family_digest: digest(117),
            candidate_count: 3,
            period_count: 4,
            split_count: 3,
            bootstrap_draws: 1_000,
            bootstrap_seed: 42,
            bootstrap_block_length: 2,
        }
    }

    fn search() -> SearchAuthorityV2 {
        SearchAuthorityV2 {
            population_search_id: digest(200),
            ranking_validation_policy_digest: digest(201),
            walk_facts_digest: digest(202),
            source_policy_digest: digest(203),
            finalization_family_digest: digest(204),
            anchored_walk_authority_id: digest(205),
            opaque_cli_search_lineage_digest: digest(206),
            fold_count: 3,
            decided_folds: 2,
            profitable_oos_folds: 1,
            aggregate_oos_paisa: 125,
        }
    }

    fn rekey(
        sequence: u64,
        family: InstrumentFamilyV1,
        family_sequence: u64,
        population_id: [u8; 32],
        seed: u64,
    ) -> PopulationFinalizationRekeyV2 {
        PopulationFinalizationRekeyV2 {
            finalization_id: [0; 32],
            sequence,
            family,
            family_sequence,
            candidate_semantic_digest: digest(seed),
            statistics_period_digest: digest(seed + 1),
            statistics_split_digest: digest(seed + 2),
            population_id,
            final_strategy_digest: digest(seed + 3),
            population_row_payload_digest: digest(seed + 4),
            admission_v1_decision_digest: digest(seed + 5),
            admission_v2_evidence_digest: digest(seed + 6),
            admission_v2_verdict_digest: digest(seed + 7),
            admission_v2_decision_digest: digest(seed + 8),
        }
    }

    fn prepared() -> PreparedPopulationFinalizationV2 {
        let nifty = family(10, 2);
        let banknifty = family(30, 1);
        let search = search();
        let data = PopulationFinalizationDataV2 {
            finalization_id: [0; 32],
            rung_seconds: 300,
            horizon_bars: 75,
            requested_span: RequestedSpanIdentityV1::new(2022, 1, 2024, 12)
                .expect("valid fixture span"),
            feed_digest: digest(1),
            source_commit_digest: digest(2),
            calendar_policy_digest: digest(3),
            daily_reference_policy_digest: digest(4),
            ranking_policy_digest: search.ranking_validation_policy_digest,
            admission_policy_digest: digest(5),
            source_policy_digest: search.source_policy_digest,
            finalization_policy_digest: digest(6),
            nifty,
            banknifty,
            evidence: evidence(),
            search,
            rekey_count: 3,
        };
        let rekeys = vec![
            rekey(0, InstrumentFamilyV1::Nifty, 0, nifty.population_id, 300),
            rekey(1, InstrumentFamilyV1::Nifty, 1, nifty.population_id, 320),
            rekey(
                2,
                InstrumentFamilyV1::BankNifty,
                0,
                banknifty.population_id,
                340,
            ),
        ];
        reseal_semantic_identity(PreparedPopulationFinalizationV2 { data, rekeys })
    }

    fn reseal_semantic_identity(
        mut value: PreparedPopulationFinalizationV2,
    ) -> PreparedPopulationFinalizationV2 {
        value.data.finalization_id = [0; 32];
        for row in &mut value.rekeys {
            row.finalization_id = [0; 32];
        }
        let identity = derive_finalization_id(&value.data, &value.rekeys)
            .expect("fixture semantic identity derives");
        value.data.finalization_id = identity;
        for row in &mut value.rekeys {
            row.finalization_id = identity;
        }
        value
    }

    fn records() -> Vec<[u8; POPULATION_FINALIZATION_V2_RECORD_BYTES]> {
        prepared()
            .records(BLOCK_SEQUENCE, FIRST_PHYSICAL_RECORD)
            .expect("valid fixture records")
    }

    fn bounds(max_records: u64) -> PopulationFinalizationV2Bounds {
        PopulationFinalizationV2Bounds::new(
            max_records,
            max_records * POPULATION_FINALIZATION_V2_RECORD_BYTES as u64,
        )
        .expect("valid fixture bounds")
    }

    fn altered_prepared(seed: u64) -> PreparedPopulationFinalizationV2 {
        let mut value = prepared();
        value.data.feed_digest = digest(seed);
        reseal_semantic_identity(value)
    }

    fn write_ledger_records(
        root: &Path,
        records: &[[u8; POPULATION_FINALIZATION_V2_RECORD_BYTES]],
    ) {
        File::create(root.join(LOCK_FILE)).expect("create test lock");
        let mut file = File::create(root.join(LEDGER_FILE)).expect("create test ledger");
        for raw in records {
            file.write_all(raw).expect("write test record");
        }
        file.sync_data().expect("sync test ledger");
    }

    fn reseal_record(raw: &mut [u8; POPULATION_FINALIZATION_V2_RECORD_BYTES]) {
        let seal = record_seal(&raw[..PAYLOAD_BYTES]);
        raw[PAYLOAD_BYTES..].copy_from_slice(&seal);
    }

    fn assert_refuses<T>(result: Result<T, PopulationFinalizationV2Refusal>, expected: &str) {
        let Err(why) = result else {
            panic!("malformed authority must refuse");
        };
        assert!(
            why.contains(expected),
            "refusal `{why}` did not contain `{expected}`"
        );
    }

    #[test]
    fn canonical_block_roundtrips_receipt_last_and_retries_exactly() {
        let expected = prepared();
        expected.validate().expect("fixture validates");
        let first = expected
            .records(BLOCK_SEQUENCE, FIRST_PHYSICAL_RECORD)
            .expect("first encoding");
        let retry = expected
            .records(BLOCK_SEQUENCE, FIRST_PHYSICAL_RECORD)
            .expect("retry encoding");
        assert_eq!(first, retry);
        assert_eq!(first.len(), 5);
        assert_eq!(expected.finalization_id(), expected.data.finalization_id);
        assert_eq!(expected.rekey_count(), 3);

        assert_eq!(
            decode_data(&first[0], FIRST_PHYSICAL_RECORD).expect("Data decodes"),
            expected.data
        );
        for (offset, row) in expected.rekeys.iter().enumerate() {
            assert_eq!(
                decode_rekey(
                    &first[offset + 1],
                    FIRST_PHYSICAL_RECORD + u64::try_from(offset).expect("offset") + 1,
                )
                .expect("Rekey decodes"),
                *row
            );
        }
        let completion = decode_completion(
            first.last().expect("Completion record"),
            FIRST_PHYSICAL_RECORD + 4,
        )
        .expect("Completion decodes");
        assert_eq!(completion.block_sequence, BLOCK_SEQUENCE);
        assert_eq!(completion.rekey_count, 3);
        assert_eq!(completion.nifty_rekey_count, 2);
        assert_eq!(completion.banknifty_rekey_count, 1);
        assert!(
            decode_block(&first, BLOCK_SEQUENCE, FIRST_PHYSICAL_RECORD)
                .expect("complete block decodes")
                == expected
        );
    }

    #[test]
    fn every_rekey_field_is_load_bearing() {
        let base = prepared();
        let base_id = base.finalization_id();
        macro_rules! assert_binds {
            ($field:ident, $value:expr) => {{
                let mut changed = base.clone();
                changed.rekeys[0].$field = $value;
                assert_ne!(
                    derive_finalization_id(&changed.data, &changed.rekeys)
                        .expect("changed identity derives"),
                    base_id,
                    "{} was not bound",
                    stringify!($field)
                );
            }};
        }
        assert_binds!(sequence, 9);
        assert_binds!(family, InstrumentFamilyV1::BankNifty);
        assert_binds!(family_sequence, 7);
        assert_binds!(candidate_semantic_digest, digest(900));
        assert_binds!(statistics_period_digest, digest(901));
        assert_binds!(statistics_split_digest, digest(902));
        assert_binds!(population_id, digest(903));
        assert_binds!(final_strategy_digest, digest(904));
        assert_binds!(population_row_payload_digest, digest(905));
        assert_binds!(admission_v1_decision_digest, digest(906));
        assert_binds!(admission_v2_evidence_digest, digest(907));
        assert_binds!(admission_v2_verdict_digest, digest(908));
        assert_binds!(admission_v2_decision_digest, digest(909));

        let mut foreign = base.clone();
        foreign.rekeys[0].finalization_id = digest(910);
        assert_refuses(foreign.validate(), "foreign identity or sequence");
    }

    #[test]
    fn header_magic_version_domain_sequence_reserve_and_seal_are_strict() {
        let original = records()[0];

        let mut unsealed_tamper = original;
        unsealed_tamper[80] ^= 1;
        assert_refuses(
            decode_data(&unsealed_tamper, FIRST_PHYSICAL_RECORD),
            "failed its complete domain-separated seal",
        );

        let mut bad_magic = original;
        bad_magic[0] ^= 1;
        reseal_record(&mut bad_magic);
        assert_refuses(
            decode_data(&bad_magic, FIRST_PHYSICAL_RECORD),
            "magic is unknown",
        );

        let mut bad_version = original;
        bad_version[16..20].copy_from_slice(&3_u32.to_le_bytes());
        reseal_record(&mut bad_version);
        assert_refuses(
            decode_data(&bad_version, FIRST_PHYSICAL_RECORD),
            "version 3 is unknown",
        );

        let mut bad_domain = original;
        bad_domain[20..24].copy_from_slice(&REKEY_DOMAIN.to_le_bytes());
        reseal_record(&mut bad_domain);
        assert_refuses(
            decode_data(&bad_domain, FIRST_PHYSICAL_RECORD),
            "domain 2 is not expected 1",
        );

        let mut bad_physical = original;
        bad_physical[24..32].copy_from_slice(&(FIRST_PHYSICAL_RECORD + 1).to_le_bytes());
        reseal_record(&mut bad_physical);
        assert_refuses(
            decode_data(&bad_physical, FIRST_PHYSICAL_RECORD),
            "physical sequence",
        );

        let mut zero_identity = original;
        zero_identity[32..64].fill(0);
        reseal_record(&mut zero_identity);
        assert_refuses(
            decode_data(&zero_identity, FIRST_PHYSICAL_RECORD),
            "all zero",
        );

        let mut bad_span_reserve = original;
        bad_span_reserve[75] = 1;
        reseal_record(&mut bad_span_reserve);
        assert_refuses(
            decode_data(&bad_span_reserve, FIRST_PHYSICAL_RECORD),
            "requested-span first reserve",
        );

        let mut bad_second_span_reserve = original;
        bad_second_span_reserve[79] = 1;
        reseal_record(&mut bad_second_span_reserve);
        assert_refuses(
            decode_data(&bad_second_span_reserve, FIRST_PHYSICAL_RECORD),
            "requested-span last reserve",
        );

        let mut bad_second_span_boundary = original;
        bad_second_span_boundary[78] = 13;
        reseal_record(&mut bad_second_span_boundary);
        assert_refuses(
            decode_data(&bad_second_span_boundary, FIRST_PHYSICAL_RECORD),
            "requested span is invalid",
        );

        let mut bad_trailing_reserve = original;
        bad_trailing_reserve[PAYLOAD_BYTES - 1] = 1;
        reseal_record(&mut bad_trailing_reserve);
        assert_refuses(
            decode_data(&bad_trailing_reserve, FIRST_PHYSICAL_RECORD),
            "Data trailing reserve",
        );

        let mut bad_rekey_reserve = records()[1];
        bad_rekey_reserve[73] = 1;
        reseal_record(&mut bad_rekey_reserve);
        assert_refuses(
            decode_rekey(&bad_rekey_reserve, FIRST_PHYSICAL_RECORD + 1),
            "rekey family reserve",
        );
    }

    #[test]
    fn semantic_tamper_with_a_fresh_record_seal_still_refuses() {
        let mut encoded = records();
        encoded[0][80] ^= 1;
        reseal_record(&mut encoded[0]);
        assert_refuses(
            decode_block(&encoded, BLOCK_SEQUENCE, FIRST_PHYSICAL_RECORD),
            "identity does not reproduce",
        );

        let mut encoded = records();
        encoded[1][88] ^= 1;
        reseal_record(&mut encoded[1]);
        assert_refuses(
            decode_block(&encoded, BLOCK_SEQUENCE, FIRST_PHYSICAL_RECORD),
            "identity does not reproduce",
        );
    }

    #[test]
    fn zero_counts_and_authority_crosswires_refuse() {
        let mut value = prepared();
        value.data.feed_digest = [0; 32];
        assert_refuses(value.validate(), "feed identity is all zero");

        let mut value = prepared();
        value.data.nifty.admitted_count = 1;
        assert_refuses(value.validate(), "statuses total 1");

        let mut value = prepared();
        value.data.nifty.row_count = 0;
        value.data.nifty.admitted_count = 0;
        assert_refuses(value.validate(), "zero complete candidates");

        let mut value = prepared();
        value.data.evidence.candidate_count = 4;
        assert_refuses(value.validate(), "Statistics candidates");

        let mut value = prepared();
        value.data.search.opaque_cli_search_lineage_digest = [0; 32];
        assert_refuses(
            value.validate(),
            "opaque freshly reopened CLI search lineage identity is all zero",
        );

        let mut value = prepared();
        value.data.ranking_policy_digest = digest(920);
        assert_refuses(value.validate(), "crosswire anchored search");

        let mut value = prepared();
        value.data.search.profitable_oos_folds = 3;
        assert_refuses(value.validate(), "count hierarchy is invalid");

        let mut value = prepared();
        value.rekeys[0].candidate_semantic_digest = [0; 32];
        assert_refuses(value.validate(), "Candidate semantic identity is all zero");

        let mut value = prepared();
        value.rekeys.pop();
        assert_refuses(value.validate(), "has 2 rekeys, expected 3");
    }

    #[test]
    fn reordered_duplicate_foreign_and_cross_family_rows_refuse() {
        let mut value = prepared();
        value.rekeys.swap(0, 1);
        assert_refuses(value.validate(), "foreign identity or sequence");

        let mut value = prepared();
        value.rekeys[1].candidate_semantic_digest = value.rekeys[0].candidate_semantic_digest;
        value = reseal_semantic_identity(value);
        assert_refuses(value.validate(), "repeats Candidate semantic identity");

        let mut value = prepared();
        value.rekeys[1].final_strategy_digest = value.rekeys[0].final_strategy_digest;
        value = reseal_semantic_identity(value);
        assert_refuses(value.validate(), "repeats final strategy identity");

        let mut value = prepared();
        value.rekeys[2].population_id = value.data.nifty.population_id;
        value = reseal_semantic_identity(value);
        assert_refuses(value.validate(), "canonical NIFTY-then-BANKNIFTY");

        let mut encoded = records();
        encoded.swap(1, 2);
        assert_refuses(
            decode_block(&encoded, BLOCK_SEQUENCE, FIRST_PHYSICAL_RECORD),
            "physical sequence",
        );
    }

    #[test]
    fn completion_binds_counts_order_data_and_block_sequence() {
        let encoded = records();
        assert_refuses(
            decode_block(&encoded, BLOCK_SEQUENCE + 1, FIRST_PHYSICAL_RECORD),
            "block sequence is foreign",
        );

        let mut missing_rekey = encoded.clone();
        missing_rekey.remove(2);
        assert_refuses(
            decode_block(&missing_rekey, BLOCK_SEQUENCE, FIRST_PHYSICAL_RECORD),
            "record count differs",
        );

        let mut completion_early = encoded.clone();
        completion_early.swap(3, 4);
        assert_refuses(
            decode_block(&completion_early, BLOCK_SEQUENCE, FIRST_PHYSICAL_RECORD),
            "domain",
        );

        let mut altered = prepared();
        altered.data.feed_digest = digest(930);
        altered = reseal_semantic_identity(altered);
        let altered_records = altered
            .records(BLOCK_SEQUENCE, FIRST_PHYSICAL_RECORD)
            .expect("altered block encodes");
        let mut crosswired = encoded;
        let last = crosswired.len() - 1;
        crosswired[last] = *altered_records.last().expect("altered Completion");
        assert_refuses(
            decode_block(&crosswired, BLOCK_SEQUENCE, FIRST_PHYSICAL_RECORD),
            "byte-for-byte",
        );
    }

    #[test]
    fn completion_and_rekey_trailing_reserves_refuse_even_when_resealed() {
        let encoded = records();
        let mut bad_rekey = encoded[1];
        bad_rekey[PAYLOAD_BYTES - 1] = 1;
        reseal_record(&mut bad_rekey);
        assert_refuses(
            decode_rekey(&bad_rekey, FIRST_PHYSICAL_RECORD + 1),
            "Rekey trailing reserve",
        );

        let mut bad_completion = *encoded.last().expect("Completion");
        bad_completion[PAYLOAD_BYTES - 1] = 1;
        reseal_record(&mut bad_completion);
        assert_refuses(
            decode_completion(&bad_completion, FIRST_PHYSICAL_RECORD + 4),
            "Completion trailing reserve",
        );
    }

    #[test]
    fn persistence_requires_admitted_existing_roots_and_explicit_bounds() {
        assert_refuses(
            PopulationFinalizationV2Bounds::new(3, 32_768),
            "cannot hold Data, both family rows and Completion",
        );
        assert_refuses(
            PopulationFinalizationV2Bounds::new(4, 4_095),
            "cannot hold the",
        );
        let valid = bounds(16);
        assert_eq!(valid.max_records(), 16);
        assert_eq!(
            valid.max_file_bytes(),
            16 * POPULATION_FINALIZATION_V2_RECORD_BYTES as u64
        );

        let absent = TestPath::absent("absent-root");
        assert_refuses(
            persist_population_finalization_v2(absent.path(), valid, &prepared()),
            "must already exist",
        );
        assert!(!absent.path().exists());

        let file = TestPath::file("file-root");
        assert_refuses(
            persist_population_finalization_v2(file.path(), valid, &prepared()),
            "is not a directory",
        );

        let empty = TestPath::directory("read-does-not-create");
        assert_refuses(
            PopulationFinalizationV2Ledger::open_read(empty.path(), valid),
            "cannot open Finalization V2 file",
        );
        assert!(
            std::fs::read_dir(empty.path())
                .expect("read empty root")
                .next()
                .is_none()
        );
    }

    #[test]
    fn write_reuse_drop_and_fresh_read_only_reopen_are_exact() {
        let root = TestPath::directory("write-reuse");
        let limits = bounds(32);
        let value = prepared();
        let first = persist_population_finalization_v2(root.path(), limits, &value)
            .expect("new block persists");
        let first_from_commit = first.authority();
        let PopulationFinalizationV2Commit::Written(first_authority) = first else {
            panic!("first append must be Written");
        };
        assert!(first_from_commit == first_authority);
        assert_eq!(first_authority.finalization_id(), value.finalization_id());
        assert_eq!(first_authority.block_sequence(), 0);
        assert_eq!(first_authority.first_physical_record(), 0);
        assert_eq!(first_authority.rekey_count(), 3);
        assert_ne!(first_authority.data_record_digest(), [0; 32]);
        assert_ne!(first_authority.completion_id(), [0; 32]);
        assert_ne!(first_authority.completion_record_digest(), [0; 32]);
        let expected_len = 5 * POPULATION_FINALIZATION_V2_RECORD_BYTES as u64;
        assert_eq!(
            std::fs::metadata(root.path().join(LEDGER_FILE))
                .expect("stat written ledger")
                .len(),
            expected_len
        );

        let retry = persist_population_finalization_v2(root.path(), limits, &value)
            .expect("exact completed retry reuses");
        let PopulationFinalizationV2Commit::Reused(reused) = retry else {
            panic!("exact retry must be Reused");
        };
        assert!(reused == first_authority);
        assert_eq!(
            std::fs::metadata(root.path().join(LEDGER_FILE))
                .expect("stat reused ledger")
                .len(),
            expected_len
        );

        let reopened = PopulationFinalizationV2Ledger::open_read(root.path(), limits)
            .expect("fresh read-only open");
        assert_eq!(
            reopened
                .reopen_structural_receipt(&value.finalization_id())
                .expect("generation-validated lookup"),
            Some(first_authority.receipt)
        );
        assert_eq!(
            reopened
                .reopen_structural_receipt(&digest(9_999))
                .expect("bounded absence lookup"),
            None
        );
        assert_refuses(
            PopulationFinalizationV2Ledger::open_read(root.path(), limits)
                .expect("second read-only open")
                .append(&value),
            "opened read-only",
        );
    }

    #[test]
    fn exact_trailing_data_rekeys_retry_appends_only_completion() {
        let root = TestPath::directory("orphan-retry");
        let limits = bounds(32);
        let value = prepared();
        let complete = value.records(0, 0).expect("fixture block");
        write_ledger_records(root.path(), &complete[..complete.len() - 1]);
        let prefix_len = 4 * POPULATION_FINALIZATION_V2_RECORD_BYTES as u64;
        assert_eq!(
            std::fs::metadata(root.path().join(LEDGER_FILE))
                .expect("stat prepared prefix")
                .len(),
            prefix_len
        );

        let commit = persist_population_finalization_v2(root.path(), limits, &value)
            .expect("exact trailing retry completes");
        assert!(matches!(commit, PopulationFinalizationV2Commit::Written(_)));
        assert_eq!(
            std::fs::metadata(root.path().join(LEDGER_FILE))
                .expect("stat completed retry")
                .len(),
            prefix_len + POPULATION_FINALIZATION_V2_RECORD_BYTES as u64
        );
    }

    #[test]
    fn foreign_retry_cannot_hide_a_trailing_prepared_block() {
        let root = TestPath::directory("foreign-orphan");
        let limits = bounds(32);
        let original = prepared();
        let complete = original.records(0, 0).expect("fixture block");
        write_ledger_records(root.path(), &complete[..complete.len() - 1]);
        let before = std::fs::read(root.path().join(LEDGER_FILE)).expect("read orphan bytes");

        assert_refuses(
            persist_population_finalization_v2(root.path(), limits, &altered_prepared(9_800)),
            "trailing prepared block belongs to",
        );
        assert_eq!(
            std::fs::read(root.path().join(LEDGER_FILE)).expect("reread orphan bytes"),
            before
        );
    }

    #[test]
    fn ragged_torn_corrupt_resealed_reordered_and_foreign_blocks_refuse() {
        let limits = bounds(64);

        let ragged = TestPath::directory("ragged");
        File::create(ragged.path().join(LOCK_FILE)).expect("create ragged lock");
        std::fs::write(ragged.path().join(LEDGER_FILE), [1_u8]).expect("write ragged ledger");
        assert_refuses(
            PopulationFinalizationV2Ledger::open_read(ragged.path(), limits),
            "ragged against",
        );

        let torn = TestPath::directory("torn");
        let complete = prepared().records(0, 0).expect("complete block");
        write_ledger_records(torn.path(), &complete[..2]);
        assert_refuses(
            PopulationFinalizationV2Ledger::open_read(torn.path(), limits),
            "is torn",
        );

        let corrupt = TestPath::directory("corrupt");
        let mut corrupt_records = complete.clone();
        corrupt_records[0][80] ^= 1;
        write_ledger_records(corrupt.path(), &corrupt_records);
        assert_refuses(
            PopulationFinalizationV2Ledger::open_read(corrupt.path(), limits),
            "failed its complete domain-separated seal",
        );

        let resealed = TestPath::directory("resealed-semantic");
        let mut resealed_records = complete.clone();
        resealed_records[0][80] ^= 1;
        reseal_record(&mut resealed_records[0]);
        write_ledger_records(resealed.path(), &resealed_records);
        assert_refuses(
            PopulationFinalizationV2Ledger::open_read(resealed.path(), limits),
            "identity does not reproduce",
        );

        let reordered = TestPath::directory("reordered");
        let mut reordered_records = complete.clone();
        reordered_records.swap(1, 2);
        write_ledger_records(reordered.path(), &reordered_records);
        assert_refuses(
            PopulationFinalizationV2Ledger::open_read(reordered.path(), limits),
            "physical sequence",
        );

        let foreign = TestPath::directory("foreign-completion");
        let altered = altered_prepared(9_801)
            .records(0, 0)
            .expect("foreign block");
        let mut foreign_records = complete;
        let last = foreign_records.len() - 1;
        foreign_records[last] = altered[last];
        write_ledger_records(foreign.path(), &foreign_records);
        assert_refuses(
            PopulationFinalizationV2Ledger::open_read(foreign.path(), limits),
            "byte-for-byte",
        );
    }

    #[test]
    fn duplicate_identity_and_middle_orphan_refuse() {
        let limits = bounds(64);
        let value = prepared();
        let first = value.records(0, 0).expect("first block");
        let second = value.records(1, 5).expect("second physical block");
        let mut duplicate_records = first.clone();
        duplicate_records.extend(second);
        let duplicate = TestPath::directory("duplicate");
        write_ledger_records(duplicate.path(), &duplicate_records);
        assert_refuses(
            PopulationFinalizationV2Ledger::open_read(duplicate.path(), limits),
            "appears more than once",
        );

        let middle = TestPath::directory("middle-orphan");
        let mut middle_records = first[..first.len() - 1].to_vec();
        let following = altered_prepared(9_802)
            .records(1, 4)
            .expect("following block");
        middle_records.extend(following);
        write_ledger_records(middle.path(), &middle_records);
        assert_refuses(
            PopulationFinalizationV2Ledger::open_read(middle.path(), limits),
            "domain",
        );
    }

    #[test]
    fn bounded_open_and_append_refuse_before_growth() {
        let root = TestPath::directory("bounded-open");
        let value = prepared();
        let full = value.records(0, 0).expect("full block");
        write_ledger_records(root.path(), &full);
        assert_refuses(
            PopulationFinalizationV2Ledger::open_read(
                root.path(),
                PopulationFinalizationV2Bounds::new(
                    4,
                    8 * POPULATION_FINALIZATION_V2_RECORD_BYTES as u64,
                )
                .expect("four-record bound"),
            ),
            "above explicit maximum 4",
        );

        let append_root = TestPath::directory("bounded-append");
        let exact_one = bounds(5);
        persist_population_finalization_v2(append_root.path(), exact_one, &value)
            .expect("first block fits exact bound");
        let before = std::fs::read(append_root.path().join(LEDGER_FILE))
            .expect("read before bounded refusal");
        assert_refuses(
            persist_population_finalization_v2(
                append_root.path(),
                exact_one,
                &altered_prepared(9_803),
            ),
            "above explicit maximum 5",
        );
        assert_eq!(
            std::fs::read(append_root.path().join(LEDGER_FILE))
                .expect("read after bounded refusal"),
            before
        );
    }

    #[test]
    fn prepared_bounds_refuse_before_duplicate_index_or_record_allocation() {
        let root = TestPath::directory("prepared-bound-before-validation");
        let limits = bounds(5);
        let mut over_bound = prepared();
        over_bound.data.banknifty.row_count = 2;
        over_bound.data.banknifty.admitted_count = 2;
        over_bound.data.evidence.candidate_count = 4;
        over_bound.data.rekey_count = 4;
        let mut duplicate = over_bound.rekeys[2];
        duplicate.sequence = 3;
        duplicate.family_sequence = 1;
        over_bound.rekeys.push(duplicate);
        over_bound = reseal_semantic_identity(over_bound);

        assert_refuses(over_bound.validate(), "repeats Candidate semantic identity");
        let mut writer = PopulationFinalizationV2Ledger::open_write(root.path(), limits)
            .expect("open bounded allocation-order writer");
        assert_refuses(
            writer.append(&over_bound),
            "block has 6 records, above explicit maximum 5",
        );
        assert_eq!(
            std::fs::metadata(root.path().join(LEDGER_FILE))
                .expect("stat allocation-order ledger")
                .len(),
            0
        );
    }

    #[test]
    fn stale_same_length_mutation_and_path_replacement_invalidate_lookup() {
        let limits = bounds(32);

        let stale = TestPath::directory("stale-mutation");
        let value = prepared();
        persist_population_finalization_v2(stale.path(), limits, &value)
            .expect("seed stale ledger");
        let opened = PopulationFinalizationV2Ledger::open_read(stale.path(), limits)
            .expect("open before same-length mutation");
        let mut external = OpenOptions::new()
            .read(true)
            .write(true)
            .open(stale.path().join(LEDGER_FILE))
            .expect("open same-length mutator");
        external
            .seek(SeekFrom::Start(80))
            .expect("seek same-length byte");
        let mut byte = [0_u8; 1];
        external
            .read_exact(&mut byte)
            .expect("read same-length byte");
        external
            .seek(SeekFrom::Start(80))
            .expect("reseek same-length byte");
        external
            .write_all(&[byte[0] ^ 1])
            .expect("write same-length byte");
        external.sync_data().expect("sync same-length mutation");
        assert_refuses(
            opened.reopen_structural_receipt(&value.finalization_id()),
            "changed after bounded open",
        );

        let replaced = TestPath::directory("path-replacement");
        persist_population_finalization_v2(replaced.path(), limits, &value)
            .expect("seed replacement ledger");
        let opened = PopulationFinalizationV2Ledger::open_read(replaced.path(), limits)
            .expect("open before replacement");
        let data_path = replaced.path().join(LEDGER_FILE);
        let old_path = replaced.path().join("population-finalization-v2.old");
        let exact = std::fs::read(&data_path).expect("read exact replacement bytes");
        std::fs::rename(&data_path, &old_path).expect("move opened generation aside");
        std::fs::write(&data_path, exact).expect("write same-length replacement");
        assert_refuses(
            opened.reopen_structural_receipt(&value.finalization_id()),
            "changed or was path-replaced",
        );
    }

    #[test]
    fn checked_arithmetic_and_short_blocks_refuse() {
        assert_refuses(checked_sum(&[u64::MAX, 1], "fixture count"), "overflow u64");
        assert_refuses(
            decode_block(&[], BLOCK_SEQUENCE, FIRST_PHYSICAL_RECORD),
            "lacks Data or Completion",
        );
        assert_refuses(
            prepared().records(BLOCK_SEQUENCE, u64::MAX),
            "physical rekey index overflowed",
        );
    }

    #[test]
    fn root_and_lock_replacement_between_prefix_and_completion_refuse() {
        let limits = bounds(32);
        let value = prepared();

        let root = TestPath::directory("append-root-replacement");
        let moved_root = root.path().with_extension("held-generation");
        let original_root = root.path().to_path_buf();
        let mut writer = PopulationFinalizationV2Ledger::open_write(root.path(), limits)
            .expect("open root-replacement writer");
        writer.before_completion_test_hook = Some(Box::new({
            let original_root = original_root.clone();
            let moved_root = moved_root.clone();
            move || {
                std::fs::rename(&original_root, &moved_root)
                    .map_err(|why| format!("move admitted root during test: {why}"))?;
                std::fs::create_dir(&original_root)
                    .map_err(|why| format!("create replacement root during test: {why}"))
            }
        }));
        let root_result = writer.append(&value);
        drop(writer);
        assert_refuses(root_result, "path replacement refused");
        std::fs::remove_dir(&original_root).expect("remove replacement root");
        std::fs::rename(&moved_root, &original_root).expect("restore admitted root");
        assert_eq!(
            std::fs::metadata(root.path().join(LEDGER_FILE))
                .expect("stat root-replacement orphan")
                .len(),
            4 * POPULATION_FINALIZATION_V2_RECORD_BYTES as u64
        );

        let root = TestPath::directory("append-lock-replacement");
        let lock_path = root.path().join(LOCK_FILE);
        let moved_lock = root.path().join("population-finalization-v2.held-lock");
        let mut writer = PopulationFinalizationV2Ledger::open_write(root.path(), limits)
            .expect("open lock-replacement writer");
        writer.before_completion_test_hook = Some(Box::new({
            let lock_path = lock_path.clone();
            let moved_lock = moved_lock.clone();
            move || {
                std::fs::rename(&lock_path, &moved_lock)
                    .map_err(|why| format!("move held lock during test: {why}"))?;
                File::create(&lock_path)
                    .map(|_| ())
                    .map_err(|why| format!("create replacement lock during test: {why}"))
            }
        }));
        let lock_result = writer.append(&value);
        drop(writer);
        assert_refuses(lock_result, "changed or was path-replaced");
        std::fs::remove_file(&lock_path).expect("remove replacement lock");
        std::fs::rename(&moved_lock, &lock_path).expect("restore held lock");
        assert_eq!(
            std::fs::metadata(root.path().join(LEDGER_FILE))
                .expect("stat lock-replacement orphan")
                .len(),
            4 * POPULATION_FINALIZATION_V2_RECORD_BYTES as u64
        );
    }

    #[cfg(unix)]
    #[test]
    fn preplaced_root_lock_and_data_symlinks_refuse_without_touching_targets() {
        use std::os::unix::fs::symlink;

        let limits = bounds(32);
        let value = prepared();

        let real_root = TestPath::directory("real-root-for-link");
        let linked_root = TestPath::absent("linked-root");
        symlink(real_root.path(), linked_root.path()).expect("create root symlink");
        assert_refuses(
            persist_population_finalization_v2(linked_root.path(), limits, &value),
            "symbolic link",
        );
        assert!(
            std::fs::read_dir(real_root.path())
                .expect("read untouched real root")
                .next()
                .is_none()
        );

        let external_lock = TestPath::file("external-lock-target");
        let external_lock_before =
            std::fs::read(external_lock.path()).expect("read external lock target");
        let lock_root = TestPath::directory("linked-lock-root");
        symlink(external_lock.path(), lock_root.path().join(LOCK_FILE))
            .expect("create lock symlink");
        assert_refuses(
            persist_population_finalization_v2(lock_root.path(), limits, &value),
            "symbolic link",
        );
        assert_eq!(
            std::fs::read(external_lock.path()).expect("reread external lock target"),
            external_lock_before
        );

        let external_data = TestPath::file("external-data-target");
        let external_data_before =
            std::fs::read(external_data.path()).expect("read external data target");
        let data_root = TestPath::directory("linked-data-root");
        File::create(data_root.path().join(LOCK_FILE)).expect("create admitted regular lock");
        symlink(external_data.path(), data_root.path().join(LEDGER_FILE))
            .expect("create data symlink");
        assert_refuses(
            persist_population_finalization_v2(data_root.path(), limits, &value),
            "symbolic link",
        );
        assert_eq!(
            std::fs::read(external_data.path()).expect("reread external data target"),
            external_data_before
        );
    }

    #[test]
    fn self_consistent_resealed_bytes_are_structural_not_authenticated() {
        let limits = bounds(32);
        let root = TestPath::directory("structural-forgery");
        let trusted = prepared();
        let forged = altered_prepared(99_001);
        let forged_records = forged.records(0, 0).expect("encode forged block");
        write_ledger_records(root.path(), &forged_records);

        let mut reopened = PopulationFinalizationV2Ledger::open_read(root.path(), limits)
            .expect("self-consistent bytes parse structurally");
        let structural = reopened
            .reopen_structural_receipt(&forged.finalization_id())
            .expect("structural lookup succeeds")
            .expect("forged structural receipt exists");
        assert_eq!(structural.finalization_id(), forged.finalization_id());
        assert_refuses(
            reopened.authenticate_structural_receipt(structural, &trusted),
            "not the indexed opaque preparation identity",
        );
    }
}
