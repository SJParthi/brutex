//! Both cost clocks on one trade: its own day's official rates and Dhan's
//! rates of today, each under the official method and the worst reading, and
//! the headline that is the worse of the two worst (package FNO-COSTS,
//! D-8110, D-8112).
//!
//! # The rule
//!
//! Operator rule 7 (30 Oct 2026): every trade carries two cost clocks, both
//! computed and labelled — its own day's official rates (dated) and today's
//! Dhan rates (as if placed live today). The headline and the pick use the
//! worse of the two for each trade, and both are shown. [`price_trade`]
//! returns all four notes in a [`TwoClock`]: dated official, dated worst,
//! today official, today worst, and the headline
//! `max(dated worst, today worst)` (COSTS.md section 6, last line).
//!
//! # What goes in
//!
//! A [`TradeFills`]: the trade's orders in time order, the first opening it,
//! each with its per-second fills `(second, quantity, price)` from the GDFL
//! seconds walk (FNO-SEC-EXEC), and the broker's square-off orders marked.
//! Every fill is already the worst price inside its second; this module adds
//! the charges and nothing else.
//!
//! # Cost
//!
//! Summing the fills is one pass, one step per fill; every charge after it is
//! a fixed number of steps per order. `crates/costs/benches/ratio.rs` measures
//! it: `C-FNO-COSTS-02` holds the cost per fill flat from 5 to 500 fills and
//! `C-FNO-COSTS-03` the worst over every reading flat in the trade's size.

use std::hint::black_box;
use std::time::Instant;

use brutex_core::price::Paisa;

use crate::charge_error::ChargeError;
use crate::day::TradeDay;
use crate::equity_dated::{
    EQUITY_DATED_VERSION, EquityRates, TODAY_RATES, check_window, levy, rates_on,
};
use crate::equity_readings::{NoteInput, NoteLines, ReadingSet, priced, worst};
use crate::error::CostError;
use crate::exact::Ratio;
use crate::rate::OrderSide;

/// Which cost clock prices a note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Clock {
    /// The official rates in force on the trade's own day.
    Dated,
    /// Dhan's rates as read on 30 Oct 2026, as if the trade were placed live
    /// today.
    Today,
}

impl Clock {
    /// The rates this clock prices a trade on `day` at.
    ///
    /// # Errors
    ///
    /// [`ChargeError::Cost`] carrying a refusal for a day outside the
    /// table's window, on either clock: a trade outside the window is not
    /// the backtest's.
    pub fn rates(self, day: TradeDay) -> Result<EquityRates, ChargeError> {
        match self {
            Self::Dated => rates_on(day),
            Self::Today => {
                check_window(day)?;
                Ok(TODAY_RATES)
            }
        }
    }

    /// The label every figure of this clock carries.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Dated => "dated: the official rates in force on the trade day",
            Self::Today => "today: Dhan's rates of 2026-10-30, as if placed live today",
        }
    }
}

/// One fill: the traded second an order filled in, how many units, and the
/// price (the worst inside that second).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fill {
    /// The second, on any clock that increases through the day (the GDFL
    /// second's own stamp).
    pub second: i64,
    /// Units filled.
    pub quantity: i64,
    /// The price, in paise.
    pub price: Paisa,
}

/// One order and its fills.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Order {
    /// Buy or sell.
    pub side: OrderSide,
    /// Whether Dhan placed it to square off an open position (after 15:10).
    pub by_broker: bool,
    /// Its fills, one per traded second, in time order.
    pub fills: Vec<Fill>,
}

/// One trade's orders, in time order: the entry, the exits, and any square-off
/// the broker placed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TradeFills {
    /// The orders.
    pub orders: Vec<Order>,
}

/// Which clock gave the headline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HeadlineFrom {
    /// The dated clock's worst was larger.
    Dated,
    /// Today's clock's worst was larger.
    Today,
    /// The two were equal.
    Both,
}

