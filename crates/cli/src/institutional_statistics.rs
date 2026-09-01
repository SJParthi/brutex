//! Receipt-last exact family statistics for institutional evidence.
//!
//! Runner computes candidate-specific Romano--Wolf adjusted probabilities in
//! memory.  A ppm projection alone cannot prove which population, selection,
//! candidate or resampling family produced it.  This module persists the exact
//! finite-resample counts, full `f64` source bits and every identity needed to
//! join the result back to one institutional candidate.
//!
//! White and SPA are never inferred from the Romano--Wolf result.  Their exact
//! `Verdict` values and shared family denominators must be supplied and are
//! retained bit-for-bit.  The two admission projections named here are both
//! Romano--Wolf values: `fwer_p_value_ppm` is the full-family
//! maximum/intersection adjusted probability, and
//! `romano_wolf_p_value_ppm` is the selected candidate's adjusted
//! probability.
//!
//! # Persistence and cost
//!
//! One fixed-stride statistics row is appended and synced before its separate
//! completion is appended and synced last.  A valid trailing uncommitted row is
//! a crash orphan, never authority; exact retry may finish that receipt and a
//! foreign retry refuses.  Open validates both complete files, every seal,
//! physical reference and identity before exposing an authority.
//!
//! Preparation and fixed-record encoding are O(1) over an already-computed
//! receipt.  The public append path first performs stale-handle validation, so
//! append as a whole and open are O(file bytes), with explicit caller bounds;
//! locking and `sync_all` latency are not constant-time claims.  Romano--Wolf
//! construction remains O(S*N + S log S + B*N + B*S*N) and is not made
//! constant by this ledger.

use brutex_core::blake3::Hasher;
use runner::admission::PPM;
use runner::bootstrap::{RomanoWolfAdjustedReceiptV1, Verdict};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Operator-facing refusal from the institutional-statistics boundary.
pub type InstitutionalStatisticsRefusalV1 = String;

const VERSION_V1: u32 = 1;
const HEADER_VERSION_V1: u32 = 1;
const HEADER_BYTES: usize = 64;
const HEADER_PAYLOAD_BYTES: usize = 32;
const STATISTICS_MAGIC_V1: [u8; 16] = *b"BTX-ISTATS-D-V1!";
const COMPLETION_MAGIC_V1: [u8; 16] = *b"BTX-ISTATS-C-V1!";
const STATISTICS_KIND_V1: u32 = 1;
const COMPLETION_KIND_V1: u32 = 2;
const STATISTICS_PAYLOAD_BYTES: usize = 368;
const STATISTICS_STRIDE_BYTES: usize = STATISTICS_PAYLOAD_BYTES + 32;
const COMPLETION_PAYLOAD_BYTES: usize = 256;
const COMPLETION_STRIDE_BYTES: usize = COMPLETION_PAYLOAD_BYTES + 32;
const STATISTICS_SEAL_DOMAIN_V1: &[u8] = b"brutex.institutional-statistics.row-seal.v1\0";
const STATISTICS_CONTENT_DOMAIN_V1: &[u8] = b"brutex.institutional-statistics.row-content.v1\0";
const COMPLETION_SEAL_DOMAIN_V1: &[u8] = b"brutex.institutional-statistics.completion-seal.v1\0";
const COMPLETION_CONTENT_DOMAIN_V1: &[u8] =
    b"brutex.institutional-statistics.completion-content.v1\0";
const SUBJECT_DOMAIN_V1: &[u8] = b"brutex.institutional-statistics.subject.v1\0";
const FILE_SNAPSHOT_DOMAIN_V1: &[u8] = b"brutex.institutional-statistics.file.v1\0";
const HEADER_SEAL_DOMAIN_V1: &[u8] = b"brutex.institutional-statistics.header.v1\0";

/// Exact finite-resample probability retained by a durable authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DurableExactProbabilityV1 {
    numerator: u64,
    denominator: u64,
}

impl DurableExactProbabilityV1 {
    /// Exact strict-exceedance numerator including the source `+1` correction.
    #[must_use]
    pub const fn numerator(self) -> u64 {
        self.numerator
    }

    /// Exact bootstrap denominator, equal to draws plus one.
    #[must_use]
    pub const fn denominator(self) -> u64 {
        self.denominator
    }

    /// Direct full-precision projection, never rounded to admission ppm.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        clippy::float_arithmetic,
        reason = "this is a statistical-value projection; exact integer counts remain authoritative beside it"
    )]
    pub fn value(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }

    fn validate(self, name: &str, expected_denominator: u64) -> Result<(), String> {
        if self.denominator != expected_denominator
            || self.numerator == 0
            || self.numerator > self.denominator
        {
            return Err(format!(
                "institutional statistics {name} fraction {}/{} is not within 1..={expected_denominator} over the shared denominator",
                self.numerator, self.denominator
            ));
        }
        Ok(())
    }
}

/// Fully identity-bound statistics row written before its completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InstitutionalStatisticsRecordV1 {
    version: u32,
    subject_id: [u8; 32],
    population_id: [u8; 32],
    selection_receipt_digest: [u8; 32],
    ranking_policy_digest: [u8; 32],
    strategy_digest: [u8; 32],
    family_digest: [u8; 32],
    draws: u64,
    strategies: u64,
    periods: u64,
    seed: u64,
    block: u64,
    candidate_index: u64,
    candidate_rank: u64,
    candidate_observed_statistic_bits: u64,
    strict_exceedances: u64,
    initial_p_value: DurableExactProbabilityV1,
    adjusted_p_value: DurableExactProbabilityV1,
    familywise_p_value: DurableExactProbabilityV1,
    white_statistic_bits: u64,
    white_p_value_bits: u64,
    spa_statistic_bits: u64,
    spa_p_value_bits: u64,
    fwer_p_value_ppm: u64,
    romano_wolf_p_value_ppm: u64,
}

impl InstitutionalStatisticsRecordV1 {
    fn validate(self) -> Result<(), InstitutionalStatisticsRefusalV1> {
        if self.version != VERSION_V1 {
            return Err(format!(
                "institutional statistics row version {} is unknown",
                self.version
            ));
        }
        require_digest("subject", &self.subject_id)?;
        require_digest("population", &self.population_id)?;
        require_digest("selection receipt", &self.selection_receipt_digest)?;
        require_digest("ranking policy", &self.ranking_policy_digest)?;
        require_digest("strategy", &self.strategy_digest)?;
        require_digest("Romano-Wolf family", &self.family_digest)?;
        let expected_subject = subject_id_v1(
            self.population_id,
            self.selection_receipt_digest,
            self.ranking_policy_digest,
            self.strategy_digest,
        );
        if self.subject_id != expected_subject {
            return Err("institutional statistics subject identity does not match its four source identities".to_owned());
        }
        if self.draws == 0 || self.strategies == 0 || self.periods < 2 || self.block == 0 {
            return Err(
                "institutional statistics carry a zero draw/strategy/block or fewer than two periods"
                    .to_owned(),
            );
        }
        if self.candidate_index >= self.strategies || self.candidate_rank >= self.strategies {
            return Err(
                "institutional statistics candidate index or rank is outside its family".to_owned(),
            );
        }
        let denominator = self
            .draws
            .checked_add(1)
            .ok_or_else(|| "institutional statistics draw denominator overflowed".to_owned())?;
        self.initial_p_value.validate("initial", denominator)?;
        self.adjusted_p_value
            .validate("candidate-adjusted", denominator)?;
        self.familywise_p_value
            .validate("familywise", denominator)?;
        if self.strict_exceedances.checked_add(1) != Some(self.initial_p_value.numerator) {
            return Err(
                "institutional statistics strict exceedances do not reproduce the initial numerator"
                    .to_owned(),
            );
        }
        if self.adjusted_p_value.numerator < self.initial_p_value.numerator {
            return Err(
                "institutional statistics adjusted probability is smaller than its initial probability"
                    .to_owned(),
            );
        }
        require_finite(
            "candidate observed statistic",
            self.candidate_observed_statistic(),
        )?;
        require_finite("White statistic", self.white_statistic())?;
        require_probability("White", self.white_p_value())?;
        require_finite("SPA statistic", self.spa_statistic())?;
        require_probability("SPA", self.spa_p_value())?;
        if exact_ppm(self.familywise_p_value)? != self.fwer_p_value_ppm
            || exact_ppm(self.adjusted_p_value)? != self.romano_wolf_p_value_ppm
        {
            return Err(
                "institutional statistics ppm projections do not reproduce their exact counts"
                    .to_owned(),
            );
        }
        Ok(())
    }

