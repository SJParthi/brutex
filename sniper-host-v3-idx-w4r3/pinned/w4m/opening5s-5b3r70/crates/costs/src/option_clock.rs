//! `costs::option_clock` (S3 of SNIPER-IDXOPT-PLAN-v3.1, plan s2 N2): the ONE port of the robot's dated option estimator
//! into `costs`, so the index tracks, `opt_pos` and the STKOPT path call one code path (B6).
//!
//! PORTED byte for byte (logic, constants and rounding; only the module paths changed from `costs::` to `crate::`) from
//! the index robot's `options.rs`: `scratch/opening5s-idx1s/crates/stk_opt_preview/src/options.rs` (`Rates` :1703,
//! `rates_on` :1725, `side` / `rupee_up`, `CH_NAMES`, `BROKERAGE_PER_ORDER`, `charges` :1799, `charges_est` :1577,
//! `charges_leg` :1616, `today` :1625, `charged` :1631, `breakeven`), whose `rates_on` and `charges` equal the assurance
//! snapshot the plan cites (`scratch/assurance-20261108/extremes/tool-snap-1526/src/index/options.rs` :1484 ff. and
//! :1555 ff., sha16 6ae699b9b79c9f35). The settle path (plan s2 N2, B6):
//! * a remainder settled at expiry (`rem_kind` [`REM_SETTLED`]) pays no sale charge on its settled part;
//! * it pays Rs 20 brokerage (Dhan charges expired and exercised contracts: sources-charges B-05, UNVERIFIED);
//! * when in the money: the exercise STT on the settlement NOTIONAL (index close x units) before 2019-09-01
//!   (NSE/FATAX/41919, sources-charges S-04) and on the intrinsic value from then, at the repository's dated rate
//!   (0.125%, 0.15% from 2026-04-01), plus the SEBI fee on the notional.
//!
//! Readings: [`Reading::Estimator`] is Dhan's Order Estimator exactly (REQ 41h, the headline); [`Reading::PerLeg`] the
//! stricter per-leg reading printed beside. [`charged`] takes the DEARER of the dated set and today's (2026-10-30) set
//! (item 26), a tie reading the dated.

use brutex_core::instrument::Exchange;

use crate::day::TradeDay;

/// Paise per crore = rupees per crore x 100; a fraction is rate / 10^9.
pub const E9: i128 = 1_000_000_000;

/// Dhan's brokerage on F&O: Rs 20 per executed order.
pub const BROKERAGE_PER_ORDER: i128 = 2_000;

/// Charge components, paisa.
pub const CH_NAMES: [&str; 8] = ["brokerage", "square-off fee", "NSE txn", "IPFT", "SEBI fee", "STT", "stamp", "GST"];

/// `rem_kind` of a remainder settled at expiry (intrinsic value against the official index close).
pub const REM_SETTLED: u8 = 2;

/// The estimator's inputs of one option round trip (the robot's `Trip`: the fields `charges` / `charges_est` /
/// `charges_leg` read).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Trip {
    /// Units bought.
    pub qty: i64,
    /// Premium paid, paisa.
    pub buy_value: i64,
    /// Premium received, paisa (a settled part's intrinsic value included).
    pub sell_value: i64,
    /// Entry orders that executed.
    pub entry_orders: u32,
    /// Strategy exit orders that executed.
    pub exit_orders: u32,
    /// The broker's square-off orders that executed.
    pub broker_orders: u32,
    /// Executed iceberg legs of the entry (per-leg reading).
    pub entry_legs: u32,
    /// Executed legs of the exits.
    pub exit_legs: u32,
    /// Executed legs of the broker's orders.
    pub broker_legs: u32,
    /// How an unsold remainder was valued: [`REM_SETTLED`] = expiry settlement.
    pub rem_kind: u8,
    /// The settled part's intrinsic value x units (inside `sell_value`), paisa.
    pub settle_value: i64,
    /// The index close x the settled units (the exercise STT base before 2019-09-01, the SEBI fee base), paisa.
    pub settle_notional: i64,
}

/// The dated rates of one day (paise per crore of premium), with the row of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rates {
    /// NSE transaction charge.
    pub exchange: i128,
    /// Its table row.
    pub ex_row: u8,
    /// IPFT.
    pub ipft: i128,
    /// Its row.
    pub ipft_row: u8,
    /// SEBI turnover fee.
    pub sebi: i128,
    /// Its row.
    pub sebi_row: u8,
    /// STT on the sold premium.
    pub stt: i128,
    /// STT on an exercised option (paise per crore).
    pub stt_ex: i128,
    /// Whether the exercise STT is levied on the settlement notional (before 2019-09-01) rather than the intrinsic value.
    pub stt_ex_on_notional: bool,
    /// Stamp duty on the buy (from 2020-07-01).
    pub stamp_buy: i128,
    /// Stamp duty on both sides (before 2020-07-01).
    pub stamp_both: i128,
    /// GST is also levied on the SEBI fee (from 2022-07-18; the per-leg reading).
    pub gst_on_sebi: bool,
    /// 18% as a fraction x 10^7.
    pub gst: i128,
}

