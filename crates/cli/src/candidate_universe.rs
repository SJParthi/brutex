//! Durable pre-admission candidates for the two swept NSE index families.
//!
//! Population V4 contains an admission verdict in every row. That makes it
//! unsuitable as the source from which the evidence needed to decide admission
//! is derived: admission would depend on a population that already embeds the
//! answer. This module breaks that cycle. It stores the raw, directly measured
//! [`Cell`] facts first, without statistics, assurance, admission, ranking or
//! selection fields. A completion receipt is synced only after its contiguous
//! row block.
//!
//! The two files are fixed-stride and independently versioned. Opening is
//! O(rows + receipts) and builds a bounded hash index. After open, a structural
//! audit lookup and one row seek are O(1) in record count (plus bounded file
//! generation checks); a page is O(page length), and the sealed internal append
//! is O(new rows). No whole-ledger operation is described as O(1).
//!
//! Crate-internal production preparation is available only through
//! [`CandidateUniverseProductionSourceV1`]. That opaque source bundle rebuilds
//! the anchored signal column from the exact typed daily records, replaces
//! signal-local `GapFib` with the complete exact-minute context, derives the
//! checked one-minute execution column, consumes the naturally-extinct closed
//! frontier and expands both complete validated grids. It is deliberately not
//! a public authoring capability: the presently available typed inputs can
//! prove their mutual identity but cannot attest that their bytes came from the
//! concrete stored-data authority. The crate-internal stored-authority
//! orchestrator is therefore its only production owner. Public callers can
//! audit and page sealed bytes; they cannot supply a descriptor, row, digest,
//! directional run or writable ledger.
//!
//! Preparation is input-dependent at every structural layer: the naturally
//! extinct signal sweep, one signal-bar support/session scan per closed mask,
//! both complete grid evaluations, and an exact execution-series replay plus
//! `TradeRow` evidence folds for every grid cell. During production it retains
//! the engine result, the finished Candidate row block, one fixed Base Evidence
//! record per Candidate, and accepted-session observations whose size is
//! candidate count times retained period width. The bounded support-column
//! copy, one evaluated grid and one cell's materialized trades are transient.
//! This is not O(1) whole-run time or space. The constant-cost claims remain
//! the fixed mask/cell primitives, fixed record projection and a validated
//! fixed-offset row seek.
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

#[path = "population_base_evidence_v2.rs"]
pub(crate) mod population_base_evidence_v2;

#[path = "boolean_candidate_v1.rs"]
pub mod boolean_candidate_v1;

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{FileExt as _, MetadataExt as _};

use brutex_core::blake3::{Hasher, hash};
use indicators::Candle;
use indicators::anchored::{
    AnchoredEvaluator, DailyEligibility, DailyReference, overlay_exact_minute_orb_and_gapfib,
};
use indicators::column::{AnchoredColumn, Column};
use indicators::evaluator::{CHARTER_NON_REGULAR_IST_DAYS, Calendar, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use pull::session::Day;
use runner::excursion::Side;
use runner::exit_grid_policy::{
    ExecutionDispositionV1, ExecutionResolutionV1, ExecutionRunV1, ExecutionSeriesV1,
    ExitGridSelectorV1, ForcedStopV1, GlobalReplayWitnessUniverseV1, OosExecutionSeriesV1,
    RangeResolutionV1, RationalPercentileV1, ResolvedExitGridV1, ValidatedExitGridV1,
    column_digest_v1, instrument_digest_v1,
};
use runner::grid::{Cell, Chosen, Ttp, materialize_cell};
use runner::identity::{DailyReferenceBinding, Direction, Params, Run};
use runner::outcome::Horizon;
use runner::validate::{
    AnchoredSearchValidationV4, walk_forward_projected_prepared_anchored_search_v4,
};
use runner::{ClosureVerdict, PopulationMember, PopulationRun, Sweeper};

use crate::population::{
    ClosureV1, CompletionReconciliationV2, ExitCellsPerMaskV2, ExitCoordinateV1,
    InstrumentFamilyV1, LongShortExitGridIdentitiesV2, RequestedSpanIdentityV1,
    SideExitGridIdentityV2, TradeDirectionV1,
};
use crate::population_observations_v1::{
    CandidateFamilyObservationsV1, CandidateObservationBuilderV1,
};
use crate::stored::{CompleteCalendarReceiptV2, StoredSpanLoadBoundV1};

pub use self::population_base_evidence_v2::{
    BaseEvidenceLedgerBoundsV2, BaseEvidenceLedgerReaderV2, BaseEvidenceLedgerRefusalV2,
    BaseEvidenceRecordProjectionV2, BaseEvidenceReopenAuditV2,
};
pub(crate) use self::population_base_evidence_v2::{
    BaseEvidenceProductionCommitV2, PairedBaseEvidenceAuthorityV2, PairedBaseEvidenceReaderV2,
    PairedBaseEvidenceRecordProjectionV2,
};

use self::population_base_evidence_v2::{
    BaseEvidenceBoundsV2, BaseEvidenceBuilderV2, MaskSupportEvidenceV2, PreparedBaseEvidenceV2,
    append_and_reopen_base_evidence_v2, candidate_row_digest_v2, measure_mask_support_v2,
    pair_base_evidence_authority_v2,
};

#[cfg(test)]
use self::population_base_evidence_v2::fixture_prepared_base_evidence_v2;

/// Operator-facing refusal emitted by the candidate-universe boundary.
pub type CandidateUniverseRefusal = String;

/// Fixed header bytes in each candidate-universe file.
pub const HEADER_BYTES_V1: u64 = 64;
/// Bytes in one candidate row, including its full BLAKE3 record seal.
pub const CANDIDATE_ROW_STRIDE_V1: u64 = 480;
/// Bytes in one completion receipt, including its full BLAKE3 record seal.
pub const CANDIDATE_RECEIPT_STRIDE_V1: u64 = 976;
/// Largest row page one call may allocate.
pub const MAX_CANDIDATE_PAGE_ROWS_V1: u64 = 256;

const HEADER_BYTES: usize = 64;
const ROW_PAYLOAD_BYTES: usize = 448;
const ROW_STRIDE_BYTES: usize = 480;
const RECEIPT_PAYLOAD_BYTES: usize = 944;
const RECEIPT_STRIDE_BYTES: usize = 976;
const CALENDAR_COVERAGE_BYTES: usize = 96;
const SPAN_BYTES: usize = 20;
const RECORD_SEAL_BYTES: usize = 32;
const ROW_WRITE_CHUNK_ROWS: usize = 32;
const ROW_WRITE_CHUNK_BYTES: usize = ROW_WRITE_CHUNK_ROWS * ROW_STRIDE_BYTES;
const MASK_WORDS_V1: usize = 6;
const ROW_VERSION: u32 = 1;
const RECEIPT_VERSION: u32 = 1;
const HEADER_VERSION: u32 = 1;
const ROW_KIND: u32 = 1;
const RECEIPT_KIND: u32 = 2;
const NONE_U32: u32 = u32::MAX;
const ROW_MAGIC: [u8; 16] = *b"BTX-CAND-UROW-V1";
const RECEIPT_MAGIC: [u8; 16] = *b"BTX-CAND-UREC-V1";
const ROW_FILE: &str = "candidate-universe-rows-v1.bin";
const RECEIPT_FILE: &str = "candidate-universe-completions-v1.bin";
const LOCK_FILE: &str = "candidate-universe-write-v1.lock";
const HEADER_DOMAIN: &[u8] = b"brutex-candidate-universe-header-v1\0";
const ROW_SEAL_DOMAIN: &[u8] = b"brutex-candidate-universe-row-record-v1\0";
const RECEIPT_CONTENT_DOMAIN: &[u8] = b"brutex-candidate-universe-receipt-v1\0";
const UNIVERSE_ID_DOMAIN: &[u8] = b"brutex-candidate-universe-id-v1\0";
const CANDIDATE_ID_DOMAIN: &[u8] = b"brutex-candidate-semantic-id-v1\0";
const ORDERED_ROWS_DOMAIN: &[u8] = b"brutex-candidate-universe-ordered-rows-v1\0";
const VOCABULARY_ID_DOMAIN: &[u8] = b"brutex-candidate-universe-vocabulary-v1\0";
const EVALUATION_POLICY_ID_DOMAIN: &[u8] = b"brutex-candidate-universe-evaluation-policy-v1\0";
const PRODUCTION_SOURCE_ID_DOMAIN: &[u8] = b"brutex-candidate-universe-production-source-v1\0";
const GLOBAL_REPLAY_OOS_SOURCE_ID_DOMAIN: &[u8] = b"brutex-candidate-global-replay-oos-source-v1\0";
const EXECUTION_RUN_POLICY_VERSION_V1: u64 = 1;

const _: () = assert!(HEADER_BYTES as u64 == HEADER_BYTES_V1);
const _: () = assert!(ROW_STRIDE_BYTES as u64 == CANDIDATE_ROW_STRIDE_V1);
const _: () = assert!(RECEIPT_STRIDE_BYTES as u64 == CANDIDATE_RECEIPT_STRIDE_V1);
const _: () = assert!(ROW_PAYLOAD_BYTES + RECORD_SEAL_BYTES == ROW_STRIDE_BYTES);
const _: () = assert!(RECEIPT_PAYLOAD_BYTES + RECORD_SEAL_BYTES == RECEIPT_STRIDE_BYTES);
const _: () = assert!(ROW_WRITE_CHUNK_BYTES > 0);

/// Decoded source-digest facts that exist before institutional admission.
///
/// The fields are deliberately private and there is no public constructor.
/// Production values are derived only by [`produce_candidate_universe_v1`]
/// from the owning typed sources. A value decoded from disk remains a fact to
/// audit rather than an independently forgeable authoring capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateUniverseIdentitiesV1 {
    /// Composite stored-data identity used by the upstream run.
    data_digest: [u8; 32],
    /// Exact vendor/feed identity.
    feed_digest: [u8; 32],
    /// Clean source-commit identity.
    source_commit_digest: [u8; 32],
    /// Append-only vocabulary identity.
    vocabulary_digest: [u8; 32],
    /// Entry/evaluation policy identity.
    evaluation_policy_digest: [u8; 32],
    /// Exact long and short grid policy/resolution identities.
    exit_grids: LongShortExitGridIdentitiesV2,
    /// IST calendar-policy identity.
    calendar_policy_digest: [u8; 32],
    /// Causal prior-day-reference policy identity.
    daily_reference_policy_digest: [u8; 32],
}

impl CandidateUniverseIdentitiesV1 {
    /// Composite stored-data digest copied from decoded bytes.
    #[must_use]
    pub const fn data_digest(self) -> [u8; 32] {
        self.data_digest
    }

    /// Vendor/feed digest copied from decoded bytes.
    #[must_use]
    pub const fn feed_digest(self) -> [u8; 32] {
        self.feed_digest
    }

    /// Source-commit digest copied from decoded bytes.
    #[must_use]
    pub const fn source_commit_digest(self) -> [u8; 32] {
        self.source_commit_digest
    }

    /// Vocabulary digest copied from decoded bytes.
    #[must_use]
    pub const fn vocabulary_digest(self) -> [u8; 32] {
        self.vocabulary_digest
    }

    /// Evaluation-policy digest copied from decoded bytes.
    #[must_use]
    pub const fn evaluation_policy_digest(self) -> [u8; 32] {
        self.evaluation_policy_digest
    }

    /// Long/short grid identities copied from decoded bytes.
    #[must_use]
    pub const fn exit_grids(self) -> LongShortExitGridIdentitiesV2 {
        self.exit_grids
    }

    /// Calendar-policy digest copied from decoded bytes.
    #[must_use]
    pub const fn calendar_policy_digest(self) -> [u8; 32] {
        self.calendar_policy_digest
    }

    /// Prior-day-reference policy digest copied from decoded bytes.
    #[must_use]
    pub const fn daily_reference_policy_digest(self) -> [u8; 32] {
        self.daily_reference_policy_digest
    }

    fn validate(self) -> Result<(), CandidateUniverseRefusal> {
        for (name, digest) in [
            ("data_digest", self.data_digest),
            ("feed_digest", self.feed_digest),
            ("source_commit_digest", self.source_commit_digest),
            ("vocabulary_digest", self.vocabulary_digest),
            ("evaluation_policy_digest", self.evaluation_policy_digest),
            ("calendar_policy_digest", self.calendar_policy_digest),
            (
                "daily_reference_policy_digest",
                self.daily_reference_policy_digest,
            ),
        ] {
            require_nonzero_digest(name, digest)?;
        }
        self.exit_grids
            .composite_digest()
            .map(|_| ())
            .map_err(|why| format!("candidate exit-grid identity refused: {why}"))
    }
}

/// Exact signal stream whose bars produced the swept masks and support counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateSignalStreamV1 {
    count: u64,
    first_ts_micros: i64,
    last_ts_micros: i64,
    digest: [u8; 32],
}

impl CandidateSignalStreamV1 {
    fn from_bars(bars: &[Candle]) -> Result<Self, CandidateUniverseRefusal> {
        let facts = stream_facts("signal", bars)?;
        Ok(Self {
            count: facts.0,
            first_ts_micros: facts.1,
            last_ts_micros: facts.2,
            digest: facts.3,
        })
    }

    /// Exact number of signal candles offered to the indicator fold.
    #[must_use]
    pub const fn count(self) -> u64 {
        self.count
    }

    /// First signal-candle open timestamp, in UTC microseconds.
    #[must_use]
    pub const fn first_ts_micros(self) -> i64 {
        self.first_ts_micros
    }

    /// Last signal-candle open timestamp, in UTC microseconds.
    #[must_use]
    pub const fn last_ts_micros(self) -> i64 {
        self.last_ts_micros
    }

    /// Digest of all seven fields of every signal candle, in order.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.digest
    }

    fn validate(self) -> Result<(), CandidateUniverseRefusal> {
        validate_stream_facts(
            "signal",
            self.count,
            self.first_ts_micros,
            self.last_ts_micros,
            self.digest,
        )
    }
}

/// Exact requested-span one-minute stream evaluated by every candidate row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateExecutionStreamV1 {
    count: u64,
    first_ts_micros: i64,
    last_ts_micros: i64,
    digest: [u8; 32],
}

impl CandidateExecutionStreamV1 {
    fn from_series(series: ExecutionSeriesV1<'_>) -> Result<Self, CandidateUniverseRefusal> {
        let (count, first_ts_micros, last_ts_micros, digest) =
            stream_facts("execution", series.bars())?;
        Ok(Self {
            count,
            first_ts_micros,
            last_ts_micros,
            digest,
        })
    }

    /// Exact number of evaluated one-minute candles.
    #[must_use]
    pub const fn count(self) -> u64 {
        self.count
    }

    /// First evaluated one-minute open timestamp, in UTC microseconds.
    #[must_use]
    pub const fn first_ts_micros(self) -> i64 {
        self.first_ts_micros
    }

    /// Last evaluated one-minute open timestamp, in UTC microseconds.
    #[must_use]
    pub const fn last_ts_micros(self) -> i64 {
        self.last_ts_micros
    }

    /// Digest of all seven fields of every evaluated candle, in order.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.digest
    }

    fn validate(self) -> Result<(), CandidateUniverseRefusal> {
        validate_stream_facts(
            "execution",
            self.count,
            self.first_ts_micros,
            self.last_ts_micros,
            self.digest,
        )
    }
}

/// Exact complete-calendar evidence carried before population finalization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateCalendarCoverageV1 {
    signal_rung_seconds: u32,
    execution_rung_seconds: u32,
    first_day: i64,
    last_day: i64,
    signal_complete_receipt_digest: [u8; 32],
    execution_complete_receipt_digest: [u8; 32],
}

impl CandidateCalendarCoverageV1 {
    pub(crate) fn from_complete(
        rung_seconds: u32,
        requested_span: RequestedSpanIdentityV1,
        signal: CompleteCalendarReceiptV2,
        execution: CompleteCalendarReceiptV2,
    ) -> Result<Self, CandidateUniverseRefusal> {
        if signal.rung_seconds() != rung_seconds {
            return Err(format!(
                "candidate signal calendar proves {} seconds, not requested rung {rung_seconds}",
                signal.rung_seconds()
            ));
        }
        if execution.rung_seconds() != 60 {
            return Err(format!(
                "candidate execution calendar proves {} seconds; exact one-minute execution requires 60",
                execution.rung_seconds()
            ));
        }
        if signal.first_day() != execution.first_day() || signal.last_day() != execution.last_day()
        {
            return Err(format!(
                "candidate signal calendar covers {}..={}, but execution calendar covers {}..={}",
                signal.first_day(),
                signal.last_day(),
                execution.first_day(),
                execution.last_day()
            ));
        }
        let (expected_first, expected_last) = requested_span_days(requested_span)?;
        if signal.first_day() != expected_first || signal.last_day() != expected_last {
            return Err(format!(
                "candidate complete calendars cover IST days {}..={}, not the full requested month span {expected_first}..={expected_last}",
                signal.first_day(),
                signal.last_day()
            ));
        }
        let coverage = Self {
            signal_rung_seconds: rung_seconds,
            execution_rung_seconds: 60,
            first_day: expected_first,
            last_day: expected_last,
            signal_complete_receipt_digest: signal.digest(),
            execution_complete_receipt_digest: execution.digest(),
        };
        coverage.validate()?;
        Ok(coverage)
    }

    /// Signal rung proved complete.
    #[must_use]
    pub const fn signal_rung_seconds(self) -> u32 {
        self.signal_rung_seconds
    }

    /// Execution rung proved complete, always sixty seconds.
    #[must_use]
    pub const fn execution_rung_seconds(self) -> u32 {
        self.execution_rung_seconds
    }

    /// First requested IST civil day, inclusive.
    #[must_use]
    pub const fn first_day(self) -> i64 {
        self.first_day
    }

    /// Last requested IST civil day, inclusive.
    #[must_use]
    pub const fn last_day(self) -> i64 {
        self.last_day
    }

    /// Digest of the complete signal-rung calendar receipt.
    #[must_use]
    pub const fn signal_receipt_digest(self) -> [u8; 32] {
        self.signal_complete_receipt_digest
    }

    /// Digest of the complete one-minute calendar receipt.
    #[must_use]
    pub const fn execution_receipt_digest(self) -> [u8; 32] {
        self.execution_complete_receipt_digest
    }

    fn validate(self) -> Result<(), CandidateUniverseRefusal> {
        require_rung(self.signal_rung_seconds)?;
        if self.execution_rung_seconds != 60 {
            return Err(format!(
                "candidate calendar execution rung {} is not sixty seconds",
                self.execution_rung_seconds
            ));
        }
        if self.first_day > self.last_day {
            return Err(format!(
                "candidate calendar coverage runs backward: {}..={} ",
                self.first_day, self.last_day
            ));
        }
        require_nonzero_digest(
            "candidate signal calendar receipt",
            self.signal_complete_receipt_digest,
        )?;
        require_nonzero_digest(
            "candidate execution calendar receipt",
            self.execution_complete_receipt_digest,
        )
    }

    fn encode(self) -> [u8; CALENDAR_COVERAGE_BYTES] {
        let mut raw = [0_u8; CALENDAR_COVERAGE_BYTES];
        put_u32(&mut raw, 0, 1);
        put_u32(&mut raw, 4, self.signal_rung_seconds);
        put_u32(&mut raw, 8, self.execution_rung_seconds);
        put_i64(&mut raw, 16, self.first_day);
        put_i64(&mut raw, 24, self.last_day);
        put_bytes(&mut raw, 32, &self.signal_complete_receipt_digest);
        put_bytes(&mut raw, 64, &self.execution_complete_receipt_digest);
        raw
    }

    fn decode(raw: &[u8]) -> Result<Self, CandidateUniverseRefusal> {
        if raw.len() != CALENDAR_COVERAGE_BYTES {
            return Err(format!(
                "candidate calendar coverage is {} bytes, not {CALENDAR_COVERAGE_BYTES}",
                raw.len()
            ));
        }
        let version = get_u32(raw, 0)?;
        if version != 1 {
            return Err(format!(
                "candidate calendar coverage version {version} is unknown; expected 1"
            ));
        }
        require_zero(raw, 12, 4, "candidate calendar reserve")?;
        let coverage = Self {
            signal_rung_seconds: get_u32(raw, 4)?,
            execution_rung_seconds: get_u32(raw, 8)?,
            first_day: get_i64(raw, 16)?,
            last_day: get_i64(raw, 24)?,
            signal_complete_receipt_digest: get_32(raw, 32)?,
            execution_complete_receipt_digest: get_32(raw, 64)?,
        };
        coverage.validate()?;
        Ok(coverage)
    }
}

/// Immutable identity and evidence shared by one candidate-universe block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateUniverseDescriptorV1 {
    family: InstrumentFamilyV1,
    rung_seconds: u32,
    horizon_bars: u32,
    requested_span: RequestedSpanIdentityV1,
    identities: CandidateUniverseIdentitiesV1,
    calendar_coverage: CandidateCalendarCoverageV1,
    signal_stream: CandidateSignalStreamV1,
    execution_stream: CandidateExecutionStreamV1,
    signal_column_digest: [u8; 32],
    execution_column_digest: [u8; 32],
    universe_id: [u8; 32],
}

impl CandidateUniverseDescriptorV1 {
    /// Internal join used only after the production source derived every
    /// identity from its owning typed value.
    ///
    /// It remains private so arbitrary digest arrays can never be presented as
    /// source authority by another module.
    ///
    /// # Errors
    ///
    /// Refuses any absent identity, unsupported rung/family, partial calendar
    /// span, non-one-minute execution receipt, mismatched series identity, or
    /// empty execution stream.
    #[expect(
        clippy::too_many_arguments,
        reason = "every pre-admission source identity term is explicit and has no default"
    )]
    fn new(
        family: InstrumentFamilyV1,
        rung_seconds: u32,
        horizon: Horizon,
        requested_span: RequestedSpanIdentityV1,
        identities: &CandidateUniverseIdentitiesV1,
        signal_calendar: CompleteCalendarReceiptV2,
        execution_calendar: CompleteCalendarReceiptV2,
        signal_bars: &[Candle],
        signal_column: &Column,
        execution_series: ExecutionSeriesV1<'_>,
        execution_column: &Column,
    ) -> Result<Self, CandidateUniverseRefusal> {
        require_rung(rung_seconds)?;
        identities.validate()?;
        if identities.calendar_policy_digest != crate::stored::calendar_policy_digest_v2() {
            return Err(
                "candidate calendar-policy identity is not the canonical V2 measured-calendar policy"
                    .to_owned(),
            );
        }
        require_series_family(family, execution_series)?;
        if hash(execution_series.feed().as_bytes()) != identities.feed_digest {
            return Err(
                "candidate execution series feed does not match receipt feed identity".to_owned(),
            );
        }
        if hash(execution_series.commit().as_bytes()) != identities.source_commit_digest {
            return Err(
                "candidate execution series commit does not match receipt source commit identity"
                    .to_owned(),
            );
        }
        if execution_series.calendar_digest() != identities.calendar_policy_digest {
            return Err(
                "candidate execution series calendar identity does not match receipt policy"
                    .to_owned(),
            );
        }
        let calendar_coverage = CandidateCalendarCoverageV1::from_complete(
            rung_seconds,
            requested_span,
            signal_calendar,
            execution_calendar,
        )?;
        let recomputed_signal = crate::stored::calendar_receipt_v2_for_bars(
            signal_bars,
            rung_seconds,
            calendar_coverage.first_day,
            calendar_coverage.last_day,
        )
        .and_then(crate::stored::CalendarReceiptV2::require_complete)
        .map_err(|why| format!("candidate signal stream calendar refused: {why}"))?;
        if recomputed_signal != signal_calendar {
            return Err(
                "candidate signal complete-calendar receipt is not the receipt of the exact signal stream"
                    .to_owned(),
            );
        }
        let recomputed_execution = crate::stored::calendar_receipt_v2_for_bars(
            execution_series.bars(),
            60,
            calendar_coverage.first_day,
            calendar_coverage.last_day,
        )
        .and_then(crate::stored::CalendarReceiptV2::require_complete)
        .map_err(|why| format!("candidate execution stream calendar refused: {why}"))?;
        if recomputed_execution != execution_calendar {
            return Err(
                "candidate execution complete-calendar receipt is not the receipt of the exact requested-span one-minute stream"
                    .to_owned(),
            );
        }
        require_column_sources("signal", signal_column, signal_bars.len())?;
        require_column_sources("execution", execution_column, execution_series.bars().len())?;
        let signal_stream = CandidateSignalStreamV1::from_bars(signal_bars)?;
        let execution_stream = CandidateExecutionStreamV1::from_series(execution_series)?;
        let signal_column_digest = column_digest_v1(signal_column);
        let execution_column_digest = column_digest_v1(execution_column);
        require_nonzero_digest("candidate signal column", signal_column_digest)?;
        require_nonzero_digest("candidate execution column", execution_column_digest)?;
        let mut descriptor = Self {
            family,
            rung_seconds,
            horizon_bars: horizon.as_bars(),
            requested_span,
            identities: *identities,
            calendar_coverage,
            signal_stream,
            execution_stream,
            signal_column_digest,
            execution_column_digest,
            universe_id: [0; 32],
        };
        descriptor.universe_id = derive_universe_id(&descriptor);
        descriptor.validate()?;
        Ok(descriptor)
    }

    /// Swept index family.
    #[must_use]
    pub const fn family(self) -> InstrumentFamilyV1 {
        self.family
    }

    /// Signal timeframe in seconds.
    #[must_use]
    pub const fn rung_seconds(self) -> u32 {
        self.rung_seconds
    }

    /// Exact one-minute exit horizon, in bars.
    #[must_use]
    pub const fn horizon_bars(self) -> u32 {
        self.horizon_bars
    }

    /// Inclusive requested month span.
    #[must_use]
    pub const fn requested_span(self) -> RequestedSpanIdentityV1 {
        self.requested_span
    }

    /// Pre-admission source identities.
    #[must_use]
    pub const fn identities(self) -> CandidateUniverseIdentitiesV1 {
        self.identities
    }

    /// Exact complete signal and execution calendar evidence.
    #[must_use]
    pub const fn calendar_coverage(self) -> CandidateCalendarCoverageV1 {
        self.calendar_coverage
    }

    /// Exact signal bars whose indicator fold produced support and masks.
    #[must_use]
    pub const fn signal_stream(self) -> CandidateSignalStreamV1 {
        self.signal_stream
    }

    /// Exact requested-span one-minute stream identity.
    #[must_use]
    pub const fn execution_stream(self) -> CandidateExecutionStreamV1 {
        self.execution_stream
    }

    /// Digest of the exact signal column swept for support.
    #[must_use]
    pub const fn signal_column_digest(self) -> [u8; 32] {
        self.signal_column_digest
    }

    /// Digest of the exact signal-to-one-minute column used by grid evaluation.
    #[must_use]
    pub const fn execution_column_digest(self) -> [u8; 32] {
        self.execution_column_digest
    }

    /// Domain-separated identity of the entire pre-admission universe source.
    #[must_use]
    pub const fn universe_id(self) -> [u8; 32] {
        self.universe_id
    }

    fn validate(self) -> Result<(), CandidateUniverseRefusal> {
        require_rung(self.rung_seconds)?;
        if self.horizon_bars == 0 {
            return Err("candidate horizon is zero".to_owned());
        }
        self.identities.validate()?;
        if self.identities.calendar_policy_digest != crate::stored::calendar_policy_digest_v2() {
            return Err(
                "candidate calendar-policy identity is not the canonical V2 measured-calendar policy"
                    .to_owned(),
            );
        }
        self.calendar_coverage.validate()?;
        let (expected_first, expected_last) = requested_span_days(self.requested_span)?;
        if self.calendar_coverage.first_day != expected_first
            || self.calendar_coverage.last_day != expected_last
        {
            return Err(format!(
                "candidate calendar coverage {}..={} does not equal requested month span {expected_first}..={expected_last}",
                self.calendar_coverage.first_day, self.calendar_coverage.last_day
            ));
        }
        self.signal_stream.validate()?;
        self.execution_stream.validate()?;
        require_nonzero_digest("candidate signal column", self.signal_column_digest)?;
        require_nonzero_digest("candidate execution column", self.execution_column_digest)?;
        if self.calendar_coverage.signal_rung_seconds != self.rung_seconds {
            return Err("candidate descriptor/calendar signal rung mismatch".to_owned());
        }
        let expected = derive_universe_id(&self);
        if self.universe_id != expected {
            return Err("candidate universe identity does not match its fields".to_owned());
        }
        Ok(())
    }
}

/// Opaque, fully-derived crate-internal source for one candidate universe.
///
/// The constructor accepts typed market-data values and policies, never their
/// digests. It builds the anchored signal column itself, validates the exact
/// daily-record/eligibility pairing, overlays `GapFib` from the complete
/// one-minute context, and derives the checked execution column. The type and
/// all fields remain crate-private because those typed inputs do not yet carry
/// a concrete store-origin capability. Another crate cannot use self-consistent
/// caller-held slices as production authority.
#[derive(Debug)]
pub(crate) struct CandidateUniverseProductionSourceV1<'a> {
    family: InstrumentFamilyV1,
    rung_seconds: u32,
    horizon: Horizon,
    requested_span: RequestedSpanIdentityV1,
    signal_calendar: CompleteCalendarReceiptV2,
    execution_calendar: CompleteCalendarReceiptV2,
    signal_bars: &'a [Candle],
    daily_references: &'a [DailyReference],
    reference_minute_context: &'a [Candle],
    daily_reference: DailyReferenceBinding<'a>,
    execution_series: ExecutionSeriesV1<'a>,
    long_exit_grid: &'a ResolvedExitGridV1,
    short_exit_grid: &'a ResolvedExitGridV1,
    signal_load_bound: StoredSpanLoadBoundV1,
    minute_load_bound: StoredSpanLoadBoundV1,
    daily_load_bound: StoredSpanLoadBoundV1,
    search_splits: usize,
    evaluation: CandidateEvaluationInputsV1,
    signal_column: Column,
    execution_column: Column,
    data_digest: [u8; 32],
    signal_stream: CandidateSignalStreamV1,
    execution_stream: CandidateExecutionStreamV1,
    signal_column_digest: [u8; 32],
    execution_column_digest: [u8; 32],
    source_id: [u8; 32],
}

impl<'a> CandidateUniverseProductionSourceV1<'a> {
    /// Builds the exact anchored signal and one-minute execution capabilities.
    ///
    /// `daily_references` must be the typed, record-for-record interpretation of
    /// `daily_reference.daily_bars` and `daily_reference.eligibility`.
    /// `reference_minute_context` is the complete exact-minute stream read by
    /// the causal overlay; `execution_series.bars()` must be one exact
    /// contiguous subspan of it. The supplied resolutions must be the complete
    /// long and short resolutions of that exact execution series.
    ///
    /// # Errors
    ///
    /// Refuses a non-canonical policy, foreign daily record, incomplete
    /// calendar, malformed context, foreign execution subspan, missing daily
    /// anchor, exact-minute overlay failure, foreign grid/side/feed/commit or
    /// any source/column identity mismatch.
    #[expect(
        clippy::too_many_arguments,
        reason = "each argument is a distinct typed source and no identity term has a default"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "construction validates one indivisible typed source identity before the capability can exist"
    )]
    pub(crate) fn new(
        family: InstrumentFamilyV1,
        rung_seconds: u32,
        horizon: Horizon,
        requested_span: RequestedSpanIdentityV1,
        signal_calendar: CompleteCalendarReceiptV2,
        execution_calendar: CompleteCalendarReceiptV2,
        signal_bars: &'a [Candle],
        daily_references: &'a [DailyReference],
        reference_minute_context: &'a [Candle],
        daily_reference: DailyReferenceBinding<'a>,
        execution_series: ExecutionSeriesV1<'a>,
        widths: Widths,
        availability: Availability,
        thresholds: Thresholds,
        long_exit_grid: &'a ResolvedExitGridV1,
        short_exit_grid: &'a ResolvedExitGridV1,
        signal_load_bound: StoredSpanLoadBoundV1,
        minute_load_bound: StoredSpanLoadBoundV1,
        daily_load_bound: StoredSpanLoadBoundV1,
    ) -> Result<Self, CandidateUniverseRefusal> {
        require_rung(rung_seconds)?;
        require_canonical_daily_reference(daily_references, daily_reference)?;
        require_exact_execution_subspan(reference_minute_context, execution_series.bars())?;
        require_series_family(family, execution_series)?;
        if execution_series.calendar_digest() != crate::stored::calendar_policy_digest_v2() {
            return Err(
                "candidate production execution series does not carry the canonical V2 calendar policy"
                    .to_owned(),
            );
        }
        validate_candidate_exit_sources(family, execution_series, long_exit_grid, short_exit_grid)?;
        require_load_bound("signal", signal_bars.len(), signal_load_bound)?;
        require_load_bound(
            "complete minute context",
            reference_minute_context.len(),
            minute_load_bound,
        )?;
        require_load_bound(
            "daily reference",
            daily_reference.daily_bars.len(),
            daily_load_bound,
        )?;

        let coverage = CandidateCalendarCoverageV1::from_complete(
            rung_seconds,
            requested_span,
            signal_calendar,
            execution_calendar,
        )?;
        require_exact_calendar(
            "signal",
            signal_bars,
            rung_seconds,
            coverage,
            signal_calendar,
        )?;
        require_exact_calendar(
            "execution",
            execution_series.bars(),
            60,
            coverage,
            execution_calendar,
        )?;

        let search_splits = crate::walk_forward_splits(signal_bars.len());
        if search_splits < 2 {
            return Err(
                "candidate Search V4 split policy produced fewer than two folds".to_owned(),
            );
        }
        let evaluation = CandidateEvaluationInputsV1 {
            widths,
            availability,
            thresholds,
        };
        let (signal_column, execution_column) = build_candidate_columns(
            signal_bars,
            daily_references,
            reference_minute_context,
            execution_series.bars(),
            rung_seconds,
            &evaluation,
        )?;

        let data_digest = runner::identity::data_digest_with_daily_reference(
            signal_bars,
            reference_minute_context,
            daily_reference,
        )
        .map_err(|why| format!("candidate daily data identity refused: {why:?}"))?;
        let signal_stream = CandidateSignalStreamV1::from_bars(signal_bars)?;
        let execution_stream = CandidateExecutionStreamV1::from_series(execution_series)?;
        let signal_column_digest = column_digest_v1(&signal_column);
        let execution_column_digest = column_digest_v1(&execution_column);
        let mut source = Self {
            family,
            rung_seconds,
            horizon,
            requested_span,
            signal_calendar,
            execution_calendar,
            signal_bars,
            daily_references,
            reference_minute_context,
            daily_reference,
            execution_series,
            long_exit_grid,
            short_exit_grid,
            signal_load_bound,
            minute_load_bound,
            daily_load_bound,
            search_splits,
            evaluation,
            signal_column,
            execution_column,
            data_digest,
            signal_stream,
            execution_stream,
            signal_column_digest,
            execution_column_digest,
            source_id: [0; 32],
        };
        source.source_id = derive_production_source_id(&source);
        source.validate()?;
        Ok(source)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "validation compares every retained Candidate and Search V4 source term in one fail-closed boundary"
    )]
    fn validate(&self) -> Result<(), CandidateUniverseRefusal> {
        require_rung(self.rung_seconds)?;
        if self.horizon.as_bars() == 0 {
            return Err("candidate production horizon is zero".to_owned());
        }
        require_series_family(self.family, self.execution_series)?;
        if self.execution_series.calendar_digest() != crate::stored::calendar_policy_digest_v2() {
            return Err("candidate production execution calendar policy changed".to_owned());
        }
        require_canonical_daily_reference(self.daily_references, self.daily_reference)?;
        require_exact_execution_subspan(
            self.reference_minute_context,
            self.execution_series.bars(),
        )?;
        validate_resolved_source(
            "long",
            self.family,
            Side::Long,
            self.execution_series,
            self.long_exit_grid,
        )?;
        validate_resolved_source(
            "short",
            self.family,
            Side::Short,
            self.execution_series,
            self.short_exit_grid,
        )?;
        if self.long_exit_grid.policy_digest() == self.short_exit_grid.policy_digest()
            || self.long_exit_grid.digest() == self.short_exit_grid.digest()
        {
            return Err(
                "candidate production long and short grid identities became indistinguishable"
                    .to_owned(),
            );
        }
        require_load_bound("signal", self.signal_bars.len(), self.signal_load_bound)?;
        require_load_bound(
            "complete minute context",
            self.reference_minute_context.len(),
            self.minute_load_bound,
        )?;
        require_load_bound(
            "daily reference",
            self.daily_reference.daily_bars.len(),
            self.daily_load_bound,
        )?;
        if self.search_splits != crate::walk_forward_splits(self.signal_bars.len())
            || self.search_splits < 2
        {
            return Err("candidate retained Search V4 split policy changed".to_owned());
        }
        let coverage = CandidateCalendarCoverageV1::from_complete(
            self.rung_seconds,
            self.requested_span,
            self.signal_calendar,
            self.execution_calendar,
        )?;
        require_exact_calendar(
            "signal",
            self.signal_bars,
            self.rung_seconds,
            coverage,
            self.signal_calendar,
        )?;
        require_exact_calendar(
            "execution",
            self.execution_series.bars(),
            60,
            coverage,
            self.execution_calendar,
        )?;
        require_column_sources("signal", &self.signal_column, self.signal_bars.len())?;
        require_column_sources(
            "execution",
            &self.execution_column,
            self.execution_series.bars().len(),
        )?;
        require_same_evaluation_spec(&self.signal_column, &self.execution_column)?;
        if self.signal_stream != CandidateSignalStreamV1::from_bars(self.signal_bars)?
            || self.execution_stream
                != CandidateExecutionStreamV1::from_series(self.execution_series)?
        {
            return Err("candidate production exact stream identity changed".to_owned());
        }
        if self.signal_column_digest != column_digest_v1(&self.signal_column)
            || self.execution_column_digest != column_digest_v1(&self.execution_column)
        {
            return Err("candidate production exact column identity changed".to_owned());
        }
        let data_digest = runner::identity::data_digest_with_daily_reference(
            self.signal_bars,
            self.reference_minute_context,
            self.daily_reference,
        )
        .map_err(|why| format!("candidate daily data identity refused: {why:?}"))?;
        if self.data_digest != data_digest {
            return Err(
                "candidate production signal/minute/daily data identity changed".to_owned(),
            );
        }
        require_nonzero_digest("candidate production source", self.source_id)?;
        if self.source_id != derive_production_source_id(self) {
            return Err("candidate production source seal does not match its fields".to_owned());
        }
        Ok(())
    }

    /// Runs Search V4 from this exact retained Candidate source.
    ///
    /// The complete prepared signal column and both resolved side grids are
    /// borrowed directly from the same opaque source that later produces the
    /// Candidate ledger. Every fold rebuilds the anchored evaluator through
    /// [`build_candidate_signal_column`] and receives only exact-minute context
    /// ending at that causal signal prefix. The resulting Runner projection is
    /// joined back to all five signal-source terms and both side identities
    /// before the opaque validation can escape this method.
    pub(crate) fn anchored_search_v4(
        &self,
        sweeper: &Sweeper,
    ) -> Result<AnchoredSearchValidationV4, CandidateUniverseRefusal> {
        self.validate()?;
        let signal_length_micros = signal_length_micros(self.rung_seconds)?;
        let mut builder = CandidateSearchColumnBuilderV1 {
            full_signal: self.signal_bars,
            daily_references: self.daily_references,
            reference_minute_context: self.reference_minute_context,
            rung_seconds: self.rung_seconds,
            evaluation: self.evaluation,
            cursors: [CandidateCausalPrefixCursorV1::default(); 2],
        };
        let validation = walk_forward_projected_prepared_anchored_search_v4(
            self.signal_bars,
            &self.signal_column,
            self.execution_series,
            signal_length_micros,
            self.horizon,
            self.search_splits,
            sweeper,
            &mut |prefix| builder.build(prefix),
            self.long_exit_grid,
            self.short_exit_grid,
        )
        .map_err(|why| format!("candidate retained Search V4 refused: {why:?}"))?;
        let authority = validation.search_authority_projection().map_err(|why| {
            format!("candidate retained Search V4 reconciliation refused: {why:?}")
        })?;
        let search_source = authority.source_identity();
        if search_source.signal_digest() != self.signal_stream.digest()
            || search_source.signal_bars() != self.signal_stream.count()
            || search_source.signal_first_ts_micros() != self.signal_stream.first_ts_micros()
            || search_source.signal_last_ts_micros() != self.signal_stream.last_ts_micros()
            || search_source.signal_column_digest() != self.signal_column_digest
        {
            return Err(
                "candidate retained Search V4 belongs to a foreign signal stream or prepared column"
                    .to_owned(),
            );
        }
        let search_long = authority.long_grid_identity();
        let search_short = authority.short_grid_identity();
        if search_long.policy_digest() != self.long_exit_grid.policy_digest()
            || search_long.resolution_digest() != self.long_exit_grid.digest()
            || search_short.policy_digest() != self.short_exit_grid.policy_digest()
            || search_short.resolution_digest() != self.short_exit_grid.digest()
        {
            return Err(
                "candidate retained Search V4 belongs to foreign Long/Short grid identities"
                    .to_owned(),
            );
        }
        Ok(validation)
    }

    /// Canonical dynamic split count frozen with this exact signal source.
    #[must_use]
    pub(crate) const fn search_splits(&self) -> usize {
        self.search_splits
    }

    /// Rebuilds and classifies the complete stored Candidate block without
    /// exposing its bars, columns, grids, masks or detached identities.
    ///
    /// The caller must hold authenticated Candidate rows and their exact
    /// Completion. This method reproduces the original descriptor from this
    /// typed source and retained ladder, verifies the literal ordered block,
    /// evaluates each contiguous `(mask, direction)` grid once, and returns one
    /// Runner terminal disposition per row. No receipt field is accepted as a
    /// substitute for recomputation.
    #[expect(
        clippy::large_types_passed_by_value,
        reason = "the authenticated Candidate receipt is a one-use capability and must cross this replay boundary by value"
    )]
    pub(crate) fn execution_v3_replay_authority(
        &self,
        ladder: engine::Ladder,
        receipt: CandidateUniverseReceiptV1,
        rows: &[AuthenticatedCandidatePopulationRowV1],
    ) -> Result<CandidateExecutionReplayAuthorityV1, CandidateUniverseRefusal> {
        build_execution_v3_replay_authority(self, ladder, receipt, rows)
    }
}

