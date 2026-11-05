//! FNO-SIZE: the sizing and admission policies of the F&O engine, setting S82 (D-8630).
//!
//! The operator asked how to choose 20 when 50 or 100 entries want the same second, or whether
//! to drop the slot limit and split the capital. This module answers with data: the policy is a
//! named run dimension, and all four are walked and judged on their own, never one silently.
//!
//! * **SIZE-FIXED-20-SEEDED**, today's rule: FNO-NET's quantity, FNO-PORTFOLIO's 20 slots and
//!   its cap tie-break order S81 (4.1 of the part's design).
//! * **SIZE-FIXED-20-RANKED**: the same quantity, offered in the order of a learned edge, then
//!   the smaller stop in R, the larger liquidity, a seeded key, the stock and the recipe (D-8634).
//! * **SIZE-SPLIT-ALL**: the free margin of one second split equally across its signals (D-8632).
//! * **SIZE-RISK**: equal risk per trade under an open-risk cap, both budgets learned (D-8633).
//!
//! [`plan`] returns who is offered, with what quantity and in what order; FNO-PORTFOLIO stays
//! the one owner of the five caps and of S81 and admits or refuses each entry as it does today.
//! It takes every other part's figures as plain integers, so it calls no code of FNO-PORTFOLIO,
//! FNO-NET, FNO-LEVERAGE or FNO-WFT. It never re-sizes a position after its start, never forces
//! a reduction, never reads a forward month for a setting and never changes a Winner Gate
//! number. It emits no telemetry and writes no record: FNO-RUN carries its labels.
//!
//! # The decision instant
//!
//! One batch is the entry intents whose order starts in one second (D-8631). A batch whose
//! candidates differ in `start` is refused, so two signals of one candle whose stocks first
//! trade in different seconds are two instants, the earlier first.
//!
//! # Cost
//!
//! [`plan`] does a fixed amount of work per candidate, O(1) a candidate for at most
//! [`UNIVERSE`] candidates, plus two bounded batch steps that are not O(1) and say so in
//! `docs/06-limits.md`: RANKED's fixed network of at most 4,608 compare-exchanges, and the
//! duplicate check's comparisons among the candidates of one stock, at most 32,640 a batch. It
//! allocates nothing when the [`Plan`]'s rows already hold [`UNIVERSE`] capacity. The proofs
//! are the seven rows C-FNO-SIZE-01..07 of `crates/runner/benches/ratio.rs`, whose measuring
//! functions live in [`measure`] and run at tiny sizes in
//! `runner::fno_size::tests::the_measurements_run_at_tiny_sizes`.

use core::cmp::Ordering;

pub mod capital;
pub mod edge;
pub mod measure;
pub mod order;
pub mod risk;
mod share;

#[cfg(test)]
mod tests;

pub use capital::CapitalNeeded;
pub use edge::{Edge, EdgeTable, EdgeTrade, Level};
pub use risk::{RiskBook, RiskBudget};

/// The module's version, the first term of every policy's canonical bytes.
pub const VERSION: &str = "fno-size/v1";

/// The stock positions a batch can name: the 218 of `fno218.txt`, padded to 256 as
/// FNO-PORTFOLIO's stock table is. Also the most candidates one batch may hold.
pub const UNIVERSE: usize = 256;

/// Minutes of session: minute 0 is 09:15 and minute 374 is 15:29.
pub const SESSION_MINUTES: u16 = 375;

/// SPEC 11.3's margin floor, `max(0.20, M / 100)`, in parts per million. A candidate's rate is
/// already at or above it: the caller applied it.
pub const MARGIN_FLOOR_PPM: u32 = 200_000;

/// Parts per million, and millionths of a paisa per paisa.
const PPM: i128 = 1_000_000;

/// The label of SIZE-FIXED-20-SEEDED, the operator's text (section 6 of the part's design).
pub const FIXED_SEEDED_LABEL: &str = "SIZE-FIXED-20-SEEDED: Rs 1 lakh margin x the stock's leverage a position, at most 20 open, simultaneous signals in the cap tie-break order S81 with its seeded spread (today's rule)";

