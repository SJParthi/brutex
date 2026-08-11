//! The `near_*` tolerance: measured, pinned, and with the sweep it came from.
//!
//! # What this is
//!
//! Seventy-five of the 185 live positions in [`crate::table`] are `near_*`
//! conditions: *is the close near the previous day's high*, *near the R2
//! pivot*, *near the 61.8% rung*. Every one needs a band half-width, and
//! `docs/00-charter.md`, `docs/03-vocabulary.md` and `docs/04-invariants.md`
//! between them said nothing about what it is -- so for seventy-five shipped
//! and appended bits the predicate was literally undefined.
//!
//! `CLAUDE.md` §3 rule 1 forbids inventing the number, so the constants held
//! the sentinel [`UNPINNED`] until a sweep fixed it. It is pinned now, and the
//! sweep is recorded below rather than in somebody's memory.
//!
//! # The unit and the shape, both now measured
//!
//! The band is *thousandths of the anchor's range* -- the session high minus
//! its low for a Fibonacci rung, the CPR width for a pivot band. There are
//! **two** widths, because those are fractions of different quantities:
//! [`TOL_FIB_MILLI`] = 10 and [`TOL_PIVOT_MILLI`] = 500.
//!
//! **The base is the range, and deliberately not the level.** The first draft
//! of this module used the level, and the arithmetic rules it out: a NIFTY
//! level near ₹25,000.00 with a 200-point session admits ±2.00 points at the
//! measured width, but the finest band a level-relative shape can express at
//! `milli` granularity is a width of 1, which is ±25.00 points -- **12.5x
//! too coarse at its finest setting**. The shape could not express the answer,
//! never mind the number.
//!
//! A range-relative band is also the only one comparable across the two swept
//! instruments: 200 paisa is a different thing on NIFTY near 25,000 than on
//! BANKNIFTY near 57,000, but one hundredth of each instrument's own session
//! range is the same statement about both.
//!
//! # The measurement
//!
//! Swept over 75,000 synthetic bars of the current-day ladder, and cross-checked
//! on 110,625 bars against the frozen previous-day and five-session anchors:
//!
//! | width | any rung fires | busiest rung | two rungs at once |
//! |---|---|---|---|
//! | R/1000 | 0.98% | 0.28% | 0 |
//! | R/200 | 5.47% | 1.50% | 0 |
//! | **R/100** | **11.21%** | **2.98%** | **0** |
//! | R/50 | 23.40% | 5.99% | 0 |
//! | R/15 | 74.19% | 20.19% | **1,997** |
//!
//! Two rungs can both fire only when their numerator gap is at most twice the
//! scaled width, and the ladder's smallest gap is 118, so the at-most-one-rung
//! property holds while the scaled width stays under 59. `R/100` scales to 10
//! -- a 5.9x margin -- and the measured multi-fire count is 0 at every width
//! inside that bound and 1,997 at the first width outside it. The algebra
//! predicted where it breaks and the measurement found it there.
//!
//! `R/100` is chosen over the neighbouring widths because its busiest rung
//! fires on about 3% of bars: often enough to carry information, rarely enough
//! to discriminate. See `docs/05-decisions.md` D-0076.
//!
//! **Still unmeasured:** every figure above is from generated bars, not from
//! the 2020-2026 history, which is not yet pulled. An ATR-relative or
//! per-timeframe width was not tested against this one.
//!
//! # Why it is a token and not a number
//!
//! [`Tolerance`] cannot be constructed from the sentinel, and
//! [`crate::table::set_near`] cannot be called without one. The refusal path
//! stays alive now that the values are pinned: returning either constant to
//! [`UNPINNED`] makes every `near_*` bit unreachable loudly, with a named
//! reason, rather than quietly evaluating false -- which is what a `0` default
//! would have produced and what `CLAUDE.md` §4 calls a fallback that hides a
//! failure.

use crate::error::VocabError;

/// The sentinel that means *no measurement has fixed this*.
///
/// `i64::MIN` is already this repository's null for an integer that has no
/// value -- `CLAUDE.md` §7 gives it to open interest for the same reason: zero
/// is a real tolerance (exact equality) and cannot double as "unset".
pub const UNPINNED: i64 = i64::MIN;

