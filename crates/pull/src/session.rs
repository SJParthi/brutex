//! The trading session, as one set of numbers, and the calendar arithmetic a
//! vendor window needs.
//!
//! # The session, from the exchange
//!
//! `docs/00-charter.md` §3: the regular session is **09:15 inclusive to 15:30
//! exclusive IST**, the last one-minute bar opens at **15:29**, and that is
//! exactly **375** bars. Those three statements are one statement, and
//! [`SESSION_OPEN_MINUTE`], [`SESSION_CLOSE_MINUTE`] and
//! [`BARS_PER_REGULAR_SESSION`] are pinned to each other by a compile-time
//! assertion so they cannot drift apart.
//!
//! **An operator asserted 15:40. The exchange says 15:30.** The source is
//! `docs/00-charter.md` §3, which records the close, the 15:29 last bar and the
//! 375-bar count together, and records the weekend budget sessions as 375 bars
//! each *confirmed in the lake*. It is cited that way and no other way:
//! `CLAUDE.md` §3 rule 1 wants a claim about an exchange traceable to the
//! charter, and an earlier draft of this header instead claimed that NSE's
//! `marketStatus` endpoint had been read to confirm it — a live read recorded
//! in no document, which nobody reading this file can check. Ten phantom
//! minutes a day is not a rounding difference: every VWAP, every session-close
//! figure and every forced-exit bar would be computed over a window the
//! exchange never traded. The number is written **once**, here, and every
//! filter reads it from this module — the reason a constant repeated in three
//! filters is a constant that will one day disagree with itself.
//!
//! # A daily bar is exempt, and that is not a special case
//!
//! A daily bar has no intraday time. Vendors stamp it at midnight, or at the
//! open, or at the close, and none of those is inside 09:15–15:30 by
//! coincidence. Applying an intraday window to it would drop every daily bar
//! ever fetched, so [`Cadence`] is an argument to the filter rather than an
//! assumption inside it, and `pull::unit::a_daily_bar_is_exempt_from_the_
//! session_filter` is what holds it up.
//!
//! # What this module does **not** check, and says so
//!
//! **Minute alignment.** [`IstMoment::second_of_minute`] is computed and
//! exposed, and [`Window::verdict`] does not look at it: a bar stamped
//! 09:15:30 is inside the 09:15 minute and is kept. Whether a vendor stamping
//! a one-minute bar off the minute boundary is a fault is a *decoder*
//! question, not a window question, and inventing a [`DropReason`] for it here
//! would put a data-quality judgement inside a clock. `pull::unit::a_bar_that_
//! is_not_minute_aligned_is_kept_and_its_offset_is_visible` pins the current
//! behaviour so that changing it has to be deliberate.
//!
//! # The trading calendar this module reads, and what it still does not know
//!
//! **This module computes no day of the week and holds no holiday list of its
//! own.** There is no `% 7` in it. A weekend rule would be *wrong*: 2025-02-01
//! was a Saturday and a full 375-bar session, as were 2020-02-01 (Sat) and
//! 2026-02-01 (Sun).
//!
//! It reads [`crate::calendar::kind_of`], the measured NSE calendar whose
//! sources that module states, and nothing else. A bar on a day the calendar
//! records `Closed` is dropped and counted as [`DropReason::OnClosedDay`];
//! irregular sessions keep exactly their own windows; a day the calendar has
//! not measured is kept and counted by name
//! ([`DropCensus::count_unclassified_kept`]) rather than guessed either way.
//! Invariant `P-03` — "a bar on a non-trading date is dropped and counted" —
//! is proved by `pull::unit::calendar_filter` (D-2673). This header said the
//! opposite until then, when no calendar existed.
//!
//! What is filtered here is the **window**: the operator's date range, the
//! closed days, and the intraday session bounds.
//!
//! # Cost
//!
//! Every function here is arithmetic on two integers. There is no table, no
//! search and no allocation: [`Day::days_from_epoch`] and [`Day::from_days`]
//! are the standard closed-form civil-calendar pair (published as
//! `days_from_civil` and `civil_from_days`), so a timestamp in 1970 and one in
//! 9999 cost the same — `docs/07-o1-architecture.md` law 4, arithmetic beats
//! lookup. `pull::unit::the_calendar_round_trips_every_day_this_build_can_name`
//! drives every one of the 2,932,897 day counts through both directions and
//! through `succ`, so the pair is exhaustively checked rather than spot-checked.

use std::fmt;

use store::path::{PathError, YearMonth};

/// Seconds IST is ahead of UTC: 5 hours 30 minutes, fixed.
///
/// `docs/00-charter.md` §3: *"IST, fixed +05:30, no daylight saving."* India
/// has observed no daylight saving since 1945, so this is a constant rather
/// than a lookup — which is what lets a timestamp be converted with an add
/// instead of a timezone database.
pub const IST_OFFSET_SECS: i64 = 5 * 3_600 + 30 * 60;

/// Seconds in a day.
pub const SECS_PER_DAY: i64 = 86_400;

/// Seconds in a minute.
pub const SECS_PER_MINUTE: i64 = 60;

/// The first minute of the regular session, **inclusive**: 09:15 IST.
///
/// Minutes since IST midnight, so 9·60 + 15.
pub const SESSION_OPEN_MINUTE: u32 = 9 * 60 + 15;

/// The end of the regular session, **exclusive**: 15:30 IST.
///
/// Exclusive is the whole point. A bar is stamped at the **open** of its
/// interval (`docs/00-charter.md` §3), so the last one-minute bar of a session
/// opens at 15:29 and closes exactly at 15:30. A filter written `<= 15:30`
/// admits a 15:30 bar that the exchange never traded.
pub const SESSION_CLOSE_MINUTE: u32 = 15 * 60 + 30;

/// One-minute bars in one regular session.
///
/// `docs/00-charter.md` §3: 375. It is *derived* from the two bounds above by
/// the assertion below rather than merely written beside them, because the
/// arithmetic is the proof that the two bounds are the right pair: 09:15
/// through 15:29 inclusive is 375 minutes, and any other close makes this
/// number wrong.
pub const BARS_PER_REGULAR_SESSION: u32 = 375;

const _: () = assert!(SESSION_CLOSE_MINUTE - SESSION_OPEN_MINUTE == BARS_PER_REGULAR_SESSION);
const _: () = assert!(SESSION_OPEN_MINUTE == 555 && SESSION_CLOSE_MINUTE == 930);
const _: () = assert!(IST_OFFSET_SECS == 19_800);
// The store keeps its own copy to compute a month's IST span at its write
// boundary (`store::path::YearMonth::ist_bounds_micros`, D-0915) because it
// cannot depend on this crate. This is what keeps the copy honest.
const _: () = assert!(IST_OFFSET_SECS == store::path::IST_OFFSET_SECS);
// The store's copy of the open, which `pull::fold` anchors every intraday rung
// on, is this one (D-3518).
const _: () =
    assert!(SESSION_OPEN_MINUTE == store::path::Timeframe::OPEN_MINUTES_PAST_IST_MIDNIGHT);
const _: () = assert!(SECS_PER_DAY == 86_400);
// `IstMoment::from_epoch_secs` narrows this one to `u32`. The cast is exact
// only while it is 60, and this is what says so at compile time.
const _: () = assert!(SECS_PER_MINUTE == 60);
const _: () = assert!(SECS_PER_DAY % SECS_PER_MINUTE == 0);

/// The first year a bar can carry. Timestamps are seconds since the epoch.
pub const MIN_YEAR: u32 = 1_970;

/// The last year this build will name. Above it a four-digit rendering lies,
/// and `store::path::YearMonth` refuses it too.
pub const MAX_YEAR: u32 = 9_999;

