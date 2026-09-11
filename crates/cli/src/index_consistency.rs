//! Version-one index day/week consistency, additional to institutional checks.
//!
//! Calendar decisions reuse the existing NSE calendar and swept-series exclusion
//! policy. An explicit zero-trade session is evidence; an absent session is not.
//! Each civil-day update has bounded state/work. Evaluating a span is O(days);
//! retaining week rows additionally uses O(weeks) space. Gross paisa exclude costs.

use brutex_core::blake3::{Hasher, hash};
use pull::calendar::{DayKind, kind_of};
use pull::session::Day;
use runner::research_family::ResearchFamilyV1;
use std::fmt;

/// Exact fixed widths of the additive version-one observations.
pub const POLICY_BYTES: usize = 72;
/// One explicit eligible-session observation.
pub const SESSION_BYTES: usize = 40;
/// Fixed aggregate counters.
pub const SUMMARY_BYTES: usize = 176;
/// One civil week's counters, without a variable reason string.
pub const WEEK_BYTES: usize = 144;
/// Fixed result header; optional week rows are separately encoded and authenticated.
pub const EVALUATION_BYTES: usize = 536;

const _: () = assert!(crate::stored::SWEPT_SERIES_CALENDAR_POLICY == 1);
const WEEK_DOMAIN: &[u8] = b"brutex-index-consistency-weeks-v1\0";
const SESSION_DOMAIN: &[u8] = b"brutex-index-consistency-sessions-v1\0";
const CALENDAR_DOMAIN: &[u8] = b"brutex-index-consistency-calendar-v1\0";

/// Approved policy. V1 and V2 require >=3/5 winning eligible days, >=3 wins and
/// <=2 losses per complete Monday-Friday five-session week, and <=2 losing days
/// per streak; [`Policy::V3`] restates all five for the rare-winner shape.
/// Flat and no-trade days preserve a streak; only a winning day resets it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    /// The approved version-one rule: a day is a win or a loss by the SIGN of
    /// its pessimistic sum alone. Its bytes, thresholds and digest are frozen.
    V1,
    /// The same thresholds, counting only days whose outcome survives BOTH
    /// readings — D-0596.
    ///
    /// # What V1 could not say
    ///
    /// Every threshold here counts DAYS, and V1 classified a day by the sign of
    /// one number. A day netting a single paisa was therefore the identical
    /// unit of evidence as a day netting fifty thousand rupees, and a day that
    /// lost a paisa under the worst reading while winning handsomely under the
    /// best was filed as a loss. For a strategy that wins rarely and hugely
    /// that is not a rounding — it is the whole input to the rule being blind
    /// to the thing the strategy is made of.
    ///
    /// # What V2 says instead, and why it needs no new number
    ///
    /// A day is a WIN when its pessimistic sum exceeds its own
    /// [`Session::bracket_paisa`], and a LOSS when even its optimistic sum is
    /// negative. Anything else is a scratch day: its sign depends on which
    /// admissible ordering of its own trades you read, so it is evidence for
    /// neither side and is treated exactly as a no-trade day already is.
    ///
    /// The threshold is the day's own execution uncertainty, measured rather
    /// than declared — the same principle D-0595 applied to a single trade. No
    /// constant is introduced and none of the five thresholds moves.
    ///
    /// V1 is retained and is still the default everywhere; §3 rule 8 forbids
    /// mutating a shipped policy, and the two answer different questions.
    V2,
    /// V2's magnitude test with all five thresholds restated for the operator's
    /// stated objective: a setup that fires seldom, loses tiny, and pays
    /// enormously.
    ///
    /// # Why V1's and V2's five numbers could not admit that shape
    ///
    /// Measured, not argued. A complete 60min sweep of 16,000 candidates over
    /// 2024-05..2026-08 admitted NOTHING, and not one candidate was rejected on
    /// financial grounds -- the entire leaderboard failed these four day rules
    /// instead. Its best row earned +Rs 2,720.35 out of sample on data it had
    /// never seen, won 220 of 577 eligible days, and ran an 11-day losing
    /// streak. A strategy that pays in bursts is flat or slightly down on most
    /// days BY CONSTRUCTION, so >=3/5 winning days and a 2-day streak cap do
    /// not make the rare winner harder to find -- they make it unrepresentable.
    ///
    /// # Where these five numbers come from
    ///
    /// Not taste, and not the leaderboard: reverse-fitting a threshold to admit
    /// a row already seen is the exact error this apparatus exists to refuse.
    /// They are derived from thresholds the admission policy ALREADY carries.
    /// `min_worst_reward_risk_ppm` demands the smallest win clear 3x the
    /// largest loss, and `min_profit_factor_ppm` demands gross profit clear
    /// 1.5x gross loss. One win at 3x against two losses at 1x is a profit
    /// factor of exactly 1.5 -- so 1/3 is the single ratio at which the day
    /// rule stops contradicting the two trade rules already in force. Below it
    /// the day rule would admit what the profit-factor gate rejects anyway;
    /// above it the day rule is stricter than the policy's own economics ask.
    /// `min_weekly_wins` becomes 1 because 1/3 of a five-session week is 1.67,
    /// `max_weekly_losses` becomes 4 as its complement, and the streak cap
    /// becomes 10 to match `max_consecutive_losing_streak`, which the policy
    /// already permits over TRADES -- the same tolerance at both granularities
    /// rather than a sixth new number.
    ///
    /// V1 and V2 are untouched and still decode to their own frozen digests.
    V3,
}
impl Policy {
    /// Fixed encoding width for callers storing the additive policy record.
    pub const BYTE_LEN: usize = POLICY_BYTES;
    /// Whether a day must clear its own bracket to count — false on [`Self::V1`].
    #[must_use]
    pub const fn magnitude_aware(self) -> bool {
        matches!(self, Self::V2 | Self::V3)
    }
    /// The encoded version word: `1`, `2` or `3`.
    #[must_use]
    pub const fn version(self) -> u64 {
        match self {
            Self::V1 => 1,
            Self::V2 => 2,
            Self::V3 => 3,
        }
    }
    /// Required winning-day ratio numerator.
    #[must_use]
    pub const fn winning_day_numerator(self) -> u64 {
        match self {
            Self::V1 | Self::V2 => 3,
            Self::V3 => 1,
        }
    }
    /// Required winning-day ratio denominator.
    #[must_use]
    pub const fn winning_day_denominator(self) -> u64 {
        match self {
            Self::V1 | Self::V2 => 5,
            Self::V3 => 3,
        }
    }
    /// Minimum winning days in a complete five-session week.
    #[must_use]
    pub const fn min_weekly_wins(self) -> u64 {
        match self {
            Self::V1 | Self::V2 => 3,
            Self::V3 => 1,
        }
    }
    /// Maximum losing days in a complete five-session week.
    #[must_use]
    pub const fn max_weekly_losses(self) -> u64 {
        match self {
            Self::V1 | Self::V2 => 2,
            Self::V3 => 4,
        }
    }
    /// Maximum losses before a winning day resets the streak.
    #[must_use]
    pub const fn max_losing_day_streak(self) -> u64 {
        match self {
            Self::V1 | Self::V2 => 2,
            Self::V3 => 10,
        }
    }