    fn candidate_observed_statistic(self) -> f64 {
        f64::from_bits(self.candidate_observed_statistic_bits)
    }

    fn white_statistic(self) -> f64 {
        f64::from_bits(self.white_statistic_bits)
    }

    fn white_p_value(self) -> f64 {
        f64::from_bits(self.white_p_value_bits)
    }

    fn spa_statistic(self) -> f64 {
        f64::from_bits(self.spa_statistic_bits)
    }

    fn spa_p_value(self) -> f64 {
        f64::from_bits(self.spa_p_value_bits)
    }

    fn payload_bytes(self) -> Result<[u8; STATISTICS_PAYLOAD_BYTES], String> {
        self.validate()?;
        let mut raw = [0_u8; STATISTICS_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.u32(self.version)?;
        encoder.u32(0)?;
        encoder.bytes(&self.subject_id)?;
        encoder.bytes(&self.population_id)?;
        encoder.bytes(&self.selection_receipt_digest)?;
        encoder.bytes(&self.ranking_policy_digest)?;
        encoder.bytes(&self.strategy_digest)?;
        encoder.bytes(&self.family_digest)?;
        for value in [
            self.draws,
            self.strategies,
            self.periods,
            self.seed,
            self.block,
            self.candidate_index,
            self.candidate_rank,
            self.candidate_observed_statistic_bits,
            self.strict_exceedances,
            self.initial_p_value.numerator,
            self.initial_p_value.denominator,
            self.adjusted_p_value.numerator,
            self.adjusted_p_value.denominator,
            self.familywise_p_value.numerator,
            self.familywise_p_value.denominator,
            self.white_statistic_bits,
            self.white_p_value_bits,
            self.spa_statistic_bits,
            self.spa_p_value_bits,
            self.fwer_p_value_ppm,
            self.romano_wolf_p_value_ppm,
        ] {
            encoder.u64(value)?;
        }
        encoder.finish()?;
        Ok(raw)
    }

    fn content_digest(self) -> Result<[u8; 32], String> {
        Ok(domain_hash(
            STATISTICS_CONTENT_DOMAIN_V1,
            &self.payload_bytes()?,
        ))
    }

    fn to_bytes(self) -> Result<[u8; STATISTICS_STRIDE_BYTES], String> {
        let payload = self.payload_bytes()?;
        let mut raw = [0_u8; STATISTICS_STRIDE_BYTES];
        raw[..STATISTICS_PAYLOAD_BYTES].copy_from_slice(&payload);
        raw[STATISTICS_PAYLOAD_BYTES..]
            .copy_from_slice(&domain_hash(STATISTICS_SEAL_DOMAIN_V1, &payload));
        Ok(raw)
    }

    fn from_bytes(raw: &[u8; STATISTICS_STRIDE_BYTES]) -> Result<Self, String> {
        let payload = raw
            .get(..STATISTICS_PAYLOAD_BYTES)
            .ok_or_else(|| "institutional statistics payload is absent".to_owned())?;
        if raw.get(STATISTICS_PAYLOAD_BYTES..)
            != Some(domain_hash(STATISTICS_SEAL_DOMAIN_V1, payload).as_slice())
        {
            return Err("institutional statistics row failed its BLAKE3 seal".to_owned());
        }
        let mut decoder = Decoder::new(payload);
        let version = decoder.u32()?;
        if decoder.u32()? != 0 {
            return Err("institutional statistics row reserve is nonzero".to_owned());
        }
        let record = Self {
            version,
            subject_id: decoder.array_32()?,
            population_id: decoder.array_32()?,
            selection_receipt_digest: decoder.array_32()?,
            ranking_policy_digest: decoder.array_32()?,
            strategy_digest: decoder.array_32()?,
            family_digest: decoder.array_32()?,
            draws: decoder.u64()?,
            strategies: decoder.u64()?,
            periods: decoder.u64()?,
            seed: decoder.u64()?,
            block: decoder.u64()?,
            candidate_index: decoder.u64()?,
            candidate_rank: decoder.u64()?,
            candidate_observed_statistic_bits: decoder.u64()?,
            strict_exceedances: decoder.u64()?,
            initial_p_value: DurableExactProbabilityV1 {
                numerator: decoder.u64()?,
                denominator: decoder.u64()?,
            },
            adjusted_p_value: DurableExactProbabilityV1 {
                numerator: decoder.u64()?,
                denominator: decoder.u64()?,
            },
            familywise_p_value: DurableExactProbabilityV1 {
                numerator: decoder.u64()?,
                denominator: decoder.u64()?,
            },
            white_statistic_bits: decoder.u64()?,
            white_p_value_bits: decoder.u64()?,
            spa_statistic_bits: decoder.u64()?,
            spa_p_value_bits: decoder.u64()?,
            fwer_p_value_ppm: decoder.u64()?,
            romano_wolf_p_value_ppm: decoder.u64()?,
        };
        decoder.finish()?;
        record.validate()?;
        Ok(record)
    }
}

/// Prepared exact statistics awaiting receipt-last commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreparedInstitutionalStatisticsV1 {
    record: InstitutionalStatisticsRecordV1,
}