/// Why a timestamp or a date is not one.
///
/// Every variant carries the value it refused. A refusal that says only
/// "invalid date" sends an operator to a hex dump of an epoch second.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SessionError {
    /// The timestamp names an instant before 1970-01-01 00:00 IST.
    ///
    /// **The bound is on the IST instant, not on the sign of the input**, and
    /// the two differ by exactly [`IST_OFFSET_SECS`]: `-19_800` is 1970-01-01
    /// 00:00:00 IST and is accepted, `-19_801` is the second before it and is
    /// refused. A negative epoch second is therefore *not* by itself a refusal
    /// — `-1` is 1970-01-01 05:29:59 IST, a date a [`Day`] can hold. Anything
    /// this variant does carry is genuinely before the Unix epoch as well, so
    /// the message it prints is true whenever it is printed.
    ///
    /// Refused rather than clamped: a clamped timestamp is a bar filed under
    /// 1970-01-01, which is a fault that looks like data.
    /// `pull::unit::the_before_epoch_boundary_is_the_ist_instant_not_the_sign`
    /// is what holds the boundary up on both sides.
    BeforeEpoch {
        /// The value that was refused.
        secs: i64,
    },
    /// The venue's session table states no verified hours for this day.
    ///
    /// A REFUSAL and never a fallback. The alternative — quietly applying the
    /// anchor's hours — is the §4 row about hiding a failure: it would admit or
    /// drop bars against a session nobody has confirmed, and say nothing.
    VenueHoursUnknown {
        /// Which venue, in its own words.
        venue: &'static str,
        /// The day, as days from the epoch.
        days: u32,
    },
    /// A minute bar falls on a day the exchange calendar records as traded
    /// but whose session length was never measured
    /// ([`crate::calendar::DayKind::OpenLengthUnmeasured`], the five Muhurat
    /// sessions before 2025).
    ///
    /// A REFUSAL, named, rather than a drop against the regular hours or a keep
    /// against them: either would be a claim about which minutes that session
    /// owed, and nothing here measured it. The remedy is to measure the session
    /// and record it in the calendar's irregular table. CE-52, D-2670.
    SessionLengthUnmeasured {
        /// The day, as days from the epoch.
        days: u32,
    },
    /// The timestamp does not name a date this build can render.
    ///
    /// Reached three ways, all of which are the same fault to an operator —
    /// the column is not epoch seconds — so they are one variant: a value whose
    /// IST day count leaves `u32`, one whose day count overflows the calendar's
    /// epoch shift, and one whose year leaves `1970..=9999`.
    TimestampOutOfRange {
        /// The value that was refused.
        secs: i64,
    },
    /// A day count [`Day::from_days`] cannot name, because adding the civil
    /// calendar's epoch shift to it would leave `u32`.
    ///
    /// Its own variant rather than a [`SessionError::YearOutOfRange`] carrying
    /// a computed year, because there is no year to compute: the addition that
    /// would produce one is the addition that overflows. Before this refusal
    /// existed the add wrapped, and a wrapped day count is the worst shape a
    /// fault can take — `Day::from_days(u32::MAX)` **panicked in a debug build
    /// and returned `YearOutOfRange { year: 1969 }` in a release build**, which
    /// is a build-dependent answer to a pure function. `CLAUDE.md` §3 rule 5.
    DayCountOutOfRange {
        /// The day count that was refused.
        days: u32,
    },
    /// The year is outside `1970..=9999`.
    YearOutOfRange {
        /// The year that was refused.
        year: u32,
    },
    /// The month is outside `1..=12`.
    MonthOutOfRange {
        /// The month that was refused.
        month: u8,
    },
    /// The day is outside `1..=` the length of that month, in that year.
    ///
    /// Leap years are computed, not tabulated: 2024-02-29 exists and
    /// 2023-02-29 does not, and a table would be a second definition of the
    /// Gregorian rule.
    DayOutOfRange {
        /// The day that was refused.
        day: u8,
        /// How many days that month actually has.
        month_len: u8,
    },
    /// The day after this one is past [`MAX_YEAR`].
    ///
    /// Refused rather than wrapped, because the one caller is the vendor's
    /// non-inclusive `toDate` rule: a wrapped day would silently ask for a
    /// window ending in 1970.
    NoNextDay,
    /// [`split_window`] was given a cap of `Some(0)` days.
    ///
    /// Refused rather than silently read as "no cap". `None` already says "no
    /// cap" and says it deliberately; a zero is a number that went wrong, and
    /// reading the two the same way would hand a whole window to a vendor that
    /// published a limit. A loop that advances by zero would not terminate
    /// anyway.
    WindowCapIsZero,
    /// A window whose end is before its start.
    ///
    /// Refused rather than silently swapped. An operator who typed the dates in
    /// the wrong order gets told; a pull that quietly reversed them would fetch
    /// a range nobody asked for and report success.
    WindowRunsBackwards {
        /// The start.
        from: Day,
        /// The end, which does not follow it.
        to: Day,
    },
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::VenueHoursUnknown { venue, days } => write!(
                f,
                "{venue} states no verified trading hours for day {days}, so \
                 whether a bar at that time is inside the session is unknown. \
                 Refused rather than assumed: applying another day's hours \
                 would admit or drop bars against a session nobody confirmed"
            ),
            Self::SessionLengthUnmeasured { days } => write!(
                f,
                "day {days} traded a session whose length was never measured \
                 (a Muhurat with no minute series), so whether a minute bar on \
                 it was inside that session is unknown. Refused rather than \
                 judged against the regular hours; measure the session and \
                 record it in the calendar's irregular table"
            ),
            Self::BeforeEpoch { secs } => {
                write!(f, "timestamp {secs} is before the Unix epoch")
            }
            Self::TimestampOutOfRange { secs } => {
                write!(f, "timestamp {secs} names no date this build can render")
            }
            Self::DayCountOutOfRange { days } => {
                write!(f, "day count {days} names no date this build can render")
            }
            Self::YearOutOfRange { year } => {
                write!(f, "year {year} is not {MIN_YEAR}..={MAX_YEAR}")
            }
            Self::MonthOutOfRange { month } => write!(f, "month {month} is not 1..=12"),
            Self::DayOutOfRange { day, month_len } => {
                write!(f, "day {day} is not 1..={month_len} in that month")
            }
            Self::NoNextDay => write!(f, "there is no day after {MAX_YEAR}-12-31"),
            Self::WindowCapIsZero => write!(
                f,
                "a window cap of zero days cannot be split into — a vendor that \
                 publishes a limit publishes a positive one, and reading zero \
                 as \"no limit\" would send the whole window"
            ),
            Self::WindowRunsBackwards { from, to } => {
                write!(f, "window {from}..={to} runs backwards")
            }
        }
    }
}

impl std::error::Error for SessionError {}

/// Whether a year is a leap year, by the Gregorian rule.
const fn is_leap(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

/// How many days a month holds, in that year.
///
/// A `match` rather than a table, so February's dependence on the year is in
/// the same expression as the answer and cannot be looked up without it.
const fn month_len(year: u32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        // February, and every value outside `1..=12`. `Day::new` has already
        // refused those, so the arm is February's; folding them together avoids
        // an arm that no input can reach.
        _ => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
    }
}

/// One calendar date, in IST.
///
/// Fixed width, `Copy`, and validated on construction — `docs/07-o1-
/// architecture.md` layer 1. There is no constructor taking text: the vendor's
/// wire format is produced by [`std::fmt::Display`] and never parsed back, so a
/// date that was never validated is unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[expect(
    clippy::struct_field_names,
    reason = "the lint fires because `day` repeats the type name `Day`. On a \
              calendar date that repetition is the correct naming: the field \
              *is* the day of the month, and `day_of_month` or `dom` would be \
              a longer or an obscurer spelling of the same thing. The three \
              fields are the three parts of an ISO date and are named after \
              them."
)]
pub struct Day {
    // Field order is the comparison order, and that is load-bearing: the
    // derived `Ord` compares year, then month, then day, which is calendar
    // order. A window comparison is therefore one derived comparison rather
    // than a hand-written one that could get the precedence wrong.
    year: u16,
    month: u8,
    day: u8,
}

impl Day {
    /// Builds a date, refusing one that does not exist.
    ///
    /// # Errors
    ///
    /// [`SessionError::YearOutOfRange`], [`SessionError::MonthOutOfRange`], or
    /// [`SessionError::DayOutOfRange`] naming how long that month actually is.
    ///
    /// # Examples
    ///
    /// ```
    /// # use pull::session::{Day, SessionError};
    /// assert_eq!(Day::new(2024, 2, 29)?.to_string(), "2024-02-29");
    /// assert!(Day::new(2023, 2, 29).is_err(), "2023 is not a leap year");
    /// # Ok::<(), SessionError>(())
    /// ```
    pub const fn new(year: u16, month: u8, day: u8) -> Result<Self, SessionError> {
        // `year as u32` rather than `u32::from(year)`: `From` is not const-stable
        // on this toolchain, and this constructor must be callable from the
        // `const fn` calendar below. The cast is a widening one and cannot lose a
        // bit. `succ` widens the same way for the same reason.
        let wide = year as u32;
        if wide < MIN_YEAR || wide > MAX_YEAR {
            return Err(SessionError::YearOutOfRange { year: wide });
        }
        if month == 0 || month > 12 {
            return Err(SessionError::MonthOutOfRange { month });
        }
        let len = month_len(wide, month);
        if day == 0 || day > len {
            return Err(SessionError::DayOutOfRange {
                day,
                month_len: len,
            });
        }
        Ok(Self { year, month, day })
    }

    /// The year.
    #[must_use]
    pub const fn year(self) -> u16 {
        self.year
    }

    /// The month, `1..=12`.
    #[must_use]
    pub const fn month(self) -> u8 {
        self.month
    }

    /// The day of the month.
    #[must_use]
    pub const fn day(self) -> u8 {
        self.day
    }

    /// The month this date falls in, as the store names months.
    ///
    /// Returns `store::path::YearMonth` rather than a pair, because that is the
    /// type a bar file's path and a manifest key are built from — a second
    /// spelling of "which month" would let a pull write into a month the store
    /// cannot address.
    ///
    /// # Errors
    ///
    /// Whatever `YearMonth::new` refuses. Unreachable through a validated
    /// [`Day`], whose bounds are the tighter of the two, and returned rather
    /// than swallowed because `crates/store` is not this crate's to watch:
    /// if that bound ever narrows, this is a refusal rather than a panic.
    pub const fn year_month(self) -> Result<YearMonth, PathError> {
        YearMonth::new(self.year, self.month)
    }

    /// The last day of the month this date falls in.
    ///
    /// # Why this exists on `Day` and not in the caller
    ///
    /// The store addresses ONE MONTH PER FILE. A fetch window that crosses a
    /// month boundary produces a batch `fetch::land` refuses by name — and
    /// the refusal is correct, because splitting is a decision about which
    /// FILE the bars belong in, which the store cannot make for a caller that
    /// did not make it.
    ///
    /// Measured before this existed: a 37-day, 774-instrument pull read
    /// 6,907,286 rows, stored 5,496,790, and failed 699 members — every one
    /// of them an instrument whose window crossed July into August. The
    /// vendor answered and the bars decoded; they died at the write boundary
    /// for a reason the caller could have prevented for free.
    ///
    /// Leap years come from `month_len`, the same function `Day::new`
    /// validates against, so a February end-of-month can never disagree with
    /// the day it was built from.
    #[must_use]
    pub const fn end_of_month(self) -> Self {
        Self {
            year: self.year,
            month: self.month,
            day: month_len(self.year as u32, self.month),
        }
    }

