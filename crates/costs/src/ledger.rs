//! The per client-security-day accumulator a backtest drives: one
//! [`DayLedger`] per client, security and day, an O(1) [`DayLedger::add`]
//! per executed order and one [`DayLedger::close`] that prices the day in
//! either mode under an explicitly named rate set.
//!
//! # Why this exists beside [`crate::stack::ClientSecurityDay`]
//!
//! A backtest applies charges per trade, at several trade sizes, inside a
//! visitor. Two things make that more than "price each trade alone":
//!
//! * **STT and stamp duty are rounded once per client per security per day**
//!   (`NSE/CMTR/5588` section D, `NCL/CMPT/43115` Annexure 1), across every
//!   trade the mix took in that stock that day. Pricing each trade alone and
//!   summing rounds them once per trade, which no contract note does.
//! * **Brokerage is charged per executed order**, and an order whose fill
//!   spilled over several one-second bars is still ONE executed order.
//!
//! A [`DayLedger`] keeps the day's running totals and is priced once, when the
//! day closes, in the mode and under the rate set the caller names. The
//! arithmetic is the stack's own: [`Mode::Precise`] is exactly what
//! [`crate::stack::ClientSecurityDay::settle`] returns for the same orders,
//! and [`Mode::DhanCalculator`] is exactly what [`crate::dhan_calc::quote`]
//! returns for a one-buy, one-sell day — both proven on every golden case in
//! `crates/costs/tests/dhan_calculator.rs`.
//!
//! # One order filled in parts
//!
//! An [`ExecutedOrder`] is one order, however many fills it took: its parts
//! are joined with [`ExecutedOrder::with_part`] before it is added, and the
//! brokerage is charged on it once. That a part-filled order is ONE executed
//! order for **Dhan's** brokerage is **UNVERIFIED** until a Dhan contract note
//! shows a part-filled order (`docs/06-limits.md` §CHG-1, D-5127).
//!
//! # No `Default`, on purpose
//!
//! Neither [`DayLedger`] nor [`RateSelector`] has a `Default`: a ledger is
//! opened on a kind, a venue and a day, and a close names its rate set, so a
//! run's identity can carry [`RateSelector::version`] and [`Mode::version`]
//! and never an implicit choice. The compile-time check at the foot of this
//! file fails the build if either ever gains one.
//!
//! # Constant cost
//!
//! [`DayLedger::add`] is a fixed handful of checked additions and one
//! [`crate::schedule::dhan_brokerage`]; [`DayLedger::close`] is one
//! [`RateCard::resolve`] and a fixed sequence of exact-rate products, and so
//! is [`DayLedger::order_lines_unrounded`]. None walks the orders.
//! `crates/costs/benches/stack.rs` measures all three: C-CHG-07 for the add,
//! C-CHG-08 for the close and C-CHG-09 for one order's lines, in each mode.
//!
//! # One order's lines need its day
//!
//! Whether a delivery or MTF order pays delivery STT and stamp duty and a DP
//! share, or the non-delivery rates and no DP, depends on whether its day
//! squared off. [`DayLedger::order_lines_unrounded`] prices an order against
//! its own day's netting — the rule [`DayLedger::close`] nets by — and the
//! day-less [`order_lines_unrounded`] refuses a delivery or MTF order by name
//! (D-5131).

use brutex_core::instrument::Exchange;
use brutex_core::price::Paisa;

use crate::charge_error::ChargeError;
use crate::day::TradeDay;
use crate::error::CostError;
use crate::exact::{Exact, round_half_up, units_of};
use crate::rate::OrderSide;
use crate::schedule::{
    DHAN_DP_CHARGE, DHAN_SQUARE_OFF_FEE, GST, Mode, Product, RateCard, RateSelector, Settlement,
    Taxes, TradeKind, dhan_brokerage_units,
};
use crate::stack::{
    DayCharges, LINE_NAMES, Netting, Totals, equity_netting, settle_dhan, settle_precise,
};

/// One executed order: a side, the units it filled and their value in paisa
/// (price × units, or premium × units for an option), and whether the
/// broker's risk system placed it to square off a position open at the
/// intraday ceiling.
///
/// Every field is public so a caller can write the order it saw; a ledger
/// refuses a quantity or a value that is not positive when it is added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExecutedOrder {
    /// Buy or sell.
    pub side: OrderSide,
    /// Units filled, across every part of the order.
    pub qty: i64,
    /// The filled value in paisa, across every part of the order.
    pub value_paisa: Paisa,
    /// Whether the broker placed this order at the intraday ceiling. Only
    /// then is the square-off fee charged, as its own line.
    pub broker_square_off: bool,
}

impl ExecutedOrder {
    /// The same order with one more part filled: `qty` more units worth
    /// `value_paisa` more. A part is not an order — the brokerage is charged
    /// on the whole order once. (UNVERIFIED for Dhan until a contract note
    /// shows a part-filled order; D-5127.)
    ///
    /// # Errors
    ///
    /// * [`ChargeError::Cost`] carrying [`CostError::NotPositive`] for a part
    ///   whose quantity or value is not positive.
    /// * [`ChargeError::Cost`] carrying [`CostError::Overflow`] when the joined
    ///   quantity or value leaves `i64`.
    ///
    /// # Examples
    ///
    /// ```
    /// use brutex_core::price::Paisa;
    /// use costs::ledger::ExecutedOrder;
    /// use costs::rate::OrderSide;
    ///
    /// let first = ExecutedOrder {
    ///     side: OrderSide::Buy,
    ///     qty: 30,
    ///     value_paisa: Paisa::from_raw(45_600_00),
    ///     broker_square_off: false,
    /// };
    /// let whole = first.with_part(20, Paisa::from_raw(30_400_20))?;
    /// assert_eq!((whole.qty, whole.value_paisa.raw()), (50, 76_000_20));
    /// # Ok::<(), costs::charge_error::ChargeError>(())
    /// ```
    pub fn with_part(self, qty: i64, value_paisa: Paisa) -> Result<Self, ChargeError> {
        positive("order part quantity", qty)?;
        positive("order part value", value_paisa.raw())?;
        Ok(Self {
            qty: self
                .qty
                .checked_add(qty)
                .ok_or(overflow("a part-filled order's quantity"))?,
            value_paisa: self
                .value_paisa
                .raw()
                .checked_add(value_paisa.raw())
                .map(Paisa::from_raw)
                .ok_or(overflow("a part-filled order's value"))?,
            ..self
        })
    }

    /// Refuses a quantity or a value that is not positive, by name.
    fn checked(self) -> Result<Self, ChargeError> {
        positive("order quantity", self.qty)?;
        positive("order value", self.value_paisa.raw())?;
        Ok(self)
    }
}

/// A not-positive refusal naming the input, or nothing.
fn positive(quantity: &'static str, value: i64) -> Result<(), ChargeError> {
    if value > 0 {
        Ok(())
    } else {
        Err(CostError::NotPositive { quantity, value }.into())
    }
}

/// An overflow refusal naming the operation.
const fn overflow(operation: &'static str) -> ChargeError {
    ChargeError::Cost(CostError::Overflow { operation })
}

/// One client's executed orders in one security on one day, as running
/// totals, priced when the day closes.
///
/// No `Default`: a ledger is a kind of trade, at a venue, on a day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DayLedger {
    kind: TradeKind,
    exchange: Exchange,
    day: TradeDay,
    totals: Totals,
}

