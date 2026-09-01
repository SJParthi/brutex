//! Causal previous-day references supplied as sealed one-day bars.
//!
//! The ordinary [`crate::Evaluator`] derives yesterday from the signal series
//! itself.  That is correct when the signal series is one-minute, but it makes a
//! 60-minute sweep call one 60-minute session "the previous day".  This module
//! is the distinct stored-daily path: a caller supplies completed one-day bars,
//! marks which are eligible anchors, and the evaluator consumes only records
//! whose IST day is strictly before the signal bar's IST day.
//!
//! No store or filesystem type appears here.  [`DailyReference`] means the
//! caller has already proved that one [`crate::Candle`] is a sealed one-day
//! record; this crate can validate OHLCV and ordering, but it cannot infer a
//! timeframe or know whether an upstream file is still being written.
//!
//! # Join and cost
//!
//! Both inputs are ordered.  One monotonic cursor advances through daily bars,
//! and each daily bar is consumed at most once.  State is fixed-size apart from
//! the two borrowed input slices: total work is `O(signal + daily)`, while every
//! bar after the cursor is positioned reads the current reference in `O(1)`.
//! A calendar gap performs no search and needs no fabricated intermediate day.
//! This shape is **UNVERIFIED as a measured bound** until the indicators ratio
//! bench gains an anchored signal/daily scaling row.
//!
//! # Exactly which families are externally anchored
//!
//! * [`crate::daily::DailyLevels`] and [`crate::fib::prev_day_bits`] use the
//!   last eligible daily high, low and close.
//! * [`crate::fib::Prev5`] advances once per eligible sealed daily record.
//! * [`crate::session::PreviousSession`] uses the same daily high, low and close
//!   for opening-gap and prior-range predicates.
//! * The full open/high/low/close/volume/range snapshot remains observable via
//!   [`AnchoredEvaluator::current_reference`], so a caller can audit the exact
//!   daily record even though the current vocabulary has no previous-day-open
//!   or previous-day-volume bit.
//!
//! [`crate::gap::GapFib`] is deliberately NOT changed.  Its specification is
//! the last three intraday bars of the signal stream against today's first
//! three, not daily OHLC.  [`GapReferenceSource`] exposes that boundary so a
//! caller cannot mistake this path for an external `GapFib` anchor.

use crate::daily::{DailyLevels, Unusable};
use crate::evaluator::{Calendar, Evaluator, Widths};
use crate::gap::GapFib;
use crate::pattern::Thresholds;
use crate::vwap::Availability;
use crate::{Candle, Corrupt};
use vocab::ConditionMask;

const MINUTE_MICROS: i64 = 60_000_000;

/// Whether a sealed daily record is allowed to become a reference.
///
/// The caller owns this decision.  In particular, Muhurat and disaster-recovery
/// sessions are excluded here, while a full Budget weekend session may remain
/// eligible.  Deriving eligibility from weekday would be wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DailyEligibility {
    /// A regular completed session that may feed every previous-day family.
    Eligible,
    /// A real daily record that remains in history but may not become an anchor.
    Excluded,
}

/// Why one proposed daily record cannot enter an anchored series.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DailyReferenceRefusal {
    /// The OHLCV record is not a candle.
    Corrupt(Corrupt),
    /// OHLCV is valid, but its H/L/C cannot produce the declared pivot ladder.
    Unusable(Unusable),
}

/// The validated payload encoded without an impossible `Option` arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Payload {
    Eligible(DailyLevels),
    Excluded,
}

/// One caller-certified sealed one-day OHLCV record.
///
/// Fields are private so an eligible instance always carries the
/// [`DailyLevels`] built from the same immutable OHLC.  This removes any
/// possibility that cursor advancement succeeds while level construction
/// silently fails later.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DailyReference {
    bar: Candle,
    day: i64,
    range: i64,
    payload: Payload,
}

impl DailyReference {
    /// Validate and tag one caller-certified sealed daily bar.
    ///
    /// # Errors
    ///
    /// [`DailyReferenceRefusal::Corrupt`] for malformed OHLCV, or
    /// [`DailyReferenceRefusal::Unusable`] when an eligible bar cannot produce
    /// all previous-day levels.  An excluded record need not produce levels,
    /// because no family is allowed to consume it.
    pub fn new(bar: Candle, eligibility: DailyEligibility) -> Result<Self, DailyReferenceRefusal> {
        bar.check().map_err(DailyReferenceRefusal::Corrupt)?;
        let range = bar
            .range()
            .ok_or(DailyReferenceRefusal::Corrupt(Corrupt::RangeOverflows))?;
        let payload = match eligibility {
            DailyEligibility::Eligible => Payload::Eligible(
                DailyLevels::from_daily_bar(&bar).map_err(DailyReferenceRefusal::Unusable)?,
            ),
            DailyEligibility::Excluded => Payload::Excluded,
        };
        Ok(Self {
            day: crate::ist_day(bar.ts_micros),
            bar,
            range,
            payload,
        })
    }

    /// The immutable daily OHLCV record.
    #[must_use]
    pub const fn bar(&self) -> Candle {
        self.bar
    }

    /// Its IST calendar day.
    #[must_use]
    pub const fn ist_day(&self) -> i64 {
        self.day
    }

    /// Its validated high-low range in paisa.
    #[must_use]
    pub const fn range(&self) -> i64 {
        self.range
    }

    /// The caller's explicit eligibility decision.
    #[must_use]
    pub const fn eligibility(&self) -> DailyEligibility {
        match self.payload {
            Payload::Eligible(_) => DailyEligibility::Eligible,
            Payload::Excluded => DailyEligibility::Excluded,
        }
    }
}

