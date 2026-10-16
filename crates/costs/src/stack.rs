//! The precise charge stack: one executed order, and one client-security-day.
//!
//! This is the **default** mode of package CHG-1 — the contract-note method.
//! It prices what a broker's contract note would show for one client, one
//! security, one day:
//!
//! | Line | On | Rounded |
//! |---|---|---|
//! | brokerage (Dhan) | each executed order | half-up to the paisa, per order |
//! | exchange transaction charge, IPFT included | the day's turnover | half-up to the paisa, once |
//! | SEBI turnover fee | the day's turnover | half-up to the paisa, once |
//! | STT | the day's value the tax is levied on | half-up to the rupee, once |
//! | stamp duty | the day's buy value it is levied on | half-up to the rupee, once |
//! | GST, 18% | brokerage + exchange charge + SEBI fee | half-up to the paisa, once |
//! | DP charge (Dhan), and its GST | a delivery or MTF sell day | exact |
//! | broker square-off fee, and its GST | each order the caller says the broker placed | exact |
//!
//! The direction of an exact half is a choice — no source this repository
//! holds states it — and is recorded as UNVERIFIED (D-5113).
//!
//! # The intraday transaction tax is on the day's VWAP, not on the sell value
//!
//! NSE levies the non-delivery STT on the **non-delivery quantity at the
//! volume-weighted average price of every trade the client made in the
//! security that day** — buys and sells together. For a squared-off day that
//! is half the day's turnover. The operator's five contract-note vectors were
//! re-derived from that rule and every one agrees with it; none agrees with
//! the sell-value basis Dhan's calculator uses
//! (`costs::stack::the_five_contract_note_vectors_rederive_from_the_rules`).
//!
//! # What decides delivery
//!
//! The day's quantities, as the exchange nets them — not the product label.
//! Equal bought and sold quantities are a non-delivery day whatever the
//! product. A one-sided delivery or MTF day is delivery. A one-sided
//! **intraday** day is refused: the exit order is missing. A delivery day
//! with both sides of unequal size is refused: its split needs the FIFO order
//! sequence (`NCL/CMPT/43115`), and day totals do not carry it.
//!
//! # Constant cost
//!
//! [`ClientSecurityDay::add`] is a fixed sequence of checked additions and
//! one [`dhan_brokerage`]; [`ClientSecurityDay::settle`] is a fixed sequence
//! of at most nine [`crate::exact::Ratio::apply`] calls and one fixed-length
//! sum. Neither walks the orders: the day keeps running totals, so the
//! thousandth order costs what the first did. `crates/costs/benches/stack.rs`
//! measures both.

use std::fmt;

use brutex_core::instrument::Exchange;
use brutex_core::price::Paisa;

use crate::charge_error::ChargeError;
use crate::day::TradeDay;
use crate::error::CostError;
use crate::exact::{Quantum, Rupees};
use crate::rate::OrderSide;
use crate::schedule::{
    Cited, DHAN_DP_CHARGE, DHAN_SQUARE_OFF_FEE, Product, RateBasis, RateCard, RateSelector, Status,
    Taxes, TradeKind, dhan_brokerage,
};

/// What every precise output says it is.
pub const PRECISE_LABEL: &str = "precise (contract-note method)";

// ---------------------------------------------------------------------------
// One executed order
// ---------------------------------------------------------------------------

/// One executed order: a side, a quantity in units, and a price per unit.
///
/// For an option the price is the premium.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Order {
    side: OrderSide,
    quantity: i64,
    price: Paisa,
    by_broker: bool,
}

impl Order {
    /// An order the client placed.
    ///
    /// # Errors
    ///
    /// [`ChargeError::Cost`] carrying [`CostError::NotPositive`] for a
    /// quantity or a price that is not strictly positive. Dhan's calculator
    /// shows such an order as zeros; here it is refused by name.
    ///
    /// # Examples
    ///
    /// ```
    /// use brutex_core::price::Paisa;
    /// use costs::rate::OrderSide;
    /// use costs::stack::Order;
    ///
    /// let order = Order::new(OrderSide::Buy, 50, Paisa::from_raw(1_520_00))?;
    /// assert_eq!(order.value()?.raw(), 76_000_00);
    /// assert!(Order::new(OrderSide::Buy, 0, Paisa::from_raw(1_520_00)).is_err());
    /// # Ok::<(), costs::charge_error::ChargeError>(())
    /// ```
    pub fn new(side: OrderSide, quantity: i64, price: Paisa) -> Result<Self, ChargeError> {
        if quantity <= 0 {
            return Err(CostError::NotPositive {
                quantity: "order quantity",
                value: quantity,
            }
            .into());
        }
        if price.raw() <= 0 {
            return Err(CostError::NotPositive {
                quantity: "order price",
                value: price.raw(),
            }
            .into());
        }
        Ok(Self {
            side,
            quantity,
            price,
            by_broker: false,
        })
    }

    /// The same order, placed by the broker's risk system to square off a
    /// position still open at the intraday ceiling. It carries the square-off
    /// fee as a separate line, and only because the caller said so.
    #[must_use]
    pub const fn squared_off_by_broker(self) -> Self {
        Self {
            by_broker: true,
            ..self
        }
    }

    /// Buy or sell.
    #[must_use]
    pub const fn side(self) -> OrderSide {
        self.side
    }

    /// Units.
    #[must_use]
    pub const fn quantity(self) -> i64 {
        self.quantity
    }

    /// Price, or premium, per unit.
    #[must_use]
    pub const fn price(self) -> Paisa {
        self.price
    }

    /// Whether the broker placed it at the ceiling.
    #[must_use]
    pub const fn is_broker_square_off(self) -> bool {
        self.by_broker
    }

    /// `quantity × price`.
    ///
    /// # Errors
    ///
    /// [`CostError::Overflow`] when the value leaves `i64`.
    pub fn value(self) -> Result<Paisa, ChargeError> {
        self.quantity
            .checked_mul(self.price.raw())
            .map(Paisa::from_raw)
            .ok_or(overflow("order quantity x price"))
    }
}

/// What one executed order costs on its own: the fees charged per order.
///
/// The levies — exchange charge, SEBI fee, STT, stamp duty, GST — are charged
/// on the day's totals and rounded once, so they belong to [`DayCharges`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrderCharges {
    value: Paisa,
    brokerage: Paisa,
    square_off_fee: Paisa,
    square_off_gst: Paisa,
}

