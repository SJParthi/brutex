//! The F&O strategy path's money rules and order-rate rules (FNO-PORTFOLIO).
//!
//! An [`Account`] holds what SPEC 9.6, 9.7, 11.1 and 11.3 to 11.6
//! (`state/fib-strategy-20261030/SPEC.md`) say one live account holds:
//!
//! * twenty position slots in a fixed array (11.4);
//! * one lock per stock, `[Option<u8>; 256]`, naming the slot that holds it (9.6);
//! * the session's entry count, at most 100, cancelled entries included (11.4,
//!   OP-18);
//! * the open exposure and the margin in use, as running sums (11.3);
//! * the capital the margin is checked against: Rs 20 lakh at the window's first
//!   session plus every net since, under the worse cost clock (11.3, OP-21, and
//!   the READINESS ADDENDUM item 5 of `state/fib-strategy-20261030/PARTS.md`),
//!   with the reset-daily reading beside it;
//! * the order-rate rings and each resting order's modification count (9.7);
//! * the cap tie-break order (a) to (e) of 11.5, with the seed of (e).
//!
//! It decides and counts. It does not fill, size, price or charge: fills are
//! FNO-SEC-EXEC's, the quantity and every charge FNO-NET's, and every figure it
//! is handed arrives as a plain integer from its caller, because `runner` has no
//! `pull` arrow. It emits no telemetry (gate 17); its `cli` caller logs.
//!
//! # One second, in order
//!
//! [`Account::schedule_second`] takes every order that wants to go in one
//! second, puts exits before entries (risk first: 9.7, OP-43), orders each group
//! by its start second and then by the account's [`TieBreak`] (11.5), and acts
//! on them in that order: an exit is sent or queued, an entry is admitted,
//! refused or queued. The batch is checked whole before anything changes, so a
//! bad intent changes nothing. [`Account::admit`] and [`Account::send_exit`] are
//! the same steps for one order.
//!
//! An entry is checked, in this order: the stock's lock (`while_open`), a free
//! slot (`position_cap`), the session's 100 entries (`trade_cap`), the exposure
//! with this order's `q·P_s` (`exposure_cap`), the margin with this order's
//! (`margin_cap`), and only then the order-rate rings. A refusal is counted
//! under its name and is final; a queued order changed nothing and is checked
//! again, whole, when it is offered at the second the queue names (D-8181).
//!
//! # A released position stays in force through its last fill second
//!
//! [`Account::release`] records the last fill second, or the second a zero-fill
//! entry was cancelled (11.4, OP-19). Until a later second the position still
//! holds its slot and its stock's lock, still counts in the exposure and the
//! margin in use, and its net is not yet in the capital: SPEC 9.6 lets a new
//! entry on that stock start only strictly after that second, and this applies
//! the same reading, the one that admits less, to everything the position held
//! (D-8180). The next timed call at a later second settles it.
//!
//! # Cost
//!
//! Every call does a fixed amount of work, whatever the history behind it: the
//! slot table is twenty rows, the lock table 256, the rings fixed arrays, and
//! the sums running. [`Account::schedule_second`] also sorts the one second's
//! batch, which is bounded by that second's orders and never by the history
//! (`docs/06-limits.md`). C-FNO-PORT-01 measures admit and release in
//! `crates/runner/benches/ratio.rs`, through [`measure_admit_release`], which
//! `runner::fno_portfolio::the_bench_fixture_admits_every_entry_it_times` runs at
//! a tiny size.

use core::cmp::Ordering;
use std::time::Instant;

/// Capital at the first session of every window: Rs 20,00,000, in paisa
/// (SPEC 11.1, the operator's money rule; PARTS READINESS ADDENDUM item 5).
pub const CAPITAL_PAISA: i64 = 200_000_000;

/// The exposure cap: Rs 1,00,00,000 of open and pending position value at any
/// moment, in paisa (SPEC 11.1, the operator's money rule).
pub const EXPOSURE_CAP_PAISA: i64 = 1_000_000_000;

/// Positions open at once, pending entries included (SPEC 11.1, 11.4).
pub const POSITION_SLOTS: usize = 20;

/// Entry orders sent in one session, filled or not (SPEC 11.1, 11.4, OP-18).
pub const TRADES_PER_DAY: u16 = 100;

/// The margin rate's floor, 20% in parts per million: a position blocks its
/// value times `max(0.20, M/100)` (SPEC 11.3), the inverse of the operator's
/// 5x leverage cap (SPEC 11.1).
pub const MARGIN_FLOOR_PPM: u32 = 200_000;

/// The stock-lock table's length: the 218 names padded to 256 (SPEC 5.3), one
/// `u8` position each.
pub const STOCKS: usize = 256;

/// Modifications one order may take before it is cancelled and replaced
/// (SPEC 9.7). Dhan's figure, from `sources-leverage-execution.md` line 234
/// (T09-07); FNO-LAW records that source in `docs/00-charter.md`, and until it
/// does the figure is UNVERIFIED in this repository.
pub const MODIFICATIONS_PER_ORDER: u8 = 25;

/// Orders in any one second (SPEC 9.7; Dhan, T09-07, UNVERIFIED here as
/// [`MODIFICATIONS_PER_ORDER`] says).
pub const ORDERS_PER_SECOND: usize = 10;

/// Orders in any 60 seconds (SPEC 9.7; Dhan, T09-07, UNVERIFIED here).
pub const ORDERS_PER_MINUTE: usize = 250;

/// Orders in any 3,600 seconds (SPEC 9.7; Dhan, T09-07, UNVERIFIED here).
pub const ORDERS_PER_HOUR: usize = 1_000;

/// Orders in one session (SPEC 9.7; Dhan, T09-07, UNVERIFIED here).
pub const ORDERS_PER_DAY: u32 = 7_000;

/// Parts per million in one.
const PPM: i128 = 1_000_000;

/// Resting order roles per position, the length of [`OrderRole`].
const ROLES: usize = 5;

/// A slot id no admission issues: tickets start at one.
const NO_SLOT: SlotId = SlotId {
    index: 0,
    ticket: 0,
};

/// One stock of the run's universe, by its position in the universe file.
///
/// `fno218.txt` lists the 218 symbols one per line in byte order, so position
/// order is symbol order, and tie-break (d) compares these ids. A caller that
/// numbers the stocks any other way changes what (d) means, which is why the id
/// is the file position and nothing else (D-8182).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StockId(u8);

impl StockId {
    /// The stock at `position` of the universe file.
    #[must_use]
    pub const fn new(position: u8) -> Self {
        Self(position)
    }

    /// Its position in the universe file.
    #[must_use]
    pub const fn position(self) -> u8 {
        self.0
    }
}

/// A position, as [`Account::admit`] names it.
///
/// The ticket is the admission's own number in the account, so an id kept past
/// its position's release cannot act on the next position its slot holds:
/// every method refuses it as [`Invalid::StaleSlot`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SlotId {
    index: u8,
    ticket: u64,
}

impl SlotId {
    /// The slot's place in the table, below [`POSITION_SLOTS`].
    #[must_use]
    pub const fn index(self) -> u8 {
        self.index
    }

    /// The admission's number in its account, from one.
    #[must_use]
    pub const fn ticket(self) -> u64 {
        self.ticket
    }
}

/// An exact non-negative ratio `num / den` with `den > 0`, compared by value:
/// both sides cross-multiplied in `u128`, so `1/2` equals `2/4` and no float is
/// ever formed (FIB-I10).
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
    /// [`Invalid::ZeroDenominator`] when `den` is zero.
    pub const fn new(num: u64, den: u64) -> Result<Self, Invalid> {
        if den == 0 {
            return Err(Invalid::ZeroDenominator);
        }
        Ok(Self { num, den })
    }

    /// The numerator as given.
    #[must_use]
    pub const fn num(self) -> u64 {
        self.num
    }

    /// The denominator as given.
    #[must_use]
    pub const fn den(self) -> u64 {
        self.den
    }
}

impl PartialEq for Fraction {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Fraction {}

impl PartialOrd for Fraction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Fraction {
    fn cmp(&self, other: &Self) -> Ordering {
        let left = u128::from(self.num) * u128::from(other.den);
        let right = u128::from(other.num) * u128::from(self.den);
        left.cmp(&right)
    }
}

/// The three measured keys tie-breaks (a) to (c) read (SPEC 11.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TieKeys {
    /// (a) `|m|`, the move against the previous close (SPEC 5.1): larger first.
    pub strength: Fraction,
    /// (b) the stop's distance as a fraction of R, the ladder's range: smaller
    /// first.
    pub stop_in_r: Fraction,
    /// (c) the second the stock first entered the recipe's top N today:
    /// earlier first.
    pub first_in_top: i64,
}

/// The cap tie-break order, setting S81: one per account (SPEC 11.5, 14).
///
/// Every order falls through to (d) on a tie, then to the recipe's place, then
/// to the rest of the request, so two batches holding the same orders in any
/// arrangement are acted on in one order (D-8182). A procedure walks its shared
/// account under one order, its own S81 value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TieBreak {
    /// (a) the larger `|m|` first.
    Strength,
    /// (b) the smaller stop distance in R first.
    StopInR,
    /// (c) the earlier first entry into top N today first.
    FirstInTop,
    /// (d) symbol order, the neutral baseline.
    Symbol,
    /// (e) a seeded random order: the seed is one of the k the run generates.
    Seeded(u64),
}

/// Which capital the margin check reads (SPEC 11.3, OP-21; PARTS READINESS
/// ADDENDUM item 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capital {
    /// The headline: Rs 20 lakh at the window's first session plus every net
    /// since, worse clock, as a live account carries its losses (OP-21).
    Carried,
    /// The reading shown beside it: Rs 20 lakh again at every session.
    ResetDaily,
    /// No margin check: the walk item 5 runs once, whose rows serve every window
    /// start only when [`CapitalBound::holds`] says so after it.
    Unchecked,
}

/// One entry order, as its caller sizes it at the order's start (SPEC 9.2, 11.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryRequest {
    /// The stock it would lock.
    pub stock: StockId,
    /// The recipe's place in the walk: 0 for a recipe alone, its pick position
    /// in a procedure (SPEC 11.6).
    pub recipe: u32,
    /// The order's start second, the first traded second at or after `T + λ`:
    /// tie-break rule 1 (SPEC 11.5).
    pub start: i64,
    /// FNO-NET's quantity, fixed at the start (SPEC 9.2), at least one.
    pub qty: u64,
    /// `P_s`, the sizing price in paisa: the last traded GDFL price at or before
    /// the start (SPEC 11.2, OP-15).
    pub price: i64,
    /// `M / 100` in parts per million: the `VaR` + ELM percent of NSE's margin
    /// file, the largest of the readings the quantity was sized under (SPEC
    /// 11.2, 11.3). The rate applied is `max(MARGIN_FLOOR_PPM, m_ppm)`.
    pub m_ppm: u32,
    /// The keys tie-breaks (a) to (c) read.
    pub keys: TieKeys,
}

/// What [`Account::admit`] did with an entry (PARTS FNO-PORTFOLIO).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admission {
    /// Sent: a slot is taken, the stock locked and the trade counted.
    Admitted {
        /// The position.
        slot: SlotId,
    },
    /// Not sent, and counted under its reason.
    Refused(Refusal),
    /// Not sent yet, because an order-rate ring is full: nothing changed. Offer
    /// it again at this second, when every check runs again. `i64::MAX` when
    /// the session's orders are spent: no second of it allows another.
    Queued(i64),
}

impl Admission {
    /// The position, when admitted.
    #[must_use]
    pub const fn slot(self) -> Option<SlotId> {
        match self {
            Self::Admitted { slot } => Some(slot),
            Self::Refused(_) | Self::Queued(_) => None,
        }
    }
}

/// Why an entry was not sent, each counted (SPEC 9.6, 11.3, 11.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// All twenty slots are taken.
    PositionCap,
    /// The session's 100 entries are sent.
    TradeCap,
    /// The exposure with this order's `q·P_s` would pass Rs 1 crore.
    ExposureCap,
    /// The margin in use with this order's would pass the capital.
    MarginCap,
    /// The stock is locked by a position until after its last fill second.
    WhileOpen,
}

impl Refusal {
    /// The name SPEC 9.6 and 11.3 to 11.4 count it under.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::PositionCap => "position_cap",
            Self::TradeCap => "trade_cap",
            Self::ExposureCap => "exposure_cap",
            Self::MarginCap => "margin_cap",
            Self::WhileOpen => "while_open",
        }
    }
}

/// What happened to an order the rings rule on alone: an exit, or a
/// replacement's send.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dispatch {
    /// Sent, and counted in every ring.
    Sent,
    /// Waits for this second, `i64::MAX` when the session's orders are spent.
    Queued(i64),
}

/// The resting orders a position may modify (SPEC 9.1, 9.4): each has its own
/// count of [`MODIFICATIONS_PER_ORDER`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderRole {
    /// A resting stop-entry at the mapped box edge.
    StopEntry,
    /// The protective stop, a break-even move included.
    Stop,
    /// A resting target.
    Target,
    /// A trailing stop: each step of its level is a modification.
    TrailingStop,
    /// A trailing take-profit, once armed.
    TrailingTakeProfit,
}

