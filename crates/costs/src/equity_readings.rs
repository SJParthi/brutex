//! One cash-equity intraday contract note under every reading Dhan's words
//! leave open, and the worst of them (package FNO-COSTS, D-8112).
//!
//! # The open choices
//!
//! Operator rule 7 (30 Oct 2026): wherever Dhan's own words leave a choice
//! open, compute each reading and use the worse until a Dhan contract note or
//! written statement settles it. COSTS.md section 6 lists them, and each is a
//! field of [`ReadingSet`]:
//!
//! | Field | Row | Readings |
//! |---|---|---|
//! | `brokerage` | RD-BRK | per executed order; per fill (each traded second an order filled in) |
//! | `stt` | RD-STT-BASE (and RD-ROUND-TAX per order) | NSE's VWAP basis; the sell value; the sell value per sell order, each to the rupee |
//! | `stamp` | RD-ROUND-TAX | the client-day to the rupee; the security-day; each buy order |
//! | `exchange` | RD-ROUND-EX | one line; transaction charge and IPFT as two lines; per order; per order in two lines |
//! | `sebi` | RD-ROUND-EX | one line; per order |
//! | `gst`, `dp_gst` | RD-GST | 18% once; CGST 9% and SGST 9% each rounded |
//! | `dp` | RD-DP | none; Rs 12.50 once per scrip-day with a sell |
//! | `square_off` | RD-SQ | the fee on top of the order's brokerage; instead of it |
//! | `square_off_split` | SPEC.md OP-27 | one fee per broker order; one per traded second it filled in |
//! | `state`, `state_rounding` | RD-STATE | each listed State, to the paisa or the rupee |
//! | `slab` | RD-SLAB | the top (first) slab — the only one the table carries |
//!
//! RD-TIE (the quantity at an exact half) is not a charge reading: the
//! quantity is an input here, and FNO-NET pins it; CN22 and CN23 price both.
//!
//! # The worst
//!
//! Every line is a sum of non-negative terms, and GST is non-decreasing in its
//! base, so the largest total over every combination of readings is each
//! line at its own largest reading, with GST on that largest base. [`worst`]
//! returns that, and the [`ReadingSet`] that reproduces it through [`note`]
//! (`the_worst_set_reproduces_the_worst_lines`). Taking each line's worst
//! alone is at least as bad as any one consistent rounding level, so it can
//! only over-state, never flatter, a trade (D-8112).
//!
//! # Money
//!
//! Every line is computed in `i128` paise, exactly, from `i64` fills, then
//! rounded the way its source says: half-up to the paisa, and STT and stamp
//! duty half-up to the rupee after that. No float anywhere.

use brutex_core::price::Paisa;

use crate::charge_error::ChargeError;
use crate::day::TradeDay;
use crate::equity_clock::{Clock, TradeFills};
use crate::equity_dated::{
    BROKERAGE_CAP, BROKERAGE_RATE, DP_CHARGE, EquityRates, SQUARE_OFF_FEE, StateStamp, half_up,
    levy, narrow_fitting, worst_state_units,
};
use crate::error::CostError;
use crate::exact::Ratio;
use crate::rate::OrderSide;

/// The most fills one trade may carry: 131,072. An order fills at most once
/// per traded second, and a session has 22,500 seconds, so this is several
/// sessions of fills on every order of a trade. It is the bound that keeps
/// every line inside `i64` (brokerage per fill at most Rs 20 each).
pub const MAX_FILLS: usize = 1 << 17;

/// GST's central half, 9% — Notification 11/2017-Central Tax (Rate), the
/// GST-1 row's source. The State half is the same rate.
const CGST: Ratio = Ratio::new(9, 100);

/// GST, 18% (GST-1, and GST-SEBI on the SEBI fee).
const GST: Ratio = Ratio::new(18, 100);

/// Why a square-off order on the opening side is refused.
const SQUARE_OFF_ON_OPENING_SIDE: &str = "a broker square-off order on the side that opened the \
     trade (the broker only ever closes an open intraday position)";

/// Why a trade over [`MAX_FILLS`] is refused.
const TOO_MANY_FILLS: &str = "every order fills at most once per traded second and a session has \
     22,500 seconds; above 131,072 fills the per-fill brokerage could leave the bound that keeps \
     every line inside i64";

// ---------------------------------------------------------------------------
// The input: one trade's orders, summed and checked
// ---------------------------------------------------------------------------

/// One order's totals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrderTotals {
    /// Buy or sell.
    pub side: OrderSide,
    /// Whether Dhan placed it to square off (RD-SQ).
    pub by_broker: bool,
    /// Units filled.
    pub quantity: i64,
    /// Value filled, in paise.
    pub value: i64,
    /// How many fills (traded seconds).
    pub fills: usize,
    /// Brokerage charged per fill, summed (RD-BRK per fill), in paise.
    pub brokerage_per_fill: i64,
}

/// One client's one stock on one day, squared off: what a contract note is
/// computed from. Built only by [`NoteInput::from_fills`], which checks it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NoteInput {
    day: TradeDay,
    orders: Vec<OrderTotals>,
    quantity: i64,
    buy_value: i64,
    sell_value: i64,
}

/// Dhan's brokerage on one value: the lower of Rs 20 and 0.03%, half-up to
/// the paisa (BRK-1; `crate::schedule::dhan_brokerage` exactly).
fn brokerage(value: i128) -> i128 {
    levy(value, BROKERAGE_RATE).min(i128::from(BROKERAGE_CAP.raw()))
}

/// Half-up to the rupee, in paise.
fn rupee(paise: i128) -> i128 {
    half_up(paise, 100) * 100
}

/// A non-positive figure, refused by name.
const fn not_positive(quantity: &'static str, value: i64) -> ChargeError {
    ChargeError::Cost(CostError::NotPositive { quantity, value })
}