    /// Canonical version, thresholds, zero-day rule and existing calendar policy.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; POLICY_BYTES] {
        words(
            *b"BRICPO01",
            &[
                // THE VERSION WORD CARRIES THE SEMANTIC CHANGE, and no width
                // does. V2 keeps all five thresholds and the two rule codes
                // exactly as V1 states them -- what differs is which days reach
                // the counters -- so the record stays 72 bytes and V1's bytes,
                // and therefore V1's digest, are untouched. D-0596.
                //
                // V3 is the first version to move a threshold, and it moves
                // five. That is precisely why it is a NEW WORD rather than an
                // edit: the five numbers are already words two through six of
                // this record, so a changed threshold is a changed digest and
                // an old receipt can never silently acquire a new meaning. The
                // width is still 72 bytes and V1's and V2's bytes are still
                // exactly what they were.
                self.version(),
                self.winning_day_numerator(),
                self.winning_day_denominator(),
                self.min_weekly_wins(),
                self.max_weekly_losses(),
                self.max_losing_day_streak(),
                1,
                1,
            ],
        )
    }
    /// Exact policy identity.
    #[must_use]
    pub fn digest(self) -> [u8; 32] {
        hash(&self.canonical_bytes())
    }
    /// Every approved version, oldest first. Appending here is the ONLY step a
    /// new version needs to become decodable everywhere.
    pub const APPROVED: [Self; 3] = [Self::V1, Self::V2, Self::V3];

    /// Recover the version a stored digest was written under.
    ///
    /// A caller that pinned a policy keeps its digest rather than its name, so
    /// without this it must either hardcode a version -- which is how the store
    /// came to carry `Policy::V1` at four sites that could not then read
    /// anything else -- or refuse a record it can perfectly well authenticate.
    #[must_use]
    pub fn from_digest(digest: [u8; 32]) -> Option<Self> {
        Self::APPROVED
            .into_iter()
            .find(|policy| policy.digest() == digest)
    }

    /// Decode the approved version without accepting edited thresholds.
    ///
    /// # Errors
    /// Unsupported versions, widths, thresholds or rule codes are refused.
    pub fn decode(raw: &[u8]) -> Result<Self, DecodeError> {
        if raw == Self::V1.canonical_bytes() {
            Ok(Self::V1)
        } else if raw == Self::V2.canonical_bytes() {
            Ok(Self::V2)
        } else if raw == Self::V3.canonical_bytes() {
            Ok(Self::V3)
        } else {
            Err(DecodeError::Format)
        }
    }
}

/// Saved gross trade returns on one IST day, under BOTH readings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Session {
    /// Civil IST day since 1970-01-01, not a UTC trade timestamp.
    pub day: i64,
    /// Sum of the day's pessimistic gross returns, in integer paisa.
    pub pessimistic_paisa: i64,
    /// Sum of the day's OPTIMISTIC gross returns, in integer paisa — D-0596.
    ///
    /// Never below [`Self::pessimistic_paisa`]: the two are one day priced
    /// under the worst and the best ordering of its trades, so the one called
    /// pessimistic must be the low end. [`Self::decode`] refuses a record that
    /// says otherwise rather than quietly ordering them.
    ///
    /// # Why the record grew
    ///
    /// A day was classified a win or a loss by the SIGN of the pessimistic sum
    /// alone, so a day netting one paisa was the identical unit of evidence as
    /// a day netting fifty thousand rupees, and a day that lost a paisa under
    /// the worst reading and won handsomely under the best was filed as a loss.
    /// Every threshold in [`Policy`] counts days, so that classification is the
    /// whole input to the rule — and a magnitude-aware rule needs the second
    /// reading, which this record did not carry.
    pub optimistic_paisa: i64,
    /// Actual trades; zero requires a zero return.
    pub trades: u64,
}
impl Session {
    /// Fixed encoding width.
    pub const BYTE_LEN: usize = SESSION_BYTES;
    /// Fixed exact observation, including an explicit no-trade day.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; SESSION_BYTES] {
        words(
            *b"BRICDY02",
            &[
                bits(self.day),
                bits(self.pessimistic_paisa),
                bits(self.optimistic_paisa),
                self.trades,
            ],
        )
    }
    /// Exact observation identity.
    #[must_use]
    pub fn digest(self) -> [u8; 32] {
        hash(&self.canonical_bytes())
    }
    /// Decode an individual observation. Ordering/eligibility belong to evaluation.
    ///
    /// # Both widths are accepted, and a V1 row keeps its meaning exactly
    ///
    /// `BRICDY01` carried three words and no optimistic reading. It decodes
    /// with `optimistic_paisa` set EQUAL to the pessimistic one, which gives a
    /// zero bracket — and a zero bracket reproduces the old sign-only
    /// classification exactly, whichever [`Policy`] reads it. An existing row
    /// therefore says what it always said rather than acquiring a reading
    /// nobody measured. A V1 row re-encodes as `BRICDY02` and so hashes
    /// differently; no such row exists in any store today, and D-0596 records
    /// that rather than leaving it to be discovered.
    ///
    /// # Errors
    /// Bad civil days, widths, magic, an optimistic reading below the
    /// pessimistic one, or zero-trade nonzero returns are refused.
    pub fn decode(raw: &[u8]) -> Result<Self, DecodeError> {
        let row = if let Ok([day, pessimistic, optimistic, trades]) =
            read_words::<4>(raw, *b"BRICDY02")
        {
            Self {
                day: signed(day),
                pessimistic_paisa: signed(pessimistic),
                optimistic_paisa: signed(optimistic),
                trades,
            }
        } else {
            let [day, pessimistic, trades] = read_words::<3>(raw, *b"BRICDY01")?;
            Self {
                day: signed(day),
                pessimistic_paisa: signed(pessimistic),
                optimistic_paisa: signed(pessimistic),
                trades,
            }
        };
        valid_day(row.day)?;
        if row.optimistic_paisa < row.pessimistic_paisa {
            return Err(DecodeError::Inconsistent);
        }
        if row.trades == 0 && (row.pessimistic_paisa != 0 || row.optimistic_paisa != 0) {
            return Err(DecodeError::Inconsistent);
        }
        Ok(row)
    }

    /// The day's own execution uncertainty, in paisa. Never negative.
    #[must_use]
    pub const fn bracket_paisa(self) -> i64 {
        self.optimistic_paisa.saturating_sub(self.pessimistic_paisa)
    }
}

