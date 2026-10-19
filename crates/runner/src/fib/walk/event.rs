//! The event walk's tables (FM-F030; D-6390, D-6391): per side of one
//! stock-day, sparse tables of the bars' extremes, the next bar that needs a
//! full step, and prefix sums of volume and of volume times each leg's price,
//! built once in O(bars x log bars).
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
//!
//! A SPILL — an entry or an exit order filling over seconds, each giving at
//! most its own volume — is one binary search on the volume prefix over the
//! run of plain bars ahead of it (a bar a fill would flag, or a side with no
//! book, ends the run and is stepped), and its value is two reads of the
//! price prefix: O(log bars) per run instead of one step per second
//! (`spill_by_events_equals_forward_pass`).

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

/// Which leg of a trade a spill fills.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Leg {
    /// The entry: a buy, at each bar's high.
    Buy,
    /// The exit: a sell, at each bar's low.
    Sell,
}

/// What a spill took from a run of plain bars.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Spill {
    /// One past the last bar it read.
    pub(super) end: usize,
    /// Shares filled.
    pub(super) qty: u64,
    /// Their value, raw paisa, mirrored.
    pub(super) value: i128,
    /// The last bar that filled any.
    pub(super) last: Option<usize>,
}

/// One leg's prefix sums: its volume and its value, each entry `k` summing
/// bars `[0, k)`.
#[derive(Clone, Debug)]
struct Prefix {
    /// For each bar, the first bar at or after it that a spill of this leg
    /// must step: a bar [`Events::next_special`] names, or one whose fill the
    /// walk would flag (no side to take, a flagged print, a deep price outside
    /// its Zerodha minute's range).
    next_stop: Vec<u32>,
    /// The leg's price times the volume, summed WRAPPING: the difference of
    /// two entries is exact whenever the range's own sum fits an `i128`, which
    /// is when the walk's own order value fits it.
    value: Vec<i128>,
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
    /// The volume of the bars before each bar (entry `k` sums `[0, k)`): a
    /// day's volumes fit a `u128` with room to spare.
    volume: Vec<u128>,
    /// The buy leg's prefix.
    buy: Prefix,
    /// The sell leg's prefix.
    sell: Prefix,
}

/// Whether a fill of `leg` in `bar` raises no flag: the side has liquidity,
/// the print is not flagged, and a deep price lies inside its Zerodha
/// minute's converted range (`Stepper::mark_fill`'s three tests).
fn plain(bar: &SideBar, leg: Leg, deep: bool) -> bool {
    let (side, price) = match leg {
        Leg::Buy => (Bit::EntryOk, bar.hi),
        Leg::Sell => (Bit::ExitOk, bar.lo),
    };
    let witnessed = bar.is(Bit::Witnessed) && price >= bar.witness_lo && price <= bar.witness_hi;
    bar.is(side) && !bar.is(Bit::Flagged) && (!deep || witnessed)
}

/// The index of `k` as a stored `u32`.
fn stored(k: usize) -> u32 {
    u32::try_from(k).unwrap_or(u32::MAX)
}

/// A stored `u32` index read back, or `fallback`.
fn read(at: Option<&u32>, fallback: usize) -> usize {
    at.and_then(|k| usize::try_from(*k).ok())
        .unwrap_or(fallback)
}

impl Events {
    /// The tables of one side's bars.
    pub(super) fn build(bars: &[SideBar], max_gap: u32, deep: bool) -> Self {
        let n = bars.len();
        let mut next_special = vec![0; n];
        let mut buy_stop = vec![0; n];
        let mut sell_stop = vec![0; n];
        let (mut next, mut next_buy, mut next_sell) = (stored(n), stored(n), stored(n));
        let slots = next_special
            .iter_mut()
            .zip(buy_stop.iter_mut())
            .zip(sell_stop.iter_mut());
        for (k, (((special_at, buy_at), sell_at), bar)) in slots.zip(bars).enumerate().rev() {
            let special =
                !bar.usable() || bar.is(Bit::HaltCrossed) || (deep && bar.gap_missing > max_gap);
            if special {
                next = stored(k);
            }
            if special || !plain(bar, Leg::Buy, deep) {
                next_buy = stored(k);
            }
            if special || !plain(bar, Leg::Sell, deep) {
                next_sell = stored(k);
            }
            *special_at = next;
            *buy_at = next_buy;
            *sell_at = next_sell;
        }
        let mut volume = Vec::with_capacity(n.saturating_add(1));
        let mut buy_value = Vec::with_capacity(n.saturating_add(1));
        let mut sell_value = Vec::with_capacity(n.saturating_add(1));
        let (mut v, mut b, mut s): (u128, i128, i128) = (0, 0, 0);
        volume.push(v);
        buy_value.push(b);
        sell_value.push(s);
        for bar in bars {
            let shares = i128::from(bar.volume);
            v += u128::from(bar.volume);
            b = b.wrapping_add(shares.wrapping_mul(i128::from(bar.hi_raw)));
            s = s.wrapping_add(shares.wrapping_mul(i128::from(bar.lo_raw)));
            volume.push(v);
            buy_value.push(b);
            sell_value.push(s);
        }
        Self {
            lows: Table::build(bars, Keep::LowestLow),
            highs_first: Table::build(bars, Keep::HighestHighFirst),
            highs_last: Table::build(bars, Keep::HighestHighLast),
            next_special,
            volume,
            buy: Prefix {
                next_stop: buy_stop,
                value: buy_value,
            },
            sell: Prefix {
                next_stop: sell_stop,
                value: sell_value,
            },
        }
    }