    /// The next calendar day.
    ///
    /// **This is what makes the vendor's non-inclusive `toDate` invisible to an
    /// operator.** See [`Window::wire_to`].
    ///
    /// # Errors
    ///
    /// [`SessionError::NoNextDay`] after 9999-12-31.
    ///
    /// # Examples
    ///
    /// ```
    /// # use pull::session::{Day, SessionError};
    /// assert_eq!(Day::new(2024, 2, 28)?.succ()?, Day::new(2024, 2, 29)?);
    /// assert_eq!(Day::new(2024, 12, 31)?.succ()?, Day::new(2025, 1, 1)?);
    /// assert!(Day::new(9999, 12, 31)?.succ().is_err());
    /// # Ok::<(), SessionError>(())
    /// ```
    pub const fn succ(self) -> Result<Self, SessionError> {
        let wide = self.year as u32;
        if self.day < month_len(wide, self.month) {
            return Ok(Self {
                day: self.day + 1,
                ..self
            });
        }
        if self.month < 12 {
            return Ok(Self {
                month: self.month + 1,
                day: 1,
                ..self
            });
        }
        if wide >= MAX_YEAR {
            return Err(SessionError::NoNextDay);
        }
        Ok(Self {
            year: self.year + 1,
            month: 1,
            day: 1,
        })
    }

    /// This date, `months` whole calendar months earlier.
    ///
    /// # Why months and not days
    ///
    /// Because a vendor states months. Groww's own interval table gives its
    /// one-minute history as *"Last 3 months"*, and three months is 89, 90, 91
    /// or 92 days depending on where in the year you stand — writing it as a
    /// day count would be a number this repository invented rather than one it
    /// read. A floor longer than a couple of decades is stated in years
    /// instead and is carried by [`crate::vendor::HistoryFloor::Rolling`],
    /// which is why `u8` is wide enough here.
    ///
    /// # The day of the month is clamped, never rolled over
    ///
    /// The 31st has no counterpart in a 30-day month and 29 February has none
    /// in a common year. The day is taken **down** to the last day that month
    /// actually has, using the same `month_len` [`Day::new`] validates
    /// against, so a clamped date can never be one this build would refuse.
    /// Rolling forward instead — 31 May less three months as 3 March — would
    /// move a history floor *later* than the vendor's, which silently refuses
    /// days the vendor would have answered.
    ///
    /// There is no loop here and nothing is looked up: two divisions, a
    /// comparison and two subtractions, whatever `months` is. That is a
    /// statement about the code below rather than a measurement, and it is not
    /// written as a bound because nothing has timed it.
    ///
    /// # Errors
    ///
    /// [`SessionError::YearOutOfRange`] when the walk lands before
    /// [`MIN_YEAR`], which is the same refusal [`Day::new`] gives for a date
    /// this build cannot name.
    ///
    /// # Examples
    ///
    /// ```
    /// # use pull::session::{Day, SessionError};
    /// assert_eq!(Day::new(2026, 8, 12)?.months_before(3)?, Day::new(2026, 5, 12)?);
    /// // January less one month is the previous December.
    /// assert_eq!(Day::new(2026, 1, 31)?.months_before(1)?, Day::new(2025, 12, 31)?);
    /// // 31 May less three months is the LAST day February has, not 3 March.
    /// assert_eq!(Day::new(2026, 5, 31)?.months_before(3)?, Day::new(2026, 2, 28)?);
    /// # Ok::<(), SessionError>(())
    /// ```
    pub const fn months_before(self, months: u8) -> Result<Self, SessionError> {
        // The remainder crosses January when it is at least this month's own
        // index, and that is the only case a year is borrowed. `+ 12` keeps
        // the subtraction below inside `u8` — every intermediate here is in
        // its own narrow type, because a `u16 as u8` is the truncating cast
        // this workspace denies outright.
        let rest = months % 12;
        let (month, borrow) = if self.month <= rest {
            (self.month + 12 - rest, 1)
        } else {
            (self.month - rest, 0)
        };
        // SATURATING, AND THE REFUSAL IS `Day::new`'S. A year that saturates to
        // zero is not a year this build can name, so `new` refuses it by name
        // with the bound in the message — one refusal site rather than two
        // that could disagree. `as u16` widens; it cannot truncate.
        let year = self.year.saturating_sub((months / 12) as u16 + borrow);
        let len = month_len(year as u32, month);
        let day = if self.day > len { len } else { self.day };
        Self::new(year, month, day)
    }

    /// Days since 1970-01-01, by the standard civil-calendar formula.
    ///
    /// Constant time and allocation-free. The era arithmetic is the published
    /// closed form; every intermediate is non-negative for the `1970..=9999`
    /// range a [`Day`] is validated into, which is why it is written in `u32`
    /// rather than in a signed type with shifts.
    #[must_use]
    pub const fn days_from_epoch(self) -> u32 {
        let y = self.year as u32;
        let m = self.month as u32;
        let d = self.day as u32;
        // March-based year: February's leap day becomes the last day of it, so
        // the month-length pattern is regular and needs no table.
        let y = if m <= 2 { y - 1 } else { y };
        let era = y / 400;
        let yoe = y - era * 400;
        let mp = if m > 2 { m - 3 } else { m + 9 };
        let doy = (153 * mp + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// The date `days` days after 1970-01-01.
    ///
    /// The inverse of [`Day::days_from_epoch`], and
    /// `pull::unit::the_calendar_round_trips_every_day_this_build_can_name`
    /// walks all 2,932,897 of them through both.
    ///
    /// # Errors
    ///
    /// [`SessionError::YearOutOfRange`] past 9999-12-31, and
    /// [`SessionError::DayCountOutOfRange`] for a count so large that the
    /// epoch shift below cannot even be applied to it.
    pub const fn from_days(days: u32) -> Result<Self, SessionError> {
        // CHECKED, NOT PLAIN. `days` is a `u32` and every one of them is a
        // legal argument, so `days + 719_468` overflows for the top 719,468 of
        // them — which used to panic in a debug build and wrap in a release
        // build, giving a pure function two different answers depending on how
        // it was compiled. See [`SessionError::DayCountOutOfRange`].
        let Some(z) = days.checked_add(719_468) else {
            return Err(SessionError::DayCountOutOfRange { days });
        };
        let era = z / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if m <= 2 { y + 1 } else { y };
        if y > MAX_YEAR {
            return Err(SessionError::YearOutOfRange { year: y });
        }
        // Every one of the three is inside its type by construction — the year
        // was just bounded, `mp` is at most 11 so `m` is `1..=12`, and `d` is
        // `1..=31`. The conversion is written as one refusal rather than three
        // because it has one cause: a day count this build cannot name. The
        // `y > MAX_YEAR` gate above is what makes it unreachable from here, and
        // the gate itself is reached by `from_days(4_000_000)`.
        // `TryFrom` is not const-stable on this toolchain and this is a
        // `const fn`, so the conversion is a cast guarded by the bound above
        // rather than by a `Result`. `Self::new` re-checks all three, so a cast
        // that was somehow wrong is refused there rather than believed.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "y was just refused above if it exceeds MAX_YEAR = 9,999, \
                      which fits u16; m is 1..=12 and d is 1..=31 by \
                      construction of the civil-date algorithm, both fitting \
                      u8. Self::new re-checks every one."
        )]
        let (year, month, day) = (y as u16, m as u8, d as u8);
        Self::new(year, month, day)
    }
}

impl fmt::Display for Day {
    /// `YYYY-MM-DD` — the shape both vendors take on the wire.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// The largest day count a [`Day`] can carry: 9999-12-31, the last date this
/// build will name.
///
/// **Derived, not written down.** The assertion below computes it from the
/// calendar itself at compile time, so the number and the two bounds it comes
/// from cannot drift apart — the same argument
/// [`BARS_PER_REGULAR_SESSION`] is pinned by. The day *count* is one less than
/// the number of nameable days, because 1970-01-01 is day zero: 2,932,896 here
/// and 2,932,897 dates in `0..=MAX_DAY_NUMBER`.
pub const MAX_DAY_NUMBER: u32 = 2_932_896;

const _: () = assert!(match Day::new(9999, 12, 31) {
    Ok(last) => last.days_from_epoch() == MAX_DAY_NUMBER,
    Err(_) => false,
});
const _: () = assert!(match Day::new(1970, 1, 1) {
    Ok(first) => first.days_from_epoch() == 0,
    Err(_) => false,
});

/// The largest day count `u32` can hold at all, as an `i64`.
///
/// Spelled as a literal because `i64::from` is not const-stable on this
/// toolchain and this is used from a `const fn`.
/// `pull::unit::the_day_count_ceiling_is_exactly_u32_max` is what proves the
/// literal is `u32::MAX` rather than a typo, since a compile-time assertion
/// would need the very cast the literal exists to avoid.
const U32_DAYS_MAX: i64 = 4_294_967_295;

/// One instant, as IST sees it: which day, and how far into it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IstMoment {
    day: Day,
    minute_of_day: u32,
    second_of_minute: u32,
}

