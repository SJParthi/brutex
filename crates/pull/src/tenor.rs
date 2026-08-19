//! Time to expiry, measured in SECONDS — the input `greeks` asks for in years
//! and this repository has never produced.
//!
//! # The gap this closes
//!
//! `greeks::bsm::Contract` takes five inputs. Four of them exist here already:
//! the strike is parsed by [`crate::fno::read_contract`], the spot arrives on
//! Dhan's overlay or joins from the index bar, the carry is zero for an index
//! with no dividend adjustment, and the rate is Phase 2's problem. The fifth is
//! `years_to_expiry`, and **nothing in this workspace computes it** — measured
//! with `grep`, not assumed: outside `crates/greeks` the identifier does not
//! appear.
//!
//! # Why the unit here is seconds and not years
//!
//! Because seconds is what can be MEASURED and years is what must be DECLARED.
//!
//! A bar carries a microsecond stamp. A session carries a close minute. Both
//! are exact integers, so the distance between them is an exact integer, and
//! computing it needs no convention and no rounding. Turning that into a
//! fraction of a year needs a year length, and a year length is a choice —
//! 365, 365.25, or 252 trading days all have defenders and none of them is a
//! fact about NSE that `docs/00-charter.md` records. §3 rule 1 is what stops
//! this module picking one silently, so [`YearBasis`] makes the caller name it.
//!
//! There is a second reason, and it is the one that would have bitten. **Indian
//! index options expire weekly and monthly, never annually.** A NIFTY weekly
//! lives about seven calendar days — `0.019` of a year. Its whole life is spent
//! in a band that `greeks`' own finite-difference validation SKIPS:
//!
//! ```text
//! // crates/greeks/src/bsm.rs, in the analytic-vs-numerical test
//! if contract.years_to_expiry < 0.02 {
//!     continue;
//! }
//! ```
//!
//! `0.02` years is 7.3 days. Every weekly contract this engine would ever price
//! is born below that line and dies below it, so the crate's own proof that its
//! greeks match a numerical derivative **has never run at the maturity where
//! this market actually trades**. That is not an argument against the crate; it
//! is an argument for measuring the tenor exactly and reporting it in a unit
//! that makes the smallness visible, rather than handing `0.0027` to a model
//! and hoping.
//!
//! # The expiry MOMENT is not a new decision
//!
//! It is the instant trading stops on the expiry day, and this repository
//! already knows it per date: [`crate::vendor::Venue::NseDerivatives`] carries
//! the dated session table, including the change that took the derivatives
//! close from 15:30 to 15:40 on 2026-08-03. Reading it here means the tenor
//! picks that change up for free and cannot disagree with the filter that
//! decided which bars were stored in the first place.
//!
//! An expiry day whose hours are not verified is REFUSED, not guessed. A tenor
//! computed against invented hours is wrong by up to a full session on the one
//! day where a wrong tenor costs the most.
//!
//! # Cost
//!
//! **O(1) time, O(1) space, no allocation, no calendar walk.** One
//! `IstMoment::from_epoch_secs` (closed-form civil-date arithmetic), one
//! `Day::new`, one `hours_on` — bounded by
//! [`crate::vendor::MAX_LATER_SESSION_ROWS`] `+ 1 = 3` rows, a compile-time
//! constant no input can raise — and a fixed number of integer operations. The
//! only float in the module is produced by [`Tenor::years`], once, at the
//! boundary where the model demands one.

use brutex_core::instrument::Expiry;

use crate::session::{Day, IstMoment};
use crate::vendor::{SessionRefusal, Venue};

/// Seconds in a minute, named because it appears in the arithmetic twice.
const SECONDS_PER_MINUTE: i64 = 60;

/// Seconds in a calendar day.
const SECONDS_PER_DAY: i64 = 24 * 60 * SECONDS_PER_MINUTE;

/// How long a year is, for the one conversion that needs to know.
///
/// # Why this is an enum with one variant rather than a constant
///
/// A constant can be used without being named, and this quantity must never be
/// used without being named. The rate `r` and the tenor `T` enter Black-Scholes
/// through the same discounting, so a rate measured under one basis is **not
/// valid under another** — swap the basis and the same option reprices. Phase 2
/// measures `r` by solving against Dhan's published implied volatility, and
/// that measurement is only meaningful if the basis it was taken under travels
/// with it.
///
/// A second variant is a new arm here and a new measurement of `r`, not a
/// tweak. That is the point of the shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum YearBasis {
    /// 365 calendar days.
    ///
    /// **Declared by this repository, not measured from NSE.** No page in
    /// `docs/00-charter.md` records an exchange day-count for option pricing,
    /// so this is a convention chosen to be fixed and stated rather than a fact
    /// about the venue. §3 rule 1 is why that sentence is here instead of a
    /// citation that does not exist.
    Calendar365,
}