/// The label of SIZE-FIXED-20-RANKED, the operator's text.
pub const FIXED_RANKED_LABEL: &str = "SIZE-FIXED-20-RANKED: Rs 1 lakh margin x the stock's leverage a position, at most 20 open, simultaneous signals by the learned edge (net per rupee of the candidate's learning trades for that stock, side and time of day, a procedure's recipes pooled; a learning month is scored without its own trades, its entry times still shaping the time-of-day buckets), then the smaller stop in R, then liquidity, then a seeded order";

/// The label of SIZE-SPLIT-ALL, the operator's text.
pub const SPLIT_ALL_LABEL: &str = "SIZE-SPLIT-ALL: no 20-slot cap; the free margin of each second split equally across its signals, whole shares, at least one each";

/// The label of SIZE-RISK, the operator's text.
pub const RISK_LABEL: &str = "SIZE-RISK: equal risk per trade (risk budget / stop distance), capped by the stock's leverage on an equal share of the free margin, total open risk capped; both budgets from learning months only";

/// The account limits every policy keeps, the operator's text of 3 Nov 2026.
pub const LIMITS_LABEL: &str = "account limits: margin in use at most Rs 20,00,000 (or the capital, when carried losses leave less) and open exposure at most Rs 1,00,00,000, checked at every entry on positions marked at their last traded price; a later mark can carry either past its limit and forces no reduction (SPEC 11.3; Dhan's MTM rule, 9.8, acts on losses only), so the peaks are reported (operator, 3 Nov 2026)";

/// What the capital actually needed means under the two FIXED policies.
pub const CAPITAL_FIXED_LABEL: &str = "capital actually needed: the largest margin in use after an admission less the carried net then (quantities never read capital)";

/// What the capital actually needed means under SIZE-SPLIT-ALL and SIZE-RISK.
pub const CAPITAL_SHARE_LABEL: &str =
    "capital actually needed: the whole Rs 20 lakh (quantities are shares of the free margin)";

/// The base the Winner Gate's minimum returns are judged on, and what is shown beside it.
pub const RETURNS_LABEL: &str = "minimum returns judged on Rs 20 lakh (OP-22, the stricter); on the capital actually needed shown beside";

/// The value at `at`, or `otherwise` past the end. Every table this module reads this way is
/// sized for the indices it is read with, so `otherwise` is the reading of a slot that exists
/// in no table rather than a fallback for one that does.
fn read<T: Copy>(table: &[T], at: usize, otherwise: T) -> T {
    table.get(at).copied().unwrap_or(otherwise)
}

/// Writes `value` at `at`. The tables are fixed arrays indexed by a stock position (a `u8`
/// into 256 rows), a candidate index (at most [`UNIVERSE`]) or a linked-list node (at most
/// 256), so the slot is always there and the loop runs exactly once.
fn put<T: Copy>(table: &mut [T], at: usize, value: T) {
    for slot in table.iter_mut().skip(at).take(1) {
        *slot = value;
    }
}

/// One of the four sizing and admission policies, setting S82 (form V: all are run).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Policy {
    /// SIZE-FIXED-20-SEEDED: today's sizing and order.
    FixedSeeded,
    /// SIZE-FIXED-20-RANKED: today's sizing, offered by the learned edge.
    FixedRanked,
    /// SIZE-SPLIT-ALL: equal shares of the free margin, no 20-slot cap.
    SplitAll,
    /// SIZE-RISK: equal risk per trade, open risk capped.
    Risk,
}

impl Policy {
    /// Every policy, in the order the part's design names them.
    pub const ALL: [Self; 4] = [
        Self::FixedSeeded,
        Self::FixedRanked,
        Self::SplitAll,
        Self::Risk,
    ];

