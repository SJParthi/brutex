//! The walker: ONE pure walker for both stages (FIB-5; D-6380 to D-6385).
//!
//! Stage A walks each plan on the Zerodha one-minute bars of the day
//! ([`Resolution::Minute`]); stage DEEP re-walks gate-A survivors on the GDFL
//! one-second bars of the continuous phase ([`Resolution::Second`]). Both run
//! the same code: a [`DayIndex`] built ONCE per stock-day, then one small
//! state machine per slot — a plan, an exit variant, a fill variant and a
//! trade size — stepped bar by bar from its entry window to its exit. Each step
//! does a fixed number of comparisons whatever the hold length or the number
//! of active plans: O(1) per second per open position, measured at p99 by
//! `crates/runner/benches/fib_rows/walk.rs` and proved equal to single-slot
//! walks by `crates/runner/tests/fib_walk.rs`.
//!
//! # The rules (D-6381, D-6382)
//!
//! * FILLS at the worst price of each bar, always: a buy at the bar's high, a
//!   sell at its low. Stage A fills the whole quantity in one minute
//!   (labelled: "stage A assumes the whole quantity fills at the minute's
//!   worst printed price"); a second supplies at most its own volume, the rest
//!   spilling into later seconds.
//! * The FORCED exit is priced only from GDFL seconds inside
//!   `[15:09:00, 15:10:00)` (long: the lowest low; short: the highest high;
//!   capacity: the window's summed volume), or at stage A from the 15:09
//!   minute alone. No bar at or after 15:10:00 is ever read: [`DayIndex`]
//!   drops them when it is built.
//! * No look-ahead: a level is frozen at the decision, a trailing level made
//!   by bar `t` applies from bar `t + 1`, and nothing a walk computes feeds a
//!   coordinate.
//! * Long and short are separate setups; only the geometry is mirrored. The
//!   index stores each side's bars mirrored (a short's prices negated), so one
//!   state machine walks both and the mirror is exact by construction.
//! * Every level is compared EXACTLY: decision-basis levels convert to the
//!   execution feed's raw prices by DATA-1's factor pair through a common
//!   denominator `zerodha_close x 1000 x 1000`, so no level is ever rounded
//!   (D-6383).
//!
//! This module does no I/O and its loops call nothing (gates 17 and 22).

pub mod exits;
mod index;
mod slot;

#[cfg(test)]
mod tests;

pub use index::DayIndex;
pub use slot::Stepper;

use indicators::Candle;
use indicators::fibflag::types::{EntryStyle, FibCandle};
use vocab::fib_cube::Search;

use self::exits::{ExitSet, FillVariant};
use super::types::{SlotOutcome, TradeSize, WalkProfile};

pub use super::types::Resolution;

/// One plan: a decision the walker fills and exits. Every price is in the
/// DECISION basis (Zerodha paisa), frozen at the decision; the walker converts
/// and offsets it. Built by FIB-9 from FIB-4's candidates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Plan {
    /// The trade's direction.
    pub direction: Search,
    /// The entry style.
    pub style: EntryStyle,
    /// The plan's timeframe, in whole minutes.
    pub tf_minutes: u16,
    /// The decision instant: the decision candle's close, in seconds after IST
    /// midnight.
    pub decision_second: u32,
    /// The close of the next candle of the plan's timeframe: the end of the
    /// entry window under [`exits::EntryWindow::NextClose`] and at stage A
    /// (exclusive).
    pub entry_window_end: u32,
    /// The entry reference: the trigger close (CLOSE), the break level (BREAK)
    /// or the retest level (RETEST). R's risk is measured from it.
    pub entry_ref: i64,
    /// The base's far edge. The planned stop sits exactly one tick beyond it
    /// (stage A: one decision tick; deep: the converted edge less one raw
    /// tick), never twice (FM-F033).
    pub base_edge: i64,
    /// BREAK: the trigger candle's extreme, offset outward by one tick into the
    /// break level. RETEST: the level a retest must reach. CLOSE: unread.
    pub trigger_edge: i64,
    /// The day's Fibonacci level 0 measured in the trade's direction at the
    /// decision (a long: the day low).
    pub fib_zero: i64,
    /// The day's Fibonacci level 1000 measured the same way (a long: the day
    /// high).
    pub fib_one: i64,
    /// Which of [`DayInput::trail`] holds this plan's own candles.
    pub trail: u16,
}

