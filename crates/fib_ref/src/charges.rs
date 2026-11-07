//! Charges, from COSTS.md and its Sat 7 Nov addendum, in exact paise.
//!
//! * **The official reading P** (COSTS.md section 5), the headline since
//!   24.V: Rs 20 or 0.03% per executed order, the NSE charge with IPFT on the
//!   turnover as one line, the SEBI fee, STT on NSE's VWAP basis rounded to
//!   the rupee, stamp duty (state law before 2020-07-01, the unified 0.003%
//!   of the buy after it, rounded to the rupee), GST 18% (on the SEBI fee
//!   only from 2022-07-18 on the dated clock), the broker's square-off fee
//!   Rs 20 + GST per order on top of brokerage, no DP charge on a squared
//!   day. Priced on two clocks, the trade's own date's rates and Dhan's of
//!   today (30 Oct 2026); the headline is the dearer.
//! * **The worst reading W** (COSTS.md section 6): every open choice at its
//!   worse value, a labelled stress line that decides nothing.
//! * **Options** (REQ 25a): today's rates only, exchange 0.03553%, STT 0.15%
//!   of the sell, stamp 0.003% of the buy (the rate the HDFCBANK screen's
//!   own figures imply), on the official method's steps.
//! * **The Dhan estimator replica**: Dhan's screen per side (GST without the
//!   SEBI fee, every line to the paisa), which reproduces both of the
//!   addendum's screens, Rs 578.85 and Rs 794.67, to the paisa.

use crate::clock::Date;
use crate::model::{OptionBrokerage, Readings, StateStamp};
use crate::money::{half_up, to_rupee};

/// Buy or sell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderSide {
    /// A buy order.
    Buy,
    /// A sell order.
    Sell,
}

/// One executed order: its fills and whether the broker placed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Order {
    /// Side.
    pub side: OrderSide,
    /// Fills `(quantity, price in paise)`, one per fill second.
    pub fills: Vec<(i64, i64)>,
    /// Placed by the broker's 15:10 square-off.
    pub square_off: bool,
}

impl Order {
    /// The order's quantity.
    pub fn qty(&self) -> i128 {
        self.fills.iter().map(|&(q, _)| i128::from(q)).sum()
    }

    /// The order's value, paise.
    pub fn value(&self) -> i128 {
        self.fills
            .iter()
            .map(|&(q, p)| i128::from(q) * i128::from(p))
            .sum()
    }
}

/// One clock's rate set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rates {
    /// NSE transaction charge, paise per crore of turnover (top slab).
    pub nse: i128,
    /// IPFT contribution, paise per crore.
    pub ipft: i128,
    /// SEBI turnover fee, paise per crore.
    pub sebi: i128,
    /// GST is charged on the SEBI fee.
    pub gst_on_sebi: bool,
    /// Stamp duty is the state's (before 2020-07-01).
    pub state_stamp: bool,
}

/// The dated rows of COSTS.md section 3 in force on `d` (the top slab
/// before 2024-10-01; rows before the window carry the window's first rows).
pub fn dated(d: Date) -> Rates {
    let on = |y: i32, m: u32, day: u32| d >= Date { y, m, d: day };
    let nse = if on(2026, 3, 1) {
        30_699
    } else if on(2024, 10, 1) {
        29_700
    } else if on(2024, 4, 1) {
        32_200
    } else if on(2023, 4, 1) {
        32_500
    } else if on(2021, 1, 1) {
        34_500
    } else {
        32_500
    };
    let ipft = if on(2026, 3, 1) || !on(2023, 4, 1) {
        1
    } else {
        1_000
    };
    let sebi = if on(2021, 4, 1) {
        1_000
    } else if on(2020, 6, 1) {
        500
    } else if on(2019, 4, 1) {
        1_000
    } else {
        1_500
    };
    Rates {
        nse,
        ipft,
        sebi,
        gst_on_sebi: on(2022, 7, 18),
        state_stamp: !on(2020, 7, 1),
    }
}

/// Dhan's rates of 30 Oct 2026 (COSTS.md section 4).
pub const TODAY: Rates = Rates {
    nse: 30_699,
    ipft: 1,
    sebi: 1_000,
    gst_on_sebi: true,
    state_stamp: false,
};

