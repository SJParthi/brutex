//! F39 `trade::simulate`: one signal candle's would-be trade, from its own stock-day's
//! data alone (never the portfolio): entry, the mapped resting stop, trailing and the
//! flip, the time exit, the broker's square-off and the unclosable remainder
//! (PREVIEW-SPEC section 3.5; pins Q3, Q4 and Q5 of TEST-PLAN section 7).

use crate::cond::Side;
use crate::fills::{
    Print, StopScan, entry, exit_price, first_at_or_after, minute_prints, reaches, second_prints,
    worst_since,
};
use crate::market::{Minute, Second};
use crate::pricemap::{map_level, ratio_source, unmapped_level};
use crate::sizing::qty;
use crate::time::{SQUARE_OFF, TIME_EXIT};
use crate::trail::{Step, step};

/// How a trade ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExitKind {
    /// The resting stop.
    Stop,
    /// The close crossed the candle's own side band.
    Flip,
    /// The 15:07:59 order.
    Time,
    /// The broker's square-off from 15:10:00 (`by_broker`).
    SquareOff,
    /// No traded unit left: closed at the worst print since the entry (`by_broker`).
    Unclosable,
}

impl ExitKind {
    /// Every kind, in report order.
    pub const ALL: [Self; 5] = [
        Self::Stop,
        Self::Flip,
        Self::Time,
        Self::SquareOff,
        Self::Unclosable,
    ];

    /// The broker placed the exit order.
    #[must_use]
    pub const fn by_broker(self) -> bool {
        matches!(self, Self::SquareOff | Self::Unclosable)
    }

    /// `stop`, `flip`, `time`, `square-off` or `unclosable`.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Stop => "stop",
            Self::Flip => "flip",
            Self::Time => "time",
            Self::SquareOff => "square-off",
            Self::Unclosable => "unclosable",
        }
    }
}

/// One fill: the second it counts at and its price.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Fill {
    /// The fill's second of the IST day (a fallback fill: its minute's start + 59).
    pub second: u32,
    /// The price, paisa.
    pub price: i64,
}

/// A later candle of the same stock-day, as the trailing rule reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Later {
    /// Its close instant.
    pub t: u32,
    /// Its close.
    pub close: i64,
    /// The rule set's Supertrend side band after folding it (lower for a long, upper for a
    /// short); a later candle of a settled signal always has one.
    pub band: i128,
    /// It is a hole (or was refused).
    pub hole: bool,
}

/// The data a stock-day's fills read.
#[derive(Clone, Copy, Debug)]
pub struct Tape<'a> {
    /// The stored Zerodha minutes of the day.
    pub minutes: &'a [Minute],
    /// The stored GDFL seconds of the day.
    pub seconds: &'a [Second],
    /// Every fill of the day uses minutes (F38).
    pub fallback: bool,
    /// The continuous close.
    pub close: u32,
}

/// A filled would-be trade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Simulated {
    /// The entry fill.
    pub entry: Fill,
    /// Shares.
    pub qty: i64,
    /// The exit fill.
    pub exit: Fill,
    /// How it ended.
    pub kind: ExitKind,
    /// Filled on Zerodha minutes.
    pub fallback: bool,
    /// The entry filled more than 60 s after the signal close.
    pub late: bool,
    /// The resting stop at entry (raw on a GDFL day).
    pub first_stop: i64,
    /// The last stop in force.
    pub last_stop: i64,
    /// Trailing moves whose band could not be mapped.
    pub no_ratio_trail: u32,
}

/// What became of a signal's would-be trade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// It filled.
    Filled(Simulated),
    /// No traded unit at or after the signal close that day.
    NotFilled,
    /// No minute with both a Zerodha and a GDFL trade before the signal close: no stop.
    NoRatio,
    /// The entry price buys less than one share.
    ZeroQuantity,
}

/// The stop level for `band` at instant `t`, mapped on a GDFL day and as-is on a fallback day.
fn level_at(tape: &Tape<'_>, band: i128, t: u32, side: Side) -> Option<i64> {
    if tape.fallback {
        return unmapped_level(band).ok();
    }
    let ratio = ratio_source(tape.minutes, tape.seconds, t)?;
    map_level(band, &ratio, side).ok()
}