/// Consistency result, never a replacement for an institutional verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum Outcome {
    /// All three consistency conditions were measured and satisfied.
    Passed = 0,
    /// Complete evidence failed at least one consistency threshold.
    Failed = 1,
    /// Calendar or complete-week evidence cannot answer the policy.
    Unmeasured = 2,
    /// Missing, malformed, contradictory or unrepresentable evidence.
    Refused = 3,
    /// Cash research is outside this index-only policy.
    NotApplicable = 4,
}
impl Outcome {
    /// Stable machine/UI label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Unmeasured => "unmeasured",
            Self::Refused => "refused",
            Self::NotApplicable => "not_applicable",
        }
    }
    fn decode(value: u64) -> Result<Self, DecodeError> {
        match value {
            0 => Ok(Self::Passed),
            1 => Ok(Self::Failed),
            2 => Ok(Self::Unmeasured),
            3 => Ok(Self::Refused),
            4 => Ok(Self::NotApplicable),
            _ => Err(DecodeError::Format),
        }
    }
}

/// Stable independent reason bits; several checks can fail simultaneously.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum Reason {
    /// Winning eligible-day ratio is less than three fifths.
    WinningDayRatio = 1,
    /// A complete week has fewer than three winning days.
    WeeklyWins = 1 << 1,
    /// A complete week has more than two losing days.
    WeeklyLosses = 1 << 2,
    /// More than two losing days occurred without an intervening winning day.
    LosingDayStreak = 1 << 3,
    /// No complete five-session Monday-Friday week was measured.
    NoCompleteWeek = 1 << 4,
    /// No eligible session is available for the daily ratio.
    NoEligibleSession = 1 << 5,
    /// An expected eligible session has no explicit saved observation.
    MissingSession = 1 << 6,
    /// The shared calendar cannot establish a required day's eligibility.
    UnmeasuredCalendar = 1 << 7,
    /// Invalid civil days or a reversed requested span.
    InvalidSpan = 1 << 8,
    /// Duplicate, reversed or out-of-span input session rows.
    InvalidOrder = 1 << 9,
    /// A row claims trading on a closed or explicitly excluded session.
    UnexpectedSession = 1 << 10,
    /// More input days than the span, or zero trades with nonzero return.
    InvalidCountOrReturn = 1 << 11,
    /// A trade count or integer-paisa sum cannot be represented exactly.
    ArithmeticOverflow = 1 << 12,
    /// Optional retained week rows could not be allocated.
    AllocationRefused = 1 << 13,
}
impl Reason {
    /// Exact public bit, suitable for persisted comparisons.
    #[must_use]
    pub const fn mask(self) -> u64 {
        self as u64
    }
    /// Stable explanatory label for a named bit.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WinningDayRatio => "winning_day_ratio_below_three_fifths",
            Self::WeeklyWins => "complete_week_has_fewer_than_three_wins",
            Self::WeeklyLosses => "complete_week_has_more_than_two_losses",
            Self::LosingDayStreak => "losing_day_streak_exceeds_two",
            Self::NoCompleteWeek => "no_complete_five_session_week",
            Self::NoEligibleSession => "no_eligible_session",
            Self::MissingSession => "missing_expected_session",
            Self::UnmeasuredCalendar => "unmeasured_calendar",
            Self::InvalidSpan => "invalid_requested_span",
            Self::InvalidOrder => "invalid_session_order_or_span",
            Self::UnexpectedSession => "session_on_closed_or_excluded_day",
            Self::InvalidCountOrReturn => "invalid_session_count_or_return",
            Self::ArithmeticOverflow => "exact_arithmetic_overflow",
            Self::AllocationRefused => "week_output_allocation_refused",
        }
    }
}
const KNOWN_REASONS: u64 = (1 << 14) - 1;
const REFUSALS: u64 = Reason::MissingSession.mask()
    | Reason::InvalidSpan.mask()
    | Reason::InvalidOrder.mask()
    | Reason::UnexpectedSession.mask()
    | Reason::InvalidCountOrReturn.mask()
    | Reason::ArithmeticOverflow.mask()
    | Reason::AllocationRefused.mask();
const UNMEASURED: u64 = Reason::UnmeasuredCalendar.mask()
    | Reason::NoCompleteWeek.mask()
    | Reason::NoEligibleSession.mask();

/// Disjoint day outcomes: winning + losing + flat-with-trades + no-trade = observed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    /// Civil days classified inside the inclusive requested span.
    pub calendar_days: u64,
    /// Expected eligible trading days, including missing observations.
    pub eligible_days: u64,
    /// Explicit eligible-day observations.
    pub observed_days: u64,
    /// Positive summed pessimistic return days.
    pub winning_days: u64,
    /// Negative summed pessimistic return days.
    pub losing_days: u64,
    /// Flat days with at least one trade.
    pub zero_days: u64,
    /// Explicit eligible days with no trades and zero return.
    pub no_trade_days: u64,
    /// Expected eligible days whose saved observation is absent.
    pub missing_days: u64,
    /// Civil days whose eligibility the shared calendar cannot answer.
    pub unmeasured_days: u64,
    /// Existing charter exclusions, kept separate from closure.
    pub excluded_days: u64,
    /// Calendar-confirmed closed civil days.
    pub closed_days: u64,
    /// Eligible Saturday/Sunday sessions; included in the daily checks.
    pub weekend_sessions: u64,
    /// Actual measured trades across eligible days.
    pub trades: u64,
    /// Exact summed gross pessimistic return in paisa.
    pub pessimistic_paisa: i64,
    /// Largest losing-day count without an intervening winning day.
    pub longest_losing_streak: u64,
    /// Full Monday-Friday weeks with five expected eligible weekday sessions.
    pub complete_weeks: u64,
    /// Complete measured weeks satisfying the weekly thresholds.
    pub passing_weeks: u64,
    /// Complete measured weeks failing the weekly thresholds.
    pub failing_weeks: u64,
    /// Whole weekday windows shortened by closures/exclusions.
    pub short_weeks: u64,
    /// Boundary windows without all Monday-Friday dates.
    pub partial_weeks: u64,
    /// Whole weekday windows with unknown calendar facts.
    pub unmeasured_weeks: u64,
}
impl Summary {
    /// Fixed encoding width.
    pub const BYTE_LEN: usize = SUMMARY_BYTES;
    /// Fixed aggregate representation; no rounded ratios are stored.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; SUMMARY_BYTES] {
        words(
            *b"BRICSU01",
            &[
                self.calendar_days,
                self.eligible_days,
                self.observed_days,
                self.winning_days,
                self.losing_days,
                self.zero_days,
                self.no_trade_days,
                self.missing_days,
                self.unmeasured_days,
                self.excluded_days,
                self.closed_days,
                self.weekend_sessions,
                self.trades,
                bits(self.pessimistic_paisa),
                self.longest_losing_streak,
                self.complete_weeks,
                self.passing_weeks,
                self.failing_weeks,
                self.short_weeks,
                self.partial_weeks,
                self.unmeasured_weeks,
            ],
        )
    }
    /// Exact aggregate identity.
    #[must_use]
    pub fn digest(self) -> [u8; 32] {
        hash(&self.canonical_bytes())
    }
    /// Decode and reconcile disjoint counters before exposing an observation.
    ///
    /// # Errors
    /// Unknown format, overflowing or inconsistent totals are refused.
    pub fn decode(raw: &[u8]) -> Result<Self, DecodeError> {
        let [
            calendar_days,
            eligible_days,
            observed_days,
            winning_days,
            losing_days,
            zero_days,
            no_trade_days,
            missing_days,
            unmeasured_days,
            excluded_days,
            closed_days,
            weekend_sessions,
            trades,
            pessimistic,
            longest_losing_streak,
            complete_weeks,
            passing_weeks,
            failing_weeks,
            short_weeks,
            partial_weeks,
            unmeasured_weeks,
        ] = read_words(raw, *b"BRICSU01")?;
        if sum(&[winning_days, losing_days, zero_days, no_trade_days])? != observed_days
            || sum(&[observed_days, missing_days])? != eligible_days
            || sum(&[eligible_days, unmeasured_days, excluded_days, closed_days])? != calendar_days
            || weekend_sessions > eligible_days
            || longest_losing_streak > losing_days
            || sum(&[passing_weeks, failing_weeks])? > complete_weeks
            || trades < observed_days.saturating_sub(no_trade_days)
        {
            return Err(DecodeError::Inconsistent);
        }
        Ok(Self {
            calendar_days,
            eligible_days,
            observed_days,
            winning_days,
            losing_days,
            zero_days,
            no_trade_days,
            missing_days,
            unmeasured_days,
            excluded_days,
            closed_days,
            weekend_sessions,
            trades,
            pessimistic_paisa: signed(pessimistic),
            longest_losing_streak,
            complete_weeks,
            passing_weeks,
            failing_weeks,
            short_weeks,
            partial_weeks,
            unmeasured_weeks,
        })
    }
}

