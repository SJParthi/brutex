//! The Fibonacci ladders anchored on completed sessions.
//!
//! **27 vocabulary positions.** Three ladders, all frozen before today's first
//! bar prints, so none can repaint and none can look ahead:
//!
//! | Ladder | Anchor | Positions | Live |
//! |---|---|---|---:|
//! | previous day, measured DOWN from the high | yesterday's H and L | 19–29 | 9 |
//! | previous day, measured UP from the low | the same H and L | 69–71, 106–109 | 7 |
//! | the last five completed sessions, UP from the low | five sessions' extremes | 110–120 | 11 |
//!
//! # Two ladders on one range, and why that is not a duplication
//!
//! The shipped vocabulary carries both directions on yesterday's range, and they
//! are **not** the same set of prices. Downward rung `r` and upward rung `1 − r`
//! coincide, so `0.382` down equals `0.618` up — but the ladders extend past 1.0
//! in opposite directions, and a mask that requires "0.618 measured up" says
//! something a mask requiring "0.382 measured down" does not: the anchor it was
//! measured from. Positions 19 and 25 were retired precisely because they were the
//! two that coincided with `near_pdh` and `near_pdl`.
//!
//! # The band
//!
//! Both previous-day ladders and the five-session ladder are fractions of a
//! session RANGE, so all three take `vocab::tolerance::TOL_FIB_MILLI` — the width
//! D-0076 measured. That is the same base the current-session ladder uses and a
//! different base from the pivot bands, which are fractions of the CPR width.
//!
//! # One level, one test
//!
//! Each rung's level is materialised once, as an `i64`, and
//! `vocab::table::set_near` is the only thing that decides whether the close sits
//! inside the band around it.
//!
//! It used to be two decisions, and this section used to advertise the first of
//! them: an exact cross-multiplied comparison against the UNROUNDED level, which
//! needs no division. The level then handed to `vocab` was the TRUNCATED one, and
//! `set_near` decided again against that. The two agree everywhere except at the
//! band edge, where they are offset by the truncation remainder — under one paisa,
//! and silent in both directions. A rung this module had accepted was dropped
//! because `set_near` returns the mask unchanged whether the close was outside the
//! band or the position was refused; and a close inside the band of the level
//! `vocab` was actually told about never got asked, because the exact test had
//! already skipped the rung.
//!
//! [`crate::CurDayFib`] was corrected the same way — its `rung_level` carries the
//! measurement — and [`crate::gap`] is written that way from the start. The price
//! is one division per rung, by a literal 1000, over ladders whose length is fixed
//! at compile time.

use vocab::{ConditionMask, Tolerance};

use crate::daily::DailyLevels;

/// The eleven rungs, as exact integer numerators over 1000.
pub const RUNGS: [i32; 11] = [0, 236, 382, 500, 618, 786, 1000, 1272, 1618, 2000, 2618];

/// The previous-day ladder measured DOWN from yesterday's high.
///
/// `(rung, position)`. Rungs 0 and 1000 are absent: positions 19 and 25 are
/// tombstones, because rung 0 of a high-anchored ladder IS the high (17
/// `near_pdh`) and rung 1.0 IS the low (18 `near_pdl`). Those two prices are still
/// decided — by `crate::daily`, at the positions that own them.
pub const PREV_DAY_DOWN: [(i32, u16); 9] = [
    (236, 20),
    (382, 21),
    (500, 22),
    (618, 23),
    (786, 24),
    (1272, 26),
    (1618, 27),
    (2000, 28),
    (2618, 29),
];

/// The previous-day ladder measured UP from yesterday's low.
///
/// Seven positions rather than eleven, and that is the shipped table's shape, not a
/// choice made here: 69 and 70 arrived with the original vocabulary and 106–109
/// were appended for the extensions. The retracement rungs 0.382, 0.5 and 0.618 in
/// this direction coincide with 0.618, 0.5 and 0.382 in the other, which already
/// have positions.
///
/// # Rung 4.236 was refused, and refusing it was arbitrary
///
/// Position 71 `near_fib_424` sat uncomputed with a test calling it "an orphan",
/// while the vocabulary's own comment beside it reads **"Shipped."** The stated
/// reason was that no source defines the rung — but 4.236 is a member of the same
/// classical extension set as the four rungs this ladder already carries:
///
/// ```text
/// 1.272   1.414   1.618 = phi   2.0   2.618 = phi^2   3.618   4.236
/// ^shipped        ^shipped      ^shipped ^shipped             ^was refused
/// ```
///
/// Shipping 2.618 and refusing 4.236 is not a rule, it is an accident of which
/// positions the table happened to allocate first. Either both are conventional or
/// neither is, and 2.618 is already in three ladders. So 4.236 is computed here and
/// the orphan is gone.
pub const PREV_DAY_UP: [(i32, u16); 7] = [
    (236, 69),
    (786, 70),
    (1272, 106),
    (1618, 107),
    (2000, 108),
    (2618, 109),
    (4236, 71),
];

