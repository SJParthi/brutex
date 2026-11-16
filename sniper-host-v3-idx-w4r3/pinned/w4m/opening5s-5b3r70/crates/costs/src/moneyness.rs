//! Moneyness: how far a strike sits from the money, counted in strike steps.
//!
//! # The convention, stated once
//!
//! Moneyness is a **signed count of strike steps from the at-the-money rung**,
//! and the sign is read in the trade direction:
//!
//! | Value | Reads as | Renders as |
//! |---|---|---|
//! | negative | **in** the money, that many steps deep | `ITM-2` |
//! | zero | at the money | `ATM` |
//! | positive | **out** of the money, that many steps out | `OTM+5` |
//!
//! That is not a new convention. It is the source's own
//! `strike_rules.py::resolve_offset_strike` law — *"`atm_plus_N_in_dir` always
//! means further OTM in the trade direction"* — with the rule name replaced by
//! the signed integer it already carried (`param: signed integer step count,
//! negative = ITM direction`). The seven locked offset rules are exactly
//! `0, ±1, ±2, ±5` in this encoding, and a test pins all seven.
//!
//! # Two conventions exist in the source, and both are here
//!
//! The source has a **second**, different signed offset:
//! `moneyness.py::atm_offset`, which is `round((strike − spot) / step)` and is
//! **grid-relative** — positive means a higher strike, whichever side is being
//! traded. It is not the same number: for a put the two differ in sign.
//!
//! Rather than pick one and quietly drop the other, both are here and named
//! apart — [`atm_offset`] is the grid-relative one, [`moneyness_steps`] is the
//! direction-relative one — and the arrow between them is a tested law:
//! `moneyness = atm_offset` for a call, `moneyness = −atm_offset` for a put.
//!
//! # Half a step: the tie goes **down**, because the rung's tie goes up
//!
//! A strike exactly `step / 2` from spot is equidistant from two offsets, and
//! the tie is broken toward the **smaller** one: `+step/2` is `0`, `−step/2`
//! is `−1`.
//!
//! That is not a second convention invented here. It is
//! [`crate::strike::at_the_money`]'s own tie — a spot exactly halfway between
//! two rungs snaps **up**, invariant K-17 — read from the other end. The rung
//! half a step above such a spot *is* the at-the-money rung, so it must read
//! `0`; the rung half a step below it is one rung in, so it must read `−1`.
//! Stated as the identity the two halves owe each other:
//!
//! ```text
//! atm_offset(at_the_money(spot) + k·step, spot)  ==  k     for every spot
//! ```
//!
//! proven over every paisa of a whole step of spots, on both grids, by
//! `the_offset_counts_rungs_from_the_rung_at_the_money_names`.
//!
//! **What this replaces, and why it was wrong.** The rounding used to be half
//! *toward zero*, which breaks the tie the opposite way below spot. On the
//! 50-rupee grid with a spot of exactly ₹24,025.00 — a legal spot, on the grid
//! of paisa the store keeps — that rule gave `0` to **both** ₹24,000 and
//! ₹24,050, while `at_the_money` named only ₹24,050. One spot, two strikes
//! wearing the `ATM` label, and a label that no longer resolves back to the
//! strike it was read off: `strike_at(at_the_money(spot), ATM)` answered
//! ₹24,050 for a strike of ₹24,000. Every strike below the midpoint was
//! mislabelled by one step, not just the tie itself.
//!
//! # The at-the-money band is half open, and that is what it cost
//!
//! ```text
//! moneyness_steps(...) == 0   ⟺   classify(...) == Moneyness::AtTheMoney
//! ```
//!
//! is still an exact equivalence for **every** step, odd or even — `classify`
//! is literally the sign of the moneyness. What changed is the band's shape:
//! it is `(−step/2, +step/2]`, closed above and open below, where the source's
//! `classify` used a band closed at both ends.
//!
//! Both could not be kept. A closed band exactly one step wide contains **two**
//! rungs whenever spot sits at a midpoint, so it cannot coexist with a single
//! at-the-money rung. The rung wins: it decides which contract is bought, where
//! the band only decides a label. The equivalence and the band's exact edges
//! are proven by `the_bucket_is_the_sign_of_the_moneyness_and_the_band_edge_agrees`.
//!
//! # Integers only
//!
//! The source computed the offset over 28-digit decimals and its Rust twin
//! reproduced that with a proof about ulp sizes. None of that crosses over.
//! Every value here is `i64` paisa, the tie test is `2·r` against `d` in
//! `i128` so it cannot overflow, and there is no `f64` on any path.

use brutex_core::instrument::OptionSide;
use brutex_core::price::Paisa;

use crate::error::CostError;
use crate::strike::StrikeStep;