/// The Fibonacci band, in thousandths of the **session range**.
///
/// **PINNED at 10 — one hundredth of the range — by D-0076.** The sweep behind
/// it is in the module documentation.
pub const TOL_FIB_MILLI: i64 = 10;

/// The pivot band, in thousandths of the **CPR width**.
///
/// **PINNED at 500 by D-0079, and it is not a measurement — it is the design
/// source read off the page.** `docs/09-design-sources.md` §1 gives the
/// indicator's own rule:
///
/// ```text
/// cpr_half = abs(daily_pivot - daily_bc) * zone_mult      // zone_mult = 1.0
/// ```
///
/// Since `daily_tc = 2*daily_pivot - daily_bc`, the pivot sits at the exact
/// midpoint of `[bc, tc]`, so `cpr_half` is **half the CPR width** — 500
/// thousandths of it, on every day, algebraically and not approximately.
/// Verified on three instrument scales: it comes out 500 every time.
///
/// # Why this is a SECOND constant and not the same one
///
/// The first draft of this module had one global width, pinned at 10, applied
/// to every `near_*` position. That was measured correctly for the Fibonacci
/// ladder and is **fifty times too narrow** for a pivot band, and the two
/// cannot be reconciled by choosing a better single number because **they are
/// fractions of different quantities** — a session's high-minus-low against a
/// central pivot range's width.
///
/// It is worse than a wrong number. Position 6 `near_pivot_p` was retired as an
/// exact duplicate of 62 `inside_cpr`, and that identity holds **only** at 500:
/// the pivot's band `[P - cpr_half, P + cpr_half]` is exactly `[bc, tc]`. At 10
/// the band is fifty times narrower and the two are different predicates, so a
/// live condition was tombstoned on a false premise — and an index is never
/// reissued, so it cannot be undone. Recorded rather than quietly corrected.
pub const TOL_PIVOT_MILLI: i64 = 500;

/// The Fibonacci ladder's numerators, in thousandths. The rung set the design
/// source draws, in order.
pub const LADDER_NUMERATORS: [i64; 11] = [0, 236, 382, 500, 618, 786, 1000, 1272, 1618, 2000, 2618];

/// The smallest gap between two adjacent numerators on [`LADDER_NUMERATORS`].
///
/// **Computed from the ladder, not written beside it.** It was a hand-kept `118`
/// — correct for 382→500 and 500→618 — and nothing tied it to the rungs, so
/// inflating it silently disarmed the only guard on [`TOL_FIB_MILLI`]. Now a rung
/// change moves the bound with it.
/// The lesser of two, so the gap fold below needs no indexing.
const fn lesser(a: i64, b: i64) -> i64 {
    if a < b { a } else { b }
}

/// The smallest gap between two adjacent numerators on [`LADDER_NUMERATORS`].
///
/// **Computed from the ladder, not written beside it.** It was a hand-kept `118`
/// -- correct for 382 to 500 and 500 to 618 -- and nothing tied it to the rungs,
/// so inflating it silently disarmed the only guard on [`TOL_FIB_MILLI`]. Now a
/// rung change moves the bound with it, or fails to compile.
pub const SMALLEST_LADDER_GAP: i64 = {
    // DESTRUCTURED, not indexed. `clippy::indexing_slicing` is denied in this
    // workspace, and destructuring buys the same thing `MAX_VENDOR_LEN` buys by
    // the same means: adding or removing a rung makes THIS a compile error
    // rather than a bound that quietly loosens.
    let [r0, r1, r2, r3, r4, r5, r6, r7, r8, r9, r10] = LADDER_NUMERATORS;
    let gap = lesser(
        r1 - r0,
        lesser(
            r2 - r1,
            lesser(
                r3 - r2,
                lesser(
                    r4 - r3,
                    lesser(
                        r5 - r4,
                        lesser(
                            r6 - r5,
                            lesser(r7 - r6, lesser(r8 - r7, lesser(r9 - r8, r10 - r9))),
                        ),
                    ),
                ),
            ),
        ),
    );
    assert!(gap > 0, "the ladder must be strictly ascending");
    gap
};

// The value the hand-kept constant claimed, checked rather than trusted. If a
// rung is ever added or moved this fails and says the bound changed, instead of
// letting the guard loosen unnoticed.
const _: () = assert!(
    SMALLEST_LADDER_GAP == 118,
    "the smallest ladder gap moved; TOL_FIB_MILLI's bound changed with it"
);

