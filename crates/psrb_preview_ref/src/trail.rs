//! F33 `trail::Trailing`: after each later candle closes at or before 15:07:59 with the
//! trade open, a close across its own side band exits by the flip; else the stop moves
//! to the candle's side band, mapped at its close, if tighter, never back. A hole candle
//! moves nothing and fires no flip (PREVIEW-SPEC section 3.5, "Trailing").

use crate::cond::Side;

/// The close is across the candle's own side band: a long's below its lower band, a
/// short's above its upper band (equality is not across).
#[must_use]
pub fn across(close: i64, band: i128, side: Side) -> bool {
    let close = i128::from(close);
    match side {
        Side::Long => close < band,
        Side::Short => close > band,
    }
}

/// A new stop is tighter: higher for a long, lower for a short (equality is not).
#[must_use]
pub const fn tighter(new: i64, stop: i64, side: Side) -> bool {
    match side {
        Side::Long => new > stop,
        Side::Short => new < stop,
    }
}

/// What one later candle does to an open trade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The flip exit.
    Flip,
    /// The stop moves to this level.
    Move(i64),
    /// Nothing (a hole, a looser or equal band, or a band that could not be mapped).
    Hold,
    /// The band could not be mapped: the stop stays, counted "no ratio (trail)".
    NoRatio,
}

/// One trailing decision: `mapped` is the candle's side band mapped at its close, or
/// `None` when it could not be mapped.
#[must_use]
pub fn step(
    side: Side,
    close: i64,
    band: i128,
    hole: bool,
    mapped: Option<i64>,
    stop: i64,
) -> Step {
    if hole {
        return Step::Hold;
    }
    if across(close, band, side) {
        return Step::Flip;
    }
    match mapped {
        None => Step::NoRatio,
        Some(level) if tighter(level, stop, side) => Step::Move(level),
        Some(_) => Step::Hold,
    }
}

#[cfg(test)]
#[path = "trail_tests.rs"]
mod tests;
