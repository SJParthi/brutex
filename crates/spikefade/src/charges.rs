//! Dhan's option charges, COPIED from the opening robot's option version (scratch/opening5s-4r70/crates/stk_opt_preview/src/options.rs
//! `Rates`, `rates_on`, `charges_est`, `charged` with `Reading::Estimator`, `today`, `trade_day`, `BROKERAGE_PER_ORDER`, `E9`): Dhan's
//! Order Estimator reading at the DEARER of the dated rate set and today's (2026-10-30) set (item 26). Only the fields of the robot's
//! `Trip` that the estimator reads are kept ([`Trip`]); no settlement arises (a stock option is never held to its expiry: never traded
//! on its expiry day and squared off intraday), so the settled branch is kept verbatim but never taken.

use brutex_core::instrument::Exchange;
use costs::day::TradeDay;

use crate::cal::{civil, days_from_civil};

/// Paise per crore = rupees per crore x 100; a fraction is rate / 10^9.
const E9: i128 = 1_000_000_000;

/// Dhan's brokerage on F&O: Rs 20 per executed order.
pub(crate) const BROKERAGE_PER_ORDER: i128 = 2_000;

/// The estimator's inputs of one option round trip (the robot's `Trip`, the fields `charges_est` reads).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Trip {
    pub(crate) qty: i64,
    pub(crate) buy_value: i64,
    pub(crate) sell_value: i64,
    pub(crate) entry_orders: u32,
    pub(crate) exit_orders: u32,
    pub(crate) broker_orders: u32,
    pub(crate) rem_kind: u8,
    pub(crate) settle_value: i64,
    pub(crate) settle_notional: i64,
}

/// The dated rates of one day (paise per crore of premium). COPIED.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Rates {
    pub(crate) exchange: i128,
    pub(crate) ipft: i128,
    pub(crate) sebi: i128,
    pub(crate) stt: i128,
    pub(crate) stt_ex: i128,
    pub(crate) stt_ex_on_notional: bool,
    pub(crate) stamp_buy: i128,
    pub(crate) stamp_both: i128,
    pub(crate) gst: i128,
    /// fix:v33 GST (standing F4, exits F-B2): GST 18% also on the SEBI fee (NSE FATAX52993: from 2022-07-18); false = v3.3.
    pub(crate) gst_on_sebi: bool,
}

pub(crate) fn trade_day(d: i64) -> Option<TradeDay> {
    let (y, m, dd) = civil(d);
    TradeDay::new(u16::try_from(y).ok()?, u8::try_from(m).ok()?, u8::try_from(dd).ok()?).ok()
}

/// "Today's" rate set for item 26 (the latest set the robot holds: 2026-10-30). COPIED.
pub(crate) const fn today() -> i64 {
    days_from_civil(2026, 10, 30)
}

