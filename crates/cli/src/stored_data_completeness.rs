//! Durable proof that one Population V4 used complete stored input streams.
//!
//! Population V4 already seals complete signal-rung and independent one-minute
//! calendar receipts.  Its population identity also carries the composite data
//! digest used by the stored anchored run.  Neither fact, by itself, proves
//! which signal, minute-context and daily-reference bytes produced that digest.
//! This module closes that specific gap with a new append-only receipt.  The
//! receipt is prepared only after recomputing the three-stream digest from the
//! exact bytes, rebuilding both requested-span calendar receipts from the
//! supplied timestamps, and proving that the evaluated one-minute slice is a
//! contiguous byte-for-byte subslice of the complete minute context.
//!
//! The receipt is not a vendor-integrity or checksum-scrub claim.  Integrity
//! states are copied from [`runner::identity::DailyReferenceBinding`] and enter
//! both the composite data digest and the daily-reference policy identity.
//! With today's ordinary stored reader those states remain explicitly
//! `UnverifiedNoReceipt`.
//!
//! # Cost
//!
//! Preparation is O(S + M + D + E + calendar days): every supplied signal,
//! minute-context, daily and evaluated-execution record is hashed or compared
//! at least once.  Calendar reconstruction currently allocates timestamp
//! vectors proportional to S and E because the canonical calendar authority
//! accepts timestamp slices.  Ledger open and stale-handle checks hash the
//! complete receipt file.  None of these operations is a per-bar sweep inner
//! loop or one of the five constant-per-operation primitives in `AGENTS.md`.

use brutex_core::blake3::{Hasher, hash};
use indicators::Candle;
use runner::identity::{DailyReferenceBinding, data_digest, data_digest_with_daily_reference};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::population::{CompletionReceiptV4, InstrumentFamilyV1, RequestedSpanIdentityV1};
use crate::population_admission_writer::{CompletePopulationAuthorityV1, derive_population_id_v1};
use crate::stored::calendar_receipt_v2;

/// Operator-facing refusal from the stored-data completeness boundary.
pub type StoredDataCompletenessRefusal = String;

const RECEIPT_VERSION_V1: u32 = 1;
const RECEIPT_CONTENT_DOMAIN_V1: &[u8] = b"brutex.stored-data-completeness.receipt.v1\0";
const DAILY_POLICY_DOMAIN_V1: &[u8] = b"brutex.stored-data-completeness.daily-policy.v1\0";
const ELIGIBILITY_DOMAIN_V1: &[u8] = b"brutex.stored-data-completeness.eligibility.v1\0";
const EXCLUDED_DAYS_DOMAIN_V1: &[u8] = b"brutex.stored-data-completeness.excluded-days.v1\0";
const FILE_DIGEST_DOMAIN_V1: &[u8] = b"brutex.stored-data-completeness.file.v1\0";
const HEADER_MAGIC_V1: [u8; 16] = *b"BTX-DATA-COMP-V1";
const HEADER_VERSION_V1: u32 = 1;
const HEADER_PAYLOAD_BYTES: usize = 32;
const HEADER_BYTES: usize = 64;
const RECEIPT_PAYLOAD_BYTES: usize = 644;
const RECEIPT_STRIDE_BYTES: usize = RECEIPT_PAYLOAD_BYTES + 32;
const MAX_STREAM_RECORDS: u64 = i64::MAX as u64;