/// Prepares one exact candidate-family statistics record.
///
/// # Errors
///
/// Refuses absent identities, mismatched White/SPA/Romano--Wolf denominators,
/// invalid source statistics, a foreign candidate index or malformed exact
/// counts.  White and SPA are copied only from their supplied typed verdicts.
#[expect(
    clippy::too_many_arguments,
    reason = "every identity and typed statistical source is required explicitly; bundling them into a forgeable loose context would weaken the join"
)]
pub fn prepare_institutional_statistics_v1(
    population_id: [u8; 32],
    selection_receipt_digest: [u8; 32],
    ranking_policy_digest: [u8; 32],
    strategy_digest: [u8; 32],
    candidate_index: usize,
    white: &Verdict,
    spa: &Verdict,
    romano_wolf: &RomanoWolfAdjustedReceiptV1,
) -> Result<PreparedInstitutionalStatisticsV1, InstitutionalStatisticsRefusalV1> {
    require_digest("population", &population_id)?;
    require_digest("selection receipt", &selection_receipt_digest)?;
    require_digest("ranking policy", &ranking_policy_digest)?;
    require_digest("strategy", &strategy_digest)?;
    require_digest("Romano-Wolf family", &romano_wolf.family_digest())?;
    let family = (white.draws, white.strategies, white.periods);
    if family != (spa.draws, spa.strategies, spa.periods)
        || family
            != (
                romano_wolf.draws(),
                romano_wolf.strategies(),
                romano_wolf.periods(),
            )
    {
        return Err(
            "institutional statistics White, SPA and Romano-Wolf describe different families"
                .to_owned(),
        );
    }
    require_finite("White statistic", white.statistic)?;
    require_probability("White", white.p_value)?;
    require_finite("SPA statistic", spa.statistic)?;
    require_probability("SPA", spa.p_value)?;
    let candidate = romano_wolf.candidate(candidate_index).ok_or_else(|| {
        format!(
            "institutional statistics candidate index {candidate_index} is outside {} strategies",
            romano_wolf.strategies()
        )
    })?;
    if candidate.strategy() != candidate_index {
        return Err(
            "institutional statistics positional candidate lookup changed identity".to_owned(),
        );
    }
    let familywise = romano_wolf.familywise_p_value().ok_or_else(|| {
        "institutional statistics Romano-Wolf family has no leading intersection probability"
            .to_owned()
    })?;
    let initial = exact_from_runner(
        "initial",
        candidate.initial_p_value().numerator(),
        candidate.initial_p_value().denominator(),
    )?;
    let adjusted = exact_from_runner(
        "candidate-adjusted",
        candidate.adjusted_p_value().numerator(),
        candidate.adjusted_p_value().denominator(),
    )?;
    let familywise = exact_from_runner(
        "familywise",
        familywise.numerator(),
        familywise.denominator(),
    )?;
    let draws = usize_to_u64("draws", romano_wolf.draws())?;
    let record = InstitutionalStatisticsRecordV1 {
        version: VERSION_V1,
        subject_id: subject_id_v1(
            population_id,
            selection_receipt_digest,
            ranking_policy_digest,
            strategy_digest,
        ),
        population_id,
        selection_receipt_digest,
        ranking_policy_digest,
        strategy_digest,
        family_digest: romano_wolf.family_digest(),
        draws,
        strategies: usize_to_u64("strategies", romano_wolf.strategies())?,
        periods: usize_to_u64("periods", romano_wolf.periods())?,
        seed: romano_wolf.seed(),
        block: usize_to_u64("block", romano_wolf.block())?,
        candidate_index: usize_to_u64("candidate index", candidate_index)?,
        candidate_rank: usize_to_u64("candidate rank", candidate.stepdown_rank())?,
        candidate_observed_statistic_bits: candidate.observed_statistic().to_bits(),
        strict_exceedances: usize_to_u64("strict exceedances", candidate.strict_exceedances())?,
        initial_p_value: initial,
        adjusted_p_value: adjusted,
        familywise_p_value: familywise,
        white_statistic_bits: white.statistic.to_bits(),
        white_p_value_bits: white.p_value.to_bits(),
        spa_statistic_bits: spa.statistic.to_bits(),
        spa_p_value_bits: spa.p_value.to_bits(),
        fwer_p_value_ppm: exact_ppm(familywise)?,
        romano_wolf_p_value_ppm: exact_ppm(adjusted)?,
    };
    record.validate()?;
    if record.draws.checked_add(1) != Some(record.adjusted_p_value.denominator) {
        return Err(
            "institutional statistics exact denominator differs from draws plus one".to_owned(),
        );
    }
    Ok(PreparedInstitutionalStatisticsV1 { record })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InstitutionalStatisticsCompletionV1 {
    version: u32,
    subject_id: [u8; 32],
    record_index: u64,
    record_digest: [u8; 32],
    population_id: [u8; 32],
    selection_receipt_digest: [u8; 32],
    ranking_policy_digest: [u8; 32],
    strategy_digest: [u8; 32],
    family_digest: [u8; 32],
    fwer_p_value_ppm: u64,
    romano_wolf_p_value_ppm: u64,
}

impl InstitutionalStatisticsCompletionV1 {
    fn for_record(
        record: &InstitutionalStatisticsRecordV1,
        record_index: u64,
    ) -> Result<Self, String> {
        (*record).validate()?;
        let completion = Self {
            version: VERSION_V1,
            subject_id: record.subject_id,
            record_index,
            record_digest: record.content_digest()?,
            population_id: record.population_id,
            selection_receipt_digest: record.selection_receipt_digest,
            ranking_policy_digest: record.ranking_policy_digest,
            strategy_digest: record.strategy_digest,
            family_digest: record.family_digest,
            fwer_p_value_ppm: record.fwer_p_value_ppm,
            romano_wolf_p_value_ppm: record.romano_wolf_p_value_ppm,
        };
        completion.validate()?;
        Ok(completion)
    }

    fn validate(self) -> Result<(), String> {
        if self.version != VERSION_V1 {
            return Err(format!(
                "institutional statistics completion version {} is unknown",
                self.version
            ));
        }
        require_digest("completion subject", &self.subject_id)?;
        require_digest("completion record", &self.record_digest)?;
        require_digest("completion population", &self.population_id)?;
        require_digest(
            "completion selection receipt",
            &self.selection_receipt_digest,
        )?;
        require_digest("completion ranking policy", &self.ranking_policy_digest)?;
        require_digest("completion strategy", &self.strategy_digest)?;
        require_digest("completion Romano-Wolf family", &self.family_digest)?;
        if self.subject_id
            != subject_id_v1(
                self.population_id,
                self.selection_receipt_digest,
                self.ranking_policy_digest,
                self.strategy_digest,
            )
        {
            return Err(
                "institutional statistics completion subject identity is foreign".to_owned(),
            );
        }
        if self.fwer_p_value_ppm > PPM || self.romano_wolf_p_value_ppm > PPM {
            return Err(
                "institutional statistics completion carries a probability above one million ppm"
                    .to_owned(),
            );
        }
        Ok(())
    }

    fn matches(self, record: &InstitutionalStatisticsRecordV1) -> Result<(), String> {
        if self.subject_id != record.subject_id
            || self.record_digest != (*record).content_digest()?
            || self.population_id != record.population_id
            || self.selection_receipt_digest != record.selection_receipt_digest
            || self.ranking_policy_digest != record.ranking_policy_digest
            || self.strategy_digest != record.strategy_digest
            || self.family_digest != record.family_digest
            || self.fwer_p_value_ppm != record.fwer_p_value_ppm
            || self.romano_wolf_p_value_ppm != record.romano_wolf_p_value_ppm
        {
            return Err(
                "institutional statistics completion does not bind its referenced row".to_owned(),
            );
        }
        Ok(())
    }

    fn payload_bytes(self) -> Result<[u8; COMPLETION_PAYLOAD_BYTES], String> {
        self.validate()?;
        let mut raw = [0_u8; COMPLETION_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.u32(self.version)?;
        encoder.u32(0)?;
        encoder.bytes(&self.subject_id)?;
        encoder.u64(self.record_index)?;
        encoder.bytes(&self.record_digest)?;
        encoder.bytes(&self.population_id)?;
        encoder.bytes(&self.selection_receipt_digest)?;
        encoder.bytes(&self.ranking_policy_digest)?;
        encoder.bytes(&self.strategy_digest)?;
        encoder.bytes(&self.family_digest)?;
        encoder.u64(self.fwer_p_value_ppm)?;
        encoder.u64(self.romano_wolf_p_value_ppm)?;
        encoder.finish()?;
        Ok(raw)
    }

    fn content_digest(self) -> Result<[u8; 32], String> {
        Ok(domain_hash(
            COMPLETION_CONTENT_DOMAIN_V1,
            &self.payload_bytes()?,
        ))
    }

    fn to_bytes(self) -> Result<[u8; COMPLETION_STRIDE_BYTES], String> {
        let payload = self.payload_bytes()?;
        let mut raw = [0_u8; COMPLETION_STRIDE_BYTES];
        raw[..COMPLETION_PAYLOAD_BYTES].copy_from_slice(&payload);
        raw[COMPLETION_PAYLOAD_BYTES..]
            .copy_from_slice(&domain_hash(COMPLETION_SEAL_DOMAIN_V1, &payload));
        Ok(raw)
    }

    fn from_bytes(raw: &[u8; COMPLETION_STRIDE_BYTES]) -> Result<Self, String> {
        let payload = raw
            .get(..COMPLETION_PAYLOAD_BYTES)
            .ok_or_else(|| "institutional statistics completion payload is absent".to_owned())?;
        if raw.get(COMPLETION_PAYLOAD_BYTES..)
            != Some(domain_hash(COMPLETION_SEAL_DOMAIN_V1, payload).as_slice())
        {
            return Err("institutional statistics completion failed its BLAKE3 seal".to_owned());
        }
        let mut decoder = Decoder::new(payload);
        let version = decoder.u32()?;
        if decoder.u32()? != 0 {
            return Err("institutional statistics completion reserve is nonzero".to_owned());
        }
        let completion = Self {
            version,
            subject_id: decoder.array_32()?,
            record_index: decoder.u64()?,
            record_digest: decoder.array_32()?,
            population_id: decoder.array_32()?,
            selection_receipt_digest: decoder.array_32()?,
            ranking_policy_digest: decoder.array_32()?,
            strategy_digest: decoder.array_32()?,
            family_digest: decoder.array_32()?,
            fwer_p_value_ppm: decoder.u64()?,
            romano_wolf_p_value_ppm: decoder.u64()?,
        };
        decoder.finish()?;
        completion.validate()?;
        Ok(completion)
    }
}

/// Reopened, receipt-last exact statistics authority for one selected strategy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstitutionalStatisticsAuthorityV1 {
    record: InstitutionalStatisticsRecordV1,
    completion_digest: [u8; 32],
}

impl InstitutionalStatisticsAuthorityV1 {
    /// Identity joining population, selection, ranking policy and strategy.
    #[must_use]
    pub const fn subject_id(&self) -> [u8; 32] {
        self.record.subject_id
    }

    /// Exact Population V4 authority identity.
    #[must_use]
    pub const fn population_id(&self) -> [u8; 32] {
        self.record.population_id
    }

    /// Receipt-last selection authority identity.
    #[must_use]
    pub const fn selection_receipt_digest(&self) -> [u8; 32] {
        self.record.selection_receipt_digest
    }

    /// Ranking-policy identity used by the selection authority.
    #[must_use]
    pub const fn ranking_policy_digest(&self) -> [u8; 32] {
        self.record.ranking_policy_digest
    }

    /// Selected strategy identity.
    #[must_use]
    pub const fn strategy_digest(&self) -> [u8; 32] {
        self.record.strategy_digest
    }

    /// Ordered Romano--Wolf input/procedure family identity.
    #[must_use]
    pub const fn family_digest(&self) -> [u8; 32] {
        self.record.family_digest
    }

    /// Receipt-last durable authority identity.
    #[must_use]
    pub const fn completion_digest(&self) -> [u8; 32] {
        self.completion_digest
    }

    /// Deterministic bootstrap draw count.
    #[must_use]
    pub const fn draws(&self) -> u64 {
        self.record.draws
    }

    /// Complete candidate-family width.
    #[must_use]
    pub const fn strategies(&self) -> u64 {
        self.record.strategies
    }

    /// Aligned periods per strategy.
    #[must_use]
    pub const fn periods(&self) -> u64 {
        self.record.periods
    }

    /// Explicit deterministic bootstrap seed.
    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.record.seed
    }

    /// Stationary-bootstrap average block length.
    #[must_use]
    pub const fn block(&self) -> u64 {
        self.record.block
    }

    /// Caller position of the selected candidate.
    #[must_use]
    pub const fn candidate_index(&self) -> u64 {
        self.record.candidate_index
    }

    /// Canonical observed-statistic stepdown rank.
    #[must_use]
    pub const fn candidate_rank(&self) -> u64 {
        self.record.candidate_rank
    }

    /// Selected candidate's full-precision observed statistic.
    #[must_use]
    pub fn candidate_observed_statistic(&self) -> f64 {
        self.record.candidate_observed_statistic()
    }

    /// Exact selected-candidate adjusted probability.
    #[must_use]
    pub const fn adjusted_p_value(&self) -> DurableExactProbabilityV1 {
        self.record.adjusted_p_value
    }

    /// Exact full-family maximum/intersection probability.
    #[must_use]
    pub const fn familywise_p_value(&self) -> DurableExactProbabilityV1 {
        self.record.familywise_p_value
    }

    /// Full-precision White observed statistic.
    #[must_use]
    pub fn white_statistic(&self) -> f64 {
        self.record.white_statistic()
    }

    /// Full-precision White p-value supplied by its own typed verdict.
    #[must_use]
    pub fn white_p_value(&self) -> f64 {
        self.record.white_p_value()
    }

    /// Full-precision SPA observed statistic.
    #[must_use]
    pub fn spa_statistic(&self) -> f64 {
        self.record.spa_statistic()
    }

    /// Full-precision SPA p-value supplied by its own typed verdict.
    #[must_use]
    pub fn spa_p_value(&self) -> f64 {
        self.record.spa_p_value()
    }

    /// Exact-count-derived full-family probability projection.
    #[must_use]
    pub const fn fwer_p_value_ppm(&self) -> u64 {
        self.record.fwer_p_value_ppm
    }

    /// Exact-count-derived selected-candidate adjusted probability projection.
    #[must_use]
    pub const fn romano_wolf_p_value_ppm(&self) -> u64 {
        self.record.romano_wolf_p_value_ppm
    }

    /// Exact candidate decision at an operator-supplied ppm alpha.
    #[must_use]
    pub fn romano_wolf_rejects_at_ppm(&self, alpha_ppm: u64) -> Option<bool> {
        if alpha_ppm > PPM {
            return None;
        }
        let probability = self.record.adjusted_p_value;
        Some(
            u128::from(probability.numerator) * u128::from(PPM)
                <= u128::from(probability.denominator) * u128::from(alpha_ppm),
        )
    }

    /// Verifies the complete institutional join identity.
    ///
    /// # Errors
    ///
    /// Refuses any foreign population, selection, ranking or strategy.
    pub fn require_identity(
        &self,
        population_id: &[u8; 32],
        selection_receipt_digest: &[u8; 32],
        ranking_policy_digest: &[u8; 32],
        strategy_digest: &[u8; 32],
    ) -> Result<(), InstitutionalStatisticsRefusalV1> {
        if self.record.population_id != *population_id
            || self.record.selection_receipt_digest != *selection_receipt_digest
            || self.record.ranking_policy_digest != *ranking_policy_digest
            || self.record.strategy_digest != *strategy_digest
        {
            return Err(
                "institutional statistics authority belongs to a foreign population, selection, ranking policy or strategy"
                    .to_owned(),
            );
        }
        Ok(())
    }
}

