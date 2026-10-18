//! One slot's walk: a plan, an exit variant, a fill variant and a size,
//! stepped bar by bar from its entry window to its exit (FIB-5; D-6380 to
//! D-6383).
//!
//! Every step reads one bar and compares it with at most one entry trigger,
//! five exit levels and the forced instant: a fixed number of comparisons,
//! whatever the hold or the number of other slots. The trailing stop's swings
//! arrive from a list the day index built once, through a cursor that only
//! moves forward, so absorbing them is amortised O(1) per bar. The forced
//! window and the excursions to 15:09:59 are one read each. Proved by
//! `crates/runner/tests/fib_walk.rs`; measured by
//! `crates/runner/benches/fib_rows/walk.rs`.
//!
//! The walk runs on the side's MIRRORED bars, so this file reads like a long:
//! a buy pays the bar's high, a sell gets its low, a stop sits below and a
//! target above. A real price is the side's `sign` times a stored one.

use brutex_core::price::PAISA_PER_RUPEE;
use indicators::fibflag::types::EntryStyle;
use vocab::fib_cube::{
    PERMILLE_PER_UNIT, SECONDS_PER_MINUTE, SESSION_FORCED_WINDOW_START_SECS, Search,
};

use super::super::types::{
    BeyondTarget, ExitFill, ExitOrder, ExitReason, HaltRule, Resolution, SlotOutcome, SlotRefusal,
    SlotStatus, TradeFlag, TradeFlags, TradeSize, UnfilledReason, UnpricedReason,
};
use super::exits::{
    EntryWindow, ExitVariant, FillVariant, Participation, StopLoss, TakeProfit, TimeExit,
    TrailingStop, TrailingTakeProfit,
};
use super::index::{Bit, DayIndex, SideBar, SideIndex};
use super::{Plan, WalkRefusal};

/// Where a slot stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Inside the entry window: filling, possibly with something filled.
    Entering,
    /// The entry is closed and the position is open.
    Holding,
    /// An exit order fired and part of it is still open (it spills, or at
    /// stage A waits for an unlocked minute).
    Exiting,
    /// Finished.
    Done,
}

/// What arms an entry: CLOSE names neither level and arms on the first
/// admitted bar; BREAK arms on the first bar whose high reaches the break
/// level; RETEST on the first bar whose low reaches the retest level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Trigger {
    /// The level a bar's high must reach.
    above: Option<i128>,
    /// The level a bar's low must reach.
    below: Option<i128>,
}

impl Trigger {
    /// Whether the trigger arms before any bar.
    fn immediate(self) -> bool {
        self.above.is_none() && self.below.is_none()
    }

    /// Whether `bar` arms it.
    fn armed_by(self, bar: &SideBar) -> bool {
        self.above.is_none_or(|level| bar.hi >= level)
            && self.below.is_none_or(|level| bar.lo <= level)
    }
}

/// One exit order being filled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Order {
    /// Shares filled.
    qty: u64,
    /// Their value, raw paisa, mirrored.
    value: i128,
    /// The last fill's second.
    last_second: u32,
}

impl Order {
    /// An empty order.
    const EMPTY: Self = Self {
        qty: 0,
        value: 0,
        last_second: 0,
    };

    /// Books `qty` shares at `price` (raw, mirrored) in `second`.
    fn book(&mut self, qty: u64, price: i64, second: u32) {
        self.qty += qty;
        self.value += i128::from(qty) * i128::from(price);
        self.last_second = second;
    }

    /// The order as an outcome reports it, real-signed.
    fn report(self, sign: i64) -> Option<ExitOrder> {
        (self.qty > 0).then(|| ExitOrder {
            last_fill_second: self.last_second,
            filled_qty: self.qty,
            exit_value_paisa: self.value * i128::from(sign),
        })
    }
}

/// A plan's levels under one variant, frozen at the decision, mirrored and on
/// the common denominator.
#[derive(Clone, Copy, Debug)]
struct Levels {
    /// What arms the entry.
    trigger: Trigger,
    /// The entry reference.
    entry: i128,
    /// The planned stop: R's risk is measured to it.
    planned_stop: i128,
    /// The planned risk.
    risk: i128,
    /// The planned risk per share, decision paisa.
    risk_decision: i128,
    /// The stop-loss level, when the variant has one.
    stop: Option<i128>,
    /// The take-profit level, when the variant has one (and it is not void).
    target: Option<i128>,
    /// Stage A: the take-profit's resting limit price on the tick grid, raw.
    target_limit: Option<i64>,
    /// The trailing take-profit's give-back, permille.
    giveback: Option<i128>,
    /// The R-step.
    rstep: Option<i128>,
    /// The swing list and the buffer.
    swing: Option<(usize, i128)>,
}

/// `value / PERMILLE_PER_UNIT` of a product that divides exactly.
fn per_mille(value: i128) -> i128 {
    value / i128::from(PERMILLE_PER_UNIT)
}