/// One clock's components, paise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Breakdown {
    /// Brokerage.
    pub brokerage: i128,
    /// NSE transaction charge with IPFT.
    pub exchange: i128,
    /// SEBI turnover fee.
    pub sebi: i128,
    /// Securities transaction tax.
    pub stt: i128,
    /// Stamp duty.
    pub stamp: i128,
    /// GST on brokerage, exchange (and SEBI where it applies).
    pub gst: i128,
    /// DP charge (the worst reading only).
    pub dp: i128,
    /// GST on the DP charge.
    pub dp_gst: i128,
    /// The broker's square-off fee.
    pub sq_fee: i128,
    /// GST on the square-off fee.
    pub sq_gst: i128,
}

impl Breakdown {
    /// The sum of every component.
    pub fn total(&self) -> i128 {
        self.brokerage
            + self.exchange
            + self.sebi
            + self.stt
            + self.stamp
            + self.gst
            + self.dp
            + self.dp_gst
            + self.sq_fee
            + self.sq_gst
    }
}

/// A trade priced on both clocks under both readings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Priced {
    /// Official reading, dated clock.
    pub dated_p: Breakdown,
    /// Official reading, today's clock.
    pub today_p: Breakdown,
    /// Worst reading, dated clock.
    pub dated_w: Breakdown,
    /// Worst reading, today's clock.
    pub today_w: Breakdown,
}

impl Priced {
    /// The headline charge: the dearer clock on the official reading (24.V).
    pub fn headline(&self) -> i128 {
        self.dated_p.total().max(self.today_p.total())
    }

    /// The labelled stress line: the dearer clock on the worst reading.
    pub fn stress(&self) -> i128 {
        self.dated_w.total().max(self.today_w.total())
    }
}

/// Day totals of a set of orders.
#[derive(Clone, Copy, Debug)]
struct Sums {
    bq: i128,
    bv: i128,
    sq: i128,
    sv: i128,
    n_sq: i128,
}

fn sums(orders: &[Order]) -> Sums {
    let mut s = Sums {
        bq: 0,
        bv: 0,
        sq: 0,
        sv: 0,
        n_sq: 0,
    };
    for o in orders {
        match o.side {
            OrderSide::Buy => {
                s.bq += o.qty();
                s.bv += o.value();
            }
            OrderSide::Sell => {
                s.sq += o.qty();
                s.sv += o.value();
            }
        }
        if o.square_off && o.qty() > 0 {
            s.n_sq += 1;
        }
    }
    s
}

/// `min(Rs 20, 0.03% of value)`, half-up to the paisa.
pub fn brokerage_of(value: i128) -> i128 {
    half_up(value * 3, 10_000).min(2_000)
}

/// Turnover times a per-crore rate, half-up to the paisa.
fn per_crore(value: i128, rate: i128) -> i128 {
    half_up(value * rate, 1_000_000_000)
}

/// Pre-2020-07-01 stamp duty on a day's turnover (paise), to the paisa.
pub fn state_stamp(turnover: i128, reading: StateStamp) -> i128 {
    let telangana = half_up(turnover, 10_000).min(10_000);
    let tamil_nadu = half_up(turnover * 6, 100_000);
    let stated = telangana.max(tamil_nadu);
    match reading {
        StateStamp::Stated => stated,
        // Himachal Pradesh's Rs 50 a day beats every other listed state
        // below Rs 5 lakh of turnover (COSTS.md section 3's table).
        StateStamp::Literal => stated.max(5_000),
    }
}

/// STT on NSE's VWAP basis, to the paisa (before the rupee rounding).
fn stt_vwap(s: &Sums) -> i128 {
    let q = s.bq + s.sq;
    if q == 0 {
        return 0;
    }
    let vwap = half_up(s.bv + s.sv, q);
    half_up(s.bq.min(s.sq) * vwap * 25, 100_000)
}