/// A recorded halt, `[start_second, end_second)` after IST midnight (CAL-1's
/// halt authority).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HaltRange {
    /// The first halted second.
    pub start_second: u32,
    /// The first second after the halt.
    pub end_second: u32,
}

/// One bar's order-book evidence (OF-1) and print flag (PREC-3), deep only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BookMark {
    /// An ask was present: a buy could take liquidity.
    pub ask: bool,
    /// A bid was present: a sell could take liquidity.
    pub bid: bool,
    /// PREC-3 flagged the print.
    pub flagged_print: bool,
}

/// Whether the day's book evidence was supplied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BookEvidence<'a> {
    /// One mark per bar, in bar order.
    Observed(&'a [BookMark]),
    /// None was supplied: the one-sided-book rule cannot apply, and the walk
    /// says so ([`DayIndex::book_observed`]). Stage A is always this.
    NotObserved,
}

/// The stock-day's signed GDFL clock lag against Zerodha (DATA-1 / PREC-13,
/// FM-F043).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockLag {
    /// Measured, in seconds.
    Measured(i32),
    /// Not measured: a deep walk is refused `clock_lag_unmeasured`.
    Unmeasured,
    /// Not measured, walked anyway under `--seconds-trust unchecked`, and
    /// labelled ([`DayIndex::seconds_trust_unchecked`]).
    TrustUnchecked,
}

/// One timeframe's decision-basis candles for the day: the trail's input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrailCandles<'a> {
    /// The width, in whole minutes.
    pub tf_minutes: u16,
    /// The candles, ascending by close.
    pub candles: &'a [FibCandle],
}

/// One stock-day as the walker reads it.
#[derive(Clone, Copy, Debug)]
pub struct DayInput<'a> {
    /// The day's bars at the walk's resolution, in the execution feed's raw
    /// paisa, ascending and unique.
    pub bars: &'a [Candle],
    /// The day's book evidence.
    pub book: BookEvidence<'a>,
    /// The day's recorded halts, ascending and disjoint (stated empty on a
    /// tradable day).
    pub halts: &'a [HaltRange],
    /// Deep: the day's Zerodha one-minute bars, the decision basis (the
    /// minute-presence bitmap and the converted ranges, FM-F041 and FM-F047).
    /// Stage A: empty (the bars are Zerodha's).
    pub zerodha_minutes: &'a [Candle],
    /// The execution feed's observed tick, in raw paisa.
    pub raw_tick: i64,
    /// The decision basis's tick, in paisa.
    pub decision_tick: i64,
    /// The largest count of consecutive missing seconds between two real
    /// seconds over the stock's committed past days (FIB-2 `MaxSecondGap`); a
    /// longer silence inside a trade is flagged.
    pub max_second_gap: u32,
    /// The stock-day's clock lag.
    pub clock_lag: ClockLag,
    /// Each timeframe's decision-basis candles.
    pub trail: &'a [TrailCandles<'a>],
}

