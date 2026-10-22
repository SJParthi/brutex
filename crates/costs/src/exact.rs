//! Exact rates, and the half-up rounding the precise charge stack uses.
//!
//! [`crate::rate::BpsX100`] has a fixed scale of one rupee per crore, and that
//! is too coarse for the figures package CHG-1 prices: NSE circular `FA73061`
//! quotes the cash-market transaction charge as **₹306.99** per crore and the
//! investor protection fund as **₹0.01** per crore. A paisa per crore is a
//! hundred times finer than one `BpsX100` unit, so a rate here is a
//! [`Ratio`] — an integer numerator over an integer denominator, both fixed at
//! compile time, with no scale to outgrow.
//!
//! # Rounding
//!
//! The existing stack ([`crate::money`]) **ceils** every levy on purpose,
//! because its contract is to never flatter a backtest. The precise stack
//! (`crate::stack`) has the opposite contract: reproduce a contract note.
//! A contract note rounds to the **nearest** unit, so this module rounds
//! **half-up** — a remainder of at least half a unit goes up, anything less
//! goes down. Which way an exact half goes is not stated by any source this
//! repository holds, so half-up is a choice and is recorded as UNVERIFIED in
//! `docs/06-limits.md` §CHG-1 and in D-5113.
//!
//! # Constant cost
//!
//! [`round_half_up`] is one `div_euclid`, one `rem_euclid`, one doubling and
//! one comparison. [`Ratio::apply`] adds two multiplications and one
//! narrowing. No loop, no allocation, nothing that depends on a magnitude.
//!
//! # No floats, and no wrapping
//!
//! Every product is formed in `i128`, where an `i64 × i64` cannot overflow.
//! The one narrowing back to [`Paisa`] is [`crate::money::narrow`], which
//! refuses rather than saturates.

use std::fmt;

use brutex_core::price::Paisa;

use crate::charge_error::ChargeError;
use crate::error::CostError;
use crate::money::narrow;
use crate::rate::{BpsX100, RATE_SCALE};

/// The unit a charge is rounded to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Quantum {
    /// To the paisa — two decimals of a rupee.
    Paisa,
    /// To the whole rupee.
    Rupee,
}

impl Quantum {
    /// How many paisa one unit of this quantum is.
    #[must_use]
    pub const fn paisa(self) -> i64 {
        match self {
            Self::Paisa => 1,
            Self::Rupee => brutex_core::price::PAISA_PER_RUPEE,
        }
    }
}

/// An exact rate: a non-negative numerator over a positive denominator.
///
/// `charge = value × numerator / denominator`, rounded once, at the end.
///
/// The constructor is crate-private for the reason [`BpsX100`]'s is: a ratio
/// is a claim that a citation exists, and every ratio in circulation is a
/// `const` this crate owns beside the source it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ratio {
    num: i64,
    den: i64,
}

impl Ratio {
    /// Wraps a numerator and a denominator. Crate-private; see the type.
    pub(crate) const fn new(num: i64, den: i64) -> Self {
        Self { num, den }
    }

    /// A [`BpsX100`] read as the exact fraction it encodes: `bps / RATE_SCALE`.
    #[must_use]
    pub const fn from_bps_x100(rate: BpsX100) -> Self {
        Self::new(rate.get(), RATE_SCALE)
    }

    /// The numerator.
    #[must_use]
    pub const fn numerator(self) -> i64 {
        self.num
    }

    /// The denominator.
    #[must_use]
    pub const fn denominator(self) -> i64 {
        self.den
    }

    /// Whether this ratio can be a rate: a positive denominator and a
    /// numerator that is not negative.
    ///
    /// A `const fn`, so every shipped ratio is checked by the compiler.
    #[must_use]
    pub const fn is_well_shaped(self) -> bool {
        self.den > 0 && self.num >= 0
    }