/// The official reading P on one clock.
pub fn official(date: Date, orders: &[Order], rates: &Rates, r: &Readings) -> Breakdown {
    let s = sums(orders);
    let turnover = s.bv + s.sv;
    let brokerage: i128 = orders
        .iter()
        .filter(|o| o.qty() > 0)
        .map(|o| brokerage_of(o.value()))
        .sum();
    let exchange = per_crore(turnover, rates.nse + rates.ipft);
    let sebi = per_crore(turnover, rates.sebi);
    let stt = to_rupee(stt_vwap(&s));
    let stamp = if rates.state_stamp {
        state_stamp(turnover, r.state_stamp)
    } else {
        to_rupee(half_up(s.bv * 3, 100_000))
    };
    let base = brokerage + exchange + if rates.gst_on_sebi { sebi } else { 0 };
    let _ = date;
    Breakdown {
        brokerage,
        exchange,
        sebi,
        stt,
        stamp,
        gst: half_up(base * 18, 100),
        dp: 0,
        dp_gst: 0,
        sq_fee: 2_000 * s.n_sq,
        sq_gst: 360 * s.n_sq,
    }
}

/// The worst reading W on one clock (COSTS.md section 6).
pub fn worst(orders: &[Order], rates: &Rates) -> Breakdown {
    let s = sums(orders);
    let turnover = s.bv + s.sv;
    let brokerage: i128 = orders
        .iter()
        .flat_map(|o| o.fills.iter())
        .filter(|&&(q, _)| q > 0)
        .map(|&(q, p)| brokerage_of(i128::from(q) * i128::from(p)))
        .sum();
    let rate = rates.nse + rates.ipft;
    let once = per_crore(turnover, rate);
    let split = per_crore(turnover, rates.nse) + per_crore(turnover, rates.ipft);
    let per_order: i128 = orders.iter().map(|o| per_crore(o.value(), rate)).sum();
    let exchange = once.max(split).max(per_order);
    let sebi_once = per_crore(turnover, rates.sebi);
    let sebi_orders: i128 = orders
        .iter()
        .map(|o| per_crore(o.value(), rates.sebi))
        .sum();
    let sebi = sebi_once.max(sebi_orders);
    let stt_sell = to_rupee(half_up(s.sv * 25, 100_000));
    let stt_orders: i128 = orders
        .iter()
        .filter(|o| o.side == OrderSide::Sell)
        .map(|o| to_rupee(half_up(o.value() * 25, 100_000)))
        .sum();
    let stt = to_rupee(stt_vwap(&s)).max(stt_sell).max(stt_orders);
    let stamp = if rates.state_stamp {
        let v = state_stamp(turnover, StateStamp::Literal);
        v.max(to_rupee(v))
    } else {
        let day = to_rupee(half_up(s.bv * 3, 100_000));
        let per: i128 = orders
            .iter()
            .filter(|o| o.side == OrderSide::Buy)
            .map(|o| to_rupee(half_up(o.value() * 3, 100_000)))
            .sum();
        day.max(per)
    };
    let base = brokerage + exchange + if rates.gst_on_sebi { sebi } else { 0 };
    let gst = half_up(base * 18, 100).max(2 * half_up(base * 9, 100));
    let has_sell = s.sq > 0;
    Breakdown {
        brokerage,
        exchange,
        sebi,
        stt,
        stamp,
        gst,
        dp: if has_sell { 1_250 } else { 0 },
        dp_gst: if has_sell { 225 } else { 0 },
        sq_fee: 2_000 * s.n_sq,
        sq_gst: 360 * s.n_sq,
    }
}

/// An equity trade on both clocks under both readings.
pub fn price_equity(date: Date, orders: &[Order], r: &Readings) -> Priced {
    let d = dated(date);
    Priced {
        dated_p: official(date, orders, &d, r),
        today_p: official(date, orders, &TODAY, r),
        dated_w: worst(orders, &d),
        today_w: worst(orders, &TODAY),
    }
}