/// An overflow, refused by name.
const fn overflow(operation: &'static str) -> ChargeError {
    ChargeError::Cost(CostError::Overflow { operation })
}

impl NoteInput {
    /// Sums one trade's fills into its contract-note input, refusing what no
    /// contract note could carry.
    ///
    /// The orders are in time order, and the first opens the trade. Each
    /// order's fills are in strictly increasing seconds (one fill per traded
    /// second, as the seconds walk gives them).
    ///
    /// # Errors
    ///
    /// * [`ChargeError::EmptyDay`] for a trade with no order.
    /// * [`ChargeError::MismatchedLegs`] for a broker square-off order on
    ///   the side that opened the trade.
    /// * [`ChargeError::OutOfScope`] for more than [`MAX_FILLS`] fills.
    /// * [`ChargeError::Cost`] carrying [`CostError::NotPositive`] for an
    ///   order with no fill, a fill quantity or price that is not positive,
    ///   or a fill not later than its order's previous fill; or carrying
    ///   [`CostError::Overflow`] for a fill value, a side's quantity or a
    ///   side's value that leaves `i64`.
    /// * [`ChargeError::OpenIntradayPosition`] when the bought and sold
    ///   quantities differ: an intraday trade is squared off the same day.
    pub fn from_fills(fills: &TradeFills, day: TradeDay) -> Result<Self, ChargeError> {
        let Some(first) = fills.orders.first() else {
            return Err(ChargeError::EmptyDay);
        };
        let opening = first.side;
        let mut count = 0_usize;
        let mut orders = Vec::with_capacity(fills.orders.len());
        let (mut bought, mut sold) = (0_i64, 0_i64);
        let (mut buy_value, mut sell_value) = (0_i64, 0_i64);
        for order in &fills.orders {
            if order.by_broker && order.side == opening {
                return Err(ChargeError::MismatchedLegs {
                    why: SQUARE_OFF_ON_OPENING_SIDE,
                });
            }
            if order.fills.is_empty() {
                return Err(not_positive("fills in an order", 0));
            }
            count += order.fills.len();
            if count > MAX_FILLS {
                return Err(ChargeError::OutOfScope {
                    what: "a trade of more than 131,072 fills",
                    reason: TOO_MANY_FILLS,
                });
            }
            let mut totals = OrderTotals {
                side: order.side,
                by_broker: order.by_broker,
                quantity: 0,
                value: 0,
                fills: order.fills.len(),
                brokerage_per_fill: 0,
            };
            let mut last: Option<i64> = None;
            for fill in &order.fills {
                if fill.quantity <= 0 {
                    return Err(not_positive("fill quantity", fill.quantity));
                }
                if fill.price.raw() <= 0 {
                    return Err(not_positive("fill price", fill.price.raw()));
                }
                if let Some(previous) = last
                    && fill.second <= previous
                {
                    let step = i128::from(fill.second) - i128::from(previous);
                    return Err(not_positive(
                        "seconds from the order's previous fill",
                        i64::try_from(step).unwrap_or(i64::MIN),
                    ));
                }
                last = Some(fill.second);
                let value = fill
                    .quantity
                    .checked_mul(fill.price.raw())
                    .ok_or(overflow("a fill's value"))?;
                let (side_quantity, side_value) = match order.side {
                    OrderSide::Buy => (&mut bought, &mut buy_value),
                    OrderSide::Sell => (&mut sold, &mut sell_value),
                };
                *side_quantity = side_quantity
                    .checked_add(fill.quantity)
                    .ok_or(overflow("a side's quantity"))?;
                *side_value = side_value
                    .checked_add(value)
                    .ok_or(overflow("a side's value"))?;
                // At most the side's checked totals, and at most Rs 20 a fill
                // for at most MAX_FILLS fills: none of the three can overflow.
                totals.quantity += fill.quantity;
                totals.value += value;
                totals.brokerage_per_fill += narrow_fitting(brokerage(i128::from(value))).raw();
            }
            orders.push(totals);
        }
        if bought != sold {
            return Err(ChargeError::OpenIntradayPosition { bought, sold });
        }
        Ok(Self {
            day,
            orders,
            quantity: bought,
            buy_value,
            sell_value,
        })
    }

    /// The trade day.
    #[must_use]
    pub const fn day(&self) -> TradeDay {
        self.day
    }

    /// Each order's totals, in order.
    #[must_use]
    pub fn orders(&self) -> &[OrderTotals] {
        &self.orders
    }

    /// The squared quantity: bought, and sold.
    #[must_use]
    pub const fn quantity(&self) -> i64 {
        self.quantity
    }

    /// The day's buy value.
    #[must_use]
    pub const fn buy_value(&self) -> Paisa {
        Paisa::from_raw(self.buy_value)
    }

    /// The day's sell value.
    #[must_use]
    pub const fn sell_value(&self) -> Paisa {
        Paisa::from_raw(self.sell_value)
    }

    /// NSE's day VWAP: `(buy value + sell value) / (2 × quantity)`, half-up to
    /// the paisa (NSE/CMTR/5588), the price STT's official base is taken at.
    #[must_use]
    pub fn vwap(&self) -> Paisa {
        let turnover = i128::from(self.buy_value) + i128::from(self.sell_value);
        narrow_fitting(half_up(turnover, 2 * i128::from(self.quantity)))
    }

    /// The trade's result before any charge: the sell value less the buy
    /// value, long or short ("signal only, before charges", still at the
    /// worst-price fills). Both values are positive `i64`s, so the
    /// difference fits.
    #[must_use]
    pub const fn gross(&self) -> Paisa {
        Paisa::from_raw(self.sell_value - self.buy_value)
    }
}

// ---------------------------------------------------------------------------
// The readings
// ---------------------------------------------------------------------------

/// RD-BRK.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BrokerageReading {
    /// Once per executed order (Dhan's words).
    PerOrder,
    /// Once per fill: each traded second an order filled in.
    PerFill,
}

