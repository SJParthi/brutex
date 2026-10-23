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
//! [`RateSelector::CurrentOnEveryDate`] is **the rate set backtests use**, by
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
use crate::exact::{Exact, Quantum, Ratio, units_of};
use crate::rate::{GST_ON_FEE_BASE, RATE_SCALE, SEBI_TURNOVER_FEE, STAMP_DUTY_BUY_SIDE};
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
    /// Units of an exchange-traded fund. **Not priced**: no ETF charge row
    /// has been read from a source, and no calculator reading exists, so both
    /// modes refuse it by name ([`TradeKind::refusal`]).
    Etf,
    /// A commodity contract. **Not priced**: commodities trade on MCX, which
    /// is outside the engine's scope (`CLAUDE.md` §1, D-0017); both modes
    /// refuse it by name.
    Commodity,
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
            Self::Etf => "ETF",
            Self::Commodity => "commodity",
        }
    }

    /// Whether a position of this kind can be squared off by the broker at
    /// the intraday ceiling. A delivery or MTF holding is carried, not
    /// squared off, so the ceiling fee cannot apply to it; a kind the stack
    /// does not price has no ceiling here either.
    #[must_use]
    pub const fn can_be_squared_off_by_broker(self) -> bool {
        matches!(
            self,
            Self::Equity(Product::Intraday) | Self::Futures | Self::Options
        )
    }

    /// The named refusal of a kind neither mode prices, or `None` for a kind
    /// both price.
    #[must_use]
    pub const fn refusal(self) -> Option<ChargeError> {
        match self {
            Self::Etf => Some(ETF_REFUSAL),
            Self::Commodity => Some(COMMODITY_REFUSAL),
            Self::Equity(_) | Self::Futures | Self::Options => None,
        }
    }

    /// The segment a contract note puts the kind in: the cash market for
    /// equity, derivatives for futures and options. STT and stamp duty are
    /// rounded on one client's day in one segment ([`crate::stack::ClientDay`]).
    #[must_use]
    pub const fn segment(self) -> Segment {
        match self {
            Self::Futures | Self::Options | Self::Commodity => Segment::Derivatives,
            Self::Equity(_) | Self::Etf => Segment::Cash,
        }
    }
}

/// How an exercised option settles — named by the caller on every exercise,
/// because the two are priced differently and one of them not at all.
///
/// No `Default` (D-5130): a backtest that holds a stock option in the money to
/// expiry must not reach the cash-settled arithmetic by omission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Settlement {
    /// Settled in cash on the intrinsic value — an index option. STT at the
    /// exercise rate on the intrinsic value (`NSE/FATAX/73524`, sources row
    /// `S-04`).
    Cash,
    /// Settled by delivery of the underlying shares — a stock option held in
    /// the money to expiry. **Not priced**: refused by name
    /// ([`PHYSICAL_DELIVERY_REFUSAL`]).
    PhysicalDelivery,
}

/// Why the physical settlement of a stock option is not priced, in either
/// mode (D-5130).
pub const PHYSICAL_DELIVERY_REFUSAL: ChargeError = ChargeError::OutOfScope {
    what: "physical delivery settlement of a stock option",
    reason: "Dhan's 0.10% of the contract value (sources row B-06) has no read effective date \
             and the delivery leg's STT (row S-06) was never read from a source — refused rather \
             than priced as a cash-settled exercise",
};

/// The segment a trade is noted in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Segment {
    /// The cash market: equity in every product.
    Cash,
    /// Equity derivatives: futures and options.
    Derivatives,
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
/// side — ₹307.00 per crore, rounded **once** as one line. The figure is the
/// circular's; rounding the two parts as one line is UNVERIFIED (D-5122).
pub const NSE_EQUITY_EXCHANGE: Cited = Cited::named(
    30_700,
    100 * CRORE,
    "NSE circular NSE/FA/73061 dated 2026-02-27, effective 2026-03-01 \
     (https://nsearchives.nseindia.com/content/circulars/FA73061.pdf, read 13 and 15 Oct 2026, \
     sources rows E-01 and E-04): cash market transaction charge Rs 306.99 per crore + IPFT \
     Rs 0.01 per crore, each side (docs/00-charter.md, CHG-1).",
);

/// NSE futures: transaction charge ₹182.99 + IPFT ₹0.01 per crore.
pub const NSE_FUTURES_EXCHANGE: Cited = Cited::named(
    18_300,
    100 * CRORE,
    "NSE circular NSE/FA/73061 dated 2026-02-27, effective 2026-03-01 (sources row E-02): \
     futures transaction charge Rs 182.99 per crore + IPFT Rs 0.01 per crore, each side \
     (docs/00-charter.md, CHG-1).",
);

/// NSE options: transaction charge ₹3,552.99 + IPFT ₹0.01 per crore of
/// premium — 0.03553%. Carried here, in today's table, **only** because it
/// names `FA73061`; the dated path reads `crate::regime` instead.
pub const NSE_OPTIONS_EXCHANGE: Cited = Cited::named(
    355_300,
    100 * CRORE,
    "NSE circular NSE/FA/73061 dated 2026-02-27, effective 2026-03-01 (sources row E-03): \
     options transaction charge Rs 3,552.99 per crore of premium + IPFT Rs 0.01 per crore, each \
     side — 0.03553% (docs/00-charter.md, CHG-1).",
);

