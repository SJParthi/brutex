//! Version-neutral stored post-training OOS authority for Global Replay.
//!
//! This module deliberately stops below any Selection or Global Replay layout.
//! A retained Step-3 stored transaction derives the feed, swept instrument,
//! signal rung, source commit, evaluator, dynamic Long/Short grids and training
//! boundary.  The caller supplies only a civil month range and three explicit
//! load ceilings.  The store then supplies the exact signal, previous-day and
//! one-minute OHLCV bytes plus their complete calendar receipts.
//!
//! The resulting capability owns those bytes and a held root identity.  It can
//! replay an opaque Runner [`ExecutionDispositionV1`] only when the OOS first
//! minute is strictly after the retained grid's training last minute and every
//! instrument/feed/commit/calendar/side/evaluator identity still agrees.  It
//! does not accept loose timestamps, feed labels, family labels, directions,
//! masks, exits or digests.
//!
//! This is intentionally version-neutral because terminal-aware Selection may
//! produce zero through twenty-five winners per rung.  A future Global Replay
//! V4 coordinator must preflight its actual opaque Selection V6 winner set
//! before invoking this mint; this module does not reinstate V3's fixed 8x25
//! assumption.

#![allow(
    dead_code,
    reason = "the stored OOS capability is the agreed Step-3 seam for the pending Selection V6 / Global Replay V4 coordinator"
)]

use std::ops::Range;

use brutex_core::blake3::{Hasher, hash};
use indicators::evaluator::Widths;
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use runner::exit_grid_policy::{
    ExecutionDispositionV1, ExecutionSeriesV1, GlobalReplayWitnessUniverseV1, ResolvedExitGridV1,
    instrument_digest_v1,
};
use runner::identity::{DailyReferenceBinding, ReferenceIntegrity};
use runner::outcome::Horizon;

use crate::candidate_universe::CandidateGlobalReplayOosSourceV1;
use crate::population::{InstrumentFamilyV1, RequestedSpanIdentityV1};
use crate::step3_orchestrator::{AdmittedRootV1, BoundedStoredContextV1, StoredLoadCeilingsV1};
use crate::stored::{CalendarReceiptV2, StoredSpanLoadBoundV1};

const COHORT_ID_DOMAIN: &[u8] = b"brutex-stored-post-training-oos-cohort-v1\0";
const WITNESS_ID_DOMAIN: &[u8] = b"brutex-stored-post-training-oos-witness-v1\0";

type StoredPostTrainingOosRefusal = String;

/// Caller-supplied resource request with no market identity or timestamp fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StoredPostTrainingOosRequestV1 {
    from: (u16, u8),
    to: (u16, u8),
    signal_records: StoredSpanLoadBoundV1,
    minute_records: StoredSpanLoadBoundV1,
    daily_records: StoredSpanLoadBoundV1,
}

impl StoredPostTrainingOosRequestV1 {
    pub(crate) fn new(
        from: (u16, u8),
        to: (u16, u8),
        signal_records: StoredSpanLoadBoundV1,
        minute_records: StoredSpanLoadBoundV1,
        daily_records: StoredSpanLoadBoundV1,
    ) -> Result<Self, StoredPostTrainingOosRefusal> {
        RequestedSpanIdentityV1::new(from.0, from.1, to.0, to.1)
            .map_err(|why| format!("stored post-training OOS span refused: {why}"))?;
        Ok(Self {
            from,
            to,
            signal_records,
            minute_records,
            daily_records,
        })
    }

    pub(crate) const fn from(self) -> (u16, u8) {
        self.from
    }

    pub(crate) const fn to(self) -> (u16, u8) {
        self.to
    }

    pub(crate) const fn signal_bound(self) -> StoredSpanLoadBoundV1 {
        self.signal_records
    }

    pub(crate) const fn minute_bound(self) -> StoredSpanLoadBoundV1 {
        self.minute_records
    }

    pub(crate) const fn daily_bound(self) -> StoredSpanLoadBoundV1 {
        self.daily_records
    }
}

/// Read-only identity of an immutable stored OOS cohort.
///
/// The projection intentionally does not expose feed, family, instrument or
/// direction as loose facts.  It is sufficient for a successor receipt to bind
/// the causal boundary and exact source content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StoredPostTrainingOosAuditV1 {
    cohort_id: [u8; 32],
    training_last_ts_micros: i64,
    oos_first_ts_micros: i64,
    oos_last_ts_micros: i64,
    execution_bars: u64,
}

impl StoredPostTrainingOosAuditV1 {
    #[must_use]
    pub(crate) const fn cohort_id(self) -> [u8; 32] {
        self.cohort_id
    }