/// The plan's levels under `variant` (D-6382, D-6383): the planned stop one
/// tick beyond the base's far edge, the break level one tick beyond the
/// trigger's extreme, and each target frozen at the decision.
fn levels(
    day: &DayIndex,
    side: &SideIndex,
    plan: &Plan,
    variant: &ExitVariant,
) -> Result<Levels, WalkRefusal> {
    let sign = side.sign;
    let mirror = |z: i64| i128::from(z) * i128::from(sign);
    let g = day.decision_scale;
    let entry = mirror(plan.entry_ref) * g;
    let planned_stop = mirror(plan.base_edge) * g - day.tick;
    let risk_decision =
        mirror(plan.entry_ref) - mirror(plan.base_edge) + i128::from(day.decision_tick);
    let risk = risk_decision * g;
    let edge = mirror(plan.trigger_edge) * g;
    let trigger = match plan.style {
        EntryStyle::Close => Trigger {
            above: None,
            below: None,
        },
        EntryStyle::Break => Trigger {
            above: Some(edge + day.tick),
            below: None,
        },
        EntryStyle::Retest => Trigger {
            above: None,
            below: Some(edge),
        },
    };
    let stop = match variant.stop {
        StopLoss::Absent => None,
        StopLoss::BeyondBase { extra_ticks } => {
            Some(planned_stop - i128::from(extra_ticks) * day.tick)
        }
    };
    let p = i128::from(PERMILLE_PER_UNIT);
    let rung_level = |rung: i128| {
        let zero = mirror(plan.fib_zero);
        (zero * p + (mirror(plan.fib_one) - zero) * rung) * per_mille(g)
    };
    let target = match variant.take_profit {
        TakeProfit::Absent => None,
        TakeProfit::RMultiple { milli_r } => Some(entry + per_mille(risk * i128::from(milli_r))),
        TakeProfit::FibRung { rung_permille } => Some(rung_level(i128::from(rung_permille))),
        TakeProfit::NextRung => indicators::fib::RUNGS
            .iter()
            .map(|rung| rung_level(i128::from(*rung)))
            .find(|level| *level > entry),
    };
    let target_limit = target.and_then(|t| {
        let grid = i128::from(day.decision_tick) * day.raw_scale;
        let ticks = t.div_euclid(grid) + i128::from(t.rem_euclid(grid) != 0);
        i64::try_from(ticks * i128::from(day.decision_tick)).ok()
    });
    let giveback = match variant.trailing_take_profit {
        TrailingTakeProfit::Absent => None,
        TrailingTakeProfit::GiveBack { permille } => Some(i128::from(permille)),
    };
    let (rstep, swing) = match variant.trailing_stop {
        TrailingStop::Absent => (None, None),
        TrailingStop::RStep { milli_r } => (Some(per_mille(risk * i128::from(milli_r))), None),
        TrailingStop::Swing {
            width,
            buffer_paisa,
        } => {
            let mut at = None;
            for (k, indexed) in day.widths.iter().enumerate() {
                if *indexed == width {
                    at = Some(k);
                }
            }
            let at = at.ok_or(WalkRefusal::WidthNotIndexed { width })?;
            let list = usize::from(plan.trail) * day.widths.len() + at;
            (None, Some((list, i128::from(buffer_paisa) * g)))
        }
    };
    Ok(Levels {
        trigger,
        entry,
        planned_stop,
        risk,
        risk_decision,
        stop,
        target,
        target_limit,
        giveback,
        rstep,
        swing,
    })
}

/// The quantities a size buys (FM-F034): `floor(size / entry_ref)` on the
/// decision basis, and in deep `floor(size x zerodha_close / (entry_ref x
/// gdfl_close))` raw shares, both in `i128` with no division by zero.
fn quantities(day: &DayIndex, plan: &Plan, size: TradeSize) -> (u64, u64) {
    let size_paisa = i128::from(size.rupees()) * i128::from(PAISA_PER_RUPEE);
    let entry_ref = i128::from(plan.entry_ref);
    let decision_qty = u64::try_from(size_paisa / entry_ref).unwrap_or(u64::MAX);
    if day.resolution() == Resolution::Minute {
        return (decision_qty, decision_qty);
    }
    let num = size_paisa * i128::from(day.zerodha_close);
    let den = entry_ref * i128::from(day.gdfl_close);
    (decision_qty, u64::try_from(num / den).unwrap_or(u64::MAX))
}

/// The entry window `[start, end)`: the decision plus the snapshot edge and the
/// re-stamp in deep; ending at the next close at stage A and under reading 1,
/// at 15:09:00 under reading 2, at the deadline under a deadline; never past
/// 15:09:00.
fn window(day: &DayIndex, plan: &Plan, fill: FillVariant) -> (u32, u32) {
    let forced_start = u32::try_from(SESSION_FORCED_WINDOW_START_SECS).unwrap_or(u32::MAX);
    if day.resolution() == Resolution::Minute {
        return (
            plan.decision_second,
            plan.entry_window_end.min(forced_start),
        );
    }
    let end = match fill.window {
        EntryWindow::NextClose => plan.entry_window_end,
        EntryWindow::UntilForced => forced_start,
        EntryWindow::Deadline { seconds } => plan.decision_second.saturating_add(seconds),
    };
    (
        plan.decision_second.saturating_add(day.entry_shift),
        end.min(forced_start),
    )
}

