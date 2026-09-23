//! Typed mixed-family Population Admission V4 authority.
//!
//! Admission V3 remains byte-for-byte paired-nonempty.  V4 consumes the exact
//! opaque Statistics V3 successor and preserves each family's terminal shape:
//! an `Evaluated` family has one recomputed Runner decision per real Candidate,
//! `InsufficientForCscv` retains its one real Candidate lineage but has no
//! invented PBO or decision, and `NaturallyExtinct` has neither Candidate nor
//! decision rows.  NIFTY is always the first family and BANKNIFTY the second.
//!
//! Candidate, Base Evidence, Observation/Statistics and exact-grid Search V4
//! sources are equality-joined before a byte is opened for writing.  Records
//! are fixed-width and append-only.  The Data/family/decision prefix is synced
//! before its adjacent Completion; exact retry either reuses byte-identical
//! history or completes one exact receipt-less suffix.  A production authority
//! owns the freshly reopened ledger and exact preparation, so a structural
//! reopen or a self-consistent reseal cannot promote itself to Finalization.
//!
//! Whole-source authentication, policy evaluation, hashing, persistence and
//! fresh reopen are O(source bytes + Candidates), not O(1).  Only an admitted
//! fixed-record offset is constant in record count; filesystem latency is not.
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;
#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
use std::os::unix::fs::OpenOptionsExt as _;

use brutex_core::blake3::Hasher;
use runner::admission::{
    AdmissionEvidenceValuesV1, AdmissionPolicyV1, AdmissionStatusV1,
    AdmissionV3ArithmeticProjection, AdmissionVerdictV1,
};
use runner::validate::{AnchoredSearchAuthorityProjectionV4, AnchoredSearchValidationV4};

use crate::anchored_search_lineage_v4::{
    AnchoredSearchLineageMemberAuthorityV4, AuthenticatedAnchoredSearchLineageV4,
};
use crate::candidate_universe::{
    CandidateUniverseReceiptV1, CandidateUniverseReopenAuditV1, PairedBaseEvidenceReaderV2,
    PairedBaseEvidenceRecordProjectionV2,
};
use crate::population::{InstrumentFamilyV1, RequestedSpanIdentityV1};
use crate::population_statistics_v3::{
    PopulationStatisticsAdmissionCandidateV4, PopulationStatisticsAdmissionSourceV3,
    PopulationStatisticsAuthorityProjectionV3, PopulationStatisticsFamilyProjectionV3,
    StatisticsFamilyTerminalV3,
};
use crate::step3_orchestrator::{StoredSearchMemberV4, StoredSearchPairV4};

/// Bytes in the independent Admission V4 file header.
pub(crate) const POPULATION_ADMISSION_V4_HEADER_BYTES: u64 = 64;
/// Fixed bytes in every Admission V4 record, including its seal.
pub(crate) const POPULATION_ADMISSION_V4_RECORD_BYTES: u64 = 4_096;

const HEADER_BYTES: usize = 64;
const RECORD_BYTES: usize = 4_096;
const RECORD_BYTES_U32: u32 = 4_096;
const PAYLOAD_BYTES: usize = RECORD_BYTES - 32;
const VERSION: u32 = 4;
const DATA_KIND: u32 = 1;
const FAMILY_KIND: u32 = 2;
const DECISION_KIND: u32 = 3;
const COMPLETION_KIND: u32 = 4;
const HEADER_MAGIC: [u8; 16] = *b"BTX-POPADM4-HDR\0";
const RECORD_MAGIC: [u8; 16] = *b"BTX-POPADM4-REC\0";
const DATA_FILE: &str = "population-admission-v4.bin";
const LOCK_FILE: &str = "population-admission-v4.lock";
const HEADER_DOMAIN: &[u8] = b"brutex-population-admission-v4-header\0";
const RECORD_DOMAIN: &[u8] = b"brutex-population-admission-v4-record\0";
const BLOCK_DOMAIN: &[u8] = b"brutex-population-admission-v4-block\0";
const FAMILY_DOMAIN: &[u8] = b"brutex-population-admission-v4-family\0";
const DECISION_DOMAIN: &[u8] = b"brutex-population-admission-v4-decision\0";
const ORDERED_DECISIONS_DOMAIN: &[u8] = b"brutex-population-admission-v4-decisions\0";
const COMPLETION_DOMAIN: &[u8] = b"brutex-population-admission-v4-completion\0";
const POLICY_DOMAIN: &[u8] = b"brutex-population-admission-v4-policy\0";
const RUNNER_DECISION_DOMAIN: &[u8] = b"brutex-population-admission-v4-runner-decision\0";
const RUNNER_EVIDENCE_DOMAIN: &[u8] = b"brutex-population-admission-v4-runner-evidence\0";
const RUNNER_VERDICT_DOMAIN: &[u8] = b"brutex-population-admission-v4-runner-verdict\0";
const GENERATION_DOMAIN: &[u8] = b"brutex-population-admission-v4-generation\0";
const RUNNER_DECISION_BYTES: usize = runner::admission::ADMISSION_DECISION_CANONICAL_LEN_V3;
const RUNNER_POLICY_BYTES: usize = runner::admission::ADMISSION_POLICY_CANONICAL_LEN_V1;
const RUNNER_EVIDENCE_BYTES: usize = runner::admission::ADMISSION_EVIDENCE_CANONICAL_LEN_V3;
const RUNNER_VERDICT_BYTES: usize = runner::admission::ADMISSION_VERDICT_CANONICAL_LEN_V1;
const RUNNER_HEADER_BYTES: usize = 12;
const RUNNER_EVIDENCE_OFFSET: usize = RUNNER_HEADER_BYTES + RUNNER_POLICY_BYTES;
const RUNNER_VERDICT_OFFSET: usize = RUNNER_EVIDENCE_OFFSET + RUNNER_EVIDENCE_BYTES;
const READ_CHUNK_BYTES: usize = 16 * 1_024;

#[cfg(any(target_os = "android", target_os = "linux"))]
const O_NOFOLLOW_FLAG: i32 = 0x20_000;
#[cfg(target_os = "macos")]
const O_NOFOLLOW_FLAG: i32 = 0x100;

const _: () = assert!(RUNNER_VERDICT_OFFSET + RUNNER_VERDICT_BYTES == RUNNER_DECISION_BYTES);
const _: () = assert!(PAYLOAD_BYTES + 32 == RECORD_BYTES);

/// Operator-facing fail-closed refusal.
pub(crate) type PopulationAdmissionV4Refusal = String;

/// Explicit nonzero scan, allocation and append ceilings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationAdmissionV4Bounds {
    authorities: u64,
    decisions_per_authority: u64,
    file_bytes: u64,
}

impl PopulationAdmissionV4Bounds {
    /// Creates explicit bounds; there is intentionally no `Default`.
    ///
    /// # Errors
    ///
    /// Refuses zero authority/decision ceilings and a byte ceiling too small
    /// for the smallest receipt-last block.
    pub(crate) fn new(
        max_authorities: u64,
        max_decisions_per_authority: u64,
        max_file_bytes: u64,
    ) -> Result<Self, PopulationAdmissionV4Refusal> {
        let minimum = POPULATION_ADMISSION_V4_HEADER_BYTES
            .checked_add(4 * POPULATION_ADMISSION_V4_RECORD_BYTES)
            .ok_or_else(|| "Admission V4 minimum byte bound overflowed".to_owned())?;
        if max_authorities == 0 || max_decisions_per_authority == 0 || max_file_bytes < minimum {
            return Err(format!(
                "Admission V4 bounds require nonzero authorities/decisions and at least {minimum} bytes"
            ));
        }
        Ok(Self {
            authorities: max_authorities,
            decisions_per_authority: max_decisions_per_authority,
            file_bytes: max_file_bytes,
        })
    }
}

/// Canonical family order in Admission V4.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum AdmissionV4Family {
    /// NSE-NIFTY prefix.
    Nifty = 1,
    /// NSE-BANKNIFTY suffix.
    BankNifty = 2,
}

impl AdmissionV4Family {
    fn from_instrument(value: InstrumentFamilyV1) -> Self {
        match value {
            InstrumentFamilyV1::Nifty => Self::Nifty,
            InstrumentFamilyV1::BankNifty => Self::BankNifty,
        }
    }

    fn decode(value: u8) -> Result<Self, PopulationAdmissionV4Refusal> {
        match value {
            1 => Ok(Self::Nifty),
            2 => Ok(Self::BankNifty),
            _ => Err(format!("Admission V4 family tag {value} is unknown")),
        }
    }
}

/// Typed family outcome; this is not a Candidate decision status.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AdmissionV4FamilyTerminal {
    /// Complete real Candidates were statistically evaluated and decided.
    Evaluated = 1,
    /// Exactly one real Candidate exists; relative CSCV/PBO is undefined.
    InsufficientForCscv = 2,
    /// Candidate enumeration authentically reached zero.
    NaturallyExtinct = 3,
}

impl AdmissionV4FamilyTerminal {
    /// The terminal's own word, for a report an operator reads.
    ///
    /// Spelled out rather than `Debug`-printed: `InsufficientForCscv` tells a
    /// reader nothing about why the family stopped, and the three terminals are
    /// not interchangeable — "one candidate, no relative PBO" is a very
    /// different outcome from "the ladder emptied", and a run that conflated
    /// them would look like the same answer twice.
    #[must_use]
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Evaluated => "evaluated",
            Self::InsufficientForCscv => "one candidate, no relative PBO",
            Self::NaturallyExtinct => "naturally extinct",
        }
    }

    fn from_statistics(value: StatisticsFamilyTerminalV3) -> Self {
        match value {
            StatisticsFamilyTerminalV3::Evaluated => Self::Evaluated,
            StatisticsFamilyTerminalV3::InsufficientForCscv => Self::InsufficientForCscv,
            StatisticsFamilyTerminalV3::NaturallyExtinct => Self::NaturallyExtinct,
        }
    }

    fn decode(value: u8) -> Result<Self, PopulationAdmissionV4Refusal> {
        match value {
            1 => Ok(Self::Evaluated),
            2 => Ok(Self::InsufficientForCscv),
            3 => Ok(Self::NaturallyExtinct),
            _ => Err(format!("Admission V4 family terminal {value} is unknown")),
        }
    }
}

/// Recomputed per-Candidate policy disposition for an evaluated family.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AdmissionV4DecisionStatus {
    /// Every required gate admitted the Candidate.
    Admitted = 1,
    /// Complete measured evidence rejected at least one gate.
    Rejected = 2,
    /// Complete policy evaluation remained unmeasured.
    Unmeasured = 3,
    /// Runner refused the evidence shape.
    Refused = 4,
}

impl AdmissionV4DecisionStatus {
    const fn from_runner(value: AdmissionStatusV1) -> Self {
        match value {
            AdmissionStatusV1::Admitted => Self::Admitted,
            AdmissionStatusV1::Rejected => Self::Rejected,
            AdmissionStatusV1::Unmeasured => Self::Unmeasured,
            AdmissionStatusV1::Refused => Self::Refused,
        }
    }