/// Result of an idempotent receipt-last statistics commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstitutionalStatisticsCommitV1 {
    /// A new statistics row and completion were synced.
    Appended(InstitutionalStatisticsAuthorityV1),
    /// The exact already-committed authority was reused without appending.
    Reused(InstitutionalStatisticsAuthorityV1),
}

impl InstitutionalStatisticsCommitV1 {
    /// Exact authority produced or reused by this commit.
    #[must_use]
    pub const fn authority(self) -> InstitutionalStatisticsAuthorityV1 {
        match self {
            Self::Appended(authority) | Self::Reused(authority) => authority,
        }
    }
}

#[derive(Clone, Debug)]
struct StatisticsPathsV1 {
    statistics: PathBuf,
    completions: PathBuf,
    lock: PathBuf,
}

impl StatisticsPathsV1 {
    fn of(root: &Path) -> Self {
        Self {
            statistics: InstitutionalStatisticsLedgerV1::statistics_path(root),
            completions: InstitutionalStatisticsLedgerV1::completion_path(root),
            lock: InstitutionalStatisticsLedgerV1::lock_path(root),
        }
    }
}

#[derive(Debug)]
struct StatisticsFilesV1 {
    statistics: File,
    completions: File,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileSnapshotV1 {
    length: u64,
    digest: [u8; 32],
}

/// Bounded append-only receipt-last ledger for exact institutional statistics.
#[derive(Debug)]
pub struct InstitutionalStatisticsLedgerV1 {
    files: StatisticsFilesV1,
    writer_lock: File,
    paths: StatisticsPathsV1,
    snapshots: [FileSnapshotV1; 2],
    authorities: HashMap<[u8; 32], InstitutionalStatisticsAuthorityV1>,
    records: Vec<InstitutionalStatisticsRecordV1>,
    trailing_orphan: Option<usize>,
    max_records: usize,
    writable: bool,
}

impl InstitutionalStatisticsLedgerV1 {
    /// Fixed-stride exact statistics-row path.
    #[must_use]
    pub fn statistics_path(root: &Path) -> PathBuf {
        root.join("results/institutional-statistics-values-v1.bin")
    }

    /// Receipt-last completion path.
    #[must_use]
    pub fn completion_path(root: &Path) -> PathBuf {
        root.join("results/institutional-statistics-completions-v1.bin")
    }

    /// Shared writer-lock path covering both files.
    #[must_use]
    pub fn lock_path(root: &Path) -> PathBuf {
        root.join("results/institutional-statistics-write-v1.lock")
    }