/// Canonical identity of the daily-reference interpretation policy.
///
/// Eligibility decisions and market-data bytes are intentionally absent: they
/// are data and enter the composite run digest separately.  This identity binds
/// the schema, eligibility rule, exact-minute overlay rule, ordered excluded
/// IST-day list, both explicitly supplied integrity states, and the
/// swept-series calendar policy.
///
/// # The last term is APPENDED, and it was once missing
///
/// `swept_series_calendar_policy` reached the composite run identity as tag 11
/// of `data_digest_with_daily_reference` before it reached this function, and
/// for that interval two populations differing ONLY in that policy produced the
/// same `daily_reference_policy_digest` — the exact defect the field's own doc
/// comment argues against. The composite digest did distinguish them, so no run
/// identity ever collided; the POLICY identity alone did not, and this function
/// is what a sealed population is later re-checked against.
///
/// It is hashed after the excluded-day list, so every term above it keeps the
/// position it already had. Only the digest VALUE changes; the receipt payload
/// is a fixed 32 bytes either way and `RECEIPT_PAYLOAD_BYTES` is untouched.
#[must_use]
pub fn daily_reference_policy_digest_v1(reference: DailyReferenceBinding<'_>) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(DAILY_POLICY_DOMAIN_V1);
    hasher.update(&reference.schema.to_le_bytes());
    hasher.update(&reference.eligibility_policy.to_le_bytes());
    hasher.update(&reference.gap_overlay_policy.to_le_bytes());
    hasher.update(&[reference.daily_integrity.byte()]);
    hasher.update(&[reference.minute_integrity.byte()]);
    hasher.update(
        &u64::try_from(reference.excluded_ist_days.len())
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
    for day in reference.excluded_ist_days {
        hasher.update(&day.to_le_bytes());
    }
    hasher.update(&reference.swept_series_calendar_policy.to_le_bytes());
    hasher.finalize()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StreamFactsV1 {
    count: u64,
    first_ts_micros: i64,
    last_ts_micros: i64,
    digest: [u8; 32],
}

impl StreamFactsV1 {
    fn of(name: &str, bars: &[Candle]) -> Result<Self, StoredDataCompletenessRefusal> {
        let first = bars
            .first()
            .ok_or_else(|| format!("stored-data completeness {name} stream is empty"))?;
        let last = bars
            .last()
            .ok_or_else(|| format!("stored-data completeness {name} stream is empty"))?;
        let count = u64::try_from(bars.len())
            .map_err(|_| format!("stored-data completeness {name} count does not fit u64"))?;
        if count > MAX_STREAM_RECORDS {
            return Err(format!(
                "stored-data completeness {name} count {count} exceeds the fixed signed-range codec bound"
            ));
        }
        for (index, (left, right)) in bars.iter().zip(bars.iter().skip(1)).enumerate() {
            if left.ts_micros >= right.ts_micros {
                return Err(format!(
                    "stored-data completeness {name} timestamps are not strictly increasing at pair {index}"
                ));
            }
        }
        Ok(Self {
            count,
            first_ts_micros: first.ts_micros,
            last_ts_micros: last.ts_micros,
            digest: data_digest(bars),
        })
    }

    fn validate(self, name: &str) -> Result<(), StoredDataCompletenessRefusal> {
        if self.count == 0 || self.count > MAX_STREAM_RECORDS {
            return Err(format!(
                "stored-data completeness {name} count {} is outside 1..={MAX_STREAM_RECORDS}",
                self.count
            ));
        }
        if self.first_ts_micros > self.last_ts_micros {
            return Err(format!(
                "stored-data completeness {name} range {}..={} runs backward",
                self.first_ts_micros, self.last_ts_micros
            ));
        }
        require_digest(name, &self.digest)
    }

    fn encode(self, encoder: &mut Encoder<'_>) -> Result<(), StoredDataCompletenessRefusal> {
        encoder.u64(self.count)?;
        encoder.i64(self.first_ts_micros)?;
        encoder.i64(self.last_ts_micros)?;
        encoder.bytes(&self.digest)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, StoredDataCompletenessRefusal> {
        Ok(Self {
            count: decoder.u64()?,
            first_ts_micros: decoder.i64()?,
            last_ts_micros: decoder.i64()?,
            digest: decoder.array_32()?,
        })
    }
}

/// One sealed, fixed-stride proof of exact stored input reconciliation.
///
/// Fields are private.  A value can be created only by the byte-recomputing
/// preparation door or by decoding a sealed append-only record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredDataCompletenessReceiptV1 {
    version: u32,
    population_id: [u8; 32],
    population_v4_digest: [u8; 32],
    instrument_family: InstrumentFamilyV1,
    rung_seconds: u32,
    requested_span: RequestedSpanIdentityV1,
    signal: StreamFactsV1,
    minute_context: StreamFactsV1,
    execution: StreamFactsV1,
    daily: StreamFactsV1,
    eligibility_count: u64,
    eligibility_digest: [u8; 32],
    data_digest: [u8; 32],
    feed_digest: [u8; 32],
    source_commit_digest: [u8; 32],
    calendar_policy_digest: [u8; 32],
    daily_reference_policy_digest: [u8; 32],
    signal_calendar_receipt_digest: [u8; 32],
    execution_calendar_receipt_digest: [u8; 32],
    daily_schema: u32,
    eligibility_policy: u32,
    gap_overlay_policy: u32,
    daily_integrity: u8,
    minute_integrity: u8,
    excluded_days_count: u64,
    excluded_days_digest: [u8; 32],
}

impl StoredDataCompletenessReceiptV1 {
    /// Population identity whose exact V4 completion this receipt extends.
    #[must_use]
    pub const fn population_id(self) -> [u8; 32] {
        self.population_id
    }

    /// Digest of the exact Population V4 completion payload.
    #[must_use]
    pub const fn population_v4_digest(self) -> [u8; 32] {
        self.population_v4_digest
    }

    /// Exact three-stream data digest reconciled during preparation.
    #[must_use]
    pub const fn data_digest(self) -> [u8; 32] {
        self.data_digest
    }

    /// Number of exact signal-rung records hashed into the receipt.
    #[must_use]
    pub const fn signal_records(self) -> u64 {
        self.signal.count
    }

    /// Number of full one-minute context records hashed into the receipt.
    #[must_use]
    pub const fn minute_context_records(self) -> u64 {
        self.minute_context.count
    }

    /// Number of requested-span one-minute execution records.
    #[must_use]
    pub const fn execution_records(self) -> u64 {
        self.execution.count
    }

    /// Number of stored one-day reference records.
    #[must_use]
    pub const fn daily_records(self) -> u64 {
        self.daily.count
    }

    /// Domain-separated digest of the exact canonical receipt payload.
    ///
    /// # Errors
    ///
    /// Refuses any malformed decoded semantic field.
    pub fn content_digest(self) -> Result<[u8; 32], StoredDataCompletenessRefusal> {
        let payload = self.payload_bytes()?;
        let mut hasher = Hasher::new();
        hasher.update(RECEIPT_CONTENT_DOMAIN_V1);
        hasher.update(&payload);
        Ok(hasher.finalize())
    }

    fn validate(self) -> Result<(), StoredDataCompletenessRefusal> {
        if self.version != RECEIPT_VERSION_V1 {
            return Err(format!(
                "stored-data completeness receipt version {} is unknown; expected {RECEIPT_VERSION_V1}",
                self.version
            ));
        }
        require_digest("population identity", &self.population_id)?;
        require_digest("Population V4 completion", &self.population_v4_digest)?;
        validate_rung(self.rung_seconds)?;
        let canonical_span = RequestedSpanIdentityV1::new(
            self.requested_span.from_year(),
            self.requested_span.from_month(),
            self.requested_span.to_year(),
            self.requested_span.to_month(),
        )?;
        if canonical_span != self.requested_span {
            return Err("stored-data completeness requested span is not canonical".to_owned());
        }
        self.signal.validate("signal stream digest")?;
        self.minute_context
            .validate("minute-context stream digest")?;
        self.execution.validate("execution stream digest")?;
        self.daily.validate("daily-reference stream digest")?;
        if self.execution.count > self.minute_context.count
            || self.execution.first_ts_micros < self.minute_context.first_ts_micros
            || self.execution.last_ts_micros > self.minute_context.last_ts_micros
        {
            return Err(
                "stored-data completeness execution range is outside its full minute context"
                    .to_owned(),
            );
        }
        if self.eligibility_count != self.daily.count {
            return Err(format!(
                "stored-data completeness eligibility count {} differs from daily count {}",
                self.eligibility_count, self.daily.count
            ));
        }
        for (name, digest) in [
            ("eligibility digest", self.eligibility_digest),
            ("three-stream data digest", self.data_digest),
            ("feed digest", self.feed_digest),
            ("source commit digest", self.source_commit_digest),
            ("calendar policy digest", self.calendar_policy_digest),
            (
                "daily-reference policy digest",
                self.daily_reference_policy_digest,
            ),
            (
                "signal calendar receipt digest",
                self.signal_calendar_receipt_digest,
            ),
            (
                "execution calendar receipt digest",
                self.execution_calendar_receipt_digest,
            ),
            ("excluded IST days digest", self.excluded_days_digest),
        ] {
            require_digest(name, &digest)?;
        }
        if self.daily_integrity > 0 || self.minute_integrity > 0 {
            return Err(
                "stored-data completeness receipt carries an unknown integrity-evidence byte"
                    .to_owned(),
            );
        }
        if self.excluded_days_count > MAX_STREAM_RECORDS {
            return Err(format!(
                "stored-data completeness excluded-day count {} exceeds the codec bound",
                self.excluded_days_count
            ));
        }
        Ok(())
    }

    fn payload_bytes(self) -> Result<[u8; RECEIPT_PAYLOAD_BYTES], StoredDataCompletenessRefusal> {
        self.validate()?;
        let mut raw = [0_u8; RECEIPT_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.u32(self.version)?;
        encoder.u32(0)?;
        encoder.bytes(&self.population_id)?;
        encoder.bytes(&self.population_v4_digest)?;
        encoder.u8(family_byte(self.instrument_family))?;
        encoder.zeros(3)?;
        encoder.u32(self.rung_seconds)?;
        encoder.bytes(&self.requested_span.canonical_bytes())?;
        self.signal.encode(&mut encoder)?;
        self.minute_context.encode(&mut encoder)?;
        self.execution.encode(&mut encoder)?;
        self.daily.encode(&mut encoder)?;
        encoder.u64(self.eligibility_count)?;
        encoder.bytes(&self.eligibility_digest)?;
        for digest in [
            self.data_digest,
            self.feed_digest,
            self.source_commit_digest,
            self.calendar_policy_digest,
            self.daily_reference_policy_digest,
            self.signal_calendar_receipt_digest,
            self.execution_calendar_receipt_digest,
        ] {
            encoder.bytes(&digest)?;
        }
        encoder.u32(self.daily_schema)?;
        encoder.u32(self.eligibility_policy)?;
        encoder.u32(self.gap_overlay_policy)?;
        encoder.u8(self.daily_integrity)?;
        encoder.u8(self.minute_integrity)?;
        encoder.zeros(2)?;
        encoder.u64(self.excluded_days_count)?;
        encoder.bytes(&self.excluded_days_digest)?;
        encoder.finish()?;
        Ok(raw)
    }

    fn to_bytes(self) -> Result<[u8; RECEIPT_STRIDE_BYTES], StoredDataCompletenessRefusal> {
        let payload = self.payload_bytes()?;
        let mut raw = [0_u8; RECEIPT_STRIDE_BYTES];
        raw.get_mut(..RECEIPT_PAYLOAD_BYTES)
            .ok_or_else(|| "stored-data receipt payload slot is absent".to_owned())?
            .copy_from_slice(&payload);
        raw.get_mut(RECEIPT_PAYLOAD_BYTES..)
            .ok_or_else(|| "stored-data receipt seal slot is absent".to_owned())?
            .copy_from_slice(&hash(&payload));
        Ok(raw)
    }

    fn from_bytes(raw: &[u8; RECEIPT_STRIDE_BYTES]) -> Result<Self, StoredDataCompletenessRefusal> {
        let payload = raw
            .get(..RECEIPT_PAYLOAD_BYTES)
            .ok_or_else(|| "stored-data completeness payload is absent".to_owned())?;
        let stored_seal = raw
            .get(RECEIPT_PAYLOAD_BYTES..)
            .ok_or_else(|| "stored-data completeness seal is absent".to_owned())?;
        if stored_seal != hash(payload) {
            return Err("stored-data completeness receipt failed its BLAKE3 seal".to_owned());
        }
        let mut decoder = Decoder::new(payload);
        let version = decoder.u32()?;
        decoder.zeros(4, "receipt header reserve")?;
        let population_id = decoder.array_32()?;
        let population_v4_digest = decoder.array_32()?;
        let instrument_family = family_from_byte(decoder.u8()?)?;
        decoder.zeros(3, "instrument-family reserve")?;
        let rung_seconds = decoder.u32()?;
        let span = decoder.bytes::<20>()?;
        let requested_span = span_from_bytes(&span)?;
        let signal = StreamFactsV1::decode(&mut decoder)?;
        let minute_context = StreamFactsV1::decode(&mut decoder)?;
        let execution = StreamFactsV1::decode(&mut decoder)?;
        let daily = StreamFactsV1::decode(&mut decoder)?;
        let eligibility_count = decoder.u64()?;
        let eligibility_digest = decoder.array_32()?;
        let data_digest = decoder.array_32()?;
        let feed_digest = decoder.array_32()?;
        let source_commit_digest = decoder.array_32()?;
        let calendar_policy_digest = decoder.array_32()?;
        let daily_reference_policy_digest = decoder.array_32()?;
        let signal_calendar_receipt_digest = decoder.array_32()?;
        let execution_calendar_receipt_digest = decoder.array_32()?;
        let daily_schema = decoder.u32()?;
        let eligibility_policy = decoder.u32()?;
        let gap_overlay_policy = decoder.u32()?;
        let daily_integrity = decoder.u8()?;
        let minute_integrity = decoder.u8()?;
        decoder.zeros(2, "integrity reserve")?;
        let excluded_days_count = decoder.u64()?;
        let excluded_days_digest = decoder.array_32()?;
        decoder.finish()?;
        let receipt = Self {
            version,
            population_id,
            population_v4_digest,
            instrument_family,
            rung_seconds,
            requested_span,
            signal,
            minute_context,
            execution,
            daily,
            eligibility_count,
            eligibility_digest,
            data_digest,
            feed_digest,
            source_commit_digest,
            calendar_policy_digest,
            daily_reference_policy_digest,
            signal_calendar_receipt_digest,
            execution_calendar_receipt_digest,
            daily_schema,
            eligibility_policy,
            gap_overlay_policy,
            daily_integrity,
            minute_integrity,
            excluded_days_count,
            excluded_days_digest,
        };
        receipt.validate()?;
        Ok(receipt)
    }
}

/// Fully verified receipt bytes awaiting durable append.
///
/// This type is deliberately not accepted by institutional evidence.  Only the
/// post-commit [`StoredDataCompletenessAuthorityV1`] can produce `Complete`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreparedStoredDataCompletenessV1 {
    receipt: StoredDataCompletenessReceiptV1,
}

impl PreparedStoredDataCompletenessV1 {
    /// Exact receipt that will be byte-compared on an idempotent retry.
    #[must_use]
    pub const fn receipt(self) -> StoredDataCompletenessReceiptV1 {
        self.receipt
    }
}

/// Durable typed capability returned only after a sealed ledger validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredDataCompletenessAuthorityV1 {
    receipt: StoredDataCompletenessReceiptV1,
}

impl StoredDataCompletenessAuthorityV1 {
    /// Exact durable receipt carried by this capability.
    #[must_use]
    pub const fn receipt(self) -> StoredDataCompletenessReceiptV1 {
        self.receipt
    }

    /// Population identity this capability completes.
    #[must_use]
    pub const fn population_id(self) -> [u8; 32] {
        self.receipt.population_id
    }

    /// Reconciles this durable capability with one semantic population source.
    ///
    /// # Errors
    ///
    /// Refuses every population, family, rung, span, data/feed/commit/calendar,
    /// daily-policy, calendar-receipt or exact execution-series mismatch.
    pub fn require_population(
        self,
        population: &CompletePopulationAuthorityV1<'_>,
    ) -> Result<(), StoredDataCompletenessRefusal> {
        let derived = derive_population_id_v1(population).map_err(|why| {
            format!("stored-data authority could not validate population source: {why}")
        })?;
        if derived != self.receipt.population_id {
            return Err("stored-data authority belongs to another population identity".to_owned());
        }
        let receipt = self.receipt;
        if receipt.instrument_family != population.instrument_family
            || receipt.rung_seconds != population.rung_seconds
            || receipt.requested_span != population.requested_span
        {
            return Err(
                "stored-data authority family/rung/requested span differs from its population"
                    .to_owned(),
            );
        }
        let identities = population.identities;
        if (
            receipt.data_digest,
            receipt.feed_digest,
            receipt.source_commit_digest,
        ) != (
            identities.data_digest,
            identities.feed_digest,
            identities.source_commit_digest,
        ) {
            return Err("stored-data authority data/feed/commit identity differs".to_owned());
        }
        if receipt.calendar_policy_digest != identities.calendar_policy_digest
            || receipt.daily_reference_policy_digest != identities.daily_reference_policy_digest
        {
            return Err("stored-data authority calendar/daily policy identity differs".to_owned());
        }
        if receipt.signal_calendar_receipt_digest != population.signal_calendar.digest()
            || receipt.execution_calendar_receipt_digest != population.execution_calendar.digest()
        {
            return Err("stored-data authority complete-calendar receipt differs".to_owned());
        }
        let execution =
            StreamFactsV1::of("population execution", population.execution_series.bars())?;
        if execution != receipt.execution {
            return Err("stored-data authority exact execution bytes differ".to_owned());
        }
        Ok(())
    }
}

/// Recomputes one complete stored-input receipt from exact source bytes.
///
/// `minute_context` is the complete one-minute stream used by the daily/GapFib
/// join, including warm-up.  The exact requested-span execution slice comes
/// from `population.execution_series` and must occur contiguously inside it.
///
/// # Errors
///
/// Refuses malformed streams/bindings, a foreign Population V4 receipt,
/// incomplete or mismatched calendars, a non-contiguous execution slice, any
/// component/data/policy digest disagreement, or a population authority that
/// fails its own complete validation.
pub fn prepare_stored_data_completeness_v1(
    population_receipt: CompletionReceiptV4,
    population: &CompletePopulationAuthorityV1<'_>,
    signal_bars: &[Candle],
    minute_context: &[Candle],
    daily_reference: DailyReferenceBinding<'_>,
) -> Result<PreparedStoredDataCompletenessV1, StoredDataCompletenessRefusal> {
    let population_id = derive_population_id_v1(population)
        .map_err(|why| format!("stored-data completeness population authority refused: {why}"))?;
    require_population_receipt(&population_receipt, population, population_id)?;
    let signal = StreamFactsV1::of("signal", signal_bars)?;
    let minute_context_facts = StreamFactsV1::of("minute context", minute_context)?;
    let execution_bars = population.execution_series.bars();
    let execution = StreamFactsV1::of("evaluated execution", execution_bars)?;
    let daily = StreamFactsV1::of("daily reference", daily_reference.daily_bars)?;
    require_exact_subslice(minute_context, execution_bars)?;
    let (eligibility_count, excluded_days_count) = daily_reference_counts(daily_reference)?;
    let (first_day, last_day) = (
        population.signal_calendar.first_day(),
        population.signal_calendar.last_day(),
    );
    let signal_timestamps = signal_bars
        .iter()
        .map(|bar| bar.ts_micros)
        .collect::<Vec<_>>();
    let execution_timestamps = execution_bars
        .iter()
        .map(|bar| bar.ts_micros)
        .collect::<Vec<_>>();
    let rebuilt_signal = calendar_receipt_v2(
        &signal_timestamps,
        population.rung_seconds,
        first_day,
        last_day,
    )?
    .require_complete()?;
    let rebuilt_execution =
        calendar_receipt_v2(&execution_timestamps, 60, first_day, last_day)?.require_complete()?;
    if rebuilt_signal != population.signal_calendar {
        return Err(
            "stored signal timestamps do not reproduce the Population V4 complete calendar receipt"
                .to_owned(),
        );
    }
    if rebuilt_execution != population.execution_calendar {
        return Err(
            "stored execution timestamps do not reproduce the Population V4 complete calendar receipt"
                .to_owned(),
        );
    }
    let computed_data =
        data_digest_with_daily_reference(signal_bars, minute_context, daily_reference)
            .map_err(|why| format!("stored three-stream data digest refused: {why:?}"))?;
    if computed_data != population.identities.data_digest {
        return Err(
            "stored signal/minute/daily bytes do not reproduce Population V4 data_digest"
                .to_owned(),
        );
    }
    let policy_digest = daily_reference_policy_digest_v1(daily_reference);
    if policy_digest != population.identities.daily_reference_policy_digest {
        return Err(
            "stored daily-reference policy terms do not reproduce Population V4 policy identity"
                .to_owned(),
        );
    }
    let receipt = StoredDataCompletenessReceiptV1 {
        version: RECEIPT_VERSION_V1,
        population_id,
        population_v4_digest: population_receipt.content_digest()?,
        instrument_family: population.instrument_family,
        rung_seconds: population.rung_seconds,
        requested_span: population.requested_span,
        signal,
        minute_context: minute_context_facts,
        execution,
        daily,
        eligibility_count,
        eligibility_digest: eligibility_digest(daily_reference.eligibility),
        data_digest: computed_data,
        feed_digest: population.identities.feed_digest,
        source_commit_digest: population.identities.source_commit_digest,
        calendar_policy_digest: population.identities.calendar_policy_digest,
        daily_reference_policy_digest: policy_digest,
        signal_calendar_receipt_digest: rebuilt_signal.digest(),
        execution_calendar_receipt_digest: rebuilt_execution.digest(),
        daily_schema: daily_reference.schema,
        eligibility_policy: daily_reference.eligibility_policy,
        gap_overlay_policy: daily_reference.gap_overlay_policy,
        daily_integrity: daily_reference.daily_integrity.byte(),
        minute_integrity: daily_reference.minute_integrity.byte(),
        excluded_days_count,
        excluded_days_digest: digest_days(daily_reference.excluded_ist_days),
    };
    receipt.validate()?;
    Ok(PreparedStoredDataCompletenessV1 { receipt })
}

/// Outcome of an idempotent stored-data receipt commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoredDataCompletenessCommitV1 {
    /// New sealed bytes were durably appended.
    Written,
    /// The exact already-durable receipt was fully revalidated and reused.
    Reused,
}

