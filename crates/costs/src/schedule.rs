//! What is being priced, which rates price it, and where each rate came from.
//!
//! Package CHG-1 prices four things this crate could not price before: equity
//! cash in three products, index and stock futures, options under the
//! contract-note method, and the broker's own fees. Every rate it uses lives
//! here, as a [`Cited`] — the exact [`Ratio`], the source it came from, and
//! whether that source is a named circular or page ([`Status::Cited`]) or a
//! figure the brief supplied without one ([`Status::Unverified`]).
//!
//! # Two rate sets, chosen explicitly
//!
//! [`RateSelector::CurrentOnEveryDate`] is **the default for backtests**, by
//! the operator's choice: today's rates (read on [`CURRENT_AS_OF`]) applied to
//! every trade date, back to 2018 and beyond. Every output priced this way is
//! labelled `today's rates applied to <date>` ([`RateBasis`]), so no reader
//! can take a 2019 figure for a 2019 rate.
//!
//! [`RateSelector::Dated`] is the existing dated regime lookup in
//! [`crate::regime`], **unchanged**. It covers index options only, because
//! options are the only kind this repository holds a dated history for. A
//! dated equity or futures card is a [`crate::error::Refusal`] — not today's
//! rate applied backwards without a label, which is the one thing the dated
//! selector exists not to do.
//!
//! # Lookup cost
//!
//! [`RateCard::resolve`] is one `match` on the kind and the exchange, plus —
//! under [`RateSelector::Dated`] — the regime's compile-time-bounded walks.
//! Nothing here depends on a magnitude or on how many trades came before.

use std::fmt;

use brutex_core::instrument::Exchange;
use brutex_core::price::Paisa;

use crate::charge_error::ChargeError;
use crate::dated::boundary;
use crate::day::TradeDay;
use crate::error::Refusal;
use crate::exact::{Quantum, Ratio};
use crate::rate::{GST_ON_FEE_BASE, RATE_SCALE, SEBI_TURNOVER_FEE, STAMP_DUTY_BUY_SIDE, ipft};
use crate::regime::{exchange_charge_rate, stt_exercise_rate, stt_options_rate};

// ---------------------------------------------------------------------------
// What is being priced
// ---------------------------------------------------------------------------

/// The equity product an order was placed under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Product {
    /// Intraday: squared off the same day.
    Intraday,
    /// Delivery (cash-and-carry): held in the demat account.
    Delivery,
    /// Margin trading facility: delivery funded by the broker.
    Mtf,
}

/// What kind of trade a charge stack prices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TradeKind {
    /// A cash-market equity trade, in one of three products.
    Equity(Product),
    /// A futures contract.
    Futures,
    /// An options contract. Value is premium × quantity.
    Options,
}

impl TradeKind {
    /// The kind's name, as a refusal spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Equity(Product::Intraday) => "equity intraday",
            Self::Equity(Product::Delivery) => "equity delivery",
            Self::Equity(Product::Mtf) => "equity MTF",
            Self::Futures => "futures",
            Self::Options => "options",
        }
    }

    /// Whether a position of this kind can be squared off by the broker at
    /// the intraday ceiling. A delivery or MTF holding is carried, not
    /// squared off, so the ceiling fee cannot apply to it.
    #[must_use]
    pub const fn can_be_squared_off_by_broker(self) -> bool {
        !matches!(self, Self::Equity(Product::Delivery | Product::Mtf))
    }
}

impl fmt::Display for TradeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// A rate and its source
// ---------------------------------------------------------------------------

/// Whether a rate's figure traces to a named circular or page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Status {
    /// The figure is attributed to a named circular, notice, statute or
    /// broker page, recorded in `docs/00-charter.md`.
    Cited,
    /// The figure was supplied without a source this repository can name, or
    /// one of its parts is a zero nobody verified. It is priced, because the
    /// operator ordered it, and every output that uses it says so.
    Unverified,
}

/// One rate, exactly, beside the source it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cited {
    rate: Ratio,
    source: &'static str,
    status: Status,
}

impl Cited {
    /// A rate a named source supports.
    const fn named(num: i64, den: i64, source: &'static str) -> Self {
        Self {
            rate: Ratio::new(num, den),
            source,
            status: Status::Cited,
        }
    }

    /// A rate no named source supports. Priced, and flagged.
    const fn unverified(num: i64, den: i64, source: &'static str) -> Self {
        Self {
            rate: Ratio::new(num, den),
            source,
            status: Status::Unverified,
        }
    }

    /// The exact rate.
    #[must_use]
    pub const fn rate(self) -> Ratio {
        self.rate
    }

    /// The source, or the honest account of its absence.
    #[must_use]
    pub const fn source(self) -> &'static str {
        self.source
    }

    /// Whether the source is a named one.
    #[must_use]
    pub const fn status(self) -> Status {
        self.status
    }

    /// Whether this rate is well-shaped and carries a source. `const`, so
    /// every shipped rate is checked by the compiler.
    const fn is_well_shaped(self) -> bool {
        self.rate.is_well_shaped() && crate::dated::has_content(self.source)
    }
}

// ---------------------------------------------------------------------------
// The dates the rate sets turn on
// ---------------------------------------------------------------------------

/// The day today's rates were read: Dhan's pricing page and calculator were
/// read on 15 and 16 Oct 2026, and the brief that carried the statutory
/// figures is dated 16 Oct 2026.
pub const CURRENT_AS_OF: TradeDay = boundary!(2026 - 10 - 16);

/// NSE circular `FA73061`'s effective date: from 2026-03-01 the investor
/// protection fund contribution is ₹0.01 per crore, folded into the dated
/// options row package CHG-0 adds to `crate::regime`.
pub const FA73061_EFFECTIVE: TradeDay = boundary!(2026 - 3 - 1);

// ---------------------------------------------------------------------------
// Today's rates — statutory
// ---------------------------------------------------------------------------

/// One crore rupees, in rupees: the unit every circular quotes in. A rate of
/// `₹x.yz per crore` is `xyz / (100 × CRORE)`.
const CRORE: i64 = 10_000_000;

/// NSE cash market: transaction charge ₹306.99 + IPFT ₹0.01 per crore, each
/// side — ₹307.00 per crore, rounded **once** as one line.
pub const NSE_EQUITY_EXCHANGE: Cited = Cited::named(
    30_700,
    100 * CRORE,
    "NSE circular FA73061 (effective 2026-03-01): cash market transaction charge Rs 306.99 \
     per crore + IPFT Rs 0.01 per crore, each side. Figures as the CHG-1 brief states them; \
     the circular body was not fetched by this session (docs/00-charter.md, CHG-1).",
);

