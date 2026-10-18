//! The scoring of a visited mix: one ordered pass over its rows, one position
//! per stock, its month vectors and its scorecard (FIB-6; D-6402, D-6403,
//! D-6409, D-6410).
//!
//! # One position per stock per mix (D-6402)
//!
//! Overlapping trades inflate `n` and `t`, so each profile (an exit variant at
//! a trade size, FM-F117) holds one position per stock. A candidate whose
//! DECISION second is at or before the open position's block-until second is
//! skipped and counted, never traded. What a slot blocks follows its outcome
//! (FM-F056): a priced slot blocks to its exit second; a pending order (an
//! armed BREAK, an open entry window) to its window end; an unpriced slot
//! (no execution seconds, a refused factor, an unpriceable forced exit, a feed
//! hole, a corrupt bar) to the end of the stock-day; a zero-quantity slot and an
//! inapplicable variant block nothing. Ties go by canonical order, which is
//! decision-id order inside one (day, second, stock), and are counted. The raw
//! count still bounds the effective count, so the lattice's raw-count prune
//! loses no mix.
//!
//! The block state is EPOCH-STAMPED scratch (FM-F050): each (profile, stock)
//! slot holds the pass and the day it was written in, and a stale stamp reads
//! as empty, so nothing is reset per day or per mix.
//!
//! # Net, never gross
//!
//! A trade's net money is its gross money less its charges from a
//! [`ChargeModel`]: a per (stock, day) accumulator whose `add` is O(1) and whose
//! `close_day` returns the day's rounding adjustment. [`Gross`] is the zero
//! model, a labelled plumbing run only (D-6300); FIB-11 supplies CHG-1's. Net R
//! is net money over (quantity x risk per share). A trade WINS only if its net
//! money is above zero: zero is a loss.
//!
//! # The scorecard is information (D-6403)
//!
//! Streaks run in ENTRY order, drawdown is taken at each DAY CLOSE in R, and the
//! period tables (day, week, fortnight, month, quarter, half-year, year) are in
//! R: [`SCORECARD_LABEL`]. The Winner Gate's downside checks (FIB-7) are money
//! in exit order and are not these. Statistics are `f64` at full precision,
//! summed only in canonical row order, so a rerun is byte-identical.
//!
//! # Cost
//!
//! Per mix: O(support x profiles + touched months + days with trades x periods)
//! — O(1) per row-touch per profile. Memory per pass, reserved once: profiles x
//! pool block slots, profiles x months month statistics, and the ladders.
//!
//! Pure: no I/O, no print (gates 17 and 22).

#![allow(
    clippy::float_arithmetic,
    clippy::cast_precision_loss,
    reason = "statistics of R are floating point at full precision (CLAUDE.md section 7); \
              money stays i128 and is converted only to form a ratio"
)]

use vocab::fib_cube::{BP_PER_UNIT, MixKey, OPEN, Ratio, RowId, SIDE_ROW, Search};

use super::lattice::{Node, RowTable, RowView, Visitor, Wanted};
use super::types::{SlotOutcome, TradeFlag, TradeSize};

/// The one statistic type: every R, R squared, stop in bp and excursion sum.
pub type Stat = f64;

/// Zero as a statistic. No float literal is typed in a FIB strategy file
/// (operator 16 Oct 11:24 IST): this is the integer 0, converted.
#[allow(
    clippy::unnecessary_cast,
    reason = "the literal must be the integer 0 for the no-typed-numbers scan"
)]
const ZERO: Stat = 0 as Stat;

/// The label every scorecard figure carries: it informs and never decides.
pub const SCORECARD_LABEL: &str = "scorecard (R, day close, entry order) - information";

/// The two report sizes, in size order (`vocab::fib_cube::FIXED_NUMBERS`).
pub const SIZES: &[TradeSize] = &[TradeSize::Small, TradeSize::Large];

/// How many report sizes a slot carries.
pub const SIZE_COUNT: usize = SIZES.len();

/// A size's position in [`SIZES`].
#[must_use]
pub const fn size_index(size: TradeSize) -> usize {
    match size {
        TradeSize::Small => 0,
        TradeSize::Large => 1,
    }
}

/// One flag per possible exit value.
const EXIT_VALUES: usize = OPEN as usize;

/// How many side values the schema has (each search uses two of them).
pub const SIDE_VALUES: usize = SIDE_ROW.labels.len();

/// Why a slot that was entered or armed could not be priced. Each counts in
/// the raw count, adds no R, and blocks to the end of the stock-day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Unpriced {
    /// No execution second existed to price it (a deep re-walk, FM-F052).
    NoExecutionSeconds,
    /// DATA-1's factor pair was refused for the stock-day.
    FactorRefused,
    /// The forced exit found no print in its window.
    ForcedExitUnpriceable,
    /// A hole in the feed inside the trade.
    FeedHole,
    /// A corrupt bar inside the trade.
    CorruptBar,
}

impl Unpriced {
    /// The reason's stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NoExecutionSeconds => "no_execution_seconds",
            Self::FactorRefused => "factor_refused",
            Self::ForcedExitUnpriceable => "forced_exit_unpriceable",
            Self::FeedHole => "feed_hole",
            Self::CorruptBar => "corrupt_bar",
        }
    }
}

/// What one slot — a decision's exit variant at one size — came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// Filled and exited, in exact integers.
    Priced(SlotOutcome),
    /// An order armed or a window open that never filled: no trade, and it
    /// blocks the stock to `until_second`.
    Pending {
        /// The second the order or window expired.
        until_second: u32,
    },
    /// Entered or armed and not priceable.
    Unpriced(Unpriced),
    /// The size buys no share: no trade, blocks nothing.
    ZeroQty,
    /// This exit variant does not apply to the decision: no trade, blocks
    /// nothing.
    Inapplicable,
}

/// One outcome record: a decision's slot at each report size, in [`SIZES`]
/// order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// The slot at each size.
    pub slots: [Slot; SIZE_COUNT],
}

/// One profile: an exit variant of the run's own set at one report size
/// (FM-F117).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Profile {
    /// The exit variant, as its exit-row value.
    pub variant: u8,
    /// The size.
    pub size: TradeSize,
}

/// Every profile of a table: its exit variants times the report sizes, in
/// (variant, size) order. A caller splits them into passes of its measured
/// budget with `chunks`.
#[must_use]
pub fn profiles_of(table: &RowTable) -> Vec<Profile> {
    table
        .partition()
        .exits
        .clone()
        .flat_map(|variant| {
            SIZES.iter().map(move |size| Profile {
                variant,
                size: *size,
            })
        })
        .collect()
}

/// One order of a trade, as a charge model sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Order {
    /// The stock slot.
    pub stock: u32,
    /// The IST day.
    pub day: u32,
    /// A buy (or a sell).
    pub buy: bool,
    /// Shares.
    pub qty: u64,
    /// The order's value, in paisa.
    pub value_paisa: i128,
    /// Whether the broker squared it off (Dhan's fee applies).
    pub broker_square_off: bool,
}