/// One trade priced on both clocks.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TwoClock {
    day: TradeDay,
    dated_official: NoteLines,
    dated_worst: NoteLines,
    dated_readings: ReadingSet,
    dated_rates: EquityRates,
    today_official: NoteLines,
    today_worst: NoteLines,
    today_readings: ReadingSet,
    headline: Paisa,
    headline_from: HeadlineFrom,
    quantity: i64,
    buy_value: Paisa,
    sell_value: Paisa,
    gross: Paisa,
    net: Paisa,
}

impl TwoClock {
    /// The trade day.
    #[must_use]
    pub const fn day(&self) -> TradeDay {
        self.day
    }

    /// The dated clock, official method.
    #[must_use]
    pub const fn dated_official(&self) -> NoteLines {
        self.dated_official
    }

    /// The dated clock, every line at its worst reading.
    #[must_use]
    pub const fn dated_worst(&self) -> NoteLines {
        self.dated_worst
    }

    /// The readings that give the dated worst.
    #[must_use]
    pub const fn dated_readings(&self) -> ReadingSet {
        self.dated_readings
    }

    /// The rows the dated clock priced at.
    #[must_use]
    pub const fn dated_rates(&self) -> EquityRates {
        self.dated_rates
    }

    /// Today's clock, official method.
    #[must_use]
    pub const fn today_official(&self) -> NoteLines {
        self.today_official
    }

    /// Today's clock, every line at its worst reading.
    #[must_use]
    pub const fn today_worst(&self) -> NoteLines {
        self.today_worst
    }

    /// The readings that give today's worst.
    #[must_use]
    pub const fn today_readings(&self) -> ReadingSet {
        self.today_readings
    }

    /// The rows today's clock priced at.
    #[must_use]
    pub const fn today_rates(&self) -> EquityRates {
        TODAY_RATES
    }

    /// The headline charge: the worse of the two worst totals.
    #[must_use]
    pub const fn headline(&self) -> Paisa {
        self.headline
    }

    /// Which clock gave the headline.
    #[must_use]
    pub const fn headline_from(&self) -> HeadlineFrom {
        self.headline_from
    }

    /// The squared quantity.
    #[must_use]
    pub const fn quantity(&self) -> i64 {
        self.quantity
    }

    /// The buy value.
    #[must_use]
    pub const fn buy_value(&self) -> Paisa {
        self.buy_value
    }

    /// The sell value.
    #[must_use]
    pub const fn sell_value(&self) -> Paisa {
        self.sell_value
    }

    /// "Signal only, before charges": the result at the worst-price fills,
    /// with no charge.
    #[must_use]
    pub const fn gross(&self) -> Paisa {
        self.gross
    }

    /// Net of every charge: the gross less the headline.
    #[must_use]
    pub const fn net(&self) -> Paisa {
        self.net
    }

    /// The cost table's version, for the run identity.
    #[must_use]
    pub const fn version(&self) -> &'static str {
        EQUITY_DATED_VERSION
    }
}