/// NSE futures: transaction charge ₹182.99 + IPFT ₹0.01 per crore.
pub const NSE_FUTURES_EXCHANGE: Cited = Cited::named(
    18_300,
    100 * CRORE,
    "NSE circular FA73061 (effective 2026-03-01): futures transaction charge Rs 182.99 per \
     crore + IPFT Rs 0.01 per crore, each side (docs/00-charter.md, CHG-1).",
);

/// NSE options: transaction charge ₹3,552.99 + IPFT ₹0.01 per crore of
/// premium — 0.03553%. Carried here, in today's table, **only** because it
/// names `FA73061`; the dated path reads `crate::regime` instead.
pub const NSE_OPTIONS_EXCHANGE: Cited = Cited::named(
    355_300,
    100 * CRORE,
    "NSE circular FA73061 (effective 2026-03-01): options transaction charge Rs 3,552.99 per \
     crore of premium + IPFT Rs 0.01 per crore, each side — 0.03553% (docs/00-charter.md, CHG-1).",
);

/// BSE cash market: ₹375 per crore all-in. **UNVERIFIED as a circular**;
/// Dhan's calculator uses 0.000038.
pub const BSE_EQUITY_EXCHANGE: Cited = Cited::unverified(
    375,
    CRORE,
    "UNVERIFIED — BSE cash market Rs 375 per crore all-in, as the CHG-1 brief states it; no BSE \
     notice is recorded for it. Dhan's calculator uses 0.000038 (Rs 380 per crore).",
);

/// BSE SENSEX/Bankex options: ₹3,250 per crore of premium (BSE notice of
/// 27-Sep-2024, `crate::regime::BSE_EXCHANGE_CHARGE`) plus a BSE IPFT taken as
/// zero — and that zero is UNVERIFIED (`crate::rate::BSE_IPFT`).
pub const BSE_OPTIONS_EXCHANGE: Cited = Cited::unverified(
    3_250,
    CRORE,
    "BSE notice of 27-Sep-2024 (crate::regime::BSE_EXCHANGE_CHARGE, Rs 3,250 per crore of \
     premium) plus BSE IPFT taken as ZERO — that zero is UNVERIFIED (crate::rate::BSE_IPFT).",
);

/// SEBI turnover fee: ₹10 per crore, each side.
pub const SEBI_FEE: Cited = Cited::named(
    10,
    CRORE,
    "SEBI turnover fee Rs 10 per crore, each side — the sebi.gov.in fee schedule \
     (crate::rate::SEBI_TURNOVER_FEE, COSTS_VERIFIED section 4).",
);

/// GST: 18% on brokerage, the exchange charge (IPFT included) and the SEBI fee.
pub const GST: Cited = Cited::named(
    18,
    100,
    "GST 18% on brokerage + exchange transaction charge + IPFT + SEBI fee \
     (crate::rate::GST_ON_FEE_BASE, COSTS_VERIFIED).",
);

/// Equity intraday STT: 0.025% of the non-delivery value, sell side, per
/// client per security per day. The value is the non-delivery quantity at the
/// day's volume-weighted average price of **all** that client's trades in the
/// security — the NSE basis.
pub const STT_EQUITY_NON_DELIVERY: Cited = Cited::named(
    25,
    100_000,
    "STT 0.025% of the non-delivery value (non-delivery quantity x the day's VWAP of all the \
     client's trades in the security), per client per security per day, on the NSE basis — NSE \
     circular NSE/CMTR/5588 as the CHG-1 brief cites it (docs/00-charter.md, CHG-1).",
);

/// Equity delivery and MTF STT: 0.1% of the value, buy and sell.
pub const STT_EQUITY_DELIVERY: Cited = Cited::unverified(
    1,
    1_000,
    "UNVERIFIED — STT 0.1% of delivery value on both sides, as the CHG-1 brief states it; no \
     Finance Act section or NSE circular is recorded for the figure.",
);

/// Futures STT: 0.05% of the sell value.
pub const STT_FUTURES: Cited = Cited::unverified(
    5,
    10_000,
    "UNVERIFIED — STT 0.05% of the futures sell value, as the CHG-1 brief states it; no Finance \
     Act section is recorded for the futures figure (the options row is DEC-COST-003).",
);

/// Options STT: 0.15% of the sell premium. The same figure as the dated
/// regime's row from 2026-04-01 — `the_current_option_taxes_are_the_regimes_rows_on_the_as_of_day`
/// holds the two together.
pub const STT_OPTIONS: Cited = Cited::named(
    15,
    10_000,
    "STT 0.15% of the options sell premium — Finance Bill 2026 (DEC-COST-003), the \
     crate::regime row from 2026-04-01.",
);

/// STT on exercise: 0.15% of the intrinsic value. The regime's row from
/// 2026-04-01, cited there as Finance Act 2026 (DEC-COST-004).
pub const STT_EXERCISE: Cited = Cited::named(
    15,
    10_000,
    "STT on exercise 0.15% of intrinsic value — Finance Act 2026 (DEC-COST-004), the \
     crate::regime row from 2026-04-01.",
);

/// Equity intraday stamp duty: 0.003% of the FIFO buy value of the
/// non-deliverable quantity.
pub const STAMP_EQUITY_NON_DELIVERY: Cited = Cited::named(
    3,
    100_000,
    "Stamp duty 0.003% of the FIFO buy value of the non-deliverable quantity — NSE Clearing \
     circular NCL/CMPT/43115 as the CHG-1 brief cites it (docs/00-charter.md, CHG-1).",
);

/// Equity delivery and MTF stamp duty: 0.015% of the buy value.
pub const STAMP_EQUITY_DELIVERY: Cited = Cited::named(
    15,
    100_000,
    "Stamp duty 0.015% of the delivery buy value — NCL/CMPT/43115 as the CHG-1 brief cites it.",
);

/// Futures stamp duty: 0.002% of the buy value.
pub const STAMP_FUTURES: Cited = Cited::named(
    2,
    100_000,
    "Stamp duty 0.002% of the futures buy value — NCL/CMPT/43115 as the CHG-1 brief cites it.",
);

/// Options stamp duty: 0.003% of the buy premium.
pub const STAMP_OPTIONS: Cited = Cited::named(
    3,
    100_000,
    "Stamp duty 0.003% of the options buy premium — NCL/CMPT/43115; the same figure as \
     crate::rate::STAMP_DUTY_BUY_SIDE (COSTS_VERIFIED section 6).",
);