impl OrderCharges {
    /// `quantity × price`.
    #[must_use]
    pub const fn value(self) -> Paisa {
        self.value
    }

    /// Dhan's brokerage on this order, rounded half-up to the paisa.
    #[must_use]
    pub const fn brokerage(self) -> Paisa {
        self.brokerage
    }

    /// The broker square-off fee, ₹20, or zero for an order the client placed.
    #[must_use]
    pub const fn square_off_fee(self) -> Paisa {
        self.square_off_fee
    }

    /// GST on the square-off fee: 18% of ₹20, or zero.
    #[must_use]
    pub const fn square_off_gst(self) -> Paisa {
        self.square_off_gst
    }
}

// ---------------------------------------------------------------------------
// One client-security-day
// ---------------------------------------------------------------------------

/// One client's trades in one security on one day, as running totals.
///
/// The rate card is resolved once, when the day opens. Each [`Self::add`]
/// updates the totals in constant time; [`Self::settle`] prices them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientSecurityDay {
    card: RateCard,
    orders: i64,
    buy_quantity: i64,
    buy_value: Paisa,
    sell_quantity: i64,
    sell_value: Paisa,
    brokerage: Paisa,
    square_off_fees: Paisa,
    exercised: Paisa,
}

impl ClientSecurityDay {
    /// Opens a day for `kind` at `exchange` on `day`, priced by `selector`.
    ///
    /// # Errors
    ///
    /// Whatever [`RateCard::resolve`] refuses.
    ///
    /// # Examples
    ///
    /// ```
    /// use brutex_core::instrument::Exchange;
    /// use brutex_core::price::Paisa;
    /// use costs::day::TradeDay;
    /// use costs::rate::OrderSide;
    /// use costs::schedule::{Product, RateSelector, TradeKind};
    /// use costs::stack::{ClientSecurityDay, Order};
    ///
    /// let mut day = ClientSecurityDay::open(
    ///     TradeKind::Equity(Product::Intraday),
    ///     Exchange::Nse,
    ///     TradeDay::new(2026, 10, 15)?,
    ///     RateSelector::default(),
    /// )?;
    /// day.add(Order::new(OrderSide::Buy, 50, Paisa::from_raw(1_520_00))?)?;
    /// day.add(Order::new(OrderSide::Sell, 50, Paisa::from_raw(1_530_00))?)?;
    /// let charges = day.settle()?;
    /// assert_eq!(charges.total().raw(), 73_90);
    /// assert_eq!(charges.basis().to_string(), "today's rates applied to 2026-10-15");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn open(
        kind: TradeKind,
        exchange: Exchange,
        day: TradeDay,
        selector: RateSelector,
    ) -> Result<Self, ChargeError> {
        RateCard::resolve(kind, exchange, day, selector).map(Self::with_card)
    }

    /// Opens a day on a card already resolved — one resolution for many
    /// securities on the same day and kind.
    #[must_use]
    pub const fn with_card(card: RateCard) -> Self {
        Self {
            card,
            orders: 0,
            buy_quantity: 0,
            buy_value: Paisa::ZERO,
            sell_quantity: 0,
            sell_value: Paisa::ZERO,
            brokerage: Paisa::ZERO,
            square_off_fees: Paisa::ZERO,
            exercised: Paisa::ZERO,
        }
    }

    /// The rate card the day is priced on.
    #[must_use]
    pub const fn card(&self) -> &RateCard {
        &self.card
    }

    /// Records one executed order, and returns what it costs on its own.
    ///
    /// All or nothing: a refused order leaves the day exactly as it was.
    ///
    /// # Errors
    ///
    /// * [`ChargeError::SquareOffNotApplicable`] for a broker square-off on a
    ///   delivery or MTF day.
    /// * [`ChargeError::Cost`] carrying an overflow of the order value or of a
    ///   day total.
    pub fn add(&mut self, order: Order) -> Result<OrderCharges, ChargeError> {
        let kind = self.card.kind();
        if order.by_broker && !kind.can_be_squared_off_by_broker() {
            return Err(ChargeError::SquareOffNotApplicable {
                kind: kind.as_str(),
            });
        }
        let value = order.value()?;
        let brokerage = dhan_brokerage(kind, value)?;
        let (square_off_fee, square_off_gst) = if order.by_broker {
            let gst = self
                .card
                .gst()
                .rate()
                .apply(DHAN_SQUARE_OFF_FEE, 1, Quantum::Paisa)?;
            (DHAN_SQUARE_OFF_FEE, gst)
        } else {
            (Paisa::ZERO, Paisa::ZERO)
        };

        let mut next = *self;
        next.orders = self
            .orders
            .checked_add(1)
            .ok_or(overflow("the day's order count"))?;
        match order.side {
            OrderSide::Buy => {
                next.buy_quantity = add_units(
                    self.buy_quantity,
                    order.quantity,
                    "the day's bought quantity",
                )?;
                next.buy_value = add(self.buy_value, value, "the day's buy value")?;
            }
            OrderSide::Sell => {
                next.sell_quantity = add_units(
                    self.sell_quantity,
                    order.quantity,
                    "the day's sold quantity",
                )?;
                next.sell_value = add(self.sell_value, value, "the day's sell value")?;
            }
        }
        next.brokerage = add(self.brokerage, brokerage, "the day's brokerage")?;
        next.square_off_fees = add(
            self.square_off_fees,
            square_off_fee,
            "the day's square-off fees",
        )?;
        *self = next;
        Ok(OrderCharges {
            value,
            brokerage,
            square_off_fee,
            square_off_gst,
        })
    }

    /// Records an exercise of an option worth `intrinsic_value`, on which
    /// STT is levied at the exercise rate.
    ///
    /// # Errors
    ///
    /// * [`ChargeError::ExerciseNotApplicable`] on a day that is not options.
    /// * [`ChargeError::Cost`] carrying [`CostError::NotPositive`] for an
    ///   intrinsic value that is not positive — an out-of-the-money option is
    ///   not exercised — or an overflow of the day's exercised value.
    pub fn exercise(&mut self, intrinsic_value: Paisa) -> Result<(), ChargeError> {
        let kind = self.card.kind();
        if kind != TradeKind::Options {
            return Err(ChargeError::ExerciseNotApplicable {
                kind: kind.as_str(),
            });
        }
        if intrinsic_value.raw() <= 0 {
            return Err(CostError::NotPositive {
                quantity: "exercise intrinsic value",
                value: intrinsic_value.raw(),
            }
            .into());
        }
        self.exercised = add(self.exercised, intrinsic_value, "the day's exercised value")?;
        Ok(())
    }

    /// Prices the day.
    ///
    /// # Errors
    ///
    /// * [`ChargeError::EmptyDay`] when nothing was traded or exercised.
    /// * [`ChargeError::OpenIntradayPosition`] for an intraday equity day
    ///   whose bought and sold quantities differ.
    /// * [`ChargeError::PartlySquaredDay`] for a delivery or MTF day with
    ///   both sides of unequal size.
    /// * [`ChargeError::Cost`] carrying an overflow.
    pub fn settle(&self) -> Result<DayCharges, ChargeError> {
        if self.orders == 0 && self.exercised.raw() == 0 {
            return Err(ChargeError::EmptyDay);
        }
        let card = &self.card;
        let turnover = add(self.buy_value, self.sell_value, "the day's turnover")?;
        let exchange_charge = card
            .exchange_charge()
            .rate()
            .apply(turnover, 1, Quantum::Paisa)?;
        let sebi = card.sebi().rate().apply(turnover, 1, Quantum::Paisa)?;

        let levied = self.taxes(turnover)?;
        let dp = if levied.delivery_sell {
            DHAN_DP_CHARGE
        } else {
            Paisa::ZERO
        };
        let gst_rate = card.gst().rate();
        let base = sum(&[self.brokerage, exchange_charge, sebi], "the GST base")?;
        let gst = gst_rate.apply(base, 1, Quantum::Paisa)?;
        let dp_gst = gst_rate.apply(dp, 1, Quantum::Paisa)?;
        let square_off_gst = gst_rate.apply(self.square_off_fees, 1, Quantum::Paisa)?;
        let total = sum(
            &[
                self.brokerage,
                exchange_charge,
                sebi,
                levied.stt,
                levied.stt_exercise,
                levied.stamp,
                gst,
                dp,
                dp_gst,
                self.square_off_fees,
                square_off_gst,
            ],
            "the day's total",
        )?;
        Ok(DayCharges {
            basis: card.basis(),
            kind: card.kind(),
            exchange: card.exchange(),
            orders: self.orders,
            buy_quantity: self.buy_quantity,
            buy_value: self.buy_value,
            sell_quantity: self.sell_quantity,
            sell_value: self.sell_value,
            exercised: self.exercised,
            turnover,
            brokerage: self.brokerage,
            exchange_charge,
            sebi,
            stt: levied.stt,
            stt_exercise: levied.stt_exercise,
            stamp: levied.stamp,
            gst,
            dp,
            dp_gst,
            square_off_fee: self.square_off_fees,
            square_off_gst,
            total,
            rates_used: [
                Some(card.exchange_charge()),
                Some(card.sebi()),
                Some(card.gst()),
                Some(levied.stt_rate),
                Some(levied.stamp_rate),
                levied.exercise_rate,
            ],
        })
    }

    /// The transaction tax and the stamp duty, by kind.
    fn taxes(&self, turnover: Paisa) -> Result<Levied, ChargeError> {
        match self.card.taxes() {
            Taxes::Equity {
                product,
                stt_non_delivery,
                stt_delivery,
                stamp_non_delivery,
                stamp_delivery,
            } => {
                let (bought, sold) = (self.buy_quantity, self.sell_quantity);
                if bought == sold {
                    // Squared off: all non-delivery. The non-delivery value is
                    // the quantity at the day's VWAP of every trade, which for
                    // equal sides is half the turnover — the halving is the
                    // rate's divisor, so the tax is rounded once.
                    return Ok(Levied::of(
                        stt_non_delivery.rate().apply(turnover, 2, Quantum::Rupee)?,
                        stt_non_delivery,
                        stamp_non_delivery
                            .rate()
                            .apply(self.buy_value, 1, Quantum::Rupee)?,
                        stamp_non_delivery,
                        false,
                    ));
                }
                if product == Product::Intraday {
                    return Err(ChargeError::OpenIntradayPosition { bought, sold });
                }
                if bought > 0 && sold > 0 {
                    return Err(ChargeError::PartlySquaredDay { bought, sold });
                }
                // One side only: delivery. The tax is on that side's value,
                // the stamp on the buy value, and a sell debits the demat.
                Ok(Levied::of(
                    stt_delivery.rate().apply(turnover, 1, Quantum::Rupee)?,
                    stt_delivery,
                    stamp_delivery
                        .rate()
                        .apply(self.buy_value, 1, Quantum::Rupee)?,
                    stamp_delivery,
                    sold > 0,
                ))
            }
            Taxes::Futures {
                stt_sell,
                stamp_buy,
            } => Ok(Levied::of(
                stt_sell.rate().apply(self.sell_value, 1, Quantum::Rupee)?,
                stt_sell,
                stamp_buy.rate().apply(self.buy_value, 1, Quantum::Rupee)?,
                stamp_buy,
                false,
            )),
            Taxes::Options {
                stt_sell,
                stt_exercise,
                stamp_buy,
            } => {
                let mut levied = Levied::of(
                    stt_sell.rate().apply(self.sell_value, 1, Quantum::Rupee)?,
                    stt_sell,
                    stamp_buy.rate().apply(self.buy_value, 1, Quantum::Rupee)?,
                    stamp_buy,
                    false,
                );
                levied.stt_exercise =
                    stt_exercise
                        .rate()
                        .apply(self.exercised, 1, Quantum::Rupee)?;
                levied.exercise_rate = Some(stt_exercise);
                Ok(levied)
            }
        }
    }
}