    /// The first bar at or after `from` that needs a full step.
    pub(super) fn next_special(&self, from: usize) -> usize {
        read(self.next_special.get(from), self.next_special.len())
    }

    /// One leg's prefix.
    const fn leg(&self, leg: Leg) -> &Prefix {
        match leg {
            Leg::Buy => &self.buy,
            Leg::Sell => &self.sell,
        }
    }

    /// The first bar at or after `from` that a spill of `leg` must step.
    pub(super) fn next_stop(&self, leg: Leg, from: usize) -> usize {
        let stops = &self.leg(leg).next_stop;
        read(stops.get(from), stops.len())
    }

    /// The volume of the bars before bar `k`.
    fn volume_before(&self, k: usize) -> u128 {
        self.volume.get(k).copied().unwrap_or(0)
    }

    /// A spill of `want` shares of `leg` from bar `from` over the plain bars
    /// `[from, to)` (`to <= `[`Self::next_stop`]`(leg, from)`): each bar gives
    /// `min(left, its volume)` at its own worst price, as `Stepper::step`
    /// would, found by one binary search on the volume prefix. It reads up to
    /// the bar that completes the order, or the whole run.
    pub(super) fn spill(
        &self,
        bars: &[SideBar],
        leg: Leg,
        from: usize,
        to: usize,
        want: u64,
    ) -> Spill {
        let prefix = self.leg(leg);
        let value_before = |k: usize| prefix.value.get(k).copied().unwrap_or(0);
        let base = self.volume_before(from);
        let ends = self.volume.get(from + 1..=to).unwrap_or(&[]);
        let wanted = u128::from(want);
        // The bars `[from, whole_to)` give their whole volume; the order is
        // complete inside the run when the bar at `whole_to` is in it.
        let whole_to = from + ends.partition_point(|v| v.saturating_sub(base) < wanted);
        let whole =
            u64::try_from(self.volume_before(whole_to).saturating_sub(base)).unwrap_or(u64::MAX);
        let whole_value = value_before(whole_to).wrapping_sub(value_before(from));
        if whole_to < to {
            let rest = want - whole;
            let price = bars.get(whole_to).map_or(0, |bar| match leg {
                Leg::Buy => bar.hi_raw,
                Leg::Sell => bar.lo_raw,
            });
            return Spill {
                end: whole_to + 1,
                qty: want,
                value: whole_value.wrapping_add(i128::from(rest) * i128::from(price)),
                last: Some(whole_to),
            };
        }
        // The last bar that gave any is the one before the first prefix entry
        // that already holds the run's whole volume; none did when the run
        // had none.
        let total = self.volume_before(to);
        let last = from + ends.partition_point(|v| *v < total);
        Spill {
            end: to,
            qty: whole,
            value: whole_value,
            last: (whole > 0).then_some(last),
        }
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
        let pick =
            |table: &Table, keep| table.range(bars, keep, from, to).and_then(|k| bars.get(k));
        match (
            pick(&self.lows, Keep::LowestLow),
            pick(&self.highs_first, Keep::HighestHighFirst),
            pick(&self.highs_last, Keep::HighestHighLast),
        ) {
            (Some(low), Some(first), Some(last)) => Some(Extremes {
                best_raw: first.hi_raw,
                best: first.hi,
                best_first: first.second,
                best_last: last.second,
                worst_raw: low.lo_raw,
            }),
            _ => None,
        }
    }