/// BSE cash market: ₹375 per crore all-in. **UNVERIFIED as a circular**;
/// Dhan's calculator uses 0.000038.
pub const BSE_EQUITY_EXCHANGE: Cited = Cited::unverified(
    375,
    CRORE,
    "UNVERIFIED — BSE cash market Rs 375 per crore all-in, as the CHG-1 brief states it; no BSE \
     notice has been read for it. Dhan's calculator uses 0.000038 (Rs 380 per crore).",
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
    "SEBI turnover fee Rs 10 per crore, each side — SEBI notification of 2019-03-22, in force \
     2019-04-01, annexed to NSE circular FA40632 \
     (https://nsearchives.nseindia.com/content/circulars/FA40632.pdf, sources rows F-01, F-02; \
     crate::rate::SEBI_TURNOVER_FEE).",
);

/// GST: 18% on brokerage, the exchange charge (IPFT included) and the SEBI fee.
pub const GST: Cited = Cited::named(
    18,
    100,
    "GST 18% on brokerage + exchange transaction charge + IPFT + SEBI fee — Dhan pricing page \
     https://dhan.co/pricing/ (sources row G-02); on the SEBI fee from 2022-07-18 per NSE/FATAX/52993 \
     (row G-03); crate::rate::GST_ON_FEE_BASE. The legal basis for GST on the exchange charge \
     (row G-04) is UNVERIFIED.",
);

/// Equity intraday STT: 0.025% of the non-delivery value, sell side. The
/// value is the non-delivery quantity at the day's volume-weighted average
/// price of **all** that client's trades in the security, the VWAP rounded to
/// two decimals; each security's tax is rounded to two decimals and the
/// client's day total to the rupee (`crate::stack::ClientDay`).
pub const STT_EQUITY_NON_DELIVERY: Cited = Cited::named(
    25,
    100_000,
    "STT 0.025% of the non-delivery value, seller — rate NSE/FATAX/73524 dated 2026-03-31 \
     (https://nsearchives.nseindia.com/content/circulars/FATAX73524.pdf, sources row S-01); \
     method NSE/CMTR/5588 dated 2004-11-10 \
     (https://nsearchives.nseindia.com/content/circulars/cmtr5588.htm, read 15 Oct 2026): \
     min(day buy qty, day sell qty) x VWAP rounded to two decimals, each security's tax to two \
     decimals, the client's day total to the nearest rupee (docs/00-charter.md, CHG-1).",
);

/// Equity delivery and MTF STT: 0.1% of the value, buy and sell.
pub const STT_EQUITY_DELIVERY: Cited = Cited::named(
    1,
    1_000,
    "STT 0.1% of the value on both sides of a sale settled by the actual delivery or transfer of \
     the share — NSE/FATAX/73524 dated 2026-03-31 \
     (https://nsearchives.nseindia.com/content/circulars/FATAX73524.pdf, sources row S-05, read \
     13 Oct 2026).",
);

/// Futures STT: 0.05% of the sell value, from 2026-04-01.
pub const STT_FUTURES: Cited = Cited::named(
    5,
    10_000,
    "STT 0.05% of the futures sell value from 2026-04-01 — NSE/FATAX/73524 dated 2026-03-31 \
     (https://nsearchives.nseindia.com/content/circulars/FATAX73524.pdf, sources row S-02: \
     'Sale of a futures in securities has been revised to 0.05%').",
);

/// Options STT: 0.15% of the sell premium. The same figure as the dated
/// regime's row from 2026-04-01 — `the_current_option_taxes_are_the_regimes_rows_on_the_as_of_day`
/// holds the two together.
pub const STT_OPTIONS: Cited = Cited::named(
    15,
    10_000,
    "STT 0.15% of the options sell premium from 2026-04-01 — NSE/FATAX/73524 (sources row S-03); \
     Finance Bill 2026 (DEC-COST-003), the crate::regime row from 2026-04-01.",
);

/// STT on exercise: 0.15% of the intrinsic value. The regime's row from
/// 2026-04-01, cited there as Finance Act 2026 (DEC-COST-004).
pub const STT_EXERCISE: Cited = Cited::named(
    15,
    10_000,
    "STT on exercise 0.15% of intrinsic value from 2026-04-01 — NSE/FATAX/73524 (sources row \
     S-04); Finance Act 2026 (DEC-COST-004), the crate::regime row from 2026-04-01.",
);

/// Equity intraday stamp duty: 0.003% of the FIFO buy value of the
/// non-deliverable quantity; each security's duty to two decimals, the
/// client's day total to the rupee.
pub const STAMP_EQUITY_NON_DELIVERY: Cited = Cited::named(
    3,
    100_000,
    "Stamp duty 0.003% of the FIFO buy value of the non-deliverable quantity, buyer — NSE \
     Clearing circular NCL/CMPT/43115 dated 2020-01-03, from trade date 2020-07-01 per \
     NCL/CMPT/44032 (https://nsearchives.nseindia.com/content/circulars/CMPT43115.zip, sources \
     rows T-01 and R-02, read 13 and 15 Oct 2026): each security's duty to two decimals, the \
     client's day total to the nearest rupee.",
);