/// The five-session ladder, measured UP from the five-session low. All eleven
/// rungs, positions 110–120 in rung order.
pub const PREV5_UP: [(i32, u16); 11] = [
    (0, 110),
    (236, 111),
    (382, 112),
    (500, 113),
    (618, 114),
    (786, 115),
    (1000, 116),
    (1272, 117),
    (1618, 118),
    (2000, 119),
    (2618, 120),
];

/// Which way a ladder is measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    /// `level = anchor - rung * range`, anchored at the high.
    Down,
    /// `level = anchor + rung * range`, anchored at the low.
    Up,
}

/// `(p · span).div_euclid(1000)`, exactly, dividing in 64 bits whenever the
/// product fits.
///
/// Every rung on every ladder in this crate is a per-mille numerator applied to
/// a session span, and the product is formed in `i128` so that no span can
/// overflow it. Dividing that `i128` is a call into the compiler's runtime
/// library (`__divti3`), while dividing an `i64` by the literal `1000`
/// compiles to a multiply and a shift. A real
/// rung times a real session range fits `i64` by several orders of magnitude,
/// so the wide division is kept only for the spans that need it.
///
/// # Measured, because gate 8 refused the fold
///
/// `runner`'s C-R-04 read 5,284 floors against a budget of 5,000 on CI on
/// 2026-09-20. Sampling that fixture on an arm64 laptop put 17.8% of all
/// sampled time in 128-bit division, most of it reached from this module's
/// ladders, [`crate::CurDayFib`] and [`crate::gap`], which now route through
/// here; afterwards it was 2.3%. Together with [`crate::pattern`]'s
/// compile-time masks this took C-R-04's own row from 959,919 to 703,513 ps
/// per bar on that laptop, −26.7%. The split between the two changes was
/// read from an unsaved scratch harness and is not claimed. D-0677, D-0680.
///
/// # Exact on every input
///
/// The divisor is a positive literal, so the narrow division cannot overflow,
/// and Euclidean division has one answer whichever width computes it.
/// `per_mille_is_the_wide_division_on_both_sides_of_the_narrow_range` pins it
/// against the `i128` expression it replaced.
#[must_use]
pub(crate) fn per_mille(p: i32, span: i128) -> i128 {
    let product = i128::from(p) * span;
    match i64::try_from(product) {
        Ok(narrow) => i128::from(narrow.div_euclid(1000)),
        Err(_) => product.div_euclid(1000),
    }
}

/// The price of rung `p` on this ladder — `anchor ∓ (p/1000)·R` — or `None` when
/// it leaves `i64`.
///
/// Materialised ONCE, so that the only test ever applied to a rung is the one
/// `vocab::table::set_near` makes against this exact integer. The pair of a
/// separate exact test and a separate materialisation is what the module doc
/// describes, and it disagreed with itself at the band edge.
///
/// A level outside the type is `None` and the rung is skipped. It is not clamped:
/// `i64::MAX` is a price a close can actually sit on, so a clamp fires the bit
/// against a level the ladder never reached, and an `as` cast wraps it to the far
/// end of the type where it is wrong for a different close.
///
/// `div_euclid` rather than `/`, matching [`crate::CurDayFib`]'s `rung_level`. Every
/// rung numerator is non-negative and a real session's range is positive, so the two
/// round identically on every input this ladder can be reached with; naming the
/// rounding settles the sign cases rather than leaving them to an operand.
fn rung_level(anchor: i64, p: i32, range: i64, direction: Direction) -> Option<i64> {
    let step = per_mille(p, i128::from(range));
    let level = match direction {
        Direction::Down => i128::from(anchor) - step,
        Direction::Up => i128::from(anchor) + step,
    };
    i64::try_from(level).ok()
}

/// Set one ladder, deciding each rung ONCE and recording it through `vocab`.
///
/// # There is no range guard here, and there was one
///
/// `if range <= 0 { return mask }` stood at the top. `Tolerance::covers`, reached
/// through `set_near` below, refuses a non-positive range before it looks at
/// anything else — so the guard decided nothing that was not already decided, and a
/// mutation deleting it survives every test in this crate. The refusal stays with
/// the band. [`Prev5::bits`] keeps ITS guard, because that one is a different
/// refusal: it declines a five-session span that does not fit `i64` at all, and no
/// arithmetic downstream can tell a saturated span from a real one.
fn emit(
    mut mask: ConditionMask,
    rungs: &[(i32, u16)],
    anchor: i64,
    range: i64,
    direction: Direction,
    close: i64,
    tolerance: Tolerance,
) -> ConditionMask {
    for (p, index) in rungs {
        let Some(level) = rung_level(anchor, *p, range, direction) else {
            continue;
        };
        // `unwrap_or(mask)` and not `if let Ok(next)`: `set_near` refuses only a
        // position that is absent, retired, void or `Kind::Plain`, and
        // `the_positions_agree_with_the_vocabulary` proves that none of the 27 this
        // module names is any of those. An `if let` would therefore write an else
        // branch no test can take — `cargo llvm-cov` counts it as a region that never
        // runs, and a region that cannot run is a region nobody can be held to. The
        // behaviour is identical: a refusal leaves the bit clear, which is
        // `docs/03-vocabulary.md` §4, and it is what `crate::daily` writes at its three
        // set sites.
        mask = vocab::table::set_near(mask, *index, tolerance, close, level, range).unwrap_or(mask);
    }
    mask
}

