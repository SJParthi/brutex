//! Tests of the `option_clock` port (S3). Written red first: the exercise-STT vectors were run against the port with
//! the settle path cut (feature `m_oc_settle`, a named mutant below) and failed there before they passed here.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic, reason = "tests")]

use super::*;

fn d(y: i32, m: u32, dd: u32) -> i64 {
    day_number(y, m, dd)
}

/// Half up of n / den, written apart from the port.
fn hu(n: i128, den: i128) -> i128 {
    (2 * n + den) / (2 * den)
}

/// The operator's Dhan Order Estimator screen (9 Nov 2026 11:29 IST; REQ 41h): NIFTY 24600 CE, limit buy 3,250 units at
/// Rs 310: buy Rs 477.23, sell Rs 1,958.25, breakeven Rs 310.75; per-leg Rs 501.80 / Rs 1,982.80. COPIED from the
/// robot's own test `the_dhan_order_estimator_screen_is_reproduced_exactly`.
#[test]
fn req41h_the_dhan_order_estimator_screen_is_reproduced_exactly() {
    let r = rates_on(today()).unwrap();
    let value = 3_250 * 31_000;
    let buy = Trip { qty: 3_250, buy_value: value, entry_orders: 1, entry_legs: 2, ..Trip::default() };
    let sell = Trip { qty: 3_250, sell_value: value, exit_orders: 1, exit_legs: 2, ..Trip::default() };
    let (b, _, pb) = charges_est(&r, &buy);
    let (_, s, ps) = charges_est(&r, &Trip { qty: 0, ..sell });
    assert_eq!(b, 47_723, "buy side Rs 477.23");
    assert_eq!(s, 195_825, "sell side Rs 1,958.25");
    assert_eq!((pb[0], pb[2], pb[4], pb[6], pb[7]), (2_000, 35_796, 101, 3_023, 6_803));
    assert_eq!((ps[2], ps[5], ps[7]), (35_796, 151_125, 6_803));
    assert_eq!(breakeven(31_000, 3_250, b, s), 31_075, "breakeven Rs 310.75");
    let lb: i64 = charges_leg(&r, &buy).iter().sum();
    let ls: i64 = charges_leg(&r, &sell).iter().sum();
    assert_eq!((lb, ls), (50_180, 198_280), "per-leg Rs 501.80 and Rs 1,982.80");
    let trip = Trip { qty: 3_250, buy_value: value, sell_value: value, entry_orders: 1, exit_orders: 1, entry_legs: 2, exit_legs: 2, ..Trip::default() };
    let (tb, ts, _) = charges_est(&r, &trip);
    assert_eq!(tb + ts, 243_548);
}

/// The dearer of the dated and today's sets, both ways round. COPIED from the robot's test.
#[test]
fn the_dearer_rate_set_is_taken_both_ways_round() {
    let (r19, rt) = (rates_on(d(2019, 1, 15)).unwrap(), rates_on(today()).unwrap());
    let lossy = Trip { qty: 7_500, buy_value: 100_000_000, sell_value: 75_000, entry_orders: 1, exit_orders: 1, entry_legs: 5, exit_legs: 5, ..Trip::default() };
    let (tot, _, today_dearer) = charged(&r19, &rt, &lossy, Reading::Estimator);
    let (a, b, _) = charges_est(&r19, &lossy);
    assert!(!today_dearer);
    assert_eq!(tot, a + b);
    let win = Trip { qty: 7_500, buy_value: 75_000, sell_value: 100_000_000, entry_orders: 1, exit_orders: 1, entry_legs: 5, exit_legs: 5, ..Trip::default() };
    let (tot2, _, today2) = charged(&r19, &rt, &win, Reading::Estimator);
    let (c, e, _) = charges_est(&rt, &win);
    assert!(today2);
    assert_eq!(tot2, c + e);
    let (pl, _, _) = charged(&r19, &rt, &win, Reading::PerLeg);
    assert!(pl >= tot2);
}