/// Equity delivery and MTF stamp duty: 0.015% of the buy value.
pub const STAMP_EQUITY_DELIVERY: Cited = Cited::named(
    15,
    100_000,
    "Stamp duty 0.015% of the delivery buy value, buyer — NSE's levies page \
     https://www.nseindia.com/static/invest/first-time-investor-sebi-turnover-fees-stt-other-levies \
     (sources row T-04, read 13 Oct 2026).",
);

/// Futures stamp duty: 0.002% of the buy value.
pub const STAMP_FUTURES: Cited = Cited::named(
    2,
    100_000,
    "Stamp duty 0.002% of the futures buy value, buyer — NSE Clearing circular NCL/CMPT/43116 \
     (https://nsearchives.nseindia.com/content/circulars/CMPT43116.zip, sources row T-02: \
     'Equity Futures 0.002% Buyer').",
);

/// Options stamp duty: 0.003% of the buy premium.
pub const STAMP_OPTIONS: Cited = Cited::named(
    3,
    100_000,
    "Stamp duty 0.003% of the options buy premium, buyer — NSE Clearing circular NCL/CMPT/43116 \
     (sources row T-03: 'option trade shall be valued at premium'); the same figure as \
     crate::rate::STAMP_DUTY_BUY_SIDE (COSTS_VERIFIED section 6).",
);

// ---------------------------------------------------------------------------
// Today's rates — Dhan's own fees (https://dhan.co/pricing/, read 13 and 15 Oct 2026)
// ---------------------------------------------------------------------------

/// Dhan equity intraday and MTF brokerage: the **lower** of ₹20 and 0.03% of
/// the order value, per executed order. This is the 0.03% half.
pub const DHAN_EQUITY_BROKERAGE_RATE: Cited = Cited::named(
    3,
    10_000,
    "Dhan pricing page https://dhan.co/pricing/ (read 13 and 15 Oct 2026, sources row B-01): \
     equity intraday brokerage is the lower of Rs 20 and 0.03% of the order value, per executed \
     order. MTF taken the same way is the 16 Oct calculator capture's assumption.",
);

/// The ₹20 cap on Dhan's equity intraday and MTF brokerage, per order.
pub const DHAN_EQUITY_BROKERAGE_CAP: Paisa = Paisa::from_raw(2_000);

/// Dhan F&O brokerage: ₹20 per executed order, whatever its size (sources
/// rows B-03, B-04). Row B-05 applies it also to an option *"expired,
/// exercised, or assigned"*: the stack charges it on every executed order and
/// on each exercise (D-5132). An assignment and an expiry are not inputs of
/// the stack — no day can record one — so their ₹20 is NOT PRICED, and a
/// position held to either cannot be netted (`docs/06-limits.md` §CHG-1).
pub const DHAN_FNO_BROKERAGE: Paisa = Paisa::from_raw(2_000);

/// Dhan DP charge: ₹12.50 per instruction per ISIN on a delivery or MTF sell
/// day, plus GST (sources row B-10).
pub const DHAN_DP_CHARGE: Paisa = Paisa::from_raw(1_250);

/// Dhan's square-off fee for a position still open at the intraday ceiling:
/// ₹20 per order, plus GST — a separate line, charged only when the caller
/// says the exit was the broker's (sources row B-07). Whether it replaces or
/// comes on top of the order's brokerage is UNVERIFIED; it is priced on top.
pub const DHAN_SQUARE_OFF_FEE: Paisa = Paisa::from_raw(2_000);

/// The brokerage Dhan charges on one executed order of `kind` worth
/// `order_value`, rounded half-up to the paisa.
///
/// Equity delivery is free; equity intraday and MTF pay the lower of ₹20 and
/// 0.03%; futures and options pay a fixed ₹20.
///
/// # Errors
///
/// * [`ChargeError::OutOfScope`] for an ETF or a commodity.
/// * [`ChargeError::Cost`] carrying an overflow, which a real order value
///   cannot reach.
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
            // The lower of the two. `min` rather than a comparison: at equality
            // both arms are ₹20, so a `<` and a `<=` are the same function and
            // no test could tell them apart (cargo-mutants found it).
            Ok(percent.min(DHAN_EQUITY_BROKERAGE_CAP))
        }
        TradeKind::Futures | TradeKind::Options => Ok(DHAN_FNO_BROKERAGE),
        TradeKind::Etf => Err(ETF_REFUSAL),
        TradeKind::Commodity => Err(COMMODITY_REFUSAL),
    }
}

/// [`dhan_brokerage`] before its rounding: the order's brokerage in
/// [`Exact`] units of 10⁻⁹ rupee. Rounded half-up to the paisa it is
/// [`dhan_brokerage`] exactly (the cap is a whole paisa, so taking the lower
/// before or after the rounding is the same figure).
///
/// # Errors
///
/// As [`dhan_brokerage`].
pub(crate) fn dhan_brokerage_units(
    kind: TradeKind,
    order_value: Paisa,
) -> Result<Exact, ChargeError> {
    match kind {
        TradeKind::Equity(Product::Delivery) => Ok(Exact::ZERO),
        TradeKind::Equity(Product::Intraday | Product::Mtf) => {
            let percent = units_of(order_value, DHAN_EQUITY_BROKERAGE_RATE.rate(), 1)?;
            Ok(percent.min(Exact::of_paisa(DHAN_EQUITY_BROKERAGE_CAP)))
        }
        TradeKind::Futures | TradeKind::Options => Ok(Exact::of_paisa(DHAN_FNO_BROKERAGE)),
        TradeKind::Etf => Err(ETF_REFUSAL),
        TradeKind::Commodity => Err(COMMODITY_REFUSAL),
    }
}