    /// The policy's name, the setting's value.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::FixedSeeded => "SIZE-FIXED-20-SEEDED",
            Self::FixedRanked => "SIZE-FIXED-20-RANKED",
            Self::SplitAll => "SIZE-SPLIT-ALL",
            Self::Risk => "SIZE-RISK",
        }
    }

    /// The policy's label, the operator's text byte for byte.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::FixedSeeded => FIXED_SEEDED_LABEL,
            Self::FixedRanked => FIXED_RANKED_LABEL,
            Self::SplitAll => SPLIT_ALL_LABEL,
            Self::Risk => RISK_LABEL,
        }
    }

    /// How the plan's sized rows reach the account: RANKED's in the plan's order, one by one;
    /// every other policy's through the account's own schedule and tie-break.
    #[must_use]
    pub const fn offer(self) -> Offer {
        match self {
            Self::FixedRanked => Offer::PlanOrder,
            Self::FixedSeeded | Self::SplitAll | Self::Risk => Offer::AccountOrder,
        }
    }

    /// Whether the account keeps FNO-PORTFOLIO's 20 slots: true for the two FIXED policies.
    #[must_use]
    pub const fn twenty_slots(self) -> bool {
        match self {
            Self::FixedSeeded | Self::FixedRanked => true,
            Self::SplitAll | Self::Risk => false,
        }
    }

    /// Whether the quantities read the capital, so the capital actually needed is the whole
    /// window capital (D-8636): true for SIZE-SPLIT-ALL and SIZE-RISK.
    #[must_use]
    pub const fn reads_capital(self) -> bool {
        !self.twenty_slots()
    }

    /// The candidate identity term: [`VERSION`], one zero byte, then the name.
    #[must_use]
    pub fn canonical_bytes(self) -> Vec<u8> {
        let name = self.name().as_bytes();
        let mut bytes = Vec::with_capacity(VERSION.len() + 1 + name.len());
        bytes.extend_from_slice(VERSION.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(name);
        bytes
    }
}

/// How a plan's sized rows are offered to the account.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Offer {
    /// Through `Account::schedule_second`, in the account's own tie-break order.
    AccountOrder,
    /// To `Account::admit` one by one, in the plan's row order.
    PlanOrder,
}

/// The side of an entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Side {
    /// A buy that opens a long position.
    Long,
    /// A sell that opens a short position.
    Short,
}

impl Side {
    /// Both sides.
    pub const ALL: [Self; 2] = [Self::Long, Self::Short];

    /// The side's column in a table of two.
    const fn index(self) -> usize {
        match self {
            Self::Long => 0,
            Self::Short => 1,
        }
    }

    /// The distance from `price` to `stop` in the side's losing direction: positive when the
    /// stop is on the losing side of the price, zero on it, negative through it.
    fn distance(self, price: i64, stop: i64) -> i128 {
        let (price, stop) = (i128::from(price), i128::from(stop));
        match self {
            Self::Long => price - stop,
            Self::Short => stop - price,
        }
    }
}

/// A stock's position in `fno218.txt`: the meaning of FNO-PORTFOLIO's stock id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StockId(u8);

impl StockId {
    /// The stock at `position`.
    #[must_use]
    pub const fn new(position: u8) -> Self {
        Self(position)
    }

    /// The position.
    #[must_use]
    pub const fn position(self) -> u8 {
        self.0
    }

    /// The position as a table index.
    fn at(self) -> usize {
        usize::from(self.0)
    }
}

/// A set of stock positions: 256 bits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StockSet([u64; 4]);

impl StockSet {
    /// The set with no stock.
    pub const EMPTY: Self = Self([0; 4]);

    /// The word and the bit of `stock`.
    fn locate(stock: StockId) -> (usize, u64) {
        (stock.at() >> 6, 1 << (stock.0 & 63))
    }

    /// Adds `stock`; true when it was not already in the set.
    pub fn insert(&mut self, stock: StockId) -> bool {
        let (word, bit) = Self::locate(stock);
        let before = read(&self.0, word, 0);
        put(&mut self.0, word, before | bit);
        before & bit == 0
    }

    /// Removes `stock`; true when it was in the set.
    pub fn remove(&mut self, stock: StockId) -> bool {
        let (word, bit) = Self::locate(stock);
        let before = read(&self.0, word, 0);
        put(&mut self.0, word, before & !bit);
        before & bit != 0
    }

    /// Whether `stock` is in the set.
    #[must_use]
    pub fn contains(&self, stock: StockId) -> bool {
        let (word, bit) = Self::locate(stock);
        read(&self.0, word, 0) & bit != 0
    }

    /// How many stocks the set holds.
    #[must_use]
    pub const fn len(&self) -> u32 {
        let [a, b, c, d] = self.0;
        a.count_ones() + b.count_ones() + c.count_ones() + d.count_ones()
    }