impl DayLedger {
    /// Opens an empty ledger for `kind` at `exchange` on `day`.
    ///
    /// # Errors
    ///
    /// [`ChargeError::OutOfScope`] for an ETF or a commodity, which neither
    /// mode prices (D-5125).
    ///
    /// # Examples
    ///
    /// ```
    /// use brutex_core::instrument::Exchange;
    /// use brutex_core::price::Paisa;
    /// use costs::day::TradeDay;
    /// use costs::ledger::{DayLedger, ExecutedOrder};
    /// use costs::rate::OrderSide;
    /// use costs::schedule::{Mode, Product, RateSelector, TradeKind};
    ///
    /// let mut day = DayLedger::new(
    ///     TradeKind::Equity(Product::Intraday),
    ///     Exchange::Nse,
    ///     TradeDay::new(2026, 10, 15)?,
    /// )?;
    /// for (side, value) in [(OrderSide::Buy, 76_000_00), (OrderSide::Sell, 76_500_00)] {
    ///     day.add(ExecutedOrder {
    ///         side,
    ///         qty: 50,
    ///         value_paisa: Paisa::from_raw(value),
    ///         broker_square_off: false,
    ///     })?;
    /// }
    /// let precise = day.close(Mode::Precise, RateSelector::CurrentOnEveryDate)?;
    /// let dhan = day.close(Mode::DhanCalculator, RateSelector::CurrentOnEveryDate)?;
    /// assert_eq!((precise.total().raw(), dhan.total().raw()), (73_90, 74_28));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(kind: TradeKind, exchange: Exchange, day: TradeDay) -> Result<Self, ChargeError> {
        if let Some(refusal) = kind.refusal() {
            return Err(refusal);
        }
        Ok(Self {
            kind,
            exchange,
            day,
            totals: Totals::EMPTY,
        })
    }

    /// What the ledger holds.
    #[must_use]
    pub const fn kind(&self) -> TradeKind {
        self.kind
    }

    /// Where.
    #[must_use]
    pub const fn exchange(&self) -> Exchange {
        self.exchange
    }

    /// The trade day.
    #[must_use]
    pub const fn day(&self) -> TradeDay {
        self.day
    }

    /// How many executed orders the day holds.
    #[must_use]
    pub const fn orders(&self) -> i64 {
        self.totals.orders()
    }

    /// Records one executed order. All or nothing: a refused order leaves the
    /// ledger exactly as it was. O(1): C-CHG-07.
    ///
    /// # Errors
    ///
    /// * [`ChargeError::Cost`] carrying [`CostError::NotPositive`] for a
    ///   quantity or a value that is not positive.
    /// * [`ChargeError::SquareOffNotApplicable`] for a broker square-off on a
    ///   delivery or MTF day.
    /// * [`ChargeError::Cost`] carrying an overflow of a day total.
    pub fn add(&mut self, order: ExecutedOrder) -> Result<(), ChargeError> {
        let order = order.checked()?;
        self.totals
            .add(
                self.kind,
                order.side,
                order.qty,
                order.value_paisa,
                order.broker_square_off,
            )
            .map(|_| ())
    }

    /// Records an exercise of an option worth `intrinsic_value`, settled as
    /// the caller names it: only [`Settlement::Cash`] is priced (D-5130). The
    /// exercise pays Dhan's ₹20 brokerage, and GST on it, as an executed
    /// order does (sources row B-05, D-5132); it is not counted in
    /// [`Self::orders`]. O(1): C-CHG-10.
    ///
    /// # Errors
    ///
    /// As [`crate::stack::ClientSecurityDay::exercise`]: a stock option
    /// settled by physical delivery is refused by name
    /// ([`crate::schedule::PHYSICAL_DELIVERY_REFUSAL`]) and the ledger is
    /// left exactly as it was.
    pub fn exercise(
        &mut self,
        intrinsic_value: Paisa,
        settlement: Settlement,
    ) -> Result<(), ChargeError> {
        self.totals.exercise(self.kind, intrinsic_value, settlement)
    }

    /// Prices the day in `mode` under `rate_set`: every line unrounded and
    /// rounded, the total, and the rounding adjustment between them. STT and
    /// stamp duty are rounded once for the day, as if this security were the
    /// client's only one that day ([`crate::stack::ClientDay`] rounds them
    /// across several). O(1): C-CHG-08.
    ///
    /// # Errors
    ///
    /// * Whatever [`RateCard::resolve`] refuses, in precise mode.
    /// * [`ChargeError::NotOnPage`] for [`Mode::DhanCalculator`] under
    ///   [`RateSelector::Dated`] (the page holds only today's rates), for an
    ///   exercise, and for BSE futures and options.
    /// * Whatever [`crate::stack::ClientSecurityDay::settle`] refuses: an
    ///   empty day, an open intraday position, a partly squared delivery day.
    pub fn close(&self, mode: Mode, rate_set: RateSelector) -> Result<DayCharges, ChargeError> {
        match mode {
            Mode::Precise => {
                let card = RateCard::resolve(self.kind, self.exchange, self.day, rate_set)?;
                settle_precise(&card, &self.totals)
            }
            Mode::DhanCalculator => match rate_set {
                RateSelector::CurrentOnEveryDate => {
                    settle_dhan(self.kind, self.exchange, self.day, &self.totals)
                }
                RateSelector::Dated => Err(DHAN_HAS_NO_DATED_RATES),
            },
        }
    }
}

/// Why the calculator mode cannot price under the dated rate set.
const DHAN_HAS_NO_DATED_RATES: ChargeError = ChargeError::NotOnPage {
    what: "dated rates (the calculator page holds only today's rates; price under \
           RateSelector::CurrentOnEveryDate, labelled)",
};