// THE AT-MOST-ONE-RUNG BOUND, enforced at COMPILE time rather than in a test.
//
// Two Fibonacci rungs can both fire on one bar only if their numerator gap is at
// most twice the scaled width. The ladder's smallest gap is 118, so the property
// holds while `2 * TOL_FIB_MILLI < 118`. At the pinned 10 that is a 5.9x margin,
// and the sweep in the module documentation measured 0 multi-fires at every
// width inside the bound and 1,997 at the first width outside it.
//
// IT BINDS THE FIBONACCI WIDTH ONLY. Applying it to TOL_PIVOT_MILLI was the
// defect D-0079 records: 500 exceeds the cap of 58, so the design source's own
// zone width was a BUILD FAILURE. The pivot levels are not a ladder of rungs on
// one range and this bound says nothing about them — the R/S levels are spaced
// by whole multiples of (H - L), not by 118 thousandths of anything.
//
// A `const` assertion and not a `#[test]`: widening the Fibonacci band past the
// bound should fail the BUILD, not a test run somebody can skip.
const _: () = assert!(
    2 * TOL_FIB_MILLI < SMALLEST_LADDER_GAP,
    "TOL_FIB_MILLI is wide enough for two adjacent Fibonacci rungs to fire on one bar"
);

// The pivot band has its own bound: a band wider than the whole CPR would swallow
// tc and bc, making `inside` meaningless. Half the width is the design's value
// and also the ceiling.
const _: () = assert!(
    TOL_PIVOT_MILLI <= 500,
    "a pivot band wider than half the CPR width swallows tc and bc"
);

/// A measured tolerance. The only key that opens [`crate::table::set_near`].
///
/// It is a newtype and not an `i64` so that the check happens once, at the
/// boundary, and every `near_*` evaluation downstream is holding proof that
/// somebody pinned the number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tolerance(i64);

impl Tolerance {
    /// Pin the tolerance to `milli` thousandths of the anchor's range.
    ///
    /// # Errors
    ///
    /// [`VocabError::ToleranceUnpinned`] when handed [`UNPINNED`];
    /// [`VocabError::ToleranceNegative`] for any other negative width.
    pub const fn from_milli(milli: i64) -> Result<Self, VocabError> {
        if milli == UNPINNED {
            return Err(VocabError::ToleranceUnpinned);
        }
        if milli < 0 {
            return Err(VocabError::ToleranceNegative { milli });
        }
        Ok(Self(milli))
    }

    /// The pinned width, in thousandths of the anchor's range.
    #[must_use]
    pub const fn milli(self) -> i64 {
        self.0
    }

    /// Is `value` inside the band around `level`? All three are paisa integers
    /// -- `CLAUDE.md` §7, and there is no floating point anywhere in the
    /// comparison.
    ///
    /// `|value - level| * 1000 <= milli * |range|`, evaluated in `i128` so a
    /// paisa price multiplied by a width cannot overflow: the largest term is
    /// about `2^63 * 2^63`, which `i128` holds and `i64` does not.
    ///
    /// `range_paisa` is the span of the anchor the level was derived from --
    /// the session's high minus its low for a Fibonacci rung, the CPR width for
    /// a pivot band. **It is the base, and the level is not.** See the module
    /// documentation for the measurement that settled that.
    ///
    /// A non-positive range yields `false` rather than an error: a zero-range
    /// anchor is a real market state (a circuit-frozen session), and
    /// `docs/03-vocabulary.md` §4 requires a bit that cannot be evaluated to be
    /// false rather than "probably".
    #[must_use]
    pub fn covers(self, value_paisa: i64, level_paisa: i64, range_paisa: i64) -> bool {
        if range_paisa <= 0 {
            return false;
        }
        let delta = (i128::from(value_paisa) - i128::from(level_paisa)).abs();
        let band = i128::from(self.0) * i128::from(range_paisa);
        delta * 1000 <= band
    }
}

/// The Fibonacci band as pinned today. See [`TOL_FIB_MILLI`].
///
/// # Errors
///
/// [`VocabError::ToleranceUnpinned`] if the constant is ever returned to the
/// sentinel, and [`VocabError::ToleranceNegative`] for a negative pin. Neither
/// can happen at the value pinned today; the function keeps the refusal path
/// alive so that unpinning stays possible without a type change.
pub const fn pinned_fib() -> Result<Tolerance, VocabError> {
    Tolerance::from_milli(TOL_FIB_MILLI)
}