/// The charges of each trade: a per (stock, day) accumulator, O(1) per order.
pub trait ChargeModel: Send {
    /// The model's money unit per paisa (CHG-1's scale; one for [`Gross`]).
    fn scale(&self) -> i128;

    /// Adds one order to its (stock, day) accumulator and returns that order's
    /// charges, unrounded, in the model's unit.
    fn add(&mut self, order: &Order) -> i128;

    /// Closes every accumulator of `day` and returns the day's rounding
    /// adjustment in the model's unit: positive is an extra charge.
    fn close_day(&mut self, day: u32) -> i128;
}

/// The zero charge model: every figure gross, a labelled plumbing run only
/// (D-6300). Never a gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gross;

impl ChargeModel for Gross {
    fn scale(&self) -> i128 {
        1
    }

    fn add(&mut self, _order: &Order) -> i128 {
        0
    }

    fn close_day(&mut self, _day: u32) -> i128 {
        0
    }
}

/// The calendar keys of one observed tradable day (FIB-10's table): opaque,
/// non-decreasing period keys and the day's tradable index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DayKeys {
    /// Observed tradable days before this one: the clock drawdown lengths run
    /// on.
    pub tradable_index: u32,
    /// The ISO week key.
    pub week: u32,
    /// The fortnight key.
    pub fortnight: u32,
    /// The calendar month key.
    pub month: u32,
    /// The quarter key.
    pub quarter: u32,
    /// The half-year key.
    pub half_year: u32,
    /// The year key.
    pub year: u32,
}

/// The period kinds every scorecard tabulates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PeriodKind {
    /// The IST day.
    Day,
    /// The ISO week.
    Week,
    /// The fortnight.
    Fortnight,
    /// The calendar month.
    Month,
    /// The quarter.
    Quarter,
    /// The half-year.
    HalfYear,
    /// The year.
    Year,
}

impl PeriodKind {
    /// Every kind, in table order.
    pub const ALL: &'static [Self] = &[
        Self::Day,
        Self::Week,
        Self::Fortnight,
        Self::Month,
        Self::Quarter,
        Self::HalfYear,
        Self::Year,
    ];

    /// The kind's name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
            Self::Fortnight => "fortnight",
            Self::Month => "month",
            Self::Quarter => "quarter",
            Self::HalfYear => "half_year",
            Self::Year => "year",
        }
    }

    /// The key of `day` in this kind.
    const fn key(self, day: u32, keys: &DayKeys) -> u32 {
        match self {
            Self::Day => day,
            Self::Week => keys.week,
            Self::Fortnight => keys.fortnight,
            Self::Month => keys.month,
            Self::Quarter => keys.quarter,
            Self::HalfYear => keys.half_year,
            Self::Year => keys.year,
        }
    }
}

/// How many period kinds a scorecard tabulates.
pub const PERIOD_KINDS: usize = PeriodKind::ALL.len();

/// The observed tradable days of a run, by IST civil day: O(1) per lookup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Calendar {
    /// The civil day of the first entry.
    first_day: u32,
    /// Each civil day's keys, or `None` for a day that is not an observed
    /// tradable day.
    days: Vec<Option<DayKeys>>,
}

impl Calendar {
    /// A calendar from `first_day` onwards, one entry per civil day.
    ///
    /// # Errors
    ///
    /// [`ScoreRefusal::CalendarIndexNotIncreasing`] and
    /// [`ScoreRefusal::CalendarKeyDecreasing`].
    pub fn new(first_day: u32, days: Vec<Option<DayKeys>>) -> Result<Self, ScoreRefusal> {
        let mut before: Option<(u32, DayKeys)> = None;
        for (day, keys) in (first_day..).zip(&days) {
            let Some(keys) = keys else {
                continue;
            };
            if let Some((_, earlier)) = before {
                if keys.tradable_index <= earlier.tradable_index {
                    return Err(ScoreRefusal::CalendarIndexNotIncreasing { day });
                }
                for kind in PeriodKind::ALL {
                    if kind.key(day, keys) < kind.key(day, &earlier) {
                        return Err(ScoreRefusal::CalendarKeyDecreasing {
                            day,
                            period: kind.name(),
                        });
                    }
                }
            }
            before = Some((day, *keys));
        }
        Ok(Self { first_day, days })
    }

    /// The keys of `day`, or `None` when it is not an observed tradable day.
    #[must_use]
    pub fn keys(&self, day: u32) -> Option<&DayKeys> {
        let at = day.checked_sub(self.first_day)?;
        self.days.get(at as usize)?.as_ref()
    }
}

/// The report ladders of the run's Generation version (R4): rungs of net R
/// for winning trades and of MFE in R for every trade, each strictly
/// increasing and exact. Report only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rungs {
    /// The net-R rungs.
    net_r: Vec<Ratio>,
    /// The MFE rungs, in R.
    mfe_r: Vec<Ratio>,
}

impl Rungs {
    /// The two ladders.
    ///
    /// # Errors
    ///
    /// [`ScoreRefusal::RungsNotIncreasing`] naming the ladder.
    pub fn new(net_r: Vec<Ratio>, mfe_r: Vec<Ratio>) -> Result<Self, ScoreRefusal> {
        for (ladder, rungs) in [("net_r", &net_r), ("mfe_r", &mfe_r)] {
            let increasing = rungs
                .iter()
                .zip(rungs.iter().skip(1))
                .all(|(a, b)| a.cmp_exact(*b).is_lt());
            if !increasing {
                return Err(ScoreRefusal::RungsNotIncreasing { ladder });
            }
        }
        Ok(Self { net_r, mfe_r })
    }

    /// How many rungs of `ladder` are at or below `num / den` (`den > 0`),
    /// compared exactly: the highest rung reached, plus one; zero below the
    /// first.
    fn reached(ladder: &[Ratio], num: i128, den: i128) -> usize {
        ladder.partition_point(|rung| {
            let left = num.saturating_mul(i128::from(rung.den()));
            let right = i128::from(rung.num()).saturating_mul(den);
            left >= right
        })
    }
}

