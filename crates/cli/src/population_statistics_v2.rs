//! Selection-independent Population Statistics V2 ledger.
//!
//! D-0464 requires one complete statistical family for exactly one signal
//! rung: every NIFTY candidate first, then every BANKNIFTY candidate.  The
//! family exists before ranking and admission.  It retains aligned period
//! returns, complete CSCV split scores, exact Wilson sources, genuine
//! split-derived PBO, White, SPA and candidate-specific Romano--Wolf evidence.
//!
//! The public writer accepts only an opaque prepared capability, syncs the raw
//! block before its Completion, drops the writable handle and freshly reopens
//! the exact audit read-only.  The production constructor accepts only a sealed
//! paired Observation V1 capability, its exact freshly reopened durable audit,
//! and the two exact Pre-Admission reopen audits.  Caller-authored period rows,
//! split scores and digests remain unreachable.
//!
//! One family is a fixed-record block: `Data`, all candidate summaries, all
//! period rows in period-major order, all CSCV scores in split-major order, and
//! `Completion` last.  Data records are synced before Completion.  A valid
//! trailing prefix is recoverable only by a byte-identical retry; corruption,
//! a ragged/torn record, foreign retry, stale handle or bound breach refuses.
//!
//! Opening and generation checking scan bounded file bytes.  Recomputing the
//! statistics is input-dependent and includes the bootstrap costs recorded in
//! `docs/06-limits.md` §147.  Only fixed-record offset arithmetic is worst-case
//! O(1) in record count.  Hash-map lookup is average O(1), and file hashing,
//! locks, allocation, bootstrap work and `sync_all` are not constant-time.

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
use runner::bootstrap::{
    RomanoWolfAdjustedReceiptV1, SpaReceiptV1, WhiteRealityCheckReceiptV1,
    romano_wolf_adjusted_p_values_v1, spa_receipt_v1, white_reality_check_receipt_v1,
};

use crate::candidate_universe::CANDIDATE_SIGNAL_RUNGS_SECONDS_V1;
use crate::population::{InstrumentFamilyV1, RequestedSpanIdentityV1};
use crate::population_observations_v1::{
    CandidateFamilyObservationsV1, ObservationAuthorityAuditV1, PairedCandidateObservationsV1,
};
use crate::pre_admission_data::{PreAdmissionDataReopenAuditV1, PreAdmissionDataV1};

/// Operator-facing refusal from the Population Statistics V2 audit boundary.
pub type PopulationStatisticsV2Refusal = String;

/// Header bytes in the Population Statistics V2 audit file.
pub const POPULATION_STATISTICS_V2_HEADER_BYTES: u64 = 64;
/// Bytes in every fixed Population Statistics V2 record.
pub const POPULATION_STATISTICS_V2_RECORD_STRIDE: u64 = 1_024;
/// Largest candidate page one call may allocate.
pub const MAX_POPULATION_STATISTICS_V2_PAGE_ROWS: u64 = 256;

const HEADER_BYTES: usize = 64;
const PAYLOAD_BYTES: usize = 992;
const RECORD_BYTES: usize = 1_024;
const SEAL_BYTES: usize = 32;
const HEADER_VERSION: u32 = 2;
const RECORD_VERSION: u32 = 2;
const HEADER_KIND: u32 = 1;
const DATA_KIND: u32 = 1;
const CANDIDATE_KIND: u32 = 2;
const PERIOD_KIND: u32 = 3;
const SPLIT_KIND: u32 = 4;
const COMPLETION_KIND: u32 = 5;
const HEADER_MAGIC: [u8; 16] = *b"BTX-POPSTATS-V2\0";
const DATA_FILE: &str = "population-statistics-v2-audit.bin";
const LOCK_FILE: &str = "population-statistics-v2-audit.lock";
const HEADER_DOMAIN: &[u8] = b"brutex-population-statistics-v2-header\0";
const RECORD_DOMAIN: &[u8] = b"brutex-population-statistics-v2-record\0";
const COMPLETION_RECORD_DIGEST_DOMAIN: &[u8] =
    b"brutex-population-statistics-v2-completion-record-digest-v1\0";
const AUDIT_DOMAIN: &[u8] = b"brutex-population-statistics-v2-audit-id\0";
const GENERATION_DOMAIN: &[u8] = b"brutex-population-statistics-v2-generation\0";
const CANDIDATE_ORDER_DOMAIN: &[u8] = b"brutex-population-statistics-v2-candidates\0";
const PERIOD_ORDER_DOMAIN: &[u8] = b"brutex-population-statistics-v2-periods\0";
const SPLIT_ORDER_DOMAIN: &[u8] = b"brutex-population-statistics-v2-splits\0";
const WILSON_POLICY_DOMAIN: &[u8] = b"brutex-wilson-95-lower-v1\0";
const CSCV_POLICY_DOMAIN: &[u8] = b"brutex-cscv-complementary-half-splits-v2\0";
const SPLIT_ID_DOMAIN: &[u8] = b"brutex-population-statistics-v2-split-id\0";
const OBSERVATION_PERIOD_LINK_DOMAIN: &[u8] =
    b"brutex-population-statistics-v2-observation-period-link-v1\0";
const OBSERVATION_PROJECTION_POLICY_DOMAIN: &[u8] =
    b"brutex-population-statistics-v2-observation-projection-policy-v1\0";
const OBSERVATION_STATISTICS_LINK_DOMAIN: &[u8] =
    b"brutex-population-statistics-v2-observation-link-v1\0";
const STATISTICS_V3_SINGLE_CANDIDATE_ORDER_DOMAIN: &[u8] =
    b"brutex-population-statistics-v3-single-family-candidates\0";
const STATISTICS_V3_SINGLE_PERIOD_ORDER_DOMAIN: &[u8] =
    b"brutex-population-statistics-v3-single-family-periods\0";
const STATISTICS_V3_SINGLE_SPLIT_ORDER_DOMAIN: &[u8] =
    b"brutex-population-statistics-v3-single-family-splits\0";
const READ_CHUNK_BYTES: usize = 16 * 1_024;
const LOCK_FILE_MAX_BYTES: u64 = 0;

#[cfg(any(target_os = "android", target_os = "linux"))]
const O_NOFOLLOW_FLAG: i32 = 0x20_000;
#[cfg(target_os = "macos")]
const O_NOFOLLOW_FLAG: i32 = 0x100;

const _: () = assert!(PAYLOAD_BYTES + SEAL_BYTES == RECORD_BYTES);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RecordKindV2 {
    Data,
    Candidate,
    Period,
    Split,
    Completion,
}

impl RecordKindV2 {
    const fn byte(self) -> u32 {
        match self {
            Self::Data => DATA_KIND,
            Self::Candidate => CANDIDATE_KIND,
            Self::Period => PERIOD_KIND,
            Self::Split => SPLIT_KIND,
            Self::Completion => COMPLETION_KIND,
        }
    }

    fn from_byte(value: u32) -> Result<Self, PopulationStatisticsV2Refusal> {
        match value {
            DATA_KIND => Ok(Self::Data),
            CANDIDATE_KIND => Ok(Self::Candidate),
            PERIOD_KIND => Ok(Self::Period),
            SPLIT_KIND => Ok(Self::Split),
            COMPLETION_KIND => Ok(Self::Completion),
            _ => Err(format!(
                "population-statistics record kind {value} is unknown"
            )),
        }
    }
}

/// Explicit deterministic bootstrap procedure for one complete Statistics V2 family.
///
/// Segment and split shape are deliberately absent: they come only from the
/// sealed Observation V1 layout.  There is no `Default`; callers must name the
/// finite resample count, deterministic seed and stationary-bootstrap block
/// length for every production preparation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationStatisticsProcedureV2 {
    draws: u64,
    seed: u64,
    block_length: u64,
}

/// Crate-private, fully measured candidate projection used only by the
/// version-separated mixed-family Statistics V3 authority.
///
/// There is deliberately no public constructor.  Every field is recomputed
/// from one opaque Observation V1 family below; callers cannot author a
/// statistic, period digest or split digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SingleFamilyCandidateStatisticsV3 {
    family_sequence: u64,
    candidate_semantic_digest: [u8; 32],
    trades: u64,
    wins: u64,
    wilson_lower_bits: u64,
    romano_wolf_statistic_bits: u64,
    romano_wolf_rank: u64,
    romano_wolf_strict_exceedances: u64,
    romano_wolf_initial: PopulationStatisticsFractionV2,
    romano_wolf_adjusted: PopulationStatisticsFractionV2,
    ordered_period_digest: [u8; 32],
    ordered_split_digest: [u8; 32],
}

impl SingleFamilyCandidateStatisticsV3 {
    pub(crate) const fn family_sequence(self) -> u64 {
        self.family_sequence
    }

    pub(crate) const fn candidate_semantic_digest(self) -> [u8; 32] {
        self.candidate_semantic_digest
    }

    pub(crate) const fn trades(self) -> u64 {
        self.trades
    }

    pub(crate) const fn wins(self) -> u64 {
        self.wins
    }

    pub(crate) const fn wilson_lower_bits(self) -> u64 {
        self.wilson_lower_bits
    }

    pub(crate) const fn romano_wolf_statistic_bits(self) -> u64 {
        self.romano_wolf_statistic_bits
    }

    pub(crate) const fn romano_wolf_rank(self) -> u64 {
        self.romano_wolf_rank
    }

    pub(crate) const fn romano_wolf_strict_exceedances(self) -> u64 {
        self.romano_wolf_strict_exceedances
    }

    pub(crate) const fn romano_wolf_initial(self) -> PopulationStatisticsFractionV2 {
        self.romano_wolf_initial
    }

    pub(crate) const fn romano_wolf_adjusted(self) -> PopulationStatisticsFractionV2 {
        self.romano_wolf_adjusted
    }

    pub(crate) const fn ordered_period_digest(self) -> [u8; 32] {
        self.ordered_period_digest
    }

    pub(crate) const fn ordered_split_digest(self) -> [u8; 32] {
        self.ordered_split_digest
    }
}

/// Opaque single-family statistical evidence for the mixed-family V3 writer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SingleFamilyStatisticsV3Projection {
    segment_count: u32,
    period_count: u64,
    split_count: u64,
    layout_digest: [u8; 32],
    white: PopulationFamilyTestV2,
    spa: PopulationFamilyTestV2,
    romano_wolf_family_digest: [u8; 32],
    pbo: Option<PopulationCscvPboV2>,
    ordered_candidate_digest: [u8; 32],
    ordered_period_digest: [u8; 32],
    ordered_split_digest: [u8; 32],
    candidates: Vec<SingleFamilyCandidateStatisticsV3>,
}

impl SingleFamilyStatisticsV3Projection {
    pub(crate) const fn segment_count(&self) -> u32 {
        self.segment_count
    }

    pub(crate) const fn period_count(&self) -> u64 {
        self.period_count
    }

    pub(crate) const fn split_count(&self) -> u64 {
        self.split_count
    }

    pub(crate) const fn layout_digest(&self) -> [u8; 32] {
        self.layout_digest
    }

    pub(crate) const fn white(&self) -> PopulationFamilyTestV2 {
        self.white
    }

    pub(crate) const fn spa(&self) -> PopulationFamilyTestV2 {
        self.spa
    }

    pub(crate) const fn romano_wolf_family_digest(&self) -> [u8; 32] {
        self.romano_wolf_family_digest
    }

    pub(crate) const fn pbo(&self) -> Option<PopulationCscvPboV2> {
        self.pbo
    }

    pub(crate) const fn ordered_candidate_digest(&self) -> [u8; 32] {
        self.ordered_candidate_digest
    }

    pub(crate) const fn ordered_period_digest(&self) -> [u8; 32] {
        self.ordered_period_digest
    }

    pub(crate) const fn ordered_split_digest(&self) -> [u8; 32] {
        self.ordered_split_digest
    }

    pub(crate) fn candidates(&self) -> &[SingleFamilyCandidateStatisticsV3] {
        &self.candidates
    }
}

impl PopulationStatisticsProcedureV2 {
    /// Constructs one explicit nonzero finite bootstrap procedure.
    ///
    /// # Errors
    ///
    /// Refuses zero draws or a zero block length.  A seed of zero is a valid,
    /// explicit deterministic seed and is not replaced by a default.
    pub fn new(
        draws: u64,
        seed: u64,
        block_length: u64,
    ) -> Result<Self, PopulationStatisticsV2Refusal> {
        if draws == 0 || block_length == 0 {
            return Err(
                "population-statistics production draws and block length must be nonzero"
                    .to_owned(),
            );
        }
        Ok(Self {
            draws,
            seed,
            block_length,
        })
    }

    /// Exact finite resample count.
    #[must_use]
    pub const fn draws(self) -> u64 {
        self.draws
    }

    /// Explicit deterministic bootstrap seed.
    #[must_use]
    pub const fn seed(self) -> u64 {
        self.seed
    }

    /// Stationary-bootstrap block length in aligned observation periods.
    #[must_use]
    pub const fn block_length(self) -> u64 {
        self.block_length
    }
}

/// Opaque identity link from durable Observation V1 to Statistics V2.
///
/// This link is intentionally not encoded into the existing Statistics V2
/// manifest or rows.  It therefore preserves the V2 byte format while giving
/// the later Population-Finalization authority one typed value that binds the
/// exact observation Completion to the freshly reopened statistics audit.
/// No public constructor can mint or alter it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationStatisticsObservationLinkV2 {
    observation_authority_id: [u8; 32],
    observation_pair_identity: [u8; 32],
    statistics_audit_id: [u8; 32],
    projection_policy_digest: [u8; 32],
    candidate_count: u64,
    period_count: u64,
    split_count: u64,
    identity: [u8; 32],
}

impl PopulationStatisticsObservationLinkV2 {
    /// Freshly reopened Observation V1 authority consumed by the projection.
    #[must_use]
    pub const fn observation_authority_id(self) -> [u8; 32] {
        self.observation_authority_id
    }

    /// Complete sealed in-memory Observation V1 pair consumed by the projection.
    #[must_use]
    pub const fn observation_pair_identity(self) -> [u8; 32] {
        self.observation_pair_identity
    }

    /// Exact Statistics V2 audit identity prepared from those observations.
    #[must_use]
    pub const fn statistics_audit_id(self) -> [u8; 32] {
        self.statistics_audit_id
    }

    /// Versioned NIFTY-first period/split projection policy.
    #[must_use]
    pub const fn projection_policy_digest(self) -> [u8; 32] {
        self.projection_policy_digest
    }

    /// Domain-separated identity binding both authorities and all projection terms.
    #[must_use]
    pub const fn identity(self) -> [u8; 32] {
        self.identity
    }

    /// Requires one freshly reopened Statistics V2 audit to be the exact linked result.
    ///
    /// # Errors
    ///
    /// Refuses a foreign audit identity or any candidate/period/split cardinality
    /// that differs from the durable Observation V1 authority.
    pub fn require_reopened_statistics(
        self,
        audit: PopulationStatisticsV2ReopenAudit,
    ) -> Result<(), PopulationStatisticsV2Refusal> {
        if audit.audit_id() != self.statistics_audit_id
            || audit.candidate_count() != self.candidate_count
            || audit.period_count() != self.period_count
            || audit.split_count() != self.split_count
        {
            return Err(
                "population-statistics observation link names a foreign reopened statistics audit"
                    .to_owned(),
            );
        }
        Ok(())
    }
}

/// Exact rational probability retained without a rounded storage substitute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationStatisticsFractionV2 {
    numerator: u64,
    denominator: u64,
}

impl PopulationStatisticsFractionV2 {
    /// Exact numerator.
    #[must_use]
    pub const fn numerator(self) -> u64 {
        self.numerator
    }

    /// Exact nonzero denominator.
    #[must_use]
    pub const fn denominator(self) -> u64 {
        self.denominator
    }

    fn new(numerator: u64, denominator: u64) -> Result<Self, PopulationStatisticsV2Refusal> {
        let value = Self {
            numerator,
            denominator,
        };
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<(), PopulationStatisticsV2Refusal> {
        if self.denominator == 0 || self.numerator > self.denominator {
            return Err(format!(
                "population-statistics fraction {}/{} is outside [0,1]",
                self.numerator, self.denominator
            ));
        }
        Ok(())
    }

    #[expect(
        clippy::cast_precision_loss,
        clippy::float_arithmetic,
        reason = "this is an exact-count statistical projection, never a price"
    )]
    fn bits(self) -> u64 {
        (self.numerator as f64 / self.denominator as f64).to_bits()
    }
}

/// Exact family-level White or SPA evidence copied from recomputed raw periods.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationFamilyTestV2 {
    statistic_bits: u64,
    probability_bits: u64,
    probability: PopulationStatisticsFractionV2,
    family_digest: [u8; 32],
}

impl PopulationFamilyTestV2 {
    /// Full-precision observed statistic.
    #[must_use]
    pub const fn statistic(self) -> f64 {
        f64::from_bits(self.statistic_bits)
    }

    /// Full-precision finite-resample probability.
    #[must_use]
    pub const fn probability(self) -> f64 {
        f64::from_bits(self.probability_bits)
    }

    /// Exact finite-resample probability.
    #[must_use]
    pub const fn exact_probability(self) -> PopulationStatisticsFractionV2 {
        self.probability
    }

    /// Ordered return-family and procedure digest.
    #[must_use]
    pub const fn family_digest(self) -> [u8; 32] {
        self.family_digest
    }

    fn validate(self, name: &str) -> Result<(), PopulationStatisticsV2Refusal> {
        self.probability.validate()?;
        require_finite(name, self.statistic_bits)?;
        require_finite(&format!("{name} probability"), self.probability_bits)?;
        if self.probability.bits() != self.probability_bits {
            return Err(format!(
                "population-statistics {name} probability bits differ from exact counts"
            ));
        }
        require_nonzero(&format!("{name} family"), self.family_digest)
    }
}

/// Exact genuine-CSCV family PBO evidence derived from complementary splits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationCscvPboV2 {
    contributing_splits: u64,
    bottom_half_splits: u64,
    unrankable_splits: u64,
    probability_bits: u64,
    probability: PopulationStatisticsFractionV2,
}

impl PopulationCscvPboV2 {
    /// Rankable complementary splits.
    #[must_use]
    pub const fn contributing_splits(self) -> u64 {
        self.contributing_splits
    }

    /// Rankable splits whose in-sample winner landed below the OOS midpoint.
    #[must_use]
    pub const fn bottom_half_splits(self) -> u64 {
        self.bottom_half_splits
    }

    /// Splits that could not produce a relative placement.
    #[must_use]
    pub const fn unrankable_splits(self) -> u64 {
        self.unrankable_splits
    }

    /// Exact bottom-half fraction.
    #[must_use]
    pub const fn exact_probability(self) -> PopulationStatisticsFractionV2 {
        self.probability
    }

    /// Full-precision projection of the exact bottom-half fraction.
    #[must_use]
    pub const fn probability(self) -> f64 {
        f64::from_bits(self.probability_bits)
    }

    fn validate(self, split_count: u64) -> Result<(), PopulationStatisticsV2Refusal> {
        self.probability.validate()?;
        if self.contributing_splits > split_count
            || self.unrankable_splits > split_count
            || self.contributing_splits.checked_add(self.unrankable_splits) != Some(split_count)
            || self.bottom_half_splits > self.contributing_splits
            || self.probability.numerator != self.bottom_half_splits
            || self.probability.denominator != self.contributing_splits
            || self.probability.bits() != self.probability_bits
        {
            return Err("population-statistics CSCV/PBO counts disagree".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PreAdmissionBindingV2 {
    family: InstrumentFamilyV1,
    sequence: u64,
    authority_id: [u8; 32],
    candidate_universe_id: [u8; 32],
    candidate_completion_digest: [u8; 32],
    candidate_count: u64,
}

impl PreAdmissionBindingV2 {
    fn from_value(value: &PreAdmissionDataV1) -> Self {
        Self {
            family: value.family(),
            sequence: value.sequence(),
            authority_id: value.authority_id(),
            candidate_universe_id: value.candidate_universe_id(),
            candidate_completion_digest: value.candidate_completion_digest(),
            candidate_count: value.candidate_row_count(),
        }
    }

    fn validate(self, expected: InstrumentFamilyV1) -> Result<(), PopulationStatisticsV2Refusal> {
        if self.family != expected {
            return Err(format!(
                "population-statistics pre-admission family {:?} is not expected {:?}",
                self.family, expected
            ));
        }
        for (name, digest) in [
            ("pre-admission authority", self.authority_id),
            ("candidate universe", self.candidate_universe_id),
            ("candidate completion", self.candidate_completion_digest),
        ] {
            require_nonzero(name, digest)?;
        }
        if self.candidate_count == 0 {
            return Err("population-statistics family has zero candidates".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PopulationStatisticsManifestV2 {
    sequence: u64,
    audit_id: [u8; 32],
    rung_seconds: u32,
    horizon_bars: u32,
    requested_span: RequestedSpanIdentityV1,
    segment_count: u32,
    period_count: u64,
    split_count: u64,
    draws: u64,
    seed: u64,
    block_length: u64,
    nifty: PreAdmissionBindingV2,
    banknifty: PreAdmissionBindingV2,
    feed_digest: [u8; 32],
    source_commit_digest: [u8; 32],
    calendar_policy_digest: [u8; 32],
    daily_reference_policy_digest: [u8; 32],
    wilson_policy_digest: [u8; 32],
    cscv_policy_digest: [u8; 32],
    white: PopulationFamilyTestV2,
    spa: PopulationFamilyTestV2,
    romano_wolf_family_digest: [u8; 32],
    pbo: PopulationCscvPboV2,
    ordered_candidate_digest: [u8; 32],
    ordered_period_digest: [u8; 32],
    ordered_split_digest: [u8; 32],
}

impl PopulationStatisticsManifestV2 {
    fn candidate_count(self) -> Result<u64, PopulationStatisticsV2Refusal> {
        self.nifty
            .candidate_count
            .checked_add(self.banknifty.candidate_count)
            .ok_or_else(|| "population-statistics candidate count overflowed u64".to_owned())
    }

    fn block_record_count(self) -> Result<u64, PopulationStatisticsV2Refusal> {
        let candidates = self.candidate_count()?;
        let periods = candidates
            .checked_mul(self.period_count)
            .ok_or_else(|| "population-statistics period record count overflowed".to_owned())?;
        let splits = candidates
            .checked_mul(self.split_count)
            .ok_or_else(|| "population-statistics split record count overflowed".to_owned())?;
        2_u64
            .checked_add(candidates)
            .and_then(|count| count.checked_add(periods))
            .and_then(|count| count.checked_add(splits))
            .ok_or_else(|| "population-statistics block record count overflowed".to_owned())
    }

    fn with_sequence(mut self, sequence: u64) -> Self {
        self.sequence = sequence;
        self
    }

    fn validate(self) -> Result<(), PopulationStatisticsV2Refusal> {
        self.validate_with_rung(require_new_production_rung)
    }

    fn validate_stored_compatible(self) -> Result<(), PopulationStatisticsV2Refusal> {
        self.validate_with_rung(require_stored_compatible_rung)
    }

    fn validate_with_rung(
        self,
        require_signal_rung: fn(u32) -> Result<(), PopulationStatisticsV2Refusal>,
    ) -> Result<(), PopulationStatisticsV2Refusal> {
        self.nifty.validate(InstrumentFamilyV1::Nifty)?;
        self.banknifty.validate(InstrumentFamilyV1::BankNifty)?;
        require_signal_rung(self.rung_seconds)?;
        if self.horizon_bars == 0
            || self.period_count < 2
            || self.draws == 0
            || self.block_length == 0
        {
            return Err(
                "population-statistics horizon, >=2 periods, draws and block length are required"
                    .to_owned(),
            );
        }
        RequestedSpanIdentityV1::new(
            self.requested_span.from_year(),
            self.requested_span.from_month(),
            self.requested_span.to_year(),
            self.requested_span.to_month(),
        )?;
        for (name, digest) in [
            ("audit", self.audit_id),
            ("feed", self.feed_digest),
            ("source commit", self.source_commit_digest),
            ("calendar policy", self.calendar_policy_digest),
            ("daily-reference policy", self.daily_reference_policy_digest),
            ("Wilson policy", self.wilson_policy_digest),
            ("CSCV policy", self.cscv_policy_digest),
            ("Romano-Wolf family", self.romano_wolf_family_digest),
            ("ordered candidates", self.ordered_candidate_digest),
            ("ordered periods", self.ordered_period_digest),
            ("ordered splits", self.ordered_split_digest),
        ] {
            require_nonzero(name, digest)?;
        }
        if self.wilson_policy_digest != digest_domain(WILSON_POLICY_DOMAIN, &[])
            || self.cscv_policy_digest != cscv_policy_digest(self.segment_count)?
            || self.split_count != canonical_split_count(self.segment_count)?
        {
            return Err("population-statistics policy or CSCV split count differs".to_owned());
        }
        self.white.validate("White")?;
        self.spa.validate("SPA")?;
        self.pbo.validate(self.split_count)?;
        self.block_record_count()?;
        if derive_audit_id(self)? != self.audit_id {
            return Err("population-statistics audit identity differs from fields".to_owned());
        }
        Ok(())
    }
}

/// Recomputed candidate-level statistics and exact source digests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationStatisticsCandidateV2 {
    audit_id: [u8; 32],
    sequence: u64,
    family: InstrumentFamilyV1,
    family_sequence: u64,
    candidate_semantic_digest: [u8; 32],
    pre_admission_id: [u8; 32],
    trades: u64,
    wins: u64,
    wilson_lower_bits: u64,
    romano_wolf_statistic_bits: u64,
    romano_wolf_rank: u64,
    romano_wolf_strict_exceedances: u64,
    romano_wolf_initial: PopulationStatisticsFractionV2,
    romano_wolf_adjusted: PopulationStatisticsFractionV2,
    ordered_period_digest: [u8; 32],
    ordered_split_digest: [u8; 32],
}

impl PopulationStatisticsCandidateV2 {
    /// Zero-based position in the paired NIFTY-then-BANKNIFTY family.
    #[must_use]
    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    /// NIFTY or BANKNIFTY.
    #[must_use]
    pub const fn family(self) -> InstrumentFamilyV1 {
        self.family
    }

    /// Zero-based position inside its instrument family.
    #[must_use]
    pub const fn family_sequence(self) -> u64 {
        self.family_sequence
    }

    /// Candidate identity independent of a future final Population-ID.
    #[must_use]
    pub const fn candidate_semantic_digest(self) -> [u8; 32] {
        self.candidate_semantic_digest
    }

    /// Exact trade denominator used by Wilson.
    #[must_use]
    pub const fn trades(self) -> u64 {
        self.trades
    }

    /// Exact winning-trade numerator used by Wilson.
    #[must_use]
    pub const fn wins(self) -> u64 {
        self.wins
    }

    /// Full-precision 95% Wilson lower bound.
    #[must_use]
    pub const fn wilson_lower(self) -> f64 {
        f64::from_bits(self.wilson_lower_bits)
    }

    /// Full-precision observed Romano--Wolf statistic.
    #[must_use]
    pub const fn romano_wolf_statistic(self) -> f64 {
        f64::from_bits(self.romano_wolf_statistic_bits)
    }

    /// Canonical descending-statistic Romano--Wolf rank.
    #[must_use]
    pub const fn romano_wolf_rank(self) -> u64 {
        self.romano_wolf_rank
    }

    /// Exact initial Romano--Wolf probability.
    #[must_use]
    pub const fn romano_wolf_initial(self) -> PopulationStatisticsFractionV2 {
        self.romano_wolf_initial
    }

    /// Exact monotone-adjusted Romano--Wolf probability.
    #[must_use]
    pub const fn romano_wolf_adjusted(self) -> PopulationStatisticsFractionV2 {
        self.romano_wolf_adjusted
    }

    fn validate(
        self,
        manifest: &PopulationStatisticsManifestV2,
    ) -> Result<(), PopulationStatisticsV2Refusal> {
        if self.audit_id != manifest.audit_id {
            return Err("population-statistics candidate names another audit".to_owned());
        }
        let nifty_count = manifest.nifty.candidate_count;
        let expected = if self.sequence < nifty_count {
            (
                InstrumentFamilyV1::Nifty,
                self.sequence,
                manifest.nifty.authority_id,
            )
        } else {
            (
                InstrumentFamilyV1::BankNifty,
                self.sequence.checked_sub(nifty_count).ok_or_else(|| {
                    "population-statistics family sequence underflowed".to_owned()
                })?,
                manifest.banknifty.authority_id,
            )
        };
        if self.sequence >= manifest.candidate_count()?
            || (self.family, self.family_sequence, self.pre_admission_id) != expected
        {
            return Err(
                "population-statistics candidate order or pre-admission binding differs".to_owned(),
            );
        }
        require_nonzero("candidate semantic", self.candidate_semantic_digest)?;
        require_nonzero("candidate period digest", self.ordered_period_digest)?;
        require_nonzero("candidate split digest", self.ordered_split_digest)?;
        if self.wins > self.trades
            || wilson_lower_bits(self.wins, self.trades) != self.wilson_lower_bits
        {
            return Err("population-statistics Wilson source or result differs".to_owned());
        }
        require_finite("Romano-Wolf statistic", self.romano_wolf_statistic_bits)?;
        self.romano_wolf_initial.validate()?;
        self.romano_wolf_adjusted.validate()?;
        let resample_denominator = manifest
            .draws
            .checked_add(1)
            .ok_or_else(|| "population-statistics bootstrap denominator overflowed".to_owned())?;
        let initial_from_exceedances = self
            .romano_wolf_strict_exceedances
            .checked_add(1)
            .ok_or_else(|| "Romano-Wolf exceedance numerator overflowed".to_owned())?;
        if self.romano_wolf_initial.denominator != resample_denominator
            || self.romano_wolf_adjusted.denominator != resample_denominator
            || self.romano_wolf_adjusted.numerator < self.romano_wolf_initial.numerator
            || initial_from_exceedances != self.romano_wolf_initial.numerator
            || self.romano_wolf_rank >= manifest.candidate_count()?
        {
            return Err("population-statistics Romano-Wolf candidate fields differ".to_owned());
        }
        Ok(())
    }
}

/// One raw aligned period source retained before selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationStatisticsPeriodSourceV2 {
    audit_id: [u8; 32],
    candidate_sequence: u64,
    period_sequence: u64,
    candidate_semantic_digest: [u8; 32],
    return_paisa: i64,
    trades: u64,
    wins: u64,
    period_identity: [u8; 32],
}

impl PopulationStatisticsPeriodSourceV2 {
    /// Candidate position in the paired family.
    #[must_use]
    pub const fn candidate_sequence(self) -> u64 {
        self.candidate_sequence
    }

    /// Aligned period ordinal.
    #[must_use]
    pub const fn period_sequence(self) -> u64 {
        self.period_sequence
    }

    /// Integer-paisa return measured for this period.
    #[must_use]
    pub const fn return_paisa(self) -> i64 {
        self.return_paisa
    }

    /// Trades contributing to the period.
    #[must_use]
    pub const fn trades(self) -> u64 {
        self.trades
    }

    /// Wins contributing to the period.
    #[must_use]
    pub const fn wins(self) -> u64 {
        self.wins
    }

    /// Identity of the shared aligned time period.
    #[must_use]
    pub const fn period_identity(self) -> [u8; 32] {
        self.period_identity
    }

    fn validate(
        self,
        manifest: &PopulationStatisticsManifestV2,
        candidates: &[PopulationStatisticsCandidateV2],
    ) -> Result<(), PopulationStatisticsV2Refusal> {
        if self.audit_id != manifest.audit_id
            || self.period_sequence >= manifest.period_count
            || self.candidate_sequence >= manifest.candidate_count()?
            || self.wins > self.trades
        {
            return Err("population-statistics period coordinates or counts differ".to_owned());
        }
        let candidate = candidates
            .get(usize_of(self.candidate_sequence, "candidate sequence")?)
            .ok_or_else(|| "population-statistics period candidate is absent".to_owned())?;
        if candidate.candidate_semantic_digest != self.candidate_semantic_digest {
            return Err("population-statistics period moved to another candidate".to_owned());
        }
        require_nonzero("period identity", self.period_identity)
    }
}

/// One raw candidate score on one complementary CSCV split.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationStatisticsSplitSourceV2 {
    audit_id: [u8; 32],
    candidate_sequence: u64,
    split_sequence: u64,
    candidate_semantic_digest: [u8; 32],
    segment_count: u32,
    train_mask: u64,
    test_mask: u64,
    train_score: i64,
    test_score: i64,
    split_identity: [u8; 32],
}

impl PopulationStatisticsSplitSourceV2 {
    /// Candidate position in the paired family.
    #[must_use]
    pub const fn candidate_sequence(self) -> u64 {
        self.candidate_sequence
    }

    /// Canonical complementary split ordinal.
    #[must_use]
    pub const fn split_sequence(self) -> u64 {
        self.split_sequence
    }

    /// In-sample score used to choose the first strict maximum.
    #[must_use]
    pub const fn train_score(self) -> i64 {
        self.train_score
    }

    /// Complementary out-of-sample score used for the exact midrank.
    #[must_use]
    pub const fn test_score(self) -> i64 {
        self.test_score
    }

    /// Bit mask of in-sample segments; bit zero is canonically absent.
    #[must_use]
    pub const fn train_mask(self) -> u64 {
        self.train_mask
    }

    /// Exact complementary out-of-sample segment mask.
    #[must_use]
    pub const fn test_mask(self) -> u64 {
        self.test_mask
    }

    fn validate(
        self,
        manifest: &PopulationStatisticsManifestV2,
        candidates: &[PopulationStatisticsCandidateV2],
        expected_train_mask: u64,
    ) -> Result<(), PopulationStatisticsV2Refusal> {
        let full = segment_mask(manifest.segment_count)?;
        if self.audit_id != manifest.audit_id
            || self.split_sequence >= manifest.split_count
            || self.candidate_sequence >= manifest.candidate_count()?
            || self.segment_count != manifest.segment_count
            || self.train_mask != expected_train_mask
            || self.test_mask != full ^ expected_train_mask
            || self.train_mask & self.test_mask != 0
            || self.train_mask.count_ones() != manifest.segment_count / 2
            || self.train_mask & 1 != 0
            || self.split_identity
                != split_identity(manifest.segment_count, self.train_mask, self.test_mask)
        {
            return Err(
                "population-statistics CSCV split is not canonical/complementary".to_owned(),
            );
        }
        let candidate = candidates
            .get(usize_of(self.candidate_sequence, "candidate sequence")?)
            .ok_or_else(|| "population-statistics split candidate is absent".to_owned())?;
        if candidate.candidate_semantic_digest != self.candidate_semantic_digest {
            return Err("population-statistics split moved to another candidate".to_owned());
        }
        Ok(())
    }
}

/// Explicit nonzero resource ceilings for one audit open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationStatisticsV2Bounds {
    audits: u64,
    candidates_per_audit: u64,
    periods_per_candidate: u64,
    splits_per_candidate: u64,
    file_bytes: u64,
}

impl PopulationStatisticsV2Bounds {
    /// Constructs explicit nonzero audit/candidate/period/split/byte ceilings.
    ///
    /// # Errors
    ///
    /// Refuses zero or a byte ceiling unable to hold the header and the two
    /// manifest records of one empty structural block.
    pub fn new(
        max_audits: u64,
        max_candidates_per_audit: u64,
        max_periods_per_candidate: u64,
        max_splits_per_candidate: u64,
        max_file_bytes: u64,
    ) -> Result<Self, PopulationStatisticsV2Refusal> {
        if [
            max_audits,
            max_candidates_per_audit,
            max_periods_per_candidate,
            max_splits_per_candidate,
            max_file_bytes,
        ]
        .contains(&0)
        {
            return Err("population-statistics bounds must all be nonzero".to_owned());
        }
        let minimum = POPULATION_STATISTICS_V2_HEADER_BYTES
            .checked_add(2 * POPULATION_STATISTICS_V2_RECORD_STRIDE)
            .ok_or_else(|| "population-statistics minimum byte bound overflowed".to_owned())?;
        if max_file_bytes < minimum {
            return Err(format!(
                "population-statistics byte ceiling {max_file_bytes} is below minimum {minimum}"
            ));
        }
        Ok(Self {
            audits: max_audits,
            candidates_per_audit: max_candidates_per_audit,
            periods_per_candidate: max_periods_per_candidate,
            splits_per_candidate: max_splits_per_candidate,
            file_bytes: max_file_bytes,
        })
    }

    /// Maximum completed audits.
    #[must_use]
    pub const fn max_audits(self) -> u64 {
        self.audits
    }

    /// Maximum candidates in one paired family.
    #[must_use]
    pub const fn max_candidates_per_audit(self) -> u64 {
        self.candidates_per_audit
    }

    /// Maximum aligned periods per candidate.
    #[must_use]
    pub const fn max_periods_per_candidate(self) -> u64 {
        self.periods_per_candidate
    }

    /// Maximum complementary splits per candidate.
    #[must_use]
    pub const fn max_splits_per_candidate(self) -> u64 {
        self.splits_per_candidate
    }

    /// Maximum bytes scanned from the audit file.
    #[must_use]
    pub const fn max_file_bytes(self) -> u64 {
        self.file_bytes
    }

    fn validate_manifest(
        self,
        manifest: &PopulationStatisticsManifestV2,
    ) -> Result<(), PopulationStatisticsV2Refusal> {
        if manifest.candidate_count()? > self.candidates_per_audit
            || manifest.period_count > self.periods_per_candidate
            || manifest.split_count > self.splits_per_candidate
        {
            return Err(
                "population-statistics manifest exceeds explicit semantic bounds".to_owned(),
            );
        }
        Ok(())
    }
}

/// Reopened, fully recomputed audit of one completed statistics block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationStatisticsV2ReopenAudit {
    first_record: u64,
    candidate_count: u64,
    completion_record_digest: [u8; 32],
    manifest: PopulationStatisticsManifestV2,
}

impl PopulationStatisticsV2ReopenAudit {
    /// Audit identity binding both pre-admission inputs and every raw/statistical row.
    #[must_use]
    pub const fn audit_id(self) -> [u8; 32] {
        self.manifest.audit_id
    }

    /// Zero-based completed-family sequence.
    #[must_use]
    pub const fn sequence(self) -> u64 {
        self.manifest.sequence
    }

    /// Signal rung in seconds.
    #[must_use]
    pub const fn rung_seconds(self) -> u32 {
        self.manifest.rung_seconds
    }

    /// Inclusive requested month span.
    #[must_use]
    pub const fn requested_span(self) -> RequestedSpanIdentityV1 {
        self.manifest.requested_span
    }

    /// Total NIFTY plus BANKNIFTY candidates.
    #[must_use]
    pub const fn candidate_count(self) -> u64 {
        self.candidate_count
    }

    /// Digest of the exact canonical receipt-last Completion record bytes.
    ///
    /// The digest binds the physical record sequence, semantic payload and
    /// record seal read during the fresh reopen.  It is not the audit ID and
    /// cannot be reconstructed from the audit ID alone.
    #[must_use]
    pub const fn completion_record_digest(self) -> [u8; 32] {
        self.completion_record_digest
    }

    /// NIFTY candidates, which always occupy the prefix.
    #[must_use]
    pub const fn nifty_candidate_count(self) -> u64 {
        self.manifest.nifty.candidate_count
    }

    /// BANKNIFTY candidates, which always follow the NIFTY prefix.
    #[must_use]
    pub const fn banknifty_candidate_count(self) -> u64 {
        self.manifest.banknifty.candidate_count
    }

    /// Aligned return periods retained for every candidate.
    #[must_use]
    pub const fn period_count(self) -> u64 {
        self.manifest.period_count
    }

    /// Canonical complementary CSCV splits retained for every candidate.
    #[must_use]
    pub const fn split_count(self) -> u64 {
        self.manifest.split_count
    }

    /// Exact family CSCV/PBO evidence.
    #[must_use]
    pub const fn cscv_pbo(self) -> PopulationCscvPboV2 {
        self.manifest.pbo
    }

    /// Exact White Reality Check family evidence.
    #[must_use]
    pub const fn white(self) -> PopulationFamilyTestV2 {
        self.manifest.white
    }

    /// Exact Hansen SPA family evidence.
    #[must_use]
    pub const fn spa(self) -> PopulationFamilyTestV2 {
        self.manifest.spa
    }

    /// Verifies the exact reopened pre-admission pair without minting a
    /// production capability.
    ///
    /// # Errors
    ///
    /// Refuses family order or any authority/universe/completion/count,
    /// requested-span, rung, horizon, feed, commit, calendar or prior-day
    /// policy mismatch.
    pub fn verify_pre_admission_pair(
        self,
        nifty: PreAdmissionDataReopenAuditV1,
        banknifty: PreAdmissionDataReopenAuditV1,
    ) -> Result<(), PopulationStatisticsV2Refusal> {
        verify_pre_admission_pair(&self.manifest, &nifty.value(), &banknifty.value())
    }
}

/// Opaque pre-admission family terms retained by a fresh Statistics V2 reopen.
///
/// There is no public constructor.  This is immutable source evidence for a
/// later cross-authority join, not an Admission or Finalization capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationStatisticsV2FamilySource {
    binding: PreAdmissionBindingV2,
}

impl PopulationStatisticsV2FamilySource {
    /// NIFTY or BANKNIFTY family represented by these terms.
    #[must_use]
    pub const fn family(self) -> InstrumentFamilyV1 {
        self.binding.family
    }

    /// Freshly reopened Pre-Admission authority bound into Statistics V2.
    #[must_use]
    pub const fn pre_admission_authority_id(self) -> [u8; 32] {
        self.binding.authority_id
    }

    /// Exact complete Candidate Universe consumed by Pre-Admission.
    #[must_use]
    pub const fn candidate_universe_id(self) -> [u8; 32] {
        self.binding.candidate_universe_id
    }

    /// Exact Candidate Universe receipt-last completion digest.
    #[must_use]
    pub const fn candidate_completion_digest(self) -> [u8; 32] {
        self.binding.candidate_completion_digest
    }

    /// Complete candidates retained for this family.
    #[must_use]
    pub const fn candidate_count(self) -> u64 {
        self.binding.candidate_count
    }
}

/// Opaque source terms from one freshly reopened Statistics V2 commit.
///
/// The value is available only from
/// [`PopulationStatisticsObservationCommitV2::projection_source`].  It binds
/// the exact Completion record read after the writer was dropped to the
/// detached Observation-to-Statistics link.  It intentionally does not carry
/// a population-search identity, ranking policy or final Population ID: those
/// belong to the later multi-authority finalization join.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationStatisticsV2ProjectionSource {
    audit: PopulationStatisticsV2ReopenAudit,
    observation_link: PopulationStatisticsObservationLinkV2,
}

impl PopulationStatisticsV2ProjectionSource {
    /// Statistics audit identity recomputed from every retained source term.
    #[must_use]
    pub const fn audit_id(self) -> [u8; 32] {
        self.audit.audit_id()
    }

