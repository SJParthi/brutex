//! Fixed-stride proof of the global Top-25 for one signal timeframe.
//!
//! One authoritative list spans exactly two span-bound V3 populations,
//! in canonical `NSE-NIFTY` then `NSE-BANKNIFTY` order.  Construction streams
//! those durable rows twice through [`runner::topn`] and resolves the retained
//! rows in one further bounded pass.  It never accepts caller-authored counts,
//! population proofs, scores or winners.
//!
//! The ledger is append-only.  Each fixed-size record contains the two source
//! population identities/counts/digests, the exact combined ordered-population
//! proof, ranking-policy identity and up to 25 strongest-first row references.
//! Top-10 is only a slice of that persisted Top-25; it is never recalculated.
//!
//! # Cost
//!
//! Receipt construction is O(NIFTY rows + BANKNIFTY rows), as a global Top-N
//! must inspect every admitted candidate.  RAM is bounded by the two-pass
//! extrema, 25 retained candidates and one population page.  Ledger open is
//! O(selection receipts); lookup after open is one hash probe on average.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt as _;

use costs::fill::Direction;
use runner::portfolio::StrategyDigest;
use runner::topn::{
    Candidate, Extrema, MAX_TOP, Metrics, PopulationPass, PopulationProof, RankingPolicyV1,
    SCORE_SCALE, Selection, VerifiedKeeper,
};

use crate::population::{
    AdmissionStatusV1, CompletionReceiptV2, CompletionReceiptV3, CompletionReceiptV4,
    InstrumentFamilyV1, MAX_PAGE_ROWS_V1, PopulationIdentitiesV2, PopulationLedger,
    PopulationRowV1, TopMetricsV1, TradeDirectionV1,
};

/// Operator-facing refusal from selection derivation or persistence.
pub type SelectionRefusal = String;

const MAGIC: [u8; 8] = *b"BRUTXSL1";
const VERSION: u32 = 1;
const MAGIC_V2: [u8; 8] = *b"BRUTXSL2";
const VERSION_V2: u32 = 2;
const HEADER: u64 = 40;
const HEADER_BYTES: usize = 40;
const HEADER_BYTES_U32: u32 = 40;
const SEAL_BYTES: usize = 32;
const SELECTED_ENTRY_BYTES: usize = 88;
const PAYLOAD_BYTES: usize = 2_888;
const PAYLOAD_BYTES_V2: usize = 3_016;

/// Bytes in the canonical shared-cohort identity carried by every receipt.
pub const SHARED_COHORT_CANONICAL_LEN_V1: usize = 312;
const SHARED_COHORT_DOMAIN_V1: [u8; 16] = *b"brutex-cohort-v1";
const SHARED_COHORT_VERSION_V1: u32 = 1;
const SHARED_COHORT_PAYLOAD_LEN_V1: u32 = 288;
/// Bytes in one canonical V2 cohort identity.
pub const SHARED_COHORT_CANONICAL_LEN_V2: usize = 440;
/// Bytes in one canonical selected-entry record reused by later append-only
/// selection receipts.
pub const SELECTED_ENTRY_CANONICAL_LEN_V1: usize = SELECTED_ENTRY_BYTES;
const SHARED_COHORT_DOMAIN_V2: [u8; 16] = *b"brutex-cohort-v2";
const SHARED_COHORT_VERSION_V2: u32 = 2;
const SHARED_COHORT_PAYLOAD_LEN_V2: u32 = 416;

/// Bytes in one canonical version-one selection receipt.
pub const SELECTION_STRIDE_BYTES: usize = PAYLOAD_BYTES + SEAL_BYTES;
/// Fixed on-disk stride of one version-one selection receipt.
pub const SELECTION_STRIDE: u64 = 2_920;
const SELECTION_STRIDE_BYTES_U32: u32 = 2_920;
/// Bytes in one authoritative version-two selection receipt.
pub const SELECTION_V2_STRIDE_BYTES: usize = PAYLOAD_BYTES_V2 + SEAL_BYTES;
/// Fixed on-disk stride of one authoritative version-two selection receipt.
pub const SELECTION_V2_STRIDE: u64 = 3_048;
const SELECTION_V2_STRIDE_BYTES_U32: u32 = 3_048;
/// The one authoritative request size.  Top-10 is its prefix.
pub const REQUESTED_TOP_V1: u32 = 25;

const _: () = assert!(MAX_TOP == 25);
const _: () = assert!(HEADER_BYTES == 40);
const _: () = assert!(HEADER_BYTES_U32 == 40);
const _: () = assert!(HEADER == 40);
const _: () = assert!(SELECTION_STRIDE_BYTES == 2_920);
const _: () = assert!(SELECTION_STRIDE_BYTES_U32 == 2_920);
const _: () = assert!(SELECTION_STRIDE == 2_920);
const _: () = assert!(PAYLOAD_BYTES + SEAL_BYTES == SELECTION_STRIDE_BYTES);
const _: () = assert!(SHARED_COHORT_CANONICAL_LEN_V2 == 24 + 13 * 32);
const _: () = assert!(PAYLOAD_BYTES_V2 + SEAL_BYTES == SELECTION_V2_STRIDE_BYTES);
const _: () = assert!(SELECTION_V2_STRIDE_BYTES == 3_048);
const _: () = assert!(SELECTION_V2_STRIDE == 3_048);

/// Exact cross-instrument compatibility identity for one global timeframe.
///
/// Instrument-specific run/data/exit-grid identities deliberately do not
/// appear: they must differ between NIFTY and BANKNIFTY.  Every shared term
/// that V2 currently persists does appear, plus the independently persisted
/// requested-span identity required to prove both populations cover the same
/// sessions.  V2 completions do not persist that final term, so public
/// construction accepts only authoritative span-bound V3 completions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(
    clippy::struct_field_names,
    reason = "every field is explicitly named as a digest in this fixed-layout identity record"
)]
pub struct SharedCohortIdentityV1 {
    requested_span_digest: [u8; 32],
    feed_digest: [u8; 32],
    source_commit_digest: [u8; 32],
    vocabulary_digest: [u8; 32],
    evaluation_policy_digest: [u8; 32],
    admission_policy_digest: [u8; 32],
    ranking_policy_digest: [u8; 32],
    calendar_policy_digest: [u8; 32],
    daily_reference_policy_digest: [u8; 32],
}

impl SharedCohortIdentityV1 {
    fn from_v2_terms(
        nifty: &PopulationIdentitiesV2,
        bank_nifty: &PopulationIdentitiesV2,
        requested_span_digest: [u8; 32],
    ) -> Result<Self, SelectionRefusal> {
        ensure_matching_v2_shared_terms(nifty, bank_nifty)?;
        let identity = Self {
            requested_span_digest,
            feed_digest: nifty.feed_digest,
            source_commit_digest: nifty.source_commit_digest,
            vocabulary_digest: nifty.vocabulary_digest,
            evaluation_policy_digest: nifty.evaluation_policy_digest,
            admission_policy_digest: nifty.admission_policy_digest,
            ranking_policy_digest: nifty.ranking_policy_digest,
            calendar_policy_digest: nifty.calendar_policy_digest,
            daily_reference_policy_digest: nifty.daily_reference_policy_digest,
        };
        identity.validate()?;
        Ok(identity)
    }

    fn validate(self) -> Result<(), SelectionRefusal> {
        for (name, digest) in [
            ("requested-span", self.requested_span_digest),
            ("feed", self.feed_digest),
            ("source-commit", self.source_commit_digest),
            ("vocabulary", self.vocabulary_digest),
            ("evaluation-policy", self.evaluation_policy_digest),
            ("admission-policy", self.admission_policy_digest),
            ("ranking-policy", self.ranking_policy_digest),
            ("calendar-policy", self.calendar_policy_digest),
            ("daily-reference-policy", self.daily_reference_policy_digest),
        ] {
            require_digest(&format!("shared cohort {name}"), &digest)?;
        }
        Ok(())
    }

    /// Canonical, versioned fixed-size cohort bytes.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; SHARED_COHORT_CANONICAL_LEN_V1] {
        let mut bytes = [0_u8; SHARED_COHORT_CANONICAL_LEN_V1];
        if let Some(slot) = bytes.get_mut(..16) {
            slot.copy_from_slice(&SHARED_COHORT_DOMAIN_V1);
        }
        if let Some(slot) = bytes.get_mut(16..20) {
            slot.copy_from_slice(&SHARED_COHORT_VERSION_V1.to_le_bytes());
        }
        if let Some(slot) = bytes.get_mut(20..24) {
            slot.copy_from_slice(&SHARED_COHORT_PAYLOAD_LEN_V1.to_le_bytes());
        }
        let digests = [
            self.requested_span_digest,
            self.feed_digest,
            self.source_commit_digest,
            self.vocabulary_digest,
            self.evaluation_policy_digest,
            self.admission_policy_digest,
            self.ranking_policy_digest,
            self.calendar_policy_digest,
            self.daily_reference_policy_digest,
        ];
        for (index, digest) in digests.into_iter().enumerate() {
            let start = 24_usize.saturating_add(index.saturating_mul(32));
            let end = start.saturating_add(32);
            if let Some(slot) = bytes.get_mut(start..end) {
                slot.copy_from_slice(&digest);
            }
        }
        bytes
    }

    /// Domain-separated digest of the complete canonical cohort record.
    #[must_use]
    pub fn digest(self) -> [u8; 32] {
        brutex_core::blake3::hash(&self.canonical_bytes())
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, SelectionRefusal> {
        let domain = decoder.take(16)?;
        if domain != SHARED_COHORT_DOMAIN_V1 {
            return Err("selection shared-cohort domain is unknown".to_owned());
        }
        if decoder.u32()? != SHARED_COHORT_VERSION_V1 {
            return Err("selection shared-cohort version is unknown".to_owned());
        }
        if decoder.u32()? != SHARED_COHORT_PAYLOAD_LEN_V1 {
            return Err("selection shared-cohort payload length is noncanonical".to_owned());
        }
        let identity = Self {
            requested_span_digest: decoder.array_32()?,
            feed_digest: decoder.array_32()?,
            source_commit_digest: decoder.array_32()?,
            vocabulary_digest: decoder.array_32()?,
            evaluation_policy_digest: decoder.array_32()?,
            admission_policy_digest: decoder.array_32()?,
            ranking_policy_digest: decoder.array_32()?,
            calendar_policy_digest: decoder.array_32()?,
            daily_reference_policy_digest: decoder.array_32()?,
        };
        identity.validate()?;
        Ok(identity)
    }

    /// Independently persisted requested-span identity.
    #[must_use]
    pub const fn requested_span_digest(self) -> [u8; 32] {
        self.requested_span_digest
    }
}

/// Exact V2 cohort identity for one global, calendar-complete timeframe.
///
/// V1 remains audit-readable, but omitted same-side exit-policy equality and
/// actual signal/execution coverage. V2 adds those four load-bearing digests;
/// instrument-specific resolved grids remain intentionally absent because
/// NIFTY and BANKNIFTY must resolve different training bytes and levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(
    clippy::struct_field_names,
    reason = "every field is explicitly named as a digest in this fixed-layout identity record"
)]
pub struct SharedCohortIdentityV2 {
    requested_span_digest: [u8; 32],
    feed_digest: [u8; 32],
    source_commit_digest: [u8; 32],
    vocabulary_digest: [u8; 32],
    evaluation_policy_digest: [u8; 32],
    admission_policy_digest: [u8; 32],
    ranking_policy_digest: [u8; 32],
    calendar_policy_digest: [u8; 32],
    daily_reference_policy_digest: [u8; 32],
    long_exit_policy_digest: [u8; 32],
    short_exit_policy_digest: [u8; 32],
    signal_coverage_digest: [u8; 32],
    execution_coverage_digest: [u8; 32],
}

impl SharedCohortIdentityV2 {
    pub(crate) fn from_v4_receipts(
        nifty: &CompletionReceiptV4,
        bank_nifty: &CompletionReceiptV4,
    ) -> Result<Self, SelectionRefusal> {
        let nifty_v3 = nifty.v3();
        let bank_v3 = bank_nifty.v3();
        let requested_span = nifty_v3
            .requested_span()
            .require_same(bank_v3.requested_span())?;
        let nifty_identities = nifty_v3.v2().identities;
        let bank_identities = bank_v3.v2().identities;
        ensure_matching_v2_shared_terms(&nifty_identities, &bank_identities)?;
        if nifty_identities.exit_grids.long.policy_digest
            != bank_identities.exit_grids.long.policy_digest
        {
            return Err(
                "NIFTY and BANKNIFTY populations have different long exit-policy identities"
                    .to_owned(),
            );
        }
        if nifty_identities.exit_grids.short.policy_digest
            != bank_identities.exit_grids.short.policy_digest
        {
            return Err(
                "NIFTY and BANKNIFTY populations have different short exit-policy identities"
                    .to_owned(),
            );
        }
        let nifty_coverage = nifty.coverage();
        let bank_coverage = bank_nifty.coverage();
        if nifty_coverage.signal_complete_receipt_digest()
            != bank_coverage.signal_complete_receipt_digest()
        {
            return Err(
                "NIFTY and BANKNIFTY populations have different complete signal-calendar coverage identities"
                    .to_owned(),
            );
        }
        if nifty_coverage.execution_complete_receipt_digest()
            != bank_coverage.execution_complete_receipt_digest()
        {
            return Err(
                "NIFTY and BANKNIFTY populations have different complete one-minute execution-calendar coverage identities"
                    .to_owned(),
            );
        }
        let identity = Self {
            requested_span_digest: requested_span.digest(),
            feed_digest: nifty_identities.feed_digest,
            source_commit_digest: nifty_identities.source_commit_digest,
            vocabulary_digest: nifty_identities.vocabulary_digest,
            evaluation_policy_digest: nifty_identities.evaluation_policy_digest,
            admission_policy_digest: nifty_identities.admission_policy_digest,
            ranking_policy_digest: nifty_identities.ranking_policy_digest,
            calendar_policy_digest: nifty_identities.calendar_policy_digest,
            daily_reference_policy_digest: nifty_identities.daily_reference_policy_digest,
            long_exit_policy_digest: nifty_identities.exit_grids.long.policy_digest,
            short_exit_policy_digest: nifty_identities.exit_grids.short.policy_digest,
            signal_coverage_digest: nifty_coverage.signal_complete_receipt_digest(),
            execution_coverage_digest: nifty_coverage.execution_complete_receipt_digest(),
        };
        identity.validate()?;
        Ok(identity)
    }

    pub(crate) fn validate(self) -> Result<(), SelectionRefusal> {
        for (name, digest) in [
            ("requested-span", self.requested_span_digest),
            ("feed", self.feed_digest),
            ("source-commit", self.source_commit_digest),
            ("vocabulary", self.vocabulary_digest),
            ("evaluation-policy", self.evaluation_policy_digest),
            ("admission-policy", self.admission_policy_digest),
            ("ranking-policy", self.ranking_policy_digest),
            ("calendar-policy", self.calendar_policy_digest),
            ("daily-reference-policy", self.daily_reference_policy_digest),
            ("long-exit-policy", self.long_exit_policy_digest),
            ("short-exit-policy", self.short_exit_policy_digest),
            ("signal-calendar-coverage", self.signal_coverage_digest),
            (
                "one-minute-execution-calendar-coverage",
                self.execution_coverage_digest,
            ),
        ] {
            require_digest(&format!("shared V2 cohort {name}"), &digest)?;
        }
        Ok(())
    }

    /// Canonical fixed-size V2 cohort bytes.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; SHARED_COHORT_CANONICAL_LEN_V2] {
        let mut bytes = [0_u8; SHARED_COHORT_CANONICAL_LEN_V2];
        if let Some(slot) = bytes.get_mut(..16) {
            slot.copy_from_slice(&SHARED_COHORT_DOMAIN_V2);
        }
        if let Some(slot) = bytes.get_mut(16..20) {
            slot.copy_from_slice(&SHARED_COHORT_VERSION_V2.to_le_bytes());
        }
        if let Some(slot) = bytes.get_mut(20..24) {
            slot.copy_from_slice(&SHARED_COHORT_PAYLOAD_LEN_V2.to_le_bytes());
        }
        let digests = [
            self.requested_span_digest,
            self.feed_digest,
            self.source_commit_digest,
            self.vocabulary_digest,
            self.evaluation_policy_digest,
            self.admission_policy_digest,
            self.ranking_policy_digest,
            self.calendar_policy_digest,
            self.daily_reference_policy_digest,
            self.long_exit_policy_digest,
            self.short_exit_policy_digest,
            self.signal_coverage_digest,
            self.execution_coverage_digest,
        ];
        for (index, digest) in digests.into_iter().enumerate() {
            let start = 24_usize.saturating_add(index.saturating_mul(32));
            let end = start.saturating_add(32);
            if let Some(slot) = bytes.get_mut(start..end) {
                slot.copy_from_slice(&digest);
            }
        }
        bytes
    }

    /// Domain-separated digest of the complete V2 cohort record.
    #[must_use]
    pub fn digest(self) -> [u8; 32] {
        brutex_core::blake3::hash(&self.canonical_bytes())
    }

    /// Decodes one exact canonical V2 cohort without exposing the private
    /// selection-ledger decoder to append-only successor formats.
    ///
    /// # Errors
    ///
    /// Refuses wrong length/domain/version, trailing bytes, zero identities or
    /// any other invalid V2 cohort field.
    pub(crate) fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, SelectionRefusal> {
        if bytes.len() != SHARED_COHORT_CANONICAL_LEN_V2 {
            return Err(format!(
                "selection V2 cohort is {} bytes, not {SHARED_COHORT_CANONICAL_LEN_V2}",
                bytes.len()
            ));
        }
        let mut decoder = Decoder::new(bytes);
        let identity = Self::decode(&mut decoder)?;
        decoder.finish()?;
        Ok(identity)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, SelectionRefusal> {
        if decoder.take(16)? != SHARED_COHORT_DOMAIN_V2 {
            return Err("selection V2 shared-cohort domain is unknown".to_owned());
        }
        if decoder.u32()? != SHARED_COHORT_VERSION_V2 {
            return Err("selection V2 shared-cohort version is unknown".to_owned());
        }
        if decoder.u32()? != SHARED_COHORT_PAYLOAD_LEN_V2 {
            return Err("selection V2 shared-cohort payload length is noncanonical".to_owned());
        }
        let identity = Self {
            requested_span_digest: decoder.array_32()?,
            feed_digest: decoder.array_32()?,
            source_commit_digest: decoder.array_32()?,
            vocabulary_digest: decoder.array_32()?,
            evaluation_policy_digest: decoder.array_32()?,
            admission_policy_digest: decoder.array_32()?,
            ranking_policy_digest: decoder.array_32()?,
            calendar_policy_digest: decoder.array_32()?,
            daily_reference_policy_digest: decoder.array_32()?,
            long_exit_policy_digest: decoder.array_32()?,
            short_exit_policy_digest: decoder.array_32()?,
            signal_coverage_digest: decoder.array_32()?,
            execution_coverage_digest: decoder.array_32()?,
        };
        identity.validate()?;
        Ok(identity)
    }

    /// Requested inclusive-month span identity.
    #[must_use]
    pub const fn requested_span_digest(self) -> [u8; 32] {
        self.requested_span_digest
    }

    /// Same-side long exit-policy identity shared across both indices.
    #[must_use]
    pub const fn long_exit_policy_digest(self) -> [u8; 32] {
        self.long_exit_policy_digest
    }

    /// Same-side short exit-policy identity shared across both indices.
    #[must_use]
    pub const fn short_exit_policy_digest(self) -> [u8; 32] {
        self.short_exit_policy_digest
    }

    /// Exact complete signal-rung calendar coverage identity.
    #[must_use]
    pub const fn signal_coverage_digest(self) -> [u8; 32] {
        self.signal_coverage_digest
    }

    /// Exact complete one-minute execution calendar coverage identity.
    #[must_use]
    pub const fn execution_coverage_digest(self) -> [u8; 32] {
        self.execution_coverage_digest
    }

    /// Final-ranking policy shared by both authoritative populations.
    #[must_use]
    pub const fn ranking_policy_digest(self) -> [u8; 32] {
        self.ranking_policy_digest
    }
}

/// One canonical committed-population reference inside a global selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationReferenceV1 {
    family: InstrumentFamilyV1,
    population_id: [u8; 32],
    row_count: u64,
    ordered_row_digest: [u8; 32],
    completion_digest: [u8; 32],
}

impl PopulationReferenceV1 {
    fn from_receipt(
        receipt: &CompletionReceiptV3,
        expected_family: InstrumentFamilyV1,
    ) -> Result<Self, SelectionRefusal> {
        let v2 = receipt.v2();
        if v2.instrument_family != expected_family {
            return Err(format!(
                "selection expected {expected_family:?}, but population {:?} is {:#?}",
                v2.population_id, v2.instrument_family
            ));
        }
        let reference = Self {
            family: v2.instrument_family,
            population_id: v2.population_id,
            row_count: v2.row_count,
            ordered_row_digest: v2.ordered_row_digest,
            completion_digest: receipt.content_digest()?,
        };
        reference.validate(expected_family)?;
        Ok(reference)
    }

    fn validate(self, expected_family: InstrumentFamilyV1) -> Result<(), SelectionRefusal> {
        if self.family != expected_family {
            return Err(format!(
                "selection population order is noncanonical: expected {expected_family:?}, found {:?}",
                self.family
            ));
        }
        require_digest("population_id", &self.population_id)?;
        require_digest("ordered_row_digest", &self.ordered_row_digest)?;
        require_digest("V3 completion_digest", &self.completion_digest)
    }

    /// Canonical instrument family.
    #[must_use]
    pub const fn family(self) -> InstrumentFamilyV1 {
        self.family
    }

    /// Authoritative span-bound V3 population identity.
    #[must_use]
    pub const fn population_id(self) -> [u8; 32] {
        self.population_id
    }

    /// Exact committed population row count.
    #[must_use]
    pub const fn row_count(self) -> u64 {
        self.row_count
    }

    /// Digest of every committed row payload in exact sequence order.
    #[must_use]
    pub const fn ordered_row_digest(self) -> [u8; 32] {
        self.ordered_row_digest
    }