/// Exact post-training stored cohort used only to mint Global Replay
/// witnesses from already-authorized Execution V3 dispositions.
///
/// The constructor receives the same typed stored signal/daily/minute inputs
/// as Candidate production, builds the anchored and checked one-minute columns
/// itself, and binds their complete calendars and load ceilings.  It accepts no
/// feed, instrument, direction, mask, selected exit, timestamp boundary or
/// digest as a loose scalar.  Those facts remain inside the stored series and
/// the retained training capabilities supplied to [`Self::mint_witness`].
pub(crate) struct CandidateGlobalReplayOosSourceV1<'a> {
    family: InstrumentFamilyV1,
    rung_seconds: u32,
    horizon: Horizon,
    requested_span: RequestedSpanIdentityV1,
    signal_calendar: CompleteCalendarReceiptV2,
    execution_calendar: CompleteCalendarReceiptV2,
    signal_bars: &'a [Candle],
    reference_minute_context: &'a [Candle],
    daily_reference: DailyReferenceBinding<'a>,
    execution_series: ExecutionSeriesV1<'a>,
    execution_column: Column,
    data_digest: [u8; 32],
    signal_load_bound: StoredSpanLoadBoundV1,
    minute_load_bound: StoredSpanLoadBoundV1,
    daily_load_bound: StoredSpanLoadBoundV1,
    source_id: [u8; 32],
}

impl<'a> CandidateGlobalReplayOosSourceV1<'a> {
    #[expect(
        clippy::too_many_arguments,
        reason = "the stored OOS capability binds each typed stream, calendar and explicit load ceiling without a detached identity shortcut"
    )]
    pub(crate) fn new(
        family: InstrumentFamilyV1,
        rung_seconds: u32,
        horizon: Horizon,
        requested_span: RequestedSpanIdentityV1,
        signal_calendar: CompleteCalendarReceiptV2,
        execution_calendar: CompleteCalendarReceiptV2,
        signal_bars: &'a [Candle],
        daily_references: &'a [DailyReference],
        reference_minute_context: &'a [Candle],
        daily_reference: DailyReferenceBinding<'a>,
        execution_series: ExecutionSeriesV1<'a>,
        widths: Widths,
        availability: Availability,
        thresholds: Thresholds,
        signal_load_bound: StoredSpanLoadBoundV1,
        minute_load_bound: StoredSpanLoadBoundV1,
        daily_load_bound: StoredSpanLoadBoundV1,
    ) -> Result<Self, CandidateUniverseRefusal> {
        require_rung(rung_seconds)?;
        require_series_family(family, execution_series)?;
        if execution_series.calendar_digest() != crate::stored::calendar_policy_digest_v2() {
            return Err(
                "Global Replay OOS execution series does not carry the canonical V2 calendar policy"
                    .to_owned(),
            );
        }
        require_canonical_daily_reference(daily_references, daily_reference)?;
        require_exact_execution_subspan(reference_minute_context, execution_series.bars())?;
        require_load_bound(
            "Global Replay OOS signal",
            signal_bars.len(),
            signal_load_bound,
        )?;
        require_load_bound(
            "Global Replay OOS complete minute context",
            reference_minute_context.len(),
            minute_load_bound,
        )?;
        require_load_bound(
            "Global Replay OOS daily reference",
            daily_reference.daily_bars.len(),
            daily_load_bound,
        )?;
        let coverage = CandidateCalendarCoverageV1::from_complete(
            rung_seconds,
            requested_span,
            signal_calendar,
            execution_calendar,
        )?;
        require_exact_calendar(
            "Global Replay OOS signal",
            signal_bars,
            rung_seconds,
            coverage,
            signal_calendar,
        )?;
        require_exact_calendar(
            "Global Replay OOS execution",
            execution_series.bars(),
            60,
            coverage,
            execution_calendar,
        )?;
        let evaluation = CandidateEvaluationInputsV1 {
            widths,
            availability,
            thresholds,
        };
        let (_, execution_column) = build_candidate_columns(
            signal_bars,
            daily_references,
            reference_minute_context,
            execution_series.bars(),
            rung_seconds,
            &evaluation,
        )?;
        let data_digest = runner::identity::data_digest_with_daily_reference(
            signal_bars,
            reference_minute_context,
            daily_reference,
        )
        .map_err(|why| format!("Global Replay OOS daily data identity refused: {why:?}"))?;
        let mut source = Self {
            family,
            rung_seconds,
            horizon,
            requested_span,
            signal_calendar,
            execution_calendar,
            signal_bars,
            reference_minute_context,
            daily_reference,
            execution_series,
            execution_column,
            data_digest,
            signal_load_bound,
            minute_load_bound,
            daily_load_bound,
            source_id: [0; 32],
        };
        source.source_id = derive_global_replay_oos_source_id(&source);
        source.require_integrity()?;
        Ok(source)
    }

    fn require_integrity(&self) -> Result<(), CandidateUniverseRefusal> {
        let first = self
            .execution_series
            .bars()
            .first()
            .ok_or_else(|| "Global Replay OOS execution cohort is empty".to_owned())?;
        let last = self
            .execution_series
            .bars()
            .last()
            .ok_or_else(|| "Global Replay OOS execution cohort lost its last bar".to_owned())?;
        if first.ts_micros > last.ts_micros {
            return Err("Global Replay OOS execution timestamps are reversed".to_owned());
        }
        require_column_sources(
            "Global Replay OOS execution",
            &self.execution_column,
            self.execution_series.bars().len(),
        )?;
        if self.data_digest
            != runner::identity::data_digest_with_daily_reference(
                self.signal_bars,
                self.reference_minute_context,
                self.daily_reference,
            )
            .map_err(|why| format!("Global Replay OOS data identity refused: {why:?}"))?
            || self.source_id != derive_global_replay_oos_source_id(self)
        {
            return Err("Global Replay OOS source identity changed after construction".to_owned());
        }
        Ok(())
    }

    /// Replays one exact authorized training choice over this later cohort.
    ///
    /// `resolved` and `disposition` are opaque Runner capabilities rebuilt from
    /// the same retained stored transaction.  The method rejects an OOS first
    /// bar at or before the resolved training last bar and lets Runner recheck
    /// instrument, feed, commit, calendar, side, mask, policy and selected-exit
    /// identity before any witness can escape.
    pub(crate) fn mint_witness(
        &self,
        ladder: engine::Ladder,
        resolved: &ResolvedExitGridV1,
        disposition: &ExecutionDispositionV1,
    ) -> Result<GlobalReplayWitnessUniverseV1, CandidateUniverseRefusal> {
        self.mint_witness_recorded(ladder, resolved, disposition, &mut |_| Ok(()))
    }

    /// Call the fallible identity publisher immediately before trade replay.
    /// The existing mint entry point preserves its original caller contract.
    pub(crate) fn mint_witness_recorded(
        &self,
        ladder: engine::Ladder,
        resolved: &ResolvedExitGridV1,
        disposition: &ExecutionDispositionV1,
        before_replay: &mut dyn FnMut([u8; 32]) -> Result<(), String>,
    ) -> Result<GlobalReplayWitnessUniverseV1, CandidateUniverseRefusal> {
        self.require_integrity()?;
        let selected = disposition.selected().ok_or_else(|| {
            "Global Replay OOS winner has no authorized selected-exit capability".to_owned()
        })?;
        if disposition.resolution_digest() != resolved.digest()
            || disposition.horizon() != self.horizon
            || disposition.side() != resolved.side()
            || disposition.evaluation_spec_fingerprint()
                != self
                    .execution_column
                    .evaluation_spec_token()
                    .ok_or_else(|| {
                        "Global Replay OOS execution column lost its evaluation policy".to_owned()
                    })?
                    .fingerprint_v1()
                    .into_bytes()
        {
            return Err(
                "Global Replay OOS disposition belongs to a foreign resolution, horizon, side or evaluator"
                    .to_owned(),
            );
        }
        validate_global_replay_training_grid(self.family, self.execution_series, resolved)?;
        let direction = match resolved.side() {
            Side::Long => Direction::Long,
            Side::Short => Direction::Short,
        };
        let run = Run {
            mask: disposition.mask(),
            direction,
            instrument: self.execution_series.instrument(),
            timeframe: rung_label(self.rung_seconds)?,
            params: Params::of(ladder).with_policy(&[
                EXECUTION_RUN_POLICY_VERSION_V1,
                u64::from(self.rung_seconds),
                u64::from(self.horizon.as_bars()),
            ]),
            data_digest: self.data_digest,
            commit: self.execution_series.commit(),
            feed: self.execution_series.feed(),
        };
        let execution_run = ExecutionRunV1::new_with_daily_reference(
            &run,
            self.signal_bars,
            self.reference_minute_context,
            self.execution_series.bars(),
            self.daily_reference,
        )
        .map_err(|why| format!("Global Replay OOS exact run refused: {why:?}"))?;
        let oos = OosExecutionSeriesV1::new(self.execution_series, 0)
            .map_err(|why| format!("Global Replay OOS boundary refused: {why:?}"))?;
        before_replay(execution_run.run_id().bytes())?;
        resolved
            .replay_global_witness(oos, &self.execution_column, selected, execution_run)
            .map_err(|why| format!("Global Replay OOS witness refused: {why:?}"))
    }
}

fn validate_global_replay_training_grid(
    family: InstrumentFamilyV1,
    oos: ExecutionSeriesV1<'_>,
    resolved: &ResolvedExitGridV1,
) -> Result<(), CandidateUniverseRefusal> {
    let expected_family = match resolved.family() {
        runner::exit_grid_policy::InstrumentFamilyV1::Nifty => InstrumentFamilyV1::Nifty,
        runner::exit_grid_policy::InstrumentFamilyV1::BankNifty => InstrumentFamilyV1::BankNifty,
    };
    let first_oos = oos
        .bars()
        .first()
        .ok_or_else(|| "Global Replay OOS execution cohort is empty".to_owned())?
        .ts_micros;
    if !resolved.digest_is_valid()
        || expected_family != family
        || resolved.instrument() != *oos.instrument()
        || resolved.feed_digest() != hash(oos.feed().as_bytes())
        || resolved.commit_digest() != hash(oos.commit().as_bytes())
        || resolved.calendar_digest() != oos.calendar_digest()
        || resolved.cell_count() == 0
    {
        return Err(
            "Global Replay OOS cohort is crosswired to a foreign or torn training grid".to_owned(),
        );
    }
    if first_oos <= resolved.training_last_ts_micros() {
        return Err(format!(
            "Global Replay OOS first timestamp {first_oos} is not after training last {}",
            resolved.training_last_ts_micros()
        ));
    }
    Ok(())
}

fn derive_global_replay_oos_source_id(source: &CandidateGlobalReplayOosSourceV1<'_>) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(GLOBAL_REPLAY_OOS_SOURCE_ID_DOMAIN);
    hasher.update(&[family_byte(source.family)]);
    hash_u32(&mut hasher, source.rung_seconds);
    hash_u32(&mut hasher, source.horizon.as_bars());
    hasher.update(&source.requested_span.canonical_bytes());
    hasher.update(&source.signal_calendar.digest());
    hasher.update(&source.execution_calendar.digest());
    hasher.update(&source.data_digest);
    hasher.update(&column_digest_v1(&source.execution_column));
    if let Some(spec) = source.execution_column.evaluation_spec_token() {
        hasher.update(spec.fingerprint_v1().as_bytes());
    }
    hasher.update(&hash(source.execution_series.feed().as_bytes()));
    hasher.update(&hash(source.execution_series.commit().as_bytes()));
    hasher.update(&source.execution_series.calendar_digest());
    hash_u64(&mut hasher, source.signal_load_bound.max_records());
    hash_u64(&mut hasher, source.minute_load_bound.max_records());
    hash_u64(&mut hasher, source.daily_load_bound.max_records());
    if let Some(first) = source.execution_series.bars().first() {
        hasher.update(&first.ts_micros.to_le_bytes());
    }
    if let Some(last) = source.execution_series.bars().last() {
        hasher.update(&last.ts_micros.to_le_bytes());
    }
    hasher.finalize()
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CandidateEvaluationInputsV1 {
    pub(crate) widths: Widths,
    pub(crate) availability: Availability,
    pub(crate) thresholds: Thresholds,
}

/// Monotone causal adapter used only by retained Search V4.
///
/// Runner interleaves two nondecreasing prefix families: TRAIN prefixes and
/// TRAIN+OOS prefixes. This adapter proves every request starts at the retained
/// signal allocation and advances one of two fixed cursors through the daily
/// and exact-minute contexts. A fold therefore cannot see a minute after its
/// final signal close. Each retained auxiliary row is visited at most once per
/// cursor family, and the adapter allocates no prefix table.
struct CandidateSearchColumnBuilderV1<'a> {
    full_signal: &'a [Candle],
    daily_references: &'a [DailyReference],
    reference_minute_context: &'a [Candle],
    rung_seconds: u32,
    evaluation: CandidateEvaluationInputsV1,
    cursors: [CandidateCausalPrefixCursorV1; 2],
}

#[derive(Clone, Copy, Debug, Default)]
struct CandidateCausalPrefixCursorV1 {
    last_signal_len: usize,
    daily_end: usize,
    minute_end: usize,
}

impl CandidateSearchColumnBuilderV1<'_> {
    fn build(&mut self, prefix: &[Candle]) -> Result<Column, CandidateUniverseRefusal> {
        if prefix.is_empty()
            || prefix.len() > self.full_signal.len()
            || !core::ptr::eq(prefix.as_ptr(), self.full_signal.as_ptr())
        {
            return Err(
                "candidate Search V4 requested a non-causal or foreign signal prefix".to_owned(),
            );
        }
        let left = self.cursors[0].last_signal_len <= prefix.len();
        let right = self.cursors[1].last_signal_len <= prefix.len();
        let lane = match (left, right) {
            (true, false) => 0,
            (false, true) => 1,
            (true, true) => {
                usize::from(self.cursors[1].last_signal_len > self.cursors[0].last_signal_len)
            }
            (false, false) => {
                return Err(
                    "candidate Search V4 requested a prefix outside both monotone causal lanes"
                        .to_owned(),
                );
            }
        };
        let cursor = self
            .cursors
            .get_mut(lane)
            .ok_or_else(|| "candidate Search V4 causal lane is absent".to_owned())?;
        let signal_length_micros = signal_length_micros(self.rung_seconds)?;
        let final_signal_day = prefix
            .last()
            .map(|bar| indicators::ist_day(bar.ts_micros))
            .ok_or_else(|| "candidate Search V4 requested an empty signal prefix".to_owned())?;
        while self
            .daily_references
            .get(cursor.daily_end)
            .is_some_and(|reference| reference.ist_day() < final_signal_day)
        {
            cursor.daily_end = cursor
                .daily_end
                .checked_add(1)
                .ok_or_else(|| "candidate Search V4 daily-prefix width overflowed".to_owned())?;
        }
        let final_close_minute = prefix
            .last()
            .map(|bar| {
                bar.ts_micros
                    .saturating_add(signal_length_micros)
                    .saturating_sub(60_000_000)
            })
            .ok_or_else(|| "candidate Search V4 requested an empty signal prefix".to_owned())?;
        while self
            .reference_minute_context
            .get(cursor.minute_end)
            .is_some_and(|bar| bar.ts_micros <= final_close_minute)
        {
            cursor.minute_end = cursor
                .minute_end
                .checked_add(1)
                .ok_or_else(|| "candidate Search V4 minute-prefix width overflowed".to_owned())?;
        }
        let exact_close = cursor
            .minute_end
            .checked_sub(1)
            .and_then(|index| self.reference_minute_context.get(index))
            .is_some_and(|bar| bar.ts_micros == final_close_minute);
        if !exact_close {
            return Err(format!(
                "candidate Search V4 lacks exact closing minute {final_close_minute} for its signal prefix"
            ));
        }
        cursor.last_signal_len = prefix.len();
        build_candidate_signal_column(
            prefix,
            self.daily_references
                .get(..cursor.daily_end)
                .ok_or_else(|| {
                    "candidate Search V4 daily-prefix boundary is outside retained references"
                        .to_owned()
                })?,
            self.reference_minute_context
                .get(..cursor.minute_end)
                .ok_or_else(|| {
                    "candidate Search V4 minute-prefix boundary is outside retained context"
                        .to_owned()
                })?,
            self.rung_seconds,
            &self.evaluation,
        )
    }
}

pub(crate) fn build_candidate_columns(
    signal_bars: &[Candle],
    daily_references: &[DailyReference],
    reference_minute_context: &[Candle],
    execution_bars: &[Candle],
    rung_seconds: u32,
    evaluation: &CandidateEvaluationInputsV1,
) -> Result<(Column, Column), CandidateUniverseRefusal> {
    let signal_column = build_candidate_signal_column(
        signal_bars,
        daily_references,
        reference_minute_context,
        rung_seconds,
        evaluation,
    )?;
    let signal_length_micros = signal_length_micros(rung_seconds)?;
    let alignment = runner::align::onto_execution(
        signal_bars,
        signal_column.sources(),
        execution_bars,
        signal_length_micros,
    )
    .ok_or_else(|| {
        "candidate signal column could not be aligned to its exact one-minute execution series"
            .to_owned()
    })?;
    let (execution_column, _) = signal_column
        .reproject_checked(&alignment.onto, execution_bars)
        .ok_or_else(|| {
            "candidate checked execution projection is not parallel to the anchored signal column"
                .to_owned()
        })?;
    require_column_sources("signal", &signal_column, signal_bars.len())?;
    require_column_sources("execution", &execution_column, execution_bars.len())?;
    require_same_evaluation_spec(&signal_column, &execution_column)?;
    Ok((signal_column, execution_column))
}

/// Builds one exact anchored signal column from the typed daily reference and
/// exact-minute ORB/`GapFib` context accepted by Candidate production.
///
/// Keeping this as the single implementation is important for the retained
/// Search V4 successor: every causal prefix must use the same evaluator and
/// overlay semantics as the complete Candidate column, not a second closure
/// that merely happens to emit compatible masks.
fn build_candidate_signal_column(
    signal_bars: &[Candle],
    daily_references: &[DailyReference],
    reference_minute_context: &[Candle],
    rung_seconds: u32,
    evaluation: &CandidateEvaluationInputsV1,
) -> Result<Column, CandidateUniverseRefusal> {
    let mut evaluator = AnchoredEvaluator::new(
        evaluation.widths,
        evaluation.availability,
        evaluation.thresholds,
        daily_references,
    )
    .map_err(|why| format!("candidate daily reference series refused: {why:?}"))?;
    let anchored = AnchoredColumn::build_required(signal_bars, &mut evaluator).map_err(
        |missing| {
            format!(
                "candidate anchored signal column lacks a prior daily reference for {} bar(s) across {} IST day(s)",
                missing.signal_bars, missing.signal_days
            )
        },
    )?;
    if !anchored.reference_census().reconciles() {
        return Err(
            "candidate anchored signal daily-reference census does not reconcile".to_owned(),
        );
    }
    let mut signal_column = anchored.into_column();
    let signal_length_micros = signal_length_micros(rung_seconds)?;
    overlay_exact_minute_orb_and_gapfib(
        signal_bars,
        reference_minute_context,
        signal_length_micros,
        evaluation.widths,
        Calendar::charter(),
        &mut signal_column,
    )
    .map_err(|why| {
        format!(
            "candidate exact-minute ORB/GapFib overlay refused without a coarse fallback: {why:?}"
        )
    })?;
    require_column_sources("signal", &signal_column, signal_bars.len())?;
    Ok(signal_column)
}

/// Raw direct measurements copied exhaustively from one evaluated grid cell.
///
/// No derived assurance, admission, statistics or ranking value appears here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateDirectCellFactsV1 {
    trades: u64,
    wins: u64,
    pessimistic: i64,
    optimistic: i64,
    fill_cost: i64,
    stopped: u64,
    trailed_stop: u64,
    trailed_profit: u64,
    targeted: u64,
    timed_out: u64,
    ambiguous_bars: u64,
    gapped: u64,
    winner_mae: i64,
    winner_mfe: i64,
    all_mae: i64,
    worst_mae: i64,
    gross_win: i64,
    gross_loss: i64,
    best_trade: i64,
    min_win: i64,
    bars_held: u64,
    max_losing_streak: u32,
    max_winning_streak: u32,
    worst_trade: i64,
    max_drawdown: i64,
}

impl CandidateDirectCellFactsV1 {
    fn from_cell(cell: &Cell) -> Self {
        Self {
            trades: cell.trades,
            wins: cell.wins,
            pessimistic: cell.pessimistic,
            optimistic: cell.optimistic,
            fill_cost: cell.fill_cost,
            stopped: cell.stopped,
            trailed_stop: cell.trailed_stop,
            trailed_profit: cell.trailed_profit,
            targeted: cell.targeted,
            timed_out: cell.timed_out,
            ambiguous_bars: cell.ambiguous_bars,
            gapped: cell.gapped,
            winner_mae: cell.winner_mae,
            winner_mfe: cell.winner_mfe,
            all_mae: cell.all_mae,
            worst_mae: cell.worst_mae,
            gross_win: cell.gross_win,
            gross_loss: cell.gross_loss,
            best_trade: cell.best_trade,
            min_win: cell.min_win,
            bars_held: cell.bars_held,
            max_losing_streak: cell.max_losing_streak,
            max_winning_streak: cell.max_winning_streak,
            worst_trade: cell.worst_trade,
            max_drawdown: cell.max_drawdown,
        }
    }

    /// Reconstructs the exact source cell with the supplied exit coordinate.
    ///
    /// # Errors
    ///
    /// Refuses an index that does not fit this process's `usize`.
    pub fn to_cell(self, exit: ExitCoordinateV1) -> Result<Cell, CandidateUniverseRefusal> {
        let to_usize = |name: &str, value: Option<u32>| {
            value
                .map(|index| {
                    usize::try_from(index)
                        .map_err(|_| format!("candidate {name} index {index} does not fit usize"))
                })
                .transpose()
        };
        let ttp = exit
            .ttp
            .map(|(arm, trail)| -> Result<Ttp, CandidateUniverseRefusal> {
                Ok(Ttp {
                    arm: usize::try_from(arm)
                        .map_err(|_| format!("candidate TTP arm index {arm} does not fit usize"))?,
                    trail: usize::try_from(trail).map_err(|_| {
                        format!("candidate TTP trail index {trail} does not fit usize")
                    })?,
                })
            })
            .transpose()?;
        Ok(Cell {
            stop: to_usize("stop", exit.stop)?,
            target: to_usize("target", exit.target)?,
            tsl: to_usize("TSL", exit.tsl)?,
            ttp,
            trades: self.trades,
            wins: self.wins,
            pessimistic: self.pessimistic,
            optimistic: self.optimistic,
            fill_cost: self.fill_cost,
            stopped: self.stopped,
            trailed_stop: self.trailed_stop,
            trailed_profit: self.trailed_profit,
            targeted: self.targeted,
            timed_out: self.timed_out,
            ambiguous_bars: self.ambiguous_bars,
            gapped: self.gapped,
            winner_mae: self.winner_mae,
            winner_mfe: self.winner_mfe,
            all_mae: self.all_mae,
            worst_mae: self.worst_mae,
            gross_win: self.gross_win,
            gross_loss: self.gross_loss,
            best_trade: self.best_trade,
            min_win: self.min_win,
            bars_held: self.bars_held,
            max_losing_streak: self.max_losing_streak,
            max_winning_streak: self.max_winning_streak,
            worst_trade: self.worst_trade,
            max_drawdown: self.max_drawdown,
        })
    }

    fn validate(self) -> Result<(), CandidateUniverseRefusal> {
        if self.wins > self.trades {
            return Err(format!(
                "candidate wins {} exceed trades {}",
                self.wins, self.trades
            ));
        }
        let terminal = self
            .stopped
            .checked_add(self.trailed_stop)
            .and_then(|value| value.checked_add(self.trailed_profit))
            .and_then(|value| value.checked_add(self.targeted))
            .and_then(|value| value.checked_add(self.timed_out))
            .ok_or_else(|| "candidate terminal-exit counters overflow u64".to_owned())?;
        if terminal != self.trades {
            return Err(format!(
                "candidate terminal exits total {terminal}, not trades {}",
                self.trades
            ));
        }
        if self.fill_cost < 0 || self.max_drawdown < 0 {
            return Err(format!(
                "candidate non-negative facts are invalid: fill_cost={}, max_drawdown={}",
                self.fill_cost, self.max_drawdown
            ));
        }
        if self.pessimistic > self.optimistic {
            return Err(format!(
                "candidate pessimistic profit {} exceeds optimistic {}",
                self.pessimistic, self.optimistic
            ));
        }
        if self.gross_win < 0
            || self.gross_loss > 0
            || self.best_trade < 0
            || self.min_win < 0
            || self.worst_trade > 0
        {
            return Err("candidate signed direct money facts violate their domains".to_owned());
        }
        Ok(())
    }
}

/// One exact pre-admission `(closed mask, direction, exit coordinate)` row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateUniverseRowV1 {
    universe_id: [u8; 32],
    sequence: u64,
    candidate_semantic_digest: [u8; 32],
    mask_words: [u64; MASK_WORDS_V1],
    direction: TradeDirectionV1,
    family: InstrumentFamilyV1,
    rung_seconds: u32,
    support_hits: u64,
    exit: ExitCoordinateV1,
    execution_run_id: [u8; 32],
    horizon_bars: u32,
    evaluated_grid_digest: [u8; 32],
    cell_ordinal: u64,
    facts: CandidateDirectCellFactsV1,
}

impl CandidateUniverseRowV1 {
    /// Parent universe identity.
    #[must_use]
    pub const fn universe_id(self) -> [u8; 32] {
        self.universe_id
    }

    /// Zero-based row ordinal in the canonical block.
    #[must_use]
    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    /// Identity of the candidate semantics, independent of any final population ID.
    #[must_use]
    pub const fn candidate_semantic_digest(self) -> [u8; 32] {
        self.candidate_semantic_digest
    }

    /// Exact six durable condition-mask words.
    #[must_use]
    pub const fn mask_words(self) -> [u64; MASK_WORDS_V1] {
        self.mask_words
    }

    /// Long or short direction.
    #[must_use]
    pub const fn direction(self) -> TradeDirectionV1 {
        self.direction
    }

    /// Swept instrument family.
    #[must_use]
    pub const fn family(self) -> InstrumentFamilyV1 {
        self.family
    }

    /// Signal rung in seconds.
    #[must_use]
    pub const fn rung_seconds(self) -> u32 {
        self.rung_seconds
    }

    /// Closed-mask support count.
    #[must_use]
    pub const fn support_hits(self) -> u64 {
        self.support_hits
    }

    /// Exact exit coordinate.
    #[must_use]
    pub const fn exit(self) -> ExitCoordinateV1 {
        self.exit
    }

    /// Execution-run identity that produced this grid.
    #[must_use]
    pub const fn execution_run_id(self) -> [u8; 32] {
        self.execution_run_id
    }

    /// Exact one-minute horizon in bars.
    #[must_use]
    pub const fn horizon_bars(self) -> u32 {
        self.horizon_bars
    }

    /// Digest of the complete validated grid evaluation.
    #[must_use]
    pub const fn evaluated_grid_digest(self) -> [u8; 32] {
        self.evaluated_grid_digest
    }

    /// Canonical coordinate ordinal inside the validated grid.
    #[must_use]
    pub const fn cell_ordinal(self) -> u64 {
        self.cell_ordinal
    }

    /// Raw direct cell facts, before statistics or admission.
    #[must_use]
    pub const fn facts(self) -> CandidateDirectCellFactsV1 {
        self.facts
    }

    fn validate(
        self,
        descriptor: Option<&CandidateUniverseDescriptorV1>,
    ) -> Result<(), CandidateUniverseRefusal> {
        require_nonzero_digest("candidate universe", self.universe_id)?;
        require_nonzero_digest("candidate semantic", self.candidate_semantic_digest)?;
        require_nonzero_digest("candidate execution run", self.execution_run_id)?;
        require_nonzero_digest("candidate evaluated grid", self.evaluated_grid_digest)?;
        require_rung(self.rung_seconds)?;
        if self.horizon_bars == 0 {
            return Err("candidate row horizon is zero".to_owned());
        }
        runner::replay_mask::from_stored_words(self.mask_words)
            .map_err(|why| format!("candidate mask refused: {why:?}"))?;
        if self.mask_words == [0; MASK_WORDS_V1] {
            return Err("candidate row mask is empty".to_owned());
        }
        validate_exit(self.exit)?;
        self.facts.validate()?;
        if let Some(descriptor) = descriptor {
            if self.universe_id != descriptor.universe_id
                || self.family != descriptor.family
                || self.rung_seconds != descriptor.rung_seconds
                || self.horizon_bars != descriptor.horizon_bars
            {
                return Err("candidate row disagrees with its universe descriptor".to_owned());
            }
            if self.candidate_semantic_digest != candidate_semantic_digest(descriptor, &self) {
                return Err("candidate semantic digest does not match row fields".to_owned());
            }
        }
        Ok(())
    }

    fn payload(self) -> Result<[u8; ROW_PAYLOAD_BYTES], CandidateUniverseRefusal> {
        self.validate(None)?;
        let mut raw = [0_u8; ROW_PAYLOAD_BYTES];
        put_u32(&mut raw, 0, ROW_VERSION);
        put_bytes(&mut raw, 8, &self.universe_id);
        put_u64(&mut raw, 40, self.sequence);
        put_bytes(&mut raw, 48, &self.candidate_semantic_digest);
        for (index, word) in self.mask_words.iter().copied().enumerate() {
            put_u64(&mut raw, 80 + index * 8, word);
        }
        raw[128] = direction_byte(self.direction);
        raw[129] = family_byte(self.family);
        raw[130] = closure_byte(ClosureV1::Closed);
        put_u32(&mut raw, 132, self.rung_seconds);
        put_u64(&mut raw, 136, self.support_hits);
        put_u32(&mut raw, 144, encode_optional_index(self.exit.stop));
        put_u32(&mut raw, 148, encode_optional_index(self.exit.target));
        put_u32(&mut raw, 152, encode_optional_index(self.exit.tsl));
        let (arm, trail) = self.exit.ttp.unwrap_or((NONE_U32, NONE_U32));
        put_u32(&mut raw, 156, arm);
        put_u32(&mut raw, 160, trail);
        put_bytes(&mut raw, 168, &self.execution_run_id);
        put_u32(&mut raw, 200, self.horizon_bars);
        put_bytes(&mut raw, 208, &self.evaluated_grid_digest);
        put_u64(&mut raw, 240, self.cell_ordinal);
        encode_facts(self.facts, &mut raw);
        Ok(raw)
    }

    fn record(self) -> Result<[u8; ROW_STRIDE_BYTES], CandidateUniverseRefusal> {
        let payload = self.payload()?;
        let mut raw = [0_u8; ROW_STRIDE_BYTES];
        raw[..ROW_PAYLOAD_BYTES].copy_from_slice(&payload);
        put_bytes(
            &mut raw,
            ROW_PAYLOAD_BYTES,
            &digest_domain(ROW_SEAL_DOMAIN, &payload),
        );
        Ok(raw)
    }