    /// Digest of the exact freshly reopened receipt-last Completion record.
    #[must_use]
    pub const fn completion_record_digest(self) -> [u8; 32] {
        self.audit.completion_record_digest()
    }

    /// Detached Observation-to-Statistics link identity.
    #[must_use]
    pub const fn observation_statistics_link_id(self) -> [u8; 32] {
        self.observation_link.identity()
    }

    /// Durable Observation authority consumed by Statistics preparation.
    #[must_use]
    pub const fn observation_authority_id(self) -> [u8; 32] {
        self.observation_link.observation_authority_id()
    }

    /// Exact paired in-memory Observation identity consumed by preparation.
    #[must_use]
    pub const fn observation_pair_identity(self) -> [u8; 32] {
        self.observation_link.observation_pair_identity()
    }

    /// Versioned Observation-to-Statistics projection policy.
    #[must_use]
    pub const fn observation_projection_policy_digest(self) -> [u8; 32] {
        self.observation_link.projection_policy_digest()
    }

    /// Signal rung in seconds.
    #[must_use]
    pub const fn rung_seconds(self) -> u32 {
        self.audit.manifest.rung_seconds
    }

    /// Exit horizon in one-minute bars.
    #[must_use]
    pub const fn horizon_bars(self) -> u32 {
        self.audit.manifest.horizon_bars
    }

    /// Exact requested month span.
    #[must_use]
    pub const fn requested_span(self) -> RequestedSpanIdentityV1 {
        self.audit.manifest.requested_span
    }

    /// Feed identity shared by both instrument families.
    #[must_use]
    pub const fn feed_digest(self) -> [u8; 32] {
        self.audit.manifest.feed_digest
    }

    /// Source commit identity shared by both instrument families.
    #[must_use]
    pub const fn source_commit_digest(self) -> [u8; 32] {
        self.audit.manifest.source_commit_digest
    }

    /// Canonical IST calendar policy identity.
    #[must_use]
    pub const fn calendar_policy_digest(self) -> [u8; 32] {
        self.audit.manifest.calendar_policy_digest
    }

    /// Prior-day daily-reference policy identity.
    #[must_use]
    pub const fn daily_reference_policy_digest(self) -> [u8; 32] {
        self.audit.manifest.daily_reference_policy_digest
    }

    /// Exact Wilson procedure identity.
    #[must_use]
    pub const fn wilson_policy_digest(self) -> [u8; 32] {
        self.audit.manifest.wilson_policy_digest
    }

    /// Exact CSCV procedure identity.
    #[must_use]
    pub const fn cscv_policy_digest(self) -> [u8; 32] {
        self.audit.manifest.cscv_policy_digest
    }

    /// Complete ordered complementary-split family identity.
    #[must_use]
    pub const fn cscv_split_family_digest(self) -> [u8; 32] {
        self.audit.manifest.ordered_split_digest
    }

    /// Complete ordered candidate-family identity.
    #[must_use]
    pub const fn ordered_candidate_digest(self) -> [u8; 32] {
        self.audit.manifest.ordered_candidate_digest
    }

    /// Complete ordered aligned-period family identity.
    #[must_use]
    pub const fn ordered_period_digest(self) -> [u8; 32] {
        self.audit.manifest.ordered_period_digest
    }

    /// Genuine complementary-split CSCV/PBO evidence.
    #[must_use]
    pub const fn cscv_pbo(self) -> PopulationCscvPboV2 {
        self.audit.manifest.pbo
    }

    /// White Reality Check complete-family evidence.
    #[must_use]
    pub const fn white(self) -> PopulationFamilyTestV2 {
        self.audit.manifest.white
    }

    /// Hansen SPA complete-family evidence.
    #[must_use]
    pub const fn spa(self) -> PopulationFamilyTestV2 {
        self.audit.manifest.spa
    }

    /// Romano--Wolf complete ordered-family identity.
    #[must_use]
    pub const fn romano_wolf_family_digest(self) -> [u8; 32] {
        self.audit.manifest.romano_wolf_family_digest
    }

    /// Exact deterministic finite bootstrap draw count.
    #[must_use]
    pub const fn bootstrap_draws(self) -> u64 {
        self.audit.manifest.draws
    }

    /// Explicit deterministic bootstrap seed.
    #[must_use]
    pub const fn bootstrap_seed(self) -> u64 {
        self.audit.manifest.seed
    }

    /// Stationary-bootstrap block length in aligned periods.
    #[must_use]
    pub const fn bootstrap_block_length(self) -> u64 {
        self.audit.manifest.block_length
    }

    /// Complete compared-strategy count.
    #[must_use]
    pub const fn candidate_count(self) -> u64 {
        self.audit.candidate_count
    }

    /// Complete aligned-period count per candidate.
    #[must_use]
    pub const fn period_count(self) -> u64 {
        self.audit.manifest.period_count
    }

    /// Complete canonical complementary-split count per candidate.
    #[must_use]
    pub const fn split_count(self) -> u64 {
        self.audit.manifest.split_count
    }

    /// Exact source binding for one instrument family.
    #[must_use]
    pub const fn family_source(
        self,
        family: InstrumentFamilyV1,
    ) -> PopulationStatisticsV2FamilySource {
        let binding = match family {
            InstrumentFamilyV1::Nifty => self.audit.manifest.nifty,
            InstrumentFamilyV1::BankNifty => self.audit.manifest.banknifty,
        };
        PopulationStatisticsV2FamilySource { binding }
    }
}

/// Opaque exact candidate terms read under one fresh projection source.
///
/// A caller cannot construct or detach this value from its Statistics reopen.
/// It is evidence for a later Admission projection, not an Admission decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationStatisticsV2CandidateProjection {
    source: PopulationStatisticsV2ProjectionSource,
    candidate: PopulationStatisticsCandidateV2,
}

/// Opaque Statistics-to-Admission V3 projection authority.
///
/// This value can be produced only by one complete scan of a freshly reopened
/// Statistics V2 family.  That scan locates the unique Romano--Wolf rank-zero
/// row and retains its exact adjusted probability as the complete-family
/// maximum/intersection probability.  A later candidate projection is then a
/// fixed-stride read under this same source authority; the orchestrator cannot
/// supply or replace any statistical value.
///
/// Preparing this authority is O(C) record reads and O(C) rank-bit space after
/// the ledger's bounded generation validation, where C is the complete
/// candidate count.  Reading one candidate from a prepared authority uses one
/// fixed-stride record read; file locking, generation validation and I/O are
/// not O(1).  Production callers that need the complete family use the bounded
/// batch projection so generation hashing is not repeated once per candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationStatisticsAdmissionProjectionV3 {
    source: PopulationStatisticsV2ProjectionSource,
    familywise_romano_wolf_probability: PopulationStatisticsFractionV2,
}

impl PopulationStatisticsAdmissionProjectionV3 {
    /// Exact freshly reopened Statistics source retained by this authority.
    #[must_use]
    pub(crate) const fn source(&self) -> &PopulationStatisticsV2ProjectionSource {
        &self.source
    }

    /// Complete-family rank-zero Romano--Wolf probability.
    #[must_use]
    pub(crate) const fn familywise_romano_wolf_probability(
        &self,
    ) -> PopulationStatisticsFractionV2 {
        self.familywise_romano_wolf_probability
    }
}

/// One exact Statistics-to-Admission V3 candidate with source provenance.
///
/// The arithmetic draft never travels alone inside the CLI production path:
/// this opaque wrapper retains its freshly reopened Statistics source,
/// NIFTY/BANKNIFTY family, global and family ordinals, and exact Pre-Admission
/// authority.  A later Finalization join can therefore select the matching
/// Search/Base capabilities without accepting a caller-authored family label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PopulationStatisticsAdmissionCandidateV3 {
    projection: PopulationStatisticsV2CandidateProjection,
    draft: AdmissionStatisticsDraftV3,
}

impl PopulationStatisticsAdmissionCandidateV3 {
    /// Fresh Statistics source that authenticated this exact candidate.
    #[must_use]
    pub(crate) const fn source(&self) -> &PopulationStatisticsV2ProjectionSource {
        &self.projection.source
    }

    /// Global NIFTY-first candidate ordinal.
    #[must_use]
    pub(crate) const fn sequence(&self) -> u64 {
        self.projection.sequence()
    }

    /// Exact instrument family retained by Statistics.
    #[must_use]
    pub(crate) const fn family(&self) -> InstrumentFamilyV1 {
        self.projection.family()
    }

    /// Candidate ordinal inside the retained instrument family.
    #[must_use]
    pub(crate) const fn family_sequence(&self) -> u64 {
        self.projection.family_sequence()
    }

    /// Exact Candidate semantic identity retained on the Statistics row.
    #[must_use]
    pub(crate) const fn candidate_semantic_digest(&self) -> [u8; 32] {
        self.projection.candidate_semantic_digest()
    }

    /// Exact Pre-Admission authority for this family member.
    #[must_use]
    pub(crate) const fn pre_admission_authority_id(&self) -> [u8; 32] {
        self.projection.pre_admission_authority_id()
    }

    /// Complete ordered Candidate-family identity retained by Statistics.
    #[must_use]
    pub(crate) const fn ordered_candidate_digest(&self) -> [u8; 32] {
        self.projection.source.ordered_candidate_digest()
    }

    /// Complete ordered aligned-period family identity retained by Statistics.
    #[must_use]
    pub(crate) const fn ordered_period_family_digest(&self) -> [u8; 32] {
        self.projection.source.ordered_period_digest()
    }

    /// Candidate-local ordered aligned-period identity.
    #[must_use]
    pub(crate) const fn candidate_ordered_period_digest(&self) -> [u8; 32] {
        self.projection.ordered_period_digest()
    }

    /// Complete ordered complementary-split family identity retained by Statistics.
    #[must_use]
    pub(crate) const fn ordered_split_family_digest(&self) -> [u8; 32] {
        self.projection.source.cscv_split_family_digest()
    }

    /// Candidate-local ordered complementary-split identity.
    #[must_use]
    pub(crate) const fn candidate_ordered_split_digest(&self) -> [u8; 32] {
        self.projection.ordered_split_digest()
    }

    /// Complete compared-strategy count retained by Statistics.
    #[must_use]
    pub(crate) const fn candidate_count(&self) -> u64 {
        self.projection.source.candidate_count()
    }

    /// Complete aligned-period count retained by Statistics.
    #[must_use]
    pub(crate) const fn period_count(&self) -> u64 {
        self.projection.source.period_count()
    }

    /// Complete canonical complementary-split count retained by Statistics.
    #[must_use]
    pub(crate) const fn split_count(&self) -> u64 {
        self.projection.source.split_count()
    }

    /// Fixed-width Runner arithmetic draft bound to this opaque provenance.
    #[must_use]
    pub(crate) const fn draft(&self) -> &AdmissionStatisticsDraftV3 {
        &self.draft
    }
}

impl PopulationStatisticsV2CandidateProjection {
    /// Fresh Statistics source under which this candidate was read.
    #[must_use]
    pub const fn source(self) -> PopulationStatisticsV2ProjectionSource {
        self.source
    }

    /// Global NIFTY-first candidate ordinal.
    #[must_use]
    pub const fn sequence(self) -> u64 {
        self.candidate.sequence
    }

    /// NIFTY or BANKNIFTY candidate family.
    #[must_use]
    pub const fn family(self) -> InstrumentFamilyV1 {
        self.candidate.family
    }

    /// Ordinal inside the instrument family.
    #[must_use]
    pub const fn family_sequence(self) -> u64 {
        self.candidate.family_sequence
    }

    /// Exact candidate semantic identity.
    #[must_use]
    pub const fn candidate_semantic_digest(self) -> [u8; 32] {
        self.candidate.candidate_semantic_digest
    }

    /// Exact Pre-Admission authority that supplied this candidate.
    #[must_use]
    pub const fn pre_admission_authority_id(self) -> [u8; 32] {
        self.candidate.pre_admission_id
    }

    /// Exact trade denominator used by Wilson.
    #[must_use]
    pub const fn trades(self) -> u64 {
        self.candidate.trades
    }

    /// Exact winning-trade numerator used by Wilson.
    #[must_use]
    pub const fn wins(self) -> u64 {
        self.candidate.wins
    }

    /// Full-precision Wilson lower-bound bits retained on disk.
    #[must_use]
    pub const fn wilson_lower_bits(self) -> u64 {
        self.candidate.wilson_lower_bits
    }

    /// Full-precision Romano--Wolf observed-statistic bits.
    #[must_use]
    pub const fn romano_wolf_statistic_bits(self) -> u64 {
        self.candidate.romano_wolf_statistic_bits
    }

    /// Canonical descending-statistic Romano--Wolf rank.
    #[must_use]
    pub const fn romano_wolf_rank(self) -> u64 {
        self.candidate.romano_wolf_rank
    }

    /// Strict bootstrap exceedances before the finite-sample `+1`.
    #[must_use]
    pub const fn romano_wolf_strict_exceedances(self) -> u64 {
        self.candidate.romano_wolf_strict_exceedances
    }

    /// Exact initial candidate Romano--Wolf probability.
    #[must_use]
    pub const fn romano_wolf_initial(self) -> PopulationStatisticsFractionV2 {
        self.candidate.romano_wolf_initial
    }

    /// Exact monotone-adjusted candidate Romano--Wolf probability.
    #[must_use]
    pub const fn romano_wolf_adjusted(self) -> PopulationStatisticsFractionV2 {
        self.candidate.romano_wolf_adjusted
    }

    /// Ordered aligned-period rows retained for this candidate.
    #[must_use]
    pub const fn ordered_period_digest(self) -> [u8; 32] {
        self.candidate.ordered_period_digest
    }

    /// Ordered CSCV score rows retained for this candidate.
    #[must_use]
    pub const fn ordered_split_digest(self) -> [u8; 32] {
        self.candidate.ordered_split_digest
    }
}

/// Bounded candidate page from one completed audit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopulationStatisticsV2CandidatePage {
    rows: Vec<PopulationStatisticsCandidateV2>,
}

impl PopulationStatisticsV2CandidatePage {
    /// Candidate rows in canonical NIFTY-then-BANKNIFTY order.
    #[must_use]
    pub fn rows(&self) -> &[PopulationStatisticsCandidateV2] {
        &self.rows
    }
}

/// Structurally valid but incomplete receipt-last block observed at file tail.
///
/// This is audit evidence only. It does not authorize completing the block;
/// only the absent production source authority could make that write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationStatisticsV2TrailingPrefixAudit {
    audit_id: [u8; 32],
    logical_sequence: u64,
    first_record: u64,
    present_records: u64,
    planned_records: u64,
}

impl PopulationStatisticsV2TrailingPrefixAudit {
    /// Identity declared by the valid Data record at the prefix head.
    #[must_use]
    pub const fn audit_id(self) -> [u8; 32] {
        self.audit_id
    }

    /// Logical append sequence declared by the Data record.
    #[must_use]
    pub const fn logical_sequence(self) -> u64 {
        self.logical_sequence
    }

    /// Physical record at which the incomplete block begins.
    #[must_use]
    pub const fn first_record(self) -> u64 {
        self.first_record
    }

    /// Valid records currently present, including Data.
    #[must_use]
    pub const fn present_records(self) -> u64 {
        self.present_records
    }