/// A civil week is only eligible for the weekly test when all five weekday
/// dates are in the requested span and are expected eligible trading sessions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum WeekKind {
    /// Complete Monday-Friday five-session week.
    Complete = 0,
    /// Full weekday window shortened by the existing calendar/exclusion policy.
    Short = 1,
    /// Requested boundary omits one or more Monday-Friday dates.
    Partial = 2,
    /// Full weekday window has unmeasured calendar facts.
    Unmeasured = 3,
}
impl WeekKind {
    /// Stable UI label; a short calendar week does not invent a closure's cause.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Short => "short",
            Self::Partial => "partial",
            Self::Unmeasured => "unmeasured",
        }
    }
    fn decode(value: u64) -> Result<Self, DecodeError> {
        match value {
            0 => Ok(Self::Complete),
            1 => Ok(Self::Short),
            2 => Ok(Self::Partial),
            3 => Ok(Self::Unmeasured),
            _ => Err(DecodeError::Format),
        }
    }
}

/// Monday-Friday totals. Weekend sessions are reported separately and continue
/// to affect the overall daily ratio and cross-week losing-day streak.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Week {
    /// Epoch civil day of Monday; can precede the requested start.
    pub monday: i64,
    /// Whether this week can support the weekly condition.
    pub kind: WeekKind,
    /// Weekly outcome only; short/partial rows are not applicable, never passes.
    pub outcome: Outcome,
    /// Monday-Friday dates actually inside the requested span.
    pub weekday_days: u64,
    /// Expected eligible weekday sessions.
    pub eligible_days: u64,
    /// Explicit eligible weekday observations.
    pub observed_days: u64,
    /// Positive weekday sums.
    pub winning_days: u64,
    /// Negative weekday sums.
    pub losing_days: u64,
    /// Flat weekday sums with trades.
    pub zero_days: u64,
    /// Explicit weekdays without trades.
    pub no_trade_days: u64,
    /// Missing expected weekday observations.
    pub missing_days: u64,
    /// Unknown weekday calendar facts.
    pub unmeasured_days: u64,
    /// Existing excluded weekdays.
    pub excluded_days: u64,
    /// Calendar-confirmed closed weekdays.
    pub closed_weekdays: u64,
    /// Eligible weekend days, outside this row's weekday totals.
    pub weekend_sessions: u64,
    /// Actual trades on measured eligible weekdays.
    pub trades: u64,
    /// Exact gross pessimistic weekday return in paisa.
    pub pessimistic_paisa: i64,
}
impl Week {
    /// Fixed encoding width.
    pub const BYTE_LEN: usize = WEEK_BYTES;
    fn empty(monday: i64) -> Self {
        Self {
            monday,
            kind: WeekKind::Partial,
            outcome: Outcome::NotApplicable,
            weekday_days: 0,
            eligible_days: 0,
            observed_days: 0,
            winning_days: 0,
            losing_days: 0,
            zero_days: 0,
            no_trade_days: 0,
            missing_days: 0,
            unmeasured_days: 0,
            excluded_days: 0,
            closed_weekdays: 0,
            weekend_sessions: 0,
            trades: 0,
            pessimistic_paisa: 0,
        }
    }
    fn finish(mut self) -> Self {
        self.kind = if self.weekday_days < 5 {
            WeekKind::Partial
        } else if self.unmeasured_days > 0 {
            WeekKind::Unmeasured
        } else if self.eligible_days < 5 {
            WeekKind::Short
        } else {
            WeekKind::Complete
        };
        self.outcome = if self.missing_days > 0 {
            Outcome::Refused
        } else if self.unmeasured_days > 0 {
            Outcome::Unmeasured
        } else if self.kind != WeekKind::Complete {
            Outcome::NotApplicable
        } else if self.winning_days >= Policy::V1.min_weekly_wins()
            && self.losing_days <= Policy::V1.max_weekly_losses()
        {
            Outcome::Passed
        } else {
            Outcome::Failed
        };
        self
    }
    /// Canonical fixed row.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; WEEK_BYTES] {
        words(
            *b"BRICWK01",
            &[
                bits(self.monday),
                self.kind as u64,
                self.outcome as u64,
                self.weekday_days,
                self.eligible_days,
                self.observed_days,
                self.winning_days,
                self.losing_days,
                self.zero_days,
                self.no_trade_days,
                self.missing_days,
                self.unmeasured_days,
                self.excluded_days,
                self.closed_weekdays,
                self.weekend_sessions,
                self.trades,
                bits(self.pessimistic_paisa),
            ],
        )
    }
    /// Exact week identity.
    #[must_use]
    pub fn digest(self) -> [u8; 32] {
        hash(&self.canonical_bytes())
    }
    /// Decode and reconcile the row's calendar class and weekly verdict.
    ///
    /// # Errors
    /// Invalid format, impossible counts, non-Monday keys or invented passes refuse.
    pub fn decode(raw: &[u8]) -> Result<Self, DecodeError> {
        let [
            monday,
            kind,
            outcome,
            weekday_days,
            eligible_days,
            observed_days,
            winning_days,
            losing_days,
            zero_days,
            no_trade_days,
            missing_days,
            unmeasured_days,
            excluded_days,
            closed_weekdays,
            weekend_sessions,
            trades,
            pessimistic,
        ] = read_words(raw, *b"BRICWK01")?;
        let row = Self {
            monday: signed(monday),
            kind: WeekKind::decode(kind)?,
            outcome: Outcome::decode(outcome)?,
            weekday_days,
            eligible_days,
            observed_days,
            winning_days,
            losing_days,
            zero_days,
            no_trade_days,
            missing_days,
            unmeasured_days,
            excluded_days,
            closed_weekdays,
            weekend_sessions,
            trades,
            pessimistic_paisa: signed(pessimistic),
        };
        if row.monday < -3
            || valid_day(row.monday.max(0)).is_err()
            || row
                .monday
                .checked_add(3)
                .is_none_or(|d| d.rem_euclid(7) != 0)
            || weekday_days > 5
            || weekend_sessions > 2
            || sum(&[observed_days, missing_days])? != eligible_days
            || sum(&[winning_days, losing_days, zero_days, no_trade_days])? != observed_days
            || sum(&[
                eligible_days,
                unmeasured_days,
                excluded_days,
                closed_weekdays,
            ])? != weekday_days
            || trades < observed_days.saturating_sub(no_trade_days)
            || row.finish() != row
        {
            return Err(DecodeError::Inconsistent);
        }
        Ok(row)
    }
}

