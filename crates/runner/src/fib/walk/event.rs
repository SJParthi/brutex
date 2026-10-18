//! The event walk's tables (FM-F030; D-6390): per side of one stock-day,
//! sparse tables of the bars' extremes and the next bar that needs a full
//! step, built once in O(bars x log bars).
//!
//! A holding slot asks them where its next event is — the first bar at or
//! after its cursor whose low reaches a stop level, whose high reaches a
//! target, that sets a new high, that needs a full step (a halt, a corrupt
//! bar, a silence) — each by a descent of O(log bars), and the extremes of the
//! bars it skips by two reads. It then steps that bar exactly as the forward
//! walk would, so the two walks give the same bytes
//! (`event_walk_equals_forward_pass`, `crates/runner/tests/fib_walk.rs`). The
//! cost per pair is O(log bars x (1 + events)), BOUNDED by the events: the
//! touches, the swings it reads, and the new highs while a give-back or an
//! R-step trails (each new high moves their level).

use super::index::{Bit, SideBar};

/// One sparse table: at level `k`, entry `i` names the best bar of
/// `[i, i + 2^k)` by its table's order.
#[derive(Clone, Debug)]
struct Table {
    /// The levels, level 0 first.
    levels: Vec<Vec<u32>>,
}

/// Which bar a table keeps of two.
#[derive(Clone, Copy)]
enum Keep {
    /// The lower low; the earlier on a tie.
    LowestLow,
    /// The higher high; the earlier on a tie.
    HighestHighFirst,
    /// The higher high; the later on a tie.
    HighestHighLast,
}

/// The better of bars `a < b` by `keep`.
fn better(bars: &[SideBar], keep: Keep, a: u32, b: u32) -> u32 {
    let at = |k: u32| usize::try_from(k).ok().and_then(|k| bars.get(k));
    let (Some(x), Some(y)) = (at(a), at(b)) else {
        return a;
    };
    let take_b = match keep {
        Keep::LowestLow => y.lo_raw < x.lo_raw,
        Keep::HighestHighFirst => y.hi_raw > x.hi_raw,
        Keep::HighestHighLast => y.hi_raw >= x.hi_raw,
    };
    if take_b { b } else { a }
}

/// `floor(log2(n))` for `n > 0`.
fn log2(n: usize) -> usize {
    usize::try_from(usize::BITS - 1 - n.leading_zeros()).unwrap_or(0)
}

impl Table {
    /// The table of `bars` by `keep`.
    fn build(bars: &[SideBar], keep: Keep) -> Self {
        let n = bars.len();
        let mut levels: Vec<Vec<u32>> = Vec::new();
        if n == 0 {
            return Self { levels };
        }
        levels.push(
            (0..n)
                .map(|i| u32::try_from(i).unwrap_or(u32::MAX))
                .collect(),
        );
        let mut k = 1;
        while (1 << k) <= n {
            let half = 1 << (k - 1);
            let level: Vec<u32> = {
                let previous = levels.last().map_or(&[][..], Vec::as_slice);
                (0..=n - (1 << k))
                    .map(|i| {
                        let a = previous.get(i).copied().unwrap_or(0);
                        let b = previous.get(i + half).copied().unwrap_or(a);
                        better(bars, keep, a, b)
                    })
                    .collect()
            };
            levels.push(level);
            k += 1;
        }
        Self { levels }
    }

    /// The best bar of `[i, i + 2^k)`.
    fn at(&self, k: usize, i: usize) -> Option<usize> {
        self.levels
            .get(k)
            .and_then(|level| level.get(i))
            .and_then(|b| usize::try_from(*b).ok())
    }

    /// The best bar of `[from, to)`, `from < to`, by two reads.
    fn range(&self, bars: &[SideBar], keep: Keep, from: usize, to: usize) -> Option<usize> {
        let k = log2(to.checked_sub(from).filter(|d| *d > 0)?);
        let a = self.at(k, from)?;
        let b = self.at(k, to - (1 << k))?;
        let (a32, b32) = (u32::try_from(a).ok()?, u32::try_from(b).ok()?);
        usize::try_from(better(bars, keep, a32.min(b32), a32.max(b32))).ok()
    }