fn fill(prints: &[Print], i: usize, side: Side) -> Option<Fill> {
    prints.get(i).map(|p| Fill {
        second: p.slot,
        price: exit_price(p, side),
    })
}

/// F39: simulates the trade of a signal on `side` at candle close `t`, whose own side band
/// is `band`, against the day's later candles and its tape.
#[must_use]
pub fn simulate(side: Side, t: u32, band: i128, later: &[Later], tape: &Tape<'_>) -> Outcome {
    let prints = if tape.fallback {
        minute_prints(tape.minutes, tape.close)
    } else {
        second_prints(tape.seconds)
    };
    let Some((e, entry_price)) = entry(&prints, t, side) else {
        return Outcome::NotFilled;
    };
    let Some(first_stop) = level_at(tape, band, t, side) else {
        return Outcome::NoRatio;
    };
    let shares = qty(entry_price);
    if shares <= 0 {
        return Outcome::ZeroQuantity;
    }
    let entry_print = prints.get(e).copied().unwrap_or(Print {
        start: t,
        slot: t,
        high: entry_price,
        low: entry_price,
    });
    let late = entry_print.slot > t + 60;
    let mut trade = Simulated {
        entry: Fill {
            second: entry_print.slot,
            price: entry_price,
        },
        qty: shares,
        exit: Fill {
            second: tape.close,
            price: entry_price,
        },
        kind: ExitKind::Unclosable,
        fallback: tape.fallback,
        late,
        first_stop,
        last_stop: first_stop,
        no_ratio_trail: 0,
    };
    let (kind, at) = walk(side, later, tape, &prints, e, entry_print, &mut trade);
    trade.kind = kind;
    if let Some(exit) = at.and_then(|i| fill(&prints, i, side)) {
        trade.exit = exit;
    } else {
        let price = worst_since(&prints, e, side).unwrap_or(entry_price);
        trade.exit = Fill {
            second: tape.close,
            price,
        };
    }
    Outcome::Filled(trade)
}

/// The exit walk: the kind and the index of the exit print (`None` for unclosable).
fn walk(
    side: Side,
    later: &[Later],
    tape: &Tape<'_>,
    prints: &[Print],
    entry_at: usize,
    entry_print: Print,
    trade: &mut Simulated,
) -> (ExitKind, Option<usize>) {
    let mut scan = StopScan {
        stop: trade.last_stop,
        side,
        cursor: entry_at + 1,
    };
    let candles = later
        .iter()
        .filter(|c| c.t > entry_print.slot && c.t <= TIME_EXIT);
    for candle in candles {
        if let Some(at) = scan.until(prints, candle.t) {
            return (ExitKind::Stop, Some(at));
        }
        let mapped = if candle.hole {
            None
        } else {
            level_at(tape, candle.band, candle.t, side)
        };
        match step(
            side,
            candle.close,
            candle.band,
            candle.hole,
            mapped,
            scan.stop,
        ) {
            Step::Flip => {
                let at = first_at_or_after(prints, candle.t).max(entry_at + 1);
                return match prints.get(at) {
                    None => (ExitKind::Unclosable, None),
                    Some(print) if print.start >= SQUARE_OFF => (ExitKind::SquareOff, Some(at)),
                    Some(_) => (ExitKind::Flip, Some(at)),
                };
            }
            Step::Move(level) => {
                scan.stop = level;
                trade.last_stop = level;
            }
            Step::NoRatio => trade.no_ratio_trail += 1,
            Step::Hold => {}
        }
    }
    let time_rule = entry_print.start < TIME_EXIT;
    let limit = if time_rule { TIME_EXIT } else { SQUARE_OFF };
    if let Some(at) = scan.until(prints, limit) {
        return (ExitKind::Stop, Some(at));
    }
    let cursor = scan.cursor;
    let Some(print) = prints.get(cursor) else {
        return (ExitKind::Unclosable, None);
    };
    let kind = if reaches(trade.last_stop, print, side) {
        ExitKind::Stop
    } else if time_rule && print.start < SQUARE_OFF {
        ExitKind::Time
    } else {
        ExitKind::SquareOff
    };
    (kind, Some(cursor))
}

#[cfg(test)]
#[path = "trade_tests.rs"]
mod tests;