/// One slot, stepped bar by bar. Public so the bench can time one step; the
/// walker drives it through [`super::walk_pairs`].
#[derive(Clone, Debug)]
pub struct Stepper<'a> {
    /// The day.
    day: &'a DayIndex,
    /// The plan's side of the day.
    side: &'a SideIndex,
    /// The plan's direction.
    direction: Search,
    /// The plan's levels.
    levels: Levels,
    /// The next bar to read.
    cursor: usize,
    /// Where the slot stands.
    phase: Phase,
    /// What became of it, once [`Phase::Done`].
    status: SlotStatus,
    /// The first second a fill may happen in.
    entry_start: u32,
    /// The entry window's end (exclusive).
    entry_end: u32,
    /// Whether the entry is armed.
    armed: bool,
    /// The quantity at the decision basis.
    decision_qty: u64,
    /// The quantity to fill, raw shares.
    planned_qty: u64,
    /// The next swing event to read.
    swing_cursor: usize,
    /// The time exit as the variant states it.
    time_exit: TimeExit,
    /// The time exit's instant, once known.
    time_at: Option<u32>,
    /// The participation cap, permille.
    cap: Option<u64>,
    /// Shares still to fill.
    remaining: u64,
    /// Shares filled.
    filled: u64,
    /// The entry's value, raw, mirrored.
    entry_value: i128,
    /// The first fill: its second, its price (raw, mirrored) and its bar.
    first: Option<(u32, i64, usize)>,
    /// The last entry fill's second.
    last_entry_second: u32,
    /// Shares held.
    open: u64,
    /// The own exit order.
    own: Order,
    /// The forced order.
    forced: Order,
    /// The own exit's reason.
    reason: Option<ExitReason>,
    /// The second the own exit fired in.
    fired_at: Option<u32>,
    /// The trailing stop's level, applying from the next bar.
    trail: Option<i128>,
    /// The trailing take-profit's level, applying from the next bar.
    ttp: Option<i128>,
    /// The best buy-side price since the first fill, scaled.
    best: i128,
    /// The same, raw.
    best_raw: i64,
    /// The worst sell-side price since the first fill, raw.
    worst_raw: i64,
    /// The first second the best price printed.
    mfe_second: u32,
    /// The last second it was equalled.
    last_peak_second: u32,
    /// Flags.
    flags: TradeFlags,
    /// Seconds an exit order spilled past its touching bar.
    spill_seconds: u32,
    /// The largest per-second participation, permille.
    max_participation: u32,
    /// The previous bar this slot read, for silences.
    previous: Option<u32>,
    /// The next fill or exit is the first print after a halt (resume rule).
    after_halt: bool,
    /// Bars stepped and swings absorbed: the structural work counter.
    work: u64,
}

impl<'a> Stepper<'a> {
    /// A slot of `plan` (the walk's plan number `plan_at`) under `variant`,
    /// `fill` and `size`. A slot refused by the market's own numbers (a size
    /// below one share, a nonpositive planned risk, a plan after a halt start)
    /// is returned finished with that status; a plan that cannot exist is an
    /// error.
    ///
    /// # Errors
    ///
    /// [`WalkRefusal::PlanInvalid`] for a nonpositive entry reference, a zero
    /// timeframe, a window that ends at or before the decision, or a trail
    /// list that is not the plan's timeframe; [`WalkRefusal::WidthNotIndexed`]
    /// for a swing width the day was not indexed for.
    pub fn new(
        day: &'a DayIndex,
        plan: &Plan,
        plan_at: usize,
        variant: &ExitVariant,
        fill: FillVariant,
        size: TradeSize,
    ) -> Result<Self, WalkRefusal> {
        let invalid = |why| WalkRefusal::PlanInvalid { plan: plan_at, why };
        if plan.entry_ref <= 0 {
            return Err(invalid("nonpositive_entry_ref"));
        }
        if plan.tf_minutes == 0 {
            return Err(invalid("zero_timeframe"));
        }
        if plan.entry_window_end <= plan.decision_second {
            return Err(invalid("window_ends_at_decision"));
        }
        if day.trail_tfs.get(usize::from(plan.trail)) != Some(&plan.tf_minutes) {
            return Err(invalid("trail_list_not_the_plan_s_timeframe"));
        }
        let book = day.side(plan.direction);
        let levels = levels(day, book, plan, variant)?;
        let (decision_qty, planned_qty) = quantities(day, plan, size);
        let (entry_start, entry_end) = window(day, plan, fill);
        let start_minute =
            usize::try_from(u64::from(entry_start) / SECONDS_PER_MINUTE).unwrap_or(usize::MAX);
        let swing_cursor = levels
            .swing
            .and_then(|(list, _)| book.swings.get(list))
            .and_then(|list| list.by_minute.get(start_minute).copied())
            .and_then(|at| usize::try_from(at).ok())
            .unwrap_or(0);
        let mut stepper = Self {
            day,
            side: book,
            direction: plan.direction,
            levels,
            cursor: day.first_bar_at(entry_start),
            phase: Phase::Entering,
            status: SlotStatus::Priced,
            entry_start,
            entry_end,
            armed: levels.trigger.immediate(),
            decision_qty,
            planned_qty,
            swing_cursor,
            time_exit: variant.time_exit,
            time_at: match variant.time_exit {
                TimeExit::Clock { second } => Some(second),
                TimeExit::Absent | TimeExit::Hold { .. } => None,
            },
            cap: match fill.participation {
                Participation::NoCap => None,
                Participation::Cap { permille } => Some(u64::from(permille)),
            },
            remaining: planned_qty,
            filled: 0,
            entry_value: 0,
            first: None,
            last_entry_second: 0,
            open: 0,
            own: Order::EMPTY,
            forced: Order::EMPTY,
            reason: None,
            fired_at: None,
            trail: None,
            ttp: None,
            best: i128::MIN,
            best_raw: i64::MIN,
            worst_raw: i64::MAX,
            mfe_second: 0,
            last_peak_second: 0,
            flags: TradeFlags::NONE,
            spill_seconds: 0,
            max_participation: 0,
            previous: None,
            after_halt: false,
            work: 0,
        };
        if levels.risk_decision <= 0 {
            stepper.finish(SlotStatus::Refused(SlotRefusal::NonpositivePlannedRisk));
        } else if planned_qty == 0 {
            stepper.finish(SlotStatus::Refused(SlotRefusal::SizeBelowOneShare));
        } else if day.profile.halt_rule == HaltRule::WorstSeenThroughHalt
            && day.halted_by(plan.decision_second)
        {
            stepper.finish(SlotStatus::NotTradable);
        }
        Ok(stepper)
    }