/// Why scoring refused. Every refusal is named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScoreRefusal {
    /// A pass with no profile.
    ProfilesEmpty,
    /// Not one charge model per profile.
    ChargesPerProfile {
        /// Profiles.
        profiles: usize,
        /// Charge models.
        charges: usize,
    },
    /// A profile's variant is not one the table scores.
    ProfileOutsideExits {
        /// The variant.
        variant: u8,
    },
    /// A table of another shape than the pass was built for.
    TableMismatch {
        /// What differs.
        what: &'static str,
    },
    /// A row index past the table.
    RowOutOfTable {
        /// The row.
        row: u32,
    },
    /// An outcome index past the outcome records.
    OutcomeOutOfRange {
        /// The index.
        index: u32,
        /// Records given.
        outcomes: usize,
    },
    /// A row on a day that is not an observed tradable day.
    DayNotTradable {
        /// The day.
        day: u32,
    },
    /// A priced slot with no share filled.
    PricedWithoutQty {
        /// The row.
        row: u32,
    },
    /// A priced slot whose risk per share is not positive: R is undefined.
    RiskNotPositive {
        /// The row.
        row: u32,
    },
    /// A priced slot whose entry value is not positive.
    EntryValueNotPositive {
        /// The row.
        row: u32,
    },
    /// Money past `i128`.
    MoneyOverflow {
        /// The row.
        row: u32,
    },
    /// A calendar whose tradable index does not increase.
    CalendarIndexNotIncreasing {
        /// The day.
        day: u32,
    },
    /// A calendar whose period key decreases.
    CalendarKeyDecreasing {
        /// The day.
        day: u32,
        /// The period kind.
        period: &'static str,
    },
    /// A ladder that is not strictly increasing.
    RungsNotIncreasing {
        /// The ladder.
        ladder: &'static str,
    },
}

impl ScoreRefusal {
    /// The refusal's stable name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::ProfilesEmpty => "profiles_empty",
            Self::ChargesPerProfile { .. } => "charges_per_profile",
            Self::ProfileOutsideExits { .. } => "profile_outside_exits",
            Self::TableMismatch { .. } => "table_mismatch",
            Self::RowOutOfTable { .. } => "row_out_of_table",
            Self::OutcomeOutOfRange { .. } => "outcome_out_of_range",
            Self::DayNotTradable { .. } => "day_not_tradable",
            Self::PricedWithoutQty { .. } => "priced_without_qty",
            Self::RiskNotPositive { .. } => "risk_not_positive",
            Self::EntryValueNotPositive { .. } => "entry_value_not_positive",
            Self::MoneyOverflow { .. } => "money_overflow",
            Self::CalendarIndexNotIncreasing { .. } => "calendar_index_not_increasing",
            Self::CalendarKeyDecreasing { .. } => "calendar_key_decreasing",
            Self::RungsNotIncreasing { .. } => "rungs_not_increasing",
        }
    }
}

impl core::fmt::Display for ScoreRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "fib score refused ({}): {self:?}", self.name())
    }
}

impl std::error::Error for ScoreRefusal {}

/// One side value's trades in a month.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SideStats {
    /// Trades.
    pub trades: u64,
    /// Their net R.
    pub sum_r: Stat,
}

/// One month of one profile: every figure the gate and the report read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MonthStats {
    /// Trades taken (after one position per stock).
    pub trades: u64,
    /// Trades whose net money is above zero.
    pub wins: u64,
    /// The rest: zero is a loss.
    pub losses: u64,
    /// Net R.
    pub sum_r: Stat,
    /// Net R of the wins.
    pub sum_win_r: Stat,
    /// Net R of the losses.
    pub sum_loss_r: Stat,
    /// Net R squared.
    pub sum_r_squared: Stat,
    /// The largest net R.
    pub max_r: Option<Stat>,
    /// The smallest net R.
    pub min_r: Option<Stat>,
    /// Net money of the wins, in the charge model's unit.
    pub win_money: i128,
    /// Net money of the losses, in the charge model's unit.
    pub loss_money: i128,
    /// The days' rounding adjustments, in the charge model's unit.
    pub adjustment_money: i128,
    /// Per side value: trades and net R.
    pub per_side: [SideStats; SIDE_VALUES],
    /// Stops in bp of the entry price, summed: a measurement and the ranking
    /// column, never a filter.
    pub sum_stop_bp: Stat,
    /// The smallest stop in bp.
    pub min_stop_bp: Option<Stat>,
    /// Adverse excursion to the exit, in R, summed.
    pub sum_mae_r: Stat,
    /// Favourable excursion to the exit, in R, summed.
    pub sum_mfe_r: Stat,
    /// Unpriced slots taken (raw rows that add no R).
    pub unpriced: u64,
}

impl MonthStats {
    /// An empty month.
    pub const EMPTY: Self = Self {
        trades: 0,
        wins: 0,
        losses: 0,
        sum_r: ZERO,
        sum_win_r: ZERO,
        sum_loss_r: ZERO,
        sum_r_squared: ZERO,
        max_r: None,
        min_r: None,
        win_money: 0,
        loss_money: 0,
        adjustment_money: 0,
        per_side: [SideStats {
            trades: 0,
            sum_r: ZERO,
        }; SIDE_VALUES],
        sum_stop_bp: ZERO,
        min_stop_bp: None,
        sum_mae_r: ZERO,
        sum_mfe_r: ZERO,
        unpriced: 0,
    };
}

/// What happened to every raw row of one profile's pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PassCounts {
    /// Raw rows: the mix's support.
    pub raw: u64,
    /// Trades taken.
    pub trades: u64,
    /// Skipped while a priced position was open.
    pub blocked_while_open: u64,
    /// Skipped while an order was armed or a window open.
    pub blocked_by_pending: u64,
    /// Skipped after an unpriced slot that day.
    pub blocked_by_unpriced: u64,
    /// Skipped candidates whose decision second equals the blocker's.
    pub ties: u64,
    /// Pending slots taken (no fill).
    pub unfilled: u64,
    /// Unpriced slots taken.
    pub unpriced: u64,
    /// Zero-quantity slots.
    pub zero_qty: u64,
    /// Inapplicable variants.
    pub inapplicable: u64,
}

impl PassCounts {
    /// Nothing counted.
    pub const ZERO: Self = Self {
        raw: 0,
        trades: 0,
        blocked_while_open: 0,
        blocked_by_pending: 0,
        blocked_by_unpriced: 0,
        ties: 0,
        unfilled: 0,
        unpriced: 0,
        zero_qty: 0,
        inapplicable: 0,
    };
}

/// Streaks in entry order (D-6403): information.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Streaks {
    /// Consecutive wins ending at the last trade.
    pub wins_now: u64,
    /// Consecutive losses ending at the last trade.
    pub losses_now: u64,
    /// The longest run of wins.
    pub longest_wins: u64,
    /// The longest run of losses.
    pub longest_losses: u64,
    /// Net R of the losing run ending at the last trade.
    pub losing_r_now: Stat,
    /// The most negative net R of any losing run.
    pub deepest_losing_r: Stat,
}

impl Streaks {
    /// No trade yet.
    pub const ZERO: Self = Self {
        wins_now: 0,
        losses_now: 0,
        longest_wins: 0,
        longest_losses: 0,
        losing_r_now: ZERO,
        deepest_losing_r: ZERO,
    };