/// The two taxes of one day, and the rates that levied them.
struct Levied {
    stt: Paisa,
    stt_rate: Cited,
    stt_exercise: Paisa,
    exercise_rate: Option<Cited>,
    stamp: Paisa,
    stamp_rate: Cited,
    delivery_sell: bool,
}

impl Levied {
    /// The common shape, with no exercise: every arm of
    /// [`ClientSecurityDay::taxes`] builds through it.
    const fn of(
        stt: Paisa,
        stt_rate: Cited,
        stamp: Paisa,
        stamp_rate: Cited,
        delivery_sell: bool,
    ) -> Self {
        Self {
            stt,
            stt_rate,
            stt_exercise: Paisa::ZERO,
            exercise_rate: None,
            stamp,
            stamp_rate,
            delivery_sell,
        }
    }
}

/// An overflow refusal naming the operation.
const fn overflow(operation: &'static str) -> ChargeError {
    ChargeError::Cost(CostError::Overflow { operation })
}

/// `a + b` in paisa, refused rather than wrapped.
fn add(a: Paisa, b: Paisa, operation: &'static str) -> Result<Paisa, ChargeError> {
    a.raw()
        .checked_add(b.raw())
        .map(Paisa::from_raw)
        .ok_or(overflow(operation))
}

/// `a + b` in units, refused rather than wrapped.
fn add_units(a: i64, b: i64, operation: &'static str) -> Result<i64, ChargeError> {
    a.checked_add(b).ok_or(overflow(operation))
}

