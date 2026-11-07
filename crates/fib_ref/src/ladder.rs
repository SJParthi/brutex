//! The Fibonacci ladder: running (A1) and frozen (A2), its direction, its
//! twelve rungs and the candle relations (PREVIEW-SPEC 6.1).
//!
//! * **A1, the running day ladder**: the session's highest high and lowest
//!   low so far. Candle `i` is judged against the ladder of candles
//!   `0..i-1`, then folds into it (FIB-I2).
//! * **Direction**: Up when the high is the later extreme, Down when the low
//!   is, Undetermined when one candle holds both; an Undetermined ladder is
//!   unused until a later one-sided extreme (FIB-I3). An extreme moves only on
//!   a strictly new high or low (the headline reading; the "latest" reading
//!   also moves the index on an equal extreme, see `ExtremeTies`).
//! * **Taint**: a hole folded into the ladder taints it for the session
//!   (the reading that admits less; `TaintScope::ExtremeCandles` is the
//!   other).
//! * **Rungs**, exact in milli-paise: on an Up leg a retracement `r` sits at
//!   `hi - r R` (0 at the high, 1 at the low) and an extension `x` at
//!   `lo + x R`, above the high; a Down leg mirrors both.

use crate::bars::Candle;

/// The twelve rungs in thousandths: retracements 0 .. 1, extensions 1.272 .. 3.
pub const RUNGS: [u32; 12] = [
    0, 236, 382, 500, 618, 786, 1000, 1272, 1618, 2000, 2618, 3000,
];

/// Rung 1.272 in thousandths.
pub const R1272: u32 = 1272;
/// Rung 1.618 in thousandths.
pub const R1618: u32 = 1618;

/// The leg's direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Leg {
    /// The high is the later extreme.
    Up,
    /// The low is the later extreme.
    Down,
    /// One candle holds both extremes.
    Undetermined,
}

impl Leg {
    /// Short name for the golden files.
    pub fn name(self) -> &'static str {
        match self {
            Leg::Up => "up",
            Leg::Down => "down",
            Leg::Undetermined => "undetermined",
        }
    }
}

/// How an equal extreme is read (ambiguity: the kit says "the later extreme"
/// without saying whether an equal print is later).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtremeTies {
    /// Only a strictly new high or low moves the extreme (headline).
    Strict,
    /// An equal high or low also moves the extreme's candle index.
    Latest,
}

/// Which hole taints the ladder (ambiguity: "a ladder whose defining candles
/// hold one").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaintScope {
    /// Any incomplete candle folded taints the ladder for the session
    /// (headline: the reading that admits less).
    AnyFolded,
    /// Only an incomplete candle that set the high or the low taints it.
    ExtremeCandles,
}

/// A ladder at one moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Ladder {
    /// Highest high, paise.
    pub hi: i64,
    /// Lowest low, paise.
    pub lo: i64,
    /// Candle index that set the high.
    pub hi_ix: u32,
    /// Candle index that set the low.
    pub lo_ix: u32,
    /// Direction.
    pub leg: Leg,
    /// Tainted for the session.
    pub tainted: bool,
    /// Candles folded.
    pub folded: u32,
}

impl Ladder {
    /// The empty ladder before the session's first candle.
    pub const EMPTY: Ladder = Ladder {
        hi: 0,
        lo: 0,
        hi_ix: 0,
        lo_ix: 0,
        leg: Leg::Undetermined,
        tainted: false,
        folded: 0,
    };

    /// The range `R = hi - lo`, paise.
    pub fn range(&self) -> i64 {
        self.hi - self.lo
    }

    /// Usable: at least one candle, not Undetermined, not tainted, `R > 0`.
    pub fn usable(&self) -> bool {
        self.folded > 0 && self.leg != Leg::Undetermined && !self.tainted && self.range() > 0
    }