    fn decode(record: &[u8]) -> Result<Self, CandidateUniverseRefusal> {
        if record.len() != ROW_STRIDE_BYTES {
            return Err(format!(
                "candidate row record is {} bytes, not {ROW_STRIDE_BYTES}",
                record.len()
            ));
        }
        let payload = record
            .get(..ROW_PAYLOAD_BYTES)
            .ok_or_else(|| "candidate row payload is absent".to_owned())?;
        require_seal(
            "candidate row",
            record
                .get(ROW_PAYLOAD_BYTES..)
                .ok_or_else(|| "candidate row seal is absent".to_owned())?,
            digest_domain(ROW_SEAL_DOMAIN, payload),
        )?;
        if get_u32(payload, 0)? != ROW_VERSION {
            return Err("candidate row version is unknown".to_owned());
        }
        require_zero(payload, 4, 4, "candidate row leading reserve")?;
        require_zero(payload, 131, 1, "candidate row tag reserve")?;
        require_zero(payload, 164, 4, "candidate row exit reserve")?;
        require_zero(payload, 204, 4, "candidate row horizon reserve")?;
        require_zero(payload, 440, 8, "candidate row trailing reserve")?;
        let closure = get_byte(payload, 130, "candidate row closure")?;
        if closure != closure_byte(ClosureV1::Closed) {
            return Err(format!(
                "candidate row closure byte {closure} is not closed"
            ));
        }
        let mut mask_words = [0_u64; MASK_WORDS_V1];
        for (index, word) in mask_words.iter_mut().enumerate() {
            *word = get_u64(payload, 80 + index * 8)?;
        }
        let arm = get_u32(payload, 156)?;
        let trail = get_u32(payload, 160)?;
        let ttp = match (arm, trail) {
            (NONE_U32, NONE_U32) => None,
            (NONE_U32, _) | (_, NONE_U32) => {
                return Err("candidate row has half-present TTP coordinate".to_owned());
            }
            pair => Some(pair),
        };
        let row = Self {
            universe_id: get_32(payload, 8)?,
            sequence: get_u64(payload, 40)?,
            candidate_semantic_digest: get_32(payload, 48)?,
            mask_words,
            direction: direction_from_byte(get_byte(payload, 128, "candidate row direction")?)?,
            family: family_from_byte(get_byte(payload, 129, "candidate row family")?)?,
            rung_seconds: get_u32(payload, 132)?,
            support_hits: get_u64(payload, 136)?,
            exit: ExitCoordinateV1 {
                stop: decode_optional_index(get_u32(payload, 144)?),
                target: decode_optional_index(get_u32(payload, 148)?),
                tsl: decode_optional_index(get_u32(payload, 152)?),
                ttp,
            },
            execution_run_id: get_32(payload, 168)?,
            horizon_bars: get_u32(payload, 200)?,
            evaluated_grid_digest: get_32(payload, 208)?,
            cell_ordinal: get_u64(payload, 240)?,
            facts: decode_facts(payload)?,
        };
        row.validate(None)?;
        Ok(row)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CandidateCountsV1 {
    directions_expected: u64,
    directions_evaluated: u64,
    long_cells_per_mask: u64,
    short_cells_per_mask: u64,
    long_cells_evaluated: u64,
    short_cells_evaluated: u64,
    exit_cells_expected: u64,
    exit_cells_evaluated: u64,
}

/// Receipt-last structural record for one candidate-universe byte block.
///
/// Decoding proves integrity, format and internal reconciliation. It is not a
/// live Step-3 authority by itself; only the crate-private typed producer plus
/// its exact append/reopen commit can cross into Pre-Admission Data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateUniverseReceiptV1 {
    descriptor: CandidateUniverseDescriptorV1,
    reconciliation: CompletionReconciliationV2,
    counts: CandidateCountsV1,
    row_count: u64,
    ordered_row_digest: [u8; 32],
    content_digest: [u8; 32],
}

impl CandidateUniverseReceiptV1 {
    fn for_rows(
        descriptor: &CandidateUniverseDescriptorV1,
        reconciliation: CompletionReconciliationV2,
        rows: &[CandidateUniverseRowV1],
    ) -> Result<Self, CandidateUniverseRefusal> {
        descriptor.validate()?;
        let counts = expected_counts(reconciliation)?;
        let row_count = u64::try_from(rows.len())
            .map_err(|_| "candidate row count does not fit u64".to_owned())?;
        if row_count != counts.exit_cells_expected {
            return Err(format!(
                "candidate rows total {row_count}, not the reconciled expected {}",
                counts.exit_cells_expected
            ));
        }
        validate_rows(descriptor, reconciliation, rows)?;
        let ordered_row_digest = digest_ordered_rows(descriptor, rows)?;
        let mut receipt = Self {
            descriptor: *descriptor,
            reconciliation,
            counts,
            row_count,
            ordered_row_digest,
            content_digest: [0; 32],
        };
        let payload = receipt.payload_without_content_check();
        receipt.content_digest = digest_domain(RECEIPT_CONTENT_DOMAIN, &payload);
        receipt.validate()?;
        Ok(receipt)
    }

    /// Candidate-universe identity.
    #[must_use]
    pub const fn universe_id(self) -> [u8; 32] {
        self.descriptor.universe_id
    }

    /// Full content digest and on-disk record seal.
    #[must_use]
    pub const fn content_digest(self) -> [u8; 32] {
        self.content_digest
    }

    /// Swept index family.
    #[must_use]
    pub const fn family(self) -> InstrumentFamilyV1 {
        self.descriptor.family
    }

    /// Signal rung in seconds.
    #[must_use]
    pub const fn rung_seconds(self) -> u32 {
        self.descriptor.rung_seconds
    }

    /// Exact one-minute outcome horizon in bars.
    #[must_use]
    pub const fn horizon_bars(self) -> u32 {
        self.descriptor.horizon_bars
    }

    /// Inclusive requested month span.
    #[must_use]
    pub const fn requested_span(self) -> RequestedSpanIdentityV1 {
        self.descriptor.requested_span
    }

    /// Pre-admission source identities.
    #[must_use]
    pub const fn identities(self) -> CandidateUniverseIdentitiesV1 {
        self.descriptor.identities
    }

    /// Complete signal and exact-one-minute calendar evidence.
    #[must_use]
    pub const fn calendar_coverage(self) -> CandidateCalendarCoverageV1 {
        self.descriptor.calendar_coverage
    }

    /// Exact signal stream whose masks/support were swept.
    #[must_use]
    pub const fn signal_stream(self) -> CandidateSignalStreamV1 {
        self.descriptor.signal_stream
    }

    /// Exact requested-span one-minute stream evaluated by the grids.
    #[must_use]
    pub const fn execution_stream(self) -> CandidateExecutionStreamV1 {
        self.descriptor.execution_stream
    }

    /// Exact signal-column identity.
    #[must_use]
    pub const fn signal_column_digest(self) -> [u8; 32] {
        self.descriptor.signal_column_digest
    }

    /// Exact projected execution-column identity.
    #[must_use]
    pub const fn execution_column_digest(self) -> [u8; 32] {
        self.descriptor.execution_column_digest
    }

    /// Number of committed candidate rows.
    #[must_use]
    pub const fn row_count(self) -> u64 {
        self.row_count
    }

    /// Digest of the exact ordered row payload sequence.
    #[must_use]
    pub const fn ordered_row_digest(self) -> [u8; 32] {
        self.ordered_row_digest
    }

    /// Exact sweep/grid reconciliation carried by this completion.
    #[must_use]
    pub const fn reconciliation(self) -> CompletionReconciliationV2 {
        self.reconciliation
    }

    fn validate(self) -> Result<(), CandidateUniverseRefusal> {
        self.descriptor.validate()?;
        let expected = expected_counts(self.reconciliation)?;
        let cells_per_mask = self
            .reconciliation
            .exit_cells_per_mask
            .long()
            .checked_add(self.reconciliation.exit_cells_per_mask.short())
            .ok_or_else(|| "candidate per-mask Long+Short width overflowed u64".to_owned())?;
        let exact_rows = self
            .reconciliation
            .closed_itemsets
            .checked_mul(cells_per_mask)
            .ok_or_else(|| "candidate closed-mask row count overflowed u64".to_owned())?;
        if self.counts != expected
            || self.row_count != expected.exit_cells_expected
            || self.row_count != exact_rows
            || expected.exit_cells_expected != expected.exit_cells_evaluated
            || expected.directions_expected != expected.directions_evaluated
        {
            return Err("candidate receipt reconciliation fields disagree".to_owned());
        }
        if self.row_count != 0 && self.row_count < 2 {
            return Err(
                "candidate nonempty production must contain at least one Long and one Short row"
                    .to_owned(),
            );
        }
        require_nonzero_digest("candidate ordered rows", self.ordered_row_digest)?;
        require_nonzero_digest("candidate completion", self.content_digest)?;
        let payload = self.payload_without_content_check();
        if digest_domain(RECEIPT_CONTENT_DOMAIN, &payload) != self.content_digest {
            return Err("candidate completion digest does not match receipt fields".to_owned());
        }
        Ok(())
    }

    fn payload_without_content_check(self) -> [u8; RECEIPT_PAYLOAD_BYTES] {
        let mut raw = [0_u8; RECEIPT_PAYLOAD_BYTES];
        put_u32(&mut raw, 0, RECEIPT_VERSION);
        put_u32(&mut raw, 4, ROW_VERSION);
        put_bytes(&mut raw, 8, &self.descriptor.universe_id);
        put_u64(&mut raw, 40, self.row_count);
        put_bytes(&mut raw, 48, &self.ordered_row_digest);
        for (index, value) in [
            self.reconciliation.sweep_trials,
            self.reconciliation.frequent_itemsets,
            self.reconciliation.infrequent_itemsets,
            self.reconciliation.closed_itemsets,
            self.reconciliation.redundant_itemsets,
            self.reconciliation.unknown_closure_itemsets,
            self.counts.directions_expected,
            self.counts.directions_evaluated,
            self.counts.long_cells_per_mask,
            self.counts.short_cells_per_mask,
            self.counts.long_cells_evaluated,
            self.counts.short_cells_evaluated,
            self.counts.exit_cells_expected,
            self.counts.exit_cells_evaluated,
        ]
        .into_iter()
        .enumerate()
        {
            put_u64(&mut raw, 80 + index * 8, value);
        }
        put_u32(&mut raw, 192, self.reconciliation.extinction_depth);
        put_u32(&mut raw, 196, self.descriptor.rung_seconds);
        put_u32(&mut raw, 200, self.descriptor.horizon_bars);
        raw[204] = family_byte(self.descriptor.family);
        raw[205] = u8::from(self.reconciliation.extinction_complete);
        raw[206] = u8::from(self.reconciliation.closure_complete);
        let identities = self.descriptor.identities;
        for (offset, digest) in [
            (240, identities.data_digest),
            (272, identities.feed_digest),
            (304, identities.source_commit_digest),
            (336, identities.vocabulary_digest),
            (368, identities.evaluation_policy_digest),
            (400, identities.exit_grids.long.policy_digest),
            (432, identities.exit_grids.long.resolved_digest),
            (464, identities.exit_grids.short.policy_digest),
            (496, identities.exit_grids.short.resolved_digest),
            (528, identities.calendar_policy_digest),
            (560, identities.daily_reference_policy_digest),
        ] {
            put_bytes(&mut raw, offset, &digest);
        }
        let span = self.descriptor.requested_span.canonical_bytes();
        put_bytes(&mut raw, 592, &span);
        put_bytes(&mut raw, 612, &self.descriptor.requested_span.digest());
        put_bytes(&mut raw, 656, &self.descriptor.calendar_coverage.encode());
        encode_signal_stream(self.descriptor.signal_stream, &mut raw, 752);
        encode_execution_stream(self.descriptor.execution_stream, &mut raw, 808);
        put_bytes(&mut raw, 864, &self.descriptor.signal_column_digest);
        put_bytes(&mut raw, 896, &self.descriptor.execution_column_digest);
        raw
    }

    fn record(self) -> Result<[u8; RECEIPT_STRIDE_BYTES], CandidateUniverseRefusal> {
        self.validate()?;
        let payload = self.payload_without_content_check();
        let mut raw = [0_u8; RECEIPT_STRIDE_BYTES];
        raw[..RECEIPT_PAYLOAD_BYTES].copy_from_slice(&payload);
        put_bytes(&mut raw, RECEIPT_PAYLOAD_BYTES, &self.content_digest);
        Ok(raw)
    }

    fn decode(record: &[u8]) -> Result<Self, CandidateUniverseRefusal> {
        if record.len() != RECEIPT_STRIDE_BYTES {
            return Err(format!(
                "candidate receipt record is {} bytes, not {RECEIPT_STRIDE_BYTES}",
                record.len()
            ));
        }
        let payload = record
            .get(..RECEIPT_PAYLOAD_BYTES)
            .ok_or_else(|| "candidate receipt payload is absent".to_owned())?;
        let content_digest = get_32(record, RECEIPT_PAYLOAD_BYTES)?;
        require_seal(
            "candidate receipt",
            record
                .get(RECEIPT_PAYLOAD_BYTES..)
                .ok_or_else(|| "candidate receipt seal is absent".to_owned())?,
            digest_domain(RECEIPT_CONTENT_DOMAIN, payload),
        )?;
        if get_u32(payload, 0)? != RECEIPT_VERSION || get_u32(payload, 4)? != ROW_VERSION {
            return Err("candidate receipt or row schema version is unknown".to_owned());
        }
        require_zero(payload, 207, 1, "candidate receipt flags reserve")?;
        require_zero(payload, 644, 12, "candidate receipt span reserve")?;
        require_zero(payload, 928, 16, "candidate receipt trailing reserve")?;
        let requested_span = decode_span(
            payload
                .get(592..612)
                .ok_or_else(|| "candidate requested span bytes are absent".to_owned())?,
        )?;
        if requested_span.digest() != get_32(payload, 612)? {
            return Err("candidate requested-span digest does not match endpoints".to_owned());
        }
        require_zero(
            payload,
            208,
            32,
            "candidate receipt former shared-run reserve",
        )?;
        let identities = decode_receipt_identities(payload)?;
        let descriptor = decode_receipt_descriptor(payload, requested_span, &identities)?;
        let reconciliation = decode_receipt_reconciliation(payload)?;
        let receipt = Self {
            descriptor,
            reconciliation,
            counts: decode_receipt_counts(payload)?,
            row_count: get_u64(payload, 40)?,
            ordered_row_digest: get_32(payload, 48)?,
            content_digest,
        };
        receipt.validate()?;
        Ok(receipt)
    }
}

fn decode_receipt_identities(
    payload: &[u8],
) -> Result<CandidateUniverseIdentitiesV1, CandidateUniverseRefusal> {
    Ok(CandidateUniverseIdentitiesV1 {
        data_digest: get_32(payload, 240)?,
        feed_digest: get_32(payload, 272)?,
        source_commit_digest: get_32(payload, 304)?,
        vocabulary_digest: get_32(payload, 336)?,
        evaluation_policy_digest: get_32(payload, 368)?,
        exit_grids: LongShortExitGridIdentitiesV2 {
            long: SideExitGridIdentityV2 {
                policy_digest: get_32(payload, 400)?,
                resolved_digest: get_32(payload, 432)?,
            },
            short: SideExitGridIdentityV2 {
                policy_digest: get_32(payload, 464)?,
                resolved_digest: get_32(payload, 496)?,
            },
        },
        calendar_policy_digest: get_32(payload, 528)?,
        daily_reference_policy_digest: get_32(payload, 560)?,
    })
}

fn decode_receipt_descriptor(
    payload: &[u8],
    requested_span: RequestedSpanIdentityV1,
    identities: &CandidateUniverseIdentitiesV1,
) -> Result<CandidateUniverseDescriptorV1, CandidateUniverseRefusal> {
    Ok(CandidateUniverseDescriptorV1 {
        family: family_from_byte(get_byte(payload, 204, "candidate receipt family")?)?,
        rung_seconds: get_u32(payload, 196)?,
        horizon_bars: get_u32(payload, 200)?,
        requested_span,
        identities: *identities,
        calendar_coverage: CandidateCalendarCoverageV1::decode(
            payload
                .get(656..752)
                .ok_or_else(|| "candidate calendar coverage bytes are absent".to_owned())?,
        )?,
        signal_stream: decode_signal_stream(payload, 752)?,
        execution_stream: decode_execution_stream(payload, 808)?,
        signal_column_digest: get_32(payload, 864)?,
        execution_column_digest: get_32(payload, 896)?,
        universe_id: get_32(payload, 8)?,
    })
}

fn decode_receipt_reconciliation(
    payload: &[u8],
) -> Result<CompletionReconciliationV2, CandidateUniverseRefusal> {
    Ok(CompletionReconciliationV2 {
        sweep_trials: get_u64(payload, 80)?,
        frequent_itemsets: get_u64(payload, 88)?,
        infrequent_itemsets: get_u64(payload, 96)?,
        closed_itemsets: get_u64(payload, 104)?,
        redundant_itemsets: get_u64(payload, 112)?,
        unknown_closure_itemsets: get_u64(payload, 120)?,
        exit_cells_per_mask: ExitCellsPerMaskV2::new(
            get_u64(payload, 144)?,
            get_u64(payload, 152)?,
        )
        .map_err(|why| format!("candidate grid-cell counts refused: {why}"))?,
        extinction_depth: get_u32(payload, 192)?,
        extinction_complete: decode_bool(
            get_byte(payload, 205, "candidate extinction")?,
            "candidate extinction",
        )?,
        closure_complete: decode_bool(
            get_byte(payload, 206, "candidate closure")?,
            "candidate closure",
        )?,
    })
}

fn decode_receipt_counts(payload: &[u8]) -> Result<CandidateCountsV1, CandidateUniverseRefusal> {
    Ok(CandidateCountsV1 {
        directions_expected: get_u64(payload, 128)?,
        directions_evaluated: get_u64(payload, 136)?,
        long_cells_per_mask: get_u64(payload, 144)?,
        short_cells_per_mask: get_u64(payload, 152)?,
        long_cells_evaluated: get_u64(payload, 160)?,
        short_cells_evaluated: get_u64(payload, 168)?,
        exit_cells_expected: get_u64(payload, 176)?,
        exit_cells_evaluated: get_u64(payload, 184)?,
    })
}

/// Module-private rows and receipt minted only by the typed producer.
#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedCandidateUniverseV1 {
    rows: Vec<CandidateUniverseRowV1>,
    receipt: CandidateUniverseReceiptV1,
}

impl PreparedCandidateUniverseV1 {
    /// Validates the complete canonical block before any file is changed.
    ///
    /// # Errors
    ///
    /// Every row/receipt reconciliation, ordering, identity, uniqueness or
    /// arithmetic refusal is returned by name.
    fn new(
        descriptor: &CandidateUniverseDescriptorV1,
        reconciliation: CompletionReconciliationV2,
        rows: Vec<CandidateUniverseRowV1>,
    ) -> Result<Self, CandidateUniverseRefusal> {
        let receipt =
            CandidateUniverseReceiptV1::for_rows(descriptor, reconciliation, rows.as_slice())?;
        Ok(Self { rows, receipt })
    }

    /// Read-only completion receipt.
    #[cfg(test)]
    #[must_use]
    const fn receipt(&self) -> &CandidateUniverseReceiptV1 {
        &self.receipt
    }

    /// Complete canonical row sequence.
    #[cfg(test)]
    #[must_use]
    fn rows(&self) -> &[CandidateUniverseRowV1] {
        &self.rows
    }
}

/// Opaque source evidence retained only for the Candidate-to-Pre-Admission join.
///
/// No public caller can construct or unpack this bundle. Its load ceilings and
/// exact streams are fixed when the Candidate producer validates its source.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CandidatePreAdmissionInputsV1<'a> {
    signal_bars: &'a [Candle],
    minute_context: &'a [Candle],
    daily_reference: DailyReferenceBinding<'a>,
    execution_series: ExecutionSeriesV1<'a>,
    signal_calendar: CompleteCalendarReceiptV2,
    execution_calendar: CompleteCalendarReceiptV2,
    signal_load_bound: StoredSpanLoadBoundV1,
    minute_load_bound: StoredSpanLoadBoundV1,
    daily_load_bound: StoredSpanLoadBoundV1,
}

impl<'a> CandidatePreAdmissionInputsV1<'a> {
    pub(crate) const fn signal_bars(self) -> &'a [Candle] {
        self.signal_bars
    }

    pub(crate) const fn minute_context(self) -> &'a [Candle] {
        self.minute_context
    }

    pub(crate) const fn daily_reference(self) -> DailyReferenceBinding<'a> {
        self.daily_reference
    }

    pub(crate) const fn execution_series(self) -> ExecutionSeriesV1<'a> {
        self.execution_series
    }

    pub(crate) const fn signal_calendar(self) -> CompleteCalendarReceiptV2 {
        self.signal_calendar
    }

    pub(crate) const fn execution_calendar(self) -> CompleteCalendarReceiptV2 {
        self.execution_calendar
    }

    pub(crate) const fn signal_load_bound(self) -> StoredSpanLoadBoundV1 {
        self.signal_load_bound
    }

    pub(crate) const fn minute_load_bound(self) -> StoredSpanLoadBoundV1 {
        self.minute_load_bound
    }

    pub(crate) const fn daily_load_bound(self) -> StoredSpanLoadBoundV1 {
        self.daily_load_bound
    }
}

/// One complete naturally-extinct crate-internal walk and its uncommitted bytes.
///
/// Rows and the raw descriptor remain private. Only a crate-internal stored
/// authority orchestrator may prepare and commit them. Public callers receive
/// only the reopened audit/page surface.
#[derive(Debug)]
pub(crate) struct ProducedCandidateUniverseV1<'a> {
    population_run: PopulationRun,
    prepared: PreparedCandidateUniverseV1,
    pre_admission: CandidatePreAdmissionInputsV1<'a>,
    observations: CandidateFamilyObservationsV1,
    base_evidence: PreparedBaseEvidenceV2,
}

impl<'a> ProducedCandidateUniverseV1<'a> {
    /// Complete uncapped engine result retained beside the durable preparation.
    #[must_use]
    pub(crate) const fn population_run(&self) -> &PopulationRun {
        &self.population_run
    }

    /// Receipt that will be appended after every row is synced.
    #[must_use]
    pub(crate) const fn receipt(&self) -> CandidateUniverseReceiptV1 {
        self.prepared.receipt
    }

    /// Exact number of prepared fixed-stride rows.
    #[must_use]
    pub(crate) fn row_count(&self) -> usize {
        self.prepared.rows.len()
    }

    pub(crate) const fn pre_admission_inputs(&self) -> CandidatePreAdmissionInputsV1<'a> {
        self.pre_admission
    }

    /// Exact accepted-session observations derived from every evaluated cell.
    #[must_use]
    pub(crate) const fn observations(&self) -> &CandidateFamilyObservationsV1 {
        &self.observations
    }

    /// Same-pass fixed Base Evidence records bound to this Candidate receipt.
    ///
    /// These records are an in-memory Phase-A result, not durable production
    /// authority: they have not been appended, synced and freshly reopened.
    #[must_use]
    #[cfg(test)]
    pub(crate) const fn base_evidence(&self) -> &PreparedBaseEvidenceV2 {
        &self.base_evidence
    }

    /// Commits the complete block through the private writer and reopens it
    /// read-only before returning success.
    ///
    /// # Errors
    ///
    /// Returns any receipt-last append, exact-retry, corruption, stale-path,
    /// bounds, I/O or reopen mismatch refusal. No partial audit is returned.
    pub(crate) fn append_and_reopen(
        &self,
        root: impl AsRef<Path>,
        bounds: CandidateUniverseBoundsV1,
    ) -> Result<CandidateUniverseProductionCommitV1, CandidateUniverseRefusal> {
        self.base_evidence
            .validate_candidate_receipt(&self.prepared.receipt)
            .map_err(|why| why.to_string())?;
        append_produced_candidate_universe_v1(root, bounds, self)
    }

    /// Persists the same-pass Base records only after the exact Candidate
    /// completion has itself been freshly reopened.
    pub(crate) fn append_base_evidence_and_reopen(
        &self,
        root: &Path,
        bounds: BaseEvidenceLedgerBoundsV2,
        candidate: &CandidateUniverseReopenAuditV1,
    ) -> Result<BaseEvidenceProductionCommitV2, CandidateUniverseRefusal> {
        if candidate.receipt() != self.prepared.receipt {
            return Err(
                "Base Evidence durable append received a different reopened Candidate receipt"
                    .to_owned(),
            );
        }
        append_and_reopen_base_evidence_v2(root, bounds, candidate, &self.base_evidence)
            .map_err(|why| why.to_string())
    }
}

/// Requires the canonical NIFTY-then-BANKNIFTY durable Base cohort.
pub(crate) fn pair_candidate_base_evidence_v2(
    nifty: &BaseEvidenceReopenAuditV2,
    banknifty: &BaseEvidenceReopenAuditV2,
) -> Result<PairedBaseEvidenceAuthorityV2, CandidateUniverseRefusal> {
    pair_base_evidence_authority_v2(nifty, banknifty).map_err(|why| why.to_string())
}

/// Runs the uncapped candidate frontier and expands every closed mask through
/// the complete long and short dynamic exit grids.
///
/// Order is exactly ladder depth, mask words, long coordinates, then short
/// coordinates. The callback observes engine-level progress only; it cannot
/// alter enumeration, rows or identities. A sink refusal withholds the entire
/// local preparation.
///
/// # Cost
///
/// The total is input-dependent: one naturally-extinct sweep plus one complete
/// grid evaluation and validation per `(closed mask, direction)`. Every grid
/// cell is then replayed over its exact execution series by `materialize_cell`;
/// its resulting `TradeRow` sequence is folded once for Base Evidence and once
/// for accepted-session observations before the rows are dropped. Only the
/// final Candidate/Base record projections are fixed-cost. Retained Candidate
/// rows and fixed Base records are linear in the complete
/// closed-frontier-by-grid population, observation space also depends on its
/// accepted-session width, and one cell's materialized trades are transient.
/// Neither the per-cell replay/folds nor the whole operation is O(1).
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
///
/// # Errors
///
/// Refuses any changed source seal, incomplete extinction/closure, unknown
/// closure, foreign run/grid/column, arithmetic or configured row-bound breach,
/// allocation failure, duplicate/reordered row, or receipt mismatch.
#[expect(
    clippy::too_many_lines,
    reason = "one production orchestration keeps source destruction, same-pass evidence capture and receipt sealing in their exact authority order"
)]
pub(crate) fn produce_candidate_universe_v1<'a>(
    sweeper: &Sweeper,
    source: CandidateUniverseProductionSourceV1<'a>,
    bounds: CandidateUniverseBoundsV1,
    on_level: &dyn Fn(&engine::Frontier, usize, u64),
) -> Result<ProducedCandidateUniverseV1<'a>, CandidateUniverseRefusal> {
    source.validate()?;
    let identities = production_identities(&source, sweeper)?;
    let descriptor = CandidateUniverseDescriptorV1::new(
        source.family,
        source.rung_seconds,
        source.horizon,
        source.requested_span,
        &identities,
        source.signal_calendar,
        source.execution_calendar,
        source.signal_bars,
        &source.signal_column,
        source.execution_series,
        &source.execution_column,
    )?;
    let params = execution_run_params(sweeper, &source);
    let CandidateUniverseProductionSourceV1 {
        signal_column,
        execution_column,
        signal_bars,
        reference_minute_context,
        daily_reference,
        execution_series,
        long_exit_grid,
        short_exit_grid,
        horizon,
        rung_seconds,
        signal_calendar,
        execution_calendar,
        signal_load_bound,
        minute_load_bound,
        daily_load_bound,
        ..
    } = source;
    let pre_admission = CandidatePreAdmissionInputsV1 {
        signal_bars,
        minute_context: reference_minute_context,
        daily_reference,
        execution_series,
        signal_calendar,
        execution_calendar,
        signal_load_bound,
        minute_load_bound,
        daily_load_bound,
    };
    let mut observation_builder = CandidateObservationBuilderV1::from_exact_execution(
        execution_calendar,
        execution_series.bars(),
        &execution_column,
    )?;
    let base_bounds = BaseEvidenceBoundsV2::new(
        bounds.max_rows(),
        minute_load_bound.max_records(),
        signal_load_bound.max_records(),
        signal_load_bound.max_records(),
    )
    .map_err(|why| why.to_string())?;
    // The engine retains its owned Column in PopulationRun. Base Evidence
    // needs the same masks while retirement callbacks run, so take one exact,
    // explicitly bounded and fallible copy rather than rebuilding evaluator
    // state or using infallible Vec allocation.
    let support_signal_column = base_bounds
        .try_clone_signal_column(&signal_column)
        .map_err(|why| why.to_string())?;
    let mut base_builder = BaseEvidenceBuilderV2::new(base_bounds);
    let mut rows = Vec::new();
    let population_run =
        sweeper.run_prepared_population_by_reporting(signal_column, on_level, |member| {
            expand_population_member(
                member,
                &descriptor,
                params,
                signal_bars,
                reference_minute_context,
                daily_reference,
                execution_series,
                &execution_column,
                long_exit_grid,
                short_exit_grid,
                horizon,
                rung_seconds,
                bounds,
                &mut rows,
                &mut observation_builder,
                &support_signal_column,
                base_bounds,
                &mut base_builder,
            )
        })?;
    let reconciliation = reconcile_candidate_population(
        &population_run,
        &source_grid_counts(long_exit_grid, short_exit_grid)?,
        rows.len(),
    )?;
    let prepared = PreparedCandidateUniverseV1::new(&descriptor, reconciliation, rows)?;
    let observations = observation_builder.seal(&prepared.receipt)?;
    let base_evidence = base_builder
        .seal(&prepared.receipt)
        .map_err(|why| why.to_string())?;
    Ok(ProducedCandidateUniverseV1 {
        population_run,
        prepared,
        pre_admission,
        observations,
        base_evidence,
    })
}

/// Explicit resource ceilings applied before any ledger-sized allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateUniverseBoundsV1 {
    max_rows: u64,
    max_universes: u64,
}

impl CandidateUniverseBoundsV1 {
    /// Creates non-zero durable row and completion ceilings.
    ///
    /// # Errors
    ///
    /// Zero cannot describe an operational ledger and is refused.
    pub fn new(max_rows: u64, max_universes: u64) -> Result<Self, CandidateUniverseRefusal> {
        if max_rows == 0 || max_universes == 0 {
            return Err(format!(
                "candidate ledger bounds must be non-zero; rows={max_rows}, universes={max_universes}"
            ));
        }
        Ok(Self {
            max_rows,
            max_universes,
        })
    }

    /// Maximum row records accepted from disk or one append.
    #[must_use]
    pub const fn max_rows(self) -> u64 {
        self.max_rows
    }

    /// Maximum completion records accepted from disk.
    #[must_use]
    pub const fn max_universes(self) -> u64 {
        self.max_universes
    }
}

/// Structural proof recovered from existing sealed bytes.
///
/// A read-only reopen proves the sealed layout and reconciliation it checked;
/// by itself it cannot prove how those bytes were authored. The production
/// commit result is stronger because it compares this reopen byte-for-byte
/// with the private preparation minted from typed sources in the same call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateUniverseReopenAuditV1 {
    first_row: u64,
    receipt: CandidateUniverseReceiptV1,
}

impl CandidateUniverseReopenAuditV1 {
    /// First physical row record occupied by this contiguous block.
    #[must_use]
    pub const fn first_row(self) -> u64 {
        self.first_row
    }

    /// Number of committed rows in the block.
    #[must_use]
    pub const fn row_count(self) -> u64 {
        self.receipt.row_count
    }

    /// Complete typed receipt.
    #[must_use]
    pub const fn receipt(self) -> CandidateUniverseReceiptV1 {
        self.receipt
    }

    /// Candidate-universe identity.
    #[must_use]
    pub const fn universe_id(self) -> [u8; 32] {
        self.receipt.universe_id()
    }

    /// Full completion/content digest.
    #[must_use]
    pub const fn content_digest(self) -> [u8; 32] {
        self.receipt.content_digest()
    }

    /// Exact signal stream fact.
    #[must_use]
    pub const fn signal_stream(self) -> CandidateSignalStreamV1 {
        self.receipt.signal_stream()
    }

    /// Exact requested-span execution stream fact.
    #[must_use]
    pub const fn execution_stream(self) -> CandidateExecutionStreamV1 {
        self.receipt.execution_stream()
    }

    /// Exact signal-column digest.
    #[must_use]
    pub const fn signal_column_digest(self) -> [u8; 32] {
        self.receipt.signal_column_digest()
    }

    /// Exact projected execution-column digest.
    #[must_use]
    pub const fn execution_column_digest(self) -> [u8; 32] {
        self.receipt.execution_column_digest()
    }
}

/// One bounded contiguous page of committed candidate rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateUniversePageV1 {
    rows: Vec<CandidateUniverseRowV1>,
}

impl CandidateUniversePageV1 {
    /// Borrow the exact rows in physical/canonical order.
    #[must_use]
    pub fn rows(&self) -> &[CandidateUniverseRowV1] {
        &self.rows
    }
}

/// Opaque authenticated Candidate input for the successor Population seam.
///
/// Construction is private to the complete-ledger read below. The projection
/// retains both the decoded typed row and the literal sealed 480-byte record,
/// plus the exact Candidate-row identity used by Base Evidence V2. Successor
/// code may inspect these values but cannot author or replace any of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuthenticatedCandidatePopulationRowV1 {
    row: CandidateUniverseRowV1,
    canonical_record: [u8; ROW_STRIDE_BYTES],
    base_candidate_row_digest: [u8; 32],
}

impl AuthenticatedCandidatePopulationRowV1 {
    /// Decoded Candidate row authenticated by the exact completed block.
    #[must_use]
    pub(crate) const fn row(&self) -> CandidateUniverseRowV1 {
        self.row
    }

    /// Literal fixed Candidate record, including its BLAKE3 row seal.
    #[must_use]
    pub(crate) const fn canonical_record(&self) -> &[u8; ROW_STRIDE_BYTES] {
        &self.canonical_record
    }

    /// The exact Base Evidence V2 Candidate-row identity for these bytes.
    #[must_use]
    pub(crate) const fn base_candidate_row_digest(&self) -> [u8; 32] {
        self.base_candidate_row_digest
    }
}

/// Exact scalar policy/source facts projected from one retained Candidate
/// execution source for the receipt-last Execution V3 codec.
///
/// The projection is not an authoring capability: although its scalar fields
/// are visible inside this crate, the production Execution V3 door accepts only
/// [`CandidateExecutionReplayAuthorityV1`], whose fields and constructor remain
/// private here. Raw bars, columns, grids, masks and run inputs never cross this
/// boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CandidateExecutionParameterFactsV1 {
    pub(crate) family: InstrumentFamilyV1,
    pub(crate) direction: TradeDirectionV1,
    pub(crate) rung_seconds: u32,
    pub(crate) horizon_bars: u32,
    pub(crate) policy_digest: [u8; 32],
    pub(crate) resolution_digest: [u8; 32],
    pub(crate) training_digest: [u8; 32],
    pub(crate) instrument_digest: [u8; 32],
    pub(crate) feed_digest: [u8; 32],
    pub(crate) commit_digest: [u8; 32],
    pub(crate) calendar_digest: [u8; 32],
    pub(crate) cost_model_id: [u8; 32],
    pub(crate) evaluation_fingerprint: [u8; indicators::column::EVALUATION_SPEC_FINGERPRINT_V1_LEN],
    pub(crate) range_resolution: RangeResolutionV1,
    pub(crate) selector: ExitGridSelectorV1,
    pub(crate) forced_stop: ForcedStopV1,
    pub(crate) forced_stop_index: Option<u32>,
    pub(crate) run_params: [u64; 4],
    pub(crate) max_levels: u64,
    pub(crate) ratio_min_hundredths: i64,
    pub(crate) ratio_max_hundredths: i64,
    pub(crate) max_ratio_pairs: u64,
    pub(crate) policy_max_cells: u64,
    pub(crate) resolved_stop_count: u64,
    pub(crate) resolved_target_count: u64,
    pub(crate) resolved_trail_count: u64,
    pub(crate) resolved_ratio_pair_count: u64,
    pub(crate) resolved_cell_count: u64,
    pub(crate) forced_stop_ppm: i64,
    pub(crate) max_ambiguity_bars: u64,
    pub(crate) max_gap_bars: u64,
    pub(crate) training_bars: u64,
    pub(crate) training_first_ts_micros: i64,
    pub(crate) training_last_ts_micros: i64,
    pub(crate) stop_percentiles: Vec<RationalPercentileV1>,
    pub(crate) target_percentiles: Vec<RationalPercentileV1>,
    pub(crate) trail_percentiles: Vec<RationalPercentileV1>,
}

/// One authenticated Candidate row paired with Runner's exact terminal
/// classification from the same rebuilt execution column and complete grid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CandidateExecutionDispositionProjectionV1 {
    candidate: AuthenticatedCandidatePopulationRowV1,
    disposition: ExecutionDispositionV1,
}