impl OrderRole {
    /// Its row in a position's modification counts.
    const fn index(self) -> usize {
        match self {
            Self::StopEntry => 0,
            Self::Stop => 1,
            Self::Target => 2,
            Self::TrailingStop => 3,
            Self::TrailingTakeProfit => 4,
        }
    }
}

/// What [`Account::modify`] did (SPEC 9.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modify {
    /// Modified in place: the order has now taken `count` of its 25.
    Modified {
        /// Modifications the order has taken, this one included.
        count: u8,
    },
    /// The change after the 25th: the order was cancelled and replaced by a new
    /// one carrying it. That costs no charge and counts as an order in the rings.
    Replaced,
    /// The replacement waits for this second, `i64::MAX` when the session's
    /// orders are spent; until it goes the order keeps its old level.
    Queued(i64),
}

/// A call the account refuses whole, changing nothing: the caller broke its
/// contract (CLAUDE.md section 4: degrade loudly, never silently).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Invalid {
    /// A second earlier than one the account has already acted at.
    TimeBackwards {
        /// The second asked.
        at: i64,
        /// The latest second the account acted at.
        clock: i64,
    },
    /// An order offered before its own start second.
    BeforeStart {
        /// The second asked.
        at: i64,
        /// The order's start.
        start: i64,
    },
    /// A quantity of zero (FNO-NET refuses `q = 0` before this).
    ZeroQuantity,
    /// A price of zero or less.
    NonPositivePrice(i64),
    /// A fraction with a zero denominator.
    ZeroDenominator,
    /// A position value, quantity times price, past `i64::MAX` paisa.
    ValueOverflow,
    /// The slot id names no position held now.
    StaleSlot,
    /// A quantity larger than the position holds: it is fixed at the order's
    /// start (SPEC 9.2) and only falls.
    QuantityGrows {
        /// The quantity held.
        held: u64,
        /// The quantity asked.
        asked: u64,
    },
    /// A last fill earlier than the second the entry was sent.
    ReleaseBeforeSend {
        /// The last fill second given.
        last_fill: i64,
        /// The second the entry was sent.
        sent: i64,
    },
    /// A session or a window begun while positions are held or not yet past
    /// their last fill second.
    OpenPositions(usize),
}

/// A margin amount, exact: paisa times one million, the sum of value times the
/// rate in parts per million (SPEC 11.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Margin(i128);

impl Margin {
    /// The exact amount, in millionths of a paisa.
    #[must_use]
    pub const fn micro_paisa(self) -> i128 {
        self.0
    }

    /// Rounded down to the paisa: the MTM rule's 80% of it then fires no later,
    /// the stricter reading OP-33 takes (FNO-SEC-EXEC reads it).
    #[must_use]
    pub const fn floor_paisa(self) -> i128 {
        self.0.div_euclid(PPM)
    }

    /// Rounded up to the paisa.
    #[must_use]
    pub const fn ceil_paisa(self) -> i128 {
        -(-self.0).div_euclid(PPM)
    }
}

/// Every outcome the account counted since it was made or its window began
/// (SPEC 13.2's refusals by reason and cap hits).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Entries sent.
    pub admitted: u64,
    /// Offers of an entry that queued, each offer once.
    pub queued: u64,
    /// Entries refused for the stock's lock.
    pub while_open: u64,
    /// Entries refused with every slot taken.
    pub position_cap: u64,
    /// Entries refused with the session's 100 sent.
    pub trade_cap: u64,
    /// Entries refused for exposure.
    pub exposure_cap: u64,
    /// Entries refused for margin.
    pub margin_cap: u64,
    /// Exit orders sent.
    pub exits_sent: u64,
    /// Offers of an exit that queued.
    pub exits_queued: u64,
    /// Modifications made in place.
    pub modified: u64,
    /// Orders replaced after their 25th modification.
    pub replaced: u64,
    /// Offers of a replacement that queued.
    pub replacements_queued: u64,
    /// Positions released.
    pub released: u64,
}

/// What the account saw of its own capital (PARTS READINESS ADDENDUM item 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapitalBound {
    /// The largest margin in use right after an admission, the only moment a
    /// margin check reads it.
    pub peak_margin: Margin,
    /// The largest fall of the carried net from a running peak, the walk's
    /// start counted as a peak of zero, in paisa.
    pub largest_fall: i128,
    /// Entries refused for margin.
    pub margin_refusals: u64,
}

impl CapitalBound {
    /// Does this walk trade the same under every window start?
    ///
    /// Quantity never reads capital (SPEC 11.1 to 11.2) and every other check
    /// reads none, so only the margin check can tell two window starts apart.
    /// With no margin refusal, margin in use at any admission is at most the
    /// peak, and the carried net since any window start has fallen by at most
    /// the largest fall, so when the two sum to Rs 20 lakh or less no window
    /// start refuses either. A walk with a margin refusal never qualifies: the
    /// refused entry is missing from the margin it saw (D-8183).
    #[must_use]
    pub fn holds(self) -> bool {
        let used = self.peak_margin.0 + self.largest_fall * PPM;
        self.margin_refusals == 0 && used <= i128::from(CAPITAL_PAISA) * PPM
    }
}

/// One exit order of a held position, sent before every entry of its second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExitIntent {
    /// The position.
    pub slot: SlotId,
    /// The exit order's start second: tie-break rule 1 orders exits by it too.
    pub start: i64,
}

/// One order that wants to go in a second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    /// An exit order.
    Exit(ExitIntent),
    /// An entry order.
    Entry(EntryRequest),
}

/// What [`Account::schedule_second`] did with one intent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The exit's dispatch.
    Exit(Dispatch),
    /// The entry's admission.
    Entry(Admission),
}

/// A held position. `value` is quantity times the last marked price and stays
/// within `i64` paisa: admission and every mark refuse a larger one.
#[derive(Clone, Copy, Debug)]
struct Position {
    ticket: u64,
    stock: StockId,
    recipe: u32,
    sent: i64,
    qty: u64,
    price: i64,
    rate_ppm: u32,
    value: i128,
    keys: TieKeys,
    mods: [u8; ROLES],
}

impl Position {
    /// Its margin, exact, in millionths of a paisa.
    fn margin(&self) -> i128 {
        self.value * i128::from(self.rate_ppm)
    }

    /// Modifications the order in `role` has taken.
    #[expect(
        clippy::indexing_slicing,
        reason = "OrderRole::index is below ROLES, the array's length"
    )]
    const fn mods(&self, role: OrderRole) -> u8 {
        self.mods[role.index()]
    }

    /// Sets the modifications the order in `role` has taken.
    #[expect(
        clippy::indexing_slicing,
        reason = "OrderRole::index is below ROLES, the array's length"
    )]
    const fn set_mods(&mut self, role: OrderRole, count: u8) {
        self.mods[role.index()] = count;
    }
}

/// One row of the slot table.
#[derive(Clone, Copy, Debug)]
enum Slot {
    /// Nobody holds it.
    Free,
    /// A position holds it.
    Held(Position),
    /// Released at `after`, its last fill second: in force through it, settled
    /// at the first call of a later second.
    Closing {
        position: Position,
        after: i64,
        net: i64,
    },
}

impl Slot {
    /// The position, while it is held.
    const fn held(&self) -> Option<&Position> {
        match self {
            Self::Held(position) => Some(position),
            Self::Free | Self::Closing { .. } => None,
        }
    }
}

/// The last `N` send seconds, oldest first from `head`: a sliding window of
/// `N` orders.
#[derive(Clone, Debug)]
struct Ring<const N: usize> {
    sent: [i64; N],
    head: usize,
    len: usize,
}

impl<const N: usize> Ring<N> {
    /// An empty ring.
    const fn new() -> Self {
        Self {
            sent: [0; N],
            head: 0,
            len: 0,
        }
    }

    /// The second this ring next allows an order, when it allows none at `at`:
    /// full, with its oldest send less than `window` seconds before `at`.
    #[expect(
        clippy::indexing_slicing,
        reason = "head is only ever set mod N, so it is below the array's length"
    )]
    fn blocks(&self, at: i64, window: i64) -> Option<i64> {
        if self.len < N {
            return None;
        }
        let next = self.sent[self.head].saturating_add(window);
        (next > at).then_some(next)
    }

    /// Records a send at `at`, dropping the oldest when full.
    #[expect(
        clippy::indexing_slicing,
        reason = "the tail is taken mod N, so it is below the array's length"
    )]
    fn push(&mut self, at: i64) {
        self.sent[(self.head + self.len) % N] = at;
        if self.len < N {
            self.len += 1;
        } else {
            self.head = (self.head + 1) % N;
        }
    }
}

/// An entry's value and margin, from a request already checked.
#[derive(Clone, Copy, Debug)]
struct Proposed {
    value: i128,
    rate_ppm: u32,
}

impl Proposed {
    /// The value and rate of `request`; its value fits `i64` once
    /// [`check_entry`] passed.
    fn of(request: &EntryRequest) -> Self {
        Self {
            value: i128::from(request.qty) * i128::from(request.price),
            rate_ppm: request.m_ppm.max(MARGIN_FLOOR_PPM),
        }
    }

    /// Its margin, exact, in millionths of a paisa.
    fn margin(self) -> i128 {
        self.value * i128::from(self.rate_ppm)
    }
}

/// Refuses an entry its caller should never have offered at `at`.
fn check_entry(at: i64, request: &EntryRequest) -> Result<(), Invalid> {
    if request.qty == 0 {
        return Err(Invalid::ZeroQuantity);
    }
    if request.price <= 0 {
        return Err(Invalid::NonPositivePrice(request.price));
    }
    if at < request.start {
        return Err(Invalid::BeforeStart {
            at,
            start: request.start,
        });
    }
    if Proposed::of(request).value > i128::from(i64::MAX) {
        return Err(Invalid::ValueOverflow);
    }
    Ok(())
}

/// One account's money rules and order-rate rules (PARTS FNO-PORTFOLIO).
///
/// A window's walk is one account, from [`Account::new`] or
/// [`Account::begin_window`], and each of its sessions begins with
/// [`Account::begin_day`]. In a procedure every picked recipe shares one
/// account, so the caps and the stock locks are shared too (SPEC 11.6).
#[derive(Clone, Debug)]
pub struct Account {
    capital: Capital,
    tie_break: TieBreak,
    clock: i64,
    slots: [Slot; POSITION_SLOTS],
    locks: [Option<u8>; STOCKS],
    trades_today: u16,
    orders_today: u32,
    exposure: i128,
    margin: i128,
    carried: i128,
    tickets: u64,
    seconds: Ring<ORDERS_PER_SECOND>,
    minutes: Ring<ORDERS_PER_MINUTE>,
    hours: Ring<ORDERS_PER_HOUR>,
    peak_margin: i128,
    peak_net: i128,
    largest_fall: i128,
    counts: Counts,
}

impl Account {
    /// An account with no position, at Rs 20 lakh, its tie-break order set for
    /// the walk.
    #[must_use]
    pub const fn new(capital: Capital, tie_break: TieBreak) -> Self {
        Self {
            capital,
            tie_break,
            clock: i64::MIN,
            slots: [Slot::Free; POSITION_SLOTS],
            locks: [None; STOCKS],
            trades_today: 0,
            orders_today: 0,
            exposure: 0,
            margin: 0,
            carried: 0,
            tickets: 0,
            seconds: Ring::new(),
            minutes: Ring::new(),
            hours: Ring::new(),
            peak_margin: 0,
            peak_net: 0,
            largest_fall: 0,
            counts: Counts {
                admitted: 0,
                queued: 0,
                while_open: 0,
                position_cap: 0,
                trade_cap: 0,
                exposure_cap: 0,
                margin_cap: 0,
                exits_sent: 0,
                exits_queued: 0,
                modified: 0,
                replaced: 0,
                replacements_queued: 0,
                released: 0,
            },
        }
    }

    /// Starts a window at `at`, its first session: the account is as
    /// [`Account::new`] makes it, at Rs 20 lakh, with no history (PARTS
    /// READINESS ADDENDUM item 5). Only the ticket count carries on, so an id
    /// from the window before stays stale.
    ///
    /// # Errors
    ///
    /// [`Invalid::TimeBackwards`] for a second before the account's clock, and
    /// [`Invalid::OpenPositions`] while any position is held or not yet past its
    /// last fill second. Neither changes anything.
    pub fn begin_window(&mut self, at: i64) -> Result<(), Invalid> {
        self.check_flat(at)?;
        let tickets = self.tickets;
        *self = Self::new(self.capital, self.tie_break);
        self.tickets = tickets;
        self.clock = at;
        Ok(())
    }

    /// Starts a session at `at`: no entry and no order sent today yet, and under
    /// [`Capital::ResetDaily`] the capital back at Rs 20 lakh (SPEC 11.4,
    /// OP-21). The rings are not cleared: they slide, and a night empties them.
    ///
    /// # Errors
    ///
    /// As [`Account::begin_window`].
    pub fn begin_day(&mut self, at: i64) -> Result<(), Invalid> {
        self.check_flat(at)?;
        self.settle_through(at);
        self.clock = at;
        self.trades_today = 0;
        self.orders_today = 0;
        if self.capital == Capital::ResetDaily {
            self.carried = 0;
            self.peak_net = 0;
        }
        Ok(())
    }