    #[must_use]
    pub(crate) const fn training_last_ts_micros(self) -> i64 {
        self.training_last_ts_micros
    }

    #[must_use]
    pub(crate) const fn oos_first_ts_micros(self) -> i64 {
        self.oos_first_ts_micros
    }

    #[must_use]
    pub(crate) const fn oos_last_ts_micros(self) -> i64 {
        self.oos_last_ts_micros
    }

    #[must_use]
    pub(crate) const fn execution_bars(self) -> u64 {
        self.execution_bars
    }
}

/// One-use witness retaining the stored cohort identity beside Runner's opaque
/// replay universe.  A V4 writer must bind both; a raw Runner witness is not a
/// substitute for the stored-origin cohort receipt.
pub(crate) struct StoredPostTrainingOosWitnessV1 {
    cohort_id: [u8; 32],
    witness_id: [u8; 32],
    witness: GlobalReplayWitnessUniverseV1,
}

impl StoredPostTrainingOosWitnessV1 {
    #[must_use]
    pub(crate) const fn cohort_id(&self) -> [u8; 32] {
        self.cohort_id
    }

    #[must_use]
    pub(crate) const fn witness_id(&self) -> [u8; 32] {
        self.witness_id
    }

    /// Moves both opaque authorities together for the future V4 join.
    pub(crate) fn into_parts(self) -> ([u8; 32], [u8; 32], GlobalReplayWitnessUniverseV1) {
        (self.cohort_id, self.witness_id, self.witness)
    }
}

/// Immutable post-training stored snapshot paired with the retained training
/// policy that is allowed to replay it.
pub(crate) struct StoredPostTrainingOosCohortV1 {
    root: AdmittedRootV1,
    stored: BoundedStoredContextV1,
    execution_range: Range<usize>,
    source_commit: String,
    family: InstrumentFamilyV1,
    ladder: engine::Ladder,
    horizon: Horizon,
    widths: Widths,
    availability: Availability,
    thresholds: Thresholds,
    load_ceilings: StoredLoadCeilingsV1,
    long: ResolvedExitGridV1,
    short: ResolvedExitGridV1,
    audit: StoredPostTrainingOosAuditV1,
}