// ---------------------------------------------------------------------------
// Today's rates — Dhan's own fees (https://dhan.co/pricing/, read 15-16 Oct 2026)
// ---------------------------------------------------------------------------

/// Dhan equity intraday and MTF brokerage: the **lower** of ₹20 and 0.03% of
/// the order value, per executed order. This is the 0.03% half.
pub const DHAN_EQUITY_BROKERAGE_RATE: Cited = Cited::named(
    3,
    10_000,
    "Dhan pricing page https://dhan.co/pricing/ (read 15-16 Oct 2026): equity intraday and MTF \
     brokerage is the lower of Rs 20 and 0.03% of the order value, per executed order.",
);

/// The ₹20 cap on Dhan's equity intraday and MTF brokerage, per order.
pub const DHAN_EQUITY_BROKERAGE_CAP: Paisa = Paisa::from_raw(2_000);

/// Dhan F&O brokerage: ₹20 per executed order, flat.
pub const DHAN_FNO_BROKERAGE: Paisa = Paisa::from_raw(2_000);

/// Dhan DP charge: ₹12.50 per delivery or MTF sell day, plus GST.
pub const DHAN_DP_CHARGE: Paisa = Paisa::from_raw(1_250);

/// Dhan's square-off fee for a position still open at the intraday ceiling:
/// ₹20 per order, plus GST — a separate line, charged only when the caller
/// says the exit was the broker's.
pub const DHAN_SQUARE_OFF_FEE: Paisa = Paisa::from_raw(2_000);

/// The brokerage Dhan charges on one executed order of `kind` worth
/// `order_value`, rounded half-up to the paisa.
///
/// Equity delivery is free; equity intraday and MTF pay the lower of ₹20 and
/// 0.03%; futures and options pay ₹20 flat.
///
/// # Errors
///
/// [`ChargeError::Cost`] carrying an overflow, which a real order value
/// cannot reach.
///
/// # Examples
///
/// ```
/// use brutex_core::price::Paisa;
/// use costs::schedule::{Product, TradeKind, dhan_brokerage};
///
/// let small = dhan_brokerage(TradeKind::Equity(Product::Intraday), Paisa::from_raw(990_00))?;
/// assert_eq!(small.raw(), 30); // 0.03% of ₹990 is 29.7 paisa
/// let large = dhan_brokerage(TradeKind::Equity(Product::Mtf), Paisa::from_raw(76_000_00))?;
/// assert_eq!(large.raw(), 20_00); // capped
/// # Ok::<(), costs::charge_error::ChargeError>(())
/// ```
pub fn dhan_brokerage(kind: TradeKind, order_value: Paisa) -> Result<Paisa, ChargeError> {
    match kind {
        TradeKind::Equity(Product::Delivery) => Ok(Paisa::ZERO),
        TradeKind::Equity(Product::Intraday | Product::Mtf) => {
            let percent =
                DHAN_EQUITY_BROKERAGE_RATE
                    .rate()
                    .apply(order_value, 1, Quantum::Paisa)?;
            Ok(if percent.raw() < DHAN_EQUITY_BROKERAGE_CAP.raw() {
                percent
            } else {
                DHAN_EQUITY_BROKERAGE_CAP
            })
        }
        TradeKind::Futures | TradeKind::Options => Ok(DHAN_FNO_BROKERAGE),
    }
}

// Every shipped rate is well-shaped and carries its source. Not tests: a
// malformed row does not compile.
const _: () = assert!(NSE_EQUITY_EXCHANGE.is_well_shaped());
const _: () = assert!(NSE_FUTURES_EXCHANGE.is_well_shaped());
const _: () = assert!(NSE_OPTIONS_EXCHANGE.is_well_shaped());
const _: () = assert!(BSE_EQUITY_EXCHANGE.is_well_shaped());
const _: () = assert!(BSE_OPTIONS_EXCHANGE.is_well_shaped());
const _: () = assert!(SEBI_FEE.is_well_shaped());
const _: () = assert!(GST.is_well_shaped());
const _: () = assert!(STT_EQUITY_NON_DELIVERY.is_well_shaped());
const _: () = assert!(STT_EQUITY_DELIVERY.is_well_shaped());
const _: () = assert!(STT_FUTURES.is_well_shaped());
const _: () = assert!(STT_OPTIONS.is_well_shaped());
const _: () = assert!(STT_EXERCISE.is_well_shaped());
const _: () = assert!(STAMP_EQUITY_NON_DELIVERY.is_well_shaped());
const _: () = assert!(STAMP_EQUITY_DELIVERY.is_well_shaped());
const _: () = assert!(STAMP_FUTURES.is_well_shaped());
const _: () = assert!(STAMP_OPTIONS.is_well_shaped());
const _: () = assert!(DHAN_EQUITY_BROKERAGE_RATE.is_well_shaped());
// The flat figures this table shares with crate::rate are one number, not two.
const _: () = assert!(SEBI_TURNOVER_FEE.get() == 10);
const _: () = assert!(STAMP_DUTY_BUY_SIDE.get() == 300);
const _: () = assert!(GST_ON_FEE_BASE.get() == 1_800_000);
const _: () = assert!(crate::regime::BSE_EXCHANGE_CHARGE.get() == 3_250);

// ---------------------------------------------------------------------------
// The selector, and the label it puts on every output
// ---------------------------------------------------------------------------

/// Which rate set prices a trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum RateSelector {
    /// Today's rates on every trade date — the operator's choice and the
    /// default for backtests. Labelled on every output.
    #[default]
    CurrentOnEveryDate,
    /// The rates in force on the trade date, from the dated regime tables.
    /// Index options only; anything else refuses.
    Dated,
}

/// The label every precise output carries: which rate set priced it, and
/// for which day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RateBasis {
    /// Today's rates (as of [`CURRENT_AS_OF`]) applied to the trade day.
    TodaysRatesAppliedTo(TradeDay),
    /// The rates in force on the trade day.
    InForceOn(TradeDay),
}

impl RateBasis {
    /// The trade day the label names.
    #[must_use]
    pub const fn day(self) -> TradeDay {
        match self {
            Self::TodaysRatesAppliedTo(day) | Self::InForceOn(day) => day,
        }
    }
}

impl fmt::Display for RateBasis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TodaysRatesAppliedTo(day) => write!(f, "today's rates applied to {day}"),
            Self::InForceOn(day) => write!(f, "rates in force on {day}"),
        }
    }
}