    /// Opens or creates a writable bounded ledger and validates all bytes.
    ///
    /// # Errors
    ///
    /// Refuses an invalid bound, malformed/torn/corrupt/reordered/duplicate
    /// record, ambiguous orphan, lock, allocation, sync or I/O failure.
    pub fn open(root: &Path, max_records: usize) -> Result<Self, String> {
        validate_bound(max_records)?;
        fs::create_dir_all(root.join("results")).map_err(|why| {
            format!(
                "{} could not be created: {why}",
                root.join("results").display()
            )
        })?;
        let paths = StatisticsPathsV1::of(root);
        let writer_lock = open_or_create(&paths.lock)?;
        writer_lock
            .lock()
            .map_err(|why| format!("{} could not be locked: {why}", paths.lock.display()))?;
        let opened = (|| {
            let mut files = StatisticsFilesV1 {
                statistics: open_or_create(&paths.statistics)?,
                completions: open_or_create(&paths.completions)?,
            };
            ensure_header(
                &mut files.statistics,
                &paths.statistics,
                STATISTICS_MAGIC_V1,
                STATISTICS_KIND_V1,
                STATISTICS_STRIDE_BYTES,
            )?;
            ensure_header(
                &mut files.completions,
                &paths.completions,
                COMPLETION_MAGIC_V1,
                COMPLETION_KIND_V1,
                COMPLETION_STRIDE_BYTES,
            )?;
            Self::from_files(
                files,
                paths.clone(),
                writer_lock.try_clone().map_err(|why| {
                    format!("institutional statistics lock could not be cloned: {why}")
                })?,
                max_records,
                true,
            )
        })();
        let released = writer_lock
            .unlock()
            .map_err(|why| format!("{} could not be unlocked: {why}", paths.lock.display()));
        match (opened, released) {
            (Ok(ledger), Ok(())) => Ok(ledger),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// Opens and fully validates an existing ledger without write permission.
    ///
    /// # Errors
    ///
    /// Refuses absent, malformed, torn, corrupt, reordered, duplicate,
    /// over-bound or ambiguously orphaned files.
    pub fn open_read(root: &Path, max_records: usize) -> Result<Self, String> {
        validate_bound(max_records)?;
        let paths = StatisticsPathsV1::of(root);
        let writer_lock = File::open(&paths.lock)
            .map_err(|why| format!("{} could not be opened: {why}", paths.lock.display()))?;
        writer_lock
            .lock_shared()
            .map_err(|why| format!("{} could not be shared-locked: {why}", paths.lock.display()))?;
        let opened = (|| {
            Self::from_files(
                StatisticsFilesV1 {
                    statistics: open_read(&paths.statistics)?,
                    completions: open_read(&paths.completions)?,
                },
                paths.clone(),
                writer_lock.try_clone().map_err(|why| {
                    format!("institutional statistics lock could not be cloned: {why}")
                })?,
                max_records,
                false,
            )
        })();
        let released = writer_lock
            .unlock()
            .map_err(|why| format!("{} could not be unlocked: {why}", paths.lock.display()));
        match (opened, released) {
            (Ok(ledger), Ok(())) => Ok(ledger),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn from_files(
        mut files: StatisticsFilesV1,
        paths: StatisticsPathsV1,
        writer_lock: File,
        max_records: usize,
        writable: bool,
    ) -> Result<Self, String> {
        check_file(
            &mut files.statistics,
            &paths.statistics,
            STATISTICS_MAGIC_V1,
            STATISTICS_KIND_V1,
            STATISTICS_STRIDE_BYTES,
        )?;
        check_file(
            &mut files.completions,
            &paths.completions,
            COMPLETION_MAGIC_V1,
            COMPLETION_KIND_V1,
            COMPLETION_STRIDE_BYTES,
        )?;
        let records = scan_records::<STATISTICS_STRIDE_BYTES, _>(
            &mut files.statistics,
            &paths.statistics,
            STATISTICS_STRIDE_BYTES,
            max_records,
            InstitutionalStatisticsRecordV1::from_bytes,
        )?;
        let completions = scan_records::<COMPLETION_STRIDE_BYTES, _>(
            &mut files.completions,
            &paths.completions,
            COMPLETION_STRIDE_BYTES,
            max_records,
            InstitutionalStatisticsCompletionV1::from_bytes,
        )?;
        if completions.len() > records.len() || records.len() - completions.len() > 1 {
            return Err(
                "institutional statistics files contain missing rows or more than one crash orphan"
                    .to_owned(),
            );
        }
        let mut record_subjects = HashSet::new();
        for record in &records {
            if !record_subjects.insert(record.subject_id) {
                return Err(
                    "institutional statistics rows contain a duplicate subject identity".to_owned(),
                );
            }
        }
        let mut authorities = HashMap::new();
        authorities.try_reserve(completions.len()).map_err(|why| {
            format!("institutional statistics authority allocation refused: {why}")
        })?;
        for (sequence, completion) in completions.iter().copied().enumerate() {
            let expected_index = usize_to_u64("completion sequence", sequence)?;
            if completion.record_index != expected_index {
                return Err(format!(
                    "institutional statistics completion {sequence} references row {} instead of {expected_index}",
                    completion.record_index
                ));
            }
            let record = records.get(sequence).copied().ok_or_else(|| {
                format!("institutional statistics completion {sequence} references no row")
            })?;
            completion.matches(&record)?;
            let authority = InstitutionalStatisticsAuthorityV1 {
                record,
                completion_digest: completion.content_digest()?,
            };
            if authorities.insert(record.subject_id, authority).is_some() {
                return Err(
                    "institutional statistics completions contain a duplicate subject identity"
                        .to_owned(),
                );
            }
        }
        let trailing_orphan = (records.len() > completions.len()).then_some(completions.len());
        let snapshots = snapshots(&mut files, &paths)?;
        Ok(Self {
            files,
            writer_lock,
            paths,
            snapshots,
            authorities,
            records,
            trailing_orphan,
            max_records,
            writable,
        })
    }

    /// Number of receipt-last completed authorities.
    #[must_use]
    pub fn completions(&self) -> usize {
        self.authorities.len()
    }

    /// Reopened authority by complete institutional subject identity.
    #[must_use]
    pub fn authority(&self, subject_id: &[u8; 32]) -> Option<&InstitutionalStatisticsAuthorityV1> {
        self.authorities.get(subject_id)
    }

    /// Appends/syncs one statistics row before appending/syncing its completion.
    ///
    /// Exact already-committed inputs reuse byte-for-byte. An exact valid tail
    /// orphan is completed in place; a foreign or changed retry refuses.
    ///
    /// # Errors
    ///
    /// Refuses read-only/stale handles, changed identity reuse, exhausted
    /// bounds, foreign orphans, malformed records, lock, sync or I/O failure.
    pub fn append_complete(
        &mut self,
        prepared: &PreparedInstitutionalStatisticsV1,
    ) -> Result<InstitutionalStatisticsCommitV1, String> {
        if !self.writable {
            return Err("a read-only institutional statistics ledger cannot append".to_owned());
        }
        prepared.record.validate()?;
        self.writer_lock
            .lock()
            .map_err(|why| format!("{} could not be locked: {why}", self.paths.lock.display()))?;
        let attempted = self.append_complete_locked(prepared);
        let released = self
            .writer_lock
            .unlock()
            .map_err(|why| format!("{} could not be unlocked: {why}", self.paths.lock.display()));
        match (attempted, released) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn append_complete_locked(
        &mut self,
        prepared: &PreparedInstitutionalStatisticsV1,
    ) -> Result<InstitutionalStatisticsCommitV1, String> {
        self.require_files_unchanged()?;
        let record = prepared.record;
        if let Some(authority) = self.authorities.get(&record.subject_id).copied() {
            if authority.record == record {
                return Ok(InstitutionalStatisticsCommitV1::Reused(authority));
            }
            return Err(
                "institutional statistics subject is already committed with different evidence"
                    .to_owned(),
            );
        }
        let record_index = if let Some(orphan_index) = self.trailing_orphan {
            let orphan = self.records.get(orphan_index).copied().ok_or_else(|| {
                "institutional statistics trailing orphan index is absent".to_owned()
            })?;
            if orphan != record {
                return Err(
                    "institutional statistics trailing orphan is foreign; only its exact retry may continue"
                        .to_owned(),
                );
            }
            usize_to_u64("orphan row index", orphan_index)?
        } else {
            if self.records.len() >= self.max_records {
                return Err(format!(
                    "institutional statistics record bound {} is exhausted",
                    self.max_records
                ));
            }
            append_sync(
                &mut self.files.statistics,
                &self.paths.statistics,
                &record.to_bytes()?,
            )?;
            let index = self.records.len();
            self.records.push(record);
            self.trailing_orphan = Some(index);
            usize_to_u64("statistics row index", index)?
        };
        let completion = InstitutionalStatisticsCompletionV1::for_record(&record, record_index)?;
        append_sync(
            &mut self.files.completions,
            &self.paths.completions,
            &completion.to_bytes()?,
        )?;
        let authority = InstitutionalStatisticsAuthorityV1 {
            record,
            completion_digest: completion.content_digest()?,
        };
        self.authorities.insert(record.subject_id, authority);
        self.trailing_orphan = None;
        self.snapshots = snapshots(&mut self.files, &self.paths)?;
        Ok(InstitutionalStatisticsCommitV1::Appended(authority))
    }

    fn require_files_unchanged(&mut self) -> Result<(), String> {
        let actual = snapshots(&mut self.files, &self.paths)?;
        if actual != self.snapshots {
            return Err(
                "institutional statistics ledger changed behind this open handle; reopen before appending"
                    .to_owned(),
            );
        }
        for (path, expected) in [
            (&self.paths.statistics, self.snapshots[0]),
            (&self.paths.completions, self.snapshots[1]),
        ] {
            let mut reopened = open_read(path)?;
            if snapshot_file(&mut reopened, path)? != expected {
                return Err(format!(
                    "{} no longer names the file held by this institutional statistics handle",
                    path.display()
                ));
            }
        }
        Ok(())
    }
}

fn canonical_header(
    magic: [u8; 16],
    kind: u32,
    stride: usize,
) -> Result<[u8; HEADER_BYTES], String> {
    let mut payload = [0_u8; HEADER_PAYLOAD_BYTES];
    let mut encoder = Encoder::new(&mut payload);
    encoder.bytes(&magic)?;
    encoder.u32(HEADER_VERSION_V1)?;
    encoder.u32(kind)?;
    encoder.u64(
        u64::try_from(stride)
            .map_err(|_| "institutional statistics stride does not fit u64".to_owned())?,
    )?;
    encoder.finish()?;
    let mut header = [0_u8; HEADER_BYTES];
    header[..HEADER_PAYLOAD_BYTES].copy_from_slice(&payload);
    header[HEADER_PAYLOAD_BYTES..].copy_from_slice(&domain_hash(HEADER_SEAL_DOMAIN_V1, &payload));
    Ok(header)
}

fn ensure_header(
    file: &mut File,
    path: &Path,
    magic: [u8; 16],
    kind: u32,
    stride: usize,
) -> Result<(), String> {
    if measured_len(file, path)? != 0 {
        return Ok(());
    }
    file.write_all(&canonical_header(magic, kind, stride)?)
        .and_then(|()| file.sync_all())
        .map_err(|why| {
            format!(
                "{} could not receive a durable header: {why}",
                path.display()
            )
        })
}

fn check_file(
    file: &mut File,
    path: &Path,
    magic: [u8; 16],
    kind: u32,
    stride: usize,
) -> Result<(), String> {
    let len = measured_len(file, path)?;
    let header_bytes = u64::try_from(HEADER_BYTES)
        .map_err(|_| "institutional statistics header width does not fit u64".to_owned())?;
    if len < header_bytes {
        return Err(format!(
            "{} is shorter than its fixed header",
            path.display()
        ));
    }
    let body = len
        .checked_sub(header_bytes)
        .ok_or_else(|| "institutional statistics file length underflowed".to_owned())?;
    let stride_u64 = u64::try_from(stride)
        .map_err(|_| "institutional statistics stride does not fit u64".to_owned())?;
    if body % stride_u64 != 0 {
        return Err(format!(
            "{} has a torn {body}-byte body for stride {stride}",
            path.display()
        ));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("{} could not be seeked: {why}", path.display()))?;
    let mut actual = [0_u8; HEADER_BYTES];
    file.read_exact(&mut actual)
        .map_err(|why| format!("{} header could not be read: {why}", path.display()))?;
    if actual != canonical_header(magic, kind, stride)? {
        return Err(format!("{} header is noncanonical", path.display()));
    }
    Ok(())
}

fn scan_records<const STRIDE: usize, T>(
    file: &mut File,
    path: &Path,
    stride: usize,
    max_records: usize,
    decode: fn(&[u8; STRIDE]) -> Result<T, String>,
) -> Result<Vec<T>, String> {
    let header_bytes = u64::try_from(HEADER_BYTES)
        .map_err(|_| "institutional statistics header width does not fit u64".to_owned())?;
    let stride_u64 = u64::try_from(stride)
        .map_err(|_| "institutional statistics stride does not fit u64".to_owned())?;
    let count_u64 = measured_len(file, path)?
        .checked_sub(header_bytes)
        .ok_or_else(|| format!("{} is shorter than its header", path.display()))?
        / stride_u64;
    let count = usize::try_from(count_u64)
        .map_err(|_| format!("{} record count does not fit usize", path.display()))?;
    if count > max_records {
        return Err(format!(
            "{} contains {count} records above caller bound {max_records}",
            path.display()
        ));
    }
    let mut records = Vec::new();
    records
        .try_reserve_exact(count)
        .map_err(|why| format!("{} record allocation refused: {why}", path.display()))?;
    file.seek(SeekFrom::Start(header_bytes))
        .map_err(|why| format!("{} could not be seeked: {why}", path.display()))?;
    for sequence in 0..count {
        let mut raw = [0_u8; STRIDE];
        file.read_exact(&mut raw).map_err(|why| {
            format!(
                "{} record {sequence} could not be read completely: {why}",
                path.display()
            )
        })?;
        records
            .push(decode(&raw).map_err(|why| {
                format!("{} record {sequence} is invalid: {why}", path.display())
            })?);
    }
    Ok(records)
}

fn open_or_create(path: &Path) -> Result<File, String> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|why| format!("{} could not be opened: {why}", path.display()))
}

fn open_read(path: &Path) -> Result<File, String> {
    File::open(path).map_err(|why| format!("{} could not be opened: {why}", path.display()))
}

fn append_sync(file: &mut File, path: &Path, bytes: &[u8]) -> Result<(), String> {
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(bytes))
        .and_then(|()| file.sync_all())
        .map_err(|why| format!("{} append/sync failed: {why}", path.display()))
}

fn measured_len(file: &File, path: &Path) -> Result<u64, String> {
    file.metadata()
        .map(|metadata| metadata.len())
        .map_err(|why| format!("{} metadata could not be read: {why}", path.display()))
}

fn snapshots(
    files: &mut StatisticsFilesV1,
    paths: &StatisticsPathsV1,
) -> Result<[FileSnapshotV1; 2], String> {
    Ok([
        snapshot_file(&mut files.statistics, &paths.statistics)?,
        snapshot_file(&mut files.completions, &paths.completions)?,
    ])
}

fn snapshot_file(file: &mut File, path: &Path) -> Result<FileSnapshotV1, String> {
    let length = measured_len(file, path)?;
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("{} could not be seeked for hashing: {why}", path.display()))?;
    let mut hasher = Hasher::new();
    hasher.update(FILE_SNAPSHOT_DOMAIN_V1);
    hasher.update(&length.to_le_bytes());
    let mut remaining = length;
    let mut buffer = [0_u8; 8_192];
    while remaining > 0 {
        let buffer_len = u64::try_from(buffer.len())
            .map_err(|_| "institutional statistics hash buffer does not fit u64".to_owned())?;
        let wanted = usize::try_from(remaining.min(buffer_len))
            .map_err(|_| "institutional statistics hash window does not fit usize".to_owned())?;
        let window = buffer
            .get_mut(..wanted)
            .ok_or_else(|| "institutional statistics hash window exceeded buffer".to_owned())?;
        file.read_exact(window)
            .map_err(|why| format!("{} could not be hashed: {why}", path.display()))?;
        hasher.update(window);
        remaining -= u64::try_from(wanted)
            .map_err(|_| "institutional statistics hash window does not fit u64".to_owned())?;
    }
    Ok(FileSnapshotV1 {
        length,
        digest: hasher.finalize(),
    })
}

fn validate_bound(max_records: usize) -> Result<(), String> {
    if max_records == 0 {
        Err("institutional statistics record bound must be nonzero".to_owned())
    } else {
        Ok(())
    }
}

fn require_digest(name: &str, digest: &[u8; 32]) -> Result<(), String> {
    if digest.iter().all(|byte| *byte == 0) {
        Err(format!("institutional statistics {name} digest is absent"))
    } else {
        Ok(())
    }
}

fn require_finite(name: &str, value: f64) -> Result<(), String> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(format!("institutional statistics {name} is not finite"))
    }
}

