//! The precise charge stack: one executed order, one client-security-day,
//! one client-day, and the net result of one trade.
//!
//! This is the **default** mode of package CHG-1 — the contract-note method.
//! It prices what a broker's contract note would show for one client, one
//! security, one day:
//!
//! | Line | On | Rounded |
//! |---|---|---|
//! | brokerage (Dhan) | each executed order, and each option exercise (₹20, sources row B-05) | half-up to the paisa, per order |
//! | exchange transaction charge, IPFT included | the day's turnover | half-up to the paisa, once |
//! | SEBI turnover fee | the day's turnover | half-up to the paisa, once |
//! | STT | the day's value the tax is levied on | half-up to the paisa per security, then the client's day total half-up to the rupee |
//! | stamp duty | the day's buy value it is levied on | half-up to the paisa per security, then the client's day total half-up to the rupee |
//! | GST, 18% | brokerage + exchange charge + SEBI fee | half-up to the paisa, once |
//! | DP charge (Dhan), and its GST | a delivery or MTF sell day | exact |
//! | broker square-off fee, and its GST | each order the caller says the broker placed | exact |
//!
//! The two-step rounding of STT and stamp duty is what `NSE/CMTR/5588`
//! (section D) and `NCL/CMPT/43115` (Annexure 1, item 2) state for the cash
//! market: each security's figure to two decimals, the client's total for the
//! day to the nearest rupee, 50 paise and above going up. A
//! [`ClientSecurityDay`] settled on its own rounds as if it were the client's
//! only security that day; [`ClientDay`] rounds the client's day total once
//! across every security (D-5121). The direction of an exact half below a
//! paisa, the same two steps for F&O, and the level the other lines are
//! rounded at are stated by no source read, and are UNVERIFIED (D-5113,
//! D-5122).
//!
//! # The intraday transaction tax is on the day's VWAP, not on the sell value
//!
//! NSE levies the non-delivery STT on the **non-delivery quantity at the
//! volume-weighted average price of every trade the client made in the
//! security that day** — buys and sells together, the VWAP rounded to two
//! decimals (`NSE/CMTR/5588`). The operator's five contract-note vectors were
//! re-derived from that rule and every one agrees with it; three of them (V2,
//! V4, V5) disagree with the sell-value basis Dhan's calculator uses
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
//! # Net of every charge
//!
//! [`NetTrade`] is a trade's gross less **the entire precise charge stack** of
//! its days — brokerage on both legs, the exchange charge with IPFT, the SEBI
//! fee, STT, stamp duty, GST, the DP charge where it applies and the broker's
//! square-off fee when the broker exited — and a trade is a **win only if that
//! net is above zero** (D-5124).
//!
//! # Constant cost
//!
//! [`ClientSecurityDay::add`] is a fixed sequence of checked additions and
//! one [`dhan_brokerage`]; [`ClientSecurityDay::settle`] is a fixed sequence
//! of at most nine [`crate::exact::Ratio::apply`] calls, one VWAP division and
//! one fixed-length sum; [`ClientDay::add`] is six checked additions. None of
//! them walks the orders or the securities: each keeps running totals, so the
//! thousandth order costs what the first did. `crates/costs/benches/stack.rs`
//! measures all three (C-CHG-01 to C-CHG-06).

use std::fmt;

use brutex_core::instrument::Exchange;
use brutex_core::price::Paisa;

use crate::charge_error::ChargeError;
use crate::day::TradeDay;
use crate::error::CostError;
use crate::exact::{Exact, Quantum, Rupees, half_up, round_half_up, units_of};
use crate::money::narrow;
use crate::rate::OrderSide;
use crate::schedule::{
    Cited, DHAN_DP_CHARGE, DHAN_FNO_BROKERAGE, DHAN_SQUARE_OFF_FEE, GST, Mode,
    PHYSICAL_DELIVERY_REFUSAL, Product, RateBasis, RateCard, RateSelector, Segment, Settlement,
    Status, Taxes, TradeKind, dhan_brokerage, dhan_brokerage_units,
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

/// One client-security-day's running totals: what [`ClientSecurityDay`] and
/// [`crate::ledger::DayLedger`] both keep. [`Self::add`] is a fixed handful of
/// checked additions and one brokerage; nothing walks the orders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Totals {
    orders: i64,
    exercises: i64,
    buy_quantity: i64,
    buy_value: Paisa,
    sell_quantity: i64,
    sell_value: Paisa,
    brokerage: Paisa,
    brokerage_exact: Exact,
    square_off_fees: Paisa,
    exercised: Paisa,
}

impl Totals {
    /// A day with nothing in it.
    pub(crate) const EMPTY: Self = Self {
        orders: 0,
        exercises: 0,
        buy_quantity: 0,
        buy_value: Paisa::ZERO,
        sell_quantity: 0,
        sell_value: Paisa::ZERO,
        brokerage: Paisa::ZERO,
        brokerage_exact: Exact::ZERO,
        square_off_fees: Paisa::ZERO,
        exercised: Paisa::ZERO,
    };

    /// How many executed orders.
    pub(crate) const fn orders(&self) -> i64 {
        self.orders
    }

    /// One side's units and value so far.
    pub(crate) const fn side(&self, side: OrderSide) -> (i64, Paisa) {
        match side {
            OrderSide::Buy => (self.buy_quantity, self.buy_value),
            OrderSide::Sell => (self.sell_quantity, self.sell_value),
        }
    }

    /// Records one executed order of `kind`: `quantity` units worth `value`,
    /// placed by the broker if `by_broker`. Returns the order's brokerage,
    /// half-up to the paisa. All or nothing: a refused order leaves the
    /// totals exactly as they were.
    ///
    /// # Errors
    ///
    /// * [`ChargeError::SquareOffNotApplicable`] for a broker square-off on a
    ///   delivery or MTF day.
    /// * [`ChargeError::Cost`] carrying an overflow of a day total.
    pub(crate) fn add(
        &mut self,
        kind: TradeKind,
        side: OrderSide,
        quantity: i64,
        value: Paisa,
        by_broker: bool,
    ) -> Result<Paisa, ChargeError> {
        if by_broker && !kind.can_be_squared_off_by_broker() {
            return Err(ChargeError::SquareOffNotApplicable {
                kind: kind.as_str(),
            });
        }
        let brokerage = dhan_brokerage(kind, value)?;
        let brokerage_exact = dhan_brokerage_units(kind, value)?;
        let square_off_fee = if by_broker {
            DHAN_SQUARE_OFF_FEE
        } else {
            Paisa::ZERO
        };

        let mut next = *self;
        next.orders = self
            .orders
            .checked_add(1)
            .ok_or(overflow("the day's order count"))?;
        match side {
            OrderSide::Buy => {
                next.buy_quantity =
                    add_units(self.buy_quantity, quantity, "the day's bought quantity")?;
                next.buy_value = add(self.buy_value, value, "the day's buy value")?;
            }
            OrderSide::Sell => {
                next.sell_quantity =
                    add_units(self.sell_quantity, quantity, "the day's sold quantity")?;
                next.sell_value = add(self.sell_value, value, "the day's sell value")?;
            }
        }
        next.brokerage = add(self.brokerage, brokerage, "the day's brokerage")?;
        next.brokerage_exact = Exact::sum(
            &[self.brokerage_exact, brokerage_exact],
            "the day's unrounded brokerage",
        )?;
        next.square_off_fees = add(
            self.square_off_fees,
            square_off_fee,
            "the day's square-off fees",
        )?;
        *self = next;
        Ok(brokerage)
    }

    /// Records an exercise of an option worth `intrinsic_value` on a day of
    /// `kind`, settled as `settlement` says, and the ₹20 brokerage Dhan
    /// charges on it: its pricing page applies the per-order ₹20 to an
    /// option *"expired, exercised, or assigned"* (sources row B-05), so an
    /// exercise is billed as one more executed order would be, and its GST
    /// follows through the day's GST base (D-5132). All or nothing: a
    /// refused exercise leaves the totals exactly as they were.
    ///
    /// # Errors
    ///
    /// * [`ChargeError::ExerciseNotApplicable`] on a day that is not options.
    /// * [`PHYSICAL_DELIVERY_REFUSAL`] for [`Settlement::PhysicalDelivery`].
    /// * [`ChargeError::Cost`] carrying [`CostError::NotPositive`] for an
    ///   intrinsic value that is not positive, or an overflow of the day's
    ///   exercise count, exercised value or brokerage.
    pub(crate) fn exercise(
        &mut self,
        kind: TradeKind,
        intrinsic_value: Paisa,
        settlement: Settlement,
    ) -> Result<(), ChargeError> {
        if kind != TradeKind::Options {
            return Err(ChargeError::ExerciseNotApplicable {
                kind: kind.as_str(),
            });
        }
        if settlement == Settlement::PhysicalDelivery {
            return Err(PHYSICAL_DELIVERY_REFUSAL);
        }
        if intrinsic_value.raw() <= 0 {
            return Err(CostError::NotPositive {
                quantity: "exercise intrinsic value",
                value: intrinsic_value.raw(),
            }
            .into());
        }
        let mut next = *self;
        next.exercises = self
            .exercises
            .checked_add(1)
            .ok_or(overflow("the day's exercise count"))?;
        next.exercised = add(self.exercised, intrinsic_value, "the day's exercised value")?;
        // B-05 names an exercised option, so its brokerage is the F&O ₹20
        // itself, never a function of the intrinsic value.
        next.brokerage = add(self.brokerage, DHAN_FNO_BROKERAGE, "the day's brokerage")?;
        next.brokerage_exact = Exact::sum(
            &[self.brokerage_exact, Exact::of_paisa(DHAN_FNO_BROKERAGE)],
            "the day's unrounded brokerage",
        )?;
        *self = next;
        Ok(())
    }
}

/// One client's trades in one security on one day, as running totals.
///
/// The rate card is resolved once, when the day opens. Each [`Self::add`]
/// updates the totals in constant time; [`Self::settle`] prices them. The
/// thousandth order costs what the first did: C-CHG-01 and C-CHG-02 in
/// `crates/costs/benches/stack.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientSecurityDay {
    card: RateCard,
    totals: Totals,
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
    ///     RateSelector::CurrentOnEveryDate,
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
            totals: Totals::EMPTY,
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
        let square_off_gst = if order.by_broker {
            self.card
                .gst()
                .rate()
                .apply(DHAN_SQUARE_OFF_FEE, 1, Quantum::Paisa)?
        } else {
            Paisa::ZERO
        };
        let brokerage =
            self.totals
                .add(kind, order.side, order.quantity, value, order.by_broker)?;
        Ok(OrderCharges {
            value,
            brokerage,
            square_off_fee: if order.by_broker {
                DHAN_SQUARE_OFF_FEE
            } else {
                Paisa::ZERO
            },
            square_off_gst,
        })
    }

    /// Records an exercise of an option worth `intrinsic_value`, on which
    /// STT is levied at the exercise rate, and which pays Dhan's ₹20
    /// brokerage, and GST on it, as an executed order does (sources row B-05,
    /// D-5132). The caller names the settlement: only [`Settlement::Cash`] is
    /// priced (D-5130).
    ///
    /// # Errors
    ///
    /// * [`ChargeError::ExerciseNotApplicable`] on a day that is not options.
    /// * [`PHYSICAL_DELIVERY_REFUSAL`] for [`Settlement::PhysicalDelivery`]:
    ///   a stock option settled by delivery is not priced, and the day is
    ///   left exactly as it was.
    /// * [`ChargeError::Cost`] carrying [`CostError::NotPositive`] for an
    ///   intrinsic value that is not positive — an out-of-the-money option is
    ///   not exercised — or an overflow of the day's exercised value.
    pub fn exercise(
        &mut self,
        intrinsic_value: Paisa,
        settlement: Settlement,
    ) -> Result<(), ChargeError> {
        self.totals
            .exercise(self.card.kind(), intrinsic_value, settlement)
    }

    /// Prices the day, as if it were the client's only security that day: STT
    /// and stamp duty are rounded to the paisa for this security and then to
    /// the rupee. [`ClientDay`] rounds them once across several securities.
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
        settle_precise(&self.card, &self.totals)
    }
}