/// The dated exercise rows: 0.125% before 2026-04-01, 0.15% from it; on the notional before 2019-09-01.
#[test]
fn exercise_stt_rows_are_dated() {
    let a = rates_on(d(2019, 8, 29)).unwrap();
    let b = rates_on(d(2019, 9, 26)).unwrap();
    let c = rates_on(d(2026, 4, 7)).unwrap();
    assert_eq!((a.stt_ex, a.stt_ex_on_notional), (1_250_000, true));
    assert_eq!((b.stt_ex, b.stt_ex_on_notional), (1_250_000, false));
    assert_eq!((c.stt_ex, c.stt_ex_on_notional), (1_500_000, false));
    assert_eq!(rates_on(d(2026, 3, 31)).unwrap().stt_ex, 1_250_000);
}

/// A fully settled trip: bought `units` at `px`, nothing sold, the whole quantity settled at `close` vs `strike`.
fn settled_trip(units: i64, px: i64, close: i64, strike: i64, ce: bool) -> Trip {
    let (value, notional) = settle(close, strike, ce, units);
    Trip { qty: units, buy_value: units * px, sell_value: value, entry_orders: 1, entry_legs: 1, rem_kind: REM_SETTLED, settle_value: value, settle_notional: notional, ..Trip::default() }
}

/// (i) An ITM unit settled 2019-08-29 (notional basis): NIFTY close 10,948.30, 10900 CE, 75 units. The exercise STT is
/// 0.125% of the NOTIONAL (82,112,250 paisa): 102,640.3125 paisa, the estimator's STT part 102,640, the official
/// arithmetic's STT up to the rupee 102,700. (The STT trap: Rs 1,026.40 on Rs 3,622.50 of intrinsic value.)
#[test]
fn red_i_itm_settled_2019_08_29_pays_stt_on_the_notional() {
    let r = rates_on(d(2019, 8, 29)).unwrap();
    let t = settled_trip(75, 3_000, 1_094_830, 1_090_000, true);
    assert_eq!((t.settle_value, t.settle_notional), (362_250, 82_112_250));
    let (_, sell, parts) = charges_est(&r, &t);
    assert_eq!(i128::from(parts[5]), hu(82_112_250 * 1_250_000, E9), "estimator STT part on the notional");
    assert_eq!(parts[5], 102_640);
    let off = charges(&r, &t);
    assert_eq!(off[5], 102_700, "official arithmetic: ceil to the paisa, then up to the rupee");
    // the sell side, computed apart: Rs 20 (settlement order) + SEBI on the notional + STT + GST on the Rs 20.
    let want = hu(2_000 * E9 + 82_112_250 * 1_000 + 82_112_250 * 1_250_000 + hu(2_000 * E9 * 1_800_000, 10_000_000 * E9) * E9, E9);
    assert_eq!(i128::from(sell), want);
}

/// (ii) An ITM unit settled 2019-09-26 (intrinsic basis): NIFTY close 11,571.20, 11500 CE, 75 units: 0.125% of the
/// intrinsic value 534,000 paisa = 667.5, the estimator's part 668 (half up).
#[test]
fn red_ii_itm_settled_2019_09_26_pays_stt_on_the_intrinsic_value() {
    let r = rates_on(d(2019, 9, 26)).unwrap();
    let t = settled_trip(75, 6_000, 1_157_120, 1_150_000, true);
    assert_eq!(t.settle_value, 534_000);
    let (_, _, parts) = charges_est(&r, &t);
    assert_eq!(parts[5], 668);
    assert_eq!(charges(&r, &t)[5], 700);
}