/// Why a walk was refused, by name. APPEND-ONLY.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalkRefusal {
    /// Two bars out of order or with one stamp.
    BarsNotSortedUnique {
        /// The later bar's index.
        at: usize,
    },
    /// The bars span more than one IST day.
    BarsSpanDays,
    /// A bar stamp off its resolution's grid.
    BarOffGrid {
        /// The bar's index.
        at: usize,
    },
    /// The book marks are not one per bar.
    BookMarksLength,
    /// A halt that is empty, inverted, unsorted or overlapping.
    HaltsInvalid,
    /// A tick at or below zero.
    NonpositiveTick,
    /// A factor pair with a value at or below zero.
    NonpositiveFactor,
    /// Stage A walks the decision basis itself: its price map is the identity.
    StageATakesIdentity,
    /// The worse-of-both reading needs PREC-15's `runner::cross_feed`, which is
    /// not built (FM-F046).
    CrossFeedNotBuilt,
    /// A deep walk with no Zerodha minute for the day.
    ZerodhaMinutesAbsent,
    /// A measured non-zero clock lag under [`super::types::ClockRule::RefuseShifted`].
    ClockShifted {
        /// The lag, in seconds.
        lag: i32,
    },
    /// An unmeasured clock lag without `--seconds-trust unchecked`.
    ClockLagUnmeasured,
    /// A trail candle list with a zero width or unsorted candles.
    TrailListInvalid {
        /// The list's index.
        list: usize,
    },
    /// A price or factor too large for the exact comparison's headroom.
    PriceBeyondExactRange,
    /// A plan whose window, timeframe or trail list is impossible.
    PlanInvalid {
        /// The plan's index.
        plan: usize,
        /// What is wrong.
        why: &'static str,
    },
    /// A deep-only fill variant offered to a stage-A walk.
    FillVariantDeepOnly,
    /// A variant id the exit set does not hold.
    VariantUnknown {
        /// The id.
        id: u16,
    },
    /// A plan of the other direction than the exit set's (16:04 IST).
    DirectionMismatch {
        /// The plan's index.
        plan: usize,
    },
    /// A swing width the day was not indexed for: the index and the walk were
    /// given different exit sets.
    WidthNotIndexed {
        /// The width.
        width: u16,
    },
}

impl WalkRefusal {
    /// The refusal's canonical name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::BarsNotSortedUnique { .. } => "bars_not_sorted_unique",
            Self::BarsSpanDays => "bars_span_days",
            Self::BarOffGrid { .. } => "bar_off_grid",
            Self::BookMarksLength => "book_marks_length",
            Self::HaltsInvalid => "halts_invalid",
            Self::NonpositiveTick => "nonpositive_tick",
            Self::NonpositiveFactor => "nonpositive_factor",
            Self::StageATakesIdentity => "stage_a_takes_identity",
            Self::CrossFeedNotBuilt => "cross_feed_not_built",
            Self::ZerodhaMinutesAbsent => "zerodha_minutes_absent",
            Self::ClockShifted { .. } => "clock_shifted",
            Self::ClockLagUnmeasured => "clock_lag_unmeasured",
            Self::TrailListInvalid { .. } => "trail_list_invalid",
            Self::PriceBeyondExactRange => "price_beyond_exact_range",
            Self::PlanInvalid { .. } => "plan_invalid",
            Self::FillVariantDeepOnly => "fill_variant_deep_only",
            Self::VariantUnknown { .. } => "variant_unknown",
            Self::DirectionMismatch { .. } => "direction_mismatch",
            Self::WidthNotIndexed { .. } => "width_not_indexed",
        }
    }
}

/// One walked (plan, variant) pair: its outcome at each requested size, in
/// the sizes' order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Walked {
    /// The plan's index in the walk's plan list.
    pub plan: u32,
    /// The exit variant's id.
    pub variant: u16,
    /// The outcome at each size.
    pub slots: Vec<SlotOutcome>,
}

