//! Version-separated mixed-family Population Statistics V3 authority.
//!
//! Statistics V2 remains byte-for-byte unchanged and continues to require two
//! evaluated Observation V1 families.  This successor is the only boundary
//! that may join one normally evaluated Observation V1 family with one exact,
//! authenticated natural-extinction Observation V2 family (or two truthful
//! natural extinctions).  An extinct family has no candidate rows and no
//! numeric White, SPA, Romano--Wolf, Wilson or PBO substitute.
//!
//! One block is `Data`, exact `Family(NIFTY)`, exact `Family(BANKNIFTY)`, every
//! real candidate statistic in family order, then `Completion` last.  The
//! candidate rows and both family records are synchronized before Completion.
//! Reopen revalidates terminal/cardinality/statistic presence, canonical order,
//! candidate digests and the duplicate receipt.  Opening and generation checks
//! scan bounded bytes; bootstrap preparation is explicitly input-dependent.
//! Only fixed-record offset arithmetic is worst-case O(1).

#![expect(
    clippy::large_types_passed_by_value,
    reason = "V3 canonical snapshots are fixed-size Copy evidence; preserving their versioned by-value API cannot grow with population cardinality"
)]

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;
#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
use std::os::unix::fs::OpenOptionsExt as _;

use brutex_core::blake3::Hasher;
use runner::admission::{AdmissionExactProbabilityV2, AdmissionStatisticsDraftV3};

use crate::candidate_universe::CANDIDATE_SIGNAL_RUNGS_SECONDS_V1;
use crate::population::{InstrumentFamilyV1, RequestedSpanIdentityV1};
use crate::population_observations_v1::{
    CandidateFamilyObservationsV1, ObservationAuthorityCommitV2,
    ObservationNaturalExtinctionSourceV2, ProducedObservationAuthorityV2,
};
use crate::population_statistics_v2::{
    PopulationCscvPboV2, PopulationFamilyTestV2, PopulationStatisticsProcedureV2,
    SingleFamilyCandidateStatisticsV3, SingleFamilyStatisticsV3Projection,
    prepare_single_family_statistics_v3,
};
use crate::pre_admission_data::{PreAdmissionDataReopenAuditV1, PreAdmissionDataV1};

/// Operator-facing refusal at the Statistics V3 boundary.
pub type PopulationStatisticsV3Refusal = String;

/// Header bytes in the Statistics V3 audit file.
pub const POPULATION_STATISTICS_V3_HEADER_BYTES: u64 = 64;
/// Fixed stride of every Statistics V3 record.
pub const POPULATION_STATISTICS_V3_RECORD_STRIDE: u64 = 1_024;

const HEADER_BYTES: usize = 64;
const PAYLOAD_BYTES: usize = 992;
const RECORD_BYTES: usize = 1_024;
const HEADER_VERSION: u32 = 3;
const HEADER_KIND: u32 = 1;
const RECORD_VERSION: u32 = 3;
const DATA_KIND: u32 = 1;
const FAMILY_KIND: u32 = 2;
const CANDIDATE_KIND: u32 = 3;
const COMPLETION_KIND: u32 = 4;
const HEADER_MAGIC: [u8; 16] = *b"BTX-POPSTATS-V3\0";
const DATA_FILE: &str = "population-statistics-v3-audit.bin";
const LOCK_FILE: &str = "population-statistics-v3-audit.lock";
const HEADER_DOMAIN: &[u8] = b"brutex-population-statistics-v3-header\0";
const RECORD_DOMAIN: &[u8] = b"brutex-population-statistics-v3-record\0";
const AUTHORITY_DOMAIN: &[u8] = b"brutex-population-statistics-v3-authority\0";
const FAMILY_DOMAIN: &[u8] = b"brutex-population-statistics-v3-family\0";
const CANDIDATE_ORDER_DOMAIN: &[u8] = b"brutex-population-statistics-v3-candidates\0";
const ACCEPTED_SESSION_DOMAIN: &[u8] = b"brutex-population-statistics-v3-sessions\0";
const POLICY_DOMAIN: &[u8] = b"brutex-population-statistics-v3-policy\0";
const GENERATION_DOMAIN: &[u8] = b"brutex-population-statistics-v3-generation\0";
const ADMISSION_V4_CSCV_POLICY_DOMAIN: &[u8] =
    b"brutex-population-statistics-v3-admission-v4-cscv-policy\0";
const READ_CHUNK_BYTES: usize = 16 * 1_024;
const LOCK_FILE_MAX_BYTES: u64 = 0;

#[cfg(any(target_os = "android", target_os = "linux"))]
const O_NOFOLLOW_FLAG: i32 = 0x20_000;
#[cfg(target_os = "macos")]
const O_NOFOLLOW_FLAG: i32 = 0x100;

const _: () = assert!(PAYLOAD_BYTES + 32 == RECORD_BYTES);

/// Exact terminal meaning for one index family in Statistics V3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatisticsFamilyTerminalV3 {
    /// Two or more candidates have complete White/SPA/Romano--Wolf/Wilson/PBO evidence.
    Evaluated,
    /// One candidate has real evidence, but relative CSCV placement is undefined.
    InsufficientForCscv,
    /// Candidate extinction was authenticated upstream; no numeric statistic exists.
    NaturallyExtinct,
}

impl StatisticsFamilyTerminalV3 {
    const fn code(self) -> u32 {
        match self {
            Self::Evaluated => 1,
            Self::InsufficientForCscv => 2,
            Self::NaturallyExtinct => 3,
        }
    }