/// (iii) A settlement after 2026-04-01 at 0.15%: NIFTY 2026-04-07, close 22,500.00, 22400 PE (OTM for a PUT, so take
/// 22600 PE, ITM by 100.00), 65 units: intrinsic 650,000 paisa, STT 0.15% = 975 paisa exactly.
#[test]
fn red_iii_settlement_after_2026_04_01_is_charged_at_0_15_pct() {
    let r = rates_on(d(2026, 4, 7)).unwrap();
    let t = settled_trip(65, 10_000, 2_250_000, 2_260_000, false);
    assert_eq!(t.settle_value, 650_000);
    let (_, _, parts) = charges_est(&r, &t);
    assert_eq!(parts[5], 975);
    // the sale STT on 650,000 at 0.15% is also 975, so the vector pins the settle path by the SEBI fee on the notional
    // (146,250,000 paisa) and the settlement's Rs 20, which a sale would not pay (found in the red round: the mutant
    // m_oc_settle passed the STT line alone).
    assert_eq!(i128::from(parts[4]), hu(650_000 * 1_000 + 146_250_000 * 1_000, E9));
    assert_eq!(parts[0], 4_000);
}

/// (iv) An OTM unit settles at 0: no exercise STT and no SEBI fee on the notional; only the buy side's SEBI fee and the
/// settlement's Rs 20 (with its GST) remain on the sell side.
#[test]
fn red_iv_otm_settled_unit_pays_no_exercise_stt() {
    let r = rates_on(d(2019, 8, 29)).unwrap();
    let t = settled_trip(75, 3_000, 1_094_830, 1_100_000, true);
    assert_eq!(t.settle_value, 0);
    let (_, sell, parts) = charges_est(&r, &t);
    assert_eq!(parts[5], 0, "no STT");
    let sebi_buy = hu(225_000 * 1_000, E9);
    assert_eq!(i128::from(parts[4]), sebi_buy, "SEBI fee on the buy only");
    assert_eq!(i128::from(sell), hu(2_000 * E9 + hu(2_000 * E9 * 1_800_000, 10_000_000 * E9) * E9, E9), "Rs 20 + GST Rs 3.60");
    assert_eq!(sell, 2_360);
}

/// (v) Rs 20 brokerage on an exercised (settled) contract, UNVERIFIED (sources-charges B-05): the brokerage part is one
/// order more than the same trip unsettled.
#[test]
fn red_v_settlement_pays_rs_20_brokerage_unverified() {
    let r = rates_on(d(2019, 9, 26)).unwrap();
    let t = settled_trip(75, 6_000, 1_157_120, 1_150_000, true);
    let (_, _, p) = charges_est(&r, &t);
    let (_, _, q) = charges_est(&r, &Trip { rem_kind: 0, settle_value: 0, settle_notional: 0, ..t });
    assert_eq!(p[0] - q[0], 2_000);
    assert_eq!(charges(&r, &t)[0] - charges(&r, &Trip { rem_kind: 0, ..t })[0], 2_000);
}

/// The SEBI fee on the notional of a settled ITM unit (2019-09-26: Rs 10 per crore of 86,784,000 paisa = 86.784).
#[test]
fn red_sebi_fee_on_the_notional_of_a_settled_itm_unit() {
    let r = rates_on(d(2019, 9, 26)).unwrap();
    let t = settled_trip(75, 6_000, 1_157_120, 1_150_000, true);
    assert_eq!(t.settle_notional, 86_784_000);
    let (_, _, p) = charges_est(&r, &t);
    assert_eq!(i128::from(p[4]), hu(450_000 * 1_000 + 86_784_000 * 1_000, E9));
}

/// `settle` itself: CALL / PUT intrinsic, at the money, units 0.
#[test]
fn settle_values() {
    assert_eq!(settle(2_250_000, 2_200_000, true, 65), (3_250_000, 146_250_000));
    assert_eq!(settle(2_250_000, 2_200_000, false, 65), (0, 146_250_000));
    assert_eq!(settle(2_250_000, 2_250_000, true, 65), (0, 146_250_000));
    assert_eq!(settle(2_250_000, 2_300_000, false, 0), (0, 0));
}