/// RD-STT-BASE, with RD-ROUND-TAX's per-order level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SttReading {
    /// NSE/CMTR/5588: the squared quantity at the day's VWAP.
    Vwap,
    /// Dhan's page: 0.025% of the sell value.
    SellValue,
    /// The sell value of each sell order, each to the rupee.
    SellValuePerOrder,
}

/// RD-ROUND-TAX for the unified stamp duty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaxRounding {
    /// The client's day total, to the rupee (NCL/CMPT/43115).
    ClientDay,
    /// Each security's day, to the rupee: the client-day itself, since a
    /// note here is one security's day.
    SecurityDay,
    /// Each buy order, to the rupee.
    PerOrder,
}

/// RD-ROUND-EX for the exchange line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExchangeRounding {
    /// The transaction charge and the IPFT as one line, rounded once.
    Once,
    /// The transaction charge and the IPFT as two lines.
    Split,
    /// One line per order.
    PerOrder,
    /// Two lines per order.
    PerOrderSplit,
}

/// RD-ROUND-EX for the SEBI fee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SebiRounding {
    /// Once a day.
    Once,
    /// Once per order.
    PerOrder,
}

/// RD-GST.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GstRounding {
    /// 18% of the base, rounded once.
    Once,
    /// CGST 9% and SGST 9%, each rounded to the paisa.
    CgstSgst,
}

/// RD-DP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DpReading {
    /// No DP charge: nothing left the demat account.
    NotCharged,
    /// Rs 12.50 + GST once per scrip-day with a sell.
    Intraday,
}

/// RD-SQ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SquareOffReading {
    /// Rs 20 + GST on top of the square-off order's brokerage.
    OnTop,
    /// Rs 20 + GST instead of it.
    Instead,
}

/// SPEC.md OP-27's "square-off split": whether an auto square-off that fills
/// in several seconds is one order or several, for its fee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SquareOffSplit {
    /// One fee per order the broker placed (Dhan's words: "₹ 20 / order").
    OnePerOrder,
    /// One fee per traded second a broker order filled in.
    OnePerFill,
}

/// RD-STATE: which State's stamp duty, before 1 Jul 2020.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateChoice {
    /// The highest of the listed States for the day's value (operator
    /// decision 9.5).
    Worst,
    /// One named State.
    Named(&'static StateStamp),
}

/// RD-STATE's rounding (the State schedules were not read).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StateRounding {
    /// To the paisa.
    Paisa,
    /// To the rupee.
    Rupee,
}

/// RD-SLAB: the NSE slab before 1 Oct 2024. The table carries the top slab
/// only — the highest rate a client could be passed — so it is the one
/// reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SlabReading {
    /// The top (first) slab.
    Top,
}

/// One reading of every open choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReadingSet {
    /// RD-BRK.
    pub brokerage: BrokerageReading,
    /// RD-STT-BASE.
    pub stt: SttReading,
    /// RD-ROUND-TAX for the unified stamp duty.
    pub stamp: TaxRounding,
    /// RD-ROUND-EX for the exchange line.
    pub exchange: ExchangeRounding,
    /// RD-ROUND-EX for the SEBI fee.
    pub sebi: SebiRounding,
    /// RD-GST for the main GST line and the square-off fee's GST.
    pub gst: GstRounding,
    /// RD-DP.
    pub dp: DpReading,
    /// RD-GST for the DP charge's GST.
    pub dp_gst: GstRounding,
    /// RD-SQ.
    pub square_off: SquareOffReading,
    /// SPEC.md OP-27's square-off split.
    pub square_off_split: SquareOffSplit,
    /// RD-STATE.
    pub state: StateChoice,
    /// RD-STATE's rounding.
    pub state_rounding: StateRounding,
    /// RD-SLAB.
    pub slab: SlabReading,
}

impl ReadingSet {
    /// The official method (COSTS.md section 5): one brokerage per order,
    /// NSE's VWAP basis, the client-day rounding, one exchange line, GST
    /// once, no DP, the square-off fee on top, the worst listed State to the
    /// paisa and the top slab.
    pub const OFFICIAL: Self = Self {
        brokerage: BrokerageReading::PerOrder,
        stt: SttReading::Vwap,
        stamp: TaxRounding::ClientDay,
        exchange: ExchangeRounding::Once,
        sebi: SebiRounding::Once,
        gst: GstRounding::Once,
        dp: DpReading::NotCharged,
        dp_gst: GstRounding::Once,
        square_off: SquareOffReading::OnTop,
        square_off_split: SquareOffSplit::OnePerOrder,
        state: StateChoice::Worst,
        state_rounding: StateRounding::Paisa,
        slab: SlabReading::Top,
    };