/// A series-level ordering refusal, reported before any signal is evaluated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DailySeriesRefusal {
    /// UTC open timestamps are not strictly increasing.
    TimestampNotIncreasing {
        /// The later slice position that failed.
        index: usize,
        /// Timestamp immediately before it.
        previous: i64,
        /// Timestamp at `index`.
        current: i64,
    },
    /// Two records claim the same IST day; a last-one-wins choice is forbidden.
    DuplicateIstDay {
        /// The second occurrence.
        index: usize,
        /// The duplicated IST day.
        day: i64,
    },
}

/// The full daily OHLCV record currently anchoring the signal day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DailySnapshot {
    /// IST day of the sealed reference.
    pub ist_day: i64,
    /// UTC timestamp carried by the daily record.
    pub ts_micros: i64,
    /// Previous-day open, in paisa.
    pub open: i64,
    /// Previous-day high, in paisa.
    pub high: i64,
    /// Previous-day low, in paisa.
    pub low: i64,
    /// Previous-day close, in paisa.
    pub close: i64,
    /// Previous-day volume.
    pub volume: i64,
    /// Previous-day open interest, or [`crate::OI_NULL`].
    pub open_interest: i64,
    /// Exact `high - low`, in paisa.
    pub range: i64,
}

impl From<&DailyReference> for DailySnapshot {
    fn from(reference: &DailyReference) -> Self {
        let bar = reference.bar;
        Self {
            ist_day: reference.day,
            ts_micros: bar.ts_micros,
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
            volume: bar.volume,
            open_interest: bar.open_interest,
            range: reference.range,
        }
    }
}

/// Deterministic accounting for the causal daily join.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DailyReferenceCensus {
    /// Validated records supplied at construction.
    pub offered: u64,
    /// Records the caller marked eligible.
    pub eligible: u64,
    /// Records the caller explicitly excluded.
    pub excluded: u64,
    /// Records whose day became strictly earlier than an accepted signal day.
    pub consumed: u64,
    /// Consumed records installed as an anchor and pushed into Prev5.
    pub installed: u64,
    /// Consumed records skipped by caller eligibility.
    pub skipped_excluded: u64,
    /// Successfully evaluated signal bars with a daily reference.
    pub signal_bars_with_reference: u64,
    /// Successfully evaluated signal bars before any eligible prior daily bar.
    pub signal_bars_without_reference: u64,
    /// Distinct accepted signal days with a reference.
    pub signal_days_with_reference: u64,
    /// Distinct accepted signal days with no reference.
    pub signal_days_without_reference: u64,
}

impl DailyReferenceCensus {
    /// Daily and signal partitions both balance without a residual bucket.
    #[must_use]
    pub const fn reconciles(&self) -> bool {
        self.offered == self.eligible.saturating_add(self.excluded)
            && self.consumed == self.installed.saturating_add(self.skipped_excluded)
            && self.consumed <= self.offered
            && self.installed <= self.eligible
            && self.skipped_excluded <= self.excluded
    }

    /// References that were still same-day or future at the last accepted bar.
    #[must_use]
    pub const fn remaining(&self) -> u64 {
        self.offered.saturating_sub(self.consumed)
    }
}

/// The explicit `GapFib` boundary on this path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GapReferenceSource {
    /// Existing semantics: signal stream's last three bars versus today's first three.
    SignalSeriesLastThreeIntradayBars,
}

/// An evaluator whose previous-day families are driven by sealed daily OHLCV.
///
/// The reference slice is borrowed and never changed.  This type is `Copy` so
/// [`Self::step`] can commit both the cursor and the underlying evaluator only
/// after the signal bar succeeds; a refused signal cannot consume a daily row.
#[derive(Clone, Copy, Debug)]
pub struct AnchoredEvaluator<'a> {
    evaluator: Evaluator,
    references: &'a [DailyReference],
    cursor: usize,
    current: Option<DailySnapshot>,
    last_signal_day: Option<i64>,
    census: DailyReferenceCensus,
}