impl CandidateExecutionDispositionProjectionV1 {
    pub(crate) fn into_parts(
        self,
    ) -> (
        AuthenticatedCandidatePopulationRowV1,
        ExecutionDispositionV1,
    ) {
        (self.candidate, self.disposition)
    }
}

/// Nonconstructible one-family replay authority for Execution V3.
///
/// It can be minted only by revalidating a complete authenticated Candidate
/// block against a freshly rebuilt [`CandidateUniverseProductionSourceV1`].
/// The two parameter projections are canonical Long then Short and the
/// dispositions retain exact Candidate order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CandidateExecutionReplayAuthorityV1 {
    receipt: CandidateUniverseReceiptV1,
    parameters: [CandidateExecutionParameterFactsV1; 2],
    dispositions: Vec<CandidateExecutionDispositionProjectionV1>,
}

impl CandidateExecutionReplayAuthorityV1 {
    #[must_use]
    pub(crate) const fn receipt(&self) -> CandidateUniverseReceiptV1 {
        self.receipt
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        CandidateUniverseReceiptV1,
        [CandidateExecutionParameterFactsV1; 2],
        Vec<CandidateExecutionDispositionProjectionV1>,
    ) {
        (self.receipt, self.parameters, self.dispositions)
    }
}

/// Verifies one literal Candidate record embedded by a Population successor.
///
/// This is an integrity projection, not Candidate-completion provenance.  It
/// proves the nested seal, reserved bytes and row semantics, requires the
/// decoded row to reproduce every canonical byte, and derives the same Base
/// Evidence V2 Candidate-row identity used by the retained production join.
/// Every Population successor must still match the returned identities to its
/// authenticated Candidate Completion and Finalization authorities.
///
/// # Errors
///
/// Refuses a corrupt, noncanonical or semantically invalid Candidate record.
pub(crate) fn verify_population_candidate_canonical_record_v1(
    canonical_record: &[u8; ROW_STRIDE_BYTES],
) -> Result<AuthenticatedCandidatePopulationRowV1, CandidateUniverseRefusal> {
    let row = CandidateUniverseRowV1::decode(canonical_record)?;
    if row.record()? != *canonical_record {
        return Err(
            "Population Candidate V1 row does not reproduce its literal sealed record".to_owned(),
        );
    }
    Ok(AuthenticatedCandidatePopulationRowV1 {
        row,
        canonical_record: *canonical_record,
        base_candidate_row_digest: candidate_row_digest_v2(canonical_record),
    })
}

/// Backward-compatible V5-named entry point for the same version-neutral
/// Candidate V1 record verifier.
///
/// Population successors must not reinterpret the nested Candidate bytes;
/// they all share this one decoder, canonical re-encoder and Base V2 digest
/// domain while their own outer formats remain version-separated.
pub(crate) fn verify_population_v5_canonical_record(
    canonical_record: &[u8; ROW_STRIDE_BYTES],
) -> Result<AuthenticatedCandidatePopulationRowV1, CandidateUniverseRefusal> {
    verify_population_candidate_canonical_record_v1(canonical_record)
}

#[cfg(test)]
pub(crate) use tests::population_v5_test_canonical_candidate_record_for_identity;

/// Crate-internal result of a typed append followed by exact read-only reopen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CandidateUniverseProductionCommitV1 {
    /// New row bytes and then a new receipt were synced.
    Written(CandidateUniverseReopenAuditV1),
    /// Every existing byte matched the exact retry.
    Reused(CandidateUniverseReopenAuditV1),
}