    /// The first bar at or after `from` that `hit` accepts, given that `hit`
    /// of a block's best bar says whether any bar of the block is hit; the
    /// bar count when none is.
    fn descend(&self, bars: &[SideBar], from: usize, hit: impl Fn(&SideBar) -> bool) -> usize {
        let n = bars.len();
        let mut at = from;
        if at >= n {
            return n;
        }
        let mut k = log2(n);
        loop {
            let span = 1 << k;
            if at + span <= n
                && self
                    .at(k, at)
                    .and_then(|b| bars.get(b))
                    .is_some_and(|best| !hit(best))
            {
                at += span;
            }
            if k == 0 {
                break;
            }
            k -= 1;
        }
        at
    }
}

/// The skipped bars' extremes, raw and mirrored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Extremes {
    /// The best buy-side price.
    pub(super) best_raw: i64,
    /// The same, on the common denominator.
    pub(super) best: i128,
    /// The first second it printed.
    pub(super) best_first: u32,
    /// The last second it printed.
    pub(super) best_last: u32,
    /// The worst sell-side price.
    pub(super) worst_raw: i64,
}

/// One side's event tables.
#[derive(Clone, Debug)]
pub(super) struct Events {
    /// The lowest low of each block.
    lows: Table,
    /// The highest high of each block, earliest.
    highs_first: Table,
    /// The highest high of each block, latest.
    highs_last: Table,
    /// For each bar, the first bar at or after it that needs a full step: a
    /// corrupt or halted bar, a bar after a halt start, or a bar after a
    /// silence longer than the committed past's (deep).
    next_special: Vec<u32>,
}

impl Events {
    /// The tables of one side's bars.
    pub(super) fn build(bars: &[SideBar], max_gap: u32, deep: bool) -> Self {
        let mut next_special = vec![0; bars.len()];
        let mut next = u32::try_from(bars.len()).unwrap_or(u32::MAX);
        for (k, bar) in bars.iter().enumerate().rev() {
            let special =
                !bar.usable() || bar.is(Bit::HaltCrossed) || (deep && bar.gap_missing > max_gap);
            if special {
                next = u32::try_from(k).unwrap_or(u32::MAX);
            }
            if let Some(slot) = next_special.get_mut(k) {
                *slot = next;
            }
        }
        Self {
            lows: Table::build(bars, Keep::LowestLow),
            highs_first: Table::build(bars, Keep::HighestHighFirst),
            highs_last: Table::build(bars, Keep::HighestHighLast),
            next_special,
        }
    }

    /// The first bar at or after `from` that needs a full step.
    pub(super) fn next_special(&self, from: usize) -> usize {
        self.next_special
            .get(from)
            .and_then(|k| usize::try_from(*k).ok())
            .unwrap_or(self.next_special.len())
    }

    /// The first bar at or after `from` whose low reaches `level`.
    pub(super) fn low_reaches(&self, bars: &[SideBar], from: usize, level: i128) -> usize {
        self.lows.descend(bars, from, |b| b.lo <= level)
    }

    /// The first bar at or after `from` whose high reaches `level`.
    pub(super) fn high_reaches(&self, bars: &[SideBar], from: usize, level: i128) -> usize {
        self.highs_first.descend(bars, from, |b| b.hi >= level)
    }

    /// The first bar at or after `from` whose raw high is above `raw`.
    pub(super) fn high_above(&self, bars: &[SideBar], from: usize, raw: i64) -> usize {
        self.highs_first.descend(bars, from, |b| b.hi_raw > raw)
    }

    /// The extremes of `[from, to)`, or `None` for an empty range.
    pub(super) fn extremes(&self, bars: &[SideBar], from: usize, to: usize) -> Option<Extremes> {
        let low = bars.get(self.lows.range(bars, Keep::LowestLow, from, to)?)?;
        let first = bars.get(
            self.highs_first
                .range(bars, Keep::HighestHighFirst, from, to)?,
        )?;
        let last = bars.get(
            self.highs_last
                .range(bars, Keep::HighestHighLast, from, to)?,
        )?;
        Some(Extremes {
            best_raw: first.hi_raw,
            best: first.hi,
            best_first: first.second,
            best_last: last.second,
            worst_raw: low.lo_raw,
        })
    }

    /// The bytes the tables hold.
    pub(super) fn bytes(&self) -> usize {
        let table = |t: &Table| t.levels.iter().map(Vec::len).sum::<usize>();
        (table(&self.lows)
            + table(&self.highs_first)
            + table(&self.highs_last)
            + self.next_special.len())
            * core::mem::size_of::<u32>()
    }
}