/// An option trade's official reading on today's rates.
pub fn price_option(orders: &[Order], r: &Readings) -> Breakdown {
    let s = sums(orders);
    let turnover = s.bv + s.sv;
    let brokerage: i128 = orders
        .iter()
        .filter(|o| o.qty() > 0)
        .map(|o| match r.option_brokerage {
            OptionBrokerage::Flat20 => 2_000,
            OptionBrokerage::Min20 => brokerage_of(o.value()),
        })
        .sum();
    let exchange = half_up(turnover * 3_553, 10_000_000);
    let sebi = per_crore(turnover, TODAY.sebi);
    Breakdown {
        brokerage,
        exchange,
        sebi,
        stt: to_rupee(half_up(s.sv * 15, 10_000)),
        stamp: to_rupee(half_up(s.bv * 3, 100_000)),
        gst: half_up((brokerage + exchange + sebi) * 18, 100),
        dp: 0,
        dp_gst: 0,
        sq_fee: 2_000 * s.n_sq,
        sq_gst: 360 * s.n_sq,
    }
}

/// Dhan's estimator screen, one side: `(brokerage, exchange, gst, sebi,
/// stamp or stt, total)`, paise.
pub type ScreenSide = (i128, i128, i128, i128, i128, i128);

/// The replica of Dhan's order estimator for a buy order and a sell order of
/// one instrument (equity when `option` is false): each side alone, GST on
/// brokerage and exchange only, every line half-up to the paisa.
pub fn estimator(buy_value: i128, sell_value: i128, option: bool) -> (ScreenSide, ScreenSide) {
    let side = |value: i128, buy: bool| -> ScreenSide {
        let brokerage = if option { 2_000 } else { brokerage_of(value) };
        let exchange = if option {
            half_up(value * 3_553, 10_000_000)
        } else {
            per_crore(value, 30_700)
        };
        let gst = half_up((brokerage + exchange) * 18, 100);
        let sebi = per_crore(value, 1_000);
        let tax = match (buy, option) {
            (true, _) => half_up(value * 3, 100_000),
            (false, false) => half_up(value * 25, 100_000),
            (false, true) => half_up(value * 15, 10_000),
        };
        (
            brokerage,
            exchange,
            gst,
            sebi,
            tax,
            brokerage + exchange + gst + sebi + tax,
        )
    };
    (side(buy_value, true), side(sell_value, false))
}

/// COSTS.md's 24 contract-note vectors, public so the golden files can
/// write them as scenarios.
pub mod vectors {
    use super::{Order, OrderSide};