    /// Whether the set holds no stock.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// An exact non-negative fraction. Equality and order are by value: 1/2 equals 2/4.
#[derive(Clone, Copy, Debug)]
pub struct Fraction {
    num: u64,
    den: u64,
}

impl Fraction {
    /// `num / den`.
    ///
    /// # Errors
    ///
    /// [`Invalid::ZeroDenominator`] when `den` is 0.
    pub const fn new(num: u64, den: u64) -> Result<Self, Invalid> {
        if den == 0 {
            return Err(Invalid::ZeroDenominator);
        }
        Ok(Self { num, den })
    }

    /// The numerator.
    #[must_use]
    pub const fn num(self) -> u64 {
        self.num
    }

    /// The denominator, never 0.
    #[must_use]
    pub const fn den(self) -> u64 {
        self.den
    }
}

impl Ord for Fraction {
    fn cmp(&self, other: &Self) -> Ordering {
        // Each side is a u64 times a u64, so the products are exact in u128.
        (u128::from(self.num) * u128::from(other.den))
            .cmp(&(u128::from(other.num) * u128::from(self.den)))
    }
}

impl PartialOrd for Fraction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Fraction {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Fraction {}

/// One entry intent whose order starts in the batch's second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// The stock.
    pub stock: StockId,
    /// 0 for a recipe alone, the pick position in a procedure.
    pub recipe: u32,
    /// The entry's side.
    pub side: Side,
    /// The order's start second; equal for every candidate of one batch.
    pub start: i64,
    /// The minute of session of `start`, 0..=374.
    pub minute: u16,
    /// `P_s` in paise, raw: the last traded price at or before `start` (SPEC 11.2).
    pub price: i64,
    /// The margin rate in parts per million, `max(200_000, M)` of the reading with the largest
    /// M: one million over the smallest leverage.
    pub rate_ppm: u32,
    /// FNO-NET's quantity for Rs 1 lakh margin times the leverage; read by the FIXED policies only.
    pub fixed_qty: u64,
    /// The protective stop's level in raw paise after the mapping reading; `None` on a
    /// stop-less lane.
    pub stop: Option<i64>,
    /// The stop's distance as a fraction of R, the ladder's range.
    pub stop_in_r: Fraction,
    /// Session-to-date traded value in paise to the decision candle's close (D-8635).
    pub liquidity: u128,
}

impl Candidate {
    /// FNO-NET's quantity times the price, in paise. A `u64` times an `i64` is exact in `i128`.
    fn fixed_value(&self) -> i128 {
        i128::from(self.fixed_qty) * i128::from(self.price)
    }

    /// The candidate's own caller defects, in the order the module documents.
    fn check(&self, policy: Policy, start: i64) -> Result<(), Invalid> {
        if self.start != start {
            return Err(Invalid::MixedStart {
                first: start,
                other: self.start,
            });
        }
        if self.price <= 0 {
            return Err(Invalid::NonPositivePrice(self.price));
        }
        if self.rate_ppm < MARGIN_FLOOR_PPM {
            return Err(Invalid::RateBelowFloor(self.rate_ppm));
        }
        if self.minute >= SESSION_MINUTES {
            return Err(Invalid::MinuteOutOfSession(self.minute));
        }
        if policy.twenty_slots() {
            if self.fixed_qty == 0 {
                return Err(Invalid::ZeroFixedQuantity {
                    stock: self.stock.0,
                });
            }
            if self.fixed_value() > i128::from(i64::MAX) {
                return Err(Invalid::Overflow("value"));
            }
        }
        Ok(())
    }
}

/// The account as FNO-RUN reads it at the batch's second, after settling and marking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccountView {
    /// Paise: Rs 20 lakh plus the settled carried net. May be 0 or negative.
    pub capital: i128,
    /// Millionths of a paisa: the margin in use of the positions in force, at their last marks.
    pub margin_in_use: i128,
    /// Paise: the exposure of the positions in force, at their last marks.
    pub exposure: i128,
    /// Millionths of a paisa: entries the order-rate rings queued, not yet sent.
    pub reserved_margin: i128,
    /// Paise: the same entries' value.
    pub reserved_exposure: i128,
    /// Entries sent today.
    pub trades_today: u16,
    /// Entries queued, not yet sent; each takes a trade when sent.
    pub queued: u16,
    /// Paise: the risk book's open risk plus the queued entries' planned risk (SIZE-RISK).
    pub open_risk: i128,
    /// Stocks with a position in force or a queued entry.
    pub busy: StockSet,
}