/// The day number (days since 1970-01-01) of `y-m-d` (proleptic Gregorian).
#[must_use]
pub const fn day_number(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    #[allow(clippy::cast_possible_wrap, reason = "mp <= 11, d <= 31")]
    let doy = ((153 * mp + 2) / 5 + d - 1) as i32;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146_097 + doe - 719_468) as i64
}

/// `(y, m, d)` of a day number.
#[must_use]
pub const fn civil(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// The repository's trade day of a day number.
#[must_use]
pub fn trade_day(d: i64) -> Option<TradeDay> {
    let (y, m, dd) = civil(d);
    TradeDay::new(u16::try_from(y).ok()?, u8::try_from(m).ok()?, u8::try_from(dd).ok()?).ok()
}

/// `ceil(a / b)` for `b > 0`.
#[must_use]
pub const fn ceil_div(a: i128, b: i128) -> i128 {
    let q = a.div_euclid(b);
    if q * b == a { q } else { q + 1 }
}

/// The rates in force on day `d` (day number): the repository's rows (`regime`, `rate`) where they hold, the NSE
/// circular rows (as the options overlay cites them) where the repository refuses. PORTED.
///
/// # Errors
/// A day the repository refuses for STT or exercise STT (the caller refuses the trade by name).
pub fn rates_on(d: i64) -> Result<Rates, String> {
    let td = trade_day(d).ok_or_else(|| format!("trade day {d}"))?;
    let day = |y: i32, m: u32, dd: u32| day_number(y, m, dd);
    let (exchange, ex_row) = match crate::regime::exchange_charge_rate(Exchange::Nse, td) {
        Ok(r) => (i128::from(r.get()) * 100, if d >= day(2026, 3, 1) { 6 } else { 5 }),
        Err(_) => {
            if d < day(2021, 1, 1) {
                (5_000 * 100, 1)
            } else if d < day(2023, 4, 1) {
                (5_300 * 100, 2)
            } else if d < day(2024, 4, 1) {
                (5_000 * 100, 3)
            } else {
                (4_950 * 100, 4)
            }
        }
    };
    let (ipft, ipft_row) = match crate::regime::ipft_rate(Exchange::Nse, td) {
        Ok(r) => (i128::from(r.get()) * 100, 2),
        Err(_) => (1, 1),
    };
    let (sebi, sebi_row) = if d < day(2019, 4, 1) { (1_500, 1) } else { (i128::from(crate::rate::SEBI_TURNOVER_FEE.get()) * 100, 2) };
    // STT on the sold premium: 0.05% from 2016-06-01 to 2023-03-31 (NSE/FATAX/32385 and NSE/FATAX/56235, sources-charges
    // row S-03; the repository's anchored 0.0625% row does not encode it), then the repository's rows.
    let stt = if d < day(2023, 4, 1) {
        500_000
    } else {
        crate::regime::stt_options_rate(td).map(|r| i128::from(r.get()) * 100).map_err(|e| format!("STT refused on day {d}: {e:?}"))?
    };
    // STT on exercise: the repository's rows (0.125%, 0.15% from 2026-04-01) on the intrinsic value from 2019-09-01;
    // on the settlement notional before it (NSE/FATAX/41919, sources-charges S-04).
    let stt_ex = crate::regime::stt_exercise_rate(td).map(|r| i128::from(r.get()) * 100).map_err(|e| format!("exercise STT refused on day {d}: {e:?}"))?;
    let stt_ex_on_notional = d < day(2019, 9, 1);
    let (stamp_buy, stamp_both) = if d >= day(2020, 7, 1) { (i128::from(crate::rate::STAMP_DUTY_BUY_SIDE.get()) * 100, 0) } else { (0, 2_000 * 100) };
    Ok(Rates {
        exchange,
        ex_row,
        ipft,
        ipft_row,
        sebi,
        sebi_row,
        stt,
        stt_ex,
        stt_ex_on_notional,
        stamp_buy,
        stamp_both,
        gst_on_sebi: d >= day(2022, 7, 18),
        gst: i128::from(crate::rate::GST_ON_FEE_BASE.get()),
    })
}

fn side(value: i64, rate: i128) -> i128 {
    if value <= 0 || rate <= 0 {
        return 0;
    }
    ceil_div(i128::from(value) * rate, E9)
}

fn rupee_up(paisa: i128) -> i128 {
    ceil_div(paisa, 100) * 100
}

fn to64(x: i128) -> i64 {
    i64::try_from(x).unwrap_or(i64::MAX)
}

/// The charges of one round trip, every part rounded up on its own (the robot's official / per-leg arithmetic). A
/// remainder settled at expiry pays no sale charge on its settled part; it pays Rs 20 brokerage, and when in the money
/// the exercise STT (on the intrinsic value, on the notional before 2019-09-01) and the SEBI fee on the notional. PORTED.
#[must_use]
pub fn charges(r: &Rates, t: &Trip) -> [i64; 8] {
    let settled = t.rem_kind == REM_SETTLED && !cfg!(feature = "m_oc_settle");
    let orders = i128::from(t.entry_orders) + i128::from(t.exit_orders) + i128::from(t.broker_orders) + i128::from(settled);
    let brk = BROKERAGE_PER_ORDER * orders;
    let sq = i128::from(crate::schedule::DHAN_SQUARE_OFF_FEE.raw()) * i128::from(t.broker_orders);
    let sold = t.sell_value - if settled { t.settle_value } else { 0 };
    let itm = settled && t.settle_value > 0;
    let ex = side(t.buy_value, r.exchange) + side(sold, r.exchange);
    let ip = side(t.buy_value, r.ipft) + side(sold, r.ipft);
    let sebi = side(t.buy_value, r.sebi) + side(sold, r.sebi) + if itm { side(t.settle_notional, r.sebi) } else { 0 };
    let stt_ex = if itm { side(if r.stt_ex_on_notional { t.settle_notional } else { t.settle_value }, r.stt_ex) } else { 0 };
    let stt = rupee_up(side(sold, r.stt) + stt_ex);
    let stamp = rupee_up(side(t.buy_value, r.stamp_buy) + side(t.buy_value, r.stamp_both) + side(sold, r.stamp_both));
    let base = brk + sq + ex + ip + if r.gst_on_sebi { sebi } else { 0 };
    let gst = ceil_div(base * r.gst, 10_000_000);
    [to64(brk), to64(sq), to64(ex), to64(ip), to64(sebi), to64(stt), to64(stamp), to64(gst)]
}

/// Dhan's Order Estimator reading of one round trip (REQ 41h, the headline): per side Rs 20 for each order (one entry
/// order, each strategy exit order that filled, the broker's order, and a settlement), the exchange charges (NSE
/// transaction + IPFT) on the side's turnover, the SEBI fee, stamp duty on the buy (on both sides before 2020-07-01),
/// STT on the sell (and on exercise), the broker's square-off fee, and GST 18% of (brokerage + square-off fee + exchange
/// charges) rounded half up to the paisa (no GST on the SEBI fee); every other part unrounded; each side's total rounded
/// half up to the paisa. Returns (buy side, sell side, display parts in [`CH_NAMES`] order). PORTED.
#[must_use]
pub fn charges_est(r: &Rates, t: &Trip) -> (i64, i64, [i64; 8]) {
    let settled = t.rem_kind == REM_SETTLED && !cfg!(feature = "m_oc_settle");
    let sold = t.sell_value - if settled { t.settle_value } else { 0 };
    let itm = settled && t.settle_value > 0;
    let v = |x: i64| i128::from(x.max(0));
    // Numerators in paisa x 10^9.
    let brk_b = BROKERAGE_PER_ORDER * i128::from(t.entry_orders) * E9;
    let brk_s = BROKERAGE_PER_ORDER * (i128::from(t.exit_orders) + i128::from(t.broker_orders) + i128::from(settled)) * E9;
    let sq = i128::from(crate::schedule::DHAN_SQUARE_OFF_FEE.raw()) * i128::from(t.broker_orders) * E9;
    let exch_b = v(t.buy_value) * (r.exchange + r.ipft);
    let exch_s = v(sold) * (r.exchange + r.ipft);
    let sebi_b = v(t.buy_value) * r.sebi;
    let sebi_s = v(sold) * r.sebi + if itm { v(t.settle_notional) * r.sebi } else { 0 };
    let stamp_b = v(t.buy_value) * (r.stamp_buy + r.stamp_both);
    let stamp_s = v(sold) * r.stamp_both;
    let stt_ex = if itm { v(if r.stt_ex_on_notional { t.settle_notional } else { t.settle_value }) * r.stt_ex } else { 0 };
    let stt_s = v(sold) * r.stt + stt_ex;
    let half_up = |n: i128, d: i128| -> i128 { (2 * n + d).div_euclid(2 * d) };
    let gst_b = half_up((brk_b + exch_b) * r.gst, 10_000_000 * E9);
    let gst_s = half_up((brk_s + sq + exch_s) * r.gst, 10_000_000 * E9);
    let buy = half_up(brk_b + exch_b + sebi_b + stamp_b + gst_b * E9, E9);
    let sell = if t.qty > 0 || sold > 0 || settled { half_up(brk_s + sq + exch_s + sebi_s + stamp_s + stt_s + gst_s * E9, E9) } else { 0 };
    let parts = [
        to64(half_up(brk_b + brk_s, E9)),
        to64(half_up(sq, E9)),
        to64(half_up(exch_b + exch_s, E9)),
        0,
        to64(half_up(sebi_b + sebi_s, E9)),
        to64(half_up(stt_s, E9)),
        to64(half_up(stamp_b + stamp_s, E9)),
        to64(gst_b + gst_s),
    ];
    (to64(buy), to64(sell), parts)
}

/// The stricter per-leg reading (REQ 41h's line beside): Rs 20 for every executed iceberg leg, GST also on the SEBI fee
/// (from 2022-07-18), STT and stamp up to the rupee: [`charges`] with the legs as orders. PORTED.
#[must_use]
pub fn charges_leg(r: &Rates, t: &Trip) -> [i64; 8] {
    let mut l = *t;
    l.entry_orders = t.entry_legs;
    l.exit_orders = t.exit_legs;
    l.broker_orders = t.broker_legs;
    charges(r, &l)
}

/// "Today's" rate set for item 26 (the latest set held: 2026-10-30). PORTED.
#[must_use]
pub const fn today() -> i64 {
    day_number(2026, 10, 30)
}

/// The charges reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    /// Dhan's Order Estimator (REQ 41h; the headline).
    Estimator,
    /// The stricter per-leg reading (beside).
    PerLeg,
}