/// Walks every requested `(plan, variant)` pair at every size, keyed and
/// ordered by `(plan, variant)` as given. Each pair's slots are independent of
/// every other pair, so a pass of any size gives the same bytes
/// (`output_bytes_independent_of_budget`), and an incremental rerun walks only
/// the pairs it is handed (13:05 item 5).
///
/// # Errors
///
/// [`WalkRefusal::FillVariantDeepOnly`], [`WalkRefusal::VariantUnknown`],
/// [`WalkRefusal::DirectionMismatch`] or [`WalkRefusal::PlanInvalid`].
pub fn walk_pairs(
    index: &DayIndex,
    plans: &[Plan],
    exits: &ExitSet,
    fill: FillVariant,
    sizes: &[TradeSize],
    pairs: &[(u32, u16)],
) -> Result<Vec<Walked>, WalkRefusal> {
    if index.resolution() == Resolution::Minute && fill != FillVariant::STAGE_A {
        return Err(WalkRefusal::FillVariantDeepOnly);
    }
    let mut out = Vec::with_capacity(pairs.len());
    for &(plan_at, id) in pairs {
        let at = usize::try_from(plan_at).unwrap_or(usize::MAX);
        let plan = plans.get(at).ok_or(WalkRefusal::PlanInvalid {
            plan: at,
            why: "plan_index_out_of_range",
        })?;
        if plan.direction != exits.direction() {
            return Err(WalkRefusal::DirectionMismatch { plan: at });
        }
        let variant = exits.get(id).ok_or(WalkRefusal::VariantUnknown { id })?;
        let mut slots = Vec::with_capacity(sizes.len());
        for size in sizes {
            let mut stepper = Stepper::new(index, plan, at, variant, fill, *size)?;
            stepper.run();
            slots.push(stepper.outcome());
        }
        out.push(Walked {
            plan: plan_at,
            variant: id,
            slots,
        });
    }
    Ok(out)
}

/// Walks every plan against every variant of `exits`, in passes of `budget`
/// variants: pass by pass, each plan's pairs of that pass, then concatenated in
/// `(plan, variant)` order, so the result is byte-identical for every budget.
///
/// # Errors
///
/// Every refusal of [`walk_pairs`].
pub fn walk_all(
    index: &DayIndex,
    plans: &[Plan],
    exits: &ExitSet,
    fill: FillVariant,
    sizes: &[TradeSize],
    budget: exits::PassBudget,
) -> Result<Vec<Walked>, WalkRefusal> {
    let passes = exits.passes(budget);
    let mut by_pass = Vec::with_capacity(passes.len());
    for pass in &passes {
        let mut pairs = Vec::with_capacity(plans.len() * pass.len());
        for plan in 0..plans.len() {
            let plan = u32::try_from(plan).unwrap_or(u32::MAX);
            for id in pass.clone() {
                pairs.push((plan, id));
            }
        }
        by_pass.push(walk_pairs(index, plans, exits, fill, sizes, &pairs)?.into_iter());
    }
    // Each pass holds its pairs plan by plan; taking each pass's next
    // `pass.len()` pairs per plan restores `(plan, variant)` order in one sweep.
    let mut out = Vec::with_capacity(plans.len() * usize::from(exits.count()));
    for _ in 0..plans.len() {
        for (pass, walked) in passes.iter().zip(by_pass.iter_mut()) {
            out.extend(walked.by_ref().take(pass.len()));
        }
    }
    Ok(out)
}

/// The `(plan, variant)` pairs of `wanted` that `have` does not hold, in
/// `wanted`'s order: what an incremental rerun walks (13:05 item 5).
#[must_use]
pub fn missing_pairs(wanted: &[(u32, u16)], have: &[(u32, u16)]) -> Vec<(u32, u16)> {
    let mut held = std::collections::HashSet::with_capacity(have.len());
    for pair in have {
        held.insert(*pair);
    }
    wanted
        .iter()
        .filter(|pair| !held.contains(*pair))
        .copied()
        .collect()
}

/// Builds the profile's index of one stock-day; see [`DayIndex::build`].
///
/// # Errors
///
/// Every refusal of [`DayIndex::build`].
pub fn index_day(
    profile: &WalkProfile,
    input: &DayInput<'_>,
    exits: &ExitSet,
) -> Result<DayIndex, WalkRefusal> {
    DayIndex::build(profile, input, exits)
}