    /// Folds a candle into the running ladder.
    pub fn fold(&mut self, c: &Candle, ties: ExtremeTies, taint: TaintScope) {
        if self.folded == 0 {
            *self = Ladder {
                hi: c.h,
                lo: c.l,
                hi_ix: c.i,
                lo_ix: c.i,
                leg: Leg::Undetermined,
                tainted: c.incomplete,
                folded: 1,
            };
            return;
        }
        let new_hi = match ties {
            ExtremeTies::Strict => c.h > self.hi,
            ExtremeTies::Latest => c.h >= self.hi,
        };
        let new_lo = match ties {
            ExtremeTies::Strict => c.l < self.lo,
            ExtremeTies::Latest => c.l <= self.lo,
        };
        if new_hi {
            self.hi = c.h;
            self.hi_ix = c.i;
        }
        if new_lo {
            self.lo = c.l;
            self.lo_ix = c.i;
        }
        self.leg = match self.hi_ix.cmp(&self.lo_ix) {
            std::cmp::Ordering::Greater => Leg::Up,
            std::cmp::Ordering::Less => Leg::Down,
            std::cmp::Ordering::Equal => Leg::Undetermined,
        };
        let taints = match taint {
            TaintScope::AnyFolded => c.incomplete,
            TaintScope::ExtremeCandles => c.incomplete && (new_hi || new_lo),
        };
        self.tainted |= taints;
        self.folded += 1;
    }

    /// The price of rung `r` (thousandths) in milli-paise, exact; `None` on an
    /// Undetermined ladder.
    pub fn rung_milli(&self, r: u32) -> Option<i128> {
        let hi = i128::from(self.hi) * 1000;
        let lo = i128::from(self.lo) * 1000;
        let span = i128::from(self.range()) * i128::from(r);
        match (self.leg, r <= 1000) {
            (Leg::Up, true) | (Leg::Down, false) => Some(hi - span),
            (Leg::Up, false) | (Leg::Down, true) => Some(lo + span),
            (Leg::Undetermined, _) => None,
        }
    }

    /// The `m`-th rung beyond rung `r` in the given price direction
    /// (`up = true`: higher prices), ordered by price over all twelve rungs,
    /// as milli-paise. `None` when there is no such rung (S28's bounce rung).
    pub fn rung_beyond(&self, r: u32, m: u32, up: bool) -> Option<i128> {
        let level = self.rung_milli(r)?;
        let mut prices: Vec<i128> = RUNGS.iter().filter_map(|&x| self.rung_milli(x)).collect();
        prices.sort_unstable();
        prices.dedup();
        let at = prices.iter().position(|&p| p == level)?;
        let m = usize::try_from(m).ok()?;
        if up {
            prices.get(at.checked_add(m)?).copied()
        } else {
            at.checked_sub(m).and_then(|i| prices.get(i).copied())
        }
    }
}

/// The judging ladders of a day: entry `i` is the running ladder of candles
/// `0..i-1` (entry 0 is `EMPTY`), and the last entry (index `len`) is the
/// ladder after every candle.
pub fn running(candles: &[Candle], ties: ExtremeTies, taint: TaintScope) -> Vec<Ladder> {
    let mut out = Vec::with_capacity(candles.len() + 1);
    let mut ladder = Ladder::EMPTY;
    out.push(ladder);
    for c in candles {
        ladder.fold(c, ties, taint);
        out.push(ladder);
    }
    out
}

/// A candle's relation to a level `l` (milli-paise) at tolerance `t`
/// (milli-paise, at least 0), PREVIEW-SPEC 7.2 item 2. `long` picks the
/// favourable side: above the level for a long, below it for a short.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relation {
    /// `low - t <= L <= high + t`.
    Touch,
    /// Entirely on the favourable side beyond the tolerance.
    Clear,
    /// Entirely on the unfavourable side beyond the tolerance.
    Through,
}

impl Relation {
    /// Short name for the golden files.
    pub fn name(self) -> &'static str {
        match self {
            Relation::Touch => "touch",
            Relation::Clear => "clear",
            Relation::Through => "through",
        }
    }
}

