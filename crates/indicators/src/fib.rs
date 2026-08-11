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
/// Six positions rather than eleven, and that is the shipped table's shape, not a
/// choice made here: 69 and 70 arrived with the original vocabulary and 106–109
/// were appended for the extensions. The retracement rungs 0.382, 0.5 and 0.618 in
/// this direction coincide with 0.618, 0.5 and 0.382 in the other, which already
/// have positions.
pub const PREV_DAY_UP: [(i32, u16); 6] = [
    (236, 69),
    (786, 70),
    (1272, 106),
    (1618, 107),
    (2000, 108),
    (2618, 109),
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
        if let Ok(next) = vocab::table::set_near(mask, *index, tolerance, close, level, range) {
            mask = next;
        }
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
    pub fn push_completed_session(&mut self, high: i64, low: i64) {
        if let Some(slot) = self.slots.get_mut(self.next) {
            *slot = (high, low);
        }
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
        let range = high.saturating_sub(low);
        emit(mask, &PREV5_UP, low, range, Direction::Up, close, tolerance)
    }
}

/// Every position this module can set.
#[must_use]
pub fn positions() -> [u16; 26] {
    let mut out = [0u16; 26];
    let mut i = 0;
    for (_, index) in PREV_DAY_DOWN {
        if let Some(slot) = out.get_mut(i) {
            *slot = index;
        }
        i += 1;
    }
    for (_, index) in PREV_DAY_UP {
        if let Some(slot) = out.get_mut(i) {
            *slot = index;
        }
        i += 1;
    }
    for (_, index) in PREV5_UP {
        if let Some(slot) = out.get_mut(i) {
            *slot = index;
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tol() -> Tolerance {
        let Ok(t) = vocab::tolerance::pinned_fib() else {
            unreachable!("the fib width is pinned")
        };
        t
    }

    fn levels(h: i64, l: i64, c: i64) -> DailyLevels {
        let Ok(x) = DailyLevels::from_previous_session(h, l, c) else {
            unreachable!("this fixture session is sane")
        };
        x
    }

    /// Every position is live, is `Kind::Near`, and appears exactly once.
    #[test]
    fn the_positions_agree_with_the_vocabulary() {
        let mut seen = std::collections::BTreeSet::new();
        for index in positions() {
            assert!(seen.insert(index), "position {index} appears twice");
            let Some(def) = vocab::table::definition(index) else {
                unreachable!("position {index} is not in the table")
            };
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
        assert_eq!(seen.len(), 26);
    }

    /// The two tombstoned rungs are NOT in this module's plan, and the positions
    /// that decide those two prices belong to `crate::daily`.
    #[test]
    fn the_tombstoned_rungs_are_absent() {
        let owned = positions();
        assert!(!owned.contains(&19), "19 near_fib_0 is a tombstone");
        assert!(!owned.contains(&25), "25 near_fib_100 is a tombstone");
        assert!(!owned.contains(&71), "71 near_fib_424 is an orphan");
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
            assert_eq!(p5.extremes(), None, "answered with {} sessions", k + 1);
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
        let Some((high, low)) = p5.extremes() else {
            unreachable!("five sessions are present")
        };
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
        assert_eq!(run(), run());
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
}