/// One trip's charges under `reading` at the dearer of the dated set `rd` and today's `rt` (item 26): (total, parts of
/// the dearer set, whether today's set was the dearer). PORTED.
#[must_use]
pub fn charged(rd: &Rates, rt: &Rates, t: &Trip, reading: Reading) -> (i64, [i64; 8], bool) {
    let one = |r: &Rates| -> (i64, [i64; 8]) {
        match reading {
            Reading::Estimator => {
                let (b, s, p) = charges_est(r, t);
                (b + s, p)
            }
            Reading::PerLeg => {
                let p = charges_leg(r, t);
                (p.iter().sum(), p)
            }
        }
    };
    let (a, pa) = one(rd);
    let (b, pb) = one(rt);
    if b > a { (b, pb, true) } else { (a, pa, false) }
}

/// The breakeven sell price (paisa, rounded half up) of a buy of `qty` at `price` paisa whose two sides cost `buy_side`
/// and `sell_side` paisa. PORTED.
#[must_use]
pub fn breakeven(price: i64, qty: i64, buy_side: i64, sell_side: i64) -> i64 {
    if qty <= 0 {
        return price;
    }
    let n = i128::from(price) * i128::from(qty) + i128::from(buy_side) + i128::from(sell_side);
    i64::try_from((2 * n + i128::from(qty)).div_euclid(2 * i128::from(qty))).unwrap_or(i64::MAX)
}

/// The expiry settlement of `units` of an option struck at `strike` (paisa) against the official index close `close`
/// (paisa; crate S15 bullet 3): (intrinsic value x units, close x units = the notional). The intrinsic value per unit is
/// crate v2.1's `common_fill::limits::settle_intrinsic` (a CALL max(0, close - strike), a PUT max(0, strike - close)):
/// WAVE 4 (check-build-idx-S1-S6 D1) calls the crate instead of re-implementing it (one common runtime). Exact integer
/// arithmetic, saturating at `i64::MAX` (a difference outside i64 saturates too).
#[must_use]
pub fn settle(close: i64, strike: i64, ce: bool, units: i64) -> (i64, i64) {
    let intrinsic = common_fill::limits::settle_intrinsic(ce, strike, close).unwrap_or(i64::MAX);
    let u = i128::from(units.max(0));
    (to64(i128::from(intrinsic) * u), to64(i128::from(close.max(0)) * u))
}

#[cfg(test)]
#[path = "option_clock_tests.rs"]
mod tests;
