//! The `near_*` tolerance: measured, pinned, and with the sweep it came from.
//!
//! # What this is
//!
//! Eighty-one of the 238 live positions in [`crate::table`] are `near_*`
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
//! to discriminate. See `docs/05-decisions.md` D-0076 (the `near_*` band) --
//! disambiguated because that number was issued twice; D-0104 has the table.
//!
//! **Still unmeasured:** every figure above is from generated bars, not from
//! the 2020-2026 history, which is not yet pulled. An ATR-relative or
//! per-timeframe width was not tested against this one.
//!
//! # Why it is a token and not a number
//!
//! [`Tolerance`] carries **which of the two bases it measures** ([`Base`]) as
//! well as the width. Both facts are needed: the width alone cannot be checked
//! against anything, because 10 and 500 are each correct for one family and
//! wrong by fifty times for the other, and a [`Tolerance`] that had only the
//! number was accepted by [`crate::table::set_near`] for any `near_*` position
//! at all. See [`Base`] for the failure that could not be detected downstream.
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
/// **PINNED at 10 — one hundredth of the range — by D-0076 (the `near_*` band).**
/// The other D-0076 is about Groww's interval words; see D-0104. The sweep behind
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

/// The lesser of two, so the gap fold below needs no indexing.
///
/// The six lines that used to precede this one were a second copy of
/// [`SMALLEST_LADDER_GAP`]'s own documentation, describing that constant while
/// sitting on this function. Deleted rather than reworded: the surviving copy is
/// twelve lines below, on the item it is about.
const fn lesser(a: i64, b: i64) -> i64 {
    if a < b { a } else { b }
}

/// The smallest gap between two adjacent numerators on [`LADDER_NUMERATORS`].
///
/// **Computed from the ladder, not written beside it.** It was a hand-kept `118`
/// -- correct for 382 to 500 and 500 to 618 -- and nothing tied it to the rungs,
/// so inflating it silently disarmed the only guard on [`TOL_FIB_MILLI`]. Now a
/// rung change moves the bound with it, or fails to compile.
/// The ten adjacent gaps on [`LADDER_NUMERATORS`], computed once.
///
/// **One array, two facts derived from it**, and that is the whole point.
/// `SMALLEST_LADDER_GAP` and [`LADDER_GAPS_TELESCOPE`] both read THIS, so a
/// defect in one subtraction reaches both — a separate copy of the arithmetic in
/// each would let a mutation break one while the other kept passing.
///
/// DESTRUCTURED, not indexed. `clippy::indexing_slicing` is denied in this
/// workspace, and destructuring buys the same thing `MAX_VENDOR_LEN` buys by the
/// same means: adding or removing a rung makes THIS a compile error rather than
/// a bound that quietly loosens.
const LADDER_GAPS: [i64; 10] = {
    let [r0, r1, r2, r3, r4, r5, r6, r7, r8, r9, r10] = LADDER_NUMERATORS;
    [
        r1 - r0,
        r2 - r1,
        r3 - r2,
        r4 - r3,
        r5 - r4,
        r6 - r5,
        r7 - r6,
        r8 - r7,
        r9 - r8,
        r10 - r9,
    ]
};

// The adjacent gaps must sum to the ladder's whole span.
//
// # What this catches that `SMALLEST_LADDER_GAP` cannot
//
// `cargo-mutants` turned each `-` in the gap computation into a `+` — **ten of
// them, and every one survived.** The reason is arithmetic rather than
// carelessness: the ladder's smallest gap is 118 and it occurs **twice**,
// `500 - 382` and `618 - 500`. Mutating either into a sum leaves the other as
// the minimum, so the constant is still 118, the `== 118` assertion still
// holds, and nothing anywhere observes a difference.
//
// No runtime test can kill an equivalent mutant. Only a stronger invariant can
// stop it being equivalent, and this is that invariant: adjacent differences
// **telescope**, because every interior rung is added once and subtracted once,
// so their sum is exactly `last - first` whatever the rungs are. Turn any
// single `-` into a `+` and two terms stop cancelling — the total moves and
// this fails to COMPILE.
//
// It is a real property of the ladder rather than a trick played on the
// mutation tool: a fold of adjacent gaps that does not telescope is a fold that
// is not reading adjacent gaps.
const _: () = {
    let [r0, .., r10] = LADDER_NUMERATORS;
    let [g0, g1, g2, g3, g4, g5, g6, g7, g8, g9] = LADDER_GAPS;
    assert!(
        g0 + g1 + g2 + g3 + g4 + g5 + g6 + g7 + g8 + g9 == r10 - r0,
        "the adjacent gaps must telescope to the ladder's span; a term that \
         adds where it should subtract breaks this and nothing else notices"
    );
};

