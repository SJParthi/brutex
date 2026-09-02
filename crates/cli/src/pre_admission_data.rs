//! Audit-only Pre-Admission Data V1 receipt ledger.
//!
//! [`CandidateUniverseLedgerV1`](crate::candidate_universe::CandidateUniverseLedgerV1)
//! records every direct candidate cell before admission.  The next authority
//! must bind those candidates to the exact stored bytes that can later produce
//! selection-independent statistics, without naming a final Population V4.
//! This module defines that append-only format.
//!
//! One logical entry is two adjacent, fixed-width records in one file.  The
//! `Data` record is appended and synced first; its byte-identical semantic
//! `Completion` record is appended and synced last.  A crash may therefore
//! leave one valid trailing `Data` record.  Only an exact retry may complete
//! it.  A foreign completion, middle orphan, duplicate identity, bad seal,
//! ragged tail, stale handle or replaced path refuses.
//!
//! V2 uses separate append-only data/lock files, schema numbers and hash
//! domains. It repeats every V1 source identity and adds the exact authenticated
//! Candidate V1 reconciliation. V1 still rejects zero Candidate rows; V2 may
//! carry zero only when the opaque Candidate production commit proves natural
//! extinction, complete closure and exactly zero reconciled rows.
//!
//! This version intentionally exposes no public authoring capability.  Its
//! crate-private producer accepts only the sealed Candidate Universe production
//! result plus the exact receipt-last commit returned for that same result.
//! Exact bars, calendars, daily references and stored-load ceilings cross the
//! boundary inside that opaque capability; the Pre-Admission edge accepts none
//! of them from its caller.  Public callers may only open, audit and page
//! already existing bytes.
//!
//! Opening and generation validation scan bounded file bytes.  A page is
//! proportional to the returned records after that scan.  Calculating one
//! validated fixed-record offset is O(1) in record count; no whole-ledger,
//! source measurement, allocation, hash, lock, sync or filesystem latency is
//! described as O(1).
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

use brutex_core::blake3::Hasher;
use indicators::{Candle, evaluator::CHARTER_NON_REGULAR_IST_DAYS};
use runner::identity::DailyReferenceBinding;

use crate::candidate_universe::{
    CANDIDATE_SIGNAL_RUNGS_SECONDS_V1, CandidateExecutionStreamV1, CandidatePreAdmissionInputsV1,
    CandidateSignalStreamV1, CandidateUniverseProductionCommitV1, CandidateUniverseReceiptV1,
    CandidateUniverseReopenAuditV1, ProducedCandidateUniverseV1,
};
use crate::population::{CompletionReconciliationV2, InstrumentFamilyV1, RequestedSpanIdentityV1};
use crate::stored::{CompleteCalendarReceiptV2, StoredSpanLoadBoundV1};

/// Operator-facing refusal from the Pre-Admission Data V1 boundary.
pub type PreAdmissionDataRefusal = String;

/// Fixed header bytes in the Pre-Admission Data V1 file.
pub const PRE_ADMISSION_HEADER_BYTES_V1: u64 = 64;
/// Bytes in either a data or receipt-last completion record.
pub const PRE_ADMISSION_RECORD_STRIDE_V1: u64 = 740;
/// Largest logical page one read may allocate.
pub const MAX_PRE_ADMISSION_PAGE_ROWS_V1: u64 = 256;

const HEADER_BYTES: usize = 64;
const PAYLOAD_BYTES: usize = 708;
const RECORD_BYTES: usize = 740;
const SEAL_BYTES: usize = 32;
const HEADER_VERSION: u32 = 1;
const RECORD_VERSION: u32 = 1;
const HEADER_KIND: u32 = 1;
const DATA_KIND: u32 = 1;
const COMPLETION_KIND: u32 = 2;
const HEADER_MAGIC: [u8; 16] = *b"BTX-PREADMIT-V1\0";
const DATA_FILE: &str = "pre-admission-data-v1.bin";
const LOCK_FILE: &str = "pre-admission-data-v1.lock";
const HEADER_DOMAIN: &[u8] = b"brutex-pre-admission-header-v1\0";
const RECORD_DOMAIN: &[u8] = b"brutex-pre-admission-record-v1\0";
const AUTHORITY_DOMAIN: &[u8] = b"brutex-pre-admission-authority-v1\0";
const GENERATION_DOMAIN: &[u8] = b"brutex-pre-admission-generation-v1\0";
const ELIGIBILITY_DOMAIN: &[u8] = b"brutex-pre-admission-eligibility-v1\0";
const EXCLUDED_DAYS_DOMAIN: &[u8] = b"brutex-pre-admission-excluded-days-v1\0";
const READ_CHUNK_BYTES: usize = 16 * 1024;

/// Fixed header bytes in the append-only Pre-Admission Data V2 file.
pub const PRE_ADMISSION_HEADER_BYTES_V2: u64 = 64;
/// Bytes in either a V2 Data or receipt-last Completion record.
pub const PRE_ADMISSION_RECORD_STRIDE_V2: u64 = 812;

const HEADER_BYTES_V2: usize = 64;
const CORE_PAYLOAD_BYTES_V2: usize = 708;
const RECONCILIATION_BYTES_V2: usize = 72;
const PAYLOAD_BYTES_V2: usize = CORE_PAYLOAD_BYTES_V2 + RECONCILIATION_BYTES_V2;
const RECORD_BYTES_V2: usize = PAYLOAD_BYTES_V2 + SEAL_BYTES;
/// Exact byte width embedded by the versioned Observation V2 successor.
///
/// This is crate-private because no caller may author Pre-Admission bytes.
pub(crate) const PRE_ADMISSION_OBSERVATION_SOURCE_BYTES_V2: usize = RECORD_BYTES_V2;
const HEADER_VERSION_V2: u32 = 2;
const RECORD_VERSION_V2: u32 = 2;
const HEADER_KIND_V2: u32 = 2;
const HEADER_MAGIC_V2: [u8; 16] = *b"BTX-PREADMIT-V2\0";
const DATA_FILE_V2: &str = "pre-admission-data-v2.bin";
const LOCK_FILE_V2: &str = "pre-admission-data-v2.lock";
const HEADER_DOMAIN_V2: &[u8] = b"brutex-pre-admission-header-v2\0";
const RECORD_DOMAIN_V2: &[u8] = b"brutex-pre-admission-record-v2\0";
const AUTHORITY_DOMAIN_V2: &[u8] = b"brutex-pre-admission-authority-v2\0";
const GENERATION_DOMAIN_V2: &[u8] = b"brutex-pre-admission-generation-v2\0";

const _: () = assert!(HEADER_BYTES_V2 as u64 == PRE_ADMISSION_HEADER_BYTES_V2);
const _: () = assert!(RECORD_BYTES_V2 as u64 == PRE_ADMISSION_RECORD_STRIDE_V2);
const _: () = assert!(CORE_PAYLOAD_BYTES_V2 == PAYLOAD_BYTES);
const _: () = assert!(PAYLOAD_BYTES_V2 + SEAL_BYTES == RECORD_BYTES_V2);

const _: () = assert!(HEADER_BYTES as u64 == PRE_ADMISSION_HEADER_BYTES_V1);
const _: () = assert!(RECORD_BYTES as u64 == PRE_ADMISSION_RECORD_STRIDE_V1);
const _: () = assert!(PAYLOAD_BYTES + SEAL_BYTES == RECORD_BYTES);

/// Digest and cardinality of one exact ordered candle stream.
///
/// The endpoints of the signal and execution streams remain available from
/// the bound Candidate Universe completion.  This compact copy is enough to
/// reject a different ordered candle stream without duplicating those endpoint
/// fields in the new format.  There is deliberately no public constructor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreAdmissionExactStreamV1 {
    count: u64,
    digest: [u8; 32],
}

impl PreAdmissionExactStreamV1 {
    /// Number of exact ordered candle records.
    #[must_use]
    pub const fn count(self) -> u64 {
        self.count
    }

    /// `runner::identity::data_digest` of every ordered candle field.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.digest
    }

    fn validate(self, name: &str) -> Result<(), PreAdmissionDataRefusal> {
        if self.count == 0 {
            return Err(format!("pre-admission {name} count is zero"));
        }
        require_nonzero(name, self.digest)
    }
}

/// Count, endpoints and digest of a complete ordered source stream.
///
/// Used for the complete one-minute context and the prior-day daily records.
/// No public constructor accepts arbitrary bars or digests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreAdmissionStreamFactsV1 {
    count: u64,
    first_ts_micros: i64,
    last_ts_micros: i64,
    digest: [u8; 32],
}

impl PreAdmissionStreamFactsV1 {
    /// Number of exact ordered records.
    #[must_use]
    pub const fn count(self) -> u64 {
        self.count
    }

    /// First candle-open timestamp, UTC microseconds.
    #[must_use]
    pub const fn first_ts_micros(self) -> i64 {
        self.first_ts_micros
    }

    /// Last candle-open timestamp, UTC microseconds.
    #[must_use]
    pub const fn last_ts_micros(self) -> i64 {
        self.last_ts_micros
    }

    /// Digest of every ordered candle field.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.digest
    }

    fn of(name: &str, bars: &[Candle]) -> Result<Self, PreAdmissionDataRefusal> {
        let first = bars
            .first()
            .ok_or_else(|| format!("pre-admission {name} stream is empty"))?;
        let last = bars
            .last()
            .ok_or_else(|| format!("pre-admission {name} stream lost its last record"))?;
        let count = u64::try_from(bars.len())
            .map_err(|_| format!("pre-admission {name} count does not fit u64"))?;
        validate_strict_timestamps(name, bars)?;
        Ok(Self {
            count,
            first_ts_micros: first.ts_micros,
            last_ts_micros: last.ts_micros,
            digest: runner::identity::data_digest(bars),
        })
    }

    fn validate(self, name: &str) -> Result<(), PreAdmissionDataRefusal> {
        if self.count == 0 {
            return Err(format!("pre-admission {name} count is zero"));
        }
        if self.first_ts_micros > self.last_ts_micros {
            return Err(format!(
                "pre-admission {name} stream runs backward: {} is after {}",
                self.first_ts_micros, self.last_ts_micros
            ));
        }
        require_nonzero(name, self.digest)
    }
}

/// Ordered eligibility and excluded-day evidence for prior-day daily bars.
///
/// `ineligible_count` is derived rather than stored independently.  The two
/// digests keep per-record decisions separate from the canonical ordered list
/// of excluded IST civil days.  That list is in charter policy order, which is
/// append-only policy identity rather than numeric date order; construction
/// therefore compares the exact list and never sorts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreAdmissionEligibilityFactsV1 {
    count: u64,
    eligible_count: u64,
    excluded_day_count: u64,
    eligibility_digest: [u8; 32],
    excluded_days_digest: [u8; 32],
}

impl PreAdmissionEligibilityFactsV1 {
    /// Number of eligibility bytes; it must equal the daily-record count.
    #[must_use]
    pub const fn count(self) -> u64 {
        self.count
    }

    /// Number of records marked eligible (`1`).
    #[must_use]
    pub const fn eligible_count(self) -> u64 {
        self.eligible_count
    }

    /// Number of records marked ineligible (`0`).
    #[must_use]
    pub const fn ineligible_count(self) -> u64 {
        self.count.saturating_sub(self.eligible_count)
    }

    /// Number of ordered policy-excluded IST days.
    #[must_use]
    pub const fn excluded_day_count(self) -> u64 {
        self.excluded_day_count
    }

    /// Digest of every ordered `0`/`1` eligibility byte.
    #[must_use]
    pub const fn eligibility_digest(self) -> [u8; 32] {
        self.eligibility_digest
    }

    /// Digest of every ordered excluded IST civil day.
    #[must_use]
    pub const fn excluded_days_digest(self) -> [u8; 32] {
        self.excluded_days_digest
    }

    fn from_reference(
        reference: DailyReferenceBinding<'_>,
    ) -> Result<Self, PreAdmissionDataRefusal> {
        if reference.daily_bars.len() != reference.eligibility.len() {
            return Err(format!(
                "pre-admission daily/eligibility lengths differ: {} versus {}",
                reference.daily_bars.len(),
                reference.eligibility.len()
            ));
        }
        let mut eligible_count = 0_u64;
        for (index, byte) in reference.eligibility.iter().copied().enumerate() {
            match byte {
                0 => {}
                1 => {
                    eligible_count = eligible_count
                        .checked_add(1)
                        .ok_or_else(|| "pre-admission eligible count overflowed u64".to_owned())?;
                }
                _ => {
                    return Err(format!(
                        "pre-admission eligibility byte {index} is {byte}, not canonical 0 or 1"
                    ));
                }
            }
        }
        if reference.excluded_ist_days != CHARTER_NON_REGULAR_IST_DAYS {
            return Err(
                "pre-admission excluded-day policy is not the exact ordered charter list"
                    .to_owned(),
            );
        }
        let count = u64::try_from(reference.eligibility.len())
            .map_err(|_| "pre-admission eligibility count does not fit u64".to_owned())?;
        let excluded_day_count = u64::try_from(reference.excluded_ist_days.len())
            .map_err(|_| "pre-admission excluded-day count does not fit u64".to_owned())?;
        Ok(Self {
            count,
            eligible_count,
            excluded_day_count,
            eligibility_digest: digest_bytes(ELIGIBILITY_DOMAIN, count, reference.eligibility),
            excluded_days_digest: digest_days(excluded_day_count, reference.excluded_ist_days),
        })
    }

    fn validate(self, daily_count: u64) -> Result<(), PreAdmissionDataRefusal> {
        if self.count != daily_count {
            return Err(format!(
                "pre-admission eligibility count {} differs from daily count {daily_count}",
                self.count
            ));
        }
        if self.eligible_count > self.count {
            return Err(format!(
                "pre-admission eligible count {} exceeds eligibility count {}",
                self.eligible_count, self.count
            ));
        }
        require_nonzero("eligibility digest", self.eligibility_digest)?;
        require_nonzero("excluded-days digest", self.excluded_days_digest)
    }
}

/// Decoded complete-calendar facts copied into the audit record.
///
/// Construction in the sealed source path accepts only
/// [`CompleteCalendarReceiptV2`], not a status boolean or arbitrary digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreAdmissionCalendarFactsV1 {
    rung_seconds: u32,
    first_day: i64,
    last_day: i64,
    digest: [u8; 32],
}

impl PreAdmissionCalendarFactsV1 {
    /// Proved-complete rung width.
    #[must_use]
    pub const fn rung_seconds(self) -> u32 {
        self.rung_seconds
    }

    /// First fully covered IST civil day, inclusive.
    #[must_use]
    pub const fn first_day(self) -> i64 {
        self.first_day
    }

    /// Last fully covered IST civil day, inclusive.
    #[must_use]
    pub const fn last_day(self) -> i64 {
        self.last_day
    }

    /// Unchanged complete Calendar Receipt V2 digest.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.digest
    }

    const fn from_complete(receipt: CompleteCalendarReceiptV2) -> Self {
        Self {
            rung_seconds: receipt.rung_seconds(),
            first_day: receipt.first_day(),
            last_day: receipt.last_day(),
            digest: receipt.digest(),
        }
    }

    fn validate(
        self,
        name: &str,
        require_signal_rung: fn(u32) -> Result<(), PreAdmissionDataRefusal>,
    ) -> Result<(), PreAdmissionDataRefusal> {
        require_signal_rung(self.rung_seconds)?;
        if self.first_day > self.last_day {
            return Err(format!(
                "pre-admission {name} calendar runs backward: {}..={} ",
                self.first_day, self.last_day
            ));
        }
        require_nonzero(&format!("{name} complete-calendar digest"), self.digest)
    }
}

/// One decoded, sealed Pre-Admission Data V1 audit row.
///
/// This is intentionally named an audit row rather than a production
/// capability.  Reopening it proves only that existing bytes are structurally
/// self-consistent and unchanged.  Crate-private production authoring requires
/// a Candidate production result and its exact durable commit; decoding this
/// public value never recreates that capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreAdmissionDataV1 {
    sequence: u64,
    authority_id: [u8; 32],
    candidate_universe_id: [u8; 32],
    candidate_completion_digest: [u8; 32],
    candidate_row_count: u64,
    family: InstrumentFamilyV1,
    rung_seconds: u32,
    horizon_bars: u32,
    requested_span: RequestedSpanIdentityV1,
    feed_digest: [u8; 32],
    source_commit_digest: [u8; 32],
    calendar_policy_digest: [u8; 32],
    daily_reference_policy_digest: [u8; 32],
    signal: PreAdmissionExactStreamV1,
    minute_context: PreAdmissionStreamFactsV1,
    execution: PreAdmissionExactStreamV1,
    daily: PreAdmissionStreamFactsV1,
    eligibility: PreAdmissionEligibilityFactsV1,
    signal_calendar: PreAdmissionCalendarFactsV1,
    execution_calendar: PreAdmissionCalendarFactsV1,
    signal_load_ceiling: u64,
    minute_load_ceiling: u64,
    daily_load_ceiling: u64,
    execution_start_index: u64,
}

impl PreAdmissionDataV1 {
    /// Canonical zero-based logical sequence in this ledger.
    #[must_use]
    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    /// Domain-separated identity of all semantic source fields.
    #[must_use]
    pub const fn authority_id(self) -> [u8; 32] {
        self.authority_id
    }

    /// Candidate Universe V1 identity this data precedes admission for.
    #[must_use]
    pub const fn candidate_universe_id(self) -> [u8; 32] {
        self.candidate_universe_id
    }

    /// Candidate receipt content digest, including ordered-row reconciliation.
    #[must_use]
    pub const fn candidate_completion_digest(self) -> [u8; 32] {
        self.candidate_completion_digest
    }

    /// Number of completed candidate rows.
    #[must_use]
    pub const fn candidate_row_count(self) -> u64 {
        self.candidate_row_count
    }

    /// NIFTY or BANKNIFTY spot family.
    #[must_use]
    pub const fn family(self) -> InstrumentFamilyV1 {
        self.family
    }

    /// Signal rung in seconds.
    #[must_use]
    pub const fn rung_seconds(self) -> u32 {
        self.rung_seconds
    }

    /// One-minute exit horizon in bars.
    #[must_use]
    pub const fn horizon_bars(self) -> u32 {
        self.horizon_bars
    }

    /// Inclusive requested month span.
    #[must_use]
    pub const fn requested_span(self) -> RequestedSpanIdentityV1 {
        self.requested_span
    }

    /// Exact vendor/feed identity copied from the candidate completion.
    #[must_use]
    pub const fn feed_digest(self) -> [u8; 32] {
        self.feed_digest
    }

    /// Exact source commit identity copied from the candidate completion.
    #[must_use]
    pub const fn source_commit_digest(self) -> [u8; 32] {
        self.source_commit_digest
    }

    /// Canonical measured-session policy identity.
    #[must_use]
    pub const fn calendar_policy_digest(self) -> [u8; 32] {
        self.calendar_policy_digest
    }

    /// Causal prior-day reference-policy identity.
    #[must_use]
    pub const fn daily_reference_policy_digest(self) -> [u8; 32] {
        self.daily_reference_policy_digest
    }

    /// Exact signal-stream identity.
    #[must_use]
    pub const fn signal(self) -> PreAdmissionExactStreamV1 {
        self.signal
    }

    /// Complete stored one-minute context, including warm-up where required.
    #[must_use]
    pub const fn minute_context(self) -> PreAdmissionStreamFactsV1 {
        self.minute_context
    }

    /// Exact evaluated one-minute requested-span subslice.
    #[must_use]
    pub const fn execution(self) -> PreAdmissionExactStreamV1 {
        self.execution
    }

    /// Exact prior-day daily records offered to the causal join.
    #[must_use]
    pub const fn daily(self) -> PreAdmissionStreamFactsV1 {
        self.daily
    }

    /// Ordered daily eligibility and excluded-day evidence.
    #[must_use]
    pub const fn eligibility(self) -> PreAdmissionEligibilityFactsV1 {
        self.eligibility
    }

    /// Complete signal-rung calendar evidence.
    #[must_use]
    pub const fn signal_calendar(self) -> PreAdmissionCalendarFactsV1 {
        self.signal_calendar
    }

    /// Complete exact-one-minute execution calendar evidence.
    #[must_use]
    pub const fn execution_calendar(self) -> PreAdmissionCalendarFactsV1 {
        self.execution_calendar
    }

    /// Explicit signal stored-load record ceiling.
    #[must_use]
    pub const fn signal_load_ceiling(self) -> u64 {
        self.signal_load_ceiling
    }

    /// Explicit complete-minute-context stored-load record ceiling.
    #[must_use]
    pub const fn minute_load_ceiling(self) -> u64 {
        self.minute_load_ceiling
    }

    /// Explicit daily-reference stored-load record ceiling.
    #[must_use]
    pub const fn daily_load_ceiling(self) -> u64 {
        self.daily_load_ceiling
    }

    /// Zero-based location of the exact execution subslice in minute context.
    #[must_use]
    pub const fn execution_start_index(self) -> u64 {
        self.execution_start_index
    }

    fn with_sequence(mut self, sequence: u64) -> Self {
        self.sequence = sequence;
        self
    }

    fn validate(self) -> Result<(), PreAdmissionDataRefusal> {
        self.validate_with_rung(require_new_production_rung)
    }

    fn validate_stored_compatible(self) -> Result<(), PreAdmissionDataRefusal> {
        self.validate_with_rung(require_stored_compatible_rung)
    }