// The borrowed slice is two machine words and all cursor/history state is
// fixed-size.  This ceiling catches an accidental per-bar collection on the
// anchored path at compile time.
const _: () = assert!(core::mem::size_of::<AnchoredEvaluator<'static>>() <= 2_048);

impl<'a> AnchoredEvaluator<'a> {
    /// Validate `references` and build an externally anchored evaluator.
    ///
    /// # Errors
    ///
    /// A series with non-increasing timestamps or duplicate IST days is
    /// refused before any signal state exists.
    pub fn new(
        widths: Widths,
        availability: Availability,
        thresholds: Thresholds,
        references: &'a [DailyReference],
    ) -> Result<Self, DailySeriesRefusal> {
        Self::with_calendar(
            widths,
            availability,
            thresholds,
            Calendar::charter(),
            references,
        )
    }

    /// The same path with an explicit signal-series non-regular calendar.
    ///
    /// The calendar still governs signal-local `GapFib` rollover.  Daily anchor
    /// eligibility comes only from each [`DailyReference`].
    ///
    /// # Errors
    ///
    /// See [`Self::new`].
    pub fn with_calendar(
        widths: Widths,
        availability: Availability,
        thresholds: Thresholds,
        calendar: Calendar,
        references: &'a [DailyReference],
    ) -> Result<Self, DailySeriesRefusal> {
        let mut census = DailyReferenceCensus {
            offered: u64::try_from(references.len()).unwrap_or(u64::MAX),
            ..DailyReferenceCensus::default()
        };
        let mut previous: Option<&DailyReference> = None;
        for (index, reference) in references.iter().enumerate() {
            if let Some(before) = previous {
                if reference.day == before.day {
                    return Err(DailySeriesRefusal::DuplicateIstDay {
                        index,
                        day: reference.day,
                    });
                }
                if reference.bar.ts_micros <= before.bar.ts_micros {
                    return Err(DailySeriesRefusal::TimestampNotIncreasing {
                        index,
                        previous: before.bar.ts_micros,
                        current: reference.bar.ts_micros,
                    });
                }
            }
            match reference.payload {
                Payload::Eligible(_) => census.eligible = census.eligible.saturating_add(1),
                Payload::Excluded => census.excluded = census.excluded.saturating_add(1),
            }
            previous = Some(reference);
        }
        Ok(Self {
            evaluator: Evaluator::with_external_daily_references(
                widths,
                availability,
                thresholds,
                calendar,
            ),
            references,
            cursor: 0,
            current: None,
            last_signal_day: None,
            census,
        })
    }

    /// Evaluate one signal bar with commit-on-success cursor semantics.
    ///
    /// # Errors
    ///
    /// The same [`Corrupt`] refusals as [`Evaluator::step`].  A refusal changes
    /// neither indicator state, the daily cursor, nor the census.
    pub fn step(&mut self, bar: &Candle) -> Result<ConditionMask, Corrupt> {
        self.step_with_warmth(bar).map(|(mask, _)| mask)
    }

    /// Step once and return whether the complete run was warm immediately
    /// after installing every eligible prior daily record but before folding
    /// this signal bar.
    pub(crate) fn step_with_warmth(
        &mut self,
        bar: &Candle,
    ) -> Result<(ConditionMask, bool), Corrupt> {
        let mut next = *self;
        let signal_day = crate::ist_day(bar.ts_micros);
        next.advance_before(signal_day);
        let warm = next.evaluator.warmed_up();
        let mask = next.evaluator.step(bar)?;
        next.charge_signal(signal_day);
        *self = next;
        Ok((mask, warm))
    }

    /// The currently installed eligible daily OHLCV, if one exists.
    #[must_use]
    pub const fn current_reference(&self) -> Option<DailySnapshot> {
        self.current
    }

    /// Accounting through the last successfully evaluated signal bar.
    #[must_use]
    pub const fn reference_census(&self) -> DailyReferenceCensus {
        self.census
    }

    /// Existing `GapFib` remains signal-local on this anchored path.
    #[must_use]
    pub const fn gap_reference_source(&self) -> GapReferenceSource {
        GapReferenceSource::SignalSeriesLastThreeIntradayBars
    }

    /// Completed eligible daily sessions held by the fixed Prev5 ring.
    #[must_use]
    pub const fn sessions_completed(&self) -> usize {
        self.evaluator.sessions_completed()
    }

    /// Whether one usable eligible daily anchor is installed.
    #[must_use]
    pub const fn has_yesterday(&self) -> bool {
        self.evaluator.has_yesterday()
    }

    /// The ordinary aggregate warm-up verdict under external daily state.
    #[must_use]
    pub fn warmed_up(&self) -> bool {
        self.evaluator.warmed_up()
    }

    /// The ordinary evaluator choices needed to recompute only the acceptance
    /// verdict on a one-minute execution slice.
    ///
    /// Daily references decide masks, never whether OHLCV is structurally a
    /// candle.  [`crate::column::Column::reproject_checked`] carries the signal
    /// masks unchanged and uses this fresh specification solely for its
    /// per-execution-bar acceptance bitmap; it does not reconstruct a daily
    /// anchor or recompute a signal condition.
    pub(crate) const fn acceptance_spec(&self) -> crate::evaluator::EvaluationSpec {
        self.evaluator.spec()
    }

    /// Advance the monotonic cursor only across days strictly before `signal_day`.
    fn advance_before(&mut self, signal_day: i64) {
        while let Some(reference) = self.references.get(self.cursor) {
            if reference.day >= signal_day {
                break;
            }
            self.cursor = self.cursor.saturating_add(1);
            self.census.consumed = self.census.consumed.saturating_add(1);
            match reference.payload {
                Payload::Eligible(levels) => {
                    self.evaluator.install_external_daily_reference(
                        reference.bar.high,
                        reference.bar.low,
                        reference.bar.close,
                        levels,
                    );
                    self.current = Some(DailySnapshot::from(reference));
                    self.census.installed = self.census.installed.saturating_add(1);
                }
                Payload::Excluded => {
                    self.census.skipped_excluded = self.census.skipped_excluded.saturating_add(1);
                }
            }
        }
    }

    /// Count one accepted signal bar after its reference has been fixed.
    fn charge_signal(&mut self, signal_day: i64) {
        let has_reference = self.current.is_some();
        if has_reference {
            self.census.signal_bars_with_reference =
                self.census.signal_bars_with_reference.saturating_add(1);
        } else {
            self.census.signal_bars_without_reference =
                self.census.signal_bars_without_reference.saturating_add(1);
        }
        if self.last_signal_day != Some(signal_day) {
            if has_reference {
                self.census.signal_days_with_reference =
                    self.census.signal_days_with_reference.saturating_add(1);
            } else {
                self.census.signal_days_without_reference =
                    self.census.signal_days_without_reference.saturating_add(1);
            }
            self.last_signal_day = Some(signal_day);
        }
    }
}

/// Accounting for the exact-minute `GapFib` overlay on one signal column.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExactMinuteGapCensus {
    /// Exact stored one-minute bars folded through [`GapFib`].
    pub minute_bars: u64,
    /// Warm signal-column rows offered for replacement.
    pub signal_rows: u64,
    /// Rows that found the exact minute ending at the signal close.
    pub overlaid: u64,
}