    /// Digest of every authoritative V3 completion fact, including span.
    #[must_use]
    pub const fn completion_digest(self) -> [u8; 32] {
        self.completion_digest
    }

    fn encode(self, encoder: &mut Encoder<'_>) -> Result<(), SelectionRefusal> {
        encoder.u8(family_byte(self.family))?;
        encoder.zeros(7)?;
        encoder.bytes(&self.population_id)?;
        encoder.u64(self.row_count)?;
        encoder.bytes(&self.ordered_row_digest)?;
        encoder.bytes(&self.completion_digest)
    }

    fn decode(
        decoder: &mut Decoder<'_>,
        expected_family: InstrumentFamilyV1,
    ) -> Result<Self, SelectionRefusal> {
        let family = decode_family(decoder.u8()?)?;
        decoder.zeros(7, "population-reference reserve")?;
        let reference = Self {
            family,
            population_id: decoder.array_32()?,
            row_count: decoder.u64()?,
            ordered_row_digest: decoder.array_32()?,
            completion_digest: decoder.array_32()?,
        };
        reference.validate(expected_family)?;
        Ok(reference)
    }
}

/// One calendar-authoritative V4 population reference in a V2 selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationReferenceV2 {
    family: InstrumentFamilyV1,
    population_id: [u8; 32],
    row_count: u64,
    ordered_row_digest: [u8; 32],
    completion_digest: [u8; 32],
}

impl PopulationReferenceV2 {
    fn from_receipt(
        receipt: &CompletionReceiptV4,
        expected_family: InstrumentFamilyV1,
    ) -> Result<Self, SelectionRefusal> {
        let v2 = receipt.v3().v2();
        if v2.instrument_family != expected_family {
            return Err(format!(
                "selection V2 expected {expected_family:?}, but population {:?} is {:#?}",
                v2.population_id, v2.instrument_family
            ));
        }
        let reference = Self {
            family: v2.instrument_family,
            population_id: v2.population_id,
            row_count: v2.row_count,
            ordered_row_digest: v2.ordered_row_digest,
            completion_digest: receipt.content_digest()?,
        };
        reference.validate(expected_family)?;
        Ok(reference)
    }

    fn validate(self, expected_family: InstrumentFamilyV1) -> Result<(), SelectionRefusal> {
        if self.family != expected_family {
            return Err(format!(
                "selection V2 population order is noncanonical: expected {expected_family:?}, found {:?}",
                self.family
            ));
        }
        require_digest("V2 population_id", &self.population_id)?;
        require_digest("V2 ordered_row_digest", &self.ordered_row_digest)?;
        require_digest("V4 completion_digest", &self.completion_digest)
    }

    /// Canonical instrument family.
    #[must_use]
    pub const fn family(self) -> InstrumentFamilyV1 {
        self.family
    }

    /// Authoritative calendar-bound V4 population identity.
    #[must_use]
    pub const fn population_id(self) -> [u8; 32] {
        self.population_id
    }

    /// Exact committed row count.
    #[must_use]
    pub const fn row_count(self) -> u64 {
        self.row_count
    }

    /// Digest of all source row payloads in sequence order.
    #[must_use]
    pub const fn ordered_row_digest(self) -> [u8; 32] {
        self.ordered_row_digest
    }

    /// Digest of the complete V4 population receipt.
    #[must_use]
    pub const fn completion_digest(self) -> [u8; 32] {
        self.completion_digest
    }

    fn encode(self, encoder: &mut Encoder<'_>) -> Result<(), SelectionRefusal> {
        encoder.u8(family_byte(self.family))?;
        encoder.zeros(7)?;
        encoder.bytes(&self.population_id)?;
        encoder.u64(self.row_count)?;
        encoder.bytes(&self.ordered_row_digest)?;
        encoder.bytes(&self.completion_digest)
    }

    fn decode(
        decoder: &mut Decoder<'_>,
        expected_family: InstrumentFamilyV1,
    ) -> Result<Self, SelectionRefusal> {
        let family = decode_family(decoder.u8()?)?;
        decoder.zeros(7, "V2 population-reference reserve")?;
        let reference = Self {
            family,
            population_id: decoder.array_32()?,
            row_count: decoder.u64()?,
            ordered_row_digest: decoder.array_32()?,
            completion_digest: decoder.array_32()?,
        };
        reference.validate(expected_family)?;
        Ok(reference)
    }
}

/// One persisted member of the strongest-first global Top-25.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectedEntryV1 {
    family: InstrumentFamilyV1,
    population_id: [u8; 32],
    row_sequence: u64,
    strategy_digest: [u8; 32],
    score: u64,
}

impl SelectedEntryV1 {
    pub(crate) fn new(
        family: InstrumentFamilyV1,
        population_id: [u8; 32],
        row_sequence: u64,
        strategy_digest: [u8; 32],
        score: u64,
    ) -> Self {
        Self {
            family,
            population_id,
            row_sequence,
            strategy_digest,
            score,
        }
    }

    /// Source population family.
    #[must_use]
    pub const fn family(self) -> InstrumentFamilyV1 {
        self.family
    }

    /// Exact source population identity.
    #[must_use]
    pub const fn population_id(self) -> [u8; 32] {
        self.population_id
    }

    /// Canonical zero-based row sequence inside that population.
    #[must_use]
    pub const fn row_sequence(self) -> u64 {
        self.row_sequence
    }

    /// Full semantic strategy identity copied from the source row.
    #[must_use]
    pub const fn strategy_digest(self) -> [u8; 32] {
        self.strategy_digest
    }

    /// Authoritative fixed-point ranking score.
    #[must_use]
    pub const fn score(self) -> u64 {
        self.score
    }

    /// Exact canonical selected-entry bytes for append-only successor formats.
    ///
    /// # Errors
    ///
    /// Refuses only an internal fixed-layout encoding mismatch.
    pub(crate) fn canonical_bytes(
        self,
    ) -> Result<[u8; SELECTED_ENTRY_CANONICAL_LEN_V1], SelectionRefusal> {
        let mut raw = [0_u8; SELECTED_ENTRY_CANONICAL_LEN_V1];
        let mut encoder = Encoder::new(&mut raw);
        self.encode(&mut encoder)?;
        encoder.finish()?;
        Ok(raw)
    }

    /// Decodes one exact canonical selected-entry record.
    ///
    /// # Errors
    ///
    /// Refuses wrong length, an unknown family, nonzero reserve or trailing
    /// bytes. Population-reference semantics are checked by the owning receipt.
    pub(crate) fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, SelectionRefusal> {
        if bytes.len() != SELECTED_ENTRY_CANONICAL_LEN_V1 {
            return Err(format!(
                "selected entry is {} bytes, not {SELECTED_ENTRY_CANONICAL_LEN_V1}",
                bytes.len()
            ));
        }
        let mut decoder = Decoder::new(bytes);
        let entry = Self::decode(&mut decoder)?;
        decoder.finish()?;
        Ok(entry)
    }

    fn validate_against(
        self,
        nifty: PopulationReferenceV1,
        bank_nifty: PopulationReferenceV1,
    ) -> Result<(), SelectionRefusal> {
        require_digest("selected population_id", &self.population_id)?;
        require_digest("selected strategy_digest", &self.strategy_digest)?;
        if self.score > SCORE_SCALE {
            return Err(format!(
                "selected score {} exceeds the fixed-point domain 0..={SCORE_SCALE}",
                self.score
            ));
        }
        let source = match self.family {
            InstrumentFamilyV1::Nifty => nifty,
            InstrumentFamilyV1::BankNifty => bank_nifty,
        };
        if self.population_id != source.population_id {
            return Err(format!(
                "selected {:?} row names a population other than its canonical source",
                self.family
            ));
        }
        if self.row_sequence >= source.row_count {
            return Err(format!(
                "selected {:?} row sequence {} is outside its {}-row population",
                self.family, self.row_sequence, source.row_count
            ));
        }
        Ok(())
    }

    fn validate_against_v2(
        self,
        nifty: PopulationReferenceV2,
        bank_nifty: PopulationReferenceV2,
    ) -> Result<(), SelectionRefusal> {
        require_digest("selected V2 population_id", &self.population_id)?;
        require_digest("selected V2 strategy_digest", &self.strategy_digest)?;
        if self.score > SCORE_SCALE {
            return Err(format!(
                "selected V2 score {} exceeds the fixed-point domain 0..={SCORE_SCALE}",
                self.score
            ));
        }
        let source = match self.family {
            InstrumentFamilyV1::Nifty => nifty,
            InstrumentFamilyV1::BankNifty => bank_nifty,
        };
        if self.population_id != source.population_id {
            return Err(format!(
                "selected V2 {:?} row names a population other than its canonical source",
                self.family
            ));
        }
        if self.row_sequence >= source.row_count {
            return Err(format!(
                "selected V2 {:?} row sequence {} is outside its {}-row population",
                self.family, self.row_sequence, source.row_count
            ));
        }
        Ok(())
    }

    fn encode(self, encoder: &mut Encoder<'_>) -> Result<(), SelectionRefusal> {
        encoder.u8(family_byte(self.family))?;
        encoder.zeros(7)?;
        encoder.bytes(&self.population_id)?;
        encoder.u64(self.row_sequence)?;
        encoder.bytes(&self.strategy_digest)?;
        encoder.u64(self.score)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, SelectionRefusal> {
        let family = decode_family(decoder.u8()?)?;
        decoder.zeros(7, "selected-entry reserve")?;
        Ok(Self {
            family,
            population_id: decoder.array_32()?,
            row_sequence: decoder.u64()?,
            strategy_digest: decoder.array_32()?,
            score: decoder.u64()?,
        })
    }
}

/// Durable global-per-timeframe Top-25 receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionReceiptV1 {
    selection_id: [u8; 32],
    rung_seconds: u32,
    populations: [PopulationReferenceV1; 2],
    proof: PopulationProof,
    ranking_policy_digest: [u8; 32],
    cohort: SharedCohortIdentityV1,
    selected: Vec<SelectedEntryV1>,
}

impl SelectionReceiptV1 {
    /// Derives one receipt from two authoritative span-bound V3 populations.
    ///
    /// Rows are streamed in canonical NIFTY-then-BANKNIFTY order.  Both
    /// ranking passes and every winner are recomputed here; callers cannot
    /// inject a proof, reconciliation count, score or selected entry.
    ///
    /// # Errors
    ///
    /// Refuses absent/foreign populations, reversed families, different
    /// timeframes or shared cohort terms, malformed pages, any two-pass
    /// mismatch, duplicate winner identity, missing source row or invalid
    /// receipt field. A legacy V2-only population fails closed because it has
    /// no requested-span identity.
    pub fn from_committed_populations(
        populations: &mut PopulationLedger,
        nifty_population_id: [u8; 32],
        bank_nifty_population_id: [u8; 32],
        policy: RankingPolicyV1,
    ) -> Result<Self, SelectionRefusal> {
        Self::derive_from_committed_populations(
            populations,
            nifty_population_id,
            bank_nifty_population_id,
            policy,
        )
    }

    fn derive_from_committed_populations(
        populations: &mut PopulationLedger,
        nifty_population_id: [u8; 32],
        bank_nifty_population_id: [u8; 32],
        policy: RankingPolicyV1,
    ) -> Result<Self, SelectionRefusal> {
        if nifty_population_id == bank_nifty_population_id {
            return Err("NIFTY and BANKNIFTY populations cannot share one identity".to_owned());
        }
        let nifty_v3 = populations.receipt_v3(&nifty_population_id).ok_or_else(|| {
            format!(
                "NIFTY population {} is not an authoritative requested-span-bound V3 population",
                hex(&nifty_population_id)
            )
        })?;
        let bank_v3 = populations.receipt_v3(&bank_nifty_population_id).ok_or_else(|| {
                format!(
                    "BANKNIFTY population {} is not an authoritative requested-span-bound V3 population",
                    hex(&bank_nifty_population_id)
                )
            })?;
        let requested_span = nifty_v3
            .requested_span()
            .require_same(bank_v3.requested_span())?;
        let nifty_receipt = nifty_v3.v2();
        let bank_receipt = bank_v3.v2();
        if nifty_receipt.rung_seconds != bank_receipt.rung_seconds {
            return Err(format!(
                "global selection requires one timeframe, but NIFTY is {}s and BANKNIFTY is {}s",
                nifty_receipt.rung_seconds, bank_receipt.rung_seconds
            ));
        }
        let ranking_policy_digest = policy.digest();
        for (name, receipt) in [("NIFTY", nifty_receipt), ("BANKNIFTY", bank_receipt)] {
            if receipt.identities.ranking_policy_digest != ranking_policy_digest {
                return Err(format!(
                    "{name} population ranking-policy identity does not match the requested canonical policy"
                ));
            }
        }
        ensure_matching_v2_shared_terms(&nifty_receipt.identities, &bank_receipt.identities)?;
        let cohort = SharedCohortIdentityV1::from_v2_terms(
            &nifty_receipt.identities,
            &bank_receipt.identities,
            requested_span.digest(),
        )?;

        let references = [
            PopulationReferenceV1::from_receipt(&nifty_v3, InstrumentFamilyV1::Nifty)?,
            PopulationReferenceV1::from_receipt(&bank_v3, InstrumentFamilyV1::BankNifty)?,
        ];

        let mut extrema = Extrema::default();
        let mut first_pass = PopulationPass::new();
        visit_both(populations, references, |_, _row, candidate| {
            extrema.observe(&candidate);
            first_pass.observe(&candidate);
            Ok(())
        })?;
        let proof = first_pass.finish();
        validate_proof_against_receipts(proof, &nifty_receipt, &bank_receipt)?;

        let mut keeper = VerifiedKeeper::new(MAX_TOP, policy, extrema, proof)
            .map_err(|why| format!("global Top-25 could not start: {why:?}"))?;
        visit_both(populations, references, |_, _row, candidate| {
            keeper
                .offer(candidate)
                .map_err(|why| format!("global Top-25 candidate was refused: {why:?}"))
        })?;
        let selection = keeper
            .finish()
            .map_err(|why| format!("global Top-25 did not reproduce its first pass: {why:?}"))?;
        validate_selection_reconciliation(&selection, proof)?;
        let selected = resolve_selected(populations, references, &selection)?;

        let mut receipt = Self {
            selection_id: [0; 32],
            rung_seconds: nifty_receipt.rung_seconds,
            populations: references,
            proof,
            ranking_policy_digest,
            cohort,
            selected,
        };
        receipt.selection_id = receipt.derived_id()?;
        receipt.validate_semantics()?;
        Ok(receipt)
    }

    /// Recomputes this receipt from its committed populations and compares all
    /// bytes, including proof, winner order and score.
    ///
    /// # Errors
    ///
    /// Returns any construction refusal or a mismatch with the recomputed
    /// canonical receipt. Legacy V2-only receipts fail closed.
    pub fn verify_against_populations(
        &self,
        populations: &mut PopulationLedger,
        policy: RankingPolicyV1,
    ) -> Result<(), SelectionRefusal> {
        let [nifty, bank_nifty] = self.populations;
        let rebuilt = Self::from_committed_populations(
            populations,
            nifty.population_id,
            bank_nifty.population_id,
            policy,
        )?;
        if rebuilt != *self {
            return Err(
                "selection receipt differs from a full replay of its committed populations"
                    .to_owned(),
            );
        }
        Ok(())
    }

    /// Content-derived selection identity.
    #[must_use]
    pub const fn selection_id(&self) -> [u8; 32] {
        self.selection_id
    }

    /// Signal timeframe shared by both source populations.
    #[must_use]
    pub const fn rung_seconds(&self) -> u32 {
        self.rung_seconds
    }

    /// Canonical `[NIFTY, BANKNIFTY]` source references.
    #[must_use]
    pub const fn populations(&self) -> &[PopulationReferenceV1; 2] {
        &self.populations
    }

    /// Exact combined ordered-population proof.
    #[must_use]
    pub const fn population_proof(&self) -> PopulationProof {
        self.proof
    }

    /// Canonical final-ranking policy identity.
    #[must_use]
    pub const fn ranking_policy_digest(&self) -> [u8; 32] {
        self.ranking_policy_digest
    }

    /// Complete shared cohort identity bound into this selection.
    #[must_use]
    pub const fn cohort_identity(&self) -> SharedCohortIdentityV1 {
        self.cohort
    }

    fn discovery_key(&self) -> ([u8; 32], u32) {
        (self.cohort.digest(), self.rung_seconds)
    }

    /// Strongest-first Top-25, or every admitted row when fewer than 25 exist.
    #[must_use]
    pub fn top_twenty_five(&self) -> &[SelectedEntryV1] {
        &self.selected
    }

    /// First ten entries of the exact same authoritative ordering.
    #[must_use]
    pub fn top_ten(&self) -> &[SelectedEntryV1] {
        let end = self.selected.len().min(10);
        self.selected.get(..end).unwrap_or(&[])
    }

    fn validate_semantics(&self) -> Result<(), SelectionRefusal> {
        require_digest("selection_id", &self.selection_id)?;
        require_digest("ranking_policy_digest", &self.ranking_policy_digest)?;
        if ![60_u32, 120, 180, 300, 600, 900, 1_800, 3_600].contains(&self.rung_seconds) {
            return Err(format!(
                "selection timeframe {} seconds is outside the eight canonical intraday rungs",
                self.rung_seconds
            ));
        }
        require_digest(
            "population proof ordered_digest",
            &self.proof.ordered_digest,
        )?;
        self.cohort.validate()?;
        if self.cohort.ranking_policy_digest != self.ranking_policy_digest {
            return Err(
                "selection ranking-policy digest differs from its shared cohort identity"
                    .to_owned(),
            );
        }
        let [nifty, bank_nifty] = self.populations;
        nifty.validate(InstrumentFamilyV1::Nifty)?;
        bank_nifty.validate(InstrumentFamilyV1::BankNifty)?;
        if nifty.population_id == bank_nifty.population_id {
            return Err("both canonical families reference the same population".to_owned());
        }
        let expected_considered = checked_add(
            nifty.row_count,
            bank_nifty.row_count,
            "combined population row counts",
        )?;
        if self.proof.considered != expected_considered {
            return Err(format!(
                "population proof considered {}, but source row counts total {expected_considered}",
                self.proof.considered
            ));
        }
        if checked_add(
            self.proof.admitted,
            self.proof.refused,
            "proof verdict counts",
        )? != self.proof.considered
        {
            return Err("population proof admitted + refused does not equal considered".to_owned());
        }
        if self.proof.unmeasured > self.proof.considered {
            return Err("population proof unmeasured exceeds considered".to_owned());
        }
        let expected_selected = usize::try_from(self.proof.admitted)
            .unwrap_or(usize::MAX)
            .min(MAX_TOP);
        if self.selected.len() != expected_selected {
            return Err(format!(
                "selection has {} row(s), but admitted={} requires exactly {expected_selected}",
                self.selected.len(),
                self.proof.admitted
            ));
        }
        if self.proof.admitted == 0 && !self.selected.is_empty() {
            return Err("only an admitted=0 proof may carry an empty winner list".to_owned());
        }
        if self.proof.admitted > 0 && self.selected.is_empty() {
            return Err("an admitted population cannot carry an empty winner list".to_owned());
        }

        for entry in &self.selected {
            entry.validate_against(nifty, bank_nifty)?;
        }
        for (index, left) in self.selected.iter().enumerate() {
            for right in self.selected.iter().skip(index.saturating_add(1)) {
                if left.population_id == right.population_id
                    && left.row_sequence == right.row_sequence
                {
                    return Err("selection repeats one population row".to_owned());
                }
                if left.strategy_digest == right.strategy_digest {
                    return Err("selection repeats one semantic strategy digest".to_owned());
                }
            }
        }
        if self.selected.windows(2).any(|pair| {
            pair.first()
                .zip(pair.get(1))
                .is_some_and(|(a, b)| a.score < b.score)
        }) {
            return Err("selection scores are not strongest-first".to_owned());
        }
        if self.derived_id()? != self.selection_id {
            return Err("selection identity does not match its canonical content".to_owned());
        }
        Ok(())
    }

    fn derived_id(&self) -> Result<[u8; 32], SelectionRefusal> {
        let payload = self.payload_with_id([0; 32])?;
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(b"brutex-global-selection-id-v1\0");
        hasher.update(
            payload
                .get(32..)
                .ok_or_else(|| "selection identity payload is absent".to_owned())?,
        );
        Ok(hasher.finalize())
    }

    fn payload_with_id(
        &self,
        selection_id: [u8; 32],
    ) -> Result<[u8; PAYLOAD_BYTES], SelectionRefusal> {
        let mut payload = [0_u8; PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut payload);
        encoder.bytes(&selection_id)?;
        encoder.u32(self.rung_seconds)?;
        encoder.u32(REQUESTED_TOP_V1)?;
        let [nifty, bank_nifty] = self.populations;
        nifty.encode(&mut encoder)?;
        bank_nifty.encode(&mut encoder)?;
        for count in [
            self.proof.considered,
            self.proof.admitted,
            self.proof.refused,
            self.proof.unmeasured,
        ] {
            encoder.u64(count)?;
        }
        encoder.bytes(&self.proof.ordered_digest)?;
        encoder.bytes(&self.ranking_policy_digest)?;
        encoder.bytes(&self.cohort.canonical_bytes())?;
        encoder.u32(
            u32::try_from(self.selected.len())
                .map_err(|_| "selected row count does not fit u32".to_owned())?,
        )?;
        encoder.zeros(4)?;
        for entry in &self.selected {
            entry.encode(&mut encoder)?;
        }
        let unused = MAX_TOP.saturating_sub(self.selected.len());
        encoder.zeros(unused.saturating_mul(SELECTED_ENTRY_BYTES))?;
        encoder.zeros(8)?;
        encoder.finish()?;
        Ok(payload)
    }

    fn to_bytes(&self) -> Result<[u8; SELECTION_STRIDE_BYTES], SelectionRefusal> {
        self.validate_semantics()?;
        let payload = self.payload_with_id(self.selection_id)?;
        let seal = brutex_core::blake3::hash(&payload);
        let mut raw = [0_u8; SELECTION_STRIDE_BYTES];
        raw.get_mut(..PAYLOAD_BYTES)
            .ok_or_else(|| "selection receipt payload slot is absent".to_owned())?
            .copy_from_slice(&payload);
        raw.get_mut(PAYLOAD_BYTES..)
            .ok_or_else(|| "selection receipt seal slot is absent".to_owned())?
            .copy_from_slice(&seal);
        Ok(raw)
    }