/// Availability uses the same integer level and vocabulary band admission as
/// truth. Testing the level against itself certifies a Boolean answer only
/// when the range is positive, the tolerance family is correct and the rung
/// fits the price type. An unavailable level can never make NOT a signal.
fn known_ladder(
    mut known: ConditionMask,
    rungs: &[(i32, u16)],
    anchor: i64,
    range: i64,
    direction: Direction,
    tolerance: Tolerance,
) -> ConditionMask {
    for (p, index) in rungs {
        let Some(level) = rung_level(anchor, *p, range, direction) else {
            continue;
        };
        known =
            vocab::table::set_near(known, *index, tolerance, level, level, range).unwrap_or(known);
    }
    known
}

/// Both previous-day ladders, for one closing price. 16 positions.
///
/// **Sixteen, and this said fifteen.** The bullish ladder carries seven rungs, not
/// six: 4.236 was computed and position 71 stopped being an orphan, and the two
/// counts in this file were not moved with it. `POSITION_COUNT` is 27 and the
/// `const` assertion beside it has been checking 9 + 7 + 11 the whole time.
///
/// # Cost
///
/// Sixteen levels against a range fixed before the session opened, and the sixteen
/// band tests `vocab` makes against them. A count fixed at compile time, no
/// allocation, one division per rung by a literal 1000.
#[must_use]
pub fn prev_day_bits(levels: &DailyLevels, close: i64, tolerance: Tolerance) -> ConditionMask {
    let range = levels.pdh().saturating_sub(levels.pdl());
    let mask = ConditionMask::ZERO;
    let mask = emit(
        mask,
        &PREV_DAY_DOWN,
        levels.pdh(),
        range,
        Direction::Down,
        close,
        tolerance,
    );
    emit(
        mask,
        &PREV_DAY_UP,
        levels.pdl(),
        range,
        Direction::Up,
        close,
        tolerance,
    )
}

/// Which previous-day Fibonacci predicates have a sound true or false answer.
/// The same completed daily reference and 16 checked rungs serve both paths.
pub(crate) fn prev_day_known(levels: &DailyLevels, tolerance: Tolerance) -> ConditionMask {
    let range = levels.pdh().saturating_sub(levels.pdl());
    let known = known_ladder(
        ConditionMask::ZERO,
        &PREV_DAY_DOWN,
        levels.pdh(),
        range,
        Direction::Down,
        tolerance,
    );
    known_ladder(
        known,
        &PREV_DAY_UP,
        levels.pdl(),
        range,
        Direction::Up,
        tolerance,
    )
}

/// The last five **completed** sessions' extremes.
///
/// # Why a five-slot ring and not a sliding window over bars
///
/// A trailing window counted in bars needs a monotonic deque and gives amortised
/// Measured by `C-I-01` and `C-I-04`, in `crates/indicators/benches/ratio.rs`:
/// the per-candle cost is flat at 1,000, 50,000 and 200,000 candles folded, and a
/// session rollover costs what a mid-session candle costs.
///
/// O(1) with an O(window) worst case on a single bar. Counted in **sessions** it
/// needs five slots and is worst-case O(1) — five comparisons, no deque, no
/// amortisation. Where an exact worst-case-constant alternative exists, §3 rule 4
/// asks for it rather than the average.
///
/// # Static means frozen at the open
///
/// The five sessions are the five **completed** ones. Today is not among them, so
/// the ladder does not move during the day — the distinction `docs/09-design-sources.md`
/// draws between a static and a rolling window, and the one a "frozen" ladder can
/// get wrong by freezing on data through today's close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prev5 {
    /// `(high, low)` per completed session, oldest first among the filled slots.
    slots: [(i64, i64); 5],
    filled: usize,
    next: usize,
}

const _: () = assert!(core::mem::size_of::<Prev5>() <= 112);

impl Default for Prev5 {
    fn default() -> Self {
        Self::new()
    }
}