/// The port equals opt-beside's copy (`scratch/sniper-opt-beside/crates/opt_beside/src/charges.rs`, another lane's copy of
/// the robot's estimator, reproduced verbatim below as `optb`) on every calendar day 2018-09-01..2026-11-14 for a grid of
/// trips (with and without a settled remainder), both rate sets, the dearer total.
#[test]
fn equals_opt_beside_copy_on_every_day() {
    let mut n = 0u64;
    let rt = rates_on(today()).unwrap();
    let ot = optb::rates_on(optb::today()).unwrap();
    for day in d(2018, 9, 1)..=d(2026, 11, 14) {
        let (Ok(r), Ok(o)) = (rates_on(day), optb::rates_on(day)) else { panic!("rates refused on {day}") };
        for (k, &(q, b, s, eo, xo, bo, rem, sv, sn)) in GRID.iter().enumerate() {
            let t = Trip { qty: q, buy_value: b, sell_value: s, entry_orders: eo, exit_orders: xo, broker_orders: bo, rem_kind: rem, settle_value: sv, settle_notional: sn, ..Trip::default() };
            let ob = optb::Trip { qty: q, buy_value: b, sell_value: s, entry_orders: eo, exit_orders: xo, broker_orders: bo, rem_kind: rem, settle_value: sv, settle_notional: sn };
            let (b1, s1, _) = charges_est(&r, &t);
            assert_eq!((b1, s1), optb::charges_est(&o, &ob), "day {day} trip {k}");
            let (tot, _, td) = charged(&r, &rt, &t, Reading::Estimator);
            let (tot2, _, _, td2) = optb::charged(&o, &ot, &ob);
            assert_eq!((tot, td), (tot2, td2), "day {day} trip {k}");
            n += 1;
        }
    }
    assert!(n > 30_000);
}

/// The port equals the assurance snapshot's 8-part `charges` (`tool-snap-1526/src/index/options.rs` :1555 ff., the plan's
/// citation; reproduced verbatim below as `snap`) on every day for the same grid, legs as orders too.
#[test]
fn equals_tool_snap_charges_on_every_day() {
    for day in d(2018, 9, 1)..=d(2026, 11, 14) {
        let r = rates_on(day).unwrap();
        let s = snap::rates_on(day).unwrap();
        assert_eq!((r.stt_ex, r.stt_ex_on_notional, r.exchange, r.sebi, r.stt, r.stamp_buy, r.stamp_both, r.gst_on_sebi), (s.stt_ex, s.stt_ex_on_notional, s.exchange, s.sebi, s.stt, s.stamp_buy, s.stamp_both, s.gst_on_sebi), "day {day}");
        for &(q, b, sv0, eo, xo, bo, rem, sv, sn) in GRID {
            let t = Trip { qty: q, buy_value: b, sell_value: sv0, entry_orders: eo, exit_orders: xo, broker_orders: bo, entry_legs: eo + 2, exit_legs: xo + 3, broker_legs: bo, rem_kind: rem, settle_value: sv, settle_notional: sn };
            let st = snap::Trip { buy_value: b, sell_value: sv0, entry_orders: eo, exit_orders: xo, broker_orders: bo, rem_kind: rem, settle_value: sv, settle_notional: sn };
            assert_eq!(charges(&r, &t), snap::charges(&s, &st), "day {day}");
        }
    }
}

