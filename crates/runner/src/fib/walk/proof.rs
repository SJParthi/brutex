//! The real-data proof's two pure halves (FIB-5 REAL DATA; D-6392): the plans
//! the specification asks the proof to walk, and the counts it asks the proof
//! to report.
//!
//! The specification's proof walks, for a few stock-months, "plans generated
//! every 15 minutes on both sides with the stop at that 15-minute candle's far
//! edge" at both stages, and reports "counts by exit reason, the
//! gap-through-stop count and worst R, partial fills at both sizes, forced and
//! unpriceable counts, p99 seconds waited, and ... the share of stock-days with
//! no GDFL second in [15:09:00, 15:10:00)". Loading a stock-day is not FIB-5's:
//! it is FIB-8's day input, run on the Mac where the store is. This module
//! reads nothing and prints nothing (gates 17 and 22); the timeframe is the
//! candles' own, never a number typed here, and the percentile is the
//! printing program's, from the waits this module keeps in walk order.

use indicators::fibflag::types::{EntryStyle, FibCandle};
use vocab::fib_cube::{SECONDS_PER_MINUTE, SESSION_FORCED_WINDOW_START_SECS, Search};

use super::super::types::{RMultiple, SlotOutcome, TradeFlag};
use super::Plan;
use super::index::clock;

/// Whether a candle may be a decision: complete, with no missing minute, and
/// evaluable.
fn usable(c: &FibCandle) -> bool {
    c.complete && c.missing_minutes == 0 && c.candle.check_evaluable().is_ok()
}

/// The proof's plans on `direction`: at the close of every usable candle of
/// `candles` (one timeframe, ascending, decision basis) that closes before
/// 15:09:00, a CLOSE plan entering at that close, its base that candle (the
/// planned stop one tick beyond its far edge: a long's low, a short's high),
/// its entry window the next candle of the timeframe, and its Fibonacci range
/// the day's range of the usable candles so far, measured in the trade's
/// direction. `trail` names the candles' list in the day's input.
#[must_use]
pub fn plans_at_every_close(candles: &[FibCandle], trail: u16, direction: Search) -> Vec<Plan> {
    let forced = u32::try_from(SESSION_FORCED_WINDOW_START_SECS).unwrap_or(u32::MAX);
    let mut plans = Vec::with_capacity(candles.len());
    let mut range: Option<(i64, i64)> = None;
    for c in candles {
        if !usable(c) {
            continue;
        }
        let (low, high) = range.map_or((c.candle.low, c.candle.high), |(low, high)| {
            (low.min(c.candle.low), high.max(c.candle.high))
        });
        range = Some((low, high));
        let decision = clock(c.close_ts_micros).1;
        if decision >= forced {
            continue;
        }
        let width = u64::from(c.tf_minutes) * SECONDS_PER_MINUTE;
        let (base_edge, fib_zero, fib_one) = match direction {
            Search::Long => (c.candle.low, low, high),
            Search::Short => (c.candle.high, high, low),
        };
        plans.push(Plan {
            direction,
            style: EntryStyle::Close,
            tf_minutes: c.tf_minutes,
            decision_second: decision,
            entry_window_end: decision.saturating_add(u32::try_from(width).unwrap_or(u32::MAX)),
            entry_ref: c.candle.close,
            base_edge,
            trigger_edge: c.candle.close,
            fib_zero,
            fib_one,
            trail,
        });
    }
    plans
}

/// What the proof reports for one stage at one size: every slot by status,
/// the priced ones by exit reason, the gaps through the stop, the worst R, the
/// partial entries, the square-offs, and each entered slot's seconds waited
/// and seconds spilled, in walk order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tally {
    /// Slots counted.
    pub slots: u64,
    /// Slots by status name, in the order each name was first seen.
    pub statuses: Vec<(&'static str, u64)>,
    /// Priced slots by exit reason name, in the order each was first seen.
    pub reasons: Vec<(&'static str, u64)>,
    /// Slots flagged `gap_through_stop`.
    pub gap_through_stop: u64,
    /// The lowest R of the priced slots.
    pub worst_r: Option<RMultiple>,
    /// Priced slots whose R could not be compared with the worst so far: a
    /// cross product left `i128`. Counted, never dropped silently.
    pub r_incomparable: u64,
    /// Slots flagged `partial_entry`.
    pub partial_entries: u64,
    /// Slots flagged `dhan_square_off`: forced, priced or not.
    pub square_offs: u64,
    /// Each entered slot's seconds waited for its first fill.
    pub waits: Vec<u32>,
    /// Each entered slot's seconds spilled past its exit's touching bar.
    pub spills: Vec<u32>,
}

/// Adds one to `name`'s count, appending it when first seen: a handful of
/// names, read once per slot.
fn bump(counts: &mut Vec<(&'static str, u64)>, name: &'static str) {
    for entry in counts.iter_mut() {
        if entry.0 == name {
            entry.1 += 1;
            return;
        }
    }
    counts.push((name, 1));
}

impl Tally {
    /// No slot counted. Named, not `Default`: a tally starts empty by intent.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            slots: 0,
            statuses: Vec::new(),
            reasons: Vec::new(),
            gap_through_stop: 0,
            worst_r: None,
            r_incomparable: 0,
            partial_entries: 0,
            square_offs: 0,
            waits: Vec::new(),
            spills: Vec::new(),
        }
    }

    /// Counts one walked slot of `direction`.
    pub fn add(&mut self, direction: Search, outcome: &SlotOutcome) {
        self.slots += 1;
        bump(&mut self.statuses, outcome.status.name());
        let flagged = |flag| u64::from(outcome.flags.contains(flag));
        self.gap_through_stop += flagged(TradeFlag::GapThroughStop);
        self.partial_entries += flagged(TradeFlag::PartialEntry);
        self.square_offs += flagged(TradeFlag::DhanSquareOff);
        if outcome.status.entered() {
            self.waits.push(outcome.entry_wait_seconds);
            self.spills.push(outcome.exit_spill_seconds);
        }
        let Some(r) = outcome.r_multiple(direction) else {
            return;
        };
        bump(&mut self.reasons, outcome.exit_reason.name());
        self.worse(r);
    }

    /// Keeps `r` when it is below the worst so far.
    fn worse(&mut self, r: RMultiple) {
        let Some(worst) = self.worst_r else {
            self.worst_r = Some(r);
            return;
        };
        match r.cmp_exact(worst) {
            Some(core::cmp::Ordering::Less) => self.worst_r = Some(r),
            Some(_) => {}
            None => self.r_incomparable += 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cross product beyond `i128` is counted, never compared silently; a
    /// lower R replaces the worst, an equal or higher one does not.
    #[test]
    fn a_worse_r_replaces_the_worst_and_an_incomparable_one_is_counted() {
        let mut tally = Tally::empty();
        tally.worse(RMultiple { num: -1, den: 2 });
        tally.worse(RMultiple { num: -1, den: 4 });
        tally.worse(RMultiple { num: -2, den: 4 });
        assert_eq!(tally.worst_r, Some(RMultiple { num: -1, den: 2 }));
        tally.worse(RMultiple { num: -3, den: 2 });
        assert_eq!(tally.worst_r, Some(RMultiple { num: -3, den: 2 }));
        tally.worse(RMultiple {
            num: i128::MAX,
            den: i128::MAX,
        });
        assert_eq!(tally.r_incomparable, 1);
        assert_eq!(tally.worst_r, Some(RMultiple { num: -3, den: 2 }));
    }
}