/// Why an exact-minute `GapFib` overlay could not be proved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExactMinuteGapRefusal {
    /// A signal bar duration must be a positive whole number of minutes.
    InvalidSignalLength(i64),
    /// Exact-minute evidence must be non-empty, increasing, and minute-grid aligned.
    MalformedMinuteCadence {
        /// The first position at which cadence could not be proved.
        index: usize,
    },
    /// A stored minute was not valid OHLCV.
    CorruptMinute {
        /// Position in the exact-minute evidence slice.
        index: usize,
        /// The bar-level refusal.
        why: Corrupt,
    },
    /// A column source did not index the signal slice it claims to describe.
    SignalSourceOutOfBounds {
        /// Position in the signal column.
        row: usize,
        /// Offered source index.
        source: usize,
    },
    /// No exact stored minute ended at this signal bar's close.
    MissingClosingMinute {
        /// Signal-slice index whose close could not be sourced.
        source: usize,
        /// Exact minute-open timestamp that should have carried the close.
        expected_ts_micros: i64,
    },
    /// The signal aggregate and its exact final minute disagree on the close.
    SignalCloseMismatch {
        /// Signal-slice index whose aggregate close disagreed.
        source: usize,
        /// Close carried by the signal rung.
        signal_close: i64,
        /// Close carried by the exact minute ending that signal bar.
        minute_close: i64,
    },
    /// The internal overlay was not parallel to the column it was built for.
    OverlayLengthMismatch,
}

/// Replace a signal column's `GapFib` family with exact stored one-minute evidence.
///
/// A signal bar stamped `t` covers `[t, t + signal_length)`. Its closing price
/// is therefore carried by the one-minute bar opening at
/// `t + signal_length - 1 minute`. The merge below maps only that exact
/// timestamp; a later available minute is never substituted. [`GapFib`] is
/// streamed over the whole supplied minute context first, so each mapped mask
/// holds only state that existed by that minute's close.
///
/// All non-GapFib signal bits remain untouched. Positions 132..=142 are cleared
/// from the signal-local evaluator and replaced as one family, so no coarse
/// OHLCV bit can survive beside the exact-minute result.
///
/// # Errors
///
/// [`ExactMinuteGapRefusal`] names malformed cadence/OHLCV, a caller-column
/// mismatch, or an absent exact closing minute. None degrades to coarse data.
pub fn overlay_exact_minute_gapfib(
    signal: &[Candle],
    exact_minute: &[Candle],
    signal_length_micros: i64,
    widths: Widths,
    calendar: Calendar,
    column: &mut crate::column::Column,
) -> Result<ExactMinuteGapCensus, ExactMinuteGapRefusal> {
    if signal_length_micros < MINUTE_MICROS || signal_length_micros.rem_euclid(MINUTE_MICROS) != 0 {
        return Err(ExactMinuteGapRefusal::InvalidSignalLength(
            signal_length_micros,
        ));
    }
    let Some(first) = exact_minute.first() else {
        return Err(ExactMinuteGapRefusal::MalformedMinuteCadence { index: 0 });
    };
    if first.ts_micros.rem_euclid(MINUTE_MICROS) != 0 {
        return Err(ExactMinuteGapRefusal::MalformedMinuteCadence { index: 0 });
    }

    let mut gap = GapFib::new();
    let mut minute_masks = Vec::with_capacity(exact_minute.len());
    for (index, bar) in exact_minute.iter().enumerate() {
        if index > 0 {
            let Some(previous) = exact_minute.get(index.saturating_sub(1)) else {
                return Err(ExactMinuteGapRefusal::MalformedMinuteCadence { index });
            };
            let delta = bar.ts_micros.saturating_sub(previous.ts_micros);
            if delta < MINUTE_MICROS || delta.rem_euclid(MINUTE_MICROS) != 0 {
                return Err(ExactMinuteGapRefusal::MalformedMinuteCadence { index });
            }
        }
        let mask = gap
            .step(bar, widths.fib, &calendar)
            .map_err(|why| ExactMinuteGapRefusal::CorruptMinute { index, why })?;
        minute_masks.push(mask);
    }

    let mut exact_by_source = Vec::with_capacity(signal.len());
    let mut cursor = 0_usize;
    for (source, signal_bar) in signal.iter().enumerate() {
        let expected = signal_bar
            .ts_micros
            .saturating_add(signal_length_micros)
            .saturating_sub(MINUTE_MICROS);
        while exact_minute
            .get(cursor)
            .is_some_and(|minute| minute.ts_micros < expected)
        {
            cursor = cursor.saturating_add(1);
        }
        let Some(minute) = exact_minute.get(cursor) else {
            return Err(ExactMinuteGapRefusal::MissingClosingMinute {
                source,
                expected_ts_micros: expected,
            });
        };
        if minute.ts_micros != expected
            || crate::ist_day(minute.ts_micros) != crate::ist_day(signal_bar.ts_micros)
        {
            return Err(ExactMinuteGapRefusal::MissingClosingMinute {
                source,
                expected_ts_micros: expected,
            });
        }
        if minute.close != signal_bar.close {
            return Err(ExactMinuteGapRefusal::SignalCloseMismatch {
                source,
                signal_close: signal_bar.close,
                minute_close: minute.close,
            });
        }
        let Some(mask) = minute_masks.get(cursor).copied() else {
            return Err(ExactMinuteGapRefusal::MalformedMinuteCadence { index: cursor });
        };
        exact_by_source.push(mask);
    }

    let mut exact_for_signal = Vec::with_capacity(column.len());
    for (row, &source) in column.sources().iter().enumerate() {
        let Some(mask) = exact_by_source.get(source).copied() else {
            return Err(ExactMinuteGapRefusal::SignalSourceOutOfBounds { row, source });
        };
        exact_for_signal.push(mask);
    }
    if !column.replace_gapfib(&exact_for_signal) {
        return Err(ExactMinuteGapRefusal::OverlayLengthMismatch);
    }
    let rows = u64::try_from(exact_for_signal.len()).unwrap_or(u64::MAX);
    Ok(ExactMinuteGapCensus {
        minute_bars: u64::try_from(exact_minute.len()).unwrap_or(u64::MAX),
        signal_rows: rows,
        overlaid: rows,
    })
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "test fixtures must fail loudly when their declared valid daily input is not valid"
)]
mod tests {
    use super::*;
    use crate::OI_NULL;
    use crate::column::AnchoredColumn;