/// Append-only fixed-stride stored-data completeness ledger.
#[derive(Debug)]
pub struct StoredDataCompletenessLedgerV1 {
    file: File,
    path: PathBuf,
    receipts: HashMap<[u8; 32], StoredDataCompletenessReceiptV1>,
    generation: FileSnapshotV1,
    max_receipts: usize,
    writable: bool,
}

impl StoredDataCompletenessLedgerV1 {
    /// Version-one ledger path.  No earlier file is consulted or upgraded.
    #[must_use]
    pub fn path(root: &Path) -> PathBuf {
        root.join("results/stored-data-completeness-v1.bin")
    }

    /// Opens or creates a writable ledger under an explicit receipt ceiling.
    ///
    /// # Errors
    ///
    /// Refuses zero bounds, I/O/locking failures, malformed headers/records,
    /// duplicate population identities, ragged tails or a file over the bound.
    pub fn open(root: &Path, max_receipts: usize) -> Result<Self, StoredDataCompletenessRefusal> {
        validate_max(max_receipts)?;
        let path = Self::path(root);
        let parent = path
            .parent()
            .ok_or_else(|| "stored-data completeness path has no parent".to_owned())?;
        std::fs::create_dir_all(parent).map_err(|why| {
            format!(
                "stored-data completeness directory {} could not be created: {why}",
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
        Self::open_file(file, path, max_receipts, true)
    }

    /// Opens and fully validates an existing ledger without write capability.
    ///
    /// # Errors
    ///
    /// Every structural refusal from [`Self::open`], plus absence or emptiness.
    pub fn open_read(
        root: &Path,
        max_receipts: usize,
    ) -> Result<Self, StoredDataCompletenessRefusal> {
        validate_max(max_receipts)?;
        let path = Self::path(root);
        let file = File::open(&path)
            .map_err(|why| format!("{} could not be opened read-only: {why}", path.display()))?;
        Self::open_file(file, path, max_receipts, false)
    }

    fn open_file(
        mut file: File,
        path: PathBuf,
        max_receipts: usize,
        writable: bool,
    ) -> Result<Self, StoredDataCompletenessRefusal> {
        if writable {
            file.lock()
                .map_err(|why| format!("{} could not be locked: {why}", path.display()))?;
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
            let receipts = scan_file(&mut file, &path, max_receipts)?;
            let generation = snapshot(&mut file, &path)?;
            Ok((receipts, generation))
        })();
        let released = file
            .unlock()
            .map_err(|why| format!("{} could not be unlocked: {why}", path.display()));
        match (opened, released) {
            (Ok((receipts, generation)), Ok(())) => Ok(Self {
                file,
                path,
                receipts,
                generation,
                max_receipts,
                writable,
            }),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// Number of validated durable receipts.
    #[must_use]
    pub fn receipts(&self) -> usize {
        self.receipts.len()
    }

    /// Returns one durable typed authority after rechecking file generation.
    ///
    /// # Errors
    ///
    /// Refuses a stale/replaced/mutated file or an unknown population identity.
    pub fn authority(
        &mut self,
        population_id: [u8; 32],
    ) -> Result<StoredDataCompletenessAuthorityV1, StoredDataCompletenessRefusal> {
        self.file
            .lock_shared()
            .map_err(|why| format!("{} could not be shared-locked: {why}", self.path.display()))?;
        let checked = (|| {
            self.require_unchanged()?;
            self.receipts
                .get(&population_id)
                .copied()
                .map(|receipt| StoredDataCompletenessAuthorityV1 { receipt })
                .ok_or_else(|| {
                    "stored-data completeness population identity is not committed".to_owned()
                })
        })();
        let released = self
            .file
            .unlock()
            .map_err(|why| format!("{} could not be unlocked: {why}", self.path.display()));
        match (checked, released) {
            (Ok(authority), Ok(())) => Ok(authority),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// Appends or byte-identically reuses one prepared receipt.
    ///
    /// # Errors
    ///
    /// Refuses read-only use, stale/replaced bytes, bound exhaustion, a
    /// same-population conflict, encoding, write/sync/rollback or lock failure.
    pub fn commit(
        &mut self,
        prepared: PreparedStoredDataCompletenessV1,
    ) -> Result<
        (
            StoredDataCompletenessCommitV1,
            StoredDataCompletenessAuthorityV1,
        ),
        StoredDataCompletenessRefusal,
    > {
        if !self.writable {
            return Err("read-only stored-data completeness ledger cannot commit".to_owned());
        }
        prepared.receipt.validate()?;
        self.file
            .lock()
            .map_err(|why| format!("{} could not be locked: {why}", self.path.display()))?;
        let committed = (|| {
            self.require_unchanged()?;
            let receipt = prepared.receipt;
            if let Some(existing) = self.receipts.get(&receipt.population_id).copied() {
                if existing != receipt {
                    return Err(
                        "stored-data completeness population already names different canonical bytes"
                            .to_owned(),
                    );
                }
                return Ok((
                    StoredDataCompletenessCommitV1::Reused,
                    StoredDataCompletenessAuthorityV1 { receipt: existing },
                ));
            }
            if self.receipts.len() >= self.max_receipts {
                return Err(format!(
                    "stored-data completeness ledger reached its explicit {}-receipt bound",
                    self.max_receipts
                ));
            }
            self.receipts.try_reserve(1).map_err(|why| {
                format!("stored-data completeness index could not reserve one slot: {why}")
            })?;
            let raw = receipt.to_bytes()?;
            let start = self
                .file
                .seek(SeekFrom::End(0))
                .map_err(|why| format!("stored-data completeness append seek failed: {why}"))?;
            if let Err(why) = self.file.write_all(&raw) {
                return Err(rollback_message(&self.file, start, &why));
            }
            if let Err(why) = self.file.sync_all() {
                return Err(format!(
                    "stored-data completeness receipt was written but could not be synced: {why}"
                ));
            }
            self.generation = snapshot(&mut self.file, &self.path)?;
            self.receipts.insert(receipt.population_id, receipt);
            Ok((
                StoredDataCompletenessCommitV1::Written,
                StoredDataCompletenessAuthorityV1 { receipt },
            ))
        })();
        let released = self
            .file
            .unlock()
            .map_err(|why| format!("{} could not be unlocked: {why}", self.path.display()));
        match (committed, released) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn require_unchanged(&mut self) -> Result<(), StoredDataCompletenessRefusal> {
        let observed = snapshot(&mut self.file, &self.path)?;
        if observed != self.generation {
            return Err(format!(
                "{} changed since validation; reopen before using cached stored-data completeness authority",
                self.path.display()
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileSnapshotV1 {
    len: u64,
    digest: [u8; 32],
}

fn require_population_receipt(
    receipt: &CompletionReceiptV4,
    population: &CompletePopulationAuthorityV1<'_>,
    population_id: [u8; 32],
) -> Result<(), StoredDataCompletenessRefusal> {
    if receipt.population_id() != population_id {
        return Err("stored-data completeness received a foreign Population V4 receipt".to_owned());
    }
    let v3 = receipt.v3();
    let v2 = v3.v2();
    if v2.instrument_family != population.instrument_family
        || v2.rung_seconds != population.rung_seconds
        || v3.requested_span() != population.requested_span
        || v2.identities != population.identities
    {
        return Err(
            "stored-data completeness Population V4 receipt differs from its semantic authority"
                .to_owned(),
        );
    }
    let coverage = receipt.coverage();
    if coverage.signal_complete_receipt_digest() != population.signal_calendar.digest()
        || coverage.execution_complete_receipt_digest() != population.execution_calendar.digest()
        || coverage.signal_rung_seconds() != population.rung_seconds
        || coverage.execution_rung_seconds() != 60
        || (coverage.first_day(), coverage.last_day())
            != (
                population.signal_calendar.first_day(),
                population.signal_calendar.last_day(),
            )
    {
        return Err(
            "stored-data completeness Population V4 calendar coverage differs from its semantic authority"
                .to_owned(),
        );
    }
    Ok(())
}

fn require_exact_subslice(
    context: &[Candle],
    execution: &[Candle],
) -> Result<(), StoredDataCompletenessRefusal> {
    if execution.len() > context.len() {
        return Err(
            "stored-data completeness execution slice is longer than its minute context".to_owned(),
        );
    }
    if context
        .windows(execution.len())
        .any(|candidate| candidate == execution)
    {
        Ok(())
    } else {
        Err(
            "stored-data completeness execution slice is not a contiguous byte-for-byte minute-context subslice"
                .to_owned(),
        )
    }
}

fn daily_reference_counts(
    reference: DailyReferenceBinding<'_>,
) -> Result<(u64, u64), StoredDataCompletenessRefusal> {
    if reference.daily_bars.len() != reference.eligibility.len() {
        return Err(format!(
            "stored-data completeness daily/eligibility lengths differ: {} versus {}",
            reference.daily_bars.len(),
            reference.eligibility.len()
        ));
    }
    if let Some((index, byte)) = reference
        .eligibility
        .iter()
        .copied()
        .enumerate()
        .find(|(_, byte)| *byte > 1)
    {
        return Err(format!(
            "stored-data completeness eligibility byte {index} is {byte}, not canonical 0 or 1"
        ));
    }
    let eligibility = u64::try_from(reference.eligibility.len())
        .map_err(|_| "stored eligibility count does not fit u64".to_owned())?;
    let excluded_days = u64::try_from(reference.excluded_ist_days.len())
        .map_err(|_| "stored excluded-day count does not fit u64".to_owned())?;
    Ok((eligibility, excluded_days))
}

fn eligibility_digest(bytes: &[u8]) -> [u8; 32] {
    digest_bytes(ELIGIBILITY_DOMAIN_V1, bytes)
}

fn digest_bytes(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    hasher.update(&u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_le_bytes());
    hasher.update(bytes);
    hasher.finalize()
}

fn digest_days(days: &[i64]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(EXCLUDED_DAYS_DOMAIN_V1);
    hasher.update(&u64::try_from(days.len()).unwrap_or(u64::MAX).to_le_bytes());
    for day in days {
        hasher.update(&day.to_le_bytes());
    }
    hasher.finalize()
}

const fn family_byte(family: InstrumentFamilyV1) -> u8 {
    match family {
        InstrumentFamilyV1::Nifty => 1,
        InstrumentFamilyV1::BankNifty => 2,
    }
}

fn family_from_byte(byte: u8) -> Result<InstrumentFamilyV1, StoredDataCompletenessRefusal> {
    match byte {
        1 => Ok(InstrumentFamilyV1::Nifty),
        2 => Ok(InstrumentFamilyV1::BankNifty),
        _ => Err(format!(
            "stored-data completeness instrument-family byte {byte} is unknown"
        )),
    }
}

fn span_from_bytes(raw: &[u8; 20]) -> Result<RequestedSpanIdentityV1, String> {
    let mut fields = [0_u32; 5];
    for (slot, chunk) in fields.iter_mut().zip(raw.chunks_exact(4)) {
        *slot = u32::from_le_bytes(
            <[u8; 4]>::try_from(chunk)
                .map_err(|_| "stored-data requested-span field is not four bytes".to_owned())?,
        );
    }
    if fields[0] != 1 {
        return Err(format!(
            "stored-data requested-span version {} is unknown",
            fields[0]
        ));
    }
    RequestedSpanIdentityV1::new(
        u16::try_from(fields[1])
            .map_err(|_| "stored-data from-year does not fit u16".to_owned())?,
        u8::try_from(fields[2]).map_err(|_| "stored-data from-month does not fit u8".to_owned())?,
        u16::try_from(fields[3]).map_err(|_| "stored-data to-year does not fit u16".to_owned())?,
        u8::try_from(fields[4]).map_err(|_| "stored-data to-month does not fit u8".to_owned())?,
    )
}

fn validate_rung(rung: u32) -> Result<(), StoredDataCompletenessRefusal> {
    if [60, 120, 180, 300, 600, 900, 1_800, 3_600].contains(&rung) {
        Ok(())
    } else {
        Err(format!(
            "stored-data completeness signal rung {rung} is not canonical"
        ))
    }
}

fn require_digest(name: &str, digest: &[u8; 32]) -> Result<(), String> {
    if digest.iter().all(|byte| *byte == 0) {
        Err(format!(
            "stored-data completeness {name} is absent (all zero)"
        ))
    } else {
        Ok(())
    }
}

fn validate_max(max_receipts: usize) -> Result<(), StoredDataCompletenessRefusal> {
    if max_receipts == 0 {
        Err("stored-data completeness receipt bound must be nonzero".to_owned())
    } else {
        Ok(())
    }
}

fn write_header(file: &mut File) -> Result<(), StoredDataCompletenessRefusal> {
    let mut payload = [0_u8; HEADER_PAYLOAD_BYTES];
    payload
        .get_mut(..16)
        .ok_or_else(|| "stored-data completeness header magic field is absent".to_owned())?
        .copy_from_slice(&HEADER_MAGIC_V1);
    payload
        .get_mut(16..20)
        .ok_or_else(|| "stored-data completeness header version field is absent".to_owned())?
        .copy_from_slice(&HEADER_VERSION_V1.to_le_bytes());
    payload
        .get_mut(20..24)
        .ok_or_else(|| "stored-data completeness header stride field is absent".to_owned())?
        .copy_from_slice(
            &u32::try_from(RECEIPT_STRIDE_BYTES)
                .map_err(|_| "stored-data receipt stride does not fit u32".to_owned())?
                .to_le_bytes(),
        );
    let mut header = [0_u8; HEADER_BYTES];
    header[..HEADER_PAYLOAD_BYTES].copy_from_slice(&payload);
    header[HEADER_PAYLOAD_BYTES..].copy_from_slice(&hash(&payload));
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.write_all(&header))
        .map_err(|why| format!("stored-data completeness header write failed: {why}"))
}

fn validate_header(header: &[u8; HEADER_BYTES]) -> Result<(), String> {
    let payload = &header[..HEADER_PAYLOAD_BYTES];
    if header[HEADER_PAYLOAD_BYTES..] != hash(payload) {
        return Err("stored-data completeness header failed its BLAKE3 seal".to_owned());
    }
    if payload.get(..16) != Some(HEADER_MAGIC_V1.as_slice()) {
        return Err("stored-data completeness file has the wrong magic".to_owned());
    }
    let version = u32_at(payload, 16)?;
    if version != HEADER_VERSION_V1 {
        return Err(format!(
            "stored-data completeness file version {version} is unknown"
        ));
    }
    let stride = usize::try_from(u32_at(payload, 20)?)
        .map_err(|_| "stored-data completeness stride does not fit usize".to_owned())?;
    if stride != RECEIPT_STRIDE_BYTES {
        return Err(format!(
            "stored-data completeness file stride {stride} differs from {RECEIPT_STRIDE_BYTES}"
        ));
    }
    if payload
        .get(24..)
        .is_none_or(|reserve| reserve.iter().any(|byte| *byte != 0))
    {
        return Err("stored-data completeness header reserve is nonzero".to_owned());
    }
    Ok(())
}

fn scan_file(
    file: &mut File,
    path: &Path,
    max_receipts: usize,
) -> Result<HashMap<[u8; 32], StoredDataCompletenessReceiptV1>, String> {
    let len = file
        .metadata()
        .map_err(|why| format!("{} length could not be read: {why}", path.display()))?
        .len();
    let header_len = u64::try_from(HEADER_BYTES).unwrap_or(u64::MAX);
    let stride = u64::try_from(RECEIPT_STRIDE_BYTES).unwrap_or(u64::MAX);
    if len < header_len || !len.saturating_sub(header_len).is_multiple_of(stride) {
        return Err(format!(
            "{} is not a {HEADER_BYTES}-byte header plus whole {RECEIPT_STRIDE_BYTES}-byte receipts",
            path.display()
        ));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("{} header seek failed: {why}", path.display()))?;
    let mut header = [0_u8; HEADER_BYTES];
    file.read_exact(&mut header)
        .map_err(|why| format!("{} header read failed: {why}", path.display()))?;
    validate_header(&header)?;
    let count = usize::try_from(len.saturating_sub(header_len) / stride)
        .map_err(|_| "stored-data completeness receipt count does not fit usize".to_owned())?;
    if count > max_receipts {
        return Err(format!(
            "stored-data completeness ledger has {count} receipts above caller bound {max_receipts}"
        ));
    }
    let mut receipts = HashMap::new();
    receipts.try_reserve(count).map_err(|why| {
        format!("stored-data completeness index could not reserve {count} slots: {why}")
    })?;
    for index in 0..count {
        let mut raw = [0_u8; RECEIPT_STRIDE_BYTES];
        file.read_exact(&mut raw).map_err(|why| {
            format!(
                "{} receipt {index} could not be read: {why}",
                path.display()
            )
        })?;
        let receipt = StoredDataCompletenessReceiptV1::from_bytes(&raw)
            .map_err(|why| format!("{} receipt {index} refused: {why}", path.display()))?;
        if receipts.insert(receipt.population_id, receipt).is_some() {
            return Err(format!(
                "{} repeats stored-data completeness population identity at receipt {index}",
                path.display()
            ));
        }
    }
    Ok(receipts)
}

fn snapshot(file: &mut File, path: &Path) -> Result<FileSnapshotV1, String> {
    let len = file
        .metadata()
        .map_err(|why| format!("{} length could not be measured: {why}", path.display()))?
        .len();
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("{} generation seek failed: {why}", path.display()))?;
    let mut hasher = Hasher::new();
    hasher.update(FILE_DIGEST_DOMAIN_V1);
    hasher.update(&len.to_le_bytes());
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|why| format!("{} generation read failed: {why}", path.display()))?;
        if read == 0 {
            break;
        }
        let bytes = buffer
            .get(..read)
            .ok_or_else(|| format!("{} generation read exceeded buffer", path.display()))?;
        hasher.update(bytes);
    }
    Ok(FileSnapshotV1 {
        len,
        digest: hasher.finalize(),
    })
}

fn rollback_message(file: &File, start: u64, why: &std::io::Error) -> String {
    match file.set_len(start) {
        Ok(()) => format!(
            "stored-data completeness append failed: {why}; partial bytes were rolled back to {start}"
        ),
        Err(rollback) => format!(
            "stored-data completeness append failed: {why}; rollback to {start} also failed: {rollback}"
        ),
    }
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32, String> {
    let end = at
        .checked_add(4)
        .ok_or_else(|| "stored-data completeness u32 offset overflowed".to_owned())?;
    let raw = bytes
        .get(at..end)
        .ok_or_else(|| "stored-data completeness u32 field is truncated".to_owned())?;
    Ok(u32::from_le_bytes(<[u8; 4]>::try_from(raw).map_err(
        |_| "stored-data completeness u32 field has wrong width".to_owned(),
    )?))
}

struct Encoder<'a> {
    raw: &'a mut [u8],
    at: usize,
}

impl<'a> Encoder<'a> {
    const fn new(raw: &'a mut [u8]) -> Self {
        Self { raw, at: 0 }
    }

    fn bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        let end = self
            .at
            .checked_add(bytes.len())
            .ok_or_else(|| "stored-data completeness encode offset overflowed".to_owned())?;
        self.raw
            .get_mut(self.at..end)
            .ok_or_else(|| "stored-data completeness encode exceeded fixed stride".to_owned())?
            .copy_from_slice(bytes);
        self.at = end;
        Ok(())
    }

    fn zeros(&mut self, count: usize) -> Result<(), String> {
        let end = self
            .at
            .checked_add(count)
            .ok_or_else(|| "stored-data completeness zero offset overflowed".to_owned())?;
        self.raw
            .get_mut(self.at..end)
            .ok_or_else(|| "stored-data completeness zero fill exceeded fixed stride".to_owned())?
            .fill(0);
        self.at = end;
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), String> {
        self.bytes(&[value])
    }

    fn u32(&mut self, value: u32) -> Result<(), String> {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), String> {
        self.bytes(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), String> {
        self.bytes(&value.to_le_bytes())
    }

    fn finish(self) -> Result<(), String> {
        if self.at == self.raw.len() {
            Ok(())
        } else {
            Err(format!(
                "stored-data completeness encoded {} of {} fixed bytes",
                self.at,
                self.raw.len()
            ))
        }
    }
}

struct Decoder<'a> {
    raw: &'a [u8],
    at: usize,
}

impl<'a> Decoder<'a> {
    const fn new(raw: &'a [u8]) -> Self {
        Self { raw, at: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self
            .at
            .checked_add(count)
            .ok_or_else(|| "stored-data completeness decode offset overflowed".to_owned())?;
        let out = self
            .raw
            .get(self.at..end)
            .ok_or_else(|| "stored-data completeness record is truncated".to_owned())?;
        self.at = end;
        Ok(out)
    }

    fn bytes<const N: usize>(&mut self) -> Result<[u8; N], String> {
        <[u8; N]>::try_from(self.take(N)?)
            .map_err(|_| "stored-data completeness fixed field has wrong width".to_owned())
    }

    fn array_32(&mut self) -> Result<[u8; 32], String> {
        self.bytes::<32>()
    }

    fn zeros(&mut self, count: usize, name: &str) -> Result<(), String> {
        if self.take(count)?.iter().any(|byte| *byte != 0) {
            Err(format!("stored-data completeness {name} is nonzero"))
        } else {
            Ok(())
        }
    }

    fn u8(&mut self) -> Result<u8, String> {
        self.take(1)?
            .first()
            .copied()
            .ok_or_else(|| "stored-data completeness u8 is absent".to_owned())
    }

    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.bytes::<4>()?))
    }

    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.bytes::<8>()?))
    }

    fn i64(&mut self) -> Result<i64, String> {
        Ok(i64::from_le_bytes(self.bytes::<8>()?))
    }

    fn finish(self) -> Result<(), String> {
        if self.at == self.raw.len() {
            Ok(())
        } else {
            Err(format!(
                "stored-data completeness decoder left {} trailing bytes",
                self.raw.len().saturating_sub(self.at)
            ))
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "sealed-codec and authority fixtures must fail loudly at exact adversarial mutations"
)]
mod tests {
    use super::{
        HEADER_BYTES, RECEIPT_PAYLOAD_BYTES, StoredDataCompletenessCommitV1,
        StoredDataCompletenessLedgerV1, daily_reference_policy_digest_v1, eligibility_digest,
        prepare_stored_data_completeness_v1,
    };
    use brutex_core::instrument::{Exchange, InstrumentKey};
    use indicators::Candle;
    use indicators::column::Column;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use pull::calendar::{DayKind, kind_of};
    use pull::session::Day;
    use runner::admission::{AdmissionPolicyDraftV1, AdmissionPolicyV1, CompletenessV1};
    use runner::excursion::Side;
    use runner::exit_grid_policy::{
        ExecutionResolutionV1, ExecutionSeriesV1, ExitGridPolicyV1, ExitGridSelectorV1,
        ForcedStopV1, RangeResolutionV1, RatioLimitsV1, RationalPercentileV1, RungPlanV1,
        printed_ohlcv_cost_model_id_v1,
    };
    use runner::identity::{DailyReferenceBinding, ReferenceIntegrity};
    use runner::outcome::Horizon;
    use runner::topn::{RankingPolicyV1, Weights};
    use std::fs::OpenOptions;
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::institutional_evidence::{DataCompletenessSourceV1, test_data_completeness_value};
    use crate::population::{
        CompletionReceiptV4, CompletionReconciliationV2, ExitCellsPerMaskV2, InstrumentFamilyV1,
        LongShortExitGridIdentitiesV2, PopulationIdentitiesV2, RequestedSpanIdentityV1,
        SideExitGridIdentityV2,
    };
    use crate::population_admission_writer::{
        CompletePopulationAuthorityV1, derive_population_id_v1,
    };
    use crate::stored::calendar_receipt_v2;

    const FEED: &str = "fixture-stored-feed";
    const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
    const CALENDAR: [u8; 32] = [0x71; 32];
    const EXCLUDED: [i64; 2] = [20_010, 20_020];

    struct Temp {
        path: PathBuf,
    }

    impl Temp {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "brutex-stored-data-completeness-{label}-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("temporary directory");
            Self { path }
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    struct Fixture {
        signal: Vec<Candle>,
        minute_context: Vec<Candle>,
        daily: Vec<Candle>,
        eligibility: Vec<u8>,
        column: Column,
        instrument: InstrumentKey,
        long: runner::exit_grid_policy::ResolvedExitGridV1,
        short: runner::exit_grid_policy::ResolvedExitGridV1,
        policy: AdmissionPolicyV1,
        identities: PopulationIdentitiesV2,
        signal_calendar: crate::stored::CompleteCalendarReceiptV2,
        execution_calendar: crate::stored::CompleteCalendarReceiptV2,
        receipt: CompletionReceiptV4,
        population_id: [u8; 32],
    }

    impl Fixture {
        fn reference(&self) -> DailyReferenceBinding<'_> {
            DailyReferenceBinding {
                daily_bars: &self.daily,
                eligibility: &self.eligibility,
                schema: 1,
                eligibility_policy: 1,
                gap_overlay_policy: 1,
                excluded_ist_days: &EXCLUDED,
                daily_integrity: ReferenceIntegrity::UnverifiedNoReceipt,
                minute_integrity: ReferenceIntegrity::UnverifiedNoReceipt,
                swept_series_calendar_policy: crate::stored::SWEPT_SERIES_CALENDAR_POLICY,
            }
        }

        fn authority(&self) -> CompletePopulationAuthorityV1<'_> {
            self.authority_with_identities(&self.identities)
        }

        fn authority_with_identities(
            &self,
            identities: &PopulationIdentitiesV2,
        ) -> CompletePopulationAuthorityV1<'_> {
            let series =
                ExecutionSeriesV1::new(&self.instrument, FEED, COMMIT, CALENDAR, &self.signal)
                    .expect("fixture execution series");
            CompletePopulationAuthorityV1 {
                instrument_family: InstrumentFamilyV1::Nifty,
                rung_seconds: 60,
                horizon: Horizon::bars(2).expect("nonzero horizon"),
                requested_span: span(),
                identities: *identities,
                signal_calendar: self.signal_calendar,
                execution_calendar: self.execution_calendar,
                admission_policy: self.policy,
                long_exit_grid: &self.long,
                short_exit_grid: &self.short,
                execution_series: series,
                execution_column: &self.column,
            }
        }

        fn prepared(&self) -> super::PreparedStoredDataCompletenessV1 {
            prepare_stored_data_completeness_v1(
                self.receipt,
                &self.authority(),
                &self.signal,
                &self.minute_context,
                self.reference(),
            )
            .expect("canonical stored-data completeness preparation")
        }
    }

    fn span() -> RequestedSpanIdentityV1 {
        RequestedSpanIdentityV1::new(2026, 7, 2026, 7).expect("canonical test span")
    }

    fn full_month_timestamps() -> Vec<i64> {
        let span = span();
        let first = Day::new(span.from_year(), span.from_month(), 1).expect("first day");
        let last = Day::new(span.to_year(), span.to_month(), 1)
            .expect("last month")
            .end_of_month();
        let mut timestamps = Vec::new();
        for day in i64::from(first.days_from_epoch())..=i64::from(last.days_from_epoch()) {
            match kind_of(day) {
                DayKind::Open(session) => {
                    for window in session.windows.iter().take(usize::from(session.count)) {
                        for minute in window.from..=window.to {
                            timestamps.push(
                                day.saturating_mul(86_400_000_000)
                                    .saturating_add(i64::from(minute).saturating_mul(60_000_000))
                                    .saturating_sub(indicators::IST_OFFSET_MICROS),
                            );
                        }
                    }
                }
                DayKind::Closed => {}
                DayKind::OpenLengthUnmeasured | DayKind::Unmeasured => {
                    panic!("fixture day {day} is unmeasured")
                }
            }
        }
        timestamps
    }

    fn bars_of(timestamps: &[i64]) -> Vec<Candle> {
        timestamps
            .iter()
            .copied()
            .enumerate()
            .map(|(index, timestamp)| {
                let step = i64::try_from(index % 97).expect("bounded price step");
                let open = 2_000_000_i64.saturating_add(step.saturating_mul(5));
                Candle::new(
                    timestamp,
                    open,
                    open.saturating_add(100),
                    open.saturating_sub(100),
                    open.saturating_add(10),
                    10_000,
                    indicators::OI_NULL,
                )
            })
            .collect()
    }

    fn percentile() -> RationalPercentileV1 {
        RationalPercentileV1::new(1, 2).expect("canonical percentile")
    }

    fn exit_policy(side: Side) -> ExitGridPolicyV1 {
        ExitGridPolicyV1::new(
            ExecutionResolutionV1::OneMinuteOhlcv,
            RangeResolutionV1::PpmCeiling,
            side,
            RungPlanV1::new(
                vec![percentile()],
                vec![percentile()],
                vec![percentile()],
                1,
            )
            .expect("one rung per axis"),
            RatioLimitsV1::new(1, 10_000, 1).expect("wide ratio limits"),
            1_000,
            ExitGridSelectorV1::GuaranteedFloor,
            printed_ohlcv_cost_model_id_v1(),
            ForcedStopV1::Disabled,
            u64::MAX,
            u64::MAX,
        )
        .expect("canonical minimal grid policy")
    }

    fn admission_policy() -> AdmissionPolicyV1 {
        const PPM: u64 = 1_000_000;
        AdmissionPolicyV1::new(AdmissionPolicyDraftV1 {
            min_support_hits: Some(1),
            min_independent_sessions: Some(1),
            min_trades: Some(1),
            max_mae_paisa: Some(u64::MAX),
            min_worst_reward_risk_ppm: Some(0),
            min_win_rate_ppm: Some(0),
            min_wilson_win_rate_ppm: Some(0),
            min_return_drawdown_ppm: Some(0),
            min_weakest_period_return_paisa: Some(i64::MIN),
            max_pbo_ppm: Some(PPM),
            max_fwer_p_value_ppm: Some(PPM),
            max_spa_p_value_ppm: Some(PPM),
            min_decided_folds: Some(1),
            max_ambiguous_fill_rate_ppm: Some(PPM),
            max_gap_affected_rate_ppm: Some(PPM),
            max_session_concentration_ppm: Some(PPM),
            max_largest_trade_profit_share_ppm: Some(PPM),
            max_drawdown_paisa: Some(u64::MAX),
            max_worst_trade_loss_paisa: Some(u64::MAX),
            max_losing_trade_rate_ppm: Some(PPM),
            max_losing_trades: Some(u64::MAX),
            min_pessimistic_profit_paisa: Some(i64::MIN),
            min_winning_trades: Some(0),
            min_average_win_paisa: Some(0),
            max_average_loss_paisa: Some(u64::MAX),
            min_profit_factor_ppm: Some(0),
            max_consecutive_losing_streak: Some(u64::MAX),
            min_consecutive_winning_streak: Some(0),
            min_bootstrap_draws: Some(1),
            min_bootstrap_strategies: Some(1),
            min_bootstrap_periods: Some(1),
            min_pbo_contributing_folds: Some(1),
            max_pbo_unrankable_folds: Some(u64::MAX),
            min_profitable_oos_folds: Some(1),
            min_oos_pessimistic_return_paisa: Some(i64::MIN),
            max_white_reality_p_value_ppm: Some(PPM),
            max_romano_wolf_p_value_ppm: Some(PPM),
            require_white_reality_rejection: Some(false),
            require_romano_wolf_rejection: Some(false),
        })
        .expect("fully explicit admission policy")
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one canonical fixture builds the exact four streams, Population V4 receipt and matching dynamic authorities together"
    )]
    fn fixture() -> Fixture {
        let timestamps = full_month_timestamps();
        let signal = bars_of(&timestamps);
        let warmup_ts = signal
            .first()
            .expect("month has bars")
            .ts_micros
            .saturating_sub(60_000_000);
        let mut minute_context = bars_of(&[warmup_ts]);
        minute_context.extend_from_slice(&signal);
        let daily_ts = warmup_ts.saturating_sub(86_400_000_000);
        let daily = bars_of(&[daily_ts]);
        let eligibility = vec![1];
        let mut evaluator = Evaluator::new(
            Widths::pinned().expect("pinned widths"),
            Availability::Absent,
            Thresholds::CLASSICAL,
        );
        let column = Column::build(&signal, &mut evaluator);
        let instrument = InstrumentKey::index(Exchange::Nse, "NIFTY").expect("NSE NIFTY");
        let series = ExecutionSeriesV1::new(&instrument, FEED, COMMIT, CALENDAR, &signal)
            .expect("fixture series");
        let long = exit_policy(Side::Long)
            .resolve_attested(series)
            .expect("long resolution");
        let short = exit_policy(Side::Short)
            .resolve_attested(series)
            .expect("short resolution");
        let policy = admission_policy();
        let reference = DailyReferenceBinding {
            daily_bars: &daily,
            eligibility: &eligibility,
            schema: 1,
            eligibility_policy: 1,
            gap_overlay_policy: 1,
            excluded_ist_days: &EXCLUDED,
            daily_integrity: ReferenceIntegrity::UnverifiedNoReceipt,
            minute_integrity: ReferenceIntegrity::UnverifiedNoReceipt,
            swept_series_calendar_policy: crate::stored::SWEPT_SERIES_CALENDAR_POLICY,
        };
        let data_digest =
            runner::identity::data_digest_with_daily_reference(&signal, &minute_context, reference)
                .expect("three-stream identity");
        let evaluation_policy_digest = column
            .evaluation_spec_token()
            .map(|token| brutex_core::blake3::hash(token.fingerprint_v1().as_bytes()))
            .expect("column fingerprint");
        let ranking = RankingPolicyV1::new(Weights::equal()).expect("canonical ranking policy");
        let identities = PopulationIdentitiesV2 {
            run_identity: [0x11; 32],
            data_digest,
            feed_digest: brutex_core::blake3::hash(FEED.as_bytes()),
            source_commit_digest: brutex_core::blake3::hash(COMMIT.as_bytes()),
            vocabulary_digest: [0x22; 32],
            evaluation_policy_digest,
            exit_grids: LongShortExitGridIdentitiesV2 {
                long: SideExitGridIdentityV2 {
                    policy_digest: long.policy_digest(),
                    resolved_digest: long.digest(),
                },
                short: SideExitGridIdentityV2 {
                    policy_digest: short.policy_digest(),
                    resolved_digest: short.digest(),
                },
            },
            admission_policy_digest: policy.digest(),
            ranking_policy_digest: ranking.digest(),
            calendar_policy_digest: CALENDAR,
            daily_reference_policy_digest: daily_reference_policy_digest_v1(reference),
        };
        let first = Day::new(2026, 7, 1).expect("first day");
        let last = Day::new(2026, 7, 1).expect("last month").end_of_month();
        let signal_calendar = calendar_receipt_v2(
            &timestamps,
            60,
            i64::from(first.days_from_epoch()),
            i64::from(last.days_from_epoch()),
        )
        .expect("signal calendar")
        .require_complete()
        .expect("complete signal calendar");
        let execution_calendar = signal_calendar;
        let authority = CompletePopulationAuthorityV1 {
            instrument_family: InstrumentFamilyV1::Nifty,
            rung_seconds: 60,
            horizon: Horizon::bars(2).expect("nonzero horizon"),
            requested_span: span(),
            identities,
            signal_calendar,
            execution_calendar,
            admission_policy: policy,
            long_exit_grid: &long,
            short_exit_grid: &short,
            execution_series: series,
            execution_column: &column,
        };
        let population_id = derive_population_id_v1(&authority).expect("population identity");
        let receipt = CompletionReceiptV4::for_rows(
            population_id,
            InstrumentFamilyV1::Nifty,
            60,
            &[],
            span(),
            CompletionReconciliationV2 {
                sweep_trials: 0,
                frequent_itemsets: 0,
                infrequent_itemsets: 0,
                closed_itemsets: 0,
                redundant_itemsets: 0,
                unknown_closure_itemsets: 0,
                exit_cells_per_mask: ExitCellsPerMaskV2::new(long.cell_count(), short.cell_count())
                    .expect("nonempty side grids"),
                extinction_depth: 0,
                extinction_complete: true,
                closure_complete: true,
            },
            identities,
            signal_calendar,
            execution_calendar,
        )
        .expect("empty-but-complete Population V4 receipt");
        Fixture {
            signal,
            minute_context,
            daily,
            eligibility,
            column,
            instrument,
            long,
            short,
            policy,
            identities,
            signal_calendar,
            execution_calendar,
            receipt,
            population_id,
        }
    }

