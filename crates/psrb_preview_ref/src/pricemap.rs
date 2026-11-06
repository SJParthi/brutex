//! F31 `pricemap::ratio_source` and F32 `pricemap::map_level`: a Zerodha (adjusted)
//! level decided at instant T mapped to GDFL's raw prices by the in-session ratio
//! (PREVIEW-SPEC section 3.5, "The stop, mapped to raw").

use crate::cond::Side;
use crate::market::{Minute, Second};
use crate::money::{ceil_div, floor_div};
use crate::time::OPEN;

/// The ratio's two closes and the minute they come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ratio {
    /// Z: the Zerodha minute's close.
    pub z: i64,
    /// G: the close of the last GDFL second inside that minute.
    pub g: i64,
    /// The minute's start.
    pub minute: u32,
}

/// The close of the last GDFL second inside `[start, start + 60)`, if any.
fn last_second_close(seconds: &[Second], start: u32) -> Option<i64> {
    let end = seconds.partition_point(|s| s.at < start + 60);
    let last = end.checked_sub(1).and_then(|i| seconds.get(i))?;
    (last.at >= start).then_some(last.bar.close)
}

/// F31: Z is the close of the last stored Zerodha minute with volume above 0 ending at or
/// before `t`, G the close of the last GDFL second inside that minute; when that minute
/// has no GDFL trade, the latest earlier session minute that has both. `None`: "no ratio".
#[must_use]
pub fn ratio_source(minutes: &[Minute], seconds: &[Second], t: u32) -> Option<Ratio> {
    minutes
        .iter()
        .rev()
        .filter(|m| !m.padded() && m.start >= OPEN && m.start + 60 <= t)
        .find_map(|m| last_second_close(seconds, m.start).map(|g| Ratio { z: m.bar.close, g, minute: m.start }))
}

/// Why a mapped level was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Z is not positive: never divided by.
    ZNotPositive,
    /// The mapped level does not fit `i64`.
    BeyondI64,
    /// The mapped level is not positive.
    NotPositive,
}

/// F32: `L_g = L_z x G / Z` in `i128`, rounded against the position: a long's stop
/// floored, a short's ceiled.
///
/// # Errors
/// [`Refusal`] for a non-positive Z, a level beyond `i64`, or a level at or below 0.
pub fn map_level(level: i128, ratio: &Ratio, side: Side) -> Result<i64, Refusal> {
    let z = i128::from(ratio.z);
    if z <= 0 {
        return Err(Refusal::ZNotPositive);
    }
    let product = level * i128::from(ratio.g);
    let mapped = match side {
        Side::Long => floor_div(product, z),
        Side::Short => ceil_div(product, z),
    };
    let mapped = i64::try_from(mapped).map_err(|_| Refusal::BeyondI64)?;
    if mapped <= 0 {
        return Err(Refusal::NotPositive);
    }
    Ok(mapped)
}

/// A level used without mapping (a fallback day: one basis).
///
/// # Errors
/// [`Refusal::BeyondI64`] or [`Refusal::NotPositive`].
pub fn unmapped_level(level: i128) -> Result<i64, Refusal> {
    let v = i64::try_from(level).map_err(|_| Refusal::BeyondI64)?;
    if v <= 0 {
        return Err(Refusal::NotPositive);
    }
    Ok(v)
}

#[cfg(test)]
#[path = "pricemap_tests.rs"]
mod tests;