/// Which side of the money a strike is on.
///
/// The source's `NA` (not an option) and `UNKNOWN` (missing or invalid spot or
/// strike) buckets are deliberately absent: this crate refuses those inputs by
/// name through [`CostError`] instead of returning a bucket that means "the
/// question did not apply". A bucket that can mean "no answer" is a bucket a
/// caller can average.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Moneyness {
    /// The option has intrinsic value at this spot.
    InTheMoney,
    /// The strike is the rung [`crate::strike::at_the_money`] names for this
    /// spot: within half a step above it, or less than half a step below it.
    /// The band is half open on purpose — see the module documentation.
    AtTheMoney,
    /// The option has no intrinsic value at this spot.
    OutOfTheMoney,
}

impl Moneyness {
    /// The three-letter tag every Indian screen uses.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InTheMoney => "ITM",
            Self::AtTheMoney => "ATM",
            Self::OutOfTheMoney => "OTM",
        }
    }
}

impl core::fmt::Display for Moneyness {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A signed count of strike steps from the at-the-money rung.
///
/// Negative is into the money, zero is at the money, positive is out of the
/// money — see the module documentation for why that is the source's own
/// convention and not a second one.
///
/// # What bounds this
///
/// The width. `i32` is the boundary bound
/// (`docs/07-o1-architecture.md` law 5: bound every input at the boundary), and
/// beyond it the arithmetic itself refuses: [`crate::strike::strike_at`] walks
/// `moneyness × step` from the rung and returns [`CostError::Overflow`] or
/// [`CostError::NotPositive`] the moment the answer leaves the domain of real
/// prices.
///
/// There is deliberately **no cap at the chain width**. The source's own
/// `strike_offset_from_atm` says in as many words that it "doesn't
/// bounds-check", and inventing a refusal the source does not have would be a
/// behaviour change this port has no authority to make. [`Self::CHAIN_HALF_WIDTH`]
/// and [`Self::is_within_chain`] exist so a caller that *does* want the chain
/// bound can ask for it explicitly, rather than having it applied silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct MoneynessSteps(i32);

impl MoneynessSteps {
    /// The rung nearest spot.
    pub const AT_THE_MONEY: Self = Self(0);

    /// How far either side of the money the source's default chain reaches.
    ///
    /// Ten, from `strike_grid.py::grid_around_atm`'s default `half_width` — the
    /// 21-strike chain `ATM-10 .. ATM .. ATM+10`. It is a **query**, never a
    /// clamp and never a refusal: see the type's documentation.
    pub const CHAIN_HALF_WIDTH: i32 = 10;

    /// A moneyness of `steps` steps.
    ///
    /// Total: every `i32` is a representable moneyness. What is not
    /// representable is a *strike* the arithmetic cannot reach, and that is
    /// refused where the arithmetic happens.
    #[must_use]
    pub const fn new(steps: i32) -> Self {
        Self(steps)
    }

    /// The signed step count.
    #[must_use]
    pub const fn steps(self) -> i32 {
        self.0
    }

    /// Which side of the money this is — the sign, and nothing else.
    #[must_use]
    pub const fn bucket(self) -> Moneyness {
        if self.0 < 0 {
            Moneyness::InTheMoney
        } else if self.0 == 0 {
            Moneyness::AtTheMoney
        } else {
            Moneyness::OutOfTheMoney
        }
    }

    /// How many steps from the money this is, without the direction.
    #[must_use]
    pub const fn depth(self) -> u32 {
        self.0.unsigned_abs()
    }

    /// Whether this lies inside the source's default 21-strike chain.
    ///
    /// A question, asked by a caller who wants the answer. Nothing in this
    /// crate consults it, and nothing refuses on it.
    #[must_use]
    pub const fn is_within_chain(self) -> bool {
        self.depth() <= Self::CHAIN_HALF_WIDTH.unsigned_abs()
    }
}

impl core::fmt::Display for MoneynessSteps {
    /// Renders `ATM`, `ITM-2` or `OTM+5` — the spelling the operator uses.
    ///
    /// The magnitude goes through [`Self::depth`], which is an unsigned
    /// absolute value, so `i32::MIN` renders rather than overflowing on a
    /// negation that has no `i32`.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.bucket() {
            Moneyness::AtTheMoney => f.write_str("ATM"),
            Moneyness::InTheMoney => write!(f, "ITM-{}", self.depth()),
            Moneyness::OutOfTheMoney => write!(f, "OTM+{}", self.depth()),
        }
    }
}