    #[test]
    fn exact_three_stream_bytes_and_population_v4_are_required() {
        let fixture = fixture();
        let prepared = fixture.prepared();
        assert_eq!(prepared.receipt().population_id(), fixture.population_id);
        assert_eq!(
            prepared.receipt().signal_records(),
            u64::try_from(fixture.signal.len()).expect("signal count")
        );
        assert_eq!(prepared.receipt().daily_records(), 1);

        let mut changed_signal = fixture.signal.clone();
        changed_signal[0].close = changed_signal[0].close.saturating_add(1);
        let why = prepare_stored_data_completeness_v1(
            fixture.receipt,
            &fixture.authority(),
            &changed_signal,
            &fixture.minute_context,
            fixture.reference(),
        )
        .expect_err("changed signal bytes must refuse");
        assert!(why.contains("data_digest") || why.contains("calendar"));

        let why = prepare_stored_data_completeness_v1(
            fixture.receipt,
            &fixture.authority(),
            &fixture.signal,
            &fixture.minute_context[..1],
            fixture.reference(),
        )
        .expect_err("detached execution bytes must refuse");
        assert!(why.contains("subslice") || why.contains("longer"));
    }

    #[test]
    fn sealed_append_reopen_reuse_corruption_and_stale_handles_fail_closed() {
        let fixture = fixture();
        let prepared = fixture.prepared();
        let temp = Temp::new("ledger");
        let mut ledger = StoredDataCompletenessLedgerV1::open(&temp.path, 2)
            .expect("open writable completeness ledger");
        let (outcome, authority) = ledger.commit(prepared).expect("commit receipt");
        assert_eq!(outcome, StoredDataCompletenessCommitV1::Written);
        authority
            .require_population(&fixture.authority())
            .expect("durable authority matches exact population");
        let (outcome, _) = ledger.commit(prepared).expect("exact retry");
        assert_eq!(outcome, StoredDataCompletenessCommitV1::Reused);
        drop(ledger);

        let mut reopened =
            StoredDataCompletenessLedgerV1::open_read(&temp.path, 2).expect("reopen sealed ledger");
        assert_eq!(reopened.receipts(), 1);
        reopened
            .authority(fixture.population_id)
            .expect("reopened authority");

        let path = StoredDataCompletenessLedgerV1::path(&temp.path);
        let mut external = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("open external mutator");
        let at = u64::try_from(HEADER_BYTES + 12).expect("mutation offset");
        external.seek(SeekFrom::Start(at)).expect("seek mutation");
        let mut byte = [0_u8; 1];
        external.read_exact(&mut byte).expect("read mutation byte");
        byte[0] ^= 1;
        external.seek(SeekFrom::Start(at)).expect("reseak mutation");
        external.write_all(&byte).expect("same-length mutation");
        external.sync_all().expect("sync mutation");
        assert!(
            reopened.authority(fixture.population_id).is_err(),
            "cached authority must reject same-length mutation"
        );
        drop(reopened);
        let why = StoredDataCompletenessLedgerV1::open_read(&temp.path, 2)
            .expect_err("sealed corruption must refuse reopen");
        assert!(why.contains("seal"));
    }