    /// Adds one trade.
    fn add(&mut self, win: bool, r: Stat) {
        if win {
            self.wins_now += 1;
            self.longest_wins = self.longest_wins.max(self.wins_now);
            self.losses_now = 0;
            self.losing_r_now = ZERO;
        } else {
            self.losses_now += 1;
            self.longest_losses = self.longest_losses.max(self.losses_now);
            self.losing_r_now += r;
            self.deepest_losing_r = self.deepest_losing_r.min(self.losing_r_now);
            self.wins_now = 0;
        }
    }
}

/// The drawdown of the day-close equity in R (D-6403): information.
///
/// An episode starts at the first day close below the running high and ends
/// at the first later close at or above it. Its length is in observed
/// tradable days: recovery index minus start index, or, while open, the last
/// close's index minus the start index plus one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drawdown {
    /// The equity at the last close.
    pub equity_r: Stat,
    /// The running high (never below zero).
    pub high_r: Stat,
    /// The deepest fall below a high.
    pub max_drawdown_r: Stat,
    /// The day the deepest episode started.
    pub max_drawdown_start_day: Option<u32>,
    /// The day of its trough.
    pub max_drawdown_trough_day: Option<u32>,
    /// The day it recovered, if it did.
    pub max_drawdown_recovery_day: Option<u32>,
    /// Tradable days from its trough to its recovery, if it did.
    pub max_drawdown_recovery_days: Option<u32>,
    /// The longest episode, in tradable days.
    pub longest_underwater_days: u32,
    /// The open episode's length so far (zero when none is open).
    pub underwater_now_days: u32,
}

impl Drawdown {
    /// No close yet.
    pub const ZERO: Self = Self {
        equity_r: ZERO,
        high_r: ZERO,
        max_drawdown_r: ZERO,
        max_drawdown_start_day: None,
        max_drawdown_trough_day: None,
        max_drawdown_recovery_day: None,
        max_drawdown_recovery_days: None,
        longest_underwater_days: 0,
        underwater_now_days: 0,
    };
}

/// One period kind's table: information.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PeriodStats {
    /// Periods with a trade.
    pub periods: u64,
    /// Of them, net R above zero.
    pub winning: u64,
    /// Below zero.
    pub losing: u64,
    /// Exactly zero.
    pub flat: u64,
    /// The worst period's net R.
    pub worst_r: Option<Stat>,
    /// The best period's net R.
    pub best_r: Option<Stat>,
    /// The second-best period's net R (the leave-one-out's second maximum).
    pub second_best_r: Option<Stat>,
}

impl PeriodStats {
    /// No period yet.
    pub const ZERO: Self = Self {
        periods: 0,
        winning: 0,
        losing: 0,
        flat: 0,
        worst_r: None,
        best_r: None,
        second_best_r: None,
    };

    /// Adds one closed period.
    fn close(&mut self, r: Stat) {
        self.periods += 1;
        if r > ZERO {
            self.winning += 1;
        } else if r < ZERO {
            self.losing += 1;
        } else {
            self.flat += 1;
        }
        self.worst_r = Some(self.worst_r.map_or(r, |worst| worst.min(r)));
        match self.best_r {
            Some(best) if r <= best => {
                self.second_best_r = Some(self.second_best_r.map_or(r, |second| second.max(r)));
            }
            other => {
                self.second_best_r = other;
                self.best_r = Some(r);
            }
        }
    }
}

/// One profile's result for one mix.
#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    /// The profile.
    pub profile: Profile,
    /// One entry per run month.
    pub months: Vec<MonthStats>,
    /// Per (month, net-R rung): winning trades whose highest rung reached is
    /// this one; month-major.
    pub rung_net: Vec<u64>,
    /// Their net R, in the same layout.
    pub rung_net_r: Vec<Stat>,
    /// Per (month, MFE rung): trades whose highest MFE rung reached is this
    /// one; month-major.
    pub rung_mfe: Vec<u64>,
    /// What happened to every raw row.
    pub counts: PassCounts,
    /// Streaks.
    pub streaks: Streaks,
    /// Drawdown.
    pub drawdown: Drawdown,
    /// One table per [`PeriodKind::ALL`] entry.
    pub periods: Vec<PeriodStats>,
}

/// Appends a statistic's bits.
fn push_stat(out: &mut Vec<u8>, value: Stat) {
    out.extend_from_slice(&value.to_bits().to_le_bytes());
}

/// Appends an optional statistic: a presence byte, then its bits.
fn push_option(out: &mut Vec<u8>, value: Option<Stat>) {
    match value {
        None => out.push(0),
        Some(value) => {
            out.push(1);
            push_stat(out, value);
        }
    }
}

/// Appends an optional day.
fn push_day(out: &mut Vec<u8>, value: Option<u32>) {
    match value {
        None => out.push(0),
        Some(value) => {
            out.push(1);
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
}

impl Card {
    /// The card's canonical bytes, little-endian: what "byte-identical" is
    /// measured on.
    pub fn push_bytes(&self, out: &mut Vec<u8>) {
        out.push(self.profile.variant);
        out.extend_from_slice(&(size_index(self.profile.size) as u64).to_le_bytes());
        for month in &self.months {
            for count in [month.trades, month.wins, month.losses, month.unpriced] {
                out.extend_from_slice(&count.to_le_bytes());
            }
            for stat in [
                month.sum_r,
                month.sum_win_r,
                month.sum_loss_r,
                month.sum_r_squared,
                month.sum_stop_bp,
                month.sum_mae_r,
                month.sum_mfe_r,
            ] {
                push_stat(out, stat);
            }
            for option in [month.max_r, month.min_r, month.min_stop_bp] {
                push_option(out, option);
            }
            for money in [month.win_money, month.loss_money, month.adjustment_money] {
                out.extend_from_slice(&money.to_le_bytes());
            }
            for side in &month.per_side {
                out.extend_from_slice(&side.trades.to_le_bytes());
                push_stat(out, side.sum_r);
            }
        }
        for count in self.rung_net.iter().chain(&self.rung_mfe) {
            out.extend_from_slice(&count.to_le_bytes());
        }
        for stat in &self.rung_net_r {
            push_stat(out, *stat);
        }
        let c = &self.counts;
        for count in [
            c.raw,
            c.trades,
            c.blocked_while_open,
            c.blocked_by_pending,
            c.blocked_by_unpriced,
            c.ties,
            c.unfilled,
            c.unpriced,
            c.zero_qty,
            c.inapplicable,
        ] {
            out.extend_from_slice(&count.to_le_bytes());
        }
        let s = &self.streaks;
        for count in [s.wins_now, s.losses_now, s.longest_wins, s.longest_losses] {
            out.extend_from_slice(&count.to_le_bytes());
        }
        push_stat(out, s.losing_r_now);
        push_stat(out, s.deepest_losing_r);
        let d = &self.drawdown;
        for stat in [d.equity_r, d.high_r, d.max_drawdown_r] {
            push_stat(out, stat);
        }
        for day in [
            d.max_drawdown_start_day,
            d.max_drawdown_trough_day,
            d.max_drawdown_recovery_day,
            d.max_drawdown_recovery_days,
        ] {
            push_day(out, day);
        }
        out.extend_from_slice(&d.longest_underwater_days.to_le_bytes());
        out.extend_from_slice(&d.underwater_now_days.to_le_bytes());
        for period in &self.periods {
            for count in [period.periods, period.winning, period.losing, period.flat] {
                out.extend_from_slice(&count.to_le_bytes());
            }
            for option in [period.worst_r, period.best_r, period.second_best_r] {
                push_option(out, option);
            }
        }
    }
}

/// What a (profile, stock) block slot was written by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Blocker {
    /// A priced position.
    Open,
    /// An armed order or an open window.
    Pending,
    /// An unpriced slot.
    Unpriced,
}

/// One epoch-stamped (profile, stock) slot (FM-F050): live only when its pass
/// and day match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Block {
    /// The pass that wrote it.
    epoch: u64,
    /// The day it was written on.
    day: u32,
    /// The last second it blocks.
    until: u32,
    /// The blocker's decision second.
    decided: u32,
    /// What wrote it.
    by: Blocker,
}