    /// Records required for a complete receipt-last block.
    #[must_use]
    pub const fn planned_records(self) -> u64 {
        self.planned_records
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DecodedRecordV2 {
    kind: RecordKindV2,
    payload: [u8; PAYLOAD_BYTES],
}

fn encode_manifest_record(
    manifest: &PopulationStatisticsManifestV2,
    kind: RecordKindV2,
    physical_sequence: u64,
) -> Result<[u8; RECORD_BYTES], PopulationStatisticsV2Refusal> {
    encode_manifest_record_with_validation(
        manifest,
        kind,
        physical_sequence,
        PopulationStatisticsManifestV2::validate,
    )
}

#[cfg(test)]
fn encode_stored_compatible_manifest_record(
    manifest: &PopulationStatisticsManifestV2,
    kind: RecordKindV2,
    physical_sequence: u64,
) -> Result<[u8; RECORD_BYTES], PopulationStatisticsV2Refusal> {
    encode_manifest_record_with_validation(
        manifest,
        kind,
        physical_sequence,
        PopulationStatisticsManifestV2::validate_stored_compatible,
    )
}

fn encode_manifest_record_with_validation(
    manifest: &PopulationStatisticsManifestV2,
    kind: RecordKindV2,
    physical_sequence: u64,
    validate: fn(PopulationStatisticsManifestV2) -> Result<(), PopulationStatisticsV2Refusal>,
) -> Result<[u8; RECORD_BYTES], PopulationStatisticsV2Refusal> {
    if !matches!(kind, RecordKindV2::Data | RecordKindV2::Completion) {
        return Err("population-statistics manifest used a non-manifest kind".to_owned());
    }
    validate(*manifest)?;
    let mut payload = [0_u8; PAYLOAD_BYTES];
    put_u32(&mut payload, 0, RECORD_VERSION)?;
    put_u32(&mut payload, 4, kind.byte())?;
    put_u64(&mut payload, 8, physical_sequence)?;
    put_bytes(&mut payload, 16, &manifest.audit_id)?;
    put_u64(&mut payload, 48, manifest.sequence)?;
    put_u32(&mut payload, 56, manifest.rung_seconds)?;
    put_u32(&mut payload, 60, manifest.horizon_bars)?;
    encode_span(manifest.requested_span, &mut payload, 64)?;
    put_u32(&mut payload, 84, manifest.segment_count)?;
    put_u64(&mut payload, 88, manifest.period_count)?;
    put_u64(&mut payload, 96, manifest.split_count)?;
    put_u64(&mut payload, 104, manifest.draws)?;
    put_u64(&mut payload, 112, manifest.seed)?;
    put_u64(&mut payload, 120, manifest.block_length)?;
    encode_binding(manifest.nifty, &mut payload, 128)?;
    encode_binding(manifest.banknifty, &mut payload, 248)?;
    put_bytes(&mut payload, 368, &manifest.feed_digest)?;
    put_bytes(&mut payload, 400, &manifest.source_commit_digest)?;
    put_bytes(&mut payload, 432, &manifest.calendar_policy_digest)?;
    put_bytes(&mut payload, 464, &manifest.daily_reference_policy_digest)?;
    put_bytes(&mut payload, 496, &manifest.wilson_policy_digest)?;
    put_bytes(&mut payload, 528, &manifest.cscv_policy_digest)?;
    encode_family_test(manifest.white, &mut payload, 560)?;
    encode_family_test(manifest.spa, &mut payload, 624)?;
    put_bytes(&mut payload, 688, &manifest.romano_wolf_family_digest)?;
    put_u64(&mut payload, 720, manifest.pbo.contributing_splits)?;
    put_u64(&mut payload, 728, manifest.pbo.bottom_half_splits)?;
    put_u64(&mut payload, 736, manifest.pbo.unrankable_splits)?;
    put_u64(&mut payload, 744, manifest.pbo.probability.numerator)?;
    put_u64(&mut payload, 752, manifest.pbo.probability.denominator)?;
    put_u64(&mut payload, 760, manifest.pbo.probability_bits)?;
    put_bytes(&mut payload, 768, &manifest.ordered_candidate_digest)?;
    put_bytes(&mut payload, 800, &manifest.ordered_period_digest)?;
    put_bytes(&mut payload, 832, &manifest.ordered_split_digest)?;
    seal_payload(&payload)
}

fn decode_manifest(
    payload: &[u8; PAYLOAD_BYTES],
    kind: RecordKindV2,
) -> Result<PopulationStatisticsManifestV2, PopulationStatisticsV2Refusal> {
    require_zero(payload, 129, 7, "NIFTY binding reserve")?;
    require_zero(payload, 249, 7, "BANKNIFTY binding reserve")?;
    require_zero(payload, 864, PAYLOAD_BYTES - 864, "manifest tail reserve")?;
    let manifest = PopulationStatisticsManifestV2 {
        sequence: get_u64(payload, 48)?,
        audit_id: get_32(payload, 16)?,
        rung_seconds: get_u32(payload, 56)?,
        horizon_bars: get_u32(payload, 60)?,
        requested_span: decode_span(payload, 64)?,
        segment_count: get_u32(payload, 84)?,
        period_count: get_u64(payload, 88)?,
        split_count: get_u64(payload, 96)?,
        draws: get_u64(payload, 104)?,
        seed: get_u64(payload, 112)?,
        block_length: get_u64(payload, 120)?,
        nifty: decode_binding(payload, 128)?,
        banknifty: decode_binding(payload, 248)?,
        feed_digest: get_32(payload, 368)?,
        source_commit_digest: get_32(payload, 400)?,
        calendar_policy_digest: get_32(payload, 432)?,
        daily_reference_policy_digest: get_32(payload, 464)?,
        wilson_policy_digest: get_32(payload, 496)?,
        cscv_policy_digest: get_32(payload, 528)?,
        white: decode_family_test(payload, 560)?,
        spa: decode_family_test(payload, 624)?,
        romano_wolf_family_digest: get_32(payload, 688)?,
        pbo: PopulationCscvPboV2 {
            contributing_splits: get_u64(payload, 720)?,
            bottom_half_splits: get_u64(payload, 728)?,
            unrankable_splits: get_u64(payload, 736)?,
            probability: PopulationStatisticsFractionV2::new(
                get_u64(payload, 744)?,
                get_u64(payload, 752)?,
            )?,
            probability_bits: get_u64(payload, 760)?,
        },
        ordered_candidate_digest: get_32(payload, 768)?,
        ordered_period_digest: get_32(payload, 800)?,
        ordered_split_digest: get_32(payload, 832)?,
    };
    if !matches!(kind, RecordKindV2::Data | RecordKindV2::Completion) {
        return Err("population-statistics decoded manifest kind differs".to_owned());
    }
    manifest.validate_stored_compatible()?;
    Ok(manifest)
}

fn encode_candidate_record(
    candidate: &PopulationStatisticsCandidateV2,
    manifest: &PopulationStatisticsManifestV2,
    physical_sequence: u64,
) -> Result<[u8; RECORD_BYTES], PopulationStatisticsV2Refusal> {
    candidate.validate(manifest)?;
    let mut payload = base_payload(
        RecordKindV2::Candidate,
        physical_sequence,
        candidate.audit_id,
    )?;
    put_u64(&mut payload, 48, candidate.sequence)?;
    put_u8(&mut payload, 56, family_byte(candidate.family))?;
    put_u64(&mut payload, 64, candidate.family_sequence)?;
    put_bytes(&mut payload, 72, &candidate.candidate_semantic_digest)?;
    put_bytes(&mut payload, 104, &candidate.pre_admission_id)?;
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
    seal_payload(&payload)
}

fn decode_candidate(
    payload: &[u8; PAYLOAD_BYTES],
) -> Result<PopulationStatisticsCandidateV2, PopulationStatisticsV2Refusal> {
    require_zero(payload, 57, 7, "candidate family reserve")?;
    require_zero(payload, 280, PAYLOAD_BYTES - 280, "candidate tail reserve")?;
    let candidate = PopulationStatisticsCandidateV2 {
        audit_id: get_32(payload, 16)?,
        sequence: get_u64(payload, 48)?,
        family: family_from_byte(get_u8(payload, 56)?)?,
        family_sequence: get_u64(payload, 64)?,
        candidate_semantic_digest: get_32(payload, 72)?,
        pre_admission_id: get_32(payload, 104)?,
        trades: get_u64(payload, 136)?,
        wins: get_u64(payload, 144)?,
        wilson_lower_bits: get_u64(payload, 152)?,
        romano_wolf_statistic_bits: get_u64(payload, 160)?,
        romano_wolf_rank: get_u64(payload, 168)?,
        romano_wolf_strict_exceedances: get_u64(payload, 176)?,
        romano_wolf_initial: PopulationStatisticsFractionV2::new(
            get_u64(payload, 184)?,
            get_u64(payload, 192)?,
        )?,
        romano_wolf_adjusted: PopulationStatisticsFractionV2::new(
            get_u64(payload, 200)?,
            get_u64(payload, 208)?,
        )?,
        ordered_period_digest: get_32(payload, 216)?,
        ordered_split_digest: get_32(payload, 248)?,
    };
    Ok(candidate)
}

fn encode_period_record(
    period: &PopulationStatisticsPeriodSourceV2,
    manifest: &PopulationStatisticsManifestV2,
    candidates: &[PopulationStatisticsCandidateV2],
    physical_sequence: u64,
) -> Result<[u8; RECORD_BYTES], PopulationStatisticsV2Refusal> {
    period.validate(manifest, candidates)?;
    let mut payload = base_payload(RecordKindV2::Period, physical_sequence, period.audit_id)?;
    put_u64(&mut payload, 48, period.candidate_sequence)?;
    put_u64(&mut payload, 56, period.period_sequence)?;
    put_bytes(&mut payload, 64, &period.candidate_semantic_digest)?;
    put_i64(&mut payload, 96, period.return_paisa)?;
    put_u64(&mut payload, 104, period.trades)?;
    put_u64(&mut payload, 112, period.wins)?;
    put_bytes(&mut payload, 120, &period.period_identity)?;
    seal_payload(&payload)
}

fn decode_period(
    payload: &[u8; PAYLOAD_BYTES],
) -> Result<PopulationStatisticsPeriodSourceV2, PopulationStatisticsV2Refusal> {
    require_zero(payload, 152, PAYLOAD_BYTES - 152, "period tail reserve")?;
    Ok(PopulationStatisticsPeriodSourceV2 {
        audit_id: get_32(payload, 16)?,
        candidate_sequence: get_u64(payload, 48)?,
        period_sequence: get_u64(payload, 56)?,
        candidate_semantic_digest: get_32(payload, 64)?,
        return_paisa: get_i64(payload, 96)?,
        trades: get_u64(payload, 104)?,
        wins: get_u64(payload, 112)?,
        period_identity: get_32(payload, 120)?,
    })
}

fn encode_split_record(
    split: &PopulationStatisticsSplitSourceV2,
    manifest: &PopulationStatisticsManifestV2,
    candidates: &[PopulationStatisticsCandidateV2],
    expected_mask: u64,
    physical_sequence: u64,
) -> Result<[u8; RECORD_BYTES], PopulationStatisticsV2Refusal> {
    split.validate(manifest, candidates, expected_mask)?;
    let mut payload = base_payload(RecordKindV2::Split, physical_sequence, split.audit_id)?;
    put_u64(&mut payload, 48, split.candidate_sequence)?;
    put_u64(&mut payload, 56, split.split_sequence)?;
    put_bytes(&mut payload, 64, &split.candidate_semantic_digest)?;
    put_u32(&mut payload, 96, split.segment_count)?;
    put_u64(&mut payload, 104, split.train_mask)?;
    put_u64(&mut payload, 112, split.test_mask)?;
    put_i64(&mut payload, 120, split.train_score)?;
    put_i64(&mut payload, 128, split.test_score)?;
    put_bytes(&mut payload, 136, &split.split_identity)?;
    seal_payload(&payload)
}

fn decode_split(
    payload: &[u8; PAYLOAD_BYTES],
) -> Result<PopulationStatisticsSplitSourceV2, PopulationStatisticsV2Refusal> {
    require_zero(payload, 100, 4, "split segment reserve")?;
    require_zero(payload, 168, PAYLOAD_BYTES - 168, "split tail reserve")?;
    Ok(PopulationStatisticsSplitSourceV2 {
        audit_id: get_32(payload, 16)?,
        candidate_sequence: get_u64(payload, 48)?,
        split_sequence: get_u64(payload, 56)?,
        candidate_semantic_digest: get_32(payload, 64)?,
        segment_count: get_u32(payload, 96)?,
        train_mask: get_u64(payload, 104)?,
        test_mask: get_u64(payload, 112)?,
        train_score: get_i64(payload, 120)?,
        test_score: get_i64(payload, 128)?,
        split_identity: get_32(payload, 136)?,
    })
}

fn decode_record(
    raw: &[u8; RECORD_BYTES],
    expected_physical_sequence: u64,
) -> Result<DecodedRecordV2, PopulationStatisticsV2Refusal> {
    let payload: &[u8; PAYLOAD_BYTES] = raw
        .get(..PAYLOAD_BYTES)
        .ok_or_else(|| "population-statistics payload is absent".to_owned())?
        .try_into()
        .map_err(|_| "population-statistics payload width differs".to_owned())?;
    let seal = raw
        .get(PAYLOAD_BYTES..)
        .ok_or_else(|| "population-statistics record seal is absent".to_owned())?;
    if digest_domain(RECORD_DOMAIN, payload).as_slice() != seal {
        return Err("population-statistics record seal differs".to_owned());
    }
    if get_u32(payload, 0)? != RECORD_VERSION {
        return Err("population-statistics record version is unknown".to_owned());
    }
    if get_u64(payload, 8)? != expected_physical_sequence {
        return Err(format!(
            "population-statistics physical sequence differs at record {expected_physical_sequence}"
        ));
    }
    let kind = RecordKindV2::from_byte(get_u32(payload, 4)?)?;
    Ok(DecodedRecordV2 {
        kind,
        payload: *payload,
    })
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGenerationV2 {
    len: u64,
    content_digest: [u8; 32],
    device: u64,
    inode: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(not(unix))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGenerationV2 {
    len: u64,
    content_digest: [u8; 32],
    modified: Option<std::time::SystemTime>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OrphanV2 {
    first_record: u64,
    present_records: u64,
    planned_records: u64,
    manifest: PopulationStatisticsManifestV2,
}

/// Read-only Population Statistics V2 audit ledger.
pub struct PopulationStatisticsV2Ledger {
    lock_path: PathBuf,
    data_path: PathBuf,
    lock_file: File,
    data_file: File,
    bounds: PopulationStatisticsV2Bounds,
    audits: HashMap<[u8; 32], PopulationStatisticsV2ReopenAudit>,
    completed_audits: u64,
    orphan: Option<OrphanV2>,
    lock_generation: FileGenerationV2,
    data_generation: FileGenerationV2,
}

impl PopulationStatisticsV2Ledger {
    /// Opens existing statistics bytes without creating or modifying a path.
    ///
    /// # Errors
    ///
    /// Refuses absent files, a zero/breached bound, unknown header, ragged or
    /// corrupt record, incomplete middle block, noncanonical family/split
    /// ordering, statistics that do not recompute from raw rows, duplicate
    /// audit, stale/replaced path, lock failure or I/O error.
    pub fn open_read(
        root: impl AsRef<Path>,
        bounds: PopulationStatisticsV2Bounds,
    ) -> Result<Self, PopulationStatisticsV2Refusal> {
        Self::open_inner(root.as_ref(), bounds, false)
    }

    fn open_writer(
        root: impl AsRef<Path>,
        bounds: PopulationStatisticsV2Bounds,
    ) -> Result<Self, PopulationStatisticsV2Refusal> {
        Self::open_inner(root.as_ref(), bounds, true)
    }

    fn open_inner(
        root: &Path,
        bounds: PopulationStatisticsV2Bounds,
        writable: bool,
    ) -> Result<Self, PopulationStatisticsV2Refusal> {
        let root_metadata = std::fs::metadata(root).map_err(|why| {
            format!(
                "population-statistics root {} must already exist: {why}",
                root.display()
            )
        })?;
        if !root_metadata.is_dir() {
            return Err(format!(
                "population-statistics root {} is not a directory",
                root.display()
            ));
        }
        let lock_path = root.join(LOCK_FILE);
        let data_path = root.join(DATA_FILE);
        let lock_file = open_file(&lock_path, writable, writable)?;
        if writable {
            lock_file.lock().map_err(|why| {
                format!(
                    "cannot lock population-statistics writer {}: {why}",
                    lock_path.display()
                )
            })?;
        } else {
            lock_file.lock_shared().map_err(|why| {
                format!(
                    "cannot take population-statistics shared lock {}: {why}",
                    lock_path.display()
                )
            })?;
        }
        let held_lock = lock_file.try_clone().map_err(|why| {
            format!(
                "cannot clone population-statistics lock {}: {why}",
                lock_path.display()
            )
        })?;
        let opened = (|| {
            let mut data_file = open_file(&data_path, writable, writable)?;
            if writable {
                ensure_header(&mut data_file, &data_path)?;
            } else {
                verify_header(&mut data_file, &data_path)?;
            }
            let lock_generation = file_generation(&held_lock, &lock_path, LOCK_FILE_MAX_BYTES)?;
            let data_generation = file_generation(&data_file, &data_path, bounds.file_bytes)?;
            let mut audits = HashMap::new();
            audits
                .try_reserve(usize_of(bounds.audits, "audit bound")?)
                .map_err(|why| format!("cannot reserve population-statistics index: {why}"))?;
            let mut ledger = Self {
                lock_path: lock_path.clone(),
                data_path,
                lock_file: held_lock,
                data_file,
                bounds,
                audits,
                completed_audits: 0,
                orphan: None,
                lock_generation,
                data_generation,
            };
            ledger.scan()?;
            Ok(ledger)
        })();
        let released = lock_file.unlock().map_err(|why| {
            format!(
                "cannot release population-statistics open lock {}: {why}",
                lock_path.display()
            )
        });
        match (opened, released) {
            (Ok(ledger), Ok(())) => Ok(ledger),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn scan(&mut self) -> Result<(), PopulationStatisticsV2Refusal> {
        verify_header(&mut self.data_file, &self.data_path)?;
        let file_len = self
            .data_file
            .metadata()
            .map_err(|why| format!("cannot stat {}: {why}", self.data_path.display()))?
            .len();
        if file_len > self.bounds.file_bytes {
            return Err(format!(
                "population-statistics file has {file_len} bytes above explicit maximum {}",
                self.bounds.file_bytes
            ));
        }
        let records = record_count(file_len)?;
        let mut cursor = 0_u64;
        while cursor < records {
            if self.completed_audits >= self.bounds.audits {
                return Err("population-statistics audits exceed explicit maximum".to_owned());
            }
            let decoded = read_record(&mut self.data_file, cursor)?;
            if decoded.kind != RecordKindV2::Data {
                return Err(format!(
                    "population-statistics block at record {cursor} does not begin with Data"
                ));
            }
            let manifest = decode_manifest(&decoded.payload, decoded.kind)?;
            if manifest.sequence != self.completed_audits {
                return Err(format!(
                    "population-statistics logical sequence {} is not canonical {}",
                    manifest.sequence, self.completed_audits
                ));
            }
            self.bounds.validate_manifest(&manifest)?;
            let block_records = manifest.block_record_count()?;
            let remaining = records
                .checked_sub(cursor)
                .ok_or_else(|| "population-statistics remaining record underflowed".to_owned())?;
            if remaining < block_records {
                validate_orphan_prefix(&mut self.data_file, cursor, remaining, &manifest)?;
                self.orphan = Some(OrphanV2 {
                    first_record: cursor,
                    present_records: remaining,
                    planned_records: block_records,
                    manifest,
                });
                cursor = records;
                continue;
            }
            let audit = validate_complete_block(&mut self.data_file, cursor, &manifest)?;
            if self.audits.insert(manifest.audit_id, audit).is_some() {
                return Err(format!(
                    "population-statistics audit {} appears more than once",
                    hex32(manifest.audit_id)
                ));
            }
            self.completed_audits = self
                .completed_audits
                .checked_add(1)
                .ok_or_else(|| "population-statistics audit count overflowed".to_owned())?;
            cursor = cursor
                .checked_add(block_records)
                .ok_or_else(|| "population-statistics cursor overflowed".to_owned())?;
        }
        Ok(())
    }

    /// Number of completed, fully recomputed audits.
    #[must_use]
    pub const fn completed_audits(&self) -> u64 {
        self.completed_audits
    }

    /// Returns audit evidence for the sole valid incomplete tail, if present.
    ///
    /// The result is observational and confers no write or completion power.
    ///
    /// # Errors
    ///
    /// Refuses a stale/replaced data or lock path and I/O/lock errors.
    pub fn trailing_prefix_audit(
        &mut self,
    ) -> Result<Option<PopulationStatisticsV2TrailingPrefixAudit>, PopulationStatisticsV2Refusal>
    {
        self.with_shared_lock(|ledger| {
            Ok(ledger
                .orphan
                .map(|orphan| PopulationStatisticsV2TrailingPrefixAudit {
                    audit_id: orphan.manifest.audit_id,
                    logical_sequence: orphan.manifest.sequence,
                    first_record: orphan.first_record,
                    present_records: orphan.present_records,
                    planned_records: orphan.planned_records,
                }))
        })
    }

    /// Looks up one completed audit after stale-generation validation.
    ///
    /// # Errors
    ///
    /// Refuses a stale/replaced data or lock path and I/O/lock errors.
    pub fn reopen_audit(
        &mut self,
        audit_id: &[u8; 32],
    ) -> Result<Option<PopulationStatisticsV2ReopenAudit>, PopulationStatisticsV2Refusal> {
        self.with_shared_lock(|ledger| Ok(ledger.audits.get(audit_id).copied()))
    }

    /// Reads one candidate by validated fixed-record sequence.
    ///
    /// # Errors
    ///
    /// Refuses an absent audit/row, stale generation, changed kind, identity or
    /// any record that no longer validates against its completion.
    pub fn candidate(
        &mut self,
        audit_id: &[u8; 32],
        sequence: u64,
    ) -> Result<PopulationStatisticsCandidateV2, PopulationStatisticsV2Refusal> {
        self.with_shared_lock(|ledger| {
            let audit = ledger
                .audits
                .get(audit_id)
                .copied()
                .ok_or_else(|| "population-statistics audit is absent".to_owned())?;
            if sequence >= audit.candidate_count() {
                return Err("population-statistics candidate sequence is outside audit".to_owned());
            }
            read_candidate_at(
                &mut ledger.data_file,
                audit.first_record,
                &audit.manifest,
                sequence,
            )
        })
    }

    /// Reads one exact candidate under an opaque fresh-reopen projection source.
    ///
    /// # Errors
    ///
    /// Refuses a foreign/stale source, absent or out-of-range candidate,
    /// changed record, path replacement, corruption, lock or I/O failure.
    pub fn projection_candidate(
        &mut self,
        source: PopulationStatisticsV2ProjectionSource,
        sequence: u64,
    ) -> Result<PopulationStatisticsV2CandidateProjection, PopulationStatisticsV2Refusal> {
        self.with_shared_lock(|ledger| {
            source
                .observation_link
                .require_reopened_statistics(source.audit)?;
            let reopened = ledger
                .audits
                .get(&source.audit_id())
                .copied()
                .ok_or_else(|| "population-statistics projection source is absent".to_owned())?;
            if reopened != source.audit {
                return Err(
                    "population-statistics projection source differs from fresh reopen".to_owned(),
                );
            }
            if sequence >= reopened.candidate_count() {
                return Err(
                    "population-statistics projection candidate is outside audit".to_owned(),
                );
            }
            let candidate = read_candidate_at(
                &mut ledger.data_file,
                reopened.first_record,
                &reopened.manifest,
                sequence,
            )?;
            Ok(PopulationStatisticsV2CandidateProjection { source, candidate })
        })
    }

    /// Scans one complete reopened Statistics family and seals the exact
    /// Statistics-to-Admission V3 projection source.
    ///
    /// # Errors
    ///
    /// Refuses a foreign/stale source, changed or corrupt candidate record,
    /// missing/duplicate Romano--Wolf rank zero, incomplete rank permutation,
    /// path replacement, lock or I/O failure.
    pub(crate) fn prepare_admission_projection_v3(
        &mut self,
        source: &PopulationStatisticsV2ProjectionSource,
    ) -> Result<PopulationStatisticsAdmissionProjectionV3, PopulationStatisticsV2Refusal> {
        self.with_shared_lock(|ledger| {
            source
                .observation_link
                .require_reopened_statistics(source.audit)?;
            let reopened = ledger
                .audits
                .get(&source.audit_id())
                .copied()
                .ok_or_else(|| "population-statistics Admission V3 source is absent".to_owned())?;
            if reopened != source.audit {
                return Err(
                    "population-statistics Admission V3 source differs from fresh reopen"
                        .to_owned(),
                );
            }

            let count = reopened.candidate_count();
            let mut seen_ranks = Vec::new();
            seen_ranks
                .try_reserve_exact(usize_of(count, "Admission V3 rank count")?)
                .map_err(|why| {
                    format!("cannot reserve Admission V3 Romano-Wolf rank index: {why}")
                })?;
            seen_ranks.resize(usize_of(count, "Admission V3 rank count")?, false);
            let mut familywise = None;
            for sequence in 0..count {
                let candidate = read_candidate_at(
                    &mut ledger.data_file,
                    reopened.first_record,
                    &reopened.manifest,
                    sequence,
                )?;
                let rank = usize_of(candidate.romano_wolf_rank, "Admission V3 Romano-Wolf rank")?;
                let rank_slot = seen_ranks.get_mut(rank).ok_or_else(|| {
                    "population-statistics Admission V3 Romano-Wolf rank is outside family"
                        .to_owned()
                })?;
                if *rank_slot {
                    return Err(
                        "population-statistics Admission V3 has a duplicate Romano-Wolf rank"
                            .to_owned(),
                    );
                }
                *rank_slot = true;
                if rank == 0 && familywise.replace(candidate.romano_wolf_adjusted).is_some() {
                    return Err(
                        "population-statistics Admission V3 has more than one rank-zero candidate"
                            .to_owned(),
                    );
                }
            }
            if seen_ranks.contains(&false) {
                return Err(
                    "population-statistics Admission V3 Romano-Wolf rank permutation is incomplete"
                        .to_owned(),
                );
            }
            let familywise_romano_wolf_probability = familywise.ok_or_else(|| {
                "population-statistics Admission V3 rank-zero probability is absent".to_owned()
            })?;
            Ok(PopulationStatisticsAdmissionProjectionV3 {
                source: *source,
                familywise_romano_wolf_probability,
            })
        })
    }

    /// Projects one fixed-stride Statistics candidate into exact Admission V3
    /// arithmetic under a previously completed family scan.
    ///
    /// # Errors
    ///
    /// Refuses a stale/foreign projection, an out-of-range or changed record,
    /// malformed exact probability, non-finite Wilson value, path replacement,
    /// lock or I/O failure.
    pub(crate) fn admission_candidate_v3(
        &mut self,
        authority: &PopulationStatisticsAdmissionProjectionV3,
        sequence: u64,
    ) -> Result<PopulationStatisticsAdmissionCandidateV3, PopulationStatisticsV2Refusal> {
        self.with_shared_lock(|ledger| {
            let reopened = require_admission_projection_source_v3(ledger, authority)?;
            if sequence >= reopened.candidate_count() {
                return Err(
                    "population-statistics Admission V3 candidate is outside audit".to_owned(),
                );
            }
            let candidate = read_candidate_at(
                &mut ledger.data_file,
                reopened.first_record,
                &reopened.manifest,
                sequence,
            )?;
            admission_candidate_projection_v3(authority, &candidate)
        })
    }

    /// Projects the complete Statistics family under one bounded lock and one
    /// pre/post generation-validation pair.
    ///
    /// This is the production path for Admission V3.  It is O(C) fixed-record
    /// reads and O(C) returned output space, not O(1) for the whole family; the
    /// Runner arithmetic performed on each returned candidate remains
    /// fixed-width.  Unlike repeated single-candidate calls, generation hashing
    /// is not multiplied by C.
    ///
    /// # Errors
    ///
    /// Refuses a stale/foreign authority, allocation failure, any changed or
    /// corrupt candidate record, path replacement, lock or I/O failure.
    pub(crate) fn admission_candidates_v3(
        &mut self,
        authority: &PopulationStatisticsAdmissionProjectionV3,
    ) -> Result<Vec<PopulationStatisticsAdmissionCandidateV3>, PopulationStatisticsV2Refusal> {
        self.with_shared_lock(|ledger| {
            let reopened = require_admission_projection_source_v3(ledger, authority)?;
            let count = reopened.candidate_count();
            let mut candidates = Vec::new();
            candidates
                .try_reserve_exact(usize_of(count, "Admission V3 candidate count")?)
                .map_err(|why| {
                    format!("cannot reserve Admission V3 candidate projections: {why}")
                })?;
            for sequence in 0..count {
                let candidate = read_candidate_at(
                    &mut ledger.data_file,
                    reopened.first_record,
                    &reopened.manifest,
                    sequence,
                )?;
                candidates.push(admission_candidate_projection_v3(authority, &candidate)?);
            }
            Ok(candidates)
        })
    }

    /// Reads a bounded contiguous candidate page.
    ///
    /// # Errors
    ///
    /// Refuses zero/over-256 limit, absent audit, out-of-range offset, stale
    /// generation, allocation failure or changed/corrupt rows.
    pub fn page_candidates(
        &mut self,
        audit_id: &[u8; 32],
        offset: u64,
        limit: u64,
    ) -> Result<PopulationStatisticsV2CandidatePage, PopulationStatisticsV2Refusal> {
        if limit == 0 || limit > MAX_POPULATION_STATISTICS_V2_PAGE_ROWS {
            return Err(format!(
                "population-statistics page limit {limit} is outside 1..={MAX_POPULATION_STATISTICS_V2_PAGE_ROWS}"
            ));
        }
        self.with_shared_lock(|ledger| {
            let audit = ledger
                .audits
                .get(audit_id)
                .copied()
                .ok_or_else(|| "population-statistics audit is absent".to_owned())?;
            if offset > audit.candidate_count() {
                return Err("population-statistics page offset exceeds candidate count".to_owned());
            }
            let remaining = audit
                .candidate_count()
                .checked_sub(offset)
                .ok_or_else(|| "population-statistics page offset underflowed".to_owned())?;
            let count = limit.min(remaining);
            let mut rows = Vec::new();
            rows.try_reserve_exact(usize_of(count, "page count")?)
                .map_err(|why| format!("cannot reserve population-statistics page: {why}"))?;
            for local in 0..count {
                let sequence = offset
                    .checked_add(local)
                    .ok_or_else(|| "population-statistics page sequence overflowed".to_owned())?;
                rows.push(read_candidate_at(
                    &mut ledger.data_file,
                    audit.first_record,
                    &audit.manifest,
                    sequence,
                )?);
            }
            Ok(PopulationStatisticsV2CandidatePage { rows })
        })
    }

    fn with_shared_lock<T>(
        &mut self,
        action: impl FnOnce(&mut Self) -> Result<T, PopulationStatisticsV2Refusal>,
    ) -> Result<T, PopulationStatisticsV2Refusal> {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take population-statistics audit lock: {why}"))?;
        let result = (|| {
            self.require_unchanged()?;
            let value = action(self)?;
            self.require_unchanged()?;
            Ok(value)
        })();
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release population-statistics audit lock: {why}"));
        match (result, released) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn require_unchanged(&self) -> Result<(), PopulationStatisticsV2Refusal> {
        require_generation(self.lock_generation, &self.lock_file, &self.lock_path)?;
        require_generation(self.data_generation, &self.data_file, &self.data_path)
    }
}

fn require_admission_projection_source_v3(
    ledger: &PopulationStatisticsV2Ledger,
    authority: &PopulationStatisticsAdmissionProjectionV3,
) -> Result<PopulationStatisticsV2ReopenAudit, PopulationStatisticsV2Refusal> {
    let source = authority.source();
    source
        .observation_link
        .require_reopened_statistics(source.audit)?;
    let reopened = ledger
        .audits
        .get(&source.audit_id())
        .copied()
        .ok_or_else(|| "population-statistics Admission V3 source is absent".to_owned())?;
    if reopened != source.audit {
        return Err(
            "population-statistics Admission V3 source differs from fresh reopen".to_owned(),
        );
    }
    Ok(reopened)
}

fn admission_candidate_projection_v3(
    authority: &PopulationStatisticsAdmissionProjectionV3,
    candidate: &PopulationStatisticsCandidateV2,
) -> Result<PopulationStatisticsAdmissionCandidateV3, PopulationStatisticsV2Refusal> {
    let source = authority.source();
    let draft = admission_statistics_draft_v3(source, candidate, authority)?;
    Ok(PopulationStatisticsAdmissionCandidateV3 {
        projection: PopulationStatisticsV2CandidateProjection {
            source: *source,
            candidate: *candidate,
        },
        draft,
    })
}

fn admission_statistics_draft_v3(
    source: &PopulationStatisticsV2ProjectionSource,
    candidate: &PopulationStatisticsCandidateV2,
    authority: &PopulationStatisticsAdmissionProjectionV3,
) -> Result<AdmissionStatisticsDraftV3, PopulationStatisticsV2Refusal> {
    if authority.source() != source {
        return Err("population-statistics Admission V3 authority source differs".to_owned());
    }
    let pbo = source.cscv_pbo();
    let white = source.white();
    let spa = source.spa();
    Ok(AdmissionStatisticsDraftV3 {
        candidate_semantic_id: candidate.candidate_semantic_digest,
        statistics_audit_id: source.audit_id(),
        statistics_completion_digest: source.completion_record_digest(),
        observation_statistics_link_id: source.observation_statistics_link_id(),
        cscv_policy_digest: source.cscv_policy_digest(),
        cscv_split_family_digest: source.cscv_split_family_digest(),
        white_family_digest: white.family_digest(),
        spa_family_digest: spa.family_digest(),
        romano_wolf_family_digest: source.romano_wolf_family_digest(),
        trades: candidate.trades,
        wins: candidate.wins,
        wilson_lower_bits: candidate.wilson_lower_bits,
        wilson_win_rate_ppm: admission_wilson_ppm_v3(candidate.wilson_lower_bits)?,
        cscv_split_count: source.split_count(),
        pbo_contributing_splits: pbo.contributing_splits(),
        pbo_unrankable_splits: pbo.unrankable_splits(),
        pbo_probability: admission_probability_v3(pbo.exact_probability(), "CSCV/PBO")?,
        white_probability: admission_probability_v3(
            white.exact_probability(),
            "White Reality Check",
        )?,
        spa_probability: admission_probability_v3(spa.exact_probability(), "Hansen SPA")?,
        familywise_romano_wolf_probability: admission_probability_v3(
            authority.familywise_romano_wolf_probability(),
            "familywise Romano-Wolf",
        )?,
        candidate_romano_wolf_probability: admission_probability_v3(
            candidate.romano_wolf_adjusted,
            "candidate Romano-Wolf",
        )?,
        bootstrap_draws: source.bootstrap_draws(),
        bootstrap_strategies: source.candidate_count(),
        bootstrap_periods: source.period_count(),
    })
}

fn admission_probability_v3(
    value: PopulationStatisticsFractionV2,
    name: &str,
) -> Result<AdmissionExactProbabilityV2, PopulationStatisticsV2Refusal> {
    AdmissionExactProbabilityV2::new(value.numerator(), value.denominator()).map_err(|why| {
        format!("population-statistics Admission V3 {name} probability refused: {why:?}")
    })
}

#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::float_arithmetic,
    reason = "canonical comparison-only floor-ppm projection of Statistics' retained full-precision Wilson value"
)]
fn admission_wilson_ppm_v3(bits: u64) -> Result<u64, PopulationStatisticsV2Refusal> {
    const PPM: u64 = 1_000_000;
    let value = f64::from_bits(bits);
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(
            "population-statistics Admission V3 Wilson value is not finite within [0,1]".to_owned(),
        );
    }
    Ok((value * PPM as f64).floor() as u64)
}

fn validate_candidate_records(
    file: &mut File,
    first: u64,
    manifest: &PopulationStatisticsManifestV2,
) -> Result<Vec<PopulationStatisticsCandidateV2>, PopulationStatisticsV2Refusal> {
    let candidate_count = manifest.candidate_count()?;
    let candidate_capacity = usize_of(candidate_count, "candidate count")?;
    let mut candidates = Vec::new();
    candidates
        .try_reserve_exact(candidate_capacity)
        .map_err(|why| format!("cannot reserve population-statistics candidates: {why}"))?;
    let mut seen = HashMap::new();
    seen.try_reserve(candidate_capacity)
        .map_err(|why| format!("cannot reserve candidate-identity index: {why}"))?;
    for sequence in 0..candidate_count {
        let physical = candidate_record_index(first, sequence)?;
        let decoded = read_record(file, physical)?;
        if decoded.kind != RecordKindV2::Candidate {
            return Err(format!(
                "population-statistics candidate record {physical} has another kind"
            ));
        }
        let candidate = decode_candidate(&decoded.payload)?;
        candidate.validate(manifest)?;
        if candidate.sequence != sequence {
            return Err("population-statistics candidate sequence is reordered".to_owned());
        }
        if seen
            .insert(candidate.candidate_semantic_digest, sequence)
            .is_some()
        {
            return Err("population-statistics duplicate candidate semantic identity".to_owned());
        }
        candidates.push(candidate);
    }
    if ordered_candidate_digest(&candidates)? != manifest.ordered_candidate_digest {
        return Err("population-statistics ordered candidate digest differs".to_owned());
    }
    Ok(candidates)
}

fn validate_period_records(
    file: &mut File,
    first: u64,
    manifest: &PopulationStatisticsManifestV2,
    candidates: &[PopulationStatisticsCandidateV2],
) -> Result<Vec<Vec<i64>>, PopulationStatisticsV2Refusal> {
    let candidate_count = manifest.candidate_count()?;
    let candidate_capacity = usize_of(candidate_count, "candidate count")?;
    let periods = usize_of(manifest.period_count, "period count")?;
    let mut returns = Vec::new();
    returns
        .try_reserve_exact(candidate_capacity)
        .map_err(|why| format!("cannot reserve return-family outer vector: {why}"))?;
    let mut period_hashers = Vec::new();
    period_hashers
        .try_reserve_exact(candidate_capacity)
        .map_err(|why| format!("cannot reserve period hashers: {why}"))?;
    for sequence in 0..candidate_count {
        let mut series = Vec::new();
        series
            .try_reserve_exact(periods)
            .map_err(|why| format!("cannot reserve candidate return periods: {why}"))?;
        returns.push(series);
        period_hashers.push(candidate_source_hasher(PERIOD_ORDER_DOMAIN, sequence));
    }
    let mut all_periods = Hasher::new();
    all_periods.update(PERIOD_ORDER_DOMAIN);
    all_periods.update(&candidate_count.to_le_bytes());
    all_periods.update(&manifest.period_count.to_le_bytes());
    let mut trade_totals = Vec::new();
    trade_totals
        .try_reserve_exact(candidate_capacity)
        .map_err(|why| format!("cannot reserve candidate trade totals: {why}"))?;
    trade_totals.resize(candidate_capacity, (0_u64, 0_u64));
    for period_sequence in 0..manifest.period_count {
        let mut shared_period = None;
        for candidate_sequence in 0..candidate_count {
            let physical =
                period_record_index(first, manifest, candidate_sequence, period_sequence)?;
            let decoded = read_record(file, physical)?;
            if decoded.kind != RecordKindV2::Period {
                return Err("population-statistics period area contains another kind".to_owned());
            }
            let period = decode_period(&decoded.payload)?;
            period.validate(manifest, candidates)?;
            if (period.candidate_sequence, period.period_sequence)
                != (candidate_sequence, period_sequence)
            {
                return Err("population-statistics period-major order differs".to_owned());
            }
            match shared_period {
                None => shared_period = Some(period.period_identity),
                Some(identity) if identity == period.period_identity => {}
                Some(_) => {
                    return Err(
                        "population-statistics candidates do not share aligned period identity"
                            .to_owned(),
                    );
                }
            }
            hash_period(&mut all_periods, period);
            hash_period(
                period_hashers
                    .get_mut(usize_of(candidate_sequence, "candidate sequence")?)
                    .ok_or_else(|| "candidate period hasher is absent".to_owned())?,
                period,
            );
            returns
                .get_mut(usize_of(candidate_sequence, "candidate sequence")?)
                .ok_or_else(|| "candidate return series is absent".to_owned())?
                .push(period.return_paisa);
            let totals = trade_totals
                .get_mut(usize_of(candidate_sequence, "candidate sequence")?)
                .ok_or_else(|| "candidate trade total is absent".to_owned())?;
            totals.0 = totals
                .0
                .checked_add(period.trades)
                .ok_or_else(|| "population-statistics trade total overflowed".to_owned())?;
            totals.1 = totals
                .1
                .checked_add(period.wins)
                .ok_or_else(|| "population-statistics win total overflowed".to_owned())?;
        }
    }
    if all_periods.finalize() != manifest.ordered_period_digest {
        return Err("population-statistics ordered period digest differs".to_owned());
    }
    for (sequence, ((candidate, totals), hasher)) in candidates
        .iter()
        .zip(&trade_totals)
        .zip(period_hashers)
        .enumerate()
    {
        if (candidate.trades, candidate.wins) != *totals
            || candidate.ordered_period_digest != hasher.finalize()
        {
            return Err(format!(
                "population-statistics candidate {sequence} period totals/digest differ"
            ));
        }
    }
    Ok(returns)
}

fn validate_split_records(
    file: &mut File,
    first: u64,
    manifest: &PopulationStatisticsManifestV2,
    candidates: &[PopulationStatisticsCandidateV2],
) -> Result<(), PopulationStatisticsV2Refusal> {
    let candidate_count = manifest.candidate_count()?;
    let candidate_capacity = usize_of(candidate_count, "candidate count")?;
    let mut split_hashers = Vec::new();
    split_hashers
        .try_reserve_exact(candidate_capacity)
        .map_err(|why| format!("cannot reserve split hashers: {why}"))?;
    for sequence in 0..candidate_count {
        split_hashers.push(candidate_source_hasher(SPLIT_ORDER_DOMAIN, sequence));
    }
    let mut all_splits = Hasher::new();
    all_splits.update(SPLIT_ORDER_DOMAIN);
    all_splits.update(&candidate_count.to_le_bytes());
    all_splits.update(&manifest.split_count.to_le_bytes());
    let mut expected_mask = first_train_mask(manifest.segment_count)?;
    let mut bottom_half = 0_u64;
    let mut contributing = 0_u64;
    for split_sequence in 0..manifest.split_count {
        let mut train_scores = Vec::new();
        let mut test_scores = Vec::new();
        train_scores
            .try_reserve_exact(candidate_capacity)
            .map_err(|why| format!("cannot reserve CSCV train scores: {why}"))?;
        test_scores
            .try_reserve_exact(candidate_capacity)
            .map_err(|why| format!("cannot reserve CSCV test scores: {why}"))?;
        for candidate_sequence in 0..candidate_count {
            let physical = split_record_index(first, manifest, candidate_sequence, split_sequence)?;
            let decoded = read_record(file, physical)?;
            if decoded.kind != RecordKindV2::Split {
                return Err("population-statistics split area contains another kind".to_owned());
            }
            let split = decode_split(&decoded.payload)?;
            split.validate(manifest, candidates, expected_mask)?;
            if (split.candidate_sequence, split.split_sequence)
                != (candidate_sequence, split_sequence)
            {
                return Err("population-statistics split-major order differs".to_owned());
            }
            hash_split(&mut all_splits, split);
            hash_split(
                split_hashers
                    .get_mut(usize_of(candidate_sequence, "candidate sequence")?)
                    .ok_or_else(|| "candidate split hasher is absent".to_owned())?,
                split,
            );
            train_scores.push(split.train_score);
            test_scores.push(split.test_score);
        }
        let (is_bottom, is_rankable) = cscv_placement(&train_scores, &test_scores)?;
        if is_rankable {
            contributing = contributing
                .checked_add(1)
                .ok_or_else(|| "CSCV contributing count overflowed".to_owned())?;
            if is_bottom {
                bottom_half = bottom_half
                    .checked_add(1)
                    .ok_or_else(|| "CSCV bottom-half count overflowed".to_owned())?;
            }
        }
        if split_sequence
            .checked_add(1)
            .ok_or_else(|| "CSCV split sequence overflowed".to_owned())?
            < manifest.split_count
        {
            expected_mask = next_train_mask(expected_mask, manifest.segment_count)?;
        }
    }
    if all_splits.finalize() != manifest.ordered_split_digest {
        return Err("population-statistics ordered split digest differs".to_owned());
    }
    for (sequence, (candidate, hasher)) in candidates.iter().zip(split_hashers).enumerate() {
        if candidate.ordered_split_digest != hasher.finalize() {
            return Err(format!(
                "population-statistics candidate {sequence} split digest differs"
            ));
        }
    }
    let pbo = pbo_from_counts(manifest.split_count, contributing, bottom_half)?;
    if manifest.pbo != pbo {
        return Err("population-statistics CSCV/PBO does not recompute".to_owned());
    }
    Ok(())
}

fn validate_complete_block(
    file: &mut File,
    first: u64,
    manifest: &PopulationStatisticsManifestV2,
) -> Result<PopulationStatisticsV2ReopenAudit, PopulationStatisticsV2Refusal> {
    let candidates = validate_candidate_records(file, first, manifest)?;
    let returns = validate_period_records(file, first, manifest, &candidates)?;
    validate_split_records(file, first, manifest, &candidates)?;
    verify_bootstrap(manifest, &candidates, &returns)?;
    let candidate_count = manifest.candidate_count()?;

    let completion_ordinal = manifest
        .block_record_count()?
        .checked_sub(1)
        .ok_or_else(|| "population-statistics block lacks a completion ordinal".to_owned())?;
    let completion_index = first
        .checked_add(completion_ordinal)
        .ok_or_else(|| "population-statistics completion index overflowed".to_owned())?;
    let decoded = read_record(file, completion_index)?;
    if decoded.kind != RecordKindV2::Completion {
        return Err("population-statistics block lacks receipt-last Completion".to_owned());
    }
    let completion = decode_manifest(&decoded.payload, decoded.kind)?;
    if completion != *manifest {
        return Err("population-statistics Completion differs from Data semantics".to_owned());
    }
    Ok(PopulationStatisticsV2ReopenAudit {
        first_record: first,
        candidate_count,
        completion_record_digest: digest_completion_record(&decoded.payload),
        manifest: *manifest,
    })
}

fn verify_bootstrap(
    manifest: &PopulationStatisticsManifestV2,
    candidates: &[PopulationStatisticsCandidateV2],
    returns: &[Vec<i64>],
) -> Result<(), PopulationStatisticsV2Refusal> {
    let draws = usize_of(manifest.draws, "bootstrap draws")?;
    let block = usize_of(manifest.block_length, "bootstrap block")?;
    let white =
        white_reality_check_receipt_v1(returns, draws, manifest.seed, block).ok_or_else(|| {
            "population-statistics raw periods cannot produce White evidence".to_owned()
        })?;
    let spa = spa_receipt_v1(returns, draws, manifest.seed, block).ok_or_else(|| {
        "population-statistics raw periods cannot produce SPA evidence".to_owned()
    })?;
    let romano = romano_wolf_adjusted_p_values_v1(returns, draws, manifest.seed, block)
        .ok_or_else(|| {
            "population-statistics raw periods cannot produce Romano-Wolf evidence".to_owned()
        })?;
    if manifest.white != family_test_from_white(white)?
        || manifest.spa != family_test_from_spa(spa)?
        || manifest.romano_wolf_family_digest != romano.family_digest()
        || romano.strategies() != candidates.len()
        || romano.periods() != usize_of(manifest.period_count, "period count")?
        || romano.draws() != draws
        || romano.seed() != manifest.seed
        || romano.block() != block
    {
        return Err("population-statistics family bootstrap evidence differs".to_owned());
    }
    let denominator = manifest
        .draws
        .checked_add(1)
        .ok_or_else(|| "population-statistics bootstrap denominator overflowed".to_owned())?;
    let mut seen_ranks = Vec::new();
    seen_ranks
        .try_reserve_exact(candidates.len())
        .map_err(|why| format!("cannot reserve Romano-Wolf rank index: {why}"))?;
    seen_ranks.resize(candidates.len(), false);
    for (sequence, candidate) in candidates.iter().copied().enumerate() {
        let measured = romano
            .candidate(sequence)
            .ok_or_else(|| "Romano-Wolf candidate is absent".to_owned())?;
        let initial = PopulationStatisticsFractionV2::new(
            u64_of(
                measured.initial_p_value().numerator(),
                "Romano-Wolf initial numerator",
            )?,
            u64_of(
                measured.initial_p_value().denominator(),
                "Romano-Wolf initial denominator",
            )?,
        )?;
        let adjusted = PopulationStatisticsFractionV2::new(
            u64_of(
                measured.adjusted_p_value().numerator(),
                "Romano-Wolf adjusted numerator",
            )?,
            u64_of(
                measured.adjusted_p_value().denominator(),
                "Romano-Wolf adjusted denominator",
            )?,
        )?;
        let rank = u64_of(measured.stepdown_rank(), "Romano-Wolf rank")?;
        let rank_slot = seen_ranks
            .get_mut(usize_of(rank, "Romano-Wolf rank")?)
            .ok_or_else(|| "Romano-Wolf rank is outside family".to_owned())?;
        if *rank_slot {
            return Err("population-statistics duplicate Romano-Wolf rank".to_owned());
        }
        *rank_slot = true;
        if measured.strategy() != sequence
            || candidate.romano_wolf_statistic_bits != measured.observed_statistic().to_bits()
            || candidate.romano_wolf_rank != rank
            || candidate.romano_wolf_strict_exceedances
                != u64_of(
                    measured.strict_exceedances(),
                    "Romano-Wolf strict exceedances",
                )?
            || candidate.romano_wolf_initial != initial
            || candidate.romano_wolf_adjusted != adjusted
            || initial.denominator != denominator
            || adjusted.denominator != denominator
        {
            return Err(format!(
                "population-statistics candidate {sequence} Romano-Wolf evidence differs"
            ));
        }
    }
    if seen_ranks.contains(&false) {
        return Err("population-statistics Romano-Wolf rank permutation is incomplete".to_owned());
    }
    Ok(())
}

fn validate_orphan_prefix(
    file: &mut File,
    first: u64,
    present: u64,
    manifest: &PopulationStatisticsManifestV2,
) -> Result<(), PopulationStatisticsV2Refusal> {
    if present == 0 || present >= manifest.block_record_count()? {
        return Err("population-statistics orphan prefix length is invalid".to_owned());
    }
    let candidate_count = manifest.candidate_count()?;
    let candidate_end = 1_u64
        .checked_add(candidate_count)
        .ok_or_else(|| "orphan candidate end overflowed".to_owned())?;
    let period_records = candidate_count
        .checked_mul(manifest.period_count)
        .ok_or_else(|| "orphan period records overflowed".to_owned())?;
    let period_end = candidate_end
        .checked_add(period_records)
        .ok_or_else(|| "orphan period end overflowed".to_owned())?;
    let present_after_data = present
        .checked_sub(1)
        .ok_or_else(|| "population-statistics orphan lacks Data".to_owned())?;
    let mut candidates = Vec::new();
    candidates
        .try_reserve_exact(usize_of(
            candidate_count.min(present_after_data),
            "orphan candidates",
        )?)
        .map_err(|why| format!("cannot reserve orphan candidates: {why}"))?;
    let mut expected_split_mask = first_train_mask(manifest.segment_count)?;
    for relative in 1..present {
        let physical = first
            .checked_add(relative)
            .ok_or_else(|| "orphan physical index overflowed".to_owned())?;
        let decoded = read_record(file, physical)?;
        if relative < candidate_end {
            if decoded.kind != RecordKindV2::Candidate {
                return Err("population-statistics orphan candidate prefix changed kind".to_owned());
            }
            let candidate = decode_candidate(&decoded.payload)?;
            candidate.validate(manifest)?;
            if candidate.sequence != relative - 1 {
                return Err("population-statistics orphan candidate order differs".to_owned());
            }
            candidates.push(candidate);
        } else if relative < period_end {
            if candidates.len() != usize_of(candidate_count, "candidate count")? {
                return Err("population-statistics orphan skipped candidate rows".to_owned());
            }
            let ordinal = relative - candidate_end;
            let period_sequence = ordinal / candidate_count;
            let candidate_sequence = ordinal % candidate_count;
            if decoded.kind != RecordKindV2::Period {
                return Err("population-statistics orphan period prefix changed kind".to_owned());
            }
            let period = decode_period(&decoded.payload)?;
            period.validate(manifest, &candidates)?;
            if (period.candidate_sequence, period.period_sequence)
                != (candidate_sequence, period_sequence)
            {
                return Err("population-statistics orphan period order differs".to_owned());
            }
        } else {
            if candidates.len() != usize_of(candidate_count, "candidate count")? {
                return Err("population-statistics orphan skipped candidate rows".to_owned());
            }
            let ordinal = relative - period_end;
            let split_sequence = ordinal / candidate_count;
            let candidate_sequence = ordinal % candidate_count;
            if decoded.kind != RecordKindV2::Split {
                return Err("population-statistics orphan split prefix changed kind".to_owned());
            }
            let split = decode_split(&decoded.payload)?;
            split.validate(manifest, &candidates, expected_split_mask)?;
            if (split.candidate_sequence, split.split_sequence)
                != (candidate_sequence, split_sequence)
            {
                return Err("population-statistics orphan split order differs".to_owned());
            }
            if candidate_sequence
                .checked_add(1)
                .ok_or_else(|| "orphan candidate sequence overflowed".to_owned())?
                == candidate_count
                && split_sequence
                    .checked_add(1)
                    .ok_or_else(|| "orphan split sequence overflowed".to_owned())?
                    < manifest.split_count
            {
                expected_split_mask = next_train_mask(expected_split_mask, manifest.segment_count)?;
            }
        }
    }
    Ok(())
}

fn read_candidate_at(
    file: &mut File,
    first: u64,
    manifest: &PopulationStatisticsManifestV2,
    sequence: u64,
) -> Result<PopulationStatisticsCandidateV2, PopulationStatisticsV2Refusal> {
    let physical = candidate_record_index(first, sequence)?;
    let decoded = read_record(file, physical)?;
    if decoded.kind != RecordKindV2::Candidate {
        return Err("population-statistics candidate offset changed kind".to_owned());
    }
    let candidate = decode_candidate(&decoded.payload)?;
    candidate.validate(manifest)?;
    if candidate.sequence != sequence {
        return Err("population-statistics candidate offset changed sequence".to_owned());
    }
    Ok(candidate)
}

fn candidate_record_index(
    first: u64,
    candidate_sequence: u64,
) -> Result<u64, PopulationStatisticsV2Refusal> {
    first
        .checked_add(1)
        .and_then(|index| index.checked_add(candidate_sequence))
        .ok_or_else(|| "population-statistics candidate record offset overflowed".to_owned())
}

fn period_record_index(
    first: u64,
    manifest: &PopulationStatisticsManifestV2,
    candidate_sequence: u64,
    period_sequence: u64,
) -> Result<u64, PopulationStatisticsV2Refusal> {
    let candidates = manifest.candidate_count()?;
    first
        .checked_add(1)
        .and_then(|index| index.checked_add(candidates))
        .and_then(|index| index.checked_add(period_sequence.checked_mul(candidates)?))
        .and_then(|index| index.checked_add(candidate_sequence))
        .ok_or_else(|| "population-statistics period record offset overflowed".to_owned())
}

fn split_record_index(
    first: u64,
    manifest: &PopulationStatisticsManifestV2,
    candidate_sequence: u64,
    split_sequence: u64,
) -> Result<u64, PopulationStatisticsV2Refusal> {
    let candidates = manifest.candidate_count()?;
    let periods = candidates
        .checked_mul(manifest.period_count)
        .ok_or_else(|| "population-statistics period area overflowed".to_owned())?;
    first
        .checked_add(1)
        .and_then(|index| index.checked_add(candidates))
        .and_then(|index| index.checked_add(periods))
        .and_then(|index| index.checked_add(split_sequence.checked_mul(candidates)?))
        .and_then(|index| index.checked_add(candidate_sequence))
        .ok_or_else(|| "population-statistics split record offset overflowed".to_owned())
}

#[derive(Clone, Debug)]
struct PreAdmissionSourceV2 {
    binding: PreAdmissionBindingV2,
    rung_seconds: u32,
    horizon_bars: u32,
    requested_span: RequestedSpanIdentityV1,
    feed_digest: [u8; 32],
    source_commit_digest: [u8; 32],
    calendar_policy_digest: [u8; 32],
    daily_reference_policy_digest: [u8; 32],
}

impl PreAdmissionSourceV2 {
    fn from_audit(audit: &PreAdmissionDataReopenAuditV1) -> Self {
        let value = audit.value();
        Self {
            binding: PreAdmissionBindingV2::from_value(&value),
            rung_seconds: value.rung_seconds(),
            horizon_bars: value.horizon_bars(),
            requested_span: value.requested_span(),
            feed_digest: value.feed_digest(),
            source_commit_digest: value.source_commit_digest(),
            calendar_policy_digest: value.calendar_policy_digest(),
            daily_reference_policy_digest: value.daily_reference_policy_digest(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ObservationAuthorityFactsV2 {
    authority_id: [u8; 32],
    pair_identity: [u8; 32],
    source_identity: [u8; 32],
    observation_policy_digest: [u8; 32],
    layout_policy_digest: [u8; 32],
    candidate_count: u64,
    period_count: u64,
    split_count: u64,
    split_score_row_count: u64,
    data_record_digest: [u8; 32],
    completion_digest: [u8; 32],
}

impl ObservationAuthorityFactsV2 {
    const fn from_audit(audit: &ObservationAuthorityAuditV1) -> Self {
        Self {
            authority_id: audit.authority_id(),
            pair_identity: audit.pair_identity(),
            source_identity: audit.source_identity(),
            observation_policy_digest: audit.observation_policy_digest(),
            layout_policy_digest: audit.layout_policy_digest(),
            candidate_count: audit.candidate_count(),
            period_count: audit.period_count(),
            split_count: audit.split_count(),
            split_score_row_count: audit.split_score_row_count(),
            data_record_digest: audit.data_record_digest(),
            completion_digest: audit.completion_digest(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct RawPeriodV2 {
    identity: [u8; 32],
    return_paisa: i64,
    trades: u64,
    wins: u64,
}

#[derive(Clone, Debug)]
struct RawCandidateV2 {
    family: InstrumentFamilyV1,
    semantic_digest: [u8; 32],
    periods: Vec<RawPeriodV2>,
}

#[derive(Clone, Debug)]
struct RawSplitV2 {
    train_mask: u64,
    test_mask: u64,
    train_scores: Vec<i64>,
    test_scores: Vec<i64>,
}

/// Opaque complete Statistics V2 block accepted by the receipt-last writer.
///
/// All fields are private.  The production constructor derives this value only
/// from typed Pre-Admission and durable paired Observation authorities; it does
/// not accept caller-authored digests, split scores or retained statistics.
#[derive(Clone, Debug)]
pub struct PreparedPopulationStatisticsV2 {
    manifest: PopulationStatisticsManifestV2,
    candidates: Vec<PopulationStatisticsCandidateV2>,
    periods: Vec<PopulationStatisticsPeriodSourceV2>,
    splits: Vec<PopulationStatisticsSplitSourceV2>,
}

/// Opaque typed Observation-to-Statistics preparation.
///
/// The private Statistics V2 bytes stay coupled to the exact two
/// Pre-Admission reopen audits and to the detached observation link until a
/// fresh receipt-last append succeeds.
#[derive(Clone, Debug)]
pub struct PreparedObservationStatisticsV2 {
    prepared: PreparedPopulationStatisticsV2,
    nifty_pre_admission: PreAdmissionDataReopenAuditV1,
    banknifty_pre_admission: PreAdmissionDataReopenAuditV1,
    link: PopulationStatisticsObservationLinkV2,
}

impl PreparedObservationStatisticsV2 {
    /// Opaque cross-authority link that a later finalization receipt must retain.
    #[must_use]
    pub const fn link(&self) -> PopulationStatisticsObservationLinkV2 {
        self.link
    }

    /// Appends existing Statistics V2 bytes receipt-last and freshly reopens them.
    ///
    /// The reopened audit is checked again against both exact Pre-Admission
    /// audits and the detached Observation link before success is returned.
    ///
    /// # Errors
    ///
    /// Returns any bounded Statistics V2 append/reopen refusal, foreign
    /// Pre-Admission binding, or foreign Observation-to-Statistics link.
    pub fn append_and_reopen(
        &self,
        root: impl AsRef<Path>,
        bounds: PopulationStatisticsV2Bounds,
    ) -> Result<PopulationStatisticsObservationCommitV2, PopulationStatisticsV2Refusal> {
        let append = append_population_statistics_v2(root, bounds, &self.prepared)?;
        let audit = append.audit();
        audit.verify_pre_admission_pair(self.nifty_pre_admission, self.banknifty_pre_admission)?;
        self.link.require_reopened_statistics(audit)?;
        Ok(PopulationStatisticsObservationCommitV2 {
            statistics: append,
            link: self.link,
        })
    }
}

/// Freshly reopened Statistics V2 audit plus its unforgeable Observation link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationStatisticsObservationCommitV2 {
    statistics: PopulationStatisticsV2Append,
    link: PopulationStatisticsObservationLinkV2,
}

impl PopulationStatisticsObservationCommitV2 {
    /// Freshly reopened Statistics V2 audit.
    #[must_use]
    pub const fn audit(self) -> PopulationStatisticsV2ReopenAudit {
        self.statistics.audit()
    }

    /// Exact detached Observation-to-Statistics link.
    #[must_use]
    pub const fn link(self) -> PopulationStatisticsObservationLinkV2 {
        self.link
    }

    /// Produces immutable Statistics-side source terms for a later projection.
    ///
    /// # Errors
    ///
    /// Refuses if the detached Observation link does not name this exact fresh
    /// Statistics reopen.  The returned value is not an Admission or
    /// Finalization capability.
    pub fn projection_source(
        self,
    ) -> Result<PopulationStatisticsV2ProjectionSource, PopulationStatisticsV2Refusal> {
        let audit = self.audit();
        self.link.require_reopened_statistics(audit)?;
        require_nonzero(
            "receipt-last Completion record",
            audit.completion_record_digest(),
        )?;
        Ok(PopulationStatisticsV2ProjectionSource {
            audit,
            observation_link: self.link,
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct StatisticsProcedureInputsV2 {
    segment_count: u32,
    draws: u64,
    seed: u64,
    block_length: u64,
}

fn validate_raw_shape(
    nifty: &PreAdmissionSourceV2,
    banknifty: &PreAdmissionSourceV2,
    raw_candidates: &[RawCandidateV2],
    raw_splits: &[RawSplitV2],
    segment_count: u32,
) -> Result<(u64, usize, u64), PopulationStatisticsV2Refusal> {
    validate_source_pair(nifty, banknifty)?;
    let candidate_count = nifty
        .binding
        .candidate_count
        .checked_add(banknifty.binding.candidate_count)
        .ok_or_else(|| "fixture candidate count overflowed".to_owned())?;
    if usize_of(candidate_count, "candidate count")? != raw_candidates.len() {
        return Err("population-statistics raw candidate cardinality differs".to_owned());
    }
    let period_count = raw_candidates
        .first()
        .map(|candidate| candidate.periods.len())
        .ok_or_else(|| "population-statistics raw family is empty".to_owned())?;
    if period_count < 2
        || raw_candidates
            .iter()
            .any(|candidate| candidate.periods.len() != period_count)
    {
        return Err("population-statistics raw periods are empty or misaligned".to_owned());
    }
    let split_count = canonical_split_count(segment_count)?;
    if usize_of(split_count, "split count")? != raw_splits.len() {
        return Err("population-statistics raw CSCV family is incomplete".to_owned());
    }
    let mut seen_candidates = HashMap::new();
    seen_candidates
        .try_reserve(raw_candidates.len())
        .map_err(|why| format!("cannot reserve raw candidate index: {why}"))?;
    for (sequence, candidate) in raw_candidates.iter().enumerate() {
        let sequence_u64 = u64_of(sequence, "candidate sequence")?;
        let expected = if sequence_u64 < nifty.binding.candidate_count {
            InstrumentFamilyV1::Nifty
        } else {
            InstrumentFamilyV1::BankNifty
        };
        if candidate.family != expected {
            return Err("population-statistics candidates are not NIFTY then BANKNIFTY".to_owned());
        }
        require_nonzero("raw candidate semantic", candidate.semantic_digest)?;
        if seen_candidates
            .insert(candidate.semantic_digest, sequence)
            .is_some()
        {
            return Err("population-statistics raw candidate identity is duplicated".to_owned());
        }
        for (period_sequence, period) in candidate.periods.iter().enumerate() {
            if period.wins > period.trades {
                return Err("population-statistics raw period wins exceed trades".to_owned());
            }
            let shared = raw_candidates
                .first()
                .and_then(|first| first.periods.get(period_sequence))
                .ok_or_else(|| "shared raw period is absent".to_owned())?;
            if shared.identity != period.identity {
                return Err("population-statistics raw period identities are misaligned".to_owned());
            }
            require_nonzero("raw period identity", period.identity)?;
        }
    }
    let mut expected_mask = first_train_mask(segment_count)?;
    for (split_sequence, split) in raw_splits.iter().enumerate() {
        if split.train_mask != expected_mask
            || split.test_mask != segment_mask(segment_count)? ^ expected_mask
            || split.train_scores.len() != raw_candidates.len()
            || split.test_scores.len() != raw_candidates.len()
        {
            return Err("population-statistics raw CSCV split differs".to_owned());
        }
        if split_sequence.saturating_add(1) < raw_splits.len() {
            expected_mask = next_train_mask(expected_mask, segment_count)?;
        }
    }
    Ok((candidate_count, period_count, split_count))
}

fn build_raw_periods(
    raw_candidates: &[RawCandidateV2],
    candidate_count: u64,
    period_count: usize,
) -> Result<Vec<PopulationStatisticsPeriodSourceV2>, PopulationStatisticsV2Refusal> {
    let mut periods = Vec::new();
    let raw_period_records = candidate_count
        .checked_mul(u64_of(period_count, "period count")?)
        .ok_or_else(|| "raw period record count overflowed".to_owned())?;
    periods
        .try_reserve_exact(usize_of(raw_period_records, "raw period records")?)
        .map_err(|why| format!("cannot reserve raw period records: {why}"))?;
    for period_sequence in 0..period_count {
        for (candidate_sequence, candidate) in raw_candidates.iter().enumerate() {
            let period = candidate
                .periods
                .get(period_sequence)
                .copied()
                .ok_or_else(|| "raw period disappeared".to_owned())?;
            periods.push(PopulationStatisticsPeriodSourceV2 {
                audit_id: [0; 32],
                candidate_sequence: u64_of(candidate_sequence, "candidate sequence")?,
                period_sequence: u64_of(period_sequence, "period sequence")?,
                candidate_semantic_digest: candidate.semantic_digest,
                return_paisa: period.return_paisa,
                trades: period.trades,
                wins: period.wins,
                period_identity: period.identity,
            });
        }
    }
    Ok(periods)
}

fn build_raw_splits(
    raw_candidates: &[RawCandidateV2],
    raw_splits: &[RawSplitV2],
    candidate_count: u64,
    split_count: u64,
    segment_count: u32,
) -> Result<Vec<PopulationStatisticsSplitSourceV2>, PopulationStatisticsV2Refusal> {
    let mut splits = Vec::new();
    let raw_split_records = candidate_count
        .checked_mul(split_count)
        .ok_or_else(|| "raw split record count overflowed".to_owned())?;
    splits
        .try_reserve_exact(usize_of(raw_split_records, "raw split records")?)
        .map_err(|why| format!("cannot reserve raw split records: {why}"))?;
    for (split_sequence, split) in raw_splits.iter().enumerate() {
        for (candidate_sequence, candidate) in raw_candidates.iter().enumerate() {
            splits.push(PopulationStatisticsSplitSourceV2 {
                audit_id: [0; 32],
                candidate_sequence: u64_of(candidate_sequence, "candidate sequence")?,
                split_sequence: u64_of(split_sequence, "split sequence")?,
                candidate_semantic_digest: candidate.semantic_digest,
                segment_count,
                train_mask: split.train_mask,
                test_mask: split.test_mask,
                train_score: *split
                    .train_scores
                    .get(candidate_sequence)
                    .ok_or_else(|| "raw train score is absent".to_owned())?,
                test_score: *split
                    .test_scores
                    .get(candidate_sequence)
                    .ok_or_else(|| "raw test score is absent".to_owned())?,
                split_identity: split_identity(segment_count, split.train_mask, split.test_mask),
            });
        }
    }
    Ok(splits)
}

fn build_raw_candidates(
    nifty: &PreAdmissionSourceV2,
    banknifty: &PreAdmissionSourceV2,
    raw_candidates: &[RawCandidateV2],
    periods: &[PopulationStatisticsPeriodSourceV2],
    splits: &[PopulationStatisticsSplitSourceV2],
    romano: &RomanoWolfAdjustedReceiptV1,
) -> Result<Vec<PopulationStatisticsCandidateV2>, PopulationStatisticsV2Refusal> {
    let mut candidates = Vec::new();
    candidates
        .try_reserve_exact(raw_candidates.len())
        .map_err(|why| format!("cannot reserve candidate summaries: {why}"))?;
    for (sequence, raw) in raw_candidates.iter().enumerate() {
        let sequence_u64 = u64_of(sequence, "candidate sequence")?;
        let family_sequence = if raw.family == InstrumentFamilyV1::Nifty {
            sequence_u64
        } else {
            sequence_u64
                .checked_sub(nifty.binding.candidate_count)
                .ok_or_else(|| "BANKNIFTY family sequence underflowed".to_owned())?
        };
        let (trades, wins) = raw
            .periods
            .iter()
            .try_fold((0_u64, 0_u64), |(trades, wins), period| {
                Some((
                    trades.checked_add(period.trades)?,
                    wins.checked_add(period.wins)?,
                ))
            })
            .ok_or_else(|| "candidate period totals overflowed".to_owned())?;
        let measured = romano
            .candidate(sequence)
            .ok_or_else(|| "raw Romano-Wolf candidate is absent".to_owned())?;
        let candidate_periods: Vec<_> = periods
            .iter()
            .copied()
            .filter(|period| period.candidate_sequence == sequence_u64)
            .collect();
        let candidate_splits: Vec<_> = splits
            .iter()
            .copied()
            .filter(|split| split.candidate_sequence == sequence_u64)
            .collect();
        candidates.push(PopulationStatisticsCandidateV2 {
            audit_id: [0; 32],
            sequence: sequence_u64,
            family: raw.family,
            family_sequence,
            candidate_semantic_digest: raw.semantic_digest,
            pre_admission_id: if raw.family == InstrumentFamilyV1::Nifty {
                nifty.binding.authority_id
            } else {
                banknifty.binding.authority_id
            },
            trades,
            wins,
            wilson_lower_bits: wilson_lower_bits(wins, trades),
            romano_wolf_statistic_bits: measured.observed_statistic().to_bits(),
            romano_wolf_rank: u64_of(measured.stepdown_rank(), "Romano-Wolf rank")?,
            romano_wolf_strict_exceedances: u64_of(
                measured.strict_exceedances(),
                "Romano-Wolf strict exceedances",
            )?,
            romano_wolf_initial: PopulationStatisticsFractionV2::new(
                u64_of(measured.initial_p_value().numerator(), "initial numerator")?,
                u64_of(
                    measured.initial_p_value().denominator(),
                    "initial denominator",
                )?,
            )?,
            romano_wolf_adjusted: PopulationStatisticsFractionV2::new(
                u64_of(
                    measured.adjusted_p_value().numerator(),
                    "adjusted numerator",
                )?,
                u64_of(
                    measured.adjusted_p_value().denominator(),
                    "adjusted denominator",
                )?,
            )?,
            ordered_period_digest: ordered_period_digest_for(sequence_u64, &candidate_periods),
            ordered_split_digest: ordered_split_digest_for(sequence_u64, &candidate_splits),
        });
    }
    Ok(candidates)
}

fn pbo_counts_from_raw_splits(
    raw_splits: &[RawSplitV2],
) -> Result<(u64, u64), PopulationStatisticsV2Refusal> {
    let mut contributing = 0_u64;
    let mut bottom = 0_u64;
    for split in raw_splits {
        let (is_bottom, rankable) = cscv_placement(&split.train_scores, &split.test_scores)?;
        if rankable {
            contributing = contributing
                .checked_add(1)
                .ok_or_else(|| "fixture contributing count overflowed".to_owned())?;
            if is_bottom {
                bottom = bottom
                    .checked_add(1)
                    .ok_or_else(|| "fixture bottom count overflowed".to_owned())?;
            }
        }
    }
    Ok((contributing, bottom))
}

impl PreparedPopulationStatisticsV2 {
    fn new(
        nifty: &PreAdmissionSourceV2,
        banknifty: &PreAdmissionSourceV2,
        raw_candidates: &[RawCandidateV2],
        raw_splits: &[RawSplitV2],
        statistics: StatisticsProcedureInputsV2,
    ) -> Result<Self, PopulationStatisticsV2Refusal> {
        let StatisticsProcedureInputsV2 {
            segment_count,
            draws,
            seed,
            block_length,
        } = statistics;
        let (candidate_count, period_count, split_count) =
            validate_raw_shape(nifty, banknifty, raw_candidates, raw_splits, segment_count)?;

        let returns: Vec<Vec<i64>> = raw_candidates
            .iter()
            .map(|candidate| {
                candidate
                    .periods
                    .iter()
                    .map(|period| period.return_paisa)
                    .collect()
            })
            .collect();
        let draws_usize = usize_of(draws, "draws")?;
        let block_usize = usize_of(block_length, "block length")?;
        let white = white_reality_check_receipt_v1(&returns, draws_usize, seed, block_usize)
            .ok_or_else(|| "raw family cannot produce exact White evidence".to_owned())?;
        let spa = spa_receipt_v1(&returns, draws_usize, seed, block_usize)
            .ok_or_else(|| "raw family cannot produce exact SPA evidence".to_owned())?;
        let romano = romano_wolf_adjusted_p_values_v1(&returns, draws_usize, seed, block_usize)
            .ok_or_else(|| "raw family cannot produce exact Romano-Wolf evidence".to_owned())?;

        let mut periods = build_raw_periods(raw_candidates, candidate_count, period_count)?;
        let mut splits = build_raw_splits(
            raw_candidates,
            raw_splits,
            candidate_count,
            split_count,
            segment_count,
        )?;

        let ordered_period_digest = ordered_period_digest(
            &periods,
            candidate_count,
            u64_of(period_count, "period count")?,
        );
        let ordered_split_digest = ordered_split_digest(&splits, candidate_count, split_count);
        let mut candidates =
            build_raw_candidates(nifty, banknifty, raw_candidates, &periods, &splits, &romano)?;
        let (contributing, bottom) = pbo_counts_from_raw_splits(raw_splits)?;
        let mut manifest = PopulationStatisticsManifestV2 {
            sequence: 0,
            audit_id: [0; 32],
            rung_seconds: nifty.rung_seconds,
            horizon_bars: nifty.horizon_bars,
            requested_span: nifty.requested_span,
            segment_count,
            period_count: u64_of(period_count, "period count")?,
            split_count,
            draws,
            seed,
            block_length,
            nifty: nifty.binding,
            banknifty: banknifty.binding,
            feed_digest: nifty.feed_digest,
            source_commit_digest: nifty.source_commit_digest,
            calendar_policy_digest: nifty.calendar_policy_digest,
            daily_reference_policy_digest: nifty.daily_reference_policy_digest,
            wilson_policy_digest: digest_domain(WILSON_POLICY_DOMAIN, &[]),
            cscv_policy_digest: cscv_policy_digest(segment_count)?,
            white: family_test_from_white(white)?,
            spa: family_test_from_spa(spa)?,
            romano_wolf_family_digest: romano.family_digest(),
            pbo: pbo_from_counts(split_count, contributing, bottom)?,
            ordered_candidate_digest: ordered_candidate_digest(&candidates)?,
            ordered_period_digest,
            ordered_split_digest,
        };
        manifest.audit_id = derive_audit_id(manifest)?;
        for candidate in &mut candidates {
            candidate.audit_id = manifest.audit_id;
        }
        for period in &mut periods {
            period.audit_id = manifest.audit_id;
        }
        for split in &mut splits {
            split.audit_id = manifest.audit_id;
        }
        manifest.validate()?;
        for candidate in &candidates {
            candidate.validate(&manifest)?;
        }
        Ok(Self {
            manifest,
            candidates,
            periods,
            splits,
        })
    }

    fn records(
        &self,
        logical_sequence: u64,
        first_physical: u64,
    ) -> Result<Vec<[u8; RECORD_BYTES]>, PopulationStatisticsV2Refusal> {
        let manifest = self.manifest.with_sequence(logical_sequence);
        let record_count = manifest.block_record_count()?;
        let mut records = Vec::new();
        records
            .try_reserve_exact(usize_of(record_count, "prepared records")?)
            .map_err(|why| format!("cannot reserve prepared records: {why}"))?;
        records.push(encode_manifest_record(
            &manifest,
            RecordKindV2::Data,
            first_physical,
        )?);
        for candidate in &self.candidates {
            let physical = first_physical
                .checked_add(u64_of(records.len(), "prepared record count")?)
                .ok_or_else(|| "prepared candidate physical index overflowed".to_owned())?;
            records.push(encode_candidate_record(candidate, &manifest, physical)?);
        }
        for period in &self.periods {
            let physical = first_physical
                .checked_add(u64_of(records.len(), "prepared record count")?)
                .ok_or_else(|| "prepared period physical index overflowed".to_owned())?;
            records.push(encode_period_record(
                period,
                &manifest,
                &self.candidates,
                physical,
            )?);
        }
        let candidate_count = manifest.candidate_count()?;
        let mut expected_split_mask = first_train_mask(manifest.segment_count)?;
        for split in &self.splits {
            let physical = first_physical
                .checked_add(u64_of(records.len(), "prepared record count")?)
                .ok_or_else(|| "prepared split physical index overflowed".to_owned())?;
            records.push(encode_split_record(
                split,
                &manifest,
                &self.candidates,
                expected_split_mask,
                physical,
            )?);
            if split
                .candidate_sequence
                .checked_add(1)
                .ok_or_else(|| "prepared candidate sequence overflowed".to_owned())?
                == candidate_count
                && split
                    .split_sequence
                    .checked_add(1)
                    .ok_or_else(|| "prepared split sequence overflowed".to_owned())?
                    < manifest.split_count
            {
                expected_split_mask = next_train_mask(expected_split_mask, manifest.segment_count)?;
            }
        }
        let physical = first_physical
            .checked_add(u64_of(records.len(), "prepared record count")?)
            .ok_or_else(|| "prepared completion physical index overflowed".to_owned())?;
        records.push(encode_manifest_record(
            &manifest,
            RecordKindV2::Completion,
            physical,
        )?);
        if u64_of(records.len(), "prepared record count")? != record_count {
            return Err("prepared population-statistics record count differs".to_owned());
        }
        Ok(records)
    }
}

/// Recomputes one nonempty evaluated-family projection for Statistics V3.
///
/// The function revalidates the opaque Observation V1 family against its exact
/// Pre-Admission audit, derives the canonical single-family CSCV layout and
/// split scores, and recomputes White, SPA, Romano--Wolf, Wilson and—when at
/// least two Candidates exist—PBO. Callers cannot supply rows, scores,
/// statistics, terminal meaning or identity digests.
///
/// # Cost
///
/// Validation and projection are O(C * (P + S)) time and retained space for C
/// candidates, P aligned periods and S canonical splits, before the explicit
/// finite-bootstrap work. This whole-family preparation is not O(1).
///
/// # Errors
///
/// Refuses a zero family, a foreign Pre-Admission binding, noncanonical
/// order/layout, arithmetic or allocation failure, or a statistical family
/// that cannot produce exact finite-resample evidence.
pub(crate) fn prepare_single_family_statistics_v3(
    observations: &CandidateFamilyObservationsV1,
    pre_admission: &PreAdmissionDataReopenAuditV1,
    procedure: PopulationStatisticsProcedureV2,
) -> Result<SingleFamilyStatisticsV3Projection, PopulationStatisticsV2Refusal> {
    require_observation_pre_admission_source(observations, pre_admission, observations.family())?;
    if observations.candidate_count() == 0 {
        return Err(
            "Statistics V3 evaluated-family projection received a zero Candidate family".to_owned(),
        );
    }
    let (layout, split_rows) = observations.statistics_v3_projection()?;
    let candidate_count = observations.candidate_count();
    let period_count = observations.period_count();
    let split_count = usize_of(layout.split_count(), "Statistics V3 split count")?;
    let expected_splits = candidate_count
        .checked_mul(split_count)
        .ok_or_else(|| "Statistics V3 split cardinality overflowed usize".to_owned())?;
    if split_rows.len() != expected_splits {
        return Err("Statistics V3 single-family split projection is incomplete".to_owned());
    }

    let returns: Vec<Vec<i64>> = observations
        .candidates()
        .iter()
        .map(|candidate| {
            candidate
                .periods()
                .iter()
                .map(|period| period.return_paisa())
                .collect()
        })
        .collect();
    let draws = usize_of(procedure.draws(), "Statistics V3 draws")?;
    let block = usize_of(procedure.block_length(), "Statistics V3 block length")?;
    let white = white_reality_check_receipt_v1(&returns, draws, procedure.seed(), block)
        .ok_or_else(|| "Statistics V3 evaluated family cannot produce White evidence".to_owned())?;
    let spa = spa_receipt_v1(&returns, draws, procedure.seed(), block)
        .ok_or_else(|| "Statistics V3 evaluated family cannot produce SPA evidence".to_owned())?;
    let romano = romano_wolf_adjusted_p_values_v1(&returns, draws, procedure.seed(), block)
        .ok_or_else(|| {
            "Statistics V3 evaluated family cannot produce Romano-Wolf evidence".to_owned()
        })?;
    if romano.strategies() != candidate_count
        || romano.periods() != period_count
        || romano.draws() != draws
        || romano.seed() != procedure.seed()
        || romano.block() != block
    {
        return Err("Statistics V3 recomputed Romano-Wolf shape differs".to_owned());
    }

    let mut candidates = Vec::new();
    candidates
        .try_reserve_exact(candidate_count)
        .map_err(|why| format!("cannot reserve Statistics V3 candidates: {why}"))?;
    let mut family_period_hasher = Hasher::new();
    family_period_hasher.update(STATISTICS_V3_SINGLE_PERIOD_ORDER_DOMAIN);
    let mut family_split_hasher = Hasher::new();
    family_split_hasher.update(STATISTICS_V3_SINGLE_SPLIT_ORDER_DOMAIN);
    for (candidate_index, candidate) in observations.candidates().iter().enumerate() {
        let family_sequence = u64_of(candidate_index, "Statistics V3 family sequence")?;
        let measured = romano
            .candidate(candidate_index)
            .ok_or_else(|| "Statistics V3 Romano-Wolf candidate disappeared".to_owned())?;
        if measured.strategy() != candidate_index {
            return Err("Statistics V3 Romano-Wolf candidate order differs".to_owned());
        }
        let initial = measured.initial_p_value();
        let adjusted = measured.adjusted_p_value();
        let mut candidate_period_hasher = Hasher::new();
        candidate_period_hasher.update(STATISTICS_V3_SINGLE_PERIOD_ORDER_DOMAIN);
        candidate_period_hasher.update(&family_sequence.to_le_bytes());
        for period in candidate.periods() {
            family_period_hasher.update(&family_sequence.to_le_bytes());
            family_period_hasher.update(&period.identity());
            candidate_period_hasher.update(&period.identity());
        }
        let mut candidate_split_hasher = Hasher::new();
        candidate_split_hasher.update(STATISTICS_V3_SINGLE_SPLIT_ORDER_DOMAIN);
        candidate_split_hasher.update(&family_sequence.to_le_bytes());
        for split_index in 0..split_count {
            let row_index = candidate_index
                .checked_mul(split_count)
                .and_then(|index| index.checked_add(split_index))
                .ok_or_else(|| "Statistics V3 split row index overflowed".to_owned())?;
            let row = split_rows
                .get(row_index)
                .copied()
                .ok_or_else(|| "Statistics V3 split row disappeared".to_owned())?;
            if row.family() != observations.family()
                || row.family_candidate_sequence() != family_sequence
                || row.global_candidate_sequence() != family_sequence
                || row.candidate_semantic_digest() != candidate.candidate_semantic_digest()
                || row.split_sequence() != u64_of(split_index, "Statistics V3 split sequence")?
            {
                return Err("Statistics V3 split row coordinates differ".to_owned());
            }
            family_split_hasher.update(&family_sequence.to_le_bytes());
            family_split_hasher.update(&row.identity());
            candidate_split_hasher.update(&row.identity());
        }
        candidates.push(SingleFamilyCandidateStatisticsV3 {
            family_sequence,
            candidate_semantic_digest: candidate.candidate_semantic_digest(),
            trades: candidate.total_trades(),
            wins: candidate.total_wins(),
            wilson_lower_bits: wilson_lower_bits(candidate.total_wins(), candidate.total_trades()),
            romano_wolf_statistic_bits: measured.observed_statistic().to_bits(),
            romano_wolf_rank: u64_of(measured.stepdown_rank(), "Statistics V3 Romano-Wolf rank")?,
            romano_wolf_strict_exceedances: u64_of(
                measured.strict_exceedances(),
                "Statistics V3 Romano-Wolf exceedances",
            )?,
            romano_wolf_initial: PopulationStatisticsFractionV2::new(
                u64_of(initial.numerator(), "Statistics V3 initial numerator")?,
                u64_of(initial.denominator(), "Statistics V3 initial denominator")?,
            )?,
            romano_wolf_adjusted: PopulationStatisticsFractionV2::new(
                u64_of(adjusted.numerator(), "Statistics V3 adjusted numerator")?,
                u64_of(adjusted.denominator(), "Statistics V3 adjusted denominator")?,
            )?,
            ordered_period_digest: candidate_period_hasher.finalize(),
            ordered_split_digest: candidate_split_hasher.finalize(),
        });
    }

    let pbo = if candidate_count < 2 {
        None
    } else {
        let mut contributing = 0_u64;
        let mut bottom = 0_u64;
        for split_index in 0..split_count {
            let mut train = Vec::new();
            let mut test = Vec::new();
            train
                .try_reserve_exact(candidate_count)
                .and_then(|()| test.try_reserve_exact(candidate_count))
                .map_err(|why| format!("cannot reserve Statistics V3 split family: {why}"))?;
            for candidate_index in 0..candidate_count {
                let row_index = candidate_index
                    .checked_mul(split_count)
                    .and_then(|index| index.checked_add(split_index))
                    .ok_or_else(|| "Statistics V3 PBO row index overflowed".to_owned())?;
                let row = split_rows
                    .get(row_index)
                    .copied()
                    .ok_or_else(|| "Statistics V3 PBO row disappeared".to_owned())?;
                train.push(row.train_score_paisa());
                test.push(row.test_score_paisa());
            }
            let (is_bottom, rankable) = cscv_placement(&train, &test)?;
            if rankable {
                contributing = contributing
                    .checked_add(1)
                    .ok_or_else(|| "Statistics V3 contributing count overflowed".to_owned())?;
                if is_bottom {
                    bottom = bottom
                        .checked_add(1)
                        .ok_or_else(|| "Statistics V3 bottom count overflowed".to_owned())?;
                }
            }
        }
        Some(pbo_from_counts(layout.split_count(), contributing, bottom)?)
    };

    let ordered_period_digest = family_period_hasher.finalize();
    let ordered_split_digest = family_split_hasher.finalize();
    let mut candidate_hasher = Hasher::new();
    candidate_hasher.update(STATISTICS_V3_SINGLE_CANDIDATE_ORDER_DOMAIN);
    for candidate in &candidates {
        candidate_hasher.update(&candidate.family_sequence.to_le_bytes());
        candidate_hasher.update(&candidate.candidate_semantic_digest);
        candidate_hasher.update(&candidate.trades.to_le_bytes());
        candidate_hasher.update(&candidate.wins.to_le_bytes());
        candidate_hasher.update(&candidate.wilson_lower_bits.to_le_bytes());
        candidate_hasher.update(&candidate.romano_wolf_statistic_bits.to_le_bytes());
        candidate_hasher.update(&candidate.romano_wolf_rank.to_le_bytes());
        candidate_hasher.update(&candidate.romano_wolf_strict_exceedances.to_le_bytes());
        candidate_hasher.update(&candidate.romano_wolf_initial.numerator.to_le_bytes());
        candidate_hasher.update(&candidate.romano_wolf_initial.denominator.to_le_bytes());
        candidate_hasher.update(&candidate.romano_wolf_adjusted.numerator.to_le_bytes());
        candidate_hasher.update(&candidate.romano_wolf_adjusted.denominator.to_le_bytes());
        candidate_hasher.update(&candidate.ordered_period_digest);
        candidate_hasher.update(&candidate.ordered_split_digest);
    }
    Ok(SingleFamilyStatisticsV3Projection {
        segment_count: layout.segment_count(),
        period_count: layout.period_count(),
        split_count: layout.split_count(),
        layout_digest: layout.digest(),
        white: family_test_from_white(white)?,
        spa: family_test_from_spa(spa)?,
        romano_wolf_family_digest: romano.family_digest(),
        pbo,
        ordered_candidate_digest: candidate_hasher.finalize(),
        ordered_period_digest,
        ordered_split_digest,
        candidates,
    })
}

/// Prepares one complete two-family Statistics V2 block from the exact durable
/// Observation V1 authority and its matching Pre-Admission reopen audits.
///
/// The caller supplies no candidate rows, period returns, split scores,
/// statistics or digests.  All of them are projected and recomputed from the
/// sealed NIFTY-then-BANKNIFTY Observation capability.  Statistics V2 retains
/// its original two-evaluated-family semantics; mixed natural extinction is a
/// separate Statistics V3 authority.
///
/// # Errors
///
/// Refuses a foreign/incomplete Observation audit, a cross-wired or mismatched
/// Pre-Admission source, noncanonical family/layout/order, invalid procedure,
/// arithmetic/allocation failure, or statistical evidence that cannot be
/// recomputed exactly.
pub fn prepare_population_statistics_v2_from_observations(
    observations: &PairedCandidateObservationsV1,
    observation_audit: ObservationAuthorityAuditV1,
    nifty_pre_admission: PreAdmissionDataReopenAuditV1,
    banknifty_pre_admission: PreAdmissionDataReopenAuditV1,
    procedure: PopulationStatisticsProcedureV2,
) -> Result<PreparedObservationStatisticsV2, PopulationStatisticsV2Refusal> {
    observations
        .require_reopened_authority(&observation_audit)
        .map_err(|why| format!("population-statistics Observation authority refused: {why}"))?;
    let observation_facts = ObservationAuthorityFactsV2::from_audit(&observation_audit);

    let nifty_source = PreAdmissionSourceV2::from_audit(&nifty_pre_admission);
    let banknifty_source = PreAdmissionSourceV2::from_audit(&banknifty_pre_admission);
    validate_source_pair(&nifty_source, &banknifty_source)?;
    require_observation_pre_admission_source(
        observations.nifty(),
        &nifty_pre_admission,
        InstrumentFamilyV1::Nifty,
    )?;
    require_observation_pre_admission_source(
        observations.banknifty(),
        &banknifty_pre_admission,
        InstrumentFamilyV1::BankNifty,
    )?;

    let raw_candidates = build_observation_candidates(observations, observation_facts)?;
    let raw_splits = build_observation_splits(observations, &raw_candidates)?;
    let prepared = PreparedPopulationStatisticsV2::new(
        &nifty_source,
        &banknifty_source,
        &raw_candidates,
        &raw_splits,
        StatisticsProcedureInputsV2 {
            segment_count: observations.layout().segment_count(),
            draws: procedure.draws(),
            seed: procedure.seed(),
            block_length: procedure.block_length(),
        },
    )?;
    verify_pre_admission_pair(
        &prepared.manifest,
        &nifty_pre_admission.value(),
        &banknifty_pre_admission.value(),
    )?;
    let link = observation_statistics_link(observation_facts, &prepared.manifest, procedure)?;
    Ok(PreparedObservationStatisticsV2 {
        prepared,
        nifty_pre_admission,
        banknifty_pre_admission,
        link,
    })
}

fn require_observation_pre_admission_source(
    observations: &CandidateFamilyObservationsV1,
    pre_admission: &PreAdmissionDataReopenAuditV1,
    expected_family: InstrumentFamilyV1,
) -> Result<(), PopulationStatisticsV2Refusal> {
    let source = observations.source();
    let value = pre_admission.value();
    let identities = source.identities();
    if source.family() != expected_family
        || value.family() != expected_family
        || source.universe_id() != value.candidate_universe_id()
        || source.content_digest() != value.candidate_completion_digest()
        || source.row_count() != value.candidate_row_count()
        || source.rung_seconds() != value.rung_seconds()
        || source.horizon_bars() != value.horizon_bars()
        || source.requested_span() != value.requested_span()
        || identities.feed_digest() != value.feed_digest()
        || identities.source_commit_digest() != value.source_commit_digest()
        || identities.calendar_policy_digest() != value.calendar_policy_digest()
        || identities.daily_reference_policy_digest() != value.daily_reference_policy_digest()
    {
        return Err(format!(
            "population-statistics {expected_family:?} Observation source differs from reopened Pre-Admission authority"
        ));
    }
    Ok(())
}

fn build_observation_candidates(
    observations: &PairedCandidateObservationsV1,
    authority: ObservationAuthorityFactsV2,
) -> Result<Vec<RawCandidateV2>, PopulationStatisticsV2Refusal> {
    let candidate_count = observations.candidate_count();
    let mut candidates = Vec::new();
    candidates
        .try_reserve_exact(candidate_count)
        .map_err(|why| format!("cannot reserve Observation candidate projection: {why}"))?;
    append_observation_family_candidates(
        &mut candidates,
        observations.nifty(),
        InstrumentFamilyV1::Nifty,
        authority,
    )?;
    append_observation_family_candidates(
        &mut candidates,
        observations.banknifty(),
        InstrumentFamilyV1::BankNifty,
        authority,
    )?;
    if candidates.len() != candidate_count {
        return Err(
            "population-statistics Observation candidate projection is incomplete".to_owned(),
        );
    }
    Ok(candidates)
}

fn append_observation_family_candidates(
    output: &mut Vec<RawCandidateV2>,
    observations: &CandidateFamilyObservationsV1,
    expected_family: InstrumentFamilyV1,
    authority: ObservationAuthorityFactsV2,
) -> Result<(), PopulationStatisticsV2Refusal> {
    if observations.family() != expected_family {
        return Err(
            "population-statistics Observation families are not NIFTY then BANKNIFTY".to_owned(),
        );
    }
    for (candidate_index, candidate) in observations.candidates().iter().enumerate() {
        let expected_candidate = u64_of(candidate_index, "Observation candidate sequence")?;
        if candidate.candidate_sequence() != expected_candidate {
            return Err("population-statistics Observation candidate sequence differs".to_owned());
        }
        let mut periods = Vec::new();
        periods
            .try_reserve_exact(candidate.periods().len())
            .map_err(|why| format!("cannot reserve Observation period projection: {why}"))?;
        for (period_index, period) in candidate.periods().iter().copied().enumerate() {
            let period_sequence = u64_of(period_index, "Observation period sequence")?;
            let expected_day = observations
                .accepted_ist_sessions()
                .get(period_index)
                .copied()
                .ok_or_else(|| "Observation accepted IST session disappeared".to_owned())?;
            if period.candidate_sequence() != expected_candidate
                || period.candidate_semantic_digest() != candidate.candidate_semantic_digest()
                || period.period_sequence() != period_sequence
                || period.exit_ist_day() != expected_day
            {
                return Err(
                    "population-statistics Observation period coordinates differ".to_owned(),
                );
            }
            periods.push(RawPeriodV2 {
                identity: observation_period_identity(authority, period_sequence, expected_day),
                return_paisa: period.return_paisa(),
                trades: period.trades(),
                wins: period.wins(),
            });
        }
        output.push(RawCandidateV2 {
            family: expected_family,
            semantic_digest: candidate.candidate_semantic_digest(),
            periods,
        });
    }
    Ok(())
}

fn build_observation_splits(
    observations: &PairedCandidateObservationsV1,
    candidates: &[RawCandidateV2],
) -> Result<Vec<RawSplitV2>, PopulationStatisticsV2Refusal> {
    let split_count = usize_of(
        observations.layout().split_count(),
        "Observation split count",
    )?;
    let candidate_count = candidates.len();
    let expected_rows = candidate_count
        .checked_mul(split_count)
        .ok_or_else(|| "Observation split projection cardinality overflowed".to_owned())?;
    if observations.split_scores().len() != expected_rows {
        return Err("population-statistics Observation split family is incomplete".to_owned());
    }
    let nifty_count = observations.nifty().candidate_count();
    let mut splits = Vec::new();
    splits
        .try_reserve_exact(split_count)
        .map_err(|why| format!("cannot reserve Observation split projection: {why}"))?;
    let mut expected_train_mask = first_train_mask(observations.layout().segment_count())?;
    let full_mask = segment_mask(observations.layout().segment_count())?;
    for split_index in 0..split_count {
        let split_sequence = u64_of(split_index, "Observation split sequence")?;
        let mut train_scores = Vec::new();
        let mut test_scores = Vec::new();
        train_scores
            .try_reserve_exact(candidate_count)
            .and_then(|()| test_scores.try_reserve_exact(candidate_count))
            .map_err(|why| format!("cannot reserve Observation split score vectors: {why}"))?;
        for (candidate_index, candidate) in candidates.iter().enumerate() {
            let row_index = candidate_index
                .checked_mul(split_count)
                .and_then(|index| index.checked_add(split_index))
                .ok_or_else(|| "Observation split row index overflowed".to_owned())?;
            let row = observations
                .split_scores()
                .get(row_index)
                .copied()
                .ok_or_else(|| "Observation split score row disappeared".to_owned())?;
            let global_sequence = u64_of(candidate_index, "Observation global candidate sequence")?;
            let (expected_family, family_sequence) = if candidate_index < nifty_count {
                (InstrumentFamilyV1::Nifty, global_sequence)
            } else {
                (
                    InstrumentFamilyV1::BankNifty,
                    u64_of(
                        candidate_index
                            .checked_sub(nifty_count)
                            .ok_or_else(|| "BANKNIFTY candidate offset underflowed".to_owned())?,
                        "Observation family candidate sequence",
                    )?,
                )
            };
            if row.global_candidate_sequence() != global_sequence
                || row.family() != expected_family
                || row.family_candidate_sequence() != family_sequence
                || row.candidate_semantic_digest() != candidate.semantic_digest
                || row.split_sequence() != split_sequence
                || row.train_mask() != expected_train_mask
                || row.test_mask() != full_mask ^ expected_train_mask
            {
                return Err("population-statistics Observation split coordinates differ".to_owned());
            }
            train_scores.push(row.train_score_paisa());
            test_scores.push(row.test_score_paisa());
        }
        splits.push(RawSplitV2 {
            train_mask: expected_train_mask,
            test_mask: full_mask ^ expected_train_mask,
            train_scores,
            test_scores,
        });
        if split_index.saturating_add(1) < split_count {
            expected_train_mask =
                next_train_mask(expected_train_mask, observations.layout().segment_count())?;
        }
    }
    Ok(splits)
}

fn observation_period_identity(
    authority: ObservationAuthorityFactsV2,
    period_sequence: u64,
    exit_ist_day: i64,
) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(OBSERVATION_PERIOD_LINK_DOMAIN);
    hasher.update(&authority.authority_id);
    hasher.update(&authority.pair_identity);
    hasher.update(&authority.source_identity);
    hasher.update(&authority.observation_policy_digest);
    hasher.update(&authority.layout_policy_digest);
    hasher.update(&period_sequence.to_le_bytes());
    hasher.update(&exit_ist_day.to_le_bytes());
    hasher.finalize()
}

fn observation_projection_policy_digest() -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(OBSERVATION_PROJECTION_POLICY_DOMAIN);
    hasher.update(&HEADER_VERSION.to_le_bytes());
    hasher.update(&RECORD_VERSION.to_le_bytes());
    hasher.update(OBSERVATION_PERIOD_LINK_DOMAIN);
    hasher.update(b"nifty-then-banknifty;period-major;split-major;no-v1-byte-reinterpretation");
    hasher.finalize()
}

fn observation_statistics_link(
    observation: ObservationAuthorityFactsV2,
    statistics: &PopulationStatisticsManifestV2,
    procedure: PopulationStatisticsProcedureV2,
) -> Result<PopulationStatisticsObservationLinkV2, PopulationStatisticsV2Refusal> {
    if statistics.candidate_count()? != observation.candidate_count
        || statistics.period_count != observation.period_count
        || statistics.split_count != observation.split_count
        || observation
            .candidate_count
            .checked_mul(observation.split_count)
            != Some(observation.split_score_row_count)
    {
        return Err(
            "population-statistics prepared cardinalities differ from Observation authority"
                .to_owned(),
        );
    }
    let projection_policy_digest = observation_projection_policy_digest();
    let mut hasher = Hasher::new();
    hasher.update(OBSERVATION_STATISTICS_LINK_DOMAIN);
    hasher.update(&observation.authority_id);
    hasher.update(&observation.pair_identity);
    hasher.update(&observation.source_identity);
    hasher.update(&observation.observation_policy_digest);
    hasher.update(&observation.layout_policy_digest);
    hasher.update(&observation.data_record_digest);
    hasher.update(&observation.completion_digest);
    hasher.update(&statistics.audit_id);
    hasher.update(&statistics.cscv_policy_digest);
    hasher.update(&projection_policy_digest);
    hasher.update(&procedure.draws().to_le_bytes());
    hasher.update(&procedure.seed().to_le_bytes());
    hasher.update(&procedure.block_length().to_le_bytes());
    hasher.update(&observation.candidate_count.to_le_bytes());
    hasher.update(&observation.period_count.to_le_bytes());
    hasher.update(&observation.split_count.to_le_bytes());
    hasher.update(&observation.split_score_row_count.to_le_bytes());
    let value = PopulationStatisticsObservationLinkV2 {
        observation_authority_id: observation.authority_id,
        observation_pair_identity: observation.pair_identity,
        statistics_audit_id: statistics.audit_id,
        projection_policy_digest,
        candidate_count: observation.candidate_count,
        period_count: observation.period_count,
        split_count: observation.split_count,
        identity: hasher.finalize(),
    };
    for (name, digest) in [
        ("Observation authority", value.observation_authority_id),
        ("Observation pair", value.observation_pair_identity),
        ("Statistics audit", value.statistics_audit_id),
        (
            "Observation projection policy",
            value.projection_policy_digest,
        ),
        ("Observation-to-Statistics link", value.identity),
    ] {
        require_nonzero(name, digest)?;
    }
    Ok(value)
}

/// Result of an exact receipt-last append followed by a fresh read-only reopen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopulationStatisticsV2Append {
    /// New Data/raw rows and then Completion were durably appended.
    Written(PopulationStatisticsV2ReopenAudit),
    /// Every existing byte matched the exact prepared retry.
    Reused(PopulationStatisticsV2ReopenAudit),
}

impl PopulationStatisticsV2Append {
    /// Freshly reopened audit produced by either exact branch.
    #[must_use]
    pub const fn audit(self) -> PopulationStatisticsV2ReopenAudit {
        match self {
            Self::Written(audit) | Self::Reused(audit) => audit,
        }
    }
}

impl PopulationStatisticsV2Ledger {
    fn append_locked(
        &mut self,
        prepared: &PreparedPopulationStatisticsV2,
    ) -> Result<PopulationStatisticsV2Append, PopulationStatisticsV2Refusal> {
        self.require_unchanged()?;
        if let Some(existing) = self.audits.get(&prepared.manifest.audit_id).copied() {
            let planned = prepared.records(existing.sequence(), existing.first_record)?;
            compare_planned_records(&mut self.data_file, existing.first_record, &planned)?;
            return Ok(PopulationStatisticsV2Append::Reused(existing));
        }
        if let Some(orphan) = self.orphan {
            return self.resume_orphan(prepared, &orphan);
        }
        if self.completed_audits >= self.bounds.audits {
            return Err("population-statistics append reached audit bound".to_owned());
        }
        self.bounds.validate_manifest(&prepared.manifest)?;
        let first = record_count(
            self.data_file
                .metadata()
                .map_err(|why| format!("cannot stat append file: {why}"))?
                .len(),
        )?;
        let planned = prepared.records(self.completed_audits, first)?;
        let added_bytes = u64_of(planned.len(), "planned records")?
            .checked_mul(POPULATION_STATISTICS_V2_RECORD_STRIDE)
            .ok_or_else(|| "planned byte count overflowed".to_owned())?;
        let desired = self
            .data_file
            .metadata()
            .map_err(|why| format!("cannot stat append file: {why}"))?
            .len()
            .checked_add(added_bytes)
            .ok_or_else(|| "planned file size overflowed".to_owned())?;
        if desired > self.bounds.file_bytes {
            return Err(format!(
                "population-statistics append would produce {desired} bytes above explicit maximum {}",
                self.bounds.file_bytes
            ));
        }
        let completion = planned
            .len()
            .checked_sub(1)
            .ok_or_else(|| "planned block lacks completion".to_owned())?;
        for raw in planned
            .get(..completion)
            .ok_or_else(|| "planned data prefix is absent".to_owned())?
        {
            append_raw_record(&mut self.data_file, raw)?;
        }
        self.data_file
            .sync_all()
            .map_err(|why| format!("cannot sync population-statistics Data block: {why}"))?;
        append_raw_record(
            &mut self.data_file,
            planned
                .get(completion)
                .ok_or_else(|| "planned completion is absent".to_owned())?,
        )?;
        self.data_file
            .sync_all()
            .map_err(|why| format!("cannot sync population-statistics Completion: {why}"))?;
        let manifest = prepared.manifest.with_sequence(self.completed_audits);
        let audit = validate_complete_block(&mut self.data_file, first, &manifest)?;
        self.audits.insert(prepared.manifest.audit_id, audit);
        self.completed_audits = self
            .completed_audits
            .checked_add(1)
            .ok_or_else(|| "completed audit count overflowed".to_owned())?;
        self.data_generation =
            file_generation(&self.data_file, &self.data_path, self.bounds.file_bytes)?;
        Ok(PopulationStatisticsV2Append::Written(audit))
    }

    fn resume_orphan(
        &mut self,
        prepared: &PreparedPopulationStatisticsV2,
        orphan: &OrphanV2,
    ) -> Result<PopulationStatisticsV2Append, PopulationStatisticsV2Refusal> {
        if orphan.manifest.audit_id != prepared.manifest.audit_id {
            return Err(format!(
                "population-statistics trailing orphan belongs to {}, not exact retry {}",
                hex32(orphan.manifest.audit_id),
                hex32(prepared.manifest.audit_id)
            ));
        }
        let planned = prepared.records(orphan.manifest.sequence, orphan.first_record)?;
        compare_planned_prefix(
            &mut self.data_file,
            orphan.first_record,
            orphan.present_records,
            &planned,
        )?;
        let completion = planned
            .len()
            .checked_sub(1)
            .ok_or_else(|| "orphan retry plan lacks Completion".to_owned())?;
        let present = usize_of(orphan.present_records, "orphan present records")?;
        for raw in planned
            .get(present..completion)
            .ok_or_else(|| "orphan retry Data suffix is outside plan".to_owned())?
        {
            append_raw_record(&mut self.data_file, raw)?;
        }
        self.data_file
            .sync_all()
            .map_err(|why| format!("cannot sync orphan Data suffix: {why}"))?;
        append_raw_record(
            &mut self.data_file,
            planned
                .get(completion)
                .ok_or_else(|| "orphan retry Completion is absent".to_owned())?,
        )?;
        self.data_file
            .sync_all()
            .map_err(|why| format!("cannot sync orphan completion: {why}"))?;
        let audit =
            validate_complete_block(&mut self.data_file, orphan.first_record, &orphan.manifest)?;
        self.audits.insert(prepared.manifest.audit_id, audit);
        self.completed_audits = self
            .completed_audits
            .checked_add(1)
            .ok_or_else(|| "completed audit count overflowed".to_owned())?;
        self.orphan = None;
        self.data_generation =
            file_generation(&self.data_file, &self.data_path, self.bounds.file_bytes)?;
        Ok(PopulationStatisticsV2Append::Written(audit))
    }

    fn append(
        &mut self,
        prepared: &PreparedPopulationStatisticsV2,
    ) -> Result<PopulationStatisticsV2Append, PopulationStatisticsV2Refusal> {
        self.lock_file
            .lock()
            .map_err(|why| format!("cannot take population-statistics append lock: {why}"))?;
        let result = self.append_locked(prepared);
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release population-statistics append lock: {why}"));
        match (result, released) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }
}

/// Durably appends one opaque prepared block and freshly reopens it read-only.
///
/// The directory must already exist.  The writer never manufactures a missing
/// configured root.  Data, candidate, period and split records are synced
/// before the Completion record is appended and synced.  Success is returned
/// only after dropping the writable ledger, opening a new read-only ledger and
/// finding the byte-identical recomputed audit.
///
/// This is a durability boundary, not a preparation authority: callers cannot
/// construct [`PreparedPopulationStatisticsV2`] through the public API.
///
/// # Errors
///
/// Refuses an absent/non-directory root, bounds, lock, I/O, corruption,
/// foreign retry, stale generation, failed recomputation or fresh-reopen
/// mismatch.
pub fn append_population_statistics_v2(
    root: impl AsRef<Path>,
    bounds: PopulationStatisticsV2Bounds,
    prepared: &PreparedPopulationStatisticsV2,
) -> Result<PopulationStatisticsV2Append, PopulationStatisticsV2Refusal> {
    let root = root.as_ref();
    let mut writer = PopulationStatisticsV2Ledger::open_writer(root, bounds)?;
    let committed = writer.append(prepared)?;
    let expected = committed.audit();
    drop(writer);

    let reopened = PopulationStatisticsV2Ledger::open_read(root, bounds)?
        .reopen_audit(&expected.audit_id())?
        .ok_or_else(|| {
            format!(
                "population-statistics audit {} disappeared after receipt-last append",
                hex32(expected.audit_id())
            )
        })?;
    if reopened != expected {
        return Err(format!(
            "population-statistics audit {} did not freshly reopen with exact semantics",
            hex32(expected.audit_id())
        ));
    }
    Ok(match committed {
        PopulationStatisticsV2Append::Written(_) => PopulationStatisticsV2Append::Written(reopened),
        PopulationStatisticsV2Append::Reused(_) => PopulationStatisticsV2Append::Reused(reopened),
    })
}

fn compare_planned_records(
    file: &mut File,
    first: u64,
    planned: &[[u8; RECORD_BYTES]],
) -> Result<(), PopulationStatisticsV2Refusal> {
    compare_planned_prefix(
        file,
        first,
        u64_of(planned.len(), "planned records")?,
        planned,
    )
}

fn compare_planned_prefix(
    file: &mut File,
    first: u64,
    present: u64,
    planned: &[[u8; RECORD_BYTES]],
) -> Result<(), PopulationStatisticsV2Refusal> {
    if present > u64_of(planned.len(), "planned records")? {
        return Err("population-statistics retry prefix exceeds planned block".to_owned());
    }
    for local in 0..present {
        let physical = first
            .checked_add(local)
            .ok_or_else(|| "retry physical index overflowed".to_owned())?;
        let observed = read_raw_record(file, physical)?;
        let expected = planned
            .get(usize_of(local, "retry local record")?)
            .ok_or_else(|| "retry planned record is absent".to_owned())?;
        if &observed != expected {
            return Err(format!(
                "population-statistics exact retry differs at local record {local}"
            ));
        }
    }
    Ok(())
}

fn verify_pre_admission_pair(
    manifest: &PopulationStatisticsManifestV2,
    nifty: &PreAdmissionDataV1,
    banknifty: &PreAdmissionDataV1,
) -> Result<(), PopulationStatisticsV2Refusal> {
    let actual_nifty = PreAdmissionBindingV2::from_value(nifty);
    let actual_banknifty = PreAdmissionBindingV2::from_value(banknifty);
    if actual_nifty != manifest.nifty || actual_banknifty != manifest.banknifty {
        return Err("population-statistics pre-admission binding differs".to_owned());
    }
    for (name, left, right, expected) in [
        (
            "rung",
            u64::from(nifty.rung_seconds()),
            u64::from(banknifty.rung_seconds()),
            u64::from(manifest.rung_seconds),
        ),
        (
            "horizon",
            u64::from(nifty.horizon_bars()),
            u64::from(banknifty.horizon_bars()),
            u64::from(manifest.horizon_bars),
        ),
    ] {
        if left != right || left != expected {
            return Err(format!(
                "population-statistics pre-admission {name} differs"
            ));
        }
    }
    if nifty.requested_span() != banknifty.requested_span()
        || nifty.requested_span() != manifest.requested_span
    {
        return Err("population-statistics requested span differs".to_owned());
    }
    for (name, left, right, expected) in [
        (
            "feed",
            nifty.feed_digest(),
            banknifty.feed_digest(),
            manifest.feed_digest,
        ),
        (
            "source commit",
            nifty.source_commit_digest(),
            banknifty.source_commit_digest(),
            manifest.source_commit_digest,
        ),
        (
            "calendar policy",
            nifty.calendar_policy_digest(),
            banknifty.calendar_policy_digest(),
            manifest.calendar_policy_digest,
        ),
        (
            "daily-reference policy",
            nifty.daily_reference_policy_digest(),
            banknifty.daily_reference_policy_digest(),
            manifest.daily_reference_policy_digest,
        ),
    ] {
        if left != right || left != expected {
            return Err(format!(
                "population-statistics pre-admission {name} identity differs"
            ));
        }
    }
    Ok(())
}

fn validate_source_pair(
    nifty: &PreAdmissionSourceV2,
    banknifty: &PreAdmissionSourceV2,
) -> Result<(), PopulationStatisticsV2Refusal> {
    nifty.binding.validate(InstrumentFamilyV1::Nifty)?;
    banknifty.binding.validate(InstrumentFamilyV1::BankNifty)?;
    if nifty.rung_seconds != banknifty.rung_seconds
        || nifty.horizon_bars != banknifty.horizon_bars
        || nifty.requested_span != banknifty.requested_span
        || nifty.feed_digest != banknifty.feed_digest
        || nifty.source_commit_digest != banknifty.source_commit_digest
        || nifty.calendar_policy_digest != banknifty.calendar_policy_digest
        || nifty.daily_reference_policy_digest != banknifty.daily_reference_policy_digest
    {
        return Err(
            "population-statistics NIFTY/BANKNIFTY pre-admission identities differ".to_owned(),
        );
    }
    require_new_production_rung(nifty.rung_seconds)?;
    if nifty.horizon_bars == 0 {
        return Err("population-statistics pre-admission horizon is zero".to_owned());
    }
    RequestedSpanIdentityV1::new(
        nifty.requested_span.from_year(),
        nifty.requested_span.from_month(),
        nifty.requested_span.to_year(),
        nifty.requested_span.to_month(),
    )?;
    for (name, digest) in [
        ("feed", nifty.feed_digest),
        ("source commit", nifty.source_commit_digest),
        ("calendar policy", nifty.calendar_policy_digest),
        (
            "daily-reference policy",
            nifty.daily_reference_policy_digest,
        ),
    ] {
        require_nonzero(name, digest)?;
    }
    Ok(())
}

fn family_test_from_white(
    receipt: WhiteRealityCheckReceiptV1,
) -> Result<PopulationFamilyTestV2, PopulationStatisticsV2Refusal> {
    let exact = receipt.exact_p_value();
    let value = PopulationFamilyTestV2 {
        statistic_bits: receipt.statistic_bits(),
        probability_bits: receipt.p_value_bits(),
        probability: PopulationStatisticsFractionV2::new(
            u64_of(exact.numerator(), "White numerator")?,
            u64_of(exact.denominator(), "White denominator")?,
        )?,
        family_digest: receipt.family_digest(),
    };
    value.validate("White")?;
    Ok(value)
}

fn family_test_from_spa(
    receipt: SpaReceiptV1,
) -> Result<PopulationFamilyTestV2, PopulationStatisticsV2Refusal> {
    let exact = receipt.exact_p_value();
    let value = PopulationFamilyTestV2 {
        statistic_bits: receipt.statistic_bits(),
        probability_bits: receipt.p_value_bits(),
        probability: PopulationStatisticsFractionV2::new(
            u64_of(exact.numerator(), "SPA numerator")?,
            u64_of(exact.denominator(), "SPA denominator")?,
        )?,
        family_digest: receipt.family_digest(),
    };
    value.validate("SPA")?;
    Ok(value)
}

fn pbo_from_counts(
    split_count: u64,
    contributing: u64,
    bottom_half: u64,
) -> Result<PopulationCscvPboV2, PopulationStatisticsV2Refusal> {
    let unrankable = split_count
        .checked_sub(contributing)
        .ok_or_else(|| "CSCV contributing splits exceed total".to_owned())?;
    if contributing == 0 {
        return Err("CSCV has no rankable complementary split".to_owned());
    }
    let probability = PopulationStatisticsFractionV2::new(bottom_half, contributing)?;
    let value = PopulationCscvPboV2 {
        contributing_splits: contributing,
        bottom_half_splits: bottom_half,
        unrankable_splits: unrankable,
        probability_bits: probability.bits(),
        probability,
    };
    value.validate(split_count)?;
    Ok(value)
}

fn cscv_placement(
    train_scores: &[i64],
    test_scores: &[i64],
) -> Result<(bool, bool), PopulationStatisticsV2Refusal> {
    if train_scores.len() != test_scores.len() || train_scores.is_empty() {
        return Err("CSCV score families are empty or misaligned".to_owned());
    }
    if train_scores.len() < 2 {
        return Ok((false, false));
    }
    let mut winner = 0_usize;
    let mut best = *train_scores
        .first()
        .ok_or_else(|| "CSCV first train score is absent".to_owned())?;
    for (index, score) in train_scores.iter().copied().enumerate().skip(1) {
        if score > best {
            best = score;
            winner = index;
        }
    }
    let winner_score = *test_scores
        .get(winner)
        .ok_or_else(|| "CSCV winner test score is absent".to_owned())?;
    let better = test_scores
        .iter()
        .filter(|score| **score > winner_score)
        .count();
    let tied = test_scores
        .iter()
        .filter(|score| **score == winner_score)
        .count();
    let rank_twice = better
        .checked_mul(2)
        .and_then(|rank| rank.checked_add(tied.checked_sub(1)?))
        .ok_or_else(|| "CSCV doubled rank overflowed".to_owned())?;
    let last_twice = train_scores
        .len()
        .checked_sub(1)
        .ok_or_else(|| "CSCV score family lacks a last rank".to_owned())?
        .checked_mul(2)
        .ok_or_else(|| "CSCV doubled last rank overflowed".to_owned())?;
    let doubled_rank = u128::try_from(rank_twice)
        .map_err(|_| "CSCV doubled rank does not fit u128".to_owned())?
        .checked_mul(2)
        .ok_or_else(|| "CSCV midpoint comparison overflowed".to_owned())?;
    let doubled_last = u128::try_from(last_twice)
        .map_err(|_| "CSCV doubled last rank does not fit u128".to_owned())?;
    Ok((doubled_rank > doubled_last, true))
}

fn wilson_lower_bits(wins: u64, trades: u64) -> u64 {
    if trades == 0 {
        return 0.0_f64.to_bits();
    }
    wilson_lower(wins, trades).to_bits()
}

#[expect(
    clippy::cast_precision_loss,
    clippy::float_arithmetic,
    reason = "Wilson is a full-precision statistic over counts, never a price"
)]
fn wilson_lower(wins: u64, trades: u64) -> f64 {
    const Z: f64 = 1.959_964;
    let n = trades as f64;
    let p = wins.min(trades) as f64 / n;
    let z2 = Z * Z;
    let denominator = 1.0 + z2 / n;
    let centre = p + z2 / (2.0 * n);
    let margin = Z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt();
    ((centre - margin) / denominator).clamp(0.0, 1.0)
}

fn canonical_split_count(segment_count: u32) -> Result<u64, PopulationStatisticsV2Refusal> {
    validate_segment_count(segment_count)?;
    choose_u64(segment_count - 1, segment_count / 2)
}

fn validate_segment_count(segment_count: u32) -> Result<(), PopulationStatisticsV2Refusal> {
    if !(2..=64).contains(&segment_count) || !segment_count.is_multiple_of(2) {
        return Err(format!(
            "CSCV segment count {segment_count} is not even in 2..=64"
        ));
    }
    Ok(())
}

fn choose_u64(n: u32, k: u32) -> Result<u64, PopulationStatisticsV2Refusal> {
    let k = k.min(
        n.checked_sub(k)
            .ok_or_else(|| "CSCV combination k exceeds n".to_owned())?,
    );
    let mut value = 1_u128;
    for index in 0..k {
        value = value
            .checked_mul(u128::from(n - index))
            .ok_or_else(|| "CSCV combination count overflowed u128".to_owned())?
            / u128::from(index + 1);
    }
    u64::try_from(value).map_err(|_| "CSCV combination count exceeds u64".to_owned())
}

fn segment_mask(segment_count: u32) -> Result<u64, PopulationStatisticsV2Refusal> {
    validate_segment_count(segment_count)?;
    Ok(if segment_count == 64 {
        u64::MAX
    } else {
        1_u64
            .checked_shl(segment_count)
            .ok_or_else(|| "CSCV segment mask shift overflowed".to_owned())?
            .checked_sub(1)
            .ok_or_else(|| "CSCV segment mask is empty".to_owned())?
    })
}

fn first_train_mask(segment_count: u32) -> Result<u64, PopulationStatisticsV2Refusal> {
    validate_segment_count(segment_count)?;
    let half = segment_count / 2;
    let compressed = 1_u64
        .checked_shl(half)
        .ok_or_else(|| "CSCV first mask shift overflowed".to_owned())?
        .checked_sub(1)
        .ok_or_else(|| "CSCV first mask is empty".to_owned())?;
    compressed
        .checked_shl(1)
        .ok_or_else(|| "CSCV first train mask overflowed".to_owned())
}

fn next_train_mask(current: u64, segment_count: u32) -> Result<u64, PopulationStatisticsV2Refusal> {
    validate_segment_count(segment_count)?;
    let compressed = current >> 1;
    let smallest = compressed & compressed.wrapping_neg();
    if smallest == 0 {
        return Err("CSCV train mask has no set bit".to_owned());
    }
    let ripple = compressed
        .checked_add(smallest)
        .ok_or_else(|| "CSCV next-mask ripple overflowed".to_owned())?;
    let ones = ((compressed ^ ripple) >> 2) / smallest;
    let next = ripple | ones;
    let compressed_bits = segment_count - 1;
    if compressed_bits < 64 && next >= (1_u64 << compressed_bits) {
        return Err("CSCV canonical train-mask sequence is exhausted".to_owned());
    }
    next.checked_shl(1)
        .ok_or_else(|| "CSCV next train mask overflowed".to_owned())
}

fn split_identity(segment_count: u32, train_mask: u64, test_mask: u64) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(SPLIT_ID_DOMAIN);
    hasher.update(&segment_count.to_le_bytes());
    hasher.update(&train_mask.to_le_bytes());
    hasher.update(&test_mask.to_le_bytes());
    hasher.finalize()
}

fn cscv_policy_digest(segment_count: u32) -> Result<[u8; 32], PopulationStatisticsV2Refusal> {
    let mut hasher = Hasher::new();
    hasher.update(CSCV_POLICY_DOMAIN);
    hasher.update(&segment_count.to_le_bytes());
    hasher.update(&canonical_split_count(segment_count)?.to_le_bytes());
    hasher.update(
        b"train-bit-zero-absent;half-segments;ascending-gosper;first-strict-max;exact-midrank",
    );
    Ok(hasher.finalize())
}

fn candidate_source_hasher(domain: &[u8], sequence: u64) -> Hasher {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    hasher.update(&sequence.to_le_bytes());
    hasher
}

fn ordered_candidate_digest(
    candidates: &[PopulationStatisticsCandidateV2],
) -> Result<[u8; 32], PopulationStatisticsV2Refusal> {
    let mut hasher = Hasher::new();
    hasher.update(CANDIDATE_ORDER_DOMAIN);
    hasher.update(&u64_of(candidates.len(), "ordered candidate count")?.to_le_bytes());
    for candidate in candidates {
        hash_candidate(&mut hasher, candidate);
    }
    Ok(hasher.finalize())
}

fn ordered_period_digest(
    periods: &[PopulationStatisticsPeriodSourceV2],
    candidate_count: u64,
    period_count: u64,
) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(PERIOD_ORDER_DOMAIN);
    hasher.update(&candidate_count.to_le_bytes());
    hasher.update(&period_count.to_le_bytes());
    for period in periods {
        hash_period(&mut hasher, *period);
    }
    hasher.finalize()
}

fn ordered_period_digest_for(
    sequence: u64,
    periods: &[PopulationStatisticsPeriodSourceV2],
) -> [u8; 32] {
    let mut hasher = candidate_source_hasher(PERIOD_ORDER_DOMAIN, sequence);
    for period in periods {
        hash_period(&mut hasher, *period);
    }
    hasher.finalize()
}

fn ordered_split_digest(
    splits: &[PopulationStatisticsSplitSourceV2],
    candidate_count: u64,
    split_count: u64,
) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(SPLIT_ORDER_DOMAIN);
    hasher.update(&candidate_count.to_le_bytes());
    hasher.update(&split_count.to_le_bytes());
    for split in splits {
        hash_split(&mut hasher, *split);
    }
    hasher.finalize()
}

fn ordered_split_digest_for(
    sequence: u64,
    splits: &[PopulationStatisticsSplitSourceV2],
) -> [u8; 32] {
    let mut hasher = candidate_source_hasher(SPLIT_ORDER_DOMAIN, sequence);
    for split in splits {
        hash_split(&mut hasher, *split);
    }
    hasher.finalize()
}

fn hash_candidate(hasher: &mut Hasher, candidate: &PopulationStatisticsCandidateV2) {
    hasher.update(&candidate.sequence.to_le_bytes());
    hasher.update(&[family_byte(candidate.family)]);
    hasher.update(&candidate.family_sequence.to_le_bytes());
    hasher.update(&candidate.candidate_semantic_digest);
    hasher.update(&candidate.pre_admission_id);
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

fn hash_period(hasher: &mut Hasher, period: PopulationStatisticsPeriodSourceV2) {
    hasher.update(&period.candidate_sequence.to_le_bytes());
    hasher.update(&period.period_sequence.to_le_bytes());
    hasher.update(&period.candidate_semantic_digest);
    hasher.update(&period.return_paisa.to_le_bytes());
    hasher.update(&period.trades.to_le_bytes());
    hasher.update(&period.wins.to_le_bytes());
    hasher.update(&period.period_identity);
}

fn hash_split(hasher: &mut Hasher, split: PopulationStatisticsSplitSourceV2) {
    hasher.update(&split.candidate_sequence.to_le_bytes());
    hasher.update(&split.split_sequence.to_le_bytes());
    hasher.update(&split.candidate_semantic_digest);
    hasher.update(&split.segment_count.to_le_bytes());
    hasher.update(&split.train_mask.to_le_bytes());
    hasher.update(&split.test_mask.to_le_bytes());
    hasher.update(&split.train_score.to_le_bytes());
    hasher.update(&split.test_score.to_le_bytes());
    hasher.update(&split.split_identity);
}

fn derive_audit_id(
    mut manifest: PopulationStatisticsManifestV2,
) -> Result<[u8; 32], PopulationStatisticsV2Refusal> {
    manifest.audit_id = [0; 32];
    manifest.sequence = 0;
    let mut payload = [0_u8; PAYLOAD_BYTES];
    put_u32(&mut payload, 0, RECORD_VERSION)?;
    put_u32(&mut payload, 4, DATA_KIND)?;
    put_u32(&mut payload, 56, manifest.rung_seconds)?;
    put_u32(&mut payload, 60, manifest.horizon_bars)?;
    encode_span(manifest.requested_span, &mut payload, 64)?;
    put_u32(&mut payload, 84, manifest.segment_count)?;
    put_u64(&mut payload, 88, manifest.period_count)?;
    put_u64(&mut payload, 96, manifest.split_count)?;
    put_u64(&mut payload, 104, manifest.draws)?;
    put_u64(&mut payload, 112, manifest.seed)?;
    put_u64(&mut payload, 120, manifest.block_length)?;
    encode_binding(manifest.nifty, &mut payload, 128)?;
    encode_binding(manifest.banknifty, &mut payload, 248)?;
    put_bytes(&mut payload, 368, &manifest.feed_digest)?;
    put_bytes(&mut payload, 400, &manifest.source_commit_digest)?;
    put_bytes(&mut payload, 432, &manifest.calendar_policy_digest)?;
    put_bytes(&mut payload, 464, &manifest.daily_reference_policy_digest)?;
    put_bytes(&mut payload, 496, &manifest.wilson_policy_digest)?;
    put_bytes(&mut payload, 528, &manifest.cscv_policy_digest)?;
    encode_family_test(manifest.white, &mut payload, 560)?;
    encode_family_test(manifest.spa, &mut payload, 624)?;
    put_bytes(&mut payload, 688, &manifest.romano_wolf_family_digest)?;
    put_u64(&mut payload, 720, manifest.pbo.contributing_splits)?;
    put_u64(&mut payload, 728, manifest.pbo.bottom_half_splits)?;
    put_u64(&mut payload, 736, manifest.pbo.unrankable_splits)?;
    put_u64(&mut payload, 744, manifest.pbo.probability.numerator)?;
    put_u64(&mut payload, 752, manifest.pbo.probability.denominator)?;
    put_u64(&mut payload, 760, manifest.pbo.probability_bits)?;
    put_bytes(&mut payload, 768, &manifest.ordered_candidate_digest)?;
    put_bytes(&mut payload, 800, &manifest.ordered_period_digest)?;
    put_bytes(&mut payload, 832, &manifest.ordered_split_digest)?;
    Ok(digest_domain(AUDIT_DOMAIN, &payload))
}

fn encode_binding(
    binding: PreAdmissionBindingV2,
    raw: &mut [u8],
    offset: usize,
) -> Result<(), PopulationStatisticsV2Refusal> {
    put_u8(raw, offset, family_byte(binding.family))?;
    put_u64(raw, offset + 8, binding.sequence)?;
    put_bytes(raw, offset + 16, &binding.authority_id)?;
    put_bytes(raw, offset + 48, &binding.candidate_universe_id)?;
    put_bytes(raw, offset + 80, &binding.candidate_completion_digest)?;
    put_u64(raw, offset + 112, binding.candidate_count)
}

fn decode_binding(
    raw: &[u8],
    offset: usize,
) -> Result<PreAdmissionBindingV2, PopulationStatisticsV2Refusal> {
    Ok(PreAdmissionBindingV2 {
        family: family_from_byte(
            *raw.get(offset)
                .ok_or_else(|| "population-statistics binding family byte is absent".to_owned())?,
        )?,
        sequence: get_u64(raw, offset + 8)?,
        authority_id: get_32(raw, offset + 16)?,
        candidate_universe_id: get_32(raw, offset + 48)?,
        candidate_completion_digest: get_32(raw, offset + 80)?,
        candidate_count: get_u64(raw, offset + 112)?,
    })
}

fn encode_family_test(
    value: PopulationFamilyTestV2,
    raw: &mut [u8],
    offset: usize,
) -> Result<(), PopulationStatisticsV2Refusal> {
    put_u64(raw, offset, value.statistic_bits)?;
    put_u64(raw, offset + 8, value.probability_bits)?;
    put_u64(raw, offset + 16, value.probability.numerator)?;
    put_u64(raw, offset + 24, value.probability.denominator)?;
    put_bytes(raw, offset + 32, &value.family_digest)
}

fn decode_family_test(
    raw: &[u8],
    offset: usize,
) -> Result<PopulationFamilyTestV2, PopulationStatisticsV2Refusal> {
    Ok(PopulationFamilyTestV2 {
        statistic_bits: get_u64(raw, offset)?,
        probability_bits: get_u64(raw, offset + 8)?,
        probability: PopulationStatisticsFractionV2::new(
            get_u64(raw, offset + 16)?,
            get_u64(raw, offset + 24)?,
        )?,
        family_digest: get_32(raw, offset + 32)?,
    })
}

fn encode_span(
    span: RequestedSpanIdentityV1,
    raw: &mut [u8],
    offset: usize,
) -> Result<(), PopulationStatisticsV2Refusal> {
    RequestedSpanIdentityV1::new(
        span.from_year(),
        span.from_month(),
        span.to_year(),
        span.to_month(),
    )?;
    put_u32(raw, offset, 1)?;
    put_u32(raw, offset + 4, u32::from(span.from_year()))?;
    put_u32(raw, offset + 8, u32::from(span.from_month()))?;
    put_u32(raw, offset + 12, u32::from(span.to_year()))?;
    put_u32(raw, offset + 16, u32::from(span.to_month()))
}

fn decode_span(
    raw: &[u8],
    offset: usize,
) -> Result<RequestedSpanIdentityV1, PopulationStatisticsV2Refusal> {
    if get_u32(raw, offset)? != 1 {
        return Err("population-statistics requested-span version is unknown".to_owned());
    }
    RequestedSpanIdentityV1::new(
        u16::try_from(get_u32(raw, offset + 4)?)
            .map_err(|_| "requested from-year exceeds u16".to_owned())?,
        u8::try_from(get_u32(raw, offset + 8)?)
            .map_err(|_| "requested from-month exceeds u8".to_owned())?,
        u16::try_from(get_u32(raw, offset + 12)?)
            .map_err(|_| "requested to-year exceeds u16".to_owned())?,
        u8::try_from(get_u32(raw, offset + 16)?)
            .map_err(|_| "requested to-month exceeds u8".to_owned())?,
    )
}

fn base_payload(
    kind: RecordKindV2,
    physical_sequence: u64,
    audit_id: [u8; 32],
) -> Result<[u8; PAYLOAD_BYTES], PopulationStatisticsV2Refusal> {
    let mut payload = [0_u8; PAYLOAD_BYTES];
    put_u32(&mut payload, 0, RECORD_VERSION)?;
    put_u32(&mut payload, 4, kind.byte())?;
    put_u64(&mut payload, 8, physical_sequence)?;
    put_bytes(&mut payload, 16, &audit_id)?;
    Ok(payload)
}

fn seal_payload(
    payload: &[u8; PAYLOAD_BYTES],
) -> Result<[u8; RECORD_BYTES], PopulationStatisticsV2Refusal> {
    let mut raw = [0_u8; RECORD_BYTES];
    raw.get_mut(..PAYLOAD_BYTES)
        .ok_or_else(|| "population-statistics record payload destination is absent".to_owned())?
        .copy_from_slice(payload);
    raw.get_mut(PAYLOAD_BYTES..)
        .ok_or_else(|| "population-statistics record seal destination is absent".to_owned())?
        .copy_from_slice(&digest_domain(RECORD_DOMAIN, payload));
    Ok(raw)
}

fn digest_completion_record(payload: &[u8; PAYLOAD_BYTES]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(COMPLETION_RECORD_DIGEST_DOMAIN);
    hasher.update(payload);
    hasher.update(&digest_domain(RECORD_DOMAIN, payload));
    hasher.finalize()
}

fn header() -> Result<[u8; HEADER_BYTES], PopulationStatisticsV2Refusal> {
    let mut payload = [0_u8; 32];
    put_bytes(&mut payload, 0, &HEADER_MAGIC)?;
    put_u32(&mut payload, 16, HEADER_VERSION)?;
    put_u32(&mut payload, 20, HEADER_KIND)?;
    put_u64(&mut payload, 24, POPULATION_STATISTICS_V2_RECORD_STRIDE)?;
    let mut header = [0_u8; HEADER_BYTES];
    put_bytes(&mut header, 0, &payload)?;
    put_bytes(&mut header, 32, &digest_domain(HEADER_DOMAIN, &payload))?;
    Ok(header)
}

fn ensure_header(file: &mut File, path: &Path) -> Result<(), PopulationStatisticsV2Refusal> {
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

fn verify_header(file: &mut File, path: &Path) -> Result<(), PopulationStatisticsV2Refusal> {
    let len = file
        .metadata()
        .map_err(|why| format!("cannot stat {}: {why}", path.display()))?
        .len();
    if len < POPULATION_STATISTICS_V2_HEADER_BYTES {
        return Err(format!(
            "{} is {len} bytes, shorter than population-statistics header",
            path.display()
        ));
    }
    let mut observed = [0_u8; HEADER_BYTES];
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_exact(&mut observed))
        .map_err(|why| format!("cannot read {} header: {why}", path.display()))?;
    if observed != header()? {
        return Err(format!(
            "{} population-statistics header is unknown or corrupt",
            path.display()
        ));
    }
    Ok(())
}

fn record_count(file_len: u64) -> Result<u64, PopulationStatisticsV2Refusal> {
    let body = file_len
        .checked_sub(POPULATION_STATISTICS_V2_HEADER_BYTES)
        .ok_or_else(|| "population-statistics file is shorter than header".to_owned())?;
    if body % POPULATION_STATISTICS_V2_RECORD_STRIDE != 0 {
        return Err(format!(
            "population-statistics body has {body} ragged bytes against stride {POPULATION_STATISTICS_V2_RECORD_STRIDE}"
        ));
    }
    Ok(body / POPULATION_STATISTICS_V2_RECORD_STRIDE)
}

fn record_offset(index: u64) -> Result<u64, PopulationStatisticsV2Refusal> {
    POPULATION_STATISTICS_V2_HEADER_BYTES
        .checked_add(
            index
                .checked_mul(POPULATION_STATISTICS_V2_RECORD_STRIDE)
                .ok_or_else(|| "population-statistics byte offset overflowed".to_owned())?,
        )
        .ok_or_else(|| "population-statistics header offset overflowed".to_owned())
}

fn read_record(
    file: &mut File,
    index: u64,
) -> Result<DecodedRecordV2, PopulationStatisticsV2Refusal> {
    let mut raw = [0_u8; RECORD_BYTES];
    file.seek(SeekFrom::Start(record_offset(index)?))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| format!("cannot read population-statistics record {index}: {why}"))?;
    decode_record(&raw, index)
}

fn read_raw_record(
    file: &mut File,
    index: u64,
) -> Result<[u8; RECORD_BYTES], PopulationStatisticsV2Refusal> {
    let mut raw = [0_u8; RECORD_BYTES];
    file.seek(SeekFrom::Start(record_offset(index)?))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| format!("cannot read raw population-statistics record {index}: {why}"))?;
    Ok(raw)
}

fn append_raw_record(
    file: &mut File,
    raw: &[u8; RECORD_BYTES],
) -> Result<(), PopulationStatisticsV2Refusal> {
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(raw))
        .map_err(|why| format!("cannot append population-statistics record: {why}"))
}

fn open_file(
    path: &Path,
    writable: bool,
    create: bool,
) -> Result<File, PopulationStatisticsV2Refusal> {
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
) -> Result<(), PopulationStatisticsV2Refusal> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(format!(
            "population-statistics path {} is a symbolic link; no-follow refused it",
            path.display()
        )),
        Ok(_) => Ok(()),
        Err(why) if absent_is_allowed && why.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(why) => Err(format!(
            "cannot inspect population-statistics path {} without following links: {why}",
            path.display()
        )),
    }
}

fn require_regular_file(file: &File, path: &Path) -> Result<(), PopulationStatisticsV2Refusal> {
    if !file
        .metadata()
        .map_err(|why| {
            format!(
                "cannot stat population-statistics file {}: {why}",
                path.display()
            )
        })?
        .is_file()
    {
        return Err(format!(
            "population-statistics path {} is not a regular file",
            path.display()
        ));
    }
    Ok(())
}

fn file_generation(
    file: &File,
    path: &Path,
    max_bytes: u64,
) -> Result<FileGenerationV2, PopulationStatisticsV2Refusal> {
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
    let content_digest = hash_file(&mut named, path, measured_len)?;
    let repeated_digest = hash_file(&mut named, path, measured_len)?;
    if repeated_digest != content_digest {
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
    let zero_digest = [0_u8; 32];
    if generation_of(&held_before, zero_digest) != generation_of(&held_after, zero_digest)
        || generation_of(&named_before, zero_digest) != generation_of(&named_after, zero_digest)
        || generation_of(&named_after, zero_digest)
            != generation_of(&post_named_metadata, zero_digest)
    {
        return Err(format!(
            "{} changed while generation was measured",
            path.display()
        ));
    }
    Ok(generation_of(&held_after, content_digest))
}

#[cfg(unix)]
fn generation_of(metadata: &std::fs::Metadata, content_digest: [u8; 32]) -> FileGenerationV2 {
    FileGenerationV2 {
        len: metadata.len(),
        content_digest,
        device: metadata.dev(),
        inode: metadata.ino(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    }
}

#[cfg(not(unix))]
fn generation_of(metadata: &std::fs::Metadata, content_digest: [u8; 32]) -> FileGenerationV2 {
    FileGenerationV2 {
        len: metadata.len(),
        content_digest,
        modified: metadata.modified().ok(),
    }
}

fn hash_file(
    file: &mut File,
    path: &Path,
    exact_bytes: u64,
) -> Result<[u8; 32], PopulationStatisticsV2Refusal> {
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("cannot seek {} for generation hash: {why}", path.display()))?;
    let mut hasher = Hasher::new();
    hasher.update(GENERATION_DOMAIN);
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    let mut remaining = exact_bytes;
    while remaining != 0 {
        let requested = usize_of(
            remaining.min(READ_CHUNK_BYTES as u64),
            "generation hash chunk",
        )?;
        let read = file
            .read(
                buffer
                    .get_mut(..requested)
                    .ok_or_else(|| "generation hash request exceeded buffer".to_owned())?,
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
                .ok_or_else(|| "generation hash read exceeded buffer".to_owned())?,
        );
        remaining = remaining
            .checked_sub(
                u64::try_from(read)
                    .map_err(|why| format!("generation hash read width does not fit u64: {why}"))?,
            )
            .ok_or_else(|| "generation hash remaining-byte underflowed".to_owned())?;
    }
    Ok(hasher.finalize())
}

fn require_generation(
    expected: FileGenerationV2,
    file: &File,
    path: &Path,
) -> Result<(), PopulationStatisticsV2Refusal> {
    if file_generation(file, path, expected.len)? != expected {
        return Err(format!(
            "{} changed after population-statistics open; cached audit refused",
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

fn require_nonzero(name: &str, digest: [u8; 32]) -> Result<(), PopulationStatisticsV2Refusal> {
    if digest == [0; 32] {
        return Err(format!("population-statistics {name} digest is zero"));
    }
    Ok(())
}

fn require_finite(name: &str, bits: u64) -> Result<(), PopulationStatisticsV2Refusal> {
    if !f64::from_bits(bits).is_finite() {
        return Err(format!("population-statistics {name} is not finite"));
    }
    Ok(())
}

fn require_new_production_rung(rung_seconds: u32) -> Result<(), PopulationStatisticsV2Refusal> {
    if !CANDIDATE_SIGNAL_RUNGS_SECONDS_V1.contains(&rung_seconds) {
        return Err(format!(
            "population-statistics new-production rung {rung_seconds} is not one of the eight Candidate rungs"
        ));
    }
    Ok(())
}

fn require_stored_compatible_rung(rung_seconds: u32) -> Result<(), PopulationStatisticsV2Refusal> {
    if ![60, 120, 180, 300, 600, 900, 1_800, 3_600, 7_200, 14_400].contains(&rung_seconds) {
        return Err(format!(
            "population-statistics stored rung {rung_seconds} is neither current canonical nor legacy-compatible"
        ));
    }
    Ok(())
}

fn family_byte(family: InstrumentFamilyV1) -> u8 {
    match family {
        InstrumentFamilyV1::Nifty => 1,
        InstrumentFamilyV1::BankNifty => 2,
    }
}

fn family_from_byte(value: u8) -> Result<InstrumentFamilyV1, PopulationStatisticsV2Refusal> {
    match value {
        1 => Ok(InstrumentFamilyV1::Nifty),
        2 => Ok(InstrumentFamilyV1::BankNifty),
        _ => Err(format!(
            "population-statistics instrument family byte {value} is unknown"
        )),
    }
}

fn usize_of(value: u64, name: &str) -> Result<usize, PopulationStatisticsV2Refusal> {
    usize::try_from(value).map_err(|_| format!("population-statistics {name} does not fit usize"))
}

fn u64_of(value: usize, name: &str) -> Result<u64, PopulationStatisticsV2Refusal> {
    u64::try_from(value).map_err(|_| format!("population-statistics {name} does not fit u64"))
}

fn put_bytes(
    raw: &mut [u8],
    offset: usize,
    bytes: &[u8],
) -> Result<(), PopulationStatisticsV2Refusal> {
    let end = offset
        .checked_add(bytes.len())
        .ok_or_else(|| "population-statistics encode offset overflowed".to_owned())?;
    let destination = raw
        .get_mut(offset..end)
        .ok_or_else(|| "population-statistics encode destination is outside record".to_owned())?;
    destination.copy_from_slice(bytes);
    Ok(())
}

fn put_u8(raw: &mut [u8], offset: usize, value: u8) -> Result<(), PopulationStatisticsV2Refusal> {
    put_bytes(raw, offset, &[value])
}

fn put_u32(raw: &mut [u8], offset: usize, value: u32) -> Result<(), PopulationStatisticsV2Refusal> {
    put_bytes(raw, offset, &value.to_le_bytes())
}

fn put_u64(raw: &mut [u8], offset: usize, value: u64) -> Result<(), PopulationStatisticsV2Refusal> {
    put_bytes(raw, offset, &value.to_le_bytes())
}

fn put_i64(raw: &mut [u8], offset: usize, value: i64) -> Result<(), PopulationStatisticsV2Refusal> {
    put_bytes(raw, offset, &value.to_le_bytes())
}

fn get_u8(raw: &[u8], offset: usize) -> Result<u8, PopulationStatisticsV2Refusal> {
    raw.get(offset)
        .copied()
        .ok_or_else(|| "population-statistics u8 field is outside record".to_owned())
}

fn get_array<const N: usize>(
    raw: &[u8],
    offset: usize,
) -> Result<[u8; N], PopulationStatisticsV2Refusal> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| "population-statistics decode offset overflowed".to_owned())?;
    raw.get(offset..end)
        .ok_or_else(|| {
            format!(
                "population-statistics decode requested {offset}..{end} from {} bytes",
                raw.len()
            )
        })?
        .try_into()
        .map_err(|_| "population-statistics fixed decode width differs".to_owned())
}

fn get_u32(raw: &[u8], offset: usize) -> Result<u32, PopulationStatisticsV2Refusal> {
    Ok(u32::from_le_bytes(get_array(raw, offset)?))
}

fn get_u64(raw: &[u8], offset: usize) -> Result<u64, PopulationStatisticsV2Refusal> {
    Ok(u64::from_le_bytes(get_array(raw, offset)?))
}

fn get_i64(raw: &[u8], offset: usize) -> Result<i64, PopulationStatisticsV2Refusal> {
    Ok(i64::from_le_bytes(get_array(raw, offset)?))
}

fn get_32(raw: &[u8], offset: usize) -> Result<[u8; 32], PopulationStatisticsV2Refusal> {
    get_array(raw, offset)
}

fn require_zero(
    raw: &[u8],
    offset: usize,
    len: usize,
    name: &str,
) -> Result<(), PopulationStatisticsV2Refusal> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| format!("population-statistics {name} reserve overflowed"))?;
    if raw
        .get(offset..end)
        .ok_or_else(|| format!("population-statistics {name} reserve is absent"))?
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err(format!("population-statistics {name} reserve is nonzero"));
    }
    Ok(())
}

fn hex32(value: [u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in value {
        out.push(hex_digit(byte >> 4));
        out.push(hex_digit(byte & 0x0f));
    }
    out
}

fn hex_digit(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0' + nibble),
        10..=15 => char::from(b'a' + (nibble - 10)),
        _ => '?',
    }
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

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new(label: &str) -> Self {
            let nth = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "brutex-population-statistics-v2-{label}-{}-{nth}",
                std::process::id()
            ));
            std::fs::create_dir_all(&root).expect("temporary root is creatable");
            Self(root)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn digest(tag: u8) -> [u8; 32] {
        let mut value = [tag; 32];
        value[31] = tag.wrapping_add(1);
        value
    }

    fn bounds() -> PopulationStatisticsV2Bounds {
        PopulationStatisticsV2Bounds::new(8, 64, 64, 64, 2 * 1_024 * 1_024)
            .expect("fixture bounds are explicit and nonzero")
    }

    fn source(family: InstrumentFamilyV1, family_tag: u8, common_tag: u8) -> PreAdmissionSourceV2 {
        PreAdmissionSourceV2 {
            binding: PreAdmissionBindingV2 {
                family,
                sequence: 0,
                authority_id: digest(family_tag),
                candidate_universe_id: digest(family_tag.wrapping_add(1)),
                candidate_completion_digest: digest(family_tag.wrapping_add(2)),
                candidate_count: 1,
            },
            rung_seconds: 300,
            horizon_bars: 12,
            requested_span: RequestedSpanIdentityV1::new(2024, 1, 2024, 2)
                .expect("fixture span is valid"),
            feed_digest: digest(common_tag),
            source_commit_digest: digest(common_tag.wrapping_add(1)),
            calendar_policy_digest: digest(common_tag.wrapping_add(2)),
            daily_reference_policy_digest: digest(common_tag.wrapping_add(3)),
        }
    }

    fn raw_candidate(family: InstrumentFamilyV1, tag: u8, returns: [i64; 4]) -> RawCandidateV2 {
        let periods = returns
            .into_iter()
            .enumerate()
            .map(|(period, return_paisa)| RawPeriodV2 {
                identity: digest(100_u8.wrapping_add(u8::try_from(period).expect("small period"))),
                return_paisa,
                trades: u64::try_from(period).expect("small period") + 2,
                wins: u64::from(period % 2 == 0) + 1,
            })
            .collect();
        RawCandidateV2 {
            family,
            semantic_digest: digest(tag),
            periods,
        }
    }

    fn fixture_at_rung(
        tag: u8,
        rung_seconds: u32,
    ) -> Result<PreparedPopulationStatisticsV2, PopulationStatisticsV2Refusal> {
        let mut nifty = source(InstrumentFamilyV1::Nifty, tag, 40);
        let mut banknifty = source(InstrumentFamilyV1::BankNifty, tag.wrapping_add(10), 40);
        nifty.rung_seconds = rung_seconds;
        banknifty.rung_seconds = rung_seconds;
        let candidates = [
            raw_candidate(
                InstrumentFamilyV1::Nifty,
                tag.wrapping_add(20),
                [10, 20, -5, 15],
            ),
            raw_candidate(
                InstrumentFamilyV1::BankNifty,
                tag.wrapping_add(21),
                [-5, 10, 20, -10],
            ),
        ];
        let splits = [RawSplitV2 {
            train_mask: 2,
            test_mask: 1,
            train_scores: vec![10, 5],
            test_scores: vec![0, 10],
        }];
        PreparedPopulationStatisticsV2::new(
            &nifty,
            &banknifty,
            &candidates,
            &splits,
            StatisticsProcedureInputsV2 {
                segment_count: 2,
                draws: 16,
                seed: 77,
                block_length: 2,
            },
        )
    }

    fn fixture(tag: u8) -> PreparedPopulationStatisticsV2 {
        fixture_at_rung(tag, 300).expect("fixture is a complete paired statistical family")
    }

    fn observation_facts(tag: u8) -> ObservationAuthorityFactsV2 {
        ObservationAuthorityFactsV2 {
            authority_id: digest(tag),
            pair_identity: digest(tag.wrapping_add(1)),
            source_identity: digest(tag.wrapping_add(2)),
            observation_policy_digest: digest(tag.wrapping_add(3)),
            layout_policy_digest: digest(tag.wrapping_add(4)),
            candidate_count: 2,
            period_count: 4,
            split_count: 1,
            split_score_row_count: 2,
            data_record_digest: digest(tag.wrapping_add(5)),
            completion_digest: digest(tag.wrapping_add(6)),
        }
    }

    #[test]
    fn production_procedure_is_explicit_and_refuses_zero_resource_terms() {
        assert_eq!(
            PopulationStatisticsProcedureV2::new(17, 0, 3),
            Ok(PopulationStatisticsProcedureV2 {
                draws: 17,
                seed: 0,
                block_length: 3,
            })
        );
        for (draws, block_length) in [(0, 1), (1, 0), (0, 0)] {
            assert!(matches!(
                PopulationStatisticsProcedureV2::new(draws, 9, block_length),
                Err(why) if why.contains("must be nonzero")
            ));
        }
    }

    #[test]
    fn short_candidate_rungs_traverse_statistics_and_legacy_manifests_are_read_only() {
        for (tag, rung_seconds) in [(74, 120), (75, 180)] {
            let prepared = fixture_at_rung(tag, rung_seconds)
                .expect("current Candidate rung prepares Statistics V2");
            let records = prepared
                .records(0, 0)
                .expect("current Candidate rung encodes a complete Statistics V2 block");
            let first = records.first().expect("Data record is present");
            let decoded = decode_record(first, 0).expect("Data record seal validates");
            let manifest = decode_manifest(&decoded.payload, decoded.kind)
                .expect("current Candidate rung decodes through stored validation");
            assert_eq!(manifest.rung_seconds, rung_seconds);
        }

        let prepared = fixture(76);
        for rung_seconds in [7_200, 14_400] {
            let mut manifest = prepared.manifest;
            manifest.rung_seconds = rung_seconds;
            manifest.audit_id =
                derive_audit_id(manifest).expect("legacy rung has a canonical audit identity");
            manifest
                .validate_stored_compatible()
                .expect("legacy manifest remains readable");
            assert!(matches!(
                manifest.validate(),
                Err(why) if why.contains("new-production rung")
            ));
            let raw = encode_stored_compatible_manifest_record(&manifest, RecordKindV2::Data, 0)
                .expect("test-only legacy-compatible manifest encodes");
            let decoded = decode_record(&raw, 0).expect("legacy record seal validates");
            assert_eq!(
                decode_manifest(&decoded.payload, decoded.kind),
                Ok(manifest)
            );
            assert!(matches!(
                encode_manifest_record(&manifest, RecordKindV2::Data, 0),
                Err(why) if why.contains("new-production rung")
            ));
            assert!(matches!(
                fixture_at_rung(77, rung_seconds),
                Err(why) if why.contains("new-production rung")
            ));
        }

        let mut unsupported = prepared.manifest;
        unsupported.rung_seconds = 240;
        unsupported.audit_id =
            derive_audit_id(unsupported).expect("unknown rung still has deterministic identity");
        assert!(matches!(
            unsupported.validate_stored_compatible(),
            Err(why) if why.contains("neither current canonical nor legacy-compatible")
        ));
    }

    #[test]
    fn detached_observation_link_is_deterministic_sensitive_and_does_not_change_v2_bytes() {
        let prepared = fixture(70);
        let before = prepared
            .records(0, 0)
            .expect("existing Statistics V2 bytes encode");
        let facts = observation_facts(80);
        let procedure =
            PopulationStatisticsProcedureV2::new(16, 77, 2).expect("explicit procedure is valid");
        let first = observation_statistics_link(facts, &prepared.manifest, procedure)
            .expect("exact Observation facts link");
        let second = observation_statistics_link(facts, &prepared.manifest, procedure)
            .expect("identical facts link identically");
        assert_eq!(first, second);
        assert_eq!(first.statistics_audit_id(), prepared.manifest.audit_id);
        assert_eq!(first.observation_authority_id(), facts.authority_id);
        assert_eq!(first.observation_pair_identity(), facts.pair_identity);

        let mut changed_completion = facts;
        changed_completion.completion_digest = digest(99);
        let changed =
            observation_statistics_link(changed_completion, &prepared.manifest, procedure)
                .expect("changed valid facts still form a distinct link");
        assert_ne!(first.identity(), changed.identity());

        let changed_procedure = observation_statistics_link(
            facts,
            &prepared.manifest,
            PopulationStatisticsProcedureV2::new(17, 77, 2)
                .expect("changed explicit procedure is valid"),
        )
        .expect("changed procedure forms a distinct link");
        assert_ne!(first.identity(), changed_procedure.identity());
        assert_eq!(
            before,
            prepared
                .records(0, 0)
                .expect("detached link leaves Statistics V2 bytes unchanged")
        );
    }

    #[test]
    fn observation_link_accepts_only_its_exact_reopened_statistics_audit() {
        let root = TempRoot::new("observation-link");
        let first_prepared = fixture(71);
        let second_prepared = fixture(72);
        let procedure =
            PopulationStatisticsProcedureV2::new(16, 77, 2).expect("explicit procedure is valid");
        let link =
            observation_statistics_link(observation_facts(81), &first_prepared.manifest, procedure)
                .expect("exact link prepares");
        let first = append_population_statistics_v2(root.path(), bounds(), &first_prepared)
            .expect("linked Statistics V2 bytes reopen")
            .audit();
        let foreign = append_population_statistics_v2(root.path(), bounds(), &second_prepared)
            .expect("foreign Statistics V2 bytes also reopen")
            .audit();
        assert_eq!(link.require_reopened_statistics(first), Ok(()));
        assert!(matches!(
            link.require_reopened_statistics(foreign),
            Err(why) if why.contains("foreign reopened statistics audit")
        ));
    }

    #[test]
    fn observation_period_link_is_shared_per_period_and_authority_sensitive() {
        let facts = observation_facts(82);
        let shared = observation_period_identity(facts, 3, 20_001);
        assert_eq!(shared, observation_period_identity(facts, 3, 20_001));
        assert_ne!(shared, observation_period_identity(facts, 4, 20_001));
        assert_ne!(shared, observation_period_identity(facts, 3, 20_002));
        let mut foreign = facts;
        foreign.authority_id = digest(120);
        assert_ne!(shared, observation_period_identity(foreign, 3, 20_001));
    }

    #[test]
    fn observation_link_refuses_any_cardinality_not_in_statistics_bytes() {
        let prepared = fixture(73);
        let procedure =
            PopulationStatisticsProcedureV2::new(16, 77, 2).expect("explicit procedure is valid");
        for field in 0..4 {
            let mut facts = observation_facts(83);
            match field {
                0 => facts.candidate_count = facts.candidate_count.saturating_add(1),
                1 => facts.period_count = facts.period_count.saturating_add(1),
                2 => facts.split_count = facts.split_count.saturating_add(1),
                _ => {
                    facts.split_score_row_count = facts.split_score_row_count.saturating_add(1);
                }
            }
            assert!(matches!(
                observation_statistics_link(facts, &prepared.manifest, procedure),
                Err(why) if why.contains("cardinalities differ")
            ));
        }
    }

    fn write_bytes(path: &Path, bytes: &[u8]) {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("fixture file opens");
        file.seek(SeekFrom::End(0)).expect("fixture seeks");
        file.write_all(bytes).expect("fixture bytes append");
        file.sync_all().expect("fixture bytes sync");
    }

    fn rewrite_resealed_record(path: &Path, index: u64, edit: impl FnOnce(&mut [u8])) {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("fixture file opens");
        let mut raw = [0_u8; RECORD_BYTES];
        file.seek(SeekFrom::Start(record_offset(index).expect("offset")))
            .and_then(|_| file.read_exact(&mut raw))
            .expect("record reads");
        edit(&mut raw[..PAYLOAD_BYTES]);
        let seal = digest_domain(RECORD_DOMAIN, &raw[..PAYLOAD_BYTES]);
        raw[PAYLOAD_BYTES..].copy_from_slice(&seal);
        file.seek(SeekFrom::Start(record_offset(index).expect("offset")))
            .and_then(|_| file.write_all(&raw))
            .and_then(|()| file.sync_all())
            .expect("resealed record writes");
    }

    #[test]
    fn complete_pair_recomputes_reopens_pages_and_exactly_reuses() {
        let root = TempRoot::new("reopen");
        let prepared = fixture(1);
        let mut ledger = PopulationStatisticsV2Ledger::open_writer(root.path(), bounds())
            .expect("writable fixture ledger opens");
        let written = ledger.append(&prepared).expect("first append writes");
        assert!(matches!(written, PopulationStatisticsV2Append::Written(_)));
        let audit = written.audit();
        assert_eq!(audit.candidate_count(), 2);
        assert_eq!(audit.nifty_candidate_count(), 1);
        assert_eq!(audit.banknifty_candidate_count(), 1);
        assert_eq!(audit.period_count(), 4);
        assert_eq!(audit.split_count(), 1);
        assert_eq!(audit.cscv_pbo().contributing_splits(), 1);
        assert_eq!(audit.cscv_pbo().bottom_half_splits(), 1);
        assert_eq!(audit.cscv_pbo().exact_probability().numerator(), 1);
        let reused = ledger.append(&prepared).expect("exact retry reuses");
        assert!(matches!(reused, PopulationStatisticsV2Append::Reused(_)));
        drop(ledger);

        let mut reopened = PopulationStatisticsV2Ledger::open_read(root.path(), bounds())
            .expect("complete bytes reopen and recompute");
        assert_eq!(reopened.completed_audits(), 1);
        let found = reopened
            .reopen_audit(&audit.audit_id())
            .expect("lookup works")
            .expect("audit exists");
        assert_eq!(found, audit);
        let page = reopened
            .page_candidates(&audit.audit_id(), 0, 25)
            .expect("bounded page reads");
        assert_eq!(page.rows().len(), 2);
        assert_eq!(page.rows()[0].family(), InstrumentFamilyV1::Nifty);
        assert_eq!(page.rows()[1].family(), InstrumentFamilyV1::BankNifty);
        assert_eq!(
            reopened
                .candidate(&audit.audit_id(), 1)
                .expect("fixed candidate offset reads"),
            page.rows()[1]
        );
    }

    #[test]
    fn completion_digest_binds_exact_receipt_bytes_and_fresh_reopen() {
        let root = TempRoot::new("completion-digest");
        let prepared = fixture(21);
        let planned = prepared
            .records(0, 0)
            .expect("canonical first block records encode");
        let completion = planned.last().expect("planned Completion is present");
        let completion_payload: &[u8; PAYLOAD_BYTES] = completion
            .get(..PAYLOAD_BYTES)
            .expect("Completion payload is present")
            .try_into()
            .expect("Completion payload has fixed width");
        let expected_digest = digest_completion_record(completion_payload);

        let first = append_population_statistics_v2(root.path(), bounds(), &prepared)
            .expect("receipt-last block freshly reopens")
            .audit();
        assert_eq!(first.completion_record_digest(), expected_digest);
        let reused = append_population_statistics_v2(root.path(), bounds(), &prepared)
            .expect("byte-identical retry freshly reopens")
            .audit();
        assert_eq!(reused, first);
        assert_eq!(reused.completion_record_digest(), expected_digest);

        let mut independent = PopulationStatisticsV2Ledger::open_read(root.path(), bounds())
            .expect("independent reader validates every source row");
        let independently_reopened = independent
            .reopen_audit(&first.audit_id())
            .expect("fresh audit lookup succeeds")
            .expect("fresh audit exists");
        assert_eq!(independently_reopened, first);
        assert_eq!(
            independently_reopened.completion_record_digest(),
            expected_digest
        );

        let shifted_root = TempRoot::new("completion-digest-shifted");
        append_population_statistics_v2(shifted_root.path(), bounds(), &fixture(22))
            .expect("leading block writes");
        let shifted = append_population_statistics_v2(shifted_root.path(), bounds(), &prepared)
            .expect("same semantic family writes at a later physical position")
            .audit();
        assert_eq!(shifted.audit_id(), first.audit_id());
        assert_ne!(
            shifted.completion_record_digest(),
            first.completion_record_digest(),
            "receipt identity must bind logical and physical sequence bytes"
        );
    }

    #[test]
    fn opaque_projection_exposes_only_fresh_manifest_and_link_terms() {
        let root = TempRoot::new("projection-source");
        let prepared = fixture(23);
        let facts = observation_facts(123);
        let procedure =
            PopulationStatisticsProcedureV2::new(16, 77, 2).expect("fixture procedure is explicit");
        let link = observation_statistics_link(facts, &prepared.manifest, procedure)
            .expect("detached observation link prepares");
        let statistics = append_population_statistics_v2(root.path(), bounds(), &prepared)
            .expect("Statistics bytes freshly reopen");
        let commit = PopulationStatisticsObservationCommitV2 { statistics, link };
        let source = commit
            .projection_source()
            .expect("exact linked commit produces an opaque source");
        let manifest = prepared.manifest.with_sequence(0);
        assert_eq!(source.audit_id(), manifest.audit_id);
        assert_eq!(
            source.completion_record_digest(),
            commit.audit().completion_record_digest()
        );
        assert_eq!(source.observation_statistics_link_id(), link.identity());
        assert_eq!(source.observation_authority_id(), facts.authority_id);
        assert_eq!(source.observation_pair_identity(), facts.pair_identity);
        assert_eq!(source.feed_digest(), manifest.feed_digest);
        assert_eq!(source.source_commit_digest(), manifest.source_commit_digest);
        assert_eq!(
            source.calendar_policy_digest(),
            manifest.calendar_policy_digest
        );
        assert_eq!(
            source.daily_reference_policy_digest(),
            manifest.daily_reference_policy_digest
        );
        assert_eq!(source.wilson_policy_digest(), manifest.wilson_policy_digest);
        assert_eq!(source.cscv_policy_digest(), manifest.cscv_policy_digest);
        assert_eq!(
            source.cscv_split_family_digest(),
            manifest.ordered_split_digest
        );
        assert_eq!(source.white(), manifest.white);
        assert_eq!(source.spa(), manifest.spa);
        assert_eq!(
            source.romano_wolf_family_digest(),
            manifest.romano_wolf_family_digest
        );
        assert_eq!(source.bootstrap_draws(), manifest.draws);
        assert_eq!(source.candidate_count(), 2);

        let nifty_source = source.family_source(InstrumentFamilyV1::Nifty);
        assert_eq!(nifty_source.family(), InstrumentFamilyV1::Nifty);
        assert_eq!(
            nifty_source.pre_admission_authority_id(),
            manifest.nifty.authority_id
        );
        assert_eq!(
            nifty_source.candidate_universe_id(),
            manifest.nifty.candidate_universe_id
        );
        assert_eq!(
            nifty_source.candidate_completion_digest(),
            manifest.nifty.candidate_completion_digest
        );
    }

    #[test]
    fn opaque_projection_candidate_is_exact_and_source_bound() {
        let root = TempRoot::new("projection-candidate");
        let prepared = fixture(27);
        let facts = observation_facts(127);
        let procedure =
            PopulationStatisticsProcedureV2::new(16, 77, 2).expect("fixture procedure is explicit");
        let link = observation_statistics_link(facts, &prepared.manifest, procedure)
            .expect("detached observation link prepares");
        let statistics = append_population_statistics_v2(root.path(), bounds(), &prepared)
            .expect("Statistics bytes freshly reopen");
        let source = PopulationStatisticsObservationCommitV2 { statistics, link }
            .projection_source()
            .expect("exact linked commit produces an opaque source");
        let mut reader = PopulationStatisticsV2Ledger::open_read(root.path(), bounds())
            .expect("projection reader freshly opens");
        let candidate = reader
            .projection_candidate(source, 0)
            .expect("source-bound fixed candidate reads");
        let expected = prepared.candidates[0];
        assert_eq!(candidate.source(), source);
        assert_eq!(candidate.sequence(), expected.sequence);
        assert_eq!(candidate.family(), expected.family);
        assert_eq!(candidate.family_sequence(), expected.family_sequence);
        assert_eq!(
            candidate.candidate_semantic_digest(),
            expected.candidate_semantic_digest
        );
        assert_eq!(candidate.trades(), expected.trades);
        assert_eq!(candidate.wins(), expected.wins);
        assert_eq!(candidate.wilson_lower_bits(), expected.wilson_lower_bits);
        assert_eq!(
            candidate.romano_wolf_statistic_bits(),
            expected.romano_wolf_statistic_bits
        );
        assert_eq!(candidate.romano_wolf_rank(), expected.romano_wolf_rank);
        assert_eq!(
            candidate.romano_wolf_strict_exceedances(),
            expected.romano_wolf_strict_exceedances
        );
        assert_eq!(
            candidate.romano_wolf_initial(),
            expected.romano_wolf_initial
        );
        assert_eq!(
            candidate.romano_wolf_adjusted(),
            expected.romano_wolf_adjusted
        );

        let foreign_root = TempRoot::new("projection-source-foreign");
        append_population_statistics_v2(foreign_root.path(), bounds(), &fixture(24))
            .expect("foreign Statistics bytes freshly reopen");
        let mut foreign_reader =
            PopulationStatisticsV2Ledger::open_read(foreign_root.path(), bounds())
                .expect("foreign reader opens");
        assert!(matches!(
            foreign_reader.projection_candidate(source, 0),
            Err(why) if why.contains("projection source is absent")
        ));
    }

    fn assert_admission_candidate_v3(
        projected: &PopulationStatisticsAdmissionCandidateV3,
        expected: &PopulationStatisticsCandidateV2,
        source: &PopulationStatisticsV2ProjectionSource,
        expected_familywise: PopulationStatisticsFractionV2,
    ) {
        let draft = projected.draft();
        assert_eq!(projected.source(), source);
        assert_eq!(projected.sequence(), expected.sequence);
        assert_eq!(projected.family(), expected.family);
        assert_eq!(projected.family_sequence(), expected.family_sequence);
        assert_eq!(
            projected.pre_admission_authority_id(),
            expected.pre_admission_id
        );
        assert_eq!(
            projected.candidate_semantic_digest(),
            expected.candidate_semantic_digest
        );
        assert_eq!(
            projected.ordered_candidate_digest(),
            source.ordered_candidate_digest()
        );
        assert_eq!(
            projected.ordered_period_family_digest(),
            source.ordered_period_digest()
        );
        assert_eq!(
            projected.candidate_ordered_period_digest(),
            expected.ordered_period_digest
        );
        assert_eq!(
            projected.ordered_split_family_digest(),
            source.cscv_split_family_digest()
        );
        assert_eq!(
            projected.candidate_ordered_split_digest(),
            expected.ordered_split_digest
        );
        assert_eq!(projected.candidate_count(), source.candidate_count());
        assert_eq!(projected.period_count(), source.period_count());
        assert_eq!(projected.split_count(), source.split_count());
        assert_eq!(
            draft.candidate_semantic_id,
            expected.candidate_semantic_digest
        );
        assert_eq!(draft.statistics_audit_id, source.audit_id());
        assert_eq!(
            draft.statistics_completion_digest,
            source.completion_record_digest()
        );
        assert_eq!(draft.trades, expected.trades);
        assert_eq!(draft.wins, expected.wins);
        assert_eq!(draft.wilson_lower_bits, expected.wilson_lower_bits);
        assert_eq!(
            draft.wilson_win_rate_ppm,
            admission_wilson_ppm_v3(expected.wilson_lower_bits)
                .expect("stored Wilson bits project")
        );
        assert_eq!(
            draft.familywise_romano_wolf_probability.numerator(),
            expected_familywise.numerator()
        );
        assert_eq!(
            draft.candidate_romano_wolf_probability.numerator(),
            expected.romano_wolf_adjusted.numerator()
        );
        assert_eq!(
            draft.candidate_romano_wolf_probability.denominator(),
            expected.romano_wolf_adjusted.denominator()
        );
    }

    #[test]
    fn admission_v3_projection_uses_complete_family_and_fixed_candidate_rows() {
        let root = TempRoot::new("admission-v3-projection");
        let prepared = fixture(93);
        let facts = observation_facts(193);
        let procedure =
            PopulationStatisticsProcedureV2::new(16, 77, 2).expect("fixture procedure is explicit");
        let link = observation_statistics_link(facts, &prepared.manifest, procedure)
            .expect("detached Observation link prepares");
        let statistics = append_population_statistics_v2(root.path(), bounds(), &prepared)
            .expect("Statistics bytes freshly reopen");
        let source = PopulationStatisticsObservationCommitV2 { statistics, link }
            .projection_source()
            .expect("exact linked commit produces a source");
        let mut reader = PopulationStatisticsV2Ledger::open_read(root.path(), bounds())
            .expect("Statistics projection reader opens");
        let authority = reader
            .prepare_admission_projection_v3(&source)
            .expect("complete family produces Admission V3 projection authority");
        let expected_familywise = prepared
            .candidates
            .iter()
            .find(|candidate| candidate.romano_wolf_rank == 0)
            .expect("complete family has rank zero")
            .romano_wolf_adjusted;
        assert_eq!(authority.source(), &source);
        assert_eq!(
            authority.familywise_romano_wolf_probability(),
            expected_familywise
        );

        for expected in prepared.candidates.iter().copied() {
            let projected = reader
                .admission_candidate_v3(&authority, expected.sequence)
                .expect("fixed candidate projects from authenticated family");
            assert_admission_candidate_v3(&projected, &expected, &source, expected_familywise);
        }
        let projected_family = reader
            .admission_candidates_v3(&authority)
            .expect("complete family projects under one generation validation");
        assert_eq!(projected_family.len(), prepared.candidates.len());
        for (projected, expected) in projected_family.iter().zip(prepared.candidates.iter()) {
            assert_admission_candidate_v3(projected, expected, &source, expected_familywise);
        }
        assert!(matches!(
            reader.admission_candidate_v3(&authority, source.candidate_count()),
            Err(why) if why.contains("outside audit")
        ));

        let foreign_root = TempRoot::new("admission-v3-projection-foreign");
        append_population_statistics_v2(foreign_root.path(), bounds(), &fixture(94))
            .expect("foreign Statistics bytes freshly reopen");
        let mut foreign_reader =
            PopulationStatisticsV2Ledger::open_read(foreign_root.path(), bounds())
                .expect("foreign reader opens");
        assert!(matches!(
            foreign_reader.admission_candidate_v3(&authority, 0),
            Err(why) if why.contains("source is absent")
        ));
    }

    #[test]
    fn corrupt_or_semantically_resealed_completion_never_mints_a_digest() {
        let corrupt_root = TempRoot::new("completion-corrupt");
        let corrupt_prepared = fixture(25);
        let corrupt_planned = corrupt_prepared
            .records(0, 0)
            .expect("corrupt fixture plan encodes");
        append_population_statistics_v2(corrupt_root.path(), bounds(), &corrupt_prepared)
            .expect("corrupt fixture initially reopens");
        let completion_index =
            u64::try_from(corrupt_planned.len() - 1).expect("completion index fits u64");
        let path = corrupt_root.path().join(DATA_FILE);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("completion file opens");
        let seal_byte = record_offset(completion_index)
            .expect("completion record offset exists")
            .checked_add(u64::try_from(PAYLOAD_BYTES).expect("payload width fits u64"))
            .expect("completion seal offset does not overflow");
        let mut original = [0_u8; 1];
        file.seek(SeekFrom::Start(seal_byte))
            .and_then(|_| file.read_exact(&mut original))
            .expect("completion seal byte reads");
        original[0] ^= 0xA5;
        file.seek(SeekFrom::Start(seal_byte))
            .and_then(|_| file.write_all(&original))
            .and_then(|()| file.sync_all())
            .expect("completion seal corruption writes");
        drop(file);
        assert!(matches!(
            PopulationStatisticsV2Ledger::open_read(corrupt_root.path(), bounds()),
            Err(why) if why.contains("record seal differs")
        ));

        let resealed_root = TempRoot::new("completion-resealed");
        let resealed_prepared = fixture(26);
        let resealed_planned = resealed_prepared
            .records(0, 0)
            .expect("resealed fixture plan encodes");
        append_population_statistics_v2(resealed_root.path(), bounds(), &resealed_prepared)
            .expect("resealed fixture initially reopens");
        let resealed_completion =
            u64::try_from(resealed_planned.len() - 1).expect("resealed completion index fits u64");
        rewrite_resealed_record(
            &resealed_root.path().join(DATA_FILE),
            resealed_completion,
            |payload| {
                let first = payload
                    .get_mut(368)
                    .expect("Completion feed digest field exists");
                *first ^= 0x01;
            },
        );
        assert!(matches!(
            PopulationStatisticsV2Ledger::open_read(resealed_root.path(), bounds()),
            Err(why) if why.contains("audit identity differs from fields")
        ));
    }

    #[test]
    fn opaque_writer_freshly_reopens_and_preserves_written_or_reused() {
        let root = TempRoot::new("production-writer");
        let prepared = fixture(31);
        let written = append_population_statistics_v2(root.path(), bounds(), &prepared)
            .expect("opaque prepared block writes and freshly reopens");
        assert!(matches!(written, PopulationStatisticsV2Append::Written(_)));
        let expected = written.audit();

        let reused = append_population_statistics_v2(root.path(), bounds(), &prepared)
            .expect("byte-identical opaque retry freshly reopens");
        assert!(matches!(reused, PopulationStatisticsV2Append::Reused(_)));
        assert_eq!(reused.audit(), expected);

        let mut reader = PopulationStatisticsV2Ledger::open_read(root.path(), bounds())
            .expect("independent read-only ledger opens");
        assert_eq!(
            reader
                .reopen_audit(&expected.audit_id())
                .expect("audit lookup validates generation"),
            Some(expected)
        );
    }

    #[test]
    fn writer_refuses_missing_or_non_directory_root_without_creating_it() {
        let parent = TempRoot::new("root-refusal");
        let prepared = fixture(32);
        let missing = parent.path().join("missing");
        let why = append_population_statistics_v2(&missing, bounds(), &prepared)
            .expect_err("missing writer root refuses");
        assert!(why.contains("must already exist"));
        assert!(!missing.exists());

        let file = parent.path().join("regular-file");
        std::fs::write(&file, b"not a directory").expect("fixture file writes");
        let why = append_population_statistics_v2(&file, bounds(), &prepared)
            .expect_err("regular-file writer root refuses");
        assert!(why.contains("is not a directory"));
        assert_eq!(
            std::fs::read(&file).expect("refused file remains readable"),
            b"not a directory"
        );
    }

    #[cfg(unix)]
    #[test]
    fn pre_existing_lock_and_data_symlinks_refuse_without_touching_external_targets() {
        use std::os::unix::fs::symlink;

        let prepared = fixture(33);

        let external_lock_root = TempRoot::new("external-lock-target");
        let external_lock = external_lock_root.path().join("outside.lock");
        std::fs::write(&external_lock, b"").expect("external lock target writes");
        let linked_lock_root = TempRoot::new("linked-lock-root");
        symlink(&external_lock, linked_lock_root.path().join(LOCK_FILE))
            .expect("pre-existing lock symlink is creatable");
        let why = append_population_statistics_v2(linked_lock_root.path(), bounds(), &prepared)
            .expect_err("writer must not follow a pre-existing lock symlink");
        assert!(why.contains("symbolic link"));
        assert_eq!(
            std::fs::read(&external_lock).expect("external lock target remains readable"),
            b""
        );
        assert!(!linked_lock_root.path().join(DATA_FILE).exists());

        let external_data_root = TempRoot::new("external-data-target");
        let external_data = external_data_root.path().join("outside.bin");
        std::fs::write(&external_data, b"").expect("external data target writes");
        let linked_data_root = TempRoot::new("linked-data-root");
        std::fs::write(linked_data_root.path().join(LOCK_FILE), b"")
            .expect("regular in-root lock writes");
        symlink(&external_data, linked_data_root.path().join(DATA_FILE))
            .expect("pre-existing data symlink is creatable");
        let why = append_population_statistics_v2(linked_data_root.path(), bounds(), &prepared)
            .expect_err("writer must not follow a pre-existing data symlink");
        assert!(why.contains("symbolic link"));
        assert_eq!(
            std::fs::read(&external_data).expect("external data target remains readable"),
            b""
        );
    }

    #[test]
    fn exact_trailing_prefix_retry_completes_and_foreign_retry_refuses() {
        let root = TempRoot::new("orphan");
        let prepared = fixture(2);
        let ledger = PopulationStatisticsV2Ledger::open_writer(root.path(), bounds())
            .expect("empty fixture ledger opens");
        let planned = prepared.records(0, 0).expect("planned bytes build");
        drop(ledger);
        let data_path = root.path().join(DATA_FILE);
        let prefix_len = planned.len() / 2;
        for raw in planned
            .get(..prefix_len)
            .expect("fixture prefix is inside plan")
        {
            write_bytes(&data_path, raw);
        }
        let mut orphaned = PopulationStatisticsV2Ledger::open_writer(root.path(), bounds())
            .expect("one valid trailing prefix is recoverable");
        assert_eq!(orphaned.completed_audits(), 0);
        let prefix = orphaned
            .trailing_prefix_audit()
            .expect("prefix generation remains current")
            .expect("trailing prefix is reported");
        assert_eq!(prefix.audit_id(), prepared.manifest.audit_id);
        assert_eq!(
            prefix.present_records(),
            u64::try_from(prefix_len).expect("fixture prefix fits u64")
        );
        assert!(prefix.present_records() < prefix.planned_records());
        let foreign = fixture(3);
        let why = orphaned
            .append(&foreign)
            .expect_err("foreign orphan retry refuses");
        assert!(why.contains("trailing orphan belongs"));
        let completed = orphaned
            .append(&prepared)
            .expect("byte-identical orphan retry completes");
        assert!(matches!(
            completed,
            PopulationStatisticsV2Append::Written(_)
        ));
        drop(orphaned);
        PopulationStatisticsV2Ledger::open_read(root.path(), bounds())
            .expect("completed orphan reopens");
    }

    #[test]
    fn mismatched_pre_admission_identity_candidate_order_and_cscv_family_refuse() {
        let nifty = source(InstrumentFamilyV1::Nifty, 1, 40);
        let candidates = [
            raw_candidate(InstrumentFamilyV1::Nifty, 10, [1, 3, -1, 2]),
            raw_candidate(InstrumentFamilyV1::BankNifty, 11, [2, -1, 4, 1]),
        ];
        let split = [RawSplitV2 {
            train_mask: 2,
            test_mask: 1,
            train_scores: vec![1, 2],
            test_scores: vec![2, 1],
        }];
        for mutation in 0..6 {
            let mut bank = source(InstrumentFamilyV1::BankNifty, 2, 40);
            match mutation {
                0 => bank.rung_seconds = 600,
                1 => bank.horizon_bars += 1,
                2 => {
                    bank.requested_span =
                        RequestedSpanIdentityV1::new(2024, 1, 2024, 3).expect("mutated span valid");
                }
                3 => bank.feed_digest = digest(90),
                4 => bank.source_commit_digest = digest(91),
                _ => bank.daily_reference_policy_digest = digest(92),
            }
            assert!(
                PreparedPopulationStatisticsV2::new(
                    &nifty,
                    &bank,
                    &candidates,
                    &split,
                    StatisticsProcedureInputsV2 {
                        segment_count: 2,
                        draws: 8,
                        seed: 1,
                        block_length: 2,
                    },
                )
                .is_err(),
                "pre-admission mutation {mutation} must refuse"
            );
        }
        let reordered = [candidates[1].clone(), candidates[0].clone()];
        assert!(
            PreparedPopulationStatisticsV2::new(
                &nifty,
                &source(InstrumentFamilyV1::BankNifty, 2, 40),
                &reordered,
                &split,
                StatisticsProcedureInputsV2 {
                    segment_count: 2,
                    draws: 8,
                    seed: 1,
                    block_length: 2,
                },
            )
            .is_err()
        );
        let duplicate = [
            candidates[0].clone(),
            RawCandidateV2 {
                family: InstrumentFamilyV1::BankNifty,
                ..candidates[0].clone()
            },
        ];
        assert!(
            PreparedPopulationStatisticsV2::new(
                &nifty,
                &source(InstrumentFamilyV1::BankNifty, 2, 40),
                &duplicate,
                &split,
                StatisticsProcedureInputsV2 {
                    segment_count: 2,
                    draws: 8,
                    seed: 1,
                    block_length: 2,
                },
            )
            .is_err()
        );
        let bad_split = [RawSplitV2 {
            train_mask: 1,
            test_mask: 2,
            ..split[0].clone()
        }];
        assert!(
            PreparedPopulationStatisticsV2::new(
                &source(InstrumentFamilyV1::Nifty, 1, 40),
                &source(InstrumentFamilyV1::BankNifty, 2, 40),
                &candidates,
                &bad_split,
                StatisticsProcedureInputsV2 {
                    segment_count: 2,
                    draws: 8,
                    seed: 1,
                    block_length: 2,
                },
            )
            .is_err()
        );
    }

    #[test]
    fn ragged_corrupt_and_semantically_resealed_sources_refuse() {
        let ragged_root = TempRoot::new("ragged");
        let mut ledger = PopulationStatisticsV2Ledger::open_writer(ragged_root.path(), bounds())
            .expect("fixture ledger opens");
        ledger.append(&fixture(4)).expect("fixture writes");
        drop(ledger);
        write_bytes(&ragged_root.path().join(DATA_FILE), &[1]);
        assert!(
            PopulationStatisticsV2Ledger::open_read(ragged_root.path(), bounds())
                .err()
                .expect("ragged tail refuses")
                .contains("ragged")
        );

        let corrupt_root = TempRoot::new("corrupt");
        let mut ledger = PopulationStatisticsV2Ledger::open_writer(corrupt_root.path(), bounds())
            .expect("fixture ledger opens");
        ledger.append(&fixture(5)).expect("fixture writes");
        drop(ledger);
        let path = corrupt_root.path().join(DATA_FILE);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("file opens");
        file.seek(SeekFrom::Start(record_offset(1).expect("offset") + 80))
            .and_then(|_| file.write_all(&[0xA5]))
            .and_then(|()| file.sync_all())
            .expect("corruption writes");
        assert!(
            PopulationStatisticsV2Ledger::open_read(corrupt_root.path(), bounds())
                .err()
                .expect("bad seal refuses")
                .contains("seal differs")
        );

        let resealed_root = TempRoot::new("resealed");
        let prepared = fixture(6);
        let mut ledger = PopulationStatisticsV2Ledger::open_writer(resealed_root.path(), bounds())
            .expect("fixture ledger opens");
        ledger.append(&prepared).expect("fixture writes");
        drop(ledger);
        let period_index =
            1 + u64::try_from(prepared.candidates.len()).expect("fixture candidate count fits u64");
        rewrite_resealed_record(
            &resealed_root.path().join(DATA_FILE),
            period_index,
            |payload| {
                put_i64(payload, 96, 999_999).expect("fixed period field exists");
            },
        );
        let why = PopulationStatisticsV2Ledger::open_read(resealed_root.path(), bounds())
            .err()
            .expect("resealed false period refuses recomputation");
        assert!(why.contains("digest differs") || why.contains("bootstrap evidence differs"));
    }

    #[test]
    fn explicit_bounds_and_post_open_same_length_mutation_refuse() {
        assert!(PopulationStatisticsV2Bounds::new(0, 1, 1, 1, 4_096).is_err());
        let root = TempRoot::new("bounds-stale");
        let prepared = fixture(7);
        let mut writer = PopulationStatisticsV2Ledger::open_writer(root.path(), bounds())
            .expect("fixture writer opens");
        let audit = writer.append(&prepared).expect("fixture writes").audit();
        drop(writer);
        let too_small = PopulationStatisticsV2Bounds::new(8, 1, 64, 64, 2 * 1_024 * 1_024)
            .expect("small semantic bound is valid");
        assert!(
            PopulationStatisticsV2Ledger::open_read(root.path(), too_small)
                .err()
                .expect("candidate ceiling refuses")
                .contains("semantic bounds")
        );
        let byte_tight = PopulationStatisticsV2Bounds::new(
            8,
            64,
            64,
            64,
            POPULATION_STATISTICS_V2_HEADER_BYTES + 2 * POPULATION_STATISTICS_V2_RECORD_STRIDE,
        )
        .expect("minimum nonzero byte ceiling is valid");
        assert!(
            PopulationStatisticsV2Ledger::open_read(root.path(), byte_tight)
                .err()
                .expect("oversized file refuses before generation hashing")
                .contains("above generation-hash maximum")
        );
        let mut reader = PopulationStatisticsV2Ledger::open_read(root.path(), bounds())
            .expect("reader opens before mutation");
        let path = root.path().join(DATA_FILE);
        rewrite_resealed_record(&path, 1, |payload| {
            let value = get_u64(payload, 136).expect("trade count reads");
            put_u64(payload, 136, value.saturating_add(1)).expect("fixed candidate field exists");
        });
        let why = reader
            .candidate(&audit.audit_id(), 0)
            .expect_err("same-length mutation invalidates cached handle");
        assert!(why.contains("changed after population-statistics open"));

        let during_root = TempRoot::new("generation-changes-during-action");
        let during_prepared = fixture(71);
        append_population_statistics_v2(during_root.path(), bounds(), &during_prepared)
            .expect("during-action fixture writes");
        let mut during_reader =
            PopulationStatisticsV2Ledger::open_read(during_root.path(), bounds())
                .expect("during-action reader opens");
        let during_path = during_root.path().join(DATA_FILE);
        let why = during_reader
            .with_shared_lock(|_| {
                rewrite_resealed_record(&during_path, 1, |payload| {
                    let value = get_u64(payload, 136).expect("trade count reads");
                    put_u64(payload, 136, value.saturating_add(1))
                        .expect("fixed candidate field exists");
                });
                Ok(())
            })
            .expect_err("post-action generation check catches non-cooperating mutation");
        assert!(why.contains("changed after population-statistics open"));
    }

    #[test]
    fn calendar_feed_commit_daily_and_span_are_audit_identity_terms() {
        let base = fixture(8);
        let base_id = base.manifest.audit_id;
        for mutation in 0..5 {
            let nifty = source(InstrumentFamilyV1::Nifty, 8, 40);
            let mut bank = source(InstrumentFamilyV1::BankNifty, 18, 40);
            let mut changed_nifty = nifty.clone();
            match mutation {
                0 => {
                    changed_nifty.feed_digest = digest(150);
                    bank.feed_digest = digest(150);
                }
                1 => {
                    changed_nifty.source_commit_digest = digest(151);
                    bank.source_commit_digest = digest(151);
                }
                2 => {
                    changed_nifty.calendar_policy_digest = digest(152);
                    bank.calendar_policy_digest = digest(152);
                }
                3 => {
                    changed_nifty.daily_reference_policy_digest = digest(153);
                    bank.daily_reference_policy_digest = digest(153);
                }
                _ => {
                    let span =
                        RequestedSpanIdentityV1::new(2024, 1, 2024, 3).expect("changed span valid");
                    changed_nifty.requested_span = span;
                    bank.requested_span = span;
                }
            }
            let candidates = [
                raw_candidate(InstrumentFamilyV1::Nifty, 28, [10, 20, -5, 15]),
                raw_candidate(InstrumentFamilyV1::BankNifty, 29, [-5, 10, 20, -10]),
            ];
            let splits = [RawSplitV2 {
                train_mask: 2,
                test_mask: 1,
                train_scores: vec![10, 5],
                test_scores: vec![0, 10],
            }];
            let changed = PreparedPopulationStatisticsV2::new(
                &changed_nifty,
                &bank,
                &candidates,
                &splits,
                StatisticsProcedureInputsV2 {
                    segment_count: 2,
                    draws: 16,
                    seed: 77,
                    block_length: 2,
                },
            )
            .expect("matched changed pair remains internally valid");
            assert_ne!(changed.manifest.audit_id, base_id);
        }
    }
}
