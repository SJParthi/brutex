//! Dhan's public brokerage calculator, line by line — **for comparison only**.
//!
//! <https://dhan.co/calculators/brokerage-calculator/>, read through the page
//! on 15 and 16 Oct 2026. This module reproduces what that page shows, so a
//! precise figure ([`crate::stack`]) can be set beside the figure a Dhan user
//! sees. It is **not** a contract note and never stands in for one: every
//! output carries [`DHAN_LABEL`], `Dhan calculator (estimate)`.
//!
//! # The page's arithmetic, as measured
//!
//! | Line | Dhan computes | Shown |
//! |---|---|---|
//! | brokerage | Σ over executed orders of `min(₹20, 0.0003 × order value)`; delivery ₹0; MTF as intraday; F&O ₹20 per order | 2 decimals |
//! | exchange | turnover × 0.0000307 NSE equity (IPFT included) / 0.000038 BSE equity / 0.0000183 futures / 0.0003553 options premium | 2 decimals |
//! | SEBI | turnover × 0.000001 | 2 decimals |
//! | STT | intraday 0.00025 × **sell** value; delivery and MTF 0.001 × turnover; futures 0.0005 × sell; options 0.0015 × sell premium | half-up to the rupee |
//! | stamp | intraday 0.00003 × buy; delivery and MTF 0.00015 × buy; futures 0.00002 × buy; options 0.00003 × buy premium | half-up to the rupee |
//! | DP | ₹14.75 on a delivery or MTF sell | 2 decimals |
//! | GST | 0.18 × (brokerage + the exchange line rounded to 4 decimals) — the SEBI fee is **not** in it | 2 decimals |
//! | total | the sum of the lines with STT and stamp **unrounded**, rounded to 2 decimals | 2 decimals |
//!
//! On the 48 live cases this repository holds, every line agrees. The total
//! agrees exactly on most and is **one paisa** off on the rest; which way the
//! page rounds its last paisa is UNVERIFIED, and
//! `costs::dhan_calculator::the_one_paisa_total_misses_are_listed_with_their_direction`
//! records each miss so a rule that fits all of them can be proven later.
//!
//! # Exact, with no float
//!
//! Every unrounded line is an integer count of [`UNITS_PER_PAISA`]-ths of a
//! paisa (10⁻⁹ rupee). Every constant above is a whole number of those units
//! per paisa of base, so each line is exact and their sum is exact — the
//! page's floating-point sum is reproduced by its exact value, and the one
//! rounding at the end is the only one.

use std::fmt;

use brutex_core::instrument::Exchange;
use brutex_core::price::Paisa;

use crate::charge_error::ChargeError;
use crate::error::CostError;
pub use crate::exact::{Exact, UNITS_PER_PAISA};
use crate::exact::{Rupees, half_up};
use crate::schedule::{Product, TradeKind};

/// What every calculator output says it is.
pub const DHAN_LABEL: &str = "Dhan calculator (estimate)";

/// Units per rupee.
const UNITS_PER_RUPEE: i128 = UNITS_PER_PAISA * 100;

/// Units in 10⁻⁴ rupee — the precision the page rounds the exchange line to
/// before it takes GST on it.
const UNITS_PER_TEN_THOUSANDTH_RUPEE: i128 = UNITS_PER_RUPEE / 10_000;

// The page's constants, each in units per paisa of the base it is levied on:
// `0.0003` of a paisa is 3,000 units.
const BROKERAGE_RATE: i128 = 3_000;
const BROKERAGE_CAP: i128 = 2_000 * UNITS_PER_PAISA; // ₹20
const FNO_BROKERAGE: i128 = 2_000 * UNITS_PER_PAISA; // ₹20 per order
const EXCHANGE_NSE_EQUITY: i128 = 307;
const EXCHANGE_BSE_EQUITY: i128 = 380;
const EXCHANGE_FUTURES: i128 = 183;
const EXCHANGE_OPTIONS: i128 = 3_553;
const SEBI: i128 = 10;
const STT_INTRADAY: i128 = 2_500;
const STT_DELIVERY: i128 = 10_000;
const STT_FUTURES: i128 = 5_000;
const STT_OPTIONS: i128 = 15_000;
const STAMP_INTRADAY: i128 = 300;
const STAMP_DELIVERY: i128 = 1_500;
const STAMP_FUTURES: i128 = 200;
const STAMP_OPTIONS: i128 = 300;
const DP: i128 = 1_475 * UNITS_PER_PAISA; // ₹14.75
const GST_PERCENT: i128 = 18;