    /// `value × self / divisor`, rounded half-up to `quantum`.
    ///
    /// `divisor` is the extra integer denominator one charge needs: the
    /// intraday transaction tax is levied on the day's turnover **halved**
    /// (`crate::stack`), and halving it before the rate would round twice.
    ///
    /// # Errors
    ///
    /// [`CostError::Overflow`] when the rounded charge leaves `i64`.
    /// [`CostError::NotPositive`] when `divisor` is not positive — a charge
    /// divided by nothing is not a charge.
    ///
    /// # Examples
    ///
    /// ```
    /// use brutex_core::price::Paisa;
    /// use costs::exact::Quantum;
    /// use costs::schedule::SEBI_FEE;
    ///
    /// // ₹1,52,500 of turnover at ₹10 per crore is 15.25 paisa: 15 to the paisa.
    /// let fee = SEBI_FEE.rate().apply(Paisa::from_raw(152_500_00), 1, Quantum::Paisa)?;
    /// assert_eq!(fee.raw(), 15);
    /// # Ok::<(), costs::error::CostError>(())
    /// ```
    pub fn apply(self, value: Paisa, divisor: i64, quantum: Quantum) -> Result<Paisa, CostError> {
        if divisor <= 0 {
            return Err(CostError::NotPositive {
                quantity: "rate divisor",
                value: divisor,
            });
        }
        // Two i64 factors cannot leave i128 (their product is below 2^126);
        // the third can, and is refused rather than wrapped.
        let numerator = i128::from(value.raw()) * i128::from(self.num);
        let denominator = (i128::from(self.den) * i128::from(divisor))
            .checked_mul(i128::from(quantum.paisa()))
            .ok_or(CostError::Overflow {
                operation: "the denominator of an exact rate",
            })?;
        let units = round_half_up(numerator, denominator)?;
        // `units` is at most |numerator| / quantum + 1, so scaling it back by
        // the quantum stays below 2^126 + 100: inside i128. The narrowing to
        // i64 is the one step that can refuse.
        narrow(units * i128::from(quantum.paisa()), "an exact-rate charge")
    }
}

impl fmt::Display for Ratio {
    /// `numerator/denominator`, exactly as stored. A rate is shown as the
    /// fraction it is, never as a decimal that might have been rounded.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

/// `numerator / D` rounded half-up, for a divisor this crate fixes at compile
/// time.
///
/// Infallible, where [`round_half_up`] is not: every `D` this is instantiated
/// with is a positive named constant, so the division is total, the doubled
/// remainder (below `D`) cannot leave `i128`, and `quotient + 1` is taken only
/// when `D ≥ 2`. A refusal arm here would be one no input could reach.
#[must_use]
pub(crate) const fn half_up<const D: i128>(numerator: i128) -> i128 {
    let quotient = numerator.div_euclid(D);
    if numerator.rem_euclid(D) * 2 >= D {
        quotient + 1
    } else {
        quotient
    }
}

// ---------------------------------------------------------------------------
// Unrounded lines
// ---------------------------------------------------------------------------

/// Units per paisa of an unrounded line: one unit is 10⁻⁷ paisa, 10⁻⁹ rupee.
pub const UNITS_PER_PAISA: i128 = 10_000_000;

/// One charge line **before** any rounding, as an integer count of
/// [`UNITS_PER_PAISA`]-ths of a paisa (10⁻⁹ rupee).
///
/// Every rate either mode ships is a whole number of units per paisa of the
/// value it is levied on (`costs::exact::every_shipped_rate_is_a_whole_number_of_units_per_paisa`),
/// so every line levied straight on a paisa value is exact. A line levied on
/// another line — 18% GST on an unrounded base — can fall between two units
/// and is half-up to the unit; that is the only step below the paisa.
///
/// The field is crate-visible because only this crate's own arithmetic may
/// claim a figure is a charge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Exact(pub(crate) i128);

impl Exact {
    /// Nothing.
    pub const ZERO: Self = Self(0);

    /// A whole paisa amount, exactly. An `i64` of paisa times 10⁷ is below
    /// 2⁸⁷, so this cannot leave `i128`.
    #[must_use]
    pub fn of_paisa(amount: Paisa) -> Self {
        Self(i128::from(amount.raw()) * UNITS_PER_PAISA)
    }