    fn decode(value: u8) -> Result<Self, PopulationAdmissionV4Refusal> {
        match value {
            1 => Ok(Self::Admitted),
            2 => Ok(Self::Rejected),
            3 => Ok(Self::Unmeasured),
            4 => Ok(Self::Refused),
            _ => Err(format!("Admission V4 decision status {value} is unknown")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SignalSourceV4 {
    digest: [u8; 32],
    bars: u64,
    first_ts_micros: i64,
    last_ts_micros: i64,
    column_digest: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GridSourceV4 {
    long_policy: [u8; 32],
    long_resolution: [u8; 32],
    short_policy: [u8; 32],
    short_resolution: [u8; 32],
    composite: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SearchSourceV4 {
    member_id: [u8; 32],
    source_authority_id: [u8; 32],
    validation_policy_id: [u8; 32],
    validation_family_id: [u8; 32],
    walk_id: [u8; 32],
    fold_count: u64,
    decided_folds: u64,
    profitable_oos_folds: u64,
    aggregate_oos_paisa: i64,
    evaluated_population_cells: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FamilySourceV4 {
    family: AdmissionV4Family,
    terminal: AdmissionV4FamilyTerminal,
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
    signal: SignalSourceV4,
    grid: GridSourceV4,
    search: SearchSourceV4,
    statistics_candidate_digest: [u8; 32],
    statistics_period_digest: [u8; 32],
    statistics_split_digest: [u8; 32],
    candidate_count: u64,
    decision_count: u64,
    family_id: [u8; 32],
}

impl FamilySourceV4 {
    fn validate(self) -> Result<(), PopulationAdmissionV4Refusal> {
        for (name, value) in [
            ("Statistics family", self.statistics_family_id),
            ("Candidate universe", self.candidate_universe_id),
            ("Candidate Completion", self.candidate_completion_digest),
            ("Candidate ordered rows", self.candidate_ordered_row_digest),
            ("Pre-Admission authority", self.pre_admission_authority_id),
            ("Observation identity", self.observation_identity),
            ("Observation policy", self.observation_policy_digest),
            ("Base Completion", self.base_completion_id),
            ("signal", self.signal.digest),
            ("signal column", self.signal.column_digest),
            ("Long grid policy", self.grid.long_policy),
            ("Long grid resolution", self.grid.long_resolution),
            ("Short grid policy", self.grid.short_policy),
            ("Short grid resolution", self.grid.short_resolution),
            ("grid composite", self.grid.composite),
            ("Search member", self.search.member_id),
            ("Search source authority", self.search.source_authority_id),
            ("Search policy", self.search.validation_policy_id),
            ("Search family", self.search.validation_family_id),
            ("Search walk", self.search.walk_id),
            ("family", self.family_id),
        ] {
            require_nonzero(name, value)?;
        }
        if self.signal.bars == 0
            || self.signal.first_ts_micros > self.signal.last_ts_micros
            || self.grid.long_policy == self.grid.short_policy
            || self.grid.long_resolution == self.grid.short_resolution
            || self.search.fold_count == 0
            || self.search.decided_folds > self.search.fold_count
            || self.search.profitable_oos_folds > self.search.decided_folds
        {
            return Err("Admission V4 family signal/grid/Search hierarchy differs".to_owned());
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
                    return Err("Admission V4 evaluated family shape differs".to_owned());
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
                    return Err("Admission V4 insufficient family shape differs".to_owned());
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
                        "Admission V4 extinct family fabricates rows or loses extinction proof"
                            .to_owned(),
                    );
                }
            }
        }
        if self.family_id != derive_family_id(&self) {
            return Err("Admission V4 family identity does not reproduce".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BlockSourceV4 {
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
    nifty: FamilySourceV4,
    banknifty: FamilySourceV4,
    candidate_count: u64,
    decision_count: u64,
    block_id: [u8; 32],
}

impl BlockSourceV4 {
    fn validate(&self) -> Result<(), PopulationAdmissionV4Refusal> {
        if self.rung_seconds == 0 || self.horizon_bars == 0 {
            return Err("Admission V4 rung/horizon is zero".to_owned());
        }
        RequestedSpanIdentityV1::new(
            self.requested_span.from_year(),
            self.requested_span.from_month(),
            self.requested_span.to_year(),
            self.requested_span.to_month(),
        )?;
        for (name, value) in [
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
            ("block", self.block_id),
        ] {
            require_nonzero(name, value)?;
        }
        AdmissionPolicyV1::from_canonical_bytes(&self.policy)
            .map_err(|why| format!("Admission V4 Runner policy refused: {why:?}"))?;
        if self.policy_digest != hash_slices(POLICY_DOMAIN, &[&self.policy]) {
            return Err("Admission V4 policy digest differs from its exact bytes".to_owned());
        }
        self.nifty.validate()?;
        self.banknifty.validate()?;
        if self.nifty.family != AdmissionV4Family::Nifty
            || self.banknifty.family != AdmissionV4Family::BankNifty
            || self.nifty.candidate_universe_id == self.banknifty.candidate_universe_id
            || self.nifty.base_completion_id == self.banknifty.base_completion_id
            || self.nifty.search.member_id == self.banknifty.search.member_id
        {
            return Err("Admission V4 families are not distinct NIFTY then BANKNIFTY".to_owned());
        }
        let candidates = self
            .nifty
            .candidate_count
            .checked_add(self.banknifty.candidate_count)
            .ok_or_else(|| "Admission V4 Candidate count overflowed".to_owned())?;
        let decisions = self
            .nifty
            .decision_count
            .checked_add(self.banknifty.decision_count)
            .ok_or_else(|| "Admission V4 decision count overflowed".to_owned())?;
        if candidates != self.candidate_count || decisions != self.decision_count {
            return Err("Admission V4 block/family cardinalities differ".to_owned());
        }
        if self.block_id != derive_block_id(self) {
            return Err("Admission V4 block identity does not reproduce".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DecisionRecordV4 {
    block_id: [u8; 32],
    decision_sequence: u64,
    statistics_sequence: u64,
    family: AdmissionV4Family,
    family_sequence: u64,
    status: AdmissionV4DecisionStatus,
    candidate_semantic_id: [u8; 32],
    candidate_row_digest: [u8; 32],
    pre_admission_authority_id: [u8; 32],
    statistics_period_digest: [u8; 32],
    statistics_split_digest: [u8; 32],
    search_member_id: [u8; 32],
    base_evidence_id: [u8; 32],
    runner_decision: [u8; RUNNER_DECISION_BYTES],
    runner_decision_digest: [u8; 32],
    evidence_digest: [u8; 32],
    verdict_digest: [u8; 32],
    decision_id: [u8; 32],
}

impl DecisionRecordV4 {
    fn validate(&self, source: &BlockSourceV4) -> Result<(), PopulationAdmissionV4Refusal> {
        let family = match self.family {
            AdmissionV4Family::Nifty => &source.nifty,
            AdmissionV4Family::BankNifty => &source.banknifty,
        };
        if family.terminal != AdmissionV4FamilyTerminal::Evaluated
            || self.block_id != source.block_id
            || self.family_sequence >= family.candidate_count
            || self.pre_admission_authority_id != family.pre_admission_authority_id
            || self.search_member_id != family.search.member_id
        {
            return Err("Admission V4 decision escaped its evaluated family source".to_owned());
        }
        for (name, value) in [
            ("Candidate semantic", self.candidate_semantic_id),
            ("Candidate row", self.candidate_row_digest),
            ("statistics period", self.statistics_period_digest),
            ("statistics split", self.statistics_split_digest),
            ("Base Evidence", self.base_evidence_id),
            ("Runner decision", self.runner_decision_digest),
            ("Runner evidence", self.evidence_digest),
            ("Runner verdict", self.verdict_digest),
            ("decision", self.decision_id),
        ] {
            require_nonzero(name, value)?;
        }
        let runner_evidence = self
            .runner_decision
            .get(RUNNER_EVIDENCE_OFFSET..RUNNER_VERDICT_OFFSET)
            .ok_or_else(|| "Admission V4 Runner evidence bytes are absent".to_owned())?;
        let runner_verdict = self
            .runner_decision
            .get(RUNNER_VERDICT_OFFSET..)
            .ok_or_else(|| "Admission V4 Runner verdict bytes are absent".to_owned())?;
        if self.runner_decision_digest
            != hash_slices(RUNNER_DECISION_DOMAIN, &[&self.runner_decision])
            || self.evidence_digest != hash_slices(RUNNER_EVIDENCE_DOMAIN, &[runner_evidence])
            || self.verdict_digest != hash_slices(RUNNER_VERDICT_DOMAIN, &[runner_verdict])
        {
            return Err("Admission V4 Runner nested digest differs".to_owned());
        }
        let nested_policy = self
            .runner_decision
            .get(RUNNER_HEADER_BYTES..RUNNER_EVIDENCE_OFFSET)
            .ok_or_else(|| "Admission V4 Runner policy bytes are absent".to_owned())?;
        if nested_policy != source.policy.as_slice() {
            return Err("Admission V4 Runner decision uses another policy".to_owned());
        }
        let arithmetic =
            AdmissionV3ArithmeticProjection::verify_decision_record_detached(&self.runner_decision)
                .map_err(|why| format!("Admission V4 Runner arithmetic refused: {why:?}"))?;
        if self.status != AdmissionV4DecisionStatus::from_runner(arithmetic.status())
            || arithmetic.decision_bytes() != self.runner_decision
        {
            return Err(
                "Admission V4 stored status/decision differs from recomputation".to_owned(),
            );
        }
        if self.decision_id != derive_decision_id(self) {
            return Err("Admission V4 decision identity does not reproduce".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreparedPopulationAdmissionV4 {
    source: BlockSourceV4,
    decisions: Vec<DecisionRecordV4>,
}

impl PreparedPopulationAdmissionV4 {
    fn validate(&self) -> Result<(), PopulationAdmissionV4Refusal> {
        self.source.validate()?;
        if u64::try_from(self.decisions.len()).ok() != Some(self.source.decision_count) {
            return Err("Admission V4 prepared decision cardinality differs".to_owned());
        }
        let mut next_nifty = 0_u64;
        let mut next_bank = 0_u64;
        for (index, decision) in self.decisions.iter().enumerate() {
            let expected = u64::try_from(index)
                .map_err(|_| "Admission V4 decision ordinal does not fit u64".to_owned())?;
            if decision.decision_sequence != expected {
                return Err("Admission V4 decisions are not in canonical order".to_owned());
            }
            let family_expected = match decision.family {
                AdmissionV4Family::Nifty => &mut next_nifty,
                AdmissionV4Family::BankNifty => &mut next_bank,
            };
            if decision.family_sequence != *family_expected {
                return Err("Admission V4 family decision order differs".to_owned());
            }
            *family_expected = family_expected
                .checked_add(1)
                .ok_or_else(|| "Admission V4 family sequence overflowed".to_owned())?;
            decision.validate(&self.source)?;
        }
        if next_nifty != self.source.nifty.decision_count
            || next_bank != self.source.banknifty.decision_count
        {
            return Err(
                "Admission V4 ordered decisions do not cover evaluated families".to_owned(),
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct JoinedCandidateV4 {
    statistics: PopulationStatisticsAdmissionCandidateV4,
    base: PairedBaseEvidenceRecordProjectionV2,
}

/// Prepares exact Admission V4 from typed Statistics, Candidate/Base and Search authorities.
///
/// No status, digest, family label, row or numeric statistic is caller-authored.
/// `policy` is the sole caller-selected decision input and is persisted in its
/// exact canonical bytes.
pub(crate) fn prepare_population_admission_v4(
    statistics: &PopulationStatisticsAdmissionSourceV3,
    nifty_candidate: &CandidateUniverseReopenAuditV1,
    banknifty_candidate: &CandidateUniverseReopenAuditV1,
    base: &mut PairedBaseEvidenceReaderV2,
    search: &StoredSearchPairV4,
    search_lineage: &AuthenticatedAnchoredSearchLineageV4,
    policy: &AdmissionPolicyV1,
) -> Result<PreparedPopulationAdmissionV4, PopulationAdmissionV4Refusal> {
    let statistics_authority = statistics.authority_projection();
    let nifty_statistics = statistics.nifty_family();
    let bank_statistics = statistics.banknifty_family();
    let nifty_receipt = nifty_candidate.receipt();
    let bank_receipt = banknifty_candidate.receipt();
    require_candidate_pair(&statistics_authority, &nifty_receipt, &bank_receipt)?;
    let base_authority = *base.authority();
    require_base_pair(
        &nifty_statistics,
        &bank_statistics,
        &nifty_receipt,
        &bank_receipt,
        &base_authority,
    )?;
    let nifty_projection = search.nifty().projection()?;
    let bank_projection = search.banknifty().projection()?;
    let nifty_search = require_search_member(
        "NIFTY",
        search.nifty(),
        &search_lineage.nifty(),
        &nifty_receipt,
        &nifty_projection,
    )?;
    let bank_search = require_search_member(
        "BANKNIFTY",
        search.banknifty(),
        &search_lineage.banknifty(),
        &bank_receipt,
        &bank_projection,
    )?;
    let receipt = search_lineage.structural_receipt();
    if receipt.nifty_member_id() != nifty_search.member_id
        || receipt.banknifty_member_id() != bank_search.member_id
    {
        return Err("Admission V4 Search V4 pair crosswires its durable members".to_owned());
    }
    let candidate_rows = statistics.admission_v4_candidates()?;
    let mut joined = Vec::new();
    joined
        .try_reserve_exact(candidate_rows.len())
        .map_err(|why| format!("cannot reserve Admission V4 joined Candidates: {why}"))?;
    for candidate in candidate_rows {
        let base_row = base
            .candidate(candidate.sequence())
            .map_err(|why| format!("Admission V4 Base projection refused: {why}"))?;
        require_candidate_base_join(
            &candidate,
            &base_row,
            base_authority.pair_id(),
            &nifty_statistics,
            &bank_statistics,
        )?;
        joined.push(JoinedCandidateV4 {
            statistics: candidate,
            base: base_row,
        });
    }
    let nifty_source = family_source(
        &nifty_statistics,
        &nifty_receipt,
        base_authority.nifty_audit().completion_id(),
        nifty_search,
    )?;
    let bank_source = family_source(
        &bank_statistics,
        &bank_receipt,
        base_authority.banknifty_audit().completion_id(),
        bank_search,
    )?;
    prepare_from_joined(
        &statistics_authority,
        nifty_receipt.identities().vocabulary_digest(),
        nifty_receipt.identities().evaluation_policy_digest(),
        base_authority.pair_id(),
        receipt.pair_id(),
        &nifty_source,
        &bank_source,
        joined,
        search.nifty().validation(),
        search.banknifty().validation(),
        policy,
    )
}

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "the inner boundary keeps every independently authenticated source and decision stage explicit"
)]
fn prepare_from_joined(
    statistics: &PopulationStatisticsAuthorityProjectionV3,
    vocabulary_digest: [u8; 32],
    evaluation_policy_digest: [u8; 32],
    base_pair_id: [u8; 32],
    search_pair_id: [u8; 32],
    nifty: &FamilySourceV4,
    banknifty: &FamilySourceV4,
    joined: Vec<JoinedCandidateV4>,
    nifty_validation: &AnchoredSearchValidationV4,
    banknifty_validation: &AnchoredSearchValidationV4,
    policy: &AdmissionPolicyV1,
) -> Result<PreparedPopulationAdmissionV4, PopulationAdmissionV4Refusal> {
    let policy_bytes = policy.canonical_bytes();
    let candidate_identities = (nifty.candidate_count, banknifty.candidate_count);
    let mut source = BlockSourceV4 {
        rung_seconds: statistics.rung_seconds(),
        horizon_bars: statistics.horizon_bars(),
        requested_span: statistics.requested_span(),
        feed_digest: statistics.feed_digest(),
        source_commit_digest: statistics.source_commit_digest(),
        calendar_policy_digest: statistics.calendar_policy_digest(),
        daily_reference_policy_digest: statistics.daily_reference_policy_digest(),
        vocabulary_digest,
        evaluation_policy_digest,
        statistics_authority_id: statistics.authority_id(),
        statistics_completion_digest: statistics.completion_digest(),
        statistics_policy_digest: statistics.statistics_policy_digest(),
        base_pair_id,
        search_pair_id,
        policy: policy_bytes,
        policy_digest: hash_slices(POLICY_DOMAIN, &[&policy_bytes]),
        nifty: *nifty,
        banknifty: *banknifty,
        candidate_count: candidate_identities
            .0
            .checked_add(candidate_identities.1)
            .ok_or_else(|| "Admission V4 Candidate total overflowed".to_owned())?,
        decision_count: 0,
        block_id: [0; 32],
    };
    source.decision_count = source
        .nifty
        .decision_count
        .checked_add(source.banknifty.decision_count)
        .ok_or_else(|| "Admission V4 decision total overflowed".to_owned())?;
    source.block_id = derive_block_id(&source);
    source.validate()?;
    if u64::try_from(joined.len()).ok() != Some(source.candidate_count) {
        return Err("Admission V4 joined Candidate cardinality differs".to_owned());
    }
    let mut decisions = Vec::new();
    decisions
        .try_reserve_exact(
            usize::try_from(source.decision_count)
                .map_err(|_| "Admission V4 decision count does not fit usize".to_owned())?,
        )
        .map_err(|why| format!("cannot reserve Admission V4 decisions: {why}"))?;
    for inputs in joined {
        let family_source = match inputs.statistics.family() {
            InstrumentFamilyV1::Nifty => &source.nifty,
            InstrumentFamilyV1::BankNifty => &source.banknifty,
        };
        match family_source.terminal {
            AdmissionV4FamilyTerminal::Evaluated => {
                let draft = inputs.statistics.draft().ok_or_else(|| {
                    "Admission V4 evaluated Candidate lacks Statistics draft".to_owned()
                })?;
                let validation = match inputs.statistics.family() {
                    InstrumentFamilyV1::Nifty => nifty_validation,
                    InstrumentFamilyV1::BankNifty => banknifty_validation,
                };
                let arithmetic = (*policy)
                    .evaluate_v3_exact_grid_projection(
                        inputs.base.record().admission_values(),
                        &draft,
                        validation,
                    )
                    .map_err(|why| format!("Admission V4 Runner evaluation refused: {why:?}"))?;
                let runner_decision = arithmetic.decision_bytes();
                let runner_evidence = runner_decision
                    .get(RUNNER_EVIDENCE_OFFSET..RUNNER_VERDICT_OFFSET)
                    .ok_or_else(|| "Admission V4 generated Runner evidence is absent".to_owned())?;
                let runner_verdict = runner_decision
                    .get(RUNNER_VERDICT_OFFSET..)
                    .ok_or_else(|| "Admission V4 generated Runner verdict is absent".to_owned())?;
                let decision_sequence = u64::try_from(decisions.len())
                    .map_err(|_| "Admission V4 decision sequence does not fit u64".to_owned())?;
                let mut decision = DecisionRecordV4 {
                    block_id: source.block_id,
                    decision_sequence,
                    statistics_sequence: inputs.statistics.sequence(),
                    family: AdmissionV4Family::from_instrument(inputs.statistics.family()),
                    family_sequence: inputs.statistics.family_sequence(),
                    status: AdmissionV4DecisionStatus::from_runner(arithmetic.status()),
                    candidate_semantic_id: inputs.statistics.candidate_semantic_digest(),
                    candidate_row_digest: inputs.base.record().candidate_row_digest(),
                    pre_admission_authority_id: inputs.statistics.pre_admission_authority_id(),
                    statistics_period_digest: inputs.statistics.candidate_ordered_period_digest(),
                    statistics_split_digest: inputs.statistics.candidate_ordered_split_digest(),
                    search_member_id: family_source.search.member_id,
                    base_evidence_id: inputs.base.record().evidence_id(),
                    runner_decision,
                    runner_decision_digest: hash_slices(
                        RUNNER_DECISION_DOMAIN,
                        &[&runner_decision],
                    ),
                    evidence_digest: hash_slices(RUNNER_EVIDENCE_DOMAIN, &[runner_evidence]),
                    verdict_digest: hash_slices(RUNNER_VERDICT_DOMAIN, &[runner_verdict]),
                    decision_id: [0; 32],
                };
                decision.decision_id = derive_decision_id(&decision);
                decision.validate(&source)?;
                decisions.push(decision);
            }
            AdmissionV4FamilyTerminal::InsufficientForCscv => {
                if inputs.statistics.draft().is_some() {
                    return Err("Admission V4 insufficient Candidate fabricated a draft".to_owned());
                }
            }
            AdmissionV4FamilyTerminal::NaturallyExtinct => {
                return Err("Admission V4 extinct family contributed a joined Candidate".to_owned());
            }
        }
    }
    let prepared = PreparedPopulationAdmissionV4 { source, decisions };
    prepared.validate()?;
    Ok(prepared)
}

fn require_candidate_pair(
    statistics: &PopulationStatisticsAuthorityProjectionV3,
    nifty: &CandidateUniverseReceiptV1,
    banknifty: &CandidateUniverseReceiptV1,
) -> Result<(), PopulationAdmissionV4Refusal> {
    if nifty.family() != InstrumentFamilyV1::Nifty
        || banknifty.family() != InstrumentFamilyV1::BankNifty
        || nifty.universe_id() == banknifty.universe_id()
    {
        return Err("Admission V4 Candidate pair is not distinct NIFTY then BANKNIFTY".to_owned());
    }
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
        || statistics.rung_seconds() != nifty.rung_seconds()
        || statistics.horizon_bars() != nifty.horizon_bars()
        || statistics.requested_span() != nifty.requested_span()
        || statistics.feed_digest() != left.feed_digest()
        || statistics.source_commit_digest() != left.source_commit_digest()
        || statistics.calendar_policy_digest() != left.calendar_policy_digest()
        || statistics.daily_reference_policy_digest() != left.daily_reference_policy_digest()
    {
        return Err("Admission V4 Candidate/Statistics cohort differs".to_owned());
    }
    Ok(())
}

fn require_base_pair(
    nifty_statistics: &PopulationStatisticsFamilyProjectionV3,
    bank_statistics: &PopulationStatisticsFamilyProjectionV3,
    nifty_candidate: &CandidateUniverseReceiptV1,
    bank_candidate: &CandidateUniverseReceiptV1,
    base: &crate::candidate_universe::PairedBaseEvidenceAuthorityV2,
) -> Result<(), PopulationAdmissionV4Refusal> {
    for (name, statistics, candidate, audit) in [
        (
            "NIFTY",
            nifty_statistics,
            nifty_candidate,
            base.nifty_audit(),
        ),
        (
            "BANKNIFTY",
            bank_statistics,
            bank_candidate,
            base.banknifty_audit(),
        ),
    ] {
        if statistics.family() != candidate.family()
            || statistics.candidate_universe_id() != candidate.universe_id()
            || statistics.candidate_completion_digest() != candidate.content_digest()
            || statistics.candidate_count() != candidate.row_count()
            || audit.family() != candidate.family()
            || audit.candidate_universe_id() != candidate.universe_id()
            || audit.candidate_completion_digest() != candidate.content_digest()
            || audit.candidate_ordered_row_digest() != candidate.ordered_row_digest()
            || audit.record_count() != candidate.row_count()
        {
            return Err(format!(
                "Admission V4 {name} Candidate/Statistics/Base authority differs"
            ));
        }
    }
    Ok(())
}

fn require_search_member(
    name: &str,
    retained: &StoredSearchMemberV4,
    durable: &AnchoredSearchLineageMemberAuthorityV4,
    candidate: &CandidateUniverseReceiptV1,
    projection: &AnchoredSearchAuthorityProjectionV4,
) -> Result<SearchSourceV4, PopulationAdmissionV4Refusal> {
    if retained.candidate_universe_id() != candidate.universe_id()
        || retained.candidate_completion_digest() != candidate.content_digest()
    {
        return Err(format!(
            "Admission V4 {name} retained Search belongs to another Candidate authority"
        ));
    }
    let source = projection.source_identity();
    let signal = candidate.signal_stream();
    let grids = candidate.identities().exit_grids();
    let long = projection.long_grid_identity();
    let short = projection.short_grid_identity();
    if source.signal_digest() != signal.digest()
        || source.signal_bars() != signal.count()
        || source.signal_first_ts_micros() != signal.first_ts_micros()
        || source.signal_last_ts_micros() != signal.last_ts_micros()
        || source.signal_column_digest() != candidate.signal_column_digest()
        || long.policy_digest() != grids.long.policy_digest
        || long.resolution_digest() != grids.long.resolved_digest
        || short.policy_digest() != grids.short.policy_digest
        || short.resolution_digest() != grids.short.resolved_digest
        || durable.validation_policy_id() != projection.policy_identity().digest()
        || durable.signal_digest() != source.signal_digest()
        || durable.signal_bars() != source.signal_bars()
        || durable.signal_first_ts_micros() != source.signal_first_ts_micros()
        || durable.signal_last_ts_micros() != source.signal_last_ts_micros()
        || durable.signal_column_digest() != source.signal_column_digest()
        || durable.aggregate_grid_id() != projection.grid_identity().digest()
        || durable.long_policy_id() != long.policy_digest()
        || durable.long_resolution_id() != long.resolution_digest()
        || durable.short_policy_id() != short.policy_digest()
        || durable.short_resolution_id() != short.resolution_digest()
        || durable.validation_family_id() != projection.family_identity().digest()
        || durable.walk_facts_id() != projection.walk_identity().digest()
        || durable.fold_count() != projection.fold_count()
        || durable.decided_folds() != projection.decided_folds()
        || durable.profitable_oos_folds() != projection.profitable_oos_folds()
        || durable.aggregate_oos_paisa() != projection.aggregate_oos_paisa()
        || durable.evaluated_population_cells() != projection.evaluated_population_cells()
    {
        return Err(format!(
            "Admission V4 durable {name} Search lineage differs from Candidate/Runner source"
        ));
    }
    let value = SearchSourceV4 {
        member_id: durable.member_id(),
        source_authority_id: durable.source_authority_id(),
        validation_policy_id: durable.validation_policy_id(),
        validation_family_id: durable.validation_family_id(),
        walk_id: durable.walk_facts_id(),
        fold_count: durable.fold_count(),
        decided_folds: durable.decided_folds(),
        profitable_oos_folds: durable.profitable_oos_folds(),
        aggregate_oos_paisa: durable.aggregate_oos_paisa(),
        evaluated_population_cells: durable.evaluated_population_cells(),
    };
    require_nonzero("Search member", value.member_id)?;
    Ok(value)
}

fn require_candidate_base_join(
    statistics: &PopulationStatisticsAdmissionCandidateV4,
    base: &PairedBaseEvidenceRecordProjectionV2,
    base_pair_id: [u8; 32],
    nifty: &PopulationStatisticsFamilyProjectionV3,
    banknifty: &PopulationStatisticsFamilyProjectionV3,
) -> Result<(), PopulationAdmissionV4Refusal> {
    let family = if statistics.family() == InstrumentFamilyV1::Nifty {
        nifty
    } else {
        banknifty
    };
    let record = base.record();
    if base.pair_id() != base_pair_id
        || base.global_sequence() != statistics.sequence()
        || base.family() != statistics.family()
        || base.family_sequence() != statistics.family_sequence()
        || record.candidate_universe_id() != family.candidate_universe_id()
        || record.candidate_sequence() != statistics.family_sequence()
        || record.candidate_semantic_id() != statistics.candidate_semantic_digest()
        || record.evidence_id() == [0; 32]
        || record.candidate_row_digest() == [0; 32]
        || record.trade_rows_digest() == [0; 32]
    {
        return Err("Admission V4 Statistics/Base Candidate crosswire".to_owned());
    }
    Ok(())
}

fn family_source(
    statistics: &PopulationStatisticsFamilyProjectionV3,
    candidate: &CandidateUniverseReceiptV1,
    base_completion_id: [u8; 32],
    search: SearchSourceV4,
) -> Result<FamilySourceV4, PopulationAdmissionV4Refusal> {
    let signal = candidate.signal_stream();
    let grids = candidate.identities().exit_grids();
    let terminal = AdmissionV4FamilyTerminal::from_statistics(statistics.terminal());
    let decision_count = if terminal == AdmissionV4FamilyTerminal::Evaluated {
        statistics.candidate_count()
    } else {
        0
    };
    let mut value = FamilySourceV4 {
        family: AdmissionV4Family::from_instrument(statistics.family()),
        terminal,
        statistics_family_id: statistics.identity(),
        candidate_universe_id: statistics.candidate_universe_id(),
        candidate_completion_digest: statistics.candidate_completion_digest(),
        candidate_ordered_row_digest: candidate.ordered_row_digest(),
        pre_admission_authority_id: statistics.pre_admission_authority_id(),
        observation_authority_id: statistics.observation_authority_id(),
        observation_identity: statistics.observation_identity(),
        observation_policy_digest: statistics.observation_policy_digest(),
        observation_data_digest: statistics.observation_data_digest(),
        observation_completion_digest: statistics.observation_completion_digest(),
        base_completion_id,
        signal: SignalSourceV4 {
            digest: signal.digest(),
            bars: signal.count(),
            first_ts_micros: signal.first_ts_micros(),
            last_ts_micros: signal.last_ts_micros(),
            column_digest: candidate.signal_column_digest(),
        },
        grid: GridSourceV4 {
            long_policy: grids.long.policy_digest,
            long_resolution: grids.long.resolved_digest,
            short_policy: grids.short.policy_digest,
            short_resolution: grids.short.resolved_digest,
            composite: grids
                .composite_digest()
                .map_err(|why| format!("Admission V4 Candidate grid refused: {why}"))?,
        },
        search,
        statistics_candidate_digest: statistics.ordered_candidate_digest(),
        statistics_period_digest: statistics.ordered_period_digest(),
        statistics_split_digest: statistics.ordered_split_digest(),
        candidate_count: statistics.candidate_count(),
        decision_count,
        family_id: [0; 32],
    };
    value.family_id = derive_family_id(&value);
    value.validate()?;
    Ok(value)
}

fn derive_family_id(value: &FamilySourceV4) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(FAMILY_DOMAIN);
    hasher.update(&[value.family as u8, value.terminal as u8]);
    for digest in [
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
        value.signal.digest,
        value.signal.column_digest,
        value.grid.long_policy,
        value.grid.long_resolution,
        value.grid.short_policy,
        value.grid.short_resolution,
        value.grid.composite,
        value.search.member_id,
        value.search.source_authority_id,
        value.search.validation_policy_id,
        value.search.validation_family_id,
        value.search.walk_id,
        value.statistics_candidate_digest,
        value.statistics_period_digest,
        value.statistics_split_digest,
    ] {
        hasher.update(&digest);
    }
    hasher.update(&value.signal.bars.to_le_bytes());
    hasher.update(&value.signal.first_ts_micros.to_le_bytes());
    hasher.update(&value.signal.last_ts_micros.to_le_bytes());
    hasher.update(&value.search.fold_count.to_le_bytes());
    hasher.update(&value.search.decided_folds.to_le_bytes());
    hasher.update(&value.search.profitable_oos_folds.to_le_bytes());
    hasher.update(&value.search.aggregate_oos_paisa.to_le_bytes());
    hasher.update(&value.search.evaluated_population_cells.to_le_bytes());
    hasher.update(&value.candidate_count.to_le_bytes());
    hasher.update(&value.decision_count.to_le_bytes());
    hasher.finalize()
}

fn derive_block_id(value: &BlockSourceV4) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(BLOCK_DOMAIN);
    hasher.update(&value.rung_seconds.to_le_bytes());
    hasher.update(&value.horizon_bars.to_le_bytes());
    hasher.update(&value.requested_span.canonical_bytes());
    for digest in [
        value.feed_digest,
        value.source_commit_digest,
        value.calendar_policy_digest,
        value.daily_reference_policy_digest,
        value.vocabulary_digest,
        value.evaluation_policy_digest,
        value.statistics_authority_id,
        value.statistics_completion_digest,
        value.statistics_policy_digest,
        value.base_pair_id,
        value.search_pair_id,
        value.policy_digest,
        value.nifty.family_id,
        value.banknifty.family_id,
    ] {
        hasher.update(&digest);
    }
    hasher.update(&value.policy);
    hasher.update(&value.candidate_count.to_le_bytes());
    hasher.update(&value.decision_count.to_le_bytes());
    hasher.finalize()
}

fn derive_decision_id(value: &DecisionRecordV4) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(DECISION_DOMAIN);
    hasher.update(&value.block_id);
    hasher.update(&value.decision_sequence.to_le_bytes());
    hasher.update(&value.statistics_sequence.to_le_bytes());
    hasher.update(&[value.family as u8]);
    hasher.update(&value.family_sequence.to_le_bytes());
    hasher.update(&[value.status as u8]);
    for digest in [
        value.candidate_semantic_id,
        value.candidate_row_digest,
        value.pre_admission_authority_id,
        value.statistics_period_digest,
        value.statistics_split_digest,
        value.search_member_id,
        value.base_evidence_id,
        value.runner_decision_digest,
        value.evidence_digest,
        value.verdict_digest,
    ] {
        hasher.update(&digest);
    }
    hasher.finalize()
}

fn ordered_decision_digest(
    decisions: &[DecisionRecordV4],
) -> Result<[u8; 32], PopulationAdmissionV4Refusal> {
    let mut hasher = Hasher::new();
    hasher.update(ORDERED_DECISIONS_DOMAIN);
    let count = u64::try_from(decisions.len())
        .map_err(|_| "Admission V4 ordered decision count does not fit u64".to_owned())?;
    hasher.update(&count.to_le_bytes());
    for decision in decisions {
        hasher.update(&decision.decision_id);
    }
    Ok(hasher.finalize())
}

fn hash_slices(domain: &[u8], values: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    for value in values {
        hasher.update(value);
    }
    hasher.finalize()
}

fn require_nonzero(name: &str, value: [u8; 32]) -> Result<(), PopulationAdmissionV4Refusal> {
    if value == [0; 32] {
        Err(format!("Admission V4 {name} identity is zero"))
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CompletionV4 {
    sequence: u64,
    source: BlockSourceV4,
    ordered_decision_digest: [u8; 32],
    completion_id: [u8; 32],
}

impl CompletionV4 {
    fn from_prepared(
        sequence: u64,
        prepared: &PreparedPopulationAdmissionV4,
    ) -> Result<Self, PopulationAdmissionV4Refusal> {
        let ordered_decision_digest = ordered_decision_digest(&prepared.decisions)?;
        let mut value = Self {
            sequence,
            source: prepared.source.clone(),
            ordered_decision_digest,
            completion_id: [0; 32],
        };
        value.completion_id = derive_completion_id(&value);
        Ok(value)
    }

    fn validate(&self) -> Result<(), PopulationAdmissionV4Refusal> {
        self.source.validate()?;
        require_nonzero("ordered decisions", self.ordered_decision_digest)?;
        require_nonzero("Completion", self.completion_id)?;
        if self.completion_id != derive_completion_id(self) {
            return Err("Admission V4 Completion identity does not reproduce".to_owned());
        }
        Ok(())
    }
}

fn derive_completion_id(value: &CompletionV4) -> [u8; 32] {
    hash_slices(
        COMPLETION_DOMAIN,
        &[
            &value.source.block_id,
            &value.source.nifty.family_id,
            &value.source.banknifty.family_id,
            &value.source.candidate_count.to_le_bytes(),
            &value.source.decision_count.to_le_bytes(),
            &value.ordered_decision_digest,
        ],
    )
}

fn encode_family(
    writer: &mut FixedWriter<'_>,
    value: &FamilySourceV4,
) -> Result<(), PopulationAdmissionV4Refusal> {
    value.validate()?;
    writer.u8(value.family as u8)?;
    writer.u8(value.terminal as u8)?;
    writer.zeros(6)?;
    for digest in [
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
        value.signal.digest,
        value.signal.column_digest,
        value.grid.long_policy,
        value.grid.long_resolution,
        value.grid.short_policy,
        value.grid.short_resolution,
        value.grid.composite,
        value.search.member_id,
        value.search.source_authority_id,
        value.search.validation_policy_id,
        value.search.validation_family_id,
        value.search.walk_id,
        value.statistics_candidate_digest,
        value.statistics_period_digest,
        value.statistics_split_digest,
        value.family_id,
    ] {
        writer.array(&digest)?;
    }
    writer.u64(value.signal.bars)?;
    writer.i64(value.signal.first_ts_micros)?;
    writer.i64(value.signal.last_ts_micros)?;
    writer.u64(value.search.fold_count)?;
    writer.u64(value.search.decided_folds)?;
    writer.u64(value.search.profitable_oos_folds)?;
    writer.i64(value.search.aggregate_oos_paisa)?;
    writer.u64(value.search.evaluated_population_cells)?;
    writer.u64(value.candidate_count)?;
    writer.u64(value.decision_count)
}

fn decode_family(
    reader: &mut FixedReader<'_>,
) -> Result<FamilySourceV4, PopulationAdmissionV4Refusal> {
    let family = AdmissionV4Family::decode(reader.u8()?)?;
    let terminal = AdmissionV4FamilyTerminal::decode(reader.u8()?)?;
    reader.require_zeros(6, "family reserve")?;
    let statistics_family_id = reader.array()?;
    let candidate_universe_id = reader.array()?;
    let candidate_completion_digest = reader.array()?;
    let candidate_ordered_row_digest = reader.array()?;
    let pre_admission_authority_id = reader.array()?;
    let observation_authority_id = reader.array()?;
    let observation_identity = reader.array()?;
    let observation_policy_digest = reader.array()?;
    let observation_data_digest = reader.array()?;
    let observation_completion_digest = reader.array()?;
    let base_completion_id = reader.array()?;
    let signal_digest = reader.array()?;
    let signal_column_digest = reader.array()?;
    let long_policy = reader.array()?;
    let long_resolution = reader.array()?;
    let short_policy = reader.array()?;
    let short_resolution = reader.array()?;
    let composite = reader.array()?;
    let member_id = reader.array()?;
    let source_authority_id = reader.array()?;
    let validation_policy_id = reader.array()?;
    let validation_family_id = reader.array()?;
    let walk_id = reader.array()?;
    let statistics_candidate_digest = reader.array()?;
    let statistics_period_digest = reader.array()?;
    let statistics_split_digest = reader.array()?;
    let family_id = reader.array()?;
    let value = FamilySourceV4 {
        family,
        terminal,
        statistics_family_id,
        candidate_universe_id,
        candidate_completion_digest,
        candidate_ordered_row_digest,
        pre_admission_authority_id,
        observation_authority_id,
        observation_identity,
        observation_policy_digest,
        observation_data_digest,
        observation_completion_digest,
        base_completion_id,
        signal: SignalSourceV4 {
            digest: signal_digest,
            bars: reader.u64()?,
            first_ts_micros: reader.i64()?,
            last_ts_micros: reader.i64()?,
            column_digest: signal_column_digest,
        },
        grid: GridSourceV4 {
            long_policy,
            long_resolution,
            short_policy,
            short_resolution,
            composite,
        },
        search: SearchSourceV4 {
            member_id,
            source_authority_id,
            validation_policy_id,
            validation_family_id,
            walk_id,
            fold_count: reader.u64()?,
            decided_folds: reader.u64()?,
            profitable_oos_folds: reader.u64()?,
            aggregate_oos_paisa: reader.i64()?,
            evaluated_population_cells: reader.u64()?,
        },
        statistics_candidate_digest,
        statistics_period_digest,
        statistics_split_digest,
        candidate_count: reader.u64()?,
        decision_count: reader.u64()?,
        family_id,
    };
    value.validate()?;
    Ok(value)
}

fn encode_source(
    writer: &mut FixedWriter<'_>,
    value: &BlockSourceV4,
) -> Result<(), PopulationAdmissionV4Refusal> {
    value.validate()?;
    writer.u32(value.rung_seconds)?;
    writer.u32(value.horizon_bars)?;
    writer.u16(value.requested_span.from_year())?;
    writer.u8(value.requested_span.from_month())?;
    writer.u8(0)?;
    writer.u16(value.requested_span.to_year())?;
    writer.u8(value.requested_span.to_month())?;
    writer.u8(0)?;
    for digest in [
        value.feed_digest,
        value.source_commit_digest,
        value.calendar_policy_digest,
        value.daily_reference_policy_digest,
        value.vocabulary_digest,
        value.evaluation_policy_digest,
        value.statistics_authority_id,
        value.statistics_completion_digest,
        value.statistics_policy_digest,
        value.base_pair_id,
        value.search_pair_id,
        value.policy_digest,
    ] {
        writer.array(&digest)?;
    }
    writer.array(&value.policy)?;
    encode_family(writer, &value.nifty)?;
    encode_family(writer, &value.banknifty)?;
    writer.u64(value.candidate_count)?;
    writer.u64(value.decision_count)?;
    writer.array(&value.block_id)
}

fn decode_source(
    reader: &mut FixedReader<'_>,
) -> Result<BlockSourceV4, PopulationAdmissionV4Refusal> {
    let rung_seconds = reader.u32()?;
    let horizon_bars = reader.u32()?;
    let from_year = reader.u16()?;
    let from_month = reader.u8()?;
    reader.require_zeros(1, "span first reserve")?;
    let to_year = reader.u16()?;
    let to_month = reader.u8()?;
    reader.require_zeros(1, "span second reserve")?;
    let requested_span = RequestedSpanIdentityV1::new(from_year, from_month, to_year, to_month)?;
    let value = BlockSourceV4 {
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
        policy_digest: reader.array()?,
        policy: reader.array()?,
        nifty: decode_family(reader)?,
        banknifty: decode_family(reader)?,
        candidate_count: reader.u64()?,
        decision_count: reader.u64()?,
        block_id: reader.array()?,
    };
    value.validate()?;
    Ok(value)
}

fn record_prefix(
    kind: u32,
    sequence: u64,
    writer: &mut FixedWriter<'_>,
) -> Result<(), PopulationAdmissionV4Refusal> {
    writer.array(&RECORD_MAGIC)?;
    writer.u32(VERSION)?;
    writer.u32(kind)?;
    writer.u64(sequence)
}

fn seal_record(payload: &[u8; PAYLOAD_BYTES]) -> [u8; RECORD_BYTES] {
    let mut raw = [0_u8; RECORD_BYTES];
    let (target_payload, target_seal) = raw.split_at_mut(PAYLOAD_BYTES);
    target_payload.copy_from_slice(payload);
    target_seal.copy_from_slice(&hash_slices(RECORD_DOMAIN, &[payload]));
    raw
}

fn encode_manifest(
    kind: u32,
    completion: &CompletionV4,
) -> Result<[u8; RECORD_BYTES], PopulationAdmissionV4Refusal> {
    if !matches!(kind, DATA_KIND | COMPLETION_KIND) {
        return Err("Admission V4 manifest kind is invalid".to_owned());
    }
    completion.validate()?;
    let mut payload = [0_u8; PAYLOAD_BYTES];
    let mut writer = FixedWriter::new(&mut payload);
    record_prefix(kind, completion.sequence, &mut writer)?;
    encode_source(&mut writer, &completion.source)?;
    writer.array(&completion.ordered_decision_digest)?;
    writer.array(&completion.completion_id)?;
    writer.require_remaining_zero();
    Ok(seal_record(&payload))
}

fn decode_manifest(
    raw: &[u8; RECORD_BYTES],
    expected_kind: u32,
) -> Result<CompletionV4, PopulationAdmissionV4Refusal> {
    let payload = validate_record(raw, expected_kind)?;
    let mut reader = FixedReader::new(payload);
    reader.skip(32)?;
    let value = CompletionV4 {
        sequence: u64_at(payload, 24)?,
        source: decode_source(&mut reader)?,
        ordered_decision_digest: reader.array()?,
        completion_id: reader.array()?,
    };
    reader.require_zeros(reader.remaining(), "manifest tail")?;
    value.validate()?;
    Ok(value)
}

fn encode_family_record(
    sequence: u64,
    block_id: [u8; 32],
    family: &FamilySourceV4,
) -> Result<[u8; RECORD_BYTES], PopulationAdmissionV4Refusal> {
    let mut payload = [0_u8; PAYLOAD_BYTES];
    let mut writer = FixedWriter::new(&mut payload);
    record_prefix(FAMILY_KIND, sequence, &mut writer)?;
    writer.array(&block_id)?;
    encode_family(&mut writer, family)?;
    writer.require_remaining_zero();
    Ok(seal_record(&payload))
}

fn decode_family_record(
    raw: &[u8; RECORD_BYTES],
) -> Result<(u64, [u8; 32], FamilySourceV4), PopulationAdmissionV4Refusal> {
    let payload = validate_record(raw, FAMILY_KIND)?;
    let mut reader = FixedReader::new(payload);
    reader.skip(24)?;
    let sequence = reader.u64()?;
    let block_id = reader.array()?;
    let family = decode_family(&mut reader)?;
    reader.require_zeros(reader.remaining(), "family tail")?;
    Ok((sequence, block_id, family))
}

fn encode_decision(
    sequence: u64,
    value: &DecisionRecordV4,
) -> Result<[u8; RECORD_BYTES], PopulationAdmissionV4Refusal> {
    let mut payload = [0_u8; PAYLOAD_BYTES];
    let mut writer = FixedWriter::new(&mut payload);
    record_prefix(DECISION_KIND, sequence, &mut writer)?;
    writer.array(&value.block_id)?;
    writer.u64(value.decision_sequence)?;
    writer.u64(value.statistics_sequence)?;
    writer.u8(value.family as u8)?;
    writer.u8(value.status as u8)?;
    writer.zeros(6)?;
    writer.u64(value.family_sequence)?;
    for digest in [
        value.candidate_semantic_id,
        value.candidate_row_digest,
        value.pre_admission_authority_id,
        value.statistics_period_digest,
        value.statistics_split_digest,
        value.search_member_id,
        value.base_evidence_id,
    ] {
        writer.array(&digest)?;
    }
    writer.array(&value.runner_decision)?;
    for digest in [
        value.runner_decision_digest,
        value.evidence_digest,
        value.verdict_digest,
        value.decision_id,
    ] {
        writer.array(&digest)?;
    }
    writer.require_remaining_zero();
    Ok(seal_record(&payload))
}

fn decode_decision(
    raw: &[u8; RECORD_BYTES],
) -> Result<(u64, DecisionRecordV4), PopulationAdmissionV4Refusal> {
    let payload = validate_record(raw, DECISION_KIND)?;
    let mut reader = FixedReader::new(payload);
    reader.skip(24)?;
    let sequence = reader.u64()?;
    let block_id = reader.array()?;
    let decision_sequence = reader.u64()?;
    let statistics_sequence = reader.u64()?;
    let family = AdmissionV4Family::decode(reader.u8()?)?;
    let status = AdmissionV4DecisionStatus::decode(reader.u8()?)?;
    reader.require_zeros(6, "decision reserve")?;
    let value = DecisionRecordV4 {
        block_id,
        decision_sequence,
        statistics_sequence,
        family,
        status,
        family_sequence: reader.u64()?,
        candidate_semantic_id: reader.array()?,
        candidate_row_digest: reader.array()?,
        pre_admission_authority_id: reader.array()?,
        statistics_period_digest: reader.array()?,
        statistics_split_digest: reader.array()?,
        search_member_id: reader.array()?,
        base_evidence_id: reader.array()?,
        runner_decision: reader.array()?,
        runner_decision_digest: reader.array()?,
        evidence_digest: reader.array()?,
        verdict_digest: reader.array()?,
        decision_id: reader.array()?,
    };
    reader.require_zeros(reader.remaining(), "decision tail")?;
    Ok((sequence, value))
}

fn validate_record(
    raw: &[u8; RECORD_BYTES],
    expected_kind: u32,
) -> Result<&[u8], PopulationAdmissionV4Refusal> {
    let (payload, seal) = raw.split_at(PAYLOAD_BYTES);
    if seal != hash_slices(RECORD_DOMAIN, &[payload]) {
        return Err("Admission V4 record seal differs".to_owned());
    }
    if payload.get(..16) != Some(RECORD_MAGIC.as_slice())
        || u32_at(payload, 16)? != VERSION
        || u32_at(payload, 20)? != expected_kind
    {
        return Err("Admission V4 record magic/version/kind differs".to_owned());
    }
    Ok(payload)
}

fn encoded_block(
    prepared: &PreparedPopulationAdmissionV4,
    sequence: u64,
) -> Result<Vec<[u8; RECORD_BYTES]>, PopulationAdmissionV4Refusal> {
    prepared.validate()?;
    let completion = CompletionV4::from_prepared(sequence, prepared)?;
    let mut records = Vec::new();
    let record_capacity = prepared
        .decisions
        .len()
        .checked_add(4)
        .ok_or_else(|| "Admission V4 encoded record capacity overflowed".to_owned())?;
    records
        .try_reserve_exact(record_capacity)
        .map_err(|why| format!("cannot reserve Admission V4 encoded block: {why}"))?;
    records.push(encode_manifest(DATA_KIND, &completion)?);
    records.push(encode_family_record(
        sequence,
        prepared.source.block_id,
        &prepared.source.nifty,
    )?);
    records.push(encode_family_record(
        sequence,
        prepared.source.block_id,
        &prepared.source.banknifty,
    )?);
    for decision in &prepared.decisions {
        records.push(encode_decision(sequence, decision)?);
    }
    records.push(encode_manifest(COMPLETION_KIND, &completion)?);
    Ok(records)
}

fn header() -> [u8; HEADER_BYTES] {
    let mut raw = [0_u8; HEADER_BYTES];
    let (prefix, target_seal) = raw.split_at_mut(32);
    if let Some(target) = prefix.get_mut(..16) {
        target.copy_from_slice(&HEADER_MAGIC);
    }
    if let Some(target) = prefix.get_mut(16..20) {
        target.copy_from_slice(&VERSION.to_le_bytes());
    }
    if let Some(target) = prefix.get_mut(20..24) {
        target.copy_from_slice(&RECORD_BYTES_U32.to_le_bytes());
    }
    let seal = hash_slices(HEADER_DOMAIN, &[prefix]);
    target_seal.copy_from_slice(&seal);
    raw
}

fn verify_header(file: &mut File) -> Result<(), PopulationAdmissionV4Refusal> {
    let mut raw = [0_u8; HEADER_BYTES];
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| format!("cannot read Admission V4 header: {why}"))?;
    if raw != header() {
        return Err("Admission V4 header differs".to_owned());
    }
    Ok(())
}

struct FixedWriter<'a> {
    bytes: &'a mut [u8],
    offset: usize,
}

impl<'a> FixedWriter<'a> {
    const fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, offset: 0 }
    }
    fn write(&mut self, value: &[u8]) -> Result<(), PopulationAdmissionV4Refusal> {
        let end = self
            .offset
            .checked_add(value.len())
            .ok_or_else(|| "Admission V4 writer offset overflowed".to_owned())?;
        let target = self
            .bytes
            .get_mut(self.offset..end)
            .ok_or_else(|| "Admission V4 record layout overflowed".to_owned())?;
        target.copy_from_slice(value);
        self.offset = end;
        Ok(())
    }
    fn array<const N: usize>(
        &mut self,
        value: &[u8; N],
    ) -> Result<(), PopulationAdmissionV4Refusal> {
        self.write(value)
    }
    fn u8(&mut self, value: u8) -> Result<(), PopulationAdmissionV4Refusal> {
        self.write(&[value])
    }
    fn u16(&mut self, value: u16) -> Result<(), PopulationAdmissionV4Refusal> {
        self.write(&value.to_le_bytes())
    }
    fn u32(&mut self, value: u32) -> Result<(), PopulationAdmissionV4Refusal> {
        self.write(&value.to_le_bytes())
    }
    fn u64(&mut self, value: u64) -> Result<(), PopulationAdmissionV4Refusal> {
        self.write(&value.to_le_bytes())
    }
    fn i64(&mut self, value: i64) -> Result<(), PopulationAdmissionV4Refusal> {
        self.write(&value.to_le_bytes())
    }
    fn zeros(&mut self, count: usize) -> Result<(), PopulationAdmissionV4Refusal> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| "Admission V4 zero offset overflowed".to_owned())?;
        if self.bytes.get(self.offset..end).is_none() {
            return Err("Admission V4 zero layout overflowed".to_owned());
        }
        self.offset = end;
        Ok(())
    }
    fn require_remaining_zero(&self) {
        debug_assert!(
            self.bytes
                .get(self.offset..)
                .is_some_and(|tail| tail.iter().all(|byte| *byte == 0))
        );
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
    const fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }
    fn read(&mut self, count: usize) -> Result<&'a [u8], PopulationAdmissionV4Refusal> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| "Admission V4 reader offset overflowed".to_owned())?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| "Admission V4 record ended early".to_owned())?;
        self.offset = end;
        Ok(value)
    }
    fn skip(&mut self, count: usize) -> Result<(), PopulationAdmissionV4Refusal> {
        self.read(count).map(|_| ())
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], PopulationAdmissionV4Refusal> {
        self.read(N)?
            .try_into()
            .map_err(|_| "Admission V4 array width differs".to_owned())
    }
    fn u8(&mut self) -> Result<u8, PopulationAdmissionV4Refusal> {
        Ok(u8::from_le_bytes(self.array()?))
    }
    fn u16(&mut self) -> Result<u16, PopulationAdmissionV4Refusal> {
        Ok(u16::from_le_bytes(self.array()?))
    }
    fn u32(&mut self) -> Result<u32, PopulationAdmissionV4Refusal> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    fn u64(&mut self) -> Result<u64, PopulationAdmissionV4Refusal> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    fn i64(&mut self) -> Result<i64, PopulationAdmissionV4Refusal> {
        Ok(i64::from_le_bytes(self.array()?))
    }
    fn require_zeros(
        &mut self,
        count: usize,
        name: &str,
    ) -> Result<(), PopulationAdmissionV4Refusal> {
        if self.read(count)?.iter().any(|byte| *byte != 0) {
            return Err(format!("Admission V4 {name} is nonzero"));
        }
        Ok(())
    }
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, PopulationAdmissionV4Refusal> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| "Admission V4 u32 field offset overflowed".to_owned())?;
    let raw: [u8; 4] = bytes
        .get(offset..end)
        .ok_or_else(|| "Admission V4 u32 field is absent".to_owned())?
        .try_into()
        .map_err(|_| "Admission V4 u32 width differs".to_owned())?;
    Ok(u32::from_le_bytes(raw))
}

fn u64_at(bytes: &[u8], offset: usize) -> Result<u64, PopulationAdmissionV4Refusal> {
    let end = offset
        .checked_add(8)
        .ok_or_else(|| "Admission V4 u64 field offset overflowed".to_owned())?;
    let raw: [u8; 8] = bytes
        .get(offset..end)
        .ok_or_else(|| "Admission V4 u64 field is absent".to_owned())?
        .try_into()
        .map_err(|_| "Admission V4 u64 width differs".to_owned())?;
    Ok(u64::from_le_bytes(raw))
}

/// Freshly reopened receipt for one complete mixed-family authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationAdmissionV4StructuralReceipt {
    sequence: u64,
    first_record: u64,
    block_id: [u8; 32],
    completion_id: [u8; 32],
    nifty_terminal: AdmissionV4FamilyTerminal,
    banknifty_terminal: AdmissionV4FamilyTerminal,
    nifty_candidate_count: u64,
    banknifty_candidate_count: u64,
    decision_count: u64,
}

impl PopulationAdmissionV4StructuralReceipt {
    /// Canonical logical append sequence.
    pub(crate) const fn sequence(self) -> u64 {
        self.sequence
    }
    /// First fixed-record index of the block's Data record.
    pub(crate) const fn first_record(self) -> u64 {
        self.first_record
    }
    /// Complete semantic source identity.
    pub(crate) const fn block_id(self) -> [u8; 32] {
        self.block_id
    }
    /// Receipt-last Completion identity.
    pub(crate) const fn completion_id(self) -> [u8; 32] {
        self.completion_id
    }
    /// Exact NIFTY terminal.
    pub(crate) const fn nifty_terminal(self) -> AdmissionV4FamilyTerminal {
        self.nifty_terminal
    }
    /// Exact BANKNIFTY terminal.
    pub(crate) const fn banknifty_terminal(self) -> AdmissionV4FamilyTerminal {
        self.banknifty_terminal
    }
    /// Number of real NIFTY Candidates, including an insufficient singleton.
    pub(crate) const fn nifty_candidate_count(self) -> u64 {
        self.nifty_candidate_count
    }
    /// Number of real BANKNIFTY Candidates, including an insufficient singleton.
    pub(crate) const fn banknifty_candidate_count(self) -> u64 {
        self.banknifty_candidate_count
    }
    /// Number of real evaluated-Candidate decisions.
    pub(crate) const fn decision_count(self) -> u64 {
        self.decision_count
    }
}

/// Finalization-facing common source and exact policy projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationAdmissionV4BlockProjection {
    block_id: [u8; 32],
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
    candidate_count: u64,
    decision_count: u64,
}

impl PopulationAdmissionV4BlockProjection {
    /// Complete Admission block identity.
    ///
    /// # Why it has no caller yet
    ///
    /// `ledger-v6` prints a block identity, but it reads the one on the
    /// STRUCTURAL RECEIPT — the value the commit returns — rather than
    /// reopening the block to ask it. This is the reopened projection's own
    /// answer, and the two agreeing is a check the verify surface would make,
    /// not something a report should assert on its own.
    #[expect(
        dead_code,
        reason = "the report cites the receipt's identity; comparing it to the reopened block is the verify surface's job"
    )]
    pub(crate) const fn block_id(&self) -> [u8; 32] {
        self.block_id
    }
    /// Signal-rung seconds.
    pub(crate) const fn rung_seconds(&self) -> u32 {
        self.rung_seconds
    }
    /// Exit horizon in bars.
    pub(crate) const fn horizon_bars(&self) -> u32 {
        self.horizon_bars
    }
    /// Requested civil span.
    pub(crate) const fn requested_span(&self) -> RequestedSpanIdentityV1 {
        self.requested_span
    }
    /// Exact feed identity.
    pub(crate) const fn feed_digest(&self) -> [u8; 32] {
        self.feed_digest
    }
    /// Exact source-commit identity.
    pub(crate) const fn source_commit_digest(&self) -> [u8; 32] {
        self.source_commit_digest
    }
    /// Calendar-policy identity.
    pub(crate) const fn calendar_policy_digest(&self) -> [u8; 32] {
        self.calendar_policy_digest
    }
    /// Daily-reference-policy identity.
    pub(crate) const fn daily_reference_policy_digest(&self) -> [u8; 32] {
        self.daily_reference_policy_digest
    }
    /// Vocabulary identity.
    pub(crate) const fn vocabulary_digest(&self) -> [u8; 32] {
        self.vocabulary_digest
    }
    /// Evaluation-policy identity.
    pub(crate) const fn evaluation_policy_digest(&self) -> [u8; 32] {
        self.evaluation_policy_digest
    }
    /// Statistics V3 authority identity.
    pub(crate) const fn statistics_authority_id(&self) -> [u8; 32] {
        self.statistics_authority_id
    }
    /// Statistics V3 Completion digest.
    pub(crate) const fn statistics_completion_digest(&self) -> [u8; 32] {
        self.statistics_completion_digest
    }
    /// Statistics V3 procedure identity.
    pub(crate) const fn statistics_policy_digest(&self) -> [u8; 32] {
        self.statistics_policy_digest
    }
    /// Paired Base Evidence authority identity.
    pub(crate) const fn base_pair_id(&self) -> [u8; 32] {
        self.base_pair_id
    }
    /// Paired anchored Search V4 authority identity.
    pub(crate) const fn search_pair_id(&self) -> [u8; 32] {
        self.search_pair_id
    }
    /// Exact canonical Runner policy bytes used for every evaluated Candidate.
    pub(crate) const fn policy(&self) -> &[u8; RUNNER_POLICY_BYTES] {
        &self.policy
    }
    /// Domain-separated identity of the exact policy bytes.
    pub(crate) const fn policy_digest(&self) -> [u8; 32] {
        self.policy_digest
    }
    /// Total real Candidate count across both families.
    pub(crate) const fn candidate_count(&self) -> u64 {
        self.candidate_count
    }
    /// Total evaluated-Candidate decision count.
    pub(crate) const fn decision_count(&self) -> u64 {
        self.decision_count
    }
}

impl From<&BlockSourceV4> for PopulationAdmissionV4BlockProjection {
    fn from(value: &BlockSourceV4) -> Self {
        Self {
            block_id: value.block_id,
            rung_seconds: value.rung_seconds,
            horizon_bars: value.horizon_bars,
            requested_span: value.requested_span,
            feed_digest: value.feed_digest,
            source_commit_digest: value.source_commit_digest,
            calendar_policy_digest: value.calendar_policy_digest,
            daily_reference_policy_digest: value.daily_reference_policy_digest,
            vocabulary_digest: value.vocabulary_digest,
            evaluation_policy_digest: value.evaluation_policy_digest,
            statistics_authority_id: value.statistics_authority_id,
            statistics_completion_digest: value.statistics_completion_digest,
            statistics_policy_digest: value.statistics_policy_digest,
            base_pair_id: value.base_pair_id,
            search_pair_id: value.search_pair_id,
            policy: value.policy,
            policy_digest: value.policy_digest,
            candidate_count: value.candidate_count,
            decision_count: value.decision_count,
        }
    }
}

/// Finalization-facing exact one-family projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationAdmissionV4FamilyProjection {
    source: FamilySourceV4,
}

impl PopulationAdmissionV4FamilyProjection {
    /// Canonical index family.
    pub(crate) const fn family(&self) -> AdmissionV4Family {
        self.source.family
    }
    /// Typed family terminal; it is not a Candidate decision status.
    pub(crate) const fn terminal(&self) -> AdmissionV4FamilyTerminal {
        self.source.terminal
    }
    /// Complete family-source identity.
    pub(crate) const fn family_id(&self) -> [u8; 32] {
        self.source.family_id
    }
    /// Statistics V3 family identity.
    pub(crate) const fn statistics_family_id(&self) -> [u8; 32] {
        self.source.statistics_family_id
    }
    /// Candidate Universe authority identity.
    pub(crate) const fn candidate_universe_id(&self) -> [u8; 32] {
        self.source.candidate_universe_id
    }
    /// Candidate receipt/content identity.
    pub(crate) const fn candidate_completion_digest(&self) -> [u8; 32] {
        self.source.candidate_completion_digest
    }
    /// Ordered literal Candidate-row identity.
    pub(crate) const fn candidate_ordered_row_digest(&self) -> [u8; 32] {
        self.source.candidate_ordered_row_digest
    }
    /// Pre-Admission authority identity.
    pub(crate) const fn pre_admission_authority_id(&self) -> [u8; 32] {
        self.source.pre_admission_authority_id
    }
    /// Observation V2 authority for extinction, or zero for evaluated evidence.
    pub(crate) const fn observation_authority_id(&self) -> [u8; 32] {
        self.source.observation_authority_id
    }
    /// Exact evaluated/terminal Observation identity.
    pub(crate) const fn observation_identity(&self) -> [u8; 32] {
        self.source.observation_identity
    }
    /// Observation procedure identity.
    pub(crate) const fn observation_policy_digest(&self) -> [u8; 32] {
        self.source.observation_policy_digest
    }
    /// Extinction Observation data identity, or zero for evaluated evidence.
    pub(crate) const fn observation_data_digest(&self) -> [u8; 32] {
        self.source.observation_data_digest
    }
    /// Extinction Observation Completion, or zero for evaluated evidence.
    pub(crate) const fn observation_completion_digest(&self) -> [u8; 32] {
        self.source.observation_completion_digest
    }
    /// Base Evidence family Completion identity.
    pub(crate) const fn base_completion_id(&self) -> [u8; 32] {
        self.source.base_completion_id
    }
    /// Full stored signal identity authenticated by Candidate and Search.
    pub(crate) const fn signal_digest(&self) -> [u8; 32] {
        self.source.signal.digest
    }
    /// Exact signal-bar count.
    pub(crate) const fn signal_bars(&self) -> u64 {
        self.source.signal.bars
    }
    /// First signal timestamp in epoch microseconds.
    pub(crate) const fn signal_first_ts_micros(&self) -> i64 {
        self.source.signal.first_ts_micros
    }
    /// Last signal timestamp in epoch microseconds.
    pub(crate) const fn signal_last_ts_micros(&self) -> i64 {
        self.source.signal.last_ts_micros
    }
    /// Exact prepared signal-column identity.
    pub(crate) const fn signal_column_digest(&self) -> [u8; 32] {
        self.source.signal.column_digest
    }
    /// Long exit-grid policy identity.
    pub(crate) const fn long_grid_policy_digest(&self) -> [u8; 32] {
        self.source.grid.long_policy
    }
    /// Long resolved-grid identity.
    pub(crate) const fn long_grid_resolution_digest(&self) -> [u8; 32] {
        self.source.grid.long_resolution
    }
    /// Short exit-grid policy identity.
    pub(crate) const fn short_grid_policy_digest(&self) -> [u8; 32] {
        self.source.grid.short_policy
    }
    /// Short resolved-grid identity.
    pub(crate) const fn short_grid_resolution_digest(&self) -> [u8; 32] {
        self.source.grid.short_resolution
    }
    /// Composite Long/Short grid identity.
    pub(crate) const fn grid_composite_digest(&self) -> [u8; 32] {
        self.source.grid.composite
    }
    /// Anchored exact-grid Search V4 member identity.
    pub(crate) const fn search_member_id(&self) -> [u8; 32] {
        self.source.search.member_id
    }
    /// Search V4 source-authority identity.
    pub(crate) const fn search_source_authority_id(&self) -> [u8; 32] {
        self.source.search.source_authority_id
    }
    /// Search V4 validation-policy identity.
    pub(crate) const fn search_validation_policy_id(&self) -> [u8; 32] {
        self.source.search.validation_policy_id
    }
    /// Search V4 complete fold-family identity.
    pub(crate) const fn search_validation_family_id(&self) -> [u8; 32] {
        self.source.search.validation_family_id
    }
    /// Search V4 walk/outcome identity.
    pub(crate) const fn search_walk_id(&self) -> [u8; 32] {
        self.source.search.walk_id
    }
    /// Search fold count.
    pub(crate) const fn search_fold_count(&self) -> u64 {
        self.source.search.fold_count
    }
    /// Search decided-fold count.
    pub(crate) const fn search_decided_folds(&self) -> u64 {
        self.source.search.decided_folds
    }
    /// Search profitable-OOS-fold count.
    pub(crate) const fn search_profitable_oos_folds(&self) -> u64 {
        self.source.search.profitable_oos_folds
    }
    /// Search aggregate OOS paisa.
    pub(crate) const fn search_aggregate_oos_paisa(&self) -> i64 {
        self.source.search.aggregate_oos_paisa
    }
    /// Search evaluated population-cell count.
    pub(crate) const fn search_evaluated_population_cells(&self) -> u64 {
        self.source.search.evaluated_population_cells
    }
    /// Ordered family Candidate-statistics identity, or zero at extinction.
    pub(crate) const fn statistics_candidate_digest(&self) -> [u8; 32] {
        self.source.statistics_candidate_digest
    }
    /// Ordered family period-statistics identity, or zero at extinction.
    pub(crate) const fn statistics_period_digest(&self) -> [u8; 32] {
        self.source.statistics_period_digest
    }
    /// Ordered family split-statistics identity, or zero at extinction.
    pub(crate) const fn statistics_split_digest(&self) -> [u8; 32] {
        self.source.statistics_split_digest
    }
    /// Number of real Candidate rows.
    pub(crate) const fn candidate_count(&self) -> u64 {
        self.source.candidate_count
    }
    /// Number of evaluated-Candidate decisions.
    pub(crate) const fn decision_count(&self) -> u64 {
        self.source.decision_count
    }
}

impl From<FamilySourceV4> for PopulationAdmissionV4FamilyProjection {
    fn from(value: FamilySourceV4) -> Self {
        Self { source: value }
    }
}

/// Finalization-facing exact evaluated-Candidate decision projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PopulationAdmissionV4DecisionProjection {
    decision_sequence: u64,
    statistics_sequence: u64,
    family: AdmissionV4Family,
    family_sequence: u64,
    status: AdmissionV4DecisionStatus,
    decision_id: [u8; 32],
    candidate_semantic_id: [u8; 32],
    candidate_row_digest: [u8; 32],
    pre_admission_authority_id: [u8; 32],
    statistics_period_digest: [u8; 32],
    statistics_split_digest: [u8; 32],
    search_member_id: [u8; 32],
    base_evidence_id: [u8; 32],
    runner_decision: [u8; RUNNER_DECISION_BYTES],
    comparison_values: AdmissionEvidenceValuesV1,
    verdict: AdmissionVerdictV1,
}

impl PopulationAdmissionV4DecisionProjection {
    fn from_record(
        source: &BlockSourceV4,
        value: &DecisionRecordV4,
    ) -> Result<Self, PopulationAdmissionV4Refusal> {
        value.validate(source)?;
        let arithmetic = AdmissionV3ArithmeticProjection::verify_decision_record_detached(
            &value.runner_decision,
        )
        .map_err(|why| format!("Admission V4 Finalization arithmetic refused: {why:?}"))?;
        Ok(Self {
            decision_sequence: value.decision_sequence,
            statistics_sequence: value.statistics_sequence,
            family: value.family,
            family_sequence: value.family_sequence,
            status: value.status,
            decision_id: value.decision_id,
            candidate_semantic_id: value.candidate_semantic_id,
            candidate_row_digest: value.candidate_row_digest,
            pre_admission_authority_id: value.pre_admission_authority_id,
            statistics_period_digest: value.statistics_period_digest,
            statistics_split_digest: value.statistics_split_digest,
            search_member_id: value.search_member_id,
            base_evidence_id: value.base_evidence_id,
            runner_decision: value.runner_decision,
            comparison_values: arithmetic.comparison_values(),
            verdict: arithmetic.verdict(),
        })
    }

    /// Canonical ordinal across NIFTY and then BANKNIFTY decisions.
    pub(crate) const fn decision_sequence(&self) -> u64 {
        self.decision_sequence
    }
    /// Canonical ordinal in the complete Statistics V3 Candidate sequence.
    pub(crate) const fn statistics_sequence(&self) -> u64 {
        self.statistics_sequence
    }
    /// Derived index family.
    pub(crate) const fn family(&self) -> AdmissionV4Family {
        self.family
    }
    /// Candidate ordinal inside the derived family.
    pub(crate) const fn family_sequence(&self) -> u64 {
        self.family_sequence
    }
    /// Status recomputed from the canonical Runner record.
    pub(crate) const fn status(&self) -> AdmissionV4DecisionStatus {
        self.status
    }
    /// Complete per-Candidate decision identity.
    pub(crate) const fn decision_id(&self) -> [u8; 32] {
        self.decision_id
    }
    /// Candidate semantic identity.
    pub(crate) const fn candidate_semantic_id(&self) -> [u8; 32] {
        self.candidate_semantic_id
    }
    /// Literal Candidate-row identity authenticated through Base Evidence.
    pub(crate) const fn candidate_row_digest(&self) -> [u8; 32] {
        self.candidate_row_digest
    }
    /// Pre-Admission authority identity.
    pub(crate) const fn pre_admission_authority_id(&self) -> [u8; 32] {
        self.pre_admission_authority_id
    }
    /// Exact Candidate period-family identity from Statistics V3.
    pub(crate) const fn statistics_period_digest(&self) -> [u8; 32] {
        self.statistics_period_digest
    }
    /// Exact Candidate split-family identity from Statistics V3.
    pub(crate) const fn statistics_split_digest(&self) -> [u8; 32] {
        self.statistics_split_digest
    }
    /// Exact-grid anchored Search V4 member identity.
    pub(crate) const fn search_member_id(&self) -> [u8; 32] {
        self.search_member_id
    }
    /// Same-pass Base Evidence record identity.
    pub(crate) const fn base_evidence_id(&self) -> [u8; 32] {
        self.base_evidence_id
    }
    /// Complete canonical Runner Admission V3 decision bytes.
    pub(crate) const fn runner_decision(&self) -> &[u8; RUNNER_DECISION_BYTES] {
        &self.runner_decision
    }
    /// Fixed comparison values reconstructed by Runner.
    ///
    /// # Why it has no caller yet
    ///
    /// These are the evidence values a decision was taken against, gate by
    /// gate. Printing them per decision is the "why was this rejected" report,
    /// and it needs the thirty-nine gates to carry meaning first — with
    /// thirty-seven of them supplied from knobs rather than a settled policy,
    /// a per-gate margin would be a margin against a placeholder.
    #[expect(
        dead_code,
        reason = "the per-gate rejection report waits on a settled admission policy"
    )]
    pub(crate) const fn comparison_values(&self) -> AdmissionEvidenceValuesV1 {
        self.comparison_values
    }
    /// Typed verdict reconstructed by Runner.
    pub(crate) const fn verdict(&self) -> AdmissionVerdictV1 {
        self.verdict
    }
}

/// One complete freshly authenticated handoff for a future Finalization V4.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PopulationAdmissionV4FinalizationProjection {
    receipt: PopulationAdmissionV4StructuralReceipt,
    source: PopulationAdmissionV4BlockProjection,
    families: [PopulationAdmissionV4FamilyProjection; 2],
    decisions: Vec<PopulationAdmissionV4DecisionProjection>,
}