/// How many steps above spot a strike sits, with the half-step tie going down.
///
/// The **grid-relative** offset: positive means a higher strike, whichever side
/// is being traded. This is the source's `moneyness.py::atm_offset` law with
/// its tie rule replaced by the one [`crate::strike::at_the_money`] already
/// uses (see the module documentation), and it is *not* the moneyness — see
/// [`moneyness_steps`], which is this with the trade direction applied.
///
/// # Errors
///
/// [`CostError::NotPositive`] for a strike or a spot that is not strictly
/// positive. [`CostError::Overflow`] if the offset does not fit an `i32`, which
/// needs a strike and a spot at opposite ends of the `i64` domain on a one-paisa
/// grid.
///
/// # Examples
///
/// ```
/// use brutex_core::price::Paisa;
/// use costs::moneyness::atm_offset;
/// # use costs::day::TradeDay;
/// # use costs::venue::swept_slot;
/// # use brutex_core::symbol::Symbol;
/// let step = costs::strike::strike_step_on(
///     swept_slot(Symbol::new("NIFTY")?)?,
///     TradeDay::new(2024, 10, 1)?,
/// )?;
///
/// // 24,050 is one 50-point step above a spot of 24,000.
/// assert_eq!(atm_offset(Paisa::from_raw(24_050_00), Paisa::from_raw(24_000_00), step), Ok(1));
/// // Exactly half a step: the tie goes down, to the smaller offset. Above spot
/// // that is 0, below spot it is -1 — the two ends of one rule, not two rules.
/// assert_eq!(atm_offset(Paisa::from_raw(24_025_00), Paisa::from_raw(24_000_00), step), Ok(0));
/// assert_eq!(atm_offset(Paisa::from_raw(23_975_00), Paisa::from_raw(24_000_00), step), Ok(-1));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn atm_offset(strike: Paisa, spot: Paisa, step: StrikeStep) -> Result<i32, CostError> {
    if strike.raw() <= 0 {
        return Err(CostError::NotPositive {
            quantity: "strike",
            value: strike.raw(),
        });
    }
    if spot.raw() <= 0 {
        return Err(CostError::NotPositive {
            quantity: "spot",
            value: spot.raw(),
        });
    }
    // Both positive, so the difference cannot overflow i64; i128 anyway so the
    // doubling below is free of a second guard.
    let numerator = i128::from(strike.raw()) - i128::from(spot.raw());
    let divisor = i128::from(step.raw());
    // Euclidean division, not truncating division. The divisor is a strike step
    // and is always strictly positive, so the remainder is the distance ABOVE
    // the rung below and satisfies `0 <= r < d` for **either** sign of the
    // numerator — where `%` would hand back a negative remainder below spot and
    // need a second, sign-dependent arm to interpret it.
    let quotient = numerator.div_euclid(divisor);
    let remainder = numerator.rem_euclid(divisor);
    // One comparison decides the tie for both directions at once: step up only
    // when strictly more than half a step remains above the rung below, so the
    // exact half — `2·r == d` — stays on the lower offset.
    //
    // That `>` is load-bearing, and unlike the `remainder > 0` branch this
    // replaces (docs/06-limits.md §26) the mutant that weakens it to `>=` is
    // caught rather than equivalent: both shipped grids are an even number of
    // paisa, so `2·r == d` is reachable, and it is reached by
    // `a_half_step_tie_goes_down_to_the_rung_at_the_money_names` on the NIFTY
    // grid and by `the_offset_counts_rungs_from_the_rung_at_the_money_names`
    // on both. The rule itself is `strike::at_the_money`'s own tie seen from
    // the other end — see the module documentation — and nothing here
    // re-decides it.
    let offset = if 2 * remainder > divisor {
        quotient + 1
    } else {
        quotient
    };
    i32::try_from(offset).map_err(|_| CostError::Overflow {
        operation: "strike offset in steps",
    })
}

/// How many steps from the money a strike sits, in the trade direction.
///
/// The moneyness: negative into the money, zero at the money, positive out of
/// the money. It is [`atm_offset`] with the side's sign applied — a higher
/// strike is further out of the money for a call and further into it for a put.
///
/// # Errors
///
/// As [`atm_offset`], plus [`CostError::Overflow`] when the offset is
/// `i32::MIN`, whose negation has no `i32`. That is refused rather than
/// wrapped: a wrapped moneyness would flip a deep in-the-money put to a deep
/// out-of-the-money one.
///
/// # Examples
///
/// ```
/// use brutex_core::instrument::OptionSide;
/// use brutex_core::price::Paisa;
/// use costs::moneyness::moneyness_steps;
/// # use costs::day::TradeDay;
/// # use costs::venue::swept_slot;
/// # use brutex_core::symbol::Symbol;
/// let step = costs::strike::strike_step_on(
///     swept_slot(Symbol::new("NIFTY")?)?,
///     TradeDay::new(2024, 10, 1)?,
/// )?;
/// let (strike, spot) = (Paisa::from_raw(24_100_00), Paisa::from_raw(24_000_00));
///
/// // A 24,100 call with spot at 24,000 is two steps out of the money.
/// let call = moneyness_steps(strike, spot, step, OptionSide::Call)?;
/// assert_eq!(call.to_string(), "OTM+2");
///
/// // The same strike as a put is two steps into the money.
/// let put = moneyness_steps(strike, spot, step, OptionSide::Put)?;
/// assert_eq!(put.to_string(), "ITM-2");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn moneyness_steps(
    strike: Paisa,
    spot: Paisa,
    step: StrikeStep,
    side: OptionSide,
) -> Result<MoneynessSteps, CostError> {
    let offset = atm_offset(strike, spot, step)?;
    let steps = match side {
        // A call gets further out of the money as the strike rises.
        OptionSide::Call => offset,
        // A put gets further out of the money as the strike falls.
        OptionSide::Put => offset.checked_neg().ok_or(CostError::Overflow {
            operation: "moneyness sign inversion",
        })?,
    };
    Ok(MoneynessSteps::new(steps))
}