/// An open drawdown episode.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Episode {
    /// The day it started.
    start_day: u32,
    /// Its tradable index.
    start_index: u32,
    /// Its lowest equity.
    trough_r: Stat,
    /// The day of that trough.
    trough_day: u32,
    /// Its tradable index.
    trough_index: u32,
}

/// One profile's running state inside a pass.
#[derive(Clone, Debug)]
struct Lane<C> {
    /// The variant's offset inside the table's exits.
    offset: usize,
    /// The size's index in [`SIZES`].
    size: usize,
    /// The charge model.
    charges: C,
    /// The months touched this pass.
    touched: Vec<u32>,
    /// The pass each month was last touched in.
    stamp: Vec<u64>,
    /// The day's net R so far.
    day_r: Stat,
    /// Whether the day has a trade.
    day_traded: bool,
    /// The open drawdown episode.
    episode: Option<Episode>,
    /// Whether the deepest episode is the open one.
    deepest_is_open: bool,
    /// The last close's tradable index.
    last_index: u32,
    /// The current period of each kind: its key and its net R so far.
    current: Vec<Option<(u32, Stat)>>,
}

impl<C> Lane<C> {
    /// Records that `month` is written in pass `epoch`, once per pass, so the
    /// next pass empties it: the touched list (FM-F050).
    fn touch(&mut self, month: u32, epoch: u64) {
        let at = month as usize;
        if self.stamp.get(at) != Some(&epoch) {
            if let Some(stamp) = self.stamp.get_mut(at) {
                *stamp = epoch;
            }
            self.touched.push(month);
        }
    }
}

/// The ordered pass over one mix's rows for a set of profiles: pre-sized once,
/// reused for every mix.
#[derive(Clone, Debug)]
pub struct OrderedPass<C> {
    /// The table's search.
    search: Search,
    /// The table's first exit variant.
    first_exit: u8,
    /// The run's months.
    months: usize,
    /// The pool the block slots are sized for.
    pool: usize,
    /// The ladders.
    rungs: Rungs,
    /// One lane per profile.
    lanes: Vec<Lane<C>>,
    /// One card per profile.
    cards: Vec<Card>,
    /// The (profile, stock) block slots, profile-major.
    block: Vec<Block>,
    /// The current pass.
    epoch: u64,
}

/// The empty block slot.
const NO_BLOCK: Block = Block {
    epoch: 0,
    day: 0,
    until: 0,
    decided: 0,
    by: Blocker::Open,
};

/// What every lane of one pass reads alike.
struct Ctx<'r> {
    /// The table's search.
    search: Search,
    /// The current pass.
    epoch: u64,
    /// The pool the block slots are sized for.
    pool: usize,
    /// The ladders.
    rungs: &'r Rungs,
}

/// One trade's figures, computed once and added to its month.
struct Trade {
    /// Net money, in the model's unit.
    net: i128,
    /// Net R.
    r: Stat,
    /// The stop in bp of the entry price.
    stop_bp: Stat,
    /// Adverse excursion, in R.
    mae_r: Stat,
    /// Favourable excursion, in R.
    mfe_r: Stat,
    /// Highest net-R rung reached, plus one (zero for none or a loss).
    net_rung: usize,
    /// Highest MFE rung reached, plus one.
    mfe_rung: usize,
}

impl<C: ChargeModel> OrderedPass<C> {
    /// A pass over `table`'s mixes for `profiles`, one charge model each.
    ///
    /// # Errors
    ///
    /// [`ScoreRefusal::ProfilesEmpty`], [`ScoreRefusal::ChargesPerProfile`] and
    /// [`ScoreRefusal::ProfileOutsideExits`].
    pub fn new(
        table: &RowTable,
        profiles: &[Profile],
        rungs: Rungs,
        charges: Vec<C>,
    ) -> Result<Self, ScoreRefusal> {
        if profiles.is_empty() {
            return Err(ScoreRefusal::ProfilesEmpty);
        }
        if charges.len() != profiles.len() {
            return Err(ScoreRefusal::ChargesPerProfile {
                profiles: profiles.len(),
                charges: charges.len(),
            });
        }
        let exits = &table.partition().exits;
        let months = table.months() as usize;
        let pool = table.pool() as usize;
        let mut lanes = Vec::with_capacity(profiles.len());
        let mut cards = Vec::with_capacity(profiles.len());
        for (profile, charges) in profiles.iter().zip(charges) {
            if profile.variant < exits.start || profile.variant >= exits.end {
                return Err(ScoreRefusal::ProfileOutsideExits {
                    variant: profile.variant,
                });
            }
            lanes.push(Lane {
                offset: usize::from(profile.variant - exits.start),
                size: size_index(profile.size),
                charges,
                touched: Vec::with_capacity(months),
                stamp: vec![0; months],
                day_r: ZERO,
                day_traded: false,
                episode: None,
                deepest_is_open: false,
                last_index: 0,
                current: vec![None; PERIOD_KINDS],
            });
            cards.push(Card {
                profile: *profile,
                months: vec![MonthStats::EMPTY; months],
                rung_net: vec![0; months * rungs.net_r.len()],
                rung_net_r: vec![ZERO; months * rungs.net_r.len()],
                rung_mfe: vec![0; months * rungs.mfe_r.len()],
                counts: PassCounts::ZERO,
                streaks: Streaks::ZERO,
                drawdown: Drawdown::ZERO,
                periods: vec![PeriodStats::ZERO; PERIOD_KINDS],
            });
        }
        Ok(Self {
            search: table.partition().search,
            first_exit: exits.start,
            months,
            pool,
            rungs,
            lanes,
            cards,
            block: vec![NO_BLOCK; profiles.len().saturating_mul(pool)],
            epoch: 0,
        })
    }