/// The names of a day's eleven lines, in contract-note order — the order of
/// [`DayCharges::lines`], [`DayCharges::unrounded_lines`] and
/// `crate::ledger::order_lines_unrounded`.
pub const LINE_NAMES: [&str; 11] = [
    "brokerage",
    "exchange transaction charge (incl. IPFT)",
    "SEBI turnover fee",
    "STT",
    "STT on exercise",
    "stamp duty",
    "GST",
    "DP charge",
    "GST on DP charge",
    "broker square-off fee",
    "GST on broker square-off fee",
];

/// Prices one client-security-day's totals on `card` under the precise
/// method: [`ClientSecurityDay::settle`], and `crate::ledger::DayLedger::close`
/// in [`Mode::Precise`].
///
/// # Errors
///
/// As [`ClientSecurityDay::settle`].
pub(crate) fn settle_precise(card: &RateCard, t: &Totals) -> Result<DayCharges, ChargeError> {
    if t.orders == 0 && t.exercised.raw() == 0 {
        return Err(ChargeError::EmptyDay);
    }
    let turnover = add(t.buy_value, t.sell_value, "the day's turnover")?;
    let exchange_charge = card
        .exchange_charge()
        .rate()
        .apply(turnover, 1, Quantum::Paisa)?;
    let sebi = card.sebi().rate().apply(turnover, 1, Quantum::Paisa)?;

    let levied = taxes(card, t, turnover)?;
    // The rupee lines of a day priced alone: the trade STT, and what the
    // exercise adds to the day's STT once both are rounded together, so
    // the two lines sum to the day's STT rounded once.
    let stt_paisa = add(
        levied.stt,
        levied.stt_exercise,
        "the day's STT before the rupee",
    )?;
    let stt = to_rupee(levied.stt)?;
    let stt_exercise = Paisa::from_raw(to_rupee(stt_paisa)?.raw() - stt.raw());
    let stamp = to_rupee(levied.stamp)?;
    let dp = if levied.delivery_sell {
        DHAN_DP_CHARGE
    } else {
        Paisa::ZERO
    };
    let gst_rate = card.gst().rate();
    let base = sum(&[t.brokerage, exchange_charge, sebi], "the GST base")?;
    let gst = gst_rate.apply(base, 1, Quantum::Paisa)?;
    let dp_gst = gst_rate.apply(dp, 1, Quantum::Paisa)?;
    let square_off_gst = gst_rate.apply(t.square_off_fees, 1, Quantum::Paisa)?;
    let fees = sum(
        &[
            t.brokerage,
            exchange_charge,
            sebi,
            gst,
            dp,
            dp_gst,
            t.square_off_fees,
            square_off_gst,
        ],
        "the day's fees",
    )?;
    let total = sum(&[fees, stt, stt_exercise, stamp], "the day's total")?;

    // The same lines before any rounding. GST is on the base a note bills —
    // the rounded brokerage, exchange charge and SEBI fee — exactly.
    let exercise_exact = match levied.exercise_rate {
        Some(rate) => units_of(t.exercised, rate.rate(), 1)?,
        None => Exact::ZERO,
    };
    let unrounded = [
        t.brokerage_exact,
        units_of(turnover, card.exchange_charge().rate(), 1)?,
        units_of(turnover, card.sebi().rate(), 1)?,
        units_of(levied.stt_base, levied.stt_rate.rate(), 1)?,
        exercise_exact,
        units_of(levied.stamp_base, levied.stamp_rate.rate(), 1)?,
        units_of(base, gst_rate, 1)?,
        Exact::of_paisa(dp),
        units_of(dp, gst_rate, 1)?,
        Exact::of_paisa(t.square_off_fees),
        units_of(t.square_off_fees, gst_rate, 1)?,
    ];
    Ok(DayCharges {
        mode: Mode::Precise,
        basis: card.basis(),
        kind: card.kind(),
        exchange: card.exchange(),
        orders: t.orders,
        exercises: t.exercises,
        buy_quantity: t.buy_quantity,
        buy_value: t.buy_value,
        sell_quantity: t.sell_quantity,
        sell_value: t.sell_value,
        exercised: t.exercised,
        turnover,
        brokerage: t.brokerage,
        exchange_charge,
        sebi,
        stt,
        stt_exercise,
        stamp,
        stt_paisa: levied.stt,
        stt_exercise_paisa: levied.stt_exercise,
        stamp_paisa: levied.stamp,
        gst,
        dp,
        dp_gst,
        square_off_fee: t.square_off_fees,
        square_off_gst,
        fees,
        total,
        rounding_adjustment: adjustment(total, &unrounded)?,
        unrounded,
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

/// Prices one client-security-day's totals as Dhan's calculator page would:
/// `crate::ledger::DayLedger::close` in [`Mode::DhanCalculator`]. The page's
/// own lines ([`crate::dhan_calc`]), and the broker square-off fee — which
/// the page has no input for — as its own two lines on top. Labelled
/// `Dhan calculator (estimate)` and never a decision: a [`ClientDay`] and a
/// [`NetTrade`] refuse it.
///
/// # Errors
///
/// * [`ChargeError::EmptyDay`] when no order was executed.
/// * [`ChargeError::NotOnPage`] for an exercise (the page has no exercise
///   input) and for what the page was never read for.
/// * [`ChargeError::Cost`] carrying an overflow.
pub(crate) fn settle_dhan(
    kind: TradeKind,
    exchange: Exchange,
    day: TradeDay,
    t: &Totals,
) -> Result<DayCharges, ChargeError> {
    if t.exercised.raw() > 0 {
        return Err(ChargeError::NotOnPage {
            what: "an option exercise (the calculator page has no exercise input)",
        });
    }
    if t.orders == 0 {
        return Err(ChargeError::EmptyDay);
    }
    let turnover = add(t.buy_value, t.sell_value, "the day's turnover")?;
    let page = crate::dhan_calc::page_lines(
        kind,
        exchange,
        t.buy_value,
        t.sell_value,
        t.brokerage_exact,
        t.sell_quantity > 0,
    )?;
    let square_off = Exact::of_paisa(t.square_off_fees);
    let square_off_gst = units_of(t.square_off_fees, GST.rate(), 1)?;
    let exact_total = Exact::sum(
        &[page.total, square_off, square_off_gst],
        "the calculator day's total",
    )?;
    let unrounded = [
        page.brokerage,
        page.exchange_charge,
        page.sebi,
        page.stt,
        Exact::ZERO,
        page.stamp,
        page.gst,
        page.dp,
        Exact::ZERO,
        square_off,
        square_off_gst,
    ];
    let brokerage = page.brokerage.to_paisa()?;
    let exchange_charge = page.exchange_charge.to_paisa()?;
    let sebi = page.sebi.to_paisa()?;
    let gst = page.gst.to_paisa()?;
    let dp = page.dp.to_paisa()?;
    let square_off_gst_paisa = square_off_gst.to_paisa()?;
    let fees = sum(
        &[
            brokerage,
            exchange_charge,
            sebi,
            gst,
            dp,
            t.square_off_fees,
            square_off_gst_paisa,
        ],
        "the calculator day's fees",
    )?;
    let total = exact_total.to_paisa()?;
    Ok(DayCharges {
        mode: Mode::DhanCalculator,
        basis: RateBasis::TodaysRatesAppliedTo(day),
        kind,
        exchange,
        orders: t.orders,
        exercises: 0,
        buy_quantity: t.buy_quantity,
        buy_value: t.buy_value,
        sell_quantity: t.sell_quantity,
        sell_value: t.sell_value,
        exercised: Paisa::ZERO,
        turnover,
        brokerage,
        exchange_charge,
        sebi,
        stt: page.stt.to_whole_rupee()?,
        stt_exercise: Paisa::ZERO,
        stamp: page.stamp.to_whole_rupee()?,
        stt_paisa: page.stt.to_paisa()?,
        stt_exercise_paisa: Paisa::ZERO,
        stamp_paisa: page.stamp.to_paisa()?,
        gst,
        dp,
        dp_gst: Paisa::ZERO,
        square_off_fee: t.square_off_fees,
        square_off_gst: square_off_gst_paisa,
        fees,
        total,
        rounding_adjustment: adjustment(total, &unrounded)?,
        unrounded,
        rates_used: [None; MAX_RATES_USED],
    })
}

/// `total − Σ unrounded`: what rounding added to, or took from, the day.
///
/// # Errors
///
/// [`ChargeError::Cost`] carrying an overflow of the unrounded sum.
fn adjustment(total: Paisa, unrounded: &[Exact; 11]) -> Result<Exact, ChargeError> {
    let exact = Exact::sum(unrounded, "the day's unrounded lines")?;
    // `total` in units is in [0, 2^87) and `exact` in [0, i128::MAX] — every
    // line is a non-negative rate on a non-negative value — so the difference
    // lies in (-i128::MAX, 2^87): it cannot wrap.
    Ok(Exact(Exact::of_paisa(total).units() - exact.units()))
}

/// The transaction tax and the stamp duty of one security's day, by kind,
/// each rounded half-up to the paisa — the per-security step both circulars
/// state before the client's day total is rounded to the rupee — with the
/// value each was levied on.
fn taxes(card: &RateCard, t: &Totals, turnover: Paisa) -> Result<Levied, ChargeError> {
    match card.taxes() {
        Taxes::Equity {
            product,
            stt_non_delivery,
            stt_delivery,
            stamp_non_delivery,
            stamp_delivery,
        } => {
            match equity_netting(product, t.buy_quantity, t.sell_quantity)? {
                // Squared off: all non-delivery. NSE/CMTR/5588: the value
                // is the non-delivery quantity at the day's VWAP of every
                // trade, the VWAP itself rounded to two decimals.
                Netting::Squared => Levied::of(
                    (
                        non_delivery_value(turnover, t.buy_quantity)?,
                        stt_non_delivery,
                    ),
                    (t.buy_value, stamp_non_delivery),
                    false,
                ),
                // One side only: delivery. The tax is on that side's value,
                // the stamp on the buy value, and a sell debits the demat.
                Netting::OneSided => Levied::of(
                    (turnover, stt_delivery),
                    (t.buy_value, stamp_delivery),
                    t.sell_quantity > 0,
                ),
            }
        }
        Taxes::Futures {
            stt_sell,
            stamp_buy,
        } => Levied::of((t.sell_value, stt_sell), (t.buy_value, stamp_buy), false),
        Taxes::Options {
            stt_sell,
            stt_exercise,
            stamp_buy,
        } => {
            let mut levied = Levied::of((t.sell_value, stt_sell), (t.buy_value, stamp_buy), false)?;
            levied.stt_exercise = stt_exercise.rate().apply(t.exercised, 1, Quantum::Paisa)?;
            levied.exercise_rate = Some(stt_exercise);
            Ok(levied)
        }
    }
}

/// How an equity client-security-day nets, as the exchange nets it: by the
/// day's quantities, never by the product label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Netting {
    /// Equal units bought and sold: all non-delivery, whatever the product
    /// (`NSE/CMTR/5588`) — non-delivery STT and stamp duty, and no DP.
    Squared,
    /// Units on one side only, on a delivery or MTF day: delivery — delivery
    /// STT and stamp duty, and DP on a sell.
    OneSided,
}

/// The one rule that nets an equity day: [`taxes`] prices the day by it and
/// `crate::ledger::DayLedger::order_lines_unrounded` prices each of the day's
/// orders by it, so an order's lines and its day's can never be netted two
/// ways.
///
/// # Errors
///
/// * [`ChargeError::OpenIntradayPosition`] for an intraday day whose sides
///   differ: its exit order is missing.
/// * [`ChargeError::PartlySquaredDay`] for a delivery or MTF day with both
///   sides of unequal size: its split needs the FIFO order sequence.
pub(crate) fn equity_netting(
    product: Product,
    bought: i64,
    sold: i64,
) -> Result<Netting, ChargeError> {
    if bought == sold {
        return Ok(Netting::Squared);
    }
    if product == Product::Intraday {
        return Err(ChargeError::OpenIntradayPosition { bought, sold });
    }
    if bought > 0 && sold > 0 {
        return Err(ChargeError::PartlySquaredDay { bought, sold });
    }
    Ok(Netting::OneSided)
}

/// `NSE/CMTR/5588`'s non-delivery value of a squared-off day: the quantity
/// `units` (bought, and the same sold) at the day's VWAP, where the VWAP is
/// `(BVAL + SVAL) / (BQTY + SQTY)` rounded half-up to the paisa. Which way the
/// circular sends an exact half paisa is not stated; half-up is UNVERIFIED.
fn non_delivery_value(turnover: Paisa, units: i64) -> Result<Paisa, ChargeError> {
    let traded = i128::from(units) * 2;
    let vwap = round_half_up(i128::from(turnover.raw()), traded)?;
    Ok(narrow(vwap * i128::from(units), "the non-delivery value")?)
}

/// A paisa figure rounded half-up to the whole rupee, in paisa.
fn to_rupee(amount: Paisa) -> Result<Paisa, ChargeError> {
    Ok(narrow(
        half_up::<100>(i128::from(amount.raw())) * 100,
        "a charge rounded to the rupee",
    )?)
}

/// The two taxes of one security's day, each to the paisa, the values they
/// were levied on, and the rates that levied them.
struct Levied {
    stt: Paisa,
    stt_base: Paisa,
    stt_rate: Cited,
    stt_exercise: Paisa,
    exercise_rate: Option<Cited>,
    stamp: Paisa,
    stamp_base: Paisa,
    stamp_rate: Cited,
    delivery_sell: bool,
}

impl Levied {
    /// The common shape, with no exercise: every arm of [`taxes`] builds
    /// through it. Each tax is `(the value it is levied on, its rate)`.
    fn of(
        (stt_base, stt_rate): (Paisa, Cited),
        (stamp_base, stamp_rate): (Paisa, Cited),
        delivery_sell: bool,
    ) -> Result<Self, ChargeError> {
        Ok(Self {
            stt: stt_rate.rate().apply(stt_base, 1, Quantum::Paisa)?,
            stt_base,
            stt_rate,
            stt_exercise: Paisa::ZERO,
            exercise_rate: None,
            stamp: stamp_rate.rate().apply(stamp_base, 1, Quantum::Paisa)?,
            stamp_base,
            stamp_rate,
            delivery_sell,
        })
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
    mode: Mode,
    basis: RateBasis,
    kind: TradeKind,
    exchange: Exchange,
    orders: i64,
    exercises: i64,
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
    stt_paisa: Paisa,
    stt_exercise_paisa: Paisa,
    stamp_paisa: Paisa,
    gst: Paisa,
    dp: Paisa,
    dp_gst: Paisa,
    square_off_fee: Paisa,
    square_off_gst: Paisa,
    fees: Paisa,
    total: Paisa,
    unrounded: [Exact; 11],
    rounding_adjustment: Exact,
    rates_used: [Option<Cited>; MAX_RATES_USED],
}

impl DayCharges {
    /// Which arithmetic priced the day: precise, or the Dhan calculator
    /// replica, labelled an estimate.
    #[must_use]
    pub const fn mode(&self) -> Mode {
        self.mode
    }
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
    /// How many option exercises: each pays the ₹20 brokerage of an
    /// executed order (sources row B-05), counted in [`Self::brokerage`] and
    /// not in [`Self::orders`].
    #[must_use]
    pub const fn exercises(&self) -> i64 {
        self.exercises
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
    /// Dhan's brokerage, summed over the orders and the exercises — ₹20 on
    /// each exercise (sources row B-05, D-5132).
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
    /// Every line, summed: the day's charges as if it were the client's only
    /// security that day.
    #[must_use]
    pub const fn total(&self) -> Paisa {
        self.total
    }
    /// This security's STT on trades, half-up to the paisa — the
    /// per-security figure `NSE/CMTR/5588` rounds before the client's day
    /// total is rounded to the rupee ([`ClientDay`]).
    #[must_use]
    pub const fn stt_paisa(&self) -> Paisa {
        self.stt_paisa
    }
    /// This security's STT on exercise, half-up to the paisa.
    #[must_use]
    pub const fn stt_exercise_paisa(&self) -> Paisa {
        self.stt_exercise_paisa
    }
    /// This security's stamp duty, half-up to the paisa
    /// (`NCL/CMPT/43115`, Annexure 1 item 1.2(b)).
    #[must_use]
    pub const fn stamp_paisa(&self) -> Paisa {
        self.stamp_paisa
    }
    /// Every line except STT and stamp duty: brokerage, the exchange charge,
    /// the SEBI fee, GST, the DP charge and its GST, the square-off fee and
    /// its GST.
    #[must_use]
    pub const fn fees(&self) -> Paisa {
        self.fees
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

    /// The lines, named ([`LINE_NAMES`]), in contract-note order, as charged.
    #[must_use]
    pub const fn lines(&self) -> [(&'static str, Paisa); 11] {
        let [n0, n1, n2, n3, n4, n5, n6, n7, n8, n9, n10] = LINE_NAMES;
        [
            (n0, self.brokerage),
            (n1, self.exchange_charge),
            (n2, self.sebi),
            (n3, self.stt),
            (n4, self.stt_exercise),
            (n5, self.stamp),
            (n6, self.gst),
            (n7, self.dp),
            (n8, self.dp_gst),
            (n9, self.square_off_fee),
            (n10, self.square_off_gst),
        ]
    }

    /// The same lines **before any rounding**, in units of 10⁻⁹ rupee
    /// ([`Exact`]). Precise: each line's exact value on what it is levied on —
    /// brokerage summed over the orders unrounded, STT on the
    /// `NSE/CMTR/5588` value (the VWAP is itself rounded to the paisa: that
    /// is the method, not a line's rounding), GST on the billed base.
    /// Calculator: the page's own unrounded lines.
    #[must_use]
    pub const fn unrounded_lines(&self) -> [(&'static str, Exact); 11] {
        let [n0, n1, n2, n3, n4, n5, n6, n7, n8, n9, n10] = LINE_NAMES;
        let [u0, u1, u2, u3, u4, u5, u6, u7, u8, u9, u10] = self.unrounded;
        [
            (n0, u0),
            (n1, u1),
            (n2, u2),
            (n3, u3),
            (n4, u4),
            (n5, u5),
            (n6, u6),
            (n7, u7),
            (n8, u8),
            (n9, u9),
            (n10, u10),
        ]
    }

    /// [`Self::total`] less the exact sum of [`Self::unrounded_lines`]: what
    /// rounding added to the day (positive) or took from it (negative).
    #[must_use]
    pub const fn rounding_adjustment(&self) -> Exact {
        self.rounding_adjustment
    }
}

impl fmt::Display for DayCharges {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} — {}", self.mode.label(), self.basis)?;
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

// ---------------------------------------------------------------------------
// One client-day: the level STT and stamp duty are rounded to the rupee at
// ---------------------------------------------------------------------------

/// One client's day in one segment of one exchange, as running totals of its
/// settled securities.
///
/// `NSE/CMTR/5588` (section D) rounds the client's STT for the day to the
/// nearest rupee and `NCL/CMPT/43115` (Annexure 1, item 2) does the same for
/// stamp duty, each over the per-security figures already rounded to two
/// decimals. A [`DayCharges`] settled alone rounds as if its security were
/// the only one; this rounds the client's total once. Every other line is
/// summed as each security priced it. [`Self::add`] is six checked additions
/// and [`Self::settle`] two roundings, whatever the number of securities —
/// measured by C-CHG-06 in `crates/costs/benches/stack.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientDay {
    key: Option<(RateBasis, Exchange, Segment)>,
    securities: i64,
    fees: Paisa,
    stt_paisa: Paisa,
    stamp_paisa: Paisa,
    rounded_alone: Paisa,
}

impl ClientDay {
    /// A client-day with no security in it yet.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            key: None,
            securities: 0,
            fees: Paisa::ZERO,
            stt_paisa: Paisa::ZERO,
            stamp_paisa: Paisa::ZERO,
            rounded_alone: Paisa::ZERO,
        }
    }

    /// Adds one settled security. All or nothing: a refused security leaves
    /// the client-day exactly as it was.
    ///
    /// # Errors
    ///
    /// * [`ChargeError::MixedClientDay`] for a security priced on another
    ///   day or rate set, on another exchange, or in another segment.
    /// * [`ChargeError::Cost`] carrying an overflow of a running total.
    ///
    /// # Examples
    ///
    /// ```
    /// use brutex_core::instrument::Exchange;
    /// use brutex_core::price::Paisa;
    /// use costs::day::TradeDay;
    /// use costs::rate::OrderSide;
    /// use costs::schedule::{Product, RateSelector, TradeKind};
    /// use costs::stack::{ClientDay, ClientSecurityDay, Order};
    ///
    /// let mut client = ClientDay::new();
    /// for price in [400_00, 404_00] {
    ///     let mut day = ClientSecurityDay::open(
    ///         TradeKind::Equity(Product::Intraday),
    ///         Exchange::Nse,
    ///         TradeDay::new(2026, 10, 15)?,
    ///         RateSelector::CurrentOnEveryDate,
    ///     )?;
    ///     day.add(Order::new(OrderSide::Buy, 3, Paisa::from_raw(price))?)?;
    ///     day.add(Order::new(OrderSide::Sell, 3, Paisa::from_raw(price))?)?;
    ///     client.add(&day.settle()?)?;
    /// }
    /// // Each security's STT is ₹0.30 and rounds to nothing alone;
    /// // the client's 0.60 rounds to one rupee.
    /// assert_eq!(client.settle()?.stt().raw(), 1_00);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn add(&mut self, day: &DayCharges) -> Result<(), ChargeError> {
        not_an_estimate(day, "a client-day's STT and stamp duty")?;
        let key = (day.basis, day.exchange, day.kind.segment());
        if let Some((basis, exchange, segment)) = self.key {
            // Three fixed comparisons, the first that differs named.
            let differs = [
                (basis != key.0, "trade day or rate set"),
                (exchange != key.1, "exchange"),
                (segment != key.2, "segment"),
            ]
            .into_iter()
            .find_map(|(differ, what)| differ.then_some(what));
            if let Some(differs) = differs {
                return Err(ChargeError::MixedClientDay { differs });
            }
        }
        let next = Self {
            key: Some(key),
            securities: add_units(self.securities, 1, "the client-day's securities")?,
            fees: add(self.fees, day.fees, "the client-day's fees")?,
            stt_paisa: sum(
                &[self.stt_paisa, day.stt_paisa, day.stt_exercise_paisa],
                "the client-day's STT",
            )?,
            stamp_paisa: add(
                self.stamp_paisa,
                day.stamp_paisa,
                "the client-day's stamp duty",
            )?,
            rounded_alone: add(
                self.rounded_alone,
                day.total,
                "the client-day's totals priced alone",
            )?,
        };
        *self = next;
        Ok(())
    }

    /// Prices the client-day: STT and stamp duty rounded to the rupee once,
    /// on the client's total.
    ///
    /// # Errors
    ///
    /// * [`ChargeError::EmptyDay`] when no security was added.
    /// * [`ChargeError::Cost`] carrying an overflow.
    pub fn settle(&self) -> Result<ClientDayCharges, ChargeError> {
        let Some((basis, exchange, segment)) = self.key else {
            return Err(ChargeError::EmptyDay);
        };
        let stt = to_rupee(self.stt_paisa)?;
        let stamp = to_rupee(self.stamp_paisa)?;
        Ok(ClientDayCharges {
            basis,
            exchange,
            segment,
            securities: self.securities,
            fees: self.fees,
            stt,
            stamp,
            total: sum(&[self.fees, stt, stamp], "the client-day's total")?,
            priced_alone: self.rounded_alone,
        })
    }
}

impl Default for ClientDay {
    fn default() -> Self {
        Self::new()
    }
}

/// One client-day, priced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientDayCharges {
    basis: RateBasis,
    exchange: Exchange,
    segment: Segment,
    securities: i64,
    fees: Paisa,
    stt: Paisa,
    stamp: Paisa,
    total: Paisa,
    priced_alone: Paisa,
}

impl ClientDayCharges {
    /// The label: which rate set, applied to which day.
    #[must_use]
    pub const fn basis(&self) -> RateBasis {
        self.basis
    }
    /// Where.
    #[must_use]
    pub const fn exchange(&self) -> Exchange {
        self.exchange
    }
    /// The segment.
    #[must_use]
    pub const fn segment(&self) -> Segment {
        self.segment
    }
    /// How many securities the client traded that day.
    #[must_use]
    pub const fn securities(&self) -> i64 {
        self.securities
    }
    /// Every line other than STT and stamp duty, summed as each security
    /// priced it.
    #[must_use]
    pub const fn fees(&self) -> Paisa {
        self.fees
    }
    /// The client's STT for the day — trades and exercises — rounded to the
    /// rupee once.
    #[must_use]
    pub const fn stt(&self) -> Paisa {
        self.stt
    }
    /// The client's stamp duty for the day, rounded to the rupee once.
    #[must_use]
    pub const fn stamp(&self) -> Paisa {
        self.stamp
    }
    /// The client-day's charges: fees + STT + stamp duty.
    #[must_use]
    pub const fn total(&self) -> Paisa {
        self.total
    }
    /// The sum of every security's total priced alone. It differs from
    /// [`Self::total`] only by the rupee rounding of STT and stamp duty, by
    /// at most half a rupee per security for each.
    #[must_use]
    pub const fn priced_alone(&self) -> Paisa {
        self.priced_alone
    }
}

/// Refuses a day priced by the calculator replica where only the precise
/// stack may decide: a client-day's rounding and a trade's net (D-5124).
///
/// # Errors
///
/// [`ChargeError::NotPrecise`] naming `figure`.
fn not_an_estimate(day: &DayCharges, figure: &'static str) -> Result<(), ChargeError> {
    match day.mode {
        Mode::Precise => Ok(()),
        Mode::DhanCalculator => Err(ChargeError::NotPrecise { figure }),
    }
}

// ---------------------------------------------------------------------------
// One trade, net of every charge
// ---------------------------------------------------------------------------

/// What every net figure says it is.
pub const NET_LABEL: &str =
    "net after charges, precise NSE/SEBI method (every charge of the trade, both legs)";

/// Why [`NetTrade::of_days`] refuses two days priced under different rate
/// sets.
pub const DIFFERENT_RATE_SETS: &str = "the two days were priced under different rate sets";

/// Why [`NetTrade::of_days`] refuses a closing day that is not after the
/// opening day.
pub const NOT_IN_ORDER: &str = "the closing day is not after the opening day (a trade opened \
     and closed on one day is one client-security-day: NetTrade::of_day)";

/// One trade's result after **the entire precise charge stack** of its days:
/// the gross (sold value less bought value), every charge, and the net.
///
/// A trade is a win only if its net is above zero ([`Self::is_win`]); the
/// gross is reported beside it and never decides anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetTrade {
    gross: Paisa,
    charges: Paisa,
    net: Paisa,
    rate_set: RateSelector,
}

impl NetTrade {
    /// A round trip opened and closed on one client-security-day — an
    /// intraday trade, long or short.
    ///
    /// # Errors
    ///
    /// * [`ChargeError::OneSided`] for a day that does not close itself: a
    ///   side with no units, unequal sides, or an exercise (the exercised
    ///   quantity is not recorded, so its payoff cannot be netted here).
    /// * [`ChargeError::Cost`] carrying an overflow.
    ///
    /// # Examples
    ///
    /// ```
    /// use brutex_core::instrument::Exchange;
    /// use brutex_core::price::Paisa;
    /// use costs::day::TradeDay;
    /// use costs::rate::OrderSide;
    /// use costs::schedule::{Product, RateSelector, TradeKind};
    /// use costs::stack::{ClientSecurityDay, NetTrade, Order};
    ///
    /// let mut day = ClientSecurityDay::open(
    ///     TradeKind::Equity(Product::Intraday),
    ///     Exchange::Nse,
    ///     TradeDay::new(2026, 10, 15)?,
    ///     RateSelector::CurrentOnEveryDate,
    /// )?;
    /// day.add(Order::new(OrderSide::Buy, 10, Paisa::from_raw(100_00))?)?;
    /// day.add(Order::new(OrderSide::Sell, 10, Paisa::from_raw(100_05))?)?;
    /// let trade = NetTrade::of_day(&day.settle()?)?;
    /// // ₹0.50 gross is a loss once ₹0.78 of charges are paid.
    /// assert_eq!(trade.gross().raw(), 50);
    /// assert_eq!((trade.charges().raw(), trade.net().raw()), (78, -28));
    /// assert!(!trade.is_win());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn of_day(day: &DayCharges) -> Result<Self, ChargeError> {
        not_an_estimate(day, NET_LABEL)?;
        if day.buy_quantity != day.sell_quantity || day.buy_quantity == 0 || day.exercised.raw() > 0
        {
            return Err(ChargeError::OneSided {
                figure: "the net P&L of one day (it must buy and sell the same units, with no \
                         exercise)",
            });
        }
        Self::from_parts(
            day.sell_value,
            day.buy_value,
            day.total,
            day.basis.rate_set(),
        )
    }

    /// A round trip opened on one day and closed on another — a delivery,
    /// MTF or carried F&O position, long or short.
    ///
    /// # Errors
    ///
    /// * [`ChargeError::UnmatchedRoundTrip`] when the two days are not one
    ///   buy side and one sell side of the same units, kind and exchange.
    /// * [`ChargeError::MismatchedLegs`] when they are, but were priced under
    ///   different rate sets — one net would mix two rate tables and carry
    ///   neither's version — or the closing day is not after the opening
    ///   day: a reversed pair, or one day split in two, which is one
    ///   client-security-day and nets through [`Self::of_day`] (D-5131).
    /// * [`ChargeError::Cost`] carrying an overflow.
    pub fn of_days(opening: &DayCharges, closing: &DayCharges) -> Result<Self, ChargeError> {
        not_an_estimate(opening, NET_LABEL)?;
        not_an_estimate(closing, NET_LABEL)?;
        let long = opening.sell_quantity == 0 && closing.buy_quantity == 0;
        let short = opening.buy_quantity == 0 && closing.sell_quantity == 0;
        let (opened, closed) = if long {
            (opening.buy_quantity, closing.sell_quantity)
        } else {
            (opening.sell_quantity, closing.buy_quantity)
        };
        let matched = (long || short)
            && opened == closed
            && opening.kind == closing.kind
            && opening.exchange == closing.exchange
            && opening.exercised.raw() == 0
            && closing.exercised.raw() == 0;
        if !matched {
            return Err(ChargeError::UnmatchedRoundTrip { opened, closed });
        }
        if opening.basis.rate_set() != closing.basis.rate_set() {
            return Err(ChargeError::MismatchedLegs {
                why: DIFFERENT_RATE_SETS,
            });
        }
        if !opening.basis.day().before(closing.basis.day()) {
            return Err(ChargeError::MismatchedLegs { why: NOT_IN_ORDER });
        }
        let sold = add(
            opening.sell_value,
            closing.sell_value,
            "the trade's sold value",
        )?;
        let bought = add(
            opening.buy_value,
            closing.buy_value,
            "the trade's bought value",
        )?;
        let charges = add(opening.total, closing.total, "the trade's charges")?;
        Self::from_parts(sold, bought, charges, opening.basis.rate_set())
    }

    /// `sold − bought`, then less the charges, refused rather than wrapped.
    fn from_parts(
        sold: Paisa,
        bought: Paisa,
        charges: Paisa,
        rate_set: RateSelector,
    ) -> Result<Self, ChargeError> {
        let gross = sold
            .raw()
            .checked_sub(bought.raw())
            .ok_or(overflow("the trade's gross"))?;
        let net = gross
            .checked_sub(charges.raw())
            .ok_or(overflow("the trade's net"))?;
        Ok(Self {
            gross: Paisa::from_raw(gross),
            charges,
            net: Paisa::from_raw(net),
            rate_set,
        })
    }

    /// The rate set every day of the trade was priced under.
    #[must_use]
    pub const fn rate_set(&self) -> RateSelector {
        self.rate_set
    }

    /// The two version strings a run identity records beside this net: the
    /// rate set's ([`RateSelector::version`]) and the precise method's
    /// ([`Mode::version`]) — a net is only ever precise (D-5124, D-5131).
    ///
    /// # Examples
    ///
    /// ```
    /// use brutex_core::instrument::Exchange;
    /// use brutex_core::price::Paisa;
    /// use costs::day::TradeDay;
    /// use costs::rate::OrderSide;
    /// use costs::schedule::{Product, RateSelector, TradeKind};
    /// use costs::stack::{ClientSecurityDay, NetTrade, Order};
    ///
    /// let mut day = ClientSecurityDay::open(
    ///     TradeKind::Equity(Product::Intraday),
    ///     Exchange::Nse,
    ///     TradeDay::new(2026, 10, 15)?,
    ///     RateSelector::CurrentOnEveryDate,
    /// )?;
    /// day.add(Order::new(OrderSide::Buy, 50, Paisa::from_raw(1_520_00))?)?;
    /// day.add(Order::new(OrderSide::Sell, 50, Paisa::from_raw(1_530_00))?)?;
    /// let trade = NetTrade::of_day(&day.settle()?)?;
    /// assert_eq!(trade.rate_set(), RateSelector::CurrentOnEveryDate);
    /// assert_eq!(trade.versions(), ("todays-rates/2026-10-16/r1", "precise/r2"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn versions(&self) -> (&'static str, &'static str) {
        (self.rate_set.version(), Mode::Precise.version())
    }

    /// Sold value less bought value, before any charge. A labelled extra,
    /// never a decision.
    #[must_use]
    pub const fn gross(&self) -> Paisa {
        self.gross
    }
    /// Every precise charge of every day of the trade.
    #[must_use]
    pub const fn charges(&self) -> Paisa {
        self.charges
    }
    /// The gross less every charge.
    #[must_use]
    pub const fn net(&self) -> Paisa {
        self.net
    }
    /// Whether the trade is a win: its **net** is above zero. A trade whose
    /// charges eat its gross exactly is not a win.
    #[must_use]
    pub const fn is_win(&self) -> bool {
        self.net.raw() > 0
    }
}

