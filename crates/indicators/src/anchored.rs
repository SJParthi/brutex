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

use std::collections::HashMap;

use crate::daily::{DailyLevels, Unusable};
use crate::evaluator::{Calendar, Evaluator, Widths};
use crate::gap::GapFib;
use crate::orb::Orb;
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
        self.step_with_warmth(bar).map(|(mask, _, _)| mask)
    }

    /// Step once and return whether the complete run was warm immediately
    /// after installing every eligible prior daily record but before folding
    /// this signal bar.
    pub(crate) fn step_with_warmth(
        &mut self,
        bar: &Candle,
    ) -> Result<(ConditionMask, ConditionMask, bool), Corrupt> {
        let mut next = *self;
        let signal_day = crate::ist_day(bar.ts_micros);
        next.advance_before(signal_day);
        let warm = next.evaluator.warmed_up();
        let (mask, known) = next.evaluator.step_known(bar)?;
        next.charge_signal(signal_day);
        *self = next;
        Ok((mask, known, warm && next.evaluator.warmed_up()))
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

/// Accounting for an exact-minute overlay on one signal column.
///
/// The existing name is retained for compatibility. Both the GapFib-only and
/// combined ORB/GapFib bridges count the same source minutes and aligned rows.
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

/// Micros elapsed since the IST midnight that opens this timestamp's session day.
///
/// Micros and not minutes, because that is the unit every timestamp in this crate is
/// already in and converting would only introduce a rounding policy. The ORDER is what
/// callers use it for, and micros-of-day and minute-of-day sort identically.
///
/// `rem_euclid` and never `%`, for [`crate::ist_day`]'s reason: a pre-epoch stamp gives
/// a negative remainder under `%` and would compare as an impossibly early minute.
fn ist_micros_of_day(ts_micros: i64) -> i64 {
    ts_micros
        .saturating_add(crate::IST_OFFSET_MICROS)
        .rem_euclid(crate::MICROS_PER_DAY)
}

/// The last stored one-minute bar of every IST day the exact-minute context holds.
///
/// # This was a binary search, and the search sat inside a per-signal-bar loop
///
/// It used to be `last_stored_minute_of_day(exact_minute, day)`, one
/// `slice::partition_point` per short day-final signal bucket — `O(log minutes)`
/// probes against a slice that grows with every month ingested, called from inside
/// the `for (source, signal_bar) in signal.iter().enumerate()` loop below. Gate 11
/// rule 1 refuses that spelling without exception and `docs/07-o1-architecture.md`
/// layer 4 is why: the frequency was small — one bucket per day per rung — but the
/// COST of each lookup tracked the size of the store, which is the shape the rule
/// exists to remove rather than to bound.
///
/// # What replaces it, and why the shape is not a second walk
///
/// The pair walk below is `exact_minute`'s own cadence pass, which already visits
/// every bar exactly once. `day_last` records `(day, last ts)` by overwriting the
/// tail entry while the day holds and pushing when it changes — legal because the
/// caller has already proved `exact_minute` strictly increasing, so `ist_day` is
/// non-decreasing and a day's bars are contiguous. The map is then built at the
/// EXACT size `day_last` measured, which is the day count and never the minute
/// count: `docs/07-o1-architecture.md` law 2 asks for a reservation and reserving
/// one slot per minute would over-reserve by the session length.
///
/// Lookup afterwards is one hash probe, so the per-signal-bar cost no longer
/// depends on how many minutes are in scope. Total work is `O(minutes)` once and
/// `O(1)` per query; the extra space is `O(days)`. **UNVERIFIED as a measured
/// bound** — no row in `crates/indicators/benches/ratio.rs` covers this overlay.
fn last_stored_minute_by_day(exact_minute: &[Candle]) -> HashMap<i64, i64> {
    let mut day_last: Vec<(i64, i64)> = Vec::new();
    for bar in exact_minute {
        let day = crate::ist_day(bar.ts_micros);
        match day_last.last_mut() {
            Some(seen) if seen.0 == day => seen.1 = bar.ts_micros,
            _ => day_last.push((day, bar.ts_micros)),
        }
    }
    let mut by_day = HashMap::with_capacity(day_last.len());
    by_day.extend(day_last);
    by_day
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
/// # The day's LAST bucket is short, and this demanded a minute that cannot exist
///
/// `t + signal_length - 1 minute` assumes every signal bar is full length. The NSE
/// session is 375 minutes — 09:15 through 15:29 inclusive — and of the eight swept
/// rungs **375 divides only by `1min`, `3min`, `5min` and `15min`.** On the other four
/// the day's final bucket is a stub, and the closing minute the formula asked for is
/// after the close:
///
/// | Rung | Bars per day | Last bar opens | Formula demanded | Exists |
/// |---|---:|---|---|---|
/// | `2min` | 188 | 15:29 | 15:30 | no |
/// | `10min` | 38 | 15:25 | 15:34 | no |
/// | `30min` | 13 | 15:15 | 15:44 | no |
/// | `60min` | 7 | 15:15 | **16:14** | no |
///
/// Counts MEASURED from `zerodha/NSE/INDEX/NIFTY/<rung>/2024-01.bin`, a 22-session
/// month: 4,136, 836, 286 and 154 records, and the 60-minute file's seventh record of
/// each day is stamped 15:15. Every `elite` and `screen` run on those four rungs
/// refused here, at every support step, for a minute 44 minutes past a close that has
/// never moved.
///
/// **What the fix does NOT do is relax the refusal.** The guard is correct and
/// deliberate: substituting a later minute or coarser data for a genuine hole is the
/// silent fallback `CLAUDE.md` §4 bans, and `3min` — which divides 375 exactly — must
/// still refuse its real one-minute hole. So the clamp is admitted only when both hold:
///
/// 1. the demanded minute-of-day is later than **any** minute-of-day anywhere in the
///    supplied evidence; and
/// 2. that day HAS a stored minute at or after the signal bar itself, so a bucket can
///    never be resolved by a minute that precedes it.
///
/// The clamp can only ever narrow the target and never move it later, and that is a
/// consequence rather than a third clause: the candidate minute lies on the signal
/// bar's own day, and clause 1 puts the demanded minute-of-day past every minute-of-day
/// in the evidence — including that candidate's. Writing it as a check as well would be
/// a branch no input could take.
///
/// Clause 1 is the whole distinction. A minute-of-day no session in the evidence ever
/// reached is a minute outside the session; a minute other days DO hold and this one
/// does not is a hole, and a hole still refuses. On a slice whose days all end at 15:29
/// that makes 16:14 clampable and a missing 15:29 refusable, which is exactly the split
/// wanted — and it is derived from the bars rather than from a hardcoded 15:29, because
/// this crate depends on `vocab` alone and has no exchange calendar to consult.
///
/// **There is deliberately no "is this the day's last signal bar" clause**, though it
/// reads like the obvious third one. It would be REDUNDANT, and a redundant branch is a
/// surviving mutant under §9. Clause 1 already implies the session ends inside this
/// bar's own window: `demanded` is past every session's last minute, so it is past this
/// day's, and clause 2 pins the target at or after the bar — the clamp therefore cannot
/// leave the bucket `[t, t + signal_length)` whether or not another bar follows on the
/// same day.
///
/// **The honest limit**, under §3 rule 6: a day that genuinely stops early, such as the
/// 2021-02-24 outage stub, ends before every other day in the slice, so clause 1 fails
/// and its final bucket still refuses. That is the conservative direction — a refusal
/// rather than a substitution — but it is a refusal, and a caller sweeping a month
/// containing such a day on one of the four stub rungs will still be stopped by it.
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
    overlay_exact_minute(
        signal,
        exact_minute,
        signal_length_micros,
        widths,
        calendar,
        column,
        ExactMinuteFamilies::GapFib,
    )
}

/// Replace ORB positions 86..=105 and `GapFib` positions 132..=142 from real
/// one-minute evidence, preserving all other signal-family truth and availability.
///
/// A coarse candle may cross an opening-range boundary. Folding its entire high
/// and low at its opening timestamp includes prices from after that boundary.
/// This bridge instead streams [`Orb::step`] once per supplied minute and takes
/// the truth and known masks at the signal's exact final minute. ORB references
/// therefore use only real minutes inside their own window. The GapFib-only
/// compatibility bridge and this combined bridge share every alignment and
/// refusal rule documented on [`overlay_exact_minute_gapfib`].
///
/// Replacement is transactional: cadence, OHLCV, close alignment and column
/// source checks finish before either family changes. Work and retained masks
/// grow with the supplied minute/signal counts; this is not an O(1) whole fold.
///
/// # Errors
///
/// [`ExactMinuteGapRefusal`] names the shared exact-minute evidence refusal.
pub fn overlay_exact_minute_orb_and_gapfib(
    signal: &[Candle],
    exact_minute: &[Candle],
    signal_length_micros: i64,
    widths: Widths,
    calendar: Calendar,
    column: &mut crate::column::Column,
) -> Result<ExactMinuteGapCensus, ExactMinuteGapRefusal> {
    overlay_exact_minute(
        signal,
        exact_minute,
        signal_length_micros,
        widths,
        calendar,
        column,
        ExactMinuteFamilies::OrbAndGapFib,
    )
}

#[derive(Clone, Copy)]
enum ExactMinuteFamilies {
    GapFib,
    OrbAndGapFib,
}

impl ExactMinuteFamilies {
    /// Both masks describe the same minute close; ORB's post-step known state
    /// uses the identical frozen windows that its truth emission just read.
    fn step(
        self,
        bar: &Candle,
        gap: &mut GapFib,
        orb: &mut Orb,
        widths: Widths,
        calendar: &Calendar,
    ) -> Result<(ConditionMask, ConditionMask), Corrupt> {
        let (mut truth, mut known) = gap.step_known(bar, widths.fib, calendar)?;
        if matches!(self, Self::OrbAndGapFib) {
            truth = truth.union(&orb.step(bar, widths.fib)?);
            known = known.union(&orb.known(widths.fib));
        }
        Ok((truth, known))
    }
}

fn overlay_exact_minute(
    signal: &[Candle],
    exact_minute: &[Candle],
    signal_length_micros: i64,
    widths: Widths,
    calendar: Calendar,
    column: &mut crate::column::Column,
    families: ExactMinuteFamilies,
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
    let mut orb = Orb::new();
    let mut minute_masks = Vec::with_capacity(exact_minute.len());
    // The latest minute-of-day any supplied session reached. Accumulated in the pass
    // that already walks every minute, so it costs one comparison per bar and no
    // second traversal. `i64::MIN` cannot survive a non-empty slice, and the slice was
    // proved non-empty above.
    let mut latest_session_micros = i64::MIN;
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
        let mask = families
            .step(bar, &mut gap, &mut orb, widths, &calendar)
            .map_err(|why| ExactMinuteGapRefusal::CorruptMinute { index, why })?;
        latest_session_micros = latest_session_micros.max(ist_micros_of_day(bar.ts_micros));
        minute_masks.push(mask);
    }

    // BUILT ONCE, OUTSIDE THE LOOP THAT ASKS IT. The lookup below used to be a
    // `partition_point` over the whole minute context, so its cost tracked the
    // months in scope. See [`last_stored_minute_by_day`].
    let last_minute_by_day = last_stored_minute_by_day(exact_minute);

    let mut exact_by_source = Vec::with_capacity(signal.len());
    let mut cursor = 0_usize;
    for (source, signal_bar) in signal.iter().enumerate() {
        let signal_day = crate::ist_day(signal_bar.ts_micros);
        let demanded = signal_bar
            .ts_micros
            .saturating_add(signal_length_micros)
            .saturating_sub(MINUTE_MICROS);
        let expected = if ist_micros_of_day(demanded) > latest_session_micros {
            // `filter` is the causality guard and not tidiness: it admits only a minute
            // AT OR AFTER this signal bar, so a bucket can never be resolved by a minute
            // that precedes it. A day with no stored minute at all -- and a day whose
            // minutes all end before the bucket opens -- falls through to `demanded` and
            // refuses exactly as before.
            //
            // There is deliberately no `< demanded` half. It would always be true here
            // and so would be a branch no test could take: `last` is on `signal_day` and
            // the condition above puts `demanded`'s minute-of-day past EVERY minute-of-
            // day in the evidence, `last`'s included, so `last < demanded` follows.
            last_minute_by_day
                .get(&signal_day)
                .copied()
                .filter(|last| *last >= signal_bar.ts_micros)
                .unwrap_or(demanded)
        } else {
            demanded
        };
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
        if minute.ts_micros != expected || crate::ist_day(minute.ts_micros) != signal_day {
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
    let replaced = match families {
        ExactMinuteFamilies::GapFib => column.replace_gapfib(&exact_for_signal),
        ExactMinuteFamilies::OrbAndGapFib => column.replace_orb_and_gapfib(&exact_for_signal),
    };
    if !replaced {
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

    const HOUR_MICROS: i64 = 60 * MINUTE_MICROS;

    /// Whole 375-minute sessions, resampled the way the store actually holds them.
    ///
    /// SEVEN 60-minute buckets a day — 09:15, 10:15, 11:15, 12:15, 13:15, 14:15 and
    /// 15:15 — and the seventh covers fifteen minutes, not sixty. MEASURED against
    /// `zerodha/NSE/INDEX/NIFTY/60min/2024-01.bin`: 154 records over a 22-session
    /// month is exactly seven a day, and the seventh of each day is stamped 15:15
    /// with the next record a day later.
    ///
    /// Each bucket's close is its LAST minute's close, which is what the resampler
    /// produces and what `overlay_exact_minute_gapfib` cross-checks. For the seventh
    /// bucket that minute is 15:29 and never 16:14.
    fn hourly_fixture(first_day: i64, days: i64) -> (Vec<Candle>, Vec<Candle>) {
        let price_at = |day: i64, offset: i64| {
            2_500_000_i64
                .saturating_add(day.saturating_mul(1_000))
                .saturating_add(offset)
        };
        let mut signal = Vec::new();
        let mut minute = Vec::new();
        for day in first_day..first_day.saturating_add(days) {
            for offset in 0_i64..375 {
                minute.push(signal_bar(
                    day,
                    OPEN_IST_MINUTE.saturating_add(offset),
                    price_at(day, offset),
                ));
            }
            for bucket in 0_i64..7 {
                let opens = bucket.saturating_mul(60);
                let closes = core::cmp::min(opens.saturating_add(59), 374);
                signal.push(signal_bar(
                    day,
                    OPEN_IST_MINUTE.saturating_add(opens),
                    price_at(day, closes),
                ));
            }
        }
        (signal, minute)
    }

    /// Position of the one-minute bar at `offset` minutes past that day's open.
    fn minute_at(day: i64, offset: i64) -> i64 {
        ts(day, OPEN_IST_MINUTE.saturating_add(offset))
    }

    /// The day's last 60-minute bucket resolves at 15:29, not 44 minutes after close.
    ///
    /// # The refusal this removes
    ///
    /// `t + signal_length - 1 minute` assumed every bucket was full length. 375 is not
    /// divisible by 60, so the seventh bucket opens at 15:15 and the formula demanded
    /// **16:14** — a minute-of-day no NSE session has ever contained. `elite` and
    /// `screen` therefore refused at every support step on `2min`, `10min`, `30min` and
    /// `60min`, four of the eight swept rungs, on completely intact data.
    ///
    /// The column is deliberately empty. Everything this test is about happens in the
    /// signal/minute join, before any row is consulted, and a `Column` warm enough to
    /// carry rows would need 200 hourly bars — twenty-nine sessions of fixture to
    /// assert something the join has already decided.
    #[test]
    fn the_days_last_hourly_bucket_resolves_at_the_session_close() {
        let (signal, minute) = hourly_fixture(1_200, 3);
        // The premise, stated rather than assumed: the formula's answer is not on disk
        // and cannot be, while the session's own last minute is.
        let last_bucket = ts(1_200, OPEN_IST_MINUTE + 360);
        let demanded = last_bucket
            .saturating_add(HOUR_MICROS)
            .saturating_sub(MINUTE_MICROS);
        assert_eq!(
            demanded,
            ts(1_200, 16 * 60 + 14),
            "the formula demands 16:14"
        );
        assert!(
            !minute.iter().any(|bar| bar.ts_micros == demanded),
            "16:14 is 44 minutes after the close and must not be in the fixture"
        );
        assert!(
            minute
                .iter()
                .any(|bar| bar.ts_micros == minute_at(1_200, 374)),
            "15:29 is the session's last minute and must be in the fixture"
        );

        let mut column = crate::column::Column::default();
        let census = overlay_exact_minute_gapfib(
            &signal,
            &minute,
            HOUR_MICROS,
            Widths::pinned().expect("pinned widths"),
            Calendar::charter(),
            &mut column,
        )
        .expect("every hourly bucket resolves against an intact session");
        assert_eq!(census.minute_bars, 3 * 375);
    }

    /// The clamp lands on 15:29 exactly, and not on whichever minute is nearby.
    ///
    /// `the_days_last_hourly_bucket_resolves_at_the_session_close` proves the refusal
    /// is gone; on its own that is also what a guard deleted outright would prove. This
    /// names the minute: move the close of 15:29 alone and the overlay must report the
    /// mismatch against THAT bar. A clamp landing on 15:28, or on the next day's open,
    /// reports a different `minute_close` or no refusal at all.
    #[test]
    fn the_clamped_bucket_maps_to_the_exact_session_close_and_no_neighbour() {
        let (signal, mut minute) = hourly_fixture(1_200, 3);
        let close_of_day = minute_at(1_201, 374);
        let mutated = minute
            .iter_mut()
            .find(|bar| bar.ts_micros == close_of_day)
            .expect("15:29 is in the fixture");
        mutated.close = mutated.close.saturating_add(7);
        mutated.high = mutated.high.max(mutated.close);
        let signal_close = signal
            .get(13)
            .expect("seven buckets a day puts the second day's last bucket at 13")
            .close;

        let mut column = crate::column::Column::default();
        assert_eq!(
            overlay_exact_minute_gapfib(
                &signal,
                &minute,
                HOUR_MICROS,
                Widths::pinned().expect("pinned widths"),
                Calendar::charter(),
                &mut column,
            ),
            Err(ExactMinuteGapRefusal::SignalCloseMismatch {
                source: 13,
                signal_close,
                minute_close: signal_close.saturating_add(7),
            }),
            "the day's last hourly bucket did not resolve against 15:29 itself"
        );
    }

    /// A real hole inside the session still refuses on a stub rung.
    ///
    /// The whole risk of the clamp is that it becomes a fallback. It must not: a minute
    /// OTHER sessions in the same evidence do hold, missing from this one, is a hole and
    /// `CLAUDE.md` §4 forbids papering over it. 12:14 closes the 11:15 bucket and sits
    /// nowhere near a session edge, so nothing about the day's end can excuse it.
    #[test]
    fn a_hole_inside_the_session_still_refuses_on_a_stub_rung() {
        let (signal, mut minute) = hourly_fixture(1_200, 3);
        let hole = minute_at(1_201, 179);
        assert_eq!(
            hole,
            ts(1_201, 12 * 60 + 14),
            "the 11:15 bucket closes 12:14"
        );
        minute.retain(|bar| bar.ts_micros != hole);

        let mut column = crate::column::Column::default();
        assert_eq!(
            overlay_exact_minute_gapfib(
                &signal,
                &minute,
                HOUR_MICROS,
                Widths::pinned().expect("pinned widths"),
                Calendar::charter(),
                &mut column,
            ),
            Err(ExactMinuteGapRefusal::MissingClosingMinute {
                source: 9,
                expected_ts_micros: hole,
            }),
            "a mid-session hole was clamped away instead of refused, which is the \
             silent fallback the guard exists to prevent"
        );
    }

    /// A session that stops early still refuses, and that limit is deliberate.
    ///
    /// The 2021-02-24 shape: 09:15–10:08 and nothing after it. Its only 60-minute
    /// bucket demands 10:14, and 10:14 is a minute-of-day the neighbouring sessions DO
    /// reach — so the clamp's first clause fails and the join refuses rather than
    /// mapping the bucket onto 10:08.
    ///
    /// This is the conservative direction and it is asserted rather than merely
    /// documented, because the alternative — clamping any final bucket to whatever
    /// minute the day happens to end on — would silently accept a truncated session as
    /// a complete one. `docs/00-charter.md` §3 records that the day's own reopening is
    /// outside the pull window and cannot be recovered; a run over it must say so.
    #[test]
    fn a_session_that_stops_early_refuses_rather_than_mapping_its_last_minute() {
        let (signal, minute) = hourly_fixture(1_200, 3);
        // Keep day 1_201's first 54 minutes and its first bucket only, exactly the
        // outage stub's measured shape.
        let stub_end = minute_at(1_201, 53);
        let minute = minute
            .into_iter()
            .filter(|bar| crate::ist_day(bar.ts_micros) != 1_201 || bar.ts_micros <= stub_end)
            .collect::<Vec<_>>();
        let signal = signal
            .into_iter()
            .filter(|bar| {
                crate::ist_day(bar.ts_micros) != 1_201 || bar.ts_micros == minute_at(1_201, 0)
            })
            .collect::<Vec<_>>();

        let mut column = crate::column::Column::default();
        assert_eq!(
            overlay_exact_minute_gapfib(
                &signal,
                &minute,
                HOUR_MICROS,
                Widths::pinned().expect("pinned widths"),
                Calendar::charter(),
                &mut column,
            ),
            Err(ExactMinuteGapRefusal::MissingClosingMinute {
                source: 7,
                expected_ts_micros: minute_at(1_201, 59),
            }),
            "a 54-minute stub was mapped onto its own last bar, so a halted session is \
             now indistinguishable from a complete one"
        );
    }

    /// A bucket is never resolved by a minute that opens before it.
    ///
    /// The clamp's second clause, and the case that makes it load-bearing rather than
    /// decorative. Every session here stops at 09:20, so a 60-minute bucket opening at
    /// 10:15 has its demanded minute past every minute-of-day in the evidence and the
    /// first clause admits it — but the day's last stored minute is 09:20, which is
    /// BEFORE the bucket opens. Resolving the bucket there would price it off a bar it
    /// does not contain, so the clamp is withdrawn and the join refuses at the minute it
    /// originally asked for.
    #[test]
    fn a_bucket_is_never_resolved_by_a_minute_that_precedes_it() {
        let mut signal = Vec::new();
        let mut minute = Vec::new();
        for day in 1_200_i64..1_203 {
            for offset in 0_i64..6 {
                minute.push(signal_bar(
                    day,
                    OPEN_IST_MINUTE.saturating_add(offset),
                    2_500_000_i64.saturating_add(offset),
                ));
            }
            // One hourly bucket, opening a full hour after the last stored minute.
            signal.push(signal_bar(day, OPEN_IST_MINUTE + 60, 2_500_000));
        }

        assert_eq!(
            overlay_exact_minute_gapfib(
                &signal,
                &minute,
                HOUR_MICROS,
                Widths::pinned().expect("pinned widths"),
                Calendar::charter(),
                &mut crate::column::Column::default(),
            ),
            Err(ExactMinuteGapRefusal::MissingClosingMinute {
                source: 0,
                expected_ts_micros: minute_at(1_200, 119),
            }),
            "a 10:15 bucket was resolved by a 09:20 minute, which is a bar it does not \
             contain and cannot have closed on"
        );
    }

    /// A day with no stored minutes refuses, from either side of the evidence.
    ///
    /// Two arms of the same lookup, and both are reachable only once the clamp has been
    /// admitted, which is why they need saying. Asking for the last minute of a day that
    /// is BEFORE every stored minute finds no prefix at all; asking for one that falls
    /// between two stored days finds a prefix whose last bar belongs to the wrong day.
    /// Neither may be answered with somebody else's minute.
    #[test]
    fn a_day_with_no_stored_minutes_refuses_from_either_side() {
        let full = hourly_fixture(1_200, 3).1;

        // BEFORE the evidence: no prefix exists at all.
        let early = vec![signal_bar(1_199, OPEN_IST_MINUTE + 360, 2_500_000)];
        assert_eq!(
            overlay_exact_minute_gapfib(
                &early,
                &full,
                HOUR_MICROS,
                Widths::pinned().expect("pinned widths"),
                Calendar::charter(),
                &mut crate::column::Column::default(),
            ),
            Err(ExactMinuteGapRefusal::MissingClosingMinute {
                source: 0,
                expected_ts_micros: ts(1_199, 16 * 60 + 14),
            }),
            "a day before every stored minute borrowed a later day's session close"
        );

        // INSIDE the evidence: a prefix exists and its last bar is another day's.
        let punched = full
            .into_iter()
            .filter(|bar| crate::ist_day(bar.ts_micros) != 1_201)
            .collect::<Vec<_>>();
        let absent = vec![signal_bar(1_201, OPEN_IST_MINUTE + 360, 2_500_000)];
        assert_eq!(
            overlay_exact_minute_gapfib(
                &absent,
                &punched,
                HOUR_MICROS,
                Widths::pinned().expect("pinned widths"),
                Calendar::charter(),
                &mut crate::column::Column::default(),
            ),
            Err(ExactMinuteGapRefusal::MissingClosingMinute {
                source: 0,
                expected_ts_micros: ts(1_201, 16 * 60 + 14),
            }),
            "a day with no minutes of its own was closed off the previous day's 15:29"
        );
    }

    /// A rung that divides 375 exactly is untouched by the clamp.
    ///
    /// `3min` gives 125 buckets a day and every one of them is full length, so no
    /// demanded minute can be past the session close and the clamp can never fire. The
    /// measured failure on this rung — 2020-02-13 10:32 — is a genuine one-minute hole,
    /// and it must still refuse.
    #[test]
    fn an_exactly_dividing_rung_still_refuses_its_one_minute_hole() {
        let minute = hourly_fixture(1_200, 3).1;
        let mut signal = Vec::new();
        for day in 1_200_i64..1_203 {
            for bucket in 0_i64..125 {
                let opens = bucket.saturating_mul(3);
                signal.push(signal_bar(
                    day,
                    OPEN_IST_MINUTE.saturating_add(opens),
                    2_500_000_i64
                        .saturating_add(day.saturating_mul(1_000))
                        .saturating_add(opens.saturating_add(2)),
                ));
            }
        }
        let intact = overlay_exact_minute_gapfib(
            &signal,
            &minute,
            3 * MINUTE_MICROS,
            Widths::pinned().expect("pinned widths"),
            Calendar::charter(),
            &mut crate::column::Column::default(),
        );
        assert!(
            intact.is_ok(),
            "an exactly dividing rung over intact sessions must resolve every bucket"
        );

        let hole = minute_at(1_201, 197);
        let punched = minute
            .into_iter()
            .filter(|bar| bar.ts_micros != hole)
            .collect::<Vec<_>>();
        assert_eq!(
            overlay_exact_minute_gapfib(
                &signal,
                &punched,
                3 * MINUTE_MICROS,
                Widths::pinned().expect("pinned widths"),
                Calendar::charter(),
                &mut crate::column::Column::default(),
            ),
            Err(ExactMinuteGapRefusal::MissingClosingMinute {
                source: 190,
                expected_ts_micros: hole,
            }),
            "the clamp fired on a rung whose buckets are all full length"
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
        assert_eq!(left.known(), right.known());
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