impl StoredPostTrainingOosCohortV1 {
    #[expect(
        clippy::too_many_arguments,
        reason = "construction moves every retained training authority and exact stored OOS source into one nonconstructible cohort"
    )]
    pub(crate) fn from_retained(
        root: AdmittedRootV1,
        stored: BoundedStoredContextV1,
        execution_range: Range<usize>,
        source_commit: String,
        family: InstrumentFamilyV1,
        ladder: engine::Ladder,
        horizon: Horizon,
        widths: Widths,
        availability: Availability,
        thresholds: Thresholds,
        load_ceilings: StoredLoadCeilingsV1,
        long: ResolvedExitGridV1,
        short: ResolvedExitGridV1,
    ) -> Result<Self, StoredPostTrainingOosRefusal> {
        let training_last = require_same_training_source(&long, &short)?;
        let (oos_first_ts_micros, oos_last_ts_micros, execution_bars) = {
            let execution = stored
                .minute
                .bars
                .get(execution_range.clone())
                .ok_or_else(|| {
                    "stored OOS execution range escaped its owned minute context".to_owned()
                })?;
            let first = execution
                .first()
                .ok_or_else(|| "stored post-training OOS execution cohort is empty".to_owned())?;
            let last = execution.last().ok_or_else(|| {
                "stored post-training OOS execution cohort lost its last bar".to_owned()
            })?;
            (
                first.ts_micros,
                last.ts_micros,
                usize_to_u64(execution.len(), "execution bars")?,
            )
        };
        if oos_first_ts_micros <= training_last {
            return Err(format!(
                "stored OOS first timestamp {oos_first_ts_micros} is not after training last {training_last}"
            ));
        }
        let mut cohort = Self {
            root,
            stored,
            execution_range,
            source_commit,
            family,
            ladder,
            horizon,
            widths,
            availability,
            thresholds,
            load_ceilings,
            long,
            short,
            audit: StoredPostTrainingOosAuditV1 {
                cohort_id: [0; 32],
                training_last_ts_micros: training_last,
                oos_first_ts_micros,
                oos_last_ts_micros,
                execution_bars,
            },
        };
        cohort.audit.cohort_id = cohort.derive_cohort_id()?;
        cohort.require_integrity()?;
        Ok(cohort)
    }

    #[must_use]
    pub(crate) const fn audit(&self) -> StoredPostTrainingOosAuditV1 {
        self.audit
    }

    fn require_integrity(&self) -> Result<(), StoredPostTrainingOosRefusal> {
        self.root
            .require_same("while authenticating stored post-training OOS cohort")?;
        let training_last = require_same_training_source(&self.long, &self.short)?;
        let execution = self.execution()?;
        let first = execution
            .first()
            .ok_or_else(|| "stored post-training OOS execution cohort is empty".to_owned())?;
        let last = execution.last().ok_or_else(|| {
            "stored post-training OOS execution cohort lost its last bar".to_owned()
        })?;
        if training_last != self.audit.training_last_ts_micros
            || first.ts_micros != self.audit.oos_first_ts_micros
            || last.ts_micros != self.audit.oos_last_ts_micros
            || usize_to_u64(execution.len(), "execution bars")? != self.audit.execution_bars
            || first.ts_micros <= training_last
            || self.audit.cohort_id != self.derive_cohort_id()?
        {
            return Err("stored post-training OOS cohort identity changed".to_owned());
        }
        Ok(())
    }

    fn execution(&self) -> Result<&[indicators::Candle], StoredPostTrainingOosRefusal> {
        self.stored
            .minute
            .bars
            .get(self.execution_range.clone())
            .ok_or_else(|| "stored OOS execution range escaped its owned minute context".to_owned())
    }

    /// Mints one stored-origin witness from an opaque authorized Execution
    /// disposition.  The future V4 coordinator is responsible for invoking
    /// this only after its complete actual Selection V6 winner set preflights.
    pub(crate) fn mint_witness(
        &self,
        disposition: &ExecutionDispositionV1,
    ) -> Result<StoredPostTrainingOosWitnessV1, StoredPostTrainingOosRefusal> {
        self.require_integrity()?;
        let execution = self.execution()?;
        let execution_calendar = crate::stored::calendar_receipt_v2_for_bars(
            execution,
            60,
            self.stored.first_day,
            self.stored.last_day,
        )
        .and_then(CalendarReceiptV2::require_complete)
        .map_err(|why| format!("stored OOS execution calendar refused: {why}"))?;
        let series = ExecutionSeriesV1::new(
            &self.stored.signal.key,
            self.stored.signal.vendor.as_str(),
            &self.source_commit,
            crate::stored::calendar_policy_digest_v2(),
            execution,
        )
        .map_err(|why| format!("stored OOS execution series refused: {why}"))?;
        let daily_reference = self.daily_reference();
        let source = CandidateGlobalReplayOosSourceV1::new(
            self.family,
            self.stored.rung_seconds,
            self.horizon,
            self.stored.requested_span,
            self.stored.signal_calendar,
            execution_calendar,
            &self.stored.signal.bars,
            &self.stored.daily.references,
            &self.stored.minute.bars,
            daily_reference,
            series,
            self.widths,
            self.availability,
            self.thresholds,
            StoredSpanLoadBoundV1::new(self.load_ceilings.signal)
                .map_err(|why| format!("stored OOS signal ceiling refused: {why}"))?,
            StoredSpanLoadBoundV1::new(self.load_ceilings.minute)
                .map_err(|why| format!("stored OOS minute ceiling refused: {why}"))?,
            StoredSpanLoadBoundV1::new(self.load_ceilings.daily)
                .map_err(|why| format!("stored OOS daily ceiling refused: {why}"))?,
        )?;
        let resolved = match disposition.side() {
            runner::excursion::Side::Long => &self.long,
            runner::excursion::Side::Short => &self.short,
        };
        let witness = source.mint_witness(self.ladder, resolved, disposition)?;
        witness
            .require_integrity()
            .map_err(|why| format!("stored OOS Runner witness integrity refused: {why}"))?;
        self.root
            .require_same("after minting stored post-training OOS witness")?;
        self.require_integrity()?;
        let run_id = witness.run_id().bytes();
        let selected_exit_digest = witness.selected_exit_digest();
        let universe_digest = witness.universe_digest();
        let witness_id = hash_parts(
            WITNESS_ID_DOMAIN,
            &[
                &self.audit.cohort_id,
                &run_id,
                &selected_exit_digest,
                &universe_digest,
            ],
        );
        Ok(StoredPostTrainingOosWitnessV1 {
            cohort_id: self.audit.cohort_id,
            witness_id,
            witness,
        })
    }

    fn derive_cohort_id(&self) -> Result<[u8; 32], StoredPostTrainingOosRefusal> {
        let execution = self.execution()?;
        let execution_calendar = crate::stored::calendar_receipt_v2_for_bars(
            execution,
            60,
            self.stored.first_day,
            self.stored.last_day,
        )
        .and_then(CalendarReceiptV2::require_complete)
        .map_err(|why| format!("stored OOS execution calendar refused: {why}"))?;
        let daily_reference = self.daily_reference();
        let exact_data_digest = runner::identity::data_digest_with_daily_reference(
            &self.stored.signal.bars,
            &self.stored.minute.bars,
            daily_reference,
        )
        .map_err(|why| format!("stored OOS exact data identity refused: {why:?}"))?;
        let mut hasher = Hasher::new();
        hasher.update(COHORT_ID_DOMAIN);
        hasher.update(&self.root.generation_bytes());
        hasher.update(&[match self.family {
            InstrumentFamilyV1::Nifty => 1,
            InstrumentFamilyV1::BankNifty => 2,
        }]);
        hasher.update(&self.stored.rung_seconds.to_le_bytes());
        hasher.update(&self.horizon.as_bars().to_le_bytes());
        hasher.update(&self.stored.requested_span.canonical_bytes());
        hasher.update(&hash(self.stored.signal.vendor.as_str().as_bytes()));
        hasher.update(&instrument_digest_v1(&self.stored.signal.key));
        hasher.update(&hash(self.source_commit.as_bytes()));
        hasher.update(&crate::stored::calendar_policy_digest_v2());
        hasher.update(&self.stored.signal_calendar.digest());
        hasher.update(&execution_calendar.digest());
        hasher.update(
            &crate::stored_data_completeness::daily_reference_policy_digest_v1(daily_reference),
        );
        hasher.update(&exact_data_digest);
        hasher.update(&runner::identity::data_digest(&self.stored.signal.bars));
        hasher.update(&runner::identity::data_digest(&self.stored.daily.bars));
        hasher.update(&runner::identity::data_digest(&self.stored.minute.bars));
        hasher.update(&runner::identity::data_digest(execution));
        hasher.update(&self.long.digest());
        hasher.update(&self.short.digest());
        hasher.update(&self.long.training_digest());
        hasher.update(&self.long.training_last_ts_micros().to_le_bytes());
        hasher.update(&self.audit.oos_first_ts_micros.to_le_bytes());
        hasher.update(&self.audit.oos_last_ts_micros.to_le_bytes());
        hasher.update(&self.audit.execution_bars.to_le_bytes());
        hasher.update(&self.load_ceilings.signal.to_le_bytes());
        hasher.update(&self.load_ceilings.minute.to_le_bytes());
        hasher.update(&self.load_ceilings.daily.to_le_bytes());
        Ok(hasher.finalize())
    }

    fn daily_reference(&self) -> DailyReferenceBinding<'_> {
        DailyReferenceBinding {
            daily_bars: &self.stored.daily.bars,
            eligibility: &self.stored.daily.eligibility,
            schema: crate::stored::DAILY_REFERENCE_SCHEMA,
            eligibility_policy: crate::stored::DAILY_ELIGIBILITY_POLICY,
            gap_overlay_policy: crate::stored::EXACT_MINUTE_GAP_POLICY,
            excluded_ist_days: &indicators::evaluator::CHARTER_NON_REGULAR_IST_DAYS,
            daily_integrity: ReferenceIntegrity::UnverifiedNoReceipt,
            minute_integrity: ReferenceIntegrity::UnverifiedNoReceipt,
        }
    }
}