    /// COSTS.md's 24 contract notes: (date, orders, dated P, today P,
    /// dated W, today W), all in paise.
    #[allow(
        clippy::too_many_lines,
        reason = "twenty-four contract notes, one row each"
    )]
    pub fn contract_notes() -> Vec<(&'static str, Vec<Order>, [i128; 4])> {
        let o = |side, fills: Vec<(i64, i64)>, sq| Order {
            side,
            fills,
            square_off: sq,
        };
        let (b, s) = (OrderSide::Buy, OrderSide::Sell);
        vec![
            (
                "2018-09-03",
                rt(400, 125_000, 126_250),
                [31_325, 22_579, 32_800, 24_055],
            ),
            (
                "2019-03-29",
                vec![
                    o(s, vec![(615, 81_235)], false),
                    o(b, vec![(615, 80_500)], false),
                ],
                [31_031, 22_341, 32_606, 23_917],
            ),
            (
                "2019-04-01",
                rt(213, 234_565, 233_010),
                [30_999, 22_346, 32_515, 23_821],
            ),
            (
                "2020-05-29",
                rt(3197, 15_640, 15_985),
                [31_298, 22_602, 32_974, 24_277],
            ),
            (
                "2020-06-01",
                vec![
                    o(s, vec![(124, 402_000)], false),
                    o(b, vec![(124, 399_055)], false),
                ],
                [30_912, 22_335, 32_488, 23_910],
            ),
            (
                "2020-06-30",
                rt(833, 60_000, 59_405),
                [30_931, 22_341, 32_406, 23_816],
            ),
            (
                "2020-07-01",
                rt(833, 60_000, 60_600),
                [22_723, 22_577, 24_198, 24_053],
            ),
            (
                "2020-12-31",
                rt(500, 100_000, 101_500),
                [22_733, 22_589, 24_309, 24_164],
            ),
            (
                "2021-01-01",
                vec![
                    o(s, vec![(500, 100_000)], false),
                    o(b, vec![(500, 101_200)], false),
                ],
                [22_966, 22_583, 24_441, 24_058],
            ),
            (
                "2021-03-31",
                rt(1000, 49_995, 50_500),
                [22_961, 22_578, 24_436, 24_055],
            ),
            (
                "2021-04-01",
                rt(1000, 49_995, 50_500),
                [23_011, 22_578, 24_487, 24_055],
            ),
            (
                "2022-07-15",
                rt(200, 250_000, 253_750),
                [23_023, 22_589, 24_598, 24_164],
            ),
            (
                "2022-07-18",
                rt(200, 250_000, 253_750),
                [23_041, 22_589, 24_616, 24_164],
            ),
            (
                "2023-03-31",
                vec![
                    o(s, vec![(6423, 7_785)], false),
                    o(b, vec![(6423, 7_630)], false),
                ],
                [22_768, 22_324, 24_343, 23_900],
            ),
            (
                "2023-04-03",
                vec![
                    o(s, vec![(6423, 7_785)], false),
                    o(b, vec![(6423, 7_630)], false),
                ],
                [22_651, 22_324, 24_226, 23_900],
            ),
            (
                "2024-03-28",
                rt(312, 160_100, 164_000),
                [22_936, 22_602, 24_611, 24_279],
            ),
            (
                "2024-04-01",
                rt(312, 160_100, 164_000),
                [22_900, 22_602, 24_575, 24_279],
            ),
            (
                "2024-09-30",
                rt(51, 987_500, 980_000),
                [22_769, 22_474, 24_244, 23_949],
            ),
            (
                "2024-10-01",
                rt(51, 987_500, 980_000),
                [22_474, 22_474, 23_949, 23_949],
            ),
            (
                "2026-02-27",
                vec![
                    o(s, vec![(40, 500_000), (35, 499_950), (25, 499_900)], false),
                    o(b, vec![(60, 495_000), (40, 495_150)], false),
                ],
                [22_343, 22_343, 30_998, 30_998],
            ),
            (
                "2026-03-02",
                vec![
                    o(s, vec![(40, 500_000), (35, 499_950), (25, 499_900)], false),
                    o(b, vec![(60, 495_000), (40, 495_150)], false),
                ],
                [22_343, 22_343, 30_998, 30_998],
            ),
            (
                "2026-10-30",
                vec![
                    o(
                        b,
                        vec![
                            (40, 160_000),
                            (40, 160_005),
                            (40, 160_010),
                            (40, 160_010),
                            (40, 160_020),
                            (40, 160_025),
                            (40, 160_030),
                            (33, 160_040),
                        ],
                        false,
                    ),
                    o(s, vec![(120, 161_700), (80, 161_650)], false),
                    o(s, vec![(113, 161_105)], true),
                ],
                [27_304, 27_304, 46_609, 46_609],
            ),
            (
                "2026-10-30",
                vec![
                    o(
                        b,
                        vec![
                            (40, 160_000),
                            (40, 160_005),
                            (40, 160_010),
                            (40, 160_010),
                            (40, 160_020),
                            (40, 160_025),
                            (40, 160_030),
                            (32, 160_040),
                        ],
                        false,
                    ),
                    o(s, vec![(120, 161_700), (80, 161_650)], false),
                    o(s, vec![(112, 161_105)], true),
                ],
                [27_191, 27_191, 46_439, 46_439],
            ),
            (
                "2026-10-30",
                vec![
                    o(
                        b,
                        vec![
                            (313, 16_000),
                            (313, 16_000),
                            (313, 16_005),
                            (312, 16_005),
                            (312, 16_010),
                            (312, 16_010),
                            (312, 16_015),
                            (312, 16_015),
                            (313, 16_020),
                            (313, 16_020),
                        ],
                        false,
                    ),
                    o(s, vec![(3125, 15_840)], false),
                ],
                [22_344, 22_344, 39_170, 39_170],
            ),
        ]
    }

    /// A one-fill round trip.
    fn rt(qty: i64, buy: i64, sell: i64) -> Vec<Order> {
        vec![
            Order {
                side: OrderSide::Buy,
                fills: vec![(qty, buy)],
                square_off: false,
            },
            Order {
                side: OrderSide::Sell,
                fills: vec![(qty, sell)],
                square_off: false,
            },
        ]
    }
}

#[cfg(test)]
#[path = "charges_tests.rs"]
pub(crate) mod tests;