    /// Whether the slot is finished.
    #[must_use]
    pub fn done(&self) -> bool {
        self.phase == Phase::Done
    }

    /// Bars stepped and swings absorbed so far: the bench's structural work
    /// counter.
    #[must_use]
    pub const fn work(&self) -> u64 {
        self.work
    }

    /// Steps until the slot is finished.
    pub fn run(&mut self) {
        while self.step() {}
    }

    /// Runs the slot to its end by EVENTS (FM-F030): a held position jumps
    /// over every bar that cannot change it, straight to its next touch, time
    /// instant, swing, new high (while a give-back or an R-step trails), halt,
    /// corrupt bar, silence or the forced window, and steps that bar as
    /// [`Self::run`] would. The outcome is the same, byte for byte
    /// (`event_walk_equals_forward_pass`); the work is O(log bars) per event.
    pub fn run_events(&mut self) {
        loop {
            if self.phase == Phase::Holding && !self.after_halt {
                self.jump();
            }
            if !self.step() {
                break;
            }
        }
    }

    /// Moves a held position's cursor to its next event bar, folding the
    /// skipped bars' extremes into its excursions.
    fn jump(&mut self) {
        let bars = &self.side.bars;
        let events = &self.side.events;
        let from = self.cursor;
        let forced_start = u32::try_from(SESSION_FORCED_WINDOW_START_SECS).unwrap_or(u32::MAX);
        let mut to = events
            .next_special(from)
            .min(self.day.first_bar_at(forced_start));
        for level in [self.levels.stop, self.trail, self.ttp]
            .into_iter()
            .flatten()
        {
            to = to.min(events.low_reaches(bars, from, level));
        }
        match self.day.resolution() {
            Resolution::Second => {
                if let Some(target) = self.levels.target {
                    to = to.min(events.high_reaches(bars, from, target));
                }
            }
            Resolution::Minute => {
                if let Some(limit) = self.levels.target_limit {
                    to = to.min(events.high_above(bars, from, limit));
                }
            }
        }
        if let Some(at) = self.time_at {
            to = to.min(self.day.first_bar_at(at));
        }
        let next_swing = self
            .levels
            .swing
            .and_then(|(list, _)| self.side.swings.get(list))
            .and_then(|list| list.events.get(self.swing_cursor));
        if let Some(event) = next_swing {
            to = to.min(self.day.first_bar_at(event.confirm_second));
        }
        if self.levels.giveback.is_some() || self.levels.rstep.is_some() {
            to = to.min(events.high_above(bars, from, self.best_raw));
        }
        let Some(skipped) = events.extremes(bars, from, to) else {
            return;
        };
        if skipped.best_raw > self.best_raw {
            self.best_raw = skipped.best_raw;
            self.best = skipped.best;
            self.mfe_second = skipped.best_first;
            self.last_peak_second = skipped.best_last;
        } else if skipped.best_raw == self.best_raw {
            self.last_peak_second = skipped.best_last;
        }
        self.worst_raw = self.worst_raw.min(skipped.worst_raw);
        self.previous = to
            .checked_sub(1)
            .and_then(|k| bars.get(k))
            .map(|b| b.second);
        self.cursor = to;
        self.work += 1;
    }

    /// Ends the slot with `status`.
    fn finish(&mut self, status: SlotStatus) {
        self.status = status;
        self.phase = Phase::Done;
    }

    /// Sets `flag`.
    fn flag(&mut self, flag: TradeFlag) {
        self.flags = self.flags.with(flag);
    }