impl fmt::Display for NetTrade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{NET_LABEL}: {} ({}; gross {}, charges {})",
            Rupees(self.net),
            if self.is_win() { "win" } else { "not a win" },
            Rupees(self.gross),
            Rupees(self.charges)
        )
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
    use crate::schedule::{BSE_EQUITY_EXCHANGE, CURRENT_AS_OF, FA73061_EFFECTIVE};

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
        ClientSecurityDay::open(
            kind,
            exchange,
            day(2026, 10, 15),
            RateSelector::CurrentOnEveryDate,
        )
        .expect("today's rates price every day")
    }

    fn priced(kind: TradeKind, exchange: Exchange, orders: &[Order]) -> DayCharges {
        let mut csd = open(kind, exchange);
        for &order in orders {
            csd.add(order).expect("in range");
        }
        csd.settle().expect("a settled day")
    }

    /// A day priced on `on` under `selector`: the closing leg of a carried
    /// trade, a day after [`priced`]'s.
    fn priced_on(
        kind: TradeKind,
        exchange: Exchange,
        orders: &[Order],
        on: TradeDay,
        selector: RateSelector,
    ) -> DayCharges {
        let mut csd = ClientSecurityDay::open(kind, exchange, on, selector).expect("priced");
        for &order in orders {
            csd.add(order).expect("in range");
        }
        csd.settle().expect("a settled day")
    }

    /// The day after [`priced`]'s, at today's rates.
    fn next(kind: TradeKind, exchange: Exchange, orders: &[Order]) -> DayCharges {
        priced_on(kind, exchange, orders, day(2026, 10, 16), TODAY)
    }

    const TODAY: RateSelector = RateSelector::CurrentOnEveryDate;
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
        // Delivery STT names NSE/FATAX/73524 (sources row S-05): no UNVERIFIED rate.
        assert_eq!(
            bought.unverified_sources().collect::<Vec<_>>(),
            Vec::<&str>::new()
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
            Vec::<&str>::new(),
            "futures STT names NSE/FATAX/73524 (sources row S-02)"
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
        exercised
            .exercise(rs(9_750_00), Settlement::Cash)
            .expect("an option");
        let charges = exercised.settle().expect("settled");
        assert_eq!(charges.stt_exercise(), rs(15_00));
        assert_eq!(charges.exercised(), rs(9_750_00));
        assert_eq!(charges.rates_used()[5], Some(crate::schedule::STT_EXERCISE));
        // The buy and the exercise each pay Dhan's ₹20 (sources rows B-04,
        // B-05): ₹40 of brokerage on one executed order and one exercise,
        // and GST on it: 18% of ₹40 + ₹2.31 + ₹0.01 = ₹7.6176 -> ₹7.62.
        assert_eq!((charges.orders(), charges.exercises()), (1, 1));
        // Brokerage, exchange, SEBI, trade STT, stamp, GST and the total:
        // ₹40 + ₹2.31 + ₹0.01 + ₹7.62 + the exercise's ₹15 = ₹64.94.
        assert_eq!(lines(&charges), [40_00, 2_31, 1, 0, 0, 7_62, 64_94]);
        // An exercise alone is a day: no order, still a tax, and still
        // Dhan's ₹20 and its GST — ₹15 + ₹20 + ₹3.60 (D-5132).
        let mut only = open(TradeKind::Options, Exchange::Nse);
        only.exercise(rs(9_750_00), Settlement::Cash)
            .expect("an option");
        let alone = only.settle().expect("an exercise is something to price");
        assert_eq!((alone.orders(), alone.exercises()), (0, 1));
        assert_eq!(
            (alone.brokerage(), alone.gst(), alone.stt_exercise()),
            (rs(20_00), rs(3_60), rs(15_00))
        );
        assert_eq!(alone.total(), rs(38_60));
        // Two exercises on one day are two of B-05's ₹20.
        only.exercise(rs(1_00), Settlement::Cash)
            .expect("an option");
        let twice = only.settle().expect("settled");
        assert_eq!((twice.exercises(), twice.brokerage()), (2, rs(40_00)));
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
                opt.exercise(rs(value), Settlement::Cash),
                Err(ChargeError::Cost(CostError::NotPositive {
                    quantity: "exercise intrinsic value",
                    value
                }))
            );
        }
        for kind in [INTRADAY, DELIVERY, MTF, TradeKind::Futures] {
            assert_eq!(
                open(kind, Exchange::Nse).exercise(rs(1_00), Settlement::Cash),
                Err(ChargeError::ExerciseNotApplicable {
                    kind: kind.as_str()
                })
            );
        }
    }

    #[test]
    fn a_stock_option_settled_by_physical_delivery_is_refused_by_name_and_changes_nothing() {
        // D-5130: a cash-settled exercise is priced (STT on the intrinsic
        // value, sources row S-04); a stock option settled by delivery is not
        // (Dhan's 0.10% of the contract value, row B-06, has no read
        // effective date; the delivery leg's STT, row S-06, was never read),
        // so it is refused by name at every intrinsic value, on both
        // exchanges, and the day is left exactly as it was.
        for exchange in [Exchange::Nse, Exchange::Bse] {
            let mut opt = open(TradeKind::Options, exchange);
            opt.add(buy(500, 24_50)).expect("a buy");
            let before = opt;
            for value in [1, 9_750_00, i64::MAX, 0, -1] {
                assert_eq!(
                    opt.exercise(rs(value), Settlement::PhysicalDelivery),
                    Err(PHYSICAL_DELIVERY_REFUSAL),
                    "{exchange:?} {value}"
                );
                assert_eq!(opt, before, "a refused delivery changes nothing");
            }
            // The cash-settled exercise of the same option is priced.
            let mut cash = before;
            cash.exercise(rs(9_750_00), Settlement::Cash)
                .expect("an index option");
            assert_eq!(cash.totals.exercised, rs(9_750_00));
            assert_eq!(opt.totals.exercised, Paisa::ZERO);
        }
        // A non-option is refused as a non-option first, whatever settlement
        // is named.
        assert_eq!(
            open(TradeKind::Futures, Exchange::Nse)
                .exercise(rs(1_00), Settlement::PhysicalDelivery),
            Err(ChargeError::ExerciseNotApplicable { kind: "futures" })
        );
        // The refusal says what and why in its own words, naming both rows.
        let words = PHYSICAL_DELIVERY_REFUSAL.to_string();
        for needle in ["physical delivery", "B-06", "S-06", "not priced"] {
            assert!(words.contains(needle), "{needle}: {words}");
        }
        assert!(matches!(
            PHYSICAL_DELIVERY_REFUSAL,
            ChargeError::OutOfScope { .. }
        ));
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

    /// An options day already at one edge, and the words its next exercise
    /// must be refused with.
    type ExerciseAtTheEdge = (fn(&mut ClientSecurityDay), &'static str);

    /// A client-day already at one of its edges, and the refusal it must name.
    type ClientEdge = (fn(&mut ClientDay), &'static str);

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
        let cases: [AtTheEdge; 8] = [
            (
                |d| d.totals.orders = i64::MAX,
                buy(1, 1),
                "the day's order count",
            ),
            (
                |d| d.totals.buy_quantity = i64::MAX,
                buy(1, 1),
                "the day's bought quantity",
            ),
            (
                |d| d.totals.buy_value = rs(i64::MAX),
                buy(1, 1),
                "the day's buy value",
            ),
            (
                |d| d.totals.sell_quantity = i64::MAX,
                sell(1, 1),
                "the day's sold quantity",
            ),
            (
                |d| d.totals.sell_value = rs(i64::MAX),
                sell(1, 1),
                "the day's sell value",
            ),
            (
                |d| d.totals.brokerage = rs(i64::MAX),
                buy(1, 100_00),
                "the day's brokerage",
            ),
            (
                |d| d.totals.square_off_fees = rs(i64::MAX),
                buy(1, 1).squared_off_by_broker(),
                "the day's square-off fees",
            ),
            (
                |d| d.totals.brokerage_exact = Exact(i128::MAX),
                buy(1, 100_00),
                "the day's unrounded brokerage",
            ),
        ];
        for (set, order, operation) in cases {
            let mut csd = edge(set);
            let before = csd;
            assert_eq!(csd.add(order).err(), named(operation));
            assert_eq!(csd, before, "{operation}: a refused order changes nothing");
        }
        // An exercise moves four totals, all or nothing (D-5132).
        let exercise_cases: [ExerciseAtTheEdge; 4] = [
            (
                |d| d.totals.exercises = i64::MAX,
                "the day's exercise count",
            ),
            (
                |d| d.totals.exercised = rs(i64::MAX),
                "the day's exercised value",
            ),
            (|d| d.totals.brokerage = rs(i64::MAX), "the day's brokerage"),
            (
                |d| d.totals.brokerage_exact = Exact(i128::MAX),
                "the day's unrounded brokerage",
            ),
        ];
        for (set, operation) in exercise_cases {
            let mut opt = open(TradeKind::Options, Exchange::Nse);
            set(&mut opt);
            let before = opt;
            assert_eq!(
                opt.exercise(rs(1), Settlement::Cash).err(),
                named(operation)
            );
            assert_eq!(
                opt, before,
                "{operation}: a refused exercise changes nothing"
            );
        }

        // Settlement: the turnover, the GST base and the total.
        let mut wide = open(TradeKind::Futures, Exchange::Nse);
        wide.totals.orders = 2;
        wide.totals.buy_value = rs(i64::MAX);
        wide.totals.sell_value = rs(1);
        assert_eq!(wide.settle().err(), named("the day's turnover"));
        let mut base = open(TradeKind::Futures, Exchange::Nse);
        base.totals.orders = 1;
        // ₹10 lakh of turnover: an exchange charge of ₹18.30 to add to it.
        base.totals.buy_value = rs(1_000_000_00);
        base.totals.brokerage = rs(i64::MAX);
        assert_eq!(base.settle().err(), named("the GST base"));
        let mut total = open(TradeKind::Futures, Exchange::Nse);
        total.totals.orders = 1;
        total.totals.buy_value = rs(1_00);
        total.totals.square_off_fees = rs(i64::MAX - 1);
        assert_eq!(total.settle().err(), named("the day's fees"));
        // The unrounded lines, summed for the rounding adjustment.
        let mut exact = open(INTRADAY, Exchange::Nse);
        exact.add(buy(1, 100_00)).expect("in range");
        exact.add(sell(1, 100_00)).expect("in range");
        exact.totals.brokerage_exact = Exact(i128::MAX);
        assert_eq!(exact.settle().err(), named("the day's unrounded lines"));
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
            RateSelector::CurrentOnEveryDate,
        )
        .expect("priced");
        let from_card = ClientSecurityDay::with_card(card);
        assert_eq!(from_card.card(), &card);
        assert_eq!(
            ClientSecurityDay::open(
                INTRADAY,
                Exchange::Nse,
                CURRENT_AS_OF,
                RateSelector::CurrentOnEveryDate
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

    /// The rows a dated NSE options day's exchange line is made of: the
    /// regime's exchange row on `on`, what the card adds to it, and the
    /// line's source — read from the rate the day was actually priced at.
    fn exchange_rows(charges: &DayCharges, on: TradeDay) -> (i64, i64, &'static str) {
        let line = charges.rates_used()[0].expect("the exchange line's rate");
        assert_eq!(line.rate().denominator(), crate::rate::RATE_SCALE, "{on}");
        let row = crate::regime::exchange_charge_rate(Exchange::Nse, on)
            .expect("a verified row")
            .get();
        (row, line.rate().numerator() - row, line.source())
    }

    #[test]
    fn a_dated_option_exchange_line_is_fa64232_plus_ipft_on_2026_02_27_and_fa73061_alone_on_2026_03_02()
     {
        // The same orders on the last session under FA64232 and the first
        // under FA73061 (2026-03-01 is a Sunday): 65 bought at ₹100 and sold at
        // ₹120, ₹14,300 of premium. The totals agree on both sides of the date
        // and always did; what this pins is which rows make them (D-6741).
        let orders = [buy(65, 100_00), sell(65, 120_00)];
        let (last, first) = (day(2026, 2, 27), day(2026, 3, 2));
        let dated = |on| {
            priced_on(
                TradeKind::Options,
                Exchange::Nse,
                &orders,
                on,
                RateSelector::Dated,
            )
        };
        let (before, after) = (dated(last), dated(first));
        let (row, ipft, source) = exchange_rows(&before, last);
        assert_eq!((row, ipft), (3_503, 50), "FA64232 plus the FA56129 IPFT");
        assert!(
            source.contains("FA64232") && source.contains("FA56129"),
            "{source}"
        );
        let (row, ipft, source) = exchange_rows(&after, first);
        assert_eq!((row, ipft), (3_553, 0), "FA73061's folded row alone");
        assert!(
            source.contains("NSE/FA/73061") && !source.contains("FA64232"),
            "{source}"
        );
        // What the card adds is the regime's IPFT on the day, and nothing else.
        for (on, ipft) in [(last, 50), (first, 0)] {
            assert_eq!(
                crate::regime::ipft_rate(Exchange::Nse, on).map(crate::rate::BpsX100::get),
                Ok(ipft),
                "{on}"
            );
        }
        // ₹3,553 per crore on both days — today's NSE_OPTIONS_EXCHANGE — so
        // ₹14,300 × 0.03553% = ₹5.08079 -> ₹5.08 under either rate set.
        let today = priced_on(TradeKind::Options, Exchange::Nse, &orders, first, TODAY);
        let fa73061 = crate::schedule::NSE_OPTIONS_EXCHANGE.rate();
        for (charges, on) in [(&before, last), (&after, first)] {
            assert_eq!(charges.exchange_charge(), rs(5_08), "{on}");
            let rate = charges.rates_used()[0].expect("the exchange line").rate();
            assert_eq!(
                i128::from(rate.numerator()) * i128::from(fa73061.denominator()),
                i128::from(fa73061.numerator()) * i128::from(rate.denominator()),
                "{on}"
            );
        }
        assert_eq!(today.exchange_charge(), rs(5_08));
        // Every other line is priced at the same rows on both days, so the two
        // days' lines and totals are equal. STT is the 0.10% row on both
        // (₹7,800 × 0.10% = ₹7.80 -> ₹8); today's table is 0.15% (₹11.70 -> ₹12).
        assert_eq!(before.lines(), after.lines());
        assert_eq!(before.total(), after.total());
        assert_eq!(before.rates_used()[1..], after.rates_used()[1..]);
        assert_eq!(
            (before.stt(), after.stt(), today.stt()),
            (rs(8_00), rs(8_00), rs(12_00))
        );
        assert_eq!(after.basis().to_string(), "rates in force on 2026-03-02");
        // Neither NSE line is flagged: both sources are cited rows.
        assert_eq!(before.unverified_sources().count(), 0);
        assert_eq!(after.unverified_sources().count(), 0);
    }

    #[test]
    fn a_position_carried_across_fa73061_prices_each_day_at_its_own_row() {
        // Bought on the last session under FA64232, sold on the first under
        // FA73061: two client-security-days, each priced on its own day's card.
        let (last, first) = (day(2026, 2, 27), day(2026, 3, 2));
        let options = TradeKind::Options;
        let bought = priced_on(
            options,
            Exchange::Nse,
            &[buy(65, 100_00)],
            last,
            RateSelector::Dated,
        );
        let sold = priced_on(
            options,
            Exchange::Nse,
            &[sell(65, 120_00)],
            first,
            RateSelector::Dated,
        );
        // ₹6,500 × 0.03553% = ₹2.30945 -> ₹2.31; ₹7,800 × 0.03553% = ₹2.77134
        // -> ₹2.77. Here the two days' lines add to the one day's ₹5.08.
        assert_eq!(
            (bought.exchange_charge(), sold.exchange_charge()),
            (rs(2_31), rs(2_77))
        );
        assert_eq!(exchange_rows(&bought, last).0, 3_503);
        assert_eq!(exchange_rows(&sold, first).0, 3_553);
        assert!(exchange_rows(&bought, last).2.contains("FA64232"));
        let sold_source = exchange_rows(&sold, first).2;
        assert!(sold_source.contains("NSE/FA/73061") && !sold_source.contains("FA64232"));
        // STT is the sell leg's alone, at its own day's 0.10% row.
        assert_eq!((bought.stt(), sold.stt()), (rs(0), rs(8_00)));
        assert_eq!(bought.basis().to_string(), "rates in force on 2026-02-27");
        assert_eq!(sold.basis().to_string(), "rates in force on 2026-03-02");
    }

    #[test]
    fn the_smallest_premium_and_a_crore_price_alike_on_both_sides_of_fa73061() {
        // One paisa of premium each way costs no exchange charge on either side
        // of the date; ₹1 crore of premium costs FA73061's ₹3,553 exactly on
        // both, under the dated rows (3,503 + 50, then 3,553 + 0) and today's
        // table alike — the sum is taken before the one rounding, so the split
        // cannot show in the line.
        for on in [day(2026, 2, 27), FA73061_EFFECTIVE, day(2026, 3, 2)] {
            for selector in [RateSelector::Dated, TODAY] {
                let tiny = priced_on(
                    TradeKind::Options,
                    Exchange::Nse,
                    &[buy(1, 1), sell(1, 1)],
                    on,
                    selector,
                );
                assert_eq!(tiny.exchange_charge(), rs(0), "{on} {selector:?}");
                let crore = priced_on(
                    TradeKind::Options,
                    Exchange::Nse,
                    &[buy(100, 50_000_00), sell(100, 50_000_00)],
                    on,
                    selector,
                );
                assert_eq!(crore.exchange_charge(), rs(3_553_00), "{on} {selector:?}");
            }
        }
    }

    #[test]
    fn fourteen_threads_open_the_dated_option_card_around_fa73061_and_get_one_card() {
        // Out of the box: the card is a pure function of its inputs, so every
        // thread asking about the days around the date at once gets the card,
        // source line included, that one thread gets alone.
        let days: Vec<TradeDay> = (0..10)
            .map(|offset| day(2026, 2, 25).plus_days(offset).expect("in range"))
            .collect();
        let open = |on| {
            ClientSecurityDay::open(TradeKind::Options, Exchange::Nse, on, RateSelector::Dated)
                .map(|csd| *csd.card())
        };
        let alone: Vec<_> = days.iter().map(|&on| open(on)).collect();
        let answers: Vec<Vec<_>> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..14)
                .map(|_| scope.spawn(|| days.iter().map(|&on| open(on)).collect::<Vec<_>>()))
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("no thread panics"))
                .collect()
        });
        assert_eq!(answers.len(), 14);
        for answer in answers {
            assert_eq!(answer, alone);
        }
        // The window crosses the date: both sources are in it.
        let sources: Vec<&str> = alone
            .iter()
            .map(|card| card.as_ref().expect("priced").exchange_charge().source())
            .collect();
        assert!(sources[..4].iter().all(|source| source.contains("FA64232")));
        assert!(
            sources[4..]
                .iter()
                .all(|source| !source.contains("FA64232"))
        );
        assert_eq!(days[4], FA73061_EFFECTIVE);
    }

    #[test]
    fn the_intraday_tax_is_on_the_vwap_rounded_to_the_paisa_as_nse_cmtr_5588_states() {
        // NSE/CMTR/5588: VWAP = (BVAL + SVAL) / (BQTY + SQTY) "rounded off to
        // two decimals", SNDELVAL = min(BQTY, SQTY) x VWAP. 100000 @ 50.01 ->
        // 50.02: the VWAP is 50.015, which rounds to 50.02, so the value is
        // ₹50,02,000 and the tax ₹1,250.50 -> ₹1,251. Half the turnover
        // unrounded (₹50,01,500) gives ₹1,250.375 -> ₹1,250.
        let day = priced(
            INTRADAY,
            Exchange::Nse,
            &[buy(100_000, 50_01), sell(100_000, 50_02)],
        );
        assert_eq!(day.stt(), rs(1_251_00));
    }

    #[test]
    fn the_intraday_tax_and_stamp_round_to_the_paisa_before_the_rupee() {
        // NSE/CMTR/5588 rounds each security's STT to two decimals and the
        // client's total to the rupee; NCL/CMPT/43115 does the same for stamp
        // duty. 2 @ 38,990 -> 38,998: VWAP 38,994, value ₹77,988, STT
        // 19.497 -> 19.50 -> ₹20 (rounding straight to the rupee gives ₹19).
        let stt = priced(
            INTRADAY,
            Exchange::Nse,
            &[buy(2, 38_990_00), sell(2, 38_998_00)],
        );
        assert_eq!(stt.stt(), rs(20_00));
        // 1 @ 16,650 -> 16,700: stamp 0.003% of ₹16,650 = 0.4995 -> 0.50 -> ₹1.
        let stamp = priced(
            INTRADAY,
            Exchange::Nse,
            &[buy(1, 16_650_00), sell(1, 16_700_00)],
        );
        assert_eq!(stamp.stamp(), rs(1_00));
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
        // A BSE day still uses an UNVERIFIED rate (BSE cash, Rs 375 per crore).
        let bse = priced(
            INTRADAY,
            Exchange::Bse,
            &[buy(50, 1_520_00), sell(50, 1_530_00)],
        );
        assert_eq!(writes_needed(&v1), 145);
        assert_eq!(writes_needed(&bse), 146);
        assert!(
            writes_needed(&bse) < 256,
            "the budget is not itself the answer"
        );
        assert!(
            writes_needed(&bse) > writes_needed(&v1),
            "the UNVERIFIED line is written too"
        );
    }

    #[test]
    fn a_client_day_rounds_stt_and_stamp_once_across_its_securities() {
        // Two securities, 3 @ ₹400 and 3 @ ₹404 bought and sold at the same
        // price: STT ₹0.30 each to the paisa, nothing alone, ₹1 together.
        let a = priced(INTRADAY, Exchange::Nse, &[buy(3, 400_00), sell(3, 400_00)]);
        let b = priced(INTRADAY, Exchange::Nse, &[buy(3, 404_00), sell(3, 404_00)]);
        assert_eq!((a.stt_paisa(), b.stt_paisa()), (rs(30), rs(30)));
        assert_eq!((a.stt(), b.stt()), (Paisa::ZERO, Paisa::ZERO));
        // Stamp duty: 0.003% of ₹1,200 and ₹1,212 is 3.6 and 3.636 paisa.
        assert_eq!((a.stamp_paisa(), b.stamp_paisa()), (rs(4), rs(4)));
        let mut client = ClientDay::new();
        assert_eq!(client, ClientDay::default());
        assert_eq!(client.settle(), Err(ChargeError::EmptyDay));
        client.add(&a).expect("one segment");
        client.add(&b).expect("one segment");
        let two = client.settle().expect("two securities");
        assert_eq!(two.securities(), 2);
        assert_eq!(two.stt(), rs(1_00));
        assert_eq!(two.stamp(), Paisa::ZERO); // 8 paisa: below half a rupee
        assert_eq!(two.fees(), rs(a.fees().raw() + b.fees().raw()));
        assert_eq!(two.total(), rs(two.fees().raw() + 1_00));
        assert_eq!(two.priced_alone(), rs(a.total().raw() + b.total().raw()));
        assert_eq!(two.total().raw() - two.priced_alone().raw(), 1_00);
        assert_eq!(two.basis(), a.basis());
        assert_eq!(two.exchange(), Exchange::Nse);
        assert_eq!(two.segment(), Segment::Cash);
        // One security alone: the client-day is exactly the day priced alone,
        // on every vector, and fees + STT + stamp is the total.
        for orders in [
            [buy(50, 1_520_00), sell(50, 1_530_00)],
            [buy(100_000, 50_00), sell(100_000, 50_50)],
            [sell(500, 2_000_00), buy(500, 1_980_00)],
        ] {
            let alone = priced(INTRADAY, Exchange::Nse, &orders);
            let mut one = ClientDay::new();
            one.add(&alone).expect("one");
            let settled = one.settle().expect("one");
            assert_eq!(settled.total(), alone.total());
            assert_eq!(settled.stt(), alone.stt());
            assert_eq!(settled.stamp(), alone.stamp());
            assert_eq!(
                alone.fees().raw() + alone.stt().raw() + alone.stamp().raw(),
                alone.total().raw()
            );
        }
        // Delivery and intraday are one cash-market client-day.
        let held = priced(DELIVERY, Exchange::Nse, &[buy(50, 1_520_00)]);
        let mut with_delivery = client;
        assert!(with_delivery.add(&held).is_ok());
    }

    #[test]
    fn a_client_day_refuses_another_day_venue_or_segment_and_stays_as_it_was() {
        let v1 = priced(
            INTRADAY,
            Exchange::Nse,
            &[buy(50, 1_520_00), sell(50, 1_530_00)],
        );
        let mut client = ClientDay::new();
        client.add(&v1).expect("first");
        let before = client;
        let other_day = {
            let mut csd = ClientSecurityDay::open(
                INTRADAY,
                Exchange::Nse,
                day(2026, 10, 14),
                RateSelector::CurrentOnEveryDate,
            )
            .expect("priced");
            csd.add(buy(1, 100_00)).expect("in range");
            csd.add(sell(1, 100_00)).expect("in range");
            csd.settle().expect("settled")
        };
        let bse = priced(INTRADAY, Exchange::Bse, &[buy(1, 100_00), sell(1, 100_00)]);
        let fno = priced(
            TradeKind::Options,
            Exchange::Nse,
            &[buy(65, 100_00), sell(65, 120_00)],
        );
        for (security, differs) in [
            (other_day, "trade day or rate set"),
            (bse, "exchange"),
            (fno, "segment"),
        ] {
            assert_eq!(
                client.add(&security),
                Err(ChargeError::MixedClientDay { differs })
            );
            assert_eq!(
                client, before,
                "{differs}: a refused security changes nothing"
            );
        }
        // An exercise's STT is rounded with the trades' on the client's day.
        let mut exercised = open(TradeKind::Options, Exchange::Nse);
        exercised.add(sell(65, 120_00)).expect("in range");
        exercised
            .exercise(rs(9_750_00), Settlement::Cash)
            .expect("an option");
        let ex = exercised.settle().expect("settled");
        // Trade STT 0.15% of ₹7,800 = ₹11.70; exercise 0.15% of ₹9,750 =
        // ₹14.625 -> 14.63. Alone: ₹12 for the trades, and the exercise adds
        // what the day's ₹26.33 rounds to beyond them: ₹26 - ₹12 = ₹14.
        assert_eq!(
            (ex.stt_paisa(), ex.stt_exercise_paisa()),
            (rs(11_70), rs(14_63))
        );
        assert_eq!((ex.stt(), ex.stt_exercise()), (rs(12_00), rs(14_00)));
        let mut derivatives = ClientDay::new();
        derivatives.add(&ex).expect("first");
        derivatives.add(&fno).expect("same segment");
        // ₹26.33 + the 25300 CE round trip's ₹11.70 = ₹38.03, rounded once.
        let settled = derivatives.settle().expect("settled");
        assert_eq!(settled.stt(), rs(38_00)); // 11.70 + 14.63 + 11.70 = 38.03
        assert_eq!(settled.segment(), Segment::Derivatives);
    }

    #[test]
    fn a_trade_is_a_win_only_if_its_net_after_every_charge_is_above_zero() {
        // V1, intraday long: ₹500 gross less the entire precise stack, ₹73.90.
        let v1 = priced(
            INTRADAY,
            Exchange::Nse,
            &[buy(50, 1_520_00), sell(50, 1_530_00)],
        );
        let trade = NetTrade::of_day(&v1).expect("a round trip");
        assert_eq!(
            (trade.gross(), trade.charges(), trade.net()),
            (rs(500_00), rs(73_90), rs(426_10))
        );
        assert!(trade.is_win());
        assert_eq!(
            trade.to_string(),
            "net after charges, precise NSE/SEBI method (every charge of the trade, both legs): \
             ₹426.10 (win; gross ₹500.00, charges ₹73.90)"
        );
        // V3: a loss before charges stays a loss, and grows by them.
        let v3 = priced(INTRADAY, Exchange::Nse, &[buy(10, 100_00), sell(10, 99_00)]);
        let loss = NetTrade::of_day(&v3).expect("a round trip");
        assert_eq!((loss.gross(), loss.net()), (rs(-10_00), rs(-10_78)));
        assert!(!loss.is_win());
        assert!(loss.to_string().contains("-₹10.78 (not a win;"));
        // A gross win the charges eat is not a win: 10 @ 100.00 -> 100.05.
        let thin = priced(
            INTRADAY,
            Exchange::Nse,
            &[buy(10, 100_00), sell(10, 100_05)],
        );
        let thin = NetTrade::of_day(&thin).expect("a round trip");
        assert_eq!(
            (thin.gross(), thin.charges(), thin.net()),
            (rs(50), rs(78), rs(-28))
        );
        assert!(thin.gross().raw() > 0 && !thin.is_win());
        // Exactly break-even is not a win: the net must be ABOVE zero.
        let even = NetTrade::from_parts(rs(10_00), rs(5_00), rs(5_00), TODAY).expect("in range");
        assert_eq!(even.net(), Paisa::ZERO);
        assert!(!even.is_win());
        assert!(
            NetTrade::from_parts(rs(10_00), rs(5_00), rs(4_99), TODAY)
                .expect("in range")
                .is_win()
        );
        // V5, intraday short: sold first, bought back lower.
        let v5 = priced(
            INTRADAY,
            Exchange::Nse,
            &[sell(500, 2_000_00), buy(500, 1_980_00)],
        );
        assert_eq!(NetTrade::of_day(&v5).expect("short").net(), rs(9_599_37));
        // D1, delivery: bought one day, sold another; both days' charges.
        let bought = priced(DELIVERY, Exchange::Nse, &[buy(50, 1_520_00)]);
        let sold = next(DELIVERY, Exchange::Nse, &[sell(50, 1_530_00)]);
        let d1 = NetTrade::of_days(&bought, &sold).expect("a delivery round trip");
        assert_eq!(
            (d1.gross(), d1.charges(), d1.net()),
            (rs(500_00), rs(184_46), rs(315_54))
        );
        // A carried futures short: sold one day, covered another.
        let opened = priced(TradeKind::Futures, Exchange::Nse, &[sell(65, 25_250_00)]);
        let covered = next(TradeKind::Futures, Exchange::Nse, &[buy(65, 25_200_00)]);
        let short = NetTrade::of_days(&opened, &covered).expect("a futures round trip");
        assert_eq!(short.gross(), rs(3_250_00));
        assert_eq!(
            short.charges(),
            rs(opened.total().raw() + covered.total().raw())
        );
        assert!(short.is_win());
    }

    #[test]
    fn a_net_figure_is_refused_wherever_the_trade_does_not_close() {
        let one_day = ChargeError::OneSided {
            figure: "the net P&L of one day (it must buy and sell the same units, with no \
                     exercise)",
        };
        let bought = priced(DELIVERY, Exchange::Nse, &[buy(50, 1_520_00)]);
        let sold = next(DELIVERY, Exchange::Nse, &[sell(50, 1_530_00)]);
        let partly = priced(DELIVERY, Exchange::Nse, &[sell(49, 1_530_00)]);
        assert_eq!(NetTrade::of_day(&bought), Err(one_day));
        assert_eq!(NetTrade::of_day(&sold), Err(one_day));
        let mut exercised = open(TradeKind::Options, Exchange::Nse);
        exercised.add(buy(65, 100_00)).expect("in range");
        exercised.add(sell(65, 120_00)).expect("in range");
        exercised
            .exercise(rs(9_750_00), Settlement::Cash)
            .expect("an option");
        let ex = exercised.settle().expect("settled");
        assert_eq!(NetTrade::of_day(&ex), Err(one_day));
        let mut alone = open(TradeKind::Options, Exchange::Nse);
        alone
            .exercise(rs(9_750_00), Settlement::Cash)
            .expect("an option");
        assert_eq!(
            NetTrade::of_day(&alone.settle().expect("settled")),
            Err(one_day)
        );
        // Two days that do not close each other.
        let unmatched = |opened, closed| Err(ChargeError::UnmatchedRoundTrip { opened, closed });
        assert_eq!(NetTrade::of_days(&bought, &partly), unmatched(50, 49));
        assert_eq!(NetTrade::of_days(&bought, &bought), unmatched(0, 50));
        assert_eq!(NetTrade::of_days(&sold, &sold), unmatched(50, 0));
        let mtf_sold = priced(MTF, Exchange::Nse, &[sell(50, 1_530_00)]);
        assert_eq!(NetTrade::of_days(&bought, &mtf_sold), unmatched(50, 50));
        let bse_sold = priced(DELIVERY, Exchange::Bse, &[sell(50, 1_530_00)]);
        assert_eq!(NetTrade::of_days(&bought, &bse_sold), unmatched(50, 50));
        let opt_open = priced(TradeKind::Options, Exchange::Nse, &[buy(65, 100_00)]);
        let mut close_ex = open(TradeKind::Options, Exchange::Nse);
        close_ex.add(sell(65, 120_00)).expect("in range");
        close_ex
            .exercise(rs(1_00), Settlement::Cash)
            .expect("an option");
        let close_ex = close_ex.settle().expect("settled");
        assert_eq!(NetTrade::of_days(&opt_open, &close_ex), unmatched(65, 65));
        assert_eq!(NetTrade::of_days(&close_ex, &opt_open), unmatched(65, 65));
        // A short opened by a sell is not closed by a day that buys AND
        // sells: that day is not one buy side, whatever its buy quantity.
        let short_open = priced(TradeKind::Futures, Exchange::Nse, &[sell(65, 25_200_00)]);
        let both_sides = priced(
            TradeKind::Futures,
            Exchange::Nse,
            &[buy(65, 25_150_00), sell(65, 25_160_00)],
        );
        assert_eq!(
            NetTrade::of_days(&short_open, &both_sides),
            unmatched(65, 65)
        );
        // A gross or a net past i64 is refused by name, never wrapped.
        let named = |operation| Err(ChargeError::Cost(CostError::Overflow { operation }));
        assert_eq!(
            NetTrade::from_parts(rs(i64::MIN), rs(1), rs(0), TODAY),
            named("the trade's gross")
        );
        assert_eq!(
            NetTrade::from_parts(rs(i64::MIN + 1), rs(1), rs(1), TODAY),
            named("the trade's net")
        );
        let mut huge = bought;
        huge.total = rs(i64::MAX);
        let mut huge_sold = sold;
        huge_sold.total = rs(1);
        assert_eq!(
            NetTrade::of_days(&huge, &huge_sold),
            named("the trade's charges")
        );
        huge_sold.sell_value = rs(i64::MAX);
        huge.sell_value = rs(1);
        huge.sell_quantity = 0;
        assert_eq!(
            NetTrade::of_days(&huge, &huge_sold),
            named("the trade's sold value")
        );
        let mut huge_bought = bought;
        huge_bought.buy_value = rs(i64::MAX);
        let mut extra = sold;
        extra.buy_value = rs(1);
        extra.buy_quantity = 0;
        assert_eq!(
            NetTrade::of_days(&huge_bought, &extra),
            named("the trade's bought value")
        );
    }

    #[test]
    fn a_net_across_two_rate_sets_or_out_of_order_days_is_refused_and_a_net_names_its_versions() {
        let mismatched = |why| Err(ChargeError::MismatchedLegs { why });
        let at = |kind, orders: &[Order], on, selector| {
            priced_on(kind, Exchange::Nse, orders, on, selector)
        };
        // A delivery trade bought on 15 Oct: sold on 16 Oct it nets; sold on
        // 14 Oct (reversed) or on 15 Oct itself (one day split in two) it is
        // refused by name.
        let bought = priced(DELIVERY, Exchange::Nse, &[buy(50, 1_520_00)]);
        let sold = next(DELIVERY, Exchange::Nse, &[sell(50, 1_530_00)]);
        let earlier = at(DELIVERY, &[sell(50, 1_530_00)], day(2026, 10, 14), TODAY);
        let same = priced(DELIVERY, Exchange::Nse, &[sell(50, 1_530_00)]);
        assert_eq!(
            NetTrade::of_days(&bought, &earlier),
            mismatched(NOT_IN_ORDER)
        );
        assert_eq!(NetTrade::of_days(&bought, &same), mismatched(NOT_IN_ORDER));
        let d1 = NetTrade::of_days(&bought, &sold).expect("in order");
        assert_eq!(d1.rate_set(), TODAY);
        assert_eq!(d1.versions(), ("todays-rates/2026-10-16/r1", "precise/r2"));
        // Options, the kind the dated set prices: one leg at the rates in
        // force, the other at today's, either way round, is refused.
        let (d14, d15) = (day(2026, 10, 14), day(2026, 10, 15));
        let dated = RateSelector::Dated;
        let open_dated = at(TradeKind::Options, &[buy(65, 100_00)], d14, dated);
        let open_today = at(TradeKind::Options, &[buy(65, 100_00)], d14, TODAY);
        let close_dated = at(TradeKind::Options, &[sell(65, 120_00)], d15, dated);
        let close_today = at(TradeKind::Options, &[sell(65, 120_00)], d15, TODAY);
        assert_eq!(
            NetTrade::of_days(&open_dated, &close_today),
            mismatched(DIFFERENT_RATE_SETS)
        );
        assert_eq!(
            NetTrade::of_days(&open_today, &close_dated),
            mismatched(DIFFERENT_RATE_SETS)
        );
        // One rate set, in order: netted, and the net names that set.
        let carried = NetTrade::of_days(&open_dated, &close_dated).expect("one rate set");
        assert_eq!(carried.rate_set(), dated);
        assert_eq!(carried.versions(), ("dated-regime/r2", "precise/r2"));
        assert_eq!(
            carried.charges(),
            rs(open_dated.total().raw() + close_dated.total().raw())
        );
        assert_eq!(
            NetTrade::of_days(&open_today, &close_today)
                .expect("one rate set")
                .rate_set(),
            TODAY
        );
        // A one-day net names its day's rate set.
        let intraday = at(
            TradeKind::Options,
            &[buy(65, 100_00), sell(65, 120_00)],
            d15,
            dated,
        );
        assert_eq!(
            NetTrade::of_day(&intraday)
                .expect("a round trip")
                .versions(),
            ("dated-regime/r2", "precise/r2")
        );
    }

    #[test]
    fn every_new_overflow_site_is_refused_by_name() {
        let named = |operation| Some(ChargeError::Cost(CostError::Overflow { operation }));
        // The non-delivery value cannot leave i64: the VWAP is at most
        // T / 2u + 1/2, so u x VWAP is at most T/2 + u/2 <= i64::MAX. The
        // widest edge lands exactly on it, and the refusal stays as a guard.
        assert_eq!(non_delivery_value(rs(i64::MAX), i64::MAX), Ok(rs(i64::MAX)));
        assert_eq!(
            non_delivery_value(rs(i64::MAX - 1), i64::MAX),
            Ok(Paisa::ZERO)
        );
        assert_eq!(non_delivery_value(rs(152_500_00), 50), Ok(rs(76_250_00)));
        // 3 units at a VWAP of 100.015 (an exact half paisa) value at 100.02.
        assert_eq!(non_delivery_value(rs(600_09), 3), Ok(rs(300_06)));
        assert_eq!(
            non_delivery_value(rs(1), 0).err(),
            Some(ChargeError::Cost(CostError::NotPositive {
                quantity: "rounding denominator",
                value: 0
            }))
        );
        // Rounding to the rupee cannot leave i64 either: i64::MAX ends in 07,
        // so the largest result is ...800.
        assert_eq!(to_rupee(rs(i64::MAX)), Ok(rs(9_223_372_036_854_775_800)));
        assert_eq!(to_rupee(rs(1_49)), Ok(rs(1_00)));
        assert_eq!(to_rupee(rs(1_50)), Ok(rs(2_00)));
        assert_eq!(to_rupee(Paisa::ZERO), Ok(Paisa::ZERO));
        // A client-day's running totals.
        let v1 = priced(
            INTRADAY,
            Exchange::Nse,
            &[buy(50, 1_520_00), sell(50, 1_530_00)],
        );
        let edges: [ClientEdge; 5] = [
            (|c| c.securities = i64::MAX, "the client-day's securities"),
            (|c| c.fees = rs(i64::MAX), "the client-day's fees"),
            (|c| c.stt_paisa = rs(i64::MAX), "the client-day's STT"),
            (
                |c| c.stamp_paisa = rs(i64::MAX),
                "the client-day's stamp duty",
            ),
            (
                |c| c.rounded_alone = rs(i64::MAX),
                "the client-day's totals priced alone",
            ),
        ];
        for (set, operation) in edges {
            let mut client = ClientDay::new();
            client.add(&v1).expect("first");
            set(&mut client);
            let before = client;
            assert_eq!(client.add(&v1).err(), named(operation));
            assert_eq!(client, before, "{operation}");
        }
        let mut wide = ClientDay::new();
        wide.add(&v1).expect("first");
        wide.fees = rs(i64::MAX - 10_00);
        assert_eq!(wide.settle().err(), named("the client-day's total"));
        // The widest exercise a day can hold still settles: its STT is 0.15%
        // of it, so the trade and exercise STT together stay inside i64.
        let mut opt = open(TradeKind::Options, Exchange::Nse);
        opt.add(sell(1, 100_00)).expect("in range");
        opt.totals.exercised = rs(i64::MAX);
        let widest = opt.settle().expect("inside i64");
        assert_eq!(widest.stt_paisa(), rs(15));
        assert_eq!(widest.stt_exercise_paisa(), rs(13_835_058_055_282_164));
        // The calculator day's fees: square-off fees near i64::MAX (a running
        // total no sequence of adds reaches, set here directly) and their GST
        // beside the page's lines leave i64, and the guard refuses by name
        // rather than wrapping; one fee less is still priced.
        let mut squared = open(INTRADAY, Exchange::Nse);
        squared.add(buy(1, 100_00)).expect("in range");
        squared.add(sell(1, 100_00)).expect("in range");
        squared.totals.square_off_fees = rs(i64::MAX - 1);
        let on = day(2026, 10, 15);
        assert_eq!(
            settle_dhan(INTRADAY, Exchange::Nse, on, &squared.totals).err(),
            named("the calculator day's fees")
        );
        squared.totals.square_off_fees = rs(20_00);
        let page = settle_dhan(INTRADAY, Exchange::Nse, on, &squared.totals).expect("in range");
        assert_eq!(page.square_off_fee(), rs(20_00));
        assert_eq!(page.lines()[9], ("broker square-off fee", rs(20_00)));
    }
}