/// The smallest gap between two adjacent numerators on [`LADDER_NUMERATORS`].
///
/// **Computed from the ladder, not written beside it.** It was a hand-kept `118`
/// -- correct for 382 to 500 and 500 to 618 -- and nothing tied it to the rungs,
/// so inflating it silently disarmed the only guard on [`TOL_FIB_MILLI`]. Now a
/// rung change moves the bound with it, or fails to compile.
pub const SMALLEST_LADDER_GAP: i64 = {
    let [g0, g1, g2, g3, g4, g5, g6, g7, g8, g9] = LADDER_GAPS;
    let gap = lesser(
        g0,
        lesser(
            g1,
            lesser(
                g2,
                lesser(
                    g3,
                    lesser(g4, lesser(g5, lesser(g6, lesser(g7, lesser(g8, g9))))),
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

/// Which quantity a band is a fraction of.
///
/// # Why the base is a type and not a paragraph
///
/// [`TOL_FIB_MILLI`] is 10 and [`TOL_PIVOT_MILLI`] is 500, and those two
/// numbers are not on one scale: the first is thousandths of the **session
/// range**, the second thousandths of the **CPR width**. The paragraph above
/// [`TOL_PIVOT_MILLI`] has said so since D-0079, and saying so was all that
/// stood between a caller and the wrong band -- because both widths arrived at
/// [`crate::table::set_near`] as a bare [`Tolerance`], indistinguishable once
/// constructed, and that function checked only that the position was a `near_*`
/// row. Handing the pivot width to a Fibonacci rung returned `Ok` and set the
/// bit; so did handing the Fibonacci width to a pivot level.
///
/// **The result is undetectable downstream, which is what makes it worth a
/// type.** A [`crate::ConditionMask`] is a set of positions and carries no
/// record of the band that decided them, so a stored result set built on a
/// fifty-times-wrong width is byte for byte a correct one. There is no later
/// check that could find it and no test on the mask that could fail.
/// `CLAUDE.md` §4 bans a fallback that hides a failure: this enum is what makes
/// the two families different *values*, so a mismatch is
/// [`VocabError::WrongBand`] at the call rather than a silent `Ok`.
///
/// # What it does not fix
///
/// Nothing about *which* base a position should have. That is declared once per
/// row in [`crate::table::TABLE`] and this type only makes the declaration
/// checkable. A row declared into the wrong family is still wrong, and is still
/// caught only by reading it against the module that sets it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Base {
    /// Thousandths of the session's high minus its low.
    ///
    /// Every Fibonacci ladder (previous-day, five-session, current-day, gap),
    /// the opening-range bands, the VWAP bands, the gap mid and the swing levels
    /// measure against this. [`TOL_FIB_MILLI`] is its pinned width and the sweep
    /// in this module's documentation is the measurement behind it.
    SessionRange,
    /// Thousandths of the CPR width, which is `tc - bc`.
    ///
    /// The pivot ladder R1–R5 / S1–S5, the CPR's own two edges, and the previous
    /// day's high and low measure against this -- the last two because
    /// `crates/indicators/src/daily.rs` builds them from the same daily levels
    /// and hands the whole plan one width. [`TOL_PIVOT_MILLI`] is its pinned
    /// width and D-0079 is where it comes from.
    CprWidth,
}

/// A measured tolerance, and the quantity it is measured against.
///
/// It is a struct and not an `i64` so that the check happens once, at the
/// boundary, and every `near_*` evaluation downstream is holding proof that
/// somebody pinned the number.
///
/// # The base rides on the value, and that is deliberate
///
/// The width alone was the whole type until the defect [`Base`] documents. A
/// width with no base is a number that cannot be checked against anything --
/// `500` is right for a pivot band and fifty times too wide for a Fibonacci
/// rung, and the value carried nothing that could tell the two apart. Now it
/// does, and [`crate::table::set_near`] compares it against the base the row
/// declares.
///
/// `base` is an `Option` because [`Tolerance::from_milli`] exists and states a
/// width without saying what it is a fraction of. `None` is not a default base;
/// it is the absence of one, and it sets no `near_*` bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tolerance {
    /// The width, in thousandths of whatever `base` names.
    milli: i64,
    /// What the width is a fraction of, or `None` when it was never stated.
    base: Option<Base>,
}

impl Tolerance {
    /// Pin a width of `milli` thousandths **without saying what of**.
    ///
    /// # This cannot set a `near_*` bit, and that is the point
    ///
    /// Every position in [`crate::table::TABLE`] that needs a band declares
    /// which quantity the band is a fraction of, so a tolerance that names no
    /// base matches no position: [`crate::table::set_near`] refuses it with
    /// [`VocabError::WrongBand`] carrying `got: None`. Use
    /// [`Tolerance::from_milli_on`], or the two pinned constructors below, to
    /// build one that can decide a bit.
    ///
    /// The constructor stays because it is where the width itself is validated
    /// -- one law for the sentinel and one for a negative band, called by
    /// everything else here rather than copied -- and because a caller that only
    /// wants to ask *is this width legal* should not have to invent a base to
    /// find out.
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
        Ok(Self { milli, base: None })
    }

    /// Pin a width of `milli` thousandths **of `base`**.
    ///
    /// The width is validated by [`Tolerance::from_milli`] and this adds the
    /// base to the result, so there is exactly one place that decides whether a
    /// width is legal and exactly one place that decides what it measures.
    ///
    /// # Errors
    ///
    /// As [`Tolerance::from_milli`]. The base is not validated because there is
    /// nothing to validate -- [`Base`] has two variants and both are real.
    pub const fn from_milli_on(base: Base, milli: i64) -> Result<Self, VocabError> {
        match Self::from_milli(milli) {
            Ok(Self { milli, .. }) => Ok(Self {
                milli,
                base: Some(base),
            }),
            Err(e) => Err(e),
        }
    }

    /// The pinned width, in thousandths of whatever [`Tolerance::base`] names.
    ///
    /// **A bare number, and comparing two of them across bases is meaningless.**
    /// 10 and 500 are not 50x apart in any sense a caller can use; they are
    /// fractions of different quantities.
    #[must_use]
    pub const fn milli(self) -> i64 {
        self.milli
    }

    /// What the width is a fraction of, or `None` when it was never stated.
    #[must_use]
    pub const fn base(self) -> Option<Base> {
        self.base
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
    /// **It does not check that `range_paisa` is the quantity
    /// [`Tolerance::base`] names, and it cannot.** A caller passing the session
    /// range with a pivot-based tolerance hands this function two `i64` that are
    /// both spans, and no arithmetic here can tell them apart. That check lives
    /// one level up, in [`crate::table::set_near`], where the *position* says
    /// which family it belongs to.
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
        let band = i128::from(self.milli) * i128::from(range_paisa);
        delta * 1000 <= band
    }
}

/// The Fibonacci band as pinned today, on [`Base::SessionRange`]. See
/// [`TOL_FIB_MILLI`].
///
/// # Errors
///
/// [`VocabError::ToleranceUnpinned`] if the constant is ever returned to the
/// sentinel, and [`VocabError::ToleranceNegative`] for a negative pin. Neither
/// can happen at the value pinned today; the function keeps the refusal path
/// alive so that unpinning stays possible without a type change.
pub const fn pinned_fib() -> Result<Tolerance, VocabError> {
    Tolerance::from_milli_on(Base::SessionRange, TOL_FIB_MILLI)
}

/// The pivot band as pinned today, on [`Base::CprWidth`]. See
/// [`TOL_PIVOT_MILLI`].
///
/// # What changed here, and what it costs a caller
///
/// These two used to differ **only** by which constant they read, so their
/// results were interchangeable at every call site that took a [`Tolerance`].
/// They now differ in the type's own value as well, which is what lets
/// [`crate::table::set_near`] tell them apart. A caller that was passing the
/// wrong one has not been slowed down; it has been stopped.
///
/// # Errors
///
/// As [`pinned_fib`].
pub const fn pinned_pivot() -> Result<Tolerance, VocabError> {
    Tolerance::from_milli_on(Base::CprWidth, TOL_PIVOT_MILLI)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand `body` the tolerance `pin` carries, and fail naming the pin if it
    /// was refused.
    ///
    /// Every test below needs a [`Tolerance`] and [`Tolerance::from_milli`]
    /// returns a `Result`, so each of them opened with
    ///
    /// ```text
    /// let Ok(tol) = pinned_fib() else { unreachable!("a legal width") };
    /// ```
    ///
    /// and that `else` arm costs something invisible: `unreachable!` expands to
    /// a panic **inside this crate**, so `cargo llvm-cov` records a region that
    /// a correct build can never enter and no test run can ever close. There
    /// were seven of them in this module. Routing through `Result::map` states
    /// the same thing as an assertion that can actually fail -- `Ok(())` is
    /// produced only when the closure ran -- and leaves no arm behind.
    ///
    /// `Result<Tolerance, VocabError>` is `Copy`, so the failure message can
    /// still name the pin that was refused.
    fn with_pin(pin: Result<Tolerance, VocabError>, body: impl FnOnce(Tolerance)) {
        assert_eq!(
            pin.map(body),
            Ok(()),
            "a width this test needs was refused: {pin:?}"
        );
    }

    /// The number is pinned at the swept value, and the refusal path that
    /// guarded it while it was unknown still works.
    #[test]
    fn the_tolerance_is_pinned_at_the_measured_width() {
        assert_eq!(
            TOL_FIB_MILLI, 10,
            "D-0076 (the `near_*` band) pinned the Fibonacci band at one hundredth \
             of the session range"
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
        with_pin(Tolerance::from_milli(0), |exact| {
            assert_eq!(exact.milli(), 0);
            assert!(exact.covers(100, 100, 10_000));
            assert!(!exact.covers(101, 100, 10_000));
        });
    }

    /// Both refusals, at RUN time, over widths that arrive from a list.
    ///
    /// `from_milli` is a `const fn` and every other call to it in this crate
    /// hands it a literal or one of this crate's own constants, which is a thing
    /// a compiler is free to fold away. These arrive from a slice, and each one
    /// is checked against the *whole* error it must produce -- the negative arm
    /// has to carry the width it was handed, or an operator reading the message
    /// learns only that something was negative.
    ///
    /// `i64::MIN + 1` is the case that separates the two arms: it is the
    /// largest-magnitude negative width that is **not** the sentinel, so an
    /// order that tested `milli < 0` first, or a sentinel test written
    /// `milli <= UNPINNED`, would mis-file exactly one value and this is it.
    #[test]
    fn every_illegal_width_is_refused_and_the_error_names_the_width() {
        assert_eq!(
            Tolerance::from_milli(UNPINNED),
            Err(VocabError::ToleranceUnpinned),
            "the sentinel means no measurement has fixed the band, and it must \
             not be reported as an ordinary negative width"
        );
        for milli in [-1i64, -10, -1_000, i64::MIN + 1] {
            assert_eq!(
                Tolerance::from_milli(milli),
                Err(VocabError::ToleranceNegative { milli }),
                "a band with a negative half-width is not a band, and the \
                 refusal has to name the width it was offered"
            );
        }
    }

    /// **Naming a base does not make an illegal width legal.**
    ///
    /// [`Tolerance::from_milli_on`] is one law and one addition:
    /// [`Tolerance::from_milli`] decides whether a width is legal, and this adds
    /// the base to what came back. Only the *addition* was ever executed. The
    /// two callers in this workspace are [`pinned_fib`] and [`pinned_pivot`],
    /// both handing it a constant that is legal today, so the `Err` arm was a
    /// refusal nothing had ever taken -- `cargo llvm-cov` measured it as the one
    /// uncovered line in this module.
    ///
    /// **What an unwitnessed arm can quietly become.** Written
    /// `Err(_) => Ok(Self { milli, base: Some(base) })` it compiles, and the
    /// only difference is that a [`Tolerance`] can now be minted from
    /// [`UNPINNED`] or from a negative width. That value goes to
    /// [`crate::table::set_near`], which checks the *base* and never re-checks
    /// the width, so a `near_*` bit would be decided by a band nobody measured
    /// -- the invention `CLAUDE.md` §3 rule 1 forbids and the whole reason this
    /// module holds a sentinel instead of a default. A [`crate::ConditionMask`]
    /// carries no record of the band that set its bits, so nothing downstream
    /// could ever find it.
    ///
    /// Both refusals are checked through **both** bases, because the base is
    /// precisely the argument the arm is declining to attach; and the accepting
    /// arm is checked in the same loop, so this cannot pass by refusing
    /// everything.
    #[test]
    fn a_width_with_a_base_is_refused_by_the_same_law_as_a_bare_one() {
        for base in [Base::SessionRange, Base::CprWidth] {
            assert_eq!(
                Tolerance::from_milli_on(base, UNPINNED),
                Err(VocabError::ToleranceUnpinned),
                "the sentinel still means no measurement fixed the band, and \
                 saying which quantity it would have been a fraction of does \
                 not fix it"
            );
            for milli in [-1i64, -500, i64::MIN + 1] {
                assert_eq!(
                    Tolerance::from_milli_on(base, milli),
                    Err(VocabError::ToleranceNegative { milli }),
                    "the refusal has to survive the base being added, and it \
                     still has to name the width it was offered"
                );
            }
            // The other arm, in the same loop: a legal width DOES come back
            // wearing the base it was handed. Without this the four assertions
            // above would be satisfied by a constructor that refuses
            // everything, which is the failure mode a refusal test invites.
            with_pin(Tolerance::from_milli_on(base, 0), |exact| {
                assert_eq!(exact.milli(), 0, "zero is a band: exact equality");
                assert_eq!(
                    exact.base(),
                    Some(base),
                    "the base the caller named is the base the value carries"
                );
            });
        }
    }

    /// [`lesser`] is reached only from a `const` initializer, so nothing ever
    /// executed it and neither side of its comparison was covered.
    ///
    /// The compile-time assertion on [`SMALLEST_LADDER_GAP`] would catch an
    /// inverted comparison **today**, because the ladder's smallest gap is 118
    /// and its largest is 618 -- but only while that assertion names a number
    /// that tells the two apart. This states the contract where the function is,
    /// including the equal case, which no fold over the ladder reaches.
    #[test]
    fn lesser_returns_the_smaller_width_whichever_side_it_is_on() {
        assert_eq!(lesser(118, 618), 118, "the smaller argument came first");
        assert_eq!(lesser(618, 118), 118, "the smaller argument came second");
        assert_eq!(lesser(7, 7), 7, "equal widths must not depend on the order");
        assert_eq!(
            lesser(-1, 0),
            -1,
            "the fold must not assume both sides are positive: a rung pair in \
             descending order produces a negative difference, and the `gap > 0` \
             assertion on SMALLEST_LADDER_GAP can only refuse it if that \
             difference is what comes back"
        );
    }

    #[test]
    fn the_band_is_relative_to_the_range_and_not_to_the_level() {
        // A 200.00-point session on NIFTY, in paisa. At the pinned width the
        // band is one hundredth of that span: 200 paisa, or 2.00 points.
        with_pin(pinned_fib(), |tol| {
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
        });
    }

    /// The pivot band is **half the CPR width**, which is the claim
    /// [`TOL_PIVOT_MILLI`]'s documentation makes and the premise position 6 was
    /// retired on.
    ///
    /// 500 thousandths of the range is the range halved, so with the CPR width
    /// as the range the band edges land exactly on `bc` and `tc` and
    /// `near_pivot_p` is `inside_cpr`. Nothing tested that the two constants
    /// mean what the paragraph says: at any other width the retirement of
    /// position 6 stops being true, and an index is never reissued, so it cannot
    /// be undone.
    #[test]
    fn the_pivot_band_is_exactly_half_the_cpr_width() {
        with_pin(pinned_pivot(), |tol| {
            let cpr_width = 1_000i64;
            let half = cpr_width / 2;
            let pivot = 2_500_000i64;
            assert!(tol.covers(pivot, pivot, cpr_width));
            assert!(
                tol.covers(pivot + half, pivot, cpr_width),
                "tc sits on the band's edge and is inside it"
            );
            assert!(
                tol.covers(pivot - half, pivot, cpr_width),
                "bc sits on the band's edge and is inside it"
            );
            assert!(
                !tol.covers(pivot + half + 1, pivot, cpr_width),
                "one paisa past tc is outside the CPR and outside the band"
            );
            assert!(
                !tol.covers(pivot - half - 1, pivot, cpr_width),
                "one paisa past bc is outside the CPR and outside the band"
            );
        });
    }

    /// A circuit-frozen session has no range, so every `near_*` bit is false
    /// rather than "probably" -- `docs/03-vocabulary.md` §4.
    #[test]
    fn a_zero_or_negative_range_covers_nothing() {
        with_pin(pinned_fib(), |tol| {
            assert!(!tol.covers(100, 100, 0));
            assert!(!tol.covers(100, 100, -1));
            assert!(!tol.covers(100, 100, i64::MIN));
        });
    }

    #[test]
    fn a_level_at_the_edge_of_the_type_does_not_overflow() {
        with_pin(Tolerance::from_milli(i64::MAX), |tol| {
            assert!(tol.covers(i64::MIN, i64::MAX, i64::MAX));
            assert!(tol.covers(i64::MAX, i64::MIN, i64::MAX));
        });
        with_pin(Tolerance::from_milli(0), |tight| {
            assert!(!tight.covers(i64::MIN, i64::MAX, i64::MAX));
        });
    }

    #[test]
    fn a_tolerance_carries_its_own_value() {
        with_pin(Tolerance::from_milli(7), |a| {
            assert_eq!(
                Tolerance::from_milli(7),
                Ok(a),
                "two pins of the same width are the same tolerance"
            );
            assert_ne!(
                Tolerance::from_milli(8),
                Ok(a),
                "two pins of different widths are not"
            );
            assert_eq!(a.milli(), 7);
            assert!(format!("{a:?}").contains('7'));
        });
    }
}