/// Why an ETF is not priced, in either mode.
pub const ETF_REFUSAL: ChargeError = ChargeError::OutOfScope {
    what: "ETF",
    reason: "no ETF charge row (STT, stamp duty, exchange charge on ETF units) has been read \
             from a source, and no calculator reading exists — refused rather than priced as \
             equity",
};

/// Why a commodity contract is not priced, in either mode.
pub const COMMODITY_REFUSAL: ChargeError = ChargeError::OutOfScope {
    what: "commodity",
    reason: "commodity contracts trade on MCX, which is outside the engine's scope (CLAUDE.md \
             section 1, D-0017)",
};

// The regulatory options brokerage cap — not more than the higher of 2.5% of
// the premium and Rs 100 per lot (NSE circular INSP53297, sources row B-14) —
// is never below Rs 100 for an order of at least one lot, so it can never bind
// Dhan's Rs 20 per order. If Dhan's F&O figure ever reached Rs 100 the cap
// would have to be priced, and this stops the build until it is.
const _: () = assert!(DHAN_FNO_BROKERAGE.raw() < 10_000); // ₹100 in paisa

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
///
/// **No `Default`** (D-5127): every caller names the rate set, so a run's
/// identity carries [`Self::version`] and never an implicit choice. Today's
/// rates on every date is the set a backtest uses, by the operator's choice;
/// it is named, not defaulted. `crate::ledger` fails the build if this type
/// ever gains a `Default`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RateSelector {
    /// Today's rates on every trade date — the operator's choice for
    /// backtests. Labelled on every output.
    CurrentOnEveryDate,
    /// The rates in force on the trade date, from the dated regime tables.
    /// Index options only; anything else refuses.
    Dated,
}

/// The version of today's rate table: the day it was read, and a revision.
/// A change to any rate in the table, or to Dhan's brokerage, DP or
/// square-off figures, moves it —
/// `the_rate_set_versions_are_pinned_to_the_tables_they_name` fails until it
/// does.
pub const TODAYS_RATES_VERSION: &str = "todays-rates/2026-10-16/r1";

/// The version of the dated rate set: the dated regime tables of
/// `crate::regime` as resolved by [`RateCard::resolve`]. A change to any
/// dated rate the card carries moves it — the same test fails until it does.
/// r2 (D-5131): r1 dropped the NSE options IPFT from 2026-03-01 and charged
/// 0.03503%; r2 charges the 0.03553% the trip stack does. Package CHG-0's
/// FA73061 row re-splits that same total, so it leaves every figure here —
/// and the fingerprint — as they are.
pub const DATED_RATES_VERSION: &str = "dated-regime/r2";

impl RateSelector {
    /// The version string a run identity records for this rate set.
    ///
    /// # Examples
    ///
    /// ```
    /// use costs::schedule::RateSelector;
    ///
    /// assert_eq!(
    ///     RateSelector::CurrentOnEveryDate.version(),
    ///     "todays-rates/2026-10-16/r1"
    /// );
    /// assert_eq!(RateSelector::Dated.version(), "dated-regime/r2");
    /// ```
    #[must_use]
    pub const fn version(self) -> &'static str {
        match self {
            Self::CurrentOnEveryDate => TODAYS_RATES_VERSION,
            Self::Dated => DATED_RATES_VERSION,
        }
    }
}

/// Which arithmetic prices a day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Mode {
    /// The contract-note method under NSE's and SEBI's rules
    /// (`crate::stack`) — **the default**, and the only mode a win or a loss
    /// is decided in (D-5124).
    #[default]
    Precise,
    /// Dhan's public calculator, line by line (`crate::dhan_calc`) — a
    /// labelled comparison column, never a decision.
    DhanCalculator,
}

/// The version of the precise method: a revision of its rounding and
/// aggregation rules (D-5113, D-5121), and of what it charges on.
///
/// r2 (D-5132): r1 charged an option exercise its STT alone; r2 also charges
/// Dhan's ₹20 brokerage on each exercise, and GST on it (sources row B-05),
/// so a day with an exercise prices ₹23.60 higher per exercise under r2.
/// Every day without an exercise prices exactly as under r1.
pub const PRECISE_VERSION: &str = "precise/r2";

/// The version of the calculator replica: the day the page was read, and a
/// revision of its constants (D-5112).
pub const DHAN_CALCULATOR_VERSION: &str = "dhan-calculator/2026-10-16/r1";