    /// The line in units of 10⁻⁷ paisa.
    #[must_use]
    pub const fn units(self) -> i128 {
        self.0
    }

    /// The line as a money line is shown: half-up to the paisa.
    ///
    /// # Errors
    ///
    /// [`ChargeError::Cost`] carrying an overflow when it leaves `i64`.
    pub fn to_paisa(self) -> Result<Paisa, ChargeError> {
        Ok(narrow(
            half_up::<UNITS_PER_PAISA>(self.0),
            "a calculator line",
        )?)
    }

    /// The line half-up to the rupee, returned in paisa — how Dhan's page
    /// shows STT and stamp duty.
    ///
    /// # Errors
    ///
    /// [`ChargeError::Cost`] carrying an overflow when it leaves `i64`.
    pub fn to_whole_rupee(self) -> Result<Paisa, ChargeError> {
        // At most i128::MAX / 10^9 rupees, so the paisa are inside i128.
        let rupees = half_up::<{ UNITS_PER_PAISA * 100 }>(self.0);
        Ok(narrow(rupees * 100, "a whole-rupee calculator line")?)
    }

    /// The sum of a fixed handful of lines, refused rather than wrapped. The
    /// slice is always a literal, so the walk is bounded at the call site.
    ///
    /// # Errors
    ///
    /// [`ChargeError::Cost`] carrying an overflow naming `operation`.
    pub(crate) fn sum(parts: &[Self], operation: &'static str) -> Result<Self, ChargeError> {
        let mut total = 0_i128;
        for part in parts {
            total = total
                .checked_add(part.0)
                .ok_or(CostError::Overflow { operation })?;
        }
        Ok(Self(total))
    }

    /// `self × rate`, half-up to the unit: GST on an unrounded base.
    ///
    /// # Errors
    ///
    /// [`ChargeError::Cost`] carrying an overflow.
    pub(crate) fn times(self, rate: Ratio) -> Result<Self, ChargeError> {
        let numerator = self
            .0
            .checked_mul(i128::from(rate.num))
            .ok_or(CostError::Overflow {
                operation: "an unrounded line times a rate",
            })?;
        Ok(Self(round_half_up(numerator, i128::from(rate.den))?))
    }
}

/// `value × rate ÷ divisor` in [`Exact`] units: exact wherever the rate is a
/// whole number of units per paisa (every shipped rate is), otherwise half-up
/// to the unit — never to the paisa, which is the line's own rounding and is
/// taken by its caller.
///
/// # Errors
///
/// [`ChargeError::Cost`] carrying [`CostError::NotPositive`] for a divisor
/// that is not positive, or an overflow of the product.
pub(crate) fn units_of(value: Paisa, rate: Ratio, divisor: i64) -> Result<Exact, ChargeError> {
    if divisor <= 0 {
        return Err(CostError::NotPositive {
            quantity: "rate divisor",
            value: divisor,
        }
        .into());
    }
    let overflow = CostError::Overflow {
        operation: "an unrounded charge",
    };
    // value × num cannot leave i128 (two i64 factors); × 10^7 can.
    let numerator = (i128::from(value.raw()) * i128::from(rate.num))
        .checked_mul(UNITS_PER_PAISA)
        .ok_or(overflow)?;
    // Two positive i64 factors cannot leave i128.
    let denominator = i128::from(rate.den) * i128::from(divisor);
    Ok(Exact(round_half_up(numerator, denominator)?))
}

/// A paisa amount rendered as rupees with two decimals: `₹73.90`, `-₹0.05`.
///
/// Integer arithmetic only — a division and a remainder on the magnitude.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rupees(pub Paisa);

impl fmt::Display for Rupees {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let raw = self.0.raw();
        let sign = if raw < 0 { "-" } else { "" };
        let magnitude = raw.unsigned_abs();
        write!(f, "{sign}₹{}.{:02}", magnitude / 100, magnitude % 100)
    }
}