    /// Settles at `at` every position whose last fill second is before it,
    /// freeing its slot and lock and carrying its net. Every timed call that
    /// reads the caps does this first; a caller reads a figure at a second with
    /// this.
    ///
    /// # Errors
    ///
    /// [`Invalid::TimeBackwards`] for a second before the account's clock.
    pub fn settle(&mut self, at: i64) -> Result<(), Invalid> {
        self.check_time(at)?;
        self.clock = at;
        self.settle_through(at);
        Ok(())
    }

    /// Admits, refuses or queues one entry at `at`, the second it would be sent
    /// (SPEC 9.6, 11.3, 11.4).
    ///
    /// # Errors
    ///
    /// [`Invalid::TimeBackwards`], [`Invalid::BeforeStart`],
    /// [`Invalid::ZeroQuantity`], [`Invalid::NonPositivePrice`] and
    /// [`Invalid::ValueOverflow`]; each changes nothing.
    pub fn admit(&mut self, at: i64, request: &EntryRequest) -> Result<Admission, Invalid> {
        self.check_time(at)?;
        check_entry(at, request)?;
        self.clock = at;
        self.settle_through(at);
        Ok(self.decide(at, request))
    }

    /// Sends or queues one exit order of a held position at `at` (SPEC 9.7).
    ///
    /// # Errors
    ///
    /// [`Invalid::TimeBackwards`] and [`Invalid::StaleSlot`]; each changes
    /// nothing.
    pub fn send_exit(&mut self, at: i64, slot: SlotId) -> Result<Dispatch, Invalid> {
        self.check_time(at)?;
        self.held(slot)?;
        self.clock = at;
        Ok(self.dispatch_exit(at))
    }

    /// Modifies one resting order of a held position at `at`: in place up to
    /// its 25th modification, then by a replacement that counts in the rings
    /// (SPEC 9.7). Only the replacement is an order; a modification in place
    /// takes no ring (D-8181).
    ///
    /// # Errors
    ///
    /// [`Invalid::TimeBackwards`] and [`Invalid::StaleSlot`]; each changes
    /// nothing.
    pub fn modify(&mut self, at: i64, slot: SlotId, role: OrderRole) -> Result<Modify, Invalid> {
        self.check_time(at)?;
        let (index, mut position) = self.held(slot)?;
        self.clock = at;
        let used = position.mods(role);
        if used < MODIFICATIONS_PER_ORDER {
            position.set_mods(role, used + 1);
            self.put(index, Slot::Held(position));
            self.counts.modified += 1;
            return Ok(Modify::Modified { count: used + 1 });
        }
        if let Some(next) = self.rate_blocks(at) {
            self.counts.replacements_queued += 1;
            return Ok(Modify::Queued(next));
        }
        self.record_send(at);
        position.set_mods(role, 0);
        self.put(index, Slot::Held(position));
        self.counts.replaced += 1;
        Ok(Modify::Replaced)
    }

    /// Marks a held position at its last traded price, in paisa: its value, the
    /// exposure and the margin in use follow (SPEC 11.3). A mark never forces a
    /// reduction; it only changes what the next entry is checked against.
    ///
    /// # Errors
    ///
    /// [`Invalid::StaleSlot`], [`Invalid::NonPositivePrice`] and
    /// [`Invalid::ValueOverflow`]; each changes nothing.
    pub fn mark(&mut self, slot: SlotId, price: i64) -> Result<(), Invalid> {
        let (index, position) = self.held(slot)?;
        if price <= 0 {
            return Err(Invalid::NonPositivePrice(price));
        }
        let value = i128::from(position.qty) * i128::from(price);
        if value > i128::from(i64::MAX) {
            return Err(Invalid::ValueOverflow);
        }
        self.revalue(index, position, position.qty, price, value);
        Ok(())
    }

    /// Sets a held position's quantity: an entry cut after a partial fill
    /// (SPEC 9.5), or an exit's fills. It only falls.
    ///
    /// # Errors
    ///
    /// [`Invalid::StaleSlot`], [`Invalid::ZeroQuantity`] (release a closed
    /// position instead) and [`Invalid::QuantityGrows`]; each changes nothing.
    pub fn resize(&mut self, slot: SlotId, qty: u64) -> Result<(), Invalid> {
        let (index, position) = self.held(slot)?;
        if qty == 0 {
            return Err(Invalid::ZeroQuantity);
        }
        if qty > position.qty {
            return Err(Invalid::QuantityGrows {
                held: position.qty,
                asked: qty,
            });
        }
        let value = i128::from(qty) * i128::from(position.price);
        self.revalue(index, position, qty, position.price, value);
        Ok(())
    }

    /// Releases a position at `last_fill`, its exit's last fill second or the
    /// second a zero-fill entry was cancelled, with its net under the worse
    /// clock, in paisa (SPEC 11.3, 11.4, OP-19, OP-21). It stays in force
    /// through `last_fill`; a call at a later second settles it.
    ///
    /// # Errors
    ///
    /// [`Invalid::StaleSlot`] and [`Invalid::ReleaseBeforeSend`]; each changes
    /// nothing.
    pub fn release(&mut self, slot: SlotId, last_fill: i64, net: i64) -> Result<(), Invalid> {
        let (index, position) = self.held(slot)?;
        if last_fill < position.sent {
            return Err(Invalid::ReleaseBeforeSend {
                last_fill,
                sent: position.sent,
            });
        }
        self.put(
            index,
            Slot::Closing {
                position,
                after: last_fill,
                net,
            },
        );
        self.counts.released += 1;
        Ok(())
    }

    /// Acts on every order that wants to go at `at`, exits before entries, each
    /// group by start second and then by the account's tie-break order (SPEC
    /// 9.7, 11.5, OP-43). `intents` is left in that order and `out` holds one
    /// outcome per intent, in the same order.
    ///
    /// # Errors
    ///
    /// Any error [`Account::admit`] or [`Account::send_exit`] gives for one of
    /// the intents. The batch is checked whole first, so an error changes
    /// nothing, `out` included.
    pub fn schedule_second(
        &mut self,
        at: i64,
        intents: &mut [Intent],
        out: &mut Vec<Outcome>,
    ) -> Result<(), Invalid> {
        self.check_time(at)?;
        for intent in intents.iter() {
            self.check_intent(at, intent)?;
        }
        let order = self.tie_break;
        let slots = &self.slots;
        intents.sort_unstable_by(|a, b| compare(order, slots, a, b));
        out.clear();
        self.clock = at;
        self.settle_through(at);
        for intent in intents.iter() {
            out.push(match intent {
                Intent::Exit(_) => Outcome::Exit(self.dispatch_exit(at)),
                Intent::Entry(request) => Outcome::Entry(self.decide(at, request)),
            });
        }
        Ok(())
    }

    /// The latest second the account acted at.
    #[must_use]
    pub const fn clock(&self) -> i64 {
        self.clock
    }

    /// The capital the margin check reads now: Rs 20 lakh plus the settled
    /// carried net, in paisa.
    #[must_use]
    pub fn capital(&self) -> i128 {
        i128::from(CAPITAL_PAISA) + self.carried
    }

    /// The settled net carried since the window began (or the session, under
    /// [`Capital::ResetDaily`]), in paisa.
    #[must_use]
    pub const fn carried_net(&self) -> i128 {
        self.carried
    }

    /// The open and pending positions' value at their last marks, in paisa.
    #[must_use]
    pub const fn exposure(&self) -> i128 {
        self.exposure
    }

    /// The margin those positions block, exact.
    #[must_use]
    pub const fn margin_in_use(&self) -> Margin {
        Margin(self.margin)
    }

    /// The margin one held position blocks at its last mark, exact: the base of
    /// the MTM rule (SPEC 9.8, OP-33).
    ///
    /// # Errors
    ///
    /// [`Invalid::StaleSlot`].
    pub fn margin(&self, slot: SlotId) -> Result<Margin, Invalid> {
        Ok(Margin(self.held(slot)?.1.margin()))
    }

    /// Slots held, or released and not yet past their last fill second.
    #[must_use]
    pub fn slots_taken(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| !matches!(slot, Slot::Free))
            .count()
    }

    /// Entries sent this session.
    #[must_use]
    pub const fn trades_today(&self) -> u16 {
        self.trades_today
    }

    /// Orders sent this session: entries, exits and replacements.
    #[must_use]
    pub const fn orders_today(&self) -> u32 {
        self.orders_today
    }

    /// Every outcome counted since the account or its window began.
    #[must_use]
    pub const fn counts(&self) -> Counts {
        self.counts
    }

    /// What the account saw of its own capital, for the window-start proof.
    #[must_use]
    pub const fn capital_bound(&self) -> CapitalBound {
        CapitalBound {
            peak_margin: Margin(self.peak_margin),
            largest_fall: self.largest_fall,
            margin_refusals: self.counts.margin_cap,
        }
    }

    /// Refuses a second before the clock.
    const fn check_time(&self, at: i64) -> Result<(), Invalid> {
        if at < self.clock {
            return Err(Invalid::TimeBackwards {
                at,
                clock: self.clock,
            });
        }
        Ok(())
    }

    /// Refuses a session or window start before the clock or with a position
    /// still in force at `at`.
    fn check_flat(&self, at: i64) -> Result<(), Invalid> {
        self.check_time(at)?;
        let open = self
            .slots
            .iter()
            .filter(|slot| match slot {
                Slot::Free => false,
                Slot::Held(_) => true,
                Slot::Closing { after, .. } => *after >= at,
            })
            .count();
        if open > 0 {
            return Err(Invalid::OpenPositions(open));
        }
        Ok(())
    }

    /// Refuses an intent [`Account::schedule_second`] should never have been
    /// offered at `at`.
    fn check_intent(&self, at: i64, intent: &Intent) -> Result<(), Invalid> {
        match intent {
            Intent::Exit(exit) => {
                self.held(exit.slot)?;
                if at < exit.start {
                    return Err(Invalid::BeforeStart {
                        at,
                        start: exit.start,
                    });
                }
                Ok(())
            }
            Intent::Entry(request) => check_entry(at, request),
        }
    }

    /// The slot and position `slot` names, if it is held now.
    fn held(&self, slot: SlotId) -> Result<(usize, Position), Invalid> {
        let index = usize::from(slot.index);
        self.slots
            .get(index)
            .and_then(Slot::held)
            .filter(|position| position.ticket == slot.ticket)
            .map(|position| (index, *position))
            .ok_or(Invalid::StaleSlot)
    }

    /// Writes one row of the slot table.
    #[expect(
        clippy::indexing_slicing,
        reason = "every index here came from `held`, `free_slot` or a loop below POSITION_SLOTS"
    )]
    const fn put(&mut self, index: usize, slot: Slot) {
        self.slots[index] = slot;
    }

    /// The slot holding `stock`'s lock.
    #[expect(
        clippy::indexing_slicing,
        reason = "a u8 position is below STOCKS, the table's length"
    )]
    fn lock(&self, stock: StockId) -> Option<u8> {
        self.locks[usize::from(stock.0)]
    }

    /// Sets `stock`'s lock.
    #[expect(
        clippy::indexing_slicing,
        reason = "a u8 position is below STOCKS, the table's length"
    )]
    fn set_lock(&mut self, stock: StockId, slot: Option<u8>) {
        self.locks[usize::from(stock.0)] = slot;
    }

    /// The first free slot, lowest index first.
    fn free_slot(&self) -> Option<u8> {
        self.slots
            .iter()
            .zip(0_u8..)
            .find_map(|(slot, index)| matches!(slot, Slot::Free).then_some(index))
    }

    /// Settles every position released before `at`.
    fn settle_through(&mut self, at: i64) {
        for index in 0..POSITION_SLOTS {
            let Some((position, net)) = self.closing_before(index, at) else {
                continue;
            };
            self.put(index, Slot::Free);
            self.set_lock(position.stock, None);
            self.exposure -= position.value;
            self.margin -= position.margin();
            self.carried += i128::from(net);
            self.peak_net = self.peak_net.max(self.carried);
            self.largest_fall = self.largest_fall.max(self.peak_net - self.carried);
        }
    }

    /// The position in slot `index` and its net, if it was released before `at`.
    fn closing_before(&self, index: usize, at: i64) -> Option<(Position, i64)> {
        match self.slots.get(index) {
            Some(Slot::Closing {
                position,
                after,
                net,
            }) if *after < at => Some((*position, *net)),
            _ => None,
        }
    }

    /// Replaces a held position's quantity, price and value, the sums with them.
    fn revalue(&mut self, index: usize, mut position: Position, qty: u64, price: i64, value: i128) {
        let delta = value - position.value;
        self.exposure += delta;
        self.margin += delta * i128::from(position.rate_ppm);
        position.qty = qty;
        position.price = price;
        position.value = value;
        self.put(index, Slot::Held(position));
    }

    /// The checks of one entry already checked for its caller's contract, in
    /// order, and the admission if it passes them all.
    fn decide(&mut self, at: i64, request: &EntryRequest) -> Admission {
        let proposed = Proposed::of(request);
        if self.lock(request.stock).is_some() {
            return self.refuse(Refusal::WhileOpen);
        }
        let Some(index) = self.free_slot() else {
            return self.refuse(Refusal::PositionCap);
        };
        if self.trades_today >= TRADES_PER_DAY {
            return self.refuse(Refusal::TradeCap);
        }
        if self.exposure + proposed.value > i128::from(EXPOSURE_CAP_PAISA) {
            return self.refuse(Refusal::ExposureCap);
        }
        if self.margin_binds(proposed.margin()) {
            return self.refuse(Refusal::MarginCap);
        }
        if let Some(next) = self.rate_blocks(at) {
            self.counts.queued += 1;
            return Admission::Queued(next);
        }
        self.tickets += 1;
        let position = Position {
            ticket: self.tickets,
            stock: request.stock,
            recipe: request.recipe,
            sent: at,
            qty: request.qty,
            price: request.price,
            rate_ppm: proposed.rate_ppm,
            value: proposed.value,
            keys: request.keys,
            mods: [0; ROLES],
        };
        let slot = SlotId {
            index,
            ticket: self.tickets,
        };
        self.put(usize::from(index), Slot::Held(position));
        self.set_lock(request.stock, Some(index));
        self.trades_today += 1;
        self.exposure += proposed.value;
        self.margin += proposed.margin();
        self.peak_margin = self.peak_margin.max(self.margin);
        self.record_send(at);
        self.counts.admitted += 1;
        Admission::Admitted { slot }
    }

    /// Would `extra` more margin pass the capital the account reads?
    fn margin_binds(&self, extra: i128) -> bool {
        match self.capital {
            Capital::Unchecked => false,
            Capital::Carried | Capital::ResetDaily => self.margin + extra > self.capital() * PPM,
        }
    }

    /// Counts a refusal under its name and returns it.
    const fn refuse(&mut self, refusal: Refusal) -> Admission {
        let count = match refusal {
            Refusal::PositionCap => &mut self.counts.position_cap,
            Refusal::TradeCap => &mut self.counts.trade_cap,
            Refusal::ExposureCap => &mut self.counts.exposure_cap,
            Refusal::MarginCap => &mut self.counts.margin_cap,
            Refusal::WhileOpen => &mut self.counts.while_open,
        };
        *count += 1;
        Admission::Refused(refusal)
    }

    /// The second the rings next allow an order, when they allow none at `at`:
    /// the latest of the full rings' expiries, `i64::MAX` when the session's
    /// orders are spent (D-8181).
    fn rate_blocks(&self, at: i64) -> Option<i64> {
        let day = (self.orders_today >= ORDERS_PER_DAY).then_some(i64::MAX);
        let second = self.seconds.blocks(at, 1);
        let minute = self.minutes.blocks(at, 60);
        let hour = self.hours.blocks(at, 3_600);
        day.max(second).max(minute).max(hour)
    }

    /// Counts one order sent at `at` in every ring and in the session.
    fn record_send(&mut self, at: i64) {
        self.seconds.push(at);
        self.minutes.push(at);
        self.hours.push(at);
        self.orders_today += 1;
    }

    /// Sends or queues one exit order at `at`.
    fn dispatch_exit(&mut self, at: i64) -> Dispatch {
        if let Some(next) = self.rate_blocks(at) {
            self.counts.exits_queued += 1;
            return Dispatch::Queued(next);
        }
        self.record_send(at);
        self.counts.exits_sent += 1;
        Dispatch::Sent
    }
}

