//! The seven reading states, each rewritten from its published, written definition
//! (the kit's PREVIEW-SPEC section 3.1 and the design texts it cites), never from code.
//!
//! Every state folds one candle at a time and keeps fixed-size state; a period's ring
//! is allocated once. All arithmetic is `i128` on paisa; the smoothed averages are held
//! scaled by [`SCALE`] = 10^6 and floored with `div_euclid`.

pub mod atr;
pub mod bollinger;
pub mod cci;
pub mod ema;
pub mod rsi;
pub mod supertrend;
pub mod willr;

use crate::market::Bar;

/// The fixed-point scale of every smoothed average: 10^6 units per paisa.
pub const SCALE: i128 = 1_000_000;

/// Why a candle cannot be evaluated (the candle's own check).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corrupt {
    /// A price at or below zero.
    NonPositive,
    /// High below low.
    Inverted,
    /// Open or close outside the low..=high range.
    OutOfRange,
    /// A negative volume.
    NegativeVolume,
}

/// The candle check every reading applies first: every price positive, `low <= high`,
/// open and close inside the range, volume not negative.
///
/// # Errors
/// The first failing rule, in the order above.
pub const fn check_evaluable(bar: &Bar) -> Result<(), Corrupt> {
    if bar.low <= 0 || bar.high <= 0 || bar.open <= 0 || bar.close <= 0 {
        return Err(Corrupt::NonPositive);
    }
    if bar.high < bar.low {
        return Err(Corrupt::Inverted);
    }
    if bar.open < bar.low || bar.open > bar.high || bar.close < bar.low || bar.close > bar.high {
        return Err(Corrupt::OutOfRange);
    }
    if bar.volume < 0 {
        return Err(Corrupt::NegativeVolume);
    }
    Ok(())
}

/// A Supertrend direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Trend {
    /// The line is the lower band.
    Up,
    /// The line is the upper band.
    Down,
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