/// One quote from the calculator: the inputs, every line unrounded, and the
/// total the page shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DhanQuote {
    kind: TradeKind,
    exchange: Exchange,
    quantity: i64,
    buy_value: Option<Paisa>,
    sell_value: Option<Paisa>,
    turnover: Paisa,
    brokerage: Exact,
    exchange_charge: Exact,
    sebi: Exact,
    stt: Exact,
    stamp: Exact,
    gst: Exact,
    dp: Exact,
    total: Exact,
}

/// Quotes one calculator input: a kind, a venue, a quantity in units, and a
/// buy price, a sell price, or both. Each side present is one executed order.
///
/// # Errors
///
/// * [`ChargeError::Cost`] carrying [`CostError::NotPositive`] for a quantity
///   or a price that is not strictly positive — the inputs Dhan's page shows
///   as `0` or `NaN`, refused here by name.
/// * [`ChargeError::NoOrder`] when neither price is given.
/// * [`ChargeError::NotOnPage`] for BSE futures or options, an ETF or a
///   commodity, none of which was ever read from the page.
/// * [`ChargeError::Cost`] carrying an overflow of a value or the turnover.
///
/// # Examples
///
/// ```
/// use brutex_core::instrument::Exchange;
/// use brutex_core::price::Paisa;
/// use costs::dhan_calc::quote;
/// use costs::schedule::{Product, TradeKind};
///
/// let q = quote(
///     TradeKind::Equity(Product::Intraday),
///     Exchange::Nse,
///     50,
///     Some(Paisa::from_raw(1_520_00)),
///     Some(Paisa::from_raw(1_530_00)),
/// )?;
/// assert_eq!(q.total()?.raw(), 74_28);
/// assert!(q.to_string().starts_with("Dhan calculator (estimate)"));
/// # Ok::<(), costs::charge_error::ChargeError>(())
/// ```
pub fn quote(
    kind: TradeKind,
    exchange: Exchange,
    quantity: i64,
    buy: Option<Paisa>,
    sell: Option<Paisa>,
) -> Result<DhanQuote, ChargeError> {
    if quantity <= 0 {
        return Err(not_positive("quantity", quantity));
    }
    let buy_value = side_value(quantity, buy, "buy price")?;
    let sell_value = side_value(quantity, sell, "sell price")?;
    if buy_value.is_none() && sell_value.is_none() {
        return Err(ChargeError::NoOrder);
    }
    let turnover = buy_value
        .map_or(0, Paisa::raw)
        .checked_add(sell_value.map_or(0, Paisa::raw))
        .map(Paisa::from_raw)
        .ok_or(ChargeError::Cost(CostError::Overflow {
            operation: "the calculator turnover",
        }))?;
    let brokerage = Exact(order_brokerage(kind, buy_value) + order_brokerage(kind, sell_value));
    let lines = page_lines(
        kind,
        exchange,
        buy_value.unwrap_or(Paisa::ZERO),
        sell_value.unwrap_or(Paisa::ZERO),
        brokerage,
        sell_value.is_some(),
    )?;
    Ok(DhanQuote {
        kind,
        exchange,
        quantity,
        buy_value,
        sell_value,
        turnover,
        brokerage: lines.brokerage,
        exchange_charge: lines.exchange_charge,
        sebi: lines.sebi,
        stt: lines.stt,
        stamp: lines.stamp,
        gst: lines.gst,
        dp: lines.dp,
        total: lines.total,
    })
}

/// The page's constants for one kind at one venue.
struct PageRates {
    /// Exchange charge, units per paisa of turnover.
    exchange: i128,
    /// STT, units per paisa of what it is levied on.
    stt: i128,
    /// Whether STT is levied on the turnover (delivery, MTF) rather than on
    /// the sell value.
    stt_on_turnover: bool,
    /// Stamp duty, units per paisa of the buy value.
    stamp: i128,
    /// Whether a sell pays the DP charge (delivery, MTF).
    dp_on_sell: bool,
}