/// Which side of the money a strike is on, at a given spot.
///
/// The source's `moneyness.py::classify` law with the at-the-money band fixed
/// at half a step, **half open**: `(−step/2, +step/2]` around spot, closed
/// above and open below. The source's band was closed at both ends, which
/// admits two rungs at a midpoint spot and therefore cannot coexist with a
/// single at-the-money rung — see the module documentation for which of the two
/// yielded and why.
///
/// It is exactly `moneyness_steps(..).bucket()`, and the test
/// `the_bucket_is_the_sign_of_the_moneyness_and_the_band_edge_agrees` proves
/// that against an independent re-derivation of the rule (`diff = spot −
/// strike`; `−step ≤ 2·diff < step` is at the money; a call is in the money
/// when `diff > 0`, a put when `diff < 0`).
///
/// # Errors
///
/// As [`moneyness_steps`].
///
/// # Examples
///
/// ```
/// use brutex_core::instrument::OptionSide;
/// use brutex_core::price::Paisa;
/// use costs::moneyness::{classify, Moneyness};
/// # use costs::day::TradeDay;
/// # use costs::venue::swept_slot;
/// # use brutex_core::symbol::Symbol;
/// let step = costs::strike::strike_step_on(
///     swept_slot(Symbol::new("NIFTY")?)?,
///     TradeDay::new(2024, 10, 1)?,
/// )?;
/// let spot = Paisa::from_raw(24_100_00);
///
/// // Spot above the strike: the call has intrinsic value, the put does not.
/// let strike = Paisa::from_raw(24_000_00);
/// assert_eq!(classify(strike, spot, step, OptionSide::Call)?, Moneyness::InTheMoney);
/// assert_eq!(classify(strike, spot, step, OptionSide::Put)?, Moneyness::OutOfTheMoney);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn classify(
    strike: Paisa,
    spot: Paisa,
    step: StrikeStep,
    side: OptionSide,
) -> Result<Moneyness, CostError> {
    moneyness_steps(strike, spot, step, side).map(MoneynessSteps::bucket)
}

#[cfg(test)]
// A money literal is written `rupees_paisa` — `24_012_00` reads as the twelve
// rupees over twenty-four thousand a circular or a screen would show, where
// `2_401_200` reads as nothing at all. The grouping is deliberate.
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::inconsistent_digit_grouping
)]
mod tests {
    use super::*;

    use crate::strike::{at_the_money, strike_at};

    /// The NIFTY grid, in paisa: 50 rupees a step.
    fn nifty_step() -> StrikeStep {
        step_of("NIFTY")
    }

    /// The BANKNIFTY grid, in paisa: 100 rupees a step.
    fn banknifty_step() -> StrikeStep {
        step_of("BANKNIFTY")
    }

    fn step_of(underlying: &str) -> StrikeStep {
        crate::strike::strike_step_on(
            crate::venue::swept_slot(
                brutex_core::symbol::Symbol::new(underlying).expect("a valid symbol"),
            )
            .expect("a swept underlying"),
            crate::day::TradeDay::new(2024, 10, 1).expect("a real date"),
        )
        .expect("inside the verified window")
    }

    fn paisa(rupees: i64) -> Paisa {
        Paisa::from_raw(rupees * 100)
    }