impl IstMoment {
    /// The IST moment of an epoch second.
    ///
    /// The vendor sends UTC epoch seconds; the exchange trades in IST. This is
    /// the one place the two are reconciled, and it is an add.
    ///
    /// # Errors
    ///
    /// [`SessionError::BeforeEpoch`] for a value below `-19_800` — that is,
    /// one whose *IST* instant precedes 1970-01-01 00:00 IST, which is not the
    /// same set as "negative"; see that variant. Or
    /// [`SessionError::TimestampOutOfRange`] for one above `253_402_280_999`,
    /// whose IST day is past 9999-12-31 or whose day count does not fit `u32`
    /// at all. Both are what a vendor column read at the wrong offset looks
    /// like.
    ///
    /// # Examples
    ///
    /// ```
    /// # use pull::session::{IstMoment, SessionError};
    /// // 1326220200 is 18:30:00 UTC, which is 00:00:00 the next day in IST.
    /// // That is what a *daily* bar's timestamp looks like, and it is the
    /// // reason `Cadence::Daily` is exempt from the intraday window: midnight
    /// // is not inside 09:15..15:30, so an unconditional filter would drop
    /// // every daily bar ever fetched.
    /// let at = IstMoment::from_epoch_secs(1_326_220_200)?;
    /// assert_eq!(at.day().to_string(), "2012-01-11");
    /// assert_eq!(at.minute_of_day(), 0, "IST midnight — a daily stamp");
    ///
    /// // An intraday opening bar, for contrast: the same day at 09:15 IST is
    /// // 03:45 UTC, which is 33,300 seconds later.
    /// let open = IstMoment::from_epoch_secs(1_326_220_200 + 9 * 3_600 + 15 * 60)?;
    /// assert_eq!(open.minute_of_day(), 9 * 60 + 15);
    /// # Ok::<(), SessionError>(())
    /// ```
    pub const fn from_epoch_secs(epoch_secs: i64) -> Result<Self, SessionError> {
        let Some(local) = epoch_secs.checked_add(IST_OFFSET_SECS) else {
            return Err(SessionError::TimestampOutOfRange { secs: epoch_secs });
        };
        if local < 0 {
            return Err(SessionError::BeforeEpoch { secs: epoch_secs });
        }
        let day_number = local / SECS_PER_DAY;
        let into_day = local % SECS_PER_DAY;
        // THE CAST BELOW IS ONLY SAFE BECAUSE OF THIS GUARD, AND IT WAS NOT
        // HERE. `local` is bounded only by `i64`, so `day_number` reaches
        // 106,751,991,167,300 and `as u32` silently kept its low 32 bits:
        // epoch second 371,086,500,594,600 is day 4,294,982,646, which is
        // 11761233-01-30, and it came back as `Ok(2012-01-11 00:00:00)`. Not a
        // refusal, not a panic, a *plausible date* — the one outcome
        // `CLAUDE.md` §4 forbids outright. The bound is `u32::MAX` rather than
        // [`MAX_DAY_NUMBER`] so that the calendar keeps its own refusal below
        // rather than having it pre-empted here into an arm no input reaches.
        //
        // THE ONE EQUIVALENT MUTANT IN THIS MODULE IS ON THIS LINE, and it is
        // recorded rather than papered over. `cargo mutants` replaces `>` with
        // `>=`; the whole suite still passes, and it always will, because the
        // single day count that distinguishes them — exactly `u32::MAX` — is
        // refused either way and with the identical error: by this guard under
        // `>=`, and by `Day::from_days`'s own `DayCountOutOfRange` (remapped
        // just below) under `>`. It is not a missing test. Every constant this
        // guard could take lies in `MAX_DAY_NUMBER + 1 ..= u32::MAX`, and both
        // sides of every one of them refuse, so no choice of bound removes it;
        // the only structure that would is guarding at `MAX_DAY_NUMBER`, which
        // makes the refusal below unreachable and costs the coverage gate a
        // line instead. `pull::unit::the_guard_and_the_calendar_refuse_the_
        // same_boundary_identically` asserts the equivalence as a fact rather
        // than leaving it as this comment's assertion.
        if day_number > U32_DAYS_MAX {
            return Err(SessionError::TimestampOutOfRange { secs: epoch_secs });
        }
        // Same const-fn constraint as `Day::from_days`.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "day_number was just compared against U32_DAYS_MAX and \
                      refused above it, so the cast is exact; into_day is a \
                      remainder mod 86,400. `the_day_count_that_does_not_fit_\
                      u32_is_refused_not_truncated` is the test that holds the \
                      guard up."
        )]
        #[expect(
            clippy::cast_sign_loss,
            reason = "the `local < 0` guard above is what makes both \
                      non-negative: day_number is a truncating division of a \
                      non-negative value and into_day its remainder, so \
                      neither can be signed here. `the_before_epoch_boundary_\
                      is_the_ist_instant_not_the_sign` is the test that holds \
                      the guard up."
        )]
        let (days, second_of_day) = (day_number as u32, into_day as u32);
        // A day count that fits `u32` but that the calendar still refuses —
        // either its year is past 9999, or the epoch shift will not fit. Both
        // are reported as an out-of-range *timestamp* rather than passed
        // through, because the caller handed over a second and a second is
        // what it can act on. Both arms are reachable from here: the first at
        // `epoch_secs = 253_402_281_000`, the second above
        // `4_294_247_827` days, which is inside the `u32` the guard admits.
        let Ok(day) = Day::from_days(days) else {
            return Err(SessionError::TimestampOutOfRange { secs: epoch_secs });
        };
        // `SECS_PER_MINUTE`, not a bare `60`. The module header's own argument
        // about a constant repeated in three filters applies to this one too,
        // and it was declared here and then not used.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "SECS_PER_MINUTE is the literal 60 widened to i64; the \
                      cast back is exact and the compile-time assertion below \
                      is what keeps it exact if the constant ever moves."
        )]
        let per_minute = SECS_PER_MINUTE as u32;
        Ok(Self {
            day,
            minute_of_day: second_of_day / per_minute,
            second_of_minute: second_of_day % per_minute,
        })
    }

    /// The IST date.
    #[must_use]
    pub const fn day(self) -> Day {
        self.day
    }

    /// Minutes since IST midnight.
    #[must_use]
    pub const fn minute_of_day(self) -> u32 {
        self.minute_of_day
    }

    /// Seconds past that minute. Non-zero means the bar is not minute-aligned.
    #[must_use]
    pub const fn second_of_minute(self) -> u32 {
        self.second_of_minute
    }

    /// Whether this moment is inside the regular session, 09:15 inclusive to
    /// 15:30 exclusive.
    #[must_use]
    pub const fn in_regular_session(self) -> bool {
        self.minute_of_day >= SESSION_OPEN_MINUTE && self.minute_of_day < SESSION_CLOSE_MINUTE
    }
}

/// Which cadence a bar was fetched at.
///
/// An argument rather than an assumption: a daily bar carries no intraday time,
/// so the session bounds do not apply to it. See this module's header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cadence {
    /// One bar per minute. The session filter applies.
    Minute,
    /// One bar per trading day. **Exempt** from the session filter.
    Daily,
}

/// Why one bar was not stored.
///
/// Each reason is counted separately because they say different things: a bar
/// before the session open is the vendor including the pre-open auction, and a
/// bar outside the requested window is the vendor ignoring the range — and an
/// operator who sees one total cannot tell which happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DropReason {
    /// Before 09:15 IST — the pre-open auction, which is not a bar.
    BeforeSessionOpen,
    /// At or after 15:30 IST. **At** is a drop: the close is exclusive.
    AtOrAfterSessionClose,
    /// The date is before the operator's window.
    BeforeWindow,
    /// The date is after the operator's window.
    ///
    /// This is the reason that fires when the `toDate + 1` sent on the wire
    /// brings back a bar from the extra day. The bar is dropped, counted, and
    /// visible — `CLAUDE.md` §4, degrade loudly and name the reason.
    AfterWindow,
    /// The date is one [`crate::calendar::kind_of`] records as
    /// [`crate::calendar::DayKind::Closed`]: the exchange did not trade, so a
    /// bar stamped on it is not a session bar. Invariant P-03, D-2673.
    ///
    /// Only the calendar's own answer drops a bar here. A day the calendar has
    /// not measured is never dropped for being "probably" closed; it is kept
    /// and counted by name instead ([`DropCensus::unclassified_kept`]).
    OnClosedDay,
}

impl DropReason {
    /// A short, stable label, for a report.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::BeforeSessionOpen => "before the session open",
            Self::AtOrAfterSessionClose => "at or after the session close",
            Self::BeforeWindow => "before the requested window",
            Self::AfterWindow => "after the requested window",
            Self::OnClosedDay => "on a day the exchange calendar records closed",
        }
    }
}

impl fmt::Display for DropReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// How many bars were dropped, by reason.
///
/// Counters, not a log: `docs/07-o1-architecture.md` law 3. Answering "how many
/// did we discard" must be a read, and the reason must survive to the answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct DropCensus {
    before_open: u32,
    after_close: u32,
    before_window: u32,
    after_window: u32,
    closed_day: u32,
    unclassified_kept: u32,
}