fn require_same_training_source(
    long: &ResolvedExitGridV1,
    short: &ResolvedExitGridV1,
) -> Result<i64, StoredPostTrainingOosRefusal> {
    if !long.digest_is_valid()
        || !short.digest_is_valid()
        || long.side() != runner::excursion::Side::Long
        || short.side() != runner::excursion::Side::Short
        || long.instrument() != short.instrument()
        || long.feed_digest() != short.feed_digest()
        || long.commit_digest() != short.commit_digest()
        || long.calendar_digest() != short.calendar_digest()
        || long.training_digest() != short.training_digest()
        || long.training_bars() != short.training_bars()
        || long.training_first_ts_micros() != short.training_first_ts_micros()
        || long.training_last_ts_micros() != short.training_last_ts_micros()
    {
        return Err("stored OOS Long/Short training authorities differ or are torn".to_owned());
    }
    Ok(long.training_last_ts_micros())
}

fn usize_to_u64(value: usize, name: &str) -> Result<u64, StoredPostTrainingOosRefusal> {
    u64::try_from(value).map_err(|_| format!("stored OOS {name} does not fit u64"))
}

fn hash_parts(domain: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    for part in parts {
        hasher.update(&u64::try_from(part.len()).unwrap_or(u64::MAX).to_le_bytes());
        hasher.update(part);
    }
    hasher.finalize()
}