/// Detached exact policy observation. Source receipts are bound by its caller.
/// Week retention does not change the fixed canonical bytes or result identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evaluation {
    /// Exact versioned policy.
    pub policy: Policy,
    /// Exact research family, never a loosely matched symbol.
    pub family: ResearchFamilyV1,
    /// Inclusive requested IST civil start.
    pub first_day: i64,
    /// Inclusive requested IST civil end.
    pub last_day: i64,
    /// Consistency result separate from institutional qualification.
    pub outcome: Outcome,
    /// Independent reason bits.
    pub reasons: u64,
    /// First reported issue day (a failed week uses its Monday), if available.
    pub first_issue_day: Option<i64>,
    /// Exact summary; refused partial work never represents complete evidence.
    pub summary: Summary,
    /// Optional retained week rows; empty when retention was not requested.
    pub weeks: Vec<Week>,
    /// Number of completed emitted week rows, independent of retention.
    pub week_count: u64,
    /// Ordered digest of all emitted canonical week rows.
    pub weeks_digest: [u8; 32],
    /// Ordered digest of supplied explicit session rows.
    pub sessions_digest: [u8; 32],
    /// Ordered digest of actual shared calendar facts used in this span.
    pub calendar_digest: [u8; 32],
}
impl Evaluation {
    /// Fixed header width; detailed weeks are separate fixed rows.
    pub const BYTE_LEN: usize = EVALUATION_BYTES;
    /// Fixed record binding policy, family, span, reasons, aggregates and detail hashes.
    #[must_use]
    pub fn canonical_bytes(&self) -> [u8; EVALUATION_BYTES] {
        let span: [u8; 64] = words(
            *b"BRICEV01",
            &[
                bits(self.first_day),
                bits(self.last_day),
                self.outcome as u64,
                self.reasons,
                u64::from(self.first_issue_day.is_some()),
                bits(self.first_issue_day.unwrap_or(0)),
                self.week_count,
            ],
        );
        parts(&[
            &span,
            &self.policy.canonical_bytes(),
            &self.family.encode(),
            &self.summary.canonical_bytes(),
            &self.weeks_digest,
            &self.sessions_digest,
            &self.calendar_digest,
        ])
    }
    /// Stable result identity, identical with and without retained week rows.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        hash(&self.canonical_bytes())
    }
    /// Reopen a detached observation; optional rows are authenticated without rerunning prices.
    ///
    /// # Errors
    /// Bad format/scope, inconsistent outcomes, reordered or altered weeks refuse.
    pub fn decode(raw: &[u8], weeks: Option<&[Week]>) -> Result<Self, DecodeError> {
        if raw.len() != EVALUATION_BYTES {
            return Err(DecodeError::Format);
        }
        let mut read = Read { raw, at: 0 };
        let [first, last, outcome, reasons, has_issue, issue, week_count] =
            read_words(&read.take::<64>()?, *b"BRICEV01")?;
        let policy = Policy::decode(&read.take::<POLICY_BYTES>()?)?;
        let family =
            ResearchFamilyV1::decode(&read.take::<128>()?).map_err(|_| DecodeError::Scope)?;
        let summary = Summary::decode(&read.take::<SUMMARY_BYTES>()?)?;
        let mut result = Self {
            policy,
            family,
            first_day: signed(first),
            last_day: signed(last),
            outcome: Outcome::decode(outcome)?,
            reasons,
            first_issue_day: (has_issue == 1).then_some(signed(issue)),
            summary,
            weeks: Vec::new(),
            week_count,
            weeks_digest: read.take()?,
            sessions_digest: read.take()?,
            calendar_digest: read.take()?,
        };
        if has_issue > 1
            || (has_issue == 0 && issue != 0)
            || reasons & !KNOWN_REASONS != 0
            || result.outcome != outcome_for(family, reasons)
            || (result.outcome != Outcome::NotApplicable
                && reasons & Reason::InvalidSpan.mask() == 0
                && (valid_day(result.first_day).is_err()
                    || valid_day(result.last_day).is_err()
                    || result.first_day > result.last_day))
            || sum(&[
                summary.complete_weeks,
                summary.short_weeks,
                summary.partial_weeks,
                summary.unmeasured_weeks,
            ])? != week_count
        {
            return Err(DecodeError::Inconsistent);
        }
        result.validate_summary()?;
        if let Some(rows) = weeks {
            result.attach_weeks(rows)?;
        }
        if result.canonical_bytes().as_slice() != raw {
            return Err(DecodeError::Format);
        }
        Ok(result)
    }
    fn attach_weeks(&mut self, rows: &[Week]) -> Result<(), DecodeError> {
        if rows.len() as u64 != self.week_count {
            return Err(DecodeError::Inconsistent);
        }
        let mut hash = domain(WEEK_DOMAIN);
        let mut previous = None;
        for row in rows {
            Week::decode(&row.canonical_bytes())?;
            if previous.is_some_and(|day: i64| day.checked_add(7) != Some(row.monday)) {
                return Err(DecodeError::Inconsistent);
            }
            hash.update(&row.canonical_bytes());
            previous = Some(row.monday);
        }
        if let Some(first) = rows.first()
            && (valid_day(self.first_day).is_err() || first.monday != monday(self.first_day))
        {
            return Err(DecodeError::Inconsistent);
        }
        if self.outcome != Outcome::Refused && self.outcome != Outcome::NotApplicable {
            let counts = rows.iter().try_fold([0_u64; 6], |mut counts, row| {
                let kind = match row.kind {
                    WeekKind::Complete => 0,
                    WeekKind::Short => 1,
                    WeekKind::Partial => 2,
                    WeekKind::Unmeasured => 3,
                };
                let count = counts.get_mut(kind).ok_or(DecodeError::Inconsistent)?;
                *count = count.checked_add(1).ok_or(DecodeError::Inconsistent)?;
                if row.outcome == Outcome::Passed {
                    counts[4] += 1;
                }
                if row.outcome == Outcome::Failed {
                    counts[5] += 1;
                }
                Ok(counts)
            })?;
            if counts
                != [
                    self.summary.complete_weeks,
                    self.summary.short_weeks,
                    self.summary.partial_weeks,
                    self.summary.unmeasured_weeks,
                    self.summary.passing_weeks,
                    self.summary.failing_weeks,
                ]
            {
                return Err(DecodeError::Inconsistent);
            }
        }
        if hash.finalize() != self.weeks_digest {
            return Err(DecodeError::Inconsistent);
        }
        self.weeks
            .try_reserve_exact(rows.len())
            .map_err(|_| DecodeError::Allocation)?;
        self.weeks.extend_from_slice(rows);
        Ok(())
    }
    fn validate_summary(&self) -> Result<(), DecodeError> {
        let s = self.summary;
        if self.outcome == Outcome::NotApplicable {
            if s != Summary::default()
                || self.week_count != 0
                || self.reasons != 0
                || self.first_issue_day.is_some()
            {
                return Err(DecodeError::Inconsistent);
            }
            return Ok(());
        }
        for (condition, reason) in [
            (s.missing_days > 0, Reason::MissingSession),
            (s.unmeasured_days > 0, Reason::UnmeasuredCalendar),
            (
                s.longest_losing_streak > self.policy.max_losing_day_streak(),
                Reason::LosingDayStreak,
            ),
        ] {
            if condition != (self.reasons & reason.mask() != 0) {
                return Err(DecodeError::Inconsistent);
            }
        }
        if self.outcome == Outcome::Refused {
            return Ok(());
        }
        let expected = u64::try_from(self.last_day - self.first_day + 1)
            .map_err(|_| DecodeError::Inconsistent)?;
        let ratio_failed = s.eligible_days != 0
            && u128::from(s.winning_days) * u128::from(self.policy.winning_day_denominator())
                < u128::from(s.eligible_days) * u128::from(self.policy.winning_day_numerator());
        if s.calendar_days != expected
            || (s.eligible_days == 0) != (self.reasons & Reason::NoEligibleSession.mask() != 0)
            || (s.complete_weeks == 0) != (self.reasons & Reason::NoCompleteWeek.mask() != 0)
            || ratio_failed != (self.reasons & Reason::WinningDayRatio.mask() != 0)
            || (s.failing_weeks > 0)
                != (self.reasons & (Reason::WeeklyWins.mask() | Reason::WeeklyLosses.mask()) != 0)
        {
            return Err(DecodeError::Inconsistent);
        }
        Ok(())
    }
}