    fn from_bytes(raw: &[u8; SELECTION_STRIDE_BYTES]) -> Result<Self, SelectionRefusal> {
        let payload = raw
            .get(..PAYLOAD_BYTES)
            .ok_or_else(|| "selection receipt payload is absent".to_owned())?;
        let stored_seal = raw
            .get(PAYLOAD_BYTES..)
            .ok_or_else(|| "selection receipt seal is absent".to_owned())?;
        if stored_seal != brutex_core::blake3::hash(payload) {
            return Err("selection receipt failed its complete BLAKE3 seal".to_owned());
        }
        let mut decoder = Decoder::new(payload);
        let selection_id = decoder.array_32()?;
        let rung_seconds = decoder.u32()?;
        let requested = decoder.u32()?;
        if requested != REQUESTED_TOP_V1 {
            return Err(format!(
                "selection requests {requested}; version one requires exactly {REQUESTED_TOP_V1}"
            ));
        }
        let populations = [
            PopulationReferenceV1::decode(&mut decoder, InstrumentFamilyV1::Nifty)?,
            PopulationReferenceV1::decode(&mut decoder, InstrumentFamilyV1::BankNifty)?,
        ];
        let proof = PopulationProof {
            considered: decoder.u64()?,
            admitted: decoder.u64()?,
            refused: decoder.u64()?,
            unmeasured: decoder.u64()?,
            ordered_digest: decoder.array_32()?,
        };
        let ranking_policy_digest = decoder.array_32()?;
        let cohort = SharedCohortIdentityV1::decode(&mut decoder)?;
        let selected_count = usize::try_from(decoder.u32()?)
            .map_err(|_| "selected count does not fit this machine".to_owned())?;
        if selected_count > MAX_TOP {
            return Err(format!(
                "selection stores {selected_count} rows above the fixed maximum {MAX_TOP}"
            ));
        }
        decoder.zeros(4, "selection-count reserve")?;
        let mut selected = Vec::new();
        selected.try_reserve_exact(selected_count).map_err(|why| {
            format!("selection could not reserve {selected_count} bounded rows: {why}")
        })?;
        for _ in 0..selected_count {
            selected.push(SelectedEntryV1::decode(&mut decoder)?);
        }
        decoder.zeros(
            MAX_TOP
                .saturating_sub(selected_count)
                .saturating_mul(SELECTED_ENTRY_BYTES),
            "unused selected-entry slots",
        )?;
        decoder.zeros(8, "selection trailing reserve")?;
        decoder.finish()?;
        let receipt = Self {
            selection_id,
            rung_seconds,
            populations,
            proof,
            ranking_policy_digest,
            cohort,
            selected,
        };
        receipt.validate_semantics()?;
        Ok(receipt)
    }
}

/// Durable global Top-25 selected only from calendar-authoritative V4 inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionReceiptV2 {
    selection_id: [u8; 32],
    rung_seconds: u32,
    populations: [PopulationReferenceV2; 2],
    proof: PopulationProof,
    ranking_policy_digest: [u8; 32],
    cohort: SharedCohortIdentityV2,
    selected: Vec<SelectedEntryV1>,
}

impl SelectionReceiptV2 {
    /// Recomputes one global Top-25 from exactly two V4 populations.
    ///
    /// # Errors
    ///
    /// Refuses missing V4 authority, family/rung/span/shared-policy mismatch,
    /// same-side exit-policy mismatch, different actual signal or one-minute
    /// coverage, malformed pages, two-pass disagreement, or any invalid field.
    pub fn from_committed_populations(
        populations: &mut PopulationLedger,
        nifty_population_id: [u8; 32],
        bank_nifty_population_id: [u8; 32],
        policy: RankingPolicyV1,
    ) -> Result<Self, SelectionRefusal> {
        if nifty_population_id == bank_nifty_population_id {
            return Err("NIFTY and BANKNIFTY V4 populations cannot share one identity".to_owned());
        }
        let nifty_v4 = populations
            .receipt_v4(&nifty_population_id)
            .ok_or_else(|| {
                format!(
                    "NIFTY population {} is not an authoritative calendar-bound V4 population",
                    hex(&nifty_population_id)
                )
            })?;
        let bank_v4 = populations
            .receipt_v4(&bank_nifty_population_id)
            .ok_or_else(|| {
                format!(
                    "BANKNIFTY population {} is not an authoritative calendar-bound V4 population",
                    hex(&bank_nifty_population_id)
                )
            })?;
        let nifty_receipt = nifty_v4.v3().v2();
        let bank_receipt = bank_v4.v3().v2();
        if nifty_receipt.rung_seconds != bank_receipt.rung_seconds {
            return Err(format!(
                "global V2 selection requires one timeframe, but NIFTY is {}s and BANKNIFTY is {}s",
                nifty_receipt.rung_seconds, bank_receipt.rung_seconds
            ));
        }
        let ranking_policy_digest = policy.digest();
        for (name, receipt) in [("NIFTY", nifty_receipt), ("BANKNIFTY", bank_receipt)] {
            if receipt.identities.ranking_policy_digest != ranking_policy_digest {
                return Err(format!(
                    "{name} V4 population ranking-policy identity does not match the requested canonical policy"
                ));
            }
        }
        let cohort = SharedCohortIdentityV2::from_v4_receipts(&nifty_v4, &bank_v4)?;
        let references = [
            PopulationReferenceV2::from_receipt(&nifty_v4, InstrumentFamilyV1::Nifty)?,
            PopulationReferenceV2::from_receipt(&bank_v4, InstrumentFamilyV1::BankNifty)?,
        ];

        let mut extrema = Extrema::default();
        let mut first_pass = PopulationPass::new();
        visit_both_v2(populations, references, |_, _row, candidate| {
            extrema.observe(&candidate);
            first_pass.observe(&candidate);
            Ok(())
        })?;
        let proof = first_pass.finish();
        validate_proof_against_receipts(proof, &nifty_receipt, &bank_receipt)?;

        let mut keeper = VerifiedKeeper::new(MAX_TOP, policy, extrema, proof)
            .map_err(|why| format!("global V2 Top-25 could not start: {why:?}"))?;
        visit_both_v2(populations, references, |_, _row, candidate| {
            keeper
                .offer(candidate)
                .map_err(|why| format!("global V2 Top-25 candidate was refused: {why:?}"))
        })?;
        let selection = keeper
            .finish()
            .map_err(|why| format!("global V2 Top-25 did not reproduce its first pass: {why:?}"))?;
        validate_selection_reconciliation(&selection, proof)?;
        let selected = resolve_selected_v2(populations, references, &selection)?;

        let mut receipt = Self {
            selection_id: [0; 32],
            rung_seconds: nifty_receipt.rung_seconds,
            populations: references,
            proof,
            ranking_policy_digest,
            cohort,
            selected,
        };
        receipt.selection_id = receipt.derived_id()?;
        receipt.validate_semantics()?;
        Ok(receipt)
    }

    /// Rebuilds and byte-compares the complete receipt.
    ///
    /// # Errors
    ///
    /// Every V2 construction refusal or any byte mismatch.
    pub fn verify_against_populations(
        &self,
        populations: &mut PopulationLedger,
        policy: RankingPolicyV1,
    ) -> Result<(), SelectionRefusal> {
        let [nifty, bank_nifty] = self.populations;
        let rebuilt = Self::from_committed_populations(
            populations,
            nifty.population_id,
            bank_nifty.population_id,
            policy,
        )?;
        if rebuilt != *self {
            return Err(
                "selection V2 receipt differs from a full replay of its V4 populations".to_owned(),
            );
        }
        Ok(())
    }

    /// Content-derived selection identity.
    #[must_use]
    pub const fn selection_id(&self) -> [u8; 32] {
        self.selection_id
    }

    /// Shared signal timeframe.
    #[must_use]
    pub const fn rung_seconds(&self) -> u32 {
        self.rung_seconds
    }

    /// Canonical `[NIFTY, BANKNIFTY]` V4 references.
    #[must_use]
    pub const fn populations(&self) -> &[PopulationReferenceV2; 2] {
        &self.populations
    }

    /// Exact two-pass combined population proof.
    #[must_use]
    pub const fn population_proof(&self) -> PopulationProof {
        self.proof
    }

    /// Canonical final-ranking policy identity.
    #[must_use]
    pub const fn ranking_policy_digest(&self) -> [u8; 32] {
        self.ranking_policy_digest
    }

    /// Complete shared V2 cohort identity.
    #[must_use]
    pub const fn cohort_identity(&self) -> SharedCohortIdentityV2 {
        self.cohort
    }

    fn discovery_key(&self) -> ([u8; 32], u32) {
        (self.cohort.digest(), self.rung_seconds)
    }

    /// Strongest-first Top-25 or all admitted rows when fewer exist.
    #[must_use]
    pub fn top_twenty_five(&self) -> &[SelectedEntryV1] {
        &self.selected
    }

    /// First ten rows of the exact persisted Top-25 ordering.
    #[must_use]
    pub fn top_ten(&self) -> &[SelectedEntryV1] {
        let end = self.selected.len().min(10);
        self.selected.get(..end).unwrap_or(&[])
    }

    fn validate_semantics(&self) -> Result<(), SelectionRefusal> {
        require_digest("V2 selection_id", &self.selection_id)?;
        require_digest("V2 ranking_policy_digest", &self.ranking_policy_digest)?;
        if ![60_u32, 120, 180, 300, 600, 900, 1_800, 3_600].contains(&self.rung_seconds) {
            return Err(format!(
                "selection V2 timeframe {} seconds is outside the eight canonical intraday rungs",
                self.rung_seconds
            ));
        }
        require_digest(
            "V2 population proof ordered_digest",
            &self.proof.ordered_digest,
        )?;
        self.cohort.validate()?;
        if self.cohort.ranking_policy_digest != self.ranking_policy_digest {
            return Err(
                "selection V2 ranking-policy digest differs from its cohort identity".to_owned(),
            );
        }
        let [nifty, bank_nifty] = self.populations;
        nifty.validate(InstrumentFamilyV1::Nifty)?;
        bank_nifty.validate(InstrumentFamilyV1::BankNifty)?;
        if nifty.population_id == bank_nifty.population_id {
            return Err("both V2 families reference the same population".to_owned());
        }
        let expected_considered = checked_add(
            nifty.row_count,
            bank_nifty.row_count,
            "combined V2 population row counts",
        )?;
        if self.proof.considered != expected_considered {
            return Err(format!(
                "V2 population proof considered {}, but source row counts total {expected_considered}",
                self.proof.considered
            ));
        }
        if checked_add(
            self.proof.admitted,
            self.proof.refused,
            "V2 proof verdict counts",
        )? != self.proof.considered
        {
            return Err(
                "V2 population proof admitted + refused does not equal considered".to_owned(),
            );
        }
        if self.proof.unmeasured > self.proof.considered {
            return Err("V2 population proof unmeasured exceeds considered".to_owned());
        }
        let expected_selected = usize::try_from(self.proof.admitted)
            .unwrap_or(usize::MAX)
            .min(MAX_TOP);
        if self.selected.len() != expected_selected {
            return Err(format!(
                "selection V2 has {} row(s), but admitted={} requires exactly {expected_selected}",
                self.selected.len(),
                self.proof.admitted
            ));
        }
        for entry in &self.selected {
            entry.validate_against_v2(nifty, bank_nifty)?;
        }
        for (index, left) in self.selected.iter().enumerate() {
            for right in self.selected.iter().skip(index.saturating_add(1)) {
                if left.population_id == right.population_id
                    && left.row_sequence == right.row_sequence
                {
                    return Err("selection V2 repeats one population row".to_owned());
                }
                if left.strategy_digest == right.strategy_digest {
                    return Err("selection V2 repeats one semantic strategy digest".to_owned());
                }
            }
        }
        if self.selected.windows(2).any(|pair| {
            pair.first()
                .zip(pair.get(1))
                .is_some_and(|(a, b)| a.score < b.score)
        }) {
            return Err("selection V2 scores are not strongest-first".to_owned());
        }
        if self.derived_id()? != self.selection_id {
            return Err("selection V2 identity does not match its canonical content".to_owned());
        }
        Ok(())
    }

    fn derived_id(&self) -> Result<[u8; 32], SelectionRefusal> {
        let payload = self.payload_with_id([0; 32])?;
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(b"brutex-global-selection-id-v2\0");
        hasher.update(
            payload
                .get(32..)
                .ok_or_else(|| "selection V2 identity payload is absent".to_owned())?,
        );
        Ok(hasher.finalize())
    }

    fn payload_with_id(
        &self,
        selection_id: [u8; 32],
    ) -> Result<[u8; PAYLOAD_BYTES_V2], SelectionRefusal> {
        let mut payload = [0_u8; PAYLOAD_BYTES_V2];
        let mut encoder = Encoder::new(&mut payload);
        encoder.bytes(&selection_id)?;
        encoder.u32(self.rung_seconds)?;
        encoder.u32(REQUESTED_TOP_V1)?;
        let [nifty, bank_nifty] = self.populations;
        nifty.encode(&mut encoder)?;
        bank_nifty.encode(&mut encoder)?;
        for count in [
            self.proof.considered,
            self.proof.admitted,
            self.proof.refused,
            self.proof.unmeasured,
        ] {
            encoder.u64(count)?;
        }
        encoder.bytes(&self.proof.ordered_digest)?;
        encoder.bytes(&self.ranking_policy_digest)?;
        encoder.bytes(&self.cohort.canonical_bytes())?;
        encoder.u32(
            u32::try_from(self.selected.len())
                .map_err(|_| "selected V2 row count does not fit u32".to_owned())?,
        )?;
        encoder.zeros(4)?;
        for entry in &self.selected {
            entry.encode(&mut encoder)?;
        }
        encoder.zeros(
            MAX_TOP
                .saturating_sub(self.selected.len())
                .saturating_mul(SELECTED_ENTRY_BYTES),
        )?;
        encoder.zeros(8)?;
        encoder.finish()?;
        Ok(payload)
    }

    fn to_bytes(&self) -> Result<[u8; SELECTION_V2_STRIDE_BYTES], SelectionRefusal> {
        self.validate_semantics()?;
        let payload = self.payload_with_id(self.selection_id)?;
        let seal = brutex_core::blake3::hash(&payload);
        let mut raw = [0_u8; SELECTION_V2_STRIDE_BYTES];
        raw.get_mut(..PAYLOAD_BYTES_V2)
            .ok_or_else(|| "selection V2 payload slot is absent".to_owned())?
            .copy_from_slice(&payload);
        raw.get_mut(PAYLOAD_BYTES_V2..)
            .ok_or_else(|| "selection V2 seal slot is absent".to_owned())?
            .copy_from_slice(&seal);
        Ok(raw)
    }

    fn from_bytes(raw: &[u8; SELECTION_V2_STRIDE_BYTES]) -> Result<Self, SelectionRefusal> {
        let payload = raw
            .get(..PAYLOAD_BYTES_V2)
            .ok_or_else(|| "selection V2 payload is absent".to_owned())?;
        let stored_seal = raw
            .get(PAYLOAD_BYTES_V2..)
            .ok_or_else(|| "selection V2 seal is absent".to_owned())?;
        if stored_seal != brutex_core::blake3::hash(payload) {
            return Err("selection V2 receipt failed its complete BLAKE3 seal".to_owned());
        }
        let mut decoder = Decoder::new(payload);
        let selection_id = decoder.array_32()?;
        let rung_seconds = decoder.u32()?;
        let requested = decoder.u32()?;
        if requested != REQUESTED_TOP_V1 {
            return Err(format!(
                "selection V2 requests {requested}; version two requires exactly {REQUESTED_TOP_V1}"
            ));
        }
        let populations = [
            PopulationReferenceV2::decode(&mut decoder, InstrumentFamilyV1::Nifty)?,
            PopulationReferenceV2::decode(&mut decoder, InstrumentFamilyV1::BankNifty)?,
        ];
        let proof = PopulationProof {
            considered: decoder.u64()?,
            admitted: decoder.u64()?,
            refused: decoder.u64()?,
            unmeasured: decoder.u64()?,
            ordered_digest: decoder.array_32()?,
        };
        let ranking_policy_digest = decoder.array_32()?;
        let cohort = SharedCohortIdentityV2::decode(&mut decoder)?;
        let selected_count = usize::try_from(decoder.u32()?)
            .map_err(|_| "selected V2 count does not fit this machine".to_owned())?;
        if selected_count > MAX_TOP {
            return Err(format!(
                "selection V2 stores {selected_count} rows above fixed maximum {MAX_TOP}"
            ));
        }
        decoder.zeros(4, "selection V2 count reserve")?;
        let mut selected = Vec::new();
        selected.try_reserve_exact(selected_count).map_err(|why| {
            format!("selection V2 could not reserve {selected_count} bounded rows: {why}")
        })?;
        for _ in 0..selected_count {
            selected.push(SelectedEntryV1::decode(&mut decoder)?);
        }
        decoder.zeros(
            MAX_TOP
                .saturating_sub(selected_count)
                .saturating_mul(SELECTED_ENTRY_BYTES),
            "unused selection V2 entry slots",
        )?;
        decoder.zeros(8, "selection V2 trailing reserve")?;
        decoder.finish()?;
        let receipt = Self {
            selection_id,
            rung_seconds,
            populations,
            proof,
            ranking_policy_digest,
            cohort,
            selected,
        };
        receipt.validate_semantics()?;
        Ok(receipt)
    }
}