    #[test]
    fn institutional_complete_requires_reopened_matching_authority() {
        let fixture = fixture();
        let temp = Temp::new("institutional-binding");
        let mut ledger =
            StoredDataCompletenessLedgerV1::open(&temp.path, 1).expect("open completeness ledger");
        let (_, data_authority) = ledger
            .commit(fixture.prepared())
            .expect("durably commit completeness receipt");
        let population_authority = fixture.authority();
        assert_eq!(
            test_data_completeness_value(
                fixture.population_id,
                &population_authority,
                DataCompletenessSourceV1::Complete(&data_authority),
            )
            .expect("matching durable authority projects complete"),
            CompletenessV1::Complete
        );
        let why = test_data_completeness_value(
            [0x99; 32],
            &population_authority,
            DataCompletenessSourceV1::Complete(&data_authority),
        )
        .expect_err("a different evidence population cannot borrow complete data authority");
        assert!(why.contains("another evidence population"));
    }

    #[test]
    fn ragged_tail_and_foreign_population_are_never_hidden() {
        let fixture = fixture();
        let temp = Temp::new("ragged");
        let mut ledger =
            StoredDataCompletenessLedgerV1::open(&temp.path, 1).expect("open completeness ledger");
        let (_, authority) = ledger.commit(fixture.prepared()).expect("commit receipt");
        let mut foreign_identities = fixture.identities;
        foreign_identities.run_identity[0] ^= 1;
        let why = authority
            .require_population(&fixture.authority_with_identities(&foreign_identities))
            .expect_err("durable authority must refuse a foreign Population V4 identity");
        assert!(why.contains("identity"));
        drop(ledger);
        let path = StoredDataCompletenessLedgerV1::path(&temp.path);
        let mut file = OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open ragged appender");
        file.write_all(&[1]).expect("append ragged byte");
        file.sync_all().expect("sync ragged byte");
        let why = StoredDataCompletenessLedgerV1::open_read(&temp.path, 2)
            .expect_err("ragged tail must refuse");
        assert!(why.contains("whole"));
    }

    #[test]
    fn codec_reserves_and_seals_are_not_ignored() {
        let receipt = fixture().prepared().receipt();
        let raw = receipt.to_bytes().expect("canonical bytes");
        assert_eq!(
            super::StoredDataCompletenessReceiptV1::from_bytes(&raw).expect("round trip"),
            receipt
        );
        let mut reserve = raw;
        reserve[4] = 1;
        let reseal = brutex_core::blake3::hash(&reserve[..RECEIPT_PAYLOAD_BYTES]);
        reserve[RECEIPT_PAYLOAD_BYTES..].copy_from_slice(&reseal);
        let why = super::StoredDataCompletenessReceiptV1::from_bytes(&reserve)
            .expect_err("resealed reserve must refuse");
        assert!(why.contains("reserve"));

        let mut seal = raw;
        seal[RECEIPT_PAYLOAD_BYTES] ^= 1;
        assert!(super::StoredDataCompletenessReceiptV1::from_bytes(&seal).is_err());
        assert_ne!(eligibility_digest(&[1]), eligibility_digest(&[0]));
    }
}