/// `numerator / denominator` rounded half-up, for a **positive** denominator.
///
/// The quotient is floored with `div_euclid` and the non-negative remainder
/// compared against half the denominator, by doubling the remainder rather
/// than halving the denominator — so an odd denominator is exact. "Half-up"
/// is toward `+∞`: `-2.5` rounds to `-2`, `2.5` to `3`.
///
/// # Errors
///
/// [`CostError::NotPositive`] for a denominator that is not positive.
///
/// # Examples
///
/// ```
/// use costs::exact::round_half_up;
///
/// assert_eq!(round_half_up(5, 2)?, 3);   // 2.5 → 3
/// assert_eq!(round_half_up(4, 2)?, 2);   // exact
/// assert_eq!(round_half_up(7, 3)?, 2);   // 2.33 → 2
/// assert_eq!(round_half_up(-5, 2)?, -2); // -2.5 → -2, toward +∞
/// # Ok::<(), costs::error::CostError>(())
/// ```
pub fn round_half_up(numerator: i128, denominator: i128) -> Result<i128, CostError> {
    if denominator <= 0 {
        return Err(CostError::NotPositive {
            quantity: "rounding denominator",
            value: i64::try_from(denominator).unwrap_or(i64::MIN),
        });
    }
    let quotient = numerator.div_euclid(denominator);
    let remainder = numerator.rem_euclid(denominator);
    // `remainder < denominator`, so doubling it cannot pass `2 × i128::MAX`;
    // it is checked anyway, because "cannot" is a claim and this is a refusal.
    let doubled = remainder.checked_mul(2).ok_or(CostError::Overflow {
        operation: "rounding half-up",
    })?;
    if doubled >= denominator {
        Ok(quotient + 1)
    } else {
        Ok(quotient)
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::inconsistent_digit_grouping
)]
mod tests {
    use super::*;

    fn paisa(raw: i64) -> Paisa {
        Paisa::from_raw(raw)
    }

    #[test]
    fn half_up_rounds_a_half_up_and_less_than_a_half_down_on_both_signs() {
        let r = |n: i128, d: i128| round_half_up(n, d).expect("positive denominator");
        // Exact quotients are fixed points.
        assert_eq!(r(0, 7), 0);
        assert_eq!(r(14, 7), 2);
        assert_eq!(r(-14, 7), -2);
        // Below a half goes down, a half and above goes up.
        assert_eq!(r(1, 3), 0);
        assert_eq!(r(2, 3), 1);
        assert_eq!(r(1, 2), 1);
        assert_eq!(r(49, 100), 0);
        assert_eq!(r(50, 100), 1);
        assert_eq!(r(51, 100), 1);
        assert_eq!(r(149, 100), 1);
        assert_eq!(r(150, 100), 2);
        // Toward +infinity for a negative half.
        assert_eq!(r(-1, 2), 0);
        assert_eq!(r(-3, 2), -1);
        assert_eq!(r(-51, 100), -1);
        assert_eq!(r(-50, 100), 0);
        assert_eq!(r(-49, 100), 0);
    }

    #[test]
    fn half_up_is_the_nearest_integer_with_halves_going_up_densely() {
        // The defining property over every numerator in a window and several
        // denominators, odd and even: |n - r·d| ≤ d/2, and a tie goes up.
        let mut checked = 0_u32;
        for d in [1_i128, 2, 3, 7, 10, 100, 10_000_000] {
            for n in -300_i128..=300 {
                let got = round_half_up(n, d).expect("positive");
                let gap = n - got * d; // in [-d/2, d/2): a tie lands on -d/2
                assert!(2 * gap < d, "{n}/{d} -> {got}: rounded down too far");
                assert!(2 * gap >= -d, "{n}/{d} -> {got}: rounded up too far");
                checked += 1;
            }
        }
        assert_eq!(checked, 7 * 601);
    }