impl CandidateUniverseProductionCommitV1 {
    /// Exact reopened structural proof produced by either append branch.
    #[must_use]
    pub(crate) const fn audit(self) -> CandidateUniverseReopenAuditV1 {
        match self {
            Self::Written(audit) | Self::Reused(audit) => audit,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OrphanBlockV1 {
    universe_id: [u8; 32],
    first_row: u64,
    row_count: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGenerationV1 {
    len: u64,
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

/// Open, indexed candidate-universe ledger.
#[derive(Debug)]
pub struct CandidateUniverseLedgerV1 {
    lock_path: PathBuf,
    row_path: PathBuf,
    receipt_path: PathBuf,
    row_file: File,
    receipt_file: File,
    writer_lock: File,
    bounds: CandidateUniverseBoundsV1,
    audits: HashMap<[u8; 32], CandidateUniverseReopenAuditV1>,
    orphan: Option<OrphanBlockV1>,
    total_rows: u64,
    lock_generation: FileGenerationV1,
    row_generation: FileGenerationV1,
    receipt_generation: FileGenerationV1,
    writable: bool,
}

impl CandidateUniverseLedgerV1 {
    /// Opens or initializes the two fixed-stride files and builds the bounded
    /// receipt index.
    ///
    /// # Errors
    ///
    /// Refuses malformed headers/records, a limit breach, duplicate completion,
    /// committed-row mismatch, more than one orphan identity, I/O error, or
    /// path replacement.
    fn open(
        root: impl AsRef<Path>,
        bounds: CandidateUniverseBoundsV1,
    ) -> Result<Self, CandidateUniverseRefusal> {
        Self::open_inner(root.as_ref(), bounds, true)
    }

    /// Opens an existing ledger without creating or modifying any file.
    ///
    /// # Errors
    ///
    /// The same fail-closed validations as [`Self::open`], plus absence of any
    /// required file.
    pub fn open_read(
        root: impl AsRef<Path>,
        bounds: CandidateUniverseBoundsV1,
    ) -> Result<Self, CandidateUniverseRefusal> {
        Self::open_inner(root.as_ref(), bounds, false)
    }

    fn open_inner(
        root: &Path,
        bounds: CandidateUniverseBoundsV1,
        writable: bool,
    ) -> Result<Self, CandidateUniverseRefusal> {
        let (row_path, receipt_path, lock_path) = candidate_ledger_paths(root)?;
        let writer_lock = open_file(&lock_path, writable, writable)?;
        if writable {
            writer_lock.lock().map_err(|why| {
                format!(
                    "cannot lock candidate writer {}: {why}",
                    lock_path.display()
                )
            })?;
        } else {
            writer_lock.lock_shared().map_err(|why| {
                format!(
                    "cannot take shared candidate lock {}: {why}",
                    lock_path.display()
                )
            })?;
        }
        let ledger_lock = writer_lock.try_clone().map_err(|why| {
            format!(
                "cannot clone candidate writer lock {}: {why}",
                lock_path.display()
            )
        })?;
        let lock_generation = file_generation(&writer_lock, &lock_path)?;
        let opened = (|| {
            let mut row_file = open_file(&row_path, writable, writable)?;
            let mut receipt_file = open_file(&receipt_path, writable, writable)?;
            if writable {
                ensure_header(
                    &mut row_file,
                    ROW_MAGIC,
                    ROW_KIND,
                    CANDIDATE_ROW_STRIDE_V1,
                    &row_path,
                )?;
                ensure_header(
                    &mut receipt_file,
                    RECEIPT_MAGIC,
                    RECEIPT_KIND,
                    CANDIDATE_RECEIPT_STRIDE_V1,
                    &receipt_path,
                )?;
            } else {
                verify_header(
                    &mut row_file,
                    ROW_MAGIC,
                    ROW_KIND,
                    CANDIDATE_ROW_STRIDE_V1,
                    &row_path,
                )?;
                verify_header(
                    &mut receipt_file,
                    RECEIPT_MAGIC,
                    RECEIPT_KIND,
                    CANDIDATE_RECEIPT_STRIDE_V1,
                    &receipt_path,
                )?;
            }
            let row_generation = file_generation(&row_file, &row_path)?;
            let receipt_generation = file_generation(&receipt_file, &receipt_path)?;
            let mut ledger = Self {
                lock_path: lock_path.clone(),
                row_path,
                receipt_path,
                row_file,
                receipt_file,
                writer_lock: ledger_lock,
                bounds,
                audits: HashMap::new(),
                orphan: None,
                total_rows: 0,
                lock_generation,
                row_generation,
                receipt_generation,
                writable,
            };
            ledger.scan()?;
            Ok(ledger)
        })();
        let released = writer_lock.unlock().map_err(|why| {
            format!(
                "cannot release candidate open lock {}: {why}",
                lock_path.display()
            )
        });
        match (opened, released) {
            (Ok(ledger), Ok(())) => Ok(ledger),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn scan(&mut self) -> Result<(), CandidateUniverseRefusal> {
        verify_header(
            &mut self.row_file,
            ROW_MAGIC,
            ROW_KIND,
            CANDIDATE_ROW_STRIDE_V1,
            &self.row_path,
        )?;
        verify_header(
            &mut self.receipt_file,
            RECEIPT_MAGIC,
            RECEIPT_KIND,
            CANDIDATE_RECEIPT_STRIDE_V1,
            &self.receipt_path,
        )?;
        let total_rows = record_count(
            &self.row_file,
            CANDIDATE_ROW_STRIDE_V1,
            self.bounds.max_rows,
            "candidate rows",
        )?;
        let receipt_count = record_count(
            &self.receipt_file,
            CANDIDATE_RECEIPT_STRIDE_V1,
            self.bounds.max_universes,
            "candidate completions",
        )?;
        let capacity = usize::try_from(receipt_count)
            .map_err(|_| "candidate completion count does not fit usize".to_owned())?;
        self.audits = HashMap::new();
        self.audits
            .try_reserve(capacity)
            .map_err(|why| format!("cannot reserve candidate completion audit index: {why}"))?;
        let mut committed = 0_u64;
        for index in 0..receipt_count {
            let receipt = read_receipt(&mut self.receipt_file, index)?;
            let end = committed
                .checked_add(receipt.row_count)
                .ok_or_else(|| "candidate committed row cursor overflowed u64".to_owned())?;
            if end > total_rows {
                return Err(format!(
                    "candidate receipt {} commits rows {committed}..{end}, beyond physical total {total_rows}",
                    hex32(receipt.universe_id())
                ));
            }
            validate_file_block(&mut self.row_file, committed, &receipt)?;
            let audit = CandidateUniverseReopenAuditV1 {
                first_row: committed,
                receipt,
            };
            if self.audits.insert(receipt.universe_id(), audit).is_some() {
                return Err(format!(
                    "candidate universe {} has more than one completion receipt",
                    hex32(receipt.universe_id())
                ));
            }
            committed = end;
        }
        self.orphan = scan_orphan(&mut self.row_file, committed, total_rows)?;
        self.total_rows = total_rows;
        self.row_generation = file_generation(&self.row_file, &self.row_path)?;
        self.receipt_generation = file_generation(&self.receipt_file, &self.receipt_path)?;
        Ok(())
    }

    /// Returns one audit-only reopened receipt after locking and rechecking all
    /// three file generations/path identities.
    ///
    /// # Errors
    ///
    /// Refuses a lock, stale/replaced file or unlock failure. Absence is
    /// returned as `Ok(None)` and cannot be mistaken for an I/O refusal.
    pub fn reopen_audit(
        &self,
        universe_id: &[u8; 32],
    ) -> Result<Option<CandidateUniverseReopenAuditV1>, CandidateUniverseRefusal> {
        self.writer_lock
            .lock_shared()
            .map_err(|why| format!("cannot take shared candidate audit lock: {why}"))?;
        let result = self
            .require_unchanged()
            .map(|()| self.audits.get(universe_id).copied());
        let released = self
            .writer_lock
            .unlock()
            .map_err(|why| format!("cannot release candidate audit lock: {why}"));
        match (result, released) {
            (Ok(audit), Ok(())) => Ok(audit),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// Reads at most 256 exact rows from one completed universe.
    ///
    /// # Errors
    ///
    /// Refuses an unknown universe, an over-limit request, offset arithmetic,
    /// stale/replaced files, corrupt row or row outside the named block.
    pub fn page(
        &self,
        universe_id: &[u8; 32],
        offset: u64,
        limit: u64,
    ) -> Result<CandidateUniversePageV1, CandidateUniverseRefusal> {
        if limit > MAX_CANDIDATE_PAGE_ROWS_V1 {
            return Err(format!(
                "candidate page limit {limit} exceeds {MAX_CANDIDATE_PAGE_ROWS_V1}"
            ));
        }
        self.writer_lock
            .lock_shared()
            .map_err(|why| format!("cannot take shared candidate page lock: {why}"))?;
        let result = (|| {
            self.require_unchanged()?;
            let audit = self.audits.get(universe_id).copied().ok_or_else(|| {
                format!("candidate universe {} is not complete", hex32(*universe_id))
            })?;
            if offset > audit.row_count() {
                return Err(format!(
                    "candidate page offset {offset} exceeds row count {}",
                    audit.row_count()
                ));
            }
            let available = audit.row_count().saturating_sub(offset);
            let count = available.min(limit);
            let mut file = open_file(&self.row_path, false, false)?;
            require_generation(self.row_generation, &file, &self.row_path)?;
            let capacity = usize::try_from(count)
                .map_err(|_| "candidate page count does not fit usize".to_owned())?;
            let mut rows = Vec::new();
            rows.try_reserve(capacity)
                .map_err(|why| format!("cannot reserve candidate page: {why}"))?;
            let start = audit
                .first_row
                .checked_add(offset)
                .ok_or_else(|| "candidate page physical offset overflowed u64".to_owned())?;
            for index in 0..count {
                let physical = start
                    .checked_add(index)
                    .ok_or_else(|| "candidate page row index overflowed u64".to_owned())?;
                let logical = offset
                    .checked_add(index)
                    .ok_or_else(|| "candidate page logical index overflowed u64".to_owned())?;
                let row = read_row(&mut file, physical)?;
                if row.universe_id != *universe_id || row.sequence != logical {
                    return Err("candidate page row escaped its committed block".to_owned());
                }
                rows.push(row);
            }
            Ok(CandidateUniversePageV1 { rows })
        })();
        let released = self
            .writer_lock
            .unlock()
            .map_err(|why| format!("cannot release candidate page lock: {why}"));
        match (result, released) {
            (Ok(page), Ok(())) => Ok(page),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// Recovers the complete opaque Population input family bound to one exact
    /// reopened completion audit.
    ///
    /// This door is crate-private because a structural reopen is not a public
    /// production-authoring capability. It takes one shared ledger lock,
    /// validates every held path generation before and after the complete
    /// positional read, and allocates exactly the audited row count. Each
    /// result retains the literal sealed record and derives its row identity
    /// through Base Evidence V2's single domain/helper. The decoded row must
    /// re-encode byte-for-byte to that literal record. The audit must be
    /// byte-for-byte equal to the completion indexed by this ledger; a caller
    /// cannot substitute a universe ID, row count, record or digest.
    ///
    /// # Errors
    ///
    /// Refuses an unknown or non-identical audit, a configured/allocation
    /// bound, offset overflow, a stale/replaced file, corrupt row bytes, a row
    /// outside the audited universe or sequence, non-canonical ordering, or an
    /// ordered-row digest mismatch. The row family is returned only after all
    /// rows and the trailing generation validation succeed.
    pub(crate) fn complete_population_rows(
        &self,
        audit: &CandidateUniverseReopenAuditV1,
    ) -> Result<Vec<AuthenticatedCandidatePopulationRowV1>, CandidateUniverseRefusal> {
        self.writer_lock
            .lock_shared()
            .map_err(|why| format!("cannot take shared candidate complete-read lock: {why}"))?;
        let result = (|| {
            self.require_unchanged()?;
            let read = (|| {
                let indexed = self
                    .audits
                    .get(&audit.universe_id())
                    .copied()
                    .ok_or_else(|| {
                        format!(
                            "candidate universe {} is not complete",
                            hex32(audit.universe_id())
                        )
                    })?;
                if indexed != *audit {
                    return Err(format!(
                        "candidate universe {} complete-read audit is not the exact indexed completion",
                        hex32(audit.universe_id())
                    ));
                }
                if audit.row_count() > self.bounds.max_rows {
                    return Err(format!(
                        "candidate complete-read has {} rows, above configured maximum {}",
                        audit.row_count(),
                        self.bounds.max_rows
                    ));
                }
                let capacity = usize::try_from(audit.row_count()).map_err(|_| {
                    "candidate complete-read row count does not fit usize".to_owned()
                })?;
                let mut authenticated = Vec::new();
                authenticated
                    .try_reserve_exact(capacity)
                    .map_err(|why| format!("cannot reserve candidate complete-read rows: {why}"))?;
                for sequence in 0..audit.row_count() {
                    let physical = audit.first_row().checked_add(sequence).ok_or_else(|| {
                        "candidate complete-read physical row index overflowed u64".to_owned()
                    })?;
                    let canonical_record = read_row_record_positioned(&self.row_file, physical)?;
                    let row = CandidateUniverseRowV1::decode(&canonical_record)?;
                    row.validate(Some(&audit.receipt.descriptor))?;
                    if row.universe_id() != audit.universe_id() || row.sequence() != sequence {
                        return Err(format!(
                            "candidate complete-read row at sequence {sequence} escaped its audited block"
                        ));
                    }
                    if row.record()? != canonical_record {
                        return Err(format!(
                            "candidate complete-read row at sequence {sequence} does not re-encode to its literal sealed record"
                        ));
                    }
                    authenticated.push(AuthenticatedCandidatePopulationRowV1 {
                        row,
                        base_candidate_row_digest: candidate_row_digest_v2(&canonical_record),
                        canonical_record,
                    });
                }
                let ordered = validate_row_sequence(
                    &audit.receipt.descriptor,
                    audit.receipt.reconciliation,
                    audit.row_count(),
                    |sequence| {
                        let index = usize::try_from(sequence).map_err(|_| {
                            "candidate complete-read validation index does not fit usize".to_owned()
                        })?;
                        authenticated
                            .get(index)
                            .map(AuthenticatedCandidatePopulationRowV1::row)
                            .ok_or_else(|| {
                                format!(
                                    "candidate complete-read validation row {sequence} is absent"
                                )
                            })
                    },
                )?;
                if ordered != audit.receipt.ordered_row_digest {
                    return Err(format!(
                        "candidate universe {} complete-read ordered-row digest does not match its audit",
                        hex32(audit.universe_id())
                    ));
                }
                Ok(authenticated)
            })();
            let trailing_generation = self.require_unchanged();
            match (read, trailing_generation) {
                (Ok(authenticated), Ok(())) => Ok(authenticated),
                (Err(why), _) | (Ok(_), Err(why)) => Err(why),
            }
        })();
        let released = self
            .writer_lock
            .unlock()
            .map_err(|why| format!("cannot release candidate complete-read lock: {why}"));
        match (result, released) {
            (Ok(authenticated), Ok(())) => Ok(authenticated),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// Appends rows, syncs them, then appends and syncs the completion receipt.
    /// Exact retries reuse bytes; every non-identical collision refuses.
    ///
    /// # Errors
    ///
    /// Refuses read-only access, stale files, bounds, changed retry/orphan bytes,
    /// another orphan identity, or any I/O/integrity failure.
    fn append_complete(
        &mut self,
        prepared: &PreparedCandidateUniverseV1,
    ) -> Result<CandidateUniverseProductionCommitV1, CandidateUniverseRefusal> {
        if !self.writable {
            return Err("candidate ledger was opened read-only".to_owned());
        }
        self.writer_lock
            .lock()
            .map_err(|why| format!("cannot take candidate append lock: {why}"))?;
        let result = self.append_complete_locked(prepared);
        let released = self
            .writer_lock
            .unlock()
            .map_err(|why| format!("cannot release candidate append lock: {why}"));
        match (result, released) {
            (Ok(outcome), Ok(())) => Ok(outcome),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn append_complete_locked(
        &mut self,
        prepared: &PreparedCandidateUniverseV1,
    ) -> Result<CandidateUniverseProductionCommitV1, CandidateUniverseRefusal> {
        self.require_unchanged()?;
        let receipt = prepared.receipt;
        receipt.validate()?;
        if receipt.row_count > self.bounds.max_rows {
            return Err(format!(
                "candidate append has {} rows, above ledger maximum {}",
                receipt.row_count, self.bounds.max_rows
            ));
        }
        if let Some(existing) = self.audits.get(&receipt.universe_id()).copied() {
            if existing.receipt != receipt {
                return Err(format!(
                    "candidate universe {} is already complete with different bytes",
                    hex32(receipt.universe_id())
                ));
            }
            compare_rows(
                &mut self.row_file,
                existing.first_row,
                prepared.rows.as_slice(),
            )?;
            return Ok(CandidateUniverseProductionCommitV1::Reused(existing));
        }
        let current_universes = u64::try_from(self.audits.len())
            .map_err(|_| "candidate index size does not fit u64".to_owned())?;
        if current_universes >= self.bounds.max_universes {
            return Err(format!(
                "candidate completion count reached configured maximum {}",
                self.bounds.max_universes
            ));
        }
        let (first_row, prefix) = match self.orphan {
            Some(orphan) => {
                if orphan.universe_id != receipt.universe_id() {
                    return Err(format!(
                        "candidate row tail belongs to {}, not requested {}; no fallback may hide it",
                        hex32(orphan.universe_id),
                        hex32(receipt.universe_id())
                    ));
                }
                if orphan.row_count > receipt.row_count {
                    return Err(format!(
                        "candidate orphan has {} rows, longer than exact retry {}",
                        orphan.row_count, receipt.row_count
                    ));
                }
                let prefix = usize::try_from(orphan.row_count)
                    .map_err(|_| "candidate orphan prefix does not fit usize".to_owned())?;
                compare_rows(
                    &mut self.row_file,
                    orphan.first_row,
                    prepared
                        .rows
                        .get(..prefix)
                        .ok_or_else(|| "candidate retry lost its orphan prefix".to_owned())?,
                )?;
                (orphan.first_row, orphan.row_count)
            }
            None => (self.total_rows, 0),
        };
        let desired_total = first_row
            .checked_add(receipt.row_count)
            .ok_or_else(|| "candidate append row total overflowed u64".to_owned())?;
        if desired_total > self.bounds.max_rows {
            return Err(format!(
                "candidate append would produce {desired_total} rows, above maximum {}",
                self.bounds.max_rows
            ));
        }
        let prefix_usize = usize::try_from(prefix)
            .map_err(|_| "candidate append prefix does not fit usize".to_owned())?;
        append_rows(
            &mut self.row_file,
            prepared
                .rows
                .get(prefix_usize..)
                .ok_or_else(|| "candidate append prefix exceeds prepared rows".to_owned())?,
        )?;
        self.row_file
            .sync_data()
            .map_err(|why| format!("cannot sync candidate rows: {why}"))?;
        self.total_rows = desired_total;
        self.orphan = Some(OrphanBlockV1 {
            universe_id: receipt.universe_id(),
            first_row,
            row_count: receipt.row_count,
        });
        self.row_generation = file_generation(&self.row_file, &self.row_path)?;
        append_receipt(&mut self.receipt_file, &receipt)?;
        self.receipt_file
            .sync_data()
            .map_err(|why| format!("cannot sync candidate completion: {why}"))?;
        let audit = CandidateUniverseReopenAuditV1 { first_row, receipt };
        self.audits.insert(receipt.universe_id(), audit);
        self.orphan = None;
        self.receipt_generation = file_generation(&self.receipt_file, &self.receipt_path)?;
        Ok(CandidateUniverseProductionCommitV1::Written(audit))
    }

    fn require_unchanged(&self) -> Result<(), CandidateUniverseRefusal> {
        require_generation(self.lock_generation, &self.writer_lock, &self.lock_path)?;
        require_generation(self.row_generation, &self.row_file, &self.row_path)?;
        require_generation(
            self.receipt_generation,
            &self.receipt_file,
            &self.receipt_path,
        )
    }
}

fn candidate_ledger_paths(
    root: &Path,
) -> Result<(PathBuf, PathBuf, PathBuf), CandidateUniverseRefusal> {
    let metadata = std::fs::metadata(root).map_err(|why| {
        format!(
            "candidate ledger root {} must already exist and be a directory: {why}",
            root.display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "candidate ledger root {} is not a directory",
            root.display()
        ));
    }
    let admitted_root = root.canonicalize().map_err(|why| {
        format!(
            "candidate ledger root {} cannot be admitted canonically: {why}",
            root.display()
        )
    })?;
    Ok((
        admitted_root.join(ROW_FILE),
        admitted_root.join(RECEIPT_FILE),
        admitted_root.join(LOCK_FILE),
    ))
}

fn append_produced_candidate_universe_v1(
    root: impl AsRef<Path>,
    bounds: CandidateUniverseBoundsV1,
    produced: &ProducedCandidateUniverseV1<'_>,
) -> Result<CandidateUniverseProductionCommitV1, CandidateUniverseRefusal> {
    let root = root.as_ref();
    let mut ledger = CandidateUniverseLedgerV1::open(root, bounds)?;
    let committed = ledger.append_complete(&produced.prepared)?;
    let expected = committed.audit();
    drop(ledger);

    let reopened = CandidateUniverseLedgerV1::open_read(root, bounds)?
        .reopen_audit(&expected.universe_id())?
        .ok_or_else(|| {
            format!(
                "candidate production universe {} disappeared after receipt-last append",
                hex32(expected.universe_id())
            )
        })?;
    if reopened != expected || reopened.receipt() != produced.prepared.receipt {
        return Err(format!(
            "candidate production universe {} did not reopen with the exact prepared semantic bytes",
            hex32(expected.universe_id())
        ));
    }
    Ok(match committed {
        CandidateUniverseProductionCommitV1::Written(_) => {
            CandidateUniverseProductionCommitV1::Written(reopened)
        }
        CandidateUniverseProductionCommitV1::Reused(_) => {
            CandidateUniverseProductionCommitV1::Reused(reopened)
        }
    })
}

fn require_canonical_daily_policy(
    reference: DailyReferenceBinding<'_>,
) -> Result<(), CandidateUniverseRefusal> {
    if reference.schema != crate::stored::DAILY_REFERENCE_SCHEMA {
        return Err(format!(
            "candidate daily-reference schema {} is not canonical schema {}",
            reference.schema,
            crate::stored::DAILY_REFERENCE_SCHEMA
        ));
    }
    if reference.eligibility_policy != crate::stored::DAILY_ELIGIBILITY_POLICY {
        return Err(format!(
            "candidate daily eligibility policy {} is not canonical policy {}",
            reference.eligibility_policy,
            crate::stored::DAILY_ELIGIBILITY_POLICY
        ));
    }
    if reference.gap_overlay_policy != crate::stored::EXACT_MINUTE_GAP_POLICY {
        return Err(format!(
            "candidate exact-minute gap policy {} is not canonical policy {}",
            reference.gap_overlay_policy,
            crate::stored::EXACT_MINUTE_GAP_POLICY
        ));
    }
    if reference.excluded_ist_days != CHARTER_NON_REGULAR_IST_DAYS {
        return Err(
            "candidate excluded-day policy is not the complete ordered charter list".to_owned(),
        );
    }
    if reference.daily_bars.len() != reference.eligibility.len() {
        return Err(format!(
            "candidate daily records {} do not equal eligibility decisions {}",
            reference.daily_bars.len(),
            reference.eligibility.len()
        ));
    }
    for (index, decision) in reference.eligibility.iter().copied().enumerate() {
        if decision > 1 {
            return Err(format!(
                "candidate daily eligibility byte {decision} at {index} is not 0 or 1"
            ));
        }
    }
    Ok(())
}

fn require_canonical_daily_reference(
    references: &[DailyReference],
    binding: DailyReferenceBinding<'_>,
) -> Result<(), CandidateUniverseRefusal> {
    require_canonical_daily_policy(binding)?;
    if references.len() != binding.daily_bars.len() {
        return Err(format!(
            "candidate typed daily references {} do not equal stored daily records {}",
            references.len(),
            binding.daily_bars.len()
        ));
    }
    for (index, ((reference, bar), decision)) in references
        .iter()
        .zip(binding.daily_bars)
        .zip(binding.eligibility)
        .enumerate()
    {
        if reference.bar() != *bar {
            return Err(format!(
                "candidate typed daily reference {index} does not carry the exact stored daily candle"
            ));
        }
        let expected = match *decision {
            0 => DailyEligibility::Excluded,
            1 => DailyEligibility::Eligible,
            byte => {
                return Err(format!(
                    "candidate daily eligibility byte {byte} at {index} is not 0 or 1"
                ));
            }
        };
        if reference.eligibility() != expected {
            return Err(format!(
                "candidate typed daily reference {index} eligibility differs from its ordered stored decision"
            ));
        }
    }
    Ok(())
}

fn require_exact_execution_subspan(
    context: &[Candle],
    execution: &[Candle],
) -> Result<(), CandidateUniverseRefusal> {
    let first = execution
        .first()
        .ok_or_else(|| "candidate evaluated execution subspan is empty".to_owned())?;
    if context.is_empty() {
        return Err("candidate complete minute context is empty".to_owned());
    }
    for (index, pair) in context.windows(2).enumerate() {
        let [left, right] = pair else {
            return Err("candidate minute-context pair shape is impossible".to_owned());
        };
        if left.ts_micros >= right.ts_micros {
            return Err(format!(
                "candidate minute context is not strictly ordered at pair {index}"
            ));
        }
    }
    let start = context
        .iter()
        .position(|bar| bar.ts_micros == first.ts_micros)
        .ok_or_else(|| {
            "candidate evaluated execution first timestamp is absent from minute context".to_owned()
        })?;
    let end = start
        .checked_add(execution.len())
        .ok_or_else(|| "candidate evaluated execution subspan bounds overflow usize".to_owned())?;
    let exact = context.get(start..end).ok_or_else(|| {
        "candidate evaluated execution count extends beyond minute context".to_owned()
    })?;
    if exact.first() != execution.first()
        || exact.last() != execution.last()
        || exact.len() != execution.len()
        || runner::identity::data_digest(exact) != runner::identity::data_digest(execution)
        || exact != execution
    {
        return Err(
            "candidate evaluated execution is not one exact contiguous minute-context subspan"
                .to_owned(),
        );
    }
    Ok(())
}

fn signal_length_micros(rung_seconds: u32) -> Result<i64, CandidateUniverseRefusal> {
    i64::from(rung_seconds)
        .checked_mul(1_000_000)
        .ok_or_else(|| "candidate signal duration overflowed microseconds".to_owned())
}

pub(crate) fn require_exact_calendar(
    name: &str,
    bars: &[Candle],
    rung_seconds: u32,
    coverage: CandidateCalendarCoverageV1,
    offered: CompleteCalendarReceiptV2,
) -> Result<(), CandidateUniverseRefusal> {
    let exact = crate::stored::calendar_receipt_v2_for_bars(
        bars,
        rung_seconds,
        coverage.first_day,
        coverage.last_day,
    )
    .and_then(crate::stored::CalendarReceiptV2::require_complete)
    .map_err(|why| format!("candidate {name} exact complete calendar refused: {why}"))?;
    if exact != offered {
        return Err(format!(
            "candidate {name} complete calendar receipt does not belong to the exact offered bars"
        ));
    }
    Ok(())
}

fn validate_candidate_exit_sources(
    family: InstrumentFamilyV1,
    execution_series: ExecutionSeriesV1<'_>,
    long_exit_grid: &ResolvedExitGridV1,
    short_exit_grid: &ResolvedExitGridV1,
) -> Result<(), CandidateUniverseRefusal> {
    validate_resolved_source("long", family, Side::Long, execution_series, long_exit_grid)?;
    validate_resolved_source(
        "short",
        family,
        Side::Short,
        execution_series,
        short_exit_grid,
    )?;
    if long_exit_grid.policy_digest() == short_exit_grid.policy_digest()
        || long_exit_grid.digest() == short_exit_grid.digest()
    {
        return Err(
            "candidate production long and short grid identities are copied or indistinguishable"
                .to_owned(),
        );
    }
    Ok(())
}

fn validate_resolved_source(
    name: &str,
    family: InstrumentFamilyV1,
    side: Side,
    series: ExecutionSeriesV1<'_>,
    resolved: &ResolvedExitGridV1,
) -> Result<(), CandidateUniverseRefusal> {
    if !resolved.digest_is_valid() {
        return Err(format!(
            "candidate {name} exit-grid resolution digest is torn"
        ));
    }
    let resolved_family = match resolved.family() {
        runner::exit_grid_policy::InstrumentFamilyV1::Nifty => InstrumentFamilyV1::Nifty,
        runner::exit_grid_policy::InstrumentFamilyV1::BankNifty => InstrumentFamilyV1::BankNifty,
    };
    if resolved_family != family
        || resolved.instrument() != *series.instrument()
        || resolved.side() != side
    {
        return Err(format!(
            "candidate {name} exit-grid family, instrument or side is foreign"
        ));
    }
    if resolved.feed_digest() != hash(series.feed().as_bytes())
        || resolved.commit_digest() != hash(series.commit().as_bytes())
        || resolved.calendar_digest() != series.calendar_digest()
    {
        return Err(format!(
            "candidate {name} exit-grid feed, commit or calendar identity is foreign"
        ));
    }
    let (count, first, last, digest) = stream_facts("grid training", series.bars())?;
    if resolved.training_bars() != count
        || resolved.training_first_ts_micros() != first
        || resolved.training_last_ts_micros() != last
        || resolved.training_digest() != digest
    {
        return Err(format!(
            "candidate {name} exit-grid training stream is not the exact execution series"
        ));
    }
    if resolved.cell_count() == 0 {
        return Err(format!(
            "candidate {name} exit-grid resolution contains no complete coordinates"
        ));
    }
    Ok(())
}

fn require_same_evaluation_spec(
    signal: &Column,
    execution: &Column,
) -> Result<(), CandidateUniverseRefusal> {
    let signal_spec = signal
        .evaluation_spec_token()
        .ok_or_else(|| "candidate signal column has no typed evaluation policy".to_owned())?;
    let execution_spec = execution
        .evaluation_spec_token()
        .ok_or_else(|| "candidate execution column has no typed evaluation policy".to_owned())?;
    if signal_spec != execution_spec {
        return Err(
            "candidate signal and execution columns carry different evaluation policies".to_owned(),
        );
    }
    Ok(())
}

fn derive_production_source_id(source: &CandidateUniverseProductionSourceV1<'_>) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(PRODUCTION_SOURCE_ID_DOMAIN);
    hasher.update(&[family_byte(source.family)]);
    hash_u32(&mut hasher, source.rung_seconds);
    hash_u32(&mut hasher, source.horizon.as_bars());
    hasher.update(&source.requested_span.canonical_bytes());
    hasher.update(&source.signal_calendar.digest());
    hasher.update(&source.execution_calendar.digest());
    hasher.update(&source.data_digest);
    hash_signal_stream(&mut hasher, source.signal_stream);
    hash_execution_stream(&mut hasher, source.execution_stream);
    hasher.update(&source.signal_column_digest);
    hasher.update(&source.execution_column_digest);
    if let Some(spec) = source.signal_column.evaluation_spec_token() {
        hasher.update(spec.fingerprint_v1().as_bytes());
    }
    hasher.update(&hash(source.execution_series.feed().as_bytes()));
    hasher.update(&hash(source.execution_series.commit().as_bytes()));
    hasher.update(&source.execution_series.calendar_digest());
    hasher.update(&source.long_exit_grid.policy_digest());
    hasher.update(&source.long_exit_grid.digest());
    hasher.update(&source.short_exit_grid.policy_digest());
    hasher.update(&source.short_exit_grid.digest());
    hash_u64(&mut hasher, source.signal_load_bound.max_records());
    hash_u64(&mut hasher, source.minute_load_bound.max_records());
    hash_u64(&mut hasher, source.daily_load_bound.max_records());
    hasher.update(
        &crate::stored_data_completeness::daily_reference_policy_digest_v1(source.daily_reference),
    );
    hasher.finalize()
}

fn canonical_vocabulary_digest_v1() -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(VOCABULARY_ID_DOMAIN);
    hash_u32(&mut hasher, vocab::VOCAB_VERSION);
    hash_u64(
        &mut hasher,
        u64::try_from(vocab::mask::WORDS).unwrap_or(u64::MAX),
    );
    hash_u64(
        &mut hasher,
        u64::try_from(vocab::table::TABLE.len()).unwrap_or(u64::MAX),
    );
    for definition in &vocab::table::TABLE {
        hasher.update(&definition.index.to_le_bytes());
        hash_framed_bytes(&mut hasher, definition.name.as_bytes());
        hasher.update(&[match definition.kind {
            vocab::Kind::Plain => 0,
            vocab::Kind::Near => 1,
        }]);
        hasher.update(&[match definition.band {
            None => 0,
            Some(vocab::tolerance::Base::SessionRange) => 1,
            Some(vocab::tolerance::Base::CprWidth) => 2,
        }]);
        match definition.status {
            vocab::BitStatus::Live => hasher.update(&[0]),
            vocab::BitStatus::Retired { duplicate_of } => {
                hasher.update(&[1]);
                hasher.update(&duplicate_of.to_le_bytes());
            }
            vocab::BitStatus::Void { reason } => {
                hasher.update(&[2]);
                hash_framed_bytes(&mut hasher, reason.as_bytes());
            }
        }
    }
    hasher.finalize()
}

fn production_identities(
    source: &CandidateUniverseProductionSourceV1<'_>,
    sweeper: &Sweeper,
) -> Result<CandidateUniverseIdentitiesV1, CandidateUniverseRefusal> {
    production_identities_with_ladder(source, sweeper.ladder())
}

fn production_identities_with_ladder(
    source: &CandidateUniverseProductionSourceV1<'_>,
    ladder: engine::Ladder,
) -> Result<CandidateUniverseIdentitiesV1, CandidateUniverseRefusal> {
    let evaluation_spec = source
        .signal_column
        .evaluation_spec_token()
        .ok_or_else(|| "candidate signal column has no evaluation policy identity".to_owned())?;
    let params = Params::of(ladder);
    let mut evaluation = Hasher::new();
    evaluation.update(EVALUATION_POLICY_ID_DOMAIN);
    evaluation.update(evaluation_spec.fingerprint_v1().as_bytes());
    hash_u64(&mut evaluation, params.min_hits);
    hash_u64(&mut evaluation, params.ceiling);
    hash_u64(&mut evaluation, params.pair_budget);
    hash_u64(&mut evaluation, EXECUTION_RUN_POLICY_VERSION_V1);
    hash_u32(&mut evaluation, source.rung_seconds);
    hash_u32(&mut evaluation, source.horizon.as_bars());
    let identities = CandidateUniverseIdentitiesV1 {
        data_digest: source.data_digest,
        feed_digest: hash(source.execution_series.feed().as_bytes()),
        source_commit_digest: hash(source.execution_series.commit().as_bytes()),
        vocabulary_digest: canonical_vocabulary_digest_v1(),
        evaluation_policy_digest: evaluation.finalize(),
        exit_grids: LongShortExitGridIdentitiesV2 {
            long: SideExitGridIdentityV2 {
                policy_digest: source.long_exit_grid.policy_digest(),
                resolved_digest: source.long_exit_grid.digest(),
            },
            short: SideExitGridIdentityV2 {
                policy_digest: source.short_exit_grid.policy_digest(),
                resolved_digest: source.short_exit_grid.digest(),
            },
        },
        calendar_policy_digest: crate::stored::calendar_policy_digest_v2(),
        daily_reference_policy_digest:
            crate::stored_data_completeness::daily_reference_policy_digest_v1(
                source.daily_reference,
            ),
    };
    identities.validate()?;
    Ok(identities)
}

fn execution_run_params(
    sweeper: &Sweeper,
    source: &CandidateUniverseProductionSourceV1<'_>,
) -> Params {
    execution_run_params_with_ladder(sweeper.ladder(), source)
}

fn execution_run_params_with_ladder(
    ladder: engine::Ladder,
    source: &CandidateUniverseProductionSourceV1<'_>,
) -> Params {
    Params::of(ladder).with_policy(&[
        EXECUTION_RUN_POLICY_VERSION_V1,
        u64::from(source.rung_seconds),
        u64::from(source.horizon.as_bars()),
    ])
}

/// Rebuild one complete Candidate block into the exact Runner dispositions
/// needed by Execution V3.
///
/// This is intentionally inside the Candidate module: it reuses the sole
/// production descriptor/run builders and the private typed source. A caller
/// cannot offer a detached mask, column, resolution, disposition or digest.
#[expect(
    clippy::too_many_lines,
    reason = "one fail-closed replay keeps Candidate authentication, exact grid evaluation and terminal classification in their source order"
)]
#[expect(
    clippy::large_types_passed_by_value,
    reason = "the authenticated Candidate receipt is consumed as one capability rather than borrowed as detached facts"
)]
fn build_execution_v3_replay_authority(
    source: &CandidateUniverseProductionSourceV1<'_>,
    ladder: engine::Ladder,
    receipt: CandidateUniverseReceiptV1,
    authenticated: &[AuthenticatedCandidatePopulationRowV1],
) -> Result<CandidateExecutionReplayAuthorityV1, CandidateUniverseRefusal> {
    source.validate()?;
    receipt.validate()?;
    let identities = production_identities_with_ladder(source, ladder)?;
    let descriptor = CandidateUniverseDescriptorV1::new(
        source.family,
        source.rung_seconds,
        source.horizon,
        source.requested_span,
        &identities,
        source.signal_calendar,
        source.execution_calendar,
        source.signal_bars,
        &source.signal_column,
        source.execution_series,
        &source.execution_column,
    )?;
    if receipt.descriptor != descriptor {
        return Err(
            "Candidate Execution V3 replay source does not reproduce the committed descriptor"
                .to_owned(),
        );
    }
    let expected_len = usize::try_from(receipt.row_count())
        .map_err(|_| "Candidate Execution V3 row count does not fit usize".to_owned())?;
    if authenticated.len() != expected_len {
        return Err(format!(
            "Candidate Execution V3 replay received {} authenticated rows, expected {expected_len}",
            authenticated.len()
        ));
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(authenticated.len()).map_err(|why| {
        format!(
            "Candidate Execution V3 could not reserve {} decoded rows: {why}",
            authenticated.len()
        )
    })?;
    for row in authenticated {
        let decoded = row.row();
        if decoded.record()? != *row.canonical_record() {
            return Err(
                "Candidate Execution V3 authenticated row no longer reproduces its sealed bytes"
                    .to_owned(),
            );
        }
        rows.push(decoded);
    }
    validate_rows(&descriptor, receipt.reconciliation(), &rows)?;
    if digest_ordered_rows(&descriptor, &rows)? != receipt.ordered_row_digest() {
        return Err(
            "Candidate Execution V3 ordered rows do not reproduce the Completion digest".to_owned(),
        );
    }

    let params = execution_run_params_with_ladder(ladder, source);
    let evaluation_fingerprint = source
        .execution_column
        .evaluation_spec_token()
        .ok_or_else(|| {
            "Candidate Execution V3 exact execution column has no evaluation policy".to_owned()
        })?
        .fingerprint_v1()
        .into_bytes();
    let long_parameter = execution_parameter_facts(
        source,
        TradeDirectionV1::Long,
        source.long_exit_grid,
        params,
        evaluation_fingerprint,
    )?;
    let short_parameter = execution_parameter_facts(
        source,
        TradeDirectionV1::Short,
        source.short_exit_grid,
        params,
        evaluation_fingerprint,
    )?;
    if long_parameter.instrument_digest != short_parameter.instrument_digest {
        return Err(
            "Candidate Execution V3 Long and Short grids name different full instruments"
                .to_owned(),
        );
    }

    let long_width = usize::try_from(receipt.counts.long_cells_per_mask)
        .map_err(|_| "Candidate Execution V3 Long grid width does not fit usize".to_owned())?;
    let short_width = usize::try_from(receipt.counts.short_cells_per_mask)
        .map_err(|_| "Candidate Execution V3 Short grid width does not fit usize".to_owned())?;
    let group_width = long_width
        .checked_add(short_width)
        .ok_or_else(|| "Candidate Execution V3 mask-group width overflowed usize".to_owned())?;
    if group_width == 0 || rows.len() % group_width != 0 {
        return Err(
            "Candidate Execution V3 rows do not end on a complete Long/Short mask group".to_owned(),
        );
    }
    let mut dispositions = Vec::new();
    dispositions
        .try_reserve_exact(authenticated.len())
        .map_err(|why| {
            format!(
                "Candidate Execution V3 could not reserve {} dispositions: {why}",
                authenticated.len()
            )
        })?;
    let mut group_start = 0_usize;
    while group_start < rows.len() {
        replay_execution_side(
            source,
            params,
            &rows,
            authenticated,
            group_start,
            long_width,
            TradeDirectionV1::Long,
            Direction::Long,
            source.long_exit_grid,
            &mut dispositions,
        )?;
        let short_start = group_start
            .checked_add(long_width)
            .ok_or_else(|| "Candidate Execution V3 Short group start overflowed".to_owned())?;
        replay_execution_side(
            source,
            params,
            &rows,
            authenticated,
            short_start,
            short_width,
            TradeDirectionV1::Short,
            Direction::Short,
            source.short_exit_grid,
            &mut dispositions,
        )?;
        group_start = group_start
            .checked_add(group_width)
            .ok_or_else(|| "Candidate Execution V3 group cursor overflowed".to_owned())?;
    }
    if dispositions.len() != authenticated.len() {
        return Err(
            "Candidate Execution V3 terminal disposition count changed during replay".to_owned(),
        );
    }
    source.validate()?;
    Ok(CandidateExecutionReplayAuthorityV1 {
        receipt,
        parameters: [long_parameter, short_parameter],
        dispositions,
    })
}

fn execution_parameter_facts(
    source: &CandidateUniverseProductionSourceV1<'_>,
    direction: TradeDirectionV1,
    resolved: &ResolvedExitGridV1,
    params: Params,
    evaluation_fingerprint: [u8; indicators::column::EVALUATION_SPEC_FINGERPRINT_V1_LEN],
) -> Result<CandidateExecutionParameterFactsV1, CandidateUniverseRefusal> {
    if resolved.policy().execution_resolution() != ExecutionResolutionV1::OneMinuteOhlcv {
        return Err("Candidate Execution V3 source is not exact one-minute OHLCV".to_owned());
    }
    let expected_side = match direction {
        TradeDirectionV1::Long => Side::Long,
        TradeDirectionV1::Short => Side::Short,
    };
    if resolved.side() != expected_side {
        return Err("Candidate Execution V3 direction and resolved side disagree".to_owned());
    }
    let policy = resolved.policy();
    let forced_stop_ppm = match policy.forced_stop() {
        ForcedStopV1::Disabled => 0,
        ForcedStopV1::IncludeExactObserved(ppm) | ForcedStopV1::RequireExactObserved(ppm) => ppm,
    };
    let forced_stop_index = resolved
        .forced_stop_index()
        .map(|index| {
            u32::try_from(index)
                .map_err(|_| "Candidate Execution V3 forced-stop index does not fit u32".to_owned())
        })
        .transpose()?;
    let max_levels = u64::try_from(policy.rungs().max_levels_per_axis())
        .map_err(|_| "Candidate Execution V3 rung bound does not fit u64".to_owned())?;
    let resolved_stop_count = u64::try_from(resolved.stop_levels_ppm().len())
        .map_err(|_| "Candidate Execution V3 stop count does not fit u64".to_owned())?;
    let resolved_target_count = u64::try_from(resolved.target_levels_ppm().len())
        .map_err(|_| "Candidate Execution V3 target count does not fit u64".to_owned())?;
    let resolved_trail_count = u64::try_from(resolved.trail_levels_ppm().len())
        .map_err(|_| "Candidate Execution V3 trail count does not fit u64".to_owned())?;
    let resolved_ratio_pair_count = u64::try_from(resolved.ratio_pairs().len())
        .map_err(|_| "Candidate Execution V3 ratio-pair count does not fit u64".to_owned())?;
    let instrument = source.execution_series.instrument();
    if resolved.instrument() != *instrument {
        return Err("Candidate Execution V3 resolution names a foreign instrument".to_owned());
    }
    Ok(CandidateExecutionParameterFactsV1 {
        family: source.family,
        direction,
        rung_seconds: source.rung_seconds,
        horizon_bars: source.horizon.as_bars(),
        policy_digest: resolved.policy_digest(),
        resolution_digest: resolved.digest(),
        training_digest: resolved.training_digest(),
        instrument_digest: instrument_digest_v1(instrument),
        feed_digest: resolved.feed_digest(),
        commit_digest: resolved.commit_digest(),
        calendar_digest: resolved.calendar_digest(),
        cost_model_id: policy.cost_model_id(),
        evaluation_fingerprint,
        range_resolution: policy.range_resolution(),
        selector: policy.selector(),
        forced_stop: policy.forced_stop(),
        forced_stop_index,
        run_params: [
            params.min_hits,
            params.ceiling,
            params.pair_budget,
            params.policy,
        ],
        max_levels,
        ratio_min_hundredths: policy.ratios().min_hundredths(),
        ratio_max_hundredths: policy.ratios().max_hundredths(),
        max_ratio_pairs: policy.ratios().max_pairs(),
        policy_max_cells: policy.max_cells(),
        resolved_stop_count,
        resolved_target_count,
        resolved_trail_count,
        resolved_ratio_pair_count,
        resolved_cell_count: resolved.cell_count(),
        forced_stop_ppm,
        max_ambiguity_bars: policy.max_ambiguous_bars(),
        max_gap_bars: policy.max_gap_fills(),
        training_bars: resolved.training_bars(),
        training_first_ts_micros: resolved.training_first_ts_micros(),
        training_last_ts_micros: resolved.training_last_ts_micros(),
        stop_percentiles: try_clone_percentiles("Stop", policy.rungs().stop())?,
        target_percentiles: try_clone_percentiles("Target", policy.rungs().target())?,
        trail_percentiles: try_clone_percentiles("Trail", policy.rungs().trail())?,
    })
}

fn try_clone_percentiles(
    axis: &str,
    source: &[RationalPercentileV1],
) -> Result<Vec<RationalPercentileV1>, CandidateUniverseRefusal> {
    let mut values = Vec::new();
    values.try_reserve_exact(source.len()).map_err(|why| {
        format!(
            "Candidate Execution V3 could not reserve {} {axis} percentiles: {why}",
            source.len()
        )
    })?;
    values.extend_from_slice(source);
    Ok(values)
}

#[expect(
    clippy::too_many_arguments,
    reason = "one exact directional replay keeps the authenticated row slice, side resolution and append target explicit"
)]
#[expect(
    clippy::too_many_lines,
    reason = "one directional fold keeps row identity, exact evaluation and disposition append in a single fail-closed sequence"
)]
fn replay_execution_side(
    source: &CandidateUniverseProductionSourceV1<'_>,
    params: Params,
    rows: &[CandidateUniverseRowV1],
    authenticated: &[AuthenticatedCandidatePopulationRowV1],
    start: usize,
    width: usize,
    direction: TradeDirectionV1,
    run_direction: Direction,
    resolved: &ResolvedExitGridV1,
    output: &mut Vec<CandidateExecutionDispositionProjectionV1>,
) -> Result<(), CandidateUniverseRefusal> {
    if width == 0 {
        return Err("Candidate Execution V3 directional grid is empty".to_owned());
    }
    let end = start
        .checked_add(width)
        .ok_or_else(|| "Candidate Execution V3 directional range overflowed".to_owned())?;
    let group = rows.get(start..end).ok_or_else(|| {
        "Candidate Execution V3 directional range exceeds authenticated rows".to_owned()
    })?;
    let first = group
        .first()
        .ok_or_else(|| "Candidate Execution V3 directional group is absent".to_owned())?;
    if first.direction() != direction {
        return Err("Candidate Execution V3 directional row order changed".to_owned());
    }
    let mask = runner::replay_mask::from_stored_words(first.mask_words())
        .map_err(|why| format!("Candidate Execution V3 stored mask refused: {why}"))?;
    let run = Run {
        mask,
        direction: run_direction,
        instrument: source.execution_series.instrument(),
        timeframe: rung_label(source.rung_seconds)?,
        params,
        data_digest: source.data_digest,
        commit: source.execution_series.commit(),
        feed: source.execution_series.feed(),
    };
    let execution_run = ExecutionRunV1::new_with_daily_reference(
        &run,
        source.signal_bars,
        source.reference_minute_context,
        source.execution_series.bars(),
        source.daily_reference,
    )
    .map_err(|why| format!("Candidate Execution V3 exact run refused: {why:?}"))?;
    let evaluated = resolved
        .evaluate_training_grid_attested(
            source.execution_series,
            &source.execution_column,
            source.horizon,
            execution_run,
        )
        .map_err(|why| format!("Candidate Execution V3 complete grid refused: {why:?}"))?;
    let validated = resolved
        .validate_evaluation(&evaluated)
        .map_err(|why| format!("Candidate Execution V3 grid integrity refused: {why:?}"))?;
    if evaluated.run_id().bytes() != first.execution_run_id()
        || evaluated.mask() != mask
        || evaluated.horizon() != source.horizon
        || evaluated.side() != resolved.side()
        || evaluated.column_digest() != source.execution_column_digest
        || validated.resolution_digest() != resolved.digest()
        || validated.evaluation_digest() != first.evaluated_grid_digest()
    {
        return Err(
            "Candidate Execution V3 evaluated grid does not reproduce Candidate identities"
                .to_owned(),
        );
    }
    for (within, row) in group.iter().enumerate() {
        let expected_ordinal = u64::try_from(within)
            .map_err(|_| "Candidate Execution V3 ordinal does not fit u64".to_owned())?;
        if row.direction() != direction
            || row.mask_words() != first.mask_words()
            || row.execution_run_id() != evaluated.run_id().bytes()
            || row.evaluated_grid_digest() != validated.evaluation_digest()
            || row.cell_ordinal() != expected_ordinal
        {
            return Err("Candidate Execution V3 row escaped its exact directional grid".to_owned());
        }
        let measured = row.facts().to_cell(row.exit())?;
        let exact = validated.cell(within).ok_or_else(|| {
            format!("Candidate Execution V3 validated grid lost ordinal {within}")
        })?;
        if measured != *exact {
            return Err(format!(
                "Candidate Execution V3 measured cell differs at ordinal {within}"
            ));
        }
        let disposition = resolved
            .classify_coordinate(&validated, Chosen::from_cell(exact))
            .map_err(|why| {
                format!(
                    "Candidate Execution V3 terminal classification refused ordinal {within}: {why:?}"
                )
            })?;
        if disposition.run_id().bytes() != row.execution_run_id()
            || runner::replay_mask::stored_words(&disposition.mask()) != row.mask_words()
            || disposition.horizon().as_bars() != row.horizon_bars()
            || disposition.column_digest() != source.execution_column_digest
            || disposition.resolution_digest() != resolved.digest()
            || disposition.side() != resolved.side()
            || disposition.coordinate() != Chosen::from_cell(exact)
        {
            return Err(format!(
                "Candidate Execution V3 disposition identity differs at ordinal {within}"
            ));
        }
        let source_index = start
            .checked_add(within)
            .ok_or_else(|| "Candidate Execution V3 source index overflowed".to_owned())?;
        let candidate = authenticated.get(source_index).ok_or_else(|| {
            "Candidate Execution V3 authenticated source row is absent".to_owned()
        })?;
        output.push(CandidateExecutionDispositionProjectionV1 {
            candidate: candidate.clone(),
            disposition,
        });
    }
    Ok(())
}

fn rung_label(rung_seconds: u32) -> Result<&'static str, CandidateUniverseRefusal> {
    match rung_seconds {
        60 => Ok("1min"),
        120 => Ok("2min"),
        180 => Ok("3min"),
        300 => Ok("5min"),
        600 => Ok("10min"),
        900 => Ok("15min"),
        1_800 => Ok("30min"),
        3_600 => Ok("60min"),
        _ => Err(format!(
            "candidate execution rung {rung_seconds} has no canonical timeframe label"
        )),
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "both complete side resolutions and all canonical execution-run identity sources remain explicit"
)]
fn expand_population_member(
    member: PopulationMember,
    descriptor: &CandidateUniverseDescriptorV1,
    params: Params,
    signal_bars: &[Candle],
    reference_minute_context: &[Candle],
    daily_reference: DailyReferenceBinding<'_>,
    execution_series: ExecutionSeriesV1<'_>,
    execution_column: &Column,
    long_exit_grid: &ResolvedExitGridV1,
    short_exit_grid: &ResolvedExitGridV1,
    horizon: Horizon,
    rung_seconds: u32,
    bounds: CandidateUniverseBoundsV1,
    rows: &mut Vec<CandidateUniverseRowV1>,
    observations: &mut CandidateObservationBuilderV1,
    support_signal_column: &Column,
    base_bounds: BaseEvidenceBoundsV2,
    base_builder: &mut BaseEvidenceBuilderV2,
) -> Result<(), CandidateUniverseRefusal> {
    match member.closure {
        ClosureVerdict::Redundant => Ok(()),
        ClosureVerdict::Unknown => Err(format!(
            "candidate mask {:?} reached retirement without a terminal closure verdict",
            member.item.mask.words()
        )),
        ClosureVerdict::Closed => {
            let support = measure_mask_support_v2(
                runner::replay_mask::stored_words(&member.item.mask),
                member.item.hits,
                signal_bars,
                support_signal_column,
                base_bounds,
            )
            .map_err(|why| why.to_string())?;
            expand_population_side(
                member,
                descriptor,
                TradeDirectionV1::Long,
                Direction::Long,
                params,
                signal_bars,
                reference_minute_context,
                daily_reference,
                execution_series,
                execution_column,
                long_exit_grid,
                horizon,
                rung_seconds,
                bounds,
                rows,
                observations,
                support,
                base_builder,
            )?;
            expand_population_side(
                member,
                descriptor,
                TradeDirectionV1::Short,
                Direction::Short,
                params,
                signal_bars,
                reference_minute_context,
                daily_reference,
                execution_series,
                execution_column,
                short_exit_grid,
                horizon,
                rung_seconds,
                bounds,
                rows,
                observations,
                support,
                base_builder,
            )
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "one private side expansion seals every execution identity term before evaluating its complete grid"
)]
fn expand_population_side(
    member: PopulationMember,
    descriptor: &CandidateUniverseDescriptorV1,
    trade_direction: TradeDirectionV1,
    run_direction: Direction,
    params: Params,
    signal_bars: &[Candle],
    reference_minute_context: &[Candle],
    daily_reference: DailyReferenceBinding<'_>,
    execution_series: ExecutionSeriesV1<'_>,
    execution_column: &Column,
    resolved: &ResolvedExitGridV1,
    horizon: Horizon,
    rung_seconds: u32,
    bounds: CandidateUniverseBoundsV1,
    rows: &mut Vec<CandidateUniverseRowV1>,
    observations: &mut CandidateObservationBuilderV1,
    support: MaskSupportEvidenceV2,
    base_builder: &mut BaseEvidenceBuilderV2,
) -> Result<(), CandidateUniverseRefusal> {
    let current = u64::try_from(rows.len())
        .map_err(|_| "candidate row prefix does not fit u64".to_owned())?;
    let desired = current
        .checked_add(resolved.cell_count())
        .ok_or_else(|| "candidate closed-mask grid expansion overflowed u64".to_owned())?;
    if desired > bounds.max_rows {
        return Err(format!(
            "candidate closed-mask expansion would require {desired} rows, above configured preparation bound {}",
            bounds.max_rows
        ));
    }
    let additional = usize::try_from(resolved.cell_count())
        .map_err(|_| "candidate resolved grid width does not fit usize".to_owned())?;
    rows.try_reserve_exact(additional).map_err(|why| {
        format!(
            "candidate could not reserve {additional} rows for one complete directional grid: {why}"
        )
    })?;

    let run = Run {
        mask: member.item.mask,
        direction: run_direction,
        instrument: execution_series.instrument(),
        timeframe: rung_label(rung_seconds)?,
        params,
        data_digest: descriptor.identities.data_digest,
        commit: execution_series.commit(),
        feed: execution_series.feed(),
    };
    let execution_run = ExecutionRunV1::new_with_daily_reference(
        &run,
        signal_bars,
        reference_minute_context,
        execution_series.bars(),
        daily_reference,
    )
    .map_err(|why| {
        format!(
            "candidate {:?} execution run refused mask {:?}: {why:?}",
            trade_direction,
            member.item.mask.words()
        )
    })?;
    let evaluated = resolved
        .evaluate_training_grid_attested(execution_series, execution_column, horizon, execution_run)
        .map_err(|why| {
            format!(
                "candidate {:?} complete grid evaluation refused mask {:?}: {why:?}",
                trade_direction,
                member.item.mask.words()
            )
        })?;
    if evaluated.mask() != member.item.mask
        || evaluated.side() != resolved.side()
        || evaluated.horizon() != horizon
        || evaluated.column_digest() != descriptor.execution_column_digest
        || evaluated.resolution_digest() != resolved.digest()
    {
        return Err(format!(
            "candidate {trade_direction:?} evaluated grid belongs to foreign mask, side, horizon, column or resolution"
        ));
    }
    let evaluated_count = u64::try_from(evaluated.grid().cells.len())
        .map_err(|_| "candidate evaluated grid width does not fit u64".to_owned())?;
    if evaluated_count != resolved.cell_count() {
        return Err(format!(
            "candidate {trade_direction:?} evaluated {evaluated_count} cells, not resolved width {}",
            resolved.cell_count()
        ));
    }
    let validated = resolved.validate_evaluation(&evaluated).map_err(|why| {
        format!("candidate {trade_direction:?} complete grid integrity refused: {why:?}")
    })?;
    append_validated_grid_rows(
        member,
        descriptor,
        trade_direction,
        &evaluated,
        &validated,
        execution_series.bars(),
        execution_column,
        rows,
        observations,
        support,
        base_builder,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "exact Candidate row and observation replay keep source, evaluated authority and both append targets explicit"
)]
fn append_validated_grid_rows(
    member: PopulationMember,
    descriptor: &CandidateUniverseDescriptorV1,
    direction: TradeDirectionV1,
    evaluated: &runner::exit_grid_policy::EvaluatedExitGridV1,
    validated: &ValidatedExitGridV1<'_>,
    execution_bars: &[Candle],
    execution_column: &Column,
    rows: &mut Vec<CandidateUniverseRowV1>,
    observations: &mut CandidateObservationBuilderV1,
    support: MaskSupportEvidenceV2,
    base_builder: &mut BaseEvidenceBuilderV2,
) -> Result<(), CandidateUniverseRefusal> {
    for ordinal in 0..evaluated.grid().cells.len() {
        let cell = validated.cell(ordinal).ok_or_else(|| {
            format!(
                "candidate validated grid lost canonical cell ordinal {ordinal} of {}",
                evaluated.grid().cells.len()
            )
        })?;
        let sequence = u64::try_from(rows.len())
            .map_err(|_| "candidate produced sequence does not fit u64".to_owned())?;
        let cell_ordinal = u64::try_from(ordinal)
            .map_err(|_| "candidate cell ordinal does not fit u64".to_owned())?;
        let mut row = CandidateUniverseRowV1 {
            universe_id: descriptor.universe_id,
            sequence,
            candidate_semantic_digest: [0; 32],
            mask_words: runner::replay_mask::stored_words(&member.item.mask),
            direction,
            family: descriptor.family,
            rung_seconds: descriptor.rung_seconds,
            support_hits: member.item.hits,
            exit: exit_coordinate_from_cell(cell)?,
            execution_run_id: evaluated.run_id().bytes(),
            horizon_bars: descriptor.horizon_bars,
            evaluated_grid_digest: validated.evaluation_digest(),
            cell_ordinal,
            facts: CandidateDirectCellFactsV1::from_cell(cell),
        };
        row.candidate_semantic_digest = candidate_semantic_digest(descriptor, &row);
        row.validate(Some(descriptor))?;
        let trades = materialize_cell(
            execution_bars,
            execution_column,
            &member.item.mask,
            evaluated.horizon(),
            evaluated.side(),
            evaluated.grid(),
            cell,
        )
        .map_err(|why| {
            format!("candidate {direction:?} cell {ordinal} exact trade replay refused: {why}")
        })?;
        base_builder
            .observe_candidate(&row, support, cell, &trades)
            .map_err(|why| why.to_string())?;
        observations.observe_candidate(
            row.candidate_semantic_digest,
            cell,
            execution_bars,
            &trades,
        )?;
        rows.push(row);
    }
    Ok(())
}

fn exit_coordinate_from_cell(cell: &Cell) -> Result<ExitCoordinateV1, CandidateUniverseRefusal> {
    Ok(ExitCoordinateV1 {
        stop: optional_exit_index("stop", cell.stop)?,
        target: optional_exit_index("target", cell.target)?,
        tsl: optional_exit_index("TSL", cell.tsl)?,
        ttp: cell
            .ttp
            .map(|ttp| {
                Ok::<(u32, u32), CandidateUniverseRefusal>((
                    exit_index("TTP arm", ttp.arm)?,
                    exit_index("TTP trail", ttp.trail)?,
                ))
            })
            .transpose()?,
    })
}

fn optional_exit_index(
    name: &str,
    value: Option<usize>,
) -> Result<Option<u32>, CandidateUniverseRefusal> {
    value.map(|index| exit_index(name, index)).transpose()
}

fn exit_index(name: &str, value: usize) -> Result<u32, CandidateUniverseRefusal> {
    let index = u32::try_from(value)
        .map_err(|_| format!("candidate exit {name} index {value} does not fit u32"))?;
    if index == NONE_U32 {
        return Err(format!(
            "candidate exit {name} index u32::MAX is reserved for None"
        ));
    }
    Ok(index)
}

fn source_grid_counts(
    long: &ResolvedExitGridV1,
    short: &ResolvedExitGridV1,
) -> Result<ExitCellsPerMaskV2, CandidateUniverseRefusal> {
    ExitCellsPerMaskV2::new(long.cell_count(), short.cell_count())
        .map_err(|why| format!("candidate resolved grid counts refused: {why}"))
}

fn reconcile_candidate_population(
    run: &PopulationRun,
    exit_cells_per_mask: &ExitCellsPerMaskV2,
    row_count: usize,
) -> Result<CompletionReconciliationV2, CandidateUniverseRefusal> {
    if !run.is_complete() {
        return Err(format!(
            "candidate population is incomplete: closure_complete={}, considered={}, redundant={}, closed={}, halted={:?}",
            run.outcome.closure_complete,
            run.considered,
            run.redundant,
            run.closed,
            run.outcome.sweep.halted
        ));
    }
    let classified = run
        .redundant
        .checked_add(run.closed)
        .ok_or_else(|| "candidate closed plus redundant count overflowed u64".to_owned())?;
    let unknown_closure_itemsets = run
        .considered
        .checked_sub(classified)
        .ok_or_else(|| "candidate closure counts exceed frequent itemsets".to_owned())?;
    if unknown_closure_itemsets != 0 {
        return Err(format!(
            "candidate population retained {unknown_closure_itemsets} unknown closure verdict(s)"
        ));
    }
    let infrequent_itemsets = run
        .outcome
        .trials
        .checked_sub(run.considered)
        .ok_or_else(|| "candidate frequent itemsets exceed sweep trials".to_owned())?;
    let cells_per_mask = exit_cells_per_mask
        .long()
        .checked_add(exit_cells_per_mask.short())
        .ok_or_else(|| "candidate long plus short grid width overflowed u64".to_owned())?;
    let expected_rows = run
        .closed
        .checked_mul(cells_per_mask)
        .ok_or_else(|| "candidate closed masks times grid width overflowed u64".to_owned())?;
    let actual_rows = u64::try_from(row_count)
        .map_err(|_| "candidate produced row count does not fit u64".to_owned())?;
    if actual_rows != expected_rows {
        return Err(format!(
            "candidate production emitted {actual_rows} rows, not closed masks {} times complete long+short width {cells_per_mask} = {expected_rows}",
            run.closed
        ));
    }
    let extinction_depth = u32::try_from(run.outcome.sweep.depth())
        .map_err(|_| "candidate extinction depth does not fit u32".to_owned())?;
    Ok(CompletionReconciliationV2 {
        sweep_trials: run.outcome.trials,
        frequent_itemsets: run.considered,
        infrequent_itemsets,
        closed_itemsets: run.closed,
        redundant_itemsets: run.redundant,
        unknown_closure_itemsets,
        exit_cells_per_mask: *exit_cells_per_mask,
        extinction_depth,
        extinction_complete: run.outcome.sweep.completed(),
        closure_complete: run.outcome.closure_complete,
    })
}

fn hash_framed_bytes(hasher: &mut Hasher, value: &[u8]) {
    hash_u64(hasher, u64::try_from(value.len()).unwrap_or(u64::MAX));
    hasher.update(value);
}

fn expected_counts(
    reconciliation: CompletionReconciliationV2,
) -> Result<CandidateCountsV1, CandidateUniverseRefusal> {
    let frequent = reconciliation
        .closed_itemsets
        .checked_add(reconciliation.redundant_itemsets)
        .and_then(|value| value.checked_add(reconciliation.unknown_closure_itemsets))
        .ok_or_else(|| "candidate frequent-itemset reconciliation overflowed u64".to_owned())?;
    if frequent != reconciliation.frequent_itemsets {
        return Err(format!(
            "candidate frequent itemsets {} do not equal closed+redundant+unknown {frequent}",
            reconciliation.frequent_itemsets
        ));
    }
    let sweep_trials = reconciliation
        .frequent_itemsets
        .checked_add(reconciliation.infrequent_itemsets)
        .ok_or_else(|| "candidate sweep-trial reconciliation overflowed u64".to_owned())?;
    if sweep_trials != reconciliation.sweep_trials {
        return Err(format!(
            "candidate sweep trials {} do not equal frequent+infrequent {sweep_trials}",
            reconciliation.sweep_trials
        ));
    }
    if !reconciliation.extinction_complete || !reconciliation.closure_complete {
        return Err(format!(
            "candidate completion requires natural extinction and terminal closure; extinction={}, closure={}",
            reconciliation.extinction_complete, reconciliation.closure_complete
        ));
    }
    if reconciliation.extinction_depth == 0 && reconciliation.frequent_itemsets != 0 {
        return Err("candidate zero nonempty depth carries frequent itemsets".to_owned());
    }
    let long_cells_per_mask = reconciliation.exit_cells_per_mask.long();
    let short_cells_per_mask = reconciliation.exit_cells_per_mask.short();
    let directions_expected = reconciliation
        .closed_itemsets
        .checked_mul(2)
        .ok_or_else(|| "candidate direction count overflowed u64".to_owned())?;
    let long_cells_evaluated = reconciliation
        .closed_itemsets
        .checked_mul(long_cells_per_mask)
        .ok_or_else(|| "candidate long-cell count overflowed u64".to_owned())?;
    let short_cells_evaluated = reconciliation
        .closed_itemsets
        .checked_mul(short_cells_per_mask)
        .ok_or_else(|| "candidate short-cell count overflowed u64".to_owned())?;
    let exit_cells_expected = long_cells_evaluated
        .checked_add(short_cells_evaluated)
        .ok_or_else(|| "candidate exit-cell count overflowed u64".to_owned())?;
    Ok(CandidateCountsV1 {
        directions_expected,
        directions_evaluated: directions_expected,
        long_cells_per_mask,
        short_cells_per_mask,
        long_cells_evaluated,
        short_cells_evaluated,
        exit_cells_expected,
        exit_cells_evaluated: exit_cells_expected,
    })
}

fn validate_rows(
    descriptor: &CandidateUniverseDescriptorV1,
    reconciliation: CompletionReconciliationV2,
    rows: &[CandidateUniverseRowV1],
) -> Result<(), CandidateUniverseRefusal> {
    let row_count = u64::try_from(rows.len())
        .map_err(|_| "candidate validation row count does not fit u64".to_owned())?;
    validate_row_sequence(descriptor, reconciliation, row_count, |index| {
        let index = usize::try_from(index)
            .map_err(|_| "candidate validation index does not fit usize".to_owned())?;
        rows.get(index)
            .copied()
            .ok_or_else(|| format!("candidate validation row {index} is absent"))
    })
    .map(|_| ())
}

fn validate_file_block(
    file: &mut File,
    first_row: u64,
    receipt: &CandidateUniverseReceiptV1,
) -> Result<(), CandidateUniverseRefusal> {
    let digest = validate_row_sequence(
        &receipt.descriptor,
        receipt.reconciliation,
        receipt.row_count,
        |index| {
            let physical = first_row
                .checked_add(index)
                .ok_or_else(|| "candidate physical row index overflowed u64".to_owned())?;
            read_row(file, physical)
        },
    )?;
    if digest != receipt.ordered_row_digest {
        return Err(format!(
            "candidate universe {} ordered-row digest does not match committed bytes",
            hex32(receipt.universe_id())
        ));
    }
    Ok(())
}

fn validate_row_sequence<F>(
    descriptor: &CandidateUniverseDescriptorV1,
    reconciliation: CompletionReconciliationV2,
    row_count: u64,
    mut row_at: F,
) -> Result<[u8; 32], CandidateUniverseRefusal>
where
    F: FnMut(u64) -> Result<CandidateUniverseRowV1, CandidateUniverseRefusal>,
{
    descriptor.validate()?;
    let counts = expected_counts(reconciliation)?;
    if row_count != counts.exit_cells_expected {
        return Err(format!(
            "candidate sequence has {row_count} rows, expected {}",
            counts.exit_cells_expected
        ));
    }
    let group_width = counts
        .long_cells_per_mask
        .checked_add(counts.short_cells_per_mask)
        .ok_or_else(|| "candidate per-mask group width overflowed u64".to_owned())?;
    let mut digest = Hasher::new();
    digest.update(ORDERED_ROWS_DOMAIN);
    digest.update(&ROW_VERSION.to_le_bytes());
    digest.update(&descriptor.universe_id);
    digest.update(&row_count.to_le_bytes());
    let mut prior_mask_key: Option<(u32, [u64; MASK_WORDS_V1])> = None;
    let mut group_start = 0_u64;
    while group_start < row_count {
        let first = row_at(group_start)?;
        let mask_key = (mask_popcount(first.mask_words), first.mask_words);
        if prior_mask_key.is_some_and(|prior| mask_key <= prior) {
            return Err(format!(
                "candidate mask group at sequence {group_start} is not in strict depth/word order"
            ));
        }
        prior_mask_key = Some(mask_key);
        validate_mask_group(
            descriptor,
            &counts,
            group_start,
            group_width,
            &first,
            &mut row_at,
            &mut digest,
        )?;
        group_start = group_start
            .checked_add(group_width)
            .ok_or_else(|| "candidate group cursor overflowed u64".to_owned())?;
    }
    if group_start != row_count {
        return Err("candidate rows end inside a long/short grid group".to_owned());
    }
    Ok(digest.finalize())
}

fn validate_mask_group<F>(
    descriptor: &CandidateUniverseDescriptorV1,
    counts: &CandidateCountsV1,
    group_start: u64,
    group_width: u64,
    first: &CandidateUniverseRowV1,
    row_at: &mut F,
    digest: &mut Hasher,
) -> Result<(), CandidateUniverseRefusal>
where
    F: FnMut(u64) -> Result<CandidateUniverseRowV1, CandidateUniverseRefusal>,
{
    let support = first.support_hits;
    let mut long_run = None;
    let mut long_grid = None;
    let mut short_run = None;
    let mut short_grid = None;
    let mut prior_coordinate = None;
    for within in 0..group_width {
        let sequence = group_start
            .checked_add(within)
            .ok_or_else(|| "candidate sequence index overflowed u64".to_owned())?;
        let row = if within == 0 {
            *first
        } else {
            row_at(sequence)?
        };
        row.validate(Some(descriptor))?;
        if row.sequence != sequence {
            return Err(format!(
                "candidate row sequence {} appears at canonical position {sequence}",
                row.sequence
            ));
        }
        if row.mask_words != first.mask_words || row.support_hits != support {
            return Err(format!(
                "candidate mask/support changes inside group beginning {group_start}"
            ));
        }
        let (expected_direction, expected_ordinal) = if within < counts.long_cells_per_mask {
            (TradeDirectionV1::Long, within)
        } else {
            (TradeDirectionV1::Short, within - counts.long_cells_per_mask)
        };
        if row.direction != expected_direction || row.cell_ordinal != expected_ordinal {
            return Err(format!(
                "candidate row {sequence} has direction {:?}/ordinal {}, expected {:?}/{expected_ordinal}",
                row.direction, row.cell_ordinal, expected_direction
            ));
        }
        if within == counts.long_cells_per_mask {
            prior_coordinate = None;
        }
        let coordinate_key = canonical_coordinate_key(row.exit);
        if prior_coordinate.is_some_and(|prior| coordinate_key <= prior) {
            return Err(format!(
                "candidate row {sequence} exit coordinate is duplicated or reordered inside its {:?} grid",
                row.direction
            ));
        }
        prior_coordinate = Some(coordinate_key);
        match row.direction {
            TradeDirectionV1::Long => {
                require_same_or_set(&mut long_run, row.execution_run_id, "long run")?;
                require_same_or_set(&mut long_grid, row.evaluated_grid_digest, "long evaluation")?;
            }
            TradeDirectionV1::Short => {
                require_same_or_set(&mut short_run, row.execution_run_id, "short run")?;
                require_same_or_set(
                    &mut short_grid,
                    row.evaluated_grid_digest,
                    "short evaluation",
                )?;
            }
        }
        digest.update(&row.payload()?);
    }
    if long_run == short_run || long_grid == short_grid {
        return Err(format!(
            "candidate mask group {group_start} copied one run/evaluation into both directions"
        ));
    }
    Ok(())
}

/// A constant-state ordering key for the exact order emitted by
/// `ResolvedExitGridV1::visit_coordinates`.
///
/// Each axis visits all present rung indexes first and `None` last. Within one
/// `(stop, target, tsl)` coordinate the no-TTP cell precedes every `(arm,
/// trail)` pair, whose two indexes are nested in ascending order. Ratio-pair
/// filtering can omit coordinates but cannot change that strict order.
const fn canonical_coordinate_key(exit: ExitCoordinateV1) -> (u32, u32, u32, u8, u32, u32) {
    let stop = match exit.stop {
        Some(index) => index,
        None => NONE_U32,
    };
    let target = match exit.target {
        Some(index) => index,
        None => NONE_U32,
    };
    let tsl = match exit.tsl {
        Some(index) => index,
        None => NONE_U32,
    };
    let (ttp_tag, arm, trail) = match exit.ttp {
        None => (0, 0, 0),
        Some((arm, trail)) => (1, arm, trail),
    };
    (stop, target, tsl, ttp_tag, arm, trail)
}

fn require_same_or_set(
    slot: &mut Option<[u8; 32]>,
    value: [u8; 32],
    name: &str,
) -> Result<(), CandidateUniverseRefusal> {
    match *slot {
        Some(expected) if expected != value => Err(format!(
            "candidate {name} changes inside one mask/direction grid"
        )),
        Some(_) => Ok(()),
        None => {
            *slot = Some(value);
            Ok(())
        }
    }
}

fn digest_ordered_rows(
    descriptor: &CandidateUniverseDescriptorV1,
    rows: &[CandidateUniverseRowV1],
) -> Result<[u8; 32], CandidateUniverseRefusal> {
    let row_count = u64::try_from(rows.len())
        .map_err(|_| "candidate ordered-row count does not fit u64".to_owned())?;
    let mut hasher = Hasher::new();
    hasher.update(ORDERED_ROWS_DOMAIN);
    hasher.update(&ROW_VERSION.to_le_bytes());
    hasher.update(&descriptor.universe_id);
    hasher.update(&row_count.to_le_bytes());
    for row in rows {
        hasher.update(&row.payload()?);
    }
    Ok(hasher.finalize())
}

fn derive_universe_id(descriptor: &CandidateUniverseDescriptorV1) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(UNIVERSE_ID_DOMAIN);
    hash_u32(&mut hasher, RECEIPT_VERSION);
    hasher.update(&[family_byte(descriptor.family)]);
    hash_u32(&mut hasher, descriptor.rung_seconds);
    hash_u32(&mut hasher, descriptor.horizon_bars);
    hasher.update(&descriptor.requested_span.canonical_bytes());
    hash_identities(&mut hasher, &descriptor.identities);
    hasher.update(&descriptor.calendar_coverage.encode());
    hash_signal_stream(&mut hasher, descriptor.signal_stream);
    hash_execution_stream(&mut hasher, descriptor.execution_stream);
    hasher.update(&descriptor.signal_column_digest);
    hasher.update(&descriptor.execution_column_digest);
    hasher.finalize()
}

fn candidate_semantic_digest(
    descriptor: &CandidateUniverseDescriptorV1,
    row: &CandidateUniverseRowV1,
) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(CANDIDATE_ID_DOMAIN);
    hash_u32(&mut hasher, ROW_VERSION);
    hasher.update(&descriptor.universe_id);
    hasher.update(&[family_byte(row.family)]);
    hash_u32(&mut hasher, row.rung_seconds);
    hasher.update(&descriptor.identities.evaluation_policy_digest);
    hasher.update(&row.execution_run_id);
    for word in row.mask_words {
        hash_u64(&mut hasher, word);
    }
    hasher.update(&[direction_byte(row.direction)]);
    hash_u32(&mut hasher, row.horizon_bars);
    let side = match row.direction {
        TradeDirectionV1::Long => descriptor.identities.exit_grids.long,
        TradeDirectionV1::Short => descriptor.identities.exit_grids.short,
    };
    hasher.update(&side.policy_digest);
    hasher.update(&side.resolved_digest);
    hash_exit(&mut hasher, row.exit);
    hasher.finalize()
}

fn hash_identities(hasher: &mut Hasher, identities: &CandidateUniverseIdentitiesV1) {
    for digest in [
        identities.data_digest,
        identities.feed_digest,
        identities.source_commit_digest,
        identities.vocabulary_digest,
        identities.evaluation_policy_digest,
        identities.exit_grids.long.policy_digest,
        identities.exit_grids.long.resolved_digest,
        identities.exit_grids.short.policy_digest,
        identities.exit_grids.short.resolved_digest,
        identities.calendar_policy_digest,
        identities.daily_reference_policy_digest,
    ] {
        hasher.update(&digest);
    }
}

fn hash_signal_stream(hasher: &mut Hasher, stream: CandidateSignalStreamV1) {
    hash_u64(hasher, stream.count);
    hash_i64(hasher, stream.first_ts_micros);
    hash_i64(hasher, stream.last_ts_micros);
    hasher.update(&stream.digest);
}

fn hash_execution_stream(hasher: &mut Hasher, stream: CandidateExecutionStreamV1) {
    hash_u64(hasher, stream.count);
    hash_i64(hasher, stream.first_ts_micros);
    hash_i64(hasher, stream.last_ts_micros);
    hasher.update(&stream.digest);
}

fn hash_exit(hasher: &mut Hasher, exit: ExitCoordinateV1) {
    hash_u32(hasher, encode_optional_index(exit.stop));
    hash_u32(hasher, encode_optional_index(exit.target));
    hash_u32(hasher, encode_optional_index(exit.tsl));
    let (arm, trail) = exit.ttp.unwrap_or((NONE_U32, NONE_U32));
    hash_u32(hasher, arm);
    hash_u32(hasher, trail);
}

fn stream_facts(
    name: &str,
    bars: &[Candle],
) -> Result<(u64, i64, i64, [u8; 32]), CandidateUniverseRefusal> {
    let first = bars.first().ok_or_else(|| {
        format!("candidate {name} stream is empty; exact requested-span bytes are required")
    })?;
    let last = bars.last().ok_or_else(|| {
        format!("candidate {name} stream lost its last bar after the non-empty check")
    })?;
    let count = u64::try_from(bars.len())
        .map_err(|_| format!("candidate {name}-bar count does not fit u64"))?;
    let digest = runner::identity::data_digest(bars);
    validate_stream_facts(name, count, first.ts_micros, last.ts_micros, digest)?;
    Ok((count, first.ts_micros, last.ts_micros, digest))
}

fn validate_stream_facts(
    name: &str,
    count: u64,
    first: i64,
    last: i64,
    digest: [u8; 32],
) -> Result<(), CandidateUniverseRefusal> {
    if count == 0 {
        return Err(format!("candidate {name} stream count is zero"));
    }
    if first > last {
        return Err(format!(
            "candidate {name} stream runs backward: {first} is after {last}"
        ));
    }
    require_nonzero_digest(&format!("candidate {name} stream"), digest)
}

fn require_column_sources(
    name: &str,
    column: &Column,
    bars_len: usize,
) -> Result<(), CandidateUniverseRefusal> {
    if column.bits().len() != column.sources().len() {
        return Err(format!(
            "candidate {name} column has {} masks but {} sources",
            column.bits().len(),
            column.sources().len()
        ));
    }
    let mut prior = None;
    for &source in column.sources() {
        if source >= bars_len {
            return Err(format!(
                "candidate {name} column source {source} is outside {bars_len} bars"
            ));
        }
        if prior.is_some_and(|previous| source <= previous) {
            return Err(format!(
                "candidate {name} column sources are not strictly increasing at {source}"
            ));
        }
        prior = Some(source);
    }
    Ok(())
}

fn require_series_family(
    family: InstrumentFamilyV1,
    series: ExecutionSeriesV1<'_>,
) -> Result<(), CandidateUniverseRefusal> {
    let instrument = series.instrument();
    instrument.require_sweepable().map_err(|why| {
        format!("candidate execution instrument is not one of the two swept indices: {why}")
    })?;
    let expected = match family {
        InstrumentFamilyV1::Nifty => "NIFTY",
        InstrumentFamilyV1::BankNifty => "BANKNIFTY",
    };
    if instrument.underlying.as_str() != expected {
        return Err(format!(
            "candidate family {family:?} does not match execution instrument {instrument}"
        ));
    }
    Ok(())
}

fn validate_exit(exit: ExitCoordinateV1) -> Result<(), CandidateUniverseRefusal> {
    for (name, value) in [
        ("stop", exit.stop),
        ("target", exit.target),
        ("TSL", exit.tsl),
    ] {
        if value == Some(NONE_U32) {
            return Err(format!(
                "candidate {name} index u32::MAX is reserved for None"
            ));
        }
    }
    if let Some((arm, trail)) = exit.ttp {
        if arm == NONE_U32 || trail == NONE_U32 {
            return Err("candidate TTP uses the reserved None sentinel".to_owned());
        }
        if exit.target.is_some_and(|target| arm >= target) {
            return Err(format!(
                "candidate TTP arm {arm} is not below target {}",
                exit.target.unwrap_or(NONE_U32)
            ));
        }
        if exit.tsl.is_some_and(|tsl| trail >= tsl) {
            return Err(format!(
                "candidate TTP trail {trail} is not below TSL {}",
                exit.tsl.unwrap_or(NONE_U32)
            ));
        }
    }
    Ok(())
}

pub(crate) const CANDIDATE_SIGNAL_RUNGS_SECONDS_V1: [u32; 8] =
    [60, 120, 180, 300, 600, 900, 1_800, 3_600];

fn require_rung(rung_seconds: u32) -> Result<(), CandidateUniverseRefusal> {
    if CANDIDATE_SIGNAL_RUNGS_SECONDS_V1.contains(&rung_seconds) {
        Ok(())
    } else {
        Err(format!(
            "candidate rung {rung_seconds} is not one of 60,120,180,300,600,900,1800,3600"
        ))
    }
}

fn require_load_bound(
    name: &str,
    records: usize,
    bound: StoredSpanLoadBoundV1,
) -> Result<(), CandidateUniverseRefusal> {
    let records = u64::try_from(records)
        .map_err(|_| format!("candidate {name} record count does not fit u64"))?;
    if records > bound.max_records() {
        return Err(format!(
            "candidate {name} count {records} exceeds exact stored-load ceiling {}",
            bound.max_records()
        ));
    }
    Ok(())
}

pub(crate) fn requested_span_days(
    requested_span: RequestedSpanIdentityV1,
) -> Result<(i64, i64), CandidateUniverseRefusal> {
    let first = i64::from(
        Day::new(requested_span.from_year(), requested_span.from_month(), 1)
            .map_err(|why| format!("candidate requested-span start day is invalid: {why}"))?
            .days_from_epoch(),
    );
    let last = i64::from(
        Day::new(requested_span.to_year(), requested_span.to_month(), 1)
            .map_err(|why| format!("candidate requested-span end month is invalid: {why}"))?
            .end_of_month()
            .days_from_epoch(),
    );
    Ok((first, last))
}

fn require_nonzero_digest(name: &str, digest: [u8; 32]) -> Result<(), CandidateUniverseRefusal> {
    if digest == [0; 32] {
        Err(format!("{name} digest is all zero"))
    } else {
        Ok(())
    }
}

const fn family_byte(family: InstrumentFamilyV1) -> u8 {
    match family {
        InstrumentFamilyV1::Nifty => 1,
        InstrumentFamilyV1::BankNifty => 2,
    }
}

fn family_from_byte(value: u8) -> Result<InstrumentFamilyV1, CandidateUniverseRefusal> {
    match value {
        1 => Ok(InstrumentFamilyV1::Nifty),
        2 => Ok(InstrumentFamilyV1::BankNifty),
        _ => Err(format!(
            "candidate instrument-family byte {value} is unknown"
        )),
    }
}

const fn direction_byte(direction: TradeDirectionV1) -> u8 {
    match direction {
        TradeDirectionV1::Long => 1,
        TradeDirectionV1::Short => 2,
    }
}

fn direction_from_byte(value: u8) -> Result<TradeDirectionV1, CandidateUniverseRefusal> {
    match value {
        1 => Ok(TradeDirectionV1::Long),
        2 => Ok(TradeDirectionV1::Short),
        _ => Err(format!("candidate direction byte {value} is unknown")),
    }
}

const fn closure_byte(closure: ClosureV1) -> u8 {
    match closure {
        ClosureV1::Closed => 1,
        ClosureV1::Redundant => 2,
        ClosureV1::Unknown => 3,
    }
}

const fn encode_optional_index(value: Option<u32>) -> u32 {
    match value {
        Some(value) => value,
        None => NONE_U32,
    }
}

const fn decode_optional_index(value: u32) -> Option<u32> {
    if value == NONE_U32 { None } else { Some(value) }
}

fn decode_bool(value: u8, name: &str) -> Result<bool, CandidateUniverseRefusal> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(format!("{name} boolean byte {value} is not 0 or 1")),
    }
}

fn mask_popcount(words: [u64; MASK_WORDS_V1]) -> u32 {
    words.iter().map(|word| word.count_ones()).sum()
}

fn encode_facts(facts: CandidateDirectCellFactsV1, raw: &mut [u8; ROW_PAYLOAD_BYTES]) {
    for (offset, value) in [
        (248, facts.trades),
        (256, facts.wins),
        (288, facts.stopped),
        (296, facts.trailed_stop),
        (304, facts.trailed_profit),
        (312, facts.targeted),
        (320, facts.timed_out),
        (328, facts.ambiguous_bars),
        (336, facts.gapped),
        (408, facts.bars_held),
    ] {
        put_u64(raw, offset, value);
    }
    for (offset, value) in [
        (264, facts.pessimistic),
        (272, facts.optimistic),
        (280, facts.fill_cost),
        (344, facts.winner_mae),
        (352, facts.winner_mfe),
        (360, facts.all_mae),
        (368, facts.worst_mae),
        (376, facts.gross_win),
        (384, facts.gross_loss),
        (392, facts.best_trade),
        (400, facts.min_win),
        (424, facts.worst_trade),
        (432, facts.max_drawdown),
    ] {
        put_i64(raw, offset, value);
    }
    put_u32(raw, 416, facts.max_losing_streak);
    put_u32(raw, 420, facts.max_winning_streak);
}

fn decode_facts(raw: &[u8]) -> Result<CandidateDirectCellFactsV1, CandidateUniverseRefusal> {
    let facts = CandidateDirectCellFactsV1 {
        trades: get_u64(raw, 248)?,
        wins: get_u64(raw, 256)?,
        pessimistic: get_i64(raw, 264)?,
        optimistic: get_i64(raw, 272)?,
        fill_cost: get_i64(raw, 280)?,
        stopped: get_u64(raw, 288)?,
        trailed_stop: get_u64(raw, 296)?,
        trailed_profit: get_u64(raw, 304)?,
        targeted: get_u64(raw, 312)?,
        timed_out: get_u64(raw, 320)?,
        ambiguous_bars: get_u64(raw, 328)?,
        gapped: get_u64(raw, 336)?,
        winner_mae: get_i64(raw, 344)?,
        winner_mfe: get_i64(raw, 352)?,
        all_mae: get_i64(raw, 360)?,
        worst_mae: get_i64(raw, 368)?,
        gross_win: get_i64(raw, 376)?,
        gross_loss: get_i64(raw, 384)?,
        best_trade: get_i64(raw, 392)?,
        min_win: get_i64(raw, 400)?,
        bars_held: get_u64(raw, 408)?,
        max_losing_streak: get_u32(raw, 416)?,
        max_winning_streak: get_u32(raw, 420)?,
        worst_trade: get_i64(raw, 424)?,
        max_drawdown: get_i64(raw, 432)?,
    };
    facts.validate()?;
    Ok(facts)
}

fn encode_signal_stream(stream: CandidateSignalStreamV1, raw: &mut [u8], offset: usize) {
    put_u64(raw, offset, stream.count);
    put_i64(raw, offset + 8, stream.first_ts_micros);
    put_i64(raw, offset + 16, stream.last_ts_micros);
    put_bytes(raw, offset + 24, &stream.digest);
}

fn encode_execution_stream(stream: CandidateExecutionStreamV1, raw: &mut [u8], offset: usize) {
    put_u64(raw, offset, stream.count);
    put_i64(raw, offset + 8, stream.first_ts_micros);
    put_i64(raw, offset + 16, stream.last_ts_micros);
    put_bytes(raw, offset + 24, &stream.digest);
}

fn decode_signal_stream(
    raw: &[u8],
    offset: usize,
) -> Result<CandidateSignalStreamV1, CandidateUniverseRefusal> {
    let stream = CandidateSignalStreamV1 {
        count: get_u64(raw, offset)?,
        first_ts_micros: get_i64(raw, offset + 8)?,
        last_ts_micros: get_i64(raw, offset + 16)?,
        digest: get_32(raw, offset + 24)?,
    };
    stream.validate()?;
    Ok(stream)
}

fn decode_execution_stream(
    raw: &[u8],
    offset: usize,
) -> Result<CandidateExecutionStreamV1, CandidateUniverseRefusal> {
    let stream = CandidateExecutionStreamV1 {
        count: get_u64(raw, offset)?,
        first_ts_micros: get_i64(raw, offset + 8)?,
        last_ts_micros: get_i64(raw, offset + 16)?,
        digest: get_32(raw, offset + 24)?,
    };
    stream.validate()?;
    Ok(stream)
}

fn decode_span(raw: &[u8]) -> Result<RequestedSpanIdentityV1, CandidateUniverseRefusal> {
    if raw.len() != SPAN_BYTES {
        return Err(format!(
            "candidate requested span is {} bytes, not {SPAN_BYTES}",
            raw.len()
        ));
    }
    let version = get_u32(raw, 0)?;
    if version != 1 {
        return Err(format!(
            "candidate requested-span version {version} is unknown"
        ));
    }
    let from_year = u16::try_from(get_u32(raw, 4)?)
        .map_err(|_| "candidate requested-span from year does not fit u16".to_owned())?;
    let from_month = u8::try_from(get_u32(raw, 8)?)
        .map_err(|_| "candidate requested-span from month does not fit u8".to_owned())?;
    let to_year = u16::try_from(get_u32(raw, 12)?)
        .map_err(|_| "candidate requested-span to year does not fit u16".to_owned())?;
    let to_month = u8::try_from(get_u32(raw, 16)?)
        .map_err(|_| "candidate requested-span to month does not fit u8".to_owned())?;
    RequestedSpanIdentityV1::new(from_year, from_month, to_year, to_month)
        .map_err(|why| format!("candidate requested span refused: {why}"))
}

fn digest_domain(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    hasher.update(bytes);
    hasher.finalize()
}

fn hash_u32(hasher: &mut Hasher, value: u32) {
    hasher.update(&value.to_le_bytes());
}

fn hash_u64(hasher: &mut Hasher, value: u64) {
    hasher.update(&value.to_le_bytes());
}

fn hash_i64(hasher: &mut Hasher, value: i64) {
    hasher.update(&value.to_le_bytes());
}

fn put_bytes(raw: &mut [u8], offset: usize, bytes: &[u8]) {
    let end = offset.saturating_add(bytes.len());
    assert!(
        end <= raw.len(),
        "candidate internal codec offsets are compile-time fixed"
    );
    if let Some(target) = raw.get_mut(offset..end) {
        target.copy_from_slice(bytes);
    }
}

fn put_u32(raw: &mut [u8], offset: usize, value: u32) {
    put_bytes(raw, offset, &value.to_le_bytes());
}

fn put_u64(raw: &mut [u8], offset: usize, value: u64) {
    put_bytes(raw, offset, &value.to_le_bytes());
}

fn put_i64(raw: &mut [u8], offset: usize, value: i64) {
    put_bytes(raw, offset, &value.to_le_bytes());
}

fn get_u32(raw: &[u8], offset: usize) -> Result<u32, CandidateUniverseRefusal> {
    let bytes = raw
        .get(offset..offset.saturating_add(4))
        .ok_or_else(|| format!("candidate codec lacks u32 at offset {offset}"))?;
    let mut value = [0_u8; 4];
    value.copy_from_slice(bytes);
    Ok(u32::from_le_bytes(value))
}

fn get_byte(raw: &[u8], offset: usize, name: &str) -> Result<u8, CandidateUniverseRefusal> {
    raw.get(offset)
        .copied()
        .ok_or_else(|| format!("{name} is absent at offset {offset}"))
}

fn get_u64(raw: &[u8], offset: usize) -> Result<u64, CandidateUniverseRefusal> {
    let bytes = raw
        .get(offset..offset.saturating_add(8))
        .ok_or_else(|| format!("candidate codec lacks u64 at offset {offset}"))?;
    let mut value = [0_u8; 8];
    value.copy_from_slice(bytes);
    Ok(u64::from_le_bytes(value))
}

fn get_i64(raw: &[u8], offset: usize) -> Result<i64, CandidateUniverseRefusal> {
    let bytes = raw
        .get(offset..offset.saturating_add(8))
        .ok_or_else(|| format!("candidate codec lacks i64 at offset {offset}"))?;
    let mut value = [0_u8; 8];
    value.copy_from_slice(bytes);
    Ok(i64::from_le_bytes(value))
}

fn get_32(raw: &[u8], offset: usize) -> Result<[u8; 32], CandidateUniverseRefusal> {
    let bytes = raw
        .get(offset..offset.saturating_add(32))
        .ok_or_else(|| format!("candidate codec lacks digest at offset {offset}"))?;
    let mut value = [0_u8; 32];
    value.copy_from_slice(bytes);
    Ok(value)
}

fn require_zero(
    raw: &[u8],
    offset: usize,
    len: usize,
    name: &str,
) -> Result<(), CandidateUniverseRefusal> {
    let bytes = raw
        .get(offset..offset.saturating_add(len))
        .ok_or_else(|| format!("{name} is outside candidate record"))?;
    if bytes.iter().any(|byte| *byte != 0) {
        Err(format!("{name} is not zero"))
    } else {
        Ok(())
    }
}

fn require_seal(
    name: &str,
    offered: &[u8],
    expected: [u8; 32],
) -> Result<(), CandidateUniverseRefusal> {
    if offered == expected {
        Ok(())
    } else {
        Err(format!("{name} BLAKE3 seal does not match its payload"))
    }
}

fn header_bytes(magic: [u8; 16], kind: u32, stride: u64) -> [u8; HEADER_BYTES] {
    let mut raw = [0_u8; HEADER_BYTES];
    put_bytes(&mut raw, 0, &magic);
    put_u32(&mut raw, 16, HEADER_VERSION);
    put_u32(&mut raw, 20, kind);
    put_u64(&mut raw, 24, stride);
    let seal = digest_domain(HEADER_DOMAIN, &raw[..32]);
    put_bytes(&mut raw, 32, &seal);
    raw
}

fn ensure_header(
    file: &mut File,
    magic: [u8; 16],
    kind: u32,
    stride: u64,
    path: &Path,
) -> Result<(), CandidateUniverseRefusal> {
    let len = file
        .metadata()
        .map_err(|why| format!("cannot stat candidate file {}: {why}", path.display()))?
        .len();
    if len == 0 {
        file.seek(SeekFrom::Start(0))
            .and_then(|_| file.write_all(&header_bytes(magic, kind, stride)))
            .and_then(|()| file.sync_data())
            .map_err(|why| format!("cannot initialize candidate file {}: {why}", path.display()))?;
    }
    verify_header(file, magic, kind, stride, path)
}

fn verify_header(
    file: &mut File,
    magic: [u8; 16],
    kind: u32,
    stride: u64,
    path: &Path,
) -> Result<(), CandidateUniverseRefusal> {
    let len = file
        .metadata()
        .map_err(|why| format!("cannot stat candidate file {}: {why}", path.display()))?
        .len();
    if len < HEADER_BYTES_V1 {
        return Err(format!(
            "candidate file {} is {len} bytes, shorter than header {HEADER_BYTES_V1}",
            path.display()
        ));
    }
    let mut raw = [0_u8; HEADER_BYTES];
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| format!("cannot read candidate header {}: {why}", path.display()))?;
    if raw[..16] != magic {
        return Err(format!("candidate file {} has wrong magic", path.display()));
    }
    if get_u32(&raw, 16)? != HEADER_VERSION
        || get_u32(&raw, 20)? != kind
        || get_u64(&raw, 24)? != stride
    {
        return Err(format!(
            "candidate file {} header version/kind/stride is incompatible",
            path.display()
        ));
    }
    require_seal(
        "candidate file header",
        &raw[32..64],
        digest_domain(HEADER_DOMAIN, &raw[..32]),
    )
}

fn record_count(
    file: &File,
    stride: u64,
    maximum: u64,
    name: &str,
) -> Result<u64, CandidateUniverseRefusal> {
    let len = file
        .metadata()
        .map_err(|why| format!("cannot stat {name}: {why}"))?
        .len();
    let payload = len
        .checked_sub(HEADER_BYTES_V1)
        .ok_or_else(|| format!("{name} is shorter than its header"))?;
    if !payload.is_multiple_of(stride) {
        return Err(format!(
            "{name} payload {payload} is ragged for fixed stride {stride}"
        ));
    }
    let count = payload / stride;
    if count > maximum {
        return Err(format!(
            "{name} contains {count} records, above configured maximum {maximum}"
        ));
    }
    Ok(count)
}

fn record_offset(index: u64, stride: u64) -> Result<u64, CandidateUniverseRefusal> {
    index
        .checked_mul(stride)
        .and_then(|bytes| HEADER_BYTES_V1.checked_add(bytes))
        .ok_or_else(|| "candidate record offset overflowed u64".to_owned())
}

fn read_row(
    file: &mut File,
    index: u64,
) -> Result<CandidateUniverseRowV1, CandidateUniverseRefusal> {
    CandidateUniverseRowV1::decode(&read_row_record(file, index)?)
}

fn read_row_record(
    file: &mut File,
    index: u64,
) -> Result<[u8; ROW_STRIDE_BYTES], CandidateUniverseRefusal> {
    let mut raw = [0_u8; ROW_STRIDE_BYTES];
    file.seek(SeekFrom::Start(record_offset(
        index,
        CANDIDATE_ROW_STRIDE_V1,
    )?))
    .and_then(|_| file.read_exact(&mut raw))
    .map_err(|why| format!("cannot read candidate row {index}: {why}"))?;
    Ok(raw)
}

#[cfg(unix)]
fn read_row_record_positioned(
    file: &File,
    index: u64,
) -> Result<[u8; ROW_STRIDE_BYTES], CandidateUniverseRefusal> {
    let mut raw = [0_u8; ROW_STRIDE_BYTES];
    let offset = record_offset(index, CANDIDATE_ROW_STRIDE_V1)?;
    file.read_exact_at(&mut raw, offset)
        .map_err(|why| format!("cannot position-read candidate row {index}: {why}"))?;
    Ok(raw)
}

#[cfg(not(unix))]
fn read_row_record_positioned(
    file: &File,
    index: u64,
) -> Result<[u8; ROW_STRIDE_BYTES], CandidateUniverseRefusal> {
    let mut cloned = file
        .try_clone()
        .map_err(|why| format!("cannot clone candidate row file: {why}"))?;
    read_row_record(&mut cloned, index)
}

fn read_receipt(
    file: &mut File,
    index: u64,
) -> Result<CandidateUniverseReceiptV1, CandidateUniverseRefusal> {
    let mut raw = [0_u8; RECEIPT_STRIDE_BYTES];
    file.seek(SeekFrom::Start(record_offset(
        index,
        CANDIDATE_RECEIPT_STRIDE_V1,
    )?))
    .and_then(|_| file.read_exact(&mut raw))
    .map_err(|why| format!("cannot read candidate receipt {index}: {why}"))?;
    CandidateUniverseReceiptV1::decode(&raw)
}

fn compare_rows(
    file: &mut File,
    first: u64,
    expected: &[CandidateUniverseRowV1],
) -> Result<(), CandidateUniverseRefusal> {
    for (index, expected_row) in expected.iter().copied().enumerate() {
        let index = u64::try_from(index)
            .map_err(|_| "candidate comparison index does not fit u64".to_owned())?;
        let physical = first
            .checked_add(index)
            .ok_or_else(|| "candidate comparison row index overflowed u64".to_owned())?;
        let actual = read_row(file, physical)?;
        if actual != expected_row {
            return Err(format!(
                "candidate exact retry differs at logical row {index}"
            ));
        }
    }
    Ok(())
}

fn append_rows(
    file: &mut File,
    rows: &[CandidateUniverseRowV1],
) -> Result<(), CandidateUniverseRefusal> {
    let original = file
        .metadata()
        .map_err(|why| format!("cannot stat candidate row file before append: {why}"))?
        .len();
    file.seek(SeekFrom::End(0))
        .map_err(|why| format!("cannot seek candidate row append: {why}"))?;
    let result = (|| {
        let mut chunk = [0_u8; ROW_WRITE_CHUNK_BYTES];
        for group in rows.chunks(ROW_WRITE_CHUNK_ROWS) {
            for (index, row) in group.iter().copied().enumerate() {
                let start = index * ROW_STRIDE_BYTES;
                let end = start + ROW_STRIDE_BYTES;
                chunk
                    .get_mut(start..end)
                    .ok_or_else(|| "candidate row chunk bounds are invalid".to_owned())?
                    .copy_from_slice(&row.record()?);
            }
            let written = group
                .len()
                .checked_mul(ROW_STRIDE_BYTES)
                .ok_or_else(|| "candidate row chunk length overflowed usize".to_owned())?;
            file.write_all(
                chunk
                    .get(..written)
                    .ok_or_else(|| "candidate written row chunk bounds are invalid".to_owned())?,
            )
            .map_err(|why| format!("cannot append candidate rows: {why}"))?;
        }
        Ok(())
    })();
    match result {
        Ok(()) => Ok(()),
        Err(write_why) => match file.set_len(original) {
            Ok(()) => Err(write_why),
            Err(rollback_why) => Err(format!(
                "{write_why}; candidate row rollback to {original} bytes also failed: {rollback_why}; file state is poisoned"
            )),
        },
    }
}

fn append_receipt(
    file: &mut File,
    receipt: &CandidateUniverseReceiptV1,
) -> Result<(), CandidateUniverseRefusal> {
    let original = file
        .metadata()
        .map_err(|why| format!("cannot stat candidate receipt before append: {why}"))?
        .len();
    file.seek(SeekFrom::End(0))
        .map_err(|why| format!("cannot seek candidate receipt append: {why}"))?;
    let result = file
        .write_all(&receipt.record()?)
        .map_err(|why| format!("cannot append candidate receipt: {why}"));
    match result {
        Ok(()) => Ok(()),
        Err(write_why) => match file.set_len(original) {
            Ok(()) => Err(write_why),
            Err(rollback_why) => Err(format!(
                "{write_why}; candidate receipt rollback to {original} bytes also failed: {rollback_why}; file state is poisoned"
            )),
        },
    }
}

fn scan_orphan(
    file: &mut File,
    committed: u64,
    total: u64,
) -> Result<Option<OrphanBlockV1>, CandidateUniverseRefusal> {
    if committed == total {
        return Ok(None);
    }
    let first = read_row(file, committed)?;
    if first.sequence != 0 {
        return Err(format!(
            "candidate orphan starts with sequence {}, not zero",
            first.sequence
        ));
    }
    let mut index = committed;
    while index < total {
        let row = if index == committed {
            first
        } else {
            read_row(file, index)?
        };
        let expected_sequence = index - committed;
        if row.universe_id != first.universe_id || row.sequence != expected_sequence {
            return Err(format!(
                "candidate tail contains more than one orphan block or non-contiguous sequence at physical row {index}"
            ));
        }
        index = index
            .checked_add(1)
            .ok_or_else(|| "candidate orphan row cursor overflowed u64".to_owned())?;
    }
    Ok(Some(OrphanBlockV1 {
        universe_id: first.universe_id,
        first_row: committed,
        row_count: total - committed,
    }))
}

fn open_file(path: &Path, writable: bool, create: bool) -> Result<File, CandidateUniverseRefusal> {
    OpenOptions::new()
        .read(true)
        .write(writable)
        .create(create)
        .open(path)
        .map_err(|why| format!("cannot open candidate file {}: {why}", path.display()))
}

fn file_generation(file: &File, path: &Path) -> Result<FileGenerationV1, CandidateUniverseRefusal> {
    let held = file
        .metadata()
        .map_err(|why| format!("cannot stat open candidate file {}: {why}", path.display()))?;
    let named = std::fs::metadata(path)
        .map_err(|why| format!("cannot stat named candidate file {}: {why}", path.display()))?;
    let held_generation = generation_of(&held);
    let named_generation = generation_of(&named);
    #[cfg(unix)]
    if (held_generation.device, held_generation.inode)
        != (named_generation.device, named_generation.inode)
    {
        return Err(format!(
            "{} no longer names the opened candidate file",
            path.display()
        ));
    }
    if held_generation != named_generation {
        return Err(format!(
            "{} changed while its candidate generation was measured",
            path.display()
        ));
    }
    Ok(held_generation)
}

#[cfg(unix)]
fn generation_of(metadata: &std::fs::Metadata) -> FileGenerationV1 {
    FileGenerationV1 {
        len: metadata.len(),
        device: metadata.dev(),
        inode: metadata.ino(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    }
}

#[cfg(not(unix))]
fn generation_of(metadata: &std::fs::Metadata) -> FileGenerationV1 {
    FileGenerationV1 {
        len: metadata.len(),
        modified: metadata.modified().ok(),
    }
}

fn require_generation(
    expected: FileGenerationV1,
    file: &File,
    path: &Path,
) -> Result<(), CandidateUniverseRefusal> {
    let observed = file_generation(file, path)?;
    if observed == expected {
        Ok(())
    } else {
        Err(format!(
            "candidate file {} changed since open; cached audit is stale",
            path.display()
        ))
    }
}

fn hex32(bytes: [u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(64);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "fixed-record adversarial fixtures intentionally mutate exact bytes and fail loudly"
)]
mod tests {
    use super::*;
    use brutex_core::instrument::{Exchange, InstrumentKey};
    use pull::calendar::{DayKind, kind_of};
    use runner::exit_grid_policy::{
        ExecutionResolutionV1, ExitGridPolicyV1, ExitGridSelectorV1, ForcedStopV1,
        RangeResolutionV1, RatioLimitsV1, RationalPercentileV1, RungPlanV1,
        printed_ohlcv_cost_model_id_v1,
    };
    use runner::identity::ReferenceIntegrity;

    fn digest(tag: u8) -> [u8; 32] {
        hash(&[tag])
    }

    fn descriptor(tag: u8) -> CandidateUniverseDescriptorV1 {
        let requested_span =
            RequestedSpanIdentityV1::new(2025, 1, 2025, 1).expect("the fixture month is canonical");
        let first_day = i64::from(
            Day::new(2025, 1, 1)
                .expect("the first fixture day is valid")
                .days_from_epoch(),
        );
        let last_day = i64::from(
            Day::new(2025, 1, 1)
                .expect("the fixture month is valid")
                .end_of_month()
                .days_from_epoch(),
        );
        let identities = CandidateUniverseIdentitiesV1 {
            data_digest: digest(tag.wrapping_add(2)),
            feed_digest: digest(tag.wrapping_add(3)),
            source_commit_digest: digest(tag.wrapping_add(4)),
            vocabulary_digest: digest(tag.wrapping_add(5)),
            evaluation_policy_digest: digest(tag.wrapping_add(6)),
            exit_grids: LongShortExitGridIdentitiesV2 {
                long: SideExitGridIdentityV2 {
                    policy_digest: digest(tag.wrapping_add(7)),
                    resolved_digest: digest(tag.wrapping_add(8)),
                },
                short: SideExitGridIdentityV2 {
                    policy_digest: digest(tag.wrapping_add(9)),
                    resolved_digest: digest(tag.wrapping_add(10)),
                },
            },
            calendar_policy_digest: crate::stored::calendar_policy_digest_v2(),
            daily_reference_policy_digest: digest(tag.wrapping_add(12)),
        };
        let mut value = CandidateUniverseDescriptorV1 {
            family: InstrumentFamilyV1::Nifty,
            rung_seconds: 60,
            horizon_bars: 5,
            requested_span,
            identities,
            calendar_coverage: CandidateCalendarCoverageV1 {
                signal_rung_seconds: 60,
                execution_rung_seconds: 60,
                first_day,
                last_day,
                signal_complete_receipt_digest: digest(tag.wrapping_add(13)),
                execution_complete_receipt_digest: digest(tag.wrapping_add(14)),
            },
            signal_stream: CandidateSignalStreamV1 {
                count: 2,
                first_ts_micros: 1_000_000,
                last_ts_micros: 2_000_000,
                digest: digest(tag.wrapping_add(15)),
            },
            execution_stream: CandidateExecutionStreamV1 {
                count: 2,
                first_ts_micros: 1_000_000,
                last_ts_micros: 2_000_000,
                digest: digest(tag.wrapping_add(16)),
            },
            signal_column_digest: digest(tag.wrapping_add(17)),
            execution_column_digest: digest(tag.wrapping_add(18)),
            universe_id: [0; 32],
        };
        value.universe_id = derive_universe_id(&value);
        value.validate().expect("fixture descriptor is coherent");
        value
    }

    fn facts() -> CandidateDirectCellFactsV1 {
        CandidateDirectCellFactsV1 {
            trades: 2,
            wins: 1,
            pessimistic: -100,
            optimistic: 75,
            fill_cost: 25,
            stopped: 1,
            trailed_stop: 0,
            trailed_profit: 0,
            targeted: 1,
            timed_out: 0,
            ambiguous_bars: 1,
            gapped: 1,
            winner_mae: 20,
            winner_mfe: 100,
            all_mae: 80,
            worst_mae: 60,
            gross_win: 50,
            gross_loss: -150,
            best_trade: 50,
            min_win: 50,
            bars_held: 7,
            max_losing_streak: 1,
            max_winning_streak: 1,
            worst_trade: -150,
            max_drawdown: 150,
        }
    }

    fn exit(index: usize) -> ExitCoordinateV1 {
        match index {
            0 => ExitCoordinateV1 {
                stop: Some(0),
                target: None,
                tsl: None,
                ttp: None,
            },
            1 => ExitCoordinateV1 {
                stop: None,
                target: None,
                tsl: None,
                ttp: None,
            },
            _ => panic!("fixture defines exactly two canonical coordinates"),
        }
    }

    fn row(
        descriptor: &CandidateUniverseDescriptorV1,
        sequence: u64,
        direction: TradeDirectionV1,
        ordinal: u64,
        coordinate: ExitCoordinateV1,
    ) -> CandidateUniverseRowV1 {
        let side_tag = match direction {
            TradeDirectionV1::Long => 80,
            TradeDirectionV1::Short => 90,
        };
        let mut value = CandidateUniverseRowV1 {
            universe_id: descriptor.universe_id,
            sequence,
            candidate_semantic_digest: [0; 32],
            mask_words: [1, 0, 0, 0, 0, 0],
            direction,
            family: descriptor.family,
            rung_seconds: descriptor.rung_seconds,
            support_hits: 23,
            exit: coordinate,
            execution_run_id: digest(side_tag),
            horizon_bars: descriptor.horizon_bars,
            evaluated_grid_digest: digest(side_tag.wrapping_add(1)),
            cell_ordinal: ordinal,
            facts: facts(),
        };
        value.candidate_semantic_digest = candidate_semantic_digest(descriptor, &value);
        value
            .validate(Some(descriptor))
            .expect("fixture candidate row is coherent");
        value
    }

    fn reconciliation() -> CompletionReconciliationV2 {
        CompletionReconciliationV2 {
            sweep_trials: 2,
            frequent_itemsets: 1,
            infrequent_itemsets: 1,
            closed_itemsets: 1,
            redundant_itemsets: 0,
            unknown_closure_itemsets: 0,
            exit_cells_per_mask: crate::population::ExitCellsPerMaskV2::new(2, 2)
                .expect("both fixture sides have cells"),
            extinction_depth: 1,
            extinction_complete: true,
            closure_complete: true,
        }
    }

    fn rows(descriptor: &CandidateUniverseDescriptorV1) -> Vec<CandidateUniverseRowV1> {
        vec![
            row(descriptor, 0, TradeDirectionV1::Long, 0, exit(0)),
            row(descriptor, 1, TradeDirectionV1::Long, 1, exit(1)),
            row(descriptor, 2, TradeDirectionV1::Short, 0, exit(0)),
            row(descriptor, 3, TradeDirectionV1::Short, 1, exit(1)),
        ]
    }

    fn prepared(tag: u8) -> PreparedCandidateUniverseV1 {
        let descriptor = descriptor(tag);
        PreparedCandidateUniverseV1::new(&descriptor, reconciliation(), rows(&descriptor))
            .expect("fixture universe is canonical")
    }

    #[test]
    fn nonempty_candidate_receipt_is_exact_two_sided_mask_expansion() {
        let prepared = prepared(33);
        let receipt = *prepared.receipt();
        let reconciliation = receipt.reconciliation();
        let cells_per_mask = reconciliation
            .exit_cells_per_mask
            .long()
            .checked_add(reconciliation.exit_cells_per_mask.short())
            .expect("fixture Long+Short width fits u64");
        let exact_rows = reconciliation
            .closed_itemsets
            .checked_mul(cells_per_mask)
            .expect("fixture closed-mask expansion fits u64");

        assert_eq!(receipt.row_count(), exact_rows);
        assert!(receipt.row_count() >= 2);
        assert!(
            prepared
                .rows()
                .iter()
                .any(|row| row.direction == TradeDirectionV1::Long)
        );
        assert!(
            prepared
                .rows()
                .iter()
                .any(|row| row.direction == TradeDirectionV1::Short)
        );
    }

    pub(crate) fn population_v5_test_canonical_candidate_record() -> [u8; ROW_STRIDE_BYTES] {
        let raw = prepared(32).rows[0]
            .record()
            .expect("Population V5 Candidate fixture encodes");
        verify_population_v5_canonical_record(&raw)
            .expect("Population V5 Candidate fixture passes embedded verification");
        raw
    }

    pub(crate) fn population_v5_test_canonical_candidate_record_for_identity(
        family: InstrumentFamilyV1,
        sequence: u64,
        universe_tag: u8,
        semantic_tag: u8,
    ) -> [u8; ROW_STRIDE_BYTES] {
        let mut raw = population_v5_test_canonical_candidate_record();
        raw[8..40].copy_from_slice(&digest(universe_tag.wrapping_add(120)));
        raw[40..48].copy_from_slice(&sequence.to_le_bytes());
        raw[48..80].copy_from_slice(&digest(semantic_tag.wrapping_add(121)));
        raw[129] = family_byte(family);
        let seal = digest_domain(ROW_SEAL_DOMAIN, &raw[..ROW_PAYLOAD_BYTES]);
        raw[ROW_PAYLOAD_BYTES..].copy_from_slice(&seal);
        verify_population_v5_canonical_record(&raw)
            .expect("parameterized Population V5 Candidate fixture verifies");
        raw
    }

    #[test]
    fn population_v5_embedded_candidate_verifier_covers_every_byte_and_reseal_boundary() {
        let canonical = population_v5_test_canonical_candidate_record();
        let verified = verify_population_v5_canonical_record(&canonical)
            .expect("canonical Candidate fixture verifies");
        for index in 0..ROW_STRIDE_BYTES {
            let mut changed = canonical;
            changed[index] ^= 1;
            assert!(
                verify_population_v5_canonical_record(&changed).is_err(),
                "unresealed mutation at byte {index} must fail"
            );
        }

        let mut reserved = canonical;
        reserved[4] = 1;
        let reserve_seal = digest_domain(ROW_SEAL_DOMAIN, &reserved[..ROW_PAYLOAD_BYTES]);
        reserved[ROW_PAYLOAD_BYTES..].copy_from_slice(&reserve_seal);
        assert!(
            verify_population_v5_canonical_record(&reserved)
                .expect_err("resealed reserved byte must fail")
                .contains("reserve")
        );

        let mut detached_semantic = canonical;
        detached_semantic[48] ^= 1;
        let detached_seal = digest_domain(ROW_SEAL_DOMAIN, &detached_semantic[..ROW_PAYLOAD_BYTES]);
        detached_semantic[ROW_PAYLOAD_BYTES..].copy_from_slice(&detached_seal);
        let detached = verify_population_v5_canonical_record(&detached_semantic)
            .expect("integrity-only verifier accepts a canonical detached semantic identity");
        assert_ne!(
            detached.base_candidate_row_digest(),
            verified.base_candidate_row_digest(),
            "Population V5 must separately join the Candidate-row digest to provenance"
        );
    }

    fn zero_prepared(tag: u8) -> PreparedCandidateUniverseV1 {
        let descriptor = descriptor(tag);
        let zero_row_reconciliation = CompletionReconciliationV2 {
            sweep_trials: 1,
            frequent_itemsets: 0,
            infrequent_itemsets: 1,
            closed_itemsets: 0,
            redundant_itemsets: 0,
            unknown_closure_itemsets: 0,
            exit_cells_per_mask: ExitCellsPerMaskV2::new(2, 2)
                .expect("both directional grids remain non-empty"),
            extinction_depth: 1,
            extinction_complete: true,
            closure_complete: true,
        };
        PreparedCandidateUniverseV1::new(&descriptor, zero_row_reconciliation, Vec::new())
            .expect("natural extinction seals an exact zero-row Candidate family")
    }

    #[test]
    fn initial_empty_frontier_depth_zero_requires_complete_zero_reconciliation() {
        let descriptor = descriptor(219);
        let mut proof = zero_prepared(219).receipt().reconciliation();
        proof.extinction_depth = 0;
        let prepared = PreparedCandidateUniverseV1::new(&descriptor, proof, Vec::new())
            .expect("completed k1 extinction has zero nonempty frontiers");
        assert_eq!(prepared.receipt().reconciliation().extinction_depth, 0);
        assert_eq!(prepared.receipt().row_count(), 0);
        for index in 0..4 {
            let mut changed = proof;
            match index {
                0 => changed.extinction_complete = false,
                1 => changed.closure_complete = false,
                2 => changed.unknown_closure_itemsets = 1,
                _ => {
                    changed.frequent_itemsets = 1;
                    changed.redundant_itemsets = 1;
                    changed.sweep_trials += 1;
                }
            }
            assert!(
                PreparedCandidateUniverseV1::new(&descriptor, changed, Vec::new()).is_err(),
                "missing zero-frontier proof {index}"
            );
        }
    }

    #[test]
    fn actual_candidate_resource_halt_cannot_mint_initial_empty_completion() {
        let fixture = ProductionFixture::new();
        let sweeper = Sweeper::new(
            engine::Ladder::with_min_hits(1)
                .with_ceiling(1)
                .with_pair_budget(1),
        );
        let bounds = CandidateUniverseBoundsV1::new(1_000_000, 4).expect("bounded fixture");
        let run = sweeper
            .run_prepared_population_by_reporting(
                fixture.source().signal_column,
                &|_, _, _| {},
                |_| Ok::<(), ()>(()),
            )
            .expect("non-pricing observer cannot fail");
        assert!(
            run.outcome.sweep.halted.is_some(),
            "fixture must hit an actual engine resource bound"
        );
        assert!(
            reconcile_candidate_population(
                &run,
                &ExitCellsPerMaskV2::new(2, 2).expect("bounded grid sides"),
                0
            )
            .is_err()
        );
        let result =
            produce_candidate_universe_v1(&sweeper, fixture.source(), bounds, &|_, _, _| {});
        assert!(
            result.is_err(),
            "actual exhausted resource budget cannot produce an extinction authority"
        );
    }

    #[test]
    fn actual_empty_and_cold_columns_cannot_mint_initial_empty_completion() {
        let bars = runner::synthetic::sessions(1);
        let sweeper = Sweeper::new(engine::Ladder::with_min_hits(1_000_000));
        let cells = ExitCellsPerMaskV2::new(2, 2).expect("two existing grid sides");
        for length in [0, 100] {
            let input = bars.get(..length).expect("bounded generated input");
            let mut evaluator = indicators::evaluator::Evaluator::new(
                Widths::pinned().expect("pinned widths"),
                Availability::Absent,
                Thresholds::CLASSICAL,
            );
            let column = Column::build(input, &mut evaluator);
            assert!(column.is_empty(), "fixture must contain no warmed row");
            let run = sweeper
                .run_prepared_population_by_reporting(column, &|_, _, _| {}, |_| Ok::<(), ()>(()))
                .expect("no candidate callback can fail");
            assert!(!run.is_complete());
            assert!(reconcile_candidate_population(&run, &cells, 0).is_err());
        }
    }

    #[test]
    fn base_evidence_seals_a_legitimate_zero_row_candidate_completion() {
        let descriptor = descriptor(31);
        let zero_row_reconciliation = CompletionReconciliationV2 {
            sweep_trials: 1,
            frequent_itemsets: 0,
            infrequent_itemsets: 1,
            closed_itemsets: 0,
            redundant_itemsets: 0,
            unknown_closure_itemsets: 0,
            exit_cells_per_mask: ExitCellsPerMaskV2::new(2, 2)
                .expect("both directional grids remain non-empty"),
            extinction_depth: 1,
            extinction_complete: true,
            closure_complete: true,
        };
        let candidate =
            PreparedCandidateUniverseV1::new(&descriptor, zero_row_reconciliation, Vec::new())
                .expect("natural extinction with no frequent masks is a complete Candidate family");
        assert_eq!(candidate.receipt().row_count(), 0);

        let base = BaseEvidenceBuilderV2::new(
            BaseEvidenceBoundsV2::new(1, 1, 1, 1).expect("non-zero preparation ceilings"),
        )
        .seal(candidate.receipt())
        .expect("the empty ordered Base family seals against its Candidate completion");
        assert_eq!(base.record_count(), 0);
        assert_ne!(
            base.ordered_record_digest(),
            [0; 32],
            "domain, Candidate receipt and zero count still form an exact family identity"
        );
        base.validate_candidate_receipt(candidate.receipt())
            .expect("the empty Base family remains bound to the exact empty Candidate receipt");
    }

    fn candidate_base_fixture(
        tag: u8,
        family: InstrumentFamilyV1,
    ) -> (CandidateUniverseReopenAuditV1, PreparedBaseEvidenceV2) {
        let mut descriptor = descriptor(tag);
        descriptor.family = family;
        descriptor.universe_id = derive_universe_id(&descriptor);
        descriptor
            .validate()
            .expect("family-specific fixture descriptor is coherent");
        let candidate =
            PreparedCandidateUniverseV1::new(&descriptor, reconciliation(), rows(&descriptor))
                .expect("family-specific Candidate block is canonical");
        let base = fixture_prepared_base_evidence_v2(candidate.receipt(), candidate.rows())
            .expect("fixture Base family is canonical");
        let audit = CandidateUniverseReopenAuditV1 {
            first_row: 0,
            receipt: *candidate.receipt(),
        };
        (audit, base)
    }

    #[test]
    fn durable_base_is_receipt_last_idempotent_fixed_offset_and_zero_row_safe() {
        let root = test_dir();
        let bounds = BaseEvidenceLedgerBoundsV2::new(32, 8).expect("nonzero Base bounds");
        let (candidate, base) = candidate_base_fixture(41, InstrumentFamilyV1::Nifty);
        let written = append_and_reopen_base_evidence_v2(root.path(), bounds, &candidate, &base)
            .expect("Base records sync before a freshly reopened completion");
        assert!(matches!(
            written,
            BaseEvidenceProductionCommitV2::Written(_)
        ));
        let reused = append_and_reopen_base_evidence_v2(root.path(), bounds, &candidate, &base)
            .expect("the exact retry reuses every durable byte");
        assert!(matches!(reused, BaseEvidenceProductionCommitV2::Reused(_)));
        assert_eq!(written.audit(), reused.audit());

        let mut reader = BaseEvidenceLedgerReaderV2::open(root.path(), bounds)
            .expect("fresh read authority opens");
        let audit = reader
            .audit(candidate.universe_id())
            .expect("audit lookup is generation checked")
            .expect("Candidate Base completion exists");
        assert_eq!(audit.record_count(), candidate.row_count());
        let first = reader
            .record(&audit, 0)
            .expect("fixed-offset Candidate sequence zero reads");
        assert_eq!(first.candidate_universe_id(), candidate.universe_id());
        assert_eq!(first.candidate_sequence(), 0);
        assert_ne!(first.evidence_id(), [0; 32]);
        assert_ne!(first.candidate_semantic_id(), [0; 32]);
        assert_ne!(first.candidate_row_digest(), [0; 32]);
        assert_ne!(first.trade_rows_digest(), [0; 32]);
        let _ = first.admission_values();
        assert!(matches!(
            reader.record(&audit, audit.record_count()),
            Err(BaseEvidenceLedgerRefusalV2::BoundExceeded {
                resource: "record sequence",
                ..
            })
        ));

        let foreign_root = test_dir();
        let (foreign_candidate, foreign_base) =
            candidate_base_fixture(43, InstrumentFamilyV1::BankNifty);
        let foreign_audit = append_and_reopen_base_evidence_v2(
            foreign_root.path(),
            bounds,
            &foreign_candidate,
            &foreign_base,
        )
        .expect("foreign audit fixture commits independently")
        .audit();
        assert!(
            reader
                .record(&foreign_audit, 0)
                .expect_err("an audit from another ledger must not address this ledger")
                .to_string()
                .contains("names no complete")
        );

        let mut zero_descriptor = descriptor(42);
        zero_descriptor.universe_id = derive_universe_id(&zero_descriptor);
        let zero_reconciliation = CompletionReconciliationV2 {
            sweep_trials: 1,
            frequent_itemsets: 0,
            infrequent_itemsets: 1,
            closed_itemsets: 0,
            redundant_itemsets: 0,
            unknown_closure_itemsets: 0,
            exit_cells_per_mask: ExitCellsPerMaskV2::new(2, 2)
                .expect("directional grids are complete"),
            extinction_depth: 1,
            extinction_complete: true,
            closure_complete: true,
        };
        let zero_candidate =
            PreparedCandidateUniverseV1::new(&zero_descriptor, zero_reconciliation, Vec::new())
                .expect("natural extinction may produce no Candidate rows");
        let zero_base =
            fixture_prepared_base_evidence_v2(zero_candidate.receipt(), zero_candidate.rows())
                .expect("zero-row Base family is canonical");
        let zero_audit = CandidateUniverseReopenAuditV1 {
            first_row: 0,
            receipt: *zero_candidate.receipt(),
        };
        let zero_commit =
            append_and_reopen_base_evidence_v2(root.path(), bounds, &zero_audit, &zero_base)
                .expect("zero-row Base completion remains durable and receipt-last");
        assert_eq!(zero_commit.audit().record_count(), 0);
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one adversarial lifecycle keeps orphan, swap, corruption and path-generation attacks visibly ordered"
    )]
    fn durable_base_recovers_only_exact_orphans_and_refuses_swaps_corruption_and_stale_paths() {
        let bounds = BaseEvidenceLedgerBoundsV2::new(32, 8).expect("nonzero Base bounds");
        let (candidate, base) = candidate_base_fixture(51, InstrumentFamilyV1::Nifty);

        let orphan_root = test_dir();
        append_and_reopen_base_evidence_v2(orphan_root.path(), bounds, &candidate, &base)
            .expect("fixture Base commit succeeds");
        let completion_path = orphan_root.path().join("base-evidence-completions-v2.bin");
        OpenOptions::new()
            .write(true)
            .open(&completion_path)
            .expect("completion opens")
            .set_len(64)
            .expect("fixture simulates crash before completion append");
        let repaired =
            append_and_reopen_base_evidence_v2(orphan_root.path(), bounds, &candidate, &base)
                .expect("an exact contiguous orphan prefix is reusable");
        assert!(matches!(
            repaired,
            BaseEvidenceProductionCommitV2::Written(_)
        ));

        let foreign_root = test_dir();
        append_and_reopen_base_evidence_v2(foreign_root.path(), bounds, &candidate, &base)
            .expect("foreign-orphan fixture commits");
        OpenOptions::new()
            .write(true)
            .open(foreign_root.path().join("base-evidence-completions-v2.bin"))
            .expect("completion opens")
            .set_len(64)
            .expect("completion is removed after records");
        let (foreign_candidate, foreign_base) =
            candidate_base_fixture(52, InstrumentFamilyV1::Nifty);
        assert!(
            append_and_reopen_base_evidence_v2(
                foreign_root.path(),
                bounds,
                &foreign_candidate,
                &foreign_base,
            )
            .expect_err("foreign physical tail must not be hidden")
            .to_string()
            .contains("another source")
        );

        let swap_root = test_dir();
        append_and_reopen_base_evidence_v2(swap_root.path(), bounds, &candidate, &base)
            .expect("swap fixture commits");
        let record_path = swap_root.path().join("base-evidence-records-v2.bin");
        let raw = std::fs::read(&record_path).expect("record file reads");
        let mut swapped = raw.clone();
        swapped[64..1_088].copy_from_slice(&raw[1_088..2_112]);
        swapped[1_088..2_112].copy_from_slice(&raw[64..1_088]);
        std::fs::write(&record_path, swapped).expect("fixture swaps two sealed rows");
        assert!(
            BaseEvidenceLedgerReaderV2::open(swap_root.path(), bounds)
                .err()
                .expect("swapped rows must refuse")
                .to_string()
                .contains("sequence")
        );

        let corrupt_root = test_dir();
        append_and_reopen_base_evidence_v2(corrupt_root.path(), bounds, &candidate, &base)
            .expect("corruption fixture commits");
        let corrupt_path = corrupt_root.path().join("base-evidence-records-v2.bin");
        let mut corrupt = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&corrupt_path)
            .expect("record file opens");
        corrupt
            .seek(SeekFrom::Start(64 + 120))
            .expect("payload seeks");
        corrupt.write_all(&[0xA5]).expect("payload corrupts");
        corrupt.sync_data().expect("corruption reaches disk");
        assert!(
            BaseEvidenceLedgerReaderV2::open(corrupt_root.path(), bounds)
                .err()
                .expect("changed payload must break its seal")
                .to_string()
                .contains("seal")
        );

        let stale_root = test_dir();
        append_and_reopen_base_evidence_v2(stale_root.path(), bounds, &candidate, &base)
            .expect("stale fixture commits");
        let mut stale_reader = BaseEvidenceLedgerReaderV2::open(stale_root.path(), bounds)
            .expect("reader opens before file changes");
        let stale_audit = stale_reader
            .audit(candidate.universe_id())
            .expect("pre-change audit lookup is valid")
            .expect("pre-change Candidate completion exists");
        OpenOptions::new()
            .append(true)
            .open(stale_root.path().join("base-evidence-records-v2.bin"))
            .expect("record path opens independently")
            .write_all(&[0])
            .expect("record generation changes");
        assert!(
            stale_reader
                .record(&stale_audit, 0)
                .expect_err("fixed-offset reads must refuse a stale file generation")
                .to_string()
                .contains("changed since open")
        );
        assert!(
            stale_reader
                .audit(candidate.universe_id())
                .expect_err("stale generation must refuse")
                .to_string()
                .contains("changed since open")
        );

        let replaced_root = test_dir();
        append_and_reopen_base_evidence_v2(replaced_root.path(), bounds, &candidate, &base)
            .expect("replacement fixture commits");
        let mut replacement_reader = BaseEvidenceLedgerReaderV2::open(replaced_root.path(), bounds)
            .expect("reader opens before named path replacement");
        let path = replaced_root.path().join("base-evidence-records-v2.bin");
        let displaced = replaced_root
            .path()
            .join("base-evidence-records-v2.displaced");
        std::fs::rename(&path, &displaced).expect("opened inode is displaced");
        std::fs::copy(&displaced, &path).expect("same bytes appear at a new inode");
        let replacement_refusal = replacement_reader
            .audit(candidate.universe_id())
            .expect_err("same bytes at a replacement path must refuse")
            .to_string();
        assert!(
            replacement_refusal.contains("no longer names")
                || replacement_refusal.contains("changed since open"),
            "replacement must be caught by the held root generation or the child inode: {replacement_refusal}"
        );
    }

    #[test]
    fn durable_base_completion_reserved_reseal_and_pairing_policy_fail_closed() {
        let bounds = BaseEvidenceLedgerBoundsV2::new(64, 8).expect("nonzero Base bounds");
        let root = test_dir();
        let (nifty_candidate, nifty_base) = candidate_base_fixture(61, InstrumentFamilyV1::Nifty);
        let (bank_candidate, bank_base) = candidate_base_fixture(61, InstrumentFamilyV1::BankNifty);
        let nifty =
            append_and_reopen_base_evidence_v2(root.path(), bounds, &nifty_candidate, &nifty_base)
                .expect("NIFTY Base authority commits")
                .audit();
        let bank =
            append_and_reopen_base_evidence_v2(root.path(), bounds, &bank_candidate, &bank_base)
                .expect("BANKNIFTY Base authority commits")
                .audit();
        let pair = pair_base_evidence_authority_v2(&nifty, &bank)
            .expect("canonical NIFTY then BANKNIFTY cohort pairs");
        assert_ne!(pair.pair_id(), [0; 32]);
        assert_eq!(pair.nifty_audit(), &nifty);
        assert_eq!(pair.banknifty_audit(), &bank);
        assert!(
            pair_base_evidence_authority_v2(&bank, &nifty)
                .expect_err("reversed families must refuse")
                .to_string()
                .contains("ordered")
        );

        let mismatch_root = test_dir();
        let (mismatch_candidate, mismatch_base) =
            candidate_base_fixture(62, InstrumentFamilyV1::BankNifty);
        let mismatch = append_and_reopen_base_evidence_v2(
            mismatch_root.path(),
            bounds,
            &mismatch_candidate,
            &mismatch_base,
        )
        .expect("foreign cohort Base commits independently")
        .audit();
        assert!(
            pair_base_evidence_authority_v2(&nifty, &mismatch)
                .expect_err("different shared cohort terms must refuse")
                .to_string()
                .contains("cohort policy")
        );

        let reserve_root = test_dir();
        append_and_reopen_base_evidence_v2(
            reserve_root.path(),
            bounds,
            &nifty_candidate,
            &nifty_base,
        )
        .expect("reserved-byte fixture commits");
        let completion_path = reserve_root.path().join("base-evidence-completions-v2.bin");
        let mut completion = std::fs::read(&completion_path).expect("completion bytes read");
        completion[64 + 464] = 1;
        let seal = digest_domain(
            b"brutex-base-evidence-v2-completion-seal\0",
            &completion[64..64 + 480],
        );
        completion[64 + 480..64 + 512].copy_from_slice(&seal);
        std::fs::write(&completion_path, completion).expect("reserved byte is re-sealed");
        assert!(
            BaseEvidenceLedgerReaderV2::open(reserve_root.path(), bounds)
                .err()
                .expect("re-sealed nonzero reserve must refuse")
                .to_string()
                .contains("tail reserve")
        );
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

    fn test_dir() -> TestDir {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nth = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "brutex-candidate-universe-{}-{nth}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("test directory is writable");
        TestDir(root)
    }

    const TEST_FEED: &str = "candidate-production-feed";
    const TEST_COMMIT: &str = "candidate-production-commit";
    const DAY_MICROS: i64 = 86_400_000_000;
    const MINUTE_MICROS: i64 = 60_000_000;

    struct ProductionFixture {
        instrument: InstrumentKey,
        span: RequestedSpanIdentityV1,
        context: Vec<Candle>,
        execution_start: usize,
        daily_bars: Vec<Candle>,
        eligibility: Vec<u8>,
        daily_references: Vec<DailyReference>,
        signal_calendar: CompleteCalendarReceiptV2,
        execution_calendar: CompleteCalendarReceiptV2,
        long: ResolvedExitGridV1,
        short: ResolvedExitGridV1,
    }

    impl ProductionFixture {
        fn new() -> Self {
            let instrument =
                InstrumentKey::index(Exchange::Nse, "NIFTY").expect("NIFTY is a swept index");
            let span = RequestedSpanIdentityV1::new(2025, 1, 2025, 1)
                .expect("fixture span is one measured month");
            let (first_day, last_day) = requested_span_days(span).expect("fixture day bounds");
            let prior_day = (first_day.saturating_sub(40)..first_day)
                .rev()
                .find(|day| matches!(kind_of(*day), DayKind::Open(_)))
                .expect("a measured open day precedes the fixture month");
            let context = minute_bars(prior_day, last_day);
            let execution_start = context
                .iter()
                .position(|bar| indicators::ist_day(bar.ts_micros) >= first_day)
                .expect("requested month has one measured session");
            let execution = context
                .get(execution_start..)
                .expect("execution suffix is in context");
            let signal_calendar =
                crate::stored::calendar_receipt_v2_for_bars(execution, 60, first_day, last_day)
                    .expect("signal calendar is measured")
                    .require_complete()
                    .expect("signal calendar is complete");
            let execution_calendar =
                crate::stored::calendar_receipt_v2_for_bars(execution, 60, first_day, last_day)
                    .expect("execution calendar is measured")
                    .require_complete()
                    .expect("execution calendar is complete");
            let (daily_bars, eligibility, daily_references) =
                daily_reference_fixture(prior_day, last_day);
            let series = ExecutionSeriesV1::new(
                &instrument,
                TEST_FEED,
                TEST_COMMIT,
                crate::stored::calendar_policy_digest_v2(),
                execution,
            )
            .expect("execution series is typed");
            let long = exit_policy_fixture(Side::Long)
                .resolve_attested(series)
                .expect("long grid resolves from exact execution");
            let short = exit_policy_fixture(Side::Short)
                .resolve_attested(series)
                .expect("short grid resolves from exact execution");
            Self {
                instrument,
                span,
                context,
                execution_start,
                daily_bars,
                eligibility,
                daily_references,
                signal_calendar,
                execution_calendar,
                long,
                short,
            }
        }

        fn execution(&self) -> &[Candle] {
            self.context
                .get(self.execution_start..)
                .expect("fixture execution suffix remains in context")
        }

        fn daily_binding(&self) -> DailyReferenceBinding<'_> {
            DailyReferenceBinding {
                daily_bars: &self.daily_bars,
                eligibility: &self.eligibility,
                schema: crate::stored::DAILY_REFERENCE_SCHEMA,
                eligibility_policy: crate::stored::DAILY_ELIGIBILITY_POLICY,
                gap_overlay_policy: crate::stored::EXACT_MINUTE_GAP_POLICY,
                excluded_ist_days: &CHARTER_NON_REGULAR_IST_DAYS,
                daily_integrity: ReferenceIntegrity::UnverifiedNoReceipt,
                minute_integrity: ReferenceIntegrity::UnverifiedNoReceipt,
                swept_series_calendar_policy: crate::stored::SWEPT_SERIES_CALENDAR_POLICY,
            }
        }

        fn series(&self) -> ExecutionSeriesV1<'_> {
            ExecutionSeriesV1::new(
                &self.instrument,
                TEST_FEED,
                TEST_COMMIT,
                crate::stored::calendar_policy_digest_v2(),
                self.execution(),
            )
            .expect("fixture execution series remains typed")
        }

        fn source(&self) -> CandidateUniverseProductionSourceV1<'_> {
            CandidateUniverseProductionSourceV1::new(
                InstrumentFamilyV1::Nifty,
                60,
                Horizon::DEFAULT,
                self.span,
                self.signal_calendar,
                self.execution_calendar,
                self.execution(),
                &self.daily_references,
                &self.context,
                self.daily_binding(),
                self.series(),
                Widths::pinned().expect("fixture uses measured widths"),
                Availability::Absent,
                Thresholds::CLASSICAL,
                &self.long,
                &self.short,
                StoredSpanLoadBoundV1::new(
                    u64::try_from(self.execution().len()).expect("signal length fits u64"),
                )
                .expect("signal load ceiling is nonzero"),
                StoredSpanLoadBoundV1::new(
                    u64::try_from(self.context.len()).expect("minute length fits u64"),
                )
                .expect("minute load ceiling is nonzero"),
                StoredSpanLoadBoundV1::new(
                    u64::try_from(self.daily_bars.len()).expect("daily length fits u64"),
                )
                .expect("daily load ceiling is nonzero"),
            )
            .expect("all production fixture sources agree")
        }
    }

    fn minute_bars(first_day: i64, last_day: i64) -> Vec<Candle> {
        let mut bars = Vec::new();
        let mut ordinal = 0_i64;
        for day in first_day..=last_day {
            match kind_of(day) {
                DayKind::Open(session) => {
                    for window in session.windows.iter().take(usize::from(session.count)) {
                        for minute in window.from..=window.to {
                            let price = 2_000_000_i64.saturating_add(
                                ordinal
                                    .saturating_mul(37)
                                    .saturating_add(i64::from(minute).saturating_mul(17))
                                    .rem_euclid(20_003),
                            );
                            let body = ordinal.saturating_mul(43).rem_euclid(181) - 90;
                            let close = price.saturating_add(body);
                            let high = price.max(close).saturating_add(50 + ordinal.rem_euclid(71));
                            let low = price
                                .min(close)
                                .saturating_sub(50 + ordinal.saturating_mul(13).rem_euclid(67));
                            bars.push(Candle {
                                ts_micros: day
                                    .saturating_mul(DAY_MICROS)
                                    .saturating_add(i64::from(minute).saturating_mul(MINUTE_MICROS))
                                    .saturating_sub(indicators::IST_OFFSET_MICROS),
                                open: price,
                                high,
                                low,
                                close,
                                volume: 1_000_i64
                                    .saturating_add(ordinal.saturating_mul(29).rem_euclid(9_973)),
                                open_interest: i64::MIN,
                            });
                            ordinal = ordinal.saturating_add(1);
                        }
                    }
                }
                DayKind::Closed => {}
                DayKind::OpenLengthUnmeasured | DayKind::Unmeasured => {
                    panic!("fixture day {day} has no measured minute length")
                }
            }
        }
        bars
    }

    fn daily_reference_fixture(
        first_day: i64,
        last_day: i64,
    ) -> (Vec<Candle>, Vec<u8>, Vec<DailyReference>) {
        let mut bars = Vec::new();
        let mut eligibility = Vec::new();
        let mut references = Vec::new();
        for day in first_day..=last_day {
            match kind_of(day) {
                DayKind::Open(_) => {
                    let bar = Candle {
                        ts_micros: day
                            .saturating_mul(DAY_MICROS)
                            .saturating_sub(indicators::IST_OFFSET_MICROS),
                        open: 1_900_000,
                        high: 2_100_000,
                        low: 1_800_000,
                        close: 2_000_000,
                        volume: 375_000,
                        open_interest: i64::MIN,
                    };
                    let decision = u8::from(!CHARTER_NON_REGULAR_IST_DAYS.contains(&day));
                    let typed = DailyReference::new(
                        bar,
                        if decision == 1 {
                            DailyEligibility::Eligible
                        } else {
                            DailyEligibility::Excluded
                        },
                    )
                    .expect("fixture daily record is usable");
                    bars.push(bar);
                    eligibility.push(decision);
                    references.push(typed);
                }
                DayKind::Closed => {}
                DayKind::OpenLengthUnmeasured | DayKind::Unmeasured => {
                    panic!("fixture daily day {day} is not measured")
                }
            }
        }
        (bars, eligibility, references)
    }

    fn exit_policy_fixture(side: Side) -> ExitGridPolicyV1 {
        let half = RationalPercentileV1::new(1, 2).expect("one-half is canonical");
        ExitGridPolicyV1::new(
            ExecutionResolutionV1::OneMinuteOhlcv,
            RangeResolutionV1::PpmCeiling,
            side,
            RungPlanV1::new(vec![half], vec![half], vec![half], 1)
                .expect("one rung per exact axis"),
            RatioLimitsV1::new(1, 10_000, 1).expect("one broad exact ratio interval"),
            1_000,
            ExitGridSelectorV1::GuaranteedFloor,
            printed_ohlcv_cost_model_id_v1(),
            ForcedStopV1::Disabled,
            u64::MAX,
            u64::MAX,
        )
        .expect("complete explicit exit policy")
    }

    #[test]
    fn exact_stream_and_column_facts_are_identity_terms() {
        let original = descriptor(1);
        let mut alternatives = Vec::new();
        for change in 0..4 {
            let mut changed = original;
            match change {
                0 => changed.signal_stream.digest = digest(201),
                1 => changed.execution_stream.digest = digest(202),
                2 => changed.signal_column_digest = digest(203),
                3 => changed.execution_column_digest = digest(204),
                _ => unreachable!(),
            }
            changed.universe_id = derive_universe_id(&changed);
            changed
                .validate()
                .expect("changed source fact remains valid");
            alternatives.push(changed.universe_id);
        }
        assert!(alternatives.iter().all(|id| *id != original.universe_id));
        assert_eq!(
            alternatives
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            alternatives.len(),
            "each exact source fact is a distinct universe-ID term"
        );
    }

    #[test]
    fn row_and_receipt_codecs_cover_all_raw_facts_and_full_seals() {
        let prepared = prepared(10);
        let source = prepared.rows()[0];
        let row_record = source.record().expect("row encodes");
        assert_eq!(row_record.len(), ROW_STRIDE_BYTES);
        assert_eq!(
            CandidateUniverseRowV1::decode(&row_record).expect("row decodes"),
            source
        );
        let rebuilt = source
            .facts()
            .to_cell(source.exit())
            .expect("all raw direct facts reconstruct");
        assert_eq!(
            CandidateDirectCellFactsV1::from_cell(&rebuilt),
            source.facts()
        );

        let receipt = *prepared.receipt();
        let receipt_record = receipt.record().expect("receipt encodes");
        assert_eq!(receipt_record.len(), RECEIPT_STRIDE_BYTES);
        assert_eq!(
            CandidateUniverseReceiptV1::decode(&receipt_record).expect("receipt decodes"),
            receipt
        );
        assert_eq!(receipt.signal_stream(), receipt.descriptor.signal_stream);
        assert_eq!(
            receipt.execution_stream(),
            receipt.descriptor.execution_stream
        );
        assert_eq!(
            receipt.signal_column_digest(),
            receipt.descriptor.signal_column_digest
        );
        assert_eq!(
            receipt.execution_column_digest(),
            receipt.descriptor.execution_column_digest
        );

        let mut corrupt_row = row_record;
        corrupt_row[248] ^= 1;
        assert!(
            CandidateUniverseRowV1::decode(&corrupt_row)
                .expect_err("changed fact must break the full row seal")
                .contains("seal")
        );
        let mut corrupt_receipt = receipt_record;
        corrupt_receipt[752] ^= 1;
        assert!(
            CandidateUniverseReceiptV1::decode(&corrupt_receipt)
                .expect_err("changed stream fact must break the full receipt seal")
                .contains("seal")
        );

        let mut semantically_foreign = receipt;
        semantically_foreign.descriptor.calendar_coverage.first_day = semantically_foreign
            .descriptor
            .calendar_coverage
            .first_day
            .checked_add(1)
            .expect("fixture day can advance");
        semantically_foreign.descriptor.universe_id =
            derive_universe_id(&semantically_foreign.descriptor);
        let payload = semantically_foreign.payload_without_content_check();
        semantically_foreign.content_digest = digest_domain(RECEIPT_CONTENT_DOMAIN, &payload);
        let mut resealed_record = [0_u8; RECEIPT_STRIDE_BYTES];
        resealed_record[..RECEIPT_PAYLOAD_BYTES].copy_from_slice(&payload);
        put_bytes(
            &mut resealed_record,
            RECEIPT_PAYLOAD_BYTES,
            &semantically_foreign.content_digest,
        );
        assert!(
            CandidateUniverseReceiptV1::decode(&resealed_record)
                .expect_err("a re-sealed calendar interval foreign to the request must refuse")
                .contains("requested month span")
        );
    }

    #[test]
    fn canonical_coordinate_order_is_constant_state_and_refuses_duplicates() {
        let descriptor = descriptor(20);
        let canonical = rows(&descriptor);
        PreparedCandidateUniverseV1::new(&descriptor, reconciliation(), canonical.clone())
            .expect("canonical runner coordinate order passes");

        assert!(canonical_coordinate_key(exit(0)) < canonical_coordinate_key(exit(1)));
        let without_ttp = ExitCoordinateV1 {
            stop: Some(0),
            target: Some(2),
            tsl: Some(2),
            ttp: None,
        };
        let with_ttp = ExitCoordinateV1 {
            ttp: Some((0, 0)),
            ..without_ttp
        };
        assert!(canonical_coordinate_key(without_ttp) < canonical_coordinate_key(with_ttp));

        let mut duplicate = canonical.clone();
        duplicate[1].exit = duplicate[0].exit;
        duplicate[1].candidate_semantic_digest =
            candidate_semantic_digest(&descriptor, &duplicate[1]);
        assert!(
            PreparedCandidateUniverseV1::new(&descriptor, reconciliation(), duplicate)
                .expect_err("duplicate coordinate must refuse")
                .contains("duplicated or reordered")
        );

        let mut reordered = canonical;
        let first = reordered[0].exit;
        reordered[0].exit = reordered[1].exit;
        reordered[1].exit = first;
        for row in reordered.iter_mut().take(2) {
            row.candidate_semantic_digest = candidate_semantic_digest(&descriptor, row);
        }
        assert!(
            PreparedCandidateUniverseV1::new(&descriptor, reconciliation(), reordered)
                .expect_err("reordered coordinates must refuse")
                .contains("duplicated or reordered")
        );

        let mut incomplete = rows(&descriptor);
        incomplete.pop();
        assert!(
            PreparedCandidateUniverseV1::new(&descriptor, reconciliation(), incomplete)
                .expect_err("one missing canonical row must refuse")
                .contains("not the reconciled expected")
        );
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one end-to-end Candidate fixture proves both complete grids, exact observation replay, receipt-last persistence and exact retry together"
    )]
    fn typed_producer_seals_both_full_grids_and_internal_commit_is_exactly_idempotent() {
        let fixture = ProductionFixture::new();
        let source = fixture.source();
        let (frequent_bit, max_singleton_support, max_support_ties) =
            maximum_nontrivial_live_singleton_support(&source.signal_column);
        assert!(
            max_support_ties <= 16,
            "the varied fixture must leave a bounded naturally-extinct maximum-support frontier, got {max_support_ties} tied singleton bits"
        );
        let ladder = engine::Ladder::with_min_hits(max_singleton_support);
        let sweeper = Sweeper::new(ladder);
        let bounds = CandidateUniverseBoundsV1::new(1_000_000, 4)
            .expect("production fixture bounds are explicit");

        let identities = production_identities(&source, &sweeper)
            .expect("typed identities derive without caller digests");
        let descriptor = CandidateUniverseDescriptorV1::new(
            source.family,
            source.rung_seconds,
            source.horizon,
            source.requested_span,
            &identities,
            source.signal_calendar,
            source.execution_calendar,
            source.signal_bars,
            &source.signal_column,
            source.execution_series,
            &source.execution_column,
        )
        .expect("production descriptor derives from the opaque source");
        let mask = vocab::ConditionMask::ZERO.with_bit(frequent_bit);
        let member = PopulationMember {
            item: engine::Itemset {
                mask,
                hits: max_singleton_support,
            },
            closure: ClosureVerdict::Closed,
        };
        let mut expanded = Vec::new();
        let mut expanded_observations = CandidateObservationBuilderV1::from_exact_execution(
            source.execution_calendar,
            source.execution_series.bars(),
            &source.execution_column,
        )
        .expect("fixture exact execution forms accepted-session observations");
        let expanded_base_bounds = BaseEvidenceBoundsV2::new(
            bounds.max_rows(),
            source.minute_load_bound.max_records(),
            source.signal_load_bound.max_records(),
            source.signal_load_bound.max_records(),
        )
        .expect("fixture Base Evidence bounds are explicit");
        let mut expanded_base_builder = BaseEvidenceBuilderV2::new(expanded_base_bounds);
        expand_population_member(
            member,
            &descriptor,
            execution_run_params(&sweeper, &source),
            source.signal_bars,
            source.reference_minute_context,
            source.daily_reference,
            source.execution_series,
            &source.execution_column,
            source.long_exit_grid,
            source.short_exit_grid,
            source.horizon,
            source.rung_seconds,
            bounds,
            &mut expanded,
            &mut expanded_observations,
            &source.signal_column,
            expanded_base_bounds,
            &mut expanded_base_builder,
        )
        .expect("one closed mask expands through both complete grids");
        let expected_width = source
            .long_exit_grid
            .cell_count()
            .checked_add(source.short_exit_grid.cell_count())
            .expect("fixture grid widths add");
        assert_eq!(
            u64::try_from(expanded.len()).expect("expanded rows fit u64"),
            expected_width
        );
        assert_eq!(
            expanded_observations.candidate_count(),
            expanded.len(),
            "each exact evaluated cell produces one complete accepted-session observation row"
        );
        let short_start = usize::try_from(source.long_exit_grid.cell_count())
            .expect("long grid width fits usize");
        assert!(
            expanded
                .get(..short_start)
                .expect("long prefix exists")
                .iter()
                .all(|row| row.direction() == TradeDirectionV1::Long)
        );
        assert!(
            expanded
                .get(short_start..)
                .expect("short suffix exists")
                .iter()
                .all(|row| row.direction() == TradeDirectionV1::Short)
        );
        assert_ne!(
            expanded.first().expect("long row").execution_run_id(),
            expanded.last().expect("short row").execution_run_id(),
            "direction is an execution-run identity term"
        );

        let produced = produce_candidate_universe_v1(&sweeper, source, bounds, &|_, _, _| {})
            .expect("an uncapped naturally-extinct production walk completes");
        assert!(produced.population_run().is_complete());
        assert!(
            produced.population_run().closed > 0,
            "a genuinely frequent singleton frontier must retire at least one closed mask"
        );
        let produced_rows = u64::try_from(produced.row_count()).expect("produced rows fit u64");
        assert_eq!(
            produced_rows,
            produced
                .population_run()
                .closed
                .checked_mul(expected_width)
                .expect("fixture closed masks times grid width fits u64")
        );
        assert_eq!(
            produced.observations().candidate_count(),
            produced.row_count()
        );
        assert_eq!(
            produced.base_evidence().record_count(),
            produced.row_count(),
            "each Candidate row has one exact same-pass Base Evidence record"
        );
        assert_ne!(
            produced.base_evidence().ordered_record_digest(),
            [0; 32],
            "the ordered in-memory Phase-A record block is identity-bound"
        );
        assert!(
            produced
                .base_evidence()
                .records()
                .iter()
                .zip(produced.prepared.rows.iter())
                .all(|(base, candidate)| {
                    base.evidence_id() != [0; 32]
                        && base.candidate_semantic_id() == candidate.candidate_semantic_digest()
                }),
            "Base Evidence keeps Candidate ordering and semantic identity without minting a final Population or Selection ID"
        );
        assert!(
            produced.observations().candidates().iter().all(
                |candidate| candidate.periods().len() == produced.observations().period_count()
            ),
            "every Candidate row carries the same explicit accepted-session width"
        );
        let root = test_dir();
        let written = produced
            .append_and_reopen(root.path(), bounds)
            .expect("typed production writes rows then receipt and reopens");
        assert!(matches!(
            written,
            CandidateUniverseProductionCommitV1::Written(_)
        ));
        assert_eq!(written.audit().receipt(), produced.receipt());
        let reused = produced
            .append_and_reopen(root.path(), bounds)
            .expect("an exact production retry reuses byte-identical rows and receipt");
        assert!(matches!(
            reused,
            CandidateUniverseProductionCommitV1::Reused(_)
        ));
        assert_eq!(reused.audit(), written.audit());

        let ledger = CandidateUniverseLedgerV1::open_read(root.path(), bounds)
            .expect("the exact completed Candidate ledger reopens read-only");
        let authenticated = ledger
            .complete_population_rows(&written.audit())
            .expect("the exact completed Candidate rows authenticate");
        let replay = fixture
            .source()
            .execution_v3_replay_authority(ladder, written.audit().receipt(), &authenticated)
            .expect("the retained source reproduces exact Runner terminal dispositions");
        let replay_retry = fixture
            .source()
            .execution_v3_replay_authority(ladder, written.audit().receipt(), &authenticated)
            .expect("the exact replay retry is byte-for-byte identical");
        assert_eq!(replay_retry, replay);
        assert_eq!(replay.receipt(), written.audit().receipt());

        let mut corrupt = authenticated.clone();
        corrupt
            .first_mut()
            .expect("the production fixture has Candidate rows")
            .canonical_record[0] ^= 1;
        assert!(
            fixture
                .source()
                .execution_v3_replay_authority(ladder, written.audit().receipt(), &corrupt,)
                .expect_err("a corrupt authenticated Candidate record must refuse")
                .contains("authenticated row")
        );

        let foreign_receipt = prepared(99).receipt;
        assert!(
            fixture
                .source()
                .execution_v3_replay_authority(ladder, foreign_receipt, &authenticated)
                .expect_err("a foreign Candidate Completion must refuse exact replay")
                .contains("committed descriptor")
        );

        let pre_admission =
            crate::pre_admission_data::produce_pre_admission_data_v1(&produced, &written)
                .expect("the exact durable Candidate production mints Pre-Admission data");
        assert_eq!(
            pre_admission.value().candidate_universe_id(),
            produced.receipt().universe_id()
        );
        assert_eq!(
            pre_admission.value().candidate_completion_digest(),
            produced.receipt().content_digest()
        );
        assert_eq!(
            pre_admission.value().candidate_row_count(),
            produced.receipt().row_count()
        );
        let pre_admission_bounds = crate::pre_admission_data::PreAdmissionDataBoundsV1::new(
            4,
            crate::pre_admission_data::PRE_ADMISSION_HEADER_BYTES_V1
                .checked_add(
                    crate::pre_admission_data::PRE_ADMISSION_RECORD_STRIDE_V1
                        .checked_mul(8)
                        .expect("four pair strides fit u64"),
                )
                .expect("fixture Pre-Admission file ceiling fits u64"),
        )
        .expect("fixture Pre-Admission bounds are explicit");
        let pre_written = pre_admission
            .append_and_reopen(root.path(), pre_admission_bounds)
            .expect("Pre-Admission Data writes Data then completion and reopens");
        assert!(matches!(
            pre_written,
            crate::pre_admission_data::PreAdmissionProductionCommitV1::Written(_)
        ));
        assert_eq!(
            pre_written.audit().value().authority_id(),
            pre_admission.value().authority_id()
        );
        let pre_reused = pre_admission
            .append_and_reopen(root.path(), pre_admission_bounds)
            .expect("exact Pre-Admission retry reuses the pair");
        assert!(matches!(
            pre_reused,
            crate::pre_admission_data::PreAdmissionProductionCommitV1::Reused(_)
        ));
        assert_eq!(pre_reused.audit(), pre_written.audit());

        let foreign = prepared(99);
        let foreign_commit =
            CandidateUniverseProductionCommitV1::Written(CandidateUniverseReopenAuditV1 {
                first_row: 0,
                receipt: foreign.receipt,
            });
        assert!(
            crate::pre_admission_data::produce_pre_admission_data_v1(&produced, &foreign_commit,)
                .expect_err("a foreign Candidate completion cannot mint Pre-Admission authority")
                .contains("does not belong")
        );
    }

    fn maximum_nontrivial_live_singleton_support(column: &Column) -> (u32, u64, usize) {
        let swept = u64::try_from(column.len()).expect("fixture column length fits u64");
        let mut best_bit = 0_u32;
        let mut best_support = 0_u64;
        let mut ties = 0_usize;
        for bit in 0..vocab::ConditionMask::BITS {
            if !vocab::table::LIVE.get(bit) {
                continue;
            }
            let support = column.bits().iter().fold(0_u64, |count, mask| {
                count.saturating_add(u64::from(mask.get(bit)))
            });
            if support == swept {
                // D-0080 removes always-true positions before k=1. Selecting
                // one as the threshold would therefore create a vacuous
                // depth-zero walk even though its raw support is maximal.
                continue;
            }
            if support > best_support {
                best_bit = bit;
                best_support = support;
                ties = 1;
            } else if support == best_support && support != 0 {
                ties = ties.saturating_add(1);
            }
        }
        assert!(
            best_support > 0,
            "the typed production fixture must derive at least one non-constant supported live condition"
        );
        (best_bit, best_support, ties)
    }

    #[test]
    fn opaque_production_source_refuses_foreign_policy_column_grid_calendar_feed_and_commit() {
        let fixture = ProductionFixture::new();
        let mut source = fixture.source();

        let original_daily = source.daily_reference;
        source.daily_reference.schema = source.daily_reference.schema.saturating_add(1);
        assert!(
            source
                .validate()
                .expect_err("a foreign daily schema must refuse")
                .contains("daily-reference schema")
        );
        source.daily_reference = original_daily;

        let original_span = source.requested_span;
        source.requested_span =
            RequestedSpanIdentityV1::new(2025, 2, 2025, 2).expect("foreign month is valid");
        assert!(
            source
                .validate()
                .expect_err("calendar coverage cannot bless another requested span")
                .contains("requested month span")
        );
        source.requested_span = original_span;

        let original_long = source.long_exit_grid;
        source.long_exit_grid = source.short_exit_grid;
        assert!(
            source
                .validate()
                .expect_err("the short grid cannot occupy the long capability slot")
                .contains("foreign")
        );
        source.long_exit_grid = original_long;

        let original_series = source.execution_series;
        source.execution_series = ExecutionSeriesV1::new(
            &fixture.instrument,
            "foreign-feed",
            TEST_COMMIT,
            crate::stored::calendar_policy_digest_v2(),
            fixture.execution(),
        )
        .expect("foreign feed remains a typed series");
        assert!(
            source
                .validate()
                .expect_err("a foreign feed must not borrow the resolved grids")
                .contains("feed, commit or calendar")
        );
        source.execution_series = ExecutionSeriesV1::new(
            &fixture.instrument,
            TEST_FEED,
            "foreign-commit",
            crate::stored::calendar_policy_digest_v2(),
            fixture.execution(),
        )
        .expect("foreign commit remains a typed series");
        assert!(
            source
                .validate()
                .expect_err("a foreign commit must not borrow the resolved grids")
                .contains("feed, commit or calendar")
        );
        source.execution_series = original_series;

        let original_column = source.execution_column.clone();
        source.execution_column = Column::default();
        assert!(
            source
                .validate()
                .expect_err("a foreign legacy column has no production evaluation identity")
                .contains("evaluation policy")
        );
        source.execution_column = original_column;

        let original_signal_bound = source.signal_load_bound;
        source.signal_load_bound = StoredSpanLoadBoundV1::new(
            original_signal_bound
                .max_records()
                .checked_sub(1)
                .expect("fixture signal bound exceeds one"),
        )
        .expect("reduced signal bound remains nonzero");
        assert!(
            source
                .validate()
                .expect_err("a load ceiling below exact signal count must refuse")
                .contains("exceeds exact stored-load ceiling")
        );
        source.signal_load_bound = original_signal_bound;
        source.validate().expect("restored source remains exact");
    }

    #[test]
    fn ledger_is_receipt_last_reopenable_paged_and_idempotent() {
        let root = test_dir();
        let prepared = prepared(30);
        let bounds = CandidateUniverseBoundsV1::new(32, 4).expect("fixture bounds are nonzero");
        let mut ledger =
            CandidateUniverseLedgerV1::open(root.path(), bounds).expect("empty ledger initializes");
        let written = ledger
            .append_complete(&prepared)
            .expect("first exact append writes");
        assert!(matches!(
            written,
            CandidateUniverseProductionCommitV1::Written(_)
        ));
        let audit = written.audit();
        assert_eq!(audit.first_row(), 0);
        assert_eq!(audit.row_count(), 4);
        assert_eq!(audit.content_digest(), prepared.receipt().content_digest());
        assert_eq!(audit.signal_stream(), prepared.receipt().signal_stream());
        let page = ledger
            .page(&audit.universe_id(), 1, 2)
            .expect("bounded physical seek succeeds");
        assert_eq!(page.rows(), &prepared.rows()[1..3]);
        let reused = ledger
            .append_complete(&prepared)
            .expect("identical retry reuses committed bytes");
        assert!(matches!(
            reused,
            CandidateUniverseProductionCommitV1::Reused(_)
        ));
        assert!(
            ledger
                .page(&audit.universe_id(), 0, MAX_CANDIDATE_PAGE_ROWS_V1 + 1,)
                .expect_err("unbounded page must refuse before allocation")
                .contains("exceeds")
        );
        drop(ledger);

        let reopened = CandidateUniverseLedgerV1::open_read(root.path(), bounds)
            .expect("fully synced receipt reopens read-only");
        assert_eq!(
            reopened
                .reopen_audit(&audit.universe_id())
                .expect("reopen generation checks pass"),
            Some(audit)
        );
        assert_eq!(
            reopened
                .page(&audit.universe_id(), 0, 4)
                .expect("reopened rows are seekable")
                .rows(),
            prepared.rows()
        );
        drop(reopened);
        assert!(
            CandidateUniverseLedgerV1::open_read(
                root.path(),
                CandidateUniverseBoundsV1::new(3, 4).expect("bounds are nonzero"),
            )
            .expect_err("physical rows above configured bound must refuse")
            .contains("above configured maximum")
        );
    }

    #[test]
    fn complete_population_rows_returns_only_the_exact_audited_canonical_family() {
        let root = test_dir();
        let prepared = prepared(32);
        let bounds = CandidateUniverseBoundsV1::new(32, 4).expect("fixture bounds are nonzero");
        let mut ledger = CandidateUniverseLedgerV1::open(root.path(), bounds)
            .expect("complete-read fixture ledger initializes");
        let audit = ledger
            .append_complete(&prepared)
            .expect("complete-read fixture commits")
            .audit();
        let authenticated = ledger
            .complete_population_rows(&audit)
            .expect("the exact indexed audit recovers opaque Population inputs");
        assert_eq!(authenticated.len(), prepared.rows().len());

        let base_root = test_dir();
        let base_bounds =
            BaseEvidenceLedgerBoundsV2::new(32, 4).expect("fixture Base bounds are nonzero");
        let base = fixture_prepared_base_evidence_v2(prepared.receipt(), prepared.rows())
            .expect("fixture Base family is canonical");
        append_and_reopen_base_evidence_v2(base_root.path(), base_bounds, &audit, &base)
            .expect("fixture Base family commits against the same Candidate audit");
        let mut base_reader = BaseEvidenceLedgerReaderV2::open(base_root.path(), base_bounds)
            .expect("fixture Base reader opens");
        let base_audit = base_reader
            .audit(audit.universe_id())
            .expect("fixture Base audit generation checks pass")
            .expect("fixture Base completion exists");

        for (sequence, (projection, expected)) in authenticated
            .iter()
            .zip(prepared.rows().iter().copied())
            .enumerate()
        {
            let canonical = expected
                .record()
                .expect("the prepared Candidate row re-encodes canonically");
            assert_eq!(projection.row(), expected);
            assert_eq!(projection.canonical_record(), &canonical);
            assert_eq!(
                projection.base_candidate_row_digest(),
                candidate_row_digest_v2(&canonical)
            );
            let sequence = u64::try_from(sequence).expect("fixture sequence fits u64");
            assert_eq!(
                projection.base_candidate_row_digest(),
                base_reader
                    .record(&base_audit, sequence)
                    .expect("the paired Base row reads at its fixed offset")
                    .candidate_row_digest(),
                "opaque Population row {sequence} must carry the exact Base V2 Candidate-row identity"
            );
        }
        let forged = CandidateUniverseReopenAuditV1 {
            first_row: audit
                .first_row()
                .checked_add(1)
                .expect("fixture first-row offset fits"),
            receipt: audit.receipt(),
        };
        let refused = ledger.complete_population_rows(&forged);
        assert!(
            matches!(&refused, Err(why) if why.contains("not the exact indexed completion")),
            "a caller-altered audit must return only an error, never a partial row family: {refused:?}"
        );

        drop(ledger);
        let reopened = CandidateUniverseLedgerV1::open_read(root.path(), bounds)
            .expect("the committed fixture reopens read-only");
        let reopened_rows = reopened
            .complete_population_rows(&audit)
            .expect("a read-only authority recovers the same opaque family");
        assert!(
            reopened_rows
                .iter()
                .map(AuthenticatedCandidatePopulationRowV1::row)
                .eq(prepared.rows().iter().copied())
        );
    }

    #[test]
    fn complete_population_rows_accepts_an_exact_zero_row_completion() {
        let root = test_dir();
        let prepared = zero_prepared(33);
        let bounds = CandidateUniverseBoundsV1::new(32, 4).expect("fixture bounds are nonzero");
        let mut ledger = CandidateUniverseLedgerV1::open(root.path(), bounds)
            .expect("zero-row complete-read ledger initializes");
        let audit = ledger
            .append_complete(&prepared)
            .expect("zero-row completion commits receipt-last")
            .audit();
        assert_eq!(audit.row_count(), 0);
        assert_eq!(
            ledger
                .complete_population_rows(&audit)
                .expect("the exact empty Population projection is complete"),
            Vec::<AuthenticatedCandidatePopulationRowV1>::new()
        );
    }

    #[test]
    fn complete_population_rows_refuses_stale_replaced_and_corrupt_storage() {
        let bounds = CandidateUniverseBoundsV1::new(32, 4).expect("fixture bounds are nonzero");

        let stale_root = test_dir();
        let stale_prepared = prepared(34);
        let mut stale = CandidateUniverseLedgerV1::open(stale_root.path(), bounds)
            .expect("stale complete-read ledger initializes");
        let stale_audit = stale
            .append_complete(&stale_prepared)
            .expect("stale complete-read fixture commits")
            .audit();
        let stale_row_path = stale_root.path().join(ROW_FILE);
        let mut external =
            open_file(&stale_row_path, true, false).expect("external stale-row mutator opens");
        let changed_offset = HEADER_BYTES_V1 + 248;
        external
            .seek(SeekFrom::Start(changed_offset))
            .expect("stale row mutation seeks");
        let mut changed = [0_u8; 1];
        external
            .read_exact(&mut changed)
            .expect("stale row byte reads");
        changed[0] ^= 1;
        external
            .seek(SeekFrom::Start(changed_offset))
            .and_then(|_| external.write_all(&changed))
            .and_then(|()| external.sync_data())
            .expect("stale row mutation persists");
        drop(external);
        let stale_result = stale.complete_population_rows(&stale_audit);
        assert!(
            matches!(&stale_result, Err(why) if why.contains("changed")),
            "a stale generation returns no partial rows: {stale_result:?}"
        );
        drop(stale);
        assert!(
            CandidateUniverseLedgerV1::open_read(stale_root.path(), bounds)
                .expect_err("the same corrupt row must fail a fresh structural scan")
                .contains("seal")
        );

        #[cfg(unix)]
        {
            let replaced_root = test_dir();
            let replaced_prepared = prepared(35);
            let mut replaced = CandidateUniverseLedgerV1::open(replaced_root.path(), bounds)
                .expect("replacement complete-read ledger initializes");
            let replaced_audit = replaced
                .append_complete(&replaced_prepared)
                .expect("replacement complete-read fixture commits")
                .audit();
            let named_path = replaced_root.path().join(ROW_FILE);
            let displaced = replaced_root.path().join("candidate-rows.displaced");
            std::fs::rename(&named_path, &displaced)
                .expect("the held candidate row inode is displaced");
            File::create(&named_path).expect("a replacement candidate row inode is created");
            let replaced_result = replaced.complete_population_rows(&replaced_audit);
            assert!(
                matches!(&replaced_result, Err(why) if why.contains("no longer names")),
                "a replaced row path returns no partial rows: {replaced_result:?}"
            );
        }
    }

    #[test]
    fn ledger_refuses_missing_or_non_directory_root_without_manufacturing_it() {
        let parent = test_dir();
        let missing = parent.path().join("detached-candidate-volume");
        let bounds = CandidateUniverseBoundsV1::new(32, 4).expect("fixture bounds are nonzero");
        assert!(
            CandidateUniverseLedgerV1::open(&missing, bounds)
                .expect_err("an absent configured root must refuse")
                .contains("must already exist")
        );
        assert!(
            !missing.exists(),
            "candidate writer must not recreate a detached volume path"
        );

        let file_root = parent.path().join("candidate-root-file");
        File::create(&file_root).expect("regular-file root fixture is created");
        assert!(
            CandidateUniverseLedgerV1::open(&file_root, bounds)
                .expect_err("a regular file cannot be a candidate root")
                .contains("not a directory")
        );
    }

    #[test]
    fn exact_orphan_prefix_resumes_and_foreign_orphan_refuses() {
        let root = test_dir();
        let orphan_prepared = prepared(40);
        let bounds = CandidateUniverseBoundsV1::new(32, 4).expect("fixture bounds are nonzero");
        drop(
            CandidateUniverseLedgerV1::open(root.path(), bounds)
                .expect("ledger headers initialize"),
        );
        let row_path = root.path().join(ROW_FILE);
        let mut row_file = open_file(&row_path, true, false).expect("row file reopens");
        append_rows(&mut row_file, &orphan_prepared.rows()[..2]).expect("orphan prefix writes");
        row_file.sync_data().expect("orphan prefix syncs");
        drop(row_file);
        let mut reopened = CandidateUniverseLedgerV1::open(root.path(), bounds)
            .expect("one exact orphan identity is recoverable");
        assert!(matches!(
            reopened
                .append_complete(&orphan_prepared)
                .expect("exact orphan retry appends only the suffix and receipt"),
            CandidateUniverseProductionCommitV1::Written(_)
        ));
        assert_eq!(
            reopened
                .page(&orphan_prepared.receipt().universe_id(), 0, 4)
                .expect("completed orphan rows are readable")
                .rows(),
            orphan_prepared.rows()
        );

        let foreign_root = test_dir();
        drop(
            CandidateUniverseLedgerV1::open(foreign_root.path(), bounds)
                .expect("foreign ledger headers initialize"),
        );
        let mut foreign_rows = open_file(&foreign_root.path().join(ROW_FILE), true, false)
            .expect("foreign row file reopens");
        append_rows(&mut foreign_rows, &orphan_prepared.rows()[..1])
            .expect("foreign orphan writes");
        foreign_rows.sync_data().expect("foreign orphan syncs");
        drop(foreign_rows);
        let mut foreign = CandidateUniverseLedgerV1::open(foreign_root.path(), bounds)
            .expect("one foreign orphan is indexed but not hidden");
        assert!(
            foreign
                .append_complete(&prepared(41))
                .expect_err("another universe cannot overwrite an orphan")
                .contains("row tail belongs")
        );
    }

    #[test]
    fn ragged_corrupt_and_stale_files_fail_closed() {
        let bounds = CandidateUniverseBoundsV1::new(32, 4).expect("fixture bounds are nonzero");

        let ragged_root = test_dir();
        drop(
            CandidateUniverseLedgerV1::open(ragged_root.path(), bounds)
                .expect("ragged fixture headers initialize"),
        );
        let mut ragged = open_file(&ragged_root.path().join(ROW_FILE), true, false)
            .expect("ragged row file reopens");
        ragged.seek(SeekFrom::End(0)).expect("ragged append seeks");
        ragged.write_all(&[1]).expect("ragged byte writes");
        ragged.sync_data().expect("ragged byte syncs");
        drop(ragged);
        assert!(
            CandidateUniverseLedgerV1::open(ragged_root.path(), bounds)
                .expect_err("ragged fixed-stride file must refuse")
                .contains("ragged")
        );

        let corrupt_root = test_dir();
        let corrupt_prepared = prepared(50);
        let mut corrupt_ledger = CandidateUniverseLedgerV1::open(corrupt_root.path(), bounds)
            .expect("corrupt fixture ledger opens");
        corrupt_ledger
            .append_complete(&corrupt_prepared)
            .expect("corrupt fixture first commits cleanly");
        drop(corrupt_ledger);
        let row_path = corrupt_root.path().join(ROW_FILE);
        let mut corrupt = open_file(&row_path, true, false).expect("committed row file reopens");
        let fact_offset = HEADER_BYTES_V1 + 248;
        corrupt
            .seek(SeekFrom::Start(fact_offset))
            .expect("fact mutation seeks");
        let mut byte = [0_u8; 1];
        corrupt.read_exact(&mut byte).expect("fact byte reads");
        byte[0] ^= 1;
        corrupt
            .seek(SeekFrom::Start(fact_offset))
            .and_then(|_| corrupt.write_all(&byte))
            .and_then(|()| corrupt.sync_data())
            .expect("fact mutation persists");
        drop(corrupt);
        assert!(
            CandidateUniverseLedgerV1::open(corrupt_root.path(), bounds)
                .expect_err("corrupt row seal must refuse")
                .contains("seal")
        );

        let stale_root = test_dir();
        let mut stale = CandidateUniverseLedgerV1::open(stale_root.path(), bounds)
            .expect("stale fixture ledger opens");
        let outcome = stale
            .append_complete(&prepared(51))
            .expect("stale fixture commits");
        let id = outcome.audit().universe_id();
        let mut external = open_file(&stale_root.path().join(ROW_FILE), true, false)
            .expect("external writer opens");
        let same_length_offset = HEADER_BYTES_V1 + 248;
        external
            .seek(SeekFrom::Start(same_length_offset))
            .expect("same-length mutation seeks");
        let mut changed = [0_u8; 1];
        external
            .read_exact(&mut changed)
            .expect("same-length mutation reads");
        changed[0] ^= 1;
        external
            .seek(SeekFrom::Start(same_length_offset))
            .and_then(|_| external.write_all(&changed))
            .and_then(|()| external.sync_data())
            .expect("same-length mutation persists");
        drop(external);
        assert!(
            stale
                .reopen_audit(&id)
                .expect_err("stale reopen audit must refuse changed generation")
                .contains("changed")
        );
        assert!(
            stale
                .page(&id, 0, 1)
                .expect_err("cached audit must refuse changed generation")
                .contains("changed")
        );
    }

    #[cfg(unix)]
    #[test]
    fn replaced_lock_row_and_receipt_paths_refuse_cached_audits() {
        let bounds = CandidateUniverseBoundsV1::new(32, 4).expect("fixture bounds are nonzero");
        for (tag, name) in [LOCK_FILE, ROW_FILE, RECEIPT_FILE].into_iter().enumerate() {
            let root = test_dir();
            let prepared =
                prepared(u8::try_from(60 + tag).expect("the three fixture tags fit in one byte"));
            let mut ledger = CandidateUniverseLedgerV1::open(root.path(), bounds)
                .expect("path-replacement fixture ledger opens");
            let audit = ledger
                .append_complete(&prepared)
                .expect("path-replacement fixture commits")
                .audit();
            let named_path = root.path().join(name);
            let displaced = root.path().join(format!("{name}.displaced"));
            std::fs::rename(&named_path, &displaced)
                .expect("held file inode can be renamed on Unix");
            File::create(&named_path).expect("replacement file inode is created");

            assert!(
                ledger
                    .reopen_audit(&audit.universe_id())
                    .expect_err("replacement path must invalidate the cached audit")
                    .contains("no longer names"),
                "replacement of {name} did not fail closed"
            );
        }
    }
}