    #[test]
    fn the_source_offset_pins_land_on_the_source_answers() {
        // The pins the predecessor hardcoded from its decimal oracle, in paisa,
        // MINUS the two that are exact ties below spot. Those two are the only
        // rows where this crate's tie rule and the predecessor's differ, they
        // are now answered in `a_half_step_tie_goes_down_to_the_rung_at_the_money_names`
        // beside the reason, and leaving them here under a name that claims
        // they are the source's answers would be the claim that is false.
        let step = nifty_step();
        for (strike, spot, want) in [
            (2_405_000i64, 2_400_000i64, 1i32), // one step up
            (2_402_500, 2_400_000, 0),          // exact +half tie -> 0, unchanged
            (2_407_505, 2_400_000, 2),          // 1.501 steps -> 2
            (2_407_495, 2_400_000, 1),          // 1.499 steps -> 1
            (2_392_495, 2_400_000, -2),         // -1.501 -> -2
            (5_000_000, 2_400_000, 520),        // far out of the money
        ] {
            assert_eq!(
                atm_offset(Paisa::from_raw(strike), Paisa::from_raw(spot), step),
                Ok(want),
                "atm_offset({strike}, {spot})"
            );
        }
        // A one-paisa grid, where every paisa is a step.
        let unit = StrikeStep::new_const(1);
        assert_eq!(
            atm_offset(Paisa::from_raw(10_001), Paisa::from_raw(10_000), unit),
            Ok(1)
        );
        assert_eq!(
            atm_offset(
                Paisa::from_raw(5),
                Paisa::from_raw(10),
                StrikeStep::new_const(5)
            ),
            Ok(-1)
        );
    }

    #[test]
    fn a_half_step_tie_goes_down_to_the_rung_at_the_money_names() {
        let step = nifty_step();
        let spot = paisa(24_000);
        // (k + 1/2) steps keep k; -(k + 1/2) steps take -(k + 1). Both are the
        // same rule — the tie goes to the SMALLER offset — and the second half
        // is what half-toward-zero used to get wrong.
        assert_eq!(atm_offset(Paisa::from_raw(2_407_500), spot, step), Ok(1));
        assert_eq!(atm_offset(Paisa::from_raw(2_392_500), spot, step), Ok(-2));
        assert_eq!(atm_offset(Paisa::from_raw(2_407_501), spot, step), Ok(2));
        assert_eq!(atm_offset(Paisa::from_raw(2_392_499), spot, step), Ok(-2));
        // The band edges themselves, one paisa either side of each. The upper
        // edge is IN the band and the lower edge is not: `(-step/2, +step/2]`.
        assert_eq!(atm_offset(Paisa::from_raw(2_402_500), spot, step), Ok(0));
        assert_eq!(atm_offset(Paisa::from_raw(2_402_501), spot, step), Ok(1));
        assert_eq!(atm_offset(Paisa::from_raw(2_397_500), spot, step), Ok(-1));
        assert_eq!(atm_offset(Paisa::from_raw(2_397_501), spot, step), Ok(0));
        assert_eq!(atm_offset(Paisa::from_raw(2_397_499), spot, step), Ok(-1));

        // And the tie seen from the spot's end, which is the whole reason it
        // goes this way: a spot exactly halfway between two rungs snaps UP
        // (K-17), so the rung above it is the at-the-money one and reads 0,
        // and the rung below it reads -1. Half-toward-zero gave 0 to BOTH,
        // and this is the worked case the module documentation names.
        let midpoint = Paisa::from_raw(2_402_500);
        assert_eq!(at_the_money(midpoint, step), Ok(paisa(24_050)));
        assert_eq!(atm_offset(paisa(24_050), midpoint, step), Ok(0));
        assert_eq!(atm_offset(paisa(24_000), midpoint, step), Ok(-1));
        // Exactly one rung on the whole grid wears the ATM label, and it is the
        // one `at_the_money` named. Two of them was the defect.
        assert_eq!(
            (23_900i64..=24_150)
                .step_by(50)
                .filter(|&rupees| atm_offset(paisa(rupees), midpoint, step) == Ok(0))
                .collect::<Vec<i64>>(),
            vec![24_050]
        );
    }

    #[test]
    fn the_offset_matches_a_nearest_multiple_search_across_four_whole_steps() {
        // A differential against an independently written search: minimise
        // |n − k·d| over candidate k, ties preferring the smaller k (down, the
        // rule the module documentation derives from `at_the_money`'s snap).
        // Exact i128 distances, no rounding anywhere in the reference.
        let step = nifty_step();
        let divisor = i128::from(step.raw());
        let spot = paisa(24_000);
        let mut checked = 0u32;
        for strike in (spot.raw() - 2 * step.raw())..=(spot.raw() + 2 * step.raw()) {
            let got =
                i128::from(atm_offset(Paisa::from_raw(strike), spot, step).expect("both positive"));
            let numerator = i128::from(strike) - i128::from(spot.raw());
            let floor = numerator.div_euclid(divisor);
            let mut best = floor;
            for candidate in (floor - 1)..=(floor + 2) {
                let here = (numerator - candidate * divisor).abs();
                let there = (numerator - best * divisor).abs();
                if here < there || (here == there && candidate < best) {
                    best = candidate;
                }
            }
            assert_eq!(got, best, "strike {strike}");
            checked += 1;
        }
        assert_eq!(checked, 4 * 5_000 + 1);
    }