/// One executed order's own lines, **unrounded**, in [`LINE_NAMES`] order,
/// for an order whose lines do not depend on how its day netted: an
/// **intraday** equity order (an intraday day is priced only squared off), a
/// futures order or an options order (their taxes go by side).
///
/// **A delivery or MTF order is refused by name**
/// ([`ChargeError::NeedsItsDay`]): whether it pays delivery STT on its whole
/// value, delivery stamp duty and a DP share, or the non-delivery rates and
/// no DP, depends on whether its client-security-day squared off, and one
/// order cannot tell. [`DayLedger::order_lines_unrounded`] prices it against
/// its own day's netting (D-5131).
///
/// Each line is the order's exact share of its client-security-day's line,
/// in units of 10⁻⁹ rupee: brokerage (the order's own), the exchange charge
/// and the SEBI fee on its value, stamp duty on a buy, and STT on the value
/// the day's tax is levied on — for precise equity intraday that is **half**
/// the order's value at the rate, because `NSE/CMTR/5588` taxes the
/// squared-off quantity at the day's VWAP of the buys and the sells
/// together. GST is 18% of the order's own unrounded base (precise:
/// brokerage + exchange + SEBI; calculator: brokerage + exchange), half-up to
/// the unit; the broker square-off fee and its GST are carried by the order
/// that was the broker's. What the day's orders sum to is stated, and
/// proved, on [`DayLedger::order_lines_unrounded`].
///
/// # Errors
///
/// * [`ChargeError::NeedsItsDay`] for a delivery or MTF order.
/// * Whatever [`RateCard::resolve`] refuses, in precise mode; and for the
///   calculator mode, [`ChargeError::NotOnPage`] under the dated rate set and
///   for BSE futures and options.
/// * [`ChargeError::Cost`] carrying [`CostError::NotPositive`] for an order
///   that is not positive, or an overflow.
/// * [`ChargeError::SquareOffNotApplicable`] for a broker square-off on a
///   delivery or MTF order.
///
/// # Examples
///
/// ```
/// use brutex_core::instrument::Exchange;
/// use brutex_core::price::Paisa;
/// use costs::day::TradeDay;
/// use costs::ledger::{ExecutedOrder, order_lines_unrounded};
/// use costs::rate::OrderSide;
/// use costs::schedule::{Mode, Product, RateSelector, TradeKind};
///
/// let sell = ExecutedOrder {
///     side: OrderSide::Sell,
///     qty: 50,
///     value_paisa: Paisa::from_raw(76_500_00),
///     broker_square_off: false,
/// };
/// let lines = order_lines_unrounded(
///     TradeKind::Equity(Product::Intraday),
///     Exchange::Nse,
///     TradeDay::new(2026, 10, 15)?,
///     sell,
///     Mode::Precise,
///     RateSelector::CurrentOnEveryDate,
/// )?;
/// // Its STT share: half of ₹76,500 at 0.025% is ₹9.5625, exactly.
/// assert_eq!(lines[3], ("STT", lines[3].1));
/// assert_eq!(lines[3].1.units(), 9_562_500_000);
/// // A delivery order needs its day.
/// assert!(
///     order_lines_unrounded(
///         TradeKind::Equity(Product::Delivery),
///         Exchange::Nse,
///         TradeDay::new(2026, 10, 15)?,
///         sell,
///         Mode::Precise,
///         RateSelector::CurrentOnEveryDate,
///     )
///     .is_err()
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn order_lines_unrounded(
    kind: TradeKind,
    exchange: Exchange,
    day: TradeDay,
    order: ExecutedOrder,
    mode: Mode,
    rate_set: RateSelector,
) -> Result<[(&'static str, Exact); 11], ChargeError> {
    let order = applicable(kind, order)?;
    if let TradeKind::Equity(Product::Delivery | Product::Mtf) = kind {
        return Err(ChargeError::NeedsItsDay {
            kind: kind.as_str(),
        });
    }
    // An intraday day is priced only squared off; futures and options are
    // taxed by side, and their netting is never read.
    lines_of(kind, exchange, day, order, mode, rate_set, Netting::Squared).map(named)
}

impl DayLedger {
    /// One of this day's executed orders' own lines, **unrounded**, in
    /// [`LINE_NAMES`] order, priced against **the day's own netting** — the
    /// per-trade attribution of a day a backtest closes. Call it after the
    /// day's last [`Self::add`], as [`Self::close`] is called: the netting
    /// and the DP share are the day's as it then stands. O(1): a fixed
    /// handful of exact products, never a walk of the orders (C-CHG-09 in
    /// `crates/costs/benches/stack.rs`).
    ///
    /// The day nets exactly as [`Self::close`] nets it (one rule,
    /// `crate::stack::equity_netting`): on a **squared** equity day — intraday,
    /// delivery or MTF — an order pays non-delivery STT on half its value,
    /// non-delivery stamp duty on a buy, and no DP; on a **one-sided**
    /// delivery or MTF day it pays delivery STT on its whole value, delivery
    /// stamp duty on a buy, and a sell carries its share of the day's one DP
    /// charge and its GST, in proportion to its value among the day's sells
    /// (the whole of it for the day's only sell). The calculator mode prices
    /// as the page does, which nets nothing, and shares the page's DP the
    /// same way. Futures and options are taxed by side, as in
    /// [`order_lines_unrounded`].
    ///
    /// **What the day's orders sum to.** Every share is exact to the unit of
    /// 10⁻⁹ rupee, half-up where it falls between two units, and the day
    /// rounds its own line once; so over the day's orders brokerage, the
    /// exchange charge, the SEBI fee, stamp duty and the square-off lines sum
    /// to the day's unrounded lines, STT does too except for the day's VWAP
    /// rounding to the paisa (a day step no order can take), and the DP lines
    /// sum to the day's within half a unit per sell order — exactly when the
    /// day has one sell. GST is each order's own, on its own unrounded base,
    /// and is not a share of the day's GST (the day's is on the rounded
    /// base). Proved on a squared and a one-sided intraday, delivery and MTF
    /// day by
    /// `costs::ledger::tests::a_days_orders_sum_to_its_unrounded_lines_on_every_netting`.
    ///
    /// **An option exercise is not an order, so its lines belong to no
    /// order**: on an options day with exercises the orders' brokerage sums
    /// to the day's less Dhan's ₹20 per exercise
    /// ([`DayCharges::exercises`], sources row B-05, D-5132), and STT on
    /// exercise is in no order's lines. Proved by
    /// `costs::ledger::tests::an_exercise_pays_dhans_brokerage_and_its_gst_as_an_executed_order_does`.
    ///
    /// # Errors
    ///
    /// * [`ChargeError::Cost`] carrying [`CostError::NotPositive`] for an
    ///   order that is not positive.
    /// * [`ChargeError::SquareOffNotApplicable`] for a broker square-off on a
    ///   delivery or MTF order.
    /// * [`ChargeError::OrderNotInDay`] for an order larger, on its side,
    ///   than the day's whole side, in units or in value: it cannot be one of
    ///   the day's orders. (A smaller order of another day cannot be told
    ///   from the totals; keeping days apart is the caller's.)
    /// * In precise mode, what the day's netting refuses, exactly as
    ///   [`Self::close`] does: [`ChargeError::OpenIntradayPosition`] and
    ///   [`ChargeError::PartlySquaredDay`].
    /// * Otherwise as [`order_lines_unrounded`]: a rate the rate set cannot
    ///   supply, what the calculator page was never read for, an overflow.
    ///
    /// # Examples
    ///
    /// ```
    /// use brutex_core::instrument::Exchange;
    /// use brutex_core::price::Paisa;
    /// use costs::day::TradeDay;
    /// use costs::ledger::{DayLedger, ExecutedOrder};
    /// use costs::rate::OrderSide;
    /// use costs::schedule::{Mode, Product, RateSelector, TradeKind};
    ///
    /// // A delivery day bought and sold the same 100 shares: it squared off,
    /// // so each order pays the non-delivery rates and nobody pays DP.
    /// let mut day = DayLedger::new(
    ///     TradeKind::Equity(Product::Delivery),
    ///     Exchange::Nse,
    ///     TradeDay::new(2026, 10, 15)?,
    /// )?;
    /// let buy = ExecutedOrder {
    ///     side: OrderSide::Buy,
    ///     qty: 100,
    ///     value_paisa: Paisa::from_raw(15_000_00),
    ///     broker_square_off: false,
    /// };
    /// let sell = ExecutedOrder {
    ///     side: OrderSide::Sell,
    ///     value_paisa: Paisa::from_raw(14_800_00),
    ///     ..buy
    /// };
    /// day.add(buy)?;
    /// day.add(sell)?;
    /// let lines = day.order_lines_unrounded(sell, Mode::Precise, RateSelector::CurrentOnEveryDate)?;
    /// // STT: half of ₹14,800 at 0.025% is ₹1.85; DP: none.
    /// assert_eq!((lines[3].1.units(), lines[7].1.units()), (1_850_000_000, 0));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn order_lines_unrounded(
        &self,
        order: ExecutedOrder,
        mode: Mode,
        rate_set: RateSelector,
    ) -> Result<[(&'static str, Exact); 11], ChargeError> {
        let order = applicable(self.kind, order)?;
        let (day_units, day_value) = self.totals.side(order.side);
        let (units, value) = match order.side {
            OrderSide::Buy => ("bought quantity", "bought value"),
            OrderSide::Sell => ("sold quantity", "sold value"),
        };
        if order.qty > day_units {
            return Err(ChargeError::OrderNotInDay {
                figure: units,
                order: order.qty,
                day: day_units,
            });
        }
        if order.value_paisa.raw() > day_value.raw() {
            return Err(ChargeError::OrderNotInDay {
                figure: value,
                order: order.value_paisa.raw(),
                day: day_value.raw(),
            });
        }
        let netting = match (mode, self.kind) {
            (Mode::Precise, TradeKind::Equity(product)) => equity_netting(
                product,
                self.totals.side(OrderSide::Buy).0,
                self.totals.side(OrderSide::Sell).0,
            )?,
            // Futures and options are taxed by side, and the calculator page
            // nets nothing: their netting is never read.
            _ => Netting::Squared,
        };
        let [b, x, s, stt, ex, stamp, gst, dp, dp_gst, so, so_gst] = lines_of(
            self.kind,
            self.exchange,
            self.day,
            order,
            mode,
            rate_set,
            netting,
        )?;
        // The day's DP is one charge per security per day: a sell carries
        // its share of it by value, a buy carries none (its line is zero).
        let (dp, dp_gst) = (
            share(dp, order.value_paisa, day_value)?,
            share(dp_gst, order.value_paisa, day_value)?,
        );
        Ok(named([
            b, x, s, stt, ex, stamp, gst, dp, dp_gst, so, so_gst,
        ]))
    }
}

/// The order, checked: positive, and squared off by the broker only where a
/// ceiling square-off exists.
fn applicable(kind: TradeKind, order: ExecutedOrder) -> Result<ExecutedOrder, ChargeError> {
    let order = order.checked()?;
    if order.broker_square_off && !kind.can_be_squared_off_by_broker() {
        return Err(ChargeError::SquareOffNotApplicable {
            kind: kind.as_str(),
        });
    }
    Ok(order)
}

/// `line × part ÷ whole`, half-up to the unit: an order's share of a line
/// its day pays once. `whole` is the day's value on the order's side, which
/// [`DayLedger::order_lines_unrounded`] has checked is at least `part`.
fn share(line: Exact, part: Paisa, whole: Paisa) -> Result<Exact, ChargeError> {
    let numerator = line
        .units()
        .checked_mul(i128::from(part.raw()))
        .ok_or(overflow("an order's share of a day line"))?;
    Ok(Exact(round_half_up(numerator, i128::from(whole.raw()))?))
}

/// Every line of one order, unnamed, in [`LINE_NAMES`] order; the DP lines
/// are the whole day's DP if this order were its only sell.
fn lines_of(
    kind: TradeKind,
    exchange: Exchange,
    day: TradeDay,
    order: ExecutedOrder,
    mode: Mode,
    rate_set: RateSelector,
    netting: Netting,
) -> Result<[Exact; 11], ChargeError> {
    let value = order.value_paisa;
    let (bought, sold) = match order.side {
        OrderSide::Buy => (value, Paisa::ZERO),
        OrderSide::Sell => (Paisa::ZERO, value),
    };
    let brokerage = dhan_brokerage_units(kind, value)?;
    let (square_off, square_off_gst) = if order.broker_square_off {
        (
            Exact::of_paisa(DHAN_SQUARE_OFF_FEE),
            units_of(DHAN_SQUARE_OFF_FEE, GST.rate(), 1)?,
        )
    } else {
        (Exact::ZERO, Exact::ZERO)
    };
    let lines = match mode {
        Mode::Precise => {
            let card = RateCard::resolve(kind, exchange, day, rate_set)?;
            precise_order_lines(&card, order.side, value, bought, brokerage, netting)?
        }
        Mode::DhanCalculator => match rate_set {
            RateSelector::CurrentOnEveryDate => {
                crate::dhan_calc::order_lines(kind, exchange, value, bought, sold, brokerage)?
            }
            RateSelector::Dated => return Err(DHAN_HAS_NO_DATED_RATES),
        },
    };
    let [b, x, s, stt, stt_exercise, stamp, gst, dp, dp_gst] = lines;
    Ok([
        b,
        x,
        s,
        stt,
        stt_exercise,
        stamp,
        gst,
        dp,
        dp_gst,
        square_off,
        square_off_gst,
    ])
}

/// The eleven lines with their [`LINE_NAMES`].
fn named(lines: [Exact; 11]) -> [(&'static str, Exact); 11] {
    let [n0, n1, n2, n3, n4, n5, n6, n7, n8, n9, n10] = LINE_NAMES;
    let [l0, l1, l2, l3, l4, l5, l6, l7, l8, l9, l10] = lines;
    [
        (n0, l0),
        (n1, l1),
        (n2, l2),
        (n3, l3),
        (n4, l4),
        (n5, l5),
        (n6, l6),
        (n7, l7),
        (n8, l8),
        (n9, l9),
        (n10, l10),
    ]
}

/// The first nine precise lines of one order: brokerage, exchange, SEBI,
/// STT, STT on exercise (none: an order is not an exercise), stamp, GST, DP
/// and GST on DP. An equity order is taxed by its day's `netting`; a futures
/// or options order by its side alone.
fn precise_order_lines(
    card: &RateCard,
    side: OrderSide,
    value: Paisa,
    bought: Paisa,
    brokerage: Exact,
    netting: Netting,
) -> Result<[Exact; 9], ChargeError> {
    let exchange_charge = units_of(value, card.exchange_charge().rate(), 1)?;
    let sebi = units_of(value, card.sebi().rate(), 1)?;
    let (stt, stamp, dp) = match card.taxes() {
        Taxes::Equity {
            stt_non_delivery,
            stt_delivery,
            stamp_non_delivery,
            stamp_delivery,
            ..
        } => match netting {
            // Half the order's value: the day's non-delivery value is the
            // squared quantity at the VWAP of buys and sells together —
            // whatever the product, a squared day pays no DP.
            Netting::Squared => (
                units_of(value, stt_non_delivery.rate(), 2)?,
                units_of(bought, stamp_non_delivery.rate(), 1)?,
                Paisa::ZERO,
            ),
            // One side only: delivery, on the whole value; a sell debits the
            // demat, so it carries the DP.
            Netting::OneSided => (
                units_of(value, stt_delivery.rate(), 1)?,
                units_of(bought, stamp_delivery.rate(), 1)?,
                if side == OrderSide::Sell {
                    DHAN_DP_CHARGE
                } else {
                    Paisa::ZERO
                },
            ),
        },
        Taxes::Futures {
            stt_sell,
            stamp_buy,
        }
        | Taxes::Options {
            stt_sell,
            stamp_buy,
            ..
        } => {
            let sold = if side == OrderSide::Sell {
                value
            } else {
                Paisa::ZERO
            };
            (
                units_of(sold, stt_sell.rate(), 1)?,
                units_of(bought, stamp_buy.rate(), 1)?,
                Paisa::ZERO,
            )
        }
    };
    let gst_rate = card.gst().rate();
    let base = Exact::sum(&[brokerage, exchange_charge, sebi], "an order's GST base")?;
    let gst = base.times(gst_rate)?;
    Ok([
        brokerage,
        exchange_charge,
        sebi,
        stt,
        Exact::ZERO,
        stamp,
        gst,
        Exact::of_paisa(dp),
        units_of(dp, gst_rate, 1)?,
    ])
}

// ---------------------------------------------------------------------------
// No `Default` on the ledger or the rate set: checked by the compiler.
// ---------------------------------------------------------------------------

/// Implemented for every type; implemented a second time, with another
/// parameter, for every type with a `Default`. Naming the item through an
/// inferred parameter is then ambiguous — a compile error — exactly for a
/// type that has a `Default`.
trait NoDefault<Marker> {
    /// The item the check names.
    fn witness() {}
}

// Two markers, any two distinct types: `()` for the impl every type gets,
// `u8` for the impl only a `Default` type gets.
impl<T> NoDefault<()> for T {}
impl<T: Default> NoDefault<u8> for T {}

const _: fn() = || {
    let _ = <DayLedger as NoDefault<_>>::witness;
    let _ = <RateSelector as NoDefault<_>>::witness;
};

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
    use crate::stack::{ClientDay, ClientSecurityDay, NetTrade, Order};

    const INTRADAY: TradeKind = TradeKind::Equity(Product::Intraday);
    const DELIVERY: TradeKind = TradeKind::Equity(Product::Delivery);
    const TODAY: RateSelector = RateSelector::CurrentOnEveryDate;

    fn on() -> TradeDay {
        TradeDay::new(2026, 10, 15).expect("a real date")
    }

    fn rs(raw: i64) -> Paisa {
        Paisa::from_raw(raw)
    }

    fn order(side: OrderSide, qty: i64, price: i64) -> ExecutedOrder {
        ExecutedOrder {
            side,
            qty,
            value_paisa: rs(qty * price),
            broker_square_off: false,
        }
    }

    fn buy(qty: i64, price: i64) -> ExecutedOrder {
        order(OrderSide::Buy, qty, price)
    }

    fn sell(qty: i64, price: i64) -> ExecutedOrder {
        order(OrderSide::Sell, qty, price)
    }

    fn ledger(kind: TradeKind, exchange: Exchange, orders: &[ExecutedOrder]) -> DayLedger {
        let mut day = DayLedger::new(kind, exchange, on()).expect("a priced kind");
        for &o in orders {
            day.add(o).expect("in range");
        }
        day
    }

    fn units(lines: &[(&'static str, Exact); 11]) -> [i128; 11] {
        lines.map(|(_, line)| line.units())
    }

    #[test]
    fn a_ledger_closes_precise_to_exactly_what_a_client_security_day_settles() {
        // (kind, venue, orders, exercise): the same orders through both entry
        // points, every field of the priced day compared.
        let squared = ExecutedOrder {
            broker_square_off: true,
            ..sell(50, 1_530_00)
        };
        let cases: [(TradeKind, Exchange, Vec<ExecutedOrder>, Option<i64>); 7] = [
            (
                INTRADAY,
                Exchange::Nse,
                vec![buy(50, 1_520_00), sell(50, 1_530_00)],
                None,
            ),
            (
                INTRADAY,
                Exchange::Bse,
                vec![buy(50, 1_520_00), sell(50, 1_530_00)],
                None,
            ),
            (
                INTRADAY,
                Exchange::Nse,
                vec![buy(50, 1_520_00), squared],
                None,
            ),
            (DELIVERY, Exchange::Nse, vec![buy(50, 1_520_00)], None),
            (DELIVERY, Exchange::Nse, vec![sell(50, 1_530_00)], None),
            (
                TradeKind::Futures,
                Exchange::Nse,
                vec![buy(65, 25_200_00), sell(65, 25_250_00)],
                None,
            ),
            (
                TradeKind::Options,
                Exchange::Nse,
                vec![buy(65, 100_00), sell(65, 120_00)],
                Some(1_000_00),
            ),
        ];
        for (kind, exchange, orders, exercise) in cases {
            let mut day = ledger(kind, exchange, &orders);
            let mut csd = ClientSecurityDay::open(kind, exchange, on(), TODAY).expect("priced");
            for o in &orders {
                let mut placed =
                    Order::new(o.side, o.qty, rs(o.value_paisa.raw() / o.qty)).expect("valid");
                if o.broker_square_off {
                    placed = placed.squared_off_by_broker();
                }
                csd.add(placed).expect("in range");
            }
            if let Some(intrinsic) = exercise {
                day.exercise(rs(intrinsic), Settlement::Cash)
                    .expect("an option");
                csd.exercise(rs(intrinsic), Settlement::Cash)
                    .expect("an option");
            }
            let closed = day.close(Mode::Precise, TODAY).expect("priced");
            assert_eq!(
                closed,
                csd.settle().expect("priced"),
                "{kind} on {exchange:?}"
            );
            assert_eq!(closed.mode(), Mode::Precise);
            assert_eq!(day.orders(), i64::try_from(orders.len()).expect("small"));
            assert_eq!(
                (day.kind(), day.exchange(), day.day()),
                (kind, exchange, on())
            );
        }
    }

    #[test]
    fn a_part_filled_order_is_one_executed_order_and_pays_brokerage_once() {
        // Three parts of 100 @ ₹500: 0.03% of each part is ₹15, of the
        // whole order ₹45 — capped at ₹20, once.
        let part = buy(100, 500_00);
        let whole = part
            .with_part(100, rs(50_000_00))
            .and_then(|o| o.with_part(100, rs(50_000_00)))
            .expect("in range");
        assert_eq!((whole.qty, whole.value_paisa), (300, rs(150_000_00)));
        let one = ledger(INTRADAY, Exchange::Nse, &[whole, sell(300, 500_00)])
            .close(Mode::Precise, TODAY)
            .expect("priced");
        let three = ledger(
            INTRADAY,
            Exchange::Nse,
            &[part, part, part, sell(300, 500_00)],
        )
        .close(Mode::Precise, TODAY)
        .expect("priced");
        assert_eq!((one.orders(), three.orders()), (2, 4));
        assert_eq!(one.brokerage(), rs(40_00), "₹20 + ₹20: one order a side");
        assert_eq!(three.brokerage(), rs(65_00), "₹15 × 3 + ₹20");
        // Every line levied on the day's values is the same either way.
        for at in [1, 2, 3, 5] {
            assert_eq!(one.lines()[at], three.lines()[at], "{}", LINE_NAMES[at]);
            assert_eq!(one.unrounded_lines()[at], three.unrounded_lines()[at]);
        }
        // The calculator mode charges the parts once too.
        let dhan = ledger(INTRADAY, Exchange::Nse, &[whole, sell(300, 500_00)])
            .close(Mode::DhanCalculator, TODAY)
            .expect("priced");
        assert_eq!(dhan.brokerage(), rs(40_00));
    }

    #[test]
    fn a_part_and_an_order_that_are_not_positive_or_overflow_are_refused_by_name() {
        let not_positive = |quantity, value| {
            Some(ChargeError::Cost(CostError::NotPositive {
                quantity,
                value,
            }))
        };
        let named = |operation| Some(ChargeError::Cost(CostError::Overflow { operation }));
        let base = buy(1, 1_00);
        assert_eq!(
            base.with_part(0, rs(1)).err(),
            not_positive("order part quantity", 0)
        );
        assert_eq!(
            base.with_part(1, rs(-1)).err(),
            not_positive("order part value", -1)
        );
        let full = ExecutedOrder {
            qty: i64::MAX,
            ..base
        };
        assert_eq!(
            full.with_part(1, rs(1)).err(),
            named("a part-filled order's quantity")
        );
        let rich = ExecutedOrder {
            value_paisa: rs(i64::MAX),
            ..base
        };
        assert_eq!(
            rich.with_part(1, rs(1)).err(),
            named("a part-filled order's value")
        );

        // Added to a ledger: refused, and the ledger is exactly as it was.
        let mut day = ledger(INTRADAY, Exchange::Nse, &[buy(1, 1_00)]);
        let before = day;
        let zero = ExecutedOrder { qty: 0, ..base };
        assert_eq!(day.add(zero).err(), not_positive("order quantity", 0));
        let free = ExecutedOrder {
            value_paisa: Paisa::ZERO,
            ..base
        };
        assert_eq!(day.add(free).err(), not_positive("order value", 0));
        let mut wide = ledger(INTRADAY, Exchange::Nse, &[rich]);
        let wide_before = wide;
        assert_eq!(wide.add(base).err(), named("the day's buy value"));
        assert_eq!(wide, wide_before);
        assert_eq!(day, before);
        // A delivery holding is not squared off by the broker.
        let mut delivery = ledger(DELIVERY, Exchange::Nse, &[]);
        let squared = ExecutedOrder {
            broker_square_off: true,
            ..base
        };
        assert_eq!(
            delivery.add(squared),
            Err(ChargeError::SquareOffNotApplicable {
                kind: "equity delivery"
            })
        );
        assert_eq!(delivery.orders(), 0);
        // An exercise is an option's only.
        assert_eq!(
            day.exercise(rs(1_00), Settlement::Cash),
            Err(ChargeError::ExerciseNotApplicable {
                kind: "equity intraday"
            })
        );
        // A stock option settled by delivery is refused by name (D-5130),
        // and the ledger is exactly as it was: it still closes to the same
        // charges in both modes.
        let mut option = ledger(TradeKind::Options, Exchange::Nse, &[buy(500, 24_50)]);
        let option_before = option;
        assert_eq!(
            option.exercise(rs(9_750_00), Settlement::PhysicalDelivery),
            Err(crate::schedule::PHYSICAL_DELIVERY_REFUSAL)
        );
        assert_eq!(option, option_before);
        option.add(sell(500, 30_00)).expect("a sell");
        assert!(option.close(Mode::Precise, TODAY).is_ok());
        // An ETF and a commodity are not opened at all.
        assert_eq!(
            DayLedger::new(TradeKind::Etf, Exchange::Nse, on()),
            Err(crate::schedule::ETF_REFUSAL)
        );
        assert_eq!(
            DayLedger::new(TradeKind::Commodity, Exchange::Nse, on()),
            Err(crate::schedule::COMMODITY_REFUSAL)
        );
    }

    #[test]
    fn an_exercise_pays_dhans_brokerage_and_its_gst_as_an_executed_order_does() {
        // The verifier's probe (round 2): sources row B-05 applies Dhan's ₹20
        // per executed order to an option "expired, exercised, or assigned".
        // An exercise-only day of ₹10,000 intrinsic value is the ₹15 of STT,
        // ₹20 of brokerage and ₹3.60 of GST on it: ₹38.60, never ₹15 (D-5132).
        let day = TradeDay::new(2026, 10, 27).expect("a real date");
        let mut only = DayLedger::new(TradeKind::Options, Exchange::Nse, day).expect("priced");
        only.exercise(rs(10_000_00), Settlement::Cash)
            .expect("an option");
        let alone = only.close(Mode::Precise, TODAY).expect("priced");
        assert_eq!(
            (
                alone.orders(),
                alone.exercises(),
                alone.brokerage(),
                alone.gst(),
                alone.stt_exercise(),
                alone.total()
            ),
            (0, 1, rs(20_00), rs(3_60), rs(15_00), rs(38_60))
        );
        // The unrounded brokerage is the ₹20 exactly, and GST is 18% of it.
        let unrounded = units(&alone.unrounded_lines());
        assert_eq!(
            (unrounded[0], unrounded[6]),
            (20 * 1_000_000_000, 3_600_000_000)
        );
        // A buy of 75 at ₹100 the same day: two of Dhan's ₹20, one for the
        // order and one for the exercise, and GST on both. 18% of ₹40 +
        // ₹2.66 exchange + ₹0.01 SEBI = ₹7.6806 -> ₹7.68; total ₹40 + ₹2.66
        // + ₹0.01 + ₹7.68 + ₹15 = ₹65.35.
        let mut both = DayLedger::new(TradeKind::Options, Exchange::Nse, day).expect("priced");
        both.add(buy(75, 100_00)).expect("in range");
        both.exercise(rs(10_000_00), Settlement::Cash)
            .expect("an option");
        let closed = both.close(Mode::Precise, TODAY).expect("priced");
        assert_eq!(
            (
                closed.orders(),
                closed.exercises(),
                closed.brokerage(),
                closed.exchange_charge(),
                closed.gst(),
                closed.total()
            ),
            (1, 1, rs(40_00), rs(2_66), rs(7_68), rs(65_35))
        );
        // The order's own lines carry one ₹20: an order is not an exercise.
        let lines = both
            .order_lines_unrounded(buy(75, 100_00), Mode::Precise, TODAY)
            .expect("priced");
        assert_eq!(lines[0].1.units(), 20 * 1_000_000_000);
        // So the orders' brokerage is the day's less ₹20 per exercise, and
        // STT on exercise is in no order's lines (the ledger's stated sum).
        let day_lines = units(&closed.unrounded_lines());
        assert_eq!(
            day_lines[0],
            lines[0].1.units() + i128::from(closed.exercises()) * 20 * 1_000_000_000
        );
        assert_eq!((lines[4].1.units(), day_lines[4]), (0, 15 * 1_000_000_000));
        // The stack's own day prices the exercise the same way.
        let mut csd =
            ClientSecurityDay::open(TradeKind::Options, Exchange::Nse, day, TODAY).expect("priced");
        csd.add(Order::new(OrderSide::Buy, 75, rs(100_00)).expect("in range"))
            .expect("in range");
        csd.exercise(rs(10_000_00), Settlement::Cash)
            .expect("an option");
        assert_eq!(csd.settle(), Ok(closed));
    }

    #[test]
    fn a_close_refuses_what_its_mode_cannot_price_by_name() {
        let v1 = ledger(
            INTRADAY,
            Exchange::Nse,
            &[buy(50, 1_520_00), sell(50, 1_530_00)],
        );
        // The calculator page holds today's rates only.
        assert_eq!(
            v1.close(Mode::DhanCalculator, RateSelector::Dated),
            Err(DHAN_HAS_NO_DATED_RATES)
        );
        // The dated set has no equity history: the regime's refusal.
        assert!(matches!(
            v1.close(Mode::Precise, RateSelector::Dated),
            Err(ChargeError::Cost(CostError::Unverified(_)))
        ));
        // An empty day, in both modes.
        let empty = ledger(INTRADAY, Exchange::Nse, &[]);
        for mode in [Mode::Precise, Mode::DhanCalculator] {
            assert_eq!(empty.close(mode, TODAY), Err(ChargeError::EmptyDay));
        }
        // An exercise is not on the page; BSE options are not either.
        let mut exercised = ledger(TradeKind::Options, Exchange::Nse, &[buy(65, 100_00)]);
        exercised
            .exercise(rs(1_000_00), Settlement::Cash)
            .expect("an option");
        assert_eq!(
            exercised.close(Mode::DhanCalculator, TODAY),
            Err(ChargeError::NotOnPage {
                what: "an option exercise (the calculator page has no exercise input)"
            })
        );
        let bse = ledger(TradeKind::Options, Exchange::Bse, &[buy(20, 100_00)]);
        assert_eq!(
            bse.close(Mode::DhanCalculator, TODAY),
            Err(ChargeError::NotOnPage {
                what: "BSE futures and options"
            })
        );
        assert!(bse.close(Mode::Precise, TODAY).is_ok());
        // An open intraday day is priced by the page and refused by the stack.
        let open = ledger(INTRADAY, Exchange::Nse, &[buy(50, 1_520_00)]);
        assert_eq!(
            open.close(Mode::Precise, TODAY),
            Err(ChargeError::OpenIntradayPosition {
                bought: 50,
                sold: 0
            })
        );
        assert_eq!(
            open.close(Mode::DhanCalculator, TODAY)
                .expect("the page prices it")
                .total(),
            rs(28_71)
        );
    }

    #[test]
    fn every_line_is_reported_unrounded_and_rounded_with_the_adjustment_between() {
        let v1 = ledger(
            INTRADAY,
            Exchange::Nse,
            &[buy(50, 1_520_00), sell(50, 1_530_00)],
        );
        // Precise: units of 10^-9 rupee. Turnover ₹1,52,500; the VWAP
        // ₹1,525.00 exactly, so STT is on ₹76,250.
        let precise = v1.close(Mode::Precise, TODAY).expect("priced");
        assert_eq!(
            units(&precise.unrounded_lines()),
            [
                40_000_000_000,
                4_681_750_000,
                152_500_000,
                19_062_500_000,
                0,
                2_280_000_000,
                8_069_400_000,
                0,
                0,
                0,
                0
            ]
        );
        assert_eq!(precise.total(), rs(73_90));
        assert_eq!(precise.rounding_adjustment().units(), -346_150_000);
        assert_eq!(
            precise.unrounded_lines().map(|(name, _)| name),
            precise.lines().map(|(name, _)| name)
        );
        // Calculator: the page's own unrounded lines (the 15 Oct raw reading),
        // STT on the sell value.
        let dhan = v1.close(Mode::DhanCalculator, TODAY).expect("priced");
        assert_eq!(
            units(&dhan.unrounded_lines()),
            [
                40_000_000_000,
                4_681_750_000,
                152_500_000,
                19_125_000_000,
                0,
                2_280_000_000,
                8_042_724_000,
                0,
                0,
                0,
                0
            ]
        );
        assert_eq!(dhan.total(), rs(74_28));
        assert_eq!(dhan.rounding_adjustment().units(), -1_974_000);
        assert_eq!(
            dhan.lines().map(|(_, line)| line.raw()),
            [40_00, 4_68, 15, 19_00, 0, 2_00, 8_04, 0, 0, 0, 0]
        );
        assert_eq!(
            (dhan.stt_paisa(), dhan.stamp_paisa(), dhan.fees()),
            (rs(19_13), rs(2_28), rs(52_87))
        );
        assert_eq!(dhan.mode(), Mode::DhanCalculator);
        assert_eq!(dhan.rates_used(), [None; crate::stack::MAX_RATES_USED]);
        assert!(
            dhan.to_string()
                .starts_with("Dhan calculator (estimate) — today's rates applied to 2026-10-15")
        );
        // A broker square-off is two more lines in both modes; DP on a
        // delivery sell is ₹12.50 + ₹2.25 GST precise, ₹14.75 on the page.
        let squared = ExecutedOrder {
            broker_square_off: true,
            ..sell(50, 1_530_00)
        };
        let ceiling = ledger(INTRADAY, Exchange::Nse, &[buy(50, 1_520_00), squared]);
        for mode in [Mode::Precise, Mode::DhanCalculator] {
            let day = ceiling.close(mode, TODAY).expect("priced");
            assert_eq!(
                (day.square_off_fee(), day.square_off_gst()),
                (rs(20_00), rs(3_60))
            );
            assert_eq!(
                units(&day.unrounded_lines())[9..],
                [20_000_000_000, 3_600_000_000]
            );
        }
        let sold = ledger(DELIVERY, Exchange::Nse, &[sell(50, 1_530_00)]);
        let precise_dp = sold.close(Mode::Precise, TODAY).expect("priced");
        let page_dp = sold.close(Mode::DhanCalculator, TODAY).expect("priced");
        assert_eq!(
            (precise_dp.dp(), precise_dp.dp_gst()),
            (rs(12_50), rs(2_25))
        );
        assert_eq!((page_dp.dp(), page_dp.dp_gst()), (rs(14_75), Paisa::ZERO));
    }

    #[test]
    fn an_orders_lines_sum_to_its_days_unrounded_lines_where_the_line_is_linear() {
        let orders = [buy(50, 1_520_00), sell(50, 1_530_00)];
        let day = ledger(INTRADAY, Exchange::Nse, &orders);
        for mode in [Mode::Precise, Mode::DhanCalculator] {
            let closed = units(&day.close(mode, TODAY).expect("priced").unrounded_lines());
            let mut summed = [0_i128; 11];
            for o in orders {
                let lines = order_lines_unrounded(INTRADAY, Exchange::Nse, on(), o, mode, TODAY)
                    .expect("priced");
                assert_eq!(lines.map(|(name, _)| name), LINE_NAMES);
                for (at, line) in units(&lines).into_iter().enumerate() {
                    summed[at] += line;
                }
            }
            // Brokerage, exchange, SEBI, STT (the VWAP is exact here) and stamp.
            for at in [0, 1, 2, 3, 4, 5, 7, 8, 9, 10] {
                assert_eq!(summed[at], closed[at], "{mode:?} {}", LINE_NAMES[at]);
            }
        }
        // GST of one order, on its own unrounded base: precise includes the
        // SEBI fee, the calculator does not.
        let one = |mode| {
            units(
                &order_lines_unrounded(INTRADAY, Exchange::Nse, on(), orders[0], mode, TODAY)
                    .expect("priced"),
            )
        };
        // (₹20 + ₹2.3332 + ₹0.076) × 18% and (₹20 + ₹2.3332) × 18%.
        assert_eq!(one(Mode::Precise)[6], 4_033_656_000);
        assert_eq!(one(Mode::DhanCalculator)[6], 4_019_976_000);
    }

    #[test]
    fn a_days_orders_sum_to_its_unrounded_lines_on_every_netting() {
        // (kind, orders): a squared intraday, delivery and MTF day (the
        // verifier's day: ₹15,000 bought and ₹14,800 sold, VWAP ₹149.00
        // exactly), one-sided delivery and MTF days with two orders on their
        // side, and futures and options. Each day closed in both modes; the
        // day's orders' lines from the ledger summed (D-5131).
        let mtf = TradeKind::Equity(Product::Mtf);
        let squared = vec![buy(100, 150_00), sell(100, 148_00)];
        let cases: [(TradeKind, Vec<ExecutedOrder>); 9] = [
            (INTRADAY, squared.clone()),
            (DELIVERY, squared.clone()),
            (mtf, squared),
            // Two sells, shares of the DP exact: ₹3.75 and ₹8.75.
            (DELIVERY, vec![sell(30, 100_00), sell(70, 100_00)]),
            // Two sells, shares of the DP that fall between units.
            (mtf, vec![sell(30, 150_00), sell(70, 149_00)]),
            (DELIVERY, vec![buy(30, 150_00), buy(70, 149_00)]),
            (mtf, vec![buy(1, 1_00)]),
            (
                TradeKind::Futures,
                vec![buy(65, 25_200_00), sell(65, 25_250_00)],
            ),
            (TradeKind::Options, vec![buy(65, 100_00), sell(130, 120_00)]),
        ];
        for (kind, orders) in cases {
            let day = ledger(kind, Exchange::Nse, &orders);
            let sells = i128::try_from(orders.iter().filter(|o| o.side == OrderSide::Sell).count())
                .expect("small");
            for mode in [Mode::Precise, Mode::DhanCalculator] {
                let closed = units(&day.close(mode, TODAY).expect("priced").unrounded_lines());
                let mut summed = [0_i128; 11];
                for &o in &orders {
                    let lines = day.order_lines_unrounded(o, mode, TODAY).expect("priced");
                    assert_eq!(lines.map(|(name, _)| name), LINE_NAMES);
                    for (at, line) in units(&lines).into_iter().enumerate() {
                        summed[at] += line;
                    }
                }
                let case = format!("{kind:?} {mode:?} {orders:?}");
                // Brokerage, exchange, SEBI, STT, STT on exercise, stamp and
                // the square-off lines: exactly (every VWAP here is a whole
                // paisa).
                for at in [0, 1, 2, 3, 4, 5, 9, 10] {
                    assert_eq!(summed[at], closed[at], "{case}: {}", LINE_NAMES[at]);
                }
                // DP and its GST: within half a unit per sell order.
                for at in [7, 8] {
                    assert!(
                        (summed[at] - closed[at]).abs() * 2 <= sells,
                        "{case}: {} summed {} vs the day's {}",
                        LINE_NAMES[at],
                        summed[at],
                        closed[at]
                    );
                }
            }
        }
        // The verifier's squared delivery and MTF day, line by line: the
        // non-delivery rates on each order, and no DP (the day pays none).
        for kind in [DELIVERY, mtf] {
            let day = ledger(kind, Exchange::Nse, &[buy(100, 150_00), sell(100, 148_00)]);
            let closed = day.close(Mode::Precise, TODAY).expect("priced");
            assert_eq!(
                (closed.stt_paisa(), closed.stamp_paisa(), closed.dp()),
                (rs(3_73), rs(45), Paisa::ZERO),
                "{kind:?}"
            );
            let b = units(
                &day.order_lines_unrounded(buy(100, 150_00), Mode::Precise, TODAY)
                    .expect("priced"),
            );
            let s = units(
                &day.order_lines_unrounded(sell(100, 148_00), Mode::Precise, TODAY)
                    .expect("priced"),
            );
            // STT: half of ₹15,000 and of ₹14,800 at 0.025%; stamp: 0.003%
            // of ₹15,000 on the buy; DP: none.
            assert_eq!(
                [b[3], s[3], b[5], s[5], b[7], s[7], b[8], s[8]],
                [1_875_000_000, 1_850_000_000, 450_000_000, 0, 0, 0, 0, 0],
                "{kind:?}"
            );
        }
        // The exact DP shares of two equal-price sells: ₹3.75 and ₹8.75.
        let two = ledger(
            DELIVERY,
            Exchange::Nse,
            &[sell(30, 100_00), sell(70, 100_00)],
        );
        let dp = |o| {
            units(
                &two.order_lines_unrounded(o, Mode::Precise, TODAY)
                    .expect("priced"),
            )[7]
        };
        assert_eq!(
            (dp(sell(30, 100_00)), dp(sell(70, 100_00))),
            (3_750_000_000, 8_750_000_000)
        );
        // Where the netting is never read, the ledger's lines are the free
        // function's: an intraday squared day, futures and options.
        for (kind, orders) in [
            (INTRADAY, [buy(50, 1_520_00), sell(50, 1_530_00)]),
            (
                TradeKind::Futures,
                [buy(65, 25_200_00), sell(65, 25_250_00)],
            ),
            (TradeKind::Options, [buy(65, 100_00), sell(65, 120_00)]),
        ] {
            let day = ledger(kind, Exchange::Nse, &orders);
            for o in orders {
                for mode in [Mode::Precise, Mode::DhanCalculator] {
                    assert_eq!(
                        day.order_lines_unrounded(o, mode, TODAY),
                        order_lines_unrounded(kind, Exchange::Nse, on(), o, mode, TODAY),
                        "{kind:?} {mode:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_days_order_lines_refuse_what_the_day_cannot_hold_or_price_by_name() {
        let sold = ledger(DELIVERY, Exchange::Nse, &[sell(50, 1_530_00)]);
        let refused = |day: &DayLedger, o, mode| day.order_lines_unrounded(o, mode, TODAY).err();
        let not_in = |figure, order, day| Some(ChargeError::OrderNotInDay { figure, order, day });
        for mode in [Mode::Precise, Mode::DhanCalculator] {
            // More units, more value, or a side the day never traded.
            assert_eq!(
                refused(&sold, sell(51, 1_530_00), mode),
                not_in("sold quantity", 51, 50)
            );
            assert_eq!(
                refused(&sold, sell(50, 1_530_01), mode),
                not_in("sold value", 76_500_50, 76_500_00)
            );
            assert_eq!(
                refused(&sold, buy(1, 1_00), mode),
                not_in("bought quantity", 1, 0)
            );
            let bought = ledger(INTRADAY, Exchange::Nse, &[buy(10, 1_00), sell(10, 1_00)]);
            assert_eq!(
                refused(&bought, buy(10, 1_01), mode),
                not_in("bought value", 10_10, 10_00)
            );
            // An equal order is the day's: priced.
            assert!(refused(&sold, sell(50, 1_530_00), mode).is_none());
            // Not positive, and a broker square-off on a delivery day, are
            // refused before the day is read.
            assert_eq!(
                refused(&sold, sell(0, 1_530_00), mode),
                Some(ChargeError::Cost(CostError::NotPositive {
                    quantity: "order quantity",
                    value: 0
                }))
            );
            let ceiling = ExecutedOrder {
                broker_square_off: true,
                ..sell(50, 1_530_00)
            };
            assert_eq!(
                refused(&sold, ceiling, mode),
                Some(ChargeError::SquareOffNotApplicable {
                    kind: "equity delivery"
                })
            );
        }
        // Precise mode nets the day as its close does, and refuses what the
        // close refuses; the page nets nothing and prices both.
        let open = ledger(INTRADAY, Exchange::Nse, &[buy(50, 1_520_00)]);
        let partly = ledger(
            DELIVERY,
            Exchange::Nse,
            &[buy(100, 1_520_00), sell(60, 1_530_00)],
        );
        for (day, o, why) in [
            (
                open,
                buy(50, 1_520_00),
                ChargeError::OpenIntradayPosition {
                    bought: 50,
                    sold: 0,
                },
            ),
            (
                partly,
                sell(60, 1_530_00),
                ChargeError::PartlySquaredDay {
                    bought: 100,
                    sold: 60,
                },
            ),
        ] {
            assert_eq!(day.close(Mode::Precise, TODAY).err(), Some(why));
            assert_eq!(refused(&day, o, Mode::Precise), Some(why));
            assert!(day.close(Mode::DhanCalculator, TODAY).is_ok());
            assert!(refused(&day, o, Mode::DhanCalculator).is_none());
        }
        // The rate set: the page has no dated rates; the dated set has no
        // equity history.
        assert_eq!(
            sold.order_lines_unrounded(
                sell(50, 1_530_00),
                Mode::DhanCalculator,
                RateSelector::Dated
            )
            .err(),
            Some(DHAN_HAS_NO_DATED_RATES)
        );
        assert!(matches!(
            sold.order_lines_unrounded(sell(50, 1_530_00), Mode::Precise, RateSelector::Dated),
            Err(ChargeError::Cost(CostError::Unverified(_)))
        ));
        // A share that would leave i128 is refused by name, not wrapped.
        assert_eq!(
            share(Exact(i128::MAX), rs(2), rs(2)),
            Err(overflow("an order's share of a day line"))
        );
        assert_eq!(share(Exact(10), rs(1), rs(4)), Ok(Exact(3)), "2.5 half-up");
        assert_eq!(share(Exact(10), rs(1), rs(3)), Ok(Exact(3)), "3.33 down");
    }

    #[test]
    fn an_orders_lines_carry_dp_the_square_off_fee_and_every_kinds_taxes() {
        let lines = |kind, exchange, o, mode| {
            units(&order_lines_unrounded(kind, exchange, on(), o, mode, TODAY).expect("priced"))
        };
        // A delivery or MTF order is priced against its day: the day's only
        // order, so a one-sided day.
        let alone = |kind, o: ExecutedOrder, mode| {
            units(
                &ledger(kind, Exchange::Nse, &[o])
                    .order_lines_unrounded(o, mode, TODAY)
                    .expect("priced"),
            )
        };
        // A delivery sell carries the day's DP; a delivery buy its stamp.
        let sale = alone(DELIVERY, sell(50, 1_530_00), Mode::Precise);
        assert_eq!((sale[7], sale[8]), (12_500_000_000, 2_250_000_000));
        assert_eq!(sale[3], 76_500_00 * 10_000, "0.1% of the sell value");
        let page_sale = alone(DELIVERY, sell(50, 1_530_00), Mode::DhanCalculator);
        assert_eq!((page_sale[7], page_sale[8]), (14_750_000_000, 0));
        let purchase = alone(DELIVERY, buy(50, 1_520_00), Mode::Precise);
        assert_eq!((purchase[5], purchase[7]), (76_000_00 * 1_500, 0));
        // The page charges DP on a sell only: a delivery or MTF buy carries
        // none in calculator mode either (a `>=` on the sold value would put
        // ₹14.75 on every buy).
        for kind in [DELIVERY, TradeKind::Equity(Product::Mtf)] {
            let page_purchase = alone(kind, buy(50, 1_520_00), Mode::DhanCalculator);
            assert_eq!((page_purchase[7], page_purchase[8]), (0, 0), "{kind:?}");
            // Without its day, a delivery or MTF order is refused by name.
            for mode in [Mode::Precise, Mode::DhanCalculator] {
                assert_eq!(
                    order_lines_unrounded(
                        kind,
                        Exchange::Nse,
                        on(),
                        sell(50, 1_530_00),
                        mode,
                        TODAY
                    )
                    .err(),
                    Some(ChargeError::NeedsItsDay {
                        kind: kind.as_str()
                    }),
                    "{kind:?} {mode:?}"
                );
            }
        }
        // Futures and options: STT on a sell only, stamp on a buy only.
        for (kind, stt, stamp) in [
            (TradeKind::Futures, 5_000, 200),
            (TradeKind::Options, 15_000, 300),
        ] {
            let b = lines(kind, Exchange::Nse, buy(65, 100_00), Mode::Precise);
            let s = lines(kind, Exchange::Nse, sell(65, 100_00), Mode::Precise);
            assert_eq!((b[3], b[5]), (0, 6_500_00 * stamp));
            assert_eq!((s[3], s[5]), (6_500_00 * stt, 0));
            assert_eq!(b[0], 20_000_000_000, "₹20 per F&O order");
        }
        // The broker's order carries the fee and its GST.
        let squared = ExecutedOrder {
            broker_square_off: true,
            ..sell(50, 1_530_00)
        };
        let fee = lines(INTRADAY, Exchange::Nse, squared, Mode::Precise);
        assert_eq!(fee[9..], [20_000_000_000, 3_600_000_000]);
        // Refusals, by name.
        let refused = |kind, exchange, o, mode, set| {
            order_lines_unrounded(kind, exchange, on(), o, mode, set).err()
        };
        assert_eq!(
            refused(DELIVERY, Exchange::Nse, squared, Mode::Precise, TODAY),
            Some(ChargeError::SquareOffNotApplicable {
                kind: "equity delivery"
            })
        );
        assert_eq!(
            refused(INTRADAY, Exchange::Nse, buy(0, 1), Mode::Precise, TODAY),
            Some(ChargeError::Cost(CostError::NotPositive {
                quantity: "order quantity",
                value: 0
            }))
        );
        assert_eq!(
            refused(
                INTRADAY,
                Exchange::Nse,
                buy(1, 1),
                Mode::DhanCalculator,
                RateSelector::Dated
            ),
            Some(DHAN_HAS_NO_DATED_RATES)
        );
        assert_eq!(
            refused(
                TradeKind::Options,
                Exchange::Bse,
                buy(1, 1),
                Mode::DhanCalculator,
                TODAY
            ),
            Some(ChargeError::NotOnPage {
                what: "BSE futures and options"
            })
        );
        assert_eq!(
            refused(
                TradeKind::Etf,
                Exchange::Nse,
                buy(1, 1),
                Mode::Precise,
                TODAY
            ),
            Some(crate::schedule::ETF_REFUSAL)
        );
        // An ETF and a commodity order are refused by name in both modes,
        // before any rate card or calculator constant is read.
        for (kind, refusal) in [
            (TradeKind::Etf, crate::schedule::ETF_REFUSAL),
            (TradeKind::Commodity, crate::schedule::COMMODITY_REFUSAL),
        ] {
            for mode in [Mode::Precise, Mode::DhanCalculator] {
                assert_eq!(
                    refused(kind, Exchange::Nse, sell(1, 1), mode, TODAY),
                    Some(refusal),
                    "{kind:?} {mode:?}"
                );
            }
        }
        assert!(matches!(
            refused(
                INTRADAY,
                Exchange::Nse,
                buy(1, 1),
                Mode::Precise,
                RateSelector::Dated
            ),
            Some(ChargeError::Cost(CostError::Unverified(_)))
        ));
    }

    #[test]
    fn the_winner_gate_pages_round_trip_charges_are_the_calculators_not_the_precise_stacks() {
        // state/pages-20261016/winner-gate.html, "What after charges does to a
        // tiny stop": an equal-price NSE equity intraday round trip costs
        // ₹82.65 at ₹1,00,000 and ₹224.43 at ₹5,00,000 "from our formula".
        // Reproduced here, never tuned to: those are the Dhan calculator's
        // figures (GST without the SEBI fee). The precise stack — the mode a
        // win is decided in — charges ₹82.68 and ₹224.61, and the page is
        // marked for correction (D-5128).
        for (units_bought, page, precise_total) in [(100, 82_65, 82_68), (500, 224_43, 224_61)] {
            let trip = ledger(
                INTRADAY,
                Exchange::Nse,
                &[buy(units_bought, 1_000_00), sell(units_bought, 1_000_00)],
            );
            let precise = trip.close(Mode::Precise, TODAY).expect("priced");
            let dhan = trip.close(Mode::DhanCalculator, TODAY).expect("priced");
            assert_eq!(precise.total(), rs(precise_total), "{units_bought} units");
            assert_eq!(dhan.total(), rs(page), "{units_bought} units");
            // The whole gap is GST on the SEBI fee, plus the calculator's
            // unrounded STT and stamp, which are whole rupees here.
            assert_eq!(precise.gst().raw() - dhan.gst().raw(), precise_total - page);
        }
    }

    #[test]
    fn a_calculator_day_cannot_decide_a_client_day_or_a_trade() {
        let trip = ledger(
            INTRADAY,
            Exchange::Nse,
            &[buy(50, 1_520_00), sell(50, 1_530_00)],
        );
        let dhan = trip.close(Mode::DhanCalculator, TODAY).expect("priced");
        let precise = trip.close(Mode::Precise, TODAY).expect("priced");
        let refused = |figure| Err(ChargeError::NotPrecise { figure });
        assert_eq!(
            ClientDay::new().add(&dhan),
            refused("a client-day's STT and stamp duty")
        );
        let not_a_net = Some(ChargeError::NotPrecise {
            figure: crate::stack::NET_LABEL,
        });
        assert_eq!(NetTrade::of_day(&dhan).err(), not_a_net);
        assert_eq!(NetTrade::of_days(&dhan, &precise).err(), not_a_net);
        assert_eq!(NetTrade::of_days(&precise, &dhan).err(), not_a_net);
        assert!(NetTrade::of_day(&precise).is_ok());
    }
}
