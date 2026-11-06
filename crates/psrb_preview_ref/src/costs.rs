//! F41..F52: every charge of the cost table `costs-table/2026-10-30/r1`, per trade, on
//! both clocks, each line at its worst reading, the headline the larger clock
//! (PREVIEW-SPEC section 3.7; COSTS sections 3..6 and the table's rows and readings).
//!
//! Money is paisa; a rate per crore is held in hundredths of a rupee, so a charge is
//! `value x rate / 10^9` paisa, rounded half-up to the paisa (STT and stamp then to the
//! nearest rupee, 50 paise up).

use crate::money::{half_up, to_rupee};
use crate::time::Day;

/// One fill of an order: shares and price in paisa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Part {
    /// Shares.
    pub qty: i64,
    /// Price, paisa.
    pub price: i64,
}

/// One executed order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Order {
    /// A buy (else a sell).
    pub buy: bool,
    /// Placed by the broker (the auto square-off).
    pub by_broker: bool,
    /// Its fills, each a traded second.
    pub parts: Vec<Part>,
}

impl Order {
    /// Its value in paisa.
    #[must_use]
    pub fn value(&self) -> i128 {
        self.parts
            .iter()
            .map(|p| i128::from(p.qty) * i128::from(p.price))
            .sum()
    }

    /// Its shares.
    #[must_use]
    pub fn shares(&self) -> i128 {
        self.parts.iter().map(|p| i128::from(p.qty)).sum()
    }
}

/// The ten lines of one contract note, paisa.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lines {
    /// Brokerage.
    pub brokerage: i128,
    /// NSE transaction charge plus IPFT.
    pub exchange: i128,
    /// SEBI turnover fee.
    pub sebi: i128,
    /// STT.
    pub stt: i128,
    /// Stamp duty.
    pub stamp: i128,
    /// GST on brokerage, exchange, IPFT (and SEBI where it applies).
    pub gst: i128,
    /// DP charge.
    pub dp: i128,
    /// GST on the DP charge.
    pub dp_gst: i128,
    /// The broker's square-off fee.
    pub square_off: i128,
    /// GST on the square-off fee.
    pub square_off_gst: i128,
}

impl Lines {
    /// The note's total.
    #[must_use]
    pub const fn total(&self) -> i128 {
        self.brokerage
            + self.exchange
            + self.sebi
            + self.stt
            + self.stamp
            + self.gst
            + self.dp
            + self.dp_gst
            + self.square_off
            + self.square_off_gst
    }

    /// The ten lines in table order.
    #[must_use]
    pub const fn as_array(&self) -> [i128; 10] {
        [
            self.brokerage,
            self.exchange,
            self.sebi,
            self.stt,
            self.stamp,
            self.gst,
            self.dp,
            self.dp_gst,
            self.square_off,
            self.square_off_gst,
        ]
    }
}

/// Which clock gave the headline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clock {
    /// The rates in force on the trade date.
    Dated,
    /// Dhan's rates of 2026-10-30 applied to the trade.
    Today,
}

/// One trade priced on both clocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Priced {
    /// Dated clock, official method.
    pub dated_precise: Lines,
    /// Dated clock, every line at its worst reading.
    pub dated_worst: Lines,
    /// Today's clock, official method.
    pub today_precise: Lines,
    /// Today's clock, every line at its worst reading.
    pub today_worst: Lines,
    /// max(dated worst total, today worst total).
    pub headline: i128,
    /// The clock that gave it (the dated clock on a tie, Q8).
    pub from: Clock,
}

/// A refusal to price a trade: it stops the run by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChargeError {
    /// The day lies outside the table's window 2018-09-03..=2026-10-30.
    OutsideWindow(Day),
    /// Bought and sold quantities differ: the position is not squared that day.
    OpenIntradayPosition,
    /// No order, or an order without a fill.
    Empty,
}

/// The rates of one clock on one day, per crore in hundredths of a rupee.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rates {
    /// NSE transaction charge (top slab).
    pub exchange: i128,
    /// IPFT.
    pub ipft: i128,
    /// SEBI turnover fee.
    pub sebi: i128,
    /// GST is charged on the SEBI fee.
    pub gst_on_sebi: bool,
    /// Stamp is the state levy on both sides (before 2020-07-01), else 0.003% of the buy.
    pub state_stamp: bool,
}

fn ymd(day: Day) -> (i32, u32, u32) {
    day.ymd()
}

fn on_or_after(day: Day, y: i32, m: u32, d: u32) -> bool {
    ymd(day) >= (y, m, d)
}