    /// The profiles, in lane order.
    pub fn profiles(&self) -> impl Iterator<Item = Profile> + '_ {
        self.cards.iter().map(|card| card.profile)
    }

    /// Empties every card through its touched months: O(touched + periods).
    fn reset(&mut self) {
        let net = self.rungs.net_r.len();
        let mfe = self.rungs.mfe_r.len();
        for (lane, card) in self.lanes.iter_mut().zip(&mut self.cards) {
            for month in lane.touched.drain(..) {
                let at = month as usize;
                if let Some(stats) = card.months.get_mut(at) {
                    *stats = MonthStats::EMPTY;
                }
                for slot in card.rung_net.iter_mut().skip(at * net).take(net) {
                    *slot = 0;
                }
                for slot in card.rung_net_r.iter_mut().skip(at * net).take(net) {
                    *slot = ZERO;
                }
                for slot in card.rung_mfe.iter_mut().skip(at * mfe).take(mfe) {
                    *slot = 0;
                }
            }
            card.counts = PassCounts::ZERO;
            card.streaks = Streaks::ZERO;
            card.drawdown = Drawdown::ZERO;
            card.periods.fill(PeriodStats::ZERO);
            lane.day_r = ZERO;
            lane.day_traded = false;
            lane.episode = None;
            lane.deepest_is_open = false;
            lane.last_index = 0;
            lane.current.fill(None);
        }
    }

    /// Scores one mix: its rows, ascending, against the outcome records and
    /// the calendar. The cards are the pass's own and are overwritten by the
    /// next mix.
    ///
    /// # Errors
    ///
    /// By name: a table of another shape, a row past the table, an outcome
    /// index past the records, a day not in the calendar, a priced slot with no
    /// share, a risk or an entry value that is not positive, money past
    /// `i128`. A refused mix leaves the cards partly written; the next call
    /// resets them.
    pub fn score(
        &mut self,
        table: &RowTable,
        rows: &[u32],
        outcomes: &[Outcome],
        calendar: &Calendar,
    ) -> Result<&[Card], ScoreRefusal> {
        if table.months() as usize != self.months {
            return Err(ScoreRefusal::TableMismatch { what: "months" });
        }
        if table.pool() as usize > self.pool {
            return Err(ScoreRefusal::TableMismatch { what: "pool" });
        }
        if table.partition().search != self.search
            || table.partition().exits.start != self.first_exit
        {
            return Err(ScoreRefusal::TableMismatch { what: "partition" });
        }
        self.reset();
        self.epoch += 1;
        let mut today: Option<(u32, u32, DayKeys)> = None;
        for row in rows {
            let view = table
                .row(*row)
                .ok_or(ScoreRefusal::RowOutOfTable { row: *row })?;
            if today.is_none_or(|(day, _, _)| day != view.day) {
                if let Some((day, month, keys)) = today {
                    self.close_day(day, month, &keys);
                }
                let keys = calendar
                    .keys(view.day)
                    .ok_or(ScoreRefusal::DayNotTradable { day: view.day })?;
                today = Some((view.day, view.month, *keys));
            }
            let ctx = Ctx {
                search: self.search,
                epoch: self.epoch,
                pool: self.pool,
                rungs: &self.rungs,
            };
            for (index, (lane, card)) in self.lanes.iter_mut().zip(&mut self.cards).enumerate() {
                Self::offer(
                    &ctx,
                    &mut self.block,
                    index,
                    lane,
                    card,
                    *row,
                    &view,
                    outcomes,
                )?;
            }
        }
        if let Some((day, month, keys)) = today {
            self.close_day(day, month, &keys);
        }
        self.finish();
        Ok(&self.cards)
    }

    /// Offers one row to one profile's lane.
    #[allow(
        clippy::too_many_arguments,
        reason = "the pass's shared reading, the lane's slots, its state and card, and the row"
    )]
    fn offer(
        ctx: &Ctx<'_>,
        block: &mut [Block],
        lane_index: usize,
        lane: &mut Lane<C>,
        card: &mut Card,
        row: u32,
        view: &RowView<'_>,
        outcomes: &[Outcome],
    ) -> Result<(), ScoreRefusal> {
        let index = view
            .outcomes
            .get(lane.offset)
            .copied()
            .ok_or(ScoreRefusal::RowOutOfTable { row })?;
        let slot = outcomes
            .get(index as usize)
            .and_then(|outcome| outcome.slots.get(lane.size))
            .copied()
            .ok_or(ScoreRefusal::OutcomeOutOfRange {
                index,
                outcomes: outcomes.len(),
            })?;
        let at = lane_index * ctx.pool + view.slot as usize;
        let current = block.get(at).copied().unwrap_or(NO_BLOCK);
        let blocked =
            current.epoch == ctx.epoch && current.day == view.day && view.second <= current.until;
        card.counts.raw += 1;
        if blocked {
            let counts = &mut card.counts;
            match current.by {
                Blocker::Open => counts.blocked_while_open += 1,
                Blocker::Pending => counts.blocked_by_pending += 1,
                Blocker::Unpriced => counts.blocked_by_unpriced += 1,
            }
            counts.ties += u64::from(view.second == current.decided);
            return Ok(());
        }
        let written = |until, by| Block {
            epoch: ctx.epoch,
            day: view.day,
            until,
            decided: view.second,
            by,
        };
        let next = match slot {
            Slot::Priced(outcome) => {
                Self::take(ctx, lane, card, row, view, &outcome)?;
                Some(written(outcome.exit_second, Blocker::Open))
            }
            Slot::Pending { until_second } => {
                card.counts.unfilled += 1;
                Some(written(until_second, Blocker::Pending))
            }
            Slot::Unpriced(_) => {
                card.counts.unpriced += 1;
                if let Some(stats) = card.months.get_mut(view.month as usize) {
                    stats.unpriced += 1;
                }
                lane.touch(view.month, ctx.epoch);
                Some(written(u32::MAX, Blocker::Unpriced))
            }
            Slot::ZeroQty => {
                card.counts.zero_qty += 1;
                None
            }
            Slot::Inapplicable => {
                card.counts.inapplicable += 1;
                None
            }
        };
        if let (Some(next), Some(slot)) = (next, block.get_mut(at)) {
            *slot = next;
        }
        Ok(())
    }

    /// Prices one trade through the lane's charge model.
    fn price(
        ctx: &Ctx<'_>,
        lane: &mut Lane<C>,
        row: u32,
        view: &RowView<'_>,
        outcome: &SlotOutcome,
    ) -> Result<Trade, ScoreRefusal> {
        if outcome.filled_qty == 0 {
            return Err(ScoreRefusal::PricedWithoutQty { row });
        }
        if outcome.risk_per_share_paisa <= 0 {
            return Err(ScoreRefusal::RiskNotPositive { row });
        }
        if outcome.entry_value_paisa <= 0 {
            return Err(ScoreRefusal::EntryValueNotPositive { row });
        }
        let long = ctx.search == Search::Long;
        let gross = if long {
            outcome
                .exit_value_paisa
                .checked_sub(outcome.entry_value_paisa)
        } else {
            outcome
                .entry_value_paisa
                .checked_sub(outcome.exit_value_paisa)
        }
        .ok_or(ScoreRefusal::MoneyOverflow { row })?;
        let order = |buy, value_paisa, broker_square_off| Order {
            stock: view.slot,
            day: view.day,
            buy,
            qty: outcome.filled_qty,
            value_paisa,
            broker_square_off,
        };
        let entry = order(long, outcome.entry_value_paisa, false);
        let exit = order(
            !long,
            outcome.exit_value_paisa,
            outcome.flags.contains(TradeFlag::DhanSquareOff),
        );
        let scale = lane.charges.scale();
        let charges = lane
            .charges
            .add(&entry)
            .checked_add(lane.charges.add(&exit))
            .ok_or(ScoreRefusal::MoneyOverflow { row })?;
        let qty = i128::from(outcome.filled_qty);
        let risk = i128::from(outcome.risk_per_share_paisa);
        let net = gross
            .checked_mul(scale)
            .and_then(|scaled| scaled.checked_sub(charges))
            .ok_or(ScoreRefusal::MoneyOverflow { row })?;
        let den = scale
            .checked_mul(qty)
            .and_then(|scaled| scaled.checked_mul(risk))
            .ok_or(ScoreRefusal::MoneyOverflow { row })?;
        let r = net as Stat / den as Stat;
        let bp = i128::from(BP_PER_UNIT);
        let stop = risk.saturating_mul(qty).saturating_mul(bp);
        let mfe = i128::from(outcome.mfe_to_exit_paisa);
        Ok(Trade {
            net,
            r,
            stop_bp: stop as Stat / outcome.entry_value_paisa as Stat,
            mae_r: outcome.mae_to_exit_paisa as Stat / risk as Stat,
            mfe_r: mfe as Stat / risk as Stat,
            net_rung: if net > 0 {
                Rungs::reached(&ctx.rungs.net_r, net, den)
            } else {
                0
            },
            mfe_rung: Rungs::reached(&ctx.rungs.mfe_r, mfe, risk),
        })
    }

    /// Takes one trade: prices it and adds it to its month, its streaks and
    /// its day.
    fn take(
        ctx: &Ctx<'_>,
        lane: &mut Lane<C>,
        card: &mut Card,
        row: u32,
        view: &RowView<'_>,
        outcome: &SlotOutcome,
    ) -> Result<(), ScoreRefusal> {
        let trade = Self::price(ctx, lane, row, view, outcome)?;
        let net_rungs = ctx.rungs.net_r.len();
        let mfe_rungs = ctx.rungs.mfe_r.len();
        let month = view.month as usize;
        lane.touch(view.month, ctx.epoch);
        let win = trade.net > 0;
        let r = trade.r;
        card.counts.trades += 1;
        if let Some(stats) = card.months.get_mut(month) {
            stats.trades += 1;
            stats.sum_r += r;
            stats.sum_r_squared += r * r;
            stats.max_r = Some(stats.max_r.map_or(r, |max| max.max(r)));
            stats.min_r = Some(stats.min_r.map_or(r, |min| min.min(r)));
            if win {
                stats.wins += 1;
                stats.sum_win_r += r;
                stats.win_money = stats.win_money.saturating_add(trade.net);
            } else {
                stats.losses += 1;
                stats.sum_loss_r += r;
                stats.loss_money = stats.loss_money.saturating_add(trade.net);
            }
            let side = usize::from(
                view.coords
                    .get(RowId::Side.index())
                    .copied()
                    .unwrap_or(OPEN),
            );
            if let Some(side) = stats.per_side.get_mut(side) {
                side.trades += 1;
                side.sum_r += r;
            }
            stats.sum_stop_bp += trade.stop_bp;
            stats.min_stop_bp = Some(
                stats
                    .min_stop_bp
                    .map_or(trade.stop_bp, |min| min.min(trade.stop_bp)),
            );
            stats.sum_mae_r += trade.mae_r;
            stats.sum_mfe_r += trade.mfe_r;
        }
        if let Some(rung) = trade.net_rung.checked_sub(1) {
            let at = month * net_rungs + rung;
            if let Some(count) = card.rung_net.get_mut(at) {
                *count += 1;
            }
            if let Some(sum) = card.rung_net_r.get_mut(at) {
                *sum += r;
            }
        }
        if let Some(rung) = trade.mfe_rung.checked_sub(1)
            && let Some(count) = card.rung_mfe.get_mut(month * mfe_rungs + rung)
        {
            *count += 1;
        }
        card.streaks.add(win, r);
        lane.day_r += r;
        lane.day_traded = true;
        Ok(())
    }

    /// Closes one day for every lane: the charge model's adjustment, then
    /// drawdown and the period tables on a day that had a trade.
    fn close_day(&mut self, day: u32, month: u32, keys: &DayKeys) {
        for (lane, card) in self.lanes.iter_mut().zip(&mut self.cards) {
            let adjustment = lane.charges.close_day(day);
            if adjustment != 0 {
                let at = month as usize;
                lane.touch(month, self.epoch);
                if let Some(stats) = card.months.get_mut(at) {
                    stats.adjustment_money = stats.adjustment_money.saturating_add(adjustment);
                }
            }
            if !lane.day_traded {
                continue;
            }
            let r = lane.day_r;
            lane.day_r = ZERO;
            lane.day_traded = false;
            Self::draw(lane, &mut card.drawdown, day, keys.tradable_index, r);
            for ((kind, current), table) in PeriodKind::ALL
                .iter()
                .zip(lane.current.iter_mut())
                .zip(card.periods.iter_mut())
            {
                let key = kind.key(day, keys);
                match current {
                    Some((open, sum)) if *open == key => *sum += r,
                    _ => {
                        if let Some((_, sum)) = *current {
                            table.close(sum);
                        }
                        *current = Some((key, r));
                    }
                }
            }
        }
    }

    /// Adds one day close to the drawdown.
    fn draw(lane: &mut Lane<C>, drawdown: &mut Drawdown, day: u32, index: u32, r: Stat) {
        drawdown.equity_r += r;
        lane.last_index = index;
        let equity = drawdown.equity_r;
        if equity >= drawdown.high_r {
            if let Some(episode) = lane.episode.take() {
                let length = index.saturating_sub(episode.start_index);
                drawdown.longest_underwater_days = drawdown.longest_underwater_days.max(length);
                if lane.deepest_is_open {
                    drawdown.max_drawdown_recovery_day = Some(day);
                    drawdown.max_drawdown_recovery_days =
                        Some(index.saturating_sub(episode.trough_index));
                    lane.deepest_is_open = false;
                }
            }
            drawdown.high_r = equity;
            return;
        }
        let episode = lane.episode.get_or_insert(Episode {
            start_day: day,
            start_index: index,
            trough_r: equity,
            trough_day: day,
            trough_index: index,
        });
        if equity < episode.trough_r {
            episode.trough_r = equity;
            episode.trough_day = day;
            episode.trough_index = index;
        }
        let fall = drawdown.high_r - episode.trough_r;
        if fall > drawdown.max_drawdown_r {
            drawdown.max_drawdown_r = fall;
            drawdown.max_drawdown_start_day = Some(episode.start_day);
            drawdown.max_drawdown_trough_day = Some(episode.trough_day);
            drawdown.max_drawdown_recovery_day = None;
            drawdown.max_drawdown_recovery_days = None;
            lane.deepest_is_open = true;
        }
    }

    /// Closes the open period of every kind and measures the open episode.
    fn finish(&mut self) {
        for (lane, card) in self.lanes.iter_mut().zip(&mut self.cards) {
            for (current, table) in lane.current.iter_mut().zip(card.periods.iter_mut()) {
                if let Some((_, sum)) = current.take() {
                    table.close(sum);
                }
            }
            if let Some(episode) = lane.episode {
                let open = lane.last_index.saturating_sub(episode.start_index) + 1;
                card.drawdown.underwater_now_days = open;
                card.drawdown.longest_underwater_days =
                    card.drawdown.longest_underwater_days.max(open);
            }
        }
    }
}