fn require_probability(name: &str, value: f64) -> Result<(), String> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(format!(
            "institutional statistics {name} p-value is outside 0..=1"
        ))
    }
}

fn subject_id_v1(
    population_id: [u8; 32],
    selection_receipt_digest: [u8; 32],
    ranking_policy_digest: [u8; 32],
    strategy_digest: [u8; 32],
) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(SUBJECT_DOMAIN_V1);
    hasher.update(&population_id);
    hasher.update(&selection_receipt_digest);
    hasher.update(&ranking_policy_digest);
    hasher.update(&strategy_digest);
    hasher.finalize()
}

fn exact_from_runner(
    name: &str,
    numerator: usize,
    denominator: usize,
) -> Result<DurableExactProbabilityV1, String> {
    let value = DurableExactProbabilityV1 {
        numerator: usize_to_u64(&format!("{name} numerator"), numerator)?,
        denominator: usize_to_u64(&format!("{name} denominator"), denominator)?,
    };
    value.validate(name, value.denominator)?;
    Ok(value)
}

fn exact_ppm(probability: DurableExactProbabilityV1) -> Result<u64, String> {
    if probability.denominator == 0 || probability.numerator > probability.denominator {
        return Err("institutional statistics exact probability is malformed".to_owned());
    }
    let projected = u128::from(probability.numerator)
        .checked_mul(u128::from(PPM))
        .ok_or_else(|| "institutional statistics ppm numerator overflowed".to_owned())?
        / u128::from(probability.denominator);
    u64::try_from(projected)
        .map_err(|_| "institutional statistics ppm projection does not fit u64".to_owned())
}