/// The rates in force on day `d`. COPIED (robot options.rs `rates_on`; the row numbers it printed are dropped).
pub(crate) fn rates_on(d: i64) -> Result<Rates, String> {
    let td = trade_day(d).ok_or_else(|| format!("trade day {d}"))?;
    let day = |y: i64, m: i64, dd: i64| days_from_civil(y, m, dd);
    // REPAIR A (check-engine-1 M1): the rows below 2024-10-01 are NSE's own circulars, downloaded 15 Nov 2026 (v32/nse-dl/circulars-fa,
    // sha256 in its manifest): equity options, "More than Rs 3 Crores up to Rs 100 Crores" premium slab (the first per-lakh slab; brokers pass
    // the exchange charge through at it): FA46730 (w.e.f. 2021-01-01: Rs 50.00 -> Rs 53.00 per lakh of premium), FA56129 (w.e.f.
    // 2023-04-01: Rs 53.00 -> Rs 50.00; IPFT Rs 0.01 -> Rs 50 per crore of premium), FA61137 (w.e.f. 2024-04-01: Rs 50.00 -> Rs 49.50).
    // Rs 50.00 per lakh = 5,000 per crore. Values unchanged: the circulars confirm them; the costs crate keeps refusing these dates.
    let exchange = match costs::regime::exchange_charge_rate(Exchange::Nse, td) {
        Ok(r) => i128::from(r.get()) * 100,
        Err(_) => {
            if d < day(2021, 1, 1) {
                5_000 * 100
            } else if d < day(2023, 4, 1) {
                5_300 * 100
            } else if d < day(2024, 4, 1) {
                5_000 * 100
            } else {
                4_950 * 100
            }
        }
    };
    let ipft = match costs::regime::ipft_rate(Exchange::Nse, td) {
        Ok(r) => i128::from(r.get()) * 100,
        Err(_) => 1,
    };
    let sebi = if d < day(2019, 4, 1) { 1_500 } else { i128::from(costs::rate::SEBI_TURNOVER_FEE.get()) * 100 };
    // v3.3 fix CHG33 (check-engine-1 M3r): SEBI/LAD-NRO/GN/2020/011 = NSE FA44514: "for the period June 01, 2020 to March 31, 2021, pay only 50%"
    let sebi = if crate::fix33::sebi_half(d) { sebi / 2 } else { sebi };
    // STT on the sold premium: 0.05% from 2016-06-01 to 2023-03-31, then the repository's rows (robot comment: check2 N5).
    let stt = if d < day(2023, 4, 1) {
        500_000
    } else {
        costs::regime::stt_options_rate(td).map(|r| i128::from(r.get()) * 100).map_err(|e| format!("STT refused on day {d}: {e:?}"))?
    };
    let stt_ex = costs::regime::stt_exercise_rate(td).map(|r| i128::from(r.get()) * 100).map_err(|e| format!("exercise STT refused on day {d}: {e:?}"))?;
    let stt_ex_on_notional = d < day(2019, 9, 1);
    let (stamp_buy, stamp_both) = if d >= day(2020, 7, 1) { (i128::from(costs::rate::STAMP_DUTY_BUY_SIDE.get()) * 100, 0) } else { (0, 2_000 * 100) };
    Ok(Rates { exchange, ipft, sebi, stt, stt_ex, stt_ex_on_notional, stamp_buy, stamp_both, gst: i128::from(costs::rate::GST_ON_FEE_BASE.get()), gst_on_sebi: crate::fixg::gst_on_sebi(d) })
}

/// Dhan's Order Estimator reading of one round trip: (buy side, sell side), each rounded half up to the paisa. COPIED (robot
/// options.rs `charges_est`, the display parts dropped).
pub(crate) fn charges_est(r: &Rates, t: &Trip) -> (i64, i64) {
    let settled = t.rem_kind == 2;
    let sold = t.sell_value - if settled { t.settle_value } else { 0 };
    let itm = settled && t.settle_value > 0;
    let v = |x: i64| i128::from(x.max(0));
    let brk_b = BROKERAGE_PER_ORDER * i128::from(t.entry_orders) * E9;
    let brk_s = BROKERAGE_PER_ORDER * (i128::from(t.exit_orders) + i128::from(t.broker_orders) + i128::from(settled)) * E9;
    let sq = i128::from(costs::schedule::DHAN_SQUARE_OFF_FEE.raw()) * i128::from(t.broker_orders) * E9;
    let exch_b = v(t.buy_value) * (r.exchange + r.ipft);
    let exch_s = v(sold) * (r.exchange + r.ipft);
    let sebi_b = v(t.buy_value) * r.sebi;
    let sebi_s = v(sold) * r.sebi + if itm { v(t.settle_notional) * r.sebi } else { 0 };
    let stamp_b = v(t.buy_value) * (r.stamp_buy + r.stamp_both);
    let stamp_s = v(sold) * r.stamp_both;
    let stt_ex = if itm { v(if r.stt_ex_on_notional { t.settle_notional } else { t.settle_value }) * r.stt_ex } else { 0 };
    let stt_s = v(sold) * r.stt + stt_ex;
    let half_up = |n: i128, d: i128| -> i128 { (2 * n + d).div_euclid(2 * d) };
    // fix:v33 GST: the SEBI fee in the GST base from 2022-07-18 (NSE FATAX52993); (0, 0) = v3.3
    let (gb, gs) = if r.gst_on_sebi { (sebi_b, sebi_s) } else { (0, 0) };
    let gst_b = half_up((brk_b + exch_b + gb) * r.gst, 10_000_000 * E9);
    let gst_s = half_up((brk_s + sq + exch_s + gs) * r.gst, 10_000_000 * E9);
    let buy = half_up(brk_b + exch_b + sebi_b + stamp_b + gst_b * E9, E9);
    let sell = if t.qty > 0 || sold > 0 || settled { half_up(brk_s + sq + exch_s + sebi_s + stamp_s + stt_s + gst_s * E9, E9) } else { 0 };
    let to64 = |x: i128| i64::try_from(x).unwrap_or(i64::MAX);
    (to64(buy), to64(sell))
}