/// The pivot band as pinned today. See [`TOL_PIVOT_MILLI`].
///
/// # Errors
///
/// As [`pinned_fib`].
pub const fn pinned_pivot() -> Result<Tolerance, VocabError> {
    Tolerance::from_milli(TOL_PIVOT_MILLI)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The number is pinned at the swept value, and the refusal path that
    /// guarded it while it was unknown still works.
    #[test]
    fn the_tolerance_is_pinned_at_the_measured_width() {
        assert_eq!(
            TOL_FIB_MILLI, 10,
            "D-0076 pinned the Fibonacci band at one hundredth of the session range"
        );
        assert_eq!(pinned_fib().map(Tolerance::milli), Ok(10));
        assert_eq!(pinned_pivot().map(Tolerance::milli), Ok(500));
        assert_eq!(
            Tolerance::from_milli(UNPINNED),
            Err(VocabError::ToleranceUnpinned),
            "unpinning must still refuse rather than default to zero"
        );
    }

    #[test]
    fn a_negative_band_is_refused_but_zero_is_not() {
        assert_eq!(
            Tolerance::from_milli(-1),
            Err(VocabError::ToleranceNegative { milli: -1 })
        );
        let exact = Tolerance::from_milli(0);
        let Ok(exact) = exact else {
            unreachable!("zero is a legal pin: it means exact equality")
        };
        assert_eq!(exact.milli(), 0);
        assert!(exact.covers(100, 100, 10_000));
        assert!(!exact.covers(101, 100, 10_000));
    }

    #[test]
    fn the_band_is_relative_to_the_range_and_not_to_the_level() {
        // A 200.00-point session on NIFTY, in paisa. At the pinned width the
        // band is one hundredth of that span: 200 paisa, or 2.00 points.
        let Ok(tol) = pinned_fib() else {
            unreachable!("the pinned width is neither the sentinel nor negative")
        };
        let range = 20_000i64;
        let level = 2_500_000i64;
        assert!(tol.covers(level, level, range));
        assert!(tol.covers(level + 200, level, range));
        assert!(tol.covers(level - 200, level, range));
        assert!(!tol.covers(level + 201, level, range));
        assert!(!tol.covers(level - 201, level, range));

        // The level's own magnitude must not move the band. Same range, a
        // BANKNIFTY-scale level, identical edge.
        let bank = 5_700_000i64;
        assert!(tol.covers(bank + 200, bank, range));
        assert!(!tol.covers(bank + 201, bank, range));
    }

    /// A circuit-frozen session has no range, so every `near_*` bit is false
    /// rather than "probably" -- `docs/03-vocabulary.md` §4.
    #[test]
    fn a_zero_or_negative_range_covers_nothing() {
        let Ok(tol) = pinned_fib() else {
            unreachable!("the pinned width is legal")
        };
        assert!(!tol.covers(100, 100, 0));
        assert!(!tol.covers(100, 100, -1));
        assert!(!tol.covers(100, 100, i64::MIN));
    }

    #[test]
    fn a_level_at_the_edge_of_the_type_does_not_overflow() {
        let Ok(tol) = Tolerance::from_milli(i64::MAX) else {
            unreachable!("i64::MAX is neither the sentinel nor negative")
        };
        assert!(tol.covers(i64::MIN, i64::MAX, i64::MAX));
        assert!(tol.covers(i64::MAX, i64::MIN, i64::MAX));
        let Ok(tight) = Tolerance::from_milli(0) else {
            unreachable!("zero is a legal pin")
        };
        assert!(!tight.covers(i64::MIN, i64::MAX, i64::MAX));
    }

    #[test]
    fn a_tolerance_carries_its_own_value() {
        let Ok(a) = Tolerance::from_milli(7) else {
            unreachable!("7 is a legal pin")
        };
        let Ok(b) = Tolerance::from_milli(7) else {
            unreachable!("7 is a legal pin")
        };
        assert_eq!(a, b);
        assert_eq!(a.milli(), 7);
        assert!(format!("{a:?}").contains('7'));
    }
}