    /// Reads one bar. False once the slot is finished.
    pub fn step(&mut self) -> bool {
        if self.phase == Phase::Done {
            return false;
        }
        self.work += 1;
        let Some(bar) = self.side.bars.get(self.cursor).copied() else {
            self.forced_exit();
            return false;
        };
        if u64::from(bar.second) >= SESSION_FORCED_WINDOW_START_SECS {
            self.forced_exit();
            return false;
        }
        self.cursor += 1;
        if self.phase == Phase::Entering && bar.second >= self.entry_end {
            self.window_closed();
            if self.phase == Phase::Done {
                return false;
            }
        }
        if bar.is(Bit::HaltCrossed) && self.halted() {
            return false;
        }
        if bar.is(Bit::Halted) {
            return true;
        }
        if bar.is(Bit::Corrupt) {
            self.finish(SlotStatus::Unpriceable(UnpricedReason::CorruptBarInTrade));
            return false;
        }
        if self.day.resolution() == Resolution::Second && self.silence(&bar) {
            return false;
        }
        self.previous = Some(bar.second);
        self.absorb_swings(bar.second);
        if self.phase == Phase::Exiting {
            self.continue_exit(&bar, bar.volume);
        } else {
            // A position held from earlier bars meets this bar's exits first:
            // an exit cancels the unfilled entry remainder in this bar
            // (FM-F036). A first fill in this bar meets them after it, so an
            // entry and a stop in one bar stop in that bar.
            let held = self.filled > 0;
            if held {
                self.evaluate_exits(&bar, bar.volume);
            }
            if self.open_or_entering() {
                let volume_left = self.enter(&bar);
                if !held && self.filled > 0 {
                    self.evaluate_exits(&bar, volume_left);
                }
            }
            if self.open_or_entering() && self.filled > 0 {
                self.update_levels(&bar);
            }
        }
        self.after_halt = false;
        self.phase != Phase::Done
    }

    /// Whether the slot is filling or holding, with no exit order open.
    fn open_or_entering(&self) -> bool {
        matches!(self.phase, Phase::Entering | Phase::Holding)
    }

    /// The entry window ended before this bar. In deep, an order that filled
    /// nothing while the window ran into a feed hole is
    /// `entry_unfilled_feed_hole`: its fate there is unknown (FM-F041).
    fn window_closed(&mut self) {
        if self.filled == 0 && self.day.resolution() == Resolution::Second {
            let from = self
                .previous
                .map_or(self.entry_start, |p| p.saturating_add(1))
                .max(self.entry_start);
            let missing = self.entry_end.saturating_sub(from);
            if missing > self.day.max_gap
                && self
                    .day
                    .traded_between(from, self.entry_end.saturating_sub(1))
            {
                self.finish(SlotStatus::EntryUnfilled(UnfilledReason::FeedHole));
                return;
            }
        }
        self.close_window();
    }

    /// The entry's part of a bar: arm, then fill what the bar supplies.
    /// Returns the volume the bar has left for an exit.
    fn enter(&mut self, bar: &SideBar) -> u64 {
        if self.phase != Phase::Entering {
            return bar.volume;
        }
        if !self.armed {
            self.armed = self.levels.trigger.armed_by(bar);
        }
        if !self.armed || self.remaining == 0 {
            return bar.volume;
        }
        let capacity = self.entry_capacity(bar);
        if capacity == 0 && self.filled == 0 && self.target_touched(bar) {
            self.finish(SlotStatus::EntryUnfilled(UnfilledReason::TargetBeforeFill));
            return bar.volume;
        }
        let qty = self.remaining.min(capacity);
        if qty > 0 {
            self.book_entry(bar, qty);
        }
        if self.remaining == 0 {
            self.phase = Phase::Holding;
        }
        bar.volume.saturating_sub(qty)
    }

    /// The entry window closed: nothing filled ends the slot; a part filled
    /// cancels the rest.
    fn close_window(&mut self) {
        if self.filled == 0 {
            let why = if self.armed {
                UnfilledReason::WindowExpired
            } else {
                UnfilledReason::NeverTriggered
            };
            self.finish(SlotStatus::EntryUnfilled(why));
        } else {
            if self.remaining > 0 {
                self.flag(TradeFlag::PartialEntry);
                self.remaining = 0;
            }
            self.phase = Phase::Holding;
        }
    }

    /// A recorded halt started before this bar. Under the halt-day rule an
    /// open position is unpriced and kept (`AT_WORST_SEEN` prices it) and an
    /// unfilled order is not tradable; under the resume reading the next
    /// print carries the halt. True when the slot finished.
    fn halted(&mut self) -> bool {
        match self.day.profile.halt_rule {
            HaltRule::WorstSeenThroughHalt => {
                if self.filled == 0 {
                    self.finish(SlotStatus::NotTradable);
                } else {
                    if self.remaining > 0 {
                        self.flag(TradeFlag::PartialEntry);
                        self.remaining = 0;
                    }
                    self.flag(TradeFlag::Halt);
                    self.finish(SlotStatus::Unpriceable(UnpricedReason::Halt));
                }
                true
            }
            HaltRule::ResumeAtFirstPrint => {
                self.after_halt = true;
                false
            }
        }
    }