impl PopulationAdmissionV4FinalizationProjection {
    /// Freshly reopened Admission V4 receipt.
    pub(crate) const fn receipt(&self) -> PopulationAdmissionV4StructuralReceipt {
        self.receipt
    }
    /// Complete common source and exact policy projection.
    pub(crate) const fn source(&self) -> &PopulationAdmissionV4BlockProjection {
        &self.source
    }
    /// Canonical NIFTY then BANKNIFTY family projections.
    pub(crate) const fn families(&self) -> &[PopulationAdmissionV4FamilyProjection; 2] {
        &self.families
    }
    /// Canonical NIFTY then BANKNIFTY evaluated-Candidate decisions.
    pub(crate) fn decisions(&self) -> &[PopulationAdmissionV4DecisionProjection] {
        &self.decisions
    }
}

/// Nonconstructible retained Admission V4 authority.
pub(crate) struct PopulationAdmissionV4Authority {
    receipt: PopulationAdmissionV4StructuralReceipt,
    prepared: PreparedPopulationAdmissionV4,
    ledger: PopulationAdmissionV4Ledger,
}

impl PopulationAdmissionV4Authority {
    pub(crate) const fn structural_receipt(&self) -> PopulationAdmissionV4StructuralReceipt {
        self.receipt
    }