impl DropCensus {
    /// A census of nothing dropped.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            before_open: 0,
            after_close: 0,
            before_window: 0,
            after_window: 0,
            closed_day: 0,
            unclassified_kept: 0,
        }
    }

    /// Add another census's tallies to this one, reason by reason.
    ///
    /// A window wider than the vendor's per-request cap is fetched as several
    /// chunks, each with its own census of what it declined. The receipt must
    /// account for every row the whole run read, so the tallies are summed —
    /// reporting only the last chunk's drops would make eighty of eighty-one
    /// chunks' declined rows vanish from a page whose entire purpose is that
    /// they do not.
    /// # Saturating, like every other counter in this type
    ///
    /// These four were bare `+=` while `count` beside them used
    /// `saturating_add` with the reason written out — *"a census that wrapped
    /// would report a smaller number than the truth"* — and `total` saturates
    /// too, with its own paragraph arguing the bound is honest.
    ///
    /// `overflow-checks` is on in both profiles, so a bare `+=` here **panics**
    /// rather than wrapping: an archive run whose out-of-session drops summed
    /// past `u32::MAX` would abort **after the bars were on disk and before the
    /// census was published**, which is the one window where a crash loses the
    /// most. Saturating reports a number that is too small and says so;
    /// panicking reports nothing and strands the run.
    pub const fn absorb(&mut self, other: Self) {
        self.before_open = self.before_open.saturating_add(other.before_open);
        self.after_close = self.after_close.saturating_add(other.after_close);
        self.before_window = self.before_window.saturating_add(other.before_window);
        self.after_window = self.after_window.saturating_add(other.after_window);
        self.closed_day = self.closed_day.saturating_add(other.closed_day);
        self.unclassified_kept = self
            .unclassified_kept
            .saturating_add(other.unclassified_kept);
    }

    /// Counts one drop. Saturating, because a census that wrapped would report
    /// a smaller number than the truth, which is the one direction a count must
    /// never be wrong in.
    pub const fn count(&mut self, reason: DropReason) {
        let slot = match reason {
            DropReason::BeforeSessionOpen => &mut self.before_open,
            DropReason::AtOrAfterSessionClose => &mut self.after_close,
            DropReason::BeforeWindow => &mut self.before_window,
            DropReason::AfterWindow => &mut self.after_window,
            DropReason::OnClosedDay => &mut self.closed_day,
        };
        *slot = slot.saturating_add(1);
    }

    /// How many were dropped for one reason.
    #[must_use]
    pub const fn of(&self, reason: DropReason) -> u32 {
        match reason {
            DropReason::BeforeSessionOpen => self.before_open,
            DropReason::AtOrAfterSessionClose => self.after_close,
            DropReason::BeforeWindow => self.before_window,
            DropReason::AfterWindow => self.after_window,
            DropReason::OnClosedDay => self.closed_day,
        }
    }

    /// Counts one bar that was KEPT on a day the exchange calendar cannot
    /// classify ([`crate::calendar::DayKind::Unmeasured`]: outside the
    /// calendar's measured range).
    ///
    /// Not a drop, and not part of [`Self::total`]: dropping it would claim the
    /// exchange was shut on a day nobody measured, which is an invented
    /// calendar fact. Counting it by name is what keeps the keep from being
    /// silent. D-2673.
    pub const fn count_unclassified_kept(&mut self) {
        self.unclassified_kept = self.unclassified_kept.saturating_add(1);
    }

    /// How many bars were kept on a day the exchange calendar cannot classify.
    #[must_use]
    pub const fn unclassified_kept(&self) -> u32 {
        self.unclassified_kept
    }

    /// How many were dropped in total.
    ///
    /// # The one place this is not the sum of its reasons
    ///
    /// Exactly the sum of the four counters while that sum fits `u32`, and
    /// **saturated at `u32::MAX` above it**, which is the one arithmetic in
    /// this file whose answer can be smaller than the truth. It is stated
    /// rather than hidden: reaching it needs more than 4.29 billion drops in
    /// one census, four hundred times a full day of index bars across every
    /// symbol NSE lists, so it is an honest bound rather than a live risk.
    /// `pull::unit::the_census_total_is_the_sum_of_its_reasons` proves the
    /// equality at zero, at one of each and at a mixed count;
    /// `session::tests::the_census_saturates_rather_than_wrapping` proves the
    /// saturation itself, and lives inside this module because the counters
    /// are private and no public call can reach `u32::MAX` in finite time.
    #[must_use]
    pub const fn total(&self) -> u32 {
        self.before_open
            .saturating_add(self.after_close)
            .saturating_add(self.before_window)
            .saturating_add(self.after_window)
            .saturating_add(self.closed_day)
    }

    /// Whether nothing was dropped.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

/// The operator's date range, **inclusive at both ends**.
///
/// Inclusive because that is what an operator means by "2022-01-08 to
/// 2022-02-08". The vendor's `toDate` is not inclusive, and reconciling the two
/// is [`Window::wire_to`] — the single most expensive silent bug available in
/// this crate, and the reason that method exists rather than a `+ 1` at a call
/// site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Window {
    from: Day,
    to: Day,
}

impl Window {
    /// A window from `from` to `to`, both included.
    ///
    /// # Errors
    ///
    /// [`SessionError::WindowRunsBackwards`] when `to` is before `from`. A
    /// one-day window (`from == to`) is legal and is the commonest resume
    /// shape.
    pub const fn new(from: Day, to: Day) -> Result<Self, SessionError> {
        // A hand-written comparison rather than `to < from`, because `Ord` is
        // not callable in a `const fn` — and the field order is the calendar
        // order, so this is the same comparison written out.
        let backwards = to.year < from.year
            || (to.year == from.year
                && (to.month < from.month || (to.month == from.month && to.day < from.day)));
        if backwards {
            return Err(SessionError::WindowRunsBackwards { from, to });
        }
        Ok(Self { from, to })
    }

    /// The first day the operator asked for.
    #[must_use]
    pub const fn from(self) -> Day {
        self.from
    }

    /// The last day the operator asked for.
    #[must_use]
    pub const fn to(self) -> Day {
        self.to
    }

    /// The `toDate` to put **on the wire**: the day after [`Window::to`].
    ///
    /// # The rule, and why it is a function
    ///
    /// **Dhan's `toDate` is not inclusive.** Its own documentation says so. A
    /// range an operator selects as inclusive must therefore be sent as
    /// `to + 1`, or every pull silently loses its last day — silently, because
    /// a short window looks exactly like a day the vendor had no data for, and
    /// the loss is one day per request rather than one day per pull.
    ///
    /// It is a method on the window rather than an addition at the call site
    /// for the reason `CLAUDE.md` §6 gives about a parameter: an adjustment
    /// that can be forgotten will be forgotten, and there is no symptom.
    /// `pull::unit::the_wire_to_date_is_the_day_after_the_operators_last_day`
    /// and `pull::unit::an_inclusive_window_survives_the_vendors_exclusive_
    /// to_date` are what hold it up.
    ///
    /// # Errors
    ///
    /// [`SessionError::NoNextDay`] for a window ending on 9999-12-31.
    ///
    /// # Examples
    ///
    /// ```
    /// # use pull::session::{Day, SessionError, Window};
    /// let window = Window::new(Day::new(2022, 1, 8)?, Day::new(2022, 2, 8)?)?;
    /// assert_eq!(window.to().to_string(), "2022-02-08");
    /// assert_eq!(window.wire_to()?.to_string(), "2022-02-09");
    /// # Ok::<(), SessionError>(())
    /// ```
    pub const fn wire_to(self) -> Result<Day, SessionError> {
        self.to.succ()
    }

    /// How many days the window spans, both ends included.
    ///
    /// Arithmetic on two day counts, so a one-day window and a ten-year one
    /// cost the same.
    #[must_use]
    pub const fn days(self) -> u32 {
        self.to.days_from_epoch() - self.from.days_from_epoch() + 1
    }

    /// Whether a date is inside the window.
    #[must_use]
    pub const fn contains(self, day: Day) -> bool {
        let after_start = day.days_from_epoch() >= self.from.days_from_epoch();
        after_start && day.days_from_epoch() <= self.to.days_from_epoch()
    }