    /// The silence before this bar, inside the slot's window or trade
    /// (FM-F041). True when the slot finished.
    fn silence(&mut self, bar: &SideBar) -> bool {
        let (missing, hole) = if self.previous.is_some() {
            (bar.gap_missing, bar.is(Bit::GapHole))
        } else {
            let missing = bar.second.saturating_sub(self.entry_start);
            let hole = missing > 0
                && self
                    .day
                    .traded_between(self.entry_start, bar.second.saturating_sub(1));
            (missing, hole)
        };
        if missing <= self.day.max_gap {
            return false;
        }
        if hole {
            let status = if self.filled == 0 {
                SlotStatus::EntryUnfilled(UnfilledReason::FeedHole)
            } else {
                SlotStatus::Unpriceable(UnpricedReason::ExecutionFeedHole)
            };
            self.finish(status);
            return true;
        }
        self.flag(TradeFlag::SilenceInTrade);
        false
    }

    /// Reads every swing confirmed by now; one formed after the first fill
    /// raises the trail (from this bar on, so never inside the bar that made
    /// it), and a hole is flagged. Before the first fill, swings pass unread.
    fn absorb_swings(&mut self, second: u32) {
        let Some((list, buffer)) = self.levels.swing else {
            return;
        };
        let events = self
            .side
            .swings
            .get(list)
            .map_or(&[][..], |l| l.events.as_slice());
        while let Some(event) = events.get(self.swing_cursor).copied() {
            if event.confirm_second > second {
                break;
            }
            self.swing_cursor += 1;
            self.work += 1;
            let Some((first, _, _)) = self.first else {
                continue;
            };
            if event.candle_close_second <= first {
                continue;
            }
            match event.level {
                Some(level) => {
                    let level = level - buffer;
                    self.trail = Some(self.trail.map_or(level, |t| t.max(level)));
                }
                None => self.flag(TradeFlag::TrailHole),
            }
        }
    }

    /// Shares this bar can supply to the entry.
    fn entry_capacity(&mut self, bar: &SideBar) -> u64 {
        match self.day.resolution() {
            Resolution::Minute => {
                if bar.is(Bit::EntryLocked) {
                    self.flag(TradeFlag::LockedMinuteSuspected);
                    0
                } else {
                    self.remaining
                }
            }
            Resolution::Second => {
                if !bar.is(Bit::EntryOk) {
                    self.flag(TradeFlag::OneSidedBook);
                    return 0;
                }
                self.cap.map_or(bar.volume, |permille| {
                    let capped = u128::from(bar.volume) * u128::from(permille)
                        / u128::from(PERMILLE_PER_UNIT);
                    u64::try_from(capped).unwrap_or(u64::MAX)
                })
            }
        }
    }

    /// Whether the take-profit is touched in this bar by its resolution's
    /// rule: a second's high reaching it, a minute's high trading through the
    /// limit.
    fn target_touched(&self, bar: &SideBar) -> bool {
        match self.day.resolution() {
            Resolution::Second => self.levels.target.is_some_and(|t| bar.hi >= t),
            Resolution::Minute => self
                .levels
                .target_limit
                .is_some_and(|limit| bar.hi_raw > limit),
        }
    }

    /// Books `qty` entry shares at this bar's worst buy price.
    fn book_entry(&mut self, bar: &SideBar, qty: u64) {
        if self.first.is_none() {
            self.first = Some((bar.second, bar.hi_raw, self.cursor - 1));
            self.best = bar.hi;
            self.best_raw = bar.hi_raw;
            self.mfe_second = bar.second;
            self.last_peak_second = bar.second;
            if let TimeExit::Hold { seconds } = self.time_exit {
                self.time_at = Some(bar.second.saturating_add(seconds));
            }
        }
        self.filled += qty;
        self.open += qty;
        self.remaining -= qty;
        self.entry_value += i128::from(qty) * i128::from(bar.hi_raw);
        self.last_entry_second = bar.second;
        if self.day.resolution() == Resolution::Second && bar.volume > 0 {
            let permille = u128::from(qty) * u128::from(PERMILLE_PER_UNIT) / u128::from(bar.volume);
            self.max_participation = self
                .max_participation
                .max(u32::try_from(permille).unwrap_or(u32::MAX));
        }
        self.mark_fill(bar, bar.hi);
        if bar.hi <= self.levels.planned_stop {
            self.flag(TradeFlag::EntryBeyondStop);
        }
        let Some(target) = self.levels.target else {
            return;
        };
        let beyond = self
            .entry_value
            .checked_mul(self.day.raw_scale)
            .zip(target.checked_mul(i128::from(self.filled)))
            .is_none_or(|(value, line)| value >= line);
        if beyond {
            self.flag(TradeFlag::EntryBeyondTarget);
            if self.day.profile.beyond_target == BeyondTarget::TargetVoid {
                self.levels.target = None;
                self.levels.target_limit = None;
            }
        }
    }

    /// The flags of a fill at `price` (scaled) in this bar.
    fn mark_fill(&mut self, bar: &SideBar, price: i128) {
        if self.after_halt {
            self.flag(TradeFlag::CircuitHalt);
        }
        if bar.is(Bit::Flagged) {
            self.flag(TradeFlag::FlaggedPrintFill);
        }
        if self.day.resolution() == Resolution::Second
            && (!bar.is(Bit::Witnessed) || price < bar.witness_lo || price > bar.witness_hi)
        {
            self.flag(TradeFlag::CrossFeedOutlier);
        }
    }

