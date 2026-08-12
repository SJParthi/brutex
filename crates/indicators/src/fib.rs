//! The Fibonacci ladders anchored on completed sessions.
//!
//! **26 vocabulary positions.** Three ladders, all frozen before today's first
//! bar prints, so none can repaint and none can look ahead:
//!
//! | Ladder | Anchor | Positions | Live |
//! |---|---|---|---:|
//! | previous day, measured DOWN from the high | yesterday's H and L | 19–29 | 9 |
//! | previous day, measured UP from the low | the same H and L | 69–70, 106–109 | 6 |
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
//! # No division on the evaluation path
//!
//! The rung test is cross-multiplied, exactly as in [`crate::CurDayFib`]: the
//! level is handed to `vocab::table::set_near` only so the band check happens in
//! the crate that owns it, and the decision itself never divides.

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

/// Is `close` within the band of rung `p` on this ladder?
///
/// `level = anchor ∓ (p/1000)·R`, so `|close − level| ≤ tol` becomes an exact
/// integer comparison once multiplied through by 1000. Two-sided rather than
/// through `abs()`: `i64::MIN.abs()` panics in debug and wraps in release, which
/// would make the two build profiles disagree and kill §3 rule 5.
fn near(
    close: i64,
    anchor: i64,
    p: i32,
    range: i64,
    direction: Direction,
    tolerance: Tolerance,
) -> bool {
    if range <= 0 {
        return false;
    }
    let c = i128::from(close);
    let a = i128::from(anchor);
    let rr = i128::from(range);
    let pp = i128::from(p);
    let residual = match direction {
        Direction::Down => 1000 * (c - a) + pp * rr,
        Direction::Up => 1000 * (c - a) - pp * rr,
    };
    let bound = i128::from(tolerance.milli()) * rr;
    -bound <= residual && residual <= bound
}

/// Set one ladder, deciding each rung by [`near`] and recording it through `vocab`.
fn emit(
    mut mask: ConditionMask,
    rungs: &[(i32, u16)],
    anchor: i64,
    range: i64,
    direction: Direction,
    close: i64,
    tolerance: Tolerance,
) -> ConditionMask {
    if range <= 0 {
        return mask;
    }
    for (p, index) in rungs {
        if !near(close, anchor, *p, range, direction, tolerance) {
            continue;
        }
        let step = i128::from(*p) * i128::from(range) / 1000;
        let level = match direction {
            Direction::Down => i128::from(anchor) - step,
            Direction::Up => i128::from(anchor) + step,
        };
        let Ok(level) = i64::try_from(level) else {
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

/// Both previous-day ladders, for one closing price. 15 positions.
///
/// # Cost
///
/// Fifteen cross-multiplied comparisons against a range fixed before the session
/// opened. A compile-time constant count, no allocation, no division on the
/// deciding path.
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
    /// Five comparisons to find the extremes plus eleven cross-multiplied rung
    /// tests. Both counts are compile-time constants.
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
        // claims nothing; `emit` treats a non-positive range the same way, for the same
        // reason.
        let Some(range) = high.checked_sub(low) else {
            return mask;
        };
        emit(mask, &PREV5_UP, low, range, Direction::Up, close, tolerance)
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

    /// Only the 26 positions this module owns are ever set.
    #[test]
    fn nothing_outside_the_twenty_six_positions_is_set() {
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

    /// [`near`] refuses a non-positive range itself, and does not lean on [`emit`]
    /// having checked first.
    ///
    /// Both functions guard the range, and `emit` is the only caller `near` has inside
    /// the library — so its own guard is reached from this test and nowhere else. The
    /// arithmetic is why the guard is not redundant all the same: `bound` is
    /// `tolerance.milli() * range`, so on a zero range the band collapses to
    /// `residual == 0` and rung 0 of a flat session would report itself as an exact hit
    /// on the anchor; on a NEGATIVE range — a corrupt record — the bound is negative,
    /// `-bound <= residual && residual <= bound` is unsatisfiable for every close, and
    /// the ladder would answer "nothing is near" for a reason nobody wrote down. A
    /// second caller added to this module would get the refusal, not the arithmetic.
    #[test]
    fn near_refuses_a_zero_or_negative_range_without_measuring_it() {
        // Exactly on the anchor, where the collapsed band would otherwise say "near".
        assert!(
            !near(10_000, 10_000, 0, 0, Direction::Down, tol()),
            "rung 0 of a zero-range ladder must be refused, not called an exact hit"
        );
        assert!(
            !near(10_000, 10_000, 500, -1_000, Direction::Up, tol()),
            "a negative range is a broken record and cannot decide a rung"
        );
        // The same rung on a real range DOES fire, so the two refusals above are the
        // range guard and not a fixture that could never have hit anything.
        assert!(
            near(10_000, 10_000, 0, 1_000, Direction::Down, tol()),
            "rung 0 measured down from the anchor IS the anchor"
        );
    }

    /// A rung whose level leaves `i64` sets nothing, rather than being measured against
    /// an invented one.
    ///
    /// Reachable only from a corrupt record, and that is exactly why it is worth a test:
    /// the band is a fraction of the RANGE, so a range near the width of the type gives a
    /// band near 1% of it, and rung 4.236 of such a range sits past `i64::MAX` while a
    /// representable close is still inside that band. `near` therefore says yes about a
    /// level that does not exist, and every alternative to the `try_from` invents one. A
    /// saturating clamp — the policy `crate::daily` takes on the pivot ladder — pins the
    /// level at `i64::MAX`, which is exactly where this close sits, so position 71 fires
    /// against a price the ladder never reached; an `as` cast wraps it to the far end of
    /// the type instead, where it is wrong for a different close. Skipping the rung is
    /// the only answer that claims nothing.
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
    /// numbers, and `nothing_outside_the_twenty_six_positions_is_set` would still pass,
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
}