    /// Each reading's label, `RD-<row> <reading>`, in field order: what a run
    /// records beside each trade.
    #[must_use]
    pub const fn labels(&self) -> [&'static str; 13] {
        [
            match self.brokerage {
                BrokerageReading::PerOrder => "RD-BRK per-order",
                BrokerageReading::PerFill => "RD-BRK per-fill",
            },
            match self.stt {
                SttReading::Vwap => "RD-STT-BASE vwap",
                SttReading::SellValue => "RD-STT-BASE sell-value",
                SttReading::SellValuePerOrder => "RD-STT-BASE sell-value-per-order",
            },
            match self.stamp {
                TaxRounding::ClientDay => "RD-ROUND-TAX client-day",
                TaxRounding::SecurityDay => "RD-ROUND-TAX security-day",
                TaxRounding::PerOrder => "RD-ROUND-TAX per-order",
            },
            match self.exchange {
                ExchangeRounding::Once => "RD-ROUND-EX exchange-once",
                ExchangeRounding::Split => "RD-ROUND-EX exchange-split",
                ExchangeRounding::PerOrder => "RD-ROUND-EX exchange-per-order",
                ExchangeRounding::PerOrderSplit => "RD-ROUND-EX exchange-per-order-split",
            },
            match self.sebi {
                SebiRounding::Once => "RD-ROUND-EX sebi-once",
                SebiRounding::PerOrder => "RD-ROUND-EX sebi-per-order",
            },
            match self.gst {
                GstRounding::Once => "RD-GST once",
                GstRounding::CgstSgst => "RD-GST cgst-sgst",
            },
            match self.dp {
                DpReading::NotCharged => "RD-DP none",
                DpReading::Intraday => "RD-DP intraday",
            },
            match self.dp_gst {
                GstRounding::Once => "RD-GST dp-once",
                GstRounding::CgstSgst => "RD-GST dp-cgst-sgst",
            },
            match self.square_off {
                SquareOffReading::OnTop => "RD-SQ on-top",
                SquareOffReading::Instead => "RD-SQ instead",
            },
            match self.square_off_split {
                SquareOffSplit::OnePerOrder => "RD-SQ one-fee-per-order",
                SquareOffSplit::OnePerFill => "RD-SQ one-fee-per-fill",
            },
            match self.state {
                StateChoice::Worst => "RD-STATE worst",
                StateChoice::Named(state) => state.code,
            },
            match self.state_rounding {
                StateRounding::Paisa => "RD-STATE paisa",
                StateRounding::Rupee => "RD-STATE rupee",
            },
            match self.slab {
                SlabReading::Top => "RD-SLAB top",
            },
        ]
    }
}

// ---------------------------------------------------------------------------
// The lines
// ---------------------------------------------------------------------------

/// The lines' names, in [`NoteLines::lines`] order (COSTS.md's vector rows).
pub const NOTE_LINE_NAMES: [&str; 11] = [
    "brokerage",
    "NSE transaction charge + IPFT",
    "SEBI turnover fee",
    "STT",
    "stamp duty",
    "GST",
    "DP charge",
    "GST on DP",
    "Dhan auto square-off fee",
    "GST on square-off fee",
    "total charges",
];

/// One contract note's lines, in paise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteLines {
    /// Dhan's brokerage.
    pub brokerage: Paisa,
    /// NSE's transaction charge and the IPFT.
    pub exchange: Paisa,
    /// SEBI's turnover fee.
    pub sebi: Paisa,
    /// STT.
    pub stt: Paisa,
    /// Stamp duty.
    pub stamp: Paisa,
    /// GST on brokerage, the exchange line and (from 18 Jul 2022) the SEBI
    /// fee.
    pub gst: Paisa,
    /// The DP charge.
    pub dp: Paisa,
    /// GST on the DP charge.
    pub dp_gst: Paisa,
    /// Dhan's auto square-off fee.
    pub square_off: Paisa,
    /// GST on the square-off fee.
    pub square_off_gst: Paisa,
    /// Every line above.
    pub total: Paisa,
}

impl NoteLines {
    /// The lines in [`NOTE_LINE_NAMES`] order, the total last.
    #[must_use]
    pub const fn lines(&self) -> [Paisa; 11] {
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
            self.total,
        ]
    }
}

/// Every candidate figure of every line, computed once per note and clock.
struct Facts {
    brokerage_per_order: i128,
    brokerage_per_fill: i128,
    own_brokerage_per_order: i128,
    own_brokerage_per_fill: i128,
    exchange_once: i128,
    exchange_split: i128,
    exchange_per_order: i128,
    exchange_per_order_split: i128,
    sebi_once: i128,
    sebi_per_order: i128,
    stt_vwap: i128,
    stt_sell: i128,
    stt_sell_per_order: i128,
    state: bool,
    turnover: i128,
    stamp_day: i128,
    stamp_per_order: i128,
    square_off_per_order: i128,
    square_off_per_fill: i128,
    gst_on_sebi: bool,
}

/// GST on a base under one reading.
fn gst_on(base: i128, reading: GstRounding) -> i128 {
    match reading {
        GstRounding::Once => levy(base, GST),
        GstRounding::CgstSgst => 2 * levy(base, CGST),
    }
}

/// The first of the largest: `first` unless a later option is strictly
/// larger, so a tie keeps the official reading.
fn largest<R: Copy>(first: (R, i128), rest: &[(R, i128)]) -> (R, i128) {
    rest.iter().fold(
        first,
        |best, &next| if next.1 > best.1 { next } else { best },
    )
}