    /// Whether one bar is kept, and if not, why.
    ///
    /// The order is the reason order: the window is checked before the session,
    /// so a bar from the extra day the wire `toDate` brings back is reported as
    /// *after the window* rather than as an out-of-session bar. Both are drops;
    /// only one of them tells an operator what actually happened.
    ///
    /// # Errors
    ///
    /// Whatever [`IstMoment::from_epoch_secs`] refuses. A timestamp that is not
    /// a timestamp is a refusal, never a drop: a drop is a bar this engine
    /// declined, and a bar it could not read is the vendor or the decoder being
    /// wrong.
    ///
    /// [`SessionError::SessionLengthUnmeasured`] for a minute bar on a day the
    /// exchange calendar records as a session of unmeasured length, and
    /// [`SessionError::VenueHoursUnknown`] for a day the venue's table carries
    /// no verified hours for.
    ///
    /// # Which session a minute is judged against
    ///
    /// On a day [`crate::calendar::kind_of`] records as an irregular session
    /// (the 2021-02-24 outage day, the two disaster-recovery Saturdays, the
    /// 2025 Muhurat hour) the calendar's own windows, on every venue. On every
    /// other day the venue's dated table. One session authority, so ingest
    /// never drops a minute the gap audit then calls a vendor hole. D-2670.
    pub fn verdict(
        self,
        epoch_secs: i64,
        cadence: Cadence,
        venue: crate::vendor::Venue,
    ) -> Result<Option<DropReason>, SessionError> {
        // `?` rather than a match: this stopped being a `const fn` when it
        // started reading the venue table, and the manual match only existed
        // because `?` was not const-callable.
        let at = IstMoment::from_epoch_secs(epoch_secs)?;
        let day = at.day().days_from_epoch();
        if day < self.from.days_from_epoch() {
            return Ok(Some(DropReason::BeforeWindow));
        }
        if day > self.to.days_from_epoch() {
            return Ok(Some(DropReason::AfterWindow));
        }
        // A CLOSED DAY HOLDS NO SESSION, AT ANY CADENCE (P-03, D-2673). The
        // exchange calendar's own answer, never a weekday rule: 2025-02-01 was
        // a Saturday and a full session, and `kind_of` says so. A daily bar is
        // NOT exempt here — its exemption is from intraday hours, and a closed
        // day has none. A day outside the calendar's measured range is not
        // `Closed` and is not dropped; `on_unclassified_day` lets a caller
        // count it by name.
        let kind = crate::calendar::kind_of(i64::from(day));
        if matches!(kind, crate::calendar::DayKind::Closed) {
            return Ok(Some(DropReason::OnClosedDay));
        }
        // A DAILY BAR HAS NO INTRADAY TIME AND IS EXEMPT. See the module
        // header: vendors stamp it at midnight, at the open or at the close,
        // and an intraday window would drop every one of them.
        if matches!(cadence, Cadence::Daily) {
            return Ok(None);
        }
        // THE HOURS COME FROM THE VENUE'S OWN TABLE, NOT FROM THESE CONSTANTS.
        //
        // They used to come from `SESSION_OPEN_MINUTE`/`SESSION_CLOSE_MINUTE`
        // directly, for every venue and every day, and on 2026-08-03 that
        // became wrong in the one place it matters most.
        //
        // NSE's CAS change (NSE/CMTR/74466) moved the segments apart from
        // 2026-08-03: derivatives extend to 15:40 (last bar 15:39); cash and
        // the swept INDEX keep 15:30 (last bar 15:29). `crate::vendor` has
        // each encoded, with citations, in `NSE_INDEX_SESSIONS`,
        // `NSE_CASH_SESSIONS` and `NSE_DERIVATIVES_SESSIONS` — and this
        // function never opened them.
        //
        // **Amended by D-4502 (audit satk-4).** This paragraph said the index
        // "now stops at 15:15, because every share it is computed from is
        // CAS-eligible", and the next one called its 15:15-15:29 bars "not
        // continuous-session bars". Neither is what the table this function
        // reads says. `NSE_INDEX_SESSIONS` keeps 15:30 from 2026-08-03 by the
        // operator's rule of 2026-08-20 ("the extension applies to futures and
        // options ALONE ... Nothing else moved"), and the vendors send index
        // bars through 15:29. 15:15 was an inference from the cash closing
        // auction about a derived value, not a published trading hour. So an
        // index bar from 15:15 to 15:29 is KEPT as a session bar, and the
        // charter's open question -- whether those minutes carry the frozen
        // index or the indicative auction index -- is still UNVERIFIED and
        // still a question for measurement, not for this window.
        //
        // Reading the table also removes the second source of truth. The
        // constants remain as the ANCHOR row's value — every table names them
        // — so there is one number, in one place, cited once.
        //
        // AN EXCEPTIONAL DAY ASKS THE EXCHANGE CALENDAR FIRST (CE-52, D-2670).
        // `crate::calendar::kind_of` is the session authority the gap audit,
        // `fold::minute_session` and the stored-run boundary all read. This
        // function used to read only the venue's regular hours, so on
        // 2021-02-24 it dropped the 15:45–16:59 reopening the calendar owes —
        // and the gap audit then reported those 75 minutes as a vendor hole no
        // re-pull could fill. Two authorities, one day, opposite answers.
        //
        // Only a day the calendar records as NOT a full regular session is
        // overridden: an irregular `Open` keeps exactly its own windows, and a
        // Muhurat whose length was never measured is refused by name. A full
        // day, a closed day and a day outside the calendar's measured range
        // fall through to the venue's table unchanged, which is what keeps the
        // dated 2026-08-03 derivatives row in force. One fixed-table lookup
        // per bar; `kind_of` states its own bound.
        match kind {
            crate::calendar::DayKind::Open(session)
                if session != crate::calendar::Session::full() =>
            {
                return Ok(irregular_verdict(session, at.minute_of_day()));
            }
            crate::calendar::DayKind::OpenLengthUnmeasured => {
                return Err(SessionError::SessionLengthUnmeasured { days: day });
            }
            crate::calendar::DayKind::Open(_)
            | crate::calendar::DayKind::Closed
            | crate::calendar::DayKind::Unmeasured => {}
        }
        let session = venue
            .hours_on(at.day())
            .map_err(|why| SessionError::VenueHoursUnknown {
                venue: why.venue().label(),
                days: why.day().days_from_epoch(),
            })?;
        if at.minute_of_day() < session.open_minute() {
            return Ok(Some(DropReason::BeforeSessionOpen));
        }
        if at.minute_of_day() >= session.close_minute() {
            return Ok(Some(DropReason::AtOrAfterSessionClose));
        }
        Ok(None)
    }
}

/// The verdict on one minute of a day the exchange calendar records as an
/// irregular session: kept exactly when the calendar says the session owed it.
///
/// A minute before the first window opens is [`DropReason::BeforeSessionOpen`].
/// Any other minute outside every window — after the last window, or in the
/// halt between two — is [`DropReason::AtOrAfterSessionClose`]: it is at or
/// after the close of the window it follows. No third reason is minted, so the
/// census, the receipt and the audit page keep their four named counters.
fn irregular_verdict(session: crate::calendar::Session, minute: u32) -> Option<DropReason> {
    // A minute of the day is below 1,440 and always fits; one that did not
    // would saturate to a minute outside every window and after the first
    // open, which is the after-close answer.
    let minute = u16::try_from(minute).unwrap_or(u16::MAX);
    // THE OPEN IS ASKED FIRST. A minute below the first window's open is in
    // no window, so asking it before `expects` changes no answer -- and it
    // puts the open minute itself in front of the strict `<`, where `<=` would
    // drop the session's first bar. Asked after `expects`, that minute had
    // already returned `None`, and `<` and `<=` were one program (D-2076).
    if session.windows.first().is_some_and(|w| minute < w.from) {
        return Some(DropReason::BeforeSessionOpen);
    }
    if session.expects(minute) {
        return None;
    }
    Some(DropReason::AtOrAfterSessionClose)
}

/// Whether `epoch_secs` falls on a day the exchange calendar cannot classify
/// ([`crate::calendar::DayKind::Unmeasured`]), so a bar kept on it must be
/// counted by name with [`DropCensus::count_unclassified_kept`].
///
/// `false` for a timestamp that is not one: [`Window::verdict`] refuses that
/// first, so no caller reaches this with it.
#[must_use]
pub fn on_unclassified_day(epoch_secs: i64) -> bool {
    IstMoment::from_epoch_secs(epoch_secs).is_ok_and(|at| {
        matches!(
            crate::calendar::kind_of(i64::from(at.day().days_from_epoch())),
            crate::calendar::DayKind::Unmeasured
        )
    })
}

impl fmt::Display for Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..={}", self.from, self.to)
    }
}

/// Split a window into consecutive legal chunks, each at most `cap_days` long
/// — or, when the vendor published no cap, each inside one calendar month.
///
/// **A capped chunk may cross a month boundary, and that is the design.** This
/// read *"each inside one calendar month"*, and the comments below said the
/// month bound "binds at every rung". Neither has been true since the month
/// clamp was removed for capped rungs (`ingest::months_in` now splits a landed
/// batch into one file per month, `docs/04-invariants.md` RG-04 and MR-38):
/// with `Some(cap)` the chunk ends at the earlier of the cap and the window's
/// last day, and the month is not consulted. D-1370.
///
/// ```
/// # use pull::session::{Day, SessionError, Window, split_window};
/// let window = Window::new(Day::new(2026, 7, 15)?, Day::new(2026, 8, 20)?)?;
/// // Capped: the first chunk runs straight across July into August.
/// let capped = split_window(window, Some(30))?;
/// assert_eq!(capped[0].to().to_string(), "2026-08-13");
/// // Uncapped: the month is the only bound left.
/// let uncapped = split_window(window, None)?;
/// assert_eq!(uncapped[0].to().to_string(), "2026-07-31");
/// # Ok::<(), SessionError>(())
/// ```
///
/// # Why this exists
///
/// A vendor caps how much history one request may name, and the cap is a
/// *vendor* bound, not an input bound. `api::ingest::MAX_WINDOW_DAYS` is the
/// latter — 3,653 days, wide enough for the whole stated backfill and narrow
/// enough that a fat-fingered year is caught. It says nothing about what a
/// broker will answer.
///
/// So a 2020-to-today window is accepted by the parser and, until this
/// function has a caller, handed whole to a vendor whose limit is 30 days at
/// one-minute granularity. What comes back is whatever that vendor decides to
/// do with an over-long range — an error if the operator is lucky, a silently
/// truncated answer if not, and the second one writes a gap the append-only
/// store cannot later correct.
///
/// # `None` is a cap the vendor did not publish, and it is not "no split"
///
/// The cap is per (feed, rung) — `crate::vendor::HttpSpec::window_cap_days` —
/// and a rung the charter records no figure for has none to apply. That is
/// `None`, and it means *the vendor puts no bound on this request*. It does
/// **not** mean the window goes out whole: with no cap the MONTH BOUNDARY is
/// the bound, so one unbounded request for a seven-year window is never sent.
/// The caller that read `None` as "send it whole" is the caller this argument
/// was widened to remove.
///
/// # The split
///
/// Chunks are **inclusive on both ends and non-overlapping**: a 30-day cap over
/// days 0..=64 yields `0..=29`, `30..=59`, `60..=64`. Overlapping them would
/// refetch a boundary day and make the run's cost depend on the chunking; a gap
/// between them would lose one. The last chunk is short whenever the span does
/// not divide evenly, which is the common case and not an edge case.
///
/// Returns the window unchanged in a single-element vector when it already
/// fits, so a caller has one code path rather than two.
///
/// # Errors
///
/// [`SessionError`] if `cap_days` is `Some(0)` — a cap of nothing cannot be
/// split into, and looping on it would not terminate. Refused rather than
/// silently read as `None`, which is a different claim about the vendor.
pub fn split_window(window: Window, cap_days: Option<u32>) -> Result<Vec<Window>, SessionError> {
    if cap_days == Some(0) {
        return Err(SessionError::WindowCapIsZero);
    }

    let first = window.from().days_from_epoch();
    let last = window.to().days_from_epoch();

    let mut chunks = Vec::new();
    let mut start = first;
    while start <= last {
        // THE MONTH BOUNDS ONLY AN UNCAPPED RUNG.
        //
        // This comment used to say "AND NEVER ACROSS A MONTH BOUNDARY" and that
        // the chunk ends at the earlier of the cap, the last day and the month
        // end. The code below has not done that since the clamp was removed for
        // capped rungs: `fetch::land` / `ingest::one` no longer refuse a batch
        // spanning two months, `ingest::months_in` writes one file per month
        // out of it, and clamping cost 81 requests where a 2,000-day cap needs
        // 2. So: with a published cap the chunk ends at the earlier of the cap
        // and the operator's last day; with none it ends at the earlier of the
        // month end and the last day. D-1370.
        //
        // `cap_days - 1` because both ends are INCLUDED: a 30-day chunk starting
        // at day 0 ends at day 29, not day 30. Off by one here would make every
        // chunk one day over the vendor's cap.
        let month_end = Day::from_days(start)?.end_of_month().days_from_epoch();
        let capped = match cap_days {
            Some(cap) => start.saturating_add(cap - 1),
            // A VENDOR THAT PUBLISHES NO CAP IS STILL ASKED A MONTH AT A TIME.
            // There is no number to chunk by, and one unbounded request for a
            // seven-year window is the one shape that cannot be retried
            // usefully when it fails.
            None => month_end,
        };
        let end = capped.min(last);
        chunks.push(Window::new(Day::from_days(start)?, Day::from_days(end)?)?);
        // `end + 1` cannot overflow past `last`'s guard: `end <= last` and
        // `last` is a real day, so the successor is at most one past a value
        // that already fit.
        start = end.saturating_add(1);
    }
    // THE CHUNK PLAN, RESOLVED — one line, whatever the plan's size.
    //
    // This function decides how many vendor requests a run will make and where
    // every boundary falls, and it was completely invisible. The entire opening
    // diagnosis of D-0070 was reconstructing THIS: that a five-year window at
    // the day rung becomes 61 month-shaped chunks because `cap_days` is `None`
    // and the month is the only bound. That is now one line instead of an
    // afternoon.
    //
    // `Debug` and bounded: one event per SPLIT, not per chunk. A split happens
    // once per instrument, so the count is bounded by the universe and never by
    // the window's length.
    //
    // `cap_days` is rendered as -1 when the vendor published none, never 0 —
    // "this vendor bounds nothing here" and "this vendor allows nothing" are
    // different facts and `CLAUDE.md` §3 rule 1 will not let them share a
    // number.
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::debug("pull.split", "window split")
            .with("from", telemetry::Value::Str(&window.from().to_string()))
            .with("to", telemetry::Value::Str(&window.to().to_string()))
            .with(
                "cap_days",
                telemetry::Value::Int(cap_days.map_or(-1, i64::from)),
            )
            .with("chunks", telemetry::Value::Uint(chunks.len() as u64))
            .with(
                "first_chunk_to",
                telemetry::Value::Str(
                    &chunks
                        .first()
                        .map_or_else(String::new, |c| c.to().to_string()),
                ),
            ),
    );
    Ok(chunks)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