impl Prev5 {
    /// An empty ring.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slots: [(0, 0); 5],
            filled: 0,
            next: 0,
        }
    }

    /// Record one **completed** session's extremes.
    ///
    /// The caller decides what a session is; this module only counts five of them.
    /// Passing today's partial extremes would make the ladder rolling rather than
    /// static, which is a different family.
    ///
    /// # DESTRUCTURED, not looked up
    ///
    /// `slots.get_mut(self.next)` cannot answer `None` while the cursor below is the
    /// only writer of `next` and wraps at five — so its `None` arm was a branch
    /// `cargo llvm-cov` counts and no passing test can enter. A match over the five
    /// slots is TOTAL: every arm writes a real slot, nothing is dead, and the write is
    /// still one move. `clippy::indexing_slicing` is denied in this workspace, so an
    /// index was never the alternative; this is the idiom `vocab::tolerance` uses on
    /// the rung ladder for the same reason.
    pub fn push_completed_session(&mut self, high: i64, low: i64) {
        let [s0, s1, s2, s3, s4] = &mut self.slots;
        let slot = match self.next {
            0 => s0,
            1 => s1,
            2 => s2,
            3 => s3,
            _ => s4,
        };
        *slot = (high, low);
        self.next = (self.next + 1) % self.slots.len();
        if self.filled < self.slots.len() {
            self.filled += 1;
        }
    }

    /// How many completed sessions are recorded, up to five.
    #[must_use]
    pub const fn filled(&self) -> usize {
        self.filled
    }

    /// The extreme high and low across the recorded sessions.
    ///
    /// `None` until all five are present. A four-session ladder is a different
    /// ladder, and answering with one would be the quiet substitution §4 bans.
    #[must_use]
    pub fn extremes(&self) -> Option<(i64, i64)> {
        if self.filled < self.slots.len() {
            return None;
        }
        let mut high = i64::MIN;
        let mut low = i64::MAX;
        for (h, l) in self.slots {
            if h > high {
                high = h;
            }
            if l < low {
                low = l;
            }
        }
        Some((high, low))
    }

    /// The eleven positions, for one closing price.
    ///
    /// # Cost
    ///
    /// Five comparisons to find the extremes, eleven levels, and the eleven band
    /// tests `vocab` makes against them. Every count here is fixed at compile time.
    #[must_use]
    pub fn bits(&self, close: i64, tolerance: Tolerance) -> ConditionMask {
        let mask = ConditionMask::ZERO;
        let Some((high, low)) = self.extremes() else {
            return mask;
        };
        // `checked_sub`, not `saturating_sub`. The window's high and low come from two
        // DIFFERENT sessions, so nothing that accepted either one has checked their span:
        // `DailyLevels::from_previous_session` refuses `Unusable::RangeOverflows` for one
        // session's own range, which is why the previous-day ladder above can subtract
        // without a guard, and there is no such gate on a pair. Saturating pinned a
        // 1.84e19 span at `i64::MAX` and every rung is a fraction of the span, so the
        // whole ladder was laid out over half the range it names — rung 0.236 sat 2.17e18
        // paisa from the price position 111 claims. Refusing the ladder is the answer that
        // claims nothing. A non-positive range reaching `emit` sets nothing either, because
        // the band refuses one — but that refusal cannot tell a saturated span from a real
        // one, so it is no substitute for this check.
        let Some(range) = high.checked_sub(low) else {
            return mask;
        };
        emit(mask, &PREV5_UP, low, range, Direction::Up, close, tolerance)
    }

    /// Availability for the same five completed sessions and checked span.
    /// A cold, nonpositive or overflowing reference remains unknown.
    pub(crate) fn known(&self, tolerance: Tolerance) -> ConditionMask {
        let Some((high, low)) = self.extremes() else {
            return ConditionMask::ZERO;
        };
        let Some(range) = high.checked_sub(low) else {
            return ConditionMask::ZERO;
        };
        known_ladder(
            ConditionMask::ZERO,
            &PREV5_UP,
            low,
            range,
            Direction::Up,
            tolerance,
        )
    }
}

/// The length is asserted against the three ladders rather than written twice.
/// Adding a rung to any ladder without widening this array is a compile error.
const POSITION_COUNT: usize = PREV_DAY_DOWN.len() + PREV_DAY_UP.len() + PREV5_UP.len();
const _: () = assert!(POSITION_COUNT == 27, "the ladders and positions() disagree");

/// Every position the three Fibonacci ladders own, in ladder order.
///
/// Down ladder, then up, then the five-session ladder — nine, seven and eleven.
/// Derived from the ladders themselves via [`POSITION_COUNT`], so adding a rung
/// without widening the array is a compile error rather than a silent truncation.
#[must_use]
pub fn positions() -> [u16; 27] {
    let mut out = [0u16; 27];
    // ZIPPED against the array, not counted into it. `out.get_mut(i)` cannot answer
    // `None` while `i` walks 27 slots of a 27-slot array, so its `None` arm was three
    // branches `cargo llvm-cov` counts and no passing test can enter. A zip is total in
    // that direction, and [`POSITION_COUNT`] keeps it total in the other: a ladder that
    // grows a rung without widening the array is a compile error rather than a slot
    // left at its initialiser. `crate::daily::positions` is written the same way, and
    // `the_positions_are_the_three_ladders_concatenated_in_order` pins the result.
    let ladders = PREV_DAY_DOWN.into_iter().chain(PREV_DAY_UP).chain(PREV5_UP);
    for (slot, (_, index)) in out.iter_mut().zip(ladders) {
        *slot = index;
    }
    out
}