    #[test]
    fn the_direction_flips_the_sign_for_a_put_and_never_for_a_call() {
        let step = nifty_step();
        let spot = paisa(24_000);
        for rupees in [23_750i64, 23_900, 23_950, 24_000, 24_050, 24_100, 24_250] {
            let strike = paisa(rupees);
            let offset = atm_offset(strike, spot, step).expect("positive");
            assert_eq!(
                moneyness_steps(strike, spot, step, OptionSide::Call),
                Ok(MoneynessSteps::new(offset)),
                "a call reads the grid offset unchanged"
            );
            assert_eq!(
                moneyness_steps(strike, spot, step, OptionSide::Put),
                Ok(MoneynessSteps::new(-offset)),
                "a put reads it inverted"
            );
        }
    }

    #[test]
    fn the_bucket_is_the_sign_of_the_moneyness_and_the_band_edge_agrees() {
        // The equivalence the tie rule exists for, checked against an
        // INDEPENDENT re-derivation of the classify rule over every paisa of
        // two whole steps, on both grids and both sides.
        for step in [nifty_step(), banknifty_step()] {
            let spot = paisa(24_000);
            let width = step.raw();
            let mut at_the_money_seen = 0u32;
            for strike in (spot.raw() - width)..=(spot.raw() + width) {
                for side in [OptionSide::Call, OptionSide::Put] {
                    let strike = Paisa::from_raw(strike);
                    let moneyness = moneyness_steps(strike, spot, step, side).expect("positive");
                    let bucket = classify(strike, spot, step, side).expect("positive");
                    assert_eq!(bucket, moneyness.bucket(), "the bucket is the sign");
                    // The rule, written out again from scratch. The band is
                    // HALF OPEN — `(-step/2, +step/2]` measured on the strike,
                    // which is `-step <= 2·diff < step` measured on `diff` —
                    // because a closed band holds two rungs at a midpoint spot
                    // and the at-the-money rung is one strike, not two.
                    let diff = spot.raw() - strike.raw();
                    let band_edge = -width <= 2 * diff && 2 * diff < width;
                    let want = if band_edge {
                        Moneyness::AtTheMoney
                    } else if matches!(
                        (side, diff > 0),
                        (OptionSide::Call, true) | (OptionSide::Put, false)
                    ) {
                        Moneyness::InTheMoney
                    } else {
                        Moneyness::OutOfTheMoney
                    };
                    assert_eq!(bucket, want, "strike {strike:?} as a {side:?}");
                    assert_eq!(
                        moneyness.steps() == 0,
                        band_edge,
                        "zero steps must be exactly the at-the-money band"
                    );
                    if band_edge {
                        at_the_money_seen += 1;
                    }
                }
            }
            // Half a step either side, the upper edge included and the lower
            // one excluded, on both sides of the trade: exactly `width` strikes
            // per side rather than `width + 1`. The band is one paisa narrower
            // than the closed one it replaced, and that paisa is the tie.
            assert_eq!(i64::from(at_the_money_seen), 2 * width);
        }
    }

