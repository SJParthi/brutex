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
#[path = "ladder_tests.rs"]
mod tests;