/// The sum of a fixed handful of amounts, refused rather than wrapped. The
/// slice is always a literal of at most eleven, so the walk is bounded at the
/// call site, never by the data.
fn sum(parts: &[Paisa], operation: &'static str) -> Result<Paisa, ChargeError> {
    let mut total = Paisa::ZERO;
    for &part in parts {
        total = add(total, part, operation)?;
    }
    Ok(total)
}

// ---------------------------------------------------------------------------
// The settled day
// ---------------------------------------------------------------------------

/// How many rates a settled day can have used: the shared three, the STT,
/// the stamp duty and an exercise STT.
pub const MAX_RATES_USED: usize = 6;

/// One client-security-day, priced: every line, the total, and the label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DayCharges {
    basis: RateBasis,
    kind: TradeKind,
    exchange: Exchange,
    orders: i64,
    buy_quantity: i64,
    buy_value: Paisa,
    sell_quantity: i64,
    sell_value: Paisa,
    exercised: Paisa,
    turnover: Paisa,
    brokerage: Paisa,
    exchange_charge: Paisa,
    sebi: Paisa,
    stt: Paisa,
    stt_exercise: Paisa,
    stamp: Paisa,
    gst: Paisa,
    dp: Paisa,
    dp_gst: Paisa,
    square_off_fee: Paisa,
    square_off_gst: Paisa,
    total: Paisa,
    rates_used: [Option<Cited>; MAX_RATES_USED],
}

impl DayCharges {
    /// The label: which rate set, applied to which day.
    #[must_use]
    pub const fn basis(&self) -> RateBasis {
        self.basis
    }
    /// What was traded.
    #[must_use]
    pub const fn kind(&self) -> TradeKind {
        self.kind
    }
    /// Where.
    #[must_use]
    pub const fn exchange(&self) -> Exchange {
        self.exchange
    }
    /// How many executed orders.
    #[must_use]
    pub const fn orders(&self) -> i64 {
        self.orders
    }
    /// Units bought.
    #[must_use]
    pub const fn buy_quantity(&self) -> i64 {
        self.buy_quantity
    }
    /// Value bought.
    #[must_use]
    pub const fn buy_value(&self) -> Paisa {
        self.buy_value
    }
    /// Units sold.
    #[must_use]
    pub const fn sell_quantity(&self) -> i64 {
        self.sell_quantity
    }
    /// Value sold.
    #[must_use]
    pub const fn sell_value(&self) -> Paisa {
        self.sell_value
    }
    /// Intrinsic value exercised.
    #[must_use]
    pub const fn exercised(&self) -> Paisa {
        self.exercised
    }
    /// Buy value + sell value.
    #[must_use]
    pub const fn turnover(&self) -> Paisa {
        self.turnover
    }
    /// Dhan's brokerage, summed over the orders.
    #[must_use]
    pub const fn brokerage(&self) -> Paisa {
        self.brokerage
    }
    /// The exchange transaction charge, IPFT included.
    #[must_use]
    pub const fn exchange_charge(&self) -> Paisa {
        self.exchange_charge
    }
    /// The SEBI turnover fee.
    #[must_use]
    pub const fn sebi(&self) -> Paisa {
        self.sebi
    }
    /// The securities transaction tax on trades.
    #[must_use]
    pub const fn stt(&self) -> Paisa {
        self.stt
    }
    /// The securities transaction tax on exercise.
    #[must_use]
    pub const fn stt_exercise(&self) -> Paisa {
        self.stt_exercise
    }
    /// Stamp duty.
    #[must_use]
    pub const fn stamp(&self) -> Paisa {
        self.stamp
    }
    /// GST on brokerage, the exchange charge and the SEBI fee.
    #[must_use]
    pub const fn gst(&self) -> Paisa {
        self.gst
    }
    /// Dhan's DP charge, on a delivery or MTF sell day.
    #[must_use]
    pub const fn dp(&self) -> Paisa {
        self.dp
    }
    /// GST on the DP charge.
    #[must_use]
    pub const fn dp_gst(&self) -> Paisa {
        self.dp_gst
    }
    /// The broker square-off fee — a separate line.
    #[must_use]
    pub const fn square_off_fee(&self) -> Paisa {
        self.square_off_fee
    }
    /// GST on the broker square-off fee.
    #[must_use]
    pub const fn square_off_gst(&self) -> Paisa {
        self.square_off_gst
    }
    /// Every line, summed.
    #[must_use]
    pub const fn total(&self) -> Paisa {
        self.total
    }
    /// The rates this day was actually priced at.
    #[must_use]
    pub const fn rates_used(&self) -> [Option<Cited>; MAX_RATES_USED] {
        self.rates_used
    }

    /// The sources of every UNVERIFIED rate this day used, in card order.
    pub fn unverified_sources(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.rates_used
            .iter()
            .flatten()
            .filter(|rate| rate.status() == Status::Unverified)
            .map(|rate| rate.source())
    }

    /// The lines, named, in contract-note order.
    #[must_use]
    pub const fn lines(&self) -> [(&'static str, Paisa); 11] {
        [
            ("brokerage", self.brokerage),
            (
                "exchange transaction charge (incl. IPFT)",
                self.exchange_charge,
            ),
            ("SEBI turnover fee", self.sebi),
            ("STT", self.stt),
            ("STT on exercise", self.stt_exercise),
            ("stamp duty", self.stamp),
            ("GST", self.gst),
            ("DP charge", self.dp),
            ("GST on DP charge", self.dp_gst),
            ("broker square-off fee", self.square_off_fee),
            ("GST on broker square-off fee", self.square_off_gst),
        ]
    }
}

impl fmt::Display for DayCharges {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{PRECISE_LABEL} — {}", self.basis)?;
        writeln!(
            f,
            "{} on {}: bought {} for {}, sold {} for {}",
            self.kind,
            self.exchange.as_str(),
            self.buy_quantity,
            Rupees(self.buy_value),
            self.sell_quantity,
            Rupees(self.sell_value)
        )?;
        for (name, amount) in self.lines() {
            writeln!(f, "  {name}: {}", Rupees(amount))?;
        }
        write!(f, "  total: {}", Rupees(self.total))?;
        for source in self.unverified_sources() {
            write!(f, "\n  UNVERIFIED rate used: {source}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::inconsistent_digit_grouping,
    clippy::too_many_lines
)]
mod tests {
    use super::*;
    use crate::schedule::{BSE_EQUITY_EXCHANGE, CURRENT_AS_OF, STT_EQUITY_DELIVERY, STT_FUTURES};