    fn validate_with_rung(
        self,
        require_signal_rung: fn(u32) -> Result<(), PreAdmissionDataRefusal>,
    ) -> Result<(), PreAdmissionDataRefusal> {
        require_nonzero("pre-admission authority", self.authority_id)?;
        require_nonzero("candidate universe", self.candidate_universe_id)?;
        require_nonzero("candidate completion", self.candidate_completion_digest)?;
        if self.candidate_row_count == 0 {
            return Err("pre-admission candidate row count is zero".to_owned());
        }
        require_signal_rung(self.rung_seconds)?;
        if self.horizon_bars == 0 {
            return Err("pre-admission one-minute horizon is zero".to_owned());
        }
        // Reconstructing validates the canonical store month domain and order.
        RequestedSpanIdentityV1::new(
            self.requested_span.from_year(),
            self.requested_span.from_month(),
            self.requested_span.to_year(),
            self.requested_span.to_month(),
        )?;
        for (name, digest) in [
            ("feed", self.feed_digest),
            ("source commit", self.source_commit_digest),
            ("calendar policy", self.calendar_policy_digest),
            ("daily-reference policy", self.daily_reference_policy_digest),
        ] {
            require_nonzero(name, digest)?;
        }
        if self.calendar_policy_digest != crate::stored::calendar_policy_digest_v2() {
            return Err(
                "pre-admission calendar-policy identity is not the canonical V2 measured-session policy"
                    .to_owned(),
            );
        }
        self.signal.validate("signal stream")?;
        self.minute_context.validate("minute context")?;
        self.execution.validate("execution subspan")?;
        self.daily.validate("daily reference")?;
        self.eligibility.validate(self.daily.count)?;
        self.signal_calendar
            .validate("signal", require_signal_rung)?;
        self.execution_calendar
            .validate("execution", require_signal_rung)?;
        if self.signal_calendar.rung_seconds != self.rung_seconds {
            return Err(format!(
                "pre-admission signal calendar rung {} differs from signal rung {}",
                self.signal_calendar.rung_seconds, self.rung_seconds
            ));
        }
        if self.execution_calendar.rung_seconds != 60 {
            return Err(format!(
                "pre-admission execution calendar rung {} is not sixty seconds",
                self.execution_calendar.rung_seconds
            ));
        }
        let (first_day, last_day) = requested_span_days(self.requested_span)?;
        for (name, calendar) in [
            ("signal", self.signal_calendar),
            ("execution", self.execution_calendar),
        ] {
            if calendar.first_day != first_day || calendar.last_day != last_day {
                return Err(format!(
                    "pre-admission {name} calendar covers {}..={}, not full requested span {first_day}..={last_day}",
                    calendar.first_day, calendar.last_day
                ));
            }
        }
        for (name, count, ceiling) in [
            ("signal", self.signal.count, self.signal_load_ceiling),
            (
                "minute",
                self.minute_context.count,
                self.minute_load_ceiling,
            ),
            ("daily", self.daily.count, self.daily_load_ceiling),
        ] {
            if ceiling == 0 {
                return Err(format!("pre-admission {name} load ceiling is zero"));
            }
            if count > ceiling {
                return Err(format!(
                    "pre-admission {name} count {count} exceeds explicit stored-load ceiling {ceiling}"
                ));
            }
        }
        let execution_end = self
            .execution_start_index
            .checked_add(self.execution.count)
            .ok_or_else(|| "pre-admission execution subslice end overflowed u64".to_owned())?;
        if execution_end > self.minute_context.count {
            return Err(format!(
                "pre-admission execution subslice {}..{execution_end} escapes {}-record minute context",
                self.execution_start_index, self.minute_context.count
            ));
        }
        if derive_authority_id(&self) != self.authority_id {
            return Err("pre-admission authority identity does not match its fields".to_owned());
        }
        Ok(())
    }

    fn record(self, kind: RecordKindV1) -> Result<[u8; RECORD_BYTES], PreAdmissionDataRefusal> {
        self.record_with_validation(kind, Self::validate)
    }

    #[cfg(test)]
    fn stored_compatible_record(
        self,
        kind: RecordKindV1,
    ) -> Result<[u8; RECORD_BYTES], PreAdmissionDataRefusal> {
        self.record_with_validation(kind, Self::validate_stored_compatible)
    }

    fn record_with_validation(
        self,
        kind: RecordKindV1,
        validate: fn(Self) -> Result<(), PreAdmissionDataRefusal>,
    ) -> Result<[u8; RECORD_BYTES], PreAdmissionDataRefusal> {
        validate(self)?;
        let mut payload = [0_u8; PAYLOAD_BYTES];
        put_u32(&mut payload, 0, RECORD_VERSION)?;
        put_u32(&mut payload, 4, kind.byte())?;
        put_u64(&mut payload, 8, self.sequence)?;
        put_bytes(&mut payload, 16, &self.authority_id)?;
        put_bytes(&mut payload, 48, &self.candidate_universe_id)?;
        put_bytes(&mut payload, 80, &self.candidate_completion_digest)?;
        put_u64(&mut payload, 112, self.candidate_row_count)?;
        let family = payload
            .get_mut(120)
            .ok_or_else(|| "pre-admission family encode byte is absent".to_owned())?;
        *family = family_byte(self.family);
        put_u32(&mut payload, 124, self.rung_seconds)?;
        put_u32(&mut payload, 128, self.horizon_bars)?;
        put_bytes(&mut payload, 136, &self.requested_span.canonical_bytes())?;
        put_bytes(&mut payload, 156, &self.feed_digest)?;
        put_bytes(&mut payload, 188, &self.source_commit_digest)?;
        put_bytes(&mut payload, 220, &self.calendar_policy_digest)?;
        put_bytes(&mut payload, 252, &self.daily_reference_policy_digest)?;
        encode_exact_stream(&mut payload, 284, self.signal)?;
        encode_stream(&mut payload, 324, self.minute_context)?;
        encode_exact_stream(&mut payload, 380, self.execution)?;
        encode_stream(&mut payload, 420, self.daily)?;
        encode_eligibility(&mut payload, 476, self.eligibility)?;
        encode_calendar(&mut payload, 564, self.signal_calendar)?;
        encode_calendar(&mut payload, 620, self.execution_calendar)?;
        put_u64(&mut payload, 676, self.signal_load_ceiling)?;
        put_u64(&mut payload, 684, self.minute_load_ceiling)?;
        put_u64(&mut payload, 692, self.daily_load_ceiling)?;
        put_u64(&mut payload, 700, self.execution_start_index)?;
        let mut record = [0_u8; RECORD_BYTES];
        put_bytes(&mut record, 0, &payload)?;
        put_bytes(
            &mut record,
            PAYLOAD_BYTES,
            &digest_domain(RECORD_DOMAIN, &payload),
        )?;
        Ok(record)
    }

    fn decode(record: &[u8]) -> Result<(RecordKindV1, Self), PreAdmissionDataRefusal> {
        if record.len() != RECORD_BYTES {
            return Err(format!(
                "pre-admission record is {} bytes, not {RECORD_BYTES}",
                record.len()
            ));
        }
        let payload = record
            .get(..PAYLOAD_BYTES)
            .ok_or_else(|| "pre-admission payload is absent".to_owned())?;
        require_seal(
            "pre-admission record",
            record
                .get(PAYLOAD_BYTES..)
                .ok_or_else(|| "pre-admission record seal is absent".to_owned())?,
            digest_domain(RECORD_DOMAIN, payload),
        )?;
        if get_u32(payload, 0)? != RECORD_VERSION {
            return Err("pre-admission record version is unknown".to_owned());
        }
        let kind = RecordKindV1::from_byte(get_u32(payload, 4)?)?;
        require_zero(payload, 121, 3, "pre-admission family reserve")?;
        require_zero(payload, 132, 4, "pre-admission horizon reserve")?;
        let requested_span = decode_span(
            payload
                .get(136..156)
                .ok_or_else(|| "pre-admission requested-span bytes are absent".to_owned())?,
        )?;
        let value = Self {
            sequence: get_u64(payload, 8)?,
            authority_id: get_32(payload, 16)?,
            candidate_universe_id: get_32(payload, 48)?,
            candidate_completion_digest: get_32(payload, 80)?,
            candidate_row_count: get_u64(payload, 112)?,
            family: family_from_byte(
                payload
                    .get(120)
                    .copied()
                    .ok_or_else(|| "pre-admission family byte is absent".to_owned())?,
            )?,
            rung_seconds: get_u32(payload, 124)?,
            horizon_bars: get_u32(payload, 128)?,
            requested_span,
            feed_digest: get_32(payload, 156)?,
            source_commit_digest: get_32(payload, 188)?,
            calendar_policy_digest: get_32(payload, 220)?,
            daily_reference_policy_digest: get_32(payload, 252)?,
            signal: decode_exact_stream(payload, 284)?,
            minute_context: decode_stream(payload, 324)?,
            execution: decode_exact_stream(payload, 380)?,
            daily: decode_stream(payload, 420)?,
            eligibility: decode_eligibility(payload, 476)?,
            signal_calendar: decode_calendar(payload, 564)?,
            execution_calendar: decode_calendar(payload, 620)?,
            signal_load_ceiling: get_u64(payload, 676)?,
            minute_load_ceiling: get_u64(payload, 684)?,
            daily_load_ceiling: get_u64(payload, 692)?,
            execution_start_index: get_u64(payload, 700)?,
        };
        value.validate_stored_compatible()?;
        Ok((kind, value))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RecordKindV1 {
    Data,
    Completion,
}

impl RecordKindV1 {
    const fn byte(self) -> u32 {
        match self {
            Self::Data => DATA_KIND,
            Self::Completion => COMPLETION_KIND,
        }
    }

    fn from_byte(value: u32) -> Result<Self, PreAdmissionDataRefusal> {
        match value {
            DATA_KIND => Ok(Self::Data),
            COMPLETION_KIND => Ok(Self::Completion),
            _ => Err(format!(
                "pre-admission record kind {value} is unknown; expected 1=data or 2=completion"
            )),
        }
    }
}

/// Explicit logical-row and file-byte ceilings.
///
/// There is no `Default`.  Both values are checked before ledger-sized
/// allocation and before append.  They are refusal limits, not sampling,
/// truncation or a sweep-depth parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreAdmissionDataBoundsV1 {
    max_rows: u64,
    max_file_bytes: u64,
}

impl PreAdmissionDataBoundsV1 {
    /// Construct nonzero row and byte ceilings.
    ///
    /// # Errors
    ///
    /// Refuses zero or a byte ceiling too small to hold the header plus one
    /// complete Data/Completion pair.
    pub fn new(max_rows: u64, max_file_bytes: u64) -> Result<Self, PreAdmissionDataRefusal> {
        if max_rows == 0 || max_file_bytes == 0 {
            return Err(format!(
                "pre-admission bounds must be nonzero; rows={max_rows}, bytes={max_file_bytes}"
            ));
        }
        let minimum = PRE_ADMISSION_HEADER_BYTES_V1
            .checked_add(
                PRE_ADMISSION_RECORD_STRIDE_V1
                    .checked_mul(2)
                    .ok_or_else(|| "pre-admission minimum byte bound overflowed u64".to_owned())?,
            )
            .ok_or_else(|| "pre-admission minimum file size overflowed u64".to_owned())?;
        if max_file_bytes < minimum {
            return Err(format!(
                "pre-admission byte ceiling {max_file_bytes} cannot hold one complete pair; minimum is {minimum}"
            ));
        }
        Ok(Self {
            max_rows,
            max_file_bytes,
        })
    }

    /// Maximum logical Data/Completion pairs, including a trailing orphan row.
    #[must_use]
    pub const fn max_rows(self) -> u64 {
        self.max_rows
    }

    /// Maximum complete file bytes, header included.
    #[must_use]
    pub const fn max_file_bytes(self) -> u64 {
        self.max_file_bytes
    }
}

/// Audit-only reopened proof of one exact Data/Completion pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreAdmissionDataReopenAuditV1 {
    data_record_index: u64,
    value: PreAdmissionDataV1,
}

impl PreAdmissionDataReopenAuditV1 {
    /// Physical fixed-record index of the Data member of this pair.
    #[must_use]
    pub const fn data_record_index(self) -> u64 {
        self.data_record_index
    }

    /// Logical sequence.
    #[must_use]
    pub const fn sequence(self) -> u64 {
        self.value.sequence
    }

    /// Audit-only decoded value.
    #[must_use]
    pub const fn value(self) -> PreAdmissionDataV1 {
        self.value
    }

    /// Semantic Pre-Admission identity.
    #[must_use]
    pub const fn authority_id(self) -> [u8; 32] {
        self.value.authority_id
    }
}

/// One bounded page of completed audit rows in canonical sequence order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreAdmissionDataPageV1 {
    rows: Vec<PreAdmissionDataV1>,
}