impl Facts {
    fn of(input: &NoteInput, rates: &EquityRates) -> Self {
        let exchange = rates.exchange_rate();
        let nse = rates.nse.rate.ratio();
        let ipft = rates.ipft.rate.ratio();
        let sebi = rates.sebi.rate.ratio();
        let stt = rates.stt.rate.ratio();
        let stamp = rates.stamp.rate.ratio();
        let mut facts = Self {
            brokerage_per_order: 0,
            brokerage_per_fill: 0,
            own_brokerage_per_order: 0,
            own_brokerage_per_fill: 0,
            exchange_once: 0,
            exchange_split: 0,
            exchange_per_order: 0,
            exchange_per_order_split: 0,
            sebi_once: 0,
            sebi_per_order: 0,
            stt_vwap: 0,
            stt_sell: 0,
            stt_sell_per_order: 0,
            state: rates.stamp_is_state(),
            turnover: i128::from(input.buy_value) + i128::from(input.sell_value),
            stamp_day: rupee(levy(i128::from(input.buy_value), stamp)),
            stamp_per_order: 0,
            square_off_per_order: 0,
            square_off_per_fill: 0,
            gst_on_sebi: rates.gst_on_sebi.is_some(),
        };
        for order in &input.orders {
            let value = i128::from(order.value);
            let per_order = brokerage(value);
            let per_fill = i128::from(order.brokerage_per_fill);
            facts.brokerage_per_order += per_order;
            facts.brokerage_per_fill += per_fill;
            if order.by_broker {
                let fee = i128::from(SQUARE_OFF_FEE.raw());
                facts.square_off_per_order += fee;
                // At most MAX_FILLS fills, so the product fits.
                facts.square_off_per_fill += fee * i128::try_from(order.fills).unwrap_or(0);
            } else {
                facts.own_brokerage_per_order += per_order;
                facts.own_brokerage_per_fill += per_fill;
            }
            facts.exchange_per_order += levy(value, exchange);
            facts.exchange_per_order_split += levy(value, nse) + levy(value, ipft);
            facts.sebi_per_order += levy(value, sebi);
            match order.side {
                OrderSide::Sell => facts.stt_sell_per_order += rupee(levy(value, stt)),
                OrderSide::Buy => facts.stamp_per_order += rupee(levy(value, stamp)),
            }
        }
        let turnover = facts.turnover;
        facts.exchange_once = levy(turnover, exchange);
        facts.exchange_split = levy(turnover, nse) + levy(turnover, ipft);
        facts.sebi_once = levy(turnover, sebi);
        // NSE/CMTR/5588: VWAP = (BVAL + SVAL) / (BQTY + SQTY) to the paisa,
        // the squared quantity at it, the tax to the paisa, then the rupee.
        let quantity = i128::from(input.quantity);
        let vwap = half_up(turnover, 2 * quantity);
        facts.stt_vwap = rupee(levy(vwap * quantity, stt));
        facts.stt_sell = rupee(levy(i128::from(input.sell_value), stt));
        facts
    }

    /// The State duty under one choice and rounding.
    fn state_duty(&self, choice: StateChoice, rounding: StateRounding) -> i128 {
        let duty = match choice {
            StateChoice::Worst => worst_state_units(self.turnover).1,
            StateChoice::Named(state) => state.levy_on(self.turnover),
        };
        match rounding {
            StateRounding::Paisa => duty,
            StateRounding::Rupee => rupee(duty),
        }
    }

    /// The lines under one reading set.
    fn pick(&self, readings: &ReadingSet) -> Lines {
        let brokerage = match (readings.brokerage, readings.square_off) {
            (BrokerageReading::PerOrder, SquareOffReading::OnTop) => self.brokerage_per_order,
            (BrokerageReading::PerFill, SquareOffReading::OnTop) => self.brokerage_per_fill,
            (BrokerageReading::PerOrder, SquareOffReading::Instead) => self.own_brokerage_per_order,
            (BrokerageReading::PerFill, SquareOffReading::Instead) => self.own_brokerage_per_fill,
        };
        let exchange = match readings.exchange {
            ExchangeRounding::Once => self.exchange_once,
            ExchangeRounding::Split => self.exchange_split,
            ExchangeRounding::PerOrder => self.exchange_per_order,
            ExchangeRounding::PerOrderSplit => self.exchange_per_order_split,
        };
        let sebi = match readings.sebi {
            SebiRounding::Once => self.sebi_once,
            SebiRounding::PerOrder => self.sebi_per_order,
        };
        let stt = match readings.stt {
            SttReading::Vwap => self.stt_vwap,
            SttReading::SellValue => self.stt_sell,
            SttReading::SellValuePerOrder => self.stt_sell_per_order,
        };
        let stamp = if self.state {
            self.state_duty(readings.state, readings.state_rounding)
        } else {
            match readings.stamp {
                TaxRounding::ClientDay | TaxRounding::SecurityDay => self.stamp_day,
                TaxRounding::PerOrder => self.stamp_per_order,
            }
        };
        let dp = match readings.dp {
            DpReading::NotCharged => 0,
            DpReading::Intraday => i128::from(DP_CHARGE.raw()),
        };
        let square_off = match readings.square_off_split {
            SquareOffSplit::OnePerOrder => self.square_off_per_order,
            SquareOffSplit::OnePerFill => self.square_off_per_fill,
        };
        Lines::of(
            [brokerage, exchange, sebi, stt, stamp],
            self.gst_on_sebi,
            readings.gst,
            dp,
            readings.dp_gst,
            square_off,
        )
    }

    /// The worst lines, and the reading set that gives them.
    fn worst(&self) -> (Lines, ReadingSet) {
        let (brokerage_reading, brokerage) = largest(
            (BrokerageReading::PerOrder, self.brokerage_per_order),
            &[(BrokerageReading::PerFill, self.brokerage_per_fill)],
        );
        let (exchange_reading, exchange) = largest(
            (ExchangeRounding::Once, self.exchange_once),
            &[
                (ExchangeRounding::Split, self.exchange_split),
                (ExchangeRounding::PerOrder, self.exchange_per_order),
                (
                    ExchangeRounding::PerOrderSplit,
                    self.exchange_per_order_split,
                ),
            ],
        );
        let (sebi_reading, sebi) = largest(
            (SebiRounding::Once, self.sebi_once),
            &[(SebiRounding::PerOrder, self.sebi_per_order)],
        );
        let (stt_reading, stt) = largest(
            (SttReading::Vwap, self.stt_vwap),
            &[
                (SttReading::SellValue, self.stt_sell),
                (SttReading::SellValuePerOrder, self.stt_sell_per_order),
            ],
        );
        let (stamp_reading, state_rounding, stamp) = if self.state {
            let (rounding, duty) = largest(
                (
                    StateRounding::Paisa,
                    self.state_duty(StateChoice::Worst, StateRounding::Paisa),
                ),
                &[(
                    StateRounding::Rupee,
                    self.state_duty(StateChoice::Worst, StateRounding::Rupee),
                )],
            );
            (TaxRounding::ClientDay, rounding, duty)
        } else {
            let (rounding, duty) = largest(
                (TaxRounding::ClientDay, self.stamp_day),
                &[(TaxRounding::PerOrder, self.stamp_per_order)],
            );
            (rounding, StateRounding::Paisa, duty)
        };
        let (split_reading, square_off) = largest(
            (SquareOffSplit::OnePerOrder, self.square_off_per_order),
            &[(SquareOffSplit::OnePerFill, self.square_off_per_fill)],
        );
        let gst_base = brokerage + exchange + if self.gst_on_sebi { sebi } else { 0 };
        let (gst_reading, _) = largest(
            (GstRounding::Once, gst_on(gst_base, GstRounding::Once)),
            &[(
                GstRounding::CgstSgst,
                gst_on(gst_base, GstRounding::CgstSgst),
            )],
        );
        let dp = i128::from(DP_CHARGE.raw());
        let (dp_gst_reading, _) = largest(
            (GstRounding::Once, gst_on(dp, GstRounding::Once)),
            &[(GstRounding::CgstSgst, gst_on(dp, GstRounding::CgstSgst))],
        );
        let readings = ReadingSet {
            brokerage: brokerage_reading,
            stt: stt_reading,
            stamp: stamp_reading,
            exchange: exchange_reading,
            sebi: sebi_reading,
            gst: gst_reading,
            dp: DpReading::Intraday,
            dp_gst: dp_gst_reading,
            square_off: SquareOffReading::OnTop,
            square_off_split: split_reading,
            state: StateChoice::Worst,
            state_rounding,
            slab: SlabReading::Top,
        };
        let lines = Lines::of(
            [brokerage, exchange, sebi, stt, stamp],
            self.gst_on_sebi,
            gst_reading,
            dp,
            dp_gst_reading,
            square_off,
        );
        (lines, readings)
    }
}

