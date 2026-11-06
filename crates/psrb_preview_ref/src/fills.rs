//! F30 `fills::entry`, F34 `fills::StopScan`, F35 `fills::flip`, F36 `fills::time_exit`,
//! F37 `fills::square_off` and F38 `fills::MinuteFallback` (PREVIEW-SPEC section 3.5).
//!
//! Every fill is at the worst print of the traded unit it uses: a buy at its high, a
//! sell at its low. On a GDFL day the unit is a traded second; on a fallback day it is a
//! Zerodha minute with volume above 0, whose slot second is its start + 59 s.

use crate::cond::Side;
use crate::market::{Minute, Second};
use crate::time::OPEN;

/// One traded unit a fill can use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Print {
    /// The instant it starts (a second, or a minute's start).
    pub start: u32,
    /// The second it fills at for slots, marks and the contract note.
    pub slot: u32,
    /// Its highest traded price.
    pub high: i64,
    /// Its lowest traded price.
    pub low: i64,
}

/// The prints of a GDFL stock-day: every stored second, as stored (a zero-volume second
/// is used as stored and counted, Q7).
#[must_use]
pub fn second_prints(seconds: &[Second]) -> Vec<Print> {
    seconds.iter().map(|s| Print { start: s.at, slot: s.at, high: s.bar.high, low: s.bar.low }).collect()
}

/// F38: the prints of a fallback stock-day: every in-session minute with volume above 0.
#[must_use]
pub fn minute_prints(minutes: &[Minute], close: u32) -> Vec<Print> {
    minutes
        .iter()
        .filter(|m| !m.padded() && m.start >= OPEN && m.start < close)
        .map(|m| Print { start: m.start, slot: m.start + 59, high: m.bar.high, low: m.bar.low })
        .collect()
}

/// The index of the first print starting at or after `x` (the length when none).
#[must_use]
pub fn first_at_or_after(prints: &[Print], x: u32) -> usize {
    prints.partition_point(|p| p.start < x)
}

/// The worst print for an order: a buy pays the high, a sell gets the low.
#[must_use]
pub const fn worst(print: &Print, buy: bool) -> i64 {
    if buy { print.high } else { print.low }
}

/// The entry order's direction: a long buys, a short sells.
#[must_use]
pub fn entry_buys(side: Side) -> bool {
    side == Side::Long
}

/// F30: the entry at the first print at or after `t`: its index and fill price.
#[must_use]
pub fn entry(prints: &[Print], t: u32, side: Side) -> Option<(usize, i64)> {
    let i = first_at_or_after(prints, t);
    prints.get(i).map(|p| (i, worst(p, entry_buys(side))))
}

/// F34: the resting stop reached by a print: a long's when `low <= stop`, a short's when
/// `high >= stop`.
#[must_use]
pub const fn reaches(stop: i64, print: &Print, side: Side) -> bool {
    match side {
        Side::Long => print.low <= stop,
        Side::Short => print.high >= stop,
    }
}

/// F34: the resting stop's scan over a stock-day's prints, one print per step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StopScan {
    /// The stop in force.
    pub stop: i64,
    /// The trade's side.
    pub side: Side,
    /// The next print to read.
    pub cursor: usize,
}

impl StopScan {
    /// One print: reads it, moves past it, and says whether it reached the stop.
    pub const fn step(&mut self, print: &Print) -> bool {
        self.cursor += 1;
        reaches(self.stop, print, self.side)
    }

    /// Steps over every print before `limit`: the index of the first that reaches the
    /// stop, or `None` with the cursor on the first print at or after `limit`.
    pub fn until(&mut self, prints: &[Print], limit: u32) -> Option<usize> {
        while let Some(print) = prints.get(self.cursor).filter(|p| p.start < limit) {
            if self.step(print) {
                return Some(self.cursor - 1);
            }
        }
        None
    }
}

/// The exit order's fill price (the worst print for the closing side).
#[must_use]
pub fn exit_price(print: &Print, side: Side) -> i64 {
    worst(print, !entry_buys(side))
}

/// F37's remainder rule: the worst print from the entry print on (the entry print
/// included, Q5): a long's lowest low, a short's highest high.
#[must_use]
pub fn worst_since(prints: &[Print], from: usize, side: Side) -> Option<i64> {
    let tail = prints.get(from..)?;
    match side {
        Side::Long => tail.iter().map(|p| p.low).min(),
        Side::Short => tail.iter().map(|p| p.high).max(),
    }
}

#[cfg(test)]
#[path = "fills_tests.rs"]
mod tests;