/// Constant-size evidence for the exact selection file generation indexed.
///
/// This detects ordinary same-length mutation/path replacement without hiding
/// an O(history) rescan in append. It is not authentication against an actor
/// able to forge filesystem metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGeneration {
    len: u64,
    platform: PlatformGeneration,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGeneration {
    device: u64,
    inode: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGeneration {
    volume_serial: u32,
    file_index: u64,
    creation_time: u64,
    last_write_time: u64,
}

#[cfg(not(any(unix, windows)))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGeneration;

/// Append-only fixed-stride selection receipt ledger.
#[derive(Debug)]
pub struct SelectionLedger {
    file: File,
    path: PathBuf,
    receipts: HashMap<[u8; 32], SelectionReceiptV1>,
    /// Selection identities in exact append/file order.
    order: Vec<[u8; 32]>,
    /// Last append for one exact `(shared cohort digest, signal rung)` pair.
    latest: HashMap<([u8; 32], u32), [u8; 32]>,
    scanned: u64,
    generation: FileGeneration,
    writable: bool,
    max_receipts: usize,
}

impl SelectionLedger {
    /// Version-one ledger path.
    #[must_use]
    pub fn path(root: &Path) -> PathBuf {
        root.join("results").join("global-selections-v1.bin")
    }

    /// Opens or creates the append-only ledger.
    ///
    /// # Errors
    ///
    /// Refuses I/O/locking errors, a bad header/version/stride/reserve, ragged
    /// records, a torn seal, malformed receipt, duplicate selection identity,
    /// a zero caller bound or a ledger above that bound.
    pub fn open(root: &Path, max_receipts: usize) -> Result<Self, SelectionRefusal> {
        validate_receipt_limit(max_receipts)?;
        let path = Self::path(root);
        let parent = path
            .parent()
            .ok_or_else(|| "selection ledger has no parent directory".to_owned())?;
        std::fs::create_dir_all(parent).map_err(|why| {
            format!(
                "selection ledger directory {} could not be created: {why}",
                parent.display()
            )
        })?;
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
        Self::open_file(file, path, true, max_receipts)
    }

    /// Opens an existing ledger without creating or appending anything.
    ///
    /// # Errors
    ///
    /// Returns every structural refusal from [`Self::open`], plus absence.  The
    /// caller must supply the same kind of explicit nonzero receipt bound; no
    /// process-wide or environment-derived default exists.
    pub fn open_read(root: &Path, max_receipts: usize) -> Result<Self, SelectionRefusal> {
        validate_receipt_limit(max_receipts)?;
        let path = Self::path(root);
        let file = OpenOptions::new()
            .read(true)
            .open(&path)
            .map_err(|why| format!("{} could not be opened read-only: {why}", path.display()))?;
        Self::open_file(file, path, false, max_receipts)
    }

    fn open_file(
        mut file: File,
        path: PathBuf,
        writable: bool,
        max_receipts: usize,
    ) -> Result<Self, SelectionRefusal> {
        if writable {
            file.lock().map_err(|why| {
                format!("{} could not be exclusively locked: {why}", path.display())
            })?;
        } else {
            file.lock_shared()
                .map_err(|why| format!("{} could not be shared-locked: {why}", path.display()))?;
        }
        let opened = (|| {
            let len = file
                .metadata()
                .map_err(|why| format!("{} length could not be read: {why}", path.display()))?
                .len();
            if len == 0 {
                if !writable {
                    return Err(format!("{} is empty and read-only", path.display()));
                }
                write_header(&mut file)?;
                file.sync_all().map_err(|why| {
                    format!("{} header could not be synced: {why}", path.display())
                })?;
            }
            let len = file
                .metadata()
                .map_err(|why| format!("{} length could not be reread: {why}", path.display()))?
                .len();
            let indexes = scan_file(&mut file, &path, len, max_receipts)?;
            let generation = validated_generation(&file, &path, len)?;
            Ok((indexes, len, generation))
        })();
        let released = file
            .unlock()
            .map_err(|why| format!("{} could not be unlocked after open: {why}", path.display()));
        match (opened, released) {
            (Ok((indexes, scanned, generation)), Ok(())) => Ok(Self {
                file,
                path,
                receipts: indexes.receipts,
                order: indexes.order,
                latest: indexes.latest,
                scanned,
                generation,
                writable,
                max_receipts,
            }),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// Number of indexed receipts.
    #[must_use]
    pub fn selections(&self) -> usize {
        self.order.len()
    }

    /// Selection identities in exact append/file order.
    ///
    /// The slice borrows the ledger's bounded in-memory index and performs no
    /// allocation or re-sorting. Every ID has exactly one matching receipt in
    /// [`Self::receipt`].
    #[must_use]
    pub fn selection_ids(&self) -> &[[u8; 32]] {
        &self.order
    }

    /// Average-O(1) in-memory selection lookup.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    #[must_use]
    pub fn receipt(&self, selection_id: &[u8; 32]) -> Option<&SelectionReceiptV1> {
        self.receipts.get(selection_id)
    }

    /// Latest receipt in file order for one exact cohort and signal rung.
    ///
    /// This is one average-O(1) hash lookup. It never falls back to a different
    /// cohort or timeframe and never infers a "current" span from timestamps.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    #[must_use]
    pub fn latest_selection(
        &self,
        cohort_digest: [u8; 32],
        rung_seconds: u32,
    ) -> Option<&SelectionReceiptV1> {
        self.latest
            .get(&(cohort_digest, rung_seconds))
            .and_then(|selection_id| self.receipts.get(selection_id))
    }

    /// Appends one new fixed-stride receipt and syncs it durably.
    ///
    /// # Errors
    ///
    /// Refuses a read-only handle, invalid receipt, stale-handle corruption,
    /// duplicate identity (including an exact duplicate), same-id/different
    /// bytes, arithmetic/I/O/locking failure. No prior byte is replaced.
    pub fn append(&mut self, receipt: &SelectionReceiptV1) -> Result<(), SelectionRefusal> {
        if !self.writable {
            return Err("a read-only selection ledger cannot append".to_owned());
        }
        receipt.validate_semantics()?;
        self.file.lock().map_err(|why| {
            format!(
                "{} could not be locked for append: {why}",
                self.path.display()
            )
        })?;
        let attempted = (|| {
            self.absorb_new()?;
            if let Some(existing) = self.receipts.get(&receipt.selection_id) {
                if existing == receipt {
                    return Err(format!(
                        "selection {} is already present; duplicate selection identities are refused",
                        hex(&receipt.selection_id)
                    ));
                }
                return Err(format!(
                    "selection {} already names different canonical bytes",
                    hex(&receipt.selection_id)
                ));
            }
            if self.order.len() >= self.max_receipts {
                return Err(format!(
                    "selection ledger already contains its caller-supplied maximum of {} receipt(s)",
                    self.max_receipts
                ));
            }
            self.receipts
                .try_reserve(1)
                .map_err(|why| format!("selection index could not reserve one slot: {why}"))?;
            self.order
                .try_reserve(1)
                .map_err(|why| format!("selection order could not reserve one slot: {why}"))?;
            self.latest.try_reserve(1).map_err(|why| {
                format!("selection latest index could not reserve one slot: {why}")
            })?;
            let raw = receipt.to_bytes()?;
            self.file
                .seek(SeekFrom::End(0))
                .map_err(|why| format!("selection ledger could not seek to append: {why}"))?;
            self.file
                .write_all(&raw)
                .map_err(|why| format!("selection receipt could not be appended: {why}"))?;
            self.file
                .sync_all()
                .map_err(|why| format!("selection receipt could not be synced: {why}"))?;
            self.scanned = self
                .scanned
                .checked_add(SELECTION_STRIDE)
                .ok_or_else(|| "selection scanned length overflowed u64".to_owned())?;
            self.generation = validated_generation(&self.file, &self.path, self.scanned)?;
            let selection_id = receipt.selection_id;
            self.receipts.insert(selection_id, receipt.clone());
            self.order.push(selection_id);
            self.latest.insert(receipt.discovery_key(), selection_id);
            Ok(())
        })();
        let released = self.file.unlock().map_err(|why| {
            format!(
                "{} could not be unlocked after append: {why}",
                self.path.display()
            )
        });
        match (attempted, released) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(why), _) | (Ok(()), Err(why)) => Err(why),
        }
    }

    fn absorb_new(&mut self) -> Result<(), SelectionRefusal> {
        let observed = file_generation(&self.file, &self.path)?;
        let len = observed.len;
        validate_file_length(len)?;
        let total = receipt_capacity(len, self.max_receipts)?;
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|why| format!("selection ledger could not recheck its header: {why}"))?;
        let mut header = [0_u8; HEADER_BYTES];
        self.file
            .read_exact(&mut header)
            .map_err(|why| format!("selection ledger header could not be rechecked: {why}"))?;
        validate_header(&header)?;
        if len < self.scanned {
            return Err(format!(
                "selection ledger shrank from {} to {len}; append-only history was violated",
                self.scanned
            ));
        }
        if len == self.scanned {
            require_generation_unchanged(self.generation, observed, &self.path)?;
            self.generation = observed;
            return Ok(());
        }
        let held = self.order.len();
        let new_count = total
            .checked_sub(held)
            .ok_or_else(|| "selection receipt count moved behind its in-memory index".to_owned())?;
        self.file
            .seek(SeekFrom::Start(self.scanned))
            .map_err(|why| format!("selection ledger could not seek to new receipts: {why}"))?;
        let added = scan_receipts(
            &mut self.file,
            &self.path,
            self.scanned,
            new_count,
            Some(&self.receipts),
        )?;
        self.receipts.try_reserve(new_count).map_err(|why| {
            format!("selection index could not reserve {new_count} new slot(s): {why}")
        })?;
        self.order.try_reserve(new_count).map_err(|why| {
            format!("selection order could not reserve {new_count} new slot(s): {why}")
        })?;
        self.latest.try_reserve(new_count).map_err(|why| {
            format!("selection latest index could not reserve {new_count} new slot(s): {why}")
        })?;
        let SelectionIndexes {
            receipts: added_receipts,
            order: added_order,
            latest: _,
        } = added;
        for selection_id in added_order {
            let receipt = added_receipts.get(&selection_id).ok_or_else(|| {
                "selection scan produced an ordered ID without its receipt".to_owned()
            })?;
            self.latest.insert(receipt.discovery_key(), selection_id);
            self.receipts.insert(selection_id, receipt.clone());
            self.order.push(selection_id);
        }
        self.scanned = len;
        self.generation = validated_generation(&self.file, &self.path, len)?;
        Ok(())
    }
}

/// Append-only ledger for calendar-authoritative global Selection V2 receipts.
///
/// Version one remains available through [`SelectionLedger`] for audit only;
/// this type uses its own path, codec and indexes and never falls back to V1.
#[derive(Debug)]
pub struct SelectionLedgerV2 {
    file: File,
    path: PathBuf,
    receipts: HashMap<[u8; 32], SelectionReceiptV2>,
    order: Vec<[u8; 32]>,
    latest: HashMap<([u8; 32], u32), [u8; 32]>,
    scanned: u64,
    generation: FileGeneration,
    writable: bool,
    max_receipts: usize,
}

impl SelectionLedgerV2 {
    /// Version-two ledger path. It never aliases the V1 audit file.
    #[must_use]
    pub fn path(root: &Path) -> PathBuf {
        root.join("results").join("global-selections-v2.bin")
    }

    /// Opens or creates the append-only V2 ledger.
    ///
    /// # Errors
    ///
    /// Refuses I/O/locking errors, malformed headers or receipts, duplicate
    /// identities, ragged/torn bytes, a zero bound, or a file above the bound.
    pub fn open(root: &Path, max_receipts: usize) -> Result<Self, SelectionRefusal> {
        validate_receipt_limit(max_receipts)?;
        let path = Self::path(root);
        let parent = path
            .parent()
            .ok_or_else(|| "selection V2 ledger has no parent directory".to_owned())?;
        std::fs::create_dir_all(parent).map_err(|why| {
            format!(
                "selection V2 ledger directory {} could not be created: {why}",
                parent.display()
            )
        })?;
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
        Self::open_file(file, path, true, max_receipts)
    }

    /// Opens an existing V2 ledger without creating or appending.
    ///
    /// # Errors
    ///
    /// Every structural refusal from [`Self::open`], plus absence.
    pub fn open_read(root: &Path, max_receipts: usize) -> Result<Self, SelectionRefusal> {
        validate_receipt_limit(max_receipts)?;
        let path = Self::path(root);
        let file = OpenOptions::new()
            .read(true)
            .open(&path)
            .map_err(|why| format!("{} could not be opened read-only: {why}", path.display()))?;
        Self::open_file(file, path, false, max_receipts)
    }

    fn open_file(
        mut file: File,
        path: PathBuf,
        writable: bool,
        max_receipts: usize,
    ) -> Result<Self, SelectionRefusal> {
        if writable {
            file.lock().map_err(|why| {
                format!("{} could not be exclusively locked: {why}", path.display())
            })?;
        } else {
            file.lock_shared()
                .map_err(|why| format!("{} could not be shared-locked: {why}", path.display()))?;
        }
        let opened = (|| {
            let len = file
                .metadata()
                .map_err(|why| format!("{} length could not be read: {why}", path.display()))?
                .len();
            if len == 0 {
                if !writable {
                    return Err(format!("{} is empty and read-only", path.display()));
                }
                write_header_v2(&mut file)?;
                file.sync_all().map_err(|why| {
                    format!("{} header could not be synced: {why}", path.display())
                })?;
            }
            let len = file
                .metadata()
                .map_err(|why| format!("{} length could not be reread: {why}", path.display()))?
                .len();
            let indexes = scan_file_v2(&mut file, &path, len, max_receipts)?;
            let generation = validated_generation(&file, &path, len)?;
            Ok((indexes, len, generation))
        })();
        let released = file
            .unlock()
            .map_err(|why| format!("{} could not be unlocked after open: {why}", path.display()));
        match (opened, released) {
            (Ok((indexes, scanned, generation)), Ok(())) => Ok(Self {
                file,
                path,
                receipts: indexes.receipts,
                order: indexes.order,
                latest: indexes.latest,
                scanned,
                generation,
                writable,
                max_receipts,
            }),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// Number of indexed V2 receipts.
    #[must_use]
    pub fn selections(&self) -> usize {
        self.order.len()
    }

    /// V2 identities in exact append order.
    #[must_use]
    pub fn selection_ids(&self) -> &[[u8; 32]] {
        &self.order
    }

    /// Average-O(1) exact V2 receipt lookup.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    #[must_use]
    pub fn receipt(&self, selection_id: &[u8; 32]) -> Option<&SelectionReceiptV2> {
        self.receipts.get(selection_id)
    }

    /// Average-O(1) latest lookup for one exact V2 cohort and rung.
    ///
    /// No V1 receipt, different coverage digest or different rung is eligible.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    #[must_use]
    pub fn latest_selection(
        &self,
        cohort_digest: [u8; 32],
        rung_seconds: u32,
    ) -> Option<&SelectionReceiptV2> {
        self.latest
            .get(&(cohort_digest, rung_seconds))
            .and_then(|selection_id| self.receipts.get(selection_id))
    }

    /// Appends and syncs one complete fixed-stride V2 receipt.
    ///
    /// # Errors
    ///
    /// Refuses read-only use, invalid or duplicate bytes, a stale generation,
    /// bound/arithmetic failure, or any I/O/locking failure.
    pub fn append(&mut self, receipt: &SelectionReceiptV2) -> Result<(), SelectionRefusal> {
        if !self.writable {
            return Err("a read-only selection V2 ledger cannot append".to_owned());
        }
        receipt.validate_semantics()?;
        self.file.lock().map_err(|why| {
            format!(
                "{} could not be locked for V2 append: {why}",
                self.path.display()
            )
        })?;
        let attempted = (|| {
            self.absorb_new()?;
            if let Some(existing) = self.receipts.get(&receipt.selection_id) {
                if existing == receipt {
                    return Err(format!(
                        "selection V2 {} is already present; duplicate identities are refused",
                        hex(&receipt.selection_id)
                    ));
                }
                return Err(format!(
                    "selection V2 {} already names different canonical bytes",
                    hex(&receipt.selection_id)
                ));
            }
            if self.order.len() >= self.max_receipts {
                return Err(format!(
                    "selection V2 ledger already contains its caller-supplied maximum of {} receipt(s)",
                    self.max_receipts
                ));
            }
            self.receipts
                .try_reserve(1)
                .map_err(|why| format!("selection V2 index could not reserve one slot: {why}"))?;
            self.order
                .try_reserve(1)
                .map_err(|why| format!("selection V2 order could not reserve one slot: {why}"))?;
            self.latest.try_reserve(1).map_err(|why| {
                format!("selection V2 latest index could not reserve one slot: {why}")
            })?;
            let raw = receipt.to_bytes()?;
            self.file
                .seek(SeekFrom::End(0))
                .map_err(|why| format!("selection V2 ledger could not seek to append: {why}"))?;
            self.file
                .write_all(&raw)
                .map_err(|why| format!("selection V2 receipt could not be appended: {why}"))?;
            self.file
                .sync_all()
                .map_err(|why| format!("selection V2 receipt could not be synced: {why}"))?;
            self.scanned = self
                .scanned
                .checked_add(SELECTION_V2_STRIDE)
                .ok_or_else(|| "selection V2 scanned length overflowed u64".to_owned())?;
            self.generation = validated_generation(&self.file, &self.path, self.scanned)?;
            let selection_id = receipt.selection_id;
            self.receipts.insert(selection_id, receipt.clone());
            self.order.push(selection_id);
            self.latest.insert(receipt.discovery_key(), selection_id);
            Ok(())
        })();
        let released = self.file.unlock().map_err(|why| {
            format!(
                "{} could not be unlocked after V2 append: {why}",
                self.path.display()
            )
        });
        match (attempted, released) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(why), _) | (Ok(()), Err(why)) => Err(why),
        }
    }

    fn absorb_new(&mut self) -> Result<(), SelectionRefusal> {
        let observed = file_generation(&self.file, &self.path)?;
        let len = observed.len;
        validate_file_length_v2(len)?;
        let total = receipt_capacity_v2(len, self.max_receipts)?;
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|why| format!("selection V2 ledger could not recheck its header: {why}"))?;
        let mut header = [0_u8; HEADER_BYTES];
        self.file
            .read_exact(&mut header)
            .map_err(|why| format!("selection V2 ledger header could not be rechecked: {why}"))?;
        validate_header_v2(&header)?;
        if len < self.scanned {
            return Err(format!(
                "selection V2 ledger shrank from {} to {len}; append-only history was violated",
                self.scanned
            ));
        }
        if len == self.scanned {
            require_generation_unchanged(self.generation, observed, &self.path)?;
            self.generation = observed;
            return Ok(());
        }
        let held = self.order.len();
        let new_count = total
            .checked_sub(held)
            .ok_or_else(|| "selection V2 count moved behind its in-memory index".to_owned())?;
        self.file
            .seek(SeekFrom::Start(self.scanned))
            .map_err(|why| format!("selection V2 ledger could not seek to new receipts: {why}"))?;
        let added = scan_receipts_v2(
            &mut self.file,
            &self.path,
            self.scanned,
            new_count,
            Some(&self.receipts),
        )?;
        self.receipts.try_reserve(new_count).map_err(|why| {
            format!("selection V2 index could not reserve {new_count} new slot(s): {why}")
        })?;
        self.order.try_reserve(new_count).map_err(|why| {
            format!("selection V2 order could not reserve {new_count} new slot(s): {why}")
        })?;
        self.latest.try_reserve(new_count).map_err(|why| {
            format!("selection V2 latest index could not reserve {new_count} new slot(s): {why}")
        })?;
        let SelectionIndexesV2 {
            receipts: added_receipts,
            order: added_order,
            latest: _,
        } = added;
        for selection_id in added_order {
            let receipt = added_receipts.get(&selection_id).ok_or_else(|| {
                "selection V2 scan produced an ordered ID without its receipt".to_owned()
            })?;
            self.latest.insert(receipt.discovery_key(), selection_id);
            self.receipts.insert(selection_id, receipt.clone());
            self.order.push(selection_id);
        }
        self.scanned = len;
        self.generation = validated_generation(&self.file, &self.path, len)?;
        Ok(())
    }
}

fn ensure_matching_v2_shared_terms(
    nifty: &PopulationIdentitiesV2,
    bank_nifty: &PopulationIdentitiesV2,
) -> Result<(), SelectionRefusal> {
    for (name, left, right) in [
        ("feed", nifty.feed_digest, bank_nifty.feed_digest),
        (
            "source commit",
            nifty.source_commit_digest,
            bank_nifty.source_commit_digest,
        ),
        (
            "vocabulary",
            nifty.vocabulary_digest,
            bank_nifty.vocabulary_digest,
        ),
        (
            "evaluation policy",
            nifty.evaluation_policy_digest,
            bank_nifty.evaluation_policy_digest,
        ),
        (
            "admission policy",
            nifty.admission_policy_digest,
            bank_nifty.admission_policy_digest,
        ),
        (
            "ranking policy",
            nifty.ranking_policy_digest,
            bank_nifty.ranking_policy_digest,
        ),
        (
            "calendar policy",
            nifty.calendar_policy_digest,
            bank_nifty.calendar_policy_digest,
        ),
        (
            "daily-reference policy",
            nifty.daily_reference_policy_digest,
            bank_nifty.daily_reference_policy_digest,
        ),
    ] {
        if left != right {
            return Err(format!(
                "NIFTY and BANKNIFTY populations have different shared {name} identities"
            ));
        }
    }
    Ok(())
}

fn validate_proof_against_receipts(
    proof: PopulationProof,
    nifty: &CompletionReceiptV2,
    bank_nifty: &CompletionReceiptV2,
) -> Result<(), SelectionRefusal> {
    let considered = checked_add(nifty.row_count, bank_nifty.row_count, "population rows")?;
    let admitted = checked_add(
        nifty.admitted_rows,
        bank_nifty.admitted_rows,
        "admitted population rows",
    )?;
    let nifty_refused = checked_sum(
        [
            nifty.rejected_rows,
            nifty.unmeasured_rows,
            nifty.refused_rows,
        ],
        "NIFTY non-admitted rows",
    )?;
    let bank_refused = checked_sum(
        [
            bank_nifty.rejected_rows,
            bank_nifty.unmeasured_rows,
            bank_nifty.refused_rows,
        ],
        "BANKNIFTY non-admitted rows",
    )?;
    let refused = checked_add(nifty_refused, bank_refused, "non-admitted rows")?;
    let unmeasured = checked_add(
        nifty.topn_undefined_rows,
        bank_nifty.topn_undefined_rows,
        "undefined Top-N rows",
    )?;
    if proof.considered != considered
        || proof.admitted != admitted
        || proof.refused != refused
        || proof.unmeasured != unmeasured
    {
        return Err(format!(
            "combined population proof counts {}/{}/{}/{} do not reconcile to authoritative receipts {considered}/{admitted}/{refused}/{unmeasured}",
            proof.considered, proof.admitted, proof.refused, proof.unmeasured
        ));
    }
    require_digest("combined ordered population digest", &proof.ordered_digest)
}

fn validate_selection_reconciliation(
    selection: &Selection,
    proof: PopulationProof,
) -> Result<(), SelectionRefusal> {
    if selection.requested != MAX_TOP {
        return Err(format!(
            "ranking kernel returned requested={}, not the authoritative {MAX_TOP}",
            selection.requested
        ));
    }
    if selection.considered != proof.considered
        || selection.admitted != proof.admitted
        || selection.refused != proof.refused
        || selection.unmeasured != proof.unmeasured
    {
        return Err("ranking selection counts do not reproduce the combined proof".to_owned());
    }
    let expected = usize::try_from(proof.admitted)
        .unwrap_or(usize::MAX)
        .min(MAX_TOP);
    if selection.rows.len() != expected {
        return Err(format!(
            "ranking kernel returned {} rows, but admitted={} requires {expected}",
            selection.rows.len(),
            proof.admitted
        ));
    }
    if selection.rows.windows(2).any(|pair| {
        pair.first()
            .zip(pair.get(1))
            .is_some_and(|(stronger, weaker)| stronger < weaker)
    }) {
        return Err("ranking kernel output is not strongest-first".to_owned());
    }
    Ok(())
}

fn resolve_selected(
    populations: &mut PopulationLedger,
    references: [PopulationReferenceV1; 2],
    selection: &Selection,
) -> Result<Vec<SelectedEntryV1>, SelectionRefusal> {
    let mut resolved = vec![None; selection.rows.len()];
    visit_both(populations, references, |reference, row, candidate| {
        for (index, ranked) in selection.rows.iter().enumerate() {
            if candidate.strategy_digest != ranked.candidate.strategy_digest {
                continue;
            }
            if candidate != ranked.candidate {
                return Err(format!(
                    "strategy digest {} aliases different candidate bytes across populations",
                    hex(&candidate.strategy_digest.bytes())
                ));
            }
            let slot = resolved
                .get_mut(index)
                .ok_or_else(|| "bounded selected-entry slot is absent".to_owned())?;
            if slot.is_some() {
                return Err(format!(
                    "strategy digest {} occurs more than once in the combined population",
                    hex(&candidate.strategy_digest.bytes())
                ));
            }
            *slot = Some(SelectedEntryV1::new(
                reference.family,
                reference.population_id,
                row.sequence,
                row.strategy_digest,
                ranked.score,
            ));
        }
        Ok(())
    })?;
    resolved
        .into_iter()
        .enumerate()
        .map(|(rank, entry)| {
            entry.ok_or_else(|| {
                format!(
                    "global selection rank {} has no committed source row",
                    rank + 1
                )
            })
        })
        .collect()
}

fn resolve_selected_v2(
    populations: &mut PopulationLedger,
    references: [PopulationReferenceV2; 2],
    selection: &Selection,
) -> Result<Vec<SelectedEntryV1>, SelectionRefusal> {
    let mut resolved = vec![None; selection.rows.len()];
    visit_both_v2(populations, references, |reference, row, candidate| {
        for (index, ranked) in selection.rows.iter().enumerate() {
            if candidate.strategy_digest != ranked.candidate.strategy_digest {
                continue;
            }
            if candidate != ranked.candidate {
                return Err(format!(
                    "strategy digest {} aliases different V4 candidate bytes across populations",
                    hex(&candidate.strategy_digest.bytes())
                ));
            }
            let slot = resolved
                .get_mut(index)
                .ok_or_else(|| "bounded V2 selected-entry slot is absent".to_owned())?;
            if slot.is_some() {
                return Err(format!(
                    "strategy digest {} occurs more than once in the combined V4 population",
                    hex(&candidate.strategy_digest.bytes())
                ));
            }
            *slot = Some(SelectedEntryV1::new(
                reference.family,
                reference.population_id,
                row.sequence,
                row.strategy_digest,
                ranked.score,
            ));
        }
        Ok(())
    })?;
    resolved
        .into_iter()
        .enumerate()
        .map(|(rank, entry)| {
            entry.ok_or_else(|| {
                format!(
                    "global V2 selection rank {} has no calendar-authoritative source row",
                    rank + 1
                )
            })
        })
        .collect()
}