/// The two facts about [`DropCensus`] that no external test can reach.
///
/// Everything else about this module is proved from outside, in
/// `crates/pull/tests/unit.rs`, because that is where a caller stands. These
/// two are here because the counters are private and the only public way to
/// raise one is [`DropCensus::count`], which would need 4,294,967,295 calls.
/// A saturating add nobody ever saturates is a claim, not a behaviour.
///
/// This paragraph sat above `split_window` and rendered as the first lines of
/// THAT function's documentation (D-1370).
mod tests {
    use super::{Cadence, Day, DropCensus, DropReason, SessionError, Window};

    /// A counter at the ceiling stops there. It never wraps to a number
    /// smaller than the truth, which is the one direction a census must never
    /// be wrong in.
    #[test]
    fn the_census_saturates_rather_than_wrapping() {
        let mut census = DropCensus {
            before_open: u32::MAX,
            after_close: 0,
            before_window: 0,
            after_window: 0,
            closed_day: 0,
            unclassified_kept: 0,
        };
        census.count(DropReason::BeforeSessionOpen);
        assert_eq!(
            census.of(DropReason::BeforeSessionOpen),
            u32::MAX,
            "the counter wrapped to zero instead of standing still"
        );
        assert_eq!(census.total(), u32::MAX);
        assert!(!census.is_empty());

        // Every slot saturates, not just the first one.
        for reason in [
            DropReason::BeforeSessionOpen,
            DropReason::AtOrAfterSessionClose,
            DropReason::BeforeWindow,
            DropReason::AfterWindow,
            DropReason::OnClosedDay,
        ] {
            let mut one = DropCensus::new();
            let slot = match reason {
                DropReason::BeforeSessionOpen => &mut one.before_open,
                DropReason::AtOrAfterSessionClose => &mut one.after_close,
                DropReason::BeforeWindow => &mut one.before_window,
                DropReason::AfterWindow => &mut one.after_window,
                DropReason::OnClosedDay => &mut one.closed_day,
            };
            *slot = u32::MAX;
            one.count(reason);
            assert_eq!(one.of(reason), u32::MAX, "{reason} wrapped");
        }
        // The kept-unclassified counter saturates too, and never joins the
        // drop total: those bars were kept.
        let mut kept = DropCensus::new();
        kept.unclassified_kept = u32::MAX;
        kept.count_unclassified_kept();
        assert_eq!(kept.unclassified_kept(), u32::MAX);
        assert_eq!(kept.total(), 0);
        assert!(kept.is_empty());
        let mut sum = DropCensus::new();
        sum.closed_day = 2;
        sum.unclassified_kept = 3;
        let mut into = DropCensus::new();
        into.absorb(sum);
        into.absorb(sum);
        assert_eq!(into.of(DropReason::OnClosedDay), 4);
        assert_eq!(into.unclassified_kept(), 6);
        assert_eq!(into.total(), 4);
    }

    /// `total` is the sum until it cannot be, and then it is the ceiling —
    /// never a wrapped, smaller number. This is the documented limit on
    /// [`DropCensus::total`], stated here as an executed fact.
    #[test]
    fn the_census_total_saturates_instead_of_wrapping_past_u32() {
        let census = DropCensus {
            before_open: u32::MAX,
            after_close: 1,
            before_window: 1,
            after_window: 1,
            closed_day: 0,
            unclassified_kept: 0,
        };
        // The true sum is 4,294,967,298. It does not fit, so the answer is the
        // ceiling — and emphatically not the wrapped 2.
        assert_eq!(census.total(), u32::MAX);
        assert_eq!(census.of(DropReason::AtOrAfterSessionClose), 1);

        // Two large counters, neither of them at the ceiling alone.
        let half = DropCensus {
            before_open: 3_000_000_000,
            after_close: 2_000_000_000,
            before_window: 0,
            after_window: 0,
            closed_day: 0,
            unclassified_kept: 0,
        };
        assert_eq!(half.total(), u32::MAX, "5e9 does not fit u32");

        // And the largest census whose total is still exact.
        let exact = DropCensus {
            before_open: u32::MAX - 3,
            after_close: 1,
            before_window: 1,
            after_window: 1,
            closed_day: 0,
            unclassified_kept: 0,
        };
        assert_eq!(exact.total(), u32::MAX);
        assert_eq!(
            u64::from(exact.before_open)
                + u64::from(exact.after_close)
                + u64::from(exact.before_window)
                + u64::from(exact.after_window),
            u64::from(u32::MAX),
            "this case is the boundary: the true sum is exactly u32::MAX"
        );
    }

    /// The IST epoch second at which `minute` of `epoch_day` opens.
    fn at(epoch_day: i64, minute: i64) -> i64 {
        epoch_day * 86_400 - super::IST_OFFSET_SECS + minute * 60
    }

    /// One minute verdict, for a one-day window on `epoch_day`.
    fn minute_verdict(
        epoch_day: i64,
        minute: i64,
        venue: crate::vendor::Venue,
    ) -> Result<Option<DropReason>, SessionError> {
        let day = Day::from_days(u32::try_from(epoch_day).unwrap()).unwrap();
        Window::new(day, day)
            .unwrap()
            .verdict(at(epoch_day, minute), Cadence::Minute, venue)
    }

    /// **ONE SESSION AUTHORITY (CE-52).** On every day the exchange calendar
    /// records as irregular, ingest keeps exactly the minutes the calendar says
    /// the session owed — no more and no fewer — on every NSE venue.
    ///
    /// Before this, `verdict` read only the venue's regular hours, so the
    /// 2021-02-24 reopening 15:45–16:59 (75 bars `calendar::kind_of` owes) was
    /// dropped as `AtOrAfterSessionClose` while the gap audit, asking the
    /// calendar, then reported the same 75 minutes as a vendor hole no re-pull
    /// could ever fill.
    #[test]
    fn the_ingest_verdict_keeps_exactly_what_the_calendar_owes_on_irregular_days() {
        let irregular = [
            crate::calendar::SYSTEMS_OUTAGE_DAY,
            19_784, // 2024-03-02 disaster-recovery Saturday
            19_861, // 2024-05-18 disaster-recovery Saturday
            20_382, // 2025-10-21 Muhurat, 13:45-14:44
        ];
        for epoch_day in irregular {
            let crate::calendar::DayKind::Open(session) = crate::calendar::kind_of(epoch_day)
            else {
                panic!("{epoch_day} is an open day in the calendar");
            };
            assert_ne!(session, crate::calendar::Session::full(), "{epoch_day}");
            for venue in crate::vendor::Venue::ALL {
                let mut kept = 0_u16;
                for minute in 0..1_440_u16 {
                    let verdict = minute_verdict(epoch_day, i64::from(minute), venue)
                        .expect("a calendar-measured day is never refused");
                    assert_eq!(
                        verdict.is_none(),
                        session.expects(minute),
                        "{venue} day {epoch_day} minute {minute}: the ingest verdict \
                         and the calendar disagree"
                    );
                    if verdict.is_none() {
                        kept += 1;
                    }
                }
                assert_eq!(kept, session.bars(), "{venue} day {epoch_day}");
            }
        }
        // The outage day's reopening by name: 15:45 and 16:59 kept, 17:00 not.
        let outage = crate::calendar::SYSTEMS_OUTAGE_DAY;
        let venue = crate::vendor::Venue::NseIndex;
        assert_eq!(minute_verdict(outage, 945, venue), Ok(None));
        assert_eq!(minute_verdict(outage, 1_019, venue), Ok(None));
        assert_eq!(
            minute_verdict(outage, 1_020, venue),
            Ok(Some(DropReason::AtOrAfterSessionClose))
        );
        assert_eq!(
            minute_verdict(outage, 554, venue),
            Ok(Some(DropReason::BeforeSessionOpen))
        );
        // And a regular day is still the venue's own table, untouched.
        assert_eq!(
            minute_verdict(outage + 1, 945, venue),
            Ok(Some(DropReason::AtOrAfterSessionClose))
        );
    }