    fn day(year: u16, month: u8, d: u8) -> TradeDay {
        TradeDay::new(year, month, d).expect("a real date")
    }

    fn rs(paisa: i64) -> Paisa {
        Paisa::from_raw(paisa)
    }

    fn buy(quantity: i64, price: i64) -> Order {
        Order::new(OrderSide::Buy, quantity, rs(price)).expect("a valid order")
    }

    fn sell(quantity: i64, price: i64) -> Order {
        Order::new(OrderSide::Sell, quantity, rs(price)).expect("a valid order")
    }

    fn open(kind: TradeKind, exchange: Exchange) -> ClientSecurityDay {
        ClientSecurityDay::open(kind, exchange, day(2026, 10, 15), RateSelector::default())
            .expect("today's rates price every day")
    }

    fn priced(kind: TradeKind, exchange: Exchange, orders: &[Order]) -> DayCharges {
        let mut csd = open(kind, exchange);
        for &order in orders {
            csd.add(order).expect("in range");
        }
        csd.settle().expect("a settled day")
    }

    const INTRADAY: TradeKind = TradeKind::Equity(Product::Intraday);
    const DELIVERY: TradeKind = TradeKind::Equity(Product::Delivery);
    const MTF: TradeKind = TradeKind::Equity(Product::Mtf);

    /// The lines of a day, as the test vectors list them.
    fn lines(c: &DayCharges) -> [i64; 7] {
        [
            c.brokerage().raw(),
            c.exchange_charge().raw(),
            c.sebi().raw(),
            c.stt().raw(),
            c.stamp().raw(),
            c.gst().raw(),
            c.total().raw(),
        ]
    }

    #[test]
    fn the_five_contract_note_vectors_rederive_from_the_rules() {
        // The operator's attack-run vectors, re-derived line by line from the
        // rules as written (D-5111). Columns: brokerage, exchange incl. IPFT,
        // SEBI, STT, stamp, GST, total — paisa.
        //
        // V1: 50 @1520 -> 1530. STT 0.025% x 50 x VWAP 1525 = 19.06 -> 19.
        let v1 = priced(
            INTRADAY,
            Exchange::Nse,
            &[buy(50, 1_520_00), sell(50, 1_530_00)],
        );
        assert_eq!(lines(&v1), [40_00, 4_68, 15, 19_00, 2_00, 8_07, 73_90]);
        // V2: 1000 @1000 -> 1010. STT on VWAP 1005: 251.25 -> 251, not the
        // 252.50 the sell value would give.
        let v2 = priced(
            INTRADAY,
            Exchange::Nse,
            &[buy(1_000, 1_000_00), sell(1_000, 1_010_00)],
        );
        assert_eq!(
            lines(&v2),
            [40_00, 61_71, 2_01, 251_00, 30_00, 18_67, 403_39]
        );
        // V3: 10 @100 -> 99. Brokerage 0.30 + 0.297 -> 0.30 each; both taxes
        // round to nothing.
        let v3 = priced(INTRADAY, Exchange::Nse, &[buy(10, 100_00), sell(10, 99_00)]);
        assert_eq!(lines(&v3), [60, 6, 0, 0, 0, 12, 78]);
        // V4: 100000 @50 -> 50.50. The combined exchange line rounds 308.535
        // to 308.54; the FA73061 parts rounded apart would give 308.53.
        let v4 = priced(
            INTRADAY,
            Exchange::Nse,
            &[buy(100_000, 50_00), sell(100_000, 50_50)],
        );
        assert_eq!(
            lines(&v4),
            [40_00, 308_54, 10_05, 1_256_00, 150_00, 64_55, 1_829_14]
        );
        // V5: short 500, sell 2000 then buy 1980. VWAP 1990: 248.75 -> 249.
        let v5 = priced(
            INTRADAY,
            Exchange::Nse,
            &[sell(500, 2_000_00), buy(500, 1_980_00)],
        );
        assert_eq!(
            lines(&v5),
            [40_00, 61_09, 1_99, 249_00, 30_00, 18_55, 400_63]
        );
        for charges in [v1, v2, v3, v4, v5] {
            assert_eq!(charges.dp(), Paisa::ZERO);
            assert_eq!(charges.square_off_fee(), Paisa::ZERO);
            assert_eq!(charges.orders(), 2);
            assert_eq!(charges.unverified_sources().count(), 0, "{charges}");
        }
    }

    #[test]
    fn the_sell_value_basis_disagrees_with_three_of_the_five_vectors() {
        // The basis Dhan's calculator uses — 0.025% of the SELL value — moves
        // V2, V4 and V5 by a rupee or more. The rule as written (VWAP) is the
        // one that reproduces the operator's contract-note totals.
        let sell_basis = |sell_value: i64| {
            crate::schedule::STT_EQUITY_NON_DELIVERY
                .rate()
                .apply(rs(sell_value), 1, Quantum::Rupee)
                .expect("in range")
                .raw()
        };
        assert_eq!(sell_basis(1_010_000_00), 253_00); // V2: VWAP gives 251
        assert_eq!(sell_basis(5_050_000_00), 1_263_00); // V4: VWAP gives 1256
        assert_eq!(sell_basis(1_000_000_00), 250_00); // V5: VWAP gives 249
        assert_eq!(sell_basis(76_500_00), 19_00); // V1: both give 19
    }