    #[test]
    fn a_denominator_that_is_not_positive_is_refused_by_name() {
        assert_eq!(
            round_half_up(5, 0),
            Err(CostError::NotPositive {
                quantity: "rounding denominator",
                value: 0
            })
        );
        assert_eq!(
            round_half_up(5, -3),
            Err(CostError::NotPositive {
                quantity: "rounding denominator",
                value: -3
            })
        );
        // A denominator too wide for the i64 the refusal reports is named at
        // the i64 floor rather than wrapped into a plausible-looking number.
        assert_eq!(
            round_half_up(5, i128::MIN),
            Err(CostError::NotPositive {
                quantity: "rounding denominator",
                value: i64::MIN
            })
        );
        assert_eq!(
            Ratio::new(1, 1).apply(paisa(10), 0, Quantum::Paisa),
            Err(CostError::NotPositive {
                quantity: "rate divisor",
                value: 0
            })
        );
        assert_eq!(
            Ratio::new(1, 1).apply(paisa(10), -2, Quantum::Paisa),
            Err(CostError::NotPositive {
                quantity: "rate divisor",
                value: -2
            })
        );
        // A ratio over nothing never reaches a division: the rounding refuses it.
        assert_eq!(
            Ratio::new(1, 0).apply(paisa(10), 1, Quantum::Paisa),
            Err(CostError::NotPositive {
                quantity: "rounding denominator",
                value: 0
            })
        );
    }

    #[test]
    fn half_up_handles_the_widest_operands_without_wrapping() {
        // The largest positive remainder doubles past i128::MAX: refused.
        assert_eq!(
            round_half_up(i128::MAX - 1, i128::MAX),
            Err(CostError::Overflow {
                operation: "rounding half-up"
            })
        );
        // A remainder below the doubling edge still rounds.
        assert_eq!(round_half_up(i128::MAX / 2 - 1, i128::MAX), Ok(0));
        assert_eq!(round_half_up(i128::MAX, 1), Ok(i128::MAX));
    }

    #[test]
    fn a_ratio_applies_exactly_and_rounds_once_at_the_end() {
        // ₹306.99 + ₹0.01 per crore on ₹1,52,500: 4.68175 rupees → 468 paisa.
        let exchange = Ratio::new(30_700, 1_000_000_000);
        assert_eq!(
            exchange.apply(paisa(152_500_00), 1, Quantum::Paisa),
            Ok(paisa(468))
        );
        // The two components rounded separately would give 468 too here, and
        // 308.52 + 0.01 where the combined line is 308.54 (V4) — the order of
        // rounding is a fact, tested in crate::stack.
        assert_eq!(
            exchange.apply(paisa(10_050_000_00), 1, Quantum::Paisa),
            Ok(paisa(30_854))
        );
        // To the rupee, with the halving divisor: 0.025% of half of
        // ₹20,10,000 is the V2 transaction tax, ₹251.25, which rounds to ₹251.
        let stt = Ratio::new(25, 100_000);
        assert_eq!(
            stt.apply(paisa(2_010_000_00), 2, Quantum::Rupee),
            Ok(paisa(251_00))
        );
        // And a half at the rupee goes up: ₹152.50 → ₹153.
        let delivery = Ratio::new(1, 1_000);
        assert_eq!(
            delivery.apply(paisa(152_500_00), 1, Quantum::Rupee),
            Ok(paisa(153_00))
        );
        // A zero rate charges an exact zero, never a rounding artefact.
        assert_eq!(
            Ratio::new(0, 1).apply(paisa(i64::MAX), 1, Quantum::Rupee),
            Ok(Paisa::ZERO)
        );
    }