    /// Reauthenticates the exact block once and returns its complete typed
    /// family/decision handoff. No ordinal, row, digest or status is accepted.
    pub(crate) fn finalization_projection(
        &mut self,
    ) -> Result<PopulationAdmissionV4FinalizationProjection, PopulationAdmissionV4Refusal> {
        let (completion, families, decisions) = self.ledger.read_complete(self.receipt)?;
        let expected = CompletionV4::from_prepared(self.receipt.sequence, &self.prepared)?;
        if completion != expected
            || families != [self.prepared.source.nifty, self.prepared.source.banknifty]
            || decisions != self.prepared.decisions
        {
            return Err("Admission V4 retained block differs from opaque preparation".to_owned());
        }
        let mut projected = Vec::new();
        projected
            .try_reserve_exact(decisions.len())
            .map_err(|why| format!("cannot reserve Admission V4 Finalization rows: {why}"))?;
        for decision in decisions {
            projected.push(PopulationAdmissionV4DecisionProjection::from_record(
                &self.prepared.source,
                &decision,
            )?);
        }
        let [nifty, banknifty] = families;
        Ok(PopulationAdmissionV4FinalizationProjection {
            receipt: self.receipt,
            source: PopulationAdmissionV4BlockProjection::from(&self.prepared.source),
            families: [nifty.into(), banknifty.into()],
            decisions: projected,
        })
    }
}