    const DAY_MICROS: i64 = 86_400 * 1_000_000;
    const MINUTE_MICROS: i64 = 60 * 1_000_000;
    const OPEN_IST_MINUTE: i64 = 9 * 60 + 15;

    fn ts(day: i64, minute: i64) -> i64 {
        day.saturating_mul(DAY_MICROS)
            .saturating_add(minute.saturating_mul(MINUTE_MICROS))
            .saturating_sub(crate::IST_OFFSET_MICROS)
    }

    fn daily_bar(day: i64, base: i64) -> Candle {
        Candle::new(
            ts(day, 0),
            base,
            base.saturating_add(1_000),
            base.saturating_sub(1_000),
            base.saturating_add(200),
            10_000,
            OI_NULL,
        )
    }

    fn signal_bar(day: i64, minute: i64, price: i64) -> Candle {
        Candle::new(
            ts(day, minute),
            price,
            price.saturating_add(100),
            price.saturating_sub(100),
            price,
            0,
            OI_NULL,
        )
    }

    fn eligible(day: i64, base: i64) -> DailyReference {
        DailyReference::new(daily_bar(day, base), DailyEligibility::Eligible)
            .expect("fixture daily bar is usable")
    }

    fn excluded(day: i64, base: i64) -> DailyReference {
        DailyReference::new(daily_bar(day, base), DailyEligibility::Excluded)
            .expect("fixture excluded daily bar is still valid OHLCV")
    }