    fn decode(value: u32) -> Result<Self, PopulationStatisticsV3Refusal> {
        match value {
            1 => Ok(Self::Evaluated),
            2 => Ok(Self::InsufficientForCscv),
            3 => Ok(Self::NaturallyExtinct),
            _ => Err(format!("Statistics V3 terminal {value} is unknown")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ExactFractionV3 {
    numerator: u64,
    denominator: u64,
}

impl ExactFractionV3 {
    fn new(numerator: u64, denominator: u64) -> Result<Self, PopulationStatisticsV3Refusal> {
        if denominator == 0 || numerator > denominator {
            return Err(format!(
                "Statistics V3 fraction {numerator}/{denominator} is outside [0,1]"
            ));
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    #[expect(
        clippy::cast_precision_loss,
        clippy::float_arithmetic,
        reason = "finite-resample projection is a statistic, never a price"
    )]
    fn bits(self) -> u64 {
        (self.numerator as f64 / self.denominator as f64).to_bits()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FamilyTestV3 {
    statistic_bits: u64,
    probability_bits: u64,
    probability: ExactFractionV3,
    family_digest: [u8; 32],
}

impl FamilyTestV3 {
    fn from_v2(value: PopulationFamilyTestV2) -> Result<Self, PopulationStatisticsV3Refusal> {
        let exact = value.exact_probability();
        let result = Self {
            statistic_bits: value.statistic().to_bits(),
            probability_bits: value.probability().to_bits(),
            probability: ExactFractionV3::new(exact.numerator(), exact.denominator())?,
            family_digest: value.family_digest(),
        };
        result.validate("family test")?;
        Ok(result)
    }

    fn validate(self, name: &str) -> Result<(), PopulationStatisticsV3Refusal> {
        require_finite(name, self.statistic_bits)?;
        require_finite(&format!("{name} probability"), self.probability_bits)?;
        require_nonzero(&format!("{name} family"), self.family_digest)?;
        if self.probability.bits() != self.probability_bits {
            return Err(format!(
                "Statistics V3 {name} exact probability differs from its bits"
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PboV3 {
    contributing_splits: u64,
    bottom_half_splits: u64,
    unrankable_splits: u64,
    probability_bits: u64,
    probability: ExactFractionV3,
}

impl PboV3 {
    fn from_v2(value: PopulationCscvPboV2) -> Result<Self, PopulationStatisticsV3Refusal> {
        let exact = value.exact_probability();
        let result = Self {
            contributing_splits: value.contributing_splits(),
            bottom_half_splits: value.bottom_half_splits(),
            unrankable_splits: value.unrankable_splits(),
            probability_bits: value.probability().to_bits(),
            probability: ExactFractionV3::new(exact.numerator(), exact.denominator())?,
        };
        result.validate(
            result
                .contributing_splits
                .checked_add(result.unrankable_splits)
                .ok_or_else(|| "Statistics V3 PBO split count overflowed".to_owned())?,
        )?;
        Ok(result)
    }

    fn validate(self, split_count: u64) -> Result<(), PopulationStatisticsV3Refusal> {
        if self.contributing_splits == 0
            || self.contributing_splits.checked_add(self.unrankable_splits) != Some(split_count)
            || self.bottom_half_splits > self.contributing_splits
            || self.probability.numerator != self.bottom_half_splits
            || self.probability.denominator != self.contributing_splits
            || self.probability.bits() != self.probability_bits
        {
            return Err("Statistics V3 PBO fields do not reconcile".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FamilyEvidenceV3 {
    family: InstrumentFamilyV1,
    terminal: StatisticsFamilyTerminalV3,
    pre_admission_sequence: u64,
    pre_admission_record_index: u64,
    pre_admission_authority_id: [u8; 32],
    candidate_universe_id: [u8; 32],
    candidate_completion_digest: [u8; 32],
    observation_authority_id: [u8; 32],
    observation_identity: [u8; 32],
    observation_policy_digest: [u8; 32],
    observation_data_digest: [u8; 32],
    observation_completion_digest: [u8; 32],
    accepted_session_digest: [u8; 32],
    layout_digest: [u8; 32],
    candidate_count: u64,
    period_count: u64,
    split_count: u64,
    segment_count: u32,
    white: Option<FamilyTestV3>,
    spa: Option<FamilyTestV3>,
    romano_wolf_family_digest: Option<[u8; 32]>,
    pbo: Option<PboV3>,
    ordered_candidate_digest: [u8; 32],
    ordered_period_digest: [u8; 32],
    ordered_split_digest: [u8; 32],
    identity: [u8; 32],
}

impl FamilyEvidenceV3 {
    fn validate(self) -> Result<(), PopulationStatisticsV3Refusal> {
        for (name, digest) in [
            ("pre-admission authority", self.pre_admission_authority_id),
            ("candidate universe", self.candidate_universe_id),
            ("candidate completion", self.candidate_completion_digest),
            ("observation policy", self.observation_policy_digest),
            ("family identity", self.identity),
        ] {
            require_nonzero(name, digest)?;
        }
        match self.terminal {
            StatisticsFamilyTerminalV3::Evaluated
            | StatisticsFamilyTerminalV3::InsufficientForCscv => {
                let expected_count = if self.terminal == StatisticsFamilyTerminalV3::Evaluated {
                    2
                } else {
                    1
                };
                if (self.terminal == StatisticsFamilyTerminalV3::Evaluated
                    && self.candidate_count < expected_count)
                    || (self.terminal == StatisticsFamilyTerminalV3::InsufficientForCscv
                        && self.candidate_count != expected_count)
                    || self.period_count < 2
                    || self.split_count == 0
                    || self.segment_count < 2
                    || self.observation_authority_id != [0; 32]
                    || self.observation_data_digest != [0; 32]
                    || self.observation_completion_digest != [0; 32]
                {
                    return Err(
                        "Statistics V3 evaluated terminal/cardinality/source shape differs"
                            .to_owned(),
                    );
                }
                for (name, digest) in [
                    ("observation identity", self.observation_identity),
                    ("accepted sessions", self.accepted_session_digest),
                    ("layout", self.layout_digest),
                    ("ordered candidates", self.ordered_candidate_digest),
                    ("ordered periods", self.ordered_period_digest),
                    ("ordered splits", self.ordered_split_digest),
                ] {
                    require_nonzero(name, digest)?;
                }
                self.white
                    .ok_or_else(|| "Statistics V3 evaluated family lacks White".to_owned())?
                    .validate("White")?;
                self.spa
                    .ok_or_else(|| "Statistics V3 evaluated family lacks SPA".to_owned())?
                    .validate("SPA")?;
                require_nonzero(
                    "Romano-Wolf family",
                    self.romano_wolf_family_digest.ok_or_else(|| {
                        "Statistics V3 evaluated family lacks Romano-Wolf".to_owned()
                    })?,
                )?;
                match (self.terminal, self.pbo) {
                    (StatisticsFamilyTerminalV3::Evaluated, Some(pbo)) => {
                        pbo.validate(self.split_count)?;
                    }
                    (StatisticsFamilyTerminalV3::InsufficientForCscv, None) => {}
                    _ => {
                        return Err(
                            "Statistics V3 PBO presence disagrees with terminal meaning".to_owned()
                        );
                    }
                }
            }
            StatisticsFamilyTerminalV3::NaturallyExtinct => {
                if self.candidate_count != 0
                    || self.period_count != 0
                    || self.split_count != 0
                    || self.segment_count != 0
                    || self.observation_authority_id == [0; 32]
                    || self.observation_identity != self.observation_authority_id
                    || self.observation_data_digest == [0; 32]
                    || self.observation_completion_digest == [0; 32]
                    || self.accepted_session_digest != [0; 32]
                    || self.layout_digest != [0; 32]
                    || self.white.is_some()
                    || self.spa.is_some()
                    || self.romano_wolf_family_digest.is_some()
                    || self.pbo.is_some()
                    || self.ordered_candidate_digest != [0; 32]
                    || self.ordered_period_digest != [0; 32]
                    || self.ordered_split_digest != [0; 32]
                {
                    return Err(
                        "Statistics V3 naturally extinct family fabricates rows or numeric evidence"
                            .to_owned(),
                    );
                }
            }
        }
        if family_identity(&self) != self.identity {
            return Err("Statistics V3 family identity does not recompute".to_owned());
        }
        Ok(())
    }
}

/// One complete real candidate-statistics row retained by Statistics V3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationStatisticsCandidateV3 {
    authority_id: [u8; 32],
    global_sequence: u64,
    family: InstrumentFamilyV1,
    family_sequence: u64,
    candidate_semantic_digest: [u8; 32],
    pre_admission_authority_id: [u8; 32],
    trades: u64,
    wins: u64,
    wilson_lower_bits: u64,
    romano_wolf_statistic_bits: u64,
    romano_wolf_rank: u64,
    romano_wolf_strict_exceedances: u64,
    romano_wolf_initial: ExactFractionV3,
    romano_wolf_adjusted: ExactFractionV3,
    ordered_period_digest: [u8; 32],
    ordered_split_digest: [u8; 32],
}

impl PopulationStatisticsCandidateV3 {
    /// Canonical global sequence; NIFTY precedes BANKNIFTY when nonempty.
    #[must_use]
    pub const fn sequence(self) -> u64 {
        self.global_sequence
    }

    /// Exact family.
    #[must_use]
    pub const fn family(self) -> InstrumentFamilyV1 {
        self.family
    }

    /// Candidate semantic identity.
    #[must_use]
    pub const fn candidate_semantic_digest(self) -> [u8; 32] {
        self.candidate_semantic_digest
    }

    /// Full-precision Wilson lower-bound projection.
    #[must_use]
    pub const fn wilson_lower(self) -> f64 {
        f64::from_bits(self.wilson_lower_bits)
    }

    fn validate(
        self,
        family: &FamilyEvidenceV3,
        expected_global: u64,
    ) -> Result<(), PopulationStatisticsV3Refusal> {
        if self.authority_id == [0; 32]
            || self.global_sequence != expected_global
            || self.family != family.family
            || self.family_sequence >= family.candidate_count
            || self.pre_admission_authority_id != family.pre_admission_authority_id
            || self.candidate_semantic_digest == [0; 32]
            || self.wins > self.trades
            || self.wilson_lower_bits != wilson_lower_bits_v3(self.wins, self.trades)
            || !f64::from_bits(self.romano_wolf_statistic_bits).is_finite()
            || self.romano_wolf_rank >= family.candidate_count
            || self.romano_wolf_strict_exceedances.checked_add(1)
                != Some(self.romano_wolf_initial.numerator)
            || self.romano_wolf_initial.denominator != self.romano_wolf_adjusted.denominator
            || self.romano_wolf_adjusted.numerator < self.romano_wolf_initial.numerator
            || self.ordered_period_digest == [0; 32]
            || self.ordered_split_digest == [0; 32]
        {
            return Err("Statistics V3 candidate fields do not reconcile".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ManifestV3 {
    sequence: u64,
    authority_id: [u8; 32],
    draws: u64,
    seed: u64,
    block_length: u64,
    rung_seconds: u32,
    horizon_bars: u32,
    requested_span: RequestedSpanIdentityV1,
    feed_digest: [u8; 32],
    source_commit_digest: [u8; 32],
    calendar_policy_digest: [u8; 32],
    daily_reference_policy_digest: [u8; 32],
    policy_digest: [u8; 32],
    nifty_family_identity: [u8; 32],
    banknifty_family_identity: [u8; 32],
    ordered_candidate_digest: [u8; 32],
    candidate_count: u64,
}

impl ManifestV3 {
    fn block_records(self) -> Result<u64, PopulationStatisticsV3Refusal> {
        self.candidate_count
            .checked_add(4)
            .ok_or_else(|| "Statistics V3 block record count overflowed".to_owned())
    }

    fn validate(self) -> Result<(), PopulationStatisticsV3Refusal> {
        if !CANDIDATE_SIGNAL_RUNGS_SECONDS_V1.contains(&self.rung_seconds) || self.horizon_bars == 0
        {
            return Err("Statistics V3 rung or horizon is outside production policy".to_owned());
        }
        RequestedSpanIdentityV1::new(
            self.requested_span.from_year(),
            self.requested_span.from_month(),
            self.requested_span.to_year(),
            self.requested_span.to_month(),
        )?;
        for (name, digest) in [
            ("authority", self.authority_id),
            ("feed", self.feed_digest),
            ("source commit", self.source_commit_digest),
            ("calendar policy", self.calendar_policy_digest),
            ("daily-reference policy", self.daily_reference_policy_digest),
            ("Statistics V3 policy", self.policy_digest),
            ("NIFTY family", self.nifty_family_identity),
            ("BANKNIFTY family", self.banknifty_family_identity),
        ] {
            require_nonzero(name, digest)?;
        }
        if self.policy_digest != statistics_v3_policy_digest()
            || (self.candidate_count == 0 && self.ordered_candidate_digest != [0; 32])
            || (self.candidate_count != 0 && self.ordered_candidate_digest == [0; 32])
            || manifest_identity(&self) != self.authority_id
        {
            return Err("Statistics V3 manifest identity or candidate digest differs".to_owned());
        }
        if self.candidate_count == 0 {
            if self.draws != 0 || self.seed != 0 || self.block_length != 0 {
                return Err(
                    "Statistics V3 all-extinct authority carries a fabricated procedure".to_owned(),
                );
            }
        } else if self.draws == 0 || self.block_length == 0 {
            return Err("Statistics V3 evaluated authority lacks a procedure".to_owned());
        }
        Ok(())
    }
}

/// Explicit scan and allocation ceilings for Statistics V3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationStatisticsV3Bounds {
    audits: u64,
    candidates_per_audit: u64,
    file_bytes: u64,
}

impl PopulationStatisticsV3Bounds {
    /// Creates explicit nonzero bounds.
    ///
    /// # Errors
    ///
    /// Refuses zero authority/candidate ceilings and a byte ceiling too small
    /// for the smallest receipt-last authority.
    pub fn new(
        max_audits: u64,
        max_candidates_per_audit: u64,
        max_file_bytes: u64,
    ) -> Result<Self, PopulationStatisticsV3Refusal> {
        let minimum = POPULATION_STATISTICS_V3_HEADER_BYTES
            .checked_add(4 * POPULATION_STATISTICS_V3_RECORD_STRIDE)
            .ok_or_else(|| "Statistics V3 minimum byte bound overflowed".to_owned())?;
        if max_audits == 0 || max_candidates_per_audit == 0 || max_file_bytes < minimum {
            return Err(format!(
                "Statistics V3 bounds require nonzero audits/candidates and at least {minimum} bytes"
            ));
        }
        Ok(Self {
            audits: max_audits,
            candidates_per_audit: max_candidates_per_audit,
            file_bytes: max_file_bytes,
        })
    }

    /// Maximum complete authorities admitted.
    #[must_use]
    pub const fn max_audits(self) -> u64 {
        self.audits
    }

    /// Maximum real candidate records per authority.
    #[must_use]
    pub const fn max_candidates_per_audit(self) -> u64 {
        self.candidates_per_audit
    }

    /// Maximum file bytes scanned or appended.
    #[must_use]
    pub const fn max_file_bytes(self) -> u64 {
        self.file_bytes
    }
}

/// Fresh read-only audit of one complete V3 authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationStatisticsV3ReopenAudit {
    first_record: u64,
    completion_digest: [u8; 32],
    manifest: ManifestV3,
    nifty_terminal: StatisticsFamilyTerminalV3,
    banknifty_terminal: StatisticsFamilyTerminalV3,
}

impl PopulationStatisticsV3ReopenAudit {
    /// Complete authority identity.
    #[must_use]
    pub const fn authority_id(self) -> [u8; 32] {
        self.manifest.authority_id
    }

    /// Logical append sequence.
    #[must_use]
    pub const fn sequence(self) -> u64 {
        self.manifest.sequence
    }

    /// Number of real candidate statistic rows.
    #[must_use]
    pub const fn candidate_count(self) -> u64 {
        self.manifest.candidate_count
    }

    /// NIFTY terminal meaning.
    #[must_use]
    pub const fn nifty_terminal(self) -> StatisticsFamilyTerminalV3 {
        self.nifty_terminal
    }

    /// BANKNIFTY terminal meaning.
    #[must_use]
    pub const fn banknifty_terminal(self) -> StatisticsFamilyTerminalV3 {
        self.banknifty_terminal
    }
}

/// Written or exact-reuse result after fresh reopen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopulationStatisticsV3Commit {
    /// New non-Completion records and then Completion were synchronized.
    Written(PopulationStatisticsV3ReopenAudit),
    /// The exact semantic block already existed.
    Reused(PopulationStatisticsV3ReopenAudit),
}

impl PopulationStatisticsV3Commit {
    /// Freshly reopened audit.
    #[must_use]
    pub const fn audit(self) -> PopulationStatisticsV3ReopenAudit {
        match self {
            Self::Written(audit) | Self::Reused(audit) => audit,
        }
    }
}

/// Opaque production capability; no public field or constructor accepts rows,
/// digests, terminal labels or numeric statistics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProducedPopulationStatisticsV3 {
    manifest: ManifestV3,
    nifty: FamilyEvidenceV3,
    banknifty: FamilyEvidenceV3,
    candidates: Vec<PopulationStatisticsCandidateV3>,
}

/// Opaque exact source for a later Admission successor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PopulationStatisticsAdmissionSourceV3 {
    audit: PopulationStatisticsV3ReopenAudit,
    nifty: FamilyEvidenceV3,
    banknifty: FamilyEvidenceV3,
    candidates: Vec<PopulationStatisticsCandidateV3>,
}

impl PopulationStatisticsAdmissionSourceV3 {
    pub(crate) const fn audit(&self) -> PopulationStatisticsV3ReopenAudit {
        self.audit
    }

    /// Exact common source terms authenticated by the Statistics V3 commit.
    #[must_use]
    pub(crate) const fn authority_projection(&self) -> PopulationStatisticsAuthorityProjectionV3 {
        PopulationStatisticsAuthorityProjectionV3 {
            authority_id: self.audit.manifest.authority_id,
            completion_digest: self.audit.completion_digest,
            rung_seconds: self.audit.manifest.rung_seconds,
            horizon_bars: self.audit.manifest.horizon_bars,
            requested_span: self.audit.manifest.requested_span,
            feed_digest: self.audit.manifest.feed_digest,
            source_commit_digest: self.audit.manifest.source_commit_digest,
            calendar_policy_digest: self.audit.manifest.calendar_policy_digest,
            daily_reference_policy_digest: self.audit.manifest.daily_reference_policy_digest,
            statistics_policy_digest: self.audit.manifest.policy_digest,
            draws: self.audit.manifest.draws,
            seed: self.audit.manifest.seed,
            block_length: self.audit.manifest.block_length,
            ordered_candidate_digest: self.audit.manifest.ordered_candidate_digest,
            candidate_count: self.audit.manifest.candidate_count,
        }
    }

    /// Exact ordered NIFTY family evidence.
    #[must_use]
    pub(crate) const fn nifty_family(&self) -> PopulationStatisticsFamilyProjectionV3 {
        PopulationStatisticsFamilyProjectionV3 { value: self.nifty }
    }

    /// Exact ordered BANKNIFTY family evidence.
    #[must_use]
    pub(crate) const fn banknifty_family(&self) -> PopulationStatisticsFamilyProjectionV3 {
        PopulationStatisticsFamilyProjectionV3 {
            value: self.banknifty,
        }
    }

    /// Reconstructs every real Candidate projection in canonical family order.
    ///
    /// `Evaluated` rows receive a complete Runner Admission V3 statistics
    /// draft. `InsufficientForCscv` rows deliberately carry no draft because
    /// genuine relative CSCV/PBO placement does not exist for one Candidate.
    /// `NaturallyExtinct` contributes no row at all.
    pub(crate) fn admission_v4_candidates(
        &self,
    ) -> Result<Vec<PopulationStatisticsAdmissionCandidateV4>, PopulationStatisticsV3Refusal> {
        self.audit.manifest.validate()?;
        self.nifty.validate()?;
        self.banknifty.validate()?;
        let nifty_familywise = match self.nifty.terminal {
            StatisticsFamilyTerminalV3::Evaluated => Some(familywise_romano_wolf_probability(
                &self.candidates,
                self.nifty,
            )?),
            StatisticsFamilyTerminalV3::InsufficientForCscv
            | StatisticsFamilyTerminalV3::NaturallyExtinct => None,
        };
        let banknifty_familywise = match self.banknifty.terminal {
            StatisticsFamilyTerminalV3::Evaluated => Some(familywise_romano_wolf_probability(
                &self.candidates,
                self.banknifty,
            )?),
            StatisticsFamilyTerminalV3::InsufficientForCscv
            | StatisticsFamilyTerminalV3::NaturallyExtinct => None,
        };
        let mut projected = Vec::new();
        projected
            .try_reserve_exact(self.candidates.len())
            .map_err(|why| format!("cannot reserve Statistics V3 Admission V4 rows: {why}"))?;
        for candidate in &self.candidates {
            let family = if candidate.family == InstrumentFamilyV1::Nifty {
                self.nifty
            } else {
                self.banknifty
            };
            candidate.validate(&family, candidate.global_sequence)?;
            let draft = match family.terminal {
                StatisticsFamilyTerminalV3::Evaluated => Some(admission_v4_draft(
                    self.audit.manifest,
                    family,
                    *candidate,
                    if family.family == InstrumentFamilyV1::Nifty {
                        nifty_familywise
                    } else {
                        banknifty_familywise
                    }
                    .ok_or_else(|| {
                        "Statistics V3 evaluated family lacks projected Romano-Wolf authority"
                            .to_owned()
                    })?,
                    self.audit.completion_digest,
                )?),
                StatisticsFamilyTerminalV3::InsufficientForCscv => None,
                StatisticsFamilyTerminalV3::NaturallyExtinct => {
                    return Err(
                        "Statistics V3 extinct family unexpectedly contributed a Candidate"
                            .to_owned(),
                    );
                }
            };
            projected.push(PopulationStatisticsAdmissionCandidateV4 {
                candidate: *candidate,
                draft,
            });
        }
        if u64::try_from(projected.len()).ok() != Some(self.audit.manifest.candidate_count) {
            return Err("Statistics V3 Admission V4 projection cardinality differs".to_owned());
        }
        Ok(projected)
    }
}

/// Exact common Statistics V3 identity projected for Admission V4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationStatisticsAuthorityProjectionV3 {
    authority_id: [u8; 32],
    completion_digest: [u8; 32],
    rung_seconds: u32,
    horizon_bars: u32,
    requested_span: RequestedSpanIdentityV1,
    feed_digest: [u8; 32],
    source_commit_digest: [u8; 32],
    calendar_policy_digest: [u8; 32],
    daily_reference_policy_digest: [u8; 32],
    statistics_policy_digest: [u8; 32],
    draws: u64,
    seed: u64,
    block_length: u64,
    ordered_candidate_digest: [u8; 32],
    candidate_count: u64,
}

impl PopulationStatisticsAuthorityProjectionV3 {
    pub(crate) const fn authority_id(self) -> [u8; 32] {
        self.authority_id
    }
    pub(crate) const fn completion_digest(self) -> [u8; 32] {
        self.completion_digest
    }
    pub(crate) const fn rung_seconds(self) -> u32 {
        self.rung_seconds
    }
    pub(crate) const fn horizon_bars(self) -> u32 {
        self.horizon_bars
    }
    pub(crate) const fn requested_span(self) -> RequestedSpanIdentityV1 {
        self.requested_span
    }
    pub(crate) const fn feed_digest(self) -> [u8; 32] {
        self.feed_digest
    }
    pub(crate) const fn source_commit_digest(self) -> [u8; 32] {
        self.source_commit_digest
    }
    pub(crate) const fn calendar_policy_digest(self) -> [u8; 32] {
        self.calendar_policy_digest
    }
    pub(crate) const fn daily_reference_policy_digest(self) -> [u8; 32] {
        self.daily_reference_policy_digest
    }
    pub(crate) const fn statistics_policy_digest(self) -> [u8; 32] {
        self.statistics_policy_digest
    }
    pub(crate) const fn draws(self) -> u64 {
        self.draws
    }
    pub(crate) const fn seed(self) -> u64 {
        self.seed
    }
    pub(crate) const fn block_length(self) -> u64 {
        self.block_length
    }
    pub(crate) const fn ordered_candidate_digest(self) -> [u8; 32] {
        self.ordered_candidate_digest
    }
    pub(crate) const fn candidate_count(self) -> u64 {
        self.candidate_count
    }
}

/// Exact one-family Statistics V3 lineage and terminal shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationStatisticsFamilyProjectionV3 {
    value: FamilyEvidenceV3,
}

impl PopulationStatisticsFamilyProjectionV3 {
    pub(crate) const fn family(self) -> InstrumentFamilyV1 {
        self.value.family
    }
    pub(crate) const fn terminal(self) -> StatisticsFamilyTerminalV3 {
        self.value.terminal
    }
    pub(crate) const fn identity(self) -> [u8; 32] {
        self.value.identity
    }
    pub(crate) const fn pre_admission_sequence(self) -> u64 {
        self.value.pre_admission_sequence
    }
    pub(crate) const fn pre_admission_record_index(self) -> u64 {
        self.value.pre_admission_record_index
    }
    pub(crate) const fn pre_admission_authority_id(self) -> [u8; 32] {
        self.value.pre_admission_authority_id
    }
    pub(crate) const fn candidate_universe_id(self) -> [u8; 32] {
        self.value.candidate_universe_id
    }
    pub(crate) const fn candidate_completion_digest(self) -> [u8; 32] {
        self.value.candidate_completion_digest
    }
    pub(crate) const fn observation_authority_id(self) -> [u8; 32] {
        self.value.observation_authority_id
    }
    pub(crate) const fn observation_identity(self) -> [u8; 32] {
        self.value.observation_identity
    }
    pub(crate) const fn observation_policy_digest(self) -> [u8; 32] {
        self.value.observation_policy_digest
    }
    pub(crate) const fn observation_data_digest(self) -> [u8; 32] {
        self.value.observation_data_digest
    }
    pub(crate) const fn observation_completion_digest(self) -> [u8; 32] {
        self.value.observation_completion_digest
    }
    pub(crate) const fn accepted_session_digest(self) -> [u8; 32] {
        self.value.accepted_session_digest
    }
    pub(crate) const fn layout_digest(self) -> [u8; 32] {
        self.value.layout_digest
    }
    pub(crate) const fn candidate_count(self) -> u64 {
        self.value.candidate_count
    }
    pub(crate) const fn period_count(self) -> u64 {
        self.value.period_count
    }
    pub(crate) const fn split_count(self) -> u64 {
        self.value.split_count
    }
    pub(crate) const fn segment_count(self) -> u32 {
        self.value.segment_count
    }
    pub(crate) const fn ordered_candidate_digest(self) -> [u8; 32] {
        self.value.ordered_candidate_digest
    }
    pub(crate) const fn ordered_period_digest(self) -> [u8; 32] {
        self.value.ordered_period_digest
    }
    pub(crate) const fn ordered_split_digest(self) -> [u8; 32] {
        self.value.ordered_split_digest
    }
}

/// One real Statistics V3 Candidate projected for Admission V4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationStatisticsAdmissionCandidateV4 {
    candidate: PopulationStatisticsCandidateV3,
    draft: Option<AdmissionStatisticsDraftV3>,
}

impl PopulationStatisticsAdmissionCandidateV4 {
    pub(crate) const fn sequence(self) -> u64 {
        self.candidate.global_sequence
    }
    pub(crate) const fn family(self) -> InstrumentFamilyV1 {
        self.candidate.family
    }
    pub(crate) const fn family_sequence(self) -> u64 {
        self.candidate.family_sequence
    }
    pub(crate) const fn candidate_semantic_digest(self) -> [u8; 32] {
        self.candidate.candidate_semantic_digest
    }
    pub(crate) const fn pre_admission_authority_id(self) -> [u8; 32] {
        self.candidate.pre_admission_authority_id
    }
    pub(crate) const fn candidate_ordered_period_digest(self) -> [u8; 32] {
        self.candidate.ordered_period_digest
    }
    pub(crate) const fn candidate_ordered_split_digest(self) -> [u8; 32] {
        self.candidate.ordered_split_digest
    }
    pub(crate) const fn draft(self) -> Option<AdmissionStatisticsDraftV3> {
        self.draft
    }
}

fn familywise_romano_wolf_probability(
    candidates: &[PopulationStatisticsCandidateV3],
    family: FamilyEvidenceV3,
) -> Result<ExactFractionV3, PopulationStatisticsV3Refusal> {
    let mut found = None;
    for candidate in candidates {
        if candidate.family == family.family
            && candidate.romano_wolf_rank == 0
            && found.replace(candidate.romano_wolf_adjusted).is_some()
        {
            return Err("Statistics V3 family has more than one Romano-Wolf rank zero".to_owned());
        }
    }
    found.ok_or_else(|| "Statistics V3 evaluated family lacks Romano-Wolf rank zero".to_owned())
}

fn admission_probability_v4(
    value: ExactFractionV3,
    name: &str,
) -> Result<AdmissionExactProbabilityV2, PopulationStatisticsV3Refusal> {
    AdmissionExactProbabilityV2::new(value.numerator, value.denominator)
        .map_err(|why| format!("Statistics V3 Admission V4 {name} probability refused: {why:?}"))
}

#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::float_arithmetic,
    reason = "canonical comparison-only floor-ppm projection of the retained Wilson value"
)]
fn admission_wilson_ppm_v4(bits: u64) -> Result<u64, PopulationStatisticsV3Refusal> {
    const PPM: u64 = 1_000_000;
    let value = f64::from_bits(bits);
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err("Statistics V3 Admission V4 Wilson value is outside [0,1]".to_owned());
    }
    Ok((value * PPM as f64).floor() as u64)
}

fn admission_v4_cscv_policy_digest(family: FamilyEvidenceV3) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(ADMISSION_V4_CSCV_POLICY_DOMAIN);
    hasher.update(&family.segment_count.to_le_bytes());
    hasher.update(&family.split_count.to_le_bytes());
    hasher.update(&family.layout_digest);
    hasher.finalize()
}

fn admission_v4_draft(
    manifest: ManifestV3,
    family: FamilyEvidenceV3,
    candidate: PopulationStatisticsCandidateV3,
    familywise: ExactFractionV3,
    completion_digest: [u8; 32],
) -> Result<AdmissionStatisticsDraftV3, PopulationStatisticsV3Refusal> {
    let pbo = family
        .pbo
        .ok_or_else(|| "Statistics V3 evaluated family lacks genuine PBO".to_owned())?;
    let white = family
        .white
        .ok_or_else(|| "Statistics V3 evaluated family lacks White evidence".to_owned())?;
    let spa = family
        .spa
        .ok_or_else(|| "Statistics V3 evaluated family lacks SPA evidence".to_owned())?;
    Ok(AdmissionStatisticsDraftV3 {
        candidate_semantic_id: candidate.candidate_semantic_digest,
        statistics_audit_id: manifest.authority_id,
        statistics_completion_digest: completion_digest,
        observation_statistics_link_id: family.identity,
        cscv_policy_digest: admission_v4_cscv_policy_digest(family),
        cscv_split_family_digest: family.ordered_split_digest,
        white_family_digest: white.family_digest,
        spa_family_digest: spa.family_digest,
        romano_wolf_family_digest: family
            .romano_wolf_family_digest
            .ok_or_else(|| "Statistics V3 evaluated family lacks Romano-Wolf family".to_owned())?,
        trades: candidate.trades,
        wins: candidate.wins,
        wilson_lower_bits: candidate.wilson_lower_bits,
        wilson_win_rate_ppm: admission_wilson_ppm_v4(candidate.wilson_lower_bits)?,
        cscv_split_count: family.split_count,
        pbo_contributing_splits: pbo.contributing_splits,
        pbo_unrankable_splits: pbo.unrankable_splits,
        pbo_probability: admission_probability_v4(pbo.probability, "CSCV/PBO")?,
        white_probability: admission_probability_v4(white.probability, "White")?,
        spa_probability: admission_probability_v4(spa.probability, "SPA")?,
        familywise_romano_wolf_probability: admission_probability_v4(
            familywise,
            "familywise Romano-Wolf",
        )?,
        candidate_romano_wolf_probability: admission_probability_v4(
            candidate.romano_wolf_adjusted,
            "candidate Romano-Wolf",
        )?,
        bootstrap_draws: manifest.draws,
        bootstrap_strategies: family.candidate_count,
        bootstrap_periods: family.period_count,
    })
}

impl ProducedPopulationStatisticsV3 {
    /// Persists all evidence before Completion and then freshly reopens it.
    pub(crate) fn append_and_reopen(
        &self,
        root: impl AsRef<Path>,
        bounds: PopulationStatisticsV3Bounds,
    ) -> Result<PopulationStatisticsV3Commit, PopulationStatisticsV3Refusal> {
        append_population_statistics_v3(root, bounds, self)
    }

    /// Authenticates this exact opaque production against its fresh commit.
    pub(crate) fn authenticated_admission_source(
        &self,
        commit: PopulationStatisticsV3Commit,
    ) -> Result<PopulationStatisticsAdmissionSourceV3, PopulationStatisticsV3Refusal> {
        let actual = commit.audit();
        let mut expected_manifest = self.manifest;
        expected_manifest.sequence = actual.sequence();
        if actual.manifest != expected_manifest
            || actual.nifty_terminal != self.nifty.terminal
            || actual.banknifty_terminal != self.banknifty.terminal
            || actual.completion_digest == [0; 32]
        {
            return Err("Statistics V3 Admission received a foreign commit".to_owned());
        }
        Ok(PopulationStatisticsAdmissionSourceV3 {
            audit: actual,
            nifty: self.nifty,
            banknifty: self.banknifty,
            candidates: self.candidates.clone(),
        })
    }
}

#[derive(Clone, Copy)]
struct CommonSourceV3 {
    rung_seconds: u32,
    horizon_bars: u32,
    requested_span: RequestedSpanIdentityV1,
    feed_digest: [u8; 32],
    source_commit_digest: [u8; 32],
    calendar_policy_digest: [u8; 32],
    daily_reference_policy_digest: [u8; 32],
}

impl CommonSourceV3 {
    fn from_v1(value: PreAdmissionDataV1) -> Self {
        Self {
            rung_seconds: value.rung_seconds(),
            horizon_bars: value.horizon_bars(),
            requested_span: value.requested_span(),
            feed_digest: value.feed_digest(),
            source_commit_digest: value.source_commit_digest(),
            calendar_policy_digest: value.calendar_policy_digest(),
            daily_reference_policy_digest: value.daily_reference_policy_digest(),
        }
    }

    fn from_extinct(value: ObservationNaturalExtinctionSourceV2) -> Self {
        let source = value.source();
        Self {
            rung_seconds: source.rung_seconds(),
            horizon_bars: source.horizon_bars(),
            requested_span: source.requested_span(),
            feed_digest: source.feed_digest(),
            source_commit_digest: source.source_commit_digest(),
            calendar_policy_digest: source.calendar_policy_digest(),
            daily_reference_policy_digest: source.daily_reference_policy_digest(),
        }
    }

    fn require_equal(self, other: Self) -> Result<Self, PopulationStatisticsV3Refusal> {
        if self.rung_seconds != other.rung_seconds
            || self.horizon_bars != other.horizon_bars
            || self.requested_span != other.requested_span
            || self.feed_digest != other.feed_digest
            || self.source_commit_digest != other.source_commit_digest
            || self.calendar_policy_digest != other.calendar_policy_digest
            || self.daily_reference_policy_digest != other.daily_reference_policy_digest
        {
            return Err(
                "Statistics V3 NIFTY/BANKNIFTY rung, horizon, span or source identities differ"
                    .to_owned(),
            );
        }
        Ok(self)
    }
}

/// Builds one mixed evaluated/naturally-extinct Statistics V3 authority.
///
/// The evaluated family, exact matching Pre-Admission audit, opaque extinct
/// production and its exact receipt-last commit are the only inputs.  Family
/// order is derived; no terminal, row, digest or statistic is caller-authored.
pub(crate) fn produce_mixed_population_statistics_v3(
    evaluated: &CandidateFamilyObservationsV1,
    evaluated_pre_admission: PreAdmissionDataReopenAuditV1,
    extinct: &ProducedObservationAuthorityV2,
    extinct_commit: &ObservationAuthorityCommitV2,
    procedure: PopulationStatisticsProcedureV2,
) -> Result<ProducedPopulationStatisticsV3, PopulationStatisticsV3Refusal> {
    let projection =
        prepare_single_family_statistics_v3(evaluated, evaluated_pre_admission, procedure)?;
    let extinct_source = extinct.authenticated_statistics_v3_source(extinct_commit)?;
    if evaluated.family() == extinct_source.audit().family() {
        return Err("Statistics V3 mixed authority repeats one index family".to_owned());
    }
    let evaluated_common = CommonSourceV3::from_v1(evaluated_pre_admission.value());
    let common = evaluated_common.require_equal(CommonSourceV3::from_extinct(extinct_source))?;
    let (evaluated_family, candidates) =
        evaluated_family(evaluated, evaluated_pre_admission, &projection)?;
    let extinct_family = naturally_extinct_family(extinct_source)?;
    let (nifty, banknifty) = match evaluated.family() {
        InstrumentFamilyV1::Nifty => (evaluated_family, extinct_family),
        InstrumentFamilyV1::BankNifty => (extinct_family, evaluated_family),
    };
    build_produced(common, nifty, banknifty, candidates, Some(procedure))
}

/// Builds one canonically ordered two-evaluated Statistics V3 authority.
///
/// Both families are opaque Observation productions paired with their exact
/// freshly reopened Pre-Admission V1 audits.  The constructor accepts no
/// family label, terminal, count, digest or statistical row; it derives both
/// single-family projections under the same procedure, requires NIFTY first
/// and BANKNIFTY second, and preserves the existing V3 byte grammar.
pub(crate) fn produce_evaluated_population_statistics_v3(
    nifty: &CandidateFamilyObservationsV1,
    nifty_pre_admission: PreAdmissionDataReopenAuditV1,
    banknifty: &CandidateFamilyObservationsV1,
    banknifty_pre_admission: PreAdmissionDataReopenAuditV1,
    procedure: PopulationStatisticsProcedureV2,
) -> Result<ProducedPopulationStatisticsV3, PopulationStatisticsV3Refusal> {
    if nifty.family() != InstrumentFamilyV1::Nifty
        || banknifty.family() != InstrumentFamilyV1::BankNifty
    {
        return Err(
            "Statistics V3 evaluated pair requires canonical NIFTY then BANKNIFTY authorities"
                .to_owned(),
        );
    }
    let nifty_projection =
        prepare_single_family_statistics_v3(nifty, nifty_pre_admission, procedure)?;
    let banknifty_projection =
        prepare_single_family_statistics_v3(banknifty, banknifty_pre_admission, procedure)?;
    let common = CommonSourceV3::from_v1(nifty_pre_admission.value())
        .require_equal(CommonSourceV3::from_v1(banknifty_pre_admission.value()))?;
    let (nifty_family, mut candidates) =
        evaluated_family(nifty, nifty_pre_admission, &nifty_projection)?;
    let (banknifty_family, banknifty_candidates) =
        evaluated_family(banknifty, banknifty_pre_admission, &banknifty_projection)?;
    if nifty_family.terminal != StatisticsFamilyTerminalV3::Evaluated
        || banknifty_family.terminal != StatisticsFamilyTerminalV3::Evaluated
    {
        return Err(
            "Statistics V3 evaluated pair requires at least two authentic Candidates per family"
                .to_owned(),
        );
    }
    candidates.extend(banknifty_candidates);
    build_produced(
        common,
        nifty_family,
        banknifty_family,
        candidates,
        Some(procedure),
    )
}

/// Builds one truthful all-extinct Statistics V3 authority from two opaque,
/// exact Observation V2 productions.  No bootstrap procedure or zero-valued
/// numeric placeholder is accepted.
pub(crate) fn produce_all_extinct_population_statistics_v3(
    first: &ProducedObservationAuthorityV2,
    first_commit: &ObservationAuthorityCommitV2,
    second: &ProducedObservationAuthorityV2,
    second_commit: &ObservationAuthorityCommitV2,
) -> Result<ProducedPopulationStatisticsV3, PopulationStatisticsV3Refusal> {
    let first = first.authenticated_statistics_v3_source(first_commit)?;
    let second = second.authenticated_statistics_v3_source(second_commit)?;
    if first.audit().family() == second.audit().family() {
        return Err("Statistics V3 all-extinct authority repeats one index family".to_owned());
    }
    let common =
        CommonSourceV3::from_extinct(first).require_equal(CommonSourceV3::from_extinct(second))?;
    let first = naturally_extinct_family(first)?;
    let second = naturally_extinct_family(second)?;
    let (nifty, banknifty) = if first.family == InstrumentFamilyV1::Nifty {
        (first, second)
    } else {
        (second, first)
    };
    build_produced(common, nifty, banknifty, Vec::new(), None)
}

fn evaluated_family(
    observations: &CandidateFamilyObservationsV1,
    pre_admission: PreAdmissionDataReopenAuditV1,
    projection: &SingleFamilyStatisticsV3Projection,
) -> Result<(FamilyEvidenceV3, Vec<PopulationStatisticsCandidateV3>), PopulationStatisticsV3Refusal>
{
    let source = observations.source();
    let pre = pre_admission.value();
    if source.family() != pre.family()
        || source.universe_id() != pre.candidate_universe_id()
        || source.content_digest() != pre.candidate_completion_digest()
        || source.row_count() != pre.candidate_row_count()
        || source.row_count()
            != u64::try_from(projection.candidates().len())
                .map_err(|_| "Statistics V3 candidate count does not fit u64".to_owned())?
    {
        return Err("Statistics V3 evaluated Observation/Pre-Admission binding differs".to_owned());
    }
    let terminal = if source.row_count() == 1 {
        StatisticsFamilyTerminalV3::InsufficientForCscv
    } else if source.row_count() >= 2 {
        StatisticsFamilyTerminalV3::Evaluated
    } else {
        return Err("Statistics V3 evaluated family is empty".to_owned());
    };
    if (terminal == StatisticsFamilyTerminalV3::Evaluated) != projection.pbo().is_some() {
        return Err(
            "Statistics V3 computed PBO presence disagrees with candidate count".to_owned(),
        );
    }
    let mut evidence = FamilyEvidenceV3 {
        family: observations.family(),
        terminal,
        pre_admission_sequence: pre.sequence(),
        pre_admission_record_index: pre_admission.data_record_index(),
        pre_admission_authority_id: pre.authority_id(),
        candidate_universe_id: pre.candidate_universe_id(),
        candidate_completion_digest: pre.candidate_completion_digest(),
        observation_authority_id: [0; 32],
        observation_identity: observations.identity(),
        observation_policy_digest: observations.policy().digest(),
        observation_data_digest: [0; 32],
        observation_completion_digest: [0; 32],
        accepted_session_digest: accepted_session_digest(observations.accepted_ist_sessions()),
        layout_digest: projection.layout_digest(),
        candidate_count: source.row_count(),
        period_count: projection.period_count(),
        split_count: projection.split_count(),
        segment_count: projection.segment_count(),
        white: Some(FamilyTestV3::from_v2(projection.white())?),
        spa: Some(FamilyTestV3::from_v2(projection.spa())?),
        romano_wolf_family_digest: Some(projection.romano_wolf_family_digest()),
        pbo: projection.pbo().map(PboV3::from_v2).transpose()?,
        ordered_candidate_digest: projection.ordered_candidate_digest(),
        ordered_period_digest: projection.ordered_period_digest(),
        ordered_split_digest: projection.ordered_split_digest(),
        identity: [0; 32],
    };
    evidence.identity = family_identity(&evidence);
    evidence.validate()?;

    let mut candidates = Vec::new();
    candidates
        .try_reserve_exact(projection.candidates().len())
        .map_err(|why| format!("cannot reserve Statistics V3 candidate rows: {why}"))?;
    for projected in projection.candidates() {
        candidates.push(candidate_from_projection(
            observations.family(),
            pre.authority_id(),
            *projected,
        )?);
    }
    Ok((evidence, candidates))
}

fn naturally_extinct_family(
    source: ObservationNaturalExtinctionSourceV2,
) -> Result<FamilyEvidenceV3, PopulationStatisticsV3Refusal> {
    let audit = source.audit();
    let pre = source.source();
    if pre.candidate_row_count() != 0 || audit.observation_row_count() != 0 {
        return Err("Statistics V3 extinct source contains rows".to_owned());
    }
    let mut evidence = FamilyEvidenceV3 {
        family: audit.family(),
        terminal: StatisticsFamilyTerminalV3::NaturallyExtinct,
        pre_admission_sequence: pre.sequence(),
        pre_admission_record_index: audit.pre_admission_record_index(),
        pre_admission_authority_id: audit.pre_admission_authority_id(),
        candidate_universe_id: audit.candidate_universe_id(),
        candidate_completion_digest: audit.candidate_completion_digest(),
        observation_authority_id: audit.authority_id(),
        observation_identity: audit.authority_id(),
        observation_policy_digest: audit.policy_digest(),
        observation_data_digest: audit.data_record_digest(),
        observation_completion_digest: audit.completion_digest(),
        accepted_session_digest: [0; 32],
        layout_digest: [0; 32],
        candidate_count: 0,
        period_count: 0,
        split_count: 0,
        segment_count: 0,
        white: None,
        spa: None,
        romano_wolf_family_digest: None,
        pbo: None,
        ordered_candidate_digest: [0; 32],
        ordered_period_digest: [0; 32],
        ordered_split_digest: [0; 32],
        identity: [0; 32],
    };
    evidence.identity = family_identity(&evidence);
    evidence.validate()?;
    Ok(evidence)
}

fn candidate_from_projection(
    family: InstrumentFamilyV1,
    pre_admission_authority_id: [u8; 32],
    projected: SingleFamilyCandidateStatisticsV3,
) -> Result<PopulationStatisticsCandidateV3, PopulationStatisticsV3Refusal> {
    let initial = projected.romano_wolf_initial();
    let adjusted = projected.romano_wolf_adjusted();
    Ok(PopulationStatisticsCandidateV3 {
        authority_id: [0; 32],
        global_sequence: 0,
        family,
        family_sequence: projected.family_sequence(),
        candidate_semantic_digest: projected.candidate_semantic_digest(),
        pre_admission_authority_id,
        trades: projected.trades(),
        wins: projected.wins(),
        wilson_lower_bits: projected.wilson_lower_bits(),
        romano_wolf_statistic_bits: projected.romano_wolf_statistic_bits(),
        romano_wolf_rank: projected.romano_wolf_rank(),
        romano_wolf_strict_exceedances: projected.romano_wolf_strict_exceedances(),
        romano_wolf_initial: ExactFractionV3::new(initial.numerator(), initial.denominator())?,
        romano_wolf_adjusted: ExactFractionV3::new(adjusted.numerator(), adjusted.denominator())?,
        ordered_period_digest: projected.ordered_period_digest(),
        ordered_split_digest: projected.ordered_split_digest(),
    })
}

fn build_produced(
    common: CommonSourceV3,
    nifty: FamilyEvidenceV3,
    banknifty: FamilyEvidenceV3,
    mut candidates: Vec<PopulationStatisticsCandidateV3>,
    procedure: Option<PopulationStatisticsProcedureV2>,
) -> Result<ProducedPopulationStatisticsV3, PopulationStatisticsV3Refusal> {
    if nifty.family != InstrumentFamilyV1::Nifty
        || banknifty.family != InstrumentFamilyV1::BankNifty
    {
        return Err("Statistics V3 families are not NIFTY then BANKNIFTY".to_owned());
    }
    nifty.validate()?;
    banknifty.validate()?;
    let expected_candidates = nifty
        .candidate_count
        .checked_add(banknifty.candidate_count)
        .ok_or_else(|| "Statistics V3 candidate count overflowed".to_owned())?;
    if u64::try_from(candidates.len())
        .map_err(|_| "Statistics V3 candidate length does not fit u64".to_owned())?
        != expected_candidates
    {
        return Err("Statistics V3 candidate vector does not match family counts".to_owned());
    }
    candidates.sort_by_key(|candidate| (family_code(candidate.family), candidate.family_sequence));
    let mut global = 0_u64;
    for candidate in &mut candidates {
        candidate.global_sequence = global;
        global = global
            .checked_add(1)
            .ok_or_else(|| "Statistics V3 global candidate sequence overflowed".to_owned())?;
    }
    let ordered_candidate_digest = if candidates.is_empty() {
        [0; 32]
    } else {
        ordered_candidate_digest(&candidates)
    };
    let (draws, seed, block_length) = procedure.map_or((0, 0, 0), |value| {
        (value.draws(), value.seed(), value.block_length())
    });
    let mut manifest = ManifestV3 {
        sequence: 0,
        authority_id: [0; 32],
        draws,
        seed,
        block_length,
        rung_seconds: common.rung_seconds,
        horizon_bars: common.horizon_bars,
        requested_span: common.requested_span,
        feed_digest: common.feed_digest,
        source_commit_digest: common.source_commit_digest,
        calendar_policy_digest: common.calendar_policy_digest,
        daily_reference_policy_digest: common.daily_reference_policy_digest,
        policy_digest: statistics_v3_policy_digest(),
        nifty_family_identity: nifty.identity,
        banknifty_family_identity: banknifty.identity,
        ordered_candidate_digest,
        candidate_count: expected_candidates,
    };
    manifest.authority_id = manifest_identity(&manifest);
    for candidate in &mut candidates {
        candidate.authority_id = manifest.authority_id;
        let family = if candidate.family == InstrumentFamilyV1::Nifty {
            &nifty
        } else {
            &banknifty
        };
        candidate.validate(family, candidate.global_sequence)?;
        validate_candidate_procedure(manifest, *candidate)?;
    }
    validate_family_procedure(manifest, nifty)?;
    validate_family_procedure(manifest, banknifty)?;
    manifest.validate()?;
    Ok(ProducedPopulationStatisticsV3 {
        manifest,
        nifty,
        banknifty,
        candidates,
    })
}

fn statistics_v3_policy_digest() -> [u8; 32] {
    digest_domain(
        POLICY_DOMAIN,
        b"nifty-first;typed-evaluated-insufficient-cscv-natural-extinction;no-extinct-numerics;receipt-last;exact-reuse;bounded-fresh-reopen",
    )
}

fn accepted_session_digest(sessions: &[i64]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(ACCEPTED_SESSION_DOMAIN);
    for day in sessions {
        hasher.update(&day.to_le_bytes());
    }
    hasher.finalize()
}

fn family_identity(value: &FamilyEvidenceV3) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(FAMILY_DOMAIN);
    hasher.update(&family_code(value.family).to_le_bytes());
    hasher.update(&value.terminal.code().to_le_bytes());
    hasher.update(&value.pre_admission_sequence.to_le_bytes());
    hasher.update(&value.pre_admission_record_index.to_le_bytes());
    for digest in [
        value.pre_admission_authority_id,
        value.candidate_universe_id,
        value.candidate_completion_digest,
        value.observation_authority_id,
        value.observation_identity,
        value.observation_policy_digest,
        value.observation_data_digest,
        value.observation_completion_digest,
        value.accepted_session_digest,
        value.layout_digest,
    ] {
        hasher.update(&digest);
    }
    hasher.update(&value.candidate_count.to_le_bytes());
    hasher.update(&value.period_count.to_le_bytes());
    hasher.update(&value.split_count.to_le_bytes());
    hasher.update(&value.segment_count.to_le_bytes());
    hash_optional_family_test(&mut hasher, value.white);
    hash_optional_family_test(&mut hasher, value.spa);
    hash_optional_digest(&mut hasher, value.romano_wolf_family_digest);
    hash_optional_pbo(&mut hasher, value.pbo);
    hasher.update(&value.ordered_candidate_digest);
    hasher.update(&value.ordered_period_digest);
    hasher.update(&value.ordered_split_digest);
    hasher.finalize()
}

fn hash_optional_family_test(hasher: &mut Hasher, value: Option<FamilyTestV3>) {
    hasher.update(&[u8::from(value.is_some())]);
    if let Some(value) = value {
        hasher.update(&value.statistic_bits.to_le_bytes());
        hasher.update(&value.probability_bits.to_le_bytes());
        hasher.update(&value.probability.numerator.to_le_bytes());
        hasher.update(&value.probability.denominator.to_le_bytes());
        hasher.update(&value.family_digest);
    }
}

fn hash_optional_digest(hasher: &mut Hasher, value: Option<[u8; 32]>) {
    hasher.update(&[u8::from(value.is_some())]);
    if let Some(value) = value {
        hasher.update(&value);
    }
}

fn hash_optional_pbo(hasher: &mut Hasher, value: Option<PboV3>) {
    hasher.update(&[u8::from(value.is_some())]);
    if let Some(value) = value {
        hasher.update(&value.contributing_splits.to_le_bytes());
        hasher.update(&value.bottom_half_splits.to_le_bytes());
        hasher.update(&value.unrankable_splits.to_le_bytes());
        hasher.update(&value.probability_bits.to_le_bytes());
        hasher.update(&value.probability.numerator.to_le_bytes());
        hasher.update(&value.probability.denominator.to_le_bytes());
    }
}

fn manifest_identity(value: &ManifestV3) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(AUTHORITY_DOMAIN);
    hasher.update(&value.draws.to_le_bytes());
    hasher.update(&value.seed.to_le_bytes());
    hasher.update(&value.block_length.to_le_bytes());
    hasher.update(&value.rung_seconds.to_le_bytes());
    hasher.update(&value.horizon_bars.to_le_bytes());
    hasher.update(&value.requested_span.from_year().to_le_bytes());
    hasher.update(&value.requested_span.from_month().to_le_bytes());
    hasher.update(&value.requested_span.to_year().to_le_bytes());
    hasher.update(&value.requested_span.to_month().to_le_bytes());
    for digest in [
        value.feed_digest,
        value.source_commit_digest,
        value.calendar_policy_digest,
        value.daily_reference_policy_digest,
        value.policy_digest,
        value.nifty_family_identity,
        value.banknifty_family_identity,
        value.ordered_candidate_digest,
    ] {
        hasher.update(&digest);
    }
    hasher.update(&value.candidate_count.to_le_bytes());
    hasher.finalize()
}

fn ordered_candidate_digest(candidates: &[PopulationStatisticsCandidateV3]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(CANDIDATE_ORDER_DOMAIN);
    for candidate in candidates {
        hasher.update(&candidate.global_sequence.to_le_bytes());
        hasher.update(&family_code(candidate.family).to_le_bytes());
        hasher.update(&candidate.family_sequence.to_le_bytes());
        hasher.update(&candidate.candidate_semantic_digest);
        hasher.update(&candidate.pre_admission_authority_id);
        hasher.update(&candidate.trades.to_le_bytes());
        hasher.update(&candidate.wins.to_le_bytes());
        hasher.update(&candidate.wilson_lower_bits.to_le_bytes());
        hasher.update(&candidate.romano_wolf_statistic_bits.to_le_bytes());
        hasher.update(&candidate.romano_wolf_rank.to_le_bytes());
        hasher.update(&candidate.romano_wolf_strict_exceedances.to_le_bytes());
        hasher.update(&candidate.romano_wolf_initial.numerator.to_le_bytes());
        hasher.update(&candidate.romano_wolf_initial.denominator.to_le_bytes());
        hasher.update(&candidate.romano_wolf_adjusted.numerator.to_le_bytes());
        hasher.update(&candidate.romano_wolf_adjusted.denominator.to_le_bytes());
        hasher.update(&candidate.ordered_period_digest);
        hasher.update(&candidate.ordered_split_digest);
    }
    hasher.finalize()
}

fn family_code(family: InstrumentFamilyV1) -> u32 {
    match family {
        InstrumentFamilyV1::Nifty => 1,
        InstrumentFamilyV1::BankNifty => 2,
    }
}

fn decode_family(value: u32) -> Result<InstrumentFamilyV1, PopulationStatisticsV3Refusal> {
    match value {
        1 => Ok(InstrumentFamilyV1::Nifty),
        2 => Ok(InstrumentFamilyV1::BankNifty),
        _ => Err(format!("Statistics V3 family {value} is unknown")),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RecordKindV3 {
    Data,
    Family,
    Candidate,
    Completion,
}

impl RecordKindV3 {
    const fn code(self) -> u32 {
        match self {
            Self::Data => DATA_KIND,
            Self::Family => FAMILY_KIND,
            Self::Candidate => CANDIDATE_KIND,
            Self::Completion => COMPLETION_KIND,
        }
    }

    fn decode(value: u32) -> Result<Self, PopulationStatisticsV3Refusal> {
        match value {
            DATA_KIND => Ok(Self::Data),
            FAMILY_KIND => Ok(Self::Family),
            CANDIDATE_KIND => Ok(Self::Candidate),
            COMPLETION_KIND => Ok(Self::Completion),
            _ => Err(format!("Statistics V3 record kind {value} is unknown")),
        }
    }
}

#[derive(Clone, Copy)]
struct DecodedRecordV3 {
    kind: RecordKindV3,
    payload: [u8; PAYLOAD_BYTES],
}

fn base_payload(
    kind: RecordKindV3,
    physical_sequence: u64,
    authority_id: [u8; 32],
) -> Result<[u8; PAYLOAD_BYTES], PopulationStatisticsV3Refusal> {
    let mut payload = [0_u8; PAYLOAD_BYTES];
    put_u32(&mut payload, 0, RECORD_VERSION)?;
    put_u32(&mut payload, 4, kind.code())?;
    put_u64(&mut payload, 8, physical_sequence)?;
    put_bytes(&mut payload, 16, &authority_id)?;
    Ok(payload)
}

fn encode_manifest(
    manifest: ManifestV3,
    kind: RecordKindV3,
    physical_sequence: u64,
) -> Result<[u8; RECORD_BYTES], PopulationStatisticsV3Refusal> {
    if !matches!(kind, RecordKindV3::Data | RecordKindV3::Completion) {
        return Err("Statistics V3 manifest record kind is foreign".to_owned());
    }
    manifest.validate()?;
    let mut payload = base_payload(kind, physical_sequence, manifest.authority_id)?;
    put_u64(&mut payload, 48, manifest.sequence)?;
    put_u64(&mut payload, 56, manifest.draws)?;
    put_u64(&mut payload, 64, manifest.seed)?;
    put_u64(&mut payload, 72, manifest.block_length)?;
    put_u32(&mut payload, 80, manifest.rung_seconds)?;
    put_u32(&mut payload, 84, manifest.horizon_bars)?;
    put_u32(
        &mut payload,
        88,
        u32::from(manifest.requested_span.from_year()),
    )?;
    put_u32(
        &mut payload,
        92,
        u32::from(manifest.requested_span.from_month()),
    )?;
    put_u32(
        &mut payload,
        96,
        u32::from(manifest.requested_span.to_year()),
    )?;
    put_u32(
        &mut payload,
        100,
        u32::from(manifest.requested_span.to_month()),
    )?;
    put_bytes(&mut payload, 104, &manifest.feed_digest)?;
    put_bytes(&mut payload, 136, &manifest.source_commit_digest)?;
    put_bytes(&mut payload, 168, &manifest.calendar_policy_digest)?;
    put_bytes(&mut payload, 200, &manifest.daily_reference_policy_digest)?;
    put_bytes(&mut payload, 232, &manifest.policy_digest)?;
    put_bytes(&mut payload, 264, &manifest.nifty_family_identity)?;
    put_bytes(&mut payload, 296, &manifest.banknifty_family_identity)?;
    put_bytes(&mut payload, 328, &manifest.ordered_candidate_digest)?;
    put_u64(&mut payload, 360, manifest.candidate_count)?;
    Ok(seal_payload(&payload))
}

fn decode_manifest(
    payload: &[u8; PAYLOAD_BYTES],
    kind: RecordKindV3,
) -> Result<ManifestV3, PopulationStatisticsV3Refusal> {
    if !matches!(kind, RecordKindV3::Data | RecordKindV3::Completion) {
        return Err("Statistics V3 manifest record kind is foreign".to_owned());
    }
    require_zero(payload, 368, PAYLOAD_BYTES - 368, "manifest reserve")?;
    let value = ManifestV3 {
        sequence: get_u64(payload, 48)?,
        authority_id: get_32(payload, 16)?,
        draws: get_u64(payload, 56)?,
        seed: get_u64(payload, 64)?,
        block_length: get_u64(payload, 72)?,
        rung_seconds: get_u32(payload, 80)?,
        horizon_bars: get_u32(payload, 84)?,
        requested_span: RequestedSpanIdentityV1::new(
            u16::try_from(get_u32(payload, 88)?)
                .map_err(|why| format!("Statistics V3 from-year does not fit u16: {why}"))?,
            u8::try_from(get_u32(payload, 92)?)
                .map_err(|why| format!("Statistics V3 from-month does not fit u8: {why}"))?,
            u16::try_from(get_u32(payload, 96)?)
                .map_err(|why| format!("Statistics V3 to-year does not fit u16: {why}"))?,
            u8::try_from(get_u32(payload, 100)?)
                .map_err(|why| format!("Statistics V3 to-month does not fit u8: {why}"))?,
        )?,
        feed_digest: get_32(payload, 104)?,
        source_commit_digest: get_32(payload, 136)?,
        calendar_policy_digest: get_32(payload, 168)?,
        daily_reference_policy_digest: get_32(payload, 200)?,
        policy_digest: get_32(payload, 232)?,
        nifty_family_identity: get_32(payload, 264)?,
        banknifty_family_identity: get_32(payload, 296)?,
        ordered_candidate_digest: get_32(payload, 328)?,
        candidate_count: get_u64(payload, 360)?,
    };
    value.validate()?;
    Ok(value)
}

fn encode_family(
    authority_id: [u8; 32],
    family: FamilyEvidenceV3,
    physical_sequence: u64,
) -> Result<[u8; RECORD_BYTES], PopulationStatisticsV3Refusal> {
    family.validate()?;
    let mut payload = base_payload(RecordKindV3::Family, physical_sequence, authority_id)?;
    put_u32(&mut payload, 48, family_code(family.family))?;
    put_u32(&mut payload, 52, family.terminal.code())?;
    put_u64(&mut payload, 56, family.pre_admission_sequence)?;
    put_u64(&mut payload, 64, family.pre_admission_record_index)?;
    put_bytes(&mut payload, 72, &family.pre_admission_authority_id)?;
    put_bytes(&mut payload, 104, &family.candidate_universe_id)?;
    put_bytes(&mut payload, 136, &family.candidate_completion_digest)?;
    put_bytes(&mut payload, 168, &family.observation_authority_id)?;
    put_bytes(&mut payload, 200, &family.observation_identity)?;
    put_bytes(&mut payload, 232, &family.observation_policy_digest)?;
    put_bytes(&mut payload, 264, &family.observation_data_digest)?;
    put_bytes(&mut payload, 296, &family.observation_completion_digest)?;
    put_bytes(&mut payload, 328, &family.accepted_session_digest)?;
    put_bytes(&mut payload, 360, &family.layout_digest)?;
    put_u64(&mut payload, 392, family.candidate_count)?;
    put_u64(&mut payload, 400, family.period_count)?;
    put_u64(&mut payload, 408, family.split_count)?;
    put_u32(&mut payload, 416, family.segment_count)?;
    let flags = u32::from(family.white.is_some())
        | (u32::from(family.spa.is_some()) << 1)
        | (u32::from(family.romano_wolf_family_digest.is_some()) << 2)
        | (u32::from(family.pbo.is_some()) << 3);
    put_u32(&mut payload, 420, flags)?;
    encode_family_test(&mut payload, 424, family.white)?;
    encode_family_test(&mut payload, 488, family.spa)?;
    put_bytes(
        &mut payload,
        552,
        &family.romano_wolf_family_digest.unwrap_or([0; 32]),
    )?;
    encode_pbo(&mut payload, 584, family.pbo)?;
    put_bytes(&mut payload, 632, &family.ordered_candidate_digest)?;
    put_bytes(&mut payload, 664, &family.ordered_period_digest)?;
    put_bytes(&mut payload, 696, &family.ordered_split_digest)?;
    put_bytes(&mut payload, 728, &family.identity)?;
    Ok(seal_payload(&payload))
}

fn decode_family_record(
    payload: &[u8; PAYLOAD_BYTES],
) -> Result<FamilyEvidenceV3, PopulationStatisticsV3Refusal> {
    let flags = get_u32(payload, 420)?;
    if flags & !0x0f != 0 {
        return Err("Statistics V3 family presence flags are foreign".to_owned());
    }
    require_zero(payload, 420 + 4, 0, "family flag reserve")?;
    require_zero(payload, 760, PAYLOAD_BYTES - 760, "family tail reserve")?;
    let value = FamilyEvidenceV3 {
        family: decode_family(get_u32(payload, 48)?)?,
        terminal: StatisticsFamilyTerminalV3::decode(get_u32(payload, 52)?)?,
        pre_admission_sequence: get_u64(payload, 56)?,
        pre_admission_record_index: get_u64(payload, 64)?,
        pre_admission_authority_id: get_32(payload, 72)?,
        candidate_universe_id: get_32(payload, 104)?,
        candidate_completion_digest: get_32(payload, 136)?,
        observation_authority_id: get_32(payload, 168)?,
        observation_identity: get_32(payload, 200)?,
        observation_policy_digest: get_32(payload, 232)?,
        observation_data_digest: get_32(payload, 264)?,
        observation_completion_digest: get_32(payload, 296)?,
        accepted_session_digest: get_32(payload, 328)?,
        layout_digest: get_32(payload, 360)?,
        candidate_count: get_u64(payload, 392)?,
        period_count: get_u64(payload, 400)?,
        split_count: get_u64(payload, 408)?,
        segment_count: get_u32(payload, 416)?,
        white: decode_family_test(payload, 424, flags & 1 != 0)?,
        spa: decode_family_test(payload, 488, flags & 2 != 0)?,
        romano_wolf_family_digest: (flags & 4 != 0).then(|| get_32(payload, 552)).transpose()?,
        pbo: decode_pbo(payload, 584, flags & 8 != 0)?,
        ordered_candidate_digest: get_32(payload, 632)?,
        ordered_period_digest: get_32(payload, 664)?,
        ordered_split_digest: get_32(payload, 696)?,
        identity: get_32(payload, 728)?,
    };
    if flags & 4 == 0 && get_32(payload, 552)? != [0; 32] {
        return Err("Statistics V3 absent Romano-Wolf carries bytes".to_owned());
    }
    value.validate()?;
    Ok(value)
}

fn encode_family_test(
    payload: &mut [u8; PAYLOAD_BYTES],
    offset: usize,
    value: Option<FamilyTestV3>,
) -> Result<(), PopulationStatisticsV3Refusal> {
    if let Some(value) = value {
        put_u64(payload, offset, value.statistic_bits)?;
        put_u64(payload, offset + 8, value.probability_bits)?;
        put_u64(payload, offset + 16, value.probability.numerator)?;
        put_u64(payload, offset + 24, value.probability.denominator)?;
        put_bytes(payload, offset + 32, &value.family_digest)?;
    }
    Ok(())
}

fn decode_family_test(
    payload: &[u8; PAYLOAD_BYTES],
    offset: usize,
    present: bool,
) -> Result<Option<FamilyTestV3>, PopulationStatisticsV3Refusal> {
    if !present {
        require_zero(payload, offset, 64, "absent family test")?;
        return Ok(None);
    }
    let value = FamilyTestV3 {
        statistic_bits: get_u64(payload, offset)?,
        probability_bits: get_u64(payload, offset + 8)?,
        probability: ExactFractionV3::new(
            get_u64(payload, offset + 16)?,
            get_u64(payload, offset + 24)?,
        )?,
        family_digest: get_32(payload, offset + 32)?,
    };
    value.validate("family test")?;
    Ok(Some(value))
}

fn encode_pbo(
    payload: &mut [u8; PAYLOAD_BYTES],
    offset: usize,
    value: Option<PboV3>,
) -> Result<(), PopulationStatisticsV3Refusal> {
    if let Some(value) = value {
        put_u64(payload, offset, value.contributing_splits)?;
        put_u64(payload, offset + 8, value.bottom_half_splits)?;
        put_u64(payload, offset + 16, value.unrankable_splits)?;
        put_u64(payload, offset + 24, value.probability_bits)?;
        put_u64(payload, offset + 32, value.probability.numerator)?;
        put_u64(payload, offset + 40, value.probability.denominator)?;
    }
    Ok(())
}

fn decode_pbo(
    payload: &[u8; PAYLOAD_BYTES],
    offset: usize,
    present: bool,
) -> Result<Option<PboV3>, PopulationStatisticsV3Refusal> {
    if !present {
        require_zero(payload, offset, 48, "absent PBO")?;
        return Ok(None);
    }
    Ok(Some(PboV3 {
        contributing_splits: get_u64(payload, offset)?,
        bottom_half_splits: get_u64(payload, offset + 8)?,
        unrankable_splits: get_u64(payload, offset + 16)?,
        probability_bits: get_u64(payload, offset + 24)?,
        probability: ExactFractionV3::new(
            get_u64(payload, offset + 32)?,
            get_u64(payload, offset + 40)?,
        )?,
    }))
}

fn encode_candidate(
    candidate: PopulationStatisticsCandidateV3,
    physical_sequence: u64,
) -> Result<[u8; RECORD_BYTES], PopulationStatisticsV3Refusal> {
    let mut payload = base_payload(
        RecordKindV3::Candidate,
        physical_sequence,
        candidate.authority_id,
    )?;
    put_u64(&mut payload, 48, candidate.global_sequence)?;
    put_u32(&mut payload, 56, family_code(candidate.family))?;
    put_u64(&mut payload, 64, candidate.family_sequence)?;
    put_bytes(&mut payload, 72, &candidate.candidate_semantic_digest)?;
    put_bytes(&mut payload, 104, &candidate.pre_admission_authority_id)?;
    put_u64(&mut payload, 136, candidate.trades)?;
    put_u64(&mut payload, 144, candidate.wins)?;
    put_u64(&mut payload, 152, candidate.wilson_lower_bits)?;
    put_u64(&mut payload, 160, candidate.romano_wolf_statistic_bits)?;
    put_u64(&mut payload, 168, candidate.romano_wolf_rank)?;
    put_u64(&mut payload, 176, candidate.romano_wolf_strict_exceedances)?;
    put_u64(&mut payload, 184, candidate.romano_wolf_initial.numerator)?;
    put_u64(&mut payload, 192, candidate.romano_wolf_initial.denominator)?;
    put_u64(&mut payload, 200, candidate.romano_wolf_adjusted.numerator)?;
    put_u64(
        &mut payload,
        208,
        candidate.romano_wolf_adjusted.denominator,
    )?;
    put_bytes(&mut payload, 216, &candidate.ordered_period_digest)?;
    put_bytes(&mut payload, 248, &candidate.ordered_split_digest)?;
    Ok(seal_payload(&payload))
}

fn decode_candidate_record(
    payload: &[u8; PAYLOAD_BYTES],
) -> Result<PopulationStatisticsCandidateV3, PopulationStatisticsV3Refusal> {
    require_zero(payload, 60, 4, "candidate family reserve")?;
    require_zero(payload, 280, PAYLOAD_BYTES - 280, "candidate tail reserve")?;
    Ok(PopulationStatisticsCandidateV3 {
        authority_id: get_32(payload, 16)?,
        global_sequence: get_u64(payload, 48)?,
        family: decode_family(get_u32(payload, 56)?)?,
        family_sequence: get_u64(payload, 64)?,
        candidate_semantic_digest: get_32(payload, 72)?,
        pre_admission_authority_id: get_32(payload, 104)?,
        trades: get_u64(payload, 136)?,
        wins: get_u64(payload, 144)?,
        wilson_lower_bits: get_u64(payload, 152)?,
        romano_wolf_statistic_bits: get_u64(payload, 160)?,
        romano_wolf_rank: get_u64(payload, 168)?,
        romano_wolf_strict_exceedances: get_u64(payload, 176)?,
        romano_wolf_initial: ExactFractionV3::new(get_u64(payload, 184)?, get_u64(payload, 192)?)?,
        romano_wolf_adjusted: ExactFractionV3::new(get_u64(payload, 200)?, get_u64(payload, 208)?)?,
        ordered_period_digest: get_32(payload, 216)?,
        ordered_split_digest: get_32(payload, 248)?,
    })
}

fn seal_payload(payload: &[u8; PAYLOAD_BYTES]) -> [u8; RECORD_BYTES] {
    let mut raw = [0_u8; RECORD_BYTES];
    raw[..PAYLOAD_BYTES].copy_from_slice(payload);
    raw[PAYLOAD_BYTES..].copy_from_slice(&digest_domain(RECORD_DOMAIN, payload));
    raw
}

fn decode_record(
    raw: &[u8; RECORD_BYTES],
    expected_physical: u64,
) -> Result<DecodedRecordV3, PopulationStatisticsV3Refusal> {
    let payload: &[u8; PAYLOAD_BYTES] = raw[..PAYLOAD_BYTES]
        .try_into()
        .map_err(|_| "Statistics V3 payload width differs".to_owned())?;
    if digest_domain(RECORD_DOMAIN, payload).as_slice() != &raw[PAYLOAD_BYTES..] {
        return Err("Statistics V3 record seal differs".to_owned());
    }
    if get_u32(payload, 0)? != RECORD_VERSION || get_u64(payload, 8)? != expected_physical {
        return Err("Statistics V3 record version or physical sequence differs".to_owned());
    }
    Ok(DecodedRecordV3 {
        kind: RecordKindV3::decode(get_u32(payload, 4)?)?,
        payload: *payload,
    })
}

impl ProducedPopulationStatisticsV3 {
    fn records(
        &self,
        logical_sequence: u64,
        first_physical: u64,
    ) -> Result<Vec<[u8; RECORD_BYTES]>, PopulationStatisticsV3Refusal> {
        let mut manifest = self.manifest;
        manifest.sequence = logical_sequence;
        let count = usize_of(manifest.block_records()?, "Statistics V3 block records")?;
        let mut records = Vec::new();
        records
            .try_reserve_exact(count)
            .map_err(|why| format!("cannot reserve Statistics V3 planned records: {why}"))?;
        records.push(encode_manifest(
            manifest,
            RecordKindV3::Data,
            first_physical,
        )?);
        records.push(encode_family(
            manifest.authority_id,
            self.nifty,
            first_physical
                .checked_add(1)
                .ok_or_else(|| "Statistics V3 NIFTY record index overflowed".to_owned())?,
        )?);
        records.push(encode_family(
            manifest.authority_id,
            self.banknifty,
            first_physical
                .checked_add(2)
                .ok_or_else(|| "Statistics V3 BANKNIFTY record index overflowed".to_owned())?,
        )?);
        for (index, candidate) in self.candidates.iter().copied().enumerate() {
            let physical = first_physical
                .checked_add(3)
                .and_then(|value| value.checked_add(u64::try_from(index).ok()?))
                .ok_or_else(|| "Statistics V3 candidate record index overflowed".to_owned())?;
            records.push(encode_candidate(candidate, physical)?);
        }
        let completion = first_physical
            .checked_add(
                manifest
                    .block_records()?
                    .checked_sub(1)
                    .ok_or_else(|| "Statistics V3 completion ordinal underflowed".to_owned())?,
            )
            .ok_or_else(|| "Statistics V3 completion index overflowed".to_owned())?;
        records.push(encode_manifest(
            manifest,
            RecordKindV3::Completion,
            completion,
        )?);
        if records.len() != count {
            return Err("Statistics V3 planned record count differs".to_owned());
        }
        Ok(records)
    }
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGenerationV3 {
    len: u64,
    digest: [u8; 32],
    device: u64,
    inode: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(not(unix))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGenerationV3 {
    len: u64,
    digest: [u8; 32],
    modified: Option<std::time::SystemTime>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OrphanV3 {
    first_record: u64,
    present_records: u64,
    manifest: ManifestV3,
}

/// Bounded, generation-checked Statistics V3 ledger.
pub struct PopulationStatisticsV3Ledger {
    lock_path: PathBuf,
    data_path: PathBuf,
    lock_file: File,
    data_file: File,
    bounds: PopulationStatisticsV3Bounds,
    audits: HashMap<[u8; 32], PopulationStatisticsV3ReopenAudit>,
    completed: u64,
    orphan: Option<OrphanV3>,
    lock_generation: FileGenerationV3,
    data_generation: FileGenerationV3,
    writable: bool,
}

impl PopulationStatisticsV3Ledger {
    /// Opens existing bytes read-only and validates every completed authority.
    ///
    /// # Errors
    ///
    /// Refuses an unsafe path, malformed/beyond-bound file, invalid record,
    /// noncanonical block, duplicate identity, or changed held generation.
    pub fn open_read(
        root: impl AsRef<Path>,
        bounds: PopulationStatisticsV3Bounds,
    ) -> Result<Self, PopulationStatisticsV3Refusal> {
        Self::open_inner(root.as_ref(), bounds, false)
    }

    fn open_writer(
        root: impl AsRef<Path>,
        bounds: PopulationStatisticsV3Bounds,
    ) -> Result<Self, PopulationStatisticsV3Refusal> {
        Self::open_inner(root.as_ref(), bounds, true)
    }

    fn open_inner(
        root: &Path,
        bounds: PopulationStatisticsV3Bounds,
        writable: bool,
    ) -> Result<Self, PopulationStatisticsV3Refusal> {
        let metadata = std::fs::metadata(root).map_err(|why| {
            format!(
                "Statistics V3 root {} must already exist: {why}",
                root.display()
            )
        })?;
        if !metadata.is_dir() {
            return Err(format!(
                "Statistics V3 root {} is not a directory",
                root.display()
            ));
        }
        let lock_path = root.join(LOCK_FILE);
        let data_path = root.join(DATA_FILE);
        let lock_file = open_file(&lock_path, writable, writable)?;
        if writable {
            lock_file
                .lock()
                .map_err(|why| format!("cannot lock {}: {why}", lock_path.display()))?;
        } else {
            lock_file
                .lock_shared()
                .map_err(|why| format!("cannot share-lock {}: {why}", lock_path.display()))?;
        }
        let held_lock = lock_file
            .try_clone()
            .map_err(|why| format!("cannot clone {}: {why}", lock_path.display()))?;
        let opened = (|| {
            let mut data_file = open_file(&data_path, writable, writable)?;
            if writable {
                ensure_header(&mut data_file, &data_path)?;
            } else {
                verify_header(&mut data_file, &data_path)?;
            }
            let lock_generation = file_generation(&held_lock, &lock_path, LOCK_FILE_MAX_BYTES)?;
            let data_generation = file_generation(&data_file, &data_path, bounds.file_bytes)?;
            let mut ledger = Self {
                lock_path: lock_path.clone(),
                data_path,
                lock_file: held_lock,
                data_file,
                bounds,
                audits: HashMap::new(),
                completed: 0,
                orphan: None,
                lock_generation,
                data_generation,
                writable,
            };
            ledger.scan()?;
            Ok(ledger)
        })();
        let unlocked = lock_file
            .unlock()
            .map_err(|why| format!("cannot unlock {}: {why}", lock_path.display()));
        match (opened, unlocked) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn scan(&mut self) -> Result<(), PopulationStatisticsV3Refusal> {
        verify_header(&mut self.data_file, &self.data_path)?;
        let len = self
            .data_file
            .metadata()
            .map_err(|why| format!("cannot stat {}: {why}", self.data_path.display()))?
            .len();
        if len > self.bounds.file_bytes {
            return Err(format!(
                "Statistics V3 file has {len} bytes above maximum {}",
                self.bounds.file_bytes
            ));
        }
        let records = record_count(len)?;
        let mut cursor = 0_u64;
        while cursor < records {
            if self.completed >= self.bounds.audits {
                return Err("Statistics V3 authorities exceed explicit maximum".to_owned());
            }
            let data = read_record(&mut self.data_file, cursor)?;
            if data.kind != RecordKindV3::Data {
                return Err(format!(
                    "Statistics V3 block at record {cursor} does not begin with Data"
                ));
            }
            let manifest = decode_manifest(&data.payload, data.kind)?;
            if manifest.sequence != self.completed
                || manifest.candidate_count > self.bounds.candidates_per_audit
            {
                return Err("Statistics V3 sequence or candidate bound differs".to_owned());
            }
            let planned = manifest.block_records()?;
            let remaining = records
                .checked_sub(cursor)
                .ok_or_else(|| "Statistics V3 remaining records underflowed".to_owned())?;
            if remaining < planned {
                validate_prefix(&mut self.data_file, cursor, remaining, manifest)?;
                self.orphan = Some(OrphanV3 {
                    first_record: cursor,
                    present_records: remaining,
                    manifest,
                });
                cursor = records;
                continue;
            }
            let audit = validate_complete_block(&mut self.data_file, cursor, manifest)?;
            if self.audits.insert(manifest.authority_id, audit).is_some() {
                return Err("Statistics V3 authority identity is duplicated".to_owned());
            }
            self.completed = self
                .completed
                .checked_add(1)
                .ok_or_else(|| "Statistics V3 authority count overflowed".to_owned())?;
            cursor = cursor
                .checked_add(planned)
                .ok_or_else(|| "Statistics V3 scan cursor overflowed".to_owned())?;
        }
        Ok(())
    }

    /// Completed authority count.
    #[must_use]
    pub const fn completed_audits(&self) -> u64 {
        self.completed
    }

    /// Looks up one audit only after verifying the named file generations.
    ///
    /// # Errors
    ///
    /// Refuses if the held lock/data files or their named paths changed, or if
    /// locking and generation revalidation fails.
    pub fn reopen_audit(
        &mut self,
        authority_id: &[u8; 32],
    ) -> Result<Option<PopulationStatisticsV3ReopenAudit>, PopulationStatisticsV3Refusal> {
        self.with_shared_lock(|ledger| Ok(ledger.audits.get(authority_id).copied()))
    }

    /// Reads one candidate through its validated fixed-record coordinate.
    ///
    /// # Errors
    ///
    /// Refuses a changed ledger generation, absent authority, out-of-range
    /// coordinate, corrupt record, or mismatched candidate authority/sequence.
    pub fn candidate(
        &mut self,
        authority_id: &[u8; 32],
        sequence: u64,
    ) -> Result<PopulationStatisticsCandidateV3, PopulationStatisticsV3Refusal> {
        self.with_shared_lock(|ledger| {
            let audit = ledger
                .audits
                .get(authority_id)
                .copied()
                .ok_or_else(|| "Statistics V3 authority is absent".to_owned())?;
            if sequence >= audit.candidate_count() {
                return Err("Statistics V3 candidate is outside authority".to_owned());
            }
            let nifty_record = read_record(
                &mut ledger.data_file,
                audit
                    .first_record
                    .checked_add(1)
                    .ok_or_else(|| "Statistics V3 NIFTY index overflowed".to_owned())?,
            )?;
            let bank_record = read_record(
                &mut ledger.data_file,
                audit
                    .first_record
                    .checked_add(2)
                    .ok_or_else(|| "Statistics V3 BANKNIFTY index overflowed".to_owned())?,
            )?;
            let nifty = decode_family_record(&nifty_record.payload)?;
            let bank = decode_family_record(&bank_record.payload)?;
            let physical = audit
                .first_record
                .checked_add(3)
                .and_then(|value| value.checked_add(sequence))
                .ok_or_else(|| "Statistics V3 candidate index overflowed".to_owned())?;
            let decoded = read_record(&mut ledger.data_file, physical)?;
            if decoded.kind != RecordKindV3::Candidate {
                return Err("Statistics V3 candidate record kind differs".to_owned());
            }
            let candidate = decode_candidate_record(&decoded.payload)?;
            let family = if candidate.family == InstrumentFamilyV1::Nifty {
                &nifty
            } else {
                &bank
            };
            candidate.validate(family, sequence)?;
            Ok(candidate)
        })
    }

    fn with_shared_lock<T>(
        &mut self,
        action: impl FnOnce(&mut Self) -> Result<T, PopulationStatisticsV3Refusal>,
    ) -> Result<T, PopulationStatisticsV3Refusal> {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot share-lock {}: {why}", self.lock_path.display()))?;
        let result = (|| {
            require_generation(self.lock_generation, &self.lock_file, &self.lock_path)?;
            require_generation(self.data_generation, &self.data_file, &self.data_path)?;
            action(self)
        })();
        let unlocked = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot unlock {}: {why}", self.lock_path.display()));
        match (result, unlocked) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn append(
        &mut self,
        produced: &ProducedPopulationStatisticsV3,
    ) -> Result<PopulationStatisticsV3Commit, PopulationStatisticsV3Refusal> {
        if !self.writable {
            return Err("Statistics V3 read-only ledger cannot append".to_owned());
        }
        self.lock_file
            .lock()
            .map_err(|why| format!("cannot lock {}: {why}", self.lock_path.display()))?;
        let result = self.append_locked(produced);
        let unlocked = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot unlock {}: {why}", self.lock_path.display()));
        match (result, unlocked) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one linear receipt-last append keeps exact-prefix retry, sync ordering, and generation checks adjacent"
    )]
    fn append_locked(
        &mut self,
        produced: &ProducedPopulationStatisticsV3,
    ) -> Result<PopulationStatisticsV3Commit, PopulationStatisticsV3Refusal> {
        require_generation(self.lock_generation, &self.lock_file, &self.lock_path)?;
        require_generation(self.data_generation, &self.data_file, &self.data_path)?;
        produced.manifest.validate()?;
        produced.nifty.validate()?;
        produced.banknifty.validate()?;

        if let Some(existing) = self.audits.get(&produced.manifest.authority_id).copied() {
            let planned = produced.records(existing.sequence(), existing.first_record)?;
            compare_prefix(
                &mut self.data_file,
                existing.first_record,
                u64_of(planned.len(), "Statistics V3 planned records")?,
                &planned,
            )?;
            return Ok(PopulationStatisticsV3Commit::Reused(existing));
        }

        if let Some(orphan) = self.orphan {
            if orphan.manifest.authority_id != produced.manifest.authority_id {
                return Err("Statistics V3 foreign retry cannot replace trailing prefix".to_owned());
            }
            let planned = produced.records(orphan.manifest.sequence, orphan.first_record)?;
            compare_prefix(
                &mut self.data_file,
                orphan.first_record,
                orphan.present_records,
                &planned,
            )?;
            let present = usize_of(orphan.present_records, "Statistics V3 orphan records")?;
            let completion_ordinal = planned
                .len()
                .checked_sub(1)
                .ok_or_else(|| "Statistics V3 retry completion is absent".to_owned())?;
            for raw in planned
                .get(present..completion_ordinal)
                .ok_or_else(|| "Statistics V3 retry evidence range is invalid".to_owned())?
            {
                append_raw(&mut self.data_file, raw)?;
            }
            self.data_file
                .sync_all()
                .map_err(|why| format!("cannot sync Statistics V3 retry evidence: {why}"))?;
            let completion = planned
                .last()
                .ok_or_else(|| "Statistics V3 retry completion is absent".to_owned())?;
            let current_records = record_count(
                self.data_file
                    .metadata()
                    .map_err(|why| format!("cannot stat retry file: {why}"))?
                    .len(),
            )?;
            let completion_physical = orphan
                .first_record
                .checked_add(u64_of(
                    completion_ordinal,
                    "Statistics V3 completion ordinal",
                )?)
                .ok_or_else(|| "Statistics V3 retry completion index overflowed".to_owned())?;
            if current_records != completion_physical {
                return Err("Statistics V3 retry evidence did not end before Completion".to_owned());
            }
            append_raw(&mut self.data_file, completion)?;
            self.data_file
                .sync_all()
                .map_err(|why| format!("cannot sync Statistics V3 retry completion: {why}"))?;
            let audit =
                validate_complete_block(&mut self.data_file, orphan.first_record, orphan.manifest)?;
            self.audits.insert(audit.authority_id(), audit);
            self.completed = self
                .completed
                .checked_add(1)
                .ok_or_else(|| "Statistics V3 authority count overflowed".to_owned())?;
            self.orphan = None;
            self.data_generation =
                file_generation(&self.data_file, &self.data_path, self.bounds.file_bytes)?;
            return Ok(PopulationStatisticsV3Commit::Written(audit));
        }

        let first = record_count(
            self.data_file
                .metadata()
                .map_err(|why| format!("cannot stat Statistics V3 append file: {why}"))?
                .len(),
        )?;
        let planned = produced.records(self.completed, first)?;
        let planned_bytes = u64_of(planned.len(), "Statistics V3 planned records")?
            .checked_mul(POPULATION_STATISTICS_V3_RECORD_STRIDE)
            .and_then(|bytes| bytes.checked_add(record_offset(first).ok()?))
            .ok_or_else(|| "Statistics V3 planned file size overflowed".to_owned())?;
        if produced.manifest.candidate_count > self.bounds.candidates_per_audit
            || self.completed >= self.bounds.audits
            || planned_bytes > self.bounds.file_bytes
        {
            return Err("Statistics V3 append exceeds explicit bounds".to_owned());
        }
        let non_completion = planned
            .len()
            .checked_sub(1)
            .ok_or_else(|| "Statistics V3 planned block lacks Completion".to_owned())?;
        for raw in planned.iter().take(non_completion) {
            append_raw(&mut self.data_file, raw)?;
        }
        self.data_file
            .sync_all()
            .map_err(|why| format!("cannot sync Statistics V3 evidence: {why}"))?;
        append_raw(
            &mut self.data_file,
            planned
                .last()
                .ok_or_else(|| "Statistics V3 Completion disappeared".to_owned())?,
        )?;
        self.data_file
            .sync_all()
            .map_err(|why| format!("cannot sync Statistics V3 Completion: {why}"))?;
        let mut written_manifest = produced.manifest;
        written_manifest.sequence = self.completed;
        let audit = validate_complete_block(&mut self.data_file, first, written_manifest)?;
        self.audits.insert(audit.authority_id(), audit);
        self.completed = self
            .completed
            .checked_add(1)
            .ok_or_else(|| "Statistics V3 authority count overflowed".to_owned())?;
        self.data_generation =
            file_generation(&self.data_file, &self.data_path, self.bounds.file_bytes)?;
        Ok(PopulationStatisticsV3Commit::Written(audit))
    }
}

fn append_population_statistics_v3(
    root: impl AsRef<Path>,
    bounds: PopulationStatisticsV3Bounds,
    produced: &ProducedPopulationStatisticsV3,
) -> Result<PopulationStatisticsV3Commit, PopulationStatisticsV3Refusal> {
    let root = root.as_ref();
    let mut writer = PopulationStatisticsV3Ledger::open_writer(root, bounds)?;
    let committed = writer.append(produced)?;
    let expected = committed.audit();
    drop(writer);
    let reopened = PopulationStatisticsV3Ledger::open_read(root, bounds)?
        .reopen_audit(&expected.authority_id())?
        .ok_or_else(|| "Statistics V3 authority disappeared after append".to_owned())?;
    if reopened != expected {
        return Err("Statistics V3 fresh reopen differs from committed bytes".to_owned());
    }
    Ok(match committed {
        PopulationStatisticsV3Commit::Written(_) => PopulationStatisticsV3Commit::Written(reopened),
        PopulationStatisticsV3Commit::Reused(_) => PopulationStatisticsV3Commit::Reused(reopened),
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "complete-block validation deliberately checks the full adjacent Data/families/candidates/Completion grammar in one pass"
)]
fn validate_complete_block(
    file: &mut File,
    first: u64,
    manifest: ManifestV3,
) -> Result<PopulationStatisticsV3ReopenAudit, PopulationStatisticsV3Refusal> {
    let data = read_record(file, first)?;
    if data.kind != RecordKindV3::Data || decode_manifest(&data.payload, data.kind)? != manifest {
        return Err("Statistics V3 Data differs from expected manifest".to_owned());
    }
    let nifty_record = read_record(
        file,
        first
            .checked_add(1)
            .ok_or_else(|| "Statistics V3 NIFTY index overflowed".to_owned())?,
    )?;
    let bank_record = read_record(
        file,
        first
            .checked_add(2)
            .ok_or_else(|| "Statistics V3 BANKNIFTY index overflowed".to_owned())?,
    )?;
    if nifty_record.kind != RecordKindV3::Family || bank_record.kind != RecordKindV3::Family {
        return Err("Statistics V3 block lacks both Family records".to_owned());
    }
    if get_32(&nifty_record.payload, 16)? != manifest.authority_id
        || get_32(&bank_record.payload, 16)? != manifest.authority_id
    {
        return Err("Statistics V3 Family names another authority".to_owned());
    }
    let nifty = decode_family_record(&nifty_record.payload)?;
    let banknifty = decode_family_record(&bank_record.payload)?;
    if nifty.family != InstrumentFamilyV1::Nifty
        || banknifty.family != InstrumentFamilyV1::BankNifty
        || nifty.identity != manifest.nifty_family_identity
        || banknifty.identity != manifest.banknifty_family_identity
        || nifty.candidate_count.checked_add(banknifty.candidate_count)
            != Some(manifest.candidate_count)
    {
        return Err("Statistics V3 Family order, identity or count differs".to_owned());
    }
    validate_family_procedure(manifest, nifty)?;
    validate_family_procedure(manifest, banknifty)?;
    let mut candidates = Vec::new();
    candidates
        .try_reserve_exact(usize_of(
            manifest.candidate_count,
            "Statistics V3 candidates",
        )?)
        .map_err(|why| format!("cannot reserve reopened Statistics V3 candidates: {why}"))?;
    for sequence in 0..manifest.candidate_count {
        let physical = first
            .checked_add(3)
            .and_then(|value| value.checked_add(sequence))
            .ok_or_else(|| "Statistics V3 candidate index overflowed".to_owned())?;
        let record = read_record(file, physical)?;
        if record.kind != RecordKindV3::Candidate {
            return Err("Statistics V3 candidate record kind differs".to_owned());
        }
        let candidate = decode_candidate_record(&record.payload)?;
        let family = if sequence < nifty.candidate_count {
            &nifty
        } else {
            &banknifty
        };
        candidate.validate(family, sequence)?;
        validate_candidate_procedure(manifest, candidate)?;
        let expected_family_sequence = if family.family == InstrumentFamilyV1::Nifty {
            sequence
        } else {
            sequence
                .checked_sub(nifty.candidate_count)
                .ok_or_else(|| "Statistics V3 BANKNIFTY sequence underflowed".to_owned())?
        };
        if candidate.family_sequence != expected_family_sequence {
            return Err("Statistics V3 family candidate sequence differs".to_owned());
        }
        candidates.push(candidate);
    }
    let reopened_candidate_digest = if candidates.is_empty() {
        [0; 32]
    } else {
        ordered_candidate_digest(&candidates)
    };
    if reopened_candidate_digest != manifest.ordered_candidate_digest {
        return Err("Statistics V3 candidate evidence digest does not recompute".to_owned());
    }
    validate_rank_permutation(&candidates, &nifty)?;
    validate_rank_permutation(&candidates, &banknifty)?;
    let completion_index = first
        .checked_add(
            manifest
                .block_records()?
                .checked_sub(1)
                .ok_or_else(|| "Statistics V3 completion ordinal underflowed".to_owned())?,
        )
        .ok_or_else(|| "Statistics V3 completion index overflowed".to_owned())?;
    let completion = read_record(file, completion_index)?;
    if completion.kind != RecordKindV3::Completion
        || decode_manifest(&completion.payload, completion.kind)? != manifest
    {
        return Err("Statistics V3 receipt-last Completion differs from Data".to_owned());
    }
    Ok(reopen_audit_from_parts(
        first,
        manifest,
        nifty,
        banknifty,
        digest_domain(RECORD_DOMAIN, &completion.payload),
    ))
}

fn reopen_audit_from_parts(
    first_record: u64,
    manifest: ManifestV3,
    nifty: FamilyEvidenceV3,
    banknifty: FamilyEvidenceV3,
    completion_digest: [u8; 32],
) -> PopulationStatisticsV3ReopenAudit {
    PopulationStatisticsV3ReopenAudit {
        first_record,
        completion_digest,
        manifest,
        nifty_terminal: nifty.terminal,
        banknifty_terminal: banknifty.terminal,
    }
}

fn validate_rank_permutation(
    candidates: &[PopulationStatisticsCandidateV3],
    family: &FamilyEvidenceV3,
) -> Result<(), PopulationStatisticsV3Refusal> {
    if family.candidate_count == 0 {
        return Ok(());
    }
    let mut seen = vec![false; usize_of(family.candidate_count, "Statistics V3 family ranks")?];
    for candidate in candidates
        .iter()
        .filter(|candidate| candidate.family == family.family)
    {
        let slot = seen
            .get_mut(usize_of(candidate.romano_wolf_rank, "Statistics V3 rank")?)
            .ok_or_else(|| "Statistics V3 Romano-Wolf rank is outside family".to_owned())?;
        if *slot {
            return Err("Statistics V3 Romano-Wolf rank is duplicated".to_owned());
        }
        *slot = true;
    }
    if seen.contains(&false) {
        return Err("Statistics V3 Romano-Wolf rank permutation is incomplete".to_owned());
    }
    Ok(())
}

fn validate_prefix(
    file: &mut File,
    first: u64,
    present: u64,
    manifest: ManifestV3,
) -> Result<(), PopulationStatisticsV3Refusal> {
    if present == 0 || present >= manifest.block_records()? {
        return Err("Statistics V3 trailing prefix length is invalid".to_owned());
    }
    for local in 0..present {
        let physical = first
            .checked_add(local)
            .ok_or_else(|| "Statistics V3 prefix index overflowed".to_owned())?;
        let record = read_record(file, physical)?;
        let expected = if local == 0 {
            RecordKindV3::Data
        } else if local <= 2 {
            RecordKindV3::Family
        } else {
            RecordKindV3::Candidate
        };
        if record.kind != expected {
            return Err("Statistics V3 trailing prefix record order differs".to_owned());
        }
        match record.kind {
            RecordKindV3::Data => {
                if decode_manifest(&record.payload, record.kind)? != manifest {
                    return Err("Statistics V3 prefix Data differs".to_owned());
                }
            }
            RecordKindV3::Family => {
                let family = decode_family_record(&record.payload)?;
                let expected_family = if local == 1 {
                    InstrumentFamilyV1::Nifty
                } else {
                    InstrumentFamilyV1::BankNifty
                };
                if family.family != expected_family
                    || get_32(&record.payload, 16)? != manifest.authority_id
                {
                    return Err("Statistics V3 prefix Family differs".to_owned());
                }
            }
            RecordKindV3::Candidate => {
                let candidate = decode_candidate_record(&record.payload)?;
                if candidate.authority_id != manifest.authority_id
                    || candidate.global_sequence != local - 3
                {
                    return Err("Statistics V3 prefix candidate differs".to_owned());
                }
            }
            RecordKindV3::Completion => {
                return Err("Statistics V3 incomplete prefix contains Completion".to_owned());
            }
        }
    }
    Ok(())
}

fn compare_prefix(
    file: &mut File,
    first: u64,
    present: u64,
    planned: &[[u8; RECORD_BYTES]],
) -> Result<(), PopulationStatisticsV3Refusal> {
    if present > u64_of(planned.len(), "Statistics V3 planned records")? {
        return Err("Statistics V3 retry prefix exceeds planned block".to_owned());
    }
    for local in 0..present {
        let observed = read_raw(
            file,
            first
                .checked_add(local)
                .ok_or_else(|| "Statistics V3 retry index overflowed".to_owned())?,
        )?;
        let expected = planned
            .get(usize_of(local, "Statistics V3 retry record")?)
            .ok_or_else(|| "Statistics V3 retry record disappeared".to_owned())?;
        if &observed != expected {
            return Err(format!(
                "Statistics V3 exact retry differs at local record {local}"
            ));
        }
    }
    Ok(())
}

fn validate_family_procedure(
    manifest: ManifestV3,
    family: FamilyEvidenceV3,
) -> Result<(), PopulationStatisticsV3Refusal> {
    if family.terminal == StatisticsFamilyTerminalV3::NaturallyExtinct {
        return Ok(());
    }
    let denominator = manifest
        .draws
        .checked_add(1)
        .ok_or_else(|| "Statistics V3 bootstrap denominator overflowed".to_owned())?;
    let white = family
        .white
        .ok_or_else(|| "Statistics V3 evaluated family lacks White".to_owned())?;
    let spa = family
        .spa
        .ok_or_else(|| "Statistics V3 evaluated family lacks SPA".to_owned())?;
    if white.probability.denominator != denominator || spa.probability.denominator != denominator {
        return Err("Statistics V3 family bootstrap denominator differs from procedure".to_owned());
    }
    Ok(())
}

fn validate_candidate_procedure(
    manifest: ManifestV3,
    candidate: PopulationStatisticsCandidateV3,
) -> Result<(), PopulationStatisticsV3Refusal> {
    let denominator = manifest
        .draws
        .checked_add(1)
        .ok_or_else(|| "Statistics V3 bootstrap denominator overflowed".to_owned())?;
    if candidate.romano_wolf_initial.denominator != denominator
        || candidate.romano_wolf_adjusted.denominator != denominator
    {
        return Err(
            "Statistics V3 candidate Romano-Wolf denominator differs from procedure".to_owned(),
        );
    }
    Ok(())
}

fn header() -> Result<[u8; HEADER_BYTES], PopulationStatisticsV3Refusal> {
    let mut payload = [0_u8; 32];
    put_bytes(&mut payload, 0, &HEADER_MAGIC)?;
    put_u32(&mut payload, 16, HEADER_VERSION)?;
    put_u32(&mut payload, 20, HEADER_KIND)?;
    put_u64(&mut payload, 24, POPULATION_STATISTICS_V3_RECORD_STRIDE)?;
    let mut raw = [0_u8; HEADER_BYTES];
    put_bytes(&mut raw, 0, &payload)?;
    put_bytes(&mut raw, 32, &digest_domain(HEADER_DOMAIN, &payload))?;
    Ok(raw)
}

fn ensure_header(file: &mut File, path: &Path) -> Result<(), PopulationStatisticsV3Refusal> {
    let len = file
        .metadata()
        .map_err(|why| format!("cannot stat {}: {why}", path.display()))?
        .len();
    if len == 0 {
        let bytes = header()?;
        file.seek(SeekFrom::Start(0))
            .and_then(|_| file.write_all(&bytes))
            .and_then(|()| file.sync_all())
            .map_err(|why| format!("cannot initialize {}: {why}", path.display()))?;
        return Ok(());
    }
    verify_header(file, path)
}

fn verify_header(file: &mut File, path: &Path) -> Result<(), PopulationStatisticsV3Refusal> {
    let len = file
        .metadata()
        .map_err(|why| format!("cannot stat {}: {why}", path.display()))?
        .len();
    if len < POPULATION_STATISTICS_V3_HEADER_BYTES {
        return Err(format!(
            "{} is {len} bytes, shorter than Statistics V3 header",
            path.display()
        ));
    }
    let mut observed = [0_u8; HEADER_BYTES];
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_exact(&mut observed))
        .map_err(|why| format!("cannot read {} header: {why}", path.display()))?;
    if observed != header()? {
        return Err(format!(
            "{} Statistics V3 header is unknown or corrupt",
            path.display()
        ));
    }
    Ok(())
}

fn record_count(file_len: u64) -> Result<u64, PopulationStatisticsV3Refusal> {
    let body = file_len
        .checked_sub(POPULATION_STATISTICS_V3_HEADER_BYTES)
        .ok_or_else(|| "Statistics V3 file is shorter than header".to_owned())?;
    if body % POPULATION_STATISTICS_V3_RECORD_STRIDE != 0 {
        return Err(format!(
            "Statistics V3 body has {body} ragged bytes against stride {POPULATION_STATISTICS_V3_RECORD_STRIDE}"
        ));
    }
    Ok(body / POPULATION_STATISTICS_V3_RECORD_STRIDE)
}

fn record_offset(index: u64) -> Result<u64, PopulationStatisticsV3Refusal> {
    POPULATION_STATISTICS_V3_HEADER_BYTES
        .checked_add(
            index
                .checked_mul(POPULATION_STATISTICS_V3_RECORD_STRIDE)
                .ok_or_else(|| "Statistics V3 byte offset overflowed".to_owned())?,
        )
        .ok_or_else(|| "Statistics V3 header offset overflowed".to_owned())
}

fn read_record(
    file: &mut File,
    index: u64,
) -> Result<DecodedRecordV3, PopulationStatisticsV3Refusal> {
    let raw = read_raw(file, index)?;
    decode_record(&raw, index)
}

fn read_raw(
    file: &mut File,
    index: u64,
) -> Result<[u8; RECORD_BYTES], PopulationStatisticsV3Refusal> {
    let mut raw = [0_u8; RECORD_BYTES];
    file.seek(SeekFrom::Start(record_offset(index)?))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| format!("cannot read raw Statistics V3 record {index}: {why}"))?;
    Ok(raw)
}

fn append_raw(
    file: &mut File,
    raw: &[u8; RECORD_BYTES],
) -> Result<(), PopulationStatisticsV3Refusal> {
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(raw))
        .map_err(|why| format!("cannot append Statistics V3 record: {why}"))
}

fn open_file(
    path: &Path,
    writable: bool,
    create: bool,
) -> Result<File, PopulationStatisticsV3Refusal> {
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
        .map_err(|why| format!("cannot open {}: {why}", path.display()))?;
    require_not_symlink(path, false)?;
    require_regular_file(&file, path)?;
    Ok(file)
}

fn require_not_symlink(
    path: &Path,
    absent_is_allowed: bool,
) -> Result<(), PopulationStatisticsV3Refusal> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(format!(
            "Statistics V3 path {} is a symbolic link; no-follow refused it",
            path.display()
        )),
        Ok(_) => Ok(()),
        Err(why) if absent_is_allowed && why.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(why) => Err(format!(
            "cannot inspect Statistics V3 path {} without following links: {why}",
            path.display()
        )),
    }
}

fn require_regular_file(file: &File, path: &Path) -> Result<(), PopulationStatisticsV3Refusal> {
    if !file
        .metadata()
        .map_err(|why| format!("cannot stat Statistics V3 file {}: {why}", path.display()))?
        .is_file()
    {
        return Err(format!(
            "Statistics V3 path {} is not a regular file",
            path.display()
        ));
    }
    Ok(())
}

fn file_generation(
    file: &File,
    path: &Path,
    max_bytes: u64,
) -> Result<FileGenerationV3, PopulationStatisticsV3Refusal> {
    let held_before = file
        .metadata()
        .map_err(|why| format!("cannot stat held {}: {why}", path.display()))?;
    let mut named = open_file(path, false, false)
        .map_err(|why| format!("cannot reopen named {}: {why}", path.display()))?;
    let named_before = named
        .metadata()
        .map_err(|why| format!("cannot stat named {}: {why}", path.display()))?;
    #[cfg(unix)]
    if (held_before.dev(), held_before.ino()) != (named_before.dev(), named_before.ino()) {
        return Err(format!("held file no longer names {}", path.display()));
    }
    if held_before.len() != named_before.len() {
        return Err(format!(
            "{} changed while generation was measured",
            path.display()
        ));
    }
    let measured_len = held_before.len();
    if measured_len > max_bytes {
        return Err(format!(
            "{} has {measured_len} bytes above generation-hash maximum {max_bytes}",
            path.display()
        ));
    }
    let digest = hash_file(&mut named, path, measured_len)?;
    if hash_file(&mut named, path, measured_len)? != digest {
        return Err(format!(
            "{} changed during bounded generation hashing",
            path.display()
        ));
    }
    let held_after = file
        .metadata()
        .map_err(|why| format!("cannot restat held {}: {why}", path.display()))?;
    let named_after = named
        .metadata()
        .map_err(|why| format!("cannot restat named {}: {why}", path.display()))?;
    let post_named = open_file(path, false, false)
        .map_err(|why| format!("cannot recheck named {}: {why}", path.display()))?;
    let post_named_metadata = post_named
        .metadata()
        .map_err(|why| format!("cannot recheck metadata for {}: {why}", path.display()))?;
    #[cfg(unix)]
    if (held_after.dev(), held_after.ino()) != (named_after.dev(), named_after.ino())
        || (held_after.dev(), held_after.ino())
            != (post_named_metadata.dev(), post_named_metadata.ino())
    {
        return Err(format!("held file no longer names {}", path.display()));
    }
    let zero = [0_u8; 32];
    if generation_of(&held_before, zero) != generation_of(&held_after, zero)
        || generation_of(&named_before, zero) != generation_of(&named_after, zero)
        || generation_of(&named_after, zero) != generation_of(&post_named_metadata, zero)
    {
        return Err(format!(
            "{} changed while generation was measured",
            path.display()
        ));
    }
    Ok(generation_of(&held_after, digest))
}

#[cfg(unix)]
fn generation_of(metadata: &std::fs::Metadata, digest: [u8; 32]) -> FileGenerationV3 {
    FileGenerationV3 {
        len: metadata.len(),
        digest,
        device: metadata.dev(),
        inode: metadata.ino(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    }
}

#[cfg(not(unix))]
fn generation_of(metadata: &std::fs::Metadata, digest: [u8; 32]) -> FileGenerationV3 {
    FileGenerationV3 {
        len: metadata.len(),
        digest,
        modified: metadata.modified().ok(),
    }
}

fn hash_file(
    file: &mut File,
    path: &Path,
    exact_bytes: u64,
) -> Result<[u8; 32], PopulationStatisticsV3Refusal> {
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("cannot seek {} for generation hash: {why}", path.display()))?;
    let mut hasher = Hasher::new();
    hasher.update(GENERATION_DOMAIN);
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    let mut remaining = exact_bytes;
    while remaining != 0 {
        let requested = usize_of(
            remaining.min(READ_CHUNK_BYTES as u64),
            "Statistics V3 generation chunk",
        )?;
        let read = file
            .read(
                buffer
                    .get_mut(..requested)
                    .ok_or_else(|| "Statistics V3 hash request exceeded buffer".to_owned())?,
            )
            .map_err(|why| format!("cannot hash {}: {why}", path.display()))?;
        if read == 0 {
            return Err(format!(
                "{} ended before its captured {exact_bytes}-byte generation",
                path.display()
            ));
        }
        hasher.update(
            buffer
                .get(..read)
                .ok_or_else(|| "Statistics V3 hash read exceeded buffer".to_owned())?,
        );
        remaining = remaining
            .checked_sub(u64_of(read, "Statistics V3 generation read")?)
            .ok_or_else(|| "Statistics V3 generation remaining bytes underflowed".to_owned())?;
    }
    Ok(hasher.finalize())
}

fn require_generation(
    expected: FileGenerationV3,
    file: &File,
    path: &Path,
) -> Result<(), PopulationStatisticsV3Refusal> {
    if file_generation(file, path, expected.len)? != expected {
        return Err(format!(
            "{} changed after Statistics V3 open; cached audit refused",
            path.display()
        ));
    }
    Ok(())
}

fn digest_domain(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    hasher.update(bytes);
    hasher.finalize()
}

fn require_nonzero(name: &str, digest: [u8; 32]) -> Result<(), PopulationStatisticsV3Refusal> {
    if digest == [0; 32] {
        return Err(format!("Statistics V3 {name} digest is zero"));
    }
    Ok(())
}

fn require_finite(name: &str, bits: u64) -> Result<(), PopulationStatisticsV3Refusal> {
    if !f64::from_bits(bits).is_finite() {
        return Err(format!("Statistics V3 {name} is not finite"));
    }
    Ok(())
}

fn usize_of(value: u64, name: &str) -> Result<usize, PopulationStatisticsV3Refusal> {
    usize::try_from(value).map_err(|_| format!("{name} does not fit usize"))
}

fn u64_of(value: usize, name: &str) -> Result<u64, PopulationStatisticsV3Refusal> {
    u64::try_from(value).map_err(|_| format!("{name} does not fit u64"))
}

fn put_bytes(
    raw: &mut [u8],
    offset: usize,
    bytes: &[u8],
) -> Result<(), PopulationStatisticsV3Refusal> {
    let end = offset
        .checked_add(bytes.len())
        .ok_or_else(|| "Statistics V3 encode offset overflowed".to_owned())?;
    raw.get_mut(offset..end)
        .ok_or_else(|| "Statistics V3 encode destination is outside record".to_owned())?
        .copy_from_slice(bytes);
    Ok(())
}

fn put_u32(raw: &mut [u8], offset: usize, value: u32) -> Result<(), PopulationStatisticsV3Refusal> {
    put_bytes(raw, offset, &value.to_le_bytes())
}

fn put_u64(raw: &mut [u8], offset: usize, value: u64) -> Result<(), PopulationStatisticsV3Refusal> {
    put_bytes(raw, offset, &value.to_le_bytes())
}

fn get_array<const N: usize>(
    raw: &[u8],
    offset: usize,
) -> Result<[u8; N], PopulationStatisticsV3Refusal> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| "Statistics V3 decode offset overflowed".to_owned())?;
    raw.get(offset..end)
        .ok_or_else(|| {
            format!(
                "Statistics V3 decode requested {offset}..{end} from {} bytes",
                raw.len()
            )
        })?
        .try_into()
        .map_err(|_| "Statistics V3 fixed decode width differs".to_owned())
}

fn get_u32(raw: &[u8], offset: usize) -> Result<u32, PopulationStatisticsV3Refusal> {
    Ok(u32::from_le_bytes(get_array(raw, offset)?))
}

fn get_u64(raw: &[u8], offset: usize) -> Result<u64, PopulationStatisticsV3Refusal> {
    Ok(u64::from_le_bytes(get_array(raw, offset)?))
}

fn get_32(raw: &[u8], offset: usize) -> Result<[u8; 32], PopulationStatisticsV3Refusal> {
    get_array(raw, offset)
}

fn require_zero(
    raw: &[u8],
    offset: usize,
    len: usize,
    name: &str,
) -> Result<(), PopulationStatisticsV3Refusal> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| format!("Statistics V3 {name} reserve overflowed"))?;
    if raw
        .get(offset..end)
        .ok_or_else(|| format!("Statistics V3 {name} reserve is absent"))?
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err(format!("Statistics V3 {name} reserve is nonzero"));
    }
    Ok(())
}

fn wilson_lower_bits_v3(wins: u64, trades: u64) -> u64 {
    if trades == 0 {
        return 0.0_f64.to_bits();
    }
    wilson_lower_v3(wins, trades).to_bits()
}

#[expect(
    clippy::cast_precision_loss,
    clippy::float_arithmetic,
    reason = "Wilson is a full-precision statistic over counts, never a price"
)]
fn wilson_lower_v3(wins: u64, trades: u64) -> f64 {
    const Z: f64 = 1.959_964;
    let n = trades as f64;
    let p = wins.min(trades) as f64 / n;
    let z2 = Z * Z;
    let denominator = 1.0 + z2 / n;
    let centre = p + z2 / (2.0 * n);
    let margin = Z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt();
    ((centre - margin) / denominator).clamp(0.0, 1.0)
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

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Result<Self, String> {
            let serial = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "brutex-population-statistics-v3-{}-{serial}",
                std::process::id()
            ));
            std::fs::create_dir(&path)
                .map_err(|why| format!("cannot create Statistics V3 test root: {why}"))?;
            Ok(Self(path))
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn child(&self, name: &str) -> Result<PathBuf, String> {
            let path = self.0.join(name);
            std::fs::create_dir(&path)
                .map_err(|why| format!("cannot create Statistics V3 child root: {why}"))?;
            Ok(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn test_digest(tag: u8) -> [u8; 32] {
        let mut value = [tag.max(1); 32];
        value[31] = tag.max(1).wrapping_add(1);
        value
    }

    fn common_fixture() -> Result<CommonSourceV3, String> {
        Ok(CommonSourceV3 {
            rung_seconds: 300,
            horizon_bars: 32,
            requested_span: RequestedSpanIdentityV1::new(2024, 1, 2024, 1)?,
            feed_digest: test_digest(1),
            source_commit_digest: test_digest(2),
            calendar_policy_digest: test_digest(3),
            daily_reference_policy_digest: test_digest(4),
        })
    }

    fn family_fixture(
        family: InstrumentFamilyV1,
        terminal: StatisticsFamilyTerminalV3,
    ) -> Result<FamilyEvidenceV3, String> {
        let family_tag = if family == InstrumentFamilyV1::Nifty {
            20
        } else {
            40
        };
        let candidate_count = match terminal {
            StatisticsFamilyTerminalV3::Evaluated => 2,
            StatisticsFamilyTerminalV3::InsufficientForCscv => 1,
            StatisticsFamilyTerminalV3::NaturallyExtinct => 0,
        };
        let fraction = ExactFractionV3::new(1, 8)?;
        let family_test = FamilyTestV3 {
            statistic_bits: 1.25_f64.to_bits(),
            probability_bits: fraction.bits(),
            probability: fraction,
            family_digest: test_digest(family_tag + 8),
        };
        let evaluated = terminal != StatisticsFamilyTerminalV3::NaturallyExtinct;
        let mut value = FamilyEvidenceV3 {
            family,
            terminal,
            pre_admission_sequence: 0,
            pre_admission_record_index: u64::from(family_tag),
            pre_admission_authority_id: test_digest(family_tag),
            candidate_universe_id: test_digest(family_tag + 1),
            candidate_completion_digest: test_digest(family_tag + 2),
            observation_authority_id: if evaluated {
                [0; 32]
            } else {
                test_digest(family_tag + 3)
            },
            observation_identity: test_digest(family_tag + 3),
            observation_policy_digest: test_digest(family_tag + 4),
            observation_data_digest: if evaluated {
                [0; 32]
            } else {
                test_digest(family_tag + 5)
            },
            observation_completion_digest: if evaluated {
                [0; 32]
            } else {
                test_digest(family_tag + 6)
            },
            accepted_session_digest: if evaluated {
                test_digest(family_tag + 7)
            } else {
                [0; 32]
            },
            layout_digest: if evaluated {
                test_digest(family_tag + 9)
            } else {
                [0; 32]
            },
            candidate_count,
            period_count: if evaluated { 4 } else { 0 },
            split_count: if evaluated { 2 } else { 0 },
            segment_count: if evaluated { 2 } else { 0 },
            white: evaluated.then_some(family_test),
            spa: evaluated.then_some(FamilyTestV3 {
                family_digest: test_digest(family_tag + 10),
                ..family_test
            }),
            romano_wolf_family_digest: evaluated.then_some(test_digest(family_tag + 11)),
            pbo: (terminal == StatisticsFamilyTerminalV3::Evaluated).then_some(PboV3 {
                contributing_splits: 2,
                bottom_half_splits: 1,
                unrankable_splits: 0,
                probability_bits: 0.5_f64.to_bits(),
                probability: ExactFractionV3::new(1, 2)?,
            }),
            ordered_candidate_digest: if evaluated {
                test_digest(family_tag + 12)
            } else {
                [0; 32]
            },
            ordered_period_digest: if evaluated {
                test_digest(family_tag + 13)
            } else {
                [0; 32]
            },
            ordered_split_digest: if evaluated {
                test_digest(family_tag + 14)
            } else {
                [0; 32]
            },
            identity: [0; 32],
        };
        value.identity = family_identity(&value);
        value.validate()?;
        Ok(value)
    }

    fn candidates_for(
        family: FamilyEvidenceV3,
    ) -> Result<Vec<PopulationStatisticsCandidateV3>, String> {
        let mut candidates = Vec::new();
        for sequence in 0..family.candidate_count {
            let wins = 5_u64
                .checked_add(sequence)
                .ok_or_else(|| "test wins overflowed".to_owned())?;
            let statistic = if sequence == 0 { 1.0_f64 } else { 2.0_f64 };
            candidates.push(PopulationStatisticsCandidateV3 {
                authority_id: [0; 32],
                global_sequence: 0,
                family: family.family,
                family_sequence: sequence,
                candidate_semantic_digest: test_digest(
                    if family.family == InstrumentFamilyV1::Nifty {
                        70_u8
                    } else {
                        80_u8
                    }
                    .wrapping_add(
                        u8::try_from(sequence).map_err(|why| {
                            format!("test candidate sequence does not fit u8: {why}")
                        })?,
                    ),
                ),
                pre_admission_authority_id: family.pre_admission_authority_id,
                trades: 10,
                wins,
                wilson_lower_bits: wilson_lower_bits_v3(wins, 10),
                romano_wolf_statistic_bits: statistic.to_bits(),
                romano_wolf_rank: sequence,
                romano_wolf_strict_exceedances: sequence,
                romano_wolf_initial: ExactFractionV3::new(sequence + 1, 8)?,
                romano_wolf_adjusted: ExactFractionV3::new(sequence + 1, 8)?,
                ordered_period_digest: test_digest(
                    90_u8.wrapping_add(
                        u8::try_from(sequence).map_err(|why| {
                            format!("test period sequence does not fit u8: {why}")
                        })?,
                    ),
                ),
                ordered_split_digest: test_digest(
                    100_u8
                        .wrapping_add(u8::try_from(sequence).map_err(|why| {
                            format!("test split sequence does not fit u8: {why}")
                        })?),
                ),
            });
        }
        Ok(candidates)
    }

    fn produced_fixture(
        nifty_terminal: StatisticsFamilyTerminalV3,
        banknifty_terminal: StatisticsFamilyTerminalV3,
        seed: u64,
    ) -> Result<ProducedPopulationStatisticsV3, String> {
        let nifty = family_fixture(InstrumentFamilyV1::Nifty, nifty_terminal)?;
        let banknifty = family_fixture(InstrumentFamilyV1::BankNifty, banknifty_terminal)?;
        let mut candidates = candidates_for(nifty)?;
        candidates.extend(candidates_for(banknifty)?);
        let procedure = if candidates.is_empty() {
            None
        } else {
            Some(PopulationStatisticsProcedureV2::new(7, seed, 1)?)
        };
        build_produced(common_fixture()?, nifty, banknifty, candidates, procedure)
    }

    fn bounds() -> Result<PopulationStatisticsV3Bounds, String> {
        PopulationStatisticsV3Bounds::new(16, 16, 2 * 1_024 * 1_024)
    }

    /// Bounds for a two-family (E/E) audit.
    ///
    /// `candidates_per_audit` bounds the WHOLE audit, and an E/E audit carries
    /// both families' rows. D-0498 made a Candidate row `mask x direction x
    /// exit`, so one closed mask yields at least two and the E/E fixture
    /// produces 20 per family, 40 in the audit. `bounds()`'s 16 was sized
    /// before that correction and refuses the pair by 24 rows -- which is what
    /// `Statistics V3 append exceeds explicit bounds` was reporting, correctly.
    /// The ceiling here is 64 rather than 40 so the bound is a bound and not a
    /// second copy of the fixture's arithmetic.
    fn pair_bounds() -> Result<PopulationStatisticsV3Bounds, String> {
        PopulationStatisticsV3Bounds::new(16, 64, 2 * 1_024 * 1_024)
    }

    #[test]
    fn terminals_refuse_missing_or_fabricated_evidence_and_cover_both_orientations()
    -> Result<(), String> {
        let nifty_survives = produced_fixture(
            StatisticsFamilyTerminalV3::Evaluated,
            StatisticsFamilyTerminalV3::NaturallyExtinct,
            11,
        )?;
        assert_eq!(
            nifty_survives.nifty.terminal,
            StatisticsFamilyTerminalV3::Evaluated
        );
        assert_eq!(
            nifty_survives.banknifty.terminal,
            StatisticsFamilyTerminalV3::NaturallyExtinct
        );
        let bank_survives = produced_fixture(
            StatisticsFamilyTerminalV3::NaturallyExtinct,
            StatisticsFamilyTerminalV3::Evaluated,
            12,
        )?;
        assert_eq!(
            bank_survives.nifty.terminal,
            StatisticsFamilyTerminalV3::NaturallyExtinct
        );
        assert_eq!(
            bank_survives.banknifty.terminal,
            StatisticsFamilyTerminalV3::Evaluated
        );
        let insufficient = produced_fixture(
            StatisticsFamilyTerminalV3::InsufficientForCscv,
            StatisticsFamilyTerminalV3::NaturallyExtinct,
            13,
        )?;
        assert_eq!(insufficient.nifty.candidate_count, 1);
        assert_eq!(insufficient.nifty.pbo, None);

        let mut missing = family_fixture(
            InstrumentFamilyV1::Nifty,
            StatisticsFamilyTerminalV3::Evaluated,
        )?;
        missing.white = None;
        missing.identity = family_identity(&missing);
        assert!(missing.validate().is_err());

        let mut fabricated = family_fixture(
            InstrumentFamilyV1::BankNifty,
            StatisticsFamilyTerminalV3::NaturallyExtinct,
        )?;
        fabricated.white = Some(FamilyTestV3 {
            statistic_bits: 0.0_f64.to_bits(),
            probability_bits: 0.0_f64.to_bits(),
            probability: ExactFractionV3::new(0, 1)?,
            family_digest: test_digest(120),
        });
        fabricated.identity = family_identity(&fabricated);
        assert!(fabricated.validate().is_err());

        let mut crosswired = missing;
        crosswired.white = family_fixture(
            InstrumentFamilyV1::Nifty,
            StatisticsFamilyTerminalV3::Evaluated,
        )?
        .white;
        crosswired.terminal = StatisticsFamilyTerminalV3::NaturallyExtinct;
        crosswired.identity = family_identity(&crosswired);
        assert!(crosswired.validate().is_err());

        let nifty = family_fixture(
            InstrumentFamilyV1::Nifty,
            StatisticsFamilyTerminalV3::NaturallyExtinct,
        )?;
        let bank = family_fixture(
            InstrumentFamilyV1::BankNifty,
            StatisticsFamilyTerminalV3::NaturallyExtinct,
        )?;
        assert!(
            build_produced(common_fixture()?, bank, nifty, Vec::new(), None).is_err(),
            "crosswired family order must refuse"
        );
        Ok(())
    }

    #[test]
    fn all_extinct_requires_two_exact_authenticated_observation_v2_commits() -> Result<(), String> {
        let root = TestDir::new()?;
        let observation_root = root.child("observation")?;
        let ((nifty_source, nifty_pre_commit), (bank_source, bank_pre_commit)) =
            crate::pre_admission_data::observation_v2_zero_pair_production_fixture(130)?;
        let nifty = crate::population_observations_v1::produce_natural_extinction_observation_v2(
            &nifty_source,
            &nifty_pre_commit,
        )?;
        let bank = crate::population_observations_v1::produce_natural_extinction_observation_v2(
            &bank_source,
            &bank_pre_commit,
        )?;
        let observation_bounds =
            crate::population_observations_v1::ObservationAuthorityBoundsV2::new(
                4,
                2 * 1_024 * 1_024,
            )?;
        let nifty_commit = nifty.append_and_reopen(&observation_root, observation_bounds)?;
        let bank_commit = bank.append_and_reopen(&observation_root, observation_bounds)?;
        assert!(
            produce_all_extinct_population_statistics_v3(
                &nifty,
                &bank_commit,
                &bank,
                &nifty_commit,
            )
            .is_err(),
            "crosswired Observation V2 commits must refuse"
        );
        let produced = produce_all_extinct_population_statistics_v3(
            &nifty,
            &nifty_commit,
            &bank,
            &bank_commit,
        )?;
        assert!(produced.candidates.is_empty());
        assert_eq!(
            produced.nifty.terminal,
            StatisticsFamilyTerminalV3::NaturallyExtinct
        );
        assert_eq!(
            produced.banknifty.terminal,
            StatisticsFamilyTerminalV3::NaturallyExtinct
        );
        let statistics_root = root.child("statistics")?;
        let commit = produced.append_and_reopen(&statistics_root, bounds()?)?;
        assert_eq!(commit.audit().candidate_count(), 0);
        assert_eq!(
            commit.audit().nifty_terminal(),
            StatisticsFamilyTerminalV3::NaturallyExtinct
        );
        let source = produced.authenticated_admission_source(commit)?;
        assert!(source.admission_v4_candidates()?.is_empty());
        assert_eq!(
            source.nifty_family().terminal(),
            StatisticsFamilyTerminalV3::NaturallyExtinct
        );
        assert_eq!(
            source.banknifty_family().terminal(),
            StatisticsFamilyTerminalV3::NaturallyExtinct
        );
        Ok(())
    }

    #[test]
    fn two_evaluated_production_preserves_real_lineage_and_refuses_order_crosswire_and_stale_audit()
    -> Result<(), String> {
        let fixture: crate::step3_orchestrator::StatisticsV3EvaluatedPairFixture =
            crate::step3_orchestrator::statistics_v3_evaluated_pair_fixture()?;
        let procedure = PopulationStatisticsProcedureV2::new(16, 73, 2)?;
        let produced = produce_evaluated_population_statistics_v3(
            fixture.nifty_observations(),
            fixture.nifty_pre_admission(),
            fixture.banknifty_observations(),
            fixture.banknifty_pre_admission(),
            procedure,
        )?;
        assert_eq!(
            produced.nifty.terminal,
            StatisticsFamilyTerminalV3::Evaluated
        );
        assert_eq!(
            produced.banknifty.terminal,
            StatisticsFamilyTerminalV3::Evaluated
        );
        assert!(produced.nifty.candidate_count >= 2);
        assert!(produced.banknifty.candidate_count >= 2);
        assert_eq!(
            produced.manifest.candidate_count,
            produced
                .nifty
                .candidate_count
                .checked_add(produced.banknifty.candidate_count)
                .ok_or_else(|| "test E/E Candidate count overflowed".to_owned())?
        );
        // Exact, and therefore loud. The `>= 2` assertions above are the
        // SEMANTIC claim from D-0498 -- both directions expand, so a nonempty
        // production is at least two rows -- and they hold for 2 as well as for
        // 20, which is how a fixture can grow past its own bound in silence.
        // These three pin what the fixture actually produces, so the next
        // change to it fails here, naming the number, instead of failing inside
        // `append` with a bound that reads like a defect.
        assert_eq!(produced.nifty.candidate_count, 20, "E/E NIFTY rows");
        assert_eq!(produced.banknifty.candidate_count, 20, "E/E BANKNIFTY rows");
        assert_eq!(produced.manifest.candidate_count, 40, "E/E audit rows");

        let root = TestDir::new()?;
        let commit = produced.append_and_reopen(root.path(), pair_bounds()?)?;
        let source = produced.authenticated_admission_source(commit)?;
        assert_eq!(
            source.nifty_family().terminal(),
            StatisticsFamilyTerminalV3::Evaluated
        );
        assert_eq!(
            source.banknifty_family().terminal(),
            StatisticsFamilyTerminalV3::Evaluated
        );
        assert_eq!(
            u64::try_from(source.admission_v4_candidates()?.len())
                .map_err(|_| "test E/E projected length does not fit u64".to_owned())?,
            source.audit().candidate_count()
        );

        let swapped = produce_evaluated_population_statistics_v3(
            fixture.banknifty_observations(),
            fixture.banknifty_pre_admission(),
            fixture.nifty_observations(),
            fixture.nifty_pre_admission(),
            procedure,
        )
        .expect_err("swapped evaluated authorities must refuse");
        assert!(swapped.contains("NIFTY then BANKNIFTY"));

        let crosswired = produce_evaluated_population_statistics_v3(
            fixture.nifty_observations(),
            fixture.banknifty_pre_admission(),
            fixture.banknifty_observations(),
            fixture.nifty_pre_admission(),
            procedure,
        )
        .expect_err("crosswired Pre-Admission audits must refuse");
        // The production refusal names the family AND the mismatched authority:
        // "population-statistics Nifty Observation source differs from reopened
        // Pre-Admission authority". The expectation here read "binding differs",
        // which no code path emits -- so this assertion could only ever fail,
        // and it was masked by the bound refusal firing three statements earlier.
        assert!(
            crosswired.contains("Observation source differs from reopened Pre-Admission authority"),
            "crosswire must name the mismatched authority: {crosswired}"
        );

        let stale =
            crate::step3_orchestrator::statistics_v3_evaluated_pair_fixture_with_price_shift(
                10_000,
            )?;
        let stale_refusal = produce_evaluated_population_statistics_v3(
            fixture.nifty_observations(),
            stale.nifty_pre_admission(),
            fixture.banknifty_observations(),
            fixture.banknifty_pre_admission(),
            procedure,
        )
        .expect_err("foreign same-family Pre-Admission audit must refuse");
        // Same refusal text as the crosswire above, and correctly so: both are
        // "the Observation source is not the one the reopened Pre-Admission
        // authority was built from". A price-shifted fixture and a swapped
        // family are two ways to reach one condition, not two conditions.
        assert!(
            stale_refusal
                .contains("Observation source differs from reopened Pre-Admission authority"),
            "a stale audit must name the mismatched authority: {stale_refusal}"
        );
        Ok(())
    }

    #[test]
    fn admission_v4_projection_keeps_one_real_insufficient_candidate_without_a_draft()
    -> Result<(), String> {
        let root = TestDir::new()?;
        let produced = produced_fixture(
            StatisticsFamilyTerminalV3::InsufficientForCscv,
            StatisticsFamilyTerminalV3::NaturallyExtinct,
            14,
        )?;
        let commit = produced.append_and_reopen(root.path(), bounds()?)?;
        let source = produced.authenticated_admission_source(commit)?;
        let projected = source.admission_v4_candidates()?;
        assert_eq!(projected.len(), 1);
        assert_eq!(projected[0].family(), InstrumentFamilyV1::Nifty);
        assert_eq!(projected[0].family_sequence(), 0);
        assert!(projected[0].draft().is_none());
        Ok(())
    }

    #[test]
    fn receipt_last_exact_reuse_candidate_lookup_and_foreign_commit_refusal() -> Result<(), String>
    {
        let root = TestDir::new()?;
        let produced = produced_fixture(
            StatisticsFamilyTerminalV3::Evaluated,
            StatisticsFamilyTerminalV3::NaturallyExtinct,
            21,
        )?;
        let written = produced.append_and_reopen(root.path(), bounds()?)?;
        assert!(matches!(written, PopulationStatisticsV3Commit::Written(_)));
        let reused = produced.append_and_reopen(root.path(), bounds()?)?;
        assert!(matches!(reused, PopulationStatisticsV3Commit::Reused(_)));
        assert_eq!(written.audit(), reused.audit());
        let mut ledger = PopulationStatisticsV3Ledger::open_read(root.path(), bounds()?)?;
        let first = ledger.candidate(&written.audit().authority_id(), 0)?;
        assert_eq!(first.sequence(), 0);
        assert_eq!(first.family(), InstrumentFamilyV1::Nifty);
        assert!(first.wilson_lower().is_finite());
        assert!(
            ledger
                .candidate(&written.audit().authority_id(), 2)
                .is_err()
        );

        let foreign = produced_fixture(
            StatisticsFamilyTerminalV3::Evaluated,
            StatisticsFamilyTerminalV3::NaturallyExtinct,
            22,
        )?;
        let foreign_commit = foreign.append_and_reopen(root.path(), bounds()?)?;
        assert!(
            produced
                .authenticated_admission_source(foreign_commit)
                .is_err(),
            "foreign Statistics V3 commit must not mint Admission capability"
        );
        let source = produced.authenticated_admission_source(written)?;
        assert_eq!(source.audit().candidate_count(), 2);
        let projected = source.admission_v4_candidates()?;
        assert_eq!(projected.len(), 2);
        for (sequence, candidate) in projected.iter().enumerate() {
            assert_eq!(
                candidate.sequence(),
                u64::try_from(sequence)
                    .map_err(|why| format!("test Candidate sequence does not fit u64: {why}"))?
            );
            assert_eq!(candidate.family(), InstrumentFamilyV1::Nifty);
            let draft = candidate
                .draft()
                .ok_or_else(|| "evaluated Admission V4 Candidate lacks draft".to_owned())?;
            assert_eq!(
                draft.statistics_audit_id,
                source.authority_projection().authority_id()
            );
            assert_eq!(
                draft.statistics_completion_digest,
                source.authority_projection().completion_digest()
            );
        }
        Ok(())
    }

    #[test]
    fn torn_prefix_retries_exactly_and_ragged_tail_refuses() -> Result<(), String> {
        let root = TestDir::new()?;
        let torn_root = root.child("torn")?;
        let produced = produced_fixture(
            StatisticsFamilyTerminalV3::Evaluated,
            StatisticsFamilyTerminalV3::NaturallyExtinct,
            31,
        )?;
        let planned = produced.records(0, 0)?;
        let mut file = File::create(torn_root.join(DATA_FILE))
            .map_err(|why| format!("cannot create torn Statistics V3 file: {why}"))?;
        file.write_all(&header()?)
            .and_then(|()| file.write_all(&planned[0]))
            .and_then(|()| file.write_all(&planned[1]))
            .and_then(|()| file.write_all(&planned[2]))
            .and_then(|()| file.write_all(&planned[3]))
            .and_then(|()| file.sync_all())
            .map_err(|why| format!("cannot write torn Statistics V3 prefix: {why}"))?;
        drop(file);
        let completed = produced.append_and_reopen(&torn_root, bounds()?)?;
        assert!(matches!(
            completed,
            PopulationStatisticsV3Commit::Written(_)
        ));

        let ragged_root = root.child("ragged")?;
        File::create(ragged_root.join(LOCK_FILE))
            .map_err(|why| format!("cannot create ragged lock file: {why}"))?;
        let mut ragged = File::create(ragged_root.join(DATA_FILE))
            .map_err(|why| format!("cannot create ragged data file: {why}"))?;
        ragged
            .write_all(&header()?)
            .and_then(|()| ragged.write_all(&[1]))
            .and_then(|()| ragged.sync_all())
            .map_err(|why| format!("cannot write ragged Statistics V3 file: {why}"))?;
        assert!(PopulationStatisticsV3Ledger::open_read(&ragged_root, bounds()?).is_err());
        Ok(())
    }

    #[test]
    fn corrupt_and_resealed_candidate_bytes_refuse() -> Result<(), String> {
        let root = TestDir::new()?;
        let produced = produced_fixture(
            StatisticsFamilyTerminalV3::Evaluated,
            StatisticsFamilyTerminalV3::NaturallyExtinct,
            41,
        )?;

        let corrupt_root = root.child("corrupt")?;
        produced.append_and_reopen(&corrupt_root, bounds()?)?;
        let corrupt_path = corrupt_root.join(DATA_FILE);
        let mut corrupt = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&corrupt_path)
            .map_err(|why| format!("cannot open corrupt fixture: {why}"))?;
        corrupt
            .seek(SeekFrom::Start(record_offset(3)? + 144))
            .and_then(|_| corrupt.write_all(&[0xff]))
            .and_then(|()| corrupt.sync_all())
            .map_err(|why| format!("cannot mutate corrupt fixture: {why}"))?;
        assert!(PopulationStatisticsV3Ledger::open_read(&corrupt_root, bounds()?).is_err());

        let resealed_root = root.child("resealed")?;
        produced.append_and_reopen(&resealed_root, bounds()?)?;
        let resealed_path = resealed_root.join(DATA_FILE);
        let mut resealed = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&resealed_path)
            .map_err(|why| format!("cannot open resealed fixture: {why}"))?;
        let mut raw = read_raw(&mut resealed, 3)?;
        raw[144..152].copy_from_slice(&4_u64.to_le_bytes());
        let payload: &[u8; PAYLOAD_BYTES] = raw[..PAYLOAD_BYTES]
            .try_into()
            .map_err(|_| "resealed fixture payload width differs".to_owned())?;
        let seal = digest_domain(RECORD_DOMAIN, payload);
        raw[PAYLOAD_BYTES..].copy_from_slice(&seal);
        resealed
            .seek(SeekFrom::Start(record_offset(3)?))
            .and_then(|_| resealed.write_all(&raw))
            .and_then(|()| resealed.sync_all())
            .map_err(|why| format!("cannot write resealed fixture: {why}"))?;
        assert!(PopulationStatisticsV3Ledger::open_read(&resealed_root, bounds()?).is_err());
        Ok(())
    }

    #[test]
    fn stale_handle_and_named_path_replacement_fail_closed() -> Result<(), String> {
        let root = TestDir::new()?;
        let produced = produced_fixture(
            StatisticsFamilyTerminalV3::Evaluated,
            StatisticsFamilyTerminalV3::NaturallyExtinct,
            51,
        )?;
        let stale_root = root.child("stale")?;
        let commit = produced.append_and_reopen(&stale_root, bounds()?)?;
        let mut stale = PopulationStatisticsV3Ledger::open_read(&stale_root, bounds()?)?;
        let mut mutator = OpenOptions::new()
            .write(true)
            .open(stale_root.join(DATA_FILE))
            .map_err(|why| format!("cannot open stale fixture mutator: {why}"))?;
        mutator
            .seek(SeekFrom::Start(record_offset(3)? + 144))
            .and_then(|_| mutator.write_all(&[0xfe]))
            .and_then(|()| mutator.sync_all())
            .map_err(|why| format!("cannot mutate stale fixture: {why}"))?;
        assert!(stale.reopen_audit(&commit.audit().authority_id()).is_err());

        #[cfg(unix)]
        {
            let replacement_root = root.child("replacement")?;
            let replacement_commit = produced.append_and_reopen(&replacement_root, bounds()?)?;
            let mut cached = PopulationStatisticsV3Ledger::open_read(&replacement_root, bounds()?)?;
            let named = replacement_root.join(DATA_FILE);
            let bytes = std::fs::read(&named)
                .map_err(|why| format!("cannot read replacement fixture: {why}"))?;
            std::fs::rename(&named, replacement_root.join("displaced.bin"))
                .map_err(|why| format!("cannot displace Statistics V3 path: {why}"))?;
            let mut replacement = File::create(&named)
                .map_err(|why| format!("cannot create replacement path: {why}"))?;
            replacement
                .write_all(&bytes)
                .and_then(|()| replacement.sync_all())
                .map_err(|why| format!("cannot write replacement path: {why}"))?;
            assert!(
                cached
                    .reopen_audit(&replacement_commit.audit().authority_id())
                    .is_err()
            );
        }
        Ok(())
    }
}