/// Exact write or byte-identical reuse result.
pub(crate) enum PopulationAdmissionV4Commit {
    /// New prefix and Completion were synchronized and freshly reopened.
    Written(PopulationAdmissionV4Authority),
    /// The exact block already existed and was freshly reauthenticated.
    Reused(PopulationAdmissionV4Authority),
}

impl PopulationAdmissionV4Commit {
    /// Mutable retained authority regardless of write or exact reuse.
    pub(crate) fn authority_mut(&mut self) -> &mut PopulationAdmissionV4Authority {
        match self {
            Self::Written(value) | Self::Reused(value) => value,
        }
    }

    /// Consumes the branch marker and returns the retained authority.
    pub(crate) fn into_authority(self) -> PopulationAdmissionV4Authority {
        match self {
            Self::Written(value) | Self::Reused(value) => value,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TrailingV4 {
    first_record: u64,
    source: BlockSourceV4,
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

struct PopulationAdmissionV4Ledger {
    root: PathBuf,
    root_file: File,
    root_identity: PlatformIdentity,
    lock_path: PathBuf,
    lock_file: File,
    lock_generation: FileGeneration,
    data_path: PathBuf,
    data_file: File,
    data_generation: FileGeneration,
    bounds: PopulationAdmissionV4Bounds,
    writable: bool,
    receipts: HashMap<[u8; 32], PopulationAdmissionV4StructuralReceipt>,
    trailing: Option<TrailingV4>,
    record_count: u64,
}

impl PopulationAdmissionV4Ledger {
    fn open_read(
        root: &Path,
        bounds: PopulationAdmissionV4Bounds,
    ) -> Result<Self, PopulationAdmissionV4Refusal> {
        Self::open(root, bounds, false)
    }

    fn open_write(
        root: &Path,
        bounds: PopulationAdmissionV4Bounds,
    ) -> Result<Self, PopulationAdmissionV4Refusal> {
        Self::open(root, bounds, true)
    }

    fn open(
        root: &Path,
        bounds: PopulationAdmissionV4Bounds,
        writable: bool,
    ) -> Result<Self, PopulationAdmissionV4Refusal> {
        let (root, root_file, root_identity) = open_root(root)?;
        let lock_path = root.join(LOCK_FILE);
        let data_path = root.join(DATA_FILE);
        let (lock_file, lock_created) = open_child(&lock_path, writable)?;
        if writable {
            lock_file
                .lock()
                .map_err(|why| format!("cannot lock Admission V4 writer: {why}"))?;
        } else {
            lock_file
                .lock_shared()
                .map_err(|why| format!("cannot shared-lock Admission V4 reader: {why}"))?;
        }
        let opened = (|| {
            let (mut data_file, data_created) = open_child(&data_path, writable)?;
            if data_created {
                data_file
                    .write_all(&header())
                    .and_then(|()| data_file.sync_all())
                    .map_err(|why| format!("cannot initialize Admission V4 data: {why}"))?;
            }
            if lock_created || data_created {
                root_file
                    .sync_all()
                    .map_err(|why| format!("cannot sync Admission V4 root: {why}"))?;
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
                    .map_err(|why| format!("cannot clone Admission V4 lock: {why}"))?,
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
            .map_err(|why| format!("cannot unlock Admission V4 after open: {why}"));
        combine(opened, unlocked)
    }

    fn scan(&mut self) -> Result<(), PopulationAdmissionV4Refusal> {
        self.receipts.clear();
        self.trailing = None;
        let len = self.data_generation.len;
        if len < HEADER_BYTES as u64
            || !(len - HEADER_BYTES as u64).is_multiple_of(RECORD_BYTES as u64)
        {
            return Err("Admission V4 data file is ragged".to_owned());
        }
        let total = (len - HEADER_BYTES as u64) / RECORD_BYTES as u64;
        let mut first = 0_u64;
        while first < total {
            let data_raw = read_record_at(&mut self.data_file, first)?;
            let data = decode_manifest(&data_raw, DATA_KIND)?;
            let expected_sequence = u64::try_from(self.receipts.len())
                .map_err(|_| "Admission V4 authority sequence does not fit u64".to_owned())?;
            if data.sequence != expected_sequence {
                return Err("Admission V4 logical sequence is not canonical".to_owned());
            }
            let block_records = data
                .source
                .decision_count
                .checked_add(4)
                .ok_or_else(|| "Admission V4 block record count overflowed".to_owned())?;
            if data.source.decision_count > self.bounds.decisions_per_authority {
                return Err("Admission V4 decision count exceeds explicit bound".to_owned());
            }
            let remaining = total - first;
            if remaining < block_records {
                validate_prefix(&mut self.data_file, first, remaining, &data)?;
                if self.receipts.contains_key(&data.source.block_id) {
                    return Err("Admission V4 trailing prefix repeats a complete block".to_owned());
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
            if self.receipts.insert(receipt.block_id, receipt).is_some() {
                return Err("Admission V4 block identity appears more than once".to_owned());
            }
            let authority_count = u64::try_from(self.receipts.len())
                .map_err(|_| "Admission V4 authority count does not fit u64".to_owned())?;
            if authority_count > self.bounds.authorities {
                return Err("Admission V4 authority count exceeds explicit bound".to_owned());
            }
            first = first
                .checked_add(block_records)
                .ok_or_else(|| "Admission V4 scan offset overflowed".to_owned())?;
        }
        self.record_count = total;
        self.require_unchanged()
    }

    fn append(
        &mut self,
        prepared: &PreparedPopulationAdmissionV4,
    ) -> Result<(bool, PopulationAdmissionV4StructuralReceipt), PopulationAdmissionV4Refusal> {
        if !self.writable {
            return Err("Admission V4 ledger is read-only".to_owned());
        }
        self.lock_file
            .lock()
            .map_err(|why| format!("cannot lock Admission V4 append: {why}"))?;
        let result = self.append_locked(prepared);
        let unlocked = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot unlock Admission V4 append: {why}"));
        combine(result, unlocked)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one linear receipt-last append keeps exact-prefix retry, sync ordering, and held-path checks adjacent"
    )]
    fn append_locked(
        &mut self,
        prepared: &PreparedPopulationAdmissionV4,
    ) -> Result<(bool, PopulationAdmissionV4StructuralReceipt), PopulationAdmissionV4Refusal> {
        self.require_unchanged()?;
        prepared.validate()?;
        if prepared.source.decision_count > self.bounds.decisions_per_authority {
            return Err("Admission V4 prepared decision count exceeds bound".to_owned());
        }
        if let Some(receipt) = self.receipts.get(&prepared.source.block_id).copied() {
            require_exact_records(
                &mut self.data_file,
                receipt.first_record,
                prepared,
                receipt.sequence,
            )?;
            self.data_file
                .sync_all()
                .map_err(|why| format!("cannot sync reused Admission V4 data: {why}"))?;
            self.root_file
                .sync_all()
                .map_err(|why| format!("cannot sync reused Admission V4 root: {why}"))?;
            self.require_unchanged()?;
            return Ok((false, receipt));
        }
        let sequence = u64::try_from(self.receipts.len())
            .map_err(|_| "Admission V4 sequence does not fit u64".to_owned())?;
        let records = encoded_block(prepared, sequence)?;
        let prefix = if let Some(trailing) = &self.trailing {
            if trailing.source != prepared.source
                || trailing.first_record + trailing.record_count != self.record_count
            {
                return Err("Admission V4 trailing prefix is not the exact retry".to_owned());
            }
            for index in 0..trailing.record_count {
                let stored = read_record_at(&mut self.data_file, trailing.first_record + index)?;
                let expected =
                    records
                        .get(usize::try_from(index).map_err(|_| {
                            "Admission V4 prefix index does not fit usize".to_owned()
                        })?)
                        .ok_or_else(|| {
                            "Admission V4 trailing prefix is longer than preparation".to_owned()
                        })?;
                if &stored != expected {
                    return Err(
                        "Admission V4 trailing prefix bytes differ from exact retry".to_owned()
                    );
                }
            }
            trailing.record_count
        } else {
            0
        };
        let encoded_count = u64::try_from(records.len())
            .map_err(|_| "Admission V4 record length does not fit u64".to_owned())?;
        let appended_count = encoded_count
            .checked_sub(prefix)
            .ok_or_else(|| "Admission V4 trailing prefix exceeds its exact block".to_owned())?;
        let next_record_count = self
            .record_count
            .checked_add(appended_count)
            .ok_or_else(|| "Admission V4 appended record count overflowed".to_owned())?;
        let next_bytes = (HEADER_BYTES as u64)
            .checked_add(
                next_record_count
                    .checked_mul(RECORD_BYTES as u64)
                    .ok_or_else(|| "Admission V4 appended bytes overflowed".to_owned())?,
            )
            .ok_or_else(|| "Admission V4 file bytes overflowed".to_owned())?;
        let authority_count = u64::try_from(self.receipts.len())
            .map_err(|_| "Admission V4 authority count does not fit u64".to_owned())?;
        if next_bytes > self.bounds.file_bytes || authority_count >= self.bounds.authorities {
            return Err("Admission V4 append exceeds explicit authority/file bound".to_owned());
        }
        let completion_index = records
            .len()
            .checked_sub(1)
            .ok_or_else(|| "Admission V4 encoded block omitted Completion".to_owned())?;
        let completion_index = u64::try_from(completion_index)
            .map_err(|_| "Admission V4 completion index does not fit u64".to_owned())?;
        for index in prefix..completion_index {
            let index = usize::try_from(index)
                .map_err(|_| "Admission V4 append index does not fit usize".to_owned())?;
            let record = records
                .get(index)
                .ok_or_else(|| "Admission V4 append index is absent".to_owned())?;
            append_raw(&mut self.data_file, record)?;
        }
        self.data_file
            .sync_all()
            .map_err(|why| format!("cannot sync Admission V4 evidence prefix: {why}"))?;
        append_raw(
            &mut self.data_file,
            records
                .last()
                .ok_or_else(|| "Admission V4 encoded block is empty".to_owned())?,
        )?;
        self.data_file
            .sync_all()
            .map_err(|why| format!("cannot sync Admission V4 Completion: {why}"))?;
        self.root_file
            .sync_all()
            .map_err(|why| format!("cannot sync Admission V4 directory: {why}"))?;
        self.data_generation =
            file_generation(&self.data_file, &self.data_path, self.bounds.file_bytes)?;
        self.scan()?;
        let receipt = self
            .receipts
            .get(&prepared.source.block_id)
            .copied()
            .ok_or_else(|| "Admission V4 appended block is absent after scan".to_owned())?;
        Ok((true, receipt))
    }

    fn read_complete(
        &mut self,
        receipt: PopulationAdmissionV4StructuralReceipt,
    ) -> Result<
        (CompletionV4, [FamilySourceV4; 2], Vec<DecisionRecordV4>),
        PopulationAdmissionV4Refusal,
    > {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot shared-lock Admission V4 projection: {why}"))?;
        let result = (|| {
            self.require_unchanged()?;
            let indexed = self
                .receipts
                .get(&receipt.block_id)
                .copied()
                .ok_or_else(|| "Admission V4 receipt is absent".to_owned())?;
            if indexed != receipt {
                return Err("Admission V4 receipt is stale or foreign".to_owned());
            }
            let data = decode_manifest(
                &read_record_at(&mut self.data_file, receipt.first_record)?,
                DATA_KIND,
            )?;
            let (nifty_sequence, nifty_block, nifty) = decode_family_record(&read_record_at(
                &mut self.data_file,
                receipt.first_record + 1,
            )?)?;
            let (bank_sequence, bank_block, bank) = decode_family_record(&read_record_at(
                &mut self.data_file,
                receipt.first_record + 2,
            )?)?;
            if nifty_sequence != receipt.sequence
                || bank_sequence != receipt.sequence
                || nifty_block != receipt.block_id
                || bank_block != receipt.block_id
            {
                return Err("Admission V4 family record crosswire".to_owned());
            }
            let mut decisions = Vec::new();
            decisions
                .try_reserve_exact(
                    usize::try_from(data.source.decision_count)
                        .map_err(|_| "Admission V4 decision count does not fit usize".to_owned())?,
                )
                .map_err(|why| format!("cannot reserve Admission V4 read decisions: {why}"))?;
            for ordinal in 0..data.source.decision_count {
                let (sequence, decision) = decode_decision(&read_record_at(
                    &mut self.data_file,
                    receipt.first_record + 3 + ordinal,
                )?)?;
                if sequence != receipt.sequence {
                    return Err("Admission V4 decision moved between blocks".to_owned());
                }
                decision.validate(&data.source)?;
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
                || completion.ordered_decision_digest != ordered_decision_digest(&decisions)?
                || [nifty, bank] != [data.source.nifty, data.source.banknifty]
            {
                return Err("Admission V4 complete block no longer reconciles".to_owned());
            }
            self.require_unchanged()?;
            Ok((completion, [nifty, bank], decisions))
        })();
        let unlocked = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot unlock Admission V4 projection: {why}"));
        combine(result, unlocked)
    }

    fn require_unchanged(&self) -> Result<(), PopulationAdmissionV4Refusal> {
        if named_identity(&self.root)? != self.root_identity
            || PlatformIdentity::of(
                &self
                    .root_file
                    .metadata()
                    .map_err(|why| format!("cannot stat held Admission V4 root: {why}"))?,
            ) != self.root_identity
            || file_generation(&self.lock_file, &self.lock_path, 0)? != self.lock_generation
            || file_generation(&self.data_file, &self.data_path, self.bounds.file_bytes)?
                != self.data_generation
        {
            return Err("Admission V4 retained file/root generation changed".to_owned());
        }
        Ok(())
    }
}

fn validate_prefix(
    file: &mut File,
    first: u64,
    count: u64,
    data: &CompletionV4,
) -> Result<(), PopulationAdmissionV4Refusal> {
    if count == 0 {
        return Err("Admission V4 trailing prefix omitted its Data record".to_owned());
    }
    if count >= 2 {
        let (sequence, block, family) = decode_family_record(&read_record_at(file, first + 1)?)?;
        if sequence != data.sequence || block != data.source.block_id || family != data.source.nifty
        {
            return Err("Admission V4 trailing NIFTY family differs".to_owned());
        }
    }
    if count >= 3 {
        let (sequence, block, family) = decode_family_record(&read_record_at(file, first + 2)?)?;
        if sequence != data.sequence
            || block != data.source.block_id
            || family != data.source.banknifty
        {
            return Err("Admission V4 trailing BANKNIFTY family differs".to_owned());
        }
    }
    let available_decisions = count.saturating_sub(3).min(data.source.decision_count);
    let mut last_family = AdmissionV4Family::Nifty;
    let mut next_nifty = 0_u64;
    let mut next_bank = 0_u64;
    for ordinal in 0..available_decisions {
        let (sequence, decision) = decode_decision(&read_record_at(file, first + 3 + ordinal)?)?;
        if sequence != data.sequence || decision.decision_sequence != ordinal {
            return Err("Admission V4 trailing decision order differs".to_owned());
        }
        decision.validate(&data.source)?;
        let expected = match decision.family {
            AdmissionV4Family::Nifty => &mut next_nifty,
            AdmissionV4Family::BankNifty => {
                last_family = AdmissionV4Family::BankNifty;
                &mut next_bank
            }
        };
        if decision.family == AdmissionV4Family::Nifty
            && last_family == AdmissionV4Family::BankNifty
            || decision.family_sequence != *expected
        {
            return Err("Admission V4 trailing family order differs".to_owned());
        }
        *expected = expected
            .checked_add(1)
            .ok_or_else(|| "Admission V4 trailing family count overflowed".to_owned())?;
    }
    if count > 3 + data.source.decision_count {
        return Err("Admission V4 trailing prefix contains an unaccounted Completion".to_owned());
    }
    Ok(())
}

fn validate_complete_block(
    file: &mut File,
    first: u64,
    data: &CompletionV4,
) -> Result<PopulationAdmissionV4StructuralReceipt, PopulationAdmissionV4Refusal> {
    let block_records = data
        .source
        .decision_count
        .checked_add(4)
        .ok_or_else(|| "Admission V4 block record count overflowed".to_owned())?;
    validate_prefix(file, first, block_records - 1, data)?;
    let completion = decode_manifest(
        &read_record_at(file, first + block_records - 1)?,
        COMPLETION_KIND,
    )?;
    if &completion != data {
        return Err("Admission V4 Data and Completion records differ".to_owned());
    }
    let mut decisions = Vec::new();
    decisions
        .try_reserve_exact(
            usize::try_from(data.source.decision_count)
                .map_err(|_| "Admission V4 decision count does not fit usize".to_owned())?,
        )
        .map_err(|why| format!("cannot reserve Admission V4 validation decisions: {why}"))?;
    for ordinal in 0..data.source.decision_count {
        let (_, decision) = decode_decision(&read_record_at(file, first + 3 + ordinal)?)?;
        decisions.push(decision);
    }
    if ordered_decision_digest(&decisions)? != data.ordered_decision_digest {
        return Err("Admission V4 ordered decision digest differs".to_owned());
    }
    Ok(PopulationAdmissionV4StructuralReceipt {
        sequence: data.sequence,
        first_record: first,
        block_id: data.source.block_id,
        completion_id: data.completion_id,
        nifty_terminal: data.source.nifty.terminal,
        banknifty_terminal: data.source.banknifty.terminal,
        nifty_candidate_count: data.source.nifty.candidate_count,
        banknifty_candidate_count: data.source.banknifty.candidate_count,
        decision_count: data.source.decision_count,
    })
}

fn require_exact_records(
    file: &mut File,
    first: u64,
    prepared: &PreparedPopulationAdmissionV4,
    sequence: u64,
) -> Result<(), PopulationAdmissionV4Refusal> {
    let expected = encoded_block(prepared, sequence)?;
    for (offset, raw) in expected.iter().enumerate() {
        let offset = u64::try_from(offset)
            .map_err(|_| "Admission V4 comparison offset does not fit u64".to_owned())?;
        if read_record_at(file, first + offset)? != *raw {
            return Err("Admission V4 existing bytes differ from exact preparation".to_owned());
        }
    }
    Ok(())
}

fn record_offset(index: u64) -> Result<u64, PopulationAdmissionV4Refusal> {
    (HEADER_BYTES as u64)
        .checked_add(
            index
                .checked_mul(RECORD_BYTES as u64)
                .ok_or_else(|| "Admission V4 record offset overflowed".to_owned())?,
        )
        .ok_or_else(|| "Admission V4 record address overflowed".to_owned())
}

fn read_record_at(
    file: &mut File,
    index: u64,
) -> Result<[u8; RECORD_BYTES], PopulationAdmissionV4Refusal> {
    let mut raw = [0_u8; RECORD_BYTES];
    file.seek(SeekFrom::Start(record_offset(index)?))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| format!("cannot read Admission V4 record {index}: {why}"))?;
    Ok(raw)
}

fn append_raw(
    file: &mut File,
    raw: &[u8; RECORD_BYTES],
) -> Result<(), PopulationAdmissionV4Refusal> {
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(raw))
        .map_err(|why| format!("cannot append Admission V4 record: {why}"))
}

fn open_root(
    root: &Path,
) -> Result<(PathBuf, File, PlatformIdentity), PopulationAdmissionV4Refusal> {
    let metadata = std::fs::symlink_metadata(root)
        .map_err(|why| format!("cannot stat Admission V4 root: {why}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("Admission V4 root must be an existing nonsymlink directory".to_owned());
    }
    let canonical = std::fs::canonicalize(root)
        .map_err(|why| format!("cannot canonicalize Admission V4 root: {why}"))?;
    let file =
        File::open(&canonical).map_err(|why| format!("cannot open Admission V4 root: {why}"))?;
    let identity = PlatformIdentity::of(
        &file
            .metadata()
            .map_err(|why| format!("cannot stat held Admission V4 root: {why}"))?,
    );
    if named_identity(&canonical)? != identity {
        return Err("Admission V4 root changed while opening".to_owned());
    }
    Ok((canonical, file, identity))
}

fn open_child(path: &Path, writable: bool) -> Result<(File, bool), PopulationAdmissionV4Refusal> {
    if let Ok(metadata) = std::fs::symlink_metadata(path)
        && (metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err("Admission V4 child is not a regular nonsymlink file".to_owned());
    }
    let mut options = OpenOptions::new();
    options.read(true).write(writable).create(writable);
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    options.custom_flags(O_NOFOLLOW_FLAG);
    let existed = path.exists();
    let file = options
        .open(path)
        .map_err(|why| format!("cannot open Admission V4 child: {why}"))?;
    let metadata = file
        .metadata()
        .map_err(|why| format!("cannot stat Admission V4 child: {why}"))?;
    if !metadata.is_file() || named_identity(path)? != PlatformIdentity::of(&metadata) {
        return Err("Admission V4 named child differs from held file".to_owned());
    }
    Ok((file, !existed))
}

fn named_identity(path: &Path) -> Result<PlatformIdentity, PopulationAdmissionV4Refusal> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|why| format!("cannot stat named Admission V4 path: {why}"))?;
    if metadata.file_type().is_symlink() {
        return Err("Admission V4 named path became a symlink".to_owned());
    }
    Ok(PlatformIdentity::of(&metadata))
}

fn file_generation(
    file: &File,
    path: &Path,
    maximum: u64,
) -> Result<FileGeneration, PopulationAdmissionV4Refusal> {
    let metadata = file
        .metadata()
        .map_err(|why| format!("cannot stat Admission V4 file: {why}"))?;
    let identity = PlatformIdentity::of(&metadata);
    if named_identity(path)? != identity {
        return Err("Admission V4 named file was replaced".to_owned());
    }
    if (maximum == 0 && metadata.len() != 0) || (maximum != 0 && metadata.len() > maximum) {
        return Err(format!(
            "Admission V4 file has {} bytes above bound {maximum}",
            metadata.len()
        ));
    }
    let mut clone = file
        .try_clone()
        .map_err(|why| format!("cannot clone Admission V4 file for hashing: {why}"))?;
    clone
        .seek(SeekFrom::Start(0))
        .map_err(|why| format!("cannot rewind Admission V4 file: {why}"))?;
    let mut hasher = Hasher::new();
    hasher.update(GENERATION_DOMAIN);
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    loop {
        let count = clone
            .read(&mut buffer)
            .map_err(|why| format!("cannot hash Admission V4 file: {why}"))?;
        if count == 0 {
            break;
        }
        let bytes = buffer
            .get(..count)
            .ok_or_else(|| "Admission V4 hash read exceeded its buffer".to_owned())?;
        hasher.update(bytes);
    }
    let after = file
        .metadata()
        .map_err(|why| format!("cannot restat Admission V4 file: {why}"))?;
    if PlatformIdentity::of(&after) != identity
        || after.len() != metadata.len()
        || named_identity(path)? != identity
    {
        return Err("Admission V4 file changed while hashing".to_owned());
    }
    Ok(FileGeneration {
        identity,
        len: metadata.len(),
        digest: hasher.finalize(),
    })
}

fn combine<T>(
    result: Result<T, PopulationAdmissionV4Refusal>,
    unlock: Result<(), PopulationAdmissionV4Refusal>,
) -> Result<T, PopulationAdmissionV4Refusal> {
    match (result, unlock) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(why), _) | (Ok(_), Err(why)) => Err(why),
    }
}

/// Persists, freshly reopens and returns a source-retaining Admission V4 authority.
pub(crate) fn commit_population_admission_v4(
    root: &Path,
    bounds: PopulationAdmissionV4Bounds,
    prepared: PreparedPopulationAdmissionV4,
) -> Result<PopulationAdmissionV4Commit, PopulationAdmissionV4Refusal> {
    prepared.validate()?;
    let mut writer = PopulationAdmissionV4Ledger::open_write(root, bounds)?;
    let (written, receipt) = writer.append(&prepared)?;
    drop(writer);
    let mut ledger = PopulationAdmissionV4Ledger::open_read(root, bounds)?;
    let reopened = ledger
        .receipts
        .get(&prepared.source.block_id)
        .copied()
        .ok_or_else(|| "Admission V4 fresh reopen omitted committed block".to_owned())?;
    if reopened != receipt {
        return Err("Admission V4 fresh reopen changed structural receipt".to_owned());
    }
    let (completion, families, decisions) = ledger.read_complete(reopened)?;
    if completion != CompletionV4::from_prepared(reopened.sequence, &prepared)?
        || families != [prepared.source.nifty, prepared.source.banknifty]
        || decisions != prepared.decisions
    {
        return Err("Admission V4 fresh reopen differs from exact preparation".to_owned());
    }
    let authority = PopulationAdmissionV4Authority {
        receipt: reopened,
        prepared,
        ledger,
    };
    Ok(if written {
        PopulationAdmissionV4Commit::Written(authority)
    } else {
        PopulationAdmissionV4Commit::Reused(authority)
    })
}

/// Controlled test-only Admission authority for successor codec fixtures.
#[cfg(test)]
pub(crate) fn population_finalization_v4_test_admission_for(
    root: &Path,
    bounds: PopulationAdmissionV4Bounds,
    nifty_terminal: AdmissionV4FamilyTerminal,
    banknifty_terminal: AdmissionV4FamilyTerminal,
    salt: u8,
) -> Result<PopulationAdmissionV4Authority, PopulationAdmissionV4Refusal> {
    commit_population_admission_v4(
        root,
        bounds,
        tests::prepared(nifty_terminal, banknifty_terminal, salt),
    )
    .map(PopulationAdmissionV4Commit::into_authority)
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

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new(label: &str) -> Self {
            let serial = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "brutex-population-admission-v4-{}-{label}-{serial}",
                std::process::id()
            ));
            std::fs::create_dir(&path).expect("create Admission V4 test root");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            if self.0.exists() {
                std::fs::remove_dir_all(&self.0).expect("remove Admission V4 test root");
            }
        }
    }

    fn digest(tag: u8) -> [u8; 32] {
        let mut value = [tag.max(1); 32];
        value[31] = tag.max(1).wrapping_add(1);
        value
    }

    fn bounds() -> PopulationAdmissionV4Bounds {
        PopulationAdmissionV4Bounds::new(8, 16, 2 * 1_024 * 1_024)
            .expect("Admission V4 fixture bounds")
    }

    fn family(
        family: AdmissionV4Family,
        terminal: AdmissionV4FamilyTerminal,
        tag: u8,
    ) -> FamilySourceV4 {
        let candidate_count = match terminal {
            AdmissionV4FamilyTerminal::Evaluated => 2,
            AdmissionV4FamilyTerminal::InsufficientForCscv => 1,
            AdmissionV4FamilyTerminal::NaturallyExtinct => 0,
        };
        let evaluated = terminal != AdmissionV4FamilyTerminal::NaturallyExtinct;
        let mut value = FamilySourceV4 {
            family,
            terminal,
            statistics_family_id: digest(tag),
            candidate_universe_id: digest(tag + 1),
            candidate_completion_digest: digest(tag + 2),
            candidate_ordered_row_digest: digest(tag + 3),
            pre_admission_authority_id: digest(tag + 4),
            observation_authority_id: if evaluated { [0; 32] } else { digest(tag + 5) },
            observation_identity: if evaluated {
                digest(tag + 6)
            } else {
                digest(tag + 5)
            },
            observation_policy_digest: digest(tag + 7),
            observation_data_digest: if evaluated { [0; 32] } else { digest(tag + 8) },
            observation_completion_digest: if evaluated { [0; 32] } else { digest(tag + 9) },
            base_completion_id: digest(tag + 10),
            signal: SignalSourceV4 {
                digest: digest(tag + 11),
                bars: 16,
                first_ts_micros: 1,
                last_ts_micros: 16,
                column_digest: digest(tag + 12),
            },
            grid: GridSourceV4 {
                long_policy: digest(tag + 13),
                long_resolution: digest(tag + 14),
                short_policy: digest(tag + 15),
                short_resolution: digest(tag + 16),
                composite: digest(tag + 17),
            },
            search: SearchSourceV4 {
                member_id: digest(tag + 18),
                source_authority_id: digest(tag + 19),
                validation_policy_id: digest(tag + 20),
                validation_family_id: digest(tag + 21),
                walk_id: digest(tag + 22),
                fold_count: 4,
                decided_folds: 2,
                profitable_oos_folds: 1,
                aggregate_oos_paisa: 1,
                evaluated_population_cells: candidate_count,
            },
            statistics_candidate_digest: if candidate_count == 0 {
                [0; 32]
            } else {
                digest(tag + 23)
            },
            statistics_period_digest: if candidate_count == 0 {
                [0; 32]
            } else {
                digest(tag + 24)
            },
            statistics_split_digest: if candidate_count == 0 {
                [0; 32]
            } else {
                digest(tag + 25)
            },
            candidate_count,
            decision_count: if terminal == AdmissionV4FamilyTerminal::Evaluated {
                candidate_count
            } else {
                0
            },
            family_id: [0; 32],
        };
        value.family_id = derive_family_id(&value);
        value.validate().expect("fixture family validates");
        value
    }

    fn runner_bytes(
        semantic: [u8; 32],
        row: [u8; 32],
        family: AdmissionV4Family,
        global: u64,
        family_sequence: u64,
    ) -> (
        [u8; RUNNER_DECISION_BYTES],
        AdmissionV4DecisionStatus,
        [u8; RUNNER_POLICY_BYTES],
    ) {
        let legacy_family = match family {
            AdmissionV4Family::Nifty => crate::population_admission_v3::AdmissionV3Family::Nifty,
            AdmissionV4Family::BankNifty => {
                crate::population_admission_v3::AdmissionV3Family::BankNifty
            }
        };
        let outer =
            crate::population_admission_v3::population_v5_test_canonical_admission_record_for(
                semantic,
                row,
                legacy_family,
                global,
                family_sequence,
            );
        let verified =
            crate::population_admission_v3::verify_population_v5_canonical_record(&outer)
                .expect("Admission V3 fixture arithmetic verifies");
        let runner = *verified.runner_decision();
        let arithmetic = AdmissionV3ArithmeticProjection::verify_decision_record_detached(&runner)
            .expect("Runner fixture verifies");
        let policy: [u8; RUNNER_POLICY_BYTES] = runner[RUNNER_HEADER_BYTES..RUNNER_EVIDENCE_OFFSET]
            .try_into()
            .expect("Runner fixture policy width");
        (
            runner,
            AdmissionV4DecisionStatus::from_runner(arithmetic.status()),
            policy,
        )
    }

    pub(super) fn prepared(
        nifty_terminal: AdmissionV4FamilyTerminal,
        bank_terminal: AdmissionV4FamilyTerminal,
        salt: u8,
    ) -> PreparedPopulationAdmissionV4 {
        let nifty = family(AdmissionV4Family::Nifty, nifty_terminal, 20 + salt);
        let banknifty = family(AdmissionV4Family::BankNifty, bank_terminal, 80 + salt);
        let first_family = if nifty.decision_count > 0 {
            AdmissionV4Family::Nifty
        } else {
            AdmissionV4Family::BankNifty
        };
        let (sample_runner, _, policy) =
            runner_bytes(digest(180 + salt), digest(181 + salt), first_family, 0, 0);
        let _ = sample_runner;
        let mut source = BlockSourceV4 {
            rung_seconds: 300,
            horizon_bars: 32,
            requested_span: RequestedSpanIdentityV1::new(2024, 1, 2024, 1).expect("fixture span"),
            feed_digest: digest(1 + salt),
            source_commit_digest: digest(2 + salt),
            calendar_policy_digest: digest(3 + salt),
            daily_reference_policy_digest: digest(4 + salt),
            vocabulary_digest: digest(5 + salt),
            evaluation_policy_digest: digest(6 + salt),
            statistics_authority_id: digest(7 + salt),
            statistics_completion_digest: digest(8 + salt),
            statistics_policy_digest: digest(9 + salt),
            base_pair_id: digest(10 + salt),
            search_pair_id: digest(11 + salt),
            policy,
            policy_digest: hash_slices(POLICY_DOMAIN, &[&policy]),
            nifty,
            banknifty,
            candidate_count: nifty.candidate_count + banknifty.candidate_count,
            decision_count: nifty.decision_count + banknifty.decision_count,
            block_id: [0; 32],
        };
        source.block_id = derive_block_id(&source);
        source.validate().expect("fixture source validates");
        let mut decisions = Vec::new();
        for (family, family_source, statistics_offset) in [
            (AdmissionV4Family::Nifty, source.nifty, 0_u64),
            (
                AdmissionV4Family::BankNifty,
                source.banknifty,
                source.nifty.candidate_count,
            ),
        ] {
            for family_sequence in 0..family_source.decision_count {
                let decision_sequence =
                    u64::try_from(decisions.len()).expect("fixture decision length");
                let decision_tag =
                    u8::try_from(decision_sequence).expect("fixture decision sequence fits u8");
                let semantic = digest(190_u8.wrapping_add(salt).wrapping_add(decision_tag));
                let row = digest(200_u8.wrapping_add(salt).wrapping_add(decision_tag));
                let (runner_decision, status, actual_policy) =
                    runner_bytes(semantic, row, family, decision_sequence, family_sequence);
                assert_eq!(actual_policy, source.policy);
                let mut decision = DecisionRecordV4 {
                    block_id: source.block_id,
                    decision_sequence,
                    statistics_sequence: statistics_offset + family_sequence,
                    family,
                    family_sequence,
                    status,
                    candidate_semantic_id: semantic,
                    candidate_row_digest: row,
                    pre_admission_authority_id: family_source.pre_admission_authority_id,
                    statistics_period_digest: digest(210_u8.wrapping_add(decision_tag)),
                    statistics_split_digest: digest(220_u8.wrapping_add(decision_tag)),
                    search_member_id: family_source.search.member_id,
                    base_evidence_id: digest(230_u8.wrapping_add(decision_tag)),
                    runner_decision,
                    runner_decision_digest: hash_slices(
                        RUNNER_DECISION_DOMAIN,
                        &[&runner_decision],
                    ),
                    evidence_digest: hash_slices(
                        RUNNER_EVIDENCE_DOMAIN,
                        &[&runner_decision[RUNNER_EVIDENCE_OFFSET..RUNNER_VERDICT_OFFSET]],
                    ),
                    verdict_digest: hash_slices(
                        RUNNER_VERDICT_DOMAIN,
                        &[&runner_decision[RUNNER_VERDICT_OFFSET..]],
                    ),
                    decision_id: [0; 32],
                };
                decision.decision_id = derive_decision_id(&decision);
                decision
                    .validate(&source)
                    .expect("fixture decision validates");
                decisions.push(decision);
            }
        }
        let value = PreparedPopulationAdmissionV4 { source, decisions };
        value.validate().expect("fixture preparation validates");
        value
    }

    #[test]
    fn terminals_preserve_both_mixed_orientations_insufficient_and_all_extinct() {
        for (nifty, bank, candidates, decisions) in [
            (
                AdmissionV4FamilyTerminal::Evaluated,
                AdmissionV4FamilyTerminal::NaturallyExtinct,
                2,
                2,
            ),
            (
                AdmissionV4FamilyTerminal::NaturallyExtinct,
                AdmissionV4FamilyTerminal::Evaluated,
                2,
                2,
            ),
            (
                AdmissionV4FamilyTerminal::InsufficientForCscv,
                AdmissionV4FamilyTerminal::NaturallyExtinct,
                1,
                0,
            ),
            (
                AdmissionV4FamilyTerminal::NaturallyExtinct,
                AdmissionV4FamilyTerminal::NaturallyExtinct,
                0,
                0,
            ),
        ] {
            let salt = u8::try_from(candidates).expect("fixture Candidate count fits u8") + 1;
            let value = prepared(nifty, bank, salt);
            assert_eq!(value.source.candidate_count, candidates);
            assert_eq!(value.source.decision_count, decisions);
            assert_eq!(
                u64::try_from(value.decisions.len()).expect("fixture decision count fits u64"),
                decisions
            );
            if nifty != AdmissionV4FamilyTerminal::Evaluated {
                assert_eq!(value.source.nifty.decision_count, 0);
            }
            if bank != AdmissionV4FamilyTerminal::Evaluated {
                assert_eq!(value.source.banknifty.decision_count, 0);
            }
        }
    }

    #[test]
    fn receipt_last_write_reopen_reuse_and_finalization_projection_are_exact() {
        let root = TestRoot::new("write-reuse");
        let value = prepared(
            AdmissionV4FamilyTerminal::Evaluated,
            AdmissionV4FamilyTerminal::NaturallyExtinct,
            2,
        );
        let mut written = commit_population_admission_v4(root.path(), bounds(), value.clone())
            .expect("write Admission V4");
        assert!(matches!(&written, PopulationAdmissionV4Commit::Written(_)));
        let projection = written
            .authority_mut()
            .finalization_projection()
            .expect("Finalization projection");
        assert_eq!(projection.receipt().decision_count(), 2);
        assert_eq!(
            projection.families()[0].terminal(),
            AdmissionV4FamilyTerminal::Evaluated
        );
        assert_eq!(
            projection.families()[1].terminal(),
            AdmissionV4FamilyTerminal::NaturallyExtinct
        );
        assert_eq!(projection.decisions().len(), 2);
        let nifty = &projection.families()[0];
        assert_eq!(nifty.family_id(), value.source.nifty.family_id);
        assert_eq!(
            nifty.search_source_authority_id(),
            value.source.nifty.search.source_authority_id
        );
        assert_eq!(
            nifty.search_validation_policy_id(),
            value.source.nifty.search.validation_policy_id
        );
        assert_eq!(
            nifty.search_validation_family_id(),
            value.source.nifty.search.validation_family_id
        );
        assert_eq!(nifty.search_walk_id(), value.source.nifty.search.walk_id);
        assert_eq!(
            nifty.statistics_candidate_digest(),
            value.source.nifty.statistics_candidate_digest
        );
        assert_eq!(
            nifty.signal_column_digest(),
            value.source.nifty.signal.column_digest
        );
        assert_eq!(
            nifty.grid_composite_digest(),
            value.source.nifty.grid.composite
        );
        for decision in projection.decisions() {
            assert_eq!(decision.status(), AdmissionV4DecisionStatus::Admitted);
            assert_eq!(decision.verdict().status(), AdmissionStatusV1::Admitted);
        }
        let reused = commit_population_admission_v4(root.path(), bounds(), value)
            .expect("reuse Admission V4");
        assert!(matches!(&reused, PopulationAdmissionV4Commit::Reused(_)));
    }

    #[test]
    fn exact_trailing_prefix_recovers_but_foreign_or_ragged_prefix_refuses() {
        let root = TestRoot::new("prefix");
        let value = prepared(
            AdmissionV4FamilyTerminal::Evaluated,
            AdmissionV4FamilyTerminal::NaturallyExtinct,
            3,
        );
        let records = encoded_block(&value, 0).expect("encode fixture block");
        let mut file = File::create(root.path().join(DATA_FILE)).expect("create prefix data");
        file.write_all(&header()).expect("write header");
        for raw in &records[..3] {
            file.write_all(raw).expect("write exact prefix");
        }
        file.sync_all().expect("sync exact prefix");
        drop(file);
        let committed = commit_population_admission_v4(root.path(), bounds(), value)
            .expect("complete exact prefix");
        assert!(matches!(
            &committed,
            PopulationAdmissionV4Commit::Written(_)
        ));

        let foreign = TestRoot::new("foreign-prefix");
        let foreign_value = prepared(
            AdmissionV4FamilyTerminal::NaturallyExtinct,
            AdmissionV4FamilyTerminal::Evaluated,
            8,
        );
        let foreign_records = encoded_block(&foreign_value, 0).expect("encode foreign prefix");
        let mut file = File::create(foreign.path().join(DATA_FILE)).expect("create foreign data");
        file.write_all(&header()).expect("write foreign header");
        for raw in &foreign_records[..3] {
            file.write_all(raw).expect("write foreign prefix");
        }
        file.sync_all().expect("sync foreign prefix");
        drop(file);
        let retry = prepared(
            AdmissionV4FamilyTerminal::Evaluated,
            AdmissionV4FamilyTerminal::NaturallyExtinct,
            9,
        );
        assert!(commit_population_admission_v4(foreign.path(), bounds(), retry).is_err());

        let ragged = TestRoot::new("ragged");
        File::create(ragged.path().join(LOCK_FILE)).expect("create ragged lock");
        let mut file = File::create(ragged.path().join(DATA_FILE)).expect("create ragged data");
        file.write_all(&header()).expect("write ragged header");
        file.write_all(&[1]).expect("write ragged byte");
        file.sync_all().expect("sync ragged data");
        assert!(PopulationAdmissionV4Ledger::open_read(ragged.path(), bounds()).is_err());
    }

    #[test]
    fn reordered_family_and_corrupt_or_resealed_decision_refuse() {
        let reordered = TestRoot::new("reordered");
        let value = prepared(
            AdmissionV4FamilyTerminal::Evaluated,
            AdmissionV4FamilyTerminal::NaturallyExtinct,
            4,
        );

        let mut crosswired = value.clone();
        crosswired.decisions[0].search_member_id = crosswired.source.banknifty.search.member_id;
        crosswired.decisions[0].decision_id = derive_decision_id(&crosswired.decisions[0]);
        assert!(crosswired.validate().is_err());

        let mut candidate_crosswired = value.clone();
        candidate_crosswired.decisions[0].candidate_semantic_id = digest(247);
        assert!(candidate_crosswired.validate().is_err());

        let mut foreign_status = value.clone();
        foreign_status.decisions[0].status = AdmissionV4DecisionStatus::Rejected;
        foreign_status.decisions[0].decision_id = derive_decision_id(&foreign_status.decisions[0]);
        assert!(foreign_status.validate().is_err());

        let mut foreign_policy = value.clone();
        foreign_policy.source.policy[RUNNER_POLICY_BYTES - 1] ^= 1;
        foreign_policy.source.policy_digest =
            hash_slices(POLICY_DOMAIN, &[&foreign_policy.source.policy]);
        foreign_policy.source.block_id = derive_block_id(&foreign_policy.source);
        for decision in &mut foreign_policy.decisions {
            decision.block_id = foreign_policy.source.block_id;
            decision.decision_id = derive_decision_id(decision);
        }
        assert!(foreign_policy.validate().is_err());

        let mut records = encoded_block(&value, 0).expect("encode reordered block");
        records.swap(1, 2);
        let mut file =
            File::create(reordered.path().join(DATA_FILE)).expect("create reordered data");
        file.write_all(&header()).expect("write reordered header");
        for raw in &records {
            file.write_all(raw).expect("write reordered record");
        }
        file.sync_all().expect("sync reordered data");
        assert!(PopulationAdmissionV4Ledger::open_read(reordered.path(), bounds()).is_err());

        let wrong_sequence = TestRoot::new("wrong-sequence");
        let records = encoded_block(&value, 1).expect("encode noncanonical sequence");
        let mut file =
            File::create(wrong_sequence.path().join(DATA_FILE)).expect("create sequence data");
        file.write_all(&header()).expect("write sequence header");
        for raw in &records {
            file.write_all(raw).expect("write sequence record");
        }
        file.sync_all().expect("sync sequence data");
        assert!(PopulationAdmissionV4Ledger::open_read(wrong_sequence.path(), bounds()).is_err());

        let corrupt = TestRoot::new("corrupt");
        commit_population_admission_v4(corrupt.path(), bounds(), value.clone())
            .expect("write corrupt fixture");
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(corrupt.path().join(DATA_FILE))
            .expect("open corrupt data");
        let offset = record_offset(3).expect("decision offset") + 160;
        file.seek(SeekFrom::Start(offset))
            .expect("seek corrupt byte");
        file.write_all(&[0xff]).expect("write corrupt byte");
        file.sync_all().expect("sync corrupt byte");
        assert!(PopulationAdmissionV4Ledger::open_read(corrupt.path(), bounds()).is_err());

        let resealed = TestRoot::new("resealed");
        commit_population_admission_v4(resealed.path(), bounds(), value)
            .expect("write resealed fixture");
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(resealed.path().join(DATA_FILE))
            .expect("open resealed data");
        let mut raw = read_record_at(&mut file, 3).expect("read decision record");
        raw[80] ^= 1;
        let seal = hash_slices(RECORD_DOMAIN, &[&raw[..PAYLOAD_BYTES]]);
        raw[PAYLOAD_BYTES..].copy_from_slice(&seal);
        file.seek(SeekFrom::Start(record_offset(3).expect("decision offset")))
            .expect("seek resealed record");
        file.write_all(&raw).expect("write resealed record");
        file.sync_all().expect("sync resealed record");
        assert!(PopulationAdmissionV4Ledger::open_read(resealed.path(), bounds()).is_err());
    }

    #[test]
    fn stale_same_length_and_named_path_replacement_refuse() {
        let root = TestRoot::new("stale");
        let value = prepared(
            AdmissionV4FamilyTerminal::Evaluated,
            AdmissionV4FamilyTerminal::NaturallyExtinct,
            5,
        );
        let mut authority = commit_population_admission_v4(root.path(), bounds(), value)
            .expect("write stale fixture")
            .into_authority();
        let mut mutator = OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.path().join(DATA_FILE))
            .expect("open stale mutator");
        mutator
            .seek(SeekFrom::Start(
                record_offset(3).expect("decision offset") + 160,
            ))
            .expect("seek stale byte");
        mutator.write_all(&[0xfe]).expect("write stale byte");
        mutator.sync_all().expect("sync stale byte");
        assert!(authority.finalization_projection().is_err());

        #[cfg(unix)]
        {
            let replacement = TestRoot::new("replacement");
            let value = prepared(
                AdmissionV4FamilyTerminal::Evaluated,
                AdmissionV4FamilyTerminal::NaturallyExtinct,
                6,
            );
            let mut authority = commit_population_admission_v4(replacement.path(), bounds(), value)
                .expect("write replacement fixture")
                .into_authority();
            let named = replacement.path().join(DATA_FILE);
            let bytes = std::fs::read(&named).expect("read replacement bytes");
            std::fs::rename(&named, replacement.path().join("displaced.bin"))
                .expect("displace data file");
            let mut next = File::create(&named).expect("create replacement file");
            next.write_all(&bytes).expect("write replacement bytes");
            next.sync_all().expect("sync replacement bytes");
            assert!(authority.finalization_projection().is_err());
        }
    }
}