/// Existing shared-calendar answer after the existing swept-session exclusions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eligibility {
    /// Expected and eligible for this intraday policy.
    Eligible,
    /// Confirmed calendar closure.
    Closed,
    /// Existing charter withholding, not missing data.
    Excluded,
    /// Calendar authority cannot answer.
    Unmeasured,
}

/// Classify an IST civil day without a new exchange calendar or data inference.
#[must_use]
pub fn eligibility_of(day: i64) -> Eligibility {
    if valid_day(day).is_err() {
        return Eligibility::Unmeasured;
    }
    let excluded = indicators::evaluator::CHARTER_NON_REGULAR_IST_DAYS.contains(&day);
    match kind_of(day) {
        DayKind::Open(_) | DayKind::OpenLengthUnmeasured if excluded => Eligibility::Excluded,
        DayKind::Open(_) => Eligibility::Eligible,
        DayKind::Closed => Eligibility::Closed,
        DayKind::OpenLengthUnmeasured | DayKind::Unmeasured => Eligibility::Unmeasured,
    }
}

/// Evaluate exact saved day sums. Missing rows are refused, never supplied as zeros.
/// Cash families return `NotApplicable` without inspecting a trading-day population.
/// Optional retained weeks affect memory/output only, not canonical result identity.
#[must_use]
pub fn evaluate(
    policy: Policy,
    family: ResearchFamilyV1,
    first_day: i64,
    last_day: i64,
    sessions: &[Session],
    retain_weeks: bool,
) -> Evaluation {
    let mut state = State::new(policy, family, first_day, last_day);
    for row in sessions {
        state.sessions.update(&row.canonical_bytes());
    }
    if family.legacy_index_family().is_none() {
        return state.finish();
    }
    if valid_day(first_day).is_err() || valid_day(last_day).is_err() || first_day > last_day {
        state.issue(Reason::InvalidSpan, None);
        return state.finish();
    }
    if let Err((reason, day)) = validate_sessions(first_day, last_day, sessions) {
        state.issue(reason, day);
        return state.finish();
    }
    let mut offered = sessions.iter().peekable();
    let mut week = Week::empty(monday(first_day));
    for day in first_day..=last_day {
        let row = if offered.peek().is_some_and(|row| row.day == day) {
            offered.next()
        } else {
            None
        };
        if let Err(reason) = state.day(day, row, &mut week) {
            state.issue(reason, Some(day));
            return state.finish();
        }
        if day == last_day || weekday(day) == 6 {
            if let Err(reason) = state.week(week.finish(), retain_weeks) {
                state.issue(reason, Some(day));
                return state.finish();
            }
            week = Week::empty(day + 1);
        }
    }
    if state.result.summary.eligible_days == 0 {
        state.issue(Reason::NoEligibleSession, None);
    } else if u128::from(state.result.summary.winning_days)
        * u128::from(policy.winning_day_denominator())
        < u128::from(state.result.summary.eligible_days)
            * u128::from(policy.winning_day_numerator())
    {
        state.issue(Reason::WinningDayRatio, None);
    }
    if state.result.summary.complete_weeks == 0 {
        state.issue(Reason::NoCompleteWeek, None);
    }
    state.finish()
}