impl AccountView {
    /// Each figure that is a sum of positive amounts must be at least 0; a negative one would
    /// lift the free margin, the exposure room or the risk room past its limit.
    fn check(&self) -> Result<(), Invalid> {
        for (figure, name) in [
            (self.margin_in_use, "margin_in_use"),
            (self.exposure, "exposure"),
            (self.reserved_margin, "reserved_margin"),
            (self.reserved_exposure, "reserved_exposure"),
            (self.open_risk, "open_risk"),
        ] {
            if figure < 0 {
                return Err(Invalid::NegativeView(name));
            }
        }
        Ok(())
    }
}

/// The account limits FNO-RUN passes: the operator's Rs 20,00,000 margin limit, Rs 1,00,00,000
/// exposure limit and 100 entries a day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Paise: margin in use may not pass it, nor the capital when that is less.
    pub margin_cap: i128,
    /// Paise: open exposure may not pass it.
    pub exposure_cap: i128,
    /// Entries a day.
    pub trades_per_day: u16,
}

/// A policy and the inputs it needs.
#[derive(Clone, Copy, Debug)]
pub enum PolicyInput<'e> {
    /// SIZE-FIXED-20-SEEDED.
    FixedSeeded,
    /// SIZE-FIXED-20-RANKED: the window's edge table and the walk's seed.
    FixedRanked {
        /// The table of the walk's own window (D-8634).
        edge: &'e EdgeTable,
        /// The seed of tie-break (e), the fourth key.
        seed: u64,
    },
    /// SIZE-SPLIT-ALL.
    SplitAll,
    /// SIZE-RISK.
    Risk {
        /// `None` when the learning window's SEEDED walk had no trade with a stop at a positive
        /// distance.
        budget: Option<RiskBudget>,
    },
}

impl PolicyInput<'_> {
    /// The policy.
    #[must_use]
    pub const fn policy(&self) -> Policy {
        match self {
            Self::FixedSeeded => Policy::FixedSeeded,
            Self::FixedRanked { .. } => Policy::FixedRanked,
            Self::SplitAll => Policy::SplitAll,
            Self::Risk { .. } => Policy::Risk,
        }
    }
}

/// One batch's plan. FNO-RUN keeps one per walk, so the rows' capacity is reused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// The policy planned.
    pub policy: Policy,
    /// How the sized rows are offered.
    pub offer: Offer,
    /// One row per candidate: in offering order under [`Offer::PlanOrder`], in batch order
    /// otherwise.
    pub rows: Vec<PlanRow>,
    /// Distinct stocks of the batch that are not busy.
    pub eligible: u16,
    /// The share divisor of SIZE-SPLIT-ALL and SIZE-RISK: the eligible stocks capped by the
    /// day's trade room; 0 under the FIXED policies and for a batch with no eligible stock.
    pub k: u16,
    /// Paise: `floor(F / k)`, the free margin's equal share; 0 when `k` is 0.
    pub share_margin: i128,
    /// Paise: `floor(X / k)`, the exposure room's equal share; 0 when `k` is 0.
    pub share_exposure: i128,
    /// Paise: SIZE-RISK's risk each, `b`; 0 when no risk was budgeted.
    pub risk_each: i128,
    /// RANKED's compare-exchanges that fell through to the seeded key. It is 0 exactly when no
    /// two candidates tie on edge, stop in R and liquidity (a sorting network compares every
    /// pair that ends adjacent), so whether it is 0 does not depend on the batch's arrangement,
    /// though the count can.
    pub seed_reached: u32,
}

impl Plan {
    /// An empty plan whose rows already hold [`UNIVERSE`] capacity.
    #[must_use]
    pub fn new() -> Self {
        Self {
            policy: Policy::FixedSeeded,
            offer: Offer::AccountOrder,
            rows: Vec::with_capacity(UNIVERSE),
            eligible: 0,
            k: 0,
            share_margin: 0,
            share_exposure: 0,
            risk_each: 0,
            seed_reached: 0,
        }
    }
}

impl Default for Plan {
    fn default() -> Self {
        Self::new()
    }
}

/// One candidate's outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanRow {
    /// The candidate's index in the batch.
    pub index: u16,
    /// Sized, or refused by name.
    pub outcome: Outcome,
}