/// The lines in `i128` paise, before the narrowing.
struct Lines([i128; 11]);

impl Lines {
    /// GST, the DP charge's GST and the square-off fee's GST on top of the
    /// five rounded lines, and the total.
    fn of(
        [brokerage, exchange, sebi, stt, stamp]: [i128; 5],
        gst_on_sebi: bool,
        gst: GstRounding,
        dp: i128,
        dp_gst: GstRounding,
        square_off: i128,
    ) -> Self {
        let base = brokerage + exchange + if gst_on_sebi { sebi } else { 0 };
        let gst_line = gst_on(base, gst);
        let dp_gst_line = gst_on(dp, dp_gst);
        let square_off_gst = gst_on(square_off, gst);
        let total = brokerage
            + exchange
            + sebi
            + stt
            + stamp
            + gst_line
            + dp
            + dp_gst_line
            + square_off
            + square_off_gst;
        Self([
            brokerage,
            exchange,
            sebi,
            stt,
            stamp,
            gst_line,
            dp,
            dp_gst_line,
            square_off,
            square_off_gst,
            total,
        ])
    }

    /// As [`NoteLines`]. Every line fits `i64` by the bounds of
    /// [`NoteInput::from_fills`]: values at most `i64::MAX` a side, at most
    /// [`MAX_FILLS`] fills, every rate a fraction under one.
    fn narrow(&self) -> NoteLines {
        let [
            brokerage,
            exchange,
            sebi,
            stt,
            stamp,
            gst,
            dp,
            dp_gst,
            square_off,
            square_off_gst,
            total,
        ] = self.0.map(narrow_fitting);
        NoteLines {
            brokerage,
            exchange,
            sebi,
            stt,
            stamp,
            gst,
            dp,
            dp_gst,
            square_off,
            square_off_gst,
            total,
        }
    }
}

/// One note priced under one clock: the official lines, the worst lines and
/// the reading set that gives the worst.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Priced {
    pub(crate) official: NoteLines,
    pub(crate) worst: NoteLines,
    pub(crate) readings: ReadingSet,
}

/// [`official`] and [`worst`] at once, on rates already resolved.
pub(crate) fn priced(input: &NoteInput, rates: &EquityRates) -> Priced {
    let facts = Facts::of(input, rates);
    let (worst, readings) = facts.worst();
    Priced {
        official: facts.pick(&ReadingSet::OFFICIAL).narrow(),
        worst: worst.narrow(),
        readings,
    }
}

/// One contract note under one clock and one reading set, every line in
/// paise.
///
/// # Errors
///
/// [`ChargeError::Cost`] carrying a refusal for a day outside the table's
/// window (`crate::equity_dated::check_window`).
pub fn note(
    input: &NoteInput,
    clock: Clock,
    readings: &ReadingSet,
) -> Result<NoteLines, ChargeError> {
    let rates = clock.rates(input.day())?;
    Ok(Facts::of(input, &rates).pick(readings).narrow())
}

/// The official method's note under one clock.
///
/// # Errors
///
/// As [`note`].
pub fn official(input: &NoteInput, clock: Clock) -> Result<NoteLines, ChargeError> {
    note(input, clock, &ReadingSet::OFFICIAL)
}

