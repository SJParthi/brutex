//! The sniper gates S-STOP and S-RR (PREVIEW-SPEC 24.1, SNIPER.md 1.2,
//! with 24.13's planned-net S-RR).
//!
//! Read only what is known at the decision T: REF (the last GDFL print known
//! at T), and the stop and first target 7.3 fixes, mapped by R1 "never
//! kinder" (a long floors both, a short ceils both: the stop farther, the
//! reward smaller) and not rounded to the tick.
//!
//! ```text
//! D_stop = REF - map(stop_Z)          (a short: map(stop_Z) - REF)
//! D_tgt  = max(0, map(target_Z) - REF) (a short: max(0, REF - map(target_Z)))
//! S-STOP = ceil(D_stop x 1,000,000 / REF)          ppm of REF
//! S-RR   = floor(1000 x (D_tgt x q - charges) / (D_stop x q + charges))
//! q      = floor(amount / REF); charges = one round trip at REF
//! ```

use crate::charges::{Order, OrderSide, price_equity};
use crate::clock::Date;
use crate::model::{Readings, Side, Status};
use crate::money::{ceil_div, floor_div};

/// R1's gate mapping of a Zerodha price (milli-paise) to paise: floor on a
/// long, ceil on a short; no tick.
pub fn map_gate(x_milli: i128, z: i64, g: i64, side: Side) -> i128 {
    let num = x_milli * i128::from(g);
    let den = i128::from(z) * 1000;
    if side.long() {
        floor_div(num, den)
    } else {
        ceil_div(num, den)
    }
}

/// The two distances at the decision, paise: `(D_stop, D_tgt)`; `D_tgt` is
/// `None` on a lane with no price at T.
pub fn distances(
    side: Side,
    z: i64,
    g: i64,
    ref_px: i64,
    stop_milli: i128,
    target_milli: Option<i128>,
) -> (i128, Option<i128>) {
    let r = i128::from(ref_px);
    let stop = map_gate(stop_milli, z, g, side);
    let d_stop = if side.long() { r - stop } else { stop - r };
    let d_tgt = target_milli.map(|t| {
        let t = map_gate(t, z, g, side);
        (if side.long() { t - r } else { r - t }).max(0)
    });
    (d_stop, d_tgt)
}

/// S-STOP in ppm of REF, or the refusal by name.
///
/// # Errors
/// `sn_stop_through_at_decision` when `D_stop <= 0`.
pub fn s_stop(d_stop: i128, ref_px: i64) -> Result<i128, Status> {
    if d_stop <= 0 {
        return Err(Status::SnStopThroughAtDecision);
    }
    Ok(ceil_div(d_stop * 1_000_000, i128::from(ref_px.max(1))))
}

/// SNIPER.md 1.2's price reading of S-RR (overridden by 24.13; kept for the
/// worked vectors).
///
/// # Errors
/// `s_rr_no_target` on a lane with no price at T; `sn_stop_through_at_decision`.
pub fn s_rr_price(d_stop: i128, d_tgt: Option<i128>) -> Result<i128, Status> {
    let d_tgt = d_tgt.ok_or(Status::SRrNoTarget)?;
    if d_stop <= 0 {
        return Err(Status::SnStopThroughAtDecision);
    }
    Ok(floor_div(d_tgt * 1000, d_stop))
}

/// 24.13's S-RR on planned net, with the charges of the round trip given.
///
/// # Errors
/// `s_rr_no_target`, `sn_stop_through_at_decision`, or `sn_no_quantity` at q = 0.
pub fn s_rr_net(
    d_stop: i128,
    d_tgt: Option<i128>,
    qty: i128,
    charges: i128,
) -> Result<i128, Status> {
    let d_tgt = d_tgt.ok_or(Status::SRrNoTarget)?;
    if d_stop <= 0 {
        return Err(Status::SnStopThroughAtDecision);
    }
    if qty <= 0 {
        return Err(Status::SnNoQuantity);
    }
    Ok(floor_div(
        1000 * (d_tgt * qty - charges),
        d_stop * qty + charges,
    ))
}

/// The planned quantity `floor(amount / REF)`.
pub fn planned_qty(amount: i64, ref_px: i64) -> i128 {
    floor_div(i128::from(amount), i128::from(ref_px.max(1)))
}

/// The planned round trip's charges: buy `qty` at REF and sell `qty` at REF
/// on the decision's date, the official reading's headline (24.13, 24.V).
pub fn planned_charges(date: Date, qty: i128, ref_px: i64, r: &Readings) -> i128 {
    let q = i64::try_from(qty).unwrap_or(0);
    let orders = [
        Order {
            side: OrderSide::Buy,
            fills: vec![(q, ref_px)],
            square_off: false,
        },
        Order {
            side: OrderSide::Sell,
            fills: vec![(q, ref_px)],
            square_off: false,
        },
    ];
    price_equity(date, &orders, r).headline()
}

/// The planned round trip's charges on the worst reading's stress line (what
/// 24.13's own vectors used).
pub fn planned_charges_worst(date: Date, qty: i128, ref_px: i64, r: &Readings) -> i128 {
    let q = i64::try_from(qty).unwrap_or(0);
    let orders = [
        Order {
            side: OrderSide::Buy,
            fills: vec![(q, ref_px)],
            square_off: false,
        },
        Order {
            side: OrderSide::Sell,
            fills: vec![(q, ref_px)],
            square_off: false,
        },
    ];
    price_equity(date, &orders, r).stress()
}

#[cfg(test)]
#[path = "gates_tests.rs"]
mod tests;