/// [`Relation`] of a candle's range `(high, low)` (paise) to level `l`.
pub fn relate(high: i64, low: i64, l: i128, t: i128, long: bool) -> Relation {
    let lo = i128::from(low) * 1000 - t;
    let hi = i128::from(high) * 1000 + t;
    if lo <= l && l <= hi {
        Relation::Touch
    } else if (lo > l) == long {
        Relation::Clear
    } else {
        Relation::Through
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::OPEN;

    pub(crate) fn candle(i: u32, o: i64, h: i64, l: i64, c: i64) -> Candle {
        Candle {
            i,
            first_minute: i,
            last_minute: i,
            close_t: OPEN + 60 * (i + 1),
            o,
            h,
            l,
            c,
            v: 1,
            incomplete: false,
        }
    }

    fn fold_all(cs: &[Candle], ties: ExtremeTies, taint: TaintScope) -> Ladder {
        let mut l = Ladder::EMPTY;
        for c in cs {
            l.fold(c, ties, taint);
        }
        l
    }

    #[test]
    fn the_first_candle_holds_both_extremes() {
        let l = fold_all(
            &[candle(0, 100, 110, 90, 105)],
            ExtremeTies::Strict,
            TaintScope::AnyFolded,
        );
        assert_eq!(l.leg, Leg::Undetermined);
        assert!(!l.usable());
        assert_eq!((l.hi, l.lo, l.range()), (110, 90, 20));
        assert!(!Ladder::EMPTY.usable());
        assert_eq!(Ladder::EMPTY.rung_milli(0), None);
    }

    #[test]
    fn direction_follows_the_later_one_sided_extreme() {
        let cs = [
            candle(0, 100, 110, 90, 105),
            candle(1, 105, 112, 100, 111), // new high -> Up
            candle(2, 111, 111, 95, 96),   // inside -> stays Up
            candle(3, 96, 97, 85, 86),     // new low -> Down
            candle(4, 86, 120, 80, 119),   // both -> Undetermined
            candle(5, 119, 125, 110, 124), // new high -> Up
        ];
        let ladders = running(&cs, ExtremeTies::Strict, TaintScope::AnyFolded);
        let legs: Vec<Leg> = ladders.iter().map(|l| l.leg).collect();
        assert_eq!(
            legs,
            [
                Leg::Undetermined,
                Leg::Undetermined,
                Leg::Up,
                Leg::Up,
                Leg::Down,
                Leg::Undetermined,
                Leg::Up
            ]
        );
        assert_eq!(ladders[6].hi_ix, 5);
        assert_eq!(ladders[6].lo_ix, 4);
        assert!(ladders[6].usable());
    }

    #[test]
    fn equal_extremes_move_only_under_the_latest_reading() {
        let cs = [
            candle(0, 100, 110, 90, 105),
            candle(1, 105, 112, 100, 111), // Up
            candle(2, 111, 111, 90, 95),   // equal low
        ];
        let strict = fold_all(&cs, ExtremeTies::Strict, TaintScope::AnyFolded);
        assert_eq!(strict.leg, Leg::Up);
        assert_eq!(strict.lo_ix, 0);
        let latest = fold_all(&cs, ExtremeTies::Latest, TaintScope::AnyFolded);
        assert_eq!(latest.leg, Leg::Down);
        assert_eq!(latest.lo_ix, 2);
        // an equal high under Latest moves the high's index too
        let more = [cs[0], cs[1], candle(2, 111, 112, 101, 105)];
        assert_eq!(
            fold_all(&more, ExtremeTies::Latest, TaintScope::AnyFolded).hi_ix,
            2
        );
        assert_eq!(
            fold_all(&more, ExtremeTies::Strict, TaintScope::AnyFolded).hi_ix,
            1
        );
    }

    #[test]
    fn taint_scopes_differ_on_an_inside_hole() {
        let mut hole = candle(2, 105, 108, 101, 106); // inside candle with a hole
        hole.incomplete = true;
        let cs = [
            candle(0, 100, 110, 90, 105),
            candle(1, 105, 112, 100, 111),
            hole,
        ];
        assert!(fold_all(&cs, ExtremeTies::Strict, TaintScope::AnyFolded).tainted);
        assert!(!fold_all(&cs, ExtremeTies::Strict, TaintScope::ExtremeCandles).tainted);
        let mut extreme_hole = candle(2, 111, 130, 101, 129);
        extreme_hole.incomplete = true;
        let cs2 = [cs[0], cs[1], extreme_hole];
        assert!(fold_all(&cs2, ExtremeTies::Strict, TaintScope::ExtremeCandles).tainted);
        // a first candle with a hole taints from the start
        let mut first = candle(0, 1, 2, 0, 1);
        first.incomplete = true;
        assert!(fold_all(&[first], ExtremeTies::Strict, TaintScope::ExtremeCandles).tainted);
    }

    #[test]
    fn rungs_on_both_legs_match_the_kit_chart() {
        // MAZDOCK's A1 ladder 2,075.00 - 2,093.90 on an Up leg (BRIEF fixture).
        let up = Ladder {
            hi: 209_390,
            lo: 207_500,
            hi_ix: 4,
            lo_ix: 1,
            leg: Leg::Up,
            tainted: false,
            folded: 5,
        };
        assert_eq!(up.range(), 1890);
        assert_eq!(up.rung_milli(0), Some(209_390_000));
        assert_eq!(up.rung_milli(1000), Some(207_500_000));
        assert_eq!(up.rung_milli(786), Some(209_390_000 - 786 * 1890));
        assert_eq!(up.rung_milli(500), Some(208_445_000));
        assert_eq!(up.rung_milli(1272), Some(207_500_000 + 1272 * 1890));
        assert!(up.rung_milli(1272) > up.rung_milli(0));
        let down = Ladder {
            leg: Leg::Down,
            ..up
        };
        assert_eq!(down.rung_milli(0), Some(207_500_000));
        assert_eq!(down.rung_milli(1000), Some(209_390_000));
        assert_eq!(down.rung_milli(382), Some(207_500_000 + 382 * 1890));
        assert_eq!(down.rung_milli(1618), Some(209_390_000 - 1618 * 1890));
        assert!(down.rung_milli(1618) < down.rung_milli(0));
        let und = Ladder {
            leg: Leg::Undetermined,
            ..up
        };
        assert_eq!(und.rung_milli(500), None);
    }

    #[test]
    fn rung_beyond_walks_price_order_over_all_twelve() {
        let up = Ladder {
            hi: 209_390,
            lo: 207_500,
            hi_ix: 4,
            lo_ix: 1,
            leg: Leg::Up,
            tainted: false,
            folded: 5,
        };
        // from 0.786 one rung up is 0.618 ("went to 0.618"), two is 0.5
        assert_eq!(up.rung_beyond(786, 1, true), up.rung_milli(618));
        assert_eq!(up.rung_beyond(786, 2, true), up.rung_milli(500));
        assert_eq!(up.rung_beyond(786, 5, true), up.rung_milli(0));
        assert_eq!(up.rung_beyond(786, 6, true), up.rung_milli(1272));
        assert_eq!(up.rung_beyond(786, 1, false), up.rung_milli(1000));
        assert_eq!(up.rung_beyond(1000, 1, false), None);
        assert_eq!(up.rung_beyond(3000, 1, true), None);
        assert_eq!(up.rung_beyond(786, 0, true), up.rung_milli(786));
        assert_eq!(up.rung_beyond(786, u32::MAX, true), None);
        assert_eq!(up.rung_beyond(1, 1, true), None); // not a rung
        let und = Ladder {
            leg: Leg::Undetermined,
            ..up
        };
        assert_eq!(und.rung_beyond(786, 1, true), None);
    }

    #[test]
    fn relations_mirror_for_the_short() {
        let level = 100_000; // Rs 1.00 in milli-paise of 100 paise
        assert_eq!(relate(101, 99, level, 0, true), Relation::Touch);
        assert_eq!(relate(100, 100, level, 0, true), Relation::Touch);
        assert_eq!(relate(105, 101, level, 0, true), Relation::Clear);
        assert_eq!(relate(99, 95, level, 0, true), Relation::Through);
        assert_eq!(relate(105, 101, level, 0, false), Relation::Through);
        assert_eq!(relate(99, 95, level, 0, false), Relation::Clear);
        // a tolerance of one paisa turns a near miss into a touch
        assert_eq!(relate(105, 101, level, 1_000, true), Relation::Touch);
        assert_eq!(relate(99, 95, level, 1_000, false), Relation::Touch);
        assert_eq!(relate(105, 101, level, 999, true), Relation::Clear);
        assert_eq!(Relation::Touch.name(), "touch");
        assert_eq!(Relation::Clear.name(), "clear");
        assert_eq!(Relation::Through.name(), "through");
        assert_eq!(Leg::Up.name(), "up");
        assert_eq!(Leg::Down.name(), "down");
        assert_eq!(Leg::Undetermined.name(), "undetermined");
    }
}