impl YearBasis {
    /// Seconds in one year under this basis.
    #[must_use]
    pub const fn seconds(self) -> i64 {
        match self {
            Self::Calendar365 => 365 * SECONDS_PER_DAY,
        }
    }
}

/// Why a tenor could not be measured.
///
/// Every arm names something that was READ and refused. None of them is a
/// fallback: a tenor this module cannot measure is one no caller should price
/// against, and returning a plausible number instead is the §4 failure wearing
/// a success's clothes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TenorError {
    /// The bar's stamp is not a moment any calendar can place.
    ///
    /// What a vendor column read at the wrong offset looks like — see
    /// [`IstMoment::from_epoch_secs`], which is the reader that refused it.
    StampOffCalendar {
        /// The stamp, as it arrived.
        ts_micros: i64,
    },
    /// The expiry day's trading hours are not verified for that date.
    ///
    /// Refused rather than guessed: see the module header.
    HoursUnverified(SessionRefusal),
    /// The bar is at or after the moment the contract stopped trading.
    ///
    /// **Zero is in this arm, not outside it.** A bar stamped exactly at the
    /// close has no time value left, and Black-Scholes at `T = 0` is a division
    /// by zero dressed as a limit. `greeks` refuses it too — `GreeksError::
    /// Expired` — so refusing here means the caller learns it from the reader
    /// that knows the calendar rather than from the model that does not.
    AlreadyExpired {
        /// How far past the close the bar is, in seconds. Zero means exactly
        /// at it.
        seconds_past: i64,
    },
    /// The expiry is not a day this store's calendar can represent.
    ///
    /// **Unreachable by construction**, and proved rather than asserted:
    /// `Expiry::new` and `Day::new` validate a year, a month and a day of month
    /// against the same real month lengths, and `Expiry`'s accepted years
    /// (1990..=2100) are a subset of `Day`'s. The test
    /// `the_two_calendars_agree_so_the_impossible_arm_is_impossible` walks the
    /// boundary dates and asserts the two answers never differ.
    ///
    /// It exists anyway because the alternative is a `panic!` or a substituted
    /// date, and §4 bans one and §3 bans the other.
    ExpiryOffCalendar {
        /// The expiry that could not be placed.
        expiry: Expiry,
    },
}

impl std::fmt::Display for TenorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StampOffCalendar { ts_micros } => write!(
                f,
                "the bar's timestamp {ts_micros} is not a moment any calendar \
                 can place, which is what a vendor column read at the wrong \
                 offset looks like. No tenor was measured."
            ),
            Self::HoursUnverified(refusal) => write!(
                f,
                "the expiry day's trading hours are not verified, so the \
                 moment the contract stopped trading is unknown and the tenor \
                 was refused rather than guessed: {refusal}"
            ),
            Self::AlreadyExpired { seconds_past } => write!(
                f,
                "the bar is {seconds_past} second(s) at or past the close of \
                 the expiry day, so the contract had no time value left. \
                 Black-Scholes has no answer at or below zero time and neither \
                 does this."
            ),
            Self::ExpiryOffCalendar { expiry } => write!(
                f,
                "the expiry {expiry} is not a day this store's calendar can \
                 represent, which the two calendars agreeing is supposed to \
                 make impossible. No tenor was measured."
            ),
        }
    }
}

impl std::error::Error for TenorError {}

/// How long one contract had left to run, at one bar.
///
/// Held as an exact second count. See the module header on why that is the
/// stored unit and why [`years`](Self::years) takes an explicit basis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tenor {
    /// Strictly positive — [`TenorError::AlreadyExpired`] covers the rest.
    seconds: i64,
}