/// F45, F46, F48 and F49's dated rows: the dated clock's rates on `day`.
#[must_use]
pub fn dated_rates(day: Day) -> Rates {
    let exchange = if on_or_after(day, 2026, 3, 1) {
        30_699
    } else if on_or_after(day, 2024, 10, 1) {
        29_700
    } else if on_or_after(day, 2024, 4, 1) {
        32_200
    } else if on_or_after(day, 2023, 4, 1) {
        32_500
    } else if on_or_after(day, 2021, 1, 1) {
        34_500
    } else {
        32_500
    };
    let ipft = if on_or_after(day, 2026, 3, 1) || !on_or_after(day, 2023, 4, 1) {
        1
    } else {
        1_000
    };
    let sebi = if on_or_after(day, 2021, 4, 1) {
        1_000
    } else if on_or_after(day, 2020, 6, 1) {
        500
    } else if on_or_after(day, 2019, 4, 1) {
        1_000
    } else {
        1_500
    };
    Rates {
        exchange,
        ipft,
        sebi,
        gst_on_sebi: on_or_after(day, 2022, 7, 18),
        state_stamp: !on_or_after(day, 2020, 7, 1),
    }
}

/// Today's clock: Dhan's rates of 2026-10-30 on every date.
#[must_use]
pub const fn today_rates() -> Rates {
    Rates {
        exchange: 30_699,
        ipft: 1,
        sebi: 1_000,
        gst_on_sebi: true,
        state_stamp: false,
    }
}

/// A per-crore charge on `value` paisa, half-up to the paisa.
fn per_crore(value: i128, rate: i128) -> i128 {
    half_up(value * rate, 1_000_000_000)
}

/// F42: min(Rs 20, 0.03%) of one order or fill value, half-up to the paisa.
fn brokerage_of(value: i128) -> i128 {
    half_up(value * 3, 10_000).min(2_000)
}

/// The worst listed state's stamp duty on a day's contract-note value (COSTS section 3).
#[must_use]
pub fn state_stamp(turnover: i128) -> i128 {
    let pct = |num: i128| half_up(turnover * num, 100_000);
    [
        pct(5).min(5_000),
        pct(3),
        pct(18).min(4_950),
        pct(2),
        pct(5),
        pct(2).min(20_000),
        5_000,
        pct(6),
        pct(2).min(100_000),
        pct(10).min(10_000),
    ]
    .into_iter()
    .max()
    .unwrap_or(0)
}

/// 0.025% of a sell value, to two decimals, then to the rupee.
fn stt_of(value: i128) -> i128 {
    to_rupee(half_up(value * 25, 100_000))
}

/// 0.003% of a buy value, to two decimals, then to the rupee.
fn stamp_nd_of(value: i128) -> i128 {
    to_rupee(half_up(value * 3, 100_000))
}

fn gst_once(base: i128) -> i128 {
    half_up(base * 18, 100)
}

fn gst_split(base: i128) -> i128 {
    2 * half_up(base * 9, 100)
}

