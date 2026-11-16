//! Sizing and protection prices (design 4.2 `limits`; S8, S11): the protection limit (the dated tick is for LIMITS only, never in a fill
//! price), the old HARD cap price (the `req70-2s-cap` comparison only), the quantity of a slot, and the sizing references: crate v1's seed
//! reading (sample-6's `expected_h`: REQ 58b's decision reference plus or less one dated tick) and, from change C4, the standard's.

use crate::tape::{Row, Tape, valid};
use crate::{Paisa, Side, Units, Us};

/// x rounded DOWN to the tick grid `tk` (sample-6's `round_down_t`).
#[must_use]
pub const fn round_down(x: Paisa, tk: Paisa) -> Paisa {
    let t = if tk > 1 { tk } else { 1 };
    x.div_euclid(t) * t
}

/// x rounded UP to the tick grid `tk` (sample-6's `round_up_t`).
#[must_use]
pub const fn round_up(x: Paisa, tk: Paisa) -> Paisa {
    let r = round_down(x, tk);
    // Saturating, never wrapping (OB7, G0 repair 2).
    if r == x { r } else { r.saturating_add(if tk > 1 { tk } else { 1 }) }
}

/// ceil(a / b) for b > 0 (sample-6's `ceil_div`).
#[must_use]
pub const fn ceil_div(a: i128, b: i128) -> i128 {
    let q = a.div_euclid(b);
    if q * b == a { q } else { q + 1 }
}

/// S8: Dhan's market protection limit from the last print `ltp`, `bp` basis points, rounded AGAINST the order on the dated tick `tick`
/// (sample-6's `protect_t`, exactly): a band of max(ceil(ltp x bp / 10,000), tick); a buy's limit ltp + band rounded down, a sale's ltp -
/// band rounded up and never below one tick.
#[must_use]
pub fn protection(ltp: Paisa, side: Side, bp: u32, tick: Paisa) -> Paisa {
    let band = i64::try_from(ceil_div(i128::from(ltp) * i128::from(bp), 10_000)).unwrap_or(0).max(tick);
    match side {
        Side::Buy => round_down(ltp.saturating_add(band), tick),
        Side::Sell => round_up(ltp.saturating_sub(band), tick).max(tick),
    }
}

/// The old HARD money cap price per unit (the `req70-2s-cap` comparison only, design 5.1; sample-6's `dial70`): a long's floor(slot x
/// leverage / qty) rounded down to the tick; a short's floor(slot x leverage / qty) paisa, the value cap. 0 for a quantity of 0.
#[must_use]
pub fn cap_price(slot: Paisa, lev_num: u32, lev_den: u32, qty: Units, side: Side, tick: Paisa) -> Paisa {
    if qty == 0 || slot <= 0 || lev_den == 0 {
        return 0;
    }
    let raw = i128::from(slot) * i128::from(lev_num) / i128::from(lev_den) / i128::from(qty);
    let raw = i64::try_from(raw).unwrap_or(i64::MAX);
    match side {
        Side::Buy => round_down(raw, tick),
        Side::Sell => raw,
    }
}

/// S11's quantity: floor(slot x leverage / reference / unit) x unit (sample-6's `nearest_lots`, which floors). 0 when the reference is
/// not above 0.
#[must_use]
pub fn size_units(slot: Paisa, lev_num: u32, lev_den: u32, refpx: Paisa, unit: Units) -> Units {
    let u = unit.max(1);
    if refpx <= 0 || slot <= 0 || lev_den == 0 {
        return 0;
    }
    let exposure = i128::from(slot) * i128::from(lev_num) / i128::from(lev_den);
    let n = exposure / i128::from(refpx);
    Units::try_from(n.max(0)).unwrap_or(Units::MAX) / u * u
}

/// CHANGE C4 (S11, D4): THE size reference: the last traded second stamped strictly before `t` (the order's anchor T: the bell under
/// REQ 70c), merged over a repeated stamp, its HIGH for a buy, its LOW for a sale, NO tick (the tick is a REQ 58b fill term the operator
/// dropped: "no extra ticks"). It reads no data at or after T. None when nothing traded before T (`no_reference`, the host counts it).
/// Crate v1 / sample-6 sized on [`size_ref_seed`] (REQ 58b's decision reference plus or less one dated tick).
#[must_use]
pub fn size_ref<R: Row>(tape: &Tape<'_, R>, t: Us, side: Side) -> Option<Paisa> {
    let s = tape.last_traded_before(t)?;
    // C6 (S3, S11): "if no traded second exists since the segment opened, there is no trade" (the segment holding the second before T).
    let open = tape.segment_open_at(t.saturating_sub(crate::US_S))?;
    if s.ts < open {
        return None;
    }
    Some(match side {
        Side::Buy => s.high,
        Side::Sell => s.low,
    })
}

/// Crate v1's sizing reference (the seed: sample-6's `expected_h`, exactly): the last printed row stamped strictly before `t_size`, its
/// HIGH for a buy, its LOW for a sale, plus (a buy) or less (a sale) ONE dated tick, in HALF-PAISA (never below one half-paisa). None
/// when nothing printed before `t_size`. This is REQ 58b's decision reference read at sample-6's `T69::size_at`.
#[must_use]
pub fn size_ref_seed<R: Row>(tape: &Tape<'_, R>, t_size: Us, side: Side, tick: Paisa) -> Option<i128> {
    let i = tape.first_at(t_size);
    let w = tape.rows().get(..i)?;
    // sample-6's `ref_hl`: the row just before (its rows are traded seconds); a non-print row is never a reference.
    let s = w.iter().rev().find(|s| valid(*s))?;
    let p = 2 * i128::from(match side {
        Side::Buy => s.high(),
        Side::Sell => s.low(),
    });
    let slip = 2 * i128::from(tick);
    Some(match side {
        Side::Buy => p + slip,
        Side::Sell => (p - slip).max(1),
    })
}

/// S18 (D15, D16): the fill module REFUSES a STOCK option on its own expiry day (the selector's guard: stock options use the next expiry on
/// their own expiry day; index options keep expiry-day trading, `stock_option` false).
///
/// # Errors
/// [`FillErr::StockOptionOnOwnExpiry`] for a stock option whose expiry is the trading day.
pub const fn expiry_guard(stock_option: bool, day: i64, expiry: i64) -> Result<(), crate::FillErr> {
    if stock_option && day == expiry { Err(crate::FillErr::StockOptionOnOwnExpiry) } else { Ok(()) }
}

/// S15 (D16): an INDEX option still open at the end of its own expiry day SETTLES at intrinsic value against NSE's OFFICIAL closing value
/// of the index for that day (the final settlement price, from the dated index close file the host pins), not the last print: a CE
/// max(0, close - strike), a PE max(0, strike - close), paisa a unit. A settlement, not a fill (labelled and counted by the host).
///
/// # Errors
/// [`crate::FillErr::Overflow`] when the difference leaves i64 (checked paisa).
pub const fn settle_intrinsic(call: bool, strike: Paisa, official_close: Paisa) -> Result<Paisa, crate::FillErr> {
    let d = if call { official_close.checked_sub(strike) } else { strike.checked_sub(official_close) };
    match d {
        Some(v) if v > 0 => Ok(v),
        Some(_) => Ok(0),
        None => Err(crate::FillErr::Overflow),
    }
}