/// The page's constants for `kind` at `exchange`.
///
/// # Errors
///
/// [`ChargeError::NotOnPage`] for an ETF, a commodity, and BSE futures and
/// options, none of which was ever read from the page.
fn page_rates(kind: TradeKind, exchange: Exchange) -> Result<PageRates, ChargeError> {
    let on_sell = |exchange, stt, stamp| PageRates {
        exchange,
        stt,
        stt_on_turnover: false,
        stamp,
        dp_on_sell: false,
    };
    match (kind, exchange) {
        (TradeKind::Etf, _) => Err(ChargeError::NotOnPage {
            what: "an ETF (no calculator reading exists)",
        }),
        (TradeKind::Commodity, _) => Err(ChargeError::NotOnPage {
            what: "a commodity (MCX is outside the engine's scope)",
        }),
        (TradeKind::Equity(product), _) => {
            let exchange_rate = match exchange {
                Exchange::Nse => EXCHANGE_NSE_EQUITY,
                Exchange::Bse => EXCHANGE_BSE_EQUITY,
            };
            Ok(match product {
                Product::Intraday => on_sell(exchange_rate, STT_INTRADAY, STAMP_INTRADAY),
                Product::Delivery | Product::Mtf => PageRates {
                    exchange: exchange_rate,
                    stt: STT_DELIVERY,
                    stt_on_turnover: true,
                    stamp: STAMP_DELIVERY,
                    dp_on_sell: true,
                },
            })
        }
        (TradeKind::Futures, Exchange::Nse) => {
            Ok(on_sell(EXCHANGE_FUTURES, STT_FUTURES, STAMP_FUTURES))
        }
        (TradeKind::Options, Exchange::Nse) => {
            Ok(on_sell(EXCHANGE_OPTIONS, STT_OPTIONS, STAMP_OPTIONS))
        }
        (TradeKind::Futures | TradeKind::Options, Exchange::Bse) => Err(ChargeError::NotOnPage {
            what: "BSE futures and options",
        }),
    }
}

/// Every line the page computes for one input, unrounded, and its total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PageLines {
    /// Σ over executed orders of the page's brokerage.
    pub(crate) brokerage: Exact,
    /// Exchange transaction charge, IPFT included.
    pub(crate) exchange_charge: Exact,
    /// SEBI fee.
    pub(crate) sebi: Exact,
    /// STT.
    pub(crate) stt: Exact,
    /// Stamp duty.
    pub(crate) stamp: Exact,
    /// GST, on brokerage and the exchange line rounded to 10⁻⁴ rupee.
    pub(crate) gst: Exact,
    /// DP charge, its GST included.
    pub(crate) dp: Exact,
    /// The exact sum of every line above.
    pub(crate) total: Exact,
}

/// The page's arithmetic on a day's totals: `bought` and `sold` in paisa, the
/// brokerage already summed over the executed orders, and whether a sell was
/// executed (a delivery or MTF sell pays DP). [`quote`] is this on one buy
/// and one sell; `crate::ledger::DayLedger` is this on any number of orders.
///
/// # Errors
///
/// What [`page_rates`] refuses.
pub(crate) fn page_lines(
    kind: TradeKind,
    exchange: Exchange,
    bought: Paisa,
    sold: Paisa,
    brokerage: Exact,
    sell_executed: bool,
) -> Result<PageLines, ChargeError> {
    let rates = page_rates(kind, exchange)?;
    let bought = i128::from(bought.raw());
    let sold = i128::from(sold.raw());
    let traded = bought + sold;
    let brokerage = brokerage.0;
    // Every line is an i64 value (or the sum of two) times a constant of at
    // most 15,000, or a ₹14.75 constant, so each is below 2^79; the brokerage
    // is a sum of per-order figures each at most ₹20, so even 2^63 orders of
    // it stay below 2^98. Their sum is far inside i128: no step here can
    // overflow, and none is guarded.
    let exchange_charge = traded * rates.exchange;
    let sebi = traded * SEBI;
    let stt = (if rates.stt_on_turnover { traded } else { sold }) * rates.stt;
    let stamp = bought * rates.stamp;
    let dp = if rates.dp_on_sell && sell_executed {
        DP
    } else {
        0
    };
    let exchange_for_gst =
        half_up::<UNITS_PER_TEN_THOUSANDTH_RUPEE>(exchange_charge) * UNITS_PER_TEN_THOUSANDTH_RUPEE;
    let gst = half_up::<100>((brokerage + exchange_for_gst) * GST_PERCENT);
    let total = brokerage + exchange_charge + sebi + stt + stamp + gst + dp;
    Ok(PageLines {
        brokerage: Exact(brokerage),
        exchange_charge: Exact(exchange_charge),
        sebi: Exact(sebi),
        stt: Exact(stt),
        stamp: Exact(stamp),
        gst: Exact(gst),
        dp: Exact(dp),
        total: Exact(total),
    })
}