    #[test]
    fn the_order_of_the_orders_and_their_split_move_only_the_brokerage() {
        let whole = priced(
            INTRADAY,
            Exchange::Nse,
            &[buy(50, 1_520_00), sell(50, 1_530_00)],
        );
        let reversed = priced(
            INTRADAY,
            Exchange::Nse,
            &[sell(50, 1_530_00), buy(50, 1_520_00)],
        );
        assert_eq!(whole, reversed);
        // Two buys of 25 and two sells of 25: four orders, so four brokerages
        // of min(₹20, 0.03%) — 11.40 and 11.48 each — and every levy the same.
        let split = priced(
            INTRADAY,
            Exchange::Nse,
            &[
                buy(25, 1_520_00),
                buy(25, 1_520_00),
                sell(25, 1_530_00),
                sell(25, 1_530_00),
            ],
        );
        assert_eq!(split.orders(), 4);
        assert_eq!(split.brokerage(), rs(2 * 11_40 + 2 * 11_48));
        assert_eq!(split.exchange_charge(), whole.exchange_charge());
        assert_eq!(split.sebi(), whole.sebi());
        assert_eq!(split.stt(), whole.stt());
        assert_eq!(split.stamp(), whole.stamp());
        assert_eq!(split.turnover(), whole.turnover());
    }

    #[test]
    fn a_delivery_round_trip_is_two_days_and_the_sell_day_pays_the_dp_charge() {
        // D1: 50 @1520 bought one day, sold at 1530 on a later one.
        let bought = priced(DELIVERY, Exchange::Nse, &[buy(50, 1_520_00)]);
        assert_eq!(lines(&bought), [0, 2_33, 8, 76_00, 11_00, 43, 89_84]);
        assert_eq!(bought.dp(), Paisa::ZERO);
        let sold = priced(DELIVERY, Exchange::Nse, &[sell(50, 1_530_00)]);
        // STT 76.50 is an exact half and goes up: 77.
        assert_eq!(lines(&sold), [0, 2_35, 8, 77_00, 0, 44, 94_62]);
        assert_eq!(sold.dp(), rs(12_50));
        assert_eq!(sold.dp_gst(), rs(2_25));
        assert_eq!(bought.total().raw() + sold.total().raw(), 184_46);
        // Delivery STT has no named source and the day says so.
        assert_eq!(
            bought.unverified_sources().collect::<Vec<_>>(),
            [STT_EQUITY_DELIVERY.source()]
        );
        // MTF: the same taxes and DP, plus intraday-style brokerage.
        let mtf_buy = priced(MTF, Exchange::Nse, &[buy(50, 1_520_00)]);
        let mtf_sell = priced(MTF, Exchange::Nse, &[sell(50, 1_530_00)]);
        assert_eq!(mtf_buy.brokerage(), rs(20_00));
        assert_eq!(mtf_sell.brokerage(), rs(20_00));
        assert_eq!(mtf_buy.gst(), rs(4_03)); // 18% of 22.41
        assert_eq!(mtf_sell.gst(), rs(4_04)); // 18% of 22.43
        assert_eq!(mtf_sell.dp(), rs(12_50));
        assert_eq!(mtf_buy.total().raw() + mtf_sell.total().raw(), 231_66);
    }

    #[test]
    fn a_same_day_delivery_round_trip_is_netted_as_non_delivery() {
        // Equal sides under the delivery product: the exchange nets them, so
        // the intraday taxes apply, brokerage is the delivery product's (nil)
        // and no demat debit happens.
        let netted = priced(
            DELIVERY,
            Exchange::Nse,
            &[buy(50, 1_520_00), sell(50, 1_530_00)],
        );
        assert_eq!(netted.stt(), rs(19_00));
        assert_eq!(netted.stamp(), rs(2_00));
        assert_eq!(netted.brokerage(), Paisa::ZERO);
        assert_eq!(netted.dp(), Paisa::ZERO);
        assert_eq!(netted.total(), rs(4_68 + 15 + 19_00 + 2_00 + 87));
    }

    #[test]
    fn bse_equity_prices_at_the_unverified_all_in_rate_and_says_so() {
        let b1 = priced(
            INTRADAY,
            Exchange::Bse,
            &[buy(50, 1_520_00), sell(50, 1_530_00)],
        );
        // ₹375 per crore on ₹1,52,500: 5.71875 -> 5.72.
        assert_eq!(b1.exchange_charge(), rs(5_72));
        assert_eq!(
            b1.unverified_sources().collect::<Vec<_>>(),
            [BSE_EQUITY_EXCHANGE.source()]
        );
        assert!(b1.to_string().contains("UNVERIFIED rate used"));
        assert_eq!(b1.exchange(), Exchange::Bse);
    }

    #[test]
    fn futures_pay_sell_side_stt_and_buy_side_stamp_on_the_whole_value() {
        // NIFTY OCT FUT, 65 @25200 -> 25250.
        let fut = priced(
            TradeKind::Futures,
            Exchange::Nse,
            &[buy(65, 25_200_00), sell(65, 25_250_00)],
        );
        assert_eq!(fut.turnover(), rs(3_279_250_00));
        assert_eq!(
            lines(&fut),
            [40_00, 60_01, 3_28, 821_00, 33_00, 18_59, 975_88]
        );
        assert_eq!(
            fut.unverified_sources().collect::<Vec<_>>(),
            [STT_FUTURES.source()]
        );
        // A carried futures position is one-sided and still prices.
        let carried = priced(TradeKind::Futures, Exchange::Nse, &[buy(65, 25_200_00)]);
        assert_eq!(carried.stt(), Paisa::ZERO);
        assert_eq!(carried.stamp(), rs(33_00));
    }

    #[test]
    fn options_pay_tax_on_premium_and_the_exercise_rate_on_intrinsic_value() {
        // NIFTY 25300 CE, 65 @100 -> 120, today's FA73061 rate.
        let opt = priced(
            TradeKind::Options,
            Exchange::Nse,
            &[buy(65, 100_00), sell(65, 120_00)],
        );
        assert_eq!(lines(&opt), [40_00, 5_08, 1, 12_00, 0, 8_12, 65_21]);
        assert_eq!(opt.stt_exercise(), Paisa::ZERO);
        // An exercise: 65 units ₹150 in the money is ₹9,750 of intrinsic
        // value, taxed at 0.15%: 14.625 -> 15.
        let mut exercised = open(TradeKind::Options, Exchange::Nse);
        exercised.add(buy(65, 100_00)).expect("in range");
        exercised.exercise(rs(9_750_00)).expect("an option");
        let charges = exercised.settle().expect("settled");
        assert_eq!(charges.stt_exercise(), rs(15_00));
        assert_eq!(charges.exercised(), rs(9_750_00));
        assert_eq!(charges.rates_used()[5], Some(crate::schedule::STT_EXERCISE));
        // An exercise alone is a day: no order, still a tax.
        let mut only = open(TradeKind::Options, Exchange::Nse);
        only.exercise(rs(9_750_00)).expect("an option");
        let alone = only.settle().expect("an exercise is something to price");
        assert_eq!(alone.total(), rs(15_00));
        assert_eq!(alone.orders(), 0);
    }