fn usize_to_u64(name: &str, value: usize) -> Result<u64, String> {
    u64::try_from(value).map_err(|_| format!("institutional statistics {name} does not fit u64"))
}

fn domain_hash(domain: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    hasher.update(payload);
    hasher.finalize()
}

struct Encoder<'a> {
    bytes: &'a mut [u8],
    cursor: usize,
}

impl<'a> Encoder<'a> {
    const fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), String> {
        let end = self
            .cursor
            .checked_add(value.len())
            .ok_or_else(|| "institutional statistics encoder cursor overflowed".to_owned())?;
        self.bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "institutional statistics encoder exceeded fixed payload".to_owned())?
            .copy_from_slice(value);
        self.cursor = end;
        Ok(())
    }

    fn u32(&mut self, value: u32) -> Result<(), String> {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), String> {
        self.bytes(&value.to_le_bytes())
    }

    fn finish(self) -> Result<(), String> {
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err(format!(
                "institutional statistics encoder wrote {} of {} bytes",
                self.cursor,
                self.bytes.len()
            ))
        }
    }
}

struct Decoder<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Decoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "institutional statistics decoder cursor overflowed".to_owned())?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| "institutional statistics decoder exceeded fixed payload".to_owned())?;
        self.cursor = end;
        Ok(value)
    }

    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().map_err(
            |_| "institutional statistics u32 field differs".to_owned(),
        )?))
    }

    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().map_err(
            |_| "institutional statistics u64 field differs".to_owned(),
        )?))
    }

    fn array_32(&mut self) -> Result<[u8; 32], String> {
        self.take(32)?
            .try_into()
            .map_err(|_| "institutional statistics digest field differs".to_owned())
    }

    fn finish(self) -> Result<(), String> {
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err(format!(
                "institutional statistics decoder consumed {} of {} bytes",
                self.cursor,
                self.bytes.len()
            ))
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes"
)]
mod tests {
    use super::*;
    use runner::bootstrap::romano_wolf_adjusted_p_values_v1;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    fn root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "brutex-institutional-statistics-{label}-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn digest(tag: u8) -> [u8; 32] {
        brutex_core::blake3::hash(&[tag])
    }

    fn prepared(tag: u8) -> PreparedInstitutionalStatisticsV1 {
        let returns = vec![
            vec![5, -3, 7, -2, 9, -4],
            vec![2, -5, 6, -1, 8, -3],
            vec![-2, 4, -1, 5, -3, 6],
        ];
        let adjusted = romano_wolf_adjusted_p_values_v1(&returns, 31, 0x51eed, 2)
            .expect("valid Romano-Wolf fixture");
        let white = Verdict {
            statistic: 1.234_567_890_123_45,
            p_value: 0.187_500_000_000_003,
            draws: 31,
            strategies: returns.len(),
            periods: 6,
        };
        let spa = Verdict {
            statistic: 0.987_654_321_012_345,
            p_value: 0.218_750_000_000_003,
            draws: 31,
            strategies: returns.len(),
            periods: 6,
        };
        prepare_institutional_statistics_v1(
            digest(tag),
            digest(tag.wrapping_add(32)),
            digest(tag.wrapping_add(64)),
            digest(tag.wrapping_add(96)),
            1,
            &white,
            &spa,
            &adjusted,
        )
        .expect("valid institutional statistics fixture")
    }

    fn cleanup(root: &Path) {
        if root.exists() {
            fs::remove_dir_all(root).expect("remove institutional statistics fixture");
        }
    }

    fn append_raw(path: &Path, bytes: &[u8]) {
        let mut file = OpenOptions::new()
            .append(true)
            .open(path)
            .expect("open raw institutional statistics fixture");
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .expect("append raw institutional statistics fixture");
    }

    #[test]
    fn exact_authority_appends_reopens_and_reuses_without_losing_precision() {
        let root = root("roundtrip");
        cleanup(&root);
        let prepared = prepared(1);
        let mut ledger = InstitutionalStatisticsLedgerV1::open(&root, 4)
            .expect("open institutional statistics ledger");
        let appended = ledger
            .append_complete(&prepared)
            .expect("append exact statistics authority");
        assert!(matches!(
            appended,
            InstitutionalStatisticsCommitV1::Appended(_)
        ));
        let authority = appended.authority();
        assert_eq!(authority.population_id(), digest(1));
        assert_eq!(authority.selection_receipt_digest(), digest(33));
        assert_eq!(authority.ranking_policy_digest(), digest(65));
        assert_eq!(authority.strategy_digest(), digest(97));
        assert_eq!(authority.draws(), 31);
        assert_eq!(authority.strategies(), 3);
        assert_eq!(authority.periods(), 6);
        assert_eq!(authority.seed(), 0x51eed);
        assert_eq!(authority.block(), 2);
        assert_eq!(authority.candidate_index(), 1);
        assert_eq!(
            authority.white_statistic().to_bits(),
            1.234_567_890_123_45_f64.to_bits()
        );
        assert_eq!(
            authority.white_p_value().to_bits(),
            0.187_500_000_000_003_f64.to_bits()
        );
        assert_eq!(
            authority.spa_statistic().to_bits(),
            0.987_654_321_012_345_f64.to_bits()
        );
        assert_eq!(
            authority.spa_p_value().to_bits(),
            0.218_750_000_000_003_f64.to_bits()
        );
        let adjusted = authority.adjusted_p_value();
        let fwer = authority.familywise_p_value();
        let adjusted_ppm = u64::try_from(
            u128::from(adjusted.numerator()) * u128::from(PPM) / u128::from(adjusted.denominator()),
        )
        .expect("adjusted ppm fits u64");
        let fwer_ppm = u64::try_from(
            u128::from(fwer.numerator()) * u128::from(PPM) / u128::from(fwer.denominator()),
        )
        .expect("FWER ppm fits u64");
        assert_eq!(authority.romano_wolf_p_value_ppm(), adjusted_ppm);
        assert_eq!(authority.fwer_p_value_ppm(), fwer_ppm);
        assert_eq!(authority.romano_wolf_rejects_at_ppm(PPM + 1), None);
        assert_eq!(
            authority.romano_wolf_rejects_at_ppm(50_000),
            Some(
                u128::from(adjusted.numerator()) * u128::from(PPM)
                    <= u128::from(adjusted.denominator()) * 50_000_u128
            )
        );
        assert!(matches!(
            ledger
                .append_complete(&prepared)
                .expect("reuse exact statistics authority"),
            InstitutionalStatisticsCommitV1::Reused(_)
        ));
        assert_eq!(
            fs::metadata(InstitutionalStatisticsLedgerV1::statistics_path(&root))
                .expect("statistics metadata")
                .len(),
            u64::try_from(HEADER_BYTES + STATISTICS_STRIDE_BYTES).expect("statistics length")
        );
        assert_eq!(
            fs::metadata(InstitutionalStatisticsLedgerV1::completion_path(&root))
                .expect("completion metadata")
                .len(),
            u64::try_from(HEADER_BYTES + COMPLETION_STRIDE_BYTES).expect("completion length")
        );
        drop(ledger);

        let reopened = InstitutionalStatisticsLedgerV1::open_read(&root, 4)
            .expect("reopen exact statistics authority");
        assert_eq!(reopened.completions(), 1);
        let reopened_authority = reopened
            .authority(&authority.subject_id())
            .expect("lookup reopened statistics authority");
        assert_eq!(*reopened_authority, authority);
        reopened_authority
            .require_identity(&digest(1), &digest(33), &digest(65), &digest(97))
            .expect("matching institutional identity");
        assert!(
            reopened_authority
                .require_identity(&digest(2), &digest(33), &digest(65), &digest(97))
                .expect_err("foreign population must refuse")
                .contains("foreign")
        );
        cleanup(&root);
    }

    #[test]
    fn changed_same_subject_and_malformed_sources_refuse() {
        let root = root("identity-refusals");
        cleanup(&root);
        let prepared = prepared(3);
        let mut ledger =
            InstitutionalStatisticsLedgerV1::open(&root, 2).expect("open identity-refusal ledger");
        ledger
            .append_complete(&prepared)
            .expect("commit identity-refusal baseline");
        let mut changed = prepared;
        changed.record.white_p_value_bits = 0.375_f64.to_bits();
        assert!(
            ledger
                .append_complete(&changed)
                .expect_err("changed evidence under same identity must refuse")
                .contains("different evidence")
        );
        let mut malformed = prepared;
        malformed.record.adjusted_p_value.denominator = 0;
        assert!(malformed.record.validate().is_err());

        let returns = vec![vec![1, -1, 2, -2], vec![2, -2, 3, -3]];
        let adjusted = romano_wolf_adjusted_p_values_v1(&returns, 7, 9, 2)
            .expect("valid refusal-family fixture");
        let white = Verdict {
            statistic: f64::NAN,
            p_value: 0.5,
            draws: 7,
            strategies: 2,
            periods: 4,
        };
        let spa = Verdict {
            statistic: 1.0,
            p_value: 0.5,
            draws: 7,
            strategies: 2,
            periods: 4,
        };
        assert!(
            prepare_institutional_statistics_v1(
                [0; 32],
                digest(1),
                digest(2),
                digest(3),
                0,
                &white,
                &spa,
                &adjusted,
            )
            .is_err()
        );
        assert!(
            prepare_institutional_statistics_v1(
                digest(1),
                digest(2),
                digest(3),
                digest(4),
                9,
                &Verdict {
                    statistic: 1.0,
                    ..white
                },
                &spa,
                &adjusted,
            )
            .is_err()
        );
        cleanup(&root);
    }

    #[test]
    fn only_exact_retry_can_complete_a_valid_trailing_orphan() {
        let root = root("orphan");
        cleanup(&root);
        let exact = prepared(5);
        let foreign = prepared(6);
        drop(InstitutionalStatisticsLedgerV1::open(&root, 3).expect("create orphan ledger"));
        append_raw(
            &InstitutionalStatisticsLedgerV1::statistics_path(&root),
            &exact.record.to_bytes().expect("encode exact orphan"),
        );
        let mut reopened =
            InstitutionalStatisticsLedgerV1::open(&root, 3).expect("reopen valid trailing orphan");
        assert_eq!(reopened.completions(), 0);
        assert!(
            reopened
                .append_complete(&foreign)
                .expect_err("foreign retry must not bless orphan")
                .contains("foreign")
        );
        assert!(matches!(
            reopened
                .append_complete(&exact)
                .expect("complete exact orphan retry"),
            InstitutionalStatisticsCommitV1::Appended(_)
        ));
        assert_eq!(reopened.completions(), 1);
        assert_eq!(
            fs::metadata(InstitutionalStatisticsLedgerV1::statistics_path(&root))
                .expect("orphan statistics metadata")
                .len(),
            u64::try_from(HEADER_BYTES + STATISTICS_STRIDE_BYTES).expect("orphan file length")
        );
        cleanup(&root);
    }

    #[test]
    fn torn_corrupt_duplicate_and_reordered_files_refuse() {
        let torn_root = root("torn");
        cleanup(&torn_root);
        drop(InstitutionalStatisticsLedgerV1::open(&torn_root, 2).expect("create torn ledger"));
        append_raw(
            &InstitutionalStatisticsLedgerV1::statistics_path(&torn_root),
            &[0],
        );
        assert!(
            InstitutionalStatisticsLedgerV1::open_read(&torn_root, 2)
                .expect_err("torn statistics row must refuse")
                .contains("torn")
        );
        cleanup(&torn_root);

        let corrupt_root = root("corrupt");
        cleanup(&corrupt_root);
        let corrupt_prepared = prepared(7);
        let mut ledger =
            InstitutionalStatisticsLedgerV1::open(&corrupt_root, 2).expect("open corrupt ledger");
        ledger
            .append_complete(&corrupt_prepared)
            .expect("commit corrupt baseline");
        drop(ledger);
        flip_byte(
            &InstitutionalStatisticsLedgerV1::statistics_path(&corrupt_root),
            u64::try_from(HEADER_BYTES + 12).expect("corrupt offset"),
        );
        assert!(
            InstitutionalStatisticsLedgerV1::open_read(&corrupt_root, 2)
                .expect_err("same-length corruption must refuse")
                .contains("seal")
        );
        cleanup(&corrupt_root);

        let duplicate_root = root("duplicate");
        cleanup(&duplicate_root);
        let duplicate_prepared = prepared(8);
        let mut ledger = InstitutionalStatisticsLedgerV1::open(&duplicate_root, 3)
            .expect("open duplicate ledger");
        ledger
            .append_complete(&duplicate_prepared)
            .expect("commit duplicate baseline");
        drop(ledger);
        append_raw(
            &InstitutionalStatisticsLedgerV1::statistics_path(&duplicate_root),
            &duplicate_prepared
                .record
                .to_bytes()
                .expect("encode duplicate row"),
        );
        assert!(
            InstitutionalStatisticsLedgerV1::open_read(&duplicate_root, 3)
                .expect_err("duplicate subject must refuse")
                .contains("duplicate")
        );
        cleanup(&duplicate_root);

        let reordered_root = root("reordered");
        cleanup(&reordered_root);
        let first = prepared(9);
        let second = prepared(10);
        let mut ledger = InstitutionalStatisticsLedgerV1::open(&reordered_root, 3)
            .expect("open reordered ledger");
        ledger
            .append_complete(&first)
            .expect("commit first reordered row");
        ledger
            .append_complete(&second)
            .expect("commit second reordered row");
        drop(ledger);
        swap_completion_rows(&InstitutionalStatisticsLedgerV1::completion_path(
            &reordered_root,
        ));
        assert!(
            InstitutionalStatisticsLedgerV1::open_read(&reordered_root, 3)
                .expect_err("reordered completions must refuse")
                .contains("references row")
        );
        cleanup(&reordered_root);
    }

    #[test]
    fn stale_handles_refuse_length_and_same_length_changes() {
        let length_root = root("stale-length");
        cleanup(&length_root);
        let mut first = InstitutionalStatisticsLedgerV1::open(&length_root, 4)
            .expect("open first stale-length handle");
        let mut stale = InstitutionalStatisticsLedgerV1::open(&length_root, 4)
            .expect("open second stale-length handle");
        first
            .append_complete(&prepared(11))
            .expect("append behind stale-length handle");
        assert!(
            stale
                .append_complete(&prepared(12))
                .expect_err("stale length handle must refuse")
                .contains("changed behind")
        );
        cleanup(&length_root);

        let content_root = root("stale-content");
        cleanup(&content_root);
        let mut ledger = InstitutionalStatisticsLedgerV1::open(&content_root, 4)
            .expect("open stale-content handle");
        flip_byte(
            &InstitutionalStatisticsLedgerV1::statistics_path(&content_root),
            4,
        );
        assert!(
            ledger
                .append_complete(&prepared(13))
                .expect_err("same-length stale content must refuse")
                .contains("changed behind")
        );
        cleanup(&content_root);
    }

    fn flip_byte(path: &Path, offset: u64) {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("open byte-flip fixture");
        file.seek(SeekFrom::Start(offset))
            .expect("seek byte-flip fixture");
        let mut byte = [0_u8; 1];
        file.read_exact(&mut byte).expect("read byte-flip fixture");
        let value = byte
            .first_mut()
            .expect("one-byte fixture must contain one byte");
        *value ^= 0x80;
        file.seek(SeekFrom::Start(offset))
            .and_then(|_| file.write_all(&byte))
            .and_then(|()| file.sync_all())
            .expect("write byte-flip fixture");
    }

    fn swap_completion_rows(path: &Path) {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("open completion-swap fixture");
        let mut first = [0_u8; COMPLETION_STRIDE_BYTES];
        let mut second = [0_u8; COMPLETION_STRIDE_BYTES];
        file.seek(SeekFrom::Start(
            u64::try_from(HEADER_BYTES).expect("header offset"),
        ))
        .and_then(|_| file.read_exact(&mut first))
        .and_then(|()| file.read_exact(&mut second))
        .expect("read completion-swap rows");
        file.seek(SeekFrom::Start(
            u64::try_from(HEADER_BYTES).expect("header offset"),
        ))
        .and_then(|_| file.write_all(&second))
        .and_then(|()| file.write_all(&first))
        .and_then(|()| file.sync_all())
        .expect("write completion-swap rows");
    }
}