fn visit_both<F>(
    populations: &mut PopulationLedger,
    references: [PopulationReferenceV1; 2],
    mut visit: F,
) -> Result<(), SelectionRefusal>
where
    F: FnMut(PopulationReferenceV1, PopulationRowV1, Candidate) -> Result<(), SelectionRefusal>,
{
    for reference in references {
        let mut offset = 0_u64;
        while offset < reference.row_count {
            let page = populations
                .page(&reference.population_id, offset, MAX_PAGE_ROWS_V1)?
                .ok_or_else(|| {
                    format!(
                        "committed population {} disappeared while selection streamed it",
                        hex(&reference.population_id)
                    )
                })?;
            if page.total != reference.row_count || page.offset != offset || page.rows.is_empty() {
                return Err(format!(
                    "population {} returned a noncanonical page at offset {offset}",
                    hex(&reference.population_id)
                ));
            }
            for row in page.rows {
                if row.population_id != reference.population_id
                    || row.instrument_family != reference.family
                {
                    return Err(
                        "population page row disagrees with its canonical reference".to_owned()
                    );
                }
                let candidate = candidate_from_row(&row);
                visit(reference, row, candidate)?;
                offset = offset
                    .checked_add(1)
                    .ok_or_else(|| "population selection offset overflowed u64".to_owned())?;
            }
        }
    }
    Ok(())
}

fn visit_both_v2<F>(
    populations: &mut PopulationLedger,
    references: [PopulationReferenceV2; 2],
    mut visit: F,
) -> Result<(), SelectionRefusal>
where
    F: FnMut(PopulationReferenceV2, PopulationRowV1, Candidate) -> Result<(), SelectionRefusal>,
{
    for reference in references {
        let mut offset = 0_u64;
        while offset < reference.row_count {
            let page = populations
                .page_v4(&reference.population_id, offset, MAX_PAGE_ROWS_V1)?
                .ok_or_else(|| {
                    format!(
                        "calendar-authoritative V4 population {} disappeared while selection streamed it",
                        hex(&reference.population_id)
                    )
                })?;
            if page.total != reference.row_count || page.offset != offset || page.rows.is_empty() {
                return Err(format!(
                    "V4 population {} returned a noncanonical page at offset {offset}",
                    hex(&reference.population_id)
                ));
            }
            for row in page.rows {
                if row.population_id != reference.population_id
                    || row.instrument_family != reference.family
                {
                    return Err(
                        "V4 population page row disagrees with its canonical reference".to_owned(),
                    );
                }
                let candidate = candidate_from_row(&row);
                visit(reference, row, candidate)?;
                offset = offset
                    .checked_add(1)
                    .ok_or_else(|| "V4 population selection offset overflowed u64".to_owned())?;
            }
        }
    }
    Ok(())
}

fn candidate_from_row(row: &PopulationRowV1) -> Candidate {
    Candidate {
        strategy_digest: StrategyDigest::new(row.strategy_digest),
        mask_words: row.mask_words,
        direction: match row.direction {
            TradeDirectionV1::Long => Direction::Long,
            TradeDirectionV1::Short => Direction::Short,
        },
        admitted: row.admission.status == AdmissionStatusV1::Admitted,
        metrics: metrics_from_row(row.metrics),
    }
}

const fn metrics_from_row(metrics: TopMetricsV1) -> Metrics {
    Metrics {
        drawdown: metrics.drawdown,
        worst_loss: metrics.worst_loss,
        losing_rate_ppm: metrics.losing_rate_ppm,
        losing_trades: metrics.losing_trades,
        loss_ratio_ppm: metrics.loss_ratio_ppm,
        pessimistic_profit: metrics.pessimistic_profit,
        winning_trades: metrics.winning_trades,
        win_rate_ppm: metrics.win_rate_ppm,
        reward_to_risk_ppm: metrics.reward_to_risk_ppm,
        average_win: metrics.average_win,
        average_loss: metrics.average_loss,
        assurance_ppm: metrics.assurance_ppm,
    }
}

fn write_header(file: &mut File) -> Result<(), SelectionRefusal> {
    let mut header = [0_u8; HEADER_BYTES];
    let mut encoder = Encoder::new(&mut header);
    encoder.bytes(&MAGIC)?;
    encoder.u32(VERSION)?;
    encoder.u32(HEADER_BYTES_U32)?;
    encoder.u32(SELECTION_STRIDE_BYTES_U32)?;
    encoder.u32(REQUESTED_TOP_V1)?;
    encoder.u64(SCORE_SCALE)?;
    encoder.zeros(8)?;
    encoder.finish()?;
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("selection header seek failed: {why}"))?;
    file.write_all(&header)
        .map_err(|why| format!("selection header write failed: {why}"))
}

#[derive(Debug)]
struct SelectionIndexes {
    receipts: HashMap<[u8; 32], SelectionReceiptV1>,
    order: Vec<[u8; 32]>,
    latest: HashMap<([u8; 32], u32), [u8; 32]>,
}

impl SelectionIndexes {
    fn with_capacity(capacity: usize) -> Result<Self, SelectionRefusal> {
        let mut receipts = HashMap::new();
        receipts
            .try_reserve(capacity)
            .map_err(|why| format!("selection index could not reserve {capacity} slots: {why}"))?;
        let mut order = Vec::new();
        order
            .try_reserve_exact(capacity)
            .map_err(|why| format!("selection order could not reserve {capacity} slots: {why}"))?;
        let mut latest = HashMap::new();
        latest.try_reserve(capacity).map_err(|why| {
            format!("selection latest index could not reserve {capacity} slots: {why}")
        })?;
        Ok(Self {
            receipts,
            order,
            latest,
        })
    }

    fn insert(
        &mut self,
        receipt: SelectionReceiptV1,
        location: &str,
    ) -> Result<(), SelectionRefusal> {
        let selection_id = receipt.selection_id;
        if let Some(existing) = self.receipts.get(&selection_id) {
            if existing == &receipt {
                return Err(format!(
                    "{location} repeats selection identity {}",
                    hex(&selection_id)
                ));
            }
            return Err(format!(
                "{location} reuses selection identity {} for different bytes",
                hex(&selection_id)
            ));
        }
        self.latest.insert(receipt.discovery_key(), selection_id);
        self.receipts.insert(selection_id, receipt);
        self.order.push(selection_id);
        Ok(())
    }
}

fn scan_file(
    file: &mut File,
    path: &Path,
    len: u64,
    max_receipts: usize,
) -> Result<SelectionIndexes, SelectionRefusal> {
    validate_file_length(len)?;
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("{} header could not be seeked: {why}", path.display()))?;
    let mut header = [0_u8; HEADER_BYTES];
    file.read_exact(&mut header)
        .map_err(|why| format!("{} header could not be read: {why}", path.display()))?;
    validate_header(&header)?;
    let count = receipt_capacity(len, max_receipts)?;
    scan_receipts(file, path, HEADER, count, None)
}

fn scan_receipts(
    file: &mut File,
    path: &Path,
    start: u64,
    count: usize,
    existing: Option<&HashMap<[u8; 32], SelectionReceiptV1>>,
) -> Result<SelectionIndexes, SelectionRefusal> {
    let mut indexes = SelectionIndexes::with_capacity(count)?;
    for index in 0..count {
        let index_u64 =
            u64::try_from(index).map_err(|_| "selection scan index does not fit u64".to_owned())?;
        let location = index_u64
            .checked_mul(SELECTION_STRIDE)
            .and_then(|offset| start.checked_add(offset))
            .ok_or_else(|| "selection scan byte offset overflowed u64".to_owned())?;
        let mut raw = [0_u8; SELECTION_STRIDE_BYTES];
        file.read_exact(&mut raw).map_err(|why| {
            format!(
                "{} selection record at byte {} could not be read: {why}",
                path.display(),
                location
            )
        })?;
        let receipt = SelectionReceiptV1::from_bytes(&raw).map_err(|why| {
            format!(
                "{} selection record at byte {} is invalid: {why}",
                path.display(),
                location
            )
        })?;
        if let Some(previous) = existing.and_then(|held| held.get(&receipt.selection_id)) {
            if previous == &receipt {
                return Err(format!(
                    "{} repeats selection identity {} at byte {}",
                    path.display(),
                    hex(&receipt.selection_id),
                    location
                ));
            }
            return Err(format!(
                "{} reuses selection identity {} for different bytes at byte {}",
                path.display(),
                hex(&receipt.selection_id),
                location
            ));
        }
        indexes.insert(
            receipt,
            &format!("{} selection record at byte {}", path.display(), location),
        )?;
    }
    Ok(indexes)
}

fn write_header_v2(file: &mut File) -> Result<(), SelectionRefusal> {
    let mut header = [0_u8; HEADER_BYTES];
    let mut encoder = Encoder::new(&mut header);
    encoder.bytes(&MAGIC_V2)?;
    encoder.u32(VERSION_V2)?;
    encoder.u32(HEADER_BYTES_U32)?;
    encoder.u32(SELECTION_V2_STRIDE_BYTES_U32)?;
    encoder.u32(REQUESTED_TOP_V1)?;
    encoder.u64(SCORE_SCALE)?;
    encoder.zeros(8)?;
    encoder.finish()?;
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("selection V2 header seek failed: {why}"))?;
    file.write_all(&header)
        .map_err(|why| format!("selection V2 header write failed: {why}"))
}

#[derive(Debug)]
struct SelectionIndexesV2 {
    receipts: HashMap<[u8; 32], SelectionReceiptV2>,
    order: Vec<[u8; 32]>,
    latest: HashMap<([u8; 32], u32), [u8; 32]>,
}

impl SelectionIndexesV2 {
    fn with_capacity(capacity: usize) -> Result<Self, SelectionRefusal> {
        let mut receipts = HashMap::new();
        receipts.try_reserve(capacity).map_err(|why| {
            format!("selection V2 index could not reserve {capacity} slots: {why}")
        })?;
        let mut order = Vec::new();
        order.try_reserve_exact(capacity).map_err(|why| {
            format!("selection V2 order could not reserve {capacity} slots: {why}")
        })?;
        let mut latest = HashMap::new();
        latest.try_reserve(capacity).map_err(|why| {
            format!("selection V2 latest index could not reserve {capacity} slots: {why}")
        })?;
        Ok(Self {
            receipts,
            order,
            latest,
        })
    }

    fn insert(
        &mut self,
        receipt: SelectionReceiptV2,
        location: &str,
    ) -> Result<(), SelectionRefusal> {
        let selection_id = receipt.selection_id;
        if let Some(existing) = self.receipts.get(&selection_id) {
            if existing == &receipt {
                return Err(format!(
                    "{location} repeats selection V2 identity {}",
                    hex(&selection_id)
                ));
            }
            return Err(format!(
                "{location} reuses selection V2 identity {} for different bytes",
                hex(&selection_id)
            ));
        }
        self.latest.insert(receipt.discovery_key(), selection_id);
        self.receipts.insert(selection_id, receipt);
        self.order.push(selection_id);
        Ok(())
    }
}

fn scan_file_v2(
    file: &mut File,
    path: &Path,
    len: u64,
    max_receipts: usize,
) -> Result<SelectionIndexesV2, SelectionRefusal> {
    validate_file_length_v2(len)?;
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("{} V2 header could not be seeked: {why}", path.display()))?;
    let mut header = [0_u8; HEADER_BYTES];
    file.read_exact(&mut header)
        .map_err(|why| format!("{} V2 header could not be read: {why}", path.display()))?;
    validate_header_v2(&header)?;
    let count = receipt_capacity_v2(len, max_receipts)?;
    scan_receipts_v2(file, path, HEADER, count, None)
}

fn scan_receipts_v2(
    file: &mut File,
    path: &Path,
    start: u64,
    count: usize,
    existing: Option<&HashMap<[u8; 32], SelectionReceiptV2>>,
) -> Result<SelectionIndexesV2, SelectionRefusal> {
    let mut indexes = SelectionIndexesV2::with_capacity(count)?;
    for index in 0..count {
        let index_u64 = u64::try_from(index)
            .map_err(|_| "selection V2 scan index does not fit u64".to_owned())?;
        let location = index_u64
            .checked_mul(SELECTION_V2_STRIDE)
            .and_then(|offset| start.checked_add(offset))
            .ok_or_else(|| "selection V2 scan byte offset overflowed u64".to_owned())?;
        let mut raw = [0_u8; SELECTION_V2_STRIDE_BYTES];
        file.read_exact(&mut raw).map_err(|why| {
            format!(
                "{} selection V2 record at byte {} could not be read: {why}",
                path.display(),
                location
            )
        })?;
        let receipt = SelectionReceiptV2::from_bytes(&raw).map_err(|why| {
            format!(
                "{} selection V2 record at byte {} is invalid: {why}",
                path.display(),
                location
            )
        })?;
        if let Some(previous) = existing.and_then(|held| held.get(&receipt.selection_id)) {
            if previous == &receipt {
                return Err(format!(
                    "{} repeats selection V2 identity {} at byte {}",
                    path.display(),
                    hex(&receipt.selection_id),
                    location
                ));
            }
            return Err(format!(
                "{} reuses selection V2 identity {} for different bytes at byte {}",
                path.display(),
                hex(&receipt.selection_id),
                location
            ));
        }
        indexes.insert(
            receipt,
            &format!(
                "{} selection V2 record at byte {}",
                path.display(),
                location
            ),
        )?;
    }
    Ok(indexes)
}

fn validated_generation(
    file: &File,
    path: &Path,
    expected_len: u64,
) -> Result<FileGeneration, SelectionRefusal> {
    let generation = file_generation(file, path)?;
    if generation.len != expected_len {
        return Err(format!(
            "{} changed length from the just-validated byte {expected_len} to {} before its filesystem generation could be retained; nothing further was appended",
            path.display(),
            generation.len
        ));
    }
    Ok(generation)
}

fn file_generation(file: &File, path: &Path) -> Result<FileGeneration, SelectionRefusal> {
    let held = file
        .metadata()
        .map_err(|why| format!("{} open file could not be measured: {why}", path.display()))?;
    let named = std::fs::metadata(path)
        .map_err(|why| format!("{} path could not be measured: {why}", path.display()))?;
    platform_generation(&held, &named, path)
}

#[cfg(unix)]
fn platform_generation(
    held: &std::fs::Metadata,
    named: &std::fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, SelectionRefusal> {
    let held = FileGeneration {
        len: held.len(),
        platform: PlatformGeneration {
            device: held.dev(),
            inode: held.ino(),
            modified_seconds: held.mtime(),
            modified_nanoseconds: held.mtime_nsec(),
            changed_seconds: held.ctime(),
            changed_nanoseconds: held.ctime_nsec(),
        },
    };
    let named = FileGeneration {
        len: named.len(),
        platform: PlatformGeneration {
            device: named.dev(),
            inode: named.ino(),
            modified_seconds: named.mtime(),
            modified_nanoseconds: named.mtime_nsec(),
            changed_seconds: named.ctime(),
            changed_nanoseconds: named.ctime_nsec(),
        },
    };
    if (held.platform.device, held.platform.inode) != (named.platform.device, named.platform.inode)
    {
        return Err(format!(
            "{} no longer names the opened selection ledger; a replacement or path swap was refused",
            path.display()
        ));
    }
    if held != named {
        return Err(format!(
            "{} changed while its filesystem generation was being measured; cached selections were not reused",
            path.display()
        ));
    }
    Ok(held)
}

#[cfg(windows)]
fn platform_generation(
    held: &std::fs::Metadata,
    named: &std::fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, SelectionRefusal> {
    fn of(metadata: &std::fs::Metadata, path: &Path) -> Result<FileGeneration, SelectionRefusal> {
        let volume_serial = metadata.volume_serial_number().ok_or_else(|| {
            format!(
                "{} has no Windows volume serial; same-length stale detection fails closed",
                path.display()
            )
        })?;
        let file_index = metadata.file_index().ok_or_else(|| {
            format!(
                "{} has no Windows file index; same-length stale detection fails closed",
                path.display()
            )
        })?;
        Ok(FileGeneration {
            len: metadata.len(),
            platform: PlatformGeneration {
                volume_serial,
                file_index,
                creation_time: metadata.creation_time(),
                last_write_time: metadata.last_write_time(),
            },
        })
    }

    let held = of(held, path)?;
    let named = of(named, path)?;
    if (held.platform.volume_serial, held.platform.file_index)
        != (named.platform.volume_serial, named.platform.file_index)
    {
        return Err(format!(
            "{} no longer names the opened selection ledger; a replacement or path swap was refused",
            path.display()
        ));
    }
    if held != named {
        return Err(format!(
            "{} changed while its filesystem generation was being measured; cached selections were not reused",
            path.display()
        ));
    }
    Ok(held)
}

#[cfg(not(any(unix, windows)))]
fn platform_generation(
    held: &std::fs::Metadata,
    named: &std::fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, SelectionRefusal> {
    if held.len() != named.len() {
        return Err(format!(
            "{} changed length while it was being measured; cached selections were not reused",
            path.display()
        ));
    }
    Ok(FileGeneration {
        len: held.len(),
        platform: PlatformGeneration,
    })
}

#[cfg(any(unix, windows))]
fn require_generation_unchanged(
    expected: FileGeneration,
    observed: FileGeneration,
    path: &Path,
) -> Result<(), SelectionRefusal> {
    if expected == observed {
        return Ok(());
    }
    Err(format!(
        "{} kept length {} but its validated filesystem generation changed; cached selections were not reused and nothing was appended. Reopen to validate the complete ledger",
        path.display(),
        observed.len
    ))
}

#[cfg(not(any(unix, windows)))]
fn require_generation_unchanged(
    _expected: FileGeneration,
    observed: FileGeneration,
    path: &Path,
) -> Result<(), SelectionRefusal> {
    Err(format!(
        "{} is unchanged at {} bytes, but this target exposes no stable file identity; append fails closed rather than trusting a possibly replaced same-length ledger",
        path.display(),
        observed.len
    ))
}

fn validate_receipt_limit(max_receipts: usize) -> Result<(), SelectionRefusal> {
    if max_receipts == 0 {
        Err("selection receipt limit must be nonzero".to_owned())
    } else {
        Ok(())
    }
}

fn receipt_capacity(len: u64, max_receipts: usize) -> Result<usize, SelectionRefusal> {
    validate_receipt_limit(max_receipts)?;
    let payload = len
        .checked_sub(HEADER)
        .ok_or_else(|| format!("selection ledger is shorter than its {HEADER}-byte header"))?;
    let count = payload / SELECTION_STRIDE;
    let capacity = usize::try_from(count)
        .map_err(|_| format!("selection receipt count {count} does not fit this machine"))?;
    if capacity > max_receipts {
        return Err(format!(
            "selection ledger contains {capacity} receipt(s), above the caller-supplied maximum {max_receipts}"
        ));
    }
    Ok(capacity)
}

fn receipt_capacity_v2(len: u64, max_receipts: usize) -> Result<usize, SelectionRefusal> {
    validate_receipt_limit(max_receipts)?;
    let payload = len
        .checked_sub(HEADER)
        .ok_or_else(|| format!("selection V2 ledger is shorter than its {HEADER}-byte header"))?;
    let count = payload / SELECTION_V2_STRIDE;
    let capacity = usize::try_from(count)
        .map_err(|_| format!("selection V2 receipt count {count} does not fit this machine"))?;
    if capacity > max_receipts {
        return Err(format!(
            "selection V2 ledger contains {capacity} receipt(s), above the caller-supplied maximum {max_receipts}"
        ));
    }
    Ok(capacity)
}

fn validate_file_length(len: u64) -> Result<(), SelectionRefusal> {
    if len < HEADER {
        return Err(format!(
            "selection ledger is {len} bytes, shorter than its {HEADER}-byte header"
        ));
    }
    let payload = len.saturating_sub(HEADER);
    if !payload.is_multiple_of(SELECTION_STRIDE) {
        return Err(format!(
            "selection ledger has {payload} bytes after its header, not a multiple of stride {SELECTION_STRIDE}"
        ));
    }
    Ok(())
}

fn validate_file_length_v2(len: u64) -> Result<(), SelectionRefusal> {
    if len < HEADER {
        return Err(format!(
            "selection V2 ledger is {len} bytes, shorter than its {HEADER}-byte header"
        ));
    }
    let payload = len.saturating_sub(HEADER);
    if !payload.is_multiple_of(SELECTION_V2_STRIDE) {
        return Err(format!(
            "selection V2 ledger has {payload} bytes after its header, not a multiple of stride {SELECTION_V2_STRIDE}"
        ));
    }
    Ok(())
}

fn validate_header(header: &[u8; HEADER_BYTES]) -> Result<(), SelectionRefusal> {
    let mut decoder = Decoder::new(header);
    if decoder.take(8)? != MAGIC {
        return Err("selection ledger magic is unknown".to_owned());
    }
    if decoder.u32()? != VERSION {
        return Err("selection ledger version is unknown".to_owned());
    }
    if decoder.u32()? != HEADER_BYTES_U32 {
        return Err("selection ledger header length is noncanonical".to_owned());
    }
    if decoder.u32()? != SELECTION_STRIDE_BYTES_U32 {
        return Err("selection ledger stride is noncanonical".to_owned());
    }
    if decoder.u32()? != REQUESTED_TOP_V1 {
        return Err("selection ledger requested Top-N is not exactly 25".to_owned());
    }
    if decoder.u64()? != SCORE_SCALE {
        return Err("selection ledger score scale is noncanonical".to_owned());
    }
    decoder.zeros(8, "ledger header reserve")?;
    decoder.finish()
}

fn validate_header_v2(header: &[u8; HEADER_BYTES]) -> Result<(), SelectionRefusal> {
    let mut decoder = Decoder::new(header);
    if decoder.take(8)? != MAGIC_V2 {
        return Err("selection V2 ledger magic is unknown".to_owned());
    }
    if decoder.u32()? != VERSION_V2 {
        return Err("selection V2 ledger version is unknown".to_owned());
    }
    if decoder.u32()? != HEADER_BYTES_U32 {
        return Err("selection V2 ledger header length is noncanonical".to_owned());
    }
    if decoder.u32()? != SELECTION_V2_STRIDE_BYTES_U32 {
        return Err("selection V2 ledger stride is noncanonical".to_owned());
    }
    if decoder.u32()? != REQUESTED_TOP_V1 {
        return Err("selection V2 ledger requested Top-N is not exactly 25".to_owned());
    }
    if decoder.u64()? != SCORE_SCALE {
        return Err("selection V2 ledger score scale is noncanonical".to_owned());
    }
    decoder.zeros(8, "selection V2 ledger header reserve")?;
    decoder.finish()
}

fn family_byte(family: InstrumentFamilyV1) -> u8 {
    match family {
        InstrumentFamilyV1::Nifty => 1,
        InstrumentFamilyV1::BankNifty => 2,
    }
}

fn decode_family(byte: u8) -> Result<InstrumentFamilyV1, SelectionRefusal> {
    match byte {
        1 => Ok(InstrumentFamilyV1::Nifty),
        2 => Ok(InstrumentFamilyV1::BankNifty),
        _ => Err(format!(
            "selection instrument-family byte {byte} is unknown; V1 defines only 1 and 2"
        )),
    }
}

fn require_digest(name: &str, digest: &[u8; 32]) -> Result<(), SelectionRefusal> {
    if digest.iter().all(|byte| *byte == 0) {
        return Err(format!("selection {name} is the reserved all-zero digest"));
    }
    Ok(())
}

fn checked_add(left: u64, right: u64, name: &str) -> Result<u64, SelectionRefusal> {
    left.checked_add(right)
        .ok_or_else(|| format!("selection {name} overflow u64"))
}

fn checked_sum<const N: usize>(values: [u64; N], name: &str) -> Result<u64, SelectionRefusal> {
    values.into_iter().try_fold(0_u64, |total, value| {
        total
            .checked_add(value)
            .ok_or_else(|| format!("selection {name} overflow u64"))
    })
}

fn read_u32(bytes: &[u8]) -> Result<u32, SelectionRefusal> {
    let array: [u8; 4] = bytes
        .try_into()
        .map_err(|_| "selection u32 field has the wrong width".to_owned())?;
    Ok(u32::from_le_bytes(array))
}

fn read_u64(bytes: &[u8]) -> Result<u64, SelectionRefusal> {
    let array: [u8; 8] = bytes
        .try_into()
        .map_err(|_| "selection u64 field has the wrong width".to_owned())?;
    Ok(u64::from_le_bytes(array))
}

fn hex(bytes: &[u8; 32]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push(char::from(
            DIGITS.get(usize::from(byte >> 4)).copied().unwrap_or(b'?'),
        ));
        out.push(char::from(
            DIGITS
                .get(usize::from(byte & 0x0f))
                .copied()
                .unwrap_or(b'?'),
        ));
    }
    out
}

struct Encoder<'a> {
    output: &'a mut [u8],
    cursor: usize,
}

impl<'a> Encoder<'a> {
    fn new(output: &'a mut [u8]) -> Self {
        Self { output, cursor: 0 }
    }

    fn bytes(&mut self, bytes: &[u8]) -> Result<(), SelectionRefusal> {
        let end = self
            .cursor
            .checked_add(bytes.len())
            .ok_or_else(|| "selection encoder offset overflowed usize".to_owned())?;
        let target = self
            .output
            .get_mut(self.cursor..end)
            .ok_or_else(|| "selection encoder exceeded its fixed record".to_owned())?;
        target.copy_from_slice(bytes);
        self.cursor = end;
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), SelectionRefusal> {
        self.bytes(&[value])
    }

    fn u32(&mut self, value: u32) -> Result<(), SelectionRefusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), SelectionRefusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), SelectionRefusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "selection zero reserve offset overflowed usize".to_owned())?;
        let target = self
            .output
            .get_mut(self.cursor..end)
            .ok_or_else(|| "selection zero reserve exceeded its fixed record".to_owned())?;
        target.fill(0);
        self.cursor = end;
        Ok(())
    }

    fn finish(self) -> Result<(), SelectionRefusal> {
        if self.cursor != self.output.len() {
            return Err(format!(
                "selection encoder wrote {} of {} fixed bytes",
                self.cursor,
                self.output.len()
            ));
        }
        Ok(())
    }
}