impl Mode {
    /// The label every output of this mode carries.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Precise => crate::stack::PRECISE_LABEL,
            Self::DhanCalculator => crate::dhan_calc::DHAN_LABEL,
        }
    }

    /// The version string a run identity records for this mode.
    #[must_use]
    pub const fn version(self) -> &'static str {
        match self {
            Self::Precise => PRECISE_VERSION,
            Self::DhanCalculator => DHAN_CALCULATOR_VERSION,
        }
    }
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

    /// The rate set the label names: today's rates on every date, or the
    /// rates in force on the day.
    ///
    /// # Examples
    ///
    /// ```
    /// use costs::day::TradeDay;
    /// use costs::schedule::{RateBasis, RateSelector};
    ///
    /// let on = TradeDay::new(2026, 10, 15)?;
    /// assert_eq!(
    ///     RateBasis::TodaysRatesAppliedTo(on).rate_set(),
    ///     RateSelector::CurrentOnEveryDate
    /// );
    /// assert_eq!(RateBasis::InForceOn(on).rate_set(), RateSelector::Dated);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn rate_set(self) -> RateSelector {
        match self {
            Self::TodaysRatesAppliedTo(_) => RateSelector::CurrentOnEveryDate,
            Self::InForceOn(_) => RateSelector::Dated,
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
    /// let card = RateCard::resolve(kind, Exchange::Nse, day, RateSelector::CurrentOnEveryDate)?;
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
        (TradeKind::Etf, _) => return Err(ETF_REFUSAL),
        (TradeKind::Commodity, _) => return Err(COMMODITY_REFUSAL),
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
const REGIME_EXCHANGE_NSE: &str = "crate::regime::exchange_charge_rate plus \
     crate::regime::ipft_rate on the trade day, exactly as crate::trip::Rates::resolve adds them \
     (D-5131, D-5100): to 2026-02-28 FA64232's Rs 3,503 plus the Rs 50 IPFT (FA56129); from \
     2026-03-01 FA73061's Rs 3,552.99 + Rs 0.01, carried as one Rs 3,553 row plus a zero IPFT \
     line because the regime's unit is one rupee per crore (sources row E-03) - Rs 3,553 per \
     crore of premium on both sides of the date.";

/// The source line a dated BSE options exchange charge carries.
const REGIME_EXCHANGE_BSE: &str = "crate::regime::exchange_charge_rate on the trade day, plus \
     crate::regime::ipft_rate, which for BSE is crate::rate::BSE_IPFT on every day, a zero that \
     is UNVERIFIED.";

/// What a dated equity or futures refusal says.
const NO_DATED_HISTORY: &str = "UNVERIFIED — this repository records no dated history of the \
     equity cash or futures charge rates. Today's rates are recorded (FA73061 and the CHG-1 \
     brief, docs/00-charter.md) and are applied to any date only under \
     RateSelector::CurrentOnEveryDate, labelled \"today's rates applied to <date>\".";

/// The remedy a dated equity or futures refusal names.
const NO_DATED_HISTORY_REMEDY: &str = "price under RateSelector::CurrentOnEveryDate (the backtest rate set, \
     labelled), or source each superseded circular and add a dated table for this kind to \
     crates/costs/src/schedule.rs, with a docs/05-decisions.md entry";

/// The dated card: the regime tables for options, a refusal for the rest.
fn dated_card(kind: TradeKind, exchange: Exchange, day: TradeDay) -> Result<RateCard, ChargeError> {
    let charge = match kind {
        TradeKind::Options => return dated_option_card(exchange, day),
        TradeKind::Equity(_) => "equity cash charge stack (dated rates)",
        TradeKind::Futures => "futures charge stack (dated rates)",
        TradeKind::Etf => return Err(ETF_REFUSAL),
        TradeKind::Commodity => return Err(COMMODITY_REFUSAL),
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

/// The dated options card: the regime's rows on `day`, the fixed rates from
/// `crate::rate`, exactly as `crate::trip::Rates::resolve` reads them — the
/// exchange charge is the regime row in force **plus the IPFT `trip` adds**,
/// on every day.
///
/// The IPFT is never dropped by date. Until D-5131 it was dropped from
/// FA73061's effective date (2026-03-01) on, on the expectation that package
/// CHG-0's FA73061 row (₹3,552.99 + ₹0.01 per crore, carried together as
/// 0.03553%) would be the row in force; that row was not yet in
/// `crate::regime`, so the row in force was still FA64232's 0.03503% and the
/// card charged 0.03503% — the IPFT's 0.0005% short — on every NSE options
/// day from 2026-03-01, labelled as cited. One figure, read where `trip` reads
/// it, cannot fold twice or not at all: whichever split the regime carries,
/// the card is the regime's row plus the regime's IPFT, and
/// `the_dated_option_card_is_the_trip_stacks_exchange_charge_plus_ipft_on_every_day`
/// holds it to `trip` on every day from 2018 to 2027 at both venues.
///
/// Since D-5100 (CHG-0) the regime carries the FA73061 row and dates the IPFT
/// beside it, and the IPFT is read from [`crate::regime::ipft_rate`], the one
/// dated IPFT authority `trip` reads too: ₹3,503 + ₹50 to 2026-02-28, ₹3,553 +
/// a folded zero from 2026-03-01 — the same 0.03553% on every day, so every
/// figure of this card and the `dated-regime/r2` fingerprint are unchanged.
/// The undated `crate::rate::ipft` it read before is gone: kept beside the
/// ₹3,553 row it would have charged 0.03603% (D-5101).
fn dated_option_card(exchange: Exchange, day: TradeDay) -> Result<RateCard, ChargeError> {
    let base = exchange_charge_rate(exchange, day)?;
    let stt_sell = stt_options_rate(day)?;
    let stt_exercise = stt_exercise_rate(day)?;
    let ipft_bps = crate::regime::ipft_rate(exchange, day)?.get();
    // The BSE IPFT is a zero no source states (crate::rate::BSE_IPFT).
    let exchange_charge = match exchange {
        Exchange::Nse => Status::Cited,
        Exchange::Bse => Status::Unverified,
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
        assert_eq!(unverified, ["BSE equity exchange", "BSE options exchange"]);
        // The FA73061 figures name the circular; the Dhan figure names the page.
        for rate in [
            NSE_EQUITY_EXCHANGE,
            NSE_FUTURES_EXCHANGE,
            NSE_OPTIONS_EXCHANGE,
        ] {
            assert!(rate.source().contains("NSE/FA/73061"));
        }
        assert!(
            DHAN_EQUITY_BROKERAGE_RATE
                .source()
                .contains("https://dhan.co/pricing/")
        );
        // A cited source never calls itself unverified.
        for rate in [STT_EQUITY_DELIVERY, STT_FUTURES, SEBI_FEE, STAMP_OPTIONS] {
            assert!(
                !rate.source().starts_with("UNVERIFIED"),
                "{}",
                rate.source()
            );
        }
    }

    #[test]
    fn an_etf_and_a_commodity_are_refused_by_name_in_every_entry_point() {
        // No ETF row has been read and MCX is outside the engine's scope
        // (CLAUDE.md section 1): both are named refusals, never priced as
        // equity or as an index contract.
        for (kind, refusal, name) in [
            (TradeKind::Etf, ETF_REFUSAL, "ETF"),
            (TradeKind::Commodity, COMMODITY_REFUSAL, "commodity"),
        ] {
            assert_eq!(kind.as_str(), name);
            assert_eq!(kind.to_string(), name);
            assert_eq!(kind.refusal(), Some(refusal));
            assert!(!kind.can_be_squared_off_by_broker());
            assert_eq!(dhan_brokerage(kind, paisa(1_000_00)), Err(refusal));
            // The unrounded brokerage a backtest's order lines read is refused
            // the same way, before any rate card is resolved.
            assert_eq!(dhan_brokerage_units(kind, paisa(1_000_00)), Err(refusal));
            for selector in [RateSelector::CurrentOnEveryDate, RateSelector::Dated] {
                for exchange in [Exchange::Nse, Exchange::Bse] {
                    assert_eq!(
                        RateCard::resolve(kind, exchange, CURRENT_AS_OF, selector),
                        Err(refusal)
                    );
                }
            }
            assert!(
                matches!(refusal, ChargeError::OutOfScope { what, .. } if what == name),
                "{refusal}"
            );
        }
        assert!(COMMODITY_REFUSAL.to_string().contains("MCX"));
        // Every kind the stack prices has no refusal, and the segments split
        // the cash market from derivatives.
        for kind in ALL_KINDS {
            assert_eq!(kind.refusal(), None);
        }
        assert_eq!(TradeKind::Equity(Product::Mtf).segment(), Segment::Cash);
        assert_eq!(TradeKind::Etf.segment(), Segment::Cash);
        assert_eq!(TradeKind::Futures.segment(), Segment::Derivatives);
        assert_eq!(TradeKind::Options.segment(), Segment::Derivatives);
        assert_eq!(TradeKind::Commodity.segment(), Segment::Derivatives);
        assert!(TradeKind::Equity(Product::Intraday).can_be_squared_off_by_broker());
        assert!(TradeKind::Futures.can_be_squared_off_by_broker());
        assert!(TradeKind::Options.can_be_squared_off_by_broker());
        assert!(!TradeKind::Equity(Product::Delivery).can_be_squared_off_by_broker());
        assert!(!TradeKind::Equity(Product::Mtf).can_be_squared_off_by_broker());
    }

    #[test]
    fn every_rate_names_the_primary_source_the_sourced_rows_read() {
        // state/fno-winners-20261013/sources-charges.json (read 13 Oct 2026)
        // and state/charges-deep-check-20261015.json (read 15 Oct 2026) hold
        // each of these circulars as read: the delivery and futures STT are
        // NSE/FATAX/73524 (rows S-05, S-02), F&O stamp duty is NCL/CMPT/43116
        // (T-02, T-03), delivery stamp duty is NSE's levies page (T-04), and
        // the SEBI fee is FA40632 (F-01). Only the BSE rows stay UNVERIFIED.
        assert_eq!(STT_EQUITY_DELIVERY.status(), Status::Cited);
        assert_eq!(STT_FUTURES.status(), Status::Cited);
        for rate in [STT_EQUITY_NON_DELIVERY, STT_EQUITY_DELIVERY, STT_FUTURES] {
            assert!(
                rate.source().contains("NSE/FATAX/73524"),
                "{}",
                rate.source()
            );
        }
        assert!(STT_EQUITY_NON_DELIVERY.source().contains("NSE/CMTR/5588"));
        for rate in [STAMP_FUTURES, STAMP_OPTIONS] {
            assert!(
                rate.source().contains("NCL/CMPT/43116"),
                "{}",
                rate.source()
            );
        }
        assert!(
            STAMP_EQUITY_NON_DELIVERY
                .source()
                .contains("NCL/CMPT/43115")
        );
        assert!(
            STAMP_EQUITY_DELIVERY
                .source()
                .contains("first-time-investor")
        );
        assert!(SEBI_FEE.source().contains("FA40632"));
        for rate in [
            NSE_EQUITY_EXCHANGE,
            NSE_FUTURES_EXCHANGE,
            NSE_OPTIONS_EXCHANGE,
        ] {
            assert!(rate.source().contains("2026-02-27"), "{}", rate.source());
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
                    let today = RateCard::resolve(
                        kind,
                        exchange,
                        CURRENT_AS_OF,
                        RateSelector::CurrentOnEveryDate,
                    )
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
            RateCard::resolve(
                kind,
                exchange,
                CURRENT_AS_OF,
                RateSelector::CurrentOnEveryDate,
            )
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
                    let want = if kind == TradeKind::Futures {
                        "futures charge stack (dated rates)"
                    } else {
                        "equity cash charge stack (dated rates)"
                    };
                    // Every field of the refusal, at once: the charge, the
                    // venue, the day, an open window from the start of
                    // history, the citation gap and the remedy.
                    assert_eq!(
                        refused,
                        ChargeError::Cost(CostError::Unverified(Refusal::with_remediation(
                            want,
                            Some(exchange),
                            when,
                            TradeDay::MIN,
                            None,
                            NO_DATED_HISTORY,
                            NO_DATED_HISTORY_REMEDY,
                        )))
                    );
                    assert!(refused.to_string().contains("CurrentOnEveryDate"));
                }
            }
        }
    }

    #[test]
    fn the_dated_option_card_is_the_regime_on_the_trade_day_plus_the_ipft_on_every_day() {
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
        // The same total from the other split: FA64232's row and the ₹50 IPFT
        // (D-5100's dated IPFT, read where crate::trip reads it).
        assert_eq!(
            (
                exchange_charge_rate(Exchange::Nse, day(2026, 2, 28))
                    .map(crate::rate::BpsX100::get),
                crate::regime::ipft_rate(Exchange::Nse, day(2026, 2, 28))
                    .map(crate::rate::BpsX100::get),
            ),
            (Ok(3_503), Ok(50))
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
        // From FA73061 on: still the regime row plus the IPFT. The row in
        // force is FA73061's folded 0.03553% and the dated IPFT beside it is
        // zero (CHG-0, D-5100), so the card is 0.03553%, the total FA73061
        // states (sources row E-03). Dropping the IPFT by date charged
        // 0.03503% here while the regime still held FA64232's row, labelled
        // as cited (D-5131); keeping the undated ₹50 beside the new row would
        // charge 0.03603% (D-5101).
        for on in [FA73061_EFFECTIVE, day(2026, 10, 15), CURRENT_AS_OF] {
            let from = dated(Exchange::Nse, on);
            let regime_now = exchange_charge_rate(Exchange::Nse, on).expect("verified");
            let ipft_now = crate::regime::ipft_rate(Exchange::Nse, on).expect("verified");
            assert_eq!(
                (regime_now.get(), ipft_now.get()),
                (3_553, 0),
                "the rows in force on {on}: CHG-0's FA73061 row and the folded zero IPFT"
            );
            assert_eq!(
                from.exchange_charge().rate(),
                Ratio::new(3_553, RATE_SCALE),
                "{on}"
            );
            // Today's table carries it per hundred crore: the same value.
            let (got, today) = (from.exchange_charge().rate(), NSE_OPTIONS_EXCHANGE.rate());
            assert_eq!(
                i128::from(got.numerator()) * i128::from(today.denominator()),
                i128::from(today.numerator()) * i128::from(got.denominator()),
                "the dated card and today's table agree on {on}"
            );
            assert_eq!(from.exchange_charge().status(), Status::Cited);
        }
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
    fn the_dated_option_card_is_the_trip_stacks_exchange_charge_plus_ipft_on_every_day() {
        // One dated figure, two readers: the CHG-1 dated card and the round
        // trip of crate::trip. On every day from 2018-01-01 to 2027-12-31 at
        // both venues they price together or refuse together, and where they
        // price, the card's exchange charge is trip's exchange rate plus its
        // IPFT, to the unit (D-5131). The card that dropped the IPFT from
        // 2026-03-01 fails here on 2026-03-01 at NSE.
        let (mut priced, mut refused) = (0_u32, 0_u32);
        let mut on = day(2018, 1, 1);
        while on.before(day(2028, 1, 1)) {
            for exchange in [Exchange::Nse, Exchange::Bse] {
                let card = RateCard::resolve(TradeKind::Options, exchange, on, RateSelector::Dated);
                let trip = crate::trip::Rates::resolve(crate::rate::Broker::Zerodha, exchange, on);
                match (card, trip) {
                    (Ok(card), Ok(trip)) => {
                        priced += 1;
                        assert_eq!(
                            card.exchange_charge().rate(),
                            Ratio::new(trip.exchange().get() + trip.ipft().get(), RATE_SCALE),
                            "{exchange:?} on {on}"
                        );
                        let Taxes::Options { stt_sell, .. } = card.taxes() else {
                            panic!("an options card carries option taxes");
                        };
                        assert_eq!(
                            stt_sell.rate(),
                            Ratio::new(trip.stt().get(), RATE_SCALE),
                            "{exchange:?} on {on}"
                        );
                    }
                    (Err(_), Err(_)) => refused += 1,
                    (card, trip) => panic!(
                        "{exchange:?} on {on}: the dated card {card:?} and trip {trip:?} \
                         disagree on whether the day has a rate"
                    ),
                }
            }
            on = on.plus_days(1).expect("inside the calendar");
        }
        // 2024-10-01 to 2027-12-31 prices at both venues (1,187 days);
        // 2018-01-01 to 2024-09-30 refuses at both (2,465 days).
        assert_eq!((priced, refused), (1_187 * 2, 2_465 * 2));
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
        assert_eq!(Mode::default(), Mode::Precise);
        assert_eq!(
            [Mode::Precise.label(), Mode::DhanCalculator.label()],
            [
                "precise (contract-note method)",
                "Dhan calculator (estimate)"
            ]
        );
        assert_eq!(
            [Mode::Precise.version(), Mode::DhanCalculator.version()],
            ["precise/r2", "dhan-calculator/2026-10-16/r1"]
        );
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

    /// FNV-1a over a stream of integers: a fingerprint, not a security hash.
    fn fold(hash: u64, value: i64) -> u64 {
        value.to_le_bytes().iter().fold(hash, |h, byte| {
            (h ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
        })
    }

    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

    /// Every rate a resolved card carries, folded; a refusal folds a marker.
    fn fold_card(hash: u64, card: &Result<RateCard, ChargeError>) -> u64 {
        match card {
            Ok(card) => card.rates().iter().fold(hash, |h, rate| match rate {
                Some(rate) => fold(fold(h, rate.rate().numerator()), rate.rate().denominator()),
                None => fold(h, -1),
            }),
            Err(_) => fold(hash, -2),
        }
    }

    /// Today's table, as every card resolves it, and Dhan's own fees.
    fn todays_fingerprint() -> u64 {
        let mut hash = FNV_OFFSET;
        for kind in ALL_KINDS {
            for exchange in [Exchange::Nse, Exchange::Bse] {
                hash = fold_card(
                    hash,
                    &RateCard::resolve(
                        kind,
                        exchange,
                        CURRENT_AS_OF,
                        RateSelector::CurrentOnEveryDate,
                    ),
                );
            }
        }
        for figure in [
            DHAN_EQUITY_BROKERAGE_RATE.rate().numerator(),
            DHAN_EQUITY_BROKERAGE_RATE.rate().denominator(),
            DHAN_EQUITY_BROKERAGE_CAP.raw(),
            DHAN_FNO_BROKERAGE.raw(),
            DHAN_DP_CHARGE.raw(),
            DHAN_SQUARE_OFF_FEE.raw(),
        ] {
            hash = fold(hash, figure);
        }
        hash
    }

    /// The dated set: the options card at both venues on every day from
    /// 2018-01-01 to 2027-12-31 (a refusal included), and the dated refusal
    /// of every other kind.
    fn dated_fingerprint() -> u64 {
        let mut hash = FNV_OFFSET;
        let mut on = day(2018, 1, 1);
        while on.before(day(2028, 1, 1)) {
            for kind in ALL_KINDS {
                for exchange in [Exchange::Nse, Exchange::Bse] {
                    hash = fold_card(
                        hash,
                        &RateCard::resolve(kind, exchange, on, RateSelector::Dated),
                    );
                }
            }
            on = on.plus_days(1).expect("inside the calendar");
        }
        hash
    }

    #[test]
    fn the_rate_set_versions_are_pinned_to_the_tables_they_name() {
        // A run identity carries `RateSelector::version` (D-5127). If a rate
        // in a table changes and its version does not, two runs priced at
        // different rates would carry one identity. So each version is pinned
        // beside a fingerprint of the table it names: change a rate and this
        // fails until the version moves too (bump the revision `rN`, and
        // re-pin the fingerprint here in the same change).
        // One assertion, so a failure prints both fingerprints.
        assert_eq!(
            [
                (
                    RateSelector::CurrentOnEveryDate.version(),
                    todays_fingerprint()
                ),
                (RateSelector::Dated.version(), dated_fingerprint()),
            ],
            [
                ("todays-rates/2026-10-16/r1", TODAYS_FINGERPRINT),
                ("dated-regime/r2", DATED_FINGERPRINT),
            ],
            "a rate table changed: bump its version (TODAYS_RATES_VERSION or \
             DATED_RATES_VERSION) and re-pin its fingerprint"
        );
        assert_ne!(TODAYS_RATES_VERSION, DATED_RATES_VERSION);
        assert_ne!(PRECISE_VERSION, DHAN_CALCULATOR_VERSION);
        // The fingerprint moves with a single rate: the check can fail.
        let moved = fold(fold(FNV_OFFSET, 30_700), 100 * CRORE);
        assert_ne!(moved, fold(fold(FNV_OFFSET, 30_699), 100 * CRORE));
    }

    /// `todays_fingerprint()` on the r1 table.
    const TODAYS_FINGERPRINT: u64 = 4_405_649_805_164_050_705;

    /// `dated_fingerprint()` on the r2 regime (r1 dropped the NSE options
    /// IPFT from 2026-03-01; D-5131).
    const DATED_FINGERPRINT: u64 = 11_353_183_212_436_303_133;
}