/// (qty, buy, sell, entry orders, exit orders, broker orders, rem_kind, settle value, settle notional)
/// One grid trip.
type GridTrip = (i64, i64, i64, u32, u32, u32, u8, i64, i64);
const GRID: &[GridTrip] = &[
    (3_250, 100_750_000, 100_750_000, 1, 1, 0, 0, 0, 0),
    (75, 225_000, 180_000, 1, 1, 0, 0, 0, 0),
    (7_500, 50_000_000, 0, 1, 0, 0, 0, 0, 0),
    (7_500, 75_000, 100_000_000, 1, 2, 1, 0, 0, 0),
    (1_800, 49_995_000, 51_234_567, 1, 3, 0, 0, 0, 0),
    (75, 225_000, 362_250, 1, 0, 0, 2, 362_250, 82_112_250),
    (75, 225_000, 0, 1, 0, 0, 2, 0, 82_112_250),
    (900, 48_000_000, 30_000_000 + 4_500_000, 1, 1, 0, 2, 4_500_000, 4_320_000_000),
    (65, 650_000, 1_300_000, 1, 0, 1, 2, 650_000, 146_250_000),
    (1, 1, 1, 1, 1, 0, 0, 0, 0),
    (0, 0, 0, 0, 0, 0, 0, 0, 0),
];

/// opt-beside's copy, verbatim from `scratch/sniper-opt-beside/crates/opt_beside/src/charges.rs` (sha16 at copy in
/// CHANGES-idx), with its `crate::cal` helpers replaced by the port's `day_number` / `civil` and `costs::` by `crate::`.
mod optb {
    use brutex_core::instrument::Exchange;
    use crate::day::TradeDay;
    const E9: i128 = 1_000_000_000;
    pub(crate) const BROKERAGE_PER_ORDER: i128 = 2_000;
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
    }
    pub(crate) fn trade_day(d: i64) -> Option<TradeDay> {
        let (y, m, dd) = super::civil(d);
        TradeDay::new(u16::try_from(y).ok()?, u8::try_from(m).ok()?, u8::try_from(dd).ok()?).ok()
    }
    pub(crate) const fn today() -> i64 {
        super::day_number(2026, 10, 30)
    }
    pub(crate) fn rates_on(d: i64) -> Result<Rates, String> {
        let td = trade_day(d).ok_or_else(|| format!("trade day {d}"))?;
        let day = |y: i32, m: u32, dd: u32| super::day_number(y, m, dd);
        let exchange = match crate::regime::exchange_charge_rate(Exchange::Nse, td) {
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
        let ipft = match crate::regime::ipft_rate(Exchange::Nse, td) {
            Ok(r) => i128::from(r.get()) * 100,
            Err(_) => 1,
        };
        let sebi = if d < day(2019, 4, 1) { 1_500 } else { i128::from(crate::rate::SEBI_TURNOVER_FEE.get()) * 100 };
        let stt = if d < day(2023, 4, 1) {
            500_000
        } else {
            crate::regime::stt_options_rate(td).map(|r| i128::from(r.get()) * 100).map_err(|e| format!("STT refused on day {d}: {e:?}"))?
        };
        let stt_ex = crate::regime::stt_exercise_rate(td).map(|r| i128::from(r.get()) * 100).map_err(|e| format!("exercise STT refused on day {d}: {e:?}"))?;
        let stt_ex_on_notional = d < day(2019, 9, 1);
        let (stamp_buy, stamp_both) = if d >= day(2020, 7, 1) { (i128::from(crate::rate::STAMP_DUTY_BUY_SIDE.get()) * 100, 0) } else { (0, 2_000 * 100) };
        Ok(Rates { exchange, ipft, sebi, stt, stt_ex, stt_ex_on_notional, stamp_buy, stamp_both, gst: i128::from(crate::rate::GST_ON_FEE_BASE.get()) })
    }
    pub(crate) fn charges_est(r: &Rates, t: &Trip) -> (i64, i64) {
        let settled = t.rem_kind == 2;
        let sold = t.sell_value - if settled { t.settle_value } else { 0 };
        let itm = settled && t.settle_value > 0;
        let v = |x: i64| i128::from(x.max(0));
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
        let to64 = |x: i128| i64::try_from(x).unwrap_or(i64::MAX);
        (to64(buy), to64(sell))
    }
    pub(crate) fn charged(rd: &Rates, rt: &Rates, t: &Trip) -> (i64, i64, i64, bool) {
        let (ab, as_) = charges_est(rd, t);
        let (bb, bs) = charges_est(rt, t);
        if bb + bs > ab + as_ { (bb + bs, bb, bs, true) } else { (ab + as_, ab, as_, false) }
    }
}