// ---------------------------------------------------------------------------
// The rate card
// ---------------------------------------------------------------------------

/// The taxes a kind of trade bears, and the rates they are levied at.
///
/// One variant per kind, so a futures stack cannot reach for a delivery rate
/// and an equity stack cannot reach for an exercise rate: the wrong rate is
/// not on the card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Taxes {
    /// Equity cash.
    Equity {
        /// The product the orders were placed under.
        product: Product,
        /// STT on the non-delivery value, sell side.
        stt_non_delivery: Cited,
        /// STT on delivery value, either side.
        stt_delivery: Cited,
        /// Stamp duty on the FIFO buy value of the non-deliverable quantity.
        stamp_non_delivery: Cited,
        /// Stamp duty on the delivery buy value.
        stamp_delivery: Cited,
    },
    /// Futures.
    Futures {
        /// STT on the sell value.
        stt_sell: Cited,
        /// Stamp duty on the buy value.
        stamp_buy: Cited,
    },
    /// Options.
    Options {
        /// STT on the sell premium.
        stt_sell: Cited,
        /// STT on the intrinsic value of an exercise.
        stt_exercise: Cited,
        /// Stamp duty on the buy premium.
        stamp_buy: Cited,
    },
}

/// Every rate one client-security-day is priced at, resolved once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RateCard {
    basis: RateBasis,
    exchange: Exchange,
    exchange_charge: Cited,
    sebi: Cited,
    gst: Cited,
    taxes: Taxes,
}

/// How many rates the widest card carries: the three every kind shares and
/// the four equity taxes.
pub const MAX_RATES_ON_A_CARD: usize = 7;

impl RateCard {
    /// The card for `kind` at `exchange` on `day`, from `selector`'s rate set.
    ///
    /// # Errors
    ///
    /// * [`ChargeError::Cost`] carrying a [`Refusal`] under
    ///   [`RateSelector::Dated`]: for equity and futures on every date (no
    ///   dated history is recorded), and for options wherever the regime
    ///   refuses (every day before 2024-10-01).
    /// * [`ChargeError::NoRate`] for BSE futures under today's rates — no BSE
    ///   futures figure is recorded at all.
    ///
    /// # Examples
    ///
    /// ```
    /// use brutex_core::instrument::Exchange;
    /// use costs::day::TradeDay;
    /// use costs::schedule::{Product, RateCard, RateSelector, TradeKind};
    ///
    /// let day = TradeDay::new(2019, 5, 3)?;
    /// let kind = TradeKind::Equity(Product::Intraday);
    /// let card = RateCard::resolve(kind, Exchange::Nse, day, RateSelector::default())?;
    /// assert_eq!(card.basis().to_string(), "today's rates applied to 2019-05-03");
    /// assert!(RateCard::resolve(kind, Exchange::Nse, day, RateSelector::Dated).is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn resolve(
        kind: TradeKind,
        exchange: Exchange,
        day: TradeDay,
        selector: RateSelector,
    ) -> Result<Self, ChargeError> {
        match selector {
            RateSelector::CurrentOnEveryDate => current_card(kind, exchange, day),
            RateSelector::Dated => dated_card(kind, exchange, day),
        }
    }

    /// The label: which rate set, applied to which day.
    #[must_use]
    pub const fn basis(&self) -> RateBasis {
        self.basis
    }

    /// The venue.
    #[must_use]
    pub const fn exchange(&self) -> Exchange {
        self.exchange
    }

    /// The kind this card prices, read off its taxes.
    #[must_use]
    pub const fn kind(&self) -> TradeKind {
        match self.taxes {
            Taxes::Equity { product, .. } => TradeKind::Equity(product),
            Taxes::Futures { .. } => TradeKind::Futures,
            Taxes::Options { .. } => TradeKind::Options,
        }
    }

    /// The exchange transaction charge, IPFT included, on turnover.
    #[must_use]
    pub const fn exchange_charge(&self) -> Cited {
        self.exchange_charge
    }

    /// The SEBI turnover fee.
    #[must_use]
    pub const fn sebi(&self) -> Cited {
        self.sebi
    }

    /// The GST rate.
    #[must_use]
    pub const fn gst(&self) -> Cited {
        self.gst
    }

    /// The taxes, by kind.
    #[must_use]
    pub const fn taxes(&self) -> Taxes {
        self.taxes
    }

    /// Every rate on the card, the shared three first. `None` pads a card
    /// with fewer than [`MAX_RATES_ON_A_CARD`] rates.
    #[must_use]
    pub const fn rates(&self) -> [Option<Cited>; MAX_RATES_ON_A_CARD] {
        let shared = [self.exchange_charge, self.sebi, self.gst];
        let [a, b, c] = shared;
        match self.taxes {
            Taxes::Equity {
                stt_non_delivery,
                stt_delivery,
                stamp_non_delivery,
                stamp_delivery,
                ..
            } => [
                Some(a),
                Some(b),
                Some(c),
                Some(stt_non_delivery),
                Some(stt_delivery),
                Some(stamp_non_delivery),
                Some(stamp_delivery),
            ],
            Taxes::Futures {
                stt_sell,
                stamp_buy,
            } => [
                Some(a),
                Some(b),
                Some(c),
                Some(stt_sell),
                Some(stamp_buy),
                None,
                None,
            ],
            Taxes::Options {
                stt_sell,
                stt_exercise,
                stamp_buy,
            } => [
                Some(a),
                Some(b),
                Some(c),
                Some(stt_sell),
                Some(stt_exercise),
                Some(stamp_buy),
                None,
            ],
        }
    }
}

/// Today's card: the constants above, labelled with the trade day.
fn current_card(
    kind: TradeKind,
    exchange: Exchange,
    day: TradeDay,
) -> Result<RateCard, ChargeError> {
    let (exchange_charge, taxes) = match (kind, exchange) {
        (TradeKind::Equity(product), Exchange::Nse) => (NSE_EQUITY_EXCHANGE, equity_taxes(product)),
        (TradeKind::Equity(product), Exchange::Bse) => (BSE_EQUITY_EXCHANGE, equity_taxes(product)),
        (TradeKind::Futures, Exchange::Nse) => (
            NSE_FUTURES_EXCHANGE,
            Taxes::Futures {
                stt_sell: STT_FUTURES,
                stamp_buy: STAMP_FUTURES,
            },
        ),
        (TradeKind::Futures, Exchange::Bse) => {
            return Err(ChargeError::NoRate {
                charge: "futures exchange transaction charge (BSE)",
                reason: "no BSE futures rate is recorded in this repository",
            });
        }
        (TradeKind::Options, Exchange::Nse) => (NSE_OPTIONS_EXCHANGE, current_option_taxes()),
        (TradeKind::Options, Exchange::Bse) => (BSE_OPTIONS_EXCHANGE, current_option_taxes()),
    };
    Ok(RateCard {
        basis: RateBasis::TodaysRatesAppliedTo(day),
        exchange,
        exchange_charge,
        sebi: SEBI_FEE,
        gst: GST,
        taxes,
    })
}