struct Decoder<'a> {
    input: &'a [u8],
    cursor: usize,
}

impl<'a> Decoder<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, cursor: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], SelectionRefusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "selection decoder offset overflowed usize".to_owned())?;
        let value = self
            .input
            .get(self.cursor..end)
            .ok_or_else(|| "selection decoder exceeded its fixed record".to_owned())?;
        self.cursor = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, SelectionRefusal> {
        self.take(1)?
            .first()
            .copied()
            .ok_or_else(|| "selection u8 field is absent".to_owned())
    }

    fn u32(&mut self) -> Result<u32, SelectionRefusal> {
        read_u32(self.take(4)?)
    }

    fn u64(&mut self) -> Result<u64, SelectionRefusal> {
        read_u64(self.take(8)?)
    }

    fn array_32(&mut self) -> Result<[u8; 32], SelectionRefusal> {
        self.take(32)?
            .try_into()
            .map_err(|_| "selection digest field has the wrong width".to_owned())
    }

    fn zeros(&mut self, count: usize, name: &str) -> Result<(), SelectionRefusal> {
        if self.take(count)?.iter().any(|byte| *byte != 0) {
            return Err(format!("selection {name} contains nonzero reserved bytes"));
        }
        Ok(())
    }

    fn finish(self) -> Result<(), SelectionRefusal> {
        if self.cursor != self.input.len() {
            return Err(format!(
                "selection decoder consumed {} of {} fixed bytes",
                self.cursor,
                self.input.len()
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::panic,
    reason = "tests must fail loudly when an adversarial fixture is not constructible"
)]
#[allow(
    clippy::large_types_passed_by_value,
    reason = "test builders own independent Copy fixture records so mutations cannot alias"
)]
#[allow(
    clippy::too_many_lines,
    reason = "adversarial codec tests keep each mutation beside its exact refusal assertion"
)]
mod tests {
    use super::{
        PAYLOAD_BYTES, PAYLOAD_BYTES_V2, REQUESTED_TOP_V1, SCORE_SCALE, SELECTION_STRIDE_BYTES,
        SELECTION_V2_STRIDE_BYTES, SelectionIndexes, SelectionLedger, SelectionLedgerV2,
        SelectionReceiptV1, SelectionReceiptV2,
    };
    use crate::population::{
        AdmissionStatusV1, AdmissionV1, ClosureV1, CompletionReceiptV3, CompletionReceiptV4,
        CompletionReconciliationV2, ExitCellsPerMaskV2, ExitCoordinateV1, InstrumentFamilyV1,
        LongShortExitGridIdentitiesV2, PopulationIdentitiesV2, PopulationLedger, PopulationRowV1,
        RequestedSpanIdentityV1, SideExitGridIdentityV2, TopMetricsV1, TradeDirectionV1,
    };
    use crate::stored::{CompleteCalendarReceiptV2, calendar_receipt_v2};
    use pull::calendar::{DayKind, OPEN_MINUTE, kind_of};
    use pull::session::Day;
    use runner::topn::{RankingPolicyV1, Weights};
    use std::fs::OpenOptions;
    use std::io::{Seek as _, SeekFrom, Write as _};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);
    const FIXTURE_LIVE_BITS: [u32; 32] = [
        0, 1, 2, 3, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 20, 21, 22, 23, 24, 26, 27,
        28, 29, 30, 31, 32, 33, 34,
    ];

    fn root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "brutex-selection-{name}-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    const fn digest(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn policy() -> RankingPolicyV1 {
        RankingPolicyV1::new(Weights::equal()).expect("equal policy")
    }

    fn span() -> RequestedSpanIdentityV1 {
        RequestedSpanIdentityV1::new(2020, 1, 2021, 12).expect("canonical fixture span")
    }

    fn v4_span() -> RequestedSpanIdentityV1 {
        RequestedSpanIdentityV1::new(2025, 1, 2025, 1).expect("one measured fixture month")
    }

    fn complete_calendar_v2(
        requested_span: RequestedSpanIdentityV1,
        rung_seconds: u32,
    ) -> CompleteCalendarReceiptV2 {
        let first = Day::new(requested_span.from_year(), requested_span.from_month(), 1)
            .expect("fixture first day");
        let last = Day::new(requested_span.to_year(), requested_span.to_month(), 1)
            .expect("fixture last month")
            .end_of_month();
        let first_day = i64::from(first.days_from_epoch());
        let last_day = i64::from(last.days_from_epoch());
        let width_minutes = i64::from(rung_seconds / 60);
        let mut timestamps = Vec::new();
        for day in first_day..=last_day {
            match kind_of(day) {
                DayKind::Open(session) => {
                    let count = usize::from(session.count);
                    for window in session.windows.iter().take(count) {
                        for minute in window.from..=window.to {
                            let bucket = i64::from(minute).saturating_sub(i64::from(OPEN_MINUTE))
                                / width_minutes;
                            let bucket_minute = i64::from(OPEN_MINUTE)
                                .saturating_add(bucket.saturating_mul(width_minutes));
                            let timestamp = day
                                .saturating_mul(86_400_000_000)
                                .saturating_add(bucket_minute.saturating_mul(60_000_000))
                                .saturating_sub(19_800_000_000);
                            if timestamps.last().copied() != Some(timestamp) {
                                timestamps.push(timestamp);
                            }
                        }
                    }
                }
                DayKind::Closed => {}
                DayKind::OpenLengthUnmeasured | DayKind::Unmeasured => {
                    panic!("fixture month unexpectedly contains unmeasured day {day}")
                }
            }
        }
        calendar_receipt_v2(&timestamps, rung_seconds, first_day, last_day)
            .expect("fixture calendar is measurable")
            .require_complete()
            .expect("fixture calendar is complete")
    }

    const fn identities(seed: u8, ranking_policy_digest: [u8; 32]) -> PopulationIdentitiesV2 {
        PopulationIdentitiesV2 {
            run_identity: digest(seed),
            data_digest: digest(seed.wrapping_add(1)),
            feed_digest: digest(200),
            source_commit_digest: digest(201),
            vocabulary_digest: digest(202),
            evaluation_policy_digest: digest(203),
            exit_grids: LongShortExitGridIdentitiesV2 {
                long: SideExitGridIdentityV2 {
                    policy_digest: digest(seed.wrapping_add(6)),
                    resolved_digest: digest(seed.wrapping_add(7)),
                },
                short: SideExitGridIdentityV2 {
                    policy_digest: digest(seed.wrapping_add(8)),
                    resolved_digest: digest(seed.wrapping_add(9)),
                },
            },
            admission_policy_digest: digest(204),
            ranking_policy_digest,
            calendar_policy_digest: digest(205),
            daily_reference_policy_digest: digest(206),
        }
    }

    const fn admitted() -> AdmissionV1 {
        AdmissionV1 {
            status: AdmissionStatusV1::Admitted,
            reasons: 0,
            failed: 0,
            unmeasured: 0,
            refused: 0,
        }
    }

    const fn rejected() -> AdmissionV1 {
        AdmissionV1 {
            status: AdmissionStatusV1::Rejected,
            reasons: 1,
            failed: 1,
            unmeasured: 0,
            refused: 0,
        }
    }

    fn row(
        population_id: [u8; 32],
        sequence: u64,
        mask: u32,
        family: InstrumentFamilyV1,
        direction: TradeDirectionV1,
        admit: bool,
        rung_seconds: u32,
    ) -> PopulationRowV1 {
        let family_seed = match family {
            InstrumentFamilyV1::Nifty => 40_u8,
            InstrumentFamilyV1::BankNifty => 140_u8,
        };
        let strategy_seed = family_seed.wrapping_add(u8::try_from(sequence).unwrap_or(u8::MAX));
        let wins = sequence % 7 + 1;
        let losses = sequence % 3 + 1;
        let classified = wins + losses;
        let win_rate =
            u64::try_from((u128::from(wins) * u128::from(SCORE_SCALE)) / u128::from(classified))
                .unwrap_or(SCORE_SCALE);
        let losing_rate =
            u64::try_from((u128::from(losses) * u128::from(SCORE_SCALE)) / u128::from(classified))
                .unwrap_or(SCORE_SCALE);
        let live_bit = FIXTURE_LIVE_BITS
            .get(usize::try_from(mask).expect("fixture mask ordinal fits usize"))
            .copied()
            .expect("fixture requests one of its pinned canonical live bits");
        let mut mask_words = [0_u64; 6];
        let word = usize::try_from(live_bit / 64).unwrap_or(0);
        if let Some(slot) = mask_words.get_mut(word) {
            *slot = 1_u64 << (live_bit % 64);
        }
        PopulationRowV1 {
            population_id,
            sequence,
            strategy_digest: digest(strategy_seed),
            mask_words,
            direction,
            instrument_family: family,
            closure: ClosureV1::Closed,
            rung_seconds,
            support_hits: 100_u64.saturating_sub(u64::from(mask)),
            exit: ExitCoordinateV1 {
                stop: Some(0),
                target: Some(2),
                tsl: None,
                ttp: Some((1, 1)),
            },
            metrics: TopMetricsV1 {
                drawdown: 10_000_u64.saturating_sub(sequence),
                worst_loss: 2_000_u64.saturating_sub(sequence.min(1_000)),
                losing_rate_ppm: losing_rate,
                losing_trades: losses,
                loss_ratio_ppm: (!sequence.is_multiple_of(11)).then_some(200_000 + sequence),
                pessimistic_profit: i64::try_from(sequence.saturating_mul(100)).unwrap_or(i64::MAX),
                winning_trades: wins,
                win_rate_ppm: win_rate,
                reward_to_risk_ppm: (!sequence.is_multiple_of(13)).then_some(1_000_000 + sequence),
                average_win: 1_000 + sequence,
                average_loss: 500_u64.saturating_sub(sequence.min(499)),
                assurance_ppm: 600_000 + sequence,
            },
            admission: if admit { admitted() } else { rejected() },
        }
    }

    fn rows(
        population_id: [u8; 32],
        family: InstrumentFamilyV1,
        masks: u32,
        rung_seconds: u32,
    ) -> Vec<PopulationRowV1> {
        let mut rows = Vec::new();
        for mask in 0..masks {
            for direction in [TradeDirectionV1::Long, TradeDirectionV1::Short] {
                let sequence = u64::try_from(rows.len()).unwrap_or(u64::MAX);
                rows.push(row(
                    population_id,
                    sequence,
                    mask,
                    family,
                    direction,
                    !sequence.is_multiple_of(5),
                    rung_seconds,
                ));
            }
        }
        rows
    }

    fn receipt(
        population_id: [u8; 32],
        family: InstrumentFamilyV1,
        rows: &[PopulationRowV1],
        masks: u32,
        rung_seconds: u32,
        ranking_policy_digest: [u8; 32],
        identity_seed: u8,
    ) -> CompletionReceiptV3 {
        receipt_with_identities(
            population_id,
            family,
            rows,
            masks,
            rung_seconds,
            identities(identity_seed, ranking_policy_digest),
        )
    }

    fn receipt_with_identities(
        population_id: [u8; 32],
        family: InstrumentFamilyV1,
        rows: &[PopulationRowV1],
        masks: u32,
        rung_seconds: u32,
        identities: PopulationIdentitiesV2,
    ) -> CompletionReceiptV3 {
        receipt_with_identities_and_span(
            population_id,
            family,
            rows,
            masks,
            rung_seconds,
            identities,
            span(),
        )
    }

    fn receipt_with_identities_and_span(
        population_id: [u8; 32],
        family: InstrumentFamilyV1,
        rows: &[PopulationRowV1],
        masks: u32,
        rung_seconds: u32,
        identities: PopulationIdentitiesV2,
        requested_span: RequestedSpanIdentityV1,
    ) -> CompletionReceiptV3 {
        CompletionReceiptV3::for_rows(
            population_id,
            family,
            rung_seconds,
            rows,
            requested_span,
            CompletionReconciliationV2 {
                sweep_trials: u64::from(masks).saturating_add(3),
                frequent_itemsets: u64::from(masks),
                infrequent_itemsets: 3,
                closed_itemsets: u64::from(masks),
                redundant_itemsets: 0,
                unknown_closure_itemsets: 0,
                exit_cells_per_mask: ExitCellsPerMaskV2::new(1, 1).expect("one cell per side"),
                extinction_depth: if masks == 0 { 1 } else { 2 },
                extinction_complete: true,
                closure_complete: true,
            },
            identities,
        )
        .expect("authoritative population receipt")
    }

    fn reconciliation(masks: u32) -> CompletionReconciliationV2 {
        CompletionReconciliationV2 {
            sweep_trials: u64::from(masks).saturating_add(3),
            frequent_itemsets: u64::from(masks),
            infrequent_itemsets: 3,
            closed_itemsets: u64::from(masks),
            redundant_itemsets: 0,
            unknown_closure_itemsets: 0,
            exit_cells_per_mask: ExitCellsPerMaskV2::new(1, 1).expect("one fixture cell per side"),
            extinction_depth: if masks == 0 { 1 } else { 2 },
            extinction_complete: true,
            closure_complete: true,
        }
    }

    fn v4_identities(
        seed: u8,
        ranking_policy_digest: [u8; 32],
        long_policy_digest: [u8; 32],
        short_policy_digest: [u8; 32],
    ) -> PopulationIdentitiesV2 {
        let mut value = identities(seed, ranking_policy_digest);
        value.exit_grids.long.policy_digest = long_policy_digest;
        value.exit_grids.short.policy_digest = short_policy_digest;
        value
    }

    fn committed_pair_v4(
        root: &std::path::Path,
        masks: u32,
        rung_seconds: u32,
        ranking_policy_digest: [u8; 32],
    ) -> (PopulationLedger, [u8; 32], [u8; 32]) {
        committed_pair_v4_with_policies(
            root,
            masks,
            rung_seconds,
            ranking_policy_digest,
            digest(220),
            digest(220),
            digest(221),
            digest(221),
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the fixture varies each same-side policy independently"
    )]
    fn committed_pair_v4_with_policies(
        root: &std::path::Path,
        masks: u32,
        rung_seconds: u32,
        ranking_policy_digest: [u8; 32],
        nifty_long_policy: [u8; 32],
        bank_long_policy: [u8; 32],
        nifty_short_policy: [u8; 32],
        bank_short_policy: [u8; 32],
    ) -> (PopulationLedger, [u8; 32], [u8; 32]) {
        let requested_span = v4_span();
        let signal_calendar = complete_calendar_v2(requested_span, rung_seconds);
        let execution_calendar = complete_calendar_v2(requested_span, 60);
        let nifty_id = digest(61);
        let bank_id = digest(62);
        let nifty_rows = rows(nifty_id, InstrumentFamilyV1::Nifty, masks, rung_seconds);
        let bank_rows = rows(bank_id, InstrumentFamilyV1::BankNifty, masks, rung_seconds);
        let nifty_receipt = CompletionReceiptV4::for_rows(
            nifty_id,
            InstrumentFamilyV1::Nifty,
            rung_seconds,
            &nifty_rows,
            requested_span,
            reconciliation(masks),
            v4_identities(
                70,
                ranking_policy_digest,
                nifty_long_policy,
                nifty_short_policy,
            ),
            signal_calendar,
            execution_calendar,
        )
        .expect("calendar-authoritative NIFTY receipt");
        let bank_receipt = CompletionReceiptV4::for_rows(
            bank_id,
            InstrumentFamilyV1::BankNifty,
            rung_seconds,
            &bank_rows,
            requested_span,
            reconciliation(masks),
            v4_identities(
                90,
                ranking_policy_digest,
                bank_long_policy,
                bank_short_policy,
            ),
            signal_calendar,
            execution_calendar,
        )
        .expect("calendar-authoritative BANKNIFTY receipt");
        let mut ledger = PopulationLedger::open(root).expect("population V4 ledger");
        ledger
            .append_complete_v4(&nifty_rows, &nifty_receipt)
            .expect("NIFTY V4 commit");
        ledger
            .append_complete_v4(&bank_rows, &bank_receipt)
            .expect("BANKNIFTY V4 commit");
        (ledger, nifty_id, bank_id)
    }

    fn selection_fixture_v2(
        root: &std::path::Path,
        masks: u32,
        rung_seconds: u32,
        ranking: RankingPolicyV1,
    ) -> SelectionReceiptV2 {
        let (mut populations, nifty_id, bank_id) =
            committed_pair_v4(root, masks, rung_seconds, ranking.digest());
        SelectionReceiptV2::from_committed_populations(&mut populations, nifty_id, bank_id, ranking)
            .expect("selection V2 fixture")
    }

    fn committed_pair(
        root: &std::path::Path,
        masks: u32,
        rung_seconds: u32,
        ranking_policy_digest: [u8; 32],
    ) -> (PopulationLedger, [u8; 32], [u8; 32]) {
        committed_pair_variant(
            root,
            masks,
            rung_seconds,
            ranking_policy_digest,
            PairVariant {
                nifty_id: digest(1),
                bank_id: digest(2),
                nifty_identity_seed: 10,
                bank_identity_seed: 30,
                requested_span: span(),
            },
        )
    }

    #[derive(Clone, Copy)]
    struct PairVariant {
        nifty_id: [u8; 32],
        bank_id: [u8; 32],
        nifty_identity_seed: u8,
        bank_identity_seed: u8,
        requested_span: RequestedSpanIdentityV1,
    }

    fn pair_variant(
        id_seed: u8,
        identity_seed: u8,
        requested_span: RequestedSpanIdentityV1,
    ) -> PairVariant {
        PairVariant {
            nifty_id: digest(id_seed),
            bank_id: digest(id_seed.wrapping_add(1)),
            nifty_identity_seed: identity_seed,
            bank_identity_seed: identity_seed.wrapping_add(20),
            requested_span,
        }
    }

    fn committed_pair_variant(
        root: &std::path::Path,
        masks: u32,
        rung_seconds: u32,
        ranking_policy_digest: [u8; 32],
        variant: PairVariant,
    ) -> (PopulationLedger, [u8; 32], [u8; 32]) {
        let PairVariant {
            nifty_id,
            bank_id,
            nifty_identity_seed,
            bank_identity_seed,
            requested_span,
        } = variant;
        let nifty_rows = rows(nifty_id, InstrumentFamilyV1::Nifty, masks, rung_seconds);
        let bank_rows = rows(bank_id, InstrumentFamilyV1::BankNifty, masks, rung_seconds);
        let nifty_receipt = receipt_with_identities_and_span(
            nifty_id,
            InstrumentFamilyV1::Nifty,
            &nifty_rows,
            masks,
            rung_seconds,
            identities(nifty_identity_seed, ranking_policy_digest),
            requested_span,
        );
        let bank_receipt = receipt_with_identities_and_span(
            bank_id,
            InstrumentFamilyV1::BankNifty,
            &bank_rows,
            masks,
            rung_seconds,
            identities(bank_identity_seed, ranking_policy_digest),
            requested_span,
        );
        let mut ledger = PopulationLedger::open(root).expect("population ledger");
        ledger
            .append_complete_v3(&nifty_rows, &nifty_receipt)
            .expect("NIFTY commit");
        ledger
            .append_complete_v3(&bank_rows, &bank_receipt)
            .expect("BANKNIFTY commit");
        (ledger, nifty_id, bank_id)
    }

    fn selection_fixture(
        root: &std::path::Path,
        masks: u32,
        rung_seconds: u32,
        ranking: RankingPolicyV1,
        variant: PairVariant,
    ) -> SelectionReceiptV1 {
        let (mut populations, nifty_id, bank_id) =
            committed_pair_variant(root, masks, rung_seconds, ranking.digest(), variant);
        SelectionReceiptV1::from_committed_populations(&mut populations, nifty_id, bank_id, ranking)
            .expect("selection fixture")
    }

    fn reseal(raw: &mut [u8; SELECTION_STRIDE_BYTES], update_id: bool) {
        if update_id {
            let mut hasher = brutex_core::blake3::Hasher::new();
            hasher.update(b"brutex-global-selection-id-v1\0");
            hasher.update(&raw[32..PAYLOAD_BYTES]);
            raw[..32].copy_from_slice(&hasher.finalize());
        }
        let seal = brutex_core::blake3::hash(&raw[..PAYLOAD_BYTES]);
        raw[PAYLOAD_BYTES..].copy_from_slice(&seal);
    }

    fn reseal_v2(raw: &mut [u8; SELECTION_V2_STRIDE_BYTES], update_id: bool) {
        if update_id {
            let mut hasher = brutex_core::blake3::Hasher::new();
            hasher.update(b"brutex-global-selection-id-v2\0");
            hasher.update(&raw[32..PAYLOAD_BYTES_V2]);
            raw[..32].copy_from_slice(&hasher.finalize());
        }
        let seal = brutex_core::blake3::hash(&raw[..PAYLOAD_BYTES_V2]);
        raw[PAYLOAD_BYTES_V2..].copy_from_slice(&seal);
    }

    #[test]
    fn calendar_authoritative_v2_round_trips_persists_and_top_ten_is_exact_prefix() {
        let root = root("v2-round-trip");
        let ranking = policy();
        let (mut populations, nifty_id, bank_id) =
            committed_pair_v4(&root, 20, 300, ranking.digest());
        let receipt = SelectionReceiptV2::from_committed_populations(
            &mut populations,
            nifty_id,
            bank_id,
            ranking,
        )
        .expect("derived calendar-authoritative selection");
        assert_eq!(receipt.populations()[0].family(), InstrumentFamilyV1::Nifty);
        assert_eq!(
            receipt.populations()[1].family(),
            InstrumentFamilyV1::BankNifty
        );
        assert_eq!(receipt.top_twenty_five().len(), 25);
        assert_eq!(receipt.top_ten(), &receipt.top_twenty_five()[..10]);
        assert!(
            receipt
                .top_twenty_five()
                .windows(2)
                .all(|pair| pair[0].score() >= pair[1].score())
        );
        receipt
            .verify_against_populations(&mut populations, ranking)
            .expect("full V4 replay");

        let raw = receipt.to_bytes().expect("V2 codec");
        assert_eq!(
            SelectionReceiptV2::from_bytes(&raw).expect("V2 decode"),
            receipt
        );

        let mut ledger = SelectionLedgerV2::open(&root, 2).expect("V2 ledger");
        ledger.append(&receipt).expect("one durable V2 append");
        assert_eq!(ledger.selections(), 1);
        assert_eq!(ledger.selection_ids(), &[receipt.selection_id()]);
        assert_eq!(ledger.receipt(&receipt.selection_id()), Some(&receipt));
        assert_eq!(
            ledger.latest_selection(receipt.cohort_identity().digest(), 300),
            Some(&receipt)
        );
        assert!(
            ledger
                .latest_selection(receipt.cohort_identity().digest(), 600)
                .is_none()
        );
        let why = ledger
            .append(&receipt)
            .expect_err("exact duplicate V2 identity is not another append");
        assert!(why.contains("already present"), "{why}");
        drop(ledger);

        let reopened = SelectionLedgerV2::open_read(&root, 2).expect("reopened V2 ledger");
        assert_eq!(reopened.receipt(&receipt.selection_id()), Some(&receipt));
        assert_eq!(
            reopened.latest_selection(receipt.cohort_identity().digest(), 300),
            Some(&receipt)
        );
    }

    #[test]
    fn v3_only_populations_can_never_form_selection_v2() {
        let root = root("v3-is-audit-only-for-v2");
        let ranking = policy();
        let (mut populations, nifty_id, bank_id) = committed_pair(&root, 2, 300, ranking.digest());
        let why = SelectionReceiptV2::from_committed_populations(
            &mut populations,
            nifty_id,
            bank_id,
            ranking,
        )
        .expect_err("V3 lacks actual calendar authority");
        assert!(
            why.contains("not an authoritative calendar-bound V4"),
            "{why}"
        );
    }

    #[test]
    fn selection_v2_requires_exact_same_side_policies_not_crossed_or_resolved_equality() {
        let ranking = policy();
        for (name, policies, expected) in [
            (
                "long",
                (digest(10), digest(11), digest(20), digest(20)),
                "different long exit-policy",
            ),
            (
                "short",
                (digest(10), digest(10), digest(20), digest(21)),
                "different short exit-policy",
            ),
            (
                "crossed",
                (digest(10), digest(20), digest(20), digest(10)),
                "different long exit-policy",
            ),
        ] {
            let root = root(name);
            let (nifty_long, bank_long, nifty_short, bank_short) = policies;
            let (mut populations, nifty_id, bank_id) = committed_pair_v4_with_policies(
                &root,
                2,
                300,
                ranking.digest(),
                nifty_long,
                bank_long,
                nifty_short,
                bank_short,
            );
            let why = SelectionReceiptV2::from_committed_populations(
                &mut populations,
                nifty_id,
                bank_id,
                ranking,
            )
            .expect_err("same-side mismatch must fail before ranking");
            assert!(why.contains(expected), "{name}: {why}");
        }

        let root = root("resolved-digests-may-differ");
        let (mut populations, nifty_id, bank_id) =
            committed_pair_v4(&root, 2, 300, ranking.digest());
        let nifty = populations.receipt_v4(&nifty_id).expect("NIFTY V4");
        let bank = populations.receipt_v4(&bank_id).expect("BANKNIFTY V4");
        assert_ne!(
            nifty.v3().v2().identities.exit_grids.long.resolved_digest,
            bank.v3().v2().identities.exit_grids.long.resolved_digest
        );
        assert_ne!(
            nifty.v3().v2().identities.exit_grids.short.resolved_digest,
            bank.v3().v2().identities.exit_grids.short.resolved_digest
        );
        SelectionReceiptV2::from_committed_populations(
            &mut populations,
            nifty_id,
            bank_id,
            ranking,
        )
        .expect("instrument-specific resolved digests are intentionally different");
    }

    #[test]
    fn selection_v2_refuses_each_independently_persisted_coverage_mismatch() {
        const POPULATION_HEADER_BYTES: usize = 16;
        const POPULATION_V4_PAYLOAD_BYTES: usize = 856;
        const POPULATION_V4_SEAL_BYTES: usize = 8;
        const POPULATION_V3_PAYLOAD_BYTES: usize = 760;
        const COVERAGE_SIGNAL_DIGEST_OFFSET: usize = POPULATION_V3_PAYLOAD_BYTES + 32;
        const COVERAGE_EXECUTION_DIGEST_OFFSET: usize = POPULATION_V3_PAYLOAD_BYTES + 64;

        let ranking = policy();
        for (name, coverage_offset, expected) in [
            (
                "signal-coverage",
                COVERAGE_SIGNAL_DIGEST_OFFSET,
                "different complete signal-calendar coverage",
            ),
            (
                "execution-coverage",
                COVERAGE_EXECUTION_DIGEST_OFFSET,
                "different complete one-minute execution-calendar coverage",
            ),
        ] {
            let root = root(name);
            let (populations, nifty_id, bank_id) =
                committed_pair_v4(&root, 2, 300, ranking.digest());
            drop(populations);

            let path = PopulationLedger::receipt_v4_path(&root);
            let mut bytes = std::fs::read(&path).expect("two V4 population receipts");
            let bank_record = POPULATION_HEADER_BYTES + crate::population::RECEIPT_V4_STRIDE_BYTES;
            let changed = bank_record + coverage_offset;
            bytes[changed] ^= 1;
            let payload_end = bank_record + POPULATION_V4_PAYLOAD_BYTES;
            let seal = brutex_core::blake3::hash(&bytes[bank_record..payload_end]);
            bytes[payload_end..payload_end + POPULATION_V4_SEAL_BYTES]
                .copy_from_slice(&seal[..POPULATION_V4_SEAL_BYTES]);
            std::fs::write(&path, bytes).expect("resealed V4 coverage adversary");

            let mut reopened = PopulationLedger::open(&root).expect("resealed V4 ledger reopens");
            let why = SelectionReceiptV2::from_committed_populations(
                &mut reopened,
                nifty_id,
                bank_id,
                ranking,
            )
            .expect_err("cross-instrument coverage mismatch fails before ranking");
            assert!(why.contains(expected), "{name}: {why}");
        }
    }

    #[test]
    fn sealed_v2_selection_corruption_and_reserved_bytes_fail_closed() {
        let root = root("v2-corruption");
        let receipt = selection_fixture_v2(&root, 2, 300, policy());
        let raw = receipt.to_bytes().expect("canonical V2 bytes");

        let mut bad_seal = raw;
        bad_seal[PAYLOAD_BYTES_V2] ^= 1;
        let why =
            SelectionReceiptV2::from_bytes(&bad_seal).expect_err("V2 seal mutation cannot decode");
        assert!(why.contains("BLAKE3 seal"), "{why}");

        let mut reserve = raw;
        reserve[804] = 1;
        reseal_v2(&mut reserve, true);
        let why =
            SelectionReceiptV2::from_bytes(&reserve).expect_err("V2 count reserve is load-bearing");
        assert!(why.contains("count reserve"), "{why}");

        let mut trailing = raw;
        trailing[3_008] = 1;
        reseal_v2(&mut trailing, true);
        let why = SelectionReceiptV2::from_bytes(&trailing)
            .expect_err("V2 trailing reserve is load-bearing");
        assert!(why.contains("trailing reserve"), "{why}");
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn selection_v2_stale_same_length_prefix_refuses_before_append() {
        let root = root("v2-stale-generation");
        let ranking = policy();
        let first = selection_fixture_v2(&root.join("population-first"), 2, 300, ranking);
        let second = selection_fixture_v2(&root.join("population-second"), 2, 600, ranking);
        let mut ledger = SelectionLedgerV2::open(&root, 2).expect("V2 writer");
        ledger.append(&first).expect("first V2 append");
        let path = SelectionLedgerV2::path(&root);
        let before = std::fs::metadata(&path).expect("V2 metadata").len();
        let header = usize::try_from(super::HEADER).expect("V2 header fits usize");
        let changed = std::fs::read(&path).expect("V2 bytes")[header] ^ 1;
        let mut external = OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("external V2 writer");
        external
            .seek(SeekFrom::Start(super::HEADER))
            .expect("external V2 seek");
        external
            .write_all(&[changed])
            .expect("same-length V2 mutation");
        external.sync_all().expect("V2 mutation sync");
        drop(external);

        let why = ledger
            .append(&second)
            .expect_err("cached V2 prefix must detect generation change");
        assert!(why.contains("filesystem generation changed"), "{why}");
        assert_eq!(std::fs::metadata(&path).expect("V2 metadata").len(), before);
    }

    #[test]
    fn selection_v2_never_falls_back_to_v1_and_refuses_bad_header_or_ragged_tail() {
        let v1_only = root("v2-no-v1-fallback");
        drop(SelectionLedger::open(&v1_only, 1).expect("V1 audit ledger"));
        let why =
            SelectionLedgerV2::open_read(&v1_only, 1).expect_err("V1 must never satisfy a V2 open");
        assert!(why.contains("global-selections-v2.bin"), "{why}");

        let bad_header = root("v2-bad-header");
        drop(SelectionLedgerV2::open(&bad_header, 1).expect("empty V2 ledger"));
        let path = SelectionLedgerV2::path(&bad_header);
        let mut file = OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("V2 header writer");
        file.seek(SeekFrom::Start(32)).expect("V2 reserve seek");
        file.write_all(&[1]).expect("V2 reserve mutation");
        file.sync_all().expect("V2 reserve sync");
        drop(file);
        let why = SelectionLedgerV2::open_read(&bad_header, 1)
            .expect_err("nonzero V2 header reserve refuses");
        assert!(why.contains("header reserve"), "{why}");

        let ragged = root("v2-ragged-tail");
        drop(SelectionLedgerV2::open(&ragged, 1).expect("empty V2 ledger"));
        let path = SelectionLedgerV2::path(&ragged);
        let mut file = OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("V2 tail writer");
        file.write_all(&[1]).expect("one ragged byte");
        file.sync_all().expect("ragged V2 sync");
        drop(file);
        let why = SelectionLedgerV2::open_read(&ragged, 1).expect_err("ragged V2 tail refuses");
        assert!(why.contains("not a multiple of stride"), "{why}");
    }

    #[test]
    fn committed_global_top_twenty_five_round_trips_and_top_ten_is_its_prefix() {
        let root = root("round-trip");
        let ranking = policy();
        let (mut populations, nifty_id, bank_id) = committed_pair(&root, 20, 300, ranking.digest());
        let receipt = SelectionReceiptV1::from_committed_populations(
            &mut populations,
            nifty_id,
            bank_id,
            ranking,
        )
        .expect("derived global selection");
        assert_eq!(receipt.populations()[0].family(), InstrumentFamilyV1::Nifty);
        assert_eq!(
            receipt.populations()[1].family(),
            InstrumentFamilyV1::BankNifty
        );
        assert_eq!(
            receipt.populations()[0].completion_digest(),
            populations
                .receipt_v3(&nifty_id)
                .expect("authoritative NIFTY")
                .content_digest()
                .expect("NIFTY content digest")
        );
        assert_eq!(receipt.top_twenty_five().len(), 25);
        assert_eq!(receipt.top_ten(), &receipt.top_twenty_five()[..10]);
        assert!(
            receipt
                .top_twenty_five()
                .windows(2)
                .all(|pair| pair[0].score() >= pair[1].score())
        );
        receipt
            .verify_against_populations(&mut populations, ranking)
            .expect("full replay");

        let id = receipt.selection_id();
        let mut ledger = SelectionLedger::open(&root, 1).expect("selection ledger");
        ledger.append(&receipt).expect("selection append");
        assert!(
            ledger
                .append(&receipt)
                .unwrap_err()
                .contains("already present")
        );
        drop(ledger);
        let reopened = SelectionLedger::open_read(&root, 1).expect("selection reopen");
        assert_eq!(reopened.selections(), 1);
        assert_eq!(reopened.receipt(&id), Some(&receipt));
        drop(reopened);
        drop(populations);
        std::fs::remove_dir_all(root).expect("fixture cleanup");
    }

    #[test]
    fn receipt_limit_is_nonzero_exact_and_refuses_one_over_before_indexing() {
        let zero_root = root("zero-limit");
        assert!(
            SelectionLedger::open(&zero_root, 0)
                .unwrap_err()
                .contains("must be nonzero")
        );
        assert!(
            SelectionLedger::open_read(&zero_root, 0)
                .unwrap_err()
                .contains("must be nonzero")
        );
        assert!(!SelectionLedger::path(&zero_root).exists());

        let missing_root = root("missing-read-only");
        assert!(SelectionLedger::open_read(&missing_root, 1).is_err());
        assert!(!SelectionLedger::path(&missing_root).exists());

        let root = root("bounded-open");
        let ranking = policy();
        let first = selection_fixture(
            &root.join("population-first"),
            2,
            300,
            ranking,
            pair_variant(11, 41, span()),
        );
        let second = selection_fixture(
            &root.join("population-second"),
            2,
            300,
            ranking,
            pair_variant(13, 43, span()),
        );
        let third = selection_fixture(
            &root.join("population-third"),
            2,
            300,
            ranking,
            pair_variant(15, 45, span()),
        );
        let mut writer = SelectionLedger::open(&root, 2).expect("bounded writer");
        writer.append(&first).expect("first bounded append");
        writer.append(&second).expect("exact-bound append");
        assert_eq!(writer.selections(), 2);
        assert!(
            writer
                .append(&third)
                .unwrap_err()
                .contains("caller-supplied maximum")
        );
        drop(writer);

        let exact = SelectionLedger::open_read(&root, 2).expect("exact-bound read");
        assert_eq!(
            exact.selection_ids(),
            &[first.selection_id(), second.selection_id()]
        );
        drop(exact);
        assert!(
            SelectionLedger::open_read(&root, 1)
                .unwrap_err()
                .contains("above the caller-supplied maximum 1")
        );

        std::fs::remove_dir_all(root).expect("bounded fixture cleanup");
    }

    #[test]
    fn file_order_survives_stale_absorb_and_reopen_and_read_only_stays_read_only() {
        let root = root("ordered-stale");
        let ranking = policy();
        let receipts: Vec<SelectionReceiptV1> = [
            ("population-one", 21_u8, 51_u8),
            ("population-two", 23_u8, 53_u8),
            ("population-three", 25_u8, 55_u8),
        ]
        .into_iter()
        .map(|(name, id_seed, identity_seed)| {
            selection_fixture(
                &root.join(name),
                2,
                300,
                ranking,
                pair_variant(id_seed, identity_seed, span()),
            )
        })
        .collect();
        let mut first = SelectionLedger::open(&root, 3).expect("first writer");
        let mut stale = SelectionLedger::open(&root, 3).expect("stale writer");
        first.append(&receipts[0]).expect("first append");
        stale
            .append(&receipts[1])
            .expect("stale writer absorbs first");
        first
            .append(&receipts[2])
            .expect("first writer absorbs stale append");
        let expected = [
            receipts[0].selection_id(),
            receipts[1].selection_id(),
            receipts[2].selection_id(),
        ];
        assert_eq!(first.selection_ids(), &expected);
        drop(first);
        drop(stale);

        let mut reopened = SelectionLedger::open_read(&root, 3).expect("ordered reopen");
        assert_eq!(reopened.selection_ids(), &expected);
        assert!(
            reopened
                .append(&receipts[0])
                .unwrap_err()
                .contains("read-only")
        );
        drop(reopened);
        std::fs::remove_dir_all(root).expect("ordered fixture cleanup");
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn same_length_mutation_refuses_cached_prefix_before_selection_append() {
        let root = root("same-length-mutation");
        let ranking = policy();
        let first = selection_fixture(
            &root.join("population-first"),
            2,
            300,
            ranking,
            pair_variant(27, 57, span()),
        );
        let second = selection_fixture(
            &root.join("population-second"),
            2,
            600,
            ranking,
            pair_variant(29, 59, span()),
        );
        let mut ledger = SelectionLedger::open(&root, 2).expect("selection writer");
        ledger.append(&first).expect("first selection");
        let path = SelectionLedger::path(&root);
        let before = std::fs::metadata(&path).expect("selection metadata").len();
        let original = std::fs::read(&path).expect("selection bytes");
        let header = usize::try_from(super::HEADER).expect("selection header fits usize");
        let changed = original.get(header).copied().expect("first receipt byte") ^ 1;
        let mut external = OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("external selection writer");
        external
            .seek(SeekFrom::Start(super::HEADER))
            .expect("external selection seek");
        external
            .write_all(&[changed])
            .expect("same-length selection mutation");
        external.sync_all().expect("selection mutation sync");
        drop(external);

        let why = ledger
            .append(&second)
            .expect_err("cached same-length prefix must be refused");
        assert!(why.contains("filesystem generation changed"), "{why}");
        assert_eq!(
            std::fs::metadata(&path).expect("selection metadata").len(),
            before
        );
        drop(ledger);
        std::fs::remove_dir_all(root).expect("selection fixture cleanup");
    }

    #[test]
    fn latest_lookup_is_exactly_isolated_by_cohort_and_rung() {
        let root = root("latest-isolation");
        let ranking = policy();
        let other_span =
            RequestedSpanIdentityV1::new(2022, 1, 2023, 12).expect("other fixture span");
        let first_300 = selection_fixture(
            &root.join("population-first-300"),
            2,
            300,
            ranking,
            pair_variant(31, 61, span()),
        );
        let only_600 = selection_fixture(
            &root.join("population-only-600"),
            2,
            600,
            ranking,
            pair_variant(33, 63, span()),
        );
        let other_cohort_300 = selection_fixture(
            &root.join("population-other-cohort"),
            2,
            300,
            ranking,
            pair_variant(35, 65, other_span),
        );
        let latest_300 = selection_fixture(
            &root.join("population-latest-300"),
            2,
            300,
            ranking,
            pair_variant(37, 67, span()),
        );
        let shared = first_300.cohort_identity().digest();
        let other = other_cohort_300.cohort_identity().digest();
        assert_eq!(shared, latest_300.cohort_identity().digest());
        assert_eq!(shared, only_600.cohort_identity().digest());
        assert_ne!(shared, other);

        let mut ledger = SelectionLedger::open(&root, 4).expect("latest ledger");
        for receipt in [&first_300, &only_600, &other_cohort_300, &latest_300] {
            ledger.append(receipt).expect("latest fixture append");
        }
        assert_eq!(ledger.latest_selection(shared, 300), Some(&latest_300));
        assert_eq!(ledger.latest_selection(shared, 600), Some(&only_600));
        assert_eq!(ledger.latest_selection(other, 300), Some(&other_cohort_300));
        assert_eq!(ledger.latest_selection(shared, 900), None);
        assert_eq!(ledger.latest_selection(digest(250), 300), None);
        drop(ledger);

        let reopened = SelectionLedger::open_read(&root, 4).expect("latest reopen");
        assert_eq!(reopened.latest_selection(shared, 300), Some(&latest_300));
        assert_eq!(reopened.latest_selection(shared, 600), Some(&only_600));
        assert_eq!(
            reopened.latest_selection(other, 300),
            Some(&other_cohort_300)
        );
        drop(reopened);
        std::fs::remove_dir_all(root).expect("latest fixture cleanup");
    }

    #[test]
    fn an_empty_list_is_canonical_only_when_both_populations_admit_nothing() {
        let root = root("empty");
        let ranking = policy();
        let (mut populations, nifty_id, bank_id) = committed_pair(&root, 0, 60, ranking.digest());
        let receipt = SelectionReceiptV1::from_committed_populations(
            &mut populations,
            nifty_id,
            bank_id,
            ranking,
        )
        .expect("honest empty selection");
        assert_eq!(receipt.population_proof().considered, 0);
        assert_eq!(receipt.population_proof().admitted, 0);
        assert!(receipt.top_twenty_five().is_empty());
        let mut nonzero_unused = receipt.to_bytes().expect("empty receipt bytes");
        nonzero_unused[680] = 1;
        reseal(&mut nonzero_unused, false);
        assert!(
            SelectionReceiptV1::from_bytes(&nonzero_unused)
                .unwrap_err()
                .contains("unused selected-entry slots")
        );

        let root_nonempty = self::root("false-empty");
        let (mut nonempty_populations, nifty_id, bank_id) =
            committed_pair(&root_nonempty, 2, 60, ranking.digest());
        let mut nonempty = SelectionReceiptV1::from_committed_populations(
            &mut nonempty_populations,
            nifty_id,
            bank_id,
            ranking,
        )
        .expect("nonempty selection");
        nonempty.selected.clear();
        nonempty.selection_id = nonempty.derived_id().expect("derived false-empty id");
        assert!(
            nonempty
                .validate_semantics()
                .unwrap_err()
                .contains("requires exactly")
        );

        drop(populations);
        drop(nonempty_populations);
        std::fs::remove_dir_all(root).expect("empty cleanup");
        std::fs::remove_dir_all(root_nonempty).expect("nonempty cleanup");
    }

    #[test]
    fn swapped_families_different_rungs_and_policy_mismatches_refuse() {
        let root = root("binding-refusals");
        let ranking = policy();
        let (mut populations, nifty_id, bank_id) = committed_pair(&root, 2, 300, ranking.digest());
        assert!(
            SelectionReceiptV1::from_committed_populations(
                &mut populations,
                bank_id,
                nifty_id,
                ranking
            )
            .unwrap_err()
            .contains("expected Nifty")
        );
        let other = RankingPolicyV1::new(Weights {
            drawdown: 9,
            ..Weights::equal()
        })
        .expect("other policy");
        assert!(
            SelectionReceiptV1::from_committed_populations(
                &mut populations,
                nifty_id,
                bank_id,
                other
            )
            .unwrap_err()
            .contains("ranking-policy")
        );

        let mixed_root = self::root("mixed-rungs");
        let nifty_rows = rows(nifty_id, InstrumentFamilyV1::Nifty, 1, 60);
        let bank_rows = rows(bank_id, InstrumentFamilyV1::BankNifty, 1, 120);
        let mut mixed = PopulationLedger::open(&mixed_root).expect("mixed ledger");
        mixed
            .append_complete_v3(
                &nifty_rows,
                &receipt(
                    nifty_id,
                    InstrumentFamilyV1::Nifty,
                    &nifty_rows,
                    1,
                    60,
                    ranking.digest(),
                    10,
                ),
            )
            .expect("mixed NIFTY");
        mixed
            .append_complete_v3(
                &bank_rows,
                &receipt(
                    bank_id,
                    InstrumentFamilyV1::BankNifty,
                    &bank_rows,
                    1,
                    120,
                    ranking.digest(),
                    30,
                ),
            )
            .expect("mixed BANKNIFTY");
        assert!(
            SelectionReceiptV1::from_committed_populations(&mut mixed, nifty_id, bank_id, ranking)
                .unwrap_err()
                .contains("one timeframe")
        );
        drop(populations);
        drop(mixed);
        std::fs::remove_dir_all(root).expect("binding cleanup");
        std::fs::remove_dir_all(mixed_root).expect("mixed cleanup");
    }

    #[test]
    fn legacy_v2_or_different_requested_spans_can_never_form_one_cohort() {
        let ranking = policy();
        let nifty_id = digest(1);
        let bank_id = digest(2);
        let nifty_rows = rows(nifty_id, InstrumentFamilyV1::Nifty, 1, 300);
        let bank_rows = rows(bank_id, InstrumentFamilyV1::BankNifty, 1, 300);

        let legacy_root = root("legacy-v2-only");
        let mut legacy = PopulationLedger::open(&legacy_root).expect("legacy ledger");
        legacy
            .append_complete(
                &nifty_rows,
                &receipt(
                    nifty_id,
                    InstrumentFamilyV1::Nifty,
                    &nifty_rows,
                    1,
                    300,
                    ranking.digest(),
                    10,
                )
                .v2(),
            )
            .expect("legacy NIFTY");
        legacy
            .append_complete(
                &bank_rows,
                &receipt(
                    bank_id,
                    InstrumentFamilyV1::BankNifty,
                    &bank_rows,
                    1,
                    300,
                    ranking.digest(),
                    30,
                )
                .v2(),
            )
            .expect("legacy BANKNIFTY");
        assert!(
            SelectionReceiptV1::from_committed_populations(&mut legacy, nifty_id, bank_id, ranking)
                .unwrap_err()
                .contains("requested-span-bound V3")
        );

        let mismatch_root = root("span-mismatch");
        let mut mismatch = PopulationLedger::open(&mismatch_root).expect("mismatch ledger");
        let nifty_receipt = receipt_with_identities_and_span(
            nifty_id,
            InstrumentFamilyV1::Nifty,
            &nifty_rows,
            1,
            300,
            identities(10, ranking.digest()),
            RequestedSpanIdentityV1::new(2020, 1, 2021, 12).expect("NIFTY span"),
        );
        let bank_receipt = receipt_with_identities_and_span(
            bank_id,
            InstrumentFamilyV1::BankNifty,
            &bank_rows,
            1,
            300,
            identities(30, ranking.digest()),
            RequestedSpanIdentityV1::new(2020, 2, 2021, 12).expect("BANKNIFTY span"),
        );
        mismatch
            .append_complete_v3(&nifty_rows, &nifty_receipt)
            .expect("span NIFTY");
        mismatch
            .append_complete_v3(&bank_rows, &bank_receipt)
            .expect("span BANKNIFTY");
        assert!(
            SelectionReceiptV1::from_committed_populations(
                &mut mismatch,
                nifty_id,
                bank_id,
                ranking
            )
            .unwrap_err()
            .contains("spans differ")
        );

        drop(legacy);
        drop(mismatch);
        std::fs::remove_dir_all(legacy_root).expect("legacy cleanup");
        std::fs::remove_dir_all(mismatch_root).expect("span cleanup");
    }

    #[test]
    fn every_persisted_shared_cohort_term_must_match_exactly() {
        let ranking = policy();
        let base_nifty = identities(10, ranking.digest());
        let base_bank = identities(30, ranking.digest());
        let variants = [
            (
                "feed",
                PopulationIdentitiesV2 {
                    feed_digest: digest(7),
                    ..base_bank
                },
            ),
            (
                "source commit",
                PopulationIdentitiesV2 {
                    source_commit_digest: digest(7),
                    ..base_bank
                },
            ),
            (
                "vocabulary",
                PopulationIdentitiesV2 {
                    vocabulary_digest: digest(7),
                    ..base_bank
                },
            ),
            (
                "evaluation policy",
                PopulationIdentitiesV2 {
                    evaluation_policy_digest: digest(7),
                    ..base_bank
                },
            ),
            (
                "admission policy",
                PopulationIdentitiesV2 {
                    admission_policy_digest: digest(7),
                    ..base_bank
                },
            ),
            (
                "ranking-policy",
                PopulationIdentitiesV2 {
                    ranking_policy_digest: digest(7),
                    ..base_bank
                },
            ),
            (
                "calendar policy",
                PopulationIdentitiesV2 {
                    calendar_policy_digest: digest(7),
                    ..base_bank
                },
            ),
            (
                "daily-reference policy",
                PopulationIdentitiesV2 {
                    daily_reference_policy_digest: digest(7),
                    ..base_bank
                },
            ),
        ];

        for (index, (expected, bank_identities)) in variants.into_iter().enumerate() {
            let fixture_root = root(&format!("cohort-mismatch-{index}"));
            let nifty_id = digest(1);
            let bank_id = digest(2);
            let nifty_rows = rows(nifty_id, InstrumentFamilyV1::Nifty, 1, 300);
            let bank_rows = rows(bank_id, InstrumentFamilyV1::BankNifty, 1, 300);
            let mut populations = PopulationLedger::open(&fixture_root).expect("population ledger");
            populations
                .append_complete_v3(
                    &nifty_rows,
                    &receipt_with_identities(
                        nifty_id,
                        InstrumentFamilyV1::Nifty,
                        &nifty_rows,
                        1,
                        300,
                        base_nifty,
                    ),
                )
                .expect("NIFTY cohort population");
            populations
                .append_complete_v3(
                    &bank_rows,
                    &receipt_with_identities(
                        bank_id,
                        InstrumentFamilyV1::BankNifty,
                        &bank_rows,
                        1,
                        300,
                        bank_identities,
                    ),
                )
                .expect("BANKNIFTY cohort population");
            let refusal = SelectionReceiptV1::from_committed_populations(
                &mut populations,
                nifty_id,
                bank_id,
                ranking,
            )
            .unwrap_err();
            assert!(
                refusal.contains(expected),
                "expected {expected:?} mismatch, got {refusal:?}"
            );
            drop(populations);
            std::fs::remove_dir_all(fixture_root).expect("cohort mismatch cleanup");
        }
    }

    #[test]
    fn every_shared_cohort_field_changes_its_canonical_identity() {
        let ranking = policy();
        let base = super::SharedCohortIdentityV1::from_v2_terms(
            &identities(10, ranking.digest()),
            &identities(30, ranking.digest()),
            digest(99),
        )
        .expect("matching shared cohort");
        assert_eq!(
            &base.canonical_bytes()[..16],
            b"brutex-cohort-v1",
            "the cohort codec carries its own domain"
        );
        let variants = [
            super::SharedCohortIdentityV1 {
                requested_span_digest: digest(7),
                ..base
            },
            super::SharedCohortIdentityV1 {
                feed_digest: digest(7),
                ..base
            },
            super::SharedCohortIdentityV1 {
                source_commit_digest: digest(7),
                ..base
            },
            super::SharedCohortIdentityV1 {
                vocabulary_digest: digest(7),
                ..base
            },
            super::SharedCohortIdentityV1 {
                evaluation_policy_digest: digest(7),
                ..base
            },
            super::SharedCohortIdentityV1 {
                admission_policy_digest: digest(7),
                ..base
            },
            super::SharedCohortIdentityV1 {
                ranking_policy_digest: digest(7),
                ..base
            },
            super::SharedCohortIdentityV1 {
                calendar_policy_digest: digest(7),
                ..base
            },
            super::SharedCohortIdentityV1 {
                daily_reference_policy_digest: digest(7),
                ..base
            },
        ];
        let mut seen = std::collections::HashSet::new();
        for variant in variants {
            assert_ne!(variant.digest(), base.digest());
            assert!(seen.insert(variant.digest()));
        }
    }

    #[test]
    fn receipt_decoder_rejects_unknown_reserves_counts_scores_order_and_tears() {
        let root = root("codec-adversary");
        let ranking = policy();
        let (mut populations, nifty_id, bank_id) = committed_pair(&root, 20, 300, ranking.digest());
        let receipt = SelectionReceiptV1::from_committed_populations(
            &mut populations,
            nifty_id,
            bank_id,
            ranking,
        )
        .expect("selection");
        let valid = receipt.to_bytes().expect("canonical bytes");
        assert_eq!(
            SelectionReceiptV1::from_bytes(&valid).expect("round trip"),
            receipt
        );

        let mut torn = valid;
        torn[SELECTION_STRIDE_BYTES - 1] ^= 1;
        assert!(
            SelectionReceiptV1::from_bytes(&torn)
                .unwrap_err()
                .contains("seal")
        );

        let mut unknown_family = valid;
        unknown_family[40] = 9;
        reseal(&mut unknown_family, false);
        assert!(
            SelectionReceiptV1::from_bytes(&unknown_family)
                .unwrap_err()
                .contains("unknown")
        );

        let mut reversed = valid;
        reversed[40] = 2;
        reseal(&mut reversed, true);
        assert!(
            SelectionReceiptV1::from_bytes(&reversed)
                .unwrap_err()
                .contains("noncanonical")
        );

        let mut reserve = valid;
        reserve[41] = 1;
        reseal(&mut reserve, false);
        assert!(
            SelectionReceiptV1::from_bytes(&reserve)
                .unwrap_err()
                .contains("reserve")
        );

        let mut missing_completion = valid;
        missing_completion[120..152].fill(0);
        reseal(&mut missing_completion, true);
        assert!(
            SelectionReceiptV1::from_bytes(&missing_completion)
                .unwrap_err()
                .contains("completion_digest")
        );

        let mut unknown_cohort = valid;
        unknown_cohort[360] ^= 1;
        reseal(&mut unknown_cohort, false);
        assert!(
            SelectionReceiptV1::from_bytes(&unknown_cohort)
                .unwrap_err()
                .contains("cohort domain")
        );

        let mut missing_span = valid;
        missing_span[384..416].fill(0);
        reseal(&mut missing_span, true);
        assert!(
            SelectionReceiptV1::from_bytes(&missing_span)
                .unwrap_err()
                .contains("requested-span")
        );

        let mut requested = valid;
        requested[36..40].copy_from_slice(&(REQUESTED_TOP_V1 - 1).to_le_bytes());
        reseal(&mut requested, true);
        assert!(
            SelectionReceiptV1::from_bytes(&requested)
                .unwrap_err()
                .contains("requires exactly")
        );

        let mut bad_proof = valid;
        bad_proof[264..272].copy_from_slice(&0_u64.to_le_bytes());
        reseal(&mut bad_proof, true);
        assert!(
            SelectionReceiptV1::from_bytes(&bad_proof)
                .unwrap_err()
                .contains("source row counts")
        );

        let mut missing = valid;
        missing[672..676].copy_from_slice(&0_u32.to_le_bytes());
        missing[680..2_880].fill(0);
        reseal(&mut missing, true);
        assert!(
            SelectionReceiptV1::from_bytes(&missing)
                .unwrap_err()
                .contains("requires exactly")
        );

        let mut extra = valid;
        extra[672..676].copy_from_slice(&26_u32.to_le_bytes());
        reseal(&mut extra, true);
        assert!(
            SelectionReceiptV1::from_bytes(&extra)
                .unwrap_err()
                .contains("above the fixed maximum")
        );

        let mut score = valid;
        score[760..768].copy_from_slice(&(SCORE_SCALE + 1).to_le_bytes());
        reseal(&mut score, true);
        assert!(
            SelectionReceiptV1::from_bytes(&score)
                .unwrap_err()
                .contains("score")
        );

        let mut duplicate = valid;
        duplicate.copy_within(680..768, 768);
        reseal(&mut duplicate, true);
        assert!(
            SelectionReceiptV1::from_bytes(&duplicate)
                .unwrap_err()
                .contains("repeats")
        );

        let mut ascending = valid;
        ascending[760..768].copy_from_slice(&0_u64.to_le_bytes());
        ascending[848..856].copy_from_slice(&1_u64.to_le_bytes());
        reseal(&mut ascending, true);
        assert!(
            SelectionReceiptV1::from_bytes(&ascending)
                .unwrap_err()
                .contains("strongest-first")
        );

        let mut trailing = valid;
        trailing[2_887] = 1;
        reseal(&mut trailing, false);
        assert!(
            SelectionReceiptV1::from_bytes(&trailing)
                .unwrap_err()
                .contains("trailing reserve")
        );
        drop(populations);
        std::fs::remove_dir_all(root).expect("codec cleanup");
    }

    #[test]
    fn ledger_rejects_duplicate_ids_bad_headers_ragged_records_and_reserved_bytes() {
        let root = root("ledger-adversary");
        let ranking = policy();
        let (mut populations, nifty_id, bank_id) = committed_pair(&root, 2, 300, ranking.digest());
        let receipt = SelectionReceiptV1::from_committed_populations(
            &mut populations,
            nifty_id,
            bank_id,
            ranking,
        )
        .expect("selection");
        let raw = receipt.to_bytes().expect("bytes");
        let mut ledger = SelectionLedger::open(&root, 1).expect("ledger");
        ledger.append(&receipt).expect("append");
        drop(ledger);
        let path = SelectionLedger::path(&root);
        OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("append duplicate")
            .write_all(&raw)
            .expect("duplicate bytes");
        assert!(
            SelectionLedger::open_read(&root, 2)
                .unwrap_err()
                .contains("repeats selection identity")
        );

        let header_root = self::root("bad-header");
        drop(SelectionLedger::open(&header_root, 1).expect("header ledger"));
        let header_path = SelectionLedger::path(&header_root);
        let mut header = std::fs::read(&header_path).expect("header bytes");
        header[32] = 1;
        std::fs::write(&header_path, &header).expect("bad reserve");
        assert!(
            SelectionLedger::open_read(&header_root, 1)
                .unwrap_err()
                .contains("reserve")
        );

        let ragged_root = self::root("ragged");
        drop(SelectionLedger::open(&ragged_root, 1).expect("ragged ledger"));
        OpenOptions::new()
            .append(true)
            .open(SelectionLedger::path(&ragged_root))
            .expect("ragged append")
            .write_all(&[1])
            .expect("ragged byte");
        assert!(
            SelectionLedger::open_read(&ragged_root, 1)
                .unwrap_err()
                .contains("not a multiple")
        );

        let cohort_root = self::root("cohort-corruption");
        let mut cohort_ledger = SelectionLedger::open(&cohort_root, 1).expect("cohort ledger");
        cohort_ledger.append(&receipt).expect("cohort receipt");
        drop(cohort_ledger);
        let cohort_path = SelectionLedger::path(&cohort_root);
        let mut cohort_bytes = std::fs::read(&cohort_path).expect("cohort ledger bytes");
        let cohort_feed_byte = super::HEADER_BYTES + 416;
        cohort_bytes[cohort_feed_byte] ^= 1;
        std::fs::write(&cohort_path, &cohort_bytes).expect("corrupt cohort identity");
        assert!(
            SelectionLedger::open_read(&cohort_root, 1)
                .unwrap_err()
                .contains("seal")
        );

        drop(populations);
        std::fs::remove_dir_all(root).expect("ledger cleanup");
        std::fs::remove_dir_all(header_root).expect("header cleanup");
        std::fs::remove_dir_all(ragged_root).expect("ragged cleanup");
        std::fs::remove_dir_all(cohort_root).expect("cohort cleanup");
    }

    #[test]
    fn same_id_with_different_semantics_is_rejected_before_any_append() {
        let root = root("same-id-different");
        let ranking = policy();
        let (mut populations, nifty_id, bank_id) = committed_pair(&root, 2, 300, ranking.digest());
        let receipt = SelectionReceiptV1::from_committed_populations(
            &mut populations,
            nifty_id,
            bank_id,
            ranking,
        )
        .expect("selection");
        let mut changed = receipt.clone();
        changed.rung_seconds = 600;
        assert!(
            changed
                .validate_semantics()
                .unwrap_err()
                .contains("identity does not match")
        );
        let mut ledger = SelectionLedger::open(&root, 1).expect("ledger");
        ledger.append(&receipt).expect("original append");
        assert!(
            ledger
                .append(&changed)
                .unwrap_err()
                .contains("identity does not match")
        );
        assert_eq!(ledger.selections(), 1);
        drop(ledger);
        drop(populations);
        std::fs::remove_dir_all(root).expect("same-id cleanup");
    }

    #[test]
    fn in_memory_index_distinguishes_exact_duplicates_from_same_id_conflicts() {
        let root = root("index-conflict");
        let ranking = policy();
        let receipt = selection_fixture(
            &root.join("population"),
            2,
            300,
            ranking,
            pair_variant(71, 101, span()),
        );
        let mut indexes = SelectionIndexes::with_capacity(2).expect("bounded fixture index");
        indexes
            .insert(receipt.clone(), "first fixture")
            .expect("first index insertion");
        assert!(
            indexes
                .insert(receipt.clone(), "exact fixture")
                .unwrap_err()
                .contains("repeats selection identity")
        );

        let mut conflict = receipt.clone();
        conflict.rung_seconds = 600;
        assert!(
            indexes
                .insert(conflict, "conflicting fixture")
                .unwrap_err()
                .contains("for different bytes")
        );
        assert_eq!(indexes.order.as_slice(), &[receipt.selection_id()]);
        assert_eq!(indexes.receipts.len(), 1);
        assert_eq!(indexes.latest.len(), 1);
        std::fs::remove_dir_all(root).expect("index-conflict cleanup");
    }
}