    #[test]
    fn the_moneyness_and_the_strike_are_exact_inverses_of_each_other() {
        // Resolve a strike from a moneyness, then read the moneyness back off
        // THE SPOT IT WAS RESOLVED FOR: the round trip is the identity. This is
        // the law that binds crate::strike and this module together, on both
        // sides and both grids.
        //
        // Reading it back off the rung is the weaker statement this test used
        // to make, and it was weak in exactly the place that was broken: a rung
        // is on the grid, so the half-step tie never arose and the two tie
        // rules could disagree unobserved. The spots below therefore include
        // the exact midpoint between two rungs and one paisa either side of it.
        for step in [nifty_step(), banknifty_step()] {
            // Both shipped grids are an even number of paisa, so the midpoint
            // is exact and this test is not quietly rounding its own fixture.
            assert_eq!(step.raw() % 2, 0, "an even step in paisa");
            let half = step.raw() / 2;
            let rung = paisa(24_000).raw();
            for from_rung in [0, 1, -1, half - 1, half, half + 1] {
                let spot = Paisa::from_raw(rung + from_rung);
                let atm = at_the_money(spot, step).expect("positive");
                for side in [OptionSide::Call, OptionSide::Put] {
                    for steps in -50i32..=50 {
                        let moneyness = MoneynessSteps::new(steps);
                        let strike =
                            strike_at(atm, moneyness, step, side).expect("within the grid");
                        assert_eq!(
                            moneyness_steps(strike, spot, step, side),
                            Ok(moneyness),
                            "{moneyness} as a {side:?} at spot {spot:?} did not survive"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_offset_counts_rungs_from_the_rung_at_the_money_names() {
        // The identity the tie rule exists to make true, over EVERY paisa of a
        // whole step of spots, on both grids:
        //
        //     atm_offset(at_the_money(spot) + k·step, spot) == k
        //
        // An off-grid spot is the normal case — an index level is whatever it
        // is — so this sweeps a whole step of them rather than the handful a
        // pin test can name. Under half-toward-zero every spot strictly above a
        // rung's midpoint failed this at k = -1 and below, which is what made
        // two strikes wear the ATM label for one spot.
        for step in [nifty_step(), banknifty_step()] {
            let base = paisa(24_000).raw();
            let mut checked = 0u32;
            for spot in base..(base + step.raw()) {
                let spot = Paisa::from_raw(spot);
                let atm = at_the_money(spot, step).expect("positive");
                for k in -3i32..=3 {
                    let strike = Paisa::from_raw(atm.raw() + i64::from(k) * step.raw());
                    assert_eq!(
                        atm_offset(strike, spot, step),
                        Ok(k),
                        "rung {k} from {atm:?} at spot {spot:?}"
                    );
                }
                checked += 1;
            }
            assert_eq!(i64::from(checked), step.raw(), "a whole step of spots");
        }
    }

    #[test]
    fn moneyness_at_zero_at_the_chain_edges_and_one_past_each() {
        let step = nifty_step();
        let atm = paisa(24_000);
        assert_eq!(MoneynessSteps::CHAIN_HALF_WIDTH, 10);
        assert_eq!(MoneynessSteps::AT_THE_MONEY.steps(), 0);
        assert!(MoneynessSteps::AT_THE_MONEY.is_within_chain());
        assert_eq!(MoneynessSteps::AT_THE_MONEY.bucket(), Moneyness::AtTheMoney);
        // The documented extremes: the 21-strike chain, ATM-10 .. ATM+10.
        for steps in [-10i32, 10] {
            let moneyness = MoneynessSteps::new(steps);
            assert!(moneyness.is_within_chain(), "{moneyness} is the chain edge");
            assert_eq!(moneyness.depth(), 10);
        }
        // One past each. They are OUTSIDE the chain and still perfectly
        // resolvable — the chain is a query, not a refusal.
        for steps in [-11i32, 11] {
            let moneyness = MoneynessSteps::new(steps);
            assert!(
                !moneyness.is_within_chain(),
                "{moneyness} is past the chain"
            );
            assert_eq!(moneyness.depth(), 11);
            let strike = strike_at(atm, moneyness, step, OptionSide::Call)
                .expect("outside the chain is still on the grid");
            assert_eq!(strike.raw(), atm.raw() + i64::from(steps) * step.raw());
        }
        // The chain edges resolve to the strikes the source's grid_around_atm
        // would have listed at half_width 10.
        assert_eq!(
            strike_at(atm, MoneynessSteps::new(-10), step, OptionSide::Call),
            Ok(paisa(23_500))
        );
        assert_eq!(
            strike_at(atm, MoneynessSteps::new(10), step, OptionSide::Call),
            Ok(paisa(24_500))
        );
        // And the far extremes of the type itself.
        assert!(!MoneynessSteps::new(i32::MAX).is_within_chain());
        assert!(!MoneynessSteps::new(i32::MIN).is_within_chain());
        assert_eq!(MoneynessSteps::new(i32::MIN).depth(), 2_147_483_648);
    }

    #[test]
    fn the_seven_locked_offset_rules_read_as_the_source_spells_them() {
        // The source's ALL_STRIKE_RULES offset params, and the strings they
        // read as. `atm_plus_N_in_dir` is +N; `atm_minus_N_in_dir` is -N.
        for (steps, rendered, bucket) in [
            (0i32, "ATM", Moneyness::AtTheMoney),
            (1, "OTM+1", Moneyness::OutOfTheMoney),
            (2, "OTM+2", Moneyness::OutOfTheMoney),
            (5, "OTM+5", Moneyness::OutOfTheMoney),
            (-1, "ITM-1", Moneyness::InTheMoney),
            (-2, "ITM-2", Moneyness::InTheMoney),
            (-5, "ITM-5", Moneyness::InTheMoney),
        ] {
            let moneyness = MoneynessSteps::new(steps);
            assert_eq!(moneyness.to_string(), rendered);
            assert_eq!(moneyness.bucket(), bucket);
            assert!(
                moneyness.is_within_chain(),
                "{rendered} is inside the chain"
            );
        }
        // i32::MIN renders rather than overflowing on a negation with no i32.
        assert_eq!(MoneynessSteps::new(i32::MIN).to_string(), "ITM-2147483648");
        assert_eq!(MoneynessSteps::new(i32::MAX).to_string(), "OTM+2147483647");
        assert_eq!(Moneyness::AtTheMoney.as_str(), "ATM");
        assert_eq!(Moneyness::InTheMoney.to_string(), "ITM");
        assert_eq!(Moneyness::OutOfTheMoney.to_string(), "OTM");
    }

    #[test]
    fn a_non_positive_strike_or_spot_is_refused_by_name() {
        let step = nifty_step();
        for bad in [0i64, -1, i64::MIN] {
            assert_eq!(
                atm_offset(Paisa::from_raw(bad), paisa(24_000), step),
                Err(CostError::NotPositive {
                    quantity: "strike",
                    value: bad
                })
            );
            assert_eq!(
                atm_offset(paisa(24_000), Paisa::from_raw(bad), step),
                Err(CostError::NotPositive {
                    quantity: "spot",
                    value: bad
                })
            );
            // And through both wrappers, so no path skips the check.
            assert!(classify(Paisa::from_raw(bad), paisa(24_000), step, OptionSide::Call).is_err());
            assert!(
                moneyness_steps(paisa(24_000), Paisa::from_raw(bad), step, OptionSide::Put)
                    .is_err()
            );
        }
    }

    #[test]
    fn an_offset_past_i32_is_refused_rather_than_wrapped() {
        // A one-paisa grid across the whole i64 domain: the offset needs more
        // than 31 bits and is refused rather than truncated.
        let unit = StrikeStep::new_const(1);
        assert_eq!(
            atm_offset(Paisa::from_raw(i64::MAX), Paisa::from_raw(1), unit),
            Err(CostError::Overflow {
                operation: "strike offset in steps"
            })
        );
        assert_eq!(
            atm_offset(Paisa::from_raw(1), Paisa::from_raw(i64::MAX), unit),
            Err(CostError::Overflow {
                operation: "strike offset in steps"
            })
        );
        // Exactly at the edge it still answers, which proves the refusal is the
        // boundary and not the whole neighbourhood.
        let edge = i64::from(i32::MAX);
        assert_eq!(
            atm_offset(Paisa::from_raw(edge + 1), Paisa::from_raw(1), unit),
            Ok(i32::MAX)
        );
        // And the put inversion of i32::MIN, whose negation has no i32.
        let deep = i64::from(i32::MIN);
        assert_eq!(
            atm_offset(Paisa::from_raw(1), Paisa::from_raw(1 - deep), unit),
            Ok(i32::MIN)
        );
        assert_eq!(
            moneyness_steps(
                Paisa::from_raw(1),
                Paisa::from_raw(1 - deep),
                unit,
                OptionSide::Put
            ),
            Err(CostError::Overflow {
                operation: "moneyness sign inversion"
            })
        );
        // The call reads the same input without inverting, so it answers.
        assert_eq!(
            moneyness_steps(
                Paisa::from_raw(1),
                Paisa::from_raw(1 - deep),
                unit,
                OptionSide::Call
            ),
            Ok(MoneynessSteps::new(i32::MIN))
        );
    }

    #[test]
    fn the_two_grids_disagree_about_the_same_strike_and_that_is_the_point() {
        // A single hardcoded step would read this strike as one moneyness for
        // both indices. It is two steps out on the 50-point grid and one on the
        // 100-point one.
        let spot = paisa(24_000);
        let strike = paisa(24_100);
        assert_eq!(
            moneyness_steps(strike, spot, nifty_step(), OptionSide::Call),
            Ok(MoneynessSteps::new(2))
        );
        assert_eq!(
            moneyness_steps(strike, spot, banknifty_step(), OptionSide::Call),
            Ok(MoneynessSteps::new(1))
        );
        assert_ne!(nifty_step(), banknifty_step());
    }

    #[test]
    fn a_moneyness_is_ordered_hashable_and_debuggable() {
        use std::collections::HashSet;

        assert!(MoneynessSteps::new(-5) < MoneynessSteps::AT_THE_MONEY);
        assert!(MoneynessSteps::AT_THE_MONEY < MoneynessSteps::new(5));
        assert_eq!(
            format!("{:?}", MoneynessSteps::new(-2)),
            "MoneynessSteps(-2)"
        );
        let mut set = HashSet::new();
        assert!(set.insert(MoneynessSteps::new(0)));
        assert!(!set.insert(MoneynessSteps::AT_THE_MONEY));
        assert!(set.insert(MoneynessSteps::new(1)));
        assert_eq!(set.len(), 2);
        // The bucket orders the way a chain reads: in, at, out.
        assert!(Moneyness::InTheMoney < Moneyness::AtTheMoney);
        assert!(Moneyness::AtTheMoney < Moneyness::OutOfTheMoney);
        assert_eq!(format!("{:?}", Moneyness::AtTheMoney), "AtTheMoney");
    }
}