    #[test]
    fn every_overflow_site_in_apply_is_refused_by_name() {
        // A denominator × divisor that fits i128 but whose product with the
        // rupee quantum does not.
        assert_eq!(
            Ratio::new(1, i64::MAX).apply(paisa(1), i64::MAX, Quantum::Rupee),
            Err(CostError::Overflow {
                operation: "the denominator of an exact rate"
            })
        );
        assert_eq!(
            Ratio::new(1, i64::MAX).apply(paisa(1), i64::MAX / 40, Quantum::Rupee),
            Err(CostError::Overflow {
                operation: "the denominator of an exact rate"
            })
        );
        // One step below that edge still computes: the refusal is the edge.
        assert_eq!(
            Ratio::new(1, i64::MAX).apply(paisa(1), i64::MAX / 100, Quantum::Rupee),
            Ok(Paisa::ZERO)
        );
        // At the paisa quantum the same widest denominator is in range.
        assert_eq!(
            Ratio::new(1, i64::MAX).apply(paisa(1), i64::MAX, Quantum::Paisa),
            Ok(Paisa::ZERO)
        );
        // A rounded charge whose paisa leave i64.
        assert_eq!(
            Ratio::new(i64::MAX, 1).apply(paisa(i64::MAX), 1, Quantum::Paisa),
            Err(CostError::Overflow {
                operation: "an exact-rate charge"
            })
        );
        // A rounded rupee count whose paisa leave i128 cannot be built from
        // i64 inputs (2^126 rupees × 100 < 2^127); the guard is reached with a
        // whole-rupee charge whose paisa then leave i64 instead.
        assert_eq!(
            Ratio::new(i64::MAX, 100).apply(paisa(i64::MAX), 1, Quantum::Rupee),
            Err(CostError::Overflow {
                operation: "an exact-rate charge"
            })
        );
    }

    #[test]
    fn a_ratio_reads_a_bps_x100_rate_as_the_fraction_it_encodes() {
        let sebi = Ratio::from_bps_x100(crate::rate::SEBI_TURNOVER_FEE);
        assert_eq!(sebi.numerator(), 10);
        assert_eq!(sebi.denominator(), 10_000_000);
        assert_eq!(sebi.to_string(), "10/10000000");
        assert!(sebi.is_well_shaped());
        assert!(!Ratio::new(1, 0).is_well_shaped());
        assert!(!Ratio::new(1, -1).is_well_shaped());
        assert!(!Ratio::new(-1, 1).is_well_shaped());
        assert!(Ratio::new(0, 1).is_well_shaped());
        assert_eq!(Quantum::Paisa.paisa(), 1);
        assert_eq!(Quantum::Rupee.paisa(), 100);
        assert_eq!(format!("{:?}", Quantum::Rupee), "Rupee");
    }

    #[test]
    fn rupees_render_with_two_decimals_and_a_sign_on_both_sides_of_zero() {
        let show = |raw: i64| Rupees(paisa(raw)).to_string();
        assert_eq!(show(0), "₹0.00");
        assert_eq!(show(5), "₹0.05");
        assert_eq!(show(7_390), "₹73.90");
        assert_eq!(show(182_914), "₹1829.14");
        assert_eq!(show(-5), "-₹0.05");
        assert_eq!(show(-1_234), "-₹12.34");
        assert_eq!(show(i64::MIN), "-₹92233720368547758.08");
        assert_eq!(show(i64::MAX), "₹92233720368547758.07");
    }