/// Sized, or refused by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Offered with this quantity.
    Sized(Sized),
    /// Never offered; counted under its word.
    Refused(SizeRefusal),
}

/// A sized entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sized {
    /// Shares, at least 1.
    pub qty: u64,
    /// Paise: `qty x price`.
    pub value: i128,
    /// Millionths of a paisa: `qty x price x rate_ppm`.
    pub margin: i128,
    /// Paise: SIZE-RISK's initial risk `qty x d`, at most the risk each; `None` otherwise.
    pub risk: Option<i128>,
    /// Which limit set the quantity.
    pub bound: Bound,
}

/// Which limit set a sized quantity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bound {
    /// FNO-NET's fixed target (the FIXED policies).
    FixedTarget,
    /// The stock's leverage on its equal share of the free margin.
    MarginShare,
    /// The equal share of the exposure room.
    ExposureShare,
    /// The risk each over the stop distance; a tie with a share goes here.
    Risk,
}

/// Why a candidate is not offered. The words never collide with FNO-PORTFOLIO's five, and
/// `while_open` means the same thing in both.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SizeRefusal {
    /// The stock has a position in force or a queued entry (SPEC 9.6).
    WhileOpen,
    /// The free margin is at or below 0.
    NoFreeMargin,
    /// The exposure room is at or below 0.
    NoExposureRoom,
    /// The share buys no whole share.
    BelowOneShare,
    /// The learning window budgeted no risk.
    NoLearnedRisk,
    /// The open risk is at or past its cap.
    NoRiskRoom,
    /// A stop-less lane under SIZE-RISK.
    NoStop,
    /// The stop sits at the price.
    ZeroStop,
    /// The stop sits through the price.
    StopThroughPrice,
}

impl SizeRefusal {
    /// Every refusal, for counting per policy.
    pub const ALL: [Self; 9] = [
        Self::WhileOpen,
        Self::NoFreeMargin,
        Self::NoExposureRoom,
        Self::BelowOneShare,
        Self::NoLearnedRisk,
        Self::NoRiskRoom,
        Self::NoStop,
        Self::ZeroStop,
        Self::StopThroughPrice,
    ];

    /// The word a refusal is counted under.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::WhileOpen => "while_open",
            Self::NoFreeMargin => "size_no_free_margin",
            Self::NoExposureRoom => "size_no_exposure_room",
            Self::BelowOneShare => "size_below_one_share",
            Self::NoLearnedRisk => "size_no_learned_risk",
            Self::NoRiskRoom => "size_no_risk_room",
            Self::NoStop => "size_no_stop",
            Self::ZeroStop => "size_zero_stop",
            Self::StopThroughPrice => "size_stop_through_price",
        }
    }
}