    /// Fires the exit components this bar reaches, by the fixed precedence
    /// SL > TSL > TTP > TP > time, flagging every one (FM-F037).
    fn evaluate_exits(&mut self, bar: &SideBar, volume_left: u64) {
        let stop = self.levels.stop.filter(|s| bar.lo <= *s);
        let trail = self.trail.filter(|t| bar.lo <= *t);
        let giveback = self.ttp.filter(|t| bar.lo <= *t);
        let target = self.target_touched(bar);
        let time = self.time_at.is_some_and(|at| bar.second >= at);
        let fired = [
            (ExitReason::StopLoss, stop.is_some()),
            (ExitReason::TrailingStopLoss, trail.is_some()),
            (ExitReason::TrailingTakeProfit, giveback.is_some()),
            (ExitReason::TakeProfit, target),
            (ExitReason::Time, time),
        ];
        let mut reason = None;
        for (component, hit) in fired {
            if hit {
                reason = reason.or(Some(component));
                if let Some(flag) = TradeFlag::fired(component) {
                    self.flag(flag);
                }
            }
        }
        let Some(reason) = reason else {
            return;
        };
        for level in [stop, trail].into_iter().flatten() {
            if bar.op < level || self.after_halt {
                self.flag(TradeFlag::GapThroughStop);
            }
        }
        if self.day.resolution() == Resolution::Second {
            self.mark_deep_touch(
                bar,
                [stop, trail, giveback].into_iter().flatten().max(),
                target,
            );
            if reason == ExitReason::TakeProfit {
                self.flag(TradeFlag::TargetBookedAtWorst);
            }
        }
        self.reason = Some(reason);
        self.fired_at = Some(bar.second);
        if self.phase == Phase::Entering && self.remaining > 0 {
            self.flag(TradeFlag::EntryCancelledByExit);
            self.remaining = 0;
        }
        self.phase = Phase::Exiting;
        let limit_only = reason == ExitReason::TakeProfit && !time;
        match (self.day.resolution(), self.levels.target_limit) {
            (Resolution::Minute, Some(limit)) if limit_only => {
                self.exit_shares(bar, self.open, limit);
            }
            _ => self.continue_exit(bar, volume_left),
        }
    }

    /// Flags a deep touch the bar's own Zerodha minute did not witness
    /// (FM-F041): a low touch below the converted minute, or a target touch
    /// above it.
    fn mark_deep_touch(&mut self, bar: &SideBar, low_level: Option<i128>, target: bool) {
        let witnessed = bar.is(Bit::Witnessed);
        let low = low_level.is_some_and(|l| !witnessed || bar.witness_lo > l);
        let high = target
            && self
                .levels
                .target
                .is_some_and(|t| !witnessed || bar.witness_hi < t);
        if low || high {
            self.flag(TradeFlag::TouchUnwitnessed);
        }
    }

    /// Fills what this bar can of the open exit order at its worst sell price.
    fn continue_exit(&mut self, bar: &SideBar, volume_left: u64) {
        let qty = match self.day.resolution() {
            Resolution::Minute => {
                if bar.is(Bit::ExitLocked) {
                    self.flag(TradeFlag::LockedMinuteSuspected);
                    0
                } else {
                    self.open
                }
            }
            Resolution::Second => match self.day.profile.exit_fill {
                ExitFill::FirstTouchWhole => self.open,
                ExitFill::SpillByVolume => {
                    if bar.is(Bit::ExitOk) {
                        self.open.min(volume_left)
                    } else {
                        self.flag(TradeFlag::OneSidedBook);
                        0
                    }
                }
            },
        };
        if self.fired_at.is_some_and(|at| bar.second > at) {
            self.spill_seconds = self.spill_seconds.saturating_add(1);
        }
        if qty > 0 {
            self.exit_shares(bar, qty, bar.lo_raw);
        }
    }

    /// Books `qty` exit shares of the own order at `price` (raw, mirrored).
    fn exit_shares(&mut self, bar: &SideBar, qty: u64, price: i64) {
        self.own.book(qty, price, bar.second);
        self.open -= qty;
        self.mark_fill(bar, i128::from(price) * self.day.raw_scale);
        self.track(bar);
        if self.open == 0 {
            self.finish(SlotStatus::Priced);
        }
    }

    /// The excursions through this bar.
    fn track(&mut self, bar: &SideBar) {
        if bar.hi_raw > self.best_raw {
            self.best_raw = bar.hi_raw;
            self.best = bar.hi;
            self.mfe_second = bar.second;
            self.last_peak_second = bar.second;
        } else if bar.hi_raw == self.best_raw {
            self.last_peak_second = bar.second;
        }
        self.worst_raw = self.worst_raw.min(bar.lo_raw);
    }

