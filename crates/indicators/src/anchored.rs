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
//! and each daily bar is consumed at most once by an accepted signal bar.  A
//! refused signal bar walks no daily bar at all: its refusal is decided on the
//! committed evaluator before the cursor moves (D-0942), because the cursor walk
//! runs on a copy a refusal discards and used to be repeated, from the committed
//! cursor, by every consecutive refused bar.  State is fixed-size apart from the
//! two borrowed input slices: total work is `O(signal + daily)` with at most one
//! extra fold per signal bar that has an unconsumed prior reference, while every
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
    /// [`DailyReferenceRefusal::Corrupt`] for malformed OHLCV, including a
    /// record with a nonpositive price ([`Corrupt::PriceNotPositive`]), or
    /// [`DailyReferenceRefusal::Unusable`] when an eligible bar cannot produce
    /// all previous-day levels.  An excluded record need not produce levels,
    /// because no family is allowed to consume it.
    pub fn new(bar: Candle, eligibility: DailyEligibility) -> Result<Self, DailyReferenceRefusal> {
        // `check_evaluable`, not `check` (D-0941). `check` admits an all-zero
        // record by contract, and an ELIGIBLE all-zero daily bar used to install
        // a 0/0/0 pivot, CPR and Fibonacci ladder, Prev5 and PreviousSession that
        // fired on every bar of the next day, while `Evaluator::stepped` refuses
        // the very same record as `PriceNotPositive`. An excluded record is held
        // to the same rule: it is offered and counted as evidence, and a record
        // that is not a price is not evidence of a session.
        bar.check_evaluable()
            .map_err(DailyReferenceRefusal::Corrupt)?;
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
        let signal_day = crate::ist_day(bar.ts_micros);
        // DECIDE A REFUSAL BEFORE WALKING THE DAILY CURSOR (D-0942).
        //
        // The walk below runs on a copy that a refusal discards, so a refused
        // bar used to walk every reference between the committed cursor and its
        // own day for nothing, and the next refused bar walked the same
        // references again: O(daily) per refused bar and O(signal x daily) per
        // run, against the module doc's O(signal + daily).
        //
        // Committing the walk on refusal is NOT safe: a refused bar may carry a
        // later day than the next accepted bar, and references installed for the
        // later day would be look-ahead for the earlier one. Instead the verdict
        // is taken first. Whether `Evaluator::stepped` refuses depends only on
        // the record, the last accepted timestamp and signal-folded state; the
        // three fields the walk writes (`prev5`, `yesterday`, `previous`) are
        // read by no refusal (`SessionState::step` reads `previous` only for its
        // bits, after `check_evaluable`). So the committed evaluator refuses
        // exactly the bars the advanced one refuses, with the same `Corrupt`.
        //
        // The probe runs only when the walk is non-empty, which an accepted bar
        // meets at most once per day that has an unconsumed prior reference.
        // A refused bar therefore costs one fold and no walk, and an accepted
        // bar's mask, known set, warmth and census are byte-identical to before.
        if self
            .references
            .get(self.cursor)
            .is_some_and(|reference| reference.day < signal_day)
        {
            #[cfg(test)]
            tests::count_refusal_probe();
            let mut probe = self.evaluator;
            probe.step(bar)?;
        }
        let mut next = *self;
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
            #[cfg(test)]
            tests::count_reference_walk_step();
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

/// The UTC micros of the IST midnight that opens `ist_day`.
///
/// The inverse of [`crate::ist_day`] at the day boundary. Saturating, so a day no
/// timestamp can name lands on an `i64` edge and the exact-minute lookup that follows
/// refuses it rather than wrapping onto a real day.
const fn ist_day_start_micros(ist_day: i64) -> i64 {
    ist_day
        .saturating_mul(crate::MICROS_PER_DAY)
        .saturating_sub(crate::IST_OFFSET_MICROS)
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
/// `t + signal_length - 1 minute` assumes every signal bar is full length. Which
/// rungs end the day short is set by the VENUE's session length, not by the rung. The
/// NSE index session is 375 minutes — 09:15 through 15:29 inclusive — and of the
/// eight swept rungs **375 divides only by `1min`, `3min`, `5min` and `15min`.** The
/// equity-derivatives session from 2026-08-03 is 385 minutes, which divides only by
/// `1min` and `5min`, so there `3min` and `15min` end with a 1- and a 10-minute bar
/// as well (`pull::anchor::derived_rung_stub_minutes_follow_the_venue_session`,
/// D-1447). On the index file measured below, the other four rungs' final bucket is a
/// stub, and the closing minute the formula asked for is after the close:
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
/// **What the fix does NOT do is relax the refusal.** Substituting a later minute or
/// coarser data for a genuine hole is the silent fallback `CLAUDE.md` §4 bans, and
/// `3min` — which divides 375 exactly — must still refuse its real one-minute hole.
///
/// # The clamp target is the CALLER'S session close, never a stored minute (D-0943)
///
/// `session_close(ist_day)` answers the minute-of-day at which that day's FINAL
/// one-minute bar opens — 15:29 on a regular NSE day — or `None` when the caller's
/// calendar cannot say. This crate depends on `vocab` alone and holds no exchange
/// calendar, so the answer is supplied: `cli` passes `pull::calendar::kind_of`'s last
/// window. The rule is then one line: the bar's target is
/// `min(demanded, close)` when `close` is at or after the bar's own open, and
/// `demanded` otherwise. Whatever the target is, the minute stamped exactly there must
/// be stored on that day, or the join refuses `MissingClosingMinute` naming it.
///
/// It replaced a data-derived rule with two defects (GAP12-5, GAP12-7, GAP4-47):
///
/// 1. **The target was the day's last STORED minute.** A session truncated inside
///    its last bucket — 15:29 missing, or the day stopping at 15:16 — was mapped onto
///    whatever minute it stopped at on `2min`, `10min`, `30min` and `60min`, while
///    `3min`, `5min` and `15min`, whose last bucket is full length, refused the same
///    data. Now every rung demands the session close and every rung refuses alike.
/// 2. **The threshold was read off the WHOLE minute slice.** One later day's 16:20
///    minute raised "the latest minute any session reached" past 16:14 and flipped
///    every EARLIER day's last hourly bucket from mapped to refused — a later minute
///    deciding an earlier answer, which is the look-ahead `CLAUDE.md` §3 rule 7 bans.
///    The caller's close for `signal_day` depends on that day alone, so appending any
///    later minute cannot move an earlier row.
///
/// The `close >= t` guard is the causality half: a bucket opening after the session's
/// last minute is never resolved by a minute that precedes it — it keeps `demanded` and
/// refuses. `min` makes the clamp unable to move a target later, so a full-length
/// bucket inside the session is untouched by it. Both are O(1) per signal bar: one
/// caller call and one comparison, with no per-day table and no scan. **UNVERIFIED as
/// a measured bound**: it is read off the source, and no row in
/// `crates/indicators/benches/ratio.rs` times this overlay.
///
/// **The honest limits**, under §3 rule 6. A day the caller cannot place (`None`) keeps
/// the formula's minute and so refuses its stub bucket; that is the conservative
/// direction. A day with two trading windows, such as the disaster-recovery Saturdays,
/// is clamped only at its LAST window's close; a bucket straddling the gap between
/// windows still demands its formula minute and refuses. Both kinds of day are withheld
/// from the stored sweep before this point by `SWEPT_SERIES_CALENDAR_POLICY`.
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
    session_close: impl Fn(i64) -> Option<u16>,
    column: &mut crate::column::Column,
) -> Result<ExactMinuteGapCensus, ExactMinuteGapRefusal> {
    overlay_exact_minute(
        signal,
        exact_minute,
        signal_length_micros,
        widths,
        calendar,
        &session_close,
        column,
        ExactMinuteFamilies::GapFib,
    )
}

/// The exact one-minute open stamp whose minute must carry the close of the signal
/// bar opening at `signal_ts_micros`: the bar's last minute, clamped onto `close`
/// (that IST day's session-close minute-of-day, when the caller's calendar knows it)
/// whenever the close is at or after the bar's own open (D-0943).
///
/// This is the rule [`overlay_exact_minute_gapfib`] and
/// [`overlay_exact_minute_orb_and_gapfib`] join on, made public so a caller that
/// must withhold days BEFORE the join asks the identical question rather than a
/// second copy of it (W2-cli9-3, D-1662). Only the bar's own day is consulted, so a
/// later day cannot move the answer. Saturates at both `i64` edges, which
/// `crate::anchored::tests::the_exact_closing_minute_is_the_overlays_target_at_every_edge`
/// pins.
#[must_use]
pub fn exact_closing_minute(
    signal_ts_micros: i64,
    signal_length_micros: i64,
    close: Option<u16>,
) -> i64 {
    let demanded = signal_ts_micros
        .saturating_add(signal_length_micros)
        .saturating_sub(MINUTE_MICROS);
    // THE CALLER'S SESSION CLOSE FOR THIS DAY, AND NOTHING READ FROM THE SLICE.
    // Only the bar's own day is asked about, so no other day's minutes -- later ones
    // included -- can move this target (GAP12-7, GAP4-47). `filter` is the causality
    // half: a close BEFORE the bucket opens is never its target, so such a bucket
    // keeps `demanded` and refuses. `min` means the clamp can only pull a target
    // that runs past the close back onto it, never push one later (GAP12-5). D-0943.
    close
        .map(|close| {
            ist_day_start_micros(crate::ist_day(signal_ts_micros))
                .saturating_add(i64::from(close).saturating_mul(MINUTE_MICROS))
        })
        .filter(|close| *close >= signal_ts_micros)
        .map_or(demanded, |close| demanded.min(close))
}

/// UNVERIFIED performance: no named cost test or measured latency bound is established here.
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
    session_close: impl Fn(i64) -> Option<u16>,
    column: &mut crate::column::Column,
) -> Result<ExactMinuteGapCensus, ExactMinuteGapRefusal> {
    overlay_exact_minute(
        signal,
        exact_minute,
        signal_length_micros,
        widths,
        calendar,
        &session_close,
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
        let (mut truth, mut known) = gap.step_known(bar, widths.fib(), calendar)?;
        if matches!(self, Self::OrbAndGapFib) {
            truth = truth.union(&orb.step(bar, widths.fib())?);
            known = known.union(&orb.known(widths.fib()));
        }
        Ok((truth, known))
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the two public bridges' signature plus the explicit family selection"
)]
fn overlay_exact_minute(
    signal: &[Candle],
    exact_minute: &[Candle],
    signal_length_micros: i64,
    widths: Widths,
    calendar: Calendar,
    session_close: &dyn Fn(i64) -> Option<u16>,
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
        minute_masks.push(mask);
    }

    let mut exact_by_source = Vec::with_capacity(signal.len());
    let mut cursor = 0_usize;
    for (source, signal_bar) in signal.iter().enumerate() {
        let signal_day = crate::ist_day(signal_bar.ts_micros);
        let expected = exact_closing_minute(
            signal_bar.ts_micros,
            signal_length_micros,
            session_close(signal_day),
        );
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

    /// `exact_closing_minute` is the overlay's own target rule, at every edge the
    /// stored-span census relies on (D-1662): a full bucket keeps its last minute,
    /// a day-final short bucket is clamped onto the close, an unknown close keeps
    /// the formula, a close before the bucket opens never pulls it earlier, and
    /// both `i64` edges saturate.
    #[test]
    fn the_exact_closing_minute_is_the_overlays_target_at_every_edge() {
        let day = 20_000_i64;
        let at = |minute: i64| day * DAY_MICROS - crate::IST_OFFSET_MICROS + minute * MINUTE_MICROS;
        let five = 5 * MINUTE_MICROS;
        let hour = 60 * MINUTE_MICROS;
        // 15:25 five-minute bucket: its last minute is 15:29, clamp or not.
        assert_eq!(exact_closing_minute(at(925), five, Some(929)), at(929));
        assert_eq!(exact_closing_minute(at(925), five, None), at(929));
        // 15:15 hour bucket: 16:14 by formula, the session's 15:29 by the clamp.
        assert_eq!(exact_closing_minute(at(915), hour, Some(929)), at(929));
        assert_eq!(exact_closing_minute(at(915), hour, None), at(974));
        // A full bucket inside the session is untouched by the clamp.
        assert_eq!(exact_closing_minute(at(555), five, Some(929)), at(559));
        // The close at the bucket's own open still clamps; one before it does not.
        assert_eq!(exact_closing_minute(at(929), five, Some(929)), at(929));
        assert_eq!(exact_closing_minute(at(930), five, Some(929)), at(934));
        // Saturating at both ends rather than wrapping onto a real minute.
        assert_eq!(
            exact_closing_minute(i64::MAX, five, None),
            i64::MAX - MINUTE_MICROS
        );
        assert_eq!(
            exact_closing_minute(i64::MIN, five, None),
            i64::MIN + 4 * MINUTE_MICROS
        );
    }

    /// The fixture sessions' close: every fixture day is a regular 09:15-15:29 session.
    #[allow(
        clippy::unnecessary_wraps,
        reason = "the overlay's session-close argument is an `Option`; this fixture always knows the day"
    )]
    const fn nse_close(_ist_day: i64) -> Option<u16> {
        Some(15 * 60 + 29)
    }

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
        let level_overflow = Candle::new(ts(20_000, 0), 1, i64::MAX / 2, 1, 1, 1, OI_NULL);
        assert_eq!(
            DailyReference::new(level_overflow, DailyEligibility::Eligible),
            Err(DailyReferenceRefusal::Unusable(Unusable::LevelOverflows))
        );
    }

    /// D-0941 (ET-indicators-1, UC-3). `DailyReference::new` validated with
    /// `check`, which admits an all-zero record by contract, so an ELIGIBLE
    /// 0/0/0 daily bar installed a zero pivot/CPR/Fibonacci ladder, `Prev5` and
    /// `PreviousSession` and fired pivot-family bits on every bar of the next day,
    /// while `Evaluator::stepped` refuses the same record as `PriceNotPositive`.
    /// Both doors now give the same answer for every nonpositive shape, under
    /// both eligibilities, and the smallest positive record is still admitted.
    #[test]
    fn a_nonpositive_daily_record_is_refused_exactly_as_the_evaluator_refuses_it() {
        let day = 20_000;
        let at = ts(day, 0);
        let nonpositive = [
            ("all zero", Candle::new(at, 0, 0, 0, 0, 0, OI_NULL)),
            (
                "low zero, high positive",
                Candle::new(at, 5, 10, 0, 5, 1, OI_NULL),
            ),
            ("negative low", Candle::new(at, 5, 10, -5, 5, 1, OI_NULL)),
            (
                "all negative, ordered",
                Candle::new(at, -10, -5, -20, -10, 1, OI_NULL),
            ),
            (
                "low at i64::MIN + 1",
                Candle::new(at, 0, 0, i64::MIN + 1, 0, 1, OI_NULL),
            ),
            (
                "open and close at low zero",
                Candle::new(at, 0, 7, 0, 0, 1, OI_NULL),
            ),
        ];
        for (name, bar) in nonpositive {
            for eligibility in [DailyEligibility::Eligible, DailyEligibility::Excluded] {
                assert_eq!(
                    DailyReference::new(bar, eligibility),
                    Err(DailyReferenceRefusal::Corrupt(Corrupt::PriceNotPositive)),
                    "{name} ({eligibility:?}) must not become reference evidence"
                );
            }
            // The ordinary evaluator refuses the same record for the same
            // reason, so the anchored door and the signal door agree.
            let mut ordinary = Evaluator::new(
                Widths::pinned().expect("pinned widths are valid"),
                Availability::Absent,
                Thresholds::CLASSICAL,
            );
            assert_eq!(
                ordinary.step(&bar),
                Err(Corrupt::PriceNotPositive),
                "{name}: the evaluator's own refusal"
            );
        }
        // Precedence is unchanged: an earlier structural refusal keeps its name
        // rather than being renamed to the sign refusal.
        let straddling = Candle::new(at, 0, i64::MAX, i64::MIN, 0, 1, OI_NULL);
        assert_eq!(
            DailyReference::new(straddling, DailyEligibility::Eligible),
            Err(DailyReferenceRefusal::Corrupt(Corrupt::RangeOverflows))
        );
        let negative_volume_and_price = Candle::new(at, 0, 0, 0, 0, -1, OI_NULL);
        assert_eq!(
            DailyReference::new(negative_volume_and_price, DailyEligibility::Eligible),
            Err(DailyReferenceRefusal::Corrupt(Corrupt::NegativeVolume))
        );
    }

    /// The boundary on the admitted side: one paisa is a price. A one-paisa
    /// eligible record is admitted, anchors the next day, and the anchor it
    /// installs is that record and no other.
    #[test]
    fn the_smallest_positive_daily_record_is_still_an_eligible_anchor() {
        let one = Candle::new(ts(20_000, 0), 1, 1, 1, 1, 0, OI_NULL);
        let references = [DailyReference::new(one, DailyEligibility::Eligible)
            .expect("a one-paisa record is a price")];
        assert_eq!(
            DailyReference::new(one, DailyEligibility::Excluded).map(|r| r.eligibility()),
            Ok(DailyEligibility::Excluded)
        );
        let mut evaluator = evaluator(&references);
        evaluator
            .step(&signal_bar(20_001, OPEN_IST_MINUTE, 2_500_000))
            .expect("signal is valid");
        assert!(evaluator.has_yesterday());
        let snapshot = evaluator.current_reference().expect("installed");
        assert_eq!(
            (
                snapshot.ist_day,
                snapshot.low,
                snapshot.high,
                snapshot.range
            ),
            (20_000, 1, 1, 0)
        );
        assert_eq!(evaluator.reference_census().installed, 1);
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
                nse_close,
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
                nse_close,
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
            nse_close,
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
                nse_close,
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
                nse_close,
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
    /// bucket demands 10:14, which is before the caller's 15:29 close, so the clamp
    /// cannot move it and the join refuses rather than mapping the bucket onto 10:08.
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
                nse_close,
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
    /// Every session here stops at 09:20, so the day's last STORED minute is before a
    /// 10:15 bucket opens. The retired data-derived clamp admitted this bucket and was
    /// saved only by its causality filter; the caller's 15:29 close leaves 11:14 where it
    /// is, so the join refuses at the minute the bucket originally asked for. The filter's
    /// own case, a close before the bucket opens, is
    /// `a_bucket_opening_after_the_session_close_is_never_pulled_back_onto_it`.
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
                nse_close,
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
    /// Two arms of the same lookup, both reached after the clamp has pulled the stub
    /// bucket's target back onto the caller's 15:29 close. Asking for a day that is
    /// BEFORE every stored minute finds no prefix at all; asking for one that falls
    /// between two stored days finds a prefix whose last bar belongs to the wrong day.
    /// Neither may be answered with somebody else's minute, and both name the session
    /// close they owed rather than the formula's 16:14 (D-0943).
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
                nse_close,
                &mut crate::column::Column::default(),
            ),
            Err(ExactMinuteGapRefusal::MissingClosingMinute {
                source: 0,
                expected_ts_micros: ts(1_199, 15 * 60 + 29),
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
                nse_close,
                &mut crate::column::Column::default(),
            ),
            Err(ExactMinuteGapRefusal::MissingClosingMinute {
                source: 0,
                expected_ts_micros: ts(1_201, 15 * 60 + 29),
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
            nse_close,
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
                nse_close,
                &mut crate::column::Column::default(),
            ),
            Err(ExactMinuteGapRefusal::MissingClosingMinute {
                source: 190,
                expected_ts_micros: hole,
            }),
            "the clamp fired on a rung whose buckets are all full length"
        );
    }

    /// The eight swept rungs, in minutes.
    const RUNGS: [i64; 8] = [1, 2, 3, 5, 10, 15, 30, 60];

    /// The regular NSE session close as a minute-of-day: 15:29.
    const CLOSE_MINUTE: i64 = 15 * 60 + 29;

    /// A deterministic, non-monotone price for one stored minute, so the ORB and
    /// `GapFib` families see ranges that open, break and gap rather than a ramp.
    fn wiggle_price(day: i64, minute_of_day: i64) -> i64 {
        2_500_000_i64
            .saturating_add(day.rem_euclid(17).saturating_mul(3_100))
            .saturating_add(
                minute_of_day
                    .saturating_mul(37)
                    .rem_euclid(211)
                    .saturating_mul(90),
            )
    }

    /// Stored minutes and the signal stream resampled from them the way the store
    /// does: buckets on the 09:15 grid at `rung` minutes, each stamped at its open and
    /// closing at the close of its LAST stored minute. A bucket with no stored minute
    /// has no signal bar, and the final bucket of a session that does not divide by the
    /// rung is a stub. `session(day)` gives the inclusive `[open, close]` minutes-of-day
    /// a day trades, `None` for no session at all; `keep(day, minute)` punches holes.
    fn resampled(
        days: core::ops::Range<i64>,
        rung: i64,
        session: impl Fn(i64) -> Option<(i64, i64)>,
        keep: impl Fn(i64, i64) -> bool,
    ) -> (Vec<Candle>, Vec<Candle>) {
        let mut signal = Vec::new();
        let mut minute = Vec::new();
        for day in days {
            let Some((open, close)) = session(day) else {
                continue;
            };
            let mut bucket: Option<(i64, i64)> = None;
            for minute_of_day in open..=close {
                if !keep(day, minute_of_day) {
                    continue;
                }
                let price = wiggle_price(day, minute_of_day);
                minute.push(signal_bar(day, minute_of_day, price));
                let opens = OPEN_IST_MINUTE.saturating_add(
                    minute_of_day
                        .saturating_sub(OPEN_IST_MINUTE)
                        .div_euclid(rung)
                        .saturating_mul(rung),
                );
                match bucket {
                    Some((current, _)) if current == opens => bucket = Some((opens, price)),
                    _ => {
                        if let Some((stamp, last)) = bucket {
                            signal.push(signal_bar(day, stamp, last));
                        }
                        bucket = Some((opens, price));
                    }
                }
            }
            if let Some((stamp, last)) = bucket {
                signal.push(signal_bar(day, stamp, last));
            }
        }
        (signal, minute)
    }

    #[allow(
        clippy::unnecessary_wraps,
        reason = "the fixture builder's session argument is an `Option`; a regular day always trades"
    )]
    fn regular(_day: i64) -> Option<(i64, i64)> {
        Some((OPEN_IST_MINUTE, CLOSE_MINUTE))
    }

    fn overlay(
        signal: &[Candle],
        minute: &[Candle],
        rung: i64,
        close: impl Fn(i64) -> Option<u16>,
        column: &mut crate::column::Column,
    ) -> Result<ExactMinuteGapCensus, ExactMinuteGapRefusal> {
        overlay_exact_minute_orb_and_gapfib(
            signal,
            minute,
            rung.saturating_mul(MINUTE_MICROS),
            Widths::pinned().expect("pinned widths"),
            Calendar::charter(),
            close,
            column,
        )
    }

    /// A warm anchored column over `signal`, so an overlay has real rows to replace.
    fn warm_column(signal: &[Candle], references: &[DailyReference]) -> crate::column::Column {
        let mut anchored = evaluator(references);
        let column = AnchoredColumn::build_required(signal, &mut anchored)
            .expect("every fixture day has an earlier daily reference")
            .into_column();
        assert!(!column.is_empty(), "the fixture must warm at least one row");
        column
    }

    /// Thirty-two regular sessions warm every rung, `60min` included: 7 buckets a day
    /// is 224 signal bars.
    const WARM_FIRST: i64 = 1_000;
    const WARM_LAST: i64 = 1_032;

    fn warm_references() -> Vec<DailyReference> {
        (995_i64..WARM_LAST)
            .map(|day| {
                eligible(
                    day,
                    2_480_000_i64.saturating_add(day.rem_euclid(13).saturating_mul(900)),
                )
            })
            .collect()
    }

    /// GAP4-47's rewrite of this test: a 60-minute rung and a later-day 16:20 minute.
    ///
    /// It used to append a 09:15 minute at a 15-minute rung, which no version of the
    /// clamp ever read, so it passed while the property it names was false: a later
    /// day's 16:20 raised the slice-wide threshold past 16:14 and every earlier day's
    /// 15:15 hourly bucket refused. The caller's per-day close (D-0943) makes an
    /// appended minute on a later day irrelevant to every earlier row, byte for byte.
    #[test]
    fn future_exact_minutes_cannot_change_an_already_mapped_signal_column() {
        let references = warm_references();
        let (signal, minute) = resampled(WARM_FIRST..WARM_LAST, 60, regular, |_, _| true);
        let mut left = warm_column(&signal, &references);
        let mut right = left.clone();
        let mut extended = minute.clone();
        extended.push(signal_bar(WARM_LAST, 16 * 60 + 20, 9_000_000));
        let left_census = overlay(&signal, &minute, 60, nse_close, &mut left)
            .expect("every hourly bucket resolves against an intact session");
        let right_census = overlay(&signal, &extended, 60, nse_close, &mut right)
            .expect("a later day's off-session minute is causally irrelevant");
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

    /// GAP12-7 and GAP4-47: no minute on a LATER day can move an earlier day's mapping,
    /// on any rung.
    ///
    /// Each extra minute is off-session on a day after the whole signal stream: 16:20
    /// sits past every stub rung's formula minute (15:30, 15:34, 15:44 and 16:14), and
    /// 15:40 past the `2min` and `10min` ones. Before D-0943 either raised the slice-
    /// wide threshold and turned the earlier days' last buckets from mapped into
    /// `MissingClosingMinute` on `2min`, `10min`, `30min` and `60min`.
    #[test]
    fn a_later_days_off_session_minute_cannot_change_an_earlier_mapping() {
        let references = warm_references();
        for rung in RUNGS {
            let (signal, minute) = resampled(WARM_FIRST..WARM_LAST, rung, regular, |_, _| true);
            let mut prefix = warm_column(&signal, &references);
            let fresh = prefix.clone();
            let prefix_census = overlay(&signal, &minute, rung, nse_close, &mut prefix);
            assert!(
                prefix_census.is_ok(),
                "{rung}min prefix refused: {prefix_census:?}"
            );
            let prefix_census = prefix_census.expect("checked above");
            for extra in [16 * 60 + 20, 15 * 60 + 40] {
                let mut extended = minute.clone();
                extended.push(signal_bar(WARM_LAST, extra, 9_000_000));
                let mut appended = fresh.clone();
                let census = overlay(&signal, &extended, rung, nse_close, &mut appended);
                assert!(
                    census.is_ok(),
                    "{rung}min with a later {extra}-minute bar refused: {census:?}"
                );
                let census = census.expect("checked above");
                assert_eq!(
                    prefix.bits(),
                    appended.bits(),
                    "{rung}min bits, extra {extra}"
                );
                assert_eq!(
                    prefix.known(),
                    appended.known(),
                    "{rung}min known, extra {extra}"
                );
                assert_eq!(prefix.sources(), appended.sources(), "{rung}min sources");
                assert_eq!(prefix_census.overlaid, census.overlaid);
            }
        }
    }

    /// The minute a truncated session's last bucket owes on `rung`: the formula's
    /// `t + rung - 1`, pulled back onto 15:29 when it runs past the close.
    fn owed(day: i64, opens: i64, rung: i64) -> i64 {
        ts(
            day,
            opens
                .saturating_add(rung)
                .saturating_sub(1)
                .min(CLOSE_MINUTE),
        )
    }

    /// GAP12-5: a session truncated inside its last bucket refuses alike on every rung.
    ///
    /// Before D-0943 the stub rungs clamped onto the day's last STORED minute, so with
    /// 15:29 missing `2min`, `10min`, `30min` and `60min` mapped the last bucket onto
    /// 15:28 while `3min`, `5min` and `15min` refused for 15:29; with the day stopping
    /// at 15:16, `30min` and `60min` mapped onto 15:16 while `15min` refused for 15:29.
    /// Now each rung owes `min(t + rung - 1, 15:29)` and refuses naming exactly that.
    #[test]
    fn a_truncated_final_stub_bucket_refuses_on_every_stub_rung() {
        let truncated = 1_201_i64;
        let shapes: [(&str, i64); 2] = [
            ("15:29 missing", CLOSE_MINUTE - 1),
            ("stops at 15:16", 15 * 60 + 16),
        ];
        for (shape, last_kept) in shapes {
            for rung in RUNGS {
                let (signal, minute) = resampled(1_200..1_203, rung, regular, |day, minute| {
                    day != truncated || minute <= last_kept
                });
                let source = signal
                    .iter()
                    .rposition(|bar| crate::ist_day(bar.ts_micros) == truncated)
                    .expect("the truncated day keeps signal bars");
                let last = signal.get(source).expect("rposition indexes signal");
                let opens = last
                    .ts_micros
                    .saturating_sub(ts(truncated, 0))
                    .div_euclid(MINUTE_MICROS);
                let expected_ts_micros = owed(truncated, opens, rung);
                let result = overlay(
                    &signal,
                    &minute,
                    rung,
                    nse_close,
                    &mut crate::column::Column::default(),
                );
                if expected_ts_micros <= ts(truncated, last_kept) {
                    assert!(
                        result.is_ok(),
                        "{shape}, {rung}min: the owed minute is stored, yet {result:?}"
                    );
                } else {
                    assert_eq!(
                        result,
                        Err(ExactMinuteGapRefusal::MissingClosingMinute {
                            source,
                            expected_ts_micros,
                        }),
                        "{shape}, {rung}min mapped a truncated bucket onto a stored minute"
                    );
                }
            }
        }
        // The rows the finding names, pinned rather than derived, so `owed` cannot be
        // wrong in the same way as the code: 15:29 at 10, 15, 30 and 60 minutes.
        for rung in [10, 15, 30, 60] {
            let (signal, minute) = resampled(1_200..1_203, rung, regular, |day, minute| {
                day != truncated || minute <= 15 * 60 + 16
            });
            assert!(
                matches!(
                    overlay(&signal, &minute, rung, nse_close, &mut crate::column::Column::default()),
                    Err(ExactMinuteGapRefusal::MissingClosingMinute { expected_ts_micros, .. })
                        if expected_ts_micros == ts(truncated, if rung == 10 { 15 * 60 + 24 } else { CLOSE_MINUTE })
                ),
                "{rung}min"
            );
        }
    }

    /// Intact sessions resolve on every rung, the `2min` bucket opening ON the close
    /// included: it opens at 15:29, owes 15:30, and the close it is pulled back to is
    /// its own open — the `close >= t` boundary taken with equality.
    #[test]
    fn intact_sessions_resolve_on_every_rung_including_a_bucket_opening_on_the_close() {
        for rung in RUNGS {
            let (signal, mut minute) = resampled(1_200..1_203, rung, regular, |_, _| true);
            assert!(
                overlay(
                    &signal,
                    &minute,
                    rung,
                    nse_close,
                    &mut crate::column::Column::default()
                )
                .is_ok(),
                "{rung}min over intact sessions"
            );
            // Move 15:29 on the middle day: every rung's last bucket must now name it.
            let close = ts(1_201, CLOSE_MINUTE);
            let bar = minute
                .iter_mut()
                .find(|bar| bar.ts_micros == close)
                .expect("15:29 is stored");
            bar.close = bar.close.saturating_add(7);
            bar.high = bar.high.max(bar.close);
            let source = signal
                .iter()
                .rposition(|bar| crate::ist_day(bar.ts_micros) == 1_201)
                .expect("the day has signal bars");
            let signal_close = signal.get(source).expect("indexes").close;
            assert_eq!(
                overlay(
                    &signal,
                    &minute,
                    rung,
                    nse_close,
                    &mut crate::column::Column::default()
                ),
                Err(ExactMinuteGapRefusal::SignalCloseMismatch {
                    source,
                    signal_close,
                    minute_close: signal_close.saturating_add(7),
                }),
                "{rung}min's last bucket did not resolve against 15:29 itself"
            );
        }
    }

    /// A bucket opening AFTER the caller's close is never pulled back onto it.
    ///
    /// A 60-minute bucket stamped 15:30 owes 16:29. The close, 15:29, precedes the
    /// bucket, so pricing the bucket there would use a bar it does not contain: the
    /// target stays 16:29 and the join refuses for it. If the vendor really stored
    /// 16:29 the formula's minute is honoured, exactly as on a full-length bucket.
    #[test]
    fn a_bucket_opening_after_the_session_close_is_never_pulled_back_onto_it() {
        let (mut signal, mut minute) = resampled(1_200..1_202, 60, regular, |_, _| true);
        signal.push(signal_bar(1_201, CLOSE_MINUTE + 1, 2_600_000));
        let source = signal.len().saturating_sub(1);
        assert_eq!(
            overlay(
                &signal,
                &minute,
                60,
                nse_close,
                &mut crate::column::Column::default()
            ),
            Err(ExactMinuteGapRefusal::MissingClosingMinute {
                source,
                expected_ts_micros: ts(1_201, 16 * 60 + 29),
            })
        );
        minute.push(signal_bar(1_201, 16 * 60 + 29, 2_600_000));
        assert!(
            overlay(
                &signal,
                &minute,
                60,
                nse_close,
                &mut crate::column::Column::default()
            )
            .is_ok(),
            "a stored formula minute must still be honoured"
        );
    }

    /// A Muhurat-style short session: the caller's close for that day is its own.
    ///
    /// One day trades 13:45-14:44 only. On the 09:15 grid its last `60min` bucket opens
    /// 14:15 and owes 15:14; the caller says that day closes at 14:44, so it maps there.
    /// The retired rule read 15:29 off the regular days around it and refused 15:14.
    /// With 14:44 itself missing, every rung whose last bucket covers it refuses for
    /// 14:44 and none maps onto 14:43.
    #[test]
    fn a_short_session_closes_at_the_callers_close_for_that_day() {
        const SHORT: i64 = 1_201;
        const SHORT_CLOSE: i64 = 14 * 60 + 44;
        let session = |day: i64| {
            if day == SHORT {
                Some((13 * 60 + 45, SHORT_CLOSE))
            } else {
                regular(day)
            }
        };
        let close = |day: i64| {
            u16::try_from(if day == SHORT {
                SHORT_CLOSE
            } else {
                CLOSE_MINUTE
            })
            .ok()
        };
        for rung in RUNGS {
            let (signal, minute) = resampled(1_200..1_203, rung, session, |_, _| true);
            assert!(
                overlay(
                    &signal,
                    &minute,
                    rung,
                    close,
                    &mut crate::column::Column::default()
                )
                .is_ok(),
                "{rung}min over an intact short session"
            );
            let (signal, minute) = resampled(1_200..1_203, rung, session, |day, minute| {
                day != SHORT || minute != SHORT_CLOSE
            });
            let source = signal
                .iter()
                .rposition(|bar| crate::ist_day(bar.ts_micros) == SHORT)
                .expect("the short day has signal bars");
            let opens = signal
                .get(source)
                .expect("indexes")
                .ts_micros
                .saturating_sub(ts(SHORT, 0))
                .div_euclid(MINUTE_MICROS);
            let result = overlay(
                &signal,
                &minute,
                rung,
                close,
                &mut crate::column::Column::default(),
            );
            if opens.saturating_add(rung).saturating_sub(1) >= SHORT_CLOSE && opens <= SHORT_CLOSE {
                assert_eq!(
                    result,
                    Err(ExactMinuteGapRefusal::MissingClosingMinute {
                        source,
                        expected_ts_micros: ts(SHORT, SHORT_CLOSE),
                    }),
                    "{rung}min"
                );
            } else {
                assert!(result.is_ok(), "{rung}min: {result:?}");
            }
        }
    }

    /// A day the caller cannot place keeps the formula's minute, and so does a close no
    /// minute-of-day can be: the clamp never invents a session end.
    ///
    /// Full-length rungs never needed the clamp and still resolve; stub rungs refuse for
    /// the formula minute on every day, the first included. An empty day between two
    /// sessions — no minutes and no signal bars — changes nothing on either side.
    #[test]
    fn an_unplaced_day_keeps_the_formula_minute_and_an_empty_day_changes_nothing() {
        let unknown = |_day: i64| -> Option<u16> { None };
        let beyond = |_day: i64| -> Option<u16> { Some(u16::MAX) };
        let session = |day: i64| if day == 1_201 { None } else { regular(day) };
        for rung in RUNGS {
            let (signal, minute) = resampled(1_200..1_203, rung, session, |_, _| true);
            assert!(
                overlay(
                    &signal,
                    &minute,
                    rung,
                    nse_close,
                    &mut crate::column::Column::default()
                )
                .is_ok(),
                "{rung}min across an empty day"
            );
            for close in [&unknown as &dyn Fn(i64) -> Option<u16>, &beyond] {
                let result = overlay(
                    &signal,
                    &minute,
                    rung,
                    close,
                    &mut crate::column::Column::default(),
                );
                if 375 % rung == 0 {
                    assert!(result.is_ok(), "{rung}min divides the session: {result:?}");
                } else {
                    let first_stub = signal
                        .iter()
                        .position(|bar| bar.ts_micros == ts(1_200, CLOSE_MINUTE - (375 % rung) + 1))
                        .expect("the first day's stub bucket");
                    assert_eq!(
                        result,
                        Err(ExactMinuteGapRefusal::MissingClosingMinute {
                            source: first_stub,
                            expected_ts_micros: ts(1_200, CLOSE_MINUTE - (375 % rung) + rung),
                        }),
                        "{rung}min with no caller close"
                    );
                }
            }
        }
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

    thread_local! {
        /// Daily references the cursor walk has stepped over on THIS test
        /// thread, including walks on copies a refusal discards. A count, not a
        /// clock, so the bound below is deterministic on a loaded machine.
        static REFERENCE_WALK_STEPS: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
    }

    pub(super) fn count_reference_walk_step() {
        REFERENCE_WALK_STEPS.with(|steps| steps.set(steps.get().saturating_add(1)));
    }

    fn reference_walk_steps() -> u64 {
        REFERENCE_WALK_STEPS.with(core::cell::Cell::get)
    }

    thread_local! {
        /// Refusal probes taken on THIS test thread: the extra fold D-0942 pays.
        static REFUSAL_PROBES: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
    }

    pub(super) fn count_refusal_probe() {
        REFUSAL_PROBES.with(|probes| probes.set(probes.get().saturating_add(1)));
    }

    fn refusal_probes() -> u64 {
        REFUSAL_PROBES.with(core::cell::Cell::get)
    }

    /// The probe's own price, pinned: one extra fold on the first accepted bar
    /// of a day that has an unconsumed STRICTLY earlier reference, and none on
    /// any other bar -- not on a later bar of that day, not while the cursor
    /// rests on a same-day reference, and not after the references run out.
    #[test]
    fn the_refusal_probe_runs_once_per_day_with_a_pending_prior_reference() {
        let references = [
            eligible(1, 2_500_000),
            eligible(2, 2_500_000),
            eligible(3, 2_500_000),
        ];
        let mut evaluator = evaluator(&references);
        let before = refusal_probes();
        for day in 2..=5 {
            for offset in 0..5 {
                evaluator
                    .step(&signal_bar(day, OPEN_IST_MINUTE + offset, 2_500_000))
                    .expect("ordinary bar");
            }
        }
        assert_eq!(
            refusal_probes().saturating_sub(before),
            3,
            "days 2, 3 and 4 each have one pending prior reference; day 5 has none"
        );
        assert_eq!(evaluator.reference_census().consumed, 3);
    }

    /// The pre-D-0942 step, kept verbatim as the oracle: walk the cursor on a
    /// copy, evaluate, commit only on success.
    fn oracle_step(
        evaluator: &mut AnchoredEvaluator<'_>,
        bar: &Candle,
    ) -> Result<(ConditionMask, ConditionMask, bool), Corrupt> {
        let mut next = *evaluator;
        let signal_day = crate::ist_day(bar.ts_micros);
        next.advance_before(signal_day);
        let warm = next.evaluator.warmed_up();
        let (mask, known) = next.evaluator.step_known(bar)?;
        next.charge_signal(signal_day);
        *evaluator = next;
        Ok((mask, known, warm && next.evaluator.warmed_up()))
    }

    fn zero_priced(day: i64, minute: i64) -> Candle {
        Candle::new(ts(day, minute), 0, 0, 0, 0, 0, OI_NULL)
    }

    /// W3-indicators1-0 / W3-indicators1-1 (D-0942). Every refused signal bar
    /// used to re-walk every daily reference between the committed cursor and
    /// its own day, so N refused bars over D unconsumed references cost N x D
    /// walk steps. The walk count is now exactly D for the whole run: the one
    /// accepted bar's walk, and nothing for any refusal. Every refusal kind
    /// that can arrive with a pending walk is covered: a zero price, a
    /// negative low, an inverted bar, a straddling range, a negative volume
    /// and an overflowing VWAP accumulator.
    #[test]
    fn refused_signal_bars_never_rewalk_the_daily_references() {
        const DAILY: i64 = 2_000;
        let references = (1..=DAILY)
            .map(|day| {
                if day % 7 == 0 {
                    excluded(day, 2_500_000)
                } else {
                    eligible(day, 2_500_000)
                }
            })
            .collect::<Vec<_>>();
        let mut evaluator = AnchoredEvaluator::new(
            Widths::pinned().expect("pinned widths are valid"),
            Availability::Present,
            Thresholds::CLASSICAL,
            &references,
        )
        .expect("fixture reference days are strictly increasing");
        let day = DAILY.saturating_add(1);
        let huge = 5_000_000_000_000_000_000;
        let kinds = [
            zero_priced(day, OPEN_IST_MINUTE),
            Candle::new(ts(day, OPEN_IST_MINUTE), 5, 10, -5, 5, 1, OI_NULL),
            Candle::new(ts(day, OPEN_IST_MINUTE), 100, 90, 110, 100, 1, OI_NULL),
            Candle::new(
                ts(day, OPEN_IST_MINUTE),
                0,
                i64::MAX,
                i64::MIN,
                0,
                1,
                OI_NULL,
            ),
            Candle::new(ts(day, OPEN_IST_MINUTE), 100, 110, 90, 100, -1, OI_NULL),
            Candle::new(ts(day, OPEN_IST_MINUTE), huge, huge, huge, huge, 1, OI_NULL),
        ];
        let refused_bars: u64 = 3_000;
        let before = reference_walk_steps();
        for (index, bar) in kinds.iter().cycle().take(3_000).enumerate() {
            assert!(
                evaluator.step(bar).is_err(),
                "fixture bar {index} must be refused"
            );
        }
        assert_eq!(
            reference_walk_steps().saturating_sub(before),
            0,
            "a refused bar walked daily references it then discarded"
        );
        assert_eq!(evaluator.cursor, 0, "a refusal moved the committed cursor");
        assert_eq!(evaluator.reference_census().consumed, 0);
        evaluator
            .step(&signal_bar(day, OPEN_IST_MINUTE, 2_500_000))
            .expect("an ordinary bar after the refusals is accepted");
        let walked = reference_walk_steps().saturating_sub(before);
        let daily = u64::try_from(DAILY).expect("positive");
        assert_eq!(
            walked, daily,
            "{refused_bars} refused bars and one accepted bar over {daily} references \
             must walk each reference exactly once"
        );
        assert_eq!(evaluator.reference_census().consumed, daily);
        assert!(evaluator.reference_census().reconciles());
    }

    /// The oracle's cost on the same shape, so the test above is measuring the
    /// defect and not a fixture that never walked: one refused bar over D
    /// unconsumed references walks D on the old path.
    #[test]
    fn the_pre_fix_step_walks_every_pending_reference_per_refusal() {
        let references = (1..=50)
            .map(|day| eligible(day, 2_500_000))
            .collect::<Vec<_>>();
        let mut evaluator = evaluator(&references);
        let before = reference_walk_steps();
        for _ in 0..10 {
            assert_eq!(
                oracle_step(&mut evaluator, &zero_priced(51, OPEN_IST_MINUTE)),
                Err(Corrupt::PriceNotPositive)
            );
        }
        assert_eq!(reference_walk_steps().saturating_sub(before), 500);
        let before = reference_walk_steps();
        for _ in 0..10 {
            assert_eq!(
                evaluator.step(&zero_priced(51, OPEN_IST_MINUTE)),
                Err(Corrupt::PriceNotPositive)
            );
        }
        assert_eq!(reference_walk_steps().saturating_sub(before), 0);
    }

    /// Mixed accepted and refused bars, including the case that makes
    /// committing a refused bar's walk unsafe: a refused bar on a LATER day
    /// followed by an accepted bar on an EARLIER one. Every step returns the
    /// same result as the pre-fix oracle, and after every step the cursor,
    /// installed reference, census, Prev5 fill, anchor and warmth agree.
    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one oracle comparison over one generated bar mix reads as a single table"
    )]
    fn mixed_refused_and_accepted_bars_match_the_pre_fix_step_exactly() {
        let references = (10_i64..60)
            .filter(|day| day % 5 != 3)
            .map(|day| {
                let base = 2_400_000_i64.saturating_add(day.saturating_mul(1_000));
                if day % 9 == 0 {
                    excluded(day, base)
                } else {
                    eligible(day, base)
                }
            })
            .collect::<Vec<_>>();
        for availability in [Availability::Absent, Availability::Present] {
            let build = || {
                AnchoredEvaluator::new(
                    Widths::pinned().expect("pinned widths are valid"),
                    availability,
                    Thresholds::CLASSICAL,
                    &references,
                )
                .expect("fixture reference days are strictly increasing")
            };
            let mut fixed = build();
            let mut oracle = build();
            let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
            let mut day: i64 = 8;
            let mut minute: i64 = OPEN_IST_MINUTE;
            let mut accepted = 0_u32;
            let mut refused = 0_u32;
            for step in 0..4_000_u32 {
                seed = seed
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let roll = (seed >> 33) % 100;
                let price = 2_450_000_i64
                    .saturating_add(day.saturating_mul(1_000))
                    .saturating_add(i64::try_from((seed >> 20) % 4_000).expect("small"));
                let bar = match roll {
                    0..=59 => {
                        minute = minute.saturating_add(1);
                        if minute > OPEN_IST_MINUTE + 374 || roll < 6 {
                            day = day.saturating_add(1 + i64::from(roll.is_multiple_of(3)));
                            minute = OPEN_IST_MINUTE;
                        }
                        signal_bar(day, minute, price)
                    }
                    60..=69 => zero_priced(
                        day.saturating_add(i64::try_from(roll % 7).expect("small")),
                        minute,
                    ),
                    70..=74 => zero_priced(day.saturating_add(30), minute),
                    75..=79 => Candle::new(
                        ts(day.saturating_add(2), OPEN_IST_MINUTE),
                        price,
                        price,
                        price,
                        price,
                        -1,
                        OI_NULL,
                    ),
                    80..=84 => signal_bar(day, minute, price),
                    85..=89 => signal_bar(day, minute.saturating_sub(1), price),
                    90..=94 => {
                        let huge = 5_000_000_000_000_000_000;
                        Candle::new(
                            ts(day.saturating_add(4), OPEN_IST_MINUTE),
                            huge,
                            huge,
                            huge,
                            huge,
                            1,
                            OI_NULL,
                        )
                    }
                    _ => Candle::new(
                        ts(day.saturating_add(3), minute),
                        100,
                        90,
                        110,
                        100,
                        1,
                        OI_NULL,
                    ),
                };
                let got = fixed.step_with_warmth(&bar);
                let want = oracle_step(&mut oracle, &bar);
                assert_eq!(got, want, "step {step} ({availability:?}) bar {bar:?}");
                if got.is_ok() {
                    accepted = accepted.saturating_add(1);
                } else {
                    refused = refused.saturating_add(1);
                }
                assert_eq!(fixed.cursor, oracle.cursor, "step {step}");
                assert_eq!(
                    fixed.current_reference(),
                    oracle.current_reference(),
                    "step {step}"
                );
                assert_eq!(
                    fixed.reference_census(),
                    oracle.reference_census(),
                    "step {step}"
                );
                assert_eq!(
                    fixed.sessions_completed(),
                    oracle.sessions_completed(),
                    "step {step}"
                );
                assert_eq!(fixed.has_yesterday(), oracle.has_yesterday(), "step {step}");
                assert_eq!(fixed.warmed_up(), oracle.warmed_up(), "step {step}");
            }
            assert!(
                accepted > 500 && refused > 500,
                "{accepted} accepted, {refused} refused"
            );
            assert!(fixed.reference_census().installed > 10);
        }
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