/// The worst note under one clock — every line at its largest reading — and
/// the reading set that gives it.
///
/// # Errors
///
/// As [`note`].
pub fn worst(input: &NoteInput, clock: Clock) -> Result<(NoteLines, ReadingSet), ChargeError> {
    let rates = clock.rates(input.day())?;
    let (lines, readings) = Facts::of(input, &rates).worst();
    Ok((lines.narrow(), readings))
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
    use crate::equity_clock::{Fill, Order};
    use crate::equity_dated::STATE_STAMPS;

    fn on() -> TradeDay {
        TradeDay::new(2026, 10, 30).expect("a real date")
    }

    fn fill(second: i64, quantity: i64, price: i64) -> Fill {
        Fill {
            second,
            quantity,
            price: Paisa::from_raw(price),
        }
    }

    fn order(side: OrderSide, by_broker: bool, fills: Vec<Fill>) -> Order {
        Order {
            side,
            by_broker,
            fills,
        }
    }

    fn round_trip(buy: Vec<Fill>, sell: Vec<Fill>) -> TradeFills {
        TradeFills {
            orders: vec![
                order(OrderSide::Buy, false, buy),
                order(OrderSide::Sell, false, sell),
            ],
        }
    }

    fn refused(fills: &TradeFills) -> ChargeError {
        NoteInput::from_fills(fills, on()).expect_err("refused")
    }

    #[test]
    fn a_trade_no_contract_note_could_carry_is_refused_by_name() {
        // No order.
        assert_eq!(
            refused(&TradeFills { orders: vec![] }),
            ChargeError::EmptyDay
        );
        // A square-off on the opening side, first or later.
        let broker_first = TradeFills {
            orders: vec![order(OrderSide::Buy, true, vec![fill(0, 1, 100)])],
        };
        let mismatched = ChargeError::MismatchedLegs {
            why: SQUARE_OFF_ON_OPENING_SIDE,
        };
        assert_eq!(refused(&broker_first), mismatched);
        let broker_adds = TradeFills {
            orders: vec![
                order(OrderSide::Sell, false, vec![fill(0, 1, 100)]),
                order(OrderSide::Sell, true, vec![fill(1, 1, 100)]),
            ],
        };
        assert_eq!(refused(&broker_adds), mismatched);
        // An order with no fill.
        assert_eq!(
            refused(&round_trip(vec![], vec![fill(0, 1, 100)])),
            not_positive("fills in an order", 0)
        );
        // A fill that is not positive.
        assert_eq!(
            refused(&round_trip(vec![fill(0, 0, 100)], vec![fill(1, 1, 100)])),
            not_positive("fill quantity", 0)
        );
        assert_eq!(
            refused(&round_trip(vec![fill(0, -3, 100)], vec![fill(1, 1, 100)])),
            not_positive("fill quantity", -3)
        );
        assert_eq!(
            refused(&round_trip(vec![fill(0, 1, 0)], vec![fill(1, 1, 100)])),
            not_positive("fill price", 0)
        );
        // Two fills of one order in one second, or out of order.
        let seconds = "seconds from the order's previous fill";
        assert_eq!(
            refused(&round_trip(
                vec![fill(5, 1, 100), fill(5, 1, 100)],
                vec![fill(6, 2, 100)]
            )),
            not_positive(seconds, 0)
        );
        assert_eq!(
            refused(&round_trip(
                vec![fill(5, 1, 100), fill(3, 1, 100)],
                vec![fill(6, 2, 100)]
            )),
            not_positive(seconds, -2)
        );
        // A step that leaves i64 names the floor.
        assert_eq!(
            refused(&round_trip(
                vec![fill(i64::MAX, 1, 100), fill(i64::MIN, 1, 100)],
                vec![fill(0, 2, 100)]
            )),
            not_positive(seconds, i64::MIN)
        );
        // Overflows: a fill's value, a side's quantity, a side's value.
        assert_eq!(
            refused(&round_trip(vec![fill(0, i64::MAX, 2)], vec![fill(1, 1, 1)])),
            overflow("a fill's value")
        );
        assert_eq!(
            refused(&round_trip(
                vec![fill(0, i64::MAX, 1), fill(1, 1, 1)],
                vec![fill(2, 1, 1)]
            )),
            overflow("a side's quantity")
        );
        assert_eq!(
            refused(&round_trip(
                vec![fill(0, 1, i64::MAX), fill(1, 1, 1)],
                vec![fill(2, 2, 1)]
            )),
            overflow("a side's value")
        );
        // A position left open, either way.
        assert_eq!(
            refused(&round_trip(vec![fill(0, 3, 100)], vec![fill(1, 2, 100)])),
            ChargeError::OpenIntradayPosition { bought: 3, sold: 2 }
        );
        assert_eq!(
            refused(&TradeFills {
                orders: vec![order(OrderSide::Sell, false, vec![fill(0, 4, 100)])]
            }),
            ChargeError::OpenIntradayPosition { bought: 0, sold: 4 }
        );
    }

    #[test]
    fn a_trade_past_the_fill_bound_is_refused_and_one_at_it_prices() {
        let fills = |n: usize| -> Vec<Fill> {
            (0..n)
                .map(|at| fill(i64::try_from(at).expect("small"), 1, 100_00))
                .collect()
        };
        let at_bound = round_trip(fills(MAX_FILLS - 1), vec![fill(0, 131_071, 100_00)]);
        let input = NoteInput::from_fills(&at_bound, on()).expect("at the bound");
        assert_eq!(input.orders()[0].fills, MAX_FILLS - 1);
        // Brokerage per fill: Rs 0.03 on Rs 100 each, under the cap.
        assert_eq!(input.orders()[0].brokerage_per_fill, 3 * 131_071);
        let past = round_trip(fills(MAX_FILLS), vec![fill(0, 131_072, 100_00)]);
        assert_eq!(
            refused(&past),
            ChargeError::OutOfScope {
                what: "a trade of more than 131,072 fills",
                reason: TOO_MANY_FILLS,
            }
        );
    }

    #[test]
    fn a_trade_sums_its_orders_and_keeps_them() {
        let trade = TradeFills {
            orders: vec![
                order(
                    OrderSide::Sell,
                    false,
                    vec![fill(1, 40, 500_00), fill(2, 60, 499_50)],
                ),
                order(OrderSide::Buy, false, vec![fill(9, 70, 490_00)]),
                order(OrderSide::Buy, true, vec![fill(20, 30, 491_00)]),
            ],
        };
        let input = NoteInput::from_fills(&trade, on()).expect("a short trade");
        assert_eq!(input.day(), on());
        assert_eq!(input.quantity(), 100);
        assert_eq!(input.sell_value().raw(), 40 * 500_00 + 60 * 499_50);
        assert_eq!(input.buy_value().raw(), 70 * 490_00 + 30 * 491_00);
        assert_eq!(input.gross().raw(), 49_970_00 - 49_030_00);
        // VWAP = 99,000 / 200 = 495.00.
        assert_eq!(input.vwap().raw(), 495_00);
        assert_eq!(
            input.orders(),
            [
                OrderTotals {
                    side: OrderSide::Sell,
                    by_broker: false,
                    quantity: 100,
                    value: 49_970_00,
                    fills: 2,
                    brokerage_per_fill: 6_00 + 8_99,
                },
                OrderTotals {
                    side: OrderSide::Buy,
                    by_broker: false,
                    quantity: 70,
                    value: 34_300_00,
                    fills: 1,
                    brokerage_per_fill: 10_29,
                },
                OrderTotals {
                    side: OrderSide::Buy,
                    by_broker: true,
                    quantity: 30,
                    value: 14_730_00,
                    fills: 1,
                    brokerage_per_fill: 4_42,
                },
            ]
        );
    }

    #[test]
    fn brokerage_is_the_schedules_dhan_brokerage_at_every_size() {
        for value in [
            1,
            33,
            50,
            166,
            1_00,
            66_666_66,
            66_666_67,
            66_666_68,
            1_000_000_00,
        ] {
            assert_eq!(
                brokerage(i128::from(value)),
                i128::from(
                    crate::schedule::dhan_brokerage(
                        crate::schedule::TradeKind::Equity(crate::schedule::Product::Intraday),
                        Paisa::from_raw(value)
                    )
                    .expect("in range")
                    .raw()
                ),
                "{value}"
            );
        }
        assert_eq!(rupee(49), 0);
        assert_eq!(rupee(50), 100);
        assert_eq!(rupee(12_449), 12_400);
        assert_eq!(rupee(12_450), 12_500);
    }

    #[test]
    fn every_reading_has_its_label() {
        assert_eq!(
            ReadingSet::OFFICIAL.labels(),
            [
                "RD-BRK per-order",
                "RD-STT-BASE vwap",
                "RD-ROUND-TAX client-day",
                "RD-ROUND-EX exchange-once",
                "RD-ROUND-EX sebi-once",
                "RD-GST once",
                "RD-DP none",
                "RD-GST dp-once",
                "RD-SQ on-top",
                "RD-SQ one-fee-per-order",
                "RD-STATE worst",
                "RD-STATE paisa",
                "RD-SLAB top"
            ]
        );
        let other = ReadingSet {
            brokerage: BrokerageReading::PerFill,
            stt: SttReading::SellValue,
            stamp: TaxRounding::SecurityDay,
            exchange: ExchangeRounding::Split,
            sebi: SebiRounding::PerOrder,
            gst: GstRounding::CgstSgst,
            dp: DpReading::Intraday,
            dp_gst: GstRounding::CgstSgst,
            square_off: SquareOffReading::Instead,
            square_off_split: SquareOffSplit::OnePerFill,
            state: StateChoice::Named(&STATE_STAMPS[13]),
            state_rounding: StateRounding::Rupee,
            slab: SlabReading::Top,
        };
        assert_eq!(
            other.labels(),
            [
                "RD-BRK per-fill",
                "RD-STT-BASE sell-value",
                "RD-ROUND-TAX security-day",
                "RD-ROUND-EX exchange-split",
                "RD-ROUND-EX sebi-per-order",
                "RD-GST cgst-sgst",
                "RD-DP intraday",
                "RD-GST dp-cgst-sgst",
                "RD-SQ instead",
                "RD-SQ one-fee-per-fill",
                "TS",
                "RD-STATE rupee",
                "RD-SLAB top"
            ]
        );
        let third = ReadingSet {
            stt: SttReading::SellValuePerOrder,
            stamp: TaxRounding::PerOrder,
            exchange: ExchangeRounding::PerOrder,
            ..other
        };
        assert_eq!(
            [third.labels()[1], third.labels()[2], third.labels()[3]],
            [
                "RD-STT-BASE sell-value-per-order",
                "RD-ROUND-TAX per-order",
                "RD-ROUND-EX exchange-per-order"
            ]
        );
        let fourth = ReadingSet {
            exchange: ExchangeRounding::PerOrderSplit,
            ..other
        };
        assert_eq!(fourth.labels()[3], "RD-ROUND-EX exchange-per-order-split");
        assert_eq!(NOTE_LINE_NAMES.len(), 11);
    }

    #[test]
    fn the_largest_keeps_the_first_on_a_tie() {
        assert_eq!(largest(("a", 5), &[("b", 5), ("c", 4)]), ("a", 5));
        assert_eq!(largest(("a", 5), &[("b", 6), ("c", 6)]), ("b", 6));
        assert_eq!(gst_on(242_15, GstRounding::Once), 43_59);
        assert_eq!(gst_on(242_15, GstRounding::CgstSgst), 43_58);
        assert_eq!(gst_on(12_50, GstRounding::Once), 2_25);
        assert_eq!(gst_on(12_50, GstRounding::CgstSgst), 2_26);
    }

    #[test]
    fn the_lines_at_the_input_bounds_fit_and_add_up() {
        // The largest a side may be: i64::MAX of value each way, one fill a
        // side, a pre-2020 day so the State stamp is priced too.
        let big = round_trip(vec![fill(0, 1, i64::MAX)], vec![fill(1, 1, i64::MAX)]);
        for day in [TradeDay::new(2019, 6, 3).expect("a day"), on()] {
            let input = NoteInput::from_fills(&big, day).expect("at the bound");
            let (lines, _) = worst(&input, Clock::Dated).expect("in the window");
            let sum: i128 = lines.lines()[..10]
                .iter()
                .map(|line| i128::from(line.raw()))
                .sum();
            assert_eq!(sum, i128::from(lines.total.raw()));
            assert!(lines.total.raw() < i64::MAX / 1_000);
        }
    }
}