/// One order's own lines at the page's rates, unrounded: brokerage,
/// exchange, SEBI, STT, STT on exercise (none), stamp, GST, DP and GST on DP
/// (none: the page's DP line carries its GST). Linear in the order, so a
/// day's orders sum to the day's lines; GST here is 18% of the order's
/// brokerage and its UNROUNDED exchange line, because the page's 10⁻⁴ rupee
/// step on the exchange line is a step on the day's total, not an order's.
/// `crate::ledger::order_lines_unrounded` documents the rule.
///
/// # Errors
///
/// What [`page_rates`] refuses.
pub(crate) fn order_lines(
    kind: TradeKind,
    exchange: Exchange,
    value: Paisa,
    bought: Paisa,
    sold: Paisa,
    brokerage: Exact,
) -> Result<[Exact; 9], ChargeError> {
    let rates = page_rates(kind, exchange)?;
    let value = i128::from(value.raw());
    let exchange_charge = value * rates.exchange;
    let stt_base = if rates.stt_on_turnover {
        value
    } else {
        i128::from(sold.raw())
    };
    let stt = stt_base * rates.stt;
    let dp = if rates.dp_on_sell && sold.raw() > 0 {
        DP
    } else {
        0
    };
    Ok([
        brokerage,
        Exact(exchange_charge),
        Exact(value * SEBI),
        Exact(stt),
        Exact::ZERO,
        Exact(i128::from(bought.raw()) * rates.stamp),
        Exact(half_up::<100>(
            (brokerage.0 + exchange_charge) * GST_PERCENT,
        )),
        Exact(dp),
        Exact::ZERO,
    ])
}

/// A not-positive refusal naming the input.
const fn not_positive(quantity: &'static str, value: i64) -> ChargeError {
    ChargeError::Cost(CostError::NotPositive { quantity, value })
}

/// One side's value, if that side was executed.
fn side_value(
    quantity: i64,
    price: Option<Paisa>,
    name: &'static str,
) -> Result<Option<Paisa>, ChargeError> {
    let Some(price) = price else {
        return Ok(None);
    };
    if price.raw() <= 0 {
        return Err(not_positive(name, price.raw()));
    }
    quantity
        .checked_mul(price.raw())
        .map(|value| Some(Paisa::from_raw(value)))
        .ok_or(ChargeError::Cost(CostError::Overflow {
            operation: "calculator quantity x price",
        }))
}

/// The page's brokerage on one executed order, unrounded: nothing for a side
/// that was not executed.
fn order_brokerage(kind: TradeKind, value: Option<Paisa>) -> i128 {
    let Some(value) = value else {
        return 0;
    };
    match kind {
        // An ETF or a commodity is refused by `page_lines` right after this;
        // its brokerage is never shown.
        TradeKind::Equity(Product::Delivery) | TradeKind::Etf | TradeKind::Commodity => 0,
        TradeKind::Equity(Product::Intraday | Product::Mtf) => {
            (i128::from(value.raw()) * BROKERAGE_RATE).min(BROKERAGE_CAP)
        }
        TradeKind::Futures | TradeKind::Options => FNO_BROKERAGE,
    }
}

impl DhanQuote {
    /// The kind quoted.
    #[must_use]
    pub const fn kind(&self) -> TradeKind {
        self.kind
    }
    /// The venue quoted.
    #[must_use]
    pub const fn exchange(&self) -> Exchange {
        self.exchange
    }
    /// Units.
    #[must_use]
    pub const fn quantity(&self) -> i64 {
        self.quantity
    }
    /// Buy value, if a buy was executed.
    #[must_use]
    pub const fn buy_value(&self) -> Option<Paisa> {
        self.buy_value
    }
    /// Sell value, if a sell was executed.
    #[must_use]
    pub const fn sell_value(&self) -> Option<Paisa> {
        self.sell_value
    }
    /// Buy value + sell value.
    #[must_use]
    pub const fn turnover(&self) -> Paisa {
        self.turnover
    }