/// Prices one trade on both clocks.
///
/// # Errors
///
/// * Every refusal of [`NoteInput::from_fills`]: no order, an order with no
///   fill, a fill that is not positive or not later than its order's last, a
///   trade over the fill bound, a square-off on the opening side, an
///   overflow, or a position left open.
/// * [`ChargeError::Cost`] carrying a refusal for a day outside the table's
///   window.
/// * [`ChargeError::Cost`] carrying [`CostError::Overflow`] when the net
///   leaves `i64`.
///
/// # Examples
///
/// ```
/// use brutex_core::price::Paisa;
/// use costs::day::TradeDay;
/// use costs::equity_clock::{Fill, Order, TradeFills, price_trade};
/// use costs::rate::OrderSide;
///
/// // COSTS.md CN01: 400 bought at Rs 1,250, sold at Rs 1,262.50, 2018-09-03.
/// let leg = |side, price| Order {
///     side,
///     by_broker: false,
///     fills: vec![Fill { second: 0, quantity: 400, price: Paisa::from_raw(price) }],
/// };
/// let trade = TradeFills { orders: vec![leg(OrderSide::Buy, 1_250_00), leg(OrderSide::Sell, 1_262_50)] };
/// let priced = price_trade(&trade, TradeDay::new(2018, 9, 3)?)?;
/// assert_eq!(priced.dated_official().total.raw(), 313_25);
/// assert_eq!(priced.today_official().total.raw(), 225_79);
/// assert_eq!(priced.gross().raw(), 5_000_00);
/// assert_eq!(priced.headline(), priced.dated_worst().total);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn price_trade(fills: &TradeFills, day: TradeDay) -> Result<TwoClock, ChargeError> {
    let input = NoteInput::from_fills(fills, day)?;
    let dated_rates = rates_on(day)?;
    let dated = priced(&input, &dated_rates);
    let today = priced(&input, &TODAY_RATES);
    let (dated_total, today_total) = (dated.worst.total, today.worst.total);
    let (headline, headline_from) = match dated_total.cmp(&today_total) {
        std::cmp::Ordering::Greater => (dated_total, HeadlineFrom::Dated),
        std::cmp::Ordering::Less => (today_total, HeadlineFrom::Today),
        std::cmp::Ordering::Equal => (dated_total, HeadlineFrom::Both),
    };
    let gross = input.gross();
    let net = gross
        .raw()
        .checked_sub(headline.raw())
        .ok_or(ChargeError::Cost(CostError::Overflow {
            operation: "a trade's net after charges",
        }))?;
    Ok(TwoClock {
        day,
        dated_official: dated.official,
        dated_worst: dated.worst,
        dated_readings: dated.readings,
        dated_rates,
        today_official: today.official,
        today_worst: today.worst,
        today_readings: today.readings,
        headline,
        headline_from,
        quantity: input.quantity(),
        buy_value: input.buy_value(),
        sell_value: input.sell_value(),
        gross,
        net: Paisa::from_raw(net),
    })
}

// ---------------------------------------------------------------------------
// Dhan's data API: a monthly running cost, not a per-trade charge
// ---------------------------------------------------------------------------

/// Dhan's Data API subscription: Rs 499 a month, plus taxes.
pub const DHAN_DATA_API_MONTHLY: Paisa = Paisa::from_raw(49_900);

/// Where the Data API figure was read.
pub const DHAN_DATA_API_SOURCE: &str = "Dhan support, DhanHQ Data API subscription; \
     https://dhan.co/support/platforms/dhanhq-api/how-does-the-dhanhq-data-api-subscription-work/; \
     read 2026-10-13 (state/fno-winners-20261013/sources-charges.md row B-13, line 37); \"The \
     subscription costs ₹499 plus applicable taxes per month and renews every 30 days.\" GST at \
     18% (GST-1) taken as the applicable tax; a running cost for the account replay, never a \
     per-trade charge";

/// One month of the Data API: the fee and its GST at 18%.
#[must_use]
pub fn dhan_data_api_month() -> (Paisa, Paisa) {
    let gst = levy(i128::from(DHAN_DATA_API_MONTHLY.raw()), Ratio::new(18, 100));
    (
        DHAN_DATA_API_MONTHLY,
        crate::equity_dated::narrow_fitting(gst),
    )
}

// ---------------------------------------------------------------------------
// Measurement (benches C-FNO-COSTS-01..03; the wrappers in benches/ratio.rs,
// which sort the samples: gate 11 rule 4 keeps a sort out of production code)
// ---------------------------------------------------------------------------

/// Picoseconds per unit of work from a start instant.
fn per_unit(start: Instant, units: usize) -> u64 {
    let nanos = start.elapsed().as_nanos();
    let units = u128::try_from(units.max(1)).unwrap_or(u128::MAX);
    u64::try_from(nanos * 1_000 / units).unwrap_or(u64::MAX)
}

/// `C-FNO-COSTS-01`: `samples` timings of `batch` dated lookups each,
/// cycling through `days`, in picoseconds per lookup.
#[must_use]
pub fn measure_rates_on(days: &[TradeDay], samples: usize, batch: usize) -> Vec<u64> {
    let mut cycle = days.iter().cycle();
    let mut out = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        for _ in 0..batch {
            if let Some(&day) = cycle.next() {
                let _ = black_box(rates_on(black_box(day)));
            }
        }
        out.push(per_unit(start, batch));
    }
    out
}