/// The assurance snapshot's `rates_on` and 8-part `charges`, verbatim from
/// `scratch/assurance-20261108/extremes/tool-snap-1526/src/index/options.rs` :1484-1578 (sha16 6ae699b9b79c9f35), with
/// `costs::` read as `crate::` and its day helpers as the port's.
mod snap {
    use brutex_core::instrument::Exchange;
    const E9: i128 = 1_000_000_000;
    const BROKERAGE_PER_ORDER: i128 = 2_000;
    #[derive(Clone, Copy, Debug, Default)]
    pub(crate) struct Trip {
        pub(crate) buy_value: i64,
        pub(crate) sell_value: i64,
        pub(crate) entry_orders: u32,
        pub(crate) exit_orders: u32,
        pub(crate) broker_orders: u32,
        pub(crate) rem_kind: u8,
        pub(crate) settle_value: i64,
        pub(crate) settle_notional: i64,
    }
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
        pub(crate) gst_on_sebi: bool,
        pub(crate) gst: i128,
    }
    pub(crate) fn rates_on(d: i64) -> Result<Rates, String> {
        let td = super::trade_day(d).ok_or_else(|| format!("trade day {d}"))?;
        let day = |y: i32, m: u32, dd: u32| super::day_number(y, m, dd);
        let exchange = match crate::regime::exchange_charge_rate(Exchange::Nse, td) {
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
        let ipft = match crate::regime::ipft_rate(Exchange::Nse, td) {
            Ok(r) => i128::from(r.get()) * 100,
            Err(_) => 1,
        };
        let sebi = if d < day(2019, 4, 1) { 1_500 } else { i128::from(crate::rate::SEBI_TURNOVER_FEE.get()) * 100 };
        let stt = if d < day(2023, 4, 1) {
            500_000
        } else {
            crate::regime::stt_options_rate(td).map(|r| i128::from(r.get()) * 100).map_err(|e| format!("STT refused on day {d}: {e:?}"))?
        };
        let stt_ex = crate::regime::stt_exercise_rate(td).map(|r| i128::from(r.get()) * 100).map_err(|e| format!("exercise STT refused on day {d}: {e:?}"))?;
        let stt_ex_on_notional = d < day(2019, 9, 1);
        let (stamp_buy, stamp_both) = if d >= day(2020, 7, 1) { (i128::from(crate::rate::STAMP_DUTY_BUY_SIDE.get()) * 100, 0) } else { (0, 2_000 * 100) };
        Ok(Rates { exchange, ipft, sebi, stt, stt_ex, stt_ex_on_notional, stamp_buy, stamp_both, gst_on_sebi: d >= day(2022, 7, 18), gst: i128::from(crate::rate::GST_ON_FEE_BASE.get()) })
    }
    fn side(value: i64, rate: i128) -> i128 {
        if value <= 0 || rate <= 0 {
            return 0;
        }
        super::ceil_div(i128::from(value) * rate, E9)
    }
    fn rupee_up(paisa: i128) -> i128 {
        super::ceil_div(paisa, 100) * 100
    }
    pub(crate) fn charges(r: &Rates, t: &Trip) -> [i64; 8] {
        let settled = t.rem_kind == 2;
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
        let gst = super::ceil_div(base * r.gst, 10_000_000);
        let to64 = |x: i128| i64::try_from(x).unwrap_or(i64::MAX);
        [to64(brk), to64(sq), to64(ex), to64(ip), to64(sebi), to64(stt), to64(stamp), to64(gst)]
    }
}