impl Tenor {
    /// The tenor of `expiry` at the bar stamped `ts_micros`.
    ///
    /// The expiry moment is the close of the NSE derivatives session on the
    /// expiry day, read from the dated table rather than fixed here — so the
    /// 2026-08-03 change from 15:30 to 15:40 applies without this module
    /// knowing about it.
    ///
    /// # Errors
    ///
    /// Every arm of [`TenorError`]. None is a fallback.
    ///
    /// # Cost
    ///
    /// O(1) — see the module header for the exact bound.
    pub fn between(ts_micros: i64, expiry: Expiry) -> Result<Self, TenorError> {
        let at = IstMoment::from_epoch_secs(ts_micros.div_euclid(1_000_000))
            .map_err(|_| TenorError::StampOffCalendar { ts_micros })?;
        let last_day = Day::new(expiry.year(), expiry.month(), expiry.day())
            .map_err(|_| TenorError::ExpiryOffCalendar { expiry })?;

        // THE DATED TABLE, NOT A CONSTANT. `close_minute` is exclusive — the
        // end of the session, which is exactly the instant trading stops.
        let close = Venue::NseDerivatives
            .hours_on(last_day)
            .map_err(TenorError::HoursUnverified)?
            .close_minute();

        // BOTH SIDES AS SECONDS SINCE THEIR OWN IST MIDNIGHT, then one day
        // difference. `days_from_epoch` is closed-form; nothing walks a
        // calendar. The widest span this store can hold is under 2^23 days,
        // so the multiply cannot overflow an i64 by any margin worth guarding.
        let days = i64::from(last_day.days_from_epoch()) - i64::from(at.day().days_from_epoch());
        let close_secs = i64::from(close) * SECONDS_PER_MINUTE;
        let bar_secs =
            i64::from(at.minute_of_day()) * SECONDS_PER_MINUTE + i64::from(at.second_of_minute());
        let seconds = days * SECONDS_PER_DAY + close_secs - bar_secs;

        if seconds <= 0 {
            return Err(TenorError::AlreadyExpired {
                seconds_past: -seconds,
            });
        }
        Ok(Self { seconds })
    }

    /// Seconds left, exactly.
    #[must_use]
    pub const fn seconds(self) -> i64 {
        self.seconds
    }

    /// Whole minutes left, truncated.
    ///
    /// Truncated rather than rounded, and it matters at the only place it
    /// differs: a bar 59 seconds from the close has ZERO whole minutes left,
    /// which is true. Rounding it to one would be the only lie available here.
    #[must_use]
    pub const fn minutes(self) -> i64 {
        self.seconds / SECONDS_PER_MINUTE
    }

    /// Whole calendar days left, truncated.
    ///
    /// The unit an operator reads a contract in — "three days to expiry" — and
    /// the one that makes a weekly's tenor legible where `0.0192` does not.
    #[must_use]
    pub const fn calendar_days(self) -> i64 {
        self.seconds / SECONDS_PER_DAY
    }

    /// The fraction of a year left, under an explicitly named basis.
    ///
    /// **The only float this module produces**, and the only place a convention
    /// enters. See [`YearBasis`] on why the basis is a parameter and why a rate
    /// measured under one is not valid under another.
    ///
    /// Strictly positive: [`Self::seconds`] is, by construction.
    #[must_use]
    #[expect(
        clippy::float_arithmetic,
        reason = "THE ONE FLOAT BOUNDARY IN THIS CRATE, and the lint is right \
                  to make it argue for itself. `pull` denies float arithmetic \
                  because §7 says PRICES are paisa integers and never floats, \
                  and every earlier attempt to make one a float was a bug. \
                  This is not a price. It is a fraction of a year — a ratio of \
                  two second counts, which §7 puts in the class that 'keeps \
                  full precision and is never rounded for storage' beside \
                  Sharpe and p-values. No integer type can carry it, and \
                  `greeks::bsm::Contract::years_to_expiry` is an f64 that this \
                  workspace does not get to redefine. The exact value stays \
                  available as `seconds()`; this is the lossy view, produced \
                  once, where the model demands it"
    )]
    #[expect(
        clippy::cast_precision_loss,
        reason = "both sides are second counts under 2^40, which f64 \
                  represents exactly to 2^53. The quotient is the fraction no \
                  integer type can carry"
    )]
    pub fn years(self, basis: YearBasis) -> f64 {
        self.seconds as f64 / basis.seconds() as f64
    }

    /// Whether this tenor sits below the band `greeks` validates its own
    /// derivatives in.
    ///
    /// `crates/greeks/src/bsm.rs` skips `years_to_expiry < 0.02` in its
    /// analytic-versus-numerical test, because a one-hour maturity has a theta
    /// the size of the position and the central difference needs room on both
    /// sides. **Every NIFTY weekly is born under that line**, so a caller
    /// pricing one is outside the region the crate proves itself in, and it
    /// should be able to say so on a report rather than discovering it later.
    ///
    /// This is not a refusal. It is a fact about the tenor that a caller may
    /// carry, and `docs/06-limits.md` is where the consequence belongs.
    #[must_use]
    pub fn below_validated_band(self, basis: YearBasis) -> bool {
        self.years(basis) < VALIDATED_BAND_YEARS
    }

    /// Seconds below which a tenor is outside the band `greeks` validates.
    ///
    /// The integer form of [`VALIDATED_BAND_YEARS`], so a caller can compare
    /// without producing a float at all. Exact under
    /// [`YearBasis::Calendar365`]: 0.02 × 365 days is 7.3 days to the second.
    #[must_use]
    pub const fn validated_band_seconds(basis: YearBasis) -> i64 {
        // 0.02 == 1/50, so this is the basis over fifty — an exact integer
        // division rather than a float multiply that would need rounding.
        basis.seconds() / 50
    }
}