/// One trip's charges at the dearer of the dated set `rd` and today's `rt` (item 26), the Estimator reading: (total, buy side, sell
/// side, whether today's set was the dearer). COPIED (robot options.rs `charged`, Estimator arm).
pub(crate) fn charged(rd: &Rates, rt: &Rates, t: &Trip) -> (i64, i64, i64, bool) {
    let (ab, as_) = charges_est(rd, t);
    let (bb, bs) = charges_est(rt, t);
    if bb + bs > ab + as_ { (bb + bs, bb, bs, true) } else { (ab + as_, ab, as_, false) }
}

/// SELL-DIAG (SNIPER s25 item 4: Dhan charges incl. the sell-side STT): Dhan's Order Estimator reading of one SOLD-FIRST round trip
/// (the sale opens the position, the buy-back closes it): the same rates and roundings as [`charges_est`], with the order counts on
/// their real sides: the SALE side carries the entry order (`entry_orders`) and the STT on the sold premium; the BUY-BACK side carries
/// the own exit order (`exit_orders`), the broker square-off (`broker_orders`, Rs 20 + the square-off fee, both with GST) and the stamp
/// duty on the buy. `buy_value` = the buy-back value (the units never bought back valued at the worst price, counted), `sell_value` =
/// the premium sold. Returns (buy side, sell side), each rounded half up to the paisa. A stock option is never held to expiry here, so no
/// settlement or exercise STT arises.
pub(crate) fn charges_est_sell(r: &Rates, t: &Trip) -> (i64, i64) {
    let v = |x: i64| i128::from(x.max(0));
    let brk_s = BROKERAGE_PER_ORDER * i128::from(t.entry_orders) * E9;
    let brk_b = BROKERAGE_PER_ORDER * (i128::from(t.exit_orders) + i128::from(t.broker_orders)) * E9;
    let sq = i128::from(costs::schedule::DHAN_SQUARE_OFF_FEE.raw()) * i128::from(t.broker_orders) * E9;
    let exch_b = v(t.buy_value) * (r.exchange + r.ipft);
    let exch_s = v(t.sell_value) * (r.exchange + r.ipft);
    let sebi_b = v(t.buy_value) * r.sebi;
    let sebi_s = v(t.sell_value) * r.sebi;
    let stamp_b = v(t.buy_value) * (r.stamp_buy + r.stamp_both);
    let stamp_s = v(t.sell_value) * r.stamp_both;
    let stt_s = v(t.sell_value) * r.stt;
    let half_up = |n: i128, d: i128| -> i128 { (2 * n + d).div_euclid(2 * d) };
    // fix:v33 GST: the SEBI fee in the GST base from 2022-07-18 (NSE FATAX52993); (0, 0) = v3.3
    let (gb, gs) = if r.gst_on_sebi { (sebi_b, sebi_s) } else { (0, 0) };
    let gst_b = half_up((brk_b + sq + exch_b + gb) * r.gst, 10_000_000 * E9);
    let gst_s = half_up((brk_s + exch_s + gs) * r.gst, 10_000_000 * E9);
    let buy = half_up(brk_b + sq + exch_b + sebi_b + stamp_b + gst_b * E9, E9);
    let sell = half_up(brk_s + exch_s + sebi_s + stamp_s + stt_s + gst_s * E9, E9);
    let to64 = |x: i128| i64::try_from(x).unwrap_or(i64::MAX);
    (to64(buy), to64(sell))
}

/// [`charges_est_sell`] at the dearer of the dated set `rd` and today's `rt` (item 26): (total, buy side, sell side, today dearer).
pub(crate) fn charged_sell(rd: &Rates, rt: &Rates, t: &Trip) -> (i64, i64, i64, bool) {
    let (ab, as_) = charges_est_sell(rd, t);
    let (bb, bs) = charges_est_sell(rt, t);
    if bb + bs > ab + as_ { (bb + bs, bb, bs, true) } else { (ab + as_, ab, as_, false) }
}