/// A long trade for the benches: `fills` entry fills of `40 × scale` units
/// each at Rs 1,600.00 and up by a paisa a second, and one exit fill of the
/// whole quantity at Rs 1,617.00. A bench input, not a price: a size past
/// `i64` saturates here and is then refused by [`NoteInput::from_fills`].
#[must_use]
pub fn synthetic_trade(fills: usize, scale: i64) -> TradeFills {
    let fills = fills.max(1);
    let each = scale.max(1).saturating_mul(40);
    let entry: Vec<Fill> = (0..fills)
        .map(|at| {
            let step = i64::try_from(at).unwrap_or(i64::MAX);
            Fill {
                second: step,
                quantity: each,
                price: Paisa::from_raw(160_000 + step % 50),
            }
        })
        .collect();
    let total = each.saturating_mul(i64::try_from(fills).unwrap_or(i64::MAX));
    TradeFills {
        orders: vec![
            Order {
                side: OrderSide::Buy,
                by_broker: false,
                fills: entry,
            },
            Order {
                side: OrderSide::Sell,
                by_broker: false,
                fills: vec![Fill {
                    second: i64::MAX,
                    quantity: total,
                    price: Paisa::from_raw(161_700),
                }],
            },
        ],
    }
}

/// `C-FNO-COSTS-02`: `samples` timings of [`price_trade`] on a trade of
/// `fills` entry fills, in picoseconds per fill.
///
/// # Errors
///
/// Any refusal of [`price_trade`] (none for the synthetic trade on `day`
/// inside the window).
pub fn measure_price_trade(
    fills: usize,
    day: TradeDay,
    samples: usize,
) -> Result<Vec<u64>, ChargeError> {
    let trade = synthetic_trade(fills, 1);
    let mut out = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        black_box(price_trade(black_box(&trade), day)?);
        out.push(per_unit(start, fills + 1));
    }
    Ok(out)
}

/// `C-FNO-COSTS-03`: `samples` timings of [`worst`] on the dated clock, over
/// every reading, for an eight-fill trade of `40 × scale` units a fill, in
/// picoseconds per trade.
///
/// # Errors
///
/// Any refusal of [`NoteInput::from_fills`] or [`worst`].
pub fn measure_worst(scale: i64, day: TradeDay, samples: usize) -> Result<Vec<u64>, ChargeError> {
    let input = NoteInput::from_fills(&synthetic_trade(8, scale), day)?;
    let mut out = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        black_box(worst(black_box(&input), Clock::Dated)?);
        out.push(per_unit(start, 1));
    }
    Ok(out)
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::inconsistent_digit_grouping
)]
mod tests {
    use super::*;

    fn day(year: u16, month: u8, d: u8) -> TradeDay {
        TradeDay::new(year, month, d).expect("a real date")
    }

    fn leg(side: OrderSide, quantity: i64, price: i64) -> Order {
        Order {
            side,
            by_broker: false,
            fills: vec![Fill {
                second: 0,
                quantity,
                price: Paisa::from_raw(price),
            }],
        }
    }

    #[test]
    fn each_clock_names_itself_and_reads_its_rates() {
        assert!(Clock::Dated.label().starts_with("dated"));
        assert!(Clock::Today.label().starts_with("today"));
        let early = day(2019, 1, 2);
        assert_eq!(
            Clock::Dated.rates(early).expect("in the window").nse.id,
            "EX-2009"
        );
        assert_eq!(Clock::Today.rates(early), Ok(TODAY_RATES));
        assert!(Clock::Today.rates(day(2018, 9, 2)).is_err());
        assert!(Clock::Dated.rates(day(2026, 10, 31)).is_err());
    }