    /// The trailing levels this bar makes, applying from the next bar.
    fn update_levels(&mut self, bar: &SideBar) {
        self.track(bar);
        let gain = self.best - self.levels.entry;
        if gain <= 0 {
            return;
        }
        if let Some(permille) = self.levels.giveback {
            self.ttp = Some(self.best - per_mille(gain * permille));
        }
        if let Some(step) = self.levels.rstep {
            let steps = gain / step;
            if steps > 0 {
                let level = self.levels.planned_stop + steps * step;
                self.trail = Some(self.trail.map_or(level, |t| t.max(level)));
            }
        }
    }

    /// The forced exit, priced only from `[15:09:00, 15:10:00)` (D-6381):
    /// deep, the window's worst price if its summed volume holds the open
    /// quantity, else the whole slot is unpriceable; stage A, the 15:09
    /// minute's worst price, else unpriceable. Either way the slot carries
    /// Dhan's square-off. Its one caller, `step`, has already returned for a
    /// finished slot.
    fn forced_exit(&mut self) {
        if self.filled == 0 {
            self.window_closed();
            return;
        }
        if self.remaining > 0 {
            self.flag(TradeFlag::PartialEntry);
            self.remaining = 0;
        }
        self.flag(TradeFlag::DhanSquareOff);
        if self.phase == Phase::Exiting {
            self.flag(TradeFlag::ExitSpillToForced);
        }
        // The position is open through the forced window: its excursions to
        // the exit run to 15:09:59, read from the window's summary.
        let window = self.side.forced;
        if window.best_raw > self.best_raw {
            self.best_raw = window.best_raw;
            self.mfe_second = window.best_first;
            self.last_peak_second = window.best_last;
        } else if window.bars > 0 && window.best_raw == self.best_raw {
            self.last_peak_second = window.best_last;
        }
        self.worst_raw = self.worst_raw.min(window.low_raw);
        let priced = match self.day.resolution() {
            Resolution::Second => {
                if window.bars > 0 && window.capable == 0 {
                    let locked = match self.direction {
                        Search::Long => UnpricedReason::ForcedLockedLong,
                        Search::Short => UnpricedReason::ForcedLockedShort,
                    };
                    self.finish(SlotStatus::Unpriceable(locked));
                    return;
                }
                (window.capable > 0 && window.capacity >= u128::from(self.open))
                    .then_some((window.worst_raw, window.last_second))
            }
            Resolution::Minute => window.minute.map(|price| {
                let second = u32::try_from(SESSION_FORCED_WINDOW_START_SECS).unwrap_or(u32::MAX);
                (price, second)
            }),
        };
        if let Some((price, second)) = priced {
            self.forced.book(self.open, price, second);
            self.worst_raw = self.worst_raw.min(price);
            self.open = 0;
            self.finish(SlotStatus::Priced);
        } else {
            let why = match self.day.resolution() {
                Resolution::Second => UnpricedReason::ForcedThin,
                Resolution::Minute => UnpricedReason::ForcedCheap,
            };
            self.finish(SlotStatus::Unpriceable(why));
        }
    }

    /// The slot's outcome, in exact integers, real-signed.
    #[must_use]
    pub fn outcome(&self) -> SlotOutcome {
        let sign = self.side.sign;
        let real = |v: i128| v * i128::from(sign);
        let (first_second, first_price, horizon_best, horizon_worst) = match self.first {
            Some((second, price, at)) => (
                second,
                price,
                self.side.best_from.get(at).copied().unwrap_or(price),
                self.side.worst_from.get(at).copied().unwrap_or(price),
            ),
            None => (0, 0, 0, 0),
        };
        let filled = self.first.is_some();
        let excursion = |a: i64, b: i64| if filled { a.saturating_sub(b) } else { 0 };
        let scale = i128::from(PERMILLE_PER_UNIT) * i128::from(PERMILLE_PER_UNIT);
        SlotOutcome {
            filled_qty: self.filled,
            entry_value_paisa: real(self.entry_value),
            exit_value_paisa: real(self.own.value + self.forced.value),
            risk_per_share_paisa: i64::try_from(self.levels.risk_decision).unwrap_or(i64::MAX),
            first_entry_second: first_second,
            last_entry_second: self.last_entry_second,
            exit_second: self.own.last_second.max(self.forced.last_second),
            exit_reason: self.reason.unwrap_or(ExitReason::Forced),
            mfe_to_exit_paisa: excursion(self.best_raw.max(first_price), first_price),
            mae_to_exit_paisa: excursion(first_price, self.worst_raw.min(first_price)),
            mfe_to_horizon_paisa: excursion(horizon_best.max(first_price), first_price),
            mae_to_horizon_paisa: excursion(first_price, horizon_worst.min(first_price)),
            flags: self.flags,
            status: self.status,
            decision_qty: self.decision_qty,
            planned_qty: self.planned_qty,
            risk_num: self.levels.risk / scale,
            risk_den: self.day.zerodha_close,
            first_entry_price_paisa: first_price.saturating_mul(sign),
            mfe_second: self.mfe_second,
            last_peak_second: self.last_peak_second,
            entry_wait_seconds: first_second.saturating_sub(self.entry_start),
            exit_spill_seconds: self.spill_seconds,
            max_participation_permille: self.max_participation,
            own_exit: self.own.report(sign),
            forced_exit: self.forced.report(sign),
        }
    }
}