/// A visitor that scores every node it is handed and keeps each (mix, card)
/// as canonical bytes: the lattice's own sink, and the form a parallel walk is
/// proved byte-identical in.
#[derive(Debug)]
pub struct CardSink<'a, C> {
    /// The pass.
    pass: OrderedPass<C>,
    /// The charge models a fork starts from, untouched.
    template: Vec<C>,
    /// The outcome records.
    outcomes: &'a [Outcome],
    /// The calendar.
    calendar: &'a Calendar,
    /// Every kept (mix, card), as bytes, in visit order.
    bytes: Vec<u8>,
    /// Mixes kept.
    mixes: u64,
    /// Every refused mix, in visit order.
    refusals: Vec<(MixKey, ScoreRefusal)>,
    /// One flag per exit value: the variants a guided visit asked for, set and
    /// cleared around each visit.
    asked: Vec<bool>,
}

impl<'a, C: ChargeModel + Clone> CardSink<'a, C> {
    /// A sink over `table`'s mixes for `profiles`.
    ///
    /// # Errors
    ///
    /// Whatever [`OrderedPass::new`] refuses.
    pub fn new(
        table: &RowTable,
        profiles: &[Profile],
        rungs: Rungs,
        charges: Vec<C>,
        outcomes: &'a [Outcome],
        calendar: &'a Calendar,
    ) -> Result<Self, ScoreRefusal> {
        let template = charges.clone();
        let pass = OrderedPass::new(table, profiles, rungs, charges)?;
        Ok(Self {
            pass,
            template,
            outcomes,
            calendar,
            bytes: Vec::new(),
            mixes: 0,
            refusals: Vec::new(),
            asked: vec![false; EXIT_VALUES],
        })
    }