impl PreAdmissionDataPageV1 {
    /// Borrow decoded completed rows in canonical sequence order.
    #[must_use]
    pub fn rows(&self) -> &[PreAdmissionDataV1] {
        &self.rows
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OrphanDataV1 {
    record_index: u64,
    value: PreAdmissionDataV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGenerationV1 {
    len: u64,
    content_digest: [u8; 32],
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(unix)]
    modified_seconds: i64,
    #[cfg(unix)]
    modified_nanoseconds: i64,
    #[cfg(unix)]
    changed_seconds: i64,
    #[cfg(unix)]
    changed_nanoseconds: i64,
    #[cfg(not(unix))]
    modified: Option<std::time::SystemTime>,
}

/// Open, bounded Pre-Admission Data V1 audit ledger.
#[derive(Debug)]
pub struct PreAdmissionDataLedgerV1 {
    lock_path: PathBuf,
    data_path: PathBuf,
    lock_file: File,
    data_file: File,
    bounds: PreAdmissionDataBoundsV1,
    audits: HashMap<[u8; 32], PreAdmissionDataReopenAuditV1>,
    completed_rows: u64,
    orphan: Option<OrphanDataV1>,
    lock_generation: FileGenerationV1,
    data_generation: FileGenerationV1,
    writable: bool,
}

impl PreAdmissionDataLedgerV1 {
    /// Opens an existing ledger without creating or modifying any path.
    ///
    /// # Errors
    ///
    /// Refuses an absent or non-directory admitted root, absent read-only
    /// files, lock failure, invalid header, bound breach,
    /// ragged/corrupt record, nonadjacent or foreign completion, duplicate
    /// identity/sequence, more than one orphan, I/O error or path replacement.
    pub fn open_read(
        root: impl AsRef<Path>,
        bounds: PreAdmissionDataBoundsV1,
    ) -> Result<Self, PreAdmissionDataRefusal> {
        Self::open_inner(root.as_ref(), bounds, false)
    }

    fn open(
        root: impl AsRef<Path>,
        bounds: PreAdmissionDataBoundsV1,
    ) -> Result<Self, PreAdmissionDataRefusal> {
        Self::open_inner(root.as_ref(), bounds, true)
    }

    fn open_inner(
        root: &Path,
        bounds: PreAdmissionDataBoundsV1,
        writable: bool,
    ) -> Result<Self, PreAdmissionDataRefusal> {
        let root_metadata = std::fs::metadata(root).map_err(|why| {
            format!(
                "pre-admission ledger root {} must already exist and be admitted: {why}",
                root.display()
            )
        })?;
        if !root_metadata.is_dir() {
            return Err(format!(
                "pre-admission ledger root {} is not a directory",
                root.display()
            ));
        }
        let lock_path = root.join(LOCK_FILE);
        let data_path = root.join(DATA_FILE);
        let lock_file = open_file(&lock_path, writable, writable)?;
        if writable {
            lock_file.lock().map_err(|why| {
                format!(
                    "cannot lock pre-admission writer {}: {why}",
                    lock_path.display()
                )
            })?;
        } else {
            lock_file.lock_shared().map_err(|why| {
                format!(
                    "cannot take shared pre-admission lock {}: {why}",
                    lock_path.display()
                )
            })?;
        }
        let held_lock = lock_file.try_clone().map_err(|why| {
            format!(
                "cannot clone pre-admission lock {}: {why}",
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
            let lock_generation = file_generation(&held_lock, &lock_path)?;
            let data_generation = file_generation(&data_file, &data_path)?;
            let mut ledger = Self {
                lock_path: lock_path.clone(),
                data_path,
                lock_file: held_lock,
                data_file,
                bounds,
                audits: HashMap::new(),
                completed_rows: 0,
                orphan: None,
                lock_generation,
                data_generation,
                writable,
            };
            ledger.scan()?;
            Ok(ledger)
        })();
        let released = lock_file.unlock().map_err(|why| {
            format!(
                "cannot release pre-admission open lock {}: {why}",
                lock_path.display()
            )
        });
        match (opened, released) {
            (Ok(ledger), Ok(())) => Ok(ledger),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn scan(&mut self) -> Result<(), PreAdmissionDataRefusal> {
        verify_header(&mut self.data_file, &self.data_path)?;
        let file_len = self
            .data_file
            .metadata()
            .map_err(|why| {
                format!(
                    "cannot stat pre-admission file {}: {why}",
                    self.data_path.display()
                )
            })?
            .len();
        if file_len > self.bounds.max_file_bytes {
            return Err(format!(
                "pre-admission file has {file_len} bytes, above explicit maximum {}",
                self.bounds.max_file_bytes
            ));
        }
        let physical_records = record_count(file_len)?;
        let logical_rows = physical_records
            .checked_add(1)
            .ok_or_else(|| "pre-admission logical-row count overflowed u64".to_owned())?
            / 2;
        if logical_rows > self.bounds.max_rows {
            return Err(format!(
                "pre-admission file has {logical_rows} logical rows, above explicit maximum {}",
                self.bounds.max_rows
            ));
        }
        let completed_capacity = usize::try_from(physical_records / 2)
            .map_err(|_| "pre-admission completion count does not fit usize".to_owned())?;
        self.audits = HashMap::new();
        self.audits
            .try_reserve(completed_capacity)
            .map_err(|why| format!("cannot reserve pre-admission audit index: {why}"))?;
        self.completed_rows = 0;
        self.orphan = None;
        let mut physical = 0_u64;
        while physical < physical_records {
            let (kind, data) = read_record(&mut self.data_file, physical)?;
            if kind != RecordKindV1::Data {
                return Err(format!(
                    "pre-admission physical record {physical} is a completion without its adjacent Data record"
                ));
            }
            if data.sequence != self.completed_rows {
                return Err(format!(
                    "pre-admission Data record sequence {} is not canonical {}",
                    data.sequence, self.completed_rows
                ));
            }
            let completion_index = physical
                .checked_add(1)
                .ok_or_else(|| "pre-admission completion index overflowed u64".to_owned())?;
            if completion_index == physical_records {
                self.orphan = Some(OrphanDataV1 {
                    record_index: physical,
                    value: data,
                });
                break;
            }
            let (completion_kind, completion) = read_record(&mut self.data_file, completion_index)?;
            if completion_kind != RecordKindV1::Completion {
                return Err(format!(
                    "pre-admission Data record {physical} is followed by another Data record, not its receipt-last completion"
                ));
            }
            if completion != data {
                return Err(format!(
                    "pre-admission completion at physical record {completion_index} does not exactly repeat Data semantics at {physical}"
                ));
            }
            let audit = PreAdmissionDataReopenAuditV1 {
                data_record_index: physical,
                value: data,
            };
            if self.audits.insert(data.authority_id, audit).is_some() {
                return Err(format!(
                    "pre-admission authority {} appears more than once",
                    hex32(data.authority_id)
                ));
            }
            self.completed_rows = self
                .completed_rows
                .checked_add(1)
                .ok_or_else(|| "pre-admission completed-row cursor overflowed u64".to_owned())?;
            physical = completion_index
                .checked_add(1)
                .ok_or_else(|| "pre-admission physical cursor overflowed u64".to_owned())?;
        }
        require_generation(self.lock_generation, &self.lock_file, &self.lock_path)?;
        require_generation(self.data_generation, &self.data_file, &self.data_path)?;
        Ok(())
    }

    /// Reopens one completed audit row after a shared lock and full generation
    /// revalidation.  Absence is `Ok(None)` and cannot hide I/O refusal.
    ///
    /// # Errors
    ///
    /// Refuses a stale/replaced path, changed bytes, lock or unlock failure.
    pub fn reopen_audit(
        &self,
        authority_id: &[u8; 32],
    ) -> Result<Option<PreAdmissionDataReopenAuditV1>, PreAdmissionDataRefusal> {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take shared pre-admission audit lock: {why}"))?;
        let result = self
            .require_unchanged()
            .map(|()| self.audits.get(authority_id).copied());
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release pre-admission audit lock: {why}"));
        match (result, released) {
            (Ok(audit), Ok(())) => Ok(audit),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// Reads at most 256 completed rows by canonical sequence.
    ///
    /// # Errors
    ///
    /// Refuses an over-limit page, offset overflow, stale/replaced file,
    /// allocation failure or any row/completion mismatch.
    pub fn page(
        &self,
        offset: u64,
        limit: u64,
    ) -> Result<PreAdmissionDataPageV1, PreAdmissionDataRefusal> {
        if limit > MAX_PRE_ADMISSION_PAGE_ROWS_V1 {
            return Err(format!(
                "pre-admission page limit {limit} exceeds {MAX_PRE_ADMISSION_PAGE_ROWS_V1}"
            ));
        }
        if offset > self.completed_rows {
            return Err(format!(
                "pre-admission page offset {offset} exceeds completed row count {}",
                self.completed_rows
            ));
        }
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take shared pre-admission page lock: {why}"))?;
        let result = (|| {
            self.require_unchanged()?;
            let count = self.completed_rows.saturating_sub(offset).min(limit);
            let capacity = usize::try_from(count)
                .map_err(|_| "pre-admission page count does not fit usize".to_owned())?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(capacity)
                .map_err(|why| format!("cannot reserve pre-admission page: {why}"))?;
            let mut file = open_file(&self.data_path, false, false)?;
            require_generation(self.data_generation, &file, &self.data_path)?;
            for step in 0..count {
                let sequence = offset
                    .checked_add(step)
                    .ok_or_else(|| "pre-admission page sequence overflowed u64".to_owned())?;
                let physical = sequence
                    .checked_mul(2)
                    .ok_or_else(|| "pre-admission page record index overflowed u64".to_owned())?;
                let (data_kind, data) = read_record(&mut file, physical)?;
                let completion_index = physical.checked_add(1).ok_or_else(|| {
                    "pre-admission page completion index overflowed u64".to_owned()
                })?;
                let (completion_kind, completion) = read_record(&mut file, completion_index)?;
                if data_kind != RecordKindV1::Data
                    || completion_kind != RecordKindV1::Completion
                    || data != completion
                    || data.sequence != sequence
                {
                    return Err(format!(
                        "pre-admission page pair for sequence {sequence} is no longer canonical"
                    ));
                }
                rows.push(data);
            }
            Ok(PreAdmissionDataPageV1 { rows })
        })();
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release pre-admission page lock: {why}"));
        match (result, released) {
            (Ok(page), Ok(())) => Ok(page),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn append_complete(
        &mut self,
        prepared: &PreAdmissionDataV1,
    ) -> Result<PreAdmissionProductionCommitV1, PreAdmissionDataRefusal> {
        if !self.writable {
            return Err("pre-admission ledger was opened read-only".to_owned());
        }
        self.lock_file
            .lock()
            .map_err(|why| format!("cannot take pre-admission append lock: {why}"))?;
        let result = self.append_complete_locked(prepared);
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release pre-admission append lock: {why}"));
        match (result, released) {
            (Ok(outcome), Ok(())) => Ok(outcome),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn append_complete_locked(
        &mut self,
        prepared: &PreAdmissionDataV1,
    ) -> Result<PreAdmissionProductionCommitV1, PreAdmissionDataRefusal> {
        self.require_unchanged()?;
        prepared.validate()?;
        if let Some(existing) = self.audits.get(&prepared.authority_id).copied() {
            if !same_semantics(&existing.value, prepared) {
                return Err(format!(
                    "pre-admission authority {} is already complete with different semantics",
                    hex32(prepared.authority_id)
                ));
            }
            return Ok(PreAdmissionProductionCommitV1::Reused(existing));
        }
        self.audits
            .try_reserve(1)
            .map_err(|why| format!("cannot reserve pre-admission append index: {why}"))?;
        if let Some(orphan) = self.orphan {
            if !same_semantics(&orphan.value, prepared) {
                return Err(format!(
                    "pre-admission trailing orphan belongs to {}, not exact retry {}; no fallback may hide it",
                    hex32(orphan.value.authority_id),
                    hex32(prepared.authority_id)
                ));
            }
            let completion = orphan.value.record(RecordKindV1::Completion)?;
            self.require_append_bytes(1)?;
            append_record(&mut self.data_file, &completion)?;
            self.data_file
                .sync_data()
                .map_err(|why| format!("cannot sync pre-admission completion: {why}"))?;
            let audit = PreAdmissionDataReopenAuditV1 {
                data_record_index: orphan.record_index,
                value: orphan.value,
            };
            if self
                .audits
                .insert(orphan.value.authority_id, audit)
                .is_some()
            {
                return Err(
                    "pre-admission exact-orphan completion collided with an indexed authority"
                        .to_owned(),
                );
            }
            self.completed_rows = self
                .completed_rows
                .checked_add(1)
                .ok_or_else(|| "pre-admission completed-row count overflowed u64".to_owned())?;
            self.orphan = None;
            self.data_generation = file_generation(&self.data_file, &self.data_path)?;
            return Ok(PreAdmissionProductionCommitV1::Written(audit));
        }
        if self.completed_rows >= self.bounds.max_rows {
            return Err(format!(
                "pre-admission completed rows reached explicit maximum {}",
                self.bounds.max_rows
            ));
        }
        self.require_append_bytes(2)?;
        let value = prepared.with_sequence(self.completed_rows);
        value.validate()?;
        let data_record = value.record(RecordKindV1::Data)?;
        let completion_record = value.record(RecordKindV1::Completion)?;
        let data_record_index = self
            .completed_rows
            .checked_mul(2)
            .ok_or_else(|| "pre-admission data-record index overflowed u64".to_owned())?;
        append_record(&mut self.data_file, &data_record)?;
        self.data_file
            .sync_data()
            .map_err(|why| format!("cannot sync pre-admission Data record: {why}"))?;
        self.orphan = Some(OrphanDataV1 {
            record_index: data_record_index,
            value,
        });
        self.data_generation = file_generation(&self.data_file, &self.data_path)?;
        append_record(&mut self.data_file, &completion_record)?;
        self.data_file
            .sync_data()
            .map_err(|why| format!("cannot sync pre-admission completion: {why}"))?;
        let audit = PreAdmissionDataReopenAuditV1 {
            data_record_index,
            value,
        };
        if self.audits.insert(value.authority_id, audit).is_some() {
            return Err(
                "pre-admission new completion collided with an indexed authority".to_owned(),
            );
        }
        self.completed_rows = self
            .completed_rows
            .checked_add(1)
            .ok_or_else(|| "pre-admission completed-row count overflowed u64".to_owned())?;
        self.orphan = None;
        self.data_generation = file_generation(&self.data_file, &self.data_path)?;
        Ok(PreAdmissionProductionCommitV1::Written(audit))
    }

    fn require_append_bytes(&self, records: u64) -> Result<(), PreAdmissionDataRefusal> {
        let current = self
            .data_file
            .metadata()
            .map_err(|why| format!("cannot stat pre-admission append file: {why}"))?
            .len();
        let added = records
            .checked_mul(PRE_ADMISSION_RECORD_STRIDE_V1)
            .ok_or_else(|| "pre-admission append byte count overflowed u64".to_owned())?;
        let desired = current
            .checked_add(added)
            .ok_or_else(|| "pre-admission desired file size overflowed u64".to_owned())?;
        if desired > self.bounds.max_file_bytes {
            return Err(format!(
                "pre-admission append would produce {desired} bytes, above explicit maximum {}",
                self.bounds.max_file_bytes
            ));
        }
        Ok(())
    }

    fn require_unchanged(&self) -> Result<(), PreAdmissionDataRefusal> {
        require_generation(self.lock_generation, &self.lock_file, &self.lock_path)?;
        require_generation(self.data_generation, &self.data_file, &self.data_path)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreAdmissionProductionCommitV1 {
    Written(PreAdmissionDataReopenAuditV1),
    Reused(PreAdmissionDataReopenAuditV1),
}

impl PreAdmissionProductionCommitV1 {
    pub(crate) const fn audit(self) -> PreAdmissionDataReopenAuditV1 {
        match self {
            Self::Written(audit) | Self::Reused(audit) => audit,
        }
    }
}

/// Private exact minute context with a checked contiguous execution subslice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MeasuredExecutionContextV1 {
    minute_context: PreAdmissionStreamFactsV1,
    execution: PreAdmissionStreamFactsV1,
    execution_start_index: u64,
}

impl MeasuredExecutionContextV1 {
    fn measure(
        minute_context: &[Candle],
        execution_start_index: usize,
        execution: &[Candle],
    ) -> Result<Self, PreAdmissionDataRefusal> {
        let end = execution_start_index
            .checked_add(execution.len())
            .ok_or_else(|| "pre-admission execution subslice end overflowed usize".to_owned())?;
        let actual = minute_context.get(execution_start_index..end).ok_or_else(|| {
            format!(
                "pre-admission execution subslice {execution_start_index}..{end} escapes {}-record minute context",
                minute_context.len()
            )
        })?;
        if actual != execution {
            return Err(
                "pre-admission execution bytes are not the claimed contiguous minute-context subslice"
                    .to_owned(),
            );
        }
        Ok(Self {
            minute_context: PreAdmissionStreamFactsV1::of("minute context", minute_context)?,
            execution: PreAdmissionStreamFactsV1::of("execution subspan", execution)?,
            execution_start_index: u64::try_from(execution_start_index)
                .map_err(|_| "pre-admission execution start does not fit u64".to_owned())?,
        })
    }
}

/// Private measured daily source; no digest-only constructor exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MeasuredDailyReferenceV1 {
    daily: PreAdmissionStreamFactsV1,
    eligibility: PreAdmissionEligibilityFactsV1,
    policy_digest: [u8; 32],
}

impl MeasuredDailyReferenceV1 {
    fn measure(reference: DailyReferenceBinding<'_>) -> Result<Self, PreAdmissionDataRefusal> {
        let daily = PreAdmissionStreamFactsV1::of("daily reference", reference.daily_bars)?;
        let eligibility = PreAdmissionEligibilityFactsV1::from_reference(reference)?;
        eligibility.validate(daily.count)?;
        Ok(Self {
            daily,
            eligibility,
            policy_digest: crate::stored_data_completeness::daily_reference_policy_digest_v1(
                reference,
            ),
        })
    }
}

/// Private owning measurement bundle between Candidate Universe and statistics.
///
/// There is deliberately no public constructor and no public field.  The only
/// measurement path checks actual contiguous bars and opaque completion types;
/// no caller can claim a digest or subspan relation.
#[derive(Clone, Copy, Debug)]
struct PreAdmissionSourceBundleV1 {
    candidate: CandidateUniverseReopenAuditV1,
    composite_data_digest: [u8; 32],
    signal: PreAdmissionStreamFactsV1,
    execution: MeasuredExecutionContextV1,
    daily: MeasuredDailyReferenceV1,
    signal_calendar: CompleteCalendarReceiptV2,
    execution_calendar: CompleteCalendarReceiptV2,
    signal_bound: StoredSpanLoadBoundV1,
    minute_bound: StoredSpanLoadBoundV1,
    daily_bound: StoredSpanLoadBoundV1,
}

impl PreAdmissionSourceBundleV1 {
    fn measure(
        candidate: &CandidateUniverseReopenAuditV1,
        inputs: &CandidatePreAdmissionInputsV1<'_>,
    ) -> Result<Self, PreAdmissionDataRefusal> {
        let signal_bars = inputs.signal_bars();
        let minute_context = inputs.minute_context();
        let execution_series = inputs.execution_series();
        let execution_bars = execution_series.bars();
        let execution_start_index = execution_bars
            .first()
            .and_then(|first| {
                minute_context
                    .iter()
                    .position(|bar| bar.ts_micros == first.ts_micros)
            })
            .ok_or_else(|| {
                "pre-admission Candidate execution start is absent from its complete minute context"
                    .to_owned()
            })?;
        let daily_reference = inputs.daily_reference();
        let signal_calendar = inputs.signal_calendar();
        let execution_calendar = inputs.execution_calendar();
        let signal_bound = inputs.signal_load_bound();
        let minute_bound = inputs.minute_load_bound();
        let daily_bound = inputs.daily_load_bound();
        let signal = PreAdmissionStreamFactsV1::of("signal", signal_bars)?;
        let execution = MeasuredExecutionContextV1::measure(
            minute_context,
            execution_start_index,
            execution_bars,
        )?;
        let composite_data_digest = runner::identity::data_digest_with_daily_reference(
            signal_bars,
            minute_context,
            daily_reference,
        )
        .map_err(|why| format!("pre-admission composite stored-data identity refused: {why:?}"))?;
        let daily = MeasuredDailyReferenceV1::measure(daily_reference)?;
        let candidate_receipt = candidate.receipt();
        if composite_data_digest != candidate_receipt.identities().data_digest() {
            return Err(
                "pre-admission exact signal/minute/daily bytes differ from Candidate composite data identity"
                    .to_owned(),
            );
        }
        require_candidate_stream("signal", candidate_receipt.signal_stream(), signal)?;
        require_candidate_execution(candidate_receipt.execution_stream(), execution.execution)?;
        validate_source_calendars(
            &candidate_receipt,
            signal_bars,
            execution_bars,
            signal_calendar,
            execution_calendar,
        )?;
        Ok(Self {
            candidate: *candidate,
            composite_data_digest,
            signal,
            execution,
            daily,
            signal_calendar,
            execution_calendar,
            signal_bound,
            minute_bound,
            daily_bound,
        })
    }
}

fn validate_source_calendars(
    candidate: &CandidateUniverseReceiptV1,
    signal_bars: &[Candle],
    execution_bars: &[Candle],
    signal_calendar: CompleteCalendarReceiptV2,
    execution_calendar: CompleteCalendarReceiptV2,
) -> Result<(), PreAdmissionDataRefusal> {
    let coverage = candidate.calendar_coverage();
    for (name, supplied, expected_rung, expected_digest) in [
        (
            "signal",
            signal_calendar,
            candidate.rung_seconds(),
            coverage.signal_receipt_digest(),
        ),
        (
            "execution",
            execution_calendar,
            60,
            coverage.execution_receipt_digest(),
        ),
    ] {
        if supplied.rung_seconds() != expected_rung
            || supplied.first_day() != coverage.first_day()
            || supplied.last_day() != coverage.last_day()
            || supplied.digest() != expected_digest
        {
            return Err(format!(
                "pre-admission {name} complete-calendar capability differs from Candidate completion"
            ));
        }
    }
    let recomputed_signal = crate::stored::calendar_receipt_v2_for_bars(
        signal_bars,
        candidate.rung_seconds(),
        signal_calendar.first_day(),
        signal_calendar.last_day(),
    )
    .and_then(super::stored::CalendarReceiptV2::require_complete)
    .map_err(|why| format!("pre-admission exact signal calendar refused: {why}"))?;
    if recomputed_signal != signal_calendar {
        return Err(
            "pre-admission signal calendar is not derived from the exact signal stream".to_owned(),
        );
    }
    let recomputed_execution = crate::stored::calendar_receipt_v2_for_bars(
        execution_bars,
        60,
        execution_calendar.first_day(),
        execution_calendar.last_day(),
    )
    .and_then(super::stored::CalendarReceiptV2::require_complete)
    .map_err(|why| format!("pre-admission exact execution calendar refused: {why}"))?;
    if recomputed_execution != execution_calendar {
        return Err(
            "pre-admission execution calendar is not derived from the exact contiguous execution subslice"
                .to_owned(),
        );
    }
    Ok(())
}

impl PreAdmissionDataV1 {
    fn from_source(source: &PreAdmissionSourceBundleV1) -> Result<Self, PreAdmissionDataRefusal> {
        let candidate = source.candidate.receipt();
        let identities = candidate.identities();
        if source.composite_data_digest != identities.data_digest() {
            return Err(
                "pre-admission measured composite data identity differs from Candidate completion"
                    .to_owned(),
            );
        }
        if identities.calendar_policy_digest() != crate::stored::calendar_policy_digest_v2() {
            return Err(
                "pre-admission Candidate calendar policy is not the canonical measured V2 policy"
                    .to_owned(),
            );
        }
        if source.daily.policy_digest != identities.daily_reference_policy_digest() {
            return Err(
                "pre-admission measured daily-reference policy differs from Candidate completion"
                    .to_owned(),
            );
        }
        let mut value = Self {
            sequence: 0,
            authority_id: [0; 32],
            candidate_universe_id: candidate.universe_id(),
            candidate_completion_digest: candidate.content_digest(),
            candidate_row_count: candidate.row_count(),
            family: candidate.family(),
            rung_seconds: candidate.rung_seconds(),
            horizon_bars: candidate.horizon_bars(),
            requested_span: candidate.requested_span(),
            feed_digest: identities.feed_digest(),
            source_commit_digest: identities.source_commit_digest(),
            calendar_policy_digest: identities.calendar_policy_digest(),
            daily_reference_policy_digest: identities.daily_reference_policy_digest(),
            signal: PreAdmissionExactStreamV1 {
                count: source.signal.count,
                digest: source.signal.digest,
            },
            minute_context: source.execution.minute_context,
            execution: PreAdmissionExactStreamV1 {
                count: source.execution.execution.count,
                digest: source.execution.execution.digest,
            },
            daily: source.daily.daily,
            eligibility: source.daily.eligibility,
            signal_calendar: PreAdmissionCalendarFactsV1::from_complete(source.signal_calendar),
            execution_calendar: PreAdmissionCalendarFactsV1::from_complete(
                source.execution_calendar,
            ),
            signal_load_ceiling: source.signal_bound.max_records(),
            minute_load_ceiling: source.minute_bound.max_records(),
            daily_load_ceiling: source.daily_bound.max_records(),
            execution_start_index: source.execution.execution_start_index,
        };
        value.authority_id = derive_authority_id(&value);
        value.validate()?;
        Ok(value)
    }
}

/// Opaque crate-internal preparation derived from one durable Candidate commit.
///
/// It carries no statistics procedure, assurance, admission or selection
/// policy. Those belong to the later Population Statistics boundary. The only
/// writable operation is the receipt-last append followed by exact read-only
/// reopen below.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProducedPreAdmissionDataV1 {
    value: PreAdmissionDataV1,
}

impl ProducedPreAdmissionDataV1 {
    pub(crate) const fn value(&self) -> PreAdmissionDataV1 {
        self.value
    }

    pub(crate) fn append_and_reopen(
        &self,
        root: impl AsRef<Path>,
        bounds: PreAdmissionDataBoundsV1,
    ) -> Result<PreAdmissionProductionCommitV1, PreAdmissionDataRefusal> {
        let root = root.as_ref();
        let mut ledger = PreAdmissionDataLedgerV1::open(root, bounds)?;
        let committed = ledger.append_complete(&self.value)?;
        let expected = committed.audit();
        drop(ledger);

        let reopened = PreAdmissionDataLedgerV1::open_read(root, bounds)?
            .reopen_audit(&self.value.authority_id())?
            .ok_or_else(|| {
                format!(
                    "pre-admission authority {} disappeared after receipt-last append",
                    hex32(self.value.authority_id())
                )
            })?;
        if reopened != expected || !same_semantics(&reopened.value(), &self.value) {
            return Err(format!(
                "pre-admission authority {} did not reopen with the exact derived semantics",
                hex32(self.value.authority_id())
            ));
        }
        Ok(match committed {
            PreAdmissionProductionCommitV1::Written(_) => {
                PreAdmissionProductionCommitV1::Written(reopened)
            }
            PreAdmissionProductionCommitV1::Reused(_) => {
                PreAdmissionProductionCommitV1::Reused(reopened)
            }
        })
    }
}

/// Derives Pre-Admission Data only from one Candidate production and the exact
/// durable commit returned for that same production.
///
/// The signature deliberately accepts no bars, calendars, load ceilings,
/// digests, scalar identity claims or callbacks. Every semantic field crosses
/// through the opaque Candidate result and is remeasured here.
pub(crate) fn produce_pre_admission_data_v1(
    candidate: &ProducedCandidateUniverseV1<'_>,
    candidate_commit: &CandidateUniverseProductionCommitV1,
) -> Result<ProducedPreAdmissionDataV1, PreAdmissionDataRefusal> {
    let candidate_audit = candidate_commit.audit();
    if candidate_audit.receipt() != candidate.receipt() {
        return Err(
            "pre-admission Candidate production commit does not belong to the supplied produced universe"
                .to_owned(),
        );
    }
    let inputs = candidate.pre_admission_inputs();
    let source = PreAdmissionSourceBundleV1::measure(&candidate_audit, &inputs)?;
    let value = PreAdmissionDataV1::from_source(&source)?;
    if value.candidate_universe_id() != candidate.receipt().universe_id()
        || value.candidate_completion_digest() != candidate.receipt().content_digest()
        || value.candidate_row_count() != candidate.receipt().row_count()
    {
        return Err(
            "pre-admission derived Candidate identity does not exactly bind the produced completion"
                .to_owned(),
        );
    }
    Ok(ProducedPreAdmissionDataV1 { value })
}

/// One decoded, sealed Pre-Admission Data V2 audit row.
///
/// V2 is an append-only successor rather than an in-place relaxation of V1.
/// It retains every V1 source identity and additionally seals the complete
/// Candidate V1 reconciliation.  That typed reconciliation is the only reason
/// a naturally extinct Candidate family may carry zero rows.  Decoding remains
/// audit-only: only [`produce_pre_admission_data_v2`] can mint the opaque
/// production preparation used by the writer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreAdmissionDataV2 {
    core: PreAdmissionDataV1,
    candidate_reconciliation: CompletionReconciliationV2,
}

impl PreAdmissionDataV2 {
    /// Canonical zero-based logical sequence in the V2 ledger.
    #[must_use]
    pub const fn sequence(self) -> u64 {
        self.core.sequence
    }

    /// Domain-separated identity of every source and reconciliation field.
    #[must_use]
    pub const fn authority_id(self) -> [u8; 32] {
        self.core.authority_id
    }

    /// Candidate Universe V1 identity this pre-admission row authenticates.
    #[must_use]
    pub const fn candidate_universe_id(self) -> [u8; 32] {
        self.core.candidate_universe_id
    }

    /// Candidate V1 receipt content digest.
    #[must_use]
    pub const fn candidate_completion_digest(self) -> [u8; 32] {
        self.core.candidate_completion_digest
    }

    /// Exact Candidate V1 row count; zero is legitimate only under the sealed
    /// reconciliation returned by [`Self::candidate_reconciliation`].
    #[must_use]
    pub const fn candidate_row_count(self) -> u64 {
        self.core.candidate_row_count
    }

    /// Exact natural-extinction and closure reconciliation copied from the
    /// authenticated Candidate V1 receipt.
    #[must_use]
    pub const fn candidate_reconciliation(self) -> CompletionReconciliationV2 {
        self.candidate_reconciliation
    }

    /// Exact index family.
    #[must_use]
    pub const fn family(self) -> InstrumentFamilyV1 {
        self.core.family
    }

    /// Exact signal rung in seconds.
    #[must_use]
    pub const fn rung_seconds(self) -> u32 {
        self.core.rung_seconds
    }

    /// Exact one-minute outcome horizon in bars.
    #[must_use]
    pub const fn horizon_bars(self) -> u32 {
        self.core.horizon_bars
    }

    /// Exact requested month span.
    #[must_use]
    pub const fn requested_span(self) -> RequestedSpanIdentityV1 {
        self.core.requested_span
    }

    /// Exact feed identity copied from Candidate V1.
    #[must_use]
    pub const fn feed_digest(self) -> [u8; 32] {
        self.core.feed_digest
    }

    /// Exact source-commit identity copied from Candidate V1.
    #[must_use]
    pub const fn source_commit_digest(self) -> [u8; 32] {
        self.core.source_commit_digest
    }

    /// Exact signal-stream identity.
    #[must_use]
    pub const fn signal(self) -> PreAdmissionExactStreamV1 {
        self.core.signal
    }

    /// Complete stored one-minute context identity.
    #[must_use]
    pub const fn minute_context(self) -> PreAdmissionStreamFactsV1 {
        self.core.minute_context
    }

    /// Exact evaluated one-minute requested-span subslice identity.
    #[must_use]
    pub const fn execution(self) -> PreAdmissionExactStreamV1 {
        self.core.execution
    }

    /// Exact prior-day daily-reference stream identity.
    #[must_use]
    pub const fn daily(self) -> PreAdmissionStreamFactsV1 {
        self.core.daily
    }

    /// Exact ordered daily eligibility and excluded-day evidence.
    #[must_use]
    pub const fn eligibility(self) -> PreAdmissionEligibilityFactsV1 {
        self.core.eligibility
    }

    /// Exact signal-calendar receipt facts.
    #[must_use]
    pub const fn signal_calendar(self) -> PreAdmissionCalendarFactsV1 {
        self.core.signal_calendar
    }

    /// Exact one-minute execution-calendar receipt facts.
    #[must_use]
    pub const fn execution_calendar(self) -> PreAdmissionCalendarFactsV1 {
        self.core.execution_calendar
    }

    /// Canonical calendar-policy identity.
    #[must_use]
    pub const fn calendar_policy_digest(self) -> [u8; 32] {
        self.core.calendar_policy_digest
    }

    /// Canonical daily-reference-policy identity.
    #[must_use]
    pub const fn daily_reference_policy_digest(self) -> [u8; 32] {
        self.core.daily_reference_policy_digest
    }

    /// Exact signal stored-load ceiling.
    #[must_use]
    pub const fn signal_load_ceiling(self) -> u64 {
        self.core.signal_load_ceiling
    }

    /// Exact minute-context stored-load ceiling.
    #[must_use]
    pub const fn minute_load_ceiling(self) -> u64 {
        self.core.minute_load_ceiling
    }

    /// Exact daily-reference stored-load ceiling.
    #[must_use]
    pub const fn daily_load_ceiling(self) -> u64 {
        self.core.daily_load_ceiling
    }

    /// Zero-based location of the exact execution stream inside minute context.
    #[must_use]
    pub const fn execution_start_index(self) -> u64 {
        self.core.execution_start_index
    }

    fn with_sequence(mut self, sequence: u64) -> Self {
        self.core.sequence = sequence;
        self
    }

    fn validate(self) -> Result<(), PreAdmissionDataRefusal> {
        validate_v2_core(&self.core)?;
        validate_v2_reconciliation(self.core.candidate_row_count, self.candidate_reconciliation)?;
        if derive_authority_id_v2(&self) != self.core.authority_id {
            return Err("pre-admission V2 authority identity does not match its fields".to_owned());
        }
        Ok(())
    }

    fn record(self, kind: RecordKindV2) -> Result<[u8; RECORD_BYTES_V2], PreAdmissionDataRefusal> {
        self.validate()?;
        let mut payload = [0_u8; PAYLOAD_BYTES_V2];
        encode_v2_core(&mut payload, &self.core, kind)?;
        encode_v2_reconciliation(
            &mut payload,
            CORE_PAYLOAD_BYTES_V2,
            self.candidate_reconciliation,
        )?;
        let mut record = [0_u8; RECORD_BYTES_V2];
        put_bytes(&mut record, 0, &payload)?;
        put_bytes(
            &mut record,
            PAYLOAD_BYTES_V2,
            &digest_domain(RECORD_DOMAIN_V2, &payload),
        )?;
        Ok(record)
    }

    fn decode(record: &[u8]) -> Result<(RecordKindV2, Self), PreAdmissionDataRefusal> {
        if record.len() != RECORD_BYTES_V2 {
            return Err(format!(
                "pre-admission V2 record is {} bytes, not {RECORD_BYTES_V2}",
                record.len()
            ));
        }
        let payload = record
            .get(..PAYLOAD_BYTES_V2)
            .ok_or_else(|| "pre-admission V2 payload is absent".to_owned())?;
        require_seal(
            "pre-admission V2 record",
            record
                .get(PAYLOAD_BYTES_V2..)
                .ok_or_else(|| "pre-admission V2 record seal is absent".to_owned())?,
            digest_domain(RECORD_DOMAIN_V2, payload),
        )?;
        if get_u32(payload, 0)? != RECORD_VERSION_V2 {
            return Err("pre-admission V2 record version is unknown".to_owned());
        }
        let kind = RecordKindV2::from_byte(get_u32(payload, 4)?)?;
        require_zero(payload, 121, 3, "pre-admission V2 family reserve")?;
        require_zero(payload, 132, 4, "pre-admission V2 horizon reserve")?;
        let requested_span = decode_span(
            payload
                .get(136..156)
                .ok_or_else(|| "pre-admission V2 requested-span bytes are absent".to_owned())?,
        )?;
        let core = PreAdmissionDataV1 {
            sequence: get_u64(payload, 8)?,
            authority_id: get_32(payload, 16)?,
            candidate_universe_id: get_32(payload, 48)?,
            candidate_completion_digest: get_32(payload, 80)?,
            candidate_row_count: get_u64(payload, 112)?,
            family: family_from_byte(
                payload
                    .get(120)
                    .copied()
                    .ok_or_else(|| "pre-admission V2 family byte is absent".to_owned())?,
            )?,
            rung_seconds: get_u32(payload, 124)?,
            horizon_bars: get_u32(payload, 128)?,
            requested_span,
            feed_digest: get_32(payload, 156)?,
            source_commit_digest: get_32(payload, 188)?,
            calendar_policy_digest: get_32(payload, 220)?,
            daily_reference_policy_digest: get_32(payload, 252)?,
            signal: decode_exact_stream(payload, 284)?,
            minute_context: decode_stream(payload, 324)?,
            execution: decode_exact_stream(payload, 380)?,
            daily: decode_stream(payload, 420)?,
            eligibility: decode_eligibility(payload, 476)?,
            signal_calendar: decode_calendar(payload, 564)?,
            execution_calendar: decode_calendar(payload, 620)?,
            signal_load_ceiling: get_u64(payload, 676)?,
            minute_load_ceiling: get_u64(payload, 684)?,
            daily_load_ceiling: get_u64(payload, 692)?,
            execution_start_index: get_u64(payload, 700)?,
        };
        let value = Self {
            core,
            candidate_reconciliation: decode_v2_reconciliation(payload, CORE_PAYLOAD_BYTES_V2)?,
        };
        value.validate()?;
        Ok((kind, value))
    }

    fn from_source(
        source: &PreAdmissionSourceBundleV1,
        reconciliation: CompletionReconciliationV2,
    ) -> Result<Self, PreAdmissionDataRefusal> {
        let candidate = source.candidate.receipt();
        let identities = candidate.identities();
        if source.composite_data_digest != identities.data_digest() {
            return Err(
                "pre-admission V2 measured composite data identity differs from Candidate completion"
                    .to_owned(),
            );
        }
        if identities.calendar_policy_digest() != crate::stored::calendar_policy_digest_v2() {
            return Err(
                "pre-admission V2 Candidate calendar policy is not the canonical measured V2 policy"
                    .to_owned(),
            );
        }
        if source.daily.policy_digest != identities.daily_reference_policy_digest() {
            return Err(
                "pre-admission V2 measured daily-reference policy differs from Candidate completion"
                    .to_owned(),
            );
        }
        if reconciliation != candidate.reconciliation() {
            return Err(
                "pre-admission V2 reconciliation differs from the authenticated Candidate receipt"
                    .to_owned(),
            );
        }
        let core = PreAdmissionDataV1 {
            sequence: 0,
            authority_id: [0; 32],
            candidate_universe_id: candidate.universe_id(),
            candidate_completion_digest: candidate.content_digest(),
            candidate_row_count: candidate.row_count(),
            family: candidate.family(),
            rung_seconds: candidate.rung_seconds(),
            horizon_bars: candidate.horizon_bars(),
            requested_span: candidate.requested_span(),
            feed_digest: identities.feed_digest(),
            source_commit_digest: identities.source_commit_digest(),
            calendar_policy_digest: identities.calendar_policy_digest(),
            daily_reference_policy_digest: identities.daily_reference_policy_digest(),
            signal: PreAdmissionExactStreamV1 {
                count: source.signal.count,
                digest: source.signal.digest,
            },
            minute_context: source.execution.minute_context,
            execution: PreAdmissionExactStreamV1 {
                count: source.execution.execution.count,
                digest: source.execution.execution.digest,
            },
            daily: source.daily.daily,
            eligibility: source.daily.eligibility,
            signal_calendar: PreAdmissionCalendarFactsV1::from_complete(source.signal_calendar),
            execution_calendar: PreAdmissionCalendarFactsV1::from_complete(
                source.execution_calendar,
            ),
            signal_load_ceiling: source.signal_bound.max_records(),
            minute_load_ceiling: source.minute_bound.max_records(),
            daily_load_ceiling: source.daily_bound.max_records(),
            execution_start_index: source.execution.execution_start_index,
        };
        let mut value = Self {
            core,
            candidate_reconciliation: reconciliation,
        };
        value.core.authority_id = derive_authority_id_v2(&value);
        value.validate()?;
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RecordKindV2 {
    Data,
    Completion,
}

impl RecordKindV2 {
    const fn byte(self) -> u32 {
        match self {
            Self::Data => DATA_KIND,
            Self::Completion => COMPLETION_KIND,
        }
    }

    fn from_byte(value: u32) -> Result<Self, PreAdmissionDataRefusal> {
        match value {
            DATA_KIND => Ok(Self::Data),
            COMPLETION_KIND => Ok(Self::Completion),
            _ => Err(format!(
                "pre-admission V2 record kind {value} is unknown; expected 1=data or 2=completion"
            )),
        }
    }
}

/// Explicit V2 logical-row and file-byte refusal ceilings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreAdmissionDataBoundsV2 {
    max_rows: u64,
    max_file_bytes: u64,
}

impl PreAdmissionDataBoundsV2 {
    /// Constructs nonzero bounds large enough for one complete receipt-last pair.
    ///
    /// # Errors
    ///
    /// Refuses a zero row or byte ceiling, and a byte ceiling too small to hold
    /// one header plus the two records a complete receipt-last pair needs. There
    /// is no implicit default; a caller that cannot name a ceiling has not
    /// decided one.
    pub fn new(max_rows: u64, max_file_bytes: u64) -> Result<Self, PreAdmissionDataRefusal> {
        if max_rows == 0 || max_file_bytes == 0 {
            return Err(format!(
                "pre-admission V2 bounds must be nonzero; rows={max_rows}, bytes={max_file_bytes}"
            ));
        }
        let minimum = PRE_ADMISSION_HEADER_BYTES_V2
            .checked_add(
                PRE_ADMISSION_RECORD_STRIDE_V2
                    .checked_mul(2)
                    .ok_or_else(|| {
                        "pre-admission V2 minimum byte bound overflowed u64".to_owned()
                    })?,
            )
            .ok_or_else(|| "pre-admission V2 minimum file size overflowed u64".to_owned())?;
        if max_file_bytes < minimum {
            return Err(format!(
                "pre-admission V2 byte ceiling {max_file_bytes} cannot hold one complete pair; minimum is {minimum}"
            ));
        }
        Ok(Self {
            max_rows,
            max_file_bytes,
        })
    }

    /// Maximum logical Data/Completion pairs, including one trailing orphan.
    #[must_use]
    pub const fn max_rows(self) -> u64 {
        self.max_rows
    }

    /// Maximum complete file bytes, header included.
    #[must_use]
    pub const fn max_file_bytes(self) -> u64 {
        self.max_file_bytes
    }
}

/// Audit-only proof of one exact V2 Data/Completion pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreAdmissionDataReopenAuditV2 {
    data_record_index: u64,
    value: PreAdmissionDataV2,
}

impl PreAdmissionDataReopenAuditV2 {
    /// Physical fixed-record index of the Data member.
    #[must_use]
    pub const fn data_record_index(self) -> u64 {
        self.data_record_index
    }

    /// Logical sequence.
    #[must_use]
    pub const fn sequence(self) -> u64 {
        self.value.sequence()
    }

    /// Complete audit-only V2 value.
    #[must_use]
    pub const fn value(self) -> PreAdmissionDataV2 {
        self.value
    }

    /// Exact V2 authority identity.
    #[must_use]
    pub const fn authority_id(self) -> [u8; 32] {
        self.value.authority_id()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OrphanDataV2 {
    record_index: u64,
    value: PreAdmissionDataV2,
}

/// Open, bounded Pre-Admission Data V2 audit ledger.
#[derive(Debug)]
pub struct PreAdmissionDataLedgerV2 {
    lock_path: PathBuf,
    data_path: PathBuf,
    lock_file: File,
    data_file: File,
    bounds: PreAdmissionDataBoundsV2,
    audits: HashMap<[u8; 32], PreAdmissionDataReopenAuditV2>,
    completed_rows: u64,
    orphan: Option<OrphanDataV2>,
    lock_generation: FileGenerationV1,
    data_generation: FileGenerationV1,
    writable: bool,
}

impl PreAdmissionDataLedgerV2 {
    /// Opens existing V2 files without creating or modifying any path.
    ///
    /// # Errors
    ///
    /// Refuses an absent or non-directory root, a file whose header does not
    /// authenticate, a length that is not a whole number of records, or one past
    /// the admitted ceilings. Reading never creates: a missing file is a refusal
    /// here, not an empty ledger.
    pub fn open_read(
        root: impl AsRef<Path>,
        bounds: PreAdmissionDataBoundsV2,
    ) -> Result<Self, PreAdmissionDataRefusal> {
        Self::open_inner(root.as_ref(), bounds, false)
    }

    fn open(
        root: impl AsRef<Path>,
        bounds: PreAdmissionDataBoundsV2,
    ) -> Result<Self, PreAdmissionDataRefusal> {
        Self::open_inner(root.as_ref(), bounds, true)
    }

    fn open_inner(
        root: &Path,
        bounds: PreAdmissionDataBoundsV2,
        writable: bool,
    ) -> Result<Self, PreAdmissionDataRefusal> {
        let root_metadata = std::fs::metadata(root).map_err(|why| {
            format!(
                "pre-admission V2 ledger root {} must already exist and be admitted: {why}",
                root.display()
            )
        })?;
        if !root_metadata.is_dir() {
            return Err(format!(
                "pre-admission V2 ledger root {} is not a directory",
                root.display()
            ));
        }
        let lock_path = root.join(LOCK_FILE_V2);
        let data_path = root.join(DATA_FILE_V2);
        let lock_file = open_file(&lock_path, writable, writable)?;
        if writable {
            lock_file.lock().map_err(|why| {
                format!(
                    "cannot lock pre-admission V2 writer {}: {why}",
                    lock_path.display()
                )
            })?;
        } else {
            lock_file.lock_shared().map_err(|why| {
                format!(
                    "cannot take shared pre-admission V2 lock {}: {why}",
                    lock_path.display()
                )
            })?;
        }
        let held_lock = lock_file.try_clone().map_err(|why| {
            format!(
                "cannot clone pre-admission V2 lock {}: {why}",
                lock_path.display()
            )
        })?;
        let opened = (|| {
            let mut data_file = open_file(&data_path, writable, writable)?;
            if writable {
                ensure_header_v2(&mut data_file, &data_path)?;
            } else {
                verify_header_v2(&mut data_file, &data_path)?;
            }
            let lock_generation = file_generation_v2(&held_lock, &lock_path)?;
            let data_generation = file_generation_v2(&data_file, &data_path)?;
            let mut ledger = Self {
                lock_path: lock_path.clone(),
                data_path,
                lock_file: held_lock,
                data_file,
                bounds,
                audits: HashMap::new(),
                completed_rows: 0,
                orphan: None,
                lock_generation,
                data_generation,
                writable,
            };
            ledger.scan()?;
            Ok(ledger)
        })();
        let released = lock_file.unlock().map_err(|why| {
            format!(
                "cannot release pre-admission V2 open lock {}: {why}",
                lock_path.display()
            )
        });
        match (opened, released) {
            (Ok(ledger), Ok(())) => Ok(ledger),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn scan(&mut self) -> Result<(), PreAdmissionDataRefusal> {
        verify_header_v2(&mut self.data_file, &self.data_path)?;
        let file_len = self
            .data_file
            .metadata()
            .map_err(|why| {
                format!(
                    "cannot stat pre-admission V2 file {}: {why}",
                    self.data_path.display()
                )
            })?
            .len();
        if file_len > self.bounds.max_file_bytes {
            return Err(format!(
                "pre-admission V2 file has {file_len} bytes, above explicit maximum {}",
                self.bounds.max_file_bytes
            ));
        }
        let physical_records = record_count_v2(file_len)?;
        let logical_rows = physical_records
            .checked_add(1)
            .ok_or_else(|| "pre-admission V2 logical-row count overflowed u64".to_owned())?
            / 2;
        if logical_rows > self.bounds.max_rows {
            return Err(format!(
                "pre-admission V2 file has {logical_rows} logical rows, above explicit maximum {}",
                self.bounds.max_rows
            ));
        }
        let capacity = usize::try_from(physical_records / 2)
            .map_err(|_| "pre-admission V2 completion count does not fit usize".to_owned())?;
        self.audits = HashMap::new();
        self.audits
            .try_reserve(capacity)
            .map_err(|why| format!("cannot reserve pre-admission V2 audit index: {why}"))?;
        self.completed_rows = 0;
        self.orphan = None;
        let mut physical = 0_u64;
        while physical < physical_records {
            let (kind, data) = read_record_v2(&mut self.data_file, physical)?;
            if kind != RecordKindV2::Data {
                return Err(format!(
                    "pre-admission V2 physical record {physical} is a completion without its adjacent Data record"
                ));
            }
            if data.sequence() != self.completed_rows {
                return Err(format!(
                    "pre-admission V2 Data record sequence {} is not canonical {}",
                    data.sequence(),
                    self.completed_rows
                ));
            }
            let completion_index = physical
                .checked_add(1)
                .ok_or_else(|| "pre-admission V2 completion index overflowed u64".to_owned())?;
            if completion_index == physical_records {
                self.orphan = Some(OrphanDataV2 {
                    record_index: physical,
                    value: data,
                });
                break;
            }
            let (completion_kind, completion) =
                read_record_v2(&mut self.data_file, completion_index)?;
            if completion_kind != RecordKindV2::Completion {
                return Err(format!(
                    "pre-admission V2 Data record {physical} is followed by another Data record"
                ));
            }
            if completion != data {
                return Err(format!(
                    "pre-admission V2 completion at physical record {completion_index} does not exactly repeat Data semantics at {physical}"
                ));
            }
            let audit = PreAdmissionDataReopenAuditV2 {
                data_record_index: physical,
                value: data,
            };
            if self.audits.insert(data.authority_id(), audit).is_some() {
                return Err(format!(
                    "pre-admission V2 authority {} appears more than once",
                    hex32(data.authority_id())
                ));
            }
            self.completed_rows = self
                .completed_rows
                .checked_add(1)
                .ok_or_else(|| "pre-admission V2 completed-row cursor overflowed u64".to_owned())?;
            physical = completion_index
                .checked_add(1)
                .ok_or_else(|| "pre-admission V2 physical cursor overflowed u64".to_owned())?;
        }
        self.require_unchanged()
    }

    /// Reopens one completed V2 audit after generation revalidation.
    ///
    /// # Errors
    ///
    /// Refuses if the file changed generation beneath the open handle, if the
    /// named authority is absent, or if its reopened record does not
    /// authenticate against the seal it was written with.
    pub fn reopen_audit(
        &self,
        authority_id: &[u8; 32],
    ) -> Result<Option<PreAdmissionDataReopenAuditV2>, PreAdmissionDataRefusal> {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot take shared pre-admission V2 audit lock: {why}"))?;
        let result = self
            .require_unchanged()
            .map(|()| self.audits.get(authority_id).copied());
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release pre-admission V2 audit lock: {why}"));
        match (result, released) {
            (Ok(audit), Ok(())) => Ok(audit),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn append_complete(
        &mut self,
        prepared: &PreAdmissionDataV2,
    ) -> Result<PreAdmissionProductionCommitV2, PreAdmissionDataRefusal> {
        if !self.writable {
            return Err("pre-admission V2 ledger was opened read-only".to_owned());
        }
        self.lock_file
            .lock()
            .map_err(|why| format!("cannot take pre-admission V2 append lock: {why}"))?;
        let result = self.append_complete_locked(prepared);
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot release pre-admission V2 append lock: {why}"));
        match (result, released) {
            (Ok(outcome), Ok(())) => Ok(outcome),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn append_complete_locked(
        &mut self,
        prepared: &PreAdmissionDataV2,
    ) -> Result<PreAdmissionProductionCommitV2, PreAdmissionDataRefusal> {
        self.require_unchanged()?;
        prepared.validate()?;
        if let Some(existing) = self.audits.get(&prepared.authority_id()).copied() {
            if !same_semantics_v2(&existing.value, prepared) {
                return Err(format!(
                    "pre-admission V2 authority {} is already complete with different semantics",
                    hex32(prepared.authority_id())
                ));
            }
            return Ok(PreAdmissionProductionCommitV2::Reused(existing));
        }
        self.audits
            .try_reserve(1)
            .map_err(|why| format!("cannot reserve pre-admission V2 append index: {why}"))?;
        if let Some(orphan) = self.orphan {
            if !same_semantics_v2(&orphan.value, prepared) {
                return Err(format!(
                    "pre-admission V2 trailing orphan belongs to {}, not exact retry {}; no fallback may hide it",
                    hex32(orphan.value.authority_id()),
                    hex32(prepared.authority_id())
                ));
            }
            self.require_append_bytes(1)?;
            let completion = orphan.value.record(RecordKindV2::Completion)?;
            append_record_v2(&mut self.data_file, &completion)?;
            self.data_file
                .sync_data()
                .map_err(|why| format!("cannot sync pre-admission V2 completion: {why}"))?;
            let audit = PreAdmissionDataReopenAuditV2 {
                data_record_index: orphan.record_index,
                value: orphan.value,
            };
            if self
                .audits
                .insert(orphan.value.authority_id(), audit)
                .is_some()
            {
                return Err(
                    "pre-admission V2 orphan completion collided with an indexed authority"
                        .to_owned(),
                );
            }
            self.completed_rows = self
                .completed_rows
                .checked_add(1)
                .ok_or_else(|| "pre-admission V2 completed-row count overflowed u64".to_owned())?;
            self.orphan = None;
            self.data_generation = file_generation_v2(&self.data_file, &self.data_path)?;
            return Ok(PreAdmissionProductionCommitV2::Written(audit));
        }
        if self.completed_rows >= self.bounds.max_rows {
            return Err(format!(
                "pre-admission V2 completed rows reached explicit maximum {}",
                self.bounds.max_rows
            ));
        }
        self.require_append_bytes(2)?;
        let value = prepared.with_sequence(self.completed_rows);
        value.validate()?;
        let data_record = value.record(RecordKindV2::Data)?;
        let completion_record = value.record(RecordKindV2::Completion)?;
        let data_record_index = self
            .completed_rows
            .checked_mul(2)
            .ok_or_else(|| "pre-admission V2 data-record index overflowed u64".to_owned())?;
        append_record_v2(&mut self.data_file, &data_record)?;
        self.data_file
            .sync_data()
            .map_err(|why| format!("cannot sync pre-admission V2 Data record: {why}"))?;
        self.orphan = Some(OrphanDataV2 {
            record_index: data_record_index,
            value,
        });
        self.data_generation = file_generation_v2(&self.data_file, &self.data_path)?;
        append_record_v2(&mut self.data_file, &completion_record)?;
        self.data_file
            .sync_data()
            .map_err(|why| format!("cannot sync pre-admission V2 completion: {why}"))?;
        let audit = PreAdmissionDataReopenAuditV2 {
            data_record_index,
            value,
        };
        if self.audits.insert(value.authority_id(), audit).is_some() {
            return Err(
                "pre-admission V2 completion collided with an indexed authority".to_owned(),
            );
        }
        self.completed_rows = self
            .completed_rows
            .checked_add(1)
            .ok_or_else(|| "pre-admission V2 completed-row count overflowed u64".to_owned())?;
        self.orphan = None;
        self.data_generation = file_generation_v2(&self.data_file, &self.data_path)?;
        Ok(PreAdmissionProductionCommitV2::Written(audit))
    }

    fn require_append_bytes(&self, records: u64) -> Result<(), PreAdmissionDataRefusal> {
        let current = self
            .data_file
            .metadata()
            .map_err(|why| format!("cannot stat pre-admission V2 append file: {why}"))?
            .len();
        let added = records
            .checked_mul(PRE_ADMISSION_RECORD_STRIDE_V2)
            .ok_or_else(|| "pre-admission V2 append byte count overflowed u64".to_owned())?;
        let desired = current
            .checked_add(added)
            .ok_or_else(|| "pre-admission V2 desired file size overflowed u64".to_owned())?;
        if desired > self.bounds.max_file_bytes {
            return Err(format!(
                "pre-admission V2 append would produce {desired} bytes, above explicit maximum {}",
                self.bounds.max_file_bytes
            ));
        }
        Ok(())
    }

    fn require_unchanged(&self) -> Result<(), PreAdmissionDataRefusal> {
        require_generation_v2(self.lock_generation, &self.lock_file, &self.lock_path)?;
        require_generation_v2(self.data_generation, &self.data_file, &self.data_path)
    }
}

/// Crate-internal result of a V2 append followed by exact read-only reopen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreAdmissionProductionCommitV2 {
    /// New Data bytes and then the receipt-last Completion were synced.
    Written(PreAdmissionDataReopenAuditV2),
    /// Existing bytes were an exact idempotent retry.
    Reused(PreAdmissionDataReopenAuditV2),
}

impl PreAdmissionProductionCommitV2 {
    pub(crate) const fn audit(self) -> PreAdmissionDataReopenAuditV2 {
        match self {
            Self::Written(audit) | Self::Reused(audit) => audit,
        }
    }
}

/// Opaque production preparation derived from an exact Candidate V1 commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProducedPreAdmissionDataV2 {
    value: PreAdmissionDataV2,
}

impl ProducedPreAdmissionDataV2 {
    /// The V2 record this production measured.
    ///
    /// # Why it has no caller yet
    ///
    /// The route hands the PAIR -- produced value and its commit -- to every
    /// V2 successor, because each authenticates one against the other. Reading
    /// the value alone is what a decoder or a `/pre-admission.json` route would
    /// do, and neither exists.
    #[expect(
        dead_code,
        reason = "successors take the authenticated pair; nothing reads the bare value yet"
    )]
    pub(crate) const fn value(&self) -> PreAdmissionDataV2 {
        self.value
    }

    /// Authenticates the exact fresh receipt-last commit consumed by
    /// Observation V2 and returns its canonical Data record.
    ///
    /// The returned bytes contain every V2 source and natural-extinction term;
    /// Observation never accepts a detached digest or caller-authored zero.
    pub(crate) fn authenticated_observation_source(
        &self,
        commit: &PreAdmissionProductionCommitV2,
    ) -> Result<
        (
            PreAdmissionDataReopenAuditV2,
            [u8; PRE_ADMISSION_OBSERVATION_SOURCE_BYTES_V2],
        ),
        PreAdmissionDataRefusal,
    > {
        let audit = commit.audit();
        let value = audit.value();
        value.validate()?;
        if !same_semantics_v2(&value, &self.value) {
            return Err(
                "Observation V2 received a Pre-Admission commit from a foreign production"
                    .to_owned(),
            );
        }
        Ok((audit, value.record(RecordKindV2::Data)?))
    }

    pub(crate) fn append_and_reopen(
        &self,
        root: impl AsRef<Path>,
        bounds: PreAdmissionDataBoundsV2,
    ) -> Result<PreAdmissionProductionCommitV2, PreAdmissionDataRefusal> {
        let root = root.as_ref();
        let mut ledger = PreAdmissionDataLedgerV2::open(root, bounds)?;
        let committed = ledger.append_complete(&self.value)?;
        let expected = committed.audit();
        drop(ledger);
        let reopened = PreAdmissionDataLedgerV2::open_read(root, bounds)?
            .reopen_audit(&self.value.authority_id())?
            .ok_or_else(|| {
                format!(
                    "pre-admission V2 authority {} disappeared after receipt-last append",
                    hex32(self.value.authority_id())
                )
            })?;
        if reopened != expected || !same_semantics_v2(&reopened.value(), &self.value) {
            return Err(format!(
                "pre-admission V2 authority {} did not reopen with exact derived semantics",
                hex32(self.value.authority_id())
            ));
        }
        Ok(match committed {
            PreAdmissionProductionCommitV2::Written(_) => {
                PreAdmissionProductionCommitV2::Written(reopened)
            }
            PreAdmissionProductionCommitV2::Reused(_) => {
                PreAdmissionProductionCommitV2::Reused(reopened)
            }
        })
    }
}

/// Decodes and revalidates the exact embedded Pre-Admission V2 Data record.
///
/// This is available only inside the crate so a successor can authenticate
/// stored bytes without exposing a general-purpose authoring constructor.
pub(crate) fn decode_observation_source_v2(
    record: &[u8; PRE_ADMISSION_OBSERVATION_SOURCE_BYTES_V2],
) -> Result<PreAdmissionDataV2, PreAdmissionDataRefusal> {
    let (kind, value) = PreAdmissionDataV2::decode(record)?;
    if kind != RecordKindV2::Data {
        return Err("Observation V2 source is not a Pre-Admission V2 Data record".to_owned());
    }
    Ok(value)
}

/// Derives V2 only from one Candidate production and its exact durable commit.
///
/// The signature accepts no caller-authored zero marker, reconciliation,
/// source bytes, digest, calendar or load bound.  A zero-row family crosses
/// this boundary only when the authenticated Candidate receipt itself carries
/// natural extinction and complete closure.
pub(crate) fn produce_pre_admission_data_v2(
    candidate: &ProducedCandidateUniverseV1<'_>,
    candidate_commit: &CandidateUniverseProductionCommitV1,
) -> Result<ProducedPreAdmissionDataV2, PreAdmissionDataRefusal> {
    let candidate_audit = candidate_commit.audit();
    if candidate_audit.receipt() != candidate.receipt() {
        return Err(
            "pre-admission V2 Candidate commit does not belong to the supplied produced universe"
                .to_owned(),
        );
    }
    let receipt = candidate_audit.receipt();
    let reconciliation = receipt.reconciliation();
    validate_v2_reconciliation(receipt.row_count(), reconciliation)?;
    if receipt.row_count() == 0
        && (!reconciliation.extinction_complete || !reconciliation.closure_complete)
    {
        return Err(
            "pre-admission V2 zero Candidate lacks natural-extinction and closure proof".to_owned(),
        );
    }
    let inputs = candidate.pre_admission_inputs();
    let source = PreAdmissionSourceBundleV1::measure(&candidate_audit, &inputs)?;
    let value = PreAdmissionDataV2::from_source(&source, reconciliation)?;
    if value.candidate_universe_id() != receipt.universe_id()
        || value.candidate_completion_digest() != receipt.content_digest()
        || value.candidate_row_count() != receipt.row_count()
        || value.candidate_reconciliation() != receipt.reconciliation()
    {
        return Err(
            "pre-admission V2 did not exactly bind the authenticated Candidate completion"
                .to_owned(),
        );
    }
    Ok(ProducedPreAdmissionDataV2 { value })
}

fn validate_v2_core(core: &PreAdmissionDataV1) -> Result<(), PreAdmissionDataRefusal> {
    require_nonzero("pre-admission V2 authority", core.authority_id)?;
    require_nonzero(
        "pre-admission V2 Candidate universe",
        core.candidate_universe_id,
    )?;
    require_nonzero(
        "pre-admission V2 Candidate completion",
        core.candidate_completion_digest,
    )?;
    require_new_production_rung(core.rung_seconds)?;
    if core.horizon_bars == 0 {
        return Err("pre-admission V2 one-minute horizon is zero".to_owned());
    }
    RequestedSpanIdentityV1::new(
        core.requested_span.from_year(),
        core.requested_span.from_month(),
        core.requested_span.to_year(),
        core.requested_span.to_month(),
    )?;
    for (name, digest) in [
        ("feed", core.feed_digest),
        ("source commit", core.source_commit_digest),
        ("calendar policy", core.calendar_policy_digest),
        ("daily-reference policy", core.daily_reference_policy_digest),
    ] {
        require_nonzero(&format!("pre-admission V2 {name}"), digest)?;
    }
    if core.calendar_policy_digest != crate::stored::calendar_policy_digest_v2() {
        return Err("pre-admission V2 calendar-policy identity is not canonical V2".to_owned());
    }
    core.signal.validate("V2 signal stream")?;
    core.minute_context.validate("V2 minute context")?;
    core.execution.validate("V2 execution subspan")?;
    core.daily.validate("V2 daily reference")?;
    core.eligibility.validate(core.daily.count)?;
    core.signal_calendar
        .validate("V2 signal", require_new_production_rung)?;
    core.execution_calendar
        .validate("V2 execution", require_new_production_rung)?;
    if core.signal_calendar.rung_seconds != core.rung_seconds {
        return Err("pre-admission V2 signal calendar differs from signal rung".to_owned());
    }
    if core.execution_calendar.rung_seconds != 60 {
        return Err("pre-admission V2 execution calendar is not sixty seconds".to_owned());
    }
    let (first_day, last_day) = requested_span_days(core.requested_span)?;
    for (name, calendar) in [
        ("signal", core.signal_calendar),
        ("execution", core.execution_calendar),
    ] {
        if calendar.first_day != first_day || calendar.last_day != last_day {
            return Err(format!(
                "pre-admission V2 {name} calendar does not cover the complete requested span"
            ));
        }
    }
    for (name, count, ceiling) in [
        ("signal", core.signal.count, core.signal_load_ceiling),
        (
            "minute",
            core.minute_context.count,
            core.minute_load_ceiling,
        ),
        ("daily", core.daily.count, core.daily_load_ceiling),
    ] {
        if ceiling == 0 {
            return Err(format!("pre-admission V2 {name} load ceiling is zero"));
        }
        if count > ceiling {
            return Err(format!(
                "pre-admission V2 {name} count {count} exceeds load ceiling {ceiling}"
            ));
        }
    }
    let execution_end = core
        .execution_start_index
        .checked_add(core.execution.count)
        .ok_or_else(|| "pre-admission V2 execution end overflowed u64".to_owned())?;
    if execution_end > core.minute_context.count {
        return Err("pre-admission V2 execution subslice escapes minute context".to_owned());
    }
    Ok(())
}

fn validate_v2_reconciliation(
    row_count: u64,
    value: CompletionReconciliationV2,
) -> Result<(), PreAdmissionDataRefusal> {
    if !value.extinction_complete || !value.closure_complete {
        return Err(format!(
            "pre-admission V2 requires natural extinction and complete closure; extinction={}, closure={}",
            value.extinction_complete, value.closure_complete
        ));
    }
    if value.extinction_depth == 0 {
        return Err("pre-admission V2 extinction depth is zero".to_owned());
    }
    if value.unknown_closure_itemsets != 0 {
        return Err(format!(
            "pre-admission V2 retains {} unknown closure verdicts",
            value.unknown_closure_itemsets
        ));
    }
    let frequent = value
        .closed_itemsets
        .checked_add(value.redundant_itemsets)
        .and_then(|count| count.checked_add(value.unknown_closure_itemsets))
        .ok_or_else(|| "pre-admission V2 frequent count overflowed u64".to_owned())?;
    if frequent != value.frequent_itemsets {
        return Err("pre-admission V2 frequent reconciliation disagrees".to_owned());
    }
    let trials = value
        .frequent_itemsets
        .checked_add(value.infrequent_itemsets)
        .ok_or_else(|| "pre-admission V2 trial count overflowed u64".to_owned())?;
    if trials != value.sweep_trials {
        return Err("pre-admission V2 sweep-trial reconciliation disagrees".to_owned());
    }
    let cells = value
        .exit_cells_per_mask
        .long()
        .checked_add(value.exit_cells_per_mask.short())
        .ok_or_else(|| "pre-admission V2 grid width overflowed u64".to_owned())?;
    let expected_rows = value
        .closed_itemsets
        .checked_mul(cells)
        .ok_or_else(|| "pre-admission V2 expected row count overflowed u64".to_owned())?;
    if row_count != expected_rows {
        return Err(format!(
            "pre-admission V2 Candidate rows {row_count} differ from closed masks {} times exact grid width {cells} = {expected_rows}",
            value.closed_itemsets
        ));
    }
    Ok(())
}

fn derive_authority_id_v2(value: &PreAdmissionDataV2) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(AUTHORITY_DOMAIN_V2);
    let core = value.core;
    hasher.update(&core.candidate_universe_id);
    hasher.update(&core.candidate_completion_digest);
    hasher.update(&core.candidate_row_count.to_le_bytes());
    hasher.update(&[family_byte(core.family)]);
    hasher.update(&core.rung_seconds.to_le_bytes());
    hasher.update(&core.horizon_bars.to_le_bytes());
    hasher.update(&core.requested_span.canonical_bytes());
    hasher.update(&core.feed_digest);
    hasher.update(&core.source_commit_digest);
    hasher.update(&core.calendar_policy_digest);
    hasher.update(&core.daily_reference_policy_digest);
    hash_exact_stream(&mut hasher, core.signal);
    hash_stream(&mut hasher, core.minute_context);
    hash_exact_stream(&mut hasher, core.execution);
    hash_stream(&mut hasher, core.daily);
    hash_eligibility(&mut hasher, core.eligibility);
    hash_calendar(&mut hasher, core.signal_calendar);
    hash_calendar(&mut hasher, core.execution_calendar);
    hasher.update(&core.signal_load_ceiling.to_le_bytes());
    hasher.update(&core.minute_load_ceiling.to_le_bytes());
    hasher.update(&core.daily_load_ceiling.to_le_bytes());
    hasher.update(&core.execution_start_index.to_le_bytes());
    hash_v2_reconciliation(&mut hasher, value.candidate_reconciliation);
    hasher.finalize()
}

fn hash_v2_reconciliation(hasher: &mut Hasher, value: CompletionReconciliationV2) {
    for field in [
        value.sweep_trials,
        value.frequent_itemsets,
        value.infrequent_itemsets,
        value.closed_itemsets,
        value.redundant_itemsets,
        value.unknown_closure_itemsets,
        value.exit_cells_per_mask.long(),
        value.exit_cells_per_mask.short(),
    ] {
        hasher.update(&field.to_le_bytes());
    }
    hasher.update(&value.extinction_depth.to_le_bytes());
    hasher.update(&[
        u8::from(value.extinction_complete),
        u8::from(value.closure_complete),
    ]);
}

fn same_semantics_v2(left: &PreAdmissionDataV2, right: &PreAdmissionDataV2) -> bool {
    left.with_sequence(0) == right.with_sequence(0)
}

fn encode_v2_core(
    payload: &mut [u8],
    core: &PreAdmissionDataV1,
    kind: RecordKindV2,
) -> Result<(), PreAdmissionDataRefusal> {
    put_u32(payload, 0, RECORD_VERSION_V2)?;
    put_u32(payload, 4, kind.byte())?;
    put_u64(payload, 8, core.sequence)?;
    put_bytes(payload, 16, &core.authority_id)?;
    put_bytes(payload, 48, &core.candidate_universe_id)?;
    put_bytes(payload, 80, &core.candidate_completion_digest)?;
    put_u64(payload, 112, core.candidate_row_count)?;
    *payload
        .get_mut(120)
        .ok_or_else(|| "pre-admission V2 family encode byte is absent".to_owned())? =
        family_byte(core.family);
    put_u32(payload, 124, core.rung_seconds)?;
    put_u32(payload, 128, core.horizon_bars)?;
    put_bytes(payload, 136, &core.requested_span.canonical_bytes())?;
    put_bytes(payload, 156, &core.feed_digest)?;
    put_bytes(payload, 188, &core.source_commit_digest)?;
    put_bytes(payload, 220, &core.calendar_policy_digest)?;
    put_bytes(payload, 252, &core.daily_reference_policy_digest)?;
    encode_exact_stream(payload, 284, core.signal)?;
    encode_stream(payload, 324, core.minute_context)?;
    encode_exact_stream(payload, 380, core.execution)?;
    encode_stream(payload, 420, core.daily)?;
    encode_eligibility(payload, 476, core.eligibility)?;
    encode_calendar(payload, 564, core.signal_calendar)?;
    encode_calendar(payload, 620, core.execution_calendar)?;
    put_u64(payload, 676, core.signal_load_ceiling)?;
    put_u64(payload, 684, core.minute_load_ceiling)?;
    put_u64(payload, 692, core.daily_load_ceiling)?;
    put_u64(payload, 700, core.execution_start_index)
}

fn encode_v2_reconciliation(
    payload: &mut [u8],
    offset: usize,
    value: CompletionReconciliationV2,
) -> Result<(), PreAdmissionDataRefusal> {
    for (index, field) in [
        value.sweep_trials,
        value.frequent_itemsets,
        value.infrequent_itemsets,
        value.closed_itemsets,
        value.redundant_itemsets,
        value.unknown_closure_itemsets,
        value.exit_cells_per_mask.long(),
        value.exit_cells_per_mask.short(),
    ]
    .into_iter()
    .enumerate()
    {
        put_u64(payload, offset + index * 8, field)?;
    }
    put_u32(payload, offset + 64, value.extinction_depth)?;
    *payload
        .get_mut(offset + 68)
        .ok_or_else(|| "pre-admission V2 extinction flag is absent".to_owned())? =
        u8::from(value.extinction_complete);
    *payload
        .get_mut(offset + 69)
        .ok_or_else(|| "pre-admission V2 closure flag is absent".to_owned())? =
        u8::from(value.closure_complete);
    Ok(())
}

fn decode_v2_reconciliation(
    payload: &[u8],
    offset: usize,
) -> Result<CompletionReconciliationV2, PreAdmissionDataRefusal> {
    require_zero(
        payload,
        offset + 70,
        2,
        "pre-admission V2 reconciliation reserve",
    )?;
    let extinction = payload
        .get(offset + 68)
        .copied()
        .ok_or_else(|| "pre-admission V2 extinction flag is absent".to_owned())?;
    let closure = payload
        .get(offset + 69)
        .copied()
        .ok_or_else(|| "pre-admission V2 closure flag is absent".to_owned())?;
    if extinction > 1 || closure > 1 {
        return Err("pre-admission V2 proof flags are not canonical booleans".to_owned());
    }
    Ok(CompletionReconciliationV2 {
        sweep_trials: get_u64(payload, offset)?,
        frequent_itemsets: get_u64(payload, offset + 8)?,
        infrequent_itemsets: get_u64(payload, offset + 16)?,
        closed_itemsets: get_u64(payload, offset + 24)?,
        redundant_itemsets: get_u64(payload, offset + 32)?,
        unknown_closure_itemsets: get_u64(payload, offset + 40)?,
        exit_cells_per_mask: crate::population::ExitCellsPerMaskV2::new(
            get_u64(payload, offset + 48)?,
            get_u64(payload, offset + 56)?,
        )?,
        extinction_depth: get_u32(payload, offset + 64)?,
        extinction_complete: extinction == 1,
        closure_complete: closure == 1,
    })
}

fn require_candidate_stream(
    name: &str,
    candidate: CandidateSignalStreamV1,
    measured: PreAdmissionStreamFactsV1,
) -> Result<(), PreAdmissionDataRefusal> {
    if candidate.count() != measured.count
        || candidate.first_ts_micros() != measured.first_ts_micros
        || candidate.last_ts_micros() != measured.last_ts_micros
        || candidate.digest() != measured.digest
    {
        return Err(format!(
            "pre-admission exact {name} stream differs from Candidate completion"
        ));
    }
    Ok(())
}

fn require_candidate_execution(
    candidate: CandidateExecutionStreamV1,
    measured: PreAdmissionStreamFactsV1,
) -> Result<(), PreAdmissionDataRefusal> {
    if candidate.count() != measured.count
        || candidate.first_ts_micros() != measured.first_ts_micros
        || candidate.last_ts_micros() != measured.last_ts_micros
        || candidate.digest() != measured.digest
    {
        return Err(
            "pre-admission exact execution subslice differs from Candidate completion".to_owned(),
        );
    }
    Ok(())
}

fn derive_authority_id(value: &PreAdmissionDataV1) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(AUTHORITY_DOMAIN);
    hasher.update(&value.candidate_universe_id);
    hasher.update(&value.candidate_completion_digest);
    hasher.update(&value.candidate_row_count.to_le_bytes());
    hasher.update(&[family_byte(value.family)]);
    hasher.update(&value.rung_seconds.to_le_bytes());
    hasher.update(&value.horizon_bars.to_le_bytes());
    hasher.update(&value.requested_span.canonical_bytes());
    hasher.update(&value.feed_digest);
    hasher.update(&value.source_commit_digest);
    hasher.update(&value.calendar_policy_digest);
    hasher.update(&value.daily_reference_policy_digest);
    hash_exact_stream(&mut hasher, value.signal);
    hash_stream(&mut hasher, value.minute_context);
    hash_exact_stream(&mut hasher, value.execution);
    hash_stream(&mut hasher, value.daily);
    hash_eligibility(&mut hasher, value.eligibility);
    hash_calendar(&mut hasher, value.signal_calendar);
    hash_calendar(&mut hasher, value.execution_calendar);
    hasher.update(&value.signal_load_ceiling.to_le_bytes());
    hasher.update(&value.minute_load_ceiling.to_le_bytes());
    hasher.update(&value.daily_load_ceiling.to_le_bytes());
    hasher.update(&value.execution_start_index.to_le_bytes());
    hasher.finalize()
}

fn same_semantics(left: &PreAdmissionDataV1, right: &PreAdmissionDataV1) -> bool {
    left.with_sequence(0) == right.with_sequence(0)
}

fn hash_exact_stream(hasher: &mut Hasher, value: PreAdmissionExactStreamV1) {
    hasher.update(&value.count.to_le_bytes());
    hasher.update(&value.digest);
}

fn hash_stream(hasher: &mut Hasher, value: PreAdmissionStreamFactsV1) {
    hasher.update(&value.count.to_le_bytes());
    hasher.update(&value.first_ts_micros.to_le_bytes());
    hasher.update(&value.last_ts_micros.to_le_bytes());
    hasher.update(&value.digest);
}

fn hash_eligibility(hasher: &mut Hasher, value: PreAdmissionEligibilityFactsV1) {
    hasher.update(&value.count.to_le_bytes());
    hasher.update(&value.eligible_count.to_le_bytes());
    hasher.update(&value.excluded_day_count.to_le_bytes());
    hasher.update(&value.eligibility_digest);
    hasher.update(&value.excluded_days_digest);
}

fn hash_calendar(hasher: &mut Hasher, value: PreAdmissionCalendarFactsV1) {
    hasher.update(&value.rung_seconds.to_le_bytes());
    hasher.update(&value.first_day.to_le_bytes());
    hasher.update(&value.last_day.to_le_bytes());
    hasher.update(&value.digest);
}

fn encode_exact_stream(
    raw: &mut [u8],
    offset: usize,
    value: PreAdmissionExactStreamV1,
) -> Result<(), PreAdmissionDataRefusal> {
    put_u64(raw, offset, value.count)?;
    put_bytes(raw, offset + 8, &value.digest)
}

fn decode_exact_stream(
    raw: &[u8],
    offset: usize,
) -> Result<PreAdmissionExactStreamV1, PreAdmissionDataRefusal> {
    Ok(PreAdmissionExactStreamV1 {
        count: get_u64(raw, offset)?,
        digest: get_32(raw, offset + 8)?,
    })
}

fn encode_stream(
    raw: &mut [u8],
    offset: usize,
    value: PreAdmissionStreamFactsV1,
) -> Result<(), PreAdmissionDataRefusal> {
    put_u64(raw, offset, value.count)?;
    put_i64(raw, offset + 8, value.first_ts_micros)?;
    put_i64(raw, offset + 16, value.last_ts_micros)?;
    put_bytes(raw, offset + 24, &value.digest)
}

fn decode_stream(
    raw: &[u8],
    offset: usize,
) -> Result<PreAdmissionStreamFactsV1, PreAdmissionDataRefusal> {
    Ok(PreAdmissionStreamFactsV1 {
        count: get_u64(raw, offset)?,
        first_ts_micros: get_i64(raw, offset + 8)?,
        last_ts_micros: get_i64(raw, offset + 16)?,
        digest: get_32(raw, offset + 24)?,
    })
}

fn encode_eligibility(
    raw: &mut [u8],
    offset: usize,
    value: PreAdmissionEligibilityFactsV1,
) -> Result<(), PreAdmissionDataRefusal> {
    put_u64(raw, offset, value.count)?;
    put_u64(raw, offset + 8, value.eligible_count)?;
    put_u64(raw, offset + 16, value.excluded_day_count)?;
    put_bytes(raw, offset + 24, &value.eligibility_digest)?;
    put_bytes(raw, offset + 56, &value.excluded_days_digest)
}

fn decode_eligibility(
    raw: &[u8],
    offset: usize,
) -> Result<PreAdmissionEligibilityFactsV1, PreAdmissionDataRefusal> {
    Ok(PreAdmissionEligibilityFactsV1 {
        count: get_u64(raw, offset)?,
        eligible_count: get_u64(raw, offset + 8)?,
        excluded_day_count: get_u64(raw, offset + 16)?,
        eligibility_digest: get_32(raw, offset + 24)?,
        excluded_days_digest: get_32(raw, offset + 56)?,
    })
}

fn encode_calendar(
    raw: &mut [u8],
    offset: usize,
    value: PreAdmissionCalendarFactsV1,
) -> Result<(), PreAdmissionDataRefusal> {
    put_u32(raw, offset, value.rung_seconds)?;
    put_i64(raw, offset + 8, value.first_day)?;
    put_i64(raw, offset + 16, value.last_day)?;
    put_bytes(raw, offset + 24, &value.digest)
}

fn decode_calendar(
    raw: &[u8],
    offset: usize,
) -> Result<PreAdmissionCalendarFactsV1, PreAdmissionDataRefusal> {
    require_zero(raw, offset + 4, 4, "pre-admission calendar reserve")?;
    Ok(PreAdmissionCalendarFactsV1 {
        rung_seconds: get_u32(raw, offset)?,
        first_day: get_i64(raw, offset + 8)?,
        last_day: get_i64(raw, offset + 16)?,
        digest: get_32(raw, offset + 24)?,
    })
}

fn decode_span(raw: &[u8]) -> Result<RequestedSpanIdentityV1, PreAdmissionDataRefusal> {
    if raw.len() != 20 {
        return Err(format!(
            "pre-admission requested-span identity is {} bytes, not 20",
            raw.len()
        ));
    }
    if get_u32(raw, 0)? != 1 {
        return Err("pre-admission requested-span version is unknown".to_owned());
    }
    let from_year = u16::try_from(get_u32(raw, 4)?)
        .map_err(|_| "pre-admission from-year does not fit u16".to_owned())?;
    let from_month = u8::try_from(get_u32(raw, 8)?)
        .map_err(|_| "pre-admission from-month does not fit u8".to_owned())?;
    let to_year = u16::try_from(get_u32(raw, 12)?)
        .map_err(|_| "pre-admission to-year does not fit u16".to_owned())?;
    let to_month = u8::try_from(get_u32(raw, 16)?)
        .map_err(|_| "pre-admission to-month does not fit u8".to_owned())?;
    let span = RequestedSpanIdentityV1::new(from_year, from_month, to_year, to_month)?;
    if span.canonical_bytes() != raw {
        return Err("pre-admission requested-span bytes are not canonical".to_owned());
    }
    Ok(span)
}

fn requested_span_days(
    span: RequestedSpanIdentityV1,
) -> Result<(i64, i64), PreAdmissionDataRefusal> {
    let first = pull::session::Day::new(span.from_year(), span.from_month(), 1)
        .map_err(|why| format!("pre-admission first requested day is invalid: {why}"))?;
    let last = pull::session::Day::new(span.to_year(), span.to_month(), 1)
        .map_err(|why| format!("pre-admission last requested month is invalid: {why}"))?
        .end_of_month();
    Ok((
        i64::from(first.days_from_epoch()),
        i64::from(last.days_from_epoch()),
    ))
}

fn require_new_production_rung(rung_seconds: u32) -> Result<(), PreAdmissionDataRefusal> {
    if !CANDIDATE_SIGNAL_RUNGS_SECONDS_V1.contains(&rung_seconds) {
        return Err(format!(
            "pre-admission new-production rung {rung_seconds} is not one of the eight Candidate rungs"
        ));
    }
    Ok(())
}

fn require_stored_compatible_rung(rung_seconds: u32) -> Result<(), PreAdmissionDataRefusal> {
    if ![60, 120, 180, 300, 600, 900, 1_800, 3_600, 7_200, 14_400].contains(&rung_seconds) {
        return Err(format!(
            "pre-admission stored rung {rung_seconds} is neither current canonical nor legacy-compatible"
        ));
    }
    Ok(())
}

fn validate_strict_timestamps(name: &str, bars: &[Candle]) -> Result<(), PreAdmissionDataRefusal> {
    if bars.is_empty() {
        return Err(format!("pre-admission {name} stream is empty"));
    }
    for (index, pair) in bars.windows(2).enumerate() {
        let [left, right] = pair else {
            return Err("pre-admission two-record timestamp window changed width".to_owned());
        };
        if left.ts_micros >= right.ts_micros {
            return Err(format!(
                "pre-admission {name} timestamps are not strictly increasing at pair {index}"
            ));
        }
    }
    Ok(())
}

const fn family_byte(family: InstrumentFamilyV1) -> u8 {
    match family {
        InstrumentFamilyV1::Nifty => 1,
        InstrumentFamilyV1::BankNifty => 2,
    }
}

fn family_from_byte(value: u8) -> Result<InstrumentFamilyV1, PreAdmissionDataRefusal> {
    match value {
        1 => Ok(InstrumentFamilyV1::Nifty),
        2 => Ok(InstrumentFamilyV1::BankNifty),
        _ => Err(format!(
            "pre-admission instrument family byte {value} is unknown"
        )),
    }
}

fn digest_bytes(domain: &[u8], count: u64, bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    hasher.update(&count.to_le_bytes());
    hasher.update(bytes);
    hasher.finalize()
}

fn digest_days(count: u64, days: &[i64]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(EXCLUDED_DAYS_DOMAIN);
    hasher.update(&count.to_le_bytes());
    for day in days {
        hasher.update(&day.to_le_bytes());
    }
    hasher.finalize()
}

fn digest_domain(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    hasher.update(bytes);
    hasher.finalize()
}

fn require_nonzero(name: &str, digest: [u8; 32]) -> Result<(), PreAdmissionDataRefusal> {
    if digest == [0; 32] {
        return Err(format!("pre-admission {name} digest is zero"));
    }
    Ok(())
}

fn require_seal(
    name: &str,
    actual: &[u8],
    expected: [u8; 32],
) -> Result<(), PreAdmissionDataRefusal> {
    if actual != expected.as_slice() {
        return Err(format!("{name} seal does not match its payload"));
    }
    Ok(())
}

fn put_bytes(raw: &mut [u8], offset: usize, value: &[u8]) -> Result<(), PreAdmissionDataRefusal> {
    let end = offset
        .checked_add(value.len())
        .ok_or_else(|| "pre-admission encode offset overflowed usize".to_owned())?;
    let raw_len = raw.len();
    let target = raw.get_mut(offset..end).ok_or_else(|| {
        format!("pre-admission encode requested bytes {offset}..{end} from {raw_len}")
    })?;
    target.copy_from_slice(value);
    Ok(())
}

fn put_u32(raw: &mut [u8], offset: usize, value: u32) -> Result<(), PreAdmissionDataRefusal> {
    put_bytes(raw, offset, &value.to_le_bytes())
}

fn put_u64(raw: &mut [u8], offset: usize, value: u64) -> Result<(), PreAdmissionDataRefusal> {
    put_bytes(raw, offset, &value.to_le_bytes())
}

fn put_i64(raw: &mut [u8], offset: usize, value: i64) -> Result<(), PreAdmissionDataRefusal> {
    put_bytes(raw, offset, &value.to_le_bytes())
}

fn get_u32(raw: &[u8], offset: usize) -> Result<u32, PreAdmissionDataRefusal> {
    let bytes = get_array::<4>(raw, offset)?;
    Ok(u32::from_le_bytes(bytes))
}

fn get_u64(raw: &[u8], offset: usize) -> Result<u64, PreAdmissionDataRefusal> {
    let bytes = get_array::<8>(raw, offset)?;
    Ok(u64::from_le_bytes(bytes))
}

fn get_i64(raw: &[u8], offset: usize) -> Result<i64, PreAdmissionDataRefusal> {
    let bytes = get_array::<8>(raw, offset)?;
    Ok(i64::from_le_bytes(bytes))
}

fn get_32(raw: &[u8], offset: usize) -> Result<[u8; 32], PreAdmissionDataRefusal> {
    get_array::<32>(raw, offset)
}

fn get_array<const N: usize>(
    raw: &[u8],
    offset: usize,
) -> Result<[u8; N], PreAdmissionDataRefusal> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| "pre-admission decode offset overflowed usize".to_owned())?;
    raw.get(offset..end)
        .ok_or_else(|| {
            format!(
                "pre-admission decode requested bytes {offset}..{end} from {}",
                raw.len()
            )
        })?
        .try_into()
        .map_err(|_| "pre-admission fixed decode width changed unexpectedly".to_owned())
}

fn require_zero(
    raw: &[u8],
    offset: usize,
    len: usize,
    name: &str,
) -> Result<(), PreAdmissionDataRefusal> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| format!("{name} range overflowed usize"))?;
    let bytes = raw
        .get(offset..end)
        .ok_or_else(|| format!("{name} bytes are absent"))?;
    if bytes.iter().any(|byte| *byte != 0) {
        return Err(format!("{name} is not zero"));
    }
    Ok(())
}

fn header() -> Result<[u8; HEADER_BYTES], PreAdmissionDataRefusal> {
    let mut raw = [0_u8; HEADER_BYTES];
    put_bytes(&mut raw, 0, &HEADER_MAGIC)?;
    put_u32(&mut raw, 16, HEADER_VERSION)?;
    put_u32(&mut raw, 20, HEADER_KIND)?;
    put_u64(&mut raw, 24, PRE_ADMISSION_RECORD_STRIDE_V1)?;
    let seal_input = raw
        .get(..32)
        .ok_or_else(|| "pre-admission header seal input is absent".to_owned())?;
    let seal = digest_domain(HEADER_DOMAIN, seal_input);
    put_bytes(&mut raw, 32, &seal)?;
    Ok(raw)
}

fn ensure_header(file: &mut File, path: &Path) -> Result<(), PreAdmissionDataRefusal> {
    let len = file
        .metadata()
        .map_err(|why| format!("cannot stat {}: {why}", path.display()))?
        .len();
    if len == 0 {
        let bytes = header()?;
        file.seek(SeekFrom::Start(0))
            .and_then(|_| file.write_all(&bytes))
            .and_then(|()| file.sync_data())
            .map_err(|why| format!("cannot initialize {}: {why}", path.display()))?;
        return Ok(());
    }
    verify_header(file, path)
}

fn verify_header(file: &mut File, path: &Path) -> Result<(), PreAdmissionDataRefusal> {
    let len = file
        .metadata()
        .map_err(|why| format!("cannot stat {}: {why}", path.display()))?
        .len();
    if len < PRE_ADMISSION_HEADER_BYTES_V1 {
        return Err(format!(
            "{} is {len} bytes, shorter than the {}-byte pre-admission header",
            path.display(),
            PRE_ADMISSION_HEADER_BYTES_V1
        ));
    }
    let mut observed = [0_u8; HEADER_BYTES];
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_exact(&mut observed))
        .map_err(|why| format!("cannot read {} header: {why}", path.display()))?;
    if observed != header()? {
        return Err(format!(
            "{} pre-admission header is unknown or corrupt",
            path.display()
        ));
    }
    Ok(())
}

fn record_count(file_len: u64) -> Result<u64, PreAdmissionDataRefusal> {
    let body = file_len
        .checked_sub(PRE_ADMISSION_HEADER_BYTES_V1)
        .ok_or_else(|| "pre-admission file is shorter than its header".to_owned())?;
    if !body.is_multiple_of(PRE_ADMISSION_RECORD_STRIDE_V1) {
        return Err(format!(
            "pre-admission file body has {body} bytes, ragged against {PRE_ADMISSION_RECORD_STRIDE_V1}-byte records"
        ));
    }
    Ok(body / PRE_ADMISSION_RECORD_STRIDE_V1)
}

fn record_offset(index: u64) -> Result<u64, PreAdmissionDataRefusal> {
    let body = index
        .checked_mul(PRE_ADMISSION_RECORD_STRIDE_V1)
        .ok_or_else(|| "pre-admission record offset overflowed u64".to_owned())?;
    PRE_ADMISSION_HEADER_BYTES_V1
        .checked_add(body)
        .ok_or_else(|| "pre-admission record offset overflowed header addition".to_owned())
}

fn read_record(
    file: &mut File,
    index: u64,
) -> Result<(RecordKindV1, PreAdmissionDataV1), PreAdmissionDataRefusal> {
    let mut raw = [0_u8; RECORD_BYTES];
    file.seek(SeekFrom::Start(record_offset(index)?))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| format!("cannot read pre-admission record {index}: {why}"))?;
    PreAdmissionDataV1::decode(&raw)
}

fn append_record(file: &mut File, raw: &[u8; RECORD_BYTES]) -> Result<(), PreAdmissionDataRefusal> {
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(raw))
        .map_err(|why| format!("cannot append pre-admission record: {why}"))
}

fn open_file(path: &Path, writable: bool, create: bool) -> Result<File, PreAdmissionDataRefusal> {
    OpenOptions::new()
        .read(true)
        .write(writable)
        .create(create)
        .truncate(false)
        .open(path)
        .map_err(|why| format!("cannot open {}: {why}", path.display()))
}

fn file_generation(file: &File, path: &Path) -> Result<FileGenerationV1, PreAdmissionDataRefusal> {
    let held_before = file
        .metadata()
        .map_err(|why| format!("cannot stat held {}: {why}", path.display()))?;
    let mut named = File::open(path).map_err(|why| {
        format!(
            "cannot reopen named {} for generation: {why}",
            path.display()
        )
    })?;
    let named_before = named
        .metadata()
        .map_err(|why| format!("cannot stat named {}: {why}", path.display()))?;
    #[cfg(unix)]
    if (held_before.dev(), held_before.ino()) != (named_before.dev(), named_before.ino()) {
        return Err(format!(
            "held file no longer names {}; path was replaced",
            path.display()
        ));
    }
    let expected_metadata = generation_of(&held_before, [0; 32]);
    if expected_metadata != generation_of(&named_before, [0; 32]) {
        return Err(format!(
            "{} changed while its generation was measured",
            path.display()
        ));
    }
    let content_digest = hash_file(&mut named, path)?;
    let named_after = named
        .metadata()
        .map_err(|why| format!("cannot restat named {}: {why}", path.display()))?;
    let held_after = file
        .metadata()
        .map_err(|why| format!("cannot restat held {}: {why}", path.display()))?;
    if expected_metadata != generation_of(&named_after, [0; 32])
        || expected_metadata != generation_of(&held_after, [0; 32])
    {
        return Err(format!(
            "{} changed during its generation hash",
            path.display()
        ));
    }
    Ok(generation_of(&held_after, content_digest))
}

#[cfg(unix)]
fn generation_of(metadata: &std::fs::Metadata, content_digest: [u8; 32]) -> FileGenerationV1 {
    FileGenerationV1 {
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
fn generation_of(metadata: &std::fs::Metadata, content_digest: [u8; 32]) -> FileGenerationV1 {
    FileGenerationV1 {
        len: metadata.len(),
        content_digest,
        modified: metadata.modified().ok(),
    }
}

fn hash_file(file: &mut File, path: &Path) -> Result<[u8; 32], PreAdmissionDataRefusal> {
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("cannot seek {} for generation hash: {why}", path.display()))?;
    let mut hasher = Hasher::new();
    hasher.update(GENERATION_DOMAIN);
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|why| format!("cannot hash {} generation: {why}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(
            buffer
                .get(..read)
                .ok_or_else(|| "pre-admission generation read exceeded buffer".to_owned())?,
        );
    }
    Ok(hasher.finalize())
}

fn require_generation(
    expected: FileGenerationV1,
    file: &File,
    path: &Path,
) -> Result<(), PreAdmissionDataRefusal> {
    let observed = file_generation(file, path)?;
    if observed != expected {
        return Err(format!(
            "{} changed after pre-admission open; cached audit refused",
            path.display()
        ));
    }
    Ok(())
}

fn hex32(value: [u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in value {
        for nibble in [byte >> 4, byte & 0x0f] {
            let digit = if nibble < 10 {
                b'0' + nibble
            } else {
                b'a' + (nibble - 10)
            };
            out.push(char::from(digit));
        }
    }
    out
}

fn header_v2() -> Result<[u8; HEADER_BYTES_V2], PreAdmissionDataRefusal> {
    let mut raw = [0_u8; HEADER_BYTES_V2];
    put_bytes(&mut raw, 0, &HEADER_MAGIC_V2)?;
    put_u32(&mut raw, 16, HEADER_VERSION_V2)?;
    put_u32(&mut raw, 20, HEADER_KIND_V2)?;
    put_u64(&mut raw, 24, PRE_ADMISSION_RECORD_STRIDE_V2)?;
    let seal_input = raw
        .get(..32)
        .ok_or_else(|| "pre-admission V2 header seal input is absent".to_owned())?;
    let seal = digest_domain(HEADER_DOMAIN_V2, seal_input);
    put_bytes(&mut raw, 32, &seal)?;
    Ok(raw)
}

fn ensure_header_v2(file: &mut File, path: &Path) -> Result<(), PreAdmissionDataRefusal> {
    let len = file
        .metadata()
        .map_err(|why| format!("cannot stat {}: {why}", path.display()))?
        .len();
    if len == 0 {
        let bytes = header_v2()?;
        file.seek(SeekFrom::Start(0))
            .and_then(|_| file.write_all(&bytes))
            .and_then(|()| file.sync_data())
            .map_err(|why| format!("cannot initialize {}: {why}", path.display()))?;
        return Ok(());
    }
    verify_header_v2(file, path)
}

fn verify_header_v2(file: &mut File, path: &Path) -> Result<(), PreAdmissionDataRefusal> {
    let len = file
        .metadata()
        .map_err(|why| format!("cannot stat {}: {why}", path.display()))?
        .len();
    if len < PRE_ADMISSION_HEADER_BYTES_V2 {
        return Err(format!(
            "{} is {len} bytes, shorter than the V2 pre-admission header",
            path.display()
        ));
    }
    let mut observed = [0_u8; HEADER_BYTES_V2];
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_exact(&mut observed))
        .map_err(|why| format!("cannot read {} V2 header: {why}", path.display()))?;
    if observed != header_v2()? {
        return Err(format!(
            "{} pre-admission V2 header is unknown or corrupt",
            path.display()
        ));
    }
    Ok(())
}

fn record_count_v2(file_len: u64) -> Result<u64, PreAdmissionDataRefusal> {
    let body = file_len
        .checked_sub(PRE_ADMISSION_HEADER_BYTES_V2)
        .ok_or_else(|| "pre-admission V2 file is shorter than its header".to_owned())?;
    if !body.is_multiple_of(PRE_ADMISSION_RECORD_STRIDE_V2) {
        return Err(format!(
            "pre-admission V2 file body has {body} bytes, ragged against {PRE_ADMISSION_RECORD_STRIDE_V2}-byte records"
        ));
    }
    Ok(body / PRE_ADMISSION_RECORD_STRIDE_V2)
}

fn record_offset_v2(index: u64) -> Result<u64, PreAdmissionDataRefusal> {
    let body = index
        .checked_mul(PRE_ADMISSION_RECORD_STRIDE_V2)
        .ok_or_else(|| "pre-admission V2 record offset overflowed u64".to_owned())?;
    PRE_ADMISSION_HEADER_BYTES_V2
        .checked_add(body)
        .ok_or_else(|| "pre-admission V2 record offset overflowed header addition".to_owned())
}

fn read_record_v2(
    file: &mut File,
    index: u64,
) -> Result<(RecordKindV2, PreAdmissionDataV2), PreAdmissionDataRefusal> {
    let mut raw = [0_u8; RECORD_BYTES_V2];
    file.seek(SeekFrom::Start(record_offset_v2(index)?))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| format!("cannot read pre-admission V2 record {index}: {why}"))?;
    PreAdmissionDataV2::decode(&raw)
}

fn append_record_v2(
    file: &mut File,
    raw: &[u8; RECORD_BYTES_V2],
) -> Result<(), PreAdmissionDataRefusal> {
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(raw))
        .map_err(|why| format!("cannot append pre-admission V2 record: {why}"))
}

fn file_generation_v2(
    file: &File,
    path: &Path,
) -> Result<FileGenerationV1, PreAdmissionDataRefusal> {
    let held_before = file
        .metadata()
        .map_err(|why| format!("cannot stat held {}: {why}", path.display()))?;
    let mut named = File::open(path).map_err(|why| {
        format!(
            "cannot reopen named {} for V2 generation: {why}",
            path.display()
        )
    })?;
    let named_before = named
        .metadata()
        .map_err(|why| format!("cannot stat named {}: {why}", path.display()))?;
    #[cfg(unix)]
    if (held_before.dev(), held_before.ino()) != (named_before.dev(), named_before.ino()) {
        return Err(format!(
            "held file no longer names {}; path was replaced",
            path.display()
        ));
    }
    let expected_metadata = generation_of(&held_before, [0; 32]);
    if expected_metadata != generation_of(&named_before, [0; 32]) {
        return Err(format!(
            "{} changed while its V2 generation was measured",
            path.display()
        ));
    }
    let content_digest = hash_file_v2(&mut named, path)?;
    let named_after = named
        .metadata()
        .map_err(|why| format!("cannot restat named {}: {why}", path.display()))?;
    let held_after = file
        .metadata()
        .map_err(|why| format!("cannot restat held {}: {why}", path.display()))?;
    if expected_metadata != generation_of(&named_after, [0; 32])
        || expected_metadata != generation_of(&held_after, [0; 32])
    {
        return Err(format!(
            "{} changed during its V2 generation hash",
            path.display()
        ));
    }
    Ok(generation_of(&held_after, content_digest))
}

fn hash_file_v2(file: &mut File, path: &Path) -> Result<[u8; 32], PreAdmissionDataRefusal> {
    file.seek(SeekFrom::Start(0)).map_err(|why| {
        format!(
            "cannot seek {} for V2 generation hash: {why}",
            path.display()
        )
    })?;
    let mut hasher = Hasher::new();
    hasher.update(GENERATION_DOMAIN_V2);
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|why| format!("cannot hash {} V2 generation: {why}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(
            buffer
                .get(..read)
                .ok_or_else(|| "pre-admission V2 generation read exceeded buffer".to_owned())?,
        );
    }
    Ok(hasher.finalize())
}

fn require_generation_v2(
    expected: FileGenerationV1,
    file: &File,
    path: &Path,
) -> Result<(), PreAdmissionDataRefusal> {
    let observed = file_generation_v2(file, path)?;
    if observed != expected {
        return Err(format!(
            "{} changed after pre-admission V2 open; cached audit refused",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(crate) use tests::{
    observation_v2_nonzero_production_fixture, observation_v2_zero_pair_production_fixture,
    observation_v2_zero_production_fixture,
};

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
#[allow(
    clippy::indexing_slicing,
    reason = "the byte-flip loops walk `0..RECORD_BYTES_V2` over an array of \
              exactly that length, so the index is the loop's own bound. \
              Rewriting them through `.get()` would add an `Option` arm that no \
              input can reach, and `CLAUDE.md` §4 bans a test that asserts \
              nothing -- an unreachable arm is one."
)]
mod tests {
    use super::*;

    type TestResult<T = ()> = Result<T, String>;

    fn must<T, E: std::fmt::Debug>(result: Result<T, E>, context: &str) -> TestResult<T> {
        match result {
            Ok(value) => Ok(value),
            Err(why) => Err(format!("{context}: {why:?}")),
        }
    }

    fn must_refuse<T, E: std::fmt::Debug>(result: Result<T, E>, context: &str) -> TestResult<E> {
        match result {
            Err(why) => Ok(why),
            Ok(_) => Err(format!("{context}: operation unexpectedly succeeded")),
        }
    }

    fn must_some<T>(value: Option<T>, context: &str) -> TestResult<T> {
        match value {
            Some(value) => Ok(value),
            None => Err(format!("{context}: value was absent")),
        }
    }

    fn digest(tag: u8) -> [u8; 32] {
        let mut value = [tag; 32];
        value[31] = tag.wrapping_add(1);
        value
    }

    fn fixture(tag: u8) -> TestResult<PreAdmissionDataV1> {
        let span = must(
            RequestedSpanIdentityV1::new(2024, 1, 2024, 1),
            "fixture span",
        )?;
        let (first_day, last_day) = must(requested_span_days(span), "fixture days")?;
        let mut value = PreAdmissionDataV1 {
            sequence: 0,
            authority_id: [0; 32],
            candidate_universe_id: digest(tag),
            candidate_completion_digest: digest(tag.wrapping_add(1)),
            candidate_row_count: 48,
            family: if tag.is_multiple_of(2) {
                InstrumentFamilyV1::Nifty
            } else {
                InstrumentFamilyV1::BankNifty
            },
            rung_seconds: 300,
            horizon_bars: 32,
            requested_span: span,
            feed_digest: digest(tag.wrapping_add(2)),
            source_commit_digest: digest(tag.wrapping_add(3)),
            calendar_policy_digest: crate::stored::calendar_policy_digest_v2(),
            daily_reference_policy_digest: digest(tag.wrapping_add(4)),
            signal: PreAdmissionExactStreamV1 {
                count: 100,
                digest: digest(tag.wrapping_add(5)),
            },
            minute_context: PreAdmissionStreamFactsV1 {
                count: 520,
                first_ts_micros: 1_000,
                last_ts_micros: 520_000,
                digest: digest(tag.wrapping_add(6)),
            },
            execution: PreAdmissionExactStreamV1 {
                count: 500,
                digest: digest(tag.wrapping_add(7)),
            },
            daily: PreAdmissionStreamFactsV1 {
                count: 24,
                first_ts_micros: 1_000,
                last_ts_micros: 24_000,
                digest: digest(tag.wrapping_add(8)),
            },
            eligibility: PreAdmissionEligibilityFactsV1 {
                count: 24,
                eligible_count: 22,
                excluded_day_count: 2,
                eligibility_digest: digest(tag.wrapping_add(9)),
                excluded_days_digest: digest(tag.wrapping_add(10)),
            },
            signal_calendar: PreAdmissionCalendarFactsV1 {
                rung_seconds: 300,
                first_day,
                last_day,
                digest: digest(tag.wrapping_add(11)),
            },
            execution_calendar: PreAdmissionCalendarFactsV1 {
                rung_seconds: 60,
                first_day,
                last_day,
                digest: digest(tag.wrapping_add(12)),
            },
            signal_load_ceiling: 200,
            minute_load_ceiling: 1_000,
            daily_load_ceiling: 64,
            execution_start_index: 20,
        };
        value.authority_id = derive_authority_id(&value);
        must(value.validate(), "fixture is valid")?;
        Ok(value)
    }

    fn fixture_at_rung(tag: u8, rung_seconds: u32) -> TestResult<PreAdmissionDataV1> {
        let mut value = fixture(tag)?;
        value.rung_seconds = rung_seconds;
        value.signal_calendar.rung_seconds = rung_seconds;
        value.authority_id = derive_authority_id(&value);
        Ok(value)
    }

    fn zero_reconciliation_v2() -> CompletionReconciliationV2 {
        CompletionReconciliationV2 {
            sweep_trials: 1,
            frequent_itemsets: 0,
            infrequent_itemsets: 1,
            closed_itemsets: 0,
            redundant_itemsets: 0,
            unknown_closure_itemsets: 0,
            exit_cells_per_mask: crate::population::ExitCellsPerMaskV2::new(2, 3)
                .expect("fixture grids are nonempty"),
            extinction_depth: 1,
            extinction_complete: true,
            closure_complete: true,
        }
    }

    fn zero_fixture_v2(tag: u8) -> TestResult<PreAdmissionDataV2> {
        let mut core = fixture(tag)?;
        core.candidate_row_count = 0;
        core.authority_id = [0; 32];
        let mut value = PreAdmissionDataV2 {
            core,
            candidate_reconciliation: zero_reconciliation_v2(),
        };
        value.core.authority_id = derive_authority_id_v2(&value);
        must(value.validate(), "zero-row V2 fixture is valid")?;
        Ok(value)
    }

    pub(crate) fn observation_v2_zero_production_fixture(
        tag: u8,
    ) -> TestResult<(ProducedPreAdmissionDataV2, PreAdmissionProductionCommitV2)> {
        let value = zero_fixture_v2(tag)?;
        let audit = PreAdmissionDataReopenAuditV2 {
            data_record_index: 0,
            value,
        };
        Ok((
            ProducedPreAdmissionDataV2 { value },
            PreAdmissionProductionCommitV2::Written(audit),
        ))
    }

    pub(crate) fn observation_v2_zero_pair_production_fixture(
        tag: u8,
    ) -> TestResult<(
        (ProducedPreAdmissionDataV2, PreAdmissionProductionCommitV2),
        (ProducedPreAdmissionDataV2, PreAdmissionProductionCommitV2),
    )> {
        let mut nifty = zero_fixture_v2(tag & !1)?;
        nifty.core.family = InstrumentFamilyV1::Nifty;
        nifty.core.authority_id = [0; 32];
        nifty.core.authority_id = derive_authority_id_v2(&nifty);
        must(nifty.validate(), "paired zero NIFTY fixture is valid")?;

        let mut banknifty = nifty;
        banknifty.core.family = InstrumentFamilyV1::BankNifty;
        banknifty.core.candidate_universe_id = digest(tag.wrapping_add(40));
        banknifty.core.candidate_completion_digest = digest(tag.wrapping_add(41));
        banknifty.core.authority_id = [0; 32];
        banknifty.core.authority_id = derive_authority_id_v2(&banknifty);
        must(
            banknifty.validate(),
            "paired zero BANKNIFTY fixture is valid",
        )?;

        let pair = |value: PreAdmissionDataV2| {
            let audit = PreAdmissionDataReopenAuditV2 {
                data_record_index: 0,
                value,
            };
            (
                ProducedPreAdmissionDataV2 { value },
                PreAdmissionProductionCommitV2::Written(audit),
            )
        };
        Ok((pair(nifty), pair(banknifty)))
    }

    pub(crate) fn observation_v2_nonzero_production_fixture(
        tag: u8,
    ) -> TestResult<(ProducedPreAdmissionDataV2, PreAdmissionProductionCommitV2)> {
        let mut core = fixture(tag)?;
        core.candidate_row_count = 5;
        core.authority_id = [0; 32];
        let reconciliation = CompletionReconciliationV2 {
            sweep_trials: 1,
            frequent_itemsets: 1,
            infrequent_itemsets: 0,
            closed_itemsets: 1,
            redundant_itemsets: 0,
            unknown_closure_itemsets: 0,
            exit_cells_per_mask: crate::population::ExitCellsPerMaskV2::new(2, 3)
                .expect("fixture grids are nonempty"),
            extinction_depth: 2,
            extinction_complete: true,
            closure_complete: true,
        };
        let mut value = PreAdmissionDataV2 {
            core,
            candidate_reconciliation: reconciliation,
        };
        value.core.authority_id = derive_authority_id_v2(&value);
        must(value.validate(), "nonzero-row V2 fixture is valid")?;
        let audit = PreAdmissionDataReopenAuditV2 {
            data_record_index: 0,
            value,
        };
        Ok((
            ProducedPreAdmissionDataV2 { value },
            PreAdmissionProductionCommitV2::Written(audit),
        ))
    }

    struct TestDir(PathBuf);

    impl TestDir {
        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn test_dir() -> TestResult<TestDir> {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nth = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("brutex-pre-admission-{}-{nth}", std::process::id()));
        must(std::fs::create_dir_all(&path), "test directory is writable")?;
        Ok(TestDir(path))
    }

    fn bounds(rows: u64) -> TestResult<PreAdmissionDataBoundsV1> {
        let bytes =
            PRE_ADMISSION_HEADER_BYTES_V1 + PRE_ADMISSION_RECORD_STRIDE_V1 * rows.saturating_mul(2);
        must(PreAdmissionDataBoundsV1::new(rows, bytes), "fixture bounds")
    }

    fn bounds_v2(rows: u64) -> TestResult<PreAdmissionDataBoundsV2> {
        let bytes =
            PRE_ADMISSION_HEADER_BYTES_V2 + PRE_ADMISSION_RECORD_STRIDE_V2 * rows.saturating_mul(2);
        must(
            PreAdmissionDataBoundsV2::new(rows, bytes),
            "V2 fixture bounds",
        )
    }

    #[test]
    fn exact_740_byte_data_and_completion_codecs_bind_every_semantic_field() -> TestResult {
        let value = fixture(10)?;
        let data = must(value.record(RecordKindV1::Data), "Data encodes")?;
        let completion = must(value.record(RecordKindV1::Completion), "completion encodes")?;
        assert_eq!(data.len(), RECORD_BYTES);
        assert_eq!(completion.len(), RECORD_BYTES);
        assert_ne!(data, completion, "kind is sealed independently");
        assert_eq!(
            PreAdmissionDataV1::decode(&data),
            Ok((RecordKindV1::Data, value))
        );
        assert_eq!(
            PreAdmissionDataV1::decode(&completion),
            Ok((RecordKindV1::Completion, value))
        );

        let mut corrupt = data;
        corrupt[476] ^= 1;
        assert!(
            must_refuse(
                PreAdmissionDataV1::decode(&corrupt),
                "changed eligibility bytes break full seal",
            )?
            .contains("seal")
        );

        let mut identities = Vec::new();
        for changed in 0..8 {
            let mut other = value;
            match changed {
                0 => other.feed_digest = digest(80),
                1 => other.source_commit_digest = digest(81),
                2 => other.signal.digest = digest(82),
                3 => other.minute_context.digest = digest(83),
                4 => other.execution.digest = digest(84),
                5 => other.daily.digest = digest(85),
                6 => other.eligibility.eligibility_digest = digest(86),
                7 => other.eligibility.excluded_days_digest = digest(87),
                _ => unreachable!(),
            }
            other.authority_id = derive_authority_id(&other);
            must(other.validate(), "changed semantic remains valid")?;
            identities.push(other.authority_id);
        }
        assert!(
            identities
                .iter()
                .all(|identity| *identity != value.authority_id)
        );
        Ok(())
    }

    #[test]
    fn short_candidate_rungs_write_and_legacy_rungs_remain_read_only_compatible() -> TestResult {
        for (tag, rung_seconds) in [(12, 120), (13, 180)] {
            let value = fixture_at_rung(tag, rung_seconds)?;
            must(
                value.validate(),
                "current Candidate rung validates for production",
            )?;
            let record = must(
                value.record(RecordKindV1::Data),
                "current Candidate rung encodes",
            )?;
            assert_eq!(
                PreAdmissionDataV1::decode(&record),
                Ok((RecordKindV1::Data, value))
            );
        }

        for (tag, rung_seconds) in [(14, 7_200), (15, 14_400)] {
            let value = fixture_at_rung(tag, rung_seconds)?;
            must(
                value.validate_stored_compatible(),
                "legacy stored rung remains readable",
            )?;
            assert!(
                must_refuse(value.validate(), "legacy rung cannot enter new production",)?
                    .contains("new-production rung")
            );
            let record = must(
                value.stored_compatible_record(RecordKindV1::Data),
                "legacy fixture encodes through the test-only compatibility codec",
            )?;
            assert_eq!(
                PreAdmissionDataV1::decode(&record),
                Ok((RecordKindV1::Data, value))
            );
            assert!(
                must_refuse(
                    value.record(RecordKindV1::Data),
                    "production codec must refuse a legacy rung",
                )?
                .contains("new-production rung")
            );
        }

        let unsupported = fixture_at_rung(16, 240)?;
        assert!(
            must_refuse(
                unsupported.validate_stored_compatible(),
                "unknown stored rung remains unreadable",
            )?
            .contains("neither current canonical nor legacy-compatible")
        );
        Ok(())
    }

    #[test]
    fn exact_contiguous_execution_and_daily_eligibility_are_measured_not_claimed() -> TestResult {
        fn candle(ts: i64) -> Candle {
            Candle {
                ts_micros: ts,
                open: ts,
                high: ts + 3,
                low: ts - 2,
                close: ts + 1,
                volume: 10,
                open_interest: 0,
            }
        }
        let minute = [candle(1), candle(2), candle(3), candle(4)];
        let measured = must(
            MeasuredExecutionContextV1::measure(&minute, 1, &minute[1..3]),
            "exact contiguous slice measures",
        )?;
        assert_eq!(measured.execution_start_index, 1);
        assert_eq!(measured.execution.count, 2);
        let foreign = [candle(2), candle(9)];
        assert!(
            must_refuse(
                MeasuredExecutionContextV1::measure(&minute, 1, &foreign),
                "same length with foreign bytes refuses",
            )?
            .contains("not the claimed contiguous")
        );

        let eligibility = [1_u8, 0, 1, 1];
        let reference = DailyReferenceBinding {
            daily_bars: &minute,
            eligibility: &eligibility,
            schema: 1,
            eligibility_policy: 1,
            gap_overlay_policy: 1,
            excluded_ist_days: &CHARTER_NON_REGULAR_IST_DAYS,
            daily_integrity: runner::identity::ReferenceIntegrity::UnverifiedNoReceipt,
            minute_integrity: runner::identity::ReferenceIntegrity::UnverifiedNoReceipt,
            swept_series_calendar_policy: crate::stored::SWEPT_SERIES_CALENDAR_POLICY,
        };
        let daily = must(
            MeasuredDailyReferenceV1::measure(reference),
            "parallel eligibility measures",
        )?;
        assert_eq!(daily.eligibility.count(), 4);
        assert_eq!(daily.eligibility.eligible_count(), 3);
        assert_eq!(
            daily.eligibility.excluded_day_count(),
            must(
                u64::try_from(CHARTER_NON_REGULAR_IST_DAYS.len()),
                "the fixed charter list fits u64",
            )?
        );
        let mut bad = reference;
        bad.eligibility = &[1, 2, 0, 1];
        assert!(
            must_refuse(
                MeasuredDailyReferenceV1::measure(bad),
                "unknown eligibility refuses",
            )?
            .contains("not canonical")
        );

        let mut reordered = CHARTER_NON_REGULAR_IST_DAYS;
        reordered.swap(0, 1);
        let mut foreign_policy = reference;
        foreign_policy.excluded_ist_days = &reordered;
        assert!(
            must_refuse(
                MeasuredDailyReferenceV1::measure(foreign_policy),
                "reordered excluded-day policy refuses without sorting",
            )?
            .contains("exact ordered charter list")
        );
        Ok(())
    }

    #[test]
    fn ledger_is_receipt_last_reopenable_paged_bounded_and_idempotent() -> TestResult {
        let root = test_dir()?;
        let mut ledger = must(
            PreAdmissionDataLedgerV1::open(root.path(), bounds(4)?),
            "ledger initializes",
        )?;
        let first = fixture(20)?;
        let second = fixture(21)?;
        let written = must(ledger.append_complete(&first), "first exact pair writes")?;
        assert!(matches!(
            written,
            PreAdmissionProductionCommitV1::Written(_)
        ));
        let first_audit = written.audit();
        assert_eq!(first_audit.sequence(), 0);
        assert_eq!(first_audit.data_record_index(), 0);
        assert!(matches!(
            must(ledger.append_complete(&first), "exact retry reuses")?,
            PreAdmissionProductionCommitV1::Reused(_)
        ));
        let second_audit = must(ledger.append_complete(&second), "second pair writes")?.audit();
        assert_eq!(second_audit.sequence(), 1);
        assert_eq!(second_audit.data_record_index(), 2);
        assert_eq!(must(ledger.page(0, 2), "page reads")?.rows().len(), 2);
        assert!(
            must_refuse(
                ledger.page(0, MAX_PRE_ADMISSION_PAGE_ROWS_V1 + 1),
                "unbounded page refuses before allocation",
            )?
            .contains("exceeds")
        );
        drop(ledger);

        let reopened = must(
            PreAdmissionDataLedgerV1::open_read(root.path(), bounds(4)?),
            "completed pairs reopen",
        )?;
        assert_eq!(
            must(
                reopened.reopen_audit(&first.authority_id()),
                "generation remains current",
            )?,
            Some(first_audit)
        );
        let page = must(reopened.page(1, 1), "fixed pair reads")?;
        assert_eq!(
            must_some(page.rows().first(), "fixed pair first row")?.sequence(),
            1
        );
        drop(reopened);
        assert!(
            must_refuse(
                PreAdmissionDataLedgerV1::open_read(root.path(), bounds(1)?),
                "logical pair bound refuses before index reserve",
            )?
            .contains("above explicit maximum")
        );
        Ok(())
    }

    #[test]
    fn exact_trailing_data_orphan_resumes_and_foreign_retry_refuses() -> TestResult {
        let root = test_dir()?;
        let configured = bounds(4)?;
        drop(must(
            PreAdmissionDataLedgerV1::open(root.path(), configured),
            "headers initialize",
        )?);
        let value = fixture(30)?.with_sequence(0);
        let mut file = must(
            open_file(&root.path().join(DATA_FILE), true, false),
            "data file reopens",
        )?;
        let data_record = must(value.record(RecordKindV1::Data), "orphan Data encodes")?;
        must(append_record(&mut file, &data_record), "orphan Data writes")?;
        must(file.sync_data(), "orphan Data syncs")?;
        drop(file);

        let mut reopened = must(
            PreAdmissionDataLedgerV1::open(root.path(), configured),
            "one trailing Data orphan is recoverable",
        )?;
        let foreign = fixture(31)?;
        assert!(
            must_refuse(
                reopened.append_complete(&foreign),
                "foreign retry cannot bless orphan",
            )?
            .contains("trailing orphan belongs")
        );
        assert!(matches!(
            must(
                reopened.append_complete(&value),
                "exact retry appends only completion",
            )?,
            PreAdmissionProductionCommitV1::Written(_)
        ));
        assert_eq!(
            must(reopened.page(0, 1), "completed pair reads")?.rows(),
            &[value]
        );
        Ok(())
    }

    #[test]
    fn ragged_corrupt_and_foreign_completion_pairs_fail_closed() -> TestResult {
        let configured = bounds(4)?;
        let ragged_root = test_dir()?;
        drop(must(
            PreAdmissionDataLedgerV1::open(ragged_root.path(), configured),
            "ragged header initializes",
        )?);
        let mut ragged = must(
            open_file(&ragged_root.path().join(DATA_FILE), true, false),
            "ragged file reopens",
        )?;
        must(ragged.seek(SeekFrom::End(0)), "ragged seek")?;
        must(ragged.write_all(&[1]), "ragged byte writes")?;
        must(ragged.sync_data(), "ragged byte syncs")?;
        drop(ragged);
        assert!(
            must_refuse(
                PreAdmissionDataLedgerV1::open_read(ragged_root.path(), configured),
                "ragged fixed record refuses",
            )?
            .contains("ragged")
        );

        let corrupt_root = test_dir()?;
        let mut corrupt = must(
            PreAdmissionDataLedgerV1::open(corrupt_root.path(), configured),
            "corrupt fixture opens",
        )?;
        let corrupt_value = fixture(40)?;
        must(
            corrupt.append_complete(&corrupt_value),
            "clean pair commits",
        )?;
        drop(corrupt);
        let mut file = must(
            open_file(&corrupt_root.path().join(DATA_FILE), true, false),
            "corrupt file reopens",
        )?;
        must(
            file.seek(SeekFrom::Start(PRE_ADMISSION_HEADER_BYTES_V1 + 476)),
            "corrupt seek",
        )?;
        let mut byte = [0_u8; 1];
        must(file.read_exact(&mut byte), "corrupt byte reads")?;
        byte[0] ^= 1;
        must(
            file.seek(SeekFrom::Start(PRE_ADMISSION_HEADER_BYTES_V1 + 476))
                .and_then(|_| file.write_all(&byte))
                .and_then(|()| file.sync_data()),
            "corrupt byte persists",
        )?;
        drop(file);
        assert!(
            must_refuse(
                PreAdmissionDataLedgerV1::open_read(corrupt_root.path(), configured),
                "bad full seal refuses",
            )?
            .contains("seal")
        );

        let foreign_root = test_dir()?;
        drop(must(
            PreAdmissionDataLedgerV1::open(foreign_root.path(), configured),
            "foreign headers initialize",
        )?);
        let first = fixture(41)?.with_sequence(0);
        let foreign = fixture(42)?.with_sequence(0);
        let mut file = must(
            open_file(&foreign_root.path().join(DATA_FILE), true, false),
            "foreign file reopens",
        )?;
        let first_data = must(first.record(RecordKindV1::Data), "Data encodes")?;
        let foreign_completion = must(
            foreign.record(RecordKindV1::Completion),
            "foreign completion encodes",
        )?;
        must(
            append_record(&mut file, &first_data)
                .and_then(|()| append_record(&mut file, &foreign_completion)),
            "foreign pair bytes write",
        )?;
        must(file.sync_data(), "foreign pair syncs")?;
        drop(file);
        assert!(
            must_refuse(
                PreAdmissionDataLedgerV1::open_read(foreign_root.path(), configured),
                "foreign completion cannot bless Data",
            )?
            .contains("does not exactly repeat")
        );
        Ok(())
    }

    #[test]
    fn stale_same_length_mutation_and_nonzero_bounds_fail_closed() -> TestResult {
        assert!(PreAdmissionDataBoundsV1::new(0, 2_000).is_err());
        assert!(PreAdmissionDataBoundsV1::new(1, 64).is_err());
        let root = test_dir()?;
        let configured = bounds(2)?;
        let mut ledger = must(
            PreAdmissionDataLedgerV1::open(root.path(), configured),
            "stale fixture opens",
        )?;
        let stale_value = fixture(50)?;
        let audit = must(
            ledger.append_complete(&stale_value),
            "stale fixture commits",
        )?
        .audit();
        let mut external = must(
            open_file(&root.path().join(DATA_FILE), true, false),
            "external writer opens",
        )?;
        let offset = PRE_ADMISSION_HEADER_BYTES_V1 + 476;
        must(external.seek(SeekFrom::Start(offset)), "stale seek")?;
        let mut byte = [0_u8; 1];
        must(external.read_exact(&mut byte), "stale byte reads")?;
        byte[0] ^= 1;
        must(
            external
                .seek(SeekFrom::Start(offset))
                .and_then(|_| external.write_all(&byte))
                .and_then(|()| external.sync_data()),
            "same-length mutation persists",
        )?;
        drop(external);
        assert!(
            must_refuse(
                ledger.reopen_audit(&audit.authority_id()),
                "content generation catches same-length mutation",
            )?
            .contains("changed")
        );
        assert!(
            must_refuse(ledger.page(0, 1), "page also refuses stale file")?.contains("changed")
        );
        Ok(())
    }

    #[test]
    fn writer_refuses_missing_or_non_directory_root_without_creating_it() -> TestResult {
        let parent = test_dir()?;
        let configured = bounds(2)?;
        let missing = parent.path().join("missing-ledger-root");
        assert!(
            must_refuse(
                PreAdmissionDataLedgerV1::open(&missing, configured),
                "missing ledger root must refuse",
            )?
            .contains("must already exist")
        );
        assert!(!missing.exists(), "writer recreated its missing root");

        let not_directory = parent.path().join("ledger-root-file");
        must(
            File::create(&not_directory),
            "non-directory root fixture is created",
        )?;
        assert!(
            must_refuse(
                PreAdmissionDataLedgerV1::open(&not_directory, configured),
                "non-directory ledger root must refuse",
            )?
            .contains("is not a directory")
        );
        Ok(())
    }

    #[test]
    fn v2_zero_family_codec_requires_every_extinction_proof_term() -> TestResult {
        let value = zero_fixture_v2(180)?;
        let data = must(value.record(RecordKindV2::Data), "V2 Data encodes")?;
        let completion = must(
            value.record(RecordKindV2::Completion),
            "V2 Completion encodes",
        )?;
        assert_eq!(data.len(), RECORD_BYTES_V2);
        assert_ne!(data, completion, "V2 record kind is inside the seal");
        assert_eq!(
            PreAdmissionDataV2::decode(&data),
            Ok((RecordKindV2::Data, value))
        );
        assert_eq!(value.candidate_row_count(), 0);
        assert!(value.candidate_reconciliation().extinction_complete);
        assert!(value.candidate_reconciliation().closure_complete);

        let v1 = fixture(181)?;
        let v1_bytes = must(v1.record(RecordKindV1::Data), "V1 still encodes")?;
        assert_eq!(v1_bytes.len(), RECORD_BYTES);
        assert_eq!(
            PreAdmissionDataV1::decode(&v1_bytes),
            Ok((RecordKindV1::Data, v1))
        );
        assert!(
            PreAdmissionDataV1::decode(&data).is_err(),
            "separate V2 bytes cannot be mistaken for V1"
        );

        for index in 0..RECORD_BYTES_V2 {
            let mut changed = data;
            changed[index] ^= 1;
            assert!(
                PreAdmissionDataV2::decode(&changed).is_err(),
                "unresealed V2 mutation at byte {index} must fail"
            );
        }

        let mut missing = Vec::new();
        let mut no_extinction = value;
        no_extinction.candidate_reconciliation.extinction_complete = false;
        missing.push(no_extinction);
        let mut no_closure = value;
        no_closure.candidate_reconciliation.closure_complete = false;
        missing.push(no_closure);
        let mut no_depth = value;
        no_depth.candidate_reconciliation.extinction_depth = 0;
        missing.push(no_depth);
        let mut unknown = value;
        unknown.candidate_reconciliation.unknown_closure_itemsets = 1;
        missing.push(unknown);
        let mut unexplained_rows = value;
        unexplained_rows.candidate_reconciliation.closed_itemsets = 1;
        unexplained_rows.candidate_reconciliation.frequent_itemsets = 1;
        unexplained_rows.candidate_reconciliation.sweep_trials = 2;
        missing.push(unexplained_rows);
        for mut changed in missing {
            changed.core.authority_id = derive_authority_id_v2(&changed);
            assert!(
                changed.validate().is_err(),
                "one missing or contradictory zero-family proof term must refuse"
            );
        }

        let mut detached = value;
        detached.core.candidate_completion_digest = [0; 32];
        detached.core.authority_id = derive_authority_id_v2(&detached);
        assert!(
            must_refuse(
                detached.validate(),
                "bare zero without Candidate seal refuses"
            )?
            .contains("completion")
        );

        let mut resealed = data;
        resealed[CORE_PAYLOAD_BYTES_V2 + 68] = 0;
        let new_seal = digest_domain(RECORD_DOMAIN_V2, &resealed[..PAYLOAD_BYTES_V2]);
        resealed[PAYLOAD_BYTES_V2..].copy_from_slice(&new_seal);
        assert!(
            must_refuse(
                PreAdmissionDataV2::decode(&resealed),
                "resealed missing-extinction flag refuses",
            )?
            .contains("natural extinction")
        );
        Ok(())
    }

    #[test]
    fn v2_zero_family_ledger_is_receipt_last_reopenable_and_exactly_idempotent() -> TestResult {
        let root = test_dir()?;
        let configured = bounds_v2(4)?;
        let mut ledger = must(
            PreAdmissionDataLedgerV2::open(root.path(), configured),
            "V2 ledger initializes",
        )?;
        let value = zero_fixture_v2(182)?;
        let written = must(ledger.append_complete(&value), "zero V2 pair writes")?;
        assert!(matches!(
            written,
            PreAdmissionProductionCommitV2::Written(_)
        ));
        let audit = written.audit();
        assert_eq!(audit.sequence(), 0);
        assert_eq!(audit.data_record_index(), 0);
        assert_eq!(audit.value().candidate_row_count(), 0);
        assert!(matches!(
            must(ledger.append_complete(&value), "exact V2 retry reuses")?,
            PreAdmissionProductionCommitV2::Reused(_)
        ));
        drop(ledger);
        let reopened = must(
            PreAdmissionDataLedgerV2::open_read(root.path(), configured),
            "V2 completed pair reopens",
        )?;
        assert_eq!(
            must(
                reopened.reopen_audit(&value.authority_id()),
                "V2 generation remains current",
            )?,
            Some(audit)
        );

        let orphan_root = test_dir()?;
        drop(must(
            PreAdmissionDataLedgerV2::open(orphan_root.path(), configured),
            "V2 orphan header initializes",
        )?);
        let orphan = zero_fixture_v2(183)?.with_sequence(0);
        let mut file = must(
            open_file(&orphan_root.path().join(DATA_FILE_V2), true, false),
            "V2 orphan file opens",
        )?;
        must(
            append_record_v2(
                &mut file,
                &must(orphan.record(RecordKindV2::Data), "V2 orphan encodes")?,
            ),
            "V2 orphan appends",
        )?;
        must(file.sync_data(), "V2 orphan syncs")?;
        drop(file);
        let mut orphan_ledger = must(
            PreAdmissionDataLedgerV2::open(orphan_root.path(), configured),
            "V2 orphan is recoverable",
        )?;
        let foreign = zero_fixture_v2(184)?;
        assert!(
            must_refuse(
                orphan_ledger.append_complete(&foreign),
                "foreign V2 retry cannot bless orphan",
            )?
            .contains("trailing orphan belongs")
        );
        assert!(matches!(
            must(
                orphan_ledger.append_complete(&orphan),
                "exact V2 orphan retry appends only Completion",
            )?,
            PreAdmissionProductionCommitV2::Written(_)
        ));
        Ok(())
    }

    #[test]
    fn v2_corrupt_resealed_ragged_and_stale_files_fail_closed() -> TestResult {
        let configured = bounds_v2(3)?;
        let ragged_root = test_dir()?;
        drop(must(
            PreAdmissionDataLedgerV2::open(ragged_root.path(), configured),
            "V2 ragged header initializes",
        )?);
        let mut ragged = must(
            open_file(&ragged_root.path().join(DATA_FILE_V2), true, false),
            "V2 ragged file opens",
        )?;
        must(ragged.seek(SeekFrom::End(0)), "V2 ragged seek")?;
        must(ragged.write_all(&[1]), "V2 ragged byte writes")?;
        must(ragged.sync_data(), "V2 ragged syncs")?;
        drop(ragged);
        assert!(
            must_refuse(
                PreAdmissionDataLedgerV2::open_read(ragged_root.path(), configured),
                "V2 ragged record refuses",
            )?
            .contains("ragged")
        );

        let resealed_root = test_dir()?;
        drop(must(
            PreAdmissionDataLedgerV2::open(resealed_root.path(), configured),
            "V2 resealed header initializes",
        )?);
        let value = zero_fixture_v2(185)?.with_sequence(0);
        let mut invalid_data = must(value.record(RecordKindV2::Data), "V2 Data encodes")?;
        let mut invalid_completion = must(
            value.record(RecordKindV2::Completion),
            "V2 Completion encodes",
        )?;
        for raw in [&mut invalid_data, &mut invalid_completion] {
            raw[CORE_PAYLOAD_BYTES_V2 + 69] = 0;
            let seal = digest_domain(RECORD_DOMAIN_V2, &raw[..PAYLOAD_BYTES_V2]);
            raw[PAYLOAD_BYTES_V2..].copy_from_slice(&seal);
        }
        let mut resealed_file = must(
            open_file(&resealed_root.path().join(DATA_FILE_V2), true, false),
            "V2 resealed file opens",
        )?;
        must(
            append_record_v2(&mut resealed_file, &invalid_data)
                .and_then(|()| append_record_v2(&mut resealed_file, &invalid_completion)),
            "V2 resealed pair writes",
        )?;
        must(resealed_file.sync_data(), "V2 resealed pair syncs")?;
        drop(resealed_file);
        assert!(
            must_refuse(
                PreAdmissionDataLedgerV2::open_read(resealed_root.path(), configured),
                "resealed missing closure refuses",
            )?
            .contains("complete closure")
        );

        let stale_root = test_dir()?;
        let mut stale = must(
            PreAdmissionDataLedgerV2::open(stale_root.path(), configured),
            "V2 stale fixture opens",
        )?;
        let stale_value = zero_fixture_v2(186)?;
        let stale_audit = must(
            stale.append_complete(&stale_value),
            "V2 stale fixture commits",
        )?
        .audit();
        let mut external = must(
            open_file(&stale_root.path().join(DATA_FILE_V2), true, false),
            "V2 external writer opens",
        )?;
        let offset = PRE_ADMISSION_HEADER_BYTES_V2 + 220;
        must(external.seek(SeekFrom::Start(offset)), "V2 stale seek")?;
        let mut byte = [0_u8; 1];
        must(external.read_exact(&mut byte), "V2 stale byte reads")?;
        byte[0] ^= 1;
        must(
            external
                .seek(SeekFrom::Start(offset))
                .and_then(|_| external.write_all(&byte))
                .and_then(|()| external.sync_data()),
            "V2 same-length mutation persists",
        )?;
        drop(external);
        assert!(
            must_refuse(
                stale.reopen_audit(&stale_audit.authority_id()),
                "V2 content generation catches same-length mutation",
            )?
            .contains("changed")
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn v2_replaced_lock_and_data_paths_refuse_cached_zero_family_audits() -> TestResult {
        for (tag, name) in [LOCK_FILE_V2, DATA_FILE_V2].into_iter().enumerate() {
            let root = test_dir()?;
            let configured = bounds_v2(2)?;
            let mut ledger = must(
                PreAdmissionDataLedgerV2::open(root.path(), configured),
                "V2 replacement fixture opens",
            )?;
            let fixture_tag = must(u8::try_from(190 + tag), "two V2 tags fit u8")?;
            let value = zero_fixture_v2(fixture_tag)?;
            let audit = must(
                ledger.append_complete(&value),
                "V2 replacement fixture commits",
            )?
            .audit();
            let named = root.path().join(name);
            let displaced = root.path().join(format!("{name}.displaced"));
            must(std::fs::rename(&named, &displaced), "V2 held inode moves")?;
            must(File::create(&named), "V2 replacement path is created")?;
            assert!(
                must_refuse(
                    ledger.reopen_audit(&audit.authority_id()),
                    "V2 replacement invalidates cached audit",
                )?
                .contains("no longer names"),
                "replacement of V2 {name} did not fail closed"
            );
        }
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn replaced_lock_and_data_paths_refuse_cached_audits() -> TestResult {
        for (tag, name) in [LOCK_FILE, DATA_FILE].into_iter().enumerate() {
            let root = test_dir()?;
            let configured = bounds(2)?;
            let mut ledger = must(
                PreAdmissionDataLedgerV1::open(root.path(), configured),
                "replacement fixture opens",
            )?;
            let fixture_tag = must(u8::try_from(60 + tag), "two tags fit u8")?;
            let replacement = fixture(fixture_tag)?;
            let audit = must(
                ledger.append_complete(&replacement),
                "replacement fixture commits",
            )?
            .audit();
            let named = root.path().join(name);
            let displaced = root.path().join(format!("{name}.displaced"));
            must(
                std::fs::rename(&named, &displaced),
                "held inode is displaced",
            )?;
            must(File::create(&named), "replacement path is created")?;
            assert!(
                must_refuse(
                    ledger.reopen_audit(&audit.authority_id()),
                    "replacement invalidates cached audit",
                )?
                .contains("no longer names"),
                "replacement of {name} did not fail closed"
            );
        }
        Ok(())
    }
}