    /// The bytes the tables hold.
    pub(super) fn bytes(&self) -> usize {
        let table = |t: &Table| t.levels.iter().map(Vec::len).sum::<usize>();
        let words = table(&self.lows)
            + table(&self.highs_first)
            + table(&self.highs_last)
            + self.next_special.len()
            + self.buy.next_stop.len()
            + self.sell.next_stop.len();
        let sums = self.volume.len() + self.buy.value.len() + self.sell.value.len();
        words * core::mem::size_of::<u32>() + sums * core::mem::size_of::<u128>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plain bar at `second` with raw `lo..=hi`.
    fn bar(second: u32, lo: i64, hi: i64) -> SideBar {
        SideBar {
            second,
            hi: i128::from(hi),
            lo: i128::from(lo),
            op: i128::from(lo),
            hi_raw: hi,
            lo_raw: lo,
            volume: 1,
            bits: 0,
            gap_missing: 0,
            witness_lo: 0,
            witness_hi: 0,
        }
    }

    /// Four bars: highs 5, 9, 9, 7 and lows 3, 2, 4, 1.
    fn four() -> Vec<SideBar> {
        vec![bar(10, 3, 5), bar(11, 2, 9), bar(12, 4, 9), bar(13, 1, 7)]
    }

    /// An index past the bars keeps the first of the two: the tables never
    /// ask, and the answer is defined anyway.
    #[test]
    fn better_keeps_the_first_when_a_bar_is_missing() {
        let bars = four();
        assert_eq!(better(&bars, Keep::LowestLow, 1, 9), 1);
        assert_eq!(better(&bars, Keep::LowestLow, 9, 1), 9);
        assert_eq!(better(&bars, Keep::LowestLow, 0, 1), 1);
        assert_eq!(better(&bars, Keep::HighestHighFirst, 1, 2), 1);
        assert_eq!(better(&bars, Keep::HighestHighLast, 1, 2), 2);
        // Ties: the lower low and the first high keep the earlier bar, the
        // last high the later one.
        let tied = [bar(10, 5, 9), bar(11, 5, 9)];
        assert_eq!(better(&tied, Keep::LowestLow, 0, 1), 0);
        assert_eq!(better(&tied, Keep::HighestHighFirst, 0, 1), 0);
        assert_eq!(better(&tied, Keep::HighestHighLast, 0, 1), 1);
    }

    /// No bars: no levels, no range, every descent ends at zero.
    #[test]
    fn an_empty_side_has_no_table_and_no_extremes() {
        let events = Events::build(&[], 0, true);
        assert!(events.lows.levels.is_empty());
        assert_eq!(events.extremes(&[], 0, 0), None);
        assert_eq!(events.low_reaches(&[], 0, 0), 0);
        assert_eq!(events.next_special(0), 0);
        assert_eq!(events.next_stop(Leg::Buy, 0), 0);
        assert_eq!(events.next_stop(Leg::Sell, 3), 0);
        // Each of the three prefix sums holds its leading zero.
        assert_eq!(events.bytes(), 3 * 16);
    }

    /// A descent from past the end answers the bar count; an empty range has
    /// no extremes; a full one has the tie rules.
    #[test]
    fn descents_ranges_and_bytes_of_four_bars() {
        let bars = four();
        let events = Events::build(&bars, 0, false);
        assert_eq!(events.high_reaches(&bars, 4, 0), 4);
        assert_eq!(events.high_reaches(&bars, 9, 0), 4);
        assert_eq!(events.high_reaches(&bars, 0, 8), 1);
        assert_eq!(events.high_above(&bars, 2, 8), 2);
        assert_eq!(events.low_reaches(&bars, 0, 1), 3);
        assert_eq!(events.low_reaches(&bars, 0, 0), 4);
        assert_eq!(events.extremes(&bars, 2, 2), None);
        assert_eq!(events.extremes(&bars, 3, 2), None);
        assert_eq!(
            events.extremes(&bars, 0, 4),
            Some(Extremes {
                best_raw: 9,
                best: 9,
                best_first: 11,
                best_last: 12,
                worst_raw: 1,
            })
        );
        assert_eq!(events.next_special(0), 4);
        assert_eq!(events.next_special(7), 4);
        // No bar has a book side, so every bar stops a spill of either leg.
        assert_eq!(events.next_stop(Leg::Buy, 1), 1);
        assert_eq!(events.next_stop(Leg::Sell, 2), 2);
        assert_eq!(events.next_stop(Leg::Sell, 4), 4);
        // Levels of 4, 3 and 1 entries in each of three tables, and three
        // stop entries per bar, four bytes each; three prefix sums of five
        // entries, sixteen bytes each.
        assert_eq!(events.bytes(), (3 * (4 + 3 + 1) + 3 * 4) * 4 + 3 * 5 * 16);
    }

    /// A plain bar of both legs: both book sides, witnessed by a minute
    /// spanning it.
    fn plain_bar(second: u32, lo: i64, hi: i64, volume: u64) -> SideBar {
        let mut b = bar(second, lo, hi);
        b.volume = volume;
        b.bits =
            (1 << Bit::EntryOk as u16) | (1 << Bit::ExitOk as u16) | (1 << Bit::Witnessed as u16);
        b.witness_lo = i128::from(lo);
        b.witness_hi = i128::from(hi);
        b
    }

    /// Volumes 3, 0, 4, 2, 0 at lows 10, 11, 12, 13, 14 and highs ten above.
    fn run() -> Vec<SideBar> {
        [3, 0, 4, 2, 0]
            .iter()
            .zip(0_i64..)
            .map(|(v, k)| plain_bar(20 + u32::try_from(k).unwrap_or(0), 10 + k, 20 + k, *v))
            .collect()
    }

    /// A spill takes each bar's whole volume until the bar that completes
    /// the order, which gives only what is left, at each bar's own price.
    #[test]
    fn a_spill_completing_inside_the_run_stops_at_the_completing_bar() {
        let bars = run();
        let events = Events::build(&bars, 0, true);
        assert_eq!(events.next_stop(Leg::Buy, 0), 5);
        assert_eq!(events.next_stop(Leg::Sell, 0), 5);
        // Five shares: three at 10, none at 11, two of four at 12.
        assert_eq!(
            events.spill(&bars, Leg::Sell, 0, 5, 5),
            Spill {
                end: 3,
                qty: 5,
                value: 3 * 10 + 2 * 12,
                last: Some(2),
            }
        );
        // Bought at the highs instead.
        assert_eq!(
            events.spill(&bars, Leg::Buy, 0, 5, 5).value,
            3 * 20 + 2 * 22
        );
        // Exactly the first bar's volume: it completes there, alone.
        assert_eq!(
            events.spill(&bars, Leg::Sell, 0, 5, 3),
            Spill {
                end: 1,
                qty: 3,
                value: 3 * 10,
                last: Some(0),
            }
        );
        // From the empty bar: it gives nothing, the next completes.
        assert_eq!(
            events.spill(&bars, Leg::Sell, 1, 5, 4),
            Spill {
                end: 3,
                qty: 4,
                value: 4 * 12,
                last: Some(2),
            }
        );
        // The run's last bar completes it.
        assert_eq!(
            events.spill(&bars, Leg::Sell, 0, 4, 9),
            Spill {
                end: 4,
                qty: 9,
                value: 3 * 10 + 4 * 12 + 2 * 13,
                last: Some(3),
            }
        );
    }

    /// A run too thin for the order gives all it has; its last filling bar
    /// is the last with volume, not the run's last bar.
    #[test]
    fn a_spill_the_run_cannot_complete_takes_every_share_of_it() {
        let bars = run();
        let events = Events::build(&bars, 0, true);
        assert_eq!(
            events.spill(&bars, Leg::Sell, 0, 5, 100),
            Spill {
                end: 5,
                qty: 9,
                value: 3 * 10 + 4 * 12 + 2 * 13,
                last: Some(3),
            }
        );
        assert_eq!(
            events.spill(&bars, Leg::Sell, 3, 5, 100),
            Spill {
                end: 5,
                qty: 2,
                value: 2 * 13,
                last: Some(3),
            }
        );
        // A run of empty bars fills nothing.
        assert_eq!(
            events.spill(&bars, Leg::Buy, 4, 5, 100),
            Spill {
                end: 5,
                qty: 0,
                value: 0,
                last: None,
            }
        );
    }

    /// A bar a fill would flag ends the run: no book side, a flagged print,
    /// a price outside its minute, no minute at all, or a bar needing a full
    /// step. Stage A reads no minute range.
    #[test]
    fn a_bar_a_fill_would_flag_stops_the_spill_of_its_leg() {
        let mut bars = run();
        for (k, b) in bars.iter_mut().enumerate() {
            match k {
                1 => b.bits &= !(1 << Bit::EntryOk as u16),
                2 => b.bits |= 1 << Bit::Flagged as u16,
                3 => b.witness_lo = 14,
                _ => {}
            }
        }
        let events = Events::build(&bars, 0, true);
        assert_eq!(events.next_stop(Leg::Buy, 0), 1);
        assert_eq!(events.next_stop(Leg::Sell, 0), 2);
        assert_eq!(events.next_stop(Leg::Sell, 3), 3);
        assert_eq!(events.next_stop(Leg::Buy, 3), 5);
        let mut wide = run();
        for (k, b) in wide.iter_mut().enumerate() {
            match k {
                0 => b.witness_hi = 19,
                1 => b.bits &= !(1 << Bit::Witnessed as u16),
                2 => b.bits |= 1 << Bit::HaltCrossed as u16,
                _ => {}
            }
        }
        let events = Events::build(&wide, 0, true);
        assert_eq!(events.next_stop(Leg::Buy, 0), 0);
        assert_eq!(events.next_stop(Leg::Sell, 0), 1);
        let stage_a = Events::build(&wide, 0, false);
        assert_eq!(stage_a.next_stop(Leg::Buy, 0), 2);
        assert_eq!(stage_a.next_stop(Leg::Sell, 0), 2);
    }
}