/// The official method and the worst readings of one clock (F42..F50).
fn price_clock(orders: &[Order], rates: Rates) -> (Lines, Lines) {
    let buy_value: i128 = orders.iter().filter(|o| o.buy).map(Order::value).sum();
    let sell_value: i128 = orders.iter().filter(|o| !o.buy).map(Order::value).sum();
    let buy_qty: i128 = orders.iter().filter(|o| o.buy).map(Order::shares).sum();
    let sell_qty: i128 = orders.iter().filter(|o| !o.buy).map(Order::shares).sum();
    let turnover = buy_value + sell_value;
    let both = rates.exchange + rates.ipft;

    let brk_order: i128 = orders.iter().map(|o| brokerage_of(o.value())).sum();
    let brk_fill: i128 = orders
        .iter()
        .flat_map(|o| o.parts.iter())
        .map(|p| brokerage_of(i128::from(p.qty) * i128::from(p.price)))
        .sum();

    let ex_once = per_crore(turnover, both);
    let ex_split = per_crore(turnover, rates.exchange) + per_crore(turnover, rates.ipft);
    let ex_order: i128 = orders.iter().map(|o| per_crore(o.value(), both)).sum();

    let sebi_once = per_crore(turnover, rates.sebi);
    let sebi_order: i128 = orders
        .iter()
        .map(|o| per_crore(o.value(), rates.sebi))
        .sum();

    let squared = buy_qty.min(sell_qty);
    let vwap = half_up(turnover, buy_qty + sell_qty);
    let stt_vwap = to_rupee(half_up(squared * vwap * 25, 100_000));
    let stt_sell = stt_of(sell_value);
    let stt_per_order: i128 = orders
        .iter()
        .filter(|o| !o.buy)
        .map(|o| stt_of(o.value()))
        .sum();

    let (stamp_precise, stamp_worst) = if rates.state_stamp {
        let precise = state_stamp(turnover);
        (precise, precise.max(to_rupee(precise)))
    } else {
        let precise = stamp_nd_of(buy_value);
        let per_order: i128 = orders
            .iter()
            .filter(|o| o.buy)
            .map(|o| stamp_nd_of(o.value()))
            .sum();
        (precise, precise.max(per_order))
    };

    let broker_orders = i128::try_from(orders.iter().filter(|o| o.by_broker).count()).unwrap_or(0);
    let square_off = 2_000 * broker_orders;
    let square_off_gst = gst_once(2_000) * broker_orders;

    let precise_base = brk_order + ex_once + if rates.gst_on_sebi { sebi_once } else { 0 };
    let precise = Lines {
        brokerage: brk_order,
        exchange: ex_once,
        sebi: sebi_once,
        stt: stt_vwap,
        stamp: stamp_precise,
        gst: gst_once(precise_base),
        dp: 0,
        dp_gst: 0,
        square_off,
        square_off_gst,
    };
    let brokerage = brk_order.max(brk_fill);
    let exchange = ex_once.max(ex_split).max(ex_order);
    let sebi = sebi_once.max(sebi_order);
    let worst_base = brokerage + exchange + if rates.gst_on_sebi { sebi } else { 0 };
    let worst = Lines {
        brokerage,
        exchange,
        sebi,
        stt: stt_vwap.max(stt_sell).max(stt_per_order),
        stamp: stamp_worst,
        gst: gst_once(worst_base).max(gst_split(worst_base)),
        dp: 1_250,
        dp_gst: gst_once(1_250),
        square_off,
        square_off_gst,
    };
    (precise, worst)
}

/// The first and last days the table prices.
#[must_use]
pub fn check_window() -> (Option<Day>, Option<Day>) {
    (Day::from_ymd(2018, 9, 3), Day::from_ymd(2026, 10, 30))
}

/// F51 and F52's input: prices one trade's orders on `day` on both clocks.
///
/// # Errors
/// [`ChargeError`]: a day outside the window, unequal bought and sold shares, or no fill.
pub fn price_trade(orders: &[Order], day: Day) -> Result<Priced, ChargeError> {
    let (first, last) = check_window();
    if first.is_none_or(|f| day < f) || last.is_none_or(|l| day > l) {
        return Err(ChargeError::OutsideWindow(day));
    }
    if orders.is_empty() || orders.iter().any(|o| o.parts.is_empty()) {
        return Err(ChargeError::Empty);
    }
    let bought: i128 = orders.iter().filter(|o| o.buy).map(Order::shares).sum();
    let sold: i128 = orders.iter().filter(|o| !o.buy).map(Order::shares).sum();
    if bought != sold {
        return Err(ChargeError::OpenIntradayPosition);
    }
    let (dated_precise, dated_worst) = price_clock(orders, dated_rates(day));
    let (today_precise, today_worst) = price_clock(orders, today_rates());
    let (headline, from) = if dated_worst.total() >= today_worst.total() {
        (dated_worst.total(), Clock::Dated)
    } else {
        (today_worst.total(), Clock::Today)
    };
    Ok(Priced {
        dated_precise,
        dated_worst,
        today_precise,
        today_worst,
        headline,
        from,
    })
}

/// F41: a trade's two orders: the entry (a long buys, a short sells) and the exit, one
/// fill each; the exit `by_broker` on a square-off or an unclosable exit.
#[must_use]
pub fn trade_orders(long: bool, qty: i64, entry: i64, exit: i64, by_broker: bool) -> Vec<Order> {
    vec![
        Order {
            buy: long,
            by_broker: false,
            parts: vec![Part { qty, price: entry }],
        },
        Order {
            buy: !long,
            by_broker,
            parts: vec![Part { qty, price: exit }],
        },
    ]
}

/// F52: gross (sell value minus buy value) and net (gross minus the headline).
#[must_use]
pub fn gross_and_net(long: bool, qty: i64, entry: i64, exit: i64, headline: i128) -> (i128, i128) {
    let move_ = i128::from(exit) - i128::from(entry);
    let gross = i128::from(qty) * if long { move_ } else { -move_ };
    (gross, gross - headline)
}

#[cfg(test)]
#[path = "costs_tests.rs"]
mod tests;