#[cfg(test)]
// `expect` in a fixture, and the reason is not taste. `let Ok(x) = f() else {
// unreachable!() }` expands to a panic written IN THIS CRATE, so `cargo llvm-cov`
// records a region that no test can ever execute while the code is correct — and a
// region that cannot run is one nobody can be held to. `Result::expect` panics inside
// the standard library, which is not instrumented, and refuses just as loudly. The
// workspace denies `expect_used` for library code, where a panic is a real defect; the
// same allow sits on the test module of `crate::daily`, `vocab::table` and a dozen
// others.
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn tol() -> Tolerance {
        vocab::tolerance::pinned_fib().expect("the fib width is pinned")
    }

    fn levels(h: i64, l: i64, c: i64) -> DailyLevels {
        DailyLevels::from_previous_session(h, l, c).expect("this fixture session is sane")
    }

    #[test]
    fn known_five_session_span_refuses_overflow_and_nonpositive_ranges() {
        let mut overflow = Prev5::new();
        overflow.push_completed_session(i64::MAX, i64::MAX);
        for _ in 0..4 {
            overflow.push_completed_session(i64::MIN, i64::MIN);
        }
        assert_eq!(overflow.known(tol()), ConditionMask::ZERO);
        for (high, low) in [(100, 100), (99, 100)] {
            let mut ring = Prev5::new();
            for _ in 0..5 {
                ring.push_completed_session(high, low);
            }
            assert_eq!(ring.known(tol()), ConditionMask::ZERO);
        }
        let mut valid = Prev5::new();
        for _ in 0..5 {
            valid.push_completed_session(1_000, 100);
        }
        assert_eq!(valid.known(tol()).popcount(), 11);
    }

    /// Every position is live, is `Kind::Near`, and appears exactly once.
    #[test]
    fn the_positions_agree_with_the_vocabulary() {
        let mut seen = std::collections::BTreeSet::new();
        for index in positions() {
            assert!(seen.insert(index), "position {index} appears twice");
            // Asserted before it is unwrapped, so the failure still names the position
            // that is missing: `expect` takes a string and cannot interpolate `index`,
            // and a `let ... else { unreachable!() }` would put the refusal in a region
            // this crate owns and no passing test can enter.
            let found = vocab::table::definition(index);
            assert!(found.is_some(), "position {index} is not in the table");
            let def = found.expect("asserted to be Some on the line above");
            assert!(
                vocab::table::is_live(index),
                "position {index} (`{}`) is not live",
                def.name,
            );
            assert_eq!(
                def.kind,
                vocab::Kind::Near,
                "position {index} (`{}`) is not a banded position",
                def.name,
            );
        }
        assert_eq!(seen.len(), POSITION_COUNT);
    }

    /// The two tombstoned rungs are NOT in this module's plan, and the positions
    /// that decide those two prices belong to `crate::daily`.
    #[test]
    fn the_tombstoned_rungs_are_absent() {
        let owned = positions();
        assert!(!owned.contains(&19), "19 near_fib_0 is a tombstone");
        assert!(!owned.contains(&25), "25 near_fib_100 is a tombstone");
        // 71 is NO LONGER an orphan: 4.236 is computed on the bullish anchor. See
        // PREV_DAY_UP for why refusing it while shipping 2.618 was arbitrary.
        assert!(
            owned.contains(&71),
            "71 near_fib_424 is rung 4.236 of the bullish ladder and must be computed"
        );
        // And rung 0 / rung 1.0 are absent from the DOWN ladder for that reason.
        for (p, _) in PREV_DAY_DOWN {
            assert!(p != 0 && p != 1000, "rung {p} should have no position here");
        }
    }

    /// Downward rung `r` and upward rung `1 − r` are the same price. The identity
    /// the retirements rest on, checked rather than asserted.
    #[test]
    fn a_downward_rung_equals_its_upward_complement() {
        let x = levels(11_000, 10_000, 10_500);
        let range = x.pdh() - x.pdl();
        for (down, up) in [(236, 764), (382, 618), (500, 500), (786, 214)] {
            let a = i128::from(x.pdh()) - i128::from(down) * i128::from(range) / 1000;
            let b = i128::from(x.pdl()) + i128::from(up) * i128::from(range) / 1000;
            assert_eq!(a, b, "down {down} and up {up} are different prices");
        }
    }

    /// Rung 0.5 fires when the close sits exactly at the midpoint, on both ladders.
    #[test]
    fn the_half_rung_fires_at_the_midpoint() {
        let x = levels(11_000, 10_000, 10_500);
        let mid = 10_500;
        let m = prev_day_bits(&x, mid, tol());
        assert!(
            m.get(22),
            "the downward 0.5 rung (22) did not fire at the midpoint"
        );
    }

    /// A zero-range previous session emits nothing and never divides.
    #[test]
    fn a_flat_previous_session_emits_nothing() {
        let x = levels(10_000, 10_000, 10_000);
        let m = prev_day_bits(&x, 10_000, tol());
        assert!(m.is_empty(), "a flat session produced a rung");
    }

    /// Only the 27 positions this module owns are ever set.
    #[test]
    fn nothing_outside_the_twenty_seven_positions_is_set() {
        let owned = positions();
        let x = levels(2_510_000, 2_490_000, 2_500_000);
        let mut p5 = Prev5::new();
        for k in 0..5 {
            p5.push_completed_session(2_520_000 + k * 100, 2_480_000 - k * 100);
        }
        let mut union = ConditionMask::ZERO;
        for step in -600..600 {
            let close = 2_500_000 + step * 61;
            union = union.union(&prev_day_bits(&x, close, tol()));
            union = union.union(&p5.bits(close, tol()));
        }
        for index in 0..ConditionMask::BITS {
            if union.get(index) {
                let as_u16 = u16::try_from(index).unwrap_or(u16::MAX);
                assert!(
                    owned.contains(&as_u16),
                    "position {index} is not this module's"
                );
            }
        }
    }

    /// The five-session ring answers nothing until all five are present.
    #[test]
    fn four_sessions_is_not_a_five_session_ladder() {
        let mut p5 = Prev5::new();
        for k in 0..4 {
            p5.push_completed_session(100 + k, 90 - k);
            // Counted into a local rather than added inside the message: an argument
            // computed only on failure is a region that only runs on failure, and the
            // message reads the same.
            let pushed = k + 1;
            assert_eq!(p5.extremes(), None, "answered with {pushed} sessions");
            assert!(p5.bits(95, tol()).is_empty());
        }
        p5.push_completed_session(104, 86);
        assert_eq!(p5.filled(), 5);
        assert_eq!(p5.extremes(), Some((104, 86)));
    }

    /// The ring keeps the LAST five and drops the sixth-oldest.
    #[test]
    fn the_ring_holds_exactly_the_last_five_sessions() {
        let mut p5 = Prev5::new();
        // A very wide first session that must fall out of the window.
        p5.push_completed_session(9_999, 1);
        for k in 0..5 {
            p5.push_completed_session(100 + k, 90 - k);
        }
        let (high, low) = p5.extremes().expect("five sessions are present");
        assert_eq!(
            high, 104,
            "the dropped session's high is still in the window"
        );
        assert_eq!(low, 86, "the dropped session's low is still in the window");
    }

    /// Same input, same bits, byte for byte.
    #[test]
    fn the_same_input_gives_the_same_bits() {
        let x = levels(2_510_000, 2_490_000, 2_500_000);
        let run = || {
            (0..400)
                .map(|k| prev_day_bits(&x, 2_500_000 + k * 47, tol()).words())
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run(), "the same bars must give the same bits");
        // The equality above holds for ANY deterministic body -- including
        // `bits() -> ConditionMask::ZERO` and any constant -- so it cannot see the one
        // mutation that matters. An audit applied exactly that mutation and it survived
        // ALL SIX of these idempotence tests, which between them are §3 rule 5's only
        // direct guard in this crate.
        //
        // This closes it without needing to know what the bits SHOULD be, which is what
        // the sibling tests in this module are for. A body that ignores its input emits
        // the same value on every bar, and these fixtures deliberately vary the bars. So:
        // the run must not be constant.
        let observed = run();
        assert!(
            observed
                .iter()
                .skip(1)
                .zip(observed.iter())
                .any(|(later, earlier)| later != earlier),
            "every bar produced an identical result, so this test would pass on a body \
             that ignores its input entirely"
        );
    }

    /// At most one rung per ladder can fire on a bar: two would need a numerator
    /// gap of at most twice the width, and the ladder's smallest gap is 118
    /// against a width of 10.
    #[test]
    fn at_most_one_rung_per_ladder_fires() {
        let x = levels(2_510_000, 2_490_000, 2_500_000);
        for step in -800..800 {
            let close = 2_500_000 + step * 29;
            let m = prev_day_bits(&x, close, tol());
            let down = PREV_DAY_DOWN
                .iter()
                .filter(|(_, i)| m.get(u32::from(*i)))
                .count();
            let up = PREV_DAY_UP
                .iter()
                .filter(|(_, i)| m.get(u32::from(*i)))
                .count();
            assert!(down <= 1, "{down} downward rungs fired at close {close}");
            assert!(up <= 1, "{up} upward rungs fired at close {close}");
        }
    }

    /// A non-positive range decides nothing, and the refusal is the vocabulary's.
    ///
    /// This replaces `near_refuses_a_zero_or_negative_range_without_measuring_it`, which
    /// tested a private exact-comparison helper that no longer exists — the rung is
    /// decided once now, and `vocab::table::set_near` is what decides it. Both refusals
    /// are still worth pinning and the reasons are unchanged: on a zero range the band
    /// collapses to the level itself, so rung 0 would otherwise report an exact hit on
    /// the anchor; on a NEGATIVE range — a corrupt record — the band is negative and no
    /// close can satisfy it, which is the right answer for a reason nobody wrote down
    /// unless a test states it.
    ///
    /// `emit` is called directly because a negative range reaches it through neither
    /// public entry point: `DailyLevels` refuses `pdh < pdl`, and [`Prev5::bits`] refuses
    /// a five-session span that does not subtract.
    #[test]
    fn a_non_positive_range_decides_nothing() {
        let zero = emit(
            ConditionMask::ZERO,
            &PREV_DAY_DOWN,
            10_000,
            0,
            Direction::Down,
            10_000,
            tol(),
        );
        assert!(
            zero.is_empty(),
            "a zero-range ladder called the anchor an exact hit"
        );
        let negative = emit(
            ConditionMask::ZERO,
            &PREV_DAY_UP,
            10_000,
            -1_000,
            Direction::Up,
            10_000,
            tol(),
        );
        assert!(negative.is_empty(), "a negative range decided a rung");
        // The same ladder on a real range DOES fire, so the two refusals above are the
        // range and not a fixture that could never have hit anything.
        let live = emit(
            ConditionMask::ZERO,
            &PREV_DAY_DOWN,
            10_000,
            1_000,
            Direction::Down,
            9_764,
            tol(),
        );
        assert!(
            live.get(20),
            "rung 0.236 of a 1,000-paisa range sits at 9_764 and the close is on it"
        );
    }

    /// The rung is decided against the level `vocab` is handed, and against no other.
    ///
    /// # The disagreement this reproduces, in exact numbers
    ///
    /// `pdh − pdl = 12_345` and rung 0.382: `382 × 12_345 = 4_715_790`, so the rung sits
    /// `4_715.790` paisa below the high, and the level the ladder can NAME — an `i64` of
    /// paisa, §7 — is `4_715` below it, at `2_507_630`. The band is ten thousandths of the
    /// range, 123.45 paisa.
    ///
    /// A close of `2_507_753` is 123 paisa above the named level and inside its band, and
    /// 123.79 paisa above the unrounded one, outside THAT band. The shape this replaces
    /// tested the unrounded level first, skipped the rung, and never asked `vocab` about
    /// the level it would have set: position 21 stayed clear on a close the vocabulary's
    /// own band covers. One paisa further out is outside both, which is the second
    /// assertion — without it this would pass on a ladder that fires everywhere.
    #[test]
    fn the_rung_is_decided_against_the_level_the_vocabulary_is_handed() {
        let x = levels(2_512_345, 2_500_000, 2_506_000);
        assert_eq!(
            x.pdh() - x.pdl(),
            12_345,
            "the fixture's range is the one the arithmetic above is about"
        );
        assert!(
            prev_day_bits(&x, 2_507_753, tol()).get(21),
            "a close inside the band of the level `vocab` was handed did not set rung \
             0.382, so something upstream is deciding it against a different level"
        );
        assert!(
            !prev_day_bits(&x, 2_507_754, tol()).get(21),
            "one paisa further out is outside the band around that same level, so the \
             edge asserted above is the band's and not a ladder that fires everywhere"
        );
    }

    /// A rung whose level leaves `i64` sets nothing, rather than being measured against
    /// an invented one.
    ///
    /// Reachable only from a corrupt record, and that is exactly why it is worth a test:
    /// the band is a fraction of the RANGE, so a range near the width of the type gives a
    /// band near 1% of it, and rung 4.236 of such a range sits past `i64::MAX` while a
    /// representable close is still inside that band. The level therefore has to exist
    /// before anything can be measured against it, and every alternative to the `try_from`
    /// invents one. A saturating clamp — the policy `crate::daily` takes on the pivot
    /// ladder — pins the level at `i64::MAX`, which is exactly where this close sits, so
    /// position 71 fires against a price the ladder never reached; an `as` cast wraps it
    /// to the far end of the type instead, where it is wrong for a different close.
    /// Skipping the rung is the only answer that claims nothing.
    #[test]
    fn a_rung_whose_level_leaves_the_type_is_skipped_and_not_wrapped() {
        // 4.236 * RANGE, exactly: the rung numerator times a thousandth of the range.
        const RANGE: i64 = 2_000_000_000_000_000_000;
        const STEP: i64 = 4236 * (RANGE / 1000);
        // Anchored so that the rung's level is i64::MAX + 1 exactly, and the close one
        // paisa under it — a residual of 1,000 against a bound of 10 * RANGE.
        let pdl = i64::MAX - STEP + 1;
        let x = levels(pdl + RANGE, pdl, pdl);
        assert_eq!(
            x.pdh() - x.pdl(),
            RANGE,
            "the fixture's range is the one meant"
        );

        let m = prev_day_bits(&x, i64::MAX, tol());
        assert!(
            !m.get(71),
            "rung 4.236's level is i64::MAX + 1 on this range: position 71 must stay \
             clear rather than be measured against a clamped or wrapped stand-in"
        );
        // And the rung below it, whose level DOES fit, fires from the same ladder — so
        // the clear bit above is the type boundary and not a ladder that never emits.
        let fits = pdl + 2618 * (RANGE / 1000);
        let n = prev_day_bits(&x, fits, tol());
        assert!(
            n.get(109),
            "rung 2.618's level fits the type and the close sits on it, so position 109 \
             must fire or this fixture proves nothing about the rung above it"
        );
    }

    /// A five-session span that does not fit `i64` sets NOTHING, rather than anchoring
    /// eleven rungs on a saturated one.
    ///
    /// The ring is fed one session at either end of the type. Each is a legal session —
    /// `DailyLevels` is not involved here, and nothing refuses a flat session at
    /// `-9.2e18` — but the WINDOW's span is 1.84e19, which `saturating_sub` pinned at
    /// `i64::MAX`. Every rung is a fraction of that span, so the ladder was laid out over
    /// half the range it claims: rung 0 still landed on the five-session low and fired
    /// position 110, and the saturated rung 0.236 sat at -7.02e18 while the true one is
    /// -4.86e18 — 2.17e18 paisa, or 21.7 billion rupees, from the price the bit names.
    #[test]
    fn a_five_session_span_that_leaves_the_type_sets_nothing() {
        const HI: i64 = 9_200_000_000_000_000_000;
        const LO: i64 = -9_200_000_000_000_000_000;

        let mut p5 = Prev5::new();
        p5.push_completed_session(HI, HI);
        for _ in 0..4 {
            p5.push_completed_session(LO, LO);
        }
        assert_eq!(
            p5.extremes(),
            Some((HI, LO)),
            "the fixture did not give the ring the straddling span this test is about"
        );
        // Rung 0 of an upward ladder IS the anchor, and the second close is the price the
        // saturated ladder called rung 0.236.
        for close in [LO, LO + 236 * (i64::MAX / 1000)] {
            assert!(
                p5.bits(close, tol()).is_empty(),
                "a five-session span that leaves i64 decided a rung at close {close}"
            );
        }

        // And a window spanning half as much — 9.2e18, which DOES fit — still fires rung
        // 0 on its own low, so the empty masks above are the span leaving the type and
        // not a ladder that never emits at this magnitude.
        let mut fits = Prev5::new();
        fits.push_completed_session(HI / 2, HI / 2);
        for _ in 0..4 {
            fits.push_completed_session(LO / 2, LO / 2);
        }
        assert!(
            fits.bits(LO / 2, tol()).get(110),
            "a representable 9.2e18 five-session span did not fire rung 0 on its own low"
        );
    }

    /// The default ring is the empty one, and an empty ring answers nothing.
    ///
    /// `Default` is derived by hand here and nothing in the crate calls it, so without
    /// this test a `Default` that filled the slots with zeros and set `filled` to five
    /// would compile and ship: `extremes()` would answer `(0, 0)`, a range of zero, and
    /// every five-session position would be decided against a session that never traded.
    #[test]
    fn the_default_ring_is_empty_and_answers_nothing() {
        let empty = Prev5::default();
        assert_eq!(
            empty,
            Prev5::new(),
            "`Prev5::default()` is not the empty ring `Prev5::new()` builds"
        );
        assert_eq!(empty.filled(), 0, "a default ring has recorded no session");
        assert_eq!(
            empty.extremes(),
            None,
            "an empty ring must refuse, not answer with its zero initialiser"
        );
        assert!(
            empty.bits(2_500_000, tol()).is_empty(),
            "an empty ring set a five-session position"
        );
    }

    /// [`positions`] is the three ladders concatenated, in ladder order.
    ///
    /// The array is filled by zipping the ladders against it, and the two ways that can
    /// go wrong are both silent. A zip that ran short would leave a slot at its `0`
    /// initialiser — and 0 is not one of the 27, so the mask would claim a position this
    /// module never computes. A chain in the wrong order would still hold 27 correct
    /// numbers, and `nothing_outside_the_twenty_seven_positions_is_set` would still pass,
    /// while the documented "down ladder, then up, then the five-session ladder" became
    /// false.
    #[test]
    fn the_positions_are_the_three_ladders_concatenated_in_order() {
        let expected: Vec<u16> = PREV_DAY_DOWN
            .into_iter()
            .chain(PREV_DAY_UP)
            .chain(PREV5_UP)
            .map(|(_, index)| index)
            .collect();
        assert_eq!(
            expected.len(),
            POSITION_COUNT,
            "the ladders no longer hold POSITION_COUNT rungs between them"
        );
        assert_eq!(
            positions().to_vec(),
            expected,
            "positions() is no longer the three ladders in ladder order"
        );
        assert!(
            !positions().contains(&0),
            "a slot was left at its zero initialiser, so the fill ran short of 27"
        );
    }

    /// `per_mille` is a speed change and nothing else: every answer must be the
    /// `i128` expression it replaced, on both sides of the `i64` boundary where
    /// it switches division widths.
    #[test]
    fn per_mille_is_the_wide_division_on_both_sides_of_the_narrow_range() {
        let top = i128::from(i64::MAX);
        let bottom = i128::from(i64::MIN);
        let spans = [
            0,
            1,
            -1,
            999,
            -999,
            1_000,
            -1_000,
            1_001,
            -1_001,
            2_500_000,
            -2_500_000,
            top,
            bottom,
            top + 1,
            bottom - 1,
            top * 2,
            bottom * 2,
            top - bottom,
        ];
        let rungs = [0, 1, -1, 236, 618, 1_000, 4_236, -4_236, i32::MAX, i32::MIN];
        for p in rungs {
            for span in spans {
                let wide = (i128::from(p) * span).div_euclid(1000);
                assert_eq!(per_mille(p, span), wide, "p = {p}, span = {span}");
            }
        }
        // A product exactly on either i64 bound is divided narrow, and one past
        // it wide; both land on the same floor the wide expression gives.
        assert_eq!(per_mille(1, top), 9_223_372_036_854_775);
        assert_eq!(per_mille(1, top + 1), 9_223_372_036_854_775);
        assert_eq!(per_mille(1, bottom), -9_223_372_036_854_776);
        assert_eq!(per_mille(1, bottom - 1), -9_223_372_036_854_776);
        // Floor, not truncation: a negative product rounds away from zero.
        assert_eq!(per_mille(618, -1), -1);
        assert_eq!(per_mille(-618, 1), -1);
    }
}