/// What two intents are ordered by, the position's own for an exit.
#[derive(Clone, Copy, Debug)]
struct Key {
    start: i64,
    stock: StockId,
    recipe: u32,
    keys: TieKeys,
}

/// The key of a held position's exit; `None` only for an id no checked batch
/// holds.
fn exit_key(slots: &[Slot; POSITION_SLOTS], exit: &ExitIntent) -> Option<Key> {
    let position = slots
        .get(usize::from(exit.slot.index))
        .and_then(Slot::held)?;
    Some(Key {
        start: exit.start,
        stock: position.stock,
        recipe: position.recipe,
        keys: position.keys,
    })
}

/// The order of one second's intents: exits first, each group by start second,
/// then by `order`, then by symbol, then by recipe, then by the rest of the
/// intent, so the order is total (D-8182).
fn compare(order: TieBreak, slots: &[Slot; POSITION_SLOTS], a: &Intent, b: &Intent) -> Ordering {
    match (a, b) {
        (Intent::Exit(_), Intent::Entry(_)) => Ordering::Less,
        (Intent::Entry(_), Intent::Exit(_)) => Ordering::Greater,
        (Intent::Exit(x), Intent::Exit(y)) => exit_key(slots, x)
            .zip(exit_key(slots, y))
            .map_or(Ordering::Equal, |(kx, ky)| by_key(order, &kx, &ky))
            .then((x.slot.index, x.slot.ticket).cmp(&(y.slot.index, y.slot.ticket))),
        (Intent::Entry(x), Intent::Entry(y)) => {
            by_key(order, &entry_key(x), &entry_key(y)).then_with(|| rest(x).cmp(&rest(y)))
        }
    }
}

/// An entry's key.
const fn entry_key(request: &EntryRequest) -> Key {
    Key {
        start: request.start,
        stock: request.stock,
        recipe: request.recipe,
        keys: request.keys,
    }
}

/// Two keys by start, then `order`, then symbol, then recipe.
fn by_key(order: TieBreak, a: &Key, b: &Key) -> Ordering {
    let by_order = match order {
        TieBreak::Strength => b.keys.strength.cmp(&a.keys.strength),
        TieBreak::StopInR => a.keys.stop_in_r.cmp(&b.keys.stop_in_r),
        TieBreak::FirstInTop => a.keys.first_in_top.cmp(&b.keys.first_in_top),
        TieBreak::Symbol => Ordering::Equal,
        TieBreak::Seeded(seed) => seeded(seed, a).cmp(&seeded(seed, b)),
    };
    a.start
        .cmp(&b.start)
        .then(by_order)
        .then(a.stock.cmp(&b.stock))
        .then(a.recipe.cmp(&b.recipe))
}

/// The rest of an entry, so two that tie on every key are told apart by what
/// they are, never by where they sat in the batch.
fn rest(request: &EntryRequest) -> [u64; 8] {
    [
        request.qty,
        request.price.cast_unsigned(),
        u64::from(request.m_ppm),
        request.keys.strength.num,
        request.keys.strength.den,
        request.keys.stop_in_r.num,
        request.keys.stop_in_r.den,
        request.keys.first_in_top.cast_unsigned(),
    ]
}

/// Tie-break (e)'s key: the order's start, stock and recipe mixed with the
/// seed, so one seed gives one order whatever the batch's arrangement, and each
/// second draws afresh (D-8182).
fn seeded(seed: u64, key: &Key) -> u64 {
    let mut state = mix(seed);
    state = mix(state ^ key.start.cast_unsigned());
    state = mix(state ^ u64::from(key.stock.0));
    mix(state ^ u64::from(key.recipe))
}

