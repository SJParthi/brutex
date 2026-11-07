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
mod tests {
    use super::*;

    #[test]
    fn sniper_md_worked_vectors() {
        // (side, Z, G, REF, stop_Z paise, target_Z paise or none) -> (D_stop, S-STOP, D_tgt, S-RR price)
        type Row = (
            Side,
            i64,
            i64,
            i64,
            i64,
            Option<i64>,
            i128,
            Result<i128, Status>,
            Option<i128>,
            Result<i128, Status>,
        );
        let rows: [Row; 6] = [
            (
                Side::Long,
                100_000,
                100_020,
                100_020,
                99_695,
                Some(100_605),
                306,
                Ok(3_060),
                Some(605),
                Ok(1_977),
            ),
            (
                Side::Short,
                100_000,
                99_980,
                99_980,
                100_305,
                Some(99_395),
                305,
                Ok(3_051),
                Some(604),
                Ok(1_980),
            ),
            (
                Side::Long,
                100_000,
                100_000,
                99_200,
                99_695,
                Some(100_605),
                -495,
                Err(Status::SnStopThroughAtDecision),
                Some(1_405),
                Err(Status::SnStopThroughAtDecision),
            ),
            (
                Side::Long,
                100_000,
                100_020,
                100_020,
                99_695,
                Some(99_900),
                306,
                Ok(3_060),
                Some(0),
                Ok(0),
            ),
            (
                Side::Long,
                100_000,
                100_000,
                100_000,
                99_999,
                Some(3_100_000),
                1,
                Ok(10),
                Some(3_000_000),
                Ok(3_000_000_000),
            ),
            (
                Side::Long,
                100_000,
                100_000,
                100_000,
                99_990,
                Some(100_020),
                10,
                Ok(100),
                Some(20),
                Ok(2_000),
            ),
        ];
        for (side, z, g, r, stop, tgt, d_stop, sstop, d_tgt, srr) in rows {
            let (ds, dt) = distances(
                side,
                z,
                g,
                r,
                i128::from(stop) * 1000,
                tgt.map(|t| i128::from(t) * 1000),
            );
            assert_eq!(ds, d_stop);
            assert_eq!(dt, d_tgt);
            assert_eq!(s_stop(ds, r), sstop);
            assert_eq!(s_rr_price(ds, dt), srr);
        }
        assert_eq!(s_rr_price(10, None), Err(Status::SRrNoTarget));
    }

    #[test]
    fn the_24_13_vectors_with_their_own_charges() {
        // (qty, D_stop, D_tgt, charges) -> S-RR
        for (q, ds, dt, ch, want) in [
            (1_000, 10, 50, 41_677, 161),
            (1_000, 10, 20, 41_677, -420),
            (250, 10, 50, 15_167, -151),
            (999, 306, 605, 41_671, 1_619),
            (999, 306, 0, 41_671, -120),
            (1_000, 2_500, 12_500, 41_677, 4_901),
        ] {
            assert_eq!(
                s_rr_net(ds, Some(dt), q, ch),
                Ok(want),
                "{q} {ds} {dt} {ch}"
            );
        }
        // charges 0 equal the price reading
        assert_eq!(s_rr_net(306, Some(605), 999, 0), s_rr_price(306, Some(605)));
        assert_eq!(
            s_rr_net(0, Some(5), 10, 0),
            Err(Status::SnStopThroughAtDecision)
        );
        assert_eq!(s_rr_net(5, Some(5), 0, 0), Err(Status::SnNoQuantity));
        assert_eq!(s_rr_net(5, None, 10, 0), Err(Status::SRrNoTarget));
        // D_stop = 1, a huge charge: floor of a large negative ratio
        assert_eq!(s_rr_net(1, Some(0), 1, 1_000_000), Ok(-1000));
    }

    #[test]
    fn those_charges_are_the_worst_reading_and_the_official_one_differs() {
        let d = Date {
            y: 2026,
            m: 9,
            d: 9,
        };
        let r = Readings::HEADLINE;
        assert_eq!(planned_charges_worst(d, 1_000, 100_000, &r), 41_677);
        assert_eq!(planned_charges_worst(d, 250, 100_000, &r), 15_167);
        assert_eq!(planned_charges_worst(d, 999, 100_020, &r), 41_671);
        assert_eq!(planned_charges(d, 1_000, 100_000, &r), 40_201);
        assert_eq!(planned_charges(d, 250, 100_000, &r), 13_690);
        assert_eq!(planned_charges(d, 999, 100_020, &r), 40_195);
        // the six S-RR values on the headline charges
        let p = |q: i128, ds: i128, dt: i128, refpx: i64| {
            s_rr_net(ds, Some(dt), q, planned_charges(d, q, refpx, &r))
        };
        assert_eq!(p(1_000, 10, 50, 100_000), Ok(195));
        assert_eq!(p(1_000, 10, 20, 100_000), Ok(-403));
        assert_eq!(p(250, 10, 50, 100_000), Ok(-74));
        assert_eq!(p(999, 306, 605, 100_020), Ok(1_631));
        assert_eq!(p(999, 306, 0, 100_020), Ok(-117));
        assert_eq!(p(1_000, 2_500, 12_500, 100_000), Ok(4_905));
        assert_eq!(planned_qty(100_000_000, 100_000), 1_000);
        assert_eq!(planned_qty(25_000_000, 100_020), 249);
        assert_eq!(planned_qty(5, 100_000), 0);
        assert_eq!(planned_qty(5, 0), 5);
    }

    #[test]
    fn map_gate_floors_a_long_and_ceils_a_short() {
        assert_eq!(map_gate(99_695_000, 100_000, 100_020, Side::Long), 99_714);
        assert_eq!(map_gate(99_695_000, 100_000, 100_020, Side::Short), 99_715);
        assert_eq!(
            map_gate(100_000_000, 100_000, 100_000, Side::Short),
            100_000
        );
        assert_eq!(s_stop(1, 0), Ok(1_000_000)); // REF read as at least one paisa
    }
}