    #[test]
    fn the_broker_square_off_fee_is_a_separate_line_applied_only_when_asked() {
        let mut csd = open(INTRADAY, Exchange::Nse);
        csd.add(buy(50, 1_520_00)).expect("in range");
        let exit = csd
            .add(sell(50, 1_530_00).squared_off_by_broker())
            .expect("an intraday exit");
        assert_eq!(exit.square_off_fee(), rs(20_00));
        assert_eq!(exit.square_off_gst(), rs(3_60));
        assert_eq!(exit.brokerage(), rs(20_00));
        assert_eq!(exit.value(), rs(76_500_00));
        let charges = csd.settle().expect("settled");
        assert_eq!(charges.square_off_fee(), rs(20_00));
        assert_eq!(charges.square_off_gst(), rs(3_60));
        // Exactly V1 plus ₹23.60, and nothing else moved.
        assert_eq!(charges.total(), rs(73_90 + 23_60));
        assert_eq!(charges.gst(), rs(8_07));
        // An order the client placed carries no fee.
        let client = csd.add(buy(1, 1_00)).expect("in range");
        assert_eq!(client.square_off_fee(), Paisa::ZERO);
        assert_eq!(client.square_off_gst(), Paisa::ZERO);
        // F&O positions can be squared off too; a delivery holding cannot.
        let mut fno = open(TradeKind::Options, Exchange::Nse);
        assert!(fno.add(sell(65, 120_00).squared_off_by_broker()).is_ok());
        for kind in [DELIVERY, MTF] {
            let mut held = open(kind, Exchange::Nse);
            assert_eq!(
                held.add(sell(50, 1_530_00).squared_off_by_broker()),
                Err(ChargeError::SquareOffNotApplicable {
                    kind: kind.as_str()
                })
            );
            assert_eq!(
                held,
                open(kind, Exchange::Nse),
                "a refused order changes nothing"
            );
        }
    }

    #[test]
    fn every_input_dhan_shows_as_zero_is_refused_by_name() {
        for (quantity, price, what, value) in [
            (0, 100_00, "order quantity", 0),
            (-5, 100_00, "order quantity", -5),
            (5, 0, "order price", 0),
            (5, -1, "order price", -1),
        ] {
            assert_eq!(
                Order::new(OrderSide::Buy, quantity, rs(price)),
                Err(ChargeError::Cost(CostError::NotPositive {
                    quantity: what,
                    value
                }))
            );
        }
        // A day with nothing in it is refused, not printed as zero.
        assert_eq!(
            open(INTRADAY, Exchange::Nse).settle(),
            Err(ChargeError::EmptyDay)
        );
        // An exercise of nothing, or of a non-option, is refused.
        let mut opt = open(TradeKind::Options, Exchange::Nse);
        for value in [0, -1] {
            assert_eq!(
                opt.exercise(rs(value)),
                Err(ChargeError::Cost(CostError::NotPositive {
                    quantity: "exercise intrinsic value",
                    value
                }))
            );
        }
        for kind in [INTRADAY, DELIVERY, MTF, TradeKind::Futures] {
            assert_eq!(
                open(kind, Exchange::Nse).exercise(rs(1_00)),
                Err(ChargeError::ExerciseNotApplicable {
                    kind: kind.as_str()
                })
            );
        }
    }

    #[test]
    fn an_open_intraday_day_and_a_partly_squared_delivery_day_are_refused() {
        for orders in [
            vec![buy(50, 1_520_00)],
            vec![sell(50, 1_530_00)],
            vec![buy(50, 1_520_00), sell(49, 1_530_00)],
            vec![buy(49, 1_520_00), sell(50, 1_530_00)],
        ] {
            let mut csd = open(INTRADAY, Exchange::Nse);
            let (mut bought, mut sold) = (0, 0);
            for order in &orders {
                csd.add(*order).expect("in range");
                match order.side() {
                    OrderSide::Buy => bought += order.quantity(),
                    OrderSide::Sell => sold += order.quantity(),
                }
            }
            assert_eq!(
                csd.settle(),
                Err(ChargeError::OpenIntradayPosition { bought, sold })
            );
        }
        for kind in [DELIVERY, MTF] {
            let mut csd = open(kind, Exchange::Nse);
            csd.add(buy(100, 1_520_00)).expect("in range");
            csd.add(sell(60, 1_530_00)).expect("in range");
            assert_eq!(
                csd.settle(),
                Err(ChargeError::PartlySquaredDay {
                    bought: 100,
                    sold: 60
                })
            );
        }
    }