    fn evaluator(references: &[DailyReference]) -> AnchoredEvaluator<'_> {
        AnchoredEvaluator::new(
            Widths::pinned().expect("pinned widths are valid"),
            Availability::Absent,
            Thresholds::CLASSICAL,
            references,
        )
        .expect("fixture reference days are strictly increasing")
    }

    fn coarse_overlay_fixture() -> (Vec<Candle>, Vec<DailyReference>, Vec<Candle>) {
        let references = (995_i64..=1_007)
            .map(|day| eligible(day, 2_400_000_i64.saturating_add(day.saturating_mul(100))))
            .collect::<Vec<_>>();
        let mut signal = Vec::new();
        let mut minute = vec![
            signal_bar(999, 927, 2_499_700),
            signal_bar(999, 928, 2_499_800),
            signal_bar(999, 929, 2_499_900),
        ];
        for day in 1_000_i64..=1_008 {
            for offset in 0_i64..375 {
                let price = 2_500_000_i64
                    .saturating_add(day.saturating_mul(100))
                    .saturating_add(offset);
                minute.push(signal_bar(day, OPEN_IST_MINUTE + offset, price));
            }
            for slot in 0_i64..25 {
                let minute_offset = slot.saturating_mul(15);
                let price = 2_500_000_i64
                    .saturating_add(day.saturating_mul(100))
                    .saturating_add(minute_offset)
                    .saturating_add(14);
                signal.push(signal_bar(
                    day,
                    OPEN_IST_MINUTE.saturating_add(minute_offset),
                    price,
                ));
            }
        }
        (signal, references, minute)
    }

    fn anchor_positions(mask: ConditionMask) -> Vec<u16> {
        let mut positions = crate::daily::positions().to_vec();
        positions.extend(crate::fib::positions().iter().copied().take(16));
        positions.extend([48, 49, 50, 51, 66, 67, 68]);
        positions
            .into_iter()
            .filter(|position| mask.get(u32::from(*position)))
            .collect()
    }

    #[test]
    fn first_day_and_same_day_daily_are_explicitly_unanchored() {
        let references = [eligible(20_000, 2_500_000)];
        let mut diagnostic = evaluator(&references);
        let signal = signal_bar(20_000, OPEN_IST_MINUTE, 2_500_000);
        let mask = diagnostic.step(&signal).expect("signal is valid");
        assert_eq!(diagnostic.current_reference(), None);
        assert!(anchor_positions(mask).is_empty());
        assert_eq!(
            diagnostic.reference_census().signal_bars_without_reference,
            1
        );
        assert_eq!(
            diagnostic.reference_census().signal_days_without_reference,
            1
        );
        assert_eq!(diagnostic.reference_census().remaining(), 1);

        let mut strict = evaluator(&references);
        assert_eq!(
            AnchoredColumn::build_required(&[signal], &mut strict),
            Err(crate::column::MissingDailyReference {
                signal_bars: 1,
                signal_days: 1,
            })
        );
        assert_eq!(
            strict.reference_census().signal_bars_without_reference,
            0,
            "strict admission committed evaluator state on refusal"
        );
    }

    #[test]
    fn only_a_strictly_earlier_day_can_become_the_anchor() {
        let references = [
            eligible(20_000, 2_400_000),
            eligible(20_001, 7_000_000),
            eligible(20_002, 9_000_000),
        ];
        let mut evaluator = evaluator(&references);
        evaluator
            .step(&signal_bar(20_001, OPEN_IST_MINUTE, 2_401_000))
            .expect("signal is valid");
        assert_eq!(
            evaluator
                .current_reference()
                .map(|snapshot| snapshot.ist_day),
            Some(20_000)
        );
        assert_eq!(evaluator.reference_census().consumed, 1);
        assert_eq!(evaluator.reference_census().remaining(), 2);
    }

    #[test]
    fn weekend_and_month_boundary_gaps_need_no_fabricated_day() {
        let references = [eligible(19_996, 2_400_000), eligible(20_027, 2_500_000)];
        let mut weekend = evaluator(&references[..1]);
        weekend
            .step(&signal_bar(19_999, OPEN_IST_MINUTE, 2_401_000))
            .expect("Monday-like signal is valid");
        assert_eq!(
            weekend.current_reference().map(|snapshot| snapshot.ist_day),
            Some(19_996)
        );

        let mut month = evaluator(&references);
        month
            .step(&signal_bar(20_028, OPEN_IST_MINUTE, 2_501_000))
            .expect("next-month signal is valid");
        assert_eq!(
            month.current_reference().map(|snapshot| snapshot.ist_day),
            Some(20_027)
        );
        assert_eq!(month.reference_census().installed, 2);
    }

    #[test]
    fn caller_exclusion_preserves_the_last_eligible_regular_anchor() {
        let references = [eligible(20_000, 2_400_000), excluded(20_001, 8_000_000)];
        let mut evaluator = evaluator(&references);
        evaluator
            .step(&signal_bar(20_002, OPEN_IST_MINUTE, 2_401_000))
            .expect("signal is valid");
        assert_eq!(
            evaluator
                .current_reference()
                .map(|snapshot| snapshot.ist_day),
            Some(20_000),
            "the excluded session replaced the last eligible one"
        );
        let census = evaluator.reference_census();
        assert_eq!(
            (census.consumed, census.installed, census.skipped_excluded),
            (2, 1, 1)
        );
        assert!(census.reconciles());
    }

    #[test]
    fn duplicate_and_backward_daily_series_refuse_before_signal_state_exists() {
        let mut later_stamp = daily_bar(20_000, 2_500_000);
        later_stamp.ts_micros = ts(20_000, 1);
        let duplicate_later_stamp = DailyReference::new(later_stamp, DailyEligibility::Eligible)
            .expect("the second record is valid in isolation");
        let duplicate = [eligible(20_000, 2_400_000), duplicate_later_stamp];
        assert!(matches!(
            AnchoredEvaluator::new(
                Widths::pinned().expect("pinned widths are valid"),
                Availability::Absent,
                Thresholds::CLASSICAL,
                &duplicate,
            ),
            Err(DailySeriesRefusal::DuplicateIstDay {
                index: 1,
                day: 20_000
            })
        ));

        let backward = [eligible(20_001, 2_500_000), eligible(20_000, 2_400_000)];
        assert!(matches!(
            AnchoredEvaluator::new(
                Widths::pinned().expect("pinned widths are valid"),
                Availability::Absent,
                Thresholds::CLASSICAL,
                &backward,
            ),
            Err(DailySeriesRefusal::TimestampNotIncreasing { index: 1, .. })
        ));
    }

    #[test]
    fn malformed_or_unusable_daily_ohlcv_is_named_at_construction() {
        let inverted = Candle::new(ts(20_000, 0), 100, 90, 110, 100, 1, OI_NULL);
        assert_eq!(
            DailyReference::new(inverted, DailyEligibility::Eligible),
            Err(DailyReferenceRefusal::Corrupt(Corrupt::HighBelowLow))
        );
        let negative_volume = Candle::new(ts(20_000, 0), 100, 110, 90, 100, -1, OI_NULL);
        assert_eq!(
            DailyReference::new(negative_volume, DailyEligibility::Eligible),
            Err(DailyReferenceRefusal::Corrupt(Corrupt::NegativeVolume))
        );
        let outside = Candle::new(ts(20_000, 0), 100, 110, 90, 111, 1, OI_NULL);
        assert_eq!(
            DailyReference::new(outside, DailyEligibility::Eligible),
            Err(DailyReferenceRefusal::Corrupt(Corrupt::PriceOutsideRange))
        );
        let impossible_span = Candle::new(ts(20_000, 0), 0, i64::MAX, i64::MIN, 0, 1, OI_NULL);
        assert_eq!(
            DailyReference::new(impossible_span, DailyEligibility::Eligible),
            Err(DailyReferenceRefusal::Corrupt(Corrupt::RangeOverflows))
        );
        let level_overflow = Candle::new(ts(20_000, 0), 0, i64::MAX / 2, 0, 0, 1, OI_NULL);
        assert_eq!(
            DailyReference::new(level_overflow, DailyEligibility::Eligible),
            Err(DailyReferenceRefusal::Unusable(Unusable::LevelOverflows))
        );
    }

    #[test]
    fn full_ohlcv_and_range_are_available_for_audit() {
        let references = [eligible(20_000, 2_500_000)];
        let mut evaluator = evaluator(&references);
        evaluator
            .step(&signal_bar(20_001, OPEN_IST_MINUTE, 2_500_000))
            .expect("signal is valid");
        let snapshot = evaluator
            .current_reference()
            .expect("prior daily is installed");
        assert_eq!(
            (
                snapshot.open,
                snapshot.high,
                snapshot.low,
                snapshot.close,
                snapshot.volume,
                snapshot.range,
            ),
            (2_500_000, 2_501_000, 2_499_000, 2_500_200, 10_000, 2_000)
        );
    }

    #[test]
    fn one_minute_and_coarse_signal_rungs_read_identical_daily_families() {
        let references = [eligible(20_000, 2_500_000)];
        let mut one_minute = evaluator(&references);
        let mut coarse = evaluator(&references);
        let close = 2_501_500;
        let one_mask = one_minute
            .step(&signal_bar(20_001, OPEN_IST_MINUTE, close))
            .expect("one-minute signal is valid");
        let coarse_mask = coarse
            .step(&signal_bar(20_001, OPEN_IST_MINUTE + 60, close))
            .expect("coarse signal is valid");
        assert_eq!(anchor_positions(one_mask), anchor_positions(coarse_mask));
        assert_eq!(one_minute.current_reference(), coarse.current_reference());
    }

    #[test]
    fn appending_valid_future_daily_prices_cannot_change_a_current_mask() {
        let prefix = [eligible(20_000, 2_500_000)];
        let extended = [eligible(20_000, 2_500_000), eligible(20_010, 8_000_000)];
        let signal = signal_bar(20_001, OPEN_IST_MINUTE, 2_501_500);
        let mut left = evaluator(&prefix);
        let mut right = evaluator(&extended);
        let left_mask = left.step(&signal).expect("signal is valid");
        let right_mask = right.step(&signal).expect("signal is valid");
        assert_eq!(left_mask, right_mask);
        assert_eq!(left.current_reference(), right.current_reference());
        assert_eq!(right.reference_census().remaining(), 1);
    }

    #[test]
    fn five_previous_daily_sessions_fill_prev5_without_signal_session_reconstruction() {
        let references = [
            eligible(20_000, 2_400_000),
            eligible(20_001, 2_410_000),
            eligible(20_002, 2_420_000),
            eligible(20_003, 2_430_000),
            eligible(20_004, 2_440_000),
        ];
        let mut evaluator = evaluator(&references);
        evaluator
            .step(&signal_bar(20_005, OPEN_IST_MINUTE, 2_440_000))
            .expect("signal is valid");
        assert_eq!(evaluator.sessions_completed(), 5);
        assert!(evaluator.has_yesterday());
        assert_eq!(
            evaluator.current_reference().map(|r| r.ist_day),
            Some(20_004)
        );
    }

    #[test]
    fn a_signal_session_rollover_never_reconstructs_a_daily_anchor() {
        let references = [eligible(20_000, 2_500_000)];
        let mut evaluator = evaluator(&references);
        evaluator
            .step(&signal_bar(20_001, OPEN_IST_MINUTE, 7_000_000))
            .expect("first coarse signal is valid");
        evaluator
            .step(&signal_bar(20_002, OPEN_IST_MINUTE, 8_000_000))
            .expect("next coarse session is valid");
        let reference = evaluator
            .current_reference()
            .expect("the stored daily reference remains installed");
        assert_eq!(reference.ist_day, 20_000);
        assert_eq!(reference.close, 2_500_200);
        assert_eq!(evaluator.reference_census().installed, 1);
    }

    #[test]
    fn anchored_column_carries_both_signal_and_reference_censuses() {
        let references = [eligible(20_000, 2_500_000)];
        let bars = [
            signal_bar(20_001, OPEN_IST_MINUTE, 2_501_000),
            signal_bar(20_001, OPEN_IST_MINUTE + 1, 2_501_100),
        ];
        let mut evaluator = evaluator(&references);
        let anchored = AnchoredColumn::build(&bars, &mut evaluator);
        assert_eq!(anchored.column().census().offered, 2);
        assert_eq!(anchored.reference_census().installed, 1);
        assert_eq!(anchored.reference_census().signal_bars_with_reference, 2);
        assert!(anchored.reference_census().reconciles());
        let (projected, dropped) = anchored
            .column()
            .reproject_checked(&[], &bars)
            .expect("the empty warmed column is parallel to an empty alignment");
        assert_eq!(dropped, 0);
        assert!(
            projected.acceptance_covers(bars.len()),
            "daily references fix signal masks; execution replay only needs the ordinary corruption verdict"
        );
    }

    #[test]
    fn exact_minute_overlay_never_substitutes_a_later_minute() {
        let (signal, references, mut minute) = coarse_overlay_fixture();
        let mut anchored = evaluator(&references);
        let mut column = AnchoredColumn::build_required(&signal, &mut anchored)
            .expect("the multi-session fixture warms an anchored column")
            .into_column();
        let source = *column
            .sources()
            .first()
            .expect("the multi-session fixture produces warm rows");
        let signal_bar = signal
            .get(source)
            .expect("a column source indexes its signal");
        let expected = signal_bar
            .ts_micros
            .saturating_add(15 * MINUTE_MICROS)
            .saturating_sub(MINUTE_MICROS);
        let removed = minute
            .iter()
            .position(|bar| bar.ts_micros == expected)
            .expect("the exact closing minute is present");
        minute.remove(removed);
        assert!(
            minute.iter().any(|bar| bar.ts_micros > expected),
            "the fixture must contain a later minute that a fallback could wrongly use"
        );
        assert_eq!(
            overlay_exact_minute_gapfib(
                &signal,
                &minute,
                15 * MINUTE_MICROS,
                Widths::pinned().expect("pinned widths"),
                Calendar::charter(),
                &mut column,
            ),
            Err(ExactMinuteGapRefusal::MissingClosingMinute {
                source,
                expected_ts_micros: expected,
            })
        );
    }

    #[test]
    fn signal_aggregate_close_must_equal_its_exact_final_minute() {
        let (signal, references, mut minute) = coarse_overlay_fixture();
        let mut anchored = evaluator(&references);
        let mut column = AnchoredColumn::build_required(&signal, &mut anchored)
            .expect("the fixture warms")
            .into_column();
        let source = 0_usize;
        let signal_bar = signal
            .get(source)
            .expect("a column source indexes its signal");
        let expected = signal_bar
            .ts_micros
            .saturating_add(15 * MINUTE_MICROS)
            .saturating_sub(MINUTE_MICROS);
        let exact = minute
            .iter_mut()
            .find(|bar| bar.ts_micros == expected)
            .expect("the closing minute exists");
        exact.close = exact.close.saturating_add(1);
        exact.high = exact.high.max(exact.close);
        assert_eq!(
            overlay_exact_minute_gapfib(
                &signal,
                &minute,
                15 * MINUTE_MICROS,
                Widths::pinned().expect("pinned widths"),
                Calendar::charter(),
                &mut column,
            ),
            Err(ExactMinuteGapRefusal::SignalCloseMismatch {
                source,
                signal_close: signal_bar.close,
                minute_close: signal_bar.close.saturating_add(1),
            })
        );
    }

    #[test]
    fn future_exact_minutes_cannot_change_an_already_mapped_signal_column() {
        let (signal, references, minute) = coarse_overlay_fixture();
        let mut left_evaluator = evaluator(&references);
        let mut right_evaluator = evaluator(&references);
        let mut left = AnchoredColumn::build_required(&signal, &mut left_evaluator)
            .expect("the fixture warms")
            .into_column();
        let mut right = AnchoredColumn::build_required(&signal, &mut right_evaluator)
            .expect("the fixture warms twice")
            .into_column();
        let mut extended = minute.clone();
        extended.push(signal_bar(1_009, OPEN_IST_MINUTE, 9_000_000));
        let left_census = overlay_exact_minute_gapfib(
            &signal,
            &minute,
            15 * MINUTE_MICROS,
            Widths::pinned().expect("pinned widths"),
            Calendar::charter(),
            &mut left,
        )
        .expect("every signal close has an exact minute");
        let right_census = overlay_exact_minute_gapfib(
            &signal,
            &extended,
            15 * MINUTE_MICROS,
            Widths::pinned().expect("pinned widths"),
            Calendar::charter(),
            &mut right,
        )
        .expect("future evidence is well formed but causally irrelevant");
        assert_eq!(left.bits(), right.bits());
        assert_eq!(left.sources(), right.sources());
        assert_eq!(left_census.signal_rows, right_census.signal_rows);
        assert_eq!(left_census.overlaid, right_census.overlaid);
        assert_eq!(
            right_census.minute_bars,
            left_census.minute_bars.saturating_add(1)
        );
    }

    #[test]
    fn previous_session_gap_bits_use_the_daily_close() {
        let references = [eligible(20_000, 2_500_000)];
        let mut evaluator = evaluator(&references);
        let mask = evaluator
            .step(&signal_bar(20_001, OPEN_IST_MINUTE, 2_502_000))
            .expect("signal is valid");
        assert!(mask.get(48), "today opened above the stored daily close");
        assert!(!mask.get(49));
    }

    #[test]
    fn a_refused_signal_consumes_neither_reference_nor_census() {
        let references = [eligible(20_000, 2_500_000), eligible(20_001, 2_600_000)];
        let mut evaluator = evaluator(&references);
        let first = signal_bar(20_001, OPEN_IST_MINUTE, 2_501_000);
        evaluator.step(&first).expect("first signal is valid");
        let before = (evaluator.current_reference(), evaluator.reference_census());
        let duplicate = Candle {
            open: 2_501_000,
            high: 2_501_100,
            low: 2_500_900,
            close: 2_501_000,
            ..first
        };
        assert_eq!(
            evaluator.step(&duplicate),
            Err(Corrupt::TimestampNotIncreasing)
        );
        assert_eq!(
            (evaluator.current_reference(), evaluator.reference_census()),
            before
        );
    }

    #[test]
    fn census_is_deterministic_and_gapfib_boundary_is_explicit() {
        let references = [
            eligible(20_000, 2_500_000),
            excluded(20_001, 7_000_000),
            eligible(20_002, 2_600_000),
        ];
        let bars = [
            signal_bar(19_999, OPEN_IST_MINUTE, 2_400_000),
            signal_bar(20_003, OPEN_IST_MINUTE, 2_601_000),
            signal_bar(20_003, OPEN_IST_MINUTE + 1, 2_601_100),
        ];
        let run = || {
            let mut evaluator = evaluator(&references);
            for bar in &bars {
                evaluator
                    .step(bar)
                    .expect("ordered signal fixture is valid");
            }
            assert_eq!(
                evaluator.gap_reference_source(),
                GapReferenceSource::SignalSeriesLastThreeIntradayBars
            );
            evaluator.reference_census()
        };
        let first = run();
        assert_eq!(first, run());
        assert_eq!(
            first,
            DailyReferenceCensus {
                offered: 3,
                eligible: 2,
                excluded: 1,
                consumed: 3,
                installed: 2,
                skipped_excluded: 1,
                signal_bars_with_reference: 2,
                signal_bars_without_reference: 1,
                signal_days_with_reference: 1,
                signal_days_without_reference: 1,
            }
        );
        assert!(first.reconciles());
    }
}
