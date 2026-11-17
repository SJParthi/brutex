//! v3.1 (MAIN-RULINGS-ENGINE-v22 addendum 3, operator 15 Nov 08:34 and decision (c)): the SUDDEN-JUMP stress and the measured adverse
//! moves.
//!
//! - The jump stress re-prices a BOOKED trade (a stress, not a fill): every sale k% lower and / or every buy-back k% higher than the
//!   recorded worst print, k = 1, 2, 5, 10, 25, 50, 100%, exits only and both sides. On a trade's totals: sold' = floor(sold x (1 - k)),
//!   bought' = ceil(bought x (1 + k)) (rounded against the trader), the charges recomputed on the new values by the same estimator. O(1) per
//!   trade. The never-closed units' valuation is a buy and is stressed with the buy-backs.
//! - The headline gate: a pick counts only if it stays positive at +2% adverse on both sides (spread plus impact; the operator's GRASIM
//!   screen shows a 2.4% bid-ask spread). The sweep carries every account's net at the gate stress (`Out::net_s2`, summed per account).
//! - The adverse move: the largest move against the order in the first 2, 5 and 10 s after a decision (a sale: the lowest LOW below the
//!   last print before it; a buy-back: the highest HIGH above it), from the raw seconds, in basis points of that last print.

use common_fill::{Side, Tape};
use common_fill::limits::size_ref;

/// The stress levels (bp of the price).
pub const KS_BP: [i64; 7] = [100, 200, 500, 1_000, 2_500, 5_000, 10_000];
/// The adverse-move windows (seconds).
pub const ADV_S: [i64; 3] = [2, 5, 10];

/// One trade's money as the stress reads it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Money {
    pub day: i64,
    pub units: u64,
    pub sold: i64,
    /// Everything paid to close: buy-backs + square-off + the never-closed units' valuation.
    pub bought: i64,
    /// The buy side's own exit orders (a buy-back; the own 15:10 square-off under the switch) and Dhan's square-off.
    pub own_orders: u32,
    pub broker: bool,
    /// v3.2 addendum 10: (sale-side, own buy-side, Dhan square-off) executed orders, every filling block x every freeze slice (None: v3.1's
    /// count above). The gate re-prices with the SAME orders.
    pub orders: Option<(u32, u32, u32)>,
}

/// The trade's net with every buy k bp dearer and (when `both`) every sale k bp cheaper; charges recomputed.
///
/// # Errors
/// A charge-rate refusal.
pub fn stressed(m: &Money, k_bp: i64, both: bool) -> Result<i64, String> {
    if m.units == 0 {
        return Ok(0);
    }
    let up = |v: i64, k: i64| -> i64 { i64::try_from((i128::from(v) * i128::from(10_000 + k) + 9_999).div_euclid(10_000)).unwrap_or(i64::MAX) };
    let down = |v: i64, k: i64| -> i64 { i64::try_from((i128::from(v) * i128::from(10_000 - k)).div_euclid(10_000)).unwrap_or(i64::MIN) };
    // MUTANT m_v31_stress_sign: the buy side made CHEAPER (the stress turned into a gift)
    let bought = if cfg!(feature = "m_v31_stress_sign") { down(m.bought, k_bp) } else { up(m.bought, k_bp) };
    let sold = if both { down(m.sold, k_bp) } else { m.sold };
    let (c, _, _, _) = match m.orders {
        Some((s, o, b)) => crate::trade::charges_orders_n(m.day, m.units, bought, sold, s, o, b)?,
        None => crate::trade::charges_orders(m.day, m.units, bought, sold, m.own_orders, m.broker)?,
    };
    Ok(sold - bought - c)
}

/// The largest adverse move in the first `secs` seconds after instant `at` for an order of side `side`, in bp of the last print before
/// `at` (0 when the price never moved against the order, or no print before / in the window).
#[must_use]
pub fn adverse_bp(tape: &Tape<'_>, at: i64, secs: i64, side: Side) -> i64 {
    let Some(r) = size_ref(tape, at, side) else { return 0 };
    if r <= 0 {
        return 0;
    }
    let to = at + secs * crate::cal::US_S;
    // matrix X65: O(1) range extremes through the range table
    match side {
        Side::Sell => tape.extreme(at, to, false).map_or(0, |(lo, _)| ((r - lo).max(0) * 10_000) / r),
        Side::Buy => tape.extreme(at, to, true).map_or(0, |(hi, _)| ((hi - r).max(0) * 10_000) / r),
    }
}

/// p50 / p90 / p99 / max of a sample (bp), sorted copy.
#[must_use]
pub fn dist(v: &[i64]) -> (i64, i64, i64, i64) {
    let mut s = v.to_vec();
    s.sort_unstable();
    let at = |p: usize| s.get(s.len().saturating_sub(1) * p / 100).copied().unwrap_or(0);
    (at(50), at(90), at(99), s.last().copied().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v31_stress_reprices_against_the_trader() {
        let d = crate::cal::days_from_civil(2026, 11, 4);
        let m = Money { day: d, units: 100, sold: 100 * 1_000, bought: 100 * 800, own_orders: 1, broker: false, orders: None };
        let base = stressed(&m, 0, true).unwrap_or_else(|e| panic!("{e}"));
        let s2 = stressed(&m, 200, true).unwrap_or_else(|e| panic!("{e}"));
        let s2x = stressed(&m, 200, false).unwrap_or_else(|e| panic!("{e}"));
        // both sides at 2%: sold 98,000, bought 81,600: the gross falls by 3,600 paisa; exits only: bought 81,600, the gross by 1,600
        assert!(s2 < s2x && s2x < base, "{s2} < {s2x} < {base}");
        let gross = |n: i64, sold: i64, bought: i64| n - (sold - bought);
        assert!(gross(s2, 98_000, 81_600).abs() < 10_000, "the charges (Rs 20 a side brokerage and taxes) stay small");
        // 100% on the buy side doubles every buy-back
        let s100 = stressed(&m, 10_000, false).unwrap_or_else(|e| panic!("{e}"));
        assert!(s100 < 100 * 1_000 - 2 * 100 * 800 + 1);
        assert_eq!(stressed(&Money { units: 0, ..m }, 500, true), Ok(0));
    }
}