    /// Every kept (mix, card), as bytes, in visit order.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Mixes kept.
    #[must_use]
    pub const fn mixes(&self) -> u64 {
        self.mixes
    }

    /// Every refused mix.
    #[must_use]
    pub fn refusals(&self) -> &[(MixKey, ScoreRefusal)] {
        &self.refusals
    }
}

impl<C: ChargeModel + Clone> Visitor for CardSink<'_, C> {
    fn visit(&mut self, node: &Node<'_>) {
        let cards = match self
            .pass
            .score(node.table(), node.rows(), self.outcomes, self.calendar)
        {
            Ok(cards) => cards,
            Err(refusal) => {
                self.refusals.push((*node.key(), refusal));
                return;
            }
        };
        let only = match node.wanted() {
            Wanted::Every => None,
            Wanted::Only(variants) => Some(variants),
        };
        for variant in only.unwrap_or_default() {
            if let Some(flag) = self.asked.get_mut(usize::from(*variant)) {
                *flag = true;
            }
        }
        for card in cards {
            let variant = card.profile.variant;
            let asked = only.is_none()
                || self
                    .asked
                    .get(usize::from(variant))
                    .copied()
                    .unwrap_or(false);
            let Some(mix) = node.mix(variant).filter(|_| asked) else {
                continue;
            };
            self.bytes.extend_from_slice(&mix.0);
            card.push_bytes(&mut self.bytes);
            self.mixes += 1;
        }
        for variant in only.unwrap_or_default() {
            if let Some(flag) = self.asked.get_mut(usize::from(*variant)) {
                *flag = false;
            }
        }
    }

    fn fork(&self) -> Self {
        Self {
            pass: self.pass.clone_fresh(self.template.clone()),
            template: self.template.clone(),
            outcomes: self.outcomes,
            calendar: self.calendar,
            bytes: Vec::new(),
            mixes: 0,
            refusals: Vec::new(),
            asked: vec![false; EXIT_VALUES],
        }
    }

    fn absorb(&mut self, later: Self) {
        self.bytes.extend_from_slice(&later.bytes);
        self.mixes += later.mixes;
        self.refusals.extend(later.refusals);
    }
}

impl<C: ChargeModel> OrderedPass<C> {
    /// A pass of the same shape with fresh charge models and empty scratch.
    fn clone_fresh(&self, charges: Vec<C>) -> Self {
        let mut lanes = Vec::with_capacity(self.lanes.len());
        for (lane, charges) in self.lanes.iter().zip(charges) {
            lanes.push(Lane {
                offset: lane.offset,
                size: lane.size,
                charges,
                touched: Vec::with_capacity(self.months),
                stamp: vec![0; self.months],
                day_r: ZERO,
                day_traded: false,
                episode: None,
                deepest_is_open: false,
                last_index: 0,
                current: vec![None; PERIOD_KINDS],
            });
        }
        let mut cards = self.cards.clone();
        for card in &mut cards {
            card.months.fill(MonthStats::EMPTY);
            card.rung_net.fill(0);
            card.rung_net_r.fill(ZERO);
            card.rung_mfe.fill(0);
        }
        Self {
            search: self.search,
            first_exit: self.first_exit,
            months: self.months,
            pool: self.pool,
            rungs: self.rungs.clone(),
            lanes,
            cards,
            block: vec![NO_BLOCK; self.block.len()],
            epoch: 0,
        }
    }
}