struct State {
    result: Evaluation,
    streak: u64,
    weeks: Hasher,
    sessions: Hasher,
    calendar: Hasher,
}
impl State {
    fn new(policy: Policy, family: ResearchFamilyV1, first_day: i64, last_day: i64) -> Self {
        Self {
            result: Evaluation {
                policy,
                family,
                first_day,
                last_day,
                outcome: Outcome::Unmeasured,
                reasons: 0,
                first_issue_day: None,
                summary: Summary::default(),
                weeks: Vec::new(),
                week_count: 0,
                weeks_digest: [0; 32],
                sessions_digest: [0; 32],
                calendar_digest: [0; 32],
            },
            streak: 0,
            weeks: domain(WEEK_DOMAIN),
            sessions: domain(SESSION_DOMAIN),
            calendar: domain(CALENDAR_DOMAIN),
        }
    }
    fn issue(&mut self, reason: Reason, day: Option<i64>) {
        self.result.reasons |= reason.mask();
        if self.result.first_issue_day.is_none() {
            self.result.first_issue_day = day;
        }
    }
    fn day(&mut self, day: i64, row: Option<&Session>, week: &mut Week) -> Result<(), Reason> {
        let eligibility = eligibility_of(day);
        let weekday = weekday(day) < 5;
        // Check representability before mutating any measured day totals.
        let totals = row
            .filter(|_| eligibility == Eligibility::Eligible)
            .map(|row| {
                Ok((
                    self.result
                        .summary
                        .trades
                        .checked_add(row.trades)
                        .ok_or(Reason::ArithmeticOverflow)?,
                    self.result
                        .summary
                        .pessimistic_paisa
                        .checked_add(row.pessimistic_paisa)
                        .ok_or(Reason::ArithmeticOverflow)?,
                    if weekday {
                        week.trades
                            .checked_add(row.trades)
                            .ok_or(Reason::ArithmeticOverflow)?
                    } else {
                        week.trades
                    },
                    if weekday {
                        week.pessimistic_paisa
                            .checked_add(row.pessimistic_paisa)
                            .ok_or(Reason::ArithmeticOverflow)?
                    } else {
                        week.pessimistic_paisa
                    },
                ))
            })
            .transpose()?;
        self.calendar.update(&calendar_bytes(day));
        self.result.summary.calendar_days += 1;
        if weekday {
            week.weekday_days += 1;
        }
        match eligibility {
            Eligibility::Closed => {
                self.result.summary.closed_days += 1;
                if weekday {
                    week.closed_weekdays += 1;
                }
            }
            Eligibility::Excluded => {
                self.result.summary.excluded_days += 1;
                if weekday {
                    week.excluded_days += 1;
                }
            }
            Eligibility::Unmeasured => {
                self.result.summary.unmeasured_days += 1;
                if weekday {
                    week.unmeasured_days += 1;
                }
                self.issue(Reason::UnmeasuredCalendar, Some(day));
            }
            Eligibility::Eligible => {
                self.result.summary.eligible_days += 1;
                if weekday {
                    week.eligible_days += 1;
                } else {
                    self.result.summary.weekend_sessions += 1;
                    week.weekend_sessions += 1;
                }
                if let (Some(row), Some(totals)) = (row, totals) {
                    self.record(row, week, weekday, totals);
                } else {
                    self.result.summary.missing_days += 1;
                    if weekday {
                        week.missing_days += 1;
                    }
                    self.issue(Reason::MissingSession, Some(day));
                }
            }
        }
        Ok(())
    }
    fn record(
        &mut self,
        row: &Session,
        week: &mut Week,
        weekday: bool,
        totals: (u64, i64, u64, i64),
    ) {
        let summary = &mut self.result.summary;
        (
            summary.trades,
            summary.pessimistic_paisa,
            week.trades,
            week.pessimistic_paisa,
        ) = totals;
        summary.observed_days += 1;
        if weekday {
            week.observed_days += 1;
        }
        // WHICH DAYS ARE EVIDENCE, AND WHICH ONLY LOOK LIKE IT -- D-0596.
        //
        // V1 read the SIGN of the pessimistic sum, so a day netting one paisa
        // counted exactly as much as a day netting fifty thousand rupees, and a
        // day that lost a paisa under the worst reading while winning handsomely
        // under the best was filed as a loss. Every threshold in `Policy` counts
        // days, so that classification is the entire input to the rule.
        //
        // V2 asks whether the day's outcome survives BOTH readings. A win must
        // clear the day's own bracket -- `optimistic - pessimistic`, the spread
        // between the worst and best ordering of its own trades. A loss must
        // still be a loss under the OPTIMISTIC reading. A day that satisfies
        // neither is a scratch: its sign depends on which admissible ordering
        // you read, so it is evidence for no side and falls to the `Equal` arm,
        // where it is already counted as a zero day and already preserves the
        // streak exactly as a no-trade day does.
        //
        // The threshold is the day's own measurement uncertainty, not a number
        // anybody chose -- the same principle D-0595 applied to one trade. On a
        // V1 record the bracket is zero, so this reduces to the sign test and
        // every existing row means precisely what it always meant.
        let ordering = if self.result.policy.magnitude_aware() {
            let bracket = row.bracket_paisa();
            if row.pessimistic_paisa > bracket {
                std::cmp::Ordering::Greater
            } else if row.optimistic_paisa < 0 {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        } else {
            row.pessimistic_paisa.cmp(&0)
        };
        match ordering {
            std::cmp::Ordering::Greater => {
                summary.winning_days += 1;
                self.streak = 0;
                if weekday {
                    week.winning_days += 1;
                }
            }
            std::cmp::Ordering::Less => {
                summary.losing_days += 1;
                self.streak += 1;
                summary.longest_losing_streak = summary.longest_losing_streak.max(self.streak);
                if weekday {
                    week.losing_days += 1;
                }
                if self.streak > self.result.policy.max_losing_day_streak() {
                    self.issue(Reason::LosingDayStreak, Some(row.day));
                }
            }
            std::cmp::Ordering::Equal => {
                if row.trades == 0 {
                    summary.no_trade_days += 1;
                    if weekday {
                        week.no_trade_days += 1;
                    }
                } else {
                    summary.zero_days += 1;
                    if weekday {
                        week.zero_days += 1;
                    }
                }
            }
        }
    }
    fn week(&mut self, week: Week, retain: bool) -> Result<(), Reason> {
        if retain {
            self.result
                .weeks
                .try_reserve(1)
                .map_err(|_| Reason::AllocationRefused)?;
            self.result.weeks.push(week);
        }
        self.weeks.update(&week.canonical_bytes());
        self.result.week_count += 1;
        match week.kind {
            WeekKind::Complete => {
                self.result.summary.complete_weeks += 1;
                if week.outcome == Outcome::Passed {
                    self.result.summary.passing_weeks += 1;
                }
                if week.outcome == Outcome::Failed {
                    self.result.summary.failing_weeks += 1;
                    if week.winning_days < self.result.policy.min_weekly_wins() {
                        self.issue(Reason::WeeklyWins, Some(week.monday));
                    }
                    if week.losing_days > self.result.policy.max_weekly_losses() {
                        self.issue(Reason::WeeklyLosses, Some(week.monday));
                    }
                }
            }
            WeekKind::Short => self.result.summary.short_weeks += 1,
            WeekKind::Partial => self.result.summary.partial_weeks += 1,
            WeekKind::Unmeasured => self.result.summary.unmeasured_weeks += 1,
        }
        Ok(())
    }
    fn finish(mut self) -> Evaluation {
        self.result.outcome = outcome_for(self.result.family, self.result.reasons);
        self.result.weeks_digest = self.weeks.finalize();
        self.result.sessions_digest = self.sessions.finalize();
        self.result.calendar_digest = self.calendar.finalize();
        self.result
    }
}

fn validate_sessions(first: i64, last: i64, rows: &[Session]) -> Result<(), (Reason, Option<i64>)> {
    if rows.len() as u64
        > u64::try_from(last - first + 1).map_err(|_| (Reason::InvalidSpan, None))?
    {
        return Err((Reason::InvalidCountOrReturn, None));
    }
    let mut previous = None;
    for row in rows {
        if row.day < first || row.day > last || previous.is_some_and(|day| row.day <= day) {
            return Err((Reason::InvalidOrder, Some(row.day)));
        }
        if row.trades == 0 && row.pessimistic_paisa != 0 {
            return Err((Reason::InvalidCountOrReturn, Some(row.day)));
        }
        if matches!(
            eligibility_of(row.day),
            Eligibility::Closed | Eligibility::Excluded
        ) {
            return Err((Reason::UnexpectedSession, Some(row.day)));
        }
        previous = Some(row.day);
    }
    Ok(())
}
fn outcome_for(family: ResearchFamilyV1, reasons: u64) -> Outcome {
    if family.legacy_index_family().is_none() {
        Outcome::NotApplicable
    } else if reasons & REFUSALS != 0 {
        Outcome::Refused
    } else if reasons & UNMEASURED != 0 {
        Outcome::Unmeasured
    } else if reasons != 0 {
        Outcome::Failed
    } else {
        Outcome::Passed
    }
}
/// Ordered identity of the exact offered session records, including explicit zeros.
/// This hashes O(sessions) bytes with fixed accumulator state and performs no I/O.
#[must_use]
pub fn session_digest(sessions: &[Session]) -> [u8; 32] {
    let mut hash = domain(SESSION_DOMAIN);
    for row in sessions {
        hash.update(&row.canonical_bytes());
    }
    hash.finalize()
}
fn calendar_bytes(day: i64) -> [u8; 32] {
    let excluded = u64::from(indicators::evaluator::CHARTER_NON_REGULAR_IST_DAYS.contains(&day));
    let (tag, session) = match kind_of(day) {
        DayKind::Open(session) => (1_u64, Some(session)),
        DayKind::Closed => (2, None),
        DayKind::OpenLengthUnmeasured => (3, None),
        DayKind::Unmeasured => (4, None),
    };
    let windows = session.map_or(0, |session| {
        session
            .windows
            .iter()
            .enumerate()
            .fold(0_u64, |word, (index, window)| {
                let packed = u64::from(window.from) | (u64::from(window.to) << 16);
                word | (packed << (index * 32))
            })
    });
    parts(&[
        &day.to_le_bytes(),
        &(tag | (excluded << 8) | (u64::from(session.map_or(0, |s| s.count)) << 16)).to_le_bytes(),
        &windows.to_le_bytes(),
        &u64::from(crate::stored::SWEPT_SERIES_CALENDAR_POLICY).to_le_bytes(),
    ])
}
fn weekday(day: i64) -> i64 {
    (day + 3).rem_euclid(7)
}
fn monday(day: i64) -> i64 {
    day - weekday(day)
}
fn valid_day(day: i64) -> Result<(), DecodeError> {
    Day::from_days(u32::try_from(day).map_err(|_| DecodeError::Format)?)
        .map(|_| ())
        .map_err(|_| DecodeError::Format)
}
fn bits(value: i64) -> u64 {
    u64::from_le_bytes(value.to_le_bytes())
}
fn signed(value: u64) -> i64 {
    i64::from_le_bytes(value.to_le_bytes())
}
fn domain(name: &[u8]) -> Hasher {
    let mut hash = Hasher::new();
    hash.update(name);
    hash
}
fn sum(values: &[u64]) -> Result<u64, DecodeError> {
    values.iter().try_fold(0_u64, |total, value| {
        total.checked_add(*value).ok_or(DecodeError::Inconsistent)
    })
}
fn words<const N: usize>(magic: [u8; 8], values: &[u64]) -> [u8; N] {
    let mut raw = [0; N];
    let mut chunks = raw.chunks_exact_mut(8);
    if let Some(first) = chunks.next() {
        first.copy_from_slice(&magic);
    }
    for (chunk, value) in chunks.zip(values) {
        chunk.copy_from_slice(&value.to_le_bytes());
    }
    raw
}
fn parts<const N: usize>(parts: &[&[u8]]) -> [u8; N] {
    let mut raw = [0; N];
    for (target, byte) in raw
        .iter_mut()
        .zip(parts.iter().flat_map(|part| part.iter()))
    {
        *target = *byte;
    }
    raw
}
fn read_words<const N: usize>(raw: &[u8], magic: [u8; 8]) -> Result<[u64; N], DecodeError> {
    if raw.len() != (N + 1) * 8 || raw.get(..8) != Some(magic.as_slice()) {
        return Err(DecodeError::Format);
    }
    let mut result = [0; N];
    for (word, bytes) in result.iter_mut().zip(raw.chunks_exact(8).skip(1)) {
        *word = u64::from_le_bytes(bytes.try_into().map_err(|_| DecodeError::Format)?);
    }
    Ok(result)
}
struct Read<'a> {
    raw: &'a [u8],
    at: usize,
}
impl Read<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let end = self.at.checked_add(N).ok_or(DecodeError::Format)?;
        let part = self.raw.get(self.at..end).ok_or(DecodeError::Format)?;
        self.at = end;
        part.try_into().map_err(|_| DecodeError::Format)
    }
}

/// A detached observation cannot be decoded as a valid policy/evidence shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// Unknown magic, width, civil date or rule code.
    Format,
    /// Family is not a canonical existing research member.
    Scope,
    /// Contradictory counters, outcome or authenticated rows.
    Inconsistent,
    /// Required observation storage could not be allocated.
    Allocation,
}
impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Format => "index consistency format differs",
            Self::Scope => "index consistency family differs",
            Self::Inconsistent => "index consistency evidence is inconsistent",
            Self::Allocation => "index consistency observation allocation refused",
        })
    }
}
impl std::error::Error for DecodeError {}

#[cfg(test)]
#[path = "index_consistency_tests.rs"]
mod tests;