    /// A day already at one edge, the order that pushes it over, and the words.
    type AtTheEdge = (fn(&mut ClientSecurityDay), Order, &'static str);

    #[test]
    fn every_overflow_site_is_refused_by_name_and_a_refused_order_changes_nothing() {
        let named = |operation| Some(ChargeError::Cost(CostError::Overflow { operation }));
        // The order value.
        assert_eq!(
            buy(i64::MAX, 2).value().err(),
            named("order quantity x price")
        );
        let mut csd = open(INTRADAY, Exchange::Nse);
        assert_eq!(
            csd.add(buy(i64::MAX, 2)).err(),
            named("order quantity x price")
        );

        // Each running total, from a day already at its edge.
        let edge = |set: fn(&mut ClientSecurityDay)| {
            let mut csd = open(INTRADAY, Exchange::Nse);
            set(&mut csd);
            csd
        };
        let cases: [AtTheEdge; 7] = [
            (|d| d.orders = i64::MAX, buy(1, 1), "the day's order count"),
            (
                |d| d.buy_quantity = i64::MAX,
                buy(1, 1),
                "the day's bought quantity",
            ),
            (
                |d| d.buy_value = rs(i64::MAX),
                buy(1, 1),
                "the day's buy value",
            ),
            (
                |d| d.sell_quantity = i64::MAX,
                sell(1, 1),
                "the day's sold quantity",
            ),
            (
                |d| d.sell_value = rs(i64::MAX),
                sell(1, 1),
                "the day's sell value",
            ),
            (
                |d| d.brokerage = rs(i64::MAX),
                buy(1, 100_00),
                "the day's brokerage",
            ),
            (
                |d| d.square_off_fees = rs(i64::MAX),
                buy(1, 1).squared_off_by_broker(),
                "the day's square-off fees",
            ),
        ];
        for (set, order, operation) in cases {
            let mut csd = edge(set);
            let before = csd;
            assert_eq!(csd.add(order).err(), named(operation));
            assert_eq!(csd, before, "{operation}: a refused order changes nothing");
        }
        let mut opt = open(TradeKind::Options, Exchange::Nse);
        opt.exercised = rs(i64::MAX);
        assert_eq!(
            opt.exercise(rs(1)).err(),
            named("the day's exercised value")
        );

        // Settlement: the turnover, the GST base and the total.
        let mut wide = open(TradeKind::Futures, Exchange::Nse);
        wide.orders = 2;
        wide.buy_value = rs(i64::MAX);
        wide.sell_value = rs(1);
        assert_eq!(wide.settle().err(), named("the day's turnover"));
        let mut base = open(TradeKind::Futures, Exchange::Nse);
        base.orders = 1;
        // ₹10 lakh of turnover: an exchange charge of ₹18.30 to add to it.
        base.buy_value = rs(1_000_000_00);
        base.brokerage = rs(i64::MAX);
        assert_eq!(base.settle().err(), named("the GST base"));
        let mut total = open(TradeKind::Futures, Exchange::Nse);
        total.orders = 1;
        total.buy_value = rs(1_00);
        total.square_off_fees = rs(i64::MAX - 1);
        assert_eq!(total.settle().err(), named("the day's total"));
    }

    #[test]
    fn a_settled_day_reports_its_inputs_its_lines_and_its_label() {
        let v1 = priced(
            INTRADAY,
            Exchange::Nse,
            &[buy(50, 1_520_00), sell(50, 1_530_00)],
        );
        assert_eq!(
            v1.basis(),
            RateBasis::TodaysRatesAppliedTo(day(2026, 10, 15))
        );
        assert_eq!(v1.kind(), INTRADAY);
        assert_eq!(v1.exchange(), Exchange::Nse);
        assert_eq!((v1.buy_quantity(), v1.sell_quantity()), (50, 50));
        assert_eq!(
            (v1.buy_value(), v1.sell_value()),
            (rs(76_000_00), rs(76_500_00))
        );
        assert_eq!(v1.exercised(), Paisa::ZERO);
        let sum: i64 = v1.lines().iter().map(|(_, amount)| amount.raw()).sum();
        assert_eq!(sum, v1.total().raw(), "the total is the sum of the lines");
        assert_eq!(
            v1.to_string(),
            "precise (contract-note method) — today's rates applied to 2026-10-15\n\
             equity intraday on NSE: bought 50 for ₹76000.00, sold 50 for ₹76500.00\n  \
             brokerage: ₹40.00\n  \
             exchange transaction charge (incl. IPFT): ₹4.68\n  \
             SEBI turnover fee: ₹0.15\n  \
             STT: ₹19.00\n  \
             STT on exercise: ₹0.00\n  \
             stamp duty: ₹2.00\n  \
             GST: ₹8.07\n  \
             DP charge: ₹0.00\n  \
             GST on DP charge: ₹0.00\n  \
             broker square-off fee: ₹0.00\n  \
             GST on broker square-off fee: ₹0.00\n  \
             total: ₹73.90"
        );
        assert_eq!(PRECISE_LABEL, "precise (contract-note method)");
        let order = buy(50, 1_520_00);
        assert_eq!(order.price(), rs(1_520_00));
        assert!(!order.is_broker_square_off());
        assert!(order.squared_off_by_broker().is_broker_square_off());
        assert_eq!(MAX_RATES_USED, 6);
    }

    #[test]
    fn a_day_opened_on_a_card_is_the_day_open_resolves() {
        let card = RateCard::resolve(
            INTRADAY,
            Exchange::Nse,
            CURRENT_AS_OF,
            RateSelector::default(),
        )
        .expect("priced");
        let from_card = ClientSecurityDay::with_card(card);
        assert_eq!(from_card.card(), &card);
        assert_eq!(
            ClientSecurityDay::open(
                INTRADAY,
                Exchange::Nse,
                CURRENT_AS_OF,
                RateSelector::default()
            ),
            Ok(from_card)
        );
        // And a day the dated selector cannot price is refused at open.
        assert!(
            ClientSecurityDay::open(INTRADAY, Exchange::Nse, CURRENT_AS_OF, RateSelector::Dated)
                .is_err()
        );
    }

    #[test]
    fn a_dated_option_day_before_fa73061_prices_at_the_regime_plus_ipft() {
        // 2026-02-27: FA64232's 0.03503% + IPFT 0.0005% = 0.03553%, STT 0.10%.
        let mut csd = ClientSecurityDay::open(
            TradeKind::Options,
            Exchange::Nse,
            day(2026, 2, 27),
            RateSelector::Dated,
        )
        .expect("a verified window");
        csd.add(buy(65, 100_00)).expect("in range");
        csd.add(sell(65, 120_00)).expect("in range");
        let charges = csd.settle().expect("settled");
        assert_eq!(charges.basis().to_string(), "rates in force on 2026-02-27");
        assert_eq!(charges.exchange_charge(), rs(5_08));
        assert_eq!(charges.stt(), rs(8_00)); // 0.10% of 7,800 = 7.80 -> 8
    }

    /// A writer that succeeds `budget` times and then fails for good, so
    /// every `?` in a `Display` is taken in turn (`crate::error`'s method).
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
        // A Display that swallowed a writer error would report success with a
        // truncated breakdown: a total with no lines, or lines with no label.
        let v1 = priced(
            INTRADAY,
            Exchange::Nse,
            &[buy(50, 1_520_00), sell(50, 1_530_00)],
        );
        let fut = priced(
            TradeKind::Futures,
            Exchange::Nse,
            &[buy(65, 25_200_00), sell(65, 25_250_00)],
        );
        assert_eq!(writes_needed(&v1), 145);
        assert_eq!(writes_needed(&fut), 147);
        assert!(
            writes_needed(&fut) < 256,
            "the budget is not itself the answer"
        );
        assert!(
            writes_needed(&fut) > writes_needed(&v1),
            "the UNVERIFIED line is written too"
        );
    }
}