/// A caller defect, refused whole and changing nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Invalid {
    /// More than [`UNIVERSE`] candidates.
    BatchTooLarge(usize),
    /// Two candidates of one batch start in different seconds.
    MixedStart {
        /// The first candidate's start.
        first: i64,
        /// The other start.
        other: i64,
    },
    /// One (stock, recipe) twice in one batch.
    Duplicate {
        /// The stock's position.
        stock: u8,
        /// The recipe.
        recipe: u32,
    },
    /// A price at or below 0.
    NonPositivePrice(i64),
    /// A margin rate below [`MARGIN_FLOOR_PPM`].
    RateBelowFloor(u32),
    /// A minute at or past [`SESSION_MINUTES`].
    MinuteOutOfSession(u16),
    /// A zero FNO-NET quantity under a FIXED policy (FNO-NET refuses it first).
    ZeroFixedQuantity {
        /// The stock's position.
        stock: u8,
    },
    /// A fraction with a zero denominator.
    ZeroDenominator,
    /// A product or a sum past its type, named.
    Overflow(&'static str),
    /// A view figure below 0, named.
    NegativeView(&'static str),
    /// Rungs not strictly ascending, or past minute 374.
    BadRungs,
    /// A month outside the learning window.
    MonthOutsideLearning {
        /// The month.
        month: u16,
    },
    /// Learning trades out of month order.
    MonthsOutOfOrder,
    /// A learning trade's value at or below 0.
    NonPositiveValue,
    /// A stock admitted to the risk book while held, or released and not yet settled.
    StockInForce(u8),
    /// A stock released from the risk book while not held.
    NotInForce(u8),
    /// A time before the risk book's clock.
    TimeBackwards {
        /// The time given.
        at: i64,
        /// The clock.
        clock: i64,
    },
    /// A release whose last fill second is before its admission.
    ReleaseBeforeAdmit {
        /// The last fill second given.
        after: i64,
        /// The admission second.
        admitted: i64,
    },
    /// A risk budget at or below 0.
    NonPositiveBudget,
}

/// Plans one batch: the entry intents whose orders start in one second.
///
/// Each candidate gets one row: sized, or refused by name. Under [`Offer::PlanOrder`] the
/// rows are in offering order; otherwise in batch order. Nothing in `out` changes when the
/// batch is refused.
///
/// # Errors
///
/// Every caller defect of [`Invalid`] this function can meet: more than [`UNIVERSE`]
/// candidates, a negative view figure, then per candidate in batch order a mixed start, a price
/// at or below 0, a rate below the floor, a minute out of session, a zero or overflowing FIXED
/// quantity and a duplicate (stock, recipe), then the batch's free margin, exposure room or
/// exposure share past its type.
pub fn plan(
    input: &PolicyInput<'_>,
    view: &AccountView,
    limits: &Limits,
    batch: &[Candidate],
    out: &mut Plan,
) -> Result<(), Invalid> {
    let policy = input.policy();
    let eligible = check(policy, view, batch)?;
    let figures = share::Figures::of(input, view, limits, eligible)?;
    let mut order = [0u16; UNIVERSE];
    let seed_reached = match input {
        PolicyInput::FixedRanked { edge, seed } => order::rank(batch, edge, *seed, &mut order),
        PolicyInput::FixedSeeded | PolicyInput::SplitAll | PolicyInput::Risk { .. } => {
            order::arrival(batch.len(), &mut order);
            0
        }
    };
    out.policy = policy;
    out.offer = policy.offer();
    out.eligible = eligible;
    out.k = figures.k;
    out.share_margin = figures.share_margin;
    out.share_exposure = figures.share_exposure;
    out.risk_each = figures.risk_each;
    out.seed_reached = seed_reached;
    out.rows.clear();
    let rows = order
        .iter()
        .take(batch.len())
        .filter_map(|&index| batch.get(usize::from(index)).map(|c| (index, c)));
    for (index, candidate) in rows {
        let outcome = if view.busy.contains(candidate.stock) {
            Outcome::Refused(SizeRefusal::WhileOpen)
        } else {
            figures.size(candidate)
        };
        out.rows.push(PlanRow { index, outcome });
    }
    Ok(())
}

/// The validation pass: every caller defect of the batch and the view, and the eligible count.
/// It writes nothing outside its own stack.
fn check(policy: Policy, view: &AccountView, batch: &[Candidate]) -> Result<u16, Invalid> {
    if batch.len() > UNIVERSE {
        return Err(Invalid::BatchTooLarge(batch.len()));
    }
    view.check()?;
    // The duplicate check without a sort or a map: `first` holds, per stock, one plus the index
    // of its latest candidate so far (0: none), and `next` holds, per candidate, one plus the
    // index of the previous candidate of its stock. Each candidate meets only the earlier
    // candidates of its own stock: `c (c - 1) / 2` comparisons for `c` candidates on one stock.
    let mut first = [0u16; UNIVERSE];
    let mut next = [0u16; UNIVERSE];
    let mut seen = StockSet::EMPTY;
    let mut eligible: u16 = 0;
    let start = batch.first().map_or(0, |c| c.start);
    for (candidate, number) in batch.iter().zip(1u16..) {
        candidate.check(policy, start)?;
        let mut earlier = read(&first, candidate.stock.at(), 0);
        put(&mut next, usize::from(number - 1), earlier);
        while earlier != 0 {
            let at = usize::from(earlier - 1);
            if batch
                .get(at)
                .is_some_and(|other| other.recipe == candidate.recipe)
            {
                return Err(Invalid::Duplicate {
                    stock: candidate.stock.0,
                    recipe: candidate.recipe,
                });
            }
            earlier = read(&next, at, 0);
        }
        put(&mut first, candidate.stock.at(), number);
        if !view.busy.contains(candidate.stock) {
            eligible += u16::from(seen.insert(candidate.stock));
        }
    }
    Ok(eligible)
}