/// `SplitMix64`'s step and finaliser, the generator `crate::bootstrap` draws
/// with: a fixed function of its input on every machine (CLAUDE.md section 3
/// rule 5).
const fn mix(state: u64) -> u64 {
    let mut z = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Timed calls of [`Account::admit`] and [`Account::release`] (C-FNO-PORT-01).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Samples {
    /// Nanoseconds of each timed admission, in call order.
    pub admit_ns: Vec<u64>,
    /// Nanoseconds of each timed release, in call order.
    pub release_ns: Vec<u64>,
    /// Entries the fixture saw admitted, timed or not. A run that admits fewer
    /// than every entry it offered was not timing this path.
    pub admitted: u64,
}

/// Cycles one measured session runs after its first twenty admissions:
/// 20 + 80 sends the session's 100 entries exactly.
const CYCLES: i64 = 80;

/// Seconds between two cycles: 40, so a session's 100 sends stay inside every
/// ring and only the 100-entry cap is reached.
const CYCLE_SECONDS: i64 = 40;

/// Times admit and release with the other nineteen slots held, over
/// `measured_days` sessions after `history_days` untimed ones (C-FNO-PORT-01).
///
/// Each session fills the twenty slots, then for 80 cycles releases the oldest
/// position at the second before and admits a new stock into its slot, so each
/// timed admission settles one release and finds the one free slot among
/// twenty, and the session ends at its 100-entry cap. The history is what the
/// bench scales: a call's work must not grow with it.
///
/// # Errors
///
/// Any [`Invalid`] a call returns, which a correct fixture never meets;
/// [`Invalid::StaleSlot`] when an entry it offered was not admitted.
pub fn measure_admit_release(history_days: u32, measured_days: u32) -> Result<Samples, Invalid> {
    let mut account = Account::new(Capital::Carried, TieBreak::Symbol);
    let mut samples = Samples::default();
    let mut next: u8 = 0;
    for day in 0..history_days.saturating_add(measured_days) {
        let timed = day >= history_days;
        let open = i64::from(day) * 86_400;
        account.begin_day(open)?;
        let mut held = [NO_SLOT; POSITION_SLOTS];
        let mut at = open;
        for cell in &mut held {
            *cell = account
                .admit(at, &bench_entry(next, at))?
                .slot()
                .unwrap_or(NO_SLOT);
            next = next.wrapping_add(1);
            at += 1;
        }
        for cycle in 0..CYCLES {
            at = open + 20 + CYCLE_SECONDS * cycle;
            let oldest = held[0];
            let began = Instant::now();
            let released = account.release(oldest, at - 1, 400 - 700 * (cycle % 2));
            let release_ns = elapsed_ns(began);
            released?;
            let entry = bench_entry(next, at);
            let began = Instant::now();
            let admitted = account.admit(at, &entry);
            let admit_ns = elapsed_ns(began);
            held.rotate_left(1);
            held[POSITION_SLOTS - 1] = admitted?.slot().unwrap_or(NO_SLOT);
            next = next.wrapping_add(1);
            if timed {
                samples.release_ns.push(release_ns);
                samples.admit_ns.push(admit_ns);
            }
        }
        for slot in held {
            account.release(slot, at, 0)?;
        }
    }
    samples.admitted = account.tickets;
    Ok(samples)
}

/// The bench's entry: Rs 1 lakh of a stock at Rs 1,000, at the margin floor.
const fn bench_entry(stock: u8, at: i64) -> EntryRequest {
    EntryRequest {
        stock: StockId(stock),
        recipe: 0,
        start: at,
        qty: 100,
        price: 100_000,
        m_ppm: MARGIN_FLOOR_PPM,
        keys: TieKeys {
            strength: Fraction { num: 1, den: 100 },
            stop_in_r: Fraction { num: 1, den: 4 },
            first_in_top: 0,
        },
    }
}

/// Nanoseconds since `began`, saturated.
fn elapsed_ns(began: Instant) -> u64 {
    u64::try_from(began.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "the exception every test module in this workspace takes: a test that cannot \
              panic cannot fail"
)]
mod tests {
    use super::{
        Account, Admission, CAPITAL_PAISA, Capital, Dispatch, EXPOSURE_CAP_PAISA, EntryRequest,
        ExitIntent, Fraction, Intent, Invalid, Key, MARGIN_FLOOR_PPM, MODIFICATIONS_PER_ORDER,
        Margin, Modify, ORDERS_PER_DAY, ORDERS_PER_HOUR, ORDERS_PER_MINUTE, ORDERS_PER_SECOND,
        OrderRole, Outcome, POSITION_SLOTS, Position, Refusal, STOCKS, Slot, SlotId, StockId,
        TRADES_PER_DAY, TieBreak, TieKeys, measure_admit_release, mix, seeded,
    };

    /// One day in seconds: every ring has emptied a day later.
    const DAY: i64 = 86_400;

    /// A margin's scale: millionths of a paisa.
    const MICRO: i128 = 1_000_000;

    fn frac(num: u64, den: u64) -> Fraction {
        Fraction::new(num, den).expect("a positive denominator")
    }

    fn keys(strength: (u64, u64), stop: (u64, u64), first_in_top: i64) -> TieKeys {
        TieKeys {
            strength: frac(strength.0, strength.1),
            stop_in_r: frac(stop.0, stop.1),
            first_in_top,
        }
    }

    /// Rs 1 lakh of `stock` at Rs 1,000, offered at `start`, at the 20% floor.
    fn entry(stock: u8, start: i64) -> EntryRequest {
        EntryRequest {
            stock: StockId::new(stock),
            recipe: 0,
            start,
            qty: 100,
            price: 100_000,
            m_ppm: 0,
            keys: keys((1, 100), (1, 4), 0),
        }
    }

    /// `qty` of `stock` at `price` paisa.
    fn worth(stock: u8, start: i64, qty: u64, price: i64) -> EntryRequest {
        EntryRequest {
            qty,
            price,
            ..entry(stock, start)
        }
    }

    /// A carried account with its first session begun at second 0.
    fn carried() -> Account {
        let mut account = Account::new(Capital::Carried, TieBreak::Symbol);
        account.begin_day(0).expect("a first session");
        account
    }

    /// The position `request` takes at `at`.
    fn take(account: &mut Account, at: i64, request: &EntryRequest) -> SlotId {
        account
            .admit(at, request)
            .expect("a valid entry")
            .slot()
            .expect("admitted")
    }

    /// The position a slot holds or is closing, for the consistency checks.
    fn position_of(slot: &Slot) -> Option<&Position> {
        match slot {
            Slot::Held(position) | Slot::Closing { position, .. } => Some(position),
            Slot::Free => None,
        }
    }

    /// The entry admitted in an outcome.
    fn admitted_in(outcome: Outcome) -> Option<SlotId> {
        match outcome {
            Outcome::Entry(admission) => admission.slot(),
            Outcome::Exit(_) => None,
        }
    }

    /// Every invariant the account holds after any call: slots, the day's
    /// trades and orders under their caps, one lock per taken slot naming its
    /// stock (so no stock is locked twice), the sums equal to the slots', and,
    /// with no mark, the exposure under its cap.
    fn assert_consistent(account: &Account, marked: bool) {
        assert!(account.slots_taken() <= POSITION_SLOTS);
        assert!(account.trades_today() <= TRADES_PER_DAY);
        assert!(account.orders_today() <= ORDERS_PER_DAY);
        let mut locked = 0;
        for (stock, lock) in account.locks.iter().enumerate() {
            let Some(index) = *lock else { continue };
            let position =
                position_of(&account.slots[usize::from(index)]).expect("a lock names a taken slot");
            assert_eq!(usize::from(position.stock.position()), stock);
            locked += 1;
        }
        assert_eq!(locked, account.slots_taken(), "one lock per taken slot");
        let (mut value, mut margin) = (0_i128, 0_i128);
        for position in account.slots.iter().filter_map(position_of) {
            value += position.value;
            margin += position.margin();
        }
        assert_eq!(account.exposure(), value);
        assert_eq!(account.margin_in_use().micro_paisa(), margin);
        if !marked {
            assert!(account.exposure() <= i128::from(EXPOSURE_CAP_PAISA));
        }
    }

    #[test]
    fn the_twentieth_position_is_admitted_and_the_twenty_first_refused() {
        let mut account = carried();
        let held: Vec<SlotId> = (0..20_u8)
            .map(|stock| {
                take(
                    &mut account,
                    i64::from(stock),
                    &entry(stock, i64::from(stock)),
                )
            })
            .collect();
        assert_eq!(account.slots_taken(), POSITION_SLOTS);
        let mut indexes: Vec<u8> = held.iter().map(|slot| slot.index()).collect();
        indexes.sort_unstable();
        indexes.dedup();
        assert_eq!(
            indexes.len(),
            POSITION_SLOTS,
            "each position has its own slot"
        );
        assert_eq!(
            account.admit(20, &entry(20, 20)),
            Ok(Admission::Refused(Refusal::PositionCap))
        );
        account.release(held[5], 21, -500).expect("held");
        assert_eq!(
            account.admit(21, &entry(20, 21)),
            Ok(Admission::Refused(Refusal::PositionCap)),
            "a released position keeps its slot through its last fill second"
        );
        let next = take(&mut account, 22, &entry(20, 22));
        assert_eq!(next.index(), held[5].index());
        assert_eq!(account.trades_today(), 21);
        assert_eq!(account.counts().position_cap, 2);
        assert_consistent(&account, false);
    }

    #[test]
    fn the_hundredth_trade_is_admitted_and_the_hundred_and_first_refused_with_cancelled_entries_counted()
     {
        let mut account = carried();
        for k in 0..100_i64 {
            let at = 10 * k;
            let stock = u8::try_from(k).expect("below 256");
            let slot = take(&mut account, at, &entry(stock, at));
            // The first sixty are cancelled with nothing filled in their send
            // second; the rest fill and exit five seconds later.
            let (last_fill, net) = if k < 60 { (at, 0) } else { (at + 5, -100) };
            account.release(slot, last_fill, net).expect("held");
        }
        assert_eq!(account.trades_today(), TRADES_PER_DAY);
        assert_eq!(
            account.admit(1_000, &entry(150, 1_000)),
            Ok(Admission::Refused(Refusal::TradeCap))
        );
        assert_eq!(
            account.slots_taken(),
            0,
            "every slot was free: the day's cap refused it"
        );
        assert_eq!(account.counts().trade_cap, 1);
        assert_eq!(account.counts().admitted, 100);
        assert_eq!(account.counts().released, 100);
    }

    #[test]
    fn exposure_at_exactly_one_crore_is_admitted_and_one_paisa_more_refused() {
        let mut account = carried();
        for stock in 0..10_u8 {
            let at = i64::from(stock);
            // Rs 10 lakh each: the tenth brings the exposure to Rs 1 crore and
            // the margin at 20% to exactly the Rs 20 lakh of capital.
            take(&mut account, at, &worth(stock, at, 1_000, 100_000));
        }
        assert_eq!(account.exposure(), i128::from(EXPOSURE_CAP_PAISA));
        assert_eq!(
            account.margin_in_use().micro_paisa(),
            i128::from(CAPITAL_PAISA) * MICRO
        );
        assert_eq!(
            account.admit(10, &worth(10, 10, 1, 1)),
            Ok(Admission::Refused(Refusal::ExposureCap))
        );
        assert_eq!(account.counts().exposure_cap, 1);
        assert_eq!(account.counts().margin_cap, 0, "exposure is checked first");
    }

    #[test]
    fn margin_exhausted_by_carried_losses_is_refused() {
        let mut account = carried();
        let first = take(&mut account, 0, &entry(0, 0));
        account
            .release(first, 10, -(CAPITAL_PAISA - 10_000_000))
            .expect("held");
        assert_eq!(
            account.capital(),
            i128::from(CAPITAL_PAISA),
            "the loss is not in the capital through its last fill second"
        );
        account.settle(11).expect("forward");
        assert_eq!(account.capital(), 10_000_000, "Rs 1 lakh left");
        assert_eq!(
            account.carried_net(),
            -i128::from(CAPITAL_PAISA - 10_000_000)
        );
        // Rs 5 lakh at 20% blocks the Rs 1 lakh left exactly: admitted.
        let second = take(&mut account, 11, &worth(1, 11, 500, 100_000));
        assert_eq!(account.margin_in_use().micro_paisa(), 10_000_000 * MICRO);
        // Five paisa of value at 20% is one paisa of margin more: refused.
        assert_eq!(
            account.admit(12, &worth(2, 12, 5, 1)),
            Ok(Admission::Refused(Refusal::MarginCap))
        );
        account.release(second, 13, -10_000_000).expect("held");
        account.settle(14).expect("forward");
        assert_eq!(account.capital(), 0, "all of it lost");
        assert_eq!(
            account.admit(14, &worth(3, 14, 1, 1)),
            Ok(Admission::Refused(Refusal::MarginCap))
        );
        assert_eq!(account.counts().margin_cap, 2);
        assert_eq!(account.capital_bound().margin_refusals, 2);
    }

    #[test]
    fn one_trade_per_stock_across_recipes_in_procedure_mode() {
        let mut account = carried();
        let mut batch = [
            Intent::Entry(EntryRequest {
                recipe: 1,
                ..entry(7, 5)
            }),
            Intent::Entry(EntryRequest {
                recipe: 0,
                ..entry(7, 5)
            }),
            Intent::Entry(EntryRequest {
                recipe: 2,
                ..entry(8, 5)
            }),
        ];
        let mut out = Vec::new();
        account
            .schedule_second(5, &mut batch, &mut out)
            .expect("a valid batch");
        assert_eq!(
            batch[0],
            Intent::Entry(EntryRequest {
                recipe: 0,
                ..entry(7, 5)
            }),
            "symbol order, then the recipe's place"
        );
        let held = admitted_in(out[0]).expect("recipe 0 takes stock 7");
        assert_eq!(
            out[1],
            Outcome::Entry(Admission::Refused(Refusal::WhileOpen))
        );
        assert!(admitted_in(out[2]).is_some(), "stock 8 is another stock");
        let third = EntryRequest {
            recipe: 3,
            ..entry(7, 6)
        };
        assert_eq!(
            account.admit(6, &third),
            Ok(Admission::Refused(Refusal::WhileOpen)),
            "a third recipe, long or short alike"
        );
        account.release(held, 9, 0).expect("held");
        let at_last_fill = EntryRequest { start: 9, ..third };
        assert_eq!(
            account.admit(9, &at_last_fill),
            Ok(Admission::Refused(Refusal::WhileOpen)),
            "locked through its last fill second"
        );
        let after = EntryRequest { start: 10, ..third };
        assert!(account.admit(10, &after).expect("valid").slot().is_some());
        assert_eq!(account.counts().while_open, 3);
        assert_consistent(&account, false);
    }

    /// Four entries in one second; 11 and 13 tie, by value, on every measured
    /// key, so each order falls through to symbol order between them.
    fn contenders(start: i64) -> Vec<EntryRequest> {
        vec![
            EntryRequest {
                keys: keys((5, 100), (3, 10), 300),
                ..entry(10, start)
            },
            EntryRequest {
                keys: keys((9, 100), (1, 10), 200),
                ..entry(11, start)
            },
            EntryRequest {
                keys: keys((2, 100), (2, 10), 100),
                ..entry(12, start)
            },
            EntryRequest {
                keys: keys((18, 200), (2, 20), 200),
                ..entry(13, start)
            },
        ]
    }

    /// The stocks of one second's batch at second 50, in the order `order` acts
    /// on them, with nineteen slots held so only the first takes a slot.
    fn ordered(order: TieBreak, batch: &[EntryRequest]) -> Vec<u8> {
        let mut account = Account::new(Capital::Carried, order);
        account.begin_day(0).expect("a session");
        for stock in 100..119_u8 {
            let at = i64::from(stock - 100);
            take(&mut account, at, &entry(stock, at));
        }
        let mut intents: Vec<Intent> = batch.iter().copied().map(Intent::Entry).collect();
        let mut out = Vec::new();
        account
            .schedule_second(50, &mut intents, &mut out)
            .expect("a valid batch");
        let took: Vec<bool> = out
            .iter()
            .map(|outcome| admitted_in(*outcome).is_some())
            .collect();
        let mut first_only = vec![false; batch.len()];
        first_only[0] = true;
        assert_eq!(
            took, first_only,
            "the one free slot goes to the first in order"
        );
        intents
            .iter()
            .map(|intent| {
                let index = batch
                    .iter()
                    .position(|request| Intent::Entry(*request) == *intent)
                    .expect("an intent of the batch");
                batch[index].stock.position()
            })
            .collect()
    }

    #[test]
    fn tie_break_a_takes_the_larger_move_first() {
        assert_eq!(
            ordered(TieBreak::Strength, &contenders(50)),
            [11, 13, 10, 12]
        );
    }

    #[test]
    fn tie_break_b_takes_the_smaller_stop_in_r_first() {
        assert_eq!(
            ordered(TieBreak::StopInR, &contenders(50)),
            [11, 13, 12, 10]
        );
    }

    #[test]
    fn tie_break_c_takes_the_earlier_first_entry_into_top_n_first() {
        assert_eq!(
            ordered(TieBreak::FirstInTop, &contenders(50)),
            [12, 11, 13, 10]
        );
    }

    #[test]
    fn tie_break_d_is_symbol_order() {
        assert_eq!(ordered(TieBreak::Symbol, &contenders(50)), [10, 11, 12, 13]);
    }

    #[test]
    fn an_earlier_start_second_goes_first_under_every_order() {
        let mut batch = contenders(50);
        batch.push(EntryRequest {
            keys: keys((0, 1), (99, 1), 999),
            ..entry(14, 49)
        });
        for order in [
            TieBreak::Strength,
            TieBreak::StopInR,
            TieBreak::FirstInTop,
            TieBreak::Symbol,
            TieBreak::Seeded(3),
        ] {
            assert_eq!(ordered(order, &batch)[0], 14, "{order:?}");
        }
    }

    #[test]
    fn two_entries_tied_on_every_key_are_ordered_by_what_they_are() {
        let small = worth(20, 50, 10, 100_000);
        let large = worth(20, 50, 20, 100_000);
        assert_eq!(
            ordered(TieBreak::Symbol, &[large, small]),
            ordered(TieBreak::Symbol, &[small, large])
        );
        let mut account = carried();
        let mut batch = [Intent::Entry(large), Intent::Entry(small)];
        let mut out = Vec::new();
        account
            .schedule_second(50, &mut batch, &mut out)
            .expect("a valid batch");
        assert_eq!(batch[0], Intent::Entry(small), "the smaller quantity first");
        assert_eq!(
            out[1],
            Outcome::Entry(Admission::Refused(Refusal::WhileOpen))
        );
    }

    #[test]
    fn the_seeded_spread_is_reproducible_from_its_seed() {
        let batch: Vec<EntryRequest> = (0..16_u8).map(|stock| entry(stock, 50)).collect();
        let reversed: Vec<EntryRequest> = batch.iter().rev().copied().collect();
        let first = ordered(TieBreak::Seeded(7), &batch);
        assert_eq!(
            first,
            ordered(TieBreak::Seeded(7), &reversed),
            "one seed, one order, whatever the arrangement"
        );
        assert_ne!(
            first,
            ordered(TieBreak::Seeded(8), &batch),
            "another seed, another order"
        );
        let mut by_key: Vec<u8> = (0..16).collect();
        by_key.sort_by_key(|&stock| {
            seeded(
                7,
                &Key {
                    start: 50,
                    stock: StockId::new(stock),
                    recipe: 0,
                    keys: entry(stock, 50).keys,
                },
            )
        });
        assert_eq!(first, by_key, "the order is the seeded key's");
        let mut sorted = first.clone();
        sorted.sort_unstable();
        assert_eq!(
            sorted,
            (0..16).collect::<Vec<u8>>(),
            "a permutation of the batch"
        );
        let later: Vec<EntryRequest> = (0..16_u8).map(|stock| entry(stock, 49)).collect();
        assert_ne!(
            first,
            ordered(TieBreak::Seeded(7), &later),
            "each start second draws afresh"
        );
    }

    #[test]
    fn twenty_simultaneous_exits_over_the_ten_a_second_limit_queue_to_the_next_second() {
        let mut account = carried();
        let held: Vec<SlotId> = (0..20_u8)
            .map(|stock| {
                let at = i64::from(stock) * 10;
                take(&mut account, at, &entry(stock, at))
            })
            .collect();
        let at = 1_000;
        let mut batch: Vec<Intent> = held
            .iter()
            .rev()
            .map(|&slot| Intent::Exit(ExitIntent { slot, start: at }))
            .collect();
        let mut out = Vec::new();
        account
            .schedule_second(at, &mut batch, &mut out)
            .expect("a valid batch");
        assert_eq!(out[..10], [Outcome::Exit(Dispatch::Sent); 10]);
        assert_eq!(out[10..], [Outcome::Exit(Dispatch::Queued(at + 1)); 10]);
        assert_eq!(
            batch[0],
            Intent::Exit(ExitIntent {
                slot: held[0],
                start: at
            }),
            "exits by symbol order under (d)"
        );
        let mut queued: Vec<Intent> = batch[10..].to_vec();
        account
            .schedule_second(at + 1, &mut queued, &mut out)
            .expect("a valid batch");
        assert_eq!(out, [Outcome::Exit(Dispatch::Sent); 10]);
        assert_eq!(account.counts().exits_sent, 20);
        assert_eq!(account.counts().exits_queued, 10);
        assert_eq!(account.orders_today(), 40);
    }

    #[test]
    fn exits_are_sent_before_entries_and_each_group_by_its_start_second() {
        let mut account = Account::new(Capital::Carried, TieBreak::Strength);
        account.begin_day(0).expect("a session");
        let weak = take(
            &mut account,
            0,
            &EntryRequest {
                keys: keys((1, 100), (1, 4), 0),
                ..entry(1, 0)
            },
        );
        let strong = take(
            &mut account,
            1,
            &EntryRequest {
                keys: keys((9, 100), (1, 4), 0),
                ..entry(2, 1)
            },
        );
        let old = take(&mut account, 2, &entry(3, 2));
        // Ten exit sends fill second 100's ring before the batch.
        for _ in 0..ORDERS_PER_SECOND - 3 {
            assert_eq!(account.send_exit(100, weak), Ok(Dispatch::Sent));
        }
        let entry_now = entry(4, 100);
        let mut batch = [
            Intent::Entry(entry_now),
            Intent::Exit(ExitIntent {
                slot: weak,
                start: 100,
            }),
            Intent::Exit(ExitIntent {
                slot: old,
                start: 99,
            }),
            Intent::Exit(ExitIntent {
                slot: strong,
                start: 100,
            }),
        ];
        let mut out = Vec::new();
        account
            .schedule_second(100, &mut batch, &mut out)
            .expect("a valid batch");
        assert_eq!(
            batch,
            [
                Intent::Exit(ExitIntent {
                    slot: old,
                    start: 99
                }),
                Intent::Exit(ExitIntent {
                    slot: strong,
                    start: 100
                }),
                Intent::Exit(ExitIntent {
                    slot: weak,
                    start: 100
                }),
                Intent::Entry(entry_now),
            ],
            "risk first: the queued exit, then by |m|, then the entry"
        );
        assert_eq!(
            out,
            [
                Outcome::Exit(Dispatch::Sent),
                Outcome::Exit(Dispatch::Sent),
                Outcome::Exit(Dispatch::Sent),
                Outcome::Entry(Admission::Queued(101)),
            ]
        );
        assert_eq!(account.counts().queued, 1);
        assert_eq!(account.trades_today(), 3, "a queued entry is not sent");
    }

    #[test]
    fn two_sends_of_one_position_in_one_second_are_both_acted_on() {
        let mut account = carried();
        let slot = take(&mut account, 0, &entry(0, 0));
        let exit = Intent::Exit(ExitIntent { slot, start: 5 });
        let mut batch = [exit, exit];
        let mut out = Vec::new();
        account
            .schedule_second(5, &mut batch, &mut out)
            .expect("a valid batch");
        assert_eq!(out, [Outcome::Exit(Dispatch::Sent); 2]);
    }

    #[test]
    fn an_exit_arriving_ahead_of_an_entry_stays_ahead() {
        let mut account = carried();
        let slot = take(&mut account, 0, &entry(0, 0));
        let exit = Intent::Exit(ExitIntent { slot, start: 5 });
        let later = Intent::Entry(entry(1, 5));
        let mut batch = [exit, later];
        let mut out = Vec::new();
        account
            .schedule_second(5, &mut batch, &mut out)
            .expect("a valid batch");
        assert_eq!(batch, [exit, later]);
        assert_eq!(out[0], Outcome::Exit(Dispatch::Sent));
        assert_eq!(admitted_in(out[0]), None, "an exit admits nothing");
        assert!(admitted_in(out[1]).is_some());
    }

    #[test]
    fn twenty_five_modifications_then_a_replacement_that_counts_in_the_rings() {
        let mut account = carried();
        let slot = take(&mut account, 0, &entry(0, 0));
        let orders = account.orders_today();
        for count in 1..=MODIFICATIONS_PER_ORDER {
            assert_eq!(
                account.modify(i64::from(count), slot, OrderRole::TrailingStop),
                Ok(Modify::Modified { count })
            );
        }
        assert_eq!(
            account.orders_today(),
            orders,
            "a modification in place is no order"
        );
        assert_eq!(
            account.modify(26, slot, OrderRole::TrailingStop),
            Ok(Modify::Replaced)
        );
        assert_eq!(
            account.orders_today(),
            orders + 1,
            "the replacement is one order"
        );
        assert_eq!(
            account.modify(27, slot, OrderRole::TrailingStop),
            Ok(Modify::Modified { count: 1 }),
            "the new order has its own 25"
        );
        for count in 2..=MODIFICATIONS_PER_ORDER {
            assert_eq!(
                account.modify(28, slot, OrderRole::TrailingStop),
                Ok(Modify::Modified { count })
            );
        }
        for _ in 0..ORDERS_PER_SECOND {
            assert_eq!(account.send_exit(100, slot), Ok(Dispatch::Sent));
        }
        assert_eq!(
            account.modify(100, slot, OrderRole::TrailingStop),
            Ok(Modify::Queued(101)),
            "a replacement over the ring waits, the old level kept"
        );
        assert_eq!(
            account.modify(101, slot, OrderRole::TrailingStop),
            Ok(Modify::Replaced)
        );
        let counts = account.counts();
        assert_eq!(counts.modified, 50);
        assert_eq!((counts.replaced, counts.replacements_queued), (2, 1));
    }

    #[test]
    fn each_resting_order_counts_its_own_modifications() {
        let mut account = carried();
        let slot = take(&mut account, 0, &entry(0, 0));
        let roles = [
            OrderRole::StopEntry,
            OrderRole::Stop,
            OrderRole::Target,
            OrderRole::TrailingStop,
            OrderRole::TrailingTakeProfit,
        ];
        for (times, role) in (1_u8..).zip(roles) {
            for _ in 0..times {
                account.modify(1, slot, role).expect("held");
            }
        }
        for (times, role) in (1_u8..).zip(roles) {
            assert_eq!(
                account.modify(2, slot, role),
                Ok(Modify::Modified { count: times + 1 }),
                "{role:?}"
            );
        }
        let next = take(&mut account, 3, &entry(1, 3));
        assert_eq!(
            account.modify(3, next, OrderRole::Stop),
            Ok(Modify::Modified { count: 1 }),
            "a new position starts every count at zero"
        );
    }

    #[test]
    fn the_slot_is_taken_at_send_and_freed_at_the_last_fill_or_a_zero_fill_cancel() {
        let mut account = carried();
        assert_eq!(account.slots_taken(), 0);
        let cancelled = take(&mut account, 10, &entry(0, 10));
        assert_eq!(account.slots_taken(), 1, "taken at send, before any fill");
        account.release(cancelled, 10, 0).expect("held");
        assert_eq!(
            account.slots_taken(),
            1,
            "in force through its cancel second"
        );
        account.settle(11).expect("forward");
        assert_eq!(account.slots_taken(), 0, "free after it");
        let filled = take(&mut account, 11, &entry(0, 11));
        account
            .resize(filled, 60)
            .expect("cut after a partial fill");
        assert_eq!(account.slots_taken(), 1, "a cut entry keeps its slot");
        account.release(filled, 500, 1_000).expect("held");
        account.settle(500).expect("forward");
        assert_eq!(
            account.slots_taken(),
            1,
            "until its exit's last fill second ends"
        );
        account.settle(501).expect("forward");
        assert_eq!(account.slots_taken(), 0);
        assert_eq!(account.capital(), i128::from(CAPITAL_PAISA) + 1_000);
        assert_eq!(account.exposure(), 0);
        assert_eq!(account.margin_in_use(), Margin(0));
    }

    #[test]
    fn the_daily_reset() {
        let mut account = carried();
        for k in 0..100_i64 {
            let slot = take(&mut account, k, &entry(0, k));
            account.release(slot, k, -100).expect("held");
        }
        assert_eq!(
            account.admit(100, &entry(0, 100)),
            Ok(Admission::Refused(Refusal::TradeCap))
        );
        account.begin_day(DAY).expect("the next session");
        assert_eq!((account.trades_today(), account.orders_today()), (0, 0));
        assert_eq!(
            account.capital(),
            i128::from(CAPITAL_PAISA) - 10_000,
            "the carried reading keeps the losses"
        );
        let held = take(&mut account, DAY, &entry(0, DAY));
        assert_eq!(
            account.begin_day(2 * DAY),
            Err(Invalid::OpenPositions(1)),
            "a held position"
        );
        account.release(held, 2 * DAY, 0).expect("held");
        assert_eq!(
            account.begin_day(2 * DAY),
            Err(Invalid::OpenPositions(1)),
            "a position at its last fill second"
        );
        account.begin_day(2 * DAY + 1).expect("one second later");
        assert_eq!(account.slots_taken(), 0);
    }

    #[test]
    fn the_reset_daily_reading_starts_each_session_at_twenty_lakh() {
        let mut account = Account::new(Capital::ResetDaily, TieBreak::Symbol);
        account.begin_day(0).expect("a session");
        let slot = take(&mut account, 0, &entry(0, 0));
        account.release(slot, 5, -5_000_000).expect("held");
        account.settle(6).expect("forward");
        assert_eq!(account.capital(), i128::from(CAPITAL_PAISA) - 5_000_000);
        account.begin_day(DAY).expect("the next session");
        assert_eq!(account.capital(), i128::from(CAPITAL_PAISA));
        assert_eq!(account.carried_net(), 0);
    }

    #[test]
    fn the_unchecked_reading_never_refuses_for_margin() {
        let mut account = Account::new(Capital::Unchecked, TieBreak::Symbol);
        account.begin_day(0).expect("a session");
        let first = take(&mut account, 0, &entry(0, 0));
        account.release(first, 1, -CAPITAL_PAISA).expect("held");
        account.settle(2).expect("forward");
        assert_eq!(account.capital(), 0);
        let request = EntryRequest {
            m_ppm: 1_000_000,
            ..worth(1, 2, 1_000, 100_000)
        };
        assert!(account.admit(2, &request).expect("valid").slot().is_some());
        assert_eq!(account.counts().margin_cap, 0);
    }

    #[test]
    fn the_margin_rate_has_a_twenty_percent_floor() {
        let mut account = carried();
        let floor = take(&mut account, 0, &entry(0, 0));
        assert_eq!(
            account.margin(floor),
            Ok(Margin(10_000_000 * i128::from(MARGIN_FLOOR_PPM))),
            "M under 20% blocks 20%"
        );
        let quarter = take(
            &mut account,
            1,
            &EntryRequest {
                m_ppm: 250_000,
                ..entry(1, 1)
            },
        );
        let margin = account.margin(quarter).expect("held");
        assert_eq!(
            margin.floor_paisa(),
            2_500_000,
            "M = 25% blocks Rs 25,000 of Rs 1 lakh"
        );
        let odd = take(
            &mut account,
            2,
            &EntryRequest {
                m_ppm: 221_100,
                ..worth(2, 2, 3, 333)
            },
        );
        let margin = account.margin(odd).expect("held");
        assert_eq!(
            margin.micro_paisa(),
            999 * 221_100,
            "M = 22.11%, kept exact"
        );
        assert_eq!((margin.floor_paisa(), margin.ceil_paisa()), (220, 221));
    }

    #[test]
    fn a_margin_rounds_down_for_the_mtm_rule_and_up_on_request() {
        for (micro, floor, ceil) in [
            (0, 0, 0),
            (1, 0, 1),
            (999_999, 0, 1),
            (1_000_000, 1, 1),
            (1_000_001, 1, 2),
        ] {
            let margin = Margin(micro);
            assert_eq!(margin.micro_paisa(), micro);
            assert_eq!((margin.floor_paisa(), margin.ceil_paisa()), (floor, ceil));
        }
    }

    #[test]
    fn marks_and_resizes_move_the_exposure_and_the_margin_and_never_force_a_reduction() {
        let mut account = carried();
        let slot = take(&mut account, 0, &entry(0, 0));
        account.mark(slot, 150_000).expect("held");
        assert_eq!(account.exposure(), 15_000_000);
        assert_eq!(account.margin_in_use(), Margin(15_000_000 * 200_000));
        account.resize(slot, 40).expect("held");
        assert_eq!(account.exposure(), 6_000_000);
        account.mark(slot, 30_000_000).expect("held");
        assert_eq!(account.exposure(), 1_200_000_000, "a mark may pass the cap");
        assert_eq!(account.slots_taken(), 1, "and reduces nothing");
        assert_eq!(
            account.admit(1, &worth(1, 1, 1, 1)),
            Ok(Admission::Refused(Refusal::ExposureCap)),
            "the next entry is checked against it"
        );
        assert_eq!(account.mark(slot, 0), Err(Invalid::NonPositivePrice(0)));
        assert_eq!(
            account.mark(slot, i64::MAX / 39),
            Err(Invalid::ValueOverflow)
        );
        assert_eq!(account.resize(slot, 0), Err(Invalid::ZeroQuantity));
        assert_eq!(
            account.resize(slot, 41),
            Err(Invalid::QuantityGrows {
                held: 40,
                asked: 41
            })
        );
        assert_eq!(
            account.exposure(),
            1_200_000_000,
            "a refused call changes nothing"
        );
        assert_consistent(&account, true);
    }

    #[test]
    fn queued_entries_change_nothing_and_are_checked_again_when_offered() {
        let mut account = carried();
        let held: Vec<SlotId> = (0..19_u8)
            .map(|stock| {
                take(
                    &mut account,
                    i64::from(stock),
                    &entry(stock, i64::from(stock)),
                )
            })
            .collect();
        for _ in 0..ORDERS_PER_SECOND {
            assert_eq!(account.send_exit(100, held[0]), Ok(Dispatch::Sent));
        }
        let waiting = entry(50, 100);
        let before = (
            account.trades_today(),
            account.slots_taken(),
            account.exposure(),
        );
        assert_eq!(account.admit(100, &waiting), Ok(Admission::Queued(101)));
        assert_eq!(
            (
                account.trades_today(),
                account.slots_taken(),
                account.exposure()
            ),
            before,
            "nothing changed"
        );
        take(&mut account, 101, &entry(51, 101));
        assert_eq!(
            account.admit(101, &waiting),
            Ok(Admission::Refused(Refusal::PositionCap)),
            "offered again, every check runs again"
        );
        assert_eq!(
            (account.counts().queued, account.counts().position_cap),
            (1, 1)
        );
    }

    #[test]
    fn the_minute_and_hour_rings_queue_to_the_second_their_oldest_send_expires() {
        let mut account = carried();
        let slot = take(&mut account, 0, &entry(0, 0));
        // 250 orders in 25 seconds fill the minute ring: its oldest went at 0.
        for _ in 1..ORDERS_PER_SECOND {
            assert_eq!(account.send_exit(0, slot), Ok(Dispatch::Sent));
        }
        for second in 1..=24 {
            for _ in 0..ORDERS_PER_SECOND {
                account.send_exit(second, slot).expect("held");
            }
        }
        assert_eq!(account.send_exit(25, slot), Ok(Dispatch::Queued(60)));
        assert_eq!(account.send_exit(60, slot), Ok(Dispatch::Sent));
        // A thousand orders in 250 seconds fill the hour ring, never the minute's.
        let mut hourly = carried_at(DAY);
        let slot = take(&mut hourly, DAY, &entry(0, DAY));
        for second in 0..250 {
            let sends = if second == 0 { 3 } else { 4 };
            for _ in 0..sends {
                assert_eq!(hourly.send_exit(DAY + second, slot), Ok(Dispatch::Sent));
            }
        }
        assert_eq!(
            hourly.orders_today(),
            u32::try_from(ORDERS_PER_HOUR).expect("fits")
        );
        assert_eq!(
            hourly.send_exit(DAY + 250, slot),
            Ok(Dispatch::Queued(DAY + 3_600))
        );
        assert_eq!(hourly.send_exit(DAY + 3_600, slot), Ok(Dispatch::Sent));
    }

    /// A carried account with its first session begun at `open`.
    fn carried_at(open: i64) -> Account {
        let mut account = Account::new(Capital::Carried, TieBreak::Symbol);
        account.begin_day(open).expect("a first session");
        account
    }

    #[test]
    fn the_sessions_orders_stop_every_order_until_the_next_session() {
        let mut account = carried();
        let slot = take(&mut account, 0, &entry(0, 0));
        for k in 1..ORDERS_PER_DAY {
            assert_eq!(
                account.send_exit(i64::from(k) * 4, slot),
                Ok(Dispatch::Sent)
            );
        }
        let at = i64::from(ORDERS_PER_DAY) * 4;
        assert_eq!(account.orders_today(), ORDERS_PER_DAY);
        assert_eq!(account.send_exit(at, slot), Ok(Dispatch::Queued(i64::MAX)));
        assert_eq!(
            account.admit(at, &entry(1, at)),
            Ok(Admission::Queued(i64::MAX))
        );
        account.release(slot, at, 0).expect("held");
        account.begin_day(at + 1).expect("the next session");
        assert!(
            account
                .admit(at + 1, &entry(1, at + 1))
                .expect("valid")
                .slot()
                .is_some()
        );
    }

    #[test]
    fn the_rings_slide_across_a_session_reset() {
        let mut account = carried();
        let slot = take(&mut account, 0, &entry(0, 0));
        for _ in 1..ORDERS_PER_SECOND {
            account.send_exit(0, slot).expect("held");
        }
        account.release(slot, 0, 0).expect("held");
        account.begin_day(1).expect("a session one second later");
        // The second ring held 0's ten sends; at 1 they are a second old.
        assert!(
            account
                .admit(1, &entry(0, 1))
                .expect("valid")
                .slot()
                .is_some()
        );
        let mut busy = carried();
        let slot = take(&mut busy, 0, &entry(0, 0));
        for second in 0..25 {
            let sends = if second == 0 { 9 } else { 10 };
            for _ in 0..sends {
                busy.send_exit(second, slot).expect("held");
            }
        }
        busy.release(slot, 25, 0).expect("held");
        busy.begin_day(26).expect("a session");
        assert_eq!(
            busy.admit(26, &entry(0, 26)),
            Ok(Admission::Queued(60)),
            "the minute ring is not emptied by the reset"
        );
    }

    #[test]
    fn each_window_starts_at_twenty_lakh() {
        let mut account = Account::new(Capital::Carried, TieBreak::Symbol);
        assert_eq!(account.capital(), i128::from(CAPITAL_PAISA));
        account.begin_window(0).expect("the first window");
        let slot = take(&mut account, 0, &entry(0, 0));
        assert_eq!(account.begin_window(DAY), Err(Invalid::OpenPositions(1)));
        account.release(slot, 5, -3_000_000).expect("held");
        account.settle(6).expect("forward");
        assert_eq!(account.capital(), i128::from(CAPITAL_PAISA) - 3_000_000);
        account.begin_window(DAY).expect("the next window");
        assert_eq!(
            account.capital(),
            i128::from(CAPITAL_PAISA),
            "Rs 20 lakh again"
        );
        assert_eq!(
            account.counts(),
            Account::new(Capital::Carried, TieBreak::Symbol).counts()
        );
        assert_eq!(account.clock(), DAY);
        assert_eq!(
            account.mark(slot, 1),
            Err(Invalid::StaleSlot),
            "the old id stays stale"
        );
        let next = take(&mut account, DAY, &entry(0, DAY));
        assert_eq!(next.ticket(), 2, "tickets carry on across windows");
        assert_eq!(
            account.begin_window(DAY - 1),
            Err(Invalid::TimeBackwards {
                at: DAY - 1,
                clock: DAY
            })
        );
    }

    /// One planned entry of a candidate: day, second of the day, stock,
    /// value in rupees, `M / 100` in ppm, exit second of the day, net in paisa.
    type Planned = (i64, i64, u8, u64, u32, i64, i64);

    /// Walks the entries of `plan` from day `from` on, on one account that
    /// starts there at Rs 20 lakh, and says which were admitted.
    fn walk(capital: Capital, plan: &[Planned], from: i64) -> (Vec<bool>, Account) {
        let mut account = Account::new(capital, TieBreak::Symbol);
        let mut session = None;
        let mut admitted = Vec::new();
        for &(day, second, stock, rupees, m_ppm, exit, net) in
            plan.iter().filter(|row| row.0 >= from)
        {
            if session != Some(day) {
                account.begin_day(day * DAY).expect("flat at every open");
                session = Some(day);
            }
            let at = day * DAY + second;
            let request = EntryRequest {
                m_ppm,
                ..worth(stock, at, rupees, 100)
            };
            let outcome = account.admit(at, &request).expect("a valid entry");
            if let Some(slot) = outcome.slot() {
                account.release(slot, day * DAY + exit, net).expect("held");
            }
            admitted.push(outcome.slot().is_some());
        }
        (admitted, account)
    }

    /// Three overlapping Rs 6 lakh positions at 100% margin a day, Rs 18 lakh
    /// in use, and Rs 1.5 lakh of losses at most from a peak.
    const UNDER: [Planned; 12] = [
        (0, 10, 1, 600_000, 1_000_000, 100, 5_000_000),
        (0, 11, 2, 600_000, 1_000_000, 100, -10_000_000),
        (0, 12, 3, 600_000, 1_000_000, 100, -5_000_000),
        (1, 10, 1, 600_000, 1_000_000, 100, 2_000_000),
        (1, 11, 2, 600_000, 1_000_000, 100, -1_000_000),
        (1, 12, 3, 600_000, 1_000_000, 100, 3_000_000),
        (2, 10, 1, 600_000, 1_000_000, 100, -2_000_000),
        (2, 11, 2, 600_000, 1_000_000, 100, 1_000_000),
        (2, 12, 1, 600_000, 1_000_000, 100, 0),
        (3, 10, 4, 600_000, 1_000_000, 100, 1_000_000),
        (3, 11, 5, 600_000, 1_000_000, 100, 1_000_000),
        (3, 12, 6, 600_000, 1_000_000, 100, 1_000_000),
    ];

    #[test]
    fn a_candidate_under_the_capital_bound_has_equal_rows_under_two_schemes() {
        let (unchecked, once) = walk(Capital::Unchecked, &UNDER, 0);
        let bound = once.capital_bound();
        assert_eq!(bound.peak_margin, Margin(180_000_000 * MICRO));
        assert_eq!(bound.largest_fall, 15_000_000);
        assert!(
            bound.holds(),
            "Rs 18 lakh + Rs 1.5 lakh is within Rs 20 lakh"
        );
        let (anchored, carried) = walk(Capital::Carried, &UNDER, 0);
        assert!(carried.capital_bound().holds());
        assert_eq!(anchored, unchecked, "the capital check never bound");
        // A second scheme's window starting at day 2 starts at Rs 20 lakh.
        let (rolling, _) = walk(Capital::Carried, &UNDER, 2);
        assert_eq!(rolling, anchored[6..], "the same rows from day 2");
        assert_eq!(
            rolling,
            [true, true, false, true, true, true],
            "stock 1 is still held at 12"
        );
    }

    /// A Rs 15 lakh loss on day 0, then a position needing Rs 18 lakh of margin.
    const OVER: [Planned; 2] = [
        (0, 10, 1, 100_000, 0, 100, -150_000_000),
        (1, 10, 2, 1_800_000, 1_000_000, 100, 0),
    ];

    #[test]
    fn a_candidate_over_the_capital_bound_is_walked_per_window() {
        let (_, once) = walk(Capital::Unchecked, &OVER, 0);
        assert!(
            !once.capital_bound().holds(),
            "Rs 18 lakh + Rs 15 lakh is past Rs 20 lakh"
        );
        let (span, _) = walk(Capital::Carried, &OVER, 0);
        let (window, _) = walk(Capital::Carried, &OVER, 1);
        assert_eq!(
            span,
            [true, false],
            "the carried walk had Rs 5 lakh on day 1"
        );
        assert_eq!(window, [true], "a window from day 1 has Rs 20 lakh");
    }

    /// A Rs 5 lakh loss on day 0, then a position needing Rs 18 lakh of margin.
    const REFUSED: [Planned; 2] = [
        (0, 10, 1, 100_000, 0, 100, -50_000_000),
        (1, 10, 2, 1_800_000, 1_000_000, 100, 0),
    ];

    #[test]
    fn a_walk_refused_for_capital_never_shares_its_rows() {
        let (span, carried) = walk(Capital::Carried, &REFUSED, 0);
        assert_eq!(span, [true, false]);
        let bound = carried.capital_bound();
        assert!(
            bound.peak_margin.micro_paisa() + bound.largest_fall * MICRO
                <= i128::from(CAPITAL_PAISA) * MICRO,
            "its figures alone would pass"
        );
        assert_eq!(bound.margin_refusals, 1);
        assert!(!bound.holds(), "a margin refusal is the failure itself");
        let (window, _) = walk(Capital::Carried, &REFUSED, 1);
        assert_eq!(window, [true], "a window after the fall takes that entry");
    }

    #[test]
    fn fractions_compare_by_value_and_refuse_a_zero_denominator() {
        assert_eq!(frac(1, 2), frac(2, 4));
        assert!(frac(1, 3) < frac(1, 2));
        assert!(frac(u64::MAX, 1) > frac(u64::MAX - 1, 1));
        assert!(frac(1, u64::MAX) < frac(1, u64::MAX - 1));
        assert_eq!((frac(3, 7).num(), frac(3, 7).den()), (3, 7));
        assert_eq!(Fraction::new(1, 0), Err(Invalid::ZeroDenominator));
    }

    #[test]
    fn identities_read_back() {
        assert_eq!(StockId::new(217).position(), 217);
        let mut account = carried();
        let slot = take(&mut account, 0, &entry(217, 0));
        assert_eq!((slot.index(), slot.ticket()), (0, 1));
        assert_eq!(STOCKS, 256, "every u8 position has a lock");
    }

    #[test]
    fn refusals_carry_the_names_the_spec_counts() {
        let names = [
            Refusal::PositionCap,
            Refusal::TradeCap,
            Refusal::ExposureCap,
            Refusal::MarginCap,
            Refusal::WhileOpen,
        ]
        .map(Refusal::name);
        assert_eq!(
            names,
            [
                "position_cap",
                "trade_cap",
                "exposure_cap",
                "margin_cap",
                "while_open"
            ]
        );
        assert_eq!(Admission::Refused(Refusal::TradeCap).slot(), None);
        assert_eq!(Admission::Queued(9).slot(), None);
    }

    #[test]
    fn time_going_backwards_is_refused_by_name_and_changes_nothing() {
        let mut account = carried();
        let slot = take(&mut account, 10, &entry(0, 10));
        let backwards = Invalid::TimeBackwards { at: 9, clock: 10 };
        assert_eq!(account.admit(9, &entry(1, 9)), Err(backwards));
        assert_eq!(account.send_exit(9, slot), Err(backwards));
        assert_eq!(account.modify(9, slot, OrderRole::Stop), Err(backwards));
        assert_eq!(account.settle(9), Err(backwards));
        assert_eq!(account.begin_day(9), Err(backwards));
        let mut out = vec![Outcome::Exit(Dispatch::Sent)];
        assert_eq!(
            account.schedule_second(9, &mut [], &mut out),
            Err(backwards)
        );
        assert_eq!(out, [Outcome::Exit(Dispatch::Sent)], "out is untouched");
        assert_eq!(account.clock(), 10);
        assert_eq!(account.counts().admitted, 1);
    }

    #[test]
    fn a_bad_request_is_refused_by_name() {
        let mut account = carried();
        assert_eq!(
            account.admit(5, &entry(0, 6)),
            Err(Invalid::BeforeStart { at: 5, start: 6 })
        );
        assert_eq!(
            account.admit(5, &worth(0, 5, 0, 100)),
            Err(Invalid::ZeroQuantity)
        );
        assert_eq!(
            account.admit(5, &worth(0, 5, 1, 0)),
            Err(Invalid::NonPositivePrice(0))
        );
        assert_eq!(
            account.admit(5, &worth(0, 5, 1, -5)),
            Err(Invalid::NonPositivePrice(-5))
        );
        assert_eq!(
            account.admit(5, &worth(0, 5, 2, i64::MAX)),
            Err(Invalid::ValueOverflow)
        );
        assert_eq!(
            account.admit(5, &worth(0, 5, 1, i64::MAX)),
            Ok(Admission::Refused(Refusal::ExposureCap)),
            "a value of exactly i64::MAX paisa is checked, and refused by the cap"
        );
        assert_eq!(
            account.clock(),
            5,
            "only the call that was checked moved the clock"
        );
    }

    #[test]
    fn a_stale_slot_is_refused_everywhere() {
        let mut account = carried();
        let slot = take(&mut account, 0, &entry(0, 0));
        account.release(slot, 1, 0).expect("held");
        let stale = Invalid::StaleSlot;
        assert_eq!(
            account.mark(slot, 1),
            Err(stale),
            "a closing position is no longer held"
        );
        assert_eq!(account.release(slot, 2, 0), Err(stale));
        account.settle(2).expect("forward");
        let reused = take(&mut account, 2, &entry(1, 2));
        assert_eq!(reused.index(), slot.index(), "the slot is reused");
        assert_eq!(
            account.resize(slot, 1),
            Err(stale),
            "the old ticket names nothing"
        );
        assert_eq!(account.send_exit(3, slot), Err(stale));
        assert_eq!(account.modify(3, slot, OrderRole::Stop), Err(stale));
        assert_eq!(account.margin(slot), Err(stale));
        let mut batch = [Intent::Exit(ExitIntent { slot, start: 3 })];
        let mut out = Vec::new();
        assert_eq!(account.schedule_second(3, &mut batch, &mut out), Err(stale));
        assert_eq!(account.release(slot, 3, 0), Err(stale));
        assert!(account.margin(reused).is_ok());
    }

    #[test]
    fn a_release_before_its_send_is_refused() {
        let mut account = carried();
        let slot = take(&mut account, 10, &entry(0, 10));
        assert_eq!(
            account.release(slot, 9, 0),
            Err(Invalid::ReleaseBeforeSend {
                last_fill: 9,
                sent: 10
            })
        );
        assert_eq!(account.slots_taken(), 1);
    }

    #[test]
    fn a_bad_batch_changes_nothing() {
        let mut account = carried();
        let slot = take(&mut account, 0, &entry(0, 0));
        let mut batch = [
            Intent::Entry(entry(1, 5)),
            Intent::Exit(ExitIntent { slot, start: 6 }),
        ];
        let mut out = vec![Outcome::Exit(Dispatch::Sent)];
        assert_eq!(
            account.schedule_second(5, &mut batch, &mut out),
            Err(Invalid::BeforeStart { at: 5, start: 6 })
        );
        let mut entries = [Intent::Entry(entry(1, 5)), Intent::Entry(worth(2, 5, 0, 1))];
        assert_eq!(
            account.schedule_second(5, &mut entries, &mut out),
            Err(Invalid::ZeroQuantity)
        );
        assert_eq!(out, [Outcome::Exit(Dispatch::Sent)]);
        assert_eq!(
            batch[0],
            Intent::Entry(entry(1, 5)),
            "the batch was not reordered"
        );
        assert_eq!((account.clock(), account.trades_today()), (0, 1));
    }

    #[test]
    fn the_capital_bound_needs_both_halves() {
        let bound =
            |peak_rupees: i128, fall_rupees: i128, margin_refusals: u64| super::CapitalBound {
                peak_margin: Margin(peak_rupees * 100 * MICRO),
                largest_fall: fall_rupees * 100,
                margin_refusals,
            };
        assert!(bound(1_500_000, 500_000, 0).holds(), "exactly Rs 20 lakh");
        assert!(!bound(1_500_000, 500_001, 0).holds(), "one rupee past it");
        assert!(!bound(0, 0, 1).holds(), "a refusal");
    }

    #[test]
    fn the_bench_fixture_admits_every_entry_it_times() {
        let samples = measure_admit_release(1, 1).expect("the fixture runs as built");
        assert_eq!(samples.admitted, 200, "two sessions of 100 entries");
        assert_eq!((samples.admit_ns.len(), samples.release_ns.len()), (80, 80));
        let none = measure_admit_release(0, 0).expect("nothing to run");
        assert_eq!(none, super::Samples::default());
    }

    #[test]
    fn after_any_random_sequence_slots_exposure_trades_and_locks_hold() {
        for seed in 0..48_u64 {
            random_walk(seed);
        }
    }

    /// One seeded walk of 1,500 random calls, every invariant checked after
    /// each, and every send checked against all three rings.
    fn random_walk(seed: u64) {
        let marks = seed % 2 == 1;
        let capital = [Capital::Carried, Capital::ResetDaily, Capital::Unchecked]
            [usize::try_from(seed % 3).expect("small")];
        let order = [
            TieBreak::Strength,
            TieBreak::StopInR,
            TieBreak::FirstInTop,
            TieBreak::Symbol,
            TieBreak::Seeded(seed),
        ][usize::try_from(seed % 5).expect("small")];
        let mut account = Account::new(capital, order);
        let mut state = mix(seed);
        let mut at = 0_i64;
        let mut held: Vec<SlotId> = Vec::new();
        let mut sends: Vec<i64> = Vec::new();
        account.begin_day(at).expect("a session");
        for _ in 0..1_500 {
            state = mix(state);
            at += i64::try_from(state % 3).expect("small");
            let pick = (state >> 8) % 9;
            let which = usize::try_from(state >> 40).expect("fits") % held.len().max(1);
            match pick {
                0..=2 => {
                    let request = random_entry(state, at);
                    let admission = account.admit(at, &request).expect("a valid entry");
                    note_admission(&account, admission, at, &mut held, &mut sends);
                }
                3 if !held.is_empty() => {
                    let slot = held.swap_remove(which);
                    let last_fill = at + i64::try_from((state >> 16) % 5).expect("small");
                    let net = i64::try_from((state >> 24) % 400_001).expect("small") - 200_000;
                    account.release(slot, last_fill, net).expect("held");
                }
                4 if marks && !held.is_empty() => {
                    let price = 1_000 + i64::try_from((state >> 16) % 400_000).expect("small");
                    account.mark(held[which], price).expect("held");
                }
                5 if !held.is_empty() => {
                    let dispatch = account.send_exit(at, held[which]).expect("held");
                    note_dispatch(dispatch, at, &mut sends);
                }
                6 if !held.is_empty() => {
                    // A burst on one order, so the 26th change replaces it and
                    // replacements fill the second's ring.
                    let roles = [OrderRole::Stop, OrderRole::TrailingStop, OrderRole::Target];
                    let role = roles[usize::try_from((state >> 16) % 3).expect("small")];
                    for _ in 0..(state >> 24) % 320 {
                        match account.modify(at, held[which], role).expect("held") {
                            Modify::Modified { count } => assert!(count <= MODIFICATIONS_PER_ORDER),
                            Modify::Replaced => note_send(at, &mut sends),
                            Modify::Queued(next) => assert!(next > at),
                        }
                    }
                }
                7 => {
                    // Exits first in the batch, entries after: the sort must
                    // put them right whichever way round they arrive.
                    let mut batch: Vec<Intent> = held
                        .iter()
                        .take(2)
                        .map(|&slot| Intent::Exit(ExitIntent { slot, start: at }))
                        .chain(
                            (0..(state >> 16) % 4)
                                .map(|k| Intent::Entry(random_entry(mix(state ^ k), at))),
                        )
                        .collect();
                    let mut out = Vec::new();
                    account
                        .schedule_second(at, &mut batch, &mut out)
                        .expect("a valid batch");
                    for outcome in out {
                        match outcome {
                            Outcome::Entry(admission) => {
                                note_admission(&account, admission, at, &mut held, &mut sends);
                            }
                            Outcome::Exit(dispatch) => note_dispatch(dispatch, at, &mut sends),
                        }
                    }
                }
                8 if (state >> 20).is_multiple_of(16) => {
                    for slot in held.drain(..) {
                        account.release(slot, at, 0).expect("held");
                    }
                    at += DAY;
                    account.begin_day(at).expect("flat after every release");
                }
                _ => {}
            }
            assert_consistent(&account, marks);
            sends.retain(|&sent| sent > at - 3_600);
        }
        assert!(
            account.counts().admitted > 0,
            "seed {seed} admitted something"
        );
    }

    /// A random entry at `at`: one of 40 stocks, so locks collide, and up to
    /// Rs 60 lakh, so every cap binds now and then.
    fn random_entry(state: u64, at: i64) -> EntryRequest {
        let stock = u8::try_from((state >> 12) % 40).expect("small");
        let qty = 1 + (state >> 18) % 2_000;
        let price = 1_000 + i64::try_from((state >> 30) % 300_000).expect("small");
        EntryRequest {
            recipe: u32::try_from((state >> 50) % 3).expect("small"),
            m_ppm: u32::try_from((state >> 52) % 600_000).expect("small"),
            keys: keys(((state >> 4) % 50, 100), ((state >> 6) % 9, 10), at),
            ..worth(stock, at, qty, price)
        }
    }

    /// Records an admission's send and holds its position; checks a queue
    /// names a later second and an admission stays under its caps.
    fn note_admission(
        account: &Account,
        admission: Admission,
        at: i64,
        held: &mut Vec<SlotId>,
        sends: &mut Vec<i64>,
    ) {
        if let Some(slot) = admission.slot() {
            held.push(slot);
            note_send(at, sends);
            assert!(account.exposure() <= i128::from(EXPOSURE_CAP_PAISA));
            if account.capital != Capital::Unchecked {
                assert!(account.margin_in_use().micro_paisa() <= account.capital() * MICRO);
            }
        }
        if let Admission::Queued(next) = admission {
            assert!(next > at);
        }
    }

    /// Records a dispatch's send; checks a queue names a later second.
    fn note_dispatch(dispatch: Dispatch, at: i64, sends: &mut Vec<i64>) {
        match dispatch {
            Dispatch::Sent => note_send(at, sends),
            Dispatch::Queued(next) => assert!(next > at),
        }
    }

    /// Records one send at `at` and checks the three rings' windows hold.
    fn note_send(at: i64, sends: &mut Vec<i64>) {
        sends.push(at);
        let within = |window: i64| sends.iter().filter(|&&sent| sent > at - window).count();
        assert!(within(1) <= ORDERS_PER_SECOND);
        assert!(within(60) <= ORDERS_PER_MINUTE);
        assert!(within(3_600) <= ORDERS_PER_HOUR);
    }
}