/// The equity taxes, which do not depend on the venue.
const fn equity_taxes(product: Product) -> Taxes {
    Taxes::Equity {
        product,
        stt_non_delivery: STT_EQUITY_NON_DELIVERY,
        stt_delivery: STT_EQUITY_DELIVERY,
        stamp_non_delivery: STAMP_EQUITY_NON_DELIVERY,
        stamp_delivery: STAMP_EQUITY_DELIVERY,
    }
}

/// Today's option taxes.
const fn current_option_taxes() -> Taxes {
    Taxes::Options {
        stt_sell: STT_OPTIONS,
        stt_exercise: STT_EXERCISE,
        stamp_buy: STAMP_OPTIONS,
    }
}

/// The source line every regime-derived rate carries.
const REGIME_ROW: &str = "crate::regime — the dated regime row in force on the trade day; its \
     citation is on the row (crates/costs/src/regime.rs).";

/// The source line a dated NSE options exchange charge carries.
const REGIME_EXCHANGE_NSE: &str = "crate::regime::exchange_charge_rate on the trade day, plus \
     crate::rate::NSE_IPFT before FA73061's effective date (2026-03-01); from that date the \
     regime row alone, which FA73061 states as transaction charge and IPFT together.";

/// The source line a dated BSE options exchange charge carries.
const REGIME_EXCHANGE_BSE: &str = "crate::regime::exchange_charge_rate on the trade day, plus \
     crate::rate::BSE_IPFT, a zero that is UNVERIFIED.";

/// What a dated equity or futures refusal says.
const NO_DATED_HISTORY: &str = "UNVERIFIED — this repository records no dated history of the \
     equity cash or futures charge rates. Today's rates are recorded (FA73061 and the CHG-1 \
     brief, docs/00-charter.md) and are applied to any date only under \
     RateSelector::CurrentOnEveryDate, labelled \"today's rates applied to <date>\".";

/// The remedy a dated equity or futures refusal names.
const NO_DATED_HISTORY_REMEDY: &str = "price under RateSelector::CurrentOnEveryDate (the default, \
     labelled), or source each superseded circular and add a dated table for this kind to \
     crates/costs/src/schedule.rs, with a docs/05-decisions.md entry";

/// The dated card: the regime tables for options, a refusal for the rest.
fn dated_card(kind: TradeKind, exchange: Exchange, day: TradeDay) -> Result<RateCard, ChargeError> {
    let charge = match kind {
        TradeKind::Options => return dated_option_card(exchange, day),
        TradeKind::Equity(_) => "equity cash charge stack (dated rates)",
        TradeKind::Futures => "futures charge stack (dated rates)",
    };
    Err(Refusal::with_remediation(
        charge,
        Some(exchange),
        day,
        TradeDay::MIN,
        None,
        NO_DATED_HISTORY,
        NO_DATED_HISTORY_REMEDY,
    )
    .into())
}