    /// A Muhurat whose session length was never measured is REFUSED by name,
    /// never silently dropped against the regular hours and never admitted
    /// against them: nobody knows which minutes that session owed.
    #[test]
    fn an_unmeasured_muhurat_minute_is_refused_by_name_not_dropped() {
        for epoch_day in [18_580_i64, 18_935, 19_289, 19_673, 20_028] {
            assert_eq!(
                crate::calendar::kind_of(epoch_day),
                crate::calendar::DayKind::OpenLengthUnmeasured
            );
            for minute in [600_i64, 1_095] {
                for venue in crate::vendor::Venue::ALL {
                    assert_eq!(
                        minute_verdict(epoch_day, minute, venue),
                        Err(SessionError::SessionLengthUnmeasured {
                            days: u32::try_from(epoch_day).unwrap(),
                        }),
                        "{venue} {epoch_day} {minute}"
                    );
                }
            }
            // A daily bar has no intraday time and stays exempt.
            let day = Day::from_days(u32::try_from(epoch_day).unwrap()).unwrap();
            assert_eq!(
                Window::new(day, day).unwrap().verdict(
                    at(epoch_day, 0),
                    Cadence::Daily,
                    crate::vendor::Venue::NseIndex
                ),
                Ok(None)
            );
        }
        let text = SessionError::SessionLengthUnmeasured { days: 18_935 }.to_string();
        assert!(text.contains("18935") && text.contains("length"), "{text}");
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod month_boundary {
    use super::{Day, Window, split_window};

    fn day(y: u16, m: u8, d: u8) -> Day {
        Day::new(y, m, d).expect("a real calendar date")
    }

    /// **A chunk fills the vendor's cap, and the store splits it afterwards.**
    ///
    /// This used to assert that no chunk ever spanned two months, because
    /// `ingest::one` refused a batch that did. That refusal is gone —
    /// `ingest::months_in` writes one file per month out of a batch that may
    /// cover many — and the clamp it forced was expensive: Zerodha publishes
    /// **2,000 days per request at `day` level** (`Zerodha Docs/13-historical.md`
    /// Appendix A), and clamping to a month turned the operator's 2,460-day
    /// window into **81 requests where 2 suffice**. Measured against the
    /// 3-requests-per-second ceiling: 94 minutes rather than 2.3.
    ///
    /// What must still hold is what the vendor and the operator actually said:
    /// no chunk wider than the cap, and the chunks tile the window exactly.
    #[test]
    fn a_chunk_fills_the_cap_and_the_chunks_tile_the_window() {
        let cases = [
            (day(2026, 7, 1), day(2026, 8, 6)),
            (day(2024, 1, 1), day(2024, 12, 31)),
            (day(2024, 2, 1), day(2024, 3, 1)),
            (day(2020, 1, 1), day(2026, 8, 6)),
        ];
        // `None` IS IN THIS LIST, and it is the one case the month clamp still
        // serves: a rung the vendor published no cap for has no number to chunk
        // by, and one unbounded request for a seven-year window is the shape
        // that cannot be usefully retried when it fails.
        for cap in [Some(30_u32), Some(90), Some(1), Some(365), Some(2000), None] {
            for (from, to) in cases {
                let window = Window::new(from, to).expect("from precedes to");
                let chunks = split_window(window, cap).expect("a splittable window");
                let mut seen = Vec::new();
                for c in &chunks {
                    assert!(
                        cap.is_none_or(|cap| {
                            c.to().days_from_epoch() - c.from().days_from_epoch() < cap
                        }),
                        "cap {cap:?}: chunk is wider than the vendor allows",
                    );
                    if cap.is_none() {
                        assert_eq!(
                            (c.from().year(), c.from().month()),
                            (c.to().year(), c.to().month()),
                            "with no cap the month is the only bound left",
                        );
                    }
                    seen.push((c.from().days_from_epoch(), c.to().days_from_epoch()));
                }
                // AND NOTHING IS LOST. The chunks must tile the window exactly:
                // no gap (a missing day is missing data) and no overlap (a
                // duplicate day is a wasted request).
                let mut want = from.days_from_epoch();
                for (start, end) in &seen {
                    assert_eq!(*start, want, "cap {cap:?}: chunks do not tile at {want}");
                    want = end + 1;
                }
                assert_eq!(
                    want,
                    to.days_from_epoch() + 1,
                    "cap {cap:?}: the chunks do not reach the end of the window",
                );
            }
        }
    }

    /// **The saving, in the numbers the operator's own run produced.**
    ///
    /// 2,460 days at Zerodha's documented caps. Asserted rather than described,
    /// because the whole point of the change is the request count and a
    /// sentence about it cannot fail.
    #[test]
    fn the_documented_caps_turn_a_seven_year_window_into_a_handful_of_requests() {
        let window = Window::new(day(2019, 12, 2), day(2026, 8, 26)).expect("a real window");

        // `day` — 2,000 days per request.
        let daily = split_window(window, Some(2000)).expect("splittable");
        assert_eq!(daily.len(), 2, "2,460 days at 2,000 per request");

        // `minute` — 60 days per request.
        let minute = split_window(window, Some(60)).expect("splittable");
        assert_eq!(minute.len(), 41, "2,460 days at 60 per request");

        // AND WHAT THE MONTH CLAMP COST, kept as the comparison: one chunk per
        // month over the same window, at both rungs, is what this replaced.
        let by_month = split_window(window, None).expect("splittable");
        assert_eq!(by_month.len(), 81, "the old behaviour, for both rungs");
        assert!(
            by_month.len() / daily.len() >= 40,
            "the day pass issued at least forty times the requests it needed"
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod rolling_months {
    use super::{Day, MIN_YEAR, SessionError};

    fn day(y: u16, m: u8, d: u8) -> Day {
        Day::new(y, m, d).expect("a real calendar date")
    }

    /// A rolling floor stated in months lands on the same day of an earlier
    /// month, and crosses the year boundary by borrowing rather than by
    /// arithmetic on a day count.
    ///
    /// This is the shape `HistoryFloor::RollingMonths` resolves through, and
    /// the reason it is not a day count: the three-month floor Groww publishes
    /// is 89, 90, 91 or 92 days wide depending on where it is measured from,
    /// and every one of the cases below would be off by one to three days if
    /// the span were fixed at 90.
    #[test]
    fn a_month_walk_keeps_the_day_and_borrows_across_january() {
        for (from, months, want) in [
            // The vendor's own figure, from four different starting months.
            (day(2026, 8, 12), 3, day(2026, 5, 12)),
            (day(2026, 3, 15), 3, day(2025, 12, 15)),
            (day(2026, 1, 1), 1, day(2025, 12, 1)),
            (day(2026, 1, 31), 12, day(2025, 1, 31)),
            // Zero months is this day. A floor of "none of the past" is not a
            // shape any vendor states, and the identity still has to hold.
            (day(2026, 8, 12), 0, day(2026, 8, 12)),
            // More than a year, so both the whole-year division and the
            // remainder borrow at once.
            (day(2026, 2, 10), 14, day(2024, 12, 10)),
            (day(2026, 12, 31), 24, day(2024, 12, 31)),
        ] {
            assert_eq!(
                from.months_before(months).expect("inside the calendar"),
                want,
                "{from} less {months} months"
            );
        }
    }

    /// THE DAY IS CLAMPED DOWN, NEVER ROLLED FORWARD.
    ///
    /// 31 May less three months is the last day February has. Rolling forward
    /// into March — which is what a naive date library does — would move the
    /// floor LATER than the vendor's own, and a floor that is too late refuses
    /// days the vendor would have answered. Silently: the operator sees a
    /// shorter window, not a refusal.
    #[test]
    fn a_short_month_clamps_the_day_rather_than_spilling_into_the_next() {
        assert_eq!(
            day(2026, 5, 31).months_before(3).expect("February exists"),
            day(2026, 2, 28),
            "2026 is not a leap year"
        );
        assert_eq!(
            day(2024, 3, 31).months_before(1).expect("February exists"),
            day(2024, 2, 29),
            "2024 is, and the clamp reads the same month_len `new` validates with"
        );
        assert_eq!(
            day(2026, 7, 31).months_before(1).expect("June exists"),
            day(2026, 6, 30),
            "a 30-day month takes the 30th, not the 1st of July"
        );
        // AND AN ORDINARY DAY IS UNTOUCHED, which is the branch the three
        // above would pass without if the clamp fired unconditionally.
        assert_eq!(
            day(2026, 5, 15).months_before(3).expect("May"),
            day(2026, 2, 15)
        );
    }

    /// A walk that lands before 1970 is refused by name, with the bound in the
    /// message — the same refusal `Day::new` gives, because it IS `Day::new`'s.
    #[test]
    fn a_walk_below_the_first_year_this_build_can_name_is_refused() {
        let first = day(u16::try_from(MIN_YEAR).expect("1970 fits"), 1, 1);
        let Err(why) = first.months_before(1) else {
            panic!("there is no December 1969 in this calendar")
        };
        assert!(
            matches!(why, SessionError::YearOutOfRange { year: 1969 }),
            "the year it landed on is named: {why}"
        );
        // The whole-years path lands below the floor too, and saturates rather
        // than wrapping to a year in the far future.
        assert!(
            first.months_before(u8::MAX).is_err(),
            "twenty-one years before 1970 is not a date either"
        );
    }
}