    #[test]
    fn every_shipped_rate_is_a_whole_number_of_units_per_paisa() {
        use crate::schedule::{
            BSE_EQUITY_EXCHANGE, BSE_OPTIONS_EXCHANGE, DHAN_EQUITY_BROKERAGE_RATE, GST,
            NSE_EQUITY_EXCHANGE, NSE_FUTURES_EXCHANGE, NSE_OPTIONS_EXCHANGE, SEBI_FEE,
            STAMP_EQUITY_DELIVERY, STAMP_EQUITY_NON_DELIVERY, STAMP_FUTURES, STAMP_OPTIONS,
            STT_EQUITY_DELIVERY, STT_EQUITY_NON_DELIVERY, STT_EXERCISE, STT_FUTURES, STT_OPTIONS,
        };
        // So a line levied on a paisa value is exact in units: the claim
        // `Exact` makes, held for every rate today's table carries.
        let whole = |rate: Ratio, divisor: i128| {
            i128::from(rate.numerator()) * UNITS_PER_PAISA
                % (i128::from(rate.denominator()) * divisor)
                == 0
        };
        for cited in [
            NSE_EQUITY_EXCHANGE,
            NSE_FUTURES_EXCHANGE,
            NSE_OPTIONS_EXCHANGE,
            BSE_EQUITY_EXCHANGE,
            BSE_OPTIONS_EXCHANGE,
            SEBI_FEE,
            GST,
            STT_EQUITY_NON_DELIVERY,
            STT_EQUITY_DELIVERY,
            STT_FUTURES,
            STT_OPTIONS,
            STT_EXERCISE,
            STAMP_EQUITY_NON_DELIVERY,
            STAMP_EQUITY_DELIVERY,
            STAMP_FUTURES,
            STAMP_OPTIONS,
            DHAN_EQUITY_BROKERAGE_RATE,
        ] {
            assert!(whole(cited.rate(), 1), "{}", cited.rate());
        }
        // An order's share of the intraday STT is half its value at the rate.
        assert!(whole(STT_EQUITY_NON_DELIVERY.rate(), 2));
        // A dated rate is basis points over RATE_SCALE, which is the unit.
        assert_eq!(i128::from(RATE_SCALE), UNITS_PER_PAISA);
        // And the check can fail: a third of a paisa is not whole.
        assert!(!whole(Ratio::new(1, 3), 1));
    }

    #[test]
    fn an_unrounded_line_is_exact_and_refuses_what_it_cannot_hold() {
        // V1's turnover at FA73061's cash rate, exactly: ₹4.68175.
        assert_eq!(
            units_of(paisa(15_250_000), Ratio::new(30_700, 1_000_000_000), 1),
            Ok(Exact(4_681_750_000))
        );
        // Where a rate is not whole, half-up to the unit, never to the paisa.
        assert_eq!(
            units_of(paisa(1), Ratio::new(1, 3), 1),
            Ok(Exact(3_333_333))
        );
        assert_eq!(
            units_of(paisa(2), Ratio::new(1, 3), 1),
            Ok(Exact(6_666_667))
        );
        assert_eq!(
            units_of(paisa(1), Ratio::new(1, 4), 2),
            Ok(Exact(1_250_000))
        );
        let refused = |cost| Err(ChargeError::Cost(cost));
        assert_eq!(
            units_of(paisa(1), Ratio::new(1, 1), 0),
            refused(CostError::NotPositive {
                quantity: "rate divisor",
                value: 0
            })
        );
        assert_eq!(
            units_of(paisa(1), Ratio::new(1, 0), 1),
            refused(CostError::NotPositive {
                quantity: "rounding denominator",
                value: 0
            })
        );
        assert_eq!(
            units_of(paisa(i64::MAX), Ratio::new(i64::MAX, 1), 1),
            refused(CostError::Overflow {
                operation: "an unrounded charge"
            })
        );
        // Whole paisa, sums, and a rate on a line.
        assert_eq!(Exact::of_paisa(paisa(-73_90)).units(), -73_900_000_000);
        assert_eq!(Exact::ZERO.units(), 0);
        assert_eq!(Exact::sum(&[Exact(1), Exact(2)], "a sum"), Ok(Exact(3)));
        assert_eq!(
            Exact::sum(&[Exact(i128::MAX), Exact(1)], "a sum"),
            refused(CostError::Overflow { operation: "a sum" })
        );
        // V1's GST base, ₹44.83, at 18%: ₹8.0694 exactly.
        assert_eq!(
            Exact(4_483 * UNITS_PER_PAISA).times(Ratio::new(18, 100)),
            Ok(Exact(8_069_400_000))
        );
        assert_eq!(Exact(5).times(Ratio::new(1, 2)), Ok(Exact(3)), "half-up");
        assert_eq!(
            Exact(i128::MAX).times(Ratio::new(2, 1)),
            refused(CostError::Overflow {
                operation: "an unrounded line times a rate"
            })
        );
        assert_eq!(
            Exact(1).times(Ratio::new(1, 0)),
            refused(CostError::NotPositive {
                quantity: "rounding denominator",
                value: 0
            })
        );
    }
}