/// The dated options card: the regime's rows on `day`, the flat rates from
/// `crate::rate`, exactly as `crate::trip::Rates::resolve` reads them.
fn dated_option_card(exchange: Exchange, day: TradeDay) -> Result<RateCard, ChargeError> {
    let base = exchange_charge_rate(exchange, day)?;
    let stt_sell = stt_options_rate(day)?;
    let stt_exercise = stt_exercise_rate(day)?;
    // FA73061 folds the IPFT into the NSE row from its effective date; before
    // it, and on BSE always, the IPFT is its own figure added on top.
    let (ipft_bps, exchange_charge) = match exchange {
        Exchange::Nse if !day.before(FA73061_EFFECTIVE) => (0, Status::Cited),
        Exchange::Nse => (ipft(exchange).get(), Status::Cited),
        Exchange::Bse => (ipft(exchange).get(), Status::Unverified),
    };
    let source = match exchange {
        Exchange::Nse => REGIME_EXCHANGE_NSE,
        Exchange::Bse => REGIME_EXCHANGE_BSE,
    };
    Ok(RateCard {
        basis: RateBasis::InForceOn(day),
        exchange,
        exchange_charge: Cited {
            rate: Ratio::new(base.get() + ipft_bps, RATE_SCALE),
            source,
            status: exchange_charge,
        },
        sebi: Cited::named(SEBI_TURNOVER_FEE.get(), RATE_SCALE, SEBI_FEE.source),
        gst: Cited::named(GST_ON_FEE_BASE.get(), RATE_SCALE, GST.source),
        taxes: Taxes::Options {
            stt_sell: Cited::named(stt_sell.get(), RATE_SCALE, REGIME_ROW),
            stt_exercise: Cited::named(stt_exercise.get(), RATE_SCALE, REGIME_ROW),
            stamp_buy: Cited::named(STAMP_DUTY_BUY_SIDE.get(), RATE_SCALE, STAMP_OPTIONS.source),
        },
    })
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
    use crate::error::CostError;

    fn day(year: u16, month: u8, d: u8) -> TradeDay {
        TradeDay::new(year, month, d).expect("a real date")
    }

    fn paisa(raw: i64) -> Paisa {
        Paisa::from_raw(raw)
    }

    const ALL_KINDS: [TradeKind; 5] = [
        TradeKind::Equity(Product::Intraday),
        TradeKind::Equity(Product::Delivery),
        TradeKind::Equity(Product::Mtf),
        TradeKind::Futures,
        TradeKind::Options,
    ];

    /// One crore rupees in paisa, the unit the circulars quote in.
    const ONE_CRORE: Paisa = Paisa::from_raw(100 * CRORE);

    fn per_crore(rate: Cited) -> i64 {
        rate.rate()
            .apply(ONE_CRORE, 1, Quantum::Paisa)
            .expect("in range")
            .raw()
    }

    #[test]
    fn every_current_rate_is_the_figure_its_source_states_per_crore() {
        // Paisa per ₹1 crore, which is how the circulars quote them.
        assert_eq!(per_crore(NSE_EQUITY_EXCHANGE), 307_00);
        assert_eq!(per_crore(NSE_FUTURES_EXCHANGE), 183_00);
        assert_eq!(per_crore(NSE_OPTIONS_EXCHANGE), 3_553_00);
        assert_eq!(per_crore(BSE_EQUITY_EXCHANGE), 375_00);
        assert_eq!(per_crore(BSE_OPTIONS_EXCHANGE), 3_250_00);
        assert_eq!(per_crore(SEBI_FEE), 10_00);
        assert_eq!(per_crore(STT_EQUITY_NON_DELIVERY), 2_500_00);
        assert_eq!(per_crore(STT_EQUITY_DELIVERY), 10_000_00);
        assert_eq!(per_crore(STT_FUTURES), 5_000_00);
        assert_eq!(per_crore(STT_OPTIONS), 15_000_00);
        assert_eq!(per_crore(STT_EXERCISE), 15_000_00);
        assert_eq!(per_crore(STAMP_EQUITY_NON_DELIVERY), 300_00);
        assert_eq!(per_crore(STAMP_EQUITY_DELIVERY), 1_500_00);
        assert_eq!(per_crore(STAMP_FUTURES), 200_00);
        assert_eq!(per_crore(STAMP_OPTIONS), 300_00);
        assert_eq!(per_crore(GST), 18_00_000_00);
        assert_eq!(per_crore(DHAN_EQUITY_BROKERAGE_RATE), 3_000_00);
        // The FA73061 split, component by component: ₹306.99 + ₹0.01.
        assert_eq!(
            Ratio::new(30_699, 100 * CRORE)
                .apply(ONE_CRORE, 1, Quantum::Paisa)
                .expect("in range")
                .raw()
                + Ratio::new(1, 100 * CRORE)
                    .apply(ONE_CRORE, 1, Quantum::Paisa)
                    .expect("in range")
                    .raw(),
            per_crore(NSE_EQUITY_EXCHANGE)
        );
        assert_eq!(DHAN_EQUITY_BROKERAGE_CAP.raw(), 2_000);
        assert_eq!(DHAN_FNO_BROKERAGE.raw(), 2_000);
        assert_eq!(DHAN_DP_CHARGE.raw(), 1_250);
        assert_eq!(DHAN_SQUARE_OFF_FEE.raw(), 2_000);
    }

    #[test]
    fn the_unverified_figures_are_exactly_the_ones_the_brief_did_not_source() {
        let unverified: Vec<&str> = [
            ("NSE equity exchange", NSE_EQUITY_EXCHANGE),
            ("NSE futures exchange", NSE_FUTURES_EXCHANGE),
            ("NSE options exchange", NSE_OPTIONS_EXCHANGE),
            ("BSE equity exchange", BSE_EQUITY_EXCHANGE),
            ("BSE options exchange", BSE_OPTIONS_EXCHANGE),
            ("SEBI", SEBI_FEE),
            ("GST", GST),
            ("STT non-delivery", STT_EQUITY_NON_DELIVERY),
            ("STT delivery", STT_EQUITY_DELIVERY),
            ("STT futures", STT_FUTURES),
            ("STT options", STT_OPTIONS),
            ("STT exercise", STT_EXERCISE),
            ("stamp non-delivery", STAMP_EQUITY_NON_DELIVERY),
            ("stamp delivery", STAMP_EQUITY_DELIVERY),
            ("stamp futures", STAMP_FUTURES),
            ("stamp options", STAMP_OPTIONS),
            ("Dhan brokerage", DHAN_EQUITY_BROKERAGE_RATE),
        ]
        .into_iter()
        .filter(|(_, rate)| rate.status() == Status::Unverified)
        .map(|(name, rate)| {
            assert!(
                rate.source().contains("UNVERIFIED"),
                "{name} is flagged and must say so in its source"
            );
            name
        })
        .collect();
        assert_eq!(
            unverified,
            [
                "BSE equity exchange",
                "BSE options exchange",
                "STT delivery",
                "STT futures"
            ]
        );
        // The FA73061 figures name the circular; the Dhan figure names the page.
        for rate in [
            NSE_EQUITY_EXCHANGE,
            NSE_FUTURES_EXCHANGE,
            NSE_OPTIONS_EXCHANGE,
        ] {
            assert!(rate.source().contains("FA73061"));
        }
        assert!(
            DHAN_EQUITY_BROKERAGE_RATE
                .source()
                .contains("https://dhan.co/pricing/")
        );
        assert!(STT_EQUITY_NON_DELIVERY.source().contains("NSE/CMTR/5588"));
        for rate in [
            STAMP_EQUITY_NON_DELIVERY,
            STAMP_EQUITY_DELIVERY,
            STAMP_FUTURES,
            STAMP_OPTIONS,
        ] {
            assert!(rate.source().contains("NCL/CMPT/43115"));
        }
    }

    #[test]
    fn the_current_option_taxes_are_the_regimes_rows_on_the_as_of_day() {
        // Today's table carries the option STT as a constant. It must be the
        // regime's own figure on the day it was read, or the two rate sets
        // disagree about today.
        let stt = stt_options_rate(CURRENT_AS_OF).expect("verified");
        let exercise = stt_exercise_rate(CURRENT_AS_OF).expect("verified");
        assert_eq!(Ratio::from_bps_x100(stt), Ratio::new(15_000, RATE_SCALE));
        assert_eq!(per_crore(STT_OPTIONS), stt.get() * 100);
        assert_eq!(per_crore(STT_EXERCISE), exercise.get() * 100);
        assert_eq!(per_crore(STAMP_OPTIONS), STAMP_DUTY_BUY_SIDE.get() * 100);
        assert_eq!(per_crore(SEBI_FEE), SEBI_TURNOVER_FEE.get() * 100);
        assert_eq!(CURRENT_AS_OF, day(2026, 10, 16));
        assert_eq!(FA73061_EFFECTIVE, day(2026, 3, 1));
        assert_ne!(CURRENT_AS_OF, TradeDay::MIN);
        assert_ne!(FA73061_EFFECTIVE, TradeDay::MIN);
    }

    #[test]
    fn dhan_brokerage_is_the_lower_of_twenty_rupees_and_three_basis_points_per_order() {
        let intraday = TradeKind::Equity(Product::Intraday);
        let mtf = TradeKind::Equity(Product::Mtf);
        let charge = |kind, value| dhan_brokerage(kind, paisa(value)).expect("in range").raw();
        // 0.03% of ₹1,000 is 30 paisa; of ₹990, 29.7 → 30.
        assert_eq!(charge(intraday, 1_000_00), 30);
        assert_eq!(charge(intraday, 990_00), 30);
        assert_eq!(charge(intraday, 20_55), 1); // 0.6165 paisa → 1
        assert_eq!(charge(intraday, 16_66), 0); // 0.4998 paisa → 0
        // Just under, at and over the cap: ₹66,666.66, ₹66,666.67, ₹66,700.
        assert_eq!(charge(intraday, 66_666_66), 2_000);
        assert_eq!(charge(intraday, 66_650_00), 2_000); // 1999.5 → 2000
        assert_eq!(charge(intraday, 66_640_00), 1_999); // 1999.2
        assert_eq!(charge(intraday, 66_700_00), 2_000);
        assert_eq!(charge(mtf, 76_000_00), 2_000);
        assert_eq!(charge(mtf, 15_000_00), 450);
        // Delivery is free, at any size.
        for value in [1, 1_000_00, 100_000_000_00] {
            assert_eq!(charge(TradeKind::Equity(Product::Delivery), value), 0);
        }
        // F&O is flat: one rupee of premium and a crore pay the same.
        for value in [1, 1_000_00, 100_000_000_00] {
            assert_eq!(charge(TradeKind::Futures, value), 2_000);
            assert_eq!(charge(TradeKind::Options, value), 2_000);
        }
        // An order value past the rate's range is refused, not wrapped. The
        // percentage is 0.03% so no i64 value can overflow it; the guard is
        // the exact-rate one, reached in crate::exact's own tests.
        assert_eq!(charge(intraday, i64::MAX), 2_000);
    }

    #[test]
    fn todays_card_is_labelled_with_the_trade_day_on_every_kind_and_venue() {
        for kind in ALL_KINDS {
            for exchange in [Exchange::Nse, Exchange::Bse] {
                if (kind, exchange) == (TradeKind::Futures, Exchange::Bse) {
                    continue;
                }
                for when in [day(2018, 1, 1), day(2024, 9, 30), CURRENT_AS_OF] {
                    let card =
                        RateCard::resolve(kind, exchange, when, RateSelector::CurrentOnEveryDate)
                            .expect("today's rates price every date");
                    assert_eq!(card.basis(), RateBasis::TodaysRatesAppliedTo(when));
                    assert_eq!(card.basis().day(), when);
                    assert_eq!(
                        card.basis().to_string(),
                        format!("today's rates applied to {when}")
                    );
                    assert_eq!(card.kind(), kind);
                    assert_eq!(card.exchange(), exchange);
                    assert_eq!(card.sebi(), SEBI_FEE);
                    assert_eq!(card.gst(), GST);
                    // The same card on every date: today's rates do not move.
                    let today =
                        RateCard::resolve(kind, exchange, CURRENT_AS_OF, RateSelector::default())
                            .expect("priced");
                    assert_eq!(card.rates(), today.rates());
                    assert_eq!(card.taxes(), today.taxes());
                }
            }
        }
    }

    #[test]
    fn todays_card_carries_the_venue_and_kind_rates_and_nothing_else() {
        let card = |kind, exchange| {
            RateCard::resolve(kind, exchange, CURRENT_AS_OF, RateSelector::default())
                .expect("priced")
        };
        let nse_eq = card(TradeKind::Equity(Product::Intraday), Exchange::Nse);
        assert_eq!(nse_eq.exchange_charge(), NSE_EQUITY_EXCHANGE);
        assert_eq!(
            nse_eq.taxes(),
            Taxes::Equity {
                product: Product::Intraday,
                stt_non_delivery: STT_EQUITY_NON_DELIVERY,
                stt_delivery: STT_EQUITY_DELIVERY,
                stamp_non_delivery: STAMP_EQUITY_NON_DELIVERY,
                stamp_delivery: STAMP_EQUITY_DELIVERY,
            }
        );
        assert_eq!(
            nse_eq.rates(),
            [
                Some(NSE_EQUITY_EXCHANGE),
                Some(SEBI_FEE),
                Some(GST),
                Some(STT_EQUITY_NON_DELIVERY),
                Some(STT_EQUITY_DELIVERY),
                Some(STAMP_EQUITY_NON_DELIVERY),
                Some(STAMP_EQUITY_DELIVERY),
            ]
        );
        assert_eq!(
            card(TradeKind::Equity(Product::Mtf), Exchange::Bse).exchange_charge(),
            BSE_EQUITY_EXCHANGE
        );
        let fut = card(TradeKind::Futures, Exchange::Nse);
        assert_eq!(fut.exchange_charge(), NSE_FUTURES_EXCHANGE);
        assert_eq!(
            fut.rates(),
            [
                Some(NSE_FUTURES_EXCHANGE),
                Some(SEBI_FEE),
                Some(GST),
                Some(STT_FUTURES),
                Some(STAMP_FUTURES),
                None,
                None,
            ]
        );
        let opt = card(TradeKind::Options, Exchange::Nse);
        assert_eq!(opt.exchange_charge(), NSE_OPTIONS_EXCHANGE);
        assert_eq!(
            opt.rates(),
            [
                Some(NSE_OPTIONS_EXCHANGE),
                Some(SEBI_FEE),
                Some(GST),
                Some(STT_OPTIONS),
                Some(STT_EXERCISE),
                Some(STAMP_OPTIONS),
                None,
            ]
        );
        assert_eq!(
            card(TradeKind::Options, Exchange::Bse).exchange_charge(),
            BSE_OPTIONS_EXCHANGE
        );
    }

    #[test]
    fn bse_futures_have_no_rate_under_todays_table_and_are_refused_by_name() {
        for when in [day(2018, 1, 1), CURRENT_AS_OF] {
            assert_eq!(
                RateCard::resolve(
                    TradeKind::Futures,
                    Exchange::Bse,
                    when,
                    RateSelector::CurrentOnEveryDate
                ),
                Err(ChargeError::NoRate {
                    charge: "futures exchange transaction charge (BSE)",
                    reason: "no BSE futures rate is recorded in this repository",
                })
            );
        }
    }

    #[test]
    fn the_dated_selector_refuses_equity_and_futures_on_every_date_and_names_the_remedy() {
        for kind in [
            TradeKind::Equity(Product::Intraday),
            TradeKind::Equity(Product::Delivery),
            TradeKind::Equity(Product::Mtf),
            TradeKind::Futures,
        ] {
            for exchange in [Exchange::Nse, Exchange::Bse] {
                for when in [TradeDay::MIN, day(2018, 1, 1), CURRENT_AS_OF, TradeDay::MAX] {
                    let refused = RateCard::resolve(kind, exchange, when, RateSelector::Dated)
                        .expect_err("no dated equity or futures history");
                    let ChargeError::Cost(CostError::Unverified(refusal)) = refused else {
                        panic!("a dated refusal, not {refused:?}");
                    };
                    assert_eq!(refusal.on(), when);
                    assert_eq!(refusal.exchange(), Some(exchange));
                    assert_eq!(refusal.window_start(), TradeDay::MIN);
                    assert_eq!(refusal.verified_from(), None);
                    assert_eq!(refusal.source(), NO_DATED_HISTORY);
                    assert_eq!(refusal.remediation(), NO_DATED_HISTORY_REMEDY);
                    let want = if kind == TradeKind::Futures {
                        "futures charge stack (dated rates)"
                    } else {
                        "equity cash charge stack (dated rates)"
                    };
                    assert_eq!(refusal.charge(), want);
                    assert!(refused.to_string().contains("CurrentOnEveryDate"));
                }
            }
        }
    }

    #[test]
    fn the_dated_option_card_is_the_regime_on_the_trade_day_with_the_ipft_folded_at_fa73061() {
        let dated = |exchange, when| {
            RateCard::resolve(TradeKind::Options, exchange, when, RateSelector::Dated)
                .expect("a verified window")
        };
        // Before FA73061: the regime's 0.03503% plus the NSE IPFT, 0.0005%.
        let before = dated(Exchange::Nse, day(2026, 2, 28));
        assert_eq!(before.basis(), RateBasis::InForceOn(day(2026, 2, 28)));
        assert_eq!(before.basis().to_string(), "rates in force on 2026-02-28");
        assert_eq!(
            before.exchange_charge().rate(),
            Ratio::new(3_553, RATE_SCALE)
        );
        assert_eq!(before.exchange_charge().status(), Status::Cited);
        assert_eq!(before.exchange_charge().source(), REGIME_EXCHANGE_NSE);
        assert_eq!(
            before.taxes(),
            Taxes::Options {
                stt_sell: Cited::named(10_000, RATE_SCALE, REGIME_ROW),
                stt_exercise: Cited::named(12_500, RATE_SCALE, REGIME_ROW),
                stamp_buy: Cited::named(300, RATE_SCALE, STAMP_OPTIONS.source()),
            }
        );
        assert_eq!(before.sebi().rate(), Ratio::new(10, RATE_SCALE));
        assert_eq!(before.gst().rate(), Ratio::new(1_800_000, RATE_SCALE));
        assert_eq!(before.kind(), TradeKind::Options);
        // From FA73061: the regime row alone. Until package CHG-0 adds the
        // 0.03553% row, that row is still FA64232's 0.03503% — so this is
        // the figure the IGNORED tests in crate::stack wait on.
        let from = dated(Exchange::Nse, FA73061_EFFECTIVE);
        let regime_now = exchange_charge_rate(Exchange::Nse, FA73061_EFFECTIVE).expect("verified");
        assert_eq!(
            from.exchange_charge().rate(),
            Ratio::new(regime_now.get(), RATE_SCALE)
        );
        // On BSE the IPFT is a zero, always added, and flagged.
        let bse = dated(Exchange::Bse, CURRENT_AS_OF);
        assert_eq!(bse.exchange_charge().rate(), Ratio::new(3_250, RATE_SCALE));
        assert_eq!(bse.exchange_charge().status(), Status::Unverified);
        assert_eq!(bse.exchange_charge().source(), REGIME_EXCHANGE_BSE);
        // After 2026-04-01 the option taxes are today's.
        let late = dated(Exchange::Nse, CURRENT_AS_OF);
        assert_eq!(
            late.taxes(),
            Taxes::Options {
                stt_sell: Cited::named(15_000, RATE_SCALE, REGIME_ROW),
                stt_exercise: Cited::named(15_000, RATE_SCALE, REGIME_ROW),
                stamp_buy: Cited::named(300, RATE_SCALE, STAMP_OPTIONS.source()),
            }
        );
    }

    #[test]
    fn the_dated_option_card_refuses_the_pre_2024_window_with_the_regimes_own_refusal() {
        for exchange in [Exchange::Nse, Exchange::Bse] {
            let when = day(2024, 9, 30);
            let refused =
                RateCard::resolve(TradeKind::Options, exchange, when, RateSelector::Dated)
                    .expect_err("the slab-ladder window has no rate");
            let regime = exchange_charge_rate(exchange, when).expect_err("refused there too");
            assert_eq!(refused, ChargeError::Cost(CostError::Unverified(regime)));
        }
        // And today's rates price that same day, labelled.
        let card = RateCard::resolve(
            TradeKind::Options,
            Exchange::Nse,
            day(2024, 9, 30),
            RateSelector::CurrentOnEveryDate,
        )
        .expect("priced at today's rates");
        assert_eq!(
            card.basis().to_string(),
            "today's rates applied to 2024-09-30"
        );
    }

    #[test]
    fn the_kind_names_and_the_square_off_scope_are_pinned() {
        let names: Vec<String> = ALL_KINDS.iter().map(ToString::to_string).collect();
        assert_eq!(
            names,
            [
                "equity intraday",
                "equity delivery",
                "equity MTF",
                "futures",
                "options"
            ]
        );
        let squared: Vec<bool> = ALL_KINDS
            .iter()
            .map(|kind| kind.can_be_squared_off_by_broker())
            .collect();
        assert_eq!(squared, [true, false, false, true, true]);
        assert_eq!(RateSelector::default(), RateSelector::CurrentOnEveryDate);
        assert_ne!(Status::Cited, Status::Unverified);
        assert_eq!(MAX_RATES_ON_A_CARD, 7);
    }

    #[test]
    fn a_rate_without_a_source_or_with_a_bad_shape_is_not_well_shaped() {
        assert!(Cited::named(1, 1, "a source").is_well_shaped());
        assert!(!Cited::named(1, 1, "").is_well_shaped());
        assert!(!Cited::named(1, 1, "  \t").is_well_shaped());
        assert!(!Cited::named(1, 0, "a source").is_well_shaped());
        assert!(!Cited::unverified(-1, 1, "a source").is_well_shaped());
    }
}