    /// Every line, unrounded, in the page's order: brokerage, exchange, SEBI,
    /// STT, stamp, GST, DP.
    #[must_use]
    pub const fn exact_lines(&self) -> [(&'static str, Exact); 7] {
        [
            ("BROKERAGE", self.brokerage),
            ("EXCHANGE_CHARGES", self.exchange_charge),
            ("SEBI_CHARGES", self.sebi),
            ("STT_CHARGES", self.stt),
            ("STAMP_DUTY", self.stamp),
            ("GST_CHARGES", self.gst),
            ("DP_CHARGES", self.dp),
        ]
    }

    /// Brokerage, as shown.
    ///
    /// # Errors
    ///
    /// [`ChargeError::Cost`] carrying an overflow.
    pub fn brokerage(&self) -> Result<Paisa, ChargeError> {
        self.brokerage.to_paisa()
    }
    /// Exchange transaction charge, IPFT included, as shown.
    ///
    /// # Errors
    ///
    /// [`ChargeError::Cost`] carrying an overflow.
    pub fn exchange_charge(&self) -> Result<Paisa, ChargeError> {
        self.exchange_charge.to_paisa()
    }
    /// SEBI fee, as shown.
    ///
    /// # Errors
    ///
    /// [`ChargeError::Cost`] carrying an overflow.
    pub fn sebi(&self) -> Result<Paisa, ChargeError> {
        self.sebi.to_paisa()
    }
    /// STT, as shown: half-up to the rupee.
    ///
    /// # Errors
    ///
    /// [`ChargeError::Cost`] carrying an overflow.
    pub fn stt(&self) -> Result<Paisa, ChargeError> {
        self.stt.to_whole_rupee()
    }
    /// Stamp duty, as shown: half-up to the rupee.
    ///
    /// # Errors
    ///
    /// [`ChargeError::Cost`] carrying an overflow.
    pub fn stamp(&self) -> Result<Paisa, ChargeError> {
        self.stamp.to_whole_rupee()
    }
    /// GST, as shown.
    ///
    /// # Errors
    ///
    /// [`ChargeError::Cost`] carrying an overflow.
    pub fn gst(&self) -> Result<Paisa, ChargeError> {
        self.gst.to_paisa()
    }
    /// DP charge, as shown.
    ///
    /// # Errors
    ///
    /// [`ChargeError::Cost`] carrying an overflow.
    pub fn dp(&self) -> Result<Paisa, ChargeError> {
        self.dp.to_paisa()
    }

    /// The total the page shows: the exact sum of the unrounded lines, half-up
    /// to the paisa. Within one paisa of the page on every case measured;
    /// the page's own last-paisa rule is UNVERIFIED.
    ///
    /// # Errors
    ///
    /// [`ChargeError::Cost`] carrying an overflow.
    pub fn total(&self) -> Result<Paisa, ChargeError> {
        self.total.to_paisa()
    }

    /// The total, unrounded.
    #[must_use]
    pub const fn exact_total(&self) -> Exact {
        self.total
    }

    /// `(sell − buy) × quantity − total`.
    ///
    /// # Errors
    ///
    /// * [`ChargeError::OneSided`] for a buy-only or sell-only quote, which
    ///   Dhan's page shows as a zero P&L. A one-sided order has no P&L.
    /// * [`ChargeError::Cost`] carrying an overflow.
    pub fn net_pnl(&self) -> Result<Paisa, ChargeError> {
        let (Some(bought), Some(sold)) = (self.buy_value, self.sell_value) else {
            return Err(ChargeError::OneSided { figure: "net P&L" });
        };
        let total = self.total()?;
        sold.raw()
            .checked_sub(bought.raw())
            .and_then(|gross| gross.checked_sub(total.raw()))
            .map(Paisa::from_raw)
            .ok_or(ChargeError::Cost(CostError::Overflow {
                operation: "the calculator net P&L",
            }))
    }

    /// The price move per unit that pays the charges: `total ÷ quantity`,
    /// half-up to the paisa. The page's rounding of this figure is
    /// UNVERIFIED. A quantity of zero, which would make it undefined, never
    /// reaches here: [`quote`] refuses it by name.
    ///
    /// # Errors
    ///
    /// [`ChargeError::Cost`] carrying an overflow.
    pub fn points_to_breakeven(&self) -> Result<Paisa, ChargeError> {
        let total = self.total()?.raw();
        // `quantity` is at least 1 -- `quote` refuses anything else -- so the
        // division is total, the remainder is below it, and comparing the
        // remainder with what is left of the divisor is the half-up test
        // without doubling anything that could overflow.
        let quotient = total.div_euclid(self.quantity);
        let remainder = total.rem_euclid(self.quantity);
        Ok(Paisa::from_raw(if remainder >= self.quantity - remainder {
            quotient + 1
        } else {
            quotient
        }))
    }
}

impl fmt::Display for DhanQuote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{DHAN_LABEL} — {} on {}, {} units:",
            self.kind,
            self.exchange.as_str(),
            self.quantity
        )?;
        match self.buy_value {
            Some(value) => write!(f, " bought {}", Rupees(value))?,
            None => f.write_str(" no buy")?,
        }
        match self.sell_value {
            Some(value) => write!(f, ", sold {}", Rupees(value))?,
            None => f.write_str(", no sell")?,
        }
        for (name, line) in [
            ("brokerage", self.brokerage.to_paisa()),
            ("exchange", self.exchange_charge.to_paisa()),
            ("SEBI", self.sebi.to_paisa()),
            ("STT", self.stt.to_whole_rupee()),
            ("stamp", self.stamp.to_whole_rupee()),
            ("GST", self.gst.to_paisa()),
            ("DP", self.dp.to_paisa()),
        ] {
            match line {
                Ok(amount) => write!(f, "\n  {name}: {}", Rupees(amount))?,
                Err(_) => write!(f, "\n  {name}: refused (overflow)")?,
            }
        }
        match self.total() {
            Ok(amount) => write!(f, "\n  total: {}", Rupees(amount)),
            Err(_) => f.write_str("\n  total: refused (overflow)"),
        }
    }
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

    fn rs(raw: i64) -> Paisa {
        Paisa::from_raw(raw)
    }

    const INTRADAY: TradeKind = TradeKind::Equity(Product::Intraday);

    #[test]
    fn every_page_constant_is_the_figure_the_page_uses() {
        // In units of 10^-7 per paisa of base: 0.0003 is 3,000 units.
        assert_eq!(BROKERAGE_RATE, 3_000);
        assert_eq!(BROKERAGE_CAP, 20_000_000_000);
        assert_eq!(FNO_BROKERAGE, 20_000_000_000);
        assert_eq!(
            [
                EXCHANGE_NSE_EQUITY,
                EXCHANGE_BSE_EQUITY,
                EXCHANGE_FUTURES,
                EXCHANGE_OPTIONS
            ],
            [307, 380, 183, 3_553]
        );
        assert_eq!(SEBI, 10);
        assert_eq!(
            [STT_INTRADAY, STT_DELIVERY, STT_FUTURES, STT_OPTIONS],
            [2_500, 10_000, 5_000, 15_000]
        );
        assert_eq!(
            [STAMP_INTRADAY, STAMP_DELIVERY, STAMP_FUTURES, STAMP_OPTIONS],
            [300, 1_500, 200, 300]
        );
        assert_eq!(DP, 14_750_000_000);
        assert_eq!(GST_PERCENT, 18);
        assert_eq!(UNITS_PER_RUPEE, 1_000_000_000);
        assert_eq!(UNITS_PER_TEN_THOUSANDTH_RUPEE, 100_000);
        assert_eq!(DHAN_LABEL, "Dhan calculator (estimate)");
    }

    #[test]
    fn an_exact_line_rounds_half_up_to_the_paisa_and_to_the_rupee() {
        assert_eq!(Exact(4_681_750_000).to_paisa(), Ok(rs(468)));
        assert_eq!(Exact(5_000_000).to_paisa(), Ok(rs(1)));
        assert_eq!(Exact(4_999_999).to_paisa(), Ok(rs(0)));
        assert_eq!(Exact(152_500_000_000).to_whole_rupee(), Ok(rs(153_00)));
        assert_eq!(Exact(152_499_999_999).to_whole_rupee(), Ok(rs(152_00)));
        assert_eq!(Exact(0).to_whole_rupee(), Ok(Paisa::ZERO));
        assert_eq!(Exact(7).units(), 7);
        // Past i64 at either rounding: refused, never saturated.
        assert_eq!(
            Exact(i128::MAX).to_paisa(),
            Err(ChargeError::Cost(CostError::Overflow {
                operation: "a calculator line"
            }))
        );
        assert_eq!(
            Exact(i128::from(i64::MAX) * UNITS_PER_RUPEE).to_whole_rupee(),
            Err(ChargeError::Cost(CostError::Overflow {
                operation: "a whole-rupee calculator line"
            }))
        );
    }

    #[test]
    fn every_input_the_page_shows_as_zero_or_nan_is_refused_by_name() {
        let q = |quantity, buy: Option<i64>, sell: Option<i64>| {
            quote(INTRADAY, Exchange::Nse, quantity, buy.map(rs), sell.map(rs))
        };
        assert_eq!(
            q(0, Some(100_00), Some(101_00)),
            Err(not_positive("quantity", 0))
        );
        assert_eq!(q(-1, Some(100_00), None), Err(not_positive("quantity", -1)));
        assert_eq!(
            q(10, Some(0), Some(101_00)),
            Err(not_positive("buy price", 0))
        );
        assert_eq!(q(10, Some(-5), None), Err(not_positive("buy price", -5)));
        assert_eq!(
            q(10, Some(100_00), Some(0)),
            Err(not_positive("sell price", 0))
        );
        assert_eq!(q(10, None, Some(-1)), Err(not_positive("sell price", -1)));
        assert_eq!(q(10, None, None), Err(ChargeError::NoOrder));
        for kind in [TradeKind::Futures, TradeKind::Options] {
            assert_eq!(
                quote(kind, Exchange::Bse, 10, Some(rs(100_00)), None),
                Err(ChargeError::NotOnPage {
                    what: "BSE futures and options"
                })
            );
        }
        // No calculator reading exists for an ETF, and MCX is outside the
        // engine's scope: both refused by name on either venue.
        for exchange in [Exchange::Nse, Exchange::Bse] {
            assert_eq!(
                quote(TradeKind::Etf, exchange, 10, Some(rs(100_00)), None),
                Err(ChargeError::NotOnPage {
                    what: "an ETF (no calculator reading exists)"
                })
            );
            assert_eq!(
                quote(TradeKind::Commodity, exchange, 10, None, Some(rs(100_00))),
                Err(ChargeError::NotOnPage {
                    what: "a commodity (MCX is outside the engine's scope)"
                })
            );
        }
    }

    #[test]
    fn a_value_or_a_turnover_past_i64_is_refused_by_name() {
        let overflow = |operation| Some(ChargeError::Cost(CostError::Overflow { operation }));
        assert_eq!(
            quote(INTRADAY, Exchange::Nse, i64::MAX, Some(rs(2)), None).err(),
            overflow("calculator quantity x price")
        );
        assert_eq!(
            quote(INTRADAY, Exchange::Nse, 1, Some(rs(i64::MAX)), Some(rs(1))).err(),
            overflow("the calculator turnover")
        );
        // The net P&L of a short whose loss leaves i64, and of a gross that
        // the charges then push past it.
        let wide = quote(INTRADAY, Exchange::Nse, 1, Some(rs(i64::MAX)), None).expect("in range");
        let mut short = wide;
        short.sell_value = Some(rs(-2));
        assert_eq!(short.net_pnl().err(), overflow("the calculator net P&L"));
        let mut charged = wide;
        charged.buy_value = Some(rs(0));
        charged.sell_value = Some(rs(i64::MIN + 1));
        assert_eq!(charged.net_pnl().err(), overflow("the calculator net P&L"));
    }

    #[test]
    fn a_one_sided_quote_has_charges_but_no_net_pnl() {
        let buy_only =
            quote(INTRADAY, Exchange::Nse, 50, Some(rs(1_520_00)), None).expect("priced");
        assert_eq!(buy_only.brokerage(), Ok(rs(20_00)));
        assert_eq!(buy_only.stt(), Ok(Paisa::ZERO));
        assert_eq!(buy_only.sell_value(), None);
        assert_eq!(
            buy_only.net_pnl(),
            Err(ChargeError::OneSided { figure: "net P&L" })
        );
        let sell_only =
            quote(INTRADAY, Exchange::Nse, 50, None, Some(rs(1_530_00))).expect("priced");
        assert_eq!(sell_only.stamp(), Ok(Paisa::ZERO));
        assert_eq!(sell_only.stt(), Ok(rs(19_00)));
        assert_eq!(sell_only.buy_value(), None);
        assert_eq!(
            sell_only.net_pnl(),
            Err(ChargeError::OneSided { figure: "net P&L" })
        );
    }

    #[test]
    fn the_net_and_the_breakeven_follow_from_the_total() {
        let v1 = quote(
            INTRADAY,
            Exchange::Nse,
            50,
            Some(rs(1_520_00)),
            Some(rs(1_530_00)),
        )
        .expect("priced");
        assert_eq!(v1.total(), Ok(rs(74_28)));
        // 40 + 4.68175 + 0.1525 + 19.125 + 2.28 + 8.042724 = ₹74.281974, exactly.
        assert_eq!(v1.exact_total().units(), 74_281_974_000);
        // Gross ₹500 less ₹74.28.
        assert_eq!(v1.net_pnl(), Ok(rs(425_72)));
        // ₹74.28 over 50 units: ₹1.4856 -> ₹1.49.
        assert_eq!(v1.points_to_breakeven(), Ok(rs(1_49)));
        assert_eq!(v1.kind(), INTRADAY);
        assert_eq!(v1.exchange(), Exchange::Nse);
        assert_eq!(v1.quantity(), 50);
        assert_eq!(v1.turnover(), rs(152_500_00));
        // A loss: SBI 50 @800 -> 792 loses ₹400 gross and ₹42.24 in charges.
        let sbi = quote(
            INTRADAY,
            Exchange::Nse,
            50,
            Some(rs(800_00)),
            Some(rs(792_00)),
        )
        .expect("priced");
        assert_eq!(sbi.net_pnl(), Ok(rs(-442_24)));
        // The widest total still has a breakeven: a quotient never grows.
        let mut wide = v1;
        wide.total = Exact(i128::from(i64::MAX) * UNITS_PER_PAISA);
        wide.quantity = 1;
        assert_eq!(wide.points_to_breakeven(), Ok(rs(i64::MAX)));
        // A remainder of exactly half the quantity goes up; less goes down.
        let mut half = v1;
        half.total = Exact(5 * UNITS_PER_PAISA);
        half.quantity = 2;
        assert_eq!(half.points_to_breakeven(), Ok(rs(3)));
        half.total = Exact(7 * UNITS_PER_PAISA);
        half.quantity = 3;
        assert_eq!(half.points_to_breakeven(), Ok(rs(2)));
        half.total = Exact(8 * UNITS_PER_PAISA);
        assert_eq!(half.points_to_breakeven(), Ok(rs(3)));
        // A total too wide to show refuses every figure built on it.
        let mut past = v1;
        past.total = Exact(i128::MAX);
        let refused = Err(ChargeError::Cost(CostError::Overflow {
            operation: "a calculator line",
        }));
        assert_eq!(past.total(), refused);
        assert_eq!(past.net_pnl(), refused);
        assert_eq!(past.points_to_breakeven(), refused);
        assert!(past.to_string().ends_with("total: refused (overflow)"));
    }

    #[test]
    fn the_quote_renders_its_label_its_inputs_and_every_line() {
        let v1 = quote(
            INTRADAY,
            Exchange::Nse,
            50,
            Some(rs(1_520_00)),
            Some(rs(1_530_00)),
        )
        .expect("priced");
        assert_eq!(
            v1.to_string(),
            "Dhan calculator (estimate) — equity intraday on NSE, 50 units: bought ₹76000.00, \
             sold ₹76500.00\n  brokerage: ₹40.00\n  exchange: ₹4.68\n  SEBI: ₹0.15\n  \
             STT: ₹19.00\n  stamp: ₹2.00\n  GST: ₹8.04\n  DP: ₹0.00\n  total: ₹74.28"
        );
        let buy_only =
            quote(INTRADAY, Exchange::Nse, 50, Some(rs(1_520_00)), None).expect("priced");
        assert!(buy_only.to_string().contains("bought ₹76000.00, no sell"));
        let sell_only =
            quote(INTRADAY, Exchange::Nse, 50, None, Some(rs(1_530_00))).expect("priced");
        assert!(
            sell_only
                .to_string()
                .contains("units: no buy, sold ₹76500.00")
        );
        // A line too wide to show is named as refused rather than printed.
        let mut wide = v1;
        wide.stt = Exact(i128::MAX);
        assert!(wide.to_string().contains("STT: refused (overflow)"));
        let names: Vec<&str> = v1.exact_lines().iter().map(|(name, _)| *name).collect();
        assert_eq!(
            names,
            [
                "BROKERAGE",
                "EXCHANGE_CHARGES",
                "SEBI_CHARGES",
                "STT_CHARGES",
                "STAMP_DUTY",
                "GST_CHARGES",
                "DP_CHARGES"
            ]
        );
    }

    /// A writer that succeeds `budget` times and then fails for good.
    struct FailsAfter {
        budget: usize,
        written: usize,
    }

    impl fmt::Write for FailsAfter {
        fn write_str(&mut self, _: &str) -> fmt::Result {
            if self.written < self.budget {
                self.written += 1;
                Ok(())
            } else {
                Err(fmt::Error)
            }
        }
    }

    /// How many budgets leave `value` unrendered: the writes it needs.
    fn writes_needed(value: &impl fmt::Display) -> usize {
        use fmt::Write as _;
        (0..256)
            .filter(|&budget| {
                let mut writer = FailsAfter { budget, written: 0 };
                write!(writer, "{value}").is_err()
            })
            .count()
    }

    #[test]
    fn display_propagates_a_writer_error_from_every_part_it_writes() {
        let both = quote(
            INTRADAY,
            Exchange::Nse,
            50,
            Some(rs(1_520_00)),
            Some(rs(1_530_00)),
        )
        .expect("priced");
        let buy_only =
            quote(INTRADAY, Exchange::Nse, 50, Some(rs(1_520_00)), None).expect("priced");
        let sell_only =
            quote(INTRADAY, Exchange::Nse, 50, None, Some(rs(1_530_00))).expect("priced");
        let mut refused_line = both;
        refused_line.stt = Exact(i128::MAX);
        assert_eq!(
            [
                writes_needed(&both),
                writes_needed(&buy_only),
                writes_needed(&sell_only),
                writes_needed(&refused_line),
            ],
            // A missing side writes one literal where a present one writes
            // two parts and a value; a refused line writes its literal alone.
            [89, 84, 84, 83]
        );
    }
}