    #[test]
    fn the_headline_comes_from_whichever_clock_is_worse() {
        // 2025-01-15, Rs 25 lakh a side: the dated EX-2024B (Rs 297) and
        // IP-2023 (Rs 10) split exactly, Rs 148.50 + Rs 5.00 a side; today's
        // Rs 306.99 + Rs 0.01 split on Rs 50 lakh rounds Rs 153.495 and
        // Rs 0.005 up, so the exchange line's worst is a paisa dearer today
        // and the headline is today's.
        let trade = TradeFills {
            orders: vec![
                leg(OrderSide::Buy, 2_500, 1_000_00),
                leg(OrderSide::Sell, 2_500, 1_000_00),
            ],
        };
        let priced = price_trade(&trade, day(2025, 1, 15)).expect("in the window");
        assert_eq!(priced.dated_worst().exchange.raw(), 153_50);
        assert_eq!(priced.today_worst().exchange.raw(), 153_51);
        assert_eq!(priced.headline_from(), HeadlineFrom::Today);
        assert_eq!(priced.headline(), priced.today_worst().total);
        assert!(priced.today_worst().total > priced.dated_worst().total);
        assert_eq!(priced.gross().raw(), 0);
        // A pre-2020 day: the State stamp makes the dated clock the worse.
        let early = price_trade(&trade, day(2019, 1, 2)).expect("in the window");
        assert_eq!(early.headline_from(), HeadlineFrom::Dated);
        // The read day: the two clocks are one.
        let same = price_trade(&trade, day(2026, 10, 30)).expect("in the window");
        assert_eq!(same.headline_from(), HeadlineFrom::Both);
        assert_eq!(same.dated_rates(), same.today_rates());
        assert_eq!(same.dated_readings(), same.today_readings());
    }

    #[test]
    fn a_net_that_leaves_i64_is_refused_and_every_input_refusal_passes_through() {
        // Bought one share at the largest price, sold it at a paisa: the
        // gross is about -i64::MAX and the charges push the net below it.
        let trade = TradeFills {
            orders: vec![leg(OrderSide::Buy, 1, i64::MAX), leg(OrderSide::Sell, 1, 1)],
        };
        assert_eq!(
            price_trade(&trade, day(2026, 10, 30)),
            Err(ChargeError::Cost(CostError::Overflow {
                operation: "a trade's net after charges"
            }))
        );
        assert_eq!(
            price_trade(&TradeFills { orders: vec![] }, day(2026, 10, 30)),
            Err(ChargeError::EmptyDay)
        );
        let fine = TradeFills {
            orders: vec![leg(OrderSide::Buy, 1, 100), leg(OrderSide::Sell, 1, 100)],
        };
        assert!(price_trade(&fine, day(2018, 9, 2)).is_err());
    }

    #[test]
    fn the_data_api_is_rs_499_and_its_gst_a_month() {
        assert_eq!(
            dhan_data_api_month(),
            (Paisa::from_raw(499_00), Paisa::from_raw(89_82))
        );
        assert!(DHAN_DATA_API_SOURCE.contains("B-13"));
    }

    #[test]
    fn the_measurements_run_at_tiny_sizes_and_refuse_what_they_cannot_price() {
        let days = [day(2019, 1, 2), day(2024, 10, 1)];
        assert_eq!(measure_rates_on(&days, 3, 4).len(), 3);
        assert_eq!(measure_rates_on(&[], 2, 4).len(), 2);
        assert_eq!(measure_rates_on(&days, 2, 0).len(), 2);
        let on = day(2026, 10, 30);
        assert_eq!(measure_price_trade(3, on, 2).expect("prices").len(), 2);
        assert!(measure_price_trade(3, day(2026, 10, 31), 1).is_err());
        assert_eq!(
            measure_worst(10, day(2019, 1, 2), 2).expect("prices").len(),
            2
        );
        assert!(measure_worst(1, day(2026, 10, 31), 1).is_err());
        assert!(measure_worst(i64::MAX, on, 1).is_err());
        // The synthetic trade: a floor of one fill and one unit-scale, and
        // the shape the benches measure.
        let one = synthetic_trade(0, 0);
        assert_eq!(one.orders[0].fills.len(), 1);
        assert_eq!(one.orders[0].fills[0].quantity, 40);
        let eight = synthetic_trade(8, 3);
        assert_eq!(eight.orders[0].fills.len(), 8);
        assert_eq!(eight.orders[1].fills[0].quantity, 8 * 120);
        assert!(price_trade(&eight, on).is_ok());
        // A synthetic size past i64 is refused, not wrapped.
        assert!(price_trade(&synthetic_trade(2, i64::MAX), on).is_err());
    }
}