/// The maturity below which `greeks`' own finite-difference test does not run.
///
/// Read from `crates/greeks/src/bsm.rs`, where the loop reads
/// `if contract.years_to_expiry < 0.02 { continue; }`. Named here so the number
/// has one home and a reader can find why it is 0.02 rather than guessing.
pub const VALIDATED_BAND_YEARS: f64 = 0.02;

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::*;

    /// Epoch micros for an IST wall-clock moment.
    fn ist(year: u16, month: u8, day: u8, hour: i64, minute: i64) -> i64 {
        let d = Day::new(year, month, day).expect("a real day");
        let secs =
            i64::from(d.days_from_epoch()) * SECONDS_PER_DAY - 19_800 + hour * 3_600 + minute * 60;
        secs * 1_000_000
    }

    fn expiry(year: u16, month: u8, day: u8) -> Expiry {
        Expiry::new(year, month, day).expect("a real expiry")
    }

    /// **THE DATED SESSION TABLE IS READ, NOT A CONSTANT.**
    ///
    /// The derivatives close moved from 15:30 to 15:40 on 2026-08-03. A bar at
    /// 15:00 on an expiry day therefore has thirty minutes left before that
    /// date and forty minutes left after it, and this module must not have to
    /// be edited for that to be true.
    #[test]
    fn the_expiry_moment_follows_the_dated_close_and_is_not_hardcoded() {
        let before = Tenor::between(ist(2026, 7, 30, 15, 0), expiry(2026, 7, 30))
            .expect("a July expiry day at 15:00");
        let after = Tenor::between(ist(2026, 8, 27, 15, 0), expiry(2026, 8, 27))
            .expect("an August expiry day at 15:00");

        assert_eq!(before.minutes(), 30, "15:00 to a 15:30 close");
        assert_eq!(
            after.minutes(),
            40,
            "15:00 to a 15:40 close — the 2026-08-03 change, read from the \
             table rather than written here"
        );
    }

    /// **A WEEKLY'S WHOLE LIFE IS BELOW THE BAND `greeks` VALIDATES.**
    ///
    /// This is the unit that makes the module's argument checkable. A contract
    /// listed seven days before it expires never once reaches `0.02` years, so
    /// the analytic-versus-numerical proof in `crates/greeks` has never run at
    /// the maturity this market trades.
    #[test]
    fn every_bar_of_a_seven_day_weekly_sits_below_the_validated_band() {
        let dies = expiry(2026, 8, 27);
        for day in 20..=27 {
            let at = Tenor::between(ist(2026, 8, day, 9, 15), dies).expect("a bar in its life");
            assert!(
                at.below_validated_band(YearBasis::Calendar365),
                "day {day}: {} years is not below {VALIDATED_BAND_YEARS}",
                at.years(YearBasis::Calendar365)
            );
        }
        // AND THE NUMBER ITSELF, so the claim is legible rather than relative.
        //
        // 0.0199 — the first bar of a seven-day weekly sits HALF A PERCENT
        // under the 0.02 line, and every bar after it is further under. The
        // margin is that thin, which is why the band is worth naming: a
        // contract listed eight days out would cross it and one listed seven
        // never does, and no report today can tell those two apart.
        let born = Tenor::between(ist(2026, 8, 20, 9, 15), dies).expect("the first bar");
        assert_eq!(born.calendar_days(), 7);
        assert_eq!(
            born.seconds(),
            7 * 86_400 + 6 * 3_600 + 25 * 60,
            "seven days plus 09:15 to the 15:40 close"
        );
        assert!(
            (born.years(YearBasis::Calendar365) - 0.019_910).abs() < 1e-5,
            "a seven-day weekly is {} of a year",
            born.years(YearBasis::Calendar365)
        );
    }

    /// Zero time left is EXPIRED, not a tenor of zero.
    ///
    /// The boundary is exact and it is closed on the wrong side for a model:
    /// Black-Scholes at `T = 0` is a division by zero, so the reader that knows
    /// the calendar refuses before the model that does not has to.
    #[test]
    fn a_bar_at_or_past_the_close_is_expired_rather_than_a_zero_tenor() {
        let dies = expiry(2026, 8, 27);
        assert_eq!(
            Tenor::between(ist(2026, 8, 27, 15, 40), dies),
            Err(TenorError::AlreadyExpired { seconds_past: 0 }),
            "exactly at the close is past it, not at it"
        );
        assert_eq!(
            Tenor::between(ist(2026, 8, 27, 15, 41), dies),
            Err(TenorError::AlreadyExpired { seconds_past: 60 })
        );
        // ONE MINUTE EARLIER IS A REAL, TINY TENOR.
        let alive = Tenor::between(ist(2026, 8, 27, 15, 39), dies).expect("the last live bar");
        assert_eq!(alive.seconds(), 60);
        assert_eq!(alive.minutes(), 1);
        assert_eq!(alive.calendar_days(), 0);
    }

    /// Minutes truncate, and the one place it shows is the honest one.
    #[test]
    fn a_partial_minute_is_zero_whole_minutes_rather_than_one() {
        let dies = expiry(2026, 8, 27);
        // 15:39:01 is 59 seconds from a 15:40 close.
        let at = Tenor::between(ist(2026, 8, 27, 15, 39) + 1_000_000, dies).expect("59 seconds");
        assert_eq!(at.seconds(), 59);
        assert_eq!(at.minutes(), 0, "59 seconds is not a minute");
    }

    /// A stamp no calendar can place is refused, naming the stamp.
    #[test]
    fn a_stamp_off_the_calendar_is_refused_and_names_itself() {
        let error = Tenor::between(i64::MIN, expiry(2026, 8, 27)).unwrap_err();
        assert_eq!(
            error,
            TenorError::StampOffCalendar {
                ts_micros: i64::MIN
            }
        );
        assert!(
            error.to_string().contains("wrong offset"),
            "the message must say what this looks like: {error}"
        );
    }

    /// The year basis is named at the call site and changes the answer.
    ///
    /// Not a tautology: it is the unit that would catch a basis silently
    /// hardcoded inside `years` and ignoring its argument.
    #[test]
    fn the_year_basis_is_the_only_convention_and_it_is_applied() {
        let at = Tenor::between(ist(2026, 8, 20, 9, 15), expiry(2026, 8, 27)).expect("seven days");
        assert_eq!(YearBasis::Calendar365.seconds(), 365 * 24 * 60 * 60);
        #[expect(
            clippy::cast_precision_loss,
            reason = "a unit that checks a float conversion has to widen an \
                      integer to check it"
        )]
        {
            let years = at.years(YearBasis::Calendar365);
            assert!(
                (years * 365.0 - at.seconds() as f64 / 86_400.0).abs() < 1e-9,
                "years must be exactly seconds over the basis"
            );
        }
        // AND THE INTEGER FORM AGREES, which is the comparison a caller should
        // reach for: no float is produced at all.
        assert_eq!(
            Tenor::validated_band_seconds(YearBasis::Calendar365),
            630_720
        );
        assert!(at.seconds() < Tenor::validated_band_seconds(YearBasis::Calendar365));
    }

    /// **PROOF THAT [`TenorError::ExpiryOffCalendar`] CANNOT HAPPEN.**
    ///
    /// `Expiry::new` and `Day::new` validate the same three fields against the
    /// same real month lengths, and `Expiry`'s years are a subset of `Day`'s.
    /// This walks the dates where two calendars are most likely to disagree —
    /// month ends, leap days, century non-leaps and both range bounds — and
    /// asserts they never do. The arm stays in the type because the
    /// alternatives are a panic or a substituted date.
    #[test]
    fn the_two_calendars_agree_so_the_impossible_arm_is_impossible() {
        let mut checked = 0_u32;
        for year in [1990_u16, 1999, 2000, 2020, 2023, 2024, 2026, 2099, 2100] {
            for month in 1_u8..=12 {
                for day in [1_u8, 28, 29, 30, 31, 32, 0] {
                    assert_eq!(
                        Expiry::new(year, month, day).is_ok(),
                        Day::new(year, month, day).is_ok(),
                        "{year}-{month}-{day}: the two calendars disagree, so \
                         ExpiryOffCalendar is reachable after all"
                    );
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, 9 * 12 * 7, "every boundary date was compared");
    }

    /// A tenor is ordered by what it is, so a caller can take the shortest.
    #[test]
    fn tenors_compare_by_time_left() {
        let dies = expiry(2026, 8, 27);
        let early = Tenor::between(ist(2026, 8, 20, 9, 15), dies).expect("a week out");
        let late = Tenor::between(ist(2026, 8, 27, 9, 15), dies).expect("expiry morning");
        assert!(late < early, "less time left sorts first");
    }
}
